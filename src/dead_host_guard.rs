use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use http::Extensions;
use reqwest::{Request, Response};
use reqwest_middleware::{Error, Middleware, Next, Result};

fn base_cooldown_secs() -> u64 {
    std::env::var("DEAD_HOST_COOLDOWN_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30)
}

/// `base * 2^(failures - 1)`, uncapped - a host that stays dead is skipped for longer and
/// longer, forever. Saturates to `Duration::MAX` instead of overflowing.
fn cooldown(failures: u32) -> Duration {
    2u64.checked_pow(failures.saturating_sub(1))
        .and_then(|multiplier| base_cooldown_secs().checked_mul(multiplier))
        .map_or(Duration::MAX, Duration::from_secs)
}

/// Skips a request outright - retries and all - if its host recently failed at the transport
/// level (DNS, connect, TLS, timeout, ...). Without this, a genuinely dead domain still pays
/// the full 3-attempt exponential backoff (several seconds) on *every* request against it:
/// every chapter in a prefetch, every hostname in a sequential search, every manual refresh.
/// One real failure marks the host; everything else against it fails instantly until the
/// cooldown expires, and a success clears the mark immediately (a host can be genuinely
/// flaky, not just dead). Each failed probe after a cooldown expires doubles the next one.
///
/// Deliberately in-memory only, not persisted anywhere - see the module doc on why.
pub struct DeadHostGuard;

lazy_static::lazy_static! {
    static ref DEAD_HOSTS: Mutex<HashMap<String, DeadHost>> = Mutex::new(HashMap::new());
}

struct DeadHost {
    since: Instant,
    failures: u32,
}

impl DeadHost {
    fn cooling_down(&self) -> bool {
        self.since.elapsed() < cooldown(self.failures)
    }
}

#[async_trait::async_trait]
impl Middleware for DeadHostGuard {
    async fn handle(&self, req: Request, extensions: &mut Extensions, next: Next<'_>) -> Result<Response> {
        let Some(host) = req.url().host_str().map(str::to_string) else {
            return next.run(req, extensions).await;
        };

        if let Some(dead) = DEAD_HOSTS.lock().unwrap().get(&host) {
            if dead.cooling_down() {
                return Err(Error::Middleware(anyhow::anyhow!(
                    "{host} recently failed to connect - skipping until its cooldown expires"
                )));
            }
        }

        let result = next.run(req, extensions).await;

        match &result {
            Ok(_) => {
                DEAD_HOSTS.lock().unwrap().remove(&host);
            }
            Err(e) if is_transport_error(e) => {
                let mut dead_hosts = DEAD_HOSTS.lock().unwrap();
                let dead = dead_hosts.entry(host).or_insert(DeadHost {
                    since: Instant::now(),
                    failures: 0,
                });
                // Requests already in flight when the host got marked shouldn't each double
                // the cooldown - only a probe let through after it expired counts.
                if dead.failures == 0 || !dead.cooling_down() {
                    dead.since = Instant::now();
                    dead.failures = dead.failures.saturating_add(1);
                }
            }
            Err(_) => {}
        }

        result
    }
}

/// Anything that never got an HTTP response back. A response with an error status is `Ok`
/// here (the host is alive, it just said no), and a builder error is a bad request, not a
/// bad host. Everything reaching this middleware as an `Err` has already been through
/// `RetryTransientMiddleware`'s attempts.
fn is_transport_error(e: &Error) -> bool {
    match e {
        Error::Reqwest(e) => !e.is_builder(),
        Error::Middleware(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cooldown_doubles_without_overflowing() {
        let base = base_cooldown_secs();
        assert_eq!(cooldown(0), Duration::from_secs(base));
        assert_eq!(cooldown(1), Duration::from_secs(base));
        assert_eq!(cooldown(2), Duration::from_secs(base * 2));
        assert_eq!(cooldown(11), Duration::from_secs(base * 1024));
        assert_eq!(cooldown(64), Duration::MAX);
        assert_eq!(cooldown(u32::MAX), Duration::MAX);
    }
}
