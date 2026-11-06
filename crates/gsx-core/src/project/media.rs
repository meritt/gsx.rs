use crate::document::{Kind, Layout, Node, Range};
use crate::measure::{self, Fact, MediaKind, Placement};

use super::change::{Change, Feature, FieldName, Truncated};
use super::facts::{Facts, Format, MediaFact};
use super::table::{BlocksForm, Target, When};
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    Text(String),
    Media { items: Vec<Item>, layout: Option<Layout> },
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
        let truncated = Truncated {
            unit: worst.unit,
            maximum: worst.maximum.unwrap_or(0) as usize,
            value: worst.value as usize,
            exact: false,
        };
        Change { class, feature: Feature::Media, reason, field: FieldName::Alt, range, truncated: Some(truncated) }
    });
    Alt { text, measurements, change }
}

fn authored<'a>(fact: &MediaFact<'a>) -> Option<&'a str> {
    fact.alt.filter(|alt| !alt.trim().is_empty())
}

fn range_of(children: &[Node], id: &str) -> Option<Range> {
    children.iter().find(|node| referenced(std::slice::from_ref(node), id)).map(|node| node.range)
}

pub(super) fn attach(target: &'static Target, facts: &Facts, children: &[Node]) -> Attached {
    let resolved = super::resolved(facts);
    let mut attached = Attached { items: vec![], changes: vec![], measurements: vec![] };
    for (index, fact) in facts.items.iter().enumerate() {
        let alt = authored(fact).and_then(|alt| {
            let fitted = fit_alt(target, alt, fact.format, range_of(children, fact.id), &resolved, (None, index));
            attached.measurements.extend(fitted.measurements);
            attached.changes.extend(fitted.change);
            fitted.text
        });
        attached.items.push(Attachment { reference: format!("media:{}", fact.id), alt });
    }
    attached
}

pub fn placed(node: &Node) -> Option<Vec<&str>> {
    node.media_ids().map(|ids| ids.iter().map(String::as_str).collect())
}

pub fn referenced(nodes: &[Node], id: &str) -> bool {
    nodes.iter().any(|node| match node.media_ids() {
        Some(ids) => ids.iter().any(|placed| placed == id),
        None => referenced(&node.children, id),
    })
}

fn kind(format: Format) -> Option<MediaKind> {
    match format {
        Format::Image | Format::Gif => Some(MediaKind::Image),
        Format::Video => Some(MediaKind::Video),
        Format::Audio => None,
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
        let (class, reason) = self.target.rule(Feature::Media).outcome(Some(when));
        let change = Change { class, feature: Feature::Media, reason, field, range, truncated: None };
        if !self.laid.changes.contains(&change) {
            self.laid.changes.push(change);
        }
    }

    fn item(&mut self, fact: &MediaFact, range: Option<Range>, slot: (usize, usize)) -> Item {
        if !(fact.inline && self.form.formats.contains(&fact.format)) {
            self.change(FieldName::Text, range, When::Inline);
        }
        if fact.spoiler && !self.form.spoiler {
            self.change(FieldName::Text, range, When::Spoiler);
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
    let mut layouter = Layouter {
        target,
        form,
        resolved: super::resolved(facts),
        laid: Laid { blocks: vec![], changes: vec![], measurements: vec![] },
    };
    for fact in facts.items.iter().skip(maximum) {
        layouter.change(FieldName::Text, range_of(children, fact.id), When::Count);
    }
    let mut cursor = 0;
    for anchor in anchors {
        layouter.text(&text[cursor..anchor.at]);
        cursor = anchor.at;
        let Some(node) = anchor.node.and_then(|range| children.iter().find(|node| node.range == range)) else {
            let unplaced: Vec<_> =
                facts.items.iter().take(maximum).filter(|fact| !referenced(children, fact.id)).collect();
            for fact in unplaced {
                layouter.media(&[fact], None, false);
            }
            continue;
        };
        let mut kept = vec![];
        for id in placed(node).unwrap_or_default() {
            match facts.items.iter().position(|item| item.id == id) {
                Some(index) if index < maximum => kept.push(&facts.items[index]),
                Some(_) => {}
                None => {
                    layouter.change(FieldName::Text, Some(node.range), When::Inline);
                }
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
                Block::Media { items, layout: Some(_) } => 1 + items.len(),
                _ => 1,
            })
            .sum();
        if count > ceiling {
            layouter.change(FieldName::Text, None, When::Blocks);
        }
    }
    layouter.laid
}
