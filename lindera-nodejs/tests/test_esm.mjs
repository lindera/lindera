import assert from "node:assert";
import { describe, it } from "node:test";

// The package entry point is CommonJS, but the `exports` map declares an `import` condition so ESM
// consumers can `import` it directly. Node synthesises named exports from the static
// `module.exports.X = ...` assignments napi generates.
//
// Without the `import` condition this file fails with ERR_PACKAGE_PATH_NOT_EXPORTED when the
// package is consumed by name — the reason ESM users previously needed a `createRequire` shim.
import lindera, { Tokenizer, TokenizerBuilder, loadDictionary, version } from "../index.js";

// Matches the guard in test_tokenize_ipadic.js: the embedded dictionary is behind the
// `embed-ipadic` feature, which is not on by default.
const hasEmbeddedIpadic = (() => {
  try {
    loadDictionary("embedded://ipadic");
    return true;
  } catch {
    return false;
  }
})();

describe("ESM interop", () => {
  it("exposes named exports to ESM importers", () => {
    assert.strictEqual(typeof TokenizerBuilder, "function");
    assert.strictEqual(typeof Tokenizer, "function");
    assert.strictEqual(typeof loadDictionary, "function");
    assert.strictEqual(typeof version, "function");
  });

  it("exposes the same bindings via the default export", () => {
    assert.strictEqual(lindera.Tokenizer, Tokenizer);
    assert.strictEqual(lindera.loadDictionary, loadDictionary);
  });

  it("can tokenize through an ESM-imported binding", {
    skip: !hasEmbeddedIpadic && "embedded://ipadic not available"
  }, () => {
    const builder = new TokenizerBuilder();
    builder.setMode("normal");
    builder.setDictionary("embedded://ipadic");
    const tokenizer = builder.build();
    const tokens = tokenizer.tokenize("東京タワー");
    assert.ok(tokens.length > 0);
    assert.strictEqual(tokens[0].surface, "東京");
  });
});
