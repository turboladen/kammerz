# Show available recipes
default:
    @just --list

# Run backend (axum, :3001) and frontend (vite, :5173) together for dev.
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

check:
    cd frontend && bun run check
    cargo build
    cargo test

migrate:
    cargo run -- # migrations run on startup; this just boots once

# Generate the argon2 hash for KAMMERZ_PASSWORD_HASH (reads the password from
# stdin — prompts on a TTY, or pipe it: `echo -n <pw> | just hash-password`).
hash-password:
    cargo run -q -- hash-password
