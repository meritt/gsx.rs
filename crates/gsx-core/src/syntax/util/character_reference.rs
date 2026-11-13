use crate::syntax::util::constant::{
    CHARACTER_REFERENCE_DECIMAL_SIZE_MAX, CHARACTER_REFERENCE_HEXADECIMAL_SIZE_MAX, CHARACTER_REFERENCE_NAMED_SIZE_MAX,
    CHARACTER_REFERENCES, CHARACTER_REFERENCES_HTML_4,
};
use std::cmp::Ordering;
use std::str;

const NAME_END: u8 = b';';
const RECORD_END: u8 = b'\0';
const FIRST: u8 = b'A';
const BUCKETS: usize = (b'z' - FIRST + 2) as usize;

const PACKED_SIZE: usize = {
    let mut size = 0;
    let mut index = 0;
    while index < CHARACTER_REFERENCES.len() {
        size += CHARACTER_REFERENCES[index].0.len() + CHARACTER_REFERENCES[index].1.len() + 2;
        index += 1;
    }
    size
};

static PACKED_BYTES: [u8; PACKED_SIZE] = {
    let mut packed = [0; PACKED_SIZE];
    let mut at = 0;
    let mut index = 0;
    while index < CHARACTER_REFERENCES.len() {
        let (name, value) = (CHARACTER_REFERENCES[index].0.as_bytes(), CHARACTER_REFERENCES[index].1.as_bytes());
        assert!(index == 0 || precedes(CHARACTER_REFERENCES[index - 1].0.as_bytes(), name));
        let mut byte = 0;
        while byte < name.len() {
            assert!(name[byte].is_ascii_alphanumeric());
            packed[at] = name[byte];
            (at, byte) = (at + 1, byte + 1);
        }
        packed[at] = NAME_END;
        at += 1;
        byte = 0;
        while byte < value.len() {
            assert!(value[byte] != RECORD_END);
            packed[at] = value[byte];
            (at, byte) = (at + 1, byte + 1);
        }
        packed[at] = RECORD_END;
        at += 1;
        index += 1;
    }
    packed
};

static PACKED: &str = match str::from_utf8(&PACKED_BYTES) {
    Ok(packed) => packed,
    Err(_) => panic!("character reference values are UTF-8"),
};

static BUCKET_STARTS: [u16; BUCKETS] = {
    assert!(PACKED_SIZE <= u16::MAX as usize);
    let mut buckets = [PACKED_SIZE as u16; BUCKETS];
    let (mut at, mut next, mut index) = (0, 0, 0);
    while index < CHARACTER_REFERENCES.len() {
        let (name, value) = CHARACTER_REFERENCES[index];
        while next <= (name.as_bytes()[0] - FIRST) as usize {
            buckets[next] = at as u16;
            next += 1;
        }
        at += name.len() + value.len() + 2;
        index += 1;
    }
    buckets
};

const fn precedes(left: &[u8], right: &[u8]) -> bool {
    let mut index = 0;
    while index < left.len() && index < right.len() {
        if left[index] != right[index] {
            return left[index] < right[index];
        }
        index += 1;
    }
    left.len() < right.len()
}

fn lookup(name: &str) -> Option<&'static str> {
    let (bytes, key) = (PACKED.as_bytes(), name.as_bytes());
    let bucket = usize::from(key.first()?.checked_sub(FIRST)?);
    let (mut low, mut high) = (usize::from(*BUCKET_STARTS.get(bucket)?), usize::from(*BUCKET_STARTS.get(bucket + 1)?));
    while low < high {
        let middle = usize::midpoint(low, high);
        let start = if middle == 0 || bytes[middle - 1] == RECORD_END {
            middle
        } else {
            match bytes[middle..high].iter().position(|&byte| byte == RECORD_END) {
                Some(at) if middle + at + 1 < high => middle + at + 1,
                _ => {
                    high = middle;
                    continue;
                }
            }
        };
        let mut at = start + 1;
        let order = loop {
            match (bytes[at], key.get(at - start)) {
                (NAME_END, Some(_)) => break Ordering::Less,
                (NAME_END, None) => break Ordering::Equal,
                (byte, Some(&expected)) if byte == expected => at += 1,
                (byte, Some(expected)) => break byte.cmp(expected),
                (_, None) => break Ordering::Greater,
            }
        };
        match order {
            Ordering::Less => low = at + 1,
            Ordering::Greater => high = middle,
            Ordering::Equal => {
                let end = at + bytes[at..].iter().position(|&byte| byte == RECORD_END)?;
                return PACKED.get(at + 1..end);
            }
        }
    }
    None
}

pub fn decode_named(value: &str, html5: bool) -> Option<&'static str> {
    if html5 {
        return lookup(value);
    }
    CHARACTER_REFERENCES_HTML_4.iter().find(|d| d.0 == value).map(|d| d.1)
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
        b'&' => decode_named(value, html5).map(String::from),
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
