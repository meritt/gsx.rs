use icu_properties::CodePointMapData;
use icu_properties::props::{GeneralCategory, GeneralCategoryGroup};

static SPACE: &[(u32, u32)] = &[
    (0x9, 0xD),
    (0x20, 0x20),
    (0xA0, 0xA0),
    (0x1680, 0x1680),
    (0x2000, 0x200A),
    (0x2028, 0x2029),
    (0x202F, 0x202F),
    (0x205F, 0x205F),
    (0x3000, 0x3000),
    (0xFEFF, 0xFEFF),
];

fn category(point: u32) -> GeneralCategory {
    CodePointMapData::<GeneralCategory>::new().get32(point)
}

pub fn space(point: u32) -> bool {
    SPACE.iter().any(|&(from, to)| (from..=to).contains(&point))
}

pub fn punctuation(point: u32) -> bool {
    GeneralCategoryGroup::Punctuation.contains(category(point))
}

pub fn invisible(point: u32) -> bool {
    space(point) || GeneralCategoryGroup::Other.union(GeneralCategoryGroup::Mark).contains(category(point))
}

pub fn letter_or_number(point: u32) -> bool {
    GeneralCategoryGroup::Letter.union(GeneralCategoryGroup::Number).contains(category(point))
}

pub fn word(point: u32) -> bool {
    let category = category(point);
    GeneralCategoryGroup::Letter.union(GeneralCategoryGroup::Mark).contains(category)
        || matches!(category, GeneralCategory::DecimalNumber | GeneralCategory::ConnectorPunctuation)
}

pub fn mention(point: u32) -> bool {
    char::from_u32(point).is_some_and(|char| char.is_ascii_alphanumeric() || char == '_')
        || point == 0x17F
        || point == 0x212A
}
