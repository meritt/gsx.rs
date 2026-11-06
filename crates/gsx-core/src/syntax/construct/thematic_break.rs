use crate::syntax::construct::partial_space_or_tab::{space_or_tab, space_or_tab_min_max};
use crate::syntax::event::Name;
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::tokenizer::Tokenizer;
use crate::syntax::util::constant::{TAB_SIZE, THEMATIC_BREAK_MARKER_COUNT_MIN};

pub fn start(tokenizer: &mut Tokenizer) -> State {
    tokenizer.enter(Name::ThematicBreak);

    if matches!(tokenizer.current, Some(b'\t' | b' ')) {
        tokenizer.attempt(State::Next(StateName::ThematicBreakBefore), State::Nok);
        State::Retry(space_or_tab_min_max(tokenizer, 0, TAB_SIZE - 1))
    } else {
        State::Retry(StateName::ThematicBreakBefore)
    }
}

pub fn before(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'*' | b'-' | b'_') => {
            tokenizer.tokenize_state.marker = tokenizer.current.unwrap();
            State::Retry(StateName::ThematicBreakAtBreak)
        }
        _ => State::Nok,
    }
}

pub fn at_break(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(tokenizer.tokenize_state.marker) {
        tokenizer.enter(Name::ThematicBreakSequence);
        State::Retry(StateName::ThematicBreakSequence)
    } else if tokenizer.tokenize_state.size >= THEMATIC_BREAK_MARKER_COUNT_MIN
        && matches!(tokenizer.current, None | Some(b'\n'))
    {
        tokenizer.tokenize_state.marker = 0;
        tokenizer.tokenize_state.size = 0;
        tokenizer.exit(Name::ThematicBreak);
        tokenizer.interrupt = false;
        State::Ok
    } else {
        tokenizer.tokenize_state.marker = 0;
        tokenizer.tokenize_state.size = 0;
        State::Nok
    }
}

pub fn sequence(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(tokenizer.tokenize_state.marker) {
        tokenizer.consume();
        tokenizer.tokenize_state.size += 1;
        State::Next(StateName::ThematicBreakSequence)
    } else if matches!(tokenizer.current, Some(b'\t' | b' ')) {
        tokenizer.exit(Name::ThematicBreakSequence);
        tokenizer.attempt(State::Next(StateName::ThematicBreakAtBreak), State::Nok);
        State::Retry(space_or_tab(tokenizer))
    } else {
        tokenizer.exit(Name::ThematicBreakSequence);
        State::Retry(StateName::ThematicBreakAtBreak)
    }
}
