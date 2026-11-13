use std::cell::{OnceCell, RefCell};
use std::collections::{HashMap, HashSet};

use crate::document::tree::{self, Prefix, decode_string};
use crate::document::{
    Code, Details, Diagnostic, Kind, Layout, MEDIA_GROUP_MAX, MEDIA_GROUP_MIN, MEDIA_ID_MAX_BYTES, Media, Node,
    Provenance, Range, Scheme, Severity, TimeFormat, time,
};
use crate::source::LineIndex;
use crate::syntax::{self, whitespace};

struct Context<'a> {
    text: &'a str,
    lines: &'a LineIndex,
    prefixes: &'a [Prefix],
    diagnostics: RefCell<Vec<Diagnostic>>,
    placed: Vec<(usize, Kind)>,
    closings: OnceCell<Vec<[u32; 2]>>,
}

impl Context<'_> {
    fn range(&self, start: usize, end: usize) -> Range {
        Range { start: self.lines.utf16(start), end: self.lines.utf16(end) }
    }

    fn line(&self, byte: usize) -> u32 {
        self.lines.point(self.lines.utf16(byte)).line
    }

    fn diagnostic(&self, code: Code, severity: Severity, details: Details, start: usize, end: usize) {
        let range = self.range(start, end);
        self.diagnostics.borrow_mut().push(Diagnostic { code, severity, range, details });
    }

    fn refused(&self, code: Code, range: (usize, usize), start: usize, end: usize) -> Node {
        self.diagnostic(code, Severity::Error, Details::None, range.0, range.1);
        self.literal(start, end, None)
    }

    fn node(&self, kind: Kind, start: usize, end: usize) -> Node {
        Node {
            kind,
            range: self.range(start, end),
            provenance: Provenance::Authored,
            gap_before: None,
            children: vec![],
        }
    }

    fn literal(&self, start: usize, end: usize, value: Option<String>) -> Node {
        let value = value.unwrap_or_else(|| self.text[start..end].into());
        Node { provenance: Provenance::Literal, ..self.node(Kind::Text { value }, start, end) }
    }

    fn text(&self, value: String, start: usize, end: usize) -> Node {
        if let Some(offset) = time::reserved_offset(&self.text[start..end]) {
            let range = self.malformed(start + offset);
            self.diagnostic(Code::MarkdownMalformed, Severity::Error, Details::None, range.0, range.1);
            return self.literal(start, end, Some(value));
        }
        self.node(Kind::Text { value }, start, end)
    }

    fn malformed(&self, start: usize) -> (usize, usize) {
        let bytes = self.text.as_bytes();
        (start, time::reserved_end(bytes, self.closings.get_or_init(|| time::closing_ends(bytes)), start))
    }
}

pub fn is_media_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    (1..=MEDIA_ID_MAX_BYTES).contains(&bytes.len())
        && bytes[0].is_ascii_alphanumeric()
        && bytes[1..].iter().all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(byte))
}

fn parse_media(raw: &str) -> Option<Kind> {
    if let Some(id) = raw.strip_prefix("::media{id=\"").and_then(|rest| rest.strip_suffix("\"}"))
        && !id.contains('"')
        && is_media_id(id)
    {
        return Some(Kind::MediaReference { id: id.into() });
    }
    let rest = raw.strip_prefix("::media{ids=\"")?;
    let (ids, rest) = rest.split_once('"')?;
    let layout = match rest.strip_prefix(" layout=\"")?.strip_suffix("\"}")? {
        "collage" => Layout::Collage,
        "slideshow" => Layout::Slideshow,
        _ => return None,
    };
    let ids: Vec<String> = ids.split(',').map(Into::into).collect();
    if !(MEDIA_GROUP_MIN..=MEDIA_GROUP_MAX).contains(&ids.len()) || !ids.iter().all(|id| is_media_id(id)) {
        return None;
    }
    Some(Kind::MediaGroup { ids, layout })
}

struct Candidate<'n> {
    start: usize,
    end: usize,
    node: Option<(tree::Kind, &'n [tree::Node], usize)>,
}

