use std::ops::Range as Span;

use crate::document::Range;

use super::change::Feature;
use super::table::{Escape, Style};

pub const BULLET: &str = "– ";
pub const CHECKED: &str = "☑ ";
pub const UNCHECKED: &str = "☐ ";
pub const RULE: &str = "———";
pub const QUOTE: &str = "› ";
pub const CELL: &str = " | ";
pub const ELLIPSIS: &str = "…";

pub fn marker(ordered: bool, start: Option<u32>, index: usize, checked: Option<bool>) -> String {
    let task = match checked {
        None => "",
        Some(true) => CHECKED,
        Some(false) => UNCHECKED,
    };
    if ordered {
        format!("{}. {task}", u64::from(start.unwrap_or(1)) + index as u64)
    } else if task.is_empty() {
        BULLET.into()
    } else {
        task.into()
    }
}

#[derive(Debug)]
enum Prefix {
    Quote { pending: bool, mark: &'static str },
    Item { width: usize, marker: Option<String> },
}

impl Prefix {
    fn pending(&self) -> bool {
        matches!(self, Prefix::Quote { pending: true, .. } | Prefix::Item { marker: Some(_), .. })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Styled {
    pub style: Style,
    pub value: Option<String>,
    pub span: Span<usize>,
    pub source: Option<(Feature, Range)>,
    pub decoration: bool,
}

#[derive(Debug, Default)]
pub struct Written {
    pub text: String,
    pub links: Vec<Span<usize>>,
    pub replaced: Vec<Range>,
    pub styled: Vec<Styled>,
    pub prefixes: Vec<Span<usize>>,
    pub eligible: Vec<Span<usize>>,
    pub suffix: Option<Span<usize>>,
    pub origins: Vec<(Span<usize>, Range)>,
}

#[derive(Debug)]
pub struct Writer {
    out: Written,
    escape: Escape,
    prefixes: Vec<Prefix>,
    fresh: bool,
    emitted: usize,
    verbatim: bool,
    source: Option<Range>,
}

impl Writer {
    pub fn new(escape: Escape) -> Self {
        Writer {
            out: Written::default(),
            escape,
            prefixes: vec![],
            fresh: true,
            emitted: 0,
            verbatim: false,
            source: None,
        }
    }

    pub fn source(&mut self, range: Range) {
        self.source = Some(range);
    }

    pub fn push(&mut self, value: &str) {
        let mut lines = value.split('\n');
        if let Some(line) = lines.next() {
            self.put_line(line);
        }
        for line in lines {
            if self.verbatim {
                self.out.text.push('\n');
            } else if self.escape == Escape::HtmlBreaks {
                self.raw("<br>");
            } else {
                if self.fresh {
                    self.blank();
                }
                self.out.text.push('\n');
                self.fresh = true;
                self.emitted = 0;
            }
            self.put_line(line);
        }
    }

    fn put_line(&mut self, line: &str) {
        if line.is_empty() {
            return;
        }
        self.begin();
        let escaped: &[u8] = match self.escape {
            Escape::None => b"",
            Escape::LinkedIn => b"|{}@[]()<>#\\*_~",
            Escape::Html | Escape::HtmlBreaks => b"&<>",
            Escape::Angle => b"<>",
        };
        let mut rest = line;
        while let Some(at) = rest.bytes().position(|byte| escaped.contains(&byte)) {
            self.out.text.push_str(&rest[..at]);
            let byte = rest.as_bytes()[at];
            if self.escape == Escape::LinkedIn && byte == b'#' && self.hashtag(&rest[at + 1..]) {
                self.out.text.push('#');
            } else {
                self.put(char::from(byte));
            }
            rest = &rest[at + 1..];
        }
        self.out.text.push_str(rest);
    }

    pub fn raw(&mut self, markup: &str) {
        self.flush();
        self.out.text.push_str(markup);
    }

    pub fn verbatim(&mut self, verbatim: bool) {
        self.verbatim = verbatim;
    }

    pub fn flush(&mut self) {
        if self.fresh {
            self.begin_with(self.prefixes.len());
            self.emitted = self.prefixes.len();
        }
    }

    pub fn markers(&mut self) {
        if self.fresh
            && let Some(last) = self.prefixes.iter().rposition(Prefix::pending)
        {
            self.begin_with(last + 1);
            self.emitted = self.emitted.max(last + 1);
        }
    }

    pub fn insert(&mut self, at: usize, markup: &str) {
        self.unspanned();
        self.out.text.insert_str(at, markup);
    }

    pub fn edit(&mut self, edit: impl FnOnce(&mut String)) {
        self.unspanned();
        edit(&mut self.out.text);
    }

    fn unspanned(&mut self) {
        debug_assert!(
            self.out.links.is_empty() && self.out.styled.is_empty() && self.out.replaced.is_empty(),
            "text rewritten under recorded links, styles or replacements"
        );
        self.out.prefixes.clear();
        self.out.eligible.clear();
        self.out.suffix = None;
        self.out.origins.clear();
    }

    pub fn newlines(&mut self, count: u32) {
        for _ in 0..count {
            self.push("\n");
        }
    }

    pub fn link(&mut self, href: &str) {
        self.begin();
        let start = self.out.text.len();
        self.push(href);
        self.out.links.push(start..self.out.text.len());
    }

    pub fn open_quote(&mut self, mark: &'static str) {
        self.prefixes.push(Prefix::Quote { pending: true, mark });
    }

    pub fn open_item(&mut self, marker: String) {
        self.prefixes.push(Prefix::Item { width: marker.chars().count(), marker: Some(marker) });
    }

    pub fn close(&mut self) {
        if self.prefixes.last().is_some_and(Prefix::pending) {
            self.begin_with(self.prefixes.len());
            self.fresh = false;
            self.emitted = 0;
        }
        self.prefixes.pop();
        self.emitted = self.emitted.min(self.prefixes.len());
    }

    pub fn mark(&self) -> usize {
        self.out.text.len()
    }

    pub fn lines(&self, mark: usize) -> Vec<Span<usize>> {
        let text = &self.out.text;
        let mut prefixes = self.out.prefixes.iter().skip_while(|span| span.start < mark).peekable();
        let mut lines = vec![];
        let mut start = mark;
        loop {
            let end = text[start..].find('\n').map_or(text.len(), |at| start + at);
            while prefixes.next_if(|span| span.start < start).is_some() {}
            let content = prefixes.next_if(|span| span.start == start).map_or(start, |span| span.end.min(end));
            lines.push(content..end);
            if end == text.len() {
                return lines;
            }
            start = end + 1;
        }
    }

    pub fn since(&self, mark: usize) -> &str {
        &self.out.text[mark..]
    }

    pub fn style(&mut self, style: Style, value: Option<String>, mark: usize, source: Option<(Feature, Range)>) {
        let span = mark..self.out.text.len();
        self.out.styled.push(Styled { style, value, span, source, decoration: false });
    }

    pub fn decorate(&mut self, style: Style, mark: usize) {
        let span = mark..self.out.text.len();
        self.out.styled.push(Styled { style, value: None, span, source: None, decoration: true });
    }

    pub fn origin(&mut self, start: usize, range: Range) {
        let end = self.out.text.len();
        if end > start {
            self.out.origins.push((start..end, range));
        }
    }

    pub fn eligible(&mut self, span: Span<usize>) {
        if !span.is_empty() {
            self.out.eligible.push(span);
        }
    }

    pub fn suffix(&mut self, span: Span<usize>) {
        self.out.suffix = Some(span);
    }

    pub fn finish(self) -> Written {
        self.out
    }

    fn begin(&mut self) {
        if self.fresh {
            self.begin_with(self.prefixes.len());
            self.fresh = false;
            self.emitted = 0;
        }
    }

    fn blank(&mut self) {
        let last = self.prefixes.iter().rposition(|prefix| matches!(prefix, Prefix::Quote { .. }) || prefix.pending());
        if let Some(last) = last {
            self.begin_with(last + 1);
        }
    }

    fn begin_with(&mut self, count: usize) {
        let start = self.out.text.len();
        self.emit(self.emitted.min(count)..count);
        if self.out.text.len() > start {
            self.out.prefixes.push(start..self.out.text.len());
        }
    }

    fn emit(&mut self, range: Span<usize>) {
        let mut prefixes = std::mem::take(&mut self.prefixes);
        for prefix in &mut prefixes[range] {
            match prefix {
                Prefix::Quote { pending, mark } => {
                    *pending = false;
                    mark.chars().for_each(|char| self.put(char));
                }
                Prefix::Item { marker: marker @ Some(_), .. } => {
                    marker.take().unwrap().chars().for_each(|char| self.put(char));
                }
                Prefix::Item { width, marker: None } => (0..*width).for_each(|_| self.put(' ')),
            }
        }
        self.prefixes = prefixes;
    }

    fn hashtag(&self, after: &str) -> bool {
        self.out.text.chars().next_back().is_none_or(char::is_whitespace)
            && after.chars().next().is_some_and(|char| char.is_alphanumeric() || char == '_')
    }

    fn put(&mut self, char: char) {
        let text = &mut self.out.text;
        match (self.escape, char) {
            (
                Escape::LinkedIn,
                '|' | '{' | '}' | '@' | '[' | ']' | '(' | ')' | '<' | '>' | '#' | '\\' | '*' | '_' | '~',
            ) => {
                text.push('\\');
                text.push(char);
            }
            (Escape::Html | Escape::HtmlBreaks, '&') => text.push_str("&amp;"),
            (Escape::Html | Escape::HtmlBreaks, '<') => text.push_str("&lt;"),
            (Escape::Html | Escape::HtmlBreaks, '>') => text.push_str("&gt;"),
            (Escape::Angle, '<' | '>') => {
                text.push(if char == '<' { '‹' } else { '›' });
                if let Some(source) = self.source
                    && self.out.replaced.last() != Some(&source)
                {
                    self.out.replaced.push(source);
                }
            }
            _ => text.push(char),
        }
    }
}

pub(crate) fn blank(char: char) -> bool {
    char.is_whitespace() || char == '\u{feff}'
}

pub fn single_line(text: &str) -> String {
    text.split(blank).filter(|word| !word.is_empty()).collect::<Vec<_>>().join(" ")
}
