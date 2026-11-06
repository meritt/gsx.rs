use crate::syntax::construct::partial_space_or_tab::{Options as SpaceOrTabOptions, space_or_tab_with_options};
use crate::syntax::event::Name;
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::tokenizer::Tokenizer;
use crate::syntax::util::{
    constant::{HTML_BLOCK_NAMES, HTML_CDATA_PREFIX, HTML_RAW_NAMES, HTML_RAW_SIZE_MAX, TAB_SIZE},
    slice::Slice,
};

const RAW: u8 = 1;
const COMMENT: u8 = 2;
const INSTRUCTION: u8 = 3;
const DECLARATION: u8 = 4;
const CDATA: u8 = 5;
const BASIC: u8 = 6;
const COMPLETE: u8 = 7;

pub fn start(tokenizer: &mut Tokenizer) -> State {
    tokenizer.enter(Name::HtmlFlow);

    if matches!(tokenizer.current, Some(b'\t' | b' ')) {
        tokenizer.attempt(State::Next(StateName::HtmlFlowBefore), State::Nok);
        State::Retry(space_or_tab_with_options(
            tokenizer,
            SpaceOrTabOptions { kind: Name::HtmlFlowData, min: 0, max: TAB_SIZE - 1, connect: false, content: None },
        ))
    } else {
        State::Retry(StateName::HtmlFlowBefore)
    }
}

pub fn before(tokenizer: &mut Tokenizer) -> State {
    if Some(b'<') == tokenizer.current {
        tokenizer.enter(Name::HtmlFlowData);
        tokenizer.consume();
        State::Next(StateName::HtmlFlowOpen)
    } else {
        State::Nok
    }
}

pub fn open(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'!') => {
            tokenizer.consume();
            State::Next(StateName::HtmlFlowDeclarationOpen)
        }
        Some(b'/') => {
            tokenizer.consume();
            tokenizer.tokenize_state.seen = true;
            tokenizer.tokenize_state.start = tokenizer.point.index;
            State::Next(StateName::HtmlFlowTagCloseStart)
        }
        Some(b'?') => {
            tokenizer.consume();
            tokenizer.tokenize_state.marker = INSTRUCTION;
            tokenizer.concrete = true;
            State::Next(StateName::HtmlFlowContinuationDeclarationInside)
        }
        Some(b'A'..=b'Z' | b'a'..=b'z') => {
            tokenizer.tokenize_state.start = tokenizer.point.index;
            State::Retry(StateName::HtmlFlowTagName)
        }
        _ => State::Nok,
    }
}

pub fn declaration_open(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'-') => {
            tokenizer.consume();
            tokenizer.tokenize_state.marker = COMMENT;
            State::Next(StateName::HtmlFlowCommentOpenInside)
        }
        Some(b'A'..=b'Z' | b'a'..=b'z') => {
            tokenizer.consume();
            tokenizer.tokenize_state.marker = DECLARATION;
            tokenizer.concrete = true;
            State::Next(StateName::HtmlFlowContinuationDeclarationInside)
        }
        Some(b'[') => {
            tokenizer.consume();
            tokenizer.tokenize_state.marker = CDATA;
            State::Next(StateName::HtmlFlowCdataOpenInside)
        }
        _ => State::Nok,
    }
}

pub fn comment_open_inside(tokenizer: &mut Tokenizer) -> State {
    if let Some(b'-') = tokenizer.current {
        tokenizer.consume();
        tokenizer.concrete = true;
        State::Next(StateName::HtmlFlowContinuationDeclarationInside)
    } else {
        tokenizer.tokenize_state.marker = 0;
        State::Nok
    }
}

pub fn cdata_open_inside(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(HTML_CDATA_PREFIX[tokenizer.tokenize_state.size]) {
        tokenizer.consume();
        tokenizer.tokenize_state.size += 1;

        if tokenizer.tokenize_state.size == HTML_CDATA_PREFIX.len() {
            tokenizer.tokenize_state.size = 0;
            tokenizer.concrete = true;
            State::Next(StateName::HtmlFlowContinuation)
        } else {
            State::Next(StateName::HtmlFlowCdataOpenInside)
        }
    } else {
        tokenizer.tokenize_state.marker = 0;
        tokenizer.tokenize_state.size = 0;
        State::Nok
    }
}