fn collect_candidates<'n>(node: &'n tree::Node, bytes: &[u8], out: &mut Vec<Candidate<'n>>) {
    if matches!(node.kind, tree::Kind::Code | tree::Kind::InlineCode) {
        return;
    }
    let scan = |from: usize, to: usize, out: &mut Vec<Candidate>| {
        let mut at = from;
        while at < to {
            if bytes[at] == b':'
                && let Some(end) = syntax::media_candidate_end(bytes, at)
            {
                out.push(Candidate { start: at, end, node: None });
                at = end;
                continue;
            }
            at += 1;
        }
    };
    let mut cursor = node.start;
    for (index, child) in node.children.iter().enumerate() {
        scan(cursor, child.start, out);
        cursor = child.end;
        if child.kind == tree::Kind::MediaDirective {
            out.push(Candidate { start: child.start, end: child.end, node: Some((node.kind, &node.children, index)) });
        } else {
            collect_candidates(child, bytes, out);
        }
    }
    scan(cursor, node.end, out);
}

fn resolve_media(root: &tree::Node, media: &[Media], context: &mut Context) {
    let text = context.text;
    let bytes = text.as_bytes();
    let known: HashMap<&str, usize> = media.iter().enumerate().map(|(at, item)| (item.id, at)).collect();
    let covers: HashMap<&str, usize> =
        media.iter().enumerate().filter_map(|(at, item)| Some((item.cover?, at))).collect();
    let mut seen: HashSet<String> = HashSet::new();
    let paired: Vec<(usize, usize)> = pair_html(&root.children, text)
        .into_iter()
        .filter(|&(child, end)| end > child.end)
        .map(|(child, end)| (child.end, end))
        .collect();
    let mut candidates = vec![];
    collect_candidates(root, bytes, &mut candidates);
    crate::order::sort_by_key(&mut candidates, |candidate| candidate.start as u64);
    let (mut line_start, mut scanned) = (0, 0);
    for candidate in candidates {
        let (start, end) = (candidate.start, candidate.end);
        let Some(parsed) = parse_media(&text[start..end]) else {
            context.diagnostic(Code::MediaDirectiveInvalid, Severity::Error, Details::None, start, end);
            continue;
        };
        line_start = text[scanned..start].rfind('\n').map_or(line_start, |at| scanned + at + 1);
        scanned = start;
        let prefix_ok = start - line_start <= 3 && bytes[line_start..start].iter().all(|byte| *byte == b' ');
        let context_ok = match candidate.node {
            Some((tree::Kind::Root, siblings, index)) => {
                let is_media = |kind: tree::Kind| kind == tree::Kind::MediaDirective;
                let previous = index.checked_sub(1).map(|at| &siblings[at]);
                let next = siblings.get(index + 1);
                let previous_ok = previous
                    .is_none_or(|node| is_media(node.kind) || context.line(start) - context.line(node.end) >= 2);
                let next_ok =
                    next.is_none_or(|node| is_media(node.kind) || context.line(node.start) - context.line(end) >= 2);
                let after = paired.partition_point(|&(from, _)| from <= start);
                let unpaired = after == 0 || paired[after - 1].1 <= start;
                prefix_ok && previous_ok && next_ok && unpaired
            }
            _ => false,
        };
        if !context_ok {
            context.diagnostic(Code::MediaReferenceContextUnsupported, Severity::Error, Details::None, start, end);
            continue;
        }
        let (ids, group) = match &parsed {
            Kind::MediaGroup { ids, .. } => (ids.clone(), true),
            Kind::MediaReference { id } => (vec![id.clone()], false),
            _ => continue,
        };
        for (index, id) in ids.into_iter().enumerate() {
            let details =
                || Details::Media { id: id.clone(), index: group.then(|| u32::try_from(index).unwrap_or(u32::MAX)) };
            if !known.contains_key(id.as_str()) {
                context.diagnostic(Code::MediaReferenceUnknown, Severity::Error, details(), start, end);
            }
            if seen.contains(&id) {
                context.diagnostic(Code::MediaReferenceDuplicate, Severity::Error, details(), start, end);
            }
            if covers.contains_key(id.as_str()) {
                context.diagnostic(Code::MediaReferenceCover, Severity::Error, details(), start, end);
            }
            seen.insert(id);
        }
        match context.placed.last_mut() {
            Some((at, kind)) if *at == start => *kind = parsed,
            _ => context.placed.push((start, parsed)),
        }
    }
}

#[derive(Default)]
struct Ahead {
    label: Option<(usize, Option<usize>)>,
    reference: Option<(usize, Option<usize>)>,
}

