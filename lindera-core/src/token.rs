use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct Token<'a> {
    pub text: &'a str,
    pub details: Option<Vec<String>>,
}
