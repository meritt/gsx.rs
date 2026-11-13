use crate::source::Limit;
use crate::syntax::event::{Event, Kind, Name};
use crate::syntax::parser::ParseState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Exceeded {
    pub limit: Limit,
    pub start: usize,
    pub end: usize,
}

pub fn lines(bytes: &[u8]) -> Option<Exceeded> {
    let maximum = Limit::DocumentLines.maximum() as usize;
    let start = bytes.iter().enumerate().filter(|(_, byte)| **byte == b'\n').nth(maximum - 1)?.0 + 1;
    let end = bytes[start..].iter().position(|byte| *byte == b'\n').map_or(bytes.len(), |at| start + at);
    Some(Exceeded { limit: Limit::DocumentLines, start, end })
}

fn counted(name: Name) -> Option<Limit> {
    match name {
        Name::CharacterEscape => Some(Limit::DocumentEscapes),
        Name::HardBreakEscape | Name::HardBreakTrailing => Some(Limit::DocumentHardBreaks),
        Name::ListItemPrefix => Some(Limit::DocumentListItems),
        Name::GsxMath => Some(Limit::DocumentMathLiterals),
        Name::GsxSpoiler | Name::GsxUnderline | Name::GsxHighlight => Some(Limit::DocumentPairedDelimiters),
        Name::GfmTableRow => Some(Limit::DocumentTableCells),
        _ => None,
    }
}

fn exit(events: &[Event], enter: usize) -> usize {
    let mut depth = 0_usize;
    for (index, event) in events.iter().enumerate().skip(enter) {
        if event.kind == Kind::Enter {
            depth += 1;
        } else {
            depth -= 1;
            if depth == 0 {
                return index;
            }
        }
    }
    events.len() - 1
}

fn inline(name: Name) -> bool {
    matches!(
        name,
        Name::Emphasis
            | Name::Strong
            | Name::GfmStrikethrough
            | Name::GsxUnderline
            | Name::GsxHighlight
            | Name::GsxSpoiler
            | Name::GsxDirectiveText
            | Name::Link
            | Name::Image
    )
}

pub fn count(events: &[Event], state: &ParseState) {
    let mut counts = [0_u32; Limit::ALL.len()];
    let mut columns = 0;
    let mut depth = 0_u32;
    for (index, event) in events.iter().enumerate() {
        if inline(event.name) {
            if event.kind == Kind::Exit {
                depth -= 1;
                continue;
            }
            depth += 1;
            if depth == Limit::InlineDepth.maximum() + 1 {
                let end = events[exit(events, index)].point.index;
                state.exceed(Limit::InlineDepth, event.point.index as usize, end as usize);
            }
        }
        if event.kind != Kind::Enter {
            continue;
        }
        if event.name == Name::GfmTable {
            columns = 0;
        }
        let Some(limit) = counted(event.name) else {
            continue;
        };
        let count = &mut counts[limit as usize];
        if *count > limit.maximum() {
            continue;
        }
        let mut range = index;
        if limit == Limit::DocumentTableCells {
            let end = exit(events, index);
            let cells = events[index..end]
                .iter()
                .filter(|event| event.kind == Kind::Enter && event.name == Name::GfmTableCell)
                .count();
            if columns == 0 {
                columns = cells;
            }
            *count = count.saturating_add(u32::try_from(cells.max(columns)).unwrap_or(u32::MAX));
        } else {
            *count += 1;
            if limit == Limit::DocumentPairedDelimiters {
                range = index + 1;
            }
        }
        if *count > limit.maximum() {
            let end = events[exit(events, range)].point.index;
            state.exceed(limit, events[range].point.index as usize, end as usize);
        }
    }
}
