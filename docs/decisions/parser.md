# Parser

An own parser in `gsx-core`, derived from `markdown` 1.0.0 (markdown-rs, MIT): first-party code, no crate dependency, no fork, no upstream tracking. The MIT notice is retained.

## Grounds

- markdown-rs emits a positioned event stream, so literals, markers and the range index come from tokens. Literals, escapes and markers are fixed during tokenization and budgets are counted there: no reparsing of fragments, no regex pass beside the grammar.
- The gsx constructs change the state machine, events, dispatch, GFM tables and attention. A fork would carry manual merges in the most active modules for no upstream benefit.
- comrak diverges on paired delimiters and tables, has no directives or marker tokens, and would need a fork too.

## Shape

- Kept: tokenizer, state machine, subtokenize, CommonMark and GFM constructs, utilities, the generated Unicode punctuation and character reference tables.
- Removed: HTML and mdast compilers, MDX, frontmatter, unused options.
- Added: paired delimiters (`||`, `++`, `==`), `:time`, `::media`. The table spoiler heuristic lives in the table construct, not in a pre-pass.
- Base behaviour follows the spec and current micromark, not markdown-rs 1.0.0. Known defects in micromark, markdown-rs, cmark, commonmark.js, pulldown-cmark and comrak are folded in, each pinned by a corpus case ([conformance.md](conformance.md)).
- Link destinations are parsed and serialized per the WHATWG URL Standard with the `url` crate, as `new URL()` does: scheme and credential checks, IDNA, percent-encoding, the serialized `href`.
