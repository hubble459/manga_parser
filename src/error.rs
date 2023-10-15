use core::fmt;
use std::{sync::{PoisonError, MutexGuard}, fmt::Display};

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

    #[error("Selector error: {0}")]
    SelectorError(String),

    #[error("Website is not supported: {0}")]
    WebsiteNotSupported(String),
}

impl serde::de::Error for ScrapeError {
    fn custom<T: Display>(msg: T) -> Self {
        ScrapeError::ConfigDeserializeError(msg.to_string())
    }
}

