use serde::Deserialize;

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize)]
pub struct Accept {
    #[serde(default)]
    pub selectors: Vec<String>,
    #[serde(default)]
    pub hostnames: Vec<String>,
}

