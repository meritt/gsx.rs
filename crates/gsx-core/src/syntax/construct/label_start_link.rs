use crate::source::Limit;
use crate::syntax::event::Name;
use crate::syntax::resolve::Name as ResolveName;
use crate::syntax::state::State;
use crate::syntax::tokenizer::{LabelKind, LabelStart, Tokenizer};

pub fn start(tokenizer: &mut Tokenizer) -> State {
    let index = tokenizer.point.index;
    if tokenizer.current == Some(b'[') && tokenizer.spend(Limit::BlockLabels, index, |_| index + 1) {
        let start = tokenizer.events.len();
        tokenizer.enter(Name::LabelLink);
        tokenizer.enter(Name::LabelMarker);
        tokenizer.consume();
        tokenizer.exit(Name::LabelMarker);
        tokenizer.exit(Name::LabelLink);
        tokenizer.tokenize_state.label_starts.push(LabelStart {
            kind: LabelKind::Link,
            start: (start, tokenizer.events.len() - 1),
            inactive: false,
        });
        tokenizer.register_resolver_before(ResolveName::Label);
        State::Ok
    } else {
        State::Nok
    }
}
