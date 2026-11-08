#![expect(unsafe_code, reason = "a C ABI takes and returns raw pointers")]

use std::ffi::{CString, c_char};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::sync::LazyLock;

use gsx_core::document::Media as CoreMedia;
use gsx_core::project::{Facts as CoreFacts, Format, MediaFact as CoreMediaFact, MediaKind, Overflow, Placement};
use gsx_core::{targets, wire};

pub use gsx_core::VERSION;

#[derive(Debug)]
pub struct Output(CString);

#[derive(Debug)]
pub struct Error(CString);

#[repr(C)]
#[derive(Debug)]
pub struct Media {
    pub id: *const u8,
    pub id_len: usize,
    pub cover: *const u8,
    pub cover_len: usize,
}

fn c_string(text: String) -> CString {
    CString::new(text).unwrap_or_else(|e| {
        let text = String::from_utf8_lossy(&e.into_vec()).replace('\0', "\u{FFFD}");
        CString::new(text).expect("no interior NUL")
    })
}

/// # Safety
/// `slot` is `NULL` or writable.
unsafe fn guarded<T>(slot: *mut *mut Error, f: impl FnOnce() -> Result<T, String>) -> Option<T> {
    if !slot.is_null() {
        // SAFETY: `slot` is not NULL, and the caller promises it is writable.
        unsafe { *slot = ptr::null_mut() };
    }
    let message = match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(value)) => return Some(value),
        Ok(Err(message)) => message,
        Err(panic) => panic
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| panic.downcast_ref::<&str>().map(|s| (*s).to_owned()))
            .unwrap_or_else(|| "panic".to_owned()),
    };
    if !slot.is_null() {
        // SAFETY: `slot` is not NULL, and the caller promises it is writable.
        unsafe { *slot = Box::into_raw(Box::new(Error(c_string(message)))) };
    }
    None
}

/// # Safety
/// `slot` is `NULL` or writable.
unsafe fn output(slot: *mut *mut Error, f: impl FnOnce() -> Result<String, String>) -> *mut Output {
    // SAFETY: `slot` is as this function's contract states.
    unsafe { guarded(slot, || f().map(|json| Output(c_string(json)))) }
        .map_or(ptr::null_mut(), |output| Box::into_raw(Box::new(output)))
}

/// # Safety
/// `entries` points to `len` entries, or `len` is 0.
unsafe fn slice<'a, T>(entries: *const T, len: usize, what: &str) -> Result<&'a [T], String> {
    match (entries.is_null(), len) {
        (_, 0) => Ok(&[]),
        (true, _) => Err(format!("{what} is NULL")),
        // SAFETY: not NULL, and the caller promises `len` readable entries.
        (false, _) => Ok(unsafe { std::slice::from_raw_parts(entries, len) }),
    }
}

/// # Safety
/// `bytes` points to `len` bytes, or `len` is 0.
unsafe fn string<'a>(bytes: *const u8, len: usize, what: &str) -> Result<&'a str, String> {
    // SAFETY: `bytes` is as this function's contract states.
    std::str::from_utf8(unsafe { slice(bytes, len, what) }?).map_err(|_| format!("{what} must be UTF-8"))
}

/// # Safety
/// `bytes` is `NULL` or points to `len` bytes.
unsafe fn optional_string<'a>(bytes: *const u8, len: usize, what: &str) -> Result<Option<&'a str>, String> {
    if bytes.is_null() {
        return Ok(None);
    }
    // SAFETY: not NULL, and the caller promises `len` readable bytes.
    unsafe { string(bytes, len, what) }.map(Some)
}

pub const GSX_MEDIA_NONE: u32 = 0;
pub const GSX_MEDIA_IMAGE: u32 = 1;
pub const GSX_MEDIA_VIDEO: u32 = 2;
pub const GSX_MEDIA_GIF: u32 = 3;
pub const GSX_MEDIA_AUDIO: u32 = 4;

