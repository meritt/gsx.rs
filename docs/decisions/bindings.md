# Bindings

Bindings pass text and explicit facts into `gsx-core` and decode its JSON. They own encoding, memory and error translation; all format behavior remains in Rust.

## Boundary decisions

| Choice | Reason |
| --- | --- |
| JSON across bindings | One inspectable contract without a second binary encoding or binding-owned document handles; a binary editor buffer requires evidence that JSON dominates keystroke latency |
| UTF-8 input, UTF-16 source ranges | Native Rust input with offsets directly usable by JavaScript and Foundation string APIs |
| Shared C ABI for Swift and Python | `ctypes` reuses the native library without a Python extension ABI or a separate implementation |
| Reused Wasm input buffer | `TextEncoder.encodeInto` writes directly into linear memory, avoiding a per-call allocation and intermediate copy |

## API

| Operation | Rust | JavaScript / Swift | Python |
| --- | --- | --- | --- |
| Parse | `document::parse` | `parse` | `parse` |
| Serialize | `serialize::serialize` | `serialize` | `serialize` |
| Project | `project::project` | `project` | `project` |
| Targets | `targets::TARGETS` | `targets()` / `GSX.targets` | `targets()` |
| Media ID rule | `document::{is_media_id, MEDIA_ID_MAX_BYTES, MEDIA_ID_PATTERN, MEDIA_GROUP_MIN, MEDIA_GROUP_MAX}` | `isMediaId`, `MEDIA_ID_MAX_BYTES`, `MEDIA_ID_PATTERN`, `MEDIA_GROUP_MIN`, `MEDIA_GROUP_MAX` / `GSX.isMediaId(_:)`, `GSX.mediaIdMaximumBytes`, `GSX.mediaIdPattern`, `GSX.mediaGroupMinimumCount`, `GSX.mediaGroupMaximumCount` | `is_media_id`, `MEDIA_ID_MAX_BYTES`, `MEDIA_ID_PATTERN`, `MEDIA_GROUP_MIN`, `MEDIA_GROUP_MAX` |
| Raw JSON | `wire::{parse, serialize, project, targets}` | `parseJSON`, `serializeJSON`, `projectJSON` | `parse_json`, `serialize_json`, `project_json` |

Rust serializes and projects an existing document. Other bindings accept source text and optional parse-time media declarations on each call. Typed methods raise a parse error for rejected input; raw methods retain the error object. Document diagnostics are returned without raising, by `parse` in the document and by `project` in the projection.

Signatures and types: [TypeScript](../../packages/npm/src/gsx.d.ts), [Swift](../../packages/swift/Sources/GSX/GSX.swift), [Python](../../packages/python/src/gsx/__init__.py).

Swift nests every public type in the `GSX` enum, which shares the module's name: client code writes `GSX.Document`, `GSX.Measurement`, `GSX.Unit` or `GSX.Layout`, and Foundation or SwiftUI types of the same names remain unambiguous. `GSX.targets` is a throwing property: a failed call or decode throws instead of trapping.

The media ID members expose the core's [media ID and group rules](conformance.md#syntax): on the object returned by JavaScript `load()`, on the `GSX` enum and at Python module level. The predicate answers for any string, including one with lone surrogates, which is never a media ID; a non-string argument raises `TypeError` in JavaScript and Python. The pattern states the predicate as an ECMA-262 regular expression for a JSON Schema `pattern` or form validation. It is ASCII-only with the byte limit folded into its quantifier: every character it matches is one byte, one UTF-16 unit and one code point. It compiles without flags, with `u` and with `v`, which HTML `pattern` attributes use. Apply it as a whole-string match: in Python use `re.fullmatch`, because `$` also matches before a trailing newline, and require an `NSRegularExpression` match to cover the whole string for the same reason.

## Packages

| Package | Runtime and artifact |
| --- | --- |
| `@meritt/gsx-wasm` | Browser or Node.js 26.13+; ESM and `gsx.wasm` |
| `GSX` | Swift tools 6.2, macOS 27+; universal arm64/x86_64 static XCFramework |
| `gsx` | Python with `ctypes`; macOS 11+ universal2, manylinux_2_28 or musllinux_1_2 on x86_64/aarch64 |

