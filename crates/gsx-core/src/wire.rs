use std::fmt::Write;

use crate::document::{
    self, Align, Code, Details, Diagnostic, Document, Error, Kind, Layout, Marker, MarkerKind, Media, Node, Provenance,
    Range, Severity,
};
use crate::project::{
    self, Attachment, Block, Change, Entity, Facts, Field, Item, Measurement, MediaFact, Offsets, Part, Projection,
    Slot, Target, Truncated,
};
use crate::targets::TARGETS;

pub fn parse(input: &[u8], media: &[Media]) -> String {
    match decode(input) {
        Ok(text) => result(&document::parse(text, media)),
        Err(invalid) => invalid,
    }
}

fn parsed(input: &[u8], media: &[Media]) -> Result<Document, String> {
    document::parse(decode(input)?, media).map_err(|error| result(&Err(error)))
}

pub fn serialize(input: &[u8], media: &[Media]) -> String {
    let document = match parsed(input, media) {
        Ok(document) => document,
        Err(out) => return out,
    };
    let text = crate::serialize::serialize(&document);
    let mut out = Out(String::with_capacity(text.len() + 16));
    out.raw("{\"serialized\":");
    out.string(&text);
    out.0.push('}');
    out.0
}

pub fn project(input: &[u8], media: &[Media], target: &'static Target, facts: &Facts) -> String {
    let document = match parsed(input, media) {
        Ok(document) => document,
        Err(out) => return out,
    };
    let Projection { fields, measurements, changes, media, blocks, items, parts } =
        project::project(&document, target, facts);
    let mut out = Out(String::with_capacity(256));
    out.raw("{\"projection\":{\"fields\":");
    out.list(&fields, Out::field);
    out.raw(",\"measurements\":");
    out.list(&measurements, Out::measurement);
    out.raw(",\"changes\":");
    out.list(&changes, Out::change);
    out.raw(",\"media\":");
    out.list(&media, |out, id| out.string(id));
    if target.form.blocks().is_some() {
        out.raw(",\"blocks\":");
        out.list(&blocks, Out::block);
    }
    if !items.is_empty() {
        out.raw(",\"items\":");
        out.list(&items, Out::attachment);
    }
    if !parts.is_empty() {
        out.raw(",\"parts\":");
        out.list(&parts, |out, part| {
            let Part { fields, measurements, media, items, range, boundary } = part;
            out.raw("{\"fields\":");
            out.list(fields, Out::field);
            out.raw(",\"measurements\":");
            out.list(measurements, Out::measurement);
            out.raw(",\"media\":");
            out.list(media, |out, id| out.string(id));
            out.raw(",\"items\":");
            out.list(items, Out::attachment);
            out.raw(",\"range\":");
            out.range(*range);
            out.raw(",\"boundary\":");
            match boundary {
                Some(boundary) => out.string(boundary.name()),
                None => out.raw("null"),
            }
            out.0.push('}');
        });
    }
    out.raw("}}");
    out.0
}

pub fn targets() -> String {
    let mut out = Out(String::with_capacity(16_384));
    out.raw("{\"targets\":");
    out.list(&TARGETS, |out, target| {
        out.raw("{\"name\":");
        out.string(target.name);
        out.key("form");
        out.string(target.form.version());
        out.key("offsets");
        match target.form.entities().map(|form| form.offsets) {
            Some(Offsets::Utf8) => out.string("utf8"),
            Some(Offsets::Utf16) => out.string("utf16"),
            None => out.raw("null"),
        }
        out.key("digest");
        out.string(&target.digest());
        out.key("manifest");
        out.string(&target.manifest());
        out.key("codes");
        out.list(&target.codes(), |out, code| out.string(code));
        out.key("threads");
        out.raw(if target.threads() { "true" } else { "false" });
        out.0.push('}');
    });
    out.0.push('}');
    out.0
}

fn decode(input: &[u8]) -> Result<&str, String> {
    std::str::from_utf8(input).map_err(|error| {
        let (valid, rest) = input.split_at(error.valid_up_to());
        let offset = std::str::from_utf8(valid).unwrap_or_default().encode_utf16().count() as u32;
        let surrogate = match rest {
            [0xed, high @ 0xa0..=0xbf, low @ 0x80..=0xbf, ..] => {
                Some(0xd000 | u32::from(high & 0x3f) << 6 | u32::from(low & 0x3f))
            }
            _ => None,
        };
        invalid_text(offset, surrogate)
    })
}

