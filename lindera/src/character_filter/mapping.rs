use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use lindera_core::character_filter::CharacterFilter;

use crate::{error::LinderaErrorKind, LinderaResult};

pub const MAPPING_CHARACTER_FILTER_NAME: &str = "mapping";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct MappingCharacterFilterConfig {
    pub mapping: HashMap<char, char>,
}

impl MappingCharacterFilterConfig {
    pub fn new(map: HashMap<char, char>) -> Self {
        Self { mapping: map }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        serde_json::from_slice(data).map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }
}

#[derive(Clone, Debug)]
pub struct MappingCharacterFilter {
    config: MappingCharacterFilterConfig,
}

impl MappingCharacterFilter {
    pub fn new(config: MappingCharacterFilterConfig) -> Self {
        Self { config }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        Ok(Self::new(MappingCharacterFilterConfig::from_slice(data)?))
    }
}

impl CharacterFilter for MappingCharacterFilter {
    fn apply(&self, text: &mut String) -> LinderaResult<()> {
        *text = text
            .chars()
            .map(|c| {
                if let Some(ch) = self.config.mapping.get(&c) {
                    *ch
                } else {
                    c
                }
            })
            .collect::<String>();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lindera_core::character_filter::CharacterFilter;

    use crate::character_filter::mapping::{MappingCharacterFilter, MappingCharacterFilterConfig};

    #[test]
    fn test_mapping_character_filter_config_from_slice() {
        let config_str = r#"
        {
            "mapping": {
                "ｱ": "ア",
                "ｲ": "イ",
                "ｳ": "ウ",
                "ｴ": "エ",
                "ｵ": "オ"
            }
        }
        "#;
        let config = MappingCharacterFilterConfig::from_slice(config_str.as_bytes()).unwrap();

        assert_eq!(&'ア', config.mapping.get(&'ｱ').unwrap());
    }

    #[test]
    fn test_mapping_character_filter_from_slice() {
        let config_str = r#"
        {
            "mapping": {
                "ｱ": "ア",
                "ｲ": "イ",
                "ｳ": "ウ",
                "ｴ": "エ",
                "ｵ": "オ"
            }
        }
        "#;
        let result = MappingCharacterFilter::from_slice(config_str.as_bytes());

        assert_eq!(true, result.is_ok());
    }

    #[test]
    fn test_mapping_character_filter_apply() {
        let config_str = r#"
        {
            "mapping": {
                "ｱ": "ア",
                "ｲ": "イ",
                "ｳ": "ウ",
                "ｴ": "エ",
                "ｵ": "オ"
            }
        }
        "#;
        let filter = MappingCharacterFilter::from_slice(config_str.as_bytes()).unwrap();

        let mut text = "ｱｲｳｴｵ".to_string();
        filter.apply(&mut text).unwrap();
        assert_eq!("アイウエオ", text);
    }
}
