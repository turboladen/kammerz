//! Authoritative input validation — the backend half of kammerz-igc's
//! defense-in-depth. The frontend gates Save/Confirm buttons, but invalid data
//! must never persist regardless of client (kammerz-grd), so every create/update
//! handler runs its inputs through here before touching the DB.
//!
//! Date validation (`validate_date_opt`) accepts the same shape as the frontend:
//! empty (dates are optional) or a *complete* `YYYY-MM-DD` with year 1800–2100 and
//! a real calendar day. Partial `YYYY`/`YYYY-MM` are rejected (ADR-0011).
//!
//! The remaining helpers cover required strings, non-negative numbers (costs,
//! counts, dimensions — negatives are nonsensical and skew `/api/stats`), and
//! GPS coordinates (hard geographic bounds). Every helper names its `field` in
//! the 422 message so the user knows which input to fix. The numeric/coordinate
//! helpers take an `Option<T>` and pass `None` through (no value to check), so
//! an update handler validates a `double_option` field by peeling one layer —
//! `if let Some(inner) = data.cost { validate_non_negative_f64("cost", inner)? }`
//! — mirroring how `validate_date_opt` accepts the inner `&Option<String>`.

use crate::error::{AppError, AppResult};

const YEAR_MIN: i32 = 1800;
const YEAR_MAX: i32 = 2100;

/// Validate a required string from a create/update DTO, returning the trimmed
/// value for the caller to `Set()`. Whitespace-only input is rejected with a
/// 422 naming the field — a bare `trim()` would otherwise satisfy a `NOT NULL`
/// column with `""`, and for `roll_id` an empty value collides on the UNIQUE
/// index and surfaces a confusing duplicate error instead.
pub fn require_nonempty(field: &str, value: &str) -> AppResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(AppError::UnprocessableEntity(format!(
            "{field}: must not be empty"
        )))
    } else {
        Ok(trimmed.to_string())
    }
}

/// Reject a negative optional integer (costs, counts, dimensions). `None` is
/// accepted (the column is nullable / the field was omitted).
pub fn validate_non_negative_i32(field: &str, value: Option<i32>) -> AppResult<()> {
    match value {
        Some(v) if v < 0 => Err(AppError::UnprocessableEntity(format!(
            "{field}: must be 0 or greater"
        ))),
        _ => Ok(()),
    }
}

/// Reject a negative, NaN, or infinite optional float (monetary costs). `None`
/// is accepted. NaN/infinity can't be stored or aggregated meaningfully, so they
/// are rejected alongside negatives.
pub fn validate_non_negative_f64(field: &str, value: Option<f64>) -> AppResult<()> {
    match value {
        Some(v) if !v.is_finite() || v < 0.0 => Err(AppError::UnprocessableEntity(format!(
            "{field}: must be a finite number 0 or greater"
        ))),
        _ => Ok(()),
    }
}

/// Validate an optional latitude: finite and within −90..=90 degrees. `None` ok.
pub fn validate_lat(field: &str, value: Option<f64>) -> AppResult<()> {
    validate_coord(field, value, 90.0)
}

/// Validate an optional longitude: finite and within −180..=180 degrees. `None` ok.
pub fn validate_lon(field: &str, value: Option<f64>) -> AppResult<()> {
    validate_coord(field, value, 180.0)
}

/// Shared coordinate check: `None` passes; `Some(v)` must be finite and within
/// `±bound` (inclusive). Rejects NaN/infinity (the `!is_finite` arm).
fn validate_coord(field: &str, value: Option<f64>, bound: f64) -> AppResult<()> {
    match value {
        Some(v) if !v.is_finite() || v < -bound || v > bound => Err(AppError::UnprocessableEntity(
            format!("{field}: must be between {} and {bound}", -bound),
        )),
        _ => Ok(()),
    }
}

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
            "{field}: use YYYY-MM-DD"
        )))
    }
}

/// Validate an optional canonical 24-hour `HH:MM` time. `None`/blank are accepted
/// (the field is optional); a present value must be exactly `HH:MM` in range.
pub fn validate_time(field: &str, value: &Option<String>) -> AppResult<()> {
    let Some(raw) = value else { return Ok(()) };
    let v = raw.trim();
    if v.is_empty() {
        return Ok(());
    }
    if is_valid_time(v) {
        Ok(())
    } else {
        Err(AppError::UnprocessableEntity(format!(
            "{field}: use 24-hour HH:MM"
        )))
    }
}

/// True when `v` is a canonical `HH:MM` 24-hour time (two-digit hour 00–23 and
/// two-digit minute 00–59). Assumes `v` is already trimmed and non-empty.
fn is_valid_time(v: &str) -> bool {
    match v.split(':').collect::<Vec<&str>>().as_slice() {
        [h, m] => {
            let (Some(hour), Some(minute)) = (parse_fixed(h, 2), parse_fixed(m, 2)) else {
                return false;
            };
            (0..=23).contains(&hour) && (0..=59).contains(&minute)
        }
        _ => false,
    }
}

