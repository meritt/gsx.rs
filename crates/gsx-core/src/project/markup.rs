use std::borrow::Cow;
use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashMap;

use crate::document::{Document, Kind, Node, Range};
use crate::measure::{self, MediaKind, clusters};

use super::change::{Change, Feature};
use super::facts::Facts;
use super::outline::{Cut, Outline};
use super::table::When;
use super::thread::{closing, terminal};
use super::{Bound, Boundary, Fitted, Rendered, Request, Split, appended, render};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct At {
    block: usize,
    child: usize,
    byte: usize,
}

impl At {
    const fn block(block: usize) -> Self {
        At { block, child: 0, byte: 0 }
    }

    const fn edge(self) -> bool {
        self.child == 0 && self.byte == 0
    }
}

#[derive(Clone, Copy, Debug)]
struct Cursor {
    at: At,
    units: usize,
}

impl Cursor {
    const fn block(block: usize) -> Self {
        Cursor { at: At::block(block), units: 0 }
    }
}

#[derive(Clone, Copy, Debug)]
struct Stop {
    end: At,
    next: At,
    before: usize,
    after: usize,
    class: Boundary,
}

struct Pieces {
    stops: Vec<Stop>,
    starts: Vec<usize>,
}

impl Pieces {
    fn total(&self) -> usize {
        self.starts.last().copied().unwrap_or(0)
    }
}

struct Estimate {
    units: usize,
    blank: bool,
}

struct Edge {
    to: usize,
    units: usize,
    kind: Option<MediaKind>,
    blank: bool,
}

#[derive(Clone, Copy, Debug)]
struct Plan {
    to: At,
    next: Cursor,
    boundary: Boundary,
    kind: Option<MediaKind>,
    oversized: bool,
}

enum Descent {
    Cut(Plan),
    Oversized(usize),
    Stuck,
}

struct Entry {
    from: At,
    to: At,
    fitted: Fitted,
    boundary: Option<Boundary>,
    media: Vec<String>,
    truncated: Option<Change>,
}

#[derive(Default)]
struct Leaf {
    exact: bool,
    runs: Option<Vec<(usize, usize, u32, u32)>>,
    reached: (usize, u32),
}

struct Limits {
    bounds: Vec<Bound>,
    cap: usize,
}

const KINDS: [Option<MediaKind>; 5] =
    [None, Some(MediaKind::Image), Some(MediaKind::Video), Some(MediaKind::Gif), Some(MediaKind::Audio)];

const fn slot(kind: Option<MediaKind>) -> usize {
    match kind {
        None => 0,
        Some(MediaKind::Image) => 1,
        Some(MediaKind::Video) => 2,
        Some(MediaKind::Gif) => 3,
        Some(MediaKind::Audio) => 4,
    }
}

pub(super) fn split(
    request: &Request,
    document: &Document,
    body: &[Node],
    full: &Rendered,
    suffix: Option<&str>,
) -> Option<Split> {
    let splitter = Splitter::new(request, document, body, suffix);
    let whole = splitter.leading.or_else(|| splitter.carried.iter().find_map(|kind| *kind));
    if splitter.within(&appended(request.target, Rendered::text(full.text.clone()), suffix), whole) {
        return None;
    }
    let mut entries: Vec<Entry> = vec![];
    let mut cursor = Cursor::block(0);
    while cursor.at.block < body.len() {
        let first = entries.is_empty();
        let plan = splitter.plan(cursor, first)?;
        let (mut fitted, truncated) = splitter.fitted(cursor.at, &plan, first)?;
        fitted.rendered.changes = vec![];
        fitted.rendered.alts = vec![];
        let mut media = vec![];
        for id in splitter.ids(cursor.at, plan.to) {
            if !media.contains(&id) {
                media.push(id);
            }
        }
        let blank = request.target.form.visible_blank(&fitted.rendered.text, measure::blank);
        match entries.last_mut() {
            Some(previous) if blank && media.is_empty() && truncated.is_none() => {
                previous.to = plan.to;
                previous.boundary = Some(plan.boundary);
            }
            _ => entries.push(Entry {
                from: cursor.at,
                to: plan.to,
                fitted,
                boundary: Some(plan.boundary),
                media,
                truncated,
            }),
        }
        cursor = plan.next;
    }
    splitter.assemble(entries)
}

