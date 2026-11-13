mod support;

use gsx_core::document::{Code, Details, Error, Kind, Limit, MarkerKind, Provenance, Range, Scheme, Severity, parse};
use std::time::{Duration, Instant};
use support::bench::Bench;

fn too_complex(text: &str) -> Option<(Limit, u32, u32)> {
    match parse(text, &[]) {
        Err(Error { code: Code::MarkdownTooComplex, range: Some(range), details: Details::Limit(limit) }) => {
            Some((limit, range.start, range.end))
        }
        _ => None,
    }
}

#[test]
fn rejects_control_characters() {
    let error = parse("😀\r\n\u{0}b", &[]).unwrap_err();
    assert_eq!(
        (error.code, error.range, error.details),
        (Code::MarkdownInvalidText, Some(Range { start: 4, end: 5 }), Details::CodePoint(0))
    );
}

#[test]
fn skips_a_leading_bom_but_counts_it() {
    let document = parse("\u{feff}# Hi", &[]).unwrap();
    assert_eq!(
        (document.children()[0].kind.clone(), document.children()[0].range),
        (Kind::Heading { depth: 1 }, Range { start: 1, end: 5 })
    );
}

#[test]
fn excludes_paired_markers_from_flanking_punctuation() {
    for (text, inner) in [
        ("a*~~b~~*c", Kind::Delete),
        ("a*==b==*c", Kind::Highlight),
        ("a*++b++*c", Kind::Underline),
        ("a*||b||*c", Kind::Spoiler),
    ] {
        let document = parse(text, &[]).unwrap();
        let emphasis = &document.children()[0].children[1];
        assert_eq!((&emphasis.kind, &emphasis.children[0].kind), (&Kind::Emphasis, &inner), "{text}");
    }
}

#[test]
fn flanks_on_unicode_punctuation_and_symbols() {
    let data = include_str!("syntax/punctuation.txt");
    let mut lines = data.lines();
    assert_eq!(lines.next(), Some(gsx_core::measure::UNICODE_VERSION));
    let listed: Vec<(u32, u32)> = lines
        .map(|line| {
            let (start, end) = line.split_once("..").unwrap_or((line, line));
            (u32::from_str_radix(start, 16).unwrap(), u32::from_str_radix(end, 16).unwrap())
        })
        .collect();
    let punctuation = |char: char| {
        let code = u32::from(char);
        listed.get(listed.partition_point(|&(_, end)| end < code)).is_some_and(|&(start, _)| start <= code)
    };
    let chars: Vec<char> = ('\u{80}'..'\u{40000}')
        .filter(|&char| !char.is_whitespace() && !char.is_control() && char != '\u{feff}')
        .collect();
    for chunk in chars.chunks(8000) {
        let text: String = chunk.iter().flat_map(|&char| ['a', '*', char, 'b', '*', '\n', '\n']).collect();
        let document = parse(&text, &[]).unwrap();
        assert_eq!(document.children().len(), chunk.len());
        for (&char, paragraph) in chunk.iter().zip(document.children()) {
            let emphasis = paragraph.children.iter().any(|node| node.kind == Kind::Emphasis);
            assert_eq!(emphasis, !punctuation(char), "{:X}", u32::from(char));
        }
    }
}

#[test]
fn links_to_the_serialized_href() {
    for (text, url, source) in [
        ("[a](https://EXAMPLE.com)", "https://example.com/", "https://EXAMPLE.com"),
        ("[a](https://example.com\\\\a)", "https://example.com/a", "https://example.com\\\\a"),
        ("<https://Example.com/a>", "https://example.com/a", "https://Example.com/a"),
    ] {
        let document = parse(text, &[]).unwrap();
        let link = &document.children()[0].children[0];
        let expected = Kind::Link { url: url.into(), source: source.into(), title: None };
        assert_eq!((&link.kind, link.provenance), (&expected, Provenance::Authored), "{text}");
        assert!(document.diagnostics().is_empty(), "{text}");
    }
}

#[test]
fn literalizes_refused_links_with_a_warning() {
    for (text, scheme) in [
        ("<foo@bar.example.com>", Scheme::Unsupported),
        ("<irc://foo.bar:2233/baz>", Scheme::Unsupported),
        ("[t](\u{2068}http://localhost:3000/privacy\u{2069})", Scheme::Relative),
        ("[t](https://user@example.com)", Scheme::Https),
    ] {
        let document = parse(text, &[]).unwrap();
        assert_eq!(document.children()[0].children[0].provenance, Provenance::Literal, "{text}");
        let [diagnostic] = document.diagnostics() else { panic!("{text}: {:?}", document.diagnostics()) };
        assert_eq!((diagnostic.code, &diagnostic.details), (Code::LinkLiteralized, &Details::Scheme(scheme)), "{text}");
    }
}