JavaScript `load()` reads `gsx.wasm` beside the module by default, or accepts a URL, response, response promise, bytes or compiled `WebAssembly.Module`. Keep the Wasm asset available when bundling. A version mismatch raises `VersionError`. A Wasm trap is rethrown after replacing the failed instance; the next call uses fresh memory rather than retrying the failed operation. The gzip size ceiling is enforced by the [npm replay](../../AGENTS.md#replay-and-performance).

SwiftPM prefers `packages/swift/GSXFFI.xcframework` when present; otherwise it uses the URL and checksum in [`Package.swift`](../../Package.swift). Python wheels include the shared library and require no Python extension ABI.

Linux wheels are built in their PyPA images. musl builds disable `crt-static` so the C ABI can be a shared library. CI invokes these images through `docker run`, keeping JavaScript actions on the host, including for musl on Arm.

## Wire format

[`schema/gsx-1.schema.json`](../../schema/gsx-1.schema.json) defines the JSON written by [`wire.rs`](../../crates/gsx-core/src/wire.rs). Results are `{document: ...}`, `{serialized: ...}`, `{projection: ...}`, `{targets: ...}` or `{error: ...}`. Keys use camelCase; ranges are half-open `[start, end)` pairs. A projection always includes `diagnostics`, written by the same writer as `document.diagnostics` and byte-identical to it, so a caller deciding whether to publish needs no separate parse. Rejected input gives the same `{error: ...}` from `project` as from `parse`. Rust `project::project` takes a parsed `Document` and leaves its diagnostics there.

A node carries `type`, `range` and its kind-specific fields, using `null` for absent nullable values, such as a datetime's `format`. An entity carries `style`, `range` and `value`; a Telegram `date` entity adds `format` only when its source has one. `provenance` appears only as `"literal"`, `gapBefore` only when set, and `children` only when non-empty. Exhaustive Rust matches enforce model-to-wire coverage; [schema tests](../../crates/gsx-core/tests/schema.rs) validate the separate schema against corpus results.

| Range | Coordinate system |
| --- | --- |
| Nodes, markers, diagnostics, errors, changes | UTF-16 code units of the original input |
| Field `links` | UTF-16 code units of that field's output text |
| Field `entities` | Output text in `target.offsets`: UTF-8 bytes for Bluesky, UTF-16 for Telegram and Threads |

`targets` includes `name`, `form`, `offsets`, `digest`, `manifest`, `codes`, `threads`, `facts`, `fields`, `limits`, `features` and `media` in target-table order. `codes` lists every code a blocked change can carry; `threads` states whether thread overflow is supported. The remaining capabilities are written from the same target table that drives rendering and the manifest, so they cannot drift from behavior:

| Key | Content |
| --- | --- |
| `facts` | Facts that can change the target's projection, in the order of the facts object and in their wire spelling (`maxCharacters`, not `max_characters`) on every binding |
| `fields` | Text fields in output order: `name`, `required`, and `media` (`any`, `image`, `video`), the media kind the field needs |
| `limits` | `field`, `unit`, `maximum`, `fact`, `placements` and `media` (`any`, `with`, `without` or a media kind) under which the limit applies, and `rendering` (`visible` measures the text a reader sees, `payload` the sent text). A fixed limit has `maximum` and a null `fact`; a fact with a default has both; a fact without a default has `fact` only; an unlimited count has neither. `placements: null` means every placement |
| `features` | Each declared rule: `feature`, its default `class` and `reason`, and `conditions` of `when`, `class` and `reason`. `when` is a closed snake_case vocabulary. A feature the target always renders natively has no rule and no entry |
| `media` | `formats`, `alt`, `spoiler`, `inline` and `groups`, described below |

At most one limit applies to one field, unit, placement and media kind; a test over the whole table enforces it. Limits cover text only: media count, size, duration and the number of thread parts are platform claims the caller keeps.

`media` answers how the target treats `items`. `formats` lists the item formats it publishes as media; on a target that reads `inline` (`inline: true`, the blocks targets), only an inline item of these formats is placed, and any other item is published by the target's fallback with a change. `alt` lists the formats whose alt the projection publishes, stated explicitly rather than inferred from the absence of an alt limit. `spoiler.mode` is `carried` (each item keeps its own spoiler for the listed `spoiler.formats`), `widened` (a marked item hides every item of the post) or `omitted` (no spoiler is published; `formats` is empty); outside the `blocks/1` form, story placement omits every spoiler. `groups` lists the media group layouts the target renders as one group, each with the item `formats` such a group may contain and the smallest and largest group it renders that way, and is empty when the target renders none. A group that contains an item of another format is not rendered as a group; on the blocks targets a group's `formats` equal `media.formats`.

Use these capabilities directly; `manifest` is readable configuration, not a parsing interface. The digest incorporates the manifest, Unicode version and package version, including form versions and declared rules. It has no manually incremented revision; a package version change changes the digest even if a renderer edit leaves the declarations unchanged.

Invalid UTF-8 and lone surrogates produce `MARKDOWN_INVALID_TEXT` with a one-unit range at the valid prefix's UTF-16 length; the range end saturates at 2³² − 1. The [size limit](conformance.md#limits) takes precedence, so an oversized input is rejected as too complex whatever its encoding. An encoded surrogate includes `details.codePoint`, as rejected controls do. JavaScript applies the size limit and detects lone surrogates without encoding the text, and the core writes both errors (see [Wasm boundary](#wasm-boundary)); a lone surrogate counts as one unit, as U+FFFD would. Python combines valid surrogate pairs and passes lone surrogates to the core with `surrogatepass`. Swift strings cannot represent them.

## C ABI

The generated [gsx.h](../../crates/gsx-ffi/include/gsx.h) defines the ABI. `gsx_parse`, `gsx_serialize`, `gsx_project` and `gsx_targets` return an opaque output handle. Read its borrowed bytes with `gsx_output_json`, then release the handle with `gsx_output_free`. A binding failure returns null and an error handle; read `gsx_error_message` and release it with `gsx_error_free`. Parsing failures remain JSON results. Operation dispatch catches Rust panics; pointer validity remains the caller's responsibility. `gsx_is_media_id` applies the core's media ID rule to UTF-8 bytes, and `GSX_MEDIA_ID_MAX_BYTES`, `GSX_MEDIA_GROUP_MIN` and `GSX_MEDIA_GROUP_MAX` mirror the core constants under a compile-time equality check. cbindgen emits no string constants, so `cargo xtask header` writes `GSX_MEDIA_ID_PATTERN` from `document::MEDIA_ID_PATTERN`, and `cargo xtask header --check` rejects a stale value. Callers that cannot read macros, such as `ctypes`, read the same values from `gsx_media_id_max_bytes`, `gsx_media_group_min`, `gsx_media_group_max` and `gsx_media_id_pattern`; like `gsx_version`, each takes no arguments and needs no struct layout shared with the caller, and the pattern is a static NUL-terminated string.

Optional C limits use negative values for absence. Null string pointers mean absent strings. For `facts.items`, a null pointer with zero length means no selection fact; a non-null pointer with zero length means an explicitly empty selection. An item ID outside the core's media ID rule is a binding error on every binding, with the message from `wire::refused_item`, so every generated `media:` reference matches the grammar consumers rewrite. `gsx_version` identifies the linked build.

## Wasm boundary

The [raw Wasm exports](../../crates/gsx-wasm/src/lib.rs) receive UTF-8 in a reusable buffer and facts as plain values. `input(capacity)` grows the buffer to at least `capacity` bytes and retains its largest size, up to 786432 bytes: three per unit of the `documentUnits` limit, the longest UTF-8 encoding of an accepted document. Any other capacity, including one of 2³² or more that a 32-bit conversion would wrap, raises a binding error and leaves the buffer unchanged. Pass the number of bytes actually written; do not retain the pointer across calls. Text over the limit never enters the buffer: `documentUnits()` returns the limit, and `oversized(units, codePoint, target)` returns the core's size error from the text's UTF-16 length and the code point that starts at unit `documentUnits() − 1`, because the error's range starts at the character crossing the limit and ends at the total length. `invalidText(offset, unit, target)` reports a lone surrogate the caller found at `offset`. Both resolve `target` when one is passed, so an unknown target stays a binding error ahead of the text's error. `isMediaId(len)` applies the media ID rule to the buffer; `mediaIdMaxBytes`, `mediaIdPattern`, `mediaGroupMin` and `mediaGroupMax` return the constants that `load()` copies onto its object. The JavaScript wrapper copies an ID only within `mediaIdMaxBytes` UTF-16 units; a longer one is more bytes and is answered `false` without a copy.

| Value | Encoding |
| --- | --- |
| Media kind, also C `GSX_MEDIA_*` | 0 absent, 1 image, 2 video, 3 GIF, 4 audio |
| Placement, also C `GSX_PLACEMENT_*` | 0 absent, 1 post, 2 reel, 3 story |
| Overflow, also C `GSX_OVERFLOW_*` | 0 truncate, 1 thread |
| Item format, also C `GSX_FORMAT_*` | 0 image, 1 GIF, 2 video, 3 audio |
| Wasm item flags | Bit 0 spoiler, bit 1 inline; C uses separate boolean fields |

Item IDs, formats, alts and flags are parallel arrays of equal length. An absent alt is `undefined`; `listed` distinguishes an omitted items fact from an empty selection. Parse-time media IDs and covers also have equal lengths. Unknown targets, invalid facts and mismatched arrays are binding errors, distinct from parse-error JSON.
