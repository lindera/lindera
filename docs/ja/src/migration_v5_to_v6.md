# v5 から v6 への移行

Lindera v6.0.0 では、ビルド済み辞書のオンディスクフォーマットが変わります。システム
前方一致辞書のバイト単位 `daachorse` Aho-Corasick オートマトン（`dict.da`）が、文字単位の
ダブル配列トライ（`dict.trie`）と単語値インデックス（`dict.valsidx`）に置き換えられました。
また `metadata.json` がフォーマットバージョンを記録するようになり、バージョンが一致しない
辞書は黙って誤読される代わりに、ロード時に明確なエラーで拒否されます。トークナイズ結果は
変わりません — 同じ入力と辞書に対して、v6 は v5 とバイト単位で同一のトークンを出力します。

## 概要

| 変更 | 影響範囲 | 対処方法 |
| --- | --- | --- |
| ビルド済み辞書フォーマットがバージョン 2 に（`dict.da` → `dict.trie` + `dict.valsidx`） | 自前ビルド辞書 | `lindera build` で再ビルド |
| `metadata.json` が `format_version` を記録し、不一致はロード時に拒否 | v5 世代のビルド済み辞書を読み込むすべてのユーザー | 再ビルド、または `lindera download` で再取得 |
| `loadDictionaryFromBytes()` が 9 引数に | バイトデータから辞書を読み込む WASM ユーザー | `dictDa` の代わりに `dictTrie` と `dictValsIdx` を渡す |
| OPFS の `DictionaryFiles` が `dictDa` を `dictTrie` + `dictValsIdx` に置き換え | `opfs` ヘルパーを使う WASM ユーザー | OPFS にキャッシュ済みの辞書を再ダウンロード |

## 辞書フォーマットバージョン 2

システム前方一致辞書は、シリアライズされた `daachorse` Aho-Corasick オートマトン
（`dict.da`）として保存されなくなりました。ビルダはビルド時に `crawdad` で文字単位の
ダブル配列トライを構築して `dict.trie` として書き出し、単語値への `u32` 累積和
インデックス（`dict.valsidx`）を併せて出力します。実行時にはトライをシリアライズ済み
バイト列上で直接走査するため、ロードはデシリアライズではなく O(1) のヘッダ検査で済み、
`mmap` 下では他の大きなコンポーネントと同様に遅延ページングされたままになります。

ビルド済み辞書ディレクトリは以下の 9 ファイルで構成されます:

| ファイル | 説明 |
| --- | --- |
| `metadata.json` | 辞書メタデータ（`format_version` を含むようになった） |
| `dict.trie` | 文字単位のダブル配列トライ構造（新規） |
| `dict.valsidx` | 単語値インデックス（新規） |
| `dict.vals` | 単語値データ |
| `dict.wordsidx` | 単語詳細インデックス |
| `dict.words` | 単語詳細（形態素素性） |
| `matrix.mtx` | 連接コスト行列 |
| `char_def.bin` | 文字カテゴリ定義 |
| `unk.bin` | 未知語辞書 |

`dict.trie` と `dict.valsidx` 以外のファイルは v5 から変更ありません。`dict.da` は
存在しなくなりました。

### 古い辞書のロードは明確なエラーで失敗する

ビルド済み辞書の `metadata.json` は `format_version: 2` を記録し、ローダーがこれを検証
します。v5 でビルドした辞書のロードは、ヘッダを持たないバイナリファイルを誤読する
代わりに、対処方法を含むエラーで失敗します:

```text
Dictionary 'ipadic' has format version 1, but this build of Lindera reads format version 2. To fix this, rebuild it with `lindera build`, or download a matching prebuilt dictionary with `lindera download`.
```

## 対応が必要なケース

### 自前ビルド辞書

自分でソースファイルからコンパイルした辞書は、v6 の CLI で再ビルドしてください。
コマンドラインインターフェースは変わっていないため、以前と同じ `lindera build` の
呼び出しで再ビルドできます:

```sh
lindera build \
  --src ./dictionary-source \
  --dest ./dictionary \
  --metadata ./metadata.json
```

または、対応するビルド済み辞書を再ダウンロードしてください:

```sh
lindera download ipadic
```

ダウンロード済み辞書は `<データディレクトリ>/lindera/dictionaries/<バージョン>/`
（例: Linux では `~/.local/share/lindera/dictionaries/6.0.0/`）にバージョン単位で
隔離されるため、v5 のコピーが再利用・上書きされることはなく、掃除も必須では
ありません。

### WASM: `loadDictionaryFromBytes()`

従来の `dictDa` 引数が `dictTrie` と `dictValsIdx` に置き換えられ、シグネチャが
8 引数から 9 引数になりました:

```javascript
// v5 — 8 引数
const dictionary = loadDictionaryFromBytes(
    files.metadata, files.dictDa, files.dictVals, files.dictWordsIdx,
    files.dictWords, files.matrixMtx, files.charDef, files.unk,
);

// v6 — 9 引数
const dictionary = loadDictionaryFromBytes(
    files.metadata, files.dictTrie, files.dictValsIdx, files.dictVals,
    files.dictWordsIdx, files.dictWords, files.matrixMtx, files.charDef, files.unk,
);
```

### WASM: OPFS ヘルパー

`loadDictionaryFiles()` が返す `DictionaryFiles` オブジェクトは、`dictDa` の代わりに
`dictTrie` と `dictValsIdx` プロパティを持つようになり、OPFS に保存されるファイル
一式も同様に変わりました。v5 で OPFS にダウンロードした辞書には `dict.trie` と
`dict.valsidx` が無いため、削除して v6 のアーカイブをダウンロードしてください:

```javascript
import { downloadDictionary, removeDictionary } from 'lindera-wasm-web/opfs';

await removeDictionary("ipadic");
await downloadDictionary(DICT_URL_V6, "ipadic");
```

## 対応が不要なケース

- **ユーザー辞書**: ビルド済みユーザー辞書の `.bin` ファイルは影響を受けません。
  内部では引き続き `daachorse` オートマトンを使用しており、再ビルドせずにそのまま
  ロードできます。`.csv` から読み込むユーザー辞書も、従来どおりロード時に
  コンパイルされます。
- **トークナイズ結果**: 変わりません。同じ入力と辞書に対して、v6 は v5 とバイト
  単位で同一のトークンを出力します。
- **Rust API**: シグネチャの変更はありません。パスや URI から辞書をロードする
  コードは、辞書自体を再ビルドまたは再ダウンロードすればそのままコンパイル・動作
  します。`embed-*` の埋め込み辞書は各辞書クレートのビルドスクリプトがコンパイル
  するため、常に実行中のバージョンと一致します。

## アップグレードチェックリスト

- 自前ビルドのシステム辞書を v6 の `lindera build` で再ビルドするか、`lindera
  download` でビルド済み辞書を再取得する。
- WASM: `loadDictionaryFromBytes()` の呼び出しを 9 引数のシグネチャに更新する
  （`dictDa` の代わりに `dictTrie` と `dictValsIdx`）。
- WASM: v5 世代の辞書を OPFS から削除し、v6 のアーカイブをダウンロードする。
- ユーザー辞書の `.bin` ファイルは対応不要。出力の再検証も不要。
