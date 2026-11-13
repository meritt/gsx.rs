use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use gsx_core::document::{MEDIA_GROUP_MAX, MEDIA_GROUP_MIN, MEDIA_ID_MAX_BYTES, MEDIA_ID_PATTERN, Media, is_media_id};
use gsx_core::project::{
    FACT_MAX_ITEMS, FACT_MAX_UNITS, Facts, Format, MediaFact, MediaKind, Overflow, Placement, Target,
};
use gsx_core::source::Limit;
use gsx_core::targets::{TARGETS, target};
use gsx_core::wire;

use crate::{Result, at, read};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operation {
    Parse,
    Serialize,
    Project,
    Media,
    Targets,
}

#[derive(Debug)]
pub struct Case {
    id: String,
    expected: String,
    well_formed: bool,
    operation: Operation,
}

pub fn load(files: &[impl AsRef<Path>]) -> Result<Vec<Case>> {
    let mut cases = Vec::new();
    for file in files {
        let file = file.as_ref();
        let text = read(file)?;
        for (number, _, value) in records(file, &text)? {
            let place = format!("{}:{number}", file.display());
            if let Some(case) = media_case(&value) {
                cases.push(case);
                continue;
            }
            let target = match value.get("target").string() {
                Some(name) => Some(target(&name).ok_or_else(|| format!("{place}: no target {name}"))?),
                None => None,
            };
            let operations: &[Operation] = match target {
                Some(_) => &[Operation::Project],
                None => &[Operation::Parse, Operation::Serialize],
            };
            let id = value.get("id").string().unwrap_or_default();
            let well_formed = value.get("markdown").units().is_some_and(|units| String::from_utf16(units).is_ok());
            for &operation in operations {
                let expected = outcome(&value, value.get("facts"), target, operation, &place)?;
                cases.push(Case { id: id.clone(), expected, well_formed, operation });
            }
        }
    }
    Ok(cases)
}

fn media_case(value: &Json) -> Option<Case> {
    if let Json::Bool(true) = value.get("targets") {
        let id = value.get("id").string().unwrap_or_default();
        return Some(Case { id, expected: wire::targets(), well_formed: true, operation: Operation::Targets });
    }
    let expected = match (value.get("mediaRule"), value.get("mediaId").string()) {
        (Json::Bool(true), _) => format!(
            "{{\"mediaIdMaxBytes\":{MEDIA_ID_MAX_BYTES},\"mediaGroupMin\":{MEDIA_GROUP_MIN},\
             \"mediaGroupMax\":{MEDIA_GROUP_MAX},\"mediaIdPattern\":{}}}",
            json_text(MEDIA_ID_PATTERN)
        ),
        (_, Some(id)) => is_media_id(&id).to_string(),
        _ => return None,
    };
    let id = value.get("id").string().unwrap_or_default();
    Some(Case { id, expected, well_formed: true, operation: Operation::Media })
}

pub fn media(out: &Path) -> Result {
    let longest = "a".repeat(MEDIA_ID_MAX_BYTES);
    let samples = [
        "",
        "p",
        &longest,
        &format!("{longest}a"),
        ".a",
        "a.b",
        "_a",
        "a_b",
        ":a",
        "a:b",
        "-a",
        "a-b",
        "A",
        "z",
        "0",
        "AZaz09",
        "a b",
        "a\"b",
        "a,b",
        "a/b",
        "é",
        "aé",
        "a😀",
        "a\n",
        "a\0b",
    ];
    let mut lines = vec![
        "{\"corpus\":\"gsx-media\"}".to_owned(),
        "{\"id\":\"media-rule\",\"mediaRule\":true}".to_owned(),
        "{\"id\":\"targets\",\"targets\":true}".to_owned(),
    ];
    for (index, id) in samples.iter().enumerate() {
        lines.push(format!("{{\"id\":\"media-id-{index}\",\"mediaId\":{}}}", json_text(id)));
    }
    write_lines(out, &lines)
}

