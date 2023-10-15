use regex::Regex;
use serde::Deserialize;

use super::string_selector::StringSelector;

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
    pub base: StringSelector,
    pub url: StringSelector,
    pub title: StringSelector,
    #[serde(default)]
    pub cover_url: Option<StringSelector>,
    #[serde(default)]
    pub posted: Option<StringSelector>,
}

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize)]
pub struct SearchConfig {
    pub search_url: String,
    #[serde(default)]
    pub query_format: Option<SearchQueryFormat>,
    pub selectors: SearchSelectors,
}
