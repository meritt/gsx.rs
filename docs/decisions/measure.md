# Measurement

Measurement counts rendered fields in the units declared by each target. [`measure::measure(unit, text, &Facts) -> u32`](../../crates/gsx-core/src/measure.rs) accepts resolved limits and URL or elision spans; projection supplies those spans and applies normalization.

## Units

| Unit | Count |
| --- | --- |
| `graphemes` | Extended grapheme clusters |
| `code-points` | Unicode scalar values |
| `utf16-code-units` | UTF-16 code units |
| `utf8-bytes` | UTF-8 bytes |
| `x-weighted` | URL span: 23; longest RGI emoji match: 2; U+0000-10FF, U+2000-200D, U+2010-201F and U+2032-2037: 1 per code point; other code points: 2 |
| `mastodon-weighted` | Graphemes after replacing URL spans with `x` and removing elided spans, plus `charactersReservedPerUrl - 1` per URL; zero removes URLs |
| `threads-weighted` | One per grapheme, except an exact RGI emoji sequence or single Emoji_Presentation code point counts its UTF-8 bytes |
| `hashtags`, `mentions` | A sigil at the start or after whitespace, followed by a valid marker character |
| `links` | Distinct URL-span texts |

Hashtag characters are Unicode alphanumerics or `_`; mention characters are ASCII alphanumerics, `.` or `_`. These counts and link counts constrain output but do not select truncation points.

`measure::Facts` spans use UTF-8 byte ranges of the measured string, ordered and non-overlapping. Empty, reversed, out-of-bounds and non-boundary spans are ignored. Weighted counters skip spans that overlap an already consumed span. `measure::clusters(text)` supplies the same grapheme boundaries used for fitting. X receives NFC-normalized text. Mastodon elides the domain of mentions, reserves 23 per URL by default and counts `spoilerText` with the body.

## Limit selection

[`Target::limits`](../../crates/gsx-core/src/project/table.rs) is the source of field limits; `targets().manifest` exposes the target configuration for inspection. Each limit declares field, placements, media condition, rendering, unit and maximum. A projection reports every applicable measurement, including unlimited counts with `maximum: null`.

| Property | Selection |
| --- | --- |
| Placement | Requested placement if it declares a limit for the field; otherwise the first applicable placement among post, reel and story |
| Media | No media, any media or a specific kind; alt uses the item's kind, with GIF treated as image |
| Rendering | `Visible`: markup removed and entities decoded; `Payload`: field text as sent |
| Maximum | Fixed; resolved fact with a default; supplied fact without a default; or unlimited |

