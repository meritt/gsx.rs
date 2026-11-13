use std::collections::HashSet;

use gsx_core::project::nfc::{UNICODE_VERSION, nfc};

fn decode(column: &str) -> String {
    column.split(' ').map(|code| char::from_u32(u32::from_str_radix(code, 16).unwrap()).unwrap()).collect()
}

#[test]
fn normalization_test_passes() {
    let path = format!("{}/tests/project/NormalizationTest.txt", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(path).unwrap();
    let mut lines = text.lines();
    assert_eq!(lines.next(), Some(UNICODE_VERSION));
    let mut listed = HashSet::new();
    for line in lines {
        let columns: Vec<String> = line.split(';').map(decode).collect();
        let [source, composed, decomposed] = &columns[..] else { panic!("{line}") };
        if source.chars().count() == 1 {
            listed.insert(source.chars().next().unwrap());
        }
        for input in [source, composed, decomposed] {
            assert_eq!(nfc(input), composed.as_str(), "{line}");
        }
    }
    for char in (0..=0x10FFFF).filter_map(char::from_u32).filter(|char| !listed.contains(char)) {
        let text = char.to_string();
        assert_eq!(nfc(&text), text.as_str(), "{:X}", char as u32);
    }
}

#[test]
fn every_combining_class_belongs_to_a_mark() {
    let classes = icu_normalizer::properties::CanonicalCombiningClassMapBorrowed::new();
    let categories = icu_properties::CodePointMapData::<icu_properties::props::GeneralCategory>::new();
    for char in (0..=0x10FFFF).filter_map(char::from_u32).filter(|&char| classes.get_u8(char) != 0) {
        let category = categories.get(char);
        assert!(icu_properties::props::GeneralCategoryGroup::Mark.contains(category), "{:X}", char as u32);
    }
}
