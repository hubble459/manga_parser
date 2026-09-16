use futures::future::join_all;
use reqwest::Url;

use crate::{
    error::ScrapeError,
    model::{Manga, SearchManga},
    scraper::{generic::GenericScraper, mangadex::MangaDex, MangaScraper},
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
                Box::new(MangaDex::new()),
                Box::new(GenericScraper::new().unwrap()),
            ],
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

    async fn search(
        &self,
        query: &str,
        hostnames: &[String],
    ) -> Result<Vec<SearchManga>, ScrapeError> {
        // One hostname's search is independent network I/O + parsing from
        // every other hostname's, so search them concurrently instead of
        // one after another -- `join_all` still returns results in
        // `hostnames` order, so the merge below sees the same per-hostname
        // outcomes (and "last error wins" precedence) as the old sequential
        // loop would have, just without paying for each hostname's latency
        // back-to-back.
        let per_hostname = join_all(hostnames.iter().map(|hostname| async move {
            let mut err = None;
            let mut results = vec![];
            for scraper in self.scrapers.iter() {
                if scraper.search_accepts(hostname) {
                    match scraper.search(query, &[hostname.to_string()]).await {
                        Ok(mut r) => results.append(&mut r),
                        Err(e) => {
                            if !matches!(e, ScrapeError::SearchNotSupported(_)) {
                                error!("Error parsing search: {:?}", e);
                                err = Some(e);
                            }
                        }
                    };
                }
            }
            (results, err)
        }))
        .await;

        let mut err = None;
        let mut search_results = vec![];
        for (mut results, hostname_err) in per_hostname {
            search_results.append(&mut results);
            if hostname_err.is_some() {
                err = hostname_err;
            }
        }

        if !search_results.is_empty() || err.is_none() {
            return Ok(search_results);
        }

        Err(err.unwrap_or(ScrapeError::SearchNotSupported(hostnames.to_vec())))
    }

    fn searchable_hostnames(&self) -> Vec<String> {
        let mut hostnames = vec![];
        for scraper in self.scrapers.iter() {
            hostnames.append(&mut scraper.searchable_hostnames());
        }
        hostnames.sort();
        hostnames
    }

    fn search_accepts(&self, hostname: &str) -> bool {
        self.searchable_hostnames().binary_search(&hostname.to_string()).is_ok()
    }
}
