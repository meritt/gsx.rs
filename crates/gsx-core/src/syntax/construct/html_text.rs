use crate::syntax::construct::partial_space_or_tab::space_or_tab;
use crate::syntax::event::Name;
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::tokenizer::Tokenizer;
use crate::syntax::util::constant::HTML_CDATA_PREFIX;

const COMMENT: usize = 0;
const CDATA: usize = 1;
const DECLARATION: usize = 2;
const INSTRUCTION: usize = 3;

fn scan(tokenizer: &mut Tokenizer, kind: usize, next: StateName) -> State {
    if tokenizer.point.index as usize >= tokenizer.tokenize_state.html_text_unclosed[kind] {
        State::Nok
    } else {
        tokenizer.tokenize_state.scan_start = tokenizer.point.index as usize;
        State::Next(next)
    }
}

fn unclosed(tokenizer: &mut Tokenizer, kind: usize) -> State {
    let start = tokenizer.tokenize_state.scan_start;
    let unclosed = &mut tokenizer.tokenize_state.html_text_unclosed[kind];
    *unclosed = (*unclosed).min(start);
    State::Nok
}

pub fn start(tokenizer: &mut Tokenizer) -> State {
    if Some(b'<') == tokenizer.current {
        tokenizer.enter(Name::HtmlText);
        tokenizer.enter(Name::HtmlTextData);
        tokenizer.consume();
        State::Next(StateName::HtmlTextOpen)
    } else {
        State::Nok
    }
}

pub fn open(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'!') => {
            tokenizer.consume();
            State::Next(StateName::HtmlTextDeclarationOpen)
        }
        Some(b'/') => {
            tokenizer.consume();
            State::Next(StateName::HtmlTextTagCloseStart)
        }
        Some(b'?') => {
            tokenizer.consume();
            scan(tokenizer, INSTRUCTION, StateName::HtmlTextInstruction)
        }
        Some(b'A'..=b'Z' | b'a'..=b'z') => {
            tokenizer.consume();
            State::Next(StateName::HtmlTextTagOpen)
        }
        _ => State::Nok,
    }
}

pub fn declaration_open(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'-') => {
            tokenizer.consume();
            State::Next(StateName::HtmlTextCommentOpenInside)
        }
        Some(b'A'..=b'Z' | b'a'..=b'z') => {
            tokenizer.consume();
            scan(tokenizer, DECLARATION, StateName::HtmlTextDeclaration)
        }
        Some(b'[') => {
            tokenizer.consume();
            State::Next(StateName::HtmlTextCdataOpenInside)
        }
        _ => State::Nok,
    }
}

pub fn comment_open_inside(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'-') => {
            tokenizer.consume();
            scan(tokenizer, COMMENT, StateName::HtmlTextCommentEnd)
        }
        _ => State::Nok,
    }
}

pub fn comment(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None => unclosed(tokenizer, COMMENT),
        Some(b'\n') => {
            tokenizer.attempt(State::Next(StateName::HtmlTextComment), State::Nok);
            State::Retry(StateName::HtmlTextLineEndingBefore)
        }
        Some(b'-') => {
            tokenizer.consume();
            State::Next(StateName::HtmlTextCommentClose)
        }
        _ => {
            tokenizer.consume();
            State::Next(StateName::HtmlTextComment)
        }
    }
}

pub fn comment_close(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'-') => {
            tokenizer.consume();
            State::Next(StateName::HtmlTextCommentEnd)
        }
        _ => State::Retry(StateName::HtmlTextComment),
    }
}
pub fn comment_end(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'>') => State::Retry(StateName::HtmlTextEnd),
        Some(b'-') => State::Retry(StateName::HtmlTextCommentClose),
        _ => State::Retry(StateName::HtmlTextComment),
    }
}

pub fn cdata_open_inside(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(HTML_CDATA_PREFIX[tokenizer.tokenize_state.size]) {
        tokenizer.tokenize_state.size += 1;
        tokenizer.consume();

        if tokenizer.tokenize_state.size == HTML_CDATA_PREFIX.len() {
            tokenizer.tokenize_state.size = 0;
            scan(tokenizer, CDATA, StateName::HtmlTextCdata)
        } else {
            State::Next(StateName::HtmlTextCdataOpenInside)
        }
    } else {
        tokenizer.tokenize_state.size = 0;
        State::Nok
    }
}

