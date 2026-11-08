use std::collections::HashSet;

use gsx_core::document::{Document, parse};
use gsx_core::project::{
    Class, Facts, Feature, FieldName, Format, MediaFact, MediaKind, Options, Overflow, Placement, Projection, Style,
    Target, Unit, project, project_with,
};
use gsx_core::targets::{TARGETS, target};

fn run(name: &str, markdown: &str, facts: &Facts) -> Projection {
    let document = parse(markdown, &[]).unwrap();
    project(&document, target(name).unwrap(), facts)
}

fn text(projection: &Projection, name: FieldName) -> &str {
    &projection.field(name).unwrap().text
}

fn fastest(document: &Document, target: &'static Target) -> std::time::Duration {
    let options = Options { search: None, maximum: Some(500) };
    (0..2)
        .map(|_| {
            let started = std::time::Instant::now();
            project_with(document, target, &Facts::default(), &options);
            started.elapsed()
        })
        .min()
        .unwrap()
}

#[test]
fn whitespace_free_runs_project_linearly() {
    for pattern in ["http", "://", "www."] {
        let times = 6_000 / pattern.len();
        let small = parse(&pattern.repeat(times), &[]).unwrap();
        let large = parse(&pattern.repeat(4 * times), &[]).unwrap();
        for target in TARGETS {
            let (short, long) = (fastest(&small, target), fastest(&large, target));
            let floor = short.max(std::time::Duration::from_millis(2));
            assert!(long < floor * 10, "{} {pattern:?}: {short:?} for {times}, {long:?} for 4x", target.name);
        }
    }
}

#[test]
fn threads_split_linearly() {
    let threaded = |document: &Document, target: &'static Target| {
        let facts = Facts { overflow: Overflow::Thread, ..Facts::default() };
        (0..2)
            .map(|_| {
                let started = std::time::Instant::now();
                assert!(project(document, target, &facts).parts.len() > 1, "{} did not split", target.name);
                started.elapsed()
            })
            .min()
            .unwrap()
    };
    for pattern in ["word ", "x", "One sentence here. "] {
        let times = 20_000 / pattern.len();
        let small = parse(&pattern.repeat(times), &[]).unwrap();
        let large = parse(&pattern.repeat(4 * times), &[]).unwrap();
        for target in TARGETS.iter().filter(|target| target.threads()) {
            let (short, long) = (threaded(&small, target), threaded(&large, target));
            let floor = short.max(std::time::Duration::from_millis(2));
            assert!(long < floor * 10, "{} {pattern:?}: {short:?} for {times}, {long:?} for 4x", target.name);
        }
    }
}

fn reasons(projection: &Projection) -> Vec<(Class, Feature, &'static str)> {
    projection.changes.iter().map(|change| (change.class, change.feature, change.reason)).collect()
}

#[test]
fn link_with_href_label_becomes_bare_url() {
    let projection = run("x-plain", "[https://example.com](https://example.com)", &Facts::default());
    assert_eq!(text(&projection, FieldName::Text), "https://example.com/");
    assert_eq!(reasons(&projection), [(Class::Approximated, Feature::Link, "link_bare_url")]);
}

#[test]
fn link_with_other_label_keeps_label_and_destination() {
    let projection = run("x-plain", "[site](https://example.com)", &Facts::default());
    assert_eq!(text(&projection, FieldName::Text), "site (https://example.com/)");
    assert_eq!(reasons(&projection), [(Class::TextualFallback, Feature::Link, "link_destination_as_text")]);
}

#[test]
fn tiktok_photo_takes_title_from_heading() {
    let facts = Facts { media: Some(MediaKind::Image), media_count: 1, ..Facts::default() };
    let projection = run("tiktok-plain", "# Trip\n\nDay one", &facts);
    assert_eq!(text(&projection, FieldName::Title), "Trip");
    assert_eq!(text(&projection, FieldName::Text), "Day one");
    assert!(reasons(&projection).contains(&(Class::Approximated, Feature::Heading, "heading_became_title")));
}

#[test]
fn tiktok_photo_moves_first_line_into_title() {
    let facts = Facts { media: Some(MediaKind::Image), media_count: 1, ..Facts::default() };
    let projection = run("tiktok-plain", "Trip\nDay one", &facts);
    assert_eq!(text(&projection, FieldName::Title), "Trip");
    assert_eq!(text(&projection, FieldName::Text), "Day one");
}

#[test]
fn tiktok_video_has_no_title() {
    let facts = Facts { media: Some(MediaKind::Video), media_count: 1, ..Facts::default() };
    let projection = run("tiktok-plain", "Trip\nDay one", &facts);
    assert!(projection.field(FieldName::Title).is_none());
    assert_eq!(text(&projection, FieldName::Text), "Trip\nDay one");
}

