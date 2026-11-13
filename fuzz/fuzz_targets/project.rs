#![no_main]

use arbitrary::Arbitrary;
use gsx_core::document::{Media, parse};
use gsx_core::project::{Facts, Format, MediaFact, MediaKind, Options, Overflow, Placement, Search, project_with};
use gsx_core::targets::TARGETS;
use gsx_core::wire;
use libfuzzer_sys::fuzz_target;
use serde_json::Value;

const MEDIA: [Media<'static>; 4] = [
    Media { id: "a", cover: None },
    Media { id: "b", cover: Some("c") },
    Media { id: "d", cover: None },
    Media { id: "e", cover: None },
];
const IDS: [&str; 6] = ["a", "b", "c", "d", "e", "x"];
const FORMATS: [Format; 4] = [Format::Image, Format::Gif, Format::Video, Format::Audio];
const KINDS: [MediaKind; 4] = [MediaKind::Image, MediaKind::Gif, MediaKind::Video, MediaKind::Audio];
const PLACEMENTS: [Placement; 3] = [Placement::Post, Placement::Reel, Placement::Story];
const ITEMS: usize = 8;

#[derive(Arbitrary, Debug)]
struct Item<'a> {
    id: u8,
    format: u8,
    alt: Option<&'a str>,
    spoiler: bool,
    inline: bool,
}

#[derive(Arbitrary, Clone, Copy, Debug)]
enum Limit {
    Small(u16),
    Full(u32),
}

impl Limit {
    fn value(self) -> u32 {
        match self {
            Limit::Small(value) => value.into(),
            Limit::Full(value) => value,
        }
    }
}

#[derive(Arbitrary, Debug)]
struct Input<'a> {
    media: u8,
    kind: Option<u8>,
    media_count: u8,
    placement: Option<u8>,
    listed: bool,
    items: Vec<Item<'a>>,
    title: Option<&'a str>,
    target_url: Option<&'a str>,
    max_characters: Option<Limit>,
    characters_reserved_per_url: Option<Limit>,
    caption_limit: Option<Limit>,
    description_limit: Option<Limit>,
    hashtag_limit: Option<Limit>,
    spoiler_text: Option<&'a str>,
    thread: bool,
    entities_allowed: bool,
    maximum: Option<u8>,
    text: &'a str,
}

fn pick<T: Copy>(values: &[T], index: u8) -> T {
    values[usize::from(index) % values.len()]
}

fn texts(wired: &Value) -> Vec<&str> {
    wired["projection"]["fields"].as_array().into_iter().flatten().filter_map(|field| field["text"].as_str()).collect()
}

fuzz_target!(|input: Input| {
    let media = &MEDIA[..usize::from(input.media) % (MEDIA.len() + 1)];
    let Ok(document) = parse(input.text, media) else {
        return;
    };
    let items: Vec<MediaFact> = input
        .items
        .iter()
        .take(ITEMS)
        .map(|item| MediaFact {
            id: pick(&IDS, item.id),
            format: pick(&FORMATS, item.format),
            alt: item.alt,
            spoiler: item.spoiler,
            inline: item.inline,
        })
        .collect();
    let facts = Facts {
        media: input.kind.map(|kind| pick(&KINDS, kind)),
        media_count: usize::from(input.media_count),
        placement: input.placement.map(|placement| pick(&PLACEMENTS, placement)),
        title: input.title,
        target_url: input.target_url,
        max_characters: input.max_characters.map(Limit::value),
        characters_reserved_per_url: input.characters_reserved_per_url.map(Limit::value),
        caption_limit: input.caption_limit.map(Limit::value),
        description_limit: input.description_limit.map(Limit::value),
        hashtag_limit: input.hashtag_limit.map(Limit::value),
        spoiler_text: input.spoiler_text,
        overflow: if input.thread { Overflow::Thread } else { Overflow::Truncate },
        entities_allowed: input.entities_allowed,
        items: input.listed.then_some(&items[..]),
    };
    let options = Options { search: None, maximum: input.maximum.map(usize::from) };
    let without_media = Facts { media: None, media_count: 0, items: None, ..facts };
    for target in TARGETS {
        let first = project_with(&document, target, &facts, &options);
        let second = project_with(&document, target, &facts, &options);
        assert_eq!(first, second, "{} is not deterministic", target.name);
        let out = wire::project(input.text.as_bytes(), media, target, &facts);
        let wired: Value = serde_json::from_str(&out)
            .unwrap_or_else(|error| panic!("{} wire output is not JSON ({error}): {out}", target.name));
        if options.maximum.is_none() {
            let fields: Vec<&str> = first.fields.iter().map(|field| field.text.as_str()).collect();
            assert_eq!(texts(&wired), fields, "{} wire fields differ from the projection", target.name);
        }
        let verified = media.is_empty() && (!facts.entities_allowed || target.form.entities().is_some());
        if verified && input.text.len() <= 512 {
            let exhaustive = Options { search: Some(Search::Exhaustive), ..options };
            let monotone = Options { search: Some(Search::Monotone), ..options };
            assert_eq!(
                project_with(&document, target, &without_media, &monotone),
                project_with(&document, target, &without_media, &exhaustive),
                "{} monotone and exhaustive search disagree",
                target.name
            );
        }
    }
});
