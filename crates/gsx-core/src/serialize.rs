use crate::document::{Align, Document, Kind, Media, Node, Provenance, Range, parse};
use crate::syntax::{CharKind, classify};

pub fn serialize(document: &Document) -> String {
    canonical(document)
        .or_else(|| {
            let media = placed(document);
            parse(document.text(), &media).ok().and_then(|normalized| canonical(&normalized))
        })
        .unwrap_or_else(|| document.text().to_owned())
}

fn placed(document: &Document) -> Vec<Media<'_>> {
    document
        .children()
        .iter()
        .flat_map(|node| node.media_ids().unwrap_or_default())
        .map(|id| Media { id, cover: None })
        .collect()
}

pub fn canonical(document: &Document) -> Option<String> {
    let mut serializer = Serializer { document, out: String::new(), frames: vec![], trailing: false };
    serializer.blocks(document.children());
    if !serializer.trailing {
        serializer.out.pop();
    }
    let reparsed = parse(&serializer.out, &placed(document)).ok()?;
    same(document.children(), reparsed.children()).then_some(serializer.out)
}

struct Frame {
    first: String,
    rest: String,
    started: bool,
}

struct Serializer<'d> {
    document: &'d Document,
    out: String,
    frames: Vec<Frame>,
    trailing: bool,
}

