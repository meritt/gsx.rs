use std::ops::Range;

use gsx_core::measure::{Fact, Facts, Unit, X_URL_WEIGHT, clusters, measure};

fn count(unit: Unit, text: &str) -> usize {
    measure(unit, text, &Facts::default()) as usize
}

fn graphemes(text: &str) -> usize {
    count(Unit::Graphemes, text)
}

fn x_weighted(text: &str, urls: &[Range<usize>]) -> usize {
    measure(Unit::XWeighted, text, &Facts { urls, ..Facts::default() }) as usize
}

fn mastodon_weighted(text: &str, urls: &[Range<usize>], elided: &[Range<usize>], reserved: usize) -> usize {
    let resolved = [(Fact::CharactersReservedPerUrl, reserved as u32)];
    measure(Unit::MastodonWeighted, text, &Facts { urls, elided, resolved: &resolved }) as usize
}

const ALPHABET: &[&str] = &[
    "a",
    "Z",
    " ",
    "\n",
    "\r",
    "\t",
    "\u{1}",
    ".",
    "/",
    "@",
    "\u{E9}",
    "\u{301}",
    "\u{200D}",
    "\u{FE0F}",
    "\u{20E3}",
    "#",
    "1",
    "\u{1F468}",
    "\u{1F469}",
    "\u{1F44D}",
    "\u{1F3FD}",
    "\u{2764}",
    "\u{1F1FA}",
    "\u{1F1E6}",
    "\u{1F3F4}",
    "\u{E0067}",
    "\u{E007F}",
    "\u{1100}",
    "\u{1161}",
    "\u{11A8}",
    "\u{AC00}",
    "\u{915}",
    "\u{94D}",
    "\u{937}",
    "\u{93F}",
    "\u{600}",
    "\u{4E2D}",
    "\u{2014}",
    "\u{10FFFF}",
    "\u{1F600}",
];

const SEPARATOR: &str = "\u{1}";

struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }

    fn text(&mut self) -> String {
        let length = self.below(24);
        (0..length).map(|_| ALPHABET[self.below(ALPHABET.len())]).collect()
    }

    fn ranges(&mut self, text: &str) -> Vec<Range<usize>> {
        let limit = text.len() + 3;
        (0..self.below(4)).map(|_| self.below(limit)..self.below(limit)).collect()
    }
}

fn samples(seed: u64, mut check: impl FnMut(&mut Random)) {
    let mut random = Random(seed);
    for _ in 0..20_000 {
        check(&mut random);
    }
}

#[test]
fn arbitrary_spans_never_panic() {
    samples(0x9E37_79B9_7F4A_7C15, |random| {
        let text = random.text();
        let urls = random.ranges(&text);
        let elided = random.ranges(&text);
        let reserved = random.below(30);
        let x = x_weighted(&text, &urls);
        let mastodon = mastodon_weighted(&text, &urls, &elided, reserved);
        assert!(x <= 2 * count(Unit::CodePoints, &text) + X_URL_WEIGHT * urls.len());
        assert!(mastodon <= graphemes(&text) + reserved * urls.len());
    });
}

#[test]
fn clusters_partition_the_text() {
    samples(0xD1B5_4A32_D192_ED03, |random| {
        let text = random.text();
        let parts: Vec<_> = clusters(&text).collect();
        assert_eq!(parts.concat(), text);
        assert!(parts.iter().all(|part| graphemes(part) == 1), "{text:?} {parts:?}");
    });
}

#[test]
fn counts_add_across_a_hard_boundary() {
    samples(0x2545_F491_4F6C_DD1D, |random| {
        let (left, right) = (random.text(), random.text());
        let joined = format!("{left}{SEPARATOR}{right}");
        for unit in [Unit::CodePoints, Unit::Utf16CodeUnits, Unit::Utf8Bytes, Unit::Graphemes, Unit::ThreadsWeighted] {
            let sum = count(unit, &left) + count(unit, SEPARATOR) + count(unit, &right);
            assert_eq!(count(unit, &joined), sum, "{unit:?} {joined:?}");
        }
        assert_eq!(x_weighted(&joined, &[]), x_weighted(&left, &[]) + 1 + x_weighted(&right, &[]));
        assert_eq!(count(Unit::MastodonWeighted, &joined), graphemes(&left) + 1 + graphemes(&right));
    });
}

#[test]
fn urls_weigh_their_reserved_length() {
    samples(0x6A09_E667_F3BC_C909, |random| {
        let (left, url, right) = (random.text(), random.text(), random.text());
        let text = format!("{left}{SEPARATOR}{url}{SEPARATOR}{right}");
        let start = left.len() + SEPARATOR.len();
        let span = start..start + url.len();
        let reserved = random.below(30);
        let urls: &[Range<usize>] = if url.is_empty() { &[] } else { std::slice::from_ref(&span) };
        let linked = urls.len();
        assert_eq!(
            x_weighted(&text, urls),
            x_weighted(&left, &[]) + 2 + X_URL_WEIGHT * linked + x_weighted(&right, &[])
        );
        assert_eq!(
            mastodon_weighted(&text, urls, &[], reserved),
            graphemes(&left) + 2 + reserved * linked + graphemes(&right) + graphemes(&url) * (1 - linked)
        );
        assert_eq!(mastodon_weighted(&text, &[], urls, reserved), graphemes(&left) + 2 + graphemes(&right));
    });
}

#[test]
fn every_cluster_weighs_at_least_one() {
    samples(0xBB67_AE85_84CA_A73B, |random| {
        let text = random.text();
        assert!(count(Unit::ThreadsWeighted, &text) >= graphemes(&text));
        assert!(x_weighted(&text, &[]) >= graphemes(&text));
    });
}
