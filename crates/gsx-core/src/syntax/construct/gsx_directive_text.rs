use crate::syntax::event::{Content, Link, Name};
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::subtokenize::link_to;
use crate::syntax::tokenizer::Tokenizer;

const LABEL_BALANCE_MAX: usize = 32;
const LABEL_SIZE_MAX: usize = 999;

fn is_name_start(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte >= 0x80
}

fn is_name_continue(byte: u8) -> bool {
    is_name_start(byte) || matches!(byte, b'-' | b'_')
}

fn is_space_or_eol(byte: u8) -> bool {
    matches!(byte, b'\t' | b'\n' | b' ')
}

pub fn start(tokenizer: &mut Tokenizer) -> State {
    let escaped_colon = tokenizer.events.last().is_some_and(|event| event.name == Name::CharacterEscape);
    if tokenizer.current != Some(b':') || (tokenizer.previous == Some(b':') && !escaped_colon) {
        return State::Nok;
    }
    tokenizer.enter(Name::GsxDirectiveText);
    tokenizer.enter(Name::GsxDirectiveTextMarker);
    tokenizer.consume();
    tokenizer.exit(Name::GsxDirectiveTextMarker);
    State::Next(StateName::GsxDirectiveTextNameStart)
}

pub fn name_start(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(byte) if is_name_start(byte) => {
            tokenizer.enter(Name::GsxDirectiveTextIdentifier);
            tokenizer.consume();
            State::Next(StateName::GsxDirectiveTextNameInside)
        }
        _ => State::Nok,
    }
}

pub fn name_inside(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(byte) if is_name_continue(byte) => {
            tokenizer.consume();
            State::Next(StateName::GsxDirectiveTextNameInside)
        }
        _ => {
            tokenizer.exit(Name::GsxDirectiveTextIdentifier);
            if matches!(tokenizer.previous, Some(b'-' | b'_')) {
                State::Nok
            } else {
                State::Retry(StateName::GsxDirectiveTextAfterName)
            }
        }
    }
}

pub fn after_name(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b':') => State::Nok,
        Some(b'[') => {
            tokenizer.attempt(
                State::Next(StateName::GsxDirectiveTextAfterLabel),
                State::Next(StateName::GsxDirectiveTextAfterLabel),
            );
            State::Retry(StateName::GsxDirectiveTextLabelStart)
        }
        _ => State::Retry(StateName::GsxDirectiveTextAfterLabel),
    }
}

pub fn label_start(tokenizer: &mut Tokenizer) -> State {
    tokenizer.enter(Name::GsxDirectiveTextLabel);
    tokenizer.enter(Name::GsxDirectiveTextLabelMarker);
    tokenizer.consume();
    tokenizer.exit(Name::GsxDirectiveTextLabelMarker);
    tokenizer.tokenize_state.size = 0;
    tokenizer.tokenize_state.size_b = 0;
    State::Next(StateName::GsxDirectiveTextLabelAfterStart)
}

pub fn label_after_start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b']') {
        return State::Retry(StateName::GsxDirectiveTextLabelClose);
    }
    tokenizer.enter(Name::GsxDirectiveTextLabelString);
    State::Retry(StateName::GsxDirectiveTextLabelLineStart)
}

pub fn label_data(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None => State::Retry(StateName::GsxDirectiveTextLabelNok),
        _ if tokenizer.tokenize_state.size > LABEL_SIZE_MAX => State::Retry(StateName::GsxDirectiveTextLabelNok),
        Some(b'[') => {
            tokenizer.tokenize_state.size_b += 1;
            if tokenizer.tokenize_state.size_b > LABEL_BALANCE_MAX {
                return State::Retry(StateName::GsxDirectiveTextLabelNok);
            }
            tokenizer.consume();
            State::Next(StateName::GsxDirectiveTextLabelData)
        }
        Some(b']') if tokenizer.tokenize_state.size_b == 0 => {
            tokenizer.exit(Name::Data);
            State::Retry(StateName::GsxDirectiveTextLabelClose)
        }
        Some(b'\n') => {
            tokenizer.consume();
            tokenizer.exit(Name::Data);
            State::Next(StateName::GsxDirectiveTextLabelLineStart)
        }
        Some(byte) => {
            if byte == b']' {
                tokenizer.tokenize_state.size_b -= 1;
            }
            tokenizer.consume();
            State::Next(if byte == b'\\' {
                StateName::GsxDirectiveTextLabelEscape
            } else {
                StateName::GsxDirectiveTextLabelData
            })
        }
    }
}

