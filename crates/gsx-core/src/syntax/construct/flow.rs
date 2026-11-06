use crate::syntax::event::Name;
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::tokenizer::Tokenizer;

pub fn start(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(State::Next(StateName::FlowBlankLineAfter), State::Next(StateName::FlowBeforeCodeIndented));
    State::Retry(StateName::BlankLineStart)
}

pub fn before_code_indented(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(State::Next(StateName::FlowAfter), State::Next(StateName::FlowBeforeRaw));
    State::Retry(StateName::CodeIndentedStart)
}

pub fn before_raw(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(State::Next(StateName::FlowAfter), State::Next(StateName::FlowBeforeHtml));
    State::Retry(StateName::RawFlowStart)
}

pub fn before_html(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(State::Next(StateName::FlowAfter), State::Next(StateName::FlowBeforeHeadingAtx));
    State::Retry(StateName::HtmlFlowStart)
}

pub fn before_heading_atx(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(State::Next(StateName::FlowAfter), State::Next(StateName::FlowBeforeHeadingSetext));
    State::Retry(StateName::HeadingAtxStart)
}

pub fn before_heading_setext(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(State::Next(StateName::FlowAfter), State::Next(StateName::FlowBeforeThematicBreak));
    State::Retry(StateName::HeadingSetextStart)
}

pub fn before_thematic_break(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(State::Next(StateName::FlowAfter), State::Next(StateName::FlowBeforeGsxMedia));
    State::Retry(StateName::ThematicBreakStart)
}

pub fn before_gsx_media(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(State::Next(StateName::FlowAfter), State::Next(StateName::FlowBeforeGfmTable));
    State::Retry(StateName::GsxMediaFlowStart)
}

pub fn before_gfm_table(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(State::Next(StateName::FlowAfter), State::Next(StateName::FlowBeforeContent));
    State::Retry(StateName::GfmTableStart)
}

pub fn before_content(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(State::Next(StateName::FlowAfter), State::Nok);
    State::Retry(StateName::ContentChunkStart)
}

pub fn blank_line_after(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None => State::Ok,
        Some(b'\n') => {
            tokenizer.enter(Name::BlankLineEnding);
            tokenizer.consume();
            tokenizer.exit(Name::BlankLineEnding);
            tokenizer.interrupt = false;
            State::Next(StateName::FlowStart)
        }
        _ => unreachable!("expected eol/eof"),
    }
}

pub fn after(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None => State::Ok,
        Some(b'\n') => {
            tokenizer.enter(Name::LineEnding);
            tokenizer.consume();
            tokenizer.exit(Name::LineEnding);
            State::Next(StateName::FlowStart)
        }
        _ => unreachable!("expected eol/eof"),
    }
}
