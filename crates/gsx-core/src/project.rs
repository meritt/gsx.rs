mod change;
mod entities;
mod facts;
mod fit;
mod layout;
pub mod links;
mod media;
pub mod nfc;
mod outline;
mod table;

use std::collections::HashMap;
use std::ops::Range as Span;

use crate::document::{Document, Kind, Node, Provenance, Range};
use crate::measure::{self, Fact};

pub use crate::measure::{MediaKind, Placement, Unit};
pub use change::{Change, Class, Feature, FieldName, Slot, Truncated};
pub use entities::Entity;
pub use facts::{Facts, Format, MediaFact};
pub use fit::WINDOW;
pub(crate) use layout::blank;
pub use layout::{CELL, ELLIPSIS, RULE, Writer, marker, single_line};
pub use links::href;
pub use media::{Attachment, Block, Item, placed, references};
pub use table::{
    BlocksForm, Condition, Detection, EntityForm, Escape, FieldSpec, Form, HtmlBlocks, HtmlForm, LineStart,
    MarkdownForm, Media, Normalization, Offsets, Renderer, Rule, Search, Source, Structure, Style, Target, Truncation,
    When,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Projection {
    pub fields: Vec<Field>,
    pub measurements: Vec<Measurement>,
    pub changes: Vec<Change>,
    pub media: Vec<String>,
    pub blocks: Vec<Block>,
    pub items: Vec<Attachment>,
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
    positions: HashMap<&'a str, usize>,
    anchors: Vec<Anchor>,
    alts: Vec<(usize, Option<String>)>,
    urls: Vec<String>,
    url_cap: Option<usize>,
}

impl<'a> Context<'a> {
    pub fn items(&self) -> &[MediaFact<'_>] {
        self.items.unwrap_or_default()
    }

    pub fn position(&self, id: &str) -> Option<usize> {
        self.positions.get(id).copied()
    }

    pub fn admit(&mut self, href: &str) -> bool {
        let Some(cap) = self.url_cap else { return true };
        if self.urls.iter().any(|url| url == href) {
            return true;
        }
        let admitted = self.urls.len() < cap;
        if admitted {
            self.urls.push(href.to_owned());
        }
        admitted
    }

    pub fn published_alt(&mut self, item: usize, alt: Option<String>) {
        self.alts.push((item, alt));
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

    pub fn heading(&self) -> Option<Style> {
        self.open_form().and_then(|form| form.heading)
    }

    pub fn styled(&mut self, style: Style, feature: Feature, node: &Node, draw: impl FnOnce(&mut Self)) {
        let mark = self.writer.mark();
        self.open.push(style);
        draw(self);
        self.open.pop();
        if !self.open.contains(&style) {
            self.writer.style(style, None, mark, Some((feature, node.range)));
        }
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
        let ids = node.media_ids().unwrap_or_default();
        let listed = |id: &String| self.items.is_none() || self.positions.contains_key(id.as_str());
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
        }
    }

    pub fn leaf_with(&mut self, node: &Node, value: &str, map: impl Fn(&str, Piece) -> String) {
        self.writer.source(node.range);
        match self.cut_at(node) {
            Some(cut) => {
                let (kept, suffix) = cut.replacement.split_at(cut.kept);
                self.writer.push(&map(kept, Piece::Kept));
                self.push_suffix(&map(suffix, Piece::Suffix));
                self.done = true;
            }
            None => self.writer.push(&map(value, Piece::Whole)),
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

fn render(
    target: &'static Target,
    facts: &Facts,
    field: FieldName,
    cut: Option<&Cut>,
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
        positions: media::positions(facts.items.unwrap_or_default()),
        anchors: vec![],
        alts: vec![],
        urls: vec![],
        url_cap: target.form.entities().and_then(|form| form.links).map(|cap| {
            let reserved = facts.target_url.is_some() && target.handles(Feature::TargetUrl);
            cap - usize::from(reserved)
        }),
    };
    draw(&mut context);
    let mut written = std::mem::replace(&mut context.writer, Writer::new(Escape::None)).finish();
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
    let (text, links, prefixes) = match target.normalization {
        Normalization::None => (written.text, written.links, written.prefixes),
        Normalization::Nfc => {
            let (text, links) = normalized(&written.text, &written.links);
            (text, links, vec![])
        }
    };
    let mut rendered = Rendered {
        text,
        links,
        styled,
        changes: context.changes,
        anchors: context.anchors,
        alts: context.alts,
        prefixes,
    };
    cap_markers(target, field, &mut rendered);
    rendered
}

fn cap_markers(target: &'static Target, field: FieldName, rendered: &mut Rendered) {
    for (unit, feature) in [(Unit::Hashtags, Feature::Hashtag), (Unit::Mentions, Feature::Mention)] {
        let cap = target.limits.iter().find(|limit| limit.field == field && limit.unit == unit);
        let Some(maximum) = cap.and_then(|limit| limit.maximum(&measure::Facts::default())) else {
            continue;
        };
        let markers = measure::markers(unit, &rendered.text);
        let Some(dropped) = markers.get(maximum as usize..).filter(|dropped| !dropped.is_empty()) else {
            continue;
        };
        rendered.text = without_sigils(&rendered.text, dropped, &mut rendered.links);
        rendered.prefixes.clear();
        let (class, reason) = target.rule(feature).outcome(None);
        let truncated = Some(Truncated { unit, maximum, value: markers.len() as u32, exact: true });
        rendered.changes.push(Change { class, feature, reason, field, range: None, truncated, slot: None });
    }
}

fn without_sigils(text: &str, sigils: &[usize], links: &mut [Span<usize>]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    for &at in sigils {
        out.push_str(&text[cursor..at]);
        cursor = at + 1;
    }
    out.push_str(&text[cursor..]);
    let shift = |offset: usize| offset - sigils.iter().take_while(|&&at| at < offset).count();
    for link in links {
        *link = shift(link.start)..shift(link.end);
    }
    out
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

struct Fitted {
    rendered: Rendered,
    measurements: Vec<Measurement>,
}

#[derive(Clone, Copy, Debug)]
struct Bound {
    unit: Unit,
    maximum: Option<usize>,
}

struct Request<'a, 'b> {
    target: &'static Target,
    spec: &'a FieldSpec,
    facts: &'a Facts<'b>,
    options: &'a Options,
}

fn resolved(facts: &Facts) -> Vec<(Fact, u32)> {
    [
        (Fact::MaxCharacters, facts.max_characters),
        (Fact::CharactersReservedPerUrl, facts.characters_reserved_per_url),
        (Fact::CaptionLimit, facts.caption_limit),
        (Fact::DescriptionLimit, facts.description_limit),
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
        let text = self.target.form.visible(&rendered.text);
        let (urls, elided) = match unit {
            Unit::XWeighted | Unit::Links => (links::x_urls(&text), vec![]),
            Unit::MastodonWeighted => links::mastodon_entities(&text),
            _ => (vec![], vec![]),
        };
        let resolved = self.resolved();
        let facts = measure::Facts { urls: &urls, elided: &elided, resolved: &resolved };
        measure::measure(unit, &text, &facts) as usize
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
            .map(|limit| Bound { unit: limit.unit, maximum: limit.maximum(&facts).map(|maximum| maximum as usize) })
            .collect();
        if let Some(first) = limits.first_mut() {
            first.maximum = self.options.maximum.or(first.maximum);
        }
        limits
    }

    fn fits(&self, rendered: &Rendered) -> bool {
        let limits = self.limits();
        let values = self.values(&limits, rendered);
        limits.iter().zip(values).all(|(limit, value)| limit.maximum.is_none_or(|maximum| value <= maximum))
    }

    fn values(&self, limits: &[Bound], rendered: &Rendered) -> Vec<usize> {
        limits.iter().map(|limit| self.measure(limit.unit, rendered)).collect()
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
        outline: impl FnOnce() -> Outline<'o>,
        bounded: bool,
        draw: impl Fn(Option<&Cut>) -> Rendered,
    ) -> Fitted {
        let full = draw(None);
        let limits = self.limits();
        let values = self.values(&limits, &full);
        let fits = |values: &[usize]| {
            limits.iter().zip(values).all(|(limit, value)| {
                limit.unit.counts_markers() || limit.maximum.is_none_or(|maximum| *value <= maximum)
            })
        };
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
        let least = limits
            .iter()
            .filter(|limit| !limit.unit.counts_markers())
            .filter_map(|limit| limit.maximum)
            .min()
            .unwrap_or(maximum);
        let upper = bounded.then(|| outline.upper(least)).flatten().unwrap_or_else(|| outline.units() - 1);
        let load = |values: &[usize]| {
            limits
                .iter()
                .zip(values)
                .filter(|(limit, _)| !limit.unit.counts_markers())
                .filter_map(|(limit, &value)| limit.maximum.map(|maximum| value as f64 / maximum.max(1) as f64))
                .fold(0.0, f64::max)
        };
        let mut probes: HashMap<(u32, usize), (f64, Option<Probe>)> = HashMap::new();
        let mode = self.options.search.unwrap_or(self.target.search);
        let keep = fit::search(
            mode,
            upper,
            |keep| outline.clean(keep),
            |keep| {
                let cut = outline.cut(keep, self.spec.truncation);
                let (load, probe) = probes.entry(cut.key()).or_insert_with(|| {
                    let rendered = draw(Some(&cut));
                    let values = self.values(&limits, &rendered);
                    (load(&values), fits(&values).then_some((rendered, values)))
                });
                (probe.is_some(), *load)
            },
        );
        let Some(keep) = keep else {
            return overflow(full);
        };
        let cut = outline.cut(keep, self.spec.truncation);
        let (mut rendered, fitted) =
            probes.remove(&cut.key()).and_then(|(_, probe)| probe).expect("the kept cut was probed and fits");
        let (range, exact) = cut.dropped(outline.end());
        let mut change = self.change(Feature::Text, Some(range), None);
        change.truncated = Some(Truncated { unit, maximum: maximum as u32, value: value as u32, exact });
        rendered.changes.push(change);
        Fitted { rendered, measurements: self.measurements(&limits, &fitted) }
    }
}

fn appended(target: &Target, mut rendered: Rendered, url: Option<&str>) -> Rendered {
    if let Some(url) = url {
        if measure::blank(&target.form.visible(&rendered.text)) {
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
        Source::Title { fact, first_line_removed } => Some((fact, first_line_removed)),
        Source::Body => None,
    });
    let explicit = title.and_then(|(fact, _)| if fact { facts.title } else { None });
    let leading = children
        .first()
        .filter(|node| title.is_some() && explicit.is_none() && matches!(node.kind, Kind::Heading { .. }));
    let body = if leading.is_some() { &children[1..] } else { children };
    let draw_heading = |heading: &Node, field, cut: Option<&Cut>| {
        let mut rendered = render(target, facts, field, cut, |context| {
            context.report_when(Feature::Heading, Some(heading.range), Some(When::TitleSource));
            (target.renderer.inline)(context, &heading.children);
        });
        rendered.text = single_line(&rendered.text);
        rendered.links.clear();
        rendered.styled.clear();
        rendered
    };
    let titled = leading.filter(|heading| !draw_heading(heading, FieldName::Title, None).text.is_empty());
    let removed = title.is_some_and(|(_, removed)| removed) && titled.is_none() && explicit.is_none();
    let draw_body = |field, cut: Option<&Cut>| {
        let mut rendered = render(target, facts, field, cut, |context| (target.renderer.blocks)(context, body));
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
    let mut alts = vec![];
    let mut derived = false;
    for spec in target.fields(facts) {
        let request = Request { target, spec, facts, options };
        let fitted = match (spec.source, explicit, titled) {
            (Source::Body, ..) => {
                if !derived && matches!(target_url, Some(None)) {
                    changes.push(request.change(Feature::TargetUrl, None, Some(When::Refused)));
                }
                let suffix = target_url.as_ref().and_then(Option::as_deref).filter(|_| !derived).filter(|url| {
                    let blank = measure::blank(&target.form.visible(&draw_body(spec.name, None).text));
                    let least = if blank { (*url).to_owned() } else { format!("{ELLIPSIS}\n\n{url}") };
                    let fits = request.fits(&Rendered::text(least));
                    if !fits {
                        changes.push(request.change(Feature::TargetUrl, None, Some(When::Overflow)));
                    }
                    fits
                });
                let mut fitted = request.fit(
                    || Outline::of_nodes(document, body),
                    !removed,
                    |cut| appended(target, draw_body(spec.name, cut), suffix),
                );
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
            (Source::Title { .. }, None, Some(heading)) => request.fit(
                || Outline::of_nodes(document, std::slice::from_ref(heading)),
                true,
                |cut| draw_heading(heading, spec.name, cut),
            ),
            (Source::Title { .. }, None, None) => {
                let source = first_line(
                    &render(target, facts, spec.name, None, |context| (target.renderer.blocks)(context, body))
                        .unprefixed(),
                );
                let mut fitted = request.fit(
                    || Outline::of_text(&source),
                    true,
                    |cut| Rendered::text(cut.map_or_else(|| source.clone(), |cut| cut.replacement.clone())),
                );
                if let Some(heading) = leading {
                    fitted.rendered.changes.extend(draw_heading(heading, spec.name, None).changes);
                }
                fitted
            }
        };
        let Fitted { mut rendered, measurements: measured } = fitted;
        if spec.required && measure::blank(&rendered.text) {
            required.push(request.change(Feature::Title, None, Some(When::Empty)));
        }
        let over = |measured: &Measurement| measured.maximum.is_some_and(|maximum| measured.value > maximum);
        if measured.iter().any(|measured| measured.unit == Unit::Links && over(measured)) {
            rendered.changes.push(request.change(Feature::Link, None, Some(When::Overflow)));
        }
        changes.append(&mut rendered.changes);
        alts.append(&mut rendered.alts);
        measurements.extend(measured);
        if let (Some(form), Source::Body) = (target.form.blocks(), spec.source) {
            let laid = media::layout(target, form, facts, body, &rendered.text, &rendered.anchors);
            blocks = laid.blocks;
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
    if facts.media_count == 0 && fields.iter().all(|field| measure::blank(&target.form.visible(&field.text))) {
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
    let mut items = vec![];
    if target.form.blocks().is_none() {
        let attached = media::attach(target, facts, children);
        items = attached.items;
        for (index, alt) in alts {
            items[index].alt = alt;
        }
        changes.extend(attached.changes);
        measurements.extend(attached.measurements);
    }
    changes.sort_by_key(|change| change.range.map(|range| range.start));
    let mut references = vec![];
    media(children, &mut references);
    Projection { fields, measurements, changes, media: references, blocks, items }
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
