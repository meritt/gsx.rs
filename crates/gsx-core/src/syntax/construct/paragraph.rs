use crate::syntax::event::{Content, Link, Name};
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::subtokenize::link;
use crate::syntax::tokenizer::Tokenizer;

pub fn start(tokenizer: &mut Tokenizer) -> State {
    debug_assert!(tokenizer.current.is_some());
    tokenizer.enter(Name::Paragraph);
    State::Retry(StateName::ParagraphLineStart)
}

pub fn line_start(tokenizer: &mut Tokenizer) -> State {
    debug_assert!(tokenizer.current.is_some());
    tokenizer.enter_link(Name::Data, Link::new(Content::Text));

    if tokenizer.tokenize_state.connect {
        let index = tokenizer.events.len() - 1;
        link(&mut tokenizer.events, index);
    } else {
        tokenizer.tokenize_state.connect = true;
    }

    State::Retry(StateName::ParagraphInside)
}

pub fn inside(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None => {
            tokenizer.tokenize_state.connect = false;
            tokenizer.exit(Name::Data);
            tokenizer.exit(Name::Paragraph);
            State::Ok
        }
        Some(b'\n') => {
            tokenizer.consume();
            tokenizer.exit(Name::Data);
            State::Next(StateName::ParagraphLineStart)
        }
        _ => {
            tokenizer.consume_while(|_| true);
            State::Next(StateName::ParagraphInside)
        }
    }
}
