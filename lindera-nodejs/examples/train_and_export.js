#!/usr/bin/env node
/**
 * Example: Train a morphological analysis model and export dictionary files
 *
 * This example demonstrates how to:
 * 1. Train a model from a corpus using train()
 * 2. Export dictionary files from the trained model using exportModel()
 *
 * Note: This requires the 'train' feature to be enabled when building lindera-nodejs:
 *   npm run build -- --features train
 */

const fs = require("fs");
const os = require("os");
const path = require("path");
const { train, exportModel } = require("lindera");

function createTrainingData(tmpdir) {
  // Create seed lexicon (vocabulary with initial costs)
  // Format: surface,left_id,right_id,cost,features...
  const seedFile = path.join(tmpdir, "seed.csv");
  fs.writeFileSync(
    seedFile,
    [
      "外国,0,0,0,名詞,一般,*,*,*,*,外国,ガイコク,ガイコク",
      "人,0,0,0,名詞,接尾,一般,*,*,*,人,ジン,ジン",
      "参政,0,0,0,名詞,サ変接続,*,*,*,*,参政,サンセイ,サンセイ",
      "権,0,0,0,名詞,接尾,一般,*,*,*,権,ケン,ケン",
      "これ,0,0,0,名詞,代名詞,一般,*,*,*,これ,コレ,コレ",
      "は,0,0,0,助詞,係助詞,*,*,*,*,は,ハ,ワ",
      "テスト,0,0,0,名詞,サ変接続,*,*,*,*,テスト,テスト,テスト",
      "です,0,0,0,助動詞,*,*,*,特殊・デス,基本形,です,デス,デス",
      "。,0,0,0,記号,句点,*,*,*,*,。,。,。",
      "形態,0,0,0,名詞,一般,*,*,*,*,形態,ケイタイ,ケイタイ",
      "素,0,0,0,名詞,接尾,一般,*,*,*,素,ソ,ソ",
      "解析,0,0,0,名詞,サ変接続,*,*,*,*,解析,カイセキ,カイセキ",
      "を,0,0,0,助詞,格助詞,一般,*,*,*,を,ヲ,ヲ",
      "行う,0,0,0,動詞,自立,*,*,五段・ワ行促音便,基本形,行う,オコナウ,オコナウ",
    ].join("\n") + "\n",
  );

  // Create character definition (defines character types)
  const charDefFile = path.join(tmpdir, "char.def");
  fs.writeFileSync(
    charDefFile,
    [
      "# Character definition for training",
      "DEFAULT 0 1 0",
      "HIRAGANA 1 1 0",
      "KATAKANA 1 1 0",
      "KANJI 0 0 2",
      "ALPHA 1 1 0",
      "NUMERIC 1 1 0",
      "",
      "# Character mappings (simplified)",
      "0x3041..0x3096 HIRAGANA",
      "0x30A1..0x30F6 KATAKANA",
      "0x4E00..0x9FAF KANJI",
      "0x0030..0x0039 NUMERIC",
      "0x0041..0x005A ALPHA",
      "0x0061..0x007A ALPHA",
    ].join("\n") + "\n",
  );

  // Create unknown word definition (for out-of-vocabulary words)
  const unkDefFile = path.join(tmpdir, "unk.def");
  fs.writeFileSync(
    unkDefFile,
    [
      "# Unknown word definitions",
      "DEFAULT,0,0,0,名詞,一般,*,*,*,*,*,*,*",
      "HIRAGANA,0,0,0,名詞,一般,*,*,*,*,*,*,*",
      "KATAKANA,0,0,0,名詞,一般,*,*,*,*,*,*,*",
      "KANJI,0,0,0,名詞,一般,*,*,*,*,*,*,*",
      "ALPHA,0,0,0,名詞,固有名詞,一般,*,*,*,*,*,*",
      "NUMERIC,0,0,0,名詞,数,*,*,*,*,*,*,*",
    ].join("\n") + "\n",
  );

  // Create feature definition (defines features for CRF training)
  const featureDefFile = path.join(tmpdir, "feature.def");
  fs.writeFileSync(
    featureDefFile,
    [
      "# Feature template definitions for training",
      "",
      "# Unigram features (word-level features)",
      "UNIGRAM U00:%F[0]    # Part of speech",
      "UNIGRAM U01:%F[0],%F?[1]    # POS + sub-category",
      "UNIGRAM U02:%F[0],%F[1],%F?[2]    # POS hierarchy",
      "",
      "# Bigram features (transition features between words)",
      "BIGRAM B00:%L[0]/%R[0]    # POS-to-POS transition",
      "BIGRAM B01:%L[0],%L?[1]/%R[0]    # Left POS hierarchy to right POS",
      "BIGRAM B02:%L[0]/%R[0],%R?[1]    # Left POS to right POS hierarchy",
      "BIGRAM B03:%L[0],%L[1],%L?[2]/%R[0]    # Detailed left to simple right",
    ].join("\n") + "\n",
  );

  // Create rewrite definition (for feature rewriting)
  const rewriteDefFile = path.join(tmpdir, "rewrite.def");
  fs.writeFileSync(
    rewriteDefFile,
    [
      "# Rewrite rules for feature normalization",
      "",
      "名詞,一般\tNOUN,GENERAL",
      "",
      "助詞,係助詞\tPARTICLE,KAKUJOSHI",
      "",
      "# Normalize numeric expressions",
      "数\tNUM",
    ].join("\n") + "\n",
  );

  // Create training corpus (annotated text)
  const corpusFile = path.join(tmpdir, "corpus.txt");
  fs.writeFileSync(
    corpusFile,
    [
      "外国\t名詞,一般,*,*,*,*,外国,ガイコク,ガイコク",
      "人\t名詞,接尾,一般,*,*,*,人,ジン,ジン",
      "参政\t名詞,サ変接続,*,*,*,*,参政,サンセイ,サンセイ",
      "権\t名詞,接尾,一般,*,*,*,権,ケン,ケン",
      "EOS",
      "",
      "これ\t名詞,代名詞,一般,*,*,*,これ,コレ,コレ",
      "は\t助詞,係助詞,*,*,*,*,は,ハ,ワ",
      "テスト\t名詞,サ変接続,*,*,*,*,テスト,テスト,テスト",
      "です\t助動詞,*,*,*,特殊・デス,基本形,です,デス,デス",
      "。\t記号,句点,*,*,*,*,。,。,。",
      "EOS",
      "",
      "形態\t名詞,一般,*,*,*,*,形態,ケイタイ,ケイタイ",
      "素\t名詞,接尾,一般,*,*,*,素,ソ,ソ",
      "解析\t名詞,サ変接続,*,*,*,*,解析,カイセキ,カイセキ",
      "を\t助詞,格助詞,一般,*,*,*,を,ヲ,ヲ",
      "行う\t動詞,自立,*,*,五段・ワ行促音便,基本形,行う,オコナウ,オコナウ",
      "EOS",
    ].join("\n") + "\n",
  );

  // Create metadata for dictionary export
  const metadataFile = path.join(tmpdir, "metadata.json");
  fs.writeFileSync(
    metadataFile,
    JSON.stringify(
      {
        name: "custom-dict",
        version: "1.0.0",
        encoding: "utf-8",
      },
      null,
      2,
    ) + "\n",
  );

  return {
    seed: seedFile,
    charDef: charDefFile,
    unkDef: unkDefFile,
    featureDef: featureDefFile,
    rewriteDef: rewriteDefFile,
    corpus: corpusFile,
    metadata: metadataFile,
  };
}

