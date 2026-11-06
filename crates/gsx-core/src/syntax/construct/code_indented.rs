use crate::syntax::construct::partial_space_or_tab::{space_or_tab, space_or_tab_min_max};
use crate::syntax::event::Name;
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::tokenizer::Tokenizer;
use crate::syntax::util::constant::TAB_SIZE;

pub fn start(tokenizer: &mut Tokenizer) -> State {
    if !tokenizer.interrupt && matches!(tokenizer.current, Some(b'\t' | b' ')) {
        tokenizer.enter(Name::CodeIndented);
        tokenizer.attempt(State::Next(StateName::CodeIndentedAtBreak), State::Nok);
        State::Retry(space_or_tab_min_max(tokenizer, TAB_SIZE, TAB_SIZE))
    } else {
        State::Nok
    }
}

pub fn at_break(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None => State::Retry(StateName::CodeIndentedAfter),
        Some(b'\n') => {
            tokenizer.attempt(State::Next(StateName::CodeIndentedAtBreak), State::Next(StateName::CodeIndentedAfter));
            State::Retry(StateName::CodeIndentedFurtherStart)
        }
        _ => {
            tokenizer.enter(Name::CodeFlowChunk);
            State::Retry(StateName::CodeIndentedInside)
        }
    }
}

pub fn inside(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n') => {
            tokenizer.exit(Name::CodeFlowChunk);
            State::Retry(StateName::CodeIndentedAtBreak)
        }
        _ => {
            tokenizer.consume();
            State::Next(StateName::CodeIndentedInside)
        }
    }
}

pub fn after(tokenizer: &mut Tokenizer) -> State {
    tokenizer.exit(Name::CodeIndented);
    tokenizer.interrupt = false;
    State::Ok
}

pub fn further_start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.lazy || tokenizer.pierce {
        return State::Nok;
    }

    if tokenizer.current == Some(b'\n') {
        tokenizer.enter(Name::LineEnding);
        tokenizer.consume();
        tokenizer.exit(Name::LineEnding);
        State::Next(StateName::CodeIndentedFurtherStart)
    } else {
        tokenizer.attempt(State::Ok, State::Next(StateName::CodeIndentedFurtherBegin));
        State::Retry(space_or_tab_min_max(tokenizer, TAB_SIZE, TAB_SIZE))
    }
}

pub fn further_begin(tokenizer: &mut Tokenizer) -> State {
    if matches!(tokenizer.current, Some(b'\t' | b' ')) {
        tokenizer.attempt(State::Next(StateName::CodeIndentedFurtherAfter), State::Nok);
        State::Retry(space_or_tab(tokenizer))
    } else {
        State::Nok
    }
}

pub fn further_after(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'\n') => State::Retry(StateName::CodeIndentedFurtherStart),
        _ => State::Nok,
    }
}
