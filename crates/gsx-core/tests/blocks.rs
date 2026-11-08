use gsx_core::document::{Layout, Media, parse};
use gsx_core::project::{
    Block, Class, Facts, Feature, FieldName, Format, Item, MediaFact, MediaKind, Projection, Slot, project,
};
use gsx_core::targets::target;

fn image(id: &str) -> MediaFact<'_> {
    MediaFact { id, format: Format::Image, alt: None, spoiler: false, inline: true }
}

fn run(name: &str, markdown: &str, items: &[MediaFact]) -> Projection {
    let media: Vec<_> = items.iter().map(|item| Media { id: item.id, cover: None }).collect();
    let document = parse(markdown, &media).unwrap();
    let facts =
        Facts { media: Some(MediaKind::Image), media_count: items.len(), items: Some(items), ..Facts::default() };
    project(&document, target(name).unwrap(), &facts)
}

fn item(id: &str) -> Item {
    Item { reference: format!("media:{id}"), alt: None, spoiler: false }
}

fn single(id: &str) -> Block {
    Block::Media { items: vec![item(id)], layout: None }
}

fn text(value: &str) -> Block {
    Block::Text(value.to_owned())
}

const NONE: [(Class, Feature, &str); 0] = [];

fn reasons(projection: &Projection) -> Vec<(Class, Feature, &'static str)> {
    projection.changes.iter().map(|change| (change.class, change.feature, change.reason)).collect()
}

#[test]
fn text_segments_carry_the_gaps_around_media() {
    let projection = run("discord-blocks", "a *b*\n\n::media{id=\"p\"}\n\nc", &[image("p")]);
    assert_eq!(projection.blocks, [text("a *b*\n\n"), single("p"), text("\n\nc")]);
    let measured = projection.measurements.iter().find(|measured| measured.field == FieldName::Text).unwrap();
    assert_eq!(measured.value, "a *b*\n\n\n\nc".chars().count());
    assert_eq!(measured.maximum, Some(2000));
    assert_eq!(reasons(&projection), NONE);
}

#[test]
fn slack_segments_use_slack_markdown() {
    let projection = run("slack-blocks", "a.b\n\n***\n\n::media{id=\"p\"}", &[image("p")]);
    assert_eq!(projection.blocks, [text("a.b\n\n---\n\n"), single("p")]);
}

#[test]
fn native_group_layouts_stay_grouped() {
    let items = [image("p"), image("q")];
    let discord = run("discord-blocks", "::media{ids=\"p,q\" layout=\"collage\"}", &items);
    assert_eq!(discord.blocks, [Block::Media { items: vec![item("p"), item("q")], layout: Some(Layout::Collage) }]);
    let slack = run("slack-blocks", "::media{ids=\"p,q\" layout=\"slideshow\"}", &items);
    assert_eq!(slack.blocks, [Block::Media { items: vec![item("p"), item("q")], layout: Some(Layout::Slideshow) }]);
}

#[test]
fn other_group_layouts_split_into_single_items() {
    let items = [image("p"), image("q")];
    let projection = run("discord-blocks", "::media{ids=\"p,q\" layout=\"slideshow\"}", &items);
    assert_eq!(projection.blocks, [single("p"), single("q")]);
    assert_eq!(reasons(&projection), [(Class::Approximated, Feature::Media, "media_group_split")]);
}

#[test]
fn unplaced_media_lead() {
    let projection = run("discord-blocks", "a\n\n::media{id=\"q\"}", &[image("p"), image("q")]);
    assert_eq!(projection.blocks, [single("p"), text("\n\na\n\n"), single("q")]);
}

#[test]
fn media_without_an_inline_variant_blocks_the_projection() {
    let video = MediaFact { format: Format::Video, ..image("v") };
    let slack = run("slack-blocks", "::media{id=\"v\"}", &[video]);
    assert_eq!(reasons(&slack), [(Class::Blocked, Feature::Media, "media_inline_unavailable")]);
    assert_eq!(slack.changes[0].code().as_deref(), Some("MEDIA_INLINE_UNAVAILABLE"));
    let discord = run("discord-blocks", "::media{id=\"p\"}", &[MediaFact { inline: false, ..image("p") }]);
    assert_eq!(reasons(&discord), [(Class::Blocked, Feature::Media, "media_inline_unavailable")]);
    assert_eq!(reasons(&run("discord-blocks", "::media{id=\"v\"}", &[video])), NONE);
    let videos = [video, MediaFact { format: Format::Video, ..image("w") }];
    let group = run("slack-blocks", "::media{ids=\"v,w\" layout=\"slideshow\"}", &videos);
    assert_eq!(reasons(&group), [(Class::Blocked, Feature::Media, "media_inline_unavailable")]);
}

