use crate::syntax::{self, Align, Event, Kind as EventKind, Name};

const TAB_SIZE: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Root,
    Paragraph,
    Heading,
    ThematicBreak,
    Blockquote,
    List,
    ListItem,
    Code,
    Html,
    Table,
    TableRow,
    TableCell,
    Text,
    Emphasis,
    Strong,
    Delete,
    Underline,
    Highlight,
    Spoiler,
    InlineCode,
    Break,
    Link,
    Image,
    LinkReference,
    ImageReference,
    Definition,
    FootnoteReference,
    FootnoteDefinition,
    TextDirective,
    MediaDirective,
    Math,
}

#[derive(Clone, Debug)]
pub(super) struct Attribute {
    pub shortcut: bool,
    pub name: String,
    pub value: Option<String>,
}

#[derive(Clone, Debug)]
pub(super) struct Node {
    pub kind: Kind,
    pub start: usize,
    pub end: usize,
    pub children: Vec<Node>,
    pub value: String,
    pub depth: u8,
    pub ordered: bool,
    pub list_start: Option<u32>,
    pub checked: Option<bool>,
    pub spread: bool,
    extra: Option<Box<Extra>>,
}

#[derive(Clone, Debug, Default)]
pub(super) struct Extra {
    pub lang: Option<String>,
    pub meta: Option<String>,
    pub url: String,
    pub title: Option<String>,
    pub align: Vec<Align>,
    pub name: String,
    pub attributes: Vec<Attribute>,
}

static NO_EXTRA: Extra = Extra {
    lang: None,
    meta: None,
    url: String::new(),
    title: None,
    align: Vec::new(),
    name: String::new(),
    attributes: Vec::new(),
};

impl Node {
    fn new(kind: Kind, start: usize, end: usize) -> Self {
        Node {
            kind,
            start,
            end,
            children: vec![],
            value: String::new(),
            depth: 0,
            ordered: false,
            list_start: None,
            checked: None,
            spread: false,
            extra: None,
        }
    }

    pub fn extra(&self) -> &Extra {
        self.extra.as_deref().unwrap_or(&NO_EXTRA)
    }

    fn extra_mut(&mut self) -> &mut Extra {
        self.extra.get_or_insert_default()
    }

    pub fn is_parent(&self) -> bool {
        !matches!(
            self.kind,
            Kind::ThematicBreak
                | Kind::Code
                | Kind::Html
                | Kind::Text
                | Kind::InlineCode
                | Kind::Break
                | Kind::Image
                | Kind::ImageReference
                | Kind::Definition
                | Kind::FootnoteReference
                | Kind::MediaDirective
        )
    }
}

struct Builder<'a> {
    events: &'a [Event],
    bytes: &'a [u8],
    exits: Vec<usize>,
}

fn ignored(name: Name) -> bool {
    matches!(
        name,
        Name::BlockQuotePrefix
            | Name::ListItemPrefix
            | Name::SpaceOrTab
            | Name::BlankLineEnding
            | Name::HeadingAtxSequence
            | Name::HeadingSetextUnderline
            | Name::GfmTableDelimiterRow
            | Name::GfmTableCellDivider
            | Name::GfmTaskListItemCheck
            | Name::GfmFootnoteDefinitionPrefix
            | Name::EmphasisSequence
            | Name::StrongSequence
            | Name::GfmStrikethroughSequence
            | Name::GsxPairedSequence
            | Name::ThematicBreakSequence
    )
}

