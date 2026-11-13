#![allow(unsafe_code)]

#[allow(dead_code)]
#[path = "../tests/support/bench.rs"]
mod bench;

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicUsize, Ordering::Relaxed};
use std::time::{Duration, Instant};
use std::{env, fs};

use bench::{Bench, Case, typical, units};
use criterion::{Criterion, SamplingMode};
use gsx_core::document::parse;
use gsx_core::project::{Facts, MediaKind, Overflow, Target, project};
use gsx_core::serialize::serialize;
use gsx_core::targets::{TARGETS, target};
use serde_json::{Map, Value, json};

const GROUPS: [&str; 4] = ["long", "budget", "unbounded", "reject"];

struct Projected {
    id: String,
    target: &'static Target,
    facts: Facts<'static>,
}

fn projected() -> Vec<Projected> {
    let mut all: Vec<Projected> = TARGETS
        .iter()
        .map(|&target| Projected { id: target.name.to_owned(), target, facts: Facts::default() })
        .collect();
    all.push(Projected {
        id: "tiktok-plain-photo".to_owned(),
        target: target("tiktok-plain").expect("tiktok-plain is a target"),
        facts: Facts { media: Some(MediaKind::Image), media_count: 1, ..Facts::default() },
    });
    all.extend(TARGETS.iter().filter(|target| target.threads()).map(|&target| Projected {
        id: format!("{}-thread", target.name),
        target,
        facts: Facts { overflow: Overflow::Thread, ..Facts::default() },
    }));
    all
}

fn projections(home: &Path, case: &Case, markdown: &str, projected: &[Projected]) -> Value {
    let document = parse(markdown, &[]).expect("long cases are accepted");
    let group = format!("project-{}", case.id);
    let entries = projected.iter().map(|projected| {
        let mut stats = stats(home, &group, &projected.id);
        let changes = project(&document, projected.target, &projected.facts).changes;
        stats["fitted"] = changes.iter().any(|change| change.truncated.is_some()).into();
        (projected.id.clone(), stats)
    });
    Value::Object(entries.collect::<Map<_, _>>())
}

struct Counting;

static COUNTING: AtomicBool = AtomicBool::new(false);
static CURRENT: AtomicIsize = AtomicIsize::new(0);
static PEAK: AtomicIsize = AtomicIsize::new(0);
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn grow(size: usize) {
    if COUNTING.load(Relaxed) {
        let now = CURRENT.fetch_add(size as isize, Relaxed) + size as isize;
        PEAK.fetch_max(now, Relaxed);
        ALLOCATIONS.fetch_add(1, Relaxed);
    }
}

fn shrink(size: usize) {
    if COUNTING.load(Relaxed) {
        CURRENT.fetch_sub(size as isize, Relaxed);
    }
}

// SAFETY: every method forwards to `System` under the same contract and only updates counters.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller upholds `GlobalAlloc::alloc`'s contract for `layout`.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            grow(layout.size());
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller upholds `GlobalAlloc::alloc_zeroed`'s contract for `layout`.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            grow(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: `pointer` was returned by this allocator, which is `System`, for `layout`.
        unsafe { System.dealloc(pointer, layout) };
        shrink(layout.size());
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        // SAFETY: the caller upholds `GlobalAlloc::realloc`'s contract for `pointer`, `layout` and `size`.
        let moved = unsafe { System.realloc(pointer, layout, size) };
        if !moved.is_null() {
            grow(size);
            shrink(layout.size());
        }
        moved
    }
}

fn outcome(markdown: &str) -> &'static str {
    match parse(markdown, &[]) {
        Ok(_) => "accepted",
        Err(error) => error.code.as_str(),
    }
}

fn heap(markdowns: &[&str]) -> Value {
    PEAK.store(CURRENT.load(Relaxed), Relaxed);
    ALLOCATIONS.store(0, Relaxed);
    let base = CURRENT.load(Relaxed);
    COUNTING.store(true, Relaxed);
    for markdown in markdowns {
        drop(black_box(parse(black_box(markdown), &[])));
    }
    COUNTING.store(false, Relaxed);
    let peak = PEAK.load(Relaxed) - base;
    json!({ "deltaKiB": round(peak as f64 / 1024.0), "allocations": ALLOCATIONS.load(Relaxed) })
}

fn fastest(markdown: &str) -> f64 {
    let started = Instant::now();
    let mut best = f64::INFINITY;
    let mut runs = 0;
    while runs < 3 || (started.elapsed() < Duration::from_millis(300) && runs < 50) {
        let start = Instant::now();
        drop(black_box(parse(black_box(markdown), &[])));
        best = best.min(start.elapsed().as_secs_f64() * 1e3);
        runs += 1;
    }
    best
}

fn scaling(bench: &Bench, case: &Case, markdown: &str) -> Value {
    let half = bench.markdown(case, 2);
    if units(&half) == units(markdown) {
        return Value::Null;
    }
    let (full_ms, half_ms) = (fastest(markdown), fastest(&half));
    let exponent = (outcome(&half) == outcome(markdown))
        .then(|| (full_ms / half_ms).ln() / (units(markdown) as f64 / units(&half) as f64).ln());
    json!({
        "halfUnits": units(&half),
        "halfOutcome": outcome(&half),
        "fullMs": round(full_ms),
        "halfMs": round(half_ms),
        "exponent": exponent.map(|exponent| (exponent * 100.0).round() / 100.0),
    })
}