pub fn sizes(out: &Path) -> Result {
    let units = Limit::DocumentUnits.maximum() as usize;
    let a = |count| "a".repeat(count).encode_utf16().collect::<Vec<_>>();
    let with = |count, tail: &[u16]| [a(count), tail.to_vec()].concat();
    let samples = [
        ("at-limit", a(units)),
        ("at-limit-pair", with(units - 2, &[0xd83d, 0xde00])),
        ("at-limit-three-bytes", "€".repeat(units).encode_utf16().collect()),
        ("over-limit", a(units + 1)),
        ("over-limit-pair-across", with(units - 1, &[0xd83d, 0xde00])),
        ("over-limit-pair-before", with(units - 2, &[0xd83d, 0xde00, 0x61])),
        ("over-limit-lone-surrogate", with(units - 1, &[0xd800, 0x61])),
        ("over-limit-lone-surrogate-after", with(units + 1, &[0xdc00])),
        ("lone-surrogate", with(1, &[0xd800, 0x61])),
    ];
    let mut lines = vec!["{\"corpus\":\"gsx-sizes\"}".to_owned()];
    for (id, markdown) in &samples {
        let markdown = json(markdown);
        lines.push(format!("{{\"id\":\"size-{id}\",\"markdown\":{markdown},\"media\":[]}}"));
        lines.push(format!(
            "{{\"id\":\"size-{id}-project\",\"markdown\":{markdown},\"media\":[],\"target\":\"x-plain\",\"facts\":{{}}}}"
        ));
        lines.push(format!(
            "{{\"id\":\"size-{id}-refused\",\"markdown\":{markdown},\"media\":[],\"target\":\"x-plain\",\
             \"facts\":{{\"items\":[{{\"id\":\"not an id\",\"format\":\"image\",\"spoiler\":false,\"inline\":true}}]}}}}"
        ));
    }
    write_lines(out, &lines)
}

pub fn ceilings(out: &Path) -> Result {
    const MARKDOWN: &str = "Before the photo.\n\n::media{id=\"a\"}\n\nAfter the photo.";
    let units = FACT_MAX_UNITS;
    let string = |count: usize| json(&[vec![0x61; count - 2], vec![0xd83d, 0xde00]].concat());
    let url = |count: usize| json_text(&format!("https://example.com/{}", "a".repeat(count - 20)));
    let item = |id: &str, alt: Option<&str>| match alt {
        Some(alt) => {
            format!("{{\"id\":\"{id}\",\"format\":\"image\",\"alt\":{alt},\"spoiler\":false,\"inline\":true}}")
        }
        None => format!("{{\"id\":\"{id}\",\"format\":\"image\",\"spoiler\":false,\"inline\":true}}"),
    };
    let many = |count: usize| {
        let items: Vec<String> = (0..count).map(|index| item(&format!("i{index}"), None)).collect();
        format!("[{},{}]", item("a", None), items[1..].join(","))
    };
    let mut samples = vec![];
    for (name, size) in [("at", units), ("over", units + 1)] {
        samples.push((format!("title-{name}"), format!("\"title\":{}", string(size))));
        samples.push((format!("target-url-{name}"), format!("\"targetUrl\":{}", url(size))));
        samples.push((format!("spoiler-text-{name}"), format!("\"spoilerText\":{}", string(size))));
        let alt = string(size);
        samples.push((format!("alt-{name}"), format!("\"items\":[{},{}]", item("a", None), item("b", Some(&alt)))));
    }
    samples.push(("items-at".to_owned(), format!("\"items\":{}", many(FACT_MAX_ITEMS))));
    samples.push(("items-over".to_owned(), format!("\"items\":{}", many(FACT_MAX_ITEMS + 1))));
    let over = string(units + 1);
    samples.push((
        "order".to_owned(),
        format!("\"title\":{over},\"items\":[{},{}]", item("a", Some(&over)), item("-b", None)),
    ));
    let markdown = json_text(MARKDOWN);
    let mut lines = vec!["{\"corpus\":\"gsx-ceilings\"}".to_owned()];
    for target in TARGETS {
        for (name, facts) in &samples {
            lines.push(format!(
                "{{\"id\":\"ceiling-{name}\",\"markdown\":{markdown},\"media\":[{{\"id\":\"a\"}}],\
                 \"target\":\"{}\",\"facts\":{{\"media\":\"image\",\"mediaCount\":1,{facts}}}}}",
                target.name
            ));
        }
    }
    write_lines(out, &lines)
}

