const assert = require("assert");
const { describe, it } = require("node:test");
const fs = require("fs");
const os = require("os");
const path = require("path");

const lindera = require("../index.js");
const hasTrainFeature = typeof lindera.train === "function";

describe(
  "Trainer",
  { skip: !hasTrainFeature && "train feature not available" },
  () => {
    it("should train a model from corpus", () => {
      const tmpdir = fs.mkdtempSync(path.join(os.tmpdir(), "lindera-test-"));

      try {
        // Create minimal seed lexicon
        fs.writeFileSync(
          path.join(tmpdir, "seed.csv"),
          [
            "これ,0,0,0,名詞,代名詞,一般,*,*,*,これ,コレ,コレ",
            "は,0,0,0,助詞,係助詞,*,*,*,*,は,ハ,ワ",
            "テスト,0,0,0,名詞,サ変接続,*,*,*,*,テスト,テスト,テスト",
          ].join("\n") + "\n",
        );

        // Create character definition
        fs.writeFileSync(
          path.join(tmpdir, "char.def"),
          [
            "DEFAULT 0 1 0",
            "HIRAGANA 1 1 0",
            "KATAKANA 1 1 0",
            "KANJI 0 0 2",
            "ALPHA 1 1 0",
            "NUMERIC 1 1 0",
            "",
            "0x3041..0x3096 HIRAGANA",
            "0x30A1..0x30F6 KATAKANA",
            "0x4E00..0x9FAF KANJI",
            "0x0030..0x0039 NUMERIC",
            "0x0041..0x005A ALPHA",
            "0x0061..0x007A ALPHA",
          ].join("\n") + "\n",
        );

        // Create unknown word definition
        fs.writeFileSync(
          path.join(tmpdir, "unk.def"),
          [
            "DEFAULT,0,0,0,名詞,一般,*,*,*,*,*,*,*",
            "HIRAGANA,0,0,0,名詞,一般,*,*,*,*,*,*,*",
            "KATAKANA,0,0,0,名詞,一般,*,*,*,*,*,*,*",
            "KANJI,0,0,0,名詞,一般,*,*,*,*,*,*,*",
            "ALPHA,0,0,0,名詞,固有名詞,一般,*,*,*,*,*,*",
            "NUMERIC,0,0,0,名詞,数,*,*,*,*,*,*,*",
          ].join("\n") + "\n",
        );

        // Create feature definition
        fs.writeFileSync(
          path.join(tmpdir, "feature.def"),
          [
            "UNIGRAM U00:%F[0]",
            "UNIGRAM U01:%F[0],%F?[1]",
            "BIGRAM B00:%L[0]/%R[0]",
          ].join("\n") + "\n",
        );

        // Create rewrite definition
        fs.writeFileSync(
          path.join(tmpdir, "rewrite.def"),
          "名詞,一般\tNOUN,GENERAL\n",
        );

        // Create training corpus
        fs.writeFileSync(
          path.join(tmpdir, "corpus.txt"),
          [
            "これ\t名詞,代名詞,一般,*,*,*,これ,コレ,コレ",
            "は\t助詞,係助詞,*,*,*,*,は,ハ,ワ",
            "テスト\t名詞,サ変接続,*,*,*,*,テスト,テスト,テスト",
            "EOS",
          ].join("\n") + "\n",
        );

        const modelFile = path.join(tmpdir, "model.dat");

        // Train
        lindera.train({
          seed: path.join(tmpdir, "seed.csv"),
          corpus: path.join(tmpdir, "corpus.txt"),
          charDef: path.join(tmpdir, "char.def"),
          unkDef: path.join(tmpdir, "unk.def"),
          featureDef: path.join(tmpdir, "feature.def"),
          rewriteDef: path.join(tmpdir, "rewrite.def"),
          output: modelFile,
          lambda: 0.01,
          maxIter: 5,
          maxThreads: 1,
        });

        // Verify model file was created
        assert.ok(fs.existsSync(modelFile));
        assert.ok(fs.statSync(modelFile).size > 0);
      } finally {
        fs.rmSync(tmpdir, { recursive: true, force: true });
      }
    });

    it("should export a trained model to dictionary files", () => {
      const tmpdir = fs.mkdtempSync(path.join(os.tmpdir(), "lindera-test-"));

      try {
        // Create minimal training data
        fs.writeFileSync(
          path.join(tmpdir, "seed.csv"),
          [
            "これ,0,0,0,名詞,代名詞,一般,*,*,*,これ,コレ,コレ",
            "は,0,0,0,助詞,係助詞,*,*,*,*,は,ハ,ワ",
          ].join("\n") + "\n",
        );

        fs.writeFileSync(
          path.join(tmpdir, "char.def"),
          [
            "DEFAULT 0 1 0",
            "HIRAGANA 1 1 0",
            "",
            "0x3041..0x3096 HIRAGANA",
          ].join("\n") + "\n",
        );

        fs.writeFileSync(
          path.join(tmpdir, "unk.def"),
          [
            "DEFAULT,0,0,0,名詞,一般,*,*,*,*,*,*,*",
            "HIRAGANA,0,0,0,名詞,一般,*,*,*,*,*,*,*",
          ].join("\n") + "\n",
        );

        fs.writeFileSync(
          path.join(tmpdir, "feature.def"),
          ["UNIGRAM U00:%F[0]", "BIGRAM B00:%L[0]/%R[0]"].join("\n") + "\n",
        );

        fs.writeFileSync(path.join(tmpdir, "rewrite.def"), "名詞,一般\tNOUN\n");

        fs.writeFileSync(
          path.join(tmpdir, "corpus.txt"),
          [
            "これ\t名詞,代名詞,一般,*,*,*,これ,コレ,コレ",
            "は\t助詞,係助詞,*,*,*,*,は,ハ,ワ",
            "EOS",
          ].join("\n") + "\n",
        );

        const modelFile = path.join(tmpdir, "model.dat");

        // Train
        lindera.train({
          seed: path.join(tmpdir, "seed.csv"),
          corpus: path.join(tmpdir, "corpus.txt"),
          charDef: path.join(tmpdir, "char.def"),
          unkDef: path.join(tmpdir, "unk.def"),
          featureDef: path.join(tmpdir, "feature.def"),
          rewriteDef: path.join(tmpdir, "rewrite.def"),
          output: modelFile,
          lambda: 0.01,
          maxIter: 5,
          maxThreads: 1,
        });

        // Export
        const exportDir = path.join(tmpdir, "exported");
        lindera.exportModel({
          model: modelFile,
          output: exportDir,
        });

        // Verify exported files
        assert.ok(fs.existsSync(path.join(exportDir, "lex.csv")));
        assert.ok(fs.existsSync(path.join(exportDir, "matrix.def")));
        assert.ok(fs.existsSync(path.join(exportDir, "unk.def")));
        assert.ok(fs.existsSync(path.join(exportDir, "char.def")));

        assert.ok(fs.statSync(path.join(exportDir, "lex.csv")).size > 0);
        assert.ok(fs.statSync(path.join(exportDir, "matrix.def")).size > 0);
      } finally {
        fs.rmSync(tmpdir, { recursive: true, force: true });
      }
    });
  },
);
