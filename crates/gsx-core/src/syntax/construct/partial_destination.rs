use crate::syntax::event::{Content, Link, Name};
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::tokenizer::Tokenizer;

pub fn start(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'<') => {
            tokenizer.enter(tokenizer.tokenize_state.token_1);
            tokenizer.enter(tokenizer.tokenize_state.token_2);
            tokenizer.enter(tokenizer.tokenize_state.token_3);
            tokenizer.consume();
            tokenizer.exit(tokenizer.tokenize_state.token_3);
            State::Next(StateName::DestinationEnclosedBefore)
        }
        None | Some(0x01..=0x1F | b' ' | b')' | 0x7F) => State::Nok,
        Some(_) => {
            tokenizer.enter(tokenizer.tokenize_state.token_1);
            tokenizer.enter(tokenizer.tokenize_state.token_4);
            tokenizer.enter(tokenizer.tokenize_state.token_5);
            tokenizer.enter_link(Name::Data, Link::new(Content::String));
            State::Retry(StateName::DestinationRaw)
        }
    }
}

pub fn enclosed_before(tokenizer: &mut Tokenizer) -> State {
    if let Some(b'>') = tokenizer.current {
        tokenizer.enter(tokenizer.tokenize_state.token_3);
        tokenizer.consume();
        tokenizer.exit(tokenizer.tokenize_state.token_3);
        tokenizer.exit(tokenizer.tokenize_state.token_2);
        tokenizer.exit(tokenizer.tokenize_state.token_1);
        State::Ok
    } else {
        tokenizer.enter(tokenizer.tokenize_state.token_5);
        tokenizer.enter_link(Name::Data, Link::new(Content::String));
        State::Retry(StateName::DestinationEnclosed)
    }
}

pub fn enclosed(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n' | b'<') => State::Nok,
        Some(b'>') => {
            tokenizer.exit(Name::Data);
            tokenizer.exit(tokenizer.tokenize_state.token_5);
            State::Retry(StateName::DestinationEnclosedBefore)
        }
        Some(b'\\') => {
            tokenizer.consume();
            State::Next(StateName::DestinationEnclosedEscape)
        }
        _ => {
            tokenizer.consume();
            State::Next(StateName::DestinationEnclosed)
        }
    }
}

pub fn enclosed_escape(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'<' | b'>' | b'\\') => {
            tokenizer.consume();
            State::Next(StateName::DestinationEnclosed)
        }
        _ => State::Retry(StateName::DestinationEnclosed),
    }
}

pub fn raw(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.tokenize_state.size == 0 && matches!(tokenizer.current, None | Some(b'\t' | b'\n' | b' ' | b')')) {
        tokenizer.exit(Name::Data);
        tokenizer.exit(tokenizer.tokenize_state.token_5);
        tokenizer.exit(tokenizer.tokenize_state.token_4);
        tokenizer.exit(tokenizer.tokenize_state.token_1);
        tokenizer.tokenize_state.size = 0;
        State::Ok
    } else if tokenizer.tokenize_state.size < tokenizer.tokenize_state.size_b && tokenizer.current == Some(b'(') {
        tokenizer.consume();
        tokenizer.tokenize_state.size += 1;
        State::Next(StateName::DestinationRaw)
    } else if tokenizer.current == Some(b')') {
        tokenizer.consume();
        tokenizer.tokenize_state.size -= 1;
        State::Next(StateName::DestinationRaw)
    } else if matches!(tokenizer.current, None | Some(0x01..=0x1F | b' ' | b'(' | 0x7F)) {
        tokenizer.tokenize_state.size = 0;
        State::Nok
    } else if tokenizer.current == Some(b'\\') {
        tokenizer.consume();
        State::Next(StateName::DestinationRawEscape)
    } else {
        tokenizer.consume();
        State::Next(StateName::DestinationRaw)
    }
}

pub fn raw_escape(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'(' | b')' | b'\\') => {
            tokenizer.consume();
            State::Next(StateName::DestinationRaw)
        }
        _ => State::Retry(StateName::DestinationRaw),
    }
}
