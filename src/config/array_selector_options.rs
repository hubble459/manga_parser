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
    /// Drop repeated items, keeping the first occurrence -- for pages that
    /// list the same value more than once (e.g. one person as both author
    /// and artist).
    #[serde(default)]
    pub unique: bool,
}

/// Splits on `,` `;` `|`, and on a dash only with spaces around it, so hyphenated values
/// ("Sci-Fi", "4-Koma", "Jae-Hoon") stay whole.
fn default_text_split_regex() -> Option<Regex> {
    Regex::new(r" *[,;|]+ *| +[-–] +").ok()
}

impl Default for ArraySelectorOptions {
    fn default() -> Self {
        Self {
            cleanup: vec![],
            text_selection: StringSelection::default(),
            fix_capitalization: FixCapitalization::default(),
            text_split_regex: default_text_split_regex(),
            unique: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::default_text_split_regex;

    #[test]
    fn default_split_keeps_hyphenated_values() {
        let regex = default_text_split_regex().unwrap();
        let split = |text: &str| regex.split(text).map(str::to_string).collect::<Vec<_>>();
        assert_eq!(split("Action, Sci-Fi; 4-Koma | Drama"), ["Action", "Sci-Fi", "4-Koma", "Drama"]);
        assert_eq!(split("Kim Jae-Hoon - Lee Min"), ["Kim Jae-Hoon", "Lee Min"]);
        assert_eq!(split("martial arts,shonen ,school"), ["martial arts", "shonen", "school"]);
    }
}
