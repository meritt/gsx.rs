use std::borrow::Cow;
use std::fmt::Write as _;

use crate::document::{Layout, MEDIA_GROUP_MAX, MEDIA_GROUP_MIN, Node};

use super::Context;
use super::change::{Class, Feature, FieldName};
use super::facts::{Facts, Format, MediaFact};
use crate::measure::{self, Fact, Maximum, MediaKind, Placement, Unit};

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
    Refused,
    Thread,
    Code,
    Post,
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
            When::Refused => "refused",
            When::Thread => "thread",
            When::Code => "code",
            When::Post => "post",
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
    pub const fn name(self) -> &'static str {
        match self {
            Media::Any => "any",
            Media::Image => "image",
            Media::Video => "video",
        }
    }

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
    Title { fact: bool },
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

impl Search {
    pub const fn name(self) -> &'static str {
        match self {
            Search::Monotone => "monotone",
            Search::Exhaustive => "exhaustive",
        }
    }
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
    Pre,
    Blockquote,
    Date,
    Mention,
}

const DATE_LABEL: usize = 127;

pub fn date_refusal(label: &str, unix: i64) -> Option<When> {
    if label.encode_utf16().count() > DATE_LABEL {
        Some(When::Overflow)
    } else if !(0..=i64::from(i32::MAX)).contains(&unix) {
        Some(When::Refused)
    } else {
        None
    }
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
            Style::Pre => "pre",
            Style::Blockquote => "blockquote",
            Style::Date => "date",
            Style::Mention => "mention",
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
    pub links: Option<usize>,
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
    pub groups: &'static [(Layout, &'static [Format])],
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

    pub fn groups(&self, layout: Layout) -> &'static [Format] {
        self.groups.iter().find(|(native, _)| *native == layout).map_or(&[], |(_, formats)| formats)
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
    pub url_labels: bool,
}

impl MarkdownForm {
    pub fn delimiter(&self, feature: Feature) -> Option<&'static str> {
        self.delimiters.iter().find(|(native, _)| *native == feature).map(|(_, delimiter)| *delimiter)
    }

    pub fn native(&self, feature: Feature) -> bool {
        self.native.contains(&feature)
    }

    fn describe(&self, out: &mut String) {
        let delimiters = join(
            self.delimiters.iter().map(|(feature, delimiter)| format!("{}={}", feature.name(), quoted(delimiter))),
        );
        let neutralised = join(self.neutralised.iter().map(|start| match start {
            LineStart::Heading => "heading",
            LineStart::Ordered => "ordered",
            LineStart::Setext => "setext",
        }));
        let quote_depth = self.quote_depth.map_or_else(|| "none".to_owned(), |depth| depth.to_string());
        let _ = writeln!(
            out,
            "  delimiters {delimiters}\n  native {}\n  escaped {}\n  owned {}\n  neutralised {neutralised}\n  html {}\n  \
             datetime {}\n  separators {}\n  quote_depth {quote_depth}\n  url_labels {}",
            join(self.native.iter().map(|feature| feature.name())),
            quoted(self.escaped),
            self.owned,
            self.html,
            quoted(self.datetime),
            quoted(self.separators),
            self.url_labels,
        );
    }
}

fn join<T: AsRef<str>>(items: impl Iterator<Item = T>) -> String {
    let mut out = String::new();
    for item in items {
        if !out.is_empty() {
            out.push(',');
        }
        out.push_str(item.as_ref());
    }
    if out.is_empty() { "none".to_owned() } else { out }
}

fn optional(value: Option<usize>) -> String {
    value.map_or_else(|| "none".to_owned(), |value| value.to_string())
}

fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for char in text.chars() {
        match char {
            '"' | '\\' => {
                out.push('\\');
                out.push(char);
            }
            '\n' => out.push_str("\\n"),
            _ => out.push(char),
        }
    }
    out.push('"');
    out
}

#[derive(Debug, PartialEq, Eq)]
pub struct BlocksForm {
    pub text: &'static MarkdownForm,
    pub formats: &'static [Format],
    pub group: Option<(Layout, usize)>,
    pub items: Option<usize>,
    pub ceiling: Option<Ceiling>,
    pub spoiler: bool,
    pub separator: bool,
    pub unplaced: Unplaced,
}

impl BlocksForm {
    pub fn places(&self, fact: &MediaFact) -> bool {
        fact.inline && self.formats.contains(&fact.format)
    }