#[test]
fn binds_math_literals_before_other_inline_constructs() {
    for text in ["*a $b* c$", "$a *b$ c*", "$a `b$ c`", "[a $b](https://example.com) c$", "$a ::time[b$ c]{unix=\"1\"}"]
    {
        let document = parse(text, &[]).unwrap();
        let math: Vec<&str> = document.children()[0]
            .children
            .iter()
            .filter(|node| matches!(node.kind, Kind::Math { .. }))
            .map(|node| document.source(node.range))
            .collect();
        let start = text.find('$').unwrap();
        let end = text[start + 1..].find('$').unwrap() + start + 2;
        assert_eq!(math, [&text[start..end]], "{text}");
    }
}

#[test]
fn escapes_one_character() {
    let document = parse("x \\||a\\|| y ||z||", &[]).unwrap();
    let kinds: Vec<(&str, Provenance)> =
        document.children()[0].children.iter().map(|node| (document.source(node.range), node.provenance)).collect();
    assert_eq!(
        kinds,
        [
            ("x ", Provenance::Authored),
            ("\\|", Provenance::Literal),
            ("|a", Provenance::Authored),
            ("\\|", Provenance::Literal),
            ("| y ", Provenance::Authored),
            ("||z||", Provenance::Authored),
        ]
    );
}

#[test]
fn keeps_malformed_times_as_literal_text_with_an_error() {
    for (text, code, literal) in [
        ("a ::time[x]{unix=\"x\"} b", Code::DatetimeInvalid, "::time[x]{unix=\"x\"}"),
        (
            "a ::time[x]{datetime=\"2026-01-01T12:00\"} b",
            Code::DatetimeTimezoneRequired,
            "::time[x]{datetime=\"2026-01-01T12:00\"}",
        ),
        ("a ::time[ ]{unix=\"1\"} b", Code::DatetimeInvalid, "::time[ ]{unix=\"1\"}"),
        ("a ::time b", Code::MarkdownMalformed, "::time"),
    ] {
        let document = parse(text, &[]).unwrap();
        let literals: Vec<&str> = document.children()[0]
            .children
            .iter()
            .filter(|node| node.provenance == Provenance::Literal)
            .map(|node| document.source(node.range))
            .collect();
        assert_eq!(literals, [literal], "{text}");
        let [diagnostic] = document.diagnostics() else { panic!("{text}: {:?}", document.diagnostics()) };
        assert_eq!((diagnostic.code, diagnostic.severity), (code, Severity::Error), "{text}");
    }
}

#[test]
fn normalizes_line_endings() {
    let document = parse("a\r\nb\rc", &[]).unwrap();
    assert_eq!(document.text(), "a\nb\nc");
    assert_eq!((document.end().line, document.end().column, document.end().offset), (3, 2, 6));
}

#[test]
fn reports_offsets_in_the_input() {
    for (text, start) in [("a\r\n\r\n**b**\r\nc\rd", 5), ("a\r\r**b**\rc\nd", 3), ("a\n\r\n**b**\r\nc\r\nd", 4)] {
        let document = parse(text, &[]).unwrap();
        let paragraph = &document.children()[1];
        let strong = &paragraph.children[0];
        assert_eq!(strong.range, Range { start, end: start + 5 }, "{text:?}");
        assert_eq!(document.source(strong.range), "**b**", "{text:?}");
        assert_eq!(document.source(paragraph.range), &text[start as usize..], "{text:?}");
        assert_eq!((document.point(start).line, document.point(start).column), (3, 1), "{text:?}");
        assert_eq!(document.end().offset, text.len() as u32, "{text:?}");
        let marker = document.markers().iter().find(|marker| marker.kind == MarkerKind::Strong).unwrap();
        assert_eq!(marker.range, Range { start, end: start + 2 }, "{text:?}");
    }
    for (text, end) in [("a\r\n=", 4), ("a\r\n-", 4), ("a\r\n=\r\n", 6)] {
        let document = parse(text, &[]).unwrap();
        assert_eq!(document.markers()[0].range, Range { start: 3, end: 4 }, "{text:?}");
        assert_eq!(document.end().offset, end, "{text:?}");
    }
    let document = parse("x\r\n[t](ftp://a)", &[]).unwrap();
    assert_eq!(document.diagnostics()[0].range, Range { start: 3, end: 15 });
    let error = parse(&"a\r\n".repeat(16_384), &[]).unwrap_err();
    assert_eq!(error.range, Some(Range { start: 49_152, end: 49_152 }));
}

