use super::Chapter;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(PartialEq)]
pub struct Manga {
    pub url: String,
    pub title: String,
    pub description: String,
    pub cover_url: Option<String>,
    pub status: Option<String>,
    pub is_ongoing: bool,
    pub authors: Vec<String>,
    pub genres: Vec<String>,
    pub alternative_titles: Vec<String>,
    pub chapters: Vec<Chapter>,
}
