#[derive(Clone, Debug)]
pub struct AppConfig {
    /// Argon2 hash of the single shared password. If None, auth is OPEN
    /// (LAN-trust mode) and a warning is logged at startup.
    pub password_hash: Option<String>,
    /// Optional Anthropic API key override. If None, the import feature falls
    /// back to the `claude_api_key` row in the settings table.
    pub anthropic_api_key: Option<String>,
    /// Session cookie Secure flag (set true when behind TLS/VPN with HTTPS).
    pub secure_cookies: bool,
    /// Trust `X-Forwarded-For` for the login rate-limiter's per-client key. Set
    /// true ONLY when behind a reverse proxy that *replaces* the header with the
    /// single real client IP — the limiter keys on the leftmost parseable entry,
    /// so an *appending* proxy is unsafe (an attacker forges a leading IP the proxy
    /// preserves). Otherwise every request arrives from the proxy's single IP and
    /// the limiter collapses to one shared bucket (an attacker could then lock out
    /// the real user). Default false: XFF is client-spoofable when no overwriting
    /// proxy exists, so trusting it would let an attacker forge a fresh bucket per
    /// request and defeat the throttle entirely. See `auth::rate_limit`.
    pub trust_proxy: bool,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            password_hash: std::env::var("KAMMERZ_PASSWORD_HASH")
                .ok()
                .filter(|s| !s.is_empty()),
            anthropic_api_key: std::env::var("ANTHROPIC_API_KEY")
                .ok()
                .filter(|s| !s.is_empty()),
            secure_cookies: env_flag("SECURE_COOKIES"),
            trust_proxy: env_flag("KAMMERZ_TRUST_PROXY"),
        }
    }

    /// Whether auth is enforced. `true` when a password hash is configured;
    /// `false` is OPEN LAN-trust mode. Single source of truth for the open-mode
    /// predicate so the `RequireAuth` extractor and the auth handlers can't drift.
    pub fn auth_enabled(&self) -> bool {
        self.password_hash.is_some()
    }
}

/// Read a boolean toggle env var. `true` only for the literal `"true"` or `"1"`;
/// anything else (including unset, empty, or garbage) is `false`. Shared by every
/// bool config flag so they parse identically.
fn env_flag(name: &str) -> bool {
    std::env::var(name).map(parse_flag).unwrap_or(false)
}

/// The value-parsing half of [`env_flag`], split out so it can be unit-tested
/// without mutating process env (which races under the parallel test harness).
fn parse_flag(value: String) -> bool {
    value == "true" || value == "1"
}

#[cfg(test)]
mod tests {
    use super::parse_flag;

    #[test]
    fn flag_true_for_true_and_one() {
        assert!(parse_flag("true".to_string()));
        assert!(parse_flag("1".to_string()));
    }

    #[test]
    fn flag_false_for_false_empty_and_garbage() {
        assert!(!parse_flag("false".to_string()));
        assert!(!parse_flag("0".to_string()));
        assert!(!parse_flag(String::new()));
        assert!(!parse_flag("yes".to_string()));
        assert!(!parse_flag("TRUE".to_string())); // case-sensitive by design
    }
}
