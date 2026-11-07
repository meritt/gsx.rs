use super::{media, wrapper};
use crate::document::{Align, Kind, Node, Provenance};
use crate::project::links::accepted;
use crate::project::{CELL, Class, Context, Feature, LineStart, MarkdownForm, Piece, Renderer, When, href};

pub const RENDERER: Renderer = Renderer { blocks, inline };

fn form(context: &Context) -> &'static MarkdownForm {
    context.target.form.markdown().expect("a markdown renderer runs on a markdown form")
}

pub fn blocks(context: &mut Context, nodes: &[Node]) {
    blocks_at(context, nodes, 0);
}

fn blocks_at(context: &mut Context, nodes: &[Node], depth: u32) {
    let mut first = true;
    for node in nodes {
        if context.done() {
            break;
        }
        if media(node) {
            context.report_media(node);
            continue;
        }
        if !first {
            context.writer().newlines(node.gap_before.unwrap_or(1));
        }
        first = false;
        block(context, node, depth);
    }
}

pub(crate) fn block(context: &mut Context, node: &Node, depth: u32) {
    let form = form(context);
    match &node.kind {
        Kind::Paragraph => {
            let mark = context.writer().mark();
            inline_at(context, &node.children, false);
            neutralise(context, mark);
        }
        Kind::Heading { .. } => {
            context.report(Feature::Heading, node.range);
            let delimiter = form.delimiter(Feature::Strong).unwrap_or("");
            let mark = context.writer().mark();
            context.writer().source(node.range);
            context.writer().push(delimiter);
            inline_at(context, &node.children, false);
            context.writer().push(delimiter);
            neutralise(context, mark);
        }
        Kind::ThematicBreak => {
            context.writer().source(node.range);
            if form.native(Feature::ThematicBreak) {
                context.writer().push("---");
            } else {
                context.report(Feature::ThematicBreak, node.range);
                let mut rule = String::new();
                escape(&mut rule, "---", form.escaped);
                context.writer().push(&rule);
            }
        }
        Kind::Code { lang, meta, value } => {
            let language = language(lang.as_deref());
            if meta.is_some() || lang.is_some() && language.is_empty() {
                context.report_when(Feature::CodeBlock, Some(node.range), Some(When::Info));
            }
            let fence = "`".repeat(3.max(longest_run(&code(form, context.rendered(node, value), false)) + 1));
            context.writer().source(node.range);
            context.writer().push(&fence);
            context.writer().push(language);
            context.writer().push("\n");
            context.leaf_with(node, value, |piece, _| code(form, piece, false));
            context.writer().push("\n");
            context.writer().push(&fence);
        }
        Kind::Blockquote => {
            if form.quote_depth.is_some_and(|maximum| depth >= maximum) {
                context.report_when(Feature::Blockquote, Some(node.range), Some(When::Nested));
                blocks_at(context, &node.children, depth + 1);
            } else {
                context.writer().source(node.range);
                context.writer().open_quote();
                blocks_at(context, &node.children, depth + 1);
                context.writer().close();
            }
        }
        Kind::List { ordered, start, .. } => list(context, node, *ordered, *start, depth),
        Kind::Table { align } => table(context, node, align),
        _ => inline_at(context, std::slice::from_ref(node), false),
    }
}

fn list(context: &mut Context, node: &Node, ordered: bool, start: Option<u32>, depth: u32) {
    let form = form(context);
    let feature = if ordered { Feature::OrderedList } else { Feature::BulletList };
    let checks = [
        (feature, true),
        (Feature::CompoundList, node.children.iter().any(|item| item.children.len() > 1)),
        (
            Feature::TaskList,
            node.children.iter().any(|item| matches!(item.kind, Kind::ListItem { checked: Some(_), .. })),
        ),
    ];
    for (feature, present) in checks {
        if present && !form.native(feature) {
            context.report(feature, node.range);
        }
    }
    for (index, item) in node.children.iter().enumerate() {
        if context.done() {
            break;
        }
        if index > 0 {
            context.writer().push("\n");
        }
        let marker =
            if ordered { format!("{}. ", u64::from(start.unwrap_or(1)) + index as u64) } else { "- ".to_owned() };
        context.writer().source(item.range);
        context.writer().open_item(marker);
        match item.kind {
            Kind::ListItem { checked: Some(true), .. } => context.writer().push("[x] "),
            Kind::ListItem { checked: Some(false), .. } => context.writer().push("[ ] "),
            _ => {}
        }
        blocks_at(context, &item.children, depth);
        context.writer().close();
    }
}

