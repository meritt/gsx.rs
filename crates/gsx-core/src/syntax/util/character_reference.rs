use crate::syntax::util::constant::{
    CHARACTER_REFERENCE_DECIMAL_SIZE_MAX, CHARACTER_REFERENCE_HEXADECIMAL_SIZE_MAX, CHARACTER_REFERENCE_NAMED_SIZE_MAX,
    CHARACTER_REFERENCES, CHARACTER_REFERENCES_HTML_4,
};
use std::str;

pub fn decode_named(value: &str, html5: bool) -> Option<String> {
    let mut iter = if html5 { CHARACTER_REFERENCES.iter() } else { CHARACTER_REFERENCES_HTML_4.iter() };
    iter.find(|d| d.0 == value).map(|d| d.1.into())
}

pub fn decode_numeric(value: &str, radix: u32) -> String {
    if let Some(char) = u32::from_str_radix(value, radix).ok().and_then(char::from_u32)
        && !matches!(char,
            '\0'..='\u{08}' | '\u{0B}' | '\u{0E}'..='\u{1F}' |
            '\u{7F}'..='\u{9F}'
        )
    {
        return char.into();
    }

    char::REPLACEMENT_CHARACTER.into()
}

pub fn decode(value: &str, marker: u8, html5: bool) -> Option<String> {
    match marker {
        b'#' => Some(decode_numeric(value, 10)),
        b'x' => Some(decode_numeric(value, 16)),
        b'&' => decode_named(value, html5),
        _ => unreachable!("Unexpected marker `{}`", marker),
    }
}

pub fn value_max(marker: u8) -> usize {
    match marker {
        b'&' => CHARACTER_REFERENCE_NAMED_SIZE_MAX,
        b'x' => CHARACTER_REFERENCE_HEXADECIMAL_SIZE_MAX,
        b'#' => CHARACTER_REFERENCE_DECIMAL_SIZE_MAX,
        _ => unreachable!("Unexpected marker `{}`", marker),
    }
}

pub fn value_test(marker: u8) -> fn(&u8) -> bool {
    match marker {
        b'&' => u8::is_ascii_alphanumeric,
        b'x' => u8::is_ascii_hexdigit,
        b'#' => u8::is_ascii_digit,
        _ => unreachable!("Unexpected marker `{}`", marker),
    }
}
