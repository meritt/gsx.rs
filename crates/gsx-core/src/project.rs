mod change;
mod entities;
mod facts;
mod fit;
mod layout;
pub mod links;
mod markup;
mod media;
pub mod nfc;
mod outline;
mod table;
mod thread;

use std::borrow::Cow;
use std::cell::OnceCell;
use std::collections::HashMap;
use std::ops::Range as Span;

use crate::document::{Document, Kind, Node, Provenance, Range, TimeFormat};
use crate::measure::{self, Fact};

pub use crate::measure::{MediaKind, Placement, Unit};
pub use change::{Change, Class, Feature, FieldName, Slot, Truncated};
pub use entities::Entity;
pub use facts::{FACT_MAX_ITEMS, FACT_MAX_UNITS, Facts, Format, MediaFact, Overflow};
pub use fit::WINDOW;
pub(crate) use layout::blank;
pub use layout::{CELL, ELLIPSIS, QUOTE, RULE, Writer, marker, single_line};
pub use links::href;
pub use media::{Attachment, Block, Item, placed, references};
pub use table::{
    BlocksForm, Ceiling, Condition, Detection, EntityForm, Escape, FieldSpec, Form, Group, HtmlBlocks, HtmlForm,
    LineStart, MarkdownForm, MathTags, Media, MediaShape, Normalization, Offsets, Renderer, Rule, Search, Source,
    SpoilerMode, Spoilers, Structure, Style, Target, TimeFormats, Truncation, Unplaced, When, date_refusal,
    time_format,
};

use layout::Styled;
use media::Anchor;
use outline::{Cut, Outline};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    pub name: FieldName,
    pub text: String,
    pub links: Vec<Range>,
    pub entities: Vec<Entity>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Measurement {
    pub field: FieldName,
    pub unit: Unit,
    pub value: usize,
    pub maximum: Option<usize>,
    pub block: Option<usize>,
    pub item: Option<usize>,
}

impl Measurement {
    fn exceeds(&self) -> bool {
        self.maximum.is_some_and(|maximum| self.value > maximum)
    }

    fn links_exceeded(measurements: &[Measurement]) -> bool {
        measurements.iter().any(|measured| measured.unit == Unit::Links && measured.exceeds())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Projection {
    pub fields: Vec<Field>,
    pub measurements: Vec<Measurement>,
    pub changes: Vec<Change>,
    pub media: Vec<String>,
    pub blocks: Vec<Block>,
    pub items: Vec<Attachment>,
    pub parts: Vec<Part>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Part {
    pub fields: Vec<Field>,
    pub measurements: Vec<Measurement>,
    pub media: Vec<String>,
    pub items: Vec<Attachment>,
    pub range: Range,
    pub boundary: Option<Boundary>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boundary {
    Paragraph,
    Sentence,
    Word,
    Grapheme,
}

impl Boundary {
    pub const fn name(self) -> &'static str {
        match self {
            Boundary::Paragraph => "paragraph",
            Boundary::Sentence => "sentence",
            Boundary::Word => "word",
            Boundary::Grapheme => "grapheme",
        }
    }
}

impl Projection {
    pub fn field(&self, name: FieldName) -> Option<&Field> {
        self.fields.iter().find(|field| field.name == name)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    pub search: Option<Search>,
    pub maximum: Option<usize>,
}

#[derive(Debug)]
pub struct Context<'a> {
    pub target: &'static Target,
    field: FieldName,
    writer: Writer,
    changes: Vec<Change>,
    cut: Option<&'a Cut<'a>>,
    done: bool,
    link_depth: u32,
    open: Vec<Style>,
    allowed: bool,
    items: Option<&'a [MediaFact<'a>]>,
    positions: OnceCell<HashMap<&'a str, usize>>,
    anchors: Vec<Anchor>,
    alts: Vec<(usize, Option<String>)>,
    urls: Vec<String>,
    url_cap: Option<usize>,
    uncapped: bool,
    thread: bool,
}

impl<'a> Context<'a> {
    pub fn items(&self) -> &[MediaFact<'_>] {
        self.items.unwrap_or_default()
    }

    fn positions(&self) -> &HashMap<&'a str, usize> {
        self.positions.get_or_init(|| media::positions(self.items.unwrap_or_default()))
    }

    pub fn position(&self, id: &str) -> Option<usize> {
        self.positions().get(id).copied()
    }

    pub fn admit(&mut self, href: &str) -> bool {
        let Some(cap) = self.url_cap else { return true };
        if self.urls.iter().any(|url| url == href) {
            return true;
        }
        if self.urls.len() < cap {
            self.urls.push(href.to_owned());
            return true;
        }
        self.uncapped = self.thread;
        self.thread
    }

    pub fn published_alt(&mut self, item: usize, alt: Option<String>) {
        self.alts.push((item, alt));
    }

    pub fn gap(&mut self, after: Option<u32>, node: &Node, gap: u32) -> u32 {
        if gap <= 2 {
            return gap;
        }
        if !self.target.conditions(Feature::Text, When::BlankLines) {
            return gap;
        }
        if let Some(start) = after {
            let range = Range { start, end: node.range.start.max(start) };
            self.report_when(Feature::Text, Some(range), Some(When::BlankLines));
        }
        2
    }

    pub fn break_before(&mut self, after: Option<u32>, node: &Node) {
        let gap = self.gap(after, node, node.gap_before.unwrap_or(1));
        self.writer.newlines(gap);
    }

    pub fn anchor(&mut self, node: Option<&Node>) {
        self.anchors.push(Anchor { at: self.writer.mark(), node: node.map(|node| node.range), cut: self.done });
    }

    pub fn declared(&self, feature: Feature) -> bool {
        self.target.form.entities().is_some_and(|form| form.style(feature).is_some())
    }

    fn open_form(&self) -> Option<&'static EntityForm> {
        self.target.form.entities().filter(|form| self.allowed || !form.gated)
    }