pub fn label_line_start(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None => State::Retry(StateName::GsxDirectiveTextLabelNok),
        Some(b']') if tokenizer.tokenize_state.size_b == 0 => State::Retry(StateName::GsxDirectiveTextLabelClose),
        _ => {
            let index = tokenizer.events.len();
            let previous = (tokenizer.events[index - 1].name == Name::Data).then(|| index - 2);
            tokenizer.enter_link(Name::Data, Link::new(Content::Text));
            if let Some(previous) = previous {
                let next = tokenizer.events.len() - 1;
                link_to(&mut tokenizer.events, previous, next);
            }
            State::Retry(StateName::GsxDirectiveTextLabelData)
        }
    }
}

pub fn label_escape(tokenizer: &mut Tokenizer) -> State {
    if matches!(tokenizer.current, Some(b'[' | b'\\' | b']')) {
        tokenizer.consume();
        tokenizer.tokenize_state.size += 1;
        State::Next(StateName::GsxDirectiveTextLabelData)
    } else {
        State::Retry(StateName::GsxDirectiveTextLabelData)
    }
}

pub fn label_close(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.events.last().map(|event| &event.name) != Some(&Name::GsxDirectiveTextLabelMarker) {
        tokenizer.exit(Name::GsxDirectiveTextLabelString);
    }
    tokenizer.enter(Name::GsxDirectiveTextLabelMarker);
    tokenizer.consume();
    tokenizer.exit(Name::GsxDirectiveTextLabelMarker);
    tokenizer.exit(Name::GsxDirectiveTextLabel);
    tokenizer.tokenize_state.size = 0;
    tokenizer.tokenize_state.size_b = 0;
    State::Ok
}

pub fn label_nok(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.size = 0;
    tokenizer.tokenize_state.size_b = 0;
    State::Nok
}

pub fn after_label(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b'{') {
        tokenizer.attempt(
            State::Next(StateName::GsxDirectiveTextAfterAttributes),
            State::Next(StateName::GsxDirectiveTextAfterAttributes),
        );
        State::Retry(StateName::GsxDirectiveTextAttributesStart)
    } else {
        State::Retry(StateName::GsxDirectiveTextAfterAttributes)
    }
}

pub fn after_attributes(tokenizer: &mut Tokenizer) -> State {
    tokenizer.exit(Name::GsxDirectiveText);
    State::Ok
}

pub fn attributes_start(tokenizer: &mut Tokenizer) -> State {
    tokenizer.enter(Name::GsxDirectiveTextAttributes);
    tokenizer.enter(Name::GsxDirectiveTextAttributesMarker);
    tokenizer.consume();
    tokenizer.exit(Name::GsxDirectiveTextAttributesMarker);
    State::Next(StateName::GsxDirectiveTextAttributesBetween)
}

pub fn attributes_between(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'#' | b'.') => {
            tokenizer.enter(Name::GsxDirectiveTextAttribute);
            tokenizer.enter(Name::GsxDirectiveTextAttributeShortcut);
            tokenizer.consume();
            State::Next(StateName::GsxDirectiveTextAttributeShortcutStart)
        }
        Some(byte) if is_space_or_eol(byte) => {
            tokenizer.consume();
            State::Next(StateName::GsxDirectiveTextAttributesBetween)
        }
        Some(byte) if is_name_continue(byte) => {
            tokenizer.enter(Name::GsxDirectiveTextAttribute);
            tokenizer.enter(Name::GsxDirectiveTextAttributeKey);
            tokenizer.consume();
            State::Next(StateName::GsxDirectiveTextAttributeKey)
        }
        _ => State::Retry(StateName::GsxDirectiveTextAttributesEnd),
    }
}

fn is_shortcut_stop(byte: u8) -> bool {
    matches!(byte, b'"' | b'\'' | b'<' | b'=' | b'>' | b'`')
}

pub fn attribute_shortcut_start(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None => State::Nok,
        Some(byte) if is_shortcut_stop(byte) || is_space_or_eol(byte) => State::Nok,
        Some(b'#' | b'.' | b'}') => State::Nok,
        Some(_) => {
            tokenizer.consume();
            State::Next(StateName::GsxDirectiveTextAttributeShortcut)
        }
    }
}

