#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(PartialEq)]
pub struct Chapter {
    pub href: String,
    pub number: f32,
    pub title: String,
    pub date: chrono::DateTime<chrono::Utc>,
}
