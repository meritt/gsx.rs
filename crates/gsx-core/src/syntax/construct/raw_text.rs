use crate::syntax::event::Name;
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::tokenizer::Tokenizer;

pub fn start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b'`')
        && (tokenizer.previous != tokenizer.current
            || (!tokenizer.events.is_empty()
                && tokenizer.events[tokenizer.events.len() - 1].name == Name::CharacterEscape))
    {
        tokenizer.tokenize_state.token_1 = Name::CodeText;
        tokenizer.tokenize_state.token_2 = Name::CodeTextSequence;
        tokenizer.tokenize_state.token_3 = Name::CodeTextData;
        tokenizer.tokenize_state.marker = b'`';
        tokenizer.enter(tokenizer.tokenize_state.token_1);
        tokenizer.enter(tokenizer.tokenize_state.token_2);
        State::Retry(StateName::RawTextSequenceOpen)
    } else {
        State::Nok
    }
}

pub fn sequence_open(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(tokenizer.tokenize_state.marker) {
        tokenizer.tokenize_state.size += 1;
        tokenizer.consume();
        State::Next(StateName::RawTextSequenceOpen)
    } else {
        let state = &tokenizer.tokenize_state;
        let start = tokenizer.point.index - state.size;
        if start >= state.raw_text_scanned && state.raw_text_runs.get(&state.size).is_none_or(|&at| at <= start) {
            return nok(tokenizer);
        }
        tokenizer.tokenize_state.scan_start = tokenizer.point.index;
        tokenizer.exit(tokenizer.tokenize_state.token_2);
        State::Retry(StateName::RawTextBetween)
    }
}

fn nok(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.marker = 0;
    tokenizer.tokenize_state.size = 0;
    tokenizer.tokenize_state.token_1 = Name::Data;
    tokenizer.tokenize_state.token_2 = Name::Data;
    tokenizer.tokenize_state.token_3 = Name::Data;
    State::Nok
}

pub fn between(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None => {
            let state = &mut tokenizer.tokenize_state;
            state.raw_text_scanned = state.raw_text_scanned.min(state.scan_start);
            nok(tokenizer)
        }
        Some(b'\n') => {
            tokenizer.enter(Name::LineEnding);
            tokenizer.consume();
            tokenizer.exit(Name::LineEnding);
            State::Next(StateName::RawTextBetween)
        }
        _ => {
            if tokenizer.current == Some(tokenizer.tokenize_state.marker) {
                tokenizer.enter(tokenizer.tokenize_state.token_2);
                State::Retry(StateName::RawTextSequenceClose)
            } else {
                tokenizer.enter(tokenizer.tokenize_state.token_3);
                State::Retry(StateName::RawTextData)
            }
        }
    }
}

pub fn data(tokenizer: &mut Tokenizer) -> State {
    if matches!(tokenizer.current, None | Some(b'\n')) || tokenizer.current == Some(tokenizer.tokenize_state.marker) {
        tokenizer.exit(tokenizer.tokenize_state.token_3);
        State::Retry(StateName::RawTextBetween)
    } else {
        tokenizer.consume();
        State::Next(StateName::RawTextData)
    }
}

pub fn sequence_close(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(tokenizer.tokenize_state.marker) {
        tokenizer.tokenize_state.size_b += 1;
        tokenizer.consume();
        State::Next(StateName::RawTextSequenceClose)
    } else {
        tokenizer.exit(tokenizer.tokenize_state.token_2);
        let size = tokenizer.tokenize_state.size_b;
        let at = tokenizer.tokenize_state.raw_text_runs.entry(size).or_default();
        *at = (*at).max(tokenizer.point.index - size);
        if tokenizer.tokenize_state.size == tokenizer.tokenize_state.size_b {
            tokenizer.exit(tokenizer.tokenize_state.token_1);
            tokenizer.tokenize_state.marker = 0;
            tokenizer.tokenize_state.size = 0;
            tokenizer.tokenize_state.size_b = 0;
            tokenizer.tokenize_state.token_1 = Name::Data;
            tokenizer.tokenize_state.token_2 = Name::Data;
            tokenizer.tokenize_state.token_3 = Name::Data;
            State::Ok
        } else {
            let len = tokenizer.events.len();
            tokenizer.events[len - 2].name = tokenizer.tokenize_state.token_3;
            tokenizer.events[len - 1].name = tokenizer.tokenize_state.token_3;
            tokenizer.tokenize_state.size_b = 0;
            State::Retry(StateName::RawTextBetween)
        }
    }
}
