const assert = require("assert");
const { describe, it } = require("node:test");
const fs = require("fs");
const os = require("os");
const path = require("path");

const lindera = require("../index.js");
const hasTrainFeature = typeof lindera.train === "function";

// The files `lindera export` writes besides metadata.json, which
// exportModel() writes too (#1169).
const EXPORTED_FILES = [
  "lex.csv",
  "matrix.def",
  "unk.def",
  "char.def",
  "feature.def",
  "rewrite.def",
  "left-id.def",
  "right-id.def",
];

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

        // Verify exported files: those of `lindera export`, none empty
        assert.deepStrictEqual(
          fs.readdirSync(exportDir).sort(),
          [...EXPORTED_FILES].sort(),
        );
        for (const name of EXPORTED_FILES) {
          assert.ok(fs.statSync(path.join(exportDir, name)).size > 0, name);
        }

        // The definition files are those the model was trained with
        for (const name of ["char.def", "feature.def", "rewrite.def"]) {
          assert.deepStrictEqual(
            fs.readFileSync(path.join(exportDir, name)),
            fs.readFileSync(path.join(tmpdir, name)),
            name,
          );
        }

        // With a base metadata.json, metadata.json too
        const metadataFile = path.join(tmpdir, "metadata.json");
        fs.writeFileSync(
          metadataFile,
          JSON.stringify({
            name: "test-dict",
            version: "1.0.0",
            encoding: "utf-8",
          }),
        );
        const exportDirWithMetadata = path.join(
          tmpdir,
          "exported-with-metadata",
        );
        lindera.exportModel({
          model: modelFile,
          output: exportDirWithMetadata,
          metadata: metadataFile,
        });
        assert.deepStrictEqual(
          fs.readdirSync(exportDirWithMetadata).sort(),
          [...EXPORTED_FILES, "metadata.json"].sort(),
        );
      } finally {
        fs.rmSync(tmpdir, { recursive: true, force: true });
      }
    });

    // Leaving lambda out trains with 1.0, the L2 coefficient of MeCab's
    // -c 1.0 (#1261): the model is byte-identical to one trained with
    // lambda 1.0.
    it("should train with lambda 1.0 by default", () => {
      const tmpdir = fs.mkdtempSync(path.join(os.tmpdir(), "lindera-test-"));

      try {
        const inputs = {
          seed: [
            "seed.csv",
            "これ,0,0,0,名詞,代名詞,一般,*,*,*,これ,コレ,コレ\n" +
              "は,0,0,0,助詞,係助詞,*,*,*,*,は,ハ,ワ\n",
          ],
          corpus: [
            "corpus.txt",
            "これ\t名詞,代名詞,一般,*,*,*,これ,コレ,コレ\n" +
              "は\t助詞,係助詞,*,*,*,*,は,ハ,ワ\n" +
              "EOS\n",
          ],
          charDef: [
            "char.def",
            "DEFAULT 0 1 0\nHIRAGANA 1 1 0\n\n0x3041..0x3096 HIRAGANA\n",
          ],
          unkDef: [
            "unk.def",
            "DEFAULT,0,0,0,名詞,一般,*,*,*,*,*,*,*\n" +
              "HIRAGANA,0,0,0,名詞,一般,*,*,*,*,*,*,*\n",
          ],
          featureDef: [
            "feature.def",
            "UNIGRAM U00:%F[0]\nBIGRAM B00:%L[0]/%R[0]\n",
          ],
          rewriteDef: ["rewrite.def", "名詞,一般\tNOUN\n"],
        };
        const paths = {};
        for (const [key, [name, content]] of Object.entries(inputs)) {
          paths[key] = path.join(tmpdir, name);
          fs.writeFileSync(paths[key], content);
        }
        const train = (name, options) => {
          const output = path.join(tmpdir, name);
          lindera.train({
            ...paths,
            output,
            maxIter: 5,
            maxThreads: 1,
            ...options,
          });
          return fs.readFileSync(output);
        };

        const byDefault = train("default.dat", {});
        const explicit = train("explicit.dat", { lambda: 1.0 });
        assert.ok(byDefault.length > 0);
        assert.deepStrictEqual(byDefault, explicit);
      } finally {
        fs.rmSync(tmpdir, { recursive: true, force: true });
      }
    });

    it("should train with a zero effective L1 coefficient", () => {
      const tmpdir = fs.mkdtempSync(path.join(os.tmpdir(), "lindera-zero-l1-"));
      const training = path.join(
        __dirname,
        "..",
        "..",
        "resources",
        "training",
      );
      try {
        const cases = [
          { regularization: "l1", lambda: 0 },
          { regularization: "elasticnet", lambda: 0 },
          { regularization: "elasticnet", lambda: 1, elasticNetL1Ratio: 0 },
        ];
        for (const [i, options] of cases.entries()) {
          const output = path.join(tmpdir, `zero-l1-${i}.dat`);
          lindera.train({
            seed: path.join(training, "seed.csv"),
            corpus: path.join(training, "corpus.txt"),
            charDef: path.join(training, "char.def"),
            unkDef: path.join(training, "unk.def"),
            featureDef: path.join(training, "feature.def"),
            rewriteDef: path.join(training, "rewrite.def"),
            output,
            maxIter: 5,
            maxThreads: 1,
            ...options,
          });
          assert.ok(fs.statSync(output).size > 0);
        }
      } finally {
        fs.rmSync(tmpdir, { recursive: true, force: true });
      }
    });

    // regularization, elasticNetL1Ratio and eta reach the trainer, with the
    // CLI's defaults (#1262). On the repository's training files the default
    // converges before 100 iterations, so every setting below trains a
    // different model, eta 0 included.
    it("should take the regularization type, the ratio and eta", () => {
      const tmpdir = fs.mkdtempSync(path.join(os.tmpdir(), "lindera-test-"));
      const training = path.join(
        __dirname,
        "..",
        "..",
        "resources",
        "training",
      );

      try {
        const train = (name, options) => {
          const output = path.join(tmpdir, name);
          lindera.train({
            seed: path.join(training, "seed.csv"),
            corpus: path.join(training, "corpus.txt"),
            charDef: path.join(training, "char.def"),
            unkDef: path.join(training, "unk.def"),
            featureDef: path.join(training, "feature.def"),
            rewriteDef: path.join(training, "rewrite.def"),
            output,
            maxIter: 100,
            maxThreads: 1,
            ...options,
          });
          return fs.readFileSync(output).toString("base64");
        };

        const byDefault = train("default.dat", {});
        assert.strictEqual(
          train("explicit.dat", {
            regularization: "l2",
            elasticNetL1Ratio: 0.5,
            eta: 5e-5,
          }),
          byDefault,
        );
        // A ratio is used only with elasticnet.
        assert.strictEqual(
          train("l2-ratio.dat", {
            regularization: "l2",
            elasticNetL1Ratio: 0.2,
          }),
          byDefault,
        );
        const models = {
          default: byDefault,
          l1: train("l1.dat", { regularization: "l1" }),
          "elasticnet 0.2": train("en02.dat", {
            regularization: "elasticnet",
            elasticNetL1Ratio: 0.2,
          }),
          "elasticnet 0.8": train("en08.dat", {
            regularization: "elasticnet",
            elasticNetL1Ratio: 0.8,
          }),
          "eta 0": train("eta0.dat", { eta: 0 }),
        };
        const same = Object.keys(models).filter(
          (name) =>
            Object.values(models).filter((model) => model === models[name])
              .length > 1,
        );
        assert.deepStrictEqual(
          same,
          [],
          "these settings trained the same model",
        );
      } finally {
        fs.rmSync(tmpdir, { recursive: true, force: true });
      }
    });
  },
);

