use crate::document::{Kind, Layout, Node, Range};
use crate::measure::{self, MediaKind, Placement};

use super::change::{Change, Feature, FieldName, Truncated};
use super::facts::{Facts, Format, MediaFact};
use super::table::{BlocksForm, Target, When};
use super::{Measurement, Unit, shortened};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub reference: String,
    pub alt: Option<String>,
    pub spoiler: bool,
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
    laid: Laid,
}

impl Layouter {
    fn change(&mut self, field: FieldName, range: Option<Range>, when: When) -> &mut Change {
        let (class, reason) = self.target.rule(Feature::Media).outcome(Some(when));
        self.laid.changes.push(Change { class, feature: Feature::Media, reason, field, range, truncated: None });
        self.laid.changes.last_mut().unwrap()
    }

    fn item(&mut self, fact: &MediaFact, range: Option<Range>, slot: (usize, usize)) -> Item {
        if !(fact.inline && self.form.formats.contains(&fact.format)) {
            self.change(FieldName::Text, range, When::Inline);
        }
        if fact.spoiler && !self.form.spoiler {
            self.change(FieldName::Text, range, When::Spoiler);
        }
        let alt = fact.alt.filter(|alt| !alt.trim().is_empty()).map(|alt| self.alt(alt, fact.format, range, slot));
        Item { reference: format!("media:{}", fact.id), alt, spoiler: fact.spoiler && self.form.spoiler }
    }

    fn alt(&mut self, alt: &str, format: Format, range: Option<Range>, (block, item): (usize, usize)) -> String {
        let context = measure::Context { placement: Placement::Post, media: kind(format) };
        let facts = measure::Facts::default();
        let limits: Vec<_> = self.target.applicable(FieldName::Alt, context).collect();
        let fits = |text: &str| limits.iter().all(|limit| !limit.measure(text, &facts).exceeds());
        let worst = limits.iter().map(|limit| limit.measure(alt, &facts)).find(measure::Measurement::exceeds);
        let kept = if worst.is_some() { shortened(alt, fits) } else { alt.to_owned() };
        for limit in &limits {
            let measured = limit.measure(&kept, &facts);
            self.laid.measurements.push(Measurement {
                field: FieldName::Alt,
                unit: measured.unit,
                value: measured.value as usize,
                maximum: measured.maximum.map(|maximum| maximum as usize),
                block: Some(block),
                item: Some(item),
            });
        }
        if let Some(worst) = worst {
            let (unit, maximum, value): (Unit, usize, usize) =
                (worst.unit, worst.maximum.unwrap_or(0) as usize, worst.value as usize);
            self.change(FieldName::Alt, range, When::Truncated).truncated =
                Some(Truncated { unit, maximum, value, exact: false });
        }
        kept
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
    let mut layouter = Layouter { target, form, laid: Laid { blocks: vec![], changes: vec![], measurements: vec![] } };
    for fact in facts.items.iter().skip(maximum) {
        let range = children.iter().find(|node| referenced(std::slice::from_ref(node), fact.id)).map(|node| node.range);
        layouter.change(FieldName::Text, range, When::Count);
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
