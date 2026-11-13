use crate::source::Limit;
use crate::syntax::event::Name;
use crate::syntax::resolve::Name as ResolveName;
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::tokenizer::{LabelKind, LabelStart, Tokenizer};

pub fn start(tokenizer: &mut Tokenizer) -> State {
    let index = tokenizer.point.index as usize;
    if tokenizer.current == Some(b'[') && tokenizer.spend(Limit::BlockLabels, index, |_| index + 1) {
        tokenizer.enter(Name::GfmFootnoteCallLabel);
        tokenizer.enter(Name::LabelMarker);
        tokenizer.consume();
        tokenizer.exit(Name::LabelMarker);
        State::Next(StateName::GfmLabelStartFootnoteOpen)
    } else {
        State::Nok
    }
}

pub fn open(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'^') => {
            tokenizer.enter(Name::GfmFootnoteCallMarker);
            tokenizer.consume();
            tokenizer.exit(Name::GfmFootnoteCallMarker);
            tokenizer.exit(Name::GfmFootnoteCallLabel);
            tokenizer.tokenize_state.label_starts.push(LabelStart {
                kind: LabelKind::GfmFootnote,
                start: (tokenizer.events.len() - 6, tokenizer.events.len() - 1),
                inactive: false,
            });
            tokenizer.register_resolver_before(ResolveName::Label);
            State::Ok
        }
        _ => State::Nok,
    }
}
