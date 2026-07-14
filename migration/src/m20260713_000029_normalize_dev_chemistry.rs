//! Normalize existing `development_selves` chemistry free-text to the canonical
//! names seeded in m..028 (kammerz-9fx).
//!
//! Source of truth: `~/kammerz-import/staging/dev-normalization.json`'s per-field
//! maps. Each entry is a guarded, idempotent `UPDATE ... WHERE <field> = '<from>'`
//! — the `WHERE` matches only the pre-normalization string, so re-running (or
//! running against an already-canonical row) is a no-op. Verified: no `to` value
//! is also a `from` within its field, so the set is order-independent (no
//! chaining). SQLite's default BINARY collation is case-sensitive, which is why
//! the map enumerates case variants (XTOL/Xtol, Photo-Flo/Photo-flo) explicitly.
//!
//! `NORMALIZATIONS` + `apply_normalization` are `pub` so `tests/chemicals.rs`
//! exercises the exact same data and apply step this migration does (they can't
//! drift). The live dev DB was already normalized during the 2026-06 import, so
//! on it these are all no-ops — the value is for fresh DBs / any un-normalized row.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

/// `(column, from, to)` normalizations for the `development_selves` chemistry
/// fields. `column` is always one of the five fixed chemistry column names —
/// never user input — so it is safe to interpolate as an identifier.
pub const NORMALIZATIONS: &[(&str, &str, &str)] = &[
    ("developer", "D76", "Kodak D-76"),
    ("developer", "Kodak D76", "Kodak D-76"),
    ("developer", "Xtol", "Kodak XTOL"),
    ("developer", "XTOL", "Kodak XTOL"),
    ("developer", "Kodak Xtol", "Kodak XTOL"),
    ("developer", "FPP monobath", "FPP Monobath"),
    ("developer", "Rodinal", "Adox Rodinal"),
    ("developer", "Rollei R09 One Shot (Rodinal)", "Adox Rodinal"),
    ("developer", "Rollei R90", "Adox Rodinal"),
    ("fixer", "Fixer", "Kodak Fixer"),
    ("stop_bath", "Stop bath", "Kodak Stop Bath"),
    ("wetting_agent", "Kodak Photo-Flo", "Kodak Photo-Flo 200"),
    ("wetting_agent", "Kodak Photo-flo", "Kodak Photo-Flo 200"),
    ("wetting_agent", "Photo flo", "Kodak Photo-Flo 200"),
    ("wetting_agent", "Photo-Flo", "Kodak Photo-Flo 200"),
    ("wetting_agent", "Photo-flo", "Kodak Photo-Flo 200"),
    ("wetting_agent", "photo flo", "Kodak Photo-Flo 200"),
    (
        "wetting_agent",
        "Hyper Flow",
        "Photographers Formulary Hyper Flow",
    ),
    ("clearing_agent", "Hypo Clear", "Kodak Hypo Clearing Agent"),
    ("clearing_agent", "Hypo clear", "Kodak Hypo Clearing Agent"),
];

/// The five `development_selves` chemistry columns `apply_normalization` may touch.
/// Interpolating `field` as a SQL identifier is only safe for these fixed names, so
/// the guard below rejects anything else — making the "never user input" contract
/// self-enforcing even though this helper is `pub` and re-exported.
const CHEMISTRY_COLUMNS: &[&str] = &[
    "developer",
    "fixer",
    "stop_bath",
    "wetting_agent",
    "clearing_agent",
];

/// Apply one guarded, idempotent normalization. `field` MUST be one of
/// [`CHEMISTRY_COLUMNS`] (never user input) — enforced below; `from`/`to` are
/// escaped as string literals. Shared with the migration's `up` so the test and the
/// migration apply the same step.
pub async fn apply_normalization(
    db: &impl ConnectionTrait,
    field: &str,
    from: &str,
    to: &str,
) -> Result<(), DbErr> {
    if !CHEMISTRY_COLUMNS.contains(&field) {
        return Err(DbErr::Custom(format!(
            "apply_normalization: refusing to interpolate non-chemistry column '{field}'"
        )));
    }
    let from_esc = from.replace('\'', "''");
    let to_esc = to.replace('\'', "''");
    db.execute_unprepared(&format!(
        "UPDATE development_selves SET {field} = '{to_esc}' WHERE {field} = '{from_esc}'"
    ))
    .await?;
    Ok(())
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for (field, from, to) in NORMALIZATIONS {
            apply_normalization(db, field, from, to).await?;
        }
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // Intentional no-op: many `from` values collapse onto one canonical `to`,
        // so the merge is not faithfully reversible. Leaving canonical values in
        // place is safe and correct.
        Ok(())
    }
}
