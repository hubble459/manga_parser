use regex::Regex;
use serde::Deserialize;

use super::string_selector::StringSelectors;

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize)]
pub struct Chapter {
    pub base: StringSelectors,
    pub title: StringSelectors,
    #[serde(default)]
    pub number: Option<StringSelectors>,
    #[serde(default)]
    pub date: Option<StringSelectors>,
    pub url: StringSelectors,
    #[serde(default)]
    pub fetch_external: Vec<FetchExternal>,
}

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize)]
pub struct FetchExternal {
    pub id: StringSelectors,
    #[serde(deserialize_with = "serde_regex::deserialize")]
    pub regex: Regex,
    pub url: String,
}
