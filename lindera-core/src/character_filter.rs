use crate::LinderaResult;

pub trait CharacterFilter {
    fn apply(&self, text: &mut String) -> LinderaResult<()>;
}
