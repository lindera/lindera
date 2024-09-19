use lindera::core::LinderaResult;

fn main() -> LinderaResult<()> {
    #[cfg(feature = "unidic")]
    {
        use std::borrow::Cow;
        use std::path::PathBuf;

        use lindera::core::mode::Mode;
        use lindera::dictionary::{DictionaryConfig, DictionaryKind, UserDictionaryConfig};
        use lindera::segmenter::{Segmenter, SegmenterConfig};

        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::UniDic),
            path: None,
        };

        let user_dictionary = Some(UserDictionaryConfig {
            kind: Some(DictionaryKind::UniDic),
            path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../resources")
                .join("unidic_simple_userdic.csv"),
        });

        let config = SegmenterConfig {
            dictionary,
            user_dictionary,
            mode: Mode::Normal,
        };

        #[allow(unused_variables)]
        let segmenter = Segmenter::from_config(config).unwrap();

        let tokens = segmenter.segment(Cow::Borrowed(
            "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です",
        ))?;

        for token in tokens {
            println!("{}", token.text);
        }
    }

    Ok(())
}
