use regex::Regex;
use serde::Deserialize;

use super::string_selector::StringSelector;

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize)]
pub struct Accept {
    #[serde(default)]
    pub selectors: Vec<String>,
    #[serde(default)]
    pub hostnames: Vec<String>,
}

