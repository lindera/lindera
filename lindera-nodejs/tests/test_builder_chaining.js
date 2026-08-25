const assert = require("node:assert");
const { describe, it } = require("node:test");

const { TokenizerBuilder, loadDictionary } = require("../index.js");

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

describe("TokenizerBuilder method chaining", () => {
  it("every setter returns the same builder instance", () => {
    const builder = new TokenizerBuilder();
    assert.strictEqual(builder.setMode("normal"), builder);
    assert.strictEqual(builder.setDictionary("/tmp/nonexistent"), builder);
    assert.strictEqual(builder.setUserDictionary("/tmp/nonexistent"), builder);
    assert.strictEqual(builder.setKeepWhitespace(true), builder);
    assert.strictEqual(builder.appendCharacterFilter("unicode_normalize", { kind: "nfkc" }), builder);
    assert.strictEqual(builder.appendTokenFilter("lowercase", {}), builder);
  });

  it("setMode rejects an invalid mode even when chained", () => {
    const builder = new TokenizerBuilder();
    assert.throws(() => builder.setMode("no_such_mode"));
  });

  it("builds and tokenizes through a full chain", {
    skip: !hasEmbeddedIpadic && "embedded://ipadic not available"
  }, () => {
    const tokenizer = new TokenizerBuilder()
      .setMode("normal")
      .setDictionary("embedded://ipadic")
      .appendTokenFilter("japanese_stop_tags", {
        tags: ["助詞,係助詞", "助詞,連体化"],
      })
      .build();

    const surfaces = tokenizer.tokenize("すもももももももものうち").map((t) => t.surface);
    assert.deepStrictEqual(surfaces, ["すもも", "もも", "もも", "うち"]);
  });

  it("keeps the sequential (non-chained) style working", {
    skip: !hasEmbeddedIpadic && "embedded://ipadic not available"
  }, () => {
    const builder = new TokenizerBuilder();
    builder.setMode("normal");
    builder.setDictionary("embedded://ipadic");
    const tokenizer = builder.build();

    const surfaces = tokenizer.tokenize("日本語").map((t) => t.surface);
    assert.ok(surfaces.length > 0);
  });
});
