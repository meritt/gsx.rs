pub mod bluesky;
pub mod mastodon;
mod tables;
pub mod twitter;
pub(super) mod unicode;

use std::ops::Range;

pub fn accepted(raw: &str) -> Option<String> {
    let url = url::Url::parse(raw).ok()?;
    (matches!(url.scheme(), "http" | "https")
        && url.has_host()
        && url.username().is_empty()
        && url.password().is_none())
    .then(|| url.into())
}

const X_MEASURED_UTF16: usize = 280 * 4096 / 23;

struct Units {
    units: Vec<u16>,
    bytes: Vec<usize>,
}

impl Units {
    fn new(text: &str) -> Self {
        let units: Vec<u16> = text.encode_utf16().collect();
        let mut bytes = vec![usize::MAX; units.len() + 1];
        let mut offset = 0;
        for (byte, char) in text.char_indices() {
            bytes[offset] = byte;
            offset += char.len_utf16();
        }
        bytes[units.len()] = text.len();
        Units { units, bytes }
    }

    fn bytes(&self, range: Range<usize>) -> Option<Range<usize>> {
        let (start, end) = (self.bytes[range.start], self.bytes[range.end]);
        (start != usize::MAX && end != usize::MAX).then_some(start..end)
    }
}

pub fn x_urls(text: &str) -> Vec<Range<usize>> {
    let units = Units::new(text);
    if units.units.len() > X_MEASURED_UTF16 {
        return vec![];
    }
    twitter::urls(&units.units).into_iter().filter_map(|range| units.bytes(range)).collect()
}

pub fn mastodon_entities(text: &str) -> (Vec<Range<usize>>, Vec<Range<usize>>) {
    let units = Units::new(text);
    let urls = mastodon::urls(&units.units).into_iter().filter_map(|range| units.bytes(range)).collect();
    let elided = mastodon::mentions(&units.units)
        .into_iter()
        .filter(|mention| mention.local < mention.range.end)
        .filter_map(|mention| units.bytes(mention.local..mention.range.end))
        .collect();
    (urls, elided)
}

pub fn href(url: &str) -> String {
    url::Url::parse(url).map_or_else(|_| url.to_owned(), String::from)
}
