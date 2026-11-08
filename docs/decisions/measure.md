# Measure

How each target counts text against its limits. Code: `crates/gsx-core/src/measure`.

## API

`measure(unit, text, &Facts) -> u32` is the one counting entry point; `clusters(text)` yields grapheme clusters for the fitting outline; each target's `limits` are its per-field table. `Facts` carries URL spans, elided spans and resolved provider facts. Spans are byte ranges of the measured string, in order; empty, reversed, out-of-bounds, non-boundary and overlapping spans are ignored. `measure` detects nothing: URL and mention ranges come from the projection that rendered or detected them. X text is NFC-normalized by the projection before it is measured.

## Units

| Unit | Count |
| --- | --- |
| `graphemes` | extended grapheme clusters, UAX #29 |
| `code-points` | Unicode scalar values |
| `utf16-code-units` | UTF-16 code units |
| `utf8-bytes` | UTF-8 bytes |
| `x-weighted` | twitter-text v3 config: URL span 23; longest RGI_Emoji match 2; U+0000–10FF, U+2000–200D, U+2010–201F, U+2032–2037 1; any other code point 2 |
| `mastodon-weighted` | Mastodon `StatusLengthValidator`: graphemes of the text with each URL span replaced by `x` and each elided span removed, plus `characters_reserved_per_url − 1` per URL; 0 reserved removes the URL |
| `threads-weighted` | graphemes; a cluster that is exactly an RGI_Emoji sequence or one Emoji_Presentation code point counts its UTF-8 bytes |
| `hashtags`, `mentions` | markers counted as in [projection.md](projection.md#fields); caps, never cut points |
| `links` | unique URL spans, compared as text; a cap, never a cut point |

A Mastodon mention counts as `@local`: the projection passes `@domain` as an elided span.

## Rule table

Every `Target` declares its `limits` beside its rules and form in `crates/gsx-core/src/targets/`; the manifest lists them per field. Fields follow [projection.md](projection.md): `text`, `title`, `description`, plus `alt` per media item. A `Limit` is field, placements, media condition (any, without media, with media of any kind, only images or only videos; for `alt` the item's kind, a GIF measured as an image), rendering, unit and maximum; `Target::measure` runs exactly the limits that apply. A projection measures a field in one placement: the `placement` fact when the target declares a limit for the field there, otherwise the first of post, reel, story that declares one, so a story caption on `telegram-html` is held to 2048 only with `placement: story`. `alt` limits apply in every placement. `Rendering::Visible` measures text with markup removed and entities decoded, as the provider counts it; `Payload` measures the field value as sent. `Maximum::Resolved` names a provider fact (Mastodon instance, Telegram channel-story capabilities) and its default; `Maximum::Given` names a fact with no default, unlimited when the fact is absent (Mastodon `descriptionLimit`).

| Target | Field (context) | Unit | Maximum |
| --- | --- | --- | --- |
| `telegram-html` | text (post without media), visible | code points | 4096 |
| | text (post or reel with media), visible | code points | 1024 |
| | text (story), visible | code points | 2048 |
| `telegram-html-rich` | text (post), visible | code points | 32768 |
| `telegram-entities` | text (story) | code points | caption limit, 200 |
| `max-html` | text, visible | code points | 4000 |
| `discord-markdown` | text; alt | code points | 2000; 1024 |
| `discord-blocks` | text; alt | code points | 4000; 1024 |
| `slack-markdown` | text; alt (image) | code points | 12000; 1000 |
| `slack-plain` | text (file comment); alt (image) | code points | none; 1000 |
| `slack-blocks` | text; alt | code points | 12000; 2000 |
| `bluesky-entities` | text | graphemes and utf8 | 300 and 3000 |
| | alt | graphemes and utf8 | 2000 and 20000 |
| `threads-entities` | text; alt | threads-weighted; code points | 500; 1000 |
| | text | links | 5 |
| `x-plain` | text; alt | x-weighted; code points | max characters, 280; 1000 |
| `mastodon-plain` | text, with the content warning | mastodon-weighted | max characters, 500 |
| | alt | code points | description limit, none |
| `linkedin-plain` | text; alt | code points | 3000; 4086 |
| `facebook-plain` | text | code points | 63206 |
| `instagram-plain` | text; alt | code points | 2200; 1000 |
| | text | hashtags; mentions | hashtag limit, 5; 20 |
| `vk-plain` | text; title (video name); alt (video); alt (image) | code points | none; 128; 5000; 2048 |
| `youtube-plain` | title; description | code points; utf8 | 100; 5000 |
| | description | hashtags | 60 |
| `tiktok-plain` | text (video); text (photo post); title (photo post) | utf16 | 2200; 4000; 90 |

Not measured here: Telegram rich structure limits (blocks, nesting, media, table columns), hashtag, mention and media counts, NFC normalization before X counting, URL and mention detection.

## Unicode

Unicode 17.0.0, the version of ICU 78 in Node.js 26.13.0. `tools/measure/tables.mjs` generates `crates/gsx-core/src/measure/tables.rs` from the UCD (Grapheme_Cluster_Break, Extended_Pictographic, InCB, Emoji_Presentation, Default_Ignorable_Code_Point, RGI_Emoji) and checks every sequence against Node's `\p{RGI_Emoji}` and every code point against `\p{Default_Ignorable_Code_Point}`; it also writes the UAX #29 test file. Run `node tools/measure/tables.mjs [ucd-directory]`; without a directory it fetches the pinned version. NFC before X counting comes from `icu_normalizer` 2.2, on the same Unicode version; `node tools/unicode/normalization.mjs [ucd-directory]` writes its conformance cases to `crates/gsx-core/tests/project/NormalizationTest.txt`.

No dependency: neither `unicode-segmentation` nor ICU4X's segmenter carries RGI_Emoji, which X and Threads need, so the tables are generated in-tree.

## Choices

| Case | Rule |
| --- | --- |
| `bluesky-entities` | graphemes and UTF-8 bytes both reported, each against its own maximum |
| `bluesky-entities` alt | the lexicon sets no maximum for image or video alt; 2000 graphemes and 20000 bytes are the Bluesky app's `MAX_ALT_TEXT` for both |
| `x-plain`, `threads-entities` alt | 1000 characters, the X media metadata `alt_text` schema and the Threads publishing reference, counted in code points |
| `mastodon-plain`, instance unresolved | vanilla Mastodon: 500, 23 per URL; alt is measured without a maximum and not fitted until the `descriptionLimit` fact gives the instance's limit |
| Telegram text and caption lengths | code points, though entity offsets are UTF-16. The Bot API gives "characters after entities parsing"; the MTProto config documents `caption_length_limit_*` and `story_caption_length_limit_*` as "UTF-8 length", which TDLib's `utf8_length` counts as characters, the unit it checks bot texts and captions and cuts reply quotes in |
| `telegram-entities`, capabilities resolved | code points against the resolved maximum |
| `facebook-plain` | 63 206 code points, the post limit secondary sources give; Meta documents none |
| `x-plain`, long posts | 280 weighted; the `maxCharacters` fact lifts it for accounts allowed longer posts (25 000 with X Premium) |
| `mastodon-plain`, content warning | the `spoilerText` fact is not rendered; `StatusLengthValidator` counts it joined before the text, so it is measured with the text |
| `instagram-plain` hashtags | 5: the app's cap since December 2025; the Graph API reference still says 30, so the `hashtagLimit` fact overrides it |
| `discord-blocks` text | 4000 for all text displays together: the Components V2 total that D++, JDA, Discord.NET and discord-userdoccers give; Discord's reference documents no text limit for these messages, and the 2000 of `content` does not apply to them. Whether gallery alt text counts toward the total is not documented; it is measured on its own |

VK and `slack-plain` text stay unlimited until a primary source documents a limit.

## Verification

- `GraphemeBreakTest.txt` 17.0.0: every line.
- Corpus: every measurement in `testdata/corpus/targets/*.jsonl` replays exactly. X URL ranges and Mastodon URL and mention ranges are pinned by `crates/gsx-core/tests/project/twitter-urls.jsonl` and `mastodon-entities.jsonl`.
- Properties: arbitrary spans never panic; clusters partition the text and each is one cluster; every unit adds across a control character; a URL span weighs its reserved length; every cluster weighs at least 1 in `x-weighted` and `threads-weighted`.
