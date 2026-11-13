use gsx_core::document::{Limit, Media};
use gsx_core::project::{Facts, Format, MediaFact, MediaKind, Overflow};
use gsx_core::targets::{TARGETS, target};
use gsx_core::wire::{invalid_text, parse, project, refused_item, serialize, targets};

#[test]
fn refuses_item_ids_outside_the_media_id_rule() {
    let item = |id| MediaFact { id, format: Format::Image, alt: None, spoiler: false, inline: false };
    assert_eq!(refused_item(&[]), None);
    assert_eq!(refused_item(&[item("a"), item("b.c:d-e_f")]), None);
    assert_eq!(
        refused_item(&[item("a"), item("x\"/><img src=\"media:evil")]).as_deref(),
        Some("item 1 id is not a media ID")
    );
    assert_eq!(refused_item(&[item(""), item("-a")]).as_deref(), Some("item 0 id is not a media ID"));
    let long = "a".repeat(129);
    assert_eq!(refused_item(&[item(&long[1..])]), None);
    assert_eq!(refused_item(&[item(&long)]).as_deref(), Some("item 0 id is not a media ID"));
}

#[test]
fn runs_inputs_at_the_depth_limits_within_a_thread_stack() {
    let containers = Limit::ContainerDepth.maximum() as usize;
    let inlines = Limit::InlineDepth.maximum() as usize;
    let lists: String = (0..containers - 1).flat_map(|level| ["  ".repeat(level), "- a\n".to_owned()]).collect();
    let mixed: String = (0..containers / 2).flat_map(|level| ["    ".repeat(level), "- > a\n".to_owned()]).collect();
    let inline = format!("{}a{}", "**".repeat(inlines), "**".repeat(inlines));
    let link =
        format!("[{}![a](https://e.com/i){}](https://e.com)", "**".repeat(inlines - 2), "**".repeat(inlines - 2));
    let directives = format!("{}a{}", "::x[**".repeat(inlines / 2), "**]".repeat(inlines / 2));
    let inputs = [
        format!("{}{link}", "> ".repeat(containers)),
        format!("{}{directives}", "> ".repeat(containers)),
        format!("{lists}{}- {inline}\n", "  ".repeat(containers - 1)),
        format!("{mixed}{}- > {inline}\n", "    ".repeat(containers / 2)),
    ];
    let stack = if cfg!(debug_assertions) { 1 << 20 } else { 128 << 10 };
    std::thread::Builder::new()
        .stack_size(stack)
        .spawn(move || {
            for input in &inputs {
                assert!(!parse(input.as_bytes(), &[]).contains("\"error\""), "{input}");
                assert!(serialize(input.as_bytes(), &[]).contains("\"serialized\""), "{input}");
                for target in TARGETS {
                    assert!(project(input.as_bytes(), &[], target, &Facts::default()).contains("\"projection\""));
                }
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn writes_serialized_source_or_the_parse_error() {
    assert_eq!(serialize("Title\n=====\n\n_a_\t\"q\"".as_bytes(), &[]), r##"{"serialized":"# Title\n\n*a*\t\"q\""}"##);
    assert_eq!(serialize(b"ab\xff", &[]), parse(b"ab\xff", &[]));
    let deep = ">".repeat(33);
    assert_eq!(serialize(deep.as_bytes(), &[]), parse(deep.as_bytes(), &[]));
}

#[test]
fn writes_projection() {
    let json = project("# Hi\n\n*a*".as_bytes(), &[], target("x-plain").unwrap(), &Facts::default());
    assert!(
        json.starts_with(r#"{"projection":{"fields":[{"name":"text","text":"Hi\n\na","links":[],"entities":[]}],"#)
    );
    assert!(
        json.contains(r#""measurements":[{"field":"text","unit":"x-weighted","value":5,"maximum":280}],"#),
        "{json}"
    );
    assert!(json.contains(concat!(
        r#""changes":[{"class":"textual_fallback","feature":"heading","reason":"heading_as_text","code":null,"#,
        r#""field":"text","range":[0,4],"truncated":null},"#
    )));
    assert!(json.ends_with(r#""media":[]}}"#), "{json}");
}

#[test]
fn projection_carries_the_document_diagnostics() {
    let media = [Media { id: "a", cover: None }];
    let input = "::media{id=\"x\"}\n\n::time[soon]{datetime=\"tomorrow\"} ::media{id=\"a\"}\n";
    let document = parse(input.as_bytes(), &media);
    let diagnostics = &document[document.find(",\"diagnostics\":").unwrap()..document.len() - 2];
    for code in ["MEDIA_REFERENCE_UNKNOWN", "DATETIME_INVALID", "MEDIA_REFERENCE_CONTEXT_UNSUPPORTED"] {
        assert!(diagnostics.contains(code), "{diagnostics}");
    }
    let threaded = Facts { overflow: Overflow::Thread, ..Facts::default() };
    for target in TARGETS {
        for facts in [&Facts::default(), &threaded] {
            let json = project(input.as_bytes(), &media, target, facts);
            assert!(json.contains(&format!("{diagnostics},\"media\":")), "{} {json}", target.name);
        }
    }
    let clean = project(b"a", &[], target("x-plain").unwrap(), &Facts::default());
    assert!(clean.contains(",\"diagnostics\":[],\"media\":"), "{clean}");
    let deep = ">".repeat(33);
    let rejected = project(deep.as_bytes(), &[], target("x-plain").unwrap(), &Facts::default());
    assert!(rejected.starts_with("{\"error\":"), "{rejected}");
    assert_eq!(rejected, parse(deep.as_bytes(), &[]));
}

#[test]
fn writes_entities_and_truncation() {
    let bluesky = target("bluesky-entities").unwrap();
    let json = project("**a** [b](https://c.example)".as_bytes(), &[], bluesky, &Facts::default());
    assert!(json.contains(r#""entities":[{"style":"link","range":[2,3],"value":"https://c.example/"}]"#), "{json}");
    let long = "word ".repeat(100);
    let json = project(long.as_bytes(), &[], bluesky, &Facts::default());
    assert!(json.contains(r#""truncated":{"unit":"graphemes","maximum":300,"value":"#), "{json}");
    assert!(json.contains(r#","exact":"#), "{json}");
}

#[test]
fn writes_blocked_codes_media_and_facts() {
    let json = project(b"", &[], target("x-plain").unwrap(), &Facts::default());
    assert!(json.contains(r#""class":"blocked","feature":"text","reason":"text_empty","code":"TEXT_EMPTY""#), "{json}");
    let media = [Media { id: "p1", cover: None }];
    let youtube = target("youtube-plain").unwrap();
    let facts = Facts { media: Some(MediaKind::Video), media_count: 1, title: Some("T"), ..Facts::default() };
    let json = project("a\n\n::media{id=\"p1\"}".as_bytes(), &media, youtube, &facts);
    assert!(json.contains(r#"{"name":"title","text":"T","links":[],"entities":[]}"#), "{json}");
    assert!(json.ends_with(r#""media":["p1"]}}"#), "{json}");
}

#[test]
fn projects_parse_errors_as_errors() {
    let x = target("x-plain").unwrap();
    assert_eq!(project(b"ab\xff", &[], x, &Facts::default()), parse(b"ab\xff", &[]));
    let deep = ">".repeat(65);
    assert_eq!(project(deep.as_bytes(), &[], x, &Facts::default()), parse(deep.as_bytes(), &[]));
}

#[test]
fn lists_targets_with_their_manifests() {
    let json = targets();
    for target in TARGETS {
        assert!(json.contains(&format!(r#"{{"name":"{}","form":"{}","#, target.name, target.form.version())), "{json}");
        assert!(json.contains(&format!(r#""digest":"{}""#, target.digest())), "{json}");
    }
    assert!(json.contains(r#""name":"bluesky-entities","form":"entities/1","offsets":"utf8""#), "{json}");
    assert!(json.contains(r#""name":"x-plain","form":"plain/1","offsets":null"#), "{json}");
}

#[test]
fn writes_document() {
    assert_eq!(
        parse("# a *b*".as_bytes(), &[]),
        concat!(
            r#"{"document":{"children":[{"type":"heading","range":[0,7],"depth":1,"gapBefore":0,"children":["#,
            r#"{"type":"text","range":[2,4],"value":"a "},"#,
            r#"{"type":"emphasis","range":[4,7],"children":[{"type":"text","range":[5,6],"value":"b"}]}]}],"#,
            r#""markers":[{"kind":"heading","range":[0,1]},{"kind":"emphasis","range":[4,5]},{"kind":"emphasis","range":[6,7]}],"#,
            r#""diagnostics":[]}}"#
        )
    );
}

#[test]
fn writes_optional_fields_and_gaps() {
    let json = parse("1. a\n\n```js\n\"\\\t\n```".as_bytes(), &[]);
    assert!(json.contains(r#""type":"list","range":[0,4],"ordered":true,"start":1,"spread":false"#), "{json}");
    assert!(json.contains(r#""checked":null"#), "{json}");
    assert!(json.contains(r#""lang":"js","meta":null,"value":"\"\\\t","gapBefore":2}"#), "{json}");
}

#[test]
fn writes_literal_provenance_and_diagnostics() {
    let json = parse("[a](ftp://b)".as_bytes(), &[]);
    assert!(json.contains(r#""provenance":"literal""#), "{json}");
    assert!(
        json.contains(
            r#"{"code":"LINK_LITERALIZED","severity":"warning","range":[0,12],"details":{"scheme":"unsupported"}}"#
        ),
        "{json}"
    );
}

#[test]
fn writes_media() {
    let media = [Media { id: "p1", cover: None }];
    let json = parse(r#"::media{id="p1"}"#.as_bytes(), &media);
    assert!(json.contains(r#""mediaId":"p1""#), "{json}");
    let json = parse(r#"::media{ids="p1,p2" layout="collage"}"#.as_bytes(), &media);
    assert!(json.contains(r#""code":"MEDIA_REFERENCE_UNKNOWN""#), "{json}");
    assert!(json.contains(r#""details":{"id":"p2","index":1}"#), "{json}");
}

#[test]
fn writes_errors() {
    assert_eq!(
        parse("😀\u{1}".as_bytes(), &[]),
        r#"{"error":{"code":"MARKDOWN_INVALID_TEXT","range":[2,3],"details":{"codePoint":1}}}"#
    );
    assert_eq!(
        parse(">".repeat(33).as_bytes(), &[]),
        r#"{"error":{"code":"MARKDOWN_TOO_COMPLEX","range":[32,33],"details":{"limit":"containerDepth","maximum":32}}}"#
    );
}

#[test]
fn reports_invalid_utf8_at_its_utf16_offset() {
    let mut input = "😀é".as_bytes().to_vec();
    input.extend([0xed, 0xa0, 0xbd]);
    input.extend(b"x");
    assert_eq!(parse(&input, &[]), invalid_text(3, Some(0xd83d)));
    assert_eq!(
        invalid_text(3, Some(0xd83d)),
        r#"{"error":{"code":"MARKDOWN_INVALID_TEXT","range":[3,4],"details":{"codePoint":55357}}}"#
    );
    assert_eq!(parse(b"ab\xff", &[]), r#"{"error":{"code":"MARKDOWN_INVALID_TEXT","range":[2,3],"details":{}}}"#);
}

#[test]
fn keeps_the_invalid_text_range_well_formed_at_the_last_offset() {
    assert_eq!(
        invalid_text(u32::MAX, None),
        r#"{"error":{"code":"MARKDOWN_INVALID_TEXT","range":[4294967295,4294967295],"details":{}}}"#
    );
}

#[test]
fn rejects_oversized_input_before_validating_its_encoding() {
    let too_complex = r#"{"error":{"code":"MARKDOWN_TOO_COMPLEX","range":[262144,262146],"#.to_owned()
        + r#""details":{"limit":"documentUnits","maximum":262144}}}"#;
    let mut wtf8 = "a".repeat(262_145).into_bytes();
    wtf8.extend([0xed, 0xa0, 0x80]);
    assert_eq!(parse(&wtf8, &[]), too_complex);
    assert_eq!(serialize(&wtf8, &[]), too_complex);
    let mut within = "a".repeat(262_143).into_bytes();
    within.extend([0xed, 0xa0, 0x80]);
    let invalid = r#"{"error":{"code":"MARKDOWN_INVALID_TEXT","range":[262143,262144],"details":{"codePoint":55296}}}"#;
    assert_eq!(parse(&within, &[]), invalid);
}

#[test]
fn reports_oversized_input_from_its_utf16_length() {
    let too_complex = |start: u32, end: u64| {
        format!(
            r#"{{"error":{{"code":"MARKDOWN_TOO_COMPLEX","range":[{start},{end}],"details":{{"limit":"documentUnits","maximum":262144}}}}}}"#
        )
    };
    let a = |count| "a".repeat(count);
    let cases = [
        (a(262_145), Some(too_complex(262_144, 262_145))),
        (format!("{}😀", a(262_143)), Some(too_complex(262_143, 262_145))),
        (format!("{}😀a", a(262_142)), Some(too_complex(262_144, 262_145))),
        (format!("{}é€", a(262_143)), Some(too_complex(262_144, 262_145))),
        (a(262_144), None),
        (format!("{}😀", a(262_142)), None),
        ("€".repeat(262_144), None),
    ];
    for (text, expected) in cases {
        let units: Vec<u16> = text.encode_utf16().collect();
        let at = char::decode_utf16(units[262_143..].iter().copied()).next().and_then(Result::ok).map_or(0, u32::from);
        let found = gsx_core::wire::oversized(units.len() as u64, at);
        assert_eq!(found, expected, "{} units", units.len());
        if let Some(expected) = expected {
            assert_eq!(parse(text.as_bytes(), &[]), expected);
        }
    }
    assert_eq!(gsx_core::wire::oversized(1 << 33, 0x61), Some(too_complex(262_144, u64::from(u32::MAX))));
}
