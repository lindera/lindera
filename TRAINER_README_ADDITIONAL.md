# Lindera Trainer 高度な機能・技術詳細

この文書では、Lindera Trainerの高度な機能と最新の技術実装について詳細に説明します。

## 高度な未知語処理システム

### 包括的Unicode文字種判定

最新実装では、基本的なUnicode範囲を大幅に拡張し、以下の文字セットを完全サポートしています：

#### **漢字（CJK）の完全対応**

```rust
fn is_kanji(&self, ch: char) -> bool {
    matches!(ch,
        '\u{4E00}'..='\u{9FAF}' |  // CJK Unified Ideographs（基本漢字）
        '\u{3400}'..='\u{4DBF}' |  // CJK Extension A（拡張漢字A）
        '\u{20000}'..='\u{2A6DF}' | // CJK Extension B（拡張漢字B）
        '\u{2A700}'..='\u{2B73F}' | // CJK Extension C（拡張漢字C）
        '\u{2B740}'..='\u{2B81F}' | // CJK Extension D（拡張漢字D）
        '\u{2B820}'..='\u{2CEAF}' | // CJK Extension E（拡張漢字E）
        '\u{2CEB0}'..='\u{2EBEF}' | // CJK Extension F（拡張漢字F）
        '\u{F900}'..='\u{FAFF}' |   // CJK Compatibility Ideographs
        '\u{2F800}'..='\u{2FA1F}'   // CJK Compatibility Supplement
    )
}
```

#### **カタカナの拡張対応**

```rust
fn is_katakana(&self, ch: char) -> bool {
    matches!(ch,
        '\u{30A1}'..='\u{30F6}' |  // Basic Katakana（基本カタカナ）
        '\u{30FD}'..='\u{30FF}' |  // Katakana iteration marks（音引き・促音）
        '\u{31F0}'..='\u{31FF}' |  // Katakana phonetic extensions（音韻拡張）
        '\u{32D0}'..='\u{32FE}' |  // Circled Katakana（丸囲みカタカナ）
        '\u{3300}'..='\u{3357}'    // CJK Compatibility（カタカナ互換文字）
    )
}
```

### 混合文字種の高度な分類アルゴリズム

従来の「最初の文字のみ」の判定から、**全文字の多数決+特殊ルール**ベースの分類に進化：

```rust
fn classify_unknown_word(&self, token: &Word) -> usize {
    let chars: Vec<char> = surface.chars().collect();
    let mut type_counts = [0; 6]; // 各文字種のカウント

    // 全文字をスキャンして文字種をカウント
    for &ch in &chars {
        let char_type = self.get_char_type(ch);
        type_counts[char_type] += 1;
    }

    // 特殊ルール適用
    if type_counts[3] > 0 && type_counts[1] > 0 {
        return 3; // 漢字+ひらがな → 漢字（「食べ物」等の複合語）
    }
    if type_counts[2] > 0 && type_counts[4] > 0 {
        return 2; // カタカナ+英字 → カタカナ（「サッカーGame」等の外来語）
    }

    // 多数決で決定
    most_frequent_type
}
```

### 表層形解析による拡張特徴生成

単語の表層形から豊富な特徴を自動生成：

```rust
fn generate_unknown_word_features(&self, surface: &str, char_type: usize) -> Vec<String> {
    let mut features = Vec::new();

    // 1. 長さベース特徴
    let len = surface.chars().count();
    match len {
        1 => features.push("UNK_LEN=1".to_string()),
        2 => features.push("UNK_LEN=2".to_string()),
        3..=5 => features.push("UNK_LEN=SHORT".to_string()),
        6..=10 => features.push("UNK_LEN=MEDIUM".to_string()),
        _ => features.push("UNK_LEN=LONG".to_string()),
    }

    // 2. 混合文字種検出
    if type_count > 1 {
        features.push("UNK_MIXED=TRUE".to_string());
    }

    // 3. 特殊パターン
    if has_kanji && has_hiragana {
        features.push("UNK_KANJI_HIRA=TRUE".to_string());
    }

    // 4. 位置情報特徴
    features.push(format!("UNK_FIRST={}", self.get_char_type(first_char)));
    features.push(format!("UNK_LAST={}", self.get_char_type(last_char)));

    features
}
```

