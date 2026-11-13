pub mod blocks;
pub mod html;
pub mod markdown;
pub mod plain;

use crate::document::{Kind, Node};
use crate::project::{Feature, Piece};

fn media(node: &Node) -> bool {
    node.media_ids().is_some()
}

fn math_block(lang: Option<&str>) -> bool {
    lang == Some("math")
}

fn spelled(piece: &str, kind: Piece) -> String {
    match kind {
        Piece::Whole => format!("${piece}$"),
        Piece::Kept if piece.is_empty() => String::new(),
        Piece::Kept => format!("${piece}"),
        Piece::Suffix => piece.to_owned(),
    }
}

fn dropped(piece: &str, kind: Piece) -> String {
    if kind == Piece::Suffix { piece.to_owned() } else { String::new() }
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

fn language(lang: Option<&str>) -> &str {
    lang.filter(|lang| {
        (1..=32).contains(&lang.len())
            && lang.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'+' | b'.' | b'-'))
    })
    .unwrap_or("")
}

fn label(nodes: &[Node], out: &mut String) {
    for node in nodes {
        match &node.kind {
            Kind::Text { value } | Kind::InlineCode { value } => out.push_str(value),
            Kind::Math { value } => out.push_str(&spelled(value, Piece::Whole)),
            Kind::Break => out.push('\n'),
            _ => label(&node.children, out),
        }
    }
}

fn closing_bidi(label: &str) -> String {
    let mut open = vec![];
    for char in label.chars() {
        match char {
            '\u{202A}' | '\u{202B}' | '\u{202D}' | '\u{202E}' => open.push('\u{202C}'),
            '\u{2066}'..='\u{2068}' => open.push('\u{2069}'),
            '\u{202C}' if open.last() == Some(&'\u{202C}') => {
                open.pop();
            }
            '\u{2069}' => {
                if let Some(isolate) = open.iter().rposition(|&close| close == '\u{2069}') {
                    open.truncate(isolate);
                }
            }
            '\n' | '\r' | '\u{1C}'..='\u{1E}' | '\u{85}' | '\u{2029}' => open.clear(),
            _ => {}
        }
    }
    open.into_iter().rev().collect()
}

fn underlined(nodes: &[Node]) -> bool {
    nodes.iter().any(|node| matches!(node.kind, Kind::Underline) || underlined(&node.children))
}