fn table(context: &mut Context, node: &Node, align: &[Align]) {
    let pipes = form(context).native(Feature::Table);
    if !pipes {
        context.report(Feature::Table, node.range);
    }
    let mark = context.writer().mark();
    for (index, row) in node.children.iter().enumerate() {
        if context.done() && index > 0 {
            break;
        }
        if index > 0 {
            context.writer().push("\n");
        }
        if pipes {
            context.writer().push("| ");
        }
        for (column, cell) in row.children.iter().enumerate() {
            if column > 0 {
                context.writer().push(CELL);
            }
            if !context.done() {
                inline_at(context, &cell.children, pipes);
            }
        }
        if pipes {
            context.writer().push(" |");
            if index == 0 {
                let divider: Vec<_> = align
                    .iter()
                    .map(|align| match align {
                        Align::Right => "---:",
                        Align::Center => ":---:",
                        Align::None | Align::Left => ":---",
                    })
                    .collect();
                context.writer().push(&format!("\n| {} |", divider.join(CELL)));
            }
        }
    }
    if !pipes {
        neutralise(context, mark);
    }
}

pub fn inline(context: &mut Context, nodes: &[Node]) {
    inline_at(context, nodes, false);
}

fn inline_at(context: &mut Context, nodes: &[Node], cell: bool) {
    let form = form(context);
    for node in nodes {
        if context.done() {
            break;
        }
        match &node.kind {
            Kind::Text { value } => {
                let literal = node.provenance == Provenance::Literal;
                context.leaf_with(node, value, |piece, _| text(form, piece, literal));
            }
            Kind::InlineCode { value } => {
                let shown = code(form, context.rendered(node, value), false);
                let fence = "`".repeat(longest_run(&shown).max(1) + 1);
                let padding = if padded(&shown) { " " } else { "" };
                context.writer().source(node.range);
                context.writer().push(&format!("{fence}{padding}"));
                context.leaf_with(node, value, |piece, _| code(form, piece, cell));
                context.writer().push(&format!("{padding}{fence}"));
            }
            Kind::Break => context.writer().push("\n"),
            Kind::Link { url, title, .. } => {
                if title.is_some() {
                    context.report(Feature::LinkTitle, node.range);
                }
                context.writer().source(node.range);
                context.writer().push("[");
                inline_at(context, &node.children, cell);
                context.writer().source(node.range);
                context.writer().push(&format!("]({})", raw(destination(&href(url)), cell)));
            }
            Kind::Datetime { unix, .. } => datetime(context, node, *unix, cell),
            kind => match wrapper(kind) {
                Some(feature) if let Some(delimiter) = form.delimiter(feature) => {
                    context.writer().source(node.range);
                    context.writer().push(delimiter);
                    inline_at(context, &node.children, cell);
                    context.writer().push(delimiter);
                }
                Some(feature) => {
                    context.report(feature, node.range);
                    if context.class(feature) != Class::Omitted {
                        inline_at(context, &node.children, cell);
                    }
                }
                None => inline_at(context, &node.children, cell),
            },
        }
    }
}

fn datetime(context: &mut Context, node: &Node, unix: i64, cell: bool) {
    let form = form(context);
    let template = form.datetime.replace("$unix", &unix.to_string());
    let Some((before, after)) = template.split_once("$label") else {
        if !node.children.is_empty() {
            context.report(Feature::Datetime, node.range);
        }
        context.writer().source(node.range);
        context.writer().push(&raw(template, cell));
        skip(context, &node.children);
        return;
    };
    let mut label = String::new();
    plain(&node.children, &mut label);
    if label.contains(|char| form.separators.contains(char)) {
        context.report(Feature::Datetime, node.range);
        inline_at(context, &node.children, cell);
        return;
    }
    context.writer().source(node.range);
    context.writer().push(&raw(before.to_owned(), cell));
    flat(context, &node.children, cell);
    context.writer().source(node.range);
    context.writer().push(&raw(after.to_owned(), cell));
}

fn skip(context: &mut Context, nodes: &[Node]) {
    for node in nodes {
        if !context.contains_cut(node) {
            continue;
        }
        match &node.kind {
            Kind::Text { value } | Kind::InlineCode { value } => {
                context.leaf_with(
                    node,
                    value,
                    |piece, kind| {
                        if kind == Piece::Suffix { piece.to_owned() } else { String::new() }
                    },
                );
            }
            _ => skip(context, &node.children),
        }
    }
}

fn plain(nodes: &[Node], out: &mut String) {
    for node in nodes {
        match &node.kind {
            Kind::Text { value } | Kind::InlineCode { value } => out.push_str(value),
            Kind::Break => out.push('\n'),
            Kind::Link { url, .. } => {
                plain(&node.children, out);
                out.push_str(&href(url));
            }
            _ => plain(&node.children, out),
        }
    }
}

