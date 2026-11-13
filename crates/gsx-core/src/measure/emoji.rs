use std::cmp::Ordering;

use super::tables::{EMOJI_PRESENTATION, RGI_LONGEST, RGI_SEQUENCES, RGI_SINGLE};

const SEQUENCE_END: u8 = b'\n';

pub(super) fn contains(inversion: &[u32], char: char) -> bool {
    let point = char as u32;
    point >= inversion[0] && inversion.partition_point(|&boundary| boundary <= point) % 2 == 1
}

fn continues(char: char) -> bool {
    matches!(char, '\u{FE0F}' | '\u{200D}' | '\u{1F3FB}'..='\u{1F3FF}' | '\u{E0020}'..='\u{E007F}' | '\u{1F1E6}'..='\u{1F1FF}')
}

fn longest_sequence(text: &[u8]) -> usize {
    let bytes = RGI_SEQUENCES.as_bytes();
    let mut key = text;
    while !key.is_empty() {
        let (mut low, mut high, mut below) = (0, bytes.len(), None);
        while low < high {
            let middle = usize::midpoint(low, high);
            let start = if middle == 0 || bytes[middle - 1] == SEQUENCE_END {
                middle
            } else {
                match bytes[middle..high].iter().position(|&byte| byte == SEQUENCE_END) {
                    Some(at) if middle + at + 1 < high => middle + at + 1,
                    _ => {
                        high = middle;
                        continue;
                    }
                }
            };
            let mut at = start;
            let order = loop {
                match (bytes[at], key.get(at - start)) {
                    (SEQUENCE_END, Some(_)) => break Ordering::Less,
                    (SEQUENCE_END, None) => break Ordering::Equal,
                    (byte, Some(&expected)) if byte == expected => at += 1,
                    (byte, Some(expected)) => break byte.cmp(expected),
                    (_, None) => break Ordering::Greater,
                }
            };
            match order {
                Ordering::Less => {
                    below = Some((at - start, bytes[at] == SEQUENCE_END));
                    low = at + 1;
                }
                Ordering::Greater => high = middle,
                Ordering::Equal => return key.len(),
            }
        }
        match below {
            Some((length, true)) => return length,
            Some((common, false)) => key = &key[..common],
            None => return 0,
        }
    }
    0
}

pub(super) fn presentation(char: char) -> bool {
    contains(&EMOJI_PRESENTATION, char)
}

pub(super) fn rgi_prefix(text: &str) -> usize {
    let mut chars = text.chars();
    let Some(first) = chars.next() else { return 0 };
    if chars.next().is_some_and(continues) {
        let end = text.char_indices().nth(RGI_LONGEST).map_or(text.len(), |(at, _)| at);
        let length = longest_sequence(&text.as_bytes()[..end]);
        if length > 0 {
            return length;
        }
    }
    if contains(&RGI_SINGLE, first) { first.len_utf8() } else { 0 }
}
