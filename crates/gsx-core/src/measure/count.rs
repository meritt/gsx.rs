use std::ops::Range;

use super::{X_URL_WEIGHT, clusters, emoji, grapheme};

pub(super) fn code_points(text: &str) -> usize {
    text.chars().count()
}

pub(super) fn utf16_code_units(text: &str) -> usize {
    text.chars().map(char::len_utf16).sum()
}

pub(super) fn markers(text: &str, sigil: char, word: fn(char) -> bool) -> Vec<usize> {
    let mut positions = vec![];
    let mut previous: Option<char> = None;
    let mut chars = text.char_indices().peekable();
    while let Some((at, char)) = chars.next() {
        if char == sigil
            && previous.is_none_or(char::is_whitespace)
            && chars.peek().is_some_and(|&(_, next)| word(next))
        {
            positions.push(at);
        }
        previous = Some(char);
    }
    positions
}

pub(super) fn graphemes(text: &str) -> usize {
    clusters(text).count()
}

fn valid<'a>(text: &'a str, spans: &'a [Range<usize>]) -> impl Iterator<Item = &'a Range<usize>> {
    spans.iter().filter(|span| span.start < span.end && text.get(span.start..span.end).is_some())
}

fn x_char(char: char) -> usize {
    match char {
        '\0'..='\u{10FF}' | '\u{2000}'..='\u{200D}' | '\u{2010}'..='\u{201F}' | '\u{2032}'..='\u{2037}' => 1,
        _ => 2,
    }
}

pub(super) fn x_weighted(text: &str, urls: &[Range<usize>]) -> usize {
    let mut urls = valid(text, urls).peekable();
    let bytes = text.as_bytes();
    let mut weight = 0;
    let mut at = 0;
    while let Some(char) = text[at..].chars().next() {
        while urls.next_if(|url| url.start < at).is_some() {}
        if let Some(url) = urls.next_if(|url| url.start == at) {
            weight += X_URL_WEIGHT;
            at = url.end;
            continue;
        }
        if char.is_ascii() && bytes.get(at + 1).is_none_or(u8::is_ascii) {
            weight += 1;
            at += 1;
            continue;
        }
        let emoji = emoji::rgi_prefix(&text[at..]);
        if emoji > 0 {
            weight += 2;
            at += emoji;
        } else {
            weight += x_char(char);
            at += char.len_utf8();
        }
    }
    weight
}

pub(super) fn mastodon_weighted(
    text: &str,
    urls: &[Range<usize>],
    elided: &[Range<usize>],
    reserved_per_url: usize,
) -> usize {
    let mut urls = valid(text, urls).peekable();
    let mut elided = valid(text, elided).peekable();
    let mut segmenter = grapheme::Segmenter::default();
    let mut count = 0;
    let mut feed = |part: &str| count += part.chars().filter(|&char| segmenter.starts(char)).count();
    let mut linked = 0;
    let mut cursor = 0;
    loop {
        let url = match (urls.peek(), elided.peek()) {
            (Some(url), Some(skip)) => url.start <= skip.start,
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => break,
        };
        let span = if url { urls.next() } else { elided.next() }.unwrap();
        if span.start < cursor {
            continue;
        }
        feed(&text[cursor..span.start]);
        if url {
            feed(&"x"[..usize::from(reserved_per_url > 0)]);
            linked += 1;
        }
        cursor = span.end;
    }
    feed(&text[cursor..]);
    count + linked * reserved_per_url.saturating_sub(1)
}

pub(super) fn threads_weighted(text: &str) -> usize {
    clusters(text)
        .map(|cluster| {
            let mut chars = cluster.chars();
            let single = chars.next().filter(|_| chars.next().is_none());
            if single.is_some_and(emoji::presentation) || emoji::rgi_prefix(cluster) == cluster.len() {
                cluster.len()
            } else {
                1
            }
        })
        .sum()
}
