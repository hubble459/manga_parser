use serde::Deserialize;

use super::{array_selector::ArraySelector, chapter::Chapter, string_selector::StringSelector};

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize)]
pub struct Manga {
    pub title: StringSelector,
    pub description: StringSelector,
    pub cover_url: Option<StringSelector>,
    pub status: Option<StringSelector>,
    pub authors: Option<ArraySelector>,
    pub genres: Option<ArraySelector>,
    pub alt_titles: Option<ArraySelector>,
    pub chapter: Chapter,
}
