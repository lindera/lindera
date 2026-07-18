use std::io::{BufRead, BufReader, Read};

use anyhow::Result;

/// Representation of a pair of a surface and features.
pub struct Word {
    surface: String,
    feature: String,
}

impl Word {
    pub fn new(surface: &str, feature: &str) -> Self {
        Self {
            surface: surface.to_string(),
            feature: feature.to_string(),
        }
    }

    /// Returns a surface string.
    pub fn surface(&self) -> &str {
        &self.surface
    }

    /// Returns a concatenated feature string.
    pub fn feature(&self) -> &str {
        &self.feature
    }
}

/// Representation of a sentence.
pub struct Example {
    pub(crate) sentence: String,
    pub(crate) tokens: Vec<Word>,
}

impl Example {
    /// Creates a new example from tokens.
    pub fn new(tokens: Vec<Word>) -> Self {
        let sentence = tokens.iter().map(|t| t.surface()).collect::<String>();
        Self { sentence, tokens }
    }
}

/// A corpus for training.
pub struct Corpus {
    pub(crate) examples: Vec<Example>,
}

impl Default for Corpus {
    fn default() -> Self {
        Self::new()
    }
}

impl Corpus {
    /// Creates a new empty corpus.
    pub fn new() -> Self {
        Self {
            examples: Vec::new(),
        }
    }

    /// Creates a corpus from a reader.
    ///
    /// The format should be compatible with standard tokenized output:
    /// Each line contains a token with tab-separated surface and features.
    /// Sentences are separated by empty lines.
    pub fn from_reader<R: Read>(reader: R) -> Result<Self> {
        let reader = BufReader::new(reader);
        let mut corpus = Self::new();
        let mut current_tokens = Vec::new();

        for line in reader.lines() {
            let line = line?;

            if line.trim().is_empty() || line == "EOS" {
                // End of sentence
                if !current_tokens.is_empty() {
                    corpus.examples.push(Example::new(current_tokens));
                    current_tokens = Vec::new();
                }
            } else {
                // Parse token line: surface\tfeatures
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 2 {
                    let surface = parts[0];
                    let features = parts[1];
                    current_tokens.push(Word::new(surface, features));
                }
            }
        }

        // Add remaining tokens if any
        if !current_tokens.is_empty() {
            corpus.examples.push(Example::new(current_tokens));
        }

        Ok(corpus)
    }

    /// Returns the number of examples in the corpus.
    pub fn len(&self) -> usize {
        self.examples.len()
    }

    /// Returns true if the corpus is empty.
    pub fn is_empty(&self) -> bool {
        self.examples.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

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
}
