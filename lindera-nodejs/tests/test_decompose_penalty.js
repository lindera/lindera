// The penalty of decompose mode: TokenizerBuilder.setMode and the Tokenizer
// constructor take a Mode, which holds the penalty (lindera/lindera#1215,
// #1218).
const assert = require("node:assert");
const { describe, it } = require("node:test");

const {
  Mode,
  Tokenizer,
  TokenizerBuilder,
  loadDictionary,
} = require("../index.js");

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
const skip = !hasEmbeddedIpadic && "embedded://ipadic not available";

const TEXT = "関西国際空港限定トートバッグ";
const NORMAL_SURFACES = ["関西国際空港", "限定", "トートバッグ"];
const DEFAULT_SURFACES = ["関西", "国際", "空港", "限定", "トートバッグ"];
// Without length penalties, decompose mode keeps the compound whole, as
// normal mode does.
const NO_LENGTH_PENALTY = {
  kanjiPenaltyLengthPenalty: 0,
  otherPenaltyLengthPenalty: 0,
};
const NO_LENGTH_PENALTY_SURFACES = NORMAL_SURFACES;
// With a katakana threshold of 4, the output differs from both normal mode's
// and the default penalty's, so a decompose mode that loses its penalty, or
// is read as normal mode, fails.
const SHORT_KATAKANA_THRESHOLD = {
  otherPenaltyLengthThreshold: 4,
  otherPenaltyLengthPenalty: 3000,
};
const SHORT_KATAKANA_SURFACES = [
  "関西",
  "国際",
  "空港",
  "限定",
  "トー",
  "トバッグ",
];

// [label, a function that makes the mode, the surfaces of TEXT].
const MODES = [
  ["normal", () => Mode.normal(), NORMAL_SURFACES],
  ["decompose", () => Mode.decompose(), DEFAULT_SURFACES],
  ["decompose(undefined)", () => Mode.decompose(undefined), DEFAULT_SURFACES],
  ["decompose(null)", () => Mode.decompose(null), DEFAULT_SURFACES],
  ["decompose({})", () => Mode.decompose({}), DEFAULT_SURFACES],
  [
    "no length penalty",
    () => Mode.decompose(NO_LENGTH_PENALTY),
    NO_LENGTH_PENALTY_SURFACES,
  ],
  // With only the kanji penalty 0, the other penalty does not matter for
  // this text, whose long words are all kanji.
  [
    "a field left out keeps its default",
    () => Mode.decompose({ kanjiPenaltyLengthPenalty: 0 }),
    NO_LENGTH_PENALTY_SURFACES,
  ],
  [
    "short katakana threshold",
    () => Mode.decompose(SHORT_KATAKANA_THRESHOLD),
    SHORT_KATAKANA_SURFACES,
  ],
  ['fromStr("decompose")', () => Mode.fromStr("decompose"), DEFAULT_SURFACES],
];

// Returns the surfaces of TEXT for a tokenizer built with setMode(mode).
const builderSurfaces = (mode) => {
  const builder = new TokenizerBuilder();
  builder.setDictionary("embedded://ipadic");
  builder.setMode(mode);
  return builder
    .build()
    .tokenize(TEXT)
    .map((token) => token.surface);
};

// Returns the surfaces of TEXT for a Tokenizer built directly with these
// arguments after the dictionary.
const constructorSurfaces = (...args) =>
  new Tokenizer(loadDictionary("embedded://ipadic"), ...args)
    .tokenize(TEXT)
    .map((token) => token.surface);

// Values that setMode and the constructor reject as not a Mode.
const NOT_A_MODE = [
  "decompose",
  "normal",
  0,
  true,
  {},
  { name: "decompose" },
  NO_LENGTH_PENALTY,
];

describe("TokenizerBuilder.setMode", () => {
  it("passes the penalty of the Mode to the segmenter", { skip }, () => {
    for (const [label, makeMode, expected] of MODES) {
      assert.deepStrictEqual(builderSurfaces(makeMode()), expected, label);
    }
  });

  it("returns the builder", () => {
    const builder = new TokenizerBuilder();
    assert.strictEqual(
      builder.setMode(Mode.decompose(NO_LENGTH_PENALTY)),
      builder,
    );
  });

  it("rejects a value that is not a Mode", () => {
    for (const value of [...NOT_A_MODE, null, undefined]) {
      assert.throws(
        () => new TokenizerBuilder().setMode(value),
        { code: "InvalidArg" },
        `value ${JSON.stringify(value)}`,
      );
    }
    assert.throws(
      () => new TokenizerBuilder().setMode(),
      { code: "InvalidArg" },
      "no argument",
    );
  });
});

describe("Tokenizer constructor", () => {
  it("passes the penalty of the Mode to the segmenter", { skip }, () => {
    for (const [label, makeMode, expected] of MODES) {
      assert.deepStrictEqual(constructorSurfaces(makeMode()), expected, label);
      assert.deepStrictEqual(
        constructorSurfaces(makeMode(), null, null),
        expected,
        `${label}, nulls`,
      );
    }
  });

  it("defaults to normal mode", { skip }, () => {
    assert.deepStrictEqual(constructorSurfaces(), NORMAL_SURFACES, "omitted");
    assert.deepStrictEqual(
      constructorSurfaces(undefined),
      NORMAL_SURFACES,
      "undefined",
    );
    assert.deepStrictEqual(constructorSurfaces(null), NORMAL_SURFACES, "null");
  });

  it("rejects a value that is not a Mode", { skip }, () => {
    for (const value of NOT_A_MODE) {
      assert.throws(
        () => constructorSurfaces(value),
        { code: "InvalidArg" },
        `value ${JSON.stringify(value)}`,
      );
    }
  });
});

describe("one Mode", () => {
  it("serves several tokenizers", { skip }, () => {
    const mode = Mode.decompose(SHORT_KATAKANA_THRESHOLD);
    assert.deepStrictEqual(builderSurfaces(mode), SHORT_KATAKANA_SURFACES);
    assert.deepStrictEqual(constructorSurfaces(mode), SHORT_KATAKANA_SURFACES);
    assert.deepStrictEqual(builderSurfaces(mode), SHORT_KATAKANA_SURFACES);
    assert.deepStrictEqual(constructorSurfaces(mode), SHORT_KATAKANA_SURFACES);
    assert.strictEqual(mode.penalty.otherPenaltyLengthThreshold, 4);
  });
});
