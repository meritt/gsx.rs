use super::{label, language, media, underlined, wrapper};
use crate::document::{Align, Kind, Layout, Node, Range};
use crate::measure::{self, Field, Placement};
use crate::project::{
    CELL, Change, Class, Context, Feature, FieldName, Format, HtmlBlocks, HtmlForm, RULE, Renderer, Slot, Truncated,
    When, blank, date_refusal, href, marker, placed, references, shortened,
};

pub const RENDERER: Renderer = Renderer { blocks: render, inline: render_inline };

struct Element {
    start: String,
    end: String,
    feature: Option<Feature>,
    range: Range,
    exclusive: bool,
}

#[derive(Default)]
struct Counted {
    blocks: usize,
    blocks_reported: bool,
    depth_reported: bool,
    columns_reported: bool,
}

struct Html {
    form: &'static HtmlForm,
    open: Vec<Element>,
    quoted: u32,
    segment: Option<usize>,
    counted: Counted,
    media: Vec<Medium>,
    figures: Vec<Figure>,
}

struct Medium {
    id: String,
    format: Format,
    alt: Option<String>,
    spoiler: bool,
}

struct Figure {
    start: usize,
    end: usize,
    group: bool,
    cut: bool,
    range: Option<Range>,
    alts: Vec<(usize, Option<String>)>,
}

fn form(context: &Context) -> &'static HtmlForm {
    context.target.form.html().expect("the html renderer runs html targets")
}

fn render(context: &mut Context, nodes: &[Node]) {
    let mut html = Html::new(form(context));
    match html.form.blocks {
        HtmlBlocks::Message => html.blocks(context, nodes),
        HtmlBlocks::Rich => {
            html.load(context, nodes);
            html.rich(context, nodes, 1);
            html.captions(context);
        }
    }
    html.finish(context);
}

fn render_inline(context: &mut Context, nodes: &[Node]) {
    let mut html = Html::new(form(context));
    html.inline(context, nodes);
    html.finish(context);
}

fn attribute(value: &str) -> String {
    escape(value).replace('"', "&quot;")
}

pub(super) fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn formatting(nodes: &[Node], decorated: bool) -> usize {
    nodes
        .iter()
        .map(|node| match node.kind {
            Kind::InlineCode { .. } => 1,
            Kind::Link { .. } => {
                1 + usize::from(decorated && !underlined(&node.children)) + formatting(&node.children, decorated)
            }
            Kind::Underline => 1 + formatting(&node.children, false),
            Kind::Datetime { .. } => 1 + formatting(&node.children, decorated),
            ref kind if wrapper(kind).is_some() => 1 + formatting(&node.children, decorated),
            _ => 0,
        })
        .max()
        .unwrap_or(0)
}

fn without_empty(form: &HtmlForm, text: &str) -> String {
    let removable = |name: &str| form.tags.iter().any(|&(feature, tag)| tag == name && feature != Feature::InlineCode);
    let mut out = String::with_capacity(text.len());
    let mut open: Vec<(&str, usize, usize)> = vec![];
    let mut rest = text;
    while let Some(at) = rest.find('<') {
        out.push_str(&rest[..at]);
        let end = rest[at..].find('>').map_or(rest.len(), |end| at + end + 1);
        let tag = &rest[at..end];
        rest = &rest[end..];
        let name = tag.trim_start_matches(['<', '/']).split([' ', '>']).next().unwrap_or("");
        if !removable(name) {
            out.push_str(tag);
        } else if tag.starts_with("</") {
            match open.pop() {
                Some((_, start, after)) if out.len() == after => out.truncate(start),
                _ => out.push_str(tag),
            }
        } else {
            open.push((name, out.len(), out.len() + tag.len()));
            out.push_str(tag);
        }
    }
    out.push_str(rest);
    out
}

impl Html {
    fn new(form: &'static HtmlForm) -> Self {
        Html {
            form,
            open: vec![],
            quoted: 0,
            segment: None,
            counted: Counted::default(),
            media: vec![],
            figures: vec![],
        }
    }

    fn finish(&self, context: &mut Context) {
        let form = self.form;
        context.writer().edit(|text| {
            if form.empty_removed {
                *text = without_empty(form, text);
            }
            if form.trim {
                let end = text.trim_end_matches(blank).len();
                text.truncate(end);
                let lead = text.len() - text.trim_start_matches(blank).len();
                text.replace_range(..lead, "");
            }
        });
    }

