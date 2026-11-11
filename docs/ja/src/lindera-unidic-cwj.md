# Lindera UniDic CWJ

Lindera UniDic CWJ は、国立国語研究所が配布する[現代書き言葉UniDic](https://clrd.ninjal.ac.jp/unidic/) ver.2025.12（UniDic 3.1 の後継の版）に基づく日本語辞書クレートです。1エントリあたり 33 フィールドを持ち、[UniDic 2.1.2](./lindera-unidic.md) より 12 フィールド多く、仮名形・アクセント・語彙表 ID・語彙素 ID などを含みます。

元データは lindera.dev ではなく国立国語研究所の公式の URL から取得し、クレートに固定した SHA-256 と照合します。UniDic 2.1.2 は、引き続き `lindera-unidic` が提供します。

> [!NOTE]
> UniDic CWJ は、ほかの辞書よりずっと大きい辞書です。組み立てた辞書は 1.2 GB で、そのうち 982 MiB が連接表（24,082 × 21,375）です。`embed-unidic-cwj` を使うと、実行ファイルはどれもこの 1.2 GB を含み、クレートのコンパイルには 11 GiB ほどのメモリを使います（macOS で計測）。WASM のバインディングは `embed-unidic-cwj` を提供しません。実行ファイルを小さく保つには、`lindera download unidic-cwj` で辞書を取得し、そのディレクトリから読み込んでください。

## 目次

- [辞書フォーマット](./lindera-unidic-cwj/dictionary_format.md) -- システム辞書およびユーザー辞書のフィールド定義
- [ビルド](./lindera-unidic-cwj/build.md) -- ソースからの辞書ビルド方法
- [使用例](./lindera-unidic-cwj/examples.md) -- トークナイズの例

## API リファレンス

- [lindera-unidic-cwj](https://docs.rs/lindera-unidic-cwj)
