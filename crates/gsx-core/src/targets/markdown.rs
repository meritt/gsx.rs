use super::limits::{Alt, CodePoints, Image, Limit, Only, Text, limit};
use super::plain::BODY;
use super::{DEFAULT, Rules, SPOILERS_CARRIED, THREADED_TEXT, rule};
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
        alt: Format::ALL,
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
    datetime: "<t:$unix:$format>",
    time_format: "f",
    time_formats: &["t", "T", "d", "D", "s", "f", "F", "R"],
    separators: "",
    quote_depth: Some(1),
    url_labels: false,
    line_break: "\n",
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
        rules!(Rules { media: SPOILERS_CARRIED, text: THREADED_TEXT, ..DISCORD_RULES }),
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
    datetime: "<!date^$unix^$format|$label>",
    time_format: "{date_short} {time}",
    time_formats: &[
        "{time}",
        "{time_secs}",
        "{date_num}",
        "{date}",
        "{date_num} {time}",
        "{date} {time}",
        "{date_long} {time}",
        "{ago}",
    ],
    separators: "^|\n",
    quote_depth: None,
    url_labels: true,
    line_break: "  \n",
};

pub(super) const SLACK_RULES: Rules =
    Rules { code_block: Rule { conditions: &[INFO], ..DEFAULT.code_block }, ..DEFAULT };

pub static SLACK: Target = markdown(
    "slack-markdown",
    rules!(Rules { text: THREADED_TEXT, ..SLACK_RULES }),
    &SLACK_FORM,
    &[limit(Text, CodePoints, 12000), limit(Alt, CodePoints, 1000).when(Only(Image))],
);
