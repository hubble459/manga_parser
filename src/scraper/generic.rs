use std::{collections::HashMap, ops::Deref, path::Path, time::Duration};

use chrono::{DateTime, Utc};
use config::{builder::DefaultState, ConfigBuilder, File};
use convert_case::Casing;
use kuchiki::{traits::TendrilSink, NodeRef};
use reqwest::{Method, StatusCode, Url};

use crate::{
    core::config::{
        array_selector::ArraySelectors, chapter::FetchExternal, string_selector::StringSelectors,
        string_selector_options::StringSelection, MangaScraperConfig,
    },
    error::ScrapeError,
    model::{Chapter, Manga},
    util::kuchiki_elements::ElementsTrait,
    HTTP_CLIENT,
};

use super::MangaScraper;

pub struct GenericScraper {
    configs: Vec<MangaScraperConfig>,
}

impl GenericScraper {
    pub fn new() -> Result<Self, ScrapeError> {
        Self::new_with_config_path(Path::new("configs"))
    }

    pub fn new_with_config_path(path: &Path) -> Result<Self, ScrapeError> {
        let mut configs = vec![];

        for file in path.read_dir()?.flatten() {
            if matches!(
                file.path()
                    .extension()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap(),
                "yaml" | "yml"
            ) {
                let config = ConfigBuilder::<DefaultState>::default()
                    .add_source(File::from(file.path()))
                    .build()?;
                let manga_config = config.try_deserialize::<MangaScraperConfig>()?;

                configs.push(manga_config);
            }
        }
        Ok(Self { configs })
    }

    fn select_required_url(
        &self,
        url: &Url,
        selector: &StringSelectors,
        doc: DocWrapper,
    ) -> Result<reqwest::Url, ScrapeError> {
        self.select_url(url, selector, doc)?
            .ok_or(ScrapeError::WebScrapingError(format!(
                "Missing required url with selectors: {:?}",
                selector.selectors
            )))
    }

    fn select_url(
        &self,
        url: &Url,
        selector: &StringSelectors,
        doc: DocWrapper,
    ) -> Result<Option<reqwest::Url>, ScrapeError> {
        let url_string = self.select_string(selector, doc)?;
        if let Some(url_string) = url_string {
            Ok(Some(
                url.join(&url_string)
                    .map_err(|e| ScrapeError::NotAValidURL(e.to_string()))?,
            ))
        } else {
            Ok(None)
        }
    }

    fn select_required_string(
        &self,
        selector: &StringSelectors,
        doc: DocWrapper,
    ) -> Result<String, ScrapeError> {
        self.select_string(selector, doc)?
            .ok_or(ScrapeError::WebScrapingError(format!(
                "Missing required field with selectors: {:?}",
                selector.selectors
            )))
    }

    fn select_string(
        &self,
        selectors: &StringSelectors,
        doc: DocWrapper,
    ) -> Result<Option<String>, ScrapeError> {
        for selector in &selectors.selectors {
            let elements = doc
                .select(&selector.selector)
                .map_err(|_| ScrapeError::SelectorError(selector.selector.clone()))?;

            let mut text = match &selector.options.text_selection {
                StringSelection::AllText { join_with } => elements.all_text(join_with),
                StringSelection::OwnText => elements.own_text(),
                StringSelection::Attributes(attrs) => {
                    elements.attr_first_of(attrs).unwrap_or_default()
                }
            };

            if !text.is_empty() {
                // Cleanup the text
                text = text.trim().to_string();
                for cleanup in &selector.options.cleanup {
                    text = cleanup
                        .replace_regex
                        .replace_all(&text, &cleanup.replace_with)
                        .to_string();
                }
                // Fix capitalization
                text = match &selector.options.fix_capitalization {
                    crate::core::config::string_selector_options::FixCapitalization::Title => {
                        text.to_case(convert_case::Case::Title)
                    }
                    crate::core::config::string_selector_options::FixCapitalization::Skip => text,
                };

                return Ok(Some(text));
            }
        }
        Ok(None)
    }

    fn select_string_array(
        &self,
        selectors: &ArraySelectors,
        doc: DocWrapper,
    ) -> Result<Vec<String>, ScrapeError> {
        for selector in &selectors.selectors {
            let elements = doc
                .select(&selector.selector)
                .map_err(|_| ScrapeError::SelectorError(selector.selector.clone()))?;

            let mut items = vec![];

            for element in elements {
                let element = element.as_node();

                let mut text = match &selector.options.text_selection {
                    StringSelection::AllText { join_with } => element.all_text(join_with),
                    StringSelection::OwnText => element.own_text(),
                    StringSelection::Attributes(attrs) => {
                        element.attr_first_of(attrs).unwrap_or_default()
                    }
                };
                // Cleanup the text
                for cleanup in &selector.options.cleanup {
                    text = cleanup
                        .replace_regex
                        .replace_all(&text, &cleanup.replace_with)
                        .to_string();
                }
                // Fix capitalization
                text = match &selector.options.fix_capitalization {
                    crate::core::config::string_selector_options::FixCapitalization::Title => {
                        text.to_case(convert_case::Case::Title)
                    }
                    crate::core::config::string_selector_options::FixCapitalization::Skip => text,
                };
                if !text.is_empty() {
                    // Split text
                    if let Some(split_regex) = &selector.options.text_split_regex {
                        items.append(&mut split_regex.split(&text).map(String::from).collect());
                    } else {
                        items.push(text);
                    }
                }
            }

            if !items.is_empty() {
                return Ok(items);
            }
        }

        Ok(vec![])
    }

