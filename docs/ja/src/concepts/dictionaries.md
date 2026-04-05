# 辞書

Linderaは、日本語・韓国語・中国語の形態素解析のための様々な辞書をサポートしています。各辞書は個別のクレートとして提供されます。

| 辞書 | 言語 | クレート | 説明 |
| --- | --- | --- | --- |
| [IPADIC](../lindera-ipadic.md) | 日本語 | `lindera-ipadic` | 日本語で最も一般的な辞書 |
| [IPADIC NEologd](../lindera-ipadic-neologd.md) | 日本語 | `lindera-ipadic-neologd` | 新語に対応したIPADIC |
| [UniDic](../lindera-unidic.md) | 日本語 | `lindera-unidic` | 均一な単語単位定義を持つ辞書 |
| [ko-dic](../lindera-ko-dic.md) | 韓国語 | `lindera-ko-dic` | 韓国語の形態素解析 |
| [CC-CEDICT](../lindera-cc-cedict.md) | 中国語 | `lindera-cc-cedict` | 中英辞書 |
| [Jieba](../lindera-jieba.md) | 中国語 | `lindera-jieba` | Jiebaベースの中国語辞書 |

## 辞書の入手方法

ビルド済み辞書は [GitHub Releases](https://github.com/lindera/lindera/releases) からダウンロードできます。対象言語の辞書アーカイブをダウンロードし、ローカルディレクトリに展開してください。

```rust
// ローカルパスから外部辞書を読み込む
let dictionary = load_dictionary("/path/to/ipadic")?;
```

> [!TIP]
> 外部辞書ファイルなしの自己完結型バイナリが必要な場合は、`embed-*` feature フラグを使って辞書を埋め込み、`embedded://` スキームでロードできます：
>
> ```rust
> let dictionary = load_dictionary("embedded://ipadic")?;
> ```
>
> 詳細は [Feature フラグ](../development/feature_flags.md) を参照してください。

各辞書クレートのドキュメントで、フォーマット詳細、ビルド手順、使用例を参照してください。
