const assert = require("node:assert");
const path = require("node:path");
const { describe, it } = require("node:test");

const { Metadata, TokenizerBuilder, loadDictionary } = require("../index.js");

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

// ko-dic is the one embedded dictionary that ships space_penalty rules, so it is
// the only place where the default and "off" give different output. It is behind
// the `embed-ko-dic` feature, which is not on by default.
const hasEmbeddedKoDic = (() => {
  try {
    loadDictionary("embedded://ko-dic");
    return true;
  } catch {
    return false;
  }
})();

const KO_DIC_METADATA = path.join(
  __dirname,
  "..",
  "..",
  "lindera-ko-dic",
  "metadata.json",
);

const RULES = { rules: [{ pos: ["JKS", "JX"], cost: 6000 }] };

describe("TokenizerBuilder.setSpacePenalty", () => {
  it("accepts null, undefined, booleans and a rules object", () => {
    for (const value of [null, undefined, true, false, RULES, { rules: [] }]) {
      const builder = new TokenizerBuilder();
      assert.strictEqual(
        builder.setSpacePenalty(value),
        builder,
        `value ${JSON.stringify(value)}`,
      );
    }
  });

  it("rejects values that are not a space-penalty setting", () => {
    for (const value of [
      1,
      "false",
      ["JKS"],
      { rules: "JKS" },
      { rules: [{ pos: ["JKS"] }] },
    ]) {
      const builder = new TokenizerBuilder();
      assert.throws(
        () => builder.setSpacePenalty(value),
        { name: "Error", message: /space_penalty/ },
        `value ${JSON.stringify(value)}`,
      );
    }
  });

  it(
    "requiring the dictionary's rules fails to build with IPADIC, which ships none",
    {
      skip: !hasEmbeddedIpadic && "embedded://ipadic not available",
    },
    () => {
      const builder = new TokenizerBuilder()
        .setDictionary("embedded://ipadic")
        .setSpacePenalty(true);
      assert.throws(() => builder.build(), {
        name: "Error",
        message: /ships no space_penalty rules/,
      });
    },
  );

  it(
    "builds with IPADIC when the penalty is off, default or given explicit rules",
    {
      skip: !hasEmbeddedIpadic && "embedded://ipadic not available",
    },
    () => {
      for (const value of [false, null, RULES]) {
        const tokenizer = new TokenizerBuilder()
          .setDictionary("embedded://ipadic")
          .setSpacePenalty(value)
          .build();
        const surfaces = tokenizer.tokenizeSurfaces("日本語");
        assert.ok(surfaces.length > 0, `value ${JSON.stringify(value)}`);
      }
    },
  );

  it(
    "null restores the default after the penalty was required",
    {
      skip: !hasEmbeddedIpadic && "embedded://ipadic not available",
    },
    () => {
      const builder = new TokenizerBuilder()
        .setDictionary("embedded://ipadic")
        .setSpacePenalty(true)
        .setSpacePenalty(null);
      assert.doesNotThrow(() => builder.build());
    },
  );

  it(
    "applies ko-dic's shipped rules unless the penalty is turned off",
    {
      skip: !hasEmbeddedKoDic && "embedded://ko-dic not available",
    },
    () => {
      // With the penalty, "시" after a space is not split off as the
      // honorific ending (EP); without it, it is.
      const posOfSi = (builder) => {
        const tokens = builder
          .setDictionary("embedded://ko-dic")
          .build()
          .tokenize("서울 시 에서 출발");
        const token = tokens.find((t) => t.surface === "시");
        assert.ok(token, "no token with surface 시");
        return token.details[0];
      };
      const shippedRules = JSON.parse(
        Metadata.fromJsonFile(KO_DIC_METADATA).toObject().spacePenalty,
      );

      assert.strictEqual(posOfSi(new TokenizerBuilder()), "NNG", "no call");
      for (const value of [undefined, null, true, shippedRules]) {
        assert.strictEqual(
          posOfSi(new TokenizerBuilder().setSpacePenalty(value)),
          "NNG",
          `value ${JSON.stringify(value)}`,
        );
      }
      for (const value of [false, { rules: [] }]) {
        assert.strictEqual(
          posOfSi(new TokenizerBuilder().setSpacePenalty(value)),
          "EP",
          `value ${JSON.stringify(value)}`,
        );
      }
    },
  );
});

describe("Metadata space penalty", () => {
  it("toObject exposes the rules ko-dic's metadata.json ships", () => {
    const metadata = Metadata.fromJsonFile(KO_DIC_METADATA);
    const obj = metadata.toObject();
    assert.strictEqual(typeof obj.spacePenalty, "string");
    assert.ok(obj.spacePenalty.includes("JKS"));

    const config = JSON.parse(obj.spacePenalty);
    assert.ok(Array.isArray(config.rules) && config.rules.length > 0);
  });

  it("toObject omits spacePenalty when the metadata has no rules", () => {
    const obj = new Metadata().toObject();
    assert.strictEqual(Object.hasOwn(obj, "spacePenalty"), false);
  });
});
