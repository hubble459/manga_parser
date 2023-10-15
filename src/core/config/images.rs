use serde::Deserialize;

use super::{array_selector::ArraySelectors, chapter::FetchExternal};

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize)]
pub struct Images {
    pub image_selector: ArraySelectors,
    #[serde(default)]
    pub fetch_external: Vec<FetchExternal>,
}
