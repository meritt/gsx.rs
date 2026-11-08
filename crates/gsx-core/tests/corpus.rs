mod support;

use std::collections::HashMap;

use gsx_core::document::{Media, parse};
use gsx_core::project::{Facts, Format, MediaFact, MediaKind, Placement};
use gsx_core::targets::target;
use gsx_core::wire;
use support::{Json, LONE_SURROGATE, difference, records};

fn media(case: &Json) -> Vec<Media<'_>> {
    case.get("media")
        .array()
        .iter()
        .map(|item| Media { id: item.get("id").str().unwrap(), cover: item.get("cover").str() })
        .collect()
}

fn items(json: &Json) -> Vec<MediaFact<'_>> {
    json.get("items")
        .array()
        .iter()
        .map(|item| MediaFact {
            id: item.get("id").str().unwrap(),
            format: match item.get("format").str() {
                Some("image") => Format::Image,
                Some("gif") => Format::Gif,
                Some("video") => Format::Video,
                Some("audio") => Format::Audio,
                other => panic!("no media format {other:?}"),
            },
            alt: item.get("alt").str(),
            spoiler: item.get("spoiler") == &Json::Bool(true),
            inline: item.get("inline") == &Json::Bool(true),
        })
        .collect()
}

fn facts<'a>(json: &'a Json, items: &'a [MediaFact<'a>]) -> Facts<'a> {
    if let Json::Object(object) = json {
        let unknown: Vec<_> = object.keys().filter(|key| !KNOWN_FACTS.contains(&key.as_str())).collect();
        assert!(unknown.is_empty(), "facts not read by the corpus test: {unknown:?}");
    }
    let limit = |key: &str| match json.get(key) {
        Json::Number(limit) => Some(*limit as u32),
        _ => None,
    };
    Facts {
        media: match json.get("media").str() {
            None => None,
            Some("image") => Some(MediaKind::Image),
            Some("video") => Some(MediaKind::Video),
            Some("gif") => Some(MediaKind::Gif),
            Some("audio") => Some(MediaKind::Audio),
            Some(other) => panic!("no media kind {other}"),
        },
        caption_limit: limit("captionLimit"),
        entities_allowed: json.get("entitiesAllowed") == &Json::Bool(true),
        description_limit: limit("descriptionLimit"),
        hashtag_limit: limit("hashtagLimit"),
        spoiler_text: json.get("spoilerText").str(),
        max_characters: limit("maxCharacters"),
        characters_reserved_per_url: limit("charactersReservedPerUrl"),
        title: json.get("title").str(),
        target_url: json.get("targetUrl").str(),
        media_count: match json.get("mediaCount") {
            Json::Number(count) => *count as usize,
            _ => 0,
        },
        placement: match json.get("placement").str() {
            None => None,
            Some("post") => Some(Placement::Post),
            Some("reel") => Some(Placement::Reel),
            Some("story") => Some(Placement::Story),
            Some(other) => panic!("no placement {other}"),
        },
        items: matches!(json.get("items"), Json::Array(_)).then_some(items),
    }
}

const KNOWN_FACTS: &[&str] = &[
    "media",
    "mediaCount",
    "placement",
    "captionLimit",
    "descriptionLimit",
    "hashtagLimit",
    "spoilerText",
    "maxCharacters",
    "charactersReservedPerUrl",
    "title",
    "entitiesAllowed",
    "targetUrl",
    "items",
];

fn check(failures: &mut Vec<String>, label: &str, expected: &Json, actual: &str) {
    if let Some(found) = difference("", expected, &Json::parse(actual)) {
        failures.push(format!("{label}\n    {found}"));
    }
}

fn report(failures: &[String], compared: usize) {
    assert!(
        failures.is_empty(),
        "{} of {compared} cases differ; after an intended change run `cargo xtask corpus` and review the diff:\n{}",
        failures.len(),
        failures.iter().take(40).cloned().collect::<Vec<_>>().join("\n")
    );
}

fn matches_parse_corpus(file: &str) {
    let mut failures = vec![];
    let mut compared = 0;
    for case in records(file) {
        let markdown = case.get("markdown").str().unwrap();
        if markdown.contains(LONE_SURROGATE) {
            continue;
        }
        compared += 1;
        let actual = wire::parse(markdown.as_bytes(), &media(&case));
        check(&mut failures, &format!("{} {markdown:?}", case.get("id")), case.get("expected"), &actual);
    }
    report(&failures, compared);
}

#[test]
fn matches_document_corpus() {
    matches_parse_corpus("document.jsonl");
}

#[test]
fn matches_diagnostics_corpus() {
    matches_parse_corpus("diagnostics.jsonl");
}

#[test]
fn matches_target_corpus() {
    let inputs: HashMap<String, Json> = records("document.jsonl")
        .into_iter()
        .chain(records("diagnostics.jsonl"))
        .map(|case| (case.get("id").str().unwrap().to_owned(), case))
        .collect();
    let dir = format!("{}/../../testdata/corpus/targets", env!("CARGO_MANIFEST_DIR"));
    let mut files: Vec<_> = std::fs::read_dir(dir).unwrap().map(|entry| entry.unwrap().path()).collect();
    files.sort();
    let mut failures = vec![];
    let mut compared = 0;
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        let header = Json::parse(text.lines().next().unwrap());
        let name = header.get("target").str().unwrap();
        let target = target(name).unwrap();
        for line in text.lines().skip(1).filter(|line| !line.is_empty()) {
            let record = Json::parse(line);
            let id = record.get("id").str().unwrap();
            let input = if record.get("markdown").str().is_some() { &record } else { &inputs[id] };
            let markdown = input.get("markdown").str().unwrap();
            if markdown.contains(LONE_SURROGATE) {
                continue;
            }
            compared += 1;
            let given = if record.get("facts") == &Json::Null { header.get("facts") } else { record.get("facts") };
            let items = items(given);
            let facts = facts(given, &items);
            let actual = wire::project(markdown.as_bytes(), &media(input), target, &facts);
            check(&mut failures, &format!("{name} {id} {}", record.get("facts")), record.get("expected"), &actual);
        }
    }
    report(&failures, compared);
}

#[test]
fn prefixes_do_not_panic() {
    for case in records("document.jsonl").iter().take(400) {
        let markdown = case.get("markdown").str().unwrap();
        for end in (0..=markdown.len()).filter(|&end| markdown.is_char_boundary(end)) {
            let _ = parse(&markdown[..end], &[]);
        }
    }
}