pub fn refused_item(items: &[MediaFact]) -> Option<String> {
    let index = items.iter().position(|item| !document::is_media_id(item.id))?;
    Some(format!("item {index} id is not a media ID"))
}

pub fn invalid_text(offset: u32, code_point: Option<u32>) -> String {
    let range = Range { start: offset, end: offset + 1 };
    let details = code_point.map_or(Details::None, Details::CodePoint);
    result(&Err(Error { code: Code::MarkdownInvalidText, range: Some(range), details }))
}

pub fn result(result: &Result<Document, Error>) -> String {
    let mut out = Out(String::with_capacity(256));
    match result {
        Ok(document) => {
            out.raw("{\"document\":{\"children\":");
            out.list(document.children(), Out::node);
            out.raw(",\"markers\":");
            out.list(document.markers(), Out::marker);
            out.raw(",\"diagnostics\":");
            out.list(document.diagnostics(), Out::diagnostic);
            out.raw("}}");
        }
        Err(error) => {
            out.raw("{\"error\":{\"code\":");
            out.string(error.code.as_str());
            out.raw(",\"range\":");
            match error.range {
                Some(range) => out.range(range),
                None => out.raw("null"),
            }
            out.raw(",\"details\":");
            out.details(&error.details);
            out.raw("}}");
        }
    }
    out.0
}

struct Out(String);

impl Out {
    fn raw(&mut self, text: &str) {
        self.0.push_str(text);
    }

    fn number(&mut self, value: impl std::fmt::Display) {
        let _ = write!(self.0, "{value}");
    }

    fn bool(&mut self, value: bool) {
        self.raw(if value { "true" } else { "false" });
    }

    fn string(&mut self, value: &str) {
        self.0.push('"');
        let mut start = 0;
        for (at, byte) in value.bytes().enumerate() {
            let escape = match byte {
                b'"' => "\\\"",
                b'\\' => "\\\\",
                b'\n' => "\\n",
                b'\r' => "\\r",
                b'\t' => "\\t",
                0x08 => "\\b",
                0x0c => "\\f",
                0x00..=0x1f => "",
                _ => continue,
            };
            self.0.push_str(&value[start..at]);
            if escape.is_empty() {
                let _ = write!(self.0, "\\u{byte:04x}");
            } else {
                self.0.push_str(escape);
            }
            start = at + 1;
        }
        self.0.push_str(&value[start..]);
        self.0.push('"');
    }

    fn optional(&mut self, value: Option<&str>) {
        match value {
            Some(value) => self.string(value),
            None => self.raw("null"),
        }
    }

    fn key(&mut self, key: &str) {
        self.0.push(',');
        self.string(key);
        self.0.push(':');
    }

    fn list<T>(&mut self, items: &[T], mut each: impl FnMut(&mut Self, &T)) {
        self.0.push('[');
        for (index, item) in items.iter().enumerate() {
            if index > 0 {
                self.0.push(',');
            }
            each(self, item);
        }
        self.0.push(']');
    }

    fn range(&mut self, range: Range) {
        let _ = write!(self.0, "[{},{}]", range.start, range.end);
    }

