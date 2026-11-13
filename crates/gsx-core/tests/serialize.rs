mod support;

use std::time::{Duration, Instant};

use gsx_core::document::{Document, Kind, Media, Node, Provenance, parse};
use gsx_core::serialize::{canonical, serialize};
use proptest::prelude::*;
use support::bench::{Bench, typical};
use support::markup::markup;
use support::{Json, LONE_SURROGATE};

const CORPUS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../testdata/corpus");

struct Case {
    id: String,
    markdown: String,
    media: Vec<(String, Option<String>)>,
}

fn corpus(file: &str) -> Vec<Case> {
    let text = std::fs::read_to_string(format!("{CORPUS}/{file}")).unwrap();
    text.lines()
        .filter(|line| !line.is_empty())
        .map(Json::parse)
        .filter_map(|record| {
            Some(Case {
                id: record.get("id").str()?.into(),
                markdown: record.get("markdown").str()?.into(),
                media: record
                    .get("media")
                    .array()
                    .iter()
                    .map(|item| (item.get("id").str().unwrap().into(), item.get("cover").str().map(Into::into)))
                    .collect(),
            })
        })
        .collect()
}

fn cases() -> Vec<Case> {
    let mut cases = corpus("document.jsonl");
    cases.extend(corpus("diagnostics.jsonl"));
    for (index, markdown) in typical().into_iter().enumerate() {
        cases.push(Case { id: format!("bench:typical:{index}"), markdown, media: vec![] });
    }
    let bench = Bench::load();
    for case in &bench.cases {
        if matches!(case.group.as_str(), "typical" | "long") {
            cases.push(Case { id: format!("bench:{}", case.id), markdown: bench.markdown(case, 16), media: vec![] });
        }
    }
    cases
}

fn merged(nodes: &[Node]) -> Vec<Node> {
    let mut out: Vec<Node> = Vec::with_capacity(nodes.len());
    for node in nodes {
        if let (Some(last), Kind::Text { value }) = (out.last_mut(), &node.kind)
            && node.provenance == Provenance::Authored
            && last.provenance == Provenance::Authored
            && let Kind::Text { value: previous } = &mut last.kind
        {
            previous.push_str(value);
            continue;
        }
        out.push(node.clone());
    }
    out
}

fn difference(path: &str, expected: &[Node], actual: &[Node]) -> Option<String> {
    let (expected, actual) = (&merged(expected), &merged(actual));
    if expected.len() != actual.len() {
        return Some(format!("{path}: {} children, reparsed {}", expected.len(), actual.len()));
    }
    for (index, (expected, actual)) in expected.iter().zip(actual).enumerate() {
        let path = format!("{path}/{index}");
        if expected.kind != actual.kind || expected.provenance != actual.provenance {
            return Some(format!(
                "{path}: {:?} {:?}, reparsed {:?} {:?}",
                expected.kind, expected.provenance, actual.kind, actual.provenance
            ));
        }
        if expected.gap_before != actual.gap_before {
            return Some(format!("{path}: gap {:?}, reparsed {:?}", expected.gap_before, actual.gap_before));
        }
        if let Some(difference) = difference(&path, &expected.children, &actual.children) {
            return Some(difference);
        }
    }
    None
}

fn round_trip(case: &Case, require_canonical: bool) -> Result<(), String> {
    let media: Vec<Media> = case.media.iter().map(|(id, cover)| Media { id, cover: cover.as_deref() }).collect();
    let Ok(document) = parse(&case.markdown, &media) else { return Ok(()) };
    if require_canonical && canonical(&document).is_none() {
        return Err(format!("{}: falls back to the source\n  source {:?}", case.id, case.markdown));
    }
    let serialized = serialize(&document);
    let reparsed: Document =
        parse(&serialized, &media).map_err(|error| format!("{}: reparse failed {error}\n{serialized:?}", case.id))?;
    if let Some(difference) = difference("", document.children(), reparsed.children()) {
        return Err(format!("{}: {difference}\n  source {:?}\n  serialized {serialized:?}", case.id, case.markdown));
    }
    let again = serialize(&reparsed);
    if again != serialized {
        return Err(format!("{}: not idempotent\n  first {serialized:?}\n  second {again:?}", case.id));
    }
    Ok(())
}

#[test]
fn round_trips_every_accepted_corpus_document_canonically() {
    let cases = cases();
    let failures: Vec<String> = cases
        .iter()
        .filter(|case| !case.markdown.contains(LONE_SURROGATE))
        .filter_map(|case| round_trip(case, true).err())
        .collect();
    assert!(
        failures.is_empty(),
        "{} of {} failed:\n{}",
        failures.len(),
        cases.len(),
        failures[..failures.len().min(25)].join("\n")
    );
}

proptest! {
    #![proptest_config(ProptestConfig { failure_persistence: None, ..ProptestConfig::with_cases(2048) })]

    #[test]
    fn serialize_after_parse_round_trips_and_is_idempotent(markdown in markup()) {
        let media = vec![("a".into(), None), ("b".into(), Some("c".into()))];
        if let Err(failure) = round_trip(&Case { id: "property".into(), markdown, media }, false) {
            prop_assert!(false, "{failure}");
        }
    }
}