fn flat(context: &mut Context, nodes: &[Node], cell: bool) {
    let form = form(context);
    for node in nodes {
        if context.done() {
            break;
        }
        match &node.kind {
            Kind::Text { value } | Kind::InlineCode { value } => {
                if matches!(node.kind, Kind::InlineCode { .. }) {
                    context.report(Feature::InlineCode, node.range);
                }
                context.leaf_with(node, value, |piece, _| code(form, piece, cell));
            }
            Kind::Break => context.writer().push("\n"),
            Kind::Link { url, .. } => {
                let href = href(url);
                let mut label = String::new();
                plain(&node.children, &mut label);
                let bare = !context.contains_cut(node) && accepted(&label).is_some_and(|label| label == href);
                context.report_when(Feature::Link, Some(node.range), bare.then_some(When::LabelIsHref));
                if bare {
                    context.writer().push(&code(form, &href, cell));
                } else {
                    flat(context, &node.children, cell);
                    context.writer().push(&format!(" ({})", code(form, &href, cell)));
                }
            }
            kind => {
                let datetime = matches!(kind, Kind::Datetime { .. }).then_some(Feature::Datetime);
                if let Some(feature) = wrapper(kind).or(datetime) {
                    context.report(feature, node.range);
                }
                flat(context, &node.children, cell);
            }
        }
    }
}

pub fn language(lang: Option<&str>) -> &str {
    lang.filter(|lang| {
        (1..=32).contains(&lang.len())
            && lang.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'+' | b'.' | b'-'))
    })
    .unwrap_or("")
}

fn longest_run(value: &str) -> usize {
    crate::serialize::longest_run(value, '`')
}

fn padded(value: &str) -> bool {
    value.starts_with('`')
        || value.ends_with('`')
        || value.starts_with(' ') && value.ends_with(' ') && !value.trim().is_empty()
}

pub fn destination(href: &str) -> String {
    let mut out = String::with_capacity(href.len());
    for char in href.chars() {
        if matches!(char, '\\' | '(' | ')') {
            out.push('\\');
        }
        out.push(char);
    }
    out
}

fn code(form: &MarkdownForm, value: &str, cell: bool) -> String {
    let value = if form.html { super::html::escape(value) } else { value.to_owned() };
    raw(value, cell)
}

fn raw(value: String, cell: bool) -> String {
    if !cell || !value.contains('|') {
        return value;
    }
    let mut out = String::with_capacity(value.len() + 4);
    let mut chars = value.chars().peekable();
    while let Some(char) = chars.next() {
        match char {
            '\\' if matches!(chars.peek(), Some('\\' | '|')) => {
                out.push('\\');
                out.push(chars.next().unwrap());
            }
            '|' => out.push_str("\\|"),
            char => out.push(char),
        }
    }
    out
}

fn escape(out: &mut String, value: &str, set: &str) {
    for char in value.chars() {
        if char.is_ascii() && set.contains(char) {
            out.push('\\');
        }
        out.push(char);
    }
}

fn spans(out: &mut String, line: &str, form: &MarkdownForm) {
    if !form.owned {
        escape(out, line, form.escaped);
        return;
    }
    let mut cursor = 0;
    for (start, end) in owned(line) {
        escape(out, &line[cursor..start], form.escaped);
        out.push_str(&line[start..end]);
        cursor = end;
    }
    escape(out, &line[cursor..], form.escaped);
}

fn text(form: &MarkdownForm, value: &str, literal: bool) -> String {
    let mut out = String::with_capacity(value.len() + value.len() / 4 + 2);
    for (index, line) in value.split('\n').enumerate() {
        if index > 0 {
            out.push('\n');
        }
        if literal {
            escape(&mut out, line, form.escaped);
        } else if form.neutralised.contains(&LineStart::Ordered) && escaped_ordered(line) {
            out.push('\\');
            spans(&mut out, &line[1..], form);
        } else {
            spans(&mut out, line, form);
        }
    }
    out
}

fn escaped_ordered(line: &str) -> bool {
    let Some(rest) = line.strip_prefix('\\') else { return false };
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    digits > 0 && rest[digits..].strip_prefix('.').and_then(|rest| rest.chars().next()).is_some_and(space)
}

fn neutralise(context: &mut Context, mark: usize) {
    let form = form(context);
    let writer = context.writer();
    let lines = writer.lines(mark);
    for line in lines.into_iter().rev() {
        let offset = line.start - mark;
        if let Some(at) = neutral(form, &writer.since(mark).as_bytes()[offset..offset + line.len()]) {
            writer.insert(line.start + at, "\\");
        }
    }
}

