use std::collections::HashSet;

use chrono::Utc;
use kuchiki::{
    iter::{Descendants, Elements, Select},
    traits::TendrilSink,
    NodeRef,
};
use regex::Regex;

use crate::{
    error::MangaError,
    model::{Chapter, Manga, MangaConfig},
    CONFIGS, util,
};

type Result<T> = std::result::Result<T, MangaError>;

fn html_to_doc(html: &str) -> Result<NodeRef> {
    std::panic::catch_unwind(|| kuchiki::parse_html().one(html))
        .map_err(|_e| MangaError::WebScrapingError("Could not parse HTML".to_string()))
}

pub fn scrape_manga(url: &str, html: &str) -> Result<Manga> {
    let document = html_to_doc(html)?;

    let config = CONFIGS
        .lock()
        .unwrap()
        .get("madara")
        .cloned()
        .unwrap()
        .try_deserialize::<MangaConfig>()?;

    let title = document
        .select(&config.title_selector)
        .map_err(|_| MangaError::SelectorError(config.title_selector.to_string()))?
        .next()
        .ok_or_else(|| MangaError::WebScrapingError("Title not found".to_string()))?
        .text_contents()
        .trim()
        .to_owned();
    let description = document
        .select(&config.description_selector)
        .map_err(|_| MangaError::SelectorError(config.description_selector.to_string()))?
        .next()
        .ok_or_else(|| MangaError::WebScrapingError("Description not found".to_string()))?
        .text_contents()
        .trim()
        .to_owned();
    let cover_url = document
        .select(&config.cover_url_selector)
        .map_err(|_| MangaError::SelectorError(config.cover_url_selector.to_string()))?
        .next()
        .ok_or_else(|| MangaError::WebScrapingError("Cover URL not found".to_string()))?
        .attributes
        .borrow()
        .get("src")
        .ok_or_else(|| MangaError::WebScrapingError("Cover URL not found".to_string()))?
        .to_string()
        .trim()
        .to_owned();
    let status = document
        .select(&config.status_selector)
        .map_err(|_| MangaError::SelectorError(config.status_selector.to_string()))?
        .next()
        .ok_or_else(|| MangaError::WebScrapingError("Status not found".to_string()))?
        .text_contents()
        .trim()
        .to_owned();

    let authors = parse_string_vec(
        document
            .select(&config.authors_selector)
            .map_err(|_| MangaError::SelectorError(config.authors_selector.to_string()))?,
    );
    let genres = parse_string_vec(
        document
            .select(&config.genres_selector)
            .map_err(|_| MangaError::SelectorError(config.genres_selector.to_string()))?,
    );
    let alternative_titles = parse_string_vec(
        document
            .select(&config.alternative_titles_selector)
            .map_err(|_| {
                MangaError::SelectorError(config.alternative_titles_selector.to_string())
            })?,
    );

    let mut chapter_doc = None;

    for external_selector in &config.chapter_external_selectors {
        let script = document
            .select(&external_selector.id)
            .map_err(|_| MangaError::SelectorError(external_selector.id.to_string()))?
            .next();

        if let Some(script) = script {
            let script_text = script.text_contents();
            let re = regex::Regex::new(&external_selector.regex).unwrap();
            let captures = re
                .captures(&script_text)
                .ok_or_else(|| MangaError::WebScrapingError("Script data not found".to_string()))?;

            let manga_id = captures.get(1).unwrap().as_str();

            // Construct the URL for the chapter list request using mangaID and chapterFirsID
            let chapter_list_url = reqwest::Url::parse(url)
                .unwrap()
                .join(&external_selector.url.replace("{id}", manga_id))
                .unwrap()
                .to_string();
            println!("chapters url: {chapter_list_url}");
            let chapter_html = reqwest::blocking::get(&chapter_list_url)?.text()?;
            chapter_doc = Some(html_to_doc(&chapter_html)?);
            break;
        }
    }

    let chapter_doc = chapter_doc.unwrap_or(document);

    let chapters = chapter_doc
        .select(&config.chapters_selector)
        .map_err(|_| MangaError::SelectorError(config.chapters_selector.to_string()))?
        .map(|e| {
            let e = e.as_node();

            let title_element = e
                .select(&config.chapter_title_selector)
                .map_err(|_| MangaError::SelectorError(config.chapter_title_selector.to_string()))?
                .next()
                .ok_or_else(|| {
                    MangaError::WebScrapingError("Chapter title not found".to_string())
                })?;
            let title = title_element.text_contents().trim().to_string();

            let href_element = if config.chapter_href_selector == config.chapter_title_selector {
                title_element.clone()
            } else {
                e.select(&config.chapter_href_selector)
                    .map_err(|_| {
                        MangaError::SelectorError(config.chapter_href_selector.to_string())
                    })?
                    .next()
                    .ok_or_else(|| {
                        MangaError::WebScrapingError("Chapter href not found".to_string())
                    })?
            };

            let href = href_element
                .attributes
                .borrow()
                .get("href")
                .ok_or_else(|| MangaError::WebScrapingError("Chapter href not found".to_string()))?
                .trim()
                .to_string();

            let number_str = e
                .select(&config.chapter_number_selector)
                .map_err(|_| MangaError::SelectorError(config.chapter_number_selector.to_string()))?
                .next()
                .unwrap_or(title_element)
                .text_contents()
                .trim()
                .to_string();
            let re = regex::Regex::new(r"(\d+(\.\d+)?)").unwrap();
            let captures = re.captures(&number_str).ok_or_else(|| {
                MangaError::WebScrapingError("Chapter number not found".to_string())
            })?;

            let number_str = captures.get(1).unwrap().as_str();
            let number = number_str.parse::<f32>().map_err(|_| {
                MangaError::WebScrapingError("Failed to parse chapter number".to_string())
            })?;

            let date_str = e
                .select(&config.chapter_date_selector)
                .map_err(|_| MangaError::SelectorError(config.chapter_date_selector.to_string()))?
                .next()
                .ok_or_else(|| MangaError::WebScrapingError("Chapter date not found".to_string()))?
                .text_contents()
                .trim()
                .to_string();
            let date = util::date::try_parse_date(&date_str, &config.date_formats).unwrap_or(Utc::now());

            Ok::<Chapter, MangaError>(Chapter {
                href,
                number,
                title,
                date,
            })
        })
        .collect::<Result<Vec<Chapter>>>()?;

    Ok(Manga {
        title,
        description,
        cover_url,
        status,
        authors,
        genres,
        alternative_titles,
        chapters,
    })
}

pub fn scrape_manga_from_url(url: &str) -> Result<Manga> {
    let body = reqwest::blocking::get(url)?.text()?;
    scrape_manga(url, &body)
}

fn parse_string_vec(input: Select<Elements<Descendants>>) -> Vec<String> {
    let str_iter = input.map(|e| e.text_contents().trim().to_owned());
    let strings = str_iter.collect::<Vec<String>>();
    let size = strings.len();

    if size == 1 {
        let regex = Regex::new(r"\s*[;:\-,]\s*").expect("Invalid regex pattern");

        // Split the input string using the regex pattern
        let elements: HashSet<String> = regex
            .split(&strings.into_iter().collect::<String>())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        // Convert the HashSet into a Vec
        elements.into_iter().collect()
    } else {
        strings
    }
}
