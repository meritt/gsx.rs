#[allow(dead_code)]
#[path = "../tests/support/bench.rs"]
mod loader;

use std::hint::black_box;

use gsx_core::document::{Document, Media, parse};
use gsx_core::project::{Facts, Format, MediaFact, MediaKind, Overflow, Placement, Target, project};
use gsx_core::targets::{TARGETS, target};
use gungraun::{library_benchmark, library_benchmark_group, main};
use loader::Bench;

const MEDIA: [Media<'static>; 4] = [
    Media { id: "a", cover: None },
    Media { id: "b", cover: None },
    Media { id: "c", cover: None },
    Media { id: "v", cover: None },
];

struct Publication {
    document: Document,
    target: &'static Target,
    alt: String,
}

fn case(id: &str) -> String {
    let bench = Bench::load();
    bench.markdown(bench.case(id), 1)
}

fn document(id: &str) -> Document {
    parse(&case(id), &[]).expect("fitting cases are accepted")
}

fn publication(target: &'static Target) -> Publication {
    let bench = Bench::load();
    let part = bench.markdown(bench.case("long-4k"), 2);
    let markdown = format!(
        "{part}::media{{id=\"a\"}}\n\n{part}::media{{ids=\"b,c\" layout=\"collage\"}}\n\n{part}::media{{id=\"v\"}}\n"
    );
    let document = parse(&markdown, &MEDIA).expect("the publication is accepted");
    Publication { document, target, alt: "Огни гавани Амбера в сумерках, ".repeat(40) }
}

#[library_benchmark]
#[bench::typical(loader::typical())]
fn typical(markdowns: Vec<String>) {
    for markdown in &markdowns {
        drop(black_box(parse(black_box(markdown), &[])));
    }
}

#[library_benchmark]
#[benches::case(
    args = [
        "long-32k",
        "prose-128k",
        "lines-heading",
        "emphasis-delimiters",
        "label-starts",
        "list-items",
        "container-depth",
        "table-cells",
        "nested-list-items",
        "reject-lines",
    ],
    setup = case
)]
fn cases(markdown: String) {
    drop(black_box(parse(black_box(&markdown), &[])));
}

#[library_benchmark]
#[bench::tiktok_long_32k(setup = document, args = ("long-32k"))]
fn fitting(document: Document) {
    drop(black_box(project(black_box(&document), target("tiktok-plain").unwrap(), &Facts::default())));
}

#[library_benchmark]
#[benches::publication(iter = TARGETS, setup = publication)]
fn targets(publication: Publication) {
    let items = [
        MediaFact { id: "a", format: Format::Image, alt: Some("Гавань"), spoiler: false, inline: false },
        MediaFact { id: "b", format: Format::Image, alt: Some(&publication.alt), spoiler: false, inline: false },
        MediaFact { id: "c", format: Format::Image, alt: None, spoiler: true, inline: false },
        MediaFact { id: "v", format: Format::Video, alt: Some("Арден"), spoiler: false, inline: false },
    ];
    let facts = Facts {
        media: Some(MediaKind::Image),
        media_count: items.len(),
        placement: Some(Placement::Post),
        title: None,
        target_url: Some("https://www.threads.net/@corwin/post/C1a2b3"),
        max_characters: Some(500),
        characters_reserved_per_url: Some(23),
        caption_limit: Some(2048),
        description_limit: Some(1500),
        hashtag_limit: Some(30),
        spoiler_text: Some("Spoilers"),
        overflow: Overflow::Thread,
        entities_allowed: true,
        items: Some(&items),
    };
    drop(black_box(project(black_box(&publication.document), publication.target, &facts)));
}

library_benchmark_group!(name = parse_group, benchmarks = [typical, cases]);
library_benchmark_group!(name = project_group, benchmarks = [fitting, targets]);

main!(library_benchmark_groups = parse_group, project_group);
