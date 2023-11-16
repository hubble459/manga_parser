use reqwest::{Request, Response, StatusCode};
use reqwest_middleware::{Middleware, Next, Error};
use task_local_extensions::Extensions;

pub struct CloudflareBypassMiddleware;

#[async_trait::async_trait]
impl Middleware for CloudflareBypassMiddleware {
    async fn handle(
        &self,
        req: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> Result<Response, Error> {
        // Cloning the request object before-the-fact is not ideal..
        // However, if the body of the request is not static, e.g of type `Bytes`,
        // the Clone operation should be of constant complexity and not O(N)
        // since the byte abstraction is a shared pointer over a buffer.
        let mut duplicate_request = req.try_clone().ok_or_else(|| {
            Error::Middleware(anyhow::anyhow!(
                "Request object is not clonable. Are you passing a streaming body?".to_string()
            ))
        })?;

        let response = next.clone().run(req, extensions).await;
        let status = response.as_ref().map_or_else(|e| e.status(), |v| Some(v.status()));

        if let Some(StatusCode::FORBIDDEN) = status {
            // Cloudflare IUAM
            debug!("Cloudflare IUAM detected; trying to bypass with cloudflare_bypasser");
            let mut bypasser = cloudflare_bypasser::Bypasser::default();
            for _ in 0..10 {
                if let Ok((cookie, user_agent)) =  bypasser.bypass(duplicate_request.url().as_str()) {
                    duplicate_request.headers_mut().insert(reqwest::header::COOKIE, cookie);
                    duplicate_request.headers_mut().insert(reqwest::header::USER_AGENT, user_agent);
                    return next.clone().run(duplicate_request, extensions).await;
                }
            }
        }

        response
    }
}