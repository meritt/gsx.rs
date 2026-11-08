pub mod blocks;
pub mod html;
pub mod markdown;
pub mod plain;

use crate::document::{Kind, Node};
use crate::project::Feature;

fn media(node: &Node) -> bool {
    node.media_ids().is_some()
}

fn wrapper(kind: &Kind) -> Option<Feature> {
    Some(match kind {
        Kind::Emphasis => Feature::Emphasis,
        Kind::Strong => Feature::Strong,
        Kind::Delete => Feature::Strikethrough,
        Kind::Underline => Feature::Underline,
        Kind::Highlight => Feature::Highlight,
        Kind::Spoiler => Feature::Spoiler,
        _ => return None,
    })
}

fn underlined(nodes: &[Node]) -> bool {
    nodes.iter().any(|node| matches!(node.kind, Kind::Underline) || underlined(&node.children))
}