fn furthest(count: usize, mut fits: impl FnMut(usize) -> bool) -> Option<usize> {
    if count == 0 {
        return None;
    }
    if fits(count - 1) {
        return Some(count - 1);
    }
    let (mut low, mut high) = (0, count - 1);
    while low < high {
        let middle = low + (high - low) / 2;
        if fits(middle) { low = middle + 1 } else { high = middle }
    }
    low.checked_sub(1)
}

fn class(last: Option<char>) -> Boundary {
    if last.is_some_and(terminal) { Boundary::Sentence } else { Boundary::Word }
}

fn trailing(node: &Node) -> Option<char> {
    match &node.kind {
        Kind::Text { value } | Kind::InlineCode { value } => value.chars().rev().find(|char| !closing(*char)),
        Kind::Math { .. } => Some('$'),
        Kind::Break => Some('\n'),
        _ => node.children.iter().rev().find_map(trailing),
    }
}

fn runs(text: &str) -> Vec<(usize, usize, u32, u32)> {
    let mut runs = vec![];
    let mut units = 0;
    let mut open: Option<(usize, u32)> = None;
    for (byte, char) in text.char_indices() {
        if char.is_whitespace() {
            open.get_or_insert((byte, units));
        } else if let Some((start, from)) = open.take() {
            runs.push((start, byte, from, units));
        }
        units += char.len_utf16() as u32;
    }
    if let Some((start, from)) = open {
        runs.push((start, text.len(), from, units));
    }
    runs
}

struct Splitter<'s> {
    request: &'s Request<'s, 's>,
    document: &'s Document,
    body: &'s [Node],
    suffix: Option<&'s str>,
    limits: [Limits; KINDS.len()],
    carried: Vec<Option<MediaKind>>,
    leading: Option<MediaKind>,
    collapses: bool,
    estimates: Vec<OnceCell<Estimate>>,
    pieces: Vec<OnceCell<Pieces>>,
    leaves: RefCell<HashMap<(usize, usize), Leaf>>,
    head: Cell<Option<Head>>,
    admitted: Cell<Option<Admitted>>,
}

type Head = ((At, usize), Vec<Node>);
type Admitted = ((At, At, Option<MediaKind>, bool), Rendered);

impl<'s> Splitter<'s> {
    fn new(request: &'s Request<'s, 's>, document: &'s Document, body: &'s [Node], suffix: Option<&'s str>) -> Self {
        let facts = request.facts;
        let limits = KINDS.map(|kind| {
            let facts = Facts { media: kind, ..*facts };
            let bounds = Request { facts: &facts, ..*request }.limits();
            let cap = bounds
                .iter()
                .filter(|bound| bound.unit.max_bytes().is_some())
                .filter_map(|bound| bound.maximum)
                .min()
                .unwrap_or(usize::MAX);
            Limits { bounds, cap }
        });
        let positions = facts.items.map(|items| (items, super::media::positions(items)));
        let available = |id: &str| match &positions {
            None => Some(None),
            Some((items, positions)) => positions.get(id).map(|&index| super::media::kind(items[index].format)),
        };
        let carried = body
            .iter()
            .map(|node| {
                let mut ids = vec![];
                super::media(std::slice::from_ref(node), &mut ids);
                let kind = ids.iter().find_map(|id| available(id))?;
                Some(facts.media.or(kind).unwrap_or(MediaKind::Image))
            })
            .collect();
        let referenced = super::media::references(body);
        let unreferenced = facts.items.unwrap_or_default().iter().find(|item| !referenced.contains_key(item.id));
        let leading = match unreferenced {
            Some(item) => facts.media.or(super::media::kind(item.format)),
            None if facts.items.is_some_and(|items| !items.is_empty()) => None,
            None => facts.media,
        };
        Splitter {
            request,
            document,
            body,
            suffix,
            limits,
            carried,
            leading,
            collapses: request.target.conditions(Feature::Text, When::BlankLines),
            estimates: body.iter().map(|_| OnceCell::new()).collect(),
            pieces: body.iter().map(|_| OnceCell::new()).collect(),
            leaves: RefCell::new(HashMap::new()),
            head: Cell::new(None),
            admitted: Cell::new(None),
        }
    }

    fn limits(&self, kind: Option<MediaKind>) -> &Limits {
        &self.limits[slot(kind)]
    }

