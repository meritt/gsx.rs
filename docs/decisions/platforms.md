# Platforms

Each target corresponds to a platform API at a stated version or documentation date. This record names the primary source for each family of facts behind the [field limits](measure.md#field-limits), the [target tables](../architecture.md#targets) and the [projection rules](projection.md), and marks the values that no primary source states. The values themselves live in those tables; a number appears here only to identify an assumption or a documented value that differs from the default.

Primary sources are the platform's own reference documentation, official repositories and help centres. A fact is confirmed when a primary page states it. It is an assumption when no primary page states it, when it appears only in legacy documentation, or when the page that would state it cannot be read; the record names that page. Changing a target's limits or markup requires rechecking its platform and updating its entry.

| Platform | Targets | API and version or documentation date | Checked |
| --- | --- | --- | --- |
| [Telegram](#telegram) | `telegram-html`, `telegram-html-rich`, `telegram-entities` | Bot API 10.4, 2026-11-09; MTProto client configuration | 2026-11-10 |
| [MAX](#max) | `max-html` | Bot API, unversioned; changelog of November 2026 | 2026-11-10 |
| [Discord](#discord) | `discord-markdown`, `discord-blocks` | API v10 | 2026-11-10 |
| [Slack](#slack) | `slack-markdown`, `slack-blocks`, `slack-plain` | Web API, unversioned | 2026-11-10 |
| [X](#x) | `x-plain` | API v2; twitter-text configuration v3 | 2026-11-10 |
| [Threads](#threads) | `threads-entities` | Threads API v1.0 | 2026-11-10 |
| [Bluesky](#bluesky) | `bluesky-entities` | `app.bsky` lexicons, `main` of `bluesky-social/bsky` | 2026-11-10 |
| [Mastodon](#mastodon) | `mastodon-plain` | Mastodon 4.7.5; Instance entity page of 2026-10-19 | 2026-11-10 |
| [LinkedIn](#linkedin) | `linkedin-plain` | Posts API, version 202610 | 2026-11-10 |
| [Facebook](#facebook) | `facebook-plain` | Graph API v27.0 | 2026-11-10 |
| [Instagram](#instagram) | `instagram-plain` | Graph API v27.0 | 2026-11-10 |
| [VK](#vk) | `vk-plain` | VK API JSON Schema 5.199 | 2026-11-10 |
| [YouTube](#youtube) | `youtube-plain` | Data API v3, reference of 2026-11-10 | 2026-11-10 |
| [TikTok](#tiktok) | `tiktok-plain` | Content Posting API v2, reference of 2026-10-27 | 2026-11-10 |

## Line breaks and blank lines

Every target writes one blank line between paragraphs and collapses two or more authored blank lines into one with a reported change. Where a platform documents nothing, collapsing is the conservative choice: it never adds space the platform might render as empty lines or empty blocks.

| Platforms | Blank lines beyond one | Source | Status |
| --- | --- | --- | --- |
| Mastodon | The server's `simple_format` turns `\n` into `<br />` and two or more line breaks into one paragraph break | [`text_formatter.rb`](https://github.com/mastodon/mastodon/blob/main/app/lib/text_formatter.rb), [Rails `simple_format`](https://api.rubyonrails.org/classes/ActionView/Helpers/TextHelper.html#method-i-simple_format) | Confirmed 2026-11-11 |
| Bluesky | The official app strips leading blank lines and trailing whitespace and replaces three or more line breaks with two before posting; the record keeps what a client sends | [social-app `api/index.ts`](https://github.com/bluesky-social/social-app/blob/main/src/lib/api/index.ts), [sdk `sanitization.ts`](https://github.com/bluesky-social/bsky/blob/main/packages/sdk/src/rich-text/sanitization.ts) | Confirmed 2026-11-11 for the official app; the target matches it |
| Telegram rich HTML | No element for vertical space; empty `<p>` and `<br>` between blocks are undocumented | [Bot API rich messages](https://core.telegram.org/bots/api#rich-html-style) | Assumption: the page is silent |
| Telegram HTML and entities, MAX, Discord, Slack, X, Threads, LinkedIn, Facebook, Instagram, VK, YouTube, TikTok | Not stated | Each platform's formatting or post reference listed in its section | Assumption: no page states how consecutive empty lines display; a Discord separator without a divider is not used for spacing because its height is undocumented and it spends a block |

## Telegram

`telegram-html` and `telegram-html-rich` follow the Bot API; `telegram-entities` follows MTProto story captions. Rich messages date from Bot API 10.1 (2026-06-11) to 10.3 (2026-08-24); 10.4 adds widgets and the detected `ton_address` and `bank_card_number` entities, which do not change rendering ([changelog](https://core.telegram.org/bots/api-changelog)).

| Facts | Source | Status |
| --- | --- | --- |
| Message text, media caption and story caption limits | [Bot API](https://core.telegram.org/bots/api) | Confirmed |
| HTML tags, entity nesting, UTF-16 entity offsets | [Bot API formatting options](https://core.telegram.org/bots/api#formatting-options) | Confirmed |
| Rich-message text, block, nesting, media and column limits; rich HTML tags | [Bot API rich messages](https://core.telegram.org/bots/api) | Confirmed |
| MTProto story caption limit and its premium value, story entity availability | [Client configuration](https://core.telegram.org/api/config) | Confirmed; the page measures captions in "UTF-8 length", and counting code points as for Bot API characters is an assumption |
| Date entity span, `pre` and date nesting | [Message entities](https://core.telegram.org/api/entities) | Confirmed |
| Date format letters: the `format` attribute of `tg-time`, the grammar `r\|w?[dD]?[tT]?`, the `date_time` entity (Bot API 9.5) and the MTProto `messageEntityFormattedDate` flags | [Bot API formatting options](https://core.telegram.org/bots/api#formatting-options), [Message entities](https://core.telegram.org/api/entities) | Confirmed 2026-11-11 |
| Date label of at most 31 UTF-16 units when a format is set | [Message entities](https://core.telegram.org/api/entities) | Confirmed for MTProto 2026-11-11; applying it to Bot API `tg-time` is an assumption, as for the 127-unit span: the Bot API page states no span limit |
| Math tags `<tg-math>` and `<tg-math-block>` in rich HTML only, their body as raw LaTeX, `RichTextMathematicalExpression` and `RichBlockMathematicalExpression` (Bot API 10.1), formula source counted in the 32768-character text limit; no math tag in message HTML and no math entity | [Bot API rich messages](https://core.telegram.org/bots/api#rich-html-style), [changelog](https://core.telegram.org/bots/api-changelog) | Confirmed 2026-11-11 |
| Escaping `&`, `<` and `>` inside math tags as character references that the body decodes back to LaTeX; math inside formatting tags, list items, quotes and table cells; a math block counted as one block | [Bot API rich messages](https://core.telegram.org/bots/api#rich-html-style) | Assumption: the page lists the rich entities `&lt;`, `&gt;`, `&amp;` and states no escaping, nesting, placement or per-formula rule for math |
| Entity cap of 100 on `telegram-entities` | None | Assumption: the Bot API, message entities and client configuration pages state no number |
| Rich HTML: `<br>` outside a block becomes a paragraph of line breaks; adjacent blocks receive the block gap; `<br>` inside `<p>` is a line break | Owner's published rich message of 2026-11-11 (`rich_message.blocks` of the response); [Bot API rich messages](https://core.telegram.org/bots/api#rich-html-style) | Observed in that response; the page is silent and shows `<br>` only inside block quotations |
| Rich HTML block types, `<hr/>` as a divider block, the 500-block count including nested blocks and list items | [Bot API rich messages](https://core.telegram.org/bots/api#richblock), [rich message limits](https://core.telegram.org/bots/api#rich-message-limits) | Confirmed 2026-11-11 |
| Rich text limit over "the rich message text"; whether block boundaries count | [Rich message limits](https://core.telegram.org/bots/api#rich-message-limits) | Assumption: the page is silent on boundaries, so measurement counts one line feed per boundary, an upper bound |
| Media alt: none on `telegram-html` and `telegram-entities`; images, GIFs, videos and audio on `telegram-html-rich` | [sendPhoto](https://core.telegram.org/bots/api#sendphoto), [sendVideo](https://core.telegram.org/bots/api#sendvideo), [sendAnimation](https://core.telegram.org/bots/api#sendanimation), [sendAudio](https://core.telegram.org/bots/api#sendaudio), [sendMediaGroup](https://core.telegram.org/bots/api#sendmediagroup), [stories.sendStory](https://core.telegram.org/method/stories.sendStory), [Bot API rich messages](https://core.telegram.org/bots/api#rich-html-style) | Confirmed 2026-11-13: the Bot API media methods and MTProto `stories.sendStory` have no alt parameter, so `telegram-html` and `telegram-entities` omit alt on every format; rich messages carry alt as a `<figcaption>` |

## MAX

The [API changelog](https://dev.max.ru/docs-api/changelog-api/) dates the documentation.

| Facts | Source | Status |
| --- | --- | --- |
| Message text limit, `format` values | [POST /messages](https://dev.max.ru/docs-api/methods/POST/messages) | Confirmed |
| HTML tags | [Text formatting](https://dev.max.ru/docs-api/use-cases/sending-messages/text-formatting/) | Confirmed; the tag set has no spoiler |
| Media alt: none | [POST /messages](https://dev.max.ru/docs-api/methods/POST/messages), [POST /uploads](https://dev.max.ru/docs-api/methods/POST/uploads) | Confirmed 2026-11-13: neither method has an alt parameter, so `max-html` omits alt on every format |

## Discord

| Facts | Source | Status |
| --- | --- | --- |
| Message `content` limit, attachment description | [Message resource](https://docs.discord.com/developers/resources/message) | Confirmed |
| Components V2 total, media gallery items, gallery alt and spoiler | [Component reference](https://docs.discord.com/developers/components/reference) | Confirmed |
| Combined text-display budget on `discord-blocks` | [Component reference](https://docs.discord.com/developers/components/reference), [Using message components](https://docs.discord.com/developers/components/using-message-components) | Assumption: no official page states a text budget |
| Ten attachments per message on `discord-blocks` | [Message resource](https://docs.discord.com/developers/resources/message), [uploading files](https://docs.discord.com/developers/reference#uploading-files) | Assumption: neither page states a count |
| Markdown delimiters and spoilers | [Markdown Text 101](https://support.discord.com/hc/en-us/articles/210298617-Markdown-Text-101-Chat-Formatting-Bold-Italic-Underline) | Confirmed |
| API version, timestamp syntax and styles | [API reference](https://docs.discord.com/developers/reference) | Confirmed; styles `t`, `T`, `d`, `D`, `s`, `f` (default), `F`, `R` rechecked 2026-11-11 |
| Text Display content renders like message content | [Component reference](https://docs.discord.com/developers/components/reference#text-display) | Confirmed 2026-11-11; how content treats single line breaks, blank lines and edge whitespace is not stated there or in [Markdown Text 101](https://support.discord.com/hc/en-us/articles/210298617) |

## Slack

| Facts | Source | Status |
| --- | --- | --- |
| `markdown_text` limit on `slack-markdown` | [chat.postMessage](https://docs.slack.dev/reference/methods/chat.postMessage) | Confirmed |
| Markdown block limit and syntax | [Markdown block](https://docs.slack.dev/reference/block-kit/blocks/markdown-block) | Confirmed |
| A Markdown block may become several blocks after translation | [Markdown block](https://docs.slack.dev/reference/block-kit/blocks/markdown-block#usage-info) | Confirmed 2026-11-11; the page calls the text "standard markdown" and names no dialect |
| A single line feed in a Markdown block or `markdown_text` joins lines as in CommonMark, so a line break is written as a hard break | [Markdown block](https://docs.slack.dev/reference/block-kit/blocks/markdown-block) | Assumption: the page says "standard markdown" and states no line-break rule; a hard break renders the same where a line feed already breaks the line |
| Blocks per message | [Blocks](https://docs.slack.dev/reference/block-kit/blocks) | Confirmed |
| Image block alt | [Image block](https://docs.slack.dev/reference/block-kit/blocks/image-block) | Confirmed |
| Date syntax, escaping | [Formatting message text](https://docs.slack.dev/messaging/formatting-message-text) | Confirmed |
| Date tokens `{time}`, `{time_secs}`, `{date_num}`, `{date}`, `{date_short}`, `{date_long}`, `{ago}` combined with plain text in one token string | [Formatting message text](https://docs.slack.dev/messaging/formatting-message-text) | Confirmed 2026-11-11; the page shows `{date} at {time}`, and `{date_long} {time}` follows its plain-text rule without an example |
| File comment on `slack-plain` | [files.completeUploadExternal](https://docs.slack.dev/reference/methods/files.completeUploadExternal) | Confirmed: `initial_comment` states no maximum |
| Uploaded image alt of 1000 on `slack-markdown` and `slack-plain` | [files.getUploadURLExternal](https://docs.slack.dev/reference/methods/files.getUploadURLExternal) | Confirmed 2026-11-13: `alt_txt` describes an image for screen readers, and the error `alt_txt_too_large` states the limit of 1000 |
| Uploaded alt formats on `slack-markdown` and `slack-plain`: images and GIFs | [files.getUploadURLExternal](https://docs.slack.dev/reference/methods/files.getUploadURLExternal) | Confirmed 2026-11-13 for images. Assumption: a GIF takes alt as an image; videos and audio take none |
| Slideshow group of ten images on `slack-blocks` | None | Assumption: the Block Kit and file upload pages state no group bound |

## X

| Facts | Source | Status |
| --- | --- | --- |
| Weighted length, URL weight, emoji weight, NFC | [Counting characters](https://docs.x.com/fundamentals/counting-characters) | Confirmed |
| Code point weight ranges | [twitter-text configuration v3](https://raw.githubusercontent.com/twitter/twitter-text/master/config/v3.json) | Confirmed |
| Sensitive media warning carried as the item spoiler on images, GIFs and videos | [Create media metadata](https://docs.x.com/x-api/media/create-media-metadata), XDK [`latest-openapi.json`](https://github.com/xdevplatform/xdk/blob/main/latest-openapi.json) | Confirmed 2026-11-13: the schema `SensitiveMediaWarning` under `MetadataCreateRequest.metadata.sensitive_media_warning` takes the booleans `adult_content`, `graphic_violence` and `other`; the projection reports the spoiler state and leaves the choice of category to the caller. Assumption: the warning applies to GIFs and videos as well as images |
| Alt text of 1000; alt formats images, GIFs and videos, no audio | [Create media metadata](https://docs.x.com/x-api/media/create-media-metadata), [Media introduction](https://docs.x.com/x-api/media/introduction) | Confirmed 2026-11-13 for images: `alt_text.text` takes at most 1000 characters, and media are photos, videos or animated GIFs. Assumption: GIF and video alt, since the XDK documents `alt_text` only on photos |

## Threads

| Facts | Source | Status |
| --- | --- | --- |
| Text limit, emoji weight, link count | [Posts](https://developers.facebook.com/documentation/threads/posts) | Confirmed; the page counts emoji as UTF-8 bytes, and the emoji classification of `threads-weighted` is this library's reading |
| Text spoiler count, media spoilers | [Spoilers](https://developers.facebook.com/documentation/threads/create-posts/spoilers) | Confirmed |
| UTF-16 spoiler offsets | [Spoilers](https://developers.facebook.com/documentation/threads/create-posts/spoilers) | Assumption: the page states no offset unit |
| Alt text, reply and quote parameters | [Publishing reference](https://developers.facebook.com/documentation/threads/reference/publishing) | Confirmed |
| Alt formats: images and videos | [Publishing reference](https://developers.facebook.com/documentation/threads/reference/publishing) | Confirmed 2026-11-13: `alt_text` applies to an image or video, up to 1000 characters; a GIF attached through `gif_attachment` carries no alt |

## Bluesky

The `app.bsky` lexicons are canonical in `bluesky-social/bsky` ([lexicons directory](https://github.com/bluesky-social/atproto/blob/main/lexicons/README.md)).

| Facts | Source | Status |
| --- | --- | --- |
| Post text in graphemes and UTF-8 bytes | [app.bsky.feed.post](https://github.com/bluesky-social/bsky/blob/main/lexicons/app/bsky/feed/post.json) | Confirmed |
| Mention, link and tag facets, UTF-8 byte offsets, tag length | [app.bsky.richtext.facet](https://github.com/bluesky-social/bsky/blob/main/lexicons/app/bsky/richtext/facet.json) | Confirmed |
| Image and video alt in graphemes | [social-app constants](https://github.com/bluesky-social/social-app/blob/main/src/lib/constants.ts) | Confirmed as the official app's limit; the [images](https://github.com/bluesky-social/bsky/blob/main/lexicons/app/bsky/embed/images.json) and [video](https://github.com/bluesky-social/bsky/blob/main/lexicons/app/bsky/embed/video.json) lexicons state no alt maximum |
| Alt of 20000 UTF-8 bytes | None | Assumption: neither the lexicons nor the app constants state a byte limit |
| Alt formats: images, GIFs and videos, no audio | [images](https://github.com/bluesky-social/bsky/blob/main/lexicons/app/bsky/embed/images.json) and [video](https://github.com/bluesky-social/bsky/blob/main/lexicons/app/bsky/embed/video.json) lexicons | Confirmed 2026-11-13: both embeds carry `alt`, a GIF uploads as an image or a video, and no embed carries audio |

## Mastodon

The current release is [4.7.5](https://github.com/mastodon/mastodon/releases/latest).

| Facts | Source | Status |
| --- | --- | --- |
| Status length counting, default maximum and URL weight | [StatusLengthValidator](https://github.com/mastodon/mastodon/blob/main/app/validators/status_length_validator.rb) | Confirmed |
| `max_characters`, `characters_reserved_per_url`, `description_limit` | [Instance entity](https://docs.joinmastodon.org/entities/Instance/) | Confirmed; the documented example reports a `description_limit` of 1500, and the target configures no alt maximum until `descriptionLimit` is supplied |
| Alt formats: images, GIFs, videos and audio | [MediaAttachment](https://docs.joinmastodon.org/entities/MediaAttachment/) | Confirmed 2026-11-13: the entity, shared by image, GIF, video and audio attachments, carries `description` as alternate text |

## LinkedIn

| Facts | Source | Status |
| --- | --- | --- |
| Commentary format, reserved characters, hashtags | [little Text Format](https://learn.microsoft.com/en-us/linkedin/marketing/community-management/shares/little-text-format) | Confirmed |
| Commentary limit of 3000 | [Posts API](https://learn.microsoft.com/en-us/linkedin/marketing/community-management/shares/posts-api), [Post schema](https://learn.microsoft.com/en-us/linkedin/marketing/community-management/shares/post-api-schema), [UGC Post API](https://learn.microsoft.com/en-us/linkedin/compliance/integrations/shares/ugc-post-api) | Assumption: only the legacy UGC Post API states it |
| Image alt | [Images API](https://learn.microsoft.com/en-us/linkedin/marketing/community-management/shares/images-api) | Confirmed |
| Alt of 4086 on other media | [Post schema](https://learn.microsoft.com/en-us/linkedin/marketing/community-management/shares/post-api-schema), [Videos API](https://learn.microsoft.com/en-us/linkedin/marketing/community-management/shares/videos-api) | Assumption: the Post schema `altText` states no maximum and the Videos API defines no alt |
| Alt formats: images, GIFs and videos, no audio | [Images API](https://learn.microsoft.com/en-us/linkedin/marketing/community-management/shares/images-api), [Post schema](https://learn.microsoft.com/en-us/linkedin/marketing/community-management/shares/post-api-schema) | Confirmed 2026-11-13 for images. GIF alt is inferred from images; video alt is an assumption. Marketing API version 202511 sunsets on 2026-11-16 |

## Facebook

| Facts | Source | Status |
| --- | --- | --- |
| Page post `message` limit of 63206 | [Page feed](https://developers.facebook.com/docs/graph-api/reference/v27.0/page/feed) | Assumption: `message` states no maximum |
| Alt formats: images and GIFs | [Page photos](https://developers.facebook.com/docs/graph-api/reference/v27.0/page/photos) | Confirmed 2026-11-13 for images: `alt_text_custom`; videos take no alt. Assumption: a GIF takes alt as an image |

## Instagram

| Facts | Source | Status |
| --- | --- | --- |
| Caption limit, mentions | [IG User Media](https://developers.facebook.com/documentation/instagram-platform/instagram-graph-api/reference/ig-user/media) | Confirmed |
| Alt text | [IG User Media](https://developers.facebook.com/documentation/instagram-platform/instagram-graph-api/reference/ig-user/media), [Content Publishing](https://developers.facebook.com/documentation/instagram-platform/content-publishing) | Confirmed 2026-11-11: `alt_text` is for a single image or an image in a carousel, up to 1000 characters; Reels and stories are not supported. The target publishes alt for `image` items only. Assumption: a GIF is not an image here, since JPEG is the only accepted image format, so a caller publishes it as video, which takes no alt. Reels and stories take no alt, so reel and story placement omit it, also on an image (2026-11-13) |
| Hashtags | [IG User Media](https://developers.facebook.com/documentation/instagram-platform/instagram-graph-api/reference/ig-user/media), [Use hashtags on Instagram](https://help.instagram.com/351460621611097) | Confirmed: the default follows the Help Centre; the API reference states 30, which callers supply through `hashtagLimit` |

## VK

`dev.vk.com` cannot be read: `https://dev.vk.com/ru/method/wall.post`, `https://dev.vk.com/ru/method/video.save`, `https://dev.vk.com/ru/method/photos.save` and `https://dev.vk.com/ru/reference/versions` fail to load. The official [VK API JSON Schema](https://github.com/VKCOM/vk-api-schema) substitutes for them.

| Facts | Source | Status |
| --- | --- | --- |
| Wall post text | [wall methods](https://raw.githubusercontent.com/VKCOM/vk-api-schema/master/wall/methods.json) | Confirmed: `message` states no maximum |
| Image alt as photo caption | [photos methods](https://raw.githubusercontent.com/VKCOM/vk-api-schema/master/photos/methods.json) | Confirmed |
| Video title of 128 and video alt of 5000 | [video methods](https://raw.githubusercontent.com/VKCOM/vk-api-schema/master/video/methods.json) | Assumption: the schema states no maximum for `name` or `description` |
| Alt formats: images and videos | [photos methods](https://raw.githubusercontent.com/VKCOM/vk-api-schema/master/photos/methods.json), [video methods](https://raw.githubusercontent.com/VKCOM/vk-api-schema/master/video/methods.json) | Confirmed 2026-11-13: `photos.save` takes a caption and `video.save` a description; a GIF is a document, which takes no alt |

## YouTube

| Facts | Source | Status |
| --- | --- | --- |
| Title and description limits, excluded `<` and `>` | [Videos resource](https://developers.google.com/youtube/v3/docs/videos) | Confirmed |
| Hashtags | [Find videos using hashtags](https://support.google.com/youtube/answer/6390658?hl=en) | Confirmed 2026-11-11: over 60 hashtags on a video, YouTube ignores every one of them. Assumption: the count covers the title and description together, since the page counts per video and names both as places for hashtags; the target keeps the title's hashtags and removes the description's excess sigils |
| Media alt: none | [videos.insert](https://developers.google.com/youtube/v3/docs/videos/insert), [thumbnails.set](https://developers.google.com/youtube/v3/docs/thumbnails/set) | Confirmed 2026-11-13: neither method has an alt parameter, so `youtube-plain` omits alt on every format |

## TikTok

| Facts | Source | Status |
| --- | --- | --- |
| Video caption | [Direct Post](https://developers.tiktok.com/doc/content-posting-api-reference-direct-post) | Confirmed |
| Photo title and description | [Photo](https://developers.tiktok.com/docs/en/content-posting-api-reference-photo-post) | Confirmed |
| Mentions | [Direct Post](https://developers.tiktok.com/doc/content-posting-api-reference-direct-post), [Photo](https://developers.tiktok.com/docs/en/content-posting-api-reference-photo-post), [Mentions on TikTok](https://support.tiktok.com/en/using-tiktok/messaging-and-notifications/mentions-on-tiktok) | Confirmed 2026-11-12: no page states a maximum number of mentions in a caption, title or description, so `tiktok-plain` declares no `mentions` limit; the help article only gates mentions on the mentioned account's settings |
| Media alt: none | [Direct Post](https://developers.tiktok.com/doc/content-posting-api-reference-direct-post), [Photo](https://developers.tiktok.com/docs/en/content-posting-api-reference-photo-post) | Confirmed 2026-11-13: neither post type has an alt parameter, so `tiktok-plain` omits alt on every format |
