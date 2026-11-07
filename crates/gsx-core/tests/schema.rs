mod support;
#[path = "support/schema.rs"]
mod validator;

use std::fs;
use std::path::{Path, PathBuf};
use std::thread;

use gsx_core::wire;
use serde_json::{Number, Value};
use support::Json;
use validator::{Schema, deep};

const CORPUS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../testdata/corpus");

fn value(json: &Json) -> Value {
    match json {
        Json::Null => Value::Null,
        Json::Bool(value) => Value::Bool(*value),
        Json::Number(value) if value.fract() == 0.0 && value.abs() < 2f64.powi(53) => Value::from(*value as i64),
        Json::Number(value) => Value::Number(Number::from_f64(*value).expect("JSON numbers are finite")),
        Json::String(value) => Value::String(value.clone()),
        Json::Array(items) => Value::Array(items.iter().map(value).collect()),
        Json::Object(map) => Value::Object(map.iter().map(|(key, item)| (key.clone(), value(item))).collect()),
    }
}

fn jsonl(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            jsonl(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "jsonl") {
            files.push(path);
        }
    }
}

fn expectations(schema: &Schema, file: &Path) -> (usize, Vec<String>) {
    let text = fs::read_to_string(file).unwrap();
    let mut failures = vec![];
    let mut checked = 0;
    for (index, line) in text.lines().enumerate().skip(1).filter(|(_, line)| !line.is_empty()) {
        let expected = Json::parse(line).get("expected").clone();
        if expected == Json::Null {
            continue;
        }
        checked += 1;
        if let Err(error) = schema.check(&value(&expected)) {
            failures.push(format!("{}:{} {error}", file.display(), index + 1));
        }
    }
    (checked, failures)
}

#[test]
fn corpus_expectations_match_the_schema() {
    let schema = Schema::load();
    let mut files = vec![];
    jsonl(Path::new(CORPUS), &mut files);
    files.sort();
    let results: Vec<(usize, Vec<String>)> = thread::scope(|scope| {
        let schema = &schema;
        let workers: Vec<_> =
            files.iter().map(|file| scope.spawn(move || deep(|| expectations(schema, file)))).collect();
        workers.into_iter().map(|worker| worker.join().unwrap()).collect()
    });
    let checked: usize = results.iter().map(|(checked, _)| checked).sum();
    let failures: Vec<&String> = results.iter().flat_map(|(_, failures)| failures).collect();
    assert!(checked > 0, "no corpus expectations under {CORPUS}");
    assert!(
        failures.is_empty(),
        "{} of {checked} corpus expectations violate the schema:\n{}",
        failures.len(),
        failures.iter().take(20).map(|failure| failure.as_str()).collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn targets_match_the_schema() {
    Schema::load().check_wire(&wire::targets()).unwrap();
}
