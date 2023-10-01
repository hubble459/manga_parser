#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(serde::Deserialize, Clone)]
pub struct MangaConfig {
    pub title_selector: String,
    pub description_selector: String,
    pub cover_url_selector: String,
    pub status_selector: String,
    pub authors_selector: String,
    pub genres_selector: String,
    pub alternative_titles_selector: String,
    pub chapters_selector: String,
    pub chapter_number_selector: String,
    pub chapter_title_selector: String,
    pub chapter_date_selector: String,
    pub chapter_href_selector: String,
    pub date_formats: Vec<String>,

    pub chapter_external_selectors: Vec<ChapterExternalSelector>,
}

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(serde::Deserialize, Clone)]
pub struct ChapterExternalSelector {
    pub id: String,
    pub regex: String,
    pub url: String,
}