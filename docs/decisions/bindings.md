# Bindings

## Decision

| Package | Boundary | Input | Output |
| --- | --- | --- | --- |
| crates.io `gsx-core` | Rust API | `&str` | Rust types |
| npm `@meritt/gsx-wasm` | `gsx-wasm`, wasm-bindgen | JS string | JSON, parsed in JS |
| Swift `Package.swift` | `gsx-ffi` C ABI, XCFramework | UTF-8 pointer and length | JSON, decoded with `Codable` |
| PyPI `gsx` | `gsx-ffi` C ABI via `ctypes` | UTF-8 bytes | JSON, decoded to dataclasses |

One wire format for every binding: JSON of the `gsx/1` model, written by `gsx_core::wire` and described by [`schema/gsx-1.schema.json`](../../schema/gsx-1.schema.json). Offsets are UTF-16 code units of the input everywhere, Rust included, because JavaScript strings, `NSString` and `String.utf16` index that way.

## Grounds

- JSON costs one serialization per call and gives four languages one contract. A binary marker buffer for editors is added only if a keystroke reparse shows JSON dominating.
- UTF-8 is the native string form of Rust and Swift and the cheapest copy from JS; UTF-16 is needed only for offsets, which the line index maps.
- PyO3 `abi3` would tie wheels to a minimum Python and a compiled extension; `ctypes` reuses the C ABI Swift needs anyway.

## C ABI

- `gsx_parse`, `gsx_serialize`, `gsx_project` and `gsx_targets` return an opaque `gsx_output`; `gsx_output_json` reads its JSON and `gsx_output_free` releases it.
- A parse error travels in the JSON. A binding error (NULL pointer, non-UTF-8 media id, unknown target, bad fact, panic) is a `gsx_error` out-parameter, read with `gsx_error_message` and released with `gsx_error_free`.
- Every entry point runs behind `catch_unwind`.
- `gsx_version` identifies the build. Swift links the library statically and Python ships it in the wheel; `gsx.js` refuses a `gsx.wasm` of another version.

## Packages

- npm: `gsx.js` encodes the text with `TextEncoder.encodeInto` straight into one reused Wasm buffer and passes only the written length; `input(capacity)` returns its address, valid until the next call, and keeps the largest capacity seen. A per-call allocation and copy cost more than parsing a short input. One module serves Node.js and browsers. A trap (out of memory, a panic) leaves the instance unusable, so `gsx.js` instantiates the compiled module afresh and rethrows: the failing call throws, the next one runs on a clean instance and the memory the trap held is released.
- Swift: `GSXFFI.xcframework` holds a universal static library for `arm64` and `x86_64`. `Package.swift` pins the release binary by URL and checksum.
- Python: wheels `py3-none-<platform>` for macOS `universal2` and for `manylinux_2_28` and `musllinux_1_2` on `x86_64` and `aarch64`, each built in its PyPA image. No ABI tag, so any Python 3 loads them. musl links C statically by default, which rules out a cdylib, so the musl build turns `crt-static` off. CI runs the images with `docker run` rather than as job containers, because JavaScript actions cannot run in musl containers on Arm.
- Release artifacts have fixed entry order, times and modes, and remap the workspace, Cargo home and Rust sources to `/gsx`, `/cargo` and `/rustc/<commit>`, so the checksum pinned in `Package.swift` before tagging is the checksum of the tagged build.
- `gsx.wasm` has a gzip budget of 345 000 bytes, enforced by `cargo xtask replay`.

## Wire format

- One object: `{"document":{"children","markers","diagnostics"}}` or `{"error":{"code","range","details"}}`.
- A range is `[start, end]`; keys are camelCase. A node carries `type`, `range` and every field of its kind, `null` when absent. `provenance` appears only as `"literal"`, `gapBefore` only when set, `children` only when non-empty.
- The writer maps the model with exhaustive matches: a model change fails to compile until the mapping and the schema follow.
- `serialize(text, media)` returns `{"serialized":"…"}` or the parse error ([serialize.md](serialize.md)).
- `project(text, target, facts, media)` parses and projects in one call and returns `{"projection":{"fields","measurements","changes","media"}}`, plus `"blocks"` for a blocks target, or the parse error. It takes text rather than a document because the bindings hold no Rust handles and a parse costs little next to a projection. A field carries `links` as UTF-16 ranges of its text and `entities` in the target's offset unit; a blocked change carries `code`.
- Facts cross as plain values: a `gsx_facts` struct at the C ABI (negative limit, NULL title or alt for none, NULL `items` with length 0 for no items fact, distinct from a non-NULL empty list); arguments in Wasm (media kind 0 none, 1 image, 2 video, 3 GIF, 4 audio, the `GSX_MEDIA_*` values at the C ABI; placement 0 none, 1 post, 2 reel, 3 story, the `GSX_PLACEMENT_*` values; a `listed` flag for the items fact; media as parallel arrays of ids, formats 0 image, 1 GIF, 2 video, 3 audio, alts a string or `undefined`, flags bit 0 spoiler, bit 1 inline).
- `targets()` returns `{"targets":[{"name","form","offsets","digest","manifest"}]}` in table order.
- Every binding exposes the raw JSON (`parseJSON`, `parse_json`, `projectJSON`, `project_json`) beside the typed decoder. `cargo xtask replay` installs each package as a consumer, replays the corpus and compares the raw JSON with the Rust writer byte for byte. Projection cases are the corpus target inputs against every target, once without facts and once with every fact set.

## Invalid text

Input crosses the C ABI as UTF-8, and a lone surrogate cannot be encoded, so every binding reports `MARKDOWN_INVALID_TEXT` with the same UTF-16 range the core would:

- Invalid UTF-8 at the C ABI is reported with a one-unit range at the length of the valid prefix; an encoded surrogate there adds its `codePoint`, as control characters do.
- Python encodes with `surrogatepass` after re-pairing split surrogates, so the first lone surrogate lands exactly there and the core reports it.
- `gsx.js` finds the first lone surrogate and asks the core for the same error.
- Swift strings cannot hold a lone surrogate.
