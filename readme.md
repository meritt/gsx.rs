# gsx.rs

The `gsx/1` markup for posts and messages. One Rust core parses, serializes and projects documents onto publishing targets, fits their fields to configured limits and reports transformations. Bindings expose the same results in JavaScript, Swift and Python.

## Installation

| Package | Command |
| --- | --- |
| Rust | `cargo add gsx-core` |
| JavaScript | `pnpm add @meritt/gsx-wasm` |
| Python | `pip install gsx` |

SwiftPM dependency:

```swift
.package(url: "https://github.com/meritt/gsx.rs", exact: "0.3.0")
```

Package platforms and loading: [bindings](docs/decisions/bindings.md#packages).

## Usage

### Rust

```rust
use gsx_core::{document, project, targets};

fn main() -> Result<(), document::Error> {
    let text = "Hello **world**.";
    let document = document::parse(text, &[])?;

    let target = targets::target("telegram-entities").unwrap();
    let facts = project::Facts {
        entities_allowed: true,
        ..Default::default()
    };
    let projection = project::project(&document, target, &facts);

    assert_eq!(projection.fields[0].text, "Hello world.");

    Ok(())
}
```

### JavaScript

```js
import { load } from '@meritt/gsx-wasm';

const gsx = await load();
const text = 'Hello **world**.';

const projection = gsx.project(text, 'telegram-entities', {
  entitiesAllowed: true
});

console.log(projection.fields[0]);
```

```json
{
  "name": "text",
  "text": "Hello world.",
  "links": [],
  "entities": [{
    "style": "bold",
    "range": [6, 11],
    "value": null
  }]
}
```

### Swift

```swift
import GSX

let text = "Hello **world**."
let facts = GSX.Facts(entitiesAllowed: true)

let projection = try GSX.project(text, target: "telegram-entities", facts: facts)

precondition(projection.fields[0].text == "Hello world.")
```

### Python

```python
import gsx

text = "Hello **world**."
facts = gsx.Facts(entities_allowed=True)

projection = gsx.project(text, "telegram-entities", facts)

assert projection.fields[0].text == "Hello world."
```

Check document diagnostics before publishing and reject projections with `blocked` changes. The caller resolves media and account references and sends provider requests; gsx performs no I/O.

## Formatting

```markdown
# gsx 0.2 is out

**Native projections** for 19 targets, from Telegram to TikTok. Write *once*, publish everywhere.

Read the [changelog](https://example.com/changelog "Release notes") or run `pnpm add @meritt/gsx-wasm`.

::media{ids="banner,preview" layout="slideshow"}

> Keep the message. Adapt the formatting.

- ==One Rust core== across all bindings
- ~~Per-platform markup~~ one GSX document
- ++Explicit limits++ and transformation reports
- ||A surprise|| for Discord users

Release stream ::time[11 November, 18:00 TRT]{datetime="2026-11-11T18:00:00+03:00"}. Thanks @meritt! #release #rust
```

<details>
<summary>Telegram</summary>

<p><strong><code>telegram-html</code></strong></p>

<table>
<thead><tr><th scope="col" align="left">Output</th><th scope="col" align="left">Preview</th></tr></thead>
<tbody><tr>
<td width="50%" valign="top">
<p><strong>text</strong></p>
<p><code>&lt;b&gt;gsx 0.2 is out&lt;/b&gt;<br>
<br>
&lt;b&gt;Native projections&lt;/b&gt; for 19 targets, from Telegram to TikTok. Write &lt;i&gt;once&lt;/i&gt;, publish everywhere.<br>
<br>
Read the &lt;u&gt;&lt;a href=&quot;https://example.com/changelog&quot;&gt;changelog&lt;/a&gt;&lt;/u&gt; or run &lt;code&gt;pnpm add @meritt/gsx-wasm&lt;/code&gt;.<br>
<br>
&lt;blockquote&gt;Keep the message. Adapt the formatting.&lt;/blockquote&gt;<br>
<br>
– One Rust core across all bindings<br>
– &lt;s&gt;Per-platform markup&lt;/s&gt; one GSX document<br>
– &lt;u&gt;Explicit limits&lt;/u&gt; and transformation reports<br>
– &lt;tg-spoiler&gt;A surprise&lt;/tg-spoiler&gt; for Discord users<br>
<br>
Release stream &lt;tg-time unix=&quot;1794409200&quot;&gt;11 November, 18:00 TRT&lt;/tg-time&gt;. Thanks @meritt! #release #rust</code></p>
<p><strong>items</strong></p>
<p><code>[<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:banner&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;gsx 0.2: one source, native projections&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;},<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:preview&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;The same post adapted for Telegram and Discord&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;}<br>
]</code></p>
</td>
<td width="50%" valign="top">
<p><b>gsx 0.2 is out</b></p>
<p><b>Native projections</b> for 19 targets, from Telegram to TikTok. Write <i>once</i>, publish everywhere.</p>
<p>Read the <a href="https://example.com/changelog">changelog</a> or run <code>pnpm add @meritt/gsx-wasm</code>.</p>
<blockquote>Keep the message. Adapt the formatting.</blockquote>
<p>– One Rust core across all bindings<br>– <del>Per-platform markup</del> one GSX document<br>– <ins>Explicit limits</ins> and transformation reports<br>– <span title="A surprise">████████</span> for Discord users</p>
<p>Release stream 11 November, 18:00 TRT. Thanks @meritt! #release #rust</p>
<hr><p><em>Attachments · 2 images</em><br><code>media:banner</code> <code>media:preview</code></p>
</td>
</tr></tbody>
</table>

<p><strong><code>telegram-html-rich</code></strong></p>

