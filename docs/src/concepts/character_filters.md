# Character Filters

Character filters are pre-processing steps applied to the input text before tokenization. They normalize or transform characters to improve tokenization quality and consistency.

## Available character filters

### unicode_normalize

Applies Unicode normalization to the input text. This is useful for normalizing full-width characters to half-width, or for canonicalizing equivalent Unicode representations.

Supported normalization forms:

| Form | Description |
| --- | --- |
| **NFKC** | Compatibility decomposition followed by canonical composition. Converts full-width alphanumeric characters to half-width and normalizes Katakana variants. |
| **NFC** | Canonical decomposition followed by canonical composition. |
| **NFD** | Canonical decomposition. |
| **NFKD** | Compatibility decomposition. |

### japanese_iteration_mark

Normalizes Japanese iteration marks into their expanded forms. Iteration marks are special characters that indicate the repetition of the preceding character.

| Mark | Name | Example |
| --- | --- | --- |
| 々 | Kanji iteration mark | 人々 (hitobito) |
| ゝ / ゞ | Hiragana iteration marks | いすゞ (isuzu) |
| ヽ / ヾ | Katakana iteration marks | バナナヽ |

The filter accepts two boolean parameters: whether to normalize Hiragana iteration marks and whether to normalize Katakana iteration marks.

### mapping

Performs character-level string replacement based on a user-defined mapping table. This can be used for custom normalization rules.

For example, mapping "リンデラ" to "Lindera".

## YAML configuration example

When using Lindera with a YAML configuration file, character filters can be specified in the `character_filters` section:

```yaml
segmenter:
  mode: normal
  dictionary: "embedded://ipadic"

character_filters:
  - kind: unicode_normalize
    args:
      kind: nfkc
  - kind: japanese_iteration_mark
    args:
      normalize_kanji: true
      normalize_kana: true
  - kind: mapping
    args:
      mapping:
        リンデラ: Lindera
```

## Rust API example

Character filters can be created and appended to a `Tokenizer` programmatically:

```rust
use lindera::character_filter::BoxCharacterFilter;
use lindera::character_filter::unicode_normalize::{
    UnicodeNormalizeCharacterFilter, UnicodeNormalizeKind,
};
use lindera::character_filter::japanese_iteration_mark::JapaneseIterationMarkCharacterFilter;
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::tokenizer::Tokenizer;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    let dictionary = load_dictionary("embedded://ipadic")?;
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);

    // Create character filters.
    let unicode_normalize_char_filter =
        UnicodeNormalizeCharacterFilter::new(UnicodeNormalizeKind::NFKC);

    let japanese_iteration_mark_char_filter =
        JapaneseIterationMarkCharacterFilter::new(true, true);

    // Create a tokenizer and append character filters.
    let mut tokenizer = Tokenizer::new(segmenter);

    tokenizer
        .append_character_filter(BoxCharacterFilter::from(unicode_normalize_char_filter))
        .append_character_filter(BoxCharacterFilter::from(
            japanese_iteration_mark_char_filter,
        ));

    // Tokenize text -- full-width "Ｌｉｎｄｅｒａ" will be normalized to "Lindera".
    let text = "Ｌｉｎｄｅｒａは形態素解析ｴﾝｼﾞﾝです。";
    let tokens = tokenizer.tokenize(text)?;

    for token in tokens {
        println!(
            "token: {:?}, details: {:?}",
            token.surface, token.details
        );
    }

    Ok(())
}
```

Output (with NFKC normalization applied):

```text
token: "Lindera", details: Some(["名詞", "固有名詞", "組織", "*", "*", "*", "*", "*", "*"])
token: "は", details: Some(["助詞", "係助詞", "*", "*", "*", "*", "は", "ハ", "ワ"])
token: "形態素", details: Some(["名詞", "一般", "*", "*", "*", "*", "形態素", "ケイタイソ", "ケイタイソ"])
token: "解析", details: Some(["名詞", "サ変接続", "*", "*", "*", "*", "解析", "カイセキ", "カイセキ"])
token: "エンジン", details: Some(["名詞", "一般", "*", "*", "*", "*", "エンジン", "エンジン", "エンジン"])
token: "です", details: Some(["助動詞", "*", "*", "*", "特殊・デス", "基本形", "です", "デス", "デス"])
token: "。", details: Some(["記号", "句点", "*", "*", "*", "*", "。", "。", "。"])
```
