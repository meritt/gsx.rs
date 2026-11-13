use crate::syntax::construct::partial_space_or_tab::space_or_tab_min_max;
use crate::syntax::event::{Kind, Name};
use crate::syntax::resolve::Name as ResolveName;
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::subtokenize::Subresult;
use crate::syntax::tokenizer::Tokenizer;
use crate::syntax::util::{
    constant::{LIST_ITEM_VALUE_SIZE_MAX, TAB_SIZE},
    skip,
    slice::{Position, Slice},
};

pub fn start(tokenizer: &mut Tokenizer) -> State {
    tokenizer.enter(Name::ListItem);

    if matches!(tokenizer.current, Some(b'\t' | b' ')) {
        tokenizer.attempt(State::Next(StateName::ListItemBefore), State::Nok);
        State::Retry(space_or_tab_min_max(tokenizer, 0, TAB_SIZE - 1))
    } else {
        State::Retry(StateName::ListItemBefore)
    }
}

pub fn before(tokenizer: &mut Tokenizer) -> State {
    if matches!(tokenizer.current, Some(b'*' | b'-')) {
        tokenizer.check(State::Nok, State::Next(StateName::ListItemBeforeUnordered));
        State::Retry(StateName::ThematicBreakStart)
    } else if tokenizer.current == Some(b'+') {
        State::Retry(StateName::ListItemBeforeUnordered)
    } else if matches!(tokenizer.current, Some(b'0'..=b'9')) {
        State::Retry(StateName::ListItemBeforeOrdered)
    } else {
        State::Nok
    }
}

pub fn before_unordered(tokenizer: &mut Tokenizer) -> State {
    tokenizer.enter(Name::ListItemPrefix);
    State::Retry(StateName::ListItemMarker)
}

pub fn before_ordered(tokenizer: &mut Tokenizer) -> State {
    tokenizer.enter(Name::ListItemPrefix);
    tokenizer.enter(Name::ListItemValue);
    State::Retry(StateName::ListItemValue)
}

pub fn value(tokenizer: &mut Tokenizer) -> State {
    if matches!(tokenizer.current, Some(b'.' | b')')) && (!tokenizer.interrupt || value_is_one(tokenizer)) {
        tokenizer.exit(Name::ListItemValue);
        State::Retry(StateName::ListItemMarker)
    } else if matches!(tokenizer.current, Some(b'0'..=b'9'))
        && tokenizer.tokenize_state.size + 1 < LIST_ITEM_VALUE_SIZE_MAX
    {
        tokenizer.tokenize_state.size += 1;
        tokenizer.consume();
        State::Next(StateName::ListItemValue)
    } else {
        tokenizer.tokenize_state.size = 0;
        State::Nok
    }
}

fn value_is_one(tokenizer: &Tokenizer) -> bool {
    let start = tokenizer.events[tokenizer.events.len() - 1].point.index as usize;
    let digits = &tokenizer.parse_state.bytes[start..tokenizer.point.index as usize];
    digits.last() == Some(&b'1') && digits[..digits.len() - 1].iter().all(|byte| *byte == b'0')
}

pub fn marker(tokenizer: &mut Tokenizer) -> State {
    tokenizer.enter(Name::ListItemMarker);
    tokenizer.consume();
    tokenizer.exit(Name::ListItemMarker);
    State::Next(StateName::ListItemMarkerAfter)
}

pub fn marker_after(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.size = 1;
    tokenizer.check(State::Next(StateName::ListItemAfter), State::Next(StateName::ListItemMarkerAfterFilled));
    State::Retry(StateName::BlankLineStart)
}

pub fn marker_after_filled(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.size = 0;

    tokenizer.attempt(State::Next(StateName::ListItemAfter), State::Next(StateName::ListItemPrefixOther));
    State::Retry(StateName::ListItemWhitespace)
}

pub fn whitespace(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(State::Next(StateName::ListItemWhitespaceAfter), State::Nok);
    State::Retry(space_or_tab_min_max(tokenizer, 1, TAB_SIZE))
}

pub fn whitespace_after(tokenizer: &mut Tokenizer) -> State {
    if let Some(b'\t' | b' ') = tokenizer.current { State::Nok } else { State::Ok }
}

pub fn prefix_other(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'\t' | b' ') => {
            tokenizer.enter(Name::SpaceOrTab);
            tokenizer.consume();
            tokenizer.exit(Name::SpaceOrTab);
            State::Next(StateName::ListItemAfter)
        }
        _ => State::Nok,
    }
}