<table>
<thead><tr><th scope="col" align="left">Output</th><th scope="col" align="left">Preview</th></tr></thead>
<tbody><tr>
<td width="50%" valign="top">
<p><strong>text</strong></p>
<p><code>&lt;p&gt;&lt;b&gt;gsx 0.2 is out&lt;/b&gt;&lt;br&gt;&lt;br&gt;&lt;b&gt;Native projections&lt;/b&gt; for 19 targets, from Telegram to TikTok. Write &lt;i&gt;once&lt;/i&gt;, publish everywhere.&lt;br&gt;&lt;br&gt;Read the &lt;u&gt;&lt;a href=&quot;https://example.com/changelog&quot;&gt;changelog&lt;/a&gt;&lt;/u&gt; or run &lt;code&gt;pnpm add @meritt/gsx-wasm&lt;/code&gt;.&lt;/p&gt;&lt;br&gt;&lt;br&gt;&lt;tg-slideshow&gt;&lt;img src=&quot;media:banner&quot;/&gt;&lt;img src=&quot;media:preview&quot;/&gt;&lt;figcaption&gt;1. gsx 0.2: one source, native projections&lt;br&gt;2. The same post adapted for Telegram and Discord&lt;/figcaption&gt;&lt;/tg-slideshow&gt;&lt;br&gt;&lt;br&gt;&lt;blockquote&gt;&lt;p&gt;Keep the message. Adapt the formatting.&lt;/p&gt;&lt;/blockquote&gt;&lt;br&gt;&lt;br&gt;&lt;ul&gt;&lt;li&gt;&lt;p&gt;&lt;mark&gt;One Rust core&lt;/mark&gt; across all bindings&lt;/p&gt;&lt;/li&gt;&lt;li&gt;&lt;p&gt;&lt;s&gt;Per-platform markup&lt;/s&gt; one GSX document&lt;/p&gt;&lt;/li&gt;&lt;li&gt;&lt;p&gt;&lt;u&gt;Explicit limits&lt;/u&gt; and transformation reports&lt;/p&gt;&lt;/li&gt;&lt;li&gt;&lt;p&gt;&lt;tg-spoiler&gt;A surprise&lt;/tg-spoiler&gt; for Discord users&lt;/p&gt;&lt;/li&gt;&lt;/ul&gt;&lt;br&gt;&lt;br&gt;&lt;p&gt;Release stream &lt;tg-time unix=&quot;1794409200&quot;&gt;11 November, 18:00 TRT&lt;/tg-time&gt;. Thanks @meritt! #release #rust&lt;/p&gt;</code></p>
<p><strong>items</strong></p>
<p><code>[<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:banner&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;gsx 0.2: one source, native projections&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;},<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:preview&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;The same post adapted for Telegram and Discord&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;}<br>
]</code></p>
</td>
<td width="50%" valign="top">
<p><b>gsx 0.2 is out</b></p><p><b>Native projections</b> for 19 targets, from Telegram to TikTok. Write <i>once</i>, publish everywhere.</p><p>Read the <a href="https://example.com/changelog">changelog</a> or run <code>pnpm add @meritt/gsx-wasm</code>.</p><p><em>Carousel · 2 images</em><br><code>media:banner</code> <code>media:preview</code></p><blockquote>Keep the message. Adapt the formatting.</blockquote><ul><li><mark>One Rust core</mark> across all bindings</li><li><del>Per-platform markup</del> one GSX document</li><li><ins>Explicit limits</ins> and transformation reports</li><li><span title="A surprise">████████</span> for Discord users</li></ul><p>Release stream 11 November, 18:00 TRT. Thanks @meritt! #release #rust</p>
</td>
</tr></tbody>
</table>

<p><strong><code>telegram-entities</code></strong></p>

<p>Truncated to the default 200-character limit.</p>
<table>
<thead><tr><th scope="col" align="left">Output</th><th scope="col" align="left">Preview</th></tr></thead>
<tbody><tr>
<td width="50%" valign="top">
<p><strong>text</strong></p>
<p><code>gsx 0.2 is out<br>
<br>
Native projections for 19 targets, from Telegram to TikTok. Write once, publish everywhere.<br>
<br>
Read the changelog or run pnpm add @meritt/gsx-wasm.<br>
<br>
Keep the message. Adapt the…</code></p>
<p><strong>entities</strong></p>
<p><code>[<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;style&quot;: &quot;bold&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;range&quot;: [<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;0,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;14<br>
&nbsp;&nbsp;&nbsp;&nbsp;],<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;value&quot;: null<br>
&nbsp;&nbsp;},<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;style&quot;: &quot;bold&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;range&quot;: [<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;16,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;34<br>
&nbsp;&nbsp;&nbsp;&nbsp;],<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;value&quot;: null<br>
&nbsp;&nbsp;},<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;style&quot;: &quot;italic&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;range&quot;: [<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;82,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;86<br>
&nbsp;&nbsp;&nbsp;&nbsp;],<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;value&quot;: null<br>
&nbsp;&nbsp;},<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;style&quot;: &quot;underline&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;range&quot;: [<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;118,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;127<br>
&nbsp;&nbsp;&nbsp;&nbsp;],<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;value&quot;: null<br>
&nbsp;&nbsp;},<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;style&quot;: &quot;link&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;range&quot;: [<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;118,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;127<br>
&nbsp;&nbsp;&nbsp;&nbsp;],<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;value&quot;: &quot;https://example.com/changelog&quot;<br>
&nbsp;&nbsp;},<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;style&quot;: &quot;code&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;range&quot;: [<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;135,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;160<br>
&nbsp;&nbsp;&nbsp;&nbsp;],<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;value&quot;: null<br>
&nbsp;&nbsp;},<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;style&quot;: &quot;blockquote&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;range&quot;: [<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;163,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;191<br>
&nbsp;&nbsp;&nbsp;&nbsp;],<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;value&quot;: null<br>
&nbsp;&nbsp;}<br>
]</code></p>
<p><strong>items</strong></p>
<p><code>[<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:banner&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;gsx 0.2: one source, native projections&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;},<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:preview&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;The same post adapted for Telegram and Discord&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;}<br>
]</code></p>
</td>
<td width="50%" valign="top">
<div><strong>gsx 0.2 is out</strong><br>
<br>
<strong>Native projections</strong> for 19 targets, from Telegram to TikTok. Write <em>once</em>, publish everywhere.<br>
<br>
Read the <a href="https://example.com/changelog">changelog</a> or run <code>pnpm add @meritt/gsx-wasm</code>.<br>
<br>
<blockquote>Keep the message. Adapt the…</blockquote></div>
<hr><p><em>Attachments · 2 images</em><br><code>media:banner</code> <code>media:preview</code></p>
</td>
</tr></tbody>
</table>

