use crate::source::Limit;
use crate::syntax::event::{Content, Event, Kind, Link, Name, Point, VOID_EVENTS};
use crate::syntax::parser::ParseState;
use crate::syntax::resolve::{Name as ResolveName, call as call_resolve};
use crate::syntax::state::{State, call};
use crate::syntax::subtokenize::Subresult;

use crate::syntax::util::{constant::TAB_SIZE, edit_map::EditMap};
use std::collections::HashMap;

#[derive(Debug, Eq, PartialEq)]
pub enum Container {
    BlockQuote,
    ListItem,
    GfmFootnoteDefinition,
}

#[derive(Debug)]
pub struct ContainerState {
    pub kind: Container,
    pub blank_initial: bool,
    pub size: usize,
}

#[derive(Debug, PartialEq)]
enum ByteAction {
    Normal(u8),
    Insert(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub enum LabelKind {
    Image,
    Link,
    GfmFootnote,
    GfmUndefinedFootnote,
}

#[derive(Debug)]
pub struct LabelStart {
    pub kind: LabelKind,
    pub start: (usize, usize),
    pub inactive: bool,
}

#[derive(Debug)]
pub struct Label {
    pub kind: LabelKind,
    pub start: (usize, usize),
    pub end: (usize, usize),
}

#[derive(Debug, PartialEq)]
enum AttemptKind {
    Attempt,
    Check,
}

#[derive(Debug)]
struct Attempt {
    ok: State,
    nok: State,
    kind: AttemptKind,
    progress: Option<Progress>,
}

#[derive(Clone, Debug)]
struct Progress {
    events_len: usize,
    stack_len: usize,
    previous: Option<u8>,
    current: Option<u8>,
    point: Point,
    line_start: Point,
}

#[derive(Debug)]
pub struct Markers([bool; 256]);

impl Markers {
    pub const fn new(bytes: &[u8]) -> Markers {
        let mut table = [false; 256];
        let mut index = 0;
        while index < bytes.len() {
            table[bytes[index] as usize] = true;
            index += 1;
        }
        Markers(table)
    }

    pub fn contains(&self, byte: u8) -> bool {
        self.0[usize::from(byte)]
    }
}

pub static NO_MARKERS: Markers = Markers::new(&[]);

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug)]
pub struct TokenizeState<'a> {
    pub document_child: Option<Box<Tokenizer<'a>>>,
    pub document_child_state: Option<State>,
    pub document_container_stack: Vec<ContainerState>,
    pub document_continued: usize,
    pub document_data_index: Option<usize>,
    pub document_exits: Vec<Option<Vec<Event>>>,
    pub document_lazy_accepting_before: bool,
    pub document_at_first_paragraph_of_list_item: bool,

    pub space_or_tab_eol_content: Option<Content>,
    pub space_or_tab_eol_connect: bool,
    pub space_or_tab_eol_ok: bool,
    pub space_or_tab_connect: bool,
    pub space_or_tab_content: Option<Content>,
    pub space_or_tab_min: usize,
    pub space_or_tab_max: usize,
    pub space_or_tab_size: usize,
    pub space_or_tab_token: Name,

    pub label_starts: Vec<LabelStart>,
    pub label_starts_loose: Vec<LabelStart>,
    pub labels: Vec<Label>,

    pub definitions: Vec<String>,
    pub gfm_footnote_definitions: Vec<String>,

    pub connect: bool,
    pub marker: u8,
    pub marker_b: u8,
    pub markers: &'static Markers,
    pub seen: bool,
    pub size: usize,
    pub size_b: usize,
    pub size_c: usize,
    pub start: usize,
    pub end: usize,
    pub token_1: Name,
    pub token_2: Name,
    pub token_3: Name,
    pub token_4: Name,
    pub token_5: Name,
    pub token_6: Name,

    pub gsx_paired: Vec<(usize, bool, bool)>,
    pub gsx_paired_close: bool,
    pub gfm_table_pipe_data: Vec<usize>,
    pub gfm_table_columns: usize,

    pub scan_start: usize,
    pub html_text_unclosed: [usize; 4],
    pub raw_text_runs: HashMap<usize, usize>,
    pub raw_text_scanned: usize,
    pub block_budget: [(usize, u32); 3],
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug)]
pub struct Tokenizer<'a> {
    column_start: Vec<(usize, usize)>,
    first_line: usize,
    line_start: Point,
    consumed: bool,
    pushed_to: usize,
    attempts: Vec<Attempt>,
    pub current: Option<u8>,
    pub previous: Option<u8>,
    pub point: Point,
    pub events: Vec<Event>,
    pub stack: Vec<Name>,
    pub map: EditMap,
    pub resolvers: Vec<ResolveName>,
    pub parse_state: &'a ParseState<'a>,
    pub tokenize_state: TokenizeState<'a>,
    pub interrupt: bool,
    pub concrete: bool,
    pub pierce: bool,
    pub lazy: bool,
}

