use serde::Deserialize;

use super::string_selector_options::StringSelectorOptions;

#[cfg_attr(feature = "debug", derive(Debug))]
pub struct StringSelector {
    pub selector: String,
    pub options: StringSelectorOptions,
}


#[cfg_attr(feature = "debug", derive(Debug))]
pub struct StringSelectors {
    pub selectors: Vec<StringSelector>,
}

impl<'de> Deserialize<'de> for StringSelectors {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = StringSelectors;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("Selector object")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                where
                    A: serde::de::SeqAccess<'de>, {
                let mut selectors = vec![];
                while let Ok(Some(element)) = seq.next_element() {
                    selectors.push(element);
                }

                Ok(Self::Value {
                    selectors
                })
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Self::Value {
                    selectors: vec![StringSelector {
                        selector: v.to_string(),
                        options: StringSelectorOptions::default(),
                    }],
                })
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut selector: Option<String> = None;
                let mut options: Option<StringSelectorOptions> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "selector" => {
                            if selector.is_some() {
                                return Err(serde::de::Error::duplicate_field("selector"));
                            }
                            selector = Some(map.next_value()?);
                        }
                        "options" => {
                            if options.is_some() {
                                return Err(serde::de::Error::duplicate_field("options"));
                            }
                            options = Some(map.next_value()?);
                        }
                        _ => {
                            // Ignore unknown fields
                            let _ = map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }

                match selector {
                    Some(selector) => Ok(Self::Value {
                        selectors: vec![StringSelector {
                            selector,
                            options: options.unwrap_or_default(),
                        }],
                    }),
                    _ => Err(serde::de::Error::missing_field("selector")),
                }
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

impl<'de> Deserialize<'de> for StringSelector {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = StringSelector;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("Selector object")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Self::Value {
                    selector: v.to_string(),
                    options: StringSelectorOptions::default(),
                })
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut selector: Option<String> = None;
                let mut options: Option<StringSelectorOptions> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "selector" => {
                            if selector.is_some() {
                                return Err(serde::de::Error::duplicate_field("selector"));
                            }
                            selector = Some(map.next_value()?);
                        }
                        "options" => {
                            if options.is_some() {
                                return Err(serde::de::Error::duplicate_field("options"));
                            }
                            options = Some(map.next_value()?);
                        }
                        _ => {
                            // Ignore unknown fields
                            let _ = map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }

                match selector {
                    Some(selector) => Ok(Self::Value {
                        selector,
                        options: options.unwrap_or_default(),
                    }),
                    _ => Err(serde::de::Error::missing_field("selector")),
                }
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use config::{builder::DefaultState, ConfigBuilder, File};

    #[derive(Debug, serde::Deserialize)]
    #[allow(unused)]
    struct TestSelector {
        selectors: Vec<super::StringSelectors>,
    }

    #[test]
    fn test_selector_deserialization() {
        let selector = ConfigBuilder::<DefaultState>::default()
            .add_source(File::from(Path::new(
                "tests/fragments/config/string_selectors.yaml",
            )))
            .build()
            .unwrap()
            .try_deserialize::<TestSelector>()
            .unwrap();

        println!("{:#?}", selector);
    }
}
