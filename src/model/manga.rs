use super::Chapter;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(PartialEq)]
pub struct Manga {
    pub title: String,
    pub description: String,
    pub cover_url: String,
    pub status: String,
    pub authors: Vec<String>,
    pub genres: Vec<String>,
    pub alternative_titles: Vec<String>,
    pub chapters: Vec<Chapter>,
}