function main() {
  console.log("=== Lindera Training and Export Example ===\n");

  const tmpdir = fs.mkdtempSync(path.join(os.tmpdir(), "lindera-train-"));
  console.log(`Working directory: ${tmpdir}\n`);

  try {
    // Step 1: Create training data
    console.log("Step 1: Creating training data...");
    const files = createTrainingData(tmpdir);
    console.log("Training data created\n");

    // Step 2: Train model
    console.log("Step 2: Training model...");
    const modelFile = path.join(tmpdir, "model.dat");

    train({
      seed: files.seed,
      corpus: files.corpus,
      charDef: files.charDef,
      unkDef: files.unkDef,
      featureDef: files.featureDef,
      rewriteDef: files.rewriteDef,
      output: modelFile,
      lambda: 0.01,
      maxIter: 10,
    });

    console.log(`Model trained and saved to: ${modelFile}\n`);

    // Step 3: Export dictionary files
    console.log("Step 3: Exporting dictionary files...");
    const exportDir = path.join(tmpdir, "exported_dict");

    exportModel({
      model: modelFile,
      output: exportDir,
      metadata: files.metadata,
    });

    console.log(`Dictionary files exported to: ${exportDir}\n`);

    // Step 4: List exported files
    console.log("Step 4: Exported files:");
    const exportedFiles = fs.readdirSync(exportDir).sort();
    for (const file of exportedFiles) {
      const filePath = path.join(exportDir, file);
      const stats = fs.statSync(filePath);
      console.log(`  - ${file} (${stats.size.toLocaleString()} bytes)`);
    }

    console.log("\nTraining and export completed successfully!");
  } finally {
    // Cleanup
    fs.rmSync(tmpdir, { recursive: true, force: true });
  }
}

main();
