use crate::syntax::event::Name;
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::tokenizer::Tokenizer;

pub fn start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b'\\') {
        tokenizer.enter(Name::CharacterEscape);
        tokenizer.enter(Name::CharacterEscapeMarker);
        tokenizer.consume();
        tokenizer.exit(Name::CharacterEscapeMarker);
        State::Next(StateName::CharacterEscapeInside)
    } else {
        State::Nok
    }
}

pub fn inside(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'!'..=b'/' | b':'..=b'@' | b'['..=b'`' | b'{'..=b'~') => {
            tokenizer.enter(Name::CharacterEscapeValue);
            tokenizer.consume();
            tokenizer.exit(Name::CharacterEscapeValue);
            tokenizer.exit(Name::CharacterEscape);
            State::Ok
        }
        _ => State::Nok,
    }
}
