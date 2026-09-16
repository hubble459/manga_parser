use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use http::Extensions;
use reqwest::{Request, Response};
use reqwest_middleware::{Error, Middleware, Next, Result};

fn cooldown() -> Duration {
    Duration::from_secs(
        std::env::var("DEAD_HOST_COOLDOWN_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(300),
    )
}

/// Skips a request outright - retries and all - if its host recently failed to connect or
/// resolve. Without this, a genuinely dead domain still pays the full 3-attempt exponential
/// backoff (several seconds) on *every* request against it: every chapter in a prefetch,
/// every hostname in a sequential search, every manual refresh. One real failure marks the
/// host; everything else against it fails instantly until the cooldown expires, and a
/// success clears the mark immediately (a host can be genuinely flaky, not just dead).
///
/// Deliberately in-memory only, not persisted anywhere - see the module doc on why.
pub struct DeadHostGuard;

lazy_static::lazy_static! {
    static ref DEAD_HOSTS: Mutex<HashMap<String, Instant>> = Mutex::new(HashMap::new());
}

#[async_trait::async_trait]
impl Middleware for DeadHostGuard {
    async fn handle(&self, req: Request, extensions: &mut Extensions, next: Next<'_>) -> Result<Response> {
        let Some(host) = req.url().host_str().map(str::to_string) else {
            return next.run(req, extensions).await;
        };

        if let Some(&since) = DEAD_HOSTS.lock().unwrap().get(&host) {
            if since.elapsed() < cooldown() {
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
            Err(e) if is_connect_error(e) => {
                DEAD_HOSTS.lock().unwrap().insert(host, Instant::now());
            }
            Err(_) => {}
        }

        result
    }
}

/// `http-cache-reqwest` re-wraps the underlying `reqwest::Error` through `anyhow`, losing its
/// concrete type (and `RetryTransientMiddleware` wraps that again into "Request failed after N
/// retries") - by the time it reaches here, matching the error text is the only way left to
/// tell a DNS/connection failure apart from anything else that can fail a request.
fn is_connect_error(e: &Error) -> bool {
    match e {
        Error::Reqwest(e) => e.is_connect(),
        Error::Middleware(e) => {
            let msg = format!("{e:#}");
            msg.contains("dns error") || msg.contains("client error (Connect)")
        }
    }
}