impl<'a> Builder<'a> {
    fn raw(&self, start: usize, end: usize) -> &'a str {
        std::str::from_utf8(&self.bytes[start..end]).expect("event boundaries are char boundaries")
    }

    fn start(&self, index: usize) -> usize {
        self.events[index].point.index
    }

    fn end(&self, index: usize) -> usize {
        self.events[self.exits[index]].point.index
    }

    fn underline_like(&self, start: usize) -> bool {
        let line = self.bytes[start..].split(|byte| *byte == b'\n').next().unwrap_or_default();
        let line = line.trim_ascii_end();
        !line.is_empty() && (line.iter().all(|byte| *byte == b'=') || line.iter().all(|byte| *byte == b'-'))
    }

    fn content_end(&self, exit: usize) -> usize {
        let mut index = exit - 1;
        while index > 1 && self.events[index].kind == EventKind::Exit {
            match self.events[index].name {
                Name::LineEnding | Name::BlankLineEnding => index -= 2,
                Name::SpaceOrTab if self.events[index + 1].name == Name::BlankLineEnding => index -= 2,
                Name::ListItem | Name::ListOrdered | Name::ListUnordered | Name::BlockQuote => index -= 1,
                _ => break,
            }
        }
        self.events[index].point.index
    }

    fn find(&self, from: usize, to: usize, name: Name) -> Option<usize> {
        (from + 1..to).find(|&i| self.events[i].kind == EventKind::Enter && self.events[i].name == name)
    }

    fn direct(&self, index: usize, name: Name) -> Option<usize> {
        let mut at = index + 1;
        while at < self.exits[index] {
            if self.events[at].name == name {
                return Some(at);
            }
            at = self.exits[at] + 1;
        }
        None
    }

    fn text(&self, index: usize) -> String {
        self.raw(self.start(index), self.end(index)).into()
    }

    fn chunk(&self, index: usize, out: &mut String) {
        let (start, end) = (&self.events[index].point, &self.events[self.exits[index]].point);
        let (mut from, mut to) = (start.index, end.index);
        if start.vs > 0 {
            out.extend(std::iter::repeat_n(' ', TAB_SIZE - start.vs));
            from += 1;
        }
        if end.vs > 0 {
            to += 1;
        }
        out.push_str(self.raw(from, to));
        if end.vs > 0 {
            out.extend(std::iter::repeat_n(' ', TAB_SIZE - end.vs));
        }
    }

    fn children(&self, from: usize, to: usize, phrasing: bool) -> Vec<Node> {
        let mut out: Vec<Node> = vec![];
        let mut content = None;
        let mut index = from;
        while index < to {
            let event = &self.events[index];
            if event.kind == EventKind::Exit {
                index += 1;
                continue;
            }
            if !matches!(event.name, Name::LineEnding | Name::SpaceOrTab | Name::Definition | Name::HeadingSetext) {
                content = None;
            }
            let exit = self.exits[index];
            let (start, end) = (self.start(index), self.end(index));
            match &event.name {
                name if ignored(*name) => {}
                Name::Data if phrasing => push_text(&mut out, start, end, self.raw(start, end)),
                Name::GsxPairedLiteral => push_text(&mut out, start, end, self.raw(start, end)),
                Name::CharacterEscape => push_text(&mut out, start, end, self.raw(start + 1, end)),
                Name::CharacterReference => push_text(&mut out, start, end, &self.reference(index, exit)),
                Name::LineEnding if phrasing => push_text(&mut out, start, end, "\n"),
                Name::LineEnding | Name::Data => {}
                Name::HardBreakEscape | Name::HardBreakTrailing => {
                    let mut node = Node::new(Kind::Break, start, end);
                    let next = exit + 1;
                    if next < to && self.events[next].name == Name::LineEnding {
                        node.end = self.end(next);
                        index = self.exits[next] + 1;
                        out.push(node);
                        continue;
                    }
                    out.push(node);
                }
                _ => {
                    if let Some(mut node) = self.node(index, exit) {
                        if event.name == Name::HeadingSetext
                            && let Some(start) = content
                            && !self.underline_like(node.start)
                        {
                            node.start = start;
                        }
                        content = if event.name == Name::Definition { content.or(Some(node.start)) } else { None };
                        out.push(node);
                    } else {
                        out.extend(self.children(index + 1, exit, phrasing));
                    }
                }
            }
            index = exit + 1;
        }
        out
    }

    fn reference(&self, index: usize, exit: usize) -> String {
        let marker = if self.find(index, exit, Name::CharacterReferenceMarkerHexadecimal).is_some() {
            b'x'
        } else if self.find(index, exit, Name::CharacterReferenceMarkerNumeric).is_some() {
            b'#'
        } else {
            b'&'
        };
        let value = self.find(index, exit, Name::CharacterReferenceValue).map(|i| self.text(i)).unwrap_or_default();
        syntax::decode_character_reference(&value, marker, true).unwrap_or_else(|| self.text(index))
    }

    fn phrasing_in(&self, from: usize, to: usize, name: Name) -> Vec<Node> {
        self.find(from, to, name).map_or(vec![], |i| self.children(i + 1, self.exits[i], true))
    }

    fn keep_label_space(&self, children: &mut Vec<Node>, start: usize, end: usize) {
        let space = |from: usize, to: usize| {
            let mut node = Node::new(Kind::Text, from, to);
            node.value = self.raw(from, to).into();
            node
        };
        let lead = children.first().map_or(end, |child| child.start);
        if lead > start {
            match children.first_mut().filter(|child| child.kind == Kind::Text) {
                Some(first) => {
                    first.value.insert_str(0, self.raw(start, lead));
                    first.start = start;
                }
                None => children.insert(0, space(start, lead)),
            }
        }
        let trail = children.last().map_or(end, |child| child.end);
        if trail < end {
            match children.last_mut().filter(|child| child.kind == Kind::Text) {
                Some(last) => {
                    last.value.push_str(self.raw(trail, end));
                    last.end = end;
                }
                None => children.push(space(trail, end)),
            }
        }
    }

    fn flow_start(&self, index: usize, exit: usize) -> usize {
        let name = &self.events[index].name;
        if *name != Name::CodeIndented {
            let leaf = (index + 1..exit).find(|&i| {
                self.events[i].kind == EventKind::Enter
                    && (self.events[i + 1].kind == EventKind::Exit || self.events[i].name == Name::GsxMath)
                    && !matches!(self.events[i].name, Name::SpaceOrTab | Name::LineEnding | Name::BlankLineEnding)
            });
            if let Some(leaf) = leaf {
                return self.rounded(leaf);
            }
        }
        self.rounded(index)
    }

    fn rounded(&self, index: usize) -> usize {
        let point = &self.events[index].point;
        point.index + usize::from(point.vs > 0)
    }

    fn node(&self, index: usize, exit: usize) -> Option<Node> {
        let flow = matches!(
            self.events[index].name,
            Name::Paragraph
                | Name::HeadingAtx
                | Name::HeadingSetext
                | Name::ThematicBreak
                | Name::BlockQuote
                | Name::ListOrdered
                | Name::ListUnordered
                | Name::ListItem
                | Name::CodeFenced
                | Name::CodeIndented
                | Name::HtmlFlow
                | Name::GfmTable
                | Name::Definition
                | Name::GfmFootnoteDefinition
        );
        let start = if flow { self.flow_start(index, exit) } else { self.start(index) };
        let end = self.end(index);
        let node = |kind| Node::new(kind, start, end);
        let built = match self.events[index].name {
            Name::Paragraph => {
                let mut n = node(Kind::Paragraph);
                n.children = self.children(index + 1, exit, true);
                if self.find(index, exit, Name::GfmTaskListItemCheck).is_some()
                    && let Some(first) = n.children.first_mut().filter(|c| c.kind == Kind::Text)
                {
                    let width = first.value.chars().next().map_or(0, char::len_utf8);
                    first.value.drain(..width);
                    if first.value.is_empty() {
                        n.children.remove(0);
                    } else {
                        first.start += width;
                        n.start = first.start;
                    }
                }
                n
            }
            Name::HeadingAtx => {
                let mut n = node(Kind::Heading);
                if let Some(sequence) = self.find(index, exit, Name::HeadingAtxSequence) {
                    n.depth = (self.end(sequence) - self.start(sequence)) as u8;
                }
                n.children = self.phrasing_in(index, exit, Name::HeadingAtxText);
                n
            }
            Name::HeadingSetext => {
                let mut n = node(Kind::Heading);
                let sequence = self.find(index, exit, Name::HeadingSetextUnderlineSequence);
                n.depth = if sequence.is_some_and(|s| self.bytes[self.start(s)] == b'=') { 1 } else { 2 };
                n.children = self.phrasing_in(index, exit, Name::HeadingSetextText);
                n
            }
            Name::ThematicBreak => node(Kind::ThematicBreak),
            Name::BlockQuote => {
                let mut n = node(Kind::Blockquote);
                n.children = self.children(index + 1, exit, false);
                n
            }
            Name::ListOrdered | Name::ListUnordered => {
                let mut n = node(Kind::List);
                n.ordered = self.events[index].name == Name::ListOrdered;
                if n.ordered {
                    n.list_start = self.find(index, exit, Name::ListItemValue).and_then(|v| self.text(v).parse().ok());
                }
                n.spread = syntax::list_loose(self.events, index, false);
                n.children = self.children(index + 1, exit, false);
                n.end = self.content_end(exit);
                n
            }
            Name::ListItem => {
                let mut n = node(Kind::ListItem);
                n.spread = syntax::list_item_loose(self.events, index);
                let paragraph = self.direct(index, Name::Paragraph);
                if let Some(check) = paragraph.and_then(|p| self.find(p, self.exits[p], Name::GfmTaskListItemCheck)) {
                    n.checked = Some(self.find(check, self.exits[check], Name::GfmTaskListItemValueChecked).is_some());
                }
                n.children = self.children(index + 1, exit, false);
                n.end = n.children.last().map_or_else(|| self.content_end(exit), |child| child.end);
                if self
                    .events
                    .get(exit + 1)
                    .is_some_and(|next| next.kind == EventKind::Enter && next.name == Name::ListItem)
                    && let Some(prefix) = self.direct(exit + 1, Name::ListItemPrefix)
                {
                    n.end = self.end(prefix);
                }
                n
            }
            Name::CodeFenced | Name::CodeIndented => {
                let mut n = node(Kind::Code);
                let fenced = self.events[index].name == Name::CodeFenced;
                let lang = self.find(index, exit, Name::CodeFencedFenceInfo).map(|i| decode_string(&self.text(i)));
                let meta = self.find(index, exit, Name::CodeFencedFenceMeta).map(|i| decode_string(&self.text(i)));
                if lang.is_some() || meta.is_some() {
                    let extra = n.extra_mut();
                    extra.lang = lang;
                    extra.meta = meta;
                }
                let mut value = String::new();
                let mut inside = !fenced;
                for i in index + 1..exit {
                    let event = &self.events[i];
                    match (&event.kind, &event.name) {
                        (EventKind::Exit, Name::CodeFencedFence) => inside = true,
                        (EventKind::Enter, Name::CodeFencedFence) if inside => break,
                        (EventKind::Enter, Name::CodeFlowChunk) if inside => self.chunk(i, &mut value),
                        (EventKind::Enter, Name::LineEnding) if inside => value.push('\n'),
                        _ => {}
                    }
                }
                if fenced && value.starts_with('\n') {
                    value.remove(0);
                }
                if value.ends_with('\n') {
                    value.pop();
                }
                n.value = value;
                n
            }
            Name::HtmlFlow | Name::HtmlText => node(Kind::Html),
            Name::GsxMath => node(Kind::Math),
            Name::GfmTable => {
                let mut n = node(Kind::Table);
                n.extra_mut().align = syntax::gfm_table_align(self.events, index);
                n.children = self.children(index + 1, exit, false);
                unescape_table_code(&mut n);
                n
            }
            Name::GfmTableRow => {
                let mut n = node(Kind::TableRow);
                n.children = self.children(index + 1, exit, false);
                n
            }
            Name::GfmTableCell => {
                let mut n = node(Kind::TableCell);
                n.children = self.phrasing_in(index, exit, Name::GfmTableCellText);
                n
            }
            Name::Definition => node(Kind::Definition),
            Name::GfmFootnoteDefinition => {
                let mut n = node(Kind::FootnoteDefinition);
                n.children = self.children(index + 1, exit, false);
                n
            }
            Name::GfmFootnoteCall => node(Kind::FootnoteReference),
            Name::Emphasis
            | Name::Strong
            | Name::GfmStrikethrough
            | Name::GsxSpoiler
            | Name::GsxUnderline
            | Name::GsxHighlight => {
                let kind = match self.events[index].name {
                    Name::Emphasis => Kind::Emphasis,
                    Name::Strong => Kind::Strong,
                    Name::GfmStrikethrough => Kind::Delete,
                    Name::GsxSpoiler => Kind::Spoiler,
                    Name::GsxUnderline => Kind::Underline,
                    _ => Kind::Highlight,
                };
                let mut n = node(kind);
                n.children = self.children(index + 1, exit, true);
                n
            }
            Name::CodeText => {
                let mut n = node(Kind::InlineCode);
                let sequences: Vec<usize> = (index + 1..exit)
                    .filter(|&i| {
                        self.events[i].kind == EventKind::Enter && self.events[i].name == Name::CodeTextSequence
                    })
                    .collect();
                let (Some(&open), Some(&close)) = (sequences.first(), sequences.last()) else {
                    return Some(n);
                };
                let mut inner = String::new();
                let mut line_start = false;
                for i in self.exits[open] + 1..close {
                    let event = &self.events[i];
                    if event.kind != EventKind::Enter {
                        continue;
                    }
                    if event.name == Name::LineEnding {
                        inner.push('\n');
                        line_start = true;
                    } else if event.name == Name::CodeTextData {
                        let mut data = String::new();
                        self.chunk(i, &mut data);
                        inner.push_str(if line_start { data.trim_start_matches([' ', '\t']) } else { &data });
                        line_start = false;
                    }
                }
                let inner = inner.as_str();
                let spaced = |b: u8| b == b' ' || b == b'\n';
                let bytes = inner.as_bytes();
                n.value = if bytes.len() > 2
                    && spaced(bytes[0])
                    && spaced(bytes[bytes.len() - 1])
                    && !bytes.iter().all(|b| spaced(*b))
                {
                    inner[1..inner.len() - 1].to_string()
                } else {
                    inner.to_string()
                };
                n
            }
            Name::Link | Name::Image => {
                let image = self.events[index].name == Name::Image;
                let resource = self.direct(index, Name::Resource);
                let kind = match (image, resource.is_some()) {
                    (false, true) => Kind::Link,
                    (false, false) => Kind::LinkReference,
                    (true, true) => Kind::Image,
                    (true, false) => Kind::ImageReference,
                };
                let mut n = node(kind);
                if !image {
                    n.children = self.phrasing_in(index, exit, Name::LabelText);
                }
                if let Some(resource) = resource {
                    let close = self.exits[resource];
                    if let Some(i) = self.find(resource, close, Name::ResourceDestinationString) {
                        n.extra_mut().url = decode_string(&self.text(i));
                    }
                    if let Some(i) = self.find(resource, close, Name::ResourceTitleString) {
                        n.extra_mut().title = Some(decode_string(&self.text(i)));
                    }
                }
                n
            }
            Name::Autolink
            | Name::GfmAutolinkLiteralProtocol
            | Name::GfmAutolinkLiteralWww
            | Name::GfmAutolinkLiteralEmail => node(Kind::Link),
            Name::GsxDirectiveText => {
                let mut n = node(Kind::TextDirective);
                n.extra_mut().name =
                    self.find(index, exit, Name::GsxDirectiveTextIdentifier).map(|i| self.text(i)).unwrap_or_default();
                if let Some(label) = self.find(index, exit, Name::GsxDirectiveTextLabelString) {
                    n.children = self.children(label + 1, self.exits[label], true);
                    self.keep_label_space(&mut n.children, self.start(label), self.end(label));
                }
                let mut i = index;
                while i + 1 < exit {
                    i += 1;
                    if self.events[i].kind != EventKind::Enter {
                        continue;
                    }
                    if self.events[i].name == Name::GsxDirectiveText {
                        i = self.exits[i];
                        continue;
                    }
                    if self.events[i].name != Name::GsxDirectiveTextAttribute {
                        continue;
                    }
                    let close = self.exits[i];
                    let text = |name| self.find(i, close, name).map(|j| self.text(j));
                    n.extra_mut().attributes.push(match text(Name::GsxDirectiveTextAttributeShortcut) {
                        Some(shortcut) => Attribute { shortcut: true, name: shortcut, value: None },
                        None => Attribute {
                            shortcut: false,
                            name: text(Name::GsxDirectiveTextAttributeKey).unwrap_or_default(),
                            value: text(Name::GsxDirectiveTextAttributeValue).map(|v| decode_string(&v)),
                        },
                    });
                }
                n
            }
            Name::GsxMedia => node(Kind::MediaDirective),
            _ => return None,
        };
        Some(built)
    }
}

