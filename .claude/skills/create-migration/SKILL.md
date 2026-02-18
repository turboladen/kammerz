---
name: create-migration
description: Create a new SeaORM migration for the Kammerz SQLite database
disable-model-invocation: true
---
# Create Migration

Generate a new SeaORM migration file and register it in the migrator.

## Steps

1. Read the existing migrations in `src-tauri/migration/src/` to determine the next migration number and naming convention
2. The pattern is: `m{YYYYMMDD}_{NNNNNN}_{description}.rs`
3. Create the migration file following the SeaORM 1.1 migration pattern used in existing files
4. Register the migration in `src-tauri/migration/src/lib.rs` by adding it to the `Migrator` vec
5. Run `cargo check` in `src-tauri/` to verify it compiles

## Important
- Use `sea_orm_migration::prelude::*`
- Follow the existing entity naming conventions (snake_case table names, String for timestamps)
- Add `Option<T>` for nullable columns
- Always include `created_at` and `updated_at` TEXT columns for new tables