    fn node(&mut self, node: &Node) {
        self.raw("{\"type\":");
        self.string(node_type(&node.kind));
        self.key("range");
        self.range(node.range);
        match &node.kind {
            Kind::Paragraph
            | Kind::ThematicBreak
            | Kind::Blockquote
            | Kind::TableRow
            | Kind::TableCell
            | Kind::Emphasis
            | Kind::Strong
            | Kind::Delete
            | Kind::Underline
            | Kind::Highlight
            | Kind::Spoiler
            | Kind::Break => {}
            Kind::Heading { depth } => {
                self.key("depth");
                self.number(depth);
            }
            Kind::List { ordered, start, spread } => {
                self.key("ordered");
                self.bool(*ordered);
                self.key("start");
                match start {
                    Some(start) => self.number(start),
                    None => self.raw("null"),
                }
                self.key("spread");
                self.bool(*spread);
            }
            Kind::ListItem { spread, checked } => {
                self.key("spread");
                self.bool(*spread);
                self.key("checked");
                match checked {
                    Some(checked) => self.bool(*checked),
                    None => self.raw("null"),
                }
            }
            Kind::Code { lang, meta, value } => {
                self.key("lang");
                self.optional(lang.as_deref());
                self.key("meta");
                self.optional(meta.as_deref());
                self.key("value");
                self.string(value);
            }
            Kind::Table { align } => {
                self.key("align");
                self.list(align, |out, align| match align {
                    Align::None => out.raw("null"),
                    Align::Left => out.string("left"),
                    Align::Right => out.string("right"),
                    Align::Center => out.string("center"),
                });
            }
            Kind::Text { value } | Kind::InlineCode { value } => {
                self.key("value");
                self.string(value);
            }
            Kind::Link { url, source, title } => {
                self.key("url");
                self.string(url);
                self.key("source");
                self.string(source);
                self.key("title");
                self.optional(title.as_deref());
            }
            Kind::Datetime { datetime, unix } => {
                self.key("datetime");
                self.optional(datetime.as_deref());
                self.key("unix");
                self.number(unix);
            }
            Kind::MediaReference { id } => {
                self.key("mediaId");
                self.string(id);
            }
            Kind::MediaGroup { ids, layout } => {
                self.key("mediaIds");
                self.list(ids, |out, id| out.string(id));
                self.key("layout");
                self.string(layout.as_str());
            }
        }
        if node.provenance == Provenance::Literal {
            self.key("provenance");
            self.string("literal");
        }
        if let Some(gap) = node.gap_before {
            self.key("gapBefore");
            self.number(gap);
        }
        if !node.children.is_empty() {
            self.key("children");
            self.list(&node.children, Out::node);
        }
        self.0.push('}');
    }

    fn marker(&mut self, marker: &Marker) {
        self.raw("{\"kind\":");
        self.string(marker_kind(marker.kind));
        self.key("range");
        self.range(marker.range);
        self.0.push('}');
    }

    fn diagnostic(&mut self, diagnostic: &Diagnostic) {
        self.raw("{\"code\":");
        self.string(diagnostic.code.as_str());
        self.key("severity");
        self.string(match diagnostic.severity {
            Severity::Warning => "warning",
            Severity::Error => "error",
        });
        self.key("range");
        self.range(diagnostic.range);
        self.key("details");
        self.details(&diagnostic.details);
        self.0.push('}');
    }

    fn maximum(&mut self, maximum: Option<usize>) {
        match maximum {
            Some(maximum) => self.number(maximum),
            None => self.raw("null"),
        }
    }

    fn field(&mut self, field: &Field) {
        let Field { name, text, links, entities } = field;
        self.raw("{\"name\":");
        self.string(name.name());
        self.key("text");
        self.string(text);
        self.key("links");
        self.list(links, |out, range| out.range(*range));
        self.key("entities");
        self.list(entities, |out, entity| {
            let Entity { style, range, value } = entity;
            out.raw("{\"style\":");
            out.string(style.name());
            out.key("range");
            out.range(*range);
            out.key("value");
            out.optional(value.as_deref());
            out.0.push('}');
        });
        self.0.push('}');
    }

    fn block(&mut self, block: &Block) {
        match block {
            Block::Text(text) => {
                self.raw("{\"text\":");
                self.string(text);
            }
            Block::Media { items, layout } => {
                self.raw("{\"items\":");
                self.list(items, Self::item);
                self.key("layout");
                self.optional(layout.map(Layout::as_str));
            }
            Block::File(item) => {
                self.raw("{\"file\":");
                self.item(item);
            }
            Block::Separator => self.raw("{\"separator\":true"),
        }
        self.0.push('}');
    }

    fn item(&mut self, item: &Item) {
        let Item { reference, alt, spoiler } = item;
        self.raw("{\"reference\":");
        self.string(reference);
        self.key("alt");
        self.optional(alt.as_deref());
        self.key("spoiler");
        self.bool(*spoiler);
        self.0.push('}');
    }

    fn attachment(&mut self, attachment: &Attachment) {
        let Attachment { reference, alt, spoiler } = attachment;
        self.raw("{\"reference\":");
        self.string(reference);
        self.key("alt");
        self.optional(alt.as_deref());
        self.key("spoiler");
        self.bool(*spoiler);
        self.0.push('}');
    }

    fn measurement(&mut self, measurement: &Measurement) {
        let Measurement { field, unit, value, maximum, block, item } = *measurement;
        self.raw("{\"field\":");
        self.string(field.name());
        self.key("unit");
        self.string(unit.name());
        self.key("value");
        self.number(value);
        self.key("maximum");
        self.maximum(maximum);
        if let Some(block) = block {
            self.key("block");
            self.number(block);
        }
        if let Some(item) = item {
            self.key("item");
            self.number(item);
        }
        self.0.push('}');
    }