fn push_text(out: &mut Vec<Node>, start: usize, end: usize, value: &str) {
    if let Some(last) = out.last_mut().filter(|n| n.kind == Kind::Text) {
        last.end = end;
        last.value.push_str(value);
        return;
    }
    let mut node = Node::new(Kind::Text, start, end);
    node.value.push_str(value);
    out.push(node);
}

fn unescape_table_code(node: &mut Node) {
    if node.kind == Kind::InlineCode && node.value.contains("\\|") {
        let mut value = String::with_capacity(node.value.len());
        let mut chars = node.value.chars();
        while let Some(char) = chars.next() {
            value.push(char);
            if char == '\\' {
                match chars.next() {
                    Some('|') => {
                        value.pop();
                        value.push('|');
                    }
                    Some(next) => value.push(next),
                    None => {}
                }
            }
        }
        node.value = value;
    }
    for child in &mut node.children {
        unescape_table_code(child);
    }
}

fn character_reference(raw: &str, start: usize) -> Option<(String, usize)> {
    let bytes = raw.as_bytes();
    let (marker, body) = match (bytes.get(start + 1), bytes.get(start + 2)) {
        (Some(b'#'), Some(b'x' | b'X')) => (b'x', start + 3),
        (Some(b'#'), _) => (b'#', start + 2),
        _ => (b'&', start + 1),
    };
    let allowed = |byte: &&u8| match marker {
        b'x' => byte.is_ascii_hexdigit(),
        b'#' => byte.is_ascii_digit(),
        _ => byte.is_ascii_alphanumeric(),
    };
    let maximum = syntax::character_reference_size(marker);
    let size = bytes[body..].iter().take(maximum + 1).take_while(allowed).count();
    if size == 0 || size > maximum || bytes.get(body + size) != Some(&b';') {
        return None;
    }
    syntax::decode_character_reference(&raw[body..body + size], marker, true).map(|value| (value, body + size + 1))
}

