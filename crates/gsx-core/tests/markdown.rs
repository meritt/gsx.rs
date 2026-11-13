mod support;

use gsx_core::document::parse;
use gsx_core::project::{Class, Facts, Feature, FieldName, Projection, project};
use gsx_core::targets::target;
use support::any;

fn run(name: &str, markdown: &str) -> Projection {
    project(&parse(markdown, &[]).unwrap(), target(name).unwrap(), &Facts::default())
}

fn reasons(projection: &Projection) -> Vec<(Class, Feature, &'static str)> {
    projection.changes.iter().map(|change| (change.class, change.feature, change.reason)).collect()
}

fn text(projection: &Projection) -> &str {
    &projection.field(FieldName::Text).unwrap().text
}

#[test]
fn discord_reports_the_datetime_label() {
    let projection = run("discord-markdown", "::time[Launch]{datetime=2026-07-30T15:00:00Z}");
    assert_eq!(text(&projection), "<t:1785423600:f>");
    assert_eq!(reasons(&projection), [(Class::Approximated, Feature::Datetime, "datetime_label_omitted")]);
}

#[test]
fn discord_unwraps_nested_quotes() {
    let projection = run("discord-markdown", "> a\n>\n> > b");
    assert_eq!(text(&projection), "> a\n> \n> b");
    assert_eq!(reasons(&projection), [(Class::Unwrapped, Feature::Blockquote, "blockquote_nesting_unwrapped")]);
}

#[test]
fn truncated_literal_stays_escaped() {
    let document = parse("[@a_b @c_d @e_f @g_h](ftp://x)", &[]).unwrap();
    assert!(
        any(document.children(), &|node| node.provenance == gsx_core::document::Provenance::Literal),
        "a refused link is literal text"
    );
    let discord = target("discord-markdown").unwrap();
    assert_eq!(text(&project(&document, discord, &Facts::default())), "\\[@a\\_b @c\\_d @e\\_f @g\\_h\\]\\(ftp://x\\)");
    let options = gsx_core::project::Options { search: None, maximum: Some(14) };
    let projection = gsx_core::project::project_with(&document, discord, &Facts::default(), &options);
    assert_eq!(text(&projection), "\\[@a\\_b @c\\_d…");
}

#[test]
fn slack_flattens_datetime_label_and_reports_its_constructs() {
    let projection = run("slack-markdown", "::time[**Launch** `x`]{datetime=2026-07-30T15:00:00Z}");
    assert_eq!(text(&projection), "<!date^1785423600^{date_short} {time}|Launch x>");
    assert_eq!(
        reasons(&projection),
        [
            (Class::Unwrapped, Feature::Strong, "strong_unwrapped"),
            (Class::Unwrapped, Feature::InlineCode, "inline_code_unwrapped")
        ]
    );
}

#[test]
fn slack_datetime_label_with_a_line_break_falls_back_to_text() {
    let cases = [
        (
            "Meeting ::time[today\n\\`\\`\\`]{unix=\"1794139200\"} **below**\n\n- first",
            "Meeting today\n\\`\\`\\` **below**\n\n- first",
        ),
        ("::time[today\n\\- free pizza]{unix=\"1794139200\"} ok", "today\n\\- free pizza ok"),
        ("::time[a\\\nb]{unix=\"1794139200\"}", "a\nb"),
        ("::time[a\r\n\\+ b]{unix=\"1794139200\"}", "a\n\\+ b"),
    ];
    for name in ["slack-markdown", "slack-blocks"] {
        for (markdown, expected) in cases {
            let projection = run(name, markdown);
            assert_eq!(text(&projection), expected, "{name}: {markdown:?}");
            assert_eq!(
                reasons(&projection),
                [(Class::TextualFallback, Feature::Datetime, "datetime_label_as_text")],
                "{name}: {markdown:?}"
            );
        }
    }
}

#[test]
fn code_info_loss_is_reported() {
    let projection = run("slack-markdown", "```rust title=\"a\"\nfn a() {}\n```");
    assert_eq!(text(&projection), "```rust\nfn a() {}\n```");
    assert_eq!(reasons(&projection), [(Class::Omitted, Feature::CodeBlock, "code_block_info_omitted")]);
}

