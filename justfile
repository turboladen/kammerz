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

# Hard gates first (cargo + frontend build); svelte-check is informational —
# it currently reports 31 pre-existing type errors tracked separately.
check:
    cargo build
    cargo test
    cd frontend && bun run build
    -cd frontend && bun run check

migrate:
    cargo run -- # migrations run on startup; this just boots once

# Generate the argon2 hash for KAMMERZ_PASSWORD_HASH (reads the password from
# stdin — prompts on a TTY, or pipe it: `echo -n <pw> | just hash-password`).
hash-password:
    cargo run -q -- hash-password
