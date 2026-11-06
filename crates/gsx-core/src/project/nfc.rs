use std::borrow::Cow;

pub use crate::measure::UNICODE_VERSION;

pub fn nfc(text: &str) -> Cow<'_, str> {
    icu_normalizer::ComposingNormalizerBorrowed::new_nfc().normalize(text)
}
