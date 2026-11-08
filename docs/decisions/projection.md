# Projection

## Contract

`project(document, target, facts) -> Projection` is pure. Every input that changes the output is a declared fact: media, the placement (post, reel or story), instance and account limits (Mastodon characters and description, X long posts), the channel-story caption limit, the Instagram hashtag limit, a content warning counted with the text, whether entities are allowed, an appended target URL, an explicit title. No ambient context reaches a projection.

| Field | Content |
| --- | --- |
| `fields` | named text outputs (`text`, `title`, `description`, `alt`), each with its links or entities |
| `blocks` | `blocks` targets only: ordered text and media blocks; text blocks are rendered by the platform's `markdown` target |
| `measurements` | one per limited field: unit, value, maximum; per-block limits carry the block index |
| `changes` | every change, ordered by source position, uncapped |
| `media` | selected items, covers, per-item changes |
| `items` | targets outside `blocks`, when facts list media items: one `{reference, alt}` per item in fact order, with the alt as published (fitted, or on `telegram-html-rich` cut or dropped with its figcaption by the text budget); a per-item measurement carries `item`, the index here, and no `block` |

Alt is fitted per item on every target that declares an `alt` limit for the item's kind: the longest grapheme prefix with an ellipsis that fits (`media_alt_truncated`); when not even the ellipsis fits, the alt is dropped (`omitted` `media_alt_omitted`). The Mastodon limit is the instance `descriptionLimit` fact; without it alt is measured and not fitted. On `telegram-html-rich` the alt is a figcaption inside the text budget, read in visual order with one cut point: when the body is cut, a figcaption placed before the cut is kept whole and the body is cut earlier to make room for it, provided the text before its figure, the captions kept before it and the whole caption fit the limit together; any other figcaption, before or after the cut, and all of them when the body is not cut, share what the body leaves, in order.

Media appear as references (`media:<id>`); the caller resolves URLs and builds the request.

A directive id absent from a given `items` fact is a medium the caller dropped (a gateway rejection, a size or variant filter); an empty list drops every medium. `telegram-html-rich` and `blocks` targets remove that id from its directive, and the directive itself when no id is left, with one `omitted` `media_missing` change per directive; the surrounding text closes up as if the directive were absent, with no empty section at an edge and one separator between its neighbours. A group left with one item is placed as a single item and reports no split. Other targets attach media rather than place them, so a directive reports its change wherever the text is cut: a directive with an absent id reports one `omitted` `media_missing`, and `media_attached` only when at least one of its ids is listed. Without an `items` fact, as in an analysis before media are selected, attaching targets treat every id as listed and report only `media_attached`; placing targets, which need each item's format, treat every id as absent. `telegram-html-rich` hides images and videos marked spoiler and reports a spoiler on audio, which it cannot hide, as `omitted` `media_spoiler_omitted` with the item index. `media_inline_unavailable` stays blocking: it reports a listed item that is not `inline` or whose format the form cannot place.

## Changes

| Class | Meaning |
| --- | --- |
| `approximated` | same meaning through another native construct |
| `textual_fallback` | the construct is rendered as visible text |
| `unwrapped` | the construct is removed, its content kept |
| `omitted` | the construct and its content are removed |
| `blocked` | the projection cannot be published; carries a code |

Native output is not reported. A change has `feature`, `reason`, `field` and the source range; an alt or spoiler change (`media_alt_truncated`, `media_alt_omitted`, `media_spoiler_omitted`), one per affected item, also carries the item index, and on `blocks` targets the block index, as its alt measurement does, since items of one group share the group's range. Reasons are `<feature>_<outcome>`, one per cause, with no target prefix unless the cause is target-specific. An empty projection is one `blocked` change.

- No change is silent: a datetime label, a nested quote on a `markdown` target, a code info string and the MAX spoiler each yield one.
- Highlight keeps its content: `unwrapped`, never `omitted`.
- Identical changes are reported once: in a `blocks` group every item shares the group's range, so two videos in a Slack slideshow give one `media_inline_unavailable`.
- A change belongs to the field that produced it.

## Target table

`feature → { class, reason, conditions: [{ when, class, reason }] }`, plus form configuration, measure rules per field and facts. The renderer reads only the table and the configuration; the manifest, capabilities and digest derive from it. The digest hashes the table, the form version, the measure rules and the gsx version, never a hand-bumped revision: a renderer change that leaves the table alone still changes the digest at the next release. `tests/targets.rs` checks every emitted `(feature, class, reason)` against the table.

## Forms

| Form | Configuration |
| --- | --- |
| `plain` | escape (none, LinkedIn reserved set, HTML entities, angle substitution); link `label (href)`, label omitted when equal to `href` |
| `entities` | offset unit (UTF-16, UTF-8); node → range kind; exclusive groups; code splits ranges; range cap with overflow class; empty-label policy |
| `markdown` | delimiter map; escape set; provider-owned spans; line-start neutralisers; fence and info policy; native block set; datetime template; quote nesting |
| `html` | tag map; native block set; newline form; nesting constraints; `href` schemes; empty-element removal |
| `blocks` | text segments rendered by the platform's `markdown` target; media grouping; per-item alt limit; block ceiling |

