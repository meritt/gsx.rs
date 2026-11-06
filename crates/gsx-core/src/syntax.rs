mod budget;
mod construct;
mod event;
mod parser;
mod resolve;
mod state;
mod subtokenize;
mod tokenizer;
mod util;

pub(crate) use event::{Event, Kind, Name};
pub(crate) use util::char::{Kind as CharKind, classify, whitespace};
pub(crate) use util::character_reference::{
    decode as decode_character_reference, value_max as character_reference_size,
};
pub(crate) use util::infer::{gfm_table_align, list_item_loose, list_loose};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Align {
    None,
    Left,
    Right,
    Center,
}

pub(crate) use construct::gsx_media::candidate_end as media_candidate_end;

pub(crate) use budget::Exceeded;

pub(crate) fn tokenize(text: &str) -> Result<Vec<Event>, Exceeded> {
    parser::parse(text)
}
