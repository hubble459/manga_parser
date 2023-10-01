use thiserror::Error;

#[derive(Error, Debug)]
pub enum MangaError {
    #[error("Reqwest error: {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    ConfigError(#[from] config::ConfigError),

    #[error("Web scraping error: {0}")]
    WebScrapingError(String),

    #[error("Selector error: {0}")]
    SelectorError(String),
}
