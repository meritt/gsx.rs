use std::cell::{Cell, OnceCell, RefCell};
use std::ops::Range as Span;

use crate::measure::{Unit, clusters};

use super::change::Change;
use super::layout::Styled;
use super::outline::{Cut, Outline};
use super::{Bound, Boundary, Fitted, Rendered, Request, appended, links};

pub(super) struct Thread {
    pub parts: Vec<Fitted>,
    pub spans: Vec<Span<usize>>,
    pub boundaries: Vec<Option<Boundary>>,
    pub truncated: Vec<(usize, usize, Change)>,
}

const COARSER: [Boundary; 2] = [Boundary::Paragraph, Boundary::Sentence];

pub(super) const fn closing(char: char) -> bool {
    matches!(char, '"' | '\'' | ')' | '»' | '”' | '’')
}

pub(super) const fn terminal(char: char) -> bool {
    matches!(char, '.' | '!' | '?' | '…')
}

pub(super) fn split(request: &Request, full: &Rendered, suffix: Option<&str>) -> Option<Thread> {
    let limits = part_limits(request, full, suffix);
    let decorate = |rendered: Rendered, first: bool| {
        if first { appended(request.target, rendered, suffix) } else { rendered }
    };
    let probe = |span: Span<usize>, first: bool| match suffix.filter(|_| first) {
        Some(_) => request.admits(&limits, &decorate(Rendered::text(full.text[span].to_owned()), true)),
        None => request.admits_text(&limits, &full.text[span]),
    };
    let length = full.text.len();
    if probe(0..length, true) {
        return None;
    }
    let maxima = limits.iter().filter(|limit| !limit.unit.counts_markers()).filter_map(|limit| limit.cap());
    let reach = maxima.clone().max().map(|maximum| maximum.saturating_add(1).saturating_mul(4));
    let near = maxima.min().map(|maximum| maximum.saturating_add(1).saturating_mul(4));
    let reached = Cell::new(None);
    let measured = |span: Span<usize>, first: bool| {
        let prefix = reach.map(|reach| full.text.floor_char_boundary(span.start.saturating_add(reach)));
        match prefix {
            Some(end) if end < span.end => {
                let fits = match reached.get() {
                    Some((start, fits)) if start == span.start => fits,
                    _ => {
                        let fits = probe(span.start..end, first);
                        reached.set(Some((span.start, fits)));
                        fits
                    }
                };
                fits && probe(span, first)
            }
            _ => probe(span, first),
        }
    };
    let mut guarded = full.links.clone();
    guarded.extend(full.styled.iter().filter(|styled| styled.value.is_some()).map(|styled| styled.span.clone()));
    let guarded = merged(guarded);
    let classes = breaks(&full.text, &guarded);
    let atoms = OnceCell::new();
    let atoms = || {
        atoms.get_or_init(|| {
            let counts = |units: &[Unit]| limits.iter().any(|limit| units.contains(&limit.unit));
            let x = counts(&[Unit::XWeighted, Unit::Links]);
            let urls = links::bare_urls(&full.text, x, counts(&[Unit::MastodonWeighted]));
            if urls.is_empty() { guarded.clone() } else { merged(guarded.iter().cloned().chain(urls).collect()) }
        })
    };
    let cache = RefCell::new(None::<Window>);
    let probe = |span: Span<usize>, first: bool| {
        let Some(near) = near.filter(|_| !first || suffix.is_none()) else { return measured(span, first) };
        let window = match cache.take().filter(|window| window.start == span.start) {
            Some(window) if window.covers(span.end) => window,
            previous => {
                let width = previous.map_or(near.min(FIRST_WINDOW), |window| 2 * (window.far - window.start));
                let width = width.max(span.end - span.start).min(near);
                Window::new(request, &limits, (&full.text, &classes[2]), span.start, (width, near))
            }
        };
        let admitted = window.admitted(span.end);
        cache.replace(Some(window));
        admitted.unwrap_or_else(|| measured(span, first))
    };
    let mut spans = vec![];
    let mut boundaries = vec![];
    let mut oversized = vec![];
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
            None => {
                let (end, next) = grapheme(&full.text, start, fits)?;
                let atoms = atoms();
                let inside = atoms.get(atoms.partition_point(|span| span.end <= end)).filter(|span| span.start < end);
                match inside {
                    Some(span) if span.start > start && fits(span.start) => {
                        ((span.start, span.start), Some(Boundary::Grapheme))
                    }
                    Some(span) if span.start > start => {
                        (grapheme(&full.text[..span.start], start, fits)?, Some(Boundary::Grapheme))
                    }
                    Some(span) => {
                        oversized.push(spans.len());
                        let next = skipped(&full.text, span.end);
                        let boundary = if next > span.end {
                            coarser((span.end, next)).map_or(Boundary::Word, |(_, boundary)| boundary)
                        } else {
                            Boundary::Grapheme
                        };
                        ((span.end, next), Some(boundary))
                    }
                    None => ((end, next), Some(Boundary::Grapheme)),
                }
            }
        };
        spans.push(start..end);
        boundaries.push(if next < length { boundary } else { None });
        start = next;
    }
    if spans.len() < 2 {
        return None;
    }
    let mut slicer = Slicer::new(full);
    let mut parts = Vec::with_capacity(spans.len());
    let mut truncated = vec![];
    for (index, span) in spans.iter().enumerate() {
        let first = index == 0;
        let sliced = slicer.part(span.clone());
        if oversized.binary_search(&index).is_err() {
            let rendered = decorate(sliced, first);
            let values = request.values(&limits, &rendered);
            parts.push(Fitted { measurements: request.measurements(&limits, &values), rendered });
            continue;
        }
        let (fitted, change) = request
            .fit_part(&|| Outline::of_rendered(&sliced.text), &|cut| decorate(shortened(&sliced, cut), first))?;
        if let Some(change) = change {
            let units = change.range.map_or(0, |range| range.start as usize);
            truncated.push((index, span.start + offset(&sliced.text, units), change));
        }
        parts.push(fitted);
    }
    Some(Thread { parts, spans, boundaries, truncated })
}

