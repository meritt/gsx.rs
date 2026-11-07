# Conformance

How `gsx/1` resolves each case where the spec, micromark and other parsers disagree. Every rule is pinned by the corpus cases listed; an id such as `A06` appears in a case's `sources` as `known:A06` or `parser:A06`.

## Base syntax

1. Where the CommonMark 0.31.2/GFM spec and micromark 4.0.3 disagree, the spec wins: A06, A07, A14, A16, A17, A20, A21, K02, L05–L08, L12, Q01, Q02, H15, CS1, LK01, LK10, LK24, HT01, W01.
2. The spec also governs the cases implementations commonly get wrong: rule of three, astral punctuation, list interruption and looseness, lazy lines, link titles, empty destinations, code span indentation, CDATA, tab before a hard break.
3. Cases the spec leaves open follow micromark, cmark and markdown-rs where they agree:

   | Case | Rule |
   | --- | --- |
   | H04 | `---` after a definition-only paragraph is a thematic break |
   | E03 | C1 numeric references decode to U+FFFD |
   | LK08 | Destination parentheses nest up to 32 levels |
   | K05, K09, K11 | A checkbox needs content on its line or a continuation and only opens a paragraph |
   | W04 | A leading BOM is skipped; offsets still count it |
   | D01, D03, D04 | Directives follow micromark-extension-directive 4.0.0 |

4. Flanking (A08, A09): `~`, `=`, `+`, `|` form one class. Each is excluded from punctuation when `*`/`_` flanking is computed, as micromark does for `~`.

## gsx/1 rules

1. **Tolerant parse.** Parsing always yields a document. Malformed constructs become literal text with an error diagnostic and a range; `:time` included. Publishing callers refuse a document with errors; editors show them.
2. **Rejections carry ranges.** MARKDOWN_TOO_COMPLEX names the limit, its value and the range where it was exceeded. MARKDOWN_INVALID_TEXT names the code point and its range.
3. **Invalid text.** NUL, other forbidden control characters and lone surrogates are MARKDOWN_INVALID_TEXT, not U+FFFD (LK06, E07). Lone surrogates are detected at the binding boundary, before UTF-8 conversion.
4. **Budgets count what they name.** Cells count cells, pairs count pairs, lines count lines. Limits are calibrated on `testdata/bench/`.
5. **Linear parsing.** Every input within the limits parses in linear time. Raw HTML openers (`html-comment-opens`, `html-instruction-opens`, `html-comment-tags`), escaped code ticks, quote paragraphs, setext headings, nested lists and code-tick ramps have no budget of their own and are bench cases. Table rows padded to the header's width are capped by `documentTableCells`.
6. **One URL path.** Link destinations and URLs a target links on its own (Bluesky facets) pass one WHATWG parse. Accepted: absolute `http`/`https` without credentials (LK16 is literal). Every target and every field uses the serialized `href`. LINK_LITERALIZED is reported only when a syntactic link is refused.
7. **Autolinks.** `<https://…>` is a link under rule 6.
8. **Math literals bind like code spans.** `$…$` takes precedence over emphasis regardless of which opens first and is its own node. An escape affects one character: `\||` does not open a literal that swallows the following text.
9. **Every limited field is measured and fitted.** YouTube titles are fitted with the same rules as descriptions.
10. **Media directives** stay paragraph-only (two-space indent allowed); elsewhere they are literal with MEDIA_REFERENCE_CONTEXT_UNSUPPORTED.
11. **Task item ranges stay on their line.** A task item's text range ends at its line ending; it never extends past it, as the mdast position does.

## Limits

Exceeding any limit rejects the document with MARKDOWN_TOO_COMPLEX and the range of the construct that went over. `containerDepth` and `inlineDepth` bound the tree every stage walks recursively: a document at both limits parses, serializes and projects onto every target in 96 KiB of stack in a release build, within the 128 KiB a musl thread gets by default; `tests/wire.rs` runs it on a 128 KiB thread in release builds. The `testdata/bench/` budget cases sit one below each limit; the reject cases exceed one.

| Limit | Value | Counts | Where counted |
| --- | --- | --- | --- |
| `documentUnits` | 262144 | UTF-16 code units of the input; eight times the largest field limit (Telegram rich messages, 32768) | Before any other work on the input |
| `documentLines` | 16384 | Lines: line endings plus one | Before tokenizing |
| `containerDepth` | 32 | Nested block quotes, list items and footnote definitions | Document tokenizer; no deeper container opens |
| `inlineDepth` | 16 | Nested emphasis, strong, strikethrough, underline, highlight, spoiler, links and images | Final events |
| `blockDelimiters` | 1024 | Characters of `*`, `_`, `~` sequences that reach attention, per paragraph, heading or cell | Inline tokenizer; later sequences stay text |
| `blockLabels` | 4096 | `[`, `![`, `[^` label starts, per paragraph, heading or cell | Inline tokenizer; later starts stay text |
| `blockLabelEnds` | 256 | `]` reached by text, per paragraph, heading or cell | Inline tokenizer; later ends stay text |
| `documentEscapes` | 4096 | Character escapes | Final events |
| `documentHardBreaks` | 1024 | Hard breaks, escaped and trailing | Final events |
| `documentListItems` | 8192 | List items | Final events |
| `documentMathLiterals` | 1024 | `$…$` literals | Final events |
| `documentPairedDelimiters` | 1024 | Matched `++`, `==`, `\|\|` pairs | Final events |
| `documentTableCells` | 8192 | Table cells, each row counted at least at the header's column count | Final events |
