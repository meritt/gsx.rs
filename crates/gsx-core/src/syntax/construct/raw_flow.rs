use crate::syntax::construct::partial_space_or_tab::{space_or_tab, space_or_tab_min_max};
use crate::syntax::event::{Content, Link, Name};
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::tokenizer::Tokenizer;
use crate::syntax::util::{
    constant::{CODE_FENCED_SEQUENCE_SIZE_MIN, TAB_SIZE},
    slice::{Position, Slice},
};

pub fn start(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'\t' | b' ') => {
            tokenizer.attempt(State::Next(StateName::RawFlowBeforeSequenceOpen), State::Nok);
            State::Retry(space_or_tab_min_max(tokenizer, 0, TAB_SIZE - 1))
        }
        Some(b'`' | b'~') => State::Retry(StateName::RawFlowBeforeSequenceOpen),
        _ => State::Nok,
    }
}

pub fn before_sequence_open(tokenizer: &mut Tokenizer) -> State {
    let tail = tokenizer.events.last();
    let mut prefix = 0;

    if let Some(event) = tail
        && event.name == Name::SpaceOrTab
    {
        prefix = Slice::from_position(
            tokenizer.parse_state.bytes,
            &Position::from_exit_event(&tokenizer.events, tokenizer.events.len() - 1),
        )
        .len();
    }

    if matches!(tokenizer.current, Some(b'`' | b'~')) {
        tokenizer.tokenize_state.marker = tokenizer.current.unwrap();
        tokenizer.tokenize_state.size_c = prefix;
        tokenizer.tokenize_state.token_1 = Name::CodeFenced;
        tokenizer.tokenize_state.token_2 = Name::CodeFencedFence;
        tokenizer.tokenize_state.token_3 = Name::CodeFencedFenceSequence;
        tokenizer.tokenize_state.token_4 = Name::CodeFencedFenceInfo;
        tokenizer.tokenize_state.token_5 = Name::CodeFencedFenceMeta;
        tokenizer.tokenize_state.token_6 = Name::CodeFlowChunk;

        tokenizer.enter(tokenizer.tokenize_state.token_1);
        tokenizer.enter(tokenizer.tokenize_state.token_2);
        tokenizer.enter(tokenizer.tokenize_state.token_3);
        State::Retry(StateName::RawFlowSequenceOpen)
    } else {
        State::Nok
    }
}

pub fn sequence_open(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(tokenizer.tokenize_state.marker) {
        tokenizer.tokenize_state.size += 1;
        tokenizer.consume();
        State::Next(StateName::RawFlowSequenceOpen)
    } else if tokenizer.tokenize_state.size < CODE_FENCED_SEQUENCE_SIZE_MIN {
        tokenizer.tokenize_state.marker = 0;
        tokenizer.tokenize_state.size_c = 0;
        tokenizer.tokenize_state.size = 0;
        tokenizer.tokenize_state.token_1 = Name::Data;
        tokenizer.tokenize_state.token_2 = Name::Data;
        tokenizer.tokenize_state.token_3 = Name::Data;
        tokenizer.tokenize_state.token_4 = Name::Data;
        tokenizer.tokenize_state.token_5 = Name::Data;
        tokenizer.tokenize_state.token_6 = Name::Data;
        State::Nok
    } else {
        let next = StateName::RawFlowInfoBefore;

        if matches!(tokenizer.current, Some(b'\t' | b' ')) {
            tokenizer.exit(tokenizer.tokenize_state.token_3);
            tokenizer.attempt(State::Next(next), State::Nok);
            State::Retry(space_or_tab(tokenizer))
        } else {
            tokenizer.exit(tokenizer.tokenize_state.token_3);
            State::Retry(next)
        }
    }
}

pub fn info_before(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n') => {
            tokenizer.exit(tokenizer.tokenize_state.token_2);
            tokenizer.concrete = true;
            tokenizer.check(State::Next(StateName::RawFlowAtNonLazyBreak), State::Next(StateName::RawFlowAfter));
            State::Retry(StateName::NonLazyContinuationStart)
        }
        _ => {
            tokenizer.enter(tokenizer.tokenize_state.token_4);
            tokenizer.enter_link(Name::Data, Link::new(Content::String));
            State::Retry(StateName::RawFlowInfo)
        }
    }
}

pub fn info(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n') => {
            tokenizer.exit(Name::Data);
            tokenizer.exit(tokenizer.tokenize_state.token_4);
            State::Retry(StateName::RawFlowInfoBefore)
        }
        Some(b'\t' | b' ') => {
            tokenizer.exit(Name::Data);
            tokenizer.exit(tokenizer.tokenize_state.token_4);
            tokenizer.attempt(State::Next(StateName::RawFlowMetaBefore), State::Nok);
            State::Retry(space_or_tab(tokenizer))
        }
        Some(byte) => {
            if tokenizer.tokenize_state.marker == byte && matches!(byte, b'$' | b'`') {
                tokenizer.concrete = false;
                tokenizer.tokenize_state.marker = 0;
                tokenizer.tokenize_state.size_c = 0;
                tokenizer.tokenize_state.size = 0;
                tokenizer.tokenize_state.token_1 = Name::Data;
                tokenizer.tokenize_state.token_2 = Name::Data;
                tokenizer.tokenize_state.token_3 = Name::Data;
                tokenizer.tokenize_state.token_4 = Name::Data;
                tokenizer.tokenize_state.token_5 = Name::Data;
                tokenizer.tokenize_state.token_6 = Name::Data;
                State::Nok
            } else {
                tokenizer.consume();
                State::Next(StateName::RawFlowInfo)
            }
        }
    }
}

