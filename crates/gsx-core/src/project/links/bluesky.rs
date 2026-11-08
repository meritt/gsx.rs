use std::ops::Range;

use super::accepted;
use super::unicode;
use crate::measure::clusters;

const MAX_TAG_BYTES: usize = 640;
const MAX_TAG_LENGTH: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Autolink {
    Link { range: Range<usize>, uri: String },
    Tag { range: Range<usize>, tag: String },
    Mention { range: Range<usize>, handle: String },
}

impl Autolink {
    pub fn range(&self) -> &Range<usize> {
        match self {
            Autolink::Link { range, .. } | Autolink::Tag { range, .. } | Autolink::Mention { range, .. } => range,
        }
    }
}

pub fn space(char: char) -> bool {
    unicode::space(u32::from(char))
}

pub fn visible(text: &str) -> bool {
    text.chars().any(|char| !unicode::invisible(u32::from(char)))
}

fn punctuation(char: char) -> bool {
    unicode::punctuation(u32::from(char))
}

fn starts_with_ignore_case(text: &str, prefix: &str) -> bool {
    text.len() >= prefix.len() && text.as_bytes()[..prefix.len()].eq_ignore_ascii_case(prefix.as_bytes())
}

fn line_start(text: &str, at: usize) -> bool {
    text[..at].chars().next_back().is_none_or(|char| matches!(char, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
}

fn link_at(text: &str, at: usize) -> Option<usize> {
    let rest = &text[at..];
    let protocol = ["https://", "http://", "www."].into_iter().find(|prefix| starts_with_ignore_case(rest, prefix))?;
    let run = rest[protocol.len()..].find(space).unwrap_or(rest.len() - protocol.len());
    (run > 0).then_some(at + protocol.len() + run)
}

fn links(text: &str) -> Vec<Autolink> {
    let mut found = vec![];
    let mut at = 0;
    while at <= text.len() {
        let next = text[at..].chars().next();
        let matched = if line_start(text, at)
            && let Some(end) = link_at(text, at)
        {
            Some((at, end))
        } else if let Some(char) = next.filter(|&char| space(char) || char == '(')
            && let Some(end) = link_at(text, at + char.len_utf8())
        {
            Some((at + char.len_utf8(), end))
        } else {
            None
        };
        if let Some((start, end)) = matched {
            let mut value = &text[start..end];
            while value.ends_with(['.', ',', ';', ':', '!', '?']) || (value.ends_with(')') && !value.contains('(')) {
                value = &value[..value.len() - 1];
            }
            let raw =
                if starts_with_ignore_case(value, "www.") { format!("https://{value}") } else { value.to_owned() };
            if let Some(uri) = accepted(&raw) {
                found.push(Autolink::Link { range: start..start + value.len(), uri });
            }
            at = end;
            continue;
        }
        at += next.map_or(1, char::len_utf8);
    }
    found
}

fn tag_char(char: char) -> bool {
    !space(char)
        && !matches!(char, '\u{AD}' | '\u{2060}' | '\u{200A}' | '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{20E2}')
}

fn tags(text: &str, links: &[Autolink]) -> Vec<Autolink> {
    let mut found = vec![];
    let mut at = 0;
    while at < text.len() {
        let next = text[at..].chars().next().unwrap();
        let hash = if at == 0 && matches!(next, '#' | '＃') {
            Some(at)
        } else if space(next) && text[at + next.len_utf8()..].starts_with(['#', '＃']) {
            Some(at + next.len_utf8())
        } else {
            None
        };
        let Some(hash) = hash else {
            at += next.len_utf8();
            continue;
        };
        let body = hash + text[hash..].chars().next().unwrap().len_utf8();
        let run = text[body..].find(|char| !tag_char(char)).map_or(text.len(), |end| body + end);
        let group = &text[body..run];
        let valid =
            !group.starts_with('\u{FE0F}') && group.chars().any(|char| !char.is_ascii_digit() && !punctuation(char));
        at = if valid { run } else { body };
        if !valid {
            continue;
        }
        let value = group.trim_end_matches(punctuation);
        if value.is_empty()
            || value.len() > MAX_TAG_BYTES
            || (value.encode_utf16().count() > MAX_TAG_LENGTH && clusters(value).count() > MAX_TAG_LENGTH)
        {
            continue;
        }
        let range = hash..body + value.len();
        if links.iter().any(|link| link.range().start < range.end && range.start < link.range().end) {
            continue;
        }
        found.push(Autolink::Tag { range, tag: value.to_owned() });
    }
    found
}

fn handle(value: &str) -> bool {
    let label = |label: &str| {
        (1..=63).contains(&label.len())
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    };
    let mut labels = value.split('.');
    let last = labels.next_back().filter(|last| last.starts_with(|char: char| char.is_ascii_alphabetic()));
    value.len() <= 253 && value.contains('.') && last.is_some_and(label) && labels.all(label)
}

fn mentions(text: &str, links: &[Autolink]) -> Vec<Autolink> {
    let bytes = text.as_bytes();
    let word = |at: usize| bytes.get(at).is_some_and(|&byte| byte.is_ascii_alphanumeric() || byte == b'_');
    let mut found = vec![];
    for (at, _) in text.match_indices('@') {
        if text[..at].chars().next_back().is_some_and(|char| !space(char) && char != '(') {
            continue;
        }
        let body = at + 1;
        let run = text[body..]
            .find(|char: char| !(char.is_ascii_alphanumeric() || matches!(char, '.' | '-')))
            .map_or(text.len(), |end| body + end);
        let mut end = run;
        while end > body && word(end - 1) == word(end) {
            end -= 1;
        }
        let range = at..end;
        if end == body
            || !handle(&text[body..end])
            || links.iter().any(|link| link.range().start < range.end && range.start < link.range().end)
        {
            continue;
        }
        found.push(Autolink::Mention { handle: text[body..end].to_owned(), range });
    }
    found
}

pub fn autolinks(text: &str) -> Vec<Autolink> {
    let links = links(text);
    let tags = tags(text, &links);
    let mentions = mentions(text, &links);
    let mut all: Vec<Autolink> = links.into_iter().chain(tags).chain(mentions).collect();
    all.sort_by_key(|item| item.range().start);
    all
}
