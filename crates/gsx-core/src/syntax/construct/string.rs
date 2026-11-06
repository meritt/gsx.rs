use crate::syntax::construct::partial_whitespace::resolve_whitespace;
use crate::syntax::resolve::Name as ResolveName;
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::subtokenize::Subresult;
use crate::syntax::tokenizer::{Markers, Tokenizer};

static MARKERS: Markers = Markers::new(b"&\\");

pub fn start(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.markers = &MARKERS;
    State::Retry(StateName::StringBefore)
}

pub fn before(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None => {
            tokenizer.register_resolver(ResolveName::Data);
            tokenizer.register_resolver(ResolveName::String);
            State::Ok
        }
        Some(b'&') => {
            tokenizer.attempt(State::Next(StateName::StringBefore), State::Next(StateName::StringBeforeData));
            State::Retry(StateName::CharacterReferenceStart)
        }
        Some(b'\\') => {
            tokenizer.attempt(State::Next(StateName::StringBefore), State::Next(StateName::StringBeforeData));
            State::Retry(StateName::CharacterEscapeStart)
        }
        _ => State::Retry(StateName::StringBeforeData),
    }
}

pub fn before_data(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(State::Next(StateName::StringBefore), State::Nok);
    State::Retry(StateName::DataStart)
}

pub fn resolve(tokenizer: &mut Tokenizer) -> Option<Subresult> {
    resolve_whitespace(tokenizer, false, false);
    None
}
