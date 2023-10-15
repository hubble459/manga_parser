use crate::{
    error::ScrapeError,
    model::{Manga, SearchManga},
};
use reqwest::Url;

pub mod generic;
pub mod scraper_manager;

#[async_trait::async_trait]
pub trait MangaScraper: Send + Sync {
    async fn accepts(&self, url: &Url) -> bool;
    async fn manga(&self, url: &Url) -> Result<Manga, ScrapeError>;
    async fn chapter_images(&self, chapter_url: &Url) -> Result<Vec<Url>, ScrapeError>;
}

#[async_trait::async_trait]
pub trait SearchScraper: Send + Sync {
    async fn accepts(&self, url: &Url) -> bool;
    async fn search(&self) -> Result<Vec<SearchManga>, ScrapeError>;
}
