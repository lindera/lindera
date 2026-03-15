# キャラクターフィルター

キャラクターフィルターは、トークナイズ前に入力テキストに適用される前処理ステップです。文字を正規化または変換して、トークナイズの品質と一貫性を向上させます。

## 利用可能なキャラクターフィルター

### unicode_normalize

入力テキストにUnicode正規化を適用します。全角文字を半角に変換したり、等価なUnicode表現を正規化する場合に有用です。

サポートされている正規化形式：

| 形式 | 説明 |
| --- | --- |
| **NFKC** | 互換分解の後に正準合成を行います。全角英数字を半角に変換し、カタカナのバリエーションを正規化します。 |
| **NFC** | 正準分解の後に正準合成を行います。 |
| **NFD** | 正準分解を行います。 |
| **NFKD** | 互換分解を行います。 |

### japanese_iteration_mark

日本語の踊り字（繰り返し記号）を展開形に正規化します。踊り字は直前の文字の繰り返しを示す特殊な文字です。

| 記号 | 名前 | 例 |
| --- | --- | --- |
| 々 | 漢字の踊り字 | 人々 (hitobito) |
| ゝ / ゞ | ひらがなの踊り字 | いすゞ (isuzu) |
| ヽ / ヾ | カタカナの踊り字 | バナナヽ |

このフィルターは2つのブール値パラメータを受け付けます：ひらがなの踊り字を正規化するかどうか、およびカタカナの踊り字を正規化するかどうかです。

### mapping

ユーザー定義のマッピングテーブルに基づいて、文字レベルの文字列置換を実行します。カスタム正規化ルールに使用できます。

例えば、"リンデラ" を "Lindera" にマッピングします。

## YAML設定の例

YAML設定ファイルでLinderaを使用する場合、キャラクターフィルターは`character_filters`セクションで指定できます：

```yaml
segmenter:
  dictionary:
    kind: embedded
    dict: ipadic
  mode: normal

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

## Rust APIの例

キャラクターフィルターはプログラムから作成して`Tokenizer`に追加できます：

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

実行結果は以下のようになります（NFKC正規化適用後）：

```text
token: "Lindera", details: Some(["名詞", "固有名詞", "組織", "*", "*", "*", "*", "*", "*"])
token: "は", details: Some(["助詞", "係助詞", "*", "*", "*", "*", "は", "ハ", "ワ"])
token: "形態素", details: Some(["名詞", "一般", "*", "*", "*", "*", "形態素", "ケイタイソ", "ケイタイソ"])
token: "解析", details: Some(["名詞", "サ変接続", "*", "*", "*", "*", "解析", "カイセキ", "カイセキ"])
token: "エンジン", details: Some(["名詞", "一般", "*", "*", "*", "*", "エンジン", "エンジン", "エンジン"])
token: "です", details: Some(["助動詞", "*", "*", "*", "特殊・デス", "基本形", "です", "デス", "デス"])
token: "。", details: Some(["記号", "句点", "*", "*", "*", "*", "。", "。", "。"])
```