    pub fn native(&self, feature: Feature) -> Option<Style> {
        self.open_form().and_then(|form| form.style(feature))
    }

    pub fn time_format(&self, format: Option<TimeFormat>) -> Option<&'static str> {
        self.open_form().and_then(|form| table::time_format(form.time_formats, format))
    }

    pub fn heading(&self) -> Option<Style> {
        self.open_form().and_then(|form| form.heading)
    }

    pub fn styled(
        &mut self,
        style: Style,
        value: Option<String>,
        feature: Feature,
        node: &Node,
        draw: impl FnOnce(&mut Self),
    ) {
        self.formatted(style, value, None, feature, node, draw);
    }

    pub fn formatted(
        &mut self,
        style: Style,
        value: Option<String>,
        format: Option<&'static str>,
        feature: Feature,
        node: &Node,
        draw: impl FnOnce(&mut Self),
    ) {
        let mark = self.writer.mark();
        self.within(style, draw);
        if !self.open.contains(&style) {
            self.writer.style(style, value, format, mark, Some((feature, node.range)));
        }
    }

    pub fn within(&mut self, style: Style, draw: impl FnOnce(&mut Self)) {
        self.open.push(style);
        draw(self);
        self.open.pop();
    }

    pub fn inside(&self, style: Style) -> bool {
        self.open.contains(&style)
    }

    pub fn nested(&self) -> bool {
        !self.open.is_empty() || self.link_depth > 0
    }

    pub fn linked(&self) -> bool {
        self.link_depth > 0
    }

    pub fn within_link(&mut self, draw: impl FnOnce(&mut Self)) {
        self.link_depth += 1;
        draw(self);
        self.link_depth -= 1;
    }

    pub fn report(&mut self, feature: Feature, range: Range) {
        self.report_when(feature, Some(range), None);
    }

    pub fn report_media(&mut self, node: &Node) {
        self.anchor(Some(node));
        let ids = node.media_ids().unwrap_or_default();
        let listed = |id: &String| self.items.is_none() || self.positions().contains_key(id.as_str());
        let (some_listed, some_missing) = (ids.iter().any(listed), !ids.iter().all(listed));
        if some_missing {
            self.report_when(Feature::Media, Some(node.range), Some(When::Missing));
        }
        if some_listed {
            self.report(Feature::Media, node.range);
        }
    }

    pub fn report_when(&mut self, feature: Feature, range: Option<Range>, when: Option<When>) {
        let (class, reason) = self.target.rule(feature).outcome(when);
        self.changes.push(Change { class, feature, reason, field: self.field, range, truncated: None, slot: None });
    }

    pub fn record(&mut self, change: Change) {
        self.changes.push(change);
    }

    pub fn class(&self, feature: Feature) -> Class {
        self.target.class(feature)
    }

    pub fn writer(&mut self) -> &mut Writer {
        &mut self.writer
    }

    pub fn group(&mut self, mark: usize) {
        if self.thread {
            self.writer.group(mark);
        }
    }

    pub fn done(&self) -> bool {
        self.done
    }

    pub fn leaf(&mut self, node: &Node, value: &str) {
        self.writer.source(node.range);
        let authored =
            self.link_depth == 0 && node.provenance == Provenance::Authored && matches!(node.kind, Kind::Text { .. });
        let start = self.writer.mark();
        if let Some(cut) = self.cut_at(node) {
            self.writer.push(&cut.replacement[..cut.kept]);
            let kept = self.writer.mark();
            self.push_suffix(&cut.replacement[cut.kept..]);
            if authored {
                self.writer.eligible(start..kept);
            }
            self.done = true;
        } else {
            self.writer.push(value);
            if authored {
                let end = self.writer.mark();
                self.writer.eligible(start..end);
            }
            if self.thread {
                self.writer.origin(start, node.range);
            }
        }
    }

    pub fn leaf_with(&mut self, node: &Node, value: &str, map: impl Fn(&str, Piece) -> String) {
        self.writer.source(node.range);
        if let Some(cut) = self.cut_at(node) {
            let (kept, suffix) = cut.replacement.split_at(cut.kept);
            self.writer.push(&map(kept, Piece::Kept));
            self.push_suffix(&map(suffix, Piece::Suffix));
            self.done = true;
        } else {
            let start = self.writer.mark();
            self.writer.push(&map(value, Piece::Whole));
            if self.thread {
                self.writer.origin(start, node.range);
            }
        }
    }

    fn cut_at(&self, node: &Node) -> Option<&'a Cut<'a>> {
        self.cut.filter(|cut| cut.leaf.node.is_some_and(|leaf| std::ptr::eq(leaf, node)))
    }

    fn push_suffix(&mut self, suffix: &str) {
        let start = self.writer.mark();
        self.writer.push(suffix);
        let end = self.writer.mark();
        if end > start {
            self.writer.suffix(start..end);
        }
    }

    pub fn rendered<'s>(&'s self, node: &Node, value: &'s str) -> &'s str {
        self.cut_at(node).map_or(value, |cut| &cut.replacement)
    }

    pub fn contains_cut(&self, node: &Node) -> bool {
        self.cut.and_then(|cut| cut.leaf.node).is_some_and(|leaf| contains(node, leaf))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Piece {
    Whole,
    Kept,
    Suffix,
}

fn contains(node: &Node, leaf: &Node) -> bool {
    std::ptr::eq(node, leaf) || node.children.iter().any(|child| contains(child, leaf))
}

#[derive(Clone, Debug)]
struct Rendered {
    text: String,
    links: Vec<Span<usize>>,
    styled: Vec<Styled>,
    changes: Vec<Change>,
    anchors: Vec<Anchor>,
    alts: Vec<(usize, Option<String>)>,
    prefixes: Vec<Span<usize>>,
    origins: Vec<(Span<usize>, Range)>,
    groups: Vec<Span<usize>>,
    uncapped: bool,
}

impl Rendered {
    fn text(text: String) -> Self {
        Rendered {
            text,
            links: vec![],
            styled: vec![],
            changes: vec![],
            anchors: vec![],
            alts: vec![],
            prefixes: vec![],
            origins: vec![],
            groups: vec![],
            uncapped: false,
        }
    }

    fn unprefixed(&self) -> String {
        let mut out = String::with_capacity(self.text.len());
        let mut cursor = 0;
        for prefix in &self.prefixes {
            out.push_str(&self.text[cursor..prefix.start]);
            cursor = prefix.end;
        }
        out.push_str(&self.text[cursor..]);
        out
    }
}

const MARKERS: [(Unit, Feature); 2] = [(Unit::Hashtags, Feature::Hashtag), (Unit::Mentions, Feature::Mention)];

type Spent = [usize; 2];

fn render(
    target: &'static Target,
    facts: &Facts,
    field: FieldName,
    spent: Spent,
    cut: Option<&Cut>,
    thread: bool,
    draw: impl FnOnce(&mut Context),
) -> Rendered {
    let mut context = Context {
        target,
        field,
        writer: Writer::new(target.form.escape()),
        changes: vec![],
        cut,
        done: false,
        link_depth: 0,
        open: vec![],
        allowed: facts.entities_allowed,
        items: facts.items,
        positions: OnceCell::new(),
        anchors: vec![],
        alts: vec![],
        urls: vec![],
        url_cap: target.form.entities().and_then(|form| form.links).map(|cap| {
            let reserved = facts.target_url.is_some() && target.handles(Feature::TargetUrl);
            cap - usize::from(reserved)
        }),
        uncapped: false,
        thread,
    };
    draw(&mut context);
    let mut written = std::mem::replace(&mut context.writer, Writer::new(Escape::None)).finish();
    let kept = written.trim_lines();
    for anchor in &mut context.anchors {
        anchor.at = anchor.at.clamp(kept.start, kept.end) - kept.start;
    }
    for &range in &written.replaced {
        context.report(Feature::AngleBrackets, range);
    }
    let styled = match target.form.entities() {
        Some(form) => {
            let finished = entities::finish(form, &mut written);
            for (feature, range) in finished.capped {
                context.report_when(feature, Some(range), Some(When::Capped));
            }
            finished.styled
        }
        None => vec![],
    };
    let mut anchors = context.anchors;
    let mut origins = written.origins;
    let mut groups = written.groups;
    let (text, links, prefixes) = match target.normalization {
        Normalization::None => (written.text, written.links, written.prefixes),
        Normalization::Nfc => {
            nfc_offsets(&written.text, anchors.iter_mut().map(|anchor| &mut anchor.at));
            nfc_offsets(&written.text, origins.iter_mut().flat_map(|(span, _)| [&mut span.start, &mut span.end]));
            nfc_offsets(&written.text, groups.iter_mut().flat_map(|span| [&mut span.start, &mut span.end]));
            let (text, links) = normalized(&written.text, &written.links);
            (text, links, vec![])
        }
    };
    let mut rendered = Rendered {
        text,
        links,
        styled,
        changes: context.changes,
        anchors,
        alts: context.alts,
        prefixes,
        origins,
        groups,
        uncapped: context.uncapped,
    };
    cap_markers(target, &resolved(facts), field, spent, &mut rendered);
    rendered
}

fn cap_markers(
    target: &'static Target,
    resolved: &[(Fact, u32)],
    field: FieldName,
    spent: Spent,
    rendered: &mut Rendered,
) {
    let facts = measure::Facts { resolved, ..measure::Facts::default() };
    for ((unit, feature), spent) in MARKERS.into_iter().zip(spent) {
        let cap = target.limits.iter().find(|limit| limit.field == field && limit.unit == unit);
        let Some(maximum) = cap.and_then(|limit| limit.maximum(&facts)) else {
            continue;
        };
        let markers = measure::markers(unit, &rendered.text);
        let kept = (maximum as usize).saturating_sub(spent);
        let Some(dropped) = markers.get(kept..).filter(|dropped| !dropped.is_empty()) else {
            continue;
        };
        rendered.text =
            without_sigils(&rendered.text, dropped, &mut rendered.links, &mut rendered.groups, &mut rendered.origins);
        rendered.prefixes.clear();
        let (class, reason) = target.rule(feature).outcome(None);
        let value = u32::try_from(spent + markers.len()).unwrap_or(u32::MAX);
        let truncated = Some(Truncated { unit, maximum, value, exact: true });
        rendered.changes.push(Change { class, feature, reason, field, range: None, truncated, slot: None });
    }
}

fn without_sigils(
    text: &str,
    sigils: &[usize],
    links: &mut [Span<usize>],
    groups: &mut [Span<usize>],
    origins: &mut [(Span<usize>, Range)],
) -> String {
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    for &at in sigils {
        out.push_str(&text[cursor..at]);
        cursor = at + 1;
    }
    out.push_str(&text[cursor..]);
    let shift = |offset: usize| offset - sigils.partition_point(|&at| at < offset);
    for span in links.iter_mut().chain(groups.iter_mut()).chain(origins.iter_mut().map(|(span, _)| span)) {
        *span = shift(span.start)..shift(span.end);
    }
    out
}

fn nfc_offsets<'o>(text: &str, offsets: impl Iterator<Item = &'o mut usize>) {
    let (mut from, mut to) = (0, 0);
    for offset in offsets {
        to += nfc::nfc(&text[from..*offset]).len();
        from = *offset;
        *offset = to;
    }
}