pub fn limits(out: &Path) -> Result {
    const MARKDOWN: &str = "# Heading\n\n\
        Read https://example.com/a and [docs](https://example.org/b \"t\") #one #two #three #four #five #six \
        @alice @bob.\n\n\
        ::media{id=\"a\"}\n\n\
        > Quote with `code`, **strong** and ||spoiler||\n\n\
        1. First\n2. Second ::time[today]{unix=\"1785423600\"}";
    const ITEMS: &str = r#"[{"id":"a","format":"image","alt":"An alt text","spoiler":false,"inline":true}]"#;
    const KEYS: [&str; 5] =
        ["maxCharacters", "charactersReservedPerUrl", "captionLimit", "descriptionLimit", "hashtagLimit"];
    const VARIANTS: [(&str, &str); 3] = [("truncate", "post"), ("thread", "post"), ("truncate", "story")];
    let markdown = json_text(MARKDOWN);
    let mut lines = vec!["{\"corpus\":\"gsx-limits\"}".to_owned()];
    for target in TARGETS {
        let sets = KEYS.iter().map(|key| (*key, vec![*key])).chain([("all", KEYS.to_vec())]);
        for (name, keys) in sets {
            for value in [0, 1, u32::MAX] {
                for (overflow, placement) in VARIANTS {
                    let limits = keys.iter().map(|key| format!("\"{key}\":{value}")).collect::<Vec<_>>().join(",");
                    lines.push(format!(
                        "{{\"id\":\"limit-{name}-{value}-{overflow}-{placement}\",\"markdown\":{markdown},\
                         \"media\":[{{\"id\":\"a\"}}],\"target\":\"{}\",\"facts\":{{{limits},\"media\":\"image\",\
                         \"mediaCount\":1,\"placement\":\"{placement}\",\"title\":\"Title\",\
                         \"targetUrl\":\"https://example.com/t\",\"spoilerText\":\"CW\",\"overflow\":\"{overflow}\",\
                         \"entitiesAllowed\":true,\"items\":{ITEMS}}}}}",
                        target.name
                    ));
                }
            }
        }
    }
    write_lines(out, &lines)
}

pub fn regenerate(dir: &Path) -> Result {
    let mut inputs = HashMap::new();
    for name in ["document.jsonl", "diagnostics.jsonl"] {
        let file = dir.join(name);
        let text = read(&file)?;
        let mut lines = vec![text.lines().next().unwrap_or_default().to_owned()];
        for (number, line, value) in records(&file, &text)? {
            let place = format!("{}:{number}", file.display());
            lines.push(with_expected(line, &outcome(&value, &NULL, None, Operation::Parse, &place)?));
            inputs.insert(value.get("id").string().unwrap_or_default(), value);
        }
        write_lines(&file, &lines)?;
    }
    let targets = dir.join("targets");
    let mut files = fs::read_dir(&targets)
        .and_then(|entries| entries.map(|entry| entry.map(|entry| entry.path())).collect::<std::io::Result<Vec<_>>>())
        .map_err(at(targets.display()))?;
    files.sort();
    for file in files {
        let text = read(&file)?;
        let first = text.lines().next().unwrap_or_default();
        let header = Parser { bytes: first.as_bytes(), at: 0 }
            .value()
            .ok_or_else(|| format!("{}: no header", file.display()))?;
        let name = header.get("target").string().unwrap_or_default();
        let target = target(&name).ok_or_else(|| format!("{}: no target {name}", file.display()))?;
        let mut lines = vec![first.to_owned()];
        for (number, line, record) in records(&file, &text)? {
            let place = format!("{}:{number}", file.display());
            let id = record.get("id").string().unwrap_or_default();
            let input = match record.get("markdown") {
                Json::String(_) => &record,
                _ => inputs.get(&id).ok_or_else(|| format!("{place}: no input {id}"))?,
            };
            let facts = match record.get("facts") {
                Json::Null => header.get("facts"),
                facts => facts,
            };
            lines.push(with_expected(line, &outcome(input, facts, Some(target), Operation::Project, &place)?));
        }
        write_lines(&file, &lines)?;
    }
    Ok(())
}

