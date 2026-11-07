# Projection

## Contract

`project(document, target, facts) -> Projection` is pure. Every input that changes the output is a declared fact: media, instance limits, the channel-story caption limit, whether entities are allowed, an appended target URL, an explicit title. No ambient context reaches a projection.

| Field | Content |
| --- | --- |
| `fields` | named text outputs (`text`, `title`, `description`, `alt`), each with its links or entities |
| `blocks` | `blocks` targets only: ordered text and media blocks; text blocks are rendered by the platform's `markdown` target |
| `measurements` | one per limited field: unit, value, maximum; per-block limits carry the block index |
| `changes` | every change, ordered by source position, uncapped |
| `media` | selected items, covers, per-item changes |
| `items` | targets outside `blocks`, when facts list media items: one `{reference, alt}` per item in fact order, with the alt as published (fitted, or on `telegram-html-rich` cut or dropped with its figcaption by the text budget); a per-item measurement carries `item`, the index here, and no `block` |

Alt is fitted per item on every target that declares an `alt` limit for the item's kind: the longest grapheme prefix with an ellipsis that fits (`media_alt_truncated`); when not even the ellipsis fits, the alt is dropped (`omitted` `media_alt_omitted`). The Mastodon limit is the instance `descriptionLimit` fact; without it alt is measured and not fitted. On `telegram-html-rich` the alt is a figcaption inside the text budget, read in visual order with one cut point: when the body is cut, a figcaption placed before the cut is kept whole and the body is cut earlier to make room for it; figcaptions after the cut, and all of them when the body is not cut, share what the body leaves, in order.

Media appear as references (`media:<id>`); the caller resolves URLs and builds the request.

A directive id absent from the `items` fact is a medium the caller dropped (a gateway rejection, a size or variant filter, an offline analysis). `telegram-html-rich` and `blocks` targets remove that id from its directive, and the directive itself when no id is left, with one `omitted` `media_missing` change per directive; the surrounding text closes up as if the directive were absent, with no empty section at an edge and one separator between its neighbours. A group left with one item is placed as a single item and reports no split. Other targets attach media rather than place them: a directive with an absent id reports one `omitted` `media_missing`, and `media_attached` only when at least one of its ids is listed. Without an `items` fact every id is absent. `media_inline_unavailable` stays blocking: it reports a listed item that is not `inline` or whose format the form cannot place.

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

`feature → { class, reason, conditions: [{ when, class, reason }] }`, plus form configuration, measure rules per field and facts. The renderer reads only the table and the configuration; the manifest, capabilities and digest derive from it. The digest hashes the table, the form version and the measure rules, never a hand-bumped revision. `tests/targets.rs` checks every emitted `(feature, class, reason)` against the table.

## Forms

| Form | Configuration |
| --- | --- |
| `plain` | escape (none, LinkedIn reserved set, HTML entities, angle substitution); link `label (href)`, label omitted when equal to `href` |
| `entities` | offset unit (UTF-16, UTF-8); node → range kind; exclusive groups; code splits ranges; range cap with overflow class; empty-label policy |
| `markdown` | delimiter map; escape set; provider-owned spans; line-start neutralisers; fence and info policy; native block set; datetime template; quote nesting |
| `html` | tag map; native block set; newline form; nesting constraints; `href` schemes; empty-element removal |
| `blocks` | text segments rendered by the platform's `markdown` target; media grouping; per-item alt limit; block ceiling |

All forms share block layout: one list-marker set, continuation lines indented by the marker width, a quote prefix on every line including blank ones, one table cell separator. A target overrides a rule only where the platform renders differently. In `entities` targets a range nested in a range of the same style is dropped.

## Fields

A derived field (YouTube title, TikTok photo title, VK video name) declares its source (explicit fact, leading heading, first line), whether it leaves the body, its unit, maximum and truncation (ellipsis or hard). Every limited field is measured and fitted on its own; fitting one never cuts another. The VK video name (field `title`, video only) is the whole body fitted again to its own limit; its rendering changes belong to `text`, so it reports only its truncation.

A target URL (Threads reply or quote target) is appended to the body after a blank line, or replaces a body whose visible text is empty or only whitespace (NBSP included, the test `text_empty` uses), and is never cut: the body is fitted with the URL counted, and the change is `textual_fallback` `target_url_as_text`. When `…`, a blank line and the URL alone exceed a limit, nothing is appended and the change is `omitted` `target_url_omitted`. Targets without a `target_url` rule ignore the fact.

Instagram counts hashtags (`#` after the start or whitespace, then letters, digits or `_`) and mentions (`@`, then ASCII letters, digits, `.` or `_`) in each rendered probe, before measuring. Beyond the 30th hashtag or the 20th mention the sigil is removed and the word kept, with one `approximated` change per cap (`hashtag_limit_exceeded`, `mention_limit_exceeded`) carrying unit `hashtags` or `mentions`, the cap and the count. The caps are `text` limits in those units, so they are measured and never drive the cut.

Generated ellipsis, cut position and the gap before a block are node fields. A truncated literal stays literal: escaping and entity detection treat it as authored text.

## Fitting

- One outline per document: grapheme segments over inline flows; cuts snap back out of URLs, mentions, hashtags and emails.
- Per field, a lower bound from the retained segments, then a monotone search: a gallop from the point the overflow ratio predicts (upper bound × maximum / measured value), a binary search, and a rescan of up to 64 units.
- A later cut measures less than an earlier one only when the earlier cut sits inside an inline container (link, datetime, styled span render differently when cut) or keeps a partial word (ellipsis back-off depends on closing punctuation; X and Mastodon weigh completed URLs and mentions differently). Above such cuts lengths only grow, so the rescan stops at the first clean cut above the boundary and scans the whole window only when that cut fits.
- Every target uses the monotone search: O(log n) renders of the document plus at most 64 rescan probes. `tests/fit.rs` checks it against exhaustive search for every target on the corpus and generated inputs.
