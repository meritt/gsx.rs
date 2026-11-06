use crate::syntax::event::{Kind, Name};
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::subtokenize::Subresult;
use crate::syntax::tokenizer::Tokenizer;

pub fn start(tokenizer: &mut Tokenizer) -> State {
    if let Some(byte) = tokenizer.current
        && tokenizer.tokenize_state.markers.contains(byte)
    {
        tokenizer.enter(Name::Data);
        tokenizer.consume();
        return State::Next(StateName::DataInside);
    }

    State::Retry(StateName::DataAtBreak)
}

pub fn at_break(tokenizer: &mut Tokenizer) -> State {
    if let Some(byte) = tokenizer.current
        && !tokenizer.tokenize_state.markers.contains(byte)
    {
        if byte == b'\n' {
            tokenizer.enter(Name::LineEnding);
            tokenizer.consume();
            tokenizer.exit(Name::LineEnding);
            return State::Next(StateName::DataAtBreak);
        }
        tokenizer.enter(Name::Data);
        return State::Retry(StateName::DataInside);
    }

    State::Ok
}

pub fn inside(tokenizer: &mut Tokenizer) -> State {
    if let Some(byte) = tokenizer.current
        && byte != b'\n'
        && (!tokenizer.tokenize_state.markers.contains(byte) || (byte == b'`' && tokenizer.previous == Some(b'`')))
    {
        let markers = tokenizer.tokenize_state.markers;
        tokenizer.consume_while(|byte| !markers.contains(byte));
        return State::Next(StateName::DataInside);
    }

    tokenizer.exit(Name::Data);
    State::Retry(StateName::DataAtBreak)
}

pub fn resolve(tokenizer: &mut Tokenizer) -> Option<Subresult> {
    let mut index = 0;

    while index < tokenizer.events.len() {
        let event = &tokenizer.events[index];

        if event.kind == Kind::Enter && event.name == Name::Data && event.link.is_none() {
            index += 1;

            let mut exit_index = index;

            while exit_index + 1 < tokenizer.events.len()
                && tokenizer.events[exit_index + 1].name == Name::Data
                && tokenizer.events[exit_index + 1].link.is_none()
            {
                exit_index += 2;
            }

            if exit_index > index {
                tokenizer.map.add(index, exit_index - index, vec![]);
                tokenizer.events[index].point = tokenizer.events[exit_index].point;
                index = exit_index;
            }
        }

        index += 1;
    }

    tokenizer.map.consume(&mut tokenizer.events);
    None
}
