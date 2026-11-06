use crate::syntax::event::{Content, Link, Name};
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::subtokenize::link;
use crate::syntax::tokenizer::Tokenizer;

#[derive(Debug)]
pub struct Options {
    pub min: usize,
    pub max: usize,
    pub kind: Name,
    pub connect: bool,
    pub content: Option<Content>,
}

pub fn space_or_tab(tokenizer: &mut Tokenizer) -> StateName {
    space_or_tab_min_max(tokenizer, 1, usize::MAX)
}

pub fn space_or_tab_min_max(tokenizer: &mut Tokenizer, min: usize, max: usize) -> StateName {
    space_or_tab_with_options(tokenizer, Options { kind: Name::SpaceOrTab, min, max, content: None, connect: false })
}

pub fn space_or_tab_with_options(tokenizer: &mut Tokenizer, options: Options) -> StateName {
    tokenizer.tokenize_state.space_or_tab_connect = options.connect;
    tokenizer.tokenize_state.space_or_tab_content = options.content;
    tokenizer.tokenize_state.space_or_tab_min = options.min;
    tokenizer.tokenize_state.space_or_tab_max = options.max;
    tokenizer.tokenize_state.space_or_tab_token = options.kind;
    StateName::SpaceOrTabStart
}

pub fn start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.tokenize_state.space_or_tab_max > 0 && matches!(tokenizer.current, Some(b'\t' | b' ')) {
        if let Some(ref content) = tokenizer.tokenize_state.space_or_tab_content {
            tokenizer.enter_link(tokenizer.tokenize_state.space_or_tab_token, Link::new(*content));
        } else {
            tokenizer.enter(tokenizer.tokenize_state.space_or_tab_token);
        }

        if tokenizer.tokenize_state.space_or_tab_connect {
            let index = tokenizer.events.len() - 1;
            link(&mut tokenizer.events, index);
        } else {
            tokenizer.tokenize_state.space_or_tab_connect = true;
        }

        State::Retry(StateName::SpaceOrTabInside)
    } else {
        State::Retry(StateName::SpaceOrTabAfter)
    }
}

pub fn inside(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'\t' | b' ')
            if tokenizer.tokenize_state.space_or_tab_size < tokenizer.tokenize_state.space_or_tab_max =>
        {
            tokenizer.consume();
            tokenizer.tokenize_state.space_or_tab_size += 1;
            State::Next(StateName::SpaceOrTabInside)
        }
        _ => {
            tokenizer.exit(tokenizer.tokenize_state.space_or_tab_token);
            State::Retry(StateName::SpaceOrTabAfter)
        }
    }
}

pub fn after(tokenizer: &mut Tokenizer) -> State {
    let state = if tokenizer.tokenize_state.space_or_tab_size >= tokenizer.tokenize_state.space_or_tab_min {
        State::Ok
    } else {
        State::Nok
    };
    tokenizer.tokenize_state.space_or_tab_connect = false;
    tokenizer.tokenize_state.space_or_tab_content = None;
    tokenizer.tokenize_state.space_or_tab_size = 0;
    tokenizer.tokenize_state.space_or_tab_max = 0;
    tokenizer.tokenize_state.space_or_tab_min = 0;
    tokenizer.tokenize_state.space_or_tab_token = Name::SpaceOrTab;
    state
}
