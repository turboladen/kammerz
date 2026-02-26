---
name: warn-legacy-svelte
enabled: true
event: file
action: warn
conditions:
  - field: file_path
    operator: regex_match
    pattern: \.svelte$
  - field: new_text
    operator: regex_match
    pattern: (\$:\s|export\s+let\s+)
---

Legacy Svelte reactivity detected in a .svelte file.

This project uses **Svelte 5 runes** exclusively:
- `$:` reactive statements → use `$derived()` or `$derived.by()`
- `export let` → use `$props()` with destructuring (or `$bindable()` for two-way bindings)

See CLAUDE.md "Svelte 5 Patterns" section for conventions.