## 特徴重み最適化

### 多段階正規化システム

学習済みCRFモデルから抽出した重みを、辞書生成に適した形式に変換するための高度な正規化システム：

#### **1. 特徴レベル正規化**

```rust
fn normalize_feature_weight(&self, weight: f64, feature_index: usize) -> f64 {
    let base_normalization = if feature_index < self.config.surfaces.len() {
        weight * 1.0        // 既知語: 標準正規化
    } else {
        weight * 0.8        // 未知語: 過学習防止のため軽減
    };
    base_normalization.clamp(-10.0, 10.0)  // 極値制限
}
```

#### **2. 接続重み正規化**

```rust
fn normalize_connection_weight(&self, weight: f64, left_id: usize, right_id: usize) -> f64 {
    let context_factor = if left_id == right_id {
        1.2 // 同一コンテキスト接続を強化
    } else {
        1.0 // 異なるコンテキストは標準
    };

    let normalized = weight * context_factor;
    normalized.clamp(-8.0, 8.0)  // 接続重み用の範囲制限
}
```

#### **3. グローバル重み正規化**

```rust
fn apply_global_weight_normalization(&self, mut weights: Vec<f64>) -> Vec<f64> {
    let mean_abs_weight = weights.iter().map(|w| w.abs()).sum::<f64>() / weights.len() as f64;

    // 自動スケーリング
    let scale_factor = if mean_abs_weight > 5.0 {
        5.0 / mean_abs_weight       // 大きすぎる重みを縮小
    } else if mean_abs_weight < 0.1 && mean_abs_weight > 0.0 {
        0.1 / mean_abs_weight       // 小さすぎる重みを拡大
    } else {
        1.0                         // スケーリング不要
    };

    if scale_factor != 1.0 {
        for weight in &mut weights {
            *weight *= scale_factor;
        }
    }
    weights
}
```

### i16範囲への重み変換

辞書ファイルで使用される16ビット整数範囲に最適化された重み変換：

```rust
fn calculate_weight_scale_factor(&self, merged_model: &rucrf::MergedModel) -> f64 {
    let mut weight_abs_max = 0f64;

    // 最大絶対重みを取得
    for feature_set in &merged_model.feature_sets {
        weight_abs_max = weight_abs_max.max(feature_set.weight.abs());
    }

    // i16範囲（-32768〜32767）に収まるようスケーリング
    f64::from(i16::MAX) / weight_abs_max
}
```

## 高度な辞書出力システム

### 動的コスト計算

未知語カテゴリごとに学習重みを活用した動的コスト計算：

```rust
pub fn get_unknown_word_cost(&self, category: usize) -> i32 {
    if !self.feature_weights.is_empty() && category < self.feature_weights.len() {
        let raw_weight = self.feature_weights[category];
        let normalized_weight = (raw_weight / 10.0).clamp(-2.0, 2.0);
        let calculated_cost = (-normalized_weight * 500.0) as i32 + 2000;

        // カテゴリ別調整
        let category_adjustment = match category {
            0 => 0,    // DEFAULT
            1 => -200, // HIRAGANA（頻出、低コスト）
            2 => -200, // KATAKANA（頻出、低コスト）
            3 => 200,  // KANJI（予測困難、高コスト）
            4 => 100,  // ALPHA（外来語、中程度）
            5 => -100, // NUMERIC（規則的、低コスト）
            _ => 0,
        };

        (calculated_cost + category_adjustment).clamp(1000, 3000)
    } else {
        // フォールバック用固定コスト
        match category {
            0 => 2000, 1 => 1800, 2 => 1800, 3 => 2200, 4 => 2100, 5 => 1900, _ => 2000
        }
    }
}
```

### 辞書エクスポート機能

学習済みモデルから実用的な辞書ファイルセットを生成：

