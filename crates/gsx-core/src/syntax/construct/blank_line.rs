use crate::syntax::construct::partial_space_or_tab::space_or_tab;
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::tokenizer::Tokenizer;

pub fn start(tokenizer: &mut Tokenizer) -> State {
    if matches!(tokenizer.current, Some(b'\t' | b' ')) {
        tokenizer.attempt(State::Next(StateName::BlankLineAfter), State::Nok);
        State::Retry(space_or_tab(tokenizer))
    } else {
        State::Retry(StateName::BlankLineAfter)
    }
}

pub fn after(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n') => State::Ok,
        _ => State::Nok,
    }
}
