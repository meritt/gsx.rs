mod convert;
mod markers;
mod time;
mod tree;

use std::borrow::Cow;
use std::fmt;

pub use crate::source::{Limit, Point};
pub use crate::syntax::Align;

use crate::source::{self, LineIndex};
use crate::syntax;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Media<'a> {
    pub id: &'a str,
    pub cover: Option<&'a str>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Range {
    pub start: u32,
    pub end: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Provenance {
    Authored,
    Literal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Layout {
    Collage,
    Slideshow,
}

impl Layout {
    pub const fn as_str(self) -> &'static str {
        match self {
            Layout::Collage => "collage",
            Layout::Slideshow => "slideshow",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Paragraph,
    Heading { depth: u8 },
    ThematicBreak,
    Blockquote,
    List { ordered: bool, start: Option<u32>, spread: bool },
    ListItem { spread: bool, checked: Option<bool> },
    Code { lang: Option<String>, meta: Option<String>, value: String },
    Table { align: Vec<Align> },
    TableRow,
    TableCell,
    Text { value: String },
    Emphasis,
    Strong,
    Delete,
    Underline,
    Highlight,
    Spoiler,
    InlineCode { value: String },
    Break,
    Link { url: String, source: String, title: Option<String> },
    Datetime { datetime: Option<String>, unix: i64 },
    MediaReference { id: String },
    MediaGroup { ids: Vec<String>, layout: Layout },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Node {
    pub kind: Kind,
    pub range: Range,
    pub provenance: Provenance,
    pub gap_before: Option<u32>,
    pub children: Vec<Node>,
}

impl Node {
    pub(crate) fn media_ids(&self) -> Option<&[String]> {
        match &self.kind {
            Kind::MediaReference { id } => Some(std::slice::from_ref(id)),
            Kind::MediaGroup { ids, .. } => Some(ids),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MarkerKind {
    Emphasis,
    Strong,
    Strikethrough,
    Spoiler,
    Underline,
    Highlight,
    CodeText,
    CodeFence,
    Heading,
    ThematicBreak,
    BlockQuote,
    ListItem,
    TaskCheck,
    TableDelimiter,
    Link,
    Autolink,
    Escape,
    CharacterReference,
    HardBreak,
    Directive,
    Media,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Marker {
    pub kind: MarkerKind,
    pub range: Range,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Code {
    DatetimeInvalid,
    DatetimeTimezoneRequired,
    LinkLiteralized,
    MarkdownInvalidText,
    MarkdownMalformed,
    MarkdownTooComplex,
    MediaDirectiveInvalid,
    MediaReferenceContextUnsupported,
    MediaReferenceCover,
    MediaReferenceDuplicate,
    MediaReferenceUnknown,
}

impl Code {
    pub const fn as_str(self) -> &'static str {
        match self {
            Code::DatetimeInvalid => "DATETIME_INVALID",
            Code::DatetimeTimezoneRequired => "DATETIME_TIMEZONE_REQUIRED",
            Code::LinkLiteralized => "LINK_LITERALIZED",
            Code::MarkdownInvalidText => "MARKDOWN_INVALID_TEXT",
            Code::MarkdownMalformed => "MARKDOWN_MALFORMED",
            Code::MarkdownTooComplex => "MARKDOWN_TOO_COMPLEX",
            Code::MediaDirectiveInvalid => "MEDIA_DIRECTIVE_INVALID",
            Code::MediaReferenceContextUnsupported => "MEDIA_REFERENCE_CONTEXT_UNSUPPORTED",
            Code::MediaReferenceCover => "MEDIA_REFERENCE_COVER",
            Code::MediaReferenceDuplicate => "MEDIA_REFERENCE_DUPLICATE",
            Code::MediaReferenceUnknown => "MEDIA_REFERENCE_UNKNOWN",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scheme {
    Http,
    Https,
    Relative,
    Unsupported,
}

impl Scheme {
    pub const fn as_str(self) -> &'static str {
        match self {
            Scheme::Http => "http",
            Scheme::Https => "https",
            Scheme::Relative => "relative",
            Scheme::Unsupported => "unsupported",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Details {
    None,
    Scheme(Scheme),
    Media { id: String, index: Option<u32> },
    Limit(Limit),
    CodePoint(u32),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Diagnostic {
    pub code: Code,
    pub severity: Severity,
    pub range: Range,
    pub details: Details,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Error {
    pub code: Code,
    pub range: Option<Range>,
    pub details: Details,
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code.as_str())?;
        if let Some(range) = self.range {
            write!(formatter, " at {}..{}", range.start, range.end)?;
        }
        match self.details {
            Details::Limit(limit) => write!(formatter, " ({} > {})", limit.key(), limit.maximum())?,
            Details::CodePoint(code_point) => write!(formatter, " (U+{code_point:04X})")?,
            _ => {}
        }
        Ok(())
    }
}

impl std::error::Error for Error {}

#[derive(Clone, Debug)]
pub struct Document {
    text: String,
    input: Option<String>,
    lines: LineIndex,
    children: Vec<Node>,
    markers: Vec<Marker>,
    diagnostics: Vec<Diagnostic>,
}

impl Document {
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn children(&self) -> &[Node] {
        &self.children
    }

    pub fn markers(&self) -> &[Marker] {
        &self.markers
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn point(&self, offset: u32) -> Point {
        self.lines.point(offset)
    }

    pub fn end(&self) -> Point {
        self.lines.point(self.lines.len())
    }

    pub fn source(&self, range: Range) -> &str {
        let (start, end) = (self.lines.byte(range.start), self.lines.byte(range.end));
        match &self.input {
            Some(input) => &input[self.lines.input_byte(start)..self.lines.input_byte(end)],
            None => &self.text[start..end],
        }
    }
}

pub fn parse(input: &str, media: &[Media]) -> Result<Document, Error> {
    let error = |code, range, details| Error { code, range, details };
    let normalized = source::normalize(input).map_err(|invalid| {
        let range = Range { start: invalid.start, end: invalid.start + 1 };
        error(Code::MarkdownInvalidText, Some(range), Details::CodePoint(invalid.code_point))
    })?;
    let lines = LineIndex::new(&normalized, input);
    let (text, input) = match normalized {
        Cow::Borrowed(text) => (text.to_owned(), None),
        Cow::Owned(text) => (text, Some(input.to_owned())),
    };
    let events = syntax::tokenize(&text).map_err(|exceeded| {
        let range = Range { start: lines.utf16(exceeded.start), end: lines.utf16(exceeded.end) };
        error(Code::MarkdownTooComplex, Some(range), Details::Limit(exceeded.limit))
    })?;
    let (root, prefixes) = tree::build(&events, text.as_bytes());
    let (children, diagnostics) = convert::convert(&root, &prefixes, &text, &lines, media);
    let markers = markers::collect(&events, text.as_bytes(), &lines);
    Ok(Document { text, input, lines, children, markers, diagnostics })
}