#[test]
fn line_starts_are_neutralised() {
    assert_eq!(text(&run("discord-markdown", "\\# a\n1\\. b\n\\#")), "\\# a\n1\\. b\n\\#");
    assert_eq!(text(&run("slack-markdown", "a\\\n\\=\\=")), "a\n\\==");
}

#[test]
fn provider_spans_stay_live() {
    assert_eq!(text(&run("discord-markdown", "a_b https://x.io/a_b @c_d")), "a\\_b https://x.io/a_b @c_d");
    assert_eq!(text(&run("slack-markdown", "a_b https://x.io/a_b. c.d")), "a\\_b https://x.io/a_b. c.d");
}

#[test]
fn provider_spans_do_not_carry_syntax_out_of_their_construct() {
    let cases = [
        ("slack-markdown", "[a https://a.b/&#93;](https://c.d/)", "[a https://a.b/\\]](https://c.d/)"),
        ("slack-markdown", "[see https://a.b/c_d @e_f](https://c.d/)", "[see https://a.b/c\\_d @e\\_f](https://c.d/)"),
        ("discord-markdown", "[x https://a.b/&#93;&#91;y](https://c.d/)", "__[x https://a.b/\\]\\[y](https://c.d/)__"),
        ("discord-markdown", "https://`\n\nb https://`", "https://\\`\n\nb https://\\`"),
        ("discord-markdown", "https://_ a b https://_", "https://\\_ a b https://\\_"),
        ("discord-markdown", "x_y@a.bc z@a_.bc", "x_y@a.bc z@a\\_.bc"),
        ("discord-markdown", "@\\_a\\_ #b_ $\\_c d@e_f.gh", "@\\_a\\_ #b\\_ $\\_c d@e_f.gh"),
        ("slack-markdown", "https://a.b/c[d]`e`", "https://a.b/c\\[d\\]\\`e\\`"),
    ];
    for (name, markdown, expected) in cases {
        assert_eq!(text(&run(name, markdown)), expected, "{name}: {markdown:?}");
    }
}

#[test]
fn generated_link_destinations_close_the_bidi_controls_of_their_label() {
    let cases = [
        (
            "discord-markdown",
            "[https://good.example/\u{202E}](https://evil.example/)",
            "https://good.example/\u{202E}\u{202C} (https://evil.example/)",
        ),
        (
            "x-plain",
            "[a\u{2067}b\u{202B}c](https://e.example/)",
            "a\u{2067}b\u{202B}c\u{202C}\u{2069} (https://e.example/)",
        ),
        ("facebook-plain", "[a\u{202E}b\u{202C}c](https://e.example/)", "a\u{202E}b\u{202C}c (https://e.example/)"),
        (
            "mastodon-plain",
            "[a\u{2066}b\u{202A}c\u{2069}d](https://e.example/)",
            "a\u{2066}b\u{202A}c\u{2069}d (https://e.example/)",
        ),
        ("vk-plain", "[a\u{2068}b\u{202C}c](https://e.example/)", "a\u{2068}b\u{202C}c\u{2069} (https://e.example/)"),
        ("linkedin-plain", "[a\u{202D}b\\\nc](https://e.example/)", "a\u{202D}b\nc \\(https://e.example/\\)"),
        (
            "slack-markdown",
            "::time[at [b\u{202E}](https://c.example/)]{unix=\"1794139200\"}",
            "<!date^1794139200^{date_short} {time}|at b\u{202E}\u{202C} (https://c.example/)>",
        ),
    ];
    for (name, markdown, expected) in cases {
        assert_eq!(text(&run(name, markdown)), expected, "{name}: {markdown:?}");
    }
}

#[test]
fn slack_escapes_pipes_in_table_cells() {
    let projection = run("slack-markdown", "| a | b |\n| - | -: |\n| `x\\|y` | c |");
    assert_eq!(text(&projection), "| a | b |\n| :--- | ---: |\n| `x\\|y` | c |");
    assert_eq!(projection.changes, []);
}
