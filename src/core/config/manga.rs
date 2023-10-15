use serde::Deserialize;

use super::{array_selector::ArraySelectors, chapter::Chapter, string_selector::StringSelectors};

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize)]
pub struct Manga {
    pub title: StringSelectors,
    pub description: StringSelectors,
    pub cover_url: Option<StringSelectors>,
    pub status: Option<StringSelectors>,
    pub authors: Option<ArraySelectors>,
    pub genres: Option<ArraySelectors>,
    pub alt_titles: Option<ArraySelectors>,
    pub chapter: Chapter,
}
