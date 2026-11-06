use crate::syntax::event::Name;
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::tokenizer::Tokenizer;

pub fn start(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'\n') => {
            tokenizer.enter(Name::LineEnding);
            tokenizer.consume();
            tokenizer.exit(Name::LineEnding);
            State::Next(StateName::NonLazyContinuationAfter)
        }
        _ => State::Nok,
    }
}

pub fn after(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.lazy { State::Nok } else { State::Ok }
}
