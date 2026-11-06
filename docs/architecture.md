# Architecture

gsx parses `gsx/1` text into a document whose ranges are UTF-16 offsets of the input, writes it back as canonical `gsx/1`, and projects it onto a publishing target: text fields fitted to the platform's limits plus every change the projection made. Results are byte-identical in Rust, WebAssembly, Swift and Python. The rules are specified in [decisions/](decisions/readme.md).

gsx returns a representation: text, markup or entities, measurements, a media plan and changes. The caller owns credentials, transport, media upload, publication options and the provider request (`sendMediaGroup`, `attach://…`).

## Components

```text
                      gsx-core            parser, document, serializer, measurement, projection;
                          │               no I/O, no unsafe
             ┌────────────┴────────────┐
         gsx-wasm                   gsx-ffi              C ABI; the only unsafe code
   gsx.wasm + gsx.js          ┌────────┴─────────┐
  (@meritt/gsx-wasm)   GSXFFI.xcframework      libgsx
                         + GSX (Swift)      + gsx (Python, ctypes)
```

Dependencies point down only. Bindings pass text in, take JSON out and decode it; they add no rules ([bindings.md](decisions/bindings.md)).

## Pipeline

```text
text ─► source ─► syntax ─► document ─┬─► serialize ─► canonical gsx/1
         │          │          │      │
         │          │          │      └─► project(target, facts) ─► fields, measurements, changes, media, blocks
         │          │          └─ node arena, validation, diagnostics
         │          └─ tokenizer: markers, content ranges, budgets
         └─ line endings, line index, UTF-8 ↔ UTF-16
```

| Module | Owns |
|---|---|
| `source` | line endings, line index, offset mapping, complexity budgets ([limits](decisions/conformance.md#limits)) |
| `syntax` | the `gsx/1` tokenizer, derived from markdown-rs 1.0.0 ([parser.md](decisions/parser.md)) |
| `document` | node arena, builder, validation, diagnostics |
| `serialize` | canonical `gsx/1` ([serialize.md](decisions/serialize.md)) |
| `measure` | length units ([measure.md](decisions/measure.md)) |
| `project` | target framework, change classes, media plan, fitting, manifest, digest ([projection.md](decisions/projection.md)) |
| `forms` | `plain`, `entities`, `markdown`, `html`, `blocks` |
| `targets` | one table per target: node type → output rule, change class, reason |
| `wire` | the JSON every binding returns, described by [`schema/gsx-1.schema.json`](../schema/gsx-1.schema.json) |

## Targets

A name is `<platform>-<form>[-<variant>]`: the current platform name in lowercase (`x`, not `twitter`), a form, and a variant only to separate two targets of one form on one platform.

| Target | Platform surface |
| --- | --- |
| `x-plain` | post |
| `mastodon-plain` | status |
| `linkedin-plain` | post, Little Text Format |
| `facebook-plain` | post, caption |
| `instagram-plain` | caption |
| `vk-plain` | wall post |
| `youtube-plain` | video title and description |
| `tiktok-plain` | video and photo post caption |
| `slack-plain` | file upload comment (`file_initial_comment`) |
| `telegram-entities` | story caption with entities via MTProto |
| `bluesky-entities` | post text with facets |
| `threads-entities` | post text with spoilers |
| `telegram-html` | Bot API messages and captions, `parse_mode: HTML` |
| `telegram-html-rich` | rich message HTML with native blocks |
| `max-html` | MAX messages, HTML |
| `discord-markdown` | message content |
| `slack-markdown` | message text in Slack markdown |
| `discord-blocks` | message components |
| `slack-blocks` | Block Kit message |
