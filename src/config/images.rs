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
}