pub const GSX_PLACEMENT_NONE: u32 = 0;
pub const GSX_PLACEMENT_POST: u32 = 1;
pub const GSX_PLACEMENT_REEL: u32 = 2;
pub const GSX_PLACEMENT_STORY: u32 = 3;
pub const GSX_OVERFLOW_TRUNCATE: u32 = 0;
pub const GSX_OVERFLOW_THREAD: u32 = 1;

pub const GSX_FORMAT_IMAGE: u32 = 0;
pub const GSX_FORMAT_GIF: u32 = 1;
pub const GSX_FORMAT_VIDEO: u32 = 2;
pub const GSX_FORMAT_AUDIO: u32 = 3;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MediaFact {
    pub id: *const u8,
    pub id_len: usize,
    pub format: u32,
    pub alt: *const u8,
    pub alt_len: usize,
    pub spoiler: bool,
    pub inline: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Facts {
    pub media: u32,
    pub media_count: usize,
    pub placement: u32,
    pub title: *const u8,
    pub title_len: usize,
    pub target_url: *const u8,
    pub target_url_len: usize,
    pub max_characters: i64,
    pub characters_reserved_per_url: i64,
    pub caption_limit: i64,
    pub description_limit: i64,
    pub entities_allowed: bool,
    pub items: *const MediaFact,
    pub items_len: usize,
    pub hashtag_limit: i64,
    pub spoiler_text: *const u8,
    pub spoiler_text_len: usize,
    pub overflow: u32,
}

impl Facts {
    /// # Safety
    /// `items` is `NULL` with `items_len` 0, or points to `items_len` entries whose `id` points to
    /// `id_len` bytes and whose `alt` is `NULL` or points to `alt_len` bytes.
    unsafe fn items(&self) -> Result<Option<Vec<CoreMediaFact<'_>>>, String> {
        if self.items.is_null() && self.items_len == 0 {
            return Ok(None);
        }
        // SAFETY: the caller promises `items_len` readable entries.
        let entries = unsafe { slice(self.items, self.items_len, "items") }?;
        let mut list = Vec::with_capacity(entries.len());
        for entry in entries {
            // SAFETY: the caller promises each entry's pointers.
            let id = unsafe { string(entry.id, entry.id_len, "item id") }?;
            // SAFETY: as above.
            let alt = unsafe { optional_string(entry.alt, entry.alt_len, "item alt") }?;
            let format = match entry.format {
                GSX_FORMAT_IMAGE => Format::Image,
                GSX_FORMAT_GIF => Format::Gif,
                GSX_FORMAT_VIDEO => Format::Video,
                GSX_FORMAT_AUDIO => Format::Audio,
                other => return Err(format!("item format {other} is not GSX_FORMAT_*")),
            };
            list.push(CoreMediaFact { id, format, alt, spoiler: entry.spoiler, inline: entry.inline });
        }
        Ok(Some(list))
    }

    /// # Safety
    /// `title` is `NULL` or points to `title_len` bytes; `target_url` is `NULL` or points to
    /// `target_url_len` bytes; `spoiler_text` is `NULL` or points to `spoiler_text_len` bytes.
    unsafe fn core<'a>(&'a self, items: Option<&'a [CoreMediaFact<'a>]>) -> Result<CoreFacts<'a>, String> {
        let limit = |value: i64, what: &str| match value {
            ..0 => Ok(None),
            _ => u32::try_from(value).map(Some).map_err(|_| format!("{what} {value} is over 2^32 - 1")),
        };
        // SAFETY: the caller promises `title` is NULL or points to `title_len` readable bytes.
        let title = unsafe { optional_string(self.title, self.title_len, "title") }?;
        // SAFETY: the caller promises `target_url` is NULL or points to `target_url_len` readable bytes.
        let target_url = unsafe { optional_string(self.target_url, self.target_url_len, "target_url") }?;
        // SAFETY: the caller promises `spoiler_text` is NULL or points to `spoiler_text_len` readable bytes.
        let spoiler_text = unsafe { optional_string(self.spoiler_text, self.spoiler_text_len, "spoiler_text") }?;
        Ok(CoreFacts {
            media: match self.media {
                GSX_MEDIA_NONE => None,
                GSX_MEDIA_IMAGE => Some(MediaKind::Image),
                GSX_MEDIA_VIDEO => Some(MediaKind::Video),
                GSX_MEDIA_GIF => Some(MediaKind::Gif),
                GSX_MEDIA_AUDIO => Some(MediaKind::Audio),
                other => return Err(format!("media kind {other} is not GSX_MEDIA_*")),
            },
            media_count: self.media_count,
            placement: match self.placement {
                GSX_PLACEMENT_NONE => None,
                GSX_PLACEMENT_POST => Some(Placement::Post),
                GSX_PLACEMENT_REEL => Some(Placement::Reel),
                GSX_PLACEMENT_STORY => Some(Placement::Story),
                other => return Err(format!("placement {other} is not GSX_PLACEMENT_*")),
            },
            title,
            target_url,
            max_characters: limit(self.max_characters, "max_characters")?,
            characters_reserved_per_url: limit(self.characters_reserved_per_url, "characters_reserved_per_url")?,
            caption_limit: limit(self.caption_limit, "caption_limit")?,
            description_limit: limit(self.description_limit, "description_limit")?,
            hashtag_limit: limit(self.hashtag_limit, "hashtag_limit")?,
            spoiler_text,
            overflow: match self.overflow {
                GSX_OVERFLOW_TRUNCATE => Overflow::Truncate,
                GSX_OVERFLOW_THREAD => Overflow::Thread,
                other => return Err(format!("overflow {other} is not GSX_OVERFLOW_*")),
            },
            entities_allowed: self.entities_allowed,
            items,
        })
    }
}

