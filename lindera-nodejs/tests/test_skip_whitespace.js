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

// ko-dic is behind the `embed-ko-dic` feature, which is not on by default.
const hasEmbeddedKoDic = (() => {
  try {
    loadDictionary("embedded://ko-dic");
    return true;
  } catch {
    return false;
  }
})();

// Returns details[0] of the token whose surface is `surface`.
const posOf = (builder, text, surface) => {
  const token = builder
    .build()
    .tokenize(text)
    .find((t) => t.surface === surface);
  assert.ok(token, `no token with surface ${surface}`);
  return token.details[0];
};

// With whitespace skipped (the default for IPADIC), "が" connects to the word before
// the space and reads as a particle (助詞), as in MeCab; with the whitespace node in
// the lattice it reads as a conjunction (接続詞).
const TEXT = "Google が 新しい";

const ipadicBuilder = (...values) => {
  const builder = new TokenizerBuilder().setDictionary("embedded://ipadic");
  for (const value of values) {
    builder.setSkipWhitespace(value);
  }
  return builder;
};

describe("TokenizerBuilder.setSkipWhitespace", () => {
  it("accepts booleans, null and undefined, and returns the builder", () => {
    for (const value of [true, false, null, undefined]) {
      const builder = new TokenizerBuilder();
      assert.strictEqual(builder.setSkipWhitespace(value), builder);
    }
  });

  it("rejects values that are not booleans", () => {
    for (const value of ["false", 0, 1]) {
      assert.throws(
        () => new TokenizerBuilder().setSkipWhitespace(value),
        `value ${JSON.stringify(value)}`,
      );
    }
  });

  it(
    "turns whitespace skipping off and on with IPADIC",
    {
      skip: !hasEmbeddedIpadic && "embedded://ipadic not available",
    },
    () => {
      const cases = [
        [[], "助詞"],
        [[true], "助詞"],
        [[false], "接続詞"],
        // null and undefined go back to the dictionary's default, which skips.
        [[false, null], "助詞"],
        [[false, undefined], "助詞"],
        // The last call wins.
        [[true, false], "接続詞"],
      ];
      for (const [values, expected] of cases) {
        assert.strictEqual(
          posOf(ipadicBuilder(...values), TEXT, "が"),
          expected,
          `values ${JSON.stringify(values)}`,
        );
      }
    },
  );

  it(
    "has no effect while whitespace tokens are kept",
    {
      skip: !hasEmbeddedIpadic && "embedded://ipadic not available",
    },
    () => {
      const surfaces = (builder) =>
        builder.setKeepWhitespace(true).build().tokenizeSurfaces(TEXT);
      const kept = surfaces(ipadicBuilder());
      assert.ok(kept.includes(" "), JSON.stringify(kept));
      assert.deepStrictEqual(surfaces(ipadicBuilder(true)), kept);
    },
  );

  it(
    "turns whitespace skipping off and on with ko-dic",
    {
      skip: !hasEmbeddedKoDic && "embedded://ko-dic not available",
    },
    () => {
      // With whitespace skipped, "전" in "2년 전 대회" is the noun NNG, as in
      // mecab-ko; with the whitespace node, which resets the context at the space,
      // it is 저/NP + ㄴ/JX.
      const posOfJeon = (...values) => {
        const builder = new TokenizerBuilder().setDictionary(
          "embedded://ko-dic",
        );
        for (const value of values) {
          builder.setSkipWhitespace(value);
        }
        return posOf(builder, "2년 전 대회", "전");
      };
      assert.strictEqual(posOfJeon(), "NNG");
      assert.strictEqual(posOfJeon(true), "NNG");
      assert.strictEqual(posOfJeon(false), "NP+JX");
      assert.strictEqual(posOfJeon(false, null), "NNG");
    },
  );
});