pub fn meta_before(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n') => State::Retry(StateName::RawFlowInfoBefore),
        _ => {
            tokenizer.enter(tokenizer.tokenize_state.token_5);
            tokenizer.enter_link(Name::Data, Link::new(Content::String));
            State::Retry(StateName::RawFlowMeta)
        }
    }
}

pub fn meta(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n') => {
            tokenizer.exit(Name::Data);
            tokenizer.exit(tokenizer.tokenize_state.token_5);
            State::Retry(StateName::RawFlowInfoBefore)
        }
        Some(byte) => {
            if tokenizer.tokenize_state.marker == byte && matches!(byte, b'$' | b'`') {
                tokenizer.concrete = false;
                tokenizer.tokenize_state.marker = 0;
                tokenizer.tokenize_state.size_c = 0;
                tokenizer.tokenize_state.size = 0;
                tokenizer.tokenize_state.token_1 = Name::Data;
                tokenizer.tokenize_state.token_2 = Name::Data;
                tokenizer.tokenize_state.token_3 = Name::Data;
                tokenizer.tokenize_state.token_4 = Name::Data;
                tokenizer.tokenize_state.token_5 = Name::Data;
                tokenizer.tokenize_state.token_6 = Name::Data;
                State::Nok
            } else {
                tokenizer.consume();
                State::Next(StateName::RawFlowMeta)
            }
        }
    }
}

pub fn at_non_lazy_break(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(State::Next(StateName::RawFlowAfter), State::Next(StateName::RawFlowContentBefore));
    tokenizer.enter(Name::LineEnding);
    tokenizer.consume();
    tokenizer.exit(Name::LineEnding);
    State::Next(StateName::RawFlowCloseStart)
}

pub fn close_start(tokenizer: &mut Tokenizer) -> State {
    tokenizer.enter(tokenizer.tokenize_state.token_2);

    if matches!(tokenizer.current, Some(b'\t' | b' ')) {
        tokenizer.attempt(State::Next(StateName::RawFlowBeforeSequenceClose), State::Nok);

        State::Retry(space_or_tab_min_max(tokenizer, 0, TAB_SIZE - 1))
    } else {
        State::Retry(StateName::RawFlowBeforeSequenceClose)
    }
}

pub fn before_sequence_close(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(tokenizer.tokenize_state.marker) {
        tokenizer.enter(tokenizer.tokenize_state.token_3);
        State::Retry(StateName::RawFlowSequenceClose)
    } else {
        State::Nok
    }
}

pub fn sequence_close(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(tokenizer.tokenize_state.marker) {
        tokenizer.tokenize_state.size_b += 1;
        tokenizer.consume();
        State::Next(StateName::RawFlowSequenceClose)
    } else if tokenizer.tokenize_state.size_b >= tokenizer.tokenize_state.size {
        tokenizer.tokenize_state.size_b = 0;
        tokenizer.exit(tokenizer.tokenize_state.token_3);

        if matches!(tokenizer.current, Some(b'\t' | b' ')) {
            tokenizer.attempt(State::Next(StateName::RawFlowAfterSequenceClose), State::Nok);
            State::Retry(space_or_tab(tokenizer))
        } else {
            State::Retry(StateName::RawFlowAfterSequenceClose)
        }
    } else {
        tokenizer.tokenize_state.size_b = 0;
        State::Nok
    }
}

pub fn sequence_close_after(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n') => {
            tokenizer.exit(tokenizer.tokenize_state.token_2);
            State::Ok
        }
        _ => State::Nok,
    }
}

pub fn content_before(tokenizer: &mut Tokenizer) -> State {
    tokenizer.enter(Name::LineEnding);
    tokenizer.consume();
    tokenizer.exit(Name::LineEnding);
    State::Next(StateName::RawFlowContentStart)
}

pub fn content_start(tokenizer: &mut Tokenizer) -> State {
    if matches!(tokenizer.current, Some(b'\t' | b' ')) {
        tokenizer.attempt(State::Next(StateName::RawFlowBeforeContentChunk), State::Nok);
        State::Retry(space_or_tab_min_max(tokenizer, 0, tokenizer.tokenize_state.size_c))
    } else {
        State::Retry(StateName::RawFlowBeforeContentChunk)
    }
}

pub fn before_content_chunk(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n') => {
            tokenizer.check(State::Next(StateName::RawFlowAtNonLazyBreak), State::Next(StateName::RawFlowAfter));
            State::Retry(StateName::NonLazyContinuationStart)
        }
        _ => {
            tokenizer.enter(tokenizer.tokenize_state.token_6);
            State::Retry(StateName::RawFlowContentChunk)
        }
    }
}

pub fn content_chunk(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n') => {
            tokenizer.exit(tokenizer.tokenize_state.token_6);
            State::Retry(StateName::RawFlowBeforeContentChunk)
        }
        _ => {
            tokenizer.consume();
            State::Next(StateName::RawFlowContentChunk)
        }
    }
}

pub fn after(tokenizer: &mut Tokenizer) -> State {
    tokenizer.exit(tokenizer.tokenize_state.token_1);
    tokenizer.tokenize_state.marker = 0;
    tokenizer.tokenize_state.size_c = 0;
    tokenizer.tokenize_state.size = 0;
    tokenizer.tokenize_state.token_1 = Name::Data;
    tokenizer.tokenize_state.token_2 = Name::Data;
    tokenizer.tokenize_state.token_3 = Name::Data;
    tokenizer.tokenize_state.token_4 = Name::Data;
    tokenizer.tokenize_state.token_5 = Name::Data;
    tokenizer.tokenize_state.token_6 = Name::Data;
    tokenizer.interrupt = false;
    tokenizer.concrete = false;
    State::Ok
}
