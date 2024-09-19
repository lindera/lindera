use lindera::core::LinderaResult;

fn main() -> LinderaResult<()> {
    #[cfg(feature = "cc-cedict")]
    {
        use std::borrow::Cow;
        use std::path::PathBuf;

        use lindera::core::mode::Mode;
        use lindera::dictionary::{DictionaryConfig, DictionaryKind, UserDictionaryConfig};
        use lindera::segmenter::{Segmenter, SegmenterConfig};

        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::CcCedict),
            path: None,
        };

        let user_dictionary = Some(UserDictionaryConfig {
            kind: Some(DictionaryKind::CcCedict),
            path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../resources")
                .join("cc-cedict_simple_userdic.csv"),
        });

        let config = SegmenterConfig {
            dictionary,
            user_dictionary,
            mode: Mode::Normal,
        };

        #[allow(unused_variables)]
        let segmenter = Segmenter::from_config(config).unwrap();

        let tokens = segmenter.segment(Cow::Borrowed("羽田机场限定托特包。"))?;

        for token in tokens {
            println!("{}", token.text);
        }
    }

    Ok(())
}
