use crate::syntax::event::Name;
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::tokenizer::Tokenizer;
use crate::syntax::util::{
    character_reference::{decode_named, value_max, value_test},
    slice::Slice,
};

pub fn start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b'&') {
        tokenizer.enter(Name::CharacterReference);
        tokenizer.enter(Name::CharacterReferenceMarker);
        tokenizer.consume();
        tokenizer.exit(Name::CharacterReferenceMarker);
        State::Next(StateName::CharacterReferenceOpen)
    } else {
        State::Nok
    }
}

pub fn open(tokenizer: &mut Tokenizer) -> State {
    if let Some(b'#') = tokenizer.current {
        tokenizer.enter(Name::CharacterReferenceMarkerNumeric);
        tokenizer.consume();
        tokenizer.exit(Name::CharacterReferenceMarkerNumeric);
        State::Next(StateName::CharacterReferenceNumeric)
    } else {
        tokenizer.tokenize_state.marker = b'&';
        tokenizer.enter(Name::CharacterReferenceValue);
        State::Retry(StateName::CharacterReferenceValue)
    }
}

pub fn numeric(tokenizer: &mut Tokenizer) -> State {
    if let Some(b'x' | b'X') = tokenizer.current {
        tokenizer.enter(Name::CharacterReferenceMarkerHexadecimal);
        tokenizer.consume();
        tokenizer.exit(Name::CharacterReferenceMarkerHexadecimal);
        tokenizer.enter(Name::CharacterReferenceValue);
        tokenizer.tokenize_state.marker = b'x';
        State::Next(StateName::CharacterReferenceValue)
    } else {
        tokenizer.enter(Name::CharacterReferenceValue);
        tokenizer.tokenize_state.marker = b'#';
        State::Retry(StateName::CharacterReferenceValue)
    }
}

pub fn value(tokenizer: &mut Tokenizer) -> State {
    if matches!(tokenizer.current, Some(b';')) && tokenizer.tokenize_state.size > 0 {
        if tokenizer.tokenize_state.marker == b'&' {
            let slice = Slice::from_indices(
                tokenizer.parse_state.bytes,
                tokenizer.point.index as usize - tokenizer.tokenize_state.size,
                tokenizer.point.index as usize,
            );

            if decode_named(slice.as_str(), true).is_none() {
                tokenizer.tokenize_state.marker = 0;
                tokenizer.tokenize_state.size = 0;
                return State::Nok;
            }
        }

        tokenizer.exit(Name::CharacterReferenceValue);
        tokenizer.enter(Name::CharacterReferenceMarkerSemi);
        tokenizer.consume();
        tokenizer.exit(Name::CharacterReferenceMarkerSemi);
        tokenizer.exit(Name::CharacterReference);
        tokenizer.tokenize_state.marker = 0;
        tokenizer.tokenize_state.size = 0;
        return State::Ok;
    }

    if let Some(byte) = tokenizer.current
        && tokenizer.tokenize_state.size < value_max(tokenizer.tokenize_state.marker)
        && value_test(tokenizer.tokenize_state.marker)(&byte)
    {
        tokenizer.tokenize_state.size += 1;
        tokenizer.consume();
        return State::Next(StateName::CharacterReferenceValue);
    }

    tokenizer.tokenize_state.marker = 0;
    tokenizer.tokenize_state.size = 0;
    State::Nok
}