pub fn after(tokenizer: &mut Tokenizer) -> State {
    let blank = tokenizer.tokenize_state.size == 1;
    tokenizer.tokenize_state.size = 0;

    if blank && tokenizer.interrupt {
        State::Nok
    } else {
        let start = skip::to_back(&tokenizer.events, tokenizer.events.len() - 1, &[Name::ListItem]);
        let mut prefix = Slice::from_position(
            tokenizer.parse_state.bytes,
            &Position { start: &tokenizer.events[start].point, end: &tokenizer.point },
        )
        .len();

        if blank {
            prefix += 1;
        }

        let container =
            &mut tokenizer.tokenize_state.document_container_stack[tokenizer.tokenize_state.document_continued];

        container.blank_initial = blank;
        container.size = prefix;

        tokenizer.exit(Name::ListItemPrefix);
        tokenizer.register_resolver_before(ResolveName::ListItem);
        State::Ok
    }
}

pub fn cont_start(tokenizer: &mut Tokenizer) -> State {
    tokenizer.check(State::Next(StateName::ListItemContBlank), State::Next(StateName::ListItemContFilled));
    State::Retry(StateName::BlankLineStart)
}

pub fn cont_blank(tokenizer: &mut Tokenizer) -> State {
    let container = &mut tokenizer.tokenize_state.document_container_stack[tokenizer.tokenize_state.document_continued];
    let size = container.size;

    if container.blank_initial {
        State::Nok
    } else if matches!(tokenizer.current, Some(b'\t' | b' ')) {
        State::Retry(space_or_tab_min_max(tokenizer, 0, size))
    } else {
        State::Ok
    }
}

pub fn cont_filled(tokenizer: &mut Tokenizer) -> State {
    let container = &mut tokenizer.tokenize_state.document_container_stack[tokenizer.tokenize_state.document_continued];
    let size = container.size;

    container.blank_initial = false;

    if matches!(tokenizer.current, Some(b'\t' | b' ')) {
        State::Retry(space_or_tab_min_max(tokenizer, size, size))
    } else {
        State::Nok
    }
}

pub fn resolve(tokenizer: &mut Tokenizer) -> Option<Subresult> {
    let mut lists_wip: Vec<(u8, usize, usize, usize)> = vec![];
    let mut lists: Vec<(u8, usize, usize, usize)> = vec![];
    let mut index = 0;
    let mut balance = 0;

    while index < tokenizer.events.len() {
        let event = &tokenizer.events[index];

        if event.name == Name::ListItem {
            if event.kind == Kind::Enter {
                let mut end = index + 1;
                let mut depth = 1;
                while depth > 0 {
                    if tokenizer.events[end].name == Name::ListItem {
                        depth = if tokenizer.events[end].kind == Kind::Enter { depth + 1 } else { depth - 1 };
                    }
                    end += 1;
                }
                let end = end - 1;
                let marker = skip::to(&tokenizer.events, index, &[Name::ListItemMarker]);
                let marker = tokenizer.parse_state.bytes[tokenizer.events[marker].point.index as usize];
                let current = (marker, balance, index, end);

                let mut list_index = lists_wip.len();
                let mut matched = false;

                while list_index > 0 {
                    list_index -= 1;
                    let previous = &lists_wip[list_index];
                    let before = skip::opt(
                        &tokenizer.events,
                        previous.3 + 1,
                        &[Name::SpaceOrTab, Name::LineEnding, Name::BlankLineEnding, Name::BlockQuotePrefix],
                    );

                    if previous.0 == current.0 && previous.1 == current.1 && before == current.2 {
                        let previous_mut = &mut lists_wip[list_index];
                        previous_mut.3 = current.3;
                        lists.append(&mut lists_wip.split_off(list_index + 1));
                        matched = true;
                        break;
                    }
                }

                if !matched {
                    let mut index = lists_wip.len();
                    let mut exit = None;

                    while index > 0 {
                        index -= 1;

                        if current.2 > lists_wip[index].3 {
                            exit = Some(index);
                        } else {
                            break;
                        }
                    }

                    if let Some(exit) = exit {
                        lists.append(&mut lists_wip.split_off(exit));
                    }

                    lists_wip.push(current);
                }

                balance += 1;
            } else {
                balance -= 1;
            }
        }

        index += 1;
    }

    lists.append(&mut lists_wip);

    let mut index = 0;
    while index < lists.len() {
        let list_item = &lists[index];
        let mut list_start = tokenizer.events[list_item.2];
        let mut list_end = tokenizer.events[list_item.3];
        let name = match list_item.0 {
            b'.' | b')' => Name::ListOrdered,
            _ => Name::ListUnordered,
        };
        list_start.name = name;
        list_end.name = name;

        tokenizer.map.add(list_item.2, 0, vec![list_start]);
        tokenizer.map.add(list_item.3 + 1, 0, vec![list_end]);

        index += 1;
    }

    tokenizer.map.consume(&mut tokenizer.events);
    None
}