    fn cap(&self, kind: Option<MediaKind>) -> usize {
        self.limits(kind).cap
    }

    fn within(&self, rendered: &Rendered, kind: Option<MediaKind>) -> bool {
        let bounds = &self.limits(kind).bounds;
        self.request.admits(bounds, rendered)
    }

    fn draw(&self, nodes: &[Node], cut: Option<&Cut>, first: bool) -> Rendered {
        let target = self.request.target;
        let rendered =
            render(target, self.request.facts, self.request.spec.name, self.request.spent, cut, false, |context| {
                (target.renderer.blocks)(context, nodes);
            });
        if first { appended(target, rendered, self.suffix) } else { rendered }
    }

    fn visible(&self, rendered: &Rendered) -> Estimate {
        let visible = self.request.target.form.visible(&rendered.text);
        Estimate { units: visible.chars().count(), blank: measure::blank(&visible) }
    }

    fn estimate(&self, index: usize) -> &Estimate {
        self.estimates[index].get_or_init(|| {
            let node = &self.body[index];
            if node.media_ids().is_some() {
                return Estimate { units: 0, blank: true };
            }
            self.visible(&self.draw(std::slice::from_ref(node), None, false))
        })
    }

    fn inline(&self, node: &Node) -> usize {
        let target = self.request.target;
        let rendered =
            render(target, self.request.facts, self.request.spec.name, self.request.spent, None, false, |context| {
                (target.renderer.inline)(context, std::slice::from_ref(node));
            });
        self.visible(&rendered).units
    }

    fn pieces(&self, block: usize) -> &Pieces {
        self.pieces[block].get_or_init(|| {
            let children = &self.body[block].children;
            let mut stops = vec![];
            let mut starts = Vec::with_capacity(children.len() + 1);
            let (mut units, mut content, mut last) = (0, false, None);
            let mut run: Option<(At, usize, Boundary)> = None;
            for (child, node) in children.iter().enumerate() {
                starts.push(units);
                let at = |byte| At { block, child, byte };
                match &node.kind {
                    Kind::Text { value } => {
                        for (byte, char) in value.char_indices() {
                            if char.is_whitespace() {
                                if content && run.is_none() {
                                    run = Some((at(byte), units, class(last)));
                                }
                            } else {
                                if let Some((end, before, class)) = run.take() {
                                    stops.push(Stop { end, next: at(byte), before, after: units, class });
                                }
                                content = true;
                            }
                            if !closing(char) {
                                last = Some(char);
                            }
                            units += 1;
                        }
                    }
                    Kind::Break => {
                        if content && run.is_none() {
                            run = Some((at(0), units, class(last)));
                        }
                        last = Some('\n');
                        units += 1;
                    }
                    _ => {
                        match run.take() {
                            Some((end, before, class)) => {
                                stops.push(Stop { end, next: at(0), before, after: units, class });
                            }
                            None if content => {
                                stops.push(Stop {
                                    end: at(0),
                                    next: at(0),
                                    before: units,
                                    after: units,
                                    class: class(last),
                                });
                            }
                            None => {}
                        }
                        content = true;
                        units += self.inline(node);
                        last = trailing(node).or(last);
                    }
                }
            }
            starts.push(units);
            Pieces { stops, starts }
        })
    }

    fn normal(&self, at: At) -> At {
        let children = &self.body[at.block].children;
        let at = match children.get(at.child).map(|node| &node.kind) {
            Some(Kind::Text { value }) if at.byte >= value.len() => At { child: at.child + 1, byte: 0, ..at },
            _ => at,
        };
        if at.child >= children.len() { At::block(at.block + 1) } else { at }
    }

    fn walk(&self, cursor: Cursor, kind: Option<MediaKind>) -> Vec<Edge> {
        let mut edges = vec![];
        let (mut units, mut seen, mut blank, mut kind) = (0, false, true, kind);
        let mut index = cursor.at.block;
        if !cursor.at.edge() {
            units = self.pieces(index).total().saturating_sub(cursor.units);
            (seen, blank) = (true, false);
            index += 1;
            edges.push(Edge { to: index, units, kind, blank });
        }
        while index < self.body.len() && units <= self.cap(kind) {
            let node = &self.body[index];
            kind = kind.or(self.carried[index]);
            if node.media_ids().is_none() {
                let estimate = self.estimate(index);
                if seen {
                    let gap = node.gap_before.unwrap_or(1);
                    units += if gap > 2 && self.collapses { 2 } else { gap as usize };
                }
                units += estimate.units;
                seen = true;
                blank &= estimate.blank;
            }
            index += 1;
            edges.push(Edge { to: index, units, kind, blank });
        }
        edges
    }

