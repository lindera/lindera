const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { describe, it } = require("node:test");

const lindera = require("../index.js");

// Skip all tests if embedded IPADIC is not available
const hasEmbeddedIpadic = (() => {
  try {
    lindera.loadDictionary("embedded://ipadic");
    return true;
  } catch {
    return false;
  }
})();

// Line 1 lies inside IPADIC's 1316 x 1316 connection matrix; line 2 has the
// context IDs 60000, far outside it.
const OUT_OF_RANGE_CSV =
  "東京タワー,1288,1288,-1000,名詞,固有名詞,一般,*,*,*,東京タワー,トウキョウタワー,トウキョウタワー\n" +
  "東京スカイツリー,60000,60000,-1000,名詞,固有名詞,一般,*,*,*,東京スカイツリー,トウキョウスカイツリー,トウキョウスカイツリー\n";

const SIMPLE_CSV = "東京スカイツリー,カスタム名詞,トウキョウスカイツリー\n";

// Writes `content` to a CSV file in a new temporary directory, passes its
// path to `fn`, and removes the directory afterwards.
function withCsv(content, fn) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "lindera-userdic-"));
  try {
    const csv = path.join(dir, "userdic.csv");
    fs.writeFileSync(csv, content, "utf8");
    return fn(csv);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
}

describe(
  "loadUserDictionaryFor",
  { skip: !hasEmbeddedIpadic && "embedded://ipadic not available" },
  () => {
    it("names the CSV line and surface of a row outside the matrix", () => {
      const dictionary = lindera.loadDictionary("embedded://ipadic");
      withCsv(OUT_OF_RANGE_CSV, (csv) => {
        assert.throws(
          () => lindera.loadUserDictionaryFor(csv, dictionary),
          /line 2 \(surface: '東京スカイツリー'\): left context id 60000 is out of range/,
        );
      });
    });

    it("loads a CSV inside the matrix", () => {
      const dictionary = lindera.loadDictionary("embedded://ipadic");
      const baseline = new lindera.Tokenizer(dictionary, lindera.Mode.normal())
        .tokenize("東京スカイツリー")
        .map((token) => token.surface);
      assert.ok(baseline.length > 1, `fixture assumption: ${baseline}`);

      const userDictionary = withCsv(SIMPLE_CSV, (csv) =>
        lindera.loadUserDictionaryFor(csv, dictionary),
      );
      const tokenizer = new lindera.Tokenizer(
        dictionary,
        lindera.Mode.normal(),
        userDictionary,
      );
      const surfaces = tokenizer
        .tokenize("東京スカイツリー")
        .map((token) => token.surface);
      assert.deepStrictEqual(surfaces, ["東京スカイツリー"]);
    });
  },
);

describe(
  "buildUserDictionaryFor",
  { skip: !hasEmbeddedIpadic && "embedded://ipadic not available" },
  () => {
    it("names the CSV line and surface of a row outside the matrix", () => {
      const dictionary = lindera.loadDictionary("embedded://ipadic");
      withCsv(OUT_OF_RANGE_CSV, (csv) => {
        const output = path.join(path.dirname(csv), "userdic.bin");
        assert.throws(
          () => lindera.buildUserDictionaryFor(csv, output, dictionary),
          /line 2 \(surface: '東京スカイツリー'\): left context id 60000 is out of range/,
        );
        assert.strictEqual(fs.existsSync(output), false);
      });
    });

    it("writes a CSV inside the matrix to the output file", () => {
      const dictionary = lindera.loadDictionary("embedded://ipadic");
      const userDictionary = withCsv(SIMPLE_CSV, (csv) => {
        const output = path.join(path.dirname(csv), "out", "userdic.bin");
        lindera.buildUserDictionaryFor(csv, output, dictionary);
        assert.ok(fs.statSync(output).isFile());
        return lindera.loadUserDictionaryFor(output, dictionary);
      });
      const tokenizer = new lindera.Tokenizer(
        dictionary,
        lindera.Mode.normal(),
        userDictionary,
      );
      const surfaces = tokenizer
        .tokenize("東京スカイツリー")
        .map((token) => token.surface);
      assert.deepStrictEqual(surfaces, ["東京スカイツリー"]);
    });
  },
);

describe(
  "buildUserDictionary",
  { skip: !hasEmbeddedIpadic && "embedded://ipadic not available" },
  () => {
    // `outputDir` is the path of the file to write, not a directory.
    it("writes the file it is given", () => {
      const dictionary = lindera.loadDictionary("embedded://ipadic");
      withCsv(SIMPLE_CSV, (csv) => {
        const output = path.join(path.dirname(csv), "userdic.bin");
        lindera.buildUserDictionary("ipadic", csv, output);
        assert.ok(fs.statSync(output).isFile());
        lindera.loadUserDictionaryFor(output, dictionary);
      });
    });
  },
);
