use crate::project::Class::{Approximated, Blocked, Omitted, TextualFallback, Unwrapped};
use crate::project::{Class, Condition, Feature, Rule, Target, When};

const fn rule(feature: Feature, class: Class, reason: &'static str) -> Rule {
    Rule { feature, class, reason, conditions: &[] }
}

const LINK: Rule = Rule {
    feature: Feature::Link,
    class: TextualFallback,
    reason: "link_destination_as_text",
    conditions: &[Condition { when: When::LabelIsHref, class: Approximated, reason: "link_bare_url" }],
};
const TEXT: Rule = Rule {
    feature: Feature::Text,
    class: Approximated,
    reason: "text_truncated",
    conditions: &[
        Condition { when: When::Empty, class: Blocked, reason: "text_empty" },
        Condition { when: When::Overflow, class: Blocked, reason: "text_too_long" },
    ],
};

struct Rules {
    heading: Rule,
    blockquote: Rule,
    code_block: Rule,
    emphasis: Rule,
    strong: Rule,
    strikethrough: Rule,
    underline: Rule,
    highlight: Rule,
    spoiler: Rule,
    inline_code: Rule,
    link: Rule,
    link_title: Rule,
    datetime: Rule,
    text: Rule,
    media: Rule,
}

const DEFAULT: Rules = Rules {
    heading: rule(Feature::Heading, TextualFallback, "heading_as_text"),
    blockquote: rule(Feature::Blockquote, TextualFallback, "blockquote_as_text"),
    code_block: rule(Feature::CodeBlock, TextualFallback, "code_block_as_text"),
    emphasis: rule(Feature::Emphasis, Unwrapped, "emphasis_unwrapped"),
    strong: rule(Feature::Strong, Unwrapped, "strong_unwrapped"),
    strikethrough: rule(Feature::Strikethrough, Unwrapped, "strikethrough_unwrapped"),
    underline: rule(Feature::Underline, Unwrapped, "underline_unwrapped"),
    highlight: rule(Feature::Highlight, Unwrapped, "highlight_unwrapped"),
    spoiler: rule(Feature::Spoiler, Unwrapped, "spoiler_unwrapped"),
    inline_code: rule(Feature::InlineCode, Unwrapped, "inline_code_unwrapped"),
    link: LINK,
    link_title: rule(Feature::LinkTitle, Omitted, "link_title_omitted"),
    datetime: rule(Feature::Datetime, TextualFallback, "datetime_label_as_text"),
    text: TEXT,
    media: Rule {
        feature: Feature::Media,
        class: Approximated,
        reason: "media_attached",
        conditions: &[
            Condition { when: When::Truncated, class: Approximated, reason: "media_alt_truncated" },
            Condition { when: When::Overflow, class: Omitted, reason: "media_alt_omitted" },
            Condition { when: When::Missing, class: Omitted, reason: "media_missing" },
        ],
    },
};

macro_rules! rules {
    ($rules:expr $(, $extra:expr)* $(,)?) => {{
        const RULES: Rules = $rules;
        &[
            RULES.heading,
            rule(Feature::ThematicBreak, TextualFallback, "thematic_break_as_text"),
            RULES.blockquote,
            rule(Feature::BulletList, TextualFallback, "bullet_list_as_text"),
            rule(Feature::OrderedList, TextualFallback, "ordered_list_as_text"),
            rule(Feature::CompoundList, TextualFallback, "compound_list_as_text"),
            RULES.code_block,
            rule(Feature::Table, TextualFallback, "table_as_text"),
            rule(Feature::TaskList, TextualFallback, "task_list_as_text"),
            RULES.media,
            RULES.emphasis,
            RULES.strong,
            RULES.strikethrough,
            RULES.underline,
            RULES.highlight,
            RULES.spoiler,
            RULES.inline_code,
            RULES.link,
            RULES.link_title,
            RULES.datetime,
            RULES.text,
            $($extra,)*
        ]
    }};
}

mod limits {
    pub(super) use crate::measure::Condition::{Only, With, Without};
    pub(super) use crate::measure::Field::{Alt, Description, Text, Title};
    pub(super) use crate::measure::MediaKind::{Image, Video};
    pub(super) use crate::measure::Placement::{Post, Reel, Story};
    pub(super) use crate::measure::Unit::{
        CodePoints, Graphemes, Hashtags, MastodonWeighted, Mentions, ThreadsWeighted, Utf8Bytes, Utf16CodeUnits,
        XWeighted,
    };
    pub(super) use crate::measure::{Fact, Limit, given, limit, resolved, unlimited};
}

mod blocks;
mod entities;
mod html;
mod markdown;
mod plain;

pub static TARGETS: [&Target; 19] = [
    &plain::X,
    &plain::MASTODON,
    &plain::LINKEDIN,
    &plain::FACEBOOK,
    &plain::INSTAGRAM,
    &plain::VK,
    &plain::YOUTUBE,
    &plain::TIKTOK,
    &plain::SLACK,
    &entities::BLUESKY,
    &entities::THREADS,
    &entities::TELEGRAM,
    &html::TELEGRAM,
    &html::TELEGRAM_RICH,
    &html::MAX,
    &markdown::DISCORD,
    &markdown::SLACK,
    &blocks::DISCORD,
    &blocks::SLACK,
];

pub fn target(name: &str) -> Option<&'static Target> {
    TARGETS.iter().copied().find(|target| target.name == name)
}
