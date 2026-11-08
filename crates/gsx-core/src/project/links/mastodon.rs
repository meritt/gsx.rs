use std::ops::Range;

use super::tables::{LATIN_FOLDED, PRECEDING};
use super::twitter::{
    Chains, MAX_LABEL_LENGTH, MAX_TCO_SLUG_LENGTH, MAX_URL_LENGTH, ascii_length, at, idna, is, literal, parens, tco,
};
use super::unicode::{mention, word};

const MAX_LABELS: usize = 127;
const MAX_MENTION_DOMAIN_LENGTH: usize = 253;
const PROTOCOLS: [&str; 9] =
    ["https://", "http://", "dat://", "dweb://", "ipfs://", "ipns://", "ssb://", "gopher://", "gemini://"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mention {
    pub range: Range<usize>,
    pub local: usize,
}

fn one_of(unit: u16, set: &str) -> bool {
    set.encode_utf16().any(|item| item == unit)
}

fn white(unit: u16) -> bool {
    matches!(unit, 0x09..=0x0D | 0x20 | 0x85 | 0xA0 | 0x1680 | 0x2000..=0x200A | 0x2028 | 0x2029 | 0x202F | 0x205F | 0x3000)
}

fn path_char(unit: u16) -> bool {
    !white(unit) && !one_of(unit, "<>()?")
}

fn path_end(unit: u16) -> bool {
    !white(unit) && !one_of(unit, "()?!*\"'「」<>;:=,.$%[]~&|")
}

fn bmp(unit: u16) -> bool {
    matches!(unit, 0xA0..=0xD7FF | 0xE000..=0xF8FF | 0xF900..=0xFDCF | 0xFDF0..=0xFFEF)
}

fn query_char(unit: u16) -> bool {
    u8::try_from(unit).is_ok_and(|byte| byte.is_ascii_alphanumeric())
        || one_of(unit, "!?*'();:&=+$/%#[]-_.,~|@^")
        || bmp(unit)
}

fn query_end(unit: u16) -> bool {
    u8::try_from(unit).is_ok_and(|byte| byte.is_ascii_alphanumeric()) || one_of(unit, "_&=#/-") || bmp(unit)
}

fn pair(text: &[u16], index: usize) -> bool {
    matches!(at(text, index), Some(0xD800..=0xDB3F | 0xDB44..=0xDBFF))
        && matches!(at(text, index + 1), Some(0xDC00..=0xDFFF))
}

fn run(text: &[u16], start: usize) -> usize {
    start + text[start..].iter().take_while(|&&unit| path_char(unit)).count()
}

fn group(text: &[u16], start: usize) -> Option<usize> {
    if !is(text, start, '(') {
        return None;
    }
    let first = run(text, start + 1);
    if first > start + 1 && is(text, first, ')') {
        return Some(first + 1);
    }
    if !is(text, first, '(') {
        return None;
    }
    let inner = run(text, first + 1);
    if inner == first + 1 || !is(text, inner, ')') {
        return None;
    }
    let last = run(text, inner + 1);
    is(text, last, ')').then_some(last + 1)
}

fn segment(text: &[u16], start: usize) -> Option<usize> {
    let mut runs = vec![(start, run(text, start))];
    let mut groups = vec![];
    let mut last = runs[0].1;
    while let Some(end) = group(text, last) {
        groups.push(last);
        last = run(text, end);
        runs.push((end, last));
    }
    for (index, &(from, to)) in runs.iter().enumerate().rev() {
        if let Some(found) = (from..to).rev().find(|&unit| path_end(text[unit])) {
            return Some(found + 1);
        }
        if index > 0
            && let Some(end) = parens(text, groups[index - 1])
        {
            return Some(end);
        }
    }
    None
}

fn path(text: &[u16], start: usize) -> usize {
    let mut end = start;
    while let Some(next) = segment(text, end) {
        end = next;
    }
    end
}

fn query(text: &[u16], start: usize) -> Option<usize> {
    let mut items = vec![];
    let mut index = start;
    loop {
        if at(text, index).is_some_and(query_char) {
            items.push(index);
            index += 1;
        } else if pair(text, index) {
            items.push(index);
            index += 2;
        } else {
            break;
        }
    }
    items
        .into_iter()
        .rev()
        .find_map(|item| if query_end(text[item]) { Some(item + 1) } else { pair(text, item).then_some(item + 2) })
}

fn overlong(text: &[u16], domain: Range<usize>) -> Option<usize> {
    let mut start = domain.start;
    let mut labels = 0;
    for index in domain.clone() {
        if text[index] != u16::from(b'.') {
            continue;
        }
        if index - start > MAX_LABEL_LENGTH {
            return Some(start.saturating_sub(1).max(domain.start));
        }
        labels += 1;
        if labels == MAX_LABELS {
            return Some(index);
        }
        start = index + 1;
    }
    None
}

struct Found {
    url: Range<usize>,
    protocol: bool,
    domain: Range<usize>,
}

fn rest(chains: &mut Chains, start: usize, after: usize) -> Option<Found> {
    let text = chains.text;
    let mut end = chains.domain(after)?;
    if let Some(cut) = overlong(text, after..end) {
        end = after + Chains::new(&text[after..cut], false).domain(0)?;
    }
    let domain = after..end;
    let mut index = end;
    if is(text, index, ':') {
        let digits = text[index + 1..].iter().take_while(|&&unit| (0x30..=0x39).contains(&unit)).count();
        if digits > 0 {
            index += 1 + digits;
        }
    }
    if is(text, index, '/') {
        index = path(text, index + 1);
    }
    if is(text, index, '?')
        && let Some(end) = query(text, index + 1)
    {
        index = end;
    }
    Some(Found { url: start..index, protocol: after > start, domain })
}

fn url(chains: &mut Chains, start: usize) -> Option<Found> {
    PROTOCOLS
        .iter()
        .find_map(|protocol| literal(chains.text, start, protocol))
        .and_then(|after| rest(chains, start, after))
        .or_else(|| rest(chains, start, start))
}

fn next(chains: &mut Chains, from: usize) -> Option<Found> {
    let text = chains.text;
    for start in from..=text.len() {
        if at(text, start).is_some_and(|unit| PRECEDING.contains(unit))
            && let Some(found) = url(chains, start + 1)
        {
            return Some(found);
        }
        if start == 0
            && let Some(found) = url(chains, 0)
        {
            return Some(found);
        }
    }
    None
}

fn growth(domain: &[u16]) -> usize {
    let labels = domain.split(|&unit| unit == u16::from(b'.'));
    let length = labels.fold(0usize, |sum, label| sum.saturating_add(ascii_length(label) + 1)).saturating_sub(1);
    length.saturating_sub(domain.len())
}

pub fn urls(text: &[u16]) -> Vec<Range<usize>> {
    if !text.contains(&u16::from(b':')) {
        return vec![];
    }
    let mut chains = Chains::new(text, false);
    let mut urls = vec![];
    let mut from = 0;
    while let Some(found) = next(&mut chains, from) {
        from = found.url.end;
        if !found.protocol {
            continue;
        }
        let end = match tco(&text[..found.url.end], found.url.start) {
            Some((slug, _)) if slug > MAX_TCO_SLUG_LENGTH => continue,
            Some((_, end)) => end,
            None => found.url.end,
        };
        let domain = &text[found.domain.clone()];
        let points = char::decode_utf16(text[found.url.start..end].iter().copied()).count();
        if idna(domain) && points.saturating_add(growth(domain)) <= MAX_URL_LENGTH {
            urls.push(found.url.start..end);
        }
    }
    urls
}

fn points(text: &[u16]) -> Vec<(usize, u32)> {
    let mut index = 0;
    char::decode_utf16(text.iter().copied())
        .map(|item| {
            let point = item.map_or_else(|error| u32::from(error.unpaired_surrogate()), u32::from);
            let at = index;
            index += if point > 0xFFFF { 2 } else { 1 };
            (at, point)
        })
        .collect()
}

fn chain(points: &[(usize, u32)], start: usize, inner: impl Fn(u32) -> bool) -> usize {
    let word = |from: usize| from + points[from..].iter().take_while(|&&(_, point)| inner(point)).count();
    let mut end = word(start);
    if end == start {
        return start;
    }
    loop {
        let joined = end + points[end..].iter().take_while(|&&(_, point)| point == 0x2E || point == 0x2D).count();
        let next = word(joined);
        if joined == end || next == joined {
            return end;
        }
        end = next;
    }
}

pub fn mentions(text: &[u16]) -> Vec<Mention> {
    if !text.iter().any(|&unit| unit == 0x40 || unit == 0xFF20) {
        return vec![];
    }
    let points = points(text);
    let offset = |index: usize| points.get(index).map_or(text.len(), |&(at, _)| at);
    let blocked = |point: u32| point == 0x3D || point == 0x2F || word(point);
    let mut found = vec![];
    let mut index = 0;
    while index < points.len() {
        let (start, point) = points[index];
        if point != 0x40 || index > 0 && blocked(points[index - 1].1) {
            index += 1;
            continue;
        }
        let local = chain(&points, index + 1, mention);
        if local == index + 1 {
            index += 1;
            continue;
        }
        let mut end = local;
        let mut domain = None;
        if points.get(local).is_some_and(|&(_, point)| point == 0x40) {
            let last = chain(&points, local + 1, word);
            if last > local + 1 {
                domain = Some(last - local - 1);
                end = last;
            }
        }
        let after = offset(end);
        let refused = at(text, after).is_some_and(|unit| unit == 0x40 || unit == 0xFF20 || LATIN_FOLDED.contains(unit))
            || text[after..].starts_with(&[0x3A, 0x2F, 0x2F]);
        if !refused && domain.is_none_or(|length| length <= MAX_MENTION_DOMAIN_LENGTH) {
            found.push(Mention { range: start..after, local: offset(local) });
        }
        index = end;
    }
    found
}