describe(
  "Training settings",
  { skip: !hasTrainFeature && "train feature not available" },
  () => {
    // A path that does not exist: the settings are checked before any file.
    const MISSING = "/nonexistent/issue1262";
    const train = (options) =>
      lindera.train({
        seed: MISSING,
        corpus: MISSING,
        charDef: MISSING,
        unkDef: MISSING,
        featureDef: MISSING,
        rewriteDef: MISSING,
        output: MISSING,
        ...options,
      });

    // A setting the trainer would reject when training starts is an
    // InvalidArg error that names it, raised before any file is read.
    it("rejects a setting before reading any file", () => {
      const cases = [
        [
          { regularization: "l3" },
          "regularization must be 'l1', 'l2' or 'elasticnet', got 'l3'",
        ],
        [{ lambda: -1 }, "lambda must be a finite number >= 0, got -1"],
        [{ lambda: NaN }, "lambda must be a finite number >= 0, got NaN"],
        [
          { lambda: Infinity },
          "lambda must be a finite number >= 0, got Infinity",
        ],
        [{ eta: -1 }, "eta must be a finite number >= 0, got -1"],
        [{ eta: NaN }, "eta must be a finite number >= 0, got NaN"],
        [{ eta: Infinity }, "eta must be a finite number >= 0, got Infinity"],
        [
          { elasticNetL1Ratio: -0.1 },
          "elasticNetL1Ratio must be a number between 0 and 1, got -0.1",
        ],
        [
          { elasticNetL1Ratio: 1.5 },
          "elasticNetL1Ratio must be a number between 0 and 1, got 1.5",
        ],
        [
          { elasticNetL1Ratio: NaN },
          "elasticNetL1Ratio must be a number between 0 and 1, got NaN",
        ],
      ];
      for (const [options, message] of cases) {
        assert.throws(
          () => train(options),
          (error) => {
            assert.strictEqual(error.code, "InvalidArg", message);
            assert.strictEqual(error.message, message);
            return true;
          },
          `accepted ${JSON.stringify(options)}`,
        );
      }
    });

    // The bounds and the other spellings pass the checks, so the missing
    // seed file is what fails.
    it("accepts the bounds of each setting", () => {
      for (const options of [
        { lambda: 0 },
        { eta: 0 },
        { elasticNetL1Ratio: 0 },
        { elasticNetL1Ratio: 1 },
        { regularization: "ElasticNet" },
        { regularization: "elastic-net" },
      ]) {
        assert.throws(
          () => train(options),
          /seed file does not exist/,
          JSON.stringify(options),
        );
      }
    });

    // napi-rs checks the type of each field: a value of another type is an
    // error, not the default.
    it("rejects a setting of another type", () => {
      for (const options of [
        { regularization: 1 },
        { regularization: null },
        { elasticNetL1Ratio: "0.5" },
        { elasticNetL1Ratio: null },
        { eta: "0" },
        { eta: null },
      ]) {
        assert.throws(
          () => train(options),
          (error) => !/does not exist/.test(error.message),
          JSON.stringify(options),
        );
      }
    });
  },
);
