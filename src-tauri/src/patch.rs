use serde::{Deserialize, Deserializer};

/// Deserializes a field as `Option<Option<T>>` to distinguish between:
/// - Field absent from JSON → `None` (don't update; requires `#[serde(default)]` on struct)
/// - Field present as `null` → `Some(None)` (set to NULL in DB)
/// - Field present with value → `Some(Some(value))` (set to value)
///
/// Use on nullable entity fields in Update DTOs:
/// ```ignore
/// #[serde(deserialize_with = "double_option")]
/// pub notes: Option<Option<String>>,
/// ```
pub fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Ok(Some(Option::deserialize(deserializer)?))
}