    fn blocks(&mut self, context: &mut Context, nodes: &[Node]) {
        let mut first = true;
        for node in nodes {
            if media(node) {
                context.report_media(node);
                continue;
            }
            if context.done() {
                continue;
            }
            if !first {
                context.writer().newlines(node.gap_before.unwrap_or(1));
            }
            first = false;
            self.block(context, node);
        }
    }

    fn block(&mut self, context: &mut Context, node: &Node) {
        match &node.kind {
            Kind::Paragraph => self.inline(context, &node.children),
            Kind::Heading { .. } => {
                context.report(Feature::Heading, node.range);
                self.heading(context, node);
            }
            Kind::ThematicBreak => {
                context.report(Feature::ThematicBreak, node.range);
                context.writer().push(RULE);
            }
            Kind::Code { lang, meta, value } => self.code_block(context, node, lang.as_deref(), meta.is_some(), value),
            Kind::Blockquote => self.blockquote(context, node),
            Kind::List { ordered, start, .. } => {
                context.report(if *ordered { Feature::OrderedList } else { Feature::BulletList }, node.range);
                if node.children.iter().any(|item| item.children.len() > 1) {
                    context.report(Feature::CompoundList, node.range);
                }
                if node.children.iter().any(|item| matches!(item.kind, Kind::ListItem { checked: Some(_), .. })) {
                    context.report(Feature::TaskList, node.range);
                }
                for (index, item) in node.children.iter().enumerate() {
                    if context.done() {
                        break;
                    }
                    if index > 0 {
                        context.writer().push("\n");
                    }
                    let checked = match item.kind {
                        Kind::ListItem { checked, .. } => checked,
                        _ => None,
                    };
                    context.writer().open_item(marker(*ordered, *start, index, checked));
                    self.blocks(context, &item.children);
                    context.writer().close();
                }
            }
            Kind::Table { .. } => {
                context.report(Feature::Table, node.range);
                for (index, row) in node.children.iter().enumerate() {
                    if context.done() {
                        break;
                    }
                    if index > 0 {
                        context.writer().push("\n");
                    }
                    for (index, cell) in row.children.iter().enumerate() {
                        if index > 0 {
                            context.writer().push(CELL);
                        }
                        if !context.done() {
                            self.inline(context, &cell.children);
                        }
                    }
                }
            }
            _ => self.inline(context, std::slice::from_ref(node)),
        }
    }

    fn heading(&mut self, context: &mut Context, node: &Node) {
        let Some(tag) = self.form.tag(Feature::Heading) else {
            return self.inline(context, &node.children);
        };
        let (start, end) = (format!("<{tag}>"), format!("</{tag}>"));
        let element = Element { start, end, feature: None, range: node.range, exclusive: false };
        self.wrapped(context, element, &node.children);
    }

    fn blockquote(&mut self, context: &mut Context, node: &Node) {
        if self.quoted > 0 && !self.form.quote_nests {
            context.report_when(Feature::Blockquote, Some(node.range), Some(When::Nested));
            self.quoted += 1;
            self.blocks(context, &node.children);
            self.quoted -= 1;
            return;
        }
        self.quoted += 1;
        if self.form.quote_contains_code {
            context.writer().raw("<blockquote>");
            self.blocks(context, &node.children);
            context.writer().raw("</blockquote>");
        } else {
            context.writer().markers();
            self.segment = Some(context.writer().mark());
            self.blocks(context, &node.children);
            self.close_segment(context);
        }
        self.quoted -= 1;
    }

    fn close_segment(&mut self, context: &mut Context) {
        let Some(start) = self.segment.take() else { return };
        let writer = context.writer();
        let written = writer.since(start);
        let Some(lead) = written.find(|char| !blank(char)) else { return };
        let trail = written.trim_end_matches(blank).len();
        writer.insert(start + trail, "</blockquote>");
        writer.insert(start + lead, "<blockquote>");
    }

    fn code_block(&mut self, context: &mut Context, node: &Node, lang: Option<&str>, meta: bool, value: &str) {
        let quoted = self.quoted > 0 && !self.form.quote_contains_code;
        if quoted {
            context.report(Feature::Blockquote, node.range);
            self.close_segment(context);
        }
        let language = language(lang);
        if meta || (lang.is_some() && (language.is_empty() || !self.form.code_info)) {
            context.report_when(Feature::CodeBlock, Some(node.range), Some(When::Info));
        }
        let open = if self.form.code_info && !language.is_empty() {
            format!("<pre><code class=\"language-{}\">", attribute(language))
        } else {
            "<pre>".to_owned()
        };
        let close = if open.len() > "<pre>".len() { "</code></pre>" } else { "</pre>" };
        context.writer().raw(&open);
        context.writer().verbatim(true);
        context.leaf(node, value);
        context.writer().verbatim(false);
        context.writer().raw(close);
        if quoted {
            self.segment = Some(context.writer().mark());
        }
    }

