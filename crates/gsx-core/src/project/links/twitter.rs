use std::collections::HashMap;
use std::ops::Range;
use std::sync::OnceLock;

use super::tables::{ASCII_LABEL, DOMAIN, PATH, PATH_END, PRECEDING, QUERY, QUERY_END, TLDS};

pub(super) const MAX_URL_LENGTH: usize = 4096;
pub(super) const MAX_TCO_SLUG_LENGTH: usize = 40;
pub(super) const MAX_LABEL_LENGTH: usize = 63;
const DEFAULT_PROTOCOL: usize = "https://".len();
const MAX_INT: u64 = 2_147_483_647;

pub(super) fn member(set: &[(u16, u16)], unit: u16) -> bool {
    set.binary_search_by(|&(from, to)| {
        if to < unit {
            std::cmp::Ordering::Less
        } else if from > unit {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    })
    .is_ok()
}

fn canonical(unit: u16) -> u16 {
    if let Ok(byte) = u8::try_from(unit)
        && byte.is_ascii()
    {
        return u16::from(byte.to_ascii_uppercase());
    }
    let Some(char) = char::from_u32(u32::from(unit)) else {
        return unit;
    };
    let mut upper = char.to_uppercase();
    match (upper.next(), upper.next()) {
        (Some(single), None) => {
            u16::try_from(u32::from(single)).ok().filter(|&mapped| unit < 128 || mapped >= 128).unwrap_or(unit)
        }
        _ => unit,
    }
}

pub(super) fn at(text: &[u16], index: usize) -> Option<u16> {
    text.get(index).copied()
}

pub(super) fn is(text: &[u16], index: usize, char: char) -> bool {
    at(text, index) == Some(char as u16)
}

pub(super) fn literal(text: &[u16], start: usize, pattern: &str) -> Option<usize> {
    let mut index = start;
    for unit in pattern.encode_utf16() {
        if canonical(at(text, index)?) != canonical(unit) {
            return None;
        }
        index += 1;
    }
    Some(index)
}

fn scan(text: &[u16], start: usize, set: &[(u16, u16)]) -> usize {
    start + text[start..].iter().take_while(|&&unit| member(set, unit)).count()
}

fn tlds() -> &'static HashMap<u16, Vec<&'static str>> {
    static INDEX: OnceLock<HashMap<u16, Vec<&'static str>>> = OnceLock::new();
    INDEX.get_or_init(|| {
        let mut index: HashMap<u16, Vec<&'static str>> = HashMap::new();
        for tld in TLDS.split('|') {
            index.entry(canonical(tld.encode_utf16().next().unwrap())).or_default().push(tld);
        }
        index
    })
}

fn top_level(text: &[u16], start: usize) -> Option<usize> {
    let separated = |end: usize| {
        at(text, end).is_none_or(|unit| {
            u8::try_from(unit).map_or(true, |byte| !byte.is_ascii_alphanumeric() && !b"@+-".contains(&byte))
        })
    };
    for tld in tlds().get(&canonical(at(text, start)?)).into_iter().flatten() {
        if let Some(end) = literal(text, start, tld)
            && separated(end)
        {
            return Some(end);
        }
    }
    let prefix = literal(text, start, "xn--")?;
    let end = prefix
        + text[prefix..]
            .iter()
            .take_while(|&&unit| unit < 128 && ((unit as u8).is_ascii_alphanumeric() || unit == u16::from(b'-')))
            .count();
    (end > prefix).then_some(end)
}

pub(super) struct Chains<'t> {
    pub(super) text: &'t [u16],
    ascii: bool,
    runs: Vec<usize>,
    underscores: Vec<usize>,
    best: Vec<Option<Option<usize>>>,
    tlds: Vec<Option<Option<usize>>>,
}

impl<'t> Chains<'t> {
    pub(super) fn new(text: &'t [u16], ascii: bool) -> Self {
        let length = text.len();
        let (mut runs, mut underscores) = (vec![length; length + 1], vec![length; length + 1]);
        for index in (0..length).rev() {
            let unit = text[index];
            let inner = if ascii {
                member(ASCII_LABEL, unit)
            } else {
                member(DOMAIN, unit) || unit == u16::from(b'_') || unit == u16::from(b'-')
            };
            runs[index] = if inner { runs[index + 1] } else { index };
            underscores[index] = if unit == u16::from(b'_') { index } else { underscores[index + 1] };
        }
        Chains { text, ascii, runs, underscores, best: vec![None; length + 2], tlds: vec![None; length + 2] }
    }

    fn label(&self, start: usize) -> Option<Range<usize>> {
        let end = *self.runs.get(start)?;
        (end > start && is(self.text, end, '.')).then_some(start..end)
    }

