use crate::syntax::construct::partial_space_or_tab_eol::{Options, space_or_tab_eol_with_options};
use crate::syntax::event::{Content, Link, Name};
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::subtokenize::link;
use crate::syntax::tokenizer::Tokenizer;
use crate::syntax::util::constant::LINK_REFERENCE_SIZE_MAX;

pub fn start(tokenizer: &mut Tokenizer) -> State {
    debug_assert_eq!(tokenizer.current, Some(b'['), "expected `[`");
    tokenizer.enter(tokenizer.tokenize_state.token_1);
    tokenizer.enter(tokenizer.tokenize_state.token_2);
    tokenizer.consume();
    tokenizer.exit(tokenizer.tokenize_state.token_2);
    tokenizer.enter(tokenizer.tokenize_state.token_3);
    State::Next(StateName::LabelAtBreak)
}

pub fn at_break(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.tokenize_state.size > LINK_REFERENCE_SIZE_MAX
        || matches!(tokenizer.current, None | Some(b'['))
        || (matches!(tokenizer.current, Some(b']')) && !tokenizer.tokenize_state.seen)
    {
        State::Retry(StateName::LabelNok)
    } else {
        match tokenizer.current {
            Some(b'\n') => {
                tokenizer.attempt(State::Next(StateName::LabelEolAfter), State::Next(StateName::LabelNok));
                State::Retry(space_or_tab_eol_with_options(
                    tokenizer,
                    Options { content: Some(Content::String), connect: tokenizer.tokenize_state.connect },
                ))
            }
            Some(b']') => {
                tokenizer.exit(tokenizer.tokenize_state.token_3);
                tokenizer.enter(tokenizer.tokenize_state.token_2);
                tokenizer.consume();
                tokenizer.exit(tokenizer.tokenize_state.token_2);
                tokenizer.exit(tokenizer.tokenize_state.token_1);
                tokenizer.tokenize_state.connect = false;
                tokenizer.tokenize_state.seen = false;
                tokenizer.tokenize_state.size = 0;
                State::Ok
            }
            _ => {
                tokenizer.enter_link(Name::Data, Link::new(Content::String));

                if tokenizer.tokenize_state.connect {
                    let index = tokenizer.events.len() - 1;
                    link(&mut tokenizer.events, index);
                } else {
                    tokenizer.tokenize_state.connect = true;
                }

                State::Retry(StateName::LabelInside)
            }
        }
    }
}

pub fn eol_after(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.connect = true;
    State::Retry(StateName::LabelAtBreak)
}

pub fn nok(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.connect = false;
    tokenizer.tokenize_state.seen = false;
    tokenizer.tokenize_state.size = 0;
    State::Nok
}

pub fn inside(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n' | b'[' | b']') => {
            tokenizer.exit(Name::Data);
            State::Retry(StateName::LabelAtBreak)
        }
        Some(byte) => {
            if tokenizer.tokenize_state.size > LINK_REFERENCE_SIZE_MAX {
                tokenizer.exit(Name::Data);
                State::Retry(StateName::LabelAtBreak)
            } else {
                tokenizer.consume();
                tokenizer.tokenize_state.size += 1;
                if !tokenizer.tokenize_state.seen && !matches!(byte, b'\t' | b' ') {
                    tokenizer.tokenize_state.seen = true;
                }
                State::Next(if matches!(byte, b'\\') { StateName::LabelEscape } else { StateName::LabelInside })
            }
        }
    }
}

pub fn escape(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'[' | b'\\' | b']') => {
            tokenizer.consume();
            tokenizer.tokenize_state.size += 1;
            State::Next(StateName::LabelInside)
        }
        _ => State::Retry(StateName::LabelInside),
    }
}
