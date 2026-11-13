use crate::syntax::event::{Content, Event, Kind, Name, VOID_EVENTS};
use crate::syntax::parser::ParseState;
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::tokenizer::Tokenizer;
use crate::syntax::util::{edit_map::EditMap, skip};

#[derive(Debug)]
pub struct Subresult {
    pub done: bool,
    pub gfm_footnote_definitions: Vec<String>,
    pub definitions: Vec<String>,
}

pub fn link(events: &mut [Event], index: usize) {
    link_to(events, index - 2, index);
}

pub fn link_to(events: &mut [Event], previous: usize, next: usize) {
    debug_assert_eq!(events[previous].kind, Kind::Enter);
    debug_assert!(VOID_EVENTS.iter().any(|d| d == &events[previous].name), "expected event to be void");
    debug_assert_eq!(events[previous + 1].kind, Kind::Exit);
    debug_assert_eq!(events[previous].name, events[previous + 1].name);
    debug_assert_eq!(events[next].kind, Kind::Enter);
    debug_assert!(VOID_EVENTS.iter().any(|d| d == &events[next].name), "expected event to be void");

    let link_previous = events[previous].link.as_mut().expect("expected `link` on previous");
    link_previous.set_next(Some(next));
    let link_next = events[next].link.as_mut().expect("expected `link` on next");
    link_next.set_previous(Some(previous));

    debug_assert_eq!(
        events[previous].link.as_ref().unwrap().content,
        events[next].link.as_ref().unwrap().content,
        "expected `content` to match"
    );
}

pub fn subtokenize(events: &mut Vec<Event>, parse_state: &ParseState, filter: Option<&Content>) -> Subresult {
    let mut map = EditMap::new();
    let mut index = 0;
    let mut value = Subresult { done: true, gfm_footnote_definitions: vec![], definitions: vec![] };
    let mut acc = (0, 0);

    while index < events.len() {
        let event = &events[index];

        if let Some(ref link) = event.link {
            debug_assert_eq!(event.kind, Kind::Enter);

            if link.previous().is_none() && filter.is_none_or(|filter| link.content == *filter) {
                let mut link_index = Some(index);
                let mut tokenizer = Tokenizer::new(event.point, parse_state);
                debug_assert!(!matches!(link.content, Content::Flow), "cannot use flow as subcontent yet");
                let mut state = State::Next(match link.content {
                    Content::Content => StateName::ContentDefinitionBefore,
                    Content::String => StateName::StringStart,
                    _ => StateName::TextStart,
                });

                if index > 2 && events[index - 1].kind == Kind::Enter && events[index - 1].name == Name::Paragraph {
                    let before = skip::opt_back(
                        events,
                        index - 2,
                        &[Name::BlankLineEnding, Name::Definition, Name::LineEnding, Name::SpaceOrTab],
                    );

                    if events[before].kind == Kind::Exit && events[before].name == Name::ListItemPrefix {
                        tokenizer.tokenize_state.document_at_first_paragraph_of_list_item = true;
                    }
                }

                while let Some(index) = link_index {
                    let enter = &events[index];
                    let link_curr = enter.link.as_ref().expect("expected link");
                    debug_assert_eq!(enter.kind, Kind::Enter);

                    if link_curr.previous().is_some() {
                        tokenizer.define_skip(enter.point);
                    }

                    let end = &events[index + 1].point;

                    state = tokenizer.push((enter.point.index, enter.point.vs), (end.index, end.vs), state);

                    link_index = link_curr.next();
                }

                let mut result = tokenizer.flush(state, true);
                value.gfm_footnote_definitions.append(&mut result.gfm_footnote_definitions);
                value.definitions.append(&mut result.definitions);
                value.done = false;

                acc = divide_events(&mut map, events, index, &mut tokenizer.events, acc);
            }
        }

        index += 1;
    }

    map.consume(events);

    value
}

pub fn divide_events(
    map: &mut EditMap,
    events: &[Event],
    mut link_index: usize,
    child_events: &mut Vec<Event>,
    acc_before: (usize, usize),
) -> (usize, usize) {
    let mut child_index = 0;
    let mut slices = vec![];
    let mut slice_start = 0;
    let mut old_prev: Option<usize> = None;
    let len = child_events.len();

    while child_index < len {
        let current = &child_events[child_index].point;
        let end = &events[link_index + 1].point;

        if current.index > end.index || (current.index == end.index && current.vs > end.vs) {
            slices.push((link_index, slice_start));
            slice_start = child_index;
            link_index = events[link_index].link.as_ref().unwrap().next().unwrap();
        }

        if let Some(sublink_curr) = &child_events[child_index].link
            && sublink_curr.previous().is_some()
        {
            let new_link = child_index + link_index - slices.len() * 2;
            let prev_event = &mut child_events[old_prev.unwrap()];
            prev_event.link.as_mut().unwrap().set_next(Some(new_link + acc_before.1 - acc_before.0));
        }

        if let Some(sublink_curr) = &child_events[child_index].link
            && let Some(next) = sublink_curr.next()
        {
            let sublink_next = child_events[next].link.as_mut().unwrap();

            old_prev = sublink_next.previous();

            sublink_next.set_previous(
                old_prev.map(|previous| previous + link_index - (slices.len() * 2) + acc_before.1 - acc_before.0),
            );
        }

        child_index += 1;
    }

    if !child_events.is_empty() {
        slices.push((link_index, slice_start));
    }

    let mut index = slices.len();

    while index > 0 {
        index -= 1;
        debug_assert!(slices[index].0 < events.len(), "expected slice start in bounds");
        let slice =
            if slices[index].1 == 0 { std::mem::take(child_events) } else { child_events.split_off(slices[index].1) };
        map.add(slices[index].0, 2, slice);
    }

    (acc_before.0 + (slices.len() * 2), acc_before.1 + len)
}
