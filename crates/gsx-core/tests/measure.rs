use std::ops::Range;

use gsx_core::measure::Maximum::Fixed;
use gsx_core::measure::{Context, Fact, Facts, Field, MediaKind, Placement, Rendering, Unit, clusters, measure};
use gsx_core::targets::target;

const POST: Context = Context { placement: Placement::Post, media: None };

fn count(unit: Unit, text: &str) -> u32 {
    measure(unit, text, &Facts::default())
}

fn x(text: &str, urls: &[Range<usize>]) -> u32 {
    measure(Unit::XWeighted, text, &Facts { urls, ..Facts::default() })
}

fn mastodon(text: &str, urls: &[Range<usize>], elided: &[Range<usize>], reserved: u32) -> u32 {
    let resolved = [(Fact::CharactersReservedPerUrl, reserved)];
    measure(Unit::MastodonWeighted, text, &Facts { urls, elided, resolved: &resolved })
}

#[test]
fn segments_like_unicode_grapheme_break_test() {
    let data = include_str!("measure/GraphemeBreakTest.txt");
    let mut lines = data.lines();
    assert_eq!(lines.next(), Some(gsx_core::measure::UNICODE_VERSION));
    let mut checked = 0;
    for line in lines {
        let mut text = String::new();
        let mut expected = vec![];
        for token in line.split_whitespace() {
            match token {
                "÷" => expected.push(text.len()),
                "×" => {}
                hex => text.push(char::from_u32(u32::from_str_radix(hex, 16).unwrap()).unwrap()),
            }
        }
        expected.pop();
        let mut actual = vec![];
        let mut at = 0;
        for cluster in clusters(&text) {
            actual.push(at);
            at += cluster.len();
        }
        assert_eq!(actual, expected, "{line}");
        checked += 1;
    }
    assert!(checked > 700);
}

#[test]
fn counts_units() {
    let text = "a\u{E9}\u{4E2D}\u{1F44D}";
    assert_eq!(count(Unit::CodePoints, text), 4);
    assert_eq!(count(Unit::Utf16CodeUnits, text), 5);
    assert_eq!(count(Unit::Utf8Bytes, text), 1 + 2 + 3 + 4);
    assert_eq!(count(Unit::Graphemes, "e\u{301}\r\n\u{1F1FA}\u{1F1E6}\u{1F1FA}"), 4);
    assert_eq!(count(Unit::Graphemes, "\u{915}\u{94D}\u{937}"), 1);
}

#[test]
fn weighs_x_like_twitter_text() {
    assert_eq!(x("abc", &[]), 3);
    assert_eq!(x("\u{10FF}\u{1100}", &[]), 3);
    assert_eq!(x("\u{2014}\u{2026}\u{2033}\u{3042}", &[]), 1 + 2 + 1 + 2);
    assert_eq!(x("\u{1F44D}\u{1F3FD}", &[]), 2);
    assert_eq!(x("\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F466}", &[]), 2);
    assert_eq!(x("\u{1F1FA}\u{1F1E6}", &[]), 2);
    assert_eq!(x("#\u{FE0F}\u{20E3}", &[]), 2);
    assert_eq!(x("\u{2764}", &[]), 2);
    assert_eq!(x("\u{2764}\u{FE0F}", &[]), 2);
    assert_eq!(x("\u{1F468}\u{200D}\u{1F468}", &[]), 2 + 1 + 2);
    assert_eq!(x("see https://example.com/a now", &[4..25, 99..100]), 4 + 23 + 4);
    assert_eq!(x("a\u{1F44D}b", &[1..3, 2..3]), 1 + 2 + 1);
    assert_eq!(x("ab", &[1..1, 5..9, 0..2]), 23);
}

