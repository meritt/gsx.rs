use crate::source::Limit;
use crate::syntax::event::{Event, Kind, Name, Point};
use crate::syntax::resolve::Name as ResolveName;
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::subtokenize::Subresult;
use crate::syntax::tokenizer::Tokenizer;
use crate::syntax::util::char::{
    Kind as CharacterKind, after_index as char_after_index, before_index as char_before_index, classify_opt,
};

#[derive(Debug)]
struct Sequence {
    marker: u8,
    stack: Vec<usize>,
    index: usize,
    start_point: Point,
    end_point: Point,
    size: usize,
    length: usize,
    open: bool,
    close: bool,
}

pub fn start(tokenizer: &mut Tokenizer) -> State {
    if let Some(marker @ (b'*' | b'_' | b'~')) = tokenizer.current {
        let start = tokenizer.point.index as usize;
        if !tokenizer.spend(Limit::BlockDelimiters, start, |bytes| {
            start + bytes[start..].iter().take_while(|byte| **byte == marker).count()
        }) {
            return State::Nok;
        }
        tokenizer.tokenize_state.marker = marker;
        tokenizer.enter(Name::AttentionSequence);
        State::Retry(StateName::AttentionInside)
    } else {
        State::Nok
    }
}

pub fn inside(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(tokenizer.tokenize_state.marker) {
        tokenizer.consume();
        State::Next(StateName::AttentionInside)
    } else {
        tokenizer.exit(Name::AttentionSequence);
        tokenizer.register_resolver(ResolveName::Attention);
        tokenizer.tokenize_state.marker = 0;
        State::Ok
    }
}

pub fn resolve(tokenizer: &mut Tokenizer) -> Option<Subresult> {
    let mut sequences = get_sequences(tokenizer);

    let mut close = 0;

    while close < sequences.len() {
        let sequence_close = &sequences[close];
        let mut next_index = close + 1;

        if sequence_close.close {
            let mut open = close;

            while open > 0 {
                open -= 1;

                let sequence_open = &sequences[open];

                if sequence_open.open
                    && sequence_close.marker == sequence_open.marker
                    && sequence_close.stack == sequence_open.stack
                {
                    if (sequence_open.close || sequence_close.open)
                        && !sequence_close.length.is_multiple_of(3)
                        && (sequence_open.length + sequence_close.length).is_multiple_of(3)
                    {
                        continue;
                    }

                    if sequence_close.marker == b'~'
                        && (sequence_close.size != sequence_open.size
                            || sequence_close.size > 2
                            || sequence_close.size == 1)
                    {
                        continue;
                    }

                    next_index = match_sequences(tokenizer, &mut sequences, open, close);

                    break;
                }
            }
        }

        close = next_index;
    }

    let mut index = 0;
    while index < sequences.len() {
        let sequence = &sequences[index];
        tokenizer.events[sequence.index].name = Name::Data;
        tokenizer.events[sequence.index + 1].name = Name::Data;
        index += 1;
    }

    tokenizer.map.consume(&mut tokenizer.events);
    None
}

fn get_sequences(tokenizer: &mut Tokenizer) -> Vec<Sequence> {
    let mut index = 0;
    let mut stack = vec![];
    let mut sequences = vec![];

    while index < tokenizer.events.len() {
        let enter = &tokenizer.events[index];

        if enter.name == Name::AttentionSequence {
            if enter.kind == Kind::Enter {
                let end = index + 1;
                let exit = &tokenizer.events[end];

                let marker = tokenizer.parse_state.bytes[enter.point.index as usize];
                let before_char = if index == 0 || tokenizer.events[index - 1].point.index < enter.point.index {
                    None
                } else {
                    char_before_index(tokenizer.parse_state.bytes, enter.point.index as usize)
                };
                let before = classify_opt(before_char);
                let after_char = char_after_index(tokenizer.parse_state.bytes, exit.point.index as usize);
                let after = classify_opt(after_char);
                let is_marker = |char: Option<char>| matches!(char, Some('~' | '|' | '+' | '='));
                let open = after == CharacterKind::Other
                    || (after == CharacterKind::Punctuation && before != CharacterKind::Other)
                    || (marker != b'~' && is_marker(after_char));
                let close = before == CharacterKind::Other
                    || (before == CharacterKind::Punctuation && after != CharacterKind::Other)
                    || (marker != b'~' && is_marker(before_char));

                sequences.push(Sequence {
                    index,
                    stack: stack.clone(),
                    start_point: enter.point,
                    end_point: exit.point,
                    size: (exit.point.index - enter.point.index) as usize,
                    length: (exit.point.index - enter.point.index) as usize,
                    open: if marker == b'_' { open && (before != CharacterKind::Other || !close) } else { open },
                    close: if marker == b'_' { close && (after != CharacterKind::Other || !open) } else { close },
                    marker,
                });
            }
        } else if enter.kind == Kind::Enter {
            stack.push(index);
        } else {
            stack.pop();
        }

        index += 1;
    }

    sequences
}

#[allow(clippy::too_many_lines)]
fn match_sequences(tokenizer: &mut Tokenizer, sequences: &mut Vec<Sequence>, open: usize, close: usize) -> usize {
    let mut next = close;

    let take = if sequences[open].size > 1 && sequences[close].size > 1 { 2 } else { 1 };

    let mut between = open + 1;

    while between < close {
        sequences[between].open = false;
        between += 1;
    }

    let (group_name, seq_name, text_name) = if sequences[open].marker == b'~' {
        (Name::GfmStrikethrough, Name::GfmStrikethroughSequence, Name::GfmStrikethroughText)
    } else if take == 1 {
        (Name::Emphasis, Name::EmphasisSequence, Name::EmphasisText)
    } else {
        (Name::Strong, Name::StrongSequence, Name::StrongText)
    };
    let open_index = sequences[open].index;
    let close_index = sequences[close].index;
    let open_exit = sequences[open].end_point;
    let close_enter = sequences[close].start_point;

    sequences[open].size -= take;
    sequences[close].size -= take;
    sequences[open].end_point.column -= take as u32;
    sequences[open].end_point.index -= take as u32;
    sequences[close].start_point.column += take as u32;
    sequences[close].start_point.index += take as u32;

    tokenizer.map.add_before(
        open_index + 2,
        0,
        vec![
            Event { kind: Kind::Enter, name: group_name, point: sequences[open].end_point, link: None },
            Event { kind: Kind::Enter, name: seq_name, point: sequences[open].end_point, link: None },
            Event { kind: Kind::Exit, name: seq_name, point: open_exit, link: None },
            Event { kind: Kind::Enter, name: text_name, point: open_exit, link: None },
        ],
    );
    tokenizer.map.add(
        close_index,
        0,
        vec![
            Event { kind: Kind::Exit, name: text_name, point: close_enter, link: None },
            Event { kind: Kind::Enter, name: seq_name, point: close_enter, link: None },
            Event { kind: Kind::Exit, name: seq_name, point: sequences[close].start_point, link: None },
            Event { kind: Kind::Exit, name: group_name, point: sequences[close].start_point, link: None },
        ],
    );

    if sequences[close].size == 0 {
        sequences.remove(close);
        tokenizer.map.add(close_index, 2, vec![]);
    } else {
        tokenizer.events[close_index].point = sequences[close].start_point;
    }

    if sequences[open].size == 0 {
        sequences.remove(open);
        tokenizer.map.add(open_index, 2, vec![]);
        next -= 1;
    } else {
        tokenizer.events[open_index + 1].point = sequences[open].end_point;
    }

    next
}
