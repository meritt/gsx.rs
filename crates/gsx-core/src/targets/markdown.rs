use super::limits::{Alt, CodePoints, Image, Limit, Only, Text, limit};
use super::plain::BODY;
use super::{DEFAULT, Rules, SPOILERS_CARRIED, rule};
use crate::forms::markdown::RENDERER;
use crate::project::Class::{Approximated, Omitted, TextualFallback, Unwrapped};
use crate::project::{
    Condition, Feature, Form, Format, LineStart, MarkdownForm, Normalization, Rule, Search, Spoilers, Target, When,
};

const fn markdown(
    name: &'static str,
    rules: &'static [Rule],
    form: &'static MarkdownForm,
    limits: &'static [Limit],
) -> Target {
    Target {
        name,
        rules,
        form: Form::Markdown(form),
        fields: BODY,
        search: Search::Monotone,
        normalization: Normalization::None,
        renderer: RENDERER,
        limits,
        spoilers: Spoilers::None,
    }
}

const INFO: Condition = Condition { when: When::Info, class: Omitted, reason: "code_block_info_omitted" };

pub(super) static DISCORD_FORM: MarkdownForm = MarkdownForm {
    delimiters: &[
        (Feature::Emphasis, "*"),
        (Feature::Strong, "**"),
        (Feature::Underline, "__"),
        (Feature::Strikethrough, "~~"),
        (Feature::Spoiler, "||"),
    ],
    native: &[
        Feature::Blockquote,
        Feature::BulletList,
        Feature::OrderedList,
        Feature::CompoundList,
        Feature::CodeBlock,
    ],
    escaped: "\\`*_~|{}[]()+-!&<>",
    owned: true,
    neutralised: &[LineStart::Heading, LineStart::Ordered],
    html: false,
    datetime: "<t:$unix:f>",
    separators: "",
    quote_depth: Some(1),
    url_labels: false,
};

pub(super) const DISCORD_RULES: Rules = Rules {
    blockquote: Rule {
        conditions: &[Condition { when: When::Nested, class: Unwrapped, reason: "blockquote_nesting_unwrapped" }],
        ..DEFAULT.blockquote
    },
    code_block: Rule { conditions: &[INFO], ..DEFAULT.code_block },
    datetime: rule(Feature::Datetime, Approximated, "datetime_label_omitted"),
    ..DEFAULT
};

pub static DISCORD: Target = Target {
    spoilers: Spoilers::Items(&[Format::Image, Format::Gif, Format::Video, Format::Audio]),
    ..markdown(
        "discord-markdown",
        rules!(Rules { media: SPOILERS_CARRIED, ..DISCORD_RULES }),
        &DISCORD_FORM,
        &[limit(Text, CodePoints, 2000), limit(Alt, CodePoints, 1024)],
    )
};

pub(super) static SLACK_FORM: MarkdownForm = MarkdownForm {
    delimiters: &[(Feature::Emphasis, "*"), (Feature::Strong, "**"), (Feature::Strikethrough, "~~")],
    native: &[
        Feature::ThematicBreak,
        Feature::Blockquote,
        Feature::BulletList,
        Feature::OrderedList,
        Feature::CompoundList,
        Feature::CodeBlock,
        Feature::Table,
        Feature::TaskList,
    ],
    escaped: "\\`*_~|{}[]()+-!&<>",
    owned: true,
    neutralised: &[LineStart::Heading, LineStart::Ordered, LineStart::Setext],
    html: true,
    datetime: "<!date^$unix^{date_short} {time}|$label>",
    separators: "^|",
    quote_depth: None,
    url_labels: true,
};

pub(super) const SLACK_RULES: Rules =
    Rules { code_block: Rule { conditions: &[INFO], ..DEFAULT.code_block }, ..DEFAULT };

pub static SLACK: Target = markdown(
    "slack-markdown",
    rules!(SLACK_RULES),
    &SLACK_FORM,
    &[limit(Text, CodePoints, 12000), limit(Alt, CodePoints, 1000).when(Only(Image))],
);
