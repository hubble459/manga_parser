use chrono::{DateTime, Utc};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(PartialEq)]
pub struct SearchManga {
    pub url: String,
    pub title: String,
    pub cover_url: Option<String>,
    pub posted: Option<DateTime<Utc>>,
}
