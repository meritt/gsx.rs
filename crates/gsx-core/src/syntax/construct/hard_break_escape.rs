use crate::syntax::event::Name;
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::tokenizer::Tokenizer;

pub fn start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b'\\') {
        tokenizer.enter(Name::HardBreakEscape);
        tokenizer.consume();
        State::Next(StateName::HardBreakEscapeAfter)
    } else {
        State::Nok
    }
}

pub fn after(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'\n') => {
            tokenizer.exit(Name::HardBreakEscape);
            State::Ok
        }
        _ => State::Nok,
    }
}
