use crate::measure::{MediaKind, Placement};

pub const FACT_MAX_UNITS: usize = 65_536;
pub const FACT_MAX_ITEMS: usize = 1024;

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
    pub hashtag_limit: Option<u32>,
    pub spoiler_text: Option<&'a str>,
    pub overflow: Overflow,
    pub entities_allowed: bool,
    pub items: Option<&'a [MediaFact<'a>]>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Overflow {
    #[default]
    Truncate,
    Thread,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Image,
    Gif,
    Video,
    Audio,
}

impl Format {
    pub const ALL: &[Format] = &[Format::Image, Format::Gif, Format::Video, Format::Audio];

    pub const fn name(self) -> &'static str {
        match self {
            Format::Image => "image",
            Format::Gif => "gif",
            Format::Video => "video",
            Format::Audio => "audio",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MediaFact<'a> {
    pub id: &'a str,
    pub format: Format,
    pub alt: Option<&'a str>,
    pub spoiler: bool,
    pub inline: bool,
}