All forms share block layout: one list-marker set, continuation lines indented by the marker width, a quote prefix on every line including blank ones, one table cell separator. A target overrides a rule only where the platform renders differently. In `entities` targets a range nested in a range of the same style is dropped.

The LinkedIn reserved set is escaped with a backslash except `#` at the start or after whitespace and before a letter, digit or `_`: Little Text Format reads that as a hashtag, which the author wrote.

A heading is rendered as bold text wherever the form has bold (the `b` tag, the strong delimiter, the bold entity), and as plain text elsewhere. This holds on every target, including the platforms with native headings (Telegram rich messages, MAX, Discord, Slack `markdown`); native heading constructs are never emitted.

## Fields

A derived field (YouTube title, TikTok photo title, VK video name) declares its source (explicit fact, leading heading, first line; a first line without the list markers and quote prefixes of the layout), whether it leaves the body, its unit, maximum and truncation (ellipsis or hard). Every limited field is measured and fitted on its own; fitting one never cuts another. A first line that leaves the body (TikTok photo title) leaves it only when it fits the title whole, reported as `approximated` `first_line_became_title`; a longer one stays in the body, so nothing is lost, and the title carries its fitted prefix. The VK video name (field `title`, video only) is the whole body fitted again to its own limit; its rendering changes belong to `text`, so it reports only its truncation.

A target URL (Threads reply or quote target) is appended to the body after a blank line, or replaces a blank body, and is never cut: the body is fitted with the URL counted, and the change is `textual_fallback` `target_url_as_text`. A replaced body reports no rendering changes, only its media. The URL takes the one WHATWG parse of [conformance](conformance.md) rule 6 and is appended serialized; a URL that parse refuses is not appended and the change is `omitted` `target_url_refused`. When the URL, after `…` and a blank line unless the body is blank, exceeds a limit, nothing is appended and the change is `omitted` `target_url_omitted`. Text is blank when it holds only White_Space and Default_Ignorable_Code_Point characters (NBSP, zero-width space, Hangul filler); `text_empty` and required fields use the same test. Targets without a `target_url` rule ignore the fact.

Instagram counts hashtags (`#` after the start or whitespace, then letters, digits or `_`) and mentions (`@`, then ASCII letters, digits, `.` or `_`) in each rendered probe, before measuring. Beyond the 5th hashtag (or the `hashtagLimit` fact) or the 20th mention the sigil is removed and the word kept, with one `approximated` change per cap (`hashtag_limit_exceeded`, `mention_limit_exceeded`) carrying unit `hashtags` or `mentions`, the cap and the count. The caps are `text` limits in those units, so they are measured and never drive the cut. YouTube caps the description the same way at 60 hashtags, beyond which it ignores every hashtag of the video.

Threads publishes at most 5 unique URLs. While rendering, link destinations count in order, a repeated URL once, with one slot kept for a target URL when the fact is set; a link with a label beyond the cap keeps its label and drops its destination (`unwrapped` `link_limit_exceeded`). The rendered text is then measured in unit `links`: the unique URLs the X detector finds, bare URLs and the target URL included. A measure above 5 cannot be fixed without dropping text, so the projection is `blocked` `link_limit_exceeded`; like the Instagram caps, `links` never drives the cut.

Generated ellipsis, cut position and the gap before a block are node fields. A truncated literal stays literal: escaping and entity detection treat it as authored text.

## Fitting

- One outline per document: grapheme segments over inline flows; cuts snap back out of URLs, mentions, hashtags and emails. It is built on demand in chunks of at least 512 bytes that end after whitespace, which none of those entities, nor a token the lower bound skips, spans.
- Per field, a lower bound from the retained segments, then a monotone search: a gallop from the point the overflow ratio predicts (upper bound × maximum / measured value), a binary search, and a rescan of up to 64 units.
- A later cut measures less than an earlier one only when the earlier cut sits inside an inline container (link, datetime, styled span render differently when cut) or keeps a partial word (ellipsis back-off depends on closing punctuation; X and Mastodon weigh completed URLs and mentions differently). Above such cuts lengths only grow, so the rescan stops at the first clean cut above the boundary and scans the whole window only when that cut fits.
- Every target uses the monotone search: O(log n) renders of the document plus at most 64 rescan probes. `tests/fit.rs` checks it against exhaustive search for every target on the corpus and generated inputs.

## Threads

The `overflow` fact chooses what a body that does not fit becomes: `truncate`, the default, fits it as above; `thread` splits it into a thread of posts on a target that publishes threads (`x-plain`, `bluesky-entities`, `threads-entities`, `mastodon-plain`). Other targets ignore the fact, and a thread is never implied.

- The body is rendered whole and split greedily: each part is the longest rest of it that fits every text limit and ends at a paragraph break, else at a sentence end, else after a word, else at a grapheme boundary. A cut never falls inside a link, a link facet or a URL; the whitespace at the cut is dropped. Parts carry no numbering.
- Links, entities and measurements belong to the part that holds them; a styled range across a cut is split in two.
- A media directive goes with the part that holds its position; listed items no directive references go with the first part. A target URL goes with the first part.
- `parts` lists every part as `{fields, measurements, media, items}` in order; the top-level `fields`, `measurements`, `media` and `items` are the first part's. A body that fits yields no `parts`.
- The split is one `approximated` `text_threaded` change; rendering changes are reported once, at their source range.