fn normalized(text: &str, links: &[Span<usize>]) -> (String, Vec<Span<usize>>) {
    let mut out = String::with_capacity(text.len());
    let mut spans = Vec::with_capacity(links.len());
    let mut cursor = 0;
    for link in links {
        out.push_str(&nfc::nfc(&text[cursor..link.start]));
        let start = out.len();
        out.push_str(&nfc::nfc(&text[link.clone()]));
        spans.push(start..out.len());
        cursor = link.end;
    }
    out.push_str(&nfc::nfc(&text[cursor..]));
    (out, spans)
}

type Probe = (Rendered, Vec<usize>);
type Entities<'t> = (Cow<'t, str>, Vec<Span<usize>>, Vec<Span<usize>>, bool);

fn within(limits: &[Bound], values: &[usize]) -> bool {
    limits.iter().zip(values).all(|(limit, value)| limit.cap().is_none_or(|maximum| *value <= maximum))
}

struct Fitted {
    rendered: Rendered,
    measurements: Vec<Measurement>,
}

struct Split {
    parts: Vec<Fitted>,
    ranges: Vec<Range>,
    boundaries: Vec<Option<Boundary>>,
    media: Vec<Vec<String>>,
}

#[derive(Clone, Copy, Debug)]
struct Bound {
    unit: Unit,
    maximum: Option<usize>,
    binding: bool,
}

