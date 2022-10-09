use crate::{token::Token, LinderaResult};

pub trait TokenFilter {
    fn apply<'a>(&self, tokens: Vec<Token<'a>>) -> LinderaResult<Vec<Token<'a>>>;
}