impl Serializer<'_> {
    fn prefix(&mut self) -> String {
        let mut prefix = String::new();
        for frame in &mut self.frames {
            prefix.push_str(if frame.started { &frame.rest } else { &frame.first });
            frame.started = true;
        }
        prefix
    }

    fn line(&mut self, content: &str) {
        self.trailing = false;
        let prefix = self.prefix();
        if content.is_empty() {
            self.out.push_str(prefix.trim_end());
        } else {
            self.out.push_str(&prefix);
            self.out.push_str(content);
        }
        self.out.push('\n');
    }

    fn lines(&mut self, content: &str, raw: &[usize]) {
        let mut start = 0;
        let mut raw = raw.iter().peekable();
        for line in content.split('\n') {
            if raw.next_if_eq(&&start).is_some() {
                self.out.push_str(line);
                self.out.push('\n');
            } else {
                self.line(line);
            }
            start += line.len() + 1;
        }
    }

    fn blocks(&mut self, nodes: &[Node]) {
        let mut marker = 0;
        for (index, node) in nodes.iter().enumerate() {
            let previous = index.checked_sub(1).map(|previous| &nodes[previous]);
            if previous.is_some() {
                for _ in 1..node.gap_before.unwrap_or(1).max(1) {
                    self.line("");
                }
            }
            marker = match (previous.map(|previous| &previous.kind), &node.kind) {
                (Some(Kind::List { ordered: before, .. }), Kind::List { ordered, .. }) if before == ordered => {
                    1 - marker
                }
                _ => 0,
            };
            self.block(node, previous, marker);
            let lazy = nodes.get(index + 1).is_some_and(|next| {
                next.kind == Kind::Paragraph
                    && next.gap_before.unwrap_or(1) <= 1
                    && open_end(self.document, node, false)
            });
            if lazy {
                self.close(node);
            }
        }
    }

    fn close(&mut self, node: &Node) {
        let mut content = String::new();
        let mut node = node;
        loop {
            match &node.kind {
                Kind::Blockquote => content.push_str("> "),
                Kind::List { ordered, start, .. } => {
                    let width = if *ordered {
                        (start.unwrap_or(1) as usize + node.children.len() - 1).to_string().len() + 2
                    } else {
                        2
                    };
                    content.extend(std::iter::repeat_n(' ', width));
                    let Some(item) = node.children.last() else { break };
                    node = item;
                }
                _ => break,
            }
            let Some(last) = node.children.last() else { break };
            node = last;
        }
        if content.ends_with("> ") {
            content.pop();
        }
        self.line(&content);
    }

    fn block(&mut self, node: &Node, previous: Option<&Node>, marker: usize) {
        match &node.kind {
            Kind::Paragraph => {
                let (content, raw, _) = self.checked(node);
                self.lines(&content, &raw);
            }
            Kind::Heading { depth } => self.heading(node, *depth),
            Kind::ThematicBreak => {
                let after_paragraph = previous
                    .is_some_and(|previous| previous.kind == Kind::Paragraph && node.gap_before.unwrap_or(1) <= 1);
                let first_in_item = previous.is_none() && self.frames.last().is_some_and(|frame| !frame.started);
                self.line(if after_paragraph || first_in_item { "___" } else { "---" });
            }
            Kind::Blockquote => {
                self.frames.push(Frame { first: "> ".into(), rest: "> ".into(), started: false });
                if node.children.is_empty() {
                    self.line("");
                }
                self.blocks(&node.children);
                self.frames.pop();
            }
            Kind::List { ordered, start, .. } => {
                for (index, item) in node.children.iter().enumerate() {
                    if index > 0 {
                        for _ in 1..item.gap_before.unwrap_or(1).max(1) {
                            self.line("");
                        }
                    }
                    let number = start.unwrap_or(1) as usize + index;
                    let bullet = match (ordered, marker) {
                        (true, 0) => format!("{number}. "),
                        (true, _) => format!("{number}) "),
                        (false, 0) => "- ".into(),
                        (false, _) => "* ".into(),
                    };
                    self.item(item, bullet);
                }
            }
            Kind::Code { lang, meta, value } => self.code(lang.as_deref(), meta.as_deref(), value),
            Kind::Table { align } => self.table(node, align),
            Kind::Text { .. } if node.provenance == Provenance::Literal => {
                let source = self.literal(node);
                let content = source.strip_suffix('\n').unwrap_or(&source);
                let raw: Vec<usize> = content.match_indices('\n').map(|(at, _)| at + 1).collect();
                self.lines(content, &raw);
                self.trailing = content.len() < source.len();
            }
            Kind::MediaReference { id } => self.line(&format!("::media{{id=\"{id}\"}}")),
            Kind::MediaGroup { ids, layout } => {
                self.line(&format!("::media{{ids=\"{}\" layout=\"{}\"}}", ids.join(","), layout.as_str()));
            }
            _ => {
                let (content, raw) = self.inline(std::slice::from_ref(node), Context::Line);
                self.lines(&content, &raw);
            }
        }
    }

    fn item(&mut self, item: &Node, bullet: String) {
        let Kind::ListItem { checked, .. } = item.kind else {
            return self.block(item, None, 0);
        };
        let rest = " ".repeat(bullet.len());
        let first = match checked {
            Some(true) => format!("{bullet}[x] "),
            Some(false) => format!("{bullet}[ ] "),
            None => bullet,
        };
        self.frames.push(Frame { first, rest, started: false });
        if item.children.is_empty() {
            self.line("");
        }
        self.blocks(&item.children);
        self.frames.pop();
    }

    fn heading(&mut self, node: &Node, depth: u8) {
        let (mut content, raw, canonical) = self.checked(node);
        if content.contains('\n') && depth <= 2 {
            self.lines(&content, &raw);
            self.line(if depth == 1 { "===" } else { "---" });
            return;
        }
        let run = content.len() - content.trim_end_matches('#').len();
        let open = run > 0 && (run == content.len() || content[..content.len() - run].ends_with([' ', '\t', '\0']));
        if canonical && open {
            content.truncate(content.len() - 1);
            content.push_str("&#35;");
        }
        let marks = "#".repeat(depth as usize);
        if content.is_empty() { self.line(&marks) } else { self.line(&format!("{marks} {content}")) }
    }

    fn code(&mut self, lang: Option<&str>, meta: Option<&str>, value: &str) {
        let info = match (lang, meta) {
            (Some(lang), Some(meta)) => format!("{} {}", info(lang), info(meta)),
            (Some(lang), None) => info(lang),
            (None, Some(meta)) => info(meta),
            (None, None) => String::new(),
        };
        let fence_char = if info.contains('`') { '~' } else { '`' };
        let fence = fence_char.to_string().repeat(longest_run(value, fence_char).max(2) + 1);
        self.line(&format!("{fence}{info}"));
        if !value.is_empty() {
            self.lines(value, &[]);
        }
        self.line(&fence);
    }

    fn table(&mut self, node: &Node, align: &[Align]) {
        for (index, row) in node.children.iter().enumerate() {
            let cells: Vec<String> =
                row.children.iter().map(|cell| self.inline(&cell.children, Context::Cell).0).collect();
            self.line(&format!("| {} |", cells.join(" | ")));
            if index == 0 {
                let delimiters: Vec<&str> = align
                    .iter()
                    .map(|align| match align {
                        Align::None => "---",
                        Align::Left => ":---",
                        Align::Right => "---:",
                        Align::Center => ":---:",
                    })
                    .collect();
                self.line(&format!("| {} |", delimiters.join(" | ")));
            }
        }
    }

    fn literal(&self, node: &Node) -> String {
        self.document.source(node.range).replace("\r\n", "\n").replace('\r', "\n")
    }

    fn checked(&self, node: &Node) -> (String, Vec<usize>, bool) {
        let (content, raw) = self.inline(&node.children, Context::Line);
        let same = node.children.is_empty()
            || parse(&content, &[]).is_ok_and(|reparsed| match reparsed.children() {
                [paragraph] => paragraph.kind == Kind::Paragraph && same(&node.children, &paragraph.children),
                _ => false,
            });
        if same {
            return (content, raw, true);
        }
        let start = node.children.first().map_or(node.range.start, |first| first.range.start);
        let end = node.children.last().map_or(node.range.end, |last| last.range.end);
        let source = self.document.source(Range { start, end });
        let source = source.strip_prefix([' ', '\t']).unwrap_or(source).replace("\r\n", "\n").replace('\r', "\n");
        let raw = source.match_indices('\n').map(|(at, _)| at + 1).collect();
        (source, raw, false)
    }

    fn inline(&self, nodes: &[Node], context: Context) -> (String, Vec<usize>) {
        let mut inline = Inline::new(self, context, None, context == Context::Line);
        inline.nodes(nodes, None, true, None);
        (inline.out, inline.raw)
    }
}