#[test]
fn youtube_explicit_title_over_limit_is_blocked() {
    let title = "a".repeat(101);
    let facts = Facts { title: Some(&title), ..Facts::default() };
    let projection = run("youtube-plain", "Body", &facts);
    let blocked: Vec<_> = projection.changes.iter().filter(|change| change.class == Class::Blocked).collect();
    assert_eq!(blocked.len(), 1);
    assert_eq!(blocked[0].field, FieldName::Title);
    assert_eq!(blocked[0].code().as_deref(), Some("TITLE_TOO_LONG"));
}

#[test]
fn empty_document_is_blocked_once() {
    for target in TARGETS {
        let document = parse("", &[]).unwrap();
        let projection = project(&document, target, &Facts::default());
        let blocked: Vec<_> = projection.changes.iter().filter(|change| change.class == Class::Blocked).collect();
        assert_eq!(blocked.len(), 1, "{}", target.name);
        assert_eq!(blocked[0].code().as_deref(), Some("TEXT_EMPTY"), "{}", target.name);
    }
}

#[test]
fn mastodon_limit_follows_instance_fact() {
    let facts = Facts { max_characters: Some(12), ..Facts::default() };
    let projection = run("mastodon-plain", "one two three four five", &facts);
    let measurement = projection.measurements.iter().find(|item| item.field == FieldName::Text).unwrap();
    assert_eq!(measurement.maximum, Some(12));
    assert!(measurement.value <= 12);
    assert!(text(&projection, FieldName::Text).ends_with('…'));
    assert!(reasons(&projection).contains(&(Class::Approximated, Feature::Text, "text_truncated")));
}

#[test]
fn attached_media_report_only_listed_items() {
    let markdown = "Look\n\n::media{id=\"a\"}\n\n::media{ids=\"b,c\" layout=\"collage\"}\n\n::media{id=\"c\"}";
    let items = [MediaFact { id: "c", format: Format::Image, alt: None, spoiler: false, inline: false }];
    let listed = Facts { media: Some(MediaKind::Image), media_count: 1, items: Some(&items), ..Facts::default() };
    let unlisted = Facts { items: None, ..listed };
    let attached = (Class::Approximated, "media_attached");
    let missing = (Class::Omitted, "media_missing");
    for target in TARGETS.iter().filter(|target| target.form.blocks().is_none() && target.name != "telegram-html-rich")
    {
        let media = |facts: &Facts| -> Vec<_> {
            run(target.name, markdown, facts)
                .changes
                .iter()
                .filter(|change| change.feature == Feature::Media)
                .map(|change| (change.range.unwrap().start, (change.class, change.reason)))
                .collect()
        };
        assert_eq!(media(&listed), [(6, missing), (23, missing), (23, attached), (60, attached)], "{}", target.name);
        assert_eq!(media(&unlisted), [(6, attached), (23, attached), (60, attached)], "{}", target.name);
    }
}

#[test]
fn media_after_the_cut_still_report_their_change() {
    let markdown = format!("{}\n\n::media{{id=\"c\"}}", "word ".repeat(2000));
    let items = [MediaFact { id: "c", format: Format::Image, alt: None, spoiler: false, inline: false }];
    let facts = Facts { media: Some(MediaKind::Image), media_count: 1, items: Some(&items), ..Facts::default() };
    let mut cut = vec![];
    for target in TARGETS.iter().filter(|target| target.form.blocks().is_none() && target.name != "telegram-html-rich")
    {
        let projection = run(target.name, &markdown, &facts);
        let reasons: Vec<_> = projection.changes.iter().map(|change| change.reason).collect();
        if reasons.contains(&"text_truncated") {
            cut.push(target.name);
            assert!(reasons.contains(&"media_attached"), "{}: {reasons:?}", target.name);
        }
    }
    assert!(cut.contains(&"x-plain") && cut.contains(&"telegram-html") && cut.contains(&"discord-markdown"), "{cut:?}");
}

#[test]
fn telegram_limits_count_code_points() {
    let emoji = "\u{1F600}";
    let cases = [
        ("telegram-html", Facts { media: Some(MediaKind::Image), media_count: 1, ..Facts::default() }, 1024),
        ("telegram-entities", Facts { caption_limit: Some(200), ..Facts::default() }, 200),
    ];
    for (name, facts, maximum) in cases {
        let full = run(name, &emoji.repeat(maximum), &facts);
        assert_eq!(text(&full, FieldName::Text), emoji.repeat(maximum), "{name}");
        let measurement = full.measurements.iter().find(|item| item.field == FieldName::Text).unwrap();
        assert_eq!(
            (measurement.unit, measurement.value, measurement.maximum),
            (Unit::CodePoints, maximum, Some(maximum))
        );
        let over = run(name, &emoji.repeat(maximum + 1), &facts);
        assert_eq!(text(&over, FieldName::Text), format!("{}…", emoji.repeat(maximum - 1)), "{name}");
        assert!(reasons(&over).contains(&(Class::Approximated, Feature::Text, "text_truncated")), "{name}");
    }
}

