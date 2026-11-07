use super::{media, wrapper};
use crate::document::{Kind, Node};
use crate::project::links::{self, bluesky::visible};
use crate::project::{CELL, Class, Context, Feature, RULE, Renderer, When, href, marker};

pub const RENDERER: Renderer = Renderer { blocks, inline };

pub fn blocks(context: &mut Context, nodes: &[Node]) {
    let mut first = true;
    for node in nodes {
        if media(node) {
            context.report_media(node);
            continue;
        }
        if context.done() {
            continue;
        }
        if !first {
            context.writer().newlines(node.gap_before.unwrap_or(1));
        }
        first = false;
        block(context, node);
    }
}

fn block(context: &mut Context, node: &Node) {
    match &node.kind {
        Kind::Paragraph => inline(context, &node.children),
        Kind::Heading { .. } => {
            context.report(Feature::Heading, node.range);
            match context.heading() {
                Some(style) => context.styled(style, Feature::Heading, node, |context| inline(context, &node.children)),
                None => inline(context, &node.children),
            }
        }
        Kind::ThematicBreak => {
            context.report(Feature::ThematicBreak, node.range);
            context.writer().source(node.range);
            context.writer().push(RULE);
        }
        Kind::Code { value, .. } => {
            context.report(Feature::CodeBlock, node.range);
            context.leaf(node, value);
        }
        Kind::Blockquote => {
            context.report(Feature::Blockquote, node.range);
            context.writer().source(node.range);
            context.writer().open_quote();
            blocks(context, &node.children);
            context.writer().close();
        }
        Kind::List { ordered, start, .. } => {
            context.report(if *ordered { Feature::OrderedList } else { Feature::BulletList }, node.range);
            if node.children.iter().any(|item| item.children.len() > 1) {
                context.report(Feature::CompoundList, node.range);
            }
            if node.children.iter().any(|item| matches!(item.kind, Kind::ListItem { checked: Some(_), .. })) {
                context.report(Feature::TaskList, node.range);
            }
            for (index, item) in node.children.iter().enumerate() {
                if context.done() {
                    break;
                }
                if index > 0 {
                    context.writer().push("\n");
                }
                let checked = match item.kind {
                    Kind::ListItem { checked, .. } => checked,
                    _ => None,
                };
                context.writer().source(item.range);
                context.writer().open_item(marker(*ordered, *start, index, checked));
                blocks(context, &item.children);
                context.writer().close();
            }
        }
        Kind::Table { .. } => {
            context.report(Feature::Table, node.range);
            for (index, row) in node.children.iter().enumerate() {
                if context.done() {
                    break;
                }
                if index > 0 {
                    context.writer().push("\n");
                }
                for (index, cell) in row.children.iter().enumerate() {
                    if index > 0 {
                        context.writer().push(CELL);
                    }
                    if !context.done() {
                        inline(context, &cell.children);
                    }
                }
            }
        }
        _ => inline(context, std::slice::from_ref(node)),
    }
}

fn label(nodes: &[Node], out: &mut String) {
    for node in nodes {
        match &node.kind {
            Kind::Text { value } | Kind::InlineCode { value } => out.push_str(value),
            Kind::Break => out.push('\n'),
            _ => label(&node.children, out),
        }
    }
}

pub fn inline(context: &mut Context, nodes: &[Node]) {
    for node in nodes {
        if context.done() {
            break;
        }
        match &node.kind {
            Kind::Text { value } => context.leaf(node, value),
            Kind::InlineCode { value } => {
                if let Some(style) = context.native(Feature::InlineCode) {
                    context.styled(style, Feature::InlineCode, node, |context| context.leaf(node, value));
                } else {
                    context.report(Feature::InlineCode, node.range);
                    context.leaf(node, value);
                }
            }
            Kind::Break => context.writer().push("\n"),
            Kind::Link { url, title, .. } => {
                let href = href(url);
                let mut text = String::new();
                label(&node.children, &mut text);
                let bare = !context.contains_cut(node) && links::accepted(&text).is_some_and(|label| label == href);
                if context.declared(Feature::Link) {
                    if title.is_some() {
                        context.report(Feature::LinkTitle, node.range);
                    }
                    let mark = context.writer().mark();
                    context.within_link(|context| inline(context, &node.children));
                    match context.native(Feature::Link) {
                        Some(style) if visible(context.writer().since(mark)) => {
                            context.writer().style(style, Some(href), mark, Some((Feature::Link, node.range)));
                        }
                        Some(_) => context.report_when(Feature::Link, Some(node.range), Some(When::LabelInvisible)),
                        None => context.report(Feature::Link, node.range),
                    }
                    continue;
                }
                let admitted = context.admit(&href);
                if !admitted && !bare {
                    context.report_when(Feature::Link, Some(node.range), Some(When::Count));
                    if title.is_some() {
                        context.report(Feature::LinkTitle, node.range);
                    }
                    context.within_link(|context| inline(context, &node.children));
                    continue;
                }
                context.report_when(Feature::Link, Some(node.range), bare.then_some(When::LabelIsHref));
                if title.is_some() {
                    context.report(Feature::LinkTitle, node.range);
                }
                if bare {
                    context.writer().source(node.range);
                    context.writer().link(&href);
                    continue;
                }
                context.within_link(|context| inline(context, &node.children));
                context.writer().source(node.range);
                context.writer().push(" (");
                context.writer().link(&href);
                context.writer().push(")");
            }
            Kind::Datetime { .. } => {
                context.report(Feature::Datetime, node.range);
                inline(context, &node.children);
            }
            kind => match wrapper(kind) {
                Some(feature) if let Some(style) = context.native(feature) => {
                    context.styled(style, feature, node, |context| inline(context, &node.children));
                }
                Some(feature) => {
                    context.report(feature, node.range);
                    if context.class(feature) != Class::Omitted {
                        inline(context, &node.children);
                    }
                }
                None => inline(context, &node.children),
            },
        }
    }
}
