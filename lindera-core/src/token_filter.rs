use crate::{token::Token, LinderaResult};

pub trait TokenFilter: 'static + Send + Sync + TokenFilterClone {
    fn name(&self) -> &str;
    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()>;
}

pub trait TokenFilterClone {
    fn box_clone(&self) -> Box<dyn TokenFilter + 'static + Send + Sync>;
}

impl<T: TokenFilter + Clone + 'static> TokenFilterClone for T {
    fn box_clone(&self) -> Box<dyn TokenFilter + 'static + Send + Sync> {
        Box::new(self.clone())
    }
}
