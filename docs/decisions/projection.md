# Projection

`project(document, target, facts) -> Projection` is pure. Target rules and explicit facts determine rendering, field limits, media handling and overflow. The caller validates document diagnostics and refuses publication when a change has class `blocked`. The projection result carries the document's diagnostics, so one `project` call and one parse answer a publish decision.

## Facts

[`Facts`](../../packages/npm/src/gsx.d.ts) describes placement, media, account limits, titles, target URLs and overflow. Names below use the wire and JavaScript spelling; Rust and Python use snake_case.

| Facts | Purpose |
| --- | --- |
| `media`, `mediaCount`, `placement` | Select rules for the publication context |
| `maxCharacters`, `charactersReservedPerUrl`, `captionLimit`, `descriptionLimit`, `hashtagLimit` | Supply target-specific limits |
| `title`, `targetUrl`, `spoilerText` | Supply a title, appended URL or Mastodon content warning |
| `entitiesAllowed` | Enable Telegram story entities; false by default |
| `items` | Selected media with ID, format, alt, spoiler and inline availability |
| `overflow` | `truncate` by default; `thread` on supporting targets |

Facts affect only targets that declare the corresponding behavior; `targets()` lists them per target in `facts`. Supply media context explicitly; `items` does not infer `media` or `mediaCount`. Discover capabilities through `targets()`, described in [bindings](bindings.md#wire-format):

```js
const target = gsx.targets().find(({ name }) => name === 'telegram-html');

console.log(target.facts);
console.log(target.limits.filter(({ field }) => field === 'text'));
console.log(target.media.spoiler);
```

## Output

| Property | Content |
| --- | --- |
| `fields` | Named text outputs, each with link ranges and entities |
| `measurements` | Counts and maxima for applicable limits; item or block indices when needed |
| `changes` | Transformations and blocking conditions, ordered by source position, uncapped |
| `diagnostics` | The parsed document's diagnostics, identical to `document.diagnostics` for the same text and media declarations; independent of target, facts and parts |
| `media` | IDs referenced by document directives; in a thread, those assigned to the part |
| `blocks` | Block targets: text, media groups, files and separators in order |
| `items` | Media attached to the message with published alt and spoiler state, in fact order: every item on non-block targets, items no block can show on Slack blocks; omitted from JSON when empty |
| `parts` | Split thread, including the first part also represented at the top level; omitted when unsplit |

Output ranges use the [binding coordinate systems](bindings.md#wire-format). Media references such as `media:photo` are resolved by the caller. In item measurements and changes, `item` indexes the output `items`; on block targets, `block` indexes `blocks` and `item` indexes that block's items.

### Changes

| Class | Meaning |
| --- | --- |
| `approximated` | Adapted representation, including fitting |
| `textual_fallback` | Construct represented as visible text |
| `unwrapped` | Formatting removed, content retained |
| `omitted` | Construct or value removed |
| `blocked` | Output cannot be published; `code` identifies the condition |

A change includes feature, reason, field and an optional input range. Truncation includes its measurement details. Every non-native transformation is reported, including omitted code info and unsupported nesting; native rendering produces no change. Highlight removal is `unwrapped`, retaining its content. Rules, conditional outcomes, capabilities and manifests derive from the same [target tables](../architecture.md#targets).

Reasons identify the cause without a target prefix unless it is target-specific. Each change belongs to the field that produced it. Alt and spoiler changes are per affected item, even when items share a directive's range. Identical block-group changes without an item index are reported once: two videos in a Slack slideshow produce one `media_attached`. [Target tests](../../crates/gsx-core/tests/targets.rs) check emitted feature/class/reason combinations against the declarations.

## Fields and rendering

Form configuration lives in [`project/table.rs`](../../crates/gsx-core/src/project/table.rs); renderers read it rather than maintaining separate target rules.

| Form | Configuration |
| --- | --- |
| `plain` | Escaping policy; links render as `label (href)`, or just `href` when equal |
| `entities` | Offset unit, style mapping, heading style, availability gate, code splitting, detection, entity and link caps, whitespace trimming, styled suffixes |
| `markdown` | Delimiters, native blocks, escapes, provider-owned spans, line-start neutralization, HTML escaping, datetime template, separators, quote depth, URL-label policy |
| `html` | Block mode and structural limits, tags, exclusive styles, code and quote nesting, code info, empty elements, whitespace trimming |
| `blocks` | Markdown text form, supported media formats, grouping and item limits, block ceiling, spoilers, separators, fallback for media outside a media block |

- Headings become bold where supported and plain text elsewhere. Native heading constructs are not emitted. Link titles are omitted on every target. A native link is also underlined where the target renders underline natively, such as `<u><a href="…">label</a></u>` or `__[label](URL)__`, unless it is inside or contains an authored underline; this adds no change. An entity cap drops these underlines before authored entities. Bare URLs remain text for the platform to link.
- Text layouts share `– ` bullets, continuation indentation by marker width, `› ` quote prefixes on every line including blank lines, and table separators; Markdown targets keep their native `- ` and `> `. Targets override layout only where their rendering differs. Markdown and HTML forms escape authored text for their target grammar. Discord and Slack Markdown leave provider-owned spans unescaped so the platform still links them: an HTTP(S) URL whose body has at least two characters and ends before whitespace, `<`, `>`, `(`, `)`, `[`, `]` or a backtick; an email address; a name of `@`, `#` or `$` followed by letters, digits or `_`. Underscores at the edges of an email or name, and every span inside a link label, are escaped: the platform links neither, and their raw syntax would pair with Markdown outside the span or close the label. Blocks render text through the platform's Markdown target.
- LinkedIn preserves `#` at the start or after whitespace when followed by a letter, digit or `_`, while backslash-escaping other reserved characters. Discord renders URL-shaped link labels as a bare URL when destinations match, or `label (URL)` otherwise. Before a generated ` (URL)` on any target, the label's unterminated bidi embeddings, overrides and isolates are closed with U+202C and U+2069 in nesting order, so the author's direction does not reorder the disclosed destination; a line break in the label already ends them. Markdown inline code uses one more backtick than the longest run in its value. Slack writes a datetime as `<!date^…|label>` with the label flattened to text; a label containing `^`, `|` or a line break remains text (`textual_fallback` `datetime_label_as_text`), because Markdown decides block structure line by line before reading the token, so a second label line could open a code fence or list item.
- YouTube titles use `title`, then a leading heading, then the first line. TikTok photo titles use the heading or first line and ignore `title`. An extracted heading leaves the body; an extracted first line leaves only when it fits the title whole (`first_line_became_title`), otherwise the body retains it and the title gets its fitted prefix. A supplied `title` leaves the body unchanged. VK video titles fit the whole body again; rendering changes belong to `text`, so `title` reports only truncation. Each field declares its own unit, maximum and ellipsis or hard-cut policy; fitting one never cuts another. Title extraction removes layout list markers and quote prefixes.
- Threads appends `targetUrl` after a blank line or replaces a blank body, reporting `textual_fallback` `target_url_as_text`. It uses the serialized HTTP(S) URL from the [shared URL policy](conformance.md#syntax) and never cuts it. Fitting reserves the URL plus an ellipsis and blank line for a nonblank body. Refused URLs report `omitted` `target_url_refused`; URLs that cannot fit report `omitted` `target_url_omitted`. A replaced blank body retains media changes only.
- Blank output contains only whitespace or default-ignorable Unicode characters. Required empty fields report a blocking change.

A datetime renders natively on Telegram (`tg-time` on both HTML targets, the `date` entity on `telegram-entities`), Discord (`<t:…>`, blocks included) and Slack (`<!date^…|label>`, blocks included); elsewhere its label remains text. Its `format` selects the platform's display format from the target table, and the source name is never echoed. Without `format`, Telegram writes no format, Discord uses `f` and Slack `{date_short} {time}`. Other targets ignore `format`.

| `format` | Telegram | Discord | Slack |
| --- | --- | --- | --- |
| `time` | `t` | `t` | `{time}` |
| `time-seconds` | `T` | `T` | `{time_secs}` |
| `date-short` | `d` | `d` | `{date_num}` |
| `date` | `D` | `D` | `{date}` |
| `datetime-short` | `dt` | `s` | `{date_num} {time}` |
| `datetime` | `Dt` | `f` | `{date} {time}` |
| `full` | `wDt` | `F` | `{date_long} {time}` |
| `relative` | `r` | `R` | `{ago}` |

Telegram HTML writes the letters as `<tg-time unix="…" format="…">`. A Telegram `date` entity carries them in an optional `format` key, present only when the source has a format; entities without it are unchanged.

Math renders natively only on Telegram rich HTML: a `math` node becomes `<tg-math>value</tg-math>` and a `math` code block becomes `<tg-math-block>value</tg-math-block>`, which omits the remaining info string (`omitted` `code_block_info_omitted`). Elsewhere a `math` node is its source spelling `$value$` as text, escaped for the target grammar and cut like any text, and a `math` code block stays a code block; both report `textual_fallback` `math_as_text`. Two guarantees hold on rich HTML. The tag body is escaped text: `&`, `<` and `>` become character references, so authored math never opens or closes a tag, and a formula containing `<img src="media:x">` cannot be mistaken for a media reference by a caller that rewrites such tags by pattern. A formula is atomic for truncation: it is kept whole or dropped with the rest of the cut, so a cut that would fall inside it moves back to its start, and the measured length counts the formula's source text.

### Paragraphs and gaps

A paragraph of the format renders as a paragraph of the platform and a line break inside it as a line break, and rendering adds or removes no space between blocks. Each block and list item keeps the number of line breaks the author wrote before it (`gapBefore`), so the items of a loose list stay separated by a blank line. Text targets write that number between blocks, so a blank line separates paragraphs and a single line break separates blocks written on adjacent lines, such as `Foo` followed by `- bar`. Two or more blank lines become one on every target, with an `approximated` `text` change `blank_lines_collapsed` (`when` `blank_lines`) over the gap: Mastodon and the Bluesky app collapse them before display, no other platform documents how it displays them, and rich HTML and blocks have no documented element for vertical space ([platforms](platforms.md#line-breaks-and-blank-lines)). No field starts or ends with a line break: after rendering, leading blank lines and trailing whitespace that contains a line break are removed.

Telegram rich HTML writes each paragraph, heading and block-level literal text as its own `<p>`, and writes nothing between blocks: Telegram turns `<br>` outside a paragraph into a paragraph of line breaks, while adjacent blocks receive the platform's block gap. A line break inside a paragraph is `<br>` inside its `<p>`; line breaks at a paragraph's edges and paragraphs without visible text are dropped. Two blocks written on adjacent lines, such as a heading directly followed by text, are two blocks of the format and receive the same block gap without a change. Every `<p>` counts as one block against the 500-block ceiling. The measured text counts one line feed between consecutive blocks that carry text, and a figcaption's budget reserves that line feed, because Telegram does not state whether block boundaries count toward the rich text limit.

Slack Markdown and Slack blocks write a line break inside a paragraph as a Markdown hard break, two spaces before the line feed, because Slack describes its Markdown as standard Markdown, where a single line feed joins lines; the trailing spaces are invisible where a line feed already breaks the line. Two blocks on adjacent lines stay apart there as well. A code block, quote, list or table interrupts the block before it, so a line feed precedes it; a list that interrupts a paragraph starts with a bullet or with 1 and has a non-blank first line, as CommonMark requires, because the input list already interrupted the paragraph. Before other blocks, the line break after a paragraph or heading is a hard break, and before a thematic break it becomes a blank line so `---` does not make a setext heading; after a quote, list or table followed by text it becomes a blank line so the text does not continue the previous block. Discord writes a line feed, which it renders as a line break.

On Discord and Slack blocks, a text block holds the consecutive paragraphs between media blocks and separators, separated by blank lines as in the target's Markdown, which the platform renders as paragraphs; one block per paragraph would spend the 40- and 50-block ceilings on prose. A text block never starts or ends with a line break. The `text` field writes the gap around a media block or separator once, between the neighbouring texts.

### Entities and caps

Telegram supports `pre`, `blockquote` and `date` entities when enabled. Code blocks split surrounding entities; quotes cannot nest (`unwrapped` `blockquote_cannot_nest`), and code splits a quote (`unwrapped` `code_block_cannot_be_quoted`). A date's `value` is the decimal UNIX time. Its label remains text with an `unwrapped` change inside another entity or when containing a link, code or date (`entity_cannot_contain_entity`), above 127 UTF-16 units, 31 with a format (`datetime_label_too_long`), or outside UNIX times 0 through 2³¹ − 1 (`datetime_out_of_range`). Telegram HTML and rich HTML apply the same label length and time bounds to `tg-time` with the same reasons, because Telegram converts the tag to the same date entity. The projection has no clock and does not validate time-relative provider restrictions.

`pre` stores the language and omits the remaining code info string (`omitted` `code_block_info_omitted`); its lines carry no list indentation. A code language written anywhere (Markdown fence, HTML `language-` class, `pre` value) is 1 to 32 ASCII letters, digits, `_`, `+`, `.` or `-`; any other language is omitted with the same change, so every target carries the same languages. Quote entities contain their own list markers without an enclosing quote prefix. Telegram ranges exclude trailing whitespace; quote ranges also exclude leading whitespace. Without `entitiesAllowed`, styles report `unwrapped` `entities_unavailable` and quotes retain their prefixes with `textual_fallback` `blockquote_as_text`. Repeated nesting of the same entity style produces one range.

Bluesky detects link, tag and mention facets in authored text. A mention begins with `@` at the start, after whitespace or after `(` and ends at a word boundary. Handles use ASCII letters, digits, dots and hyphens: at least two labels of 1-63 characters, no leading or trailing hyphen, a final label starting with a letter, and at most 253 characters overall. A mention entity carries the handle; the caller resolves it to a DID and drops unresolved facets. Local syntax validation avoids maintaining a TLD registry; account existence remains a caller concern.

Instagram removes excess hashtag or mention sigils while retaining their words, with one `approximated` `hashtag_limit_exceeded` or `mention_limit_exceeded` change per cap. YouTube counts the hashtags of the title and description together, as the platform ignores every hashtag of a video over its cap: the title keeps its hashtags, the description's excess sigils are removed and the change belongs to the description, with the combined count as its value. Threads counts distinct URLs in rendering order and reserves a slot when the `targetUrl` fact is set; excess explicit destinations lose their links but retain labels (`unwrapped` `link_limit_exceeded`). Remaining excess detected URLs produce `blocked` `link_limit_exceeded`. These count limits appear in measurements but never drive the text cut; values are in [field limits](measure.md#field-limits). A thread part is its own Threads post, so with `overflow: 'thread'` the link limit bounds each part like a length limit: the destination budget, the `targetUrl` slot and the `blocked` check apply per part, and the slot counts only in the first part, which carries `targetUrl`.

Discord Markdown renders a thematic break as `———` (`textual_fallback` `thematic_break_as_text`). On Discord blocks, a top-level break becomes a native separator; breaks after the cut are omitted. The Discord ceiling is 40 blocks, counting a gallery once regardless of its items. Slack allows 50 blocks and counts a slideshow as one plus its images. Exceeding a ceiling reports `blocked` `blocks_too_many`.

## Media

Parse-time media declarations validate directive IDs. Projection-time `items` selects what is available for publication:

```js
const text = 'At the harbor.\n\n::media{id="photo"}';
const media = [{ id: 'photo' }];

const projection = gsx.project(text, 'discord-blocks', {
  items: [{
    id: 'photo',
    format: 'image',
    alt: 'A lighthouse above the harbor',
    inline: true
  }]
}, media);

console.log(projection.blocks);
```

| Selection | Attaching targets | Rich HTML and block targets |
| --- | --- | --- |
| No `items` fact | Treat directive IDs as available | Treat them as missing because formats are unknown |
| Empty `items` | Drop all referenced media | Drop all referenced media |
| Some IDs missing | Report `media_missing`; attach listed IDs | Remove missing IDs and empty directives; close surrounding gaps |

A directive with missing IDs reports one `omitted` `media_missing`. Placing targets close the gap to one separator between neighboring blocks and leave no empty edge section; a group reduced to one item is placed singly without a split change. Attaching targets report `media_attached` only if at least one ID is available, even when the directive lies beyond the text cut. Unreferenced listed items remain available as attachments.

A media block takes a listed item only when it is `inline` and its format fits the block: Discord galleries take images, GIFs and videos; Slack image blocks take images and GIFs, since Slack video blocks embed third-party players rather than uploaded files. Other items are still published. Discord shows each as a `file` block in place, because Components V2 messages display only attachments referenced by a component (`approximated` `media_as_file`). Slack attaches them to the message as files in `items`, shared together with the blocks, and closes their gap like missing media (`approximated` `media_attached`). Neither fallback carries alt (`omitted` `media_alt_omitted`); Discord files keep spoilers, Slack attachments report `media_spoiler_omitted`.

Alt uses the longest grapheme prefix plus ellipsis that fits every applicable item limit (`approximated` `media_alt_truncated`). If the ellipsis cannot fit, alt is omitted (`omitted` `media_alt_omitted`). The search reads only the prefix that holds one unit more than the smallest code point, UTF-16 or byte limit allows, counting four, three and one bytes per unit, and an item placed several times is fitted once, so fitting stays linear in the alt and the placements. A target publishes alt only for the formats its `media.alt` capability lists; alt on another format is omitted with the same change and no measurement. Instagram lists only images, because its API takes alt for images alone and accepts no GIF as an image. Published alt is returned in `items` or block items.

`telegram-html-rich` places alt in figcaptions that share the body budget. When the body is cut, a preceding caption is retained whole if the text before its figure, earlier retained captions and that caption fit together; the body cut moves earlier to reserve its space. All other captions use the remaining budget in visual order, including every caption when the body fits whole.

Spoiler behavior is part of each target's configuration. Telegram HTML and X carry image, GIF and video spoilers; Discord Markdown carries them for every media format. Mastodon and Threads widen a marked item to the whole post, reporting `media_spoiler_widened` on newly hidden items. Unsupported attachment marks, including those in story placement, report `media_spoiler_omitted`. Blocks carry their per-item state. `spoilerText` affects Mastodon measurement but does not set item spoiler flags.

## Fitting and threads

Truncation uses a grapheme outline and avoids cuts inside URLs, mentions, hashtags and emails. Each field is rendered and measured against every applicable length limit. Monotone search with bounded rescanning selects a fitting prefix; [fit.rs](../../crates/gsx-core/tests/fit.rs) compares the result with exhaustive search.

One outline is shared per document and built lazily in chunks of at least 512 bytes ending at whitespace; protected entities and tokens skipped by the lower bound cannot cross that boundary. Per field, a lower bound from retained segments caps the search. When that bound exists, the first probe is the segment at the share `maximum / measured value` of the full render's outline bytes, below the bound, and the bound itself is not rendered; otherwise the estimate `upper bound × maximum / measured value` follows a render at the bound. The first probe's measured value sets the first galloping step to its proportional distance from the limit, followed by binary search. This takes O(log n) renders plus up to 64 rescan probes.

Closing an inline container or completing a partial word can reduce the measured length: link rendering, ellipsis back-off and weighted URL or mention recognition can change at that boundary. The rescan reaches the first clean cut above the search boundary and continues across the remaining window only if that cut fits. Generated ellipses, cut positions and block gaps remain node data. Truncated literals retain their provenance so escaping and entity detection remain consistent.

`overflow: 'thread'` applies to X, Bluesky, Threads, Mastodon, Telegram HTML, Max, Discord Markdown and Slack Markdown. Other targets retain truncation: Slack plain text and VK have no text limit, a story caption is one publication that cannot chain, so Telegram entities and the story placement of Telegram HTML truncate, Telegram rich HTML shares its body budget with figcaptions and structural ceilings, and a Discord or Slack blocks part would be a block list, which `parts` cannot carry. Each part takes as much of the remaining body as fits every text length limit that applies to it, and on Threads the link limit, preferring paragraph, sentence, word, then grapheme boundaries, and adds no numbering. A directive's media follows its part; unreferenced items and the target URL go to the first part.

Plain and entity targets split the rendered body greedily: each part is the longest remaining prefix that fits. Links stay intact, a `label (URL)` rendering stays with its label when the pair fits in one part, together with `targetUrl` when the pair opens the body, and boundary whitespace is trimmed; a longer label splits like other text. Links, entities and measurements belong to their part; entities crossing a boundary are split. A part whose URLs still exceed the link limit reports `blocked` `link_limit_exceeded` with the part's range. Link destinations, entities with a value, such as Bluesky link, tag and mention facets, and bare URLs that the target's own counting recognizes (`x-plain` and `threads-entities` with X's URL rules, `mastodon-plain` with Mastodon's) are never cut: a grapheme cut that would fall inside one moves back to its start, and one that alone exceeds the limit takes a part of its own, truncated as the whole body would be, so the cut avoids URLs, mentions, hashtags and emails and may leave only the ellipsis. Entities covering the cut extend over the ellipsis, while a link destination that loses text is dropped. Each such cut is a separate `text_truncated` change whose range lies inside that part's range, which covers the whole span including the dropped tail; if the span cannot fit even truncated, the body is truncated instead of threaded. That part's `boundary` names the break that follows the span, or `grapheme` when no whitespace follows it.

HTML and Markdown targets never split markup. A part is a run of whole top-level blocks, possibly beginning or ending inside a paragraph, rendered by the unchanged renderer as its own document, so no tag, fence, quote marker or list numbering continues across parts. Cut candidates are edges between top-level blocks and whitespace in a top-level paragraph's own text outside inline elements. Headings, lists, quotes, code blocks, tables and inline elements such as emphasis, links, inline code and spoilers stay whole: a cut that would fall inside an inline element moves back to the element's start, and the element opens the next part. An element that alone exceeds the limit takes a part of its own and is truncated there with an ellipsis; each such cut is a separate `text_truncated` change whose range lies inside that part's range, which covers the whole element including the dropped tail. Estimates of block and element lengths bound the candidates, and only candidate parts are rendered, so splitting stays near-linear in the document size.

The media condition of a limit is decided per part with the facts of the call: a part is with media when it carries directive media or items. When `items` is absent or empty, the first part is also with media if `media` is set, since the caller has not said which media the post carries. With listed items, a part that carries none of them takes the text limit, so a caller can predict each part's limit from the media assigned to it. On Telegram HTML such a part is a caption, so a part ends before a media directive when taking it would exceed the caption limit, and a part carries media without text when the element after it fits whole only in a part without media.

```js
const paragraph = 'A paragraph with enough words to demonstrate thread splitting.';
const text = Array(6).fill(paragraph).join('\n\n');

const projection = gsx.project(text, 'bluesky-entities', {
  overflow: 'thread'
});

for (const part of projection.parts ?? [projection]) {
  console.log(part.fields[0].text);
}
```

`parts` holds each post's fields, measurements, media, items, `range` and `boundary`. Top-level output repeats the first part. Rendering changes appear once; the split adds `text_threaded`.

A part's `range` locates its source as UTF-16 offsets in the original input, so an editor can mark each cut where the author wrote it; output offsets alone cannot be mapped back without repeating the renderer. On plain and entity targets, rendering records the source range of every text leaf it writes. Inside a leaf whose rendered text equals its source, the range is exact; elsewhere, including escapes, entities, normalization and generated text such as list markers, it widens to the enclosing leaves, so it always covers the part and may overlap a neighbor. On HTML and Markdown targets, ranges follow whole source nodes and cut positions, so they are in source order and never overlap; inside a leaf whose source differs from its text, a cut maps to the matching whitespace run, or to the leaf start when the runs differ. `boundary` names the rule that ended the part — `paragraph`, `sentence`, `word` or `grapheme` — and is null on the last part; a grapheme boundary means no whitespace break fit, which a client may present differently from a natural break. On HTML and Markdown targets an edge between blocks is `paragraph`, and a cut moved back to an inline element's start is `sentence` or `word` by the text before it.
