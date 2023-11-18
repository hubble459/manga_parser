
use std::{fmt::Display, collections::HashMap};

use thiserror::Error;

#[derive(Error, Debug)]
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
}

impl serde::de::Error for ScrapeError {
    fn custom<T: Display>(msg: T) -> Self {
        ScrapeError::ConfigDeserializeError(msg.to_string())
    }
}