</details>

<details>
<summary>Discord</summary>

<p><strong><code>discord-markdown</code></strong></p>

<table>
<thead><tr><th scope="col" align="left">Output</th><th scope="col" align="left">Preview</th></tr></thead>
<tbody><tr>
<td width="50%" valign="top">
<p><strong>text</strong></p>
<p><code>**gsx 0.2 is out**<br>
<br>
**Native projections** for 19 targets, from Telegram to TikTok. Write *once*, publish everywhere.<br>
<br>
Read the __[changelog](https://example.com/changelog)__ or run `pnpm add @meritt/gsx-wasm`.<br>
<br>
&gt; Keep the message. Adapt the formatting.<br>
<br>
- One Rust core across all bindings<br>
- ~~Per\-platform markup~~ one GSX document<br>
- __Explicit limits__ and transformation reports<br>
- ||A surprise|| for Discord users<br>
<br>
Release stream &lt;t:1794409200:f&gt;. Thanks @meritt\! #release #rust</code></p>
<p><strong>items</strong></p>
<p><code>[<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:banner&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;gsx 0.2: one source, native projections&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;},<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:preview&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;The same post adapted for Telegram and Discord&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;}<br>
]</code></p>
</td>
<td width="50%" valign="top">
<p><strong>gsx 0.2 is out</strong></p>
<p><strong>Native projections</strong> for 19 targets, from Telegram to TikTok. Write <em>once</em>, publish everywhere.</p>
<p>Read the <a href="https://example.com/changelog">changelog</a> or run <code>pnpm add @meritt/gsx-wasm</code>.</p>
<blockquote>Keep the message. Adapt the formatting.</blockquote>
<ul><li>One Rust core across all bindings</li><li><del>Per-platform markup</del> one GSX document</li><li><ins>Explicit limits</ins> and transformation reports</li><li><span title="A surprise">████████</span> for Discord users</li></ul>
<p>Release stream <span title="11 November 2026, 18:00 TRT">11 November 2026 at 18:00</span>. Thanks @meritt! #release #rust</p>
<hr><p><em>Attachments · 2 images</em><br><code>media:banner</code> <code>media:preview</code></p>
</td>
</tr></tbody>
</table>

<p><strong><code>discord-blocks</code></strong></p>

<table>
<thead><tr><th scope="col" align="left">Output</th><th scope="col" align="left">Preview</th></tr></thead>
<tbody><tr>
<td width="50%" valign="top">
<p><strong>blocks</strong></p>
<p><code>[<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;text&quot;: &quot;**gsx 0.2 is out**\n\n**Native projections** for 19 targets, from Telegram to TikTok. Write *once*, publish everywhere.\n\nRead the __[changelog](https://example.com/changelog)__ or run `pnpm add @meritt/gsx-wasm`.\n\n&quot;<br>
&nbsp;&nbsp;},<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;items&quot;: [<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:banner&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;gsx 0.2: one source, native projections&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;}<br>
&nbsp;&nbsp;&nbsp;&nbsp;],<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;layout&quot;: null<br>
&nbsp;&nbsp;},<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;items&quot;: [<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:preview&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;The same post adapted for Telegram and Discord&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;}<br>
&nbsp;&nbsp;&nbsp;&nbsp;],<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;layout&quot;: null<br>
&nbsp;&nbsp;},<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;text&quot;: &quot;\n\n&gt; Keep the message. Adapt the formatting.\n\n- One Rust core across all bindings\n- ~~Per\\-platform markup~~ one GSX document\n- __Explicit limits__ and transformation reports\n- ||A surprise|| for Discord users\n\nRelease stream &lt;t:1794409200:f&gt;. Thanks @meritt\\! #release #rust&quot;<br>
&nbsp;&nbsp;}<br>
]</code></p>
</td>
<td width="50%" valign="top">
<p><strong>gsx 0.2 is out</strong></p>
<p><strong>Native projections</strong> for 19 targets, from Telegram to TikTok. Write <em>once</em>, publish everywhere.</p>
<p>Read the <a href="https://example.com/changelog">changelog</a> or run <code>pnpm add @meritt/gsx-wasm</code>.</p>
<p><em>Image</em><br><code>media:banner</code></p>
<p><em>Image</em><br><code>media:preview</code></p>
<blockquote>Keep the message. Adapt the formatting.</blockquote>
<ul><li>One Rust core across all bindings</li><li><del>Per-platform markup</del> one GSX document</li><li><ins>Explicit limits</ins> and transformation reports</li><li><span title="A surprise">████████</span> for Discord users</li></ul>
<p>Release stream <span title="11 November 2026, 18:00 TRT">11 November 2026 at 18:00</span>. Thanks @meritt! #release #rust</p>
</td>
</tr></tbody>
</table>

</details>

<details>
<summary>Slack</summary>

<p><strong><code>slack-markdown</code></strong></p>