/// # Safety
/// `text` points to `len` bytes; `media` points to `media_len` entries whose `id` points to
/// `id_len` bytes and whose `cover` is `NULL` or points to `cover_len` bytes; all stay valid for
/// the call. `error` is `NULL` or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gsx_parse(
    text: *const u8,
    len: usize,
    media: *const Media,
    media_len: usize,
    error: *mut *mut Error,
) -> *mut Output {
    // SAFETY: every pointer is as this function's contract states.
    unsafe { output(error, || Ok(wire::parse(slice(text, len, "text")?, &media_list(media, media_len)?))) }
}

/// # Safety
/// As for `gsx_parse`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gsx_serialize(
    text: *const u8,
    len: usize,
    media: *const Media,
    media_len: usize,
    error: *mut *mut Error,
) -> *mut Output {
    // SAFETY: every pointer is as this function's contract states.
    unsafe { output(error, || Ok(wire::serialize(slice(text, len, "text")?, &media_list(media, media_len)?))) }
}

/// # Safety
/// As for `gsx_parse`; `target` points to `target_len` bytes; `facts` is `NULL` (no facts) or
/// points to a `gsx_facts` whose `title` is `NULL` or points to `title_len` bytes and whose `items`
/// is `NULL` with `items_len` 0 (no items fact) or points to `items_len` media facts, each `id` and
/// `alt` as `id` and `cover` of `gsx_parse` media.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gsx_project(
    text: *const u8,
    len: usize,
    media: *const Media,
    media_len: usize,
    target: *const u8,
    target_len: usize,
    facts: *const Facts,
    error: *mut *mut Error,
) -> *mut Output {
    // SAFETY: every pointer is as this function's contract states.
    unsafe {
        output(error, || {
            let text = slice(text, len, "text")?;
            let name = string(target, target_len, "target")?;
            let target = targets::target(name).ok_or_else(|| format!("unknown target {name:?}"))?;
            let media = media_list(media, media_len)?;
            let Some(facts) = facts.as_ref() else {
                return Ok(wire::project(text, &media, target, &CoreFacts::default()));
            };
            let items = facts.items()?;
            Ok(wire::project(text, &media, target, &facts.core(items.as_deref())?))
        })
    }
}

/// # Safety
/// `error` is `NULL` or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gsx_targets(error: *mut *mut Error) -> *mut Output {
    // SAFETY: `error` is as this function's contract states.
    unsafe { output(error, || Ok(wire::targets())) }
}

