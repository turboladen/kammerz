use std::net::{IpAddr, Ipv4Addr};

/// Bind address used when `BIND_ADDR` is unset in AUTHED mode: `0.0.0.0` (all
/// interfaces), preserving the behavior from before `BIND_ADDR` existed.
const DEFAULT_BIND_ADDR: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);

/// Bind address used when `BIND_ADDR` is unset in OPEN (no-password) mode:
/// `127.0.0.1` (loopback only). Open mode leaves the whole catalog, the DB
/// backup download, and settings writes unauthenticated, so an operator who
/// forgets `KAMMERZ_PASSWORD_HASH` must not silently expose them to the LAN/VPN
/// — they have to opt into off-host reachability with an explicit `BIND_ADDR`
/// (kammerz-vlyu.13).
const LOOPBACK_BIND_ADDR: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

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
    /// IP the listener binds to. When `BIND_ADDR` is unset the default depends
    /// on the auth posture: AUTHED mode keeps `0.0.0.0` (all interfaces, the
    /// pre-`BIND_ADDR` behavior); OPEN (no-password) mode defaults to `127.0.0.1`
    /// (loopback only) so a forgotten password hash can't expose the catalog
    /// off-host (kammerz-vlyu.13). An explicit `BIND_ADDR` always wins in either
    /// mode — set `0.0.0.0` to expose an open-mode instance deliberately.
    pub bind_addr: IpAddr,
}

impl Default for AppConfig {
    /// All-defaults config: OPEN auth, no API key, insecure cookies, peer-IP
    /// rate limiting, and the `0.0.0.0` bind. Tests spread `..Default::default()`
    /// over this so adding a field here doesn't churn every config literal. Note
    /// the bind default here is `0.0.0.0` regardless of auth — the open-mode
    /// loopback rule (kammerz-vlyu.13) lives only in `from_env`/`resolve_bind_addr`;
    /// tests use `oneshot` and never actually bind, so it doesn't matter here.
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
        let password_hash = std::env::var("KAMMERZ_PASSWORD_HASH")
            .ok()
            .filter(|s| !s.is_empty());
        // The unset-BIND_ADDR default depends on the auth posture: authed keeps
        // 0.0.0.0, open mode binds loopback-only (kammerz-vlyu.13). A malformed
        // explicit value still fails fast (rather than silently defaulting)
        // because a typo'd bind would re-expose the catalog — the footgun
        // BIND_ADDR exists to close.
        let bind_addr =
            resolve_bind_addr(std::env::var("BIND_ADDR").ok(), password_hash.is_some())?;
        Ok(Self {
            password_hash,
            anthropic_api_key: std::env::var("ANTHROPIC_API_KEY")
                .ok()
                .filter(|s| !s.is_empty()),
            secure_cookies: env_flag("SECURE_COOKIES"),
            trust_proxy: env_flag("KAMMERZ_TRUST_PROXY"),
            bind_addr,
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

/// Resolve the effective bind address from the (optional) `BIND_ADDR` value and
/// the auth posture. An explicit, non-empty `BIND_ADDR` always wins — it's
/// parsed by [`parse_bind_addr`] and a bad literal errors. When unset/empty the
/// default depends on `auth_enabled`: authed keeps `0.0.0.0` (all interfaces),
/// OPEN (no-password) mode binds `127.0.0.1` (loopback only) so a forgotten
/// password hash can't silently expose the catalog off-host (kammerz-vlyu.13).
/// Pure over its inputs so it's unit-testable without mutating process env.
fn resolve_bind_addr(value: Option<String>, auth_enabled: bool) -> Result<IpAddr, String> {
    // Whether the operator supplied a real value (not unset / whitespace-only).
    let has_explicit = value
        .as_deref()
        .map(str::trim)
        .is_some_and(|s| !s.is_empty());
    if has_explicit {
        parse_bind_addr(value) // explicit value wins in either mode
    } else if auth_enabled {
        Ok(DEFAULT_BIND_ADDR)
    } else {
        Ok(LOOPBACK_BIND_ADDR)
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
    use super::{
        DEFAULT_BIND_ADDR, LOOPBACK_BIND_ADDR, parse_bind_addr, parse_flag, resolve_bind_addr,
    };
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
    fn resolve_bind_defaults_by_auth_posture_when_unset() {
        assert_eq!(LOOPBACK_BIND_ADDR, IpAddr::from([127, 0, 0, 1]));
        // Authed + unset → historical all-interfaces default.
        assert_eq!(
            resolve_bind_addr(None, true).unwrap(),
            DEFAULT_BIND_ADDR,
            "authed mode keeps 0.0.0.0 when BIND_ADDR is unset"
        );
        // Open + unset → loopback only (the kammerz-vlyu.13 default).
        assert_eq!(
            resolve_bind_addr(None, false).unwrap(),
            LOOPBACK_BIND_ADDR,
            "open mode binds loopback-only when BIND_ADDR is unset"
        );
        // Empty / whitespace-only is treated as unset, so the same posture rule
        // applies (not garbage → not an error).
        assert_eq!(
            resolve_bind_addr(Some("   ".to_string()), false).unwrap(),
            LOOPBACK_BIND_ADDR
        );
        assert_eq!(
            resolve_bind_addr(Some(String::new()), true).unwrap(),
            DEFAULT_BIND_ADDR
        );
    }

    #[test]
    fn resolve_bind_explicit_value_wins_in_either_mode() {
        // An explicit BIND_ADDR overrides the posture default — including a
        // deliberate open-mode exposure on all interfaces.
        assert_eq!(
            resolve_bind_addr(Some("0.0.0.0".to_string()), false).unwrap(),
            IpAddr::from([0, 0, 0, 0]),
            "explicit 0.0.0.0 exposes an open-mode instance on purpose"
        );
        // And an explicit loopback in authed mode is honored too.
        assert_eq!(
            resolve_bind_addr(Some("127.0.0.1".to_string()), true).unwrap(),
            IpAddr::from([127, 0, 0, 1])
        );
        // A malformed explicit value still fails fast regardless of posture.
        assert!(resolve_bind_addr(Some("localhost".to_string()), false).is_err());
        assert!(resolve_bind_addr(Some("999.1.1.1".to_string()), true).is_err());
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
