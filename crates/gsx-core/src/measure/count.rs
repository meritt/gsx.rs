use std::iter::Peekable;
use std::ops::Range;

use super::{X_URL_WEIGHT, clusters, emoji, grapheme};

pub(super) fn code_points(text: &str) -> usize {
    text.chars().count()
}

pub(super) fn utf16_code_units(text: &str) -> usize {
    text.chars().map(char::len_utf16).sum()
}

pub(super) fn markers(text: &str, sigil: char, word: fn(char) -> bool) -> Vec<usize> {
    text.match_indices(sigil)
        .map(|(at, _)| at)
        .filter(|&at| {
            text[..at].chars().next_back().is_none_or(char::is_whitespace)
                && text[at + sigil.len_utf8()..].chars().next().is_some_and(word)
        })
        .collect()
}

pub(super) fn graphemes(text: &str) -> usize {
    clusters(text).count()
}

fn valid<'a>(text: &'a str, spans: &'a [Range<usize>]) -> impl Iterator<Item = &'a Range<usize>> {
    spans.iter().filter(|span| span.start < span.end && text.get(span.start..span.end).is_some())
}

pub(super) fn unique_urls(text: &str, urls: &[Range<usize>]) -> usize {
    let mut seen = vec![];
    admit_urls(text, &mut valid(text, urls).peekable(), text.len(), &mut seen);
    seen.len()
}

pub(super) fn unique_url_prefixes(text: &str, urls: &[Range<usize>], ends: &[usize]) -> Measured {
    let mut prefixes = Prefixes::new(ends);
    let mut urls = valid(text, urls).peekable();
    let mut seen: Vec<&str> = vec![];
    for (at, char) in text.char_indices() {
        if prefixes.wanted(at) && char.is_whitespace() {
            admit_urls(text, &mut urls, at, &mut seen);
            prefixes.set(seen.len());
        }
    }
    admit_urls(text, &mut urls, text.len(), &mut seen);
    prefixes.done(text.len(), seen.len())
}

fn admit_urls<'t>(
    text: &'t str,
    urls: &mut Peekable<impl Iterator<Item = &'t Range<usize>>>,
    at: usize,
    seen: &mut Vec<&'t str>,
) {
    while let Some(url) = urls.next_if(|url| url.end <= at) {
        let url = &text[url.clone()];
        if !seen.contains(&url) {
            seen.push(url);
        }
    }
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
    mastodon_weighted_prefixes(text, urls, elided, reserved_per_url, &[]).1
}

pub(super) fn threads_weighted(text: &str) -> usize {
    cluster_prefixes(text, &[], true).1
}

type Measured = (Vec<Option<usize>>, usize);

struct Prefixes<'a> {
    ends: &'a [usize],
    next: usize,
    upcoming: usize,
    values: Vec<Option<usize>>,
}

impl<'a> Prefixes<'a> {
    fn new(ends: &'a [usize]) -> Self {
        Prefixes {
            ends,
            next: 0,
            upcoming: ends.first().copied().unwrap_or(usize::MAX),
            values: vec![None; ends.len()],
        }
    }

    fn wanted(&mut self, at: usize) -> bool {
        at >= self.upcoming && self.reached(at)
    }

    fn reached(&mut self, at: usize) -> bool {
        while self.ends.get(self.next).is_some_and(|&end| end < at) {
            self.next += 1;
        }
        self.upcoming = self.ends.get(self.next).copied().unwrap_or(usize::MAX);
        self.upcoming == at
    }

    fn set(&mut self, value: usize) {
        self.values[self.next] = Some(value.min(u32::MAX as usize));
        self.next += 1;
        self.upcoming = self.ends.get(self.next).copied().unwrap_or(usize::MAX);
    }

    fn record(&mut self, at: usize, value: usize) {
        if self.wanted(at) {
            self.set(value);
        }
    }

    fn done(mut self, at: usize, value: usize) -> Measured {
        self.record(at, value);
        (self.values, value)
    }
}