fn next_byte(bytes: &[u8], byte: u8, from: usize, cache: &mut Option<(usize, Option<usize>)>) -> Option<usize> {
    if let Some((searched, found)) = *cache
        && searched <= from
        && found.is_none_or(|at| at >= from)
    {
        return found;
    }
    let found = bytes.get(from..).and_then(|rest| rest.iter().position(|&current| current == byte)).map(|at| at + from);
    *cache = Some((from, found));
    found
}

fn literal_end(raw: &str, index: usize, ahead: &mut Ahead) -> usize {
    let bytes = raw.as_bytes();
    match bytes[index] {
        b'\\' if bytes.get(index + 1).is_some_and(u8::is_ascii_punctuation) => index + 2,
        b'[' => {
            let Some(label_end) = next_byte(bytes, b']', index + 1, &mut ahead.label) else {
                return index;
            };
            if bytes.get(label_end + 1) != Some(&b'[') {
                return index;
            }
            next_byte(bytes, b']', label_end + 2, &mut ahead.reference).map_or(index, |at| at + 1)
        }
        marker @ (b'+' | b'=' | b'|') if bytes.get(index + 1) == Some(&marker) => index + 2,
        _ => index,
    }
}

fn decoded_text(value: &str) -> String {
    if !value.contains(['&', '\\']) {
        return value.into();
    }
    decode_string(value)
}

fn decoded_literal_text(value: &str) -> String {
    if !value.contains(['\0', '\n', '\r', '&']) {
        return value.into();
    }
    let mut escaped = String::with_capacity(value.len());
    for char in value.chars() {
        if matches!(char, '\\' | '`' | '*' | '_' | '[' | ']' | '<' | '>') {
            escaped.push('\\');
        }
        escaped.push(char);
    }
    decoded_text(&escaped)
}

fn unescaped(token: &str) -> String {
    let bytes = token.as_bytes();
    let mut out = String::with_capacity(token.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\\' && bytes.get(index + 1).is_some_and(u8::is_ascii_punctuation) {
            index += 1;
        }
        let char = token[index..].chars().next().expect("index is a char boundary");
        out.push(char);
        index += char.len_utf8();
    }
    out
}

struct Token {
    start: usize,
    end: usize,
    alignment: String,
}

fn lines_of(text: &str) -> Vec<(usize, usize)> {
    let mut out = vec![];
    let mut start = 0;
    for (at, byte) in text.bytes().enumerate() {
        if byte == b'\n' {
            out.push((start, at));
            start = at + 1;
        }
    }
    out.push((start, text.len()));
    out
}

fn aligned_tokens(raw: &str, value: &str, tokens: &[Token]) -> Option<Vec<(usize, usize)>> {
    let raw_lines = lines_of(raw);
    let value_lines = lines_of(value);
    if raw_lines.len() != value_lines.len() {
        return None;
    }
    let mut result = vec![];
    let mut token_index = 0;
    for (&(line_start, line_end), &(value_start, value_end)) in raw_lines.iter().zip(&value_lines) {
        let mut units: Vec<(Option<&Token>, String)> = vec![];
        let mut cursor = line_start;
        while token_index < tokens.len() && tokens[token_index].start < line_end {
            let token = &tokens[token_index];
            if token.end > line_end {
                return None;
            }
            if token.start > cursor {
                units.push((None, decoded_text(&raw[cursor..token.start])));
            }
            units.push((Some(token), token.alignment.clone()));
            cursor = token.end;
            token_index += 1;
        }
        if cursor < line_end {
            units.push((None, decoded_text(&raw[cursor..line_end])));
        }
        let line_value = &value[value_start..value_end];
        let visible: String = units.iter().map(|(_, unit)| unit.as_str()).collect();
        if !visible.ends_with(line_value) {
            return None;
        }
        let prefix = visible.len() - line_value.len();
        let mut offset = 0;
        for (token, unit) in &units {
            let end = offset + unit.len();
            if token.is_some() {
                if offset < prefix {
                    return None;
                }
                result.push((value_start + offset - prefix, value_start + end - prefix));
            }
            offset = end;
        }
    }
    (token_index == tokens.len()).then_some(result)
}

fn char_width(text: &str, index: usize) -> usize {
    text[index..].chars().next().map_or(1, char::len_utf8)
}

