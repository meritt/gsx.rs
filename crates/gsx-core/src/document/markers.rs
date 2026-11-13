use crate::document::{Marker, MarkerKind, Range};
use crate::source::LineIndex;
use crate::syntax::{Event, Kind, Name};

fn known_directive(events: &[Event], index: usize, bytes: &[u8], enclosing: bool) -> bool {
    let name = events[index..].iter().position(|event| event.name == Name::GsxDirectiveTextIdentifier).map(|offset| {
        &bytes[events[index + offset].point.index as usize..events[index + offset + 1].point.index as usize]
    });
    enclosing && name == Some(b"time".as_slice())
}

pub(super) fn collect(events: &[Event], bytes: &[u8], lines: &LineIndex) -> Vec<Marker> {
    let mut stack: Vec<&Name> = vec![];
    let mut open: Vec<usize> = vec![];
    let mut out: Vec<Marker> = vec![];
    let mut directives: Vec<bool> = vec![];
    for (index, event) in events.iter().enumerate() {
        if event.kind == Kind::Exit {
            if event.name == Name::GsxDirectiveText {
                directives.pop();
            }
            stack.pop();
            if let Some(marker) = open.pop().and_then(|at| out.get_mut(at)) {
                marker.range.end = lines.utf16(event.point.index as usize);
            }
            continue;
        }
        if event.name == Name::GsxDirectiveText {
            let enclosing = directives.last().copied().unwrap_or(true);
            directives.push(known_directive(events, index, bytes, enclosing));
        }
        let within = |name: &Name| stack.last().is_some_and(|last| *last == name);
        let kind = match event.name {
            Name::EmphasisSequence => Some(MarkerKind::Emphasis),
            Name::StrongSequence => Some(MarkerKind::Strong),
            Name::GfmStrikethroughSequence => Some(MarkerKind::Strikethrough),
            Name::GsxPairedSequence if within(&Name::GsxSpoiler) => Some(MarkerKind::Spoiler),
            Name::GsxPairedSequence if within(&Name::GsxUnderline) => Some(MarkerKind::Underline),
            Name::GsxPairedSequence if within(&Name::GsxHighlight) => Some(MarkerKind::Highlight),
            Name::CodeTextSequence => Some(MarkerKind::CodeText),
            Name::CodeFencedFenceSequence => Some(MarkerKind::CodeFence),
            Name::HeadingAtxSequence | Name::HeadingSetextUnderlineSequence => Some(MarkerKind::Heading),
            Name::ThematicBreakSequence => Some(MarkerKind::ThematicBreak),
            Name::BlockQuoteMarker => Some(MarkerKind::BlockQuote),
            Name::ListItemValue | Name::ListItemMarker => Some(MarkerKind::ListItem),
            Name::GfmTaskListItemCheck => Some(MarkerKind::TaskCheck),
            Name::GfmTableDelimiterRow => Some(MarkerKind::TableDelimiter),
            Name::GfmTableCellDivider if !stack.contains(&&Name::GfmTableDelimiterRow) => {
                Some(MarkerKind::TableDelimiter)
            }
            Name::LabelImageMarker
            | Name::LabelMarker
            | Name::ResourceMarker
            | Name::ResourceDestinationLiteralMarker
            | Name::ResourceTitleMarker
            | Name::ReferenceMarker => Some(MarkerKind::Link),
            Name::AutolinkMarker => Some(MarkerKind::Autolink),
            Name::CharacterEscapeMarker => Some(MarkerKind::Escape),
            Name::CharacterReference => Some(MarkerKind::CharacterReference),
            Name::HardBreakEscape | Name::HardBreakTrailing => Some(MarkerKind::HardBreak),
            Name::GsxDirectiveTextMarker
            | Name::GsxDirectiveTextLabelMarker
            | Name::GsxDirectiveTextAttributesMarker
                if directives.last() == Some(&true) =>
            {
                Some(MarkerKind::Directive)
            }
            Name::GsxMedia => Some(MarkerKind::Media),
            _ => None,
        };
        if let Some(kind) = kind {
            open.push(out.len());
            out.push(Marker { kind, range: Range { start: lines.utf16(event.point.index as usize), end: 0 } });
        } else {
            open.push(usize::MAX);
        }
        stack.push(&event.name);
    }
    crate::order::sort_by_key(&mut out, |marker| {
        crate::order::pair(marker.range.start as usize, marker.range.end as usize)
    });
    out
}
