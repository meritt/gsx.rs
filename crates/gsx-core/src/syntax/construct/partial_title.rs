use crate::syntax::construct::partial_space_or_tab_eol::{Options, space_or_tab_eol_with_options};
use crate::syntax::event::{Content, Link, Name};
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::subtokenize::link;
use crate::syntax::tokenizer::Tokenizer;

pub fn start(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'"' | b'\'' | b'(') => {
            let marker = tokenizer.current.unwrap();
            tokenizer.tokenize_state.marker = if marker == b'(' { b')' } else { marker };
            tokenizer.enter(tokenizer.tokenize_state.token_1);
            tokenizer.enter(tokenizer.tokenize_state.token_2);
            tokenizer.consume();
            tokenizer.exit(tokenizer.tokenize_state.token_2);
            State::Next(StateName::TitleBegin)
        }
        _ => State::Nok,
    }
}

pub fn begin(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(tokenizer.tokenize_state.marker) {
        tokenizer.enter(tokenizer.tokenize_state.token_2);
        tokenizer.consume();
        tokenizer.exit(tokenizer.tokenize_state.token_2);
        tokenizer.exit(tokenizer.tokenize_state.token_1);
        tokenizer.tokenize_state.marker = 0;
        tokenizer.tokenize_state.connect = false;
        State::Ok
    } else {
        tokenizer.enter(tokenizer.tokenize_state.token_3);
        State::Retry(StateName::TitleAtBreak)
    }
}

pub fn at_break(tokenizer: &mut Tokenizer) -> State {
    if let Some(byte) = tokenizer.current {
        if byte == tokenizer.tokenize_state.marker {
            tokenizer.exit(tokenizer.tokenize_state.token_3);
            State::Retry(StateName::TitleBegin)
        } else if byte == b'\n' {
            tokenizer.attempt(State::Next(StateName::TitleAfterEol), State::Next(StateName::TitleNok));
            State::Retry(space_or_tab_eol_with_options(
                tokenizer,
                Options { content: Some(Content::String), connect: tokenizer.tokenize_state.connect },
            ))
        } else {
            tokenizer.enter_link(Name::Data, Link::new(Content::String));

            if tokenizer.tokenize_state.connect {
                let index = tokenizer.events.len() - 1;
                link(&mut tokenizer.events, index);
            } else {
                tokenizer.tokenize_state.connect = true;
            }

            State::Retry(StateName::TitleInside)
        }
    } else {
        State::Retry(StateName::TitleNok)
    }
}

pub fn after_eol(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.connect = true;
    State::Retry(StateName::TitleAtBreak)
}

pub fn nok(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.marker = 0;
    tokenizer.tokenize_state.connect = false;
    State::Nok
}

pub fn inside(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(tokenizer.tokenize_state.marker) || matches!(tokenizer.current, None | Some(b'\n')) {
        tokenizer.exit(Name::Data);
        State::Retry(StateName::TitleAtBreak)
    } else if tokenizer.tokenize_state.marker == b')' && tokenizer.current == Some(b'(') {
        State::Retry(StateName::TitleNok)
    } else {
        let name = if tokenizer.current == Some(b'\\') { StateName::TitleEscape } else { StateName::TitleInside };
        tokenizer.consume();
        State::Next(name)
    }
}

pub fn escape(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'"' | b'\'' | b'(' | b')' | b'\\') => {
            tokenizer.consume();
            State::Next(StateName::TitleInside)
        }
        _ => State::Retry(StateName::TitleInside),
    }
}
