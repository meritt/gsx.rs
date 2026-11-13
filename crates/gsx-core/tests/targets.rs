mod support;

use gsx_core::forms::{blocks, html, markdown, plain};
use gsx_core::measure::{Context, Field, Maximum};
use gsx_core::project::{Facts, Feature, Form, HtmlBlocks, MediaKind, Placement, Renderer, Unit, When, project};
use gsx_core::targets::TARGETS;
use support::documents;

fn same(left: Renderer, right: Renderer) -> bool {
    std::ptr::fn_addr_eq(left.blocks, right.blocks) && std::ptr::fn_addr_eq(left.inline, right.inline)
}

#[test]
fn at_most_one_limit_applies_to_a_field_unit_placement_and_media_kind() {
    let kinds = [None, Some(MediaKind::Image), Some(MediaKind::Video), Some(MediaKind::Gif), Some(MediaKind::Audio)];
    for target in TARGETS {
        for field in [Field::Text, Field::Title, Field::Description, Field::Alt] {
            for &placement in Placement::ALL {
                for media in kinds {
                    let applicable: Vec<_> = target.applicable(field, Context { placement, media }).collect();
                    for limit in &applicable {
                        let same = applicable.iter().filter(|other| other.unit == limit.unit).count();
                        assert_eq!(
                            same,
                            1,
                            "{} applies {same} {} limits to {} in {} with {media:?}",
                            target.name,
                            limit.unit.name(),
                            field.name(),
                            placement.name()
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn a_target_declares_alt_refusal_exactly_when_it_omits_alt() {
    for target in TARGETS {
        let media = target.media();
        let narrowed = media.formats.iter().any(|format| !media.alt.contains(format))
            || target.alt_placements != Placement::ALL
            || target.form.blocks().is_some();
        assert_eq!(target.conditions(Feature::Media, When::Refused), narrowed, "{}", target.name);
    }
}

#[test]
fn a_marker_count_shared_by_fields_has_one_maximum_and_scope() {
    for target in TARGETS {
        for unit in [Unit::Hashtags, Unit::Mentions, Unit::Links] {
            let mut shared = target.limits.iter().filter(|limit| limit.unit == unit);
            let Some(first) = shared.next() else { continue };
            for limit in shared {
                assert!(
                    limit.field != first.field
                        && (limit.maximum, limit.placements, limit.media, limit.rendering)
                            == (first.maximum, first.placements, first.media, first.rendering),
                    "{} lists {} for {} and {} with different scopes",
                    target.name,
                    unit.name(),
                    first.field.name(),
                    limit.field.name()
                );
            }
        }
    }
}

#[test]
fn rewritten_text_keeps_recorded_offsets_valid() {
    for target in TARGETS {
        let name = target.name;
        let renderer = match target.form {
            Form::Plain { .. } | Form::Entities(_) => plain::RENDERER,
            Form::Html(_) => html::RENDERER,
            Form::Markdown(_) => markdown::RENDERER,
            Form::Blocks(_) => blocks::RENDERER,
        };
        assert!(same(target.renderer, renderer), "{name} renders with another form's renderer");
        if let Some(form) = target.form.html() {
            assert!(
                form.blocks != HtmlBlocks::Rich || form.quote_contains_code,
                "{name} closes quote segments before inserting captions"
            );
        }
        let capped = target
            .limits
            .iter()
            .any(|limit| matches!(limit.unit, Unit::Hashtags | Unit::Mentions) && limit.maximum != Maximum::Unlimited);
        let anchored = target.form.blocks().is_some() || target.threads();
        assert!(
            !capped || (target.form.entities().is_none() && !anchored),
            "{name} caps hashtags or mentions under styled spans or media anchors"
        );
    }
}

#[test]
fn emitted_changes_are_declared_by_the_table() {
    let documents = documents();
    let allowed = Facts { entities_allowed: true, caption_limit: Some(1024), ..Facts::default() };
    for target in TARGETS {
        for (document, facts) in
            documents.values().flat_map(|document| [(document, Facts::default()), (document, allowed)])
        {
            for change in project(document, target, &facts).changes {
                assert!(
                    target.declares(change.feature, change.class, change.reason),
                    "{} emits undeclared ({}, {}, {})",
                    target.name,
                    change.feature.name(),
                    change.class.name(),
                    change.reason
                );
            }
        }
    }
}

const CLOSED: [&str; 10] =
    ["p", "ul", "ol", "blockquote", "pre", "table", "figure", "tg-slideshow", "tg-collage", "details"];
const OPENED: [&str; 14] = [
    "p",
    "ul",
    "ol",
    "blockquote",
    "pre",
    "table",
    "figure",
    "tg-slideshow",
    "tg-collage",
    "details",
    "img",
    "video",
    "audio",
    "hr",
];

fn breaks_between_blocks(html: &str) -> Option<String> {
    let closed = CLOSED.iter().map(|tag| format!("</{tag}><br>"));
    let opened = OPENED.iter().map(|tag| format!("<br><{tag}"));
    let edges = ["/><br>", "<p><br>", "<br></p>", "<p></p>"].map(str::to_owned);
    closed.chain(opened).chain(edges).find(|pattern| html.contains(pattern.as_str()))
}

#[test]
fn no_line_break_stands_between_blocks_or_at_a_field_edge() {
    let documents = documents();
    for target in TARGETS {
        for (id, document) in &documents {
            let projection = project(document, target, &Facts::default());
            for field in &projection.fields {
                let text = &field.text;
                assert!(
                    !text.starts_with(['\n', '\r']) && !text.trim_end_matches([' ', '\t']).ends_with(['\n', '\r']),
                    "{} {id} {:?} starts or ends with a line break: {text:?}",
                    target.name,
                    field.name
                );
                if target.form.html().is_some_and(|form| form.blocks == HtmlBlocks::Rich) {
                    assert_eq!(breaks_between_blocks(text), None, "{} {id}: {text}", target.name);
                }
            }
            for block in &projection.blocks {
                if let gsx_core::project::Block::Text(text) = block {
                    assert!(
                        !text.starts_with('\n') && !text.ends_with('\n') && !text.trim().is_empty(),
                        "{} {id} text block {text:?}",
                        target.name
                    );
                }
            }
        }
    }
}