fn with_expected(line: &str, expected: &str) -> String {
    let input = line.find(",\"expected\":").map_or_else(|| line.strip_suffix('}').unwrap_or(line), |end| &line[..end]);
    format!("{input},\"expected\":{expected}}}")
}

fn records<'a>(file: &Path, text: &'a str) -> Result<Vec<(usize, &'a str, Json)>> {
    text.lines()
        .enumerate()
        .skip(1)
        .filter(|(_, line)| !line.is_empty())
        .map(|(index, line)| {
            let value = Parser { bytes: line.as_bytes(), at: 0 }
                .value()
                .ok_or_else(|| format!("{}:{}: not JSON", file.display(), index + 1))?;
            Ok((index + 1, line, value))
        })
        .collect()
}

fn outcome(
    input: &Json,
    facts: &Json,
    target: Option<&'static Target>,
    operation: Operation,
    place: &str,
) -> Result<String> {
    let units = input.get("markdown").units().ok_or_else(|| format!("{place}: no markdown"))?;
    let media: Vec<(String, Option<String>)> = input
        .get("media")
        .items()
        .iter()
        .map(|item| (item.get("id").string().unwrap_or_default(), item.get("cover").string()))
        .collect();
    let media: Vec<Media> = media.iter().map(|(id, cover)| Media { id, cover: cover.as_deref() }).collect();
    let described = facts
        .get("items")
        .items()
        .iter()
        .map(|item| {
            let format = match item.get("format").string().as_deref() {
                Some("image") => Format::Image,
                Some("gif") => Format::Gif,
                Some("video") => Format::Video,
                Some("audio") => Format::Audio,
                other => return Err(format!("{place}: no media format {other:?}")),
            };
            let flag = |key| matches!(item.get(key), Json::Bool(true));
            let id = item.get("id").string().unwrap_or_default();
            Ok((id, format, item.get("alt").string(), flag("spoiler"), flag("inline")))
        })
        .collect::<Result<Vec<_>>>()?;
    let items: Vec<MediaFact> = described
        .iter()
        .map(|(id, format, alt, spoiler, inline)| MediaFact {
            id,
            format: *format,
            alt: alt.as_deref(),
            spoiler: *spoiler,
            inline: *inline,
        })
        .collect();
    let title = facts.get("title").string();
    let target_url = facts.get("targetUrl").string();
    let spoiler_text = facts.get("spoilerText").string();
    let limit = |key| facts.get(key).number();
    let facts = Facts {
        media: match facts.get("media").string().as_deref() {
            None => None,
            Some("image") => Some(MediaKind::Image),
            Some("video") => Some(MediaKind::Video),
            Some("gif") => Some(MediaKind::Gif),
            Some("audio") => Some(MediaKind::Audio),
            Some(other) => return Err(format!("{place}: no media kind {other}")),
        },
        media_count: limit("mediaCount").unwrap_or(0) as usize,
        placement: match facts.get("placement").string().as_deref() {
            None => None,
            Some("post") => Some(Placement::Post),
            Some("reel") => Some(Placement::Reel),
            Some("story") => Some(Placement::Story),
            Some(other) => return Err(format!("{place}: no placement {other}")),
        },
        title: title.as_deref(),
        target_url: target_url.as_deref(),
        max_characters: limit("maxCharacters"),
        characters_reserved_per_url: limit("charactersReservedPerUrl"),
        caption_limit: limit("captionLimit"),
        description_limit: limit("descriptionLimit"),
        hashtag_limit: limit("hashtagLimit"),
        spoiler_text: spoiler_text.as_deref(),
        overflow: match facts.get("overflow").string().as_deref() {
            None | Some("truncate") => Overflow::Truncate,
            Some("thread") => Overflow::Thread,
            Some(other) => return Err(format!("{place}: no overflow {other}")),
        },
        entities_allowed: matches!(facts.get("entitiesAllowed"), Json::Bool(true)),
        items: matches!(facts.get("items"), Json::Array(_)).then_some(&items[..]),
    };
    if let (Operation::Project, Some(refusal)) = (operation, wire::refused(&facts)) {
        return Ok(format!("{{\"refused\":{}}}", json_text(&refusal)));
    }
    Ok(match (String::from_utf16(units).ok(), operation, target) {
        (Some(text), Operation::Project, Some(target)) => wire::project(text.as_bytes(), &media, target, &facts),
        (Some(text), Operation::Serialize, _) => wire::serialize(text.as_bytes(), &media),
        (Some(text), _, _) => wire::parse(text.as_bytes(), &media),
        (None, _, _) => {
            let boundary = Limit::DocumentUnits.maximum() as usize - 1;
            let code_point = units.get(boundary..).and_then(|rest| char::decode_utf16(rest.iter().copied()).next());
            let code_point =
                code_point.map_or(0, |unit| unit.map_or_else(|lone| lone.unpaired_surrogate().into(), u32::from));
            wire::oversized(units.len() as u64, code_point).unwrap_or_else(|| {
                let at = first_lone_surrogate(units);
                wire::invalid_text(at, units.get(at as usize).copied().map(u32::from))
            })
        }
    })
}

