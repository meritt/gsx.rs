use std::collections::HashSet;

use gsx_core::document::{Document, parse};
use gsx_core::project::{
    Boundary, Class, Facts, Feature, FieldName, Format, MediaFact, MediaKind, Options, Overflow, Part, Placement,
    Projection, Style, Target, Unit, project, project_with,
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

#[test]
fn threads_of_tiny_parts_split_linearly() {
    let threaded = |document: &Document, target: &'static Target, maximum: usize| {
        let facts = Facts { overflow: Overflow::Thread, ..Facts::default() };
        let options = Options { search: None, maximum: Some(maximum) };
        (0..2)
            .map(|_| {
                let started = std::time::Instant::now();
                let parts = project_with(document, target, &facts, &options).parts.len();
                assert!(parts > 50, "{} split into {parts} parts", target.name);
                started.elapsed()
            })
            .min()
            .unwrap()
    };
    for (pattern, maximum) in [("word ", 1), ("x", 1), ("*a* https://example.com/c ", 30)] {
        let times = 1_600 / pattern.len();
        let small = parse(&pattern.repeat(times), &[]).unwrap();
        let large = parse(&pattern.repeat(8 * times), &[]).unwrap();
        for target in TARGETS.iter().filter(|target| target.threads()) {
            let (short, long) = (threaded(&small, target, maximum), threaded(&large, target, maximum));
            let floor = short.max(std::time::Duration::from_millis(2));
            assert!(long < floor * 16, "{} {pattern:?}: {short:?} for {times}, {long:?} for 8x", target.name);
        }
    }
}

#[test]
fn threads_cut_a_giant_word_linearly() {
    let facts = Facts { overflow: Overflow::Thread, ..Facts::default() };
    let document = |length: usize| parse(&format!("{} y", "x".repeat(length)), &[]).unwrap();
    let (small, large) = (document(16_000), document(128_000));
    for target in TARGETS.iter().filter(|target| target.threads()) {
        let fastest = |document: &Document| {
            (0..2)
                .map(|_| {
                    let started = std::time::Instant::now();
                    assert!(project(document, target, &facts).parts.len() > 1, "{} did not split", target.name);
                    started.elapsed()
                })
                .min()
                .unwrap()
        };
        let (short, long) = (fastest(&small), fastest(&large));
        let floor = short.max(std::time::Duration::from_millis(2));
        assert!(long < floor * 16, "{}: {short:?} for 16000, {long:?} for 128000", target.name);
    }
}

