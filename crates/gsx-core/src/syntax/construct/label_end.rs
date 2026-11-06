use crate::source::Limit;
use crate::syntax::construct::partial_space_or_tab_eol::space_or_tab_eol;
use crate::syntax::event::{Event, Kind, Name};
use crate::syntax::resolve::Name as ResolveName;
use crate::syntax::state::{Name as StateName, State};
use crate::syntax::subtokenize::Subresult;
use crate::syntax::tokenizer::{Label, LabelKind, LabelStart, Tokenizer};
use crate::syntax::util::{
    constant::RESOURCE_DESTINATION_BALANCE_MAX,
    normalize_identifier::normalize_identifier,
    skip,
    slice::{Position, Slice},
};

pub fn start(tokenizer: &mut Tokenizer) -> State {
    let index = tokenizer.point.index;
    if Some(b']') == tokenizer.current
        && tokenizer.spend(Limit::BlockLabelEnds, index, |_| index + 1)
        && !tokenizer.tokenize_state.label_starts.is_empty()
    {
        let label_start = tokenizer.tokenize_state.label_starts.last().unwrap();

        tokenizer.tokenize_state.end = tokenizer.events.len();

        if label_start.inactive {
            return State::Retry(StateName::LabelEndNok);
        }

        tokenizer.enter(Name::LabelEnd);
        tokenizer.enter(Name::LabelMarker);
        tokenizer.consume();
        tokenizer.exit(Name::LabelMarker);
        tokenizer.exit(Name::LabelEnd);
        return State::Next(StateName::LabelEndAfter);
    }

    State::Nok
}

pub fn after(tokenizer: &mut Tokenizer) -> State {
    let start_index = tokenizer.tokenize_state.label_starts.len() - 1;
    let start = &tokenizer.tokenize_state.label_starts[start_index];

    let indices =
        (tokenizer.events[start.start.1].point.index, tokenizer.events[tokenizer.tokenize_state.end].point.index);

    let mut id = normalize_identifier(Slice::from_indices(tokenizer.parse_state.bytes, indices.0, indices.1).as_str());

    if start.kind == LabelKind::GfmFootnote {
        if tokenizer.parse_state.gfm_footnote_definitions.contains(&id) {
            return State::Retry(StateName::LabelEndOk);
        }

        tokenizer.tokenize_state.label_starts[start_index].kind = LabelKind::GfmUndefinedFootnote;
        let mut new_id = String::new();
        new_id.push('^');
        new_id.push_str(&id);
        id = new_id;
    }

    let defined = tokenizer.parse_state.definitions.contains(&id);

    match tokenizer.current {
        Some(b'(') => {
            tokenizer.attempt(
                State::Next(StateName::LabelEndOk),
                State::Next(if defined { StateName::LabelEndOk } else { StateName::LabelEndNok }),
            );
            State::Retry(StateName::LabelEndResourceStart)
        }
        Some(b'[') => {
            tokenizer.attempt(
                State::Next(StateName::LabelEndOk),
                State::Next(if defined { StateName::LabelEndReferenceNotFull } else { StateName::LabelEndNok }),
            );
            State::Retry(StateName::LabelEndReferenceFull)
        }
        _ => State::Retry(if defined { StateName::LabelEndOk } else { StateName::LabelEndNok }),
    }
}

pub fn reference_not_full(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(State::Next(StateName::LabelEndOk), State::Next(StateName::LabelEndNok));
    State::Retry(StateName::LabelEndReferenceCollapsed)
}

pub fn ok(tokenizer: &mut Tokenizer) -> State {
    let label_start = tokenizer.tokenize_state.label_starts.pop().unwrap();

    if label_start.kind != LabelKind::Image {
        let mut index = 0;
        while index < tokenizer.tokenize_state.label_starts.len() {
            let label_start = &mut tokenizer.tokenize_state.label_starts[index];
            if label_start.kind != LabelKind::Image {
                label_start.inactive = true;
            }
            index += 1;
        }
    }

    tokenizer.tokenize_state.labels.push(Label {
        kind: label_start.kind,
        start: label_start.start,
        end: (tokenizer.tokenize_state.end, tokenizer.events.len() - 1),
    });
    tokenizer.tokenize_state.end = 0;
    tokenizer.register_resolver_before(ResolveName::Label);
    State::Ok
}

pub fn nok(tokenizer: &mut Tokenizer) -> State {
    let start = tokenizer.tokenize_state.label_starts.pop().unwrap();
    tokenizer.tokenize_state.label_starts_loose.push(start);
    tokenizer.tokenize_state.end = 0;
    State::Nok
}

pub fn resource_start(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'(') => {
            tokenizer.enter(Name::Resource);
            tokenizer.enter(Name::ResourceMarker);
            tokenizer.consume();
            tokenizer.exit(Name::ResourceMarker);
            State::Next(StateName::LabelEndResourceBefore)
        }
        _ => unreachable!("expected `(`"),
    }
}

