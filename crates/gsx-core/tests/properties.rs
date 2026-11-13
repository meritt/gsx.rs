mod support;
#[path = "support/schema.rs"]
mod validator;

use std::sync::LazyLock;

use gsx_core::document::{Document, Media, Node, Range, parse};
use gsx_core::wire;
use proptest::prelude::*;
use support::markup::{boundaries, markup};
use validator::{Schema, deep};

const MEDIA: &[Media<'static>] = &[Media { id: "a", cover: None }, Media { id: "b", cover: Some("c") }];

static SCHEMA: LazyLock<Schema> = LazyLock::new(Schema::load);

fn check(range: Range, boundaries: &[bool], what: &str) -> Result<(), TestCaseError> {
    prop_assert!(range.start <= range.end, "{what} {range:?} is reversed");
    prop_assert!((range.end as usize) < boundaries.len(), "{what} {range:?} ends past {}", boundaries.len() - 1);
    prop_assert!(
        boundaries[range.start as usize] && boundaries[range.end as usize],
        "{what} {range:?} splits a surrogate pair"
    );
    Ok(())
}

fn sourced(document: &Document, range: Range, boundaries: &[bool], what: &str) -> Result<(), TestCaseError> {
    check(range, boundaries, what)?;
    document.source(range);
    Ok(())
}

fn nodes(document: &Document, children: &[Node], boundaries: &[bool]) -> Result<(), TestCaseError> {
    for node in children {
        sourced(document, node.range, boundaries, "node")?;
        nodes(document, &node.children, boundaries)?;
    }
    Ok(())
}

fn ranges(document: &Document, input: &str) -> Result<(), TestCaseError> {
    let boundaries = boundaries(input);
    nodes(document, document.children(), &boundaries)?;
    for marker in document.markers() {
        sourced(document, marker.range, &boundaries, "marker")?;
    }
    for diagnostic in document.diagnostics() {
        sourced(document, diagnostic.range, &boundaries, "diagnostic")?;
    }
    Ok(())
}

fn properties(text: &str) -> Result<(), TestCaseError> {
    let first = parse(text, MEDIA);
    let second = parse(text, MEDIA);
    match (&first, &second) {
        (Ok(first), Ok(second)) => {
            prop_assert_eq!(first.text(), second.text());
            prop_assert_eq!(first.children(), second.children());
            prop_assert_eq!(first.markers(), second.markers());
            prop_assert_eq!(first.diagnostics(), second.diagnostics());
            ranges(first, text)?;
        }
        (Err(first), Err(second)) => {
            prop_assert_eq!(first, second);
            if let Some(range) = first.range {
                check(range, &boundaries(text), "error")?;
            }
        }
        _ => prop_assert!(false, "outcome differs between runs: {first:?} / {second:?}"),
    }
    let outputs = [wire::parse(text.as_bytes(), MEDIA), wire::serialize(text.as_bytes(), MEDIA)];
    deep(|| outputs.iter().try_for_each(|out| SCHEMA.check_wire(out))).map_err(TestCaseError::fail)
}

#[test]
fn source_and_point_clamp_offsets_to_the_document() {
    let document = parse("hello world", &[]).unwrap();
    assert_eq!(document.source(Range { start: 5, end: 2 }), "");
    assert_eq!(document.source(Range { start: 0, end: 99 }), "hello world");
    assert_eq!(document.source(Range { start: 99, end: u32::MAX }), "");
    assert_eq!(document.point(u32::MAX), document.end());
    let crlf = parse("é😀\r\nb", &[]).unwrap();
    assert_eq!(crlf.source(Range { start: 0, end: 99 }), "é😀\r\nb");
    assert_eq!(crlf.source(Range { start: 3, end: u32::MAX }), "\r\nb");
    assert_eq!(crlf.point(u32::MAX), crlf.end());
}

fn offset() -> impl Strategy<Value = u32> {
    prop_oneof![0u32..64, Just(u32::MAX), any::<u32>()]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2048))]

    #[test]
    fn any_range_reads_the_clamped_source(text in markup(), start in offset(), end in offset()) {
        if let Ok(document) = parse(&text, MEDIA) {
            let units = document.end().offset;
            let clamped = Range { start: start.min(end).min(units), end: end.min(units) };
            prop_assert_eq!(document.source(Range { start, end }), document.source(clamped));
            prop_assert_eq!(document.point(start), document.point(start.min(units)));
        }
    }

    #[test]
    fn markup_parses_deterministically_within_bounds(text in markup()) {
        properties(&text)?;
    }

    #[test]
    fn any_text_parses_deterministically_within_bounds(text in any::<String>()) {
        properties(&text)?;
    }
}