pub fn cdata(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None => unclosed(tokenizer, CDATA),
        Some(b'\n') => {
            tokenizer.attempt(State::Next(StateName::HtmlTextCdata), State::Nok);
            State::Retry(StateName::HtmlTextLineEndingBefore)
        }
        Some(b']') => {
            tokenizer.consume();
            State::Next(StateName::HtmlTextCdataClose)
        }
        _ => {
            tokenizer.consume();
            State::Next(StateName::HtmlTextCdata)
        }
    }
}

pub fn cdata_close(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b']') => {
            tokenizer.consume();
            State::Next(StateName::HtmlTextCdataEnd)
        }
        _ => State::Retry(StateName::HtmlTextCdata),
    }
}

pub fn cdata_end(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'>') => State::Retry(StateName::HtmlTextEnd),
        Some(b']') => State::Retry(StateName::HtmlTextCdataClose),
        _ => State::Retry(StateName::HtmlTextCdata),
    }
}

pub fn declaration(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None => unclosed(tokenizer, DECLARATION),
        Some(b'>') => State::Retry(StateName::HtmlTextEnd),
        Some(b'\n') => {
            tokenizer.attempt(State::Next(StateName::HtmlTextDeclaration), State::Nok);
            State::Retry(StateName::HtmlTextLineEndingBefore)
        }
        _ => {
            tokenizer.consume();
            State::Next(StateName::HtmlTextDeclaration)
        }
    }
}

pub fn instruction(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None => unclosed(tokenizer, INSTRUCTION),
        Some(b'\n') => {
            tokenizer.attempt(State::Next(StateName::HtmlTextInstruction), State::Nok);
            State::Retry(StateName::HtmlTextLineEndingBefore)
        }
        Some(b'?') => {
            tokenizer.consume();
            State::Next(StateName::HtmlTextInstructionClose)
        }
        _ => {
            tokenizer.consume();
            State::Next(StateName::HtmlTextInstruction)
        }
    }
}

pub fn instruction_close(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'>') => State::Retry(StateName::HtmlTextEnd),
        _ => State::Retry(StateName::HtmlTextInstruction),
    }
}

pub fn tag_close_start(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'A'..=b'Z' | b'a'..=b'z') => {
            tokenizer.consume();
            State::Next(StateName::HtmlTextTagClose)
        }
        _ => State::Nok,
    }
}

pub fn tag_close(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'-' | b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z') => {
            tokenizer.consume();
            State::Next(StateName::HtmlTextTagClose)
        }
        _ => State::Retry(StateName::HtmlTextTagCloseBetween),
    }
}

pub fn tag_close_between(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'\n') => {
            tokenizer.attempt(State::Next(StateName::HtmlTextTagCloseBetween), State::Nok);
            State::Retry(StateName::HtmlTextLineEndingBefore)
        }
        Some(b'\t' | b' ') => {
            tokenizer.consume();
            State::Next(StateName::HtmlTextTagCloseBetween)
        }
        _ => State::Retry(StateName::HtmlTextEnd),
    }
}

pub fn tag_open(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'-' | b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z') => {
            tokenizer.consume();
            State::Next(StateName::HtmlTextTagOpen)
        }
        Some(b'\t' | b'\n' | b' ' | b'/' | b'>') => State::Retry(StateName::HtmlTextTagOpenBetween),
        _ => State::Nok,
    }
}

pub fn tag_open_between(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'\n') => {
            tokenizer.attempt(State::Next(StateName::HtmlTextTagOpenBetween), State::Nok);
            State::Retry(StateName::HtmlTextLineEndingBefore)
        }
        Some(b'\t' | b' ') => {
            tokenizer.consume();
            State::Next(StateName::HtmlTextTagOpenBetween)
        }
        Some(b'/') => {
            tokenizer.consume();
            State::Next(StateName::HtmlTextEnd)
        }
        Some(b':' | b'A'..=b'Z' | b'_' | b'a'..=b'z') => {
            tokenizer.consume();
            State::Next(StateName::HtmlTextTagOpenAttributeName)
        }
        _ => State::Retry(StateName::HtmlTextEnd),
    }
}

pub fn tag_open_attribute_name(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'-' | b'.' | b'0'..=b'9' | b':' | b'A'..=b'Z' | b'_' | b'a'..=b'z') => {
            tokenizer.consume();
            State::Next(StateName::HtmlTextTagOpenAttributeName)
        }
        _ => State::Retry(StateName::HtmlTextTagOpenAttributeNameAfter),
    }
}