    fn inline(&mut self, context: &mut Context, nodes: &[Node]) {
        for node in nodes {
            if context.done() {
                break;
            }
            match &node.kind {
                Kind::Text { value } => context.leaf(node, value),
                Kind::Break => context.writer().push("\n"),
                Kind::InlineCode { value } => self.code(context, node, value),
                Kind::Link { url, title, .. } => {
                    if title.is_some() {
                        context.report(Feature::LinkTitle, node.range);
                    }
                    let open = format!("<a href=\"{}\">", attribute(&href(url)));
                    self.element(context, node, Feature::Link, open);
                }
                Kind::Datetime { unix, .. } => {
                    let open = self.form.tag(Feature::Datetime).map(|tag| format!("<{tag} unix=\"{unix}\">"));
                    let refusal = open.as_ref().and_then(|_| {
                        let mut text = String::new();
                        label(&node.children, &mut text);
                        date_refusal(&text, *unix)
                    });
                    match refusal {
                        Some(when) => {
                            context.report_when(Feature::Datetime, Some(node.range), Some(when));
                            self.inline(context, &node.children);
                        }
                        None => self.element_or_fallback(context, node, Feature::Datetime, open),
                    }
                }
                kind => match wrapper(kind) {
                    Some(feature) => {
                        let open = self.form.tag(feature).map(|tag| format!("<{tag}>"));
                        self.element_or_fallback(context, node, feature, open);
                    }
                    None => self.inline(context, &node.children),
                },
            }
        }
    }

    fn element_or_fallback(&mut self, context: &mut Context, node: &Node, feature: Feature, start: Option<String>) {
        if let Some(start) = start {
            self.element(context, node, feature, start);
            return;
        }
        context.report(feature, node.range);
        if context.class(feature) != Class::Omitted {
            self.inline(context, &node.children);
        }
    }

    fn element(&mut self, context: &mut Context, node: &Node, feature: Feature, start: String) {
        let exclusive = self.form.exclusive.contains(&feature);
        if exclusive && self.open.iter().any(|element| element.exclusive) {
            context.report_when(feature, Some(node.range), Some(When::Nested));
            return self.inline(context, &node.children);
        }
        let underline = self.form.tag(Feature::Underline).filter(|_| {
            feature == Feature::Link
                && !self.open.iter().any(|element| element.feature == Some(Feature::Underline))
                && !underlined(&node.children)
        });
        if let Some(tag) = underline {
            let start = format!("<{tag}>");
            context.writer().raw(&start);
            let end = format!("</{tag}>");
            self.open.push(Element { start, end, feature: None, range: node.range, exclusive: false });
        }
        let name = start[1..].split([' ', '>']).next().unwrap_or("");
        let end = format!("</{name}>");
        let element = Element { start, end, feature: Some(feature), range: node.range, exclusive };
        if !self.wrapped(context, element, &node.children) && feature == Feature::Link {
            context.report_when(feature, Some(node.range), Some(When::LabelInvisible));
        }
        if underline.is_some() {
            let element = self.open.pop().expect("the underline pushed above");
            context.writer().raw(&element.end);
        }
    }

    fn wrapped(&mut self, context: &mut Context, element: Element, children: &[Node]) -> bool {
        context.writer().raw(&element.start);
        let content = context.writer().mark();
        self.open.push(element);
        self.inline(context, children);
        let element = self.open.pop().expect("the element pushed above");
        let written = context.writer().mark() > content;
        context.writer().raw(&element.end);
        written
    }

    fn code(&mut self, context: &mut Context, node: &Node, value: &str) {
        let Some(tag) = self.form.tag(Feature::InlineCode) else {
            context.report(Feature::InlineCode, node.range);
            return context.leaf(node, value);
        };
        let split = !self.form.code_contained && !self.open.is_empty();
        if split {
            for element in &self.open {
                if let Some(feature) = element.feature {
                    context.report(feature, element.range);
                }
            }
            for element in self.open.iter().rev() {
                context.writer().raw(&element.end);
            }
        }
        context.writer().raw(&format!("<{tag}>"));
        context.leaf(node, value);
        context.writer().raw(&format!("</{tag}>"));
        if split {
            for element in &self.open {
                context.writer().raw(&element.start);
            }
        }
    }

