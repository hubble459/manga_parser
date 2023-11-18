use super::Chapter;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(PartialEq, simple_builder::Builder, Clone)]
pub struct Manga {
    pub url: reqwest::Url,
    pub title: String,
    pub description: String,
    pub cover_url: Option<reqwest::Url>,
    pub status: Option<String>,
    pub is_ongoing: bool,
    #[builder(default)]
    pub authors: Vec<String>,
    #[builder(default)]
    pub genres: Vec<String>,
    #[builder(default)]
    pub alternative_titles: Vec<String>,
    pub chapters: Vec<Chapter>,
}
