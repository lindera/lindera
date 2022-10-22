use std::borrow::Cow;

use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct Token<'a> {
    pub text: Cow<'a, str>,
    pub details: Option<Vec<String>>,
}