    fn count(&mut self, context: &mut Context, range: impl Into<Option<Range>>, depth: usize) {
        let Some(limits) = self.form.structure else { return };
        let range = range.into();
        self.counted.blocks += 1;
        if self.counted.blocks > limits.blocks && !self.counted.blocks_reported {
            self.counted.blocks_reported = true;
            context.report_when(Feature::Text, range, Some(When::Blocks));
        }
        self.deepen(context, range, depth);
    }

    fn limit(&self) -> usize {
        self.form.structure.map_or(usize::MAX, |structure| structure.items)
    }

    fn load(&mut self, context: &mut Context, nodes: &[Node]) {
        self.media = context
            .items()
            .iter()
            .map(|fact| Medium {
                id: fact.id.to_owned(),
                format: fact.format,
                alt: fact.alt.map(str::to_owned),
                spoiler: fact.spoiler,
            })
            .collect();
        if self.media.len() > self.limit() {
            let ranges = references(nodes);
            for medium in self.media.iter().skip(self.limit()) {
                let range = ranges.get(medium.id.as_str()).map(|&top| nodes[top].range);
                context.report_when(Feature::Media, range, Some(When::Count));
            }
        }
    }

    fn resolve(&self, context: &mut Context, node: &Node, ids: &[&str]) -> Vec<usize> {
        let mut kept = vec![];
        let mut missing = false;
        for id in ids {
            match context.position(id) {
                Some(index) if index < self.limit() => kept.push(index),
                Some(_) => {}
                None => missing = true,
            }
        }
        if missing {
            context.report_when(Feature::Media, Some(node.range), Some(When::Missing));
        }
        kept
    }

    fn medium(&self, index: usize) -> String {
        let medium = &self.media[index];
        let src = attribute(&format!("media:{}", medium.id));
        let hidden = if medium.spoiler { " tg-spoiler" } else { "" };
        match medium.format {
            Format::Image => format!("<img src=\"{src}\"{hidden}/>"),
            Format::Gif | Format::Video => format!("<video src=\"{src}\"{hidden}></video>"),
            Format::Audio => format!("<audio src=\"{src}\"></audio>"),
        }
    }

    fn single(&mut self, context: &mut Context, index: usize, range: Option<Range>, level: usize) {
        self.count(context, range, level);
        let start = context.writer().mark();
        let markup = self.medium(index);
        context.writer().raw(&markup);
        let end = context.writer().mark();
        let alts = vec![(index, self.media[index].alt.clone())];
        self.figures.push(Figure { start, end, group: false, cut: context.done(), range, alts });
    }

    fn place(&mut self, context: &mut Context, node: &Node, indices: &[usize], level: usize) {
        if let Kind::MediaGroup { layout, .. } = node.kind
            && indices.len() > 1
        {
            let formats = self.form.groups(layout);
            let grouped = indices.iter().all(|&index| formats.contains(&self.media[index].format));
            if grouped {
                let name = match layout {
                    Layout::Collage => "tg-collage",
                    Layout::Slideshow => "tg-slideshow",
                };
                self.count(context, node.range, level);
                let start = context.writer().mark();
                context.writer().raw(&format!("<{name}>"));
                for &index in indices {
                    self.count(context, node.range, level + 1);
                    let markup = self.medium(index);
                    context.writer().raw(&markup);
                }
                let end = context.writer().mark();
                context.writer().raw(&format!("</{name}>"));
                let alts = indices.iter().map(|&index| (index, self.media[index].alt.clone())).collect();
                self.figures.push(Figure {
                    start,
                    end,
                    group: true,
                    cut: context.done(),
                    range: Some(node.range),
                    alts,
                });
                return;
            }
            context.report_when(Feature::Media, Some(node.range), Some(When::Group));
        }
        for (position, &index) in indices.iter().enumerate() {
            if position > 0 {
                context.writer().raw("<br>");
            }
            self.single(context, index, Some(node.range), level);
        }
    }

