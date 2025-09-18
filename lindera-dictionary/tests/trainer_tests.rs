#[cfg(feature = "train")]
#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use lindera_dictionary::trainer::{Corpus, TrainerConfig};

    #[test]
    fn test_corpus_from_reader() {
        let corpus_data = r#"外国	名詞,一般,*,*,*,*,外国,ガイコク,ガイコク
人	名詞,接尾,一般,*,*,*,人,ジン,ジン
EOS

これ	連体詞,*,*,*,*,*,これ,コレ,コレ
は	助詞,係助詞,*,*,*,*,は,ハ,ワ
EOS
"#;

        let cursor = Cursor::new(corpus_data.as_bytes());
        let corpus = Corpus::from_reader(cursor).unwrap();

        assert_eq!(corpus.len(), 2);
        // Test basic properties without accessing private fields
        // In a real implementation, we would add public accessor methods
    }

    #[test]
    fn test_trainer_config_creation() {
        // Test that TrainerConfig can be created with minimal valid data
        let lexicon_data = "外国,0,0,5000,名詞,一般,*,*,*,*,外国,ガイコク,ガイコク\n人,1,1,5000,名詞,接尾,一般,*,*,*,人,ジン,ジン\n";
        let char_data = "# char.def placeholder\n";
        let unk_data = "# unk.def placeholder\n";
        let feature_data = "UNIGRAM:%F[0]\nLEFT:%L[0]\nRIGHT:%R[0]\n";
        let rewrite_data = "# rewrite.def placeholder\n";

        let result = TrainerConfig::from_readers(
            Cursor::new(lexicon_data.as_bytes()),
            Cursor::new(char_data.as_bytes()),
            Cursor::new(unk_data.as_bytes()),
            Cursor::new(feature_data.as_bytes()),
            Cursor::new(rewrite_data.as_bytes()),
        );

        // Config creation should now succeed with the fixed implementation
        assert!(result.is_ok());
        let config = result.unwrap();
        // Verify that surfaces were extracted correctly using the getter
        assert_eq!(config.surfaces().len(), 2);
        assert!(config.surfaces().contains(&"外国".to_string()));
        assert!(config.surfaces().contains(&"人".to_string()));
    }

    #[test]
    fn test_trainer_creation() {
        use lindera_dictionary::trainer::Trainer;

        // Test that Trainer can be created from a valid config
        let lexicon_data = "外国,0,0,5000,名詞,一般,*,*,*,*,外国,ガイコク,ガイコク\n";
        let char_data = "# char.def placeholder\n";
        let unk_data = "# unk.def placeholder\n";
        let feature_data = "UNIGRAM:%F[0]\nLEFT:%L[0]\nRIGHT:%R[0]\n";
        let rewrite_data = "# rewrite.def placeholder\n";

        let config_result = TrainerConfig::from_readers(
            Cursor::new(lexicon_data.as_bytes()),
            Cursor::new(char_data.as_bytes()),
            Cursor::new(unk_data.as_bytes()),
            Cursor::new(feature_data.as_bytes()),
            Cursor::new(rewrite_data.as_bytes()),
        );

        assert!(config_result.is_ok());
        let config = config_result.unwrap();

        // Test trainer creation with builder pattern
        let trainer = Trainer::new(config)
            .unwrap()
            .regularization_cost(0.01)
            .max_iter(10)
            .num_threads(1);

        // Verify trainer settings using the getters
        assert_eq!(trainer.get_regularization_cost(), 0.01);
        assert_eq!(trainer.get_max_iter(), 10);
        assert_eq!(trainer.get_num_threads(), 1);
    }
}