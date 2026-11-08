use std::ops::Range as Span;

use crate::measure::clusters;

use super::change::Feature;
use super::layout::Styled;
use super::table::When;
use super::{Fitted, Rendered, Request, appended, within};

pub(super) struct Thread {
    pub parts: Vec<Fitted>,
    pub spans: Vec<Span<usize>>,
}

pub(super) fn split(request: &Request, full: &Rendered, suffix: Option<&str>) -> Option<Thread> {
    let limits = request.limits();
    let decorate = |rendered: Rendered, first: bool| {
        if first { appended(request.target, rendered, suffix) } else { rendered }
    };
    let probe = |span: Span<usize>, first: bool| {
        let rendered = decorate(part(full, span), first);
        let values = request.values(&limits, &rendered);
        within(&limits, &values)
    };
    let length = full.text.len();
    if probe(0..length, true) {
        return None;
    }
    let mut guarded = full.links.clone();
    guarded.extend(full.styled.iter().filter(|styled| styled.value.is_some()).map(|styled| styled.span.clone()));
    let classes = breaks(&full.text, &guarded);
    let mut spans = vec![];
    let mut start = 0;
    while start < length {
        let first = spans.is_empty();
        if probe(start..length, first) {
            spans.push(start..length);
            break;
        }
        let fits = |end: usize| probe(start..end, first);
        let cut = classes.iter().find_map(|class| {
            let candidates = &class[class.partition_point(|&(end, _)| end <= start)..];
            let count = candidates.partition_point(|&(end, _)| fits(end));
            count.checked_sub(1).map(|last| candidates[last])
        });
        let (end, next) = cut.or_else(|| grapheme(&full.text, start, fits))?;
        spans.push(start..end);
        start = next;
    }
    if spans.is_empty() {
        return None;
    }
    let mut parts: Vec<Fitted> = spans
        .iter()
        .enumerate()
        .map(|(index, span)| {
            let rendered = decorate(part(full, span.clone()), index == 0);
            let values = request.values(&limits, &rendered);
            Fitted { measurements: request.measurements(&limits, &values), rendered }
        })
        .collect();
    let first = &mut parts[0].rendered;
    first.changes.clone_from(&full.changes);
    first.alts.clone_from(&full.alts);
    first.changes.push(request.change(Feature::Text, None, Some(When::Thread)));
    Some(Thread { parts, spans })
}

fn grapheme(text: &str, start: usize, fits: impl Fn(usize) -> bool) -> Option<(usize, usize)> {
    let mut ends = vec![];
    let mut at = start;
    for cluster in clusters(&text[start..]) {
        at += cluster.len();
        ends.push(at);
    }
    let count = ends.partition_point(|&end| fits(end));
    count.checked_sub(1).map(|last| (ends[last], ends[last]))
}

fn part(full: &Rendered, span: Span<usize>) -> Rendered {
    let shift = |offset: usize| offset - span.start;
    let links = full
        .links
        .iter()
        .filter(|link| span.start <= link.start && link.end <= span.end)
        .map(|link| shift(link.start)..shift(link.end))
        .collect();
    let styled = full
        .styled
        .iter()
        .filter_map(|styled| {
            let (start, end) = (styled.span.start.max(span.start), styled.span.end.min(span.end));
            (start < end).then(|| Styled { span: shift(start)..shift(end), ..styled.clone() })
        })
        .collect();
    Rendered { links, styled, ..Rendered::text(full.text[span].to_owned()) }
}

fn breaks(text: &str, guarded: &[Span<usize>]) -> [Vec<(usize, usize)>; 3] {
    let mut classes: [Vec<(usize, usize)>; 3] = [vec![], vec![], vec![]];
    let mut at = 0;
    while let Some(char) = text[at..].chars().next() {
        if !char.is_whitespace() {
            at += char.len_utf8();
            continue;
        }
        let end = text[at..].find(|char: char| !char.is_whitespace()).map_or(text.len(), |offset| at + offset);
        if at > 0 && !guarded.iter().any(|span| span.start < at && at < span.end) {
            let closing = |char: &char| matches!(char, '"' | '\'' | ')' | '»' | '”' | '’');
            let sentence = text[..at].chars().rev().find(|char| !closing(char));
            if text[at..end].matches('\n').count() >= 2 {
                classes[0].push((at, end));
            }
            if sentence.is_some_and(|char| matches!(char, '.' | '!' | '?' | '…')) {
                classes[1].push((at, end));
            }
            classes[2].push((at, end));
        }
        at = end;
    }
    classes
}
