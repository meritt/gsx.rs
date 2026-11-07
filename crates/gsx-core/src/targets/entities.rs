use super::limits::{
    Alt, CodePoints, Fact, Graphemes, Limit, Links, Story, Text, ThreadsWeighted, Utf8Bytes, limit, resolved,
};
use super::plain::BODY;
use super::{DEFAULT, LINK, Rules, rule};
use crate::forms::plain::RENDERER;
use crate::project::Class::{Approximated, Blocked, Omitted, TextualFallback, Unwrapped};
use crate::project::{
    Condition, Detection, EntityForm, Feature, Form, Normalization, Offsets, Rule, Search, Style, Target, When,
};

const fn entities(
    name: &'static str,
    rules: &'static [Rule],
    form: &'static EntityForm,
    limits: &'static [Limit],
) -> Target {
    Target {
        name,
        rules,
        form: Form::Entities(form),
        fields: BODY,
        search: Search::Monotone,
        normalization: Normalization::None,
        renderer: RENDERER,
        limits,
    }
}

static BLUESKY_FORM: EntityForm = EntityForm {
    offsets: Offsets::Utf8,
    styles: &[(Feature::Link, Style::Link)],
    heading: None,
    gated: false,
    code_splits: false,
    detection: Detection::Bluesky,
    cap: None,
    links: None,
    trim: false,
    styled_suffix: true,
};

pub static BLUESKY: Target = entities(
    "bluesky-entities",
    rules!(Rules {
        link: rule(Feature::Link, Omitted, "link_label_invisible"),
        link_title: rule(Feature::LinkTitle, Approximated, "link_title_unsupported"),
        ..DEFAULT
    }),
    &BLUESKY_FORM,
    &[
        limit(Text, Graphemes, 300),
        limit(Text, Utf8Bytes, 3000),
        limit(Alt, Graphemes, 2000),
        limit(Alt, Utf8Bytes, 20000),
    ],
);

static THREADS_FORM: EntityForm = EntityForm {
    offsets: Offsets::Utf16,
    styles: &[(Feature::Spoiler, Style::Spoiler)],
    heading: None,
    gated: false,
    code_splits: false,
    detection: Detection::None,
    cap: Some(10),
    links: Some(5),
    trim: false,
    styled_suffix: false,
};

const THREADS_LINK: [Condition; 3] = [
    LINK.conditions[0],
    Condition { when: When::Count, class: Unwrapped, reason: "link_limit_exceeded" },
    Condition { when: When::Overflow, class: Blocked, reason: "link_limit_exceeded" },
];

pub static THREADS: Target = entities(
    "threads-entities",
    rules!(
        Rules {
            spoiler: rule(Feature::Spoiler, Unwrapped, "spoiler_limit"),
            link: Rule { conditions: &THREADS_LINK, ..LINK },
            ..DEFAULT
        },
        Rule {
            feature: Feature::TargetUrl,
            class: TextualFallback,
            reason: "target_url_as_text",
            conditions: &[
                Condition { when: When::Overflow, class: Omitted, reason: "target_url_omitted" },
                Condition { when: When::Refused, class: Omitted, reason: "target_url_refused" },
            ],
        },
    ),
    &THREADS_FORM,
    &[limit(Text, ThreadsWeighted, 500), limit(Text, Links, 5), limit(Alt, CodePoints, 1000)],
);

static TELEGRAM_FORM: EntityForm = EntityForm {
    offsets: Offsets::Utf16,
    styles: &[
        (Feature::Strong, Style::Bold),
        (Feature::Emphasis, Style::Italic),
        (Feature::Underline, Style::Underline),
        (Feature::Strikethrough, Style::Strikethrough),
        (Feature::Spoiler, Style::Spoiler),
        (Feature::InlineCode, Style::Code),
        (Feature::Link, Style::Link),
    ],
    heading: Some(Style::Bold),
    gated: true,
    code_splits: true,
    detection: Detection::None,
    cap: Some(100),
    links: None,
    trim: true,
    styled_suffix: true,
};

const CAPPED: Condition = Condition { when: When::Capped, class: Unwrapped, reason: "entity_limit" };

const fn gated(feature: Feature) -> Rule {
    Rule { feature, class: Unwrapped, reason: "entities_unavailable", conditions: &[CAPPED] }
}

pub static TELEGRAM: Target = entities(
    "telegram-entities",
    rules!(Rules {
        heading: Rule {
            feature: Feature::Heading,
            class: TextualFallback,
            reason: "heading_as_text",
            conditions: &[CAPPED]
        },
        code_block: rule(Feature::CodeBlock, Unwrapped, "code_block_unwrapped"),
        emphasis: gated(Feature::Emphasis),
        strong: gated(Feature::Strong),
        strikethrough: gated(Feature::Strikethrough),
        underline: gated(Feature::Underline),
        spoiler: gated(Feature::Spoiler),
        inline_code: gated(Feature::InlineCode),
        link: Rule {
            feature: Feature::Link,
            class: Unwrapped,
            reason: "entities_unavailable",
            conditions: &[
                Condition { when: When::LabelInvisible, class: Omitted, reason: "link_label_invisible" },
                CAPPED
            ],
        },
        link_title: rule(Feature::LinkTitle, Approximated, "link_title_unsupported"),
        datetime: rule(Feature::Datetime, Unwrapped, "datetime_unwrapped"),
        ..DEFAULT
    }),
    &TELEGRAM_FORM,
    &[resolved(Text, CodePoints, Fact::CaptionLimit, 200).on(&[Story])],
);
