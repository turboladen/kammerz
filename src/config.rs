use std::net::{IpAddr, Ipv4Addr};

/// Bind address used when `BIND_ADDR` is unset: `0.0.0.0` (all interfaces),
/// preserving the behavior from before `BIND_ADDR` existed.
const DEFAULT_BIND_ADDR: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);

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
    /// IP the listener binds to. Defaults to `0.0.0.0` (all interfaces),
    /// preserving the pre-`BIND_ADDR` behavior. Set `127.0.0.1` to bind
    /// loopback-only — the recommended posture behind a reverse proxy or when
    /// OPEN (no-password) mode must never be reachable off-host.
    pub bind_addr: IpAddr,
}

impl Default for AppConfig {
    /// All-defaults config: OPEN auth, no API key, insecure cookies, peer-IP
    /// rate limiting, and the `0.0.0.0` bind. Tests spread `..Default::default()`
    /// over this so adding a field here doesn't churn every config literal.
    fn default() -> Self {
        Self {
            password_hash: None,
            anthropic_api_key: None,
            secure_cookies: false,
            trust_proxy: false,
            bind_addr: DEFAULT_BIND_ADDR,
        }
    }
}

impl AppConfig {
    /// Build the config from environment variables. Returns `Err` with an
    /// operator-facing message when a value is present but malformed (currently
    /// only `BIND_ADDR`); `main` reports it and exits, keeping all startup
    /// config validation at one altitude (alongside the password-hash check).
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            password_hash: std::env::var("KAMMERZ_PASSWORD_HASH")
                .ok()
                .filter(|s| !s.is_empty()),
            anthropic_api_key: std::env::var("ANTHROPIC_API_KEY")
                .ok()
                .filter(|s| !s.is_empty()),
            secure_cookies: env_flag("SECURE_COOKIES"),
            trust_proxy: env_flag("KAMMERZ_TRUST_PROXY"),
            // Fail-fast (rather than silently defaulting to 0.0.0.0) because a
            // typo'd loopback bind would re-expose the catalog — the exact
            // footgun BIND_ADDR exists to close.
            bind_addr: parse_bind_addr(std::env::var("BIND_ADDR").ok())?,
        })
    }

    /// Whether auth is enforced. `true` when a password hash is configured;
    /// `false` is OPEN LAN-trust mode. Single source of truth for the open-mode
    /// predicate so the `RequireAuth` extractor and the auth handlers can't drift.
    pub fn auth_enabled(&self) -> bool {
        self.password_hash.is_some()
    }
}

/// Parse the `BIND_ADDR` env value into an [`IpAddr`]. Unset or empty →
/// [`DEFAULT_BIND_ADDR`] (`0.0.0.0`); a valid IP literal → that address;
/// anything else → `Err` with an operator-facing message. Returning `IpAddr`
/// (not `String`) means a malformed value can never reach the listener — it's
/// rejected at startup.
fn parse_bind_addr(value: Option<String>) -> Result<IpAddr, String> {
    match value.as_deref().map(str::trim) {
        None | Some("") => Ok(DEFAULT_BIND_ADDR),
        Some(s) => s.parse().map_err(|_| {
            format!(
                "BIND_ADDR='{s}' is not a valid IP address. Use an interface IP \
                 (e.g. 0.0.0.0 for all interfaces, 127.0.0.1 for loopback-only \
                 behind a reverse proxy), or leave it unset to default to 0.0.0.0."
            )
        }),
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
    use super::{parse_bind_addr, parse_flag, DEFAULT_BIND_ADDR};
    use std::net::IpAddr;

    #[test]
    fn bind_addr_defaults_to_all_interfaces_when_unset_or_empty() {
        assert_eq!(DEFAULT_BIND_ADDR, IpAddr::from([0, 0, 0, 0]));
        assert_eq!(parse_bind_addr(None).unwrap(), DEFAULT_BIND_ADDR);
        assert_eq!(
            parse_bind_addr(Some(String::new())).unwrap(),
            DEFAULT_BIND_ADDR
        );
        // Whitespace-only is treated as unset (trimmed), not as garbage.
        assert_eq!(
            parse_bind_addr(Some("  ".to_string())).unwrap(),
            DEFAULT_BIND_ADDR
        );
    }

    #[test]
    fn bind_addr_parses_valid_literals() {
        assert_eq!(
            parse_bind_addr(Some("127.0.0.1".to_string())).unwrap(),
            IpAddr::from([127, 0, 0, 1])
        );
        // Surrounding whitespace is tolerated (common in .env files).
        assert_eq!(
            parse_bind_addr(Some(" 0.0.0.0 ".to_string())).unwrap(),
            IpAddr::from([0, 0, 0, 0])
        );
        // IPv6 loopback parses too.
        assert!(parse_bind_addr(Some("::1".to_string())).unwrap().is_ipv6());
    }

    #[test]
    fn bind_addr_rejects_garbage_with_clear_message() {
        // A hostname is not an IP literal — bind() needs an address, not a name.
        let err = parse_bind_addr(Some("localhost".to_string())).unwrap_err();
        assert!(err.contains("BIND_ADDR='localhost'"));
        assert!(err.contains("not a valid IP address"));
        // An out-of-range octet is rejected, not silently truncated.
        assert!(parse_bind_addr(Some("999.1.1.1".to_string())).is_err());
        // A host:port pair is not a bare IP — the port belongs in PORT.
        assert!(parse_bind_addr(Some("127.0.0.1:3002".to_string())).is_err());
    }

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