fn part_limits(request: &Request, full: &Rendered, suffix: Option<&str>) -> Vec<Bound> {
    let mut limits = request.limits();
    for limit in &mut limits {
        limit.binding |= limit.unit == Unit::Links
            && limit.maximum.is_some_and(|maximum| suffix.is_some() || request.measure(Unit::Links, full) > maximum);
    }
    limits
}

const FIRST_WINDOW: usize = 2048;

struct Window {
    start: usize,
    far: usize,
    complete: bool,
    ends: Vec<usize>,
    admitted: Vec<Option<bool>>,
}

impl Window {
    fn new(
        request: &Request,
        limits: &[Bound],
        (text, words): (&str, &[(usize, usize)]),
        start: usize,
        (width, near): (usize, usize),
    ) -> Self {
        let far = text.floor_char_boundary(start.saturating_add(width));
        let from = words.partition_point(|&(end, _)| end <= start);
        let upto = words.partition_point(|&(end, _)| end < far);
        let mut ends: Vec<usize> = words[from..upto].iter().map(|&(end, _)| end - start).collect();
        ends.push(far - start);
        let admitted = request.admitted(limits, &text[start..far], &ends);
        Window { start, far, complete: width >= near, ends, admitted }
    }

    fn covers(&self, end: usize) -> bool {
        end <= self.far || self.complete
    }

    fn admitted(&self, end: usize) -> Option<bool> {
        if end > self.far {
            return self.admitted.last().copied().flatten().filter(|admitted| !admitted);
        }
        self.ends.binary_search(&(end - self.start)).ok().and_then(|index| self.admitted[index])
    }
}

fn skipped(text: &str, at: usize) -> usize {
    text[at..].find(|char: char| !char.is_whitespace()).map_or(text.len(), |offset| at + offset)
}

fn offset(text: &str, units: usize) -> usize {
    let mut counted = 0;
    text.char_indices()
        .find(|&(_, char)| {
            counted += char.len_utf16();
            counted > units
        })
        .map_or(text.len(), |(at, _)| at)
}

