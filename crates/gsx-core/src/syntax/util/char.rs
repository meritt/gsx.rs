use icu_properties::CodePointMapData;
use icu_properties::props::{GeneralCategory, GeneralCategoryGroup};

const PUNCTUATION: GeneralCategoryGroup = GeneralCategoryGroup::Punctuation.union(GeneralCategoryGroup::Symbol);

#[derive(Debug, PartialEq, Eq)]
pub enum Kind {
    Whitespace,
    Punctuation,
    Other,
}

pub fn before_index(bytes: &[u8], index: usize) -> Option<char> {
    let start = index.saturating_sub(4);
    String::from_utf8_lossy(&bytes[start..index]).chars().last()
}

pub fn after_index(bytes: &[u8], index: usize) -> Option<char> {
    let end = if index + 4 > bytes.len() { bytes.len() } else { index + 4 };
    String::from_utf8_lossy(&bytes[index..end]).chars().next()
}

pub fn kind_after_index(bytes: &[u8], index: usize) -> Kind {
    if index == bytes.len() {
        Kind::Whitespace
    } else {
        let byte = bytes[index];
        if byte.is_ascii_whitespace() {
            Kind::Whitespace
        } else if byte.is_ascii_punctuation() {
            Kind::Punctuation
        } else if byte.is_ascii_alphanumeric() {
            Kind::Other
        } else {
            classify_opt(after_index(bytes, index))
        }
    }
}

pub fn whitespace(char: char) -> bool {
    (char.is_whitespace() && char != '\u{85}') || char == '\u{feff}'
}

pub fn classify(char: char) -> Kind {
    if whitespace(char) {
        Kind::Whitespace
    } else if char.is_ascii_punctuation()
        || (!char.is_ascii() && PUNCTUATION.contains(CodePointMapData::<GeneralCategory>::new().get(char)))
    {
        Kind::Punctuation
    } else {
        Kind::Other
    }
}

pub fn classify_opt(char_opt: Option<char>) -> Kind {
    char_opt.map_or(Kind::Whitespace, classify)
}
