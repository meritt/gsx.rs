# Projection

`project(document, target, facts) -> Projection` is pure. Target rules and explicit facts determine rendering, field limits, media handling and overflow. The caller validates document diagnostics and refuses publication when a change has class `blocked`.

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

Facts affect only targets that declare the corresponding behavior. Supply media context explicitly; `items` does not infer `media` or `mediaCount`. Discover capabilities through `targets()`:

```js
const target = gsx.targets().find(({ name }) => name === 'bluesky-entities');

console.log(target.threads);
console.log(target.codes);
console.log(target.manifest);
```

## Output

| Property | Content |
| --- | --- |
| `fields` | Named text outputs, each with link ranges and entities |
| `measurements` | Counts and maxima for applicable limits; item or block indices when needed |
| `changes` | Transformations and blocking conditions, ordered by source position, uncapped |
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
- Text layouts share `– ` bullets, continuation indentation by marker width, `› ` quote prefixes on every line including blank lines, and table separators; Markdown targets keep their native `- ` and `> `. Targets override layout only where their rendering differs. Markdown and HTML forms escape authored text for their target grammar. Blocks render text through the platform's Markdown target.
- LinkedIn preserves `#` at the start or after whitespace when followed by a letter, digit or `_`, while backslash-escaping other reserved characters. Discord renders URL-shaped link labels as a bare URL when destinations match, or `label (URL)` otherwise. Markdown inline code uses one more backtick than the longest run in its value.
- YouTube titles use `title`, then a leading heading, then the first line. TikTok photo titles use the heading or first line and ignore `title`. An extracted heading leaves the body; an extracted first line leaves only when it fits the title whole (`first_line_became_title`), otherwise the body retains it and the title gets its fitted prefix. A supplied `title` leaves the body unchanged. VK video titles fit the whole body again; rendering changes belong to `text`, so `title` reports only truncation. Each field declares its own unit, maximum and ellipsis or hard-cut policy; fitting one never cuts another. Title extraction removes layout list markers and quote prefixes.
- Threads appends `targetUrl` after a blank line or replaces a blank body, reporting `textual_fallback` `target_url_as_text`. It uses the serialized HTTP(S) URL from the [shared URL policy](conformance.md#syntax) and never cuts it. Fitting reserves the URL plus an ellipsis and blank line for a nonblank body. Refused URLs report `omitted` `target_url_refused`; URLs that cannot fit report `omitted` `target_url_omitted`. A replaced blank body retains media changes only.
- Blank output contains only whitespace or default-ignorable Unicode characters. Required empty fields report a blocking change.

### Entities and caps

Telegram supports `pre`, `blockquote` and `date` entities when enabled. Code blocks split surrounding entities; quotes cannot nest (`unwrapped` `blockquote_cannot_nest`), and code splits a quote (`unwrapped` `code_block_cannot_be_quoted`). A date's `value` is the decimal UNIX time. Its label remains text with an `unwrapped` change inside another entity or when containing a link, code or date (`entity_cannot_contain_entity`), above 127 UTF-16 units (`datetime_label_too_long`), or outside UNIX times 0 through 2³¹ − 1 (`datetime_out_of_range`). The projection has no clock and does not validate time-relative provider restrictions.

`pre` stores the language and omits the remaining code info string (`omitted` `code_block_info_omitted`); its lines carry no list indentation. Quote entities contain their own list markers without an enclosing quote prefix. Telegram ranges exclude trailing whitespace; quote ranges also exclude leading whitespace. Without `entitiesAllowed`, styles report `unwrapped` `entities_unavailable` and quotes retain their prefixes with `textual_fallback` `blockquote_as_text`. Repeated nesting of the same entity style produces one range.

Bluesky detects link, tag and mention facets in authored text. A mention begins with `@` at the start, after whitespace or after `(` and ends at a word boundary. Handles use ASCII letters, digits, dots and hyphens: at least two labels of 1-63 characters, no leading or trailing hyphen, a final label starting with a letter, and at most 253 characters overall. A mention entity carries the handle; the caller resolves it to a DID and drops unresolved facets. Local syntax validation avoids maintaining a TLD registry; account existence remains a caller concern.

Instagram removes excess hashtag or mention sigils while retaining their words, with one `approximated` `hashtag_limit_exceeded` or `mention_limit_exceeded` change per cap. YouTube applies its hashtag cap to the description. Threads counts distinct URLs in rendering order and reserves a slot when the `targetUrl` fact is set; excess explicit destinations lose their links but retain labels (`unwrapped` `link_limit_exceeded`). Remaining excess detected URLs produce `blocked` `link_limit_exceeded`. These count limits appear in measurements but never drive the text cut; values are in [field limits](measure.md#field-limits).

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

Alt uses the longest grapheme prefix plus ellipsis that fits every applicable item limit (`approximated` `media_alt_truncated`). If the ellipsis cannot fit, alt is omitted (`omitted` `media_alt_omitted`). Published alt is returned in `items` or block items.

`telegram-html-rich` places alt in figcaptions that share the body budget. When the body is cut, a preceding caption is retained whole if the text before its figure, earlier retained captions and that caption fit together; the body cut moves earlier to reserve its space. All other captions use the remaining budget in visual order, including every caption when the body fits whole.

Spoiler behavior is part of each target's configuration. Telegram HTML and X carry image, GIF and video spoilers; Discord Markdown carries them for every media format. Mastodon and Threads widen a marked item to the whole post, reporting `media_spoiler_widened` on newly hidden items. Unsupported attachment marks, including those in story placement, report `media_spoiler_omitted`. Blocks carry their per-item state. `spoilerText` affects Mastodon measurement but does not set item spoiler flags.

## Fitting and threads

Truncation uses a grapheme outline and avoids cuts inside URLs, mentions, hashtags and emails. Each field is rendered and measured against every applicable length limit. Monotone search with bounded rescanning selects a fitting prefix; [fit.rs](../../crates/gsx-core/tests/fit.rs) compares the result with exhaustive search.

One outline is shared per document and built lazily in chunks of at least 512 bytes ending at whitespace; protected entities and tokens skipped by the lower bound cannot cross that boundary. Per field, a lower bound from retained segments and the estimate `upper bound × maximum / measured value` guide galloping and binary search. This takes O(log n) renders plus up to 64 rescan probes.

Closing an inline container or completing a partial word can reduce the measured length: link rendering, ellipsis back-off and weighted URL or mention recognition can change at that boundary. The rescan reaches the first clean cut above the search boundary and continues across the remaining window only if that cut fits. Generated ellipses, cut positions and block gaps remain node data. Truncated literals retain their provenance so escaping and entity detection remain consistent.

`overflow: 'thread'` applies to X, Bluesky, Threads and Mastodon. Other targets retain truncation. The rendered body is split greedily: each part takes the longest remaining prefix that fits every active text length limit, preferring paragraph, sentence, word, then grapheme boundaries. It keeps links intact, trims boundary whitespace and adds no numbering. Links, entities and measurements belong to their part; entities crossing a boundary are split. A directive's media follows its part; unreferenced items and the target URL go to the first part.

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

`parts` holds each post's fields, measurements, media and items. Top-level output repeats the first part. Rendering changes appear once; the split adds `text_threaded`.