A field may have several simultaneous limits, such as Bluesky graphemes and UTF-8 bytes. Alt is measured per item independently of body placement. Mastodon alt remains unlimited until `descriptionLimit` is supplied. Target defaults, overrides and fitting are defined by the [target tables](../architecture.md#targets) and [projection contract](projection.md).

Defaults describe the selected publishing surface; they do not discover account entitlements or instance settings. `maximum: null` means the target configures no cap, not that a provider accepts unlimited input. Structural block and media ceilings are projection rules, separate from text length.

## Field limits

The following are the configured limits of this release. Semicolon-separated fields, units and maxima correspond in order; `+` means simultaneous limits. Unless stated otherwise, limits measure payload text and apply in every placement. GIF alt follows image limits.

| Target | Field and context | Unit | Maximum |
| --- | --- | --- | --- |
| `telegram-html` | `text`, post without media, visible | code points | 4096 |
| `telegram-html` | `text`, post or reel with media, visible | code points | 1024 |
| `telegram-html` | `text`, story, visible | code points | 2048 |
| `telegram-html-rich` | `text`, post, visible | code points | 32768 |
| `telegram-entities` | `text`, story | code points | `captionLimit`, default 200 |
| `max-html` | `text`, visible | code points | 4000 |
| `discord-markdown` | `text`; `alt` | code points | 2000; 1024 |
| `discord-blocks` | `text`; `alt` | code points | 4000; 1024 |
| `slack-markdown` | `text`; image `alt` | code points | 12000; 1000 |
| `slack-plain` | `text`; image `alt` | code points | unlimited; 1000 |
| `slack-blocks` | `text`; `alt` | code points | 12000; 2000 |
| `bluesky-entities` | `text` | graphemes + UTF-8 bytes | 300 + 3000 |
| `bluesky-entities` | `alt` | graphemes + UTF-8 bytes | 2000 + 20000 |
| `threads-entities` | `text`; `alt` | threads-weighted; code points | 500; 1000 |
| `threads-entities` | `text` | links | 5 |
| `x-plain` | `text`; `alt` | x-weighted; code points | `maxCharacters`, default 280; 1000 |
| `mastodon-plain` | `text`, including content warning | mastodon-weighted | `maxCharacters`, default 500 |
| `mastodon-plain` | `alt` | code points | `descriptionLimit`, otherwise unlimited |
| `linkedin-plain` | `text`; `alt` | code points | 3000; 4086 |
| `facebook-plain` | `text` | code points | 63206 |
| `instagram-plain` | `text`; `alt` | code points | 2200; 1000 |
| `instagram-plain` | `text` | hashtags; mentions | `hashtagLimit`, default 5; 20 |
| `vk-plain` | `text`; video `title`; video `alt`; image `alt` | code points | unlimited; 128; 5000; 2048 |
| `youtube-plain` | `title`; `description` | code points; UTF-8 bytes | 100; 5000 |
| `youtube-plain` | `description` | hashtags | 60 |
| `tiktok-plain` | video `text`; photo-post `text`; photo-post `title` | UTF-16 code units | 2200; 4000; 90 |
| `tiktok-plain` | `text` | mentions | unlimited |

### Limit policy

- Telegram counts text and captions in code points even though entity offsets use UTF-16. `placement: 'story'` selects the story limit explicitly; omission follows the placement fallback above.
- Bluesky applies the same application-level alt limits to images and videos. A missing protocol-level cap is not treated as permission to bypass that policy.
- X `maxCharacters` changes the body maximum without changing URL or emoji weights. Mastodon account and instance limits remain explicit facts; `spoilerText` contributes to the count without being rendered.
- Instagram `hashtagLimit` accommodates publication surfaces with different caps. TikTok reports mention counts without enforcing a cap, allowing callers to apply their own policy.
- Discord's 4000-code-point block-text budget and separate gallery-alt accounting are compatibility policies. The text budget applies across text displays and is independent of the Markdown message `content` cap.
- Facebook's configured cap is a compatibility assumption, not a verified publishing-API guarantee. VK and Slack file-comment text have no configured length cap.

## Unicode

Segmentation and emoji tables use Unicode 17.0.0. [`tools/measure/tables.mjs`](../../tools/measure/tables.mjs) generates them and the grapheme conformance fixture; [`tools/unicode/normalization.mjs`](../../tools/unicode/normalization.mjs) generates the NFC fixture. Both accept a directory containing the pinned Unicode data files; without it they fetch those files. NFC uses `icu_normalizer`.

The generated tables cover Grapheme_Cluster_Break, Extended_Pictographic, InCB, Emoji_Presentation, Default_Ignorable_Code_Point and RGI_Emoji; segmentation alone cannot supply X and Threads weights. The generator checks every emoji sequence and every default-ignorable code point against Node's data. [Grapheme conformance](../../crates/gsx-core/tests/measure/GraphemeBreakTest.txt), [NFC conformance](../../crates/gsx-core/tests/project/NormalizationTest.txt), [X URL cases](../../crates/gsx-core/tests/project/twitter-urls.jsonl) and [Mastodon entity cases](../../crates/gsx-core/tests/project/mastodon-entities.jsonl) pin these boundaries. [Measurement properties](../../crates/gsx-core/tests/measure_properties.rs) cover invalid spans, cluster partitioning, additivity across hard boundaries and reserved URL weights.