    fn captions(&mut self, context: &mut Context) {
        if self.figures.is_empty() {
            return;
        }
        let facts = measure::Facts::default();
        let limit =
            context.target.applicable(Field::Text, measure::Context { placement: Placement::Post, media: None }).next();
        let cost = |text: &str| limit.map_or(0, |limit| limit.measure(text, &facts).value as usize);
        let maximum = limit.and_then(|limit| limit.maximum(&facts)).map_or(usize::MAX, |maximum| maximum as usize);
        let body = cost(&context.target.form.visible(context.writer().since(0)));
        let Some(mut remaining) = maximum.checked_sub(body) else { return };
        let cut = context.done();
        let mut inserts = vec![];
        let mut kept = 0;
        for figure in std::mem::take(&mut self.figures) {
            let before = if cut && !figure.cut {
                Some(cost(&context.target.form.visible(&context.writer().since(0)[..figure.end])))
            } else {
                None
            };
            let mut entries: Vec<String> = vec![];
            for (index, (item, alt)) in figure.alts.iter().enumerate() {
                let Some(alt) = alt.as_deref().filter(|alt| !alt.trim().is_empty()) else { continue };
                let prefix = if figure.group { format!("{}. ", index + 1) } else { String::new() };
                let separator = if entries.is_empty() { "" } else { "\n" };
                let overhead = cost(&format!("{separator}{prefix}"));
                let room = remaining.saturating_sub(overhead);
                let value = cost(alt);
                let whole = before.is_some_and(|before| {
                    before.saturating_add(kept).saturating_add(overhead).saturating_add(value) <= maximum
                });
                let text = if whole || value <= room {
                    alt.to_owned()
                } else if room == 0 {
                    String::new()
                } else {
                    shortened(alt, |candidate| cost(candidate) <= room)
                };
                if text != alt {
                    let when = if text.is_empty() { When::Empty } else { When::Truncated };
                    let (class, reason) = context.target.rule(Feature::Media).outcome(Some(when));
                    let unit = limit.map_or(measure::Unit::CodePoints, |limit| limit.unit);
                    context.record(Change {
                        class,
                        feature: Feature::Media,
                        reason,
                        field: FieldName::Alt,
                        range: figure.range,
                        truncated: Some(Truncated { unit, maximum: room as u32, value: value as u32, exact: false }),
                        slot: Some(Slot { block: None, item: *item as u32 }),
                    });
                    context.published_alt(*item, Some(text.clone()).filter(|text| !text.is_empty()));
                }
                if !text.is_empty() {
                    let spent = overhead + cost(&text);
                    remaining = remaining.saturating_sub(spent);
                    kept += spent;
                    entries.push(prefix + &text);
                }
            }
            if entries.is_empty() {
                continue;
            }
            let caption = format!("<figcaption>{}</figcaption>", escape(&entries.join("\n")).replace('\n', "<br>"));
            if figure.group {
                inserts.push((figure.end, caption));
            } else {
                inserts.push((figure.start, "<figure>".to_owned()));
                inserts.push((figure.end, caption + "</figure>"));
            }
        }
        for (at, markup) in inserts.into_iter().rev() {
            context.writer().insert(at, &markup);
        }
    }

    fn rich(&mut self, context: &mut Context, nodes: &[Node], level: usize) {
        let mut first = true;
        let mut paragraph = false;
        let mut carry = 0;
        if level == 1 {
            let placed_ids = references(nodes);
            let unplaced: Vec<usize> = (0..self.media.len().min(self.limit()))
                .filter(|&index| !placed_ids.contains_key(self.media[index].id.as_str()))
                .collect();
            for index in unplaced {
                if !first {
                    context.writer().raw("<br>");
                }
                first = false;
                self.single(context, index, None, level);
            }
            carry = u32::from(!first);
        }
        for node in nodes {
            let ids = placed(node);
            if context.done() && ids.is_none() {
                continue;
            }
            let gap = carry.max(node.gap_before.unwrap_or(1));
            if let Some(ids) = ids {
                let indices = self.resolve(context, node, &ids);
                if indices.is_empty() {
                    carry = gap;
                    continue;
                }
                if paragraph {
                    context.writer().raw("</p>");
                    paragraph = false;
                }
                if !first {
                    (0..gap).for_each(|_| context.writer().raw("<br>"));
                }
                first = false;
                carry = 0;
                if context.done() {
                    context.report_when(Feature::Media, Some(node.range), Some(When::Cut));
                }
                self.place(context, node, &indices, level);
                continue;
            }
            let gap = if first { 0 } else { gap };
            first = false;
            carry = 0;
            let prose = matches!(node.kind, Kind::Paragraph | Kind::Heading { .. });
            if paragraph && !prose {
                context.writer().raw("</p>");
                paragraph = false;
            }
            for _ in 0..gap {
                context.writer().raw("<br>");
            }
            if prose {
                let heading = matches!(node.kind, Kind::Heading { .. });
                let decorated = self.form.tag(Feature::Underline).is_some();
                let depth = level + usize::from(heading) + formatting(&node.children, decorated);
                if paragraph {
                    self.deepen(context, node.range, depth);
                } else {
                    self.count(context, node.range, depth);
                    context.writer().raw("<p>");
                    paragraph = true;
                }
                if heading {
                    context.report(Feature::Heading, node.range);
                    self.heading(context, node);
                } else {
                    self.inline(context, &node.children);
                }
            } else {
                self.rich_block(context, node, level);
            }
        }
        if paragraph {
            context.writer().raw("</p>");
        }
    }

