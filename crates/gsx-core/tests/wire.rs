use gsx_core::document::{Limit, Media};
use gsx_core::project::{Facts, MediaKind};
use gsx_core::targets::{TARGETS, target};
use gsx_core::wire::{invalid_text, parse, project, serialize, targets};

#[test]
fn runs_inputs_at_the_depth_limits_within_a_thread_stack() {
    let containers = Limit::ContainerDepth.maximum() as usize;
    let inlines = Limit::InlineDepth.maximum() as usize;
    let lists: String = (0..containers - 1).flat_map(|level| ["  ".repeat(level), "- a\n".to_owned()]).collect();
    let mixed: String = (0..containers / 2).flat_map(|level| ["    ".repeat(level), "- > a\n".to_owned()]).collect();
    let inline = format!("{}a{}", "**".repeat(inlines), "**".repeat(inlines));
    let link =
        format!("[{}![a](https://e.com/i){}](https://e.com)", "**".repeat(inlines - 2), "**".repeat(inlines - 2));
    let directives = format!("{}a{}", ":x[**".repeat(inlines / 2), "**]".repeat(inlines / 2));
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