fn shortened(sliced: &Rendered, cut: Option<&Cut>) -> Rendered {
    let Some(cut) = cut else { return sliced.clone() };
    let (kept, end) = (cut.kept, cut.replacement.len());
    let links = sliced.links.iter().filter(|link| link.end <= kept).cloned().collect();
    let styled = sliced
        .styled
        .iter()
        .filter(|styled| styled.span.start <= kept)
        .map(|styled| {
            let span = styled.span.start..if styled.span.end > kept { end } else { styled.span.end };
            Styled { span, ..styled.clone() }
        })
        .collect();
    Rendered { links, styled, ..Rendered::text(cut.replacement.clone()) }
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
            if !cluster.ends_with(char::is_whitespace) {
                ends.push(at);
            }
        }
        true
    };
    let mut frontier = 0;
    while reached(frontier, &mut ends) && fits(ends[frontier]) {
        frontier = frontier * 2 + 1;
    }
    let count = ends.len().min(frontier + 1);
    let end = ends[last_fitting(count, |index| fits(ends[index]))?];
    Some((end, skipped(text, end)))
}

struct Slicer<'a> {
    full: &'a Rendered,
    order: Vec<usize>,
    next: usize,
    active: Vec<usize>,
}

impl<'a> Slicer<'a> {
    fn new(full: &'a Rendered) -> Self {
        let mut order: Vec<usize> = (0..full.styled.len()).collect();
        crate::order::sort_by_key(&mut order, |&index| full.styled[index].span.start as u64);
        Self { full, order, next: 0, active: vec![] }
    }

    fn part(&mut self, span: Span<usize>) -> Rendered {
        let full = self.full;
        let shift = |offset: usize| offset - span.start;
        let from = full.links.partition_point(|link| link.start < span.start);
        let links = full.links[from..]
            .iter()
            .take_while(|link| link.start < span.end)
            .filter(|link| link.end <= span.end)
            .map(|link| shift(link.start)..shift(link.end))
            .collect();
        while let Some(&index) = self.order.get(self.next).filter(|&&index| full.styled[index].span.start < span.end) {
            self.active.push(index);
            self.next += 1;
        }
        self.active.retain(|&index| full.styled[index].span.end > span.start);
        crate::order::sort_by_key(&mut self.active, |&index| index as u64);
        let styled = self
            .active
            .iter()
            .filter_map(|&index| {
                let styled = &full.styled[index];
                let (start, end) = (styled.span.start.max(span.start), styled.span.end.min(span.end));
                (start < end).then(|| Styled { span: shift(start)..shift(end), ..styled.clone() })
            })
            .collect();
        Rendered { links, styled, ..Rendered::text(full.text[span].to_owned()) }
    }
}

fn merged(mut spans: Vec<Span<usize>>) -> Vec<Span<usize>> {
    crate::order::sort_by_key(&mut spans, |span| span.start as u64);
    let mut merged: Vec<Span<usize>> = vec![];
    for span in spans {
        match merged.last_mut() {
            Some(last) if span.start < last.end => last.end = last.end.max(span.end),
            _ => merged.push(span),
        }
    }
    merged
}

fn breaks(text: &str, merged: &[Span<usize>]) -> [Vec<(usize, usize)>; 3] {
    let mut covering = 0;
    let mut classes: [Vec<(usize, usize)>; 3] = [vec![], vec![], vec![]];
    let mut at = 0;
    while let Some(char) = text[at..].chars().next() {
        if !char.is_whitespace() {
            at += char.len_utf8();
            continue;
        }
        let end = skipped(text, at);
        while merged.get(covering).is_some_and(|span| span.end <= at) {
            covering += 1;
        }
        if at > 0 && merged.get(covering).is_none_or(|span| span.start >= at) {
            let sentence = text[..at].chars().rev().find(|char| !closing(*char));
            if text[at..end].matches('\n').count() >= 2 {
                classes[0].push((at, end));
            }
            if sentence.is_some_and(terminal) {
                classes[1].push((at, end));
            }
            classes[2].push((at, end));
        }
        at = end;
    }
    classes
}

#[cfg(test)]
mod tests {
    use super::super::{Facts, Options, Rendered, Request, Source};
    use super::part_limits;
    use crate::targets::TARGETS;

