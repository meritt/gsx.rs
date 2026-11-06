use gsx_core::document::{Media, parse};
use gsx_core::project::{Facts, FieldName, Format, MediaFact, MediaKind, Options, Projection, project, project_with};
use gsx_core::targets::target;

fn run(name: &str, markdown: &str) -> Projection {
    project(&parse(markdown, &[]).unwrap(), target(name).unwrap(), &Facts::default())
}

fn text(projection: &Projection) -> &str {
    &projection.field(FieldName::Text).unwrap().text
}

fn changes(projection: &Projection) -> Vec<(&str, &str, &str)> {
    projection.changes.iter().map(|change| (change.feature.name(), change.class.name(), change.reason)).collect()
}

#[test]
fn telegram_closes_entities_around_inline_code() {
    let telegram = run("telegram-html", "**a `b` c**");
    assert_eq!(text(&telegram), "<b>a </b><code>b</code><b> c</b>");
    assert_eq!(changes(&telegram), [("strong", "unwrapped", "entity_cannot_contain_code")]);
    let max = run("max-html", "**a `b` c**");
    assert_eq!(text(&max), "<b>a <code>b</code> c</b>");
    assert_eq!(changes(&max), []);
}

#[test]
fn telegram_drops_entities_left_empty_by_a_code_split() {
    let telegram = run("telegram-html", "[`b`](https://example.com)");
    assert_eq!(text(&telegram), "<code>b</code>");
    assert_eq!(changes(&telegram), [("link", "unwrapped", "entity_cannot_contain_code")]);
}

#[test]
fn telegram_quotes_around_preformatted_blocks() {
    let telegram = run("telegram-html", "> a\n>\n> ```\n> x\n> ```\n>\n> b");
    assert_eq!(text(&telegram), "<blockquote>a</blockquote>\n\n<pre>x</pre>\n\n<blockquote>b</blockquote>");
    assert_eq!(changes(&telegram), [("blockquote", "unwrapped", "code_block_cannot_be_quoted")]);
    let max = run("max-html", "> a\n>\n> ```\n> x\n> ```\n>\n> b");
    assert_eq!(text(&max), "<blockquote>a\n\n<pre>x</pre>\n\nb</blockquote>");
}

#[test]
fn telegram_flattens_nested_quotes() {
    let telegram = run("telegram-html", "> a\n>\n> > b");
    assert_eq!(text(&telegram), "<blockquote>a\n\nb</blockquote>");
    assert_eq!(changes(&telegram), [("blockquote", "unwrapped", "blockquote_cannot_nest")]);
    assert_eq!(text(&run("max-html", "> a\n>\n> > b")), "<blockquote>a\n\n<blockquote>b</blockquote></blockquote>");
}

#[test]
fn lost_code_info_is_reported() {
    let telegram = run("telegram-html", "```js title\nx\n```");
    assert_eq!(text(&telegram), "<pre><code class=\"language-js\">x</code></pre>");
    assert_eq!(changes(&telegram), [("code_block", "omitted", "code_block_info_omitted")]);
    assert_eq!(changes(&run("telegram-html", "```js\nx\n```")), []);
    let max = run("max-html", "```js\nx\n```");
    assert_eq!(text(&max), "<pre>x</pre>");
    assert_eq!(changes(&max), [("code_block", "omitted", "code_block_info_omitted")]);
}

#[test]
fn list_continuation_skips_preformatted_lines() {
    let telegram = run("telegram-html", "- a\n\n  ```\n  x\n  y\n  ```\n\n  b");
    assert_eq!(text(&telegram), "• a\n\n  <pre>x\ny</pre>\n\n  b");
}

#[test]
fn rich_blocks_keep_native_structure() {
    let rich = run("telegram-html-rich", "# t\npara\n\n- [x] a\n\n| h | i |\n| :- | -: |\n| 1 | 2 |");
    assert_eq!(
        text(&rich),
        "<p><b>t</b><br>para</p><br><br><ul><li><input type=\"checkbox\" checked><p>a</p></li></ul><br><br>\
         <table><tr><th align=\"left\">h</th><th align=\"right\">i</th></tr>\
         <tr><td align=\"left\">1</td><td align=\"right\">2</td></tr></table>"
    );
    assert_eq!(changes(&rich), [("heading", "textual_fallback", "heading_as_bold")]);
}

#[test]
fn rich_measures_breaks_as_one_code_point() {
    let rich = run("telegram-html-rich", "a &amp; b\\\nc");
    assert_eq!(text(&rich), "<p>a &amp; b<br>c</p>");
    assert_eq!(rich.measurements[0].value, 7);
}

