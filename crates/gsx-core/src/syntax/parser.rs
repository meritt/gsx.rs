use crate::source::Limit;
use crate::syntax::budget::{self, Exceeded};
use crate::syntax::event::{Event, Point};
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::subtokenize::subtokenize;
use crate::syntax::tokenizer::Tokenizer;
use std::cell::Cell;
use std::collections::HashSet;

#[derive(Debug)]
pub struct ParseState<'a> {
    pub bytes: &'a [u8],
    pub definitions: HashSet<String>,
    pub gfm_footnote_definitions: HashSet<String>,
    pub exceeded: Cell<Option<Exceeded>>,
}

impl ParseState<'_> {
    pub fn exceed(&self, limit: Limit, start: usize, end: usize) {
        if self.exceeded.get().is_none_or(|exceeded| start < exceeded.start) {
            self.exceeded.set(Some(Exceeded { limit, start, end }));
        }
    }
}

pub fn parse(value: &str) -> Result<Vec<Event>, Exceeded> {
    let bytes = value.as_bytes();
    if let Some(exceeded) = budget::lines(bytes) {
        return Err(exceeded);
    }
    let mut parse_state = ParseState {
        bytes,
        definitions: HashSet::new(),
        gfm_footnote_definitions: HashSet::new(),
        exceeded: Cell::new(None),
    };
    let start = Point { line: 1, column: 1, index: 0, vs: 0 };
    let mut tokenizer = Tokenizer::new(start, &parse_state);
    let state = tokenizer.push((0, 0), (parse_state.bytes.len() as u32, 0), State::Next(StateName::DocumentStart));
    let mut result = tokenizer.flush(state, true);
    let mut events = tokenizer.events;
    loop {
        parse_state.gfm_footnote_definitions.extend(result.gfm_footnote_definitions.drain(..));
        parse_state.definitions.extend(result.definitions.drain(..));
        if result.done {
            break;
        }
        result = subtokenize(&mut events, &parse_state, None);
    }
    budget::count(&events, &parse_state);
    match parse_state.exceeded.get() {
        Some(exceeded) => Err(exceeded),
        None => Ok(events),
    }
}