#[test]
fn enforces_budgets_where_exceeded() {
    let table = |row: &str, rows: usize| format!("| a | b |\n| - | - |\n{}", row.repeat(rows));
    for (text, expected) in [
        (format!("{} a", ">".repeat(33)), Some((Limit::ContainerDepth, 32, 34))),
        (format!("{} a", ">".repeat(32)), None),
        ("a\n".repeat(16_384), Some((Limit::DocumentLines, 32_768, 32_768))),
        ("a\n".repeat(16_383), None),
        ("$a=b$ ".repeat(1025), Some((Limit::DocumentMathLiterals, 6144, 6149))),
        ("$".repeat(5000), None),
        ("\\*".repeat(4097), Some((Limit::DocumentEscapes, 8192, 8194))),
        ("||a|| ".repeat(1025), Some((Limit::DocumentPairedDelimiters, 6144, 6146))),
        ("||".repeat(5000), None),
        (format!("a{}", "*".repeat(1025)), Some((Limit::BlockDelimiters, 1, 1026))),
        ("*a*\n\n".repeat(600), None),
        ("[a ".repeat(4097), Some((Limit::BlockLabels, 12_288, 12_289))),
        ("a]".repeat(257), Some((Limit::BlockLabelEnds, 513, 514))),
        ("- a\n".repeat(8193), Some((Limit::DocumentListItems, 32_768, 32_770))),
        ("a  \n".repeat(1025) + "a", Some((Limit::DocumentHardBreaks, 4097, 4099))),
        (table("| a | b |\n", 4095), None),
        (table("| a | b |\n", 4096), Some((Limit::DocumentTableCells, 40_970, 40_979))),
        (table("| a |\n", 4096), Some((Limit::DocumentTableCells, 24_590, 24_595))),
        (format!("{}a{}", "*".repeat(32), "*".repeat(32)), None),
        (format!("{}a{}", "*".repeat(34), "*".repeat(34)), Some((Limit::InlineDepth, 32, 37))),
        (format!("{}a{}", "::x[".repeat(17), "]".repeat(17)), Some((Limit::InlineDepth, 64, 70))),
        ("a".repeat(262_144), None),
        ("a".repeat(262_145), Some((Limit::DocumentUnits, 262_144, 262_145))),
        ("é".repeat(262_144), None),
        (format!("{}😀", "a".repeat(262_143)), Some((Limit::DocumentUnits, 262_143, 262_145))),
    ] {
        assert_eq!(too_complex(&text), expected, "{}", &text[..text.len().min(40)]);
    }
}

#[test]
fn uses_utf16_coordinates() {
    let document = parse("😀 **a**\n\n||b||", &[]).unwrap();
    let paragraph = &document.children()[0];
    let strong = &paragraph.children[1];
    assert_eq!(strong.kind, Kind::Strong);
    assert_eq!(strong.range, Range { start: 3, end: 8 });
    let point = document.point(strong.range.start);
    assert_eq!((point.line, point.column), (1, 4));
    assert_eq!(document.source(strong.range), "**a**");
    assert_eq!(document.children()[1].gap_before, Some(2));
}

#[test]
fn marks_literal_provenance() {
    let document = parse("<b>x</b> [a](/b)", &[]).unwrap();
    let children = &document.children()[0].children;
    let literals: Vec<&str> = children
        .iter()
        .filter(|node| node.provenance == Provenance::Literal)
        .map(|node| document.source(node.range))
        .collect();
    assert_eq!(literals, ["<b>x</b>", "[a](/b)"]);
}

#[test]
fn validates_link_destinations_as_whatwg_urls() {
    let kinds = |text: &str| {
        let document = parse(text, &[]).unwrap();
        let node = document.children()[0].children[0].clone();
        (node.provenance, document.diagnostics().iter().map(|diagnostic| diagnostic.code).collect::<Vec<_>>())
    };
    assert_eq!(kinds("[a](https://example.com/a?b#c)"), (Provenance::Authored, vec![]));
    assert_eq!(kinds("[a](https://xn--bcher-kva.example/)"), (Provenance::Authored, vec![]));
    assert_eq!(kinds("[a](http://example.com:65536/)"), (Provenance::Literal, vec![Code::LinkLiteralized]));
    assert_eq!(kinds("[a](https://exa%00mple.com/)"), (Provenance::Literal, vec![Code::LinkLiteralized]));
}

