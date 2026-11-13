use crate::syntax::util::constant::TAB_SIZE;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Name {
    AttentionSequence,
    Autolink,
    AutolinkEmail,
    AutolinkMarker,
    AutolinkProtocol,
    BlankLineEnding,
    BlockQuote,
    BlockQuoteMarker,
    BlockQuotePrefix,
    ByteOrderMark,
    CharacterEscape,
    CharacterEscapeMarker,
    CharacterEscapeValue,
    CharacterReference,
    CharacterReferenceMarker,
    CharacterReferenceMarkerHexadecimal,
    CharacterReferenceMarkerNumeric,
    CharacterReferenceMarkerSemi,
    CharacterReferenceValue,
    CodeFenced,
    CodeFencedFence,
    CodeFencedFenceInfo,
    CodeFencedFenceMeta,
    CodeFencedFenceSequence,
    CodeFlowChunk,
    CodeIndented,
    CodeText,
    CodeTextData,
    CodeTextSequence,
    Content,
    Data,
    Definition,
    DefinitionDestination,
    DefinitionDestinationLiteral,
    DefinitionDestinationLiteralMarker,
    DefinitionDestinationRaw,
    DefinitionDestinationString,
    DefinitionLabel,
    DefinitionLabelMarker,
    DefinitionLabelString,
    DefinitionMarker,
    DefinitionTitle,
    DefinitionTitleMarker,
    DefinitionTitleString,
    Emphasis,
    EmphasisSequence,
    EmphasisText,
    GfmAutolinkLiteralEmail,
    GfmAutolinkLiteralProtocol,
    GfmAutolinkLiteralWww,
    GfmFootnoteCall,
    GfmFootnoteCallLabel,
    GfmFootnoteCallMarker,
    GfmFootnoteDefinition,
    GfmFootnoteDefinitionPrefix,
    GfmFootnoteDefinitionLabel,
    GfmFootnoteDefinitionLabelMarker,
    GfmFootnoteDefinitionLabelString,
    GfmFootnoteDefinitionMarker,
    GfmStrikethrough,
    GfmStrikethroughSequence,
    GfmStrikethroughText,
    GfmTable,
    GfmTableBody,
    GfmTableCell,
    GfmTableCellText,
    GfmTableCellDivider,
    GfmTableDelimiterRow,
    GfmTableDelimiterMarker,
    GfmTableDelimiterCell,
    GfmTableDelimiterCellValue,
    GfmTableDelimiterFiller,
    GfmTableHead,
    GfmTableRow,
    GfmTaskListItemCheck,
    GfmTaskListItemMarker,
    GfmTaskListItemValueChecked,
    GfmTaskListItemValueUnchecked,
    HardBreakEscape,
    HardBreakTrailing,
    HeadingAtx,
    HeadingAtxSequence,
    HeadingAtxText,
    HeadingSetext,
    HeadingSetextText,
    HeadingSetextUnderline,
    HeadingSetextUnderlineSequence,
    HtmlFlow,
    HtmlFlowData,
    HtmlText,
    HtmlTextData,
    Image,
    Label,
    LabelEnd,
    LabelImage,
    LabelImageMarker,
    LabelLink,
    LabelMarker,
    LabelText,
    LineEnding,
    Link,
    ListItem,
    ListItemMarker,
    ListItemPrefix,
    ListItemValue,
    ListOrdered,
    ListUnordered,
    Paragraph,
    Reference,
    ReferenceMarker,
    ReferenceString,
    Resource,
    ResourceDestination,
    ResourceDestinationLiteral,
    ResourceDestinationLiteralMarker,
    ResourceDestinationRaw,
    ResourceDestinationString,
    ResourceMarker,
    ResourceTitle,
    ResourceTitleMarker,
    ResourceTitleString,
    SpaceOrTab,
    Strong,
    StrongSequence,
    StrongText,
    ThematicBreak,
    ThematicBreakSequence,

    GsxPairedSequenceTemporary,
    GsxPairedLiteral,
    GsxSpoiler,
    GsxUnderline,
    GsxHighlight,
    GsxPairedSequence,
    GsxPairedText,
    GsxDirectiveText,
    GsxDirectiveTextMarker,
    GsxDirectiveTextIdentifier,
    GsxDirectiveTextLabel,
    GsxDirectiveTextLabelMarker,
    GsxDirectiveTextLabelString,
    GsxDirectiveTextAttributes,
    GsxDirectiveTextAttributesMarker,
    GsxDirectiveTextAttribute,
    GsxDirectiveTextAttributeShortcut,
    GsxDirectiveTextAttributeKey,
    GsxDirectiveTextAttributeValue,
    GsxMedia,
    GsxMath,
}

