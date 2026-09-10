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
    #[serde(default = "default_method")]
    pub method: String,
    /// RFC 6901 JSON Pointer (e.g. "/data/chapters") to an array within the
    /// fetched response. When set, the response is parsed as JSON instead
    /// of HTML and flattened into synthetic elements — see
    /// `util::json::flatten_json_array_to_html`.
    #[serde(default)]
    pub json_array: Option<String>,
    /// Only used together with `json_array`, for JSON APIs that don't
    /// already include a directly usable URL/path. A template using
    /// `{host}`/`{url}` (same as `url` above) plus `{<field>}`
    /// placeholders for the item's own flattened fields. The computed
    /// result is exposed as attribute `data-__url`.
    #[serde(default)]
    pub json_url_template: Option<String>,
}

fn default_method() -> String {
    return String::from("get");
}