const FACTS: [&str; 2] = [
    "{}",
    r#"{"media":"image","mediaCount":1,"title":"Title","targetUrl":"https://example.com/t","maxCharacters":500,"charactersReservedPerUrl":23,"captionLimit":1024,"descriptionLimit":4,"hashtagLimit":3,"spoilerText":"CW","overflow":"thread","entitiesAllowed":true}"#,
];
const PLACEMENTS: [&str; 3] = ["post", "reel", "story"];

pub fn projection(inputs: &[impl AsRef<Path>], selection: &Path, out: &Path) -> Result {
    let mut sources = std::collections::HashMap::new();
    let mut with_media = Vec::new();
    let files = inputs.iter().map(|file| (file.as_ref(), false)).chain([(selection, true)]);
    for (file, own) in files {
        let text = read(file)?;
        for line in text.lines().skip(1).filter(|line| !line.is_empty()) {
            let value = Parser { bytes: line.as_bytes(), at: 0 }
                .value()
                .ok_or_else(|| format!("{}: not JSON", file.display()))?;
            if own && !matches!(value.get("markdown"), Json::String(_)) {
                continue;
            }
            let media: Vec<String> = value
                .get("media")
                .items()
                .iter()
                .map(|item| match item.get("cover").units() {
                    Some(cover) => format!(
                        "{{\"id\":{},\"cover\":{}}}",
                        json(item.get("id").units().unwrap_or_default()),
                        json(cover)
                    ),
                    None => format!("{{\"id\":{}}}", json(item.get("id").units().unwrap_or_default())),
                })
                .collect();
            let input = format!(
                "\"markdown\":{},\"media\":[{}]",
                json(value.get("markdown").units().unwrap_or_default()),
                media.join(",")
            );
            let ids: Vec<&[u16]> =
                value.get("media").items().iter().map(|item| item.get("id").units().unwrap_or_default()).collect();
            let id = value.get("id").string().unwrap_or_default();
            if !ids.is_empty() {
                with_media.push(id.clone());
            }
            sources.insert(id, (input, items(&ids)));
        }
    }
    let text = read(selection)?;
    let mut ids = Vec::new();
    for line in text.lines().skip(1).filter(|line| !line.is_empty()) {
        let id = Parser { bytes: line.as_bytes(), at: 0 }.value().and_then(|value| value.get("id").string());
        ids.push(id.ok_or_else(|| format!("{}: a case without an id", selection.display()))?);
    }
    ids.extend(with_media.into_iter().filter(|id| !ids.contains(id)).collect::<Vec<_>>());
    let mut lines = vec!["{\"corpus\":\"gsx-projection\"}".to_owned()];
    for target in TARGETS {
        for facts in FACTS {
            for (index, id) in ids.iter().enumerate() {
                let (input, items) =
                    sources.get(id).ok_or_else(|| format!("{}: no input for {id}", selection.display()))?;
                let facts = match facts.strip_suffix('}') {
                    Some(given) if facts != "{}" => {
                        let placement = PLACEMENTS[index % PLACEMENTS.len()];
                        format!("{given},\"placement\":\"{placement}\",\"items\":[{items}]}}")
                    }
                    _ => facts.to_owned(),
                };
                lines.push(format!(
                    "{{\"id\":{},{input},\"target\":\"{}\",\"facts\":{facts}}}",
                    json_text(id),
                    target.name
                ));
            }
        }
    }
    write_lines(out, &lines)
}

