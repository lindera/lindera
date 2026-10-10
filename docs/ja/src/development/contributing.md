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
    make check-docs
    ```

    > 注意: CI はこの 4 つをすべて検査します。フォーマットを確認し、各クレートに対してそのクレートのテストと同じ feature で clippy（警告はエラー扱い）とテストを実行し、どのクレートでも rustdoc に警告があれば失敗します。`make check-docs` は、ビルドにそれぞれの言語のツールチェーンが必要な言語バインディングを対象にしません。バインディングを変更した場合は、そのバインディングの CI ジョブと同じく `make check-docs-lindera-<binding>` も実行してください。`make lint` と `make test` はクレートごとに同じ feature を使います。詳しくは[ビルドとテスト](build_and_test.md)を参照してください。

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

## 言語バインディングの整合性

Lindera は Python・Node.js・Ruby・PHP・WASM のバインディングを提供しています。バインディングの機能を追加・レビューする際は次の方針に従ってください。

- **機能セットを揃える。** トークナイズ（`tokenize` / `tokenize_surfaces` / `tokenize_nbest`）、ユーザー辞書、文字フィルタ、トークンフィルタは全バインディングで利用できるようにします。あるバインディングに機能を追加したら、他のバインディングの計画（または追跡 Issue）を添えてください。
- **表現と寿命管理は各言語に従う。** トークンをクラスにするかプレーンデータにするか、命名規則（camelCase / snake_case）、変換ヘルパー（`to_dict` / `to_h` / `toArray` / `toJSON`）は言語ごとの判断で、バインディング間で一致させる必要はありません。JS バインディングはホストがクラスのファイナライザを遅延させるためプレーンオブジェクトを返し、Python・Ruby・PHP は解放が決定的なのでクラスを保ちます。

共通ロジックは `lindera-binding`（例: `TokenView`）に置き、各バインディングはコアのデータを自身の FFI 型に写すだけにします。

## テスト

- すべての新機能にユニットテストを作成してください。
- 開発中は迅速なフィードバックのために関連するテストのみを実行してください：

    ```bash
    cargo test -p <crate> <test_name>
    ```

- 学習パイプライン機能に関連する作業では、`lindera-trainer` クレートのテストを実行してください：

    ```bash
    cargo test -p lindera-trainer
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
