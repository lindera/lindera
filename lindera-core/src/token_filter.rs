use crate::{token::Token, LinderaResult};

pub trait TokenFilter {
    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()>;
}
