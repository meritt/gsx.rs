mod support;

use gsx_core::document::{Media, Range, parse};
use gsx_core::project::{
    Class, Facts, Format, MediaFact, MediaKind, Offsets, Options, Placement, Projection, Target, project_with,
};
use gsx_core::targets::TARGETS;
use proptest::prelude::*;
use support::markup::boundaries;

const FRAGMENTS: &[&str] = &[
    "a",
    "word ",
    "слово ",
    "漢字",
    "é",
    "e\u{301}",
    "🚀",
    "👩‍👩‍👧",
    "🇺🇦",
    " ",
    "\n",
    "\n\n",
    "\t",
    "*",
    "**strong**",
    "_em_",
    "~~gone~~",
    "++under++",
    "==mark==",
    "||spoiler||",
    "`code`",
    "\n```js\nlet x = 1;\n```\n",
    "\\*",
    "&amp;",
    "<b>",
    "# ",
    "## Heading\n",
    "- ",
    "1. ",
    "- [ ] ",
    "> ",
    "---\n",
    "| a | b |\n| --- | :-: |\n| c | d |\n",
    "[label](https://example.com/a \"t\")",
    "[https://example.com](https://example.com)",
    "https://example.com/path?q=1",
    "www.example.org",
    "mail@example.net",
    "@alice",
    "@alice@social.example",
    "#hashtag",
    ":time[tomorrow]{datetime=\"2026-11-06T18:00:00Z\"}",
    "::media{id=\"a\"}\n",
    "::media{id=\"b\"}\n",
    "::media{ids=\"a,b\" layout=\"collage\"}\n",
    "a-very-long-hyphenated-compound-word",
    "end.",
    "...",
];

const MEDIA: [Media<'static>; 4] = [
    Media { id: "a", cover: None },
    Media { id: "b", cover: Some("c") },
    Media { id: "d", cover: None },
    Media { id: "e", cover: None },
];
const FORMATS: [Format; 4] = [Format::Image, Format::Gif, Format::Video, Format::Audio];
const PLACEMENTS: [Placement; 3] = [Placement::Post, Placement::Reel, Placement::Story];

#[derive(Clone, Debug)]
struct Setup {
    media: usize,
    video: bool,
    placement: Option<Placement>,
    items: Vec<(usize, bool, bool, bool)>,
    title: Option<String>,
    max_characters: Option<u32>,
    characters_reserved_per_url: Option<u32>,
    caption_limit: Option<u32>,
    entities_allowed: bool,
    maximum: Option<usize>,
}

fn markup() -> impl Strategy<Value = String> {
    prop::collection::vec(prop::sample::select(FRAGMENTS), 0..64).prop_map(|parts| parts.concat())
}

fn setup() -> impl Strategy<Value = Setup> {
    (
        (0..=MEDIA.len(), any::<bool>()),
        prop::collection::vec((0..FORMATS.len(), any::<bool>(), any::<bool>(), any::<bool>()), MEDIA.len()),
        prop::option::of("[a-zé🚀 ]{0,24}"),
        (prop::option::of(1..1000u32), prop::option::of(0..64u32), prop::option::of(1..2000u32)),
        (any::<bool>(), prop::option::of(0..160usize), prop::option::of(prop::sample::select(&PLACEMENTS[..]))),
    )
        .prop_map(
            |(
                (media, video),
                items,
                title,
                (max_characters, characters_reserved_per_url, caption_limit),
                (entities_allowed, maximum, placement),
            )| {
                Setup {
                    media,
                    video,
                    placement,
                    items,
                    title,
                    max_characters,
                    characters_reserved_per_url,
                    caption_limit,
                    entities_allowed,
                    maximum,
                }
            },
        )
}

fn within(range: Range, boundaries: &[bool]) -> bool {
    let (start, end) = (range.start as usize, range.end as usize);
    start <= end && end < boundaries.len() && boundaries[start] && boundaries[end]
}

fn check(input: &str, target: &Target, projection: &Projection) -> Result<(), TestCaseError> {
    let name = target.name;
    let units = input.encode_utf16().count() as u32;
    for change in &projection.changes {
        prop_assert!(
            target.declares(change.feature, change.class, change.reason),
            "{name} emits undeclared ({}, {}, {})",
            change.feature.name(),
            change.class.name(),
            change.reason
        );
        if let Some(range) = change.range {
            prop_assert!(
                range.start <= range.end && range.end <= units,
                "{name} change range {range:?} outside {units}"
            );
        }
    }
    for measurement in &projection.measurements {
        let Some(maximum) = measurement.maximum else { continue };
        let blocked =
            projection.changes.iter().any(|change| change.field == measurement.field && change.class == Class::Blocked);
        prop_assert!(
            measurement.value <= maximum || blocked,
            "{name} {:?} measures {} > {maximum} {:?} without a blocking change",
            measurement.field,
            measurement.value,
            measurement.unit
        );
    }
    for field in &projection.fields {
        let utf16 = boundaries(&field.text);
        for link in &field.links {
            prop_assert!(within(*link, &utf16), "{name} {:?} link {link:?} off a UTF-16 boundary", field.name);
        }
        match target.form.entities().map(|form| form.offsets) {
            None => prop_assert!(field.entities.is_empty(), "{name} emits entities without an entity form"),
            Some(Offsets::Utf16) => {
                for entity in &field.entities {
                    prop_assert!(
                        within(entity.range, &utf16),
                        "{name} entity {:?} off a UTF-16 boundary",
                        entity.range
                    );
                }
            }
            Some(Offsets::Utf8) => {
                for entity in &field.entities {
                    let (start, end) = (entity.range.start as usize, entity.range.end as usize);
                    prop_assert!(
                        start <= end && field.text.is_char_boundary(start) && field.text.is_char_boundary(end),
                        "{name} entity {:?} off a UTF-8 boundary of {} bytes",
                        entity.range,
                        field.text.len()
                    );
                }
            }
        }
    }
    Ok(())
}

fn properties(input: &str, setup: &Setup) -> Result<(), TestCaseError> {
    let Ok(document) = parse(input, &MEDIA[..setup.media]) else {
        return Ok(());
    };
    let items: Vec<MediaFact> = MEDIA[..setup.media]
        .iter()
        .zip(&setup.items)
        .map(|(media, &(format, alt, spoiler, inline))| MediaFact {
            id: media.id,
            format: FORMATS[format],
            alt: alt.then_some("alt text"),
            spoiler,
            inline,
        })
        .collect();
    let facts = Facts {
        media: (setup.media > 0).then_some(if setup.video { MediaKind::Video } else { MediaKind::Image }),
        media_count: setup.media,
        placement: setup.placement,
        title: setup.title.as_deref(),
        target_url: None,
        max_characters: setup.max_characters,
        characters_reserved_per_url: setup.characters_reserved_per_url,
        caption_limit: setup.caption_limit,
        description_limit: setup.caption_limit,
        entities_allowed: setup.entities_allowed,
        items: Some(&items),
    };
    let options = Options { search: None, maximum: setup.maximum };
    for target in TARGETS {
        let first = project_with(&document, target, &facts, &options);
        let second = project_with(&document, target, &facts, &options);
        prop_assert_eq!(&first, &second, "{} is not deterministic", target.name);
        check(input, target, &first)?;
    }
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn projections_are_deterministic_declared_and_within_bounds(input in markup(), setup in setup()) {
        properties(&input, &setup)?;
    }
}
