use std::cell::RefCell;

use crate::document::{Document, Kind, Node, Range};

use super::layout::ELLIPSIS;
use super::links::unicode::letter_or_number;
use super::table::Truncation;
use crate::measure;

const LONGEST_WORD: usize = 32;
const CHUNK: usize = 512;

#[derive(Clone, Copy, Debug)]
pub struct Leaf<'a> {
    pub node: Option<&'a Node>,
    pub text: &'a str,
    pub code: bool,
    atomic: bool,
    nested: bool,
    previous: Option<u32>,
    source: Option<(u32, bool)>,
}

#[derive(Clone, Copy, Debug)]
struct Segment {
    leaf: u32,
    offset: u32,
    retained: bool,
}

#[derive(Debug)]
struct Cursor {
    first: usize,
    last: usize,
    base: usize,
    leaf: usize,
    start: usize,
    next: usize,
    segmenter: measure::Segmenter,
}

#[derive(Debug, Default)]
struct Built {
    flows: usize,
    cursor: Option<Cursor>,
    segments: Vec<Segment>,
    effective: Vec<u32>,
    letters: Vec<bool>,
    risky: Vec<bool>,
    visible: String,
    visible_starts: Vec<Option<usize>>,
    settled: usize,
    scanned: usize,
    starts: Vec<usize>,
    value: String,
    boundaries: Vec<usize>,
}

#[derive(Debug)]
pub struct Outline<'a> {
    leaves: Vec<Leaf<'a>>,
    flows: Vec<(usize, usize)>,
    end: u32,
    built: RefCell<Built>,
}

#[derive(Clone, Debug)]
pub struct Cut<'a> {
    pub leaf: Leaf<'a>,
    pub index: u32,
    pub kept: usize,
    pub replacement: String,
}

impl Cut<'_> {
    pub fn key(&self) -> (u32, usize) {
        (self.index, self.kept)
    }

    pub fn dropped(&self, end: u32) -> (Range, bool) {
        match self.leaf.source {
            Some((start, true)) => {
                let kept = self.leaf.text[..self.kept].encode_utf16().count() as u32;
                (Range { start: start + kept, end }, true)
            }
            Some((start, false)) => (Range { start, end }, false),
            None => (Range { start: 0, end }, false),
        }
    }
}

fn continuous(kind: &Kind) -> bool {
    matches!(
        kind,
        Kind::Paragraph
            | Kind::Heading { .. }
            | Kind::TableCell
            | Kind::Emphasis
            | Kind::Strong
            | Kind::Delete
            | Kind::Underline
            | Kind::Highlight
            | Kind::Spoiler
            | Kind::Link { .. }
            | Kind::Datetime { .. }
    )
}

struct Builder<'a> {
    document: Option<&'a Document>,
    leaves: Vec<Leaf<'a>>,
    flows: Vec<(usize, usize)>,
    open: usize,
    nested: usize,
    math: bool,
}

impl<'a> Builder<'a> {
    fn flush(&mut self) {
        if self.leaves.len() > self.open {
            self.flows.push((self.open, self.leaves.len()));
            self.open = self.leaves.len();
        }
    }

    fn leaf(&mut self, node: &'a Node, text: &'a str, code: bool, atomic: bool) {
        let source = self.document.map(|document| (node.range.start, document.source(node.range) == text));
        let previous = (self.leaves.len() > self.open).then(|| self.leaves.len() as u32 - 1);
        self.leaves.push(Leaf { node: Some(node), text, code, atomic, nested: self.nested > 0, previous, source });
    }

    fn visit(&mut self, node: &'a Node) {
        match &node.kind {
            Kind::Text { value } => self.leaf(node, value, false, false),
            Kind::Math { value } => self.leaf(node, value, false, self.math),
            Kind::Code { lang, value, .. } => {
                self.leaf(node, value, true, self.math && lang.as_deref() == Some("math"));
            }
            Kind::InlineCode { value } => self.leaf(node, value, true, false),
            Kind::Paragraph | Kind::Heading { .. } | Kind::TableCell => {
                node.children.iter().for_each(|child| self.visit(child));
            }
            kind if continuous(kind) => {
                self.nested += 1;
                node.children.iter().for_each(|child| self.visit(child));
                self.nested -= 1;
            }
            _ => {
                self.flush();
                for child in &node.children {
                    self.visit(child);
                    self.flush();
                }
            }
        }
    }
}