fn items(ids: &[&[u16]]) -> String {
    const FORMATS: [&str; 4] = ["image", "gif", "video", "audio"];
    let items: Vec<String> = ids
        .iter()
        .enumerate()
        .map(|(index, id)| {
            let alt = if index % 2 == 0 { format!("\"Alt {index}\"") } else { "null".to_owned() };
            format!(
                "{{\"id\":{},\"format\":\"{}\",\"alt\":{alt},\"spoiler\":{},\"inline\":{}}}",
                json(id),
                FORMATS[index % 4],
                index % 3 == 1,
                index % 2 == 0
            )
        })
        .collect();
    items.join(",")
}

fn write_lines(out: &Path, lines: &[String]) -> Result {
    fs::write(out, lines.join("\n") + "\n").map_err(at(out.display()))
}

fn json_text(text: &str) -> String {
    json(&text.encode_utf16().collect::<Vec<_>>())
}

fn json(units: &[u16]) -> String {
    let mut out = String::from("\"");
    for unit in char::decode_utf16(units.iter().copied()) {
        match unit {
            Ok('"') => out.push_str("\\\""),
            Ok('\\') => out.push_str("\\\\"),
            Ok(c) if c >= ' ' => out.push(c),
            Ok(c) => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            Err(lone) => {
                let _ = write!(out, "\\u{:04x}", lone.unpaired_surrogate());
            }
        }
    }
    out.push('"');
    out
}

fn first_lone_surrogate(units: &[u16]) -> u32 {
    let mut at = 0;
    while at < units.len() {
        match units[at] {
            0xD800..=0xDBFF if units.get(at + 1).is_some_and(|low| (0xDC00..=0xDFFF).contains(low)) => at += 2,
            0xD800..=0xDFFF => return at as u32,
            _ => at += 1,
        }
    }
    units.len() as u32
}

pub fn compare(binding: &str, cases: &[Case], out: &Path, skips_ill_formed: bool) -> Result {
    let text = read(out)?;
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() != cases.len() {
        return Err(format!("{binding}: {} lines for {} cases", lines.len(), cases.len()));
    }
    let mut failures = Vec::new();
    let mut skipped = 0;
    for (case, line) in cases.iter().zip(&lines) {
        if skips_ill_formed && !case.well_formed {
            if *line == "-" {
                skipped += 1;
            } else {
                failures.push(format!("{}: ill-formed input, expected a skip, got {line}", case.id));
            }
        } else if *line != case.expected {
            failures.push(format!("{}:\n  rust   {}\n  {binding:<6} {line}", case.id, case.expected));
        }
    }
    if !failures.is_empty() {
        let shown = failures.iter().take(10).cloned().collect::<Vec<_>>().join("\n");
        return Err(format!("{binding}: {} of {} cases differ from Rust:\n{shown}", failures.len(), cases.len()));
    }
    let count = |operation| {
        cases.iter().filter(|case| case.operation == operation && (case.well_formed || !skips_ill_formed)).count()
    };
    println!(
        "{binding}: {} parse, {} serialize, {} projection, {} media rule and {} target listing cases byte-identical to \
         Rust{}",
        count(Operation::Parse),
        count(Operation::Serialize),
        count(Operation::Project),
        count(Operation::Media),
        count(Operation::Targets),
        if skipped > 0 { format!(", {skipped} ill-formed lines skipped") } else { String::new() }
    );
    Ok(())
}

