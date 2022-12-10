use crate::{token::Token, LinderaResult};

pub trait TokenFilter {
    fn name(&self) -> &str;
    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()>;
}
