# Lindera SudachiDict

Lindera SudachiDict は、Sudachi 形態素解析器が使用する、活発にメンテナンスされている辞書 [SudachiDict](https://github.com/WorksApplications/SudachiDict) に基づく日本語辞書クレートです。上流で年に数回更新されているため、最近の語彙（`令和`、`スマホ`、`推し活`、...）が語彙に含まれます。各エントリは 19 フィールドを持ち、正規化表記・分割情報・同義語グループ ID といった SudachiDict 固有のカラムを含みます。

辞書のソースファイルは [lindera/sudachidict](https://github.com/lindera/sudachidict) にミラーされています。同梱バージョンは `20260723`（small + core + notcore レキシコン）です。

> [!NOTE]
> この辞書は SudachiDict の語彙とラティスの挙動を再現しますが、Sudachi のエンジンプラグイン（カタカナ・数詞の連結、入力正規化、A/B/C 分割モード）は再現しません。詳細は [Sudachi との挙動の違い](./sudachidict.md#sudachi-との挙動の違い)を参照してください。

## 目次

- [辞書フォーマット](./lindera-sudachidict/dictionary_format.md) -- システム辞書およびユーザー辞書のフィールド定義
- [ビルド](./lindera-sudachidict/build.md) -- ソースからの辞書ビルド方法
- [使用例](./lindera-sudachidict/examples.md) -- トークナイズの例

## API リファレンス

- [lindera-sudachidict](https://docs.rs/lindera-sudachidict)
