use std::fmt::Write;

use gsx_core::document::{Align, Document, Kind, Node, Provenance};

struct Html {
    out: String,
}

impl Html {
    fn cr(&mut self) {
        if !self.out.is_empty() && !self.out.ends_with('\n') {
            self.out.push('\n');
        }
    }

    fn escape(&mut self, value: &str, attribute: bool) {
        for char in value.chars() {
            match char {
                '&' => self.out.push_str("&amp;"),
                '<' => self.out.push_str("&lt;"),
                '>' => self.out.push_str("&gt;"),
                '"' if attribute => self.out.push_str("&quot;"),
                _ => self.out.push(char),
            }
        }
    }

    fn text(&mut self, value: &str) {
        self.escape(value, false);
    }

    fn attribute(&mut self, value: &str) {
        self.escape(value, true);
    }

    fn url(&mut self, url: &str) {
        let mut encoded = String::new();
        let bytes = url.as_bytes();
        for (index, &byte) in bytes.iter().enumerate() {
            let escape = byte == b'%'
                && bytes.get(index + 1..index + 3).is_some_and(|hex| hex.iter().all(u8::is_ascii_hexdigit));
            if byte.is_ascii_alphanumeric() || b";/?:@&=+$,-_.!~*'()#".contains(&byte) || escape {
                encoded.push(byte as char);
            } else {
                write!(encoded, "%{byte:02X}").unwrap();
            }
        }
        self.attribute(&encoded);
    }

    fn children(&mut self, node: &Node, tight: bool) {
        for child in &node.children {
            self.node(child, tight);
        }
    }

    fn node(&mut self, node: &Node, tight: bool) {
        match &node.kind {
            Kind::Paragraph if tight => self.children(node, false),
            Kind::Paragraph => {
                self.cr();
                self.out.push_str("<p>");
                self.children(node, false);
                self.out.push_str("</p>");
                self.cr();
            }
            Kind::Heading { depth } => {
                self.cr();
                write!(self.out, "<h{depth}>").unwrap();
                self.children(node, false);
                write!(self.out, "</h{depth}>").unwrap();
                self.cr();
            }
            Kind::ThematicBreak => {
                self.cr();
                self.out.push_str("<hr />");
                self.cr();
            }
            Kind::Code { lang, value, .. } => {
                self.cr();
                self.out.push_str("<pre><code");
                if let Some(lang) = lang {
                    self.out.push_str(" class=\"language-");
                    self.attribute(lang);
                    self.out.push('"');
                }
                self.out.push('>');
                self.text(value);
                if !value.is_empty() {
                    self.out.push('\n');
                }
                self.out.push_str("</code></pre>");
                self.cr();
            }
            Kind::Blockquote => {
                self.cr();
                self.out.push_str("<blockquote>");
                self.cr();
                self.children(node, false);
                self.cr();
                self.out.push_str("</blockquote>");
                self.cr();
            }
            Kind::List { ordered, start, spread } => {
                self.cr();
                match (ordered, start) {
                    (true, Some(start)) if *start != 1 => write!(self.out, "<ol start=\"{start}\">").unwrap(),
                    (true, _) => self.out.push_str("<ol>"),
                    (false, _) => self.out.push_str("<ul>"),
                }
                self.cr();
                let loose = *spread
                    || node.children.iter().any(|item| matches!(item.kind, Kind::ListItem { spread: true, .. }));
                for item in &node.children {
                    self.out.push_str("<li>");
                    if let Kind::ListItem { checked: Some(checked), .. } = item.kind {
                        self.out.push_str("<input disabled=\"\" type=\"checkbox\"");
                        self.out.push_str(if checked { " checked=\"\"/>\n" } else { "/>\n" });
                    }
                    self.children(item, !loose);
                    let last = item.children.last();
                    if (loose && last.is_some()) || last.is_some_and(|last| last.kind != Kind::Paragraph) {
                        self.cr();
                    }
                    self.out.push_str("</li>");
                    self.cr();
                }
                self.out.push_str(if *ordered { "</ol>" } else { "</ul>" });
                self.cr();
            }
            Kind::Table { align } => {
                self.cr();
                self.out.push_str("<table>\n<thead>\n");
                for (index, row) in node.children.iter().enumerate() {
                    if index == 1 {
                        self.out.push_str("<tbody>\n");
                    }
                    self.out.push_str("<tr>\n");
                    let tag = if index == 0 { "th" } else { "td" };
                    for (column, align) in align.iter().enumerate() {
                        match align {
                            Align::None => write!(self.out, "<{tag}>").unwrap(),
                            Align::Left => write!(self.out, "<{tag} style=\"text-align: left\">").unwrap(),
                            Align::Right => write!(self.out, "<{tag} style=\"text-align: right\">").unwrap(),
                            Align::Center => write!(self.out, "<{tag} style=\"text-align: center\">").unwrap(),
                        }
                        if let Some(cell) = row.children.get(column) {
                            self.children(cell, false);
                        }
                        writeln!(self.out, "</{tag}>").unwrap();
                    }
                    self.out.push_str("</tr>\n");
                    if index == 0 {
                        self.out.push_str("</thead>\n");
                    }
                }
                if node.children.len() > 1 {
                    self.out.push_str("</tbody>\n");
                } else {
                    self.out.push_str("<tbody></tbody>\n");
                }
                self.out.push_str("</table>\n");
            }
            Kind::Text { value } => self.text(value),
            Kind::Emphasis | Kind::Strong | Kind::Delete | Kind::Underline | Kind::Highlight | Kind::Spoiler => {
                let tag = match node.kind {
                    Kind::Emphasis => "em",
                    Kind::Strong => "strong",
                    Kind::Delete => "del",
                    Kind::Underline => "u",
                    Kind::Highlight => "mark",
                    _ => "span",
                };
                write!(self.out, "<{tag}>").unwrap();
                self.children(node, false);
                write!(self.out, "</{tag}>").unwrap();
            }
            Kind::InlineCode { value } => {
                self.out.push_str("<code>");
                self.text(value);
                self.out.push_str("</code>");
            }
            Kind::Break => self.out.push_str("<br />\n"),
            Kind::Link { url, title, .. } => {
                self.out.push_str("<a href=\"");
                self.url(url);
                self.out.push('"');
                if let Some(title) = title {
                    self.out.push_str(" title=\"");
                    self.attribute(title);
                    self.out.push('"');
                }
                self.out.push('>');
                self.children(node, false);
                self.out.push_str("</a>");
            }
            _ => self.children(node, false),
        }
        if node.provenance == Provenance::Literal && !matches!(node.kind, Kind::Text { .. }) {
            unreachable!("literal nodes are text");
        }
    }
}

pub fn render(document: &Document) -> String {
    let mut html = Html { out: String::new() };
    for node in document.children() {
        html.node(node, false);
    }
    html.cr();
    html.out
}
