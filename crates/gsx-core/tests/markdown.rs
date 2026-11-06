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
    let projection = run("discord-markdown", ":time[Launch]{datetime=2026-07-30T15:00:00Z}");
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
    let projection = run("slack-markdown", ":time[**Launch** `x`]{datetime=2026-07-30T15:00:00Z}");
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
fn discord_keeps_provider_spans_live() {
    assert_eq!(text(&run("discord-markdown", "a_b https://x.io/a_b @c_d")), "a\\_b https://x.io/a_b @c_d");
    assert_eq!(text(&run("slack-markdown", "a_b https://x.io")), "a\\_b https://x\\.io");
}

#[test]
fn slack_escapes_pipes_in_table_cells() {
    let projection = run("slack-markdown", "| a | b |\n| - | -: |\n| `x\\|y` | c |");
    assert_eq!(text(&projection), "| a | b |\n| :--- | ---: |\n| ``x\\|y`` | c |");
    assert_eq!(projection.changes, []);
}