pub fn tag_open_attribute_name_after(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'\n') => {
            tokenizer.attempt(State::Next(StateName::HtmlTextTagOpenAttributeNameAfter), State::Nok);
            State::Retry(StateName::HtmlTextLineEndingBefore)
        }
        Some(b'\t' | b' ') => {
            tokenizer.consume();
            State::Next(StateName::HtmlTextTagOpenAttributeNameAfter)
        }
        Some(b'=') => {
            tokenizer.consume();
            State::Next(StateName::HtmlTextTagOpenAttributeValueBefore)
        }
        _ => State::Retry(StateName::HtmlTextTagOpenBetween),
    }
}

pub fn tag_open_attribute_value_before(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'<' | b'=' | b'>' | b'`') => State::Nok,
        Some(b'\n') => {
            tokenizer.attempt(State::Next(StateName::HtmlTextTagOpenAttributeValueBefore), State::Nok);
            State::Retry(StateName::HtmlTextLineEndingBefore)
        }
        Some(b'\t' | b' ') => {
            tokenizer.consume();
            State::Next(StateName::HtmlTextTagOpenAttributeValueBefore)
        }
        Some(b'"' | b'\'') => {
            tokenizer.tokenize_state.marker = tokenizer.current.unwrap();
            tokenizer.consume();
            State::Next(StateName::HtmlTextTagOpenAttributeValueQuoted)
        }
        Some(_) => {
            tokenizer.consume();
            State::Next(StateName::HtmlTextTagOpenAttributeValueUnquoted)
        }
    }
}

pub fn tag_open_attribute_value_quoted(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(tokenizer.tokenize_state.marker) {
        tokenizer.tokenize_state.marker = 0;
        tokenizer.consume();
        State::Next(StateName::HtmlTextTagOpenAttributeValueQuotedAfter)
    } else {
        match tokenizer.current {
            None => {
                tokenizer.tokenize_state.marker = 0;
                State::Nok
            }
            Some(b'\n') => {
                tokenizer.attempt(State::Next(StateName::HtmlTextTagOpenAttributeValueQuoted), State::Nok);
                State::Retry(StateName::HtmlTextLineEndingBefore)
            }
            _ => {
                tokenizer.consume();
                State::Next(StateName::HtmlTextTagOpenAttributeValueQuoted)
            }
        }
    }
}

pub fn tag_open_attribute_value_unquoted(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'"' | b'\'' | b'<' | b'=' | b'`') => State::Nok,
        Some(b'\t' | b'\n' | b' ' | b'/' | b'>') => State::Retry(StateName::HtmlTextTagOpenBetween),
        Some(_) => {
            tokenizer.consume();
            State::Next(StateName::HtmlTextTagOpenAttributeValueUnquoted)
        }
    }
}

pub fn tag_open_attribute_value_quoted_after(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'\t' | b'\n' | b' ' | b'/' | b'>') => State::Retry(StateName::HtmlTextTagOpenBetween),
        _ => State::Nok,
    }
}

pub fn end(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'>') => {
            tokenizer.consume();
            tokenizer.exit(Name::HtmlTextData);
            tokenizer.exit(Name::HtmlText);
            State::Ok
        }
        _ => State::Nok,
    }
}

pub fn line_ending_before(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'\n') => {
            tokenizer.exit(Name::HtmlTextData);
            tokenizer.enter(Name::LineEnding);
            tokenizer.consume();
            tokenizer.exit(Name::LineEnding);
            State::Next(StateName::HtmlTextLineEndingAfter)
        }
        _ => unreachable!("expected eol"),
    }
}

pub fn line_ending_after(tokenizer: &mut Tokenizer) -> State {
    if matches!(tokenizer.current, Some(b'\t' | b' ')) {
        tokenizer.attempt(State::Next(StateName::HtmlTextLineEndingAfterPrefix), State::Nok);
        State::Retry(space_or_tab(tokenizer))
    } else {
        State::Retry(StateName::HtmlTextLineEndingAfterPrefix)
    }
}

pub fn line_ending_after_prefix(tokenizer: &mut Tokenizer) -> State {
    tokenizer.enter(Name::HtmlTextData);
    State::Ok
}