#[test]
fn weighs_mastodon_like_status_length_validator() {
    let text = "hi @alice@example.social see https://example.com/a";
    let domain = [9..24, 0..0];
    let url = [29..text.len(), 29..text.len()];
    let kept = count(Unit::Graphemes, "hi @alice see ");
    assert_eq!(mastodon(text, &url, &domain, 23), kept + 23);
    assert_eq!(mastodon(text, &url, &domain, 0), kept);
    assert_eq!(mastodon(text, &url, &[], 5), count(Unit::Graphemes, &text[..29]) + 5);
    assert_eq!(mastodon("x\u{301}", &[], &[], 23), 1);
    assert_eq!(mastodon("https://a.io\u{301}", &[0..12, 13..14], &[], 23), 23);
    assert_eq!(measure(Unit::MastodonWeighted, text, &Facts { urls: &url, ..Facts::default() }), kept + 15 + 23);
}

#[test]
fn weighs_threads_emoji_clusters_as_bytes() {
    let threads = |text| count(Unit::ThreadsWeighted, text);
    assert_eq!(threads("ab"), 2);
    assert_eq!(threads("\u{1F44D}"), 4);
    assert_eq!(threads("\u{1F44D}\u{1F3FD}"), 8);
    assert_eq!(threads("\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F466}"), 25);
    assert_eq!(threads("\u{2764}"), 1);
    assert_eq!(threads("\u{2764}\u{FE0F}"), 6);
    assert_eq!(threads("\u{1F468}\u{200D}\u{1F468}\u{200D}\u{1F468}"), 1);
    assert_eq!(threads("e\u{301}"), 1);
}

#[test]
fn selects_limits_by_field_and_context() {
    let telegram = target("telegram-html").unwrap();
    let maximum = |context| telegram.applicable(Field::Text, context).map(|limit| limit.maximum).collect::<Vec<_>>();
    let with = |placement| Context { placement, media: Some(MediaKind::Image) };
    assert_eq!(maximum(POST), [Fixed(4096)]);
    assert_eq!(maximum(with(Placement::Post)), [Fixed(1024)]);
    assert_eq!(maximum(with(Placement::Story)), [Fixed(2048)]);
    assert!(telegram.applicable(Field::Text, POST).all(|limit| limit.rendering == Rendering::Visible));
    let bluesky = target("bluesky-entities").unwrap();
    let units: Vec<_> = bluesky.applicable(Field::Text, POST).map(|limit| limit.unit).collect();
    assert_eq!(units, [Unit::Graphemes, Unit::Utf8Bytes]);
    let resolved = [(Fact::MaxCharacters, 11)];
    let facts = Facts { resolved: &resolved, ..Facts::default() };
    let text = target("mastodon-plain").unwrap().measure(Field::Text, POST, "hello world!", &facts).next().unwrap();
    assert_eq!((text.value, text.maximum, text.exceeds()), (12, Some(11), true));
    assert_eq!(target("youtube-plain").unwrap().applicable(Field::Title, POST).count(), 1);
    let tiktok = target("tiktok-plain").unwrap();
    assert_eq!(tiktok.applicable(Field::Title, POST).count(), 0);
    assert_eq!(tiktok.applicable(Field::Title, with(Placement::Post)).count(), 1);
}
#[test]
fn measures_youtube_titles_in_code_points() {
    let title = "é".repeat(100);
    let measured: Vec<_> =
        target("youtube-plain").unwrap().measure(Field::Title, POST, &title, &Facts::default()).collect();
    assert_eq!(measured.len(), 1);
    assert_eq!((measured[0].unit, Some(measured[0].value)), (Unit::CodePoints, measured[0].maximum));
    assert_eq!(measured[0].value, 100);
}

#[test]
fn reports_both_bluesky_limits() {
    let text = "👨‍👩‍👧‍👦".repeat(100);
    let measured: Vec<_> =
        target("bluesky-entities").unwrap().measure(Field::Text, POST, &text, &Facts::default()).collect();
    assert_eq!(
        measured.iter().map(|measurement| measurement.unit).collect::<Vec<_>>(),
        [Unit::Graphemes, Unit::Utf8Bytes]
    );
    assert_eq!((measured[0].value, measured[1].value), (100, 2500));
}
