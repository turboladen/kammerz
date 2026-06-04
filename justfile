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

migrate:
    cargo run -- # migrations run on startup; this just boots once

# Generate the argon2 hash for KAMMERZ_PASSWORD_HASH (reads the password from
# stdin — prompts on a TTY, or pipe it: `echo -n <pw> | just hash-password`).
hash-password:
    cargo run -q -- hash-password
