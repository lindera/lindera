# Token Filters

Token filters are post-processing components applied to tokens after segmentation. They can modify, remove, or transform tokens to suit specific use cases such as search indexing, text normalization, or linguistic analysis.

## Available Token Filters

### Japanese

| Filter | Description |
| -------- | ------------- |
| `japanese_compound_word` | Combines consecutive tokens matching specified part-of-speech tags into compound words |
| `japanese_number` | Normalizes Japanese number representations (e.g., converts Kanji numerals) |
| `japanese_stop_tags` | Removes tokens with specified part-of-speech tags |
| `japanese_katakana_stem` | Stems Katakana words by removing trailing prolonged sound marks |
| `japanese_base_form` | Normalizes tokens to their base (dictionary) form |
| `japanese_keep_tags` | Keeps only tokens matching specified part-of-speech tags, removing all others |
| `japanese_reading_form` | Converts token text to its reading form (Katakana) |
| `japanese_kana` | Converts between Hiragana and Katakana |

### Korean

| Filter | Description |
| -------- | ------------- |
| `korean_stop_tags` | Removes Korean tokens with specified part-of-speech tags |
| `korean_keep_tags` | Keeps only Korean tokens matching specified part-of-speech tags |
| `korean_reading_form` | Converts Korean tokens to their reading form |

### General

| Filter | Description |
| -------- | ------------- |
| `lowercase` | Converts token text to lowercase |
| `uppercase` | Converts token text to uppercase |
| `mapping` | Maps token text according to a user-defined mapping table |
| `length` | Filters tokens by text length (minimum and/or maximum) |
| `stop_words` | Removes tokens matching a list of stop words |
| `keep_words` | Keeps only tokens matching a list of specified words |
| `remove_diacritical_mark` | Removes diacritical marks (accent marks) from token text |

## YAML Configuration

Token filters can be configured in the YAML configuration file under the `token_filters` key:

```yaml
token_filters:
  - kind: "japanese_stop_tags"
    args:
      tags:
        - "助詞"
        - "助動詞"
        - "記号"
  - kind: "japanese_katakana_stem"
    args:
      min: 3
  - kind: "lowercase"
  - kind: "length"
    args:
      min: 2
```

## Rust API

Token filters can also be created and applied programmatically:

```rust
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::token_filter::BoxTokenFilter;
use lindera::token_filter::japanese_stop_tags::JapaneseStopTagsTokenFilter;
use lindera::token_filter::japanese_katakana_stem::JapaneseKatakanaStemTokenFilter;
use lindera::tokenizer::Tokenizer;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    let dictionary = load_dictionary("embedded://ipadic")?;
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);

    let mut tokenizer = Tokenizer::new(segmenter);

    // Add token filters
    let stop_tags_filter = JapaneseStopTagsTokenFilter::new(
        vec![
            "助詞".to_string(),
            "助動詞".to_string(),
            "記号".to_string(),
        ]
        .into_iter()
        .collect(),
    );
    tokenizer.append_token_filter(BoxTokenFilter::from(stop_tags_filter));

    let katakana_stem_filter = JapaneseKatakanaStemTokenFilter::new(3);
    tokenizer.append_token_filter(BoxTokenFilter::from(katakana_stem_filter));

    // Tokenize with filters applied
    let tokens = tokenizer.tokenize("Linderaは形態素解析エンジンです。")?;

    for token in tokens {
        println!(
            "token: {:?}, details: {:?}",
            token.surface, token.details
        );
    }

    Ok(())
}
```

The `append_token_filter` method adds filters in order. Filters are applied sequentially to the token list after segmentation.