#[derive(Clone, Copy)]
struct Parent {
    delimiter: char,
}

#[derive(Clone, Copy)]
struct Nest {
    parent: Parent,
    sole: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Context {
    Line,
    Cell,
}

struct Inline<'s, 'd> {
    serializer: &'s Serializer<'d>,
    out: String,
    context: Context,
    before: Option<char>,
    opens_line: bool,
    raw_end: usize,
    reference_next: bool,
    open: [usize; 2],
    raw: Vec<usize>,
    run_before: Option<char>,
    run_after: Option<char>,
}

impl<'s, 'd> Inline<'s, 'd> {
    fn new(serializer: &'s Serializer<'d>, context: Context, before: Option<char>, opens_line: bool) -> Self {
        Inline {
            serializer,
            out: String::new(),
            context,
            before,
            opens_line,
            raw_end: usize::MAX,
            reference_next: false,
            open: [0; 2],
            raw: vec![],
            run_before: None,
            run_after: None,
        }
    }
}

impl Inline<'_, '_> {
    fn reference_last(&mut self) {
        if self.raw_end == self.out.len()
            && let Some(last) = self.out.pop()
        {
            reference(&mut self.out, last);
        }
    }

    fn last(&self, node: &Node) -> Option<char> {
        match &node.kind {
            Kind::Text { value } if node.provenance == Provenance::Literal => {
                self.serializer.document.source(node.range).chars().next_back().or_else(|| value.chars().next_back())
            }
            Kind::Text { value } => value.chars().next_back(),
            Kind::Emphasis | Kind::Strong => Some('*'),
            Kind::Delete => Some('~'),
            Kind::Underline => Some('+'),
            Kind::Highlight => Some('='),
            Kind::Spoiler => Some('|'),
            Kind::InlineCode { .. } => Some('`'),
            Kind::Link { .. } => Some(')'),
            Kind::Datetime { .. } => Some('}'),
            Kind::Break => Some('\n'),
            _ => None,
        }
    }

