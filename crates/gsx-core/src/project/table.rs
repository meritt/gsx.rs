use std::borrow::Cow;
use std::fmt::Write as _;

use crate::document::{Layout, Node};

use super::Context;
use super::change::{Class, Feature, FieldName};
use super::facts::{Facts, Format};
use crate::measure::{self, MediaKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum When {
    LabelIsHref,
    LabelInvisible,
    TitleSource,
    Empty,
    Overflow,
    Capped,
    Nested,
    Info,
    Blocks,
    Depth,
    Columns,
    Group,
    Cut,
    Inline,
    Missing,
    Count,
    Spoiler,
    Truncated,
}

impl When {
    pub const fn name(self) -> &'static str {
        match self {
            When::LabelIsHref => "label_is_href",
            When::LabelInvisible => "label_invisible",
            When::TitleSource => "title_source",
            When::Empty => "empty",
            When::Overflow => "overflow",
            When::Capped => "capped",
            When::Nested => "nested",
            When::Info => "info",
            When::Blocks => "blocks",
            When::Depth => "depth",
            When::Columns => "columns",
            When::Group => "group",
            When::Cut => "cut",
            When::Inline => "inline",
            When::Missing => "missing",
            When::Count => "count",
            When::Spoiler => "spoiler",
            When::Truncated => "truncated",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Condition {
    pub when: When,
    pub class: Class,
    pub reason: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct Rule {
    pub feature: Feature,
    pub class: Class,
    pub reason: &'static str,
    pub conditions: &'static [Condition],
}

impl Rule {
    pub fn outcome(&self, when: Option<When>) -> (Class, &'static str) {
        when.and_then(|when| self.conditions.iter().find(|condition| condition.when == when))
            .map_or((self.class, self.reason), |condition| (condition.class, condition.reason))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Media {
    Any,
    Image,
    Video,
}

impl Media {
    pub fn applies(self, media: Option<MediaKind>) -> bool {
        match self {
            Media::Any => true,
            Media::Image => media == Some(MediaKind::Image),
            Media::Video => media == Some(MediaKind::Video),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Body,
    Title { fact: bool, first_line_removed: bool },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Truncation {
    Ellipsis,
    Hard,
}

#[derive(Clone, Copy, Debug)]
pub struct FieldSpec {
    pub name: FieldName,
    pub media: Media,
    pub source: Source,
    pub truncation: Truncation,
    pub required: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Search {
    Monotone,
    Exhaustive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Escape {
    None,
    LinkedIn,
    Html,
    HtmlBreaks,
    Angle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Offsets {
    Utf8,
    Utf16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Style {
    Bold,
    Italic,
    Underline,
    Strikethrough,
    Spoiler,
    Code,
    Link,
    Tag,
}

impl Style {
    pub const fn name(self) -> &'static str {
        match self {
            Style::Bold => "bold",
            Style::Italic => "italic",
            Style::Underline => "underline",
            Style::Strikethrough => "strikethrough",
            Style::Spoiler => "spoiler",
            Style::Code => "code",
            Style::Link => "link",
            Style::Tag => "tag",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Detection {
    None,
    Bluesky,
}

#[derive(Debug, PartialEq, Eq)]
pub struct EntityForm {
    pub offsets: Offsets,
    pub styles: &'static [(Feature, Style)],
    pub heading: Option<Style>,
    pub gated: bool,
    pub code_splits: bool,
    pub detection: Detection,
    pub cap: Option<usize>,
    pub trim: bool,
    pub styled_suffix: bool,
}

impl EntityForm {
    pub fn style(&self, feature: Feature) -> Option<Style> {
        self.styles.iter().find(|(native, _)| *native == feature).map(|(_, style)| *style)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HtmlBlocks {
    Message,
    Rich,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Structure {
    pub blocks: usize,
    pub depth: usize,
    pub columns: usize,
    pub items: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct HtmlForm {
    pub blocks: HtmlBlocks,
    pub structure: Option<Structure>,
    pub tags: &'static [(Feature, &'static str)],
    pub exclusive: &'static [Feature],
    pub code_contained: bool,
    pub quote_nests: bool,
    pub quote_contains_code: bool,
    pub code_info: bool,
    pub empty_removed: bool,
    pub trim: bool,
}

impl HtmlForm {
    pub fn tag(&self, feature: Feature) -> Option<&'static str> {
        self.tags.iter().find(|(native, _)| *native == feature).map(|(_, tag)| *tag)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineStart {
    Heading,
    Ordered,
    Setext,
}

#[derive(Debug, PartialEq, Eq)]
pub struct MarkdownForm {
    pub delimiters: &'static [(Feature, &'static str)],
    pub native: &'static [Feature],
    pub escaped: &'static str,
    pub owned: bool,
    pub neutralised: &'static [LineStart],
    pub html: bool,
    pub datetime: &'static str,
    pub separators: &'static str,
    pub quote_depth: Option<u32>,
}

impl MarkdownForm {
    pub fn delimiter(&self, feature: Feature) -> Option<&'static str> {
        self.delimiters.iter().find(|(native, _)| *native == feature).map(|(_, delimiter)| *delimiter)
    }

    pub fn native(&self, feature: Feature) -> bool {
        self.native.contains(&feature)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct BlocksForm {
    pub text: &'static MarkdownForm,
    pub formats: &'static [Format],
    pub group: Option<(Layout, usize)>,
    pub items: Option<usize>,
    pub ceiling: Option<usize>,
    pub spoiler: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    Plain { escape: Escape },
    Entities(&'static EntityForm),
    Html(&'static HtmlForm),
    Markdown(&'static MarkdownForm),
    Blocks(&'static BlocksForm),
}

impl Form {
    pub const fn version(self) -> &'static str {
        match self {
            Form::Plain { .. } => "plain/1",
            Form::Entities(_) => "entities/1",
            Form::Html(_) => "html/1",
            Form::Markdown(_) => "markdown/1",
            Form::Blocks(_) => "blocks/1",
        }
    }

    pub const fn escape(self) -> Escape {
        match self {
            Form::Plain { escape } => escape,
            Form::Entities(_) | Form::Markdown(_) | Form::Blocks(_) => Escape::None,
            Form::Html(HtmlForm { blocks: HtmlBlocks::Rich, .. }) => Escape::HtmlBreaks,
            Form::Html(_) => Escape::Html,
        }
    }

    pub const fn entities(self) -> Option<&'static EntityForm> {
        match self {
            Form::Entities(form) => Some(form),
            _ => None,
        }
    }

    pub const fn html(self) -> Option<&'static HtmlForm> {
        match self {
            Form::Html(form) => Some(form),
            _ => None,
        }
    }

    pub const fn markdown(self) -> Option<&'static MarkdownForm> {
        match self {
            Form::Markdown(form) => Some(form),
            Form::Blocks(form) => Some(form.text),
            _ => None,
        }
    }

    pub const fn blocks(self) -> Option<&'static BlocksForm> {
        match self {
            Form::Blocks(form) => Some(form),
            _ => None,
        }
    }

    pub fn visible(self, text: &str) -> Cow<'_, str> {
        if self.html().is_none() {
            return Cow::Borrowed(text);
        }
        let mut out = String::with_capacity(text.len());
        let mut rest = text;
        while let Some(at) = rest.find(['<', '&']) {
            out.push_str(&rest[..at]);
            rest = &rest[at..];
            if rest.starts_with('<') {
                let end = rest.find('>').map_or(rest.len(), |end| end + 1);
                if &rest[..end] == "<br>" {
                    out.push('\n');
                }
                rest = &rest[end..];
            } else {
                let (decoded, length) = [("&lt;", '<'), ("&gt;", '>'), ("&quot;", '"'), ("&amp;", '&')]
                    .into_iter()
                    .find(|(entity, _)| rest.starts_with(entity))
                    .map_or(('&', 1), |(entity, char)| (char, entity.len()));
                out.push(decoded);
                rest = &rest[length..];
            }
        }
        out.push_str(rest);
        Cow::Owned(out)
    }
}

#[derive(Clone, Copy)]
pub struct Renderer {
    pub blocks: fn(&mut Context, &[Node]),
    pub inline: fn(&mut Context, &[Node]),
}

impl std::fmt::Debug for Renderer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Renderer")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Normalization {
    None,
    Nfc,
}

#[derive(Debug)]
pub struct Target {
    pub name: &'static str,
    pub rules: &'static [Rule],
    pub form: Form,
    pub fields: &'static [FieldSpec],
    pub search: Search,
    pub normalization: Normalization,
    pub renderer: Renderer,
    pub limits: &'static [measure::Limit],
}

impl Target {
    pub fn applicable(
        &self,
        field: FieldName,
        context: measure::Context,
    ) -> impl Iterator<Item = &'static measure::Limit> + use<> {
        self.limits.iter().filter(move |limit| limit.applies(field, context))
    }

    pub fn measure<'a>(
        &self,
        field: FieldName,
        context: measure::Context,
        text: &'a str,
        facts: &'a measure::Facts<'a>,
    ) -> impl Iterator<Item = measure::Measurement> + use<'a> {
        self.applicable(field, context).map(move |limit| limit.measure(text, facts))
    }

    pub fn rule(&self, feature: Feature) -> &'static Rule {
        self.rules
            .iter()
            .find(|rule| rule.feature == feature)
            .unwrap_or_else(|| panic!("{} declares no rule for {}", self.name, feature.name()))
    }

    pub fn handles(&self, feature: Feature) -> bool {
        self.rules.iter().any(|rule| rule.feature == feature)
    }

    pub fn class(&self, feature: Feature) -> Class {
        self.rule(feature).class
    }

    pub fn fields(&self, facts: &Facts) -> impl Iterator<Item = &'static FieldSpec> + use<> {
        let media = facts.media;
        self.fields.iter().filter(move |field| field.media.applies(media))
    }

    pub fn declares(&self, feature: Feature, class: Class, reason: &str) -> bool {
        self.rules.iter().any(|rule| {
            rule.feature == feature
                && ((rule.class == class && rule.reason == reason)
                    || rule.conditions.iter().any(|condition| condition.class == class && condition.reason == reason))
        })
    }

    pub fn manifest(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "target {}\nform {} {:?}\nsearch {:?}",
            self.name,
            self.form.version(),
            self.form,
            self.search
        );
        if self.normalization == Normalization::Nfc {
            let _ = writeln!(out, "normalization nfc {}", super::nfc::UNICODE_VERSION);
        }
        for rule in self.rules {
            let _ = writeln!(out, "feature {} {} {}", rule.feature.name(), rule.class.name(), rule.reason);
            for condition in rule.conditions {
                let _ =
                    writeln!(out, "  when {} {} {}", condition.when.name(), condition.class.name(), condition.reason);
            }
        }
        for field in self.fields {
            let _ = writeln!(
                out,
                "field {} {:?} {:?} {:?} required={}",
                field.name.name(),
                field.media,
                field.source,
                field.truncation,
                field.required,
            );
            for limit in self.limits.iter().filter(|limit| limit.field == field.name) {
                let _ = writeln!(
                    out,
                    "  limit {} {:?} {:?} {:?} {:?}",
                    limit.unit.name(),
                    limit.maximum,
                    limit.placements,
                    limit.media,
                    limit.rendering
                );
            }
        }
        if self.form.blocks().is_some() {
            for limit in self.limits.iter().filter(|limit| limit.field == FieldName::Alt) {
                let _ = writeln!(out, "alt limit {} {:?} {:?}", limit.unit.name(), limit.maximum, limit.media);
            }
        }
        out
    }

    pub fn digest(&self) -> String {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        let version = env!("CARGO_PKG_VERSION").bytes();
        for byte in self.manifest().bytes().chain(measure::UNICODE_VERSION.bytes()).chain(version) {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
        }
        format!("{hash:016x}")
    }
}
