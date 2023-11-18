use serde::Deserialize;

use self::{accept::Accept, manga::Manga, search::SearchConfig, images::Images};

pub mod accept;
pub mod array_selector;
pub mod array_selector_options;
pub mod chapter;
pub mod manga;
pub mod search;
pub mod images;
pub mod string_selector;
pub mod string_selector_options;

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize)]
pub struct MangaScraperConfig {
    pub name: String,
    pub accept: Accept,
    pub manga: Manga,
    pub images: Images,
    #[serde(default)]
    pub search: Vec<SearchConfig>,
    pub date_formats: Vec<String>,
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use config::{builder::DefaultState, ConfigBuilder, File};

    #[test]
    fn test_selector_deserialization() {
        let manga_scraper_config = ConfigBuilder::<DefaultState>::default()
            .add_source(File::from(Path::new("configs/madara.yaml")))
            .build()
            .unwrap()
            .try_deserialize::<super::MangaScraperConfig>()
            .unwrap();

        println!("{:#?}", manga_scraper_config);
    }
}
