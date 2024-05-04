use crate::{
    error::ScrapeError,
    model::{Manga, SearchManga},
};
use reqwest::Url;

pub mod generic;
pub mod mangadex;
pub mod scraper_manager;

#[async_trait::async_trait]
pub trait MangaScraper: Send + Sync {
    async fn accepts(&self, url: &Url) -> bool;
    async fn manga(&self, url: &Url) -> Result<Manga, ScrapeError>;
    async fn chapter_images(&self, chapter_url: &Url) -> Result<Vec<Url>, ScrapeError>;

    async fn search(&self, query: &str, hostnames: &[String]) -> Result<Vec<SearchManga>, ScrapeError>;
    fn search_accepts(&self, hostname: &str) -> bool;
    fn searchable_hostnames(&self) -> Vec<String>;
}