    async fn fetch_external(
        &self,
        url: &Url,
        mut doc: DocWrapper,
        fetch_external: &[FetchExternal],
    ) -> Result<DocWrapper, ScrapeError> {
        for ext_fetch in fetch_external {
            info!("[EXT] Trying fetch on {}", url.host_str().unwrap());
            let element = self.select_string(&ext_fetch.id, doc.clone()).ok();
            if let Some(Some(text)) = element {
                info!("[EXT] Found {:?}", text);
                let id = ext_fetch.regex.captures(&text);
                if let Some(id) = id.and_then(|id| id.name("id")) {
                    let id = id.as_str();
                    info!("[EXT] Which is id {}", id);
                    let chapter_url = ext_fetch.url.replace("{id}", id);
                    info!("[EXT] URL is {}", chapter_url);
                    if let Ok(url) = url.join(&chapter_url) {
                        info!("[EXT] Full URL is {}", chapter_url);
                        let (chapter_doc, _) = fetch_doc(&url).await?;
                        doc = chapter_doc;
                        break;
                    }
                }
            }
        }
        Ok(doc)
    }

    // Generic parse functions
    async fn chapters(
        &self,
        url: &Url,
        config: &MangaScraperConfig,
        doc: DocWrapper,
    ) -> Result<Vec<Chapter>, ScrapeError> {
        let chapter_config = &config.manga.chapter;

        let doc = self
            .fetch_external(url, doc, &chapter_config.fetch_external)
            .await?;

        let elements = {
            let mut elements = None;
            for selector in &chapter_config.base.selectors {
                elements = Some(doc.select(&selector.selector).map_err(|_| {
                    ScrapeError::SelectorError("Error in chapter base selector".to_string())
                })?);
                if elements.as_ref().is_some_and(|els| !els.is_empty()) {
                    break;
                }
            }
            elements.ok_or(ScrapeError::SelectorError(
                "Error in chapter base selector".to_string(),
            ))
        }?;

        let mut chapters = vec![];
        let total_chapters = elements.len();
        for (index, element) in elements.enumerate() {
            let title = self.select_required_string(
                &chapter_config.title,
                DocWrapper(element.as_node().clone()),
            )?;
            let number_text = chapter_config
                .number
                .as_ref()
                .and_then(|selector| {
                    self.select_string(selector, DocWrapper(element.as_node().clone()))
                        .ok()
                        .flatten()
                })
                .unwrap_or_else(|| title.clone());

            chapters.push(Chapter {
                url: self.select_required_url(
                    url,
                    &chapter_config.url,
                    DocWrapper(element.as_node().clone()),
                )?,
                title,
                number: crate::util::number::try_parse_number(&number_text)
                    .unwrap_or((total_chapters - index) as f32),
                date: chapter_config
                    .date
                    .as_ref()
                    .and_then(|selector| {
                        self.select_date(
                            &config.date_formats,
                            selector,
                            DocWrapper(element.as_node().clone()),
                        )
                        .ok()
                    })
                    .flatten(),
            })
        }

        Ok(chapters)
    }

    fn select_date(
        &self,
        date_formats: &[String],
        selector: &StringSelectors,
        doc: DocWrapper,
    ) -> Result<Option<DateTime<Utc>>, ScrapeError> {
        let text = self.select_required_string(selector, doc)?;

        Ok(crate::util::date::try_parse_date(&text, date_formats))
    }

    async fn images(
        &self,
        url: Url,
        config: &MangaScraperConfig,
        doc: DocWrapper,
    ) -> Result<Vec<Url>, ScrapeError> {
        let doc = self
            .fetch_external(&url, doc, &config.images.fetch_external)
            .await?;

        let images = self.select_string_array(&config.images.image_selector, doc)?;

        let images = images
            .into_iter()
            .map(|url_string| {
                url.join(&url_string)
                    .map_err(|e| ScrapeError::NotAValidURL(e.to_string()))
            })
            .collect::<Result<Vec<Url>, ScrapeError>>()?;

        Ok(images)
    }

