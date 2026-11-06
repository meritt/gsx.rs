use crate::syntax::construct;
use crate::syntax::subtokenize::Subresult;
use crate::syntax::tokenizer::Tokenizer;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Name {
    Label,
    Attention,
    GfmTable,
    HeadingAtx,
    HeadingSetext,
    ListItem,
    Content,
    Data,
    String,
    Text,
    GsxPaired,
}

pub fn call(tokenizer: &mut Tokenizer, name: Name) -> Option<Subresult> {
    match name {
        Name::Label => construct::label_end::resolve(tokenizer),
        Name::Attention => construct::attention::resolve(tokenizer),
        Name::GfmTable => construct::gfm_table::resolve(tokenizer),
        Name::HeadingAtx => construct::heading_atx::resolve(tokenizer),
        Name::HeadingSetext => construct::heading_setext::resolve(tokenizer),
        Name::ListItem => construct::list_item::resolve(tokenizer),
        Name::Content => construct::content::resolve(tokenizer),
        Name::Data => construct::partial_data::resolve(tokenizer),
        Name::String => construct::string::resolve(tokenizer),
        Name::Text => construct::text::resolve(tokenizer),
        Name::GsxPaired => construct::gsx_paired::resolve(tokenizer),
    }
}
