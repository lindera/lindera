use std::ops::Deref;

use crate::{token::Token, LinderaResult};

pub trait TokenFilter: 'static + Send + Sync + TokenFilterClone {
    fn name(&self) -> &str;
    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()>;
}

pub struct BoxTokenFilter(Box<dyn TokenFilter + 'static + Send + Sync>);

impl Deref for BoxTokenFilter {
    type Target = dyn TokenFilter;

    fn deref(&self) -> &dyn TokenFilter {
        &*self.0
    }
}

impl<T: TokenFilter> From<T> for BoxTokenFilter {
    fn from(token_filter: T) -> BoxTokenFilter {
        BoxTokenFilter(Box::new(token_filter))
    }
}

pub trait TokenFilterClone {
    fn box_clone(&self) -> BoxTokenFilter;
}

impl<T: TokenFilter + Clone + 'static> TokenFilterClone for T {
    fn box_clone(&self) -> BoxTokenFilter {
        BoxTokenFilter::from(self.clone())
    }
}
