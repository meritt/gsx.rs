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
| Raw JSON | `wire::{parse, serialize, project, targets}` | `parseJSON`, `serializeJSON`, `projectJSON` | `parse_json`, `serialize_json`, `project_json` |

Rust serializes and projects an existing document. Other bindings accept source text and optional parse-time media declarations on each call. Typed methods raise a parse error for rejected input; raw methods retain the error object. Document diagnostics are returned without raising.

Signatures and types: [TypeScript](../../packages/npm/src/gsx.d.ts), [Swift](../../packages/swift/Sources/GSX/GSX.swift), [Python](../../packages/python/src/gsx/__init__.py).

Swift nests every public type in the `GSX` enum, which shares the module's name: client code writes `GSX.Document`, `GSX.Measurement`, `GSX.Unit` or `GSX.Layout`, and Foundation or SwiftUI types of the same names remain unambiguous. `GSX.targets` is a throwing property: a failed call or decode throws instead of trapping.

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

[`schema/gsx-1.schema.json`](../../schema/gsx-1.schema.json) defines the JSON written by [`wire.rs`](../../crates/gsx-core/src/wire.rs). Results are `{document: ...}`, `{serialized: ...}`, `{projection: ...}`, `{targets: ...}` or `{error: ...}`. Keys use camelCase; ranges are half-open `[start, end)` pairs.

A node carries `type`, `range` and its kind-specific fields, using `null` for absent nullable values. `provenance` appears only as `"literal"`, `gapBefore` only when set, and `children` only when non-empty. Exhaustive Rust matches enforce model-to-wire coverage; [schema tests](../../crates/gsx-core/tests/schema.rs) validate the separate schema against corpus results.

| Range | Coordinate system |
| --- | --- |
| Nodes, markers, diagnostics, errors, changes | UTF-16 code units of the original input |
| Field `links` | UTF-16 code units of that field's output text |
| Field `entities` | Output text in `target.offsets`: UTF-8 bytes for Bluesky, UTF-16 for Telegram and Threads |

`targets` includes `name`, `form`, `offsets`, `digest`, `manifest`, `codes` and `threads` in target-table order. `codes` lists every code a blocked change can carry; `threads` states whether thread overflow is supported. Use these capabilities directly; `manifest` is readable configuration, not a parsing interface. The digest incorporates the manifest, Unicode version and package version, including form versions and declared rules. It has no manually incremented revision; a package version change changes the digest even if a renderer edit leaves the declarations unchanged.

Invalid UTF-8 and lone surrogates produce `MARKDOWN_INVALID_TEXT` with a one-unit range at the valid prefix's UTF-16 length. An encoded surrogate includes `details.codePoint`, as rejected controls do. JavaScript detects lone surrogates before encoding; Python combines valid surrogate pairs and passes lone surrogates to the core with `surrogatepass`. Swift strings cannot represent them.

## C ABI

The generated [gsx.h](../../crates/gsx-ffi/include/gsx.h) defines the ABI. `gsx_parse`, `gsx_serialize`, `gsx_project` and `gsx_targets` return an opaque output handle. Read its borrowed bytes with `gsx_output_json`, then release the handle with `gsx_output_free`. A binding failure returns null and an error handle; read `gsx_error_message` and release it with `gsx_error_free`. Parsing failures remain JSON results. Operation dispatch catches Rust panics; pointer validity remains the caller's responsibility.

Optional C limits use negative values for absence. Null string pointers mean absent strings. For `facts.items`, a null pointer with zero length means no selection fact; a non-null pointer with zero length means an explicitly empty selection. `gsx_version` identifies the linked build.

## Wasm boundary

The [raw Wasm exports](../../crates/gsx-wasm/src/lib.rs) receive UTF-8 in a reusable buffer and facts as plain values. `input(capacity)` grows the buffer as needed and retains its largest size. Pass the number of bytes actually written; do not retain the pointer across calls.

| Value | Encoding |
| --- | --- |
| Media kind, also C `GSX_MEDIA_*` | 0 absent, 1 image, 2 video, 3 GIF, 4 audio |
| Placement, also C `GSX_PLACEMENT_*` | 0 absent, 1 post, 2 reel, 3 story |
| Overflow, also C `GSX_OVERFLOW_*` | 0 truncate, 1 thread |
| Item format, also C `GSX_FORMAT_*` | 0 image, 1 GIF, 2 video, 3 audio |
| Wasm item flags | Bit 0 spoiler, bit 1 inline; C uses separate boolean fields |

Item IDs, formats, alts and flags are parallel arrays of equal length. An absent alt is `undefined`; `listed` distinguishes an omitted items fact from an empty selection. Parse-time media IDs and covers also have equal lengths. Unknown targets, invalid facts and mismatched arrays are binding errors, distinct from parse-error JSON.
