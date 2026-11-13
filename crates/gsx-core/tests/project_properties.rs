mod support;
#[allow(dead_code)]
#[path = "support/schema.rs"]
mod validator;

use std::sync::LazyLock;

use gsx_core::document::{Document, Kind, Media, Range, parse};
use gsx_core::project::{
    Class, Facts, Form, Format, MediaFact, MediaKind, Offsets, Options, Overflow, Placement, Projection, Target,
    project_with,
};
use gsx_core::targets::TARGETS;
use gsx_core::wire;
use proptest::prelude::*;
use support::Json;
use support::markup::boundaries;
use validator::Schema;

static SCHEMA: LazyLock<Schema> = LazyLock::new(Schema::load);

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
    "::time[tomorrow]{datetime=\"2026-11-06T18:00:00Z\"}",
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
const IDS: [&str; 6] = ["a", "b", "c", "d", "e", "x"];
const FORMATS: [Format; 4] = [Format::Image, Format::Gif, Format::Video, Format::Audio];
const PLACEMENTS: [Placement; 3] = [Placement::Post, Placement::Reel, Placement::Story];
const KINDS: [MediaKind; 4] = [MediaKind::Image, MediaKind::Gif, MediaKind::Video, MediaKind::Audio];
const URLS: &[&str] = &[
    "https://example.com/post/1",
    "https://www.threads.net/@alice/post/C1a2b3",
    "http://пример.рф/путь?q=1#frag",
    "https://example.com/a_(b)*c*",
    "example.com",
    "not a url",
    "https://",
    "javascript:alert(1)",
    "",
    " ",
    "https://example.com/\nnext",
    "**https://example.com**",
];
const ALTS: &[&str] = &["", " ", "\n", "**a**", "https://example.com", "🇺🇦 e\u{301}"];

#[derive(Clone, Debug)]
struct Item {
    id: usize,
    format: usize,
    alt: Option<String>,
    spoiler: bool,
    inline: bool,
}

#[derive(Clone, Debug)]
struct Setup {
    media: usize,
    kind: Option<usize>,
    media_count: usize,
    placement: Option<Placement>,
    listed: bool,
    items: Vec<Item>,
    title: Option<String>,
    target_url: Option<String>,
    max_characters: Option<u32>,
    characters_reserved_per_url: Option<u32>,
    caption_limit: Option<u32>,
    description_limit: Option<u32>,
    hashtag_limit: Option<u32>,
    spoiler_text: Option<String>,
    thread: bool,
    entities_allowed: bool,
    maximum: Option<usize>,
}

fn markup() -> impl Strategy<Value = String> {
    prop::collection::vec(prop::sample::select(FRAGMENTS), 0..64).prop_map(|parts| parts.concat())
}

fn alt() -> impl Strategy<Value = String> {
    prop_oneof![
        "[a-zé🚀 ]{0,24}",
        "[a-z🚀 ]{1,6}".prop_map(|part| part.repeat(500)),
        prop::sample::select(ALTS).prop_map(str::to_owned),
    ]
}

fn item() -> impl Strategy<Value = Item> {
    (0..IDS.len(), 0..FORMATS.len(), prop::option::of(alt()), any::<bool>(), any::<bool>())
        .prop_map(|(id, format, alt, spoiler, inline)| Item { id, format, alt, spoiler, inline })
}

fn target_url() -> impl Strategy<Value = String> {
    prop_oneof![prop::sample::select(URLS).prop_map(str::to_owned), "[a-z:/.?#@ ]{0,32}"]
}

