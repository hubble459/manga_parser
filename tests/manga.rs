use manga_parser::{
    error::ScrapeError, model::Manga, scraper::scraper_manager::ScraperManager,
    scraper::MangaScraper,
};
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
                    std::env::set_var("RUST_LOG", "DEBUG");
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
fn t() {
    let date =
        manga_parser::util::date::try_parse_date("18 April، 2022", &vec!["%d %B، %Y".to_string()]);
    log::debug!("date: {:?}", date);

    let date =
        manga_parser::util::date::try_parse_date("an hour ago", &vec!["%d %B، %Y".to_string()]);
    log::debug!("date: {:?}", date);
}

test_manga_mod! {
    madara,
    isekaiscan: "https://isekaiscan.top/manga/moshi-fanren";
    isekaiscanmanga: "https://isekaiscanmanga.com/manga/silver-devil-king/";
    #[ignore = "CloudflareIUAM"]
    aquamanga: "https://aquamanga.com/read/my-insanely-competent-underlings";
    hubmanga: "https://hubmanga.com/reincarnation-of-the-suicidal-battle-god";
    mangapure: "https://mangapure.net/read/reincarnation-of-the-suicidal-battle-god";
    mangaonlineteam: "https://mangaonlineteam.com/manga/miss-divine-doctor-conquer-the-demon-king/";
    manhuaus: "https://manhuaus.com/manga/return-of-immortal-warlord/";
    #[ignore = "CloudflareIUAM"]
    mangaweebs: "https://mangaweebs.in/manga/2dmgoc9v5rbcjrdng8ra/";
    #[ignore = "CloudflareIUAM"]
    manhuaplus: "https://manhuaplus.com/manga/ultimate-loading-system/";
    mangasushi: "https://mangasushi.org/manga/shin-no-nakama-janai-to-yuusha-no-party-wo-oidasareta-node-henkyou-de-slow-life-suru-koto-ni-shimashita/";
    mangafoxfull: "https://mangafoxfull.com/manga/magic-emperor/";
    _1stkissmangaclub: "https://1stkissmanga.club/manga/outside-the-law/";
    #[ignore = "CloudflareIUAM"]
    _1stkissmanga: "https://1stkissmanga.io/manga/outside-the-law/";
    // #[ignore = "CloudflareIUAM"]
    s2manga: "https://s2manga.com/manga/under-the-oak-tree/";
    manhwatop: "https://manhwatop.com/manga/magic-emperor/";
    mixedmanga: "https://mixedmanga.com/manga/my-husband-is-an-antisocial-count/";
    manga68: "https://manga68.com/manga/magic-emperor/";
    manhuadex: "https://manhuadex.com/manhua/the-eunuchs-consort-rules-the-world/";
    mangachill: "https://mangachill.net/manga/the-eunuchs-consort-rules-thechbacc/";
    mangarockteam: "https://mangarockteam.com/manga/academys-undercover-professor/";
    mangazukiteam: "https://mangazukiteam.com/manga/shinjiteita-nakama-tachi-ni-dungeon/";
    azmanhwa: "https://azmanhwa.net/manga/i-have-max-level-luck";
    topmanhua: "https://topmanhua.com/manhua/the-beginning-after-the-end/";
    yaoi: "https://yaoi.mobi/manga/stack-overflow-raw-yaoi0003/";
    mangatx: "https://mangatx.com/manga/lightning-degree/";
    manhuafast: "https://manhuafast.com/manga/descending-the-mountain-as-invincible-all-chapters/";
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
        assert!(
            !manga.alternative_titles.is_empty(),
            "Missing alternative titles"
        );
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
    let images = SCRAPER_MANAGER
        .chapter_images(&first_chapter.url)
        .await
        .unwrap();
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
