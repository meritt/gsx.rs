use crate::source::Limit;
use crate::syntax::event::{Content, Event, Kind, Link, Name};
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::subtokenize::divide_events;
use crate::syntax::tokenizer::{Container, ContainerState, Tokenizer};
use crate::syntax::util::skip;

#[derive(Debug, PartialEq)]
enum Phase {
    After,
    Prefix,
    Eof,
}

pub fn start(tokenizer: &mut Tokenizer) -> State {
    let child = Tokenizer::new(tokenizer.point, tokenizer.parse_state);
    tokenizer.tokenize_state.document_child = Some(Box::new(child));

    tokenizer.attempt(
        State::Next(StateName::DocumentContainerNewBefore),
        State::Next(StateName::DocumentContainerNewBefore),
    );

    State::Retry(StateName::BomStart)
}

pub fn container_existing_before(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.tokenize_state.document_continued < tokenizer.tokenize_state.document_container_stack.len() {
        let container = &tokenizer.tokenize_state.document_container_stack[tokenizer.tokenize_state.document_continued];

        let name = match container.kind {
            Container::BlockQuote => StateName::BlockQuoteContStart,
            Container::GfmFootnoteDefinition => StateName::GfmFootnoteDefinitionContStart,
            Container::ListItem => StateName::ListItemContStart,
        };

        tokenizer.attempt(
            State::Next(StateName::DocumentContainerExistingAfter),
            State::Next(StateName::DocumentContainerNewBefore),
        );

        State::Retry(name)
    } else {
        State::Retry(StateName::DocumentContainerNewBefore)
    }
}

pub fn container_existing_after(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.document_continued += 1;
    State::Retry(StateName::DocumentContainerExistingBefore)
}

pub fn container_new_before(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.tokenize_state.document_continued > Limit::ContainerDepth.maximum() as usize {
        return State::Retry(StateName::DocumentContainersAfter);
    }
    if tokenizer.tokenize_state.document_continued == tokenizer.tokenize_state.document_container_stack.len() {
        let child = tokenizer.tokenize_state.document_child.as_ref().unwrap();

        tokenizer.interrupt = child.interrupt && !child.pierce;

        if child.concrete {
            return State::Retry(StateName::DocumentContainersAfter);
        }
    }

    let tail = tokenizer.tokenize_state.document_container_stack.len();
    tokenizer.tokenize_state.document_container_stack.push(ContainerState {
        kind: Container::BlockQuote,
        blank_initial: false,
        size: 0,
    });
    tokenizer.tokenize_state.document_container_stack.swap(tokenizer.tokenize_state.document_continued, tail);

    tokenizer.attempt(
        State::Next(StateName::DocumentContainerNewAfter),
        State::Next(StateName::DocumentContainerNewBeforeNotBlockQuote),
    );
    State::Retry(StateName::BlockQuoteStart)
}

pub fn container_new_before_not_block_quote(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.document_container_stack[tokenizer.tokenize_state.document_continued] =
        ContainerState { kind: Container::ListItem, blank_initial: false, size: 0 };

    tokenizer.attempt(
        State::Next(StateName::DocumentContainerNewAfter),
        State::Next(StateName::DocumentContainerNewBeforeNotList),
    );
    State::Retry(StateName::ListItemStart)
}

pub fn container_new_before_not_list(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.document_container_stack[tokenizer.tokenize_state.document_continued] =
        ContainerState { kind: Container::GfmFootnoteDefinition, blank_initial: false, size: 0 };

    tokenizer.attempt(
        State::Next(StateName::DocumentContainerNewAfter),
        State::Next(StateName::DocumentContainerNewBeforeNotGfmFootnoteDefinition),
    );
    State::Retry(StateName::GfmFootnoteDefinitionStart)
}

pub fn container_new_before_not_footnote_definition(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.document_container_stack.swap_remove(tokenizer.tokenize_state.document_continued);

    State::Retry(StateName::DocumentContainersAfter)
}

