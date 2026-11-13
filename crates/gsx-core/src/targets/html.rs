use super::limits::{CodePoints, Limit, Post, Reel, Story, Text, With, Without, limit};
use super::plain::BODY;
use super::{DATE_BOUNDS, DEFAULT, TELEGRAM_TIME_FORMATS, TEXT, rule};
use crate::document::Layout;
use crate::forms::html::RENDERER;
use crate::project::Class::{Approximated, Blocked, Omitted, TextualFallback, Unwrapped};
use crate::project::{
    Condition, Feature, Form, Format, HtmlBlocks, HtmlForm, Normalization, Rule, Search, Spoilers, Structure, Target,
    When,
};

const fn html(name: &'static str, rules: &'static [Rule], form: &'static HtmlForm, limits: &'static [Limit]) -> Target {
    Target {
        name,
        rules,
        form: Form::Html(form),
        fields: BODY,
        search: Search::Monotone,
        normalization: Normalization::None,
        renderer: RENDERER,
        limits,
        spoilers: Spoilers::None,
    }
}

const HEADING: Rule = rule(Feature::Heading, TextualFallback, "heading_as_bold");
const CODE_INFO: Rule = rule(Feature::CodeBlock, Omitted, "code_block_info_omitted");
const MEDIA: Rule = Rule {
    feature: Feature::Media,
    class: Approximated,
    reason: "media_attached",
    conditions: &[
        Condition { when: When::Missing, class: Omitted, reason: "media_missing" },
        Condition { when: When::Spoiler, class: Omitted, reason: "media_spoiler_omitted" },
    ],
};
const HIDDEN: Spoilers = Spoilers::Items(&[Format::Image, Format::Gif, Format::Video]);
const LINK_TITLE: Rule = rule(Feature::LinkTitle, Omitted, "link_title_omitted");
const INVISIBLE: Condition = Condition { when: When::LabelInvisible, class: Omitted, reason: "link_label_invisible" };
const TEXTUAL: [Rule; 6] = [
    rule(Feature::ThematicBreak, TextualFallback, "thematic_break_as_text"),
    rule(Feature::BulletList, TextualFallback, "bullet_list_as_text"),
    rule(Feature::OrderedList, TextualFallback, "ordered_list_as_text"),
    rule(Feature::CompoundList, TextualFallback, "compound_list_as_text"),
    rule(Feature::Table, TextualFallback, "table_as_text"),
    rule(Feature::TaskList, TextualFallback, "task_list_as_text"),
];

const INLINE: [(Feature, &str); 8] = [
    (Feature::Heading, "b"),
    (Feature::Strong, "b"),
    (Feature::Emphasis, "i"),
    (Feature::Underline, "u"),
    (Feature::Strikethrough, "s"),
    (Feature::Spoiler, "tg-spoiler"),
    (Feature::InlineCode, "code"),
    (Feature::Link, "a"),
];

static TELEGRAM_FORM: HtmlForm = HtmlForm {
    blocks: HtmlBlocks::Message,
    structure: None,
    tags: &[
        INLINE[0],
        INLINE[1],
        INLINE[2],
        INLINE[3],
        INLINE[4],
        INLINE[5],
        INLINE[6],
        INLINE[7],
        (Feature::Datetime, "tg-time"),
    ],
    exclusive: &[Feature::Link, Feature::Datetime],
    groups: &[],
    code_contained: false,
    quote_nests: false,
    quote_contains_code: false,
    code_info: true,
    empty_removed: true,
    trim: true,
    time_formats: TELEGRAM_TIME_FORMATS,
};

const NESTED: Condition = Condition { when: When::Nested, class: Unwrapped, reason: "entity_cannot_contain_entity" };

const fn entity(feature: Feature, conditions: &'static [Condition]) -> Rule {
    Rule { feature, class: Unwrapped, reason: "entity_cannot_contain_code", conditions }
}

pub static TELEGRAM: Target = Target {
    spoilers: HIDDEN,
    ..html(
        "telegram-html",
        &[
            HEADING,
            TEXTUAL[0],
            Rule {
                feature: Feature::Blockquote,
                class: Unwrapped,
                reason: "code_block_cannot_be_quoted",
                conditions: &[Condition { when: When::Nested, class: Unwrapped, reason: "blockquote_cannot_nest" }],
            },
            TEXTUAL[1],
            TEXTUAL[2],
            TEXTUAL[3],
            CODE_INFO,
            TEXTUAL[4],
            TEXTUAL[5],
            MEDIA,
            entity(Feature::Emphasis, &[]),
            entity(Feature::Strong, &[]),
            entity(Feature::Strikethrough, &[]),
            entity(Feature::Underline, &[]),
            rule(Feature::Highlight, Unwrapped, "highlight_unwrapped"),
            entity(Feature::Spoiler, &[]),
            entity(Feature::Link, &[NESTED, INVISIBLE]),
            LINK_TITLE,
            entity(Feature::Datetime, &[NESTED, DATE_BOUNDS[0], DATE_BOUNDS[1]]),
            TEXT,
        ],
        &TELEGRAM_FORM,
        &[
            limit(Text, CodePoints, 4096).on(&[Post]).when(Without).visible(),
            limit(Text, CodePoints, 1024).on(&[Post, Reel]).when(With).visible(),
            limit(Text, CodePoints, 2048).on(&[Story]).visible(),
        ],
    )
};

