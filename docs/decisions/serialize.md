# Serialization

The serializer writes a parsed document as `gsx/1`, preserving its structure and literal provenance. Implementation: [serialize.rs](../../crates/gsx-core/src/serialize.rs).

## Contract

| Rust API | Result |
| --- | --- |
| `serialize::canonical(&Document) -> Option<String>` | Canonical text only if reparsing preserves the document; otherwise `None` |
| `serialize::serialize(&Document) -> String` | Canonical text, then a canonical attempt from normalized source, then normalized source itself |

Structural equality compares node kind, provenance, block gap and children. It ignores ranges and merges adjacent authored text nodes. Reparse with the same media declarations. Serialization is idempotent; [serializer tests](../../crates/gsx-core/tests/serialize.rs) cover corpus round trips, generated inputs and source fallback.

For a parsed document `d`, `parse(serialize(d))` preserves that structure and `serialize(parse(serialize(d))) == serialize(d)`. Canonicalization is linear: it reparses each paragraph or heading once and the complete output once; the public serializer makes at most two canonical attempts. Source fallback preserves content when canonical rewriting cannot preserve the tree.

```text
#  Release             -> # Release
__gsx__ ships.         -> **gsx** ships.
3) first               -> 3. first
```

## Canonical form

| Construct | Form |
| --- | --- |
| Emphasis, strong | `*text*`, `**text**`; underscores where stars would merge, close an ancestor or fail to flank |
| Lists | `-` and `n.` from the list start; alternate with `*` and `n)` to keep adjacent lists separate |
| Task items | `- [ ] ` and `- [x] ` |
| Headings | ATX; setext for multiline content at levels 1 and 2 |
| Thematic breaks | `---`; `___` after a paragraph or first in a list item |
| Code blocks | Backtick fences one longer than the longest run in the value, at least three; tildes if the info string contains a backtick |
| Inline code | Backticks one longer than the longest run, with padding only when required |
| Tables | Padded cells; `---`, `:---`, `---:` or `:---:` alignment delimiters |
| Hard breaks | Backslash before LF |
| Links | `[text](dest "title")`, with `<dest>` where a bare destination is unsafe; `<url>` when the authored label matches the source destination and no title is present |
| Directives | Quoted attributes for time and media |
| Block gaps | Preserve `gapBefore` |

## Escaping and source preservation

Authored text uses numeric character references such as `&#42;` where characters would become syntax. Backslash escapes would change provenance. Escape contexts are:

- Always: backtick, `*`, `[`, `]`, `<`, `~`, `|`, `$` and U+FEFF.
- `\` unless followed by an alphanumeric or a space; `_` unless between alphanumerics; `&` before `#` or an ASCII alphanumeric.
- `+` and `=` when doubled or at a line start; `#`, `>` and `-` at a line start; `.` or `)` after one to nine initial ASCII digits.
- `:` before an ASCII alphanumeric, a non-ASCII character or `/`; `@` after an ASCII alphanumeric, `.`, `+`, `-` or `_`; `.` after case-insensitive `www`. These prevent autolinks from consuming a following character reference.
- Leading and trailing spaces or tabs, line endings that cannot remain soft breaks, and an alphanumeric next to emphasis when no delimiter can flank otherwise.

Literal nodes retain source spelling with normalized line endings and container prefixes, including lazy continuation lines. A literal at the start of a paragraph continuation receives four spaces to prevent block interpretation. A task paragraph opening with a hard break retains its checkbox separator exactly once. Paragraphs and headings whose canonical inline form changes the tree retain their source spelling too.

Every accepted corpus document and the typical and long benchmark cases must take the canonical path; the serializer tests enforce this independently of the general source fallback.