enum Json {
    Null,
    Other,
    Bool(bool),
    Number(u32),
    String(Vec<u16>),
    Array(Vec<Json>),
    Object(Vec<(Vec<u16>, Json)>),
}

const NULL: Json = Json::Null;

impl Json {
    fn get(&self, key: &str) -> &Json {
        let key: Vec<u16> = key.encode_utf16().collect();
        match self {
            Json::Object(entries) => entries.iter().find(|(k, _)| *k == key).map_or(&NULL, |(_, v)| v),
            _ => &NULL,
        }
    }

    fn units(&self) -> Option<&[u16]> {
        match self {
            Json::String(units) => Some(units),
            _ => None,
        }
    }

    fn number(&self) -> Option<u32> {
        match self {
            Json::Number(number) => Some(*number),
            _ => None,
        }
    }

    fn string(&self) -> Option<String> {
        self.units().map(String::from_utf16_lossy)
    }

    fn items(&self) -> &[Json] {
        match self {
            Json::Array(items) => items,
            _ => &[],
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

    fn next(&mut self) -> Option<u8> {
        self.space();
        let byte = *self.bytes.get(self.at)?;
        self.at += 1;
        Some(byte)
    }

    fn value(&mut self) -> Option<Json> {
        match self.next()? {
            b'{' => {
                let mut entries = Vec::new();
                self.space();
                if self.bytes.get(self.at) == Some(&b'}') {
                    self.at += 1;
                    return Some(Json::Object(entries));
                }
                loop {
                    let Json::String(key) = self.value()? else { return None };
                    (self.next()? == b':').then_some(())?;
                    entries.push((key, self.value()?));
                    match self.next()? {
                        b',' => {}
                        b'}' => return Some(Json::Object(entries)),
                        _ => return None,
                    }
                }
            }
            b'[' => {
                let mut items = Vec::new();
                self.space();
                if self.bytes.get(self.at) == Some(&b']') {
                    self.at += 1;
                    return Some(Json::Array(items));
                }
                loop {
                    items.push(self.value()?);
                    match self.next()? {
                        b',' => {}
                        b']' => return Some(Json::Array(items)),
                        _ => return None,
                    }
                }
            }
            b'"' => self.string().map(Json::String),
            b'n' => self.word("ull").map(|()| Json::Null),
            b't' => self.word("rue").map(|()| Json::Bool(true)),
            b'f' => self.word("alse").map(|()| Json::Bool(false)),
            b'-' | b'0'..=b'9' => {
                let start = self.at - 1;
                while self.bytes.get(self.at).is_some_and(|byte| b"+-.eE0123456789".contains(byte)) {
                    self.at += 1;
                }
                let number = std::str::from_utf8(&self.bytes[start..self.at]).ok()?;
                Some(number.parse().map_or(Json::Other, Json::Number))
            }
            _ => None,
        }
    }

    fn word(&mut self, rest: &str) -> Option<()> {
        self.bytes[self.at..].starts_with(rest.as_bytes()).then(|| self.at += rest.len())
    }

    fn string(&mut self) -> Option<Vec<u16>> {
        let mut units = Vec::new();
        loop {
            let start = self.at;
            while !matches!(self.bytes.get(self.at)?, b'"' | b'\\') {
                self.at += 1;
            }
            units.extend(std::str::from_utf8(&self.bytes[start..self.at]).ok()?.encode_utf16());
            self.at += 1;
            if self.bytes[self.at - 1] == b'"' {
                return Some(units);
            }
            let escape = *self.bytes.get(self.at)?;
            self.at += 1;
            units.push(match escape {
                b'n' => 0x0a,
                b't' => 0x09,
                b'r' => 0x0d,
                b'b' => 0x08,
                b'f' => 0x0c,
                b'u' => {
                    let hex = std::str::from_utf8(self.bytes.get(self.at..self.at + 4)?).ok()?;
                    self.at += 4;
                    u16::from_str_radix(hex, 16).ok()?
                }
                other => u16::from(other),
            });
        }
    }
}