impl Bound {
    fn cap(self) -> Option<usize> {
        self.maximum.filter(|_| self.binding)
    }

    fn binds(self) -> bool {
        self.cap().is_some()
    }
}

struct Request<'a, 'b> {
    target: &'static Target,
    spec: &'a FieldSpec,
    facts: &'a Facts<'b>,
    options: &'a Options,
    spent: Spent,
}

fn resolved(facts: &Facts) -> Vec<(Fact, u32)> {
    [
        (Fact::MaxCharacters, facts.max_characters),
        (Fact::CharactersReservedPerUrl, facts.characters_reserved_per_url),
        (Fact::CaptionLimit, facts.caption_limit),
        (Fact::DescriptionLimit, facts.description_limit),
        (Fact::HashtagLimit, facts.hashtag_limit),
    ]
    .into_iter()
    .filter_map(|(fact, value)| value.map(|value| (fact, value)))
    .collect()
}

impl Request<'_, '_> {
    fn resolved(&self) -> Vec<(Fact, u32)> {
        resolved(self.facts)
    }

    fn measure(&self, unit: Unit, rendered: &Rendered) -> usize {
        self.measure_text(unit, &rendered.text)
    }

    fn entities<'t>(&self, unit: Unit, text: &'t str) -> Entities<'t> {
        let mut text = self.target.form.visible(text);
        if let (Unit::MastodonWeighted, Some(spoiler)) = (unit, self.facts.spoiler_text) {
            text = Cow::Owned(format!("{spoiler}{text}"));
        }
        let (urls, elided, whole) = match unit {
            Unit::XWeighted | Unit::Links => match links::x_urls(&text) {
                Some(urls) => (urls, vec![], true),
                None => (vec![], vec![], false),
            },
            Unit::MastodonWeighted => {
                let (urls, elided) = links::mastodon_entities(&text);
                (urls, elided, true)
            }
            _ => (vec![], vec![], true),
        };
        (text, urls, elided, whole)
    }

    fn measure_text(&self, unit: Unit, text: &str) -> usize {
        let (text, urls, elided, _) = self.entities(unit, text);
        let resolved = self.resolved();
        let facts = measure::Facts { urls: &urls, elided: &elided, resolved: &resolved };
        let earlier = MARKERS.into_iter().zip(self.spent).find(|((marker, _), _)| *marker == unit);
        measure::measure(unit, &text, &facts) as usize + earlier.map_or(0, |(_, spent)| spent)
    }

    fn change(&self, feature: Feature, range: Option<Range>, when: Option<When>) -> Change {
        let (class, reason) = self.target.rule(feature).outcome(when);
        Change { class, feature, reason, field: self.spec.name, range, truncated: None, slot: None }
    }

    fn limits(&self) -> Vec<Bound> {
        let field = self.spec.name;
        let placement = self
            .facts
            .placement
            .into_iter()
            .chain([Placement::Post, Placement::Reel, Placement::Story])
            .find(|&placement| {
                self.target.applicable(field, measure::Context { placement, media: self.facts.media }).next().is_some()
            });
        let Some(placement) = placement else {
            return vec![];
        };
        let resolved = self.resolved();
        let facts = measure::Facts { resolved: &resolved, ..measure::Facts::default() };
        let mut limits: Vec<Bound> = self
            .target
            .applicable(field, measure::Context { placement, media: self.facts.media })
            .map(|limit| Bound {
                unit: limit.unit,
                maximum: limit.maximum(&facts).map(|maximum| maximum as usize),
                binding: !limit.unit.counts_markers(),
            })
            .collect();
        if let Some(first) = limits.iter_mut().find(|limit| limit.binding) {
            first.maximum = self.options.maximum.or(first.maximum);
        }
        limits
    }

    fn fits(&self, rendered: &Rendered) -> bool {
        self.admits(&self.limits(), rendered)
    }

    fn admits(&self, limits: &[Bound], rendered: &Rendered) -> bool {
        self.admits_text(limits, &rendered.text)
    }

    fn prefixes(&self, unit: Unit, text: &str, ends: &[usize]) -> Vec<Option<usize>> {
        let (joined, urls, elided, whole) = self.entities(unit, text);
        if !whole || self.target.form.html().is_some() {
            return vec![None; ends.len()];
        }
        let shift = joined.len() - text.len();
        let ends: Vec<usize> = ends.iter().map(|end| end + shift).collect();
        let resolved = self.resolved();
        let facts = measure::Facts { urls: &urls, elided: &elided, resolved: &resolved };
        measure::prefixes(unit, &joined, &facts, &ends)
    }

    fn admitted(&self, limits: &[Bound], text: &str, ends: &[usize]) -> Vec<Option<bool>> {
        let mut admitted = vec![Some(true); ends.len()];
        for limit in limits {
            let Some(maximum) = limit.cap() else { continue };
            for (admitted, value) in admitted.iter_mut().zip(self.prefixes(limit.unit, text, ends)) {
                *admitted = match (*admitted, value.map(|value| value <= maximum)) {
                    (Some(false), _) | (_, Some(false)) => Some(false),
                    (Some(true), Some(true)) => Some(true),
                    _ => None,
                };
            }
        }
        admitted
    }

    fn admits_text(&self, limits: &[Bound], text: &str) -> bool {
        limits.iter().all(|limit| limit.cap().is_none_or(|maximum| self.measure_text(limit.unit, text) <= maximum))
    }

    fn values(&self, limits: &[Bound], rendered: &Rendered) -> Vec<usize> {
        limits.iter().map(|limit| self.measure(limit.unit, rendered)).collect()
    }

    fn bounded(&self, limits: &[Bound], rendered: &Rendered) -> Vec<usize> {
        limits.iter().map(|limit| if limit.binds() { self.measure(limit.unit, rendered) } else { 0 }).collect()
    }

    fn measurements(&self, limits: &[Bound], values: &[usize]) -> Vec<Measurement> {
        limits
            .iter()
            .zip(values)
            .map(|(limit, &value)| Measurement {
                field: self.spec.name,
                unit: limit.unit,
                value,
                maximum: limit.maximum,
                block: None,
                item: None,
            })
            .collect()
    }

    fn fit<'o>(
        &self,
        outline: &dyn Fn() -> Outline<'o>,
        bounded: bool,
        draw: &dyn Fn(Option<&Cut>) -> Rendered,
    ) -> Fitted {
        self.fit_from(draw(None), outline, bounded, draw)
    }

    fn fit_from<'o>(
        &self,
        full: Rendered,
        outline: &dyn Fn() -> Outline<'o>,
        bounded: bool,
        draw: &dyn Fn(Option<&Cut>) -> Rendered,
    ) -> Fitted {
        let limits = self.limits();
        let values = self.values(&limits, &full);
        let fits = |values: &[usize]| within(&limits, values);
        let measurements = self.measurements(&limits, &values);
        if fits(&values) {
            return Fitted { rendered: full, measurements };
        }
        let ratio = |measurement: &Measurement| match measurement.maximum {
            Some(maximum) if !measurement.unit.counts_markers() => measurement.value as f64 / maximum.max(1) as f64,
            _ => 0.0,
        };
        let worst = measurements
            .iter()
            .fold(&measurements[0], |worst, item| if ratio(item) > ratio(worst) { item } else { worst });
        let Measurement { unit, value, maximum: Some(maximum), .. } = *worst else {
            unreachable!("an overflowing field has a limited measurement")
        };
        let outline = outline();
        let overflow = |mut rendered: Rendered| {
            rendered.changes.push(self.change(Feature::Text, None, Some(When::Overflow)));
            Fitted { rendered, measurements: measurements.clone() }
        };
        if outline.is_empty() {
            return overflow(full);
        }
        let least = limits.iter().filter_map(|limit| limit.cap()).min().unwrap_or(maximum);
        let bound = bounded.then(|| outline.upper(least)).flatten();
        let upper = bound.unwrap_or_else(|| outline.units() - 1);
        let load = |values: &[usize]| {
            limits
                .iter()
                .zip(values)
                .filter_map(|(limit, &value)| limit.cap().map(|maximum| value as f64 / maximum.max(1) as f64))
                .fold(0.0, f64::max)
        };
        let mut probes: HashMap<(u32, usize), (bool, f64)> = HashMap::new();
        let mut best: Option<(usize, (u32, usize), Probe)> = None;
        let mode = self.options.search.unwrap_or(self.target.search);
        let start = bound.map(|upper| outline.share(1.0 / load(&values), upper));
        let keep = fit::search(
            mode,
            upper,
            start,
            |keep| outline.clean(keep),
            |keep| {
                let cut = outline.cut(keep, self.spec.truncation);
                *probes.entry(cut.key()).or_insert_with(|| {
                    let rendered = draw(Some(&cut));
                    let values = self.bounded(&limits, &rendered);
                    let (fit, load) = (fits(&values), load(&values));
                    if fit && best.as_ref().is_none_or(|(kept, ..)| keep > *kept) {
                        best = Some((keep, cut.key(), (rendered, values)));
                    }
                    (fit, load)
                })
            },
        );
        let Some(keep) = keep else {
            return overflow(full);
        };
        let cut = outline.cut(keep, self.spec.truncation);
        let (mut rendered, mut fitted) = match best {
            Some((_, key, probe)) if key == cut.key() => probe,
            _ => {
                let rendered = draw(Some(&cut));
                let values = self.bounded(&limits, &rendered);
                (rendered, values)
            }
        };
        for (limit, value) in limits.iter().zip(&mut fitted) {
            if !limit.binds() {
                *value = self.measure(limit.unit, &rendered);
            }
        }
        let (range, exact) = cut.dropped(outline.end());
        let mut change = self.change(Feature::Text, Some(range), None);
        change.truncated = Some(Truncated { unit, maximum: maximum as u32, value: value as u32, exact });
        rendered.changes.push(change);
        Fitted { rendered, measurements: self.measurements(&limits, &fitted) }
    }

    fn fit_part<'o>(
        &self,
        outline: &dyn Fn() -> Outline<'o>,
        draw: &dyn Fn(Option<&Cut>) -> Rendered,
    ) -> Option<(Fitted, Option<Change>)> {
        let mut fitted = self.fit(outline, true, draw);
        if fitted.measurements.iter().any(|measured| !measured.unit.counts_markers() && measured.exceeds()) {
            return None;
        }
        let changes = &mut fitted.rendered.changes;
        let truncated = changes
            .iter()
            .rposition(|change| change.feature == Feature::Text && change.truncated.is_some())
            .map(|index| changes.remove(index));
        Some((fitted, truncated))
    }
}

