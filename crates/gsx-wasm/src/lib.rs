use gsx_core::document::{self, Media};
use gsx_core::project::{Facts, Format, MediaFact, MediaKind, Overflow, Placement};
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
    placement: u8,
    title: Option<String>,
    target_url: Option<String>,
    max_characters: Option<u32>,
    characters_reserved_per_url: Option<u32>,
    caption_limit: Option<u32>,
    description_limit: Option<u32>,
    hashtag_limit: Option<u32>,
    spoiler_text: Option<String>,
    overflow: u8,
    entities_allowed: bool,
    listed: bool,
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
    if let Some(error) = wire::refused_item(&items) {
        return Err(JsError::new(&error));
    }
    let facts = Facts {
        media: match kind {
            0 => None,
            1 => Some(MediaKind::Image),
            2 => Some(MediaKind::Video),
            3 => Some(MediaKind::Gif),
            4 => Some(MediaKind::Audio),
            _ => return Err(JsError::new(&format!("media kind {kind} is not 0, 1, 2, 3 or 4"))),
        },
        media_count: count as usize,
        placement: match placement {
            0 => None,
            1 => Some(Placement::Post),
            2 => Some(Placement::Reel),
            3 => Some(Placement::Story),
            _ => return Err(JsError::new(&format!("placement {placement} is not 0, 1, 2 or 3"))),
        },
        title: title.as_deref(),
        target_url: target_url.as_deref(),
        max_characters,
        characters_reserved_per_url,
        caption_limit,
        description_limit,
        hashtag_limit,
        spoiler_text: spoiler_text.as_deref(),
        overflow: match overflow {
            0 => Overflow::Truncate,
            1 => Overflow::Thread,
            _ => return Err(JsError::new(&format!("overflow {overflow} is not 0 or 1"))),
        },
        entities_allowed,
        items: listed.then_some(&items[..]),
    };
    let media = media(&ids, &covers)?;
    text(len, |text| Ok(wire::project(text, &media, target, &facts)))
}

#[wasm_bindgen]
pub fn targets() -> String {
    wire::targets()
}

#[wasm_bindgen(js_name = isMediaId)]
pub fn is_media_id(len: usize) -> bool {
    INPUT.with_borrow(|buffer| {
        buffer.get(..len).is_some_and(|id| std::str::from_utf8(id).is_ok_and(document::is_media_id))
    })
}

#[wasm_bindgen(js_name = mediaIdMaxBytes)]
pub fn media_id_max_bytes() -> usize {
    document::MEDIA_ID_MAX_BYTES
}

#[wasm_bindgen(js_name = mediaIdPattern)]
pub fn media_id_pattern() -> String {
    document::MEDIA_ID_PATTERN.to_owned()
}

#[wasm_bindgen(js_name = mediaGroupMin)]
pub fn media_group_min() -> usize {
    document::MEDIA_GROUP_MIN
}

#[wasm_bindgen(js_name = mediaGroupMax)]
pub fn media_group_max() -> usize {
    document::MEDIA_GROUP_MAX
}

#[wasm_bindgen(js_name = invalidText)]
pub fn invalid_text(offset: u32, unit: u16) -> String {
    wire::invalid_text(offset, Some(u32::from(unit)))
}

#[wasm_bindgen]
pub fn version() -> String {
    gsx_core::VERSION.to_owned()
}
