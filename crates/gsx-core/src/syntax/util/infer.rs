use crate::syntax::Align as AlignKind;
use crate::syntax::event::{Event, Kind, Name};

pub fn list_loose(events: &[Event], mut index: usize, include_items: bool) -> bool {
    let mut balance = 0;
    let name = &events[index].name;
    debug_assert!(matches!(name, Name::ListOrdered | Name::ListUnordered), "expected list");

    while index < events.len() {
        let event = &events[index];

        if event.kind == Kind::Enter {
            balance += 1;

            if include_items && balance == 2 && event.name == Name::ListItem && list_item_loose(events, index) {
                return true;
            }
        } else {
            balance -= 1;

            if balance == 1 && event.name == Name::BlankLineEnding && !empty_container_line(events, index) {
                return true;
            }

            if balance == 0 && event.name == *name {
                break;
            }
        }

        index += 1;
    }

    false
}

fn empty_container_line(events: &[Event], index: usize) -> bool {
    let mut before = index - 2;
    let mut inner = None;

    while events[before].kind == Kind::Exit
        && matches!(events[before].name, Name::ListItem | Name::ListOrdered | Name::ListUnordered | Name::BlockQuote)
    {
        inner = Some(&events[before].name);
        before -= 1;
    }

    if events[before].name == Name::SpaceOrTab {
        before -= 2;
    }

    match events[before].name {
        Name::ListItemPrefix => matches!(inner, None | Some(Name::ListItem)),
        Name::BlockQuotePrefix => inner == Some(&Name::BlockQuote),
        _ => false,
    }
}

pub fn list_item_loose(events: &[Event], mut index: usize) -> bool {
    debug_assert!(matches!(events[index].name, Name::ListItem), "expected list item");
    let mut balance = 0;
    let mut blank = false;

    while index < events.len() {
        let event = &events[index];

        if event.kind == Kind::Enter {
            balance += 1;

            if blank
                && balance == 2
                && !matches!(
                    event.name,
                    Name::BlankLineEnding
                        | Name::LineEnding
                        | Name::SpaceOrTab
                        | Name::BlockQuotePrefix
                        | Name::ListItemPrefix
                )
            {
                return true;
            }
        } else {
            balance -= 1;

            if balance == 1 && event.name == Name::BlankLineEnding && !empty_container_line(events, index) {
                blank = true;
            }

            if balance == 0 && event.name == Name::ListItem {
                break;
            }
        }

        index += 1;
    }

    false
}

pub fn gfm_table_align(events: &[Event], mut index: usize) -> Vec<AlignKind> {
    debug_assert!(matches!(events[index].name, Name::GfmTable), "expected table");
    let mut in_delimiter_row = false;
    let mut align = vec![];

    while index < events.len() {
        let event = &events[index];

        if in_delimiter_row {
            if event.kind == Kind::Enter {
                if event.name == Name::GfmTableDelimiterCellValue {
                    align.push(if events[index + 1].name == Name::GfmTableDelimiterMarker {
                        AlignKind::Left
                    } else {
                        AlignKind::None
                    });
                }
            } else if event.name == Name::GfmTableDelimiterCellValue {
                if events[index - 1].name == Name::GfmTableDelimiterMarker {
                    let align_index = align.len() - 1;
                    align[align_index] =
                        if align[align_index] == AlignKind::Left { AlignKind::Center } else { AlignKind::Right }
                }
            } else if event.name == Name::GfmTableDelimiterRow {
                break;
            }
        } else if event.kind == Kind::Enter && event.name == Name::GfmTableDelimiterRow {
            in_delimiter_row = true;
        }

        index += 1;
    }

    align
}