fn text_nodes(start: usize, end: usize, value: &str, context: &Context) -> Vec<Node> {
    let raw = &context.text[start..end];
    let mut tokens = vec![];
    let mut ahead = Ahead::default();
    let mut index = 0;
    while index < raw.len() {
        let close = literal_end(raw, index, &mut ahead);
        if close <= index {
            index += char_width(raw, index);
            continue;
        }
        tokens.push(Token {
            start: index,
            end: close,
            alignment: decoded_literal_text(&unescaped(&raw[index..close])),
        });
        index = close;
    }
    if tokens.is_empty() {
        return vec![context.text(value.into(), start, end)];
    }
    let Some(aligned) = aligned_tokens(raw, value, &tokens) else {
        return vec![context.literal(start, end, Some(value.into()))];
    };
    let mut out = vec![];
    let mut source_cursor = 0;
    let mut value_cursor = 0;
    for (token, (value_start, value_end)) in tokens.iter().zip(aligned) {
        if value_start > value_cursor {
            out.push(context.text(value[value_cursor..value_start].into(), start + source_cursor, start + token.start));
        }
        out.push(context.literal(start + token.start, start + token.end, Some(value[value_start..value_end].into())));
        source_cursor = token.end;
        value_cursor = value_end;
    }
    if value_cursor < value.len() {
        out.push(context.text(value[value_cursor..].into(), start + source_cursor, end));
    }
    out
}

#[derive(Clone)]
enum Item<'n> {
    Node(&'n tree::Node, usize),
    Text(usize, usize, String),
    Literal(usize, usize, String),
}

impl Item<'_> {
    fn start(&self) -> usize {
        match self {
            Item::Node(node, _) => node.start,
            Item::Text(start, ..) | Item::Literal(start, ..) => *start,
        }
    }

    fn end(&self) -> usize {
        match self {
            Item::Node(_, end) | Item::Text(_, end, _) | Item::Literal(_, end, _) => *end,
        }
    }

    fn text(&self) -> Option<&str> {
        match self {
            Item::Node(node, _) if node.kind == tree::Kind::Text => Some(&node.value),
            Item::Text(_, _, value) => Some(value),
            _ => None,
        }
    }
}

fn fragment(context: &Context, item: &Item, value: &str, start: usize, end: usize) -> Option<String> {
    let base = item.start();
    let raw = format!("{}\u{E000}", &context.text[base..item.end()]);
    let padded = format!("{value}\u{E000}");
    let marker = |at: usize| Token { start: at - base, end: at - base, alignment: String::new() };
    let aligned = aligned_tokens(&raw, &padded, &[marker(start), marker(end)])?;
    padded.get(aligned[0].0..aligned[1].0).map(Into::into)
}

fn literal_value(context: &Context, start: usize, end: usize) -> String {
    unprefixed(context, start, end).split('\n').map(decode_string).collect::<Vec<_>>().join("\n")
}

fn unprefixed(context: &Context, start: usize, end: usize) -> String {
    let prefixes = context.prefixes;
    let first = prefixes.partition_point(|prefix| prefix.end <= start);
    let mut joined = String::new();
    let mut cursor = start;
    for prefix in prefixes[first..].iter().take_while(|prefix| prefix.start < end) {
        if prefix.start > cursor {
            joined.push_str(&context.text[cursor..prefix.start]);
        }
        cursor = cursor.max(end.min(prefix.end));
        joined.extend(std::iter::repeat_n(' ', prefix.padding));
    }
    joined.push_str(&context.text[cursor..end]);
    joined
}