<table>
<thead><tr><th scope="col" align="left">Output</th><th scope="col" align="left">Preview</th></tr></thead>
<tbody><tr>
<td width="50%" valign="top">
<p><strong>text</strong></p>
<p><code>**gsx 0.2 is out**<br>
<br>
**Native projections** for 19 targets, from Telegram to TikTok. Write *once*, publish everywhere.<br>
<br>
Read the [changelog](https://example.com/changelog) or run `pnpm add @meritt/gsx-wasm`.<br>
<br>
&gt; Keep the message. Adapt the formatting.<br>
<br>
- One Rust core across all bindings<br>
- ~~Per\-platform markup~~ one GSX document<br>
- Explicit limits and transformation reports<br>
- A surprise for Discord users<br>
<br>
Release stream &lt;!date^1794409200^{date_short} {time}|11 November, 18:00 TRT&gt;. Thanks @meritt\! #release #rust</code></p>
<p><strong>items</strong></p>
<p><code>[<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:banner&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;gsx 0.2: one source, native projections&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;},<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:preview&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;The same post adapted for Telegram and Discord&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;}<br>
]</code></p>
</td>
<td width="50%" valign="top">
<p><strong>gsx 0.2 is out</strong></p>
<p><strong>Native projections</strong> for 19 targets, from Telegram to TikTok. Write <em>once</em>, publish everywhere.</p>
<p>Read the <a href="https://example.com/changelog">changelog</a> or run <code>pnpm add @meritt/gsx-wasm</code>.</p>
<blockquote>Keep the message. Adapt the formatting.</blockquote>
<ul><li>One Rust core across all bindings</li><li><del>Per-platform markup</del> one GSX document</li><li>Explicit limits and transformation reports</li><li>A surprise for Discord users</li></ul>
<p>Release stream <span title="11 November 2026, 18:00 TRT">11 November, 18:00 TRT</span>. Thanks @meritt! #release #rust</p>
<hr><p><em>Attachments · 2 images</em><br><code>media:banner</code> <code>media:preview</code></p>
</td>
</tr></tbody>
</table>

<p><strong><code>slack-blocks</code></strong></p>

<table>
<thead><tr><th scope="col" align="left">Output</th><th scope="col" align="left">Preview</th></tr></thead>
<tbody><tr>
<td width="50%" valign="top">
<p><strong>blocks</strong></p>
<p><code>[<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;text&quot;: &quot;**gsx 0.2 is out**\n\n**Native projections** for 19 targets, from Telegram to TikTok. Write *once*, publish everywhere.\n\nRead the [changelog](https://example.com/changelog) or run `pnpm add @meritt/gsx-wasm`.\n\n&quot;<br>
&nbsp;&nbsp;},<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;items&quot;: [<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:banner&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;gsx 0.2: one source, native projections&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;},<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:preview&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;The same post adapted for Telegram and Discord&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;}<br>
&nbsp;&nbsp;&nbsp;&nbsp;],<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;layout&quot;: &quot;slideshow&quot;<br>
&nbsp;&nbsp;},<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;text&quot;: &quot;\n\n&gt; Keep the message. Adapt the formatting.\n\n- One Rust core across all bindings\n- ~~Per\\-platform markup~~ one GSX document\n- Explicit limits and transformation reports\n- A surprise for Discord users\n\nRelease stream &lt;!date^1794409200^{date_short} {time}|11 November, 18:00 TRT&gt;. Thanks @meritt\\! #release #rust&quot;<br>
&nbsp;&nbsp;}<br>
]</code></p>
</td>
<td width="50%" valign="top">
<p><strong>gsx 0.2 is out</strong></p>
<p><strong>Native projections</strong> for 19 targets, from Telegram to TikTok. Write <em>once</em>, publish everywhere.</p>
<p>Read the <a href="https://example.com/changelog">changelog</a> or run <code>pnpm add @meritt/gsx-wasm</code>.</p>
<p><em>Carousel · 2 images</em><br><code>media:banner</code> <code>media:preview</code></p>
<blockquote>Keep the message. Adapt the formatting.</blockquote>
<ul><li>One Rust core across all bindings</li><li><del>Per-platform markup</del> one GSX document</li><li>Explicit limits and transformation reports</li><li>A surprise for Discord users</li></ul>
<p>Release stream <span title="11 November 2026, 18:00 TRT">11 November, 18:00 TRT</span>. Thanks @meritt! #release #rust</p>
</td>
</tr></tbody>
</table>

<p><strong><code>slack-plain</code></strong></p>

<table>
<thead><tr><th scope="col" align="left">Output</th><th scope="col" align="left">Preview</th></tr></thead>
<tbody><tr>
<td width="50%" valign="top">
<p><strong>text</strong></p>
<p><code>gsx 0.2 is out<br>
<br>
Native projections for 19 targets, from Telegram to TikTok. Write once, publish everywhere.<br>
<br>
Read the changelog (https://example.com/changelog) or run pnpm add @meritt/gsx-wasm.<br>
<br>
› Keep the message. Adapt the formatting.<br>
<br>
– One Rust core across all bindings<br>
– Per-platform markup one GSX document<br>
– Explicit limits and transformation reports<br>
– A surprise for Discord users<br>
<br>
Release stream 11 November, 18:00 TRT. Thanks @meritt! #release #rust</code></p>
<p><strong>items</strong></p>
<p><code>[<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:banner&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;gsx 0.2: one source, native projections&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;},<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:preview&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;The same post adapted for Telegram and Discord&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;}<br>
]</code></p>
</td>
<td width="50%" valign="top">
<div>gsx 0.2 is out<br>
<br>
Native projections for 19 targets, from Telegram to TikTok. Write once, publish everywhere.<br>
<br>
Read the changelog (<a href="https://example.com/changelog">https://example.com/changelog</a>) or run pnpm add @meritt/gsx-wasm.<br>
<br>
› Keep the message. Adapt the formatting.<br>
<br>
– One Rust core across all bindings<br>
– Per-platform markup one GSX document<br>
– Explicit limits and transformation reports<br>
– A surprise for Discord users<br>
<br>
Release stream 11 November, 18:00 TRT. Thanks @meritt! #release #rust</div>
<hr><p><em>Attachments · 2 images</em><br><code>media:banner</code> <code>media:preview</code></p>
</td>
</tr></tbody>
</table>

</details>

<details>
<summary>MAX</summary>

<p><strong><code>max-html</code></strong></p>

<table>
<thead><tr><th scope="col" align="left">Output</th><th scope="col" align="left">Preview</th></tr></thead>
<tbody><tr>
<td width="50%" valign="top">
<p><strong>text</strong></p>
<p><code>&lt;b&gt;gsx 0.2 is out&lt;/b&gt;<br>
<br>
&lt;b&gt;Native projections&lt;/b&gt; for 19 targets, from Telegram to TikTok. Write &lt;i&gt;once&lt;/i&gt;, publish everywhere.<br>
<br>
Read the &lt;u&gt;&lt;a href=&quot;https://example.com/changelog&quot;&gt;changelog&lt;/a&gt;&lt;/u&gt; or run &lt;code&gt;pnpm add @meritt/gsx-wasm&lt;/code&gt;.<br>
<br>
&lt;blockquote&gt;Keep the message. Adapt the formatting.&lt;/blockquote&gt;<br>
<br>
– &lt;mark&gt;One Rust core&lt;/mark&gt; across all bindings<br>
– &lt;s&gt;Per-platform markup&lt;/s&gt; one GSX document<br>
– &lt;u&gt;Explicit limits&lt;/u&gt; and transformation reports<br>
– A surprise for Discord users<br>
<br>
Release stream 11 November, 18:00 TRT. Thanks @meritt! #release #rust</code></p>
<p><strong>items</strong></p>
<p><code>[<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:banner&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;gsx 0.2: one source, native projections&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;},<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:preview&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;The same post adapted for Telegram and Discord&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;}<br>
]</code></p>
</td>
<td width="50%" valign="top">
<p><b>gsx 0.2 is out</b></p>
<p><b>Native projections</b> for 19 targets, from Telegram to TikTok. Write <i>once</i>, publish everywhere.</p>
<p>Read the <a href="https://example.com/changelog">changelog</a> or run <code>pnpm add @meritt/gsx-wasm</code>.</p>
<blockquote>Keep the message. Adapt the formatting.</blockquote>
<p>– <mark>One Rust core</mark> across all bindings<br>– <del>Per-platform markup</del> one GSX document<br>– <ins>Explicit limits</ins> and transformation reports<br>– A surprise for Discord users</p>
<p>Release stream 11 November, 18:00 TRT. Thanks @meritt! #release #rust</p>
<hr><p><em>Attachments · 2 images</em><br><code>media:banner</code> <code>media:preview</code></p>
</td>
</tr></tbody>
</table>

