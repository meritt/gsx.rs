use crate::syntax::construct::partial_space_or_tab::space_or_tab_min_max;
use crate::syntax::event::{Content, Link, Name};
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::tokenizer::Tokenizer;
use crate::syntax::util::{
    constant::{LINK_REFERENCE_SIZE_MAX, TAB_SIZE},
    normalize_identifier::normalize_identifier,
    skip,
    slice::{Position, Slice},
};

pub fn start(tokenizer: &mut Tokenizer) -> State {
    tokenizer.enter(Name::GfmFootnoteDefinition);
    if matches!(tokenizer.current, Some(b'\t' | b' ')) {
        tokenizer.attempt(State::Next(StateName::GfmFootnoteDefinitionLabelBefore), State::Nok);
        State::Retry(space_or_tab_min_max(tokenizer, 1, TAB_SIZE - 1))
    } else {
        State::Retry(StateName::GfmFootnoteDefinitionLabelBefore)
    }
}

pub fn label_before(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'[') => {
            tokenizer.enter(Name::GfmFootnoteDefinitionPrefix);
            tokenizer.enter(Name::GfmFootnoteDefinitionLabel);
            tokenizer.enter(Name::GfmFootnoteDefinitionLabelMarker);
            tokenizer.consume();
            tokenizer.exit(Name::GfmFootnoteDefinitionLabelMarker);
            State::Next(StateName::GfmFootnoteDefinitionLabelAtMarker)
        }
        _ => State::Nok,
    }
}

pub fn label_at_marker(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b'^') {
        tokenizer.enter(Name::GfmFootnoteDefinitionMarker);
        tokenizer.consume();
        tokenizer.exit(Name::GfmFootnoteDefinitionMarker);
        tokenizer.enter(Name::GfmFootnoteDefinitionLabelString);
        tokenizer.enter_link(Name::Data, Link::new(Content::String));
        State::Next(StateName::GfmFootnoteDefinitionLabelInside)
    } else {
        State::Nok
    }
}

pub fn label_inside(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.tokenize_state.size > LINK_REFERENCE_SIZE_MAX
        || matches!(tokenizer.current, None | Some(b'\t' | b'\n' | b' ' | b'['))
        || (matches!(tokenizer.current, Some(b']')) && tokenizer.tokenize_state.size == 0)
    {
        tokenizer.tokenize_state.size = 0;
        State::Nok
    } else if matches!(tokenizer.current, Some(b']')) {
        tokenizer.tokenize_state.size = 0;
        tokenizer.exit(Name::Data);
        tokenizer.exit(Name::GfmFootnoteDefinitionLabelString);
        tokenizer.enter(Name::GfmFootnoteDefinitionLabelMarker);
        tokenizer.consume();
        tokenizer.exit(Name::GfmFootnoteDefinitionLabelMarker);
        tokenizer.exit(Name::GfmFootnoteDefinitionLabel);
        State::Next(StateName::GfmFootnoteDefinitionLabelAfter)
    } else {
        let next = if matches!(tokenizer.current.unwrap(), b'\\') {
            StateName::GfmFootnoteDefinitionLabelEscape
        } else {
            StateName::GfmFootnoteDefinitionLabelInside
        };
        tokenizer.consume();
        tokenizer.tokenize_state.size += 1;
        State::Next(next)
    }
}

pub fn label_escape(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'[' | b'\\' | b']') => {
            tokenizer.tokenize_state.size += 1;
            tokenizer.consume();
            State::Next(StateName::GfmFootnoteDefinitionLabelInside)
        }
        _ => State::Retry(StateName::GfmFootnoteDefinitionLabelInside),
    }
}

pub fn label_after(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b':') => {
            let end =
                skip::to_back(&tokenizer.events, tokenizer.events.len() - 1, &[Name::GfmFootnoteDefinitionLabelString]);

            let id = normalize_identifier(
                Slice::from_position(tokenizer.parse_state.bytes, &Position::from_exit_event(&tokenizer.events, end))
                    .as_str(),
            );

            tokenizer.tokenize_state.gfm_footnote_definitions.push(id);

            tokenizer.enter(Name::DefinitionMarker);
            tokenizer.consume();
            tokenizer.exit(Name::DefinitionMarker);
            tokenizer.attempt(State::Next(StateName::GfmFootnoteDefinitionWhitespaceAfter), State::Nok);
            State::Next(space_or_tab_min_max(tokenizer, 0, usize::MAX))
        }
        _ => State::Nok,
    }
}

pub fn whitespace_after(tokenizer: &mut Tokenizer) -> State {
    tokenizer.exit(Name::GfmFootnoteDefinitionPrefix);
    State::Ok
}

pub fn cont_start(tokenizer: &mut Tokenizer) -> State {
    tokenizer.check(
        State::Next(StateName::GfmFootnoteDefinitionContBlank),
        State::Next(StateName::GfmFootnoteDefinitionContFilled),
    );
    State::Retry(StateName::BlankLineStart)
}

pub fn cont_blank(tokenizer: &mut Tokenizer) -> State {
    if matches!(tokenizer.current, Some(b'\t' | b' ')) {
        State::Retry(space_or_tab_min_max(tokenizer, 0, TAB_SIZE))
    } else {
        State::Ok
    }
}

pub fn cont_filled(tokenizer: &mut Tokenizer) -> State {
    if matches!(tokenizer.current, Some(b'\t' | b' ')) {
        State::Retry(space_or_tab_min_max(tokenizer, TAB_SIZE, TAB_SIZE))
    } else {
        State::Nok
    }
}
