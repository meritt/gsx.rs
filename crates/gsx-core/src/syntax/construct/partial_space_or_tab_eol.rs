use crate::syntax::construct::partial_space_or_tab::{Options as SpaceOrTabOptions, space_or_tab_with_options};
use crate::syntax::event::{Content, Link, Name};
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::subtokenize::link;
use crate::syntax::tokenizer::Tokenizer;

#[derive(Debug)]
pub struct Options {
    pub connect: bool,
    pub content: Option<Content>,
}

pub fn space_or_tab_eol(tokenizer: &mut Tokenizer) -> StateName {
    space_or_tab_eol_with_options(tokenizer, Options { content: None, connect: false })
}

pub fn space_or_tab_eol_with_options(tokenizer: &mut Tokenizer, options: Options) -> StateName {
    tokenizer.tokenize_state.space_or_tab_eol_content = options.content;
    tokenizer.tokenize_state.space_or_tab_eol_connect = options.connect;
    StateName::SpaceOrTabEolStart
}

pub fn start(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'\t' | b' ') => {
            tokenizer
                .attempt(State::Next(StateName::SpaceOrTabEolAfterFirst), State::Next(StateName::SpaceOrTabEolAtEol));

            State::Retry(space_or_tab_with_options(
                tokenizer,
                SpaceOrTabOptions {
                    kind: Name::SpaceOrTab,
                    min: 1,
                    max: usize::MAX,
                    content: tokenizer.tokenize_state.space_or_tab_eol_content,
                    connect: tokenizer.tokenize_state.space_or_tab_eol_connect,
                },
            ))
        }
        _ => State::Retry(StateName::SpaceOrTabEolAtEol),
    }
}

pub fn after_first(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.space_or_tab_eol_ok = true;
    debug_assert!(tokenizer.tokenize_state.space_or_tab_eol_content.is_none(), "expected no content");
    State::Retry(StateName::SpaceOrTabEolAtEol)
}

pub fn at_eol(tokenizer: &mut Tokenizer) -> State {
    if let Some(b'\n') = tokenizer.current {
        if let Some(ref content) = tokenizer.tokenize_state.space_or_tab_eol_content {
            tokenizer.enter_link(Name::LineEnding, Link::new(*content));
        } else {
            tokenizer.enter(Name::LineEnding);
        }

        if tokenizer.tokenize_state.space_or_tab_eol_connect {
            let index = tokenizer.events.len() - 1;
            link(&mut tokenizer.events, index);
        } else if tokenizer.tokenize_state.space_or_tab_eol_content.is_some() {
            tokenizer.tokenize_state.space_or_tab_eol_connect = true;
        }

        tokenizer.consume();
        tokenizer.exit(Name::LineEnding);
        State::Next(StateName::SpaceOrTabEolAfterEol)
    } else {
        let ok = tokenizer.tokenize_state.space_or_tab_eol_ok;
        tokenizer.tokenize_state.space_or_tab_eol_content = None;
        tokenizer.tokenize_state.space_or_tab_eol_connect = false;
        tokenizer.tokenize_state.space_or_tab_eol_ok = false;
        if ok { State::Ok } else { State::Nok }
    }
}

pub fn after_eol(tokenizer: &mut Tokenizer) -> State {
    if matches!(tokenizer.current, Some(b'\t' | b' ')) {
        tokenizer.attempt(State::Next(StateName::SpaceOrTabEolAfterMore), State::Nok);
        State::Retry(space_or_tab_with_options(
            tokenizer,
            SpaceOrTabOptions {
                kind: Name::SpaceOrTab,
                min: 1,
                max: usize::MAX,
                content: tokenizer.tokenize_state.space_or_tab_eol_content,
                connect: tokenizer.tokenize_state.space_or_tab_eol_connect,
            },
        ))
    } else {
        State::Retry(StateName::SpaceOrTabEolAfterMore)
    }
}

pub fn after_more(tokenizer: &mut Tokenizer) -> State {
    debug_assert!(!matches!(tokenizer.current, None | Some(b'\n')), "did not expect blank line");
    tokenizer.tokenize_state.space_or_tab_eol_content = None;
    tokenizer.tokenize_state.space_or_tab_eol_connect = false;
    tokenizer.tokenize_state.space_or_tab_eol_ok = false;
    State::Ok
}