</details>

<details>
<summary>Bluesky</summary>

<p><strong><code>bluesky-entities</code></strong></p>

<p>Truncated to the default 300-character limit.</p>
<table>
<thead><tr><th scope="col" align="left">Output</th><th scope="col" align="left">Preview</th></tr></thead>
<tbody><tr>
<td width="50%" valign="top">
<p><strong>text</strong></p>
<p><code>gsx 0.2 is out<br>
<br>
Native projections for 19 targets, from Telegram to TikTok. Write once, publish everywhere.<br>
<br>
Read the changelog or run pnpm add @meritt/gsx-wasm.<br>
<br>
› Keep the message. Adapt the formatting.<br>
<br>
– One Rust core across all bindings<br>
– Per-platform markup one GSX document<br>
– Explicit limits…</code></p>
<p><strong>entities</strong></p>
<p><code>[<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;style&quot;: &quot;link&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;range&quot;: [<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;118,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;127<br>
&nbsp;&nbsp;&nbsp;&nbsp;],<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;value&quot;: &quot;https://example.com/changelog&quot;<br>
&nbsp;&nbsp;}<br>
]</code></p>
<p><strong>items</strong></p>
<p><code>[<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:banner&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;gsx 0.2: one source, native projections&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;},<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:preview&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;The same post adapted for Telegram and Discord&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;}<br>
]</code></p>
</td>
<td width="50%" valign="top">
<div>gsx 0.2 is out<br>
<br>
Native projections for 19 targets, from Telegram to TikTok. Write once, publish everywhere.<br>
<br>
Read the <a href="https://example.com/changelog">changelog</a> or run pnpm add @meritt/gsx-wasm.<br>
<br>
› Keep the message. Adapt the formatting.<br>
<br>
– One Rust core across all bindings<br>
– Per-platform markup one GSX document<br>
– Explicit limits…</div>
<hr><p><em>Attachments · 2 images</em><br><code>media:banner</code> <code>media:preview</code></p>
</td>
</tr></tbody>
</table>

</details>

<details>
<summary>Threads</summary>

<p><strong><code>threads-entities</code></strong></p>

