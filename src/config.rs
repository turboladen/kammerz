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
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            password_hash: std::env::var("KAMMERZ_PASSWORD_HASH").ok().filter(|s| !s.is_empty()),
            anthropic_api_key: std::env::var("ANTHROPIC_API_KEY").ok().filter(|s| !s.is_empty()),
            secure_cookies: std::env::var("SECURE_COOKIES")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
        }
    }

    /// Whether auth is enforced. `true` when a password hash is configured;
    /// `false` is OPEN LAN-trust mode. Single source of truth for the open-mode
    /// predicate so the `RequireAuth` extractor and the auth handlers can't drift.
    pub fn auth_enabled(&self) -> bool {
        self.password_hash.is_some()
    }
}
