use crate::document::Range;

use crate::measure::Unit;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Class {
    Approximated,
    TextualFallback,
    Unwrapped,
    Omitted,
    Blocked,
}

impl Class {
    pub const fn name(self) -> &'static str {
        match self {
            Class::Approximated => "approximated",
            Class::TextualFallback => "textual_fallback",
            Class::Unwrapped => "unwrapped",
            Class::Omitted => "omitted",
            Class::Blocked => "blocked",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Feature {
    Heading,
    ThematicBreak,
    Blockquote,
    BulletList,
    OrderedList,
    CompoundList,
    CodeBlock,
    Table,
    TaskList,
    Media,
    Emphasis,
    Strong,
    Strikethrough,
    Underline,
    Highlight,
    Spoiler,
    InlineCode,
    Link,
    LinkTitle,
    Datetime,
    AngleBrackets,
    Text,
    Title,
    TargetUrl,
}

impl Feature {
    pub const fn name(self) -> &'static str {
        match self {
            Feature::Heading => "heading",
            Feature::ThematicBreak => "thematic_break",
            Feature::Blockquote => "blockquote",
            Feature::BulletList => "bullet_list",
            Feature::OrderedList => "ordered_list",
            Feature::CompoundList => "compound_list",
            Feature::CodeBlock => "code_block",
            Feature::Table => "table",
            Feature::TaskList => "task_list",
            Feature::Media => "media",
            Feature::Emphasis => "emphasis",
            Feature::Strong => "strong",
            Feature::Strikethrough => "strikethrough",
            Feature::Underline => "underline",
            Feature::Highlight => "highlight",
            Feature::Spoiler => "spoiler",
            Feature::InlineCode => "inline_code",
            Feature::Link => "link",
            Feature::LinkTitle => "link_title",
            Feature::Datetime => "datetime",
            Feature::AngleBrackets => "angle_brackets",
            Feature::Text => "text",
            Feature::Title => "title",
            Feature::TargetUrl => "target_url",
        }
    }
}

pub use crate::measure::Field as FieldName;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Truncated {
    pub unit: Unit,
    pub maximum: usize,
    pub value: usize,
    pub exact: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub class: Class,
    pub feature: Feature,
    pub reason: &'static str,
    pub field: FieldName,
    pub range: Option<Range>,
    pub truncated: Option<Truncated>,
}

impl Change {
    pub fn code(&self) -> Option<String> {
        (self.class == Class::Blocked).then(|| self.reason.to_ascii_uppercase())
    }
}