    pub fn shows(&self, fact: &MediaFact) -> bool {
        self.places(fact) || self.unplaced == Unplaced::File
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ceiling {
    pub blocks: usize,
    pub per_item: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unplaced {
    File,
    Attach,
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

    fn describe(self, out: &mut String) {
        match self {
            Form::Plain { escape } => {
                let escape = match escape {
                    Escape::None => "none",
                    Escape::LinkedIn => "linkedin",
                    Escape::Html => "html",
                    Escape::HtmlBreaks => "html-breaks",
                    Escape::Angle => "angle",
                };
                let _ = writeln!(out, "  escape {escape}");
            }
            Form::Entities(form) => {
                let offsets = match form.offsets {
                    Offsets::Utf8 => "utf8",
                    Offsets::Utf16 => "utf16",
                };
                let styles =
                    join(form.styles.iter().map(|(feature, style)| format!("{}={}", feature.name(), style.name())));
                let detection = match form.detection {
                    Detection::None => "none",
                    Detection::Bluesky => "bluesky",
                };
                let _ = writeln!(
                    out,
                    "  offsets {offsets}\n  styles {styles}\n  heading {}\n  gated {}\n  code_splits {}\n  detection \
                     {detection}\n  cap {}\n  links {}\n  trim {}\n  styled_suffix {}",
                    form.heading.map_or("none", Style::name),
                    form.gated,
                    form.code_splits,
                    optional(form.cap),
                    optional(form.links),
                    form.trim,
                    form.styled_suffix,
                );
            }
            Form::Html(form) => {
                let blocks = match form.blocks {
                    HtmlBlocks::Message => "message",
                    HtmlBlocks::Rich => "rich",
                };
                let structure = form.structure.map_or_else(
                    || "none".to_owned(),
                    |Structure { blocks, depth, columns, items }| {
                        format!("blocks={blocks} depth={depth} columns={columns} items={items}")
                    },
                );
                let tags = join(form.tags.iter().map(|(feature, tag)| format!("{}={tag}", feature.name())));
                let groups = join(form.groups.iter().map(|(layout, formats)| {
                    let formats: Vec<_> = formats.iter().map(|format| format.name()).collect();
                    format!("{}={}", layout.as_str(), formats.join("+"))
                }));
                let _ = writeln!(
                    out,
                    "  blocks {blocks}\n  structure {structure}\n  tags {tags}\n  exclusive {}\n  groups {groups}\n  \
                     code_contained {}\n  quote_nests {}\n  quote_contains_code {}\n  code_info {}\n  empty_removed {}\n  \
                     trim {}",
                    join(form.exclusive.iter().map(|feature| feature.name())),
                    form.code_contained,
                    form.quote_nests,
                    form.quote_contains_code,
                    form.code_info,
                    form.empty_removed,
                    form.trim,
                );
            }
            Form::Markdown(form) => form.describe(out),
            Form::Blocks(form) => {
                form.text.describe(out);
                let group = form.group.map_or_else(
                    || "none".to_owned(),
                    |(layout, maximum)| format!("{} maximum={maximum}", layout.as_str()),
                );
                let ceiling = form.ceiling.map_or_else(
                    || "none".to_owned(),
                    |Ceiling { blocks, per_item }| format!("blocks={blocks} per_item={per_item}"),
                );
                let unplaced = match form.unplaced {
                    Unplaced::File => "file",
                    Unplaced::Attach => "attach",
                };
                let _ = writeln!(
                    out,
                    "  formats {}\n  group {group}\n  items {}\n  ceiling {ceiling}\n  spoiler {}\n  separator {}\n  \
                     unplaced {unplaced}",
                    join(form.formats.iter().map(|format| format.name())),
                    optional(form.items),
                    form.spoiler,
                    form.separator,
                );
            }
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
    pub spoilers: Spoilers,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spoilers {
    None,
    Items(&'static [Format]),
    Post,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpoilerMode {
    Carried,
    Widened,
    Omitted,
}

impl SpoilerMode {
    pub const fn name(self) -> &'static str {
        match self {
            SpoilerMode::Carried => "carried",
            SpoilerMode::Widened => "widened",
            SpoilerMode::Omitted => "omitted",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Group {
    pub layout: Layout,
    pub formats: &'static [Format],
    pub minimum: usize,
    pub maximum: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaShape {
    pub formats: &'static [Format],
    pub alt: &'static [Format],
    pub spoiler: (SpoilerMode, &'static [Format]),
    pub inline: bool,
    pub groups: Vec<Group>,
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

    pub fn codes(&self) -> Vec<String> {
        let mut codes: Vec<String> = vec![];
        let blocked = self.rules.iter().flat_map(|rule| {
            std::iter::once((rule.class, rule.reason))
                .chain(rule.conditions.iter().map(|condition| (condition.class, condition.reason)))
        });
        for (_, reason) in blocked.filter(|(class, _)| *class == Class::Blocked) {
            let code = reason.to_ascii_uppercase();
            if !codes.contains(&code) {
                codes.push(code);
            }
        }
        codes
    }

    pub fn threads(&self) -> bool {
        self.rules.iter().any(|rule| {
            rule.feature == Feature::Text && rule.conditions.iter().any(|condition| condition.when == When::Thread)
        })
    }

    pub fn facts(&self) -> Vec<&'static str> {
        let bodies = || self.limits.iter().filter(|limit| limit.field != FieldName::Alt);
        let limited = |fact: Fact| {
            self.limits
                .iter()
                .any(|limit| matches!(limit.maximum, Maximum::Resolved(read, _) | Maximum::Given(read) if read == fact))
        };
        let weighted = self.limits.iter().any(|limit| limit.unit == Unit::MastodonWeighted);
        let empty = self.handles(Feature::Text)
            && self.rule(Feature::Text).conditions.iter().any(|condition| condition.when == When::Empty);
        [
            (
                "media",
                bodies().any(|limit| limit.media != measure::Condition::Any)
                    || self.fields.iter().any(|field| field.media != Media::Any),
            ),
            ("mediaCount", empty),
            (
                "placement",
                self.threads()
                    || self.spoilers != Spoilers::None
                    || bodies().any(|limit| limit.placements != Placement::ALL),
            ),
            ("title", self.fields.iter().any(|field| field.source == Source::Title { fact: true })),
            ("targetUrl", self.handles(Feature::TargetUrl)),
            ("maxCharacters", limited(Fact::MaxCharacters)),
            ("charactersReservedPerUrl", weighted),
            ("captionLimit", limited(Fact::CaptionLimit)),
            ("descriptionLimit", limited(Fact::DescriptionLimit)),
            ("hashtagLimit", limited(Fact::HashtagLimit)),
            ("spoilerText", weighted),
            ("overflow", self.threads()),
            ("entitiesAllowed", self.form.entities().is_some_and(|form| form.gated)),
            ("items", true),
        ]
        .into_iter()
        .filter_map(|(name, read)| read.then_some(name))
        .collect()
    }

    pub fn media(&self) -> MediaShape {
        match self.form {
            Form::Blocks(form) => MediaShape {
                formats: form.formats,
                alt: form.formats,
                spoiler: if form.spoiler { (SpoilerMode::Carried, Format::ALL) } else { (SpoilerMode::Omitted, &[]) },
                inline: true,
                groups: form
                    .group
                    .map(|(layout, maximum)| Group {
                        layout,
                        formats: form.formats,
                        minimum: MEDIA_GROUP_MIN,
                        maximum: maximum.min(MEDIA_GROUP_MAX),
                    })
                    .into_iter()
                    .collect(),
            },
            _ => MediaShape {
                formats: Format::ALL,
                alt: Format::ALL,
                spoiler: match self.spoilers {
                    Spoilers::None => (SpoilerMode::Omitted, &[]),
                    Spoilers::Items(formats) => (SpoilerMode::Carried, formats),
                    Spoilers::Post => (SpoilerMode::Widened, Format::ALL),
                },
                inline: false,
                groups: self.form.html().map_or_else(Vec::new, |form| {
                    let maximum = form.structure.map_or(MEDIA_GROUP_MAX, |structure| structure.items);
                    form.groups
                        .iter()
                        .map(|&(layout, formats)| Group {
                            layout,
                            formats,
                            minimum: MEDIA_GROUP_MIN,
                            maximum: maximum.min(MEDIA_GROUP_MAX),
                        })
                        .collect()
                }),
            },
        }
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
        let _ = writeln!(out, "target {}\nform {}", self.name, self.form.version());
        self.form.describe(&mut out);
        let _ = writeln!(out, "search {}", self.search.name());
        if self.normalization == Normalization::Nfc {
            let _ = writeln!(out, "normalization nfc {}", super::nfc::UNICODE_VERSION);
        }
        match self.spoilers {
            Spoilers::None => {}
            Spoilers::Items(formats) => {
                let _ = writeln!(out, "spoilers items {}", join(formats.iter().map(|format| format.name())));
            }
            Spoilers::Post => out.push_str("spoilers post\n"),
        }
        for rule in self.rules {
            let _ = writeln!(out, "feature {} {} {}", rule.feature.name(), rule.class.name(), rule.reason);
            for condition in rule.conditions {
                let _ =
                    writeln!(out, "  when {} {} {}", condition.when.name(), condition.class.name(), condition.reason);
            }
        }
        for field in self.fields {
            let source = match field.source {
                Source::Body => "body",
                Source::Title { fact: true } => "title-fact",
                Source::Title { fact: false } => "title",
            };
            let truncation = match field.truncation {
                Truncation::Ellipsis => "ellipsis",
                Truncation::Hard => "hard",
            };
            let _ = writeln!(
                out,
                "field {} media={} source={source} truncation={truncation} required={}",
                field.name.name(),
                field.media.name(),
                field.required,
            );
        }
        for limit in self.limits {
            let maximum = match limit.maximum {
                measure::Maximum::Fixed(value) => value.to_string(),
                measure::Maximum::Resolved(fact, value) => format!("{value} fact={}", fact.name()),
                measure::Maximum::Given(fact) => format!("none fact={}", fact.name()),
                measure::Maximum::Unlimited => "none".to_owned(),
            };
            let _ = writeln!(
                out,
                "limit {} {} maximum={maximum} placements={} media={} rendering={}",
                limit.field.name(),
                limit.unit.name(),
                join(limit.placements.iter().map(|placement| placement.name())),
                limit.media.name(),
                limit.rendering.name(),
            );
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