    fn tld(&mut self, start: usize) -> Option<usize> {
        *self.tlds[start].get_or_insert_with(|| top_level(self.text, start))
    }

    fn edges(&self, label: &Range<usize>) -> bool {
        self.ascii || member(DOMAIN, self.text[label.start]) && member(DOMAIN, self.text[label.end - 1])
    }

    fn named(&mut self, label: &Range<usize>) -> Option<usize> {
        if self.edges(label) && (self.ascii || self.underscores[label.start] >= label.end) {
            self.tld(label.end + 1)
        } else {
            None
        }
    }

    pub(super) fn domain(&mut self, start: usize) -> Option<usize> {
        let mut pending = vec![];
        let mut index = start;
        let mut after = loop {
            if let Some(known) = self.best[index] {
                break known;
            }
            match self.label(index) {
                Some(label) => {
                    pending.push(label.clone());
                    index = label.end + 1;
                }
                None => break None,
            }
        };
        while let Some(label) = pending.pop() {
            if !(self.edges(&label) && after.is_some()) {
                after = self.named(&label);
            }
            self.best[label.start] = Some(after);
        }
        self.label(start).and(after)
    }
}

fn ascii_domains(text: &[u16]) -> Vec<Range<usize>> {
    let mut chains = Chains::new(text, true);
    let mut found = vec![];
    let mut index = 0;
    while index < text.len() {
        match chains.domain(index) {
            Some(end) => {
                found.push(index..end);
                index = end;
            }
            None => index += 1,
        }
    }
    found
}

pub(super) fn parens(text: &[u16], start: usize) -> Option<usize> {
    if !is(text, start, '(') {
        return None;
    }
    let first = scan(text, start + 1, PATH);
    if first > start + 1 && is(text, first, ')') {
        return Some(first + 1);
    }
    if !is(text, first, '(') {
        return None;
    }
    let inner = scan(text, first + 1, PATH);
    if inner == first + 1 || !is(text, inner, ')') {
        return None;
    }
    let last = scan(text, inner + 1, PATH);
    is(text, last, ')').then_some(last + 1)
}

fn path(text: &[u16], start: usize) -> usize {
    let (mut index, mut end) = (start, start);
    loop {
        if let Some(unit) = at(text, index)
            && member(PATH, unit)
        {
            index += 1;
            if member(PATH_END, unit) {
                end = index;
            }
        } else if let Some(close) = parens(text, index) {
            index = close;
        } else {
            break;
        }
    }
    while let Some(close) = parens(text, end) {
        end = close;
    }
    end
}

fn query(text: &[u16], start: usize) -> Option<usize> {
    let run = scan(text, start, QUERY);
    (start..run).rev().find(|&index| member(QUERY_END, text[index])).map(|index| index + 1)
}

struct Found {
    url: Range<usize>,
    protocol: bool,
    domain: Range<usize>,
    path: bool,
}

fn rest(chains: &mut Chains, start: usize, after: usize) -> Option<Found> {
    let text = chains.text;
    let end = chains.domain(after)?;
    let domain = after..end;
    let mut index = end;
    if is(text, index, ':') {
        let digits = text[index + 1..].iter().take_while(|&&unit| (0x30..=0x39).contains(&unit)).count();
        if digits > 0 {
            index += 1 + digits;
        }
    }
    let path = is(text, index, '/');
    if path {
        index = self::path(text, index + 1);
    }
    if is(text, index, '?')
        && let Some(end) = query(text, index + 1)
    {
        index = end;
    }
    Some(Found { url: start..index, protocol: after > start, domain, path })
}

fn url(chains: &mut Chains, start: usize) -> Option<Found> {
    let protocol = literal(chains.text, start, "https://").or_else(|| literal(chains.text, start, "http://"));
    protocol.and_then(|after| rest(chains, start, after)).or_else(|| rest(chains, start, start))
}

fn next(chains: &mut Chains, from: usize) -> Option<(Option<u16>, Found)> {
    let text = chains.text;
    for start in from..=text.len() {
        if let Some(unit) = at(text, start)
            && member(PRECEDING, unit)
            && let Some(found) = url(chains, start + 1)
        {
            return Some((Some(unit), found));
        }
        if start == 0
            && let Some(found) = url(chains, 0)
        {
            return Some((None, found));
        }
    }
    None
}

