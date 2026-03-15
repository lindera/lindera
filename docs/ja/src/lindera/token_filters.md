# Token Filter

Token Filter はセグメンテーション後のトークンに適用される後処理コンポーネントです。検索インデックス作成、テキスト正規化、言語解析などの用途に合わせて、トークンの変更、削除、変換を行うことができます。

## 利用可能な Token Filter

### 日本語

| Filter | 説明 |
| -------- | ------ |
| `japanese_compound_word` | 指定した品詞タグに一致する連続トークンを複合語に結合 |
| `japanese_number` | 日本語の数値表現を正規化（例: 漢数字の変換） |
| `japanese_stop_tags` | 指定した品詞タグを持つトークンを除去 |
| `japanese_katakana_stem` | カタカナ語の末尾の長音記号を除去してステミング |
| `japanese_base_form` | トークンを原形（辞書形）に正規化 |
| `japanese_keep_tags` | 指定した品詞タグに一致するトークンのみを保持し、それ以外を除去 |
| `japanese_reading_form` | トークンテキストを読み形式（カタカナ）に変換 |
| `japanese_kana` | ひらがなとカタカナを相互変換 |

### 韓国語

| Filter | 説明 |
| -------- | ------ |
| `korean_stop_tags` | 指定した品詞タグを持つ韓国語トークンを除去 |
| `korean_keep_tags` | 指定した品詞タグに一致する韓国語トークンのみを保持 |
| `korean_reading_form` | 韓国語トークンを読み形式に変換 |

### 汎用

| Filter | 説明 |
| -------- | ------ |
| `lowercase` | トークンテキストを小文字に変換 |
| `uppercase` | トークンテキストを大文字に変換 |
| `mapping` | ユーザー定義のマッピングテーブルに従ってトークンテキストを変換 |
| `length` | テキスト長（最小値および/または最大値）でトークンをフィルタリング |
| `stop_words` | ストップワードリストに一致するトークンを除去 |
| `keep_words` | 指定した単語リストに一致するトークンのみを保持 |
| `remove_diacritical_mark` | トークンテキストからダイアクリティカルマーク（アクセント記号）を除去 |

## YAML設定

Token Filter はYAML設定ファイルの `token_filters` キーで設定できます：

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

Token Filter はプログラムから作成・適用することもできます：

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

    // Token Filter を追加
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

    // フィルタを適用してトークナイズ
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

`append_token_filter` メソッドはフィルタを順番に追加します。フィルタはセグメンテーション後のトークンリストに対して順次適用されます。
