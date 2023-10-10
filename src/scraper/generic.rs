use std::{path::Path, sync::Mutex};

use config::{builder::DefaultState, ConfigBuilder, File};
use kuchiki::{traits::TendrilSink, NodeRef};
use reqwest::Url;

use crate::{
    error::ScrapeError,
    model::{Manga, MangaConfig},
    HTTP_CLIENT,
};

use super::MangaScraper;

pub struct GenericScraper {
    configs: Vec<MangaConfig>,
}

impl GenericScraper {
    pub fn new() -> Result<Self, ScrapeError> {
        Self::new_with_config_path(Path::new("configs"))
    }

    pub fn new_with_config_path(path: &Path) -> Result<Self, ScrapeError> {
        let mut configs = vec![];

        for file in path.read_dir()?.flatten() {
            let config = ConfigBuilder::<DefaultState>::default()
                .add_source(File::from(file.path()))
                .build()?;
            let manga_config = config.try_deserialize::<MangaConfig>()?;

            configs.push(manga_config);
        }
        Ok(Self { configs })
    }
}

impl MangaScraper for GenericScraper {
    fn manga(&self, url: &Url) -> Result<Manga, ScrapeError> {
        todo!()
    }

    fn chapter_images(&self, chapter_url: &Url) -> Result<Vec<Url>, ScrapeError> {
        todo!()
    }

    fn accepts(&self, url: &Url) -> bool {
        let response = futures::executor::block_on(async {
            let res = HTTP_CLIENT.get(url.clone()).send().await;
            if let Ok(res) = res {
                res.text().await.map_err(|e| e.into())
            } else {
                Err(res.unwrap_err())
            }
        });

        if let Ok(html) = response {
            let doc = html_to_doc(&html);

            // for config in self.configs {
            //     config;
            // }
            true
        } else {
            false
        }
    }
}

fn html_to_doc(html: &str) -> Result<NodeRef, ScrapeError> {
    std::panic::catch_unwind(|| kuchiki::parse_html().one(html))
        .map_err(|_e| ScrapeError::WebScrapingError("Could not parse HTML".to_string()))
}
