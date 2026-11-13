#![no_main]

use arbitrary::Arbitrary;
use gsx_core::document::{Media, parse};
use gsx_core::project::{
    Attachment, Boundary, Facts, Form, Format, MediaFact, MediaKind, Options, Overflow, Placement, Projection,
    Search, Target, project_with,
};
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
const BOUNDARIES: [&str; 4] = ["paragraph", "sentence", "word", "grapheme"];

type Located<'a> = (u32, u32, Option<&'a str>);

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

fn code_points(text: &str) -> Vec<u32> {
    let mut at = 0;
    let mut offsets = vec![0];
    for char in text.chars() {
        at += char.len_utf16() as u32;
        offsets.push(at);
    }
    offsets
}

fn wired_parts<'v>(name: &str, wired: &'v Value) -> Vec<Located<'v>> {
    let Some(parts) = wired["projection"].get("parts") else {
        return vec![];
    };
    let parts = parts.as_array().unwrap_or_else(|| panic!("{name} wire parts are not a list: {parts}"));
    assert!(!parts.is_empty(), "{name} wire output keeps empty parts");
    let unit = |value: &Value| {
        value
            .as_u64()
            .and_then(|value| u32::try_from(value).ok())
            .unwrap_or_else(|| panic!("{name} wire offset {value}"))
    };
    parts
        .iter()
        .map(|part| {
            let range = part["range"].as_array().filter(|range| range.len() == 2);
            let range = range.unwrap_or_else(|| panic!("{name} wire part range is not a pair: {part}"));
            let boundary = match part.get("boundary") {
                Some(Value::Null) => None,
                Some(Value::String(boundary)) => Some(boundary.as_str()),
                _ => panic!("{name} wire part boundary is neither null nor a name: {part}"),
            };
            (unit(&range[0]), unit(&range[1]), boundary)
        })
        .collect()
}

fn located(name: &str, offsets: &[u32], parts: &[Located]) {
    assert_ne!(parts.len(), 1, "{name} reports an unsplit thread");
    for (index, &(start, end, boundary)) in parts.iter().enumerate() {
        assert!(
            start <= end && offsets.binary_search(&start).is_ok() && offsets.binary_search(&end).is_ok(),
            "{name} part {index} range [{start}, {end}) is not a code point range of the input"
        );
        assert_eq!(
            boundary.is_some(),
            index + 1 < parts.len(),
            "{name} part {index} of {} has boundary {boundary:?}",
            parts.len()
        );
        assert!(
            boundary.is_none_or(|boundary| BOUNDARIES.contains(&boundary)),
            "{name} part {index} has unknown boundary {boundary:?}"
        );
    }
    for (index, pair) in parts.windows(2).enumerate() {
        assert!(
            pair[0].0 <= pair[1].0 && pair[0].1 <= pair[1].1,
            "{name} part {} range {:?} runs before part {index} range {:?}",
            index + 1,
            pair[1],
            pair[0]
        );
    }
}

fn threads(target: &Target, facts: &Facts, offsets: &[u32], projection: &Projection, wired: &Value, exact: bool) {
    let name = target.name;
    let parts = &projection.parts;
    assert!(
        parts.is_empty() || (facts.overflow == Overflow::Thread && target.threads()),
        "{name} splits without thread overflow"
    );
    let threaded = projection.changes.iter().filter(|change| change.reason == "text_threaded").count();
    assert_eq!(threaded, usize::from(!parts.is_empty()), "{name} reports text_threaded for {} parts", parts.len());
    if let Some(first) = parts.first() {
        assert_eq!(first.media, projection.media, "{name} top-level media differ from the first part");
        assert_eq!(first.items, projection.items, "{name} top-level items differ from the first part");
        assert!(
            first.fields.iter().all(|field| projection.fields.contains(field))
                && first.measurements.iter().all(|measurement| projection.measurements.contains(measurement)),
            "{name} top level does not repeat the first part"
        );
    }
    for (index, part) in parts.iter().enumerate() {
        for measurement in part.measurements.iter().filter(|measurement| !measurement.unit.counts_markers()) {
            assert!(
                measurement.maximum.is_none_or(|maximum| measurement.value <= maximum),
                "{name} part {index} exceeds its limit: {measurement:?}"
            );
        }
        let referenced = |item: &Attachment| part.media.iter().any(|id| item.reference == format!("media:{id}"));
        assert!(index == 0 || part.items.iter().all(referenced), "{name} part {index} carries unreferenced items");
    }
    let rendered_whole = matches!(target.form, Form::Html(_) | Form::Markdown(_));
    for (index, pair) in parts.windows(2).enumerate().filter(|_| !rendered_whole) {
        let ends = pair[0].fields.iter().any(|field| field.text.ends_with(char::is_whitespace));
        let starts = pair[1].fields.iter().any(|field| field.text.starts_with(char::is_whitespace));
        assert!(!ends && !starts, "{name} keeps whitespace at the boundary after part {index}");
    }
    let ours: Vec<Located> =
        parts.iter().map(|part| (part.range.start, part.range.end, part.boundary.map(Boundary::name))).collect();
    located(name, offsets, &ours);
    let theirs = wired_parts(name, wired);
    located(&format!("{name} wire"), offsets, &theirs);
    if exact {
        assert_eq!(theirs, ours, "{name} wire parts differ from the projection");
    }
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
    let offsets = code_points(input.text);
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
        threads(target, &facts, &offsets, &first, &wired, options.maximum.is_none());
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
