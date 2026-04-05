<?php

/**
 * Example: Train a morphological analysis model and export dictionary files.
 *
 * This example demonstrates how to:
 * 1. Train a model from a corpus using Lindera\Trainer::train()
 * 2. Export dictionary files from the trained model using Lindera\Trainer::export()
 *
 * Note: This requires the 'train' feature to be enabled when building lindera-php:
 *   cargo build -p lindera-php --features embed-ipadic,train
 *
 * Usage:
 *   php -d extension=target/debug/liblindera_php.so lindera-php/examples/train_and_export.php
 */

/**
 * Create minimal training data based on lindera training format.
 */
function createTrainingData(string $tmpdir): array
{
    // Create seed lexicon (vocabulary with initial costs)
    // Format: surface,left_id,right_id,cost,features...
    $seedFile = $tmpdir . '/seed.csv';
    file_put_contents($seedFile, implode("\n", [
        '外国,0,0,0,名詞,一般,*,*,*,*,外国,ガイコク,ガイコク',
        '人,0,0,0,名詞,接尾,一般,*,*,*,人,ジン,ジン',
        '参政,0,0,0,名詞,サ変接続,*,*,*,*,参政,サンセイ,サンセイ',
        '権,0,0,0,名詞,接尾,一般,*,*,*,権,ケン,ケン',
        'これ,0,0,0,名詞,代名詞,一般,*,*,*,これ,コレ,コレ',
        'は,0,0,0,助詞,係助詞,*,*,*,*,は,ハ,ワ',
        'テスト,0,0,0,名詞,サ変接続,*,*,*,*,テスト,テスト,テスト',
        'です,0,0,0,助動詞,*,*,*,特殊・デス,基本形,です,デス,デス',
        '。,0,0,0,記号,句点,*,*,*,*,。,。,。',
        '形態,0,0,0,名詞,一般,*,*,*,*,形態,ケイタイ,ケイタイ',
        '素,0,0,0,名詞,接尾,一般,*,*,*,素,ソ,ソ',
        '解析,0,0,0,名詞,サ変接続,*,*,*,*,解析,カイセキ,カイセキ',
        'を,0,0,0,助詞,格助詞,一般,*,*,*,を,ヲ,ヲ',
        '行う,0,0,0,動詞,自立,*,*,五段・ワ行促音便,基本形,行う,オコナウ,オコナウ',
        '',
    ]));

    // Create character definition (defines character types)
    $charDefFile = $tmpdir . '/char.def';
    file_put_contents($charDefFile, implode("\n", [
        '# Character definition for training',
        'DEFAULT 0 1 0',
        'HIRAGANA 1 1 0',
        'KATAKANA 1 1 0',
        'KANJI 0 0 2',
        'ALPHA 1 1 0',
        'NUMERIC 1 1 0',
        '',
        '# Character mappings (simplified)',
        '0x3041..0x3096 HIRAGANA',
        '0x30A1..0x30F6 KATAKANA',
        '0x4E00..0x9FAF KANJI',
        '0x0030..0x0039 NUMERIC',
        '0x0041..0x005A ALPHA',
        '0x0061..0x007A ALPHA',
        '',
    ]));

    // Create unknown word definition (for out-of-vocabulary words)
    $unkDefFile = $tmpdir . '/unk.def';
    file_put_contents($unkDefFile, implode("\n", [
        '# Unknown word definitions',
        'DEFAULT,0,0,0,名詞,一般,*,*,*,*,*,*,*',
        'HIRAGANA,0,0,0,名詞,一般,*,*,*,*,*,*,*',
        'KATAKANA,0,0,0,名詞,一般,*,*,*,*,*,*,*',
        'KANJI,0,0,0,名詞,一般,*,*,*,*,*,*,*',
        'ALPHA,0,0,0,名詞,固有名詞,一般,*,*,*,*,*,*',
        'NUMERIC,0,0,0,名詞,数,*,*,*,*,*,*,*',
        '',
    ]));

    // Create feature definition (defines features for CRF training)
    $featureDefFile = $tmpdir . '/feature.def';
    file_put_contents($featureDefFile, implode("\n", [
        '# Feature template definitions for training',
        '# These define how features are extracted from the morphological data',
        '',
        '# Unigram features (word-level features)',
        'UNIGRAM U00:%F[0]    # Part of speech',
        'UNIGRAM U01:%F[0],%F?[1]    # POS + sub-category',
        'UNIGRAM U02:%F[0],%F[1],%F?[2]    # POS hierarchy',
        '',
        '# Bigram features (transition features between words)',
        '# Format: BIGRAM label:%L[index]/%R[index]',
        '# %L = left context (previous word), %R = right context (next word)',
        'BIGRAM B00:%L[0]/%R[0]    # POS-to-POS transition',
        'BIGRAM B01:%L[0],%L?[1]/%R[0]    # Left POS hierarchy to right POS',
        'BIGRAM B02:%L[0]/%R[0],%R?[1]    # Left POS to right POS hierarchy',
        'BIGRAM B03:%L[0],%L[1],%L?[2]/%R[0]    # Detailed left to simple right',
        '',
    ]));

    // Create rewrite definition (for feature rewriting)
    $rewriteDefFile = $tmpdir . '/rewrite.def';
    file_put_contents($rewriteDefFile, implode("\n", [
        '# Rewrite rules for feature normalization',
        "# Format: original_pattern\treplacement_pattern",
        '',
        '# Test rewrite: convert "名詞,一般" to "NOUN,GENERAL"',
        "名詞,一般\tNOUN,GENERAL",
        '',
        '# Test rewrite: convert "助詞,係助詞" to "PARTICLE,KAKUJOSHI"',
        "助詞,係助詞\tPARTICLE,KAKUJOSHI",
        '',
        '# Normalize numeric expressions',
        "数\tNUM",
        '',
    ]));

    // Create training corpus (annotated text)
    // Format: surface\tfeatures (tab-separated)
    // Each sentence ends with "EOS"
    $corpusFile = $tmpdir . '/corpus.txt';
    file_put_contents($corpusFile, implode("\n", [
        "外国\t名詞,一般,*,*,*,*,外国,ガイコク,ガイコク",
        "人\t名詞,接尾,一般,*,*,*,人,ジン,ジン",
        "参政\t名詞,サ変接続,*,*,*,*,参政,サンセイ,サンセイ",
        "権\t名詞,接尾,一般,*,*,*,権,ケン,ケン",
        'EOS',
        '',
        "これ\t名詞,代名詞,一般,*,*,*,これ,コレ,コレ",
        "は\t助詞,係助詞,*,*,*,*,は,ハ,ワ",
        "テスト\t名詞,サ変接続,*,*,*,*,テスト,テスト,テスト",
        "です\t助動詞,*,*,*,特殊・デス,基本形,です,デス,デス",
        "。\t記号,句点,*,*,*,*,。,。,。",
        'EOS',
        '',
        "形態\t名詞,一般,*,*,*,*,形態,ケイタイ,ケイタイ",
        "素\t名詞,接尾,一般,*,*,*,素,ソ,ソ",
        "解析\t名詞,サ変接続,*,*,*,*,解析,カイセキ,カイセキ",
        "を\t助詞,格助詞,一般,*,*,*,を,ヲ,ヲ",
        "行う\t動詞,自立,*,*,五段・ワ行促音便,基本形,行う,オコナウ,オコナウ",
        'EOS',
        '',
    ]));

    // Create metadata for dictionary export
    $metadataFile = $tmpdir . '/metadata.json';
    file_put_contents($metadataFile, json_encode([
        'name' => 'custom-dict',
        'version' => '1.0.0',
        'encoding' => 'utf-8',
    ], JSON_PRETTY_PRINT) . "\n");

    return [
        'seed' => $seedFile,
        'char_def' => $charDefFile,
        'unk_def' => $unkDefFile,
        'feature_def' => $featureDefFile,
        'rewrite_def' => $rewriteDefFile,
        'corpus' => $corpusFile,
        'metadata' => $metadataFile,
    ];
}

