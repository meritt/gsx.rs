use crate::syntax::construct::partial_space_or_tab::space_or_tab;
use crate::syntax::event::Name;
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::tokenizer::Tokenizer;

pub fn start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.tokenize_state.document_at_first_paragraph_of_list_item
        && tokenizer.current == Some(b'[')
        && tokenizer.previous.is_none()
    {
        tokenizer.enter(Name::GfmTaskListItemCheck);
        tokenizer.enter(Name::GfmTaskListItemMarker);
        tokenizer.consume();
        tokenizer.exit(Name::GfmTaskListItemMarker);
        State::Next(StateName::GfmTaskListItemCheckInside)
    } else {
        State::Nok
    }
}

pub fn inside(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'\t' | b'\n' | b' ') => {
            tokenizer.enter(Name::GfmTaskListItemValueUnchecked);
            tokenizer.consume();
            tokenizer.exit(Name::GfmTaskListItemValueUnchecked);
            State::Next(StateName::GfmTaskListItemCheckClose)
        }
        Some(b'X' | b'x') => {
            tokenizer.enter(Name::GfmTaskListItemValueChecked);
            tokenizer.consume();
            tokenizer.exit(Name::GfmTaskListItemValueChecked);
            State::Next(StateName::GfmTaskListItemCheckClose)
        }
        _ => State::Nok,
    }
}

pub fn close(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b']') => {
            tokenizer.enter(Name::GfmTaskListItemMarker);
            tokenizer.consume();
            tokenizer.exit(Name::GfmTaskListItemMarker);
            tokenizer.exit(Name::GfmTaskListItemCheck);
            State::Next(StateName::GfmTaskListItemCheckAfter)
        }
        _ => State::Nok,
    }
}

pub fn after(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'\n') => State::Ok,
        Some(b'\t' | b' ') => {
            tokenizer.check(State::Ok, State::Nok);
            tokenizer.attempt(State::Next(StateName::GfmTaskListItemCheckAfterSpaceOrTab), State::Nok);
            State::Retry(space_or_tab(tokenizer))
        }
        _ => State::Nok,
    }
}

pub fn after_space_or_tab(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current.is_none() { State::Nok } else { State::Ok }
}