    fn nodes(&mut self, nodes: &[Node], after: Option<char>, edge: bool, parent: Option<Parent>) {
        for (index, node) in nodes.iter().enumerate() {
            let next = nodes.get(index + 1).map_or(after, |next| self.first(next));
            let edges = (edge && index == 0, edge && index + 1 == nodes.len());
            let nest = parent.map(|parent| Nest { parent, sole: nodes.len() == 1 });
            self.node(node, next, edges, nest);
        }
    }

    fn first(&self, node: &Node) -> Option<char> {
        match &node.kind {
            Kind::Text { value } if node.provenance == Provenance::Literal => {
                self.serializer.document.source(node.range).chars().next().or_else(|| value.chars().next())
            }
            Kind::Text { value } => match value.chars().next() {
                Some('`' | '*' | '[' | ']' | '<' | '~' | '|' | '$') => Some('&'),
                first => first,
            },
            Kind::Emphasis | Kind::Strong => Some('*'),
            Kind::Delete => Some('~'),
            Kind::Underline => Some('+'),
            Kind::Highlight => Some('='),
            Kind::Spoiler => Some('|'),
            Kind::InlineCode { .. } => Some('`'),
            Kind::Link { .. } => Some('['),
            Kind::Datetime { .. } => Some(':'),
            Kind::Break => Some('\\'),
            _ => None,
        }
    }

    fn node(&mut self, node: &Node, next: Option<char>, edges: (bool, bool), nest: Option<Nest>) {
        if node.provenance == Provenance::Literal || !matches!(node.kind, Kind::Text { .. }) {
            self.reference_next = false;
        }
        match &node.kind {
            Kind::Text { .. } if node.provenance == Provenance::Literal => {
                if self.out.ends_with('\n') {
                    self.out.push_str("    ");
                }
                let source = self.serializer.literal(node);
                let mut lines = source.split('\n');
                self.out.push_str(lines.next().unwrap_or_default());
                for line in lines {
                    self.out.push('\n');
                    self.raw.push(self.out.len());
                    self.out.push_str(line);
                }
            }
            Kind::Text { value } => self.text(value, next, edges),
            Kind::Emphasis | Kind::Strong => self.emphasis(node, next, edges, nest),
            Kind::Delete => self.wrap(node, "~~"),
            Kind::Underline => self.wrap(node, "++"),
            Kind::Highlight => self.wrap(node, "=="),
            Kind::Spoiler => self.wrap(node, "||"),
            Kind::InlineCode { value } => self.code(value),
            Kind::Break => self.out.push_str("\\\n"),
            Kind::Link { url: _, source, title } => self.link(node, source, title.as_deref()),
            Kind::Datetime { datetime, unix } => {
                self.out.push_str(":time[");
                self.nodes(&node.children, Some(']'), false, None);
                if let Some(datetime) = datetime {
                    self.out.push_str("]{datetime=\"");
                    self.out.push_str(datetime);
                } else {
                    self.out.push_str("]{unix=\"");
                    self.out.push_str(&unix.to_string());
                }
                self.out.push_str("\"}");
            }
            _ => self.nodes(&node.children, next, false, None),
        }
    }

    fn wrap(&mut self, node: &Node, delimiter: &str) {
        self.out.push_str(delimiter);
        self.nodes(&node.children, delimiter.chars().next(), true, None);
        self.out.push_str(delimiter);
    }

