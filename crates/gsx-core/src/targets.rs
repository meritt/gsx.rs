use crate::project::Class::{Approximated, Blocked, Omitted, TextualFallback, Unwrapped};
use crate::project::{Class, Condition, Feature, Rule, Target, TimeFormats, When};

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
        Condition { when: When::BlankLines, class: Approximated, reason: "blank_lines_collapsed" },
    ],
};
const THREADED_TEXT: Rule = Rule {
    conditions: &[
        TEXT.conditions[0],
        TEXT.conditions[1],
        TEXT.conditions[2],
        Condition { when: When::Thread, class: Approximated, reason: "text_threaded" },
    ],
    ..TEXT
};

const MATH: Rule = rule(Feature::Math, TextualFallback, "math_as_text");

const TELEGRAM_TIME_FORMATS: TimeFormats = &["t", "T", "d", "D", "dt", "Dt", "wDt", "r"];

const DATE_BOUNDS: [Condition; 2] = [
    Condition { when: When::Overflow, class: Unwrapped, reason: "datetime_label_too_long" },
    Condition { when: When::Refused, class: Unwrapped, reason: "datetime_out_of_range" },
];

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
        conditions: &[ATTACHED[0], ATTACHED[1], ATTACHED[2], SPOILER_OMITTED],
    },
};

const ATTACHED: [Condition; 3] = [
    Condition { when: When::Truncated, class: Approximated, reason: "media_alt_truncated" },
    Condition { when: When::Overflow, class: Omitted, reason: "media_alt_omitted" },
    Condition { when: When::Missing, class: Omitted, reason: "media_missing" },
];

const SPOILER_OMITTED: Condition = Condition { when: When::Spoiler, class: Omitted, reason: "media_spoiler_omitted" };

const SPOILERS_CARRIED: Rule = Rule { conditions: &ATTACHED, ..DEFAULT.media };

const ALT_REFUSED: Rule = Rule {
    conditions: &[
        ATTACHED[0],
        ATTACHED[1],
        ATTACHED[2],
        SPOILER_OMITTED,
        Condition { when: When::Refused, class: Omitted, reason: "media_alt_omitted" },
    ],
    ..DEFAULT.media
};

const SPOILERS_PER_POST: Rule = Rule {
    conditions: &[
        ATTACHED[0],
        ATTACHED[1],
        ATTACHED[2],
        Condition { when: When::Post, class: Approximated, reason: "media_spoiler_widened" },
    ],
    ..DEFAULT.media
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
            $crate::targets::MATH,
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
        CodePoints, Graphemes, Hashtags, Links, MastodonWeighted, Mentions, ThreadsWeighted, Utf8Bytes, Utf16CodeUnits,
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
