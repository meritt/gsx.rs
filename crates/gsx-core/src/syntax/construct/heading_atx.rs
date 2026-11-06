use crate::syntax::construct::partial_space_or_tab::{space_or_tab, space_or_tab_min_max};
use crate::syntax::event::{Content, Event, Kind, Link, Name};
use crate::syntax::resolve::Name as ResolveName;
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::subtokenize::Subresult;
use crate::syntax::tokenizer::Tokenizer;
use crate::syntax::util::constant::{HEADING_ATX_OPENING_FENCE_SIZE_MAX, TAB_SIZE};

pub fn start(tokenizer: &mut Tokenizer) -> State {
    tokenizer.enter(Name::HeadingAtx);
    if matches!(tokenizer.current, Some(b'\t' | b' ')) {
        tokenizer.attempt(State::Next(StateName::HeadingAtxBefore), State::Nok);
        State::Retry(space_or_tab_min_max(tokenizer, 0, TAB_SIZE - 1))
    } else {
        State::Retry(StateName::HeadingAtxBefore)
    }
}

pub fn before(tokenizer: &mut Tokenizer) -> State {
    if Some(b'#') == tokenizer.current {
        tokenizer.enter(Name::HeadingAtxSequence);
        State::Retry(StateName::HeadingAtxSequenceOpen)
    } else {
        State::Nok
    }
}

pub fn sequence_open(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(b'#') && tokenizer.tokenize_state.size < HEADING_ATX_OPENING_FENCE_SIZE_MAX {
        tokenizer.tokenize_state.size += 1;
        tokenizer.consume();
        State::Next(StateName::HeadingAtxSequenceOpen)
    } else if matches!(tokenizer.current, None | Some(b'\t' | b'\n' | b' ')) {
        tokenizer.tokenize_state.size = 0;
        tokenizer.exit(Name::HeadingAtxSequence);
        State::Retry(StateName::HeadingAtxAtBreak)
    } else {
        tokenizer.tokenize_state.size = 0;
        State::Nok
    }
}

pub fn at_break(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n') => {
            tokenizer.exit(Name::HeadingAtx);
            tokenizer.register_resolver(ResolveName::HeadingAtx);
            tokenizer.interrupt = false;
            State::Ok
        }
        Some(b'\t' | b' ') => {
            tokenizer.attempt(State::Next(StateName::HeadingAtxAtBreak), State::Nok);
            State::Retry(space_or_tab(tokenizer))
        }
        Some(b'#') => {
            tokenizer.enter(Name::HeadingAtxSequence);
            State::Retry(StateName::HeadingAtxSequenceFurther)
        }
        Some(_) => {
            tokenizer.enter_link(Name::Data, Link::new(Content::Text));
            State::Retry(StateName::HeadingAtxData)
        }
    }
}

pub fn sequence_further(tokenizer: &mut Tokenizer) -> State {
    if let Some(b'#') = tokenizer.current {
        tokenizer.consume();
        State::Next(StateName::HeadingAtxSequenceFurther)
    } else {
        tokenizer.exit(Name::HeadingAtxSequence);
        State::Retry(StateName::HeadingAtxAtBreak)
    }
}

pub fn data(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\t' | b'\n' | b' ') => {
            tokenizer.exit(Name::Data);
            State::Retry(StateName::HeadingAtxAtBreak)
        }
        _ => {
            tokenizer.consume();
            State::Next(StateName::HeadingAtxData)
        }
    }
}

pub fn resolve(tokenizer: &mut Tokenizer) -> Option<Subresult> {
    let mut index = 0;

    while index < tokenizer.events.len() {
        if tokenizer.events[index].kind == Kind::Enter && tokenizer.events[index].name == Name::HeadingAtx {
            let mut items = vec![];
            let mut opened = false;
            let mut at = index + 1;

            while tokenizer.events[at].name != Name::HeadingAtx {
                let name = &tokenizer.events[at].name;
                if *name == Name::HeadingAtxSequence && !opened {
                    opened = true;
                } else if *name != Name::SpaceOrTab {
                    items.push(at);
                }
                at += 2;
            }

            if items.last().is_some_and(|&last| tokenizer.events[last].name == Name::HeadingAtxSequence) {
                items.pop();
            }

            if let (Some(&first), Some(&last)) = (items.first(), items.last()) {
                let start = tokenizer.events[first].point;
                let end = tokenizer.events[last + 1].point;
                let link = Link::new(Content::Text);
                tokenizer.map.add(
                    first,
                    last + 2 - first,
                    vec![
                        Event { kind: Kind::Enter, name: Name::HeadingAtxText, point: start, link: None },
                        Event { kind: Kind::Enter, name: Name::Data, point: start, link: Some(link) },
                        Event { kind: Kind::Exit, name: Name::Data, point: end, link: None },
                        Event { kind: Kind::Exit, name: Name::HeadingAtxText, point: end, link: None },
                    ],
                );
            }

            index = at;
        }

        index += 1;
    }

    tokenizer.map.consume(&mut tokenizer.events);
    None
}