fn literalized<'n>(items: Vec<Item<'n>>, context: &Context) -> Vec<Item<'n>> {
    if items.len() < 2 {
        return items;
    }
    let base = items[0].start();
    let raw = &context.text[base..items[items.len() - 1].end()];
    let mut ahead = Ahead::default();
    let mut pending = items;
    let mut result = vec![];
    let mut unfit = HashSet::new();
    let mut index = 0;
    while index < pending.len() {
        let item = pending[index].clone();
        let Some(value) = item.text() else {
            result.push(item);
            index += 1;
            continue;
        };
        let (node_start, node_end) = (item.start(), item.end());
        let mut merged = false;
        let mut at = node_start - base;
        while at < node_end - base {
            let end = literal_end(raw, at, &mut ahead) + base;
            let next = (at + char_width(raw, at)).max(end - base);
            if end <= node_end {
                at = next;
                continue;
            }
            let last = index + 1 + pending[index + 1..].partition_point(|item| item.end() < end);
            let Some(tail) = pending.get(last) else {
                at = next;
                continue;
            };
            if (tail.text().is_none() && tail.end() != end) || unfit.contains(&(last, end)) {
                at = next;
                continue;
            }
            let start = at + base;
            let prefix = (start > node_start).then(|| fragment(context, &item, value, node_start, start));
            if matches!(prefix, Some(None)) {
                at = next;
                continue;
            }
            let suffix =
                (tail.end() > end).then(|| fragment(context, tail, tail.text().unwrap_or_default(), end, tail.end()));
            if matches!(suffix, Some(None)) {
                unfit.insert((last, end));
                at = next;
                continue;
            }
            if let Some(Some(prefix)) = prefix {
                result.push(Item::Text(node_start, start, prefix));
            }
            result.push(Item::Literal(start, end, literal_value(context, start, end)));
            if let Some(Some(suffix)) = suffix {
                pending[last] = Item::Text(end, pending[last].end(), suffix);
                index = last;
            } else {
                index = last + 1;
            }
            merged = true;
            break;
        }
        if !merged {
            result.push(item);
            index += 1;
        }
    }
    result
}

fn href(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    (matches!(parsed.scheme(), "http" | "https") && parsed.username().is_empty() && parsed.password().is_none())
        .then(|| parsed.into())
}

fn link_scheme(url: &str) -> Scheme {
    let bytes = url.as_bytes();
    match bytes.iter().position(|byte| *byte == b':') {
        Some(length)
            if length > 0
                && bytes[0].is_ascii_alphabetic()
                && bytes[1..length].iter().all(|byte| byte.is_ascii_alphanumeric() || b"+.-".contains(byte)) =>
        {
            match url[..length].to_ascii_lowercase().as_str() {
                "http" => Scheme::Http,
                "https" => Scheme::Https,
                _ => Scheme::Unsupported,
            }
        }
        _ => Scheme::Relative,
    }
}

fn explicit_inline(raw: &str) -> bool {
    raw.match_indices(']').any(|(at, _)| raw[at + 1..].trim_start_matches(whitespace).starts_with('('))
}

fn explicit_destination(raw: &str) -> Option<&str> {
    let start = raw.find("](")? + 2;
    let bytes = raw.as_bytes();
    if bytes.get(start) == Some(&b'<') {
        let mut index = start + 1;
        while index < bytes.len() {
            match bytes[index] {
                b'\\' => index += 1,
                b'>' => return Some(&raw[start + 1..index]),
                _ => {}
            }
            index += 1;
        }
        return None;
    }
    let mut depth = 0;
    let mut index = start;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 1,
            b'(' => depth += 1,
            b')' if depth == 0 => return Some(&raw[start..index]),
            b')' => depth -= 1,
            byte if byte.is_ascii_whitespace() && depth == 0 => return Some(&raw[start..index]),
            _ => {}
        }
        index += 1;
    }
    None
}

enum Converted {
    One(Node, bool),
    Many(Vec<Node>),
}

fn time_node(node: &tree::Node, context: &Context) -> Converted {
    let refused = |code| Converted::One(context.refused(code, (node.start, node.end), node.start, node.end), false);
    let (attribute, format) = match node.extra().attributes.as_slice() {
        [attribute] => (attribute, None),
        [attribute, format] | [format, attribute] if !format.shortcut && format.name == "format" => {
            (attribute, Some(format))
        }
        _ => return refused(Code::DatetimeInvalid),
    };
    if attribute.shortcut || !matches!(attribute.name.as_str(), "datetime" | "unix") {
        return refused(Code::DatetimeInvalid);
    }
    let format = match format {
        None => None,
        Some(format) => match format.value.as_deref().and_then(TimeFormat::parse) {
            Some(format) => Some(format),
            None => return refused(Code::DatetimeInvalid),
        },
    };
    let value = attribute.value.clone().unwrap_or_default();
    let kind = if attribute.name == "unix" {
        time::unix(&value).map(|unix| Kind::Datetime { datetime: None, unix, format })
    } else {
        time::absolute(&value).map(|unix| Kind::Datetime { datetime: Some(value), unix, format })
    };
    match kind {
        Ok(kind) => Converted::One(context.node(kind, node.start, node.end), true),
        Err(code) => refused(code),
    }
}

