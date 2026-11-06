mod support;

use gsx_core::project::links::bluesky::{self, Autolink};
use gsx_core::project::links::mastodon::{self, Mention};
use gsx_core::project::links::{accepted, twitter};
use support::{Json, LONE_SURROGATE};

#[test]
fn twitter_urls_scan_long_runs_linearly() {
    let text = format!("{}.{}", "字".repeat(24_000), "字".repeat(24_000));
    let units: Vec<u16> = text.encode_utf16().collect();
    assert_eq!(twitter::urls(&units), vec![]);
    let text = format!("{} example.com", "字".repeat(48_000));
    let units: Vec<u16> = text.encode_utf16().collect();
    assert_eq!(twitter::urls(&units), vec![48_001..48_012]);
    let text = "a.".repeat(24_000);
    let units: Vec<u16> = text.encode_utf16().collect();
    assert_eq!(twitter::urls(&units), vec![]);
    let text = format!("{}xn--a.{}", "a_".repeat(12_000), "a.".repeat(12_000));
    let units: Vec<u16> = text.encode_utf16().collect();
    assert_eq!(twitter::urls(&units), vec![]);
}

fn byte(text: &str, unit: usize) -> usize {
    let mut units = 0;
    for (at, char) in text.char_indices() {
        if units >= unit {
            return at;
        }
        units += char.len_utf16();
    }
    text.len()
}

#[test]
fn bluesky_autolinks_match_reference_under_one_url_path() {
    let path = format!("{}/tests/project/bluesky-autolinks.jsonl", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(path).unwrap();
    let mut failures = vec![];
    for line in text.lines().skip(1) {
        let case = Json::parse(line);
        let input = case.get("text").str().unwrap();
        let expected: Vec<Autolink> = case
            .get("found")
            .array()
            .iter()
            .filter_map(|item| match item.array() {
                [Json::Number(start), Json::Number(end), Json::String(kind), Json::String(value)] => {
                    let range = byte(input, *start as usize)..byte(input, *end as usize);
                    match kind.as_str() {
                        "link" => accepted(value).map(|uri| Autolink::Link { range, uri }),
                        _ => Some(Autolink::Tag { range, tag: value.clone() }),
                    }
                }
                _ => panic!("{line}"),
            })
            .collect();
        let actual = bluesky::autolinks(input);
        if actual != expected {
            failures.push(format!("{input:?}: expected {expected:?}, got {actual:?}"));
        }
    }
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures[..failures.len().min(30)].join("\n"));
}

#[test]
fn twitter_urls_match_reference() {
    let path = format!("{}/tests/project/twitter-urls.jsonl", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(path).unwrap();
    let mut failures = vec![];
    for line in text.lines().skip(1) {
        let case = Json::parse(line);
        let input = case.get("text").str().unwrap();
        let expected: Vec<(usize, usize)> = case
            .get("urls")
            .array()
            .iter()
            .map(|pair| match pair.array() {
                [Json::Number(start), Json::Number(end)] => (*start as usize, *end as usize),
                _ => panic!("{line}"),
            })
            .collect();
        let units: Vec<u16> = input.encode_utf16().collect();
        let actual: Vec<(usize, usize)> =
            twitter::urls(&units).into_iter().map(|range| (range.start, range.end)).collect();
        if actual != expected {
            failures.push(format!("{input:?}: expected {expected:?}, got {actual:?}"));
        }
    }
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures[..failures.len().min(30)].join("\n"));
}

fn index(item: &Json) -> usize {
    match item {
        Json::Number(value) => *value as usize,
        _ => panic!("{item:?}"),
    }
}

#[test]
fn mastodon_entities_match_reference() {
    let path = format!("{}/tests/project/mastodon-entities.jsonl", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(path).unwrap();
    let mut failures = vec![];
    for line in text.lines().skip(1) {
        let case = Json::parse(line);
        let input = case.get("text").str().unwrap();
        if input.contains(LONE_SURROGATE) {
            continue;
        }
        let units: Vec<u16> = input.encode_utf16().collect();
        let urls: Vec<_> =
            case.get("urls").array().iter().map(|pair| index(&pair.array()[0])..index(&pair.array()[1])).collect();
        let mentions: Vec<_> = case
            .get("mentions")
            .array()
            .iter()
            .map(|item| {
                let start = index(&item.array()[0]);
                Mention { range: start..index(&item.array()[1]), local: start + index(&item.array()[2]) }
            })
            .collect();
        let (found, mentioned) = (mastodon::urls(&units), mastodon::mentions(&units));
        if found != urls || mentioned != mentions {
            failures.push(format!("{input:?}: expected {urls:?} {mentions:?}, got {found:?} {mentioned:?}"));
        }
    }
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures[..failures.len().min(30)].join("\n"));
}
