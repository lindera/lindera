use lindera::core::LinderaResult;

fn main() -> LinderaResult<()> {
    #[cfg(feature = "cc-cedict")]
    {
        use std::borrow::Cow;

        use lindera::core::mode::Mode;
        use lindera::dictionary::{DictionaryConfig, DictionaryKind};
        use lindera::segmenter::{Segmenter, SegmenterConfig};

        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::CcCedict),
            path: None,
        };

        let config = SegmenterConfig {
            dictionary,
            user_dictionary: None,
            mode: Mode::Normal,
        };

        #[allow(unused_variables)]
        let segmenter = Segmenter::from_config(config).unwrap();

        let tokens = segmenter.segment(Cow::Borrowed("可以进行中文形态学分析。"))?;

        for token in tokens {
            println!("{}", token.text);
        }
    }

    Ok(())
}