fn appended(target: &Target, mut rendered: Rendered, url: Option<&str>) -> Rendered {
    if let Some(url) = url {
        if target.form.visible_blank(&rendered.text, measure::blank) {
            rendered.text.clear();
            rendered.links.clear();
            rendered.styled.clear();
            rendered.changes.retain(|change| change.feature == Feature::Media);
        } else {
            rendered.text.push_str("\n\n");
        }
        let start = rendered.text.len();
        rendered.text.push_str(url);
        rendered.links.push(start..rendered.text.len());
    }
    rendered
}

fn without_first_line(text: &str) -> String {
    let mut lines = text.split('\n');
    lines.by_ref().find(|line| !line.trim().is_empty());
    lines.collect::<Vec<_>>().join("\n").trim_start().to_owned()
}

fn first_line(text: &str) -> String {
    single_line(text.split('\n').find(|line| !line.trim().is_empty()).unwrap_or(""))
}

pub fn shortened(text: &str, fits: impl Fn(&str) -> bool) -> String {
    let outline = Outline::of_text(text);
    let units = outline.units();
    if units == 0 {
        return ELLIPSIS.to_owned();
    }
    let (mut low, mut high) = (0, units - 1);
    let mut kept = ELLIPSIS.to_owned();
    while low <= high {
        let keep = low + (high - low) / 2;
        let candidate = outline.cut(keep, Truncation::Ellipsis).replacement;
        if fits(&candidate) {
            kept = candidate;
            low = keep + 1;
        } else if keep == 0 {
            break;
        } else {
            high = keep - 1;
        }
    }
    kept
}

