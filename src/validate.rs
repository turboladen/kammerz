//! Authoritative date validation — the backend half of kammerz-igc's
//! defense-in-depth. The frontend gates Save/Confirm buttons (`dateFieldError`
//! in `frontend/src/lib/utils/date.ts`), but a malformed date must never persist
//! regardless of client, so every create/update handler with a real date column
//! runs its inputs through here before touching the DB.
//!
//! The accepted shape mirrors the frontend exactly: a date is OK when it is
//! empty (dates are optional) or a *complete* `YYYY`, `YYYY-MM`, or `YYYY-MM-DD`
//! with year 1800–2100 and — for full dates — a real calendar day. Free-form
//! `*_fuzzy` columns are deliberately NOT validated; handlers skip them.

use crate::error::{AppError, AppResult};

const YEAR_MIN: i32 = 1800;
const YEAR_MAX: i32 = 2100;

/// Validate an optional date string from a create/update DTO. `field` names the
/// column (e.g. `"date_loaded"`) so the 422 message points the user at it.
///
/// `None` and empty/whitespace strings are accepted (the column is nullable).
/// Anything non-empty must be a complete, real date in the accepted shapes.
pub fn validate_date_opt(field: &str, value: &Option<String>) -> AppResult<()> {
    let Some(raw) = value else { return Ok(()) };
    let v = raw.trim();
    if v.is_empty() {
        return Ok(());
    }
    if is_valid_date(v) {
        Ok(())
    } else {
        Err(AppError::UnprocessableEntity(format!(
            "{field}: use YYYY, YYYY-MM, or YYYY-MM-DD"
        )))
    }
}

/// True when `v` is a complete `YYYY`, `YYYY-MM`, or `YYYY-MM-DD` within range.
/// Assumes `v` is already trimmed and non-empty.
fn is_valid_date(v: &str) -> bool {
    let parts: Vec<&str> = v.split('-').collect();
    match parts.as_slice() {
        // YYYY
        [y] => parse_fixed(y, 4).is_some_and(in_year_range),
        // YYYY-MM
        [y, m] => {
            let (Some(year), Some(month)) = (parse_fixed(y, 4), parse_fixed(m, 2)) else {
                return false;
            };
            in_year_range(year) && (1..=12).contains(&month)
        }
        // YYYY-MM-DD — chrono validates the day (leap years, month lengths).
        [y, m, d] => {
            let (Some(year), Some(_), Some(_)) =
                (parse_fixed(y, 4), parse_fixed(m, 2), parse_fixed(d, 2))
            else {
                return false;
            };
            in_year_range(year) && chrono::NaiveDate::parse_from_str(v, "%Y-%m-%d").is_ok()
        }
        _ => false,
    }
}

/// Parse an exactly-`width`-digit ASCII run as an `i32`. Rejects signs, spaces,
/// and wrong-length segments so `"2026 "`, `"-5"`, or `"6"` (for a month) fail.
fn parse_fixed(s: &str, width: usize) -> Option<i32> {
    if s.len() == width && s.bytes().all(|b| b.is_ascii_digit()) {
        s.parse().ok()
    } else {
        None
    }
}

fn in_year_range(y: i32) -> bool {
    (YEAR_MIN..=YEAR_MAX).contains(&y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err(field: &str, v: &str) -> bool {
        validate_date_opt(field, &Some(v.to_string())).is_err()
    }
    fn ok(field: &str, v: &str) -> bool {
        validate_date_opt(field, &Some(v.to_string())).is_ok()
    }

    #[test]
    fn none_and_empty_are_accepted() {
        assert!(validate_date_opt("date_loaded", &None).is_ok());
        assert!(ok("date_loaded", ""));
        assert!(ok("date_loaded", "   "));
    }

    #[test]
    fn complete_valid_dates_accepted() {
        assert!(ok("d", "2026"));
        assert!(ok("d", "2026-06"));
        assert!(ok("d", "2026-06-11"));
        assert!(ok("d", "2024-02-29")); // leap day
        assert!(ok("d", "1800"));
        assert!(ok("d", "2100-12-31"));
    }

    #[test]
    fn malformed_dates_rejected() {
        assert!(err("d", "2026-13-45"));
        assert!(err("d", "2026-02-30")); // not a real day
        assert!(err("d", "2023-02-29")); // not a leap year
        assert!(err("d", "2026-00-01"));
        assert!(err("d", "2026-01-00"));
    }

    #[test]
    fn incomplete_input_rejected() {
        assert!(err("d", "2026-"));
        assert!(err("d", "2026-06-"));
        assert!(err("d", "26"));
        assert!(err("d", "2026-6")); // month must be two digits
        assert!(err("d", "2026-06-1")); // day must be two digits
    }

    #[test]
    fn out_of_range_year_rejected() {
        assert!(err("d", "1700-01-01"));
        assert!(err("d", "2200"));
    }

    #[test]
    fn junk_rejected() {
        assert!(err("d", "not-a-date"));
        assert!(err("d", "2026/06/11"));
        assert!(err("d", "June 2026"));
    }

    #[test]
    fn error_message_names_the_field() {
        let e = validate_date_opt("date_received", &Some("nope".into())).unwrap_err();
        match e {
            AppError::UnprocessableEntity(m) => assert!(m.contains("date_received")),
            _ => panic!("expected UnprocessableEntity"),
        }
    }
}
