linux_target := "aarch64-unknown-linux-gnu"

# Show available recipes
default:
    @just --list

# Run backend (axum, :3002) and frontend (vite, :5273) together for dev.
dev:
    #!/usr/bin/env bash
    trap 'kill 0' EXIT
    cargo run &
    (cd frontend && bun run dev) &
    wait

dev-backend:
    cargo run

dev-frontend:
    cd frontend && bun run dev

# Production build: frontend → frontend/build, then embed into the release binary.
build:
    cd frontend && bun install && bun run build
    cargo build --release
    # adapter-static wipes frontend/build; restore the tracked placeholder so a
    # clean checkout still has the dir rust-embed's #[folder] points at.
    touch frontend/build/.gitkeep

# Cross-compile a release binary for the Linux server (DietPi, ARM64).
# One-time toolchain setup (see README "Deployment"): rustup target add +
# brew install messense/macos-cross-toolchains/aarch64-unknown-linux-gnu.
# The linker is set here (not .cargo/config.toml) so a NATIVE cargo build on
# an aarch64 Linux box isn't broken by a committed [target.*] section — cargo
# applies those to host builds too when the triples match. Depends on
# ci-frontend deliberately: the embedded SPA must be freshly built (a stale
# frontend/build means a stale binary), and svelte-check is kept as a gate
# because this artifact ships straight to production.
build-linux: ci-frontend
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-unknown-linux-gnu-gcc \
        cargo build --release --locked --target {{linux_target}}

# Deploy to the server: e.g. `just deploy dietpi@192.168.8.50`, or an ssh
# alias; pass a port if the server's .env overrides PORT (`just deploy box 8080`).
# Release = deploy. Requires passwordless sudo for the remote user (sudo -n
# fails fast instead of prompting). Gated on ci-backend (cargo test) — this
# ships to the only production box and Actions CI is unavailable.
#
# Order matters: the binary uploads to kammerz.new while the old binary keeps
# serving, so a dead network mid-stream strands nothing; the service stops
# only for the atomic mv + restart. The `|| true` on stop is for the
# first-ever deploy (unit not installed yet, exit 5) — a stop that silently
# fails later is caught by the build-SHA comparison, since the old binary
# would still be the one answering /api/health.
deploy host port='3002': ci-backend build-linux
    #!/usr/bin/env bash
    set -euo pipefail
    host="{{host}}"
    sha="$(git rev-parse --short=8 HEAD)"
    ssh "$host" "sudo -n tee /opt/kammerz/kammerz.new > /dev/null && sudo -n chmod +x /opt/kammerz/kammerz.new && sudo -n chown kammerz:kammerz /opt/kammerz/kammerz.new" < target/{{linux_target}}/release/kammerz
    ssh "$host" "sudo -n tee /etc/systemd/system/kammerz.service > /dev/null" < deploy/kammerz.service
    ssh "$host" "sudo -n systemctl stop kammerz 2>/dev/null || true; sudo -n mv /opt/kammerz/kammerz.new /opt/kammerz/kammerz && sudo -n systemctl daemon-reload && sudo -n systemctl start kammerz"
    echo "waiting for /api/health (port {{port}}) to report build $sha ..."
    for _ in $(seq 1 60); do
        if out=$(ssh -o ConnectTimeout=5 "$host" "curl -fs --max-time 2 http://localhost:{{port}}/api/health" 2>/dev/null); then
            if [[ "$out" == *"\"build\":\"$sha"* ]]; then
                echo "✅ deployed $sha: $out"
                exit 0
            fi
            echo "❌ server is up but reports a different build than $sha: $out" >&2
            echo "   (old binary still running? check: ssh $host 'journalctl -u kammerz -n 50')" >&2
            exit 1
        fi
        sleep 0.5
    done
    echo "❌ no answer from /api/health within ~30s — check: ssh $host 'journalctl -u kammerz -n 50'" >&2
    exit 1


# Format everything in place: dprint (Markdown/JSON/TOML/YAML), Prettier
# (frontend: Svelte/TS/CSS via prettier-plugin-svelte), rustfmt (Rust).
# Run before committing.
fmt:
    dprint fmt
    cd frontend && bun run format
    cargo fmt --all

# Verify formatting without writing — mirrors CI's format job.
fmt-check:
    dprint check
    cd frontend && bun run format:check
    cargo fmt --all --check

# Quality gates — all hard gates, matching what CI enforces on every PR and
# push to main (.github/workflows/ci.yml). Delegates to the ci-* recipes so the
# gate commands exist in exactly one place (--locked/--frozen-lockfile included:
# lockfile drift should fail here, not surface later in `just ci` or Actions).
check: fmt-check ci-backend ci-frontend

# Full local mirror of the GitHub Actions pipeline (.github/workflows/ci.yml):
# backend (cargo build+test --locked), frontend (frozen install + svelte-check
# + build), e2e (Playwright smoke against the release binary on :3002). Use
# this as the PR gate when Actions isn't available — every job a PR needs runs
# here, in the same order, with the same flags.
ci: ci-preflight fmt-check ci-backend ci-frontend e2e
    @echo "✅ just ci: all CI jobs passed (format, backend, frontend, e2e)"

