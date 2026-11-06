use gsx_core::document::Media;
use gsx_core::project::{Facts, Format, MediaFact, MediaKind};
use gsx_core::{targets, wire};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

thread_local! {
    static INPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

#[wasm_bindgen]
pub fn input(capacity: usize) -> *mut u8 {
    INPUT.with_borrow_mut(|buffer| {
        if buffer.len() < capacity {
            buffer.resize(capacity, 0);
        }
        buffer.as_mut_ptr()
    })
}

fn text<T>(len: usize, run: impl FnOnce(&[u8]) -> Result<T, JsError>) -> Result<T, JsError> {
    INPUT.with_borrow(|buffer| run(buffer.get(..len).ok_or_else(|| JsError::new("len exceeds the input buffer"))?))
}

fn media<'a>(ids: &'a [String], covers: &'a [Option<String>]) -> Result<Vec<Media<'a>>, JsError> {
    if ids.len() != covers.len() {
        return Err(JsError::new("ids and covers differ in length"));
    }
    Ok(ids.iter().zip(covers).map(|(id, cover)| Media { id, cover: cover.as_deref() }).collect())
}

fn strings(values: &[JsValue]) -> Vec<Option<String>> {
    values.iter().map(JsValue::as_string).collect()
}

fn run(len: usize, ids: &[String], covers: &[JsValue], call: fn(&[u8], &[Media]) -> String) -> Result<String, JsError> {
    let covers = strings(covers);
    let media = media(ids, &covers)?;
    text(len, |text| Ok(call(text, &media)))
}

#[wasm_bindgen]
pub fn parse(len: usize, ids: Vec<String>, covers: Vec<JsValue>) -> Result<String, JsError> {
    run(len, &ids, &covers, wire::parse)
}

#[wasm_bindgen]
pub fn serialize(len: usize, ids: Vec<String>, covers: Vec<JsValue>) -> Result<String, JsError> {
    run(len, &ids, &covers, wire::serialize)
}

#[wasm_bindgen]
#[expect(clippy::too_many_arguments, reason = "facts cross the boundary as plain values, not a JS object")]
pub fn project(
    len: usize,
    ids: Vec<String>,
    covers: Vec<JsValue>,
    target: &str,
    kind: u8,
    count: u32,
    title: Option<String>,
    target_url: Option<String>,
    max_characters: Option<u32>,
    characters_reserved_per_url: Option<u32>,
    caption_limit: Option<u32>,
    description_limit: Option<u32>,
    entities_allowed: bool,
    item_ids: Vec<String>,
    item_formats: Vec<u8>,
    item_alts: Vec<JsValue>,
    item_flags: Vec<u8>,
) -> Result<String, JsError> {
    let covers = strings(&covers);
    let target = targets::target(target).ok_or_else(|| JsError::new(&format!("unknown target {target:?}")))?;
    let n = item_ids.len();
    if item_formats.len() != n || item_alts.len() != n || item_flags.len() != n {
        return Err(JsError::new("media fact arrays differ in length"));
    }
    let alts = strings(&item_alts);
    let mut items = Vec::with_capacity(n);
    for (((id, &format), alt), &flags) in item_ids.iter().zip(&item_formats).zip(&alts).zip(&item_flags) {
        let format = match format {
            0 => Format::Image,
            1 => Format::Gif,
            2 => Format::Video,
            3 => Format::Audio,
            other => return Err(JsError::new(&format!("media format {other} is not 0, 1, 2 or 3"))),
        };
        items.push(MediaFact { id, format, alt: alt.as_deref(), spoiler: flags & 1 != 0, inline: flags & 2 != 0 });
    }
    let facts = Facts {
        media: match kind {
            0 => None,
            1 => Some(MediaKind::Image),
            2 => Some(MediaKind::Video),
            _ => return Err(JsError::new(&format!("media kind {kind} is not 0, 1 or 2"))),
        },
        media_count: count as usize,
        title: title.as_deref(),
        target_url: target_url.as_deref(),
        max_characters,
        characters_reserved_per_url,
        caption_limit,
        description_limit,
        entities_allowed,
        items: &items,
    };
    let media = media(&ids, &covers)?;
    text(len, |text| Ok(wire::project(text, &media, target, &facts)))
}

#[wasm_bindgen]
pub fn targets() -> String {
    wire::targets()
}

#[wasm_bindgen(js_name = invalidText)]
pub fn invalid_text(offset: u32, unit: u16) -> String {
    wire::invalid_text(offset, Some(u32::from(unit)))
}

#[wasm_bindgen]
pub fn version() -> String {
    gsx_core::VERSION.to_owned()
}
