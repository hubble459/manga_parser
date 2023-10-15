use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache, HttpCacheOptions};
use reqwest::Client;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};

#[macro_use]
extern crate log;

pub mod core;
pub mod error;
pub mod model;
pub mod scraper;
pub mod util;

lazy_static::lazy_static! {
    pub static ref HTTP_CLIENT: ClientWithMiddleware = {
        ClientBuilder::new(Client::new())
            .with(Cache(HttpCache {
                mode: CacheMode::ForceCache,
                manager: CACacheManager::default(),
                options: HttpCacheOptions::default(),
            }))
            .build()
    };
}

#[cfg(test)]
mod tests {
    use reqwest::Url;

    use crate::{core::scraper_manager::ScraperManager, scraper::MangaScraper};

    #[tokio::test]
    async fn manga() {
        dotenvy::dotenv().ok();
        env_logger::builder()
            .is_test(true)
            .try_init()
            .ok();
        let manager = ScraperManager::new();

        let manga = manager
            .manga(&Url::parse("https://isekaiscan.top/manga/moshi-fanren").unwrap())
            .await;
        println!("manga: {:#?}", manga);
    }
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
