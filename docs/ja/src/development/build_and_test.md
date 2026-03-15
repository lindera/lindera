# ビルドとテスト

## ビルド

### デフォルトビルド

デフォルトの feature（`compress` + `mmap`）でワークスペースをビルドします：

```bash
cargo build
```

### 学習機能付きビルド

CRF ベースの辞書学習機能を含めてビルドします：

```bash
cargo build --features train
```

### CLI のみビルド

```bash
cargo build -p lindera-cli
```

CLI ではデフォルトで `train` feature が有効になっています。

## テスト

### 単一テスト

クレート内の特定のテストを実行します（開発時はこちらを推奨）：

```bash
cargo test -p <crate> <test_name>
```

### 学習機能のテスト

```bash
cargo test -p lindera-dictionary --features train
```

### クレート単位の全機能テスト

単一クレートの全テストスイートを実行します（CI と同等）：

```bash
cargo test -p <crate> --all-features
```

### ワークスペース全体のテスト

```bash
cargo test
```

## 品質チェック

### フォーマットチェック

コードのフォーマットがプロジェクトのスタイルに一致しているか確認します：

```bash
cargo fmt --all -- --check
```

フォーマットを自動修正するには：

```bash
cargo fmt --all
```

### リント

Clippy を警告をエラーとして扱うモードで実行します：

```bash
cargo clippy -- -D warnings
```

## ドキュメント

### API ドキュメント

Rust の API ドキュメントを生成して開きます：

```bash
cargo doc --no-deps --open
```

### mdBook ドキュメント

ユーザー向けドキュメントをビルドします：

```bash
mdbook build docs
```

`http://localhost:3000` でローカルプレビュー：

```bash
mdbook serve docs
```

### Markdown リント

ドキュメントの Markdown スタイルの問題をチェックします：

```bash
markdownlint-cli2 "docs/src/**/*.md"
```

ルールはリポジトリルートの `.markdownlint.json` で設定されています。
