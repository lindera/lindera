# Lindera PHP

[Lindera](https://github.com/lindera/lindera) の PHP バインディング。CJK テキスト向け形態素解析ライブラリ。

## 動作要件

- PHP 8.1+
- Rust ツールチェーン（stable）

## ビルド

```bash
cargo build -p lindera-php --features embed-ipadic
```

共有ライブラリは `target/debug/liblindera_php.so`（macOS の場合は `.dylib`）に生成されます。

## インストール

共有ライブラリを PHP の拡張機能ディレクトリにコピーします:

```bash
cp target/release/liblindera_php.so $(php -r 'echo ini_get("extension_dir");')/lindera_php.so
```

`php.ini` で有効化します:

```ini
extension=lindera_php.so
```

または、実行時に読み込みます:

```bash
php -d extension=target/debug/liblindera_php.so your_script.php
```

## 使い方

### 基本的なトークナイズ

```php
$tokenizer = (new Lindera\TokenizerBuilder())->build();
$tokens = $tokenizer->tokenize("関西国際空港");

foreach ($tokens as $token) {
    echo $token->surface . " [" . implode(",", $token->details) . "]\n";
}
```

### 辞書の指定

```php
$dict = Lindera\load_dictionary("ipadic");
$tokenizer = new Lindera\Tokenizer($dict, "normal");
$tokens = $tokenizer->tokenize("すもももももももものうち");
```

### Decompose モード

```php
$builder = new Lindera\TokenizerBuilder();
$builder->set_mode("decompose");
$tokenizer = $builder->build();
```

### フィルタの使用

```php
$builder = new Lindera\TokenizerBuilder();
$builder->set_mode("normal");
$builder->append_character_filter("unicode_normalize", ["kind" => "nfkc"]);
$builder->append_token_filter("japanese_stop_tags", ["tags" => ["助詞"]]);
$tokenizer = $builder->build();
```

### N-Best トークナイズ

```php
$tokenizer = (new Lindera\TokenizerBuilder())->build();
$results = $tokenizer->tokenize_nbest("東京都", 3);

foreach ($results as $result) {
    echo "Cost: {$result->cost}\n";
    foreach ($result->tokens as $token) {
        echo "  {$token->surface}\n";
    }
}
```

## API リファレンス

### クラス

| クラス | 説明 |
|-------|-------------|
| `Lindera\TokenizerBuilder` | トークナイザを作成するためのビルダー |
| `Lindera\Tokenizer` | 形態素解析器 |
| `Lindera\Token` | 解析結果のトークン |
| `Lindera\NbestResult` | N-Best トークナイズ結果 |
| `Lindera\Dictionary` | 形態素辞書 |
| `Lindera\UserDictionary` | ユーザー定義辞書 |
| `Lindera\Mode` | トークナイズモード |
| `Lindera\Penalty` | Decompose モードのペナルティ |
| `Lindera\Metadata` | 辞書メタデータ |
| `Lindera\Schema` | 辞書スキーマ |
| `Lindera\FieldDefinition` | スキーマフィールド定義 |
| `Lindera\FieldType` | スキーマフィールド型 |
| `Lindera\CompressionAlgorithm` | 圧縮アルゴリズム |

### 関数

| 関数 | 説明 |
|----------|-------------|
| `Lindera\version()` | パッケージバージョンを返す |
| `Lindera\load_dictionary(uri)` | 辞書を読み込む |
| `Lindera\load_user_dictionary(uri, metadata)` | ユーザー辞書を読み込む |
| `Lindera\build_dictionary(input, output, metadata)` | 辞書をビルドする |
| `Lindera\build_user_dictionary(kind, input, output, metadata?)` | ユーザー辞書をビルドする |
| `Lindera\train(...)` | モデルを学習する（train feature） |
| `Lindera\export(model, output, metadata?)` | 辞書ファイルをエクスポートする |

## ライセンス

MIT