fn neutral(form: &MarkdownForm, line: &[u8]) -> Option<usize> {
    form.neutralised.iter().find_map(|rule| match rule {
        LineStart::Heading => {
            let hashes = line.iter().take_while(|&&byte| byte == b'#').count();
            ((1..=6).contains(&hashes) && matches!(line.get(hashes), Some(b' ' | b'\t') | None)).then_some(0)
        }
        LineStart::Ordered => {
            let digits = line.iter().take_while(|byte| byte.is_ascii_digit()).count();
            ((1..=9).contains(&digits)
                && line.get(digits) == Some(&b'.')
                && matches!(line.get(digits + 1), Some(b' ' | b'\t')))
            .then_some(digits)
        }
        LineStart::Setext => {
            let signs = line.iter().take_while(|&&byte| byte == b'=').count();
            (signs > 0 && line[signs..].iter().all(|&byte| byte == b' ' || byte == b'\t')).then_some(0)
        }
    })
}

fn space(char: char) -> bool {
    (char.is_whitespace() && char != '\u{85}') || char == '\u{feff}'
}

fn word(char: char) -> bool {
    char.is_alphabetic() || char.is_numeric()
}

pub fn owned(line: &str) -> Vec<(usize, usize)> {
    let mut found = vec![];
    let mail = line.contains('@');
    let mut at = 0;
    while at < line.len() {
        let rest = &line[at..];
        match url(rest).or_else(|| mail.then(|| email(rest)).flatten()).or_else(|| name(rest)) {
            Some(length) => {
                let kept = rest[..length].trim_end_matches('\\').len();
                if kept > 0 {
                    found.push((at, at + kept));
                }
                at += length;
            }
            None => at += rest.chars().next().map_or(1, char::len_utf8),
        }
    }
    found
}

fn url(rest: &str) -> Option<usize> {
    let scheme = ["https://", "http://"].into_iter().find(|scheme| rest.starts_with(scheme))?.len();
    let body: usize = rest[scheme..]
        .chars()
        .take_while(|&char| !space(char) && !matches!(char, '<' | '>' | '(' | ')'))
        .map(char::len_utf8)
        .sum();
    (body > 0).then_some(scheme + body)
}

fn email(rest: &str) -> Option<usize> {
    let mut local = 0;
    let mut count = 0;
    for char in rest.chars() {
        if !(word(char) || matches!(char, '.' | '_' | '%' | '+' | '-')) {
            break;
        }
        count += 1;
        if count > 64 {
            return None;
        }
        local += char.len_utf8();
    }
    if count == 0 || !rest[local..].starts_with('@') {
        return None;
    }
    let domain = local + 1;
    let mut ends = vec![0];
    for char in rest[domain..].chars() {
        if ends.len() > 253 || !(word(char) || matches!(char, '.' | '-')) {
            break;
        }
        ends.push(ends[ends.len() - 1] + char.len_utf8());
    }
    for length in (1..ends.len()).rev() {
        let dot = domain + ends[length];
        if !rest[dot..].starts_with('.') {
            continue;
        }
        let letters: Vec<_> = rest[dot + 1..].chars().take_while(|char| char.is_alphabetic()).take(63).collect();
        if letters.len() >= 2 {
            return Some(dot + 1 + letters.iter().map(|char| char.len_utf8()).sum::<usize>());
        }
    }
    None
}

fn name(rest: &str) -> Option<usize> {
    let sigil = rest.chars().next().filter(|char| matches!(char, '@' | '#' | '$'))?;
    let body: usize = rest[1..].chars().take_while(|&char| word(char) || char == '_').map(char::len_utf8).sum();
    (body > 0).then_some(sigil.len_utf8() + body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owned_spans_cover_links_mentions_and_tags() {
        fn spans(line: &str) -> Vec<&str> {
            owned(line).into_iter().map(|(start, end)| &line[start..end]).collect()
        }
        assert_eq!(spans("see https://a.b/c_d) and"), ["https://a.b/c_d"]);
        assert_eq!(spans("mail a_b@ex-ample.co.uk!"), ["a_b@ex-ample.co.uk"]);
        assert_eq!(spans("@user_1 #tag $x"), ["@user_1", "#tag", "$x"]);
        assert_eq!(spans("@\\ x\\"), Vec::<&str>::new());
        assert_eq!(spans("#a\\\\"), ["#a"]);
        assert_eq!(spans("x@y"), ["@y"]);
    }
}
