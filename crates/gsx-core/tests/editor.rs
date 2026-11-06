use std::collections::HashSet;

use gsx_core::document::Media;
use gsx_core::wire;
use serde_json::{Value, json};

const SEEDS: &[&str] = &[
    "Some **bold**, *emphasis* and ***both*** here",
    "~~gone~~, ||spoiler||, ++underline++ and ==highlight==",
    "Inline `code`, ``tick ` run`` and $x^2 + y$ math",
    "[link](https://example.com \"Title\") and ![alt](https://example.com/a.png)",
    "[full][ref], [collapsed][] and [shortcut]\n\n[ref]: https://example.com\n[shortcut]: <https://example.com/s> 'T'",
    "<https://example.com/a> and www.example.com and https://example.com/b.",
    "Escape \\* and \\\\, entity &amp; &#169; &#x1F600;",
    "Meet at :time[launch]{datetime=\"2026-11-05T10:00:00+03:00\"} today",
    ":note[label]{#id .class key=value} after",
    "Call at 10:30, ratio 3:2, :time[x] inside :other[a :time[y]]",
    "Intro\n\n::media{id=\"a\"}\n\n::media{ids=\"b,c\" layout=\"collage\"}\n",
    "# Heading\n\nSetext\n===\n\nOther\n---\n",
    "> Quote\n> - item\n> - [x] done\n>\n> 1. one\n",
    "- [ ] task\n  continued\n- next\n\n  loose\n",
    "```rust title\nfn main() {}\n```\n\n~~~\nraw\n~~~\n",
    "    indented code\n\nafter",
    "| a | b |\n| - | :-: |\n| 1 | **2** |\n",
    "a | b\n--|--\n1 | 2",
    "***\n\n- - -\n",
    "line  \nbreak\\\nend",
    "<b>html</b> and <!-- comment --> inline\n\n<div>\nblock\n</div>\n",
];

fn corpus(file: &str) -> String {
    format!("{}/../../testdata/corpus/{file}", env!("CARGO_MANIFEST_DIR"))
}

fn read(file: &str) -> Vec<Value> {
    std::fs::read_to_string(corpus(file))
        .unwrap()
        .lines()
        .skip(1)
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn media(case: &Value) -> Vec<Media<'_>> {
    case["media"].as_array().map_or_else(Vec::new, |items| {
        items.iter().map(|item| Media { id: item["id"].as_str().unwrap(), cover: item["cover"].as_str() }).collect()
    })
}

fn parsed(markdown: &str, media: &[Media]) -> Value {
    serde_json::from_str(&wire::parse(markdown.as_bytes(), media)).unwrap()
}