    fn emphasis(&mut self, node: &Node, next: Option<char>, edges: (bool, bool), nest: Option<Nest>) {
        let strong = node.kind == Kind::Strong;
        let inside = |char: Option<char>| match kind(char) {
            CharKind::Whitespace if char.is_some() => Some('&'),
            _ => char,
        };
        let first = inside(node.children.first().and_then(|child| self.first(child)));
        let last = inside(node.children.last().and_then(|child| self.last(child)));
        let emphasis = |child: Option<&Node>| child.is_some_and(|child| child.kind == Kind::Emphasis);
        let nested = node.children.len() == 1 && emphasis(node.children.first());
        let order = if nested { ['_', '*'] } else { ['*', '_'] };
        let (open, run_before, run_after) = (self.open, self.run_before, self.run_after);
        let merged = |edge: bool, delimiter: char| edge && nest.is_some_and(|nest| nest.parent.delimiter == delimiter);
        let choose = |before: Option<char>, next: Option<char>, strict: bool| {
            let clash = |nest: Nest, delimiter: char| nest.parent.delimiter == delimiter && !strong && nest.sole;
            let conflict = |delimiter: char| {
                let start = match nest {
                    Some(nest) if edges.0 => clash(nest, delimiter),
                    _ => before == Some(delimiter),
                };
                let end = match nest {
                    Some(nest) if edges.1 => clash(nest, delimiter),
                    _ => next == Some(delimiter),
                };
                start || end
            };
            let fits = |delimiter: char| {
                let (start, end) = (merged(edges.0, delimiter), merged(edges.1, delimiter));
                let opener = flanking(delimiter, if start { run_before } else { before }, first);
                let closer = flanking(delimiter, last, if end { run_after } else { next });
                opener.0
                    && closer.1
                    && !(open[usize::from(delimiter == '_')] > usize::from(start) && opener.1)
                    && !(start && (opener.1 || closer.0))
                    && !(end && (closer.0 || opener.1))
            };
            order.into_iter().find(|&delimiter| fits(delimiter) && !(strict && conflict(delimiter)))
        };
        let mut delimiter = choose(self.previous(), next, true);
        if delimiter.is_none() {
            self.reference_last();
            delimiter = choose(self.previous(), next, true);
        }
        if delimiter.is_none() && kind(next) != CharKind::Whitespace && !edges.1 {
            self.reference_next = true;
            delimiter = choose(self.previous(), Some(';'), true);
        }
        let next = if self.reference_next { Some(';') } else { next };
        let delimiter = delimiter.or_else(|| choose(self.previous(), next, false)).unwrap_or('*');
        let mut inner = Inline::new(self.serializer, self.context, Some(delimiter), false);
        inner.open = self.open;
        inner.open[usize::from(delimiter == '_')] += 1;
        inner.run_before = if merged(edges.0, delimiter) { self.run_before } else { self.previous() };
        inner.run_after = if merged(edges.1, delimiter) { self.run_after } else { next };
        inner.nodes(&node.children, Some(delimiter), true, Some(Parent { delimiter }));
        let run = if strong { 2 } else { 1 };
        self.out.extend(std::iter::repeat_n(delimiter, run));
        let base = self.out.len();
        self.raw.extend(inner.raw.iter().map(|at| at + base));
        self.out.push_str(&inner.out);
        self.out.extend(std::iter::repeat_n(delimiter, run));
    }

    fn code(&mut self, value: &str) {
        let fence = "`".repeat(longest_run(value, '`') + 1);
        let padded = value.starts_with('`')
            || value.ends_with('`')
            || (value.starts_with(' ') && value.ends_with(' ') && value.chars().any(|char| char != ' '));
        let value = if self.context == Context::Cell { value.replace('|', "\\|") } else { value.to_owned() };
        self.out.push_str(&fence);
        if padded {
            self.out.push(' ');
        }
        self.out.push_str(&value);
        if padded {
            self.out.push(' ');
        }
        self.out.push_str(&fence);
    }

