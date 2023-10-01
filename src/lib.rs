use std::{collections::HashMap, sync::Mutex};

use config::Config;

lazy_static::lazy_static! {
    static ref CONFIGS: Mutex<HashMap<String, Config>> = Mutex::new(HashMap::new());
}

pub mod core;
pub mod error;
pub mod model;
pub mod util;

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::Duration;

    use crate::{core::config::ConfigWatcher, CONFIGS};

    #[test]
    fn load_configs() -> Result<(), Box<dyn std::error::Error>> {
        let config_dir = Path::new("configs");
        let mut config_watcher = ConfigWatcher::new(config_dir.to_path_buf());

        // Load initial configurations
        config_watcher.load_configs()?;

        // Create a channel for communication between threads
        let (tx, rx) = std::sync::mpsc::channel();

        // Spawn a thread to run the directory watcher
        std::thread::spawn(move || {
            config_watcher
                .watch_config_dir(tx, rx)
                .expect("Watcher error");
        });

        let manga = crate::core::scraper::scrape_manga(
            "https://isekaiscan.top/manga/god-of-martial-arts",
            include_str!("../tests/fragments/madara.html"),
        );
        println!("manga: {:#?}", manga);

        loop {
            std::thread::sleep(Duration::from_secs(2));

            let configs = CONFIGS.lock().unwrap();
            for (_path, _config) in configs.iter() {
                // println!("{path}: {:#?}", config.clone().try_deserialize::<MangaConfig>());
            }
        }
    }
}