fn without_open_escape<'r>(raw: &'r str, text: &str, end: usize) -> &'r str {
    let trailing = raw.len() - raw.trim_end_matches('\\').len();
    if trailing % 2 == 1 && text.as_bytes().get(end).is_some_and(u8::is_ascii_punctuation) {
        &raw[..raw.len() - 1]
    } else {
        raw
    }
}

fn link_node(node: &tree::Node, context: &Context) -> Converted {
    let (start, end) = (node.start, node.end);
    let raw = &context.text[start..end];
    if !(raw.starts_with('[') && explicit_inline(raw)) {
        if let Some(inner) = raw.strip_prefix('<').and_then(|rest| rest.strip_suffix('>')) {
            let Some(url) = href(inner) else {
                let scheme = if inner.contains(':') { link_scheme(inner) } else { Scheme::Unsupported };
                context.diagnostic(Code::LinkLiteralized, Severity::Warning, Details::Scheme(scheme), start, end);
                return Converted::One(context.literal(start, end, None), false);
            };
            let mut link = context.node(Kind::Link { url, source: inner.into(), title: None }, start, end);
            link.children = vec![context.text(inner.into(), start + 1, end - 1)];
            return Converted::One(link, false);
        }
        return Converted::One(
            context.text(decode_string(without_open_escape(raw, context.text, end)), start, end),
            false,
        );
    }
    let Some(url) = href(&node.extra().url) else {
        let details = Details::Scheme(link_scheme(&node.extra().url));
        context.diagnostic(Code::LinkLiteralized, Severity::Warning, details, start, end);
        return Converted::One(context.literal(start, end, None), false);
    };
    let Some(destination) = explicit_destination(raw) else {
        return Converted::One(context.literal(start, end, None), false);
    };
    let kind = Kind::Link { url, source: destination.into(), title: node.extra().title.clone() };
    Converted::One(context.node(kind, start, end), true)
}

fn convert_node(node: &tree::Node, end: usize, context: &Context) -> Converted {
    use tree::Kind as T;
    let start = node.start;
    let kind = match node.kind {
        T::MediaDirective => {
            return Converted::One(
                match context.placed.binary_search_by_key(&start, |(at, _)| *at) {
                    Ok(at) => context.node(context.placed[at].1.clone(), start, end),
                    Err(_) => context.literal(start, end, None),
                },
                false,
            );
        }
        T::TextDirective if node.extra().name == "time" => {
            let raw = &context.text[start..node.end];
            let next = context.text.as_bytes().get(node.end);
            if !raw.starts_with("::time[") || !raw.contains(']') || next == Some(&b'{') {
                return Converted::One(
                    context.refused(Code::MarkdownMalformed, context.malformed(start), start, end),
                    false,
                );
            }
            return time_node(node, context);
        }
        T::Html
        | T::Image
        | T::ImageReference
        | T::Definition
        | T::LinkReference
        | T::FootnoteReference
        | T::FootnoteDefinition
        | T::TextDirective => return Converted::One(context.literal(start, end, None), false),
        T::Link => return link_node(node, context),
        T::Text => return Converted::Many(text_nodes(node.start, node.end, &node.value, context)),
        T::Math => Kind::Math { value: unprefixed(context, start + 1, node.end - 1) },
        T::InlineCode => Kind::InlineCode { value: node.value.replace('\n', " ") },
        T::Root | T::Paragraph => Kind::Paragraph,
        T::Heading => Kind::Heading { depth: node.depth },
        T::ThematicBreak => Kind::ThematicBreak,
        T::Blockquote => Kind::Blockquote,
        T::List => Kind::List { ordered: node.ordered, start: node.list_start, spread: node.spread },
        T::ListItem => Kind::ListItem { spread: node.spread, checked: node.checked },
        T::Code => {
            Kind::Code { lang: node.extra().lang.clone(), meta: node.extra().meta.clone(), value: node.value.clone() }
        }
        T::Table => Kind::Table { align: node.extra().align.clone() },
        T::TableRow => Kind::TableRow,
        T::TableCell => Kind::TableCell,
        T::Emphasis => Kind::Emphasis,
        T::Strong => Kind::Strong,
        T::Delete => Kind::Delete,
        T::Underline => Kind::Underline,
        T::Highlight => Kind::Highlight,
        T::Spoiler => Kind::Spoiler,
        T::Break => Kind::Break,
    };
    Converted::One(context.node(kind, start, end), node.is_parent())
}

