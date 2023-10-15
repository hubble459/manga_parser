use manga_parser::{
    core::scraper_manager::ScraperManager, error::ScrapeError, scraper::MangaScraper, model::Manga,
};

lazy_static::lazy_static! {
    static ref SCRAPER_MANAGER: ScraperManager = ScraperManager::default();
}

macro_rules! test_manga {
    ($hostname:ident: $url:literal $(, ignore = [$($ignore:literal),*])?) => {
        paste::paste! {
            #[tokio::test]
            async fn [<test_manga_ $hostname>]() -> Result<(), ScrapeError> {
                let ignored = vec![$($($ignore,)*)?];
                let url = reqwest::Url::parse($url).unwrap();
                let manga = SCRAPER_MANAGER.manga(&url).await?;

                assert_manga(manga, &ignored).await;

                Ok(())
            }
        }
    };
}

test_manga!(isekaiscan: "https://isekaiscan.top/manga/moshi-fanren");

/// Ignored may be
/// - genres
/// - authors
/// - alt_titles
/// - chapter_date
async fn assert_manga(manga: Manga, ignore: &[&'static str]) {
    assert!(!manga.title.is_empty(), "Title is empty");
    assert!(!manga.description.is_empty(), "Description is empty");
    assert_ne!(manga.description, "No description", "Description is empty");
    assert!(manga.is_ongoing, "Manga is not ongoing");
    assert!(manga.url.has_host(), "URL is missing host");
    assert!(manga.cover_url.is_some_and(|url| url.has_host()), "Manga is missing a cover");
    if !ignore.contains(&"genres") {
        assert!(!manga.genres.is_empty(), "Missing genres");
    }
    if !ignore.contains(&"authors") {
        assert!(!manga.authors.is_empty(), "Missing authors");
    }
    if !ignore.contains(&"alt_titles") {
        assert!(!manga.alternative_titles.is_empty(), "Missing alternative titles");
    }
    assert!(!manga.chapters.is_empty(), "Missing chapters");
    let mut unique_urls = vec![];
    for chapter in manga.chapters.iter() {
        assert!(chapter.url.has_host(), "Chapter url is missing host");
        let url = chapter.url.to_string();
        assert!(
            !unique_urls.contains(&url),
            "Duplicate chapter url ({url})"
        );
        unique_urls.push(url);
        if !ignore.contains(&"chapter_date") {
            assert!(
                chapter.date.is_some(),
                "Chapter {} is missing a posted date",
                chapter.number
            );
        }
    }

    let first_chapter = manga.chapters.first().unwrap();
    let images = SCRAPER_MANAGER.chapter_images(&first_chapter.url).await.unwrap();
    assert!(!images.is_empty(), "No images found in chapter");

    // let hostname = util::get_hostname(&manga.url).unwrap();
    // if CAN_SEARCH.contains(&hostname) {
    //     let search_results = SCRAPER_MANAGER.search(manga.title.clone(), vec![hostname]).await;
    //     let search_results = search_results.unwrap();

    //     assert!(!search_results.is_empty(), "No search results");
    //     let item = search_results
    //         .into_iter()
    //         .find(|item| item.title.to_ascii_lowercase() == manga.title.to_ascii_lowercase());
    //     assert!(item.is_some(), "Could not find manga in search results");
    //     let item = item.unwrap();
    //     assert!(item.url.has_host(), "Search url is missing host");
    // }
}