pub(super) fn decode_string(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = String::new();
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'\\' && bytes.get(index + 1).is_some_and(u8::is_ascii_punctuation) {
            out.push(bytes[index + 1] as char);
            index += 2;
            continue;
        }
        if byte == b'&'
            && let Some((decoded, after)) = character_reference(raw, index)
        {
            out.push_str(&decoded);
            index = after;
            continue;
        }
        let char = raw[index..].chars().next().expect("index is a char boundary");
        out.push(char);
        index += char.len_utf8();
    }
    out
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Prefix {
    pub start: usize,
    pub end: usize,
    pub padding: usize,
}

struct Columns<'b> {
    bytes: &'b [u8],
    index: usize,
    column: usize,
}

impl Columns<'_> {
    fn at(&mut self, index: usize) -> usize {
        if index < self.index {
            self.index = self.bytes[..index].iter().rposition(|byte| *byte == b'\n').map_or(0, |at| at + 1);
            self.column = 0;
        }
        for byte in &self.bytes[self.index..index] {
            self.column = match byte {
                b'\n' => 0,
                b'\t' => self.column + TAB_SIZE - self.column % TAB_SIZE,
                _ => self.column + 1,
            };
        }
        self.index = index;
        self.column
    }
}

fn prefixes(events: &[Event], exits: &[usize], bytes: &[u8]) -> Vec<Prefix> {
    let mut columns = Columns { bytes, index: 0, column: 0 };
    let mut containers: Vec<&Name> = vec![];
    let mut ranges: Vec<(usize, usize, usize, usize)> = vec![];
    let mut expect = None;
    let mut index = 0;
    while index < events.len() {
        let event = &events[index];
        if event.kind == EventKind::Exit {
            if matches!(event.name, Name::BlockQuote | Name::ListItem) {
                containers.pop();
            }
            if matches!(event.name, Name::LineEnding | Name::BlankLineEnding) {
                expect = Some(0);
            }
            index += 1;
            continue;
        }
        if let Some(at) = expect {
            let wanted = match containers.get(at) {
                Some(Name::BlockQuote) => Some(Name::BlockQuotePrefix),
                Some(Name::ListItem) => Some(Name::SpaceOrTab),
                _ => None,
            };
            if wanted.as_ref() == Some(&event.name) {
                let (from, to) = (&event.point, &events[exits[index]].point);
                let start = from.index + usize::from(from.vs > 0);
                let end = to.index + usize::from(to.vs > 0);
                let column = columns.at(from.index) + from.vs;
                let width = columns.at(to.index) + to.vs - column;
                match ranges.last_mut() {
                    Some(last) if last.1 >= start => {
                        last.1 = last.1.max(end);
                        last.3 += width;
                    }
                    _ => ranges.push((start, end, column, width)),
                }
                expect = Some(at + 1);
                index = exits[index] + 1;
                continue;
            }
            expect = None;
        }
        if matches!(event.name, Name::BlockQuote | Name::ListItem) {
            containers.push(&event.name);
        }
        index += 1;
    }
    ranges
        .into_iter()
        .map(|(start, end, column, width)| {
            let visual = bytes[start..end]
                .iter()
                .fold(column, |at, byte| at + if *byte == b'\t' { TAB_SIZE - at % TAB_SIZE } else { 1 });
            Prefix { start, end, padding: (visual - column).saturating_sub(width) }
        })
        .collect()
}

pub(super) fn build(events: &[Event], bytes: &[u8]) -> (Node, Vec<Prefix>) {
    let mut exits = vec![0; events.len()];
    let mut stack = vec![];
    for (index, event) in events.iter().enumerate() {
        match event.kind {
            EventKind::Enter => stack.push(index),
            EventKind::Exit => {
                if let Some(enter) = stack.pop() {
                    exits[enter] = index;
                }
            }
        }
    }
    let prefixes = prefixes(events, &exits, bytes);
    let builder = Builder { events, bytes, exits };
    let mut root = Node::new(Kind::Root, 0, bytes.len());
    root.children = builder.children(0, events.len(), false);
    (root, prefixes)
}