#[test]
fn placement_selects_limits() {
    let body = "word ".repeat(300);
    let image = |placement| Facts { media: Some(MediaKind::Image), media_count: 1, placement, ..Facts::default() };
    let maximum = |name, facts: &Facts| {
        let projection = run(name, &body, facts);
        projection.measurements.iter().find(|item| item.field == FieldName::Text).unwrap().maximum
    };
    assert_eq!(maximum("telegram-html", &image(None)), Some(1024));
    assert_eq!(maximum("telegram-html", &image(Some(Placement::Post))), Some(1024));
    assert_eq!(maximum("telegram-html", &image(Some(Placement::Story))), Some(2048));
    let story = run("telegram-html", &body, &image(Some(Placement::Story)));
    assert_eq!(text(&story, FieldName::Text), body.trim_end());
    assert_eq!(
        maximum("telegram-entities", &Facts { placement: Some(Placement::Post), ..Facts::default() }),
        Some(200)
    );
}

#[test]
fn list_item_continuation_is_indented() {
    let projection = run("facebook-plain", "- one\n\n  more\n- two", &Facts::default());
    assert_eq!(text(&projection, FieldName::Text), "– one\n\n  more\n– two");
}

#[test]
fn quote_prefixes_every_line() {
    let projection = run("facebook-plain", "> one\n>\n> two", &Facts::default());
    assert_eq!(text(&projection, FieldName::Text), "› one\n› \n› two");
}

#[test]
fn table_cut_keeps_row_shape() {
    let document = parse("| one two three | four | five |\n| - | - | - |\n| six | seven | eight |", &[]).unwrap();
    let options = Options { search: None, maximum: Some(14) };
    let projection = project_with(&document, target("facebook-plain").unwrap(), &Facts::default(), &options);
    assert_eq!(text(&projection, FieldName::Text), "one two… |  | ");
}

fn entities(projection: &Projection) -> Vec<(Style, u32, u32, Option<&str>)> {
    projection
        .field(FieldName::Text)
        .unwrap()
        .entities
        .iter()
        .map(|entity| (entity.style, entity.range.start, entity.range.end, entity.value.as_deref()))
        .collect()
}

#[test]
fn bluesky_facets_use_byte_offsets_and_skip_literals() {
    let markdown = "Привет [сайт](https://example.com) #тег `#code` \\#escaped www.example.org";
    let projection = run("bluesky-entities", markdown, &Facts::default());
    assert_eq!(text(&projection, FieldName::Text), "Привет сайт #тег #code #escaped www.example.org");
    assert_eq!(
        entities(&projection),
        [
            (Style::Link, 13, 21, Some("https://example.com/")),
            (Style::Tag, 22, 29, Some("тег")),
            (Style::Link, 45, 60, Some("https://www.example.org/")),
        ]
    );
    let projection = run("bluesky-entities", "[\u{200B}](https://example.com) text", &Facts::default());
    assert_eq!(entities(&projection), []);
    assert_eq!(reasons(&projection), [(Class::Omitted, Feature::Link, "link_label_invisible")]);
}

#[test]
fn threads_spoilers_skip_prefixes_and_ellipsis() {
    let projection = run("threads-entities", "> ||один\nдва|| три", &Facts::default());
    assert_eq!(text(&projection, FieldName::Text), "› один\n› два три");
    assert_eq!(entities(&projection), [(Style::Spoiler, 2, 7, None), (Style::Spoiler, 9, 12, None)]);
    let document = parse("||secret words here||", &[]).unwrap();
    let options = Options { search: None, maximum: Some(10) };
    let projection = project_with(&document, target("threads-entities").unwrap(), &Facts::default(), &options);
    assert_eq!(text(&projection, FieldName::Text), "secret…");
    assert_eq!(entities(&projection), [(Style::Spoiler, 0, 6, None)]);
}

#[test]
fn threads_caps_spoiler_ranges() {
    let markdown = (0..12).map(|index| format!("||s{index}||")).collect::<Vec<_>>().join(" ");
    let projection = run("threads-entities", &markdown, &Facts::default());
    assert_eq!(entities(&projection).len(), 10);
    let capped: Vec<_> = projection.changes.iter().filter(|change| change.reason == "spoiler_limit").collect();
    assert_eq!(capped.len(), 2);
    assert!(capped.iter().all(|change| change.class == Class::Unwrapped && change.feature == Feature::Spoiler));
}

#[test]
fn manifests_are_deterministic_and_distinct() {
    let digests: HashSet<_> = TARGETS.iter().map(|target| target.digest()).collect();
    assert_eq!(digests.len(), TARGETS.len());
    for target in TARGETS {
        assert_eq!(target.manifest(), target.manifest());
        assert_eq!(target.digest(), target.digest());
        assert_eq!(target.digest().len(), 16);
    }
}