pub fn tag_close_start(tokenizer: &mut Tokenizer) -> State {
    if let Some(b'A'..=b'Z' | b'a'..=b'z') = tokenizer.current {
        tokenizer.consume();
        State::Next(StateName::HtmlFlowTagName)
    } else {
        tokenizer.tokenize_state.seen = false;
        tokenizer.tokenize_state.start = 0;
        State::Nok
    }
}

pub fn tag_name(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\t' | b'\n' | b' ' | b'/' | b'>') => {
            let closing_tag = tokenizer.tokenize_state.seen;
            let slash = matches!(tokenizer.current, Some(b'/'));
            let slice =
                Slice::from_indices(tokenizer.parse_state.bytes, tokenizer.tokenize_state.start, tokenizer.point.index);
            let name = slice.as_str().trim().to_ascii_lowercase();
            tokenizer.tokenize_state.seen = false;
            tokenizer.tokenize_state.start = 0;

            if !slash && !closing_tag && HTML_RAW_NAMES.contains(&name.as_str()) {
                tokenizer.tokenize_state.marker = RAW;
                tokenizer.concrete = true;
                State::Retry(StateName::HtmlFlowContinuation)
            } else if HTML_BLOCK_NAMES.contains(&name.as_str()) {
                tokenizer.tokenize_state.marker = BASIC;

                if slash {
                    tokenizer.consume();
                    State::Next(StateName::HtmlFlowBasicSelfClosing)
                } else {
                    tokenizer.concrete = true;
                    State::Retry(StateName::HtmlFlowContinuation)
                }
            } else {
                tokenizer.tokenize_state.marker = COMPLETE;

                if tokenizer.interrupt {
                    tokenizer.tokenize_state.marker = 0;
                    State::Nok
                } else if closing_tag {
                    State::Retry(StateName::HtmlFlowCompleteClosingTagAfter)
                } else {
                    State::Retry(StateName::HtmlFlowCompleteAttributeNameBefore)
                }
            }
        }
        Some(b'-' | b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z') => {
            tokenizer.consume();
            State::Next(StateName::HtmlFlowTagName)
        }
        Some(_) => {
            tokenizer.tokenize_state.seen = false;
            State::Nok
        }
    }
}

pub fn basic_self_closing(tokenizer: &mut Tokenizer) -> State {
    if let Some(b'>') = tokenizer.current {
        tokenizer.consume();
        tokenizer.concrete = true;
        State::Next(StateName::HtmlFlowContinuation)
    } else {
        tokenizer.tokenize_state.marker = 0;
        State::Nok
    }
}

pub fn complete_closing_tag_after(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'\t' | b' ') => {
            tokenizer.consume();
            State::Next(StateName::HtmlFlowCompleteClosingTagAfter)
        }
        _ => State::Retry(StateName::HtmlFlowCompleteEnd),
    }
}

pub fn complete_attribute_name_before(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'\t' | b' ') => {
            tokenizer.consume();
            State::Next(StateName::HtmlFlowCompleteAttributeNameBefore)
        }
        Some(b'/') => {
            tokenizer.consume();
            State::Next(StateName::HtmlFlowCompleteEnd)
        }
        Some(b'0'..=b'9' | b':' | b'A'..=b'Z' | b'_' | b'a'..=b'z') => {
            tokenizer.consume();
            State::Next(StateName::HtmlFlowCompleteAttributeName)
        }
        _ => State::Retry(StateName::HtmlFlowCompleteEnd),
    }
}

pub fn complete_attribute_name(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'-' | b'.' | b'0'..=b'9' | b':' | b'A'..=b'Z' | b'_' | b'a'..=b'z') => {
            tokenizer.consume();
            State::Next(StateName::HtmlFlowCompleteAttributeName)
        }
        _ => State::Retry(StateName::HtmlFlowCompleteAttributeNameAfter),
    }
}

pub fn complete_attribute_name_after(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'\t' | b' ') => {
            tokenizer.consume();
            State::Next(StateName::HtmlFlowCompleteAttributeNameAfter)
        }
        Some(b'=') => {
            tokenizer.consume();
            State::Next(StateName::HtmlFlowCompleteAttributeValueBefore)
        }
        _ => State::Retry(StateName::HtmlFlowCompleteAttributeNameBefore),
    }
}

