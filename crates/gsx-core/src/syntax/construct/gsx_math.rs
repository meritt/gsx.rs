use crate::syntax::event::Name;
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::tokenizer::Tokenizer;
use crate::syntax::util::char::{after_index, whitespace};

pub fn start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b'$') && !tokenizer.previous.is_some_and(|byte| byte.is_ascii_digit()) {
        tokenizer.tokenize_state.seen = false;
        tokenizer.tokenize_state.gsx_math_trailing_ws = false;
        tokenizer.tokenize_state.gsx_math_slashes = 0;
        tokenizer.enter(Name::GsxMath);
        tokenizer.consume();
        State::Next(StateName::GsxMathInside)
    } else {
        State::Nok
    }
}

fn char_at(bytes: &[u8], index: usize, byte: u8) -> Option<char> {
    if byte.is_ascii() { Some(char::from(byte)) } else { after_index(bytes, index) }
}

pub fn inside(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None => State::Nok,
        Some(b'$') => {
            if !tokenizer.tokenize_state.seen {
                return State::Nok;
            }
            if tokenizer.tokenize_state.gsx_math_slashes % 2 == 1 {
                tokenizer.tokenize_state.gsx_math_slashes = 0;
                tokenizer.tokenize_state.gsx_math_trailing_ws = false;
                tokenizer.consume();
                return State::Next(StateName::GsxMathInside);
            }
            let bytes = &tokenizer.parse_state.bytes[..tokenizer.limit()];
            let closes = !tokenizer.tokenize_state.gsx_math_trailing_ws
                && !bytes.get(tokenizer.point.index as usize + 1).is_some_and(u8::is_ascii_digit);
            if closes {
                tokenizer.consume();
                tokenizer.exit(Name::GsxMath);
                State::Ok
            } else {
                State::Nok
            }
        }
        Some(b'\n') => {
            if !tokenizer.tokenize_state.seen {
                return State::Nok;
            }
            tokenizer.tokenize_state.gsx_math_slashes = 0;
            tokenizer.tokenize_state.gsx_math_trailing_ws = true;
            tokenizer.enter(Name::LineEnding);
            tokenizer.consume();
            tokenizer.exit(Name::LineEnding);
            State::Next(StateName::GsxMathInside)
        }
        Some(byte) => {
            if !tokenizer.tokenize_state.seen {
                if char_at(tokenizer.parse_state.bytes, tokenizer.point.index as usize, byte).is_none_or(whitespace) {
                    return State::Nok;
                }
                tokenizer.tokenize_state.seen = true;
            }
            if byte == b'\\' {
                tokenizer.tokenize_state.gsx_math_slashes += 1;
                tokenizer.tokenize_state.gsx_math_trailing_ws = false;
            } else if byte & 0xC0 != 0x80 {
                tokenizer.tokenize_state.gsx_math_slashes = 0;
                tokenizer.tokenize_state.gsx_math_trailing_ws =
                    char_at(tokenizer.parse_state.bytes, tokenizer.point.index as usize, byte).is_some_and(whitespace);
            }
            tokenizer.consume();
            State::Next(StateName::GsxMathInside)
        }
    }
}
