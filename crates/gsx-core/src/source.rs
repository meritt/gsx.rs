use std::borrow::Cow;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidText {
    pub code_point: u32,
    pub start: u32,
}

pub fn oversized(input: &[u8]) -> Option<(u32, u32)> {
    let maximum = u64::from(Limit::DocumentUnits.maximum());
    if input.len() as u64 <= maximum {
        return None;
    }
    let (mut units, mut wide) = (0, false);
    for &byte in input {
        let width = u64::from(byte & 0xC0 != 0x80) + u64::from(byte >= 0xF0);
        wide |= width == 2 && units + 1 == maximum;
        units += width;
    }
    oversized_units(units, wide)
}

pub fn oversized_units(units: u64, wide: bool) -> Option<(u32, u32)> {
    let maximum = Limit::DocumentUnits.maximum();
    (units > u64::from(maximum)).then(|| (maximum - u32::from(wide), u32::try_from(units).unwrap_or(u32::MAX)))
}

pub fn normalize(text: &str) -> Result<Cow<'_, str>, InvalidText> {
    let bytes = text.as_bytes();
    let control = bytes.iter().enumerate().position(|(at, &byte)| match byte {
        b'\0'..=b'\x08' | b'\x0b' | b'\x0c' | b'\x0e'..=b'\x1f' | b'\x7f' => true,
        0xc2 => matches!(bytes.get(at + 1), Some(0x80..=0x9f)),
        _ => false,
    });
    if let Some(at) = control {
        let code_point = text[at..].chars().next().map_or(0, u32::from);
        return Err(InvalidText { code_point, start: text[..at].encode_utf16().count() as u32 });
    }
    if !text.contains('\r') {
        return Ok(Cow::Borrowed(text));
    }
    Ok(Cow::Owned(text.replace("\r\n", "\n").replace('\r', "\n")))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Point {
    pub line: u32,
    pub column: u32,
    pub offset: u32,
}

const BLOCK: usize = 64;

#[derive(Clone, Debug)]
struct Units {
    base: Vec<u32>,
    offsets: Vec<u8>,
    total: u32,
}

impl Units {
    fn new(text: &str, collapsed: &[u32]) -> Self {
        let mut base = Vec::with_capacity(text.len().div_ceil(BLOCK));
        let mut offsets = Vec::with_capacity(text.len());
        let mut collapsed = collapsed.iter().map(|&at| at as usize).peekable();
        let (mut unit, mut width) = (0, 0);
        for (block, bytes) in text.as_bytes().chunks(BLOCK).enumerate() {
            let mut first = 0;
            offsets.extend(bytes.iter().enumerate().map(|(offset, &byte)| {
                let start = byte & 0xC0 != 0x80;
                let ends = u32::from(collapsed.next_if_eq(&(block * BLOCK + offset)).is_some());
                unit += if start { width } else { 0 };
                width = if start { 1 + u32::from(byte >= 0xF0) + ends } else { width };
                if offset == 0 {
                    first = unit;
                }
                (unit - first) as u8
            }));
            base.push(first);
        }
        Units { base, offsets, total: unit + width }
    }

    fn at(&self, byte: usize) -> u32 {
        self.offsets.get(byte).map_or(self.total, |&offset| self.base[byte / BLOCK] + u32::from(offset))
    }

    fn byte(&self, utf16: u32) -> usize {
        let Some(block) = self.base.partition_point(|&unit| unit < utf16).checked_sub(1) else {
            return 0;
        };
        let start = block * BLOCK;
        let offsets = &self.offsets[start..self.offsets.len().min(start + BLOCK)];
        let within = offsets.partition_point(|&offset| self.base[block] + u32::from(offset) < utf16);
        if start + within < self.offsets.len() {
            return start + within;
        }
        self.offsets.len() + usize::from(self.total < utf16)
    }
}

#[derive(Clone, Debug)]
pub struct LineIndex {
    line_starts: Vec<u32>,
    utf16: Option<Units>,
    collapsed: Vec<u32>,
    len: u32,
}

