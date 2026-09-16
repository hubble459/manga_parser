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
    /// RFC 6901 JSON Pointer (e.g. "/data/results") to an array within the
    /// fetched search response. When set, the response is parsed as JSON
    /// instead of HTML and flattened into synthetic elements — see
    /// `util::json::flatten_json_array_to_html`.
    #[serde(default)]
    pub json_array: Option<String>,
    /// Only used together with `json_array`, for JSON APIs that don't
    /// already include a directly usable URL/path. A template using
    /// `{host}`/`{url}` (search page context) plus `{<field>}`
    /// placeholders for the item's own flattened fields. The computed
    /// result is exposed as attribute `data-__url`.
    #[serde(default)]
    pub json_url_template: Option<String>,
}
