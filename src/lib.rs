use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache, HttpCacheOptions};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware, RequestBuilder};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};

use crate::util::cloudflare_bypass_middleware::CloudflareBypassMiddleware;

#[macro_use]
extern crate log;

pub use reqwest::Url;

pub mod config;
pub mod error;
pub mod model;
pub mod scraper;
pub mod util;

lazy_static::lazy_static! {
    pub static ref HTTP_CLIENT: ClientWithMiddleware = {
        // Retry up to 3 times with increasing intervals between attempts.
        let retry_policy = ExponentialBackoff::builder()
            .build_with_max_retries(3);
        ClientBuilder::new(
            reqwest::ClientBuilder::default()
                .cookie_store(true)
                .build()
                .unwrap()
        )
            .with(Cache(HttpCache {
                mode: CacheMode::ForceCache,
                manager: CACacheManager::default(),
                options: HttpCacheOptions::default(),
            }))
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .with(CloudflareBypassMiddleware)
            .with_init(|request: RequestBuilder| -> RequestBuilder {
                request
                    .header(reqwest::header::USER_AGENT, fake_user_agent::get_rua())
                    .header(reqwest::header::ACCEPT, "*/*")
            })
            .build()
    };
}

// #[cfg(test)]
// mod tests {
//     use std::path::Path;
//     use std::time::Duration;

//     use crate::{core::config::ConfigWatcher, CONFIGS};

//     #[test]
//     fn load_configs() -> Result<(), Box<dyn std::error::Error>> {
//         let config_dir = Path::new("configs");
//         let mut config_watcher = ConfigWatcher::new(config_dir.to_path_buf());

//         // Load initial configurations
//         config_watcher.load_configs()?;

//         // Create a channel for communication between threads
//         let (tx, rx) = std::sync::mpsc::channel();

//         // Spawn a thread to run the directory watcher
//         std::thread::spawn(move || {
//             config_watcher
//                 .watch_config_dir(tx, rx)
//                 .expect("Watcher error");
//         });

//         let manga = crate::core::scraper::scrape_manga(
//             "https://isekaiscan.top/manga/god-of-martial-arts",
//             include_str!("../tests/fragments/madara.html"),
//         );
//         println!("manga: {:#?}", manga);

//         loop {
//             std::thread::sleep(Duration::from_secs(2));

//             let configs = CONFIGS.lock().unwrap();
//             for (_path, _config) in configs.iter() {
//                 // println!("{path}: {:#?}", config.clone().try_deserialize::<MangaConfig>());
//             }
//         }
//     }
// }