    fn plan(&self, cursor: Cursor, first: bool) -> Option<Plan> {
        let initial = if first { self.leading } else { None };
        let edges = self.walk(cursor, initial);
        let count = edges.iter().take_while(|edge| edge.units <= self.cap(edge.kind)).count();
        let fitting = furthest(count, |index| {
            let edge = &edges[index];
            self.fits(cursor.at, At::block(edge.to), edge.kind, first)
        })
        .map(|index| &edges[index]);
        let at_edge = |edge: &Edge, oversized| Plan {
            to: At::block(edge.to),
            next: Cursor::block(edge.to),
            boundary: Boundary::Paragraph,
            kind: edge.kind,
            oversized,
        };
        if let Some(edge) = fitting.filter(|edge| !edge.blank) {
            return Some(at_edge(edge, false));
        }
        let frontier = fitting.map_or(cursor.at.block, |edge| edge.to);
        let kind = fitting.map_or(initial, |edge| edge.kind);
        let mut element = None;
        if self.body.get(frontier).is_some_and(|node| matches!(node.kind, Kind::Paragraph)) {
            let start = if frontier == cursor.at.block { cursor } else { Cursor::block(frontier) };
            match self.descend(cursor.at, frontier, start, kind, first) {
                Descent::Cut(plan) => return Some(plan),
                Descent::Stuck => return None,
                Descent::Oversized(child) => element = Some((start, child)),
            }
        }
        let alone = || {
            let end = element.map_or(At::block(frontier + 1), |(_, child)| {
                self.normal(At { block: frontier, child: child + 1, byte: 0 })
            });
            self.fits(At::block(frontier), end, self.carried[frontier], false)
        };
        if let Some(edge) = fitting.filter(|edge| edge.to == self.body.len() || edge.kind.is_some() && alone()) {
            return Some(at_edge(edge, false));
        }
        if fitting.is_none() && initial.is_some() && cursor.at.edge() && alone() {
            return Some(Plan {
                to: cursor.at,
                next: cursor,
                boundary: Boundary::Paragraph,
                kind: initial,
                oversized: false,
            });
        }
        let Some((start, child)) = element else {
            let edge = Edge { to: frontier + 1, units: 0, kind: kind.or(self.carried[frontier]), blank: false };
            return Some(at_edge(&edge, true));
        };
        let pieces = self.pieces(frontier);
        let end = At { block: frontier, child: child + 1, byte: 0 };
        let units = start.units + pieces.starts[child + 1] - pieces.starts[child];
        let to = self.normal(end);
        let (next, boundary) = match pieces.stops.binary_search_by_key(&end, |stop| stop.end) {
            Ok(index) => {
                (Cursor { at: pieces.stops[index].next, units: pieces.stops[index].after }, pieces.stops[index].class)
            }
            Err(_) if to.edge() => (Cursor::block(to.block), Boundary::Paragraph),
            Err(_) => (Cursor { at: to, units }, Boundary::Grapheme),
        };
        Some(Plan { to, next, boundary, kind, oversized: true })
    }

    fn descend(&self, from: At, block: usize, start: Cursor, kind: Option<MediaKind>, first: bool) -> Descent {
        let pieces = self.pieces(block);
        let cap = self.cap(kind);
        let stops = &pieces.stops[pieces.stops.partition_point(|stop| stop.end <= start.at)..];
        let count = stops.partition_point(|stop| stop.before.saturating_sub(start.units) <= cap);
        if let Some(found) = furthest(count, |index| self.fits(from, stops[index].end, kind, first)) {
            let chosen = stops[..=found].iter().rposition(|stop| stop.class == Boundary::Sentence).unwrap_or(found);
            let stop = stops[chosen];
            let next = Cursor { at: stop.next, units: stop.after };
            return Descent::Cut(Plan { to: stop.end, next, boundary: stop.class, kind, oversized: false });
        }
        let children = &self.body[block].children;
        let end = stops.first().map_or(At { block, child: children.len(), byte: 0 }, |stop| stop.end);
        let positions = self.positions(start, end, cap);
        if let Some(found) = furthest(positions.len(), |index| self.fits(from, positions[index].at, kind, first)) {
            let next = positions[found];
            return Descent::Cut(Plan { to: next.at, next, boundary: Boundary::Grapheme, kind, oversized: false });
        }
        match children[start.at.child].kind {
            Kind::Text { .. } | Kind::Break => Descent::Stuck,
            _ => Descent::Oversized(start.at.child),
        }
    }

