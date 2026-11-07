#![no_main]

use gsx_core::document::{Media, parse};
use gsx_core::wire;
use libfuzzer_sys::fuzz_target;
use serde_json::Value;

const MEDIA: &[Media<'static>] = &[Media { id: "a", cover: None }, Media { id: "b", cover: Some("c") }];

fn json(out: &str) -> Value {
    serde_json::from_str(out).unwrap_or_else(|error| panic!("wire output is not JSON ({error}): {out}"))
}

fuzz_target!(|data: &[u8]| {
    let wired = json(&wire::parse(data, MEDIA));
    let serialized = json(&wire::serialize(data, MEDIA));
    if wired.get("error").is_some() {
        assert_eq!(serialized, wired, "serialize reports another error than parse");
    }
    let Ok(text) = std::str::from_utf8(data) else {
        assert_eq!(wired["error"]["code"], "MARKDOWN_INVALID_TEXT", "invalid UTF-8 is accepted");
        return;
    };
    let first = parse(text, MEDIA);
    let second = parse(text, MEDIA);
    match (first, second) {
        (Ok(first), Ok(second)) => {
            let units = text.encode_utf16().count() as u32;
            assert!(first.children() == second.children() && first.markers() == second.markers());
            assert!(
                first
                    .markers()
                    .iter()
                    .all(|marker| marker.range.start <= marker.range.end && marker.range.end <= units)
            );
            assert!(
                first
                    .diagnostics()
                    .iter()
                    .all(|diagnostic| diagnostic.range.start <= diagnostic.range.end && diagnostic.range.end <= units)
            );
            assert!(wired.get("document").is_some(), "wire reports an error for an accepted document");
        }
        (Err(first), Err(second)) => {
            assert_eq!(first, second);
            assert_eq!(wired["error"]["code"], first.code.as_str(), "wire reports another error code");
        }
        (first, second) => panic!("outcome differs between runs: {first:?} / {second:?}"),
    }
});