impl<'a> Tokenizer<'a> {
    pub fn new(point: Point, parse_state: &'a ParseState) -> Tokenizer<'a> {
        Tokenizer {
            previous: None,
            current: None,
            column_start: vec![],
            first_line: point.line,
            line_start: point,
            consumed: true,
            pushed_to: 0,
            attempts: vec![],
            point,
            stack: vec![],
            events: vec![],
            parse_state,
            tokenize_state: TokenizeState {
                connect: false,
                document_container_stack: vec![],
                document_exits: vec![],
                document_continued: 0,
                document_lazy_accepting_before: false,
                document_data_index: None,
                document_child_state: None,
                document_child: None,
                document_at_first_paragraph_of_list_item: false,
                definitions: vec![],
                gfm_footnote_definitions: vec![],
                end: 0,
                label_starts: vec![],
                label_starts_loose: vec![],
                marker: 0,
                marker_b: 0,
                markers: &NO_MARKERS,
                labels: vec![],
                seen: false,
                size: 0,
                size_b: 0,
                size_c: 0,
                space_or_tab_eol_content: None,
                space_or_tab_eol_connect: false,
                space_or_tab_eol_ok: false,
                space_or_tab_connect: false,
                space_or_tab_content: None,
                space_or_tab_min: 0,
                space_or_tab_max: 0,
                space_or_tab_size: 0,
                space_or_tab_token: Name::SpaceOrTab,
                start: 0,
                token_1: Name::Data,
                token_2: Name::Data,
                token_3: Name::Data,
                token_4: Name::Data,
                token_5: Name::Data,
                token_6: Name::Data,
                gsx_paired: vec![],
                gsx_paired_close: false,
                gfm_table_pipe_data: vec![],
                gfm_table_columns: 0,
                scan_start: 0,
                html_text_unclosed: [usize::MAX; 4],
                raw_text_runs: HashMap::new(),
                raw_text_scanned: usize::MAX,
                block_budget: [(0, 0); 3],
            },
            map: EditMap::new(),
            interrupt: false,
            pierce: false,
            concrete: false,
            lazy: false,
            resolvers: vec![],
        }
    }

    pub fn register_resolver(&mut self, name: ResolveName) {
        if !self.resolvers.contains(&name) {
            self.resolvers.push(name);
        }
    }

    pub fn register_resolver_before(&mut self, name: ResolveName) {
        if !self.resolvers.contains(&name) {
            self.resolvers.insert(0, name);
        }
    }

    pub fn spend(&mut self, limit: Limit, start: usize, end: impl FnOnce(&[u8]) -> usize) -> bool {
        let slot = match limit {
            Limit::BlockDelimiters => 0,
            Limit::BlockLabels => 1,
            _ => 2,
        };
        let maximum = limit.maximum();
        let (next, count) = &mut self.tokenize_state.block_budget[slot];
        if *count > maximum {
            return false;
        }
        if start >= *next {
            let end = end(self.parse_state.bytes);
            *next = end.max(start + 1);
            *count = count.saturating_add(u32::try_from(end - start).unwrap_or(u32::MAX));
            if *count > maximum {
                self.parse_state.exceed(limit, start, end);
                return false;
            }
        }
        true
    }

    pub fn define_skip(&mut self, point: Point) {
        let info = (point.index, point.vs);

        let at = point.line - self.first_line;

        if at >= self.column_start.len() {
            self.column_start.push(info);
        } else {
            self.column_start[at] = info;
        }

        self.account_for_potential_skip();
    }

    fn account_for_potential_skip(&mut self) {
        let at = self.point.line - self.first_line;

        if self.point.column == 1 && at != self.column_start.len() {
            self.move_to(self.column_start[at]);
        }
    }

    fn expect(&mut self, byte: Option<u8>) {
        debug_assert!(self.consumed, "expected previous byte to be consumed");
        self.consumed = false;
        self.current = byte;
    }

    pub fn consume(&mut self) {
        debug_assert!(
            !self.consumed,
            "expected code to *not* have been consumed: this might be because `State::Retry(x)` instead of `State::Next(x)` was returned"
        );
        self.move_one();

        self.previous = self.current;
        self.current = None;
        self.consumed = true;
    }

    pub fn consume_while(&mut self, keep: impl Fn(u8) -> bool) {
        debug_assert!(self.current != Some(b'\n'), "expected a byte inside a line");
        self.consume();
        if self.point.vs != 0 {
            return;
        }
        let bytes = self.parse_state.bytes;
        let start = self.point.index;
        let end = self.pushed_to.min(bytes.len());
        let run = bytes.get(start..end).map_or(0, |rest| {
            rest.iter().position(|&byte| byte == b'\t' || byte == b'\n' || !keep(byte)).unwrap_or(rest.len())
        });
        if run > 0 {
            self.point.index += run;
            self.point.column += run;
            self.previous = Some(bytes[start + run - 1]);
        }
    }

    fn move_one(&mut self) {
        match byte_action(self.parse_state.bytes, &self.point) {
            ByteAction::Insert(byte) => {
                self.previous = Some(byte);
                self.point.column += 1;
                self.point.vs += 1;
            }
            ByteAction::Normal(byte) => {
                self.previous = Some(byte);
                self.point.vs = 0;
                self.point.index += 1;

                if byte == b'\n' {
                    self.point.line += 1;
                    self.point.column = 1;

                    if self.point.line - self.first_line + 1 > self.column_start.len() {
                        self.column_start.push((self.point.index, self.point.vs));
                    }

                    self.line_start = self.point;

                    self.account_for_potential_skip();
                } else {
                    self.point.column += 1;
                }
            }
        }
    }

    fn move_to(&mut self, to: (usize, usize)) {
        let (to_index, to_vs) = to;
        while self.point.index < to_index || self.point.index == to_index && self.point.vs < to_vs {
            self.move_one();
        }
    }

    pub fn enter(&mut self, name: Name) {
        enter_impl(self, name, None);
    }

    pub fn enter_link(&mut self, name: Name, link: Link) {
        enter_impl(self, name, Some(link));
    }

    pub fn exit(&mut self, name: Name) {
        let current = self.stack.pop().expect("cannot close w/o open tokens");

        debug_assert_eq!(current, name, "expected exit event to match current event");

        let previous = self.events.last().expect("cannot close w/o open event");
        let mut point = self.point;

        debug_assert!(
            current != previous.name || previous.point.index != point.index || previous.point.vs != point.vs,
            "expected non-empty event"
        );

        if VOID_EVENTS.iter().any(|d| d == &name) {
            debug_assert!(current == previous.name, "expected event to be void, instead of including something");
        }

        if matches!(self.previous, Some(b'\n')) {
            point = self.line_start;
        }

        let event = Event { kind: Kind::Exit, name, point, link: None };
        self.events.push(event);
    }

    fn capture(&mut self) -> Progress {
        Progress {
            previous: self.previous,
            current: self.current,
            point: self.point,
            line_start: self.line_start,
            events_len: self.events.len(),
            stack_len: self.stack.len(),
        }
    }

    fn free(&mut self, previous: Progress) {
        self.previous = previous.previous;
        self.current = previous.current;
        self.point = previous.point;
        self.line_start = previous.line_start;
        debug_assert!(self.events.len() >= previous.events_len, "expected to restore less events than before");
        self.events.truncate(previous.events_len);
        debug_assert!(self.stack.len() >= previous.stack_len, "expected to restore less stack items than before");
        self.stack.truncate(previous.stack_len);
    }

    pub fn check(&mut self, ok: State, nok: State) {
        let progress = Some(self.capture());
        let attempt = Attempt { kind: AttemptKind::Check, progress, ok, nok };
        self.attempts.push(attempt);
    }

    pub fn attempt(&mut self, ok: State, nok: State) {
        let progress = if nok == State::Nok { None } else { Some(self.capture()) };

        let attempt = Attempt { kind: AttemptKind::Attempt, progress, ok, nok };
        self.attempts.push(attempt);
    }

    pub fn push(&mut self, from: (usize, usize), to: (usize, usize), state: State) -> State {
        push_impl(self, from, to, state, false)
    }

    pub fn flush(&mut self, state: State, resolve: bool) -> Subresult {
        let to = (self.point.index, self.point.vs);
        let state = push_impl(self, to, to, state, true);
        debug_assert_eq!(state, State::Ok);

        let mut value = Subresult {
            done: false,
            gfm_footnote_definitions: self.tokenize_state.gfm_footnote_definitions.split_off(0),
            definitions: self.tokenize_state.definitions.split_off(0),
        };

        if resolve {
            let resolvers = self.resolvers.split_off(0);
            let mut index = 0;
            let defs = &mut value.definitions;
            let fn_defs = &mut value.gfm_footnote_definitions;
            while index < resolvers.len() {
                if let Some(mut result) = call_resolve(self, resolvers[index]) {
                    fn_defs.append(&mut result.gfm_footnote_definitions);
                    defs.append(&mut result.definitions);
                }
                index += 1;
            }

            self.map.consume(&mut self.events);
        }

        value
    }
}

fn enter_impl(tokenizer: &mut Tokenizer, name: Name, link: Option<Link>) {
    let point = tokenizer.point;

    tokenizer.stack.push(name);
    tokenizer.events.push(Event { kind: Kind::Enter, name, point, link });
}

fn push_impl(
    tokenizer: &mut Tokenizer,
    from: (usize, usize),
    to: (usize, usize),
    mut state: State,
    flush: bool,
) -> State {
    debug_assert!(
        from.0 > tokenizer.point.index || (from.0 == tokenizer.point.index && from.1 >= tokenizer.point.vs),
        "cannot move backwards"
    );

    tokenizer.move_to(from);
    tokenizer.pushed_to = to.0;

    loop {
        match state {
            State::Ok | State::Nok => {
                if let Some(attempt) = tokenizer.attempts.pop() {
                    if (attempt.kind == AttemptKind::Check || state == State::Nok)
                        && let Some(progress) = attempt.progress
                    {
                        tokenizer.free(progress);
                    }

                    tokenizer.consumed = true;

                    let next = if state == State::Ok { attempt.ok } else { attempt.nok };

                    state = next;
                } else {
                    break;
                }
            }
            State::Next(name) => {
                let byte =
                    if tokenizer.point.index < to.0 || (tokenizer.point.index == to.0 && tokenizer.point.vs < to.1) {
                        let (ByteAction::Insert(byte) | ByteAction::Normal(byte)) =
                            byte_action(tokenizer.parse_state.bytes, &tokenizer.point);
                        Some(byte)
                    } else if flush {
                        None
                    } else {
                        break;
                    };
                tokenizer.expect(byte);
                state = call(tokenizer, name);
            }
            State::Retry(name) => {
                state = call(tokenizer, name);
            }
        }
    }

    tokenizer.consumed = true;

    if flush {
        debug_assert!(matches!(state, State::Ok), "must be ok");
    } else {
        debug_assert!(matches!(state, State::Next(_)), "must have a next state");
    }

    state
}

fn byte_action(bytes: &[u8], point: &Point) -> ByteAction {
    if point.index < bytes.len() {
        let byte = bytes[point.index];

        if byte == b'\t' {
            let remainder = point.column % TAB_SIZE;
            let vs = if remainder == 0 { 0 } else { TAB_SIZE - remainder };

            if point.vs == 0 {
                if vs == 0 { ByteAction::Normal(byte) } else { ByteAction::Insert(byte) }
            } else if vs == 0 {
                ByteAction::Normal(b' ')
            } else {
                ByteAction::Insert(b' ')
            }
        } else {
            ByteAction::Normal(byte)
        }
    } else {
        unreachable!("out of bounds")
    }
}
