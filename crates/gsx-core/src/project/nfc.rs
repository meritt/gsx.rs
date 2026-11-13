use std::borrow::Cow;

use icu_properties::CodePointMapData;
use icu_properties::props::{GeneralCategory, GeneralCategoryGroup};

pub use crate::measure::UNICODE_VERSION;

pub fn nfc(text: &str) -> Cow<'_, str> {
    icu_normalizer::ComposingNormalizerBorrowed::new_nfc().normalize(text)
}

pub(crate) fn starter(char: char) -> bool {
    !GeneralCategoryGroup::Mark.contains(CodePointMapData::<GeneralCategory>::new().get(char))
}

pub(crate) fn opens(char: char) -> bool {
    let mut buffer = [0; 4];
    char.is_ascii() || nfc(char.encode_utf8(&mut buffer)).chars().next().is_some_and(starter)
}

pub(crate) fn boundary(before: Option<char>, after: char) -> bool {
    if after.is_ascii() {
        return true;
    }
    let mut buffer = [0; 4];
    let after = nfc(after.encode_utf8(&mut buffer));
    after.chars().next().is_some_and(starter)
        && before.is_none_or(|before| {
            let mut pair = String::with_capacity(before.len_utf8() + after.len());
            pair.push(before);
            pair.push_str(&after);
            icu_normalizer::ComposingNormalizerBorrowed::new_nfc().is_normalized(&pair)
        })
}
