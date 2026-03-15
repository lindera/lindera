# 貢献ガイド

Lindera への貢献に興味をお持ちいただきありがとうございます。このページでは、貢献を始めるためのガイドラインを紹介します。

## はじめに

1. GitHub でリポジトリをフォークします。
2. フォークをローカルにクローンします：

    ```bash
    git clone https://github.com/<your-username>/lindera.git
    cd lindera
    ```

3. feature ブランチを作成します：

    ```bash
    git checkout -b feature/my-feature
    ```

4. 変更を行い、すべてのチェックに通ることを確認します：

    ```bash
    cargo fmt --all -- --check
    cargo clippy -- -D warnings
    cargo test
    ```

5. 変更をコミットしてプッシュし、プルリクエストを開きます。

## コードスタイル

- リポジトリの既存のコードスタイルに従ってください。
- コミット前に `cargo fmt` を実行してください。
- すべての public および private アイテム（型、関数、モジュール、フィールド、定数、型エイリアス）にドキュメントコメント（`///`）を記述してください。
- trait 実装メソッドにも、実装固有の振る舞いを説明するドキュメントコメントを記述してください。
- 関数・メソッドのドキュメントには、該当する場合 `# Arguments` と `# Returns` セクションを含めてください。
- コードコメント、ドキュメントコメント、コミットメッセージ、ログメッセージ、エラーメッセージは英語で記述してください。
- 本番コードでは `unwrap()` や `expect()` を避けてください（テストコードでは使用可）。
- `unsafe` ブロックは必要な場合にのみ使用し、必ず `// SAFETY: ...` コメントを付けてください。
- モジュールは `mod.rs` スタイルではなく、ファイルベースのスタイル（`src/tokenizer.rs`）を使用してください。

## テスト

- すべての新機能にユニットテストを作成してください。
- 開発中は迅速なフィードバックのために関連するテストのみを実行してください：

    ```bash
    cargo test -p <crate> <test_name>
    ```

- `train` feature に関連する作業では、feature フラグを含めてください：

    ```bash
    cargo test -p lindera-dictionary --features train
    ```

## コミットメッセージ

[Conventional Commits](https://www.conventionalcommits.org/) の仕様に従ってください。コミットメッセージは英語で記述してください。

例：

- `feat: add Korean dictionary support`
- `fix: correct character category ID in trainer`
- `docs: update installation instructions`
- `refactor: split large training method into smaller functions`

## ドキュメント

- 変更がユーザー向けドキュメントに影響する場合は、`docs/src/` 配下の関連ファイルを更新してください。
- Markdown ファイルの編集後は、リントエラーがないことを確認してください：

    ```bash
    markdownlint-cli2 "docs/src/**/*.md"
    ```

- ルールはリポジトリルートの `.markdownlint.json` で設定されています。

## 依存関係

新しい依存関係を追加する際は、ライセンスの互換性を確認してください。Lindera は MIT / Apache-2.0 デュアルライセンスを使用しています。

## Feature フラグ

学習関連コードの条件コンパイルには `#[cfg(feature = "train")]` を使用してください。完全なリストは [Feature フラグ](./feature_flags.md) を参照してください。

## 問題の報告

バグを報告する際は、以下の情報を含めてください：

- Lindera のバージョン（`lindera --version` または `Cargo.toml` を確認）
- Rust のバージョン（`rustc --version`）
- オペレーティングシステム
- 問題の再現手順
- 期待される動作と実際の動作