    fn change(&mut self, change: &Change) {
        let Change { class, feature, reason, field, range, truncated, slot } = change;
        self.raw("{\"class\":");
        self.string(class.name());
        self.key("feature");
        self.string(feature.name());
        self.key("reason");
        self.string(reason);
        self.key("code");
        self.optional(change.code().as_deref());
        self.key("field");
        self.string(field.name());
        self.key("range");
        match range {
            Some(range) => self.range(*range),
            None => self.raw("null"),
        }
        self.key("truncated");
        match truncated {
            Some(Truncated { unit, maximum, value, exact }) => {
                self.raw("{\"unit\":");
                self.string(unit.name());
                self.key("maximum");
                self.number(maximum);
                self.key("value");
                self.number(value);
                self.key("exact");
                self.bool(*exact);
                self.0.push('}');
            }
            None => self.raw("null"),
        }
        if let Some(Slot { block, item }) = slot {
            if let Some(block) = block {
                self.key("block");
                self.number(block);
            }
            self.key("item");
            self.number(item);
        }
        self.0.push('}');
    }

    fn details(&mut self, details: &Details) {
        match details {
            Details::None => self.raw("{}"),
            Details::Scheme(scheme) => {
                self.raw("{\"scheme\":");
                self.string(scheme.as_str());
                self.0.push('}');
            }
            Details::Media { id, index } => {
                self.raw("{\"id\":");
                self.string(id);
                if let Some(index) = index {
                    self.key("index");
                    self.number(index);
                }
                self.0.push('}');
            }
            Details::Limit(limit) => {
                self.raw("{\"limit\":");
                self.string(limit.key());
                self.key("maximum");
                self.number(limit.maximum());
                self.0.push('}');
            }
            Details::CodePoint(code_point) => {
                self.raw("{\"codePoint\":");
                self.number(code_point);
                self.0.push('}');
            }
        }
    }
}

const fn node_type(kind: &Kind) -> &'static str {
    match kind {
        Kind::Paragraph => "paragraph",
        Kind::Heading { .. } => "heading",
        Kind::ThematicBreak => "thematicBreak",
        Kind::Blockquote => "blockquote",
        Kind::List { .. } => "list",
        Kind::ListItem { .. } => "listItem",
        Kind::Code { .. } => "code",
        Kind::Table { .. } => "table",
        Kind::TableRow => "tableRow",
        Kind::TableCell => "tableCell",
        Kind::Text { .. } => "text",
        Kind::Emphasis => "emphasis",
        Kind::Strong => "strong",
        Kind::Delete => "delete",
        Kind::Underline => "underline",
        Kind::Highlight => "highlight",
        Kind::Spoiler => "spoiler",
        Kind::InlineCode { .. } => "inlineCode",
        Kind::Break => "break",
        Kind::Link { .. } => "link",
        Kind::Datetime { .. } => "datetime",
        Kind::MediaReference { .. } => "mediaReference",
        Kind::MediaGroup { .. } => "mediaGroup",
    }
}

const fn marker_kind(kind: MarkerKind) -> &'static str {
    match kind {
        MarkerKind::Emphasis => "emphasis",
        MarkerKind::Strong => "strong",
        MarkerKind::Strikethrough => "strikethrough",
        MarkerKind::Spoiler => "spoiler",
        MarkerKind::Underline => "underline",
        MarkerKind::Highlight => "highlight",
        MarkerKind::CodeText => "codeText",
        MarkerKind::CodeFence => "codeFence",
        MarkerKind::Heading => "heading",
        MarkerKind::ThematicBreak => "thematicBreak",
        MarkerKind::BlockQuote => "blockQuote",
        MarkerKind::ListItem => "listItem",
        MarkerKind::TaskCheck => "taskCheck",
        MarkerKind::TableDelimiter => "tableDelimiter",
        MarkerKind::Link => "link",
        MarkerKind::Autolink => "autolink",
        MarkerKind::Escape => "escape",
        MarkerKind::CharacterReference => "characterReference",
        MarkerKind::HardBreak => "hardBreak",
        MarkerKind::Directive => "directive",
        MarkerKind::Media => "media",
    }
}
