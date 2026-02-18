---
name: migration-checker
description: Reviews SeaORM database migration files to make sure they're consistent with the rest of the stack.
tools:
  - Read
  - Grep
  - Glob
  - Bash
model: sonnet
---
# Migration Checker

Verify that a SeaORM migration is consistent with the rest of the Kamerz stack.

## Checks
1. Migration file follows naming convention and compiles (`cargo check` in src-tauri/)
2. Corresponding entity file exists in `src-tauri/src/entities/` and matches the schema
3. Service file exists with at least list/create/update/delete methods
4. Command file exists with proper DTOs and `#[tauri::command]` attributes
5. Command is registered in `src-tauri/src/lib.rs` invoke_handler
6. Frontend API wrapper exists in `src/lib/api/`
7. TypeScript types in `src/lib/types/index.ts` match the Rust entity fields
