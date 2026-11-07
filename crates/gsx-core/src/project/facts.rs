use crate::measure::{MediaKind, Placement};

#[derive(Clone, Copy, Debug, Default)]
pub struct Facts<'a> {
    pub media: Option<MediaKind>,
    pub media_count: usize,
    pub placement: Option<Placement>,
    pub title: Option<&'a str>,
    pub target_url: Option<&'a str>,
    pub max_characters: Option<u32>,
    pub characters_reserved_per_url: Option<u32>,
    pub caption_limit: Option<u32>,
    pub description_limit: Option<u32>,
    pub entities_allowed: bool,
    pub items: Option<&'a [MediaFact<'a>]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Image,
    Gif,
    Video,
    Audio,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MediaFact<'a> {
    pub id: &'a str,
    pub format: Format,
    pub alt: Option<&'a str>,
    pub spoiler: bool,
    pub inline: bool,
}