impl<'a> Outline<'a> {
    pub fn of_nodes(document: &'a Document, nodes: &'a [Node], math: bool) -> Self {
        let mut builder = Builder { document: Some(document), leaves: vec![], flows: vec![], open: 0, nested: 0, math };
        for node in nodes {
            builder.flush();
            builder.visit(node);
        }
        builder.flush();
        Self::new(builder.leaves, builder.flows, document.end().offset)
    }

    pub fn of_text(text: &'a str) -> Self {
        let leaf = Leaf { node: None, text, code: false, atomic: false, nested: false, previous: None, source: None };
        Self::new(vec![leaf], vec![(0, usize::from(!text.is_empty()))], 0)
    }

    pub fn of_rendered(text: &'a str) -> Self {
        let leaf = Leaf {
            node: None,
            text,
            code: false,
            atomic: false,
            nested: false,
            previous: None,
            source: Some((0, true)),
        };
        let units = text.encode_utf16().count() as u32;
        Self::new(vec![leaf], vec![(0, usize::from(!text.is_empty()))], units)
    }

    fn new(leaves: Vec<Leaf<'a>>, flows: Vec<(usize, usize)>, end: u32) -> Self {
        Outline { leaves, flows, end, built: RefCell::default() }
    }

    fn open(&self, built: &mut Built) -> bool {
        let Some(&(first, last)) = self.flows.get(built.flows) else {
            return false;
        };
        built.flows += 1;
        let Built { cursor, segments, starts, value, boundaries, .. } = built;
        starts.clear();
        value.clear();
        boundaries.clear();
        for leaf in &self.leaves[first..last] {
            starts.push(value.len());
            value.push_str(leaf.text);
        }
        starts.push(value.len());
        *cursor = Some(Cursor {
            first,
            last,
            base: segments.len(),
            leaf: 0,
            start: 0,
            next: 0,
            segmenter: measure::Segmenter::default(),
        });
        true
    }

    fn extend(&self, built: &mut Built) -> bool {
        if built.cursor.is_none() && !self.open(built) {
            return false;
        }
        let Built {
            cursor: Some(cursor),
            segments,
            effective,
            letters,
            visible,
            visible_starts,
            starts,
            value,
            boundaries,
            ..
        } = built
        else {
            unreachable!("an open flow has a cursor")
        };
        let flow = &self.leaves[cursor.first..cursor.last];
        let (head, fresh, first_leaf) = (cursor.start, boundaries.len(), cursor.leaf);
        let mut done = true;
        for (offset, char) in value[cursor.next..].char_indices() {
            let at = cursor.next + offset;
            if cursor.segmenter.starts(char) && at > cursor.start {
                boundaries.push(cursor.start);
                cursor.start = at;
                if at - head >= CHUNK && value[..at].ends_with(char::is_whitespace) {
                    cursor.next = at + char.len_utf8();
                    done = false;
                    break;
                }
            }
        }
        if done && cursor.start < value.len() {
            boundaries.push(cursor.start);
        }
        let tail = if done { value.len() } else { cursor.start };
        let mut leaf = first_leaf;
        for index in fresh..boundaries.len() {
            let start = boundaries[index];
            let end = boundaries.get(index + 1).copied().unwrap_or(tail);
            while leaf + 1 < flow.len() && start >= starts[leaf + 1] {
                leaf += 1;
            }
            let mut last_leaf = leaf;
            while last_leaf + 1 < flow.len() && end > starts[last_leaf + 1] {
                last_leaf += 1;
            }
            let retained = flow[leaf..=last_leaf].iter().all(|leaf| !leaf.code);
            segments.push(Segment {
                leaf: (cursor.first + leaf) as u32,
                offset: (start - starts[leaf]) as u32,
                retained,
            });
            if retained {
                visible_starts.push(Some(visible.len()));
                visible.push_str(&value[start..end]);
            } else {
                visible_starts.push(None);
            }
            letters.push(match &value.as_bytes()[start..end] {
                [byte] => byte.is_ascii_alphanumeric(),
                _ => value[start..end].chars().any(|char| letter_or_number(u32::from(char))),
            });
        }
        cursor.leaf = leaf;
        let base = cursor.base;
        effective.extend((base + fresh..segments.len()).map(|index| index as u32));
        let mut run = first_leaf;
        while run < flow.len() && starts[run] < tail {
            if flow[run].code {
                run += 1;
                continue;
            }
            let mut stop = run + 1;
            while stop < flow.len() && !flow[stop].code && starts[stop] < tail {
                stop += 1;
            }
            let offset = starts[run].max(head);
            let text = &value[offset..starts[stop].min(tail).max(offset)];
            for (from, to) in entities(text) {
                let beginning = boundaries.partition_point(|&at| at < offset + from);
                let ending = boundaries.partition_point(|&at| at < offset + to);
                for keep in beginning + 1..ending {
                    effective[base + keep] = (base + beginning) as u32;
                }
            }
            run = stop;
        }
        for index in fresh..boundaries.len() {
            let segment = segments[effective[base + index] as usize];
            let local = segment.leaf as usize - cursor.first;
            if flow[local].atomic && segment.offset > 0 {
                let first = boundaries.partition_point(|&at| at <= starts[local]) - 1;
                effective[base + index] = effective[base + first];
            }
        }
        if done {
            built.cursor = None;
        }
        self.settle(built);
        true
    }

