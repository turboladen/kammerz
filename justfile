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

# Quality gates — all hard gates, matching what CI enforces on every PR and
# push to main (.github/workflows/ci.yml). `bun run check` (svelte-check) must
# pass before opening a PR.
check:
    cargo build
    cargo test
    # bun install first so a clean checkout (frontend/node_modules is gitignored)
    # has svelte-check/svelte-kit available — mirrors what each CI job does.
    cd frontend && bun install
    cd frontend && bun run check
    cd frontend && bun run build
    # adapter-static wipes frontend/build; restore the tracked placeholder so the
    # working tree stays clean (rust-embed's #[folder] needs the dir present).
    touch frontend/build/.gitkeep

# Full local mirror of the GitHub Actions pipeline (.github/workflows/ci.yml):
# backend (cargo build+test --locked), frontend (frozen install + svelte-check
# + build), e2e (Playwright smoke against the release binary on :3002). Use
# this as the PR gate when Actions isn't available — every job a PR needs runs
# here, in the same order, with the same flags.
ci: ci-backend ci-frontend e2e
    @echo "✅ just ci: all CI jobs passed (backend, frontend, e2e)"

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

# Kill whatever holds this run's port (:3002) before e2e spawns the server.
# Port-scoped on purpose — no pattern kills (pkill -f) here: they take out
# unrelated processes on other ports (lesson learned in chorez).
e2e-preflight:
    #!/usr/bin/env bash
    lsof -ti:3002 2>/dev/null | xargs kill -9 2>/dev/null || true
    sleep 0.3

# Mirrors the `e2e` CI job: build the SPA, embed it in the release binary,
# start the server with a runtime-minted argon2 hash (no committed secret),
# run the Playwright smoke suite against it. DB lives under /tmp so a local
# run leaves no files in the repo.
e2e: e2e-preflight
    #!/usr/bin/env bash
    set -euo pipefail
    PIDS=()
    cleanup() {
        for p in "${PIDS[@]:-}"; do kill "$p" 2>/dev/null || true; done
        rm -f /tmp/kammerz-ci-e2e.db /tmp/kammerz-ci-e2e.db-wal /tmp/kammerz-ci-e2e.db-shm
    }
    trap cleanup EXIT INT TERM
    # Bounded + PID-liveness wait: fail fast if the server dies before
    # responding instead of hanging forever. 150 * 0.2s = 30s budget.
    wait_for() {
        local label=$1 pid=$2; shift 2
        for i in $(seq 1 150); do
            if ! kill -0 "$pid" 2>/dev/null; then
                echo "❌ $label (PID $pid) died before responding:" >&2
                cat /tmp/kammerz-ci-e2e-server.log >&2 || true
                exit 1
            fi
            "$@" && return 0
            sleep 0.2
        done
        echo "❌ $label did not respond within 30s." >&2
        exit 1
    }
    (cd frontend && bun install --frozen-lockfile && bun run build)
    cargo build --release --locked
    git checkout -- frontend/build/.gitkeep 2>/dev/null || touch frontend/build/.gitkeep
    HASH="$(printf '%s' secret | ./target/release/kammerz hash-password)"
    rm -f /tmp/kammerz-ci-e2e.db /tmp/kammerz-ci-e2e.db-wal /tmp/kammerz-ci-e2e.db-shm
    KAMMERZ_PASSWORD_HASH="$HASH" \
    DATABASE_URL='sqlite:/tmp/kammerz-ci-e2e.db?mode=rwc' \
    PORT=3002 \
        ./target/release/kammerz > /tmp/kammerz-ci-e2e-server.log 2>&1 &
    PIDS+=($!)
    wait_for server $! curl -fs -o /dev/null http://localhost:3002/api/health
    cd frontend
    bunx playwright install chromium
    if ! E2E_PASSWORD=secret E2E_BASE=http://localhost:3002 bun run test:e2e; then
        echo "❌ playwright smoke failed — server log:" >&2
        cat /tmp/kammerz-ci-e2e-server.log >&2 || true
        exit 1
    fi

migrate:
    cargo run -- # migrations run on startup; this just boots once

# Generate the argon2 hash for KAMMERZ_PASSWORD_HASH (reads the password from
# stdin — prompts on a TTY, or pipe it: `echo -n <pw> | just hash-password`).
hash-password:
    cargo run -q -- hash-password
