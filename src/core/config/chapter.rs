use regex::Regex;
use serde::Deserialize;

use super::string_selector::StringSelector;

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize)]
pub struct Chapter {
    pub base: StringSelector,
    pub title: StringSelector,
    #[serde(default)]
    pub number: Option<StringSelector>,
    #[serde(default)]
    pub date: Option<StringSelector>,
    pub url: StringSelector,
    #[serde(default)]
    pub fetch_external: Vec<ExternalFetch>,
}

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize)]
pub struct ExternalFetch {
    pub id: StringSelector,
    #[serde(deserialize_with = "serde_regex::deserialize")]
    pub regex: Regex,
    pub url: String,
}