/// # Safety
/// `media` points to `media_len` entries as `gsx_parse` states.
unsafe fn media_list<'a>(media: *const Media, media_len: usize) -> Result<Vec<CoreMedia<'a>>, String> {
    // SAFETY: the caller promises `media_len` readable entries.
    let entries = unsafe { slice(media, media_len, "media") }?;
    let mut list = Vec::with_capacity(entries.len());
    for entry in entries {
        // SAFETY: the caller promises each entry's pointers as `gsx_parse` states.
        let id = unsafe { string(entry.id, entry.id_len, "media id") }?;
        // SAFETY: as above.
        let cover = unsafe { optional_string(entry.cover, entry.cover_len, "media cover") }?;
        list.push(CoreMedia { id, cover });
    }
    Ok(list)
}

/// # Safety
/// `output` came from this library and is not freed yet; `len` is `NULL` or writable. The string
/// lives until `gsx_output_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gsx_output_json(output: *const Output, len: *mut usize) -> *const c_char {
    // SAFETY: `output` is a live `Output`, as this function's contract states.
    let json = unsafe { &(*output).0 };
    if !len.is_null() {
        // SAFETY: not NULL, and the caller promises it is writable.
        unsafe { *len = json.as_bytes().len() };
    }
    json.as_ptr()
}

/// # Safety
/// `output` is `NULL` or came from this library and is not used afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gsx_output_free(output: *mut Output) {
    if !output.is_null() {
        // SAFETY: the pointer came from `Box::into_raw` in this library and is freed once.
        drop(unsafe { Box::from_raw(output) });
    }
}

/// # Safety
/// `error` came from this library and is not freed yet. The string lives until `gsx_error_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gsx_error_message(error: *const Error) -> *const c_char {
    // SAFETY: `error` is a live `Error`, as this function's contract states.
    unsafe { (*error).0.as_ptr() }
}