    async fn full_manga(
        &self,
        url: Url,
        config: &MangaScraperConfig,
        doc: DocWrapper,
    ) -> Result<Manga, ScrapeError> {
        let status = config.manga.status.as_ref().map_or(Ok(None), |selector| {
            self.select_string(selector, doc.clone())
        })?;

        Ok(Manga {
            url: url.clone(),
            title: self.select_required_string(&config.manga.title, doc.clone())?,
            description: self.select_required_string(&config.manga.description, doc.clone())?,
            cover_url: config
                .manga
                .cover_url
                .as_ref()
                .map_or(Ok(None), |selector| {
                    self.select_url(&url, selector, doc.clone())
                })?,
            status: status.clone(),
            is_ongoing: self.manga_status(status),
            authors: config
                .manga
                .authors
                .as_ref()
                .map_or(Ok(vec![]), |selector| {
                    self.select_string_array(selector, doc.clone())
                })?,
            genres: config
                .manga
                .genres
                .as_ref()
                .map_or(Ok(vec![]), |selector| {
                    self.select_string_array(selector, doc.clone())
                })?,
            alternative_titles: config
                .manga
                .alt_titles
                .as_ref()
                .map_or(Ok(vec![]), |selector| {
                    self.select_string_array(selector, doc.clone())
                })?,
            chapters: self.chapters(&url, config, doc).await?,
        })
    }

    fn manga_status(&self, status: Option<String>) -> bool {
        if let Some(status) = status {
            matches!(
                status.to_lowercase().as_str(),
                "ongoing" | "on-going" | "updating" | "live"
            )
        } else {
            true
        }
    }

    fn get_configs_for_url(&self, url: &Url, doc: DocWrapper) -> Vec<&MangaScraperConfig> {
        let hostname = url.host_str().unwrap().to_string();

        let mut accepted_configs = vec![];
        for config in self.configs.iter() {
            if config.accept.hostnames.contains(&hostname) {
                accepted_configs.push(config);
            } else {
                for selector in config.accept.selectors.iter() {
                    if doc.select_first(selector).is_ok() {
                        accepted_configs.push(config);
                        break;
                    }
                }
            }
        }
        accepted_configs
    }
}

#[async_trait::async_trait]
impl MangaScraper for GenericScraper {
    async fn manga(&self, url: &Url) -> Result<Manga, ScrapeError> {
        let (doc, url) = fetch_doc(url).await?;

        let accepted_configs = self.get_configs_for_url(&url, doc.clone());

        let mut errors = HashMap::<String, ScrapeError>::new();
        for config in accepted_configs {
            match self.full_manga(url.clone(), config, doc.clone()).await {
                Ok(manga) => return Ok(manga),
                Err(e) => {
                    errors.insert(config.name.clone(), e);
                }
            };
        }

        if errors.is_empty() {
            let hostname = url.host_str().unwrap().to_string();
            Err(ScrapeError::WebsiteNotSupported(format!(
                "No scrapers found for {hostname}"
            )))
        } else {
            Err(ScrapeError::MultipleScrapingErrors(errors))
        }
    }

    async fn chapter_images(&self, chapter_url: &Url) -> Result<Vec<Url>, ScrapeError> {
        let (doc, url) = fetch_doc(chapter_url).await?;
        let accepted_configs = self.get_configs_for_url(&url, doc.clone());

        let mut errors = HashMap::<String, ScrapeError>::new();
        for config in accepted_configs {
            match self.images(url.clone(), config, doc.clone()).await {
                Ok(images) => return Ok(images),
                Err(e) => {
                    errors.insert(config.name.clone(), e);
                }
            };
        }

        if errors.is_empty() {
            let hostname = url.host_str().unwrap().to_string();
            Err(ScrapeError::WebsiteNotSupported(format!(
                "No scrapers found for {hostname}"
            )))
        } else {
            Err(ScrapeError::MultipleScrapingErrors(errors))
        }
    }

    async fn accepts(&self, _url: &Url) -> bool {
        // Assume this parser can parse any website
        true
    }
}

#[derive(Clone)]
struct DocWrapper(pub NodeRef);
unsafe impl Send for DocWrapper {}
unsafe impl Sync for DocWrapper {}
impl Deref for DocWrapper {
    type Target = NodeRef;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn html_to_doc(html: &str) -> Result<DocWrapper, ScrapeError> {
    let doc = std::panic::catch_unwind(|| kuchiki::parse_html().one(html))
        .map_err(|_e| ScrapeError::WebScrapingError("Could not parse HTML".to_string()))?;
    Ok(DocWrapper(doc))
}

async fn fetch_doc(url: &Url) -> Result<(DocWrapper, Url), ScrapeError> {
    let response = HTTP_CLIENT
        .execute(
            HTTP_CLIENT
                .request(Method::GET, url.clone())
                .header("Referer", url.to_string())
                .header("Origin", url.to_string())
                .header(
                    reqwest::header::USER_AGENT,
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:107.0) Gecko/20100101 Firefox/107.0",
                )
                .header("Accept", "*/*")
                .timeout(Duration::from_secs(5))
                .build()?,
        )
        .await?;

    let response = match response.error_for_status() {
        Ok(response) => response,
        Err(e) => {
            if let Some(StatusCode::FORBIDDEN) = e.status() {
                return Err(ScrapeError::CloudflareIUAM);
            } else {
                return Err(e.into());
            }
        }
    };

    let url = response.url().clone();
    let html = response.text().await?;
    Ok((html_to_doc(&html)?, url))
}