fn utf16_ranges(text: &str, spans: &[Span<usize>]) -> Vec<Range> {
    let mut ranges = Vec::with_capacity(spans.len());
    let (mut byte, mut unit) = (0, 0u32);
    let mut advance = |to: usize| {
        unit += text[byte..to].encode_utf16().count() as u32;
        byte = to;
        unit
    };
    for span in spans {
        let start = advance(span.start);
        ranges.push(Range { start, end: advance(span.end) });
    }
    ranges
}

fn media(nodes: &[Node], out: &mut Vec<String>) {
    for node in nodes {
        match node.media_ids() {
            Some(ids) => out.extend_from_slice(ids),
            None => media(&node.children, out),
        }
    }
}

pub fn project(document: &Document, target: &'static Target, facts: &Facts) -> Projection {
    project_with(document, target, facts, &Options::default())
}

pub fn project_with(document: &Document, target: &'static Target, facts: &Facts, options: &Options) -> Projection {
    let children = document.children();
    let title = target.fields(facts).find_map(|spec| match spec.source {
        Source::Title { fact } => Some(fact),
        Source::Body => None,
    });
    let explicit = title.and_then(|fact| if fact { facts.title } else { None });
    let leading = children
        .first()
        .filter(|node| title.is_some() && explicit.is_none() && matches!(node.kind, Kind::Heading { .. }));
    let body = if leading.is_some() { &children[1..] } else { children };
    let math = target.form.math().is_some();
    let draw_heading = |heading: &Node, field, spent, cut: Option<&Cut>| {
        let mut rendered = render(target, facts, field, spent, cut, false, |context| {
            context.report_when(Feature::Heading, Some(heading.range), Some(When::TitleSource));
            (target.renderer.inline)(context, &heading.children);
        });
        rendered.text = single_line(&rendered.text);
        rendered.links.clear();
        rendered.styled.clear();
        rendered
    };
    let titled = leading.filter(|heading| !draw_heading(heading, FieldName::Title, [0; 2], None).text.is_empty());
    let removable = title.is_some() && titled.is_none() && explicit.is_none();
    let removed = removable
        && target.fields(facts).find(|spec| matches!(spec.source, Source::Title { .. })).is_some_and(|spec| {
            let request = Request { target, spec, facts, options, spent: [0; 2] };
            let blocks = render(target, facts, spec.name, [0; 2], None, false, |context| {
                (target.renderer.blocks)(context, body);
            });
            request.fits(&Rendered::text(first_line(&blocks.unprefixed())))
        });
    let draw_body = |field, spent, cut: Option<&Cut>, origins| {
        let spent = if removed { [0; 2] } else { spent };
        let mut rendered =
            render(target, facts, field, spent, cut, origins, |context| (target.renderer.blocks)(context, body));
        if removed {
            rendered.text = without_first_line(&rendered.text);
            rendered.links.clear();
            rendered.styled.clear();
        }
        rendered
    };

    let target_url = facts.target_url.filter(|_| target.handles(Feature::TargetUrl)).map(links::accepted);

    let mut fields = vec![];
    let mut measurements = vec![];
    let mut changes = vec![];
    let mut required = vec![];
    let mut blocks = vec![];
    let mut items = vec![];
    let mut alts = vec![];
    let mut derived = false;
    let mut threaded = None;
    let mut spent: Spent = [0; 2];
    for spec in target.fields(facts) {
        let request = Request { target, spec, facts, options, spent };
        let fitted = match (spec.source, explicit, titled) {
            (Source::Body, ..) => {
                if !derived && matches!(target_url, Some(None)) {
                    changes.push(request.change(Feature::TargetUrl, None, Some(When::Refused)));
                }
                let markup = target.form.splits_by_nodes();
                let threading = facts.overflow == Overflow::Thread
                    && facts.placement != Some(Placement::Story)
                    && target.threads()
                    && !derived;
                let mut full = None;
                let suffix = target_url.as_ref().and_then(Option::as_deref).filter(|_| !derived).filter(|url| {
                    let drawn = full.insert(draw_body(spec.name, spent, None, threading && !markup));
                    let blank = target.form.visible_blank(&drawn.text, measure::blank);
                    let least = if blank { (*url).to_owned() } else { format!("{ELLIPSIS}\n\n{url}") };
                    let fits = request.fits(&Rendered::text(least));
                    if !fits {
                        changes.push(request.change(Feature::TargetUrl, None, Some(When::Overflow)));
                    }
                    fits
                });
                if threading && full.is_none() {
                    full = Some(draw_body(spec.name, spent, None, !markup));
                }
                let split = full.as_ref().filter(|_| threading).and_then(|full| {
                    if markup {
                        markup::split(&request, document, body, full, suffix)
                    } else {
                        thread::split(&request, full, suffix).map(|thread| anchored(document, full, thread))
                    }
                });
                let mut fitted = match (split, full) {
                    (Some(mut split), Some(full)) => {
                        let mut first = split.parts.remove(0);
                        let mut changes = full.changes;
                        changes.reserve_exact(first.rendered.changes.len() + 1);
                        changes.append(&mut first.rendered.changes);
                        changes.push(request.change(Feature::Text, None, Some(When::Thread)));
                        for (part, range) in split.parts.iter().zip(&split.ranges[1..]) {
                            if Measurement::links_exceeded(&part.measurements) {
                                changes.push(request.change(Feature::Link, Some(*range), Some(When::Overflow)));
                            }
                        }
                        first.rendered.changes = changes;
                        first.rendered.alts = full.alts;
                        threaded = Some((spec.name, split));
                        first
                    }
                    (_, full) => {
                        let draw =
                            |cut: Option<&Cut>| appended(target, draw_body(spec.name, spent, cut, false), suffix);
                        let full = full.filter(|full| !full.uncapped);
                        let whole = full.map_or_else(|| draw(None), |full| appended(target, full, suffix));
                        request.fit_from(whole, &|| Outline::of_nodes(document, body, math), !removed, &draw)
                    }
                };
                if suffix.is_some() {
                    fitted.rendered.changes.push(request.change(Feature::TargetUrl, None, None));
                }
                if derived {
                    fitted.rendered.changes.retain(|change| change.feature == Feature::Text);
                }
                derived = true;
                fitted
            }
            (Source::Title { .. }, Some(title), _) => explicit_title(&request, title),
            (Source::Title { .. }, None, Some(heading)) => {
                request.fit(&|| Outline::of_nodes(document, std::slice::from_ref(heading), math), true, &|cut| {
                    draw_heading(heading, spec.name, spent, cut)
                })
            }
            (Source::Title { .. }, None, None) => {
                let source = first_line(
                    &render(target, facts, spec.name, spent, None, false, |context| {
                        (target.renderer.blocks)(context, body);
                    })
                    .unprefixed(),
                );
                let mut fitted = request.fit(&|| Outline::of_text(&source), true, &|cut| {
                    Rendered::text(cut.map_or_else(|| source.clone(), |cut| cut.replacement.clone()))
                });
                if let Some(heading) = leading {
                    fitted.rendered.changes.extend(draw_heading(heading, spec.name, spent, None).changes);
                }
                if removed {
                    let range = body.first().map(|node| node.range);
                    fitted.rendered.changes.push(request.change(Feature::Title, range, Some(When::TitleSource)));
                }
                fitted
            }
        };
        let Fitted { mut rendered, measurements: measured } = fitted;
        if spec.required && measure::blank(&rendered.text) {
            required.push(request.change(Feature::Title, None, Some(When::Empty)));
        }
        if Measurement::links_exceeded(&measured) {
            rendered.changes.push(request.change(Feature::Link, None, Some(When::Overflow)));
        }
        for ((unit, _), spent) in MARKERS.into_iter().zip(&mut spent) {
            if target.limits.iter().any(|limit| limit.field == spec.name && limit.unit == unit) {
                *spent += measure::markers(unit, &rendered.text).len();
            }
        }
        changes.append(&mut rendered.changes);
        alts.append(&mut rendered.alts);
        measurements.extend(measured);
        if let (Some(form), Source::Body) = (target.form.blocks(), spec.source) {
            let laid = media::layout(target, form, facts, body, &rendered.text, &rendered.anchors);
            blocks = laid.blocks;
            items = laid.attachments;
            changes.extend(laid.changes);
            measurements.extend(laid.measurements);
        }
        fields.push(Field {
            name: spec.name,
            links: utf16_ranges(&rendered.text, &rendered.links),
            entities: target
                .form
                .entities()
                .map_or_else(Vec::new, |form| entities::offsets(form, &rendered.text, &rendered.styled)),
            text: rendered.text,
        });
    }
    let blank = |text: &str| target.form.visible_blank(text, measure::blank);
    let later = threaded.as_ref().is_some_and(|(_, split)| split.parts.iter().any(|part| !blank(&part.rendered.text)));
    if facts.media_count == 0 && !later && fields.iter().all(|field| blank(&field.text)) {
        let end = children.last().map_or(0, |node| node.range.end);
        let (class, reason) = target.rule(Feature::Text).outcome(Some(When::Empty));
        let field =
            target.fields(facts).find(|spec| spec.source == Source::Body).map_or(FieldName::Text, |spec| spec.name);
        changes.push(Change {
            class,
            feature: Feature::Text,
            reason,
            field,
            range: Some(Range { start: 0, end }),
            truncated: None,
            slot: None,
        });
    } else {
        changes.append(&mut required);
    }
    if target.form.blocks().is_none() {
        let attached = media::attach(target, facts, children);
        items = attached.items;
        for (index, alt) in alts {
            items[index].alt = alt;
        }
        changes.extend(attached.changes);
        measurements.extend(attached.measurements);
    }
    crate::order::sort_by_key(&mut changes, |change| change.range.map_or(0, |range| u64::from(range.start) + 1));
    let mut references = vec![];
    media(children, &mut references);
    let mut parts = vec![];
    if let Some(threaded) = threaded {
        parts = threaded_parts(target, threaded, &fields, &measurements, &items);
        references.clone_from(&parts[0].media);
        items.clone_from(&parts[0].items);
    }
    Projection { fields, measurements, changes, media: references, blocks, items, parts }
}