    const TOKENS: [&str; 52] = [
        "word",
        "ab",
        "x",
        "1",
        "#",
        "*",
        "é",
        "e\u{301}",
        "中文",
        "\u{600}",
        "\u{915}\u{94d}",
        "\u{915}\u{94d}\u{937}",
        "👍",
        "👍\u{1F3FD}",
        "👨\u{200D}👩\u{200D}👧",
        "1\u{FE0F}\u{20E3}",
        "#\u{FE0F}\u{20E3}",
        "🇺🇸",
        "🇺",
        "\u{2764}",
        "\u{2764}\u{FE0F}",
        "\u{200D}",
        "\u{FE0F}",
        "https://example.com",
        "https://example.com/a_b(c)?q=1&r=2",
        "http://t.co/abc123",
        "https://t.co/",
        "example.com",
        "a.co/x",
        "xn--p1ai.xn--p1ai",
        "dat://a.b",
        "gemini://x.org/",
        "a@b.co",
        "@user",
        "@user@mastodon.social",
        "@a@b",
        "#tag",
        ".",
        ",",
        "?",
        "!",
        "(",
        ")",
        "/",
        ":",
        "_",
        "-",
        "@",
        "'",
        "\"",
        "…",
        "例え.jp",
    ];
    const SEPARATORS: [&str; 10] = [" ", "\n", "\n\n", "\u{a0}", "\u{3000}", "\u{2009}", "\t", "\u{85}", "", ""];

    fn next(seed: &mut u64) -> usize {
        *seed ^= *seed << 13;
        *seed ^= *seed >> 7;
        *seed ^= *seed << 17;
        (*seed >> 16) as usize
    }

    fn generated(seed: &mut u64) -> String {
        let mut text = String::new();
        for _ in 0..=next(seed) % 16 {
            text.push_str(TOKENS[next(seed) % TOKENS.len()]);
            text.push_str(SEPARATORS[next(seed) % SEPARATORS.len()]);
        }
        text
    }

    #[test]
    fn window_prefixes_match_measurement_from_scratch() {
        let variants = [
            Facts::default(),
            Facts { spoiler_text: Some("cw see example.com "), ..Facts::default() },
            Facts { characters_reserved_per_url: Some(0), ..Facts::default() },
            Facts { characters_reserved_per_url: Some(u32::MAX), ..Facts::default() },
        ];
        let options = [Options::default(), Options { maximum: Some(12), ..Options::default() }];
        let mut tested = vec![];
        for target in TARGETS.iter().filter(|target| target.threads()) {
            if target.form.splits_by_nodes() {
                continue;
            }
            tested.push(target.name);
            let mut seed = 0x9E37_79B9_7F4A_7C15;
            for (facts, options) in
                variants.iter().flat_map(|facts| options.iter().map(move |options| (facts, options)))
            {
                let spec = target.fields(facts).find(|spec| spec.source == Source::Body).expect("body field");
                let request = Request { target, spec, facts, options, spent: [0; 2] };
                let limits = part_limits(&request, &Rendered::text(String::new()), Some(""));
                let units: Vec<_> = limits.iter().filter(|limit| limit.binding).map(|limit| limit.unit).collect();
                assert!(!units.is_empty(), "{}", target.name);
                let plain = "alpha beta\u{a0}gamma\n\ndelta 👍\u{1F3FD} 中文 end";
                for text in std::iter::once(plain.to_owned()).chain((0..60).map(|_| generated(&mut seed))) {
                    let ends: Vec<usize> = (0..=text.len()).filter(|&end| text.is_char_boundary(end)).collect();
                    for &unit in &units {
                        let prefixes = request.prefixes(unit, &text, &ends);
                        for (&end, value) in ends.iter().zip(&prefixes) {
                            let expected = request.measure_text(unit, &text[..end]);
                            assert!(
                                value.is_none_or(|value| value == expected),
                                "{} {unit:?} {text:?} {end}",
                                target.name
                            );
                            let cut = text[end..].chars().next().is_none_or(char::is_whitespace);
                            assert!(text != plain || !cut || value.is_some(), "{} {unit:?} {end}", target.name);
                        }
                    }
                    for (&end, admitted) in ends.iter().zip(request.admitted(&limits, &text, &ends)) {
                        let expected = request.admits_text(&limits, &text[..end]);
                        assert!(admitted.is_none_or(|admitted| admitted == expected), "{} {text:?} {end}", target.name);
                    }
                }
            }
        }
        for name in ["x-plain", "mastodon-plain", "bluesky-entities", "threads-entities"] {
            assert!(tested.contains(&name), "{name} in {tested:?}");
        }
    }
}
