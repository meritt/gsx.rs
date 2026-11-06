use std::borrow::Cow;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidText {
    pub code_point: u32,
    pub start: u32,
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

#[derive(Clone, Debug)]
pub struct LineIndex {
    line_starts: Vec<u32>,
    utf16: Option<Vec<u32>>,
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
        let utf16 = (!text.is_ascii() || !collapsed.is_empty()).then(|| {
            let mut table = vec![0; text.len() + 1];
            let mut unit = 0;
            let mut next = collapsed.iter().peekable();
            for (index, char) in text.char_indices() {
                table[index..index + char.len_utf8()].fill(unit);
                unit += char.len_utf16() as u32;
                if next.peek().is_some_and(|&&at| at as usize == index) {
                    next.next();
                    unit += 1;
                }
            }
            table[text.len()] = unit;
            table
        });
        let mut index = LineIndex { line_starts: vec![0], utf16, collapsed, len: 0 };
        index.len = index.utf16(text.len());
        index.line_starts.extend(text.match_indices('\n').map(|(at, _)| match &index.utf16 {
            Some(table) => table[at + 1],
            None => (at + 1) as u32,
        }));
        index
    }

    pub fn utf16(&self, byte: usize) -> u32 {
        self.utf16.as_ref().map_or(byte as u32, |table| table[byte])
    }

    pub fn byte(&self, utf16: u32) -> usize {
        self.utf16.as_ref().map_or(utf16 as usize, |table| table.partition_point(|&unit| unit < utf16))
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
}

impl Limit {
    pub const ALL: [Limit; 11] = [
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
    ];

    pub const fn maximum(self) -> u32 {
        match self {
            Limit::BlockDelimiters
            | Limit::DocumentHardBreaks
            | Limit::DocumentMathLiterals
            | Limit::DocumentPairedDelimiters => 1024,
            Limit::BlockLabels | Limit::DocumentEscapes => 4096,
            Limit::BlockLabelEnds => 256,
            Limit::ContainerDepth => 64,
            Limit::DocumentLines => 16_384,
            Limit::DocumentListItems | Limit::DocumentTableCells => 8192,
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
        }
    }
}