#[test]
fn alt_text_is_fitted_per_item() {
    let alt = "word ".repeat(220);
    let fact = MediaFact { alt: Some(&alt), ..image("p") };
    let projection = run("discord-blocks", "::media{id=\"p\"}", &[fact]);
    let Block::Media { items, .. } = &projection.blocks[0] else { panic!("media block") };
    let fitted = items[0].alt.as_deref().unwrap();
    assert!(fitted.ends_with('…') && fitted.chars().count() <= 1024, "{fitted:?}");
    let change = &projection.changes[0];
    assert_eq!(
        (change.class, change.feature, change.reason),
        (Class::Approximated, Feature::Media, "media_alt_truncated")
    );
    assert_eq!(change.field, FieldName::Alt);
    assert_eq!(change.slot, Some(Slot { block: Some(0), item: 0 }));
    assert_eq!(change.truncated.map(|truncated| (truncated.value, truncated.maximum)), Some((1100, 1024)));
    let measured = projection.measurements.iter().find(|measured| measured.field == FieldName::Alt).unwrap();
    assert_eq!((measured.value, measured.maximum), (fitted.chars().count(), Some(1024)));
    assert_eq!((measured.block, measured.item), (Some(0), Some(0)));
    let slack = run("slack-blocks", "::media{id=\"p\"}", &[fact]);
    assert!(slack.changes.is_empty(), "slack-blocks allows 2000 code points of alt text");
}

#[test]
fn alt_measurements_carry_the_block_and_item_index() {
    let items =
        [image("p"), MediaFact { alt: Some("q alt"), ..image("q") }, MediaFact { alt: Some("r alt"), ..image("r") }];
    let projection = run("discord-blocks", "a\n\n::media{ids=\"p,q\" layout=\"collage\"}\n\n::media{id=\"r\"}", &items);
    let slots: Vec<_> = projection
        .measurements
        .iter()
        .filter(|measured| measured.field == FieldName::Alt)
        .map(|measured| (measured.block, measured.item, measured.value))
        .collect();
    assert_eq!(slots, [(Some(1), Some(1), 5), (Some(2), Some(0), 5)]);
    let text = projection.measurements.iter().find(|measured| measured.field == FieldName::Text).unwrap();
    assert_eq!((text.block, text.item), (None, None));
}

#[test]
fn discord_keeps_ten_items() {
    let ids: Vec<String> = (0..11).map(|index| format!("m{index}")).collect();
    let items: Vec<_> = ids.iter().map(|id| image(id)).collect();
    let markdown = ids.iter().map(|id| format!("::media{{id=\"{id}\"}}")).collect::<Vec<_>>().join("\n\n");
    let projection = run("discord-blocks", &markdown, &items);
    assert_eq!(projection.blocks.len(), 10);
    assert_eq!(reasons(&projection), [(Class::Omitted, Feature::Media, "media_count_exceeded")]);
}

#[test]
fn media_dropped_by_the_item_limit_leave_the_text_whole() {
    let ids: Vec<String> = (0..11).map(|index| format!("m{index}")).collect();
    let items: Vec<_> = ids.iter().map(|id| image(id)).collect();
    let projection = run("discord-blocks", "a\n\n::media{id=\"m10\"}\n\nb", &items);
    let mut expected: Vec<_> = ids[..10].iter().map(|id| single(id)).collect();
    expected.push(text("\n\na\n\nb"));
    assert_eq!(projection.blocks, expected);
    assert_eq!(reasons(&projection), [(Class::Omitted, Feature::Media, "media_count_exceeded")]);
}

#[test]
fn a_group_left_with_one_item_is_not_reported_as_split() {
    let ids: Vec<String> = (0..11).map(|index| format!("m{index}")).collect();
    let items: Vec<_> = ids.iter().map(|id| image(id)).collect();
    let projection = run("discord-blocks", "::media{ids=\"m9,m10\" layout=\"slideshow\"}", &items);
    let expected: Vec<_> = ids[..10].iter().map(|id| single(id)).collect();
    assert_eq!(projection.blocks, expected);
    assert_eq!(reasons(&projection), [(Class::Omitted, Feature::Media, "media_count_exceeded")]);
}

