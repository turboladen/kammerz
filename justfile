# Show available recipes
default:
    @just --list

# Run the app in development mode (Tauri + Vite)
dev:
    bun run tauri dev

# Build the production .app bundle
build:
    bun run tauri build

# Full type check: SvelteKit frontend + Rust backend
check: check-svelte check-rust

# Type-check the SvelteKit frontend (svelte-check)
check-svelte:
    bun run check

# Compile-check the Rust backend
check-rust:
    cd src-tauri && cargo build

# Delete the SQLite database so migrations re-run from scratch on next launch
clean-db:
    rm -f "$HOME/Library/Application Support/com.kammerz.app/kammerz.db"
    @echo "Database deleted. Run 'just dev' to recreate."

# Open a sqlite3 shell against the dev database
db:
    sqlite3 "$HOME/Library/Application Support/com.kammerz.app/kammerz.db"

# Print the path to the dev database
db-path:
    @echo "$HOME/Library/Application Support/com.kammerz.app/kammerz.db"

# Free port 1420 if a previous `just dev` left an orphaned Vite process
kill-port:
    -lsof -ti:1420 | xargs kill -9
    @echo "Port 1420 freed."
