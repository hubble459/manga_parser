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
    #[serde(default)]
    pub embedded_json: Option<EmbeddedJson>,
}

/// Page state embedded in the document itself (a `<script>` holding JSON)
#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize)]
pub struct EmbeddedJson {
    /// CSS selector for the element holding the serialized state.
    pub selector: String,
    /// Decodes the element's text before `format` reads it.
    #[serde(default)]
    pub unpack: Option<EmbeddedUnpack>,
    #[serde(default)]
    pub format: EmbeddedJsonFormat,
    /// RFC 6901 JSON Pointer to the array within the decoded state (`/` in a
    /// key is escaped as `~1`). Defaults to the root, which is where the
    /// `regex` format puts its items.
    #[serde(default)]
    pub array: String,
    /// Only for the `regex` format: matched against the element's text, each
    /// match becomes one item whose fields are the regex's named groups.
    #[serde(default, with = "serde_regex")]
    pub item_regex: Option<Regex>,
    /// Same as `FetchExternal::json_url_template`.
    #[serde(default)]
    pub url_template: Option<String>,
}

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize, Default, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddedJsonFormat {
    #[default]
    Json,
    /// `application/json+devalue`, as used by Astro and SvelteKit -- see
    /// `util::devalue`.
    Devalue,
    /// State that is neither JSON nor devalue (e.g. TanStack Start's seroval
    /// stream, a JS expression): items are pulled out with `item_regex`.
    Regex,
}

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddedUnpack {
    /// Dean Edwards' `eval(function(p,a,c,k,e,d)...)` packer -- see
    /// `util::packer`.
    Packer,
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
    /// Only used together with `json_array`, for APIs that return the array
    /// a page at a time.
    #[serde(default)]
    pub json_pagination: Option<JsonPagination>,
}

/// Offset pagination for a JSON `fetch_external`: the request is repeated
/// with `offset_param` set to the number of items fetched so far, for as long
/// as the boolean at `has_more` is true and a page adds items.
#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize)]
pub struct JsonPagination {
    pub offset_param: String,
    /// RFC 6901 JSON Pointer to a boolean, e.g. "/data/pagination/has_more".
    pub has_more: String,
}

fn default_method() -> String {
    return String::from("get");
}
