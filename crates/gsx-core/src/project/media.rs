use std::collections::HashMap;

use crate::document::{Kind, Layout, Node, Range};
use crate::measure::{self, Fact, MediaKind, Placement};

use super::change::{Change, Feature, FieldName, Slot, Truncated};
use super::facts::{Facts, Format, MediaFact};
use super::table::{BlocksForm, Spoilers, Target, When};
use super::{Measurement, shortened};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub reference: String,
    pub alt: Option<String>,
    pub spoiler: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attachment {
    pub reference: String,
    pub alt: Option<String>,
    pub spoiler: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    Text(String),
    Media { items: Vec<Item>, layout: Option<Layout> },
    Separator,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Anchor {
    pub at: usize,
    pub node: Option<Range>,
    pub cut: bool,
}

pub(super) struct Laid {
    pub blocks: Vec<Block>,
    pub changes: Vec<Change>,
    pub measurements: Vec<Measurement>,
}

pub(super) struct Attached {
    pub items: Vec<Attachment>,
    pub changes: Vec<Change>,
    pub measurements: Vec<Measurement>,
}

struct Alt {
    text: Option<String>,
    measurements: Vec<Measurement>,
    change: Option<Change>,
}

fn fit_alt(
    target: &'static Target,
    alt: &str,
    format: Format,
    range: Option<Range>,
    resolved: &[(Fact, u32)],
    (block, item): (Option<usize>, usize),
) -> Alt {
    let context = measure::Context { placement: Placement::Post, media: kind(format) };
    let facts = measure::Facts { resolved, ..measure::Facts::default() };
    let limits: Vec<_> = target.applicable(FieldName::Alt, context).collect();
    let fits = |text: &str| limits.iter().all(|limit| !limit.measure(text, &facts).exceeds());
    let worst = limits.iter().map(|limit| limit.measure(alt, &facts)).find(measure::Measurement::exceeds);
    let text = match worst {
        Some(_) => Some(shortened(alt, fits)).filter(|kept| fits(kept)),
        None => Some(alt.to_owned()),
    };
    let mut measurements = vec![];
    if let Some(kept) = &text {
        for limit in &limits {
            let measured = limit.measure(kept, &facts);
            measurements.push(Measurement {
                field: FieldName::Alt,
                unit: measured.unit,
                value: measured.value as usize,
                maximum: measured.maximum.map(|maximum| maximum as usize),
                block,
                item: Some(item),
            });
        }
    }
    let change = worst.map(|worst| {
        let when = if text.is_some() { When::Truncated } else { When::Overflow };
        let (class, reason) = target.rule(Feature::Media).outcome(Some(when));
        let truncated =
            Truncated { unit: worst.unit, maximum: worst.maximum.unwrap_or(0), value: worst.value, exact: false };
        Change {
            class,
            feature: Feature::Media,
            reason,
            field: FieldName::Alt,
            range,
            truncated: Some(truncated),
            slot: Some(Slot { block: block.map(|block| block as u32), item: item as u32 }),
        }
    });
    Alt { text, measurements, change }
}

fn authored<'a>(fact: &MediaFact<'a>) -> Option<&'a str> {
    fact.alt.filter(|alt| !alt.trim().is_empty())
}

pub(super) fn attach(target: &'static Target, facts: &Facts, children: &[Node]) -> Attached {
    let resolved = super::resolved(facts);
    let mut attached = Attached { items: vec![], changes: vec![], measurements: vec![] };
    let ranges = references(children);
    let items = facts.items.unwrap_or_default();
    let spoilers = if facts.placement == Some(Placement::Story) { Spoilers::None } else { target.spoilers };
    let post = spoilers == Spoilers::Post && items.iter().any(|fact| fact.spoiler);
    for (index, fact) in items.iter().enumerate() {
        let range = ranges.get(fact.id).map(|&top| children[top].range);
        let alt = authored(fact).and_then(|alt| {
            let fitted = fit_alt(target, alt, fact.format, range, &resolved, (None, index));
            attached.measurements.extend(fitted.measurements);
            attached.changes.extend(fitted.change);
            fitted.text
        });
        let spoiler = match spoilers {
            Spoilers::None => false,
            Spoilers::Items(formats) => fact.spoiler && formats.contains(&fact.format),
            Spoilers::Post => post,
        };
        let when = match (fact.spoiler, spoiler) {
            (true, false) => Some(When::Spoiler),
            (false, true) => Some(When::Post),
            _ => None,
        };
        if let Some(when) = when {
            let (class, reason) = target.rule(Feature::Media).outcome(Some(when));
            let slot = Some(Slot { block: None, item: index as u32 });
            let field = FieldName::Text;
            attached.changes.push(Change {
                class,
                feature: Feature::Media,
                reason,
                field,
                range,
                truncated: None,
                slot,
            });
        }
        attached.items.push(Attachment { reference: format!("media:{}", fact.id), alt, spoiler });
    }
    attached
}

pub fn placed(node: &Node) -> Option<Vec<&str>> {
    node.media_ids().map(|ids| ids.iter().map(String::as_str).collect())
}

pub fn references(nodes: &[Node]) -> HashMap<&str, usize> {
    fn collect<'a>(node: &'a Node, top: usize, found: &mut HashMap<&'a str, usize>) {
        match node.media_ids() {
            Some(ids) => {
                for id in ids {
                    found.entry(id.as_str()).or_insert(top);
                }
            }
            None => {
                for child in &node.children {
                    collect(child, top, found);
                }
            }
        }
    }
    let mut found = HashMap::new();
    for (index, node) in nodes.iter().enumerate() {
        collect(node, index, &mut found);
    }
    found
}

pub fn positions<'a>(items: &[MediaFact<'a>]) -> HashMap<&'a str, usize> {
    let mut found = HashMap::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        found.entry(item.id).or_insert(index);
    }
    found
}

