use regex::Regex;
use serde::Deserialize;

use super::string_selector::StringSelectors;

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize)]
pub struct SearchQueryFormat {
    #[serde(deserialize_with = "serde_regex::deserialize")]
    pub replace_regex: Regex,
    pub replace_with: String,
}

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize)]
pub struct SearchSelectors {
    pub base: StringSelectors,
    pub url: StringSelectors,
    pub title: StringSelectors,
    #[serde(default)]
    pub cover_url: Option<StringSelectors>,
    #[serde(default)]
    pub posted: Option<StringSelectors>,
}

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize)]
pub struct SearchConfig {
    pub search_url: String,
    #[serde(default)]
    pub query_format: Option<SearchQueryFormat>,
    pub selectors: SearchSelectors,
}
