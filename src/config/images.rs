use serde::Deserialize;

use super::{
    array_selector::ArraySelectors,
    chapter::{EmbeddedJson, FetchExternal},
};

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize)]
pub struct Images {
    pub image_selector: ArraySelectors,
    #[serde(default)]
    pub fetch_external: Vec<FetchExternal>,
    #[serde(default)]
    pub embedded_json: Option<EmbeddedJson>,
    /// For paged readers that show one image per page (e.g. `/c001/2.html`):
    /// selects the URL of every reader page of the chapter. When it matches,
    /// each page is fetched and `image_selector` runs against every one of
    /// them, in order. When it matches nothing, the chapter page itself is
    /// used as-is.
    #[serde(default)]
    pub pages: Option<ArraySelectors>,
}