# Warn (never fail) when the local run can diverge from what Actions would do:
# a dirty tree means the gate result may not reflect the pushed commits, and a
# bun other than ci.yml's pinned BUN_VERSION can behave differently.
ci-preflight:
    #!/usr/bin/env bash
    if [ -n "$(git status --porcelain)" ]; then
        echo "⚠️  working tree is dirty — this gate run includes uncommitted changes" >&2
    fi
    pinned=$(sed -n 's/.*BUN_VERSION: *"\([^"]*\)".*/\1/p' .github/workflows/ci.yml)
    actual=$(bun --version 2>/dev/null || echo none)
    if [ -n "$pinned" ] && [ "$pinned" != "$actual" ]; then
        echo "⚠️  local bun $actual ≠ CI-pinned $pinned (.github/workflows/ci.yml)" >&2
    fi

# Mirrors the `backend` CI job.
ci-backend:
    cargo build --workspace --locked
    cargo test --workspace --locked

# Mirrors the `frontend` CI job (frozen install, like CI since PR #37).
ci-frontend:
    cd frontend && bun install --frozen-lockfile
    cd frontend && bun run check
    cd frontend && bun run build
    touch frontend/build/.gitkeep

# Mirrors the `e2e` CI job: embed the SPA (built by the ci-frontend dependency)
# in the release binary, start it with a runtime-minted argon2 hash (no
# committed secret), run the Playwright smoke suite. DB and server log live
# under /tmp so a local run leaves no files in the repo.
e2e: ci-frontend
    #!/usr/bin/env bash
    set -euo pipefail
    # Free this run's port first. LISTEN-scoped on purpose: a bare lsof -i:3002
    # also matches CLIENTS connected to :3002 (a browser talking to `just dev`)
    # and would kill -9 them; no pattern kills (pkill -f) either — they take
    # out unrelated processes (lessons learned in chorez).
    lsof -ti tcp:3002 -sTCP:LISTEN 2>/dev/null | xargs kill -9 2>/dev/null || true
    sleep 0.3
    PIDS=()
    cleanup() {
        for p in "${PIDS[@]:-}"; do kill "$p" 2>/dev/null || true; done
        rm -f /tmp/kammerz-ci-e2e.db /tmp/kammerz-ci-e2e.db-wal /tmp/kammerz-ci-e2e.db-shm
    }
    # cleanup runs on EXIT only; INT/TERM exit explicitly so a signal can't
    # leave the script running on (bash resumes after a non-exiting trap —
    # the suite would then run against the server cleanup just killed).
    trap cleanup EXIT
    trap 'exit 130' INT
    trap 'exit 143' TERM
    # Bounded + PID-liveness wait: fail fast if the server dies before
    # responding instead of hanging forever. 300 * 0.2s = 60s, matching CI.
    wait_for() {
        local label=$1 pid=$2; shift 2
        for i in $(seq 1 300); do
            if ! kill -0 "$pid" 2>/dev/null; then
                echo "❌ $label (PID $pid) died before responding:" >&2
                cat /tmp/kammerz-ci-e2e-server.log >&2 || true
                exit 1
            fi
            "$@" && return 0
            sleep 0.2
        done
        echo "❌ $label did not respond within 60s — server log:" >&2
        cat /tmp/kammerz-ci-e2e-server.log >&2 || true
        exit 1
    }
    cargo build --release --locked
    HASH="$(printf '%s' secret | ./target/release/kammerz hash-password)"
    # Deterministic start even if a previous run was SIGKILLed before its
    # cleanup trap could fire (the trap also rm's these on normal exits).
    rm -f /tmp/kammerz-ci-e2e.db /tmp/kammerz-ci-e2e.db-wal /tmp/kammerz-ci-e2e.db-shm
    # Pin every env var the server reads: dotenvy loads the developer's .env
    # for anything unset (CI has no .env), so an unpinned SECURE_COOKIES=true
    # or a real ANTHROPIC_API_KEY would leak into the gate run.
    KAMMERZ_PASSWORD_HASH="$HASH" \
    DATABASE_URL='sqlite:/tmp/kammerz-ci-e2e.db?mode=rwc' \
    PORT=3002 \
    SECURE_COOKIES=false \
    ANTHROPIC_API_KEY= \
        ./target/release/kammerz > /tmp/kammerz-ci-e2e-server.log 2>&1 &
    PIDS+=($!)
    wait_for server $! curl -fs -o /dev/null http://localhost:3002/api/health
    cd frontend
    bunx playwright install --with-deps chromium
    # CI=1 turns on playwright's forbidOnly so a committed test.only fails the
    # gate instead of silently shrinking the suite.
    if ! CI=1 E2E_PASSWORD=secret E2E_BASE=http://localhost:3002 bun run test:e2e; then
        echo "❌ playwright smoke failed — server log:" >&2
        cat /tmp/kammerz-ci-e2e-server.log >&2 || true
        exit 1
    fi

# One-time setup: `cargo install cargo-llvm-cov` + `rustup component add
# llvm-tools-preview`. The instrumented build uses its own target subdir, so the
# first run recompiles the workspace and won't reuse the normal build cache.
# Writes a browseable HTML report under target/llvm-cov/html (gitignored via
# /target) and prints a per-file + total summary. Local-only — not a CI gate.
# Backend test coverage report (cargo-llvm-cov).
coverage:
    cargo llvm-cov --workspace --html
    @echo "HTML report: target/llvm-cov/html/index.html"

# Same as `coverage`, but opens the HTML report in a browser when it finishes.
coverage-open:
    cargo llvm-cov --workspace --html --open

migrate:
    cargo run -- # migrations run on startup; this just boots once

# Generate the argon2 hash for KAMMERZ_PASSWORD_HASH (reads the password from
# stdin — prompts on a TTY, or pipe it: `echo -n <pw> | just hash-password`).
hash-password:
    cargo run -q -- hash-password