<table>
<thead><tr><th scope="col" align="left">Output</th><th scope="col" align="left">Preview</th></tr></thead>
<tbody><tr>
<td width="50%" valign="top">
<p><strong>text</strong></p>
<p><code>gsx 0.2 is out<br>
<br>
Native projections for 19 targets, from Telegram to TikTok. Write once, publish everywhere.<br>
<br>
Read the changelog (https://example.com/changelog) or run pnpm add @meritt/gsx-wasm.<br>
<br>
› Keep the message. Adapt the formatting.<br>
<br>
– One Rust core across all bindings<br>
– Per-platform markup one GSX document<br>
– Explicit limits and transformation reports<br>
– A surprise for Discord users<br>
<br>
Release stream 11 November, 18:00 TRT. Thanks @meritt! #release #rust</code></p>
<p><strong>entities</strong></p>
<p><code>[<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;style&quot;: &quot;spoiler&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;range&quot;: [<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;360,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;370<br>
&nbsp;&nbsp;&nbsp;&nbsp;],<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;value&quot;: null<br>
&nbsp;&nbsp;}<br>
]</code></p>
<p><strong>items</strong></p>
<p><code>[<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:banner&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;gsx 0.2: one source, native projections&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;},<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:preview&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;The same post adapted for Telegram and Discord&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;}<br>
]</code></p>
</td>
<td width="50%" valign="top">
<div>gsx 0.2 is out<br>
<br>
Native projections for 19 targets, from Telegram to TikTok. Write once, publish everywhere.<br>
<br>
Read the changelog (<a href="https://example.com/changelog">https://example.com/changelog</a>) or run pnpm add @meritt/gsx-wasm.<br>
<br>
› Keep the message. Adapt the formatting.<br>
<br>
– One Rust core across all bindings<br>
– Per-platform markup one GSX document<br>
– Explicit limits and transformation reports<br>
– <span title="A surprise">████████</span> for Discord users<br>
<br>
Release stream 11 November, 18:00 TRT. Thanks @meritt! #release #rust</div>
<hr><p><em>Attachments · 2 images</em><br><code>media:banner</code> <code>media:preview</code></p>
</td>
</tr></tbody>
</table>

</details>

<details>
<summary>X</summary>

<p><strong><code>x-plain</code></strong></p>

<p>Truncated to the default 280-character limit.</p>
<table>
<thead><tr><th scope="col" align="left">Output</th><th scope="col" align="left">Preview</th></tr></thead>
<tbody><tr>
<td width="50%" valign="top">
<p><strong>text</strong></p>
<p><code>gsx 0.2 is out<br>
<br>
Native projections for 19 targets, from Telegram to TikTok. Write once, publish everywhere.<br>
<br>
Read the changelog (https://example.com/changelog) or run pnpm add @meritt/gsx-wasm.<br>
<br>
› Keep the message. Adapt the formatting.<br>
<br>
– One Rust core across all bindings<br>
– Per-pla…</code></p>
<p><strong>items</strong></p>
<p><code>[<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:banner&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;gsx 0.2: one source, native projections&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;},<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:preview&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;The same post adapted for Telegram and Discord&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;}<br>
]</code></p>
</td>
<td width="50%" valign="top">
<div>gsx 0.2 is out<br>
<br>
Native projections for 19 targets, from Telegram to TikTok. Write once, publish everywhere.<br>
<br>
Read the changelog (<a href="https://example.com/changelog">https://example.com/changelog</a>) or run pnpm add @meritt/gsx-wasm.<br>
<br>
› Keep the message. Adapt the formatting.<br>
<br>
– One Rust core across all bindings<br>
– Per-pla…</div>
<hr><p><em>Attachments · 2 images</em><br><code>media:banner</code> <code>media:preview</code></p>
</td>
</tr></tbody>
</table>

</details>

<details>
<summary>Mastodon</summary>

<p><strong><code>mastodon-plain</code></strong></p>

<table>
<thead><tr><th scope="col" align="left">Output</th><th scope="col" align="left">Preview</th></tr></thead>
<tbody><tr>
<td width="50%" valign="top">
<p><strong>text</strong></p>
<p><code>gsx 0.2 is out<br>
<br>
Native projections for 19 targets, from Telegram to TikTok. Write once, publish everywhere.<br>
<br>
Read the changelog (https://example.com/changelog) or run pnpm add @meritt/gsx-wasm.<br>
<br>
› Keep the message. Adapt the formatting.<br>
<br>
– One Rust core across all bindings<br>
– Per-platform markup one GSX document<br>
– Explicit limits and transformation reports<br>
– A surprise for Discord users<br>
<br>
Release stream 11 November, 18:00 TRT. Thanks @meritt! #release #rust</code></p>
<p><strong>items</strong></p>
<p><code>[<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:banner&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;gsx 0.2: one source, native projections&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;},<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:preview&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;The same post adapted for Telegram and Discord&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;}<br>
]</code></p>
</td>
<td width="50%" valign="top">
<div>gsx 0.2 is out<br>
<br>
Native projections for 19 targets, from Telegram to TikTok. Write once, publish everywhere.<br>
<br>
Read the changelog (<a href="https://example.com/changelog">https://example.com/changelog</a>) or run pnpm add @meritt/gsx-wasm.<br>
<br>
› Keep the message. Adapt the formatting.<br>
<br>
– One Rust core across all bindings<br>
– Per-platform markup one GSX document<br>
– Explicit limits and transformation reports<br>
– A surprise for Discord users<br>
<br>
Release stream 11 November, 18:00 TRT. Thanks @meritt! #release #rust</div>
<hr><p><em>Attachments · 2 images</em><br><code>media:banner</code> <code>media:preview</code></p>
</td>
</tr></tbody>
</table>

</details>

<details>
<summary>LinkedIn</summary>

<p><strong><code>linkedin-plain</code></strong></p>

<table>
<thead><tr><th scope="col" align="left">Output</th><th scope="col" align="left">Preview</th></tr></thead>
<tbody><tr>
<td width="50%" valign="top">
<p><strong>text</strong></p>
<p><code>gsx 0.2 is out<br>
<br>
Native projections for 19 targets, from Telegram to TikTok. Write once, publish everywhere.<br>
<br>
Read the changelog \(https://example.com/changelog\) or run pnpm add \@meritt/gsx-wasm.<br>
<br>
› Keep the message. Adapt the formatting.<br>
<br>
– One Rust core across all bindings<br>
– Per-platform markup one GSX document<br>
– Explicit limits and transformation reports<br>
– A surprise for Discord users<br>
<br>
Release stream 11 November, 18:00 TRT. Thanks \@meritt! #release #rust</code></p>
<p><strong>items</strong></p>
<p><code>[<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:banner&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;gsx 0.2: one source, native projections&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;},<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:preview&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;The same post adapted for Telegram and Discord&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;}<br>
]</code></p>
</td>
<td width="50%" valign="top">
<div>gsx 0.2 is out<br>
<br>
Native projections for 19 targets, from Telegram to TikTok. Write once, publish everywhere.<br>
<br>
Read the changelog (<a href="https://example.com/changelog">https://example.com/changelog</a>) or run pnpm add @meritt/gsx-wasm.<br>
<br>
› Keep the message. Adapt the formatting.<br>
<br>
– One Rust core across all bindings<br>
– Per-platform markup one GSX document<br>
– Explicit limits and transformation reports<br>
– A surprise for Discord users<br>
<br>
Release stream 11 November, 18:00 TRT. Thanks @meritt! #release #rust</div>
<hr><p><em>Attachments · 2 images</em><br><code>media:banner</code> <code>media:preview</code></p>
</td>
</tr></tbody>
</table>

</details>

<details>
<summary>Facebook</summary>

<p><strong><code>facebook-plain</code></strong></p>

<table>
<thead><tr><th scope="col" align="left">Output</th><th scope="col" align="left">Preview</th></tr></thead>
<tbody><tr>
<td width="50%" valign="top">
<p><strong>text</strong></p>
<p><code>gsx 0.2 is out<br>
<br>
Native projections for 19 targets, from Telegram to TikTok. Write once, publish everywhere.<br>
<br>
Read the changelog (https://example.com/changelog) or run pnpm add @meritt/gsx-wasm.<br>
<br>
› Keep the message. Adapt the formatting.<br>
<br>
– One Rust core across all bindings<br>
– Per-platform markup one GSX document<br>
– Explicit limits and transformation reports<br>
– A surprise for Discord users<br>
<br>
Release stream 11 November, 18:00 TRT. Thanks @meritt! #release #rust</code></p>
<p><strong>items</strong></p>
<p><code>[<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:banner&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;gsx 0.2: one source, native projections&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;},<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:preview&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;The same post adapted for Telegram and Discord&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;}<br>
]</code></p>
</td>
<td width="50%" valign="top">
<div>gsx 0.2 is out<br>
<br>
Native projections for 19 targets, from Telegram to TikTok. Write once, publish everywhere.<br>
<br>
Read the changelog (<a href="https://example.com/changelog">https://example.com/changelog</a>) or run pnpm add @meritt/gsx-wasm.<br>
<br>
› Keep the message. Adapt the formatting.<br>
<br>
– One Rust core across all bindings<br>
– Per-platform markup one GSX document<br>
– Explicit limits and transformation reports<br>
– A surprise for Discord users<br>
<br>
Release stream 11 November, 18:00 TRT. Thanks @meritt! #release #rust</div>
<hr><p><em>Attachments · 2 images</em><br><code>media:banner</code> <code>media:preview</code></p>
</td>
</tr></tbody>
</table>

</details>

<details>
<summary>Instagram</summary>

<p><strong><code>instagram-plain</code></strong></p>

<table>
<thead><tr><th scope="col" align="left">Output</th><th scope="col" align="left">Preview</th></tr></thead>
<tbody><tr>
<td width="50%" valign="top">
<p><strong>text</strong></p>
<p><code>gsx 0.2 is out<br>
<br>
Native projections for 19 targets, from Telegram to TikTok. Write once, publish everywhere.<br>
<br>
Read the changelog (https://example.com/changelog) or run pnpm add @meritt/gsx-wasm.<br>
<br>
› Keep the message. Adapt the formatting.<br>
<br>
– One Rust core across all bindings<br>
– Per-platform markup one GSX document<br>
– Explicit limits and transformation reports<br>
– A surprise for Discord users<br>
<br>
Release stream 11 November, 18:00 TRT. Thanks @meritt! #release #rust</code></p>
<p><strong>items</strong></p>
<p><code>[<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:banner&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;gsx 0.2: one source, native projections&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;},<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:preview&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;The same post adapted for Telegram and Discord&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;}<br>
]</code></p>
</td>
<td width="50%" valign="top">
<div>gsx 0.2 is out<br>
<br>
Native projections for 19 targets, from Telegram to TikTok. Write once, publish everywhere.<br>
<br>
Read the changelog (https://example.com/changelog) or run pnpm add @meritt/gsx-wasm.<br>
<br>
› Keep the message. Adapt the formatting.<br>
<br>
– One Rust core across all bindings<br>
– Per-platform markup one GSX document<br>
– Explicit limits and transformation reports<br>
– A surprise for Discord users<br>
<br>
Release stream 11 November, 18:00 TRT. Thanks @meritt! #release #rust</div>
<hr><p><em>Attachments · 2 images</em><br><code>media:banner</code> <code>media:preview</code></p>
</td>
</tr></tbody>
</table>

</details>

<details>
<summary>VK</summary>

<p><strong><code>vk-plain</code></strong></p>

<table>
<thead><tr><th scope="col" align="left">Output</th><th scope="col" align="left">Preview</th></tr></thead>
<tbody><tr>
<td width="50%" valign="top">
<p><strong>text</strong></p>
<p><code>gsx 0.2 is out<br>
<br>
Native projections for 19 targets, from Telegram to TikTok. Write once, publish everywhere.<br>
<br>
Read the changelog (https://example.com/changelog) or run pnpm add @meritt/gsx-wasm.<br>
<br>
› Keep the message. Adapt the formatting.<br>
<br>
– One Rust core across all bindings<br>
– Per-platform markup one GSX document<br>
– Explicit limits and transformation reports<br>
– A surprise for Discord users<br>
<br>
Release stream 11 November, 18:00 TRT. Thanks @meritt! #release #rust</code></p>
<p><strong>items</strong></p>
<p><code>[<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:banner&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;gsx 0.2: one source, native projections&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;},<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:preview&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;The same post adapted for Telegram and Discord&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;}<br>
]</code></p>
</td>
<td width="50%" valign="top">
<div>gsx 0.2 is out<br>
<br>
Native projections for 19 targets, from Telegram to TikTok. Write once, publish everywhere.<br>
<br>
Read the changelog (<a href="https://example.com/changelog">https://example.com/changelog</a>) or run pnpm add @meritt/gsx-wasm.<br>
<br>
› Keep the message. Adapt the formatting.<br>
<br>
– One Rust core across all bindings<br>
– Per-platform markup one GSX document<br>
– Explicit limits and transformation reports<br>
– A surprise for Discord users<br>
<br>
Release stream 11 November, 18:00 TRT. Thanks @meritt! #release #rust</div>
<hr><p><em>Attachments · 2 images</em><br><code>media:banner</code> <code>media:preview</code></p>
</td>
</tr></tbody>
</table>

</details>

<details>
<summary>YouTube</summary>

<p><strong><code>youtube-plain</code></strong></p>

<table>
<thead><tr><th scope="col" align="left">Output</th><th scope="col" align="left">Preview</th></tr></thead>
<tbody><tr>
<td width="50%" valign="top">
<p><strong>title</strong></p>
<p><code>gsx 0.2 is out</code></p>
<p><strong>description</strong></p>
<p><code>Native projections for 19 targets, from Telegram to TikTok. Write once, publish everywhere.<br>
<br>
Read the changelog (https://example.com/changelog) or run pnpm add @meritt/gsx-wasm.<br>
<br>
› Keep the message. Adapt the formatting.<br>
<br>
– One Rust core across all bindings<br>
– Per-platform markup one GSX document<br>
– Explicit limits and transformation reports<br>
– A surprise for Discord users<br>
<br>
Release stream 11 November, 18:00 TRT. Thanks @meritt! #release #rust</code></p>
<p><strong>items</strong></p>
<p><code>[<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:banner&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;gsx 0.2: one source, native projections&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;},<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:preview&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;The same post adapted for Telegram and Discord&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;}<br>
]</code></p>
</td>
<td width="50%" valign="top">
<p><strong>gsx 0.2 is out</strong></p>
<div>Native projections for 19 targets, from Telegram to TikTok. Write once, publish everywhere.<br>
<br>
Read the changelog (<a href="https://example.com/changelog">https://example.com/changelog</a>) or run pnpm add @meritt/gsx-wasm.<br>
<br>
› Keep the message. Adapt the formatting.<br>
<br>
– One Rust core across all bindings<br>
– Per-platform markup one GSX document<br>
– Explicit limits and transformation reports<br>
– A surprise for Discord users<br>
<br>
Release stream 11 November, 18:00 TRT. Thanks @meritt! #release #rust</div>
<hr><p><em>Attachments · 2 images</em><br><code>media:banner</code> <code>media:preview</code></p>
</td>
</tr></tbody>
</table>

</details>

<details>
<summary>TikTok</summary>

<p><strong><code>tiktok-plain</code></strong></p>

<table>
<thead><tr><th scope="col" align="left">Output</th><th scope="col" align="left">Preview</th></tr></thead>
<tbody><tr>
<td width="50%" valign="top">
<p><strong>title</strong></p>
<p><code>gsx 0.2 is out</code></p>
<p><strong>text</strong></p>
<p><code>Native projections for 19 targets, from Telegram to TikTok. Write once, publish everywhere.<br>
<br>
Read the changelog (https://example.com/changelog) or run pnpm add @meritt/gsx-wasm.<br>
<br>
› Keep the message. Adapt the formatting.<br>
<br>
– One Rust core across all bindings<br>
– Per-platform markup one GSX document<br>
– Explicit limits and transformation reports<br>
– A surprise for Discord users<br>
<br>
Release stream 11 November, 18:00 TRT. Thanks @meritt! #release #rust</code></p>
<p><strong>items</strong></p>
<p><code>[<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:banner&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;gsx 0.2: one source, native projections&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;},<br>
&nbsp;&nbsp;{<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;reference&quot;: &quot;media:preview&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;alt&quot;: &quot;The same post adapted for Telegram and Discord&quot;,<br>
&nbsp;&nbsp;&nbsp;&nbsp;&quot;spoiler&quot;: false<br>
&nbsp;&nbsp;}<br>
]</code></p>
</td>
<td width="50%" valign="top">
<p><strong>gsx 0.2 is out</strong></p>
<div>Native projections for 19 targets, from Telegram to TikTok. Write once, publish everywhere.<br>
<br>
Read the changelog (https://example.com/changelog) or run pnpm add @meritt/gsx-wasm.<br>
<br>
› Keep the message. Adapt the formatting.<br>
<br>
– One Rust core across all bindings<br>
– Per-platform markup one GSX document<br>
– Explicit limits and transformation reports<br>
– A surprise for Discord users<br>
<br>
Release stream 11 November, 18:00 TRT. Thanks @meritt! #release #rust</div>
<hr><p><em>Attachments · 2 images</em><br><code>media:banner</code> <code>media:preview</code></p>
</td>
</tr></tbody>
</table>

</details>

## Performance

Parse medians for v0.2.0 on Apple M1 Pro, macOS 27.2, rustc 1.99.1 and Node.js 26.13.0. Wasm calls include JSON encoding and decoding. Full [native](testdata/bench/baseline/gsx-615cd7b.md) and [Wasm](testdata/bench/baseline/gsx-wasm-615cd7b.md) reports include input outcomes and memory measurements.

| Case | UTF-16 units | Native, ms | Wasm in Node.js, ms | Native peak heap, MiB |
| --- | ---: | ---: | ---: | ---: |
| 91 typical posts | 112,950 | 1.52 | 3.56 | 0.2 |
| Long document, 4k | 3,850 | 0.43 | 1.24 | 0.5 |
| Long document, 32k | 32,450 | 3.54 | 9.34 | 4.0 |
| Long document, 128k | 130,900 | 14.95 | 37.36 | 16.1 |
| Prose, 128k | 131,057 | 1.99 | 3.79 | 0.9 |
| 16,383 ATX headings | 65,532 | 17.99 | 45.27 | 31.4 |
| 8,192 nested list items | 40,960 | 17.68 | 47.20 | 24.2 |
| 8,191 setext headings | 32,764 | 14.22 | 34.82 | 17.8 |
| 2,000 quotes, 31 levels deep | 128,000 | 14.72 | 33.55 | 42.6 |
| 21,844 unclosed `<?` | 65,535 | 2.43 | 7.31 | 4.6 |
| 40,000 `*a`, rejected | 120,000 | 0.59 | 0.81 | 0.7 |

Native projection medians for the 128k document: `telegram-html-rich` 10.28 ms, `facebook-plain` 4.93 ms, `x-plain` 1.99 ms. Per-target results are in the native report.

`gsx.wasm` is 356,047 bytes with `gzip -9`. Cold load is 3.52 ms; the first parse is 5.36 ms. Reproduce with `cargo xtask bench` and `cargo xtask bench --wasm`.

## Reference

- [Contracts](docs/decisions/readme.md): syntax, serialization, projections, measurements and binding API.
- [Architecture and targets](docs/architecture.md).
- [JSON Schema](schema/gsx-1.schema.json).

## Author

[Alexey Simonenko](https://simonenko.xyz)

## License

MIT. See `LICENSE`.