fn anchored(document: &Document, full: &Rendered, mut thread: thread::Thread) -> Split {
    let children = document.children();
    let mut locator = Locator::new(document, full);
    let ranges: Vec<Range> = thread.spans.iter().map(|span| locator.range(span)).collect();
    for (index, at, mut change) in thread.truncated {
        let range = ranges[index];
        let (start, exact) = locator.start(at);
        change.range = Some(Range { start: start.clamp(range.start, range.end), end: range.end });
        if let Some(truncated) = &mut change.truncated {
            truncated.exact = exact;
        }
        thread.parts[0].rendered.changes.push(change);
    }
    let mut ids: Vec<Vec<String>> = vec![vec![]; thread.spans.len()];
    for anchor in &full.anchors {
        let Some(range) = anchor.node else { continue };
        let Ok(at) = children.binary_search_by_key(&range.start, |node| node.range.start) else { continue };
        let index = thread.spans.partition_point(|span| span.start <= anchor.at).saturating_sub(1);
        for id in children[at].media_ids().unwrap_or_default() {
            if !ids[index].contains(id) {
                ids[index].push(id.clone());
            }
        }
    }
    Split { parts: thread.parts, ranges, boundaries: thread.boundaries, media: ids }
}

fn threaded_parts(
    target: &'static Target,
    (name, split): (FieldName, Split),
    fields: &[Field],
    measurements: &[Measurement],
    items: &[Attachment],
) -> Vec<Part> {
    let placed: Vec<String> = split.media.iter().flatten().map(|id| format!("media:{id}")).collect();
    let mut parts = vec![];
    let first = fields.iter().find(|field| field.name == name).cloned();
    let mut rest = split.parts.into_iter();
    for (index, media) in split.media.into_iter().enumerate() {
        let (field, measured) = if index == 0 {
            let measured = measurements
                .iter()
                .filter(|measurement| {
                    measurement.field == name && measurement.item.is_none() && measurement.block.is_none()
                })
                .copied()
                .collect();
            (first.clone(), measured)
        } else {
            let Some(Fitted { rendered, measurements }) = rest.next() else { break };
            let field = Field {
                name,
                links: utf16_ranges(&rendered.text, &rendered.links),
                entities: target
                    .form
                    .entities()
                    .map_or_else(Vec::new, |form| entities::offsets(form, &rendered.text, &rendered.styled)),
                text: rendered.text,
            };
            (Some(field), measurements)
        };
        let attached = items
            .iter()
            .filter(|item| {
                media.iter().any(|id| item.reference == format!("media:{id}"))
                    || (index == 0 && !placed.contains(&item.reference))
            })
            .cloned()
            .collect();
        parts.push(Part {
            fields: field.into_iter().collect(),
            measurements: measured,
            media,
            items: attached,
            range: split.ranges[index],
            boundary: split.boundaries[index],
        });
    }
    parts
}