fn encoded(label: &[u16]) -> usize {
    let points: Vec<u32> = char::decode_utf16(label.iter().copied())
        .map(|item| item.map_or_else(|error| u32::from(error.unpaired_surrogate()), u32::from))
        .collect();
    if points.iter().all(|&point| (0x20..0x7F).contains(&point)) {
        return points.len();
    }
    let basic = points.iter().filter(|&&point| point < 0x80).count();
    let mut length = 4 + basic + usize::from(basic > 0);
    let (mut n, mut delta, mut bias, mut handled) = (0x80u32, 0u64, 72u64, basic);
    while handled < points.len() {
        let next = points.iter().copied().filter(|&point| point >= n).min().unwrap_or(n);
        let plus_one = handled as u64 + 1;
        delta += u64::from(next - n) * plus_one;
        n = next;
        for &point in &points {
            if point < n {
                delta += 1;
            }
            if delta > MAX_INT {
                return usize::MAX;
            }
            if point == n {
                let mut q = delta;
                let mut k = 36;
                loop {
                    let threshold = if k <= bias {
                        1
                    } else if k >= bias + 26 {
                        26
                    } else {
                        k - bias
                    };
                    if q < threshold {
                        break;
                    }
                    length += 1;
                    q = (q - threshold) / (36 - threshold);
                    k += 36;
                }
                length += 1;
                bias = adapt(delta, plus_one, handled == basic);
                delta = 0;
                handled += 1;
            }
        }
        delta += 1;
        n += 1;
    }
    length
}

fn adapt(delta: u64, points: u64, first: bool) -> u64 {
    let mut delta = if first { delta / 700 } else { delta / 2 };
    delta += delta / points;
    let mut k = 0;
    while delta > 35 * 26 / 2 {
        delta /= 35;
        k += 36;
    }
    k + 36 * delta / (delta + 38)
}

fn separator(unit: u16) -> bool {
    matches!(unit, 0x2E | 0x3002 | 0xFF0E | 0xFF61)
}

pub(super) fn ascii_length(label: &[u16]) -> usize {
    let pieces: Vec<&[u16]> = label.split(|&unit| separator(unit)).collect();
    pieces.iter().fold(pieces.len() - 1, |sum, piece| sum.saturating_add(encoded(piece)))
}

pub(super) fn idna(domain: &[u16]) -> bool {
    if domain.starts_with(&[0x78, 0x6E, 0x2D, 0x2D]) && ascii_domains(domain).is_empty() {
        return false;
    }
    domain.split(|&unit| unit == u16::from(b'.')).all(|label| (1..=MAX_LABEL_LENGTH).contains(&ascii_length(label)))
}

fn valid(text: &[u16], found: &Found) -> bool {
    let protocol = if found.protocol { found.domain.start - found.url.start } else { DEFAULT_PROTOCOL };
    idna(&text[found.domain.clone()]) && protocol + found.url.len() <= MAX_URL_LENGTH
}

pub(super) fn tco(text: &[u16], start: usize) -> Option<(usize, usize)> {
    let after = literal(text, start, "https://t.co/").or_else(|| literal(text, start, "http://t.co/"))?;
    let slug =
        after + text[after..].iter().take_while(|&&unit| unit < 128 && (unit as u8).is_ascii_alphanumeric()).count();
    if slug == after {
        return None;
    }
    let end = if is(text, slug, '?') { query(text, slug + 1).unwrap_or(slug) } else { slug };
    Some((slug - after, end))
}

fn find(haystack: &[u16], needle: &[u16], from: usize) -> Option<usize> {
    (from..=haystack.len().saturating_sub(needle.len())).find(|&index| haystack[index..].starts_with(needle))
}

pub fn urls(text: &[u16]) -> Vec<Range<usize>> {
    if !text.contains(&u16::from(b'.')) {
        return vec![];
    }
    let mut chains = Chains::new(text, false);
    let mut urls = vec![];
    let mut from = 0;
    while let Some((before, found)) = next(&mut chains, from) {
        from = found.url.end;
        if !valid(text, &found) {
            continue;
        }
        if found.protocol {
            match tco(text, found.url.start) {
                Some((slug, _)) if slug > MAX_TCO_SLUG_LENGTH => {}
                Some((_, end)) => urls.push(found.url.start..end),
                None => urls.push(found.url.clone()),
            }
            continue;
        }
        if before.is_some_and(|unit| b"-_./".iter().any(|&byte| unit == u16::from(byte))) {
            continue;
        }
        let domain = &text[found.domain.clone()];
        let mut last = None;
        let mut end = 0;
        for ascii in ascii_domains(domain) {
            let piece = &domain[ascii.clone()];
            let start = find(domain, piece, end).unwrap_or(ascii.start);
            end = start + piece.len();
            urls.push(found.domain.start + start..found.domain.start + end);
            last = Some(urls.len() - 1);
        }
        if let Some(last) = last
            && found.path
        {
            urls[last].end = found.url.end;
        }
    }
    urls
}
