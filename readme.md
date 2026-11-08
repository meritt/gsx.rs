# gsx.rs

The `gsx/1` markup for posts and messages. One Rust core parses, serializes and projects documents onto publishing targets, fits their fields to configured limits and reports transformations. Bindings expose the same results in JavaScript, Swift and Python.

## Installation

| Package | Command |
| --- | --- |
| Rust | `cargo add gsx-core` |
| JavaScript | `pnpm add @meritt/gsx-wasm` |
| Python | `pip install gsx` |

SwiftPM dependency:

```swift
.package(url: "https://github.com/meritt/gsx.rs", exact: "0.2.0")
```

Package platforms and loading: [bindings](docs/decisions/bindings.md#packages).

## Usage

### Rust

```rust
use gsx_core::{document, project, targets};

fn main() -> Result<(), document::Error> {
    let text = "Hello **world**.";
    let document = document::parse(text, &[])?;

    let target = targets::target("telegram-entities").unwrap();
    let facts = project::Facts {
        entities_allowed: true,
        ..Default::default()
    };
    let projection = project::project(&document, target, &facts);

    assert_eq!(projection.fields[0].text, "Hello world.");

    Ok(())
}
```

### JavaScript

```js
import { load } from '@meritt/gsx-wasm';

const gsx = await load();
const text = 'Hello **world**.';

const projection = gsx.project(text, 'telegram-entities', {
  entitiesAllowed: true
});

console.log(projection.fields[0]);
```

```json
{
  "name": "text",
  "text": "Hello world.",
  "links": [],
  "entities": [{
    "style": "bold",
    "range": [6, 11],
    "value": null
  }]
}
```

### Swift

```swift
import GSX

let text = "Hello **world**."
let facts = Facts(entitiesAllowed: true)

let projection = try GSX.project(text, target: "telegram-entities", facts: facts)

precondition(projection.fields[0].text == "Hello world.")
```

### Python

```python
import gsx

text = "Hello **world**."
facts = gsx.Facts(entities_allowed=True)

projection = gsx.project(text, "telegram-entities", facts)

assert projection.fields[0].text == "Hello world."
```

Check document diagnostics before publishing and reject projections with `blocked` changes. The caller resolves media and account references and sends provider requests; gsx performs no I/O.

## Performance

Parse medians for v0.2.0 on Apple M1 Pro, macOS 27.2, rustc 1.99.1 and Node.js 26.13.0. Wasm calls include JSON encoding and decoding. Full [native](testdata/bench/baseline/gsx-615cd7b.md) and [Wasm](testdata/bench/baseline/gsx-wasm-615cd7b.md) reports include input outcomes and memory measurements.

| Case | UTF-16 units | Native, ms | Wasm in Node.js, ms | Native peak heap, MiB |
| --- | ---: | ---: | ---: | ---: |
| 91 typical posts | 112,950 | 1.52 | 3.56 | 0.2 |
| Long document, 4k | 3,850 | 0.43 | 1.24 | 0.5 |
| Long document, 32k | 32,450 | 3.54 | 9.34 | 4.0 |
| Long document, 128k | 130,900 | 14.95 | 37.36 | 16.1 |
| Prose, 128k | 131,057 | 1.99 | 3.79 | 0.9 |
| 16,383 ATX headings | 65,532 | 17.99 | 45.27 | 31.4 |
| 8,192 nested list items | 40,960 | 17.68 | 47.20 | 24.2 |
| 8,191 setext headings | 32,764 | 14.22 | 34.82 | 17.8 |
| 2,000 quotes, 31 levels deep | 128,000 | 14.72 | 33.55 | 42.6 |
| 21,844 unclosed `<?` | 65,535 | 2.43 | 7.31 | 4.6 |
| 40,000 `*a`, rejected | 120,000 | 0.59 | 0.81 | 0.7 |

Native projection medians for the 128k document: `telegram-html-rich` 10.28 ms, `facebook-plain` 4.93 ms, `x-plain` 1.99 ms. Per-target results are in the native report.

`gsx.wasm` is 356,047 bytes with `gzip -9`. Cold load is 3.52 ms; the first parse is 5.36 ms. Reproduce with `cargo xtask bench` and `cargo xtask bench --wasm`.

## Reference

- [Contracts](docs/decisions/readme.md): syntax, serialization, projections, measurements and binding API.
- [Architecture and targets](docs/architecture.md).
- [JSON Schema](schema/gsx-1.schema.json).

## Author

[Alexey Simonenko](https://simonenko.xyz)

## License

MIT. See `LICENSE`.