pub const VOID_EVENTS: &[Name] = &[
    Name::GsxPairedSequenceTemporary,
    Name::GsxPairedLiteral,
    Name::GsxPairedSequence,
    Name::GsxDirectiveTextMarker,
    Name::GsxDirectiveTextIdentifier,
    Name::GsxMedia,
    Name::AttentionSequence,
    Name::AutolinkEmail,
    Name::AutolinkMarker,
    Name::AutolinkProtocol,
    Name::BlankLineEnding,
    Name::BlockQuoteMarker,
    Name::ByteOrderMark,
    Name::CharacterEscapeMarker,
    Name::CharacterEscapeValue,
    Name::CharacterReferenceMarker,
    Name::CharacterReferenceMarkerHexadecimal,
    Name::CharacterReferenceMarkerNumeric,
    Name::CharacterReferenceMarkerSemi,
    Name::CharacterReferenceValue,
    Name::CodeFencedFenceSequence,
    Name::CodeFlowChunk,
    Name::CodeTextData,
    Name::CodeTextSequence,
    Name::Data,
    Name::DefinitionDestinationLiteralMarker,
    Name::DefinitionLabelMarker,
    Name::DefinitionMarker,
    Name::DefinitionTitleMarker,
    Name::EmphasisSequence,
    Name::GfmAutolinkLiteralEmail,
    Name::GfmAutolinkLiteralProtocol,
    Name::GfmAutolinkLiteralWww,
    Name::GfmFootnoteCallMarker,
    Name::GfmFootnoteDefinitionLabelMarker,
    Name::GfmFootnoteDefinitionMarker,
    Name::GfmStrikethroughSequence,
    Name::GfmTableCellDivider,
    Name::GfmTableDelimiterMarker,
    Name::GfmTableDelimiterFiller,
    Name::GfmTaskListItemMarker,
    Name::GfmTaskListItemValueChecked,
    Name::GfmTaskListItemValueUnchecked,
    Name::HardBreakEscape,
    Name::HardBreakTrailing,
    Name::HeadingAtxSequence,
    Name::HeadingSetextUnderlineSequence,
    Name::HtmlFlowData,
    Name::HtmlTextData,
    Name::LabelImageMarker,
    Name::LabelMarker,
    Name::LineEnding,
    Name::ListItemMarker,
    Name::ListItemValue,
    Name::ReferenceMarker,
    Name::ResourceMarker,
    Name::ResourceTitleMarker,
    Name::SpaceOrTab,
    Name::StrongSequence,
    Name::ThematicBreakSequence,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Content {
    Flow,
    #[allow(clippy::enum_variant_names)]
    Content,
    String,
    Text,
}

const UNLINKED: u32 = u32::MAX;

#[derive(Clone, Copy, Debug)]
pub struct Link {
    previous: u32,
    next: u32,
    pub content: Content,
}

impl Link {
    pub fn new(content: Content) -> Link {
        Link { previous: UNLINKED, next: UNLINKED, content }
    }

    pub fn previous(&self) -> Option<usize> {
        (self.previous != UNLINKED).then_some(self.previous as usize)
    }

    pub fn next(&self) -> Option<usize> {
        (self.next != UNLINKED).then_some(self.next as usize)
    }

    pub fn set_previous(&mut self, index: Option<usize>) {
        self.previous = index.map_or(UNLINKED, |index| index as u32);
    }

    pub fn set_next(&mut self, index: Option<usize>) {
        self.next = index.map_or(UNLINKED, |index| index as u32);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Point {
    pub line: u32,
    pub column: u32,
    pub index: u32,
    pub vs: u32,
}

impl Point {
    pub fn shift_to(&self, bytes: &[u8], index: usize) -> Point {
        let mut next = *self;
        let index = index as u32;
        debug_assert!(index > next.index, "expected to shift forward");

        while next.index < index {
            match bytes[next.index as usize] {
                b'\n' | b'\r' => unreachable!("cannot move past line endings"),
                b'\t' => {
                    let remainder = next.column % TAB_SIZE as u32;
                    let vs = if remainder == 0 { 0 } else { TAB_SIZE as u32 - remainder };
                    next.index += 1;
                    next.column += 1 + vs;
                }
                _ => {
                    next.index += 1;
                    next.column += 1;
                }
            }
        }

        next
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    Enter,
    Exit,
}

#[derive(Clone, Copy, Debug)]
pub struct Event {
    pub kind: Kind,
    pub name: Name,
    pub point: Point,
    pub link: Option<Link>,
}