#[test]
fn keeps_math_bodies_raw_across_nodes_without_container_prefixes() {
    let document = parse("> a $x `b\n> c` **d** \\{ &amp;$ e", &[]).unwrap();
    let paragraph = &document.children()[0].children[0];
    let math = paragraph.children.iter().find(|node| matches!(node.kind, Kind::Math { .. })).unwrap();
    assert_eq!(document.source(math.range), "$x `b\n> c` **d** \\{ &amp;$");
    assert_eq!(math.kind, Kind::Math { value: "x `b\nc` **d** \\{ &amp;".into() });
    assert_eq!(math.provenance, Provenance::Authored);
}

#[test]
fn drops_container_prefixes_from_code_spans() {
    let document = parse("> `a\n> b`\n\n- `c\n     d`", &[]).unwrap();
    let code = |node: &gsx_core::document::Node| node.children[0].children[0].kind.clone();
    assert_eq!(code(&document.children()[0]), Kind::InlineCode { value: "a b".into() });
    assert_eq!(code(&document.children()[1].children[0]), Kind::InlineCode { value: "c d".into() });
}

#[test]
fn exposes_marker_ranges() {
    let document = parse("||a|| ++b++ ==c== *d* `e`", &[]).unwrap();
    let markers: Vec<(MarkerKind, u32, u32)> =
        document.markers().iter().map(|marker| (marker.kind, marker.range.start, marker.range.end)).collect();
    assert_eq!(
        markers,
        [
            (MarkerKind::Spoiler, 0, 2),
            (MarkerKind::Spoiler, 3, 5),
            (MarkerKind::Underline, 6, 8),
            (MarkerKind::Underline, 9, 11),
            (MarkerKind::Highlight, 12, 14),
            (MarkerKind::Highlight, 15, 17),
            (MarkerKind::Emphasis, 18, 19),
            (MarkerKind::Emphasis, 20, 21),
            (MarkerKind::CodeText, 22, 23),
            (MarkerKind::CodeText, 24, 25),
        ]
    );
}

#[test]
fn survives_pathological_input() {
    for text in [
        "*".repeat(1000),
        "[".repeat(5000),
        "]".repeat(5000),
        "||".repeat(1000),
        "`".repeat(5000),
        "a_".repeat(500),
        "[a](".repeat(1000),
        "<".repeat(5000),
        "::time[".repeat(1000),
        "::media{".repeat(1000),
        "| a ".repeat(1000) + "\n" + &"|-".repeat(1000),
        "> - ".repeat(30) + "a",
        "::m{e=\"\"".into(),
        "::m{e=\"\"}".into(),
        "::time[10:30]{e='' f=\"x\"}".into(),
    ] {
        let _ = parse(&text, &[]);
    }
}

#[test]
fn parses_labels_across_lines_and_bare_references() {
    for (text, value) in [
        ("::x[|a|\n|-|\na＠b.c]", "::x[|a|\n|-|\na＠b.c]"),
        ("::x[|a|\n|-|\na@b.c]", "::x[|a|\n|-|\na@b.c]"),
        ("::x[\na]", "::x[\na]"),
        ("*::time[\r\n\u{feff}", "*::time[\n\u{feff}"),
        ("a::time[-->\n\u{feff}user@example.coma]{x}", "a::time[-->\n\u{feff}user@example.coma]{x}"),
        ("::x[a\na@b.com]", "::x[a\na@b.com]"),
        ("::x[\na\n  b]", "::x[\na\n  b]"),
        ("&#;\\", "&#;\\"),
        ("$a &#; = &#x; = &#12345678; = &#x110000;$", "a &#; = &#x; = &#12345678; = &#x110000;"),
    ] {
        let document = parse(text, &[]).unwrap_or_else(|error| panic!("{text:?}: {error}"));
        let values: String = document.children()[0]
            .children
            .iter()
            .map(|node| match &node.kind {
                Kind::Text { value } | Kind::Math { value } => value.as_str(),
                _ => "",
            })
            .collect();
        assert_eq!(values, value, "{text:?}");
    }
}