pub fn container_new_after(tokenizer: &mut Tokenizer) -> State {
    let container =
        tokenizer.tokenize_state.document_container_stack.swap_remove(tokenizer.tokenize_state.document_continued);

    if tokenizer.tokenize_state.document_continued != tokenizer.tokenize_state.document_container_stack.len() {
        exit_containers(tokenizer, &Phase::Prefix);
    }

    tokenizer.tokenize_state.document_child.as_mut().unwrap().pierce = true;

    tokenizer.tokenize_state.document_container_stack.push(container);
    tokenizer.tokenize_state.document_continued += 1;
    if tokenizer.tokenize_state.document_continued > Limit::ContainerDepth.maximum() as usize {
        let prefix = tokenizer
            .events
            .iter()
            .rposition(|event| {
                event.kind == Kind::Enter
                    && matches!(
                        event.name,
                        Name::BlockQuotePrefix | Name::ListItemPrefix | Name::GfmFootnoteDefinitionPrefix
                    )
            })
            .map_or(tokenizer.point.index, |at| tokenizer.events[at].point.index);
        tokenizer.parse_state.exceed(Limit::ContainerDepth, prefix, tokenizer.point.index);
    }
    tokenizer.interrupt = false;
    State::Retry(StateName::DocumentContainerNewBefore)
}

pub fn containers_after(tokenizer: &mut Tokenizer) -> State {
    let child = tokenizer.tokenize_state.document_child.as_mut().unwrap();

    child.lazy = tokenizer.tokenize_state.document_continued != tokenizer.tokenize_state.document_container_stack.len();
    child.define_skip(tokenizer.point);

    if tokenizer.current.is_none() {
        State::Retry(StateName::DocumentFlowEnd)
    } else {
        let current = tokenizer.events.len();
        let previous = tokenizer.tokenize_state.document_data_index;
        if let Some(previous) = previous {
            tokenizer.events[previous].link.as_mut().unwrap().set_next(Some(current));
        }
        tokenizer.tokenize_state.document_data_index = Some(current);
        let mut link = Link::new(Content::Flow);
        link.set_previous(previous);
        tokenizer.enter_link(Name::Data, link);
        State::Retry(StateName::DocumentFlowInside)
    }
}

pub fn flow_inside(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None => {
            tokenizer.exit(Name::Data);
            State::Retry(StateName::DocumentFlowEnd)
        }
        Some(b'\n') => {
            tokenizer.consume();
            tokenizer.exit(Name::Data);
            State::Next(StateName::DocumentFlowEnd)
        }
        Some(_) => {
            tokenizer.consume_while(|_| true);
            State::Next(StateName::DocumentFlowInside)
        }
    }
}

pub fn flow_end(tokenizer: &mut Tokenizer) -> State {
    let child = tokenizer.tokenize_state.document_child.as_mut().unwrap();
    let state = tokenizer.tokenize_state.document_child_state.take().unwrap_or(State::Next(StateName::FlowStart));

    tokenizer.tokenize_state.document_exits.push(None);

    let mut state = child.push((child.point.index, child.point.vs), (tokenizer.point.index, tokenizer.point.vs), state);

    if tokenizer.current.is_none() {
        child.flush(state, false);
        state = State::Ok;
    }

    tokenizer.tokenize_state.document_child_state = Some(state);

    let mut document_lazy_continuation_current = false;
    let mut stack_index = child.stack.len();

    while !document_lazy_continuation_current && stack_index > 0 {
        stack_index -= 1;
        let name = &child.stack[stack_index];
        if name == &Name::Content || name == &Name::GfmTableHead {
            document_lazy_continuation_current = true;
        }
    }

    if !document_lazy_continuation_current && !child.events.is_empty() {
        let before = skip::opt_back(&child.events, child.events.len() - 1, &[Name::LineEnding]);
        let name = &child.events[before].name;
        if name == &Name::Content || name == &Name::HeadingSetextUnderline {
            document_lazy_continuation_current = true;
        }
    }

    child.pierce = false;

    if child.lazy && tokenizer.tokenize_state.document_lazy_accepting_before && document_lazy_continuation_current {
        tokenizer.tokenize_state.document_continued = tokenizer.tokenize_state.document_container_stack.len();
    }

    if tokenizer.tokenize_state.document_continued != tokenizer.tokenize_state.document_container_stack.len() {
        exit_containers(tokenizer, &Phase::After);
    }

    if tokenizer.current.is_none() {
        tokenizer.tokenize_state.document_continued = 0;
        exit_containers(tokenizer, &Phase::Eof);
        resolve(tokenizer);
        State::Ok
    } else {
        tokenizer.tokenize_state.document_continued = 0;
        tokenizer.tokenize_state.document_lazy_accepting_before = document_lazy_continuation_current;
        tokenizer.interrupt = false;
        State::Retry(StateName::DocumentContainerExistingBefore)
    }
}

