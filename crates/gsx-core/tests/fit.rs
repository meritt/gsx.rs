mod support;

use gsx_core::document::{Document, parse};
use gsx_core::project::{Facts, MediaKind, Options, Search, Target, project_with};
use gsx_core::targets::TARGETS;
use support::{Json, LONE_SURROGATE};

const MAXIMA: [usize; 4] = [6, 17, 40, 120];

const FRAGMENTS: [&str; 32] = [
    "word",
    "another",
    "x",
    "é",
    "e\u{301}",
    "👩‍👩‍👧",
    "🇺🇦",
    "漢字かな",
    "مرحبا",
    "https://example.com/path?q=1",
    "www.example.org",
    "mail@example.net",
    "@alice@social.example",
    "#hashtag",
    "`code`",
    "**strong text**",
    "_emphasis_",
    "~~gone~~",
    "[label](https://example.com/a)",
    "[https://example.com](https://example.com)",
    "a-very-long-hyphenated-compound-word-that-exceeds-thirty-two",
    "end.",
    "(paren)",
    "\u{feff}",
    "1234567",
    "...",
    "—",
    "<angle>",
    "&amp",
    "\\*",
    "\"quote\"",
    "?!",
];

const STARTS: [&str; 8] = ["", "", "", "- ", "1. ", "> ", "# ", "- [ ] "];

struct Random(u64);

impl Random {
    fn next(&mut self, bound: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % bound as u64) as usize
    }
}

fn generated(seed: u64) -> String {
    let mut random = Random(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let mut out = String::new();
    for line in 0..=random.next(8) {
        if line > 0 {
            out.push_str(if random.next(3) == 0 { "\n\n" } else { "\n" });
        }
        out.push_str(STARTS[random.next(STARTS.len())]);
        for word in 0..=random.next(12) {
            if word > 0 {
                out.push_str(if random.next(6) == 0 { "" } else { " " });
            }
            out.push_str(FRAGMENTS[random.next(FRAGMENTS.len())]);
        }
    }
    out
}

fn corpus() -> Vec<Document> {
    let path = format!("{}/../../testdata/corpus/document.jsonl", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(path)
        .unwrap()
        .split('\n')
        .skip(1)
        .filter(|line| !line.is_empty())
        .map(Json::parse)
        .filter_map(|case| {
            let markdown = case.get("markdown").str()?;
            if markdown.len() > 1000 || markdown.contains(LONE_SURROGATE) || !case.get("media").array().is_empty() {
                return None;
            }
            parse(markdown, &[]).ok()
        })
        .collect()
}

fn agree(document: &Document, label: &str) {
    let image = Facts { media: Some(MediaKind::Image), media_count: 1, ..Facts::default() };
    let entities = Facts { entities_allowed: true, caption_limit: Some(1024), ..Facts::default() };
    for facts in [Facts::default(), image, entities] {
        let applies = |target: &&&Target| {
            if facts.media.is_some() {
                target.name == "tiktok-plain"
            } else {
                !facts.entities_allowed || target.form.entities().is_some()
            }
        };
        for target in TARGETS.iter().filter(applies) {
            for maximum in MAXIMA {
                let options = |search| Options { search: Some(search), maximum: Some(maximum) };
                let monotone = project_with(document, target, &facts, &options(Search::Monotone));
                let exhaustive = project_with(document, target, &facts, &options(Search::Exhaustive));
                assert_eq!(monotone, exhaustive, "{label} {} maximum {maximum} {:?}", target.name, document.text());
            }
        }
    }
}

#[test]
fn monotone_search_matches_exhaustive_on_corpus() {
    for (index, document) in corpus().iter().enumerate() {
        agree(document, &format!("corpus {index}"));
    }
}

#[test]
fn monotone_search_matches_exhaustive_on_generated_inputs() {
    for seed in 0..600 {
        let markdown = generated(seed);
        if let Ok(document) = parse(&markdown, &[]) {
            agree(&document, &format!("seed {seed}"));
        }
    }
}

#[test]
#[ignore = "timing note: run with --release --ignored --nocapture"]
fn fitting_scales() {
    let target = TARGETS.iter().find(|target| target.name == "tiktok-plain").unwrap();
    for size in [5_000, 10_000, 20_000, 40_000] {
        let markdown = "word, https://example.com/x 12 ".repeat(size / 31);
        let document = parse(&markdown, &[]).unwrap();
        for search in [Search::Monotone, Search::Exhaustive] {
            let options = Options { search: Some(search), maximum: Some(size / 2) };
            let start = std::time::Instant::now();
            let projection = project_with(&document, target, &Facts::default(), &options);
            println!("{size} {search:?} {:?} {}", start.elapsed(), projection.fields[0].text.len());
        }
    }
}
