use super::limits::{Alt, CodePoints, Limit, Text, limit};
use super::markdown::{DISCORD_FORM, DISCORD_RULES, SLACK_FORM, SLACK_RULES};
use super::plain::BODY;
use super::{Rules, rule};
use crate::document::Layout;
use crate::forms::blocks::RENDERER;
use crate::project::Class::{Approximated, Blocked, Omitted, TextualFallback};
use crate::project::{
    BlocksForm, Ceiling, Condition, Feature, Form, Format, Normalization, Rule, Search, Spoilers, Target, Unplaced,
    When,
};

const fn blocks(
    name: &'static str,
    rules: &'static [Rule],
    form: &'static BlocksForm,
    limits: &'static [Limit],
) -> Target {
    Target {
        name,
        rules,
        form: Form::Blocks(form),
        fields: BODY,
        search: Search::Monotone,
        normalization: Normalization::None,
        renderer: RENDERER,
        limits,
        spoilers: Spoilers::None,
        alt: form.formats,
    }
}

const fn media(unplaced: &'static str) -> Rule {
    Rule {
        feature: Feature::Media,
        class: Approximated,
        reason: unplaced,
        conditions: &[
            Condition { when: When::Group, class: Approximated, reason: "media_group_split" },
            Condition { when: When::Cut, class: Approximated, reason: "media_after_cut" },
            Condition { when: When::Truncated, class: Approximated, reason: "media_alt_truncated" },
            Condition { when: When::Refused, class: Omitted, reason: "media_alt_omitted" },
            Condition { when: When::Spoiler, class: Omitted, reason: "media_spoiler_omitted" },
            Condition { when: When::Count, class: Omitted, reason: "media_count_exceeded" },
            Condition { when: When::Missing, class: Omitted, reason: "media_missing" },
            Condition { when: When::Blocks, class: Blocked, reason: "blocks_too_many" },
        ],
    }
}

static DISCORD_FORM_BLOCKS: BlocksForm = BlocksForm {
    text: &DISCORD_FORM,
    formats: &[Format::Image, Format::Gif, Format::Video],
    group: Some((Layout::Collage, 10)),
    items: Some(10),
    ceiling: Some(Ceiling { blocks: 40, per_item: false }),
    spoiler: true,
    separator: true,
    unplaced: Unplaced::File,
};

pub static DISCORD: Target = blocks(
    "discord-blocks",
    rules!(Rules { media: media("media_as_file"), ..DISCORD_RULES }),
    &DISCORD_FORM_BLOCKS,
    &[limit(Text, CodePoints, 4000), limit(Alt, CodePoints, 1024)],
);

static SLACK_FORM_BLOCKS: BlocksForm = BlocksForm {
    text: &SLACK_FORM,
    formats: &[Format::Image, Format::Gif],
    group: Some((Layout::Slideshow, 10)),
    items: None,
    ceiling: Some(Ceiling { blocks: 50, per_item: true }),
    spoiler: false,
    separator: false,
    unplaced: Unplaced::Attach,
};

pub static SLACK: Target = blocks(
    "slack-blocks",
    rules!(Rules { media: media("media_attached"), ..SLACK_RULES }),
    &SLACK_FORM_BLOCKS,
    &[limit(Text, CodePoints, 12000), limit(Alt, CodePoints, 2000)],
);