    fn settle(&self, built: &mut Built) {
        let from = built.settled;
        let settled = if built.flows == self.flows.len() && built.cursor.is_none() {
            built.visible.len()
        } else {
            let scanned = built.scanned;
            built.visible[scanned..]
                .char_indices()
                .rev()
                .find(|(_, char)| char.is_whitespace())
                .map_or(from, |(at, char)| scanned + at + char.len_utf8())
        };
        built.scanned = built.visible.len();
        let risky = risky_bytes(&built.visible[from..settled]);
        while let Some(&start) = built.visible_starts.get(built.risky.len()) {
            match start {
                Some(start) if start >= settled => break,
                Some(start) => built.risky.push(risky[start - from]),
                None => built.risky.push(false),
            }
        }
        built.settled = settled;
    }

    fn built(&self, enough: impl Fn(&Built) -> bool) -> std::cell::RefMut<'_, Built> {
        let mut built = self.built.borrow_mut();
        while !enough(&built) && self.extend(&mut built) {}
        built
    }

    pub fn is_empty(&self) -> bool {
        self.built(|built| !built.segments.is_empty()).segments.is_empty()
    }

    pub fn units(&self) -> usize {
        self.built(|_| false).segments.len()
    }

    pub fn clean(&self, keep: usize) -> bool {
        let segment = {
            let built = self.built(|built| built.segments.len() > keep);
            built.segments[built.effective[keep] as usize]
        };
        let leaf = self.leaves[segment.leaf as usize];
        let (before, after) = leaf.text.split_at(segment.offset as usize);
        let space = |char: Option<char>| char.is_none_or(char::is_whitespace);
        !leaf.nested && (leaf.code || space(before.chars().next_back()) || space(after.chars().next()))
    }

    pub fn upper(&self, maximum: usize) -> Option<usize> {
        let mut built = self.built.borrow_mut();
        let (mut index, mut floor) = (0, 0);
        loop {
            while index < built.risky.len() {
                if built.segments[index].retained && !built.risky[index] && built.letters[index] {
                    floor += 1;
                    if floor > maximum {
                        return Some(index);
                    }
                }
                index += 1;
            }
            if !self.extend(&mut built) {
                return None;
            }
        }
    }

    pub fn end(&self) -> u32 {
        self.end
    }

    pub fn cut(&self, keep: usize, truncation: Truncation) -> Cut<'a> {
        let segment = {
            let built = self.built(|built| built.segments.len() > keep);
            built.segments[built.effective[keep] as usize]
        };
        let leaf = self.leaves[segment.leaf as usize];
        let offset = segment.offset as usize;
        let value = leaf.text;
        let (kept, replacement) = match truncation {
            Truncation::Hard => {
                let kept = value[..offset].trim_end().len();
                (kept, value[..kept].to_owned())
            }
            Truncation::Ellipsis if leaf.code => {
                let kept = value[..offset].trim_end().len();
                (kept, format!("{}{ELLIPSIS}", &value[..kept]))
            }
            Truncation::Ellipsis => {
                let kept = kept_prose_end(value, offset);
                let prose = &value[..kept];
                let before = leaf.previous.map_or("", |previous| self.leaves[previous as usize].text);
                let tail = if address_tail(prose) || (prose.is_empty() && address_tail(before)) { " " } else { "" };
                (kept, format!("{prose}{tail}{ELLIPSIS}"))
            }
        };
        Cut { leaf, index: segment.leaf, kept, replacement }
    }
}