fn seconds_per_byte(text: &str) -> f64 {
    let mut best = f64::MAX;
    for _ in 0..3 {
        let start = Instant::now();
        let mut runs = 0_u32;
        while runs == 0 || start.elapsed() < Duration::from_millis(5) {
            let _ = parse(text, &[]);
            runs += 1;
        }
        best = best.min(start.elapsed().as_secs_f64() / f64::from(runs) / text.len() as f64);
    }
    best
}

#[test]
fn parses_unmatched_paired_sequences_in_linear_time() {
    let text = |times: usize| "==a ".repeat(times) + &"b|| ".repeat(times);
    let growth = seconds_per_byte(&text(8000)) / seconds_per_byte(&text(2000));
    assert!(growth <= 2.5, "{growth:.1}");
}

#[test]
fn pairs_html_blocks_in_linear_time() {
    let blocks = |times: usize| "<a>\n\n<B>\n\n</c>\n\n</b>\n\n</c>\n\n".repeat(times);
    assert!(parse(&blocks(1600), &[]).is_ok());
    let growth = seconds_per_byte(&blocks(1600)) / seconds_per_byte(&blocks(400));
    assert!(growth <= 2.5, "pairing costs {growth:.1} times more per byte at four times the size");

    let media = |pair: &str| pair.repeat(2000) + &"::media{id=\"x\"}\n".repeat(8000);
    let (paired, unpaired) = (media("<a>\n\n</A>\n\n"), media("</a>\n\n</A>\n\n"));
    assert!(parse(&paired, &[]).is_ok() && parse(&unpaired, &[]).is_ok());
    let ratio = seconds_per_byte(&paired) / seconds_per_byte(&unpaired);
    assert!(ratio <= 2.0, "media directives after paired blocks cost {ratio:.1} times more per byte");
}

#[test]
fn resolves_media_directives_on_one_line_in_linear_time() {
    let line = |name: &str, directive: &str, times: usize| match name {
        "joined" => directive.repeat(times),
        "separated" => format!("{directive} ").repeat(times),
        _ => "a ".repeat(times) + &directive.repeat(times / 2),
    };
    for name in ["joined", "separated", "after text"] {
        let (valid, invalid) = (line(name, "::media{id=\"x\"}", 16000), line(name, "::media{id=\"\"}", 16000));
        let codes: Vec<Code> =
            parse(&valid, &[]).unwrap().diagnostics().iter().map(|diagnostic| diagnostic.code).collect();
        assert_eq!(codes, vec![Code::MediaReferenceContextUnsupported; valid.matches("::").count()], "{name}");
        let cost = seconds_per_byte(&valid);
        let growth = cost / seconds_per_byte(&line(name, "::media{id=\"x\"}", 4000));
        let ratio = cost / seconds_per_byte(&invalid);
        assert!(growth <= 2.5, "{name} directives cost {growth:.1} times more per byte at four times the size");
        assert!(ratio <= 2.0, "{name} directives cost {ratio:.1} times more per byte than invalid ones");
    }
}

#[test]
fn literalizes_labels_across_nodes_in_linear_time() {
    let shape = |name: &str, times: usize, open: bool| match name {
        "ending in code" => "[`c`".repeat(times) + if open { "  `]`" } else { "][`]`" },
        "before unaligned text" => {
            "[`c`".repeat(times) + if open { "  x &#10;" } else { "][x]&#10;" } + &"a".repeat(times * 16)
        }
        _ => "&#10;[`c`".repeat(times) + if open { "  x " } else { "][x]" } + &"a".repeat(times * 16),
    };
    for name in ["ending in code", "before unaligned text", "after unaligned text"] {
        let (labels, open) = (shape(name, 4000, false), shape(name, 4000, true));
        assert!(parse(&labels, &[]).is_ok() && parse(&open, &[]).is_ok(), "{name}");
        let cost = seconds_per_byte(&labels);
        let growth = cost / seconds_per_byte(&shape(name, 1000, false));
        let ratio = cost / seconds_per_byte(&open);
        assert!(growth <= 2.5, "labels {name} cost {growth:.1} times more per byte at four times the size");
        assert!(ratio <= 2.0, "labels {name} cost {ratio:.1} times more per byte than unclosed ones");
    }
}

