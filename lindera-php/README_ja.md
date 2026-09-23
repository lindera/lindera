# Lindera PHP

[Lindera](https://github.com/lindera/lindera) の PHP 拡張。CJK テキスト向け形態素解析ライブラリ。

## 動作要件

- PHP 8.1+（Linux または macOS。Windows は非対応）
- ソースからビルドする場合: Rust ツールチェーン（stable）と libclang

## インストール

### PIE を使う（推奨）

この拡張は Packagist に `lindera/lindera` として公開されており（lindera v6.1.0 以降）、PHP 拡張インストーラーの [PIE](https://github.com/php/pie) でインストールします。Composer 自体は `php-ext` パッケージをインストールしないため、`composer require lindera/lindera` ではインストールできません。

```bash
pie install lindera/lindera
```

PIE は使用中の PHP 向けに拡張をソースからビルドします。ビルドには Rust ツールチェーン（<https://rustup.rs/>）、libclang（Debian/Ubuntu は `libclang-dev`、macOS は Xcode コマンドラインツールまたは `brew install llvm`）、および PIE の標準的なビルドツール（`autoconf`、`libtool`、`make`）が必要です。ビルド後、`lindera.so` を拡張ディレクトリにインストールして有効化します。別の PHP を対象にする場合は `--with-php-config=/path/to/php-config` を指定してください。

### 手動ビルド

```bash
git clone https://github.com/lindera/lindera.git
cd lindera
cargo build --release -p lindera-php
```

共有ライブラリは `target/release/liblindera_php.so`（macOS の場合は `.dylib`）に生成されます。`php.ini` に追加するか:

```ini
extension=/path/to/lindera/target/release/liblindera_php.so
```

実行時に読み込みます:

```bash
php -d extension=/path/to/liblindera_php.so your_script.php
```

どちらの方法でも拡張名は `lindera` です。`php -m` には `lindera` と表示され、`extension_loaded('lindera')` で確認できます。

### 辞書

この拡張には辞書は埋め込まれていません。ビルド済み辞書（例: `lindera-ipadic-<version>.zip`）を [GitHub Releases](https://github.com/lindera/lindera/releases) から入手して展開し、そのパスを `Lindera\Dictionary::load()` または `TokenizerBuilder::setDictionary()` に渡してください。自分でビルドする場合は `embed-*` feature（例: `--features embed-ipadic`）で辞書を埋め込み、`embedded://ipadic` として読み込むこともできます。

## 使い方

### 基本的なトークナイズ

```php
$dictionary = Lindera\Dictionary::load('/path/to/ipadic');
$tokenizer = new Lindera\Tokenizer($dictionary, 'normal');
$tokens = $tokenizer->tokenize('関西国際空港');

foreach ($tokens as $token) {
    echo $token->surface . ' [' . implode(',', $token->details) . "]\n";
}
```

### トークンをプレーンデータへ変換

`toArray()` はトークンを連想配列として返します。各フィールドは PHP の
自然な型を保つため、独自のエンコーダなしでシリアライズできます:

```php
$data = $tokens[0]->toArray();
// ['surface' => ..., 'byte_start' => 0, 'byte_end' => 9, 'position' => 0,
//  'word_id' => 12345, 'is_unknown' => false, 'details' => [...]]

echo json_encode(array_map(fn($t) => $t->toArray(), $tokens));
```

`Lindera\Token` は `JsonSerializable` を実装していないため、トークンオブジェクトを
そのまま `json_encode` に渡してもこれらのフィールドは出力されません。上記のように
先に `toArray()` を呼んでください。この拡張は ext-php-rs でビルドされており、
その `#[php_class]` はまだ実装インターフェースを宣言できません
（上流の [ext-php-rs#326](https://github.com/davidcole1340/ext-php-rs/issues/326)）。

### TokenizerBuilder

```php
$builder = new Lindera\TokenizerBuilder();
$builder->setDictionary('/path/to/ipadic');
$builder->setMode('decompose');
$tokenizer = $builder->build();
```

### フィルタの使用

```php
$builder = new Lindera\TokenizerBuilder();
$builder->setDictionary('/path/to/ipadic');
$builder->appendCharacterFilter('unicode_normalize', ['kind' => 'nfkc']);
$builder->appendTokenFilter('japanese_stop_tags', [
    'tags' => ['助詞,格助詞,一般', '助詞,係助詞', '助詞,連体化', '助動詞'],
]);
$tokenizer = $builder->build();
```

### N-Best トークナイズ

```php
$tokenizer = new Lindera\Tokenizer(Lindera\Dictionary::load('/path/to/ipadic'));
$results = $tokenizer->tokenizeNbest('東京都', 3);

foreach ($results as $result) {
    echo "Cost: {$result->cost}\n";
    foreach ($result->tokens as $token) {
        echo "  {$token->surface}\n";
    }
}
```

## API リファレンス

すべてのクラスは `Lindera` 名前空間にあります。この拡張は関数を登録しません。

| クラス | 説明 |
| --- | --- |
| `Lindera\TokenizerBuilder` | トークナイザを作成するためのビルダー |
| `Lindera\Tokenizer` | 形態素解析器 |
| `Lindera\Token` | 解析結果のトークン |
| `Lindera\NbestResult` | N-Best トークナイズ結果 |
| `Lindera\Dictionary` | 形態素辞書: `load()`、`loadUser()`、`build()`、`buildUser()`、`version()` |
| `Lindera\UserDictionary` | ユーザー定義辞書 |
| `Lindera\Mode` | トークナイズモード |
| `Lindera\Penalty` | Decompose モードのペナルティ |
| `Lindera\Metadata` | 辞書メタデータ |
| `Lindera\Schema` | 辞書スキーマ |
| `Lindera\FieldDefinition` | スキーマフィールド定義 |
| `Lindera\FieldType` | スキーマフィールド型 |
| `Lindera\Trainer` | モデル学習: `train()`、`export()`（`train` feature、デフォルトで有効） |

### 静的解析用スタブ

[`stubs/lindera.stubs.php`](stubs/lindera.stubs.php) は、拡張の全クラスを IDE・PHPStan・Psalm 向けに宣言したスタブです。ビルドした拡張から生成され、内容が古くなるとテストが失敗するため、Rust のソースとずれることはありません。ファイルのコピーを解析ツールに指定してください。拡張がクラスを宣言済みのため、実行時に include してはいけません。

```neon
# phpstan.neon
parameters:
    stubFiles:
        - path/to/lindera/lindera-php/stubs/lindera.stubs.php
```

## 開発

```bash
make test-lindera-php      # cargo test + 拡張のビルド + PHPUnit
make test-lindera-php-pie  # PIE が実行する phpize / configure / make の経路
make stubs-lindera-php     # API 変更後に stubs/lindera.stubs.php を再生成
make build-lindera-php     # リリースビルド
```

`composer.json` はこのディレクトリではなくリポジトリのルートにあります。Packagist はルートの composer.json を読み、その `php-ext.build-path` が PIE を `lindera-php/` に向けます。そこで `config.m4` と `Makefile.frag` が `cargo build` を実行します。

## ライセンス

MIT