    fn deepen(&mut self, context: &mut Context, range: impl Into<Option<Range>>, depth: usize) {
        if let Some(limits) = self.form.structure
            && depth > limits.depth
            && !self.counted.depth_reported
        {
            self.counted.depth_reported = true;
            context.report_when(Feature::Text, range.into(), Some(When::Depth));
        }
    }

    fn rich_block(&mut self, context: &mut Context, node: &Node, level: usize) {
        match &node.kind {
            Kind::ThematicBreak => {
                self.count(context, node.range, level);
                context.writer().raw("<hr/>");
            }
            Kind::Blockquote => {
                self.count(context, node.range, level);
                context.writer().raw("<blockquote>");
                self.rich(context, &node.children, level + 1);
                context.writer().raw("</blockquote>");
            }
            Kind::Code { lang, meta, value } => {
                self.count(context, node.range, level);
                self.code_block(context, node, lang.as_deref(), meta.is_some(), value);
            }
            Kind::List { ordered, start, .. } => {
                self.count(context, node.range, level);
                let open = if *ordered { format!("<ol start=\"{}\">", start.unwrap_or(1)) } else { "<ul>".to_owned() };
                context.writer().raw(&open);
                for item in &node.children {
                    if context.done() {
                        break;
                    }
                    self.count(context, item.range, level + 1);
                    context.writer().raw("<li>");
                    match item.kind {
                        Kind::ListItem { checked: Some(true), .. } => {
                            context.writer().raw("<input type=\"checkbox\" checked>");
                        }
                        Kind::ListItem { checked: Some(false), .. } => {
                            context.writer().raw("<input type=\"checkbox\">");
                        }
                        _ => {}
                    }
                    self.rich(context, &item.children, level + 2);
                    context.writer().raw("</li>");
                }
                context.writer().raw(if *ordered { "</ol>" } else { "</ul>" });
            }
            Kind::Table { align } => {
                self.count(context, node.range, level);
                context.writer().raw("<table>");
                for (index, row) in node.children.iter().enumerate() {
                    if context.done() {
                        break;
                    }
                    let decorated = self.form.tag(Feature::Underline).is_some();
                    let formatted =
                        row.children.iter().map(|cell| formatting(&cell.children, decorated)).max().unwrap_or(0);
                    self.count(context, row.range, level + 1 + formatted);
                    if let Some(limits) = self.form.structure
                        && row.children.len() > limits.columns
                        && !self.counted.columns_reported
                    {
                        self.counted.columns_reported = true;
                        context.report_when(Feature::Table, Some(node.range), Some(When::Columns));
                    }
                    let tag = if index == 0 { "th" } else { "td" };
                    context.writer().raw("<tr>");
                    for (column, cell) in row.children.iter().enumerate() {
                        let align = match align.get(column) {
                            Some(Align::Left) => " align=\"left\"",
                            Some(Align::Right) => " align=\"right\"",
                            Some(Align::Center) => " align=\"center\"",
                            _ => "",
                        };
                        context.writer().raw(&format!("<{tag}{align}>"));
                        if !context.done() {
                            self.inline(context, &cell.children);
                        }
                        context.writer().raw(&format!("</{tag}>"));
                    }
                    context.writer().raw("</tr>");
                }
                context.writer().raw("</table>");
            }
            _ => {
                self.count(context, node.range, level);
                context.writer().verbatim(true);
                self.inline(context, std::slice::from_ref(node));
                context.writer().verbatim(false);
            }
        }
    }
}
