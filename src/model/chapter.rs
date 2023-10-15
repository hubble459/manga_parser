use chrono::{DateTime, Utc};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(PartialEq)]
pub struct Chapter {
    pub url: reqwest::Url,
    pub number: f32,
    pub title: String,
    pub date: Option<DateTime<Utc>>,
}
