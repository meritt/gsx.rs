#[allow(dead_code)]
#[path = "../tests/support/bench.rs"]
mod loader;

use std::hint::black_box;

use gsx_core::document::{Document, parse};
use gsx_core::project::{Facts, project};
use gsx_core::targets::target;
use gungraun::{library_benchmark, library_benchmark_group, main};
use loader::Bench;

fn case(id: &str) -> String {
    let bench = Bench::load();
    bench.markdown(bench.case(id), 1)
}

fn document(id: &str) -> Document {
    parse(&case(id), &[]).expect("fitting cases are accepted")
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

library_benchmark_group!(name = parse_group, benchmarks = [typical, cases]);
library_benchmark_group!(name = project_group, benchmarks = [fitting]);

main!(library_benchmark_groups = parse_group, project_group);
