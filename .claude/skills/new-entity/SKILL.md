---
name: new-entity
description: Scaffold a new entity across the full Kammerz stack (migration → entity → service → command → API wrapper → types → page)
disable-model-invocation: true
---

# New Entity

Scaffold a complete new entity across all layers of the Kammerz stack.

## Layers to create (in order)

1. **Migration** — `src-tauri/migration/src/m{date}_{seq}_{name}.rs` + register in `lib.rs`
2. **Entity** — `src-tauri/src/entities/{name}.rs` + register in `mod.rs`
3. **Service** — `src-tauri/src/services/{name}.rs` with CRUD methods + register in `mod.rs`
4. **Command** — `src-tauri/src/commands/{name}.rs` with DTOs + register in `mod.rs` and `lib.rs`
5. **API wrapper** — `src/lib/api/{name}.ts` with invoke() calls
6. **Types** — Add interfaces to `src/lib/types/index.ts`
7. **Page** — `src/routes/{name}/+page.svelte` with list view, add dialog, edit/delete

## Conventions
- Follow existing patterns in each layer exactly (read a similar entity first)
- Services are static async methods on unit structs
- Commands use DTOs for create/update payloads
- Frontend uses Svelte 5 runes ($state, $derived, $props)
- Add the new route to the Sidebar component
- Include EmptyState with appropriate Lucide icon

