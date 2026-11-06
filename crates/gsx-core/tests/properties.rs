mod support;

use gsx_core::document::{Document, Media, Node, Range, parse};
use proptest::prelude::*;
use support::markup::{boundaries, markup};

const MEDIA: &[Media<'static>] = &[Media { id: "a", cover: None }, Media { id: "b", cover: Some("c") }];

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
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2048))]

    #[test]
    fn markup_parses_deterministically_within_bounds(text in markup()) {
        properties(&text)?;
    }

    #[test]
    fn any_text_parses_deterministically_within_bounds(text in any::<String>()) {
        properties(&text)?;
    }
}
