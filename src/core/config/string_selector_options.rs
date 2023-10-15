use regex::Regex;
use serde::Deserialize;

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize)]
pub struct StringSelectorOptions {
    /// Cleanup text that has been scraped with regexps
    #[serde(default)]
    pub cleanup: Vec<CleanupOption>,
    /// Fix bad capitalization
    #[serde(default)]
    pub fix_capitalization: FixCapitalization,
    /// Determine how text should be selected
    #[serde(default)]
    pub text_selection: StringSelection,
}

impl Default for StringSelectorOptions {
    fn default() -> Self {
        Self {
            cleanup: vec![],
            text_selection: StringSelection::default(),
            fix_capitalization: FixCapitalization::default(),
        }
    }
}

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Default, Deserialize)]
pub enum FixCapitalization {
    #[serde(rename = "title")]
    Title,
    #[default]
    #[serde(rename = "skip")]
    Skip,
}

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize)]
pub struct CleanupOption {
    #[serde(deserialize_with = "serde_regex::deserialize")]
    pub replace_regex: Regex,
    pub replace_with: String,
}

#[cfg_attr(feature = "debug", derive(Debug))]
pub enum StringSelection {
    AllText { join_with: String },
    OwnText,
    Attributes(Vec<String>),
}

impl Default for StringSelection {
    fn default() -> Self {
        Self::AllText {
            join_with: String::from(" "),
        }
    }
}

impl<'de> Deserialize<'de> for StringSelection {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = StringSelection;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("Text selection object")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut text_type: Option<String> = None;
                let mut join_with = None;
                let mut attributes = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "type" => {
                            if text_type.is_some() {
                                return Err(serde::de::Error::duplicate_field("type"));
                            }
                            text_type = Some(map.next_value()?);
                        }
                        "join_with" => {
                            if join_with.is_some() {
                                return Err(serde::de::Error::duplicate_field("join_with"));
                            }
                            join_with = Some(map.next_value()?);
                        }
                        "attributes" => {
                            if attributes.is_some() {
                                return Err(serde::de::Error::duplicate_field("attributes"));
                            }
                            attributes = Some(map.next_value()?);
                        }
                        _ => {
                            // Ignore unknown fields
                            let _ = map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }

                match text_type.as_deref() {
                    Some("own-text") => Ok(StringSelection::OwnText),
                    Some("all-text") => Ok(StringSelection::AllText { join_with: join_with.unwrap_or(" ".to_string()) }),
                    Some("attributes") => Ok(StringSelection::Attributes(
                        attributes.unwrap_or_default(),
                    )),
                    _ => Err(serde::de::Error::missing_field("type")),
                }
            }
        }

        deserializer.deserialize_map(Visitor)
    }
}
