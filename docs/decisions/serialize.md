# Serialize

`serialize` writes a document back to `gsx/1`: the formatter for editors and AI skills.

## API

- `serialize::canonical(&Document) -> Option<String>`: the canonical form, or `None` when it does not reparse to the same document.
- `serialize::serialize(&Document) -> String`: the canonical form; otherwise the canonical form of the document reparsed from its normalized text; otherwise that text.

Same document means structural equality: kind, provenance and block gap per node, recursively. Ranges are ignored, and adjacent authored text nodes are merged first, because tokenization may split one text.

## Guarantees

- For every accepted document `d`, `parse(serialize(d))` equals `d` and `serialize(parse(serialize(d))) == serialize(d)`.
- Every accepted corpus document and every bench typical and long case takes the canonical path.
- Linear time: a canonical attempt reparses each paragraph and heading once and the whole output once; `serialize` makes at most two attempts.

## Canonical choices

| Construct | Form |
| --- | --- |
| Emphasis, strong | `*a*`, `**a**`; `_`, `__` where `*` would merge with an adjacent run, close an open ancestor or fail to flank |
| Bullet list | `-`; `*` right after another bullet list |
| Ordered list | `n.` from the list start; `n)` right after another ordered list |
| Task item | `- [ ] `, `- [x] ` |
| Heading | ATX; setext only when the content spans lines at depth 1 or 2 |
| Thematic break | `---`; `___` right after a paragraph or first in a list item |
| Code block | fenced with backticks one longer than the longest run in the value, at least three; tildes when the info string holds a backtick |
| Inline code | backticks one longer than the longest run; space padding only when needed |
| Table | `\| a \| b \|` rows; delimiter `---`, `:---`, `---:`, `:---:` |
| Hard break | backslash before the line ending |
| Link | `[text](dest "title")`; `<dest>` when the destination needs brackets; `<url>` when the text equals the destination |
| Time | `:time[label]{datetime="…"}`, `:time[label]{unix="…"}` |
| Media | `::media{id="…"}`, `::media{ids="…" layout="collage\|slideshow"}` |
| Gaps | the blank lines the block gap records |

## Escaping

Authored characters are escaped as numeric character references (`&#42;`), never with a backslash: a backslash escape parses as a literal node and would change provenance. A character is escaped only where it could start syntax:

- always: `` ` `` `*` `[` `]` `<` `~` `|` `$` and U+FEFF;
- `\` unless followed by an alphanumeric or a space; `_` unless between alphanumerics; `&` before `#` or an alphanumeric;
- `+` and `=` when doubled or at a line start; `#` `>` `-` at a line start; `.` or `)` after one to nine digits at a line start;
- `:` before an ASCII alphanumeric, a non-ASCII character or `/`; `@` after an email local-part character; `.` after `www`, so no GFM autolink literal swallows a following reference;
- leading and trailing spaces and tabs of a line, and line endings that would not be soft breaks;
- an alphanumeric next to an emphasis delimiter when no delimiter can flank otherwise.

## Literals

Literal nodes are written byte for byte from their source range, line endings normalized. Continuation lines keep their container prefixes, lazy lines included. A literal that begins a paragraph continuation line is indented four spaces so it cannot start a block. A task item paragraph that opens with a hard break starts at the checkbox separator, which is not written twice.

A paragraph or heading whose canonical inline form does not reparse to the same inline tree is written from its source too. On random `gsx/1` fragment soup the document-level fallback takes 0.5% of multi-line and 0.01% of single-line documents.
