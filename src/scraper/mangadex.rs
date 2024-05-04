use std::time::Duration;
use std::vec;

use chrono::DateTime;
use mangadex_api::v5::schema::RelatedAttributes;
use mangadex_api::v5::MangaDexClient;
use mangadex_api_schema_rust::v5::ChapterObject;
use mangadex_api_types_rust::{
    ChapterSortOrder, IncludeFuturePublishAt, IncludeFutureUpdates, Language, MangaStatus,
    OrderDirection, ReferenceExpansionResource, RelationshipType,
};
use reqwest::Url;
use tokio::time::sleep;

use crate::error::ScrapeError;
use crate::model::*;

use super::MangaScraper;

pub struct MangaDex {
    client: MangaDexClient,
}

impl MangaDex {
    pub fn new() -> Self {
        MangaDex {
            client: MangaDexClient::default(),
        }
    }
}

#[async_trait::async_trait]
impl MangaScraper for MangaDex {
    async fn manga(&self, url: &Url) -> Result<Manga, ScrapeError> {
        let mut segments = url
            .path_segments()
            .ok_or(ScrapeError::WebsiteNotSupported(url.to_string()))?;

        segments
            .next()
            .filter(|s| s == &"title" || s == &"manga")
            .ok_or(ScrapeError::WebsiteNotSupported(url.to_string()))?;

        let uuid = &uuid::Uuid::parse_str(segments.next().ok_or(
            ScrapeError::WebsiteNotSupported(format!("No ID found in url ({})", url.as_str())),
        )?)
        .map_err(|e| ScrapeError::UnknownError(Box::new(e)))?;

        let manga = self
            .client
            .manga()
            .get()
            .add_manga_id(uuid)
            .include(&ReferenceExpansionResource::Author)
            .include(&ReferenceExpansionResource::Chapter)
            .build()
            .map_err(|e| ScrapeError::UnknownError(Box::new(e)))?
            .send()
            .await
            .map_err(|e| ScrapeError::UnknownError(Box::new(e)))?
            .data
            .first()
            .ok_or(ScrapeError::UnknownErrorStr("Manga not found"))?
            .clone();

        let cover_id = manga
            .relationships
            .iter()
            .find(|related| related.type_ == RelationshipType::CoverArt);

        let cover = if let Some(relationship) = cover_id {
            let cover = self
                .client
                .cover()
                .get()
                .add_cover_id(&relationship.id)
                .build()
                .map_err(|e| ScrapeError::UnknownError(Box::new(e)))?
                .send()
                .await
                .map_err(|e| ScrapeError::UnknownError(Box::new(e)))?;

            Some(
                Url::parse(&format!(
                    "{}/covers/{}/{}",
                    mangadex_api::constants::CDN_URL,
                    uuid,
                    cover
                        .data
                        .first()
                        .ok_or(ScrapeError::UnknownErrorStr("Manga not found"))?
                        .attributes
                        .file_name
                ))
                .map_err(|e| ScrapeError::UnknownError(Box::new(e)))?,
            )
        } else {
            None
        };

        let mut chapters: Vec<ChapterObject> = vec![];
        let mut offset: u32 = 0;
        let mut total: u32 = 0;

        while offset == 0 || offset < total {
            if offset != 0 && offset % 400 == 0 {
                // When 3 requests are made, wait one second before making the next
                sleep(Duration::from_secs(1)).await;
            }
            let results = self
                .client
                .chapter()
                .get()
                .manga_id(*uuid)
                .limit(100u32)
                .offset(offset)
                .include_future_publish_at(IncludeFuturePublishAt::Exclude)
                .include_future_updates(IncludeFutureUpdates::Exclude)
                .add_translated_language(Language::English)
                .order(ChapterSortOrder::Chapter(OrderDirection::Descending))
                .send()
                .await
                .map_err(|e| ScrapeError::UnknownError(Box::new(e)))?;

            chapters.append(&mut results.data.clone());
            total = results.total;
            offset += 100;
        }

        let chapters: Vec<Chapter> = chapters
            .iter()
            .enumerate()
            .map(|(index, chapter)| {
                let number = chapter
                    .attributes
                    .chapter
                    .as_ref()
                    .unwrap_or(&index.to_string())
                    .parse()
                    .unwrap();
                Chapter {
                    number,
                    date: Some(
                        DateTime::from_timestamp_millis(
                            chapter.attributes.created_at.as_ref().unix_timestamp(),
                        )
                        .unwrap(),
                    ),
                    title: chapter
                        .attributes
                        .title
                        .clone()
                        .unwrap_or(format!("Chapter {number}"))
                        .to_owned(),
                    url: Url::parse(&format!("{}/chapter/{}", mangadex_api::API_URL, chapter.id))
                        .unwrap(),
                }
            })
            .collect();

        Ok(Manga {
            url: url.clone(),
            cover_url: cover,
            title: manga
                .attributes
                .title
                .get(&Language::English)
                .ok_or(ScrapeError::MissingMangaTitle)?
                .to_owned(),
            description: manga
                .attributes
                .description
                .get(&Language::English)
                .unwrap_or(&"No description".to_owned())
                .to_owned(),
            alternative_titles: manga
                .attributes
                .alt_titles
                .iter()
                .flat_map(|a| a.values().map(|a| a.to_owned()).collect::<Vec<String>>())
                .collect(),
            authors: manga
                .relationships
                .iter()
                .filter(|a| a.type_ == RelationshipType::Author)
                .filter_map(|a| {
                    if let Some(RelatedAttributes::Author(author)) = &a.attributes {
                        Some(author.name.to_owned())
                    } else {
                        None
                    }
                })
                .collect(),
            genres: manga
                .attributes
                .tags
                .iter()
                .filter(|a| a.type_ == RelationshipType::Tag)
                .map(|a| a.attributes.name.values().next())
                .filter(|a| a.is_some())
                .map(|a| a.unwrap().to_owned())
                .collect(),
            chapters,
            is_ongoing: manga.attributes.status == MangaStatus::Ongoing,
            status: Some(format!("{:?}", manga.attributes.status)),
        })
    }

