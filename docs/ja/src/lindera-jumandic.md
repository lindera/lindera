# Lindera JUMAN 辞書

Lindera JUMAN 辞書（`lindera-jumandic`）は、mecab-jumandic 7.0-20130310 から作る日本語辞書クレートです。mecab-jumandic は、形態素解析器 [JUMAN](https://nlp.ist.i.kyoto-u.ac.jp/?JUMAN) の辞書を MeCab の形式にしたものです。辞書の名前は `jumandic` です。

品詞（名詞,普通名詞、動詞、判定詞、接尾辞、特殊,句点）と、活用型・活用形（子音動詞マ行、タ形）は JUMAN のものです。読みはひらがなで、7 番目のフィールドに JUMAN の意味情報（`代表表記:本/ほん 漢字読み:音 カテゴリ:人工物-その他;抽象物`）を持ちます。活用語は語尾まで含めて 1 語になります。IPADIC が 読ん と だ に分けるところが、読んだ の 1 語です。

ソースは、MeCab のリポジトリ（[taku910/mecab](https://github.com/taku910/mecab) の `mecab-jumandic/`）から取得します。ライセンスは 3 条項 BSD です。同じ版の公式の配布物と、ファイル単位で同じ辞書です。

## 目次

- [辞書フォーマット](./lindera-jumandic/dictionary_format.md) -- システム辞書およびユーザー辞書のフィールド定義
- [ビルド](./lindera-jumandic/build.md) -- ソースからの辞書ビルド方法と、飛ばされる 6 行
- [使用例](./lindera-jumandic/examples.md) -- トークナイズの例と、MeCab との一致

## API リファレンス

- [lindera-jumandic](https://docs.rs/lindera-jumandic)
