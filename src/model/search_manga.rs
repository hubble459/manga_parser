use chrono::{DateTime, Utc};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(PartialEq, simple_builder::Builder, Clone)]
pub struct SearchManga {
    pub url: reqwest::Url,
    pub title: String,
    pub cover_url: Option<reqwest::Url>,
    pub posted: Option<DateTime<Utc>>,
}
