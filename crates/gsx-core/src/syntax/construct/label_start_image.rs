use crate::source::Limit;
use crate::syntax::event::Name;
use crate::syntax::resolve::Name as ResolveName;
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::tokenizer::{LabelKind, LabelStart, Tokenizer};

pub fn start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b'!') {
        tokenizer.enter(Name::LabelImage);
        tokenizer.enter(Name::LabelImageMarker);
        tokenizer.consume();
        tokenizer.exit(Name::LabelImageMarker);
        State::Next(StateName::LabelStartImageOpen)
    } else {
        State::Nok
    }
}

pub fn open(tokenizer: &mut Tokenizer) -> State {
    let index = tokenizer.point.index as usize;
    if tokenizer.current == Some(b'[') && tokenizer.spend(Limit::BlockLabels, index, |_| index + 1) {
        tokenizer.enter(Name::LabelMarker);
        tokenizer.consume();
        tokenizer.exit(Name::LabelMarker);
        tokenizer.exit(Name::LabelImage);
        State::Next(StateName::LabelStartImageAfter)
    } else {
        State::Nok
    }
}

pub fn after(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b'^') {
        State::Nok
    } else {
        tokenizer.tokenize_state.label_starts.push(LabelStart {
            kind: LabelKind::Image,
            start: (tokenizer.events.len() - 6, tokenizer.events.len() - 1),
            inactive: false,
        });
        tokenizer.register_resolver_before(ResolveName::Label);
        State::Ok
    }
}