static RICH_FORM: HtmlForm = HtmlForm {
    blocks: HtmlBlocks::Rich,
    structure: Some(Structure { blocks: 500, depth: 16, columns: 20, items: 50 }),
    tags: &[
        INLINE[0],
        INLINE[1],
        INLINE[2],
        INLINE[3],
        INLINE[4],
        INLINE[5],
        INLINE[6],
        INLINE[7],
        (Feature::Highlight, "mark"),
        (Feature::Datetime, "tg-time"),
    ],
    exclusive: &[],
    groups: &[(Layout::Collage, &[Format::Image, Format::Video]), (Layout::Slideshow, &[Format::Image])],
    code_contained: true,
    quote_nests: true,
    quote_contains_code: true,
    code_info: true,
    empty_removed: false,
    trim: false,
    time_formats: TELEGRAM_TIME_FORMATS,
};

pub static TELEGRAM_RICH: Target = Target {
    spoilers: HIDDEN,
    ..html(
        "telegram-html-rich",
        &[
            HEADING,
            CODE_INFO,
            rule(Feature::Table, Blocked, "table_columns_exceeded"),
            Rule {
                feature: Feature::Media,
                class: Approximated,
                reason: "media_attached",
                conditions: &[
                    Condition { when: When::Group, class: Approximated, reason: "media_group_split" },
                    Condition { when: When::Cut, class: Approximated, reason: "media_after_cut" },
                    Condition { when: When::Truncated, class: Approximated, reason: "media_alt_truncated" },
                    Condition { when: When::Empty, class: Omitted, reason: "media_alt_omitted" },
                    Condition { when: When::Count, class: Omitted, reason: "media_count_exceeded" },
                    Condition { when: When::Missing, class: Omitted, reason: "media_missing" },
                    Condition { when: When::Spoiler, class: Omitted, reason: "media_spoiler_omitted" },
                ],
            },
            rule(Feature::Link, Omitted, "link_label_invisible"),
            LINK_TITLE,
            Rule { conditions: &DATE_BOUNDS, ..DEFAULT.datetime },
            RICH_TEXT,
        ],
        &RICH_FORM,
        &[limit(Text, CodePoints, 32768).on(&[Post]).visible()],
    )
};

const RICH_TEXT_CONDITIONS: [Condition; 4] = [
    TEXT.conditions[0],
    TEXT.conditions[1],
    Condition { when: When::Blocks, class: Blocked, reason: "block_limit_exceeded" },
    Condition { when: When::Depth, class: Blocked, reason: "nesting_limit_exceeded" },
];
const RICH_TEXT: Rule = Rule { conditions: &RICH_TEXT_CONDITIONS, ..TEXT };

static MAX_FORM: HtmlForm = HtmlForm {
    blocks: HtmlBlocks::Message,
    structure: None,
    tags: &[INLINE[0], INLINE[1], INLINE[2], INLINE[3], INLINE[4], INLINE[6], INLINE[7], (Feature::Highlight, "mark")],
    exclusive: &[],
    groups: &[],
    code_contained: true,
    quote_nests: true,
    quote_contains_code: true,
    code_info: false,
    empty_removed: false,
    trim: false,
    time_formats: &[],
};

pub static MAX: Target = html(
    "max-html",
    &[
        HEADING,
        TEXTUAL[0],
        TEXTUAL[1],
        TEXTUAL[2],
        TEXTUAL[3],
        CODE_INFO,
        TEXTUAL[4],
        TEXTUAL[5],
        MEDIA,
        rule(Feature::Spoiler, Unwrapped, "spoiler_unwrapped"),
        rule(Feature::Link, Omitted, "link_label_invisible"),
        LINK_TITLE,
        rule(Feature::Datetime, TextualFallback, "datetime_label_as_text"),
        TEXT,
    ],
    &MAX_FORM,
    &[limit(Text, CodePoints, 4000).visible()],
);