fn seconds_per_byte(document: &Document, bytes: usize) -> f64 {
    let mut best = f64::MAX;
    for _ in 0..3 {
        let start = Instant::now();
        let mut runs = 0_u32;
        while runs == 0 || start.elapsed() < Duration::from_millis(5) {
            let _ = serialize(document);
            runs += 1;
        }
        best = best.min(start.elapsed().as_secs_f64() / f64::from(runs) / bytes as f64);
    }
    best
}

#[test]
fn serializes_bench_cases_in_linear_time() {
    let bench = Bench::load();
    let mut failures = vec![];
    for case in bench.cases.iter().filter(|case| case.group != "reject") {
        let cost = |divisor: usize| {
            let markdown = bench.markdown(case, divisor);
            parse(&markdown, &[]).ok().map(|document| seconds_per_byte(&document, markdown.len()))
        };
        if let (Some(small), Some(large)) = (cost(32), cost(8)) {
            let growth = large / small;
            if growth > 2.5 {
                failures.push(format!("{} costs {growth:.1} times more per byte at four times the size", case.id));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn escapes_dots_on_a_long_line_in_linear_time() {
    let cost = |count: usize| {
        let markdown = "a. ".repeat(count);
        seconds_per_byte(&parse(&markdown, &[]).unwrap(), markdown.len())
    };
    let growth = cost(84_000) / cost(21_000);
    assert!(growth < 2.0, "costs {growth:.1} times more per byte at four times the size");
}

fn canonically(source: &str) -> String {
    let media = [Media { id: "a", cover: None }, Media { id: "b", cover: None }];
    let document = parse(source, &media).unwrap();
    let serialized = canonical(&document).expect("the canonical form reparses to the same document");
    let reparsed = parse(&serialized, &media).unwrap();
    assert_eq!(difference("", document.children(), reparsed.children()), None, "{serialized:?}");
    assert_eq!(serialize(&reparsed), serialized);
    serialized
}

#[test]
fn round_trips_gsx_constructs() {
    assert_eq!(
        canonically("++u++ ==h== ||s|| ::time[t]{datetime=\"2026-01-01T00:00:00Z\"} ::time[n]{unix=1}"),
        "++u++ ==h== ||s|| ::time[t]{datetime=\"2026-01-01T00:00:00Z\"} ::time[n]{unix=\"1\"}"
    );
    assert_eq!(canonically("At\\:::time[noon]{unix=\"1\"}"), "At\\:::time[noon]{unix=\"1\"}");
    assert_eq!(canonically("At&#58;::time[noon]{unix=\"1\"}"), "At&#58;::time[noon]{unix=\"1\"}");
    assert_eq!(canonically("std&#58;:vector at 10:30"), "std:&#58;vector at 10:30");
    assert_eq!(
        canonically("::media{id=\"a\"}\n\n::media{ids=\"a,b\" layout=\"slideshow\"}"),
        "::media{id=\"a\"}\n\n::media{ids=\"a,b\" layout=\"slideshow\"}"
    );
}

#[test]
fn writes_canonical_markdown() {
    assert_eq!(canonically("Title\n=====\n\n* a\n* b\n\n+ c"), "# Title\n\n- a\n- b\n\n* c");
    assert_eq!(canonically("_a_ __b__ ***c***"), "*a* **b** ***c***");
    assert_eq!(canonically("3) x\n4) y"), "3. x\n4. y");
    assert_eq!(canonically("~~~js\n```\n~~~"), "````js\n```\n````");
    assert_eq!(canonically("a  \nb"), "a\\\nb");
    assert_eq!(canonically("|a|b|\n|:-|-:|\n|c|d|"), "| a | b |\n| :--- | ---: |\n| c | d |");
    assert_eq!(canonically("\\* not a list"), "\\* not a list");
    assert_eq!(canonically("&#42; not a list"), "&#42; not a list");
    assert_eq!(canonically("a <b>c</b> $x$ [y][z] ++d"), "a <b>c</b> $x$ [y][z] ++d");
    assert_eq!(canonically("a $5 and $10"), "a &#36;5 and &#36;10");
    assert_eq!(canonically("> a\n>\n> - b\n\n    c"), "> a\n>\n> - b\n\n```\nc\n```");
    assert_eq!(canonically("- [ ]   \n*$[a]: x\n  *$"), "- [ ]   \n*$[a]: x\n  *$");
}

#[test]
fn escapes_from_the_written_text_whatever_the_text_nodes() {
    for source in ["WWW..*<U", "www..", "wWw.*.", "1.*", "12)<"] {
        canonically(source);
    }
}

#[test]
fn falls_back_to_the_normalized_source() {
    let document = parse("+ &\r\n\r\n$__-->\r\n\\!}", &[]).unwrap();
    assert_eq!(canonical(&document), None);
    assert_eq!(serialize(&document), "+ &\n\n$__-->\n\\!}");
}
