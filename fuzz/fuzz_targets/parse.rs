#![no_main]

use gsx_core::document::{Media, parse};
use libfuzzer_sys::fuzz_target;

const MEDIA: &[Media<'static>] = &[Media { id: "a", cover: None }, Media { id: "b", cover: Some("c") }];

fuzz_target!(|text: &str| {
    let first = parse(text, MEDIA);
    let second = parse(text, MEDIA);
    match (first, second) {
        (Ok(first), Ok(second)) => {
            let units = text.encode_utf16().count() as u32;
            assert!(first.children() == second.children() && first.markers() == second.markers());
            assert!(first.markers().iter().all(|marker| marker.range.start <= marker.range.end && marker.range.end <= units));
            assert!(first.diagnostics().iter().all(|diagnostic| diagnostic.range.start <= diagnostic.range.end && diagnostic.range.end <= units));
        }
        (Err(first), Err(second)) => assert_eq!(first, second),
        (first, second) => panic!("outcome differs between runs: {first:?} / {second:?}"),
    }
});