/// # Safety
/// `error` is `NULL` or came from this library and is not used afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gsx_error_free(error: *mut Error) {
    if !error.is_null() {
        // SAFETY: the pointer came from `Box::into_raw` in `guarded` and is freed once.
        drop(unsafe { Box::from_raw(error) });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn gsx_version() -> *const c_char {
    static VERSION: LazyLock<CString> = LazyLock::new(|| c_string(gsx_core::VERSION.to_owned()));
    VERSION.as_ptr()
}

#[cfg(test)]
mod tests {
    use std::ffi::CStr;

    use super::*;

    fn parse(text: &[u8], media: &[Media]) -> Result<String, String> {
        let mut error = ptr::null_mut();
        // SAFETY: local buffers with their lengths and a local slot.
        let output = unsafe { gsx_parse(text.as_ptr(), text.len(), media.as_ptr(), media.len(), &raw mut error) };
        read(output, error)
    }

    fn project(text: &[u8], target: &str, facts: Option<&Facts>) -> Result<String, String> {
        project_with(text, &[], target, facts)
    }

    fn project_with(text: &[u8], ids: &[&str], target: &str, facts: Option<&Facts>) -> Result<String, String> {
        let media: Vec<Media> = ids
            .iter()
            .map(|id| Media { id: id.as_ptr(), id_len: id.len(), cover: ptr::null(), cover_len: 0 })
            .collect();
        let mut error = ptr::null_mut();
        let facts = facts.map_or(ptr::null(), ptr::from_ref);
        // SAFETY: local buffers with their lengths and a local slot.
        let output = unsafe {
            gsx_project(
                text.as_ptr(),
                text.len(),
                media.as_ptr(),
                media.len(),
                target.as_ptr(),
                target.len(),
                facts,
                &raw mut error,
            )
        };
        read(output, error)
    }

    fn read(output: *mut Output, error: *mut Error) -> Result<String, String> {
        // SAFETY: the output or the error came from this library and is read before it is freed.
        unsafe {
            if output.is_null() {
                let message = CStr::from_ptr(gsx_error_message(error)).to_str().unwrap().to_owned();
                gsx_error_free(error);
                return Err(message);
            }
            assert!(error.is_null());
            let mut len = 0;
            let json = gsx_output_json(output, &raw mut len);
            let bytes = std::slice::from_raw_parts(json.cast::<u8>(), len + 1);
            assert_eq!(bytes[len], 0);
            let json = std::str::from_utf8(&bytes[..len]).unwrap().to_owned();
            gsx_output_free(output);
            Ok(json)
        }
    }

    fn media(id: &str, cover: Option<&str>) -> Media {
        Media {
            id: id.as_ptr(),
            id_len: id.len(),
            cover: cover.map_or(ptr::null(), str::as_ptr),
            cover_len: cover.map_or(0, str::len),
        }
    }

    #[test]
    fn returns_the_wire_json() {
        let text = "a *b* ::media{id=\"v\"}";
        let core = [CoreMedia { id: "v", cover: Some("c") }, CoreMedia { id: "c", cover: None }];
        let media = [media("v", Some("c")), media("c", None)];
        assert_eq!(parse(text.as_bytes(), &media).unwrap(), wire::parse(text.as_bytes(), &core));
        assert_eq!(parse(b"", &[]).unwrap(), wire::parse(b"", &[]));
        assert_eq!(parse(b"a\xffb", &[]).unwrap(), wire::invalid_text(1, None));
    }

    #[test]
    fn serializes() {
        let text = "_a_ ::media{id=\"v\"}";
        let core = [CoreMedia { id: "v", cover: None }];
        let media = [media("v", None)];
        let mut error = ptr::null_mut();
        // SAFETY: local buffers with their lengths and a local slot.
        let output = unsafe { gsx_serialize(text.as_ptr(), text.len(), media.as_ptr(), media.len(), &raw mut error) };
        assert_eq!(read(output, error).unwrap(), wire::serialize(text.as_bytes(), &core));
        // SAFETY: a NULL text with a length is what the call must reject; a local slot.
        let output = unsafe { gsx_serialize(ptr::null(), 1, ptr::null(), 0, &raw mut error) };
        assert_eq!(read(output, error).unwrap_err(), "text is NULL");
    }

    #[test]
    fn projects_with_facts() {
        let text = "# T\n\nbody";
        let x = targets::target("x-plain").unwrap();
        assert_eq!(
            project(text.as_bytes(), "x-plain", None).unwrap(),
            wire::project(text.as_bytes(), &[], x, &CoreFacts::default())
        );
        let (title, spoiler) = ("Title", "Spoiler");
        let facts = Facts {
            media: GSX_MEDIA_VIDEO,
            media_count: 1,
            placement: GSX_PLACEMENT_STORY,
            title: title.as_ptr(),
            title_len: title.len(),
            target_url: ptr::null(),
            target_url_len: 0,
            max_characters: 100,
            characters_reserved_per_url: -1,
            caption_limit: -1,
            description_limit: -1,
            entities_allowed: true,
            items: ptr::null(),
            items_len: 0,
            hashtag_limit: 3,
            spoiler_text: spoiler.as_ptr(),
            spoiler_text_len: spoiler.len(),
            overflow: GSX_OVERFLOW_THREAD,
        };
        let core = CoreFacts {
            media: Some(MediaKind::Video),
            media_count: 1,
            placement: Some(Placement::Story),
            title: Some(title),
            max_characters: Some(100),
            hashtag_limit: Some(3),
            spoiler_text: Some(spoiler),
            overflow: Overflow::Thread,
            entities_allowed: true,
            ..CoreFacts::default()
        };
        let youtube = targets::target("youtube-plain").unwrap();
        assert_eq!(
            project(text.as_bytes(), "youtube-plain", Some(&facts)).unwrap(),
            wire::project(text.as_bytes(), &[], youtube, &core)
        );
        assert_eq!(project(b"", "nope", None).unwrap_err(), r#"unknown target "nope""#);
        let bad = Facts { media: 5, ..facts };
        assert_eq!(project(b"", "x-plain", Some(&bad)).unwrap_err(), "media kind 5 is not GSX_MEDIA_*");
        let bad = Facts { placement: 4, ..facts };
        assert_eq!(project(b"", "x-plain", Some(&bad)).unwrap_err(), "placement 4 is not GSX_PLACEMENT_*");
        let bad = Facts { overflow: 2, ..facts };
        assert_eq!(project(b"", "x-plain", Some(&bad)).unwrap_err(), "overflow 2 is not GSX_OVERFLOW_*");
        let bad = Facts { caption_limit: 1 << 32, ..facts };
        assert_eq!(project(b"", "x-plain", Some(&bad)).unwrap_err(), "caption_limit 4294967296 is over 2^32 - 1");

        let text = "Look\n\n::media{id=\"a\"}\n\nend";
        let (id, alt) = ("a", "A cat");
        let item = MediaFact {
            id: id.as_ptr(),
            id_len: id.len(),
            format: GSX_FORMAT_GIF,
            alt: alt.as_ptr(),
            alt_len: alt.len(),
            spoiler: true,
            inline: true,
        };
        let items = [item];
        let facts = Facts { items: items.as_ptr(), items_len: 1, ..facts };
        let core_items = [CoreMediaFact { id, format: Format::Gif, alt: Some(alt), spoiler: true, inline: true }];
        let core = CoreFacts { items: Some(&core_items), ..core };
        let discord = targets::target("discord-blocks").unwrap();
        let media = [CoreMedia { id, cover: None }];
        let output = project_with(text.as_bytes(), &[id], "discord-blocks", Some(&facts)).unwrap();
        assert_eq!(output, wire::project(text.as_bytes(), &media, discord, &core));
        assert!(output.contains(r#""reference":"media:a","alt":"A cat","spoiler":true"#), "{output}");
        let bad = [MediaFact { format: 4, ..item }];
        let bad = Facts { items: bad.as_ptr(), ..facts };
        assert_eq!(project(b"", "x-plain", Some(&bad)).unwrap_err(), "item format 4 is not GSX_FORMAT_*");
        let bad = Facts { items: ptr::null(), ..facts };
        assert_eq!(project(b"", "x-plain", Some(&bad)).unwrap_err(), "items is NULL");
        let none: [MediaFact; 0] = [];
        let unlisted = Facts { items: ptr::null(), items_len: 0, ..facts };
        let output = project_with(text.as_bytes(), &[id], "x-plain", Some(&unlisted)).unwrap();
        assert!(output.contains("media_attached") && !output.contains("media_missing"), "{output}");
        let empty = Facts { items: none.as_ptr(), items_len: 0, ..facts };
        let output = project_with(text.as_bytes(), &[id], "x-plain", Some(&empty)).unwrap();
        assert!(output.contains("media_missing") && !output.contains("media_attached"), "{output}");
        let mut error = ptr::null_mut();
        // SAFETY: a local slot.
        let output = unsafe { gsx_targets(&raw mut error) };
        assert_eq!(read(output, error).unwrap(), wire::targets());
    }

    #[test]
    fn rejects_bad_arguments() {
        let mut error = ptr::null_mut();
        // SAFETY: a NULL text with a length is what the call must reject; the error is read before
        // it is freed.
        unsafe {
            assert!(gsx_parse(ptr::null(), 1, ptr::null(), 0, &raw mut error).is_null());
            assert_eq!(CStr::from_ptr(gsx_error_message(error)).to_str().unwrap(), "text is NULL");
            gsx_error_free(error);
            assert!(gsx_parse(ptr::null(), 0, ptr::null(), 1, &raw mut error).is_null());
            assert_eq!(CStr::from_ptr(gsx_error_message(error)).to_str().unwrap(), "media is NULL");
            gsx_error_free(error);
            let output = gsx_parse(ptr::null(), 0, ptr::null(), 0, ptr::null_mut());
            assert!(!output.is_null());
            gsx_output_free(output);
        }
        let bad = [0xffu8];
        let entry = Media { id: bad.as_ptr(), id_len: 1, cover: ptr::null(), cover_len: 0 };
        assert_eq!(parse(b"a", &[entry]).unwrap_err(), "media id must be UTF-8");
        // SAFETY: the version is a static NUL-terminated string.
        assert_eq!(unsafe { CStr::from_ptr(gsx_version()) }.to_str().unwrap(), gsx_core::VERSION);
    }
}