fn blocked(projection: &Projection) -> Vec<String> {
    projection.changes.iter().filter_map(gsx_core::project::Change::code).collect()
}

#[test]
fn rich_blocks_over_the_limit_are_blocked() {
    assert_eq!(blocked(&run("telegram-html-rich", &"a\n\n***\n\n".repeat(250))), [] as [String; 0]);
    let over = run("telegram-html-rich", &format!("{}a", "a\n\n***\n\n".repeat(250)));
    assert_eq!(blocked(&over), ["BLOCK_LIMIT_EXCEEDED"]);
}

#[test]
fn rich_nesting_over_the_limit_is_blocked() {
    let nested =
        |depth: usize| (0..depth).flat_map(|level| ["  ".repeat(level), "- a\n".to_owned()]).collect::<String>();
    assert_eq!(blocked(&run("telegram-html-rich", &nested(7).replace("- a\n", "- **a**\n"))), [] as [String; 0]);
    assert_eq!(blocked(&run("telegram-html-rich", &nested(8))), ["NESTING_LIMIT_EXCEEDED"]);
}

#[test]
fn rich_columns_over_the_limit_are_blocked() {
    let table = |columns: usize| format!("{}|\n{}|\n", "| a ".repeat(columns), "| - ".repeat(columns));
    assert_eq!(blocked(&run("telegram-html-rich", &table(20))), [] as [String; 0]);
    assert_eq!(blocked(&run("telegram-html-rich", &table(21))), ["TABLE_COLUMNS_EXCEEDED"]);
}

#[test]
fn rich_markup_without_visible_text_is_empty() {
    assert_eq!(blocked(&run("telegram-html-rich", "***")), ["TEXT_EMPTY"]);
}

#[test]
fn telegram_captions_use_the_media_limit() {
    let document = parse(&"word ".repeat(300), &[]).unwrap();
    let facts = Facts { media: Some(MediaKind::Image), media_count: 1, ..Facts::default() };
    let projection = project(&document, target("telegram-html").unwrap(), &facts);
    assert_eq!(projection.measurements[0].maximum, Some(1024));
    assert!(projection.measurements[0].value <= 1024);
    assert!(text(&projection).ends_with('…'));
}

fn image<'a>(id: &'a str, alt: Option<&'a str>) -> MediaFact<'a> {
    MediaFact { id, format: Format::Image, alt, spoiler: false, inline: true }
}

fn rich(markdown: &str, items: &[MediaFact]) -> Projection {
    let media: Vec<_> = items.iter().map(|item| Media { id: item.id, cover: None }).collect();
    let document = parse(markdown, &media).unwrap();
    let facts = Facts { media: Some(MediaKind::Image), media_count: items.len(), items, ..Facts::default() };
    project(&document, target("telegram-html-rich").unwrap(), &facts)
}

#[test]
fn rich_places_media_inline_with_unreferenced_items_first() {
    let items = [
        image("p", None),
        MediaFact { format: Format::Video, spoiler: true, ..image("v", None) },
        MediaFact { format: Format::Audio, spoiler: true, ..image("a", None) },
        MediaFact { format: Format::Gif, ..image("g", None) },
    ];
    let projection = rich("a\n\n::media{id=\"v\"}\n\nb\n\n::media{id=\"a\"}", &items);
    assert_eq!(
        text(&projection),
        "<img src=\"media:p\"/><br><video src=\"media:g\"></video><br><p>a</p><br><br>\
         <video src=\"media:v\" tg-spoiler></video><br><br><p>b</p><br><br><audio src=\"media:a\"></audio>"
    );
    assert_eq!(changes(&projection), []);
    assert_eq!(text(&rich("", &[image("p", None)])), "<img src=\"media:p\"/>");
    assert_eq!(blocked(&rich("", &[image("p", None)])), [] as [String; 0]);
}

#[test]
fn rich_groups_supported_kinds_and_splits_the_rest() {
    let items = [image("p", None), MediaFact { format: Format::Video, ..image("v", None) }];
    let collage = rich("::media{ids=\"p,v\" layout=\"collage\"}", &items);
    assert_eq!(text(&collage), "<tg-collage><img src=\"media:p\"/><video src=\"media:v\"></video></tg-collage>");
    assert_eq!(changes(&collage), []);
    let slideshow = rich("::media{ids=\"p,v\" layout=\"slideshow\"}", &items);
    assert_eq!(text(&slideshow), "<img src=\"media:p\"/><br><video src=\"media:v\"></video>");
    assert_eq!(changes(&slideshow), [("media", "approximated", "media_group_split")]);
}