fn id(markdown: &str, media: &Value) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in markdown.bytes().chain([0]).chain(media.to_string().bytes()) {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn shape(value: &Value, out: &mut String) {
    match value {
        Value::Object(map) => {
            for key in ["type", "kind", "code"] {
                if let Some(Value::String(name)) = map.get(key) {
                    out.push_str(name);
                }
            }
            out.push('(');
            for key in ["document", "error", "children", "markers", "diagnostics"] {
                if let Some(inner) = map.get(key) {
                    shape(inner, out);
                }
            }
            out.push(')');
        }
        Value::Array(items) => {
            out.push('[');
            for item in items {
                shape(item, out);
            }
            out.push(']');
        }
        _ => {}
    }
}

fn tail(result: &Value) -> String {
    let mut out = String::new();
    let document = &result["document"];
    let Some(last) = document["children"].as_array().and_then(|children| children.last()) else {
        shape(result, &mut out);
        return out;
    };
    shape(last, &mut out);
    let start = last["range"][0].as_u64().unwrap();
    for marker in document["markers"].as_array().into_iter().flatten() {
        if marker["range"][0].as_u64().unwrap() >= start {
            shape(marker, &mut out);
        }
    }
    shape(&document["diagnostics"], &mut out);
    out
}

fn prefixes(markdown: &str) -> impl Iterator<Item = &str> {
    (0..markdown.len()).filter(|&end| markdown.is_char_boundary(end)).map(|end| &markdown[..end])
}

struct Editor {
    cases: Vec<Value>,
    ids: HashSet<String>,
    shapes: HashSet<String>,
}

impl Editor {
    fn add(&mut self, markdown: &str, media_list: &Value, source: String, every: bool) {
        let id = id(markdown, media_list);
        if self.ids.contains(&id) {
            return;
        }
        let case = json!({ "media": media_list });
        let expected = parsed(markdown, &media(&case));
        let signature = tail(&expected);
        if !self.shapes.insert(signature) && !every {
            return;
        }
        self.ids.insert(id.clone());
        self.cases.push(json!({
            "id": id,
            "sources": [source],
            "markdown": markdown,
            "media": media_list,
            "expected": expected,
        }));
    }
}

fn generate() -> (Vec<Value>, Vec<Value>) {
    let documents = read("document.jsonl");
    let mut syntax = vec![];
    for case in &documents {
        let markdown = case["markdown"].as_str().unwrap();
        let result = parsed(markdown, &media(case));
        if let Some(markers) = result["document"].get("markers") {
            syntax.push(json!({
                "id": case["id"],
                "markdown": markdown,
                "media": case["media"],
                "markers": markers,
            }));
        }
    }
    let mut editor = Editor { cases: vec![], ids: HashSet::new(), shapes: HashSet::new() };
    let seed_media = json!([{ "id": "a", "cover": null }, { "id": "b", "cover": null }, { "id": "c", "cover": null }]);
    for (index, seed) in SEEDS.iter().enumerate() {
        for prefix in prefixes(seed).chain([*seed]) {
            editor.add(prefix, &seed_media, format!("seed:{index}"), true);
        }
    }
    for case in &documents {
        let markdown = case["markdown"].as_str().unwrap();
        if markdown.len() > 512 {
            continue;
        }
        for prefix in prefixes(markdown) {
            editor.add(prefix, &case["media"], format!("prefix:{}", case["id"].as_str().unwrap()), false);
        }
    }
    (syntax, editor.cases)
}

fn write(file: &str, area: &str, sources: &str, cases: &[Value]) {
    let header = json!({
        "corpus": "gsx-corpus",
        "area": area,
        "schema": 1,
        "reference": { "implementation": "gsx" },
        "cases": cases.len(),
        "sources": sources,
    });
    let mut text = header.to_string();
    text.push('\n');
    for case in cases {
        text.push_str(&case.to_string());
        text.push('\n');
    }
    std::fs::write(corpus(file), text).unwrap();
}

fn markers(case: &Value) -> &Value {
    if case.get("markers").is_some() { &case["markers"] } else { &case["expected"]["document"]["markers"] }
}

#[test]
fn matches_syntax_and_editor_corpus() {
    if std::env::var("GSX_BLESS").is_ok() {
        let (syntax, editor) = generate();
        write(
            "syntax.jsonl",
            "syntax",
            "every document.jsonl input that parses; markers are the UTF-16 ranges of syntax an editor highlights",
            &syntax,
        );
        write(
            "editor.jsonl",
            "editor",
            "every prefix of each seed (seed:<index>), and prefixes of document.jsonl inputs up to 512 bytes \
             (prefix:<id>) kept when they add a new shape of node types, marker kinds and diagnostic codes",
            &editor,
        );
    }
    let mut failures = vec![];
    for case in read("syntax.jsonl") {
        let actual = parsed(case["markdown"].as_str().unwrap(), &media(&case));
        if actual["document"]["markers"] != case["markers"] {
            failures.push(format!(
                "syntax {} {}\n    actual {}",
                case["id"], case["markdown"], actual["document"]["markers"]
            ));
        }
    }
    for case in read("editor.jsonl") {
        let actual = parsed(case["markdown"].as_str().unwrap(), &media(&case));
        if actual != case["expected"] {
            failures.push(format!("editor {} {}\n    actual {actual}", case["id"], case["markdown"]));
        }
    }
    assert!(failures.is_empty(), "{} cases differ:\n{}", failures.len(), failures.join("\n"));
}

#[test]
fn markers_are_ordered_disjoint_and_inside_the_text() {
    let mut failures = vec![];
    for case in read("syntax.jsonl").iter().chain(&read("editor.jsonl")) {
        let units = case["markdown"].as_str().unwrap().encode_utf16().count() as u64;
        let ranges: Vec<(u64, u64)> = markers(case)
            .as_array()
            .into_iter()
            .flatten()
            .map(|marker| (marker["range"][0].as_u64().unwrap(), marker["range"][1].as_u64().unwrap()))
            .collect();
        let inside = ranges.iter().all(|&(start, end)| start < end && end <= units);
        let ordered = ranges.windows(2).all(|pair| pair[0].1 <= pair[1].0);
        if !inside || !ordered {
            failures.push(format!("{} {} {ranges:?}", case["id"], case["markdown"]));
        }
    }
    assert!(failures.is_empty(), "{} cases:\n{}", failures.len(), failures.join("\n"));
}