    fn link(&mut self, node: &Node, source: &str, title: Option<&str>) {
        if title.is_none()
            && let [child] = node.children.as_slice()
            && child.provenance == Provenance::Authored
            && matches!(&child.kind, Kind::Text { value } if value == source)
            && !source.contains(|char: char| char.is_whitespace() || char.is_control() || matches!(char, '<' | '>'))
        {
            self.out.push('<');
            self.out.push_str(source);
            self.out.push('>');
            return;
        }
        self.out.push('[');
        self.nodes(&node.children, Some(']'), false, None);
        self.out.push_str("](");
        if bare(source) {
            self.out.push_str(source);
        } else {
            self.out.push('<');
            self.out.push_str(source);
            self.out.push('>');
        }
        if let Some(title) = title {
            self.out.push_str(" \"");
            let chars: Vec<char> = title.chars().collect();
            for (index, &char) in chars.iter().enumerate() {
                let next = chars.get(index + 1).copied();
                if matches!(char, '"' | '\\') || (char == '&' && entity_start(next)) {
                    self.out.push('\\');
                }
                self.out.push(char);
            }
            self.out.push('"');
        }
        self.out.push(')');
    }

    fn previous(&self) -> Option<char> {
        self.out.chars().next_back().or(self.before)
    }

    fn line_start(&self) -> bool {
        match self.out.chars().next_back() {
            None => self.opens_line,
            Some(last) => last == '\n',
        }
    }