impl LineIndex {
    pub fn new(text: &str, input: &str) -> Self {
        let mut collapsed = vec![];
        if input.len() != text.len() {
            for (at, _) in input.match_indices("\r\n") {
                collapsed.push((at - collapsed.len()) as u32);
            }
        }
        let utf16 = (!text.is_ascii() || !collapsed.is_empty()).then(|| Units::new(text, &collapsed));
        let mut index = LineIndex { line_starts: vec![0], utf16, collapsed, len: 0 };
        index.len = index.utf16(text.len());
        index.line_starts.extend(text.match_indices('\n').map(|(at, _)| match &index.utf16 {
            Some(units) => units.at(at + 1),
            None => (at + 1) as u32,
        }));
        index
    }

    pub fn utf16(&self, byte: usize) -> u32 {
        self.utf16.as_ref().map_or(byte as u32, |units| units.at(byte))
    }

    pub fn byte(&self, utf16: u32) -> usize {
        self.utf16.as_ref().map_or(utf16 as usize, |units| units.byte(utf16))
    }

    pub fn input_byte(&self, byte: usize) -> usize {
        byte + self.collapsed.partition_point(|&at| (at as usize) < byte)
    }

    pub fn len(&self) -> u32 {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn point(&self, offset: u32) -> Point {
        let line = self.line_starts.partition_point(|&start| start <= offset);
        Point { line: line as u32, column: offset - self.line_starts[line - 1] + 1, offset }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Limit {
    BlockDelimiters,
    BlockLabels,
    BlockLabelEnds,
    ContainerDepth,
    DocumentEscapes,
    DocumentHardBreaks,
    DocumentLines,
    DocumentListItems,
    DocumentMathLiterals,
    DocumentPairedDelimiters,
    DocumentTableCells,
    DocumentUnits,
    InlineDepth,
}

impl Limit {
    pub const ALL: [Limit; 13] = [
        Limit::BlockDelimiters,
        Limit::BlockLabels,
        Limit::BlockLabelEnds,
        Limit::ContainerDepth,
        Limit::DocumentEscapes,
        Limit::DocumentHardBreaks,
        Limit::DocumentLines,
        Limit::DocumentListItems,
        Limit::DocumentMathLiterals,
        Limit::DocumentPairedDelimiters,
        Limit::DocumentTableCells,
        Limit::DocumentUnits,
        Limit::InlineDepth,
    ];

    pub const fn maximum(self) -> u32 {
        match self {
            Limit::BlockDelimiters
            | Limit::DocumentHardBreaks
            | Limit::DocumentMathLiterals
            | Limit::DocumentPairedDelimiters => 1024,
            Limit::BlockLabels | Limit::DocumentEscapes => 4096,
            Limit::BlockLabelEnds => 256,
            Limit::ContainerDepth => 32,
            Limit::DocumentLines => 16_384,
            Limit::DocumentListItems | Limit::DocumentTableCells => 8192,
            Limit::DocumentUnits => 262_144,
            Limit::InlineDepth => 16,
        }
    }

    pub const fn key(self) -> &'static str {
        match self {
            Limit::BlockDelimiters => "blockDelimiters",
            Limit::BlockLabels => "blockLabels",
            Limit::BlockLabelEnds => "blockLabelEnds",
            Limit::ContainerDepth => "containerDepth",
            Limit::DocumentEscapes => "documentEscapes",
            Limit::DocumentHardBreaks => "documentHardBreaks",
            Limit::DocumentLines => "documentLines",
            Limit::DocumentListItems => "documentListItems",
            Limit::DocumentMathLiterals => "documentMathLiterals",
            Limit::DocumentPairedDelimiters => "documentPairedDelimiters",
            Limit::DocumentTableCells => "documentTableCells",
            Limit::DocumentUnits => "documentUnits",
            Limit::InlineDepth => "inlineDepth",
        }
    }
}
