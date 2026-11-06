use crate::syntax::construct::partial_space_or_tab::{space_or_tab, space_or_tab_min_max};
use crate::syntax::event::Name;
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::tokenizer::Tokenizer;
use crate::syntax::util::char::{Kind, after_index, classify};
use crate::syntax::util::constant::TAB_SIZE;

pub fn candidate_end(bytes: &[u8], start: usize) -> Option<usize> {
    if start > 0 && bytes[start - 1] == b':' {
        return None;
    }
    let slashes = bytes[..start].iter().rev().take_while(|b| **b == b'\\').count();
    if slashes % 2 == 1 {
        return None;
    }
    let mut end = start;
    while end < bytes.len() && bytes[end] == b':' {
        end += 1;
    }
    if end - start < 2 || !bytes[end..].get(..5)?.eq_ignore_ascii_case(b"media") {
        return None;
    }
    end += 5;
    if let Some(next) = after_index(bytes, end)
        && (next == '-' || next == '_' || classify(next) == Kind::Other)
    {
        return None;
    }
    while end < bytes.len() && matches!(bytes[end], b'{' | b'[') {
        let close = if bytes[end] == b'{' { b'}' } else { b']' };
        end += 1;
        let mut quote = 0;
        while end < bytes.len() && bytes[end] != b'\n' {
            let byte = bytes[end];
            end += 1;
            if byte == b'\\' && end < bytes.len() && bytes[end] != b'\n' {
                end += 1;
            } else if quote != 0 {
                if byte == quote {
                    quote = 0;
                }
            } else if byte == b'"' || byte == b'\'' {
                quote = byte;
            } else if byte == close {
                break;
            }
        }
    }
    Some(end)
}

pub fn flow_start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.interrupt {
        return State::Nok;
    }
    if matches!(tokenizer.current, Some(b'\t' | b' ')) {
        tokenizer.attempt(State::Next(StateName::GsxMediaFlowBefore), State::Nok);
        State::Retry(space_or_tab_min_max(tokenizer, 0, TAB_SIZE - 1))
    } else {
        State::Retry(StateName::GsxMediaFlowBefore)
    }
}

pub fn flow_before(tokenizer: &mut Tokenizer) -> State {
    let bytes = tokenizer.parse_state.bytes;
    if tokenizer.current != Some(b':') {
        return State::Nok;
    }
    let Some(end) = candidate_end(bytes, tokenizer.point.index) else {
        return State::Nok;
    };
    let mut line_end = end;
    while line_end < bytes.len() && matches!(bytes[line_end], b' ' | b'\t') {
        line_end += 1;
    }
    if line_end < bytes.len() && bytes[line_end] != b'\n' {
        return State::Nok;
    }
    tokenizer.tokenize_state.end = end;
    tokenizer.tokenize_state.seen = true;
    tokenizer.enter(Name::GsxMedia);
    State::Retry(StateName::GsxMediaInside)
}

pub fn text_start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current != Some(b':') || tokenizer.previous == Some(b':') {
        return State::Nok;
    }
    let Some(end) = candidate_end(tokenizer.parse_state.bytes, tokenizer.point.index) else {
        return State::Nok;
    };
    tokenizer.tokenize_state.end = end;
    tokenizer.tokenize_state.seen = false;
    tokenizer.enter(Name::GsxMedia);
    State::Retry(StateName::GsxMediaInside)
}

pub fn inside(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.point.index < tokenizer.tokenize_state.end {
        if tokenizer.current.is_none() {
            return State::Nok;
        }
        tokenizer.consume();
        return State::Next(StateName::GsxMediaInside);
    }
    tokenizer.exit(Name::GsxMedia);
    tokenizer.tokenize_state.end = 0;
    if !tokenizer.tokenize_state.seen {
        return State::Ok;
    }
    tokenizer.tokenize_state.seen = false;
    if matches!(tokenizer.current, Some(b'\t' | b' ')) {
        tokenizer.attempt(State::Next(StateName::GsxMediaFlowAfter), State::Nok);
        State::Retry(space_or_tab(tokenizer))
    } else {
        State::Retry(StateName::GsxMediaFlowAfter)
    }
}

pub fn flow_after(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n') => State::Ok,
        _ => State::Nok,
    }
}
