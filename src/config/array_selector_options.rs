use regex::Regex;
use serde::Deserialize;

use super::string_selector_options::{StringSelection, FixCapitalization, CleanupOption};

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize)]
pub struct ArraySelectorOptions {
    /// Cleanup text that has been scraped with regexps
    #[serde(default)]
    pub cleanup: Vec<CleanupOption>,
    /// Fix bad capitalization
    #[serde(default)]
    pub fix_capitalization: FixCapitalization,
    /// Determine how text should be selected
    #[serde(default)]
    pub text_selection: StringSelection,
    /// Determine how text should splitted
    /// When one element is selected,
    /// should a piece of text be splitted to result in an array of Strings
    /// eg. "martial arts, shonen, school" should be splitted on ', '
    #[serde(default = "default_text_split_regex", deserialize_with = "serde_regex::deserialize")]
    pub text_split_regex: Option<Regex>,
}

fn default_text_split_regex() -> Option<Regex> {
    Regex::new(r" *[,;\-|]+ *").ok()
}

impl Default for ArraySelectorOptions {
    fn default() -> Self {
        Self {
            cleanup: vec![],
            text_selection: StringSelection::default(),
            fix_capitalization: FixCapitalization::default(),
            text_split_regex: default_text_split_regex(),
        }
    }
}