fn setup() -> impl Strategy<Value = Setup> {
    (
        (0..=MEDIA.len(), prop::option::of(0..KINDS.len()), 0..=IDS.len()),
        prop::collection::vec(item(), 0..=IDS.len()),
        (
            prop::option::of("[a-zé🚀 ]{0,24}"),
            prop::option::of(target_url()),
            prop::option::of("[a-z🚀 ]{0,60}|https://example.com/cw"),
        ),
        (
            prop::option::of(1..1000u32),
            prop::option::of(0..64u32),
            prop::option::of(1..2000u32),
            prop::option::of(0..2000u32),
            prop::option::of(0..40u32),
        ),
        (
            any::<bool>(),
            prop::option::of(0..160usize),
            prop::option::of(prop::sample::select(&PLACEMENTS[..])),
            any::<bool>(),
            any::<bool>(),
        ),
    )
        .prop_map(
            |(
                (media, kind, media_count),
                items,
                (title, target_url, spoiler_text),
                (max_characters, characters_reserved_per_url, caption_limit, description_limit, hashtag_limit),
                (entities_allowed, maximum, placement, listed, thread),
            )| {
                Setup {
                    media,
                    kind,
                    media_count,
                    placement,
                    listed,
                    items,
                    title,
                    target_url,
                    max_characters,
                    characters_reserved_per_url,
                    caption_limit,
                    description_limit,
                    hashtag_limit,
                    spoiler_text,
                    thread,
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

fn whole_parts(document: &Document, target: &Target, projection: &Projection) -> Result<(), TestCaseError> {
    let name = target.name;
    let parts = &projection.parts;
    for part in parts {
        for measurement in part.measurements.iter().filter(|measurement| !measurement.unit.counts_markers()) {
            prop_assert!(
                measurement.maximum.is_none_or(|maximum| measurement.value <= maximum),
                "{name} part exceeds its limit: {measurement:?}"
            );
        }
    }
    for (index, pair) in parts.windows(2).enumerate() {
        prop_assert!(
            pair[0].range.end <= pair[1].range.start,
            "{name} part {index} range {:?} overlaps the next {:?}",
            pair[0].range,
            pair[1].range
        );
    }
    let mut elements = vec![];
    for node in document.children() {
        match node.kind {
            Kind::Paragraph => elements.extend(
                node.children
                    .iter()
                    .filter(|child| !matches!(child.kind, Kind::Text { .. } | Kind::Break))
                    .map(|child| child.range),
            ),
            _ => elements.push(node.range),
        }
    }
    for element in elements {
        let touched = parts.iter().filter(|part| part.range.start < element.end && element.start < part.range.end);
        prop_assert!(touched.count() <= 1, "{name} splits the element at {element:?} across parts");
    }
    Ok(())
}

fn check(
    input: &str,
    document: &Document,
    target: &Target,
    facts: &Facts,
    projection: &Projection,
) -> Result<(), TestCaseError> {
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
    for (index, part) in projection.parts.iter().enumerate() {
        for measurement in &part.measurements {
            let Some(maximum) = measurement.maximum else { continue };
            let blocked = projection.changes.iter().any(|change| change.class == Class::Blocked);
            prop_assert!(
                measurement.value <= maximum || measurement.unit.counts_markers() || blocked,
                "{name} part measures {} > {maximum} {:?}",
                measurement.value,
                measurement.unit
            );
        }
        let media = !part.media.is_empty() || !part.items.is_empty() || index == 0 && facts.media.is_some();
        prop_assert!(part.fields.iter().all(|field| media || !field.text.is_empty()), "{name} has an empty part");
        prop_assert!(
            part.range.start <= part.range.end && part.range.end <= units,
            "{name} part range {:?} outside the input",
            part.range
        );
    }
    for (index, pair) in projection.parts.windows(2).enumerate() {
        prop_assert!(pair[0].range.start <= pair[1].range.start, "{name} part {index} ranges run backwards");
        prop_assert!(pair[0].boundary.is_some(), "{name} part {index} has no boundary before the next part");
    }
    if let Some(last) = projection.parts.last() {
        prop_assert!(last.boundary.is_none(), "{name} last part has a boundary");
    }
    if matches!(target.form, Form::Html(_) | Form::Markdown(_)) {
        whole_parts(document, target, projection)?;
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
    let items: Vec<MediaFact> = setup
        .items
        .iter()
        .map(|item| MediaFact {
            id: IDS[item.id],
            format: FORMATS[item.format],
            alt: item.alt.as_deref(),
            spoiler: item.spoiler,
            inline: item.inline,
        })
        .collect();
    let facts = Facts {
        media: setup.kind.map(|kind| KINDS[kind]),
        media_count: setup.media_count,
        placement: setup.placement,
        title: setup.title.as_deref(),
        target_url: setup.target_url.as_deref(),
        max_characters: setup.max_characters,
        characters_reserved_per_url: setup.characters_reserved_per_url,
        caption_limit: setup.caption_limit,
        description_limit: setup.description_limit,
        hashtag_limit: setup.hashtag_limit,
        spoiler_text: setup.spoiler_text.as_deref(),
        overflow: if setup.thread { Overflow::Thread } else { Overflow::Truncate },
        entities_allowed: setup.entities_allowed,
        items: setup.listed.then_some(&items[..]),
    };
    let options = Options { search: None, maximum: setup.maximum };
    for target in TARGETS {
        let first = project_with(&document, target, &facts, &options);
        let second = project_with(&document, target, &facts, &options);
        prop_assert_eq!(&first, &second, "{} is not deterministic", target.name);
        check(input, &document, target, &facts, &first)?;
    }
    let sampled = TARGETS[input.len() % TARGETS.len()];
    let out = wire::project(input.as_bytes(), &MEDIA[..setup.media], sampled, &facts);
    SCHEMA.check_wire(&out).map_err(TestCaseError::fail)?;
    let parsed = Json::parse(&wire::parse(input.as_bytes(), &MEDIA[..setup.media]));
    let projected = Json::parse(&out);
    prop_assert_eq!(
        projected.get("projection").get("diagnostics"),
        parsed.get("document").get("diagnostics"),
        "{} diagnostics differ from the document's",
        sampled.name
    );
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn projections_are_deterministic_declared_and_within_bounds(input in markup(), setup in setup()) {
        properties(&input, &setup)?;
    }
}
