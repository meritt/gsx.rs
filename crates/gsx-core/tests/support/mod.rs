#![allow(dead_code)]

pub mod bench;
pub mod html;
pub mod markup;

use std::collections::{BTreeMap, HashMap};

pub const LONE_SURROGATE: char = '\u{10FFFF}';

use gsx_core::document::{Document, Node, parse};

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    Object(BTreeMap<String, Json>),
}

impl Json {
    pub fn parse(text: &str) -> Json {
        let mut parser = Parser { bytes: text.as_bytes(), at: 0 };
        let value = parser.value();
        parser.space();
        assert_eq!(parser.at, parser.bytes.len(), "trailing JSON");
        value
    }

    pub fn get(&self, key: &str) -> &Json {
        match self {
            Json::Object(map) => map.get(key).unwrap_or(&Json::Null),
            _ => &Json::Null,
        }
    }

    pub fn str(&self) -> Option<&str> {
        match self {
            Json::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn array(&self) -> &[Json] {
        match self {
            Json::Array(items) => items,
            _ => &[],
        }
    }
}

impl std::fmt::Display for Json {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Json::Null => formatter.write_str("null"),
            Json::Bool(value) => write!(formatter, "{value}"),
            Json::Number(value) => write!(formatter, "{value}"),
            Json::String(value) => write!(formatter, "{value:?}"),
            Json::Array(items) => {
                formatter.write_str("[")?;
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str(",")?;
                    }
                    write!(formatter, "{item}")?;
                }
                formatter.write_str("]")
            }
            Json::Object(map) => {
                formatter.write_str("{")?;
                for (index, (key, value)) in map.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str(",")?;
                    }
                    write!(formatter, "{key:?}:{value}")?;
                }
                formatter.write_str("}")
            }
        }
    }
}

struct Parser<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Parser<'_> {
    fn space(&mut self) {
        while self.bytes.get(self.at).is_some_and(u8::is_ascii_whitespace) {
            self.at += 1;
        }
    }

    fn eat(&mut self, byte: u8) {
        self.space();
        assert_eq!(self.bytes[self.at], byte, "JSON at {}", self.at);
        self.at += 1;
    }

    fn value(&mut self) -> Json {
        self.space();
        match self.bytes[self.at] {
            b'{' => {
                self.at += 1;
                let mut map = BTreeMap::new();
                self.space();
                if self.bytes[self.at] == b'}' {
                    self.at += 1;
                    return Json::Object(map);
                }
                loop {
                    self.space();
                    let Json::String(key) = self.value() else { panic!("JSON key") };
                    self.eat(b':');
                    map.insert(key, self.value());
                    self.space();
                    self.at += 1;
                    if self.bytes[self.at - 1] == b'}' {
                        return Json::Object(map);
                    }
                }
            }
            b'[' => {
                self.at += 1;
                let mut items = vec![];
                self.space();
                if self.bytes[self.at] == b']' {
                    self.at += 1;
                    return Json::Array(items);
                }
                loop {
                    items.push(self.value());
                    self.space();
                    self.at += 1;
                    if self.bytes[self.at - 1] == b']' {
                        return Json::Array(items);
                    }
                }
            }
            b'"' => Json::String(self.string()),
            b't' => self.word("true", Json::Bool(true)),
            b'f' => self.word("false", Json::Bool(false)),
            b'n' => self.word("null", Json::Null),
            _ => {
                let start = self.at;
                while self.bytes.get(self.at).is_some_and(|byte| b"+-.eE0123456789".contains(byte)) {
                    self.at += 1;
                }
                Json::Number(std::str::from_utf8(&self.bytes[start..self.at]).unwrap().parse().unwrap())
            }
        }
    }

    fn word(&mut self, word: &str, value: Json) -> Json {
        assert!(self.bytes[self.at..].starts_with(word.as_bytes()));
        self.at += word.len();
        value
    }

    fn hex(&mut self) -> u32 {
        let digits = std::str::from_utf8(&self.bytes[self.at..self.at + 4]).unwrap();
        self.at += 4;
        u32::from_str_radix(digits, 16).unwrap()
    }

    fn string(&mut self) -> String {
        self.at += 1;
        let mut out = String::new();
        loop {
            let start = self.at;
            while !matches!(self.bytes[self.at], b'"' | b'\\') {
                self.at += 1;
            }
            out.push_str(std::str::from_utf8(&self.bytes[start..self.at]).unwrap());
            let byte = self.bytes[self.at];
            self.at += 1;
            if byte == b'"' {
                return out;
            }
            let escape = self.bytes[self.at];
            self.at += 1;
            match escape {
                b'n' => out.push('\n'),
                b't' => out.push('\t'),
                b'r' => out.push('\r'),
                b'b' => out.push('\u{8}'),
                b'f' => out.push('\u{c}'),
                b'u' => {
                    let mut unit = self.hex();
                    let paired = self.bytes[self.at..].starts_with(b"\\u")
                        && std::str::from_utf8(&self.bytes[self.at + 2..self.at + 6])
                            .ok()
                            .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                            .is_some_and(|low| (0xDC00..0xE000).contains(&low));
                    if (0xD800..0xDC00).contains(&unit) && paired {
                        self.at += 2;
                        unit = 0x10000 + ((unit - 0xD800) << 10) + (self.hex() - 0xDC00);
                    }
                    out.push(char::from_u32(unit).unwrap_or(LONE_SURROGATE));
                }
                other => out.push(other as char),
            }
        }
    }
}

pub fn difference(path: &str, expected: &Json, actual: &Json) -> Option<String> {
    match (expected, actual) {
        (Json::Object(left), Json::Object(right)) => {
            for (key, value) in left {
                let next = format!("{path}.{key}");
                match right.get(key) {
                    Some(other) => {
                        if let Some(found) = difference(&next, value, other) {
                            return Some(found);
                        }
                    }
                    None => return Some(format!("{next}: missing, expected {value}")),
                }
            }
            right
                .keys()
                .find(|key| !left.contains_key(*key))
                .map(|key| format!("{path}.{key}: unexpected {}", right[key]))
        }
        (Json::Array(left), Json::Array(right)) => {
            for (index, (x, y)) in left.iter().zip(right).enumerate() {
                if let Some(found) = difference(&format!("{path}[{index}]"), x, y) {
                    return Some(found);
                }
            }
            (left.len() != right.len()).then(|| format!("{path}: length {} vs {}", left.len(), right.len()))
        }
        _ => (expected != actual).then(|| format!("{path}: expected {expected}, got {actual}")),
    }
}

pub fn records(file: &str) -> Vec<Json> {
    let path = format!("{}/../../testdata/corpus/{file}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(path)
        .unwrap()
        .split('\n')
        .skip(1)
        .filter(|line| !line.is_empty())
        .map(Json::parse)
        .collect()
}

pub fn documents() -> HashMap<String, Document> {
    records("document.jsonl")
        .iter()
        .filter_map(|case| {
            let markdown = case.get("markdown").str().unwrap();
            if markdown.contains(LONE_SURROGATE) || !case.get("media").array().is_empty() {
                return None;
            }
            Some((case.get("id").str().unwrap().to_owned(), parse(markdown, &[]).ok()?))
        })
        .collect()
}

pub fn any(nodes: &[Node], predicate: &dyn Fn(&Node) -> bool) -> bool {
    nodes.iter().any(|node| predicate(node) || any(&node.children, predicate))
}

pub fn unsigned(value: &Json) -> usize {
    match value {
        Json::Number(value) => *value as usize,
        other => panic!("number expected, got {other}"),
    }
}