    fn text(&mut self, value: &str, after: Option<char>, (edge_start, edge_end): (bool, bool)) {
        let chars: Vec<char> = value.chars().collect();
        let mut digits: Option<usize> = None;
        let mut index = 0;
        while index < chars.len() {
            let char = chars[index];
            let previous = if index == 0 { self.previous() } else { Some(chars[index - 1]) };
            let next = chars.get(index + 1).copied().or(after);
            let line_start = self.line_start();
            if matches!(char, ' ' | '\t') {
                let end = chars[index..]
                    .iter()
                    .position(|char| !matches!(char, ' ' | '\t'))
                    .map_or(chars.len(), |at| at + index);
                let leading = line_start || (index == 0 && edge_start);
                let trailing = chars.get(end) == Some(&'\n') || (end == chars.len() && (edge_end || after.is_none()));
                for &space in &chars[index..end] {
                    if leading || trailing {
                        reference(&mut self.out, space);
                    } else {
                        self.out.push(space);
                    }
                }
                index = end;
                digits = None;
                continue;
            }
            if char == '\n' {
                let soft = !line_start && next.is_some_and(|next| next != '\n');
                if soft {
                    self.out.push('\n');
                } else {
                    reference(&mut self.out, '\n');
                }
                index += 1;
                digits = None;
                continue;
            }
            if line_start {
                digits = Some(0);
            }
            let escape = match char {
                '\\' => !next.is_some_and(|next| next.is_alphanumeric() || next == ' '),
                '`' | '*' | '[' | ']' | '<' | '~' | '|' | '$' | '\u{feff}' => true,
                '_' => !(previous.is_some_and(char::is_alphanumeric) && next.is_some_and(char::is_alphanumeric)),
                '&' => entity_start(next),
                '+' | '=' => line_start || previous == Some(char) || next == Some(char),
                ':' => next.is_some_and(|next| next.is_ascii_alphanumeric() || !next.is_ascii() || next == '/'),
                '@' => previous.is_some_and(|previous| {
                    previous.is_ascii_alphanumeric() || matches!(previous, '.' | '+' | '-' | '_')
                }),
                '.' => {
                    digits.is_some_and(|count| (1..=9).contains(&count))
                        || index >= 3 && chars[index - 3..index].iter().all(|char| matches!(char, 'w' | 'W'))
                }
                '#' | '>' | '-' => line_start,
                ')' => digits.is_some_and(|count| (1..=9).contains(&count)),
                _ => false,
            };
            digits = match (digits, char.is_ascii_digit()) {
                (Some(count), true) => Some(count + 1),
                _ => None,
            };
            if escape || std::mem::take(&mut self.reference_next) {
                reference(&mut self.out, char);
            } else {
                self.out.push(char);
                self.raw_end = self.out.len();
            }
            index += 1;
        }
    }
}

fn reference(out: &mut String, char: char) {
    out.push_str("&#");
    out.push_str(&u32::from(char).to_string());
    out.push(';');
}

fn kind(char: Option<char>) -> CharKind {
    char.map_or(CharKind::Whitespace, classify)
}

fn flanking(delimiter: char, before: Option<char>, after: Option<char>) -> (bool, bool) {
    let (kind_before, kind_after) = (kind(before), kind(after));
    let marker = |char: Option<char>| matches!(char, Some('~' | '|' | '+' | '='));
    let open = kind_after == CharKind::Other
        || (kind_after == CharKind::Punctuation && kind_before != CharKind::Other)
        || marker(after);
    let close = kind_before == CharKind::Other
        || (kind_before == CharKind::Punctuation && kind_after != CharKind::Other)
        || marker(before);
    if delimiter == '_' {
        (open && (kind_before != CharKind::Other || !close), close && (kind_after != CharKind::Other || !open))
    } else {
        (open, close)
    }
}

fn same(expected: &[Node], actual: &[Node]) -> bool {
    let mut expected = expected.iter().peekable();
    let mut actual = actual.iter().peekable();
    loop {
        match (expected.next(), actual.next()) {
            (None, None) => return true,
            (Some(left), Some(right)) => {
                if let (Kind::Text { value: left_value }, Kind::Text { value: right_value }) = (&left.kind, &right.kind)
                    && left.provenance == Provenance::Authored
                    && right.provenance == Provenance::Authored
                {
                    let mut left_text = left_value.clone();
                    while let Some(next) = expected.next_if(|next| authored_text(next)) {
                        left_text.push_str(text_value(next));
                    }
                    let mut right_text = right_value.clone();
                    while let Some(next) = actual.next_if(|next| authored_text(next)) {
                        right_text.push_str(text_value(next));
                    }
                    if left_text != right_text {
                        return false;
                    }
                    continue;
                }
                if left.kind != right.kind
                    || left.provenance != right.provenance
                    || left.gap_before != right.gap_before
                    || !same(&left.children, &right.children)
                {
                    return false;
                }
            }
            _ => return false,
        }
    }
}

fn authored_text(node: &Node) -> bool {
    node.provenance == Provenance::Authored && matches!(node.kind, Kind::Text { .. })
}

fn text_value(node: &Node) -> &str {
    match &node.kind {
        Kind::Text { value } => value,
        _ => "",
    }
}

fn open_end(document: &Document, node: &Node, nested: bool) -> bool {
    match &node.kind {
        Kind::Blockquote | Kind::List { .. } | Kind::ListItem { .. } => {
            node.children.last().is_some_and(|last| open_end(document, last, true))
        }
        Kind::Paragraph => true,
        Kind::Text { .. } => {
            nested && node.provenance == Provenance::Literal && document.source(node.range).ends_with('\n')
        }
        _ => false,
    }
}

fn entity_start(next: Option<char>) -> bool {
    next.is_some_and(|next| next == '#' || next.is_ascii_alphanumeric())
}

fn info(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    let mut out = String::with_capacity(value.len());
    for (index, &char) in chars.iter().enumerate() {
        let next = chars.get(index + 1).copied();
        if (char == '\\' && next.is_none_or(|next| next.is_ascii_punctuation())) || (char == '&' && entity_start(next))
        {
            out.push('\\');
        }
        out.push(char);
    }
    out
}

fn bare(source: &str) -> bool {
    if source.is_empty() || source.starts_with('<') {
        return false;
    }
    let mut depth = 0usize;
    let mut escaped = false;
    for char in source.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        match char {
            '\\' => escaped = true,
            '(' => depth += 1,
            ')' if depth == 0 => return false,
            ')' => depth -= 1,
            char if char.is_ascii_whitespace() || char.is_ascii_control() || char == ' ' => return false,
            _ => {}
        }
    }
    depth == 0 && !escaped
}

pub(crate) fn longest_run(value: &str, target: char) -> usize {
    let (mut longest, mut run) = (0, 0);
    for char in value.chars() {
        run = if char == target { run + 1 } else { 0 };
        longest = longest.max(run);
    }
    longest
}