/**
 * Main training and export workflow.
 */
function main(): void
{
    echo "=== Lindera Training and Export Example ===\n\n";

    $tmpdir = sys_get_temp_dir() . '/lindera_train_' . uniqid();
    mkdir($tmpdir, 0755, true);
    echo "Working directory: {$tmpdir}\n\n";

    try {
        // Step 1: Create training data
        echo "Step 1: Creating training data...\n";
        $files = createTrainingData($tmpdir);
        echo "Training data created\n\n";

        // Step 2: Train model
        echo "Step 2: Training model...\n";
        $modelFile = $tmpdir . '/model.dat';

        Lindera\Trainer::train(
            $files['seed'],
            $files['corpus'],
            $files['char_def'],
            $files['unk_def'],
            $files['feature_def'],
            $files['rewrite_def'],
            $modelFile,
            0.01,   // lambda: L1 regularization
            10,     // max_iter: Number of training iterations
            null    // max_threads: Auto-detect CPU cores
        );

        echo "Model trained and saved to: {$modelFile}\n\n";

        // Step 3: Export dictionary files
        echo "Step 3: Exporting dictionary files...\n";
        $exportDir = $tmpdir . '/exported_dict';

        Lindera\Trainer::export(
            $modelFile,
            $exportDir,
            $files['metadata']
        );

        echo "Dictionary files exported to: {$exportDir}\n\n";

        // Step 4: List exported files
        echo "Step 4: Exported files:\n";
        $iterator = new DirectoryIterator($exportDir);
        foreach ($iterator as $file) {
            if ($file->isDot()) {
                continue;
            }
            $size = number_format($file->getSize());
            echo "  - {$file->getFilename()} ({$size} bytes)\n";
        }

        echo "\nTraining and export completed successfully!\n";
    } finally {
        // Cleanup (optional - comment out to inspect files)
        $cleanup = function (string $dir) use (&$cleanup): void {
            $iterator = new DirectoryIterator($dir);
            foreach ($iterator as $file) {
                if ($file->isDot()) {
                    continue;
                }
                if ($file->isDir()) {
                    $cleanup($file->getPathname());
                } else {
                    unlink($file->getPathname());
                }
            }
            rmdir($dir);
        };
        $cleanup($tmpdir);
    }
}

main();
