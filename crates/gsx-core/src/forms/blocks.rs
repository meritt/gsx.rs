use crate::document::{Kind, Node};
use crate::forms::markdown;
use crate::project::{Context, Renderer, placed, references};

pub const RENDERER: Renderer = Renderer { blocks, inline: markdown::inline };

pub fn blocks(context: &mut Context, nodes: &[Node]) {
    let placed_ids = references(nodes);
    let leading = context.items().iter().any(|item| !placed_ids.contains_key(item.id) && !dropped(context, item.id));
    let mut gap = if leading { 2 } else { 0 };
    let mut previous = leading.then_some(false);
    if leading {
        context.anchor(None);
    }
    let separator = context.target.form.blocks().is_some_and(|form| form.separator);
    for node in nodes {
        gap = gap.max(node.gap_before.unwrap_or(1));
        if separator && matches!(node.kind, Kind::ThematicBreak) {
            if context.done() {
                continue;
            }
            if previous == Some(true) {
                context.writer().newlines(gap);
            }
            context.anchor(Some(node));
            (previous, gap) = (Some(false), 0);
            continue;
        }
        if let Some(ids) = placed(node) {
            if ids.iter().all(|id| dropped(context, id)) {
                continue;
            }
            if previous == Some(true) && !context.done() {
                context.writer().newlines(gap);
            }
            context.anchor(Some(node));
            (previous, gap) = (Some(false), 0);
            continue;
        }
        if context.done() {
            continue;
        }
        if previous.is_some() {
            context.writer().newlines(gap);
        }
        markdown::block(context, node, 0);
        (previous, gap) = (Some(true), 0);
    }
}

fn dropped(context: &Context, id: &str) -> bool {
    let Some(form) = context.target.form.blocks() else { return true };
    let maximum = form.items.unwrap_or(usize::MAX);
    context.position(id).is_none_or(|index| index >= maximum || !form.shows(&context.items()[index]))
}