    fn positions(&self, start: Cursor, end: At, cap: usize) -> Vec<Cursor> {
        let block = start.at.block;
        let pieces = self.pieces(block);
        let children = &self.body[block].children;
        let mut positions = vec![];
        let mut units = start.units;
        let (mut child, mut byte) = (start.at.child, start.at.byte);
        while child < children.len() {
            match &children[child].kind {
                Kind::Text { value } => {
                    for cluster in clusters(&value[byte..]) {
                        byte += cluster.len();
                        units += cluster.chars().count();
                        let at = self.normal(At { block, child, byte });
                        if units - start.units > cap || at >= end {
                            return positions;
                        }
                        if !cluster.ends_with(char::is_whitespace) {
                            positions.push(Cursor { at, units });
                        }
                    }
                }
                Kind::Break => units += 1,
                _ => {
                    units += pieces.starts[child + 1] - pieces.starts[child];
                    let at = self.normal(At { block, child: child + 1, byte: 0 });
                    if units - start.units > cap || at >= end {
                        return positions;
                    }
                    positions.push(Cursor { at, units });
                }
            }
            (child, byte) = (child + 1, 0);
        }
        positions
    }

    fn fits(&self, from: At, to: At, kind: Option<MediaKind>, first: bool) -> bool {
        let rendered = if (from.edge() && to.edge()) || from.block == to.block {
            self.draw(&self.nodes(from, to), None, first)
        } else {
            let mut head = match self.head.take() {
                Some((key, nodes)) if key == (from, to.block) => nodes,
                _ => self.head(from, to.block),
            };
            let rendered = if to.edge() {
                self.draw(&head, None, first)
            } else {
                head.push(self.fragment(to.block, (0, 0), (to.child, to.byte)));
                let rendered = self.draw(&head, None, first);
                head.pop();
                rendered
            };
            self.head.replace(Some(((from, to.block), head)));
            rendered
        };
        let fits = self.within(&rendered, kind);
        if fits {
            self.admitted.replace(Some(((from, to, kind, first), rendered)));
        }
        fits
    }

    fn fragment(&self, block: usize, from: (usize, usize), to: (usize, usize)) -> Node {
        let paragraph = &self.body[block];
        let mut children = vec![];
        for (index, child) in paragraph.children.iter().enumerate().skip(from.0) {
            if index > to.0 || (index == to.0 && to.1 == 0) {
                break;
            }
            let start = if index == from.0 { from.1 } else { 0 };
            match &child.kind {
                Kind::Text { value } if start > 0 || index == to.0 => {
                    let end = if index == to.0 { to.1 } else { value.len() };
                    children.push(Node {
                        kind: Kind::Text { value: value[start..end].to_owned() },
                        range: child.range,
                        provenance: child.provenance,
                        gap_before: child.gap_before,
                        children: vec![],
                    });
                }
                _ => children.push(child.clone()),
            }
        }
        Node {
            kind: Kind::Paragraph,
            range: paragraph.range,
            provenance: paragraph.provenance,
            gap_before: paragraph.gap_before,
            children,
        }
    }

    fn head(&self, from: At, block: usize) -> Vec<Node> {
        let mut nodes = vec![];
        let mut whole = from.block;
        if !from.edge() {
            let end = self.body[from.block].children.len();
            nodes.push(self.fragment(from.block, (from.child, from.byte), (end, 0)));
            whole += 1;
        }
        nodes.extend_from_slice(&self.body[whole..block]);
        nodes
    }

