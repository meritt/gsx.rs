use std::cmp::Ordering;

use super::tables::{EMOJI_PRESENTATION, RGI_ENDS, RGI_LONGEST, RGI_SEQUENCES, RGI_SINGLE};

fn contains(inversion: &[u32], char: char) -> bool {
    let point = char as u32;
    point >= inversion[0] && inversion.partition_point(|&boundary| boundary <= point) % 2 == 1
}

fn continues(char: char) -> bool {
    matches!(char, '\u{FE0F}' | '\u{200D}' | '\u{1F3FB}'..='\u{1F3FF}' | '\u{E0020}'..='\u{E007F}' | '\u{1F1E6}'..='\u{1F1FF}')
}

fn sequence(candidate: &str) -> bool {
    let candidate = candidate.as_bytes();
    let bytes = RGI_SEQUENCES.as_bytes();
    let (mut low, mut high) = (0, RGI_ENDS.len());
    while low < high {
        let middle = usize::midpoint(low, high);
        let start = if middle == 0 { 0 } else { usize::from(RGI_ENDS[middle - 1]) };
        match bytes[start..usize::from(RGI_ENDS[middle])].cmp(candidate) {
            Ordering::Less => low = middle + 1,
            Ordering::Greater => high = middle,
            Ordering::Equal => return true,
        }
    }
    false
}

pub(super) fn presentation(char: char) -> bool {
    contains(&EMOJI_PRESENTATION, char)
}

pub(super) fn rgi_prefix(text: &str) -> usize {
    let mut chars = text.chars();
    let Some(first) = chars.next() else { return 0 };
    if chars.next().is_some_and(continues) {
        let mut ends = [0; RGI_LONGEST];
        let mut count = 0;
        for (at, char) in text.char_indices().skip(1).take(RGI_LONGEST - 1) {
            ends[count] = at + char.len_utf8();
            count += 1;
        }
        if let Some(&end) = ends[..count].iter().rev().find(|&&end| sequence(&text[..end])) {
            return end;
        }
    }
    if contains(&RGI_SINGLE, first) { first.len_utf8() } else { 0 }
}