pub fn resource_before(tokenizer: &mut Tokenizer) -> State {
    if matches!(tokenizer.current, Some(b'\t' | b'\n' | b' ')) {
        tokenizer.attempt(State::Next(StateName::LabelEndResourceOpen), State::Next(StateName::LabelEndResourceOpen));
        State::Retry(space_or_tab_eol(tokenizer))
    } else {
        State::Retry(StateName::LabelEndResourceOpen)
    }
}

pub fn resource_open(tokenizer: &mut Tokenizer) -> State {
    if let Some(b')') = tokenizer.current {
        State::Retry(StateName::LabelEndResourceEnd)
    } else {
        tokenizer.tokenize_state.token_1 = Name::ResourceDestination;
        tokenizer.tokenize_state.token_2 = Name::ResourceDestinationLiteral;
        tokenizer.tokenize_state.token_3 = Name::ResourceDestinationLiteralMarker;
        tokenizer.tokenize_state.token_4 = Name::ResourceDestinationRaw;
        tokenizer.tokenize_state.token_5 = Name::ResourceDestinationString;
        tokenizer.tokenize_state.size_b = RESOURCE_DESTINATION_BALANCE_MAX;

        tokenizer.attempt(
            State::Next(StateName::LabelEndResourceDestinationAfter),
            State::Next(StateName::LabelEndResourceDestinationMissing),
        );
        State::Retry(StateName::DestinationStart)
    }
}

pub fn resource_destination_after(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.token_1 = Name::Data;
    tokenizer.tokenize_state.token_2 = Name::Data;
    tokenizer.tokenize_state.token_3 = Name::Data;
    tokenizer.tokenize_state.token_4 = Name::Data;
    tokenizer.tokenize_state.token_5 = Name::Data;
    tokenizer.tokenize_state.size_b = 0;

    if matches!(tokenizer.current, Some(b'\t' | b'\n' | b' ')) {
        tokenizer.attempt(State::Next(StateName::LabelEndResourceBetween), State::Next(StateName::LabelEndResourceEnd));
        State::Retry(space_or_tab_eol(tokenizer))
    } else {
        State::Retry(StateName::LabelEndResourceEnd)
    }
}

pub fn resource_destination_missing(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.token_1 = Name::Data;
    tokenizer.tokenize_state.token_2 = Name::Data;
    tokenizer.tokenize_state.token_3 = Name::Data;
    tokenizer.tokenize_state.token_4 = Name::Data;
    tokenizer.tokenize_state.token_5 = Name::Data;
    tokenizer.tokenize_state.size_b = 0;
    State::Nok
}

pub fn resource_between(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'"' | b'\'' | b'(') => {
            tokenizer.tokenize_state.token_1 = Name::ResourceTitle;
            tokenizer.tokenize_state.token_2 = Name::ResourceTitleMarker;
            tokenizer.tokenize_state.token_3 = Name::ResourceTitleString;
            tokenizer.attempt(State::Next(StateName::LabelEndResourceTitleAfter), State::Nok);
            State::Retry(StateName::TitleStart)
        }
        _ => State::Retry(StateName::LabelEndResourceEnd),
    }
}

pub fn resource_title_after(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.token_1 = Name::Data;
    tokenizer.tokenize_state.token_2 = Name::Data;
    tokenizer.tokenize_state.token_3 = Name::Data;

    if matches!(tokenizer.current, Some(b'\t' | b'\n' | b' ')) {
        tokenizer.attempt(State::Next(StateName::LabelEndResourceEnd), State::Next(StateName::LabelEndResourceEnd));
        State::Retry(space_or_tab_eol(tokenizer))
    } else {
        State::Retry(StateName::LabelEndResourceEnd)
    }
}

pub fn resource_end(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b')') => {
            tokenizer.enter(Name::ResourceMarker);
            tokenizer.consume();
            tokenizer.exit(Name::ResourceMarker);
            tokenizer.exit(Name::Resource);
            State::Ok
        }
        _ => State::Nok,
    }
}

pub fn reference_full(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b'[') => {
            tokenizer.tokenize_state.token_1 = Name::Reference;
            tokenizer.tokenize_state.token_2 = Name::ReferenceMarker;
            tokenizer.tokenize_state.token_3 = Name::ReferenceString;
            tokenizer.attempt(
                State::Next(StateName::LabelEndReferenceFullAfter),
                State::Next(StateName::LabelEndReferenceFullMissing),
            );
            State::Retry(StateName::LabelStart)
        }
        _ => unreachable!("expected `[`"),
    }
}

pub fn reference_full_after(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.token_1 = Name::Data;
    tokenizer.tokenize_state.token_2 = Name::Data;
    tokenizer.tokenize_state.token_3 = Name::Data;

    if tokenizer.parse_state.definitions.contains(&normalize_identifier(
        Slice::from_position(
            tokenizer.parse_state.bytes,
            &Position::from_exit_event(
                &tokenizer.events,
                skip::to_back(&tokenizer.events, tokenizer.events.len() - 1, &[Name::ReferenceString]),
            ),
        )
        .as_str(),
    )) {
        State::Ok
    } else {
        State::Nok
    }
}