fn address_tail(prose: &str) -> bool {
    let token = prose.rsplit(char::is_whitespace).next().unwrap_or("");
    token.find("://").is_some_and(|at| at > 0) || token.starts_with("www.")
}

fn closing_punctuation(char: char) -> bool {
    matches!(char, '.' | ',' | ';' | ':' | '!' | '?' | ')' | ']' | '»' | '”' | '’' | '"' | '\'')
}

fn kept_prose_end(value: &str, offset: usize) -> usize {
    let mut end = offset;
    let next = value[end..].chars().next();
    let previous = value[..end].chars().next_back();
    let inside = previous.is_some_and(|char| !char.is_whitespace()) && next.is_some_and(|char| !char.is_whitespace());
    let mut distance = 0;
    if inside
        && !next.is_some_and(closing_punctuation)
        && let Some((at, _)) = value[..end]
            .char_indices()
            .rev()
            .take_while(|(_, char)| {
                distance += char.len_utf16();
                distance <= LONGEST_WORD
            })
            .find(|(_, char)| char.is_whitespace())
    {
        end = at;
    }
    end = value[..end].trim_end().len();
    value[..end].trim_end_matches(['.', ',', ';', ':']).len()
}

fn risky_bytes(visible: &str) -> Vec<bool> {
    let mut risky = vec![false; visible.len() + 1];
    let mut start = 0;
    for token in visible.split(char::is_whitespace) {
        let at = start;
        start += token.len();
        if let Some(char) = visible[start..].chars().next() {
            start += char.len_utf8();
        }
        if token.contains(['.', '@', '#', ':', '/']) {
            risky[at..at + token.len()].fill(true);
        }
    }
    risky
}

fn word(char: char) -> bool {
    char.is_alphanumeric()
}

fn token_ends(value: &str) -> impl FnMut(usize) -> usize + '_ {
    let mut end = 0;
    move |at: usize| {
        if at >= end {
            end = value[at..].find(char::is_whitespace).map_or(value.len(), |found| at + found);
        }
        end
    }
}

fn entities(value: &str) -> Vec<(usize, usize)> {
    let mut ranges = vec![];
    let mut token_end = token_ends(value);
    for (marker, _) in value.match_indices("://") {
        let start = marker
            - value[..marker]
                .bytes()
                .rev()
                .take_while(|byte| byte.is_ascii_alphanumeric() || b"+.-".contains(byte))
                .count();
        let end = token_end(marker);
        if value.as_bytes()[start].is_ascii_alphabetic() && end > marker + 3 {
            ranges.push((start, end));
        }
    }
    let mut token_end = token_ends(value);
    for (start, _) in value.match_indices("www.") {
        let end = token_end(start);
        if end > start + 4 {
            ranges.push((start, end));
        }
    }
    for (at, sigil) in value.match_indices(['@', '#']) {
        let rest = &value[at + 1..];
        if sigil == "@" {
            let start = at
                - value[..at]
                    .chars()
                    .rev()
                    .take_while(|&char| word(char) || matches!(char, '.' | '_' | '%' | '+' | '-'))
                    .map(char::len_utf8)
                    .sum::<usize>();
            let end = at
                + 1
                + rest
                    .chars()
                    .take_while(|&char| word(char) || matches!(char, '.' | '-'))
                    .map(char::len_utf8)
                    .sum::<usize>();
            if start < at && end > at + 1 {
                ranges.push((start, end));
            }
        }
        let name = |char: char| word(char) || char == '_' || (sigil == "@" && char == '.');
        let end = at + 1 + rest.chars().take_while(|&char| name(char)).map(char::len_utf8).sum::<usize>();
        if end > at + 1 {
            ranges.push((at, end));
        }
    }
    crate::order::sort_by_key(&mut ranges, |&(start, end)| crate::order::pair(start, end));
    let mut merged: Vec<(usize, usize)> = vec![];
    for (start, end) in ranges {
        match merged.last_mut() {
            Some(previous) if start < previous.1 => previous.1 = previous.1.max(end),
            _ => merged.push((start, end)),
        }
    }
    merged
}
