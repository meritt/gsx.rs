use std::collections::HashSet;
use std::ops::Range as Span;

use crate::document::Range;

use super::change::Feature;
use super::layout::{Styled, Written, blank};
use super::links::bluesky::{self, Autolink};
use super::table::{Detection, EntityForm, Offsets, Style};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entity {
    pub style: Style,
    pub range: Range,
    pub value: Option<String>,
    pub format: Option<&'static str>,
}

pub struct Finished {
    pub styled: Vec<Styled>,
    pub capped: Vec<(Feature, Range)>,
}

fn pieces(styled: &Styled, prefixes: &[Span<usize>], suffix: Option<usize>, out: &mut Vec<Styled>) {
    let span = &styled.span;
    let end = suffix.map_or(span.end, |suffix| span.end.min(suffix)).max(span.start);
    let mut start = span.start;
    let first = prefixes.partition_point(|prefix| prefix.end <= start);
    for prefix in prefixes[first..].iter().take_while(|prefix| prefix.start < end) {
        if prefix.start > start {
            out.push(Styled { span: start..prefix.start, ..styled.clone() });
        }
        start = start.max(prefix.end);
    }
    if start < end {
        out.push(Styled { span: start..end, ..styled.clone() });
    }
}

fn merged(spans: &[Span<usize>]) -> Vec<Span<usize>> {
    let mut sorted = spans.to_vec();
    crate::order::sort_by_key(&mut sorted, |span| span.start as u64);
    let mut out: Vec<Span<usize>> = vec![];
    for span in sorted {
        match out.last_mut() {
            Some(last) if span.start <= last.end => last.end = last.end.max(span.end),
            _ => out.push(span),
        }
    }
    out
}

fn detected(written: &Written, out: &mut Vec<Styled>) {
    let mut masked = written.text.clone();
    if let Some(suffix) = &written.suffix {
        masked.replace_range(suffix.clone(), &" ".repeat(suffix.len()));
    }
    let eligible = merged(&written.eligible);
    for autolink in bluesky::autolinks(&masked) {
        let range = autolink.range().clone();
        let at = eligible.partition_point(|span| span.end < range.end);
        if eligible.get(at).is_none_or(|span| span.start > range.start) {
            continue;
        }
        let (style, value) = match autolink {
            Autolink::Link { uri, .. } => (Style::Link, uri),
            Autolink::Tag { tag, .. } => (Style::Tag, tag),
            Autolink::Mention { handle, .. } => (Style::Mention, handle),
        };
        out.push(Styled { style, value: Some(value), format: None, span: range, source: None, decoration: false });
    }
}

fn trim(text: &mut String, styled: &mut Vec<Styled>, origins: &mut Vec<(Span<usize>, Range)>) {
    let lead = text.len() - text.trim_start_matches(blank).len();
    let end = text.trim_end_matches(blank).len().max(lead);
    text.truncate(end);
    text.replace_range(..lead, "");
    let kept = lead..end;
    styled.retain_mut(|entity| super::layout::within(&mut entity.span, &kept));
    origins.retain_mut(|(span, _)| super::layout::within(span, &kept));
}

fn code(style: Style) -> bool {
    matches!(style, Style::Code | Style::Pre)
}

fn tighten(text: &str, styled: &mut Vec<Styled>) {
    styled.retain_mut(|entity| {
        let Span { start, end } = entity.span;
        let end = start + text[start..end].trim_end_matches(blank).len();
        let start = match entity.style {
            Style::Blockquote => end - text[start..end].trim_start_matches(blank).len(),
            _ => start,
        };
        entity.span = start..end;
        start < end
    });
}

pub fn finish(form: &EntityForm, written: &mut Written) -> Finished {
    let suffix = written.suffix.as_ref().filter(|_| !form.styled_suffix).map(|suffix| suffix.start);
    let spans = |style: fn(Style) -> bool| {
        written.styled.iter().filter(move |entity| style(entity.style)).map(|entity| entity.span.clone())
    };
    let mut breaks = written.prefixes.clone();
    if form.code_splits {
        breaks.extend(spans(code));
        crate::order::sort_by_key(&mut breaks, |span| span.start as u64);
    }
    let pre: Vec<Span<usize>> = spans(|style| style == Style::Pre).collect();
    let mut styled = vec![];
    for entity in &written.styled {
        let breaks = match entity.style {
            Style::Code | Style::Pre => &written.prefixes,
            Style::Blockquote => &pre,
            _ => &breaks,
        };
        pieces(entity, breaks, suffix, &mut styled);
    }
    match form.detection {
        Detection::None => {}
        Detection::Bluesky => detected(written, &mut styled),
    }
    if form.trim {
        trim(&mut written.text, &mut styled, &mut written.origins);
        tighten(&written.text, &mut styled);
    }
    crate::order::sort_by_key(&mut styled, |entity| {
        crate::order::pair(entity.span.start, u32::MAX as usize - entity.span.end)
    });
    let mut capped = vec![];
    if let Some(cap) = form.cap {
        let decorations = styled.iter().filter(|entity| entity.decoration).count();
        let mut kept = decorations - styled.len().saturating_sub(cap).min(decorations);
        styled.retain(|entity| {
            let keep = !entity.decoration || kept > 0;
            kept -= usize::from(entity.decoration && keep);
            keep
        });
    }
    if let Some(cap) = form.cap
        && styled.len() > cap
    {
        let mut seen = HashSet::new();
        for entity in styled.drain(cap..) {
            if let Some(source) = entity.source
                && seen.insert(source)
            {
                capped.push(source);
            }
        }
    }
    Finished { styled, capped }
}

pub fn offsets(form: &EntityForm, text: &str, styled: &[Styled]) -> Vec<Entity> {
    let mut positions: Vec<usize> = styled.iter().flat_map(|entity| [entity.span.start, entity.span.end]).collect();
    crate::order::sort_by_key(&mut positions, |&position| position as u64);
    positions.dedup();
    let (mut byte, mut unit) = (0, 0usize);
    let units: Vec<usize> = positions
        .iter()
        .map(|&to| {
            unit += match form.offsets {
                Offsets::Utf8 => to - byte,
                Offsets::Utf16 => text[byte..to].encode_utf16().count(),
            };
            byte = to;
            unit
        })
        .collect();
    let offset = |at: usize| {
        let index = positions.binary_search(&at).expect("every boundary is mapped");
        u32::try_from(units[index]).unwrap_or(u32::MAX)
    };
    styled
        .iter()
        .map(|entity| Entity {
            style: entity.style,
            range: Range { start: offset(entity.span.start), end: offset(entity.span.end) },
            value: entity.value.clone(),
            format: entity.format,
        })
        .collect()
}