fn html_tag(node: &tree::Node, text: &str, closing: bool) -> Option<String> {
    if node.kind != tree::Kind::Html {
        return None;
    }
    let inner = text[node.start..node.end].strip_prefix('<')?.strip_suffix('>')?;
    let inner = if closing { inner.strip_prefix('/')?.trim_end() } else { inner };
    let length = inner.find(|char: char| !(char.is_ascii_alphanumeric() || char == '-')).unwrap_or(inner.len());
    let name = &inner[..length];
    let ok = name.starts_with(|char: char| char.is_ascii_alphabetic())
        && if closing { length == inner.len() } else { !inner[length..].contains('>') && !inner.starts_with('/') };
    ok.then(|| name.to_ascii_lowercase())
}

fn pair_html<'n>(children: &'n [tree::Node], text: &str) -> Vec<(&'n tree::Node, usize)> {
    if !children.iter().any(|child| child.kind == tree::Kind::Html) {
        return children.iter().map(|child| (child, child.end)).collect();
    }
    let mut nearest: HashMap<String, usize> = HashMap::new();
    let mut close = vec![None; children.len()];
    for (index, child) in children.iter().enumerate().rev() {
        if let Some(name) = html_tag(child, text, true) {
            nearest.insert(name, index);
        } else if let Some(name) = html_tag(child, text, false) {
            close[index] = nearest.get(&name).copied();
        }
    }
    let mut out = vec![];
    let mut index = 0;
    while index < children.len() {
        let child = &children[index];
        if let Some(close) = close[index] {
            out.push((child, children[close].end));
            index = close + 1;
        } else {
            out.push((child, child.end));
            index += 1;
        }
    }
    out
}

fn append(parent: tree::Kind, out: &mut Vec<Node>, mut node: Node, lines: &LineIndex) {
    if matches!(parent, tree::Kind::Root | tree::Kind::Blockquote | tree::Kind::List | tree::Kind::ListItem) {
        let gap = out.last().map_or(0, |previous| {
            let paragraphs = previous.kind == Kind::Paragraph && node.kind == Kind::Paragraph;
            let distance = lines.point(node.range.start).line.saturating_sub(lines.point(previous.range.end).line);
            distance.max(if paragraphs { 2 } else { 1 })
        });
        node.gap_before = Some(gap);
    }
    out.push(node);
}

fn convert_children(parent: &tree::Node, context: &Context) -> Vec<Node> {
    let mut out = vec![];
    let items =
        pair_html(&parent.children, context.text).into_iter().map(|(child, end)| Item::Node(child, end)).collect();
    for item in literalized(items, context) {
        let (child, end) = match item {
            Item::Node(child, end) => (child, end),
            Item::Text(start, end, value) => {
                for node in text_nodes(start, end, &value, context) {
                    append(parent.kind, &mut out, node, context.lines);
                }
                continue;
            }
            Item::Literal(start, end, value) => {
                let node = context.literal(start, end, Some(value));
                append(parent.kind, &mut out, node, context.lines);
                continue;
            }
        };
        match convert_node(child, end, context) {
            Converted::Many(nodes) => {
                for node in nodes {
                    append(parent.kind, &mut out, node, context.lines);
                }
            }
            Converted::One(mut node, descend) => {
                if descend {
                    node.children = convert_children(child, context);
                }
                if matches!(node.kind, Kind::Datetime { .. }) && !node.children.iter().any(visible) {
                    let (start, end) = (child.start, child.end);
                    node = context.refused(Code::DatetimeInvalid, (start, end), start, end);
                }
                append(parent.kind, &mut out, node, context.lines);
            }
        }
    }
    out
}

fn visible(node: &Node) -> bool {
    let value = match &node.kind {
        Kind::Text { value } | Kind::InlineCode { value } | Kind::Math { value } | Kind::Code { value, .. } => {
            Some(value)
        }
        _ => None,
    };
    value.is_some_and(|value| value.chars().any(|char| !whitespace(char))) || node.children.iter().any(visible)
}

pub(super) fn convert(
    root: &tree::Node,
    prefixes: &[Prefix],
    text: &str,
    lines: &LineIndex,
    media: &[Media],
) -> (Vec<Node>, Vec<Diagnostic>) {
    let mut context =
        Context { text, lines, prefixes, diagnostics: RefCell::default(), placed: vec![], closings: OnceCell::new() };
    resolve_media(root, media, &mut context);
    let children = convert_children(root, &context);
    (children, context.diagnostics.into_inner())
}