pub fn complete_attribute_value_before(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'<' | b'=' | b'>' | b'`') => {
            tokenizer.tokenize_state.marker = 0;
            State::Nok
        }
        Some(b'\t' | b' ') => {
            tokenizer.consume();
            State::Next(StateName::HtmlFlowCompleteAttributeValueBefore)
        }
        Some(b'"' | b'\'') => {
            tokenizer.tokenize_state.marker_b = tokenizer.current.unwrap();
            tokenizer.consume();
            State::Next(StateName::HtmlFlowCompleteAttributeValueQuoted)
        }
        _ => State::Retry(StateName::HtmlFlowCompleteAttributeValueUnquoted),
    }
}

pub fn complete_attribute_value_quoted(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(tokenizer.tokenize_state.marker_b) {
        tokenizer.consume();
        tokenizer.tokenize_state.marker_b = 0;
        State::Next(StateName::HtmlFlowCompleteAttributeValueQuotedAfter)
    } else if matches!(tokenizer.current, None | Some(b'\n')) {
        tokenizer.tokenize_state.marker = 0;
        tokenizer.tokenize_state.marker_b = 0;
        State::Nok
    } else {
        tokenizer.consume();
        State::Next(StateName::HtmlFlowCompleteAttributeValueQuoted)
    }
}

pub fn complete_attribute_value_unquoted(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\t' | b'\n' | b' ' | b'"' | b'\'' | b'/' | b'<' | b'=' | b'>' | b'`') => {
            State::Retry(StateName::HtmlFlowCompleteAttributeNameAfter)
        }
        Some(_) => {
            tokenizer.consume();
            State::Next(StateName::HtmlFlowCompleteAttributeValueUnquoted)
        }
    }
}

pub fn complete_attribute_value_quoted_after(tokenizer: &mut Tokenizer) -> State {
    if let Some(b'\t' | b' ' | b'/' | b'>') = tokenizer.current {
        State::Retry(StateName::HtmlFlowCompleteAttributeNameBefore)
    } else {
        tokenizer.tokenize_state.marker = 0;
        State::Nok
    }
}

pub fn complete_end(tokenizer: &mut Tokenizer) -> State {
    if let Some(b'>') = tokenizer.current {
        tokenizer.consume();
        State::Next(StateName::HtmlFlowCompleteAfter)
    } else {
        tokenizer.tokenize_state.marker = 0;
        State::Nok
    }
}

pub fn complete_after(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n') => {
            tokenizer.concrete = true;
            State::Retry(StateName::HtmlFlowContinuation)
        }
        Some(b'\t' | b' ') => {
            tokenizer.consume();
            State::Next(StateName::HtmlFlowCompleteAfter)
        }
        Some(_) => {
            tokenizer.tokenize_state.marker = 0;
            State::Nok
        }
    }
}

pub fn continuation(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.tokenize_state.marker == COMMENT && tokenizer.current == Some(b'-') {
        tokenizer.consume();
        State::Next(StateName::HtmlFlowContinuationCommentInside)
    } else if tokenizer.tokenize_state.marker == RAW && tokenizer.current == Some(b'<') {
        tokenizer.consume();
        State::Next(StateName::HtmlFlowContinuationRawTagOpen)
    } else if tokenizer.tokenize_state.marker == DECLARATION && tokenizer.current == Some(b'>') {
        tokenizer.consume();
        State::Next(StateName::HtmlFlowContinuationClose)
    } else if tokenizer.tokenize_state.marker == INSTRUCTION && tokenizer.current == Some(b'?') {
        tokenizer.consume();
        State::Next(StateName::HtmlFlowContinuationDeclarationInside)
    } else if tokenizer.tokenize_state.marker == CDATA && tokenizer.current == Some(b']') {
        tokenizer.consume();
        State::Next(StateName::HtmlFlowContinuationCdataInside)
    } else if matches!(tokenizer.tokenize_state.marker, BASIC | COMPLETE) && tokenizer.current == Some(b'\n') {
        tokenizer.exit(Name::HtmlFlowData);
        tokenizer.check(
            State::Next(StateName::HtmlFlowContinuationAfter),
            State::Next(StateName::HtmlFlowContinuationStart),
        );
        State::Retry(StateName::HtmlFlowBlankLineBefore)
    } else if matches!(tokenizer.current, None | Some(b'\n')) {
        tokenizer.exit(Name::HtmlFlowData);
        State::Retry(StateName::HtmlFlowContinuationStart)
    } else {
        tokenizer.consume();
        State::Next(StateName::HtmlFlowContinuation)
    }
}