fn kind(format: Format) -> Option<MediaKind> {
    match format {
        Format::Image | Format::Gif => Some(MediaKind::Image),
        Format::Video => Some(MediaKind::Video),
        Format::Audio => Some(MediaKind::Audio),
    }
}

struct Layouter {
    target: &'static Target,
    form: &'static BlocksForm,
    resolved: Vec<(Fact, u32)>,
    laid: Laid,
}

impl Layouter {
    fn change(&mut self, field: FieldName, range: Option<Range>, when: When) {
        self.change_at(field, range, when, None);
    }

    fn change_at(&mut self, field: FieldName, range: Option<Range>, when: When, slot: Option<Slot>) {
        let (class, reason) = self.target.rule(Feature::Media).outcome(Some(when));
        let change = Change { class, feature: Feature::Media, reason, field, range, truncated: None, slot };
        let mut run = self.laid.changes.iter().rev().take_while(|reported| reported.range == range);
        if !run.any(|reported| *reported == change) {
            self.laid.changes.push(change);
        }
    }

    fn item(&mut self, fact: &MediaFact, range: Option<Range>, slot: (usize, usize)) -> Item {
        if !(fact.inline && self.form.formats.contains(&fact.format)) {
            self.change(FieldName::Text, range, When::Inline);
        }
        if fact.spoiler && !self.form.spoiler {
            let slot = Slot { block: Some(slot.0 as u32), item: slot.1 as u32 };
            self.change_at(FieldName::Text, range, When::Spoiler, Some(slot));
        }
        let alt = authored(fact).and_then(|alt| {
            let fitted = fit_alt(self.target, alt, fact.format, range, &self.resolved, (Some(slot.0), slot.1));
            self.laid.measurements.extend(fitted.measurements);
            self.laid.changes.extend(fitted.change);
            fitted.text
        });
        Item { reference: format!("media:{}", fact.id), alt, spoiler: fact.spoiler && self.form.spoiler }
    }

    fn text(&mut self, text: &str) {
        if !text.is_empty() {
            self.laid.blocks.push(Block::Text(text.to_owned()));
        }
    }

    fn media(&mut self, ids: &[&MediaFact], node: Option<&Node>, cut: bool) {
        let range = node.map(|node| node.range);
        let layout = node.and_then(|node| match &node.kind {
            Kind::MediaGroup { layout, .. } => Some(*layout),
            _ => None,
        });
        if ids.is_empty() {
            return;
        }
        if cut {
            self.change(FieldName::Text, range, When::Cut);
        }
        let grouped = layout.is_some_and(|layout| {
            ids.len() > 1 && self.form.group.is_some_and(|(native, maximum)| native == layout && ids.len() <= maximum)
        });
        if ids.len() > 1 && layout.is_some() && !grouped {
            self.change(FieldName::Text, range, When::Group);
        }
        let first = self.laid.blocks.len();
        let items: Vec<Item> = ids
            .iter()
            .enumerate()
            .map(|(index, fact)| self.item(fact, range, if grouped { (first, index) } else { (first + index, 0) }))
            .collect();
        if grouped {
            self.laid.blocks.push(Block::Media { items, layout });
        } else {
            for item in items {
                self.laid.blocks.push(Block::Media { items: vec![item], layout: None });
            }
        }
    }
}

pub(super) fn layout(
    target: &'static Target,
    form: &'static BlocksForm,
    facts: &Facts,
    children: &[Node],
    text: &str,
    anchors: &[Anchor],
) -> Laid {
    let maximum = form.items.unwrap_or(usize::MAX);
    let items = facts.items.unwrap_or_default();
    let mut layouter = Layouter {
        target,
        form,
        resolved: super::resolved(facts),
        laid: Laid { blocks: vec![], changes: vec![], measurements: vec![] },
    };
    let ranges = references(children);
    let indexes = positions(items);
    for fact in items.iter().skip(maximum) {
        layouter.change(FieldName::Text, ranges.get(fact.id).map(|&top| children[top].range), When::Count);
    }
    for node in children {
        if placed(node).is_some_and(|ids| ids.iter().any(|id| !indexes.contains_key(id))) {
            layouter.change(FieldName::Text, Some(node.range), When::Missing);
        }
    }
    let mut cursor = 0;
    for anchor in anchors {
        layouter.text(&text[cursor..anchor.at]);
        cursor = anchor.at;
        let found = anchor.node.and_then(|range| {
            let at = children.binary_search_by_key(&range.start, |node| node.range.start).ok()?;
            Some(&children[at]).filter(|node| node.range == range)
        });
        let Some(node) = found else {
            let unplaced: Vec<_> = items.iter().take(maximum).filter(|fact| !ranges.contains_key(fact.id)).collect();
            for fact in unplaced {
                layouter.media(&[fact], None, false);
            }
            continue;
        };
        if matches!(node.kind, Kind::ThematicBreak) {
            layouter.laid.blocks.push(Block::Separator);
            continue;
        }
        let mut kept = vec![];
        for id in placed(node).unwrap_or_default() {
            match indexes.get(id) {
                Some(&index) if index < maximum => kept.push(&items[index]),
                _ => {}
            }
        }
        layouter.media(&kept, Some(node), anchor.cut);
    }
    layouter.text(&text[cursor..]);
    if let Some(ceiling) = form.ceiling {
        let count: usize = layouter
            .laid
            .blocks
            .iter()
            .map(|block| match block {
                Block::Media { items, layout: Some(_) } if ceiling.per_item => 1 + items.len(),
                _ => 1,
            })
            .sum();
        if count > ceiling.blocks {
            layouter.change(FieldName::Text, None, When::Blocks);
        }
    }
    layouter.laid
}
