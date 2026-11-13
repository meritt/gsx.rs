use crate::syntax::event::Name;
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::tokenizer::Tokenizer;
use crate::syntax::util::char::whitespace;

pub fn start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b'$') && !tokenizer.previous.is_some_and(|byte| byte.is_ascii_digit()) {
        tokenizer.tokenize_state.scan_start = tokenizer.point.index as usize;
        tokenizer.enter(Name::GsxMath);
        tokenizer.consume();
        State::Next(StateName::GsxMathInside)
    } else {
        State::Nok
    }
}

pub fn inside(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None => State::Nok,
        Some(b'$') => {
            let bytes = tokenizer.parse_state.bytes;
            let body =
                std::str::from_utf8(&bytes[tokenizer.tokenize_state.scan_start + 1..tokenizer.point.index as usize])
                    .unwrap_or_default();
            let escaped = body.bytes().rev().take_while(|&byte| byte == b'\\').count() % 2 == 1;
            if escaped {
                tokenizer.consume();
                return State::Next(StateName::GsxMathInside);
            }
            let edges = body.chars().next().zip(body.chars().next_back());
            let closes = edges.is_some_and(|(first, last)| !whitespace(first) && !whitespace(last))
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
            tokenizer.enter(Name::LineEnding);
            tokenizer.consume();
            tokenizer.exit(Name::LineEnding);
            State::Next(StateName::GsxMathInside)
        }
        _ => {
            tokenizer.consume();
            State::Next(StateName::GsxMathInside)
        }
    }
}
