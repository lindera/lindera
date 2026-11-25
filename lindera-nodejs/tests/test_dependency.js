// The token filter japanese_dependency (lindera/lindera#1438): a token
// object has a `dependency` property when the filter ran, and only then.
// The model is the one of the tests of lindera-analysis: a chunker over the
// part of speech and a parser that makes every bunsetsu depend on the next.
const assert = require("node:assert");
const path = require("node:path");
const { describe, it } = require("node:test");

const lindera = require("../index.js");

const MODEL = path.join(
  __dirname,
  "..",
  "..",
  "resources",
  "dependency",
  "test_model_ipadic.bin",
);
const TEXT = "彼は本を読んだ。";
const TOKEN_KEYS = [
  "surface",
  "byteStart",
  "byteEnd",
  "position",
  "wordId",
  "isUnknown",
  "details",
];
const DEPENDENCY_KEYS = ["sentence", "bunsetsu", "head", "role", "score"];

function builder() {
  return new lindera.TokenizerBuilder().setDictionary("embedded://ipadic");
}

// Whether the build has the embedded IPADIC and the filter: a build
// without the `dependency` feature refuses the filter by its name.
const reason = (() => {
  try {
    lindera.loadDictionary("embedded://ipadic");
  } catch {
    return "embedded://ipadic not available";
  }
  try {
    builder()
      .appendTokenFilter("japanese_dependency", { model: MODEL })
      .build();
  } catch (error) {
    if (String(error).includes("the feature `dependency`")) {
      return "built without the dependency feature";
    }
    throw error;
  }
  return false;
})();

function tokenizer() {
  return builder()
    .appendTokenFilter("japanese_dependency", { model: MODEL })
    .build();
}

function dependency(bunsetsu, head, role, score) {
  return { sentence: 0, bunsetsu, head, role, score };
}

describe("Dependency token filter", { skip: reason }, () => {
  it("gives a token its dependency", () => {
    const tokens = tokenizer().tokenize(TEXT);
    assert.deepStrictEqual(
      tokens.map((token) => [token.surface, token.dependency]),
      [
        ["彼", dependency(0, 1, "head", 1)],
        ["は", dependency(0, 1, "function", 1)],
        ["本", dependency(1, 2, "head", 0)],
        ["を", dependency(1, 2, "function", 0)],
        ["読ん", dependency(2, null, "head", 0)],
        ["だ", dependency(2, null, "function", 0)],
        ["。", dependency(2, null, "other", 0)],
      ],
    );
  });

  it("makes plain objects with the fields in their order", () => {
    for (const token of tokenizer().tokenize(TEXT)) {
      assert.deepStrictEqual(Object.keys(token), [...TOKEN_KEYS, "dependency"]);
      assert.deepStrictEqual(Object.keys(token.dependency), DEPENDENCY_KEYS);
      assert.strictEqual(
        Object.getPrototypeOf(token.dependency),
        Object.prototype,
      );
      for (const object of [token, token.dependency]) {
        for (const descriptor of Object.values(
          Object.getOwnPropertyDescriptors(object),
        )) {
          assert.ok(descriptor.writable);
          assert.ok(descriptor.enumerable);
          assert.ok(descriptor.configurable);
        }
      }
    }
    // null for the last bunsetsu, also as JSON.
    const last = tokenizer().tokenize(TEXT)[6];
    assert.strictEqual(last.dependency.head, null);
    assert.deepStrictEqual(
      JSON.parse(JSON.stringify(last)).dependency,
      dependency(2, null, "other", 0),
    );
  });

  it("leaves a token without the filter as it was", () => {
    for (const token of builder().build().tokenize(TEXT)) {
      assert.deepStrictEqual(Object.keys(token), TOKEN_KEYS);
      assert.strictEqual(token.dependency, undefined);
      assert.ok(!("dependency" in token));
    }
  });

  it("gives it from every method that returns tokens", async () => {
    const t = tokenizer();
    const expected = t.tokenize(TEXT);
    assert.deepStrictEqual(t.tokenizeBatch([TEXT, TEXT])[1], expected);
    assert.deepStrictEqual(
      (await t.tokenizeBatchAsync([TEXT, TEXT]))[1],
      expected,
    );
    assert.deepStrictEqual(t.tokenizeNbest(TEXT, 1)[0].tokens, expected);
    assert.deepStrictEqual(
      t.tokenizeNbestBatch([TEXT], 1)[0][0].tokens,
      expected,
    );
  });

  it("makes the objects of the derived conversion", (context) => {
    // The hooks exist only in a build with the `test-hooks` feature.
    if (typeof lindera.__tokenizeBatchDerivedForTest !== "function") {
      context.skip("built without the test-hooks feature");
      return;
    }
    const t = tokenizer();
    const texts = [TEXT, "", `${TEXT}本を読んだ。`];
    const derived = lindera.__tokenizeBatchDerivedForTest(t, texts);
    const made = t.tokenizeBatch(texts);
    assert.deepStrictEqual(made, derived);
    assert.deepStrictEqual(
      made.flat().map((token) => Object.keys(token)),
      derived.flat().map((token) => Object.keys(token)),
    );
    assert.deepStrictEqual(
      made.flat().map((token) => Object.keys(token.dependency)),
      derived.flat().map((token) => Object.keys(token.dependency)),
    );
    // Across handle scopes of two tokens each: the names of the fields are
    // made again in every scope.
    lindera.__setTokenScopeForTest(2);
    try {
      assert.deepStrictEqual(t.tokenizeBatch(texts), derived);
    } finally {
      lindera.__setTokenScopeForTest(null);
    }
  });

  it("numbers the sentences", () => {
    const tokens = tokenizer().tokenize(`${TEXT}本を読んだ。`);
    assert.deepStrictEqual(
      tokens.map((token) => token.dependency.sentence),
      [0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1],
    );
    // The bunsetsu of the second sentence start at 0 again.
    assert.strictEqual(tokens[7].surface, "本");
    assert.strictEqual(tokens[7].dependency.bunsetsu, 0);
  });

  it("refuses a model that cannot be read, when the tokenizer is built", () => {
    assert.throws(
      () =>
        builder()
          .appendTokenFilter("japanese_dependency", {
            model: "no/such/model.bin",
          })
          .build(),
      /no\/such\/model\.bin/,
    );
  });

  it("refuses tokens that an earlier filter removed", () => {
    const t = builder()
      .appendTokenFilter("japanese_stop_tags", { tags: ["助詞,係助詞"] })
      .appendTokenFilter("japanese_dependency", { model: MODEL })
      .build();
    assert.throws(() => t.tokenize(TEXT), /removed or joined/);
  });
});
