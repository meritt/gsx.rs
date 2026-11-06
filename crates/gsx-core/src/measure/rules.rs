use super::{Fact, Facts, Unit, measure};

use Placement::{Post, Reel, Story};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Field {
    Text,
    Title,
    Description,
    Alt,
}

impl Field {
    pub const fn name(self) -> &'static str {
        match self {
            Field::Text => "text",
            Field::Title => "title",
            Field::Description => "description",
            Field::Alt => "alt",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    Post,
    Reel,
    Story,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaKind {
    Image,
    Video,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Condition {
    Any,
    Without,
    With,
    Only(MediaKind),
}

impl Condition {
    fn matches(self, media: Option<MediaKind>) -> bool {
        match self {
            Condition::Any => true,
            Condition::Without => media.is_none(),
            Condition::With => media.is_some(),
            Condition::Only(kind) => media == Some(kind),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rendering {
    Payload,
    Visible,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Maximum {
    Fixed(u32),
    Resolved(Fact, u32),
    Unlimited,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Context {
    pub placement: Placement,
    pub media: Option<MediaKind>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Measurement {
    pub unit: Unit,
    pub value: u32,
    pub maximum: Option<u32>,
}

impl Measurement {
    pub fn exceeds(&self) -> bool {
        self.maximum.is_some_and(|maximum| self.value > maximum)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limit {
    pub field: Field,
    pub placements: &'static [Placement],
    pub media: Condition,
    pub rendering: Rendering,
    pub unit: Unit,
    pub maximum: Maximum,
}

impl Limit {
    pub fn applies(&self, field: Field, context: Context) -> bool {
        self.field == field && self.placements.contains(&context.placement) && self.media.matches(context.media)
    }

    pub fn maximum(&self, facts: &Facts) -> Option<u32> {
        match self.maximum {
            Maximum::Fixed(value) => Some(value),
            Maximum::Resolved(fact, default) => Some(facts.resolved(fact).unwrap_or(default)),
            Maximum::Unlimited => None,
        }
    }

    pub fn measure(&self, text: &str, facts: &Facts) -> Measurement {
        Measurement { unit: self.unit, value: measure(self.unit, text, facts), maximum: self.maximum(facts) }
    }
}

const ALL: &[Placement] = &[Post, Reel, Story];

pub(crate) const fn limit(field: Field, unit: Unit, maximum: u32) -> Limit {
    Limit {
        field,
        placements: ALL,
        media: Condition::Any,
        rendering: Rendering::Payload,
        unit,
        maximum: Maximum::Fixed(maximum),
    }
}

pub(crate) const fn unlimited(field: Field, unit: Unit) -> Limit {
    Limit { maximum: Maximum::Unlimited, ..limit(field, unit, 0) }
}

pub(crate) const fn resolved(field: Field, unit: Unit, fact: Fact, default: u32) -> Limit {
    Limit { maximum: Maximum::Resolved(fact, default), ..limit(field, unit, 0) }
}

impl Limit {
    pub(crate) const fn on(self, placements: &'static [Placement]) -> Limit {
        Limit { placements, ..self }
    }

    pub(crate) const fn when(self, media: Condition) -> Limit {
        Limit { media, ..self }
    }

    pub(crate) const fn visible(self) -> Limit {
        Limit { rendering: Rendering::Visible, ..self }
    }
}
