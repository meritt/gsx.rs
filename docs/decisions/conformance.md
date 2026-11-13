# Conformance

`gsx/1` uses CommonMark 0.31.2 and GFM tokenization with the model restrictions and extensions below. The specification takes precedence over implementation-specific parser behavior. [Parser ownership](parser.md) defines the tokenizer boundary.

## Syntax

| Construct | Input |
| --- | --- |
| Emphasis, strong, strikethrough | `*text*`, `**text**`, `~~text~~` |
| Underline, highlight, spoiler | `++text++`, `==text==`, `\|\|text\|\|` |
| Date and time | `::time[Launch]{datetime="2026-11-08T12:00:00Z"}` or `::time[Launch]{unix="1794139200"}` |
| Media | `::media{id="photo"}` |
| Media group | `::media{ids="a,b" layout="collage"}` or `layout="slideshow"` |

Headings, lists, task items, quotes, code, tables and hard breaks retain their structural nodes. Raw HTML, image syntax, reference syntax, footnotes and unsupported directives remain literal text. Math spans bind like inline code but produce literal text in the document. Backslash escapes preserve literal provenance; character references produce authored text.

Explicit links and angle autolinks accept only absolute HTTP(S) URLs with a host and without credentials. Destinations pass through the WHATWG `url` parser, including IDNA and percent-encoding, and use its serialized URL. Projection-generated link destinations use the same policy. A refused syntactic link becomes literal text with `LINK_LITERALIZED`; bare URL and mention detection belongs to the projection.

Media directives occupy top-level blocks, separated from non-media blocks by a blank line; up to three leading spaces are allowed. Invalid contexts produce `MEDIA_REFERENCE_CONTEXT_UNSUPPORTED`. A media ID has 1 to 128 bytes: an ASCII letter or digit, then ASCII letters, digits, `.`, `_`, `:` or `-`. A group lists 2 to 64 IDs separated by commas without spaces. A directive with an invalid ID or group size produces `MEDIA_DIRECTIVE_INVALID`. Parse-time `media` declares valid IDs and optional covers. Projection-time `facts.items` describes the selected media and is a separate input ([media](projection.md#media)).

## Parsing and diagnostics

`document::parse(input, media) -> Result<Document, Error>` returns a document with diagnostics for malformed constructs. Invalid controls or exceeded budgets reject the input instead:

| Result | Meaning |
| --- | --- |
| Document with diagnostics | Unsupported or malformed content is retained as text; inspect diagnostic severity before publishing |
| `MARKDOWN_INVALID_TEXT` | C0 controls except tab, LF and CR; DEL and C1 controls; invalid encoding or lone surrogates at a binding boundary |
| `MARKDOWN_TOO_COMPLEX` | A budget was exceeded; details identify the limit and maximum |

CRLF and CR normalize to LF. Source ranges still address the original input. A leading BOM is skipped but remains counted in offsets. All range conventions are defined in [bindings.md](bindings.md#wire-format).

The tokenizer bounds link-destination parentheses at 32 levels (`LK08`). A task checkbox needs content on its line or a continuation and opens only a paragraph (`K05`, `K09`, `K11`). The paired delimiters `~`, `=`, `+`, `|` are excluded from punctuation when computing `*` and `_` flanking (`A08`, `A09`). Table spoiler handling runs inside the table grammar.

| Boundary case | Resolution |
| --- | --- |
| `---` after a definition-only paragraph (`H04`) | Thematic break |
| C1 numeric character reference (`E03`) | U+FFFD; literal C1 input is rejected |
| Math span overlapping emphasis | Math binds first, regardless of opener order |
| Escaped paired delimiter | The escape affects one character only |
| Task checkbox content | The first paragraph's range ends at its own content, excluding the next block |

Corpus `sources` retain the `known:` and `parser:` case identifiers for these resolutions. Every gsx extension directive opens with `::`, inline ones included: a text directive starts at two colons not preceded by an unescaped colon, so a single colon is ordinary text. Otherwise directive grammar follows the cases pinned for micromark-extension-directive 4.0.0 (`D01`, `D03`, `D04`); `W04` pins BOM handling.

## Limits

Exceeding a budget rejects the document with the affected source range. Values are defined by [`source::Limit`](../../crates/gsx-core/src/source.rs); counts are enforced during tokenization and event validation.

| Limit | Maximum | Count |
| --- | ---: | --- |
| `documentUnits` | 262144 | Input UTF-16 code units |
| `documentLines` | 16384 | Normalized line endings plus one |
| `containerDepth` | 32 | Nested quotes, list items and footnote definitions |
| `inlineDepth` | 16 | Nested emphasis, strong, strikethrough, underline, highlight, spoiler, text directives, links and images |
| `blockDelimiters` | 1024 | Characters of `*`, `_`, `~` sequences reaching attention per paragraph, heading or cell |
| `blockLabels` | 4096 | `[`, `![`, `[^` label starts per paragraph, heading or cell |
| `blockLabelEnds` | 256 | `]` reached by text per paragraph, heading or cell |
| `documentEscapes` | 4096 | Character escapes |
| `documentHardBreaks` | 1024 | Hard breaks |
| `documentListItems` | 8192 | List items |
| `documentMathLiterals` | 1024 | Math spans |
| `documentPairedDelimiters` | 1024 | Matched underline, highlight and spoiler pairs |
| `documentTableCells` | 8192 | Cells, each row counted at least at the header width |

Input size and lines are checked before tokenization. Container depth is bounded while opening containers; inline tokenization bounds attention and labels, leaving later delimiters as text while recording the exceeded budget. Final event validation enforces the remaining limits and rejects any recorded overflow. The [corpus](../../testdata/corpus) pins syntax and diagnostic results; [repository checks](../../AGENTS.md#corpus) define regeneration and acceptance.

Parsing must scale linearly within these budgets; adversarial scaling cases are checked in [document tests](../../crates/gsx-core/tests/document.rs). Container and inline depth caps bound every recursive stage. [Wire tests](../../crates/gsx-core/tests/wire.rs) parse, serialize and project depth-limit inputs on a 128 KiB thread stack in release builds.