fn round(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn stats(home: &Path, group: &str, id: &str) -> Value {
    let path = home.join(group).join(id).join("new/sample.json");
    let sample: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    let iters = sample["iters"].as_array().unwrap();
    let mut times: Vec<f64> = sample["times"]
        .as_array()
        .unwrap()
        .iter()
        .zip(iters)
        .map(|(time, iters)| time.as_f64().unwrap() / iters.as_f64().unwrap() / 1e6)
        .collect();
    times.sort_by(f64::total_cmp);
    let at = |quantile: f64| times[((quantile * times.len() as f64) as usize).min(times.len() - 1)];
    json!({ "median": round(at(0.5)), "p95": round(at(0.95)), "min": round(times[0]), "samples": times.len() })
}

fn run(program: &str, args: &[&str]) -> String {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .unwrap_or_default()
}

fn cpu() -> String {
    if cfg!(target_os = "macos") {
        return run("sysctl", &["-n", "machdep.cpu.brand_string"]);
    }
    fs::read_to_string("/proc/cpuinfo")
        .unwrap_or_default()
        .lines()
        .find_map(|line| {
            line.strip_prefix("model name")
                .and_then(|rest| rest.split_once(':'))
                .map(|(_, name)| name.trim().to_owned())
        })
        .unwrap_or_default()
}

fn report(out: &str, bench: &Bench, typical: &[String], projected: &[Projected]) {
    let home =
        PathBuf::from(env::var("CRITERION_HOME").expect("CRITERION_HOME must name the Criterion output directory"));
    let typical: Vec<&str> = typical.iter().map(String::as_str).collect();
    let mut results = vec![json!({
        "id": "typical",
        "group": "typical",
        "documents": typical.len(),
        "units": typical.iter().map(|markdown| units(markdown)).sum::<usize>(),
        "outcome": typical.iter().map(|markdown| outcome(markdown)).find(|outcome| *outcome != "accepted").unwrap_or("accepted"),
        "parse": stats(&home, "typical", "typical"),
        "memory": heap(&typical),
    })];
    for group in GROUPS {
        for case in bench.cases.iter().filter(|case| case.group == group) {
            eprintln!("report: {}", case.id);
            let markdown = bench.markdown(case, 1);
            let mut memory = heap(&[&markdown]);
            memory["outcome"] = outcome(&markdown).into();
            let mut result = json!({
                "id": case.id,
                "group": case.group,
                "documents": 1,
                "units": units(&markdown),
                "outcome": outcome(&markdown),
                "parse": stats(&home, group, &case.id),
                "memory": memory,
                "scaling": scaling(bench, case, &markdown),
            });
            if group == "long" {
                result["projections"] = projections(&home, case, &markdown, projected);
            }
            results.push(result);
        }
    }
    let report = json!({
        "reference": {
            "implementation": "gsx-native",
            "commit": run("git", &["rev-parse", "HEAD"]),
            "dirty": !run("git", &["status", "--porcelain", "--untracked-files=no"]).is_empty(),
        },
        "environment": {
            "rustc": run("rustc", &["--version"]),
            "platform": format!("{} {} {}", env::consts::OS, run("uname", &["-r"]), env::consts::ARCH),
            "cpu": cpu(),
            "date": run("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"]),
        },
        "memory": "peak heap growth of one parse, counting allocator",
        "targets": projected.iter().map(|projected| projected.id.as_str()).collect::<Vec<_>>(),
        "results": results,
    });
    fs::write(out, format!("{}\n", serde_json::to_string_pretty(&report).unwrap())).unwrap();
    eprintln!("report: {out}");
}

fn main() {
    let bench = Bench::load();
    let typical = typical();
    let mut criterion = Criterion::default()
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(3))
        .configure_from_args();

    let mut group = criterion.benchmark_group("typical");
    group.bench_function("typical", |bencher| {
        bencher.iter(|| {
            for markdown in &typical {
                drop(black_box(parse(black_box(markdown), &[])));
            }
        });
    });
    group.finish();

    for name in GROUPS {
        let mut group = criterion.benchmark_group(name);
        if name == "unbounded" {
            group.sample_size(10).sampling_mode(SamplingMode::Flat).warm_up_time(Duration::from_millis(100));
        }
        for case in bench.cases.iter().filter(|case| case.group == name) {
            let markdown = bench.markdown(case, 1);
            group.bench_function(case.id.as_str(), |bencher| bencher.iter(|| parse(black_box(&markdown), &[])));
        }
        group.finish();
    }

    let projected = projected();
    for case in bench.cases.iter().filter(|case| case.group == "long") {
        let document = parse(&bench.markdown(case, 1), &[]).expect("long cases are accepted");
        let mut group = criterion.benchmark_group(format!("project-{}", case.id));
        group.sample_size(20).warm_up_time(Duration::from_millis(200));
        for projected in &projected {
            group.bench_function(projected.id.as_str(), |bencher| {
                bencher.iter(|| project(black_box(&document), projected.target, &projected.facts));
            });
        }
        group.finish();
    }

    let mut group = criterion.benchmark_group("serialize");
    for case in bench.cases.iter().filter(|case| case.group == "long") {
        let document = parse(&bench.markdown(case, 1), &[]).expect("long cases are accepted");
        group.bench_function(case.id.as_str(), |bencher| bencher.iter(|| serialize(black_box(&document))));
    }
    group.finish();
    criterion.final_summary();

    if let Ok(out) = env::var("GSX_BENCH_REPORT") {
        report(&out, &bench, &typical, &projected);
    }
}
