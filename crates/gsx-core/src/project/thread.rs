use std::ops::Range as Span;

use crate::measure::clusters;

use super::change::Feature;
use super::layout::Styled;
use super::table::When;
use super::{Boundary, Fitted, Rendered, Request, appended, within};

pub(super) struct Thread {
    pub parts: Vec<Fitted>,
    pub spans: Vec<Span<usize>>,
    pub boundaries: Vec<Option<Boundary>>,
}

const COARSER: [Boundary; 2] = [Boundary::Paragraph, Boundary::Sentence];

pub(super) fn split(request: &Request, full: &Rendered, suffix: Option<&str>) -> Option<Thread> {
    let limits = request.limits();
    let decorate = |rendered: Rendered, first: bool| {
        if first { appended(request.target, rendered, suffix) } else { rendered }
    };
    let probe = |span: Span<usize>, first: bool| {
        let rendered = decorate(Rendered::text(full.text[span].to_owned()), first);
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
    let mut boundaries = vec![];
    let mut start = 0;
    while start < length {
        let first = spans.is_empty();
        let fits = |end: usize| probe(start..end, first);
        let words = &classes[2][classes[2].partition_point(|&(end, _)| end <= start)..];
        let coarser = |(furthest, _): (usize, usize)| {
            classes[..2].iter().zip(COARSER).find_map(|(class, boundary)| {
                let from = class.partition_point(|&(end, _)| end <= start);
                let upto = class.partition_point(|&(end, _)| end <= furthest);
                (upto > from).then(|| (class[upto - 1], boundary))
            })
        };
        let ((end, next), boundary) = match last_fitting(words.len(), |index| fits(words[index].0)) {
            Some(last) if last + 1 == words.len() && fits(length) => ((length, length), None),
            Some(last) => {
                let (found, boundary) = coarser(words[last]).unwrap_or((words[last], Boundary::Word));
                (found, Some(boundary))
            }
            None => (grapheme(&full.text, start, fits)?, Some(Boundary::Grapheme)),
        };
        spans.push(start..end);
        boundaries.push(if next < length { boundary } else { None });
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
    Some(Thread { parts, spans, boundaries })
}

fn last_fitting(count: usize, fits: impl Fn(usize) -> bool) -> Option<usize> {
    if count == 0 || !fits(0) {
        return None;
    }
    let (mut low, mut step) = (0, 1);
    while low + step < count && fits(low + step) {
        low += step;
        step *= 2;
    }
    let high = (low + step).min(count);
    let (mut fitting, mut failing) = (low, high);
    while failing - fitting > 1 {
        let middle = fitting + (failing - fitting) / 2;
        if fits(middle) { fitting = middle } else { failing = middle }
    }
    Some(fitting)
}

fn grapheme(text: &str, start: usize, fits: impl Fn(usize) -> bool) -> Option<(usize, usize)> {
    let mut ends = vec![];
    let mut at = start;
    let mut clusters = clusters(&text[start..]);
    let mut reached = |index: usize, ends: &mut Vec<usize>| {
        while ends.len() <= index {
            let Some(cluster) = clusters.next() else { return false };
            at += cluster.len();
            ends.push(at);
        }
        true
    };
    let mut frontier = 0;
    while reached(frontier, &mut ends) && fits(ends[frontier]) {
        frontier = frontier * 2 + 1;
    }
    let count = ends.len().min(frontier + 1);
    let last = last_fitting(count, |index| fits(ends[index]))?;
    Some((ends[last], ends[last]))
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
