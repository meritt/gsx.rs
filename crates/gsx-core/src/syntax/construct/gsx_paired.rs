use crate::syntax::event::{Event, Kind, Name, Point};
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::subtokenize::Subresult;
use crate::syntax::tokenizer::Tokenizer;
use std::collections::HashMap;

fn is_whitespace(byte: u8) -> bool {
    matches!(byte, b'\t' | b'\n' | b'\r' | b' ')
}

pub fn start(tokenizer: &mut Tokenizer) -> State {
    if matches!(tokenizer.current, Some(b'|' | b'+' | b'=')) {
        tokenizer.tokenize_state.marker = tokenizer.current.unwrap();
        tokenizer.tokenize_state.gsx_paired_close = matches!(tokenizer.previous, Some(byte) if !is_whitespace(byte));
        tokenizer.enter(Name::GsxPairedSequenceTemporary);
        tokenizer.consume();
        State::Next(StateName::GsxPairedSecond)
    } else {
        State::Nok
    }
}

pub fn second(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(tokenizer.tokenize_state.marker) {
        tokenizer.consume();
        State::Next(StateName::GsxPairedAfter)
    } else {
        tokenizer.tokenize_state.marker = 0;
        State::Nok
    }
}

pub fn after(tokenizer: &mut Tokenizer) -> State {
    tokenizer.exit(Name::GsxPairedSequenceTemporary);
    let start = tokenizer.events[tokenizer.events.len() - 2].point.index;
    let open = matches!(tokenizer.current, Some(byte) if !is_whitespace(byte));
    let close = tokenizer.tokenize_state.gsx_paired_close;
    tokenizer.tokenize_state.gsx_paired.push((start, open, close));
    tokenizer.tokenize_state.marker = 0;
    State::Ok
}

#[derive(Debug)]
struct Sequence {
    enter: usize,
    marker: u8,
    start: Point,
    end: Point,
    open: bool,
    close: bool,
    scope: Option<usize>,
}

#[derive(Default)]
struct Scope {
    opened: Vec<usize>,
    by_marker: [Vec<usize>; 3],
}

fn slot(marker: u8) -> usize {
    match marker {
        b'|' => 0,
        b'+' => 1,
        _ => 2,
    }
}

pub fn resolve(tokenizer: &mut Tokenizer) -> Option<Subresult> {
    let flags: HashMap<usize, (bool, bool)> =
        tokenizer.tokenize_state.gsx_paired.iter().rev().map(|&(start, open, close)| (start, (open, close))).collect();
    let mut sequences = vec![];
    let mut stack = vec![];
    let mut index = 0;
    while index < tokenizer.events.len() {
        let event = &tokenizer.events[index];
        if event.name == Name::GsxPairedSequenceTemporary {
            if event.kind == Kind::Enter {
                let exit = &tokenizer.events[index + 1];
                let (open, close) = flags.get(&event.point.index).copied().unwrap_or((false, false));
                sequences.push(Sequence {
                    enter: index,
                    marker: tokenizer.parse_state.bytes[event.point.index],
                    start: event.point,
                    end: exit.point,
                    open,
                    close,
                    scope: stack.last().copied(),
                });
            }
        } else if event.kind == Kind::Enter {
            stack.push(index);
        } else {
            stack.pop();
        }
        index += 1;
    }

    let mut pairs: Vec<(usize, usize)> = vec![];
    let mut scopes: HashMap<Option<usize>, Scope> = HashMap::new();
    for (at, sequence) in sequences.iter().enumerate() {
        let scope = scopes.entry(sequence.scope).or_default();
        let kind = slot(sequence.marker);
        if sequence.close {
            if let Some(&top) = scope.opened.last() {
                let top_sequence = &sequences[top];
                if top_sequence.marker == sequence.marker && top_sequence.end.index < sequence.start.index {
                    scope.opened.pop();
                    scope.by_marker[kind].pop();
                    pairs.push((top, at));
                    continue;
                }
            }
            if let Some(&matching) = scope.by_marker[kind].last() {
                scope.opened.truncate(matching);
                for positions in &mut scope.by_marker {
                    while positions.last().is_some_and(|&position| position >= matching) {
                        positions.pop();
                    }
                }
                continue;
            }
        }
        if sequence.open {
            scope.by_marker[kind].push(scope.opened.len());
            scope.opened.push(at);
        }
    }

    let mut paired = vec![false; sequences.len()];
    for &(open, close) in &pairs {
        paired[open] = true;
        paired[close] = true;
        let (group, opener, closer) = (&sequences[open], &sequences[open], &sequences[close]);
        let name = match group.marker {
            b'|' => Name::GsxSpoiler,
            b'+' => Name::GsxUnderline,
            _ => Name::GsxHighlight,
        };
        let event = |kind, name: &Name, point: &Point| Event { kind, name: *name, point: *point, link: None };
        tokenizer.map.add(
            opener.enter,
            2,
            vec![
                event(Kind::Enter, &name, &opener.start),
                event(Kind::Enter, &Name::GsxPairedSequence, &opener.start),
                event(Kind::Exit, &Name::GsxPairedSequence, &opener.end),
                event(Kind::Enter, &Name::GsxPairedText, &opener.end),
            ],
        );
        tokenizer.map.add(
            closer.enter,
            2,
            vec![
                event(Kind::Exit, &Name::GsxPairedText, &closer.start),
                event(Kind::Enter, &Name::GsxPairedSequence, &closer.start),
                event(Kind::Exit, &Name::GsxPairedSequence, &closer.end),
                event(Kind::Exit, &name, &closer.end),
            ],
        );
    }
    for (at, sequence) in sequences.iter().enumerate() {
        if !paired[at] {
            tokenizer.events[sequence.enter].name = Name::GsxPairedLiteral;
            tokenizer.events[sequence.enter + 1].name = Name::GsxPairedLiteral;
        }
    }
    tokenizer.map.consume(&mut tokenizer.events);
    None
}
