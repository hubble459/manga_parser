use reqwest::Url;

use crate::{
    error::ScrapeError,
    model::Manga,
    scraper::{self, generic::GenericScraper, MangaScraper},
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
            scrapers: vec![
                Box::new(GenericScraper::new().unwrap()),
            ],
        }
    }
}

impl MangaScraper for ScraperManager {
    fn manga(&self, url: &Url) -> Result<Manga, ScrapeError> {
        let mut err = None;
        for scraper in self.scrapers.iter() {
            if scraper.accepts(url) {
                let manga = scraper.manga(url);
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

    fn chapter_images(&self, chapter_url: &Url) -> Result<Vec<Url>, ScrapeError> {
        todo!()
    }

    fn accepts(&self, url: &Url) -> bool {
        true
    }
}