fn exit_containers(tokenizer: &mut Tokenizer, phase: &Phase) {
    let mut stack_close =
        tokenizer.tokenize_state.document_container_stack.split_off(tokenizer.tokenize_state.document_continued);

    let child = tokenizer.tokenize_state.document_child.as_mut().unwrap();

    if *phase != Phase::After {
        let state = tokenizer.tokenize_state.document_child_state.take().unwrap_or(State::Next(StateName::FlowStart));

        child.flush(state, false);
    }

    if !stack_close.is_empty() {
        let index = tokenizer.tokenize_state.document_exits.len() - (if *phase == Phase::After { 2 } else { 1 });
        let mut exits = Vec::with_capacity(stack_close.len());

        while let Some(container) = stack_close.pop() {
            let name = match container.kind {
                Container::BlockQuote => Name::BlockQuote,
                Container::GfmFootnoteDefinition => Name::GfmFootnoteDefinition,
                Container::ListItem => Name::ListItem,
            };

            exits.push(Event { kind: Kind::Exit, name, point: tokenizer.point, link: None });

            let mut stack_index = tokenizer.stack.len();
            let mut found = false;

            while stack_index > 0 {
                stack_index -= 1;

                if tokenizer.stack[stack_index] == name {
                    tokenizer.stack.remove(stack_index);
                    found = true;
                    break;
                }
            }

            debug_assert!(found, "expected to find container event to exit");
        }

        debug_assert!(tokenizer.tokenize_state.document_exits[index].is_none(), "expected no exits yet");
        tokenizer.tokenize_state.document_exits[index] = Some(exits);
    }

    child.interrupt = false;
}

fn resolve(tokenizer: &mut Tokenizer) {
    let child = tokenizer.tokenize_state.document_child.as_mut().unwrap();

    let mut child_index = 0;
    let mut line = 0;

    while child_index < child.events.len() {
        if child.events[child_index].kind == Kind::Exit
            && matches!(child.events[child_index].name, Name::LineEnding | Name::BlankLineEnding)
        {
            let mut inject_index = child_index - 1;
            let mut point = &child.events[inject_index].point;

            while child_index + 1 < child.events.len() && child.events[child_index + 1].kind == Kind::Exit {
                child_index += 1;
                point = &child.events[child_index].point;
                inject_index = child_index + 1;
            }

            if line < tokenizer.tokenize_state.document_exits.len()
                && let Some(mut exits) = tokenizer.tokenize_state.document_exits[line].take()
            {
                let mut exit_index = 0;
                while exit_index < exits.len() {
                    exits[exit_index].point = *point;
                    exit_index += 1;
                }

                child.map.add(inject_index, 0, exits);
            }

            line += 1;
        }

        child_index += 1;
    }

    child.map.consume(&mut child.events);

    let mut flow_index = skip::to(&tokenizer.events, 0, &[Name::Data]);
    while flow_index < tokenizer.events.len()
        && (tokenizer.events[flow_index].link.is_none()
            || tokenizer.events[flow_index].link.as_ref().unwrap().content != Content::Flow)
    {
        flow_index = skip::to(&tokenizer.events, flow_index + 1, &[Name::Data]);
    }

    divide_events(&mut tokenizer.map, &tokenizer.events, flow_index, &mut child.events, (0, 0));

    tokenizer.map.consume(&mut tokenizer.events);

    if line < tokenizer.tokenize_state.document_exits.len()
        && let Some(mut exits) = tokenizer.tokenize_state.document_exits[line].take()
    {
        let mut exit_index = 0;
        while exit_index < exits.len() {
            exits[exit_index].point = tokenizer.point;
            exit_index += 1;
        }

        tokenizer.events.append(&mut exits);
    }

    tokenizer.resolvers.append(&mut child.resolvers);

    tokenizer.tokenize_state.definitions.append(&mut child.tokenize_state.definitions);
}
