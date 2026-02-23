pub mod cameras;
pub mod lenses;
pub mod lens_mounts;
pub mod film_stocks;
pub mod labs;
pub mod rolls;
pub mod shots;
pub mod development;
pub mod search;
pub mod stats;
pub mod settings;
pub mod import;

/// Map a DB error to a user-friendly message. Recognizes common SQLite constraint
/// errors and produces actionable text; falls back to the raw error otherwise.
/// Accepts DbErr, TransactionError<DbErr>, or any Display type.
///
/// The `context` should be a noun phrase (e.g. "roll", "camera", "film stock").
pub fn friendly_err(context: &str, e: impl std::fmt::Display) -> String {
    let raw = e.to_string();

    // SeaORM wraps SQLite errors (e.g. "Execution Error: ... UNIQUE constraint
    // failed: table.col"), so we search with `contains()` + extract the tail.

    // UNIQUE constraint failed: table.column
    if let Some(pos) = raw.find("UNIQUE constraint failed: ") {
        let rest = &raw[pos + "UNIQUE constraint failed: ".len()..];
        // Strip any trailing quote/paren that SeaORM's wrapping may add
        let rest = rest.trim_end_matches(|c: char| c == '"' || c == ')' || c.is_whitespace());
        let col = rest
            .split('.')
            .last()
            .unwrap_or("value")
            .replace('_', " ");
        return format!("A {context} with that {col} already exists.");
    }
    if raw.contains("UNIQUE constraint failed") {
        return format!("A {context} with those values already exists.");
    }

    // FOREIGN KEY constraint failed (usually on delete)
    if raw.contains("FOREIGN KEY constraint failed") {
        return format!(
            "Cannot delete this {context} because it is still referenced by other records."
        );
    }

    // NOT NULL constraint failed: table.column
    if let Some(pos) = raw.find("NOT NULL constraint failed: ") {
        let rest = &raw[pos + "NOT NULL constraint failed: ".len()..];
        let rest = rest.trim_end_matches(|c: char| c == '"' || c == ')' || c.is_whitespace());
        let col = rest
            .split('.')
            .last()
            .unwrap_or("field")
            .replace('_', " ");
        return format!("The {col} field is required.");
    }

    // Default: neutral verb so the message reads correctly with a noun context
    format!("Could not save {context}: {raw}")
}
