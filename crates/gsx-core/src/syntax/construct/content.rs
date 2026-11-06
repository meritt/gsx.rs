use crate::syntax::event::{Content, Kind, Link, Name};
use crate::syntax::resolve::Name as ResolveName;
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::subtokenize::{Subresult, subtokenize};
use crate::syntax::tokenizer::Tokenizer;

pub fn chunk_start(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n') => unreachable!("unexpected eol/eof"),
        _ => {
            tokenizer.enter_link(Name::Content, Link::new(Content::Content));
            State::Retry(StateName::ContentChunkInside)
        }
    }
}

pub fn chunk_inside(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n') => {
            tokenizer.exit(Name::Content);
            tokenizer.register_resolver_before(ResolveName::Content);
            tokenizer.interrupt = true;
            State::Ok
        }
        _ => {
            tokenizer.consume_while(|_| true);
            State::Next(StateName::ContentChunkInside)
        }
    }
}

pub fn definition_before(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(State::Next(StateName::ContentDefinitionAfter), State::Next(StateName::ParagraphStart));
    State::Retry(StateName::DefinitionStart)
}

pub fn definition_after(tokenizer: &mut Tokenizer) -> State {
    debug_assert!(matches!(tokenizer.current, None | Some(b'\n')));
    if tokenizer.current.is_none() {
        State::Ok
    } else {
        tokenizer.enter(Name::LineEnding);
        tokenizer.consume();
        tokenizer.exit(Name::LineEnding);
        State::Next(StateName::ContentDefinitionBefore)
    }
}

pub fn resolve(tokenizer: &mut Tokenizer) -> Option<Subresult> {
    let mut index = 0;

    while index < tokenizer.events.len() {
        let event = &tokenizer.events[index];

        if event.kind == Kind::Enter && event.name == Name::Content {
            let mut exit_index = index + 1;

            loop {
                let mut enter_index = exit_index + 1;

                if enter_index == tokenizer.events.len() || tokenizer.events[enter_index].name != Name::LineEnding {
                    break;
                }

                enter_index += 2;

                while enter_index < tokenizer.events.len() {
                    let event = &tokenizer.events[enter_index];

                    if event.name != Name::SpaceOrTab
                        && event.name != Name::BlockQuotePrefix
                        && event.name != Name::BlockQuoteMarker
                    {
                        break;
                    }

                    enter_index += 1;
                }

                if enter_index == tokenizer.events.len() || tokenizer.events[enter_index].name != Name::Content {
                    break;
                }

                tokenizer.events[exit_index].point = tokenizer.events[exit_index + 2].point;
                tokenizer.map.add(exit_index + 1, 2, vec![]);

                tokenizer.events[exit_index - 1].link.as_mut().unwrap().set_next(Some(enter_index));
                tokenizer.events[enter_index].link.as_mut().unwrap().set_previous(Some(exit_index - 1));

                exit_index = enter_index + 1;
            }

            index = exit_index;
        }

        index += 1;
    }

    tokenizer.map.consume(&mut tokenizer.events);

    Some(subtokenize(&mut tokenizer.events, tokenizer.parse_state, Some(&Content::Content)))
}