```rust
// 語彙辞書（lex.csv）
pub fn write_lexicon<W: Write>(&self, writer: &mut W) -> Result<()> {
    let merged_model = self.get_merged_model()?;
    let weight_scale_factor = self.calculate_weight_scale_factor(&merged_model);

    for (i, surface) in self.config.surfaces.iter().enumerate() {
        if i < merged_model.feature_sets.len() {
            let feature_set = merged_model.feature_sets[i];
            let cost = (-feature_set.weight * weight_scale_factor) as i16;
            let features = self.get_word_features(surface);
            writeln!(writer, "{},{},{},{},{}",
                surface, feature_set.left_id, feature_set.right_id, cost, features)?;
        }
    }
    Ok(())
}

// 接続コスト行列（matrix.def）
pub fn write_connection_costs<W: Write>(&self, writer: &mut W) -> Result<()> {
    let merged_model = self.get_merged_model()?;

    // 動的次元計算
    let max_context_id = self.calculate_max_context_id();
    let matrix_size = std::cmp::max(max_context_id as usize + 1,
        std::cmp::max(merged_model.right_conn_to_left_feats.len() + 1,
                     merged_model.left_conn_to_right_feats.len() + 1));

    writeln!(writer, "{} {}", matrix_size, matrix_size)?;

    // 学習済み接続コストを出力
    for (right_conn_id, hm) in merged_model.matrix.iter().enumerate() {
        for (&left_conn_id, &_w) in hm.iter() {
            let cost = self.get_trained_connection_cost(left_conn_id as usize, right_conn_id);
            writeln!(writer, "{} {} {}", right_conn_id, left_conn_id, cost)?;
        }
    }
    Ok(())
}
```

## デバッグ・診断機能

### 詳細ログ出力

学習プロセスの可視化とデバッグ用の詳細ログ：

```rust
// 特徴クリーンアップログ
println!("Feature cleanup completed. Remaining features:");
println!("  Unigram: {}", self.config.feature_extractor.unigram_feature_ids.len());
println!("  Left: {}", self.config.feature_extractor.left_feature_ids.len());
println!("  Right: {}", self.config.feature_extractor.right_feature_ids.len());

// 重み抽出デバッグログ
eprintln!("DEBUG: Total feature_weights: {}", self.feature_weights.len());
eprintln!("DEBUG: Weight scale factor: {}", weight_scale_factor);
for (i, weight) in self.feature_weights.iter().take(20).enumerate() {
    eprintln!("DEBUG: feature_weights[{}] = {}", i, weight);
}
```

### 基本モデル評価機能

```rust
pub fn evaluate(&self, _test_lattices: &[rucrf::Lattice]) -> f64 {
    // 現在は学習済み重みの平均絶対値を返す簡易評価
    // より高度な実装では実際の尤度スコアを計算する
    let weights = self.raw_model.weights();
    if weights.is_empty() {
        0.0
    } else {
        let sum: f64 = weights.iter().map(|w| w.abs()).sum();
        sum / weights.len() as f64
    }
}
```

**注意**: この関数は基本的な実装で、テストデータでの実際の精度評価はまだ実装されていません。将来的には以下のような機能が追加予定：

- **正解データとの比較**: テスト格子を使った実際の分析精度測定
- **F値計算**: 精度・再現率・F値の自動計算
- **エラー分析**: 誤分析パターンの詳細レポート

## パフォーマンス最適化

### メモリ効率化

- **遅延評価**: 必要時のみmerged_modelを作成
- **未使用特徴除去**: 学習後の不要特徴自動削除
- **効率的なバイナリ形式**: bincode使用による高速シリアライゼーション

### 並列処理対応

```rust
let trainer = rucrf::Trainer::new()
    .regularization(rucrf::Regularization::L1, regularization_cost)?
    .max_iter(max_iter)?
    .n_threads(self.num_threads)?;  // マルチスレッド学習
```

## 実用的な学習データ要件

### 推奨コーパス仕様

実際のアプリケーションで有効な辞書を生成するための推奨事項：

1. **コーパスサイズ**
   - **最小**: 100文（基本動作確認用）
   - **推奨**: 1,000文以上（実用レベル）
   - **理想**: 10,000文以上（商用品質）

2. **語彙の多様性**
   - 異なる品詞の均等な分布
   - 活用形・語尾変化の網羅
   - 専門用語・固有名詞の適切な含有

3. **品質管理**
   - 人手による形態素解析結果の検証
   - 一貫した分析基準の適用
   - エラー率5%以下の維持
