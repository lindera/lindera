use lindera::core::LinderaResult;

fn main() -> LinderaResult<()> {
    #[cfg(feature = "ko-dic")]
    {
        use std::borrow::Cow;

        use lindera::core::mode::Mode;
        use lindera::dictionary::{DictionaryConfig, DictionaryKind};
        use lindera::segmenter::{Segmenter, SegmenterConfig};

        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::KoDic),
            path: None,
        };

        let config = SegmenterConfig {
            dictionary,
            user_dictionary: None,
            mode: Mode::Normal,
        };

        #[allow(unused_variables)]
        let segment = Segmenter::from_config(config).unwrap();

        let tokens = segment.segment(Cow::Borrowed("한국어의형태해석을실시할수있습니다."))?;

        for token in tokens {
            println!("{}", token.text);
        }
    }

    Ok(())
}
