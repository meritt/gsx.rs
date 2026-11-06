use crate::source::Limit;
use crate::syntax::construct::gfm_autolink_literal::resolve as resolve_gfm_autolink_literal;
use crate::syntax::construct::partial_whitespace::resolve_whitespace;
use crate::syntax::event::Name;
use crate::syntax::resolve::Name as ResolveName;
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::subtokenize::Subresult;
use crate::syntax::tokenizer::{Markers, NO_MARKERS, Tokenizer};

static MARKERS: Markers = Markers::new(b"$|+=:!&*<HW[\\]_`hw~");

pub fn start(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.markers = &MARKERS;
    tokenizer.attempt(State::Next(StateName::TextBefore), State::Next(StateName::TextBefore));
    State::Retry(StateName::GfmTaskListItemCheckStart)
}

pub fn before(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current.is_some()
        && tokenizer.parse_state.exceeded.get().is_some_and(|exceeded| exceeded.start <= tokenizer.point.index)
    {
        tokenizer.tokenize_state.markers = &NO_MARKERS;
        return State::Retry(StateName::TextBeforeData);
    }
    match tokenizer.current {
        None => {
            tokenizer.register_resolver(ResolveName::GsxPaired);

            tokenizer.register_resolver(ResolveName::Data);
            tokenizer.register_resolver(ResolveName::Text);
            State::Ok
        }
        Some(b'!') => {
            tokenizer.attempt(State::Next(StateName::TextBefore), State::Next(StateName::TextBeforeData));
            State::Retry(StateName::LabelStartImageStart)
        }
        Some(b'`') => {
            tokenizer.attempt(State::Next(StateName::TextBefore), State::Next(StateName::TextBeforeData));
            State::Retry(StateName::RawTextStart)
        }
        Some(b'$') => {
            tokenizer.attempt(State::Next(StateName::TextBefore), State::Next(StateName::TextBeforeData));
            State::Retry(StateName::GsxMathStart)
        }
        Some(b'&') => {
            tokenizer.attempt(State::Next(StateName::TextBefore), State::Next(StateName::TextBeforeData));
            State::Retry(StateName::CharacterReferenceStart)
        }
        Some(b'*' | b'_' | b'~') => {
            tokenizer.attempt(State::Next(StateName::TextBefore), State::Next(StateName::TextBeforeData));
            State::Retry(StateName::AttentionStart)
        }
        Some(b'<') => {
            tokenizer.attempt(State::Next(StateName::TextBefore), State::Next(StateName::TextBeforeHtml));
            State::Retry(StateName::AutolinkStart)
        }
        Some(b'H' | b'h') => {
            tokenizer.attempt(State::Next(StateName::TextBefore), State::Next(StateName::TextBeforeData));
            State::Retry(StateName::GfmAutolinkLiteralProtocolStart)
        }
        Some(b'W' | b'w') => {
            tokenizer.attempt(State::Next(StateName::TextBefore), State::Next(StateName::TextBeforeData));
            State::Retry(StateName::GfmAutolinkLiteralWwwStart)
        }
        Some(b'[') => {
            tokenizer.attempt(State::Next(StateName::TextBefore), State::Next(StateName::TextBeforeLabelStartLink));
            State::Retry(StateName::GfmLabelStartFootnoteStart)
        }
        Some(b'\\') => {
            tokenizer.attempt(State::Next(StateName::TextBefore), State::Next(StateName::TextBeforeHardBreakEscape));
            State::Retry(StateName::CharacterEscapeStart)
        }
        Some(b']') if tokenizer.tokenize_state.label_starts.is_empty() => {
            tokenizer.enter(Name::Data);
            State::Retry(StateName::TextUnopenedLabelEnds)
        }
        Some(b']') => {
            tokenizer.attempt(State::Next(StateName::TextBefore), State::Next(StateName::TextBeforeData));
            State::Retry(StateName::LabelEndStart)
        }
        Some(b'|' | b'+' | b'=') => {
            tokenizer.attempt(State::Next(StateName::TextBefore), State::Next(StateName::TextBeforeData));
            State::Retry(StateName::GsxPairedStart)
        }
        Some(b':') => {
            tokenizer.attempt(State::Next(StateName::TextBefore), State::Next(StateName::TextBeforeDirective));
            State::Retry(StateName::GsxMediaTextStart)
        }
        _ => State::Retry(StateName::TextBeforeData),
    }
}

pub fn before_directive(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(State::Next(StateName::TextBefore), State::Next(StateName::TextBeforeData));
    State::Retry(StateName::GsxDirectiveTextStart)
}

pub fn before_html(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(State::Next(StateName::TextBefore), State::Next(StateName::TextBeforeData));
    State::Retry(StateName::HtmlTextStart)
}

pub fn before_hard_break_escape(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(State::Next(StateName::TextBefore), State::Next(StateName::TextBeforeData));
    State::Retry(StateName::HardBreakEscapeStart)
}

pub fn before_label_start_link(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(State::Next(StateName::TextBefore), State::Next(StateName::TextBeforeData));
    State::Retry(StateName::LabelStartLinkStart)
}

pub fn unopened_label_ends(tokenizer: &mut Tokenizer) -> State {
    let data = |byte| byte == b']' || !MARKERS.contains(byte);
    match tokenizer.current {
        Some(byte) if byte != b'\n' && data(byte) => {
            let start = tokenizer.point.index;
            tokenizer.consume_while(data);
            let run = &tokenizer.parse_state.bytes[start..tokenizer.point.index];
            for (offset, _) in run.iter().enumerate().filter(|(_, byte)| **byte == b']') {
                let index = start + offset;
                tokenizer.spend(Limit::BlockLabelEnds, index, |_| index + 1);
            }
            State::Next(StateName::TextUnopenedLabelEnds)
        }
        _ => {
            tokenizer.exit(Name::Data);
            State::Retry(StateName::TextBefore)
        }
    }
}

pub fn before_data(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(State::Next(StateName::TextBefore), State::Nok);
    State::Retry(StateName::DataStart)
}

pub fn resolve(tokenizer: &mut Tokenizer) -> Option<Subresult> {
    resolve_whitespace(tokenizer, true, true);

    resolve_gfm_autolink_literal(tokenizer);

    tokenizer.map.consume(&mut tokenizer.events);
    None
}
