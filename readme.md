# gsx.rs

The `gsx/1` markup language for posts and messages and its projections onto 19 publishing targets: Telegram, Discord, Slack, Bluesky, Threads, X, Mastodon, LinkedIn, Facebook, Instagram, VK, YouTube, TikTok, MAX. One Rust parser for the editor, the browser, Node.js and Python. A projection fits every field to the platform's limit in the platform's own unit and reports each change it made. Ranges are UTF-16 offsets of the input; results are byte-identical in every binding.

## Installation

Rust:

```bash
cargo add gsx-core
```

JavaScript:

```bash
pnpm add @meritt/gsx-wasm
```

Swift:

```swift
.package(url: "https://github.com/meritt/gsx.rs", exact: "0.2.0")
```

Python:

```bash
pip install gsx
```

## Usage

Rust:

```rust
use gsx_core::{document, project, serialize, targets};

let document = document::parse("#  Release\n\n__gsx__ ships [today](https://example.com).", &[])?;
serialize::serialize(&document); // "# Release\n\n**gsx** ships [today](https://example.com)."

let x = targets::target("x-plain").unwrap();
let projection = project::project(&document, x, &project::Facts::default());
projection.fields[0].text; // "Release\n\ngsx ships today (https://example.com/)."
// changes: heading_as_text, strong_unwrapped, link_destination_as_text
```

JavaScript:

```js
import { load } from '@meritt/gsx-wasm';

const gsx = await load();
const text = '# Release\n\n**gsx** ships [today](https://example.com) ||spoiler||.';

gsx.project(text, 'telegram-entities', { entitiesAllowed: true }).fields[0];
// { name: 'text', text: 'Release\n\ngsx ships today spoiler.', links: [], entities: [
//   { style: 'bold', range: [0, 7], value: null },
//   { style: 'bold', range: [9, 12], value: null },
//   { style: 'link', range: [19, 24], value: 'https://example.com/' },
//   { style: 'spoiler', range: [25, 32], value: null } ] }

gsx.project('word '.repeat(80), 'x-plain');
// measurements: [{ field: 'text', unit: 'x-weighted', value: 276, maximum: 280 }]
// changes: [{ class: 'approximated', reason: 'text_truncated', range: [274, 400], … }]

gsx.project('Reply **body**', 'threads-entities', { targetUrl: 'https://example.com/post/1' }).fields[0].text;
// 'Reply body\n\nhttps://example.com/post/1'; changes: target_url_as_text, strong_unwrapped

const photo = [{ id: 'p', format: 'image', alt: 'A long description' }];
gsx.project('Photo\n\n::media{id="p"}', 'mastodon-plain', { descriptionLimit: 8, items: photo }, [{ id: 'p' }]).items;
// [{ reference: 'media:p', alt: 'A long…' }]; changes: media_attached, media_alt_truncated

const long = Array.from({ length: 6 }, (_, n) => `Paragraph ${n} has a few words in it, enough to fill a post.`).join('\n\n');
gsx.project(long, 'bluesky-entities', { overflow: 'thread' }).parts.length;
// 2: paragraphs 0–4, then paragraph 5; changes: text_threaded

gsx.parse('a\u0000'); // throws ParseError { code: 'MARKDOWN_INVALID_TEXT', range: [1, 2] }
```

`load()` takes `gsx.wasm` next to the module; it also accepts a URL, a `Response`, bytes or a `WebAssembly.Module`.

Swift:

```swift
import GSX

let projection = try GSX.project(
    "**gsx** ships [today](https://example.com).",
    target: "telegram-entities",
    facts: Facts(entitiesAllowed: true)
)
projection.fields[0].text                   // "gsx ships today."
projection.fields[0].entities.map(\.range)  // [0..<3, 10..<15]
```

Python:

```python
import gsx

gsx.serialize("#  Release\n\n__gsx__ ships [today](https://example.com).")
# '# Release\n\n**gsx** ships [today](https://example.com).'

gsx.parse("a\x00")  # raises gsx.ParseError: MARKDOWN_INVALID_TEXT, Range(start=1, end=2)
```

Every binding also returns the raw JSON (`parseJSON`, `parse_json`, …) described by [`schema/gsx-1.schema.json`](schema/gsx-1.schema.json). A change carries one of five classes: `approximated`, `textual_fallback`, `unwrapped`, `omitted`, `blocked`. `targets()` lists every target with its manifest, digest, the codes its `blocked` changes can carry and whether it splits threads.

## Performance

Parse medians on an Intel i9-10910, rustc 1.99.1, Node.js 26.13.0; full reports, with projection times per target, in [`gsx-15f3837.md`](testdata/bench/baseline/gsx-15f3837.md) (native) and [`gsx-wasm-15f3837.md`](testdata/bench/baseline/gsx-wasm-15f3837.md). `cargo xtask bench` and `cargo xtask bench --wasm` write new ones.

| Case | UTF-16 units | Native, ms | Wasm in Node.js, ms | Native peak heap, MiB |
|---|---:|---:|---:|---:|
| 91 typical posts | 112 950 | 2.46 | 4.64 | 0.2 |
| Long document, 4k | 3 850 | 1.07 | 1.54 | 0.5 |
| Long document, 32k | 32 450 | 9.20 | 11.3 | 4.1 |
| Long document, 128k | 130 900 | 48.2 | 46.8 | 16.6 |
| Prose, 128k | 131 057 | 2.05 | 3.72 | 1.6 |
| 16 383 ATX headings | 65 532 | 66.5 | 60.5 | 31.4 |
| 8 192 nested list items | 40 960 | 58.3 | 56.5 | 24.2 |
| 8 191 setext headings | 32 764 | 44.7 | 42.7 | 17.8 |
| 1 000 quotes 63 deep | 128 000 | 42.2 | 46.0 | 42.2 |
| 21 844 unclosed `<?` | 65 535 | 3.72 | 8.26 | 4.6 |
| 40 000 `*a`, rejected | 120 000 | 0.94 | 0.92 | 0.7 |

From 4k to 32k the input grows 8.4× and the native time 8.6×; from 32k to 128k, 4.0× and 5.2×. Budget cases sit one below their [limits](docs/decisions/conformance.md#limits). Every target fits a 128k document in at most 20 ms (`telegram-html-rich` 19.7 ms, `facebook-plain` 10.6 ms, `x-plain` 3.7 ms). `gsx.wasm` is 337 KB in gzip and instantiates in 5 ms.

## Author

[Alexey Simonenko](https://simonenko.xyz)

## License

MIT. See `LICENSE`.