#[test]
fn rich_media_count_beyond_fifty_is_omitted() {
    let ids: Vec<String> = (0..51).map(|index| format!("m{index}")).collect();
    let items: Vec<_> = ids.iter().map(|id| image(id, None)).collect();
    let projection = rich("a", &items);
    assert_eq!(changes(&projection), [("media", "omitted", "media_count_exceeded")]);
    assert!(!text(&projection).contains("media:m50"));
    assert!(text(&projection).contains("media:m49"));
}

#[test]
fn rich_missing_media_drop_their_anchor() {
    const MISSING: (&str, &str, &str) = ("media", "omitted", "media_missing");
    let middle = rich("a\n\n::media{id=\"x\"}\n\nb", &[]);
    assert_eq!(text(&middle), "<p>a<br><br>b</p>");
    assert_eq!(changes(&middle), [MISSING]);
    assert_eq!(text(&rich("::media{id=\"x\"}\n\na", &[])), "<p>a</p>");
    assert_eq!(text(&rich("a\n\n::media{id=\"x\"}", &[])), "<p>a</p>");
    let singleton = rich("::media{ids=\"p,x\" layout=\"collage\"}", &[image("p", None)]);
    assert_eq!(text(&singleton), "<img src=\"media:p\"/>");
    assert_eq!(changes(&singleton), [MISSING]);
    let empty = rich("a\n\n::media{ids=\"x,y\" layout=\"collage\"}\n\nb", &[]);
    assert_eq!(text(&empty), "<p>a<br><br>b</p>");
    assert_eq!(changes(&empty), [MISSING]);
    let all = rich("::media{id=\"x\"}\n\n::media{id=\"y\"}", &[]);
    assert_eq!(text(&all), "");
    assert_eq!(blocked(&all), ["TEXT_EMPTY"]);
    assert_eq!(changes(&all).iter().filter(|change| **change == MISSING).count(), 2);
}

#[test]
fn rich_alts_become_figcaptions() {
    let items = [image("p", Some("a < b")), image("q", Some("one\ntwo")), image("r", None), image("s", Some("s"))];
    let single = rich("::media{id=\"p\"}", &items[..1]);
    assert_eq!(text(&single), "<figure><img src=\"media:p\"/><figcaption>a &lt; b</figcaption></figure>");
    let group = rich("::media{ids=\"q,r,s\" layout=\"collage\"}", &items[1..]);
    assert_eq!(
        text(&group),
        "<tg-collage><img src=\"media:q\"/><img src=\"media:r\"/><img src=\"media:s\"/>\
         <figcaption>1. one<br>two<br>3. s</figcaption></tg-collage>"
    );
    assert_eq!(group.measurements[0].value, "1. one\ntwo\n3. s".chars().count());
}

#[test]
fn rich_alts_share_the_text_budget() {
    let body = "x".repeat(32_768 - 6 - 7 - 1);
    let items = [image("p", Some("abc def")), image("q", Some("long alt text")), image("r", Some("z"))];
    let projection =
        rich(&format!("{body}\n\n::media{{id=\"p\"}}\n\n::media{{id=\"q\"}}\n\n::media{{id=\"r\"}}"), &items);
    assert_eq!(projection.measurements[0].value, 32_768);
    assert!(text(&projection).contains("<figcaption>abc def</figcaption>"));
    assert_eq!(
        changes(&projection),
        [("media", "approximated", "media_alt_truncated"), ("media", "omitted", "media_alt_omitted")]
    );
    let truncated = projection.changes[0].truncated.unwrap();
    assert_eq!((truncated.maximum, truncated.value), (1, 13));
    let alts: Vec<_> = projection.items.iter().map(|item| item.alt.as_deref()).collect();
    assert_eq!(alts, [Some("abc def"), Some("…"), None]);
}

#[test]
fn rich_items_carry_the_alt_left_out_of_the_text_budget() {
    let body = format!("Ready.\n\n{}\n\n::media{{id=\"a\"}}", "x".repeat(32_760));
    let projection = rich(&body, &[image("a", Some("Described"))]);
    assert!(changes(&projection).contains(&("media", "omitted", "media_alt_omitted")));
    assert!(!text(&projection).contains("figcaption"));
    assert_eq!(projection.items[0].alt, None);
}

#[test]
fn fitting_counts_visible_text_and_keeps_markup_closed() {
    let document = parse("**bold words here** and `code & more`", &[]).unwrap();
    let options = Options { maximum: Some(12), ..Options::default() };
    let projection = project_with(&document, target("telegram-html").unwrap(), &Facts::default(), &options);
    assert_eq!(text(&projection), "<b>bold words…</b>");
    assert_eq!(projection.measurements[0].value, 11);
}
