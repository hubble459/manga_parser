use crate::{
    error::ScrapeError,
    model::{Manga, SearchManga},
};
use reqwest::Url;

pub mod generic;

pub trait MangaScraper {
    fn manga(&self, url: &Url) -> Result<Manga, ScrapeError>;
    fn chapter_images(&self, chapter_url: &Url) -> Result<Vec<Url>, ScrapeError>;

    fn accepts(&self, url: &Url) -> bool;
}

pub trait SearchScraper {
    fn search(&self) -> Result<Vec<SearchManga>, ScrapeError>;

    fn accepts(&self, url: &Url) -> bool;
}
