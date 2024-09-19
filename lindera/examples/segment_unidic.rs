use lindera::core::LinderaResult;

fn main() -> LinderaResult<()> {
    #[cfg(feature = "unidic")]
    {
        use std::borrow::Cow;

        use lindera::core::mode::Mode;
        use lindera::dictionary::{DictionaryConfig, DictionaryKind};
        use lindera::segmenter::{Segmenter, SegmenterConfig};

        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::UniDic),
            path: None,
        };

        let config = SegmenterConfig {
            dictionary,
            user_dictionary: None,
            mode: Mode::Normal,
        };

        #[allow(unused_variables)]
        let segmenter = Segmenter::from_config(config).unwrap();

        let tokens =
            segmenter.segment(Cow::Borrowed("日本語の形態素解析を行うことができます。"))?;

        for token in tokens {
            println!("{}", token.text);
        }
    }

    Ok(())
}
