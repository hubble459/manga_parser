use manga_parser::{error::ScrapeError, model::Manga, scraper::scraper_manager::ScraperManager, scraper::MangaScraper};
lazy_static::lazy_static! {
    static ref SCRAPER_MANAGER: ScraperManager = ScraperManager::default();
}

macro_rules! test_manga_mod {
    ($type:ident, $($(#[$meta:meta])*$hostname:ident: $url:literal $(, ignore = [$($ignore:literal),*])?;)+) => {
        mod $type {
            use manga_parser::scraper::MangaScraper;

            $(
                #[doc = "[`url`]: $url"]
                #[allow(non_snake_case)]
                #[test_log::test(tokio::test)]
                $(#[$meta])*
                async fn $hostname() -> Result<(), $crate::ScrapeError> {
                    let ignored = vec![$($($ignore,)*)?];
                    let url = reqwest::Url::parse($url).unwrap();
                    let manga = $crate::SCRAPER_MANAGER.manga(&url).await?;

                    $crate::assert_manga(manga, &ignored).await;

                    Ok(())
                }
            )+
        }
    };
}

#[test_log::test]
fn test_dates() {
    let date = manga_parser::util::date::try_parse_date("18 April، 2022", &vec!["%d %B، %Y".to_string()]);
    log::info!("date: {:?}", date);

    let date = manga_parser::util::date::try_parse_date("an hour ago", &vec!["%d %B، %Y".to_string()]);
    log::info!("date: {:?}", date);
}

test_manga_mod! {
    mangadex,
    mangadex: "https://mangadex.org/title/c9c0f16b-7bd3-4da6-bd58-fcb4bd10112f/onnamaou-sama-wa-yuusha-kun-o-taosenai";
}

test_manga_mod! {
    madara,
    #[ignore = "CloudflareIUAM"] manhuaus: "https://manhuaus.com/manga/return-of-immortal-warlord/";
    #[ignore = "CloudflareIUAM"] manhwaclan: "https://manhwaclan.com/manga/becoming-a-cheat-level-skill-thief/";
    #[ignore = "CloudflareIUAM"] manhwatop: "https://manhwatop.com/manga/magic-emperor/";
    #[ignore = "Website doesn't exist anymore"] aquamanga: "https://aquamanga.com/read/my-insanely-competent-underlings";
    #[ignore = "Website doesn't exist anymore"] isekaiscan: "https://isekaiscan.top/manga/moshi-fanren";
    #[ignore = "Website doesn't exist anymore"] isekaiscanmanga: "https://isekaiscanmanga.com/manga/silver-devil-king/";
    #[ignore = "Website doesn't exist anymore"] mangafoxfull: "https://mangafoxfull.com/manga/magic-emperor/";
    #[ignore = "Website doesn't exist anymore"] mangaonlineteam: "https://mangaonlineteam.com/manga/miss-divine-doctor-conquer-the-demon-king/";
    #[ignore = "Website doesn't exist anymore"] mangarockteam: "https://mangarockteam.com/manga/academys-undercover-professor/";
    lhtranslation: "https://lhtranslation.net/manga/7th-demon-prince-jilbagias-the-demon-kingdom-destroyer/";
    mangasushi: "https://mangasushi.org/manga/shokei-sareta-saikyou-no-gunnyou-majutsushi-haisenkoku-no-elf-hime-to-kokka-saikensu-sokoku-yo-jama-suru-no-wa-kattedaga-sono-majutsu-tsukutta-no-ore-na-node-kikanai-ga/";
    manhuafast: "https://manhuafast.com/manga/descending-the-mountain-as-invincible-all-chapters/";
    manhuaplus: "https://manhuaplus.com/manga/demon-magic-emperor01/";
    s2manga: "https://s2manga.com/manga/i-m-ready-for-divorce/", ignore = ["authors"];
}

test_manga_mod! {
    mangakakalot,
    mangakakalot: "https://www.mangakakalot.gg/manga/after-improperly-licking-a-dog-i-became-a-billionaire", ignore = ["alt_titles"];
}

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
    assert!(
        manga.cover_url.is_some_and(|url| url.has_host()),
        "Manga is missing a cover"
    );
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
        assert!(!unique_urls.contains(&url), "Duplicate chapter url ({url})");
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
    assert!(
        !images.is_empty(),
        "No images found in chapter ({})",
        &first_chapter.url
    );

    let hostname = manga.url.host_str().expect("Missing hostname in URL");

    if SCRAPER_MANAGER.search_accepts(hostname) {
        let search_results = SCRAPER_MANAGER.search(&manga.title, &[hostname.to_string()]).await;
        let search_results = search_results.unwrap();

        log::info!("sr: {:?}", search_results);

        assert!(!search_results.is_empty(), "No search results");
        let item = search_results
            .into_iter()
            .find(|item| item.title.to_ascii_lowercase() == manga.title.to_ascii_lowercase());
        assert!(item.is_some(), "Could not find manga in search results");
        let item = item.unwrap();
        assert!(item.url.has_host(), "Search url is missing host");
    }
}
