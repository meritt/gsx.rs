#![no_main]

use arbitrary::Arbitrary;
use gsx_core::document::{Media, parse};
use gsx_core::project::{Facts, Format, MediaFact, MediaKind, Options, Search, project_with};
use gsx_core::targets::TARGETS;
use libfuzzer_sys::fuzz_target;

const MEDIA: [Media<'static>; 4] = [
    Media { id: "a", cover: None },
    Media { id: "b", cover: Some("c") },
    Media { id: "d", cover: None },
    Media { id: "e", cover: None },
];
const FORMATS: [Format; 4] = [Format::Image, Format::Gif, Format::Video, Format::Audio];

#[derive(Arbitrary, Debug)]
struct Item<'a> {
    format: u8,
    alt: Option<&'a str>,
    spoiler: bool,
    inline: bool,
}

#[derive(Arbitrary, Debug)]
struct Input<'a> {
    text: &'a str,
    title: Option<&'a str>,
    media: u8,
    video: bool,
    items: [Item<'a>; 4],
    max_characters: Option<u16>,
    characters_reserved_per_url: Option<u8>,
    caption_limit: Option<u16>,
    entities_allowed: bool,
    maximum: Option<u8>,
}

fuzz_target!(|input: Input| {
    let count = usize::from(input.media) % (MEDIA.len() + 1);
    let Ok(document) = parse(input.text, &MEDIA[..count]) else {
        return;
    };
    let items: Vec<MediaFact> = MEDIA[..count]
        .iter()
        .zip(&input.items)
        .map(|(media, item)| MediaFact {
            id: media.id,
            format: FORMATS[usize::from(item.format) % FORMATS.len()],
            alt: item.alt,
            spoiler: item.spoiler,
            inline: item.inline,
        })
        .collect();
    let facts = Facts {
        media: (count > 0).then_some(if input.video { MediaKind::Video } else { MediaKind::Image }),
        media_count: count,
        placement: None,
        title: input.title,
        target_url: None,
        max_characters: input.max_characters.map(u32::from),
        characters_reserved_per_url: input.characters_reserved_per_url.map(u32::from),
        caption_limit: input.caption_limit.map(u32::from),
        description_limit: input.caption_limit.map(u32::from),
        entities_allowed: input.entities_allowed,
        items: &items,
    };
    let options = Options { search: None, maximum: input.maximum.map(usize::from) };
    for target in TARGETS {
        let first = project_with(&document, target, &facts, &options);
        let second = project_with(&document, target, &facts, &options);
        assert_eq!(first, second, "{} is not deterministic", target.name);
        let verified = count == 0 && (!facts.entities_allowed || target.form.entities().is_some());
        if verified && input.text.len() <= 512 {
            let exhaustive = Options { search: Some(Search::Exhaustive), ..options };
            let monotone = Options { search: Some(Search::Monotone), ..options };
            assert_eq!(
                project_with(&document, target, &facts, &monotone),
                project_with(&document, target, &facts, &exhaustive),
                "{} monotone and exhaustive search disagree",
                target.name
            );
        }
    }
});