/// True when `v` is a complete `YYYY-MM-DD` within range. Assumes `v` is already
/// trimmed and non-empty. Partial `YYYY`/`YYYY-MM` are NOT accepted (ADR-0011):
/// dates are always full; an approximate date is entered as a concrete best-guess
/// with the "around" phrasing captured in notes.
fn is_valid_date(v: &str) -> bool {
    let parts: Vec<&str> = v.split('-').collect();
    match parts.as_slice() {
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

    fn time_err(field: &str, v: &str) -> bool {
        validate_time(field, &Some(v.to_string())).is_err()
    }
    fn time_ok(field: &str, v: &str) -> bool {
        validate_time(field, &Some(v.to_string())).is_ok()
    }

    #[test]
    fn time_none_and_blank_are_accepted() {
        assert!(validate_time("time", &None).is_ok());
        assert!(time_ok("time", ""));
        assert!(time_ok("time", "   "));
    }

    #[test]
    fn valid_24h_times_accepted() {
        assert!(time_ok("time", "00:00"));
        assert!(time_ok("time", "07:27"));
        assert!(time_ok("time", "19:27"));
        assert!(time_ok("time", "23:59"));
        // surrounding whitespace is trimmed like the date validator
        assert!(time_ok("time", "  08:15  "));
    }

    #[test]
    fn malformed_times_rejected() {
        assert!(time_err("time", "24:00")); // hour out of range
        assert!(time_err("time", "23:60")); // minute out of range
        assert!(time_err("time", "7:27")); // hour must be two digits
        assert!(time_err("time", "19:7")); // minute must be two digits
        assert!(time_err("time", "19:27:30")); // seconds not allowed
        assert!(time_err("time", "7:27pm")); // 12-hour notation
        assert!(time_err("time", "1927")); // missing colon
        assert!(time_err("time", "19-27")); // wrong separator
        assert!(time_err("time", "morning")); // junk
    }

    #[test]
    fn time_error_message_names_the_field() {
        let e = validate_time("shot_time", &Some("nope".into())).unwrap_err();
        match e {
            AppError::UnprocessableEntity(m) => assert!(m.contains("shot_time")),
            _ => panic!("expected UnprocessableEntity"),
        }
    }

    #[test]
    fn none_and_empty_are_accepted() {
        assert!(validate_date_opt("date_loaded", &None).is_ok());
        assert!(ok("date_loaded", ""));
        assert!(ok("date_loaded", "   "));
    }

    #[test]
    fn complete_valid_dates_accepted() {
        assert!(ok("d", "2026-06-11"));
        assert!(ok("d", "2024-02-29")); // leap day
        assert!(ok("d", "1800-01-01")); // lower year bound
        assert!(ok("d", "2100-12-31")); // upper year bound
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
        assert!(err("d", "2026")); // bare year — partials no longer accepted (ADR-0011)
        assert!(err("d", "2026-06")); // year-month
        assert!(err("d", "2026-"));
        assert!(err("d", "2026-06-"));
        assert!(err("d", "26"));
        assert!(err("d", "2026-6")); // month must be two digits
        assert!(err("d", "2026-06-1")); // day must be two digits
    }

    #[test]
    fn out_of_range_year_rejected() {
        assert!(err("d", "1700-01-01"));
        assert!(err("d", "2200-01-01"));
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

    #[test]
    fn require_nonempty_rejects_blank_and_returns_trimmed() {
        assert!(require_nonempty("brand", "").is_err());
        assert!(require_nonempty("brand", "   ").is_err());
        assert!(require_nonempty("brand", "\t\n").is_err());
        assert_eq!(require_nonempty("brand", "  Nikon  ").unwrap(), "Nikon");
        // The error names the field so the user knows which input to fix.
        let e = require_nonempty("roll_id", " ").unwrap_err();
        match e {
            AppError::UnprocessableEntity(m) => assert!(m.contains("roll_id")),
            _ => panic!("expected UnprocessableEntity"),
        }
    }

    #[test]
    fn non_negative_i32_accepts_none_zero_positive_rejects_negative() {
        assert!(validate_non_negative_i32("iso", None).is_ok());
        assert!(validate_non_negative_i32("iso", Some(0)).is_ok());
        assert!(validate_non_negative_i32("iso", Some(400)).is_ok());
        assert!(validate_non_negative_i32("iso", Some(-1)).is_err());
    }

    #[test]
    fn non_negative_f64_rejects_negative_nan_and_infinity() {
        assert!(validate_non_negative_f64("cost", None).is_ok());
        assert!(validate_non_negative_f64("cost", Some(0.0)).is_ok());
        assert!(validate_non_negative_f64("cost", Some(18.5)).is_ok());
        assert!(validate_non_negative_f64("cost", Some(-0.01)).is_err());
        assert!(validate_non_negative_f64("cost", Some(f64::NAN)).is_err());
        assert!(validate_non_negative_f64("cost", Some(f64::INFINITY)).is_err());
        assert!(validate_non_negative_f64("cost", Some(f64::NEG_INFINITY)).is_err());
    }

    #[test]
    fn lat_lon_enforce_bounds_and_reject_non_finite() {
        assert!(validate_lat("gps_lat", None).is_ok());
        assert!(validate_lat("gps_lat", Some(0.0)).is_ok());
        assert!(validate_lat("gps_lat", Some(-90.0)).is_ok());
        assert!(validate_lat("gps_lat", Some(90.0)).is_ok());
        assert!(validate_lat("gps_lat", Some(90.1)).is_err());
        assert!(validate_lat("gps_lat", Some(-91.0)).is_err());
        assert!(validate_lat("gps_lat", Some(f64::NAN)).is_err());

        assert!(validate_lon("gps_lon", Some(-180.0)).is_ok());
        assert!(validate_lon("gps_lon", Some(180.0)).is_ok());
        assert!(validate_lon("gps_lon", Some(181.0)).is_err());
        assert!(validate_lon("gps_lon", Some(f64::INFINITY)).is_err());
    }
}