pub fn reference_full_missing(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.token_1 = Name::Data;
    tokenizer.tokenize_state.token_2 = Name::Data;
    tokenizer.tokenize_state.token_3 = Name::Data;
    State::Nok
}

pub fn reference_collapsed(tokenizer: &mut Tokenizer) -> State {
    debug_assert_eq!(tokenizer.current, Some(b'['), "expected opening bracket");
    tokenizer.enter(Name::Reference);
    tokenizer.enter(Name::ReferenceMarker);
    tokenizer.consume();
    tokenizer.exit(Name::ReferenceMarker);
    State::Next(StateName::LabelEndReferenceCollapsedOpen)
}

pub fn reference_collapsed_open(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(b']') => {
            tokenizer.enter(Name::ReferenceMarker);
            tokenizer.consume();
            tokenizer.exit(Name::ReferenceMarker);
            tokenizer.exit(Name::Reference);
            State::Ok
        }
        _ => State::Nok,
    }
}

pub fn resolve(tokenizer: &mut Tokenizer) -> Option<Subresult> {
    let labels = tokenizer.tokenize_state.labels.split_off(0);
    inject_labels(tokenizer, &labels);
    let starts = tokenizer.tokenize_state.label_starts.split_off(0);
    mark_as_data(tokenizer, &starts);
    let starts = tokenizer.tokenize_state.label_starts_loose.split_off(0);
    mark_as_data(tokenizer, &starts);

    tokenizer.map.consume(&mut tokenizer.events);
    None
}

fn inject_labels(tokenizer: &mut Tokenizer, labels: &[Label]) {
    let mut index = 0;
    while index < labels.len() {
        let label = &labels[index];
        let group_name = if label.kind == LabelKind::GfmFootnote {
            Name::GfmFootnoteCall
        } else if label.kind == LabelKind::Image {
            Name::Image
        } else {
            Name::Link
        };

        let mut caret = vec![];

        if label.kind == LabelKind::GfmUndefinedFootnote {
            caret.push(Event {
                kind: Kind::Enter,
                name: Name::Data,
                point: tokenizer.events[label.start.1 - 2].point,
                link: None,
            });
            caret.push(Event {
                kind: Kind::Exit,
                name: Name::Data,
                point: tokenizer.events[label.start.1 - 1].point,
                link: None,
            });
            tokenizer.events[label.start.0].name = Name::LabelLink;
            tokenizer.events[label.start.1].name = Name::LabelLink;
            tokenizer.events[label.start.1].point = caret[0].point;
            tokenizer.map.add(label.start.1 - 2, 2, vec![]);
        }

        tokenizer.map.add(
            label.start.0,
            0,
            vec![
                Event { kind: Kind::Enter, name: group_name, point: tokenizer.events[label.start.0].point, link: None },
                Event {
                    kind: Kind::Enter,
                    name: Name::Label,
                    point: tokenizer.events[label.start.0].point,
                    link: None,
                },
            ],
        );

        if label.start.1 != label.end.0 || !caret.is_empty() {
            tokenizer.map.add_before(
                label.start.1 + 1,
                0,
                vec![Event {
                    kind: Kind::Enter,
                    name: Name::LabelText,
                    point: tokenizer.events[label.start.1].point,
                    link: None,
                }],
            );
            tokenizer.map.add(
                label.end.0,
                0,
                vec![Event {
                    kind: Kind::Exit,
                    name: Name::LabelText,
                    point: tokenizer.events[label.end.0].point,
                    link: None,
                }],
            );
        }

        if !caret.is_empty() {
            tokenizer.map.add(label.start.1 + 1, 0, caret);
        }

        tokenizer.map.add(
            label.end.0 + 4,
            0,
            vec![Event {
                kind: Kind::Exit,
                name: Name::Label,
                point: tokenizer.events[label.end.0 + 3].point,
                link: None,
            }],
        );

        tokenizer.map.add(
            label.end.1 + 1,
            0,
            vec![Event { kind: Kind::Exit, name: group_name, point: tokenizer.events[label.end.1].point, link: None }],
        );

        index += 1;
    }
}

fn mark_as_data(tokenizer: &mut Tokenizer, events: &[LabelStart]) {
    let mut index = 0;

    while index < events.len() {
        let data_enter_index = events[index].start.0;
        let data_exit_index = events[index].start.1;

        tokenizer.map.add(
            data_enter_index,
            data_exit_index - data_enter_index + 1,
            vec![
                Event {
                    kind: Kind::Enter,
                    name: Name::Data,
                    point: tokenizer.events[data_enter_index].point,
                    link: None,
                },
                Event {
                    kind: Kind::Exit,
                    name: Name::Data,
                    point: tokenizer.events[data_exit_index].point,
                    link: None,
                },
            ],
        );

        index += 1;
    }
}
