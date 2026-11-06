use super::limits::{
    Alt, CodePoints, Description, Fact, Image, Limit, MastodonWeighted, Only, Post, Text, Title, Utf8Bytes,
    Utf16CodeUnits, Video, XWeighted, given, limit, resolved, unlimited,
};
use super::{DEFAULT, Rules, rule};
use crate::forms::plain::RENDERER;
use crate::project::Class::{Approximated, Blocked, TextualFallback};
use crate::project::{
    Condition, Escape, Feature, FieldName, FieldSpec, Form, Media, Normalization, Rule, Search, Source, Target,
    Truncation, When,
};

pub(super) const fn field(name: FieldName, media: Media, source: Source, required: bool) -> FieldSpec {
    FieldSpec { name, media, source, truncation: Truncation::Ellipsis, required }
}

const TITLE_SOURCE: &[Condition] =
    &[Condition { when: When::TitleSource, class: Approximated, reason: "heading_became_title" }];

macro_rules! plain {
    ($heading:expr $(, $extra:expr)* $(,)?) => {
        rules!(
            Rules {
                heading: Rule { conditions: $heading, ..DEFAULT.heading },
                ..DEFAULT
            }
            $(, $extra)*
        )
    };
}

const fn text(
    escape: Escape,
    name: &'static str,
    rules: &'static [Rule],
    fields: &'static [FieldSpec],
    limits: &'static [Limit],
) -> Target {
    Target {
        name,
        rules,
        form: Form::Plain { escape },
        fields,
        search: Search::Monotone,
        normalization: Normalization::None,
        renderer: RENDERER,
        limits,
    }
}

pub(super) const BODY: &[FieldSpec] = &[field(FieldName::Text, Media::Any, Source::Body, false)];

pub static X: Target = Target {
    normalization: Normalization::Nfc,
    ..text(Escape::None, "x-plain", plain!(&[]), BODY, &[limit(Text, XWeighted, 280)])
};

pub static MASTODON: Target = text(
    Escape::None,
    "mastodon-plain",
    plain!(&[]),
    BODY,
    &[resolved(Text, MastodonWeighted, Fact::MaxCharacters, 500), given(Alt, CodePoints, Fact::DescriptionLimit)],
);

pub static LINKEDIN: Target = text(
    Escape::LinkedIn,
    "linkedin-plain",
    plain!(&[]),
    BODY,
    &[limit(Text, CodePoints, 3000), limit(Alt, CodePoints, 4086)],
);

pub static FACEBOOK: Target =
    text(Escape::None, "facebook-plain", plain!(&[]), BODY, &[limit(Text, CodePoints, 63206)]);

pub static INSTAGRAM: Target = text(
    Escape::None,
    "instagram-plain",
    plain!(&[]),
    BODY,
    &[limit(Text, CodePoints, 2200), limit(Alt, CodePoints, 1000)],
);

pub static VK: Target = text(
    Escape::None,
    "vk-plain",
    plain!(&[]),
    &[
        field(FieldName::Text, Media::Any, Source::Body, false),
        field(FieldName::Title, Media::Video, Source::Body, false),
    ],
    &[
        unlimited(Text, CodePoints),
        limit(Title, CodePoints, 128).when(Only(Video)),
        limit(Alt, CodePoints, 5000).when(Only(Video)),
        limit(Alt, CodePoints, 2048).when(Only(Image)),
    ],
);

pub static YOUTUBE: Target = text(
    Escape::Angle,
    "youtube-plain",
    plain!(
        TITLE_SOURCE,
        rule(Feature::AngleBrackets, Approximated, "angle_brackets_replaced"),
        Rule {
            feature: Feature::Title,
            class: Blocked,
            reason: "title_too_long",
            conditions: &[Condition { when: When::Empty, class: Blocked, reason: "title_empty" }],
        },
    ),
    &[
        field(FieldName::Title, Media::Any, Source::Title { fact: true, first_line_removed: false }, true),
        field(FieldName::Description, Media::Any, Source::Body, false),
    ],
    &[limit(Title, CodePoints, 100), limit(Description, Utf8Bytes, 5000)],
);

pub static TIKTOK: Target = text(
    Escape::None,
    "tiktok-plain",
    plain!(TITLE_SOURCE),
    &[
        field(FieldName::Title, Media::Image, Source::Title { fact: false, first_line_removed: true }, false),
        field(FieldName::Text, Media::Any, Source::Body, false),
    ],
    &[
        limit(Text, Utf16CodeUnits, 2200).when(Only(Video)),
        limit(Text, Utf16CodeUnits, 4000).on(&[Post]).when(Only(Image)),
        limit(Title, Utf16CodeUnits, 90).on(&[Post]).when(Only(Image)),
    ],
);

pub static SLACK: Target = text(
    Escape::Html,
    "slack-plain",
    plain!(&[]),
    BODY,
    &[unlimited(Text, CodePoints), limit(Alt, CodePoints, 1000).when(Only(Image))],
);
