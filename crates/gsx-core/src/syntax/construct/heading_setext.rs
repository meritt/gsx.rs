use crate::syntax::construct::partial_space_or_tab::{space_or_tab, space_or_tab_min_max};
use crate::syntax::event::{Content, Event, Kind, Link, Name};
use crate::syntax::resolve::Name as ResolveName;
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::subtokenize::Subresult;
use crate::syntax::tokenizer::Tokenizer;
use crate::syntax::util::{constant::TAB_SIZE, skip};

pub fn start(tokenizer: &mut Tokenizer) -> State {
    if !tokenizer.lazy
        && !tokenizer.pierce
        && !tokenizer.events.is_empty()
        && content_before(tokenizer, tokenizer.events.len() - 1).is_some()
    {
        tokenizer.enter(Name::HeadingSetextUnderline);

        if matches!(tokenizer.current, Some(b'\t' | b' ')) {
            tokenizer.attempt(State::Next(StateName::HeadingSetextBefore), State::Nok);
            State::Retry(space_or_tab_min_max(tokenizer, 0, TAB_SIZE - 1))
        } else {
            State::Retry(StateName::HeadingSetextBefore)
        }
    } else {
        State::Nok
    }
}

fn content_before(tokenizer: &Tokenizer, index: usize) -> Option<usize> {
    let exit = skip::opt_back(&tokenizer.events, index, &[Name::LineEnding, Name::SpaceOrTab]);
    let event = &tokenizer.events[exit];
    (event.kind == Kind::Exit && event.name == Name::Content).then_some(exit)
}

fn paragraph_before(tokenizer: &Tokenizer, mut exit: usize) -> bool {
    let events = &tokenizer.events;
    let mut end = events[exit].point;
    let mut chunks = vec![];

    loop {
        let enter = exit - 1;
        chunks.push((events[enter].point, end));

        if enter < 3 {
            break;
        }

        let before =
            skip::opt_back(events, enter - 1, &[Name::SpaceOrTab, Name::BlockQuotePrefix, Name::BlockQuoteMarker]);

        if before < 3
            || events[before].kind != Kind::Exit
            || events[before].name != Name::LineEnding
            || events[before - 2].kind != Kind::Exit
            || events[before - 2].name != Name::Content
        {
            break;
        }

        end = events[before].point;
        exit = before - 2;
    }

    chunks.reverse();
    let mut child = Tokenizer::new(chunks[0].0, tokenizer.parse_state);
    let mut state = State::Next(StateName::ContentDefinitionBefore);

    for (index, (start, end)) in chunks.iter().enumerate() {
        if index > 0 {
            child.define_skip(*start);
        }

        state = child.push((start.index, start.vs), (end.index, end.vs), state);
    }

    child.flush(state, false);
    child.events.iter().any(|event| event.name == Name::Paragraph)
}

pub fn before(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'-' | b'=') => {
            tokenizer.tokenize_state.marker = tokenizer.current.unwrap();
            tokenizer.enter(Name::HeadingSetextUnderlineSequence);
            State::Retry(StateName::HeadingSetextInside)
        }
        _ => State::Nok,
    }
}

pub fn inside(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(tokenizer.tokenize_state.marker) {
        tokenizer.consume();
        State::Next(StateName::HeadingSetextInside)
    } else {
        tokenizer.tokenize_state.marker = 0;
        tokenizer.exit(Name::HeadingSetextUnderlineSequence);

        if matches!(tokenizer.current, Some(b'\t' | b' ')) {
            tokenizer.attempt(State::Next(StateName::HeadingSetextAfter), State::Nok);
            State::Retry(space_or_tab(tokenizer))
        } else {
            State::Retry(StateName::HeadingSetextAfter)
        }
    }
}

pub fn after(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n') => {
            let enter = skip::to_back(&tokenizer.events, tokenizer.events.len() - 1, &[Name::HeadingSetextUnderline]);
            if !content_before(tokenizer, enter - 1).is_some_and(|exit| paragraph_before(tokenizer, exit)) {
                return State::Nok;
            }
            tokenizer.interrupt = false;
            tokenizer.register_resolver(ResolveName::HeadingSetext);
            tokenizer.exit(Name::HeadingSetextUnderline);
            State::Ok
        }
        _ => State::Nok,
    }
}

pub fn resolve(tokenizer: &mut Tokenizer) -> Option<Subresult> {
    let mut enter = skip::to(&tokenizer.events, 0, &[Name::HeadingSetextUnderline]);

    while enter < tokenizer.events.len() {
        let exit = skip::to(&tokenizer.events, enter + 1, &[Name::HeadingSetextUnderline]);

        let paragraph_exit_before =
            skip::opt_back(&tokenizer.events, enter - 1, &[Name::SpaceOrTab, Name::LineEnding, Name::BlockQuotePrefix]);

        if tokenizer.events[paragraph_exit_before].name == Name::Paragraph {
            let paragraph_enter = skip::to_back(&tokenizer.events, paragraph_exit_before - 1, &[Name::Paragraph]);

            tokenizer.events[paragraph_enter].name = Name::HeadingSetextText;
            tokenizer.events[paragraph_exit_before].name = Name::HeadingSetextText;

            let mut heading_enter = tokenizer.events[paragraph_enter];
            heading_enter.name = Name::HeadingSetext;
            tokenizer.map.add(paragraph_enter, 0, vec![heading_enter]);
            let mut heading_exit = tokenizer.events[exit];
            heading_exit.name = Name::HeadingSetext;
            tokenizer.map.add(exit + 1, 0, vec![heading_exit]);
        } else if exit + 3 < tokenizer.events.len()
            && tokenizer.events[exit + 1].name == Name::LineEnding
            && tokenizer.events[exit + 3].name == Name::Paragraph
        {
            tokenizer.events[enter].name = Name::Paragraph;
            tokenizer.events[exit + 1].name = Name::Data;
            tokenizer.events[exit + 2].name = Name::Data;
            tokenizer.events[exit + 1].point = tokenizer.events[enter].point;
            let mut link = Link::new(Content::Text);
            link.set_next(Some(exit + 4));
            tokenizer.events[exit + 1].link = Some(link);
            tokenizer.events[exit + 4].link.as_mut().unwrap().set_previous(Some(exit + 1));
            tokenizer.map.add(enter + 1, exit - enter, vec![]);
            tokenizer.map.add(exit + 3, 1, vec![]);
        } else {
            tokenizer.events[enter].name = Name::Paragraph;
            tokenizer.events[exit].name = Name::Paragraph;
            tokenizer.map.add(
                enter + 1,
                exit - enter - 1,
                vec![
                    Event {
                        name: Name::Data,
                        kind: Kind::Enter,
                        point: tokenizer.events[enter].point,
                        link: Some(Link::new(Content::Text)),
                    },
                    Event { name: Name::Data, kind: Kind::Exit, point: tokenizer.events[exit].point, link: None },
                ],
            );
        }

        enter = skip::to(&tokenizer.events, exit + 1, &[Name::HeadingSetextUnderline]);
    }

    tokenizer.map.consume(&mut tokenizer.events);
    None
}