pub(super) fn cluster_prefixes(text: &str, ends: &[usize], weighted: bool) -> Measured {
    let mut prefixes = Prefixes::new(ends);
    let (mut at, mut value) = (0, 0);
    for cluster in clusters(text) {
        prefixes.record(at, value);
        let mut chars = cluster.chars();
        let single = chars.next().filter(|_| chars.next().is_none());
        value += if weighted && (single.is_some_and(emoji::presentation) || emoji::rgi_prefix(cluster) == cluster.len())
        {
            cluster.len()
        } else {
            1
        };
        at += cluster.len();
    }
    prefixes.done(text.len(), value)
}

pub(super) fn x_weighted_prefixes(text: &str, urls: &[Range<usize>], ends: &[usize]) -> Measured {
    let mut prefixes = Prefixes::new(ends);
    let mut urls = valid(text, urls).peekable();
    let bytes = text.as_bytes();
    let mut weight = 0;
    let mut at = 0;
    while let Some(char) = text[at..].chars().next() {
        if prefixes.wanted(at) && char.is_whitespace() {
            prefixes.set(weight);
        }
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
    prefixes.done(text.len(), weight)
}

pub(super) fn mastodon_weighted_prefixes(
    text: &str,
    urls: &[Range<usize>],
    elided: &[Range<usize>],
    reserved_per_url: usize,
    ends: &[usize],
) -> Measured {
    let mut prefixes = Prefixes::new(ends);
    let sorted = |spans: &[Range<usize>]| spans.is_sorted_by_key(|span| span.start);
    let (mut covering, mut reach) = ([0, 0], if sorted(urls) && sorted(elided) { 0 } else { usize::MAX });
    let mut covers = |at: usize| {
        for (spans, next) in [urls, elided].into_iter().zip(&mut covering) {
            while let Some(span) = spans.get(*next).filter(|span| span.start < at) {
                reach = reach.max(span.end);
                *next += 1;
            }
        }
        at < reach
    };
    let mut segmenter = grapheme::Segmenter::default();
    let (mut count, mut linked): (usize, usize) = (0, 0);
    let weighed =
        |count: usize, linked: usize| count.saturating_add(linked.saturating_mul(reserved_per_url.saturating_sub(1)));
    let mut feed = |(from, to): (usize, usize), segmenter: &mut grapheme::Segmenter, count: &mut usize, linked| {
        if prefixes.upcoming >= to {
            *count += text[from..to].chars().filter(|&char| segmenter.starts(char)).count();
            return;
        }
        for (offset, char) in text[from..to].char_indices() {
            let at = from + offset;
            if prefixes.wanted(at) && char.is_whitespace() && !covers(at) {
                prefixes.set(weighed(*count, linked));
            }
            if segmenter.starts(char) {
                *count += 1;
            }
        }
    };
    let mut spans = (valid(text, urls).peekable(), valid(text, elided).peekable());
    let mut cursor = 0;
    loop {
        let (urls, elided) = &mut spans;
        let (span, url) =
            if let Some(span) = urls.next_if(|url| elided.peek().is_none_or(|skip| url.start <= skip.start)) {
                (span, true)
            } else if let Some(span) = elided.next() {
                (span, false)
            } else {
                break;
            };
        if span.start < cursor {
            continue;
        }
        feed((cursor, span.start), &mut segmenter, &mut count, linked);
        if url {
            if reserved_per_url > 0 && segmenter.starts('x') {
                count += 1;
            }
            linked += 1;
        }
        cursor = span.end;
    }
    feed((cursor, text.len()), &mut segmenter, &mut count, linked);
    prefixes.done(text.len(), weighed(count, linked))
}

#[cfg(test)]
mod tests {
    use super::mastodon_weighted;

    #[test]
    fn saturates_url_weight_when_the_reserved_length_fills_usize() {
        let text = "ab https://e.co https://e.org";
        assert_eq!(mastodon_weighted(text, &[3..15, 16..29], &[], usize::MAX), usize::MAX);
        assert_eq!(mastodon_weighted(text, std::slice::from_ref(&(3..15)), &[], usize::MAX), usize::MAX);
        assert_eq!(mastodon_weighted(text, &[3..15, 16..29], &[], 23), 3 + 1 + 2 * 23);
    }
}
