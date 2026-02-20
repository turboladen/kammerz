# Delete the SQLite database so migrations re-run from scratch on next launch
clean-db:
    rm -f "$HOME/Library/Application Support/com.kammerz.app/kammerz.db"
    @echo "Database deleted. Run 'bun run tauri dev' to recreate."