pub fn continuation_start(tokenizer: &mut Tokenizer) -> State {
    tokenizer.check(
        State::Next(StateName::HtmlFlowContinuationStartNonLazy),
        State::Next(StateName::HtmlFlowContinuationAfter),
    );
    State::Retry(StateName::NonLazyContinuationStart)
}

pub fn continuation_start_non_lazy(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'\n') => {
            tokenizer.enter(Name::LineEnding);
            tokenizer.consume();
            tokenizer.exit(Name::LineEnding);
            State::Next(StateName::HtmlFlowContinuationBefore)
        }
        _ => unreachable!("expected eol"),
    }
}

pub fn continuation_before(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n') => State::Retry(StateName::HtmlFlowContinuationStart),
        _ => {
            tokenizer.enter(Name::HtmlFlowData);
            State::Retry(StateName::HtmlFlowContinuation)
        }
    }
}

pub fn continuation_comment_inside(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'-') => {
            tokenizer.consume();
            State::Next(StateName::HtmlFlowContinuationDeclarationInside)
        }
        _ => State::Retry(StateName::HtmlFlowContinuation),
    }
}

pub fn continuation_raw_tag_open(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'/') => {
            tokenizer.consume();
            tokenizer.tokenize_state.start = tokenizer.point.index;
            State::Next(StateName::HtmlFlowContinuationRawEndTag)
        }
        _ => State::Retry(StateName::HtmlFlowContinuation),
    }
}

pub fn continuation_raw_end_tag(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'>') => {
            let slice =
                Slice::from_indices(tokenizer.parse_state.bytes, tokenizer.tokenize_state.start, tokenizer.point.index);
            let name = slice.as_str().to_ascii_lowercase();

            tokenizer.tokenize_state.start = 0;

            if HTML_RAW_NAMES.contains(&name.as_str()) {
                tokenizer.consume();
                State::Next(StateName::HtmlFlowContinuationClose)
            } else {
                State::Retry(StateName::HtmlFlowContinuation)
            }
        }
        Some(b'A'..=b'Z' | b'a'..=b'z')
            if tokenizer.point.index - tokenizer.tokenize_state.start < HTML_RAW_SIZE_MAX =>
        {
            tokenizer.consume();
            State::Next(StateName::HtmlFlowContinuationRawEndTag)
        }
        _ => {
            tokenizer.tokenize_state.start = 0;
            State::Retry(StateName::HtmlFlowContinuation)
        }
    }
}

pub fn continuation_cdata_inside(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b']') => {
            tokenizer.consume();
            State::Next(StateName::HtmlFlowContinuationDeclarationInside)
        }
        _ => State::Retry(StateName::HtmlFlowContinuation),
    }
}

pub fn continuation_declaration_inside(tokenizer: &mut Tokenizer) -> State {
    if (tokenizer.tokenize_state.marker == COMMENT && tokenizer.current == Some(b'-'))
        || (tokenizer.tokenize_state.marker == CDATA && tokenizer.current == Some(b']'))
    {
        tokenizer.consume();
        State::Next(StateName::HtmlFlowContinuationDeclarationInside)
    } else if tokenizer.current == Some(b'>') {
        tokenizer.consume();
        State::Next(StateName::HtmlFlowContinuationClose)
    } else {
        State::Retry(StateName::HtmlFlowContinuation)
    }
}

pub fn continuation_close(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n') => {
            tokenizer.exit(Name::HtmlFlowData);
            State::Retry(StateName::HtmlFlowContinuationAfter)
        }
        _ => {
            tokenizer.consume();
            State::Next(StateName::HtmlFlowContinuationClose)
        }
    }
}

pub fn continuation_after(tokenizer: &mut Tokenizer) -> State {
    tokenizer.exit(Name::HtmlFlow);
    tokenizer.tokenize_state.marker = 0;
    tokenizer.interrupt = false;
    tokenizer.concrete = false;
    State::Ok
}

pub fn blank_line_before(tokenizer: &mut Tokenizer) -> State {
    tokenizer.enter(Name::LineEnding);
    tokenizer.consume();
    tokenizer.exit(Name::LineEnding);
    State::Next(StateName::BlankLineStart)
}
