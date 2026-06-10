//! Brute-force guard for `POST /api/auth/login`.
//!
//! A single shared password reachable over the LAN/VPN is otherwise bounded only
//! by argon2 cost, so we throttle **failed** login attempts per client IP. Crucial
//! properties (see `auth::handlers::login`):
//!
//! - The limiter is consulted *before* the password is verified, so a locked-out
//!   IP cannot test any password (correct or not) until its window elapses — that
//!   is what actually bounds a brute-forcer's guess rate.
//! - Only a *wrong* password records against the limiter; a correct password
//!   clears the IP's record. So a legitimate user is never locked out after they
//!   finally type the right password, and a client doing many *successful* logins
//!   (e.g. the Playwright smoke suite, all from one IP) is never throttled.
//!
//! The per-IP key comes from the `ConnectInfo<SocketAddr>` that `main.rs` installs
//! via `into_make_service_with_connect_info`.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Failed attempts allowed per window before an IP is locked out. Comfortably
/// covers a legitimate user fat-fingering the password a few times.
pub const LOGIN_MAX_FAILURES: u32 = 5;

/// Sliding-window length (and lockout duration) for the failure budget. After
/// `LOGIN_MAX_FAILURES` failures within this window, the IP is locked until the
/// window — measured from its first failure — elapses.
pub(crate) const LOGIN_WINDOW_SECONDS: u64 = 60;

fn window() -> Duration {
    Duration::from_secs(LOGIN_WINDOW_SECONDS)
}

/// Failures seen from one IP within the current window.
struct FailRecord {
    count: u32,
    /// Start of the current window (time of the first failure in it).
    first_at: Instant,
}

/// Result of consulting the limiter before verifying a password.
pub enum Decision {
    Allow,
    /// IP is over its failure budget; reject without verifying. `retry_after_secs`
    /// is the (rounded-up) seconds until the window resets.
    Locked { retry_after_secs: u64 },
}

/// Per-IP failed-login tracker. Cheap `Mutex<HashMap>` — login is low-frequency
/// and the keyspace is bounded by active-window client IPs (expired entries are
/// pruned opportunistically), so no background cleanup task is needed.
#[derive(Default)]
pub struct LoginRateLimiter {
    inner: Mutex<HashMap<IpAddr, FailRecord>>,
}

impl LoginRateLimiter {
    /// Consult the limiter for `ip`. Call this *before* verifying the password.
    /// Expired records are dropped here so a stale window can't keep an IP locked.
    pub fn check(&self, ip: IpAddr) -> Decision {
        let mut map = self.inner.lock().unwrap();
        match map.get(&ip) {
            Some(rec) if rec.first_at.elapsed() >= window() => {
                map.remove(&ip); // window elapsed → fresh start
                Decision::Allow
            }
            Some(rec) if rec.count >= LOGIN_MAX_FAILURES => {
                let remaining = window().saturating_sub(rec.first_at.elapsed());
                // Round up so Retry-After never tells a client to retry too early.
                let secs = remaining.as_secs() + u64::from(remaining.subsec_nanos() > 0);
                Decision::Locked {
                    retry_after_secs: secs.max(1),
                }
            }
            _ => Decision::Allow,
        }
    }

    /// Record a wrong-password attempt from `ip`.
    pub fn record_failure(&self, ip: IpAddr) {
        let mut map = self.inner.lock().unwrap();
        map.retain(|_, rec| rec.first_at.elapsed() < window()); // bound the keyspace
        map.entry(ip)
            .or_insert_with(|| FailRecord {
                count: 0,
                first_at: Instant::now(),
            })
            .count += 1;
    }

    /// Clear `ip`'s record after a successful login.
    pub fn record_success(&self, ip: IpAddr) {
        self.inner.lock().unwrap().remove(&ip);
    }
}
