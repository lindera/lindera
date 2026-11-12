# Lindera UniDic CSJ

Lindera UniDic CSJ は、国立国語研究所が配布する[現代話し言葉UniDic](https://clrd.ninjal.ac.jp/unidic/) ver.2025.12 に基づく日本語辞書クレートです。話し言葉を書き起こしたテキストのための辞書で、国立国語研究所が日本語日常会話コーパス（CEJC）で学習しています。

見出し語と、1エントリあたり 33 のフィールドは、現代書き言葉UniDic である [UniDic CWJ](./lindera-unidic-cwj.md) と同じです。単語のコストと連接表が違うので、同じテキストでも分かち書きが違うことがあります。話し言葉の書き起こしには UniDic CSJ を、書き言葉には UniDic CWJ を使ってください。

元データは lindera.dev ではなく国立国語研究所の公式の URL から取得し、クレートに固定した SHA-256 と照合します。

> [!NOTE]
> UniDic CSJ は、UniDic CWJ と同じくらい大きい辞書です。組み立てた辞書は 1.2 GB で、そのうち 1,002 MiB が連接表（24,314 × 21,609）です。`embed-unidic-csj` を使うと、実行ファイルはどれもこの 1.2 GB を含み、クレートのコンパイルには、UniDic CWJ と同じく 8〜11 GiB のメモリを使います（macOS で計測したビルドでの値）。WASM のバインディングは `embed-unidic-csj` を提供しません。実行ファイルを小さく保つには、`lindera download unidic-csj` で辞書を取得し、そのディレクトリから読み込んでください。

## 目次

- [辞書フォーマット](./lindera-unidic-csj/dictionary_format.md) -- システム辞書およびユーザー辞書のフィールド定義
- [ビルド](./lindera-unidic-csj/build.md) -- ソースからの辞書ビルド方法
- [使用例](./lindera-unidic-csj/examples.md) -- トークナイズの例

## API リファレンス

- [lindera-unidic-csj](https://docs.rs/lindera-unidic-csj)
