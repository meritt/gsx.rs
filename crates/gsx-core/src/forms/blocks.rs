use crate::document::{Kind, Node};
use crate::forms::markdown;
use crate::project::{Context, Renderer, placed, references};

pub const RENDERER: Renderer = Renderer { blocks, inline: markdown::inline };

pub fn blocks(context: &mut Context, nodes: &[Node]) {
    let placed_ids = references(nodes);
    if context.items().iter().any(|item| !placed_ids.contains_key(item.id) && !dropped(context, item.id)) {
        context.anchor(None);
    }
    let separator = context.target.form.blocks().is_some_and(|form| form.separator);
    let mut previous: Option<&Node> = None;
    let mut gap = 0;
    for node in nodes {
        gap = gap.max(node.gap_before.unwrap_or(1));
        if separator && matches!(node.kind, Kind::ThematicBreak) {
            if !context.done() {
                context.anchor(Some(node));
            }
            continue;
        }
        if let Some(ids) = placed(node) {
            if !ids.iter().all(|id| dropped(context, id)) {
                context.anchor(Some(node));
            }
            continue;
        }
        if context.done() {
            continue;
        }
        if let Some(previous) = previous {
            let gap = context.gap(Some(previous.range.end), node, gap);
            markdown::separate(context, previous, node, gap);
        }
        previous = Some(node);
        markdown::block(context, node, 0);
        gap = 0;
    }
}

fn dropped(context: &Context, id: &str) -> bool {
    let Some(form) = context.target.form.blocks() else { return true };
    let maximum = form.items.unwrap_or(usize::MAX);
    context.position(id).is_none_or(|index| index >= maximum || !form.shows(&context.items()[index]))
}