pub fn attribute_shortcut(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None => State::Nok,
        Some(byte) if is_shortcut_stop(byte) => State::Nok,
        Some(byte) if matches!(byte, b'#' | b'.' | b'}') || is_space_or_eol(byte) => {
            tokenizer.exit(Name::GsxDirectiveTextAttributeShortcut);
            tokenizer.exit(Name::GsxDirectiveTextAttribute);
            State::Retry(StateName::GsxDirectiveTextAttributesBetween)
        }
        Some(_) => {
            tokenizer.consume();
            State::Next(StateName::GsxDirectiveTextAttributeShortcut)
        }
    }
}

pub fn attribute_name(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(byte) if is_name_continue(byte) || matches!(byte, b'.' | b':') => {
            tokenizer.consume();
            State::Next(StateName::GsxDirectiveTextAttributeKey)
        }
        _ => {
            tokenizer.exit(Name::GsxDirectiveTextAttributeKey);
            State::Retry(StateName::GsxDirectiveTextAttributeNameAfter)
        }
    }
}

pub fn attribute_name_after(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(byte) if is_space_or_eol(byte) => {
            tokenizer.consume();
            State::Next(StateName::GsxDirectiveTextAttributeNameAfter)
        }
        Some(b'=') => {
            tokenizer.consume();
            State::Next(StateName::GsxDirectiveTextAttributeValueBefore)
        }
        _ => {
            tokenizer.exit(Name::GsxDirectiveTextAttribute);
            State::Retry(StateName::GsxDirectiveTextAttributesBetween)
        }
    }
}

pub fn attribute_value_before(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'<' | b'=' | b'>' | b'`' | b'}') => State::Nok,
        Some(byte @ (b'"' | b'\'')) => {
            tokenizer.tokenize_state.marker_b = byte;
            tokenizer.consume();
            State::Next(StateName::GsxDirectiveTextAttributeValueQuoted)
        }
        Some(byte) if is_space_or_eol(byte) => {
            tokenizer.consume();
            State::Next(StateName::GsxDirectiveTextAttributeValueBefore)
        }
        Some(_) => {
            tokenizer.tokenize_state.marker_b = 0;
            tokenizer.enter(Name::GsxDirectiveTextAttributeValue);
            tokenizer.consume();
            State::Next(StateName::GsxDirectiveTextAttributeValueUnquoted)
        }
    }
}

pub fn attribute_value_unquoted(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'"' | b'\'' | b'<' | b'=' | b'>' | b'`') => State::Nok,
        Some(byte) if byte == b'}' || is_space_or_eol(byte) => {
            tokenizer.exit(Name::GsxDirectiveTextAttributeValue);
            tokenizer.exit(Name::GsxDirectiveTextAttribute);
            State::Retry(StateName::GsxDirectiveTextAttributesBetween)
        }
        Some(_) => {
            tokenizer.consume();
            State::Next(StateName::GsxDirectiveTextAttributeValueUnquoted)
        }
    }
}

pub fn attribute_value_quoted(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None => State::Nok,
        Some(byte) if byte == tokenizer.tokenize_state.marker_b => {
            if tokenizer.stack.last() == Some(&Name::GsxDirectiveTextAttributeValue) {
                tokenizer.exit(Name::GsxDirectiveTextAttributeValue);
            }
            tokenizer.consume();
            tokenizer.exit(Name::GsxDirectiveTextAttribute);
            tokenizer.tokenize_state.marker_b = 0;
            State::Next(StateName::GsxDirectiveTextAttributeValueQuotedAfter)
        }
        Some(_) => {
            if tokenizer.stack.last() != Some(&Name::GsxDirectiveTextAttributeValue) {
                tokenizer.enter(Name::GsxDirectiveTextAttributeValue);
            }
            tokenizer.consume();
            State::Next(StateName::GsxDirectiveTextAttributeValueQuoted)
        }
    }
}

pub fn attribute_value_quoted_after(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(byte) if byte == b'}' || is_space_or_eol(byte) => {
            State::Retry(StateName::GsxDirectiveTextAttributesBetween)
        }
        _ => State::Retry(StateName::GsxDirectiveTextAttributesEnd),
    }
}

pub fn attributes_end(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b'}') {
        tokenizer.enter(Name::GsxDirectiveTextAttributesMarker);
        tokenizer.consume();
        tokenizer.exit(Name::GsxDirectiveTextAttributesMarker);
        tokenizer.exit(Name::GsxDirectiveTextAttributes);
        State::Ok
    } else {
        State::Nok
    }
}
