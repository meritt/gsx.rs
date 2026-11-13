mod count;
mod emoji;
mod grapheme;
mod rules;
mod tables;

use std::ops::Range;

pub(crate) use grapheme::Segmenter;
pub use grapheme::{Clusters, clusters};
pub use rules::{Condition, Context, Field, Limit, Maximum, Measurement, MediaKind, Placement, Rendering};
pub(crate) use rules::{given, limit, resolved, unlimited};
pub use tables::UNICODE_VERSION;

pub fn blank(text: &str) -> bool {
    text.chars().all(|char| char.is_whitespace() || emoji::contains(&tables::DEFAULT_IGNORABLE, char))
}

pub const X_URL_WEIGHT: usize = 23;
pub const MASTODON_RESERVED_PER_URL: u32 = 23;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    Graphemes,
    CodePoints,
    Utf16CodeUnits,
    Utf8Bytes,
    XWeighted,
    MastodonWeighted,
    ThreadsWeighted,
    Hashtags,
    Mentions,
    Links,
}

impl Unit {
    pub const fn counts_markers(self) -> bool {
        matches!(self, Unit::Hashtags | Unit::Mentions | Unit::Links)
    }

    pub const fn max_bytes(self) -> Option<usize> {
        match self {
            Unit::Utf8Bytes => Some(1),
            Unit::Utf16CodeUnits => Some(3),
            Unit::CodePoints => Some(4),
            _ => None,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Unit::Graphemes => "graphemes",
            Unit::CodePoints => "code-points",
            Unit::Utf16CodeUnits => "utf16-code-units",
            Unit::Utf8Bytes => "utf8-bytes",
            Unit::XWeighted => "x-weighted",
            Unit::MastodonWeighted => "mastodon-weighted",
            Unit::ThreadsWeighted => "threads-weighted",
            Unit::Hashtags => "hashtags",
            Unit::Mentions => "mentions",
            Unit::Links => "links",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fact {
    MaxCharacters,
    CharactersReservedPerUrl,
    DescriptionLimit,
    CaptionLimit,
    HashtagLimit,
}

impl Fact {
    pub const fn name(self) -> &'static str {
        match self {
            Fact::MaxCharacters => "maxCharacters",
            Fact::CharactersReservedPerUrl => "charactersReservedPerUrl",
            Fact::DescriptionLimit => "descriptionLimit",
            Fact::CaptionLimit => "captionLimit",
            Fact::HashtagLimit => "hashtagLimit",
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Facts<'a> {
    pub urls: &'a [Range<usize>],
    pub elided: &'a [Range<usize>],
    pub resolved: &'a [(Fact, u32)],
}

impl Facts<'_> {
    pub fn resolved(&self, fact: Fact) -> Option<u32> {
        self.resolved.iter().find(|(name, _)| *name == fact).map(|&(_, value)| value)
    }
}

pub fn measure(unit: Unit, text: &str, facts: &Facts) -> u32 {
    let count = match unit {
        Unit::Graphemes => count::graphemes(text),
        Unit::CodePoints => count::code_points(text),
        Unit::Utf16CodeUnits => count::utf16_code_units(text),
        Unit::Utf8Bytes => text.len(),
        Unit::XWeighted => count::x_weighted(text, facts.urls),
        Unit::MastodonWeighted => count::mastodon_weighted(
            text,
            facts.urls,
            facts.elided,
            facts.resolved(Fact::CharactersReservedPerUrl).unwrap_or(MASTODON_RESERVED_PER_URL) as usize,
        ),
        Unit::ThreadsWeighted => count::threads_weighted(text),
        Unit::Hashtags | Unit::Mentions => markers(unit, text).len(),
        Unit::Links => count::unique_urls(text, facts.urls),
    };
    u32::try_from(count).unwrap_or(u32::MAX)
}

pub fn prefixes(unit: Unit, text: &str, facts: &Facts, ends: &[usize]) -> Vec<Option<usize>> {
    let (values, _) = match unit {
        Unit::Graphemes => count::cluster_prefixes(text, ends, false),
        Unit::Utf8Bytes => (ends.iter().map(|&end| text.is_char_boundary(end).then_some(end)).collect(), text.len()),
        Unit::XWeighted => count::x_weighted_prefixes(text, facts.urls, ends),
        Unit::MastodonWeighted => count::mastodon_weighted_prefixes(
            text,
            facts.urls,
            facts.elided,
            facts.resolved(Fact::CharactersReservedPerUrl).unwrap_or(MASTODON_RESERVED_PER_URL) as usize,
            ends,
        ),
        Unit::ThreadsWeighted => count::cluster_prefixes(text, ends, true),
        Unit::Links => count::unique_url_prefixes(text, facts.urls, ends),
        Unit::CodePoints | Unit::Utf16CodeUnits | Unit::Hashtags | Unit::Mentions => (vec![None; ends.len()], 0),
    };
    values
}

pub fn markers(unit: Unit, text: &str) -> Vec<usize> {
    match unit {
        Unit::Hashtags => count::markers(text, '#', |char| char.is_alphanumeric() || char == '_'),
        Unit::Mentions => count::markers(text, '@', |char| char.is_ascii_alphanumeric() || matches!(char, '.' | '_')),
        _ => vec![],
    }
}
