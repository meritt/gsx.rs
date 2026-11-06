use crate::syntax::construct::partial_space_or_tab::space_or_tab;
use crate::syntax::construct::partial_space_or_tab_eol::space_or_tab_eol;
use crate::syntax::event::Name;
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::tokenizer::Tokenizer;
use crate::syntax::util::{
    normalize_identifier::normalize_identifier,
    skip,
    slice::{Position, Slice},
};

pub fn start(tokenizer: &mut Tokenizer) -> State {
    if !tokenizer.interrupt
        || (!tokenizer.events.is_empty()
            && tokenizer.events
                [skip::opt_back(&tokenizer.events, tokenizer.events.len() - 1, &[Name::LineEnding, Name::SpaceOrTab])]
            .name
                == Name::Definition)
    {
        tokenizer.enter(Name::Definition);

        if matches!(tokenizer.current, Some(b'\t' | b' ')) {
            tokenizer.attempt(State::Next(StateName::DefinitionBefore), State::Nok);
            State::Retry(space_or_tab(tokenizer))
        } else {
            State::Retry(StateName::DefinitionBefore)
        }
    } else {
        State::Nok
    }
}

pub fn before(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'[') => {
            tokenizer.tokenize_state.token_1 = Name::DefinitionLabel;
            tokenizer.tokenize_state.token_2 = Name::DefinitionLabelMarker;
            tokenizer.tokenize_state.token_3 = Name::DefinitionLabelString;
            tokenizer.attempt(State::Next(StateName::DefinitionLabelAfter), State::Next(StateName::DefinitionLabelNok));
            State::Retry(StateName::LabelStart)
        }
        _ => State::Nok,
    }
}

pub fn label_after(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.token_1 = Name::Data;
    tokenizer.tokenize_state.token_2 = Name::Data;
    tokenizer.tokenize_state.token_3 = Name::Data;

    match tokenizer.current {
        Some(b':') => {
            tokenizer.tokenize_state.end =
                skip::to_back(&tokenizer.events, tokenizer.events.len() - 1, &[Name::DefinitionLabelString]);

            tokenizer.enter(Name::DefinitionMarker);
            tokenizer.consume();
            tokenizer.exit(Name::DefinitionMarker);
            State::Next(StateName::DefinitionMarkerAfter)
        }
        _ => State::Nok,
    }
}

pub fn label_nok(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.token_1 = Name::Data;
    tokenizer.tokenize_state.token_2 = Name::Data;
    tokenizer.tokenize_state.token_3 = Name::Data;
    State::Nok
}

pub fn marker_after(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(
        State::Next(StateName::DefinitionDestinationBefore),
        State::Next(StateName::DefinitionDestinationBefore),
    );
    State::Retry(space_or_tab_eol(tokenizer))
}

pub fn destination_before(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.token_1 = Name::DefinitionDestination;
    tokenizer.tokenize_state.token_2 = Name::DefinitionDestinationLiteral;
    tokenizer.tokenize_state.token_3 = Name::DefinitionDestinationLiteralMarker;
    tokenizer.tokenize_state.token_4 = Name::DefinitionDestinationRaw;
    tokenizer.tokenize_state.token_5 = Name::DefinitionDestinationString;
    tokenizer.tokenize_state.size_b = usize::MAX;
    tokenizer.attempt(
        State::Next(StateName::DefinitionDestinationAfter),
        State::Next(StateName::DefinitionDestinationMissing),
    );
    State::Retry(StateName::DestinationStart)
}

pub fn destination_after(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.token_1 = Name::Data;
    tokenizer.tokenize_state.token_2 = Name::Data;
    tokenizer.tokenize_state.token_3 = Name::Data;
    tokenizer.tokenize_state.token_4 = Name::Data;
    tokenizer.tokenize_state.token_5 = Name::Data;
    tokenizer.tokenize_state.size_b = 0;
    tokenizer.attempt(State::Next(StateName::DefinitionAfter), State::Next(StateName::DefinitionAfter));
    State::Retry(StateName::DefinitionTitleBefore)
}

pub fn destination_missing(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.token_1 = Name::Data;
    tokenizer.tokenize_state.token_2 = Name::Data;
    tokenizer.tokenize_state.token_3 = Name::Data;
    tokenizer.tokenize_state.token_4 = Name::Data;
    tokenizer.tokenize_state.token_5 = Name::Data;
    tokenizer.tokenize_state.size_b = 0;
    tokenizer.tokenize_state.end = 0;
    State::Nok
}

pub fn after(tokenizer: &mut Tokenizer) -> State {
    if matches!(tokenizer.current, Some(b'\t' | b' ')) {
        tokenizer.attempt(State::Next(StateName::DefinitionAfterWhitespace), State::Nok);
        State::Retry(space_or_tab(tokenizer))
    } else {
        State::Retry(StateName::DefinitionAfterWhitespace)
    }
}

pub fn after_whitespace(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n') => {
            tokenizer.exit(Name::Definition);

            tokenizer.tokenize_state.definitions.push(normalize_identifier(
                Slice::from_position(
                    tokenizer.parse_state.bytes,
                    &Position::from_exit_event(&tokenizer.events, tokenizer.tokenize_state.end),
                )
                .as_str(),
            ));

            tokenizer.tokenize_state.end = 0;

            tokenizer.interrupt = true;
            State::Ok
        }
        _ => {
            tokenizer.tokenize_state.end = 0;
            State::Nok
        }
    }
}

pub fn title_before(tokenizer: &mut Tokenizer) -> State {
    if matches!(tokenizer.current, Some(b'\t' | b'\n' | b' ')) {
        tokenizer.attempt(State::Next(StateName::DefinitionTitleBeforeMarker), State::Nok);
        State::Retry(space_or_tab_eol(tokenizer))
    } else {
        State::Nok
    }
}

pub fn title_before_marker(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.token_1 = Name::DefinitionTitle;
    tokenizer.tokenize_state.token_2 = Name::DefinitionTitleMarker;
    tokenizer.tokenize_state.token_3 = Name::DefinitionTitleString;
    tokenizer.attempt(State::Next(StateName::DefinitionTitleAfter), State::Nok);
    State::Retry(StateName::TitleStart)
}

pub fn title_after(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.token_1 = Name::Data;
    tokenizer.tokenize_state.token_2 = Name::Data;
    tokenizer.tokenize_state.token_3 = Name::Data;
    if matches!(tokenizer.current, Some(b'\t' | b' ')) {
        tokenizer.attempt(State::Next(StateName::DefinitionTitleAfterOptionalWhitespace), State::Nok);
        State::Retry(space_or_tab(tokenizer))
    } else {
        State::Retry(StateName::DefinitionTitleAfterOptionalWhitespace)
    }
}

pub fn title_after_optional_whitespace(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n') => State::Ok,
        _ => State::Nok,
    }
}
