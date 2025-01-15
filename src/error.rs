use std::{collections::HashMap, fmt::Display};

use thiserror::Error;

#[derive(Error, Debug, strum::AsRefStr)]
pub enum ScrapeError {
    #[error("Reqwest error: {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("Reqwest error: {0}")]
    ReqwestMiddlewareError(#[from] reqwest_middleware::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    ConfigError(#[from] config::ConfigError),

    #[error("Configuration error: {0}")]
    ConfigDeserializeError(String),

    #[error("Web scraping error: {0}")]
    WebScrapingError(String),

    #[error("URL parsing error: {0}")]
    NotAValidURL(String),

    #[error("Selector error: {0}")]
    SelectorError(String),

    #[error("Website is not supported: {0}")]
    WebsiteNotSupported(String),

    #[error("Search is not supported: {0:?}")]
    SearchNotSupported(Vec<String>),

    #[error("Manga scraping errors: {0:#?}")]
    MultipleScrapingErrors(HashMap<String, ScrapeError>),

    #[error("Cloudflare I'm Under Attack Mode")]
    CloudflareIUAM,

    #[error("Unknown error: {0}")]
    UnknownError(#[from] Box<dyn std::error::Error + Send>),

    #[error("Unknown error: {0}")]
    UnknownErrorStr(&'static str),

    #[error("Missing manga title")]
    MissingMangaTitle,
}

impl serde::de::Error for ScrapeError {
    fn custom<T: Display>(msg: T) -> Self {
        ScrapeError::ConfigDeserializeError(msg.to_string())
    }
}
