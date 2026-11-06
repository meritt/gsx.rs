use crate::syntax::event::{Event, Kind, Point};
use crate::syntax::util::constant::TAB_SIZE;
use std::str;

#[derive(Debug)]
pub struct Position<'a> {
    pub start: &'a Point,
    pub end: &'a Point,
}

impl<'a> Position<'a> {
    pub fn from_exit_event(events: &'a [Event], index: usize) -> Position<'a> {
        debug_assert_eq!(events[index].kind, Kind::Exit, "expected `exit` event");
        let end = &events[index].point;
        let name = &events[index].name;
        let mut index = index - 1;

        while !(events[index].kind == Kind::Enter && events[index].name == *name) {
            index -= 1;
        }

        let start = &events[index].point;

        Position { start, end }
    }
}

#[derive(Debug)]
pub struct Slice<'a> {
    pub bytes: &'a [u8],
    pub before: usize,
    pub after: usize,
}

impl<'a> Slice<'a> {
    pub fn from_position(bytes: &'a [u8], position: &Position) -> Slice<'a> {
        let mut before = position.start.vs;
        let mut after = position.end.vs;
        let mut start = position.start.index;
        let mut end = position.end.index;

        if before > 0 {
            before = TAB_SIZE - before;
            start += 1;
        }

        if after > 0 {
            after -= 1;
            end += 1;
        }

        Slice { bytes: &bytes[start..end], before, after }
    }

    pub fn from_indices(bytes: &'a [u8], start: usize, end: usize) -> Slice<'a> {
        Slice { bytes: &bytes[start..end], before: 0, after: 0 }
    }

    pub fn len(&self) -> usize {
        self.bytes.len() + self.before + self.after
    }

    pub fn as_str(&self) -> &str {
        str::from_utf8(self.bytes).unwrap()
    }
}