struct Locator<'a> {
    document: &'a Document,
    full: &'a Rendered,
    exact: Vec<Option<bool>>,
    cursor: (usize, usize, u32),
}

impl<'a> Locator<'a> {
    fn new(document: &'a Document, full: &'a Rendered) -> Self {
        Self { document, full, exact: vec![None; full.origins.len()], cursor: (usize::MAX, 0, 0) }
    }

    fn exact(&mut self, index: usize) -> bool {
        let (rendered, source) = &self.full.origins[index];
        *self.exact[index].get_or_insert_with(|| {
            self.full.text.get(rendered.clone()).is_some_and(|text| text == self.document.source(*source))
        })
    }

    fn offset(&mut self, index: usize, at: usize) -> u32 {
        let (rendered, source) = &self.full.origins[index];
        let (origin, byte, units) = self.cursor;
        let (from, counted) = if origin == index && byte <= at { (byte, units) } else { (rendered.start, 0) };
        let units = counted + self.full.text[from..at].encode_utf16().count() as u32;
        self.cursor = (index, at, units);
        source.start + units
    }

    fn start(&mut self, at: usize) -> (u32, bool) {
        let origins = &self.full.origins;
        let first = origins.partition_point(|(rendered, _)| rendered.end <= at);
        match origins.get(first) {
            Some(_) if origins[first].0.start <= at && self.exact(first) => (self.offset(first, at), true),
            Some(origin) if origin.0.start <= at => (origin.1.start, false),
            _ => (first.checked_sub(1).map_or(0, |previous| origins[previous].1.end), false),
        }
    }

    fn range(&mut self, span: &Span<usize>) -> Range {
        let origins = &self.full.origins;
        let (start, _) = self.start(span.start);
        let last = origins.partition_point(|(rendered, _)| rendered.start < span.end);
        let end = match last.checked_sub(1) {
            Some(index) if span.end <= origins[index].0.end => {
                if self.exact(index) {
                    self.offset(index, span.end)
                } else {
                    origins[index].1.end
                }
            }
            _ => origins.get(last).map_or(self.document.end().offset, |next| next.1.start),
        };
        Range { start, end: end.max(start) }
    }
}

fn explicit_title(request: &Request, title: &str) -> Fitted {
    let mut writer = Writer::new(request.target.form.escape());
    writer.push(title);
    let text = writer.finish().text;
    let mut changes = vec![];
    if text != title && request.target.form.escape() == Escape::Angle {
        changes.push(request.change(Feature::AngleBrackets, None, None));
    }
    let mut rendered = Rendered { changes, ..Rendered::text(text) };
    let limits = request.limits();
    let values = request.values(&limits, &rendered);
    if limits.iter().zip(&values).any(|(limit, value)| limit.maximum.is_some_and(|maximum| *value > maximum)) {
        rendered.changes.push(request.change(Feature::Title, None, None));
    }
    Fitted { measurements: request.measurements(&limits, &values), rendered }
}
