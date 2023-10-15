use reqwest::Url;

use crate::{
    error::ScrapeError,
    model::Manga,
    scraper::{generic::GenericScraper, MangaScraper},
};

pub struct ScraperManager {
    scrapers: Vec<Box<dyn MangaScraper>>,
}

impl ScraperManager {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for ScraperManager {
    fn default() -> Self {
        Self {
            scrapers: vec![Box::new(GenericScraper::new().unwrap())],
        }
    }
}

#[async_trait::async_trait]
impl MangaScraper for ScraperManager {
    async fn manga(&self, url: &Url) -> Result<Manga, ScrapeError> {
        let mut err = None;
        for scraper in self.scrapers.iter() {
            if scraper.accepts(url).await {
                let manga = scraper.manga(url).await;
                match manga {
                    Ok(manga) => return Ok(manga),
                    Err(e) => {
                        error!("Error parsing manga: {:?}", e);
                        err = Some(e);
                    }
                };
            }
        }

        Err(err.unwrap_or(ScrapeError::WebsiteNotSupported(url.to_string())))
    }

    async fn chapter_images(&self, chapter_url: &Url) -> Result<Vec<Url>, ScrapeError> {
        let mut err = None;
        for scraper in self.scrapers.iter() {
            if scraper.accepts(chapter_url).await {
                let manga = scraper.chapter_images(chapter_url).await;
                match manga {
                    Ok(images) => return Ok(images),
                    Err(e) => {
                        error!("Error parsing images: {:?}", e);
                        err = Some(e);
                    }
                };
            }
        }

        Err(err.unwrap_or(ScrapeError::WebsiteNotSupported(chapter_url.to_string())))
    }

    async fn accepts(&self, _url: &Url) -> bool {
        true
    }
}