    async fn chapter_images(&self, chapter_url: &Url) -> Result<Vec<Url>, ScrapeError> {
        let mut segments = chapter_url
            .path_segments()
            .ok_or(ScrapeError::WebsiteNotSupported(chapter_url.to_string()))?;

        segments
            .next()
            .filter(|s| s == &"chapter")
            .ok_or(ScrapeError::WebsiteNotSupported(chapter_url.to_string()))?;

        let uuid =
            &uuid::Uuid::parse_str(segments.next().ok_or(ScrapeError::WebsiteNotSupported(
                format!("No ID found in url ({})", chapter_url.as_str()),
            ))?)
            .map_err(|e| ScrapeError::UnknownError(Box::new(e)))?;

        let at_home = self
            .client
            .at_home()
            .server()
            .id(*uuid)
            .get()
            .build()
            .map_err(|e| ScrapeError::UnknownError(Box::new(e)))?
            .send()
            .await
            .map_err(|e| ScrapeError::UnknownError(Box::new(e)))?;

        let images: Vec<Url> = at_home
            .chapter
            .data_saver
            .iter()
            .map(|filename| {
                at_home
                    .base_url
                    .join(&format!(
                        "/{quality_mode}/{chapter_hash}/{page_filename}",
                        quality_mode = "data-saver",
                        chapter_hash = at_home.chapter.hash,
                        page_filename = filename
                    ))
                    .unwrap()
            })
            .collect();

        Ok(images)
    }

    async fn search(
        &self,
        query: &str,
        _hostnames: &[String],
    ) -> Result<Vec<SearchManga>, ScrapeError> {
        let results = self
            .client
            .search()
            .manga()
            .add_available_translated_language(Language::English)
            .title(query)
            .include(ReferenceExpansionResource::CoverArt)
            .build()
            .map_err(|e| ScrapeError::UnknownError(Box::new(e)))?
            .send()
            .await
            .map_err(|e| ScrapeError::UnknownError(Box::new(e)))?;

        let search_results = results
            .data
            .iter()
            .map(|m| SearchManga {
                title: m
                    .attributes
                    .title
                    .get(&Language::English)
                    .unwrap_or(&"No title".to_owned())
                    .to_owned(),
                posted: m.attributes.updated_at.as_ref().map(|date| {
                    DateTime::from_timestamp_millis(date.as_ref().unix_timestamp()).unwrap()
                }),
                cover_url: m
                    .relationships
                    .clone()
                    .into_iter()
                    .find(|rel| rel.type_ == RelationshipType::CoverArt)
                    .map(|cover_rel| {
                        if let Some(RelatedAttributes::CoverArt(cover)) = cover_rel.attributes {
                            Url::parse(&format!(
                                "{}/covers/{}/{}",
                                mangadex_api::constants::CDN_URL,
                                m.id,
                                cover.file_name
                            ))
                            .unwrap()
                        } else {
                            panic!();
                        }
                    }),
                url: Url::parse(&format!("{}/manga/{}", mangadex_api::API_URL, m.id)).unwrap(),
            })
            .collect();

        Ok(search_results)
    }

    async fn accepts(&self, url: &Url) -> bool {
        let hostname = url.host_str();
        if let Some(hostname) = hostname {
            self.searchable_hostnames()
                .binary_search(&hostname.to_string())
                .is_ok()
        } else {
            false
        }
    }

    fn searchable_hostnames(&self) -> Vec<String> {
        vec!["api.mangadex.org".to_string(), "mangadex.org".to_string()]
    }

    fn search_accepts(&self, hostname: &str) -> bool {
        self.searchable_hostnames()
            .binary_search(&hostname.to_string())
            .is_ok()
    }
}
