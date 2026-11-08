use std::str::CharIndices;

use super::tables::BREAK;

const OTHER: u8 = 0;
const CR: u8 = 1;
const LF: u8 = 2;
const CONTROL: u8 = 3;
const EXTEND: u8 = 4;
const ZWJ: u8 = 5;
const REGIONAL_INDICATOR: u8 = 6;
const PREPEND: u8 = 7;
const SPACING_MARK: u8 = 8;
const L: u8 = 9;
const V: u8 = 10;
const T: u8 = 11;
const LV: u8 = 12;
const LVT: u8 = 13;

const INCB_LINKER: u8 = 1;
const INCB_CONSONANT: u8 = 2;
const INCB_EXTEND: u8 = 3;

#[derive(Clone, Copy)]
struct Property(u8);

impl Property {
    fn class(self) -> u8 {
        self.0 & 0x0F
    }

    fn pictographic(self) -> bool {
        self.0 & 0x10 != 0
    }

    fn conjunct(self) -> u8 {
        self.0 >> 5
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Emoji {
    #[default]
    None,
    Pictographic,
    Joined,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Conjunct {
    #[default]
    None,
    Consonant,
    Linked,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Segmenter {
    previous: Option<u8>,
    regional_odd: bool,
    emoji: Emoji,
    conjunct: Conjunct,
    range: (u32, u32, u8),
}

impl Segmenter {
    fn property(&mut self, char: char) -> Property {
        let point = char as u32;
        if point < 0x7F {
            return Property(match point {
                0x0A => LF,
                0x0D => CR,
                0x00..0x20 => CONTROL,
                _ => OTHER,
            });
        }
        if !(self.range.0..self.range.1).contains(&point) {
            let index = BREAK.partition_point(|entry| entry >> 8 <= point) - 1;
            let end = BREAK.get(index + 1).map_or(u32::MAX, |entry| entry >> 8);
            self.range = (BREAK[index] >> 8, end, (BREAK[index] & 0xFF) as u8);
        }
        Property(self.range.2)
    }

    pub(crate) fn starts(&mut self, char: char) -> bool {
        let next = self.property(char);
        let class = next.class();
        let starts = self.previous.is_none_or(|previous| self.breaks(previous, next));
        self.regional_odd =
            class == REGIONAL_INDICATOR && !(self.previous == Some(REGIONAL_INDICATOR) && self.regional_odd);
        self.emoji = match (class, self.emoji) {
            _ if next.pictographic() => Emoji::Pictographic,
            (EXTEND, Emoji::Pictographic) => Emoji::Pictographic,
            (ZWJ, Emoji::Pictographic) => Emoji::Joined,
            _ => Emoji::None,
        };
        self.conjunct = match (next.conjunct(), self.conjunct) {
            (INCB_CONSONANT, _) => Conjunct::Consonant,
            (INCB_EXTEND, state) => state,
            (INCB_LINKER, Conjunct::Consonant | Conjunct::Linked) => Conjunct::Linked,
            _ => Conjunct::None,
        };
        self.previous = Some(class);
        starts
    }

    fn breaks(&self, previous: u8, next: Property) -> bool {
        match (previous, next.class()) {
            (CR, LF) => false,
            (CR | LF | CONTROL, _) | (_, CR | LF | CONTROL) => true,
            (L, L | V | LV | LVT) | (LV | V, V | T) | (LVT | T, T) => false,
            (_, EXTEND | ZWJ | SPACING_MARK) | (PREPEND, _) => false,
            _ if self.conjunct == Conjunct::Linked && next.conjunct() == INCB_CONSONANT => false,
            _ if self.emoji == Emoji::Joined && next.pictographic() => false,
            (REGIONAL_INDICATOR, REGIONAL_INDICATOR) => !self.regional_odd,
            _ => true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Clusters<'a> {
    text: &'a str,
    chars: CharIndices<'a>,
    segmenter: Segmenter,
    start: usize,
}

impl<'a> Iterator for Clusters<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        for (at, char) in self.chars.by_ref() {
            if self.segmenter.starts(char) && at > self.start {
                let cluster = &self.text[self.start..at];
                self.start = at;
                return Some(cluster);
            }
        }
        let cluster = &self.text[self.start..];
        self.start = self.text.len();
        (!cluster.is_empty()).then_some(cluster)
    }
}

pub fn clusters(text: &str) -> Clusters<'_> {
    Clusters { text, chars: text.char_indices(), segmenter: Segmenter::default(), start: 0 }
}