    fn nodes(&self, from: At, to: At) -> Cow<'s, [Node]> {
        if from.edge() && to.edge() {
            return Cow::Borrowed(&self.body[from.block..to.block]);
        }
        let ending = (to.child, to.byte);
        if from.block == to.block {
            return Cow::Owned(vec![self.fragment(from.block, (from.child, from.byte), ending)]);
        }
        let mut nodes = self.head(from, to.block);
        if !to.edge() {
            nodes.push(self.fragment(to.block, (0, 0), ending));
        }
        Cow::Owned(nodes)
    }

    fn ids(&self, from: At, to: At) -> Vec<String> {
        let mut ids = vec![];
        super::media(&self.body[from.block..to.block.min(self.body.len())], &mut ids);
        ids
    }

    fn fitted(&self, from: At, plan: &Plan, first: bool) -> Option<(Fitted, Option<Change>)> {
        if !plan.oversized {
            let rendered = match self.admitted.take() {
                Some((key, rendered)) if key == (from, plan.to, plan.kind, first) => rendered,
                _ => self.draw(&self.nodes(from, plan.to), None, first),
            };
            let bounds = &self.limits(plan.kind).bounds;
            let values = self.request.values(bounds, &rendered);
            return Some((Fitted { measurements: self.request.measurements(bounds, &values), rendered }, None));
        }
        let nodes = self.nodes(from, plan.to);
        let facts = Facts { media: plan.kind, ..*self.request.facts };
        let request = Request { facts: &facts, ..*self.request };
        let math = request.target.form.math().is_some();
        request.fit_part(&|| Outline::of_nodes(self.document, &nodes, math), &|cut| self.draw(&nodes, cut, first))
    }

    fn leaf(&self, at: At, node: &Node, value: &str) -> u32 {
        let mut leaves = self.leaves.borrow_mut();
        let leaf = leaves.entry((at.block, at.child)).or_insert_with(|| {
            let source = self.document.source(node.range);
            let exact = source == value;
            let (ours, theirs) = if exact { (vec![], vec![]) } else { (runs(value), runs(source)) };
            let runs = (!exact && ours.len() == theirs.len())
                .then(|| ours.iter().zip(&theirs).map(|(ours, theirs)| (ours.0, ours.1, theirs.2, theirs.3)).collect());
            Leaf { exact, runs, reached: (0, 0) }
        });
        let start = node.range.start;
        if leaf.exact {
            let (byte, units) = if leaf.reached.0 <= at.byte { leaf.reached } else { (0, 0) };
            let units = units + value[byte..at.byte].encode_utf16().count() as u32;
            leaf.reached = (at.byte, units);
            return start + units;
        }
        let Some(runs) = &leaf.runs else { return start };
        let index = runs.partition_point(|run| run.0 <= at.byte);
        match index.checked_sub(1).map(|index| runs[index]) {
            Some(run) if run.0 == at.byte || at.byte < run.1 => start + run.2,
            Some(run) => start + run.3,
            None => start,
        }
    }

    fn locate(&self, at: At) -> u32 {
        let paragraph = &self.body[at.block];
        let Some(node) = paragraph.children.get(at.child) else { return paragraph.range.end };
        match &node.kind {
            Kind::Text { value } if at.byte > 0 => self.leaf(at, node, value),
            _ => node.range.start,
        }
    }

    fn range(&self, from: At, to: At) -> Range {
        let start = if from.edge() { self.body[from.block].range.start } else { self.locate(from) };
        let end = match to {
            _ if to == from => start,
            _ if to.edge() => self.body[to.block - 1].range.end,
            _ => self.locate(to),
        };
        Range { start, end: end.max(start) }
    }

    fn assemble(&self, mut entries: Vec<Entry>) -> Option<Split> {
        if entries.len() < 2 {
            return None;
        }
        if let Some(last) = entries.last_mut() {
            last.boundary = None;
        }
        let mut split = Split { parts: vec![], ranges: vec![], boundaries: vec![], media: vec![] };
        let mut truncations = vec![];
        for entry in entries {
            let range = self.range(entry.from, entry.to);
            if let Some(mut change) = entry.truncated {
                let dropped = change.range.map_or(range.start, |dropped| dropped.start);
                change.range = Some(Range { start: dropped.clamp(range.start, range.end), end: range.end });
                truncations.push(change);
            }
            split.parts.push(entry.fitted);
            split.ranges.push(range);
            split.boundaries.push(entry.boundary);
            split.media.push(entry.media);
        }
        split.parts[0].rendered.changes = truncations;
        Some(split)
    }
}
