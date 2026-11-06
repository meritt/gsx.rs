use crate::syntax::event::Name;
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::tokenizer::Tokenizer;

const BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

pub fn start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(BOM[0]) {
        tokenizer.enter(Name::ByteOrderMark);
        State::Retry(StateName::BomInside)
    } else {
        State::Nok
    }
}

pub fn inside(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(BOM[tokenizer.tokenize_state.size]) {
        tokenizer.tokenize_state.size += 1;
        tokenizer.consume();
        if tokenizer.tokenize_state.size == BOM.len() {
            tokenizer.exit(Name::ByteOrderMark);
            tokenizer.tokenize_state.size = 0;
            State::Ok
        } else {
            State::Next(StateName::BomInside)
        }
    } else {
        tokenizer.tokenize_state.size = 0;
        State::Nok
    }
}