#[test]
fn missing_media_drop_their_anchor() {
    const MISSING: [(Class, Feature, &str); 1] = [(Class::Omitted, Feature::Media, "media_missing")];
    for name in ["discord-blocks", "slack-blocks"] {
        let middle = run(name, "a\n\n::media{id=\"x\"}\n\nb", &[]);
        assert_eq!(middle.blocks, [text("a\n\nb")]);
        assert_eq!(reasons(&middle), MISSING);
        assert_eq!(run(name, "::media{id=\"x\"}\n\na", &[]).blocks, [text("a")]);
        assert_eq!(run(name, "a\n\n::media{id=\"x\"}", &[]).blocks, [text("a")]);
        let layout = if name == "slack-blocks" { "slideshow" } else { "collage" };
        let singleton = run(name, &format!("a\n\n::media{{ids=\"p,x\" layout=\"{layout}\"}}"), &[image("p")]);
        assert_eq!(singleton.blocks, [text("a\n\n"), single("p")]);
        assert_eq!(reasons(&singleton), MISSING);
        let empty = run(name, &format!("a\n\n::media{{ids=\"x,y\" layout=\"{layout}\"}}\n\nb"), &[]);
        assert_eq!(empty.blocks, [text("a\n\nb")]);
        assert_eq!(reasons(&empty), MISSING);
        let all = run(name, "::media{id=\"x\"}\n\n::media{id=\"y\"}", &[]);
        assert_eq!(all.blocks, []);
        assert_eq!(reasons(&all), [MISSING[0], (Class::Blocked, Feature::Text, "text_empty"), MISSING[0]]);
    }
}

#[test]
fn slack_refuses_more_than_fifty_blocks() {
    let ids: Vec<String> = (0..26).map(|index| format!("m{index}")).collect();
    let items: Vec<_> = ids.iter().map(|id| image(id)).collect();
    let markdown = ids.iter().map(|id| format!("t\n\n::media{{id=\"{id}\"}}")).collect::<Vec<_>>().join("\n\n");
    let projection = run("slack-blocks", &markdown, &items);
    assert_eq!(projection.blocks.len(), 52);
    assert_eq!(reasons(&projection), [(Class::Blocked, Feature::Media, "blocks_too_many")]);
    let fewer = run("slack-blocks", &markdown[..markdown.rfind("\n\nt").unwrap()], &items[..25]);
    assert_eq!(reasons(&fewer), NONE);
}

#[test]
fn media_after_the_cut_follow_the_text() {
    let markdown = format!("{}\n\n::media{{id=\"p\"}}", "word ".repeat(500));
    let projection = run("discord-blocks", &markdown, &[image("p")]);
    let [Block::Text(kept), media] = projection.blocks.as_slice() else { panic!("{:?}", projection.blocks) };
    assert!(kept.ends_with('…') && kept.chars().count() <= 2000);
    assert_eq!(media, &single("p"));
    let reasons = reasons(&projection);
    assert!(reasons.contains(&(Class::Approximated, Feature::Text, "text_truncated")));
    assert!(reasons.contains(&(Class::Approximated, Feature::Media, "media_after_cut")));
}

#[test]
fn slack_omits_spoilers() {
    let projection = run("slack-blocks", "::media{id=\"p\"}", &[MediaFact { spoiler: true, ..image("p") }]);
    assert_eq!(projection.blocks, [single("p")]);
    assert_eq!(reasons(&projection), [(Class::Omitted, Feature::Media, "media_spoiler_omitted")]);
    let discord = run("discord-blocks", "::media{id=\"p\"}", &[MediaFact { spoiler: true, ..image("p") }]);
    assert_eq!(discord.blocks, [Block::Media { items: vec![Item { spoiler: true, ..item("p") }], layout: None }]);
}

#[test]
fn slack_reports_each_omitted_spoiler_at_its_item() {
    let items = [MediaFact { spoiler: true, ..image("p") }, image("q"), MediaFact { spoiler: true, ..image("r") }];
    let grouped = run("slack-blocks", "Look\n\n::media{ids=\"p,q,r\" layout=\"slideshow\"}", &items);
    let slots: Vec<_> = grouped.changes.iter().map(|change| (change.reason, change.slot)).collect();
    assert_eq!(
        slots,
        [
            ("media_spoiler_omitted", Some(Slot { block: Some(1), item: 0 })),
            ("media_spoiler_omitted", Some(Slot { block: Some(1), item: 2 })),
        ]
    );
    let single = run("slack-blocks", "::media{id=\"p\"}\n\n::media{id=\"q\"}\n\n::media{id=\"r\"}", &items);
    let slots: Vec<_> = single.changes.iter().map(|change| change.slot).collect();
    assert_eq!(slots, [Some(Slot { block: Some(0), item: 0 }), Some(Slot { block: Some(2), item: 0 })]);
}
