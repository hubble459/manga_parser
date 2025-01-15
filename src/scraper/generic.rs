use std::{collections::HashMap, ops::Deref, path::Path, time::Duration};

use chrono::{DateTime, Utc};
use config::{builder::DefaultState, ConfigBuilder, File};
use convert_case::Casing;
use kuchiki::{traits::TendrilSink, NodeRef};
use reqwest::{Body, Method, StatusCode, Url};

use crate::{
    config::{
        array_selector::ArraySelectors,
        chapter::FetchExternal,
        string_selector::StringSelectors,
        string_selector_options::{self, StringSelection},
        MangaScraperConfig,
    },
    error::ScrapeError,
    model::{Chapter, Manga, MangaBuilder, SearchManga},
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
                file.path().extension().unwrap_or_default().to_str().unwrap(),
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

    fn select_required_string(&self, selector: &StringSelectors, doc: DocWrapper) -> Result<String, ScrapeError> {
        self.select_string(selector, doc)?
            .ok_or(ScrapeError::WebScrapingError(format!(
                "Missing required field with selectors: {:?}",
                selector.selectors
            )))
    }

    fn select_string(&self, selectors: &StringSelectors, doc: DocWrapper) -> Result<Option<String>, ScrapeError> {
        for selector in &selectors.selectors {
            let elements = doc
                .select(&selector.selector)
                .map_err(|_| ScrapeError::SelectorError(selector.selector.clone()))?;

            let mut text = match &selector.options.text_selection {
                StringSelection::AllText { join_with } => elements.all_text(join_with),
                StringSelection::OwnText => elements.own_text(),
                StringSelection::Attributes(attrs) => {
                    let first_attr = elements.attr_first_of(attrs).unwrap_or_default();
                    first_attr
                }
            };

            // Trim text
            text = text.trim().to_string();
            if !text.is_empty() {
                // Cleanup the text
                for cleanup in &selector.options.cleanup {
                    text = cleanup
                        .replace_regex
                        .replace_all(&text, &cleanup.replace_with)
                        .to_string();
                }
                // Fix capitalization
                text = match &selector.options.fix_capitalization {
                    string_selector_options::FixCapitalization::Title => text.to_case(convert_case::Case::Title),
                    string_selector_options::FixCapitalization::Skip => text,
                };

                if !text.is_empty() {
                    return Ok(Some(text));
                } else {
                    return Ok(None);
                }
            }
        }
        Ok(None)
    }

    fn select_string_array(&self, selectors: &ArraySelectors, doc: DocWrapper) -> Result<Vec<String>, ScrapeError> {
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
                    StringSelection::Attributes(attrs) => element.attr_first_of(attrs).unwrap_or_default(),
                };
                // Trim text
                text = text.trim().to_string();
                // Cleanup the text
                for cleanup in &selector.options.cleanup {
                    text = cleanup
                        .replace_regex
                        .replace_all(&text, &cleanup.replace_with)
                        .to_string();
                }
                // Fix capitalization
                text = match &selector.options.fix_capitalization {
                    string_selector_options::FixCapitalization::Title => text.to_case(convert_case::Case::Title),
                    string_selector_options::FixCapitalization::Skip => text,
                };
                if !text.is_empty() {
                    // Split text
                    if let Some(split_regex) = &selector.options.text_split_regex {
                        let mut parts = split_regex.split(&text).map(String::from).collect();
                        debug!("Split {text} with {:?}: {:#?}", split_regex, parts);
                        items.append(&mut parts);
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

    async fn do_search(
        &self,
        config: &MangaScraperConfig,
        hostname: &str,
        query: &str,
    ) -> Result<Vec<SearchManga>, ScrapeError> {
        let search_config = &config.search;
        let search_config = search_config
            .iter()
            .find(|search| search.hostnames.contains(&hostname.to_string()));

        if search_config.is_none() {
            return Err(ScrapeError::SearchNotSupported(vec![hostname.to_string()]));
        }
        let search_config = search_config.unwrap();

        let mut query = query.to_string();
        debug!("[SEARCH]: Searching for {query} on {hostname}");
        for format in &search_config.query_format {
            query = format
                .replace_regex
                .replace_all(&query, &format.replace_with)
                .to_string();
        }

        let mut search_url = search_config
            .search_url
            .replace("{hostname}", hostname)
            .replace("{query}", &query);
        if !search_url.starts_with("http") {
            search_url = String::from("https://") + &search_url;
        }
        let search_url = Url::parse(&search_url).map_err(|e| ScrapeError::NotAValidURL(e.to_string()))?;
        debug!("[SEARCH]: Search URL is {}", search_url.to_string());

        let (doc, ..) = fetch_doc_config(&search_url, Method::GET, None::<String>).await?;

        let elements = {
            let mut elements: Option<kuchiki::iter::Select<kuchiki::iter::Elements<kuchiki::iter::Descendants>>> = None;
            for selector in &search_config.selectors.base.selectors {
                elements = Some(
                    doc.select(&selector.selector)
                        .map_err(|_| ScrapeError::SelectorError("Error in search base selector".to_string()))?,
                );
                if elements.as_ref().is_some_and(|els| !els.is_empty()) {
                    break;
                }
            }
            elements.ok_or(ScrapeError::SelectorError("Error in search base selector".to_string()))
        }?;

        let mut search_results = vec![];

        for element in elements {
            search_results.push(SearchManga {
                url: self.select_required_url(
                    &search_url,
                    &search_config.selectors.url,
                    DocWrapper(element.as_node().clone()),
                )?,
                title: self
                    .select_required_string(&search_config.selectors.title, DocWrapper(element.as_node().clone()))?,
                cover_url: search_config
                    .selectors
                    .cover_url
                    .as_ref()
                    .and_then(|selector| {
                        self.select_url(&search_url, selector, DocWrapper(element.as_node().clone()))
                            .ok()
                    })
                    .flatten(),
                posted: search_config
                    .selectors
                    .posted
                    .as_ref()
                    .and_then(|selector| {
                        self.select_date(&config.date_formats, selector, DocWrapper(element.as_node().clone()))
                            .ok()
                    })
                    .flatten(),
            })
        }

        Ok(search_results)
    }

    async fn fetch_external(
        &self,
        url: &Url,
        mut doc: DocWrapper,
        fetch_external: &[FetchExternal],
    ) -> Result<DocWrapper, ScrapeError> {
        for ext_fetch in fetch_external {
            let hostname = url.host_str().unwrap();
            debug!(
                "[external] Trying fetch on {} with selectors ({:#?})",
                hostname, ext_fetch.id.selectors
            );
            let element = self.select_string(&ext_fetch.id, doc.clone()).ok();
            if let Some(Some(text)) = element {
                debug!("[external] Found {:?}", text);
                let id = ext_fetch.regex.captures(&text);
                if let Some(id) = id.and_then(|id| id.name("id")) {
                    let id = id.as_str();
                    debug!("[external] Which is id {}", id);
                    let chapter_url = ext_fetch.url.replace("{id}", id);
                    let chapter_url = chapter_url.replace("{host}", hostname);
                    let chapter_url = chapter_url.replace("{url}", url.as_str());
                    debug!("[external] URL is {}", chapter_url);
                    if let Ok(url) = url.join(&chapter_url) {
                        debug!("[external] Full URL is {}", chapter_url);
                        let method = match ext_fetch.method.as_str() {
                            "post" => Method::POST,
                            _ => Method::GET,
                        };
                        let (chapter_doc, ..) = fetch_doc_config(&url, method, None::<String>).await?;
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

        let doc = self.fetch_external(url, doc, &chapter_config.fetch_external).await?;

        let elements = {
            let mut elements = None;
            for selector in &chapter_config.base.selectors {
                elements = Some(
                    doc.select(&selector.selector)
                        .map_err(|_| ScrapeError::SelectorError("Error in chapter base selector".to_string()))?,
                );
                if elements.as_ref().is_some_and(|els| !els.is_empty()) {
                    break;
                }
            }
            elements.ok_or(ScrapeError::SelectorError("Error in chapter base selector".to_string()))
        }?;

        let mut chapters = vec![];
        let total_chapters = elements.len();
        for (index, element) in elements.enumerate() {
            let title = self.select_required_string(&chapter_config.title, DocWrapper(element.as_node().clone()))?;
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
                url: self.select_required_url(url, &chapter_config.url, DocWrapper(element.as_node().clone()))?,
                title,
                number: crate::util::number::try_parse_number(&number_text).unwrap_or((total_chapters - index) as f32),
                date: chapter_config
                    .date
                    .as_ref()
                    .and_then(|selector| {
                        self.select_date(&config.date_formats, selector, DocWrapper(element.as_node().clone()))
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

    async fn images(&self, url: Url, config: &MangaScraperConfig, doc: DocWrapper) -> Result<Vec<Url>, ScrapeError> {
        debug!("[images] parsing images for {}", url.as_str());

        let doc = self.fetch_external(&url, doc, &config.images.fetch_external).await?;

        let images = self.select_string_array(&config.images.image_selector, doc)?;
        debug!("[images] found {} images", images.len());

        let images = images
            .into_iter()
            .map(|url_string| {
                url.join(&url_string)
                    .map_err(|e| ScrapeError::NotAValidURL(e.to_string()))
            })
            .collect::<Result<Vec<Url>, ScrapeError>>()?;

        debug!("[images] parsed {} images to URLs", images.len());

        Ok(images)
    }

    async fn full_manga<'a>(
        &self,
        url: Url,
        config: &MangaScraperConfig,
        doc: DocWrapper,
        manga_builder: &'a mut MangaBuilder,
    ) -> Result<&'a mut MangaBuilder, ScrapeError> {
        debug!("[manga] parsing manga at {}", url.as_str());

        // Overwriting URL
        manga_builder.url(url.clone());
        // Status
        if !manga_builder.has_status() {
            if let Some(status) = config
                .manga
                .status
                .as_ref()
                .map_or(Ok(None), |selector| self.select_string(selector, doc.clone()))?
            {
                manga_builder.status(status.clone());
                manga_builder.is_ongoing(self.manga_status(Some(status)));
            }
        }
        // Title
        if !manga_builder.has_title() {
            if let Some(title) = self.select_string(&config.manga.title, doc.clone())? {
                manga_builder.title(title);
            }
        }
        // Description
        if !manga_builder.has_description() {
            if let Some(description) = self.select_string(&config.manga.description, doc.clone())? {
                manga_builder.description(description);
            }
        }
        // Cover URL
        if let Some(cover_url) = config
            .manga
            .cover_url
            .as_ref()
            .map_or(Ok(None), |selector| self.select_url(&url, selector, doc.clone()))?
        {
            manga_builder.cover_url(cover_url);
        }
        // Authors
        manga_builder.authors(
            config
                .manga
                .authors
                .as_ref()
                .map_or(Ok(vec![]), |selector| self.select_string_array(selector, doc.clone()))?,
        );
        // Genres
        manga_builder.genres(
            config
                .manga
                .genres
                .as_ref()
                .map_or(Ok(vec![]), |selector| self.select_string_array(selector, doc.clone()))?,
        );
        // Alternative Titles
        manga_builder.alternative_titles(
            config
                .manga
                .alt_titles
                .as_ref()
                .map_or(Ok(vec![]), |selector| self.select_string_array(selector, doc.clone()))?,
        );
        // Chapters
        if !manga_builder.has_chapters() {
            manga_builder.chapters(self.chapters(&url, config, doc).await?);
        }

        Ok(manga_builder)
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

    fn get_search_configs_for_hostname(&self, hostname: &str) -> Vec<&MangaScraperConfig> {
        let mut accepted_configs = vec![];
        for config in self.configs.iter() {
            for search in config.search.iter() {
                if search.hostnames.contains(&hostname.to_string()) {
                    accepted_configs.push(config);
                }
            }
        }
        accepted_configs
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
        if log_enabled!(log::Level::Debug) {
            debug!(
                "[config] found {} config(s) for {}",
                accepted_configs.len(),
                url.as_str()
            );
            debug!(
                "[config] {:?}",
                accepted_configs
                    .iter()
                    .map(|config| config.name.as_str())
                    .collect::<Vec<&str>>()
            );
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
        let mut manga_builder = MangaBuilder::new();
        for config in accepted_configs {
            match self
                .full_manga(url.clone(), config, doc.clone(), &mut manga_builder)
                .await
            {
                Ok(manga_builder) => match manga_builder.build() {
                    Ok(manga) => return Ok(manga),
                    Err(e) => {
                        errors.insert(config.name.clone(), ScrapeError::WebScrapingError(e.to_string()));
                    }
                },
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
        true

        // Proper check is below, but feels like a wasted call
        // Also ignores CloudFlare error if there is one
        /*
        if let Ok((doc, url)) = fetch_doc(url).await {
            let accepted_configs = self.get_configs_for_url(&url, doc.clone());
            return !accepted_configs.is_empty();
        }
        false
        */
    }

    async fn search(&self, query: &str, hostnames: &[String]) -> Result<Vec<SearchManga>, ScrapeError> {
        let mut err = None;
        let mut results = vec![];
        for hostname in hostnames {
            let accepted_configs = self.get_search_configs_for_hostname(hostname);
            for config in accepted_configs {
                match self.do_search(&config, hostname, query).await {
                    Ok(mut search_manga) => results.append(&mut search_manga),
                    Err(e) => err = Some(e),
                };
            }
        }
        if !results.is_empty() {
            return Ok(results);
        }

        Err(err.unwrap_or(ScrapeError::SearchNotSupported(hostnames.to_vec())))
    }

    fn searchable_hostnames(&self) -> Vec<String> {
        let mut hostnames = vec![];
        for config in self.configs.iter() {
            for search in config.search.iter() {
                hostnames.append(&mut search.hostnames.clone());
            }
        }
        hostnames.sort();
        hostnames
    }

    fn search_accepts(&self, hostname: &str) -> bool {
        self.searchable_hostnames().binary_search(&hostname.to_string()).is_ok()
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

async fn fetch_doc_config<T>(url: &Url, method: Method, body: Option<T>) -> Result<(DocWrapper, Url), ScrapeError>
where
    T: Into<Body>,
{
    let mut request = HTTP_CLIENT
        .request(method, url.clone())
        .header("Referer", url.to_string())
        .header("Origin", url.to_string())
        .timeout(Duration::from_secs(5));

    if let Some(body) = body {
        request = request.body(body);
    }

    let response = HTTP_CLIENT.execute(request.build()?).await?;

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

async fn fetch_doc(url: &Url) -> Result<(DocWrapper, Url), ScrapeError> {
    fetch_doc_config(url, Method::GET, None::<String>).await
}
