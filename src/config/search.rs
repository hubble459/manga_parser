use serde::Deserialize;

use super::{string_selector::StringSelectors, string_selector_options::CleanupOption};

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
    #[serde(default)]
    pub hostnames: Vec<String>,
    pub search_url: String,
    #[serde(default)]
    pub query_format: Vec<CleanupOption>,
    pub selectors: SearchSelectors,
}
