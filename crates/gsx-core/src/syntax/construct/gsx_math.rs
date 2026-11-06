use crate::syntax::event::Name;
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::tokenizer::Tokenizer;
use crate::syntax::util::char::whitespace;

pub fn start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b'$') {
        tokenizer.tokenize_state.scan_start = tokenizer.point.index;
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
            let bytes = &tokenizer.parse_state.bytes[tokenizer.tokenize_state.scan_start + 1..tokenizer.point.index];
            let body = std::str::from_utf8(bytes).unwrap_or_default();
            let marked = body.chars().any(|char| whitespace(char) || char == '=' || char == '^');
            if marked && !body.chars().last().is_some_and(whitespace) {
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