#[test]
fn refuses_reserved_time_directives_in_linear_time() {
    let line = |name: &str, form: &str, times: usize| match form {
        "labels" => format!("::{name}[a").repeat(times),
        "quoted labels" => format!("::{name}[' ").repeat(times),
        "attributes" => format!("::{name}[a]{{").repeat(times),
        _ => format!("::{name}[").repeat(times) + &"a".repeat(times * 16),
    };
    for form in ["labels", "quoted labels", "attributes", "labels before text"] {
        let (time, other) = (line("time", form, 4000), line("tame", form, 4000));
        let ends: Vec<(Code, u32)> = parse(&time, &[])
            .unwrap()
            .diagnostics()
            .iter()
            .map(|diagnostic| (diagnostic.code, diagnostic.range.end))
            .collect();
        assert_eq!(ends, vec![(Code::MarkdownMalformed, time.len() as u32); 4000], "{form}");
        let cost = seconds_per_byte(&time);
        let growth = cost / seconds_per_byte(&line("time", form, 1000));
        let ratio = cost / seconds_per_byte(&other);
        assert!(growth <= 2.5, "{form} cost {growth:.1} times more per byte at four times the size");
        assert!(ratio <= 2.0, "{form} cost {ratio:.1} times more per byte than other directives");
    }
}

#[test]
fn scans_autolink_literal_punctuation_runs_in_linear_time() {
    for (name, prefix, run) in [("path", "http://a.b/", ","), ("domain", "http://a", "."), ("www", "www.a", ".")] {
        let line = |run: &str, times: usize| format!("{prefix}{}x", run.repeat(times));
        let link_end = |text: &str| parse(&format!("{text} y"), &[]).unwrap().children()[0].children[0].range.end;
        assert_eq!(link_end(&line(run, 8)), line(run, 8).len() as u32, "{name}");
        assert_eq!(link_end(&format!("{prefix}{}", run.repeat(8))), prefix.len() as u32, "{name}");
        let cost = seconds_per_byte(&line(run, 8000));
        let growth = cost / seconds_per_byte(&line(run, 2000));
        let ratio = cost / seconds_per_byte(&line("a", 8000));
        assert!(growth <= 2.5, "{name} runs cost {growth:.1} times more per byte at four times the size");
        assert!(ratio <= 3.0, "{name} runs cost {ratio:.1} times more per byte than letters");
    }
}

#[test]
fn looks_up_definitions_in_linear_time() {
    let defined = parse("[a]: b\n\n[c] [A]", &[]).unwrap();
    let provenances: Vec<Provenance> = defined.children()[1].children.iter().map(|node| node.provenance).collect();
    assert_eq!(provenances, [Provenance::Authored, Provenance::Literal]);
    for (name, definition, reference) in [("definitions", "[a]: b\n", "[c]"), ("footnotes", "[^a]: b\n", "[^c]")] {
        let text =
            |times: usize| definition.repeat(times) + &format!("\n{}\n", reference.repeat(255)).repeat(times / 20);
        assert!(parse(&text(4000), &[]).is_ok(), "{name}");
        let growth = seconds_per_byte(&text(4000)) / seconds_per_byte(&text(1000));
        assert!(growth <= 2.5, "{name} cost {growth:.1} times more per byte at four times the size");
    }
}

#[test]
fn normalizes_nested_labels_in_linear_time() {
    let longest = format!("[{}]: b\n\n[{}]", "a".repeat(999), "A".repeat(999));
    assert_eq!(parse(&longest, &[]).unwrap().children()[1].children[0].provenance, Provenance::Literal);
    let text = |length: usize| {
        let labels = (length / 1024).clamp(1, 255);
        format!("[a]: b\n\n{}{}{}", "[".repeat(labels), "a ".repeat(length / 2), "]".repeat(labels))
    };
    assert!(parse(&text(65536), &[]).is_ok());
    let growth = seconds_per_byte(&text(65536)) / seconds_per_byte(&text(16384));
    assert!(growth <= 2.5, "nested labels cost {growth:.1} times more per byte at four times the size");
}

#[test]
fn parses_bench_cases_in_linear_time() {
    let bench = Bench::load();
    let mut failures = vec![];
    for case in &bench.cases {
        let full = parse(&bench.markdown(case, 1), &[]);
        if case.group == "reject" {
            if full.is_ok() {
                failures.push(format!("{} is accepted", case.id));
            }
            continue;
        }
        if let Err(error) = full {
            failures.push(format!("{} is rejected: {error}", case.id));
        }
        let growth = seconds_per_byte(&bench.markdown(case, 8)) / seconds_per_byte(&bench.markdown(case, 32));
        if growth > 2.5 {
            failures.push(format!("{} costs {growth:.1} times more per byte at a quarter of the size", case.id));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