#[test]
fn threads_far_below_a_large_maximum_split_linearly() {
    let facts = Facts { overflow: Overflow::Thread, characters_reserved_per_url: Some(u32::MAX), ..Facts::default() };
    let options = Options { search: None, maximum: Some(1_000_000) };
    let target = target("mastodon-plain").unwrap();
    let document = |times: usize| parse(&"word https://example.com/c ".repeat(times), &[]).unwrap();
    let (small, large) = (document(200), document(1_600));
    let fastest = |document: &Document| {
        (0..2)
            .map(|_| {
                let started = std::time::Instant::now();
                let parts = project_with(document, target, &facts, &options).parts.len();
                assert!(parts > 50, "split into {parts} parts");
                started.elapsed()
            })
            .min()
            .unwrap()
    };
    let (short, long) = (fastest(&small), fastest(&large));
    let floor = short.max(std::time::Duration::from_millis(2));
    assert!(long < floor * 16, "{short:?} for 200, {long:?} for 1600");
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
fn youtube_counts_hashtags_across_title_and_description() {
    let tags = |prefix: &str, count: usize| (0..count).map(|i| format!("#{prefix}{i}")).collect::<Vec<_>>().join(" ");
    let hashtags = |projection: &Projection, field| {
        let measured = projection.measurements.iter().find(|m| m.field == field && m.unit == Unit::Hashtags).unwrap();
        (measured.value, measured.maximum)
    };
    let title = tags("t", 10);
    let facts = Facts { title: Some(&title), ..Facts::default() };
    let projection = run("youtube-plain", &tags("d", 55), &facts);
    assert_eq!(text(&projection, FieldName::Title), title);
    let words = (50..55).map(|i| format!("d{i}")).collect::<Vec<_>>().join(" ");
    assert_eq!(text(&projection, FieldName::Description), format!("{} {words}", tags("d", 50)));
    assert_eq!(hashtags(&projection, FieldName::Title), (10, Some(60)));
    assert_eq!(hashtags(&projection, FieldName::Description), (60, Some(60)));
    let capped: Vec<_> = projection.changes.iter().filter(|change| change.feature == Feature::Hashtag).collect();
    assert_eq!(capped.len(), 1);
    assert_eq!(capped[0].field, FieldName::Description);
    let truncated = capped[0].truncated.unwrap();
    assert_eq!((truncated.unit, truncated.maximum, truncated.value), (Unit::Hashtags, 60, 65));

    let projection = run("youtube-plain", &format!("{}\n\n{}", tags("t", 2), tags("d", 60)), &Facts::default());
    assert_eq!(text(&projection, FieldName::Title), tags("t", 2));
    assert_eq!(measure_markers(text(&projection, FieldName::Description)), 58);
    assert_eq!(hashtags(&projection, FieldName::Description), (60, Some(60)));

    let projection = run("youtube-plain", &format!("# {}\n\n{}", tags("t", 3), tags("d", 57)), &Facts::default());
    assert_eq!(text(&projection, FieldName::Title), tags("t", 3));
    assert_eq!(measure_markers(text(&projection, FieldName::Description)), 57);
    assert!(projection.changes.iter().all(|change| change.feature != Feature::Hashtag));
}

fn measure_markers(text: &str) -> usize {
    text.split_whitespace().filter(|word| word.starts_with('#')).count()
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
fn instagram_publishes_alt_for_images_only() {
    let long = "a".repeat(1001);
    let item = |id, format| MediaFact { id, format, alt: Some(&long), spoiler: false, inline: false };
    let items = [item("i", Format::Image), item("g", Format::Gif), item("v", Format::Video), item("a", Format::Audio)];
    let facts = Facts { media: Some(MediaKind::Image), media_count: 4, items: Some(&items), ..Facts::default() };
    let projection = run("instagram-plain", "Look", &facts);
    let image = projection.items[0].alt.as_deref().unwrap();
    assert_eq!((image.chars().count(), image.ends_with('…')), (1000, true));
    assert!(projection.items[1..].iter().all(|item| item.alt.is_none()));
    let alt: Vec<_> = projection
        .changes
        .iter()
        .filter(|change| change.field == FieldName::Alt)
        .map(|change| (change.class, change.reason, change.slot.unwrap().item))
        .collect();
    assert_eq!(
        alt,
        [
            (Class::Approximated, "media_alt_truncated", 0),
            (Class::Omitted, "media_alt_omitted", 1),
            (Class::Omitted, "media_alt_omitted", 2),
            (Class::Omitted, "media_alt_omitted", 3),
        ]
    );
    assert!(
        projection
            .changes
            .iter()
            .filter(|change| change.slot.is_some_and(|slot| slot.item > 0))
            .all(|change| change.truncated.is_none())
    );
    assert_eq!(projection.measurements.iter().filter(|measured| measured.field == FieldName::Alt).count(), 1);
    assert_eq!(target("instagram-plain").unwrap().media().alt, [Format::Image]);
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
fn links_are_underlined_where_underline_is_native() {
    let facts = Facts::default();
    let link = "[docs](https://example.com)";
    assert_eq!(
        text(&run("telegram-html", link, &facts), FieldName::Text),
        "<u><a href=\"https://example.com/\">docs</a></u>"
    );
    assert_eq!(text(&run("discord-markdown", link, &facts), FieldName::Text), "__[docs](https://example.com/)__");
    assert_eq!(text(&run("slack-markdown", link, &facts), FieldName::Text), "[docs](https://example.com/)");
    let outer = run("telegram-html", "++see [docs](https://example.com)++", &facts);
    assert_eq!(text(&outer, FieldName::Text), "<u>see <a href=\"https://example.com/\">docs</a></u>");
    let inner = run("discord-markdown", "[++docs++](https://example.com)", &facts);
    assert_eq!(text(&inner, FieldName::Text), "[__docs__](https://example.com/)");
    let allowed = Facts { entities_allowed: true, ..Facts::default() };
    let projection = run("telegram-entities", link, &allowed);
    assert_eq!(
        entities(&projection),
        [(Style::Underline, 0, 4, None), (Style::Link, 0, 4, Some("https://example.com/"))]
    );
    assert_eq!(reasons(&projection), []);
}

#[test]
fn entity_cap_drops_link_underlines_first() {
    let links = (0..60).map(|index| format!("[l{index}](https://example.com/{index})")).collect::<Vec<_>>();
    let markdown = format!("{} **end**", links.join(" "));
    let facts = Facts { entities_allowed: true, caption_limit: Some(1024), ..Facts::default() };
    let projection = run("telegram-entities", &markdown, &facts);
    let found = entities(&projection);
    let count = |style: Style| found.iter().filter(|entity| entity.0 == style).count();
    assert_eq!((found.len(), count(Style::Link), count(Style::Bold), count(Style::Underline)), (100, 60, 1, 39));
    assert_eq!(reasons(&projection), []);
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

fn thread(name: &str, markdown: &str) -> Projection {
    run(name, markdown, &Facts { overflow: Overflow::Thread, ..Facts::default() })
}

fn source(markdown: &str, range: gsx_core::document::Range) -> String {
    let units: Vec<u16> = markdown.encode_utf16().collect();
    String::from_utf16(&units[range.start as usize..range.end as usize]).unwrap()
}

fn boundaries(projection: &Projection) -> Vec<Option<Boundary>> {
    projection.parts.iter().map(|part| part.boundary).collect()
}

#[test]
fn thread_parts_locate_their_source() {
    let paragraph = "Ёжик в тумане 👩🏽‍💻 идёт по лесу и считает шаги до самого конца тропинки, не торопясь.";
    let cases = [
        ([paragraph; 8].join("\n\n"), Boundary::Paragraph),
        (["Каждое предложение здесь достаточно длинное."; 14].join(" "), Boundary::Sentence),
        (["слово"; 120].join(" "), Boundary::Word),
    ];
    for name in ["x-plain", "mastodon-plain", "bluesky-entities", "threads-entities"] {
        for (markdown, inner) in &cases {
            let projection = thread(name, markdown);
            assert!(projection.parts.len() > 1, "{name} did not split");
            for part in &projection.parts {
                assert_eq!(source(markdown, part.range), part.fields[0].text, "{name} {inner:?}");
            }
            let mut expected = vec![Some(*inner); projection.parts.len() - 1];
            expected.push(None);
            assert_eq!(boundaries(&projection), expected, "{name}");
        }
    }
}

#[test]
fn thread_parts_cut_a_long_word_by_graphemes() {
    let markdown = "x".repeat(700);
    let projection = thread("x-plain", &markdown);
    let ranges: Vec<_> = projection.parts.iter().map(|part| (part.range.start, part.range.end)).collect();
    assert_eq!(ranges, [(0, 280), (280, 560), (560, 700)]);
    assert_eq!(boundaries(&projection), [Some(Boundary::Grapheme), Some(Boundary::Grapheme), None]);
}

#[test]
fn thread_parts_cover_text_rendered_differently_from_its_source() {
    let markdown = [r"Звёздочка \* и решётка \# стоят в середине довольно длинного предложения без точки"; 6].join(" ");
    let projection = thread("x-plain", &markdown);
    assert!(projection.parts.len() > 1);
    for part in &projection.parts {
        let covered = source(&markdown, part.range).replace('\\', "");
        assert!(covered.contains(&part.fields[0].text), "{covered:?} misses {:?}", part.fields[0].text);
    }
}

#[test]
fn thread_with_one_part_stays_unsplit() {
    let limited = Facts { max_characters: Some(1), ..Facts::default() };
    let projection = run("x-plain", "-", &Facts { overflow: Overflow::Thread, ..limited });
    assert_eq!(projection.parts, [] as [Part; 0]);
    assert!(projection.changes.iter().all(|change| change.reason != "text_threaded"));
    assert_eq!(projection, run("x-plain", "-", &limited));
}

#[test]
fn thread_parts_trim_whitespace_at_grapheme_cuts() {
    let facts = Facts { overflow: Overflow::Thread, ..Facts::default() };
    let options = Options { search: None, maximum: Some(5) };
    let threaded = |name: &str, markdown: &str| {
        project_with(&parse(markdown, &[]).unwrap(), target(name).unwrap(), &facts, &options)
    };
    assert_eq!(threaded("x-plain", &format!("{}&", "\t".repeat(21))).parts, [] as [Part; 0]);
}

#[test]
fn thread_parts_keep_links_whole() {
    let facts = Facts { overflow: Overflow::Thread, ..Facts::default() };
    let options = Options { search: None, maximum: Some(5) };
    let threaded = |markdown: &str| {
        project_with(&parse(markdown, &[]).unwrap(), target("bluesky-entities").unwrap(), &facts, &options)
    };
    let projection = threaded("ab[aaaabbbb](https://example.com) cd");
    let texts: Vec<_> = projection.parts.iter().map(|part| part.fields[0].text.as_str()).collect();
    assert_eq!(texts, ["ab", "aaaa…", "cd"]);
    assert_eq!(boundaries(&projection), [Some(Boundary::Grapheme), Some(Boundary::Word), None]);
    let link = &projection.parts[1];
    let facets: Vec<_> = link.fields[0]
        .entities
        .iter()
        .map(|entity| (entity.style, entity.range.start, entity.range.end, entity.value.as_deref()))
        .collect();
    assert_eq!(facets, [(Style::Link, 0, 7, Some("https://example.com/"))]);
    assert_eq!((link.range.start, link.range.end), (3, 11));
    let truncated: Vec<_> = projection
        .changes
        .iter()
        .filter_map(|change| change.truncated.map(|truncated| (change.range.unwrap(), truncated)))
        .map(|(range, truncated)| {
            (range.start, range.end, truncated.unit, truncated.maximum, truncated.value, truncated.exact)
        })
        .collect();
    assert_eq!(truncated, [(7, 11, Unit::Graphemes, 5, 8, true)]);
    let whole = threaded("[aaaa bbbb](https://example.com)");
    assert_eq!(whole.parts, [] as [Part; 0]);
    assert_eq!(text(&whole, FieldName::Text), "aaaa…");
}

#[test]
fn thread_parts_cut_back_before_a_link_fit_their_maximum() {
    let facts = Facts { overflow: Overflow::Thread, ..Facts::default() };
    let options = Options { search: None, maximum: Some(7) };
    let markdown = "漢字www.example.org[https://example.com](https://example.com)";
    let projection = project_with(&parse(markdown, &[]).unwrap(), target("x-plain").unwrap(), &facts, &options);
    let texts: Vec<_> = projection.parts.iter().map(|part| part.fields[0].text.as_str()).collect();
    assert_eq!(texts, ["漢字www", ".exampl", "e.or", "g", "…"]);
    for part in &projection.parts {
        for measurement in &part.measurements {
            assert!(measurement.maximum.is_none_or(|maximum| measurement.value <= maximum), "{texts:?}");
        }
    }
}

#[test]
fn thread_parts_keep_bare_urls_whole() {
    let facts = Facts { overflow: Overflow::Thread, ..Facts::default() };
    let url = "https://example.com/abcdefghij";
    for name in ["x-plain", "mastodon-plain"] {
        let options = Options { search: None, maximum: Some(25) };
        let markdown = format!("Смотри:{url}");
        let projection = project_with(&parse(&markdown, &[]).unwrap(), target(name).unwrap(), &facts, &options);
        let texts: Vec<_> = projection.parts.iter().map(|part| part.fields[0].text.as_str()).collect();
        let holders: Vec<_> = texts.iter().filter(|text| text.contains("example")).collect();
        assert_eq!(holders.len(), 1, "{name}: {texts:?}");
        assert!(holders[0].contains(url), "{name}: {texts:?}");
        for part in &projection.parts {
            for measurement in &part.measurements {
                assert!(measurement.maximum.is_none_or(|maximum| measurement.value <= maximum), "{name}: {texts:?}");
            }
        }
    }
    let markdown = format!("Начало поста. https://example.com/{} Конец поста.", "a".repeat(600));
    let projection = run("threads-entities", &markdown, &facts);
    let texts: Vec<_> = projection.parts.iter().map(|part| part.fields[0].text.as_str()).collect();
    assert_eq!(texts, ["Начало поста.", "…", "Конец поста."]);
    assert!(reasons(&projection).contains(&(Class::Approximated, Feature::Text, "text_truncated")));
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

#[test]
fn markup_thread_fills_parts_across_collapsed_blank_lines() {
    let facts = Facts { overflow: Overflow::Thread, ..Facts::default() };
    let paragraph = "word ".repeat(180);
    let paragraph = paragraph.trim();
    let markdown = format!("{paragraph}{}{paragraph}\n\n{paragraph}", "\n".repeat(300));
    let projection = run("discord-markdown", &markdown, &facts);
    assert_eq!(projection.parts.len(), 2);
    assert_eq!(projection.parts[0].fields[0].text, format!("{paragraph}\n\n{paragraph}"));
}

#[test]
fn markup_thread_opens_with_media_alone_when_the_element_then_fits() {
    let facts = Facts { overflow: Overflow::Thread, media: Some(MediaKind::Image), ..Facts::default() };
    let code = format!("```\n{}\n```", "x".repeat(1500));
    for markdown in [code.clone(), format!("#\n\n{code}")] {
        let projection = run("telegram-html", &markdown, &facts);
        let texts: Vec<_> = projection.parts.iter().map(|part| part.fields[0].text.as_str()).collect();
        assert_eq!(texts, ["", &format!("<pre>{}</pre>", "x".repeat(1500))], "{markdown:?}");
        assert!(projection.changes.iter().all(|change| change.reason != "text_truncated"));
    }
}

#[test]
fn telegram_thread_parts_without_listed_media_take_the_text_limit() {
    let paragraph = format!("{}.", "word ".repeat(180).trim_end());
    let markdown = format!("{}\n\n::media{{id=\"photo\"}}\n\nAfter the photo.", vec![paragraph; 6].join("\n\n"));
    let document = parse(&markdown, &[]).unwrap();
    let items = [MediaFact { id: "photo", format: Format::Image, alt: None, spoiler: false, inline: false }];
    let listed = Facts {
        overflow: Overflow::Thread,
        media: Some(MediaKind::Image),
        media_count: 1,
        items: Some(&items),
        ..Facts::default()
    };
    let maximum = |part: &Part| part.measurements.iter().find(|m| m.field == FieldName::Text).unwrap().maximum;
    let projection = project(&document, target("telegram-html").unwrap(), &listed);
    let (first, rest) = projection.parts.split_first().unwrap();
    assert!(first.media.is_empty() && first.items.is_empty());
    assert_eq!(maximum(first), Some(4096));
    assert!(first.fields[0].text.chars().count() > 1024, "{}", first.fields[0].text.len());
    let carrier = rest.iter().find(|part| part.media == ["photo"]).unwrap();
    assert_eq!(maximum(carrier), Some(1024));
    assert!(rest.iter().filter(|part| part.media.is_empty()).all(|part| maximum(part) == Some(4096)));

    let unlisted = Facts { items: None, ..listed };
    let projection = project(&document, target("telegram-html").unwrap(), &unlisted);
    assert_eq!(maximum(&projection.parts[0]), Some(1024));
}

#[test]
fn markup_thread_keeps_no_media_alone_part_before_an_element_truncated_anyway() {
    let facts = Facts { overflow: Overflow::Thread, media: Some(MediaKind::Image), ..Facts::default() };
    let options = Options { search: None, maximum: Some(3) };
    let document = parse("#\n\n- abcdef ghijkl", &[]).unwrap();
    let projection = project_with(&document, target("telegram-html").unwrap(), &facts, &options);
    assert_eq!(projection.parts, [] as [Part; 0]);
    assert_eq!(text(&projection, FieldName::Text), "– …");
}
