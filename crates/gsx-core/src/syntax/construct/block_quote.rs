use crate::syntax::construct::partial_space_or_tab::space_or_tab_min_max;
use crate::syntax::event::Name;
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::tokenizer::Tokenizer;
use crate::syntax::util::constant::TAB_SIZE;

pub fn start(tokenizer: &mut Tokenizer) -> State {
    tokenizer.enter(Name::BlockQuote);
    State::Retry(StateName::BlockQuoteContStart)
}

pub fn cont_start(tokenizer: &mut Tokenizer) -> State {
    if matches!(tokenizer.current, Some(b'\t' | b' ')) {
        tokenizer.attempt(State::Next(StateName::BlockQuoteContBefore), State::Nok);
        State::Retry(space_or_tab_min_max(tokenizer, 1, TAB_SIZE - 1))
    } else {
        State::Retry(StateName::BlockQuoteContBefore)
    }
}

pub fn cont_before(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'>') => {
            tokenizer.enter(Name::BlockQuotePrefix);
            tokenizer.enter(Name::BlockQuoteMarker);
            tokenizer.consume();
            tokenizer.exit(Name::BlockQuoteMarker);
            State::Next(StateName::BlockQuoteContAfter)
        }
        _ => State::Nok,
    }
}

pub fn cont_after(tokenizer: &mut Tokenizer) -> State {
    if let Some(b'\t' | b' ') = tokenizer.current {
        tokenizer.enter(Name::SpaceOrTab);
        tokenizer.consume();
        tokenizer.exit(Name::SpaceOrTab);
    }

    tokenizer.exit(Name::BlockQuotePrefix);
    State::Ok
}
