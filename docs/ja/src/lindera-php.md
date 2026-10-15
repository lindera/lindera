# Lindera PHP

Lindera PHP は、[ext-php-rs](https://github.com/davidcole1340/ext-php-rs) を使用して構築された Lindera 形態素解析エンジンの PHP バインディングです。PHP 8.1 以降をサポートし、Lindera の高性能なトークナイズ機能を PHP エコシステムに提供します。

## 特徴

- **多言語対応**: 日本語（IPADIC、IPADIC NEologd、UniDic）、韓国語（ko-dic）、中国語（CC-CEDICT、Jieba）のテキストをトークナイズ
- **テキスト処理パイプライン**: 文字フィルタとトークンフィルタを組み合わせて、柔軟な前処理・後処理が可能
- **CRF ベースの辞書学習**: アノテーション付きコーパスからカスタム形態素解析モデルを学習（`train` feature が必要）
- **複数のトークナイズモード**: 解析粒度に応じた Normal モードと Decompose モード
- **N-best トークナイズ**: コスト順にランク付けされた複数のトークナイズ候補を取得
- **ユーザー辞書**: システム辞書をカスタム語彙で拡張

## 引数の型

型を宣言した引数は、`declare(strict_types=1)` の有無にかかわらず、その型の値だけを受け付けます。`int` の引数は `int` を受け付け、数字の文字列や float は受け付けません。`?Metadata` の引数は `Lindera\Metadata` を受け付けます。`float` の引数は `int` も受け付けます。`?int $cost_threshold = null` のような省略可能な引数では、`null` を渡すか引数を省略すると既定値になります。それ以外の値は、引数の名前を含む `Exception` を投げます。コマンドライン引数のように文字列で届く値は、宣言された型にキャストしてから渡してください。

```php
<?php

$dictionary = Lindera\Dictionary::load('embedded://ipadic');
$tokenizer = new Lindera\Tokenizer($dictionary);

$threshold = '5000'; // 例えば $argv や設定ファイルから

try {
    $tokenizer->tokenizeNbest('東京都', 2, cost_threshold: $threshold);
} catch (Exception $e) {
    echo get_class($e), ': ', $e->getMessage(), "\n";
    // Exception: Invalid value given for argument `cost_threshold`.
}

// 宣言された型にキャストする
$results = $tokenizer->tokenizeNbest('東京都', 2, cost_threshold: (int) $threshold);

// null を渡すか引数を省略すると既定値になる
$results = $tokenizer->tokenizeNbest('東京都', 2, cost_threshold: null);
```

`new Lindera\Tokenizer(...)` の `$space_penalty` やフィルタのメソッドの `$args` など、`mixed` と宣言した引数は、この規則の対象外です。どの PHP の値も受け取り、値を自分で検査します。

## ドキュメント

- [インストール](./lindera-php/installation.md) -- 前提条件、ビルド手順、feature フラグ
- [クイックスタート](./lindera-php/quickstart.md) -- 最小限の使用例
- [トークナイザー API](./lindera-php/tokenizer_api.md) -- `TokenizerBuilder`、`Tokenizer`、`Token` クラスリファレンス
- [辞書管理](./lindera-php/dictionary_management.md) -- 辞書の読み込み、ビルド、管理
- [テキスト処理パイプライン](./lindera-php/text_processing_pipeline.md) -- 文字フィルタとトークンフィルタ
- [学習](./lindera-php/training.md) -- カスタム CRF モデルの学習と辞書のエクスポート
