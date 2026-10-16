// Integer arguments reject a number the setting cannot hold (lindera/lindera#1209).
//
// napi-rs used to read these parameters as u32 / i32, converting a JS number
// with ToUint32 / ToInt32 before the binding saw it: -1 became 4294967295,
// 2 ** 32 + 5 became 5, and NaN, Infinity and fractions became other integers.
// The binding now takes the number as it is and throws an InvalidArg error
// that names the parameter and the accepted range.
const assert = require("node:assert");
const { describe, it } = require("node:test");

const lindera = require("../index.js");
const { Metadata, Schema, Tokenizer, TokenizerBuilder, loadDictionary } = lindera;

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
const hasTrainFeature = typeof lindera.train === "function";

const USIZE_MAX = "18446744073709551615";
const I64_RANGE = ["-9223372036854775808", "9223372036854775807"];

// Values that are not integers, which every integer parameter rejects.
const NOT_INTEGERS = [
  [NaN, "NaN"],
  [Infinity, "Infinity"],
  [-Infinity, "-Infinity"],
  [1.5, "1.5"],
];

// A path that does not exist: train() reads files only after checking its
// integers, so a rejected integer is reported before the missing files.
const MISSING = "/nonexistent/issue1209";

function cases() {
  const result = {
    "new Metadata({defaultWordCost})": {
      call: (v) => new Metadata({ defaultWordCost: v }),
      name: "defaultWordCost",
      range: ["-32768", "32767"],
      outside: [-32769, 32768, 100000, 2 ** 31],
    },
    "new Metadata({defaultLeftContextId})": {
      call: (v) => new Metadata({ defaultLeftContextId: v }),
      name: "defaultLeftContextId",
      range: ["0", "65535"],
      outside: [-1, 65536, 2 ** 32 + 5],
    },
    "new Metadata({defaultRightContextId})": {
      call: (v) => new Metadata({ defaultRightContextId: v }),
      name: "defaultRightContextId",
      range: ["0", "65535"],
      outside: [-1, 65536],
    },
    "metadata.defaultWordCost =": {
      call: (v) => {
        new Metadata().defaultWordCost = v;
      },
      name: "defaultWordCost",
      range: ["-32768", "32767"],
      outside: [-32769, 32768],
    },
    "metadata.defaultLeftContextId =": {
      call: (v) => {
        new Metadata().defaultLeftContextId = v;
      },
      name: "defaultLeftContextId",
      range: ["0", "65535"],
      outside: [-1, 65536],
    },
    "metadata.defaultRightContextId =": {
      call: (v) => {
        new Metadata().defaultRightContextId = v;
      },
      name: "defaultRightContextId",
      range: ["0", "65535"],
      outside: [-1, 65536],
    },
    "schema.getFieldName(index)": {
      call: (v) => Schema.createDefault().getFieldName(v),
      name: "index",
      range: ["0", USIZE_MAX],
      outside: [-1],
    },
    // The fields of a Penalty (#1215).
    "builder.setMode(mode, {kanjiPenaltyLengthThreshold})": {
      call: (v) => new TokenizerBuilder().setMode("decompose", { kanjiPenaltyLengthThreshold: v }),
      name: "kanjiPenaltyLengthThreshold",
      range: ["0", USIZE_MAX],
      outside: [-1],
    },
    "builder.setMode(mode, {kanjiPenaltyLengthPenalty})": {
      call: (v) => new TokenizerBuilder().setMode("decompose", { kanjiPenaltyLengthPenalty: v }),
      name: "kanjiPenaltyLengthPenalty",
      range: ["-2147483648", "2147483647"],
      outside: [-(2 ** 31) - 1, 2 ** 31],
    },
    "builder.setMode(mode, {otherPenaltyLengthThreshold})": {
      call: (v) => new TokenizerBuilder().setMode("decompose", { otherPenaltyLengthThreshold: v }),
      name: "otherPenaltyLengthThreshold",
      range: ["0", USIZE_MAX],
      outside: [-1],
    },
    "builder.setMode(mode, {otherPenaltyLengthPenalty})": {
      call: (v) => new TokenizerBuilder().setMode("decompose", { otherPenaltyLengthPenalty: v }),
      name: "otherPenaltyLengthPenalty",
      range: ["-2147483648", "2147483647"],
      outside: [-(2 ** 31) - 1, 2 ** 31],
    },
  };
  if (hasEmbeddedIpadic) {
    const tokenizer = new Tokenizer(loadDictionary("embedded://ipadic"));
    Object.assign(result, {
      "tokenizer.tokenizeNbest(text, n)": {
        call: (v) => tokenizer.tokenizeNbest("東京", v),
        name: "n",
        range: ["0", USIZE_MAX],
        outside: [-1],
      },
      "tokenizer.tokenizeNbest(text, n, unique, costThreshold)": {
        call: (v) => tokenizer.tokenizeNbest("東京", 2, false, v),
        name: "costThreshold",
        range: I64_RANGE,
        outside: [1e300, -1e300],
      },
      "new Tokenizer(dictionary, mode, null, null, {kanjiPenaltyLengthPenalty})": {
        call: (v) =>
          new Tokenizer(loadDictionary("embedded://ipadic"), "decompose", null, null, {
            kanjiPenaltyLengthPenalty: v,
          }),
        name: "kanjiPenaltyLengthPenalty",
        range: ["-2147483648", "2147483647"],
        outside: [-(2 ** 31) - 1, 2 ** 31],
      },
    });
  }
  if (hasTrainFeature) {
    const options = {
      seed: MISSING,
      corpus: MISSING,
      charDef: MISSING,
      unkDef: MISSING,
      featureDef: MISSING,
      rewriteDef: MISSING,
      output: MISSING,
    };
    Object.assign(result, {
      "train({maxIter})": {
        call: (v) => lindera.train({ ...options, maxIter: v }),
        name: "maxIter",
        range: ["0", "18446744073709551615"],
        outside: [-1],
      },
      "train({maxThreads})": {
        call: (v) => lindera.train({ ...options, maxThreads: v }),
        name: "maxThreads",
        range: ["0", USIZE_MAX],
        outside: [-1],
      },
    });
  }
  return result;
}

// Formats a number as the error message does.
function format(value) {
  return Number.isInteger(value) && Math.abs(value) < 1e21
    ? BigInt(value).toString()
    : String(value);
}

function assertRejected(label, { call, name, range }, value, shown) {
  assert.throws(
    () => call(value),
    (error) => {
      assert.strictEqual(error.code, "InvalidArg", `${label} <- ${shown}`);
      assert.strictEqual(
        error.message,
        `${name} must be an integer between ${range[0]} and ${range[1]}, got ${shown}`,
      );
      return true;
    },
    `${label} accepted ${shown}`,
  );
}

describe("Integer arguments", () => {
  for (const [label, c] of Object.entries(cases())) {
    describe(label, () => {
      it("rejects an integer outside the range", () => {
        for (const value of c.outside) {
          assertRejected(label, c, value, format(value));
        }
      });

      it("rejects a number that is not an integer", () => {
        for (const [value, shown] of NOT_INTEGERS) {
          assertRejected(label, c, value, shown);
        }
      });

      it("accepts the bounds", () => {
        const [min, max] = c.range.map(Number);
        for (const value of [min, Math.min(max, Number.MAX_SAFE_INTEGER)]) {
          try {
            c.call(value);
          } catch (error) {
            // train() still throws for its missing files, after taking the value.
            assert.ok(
              !error.message.includes("must be an integer between"),
              `${label} rejected ${value}: ${error.message}`,
            );
          }
        }
      });
    });
  }

  it("keeps an accepted value unchanged", () => {
    const metadata = new Metadata({
      defaultWordCost: -32768,
      defaultLeftContextId: 65535,
    });
    assert.strictEqual(metadata.defaultWordCost, -32768);
    assert.strictEqual(metadata.defaultLeftContextId, 65535);
    // 2 ** 32 used to wrap to 0 and return the first field.
    assert.strictEqual(Schema.createDefault().getFieldName(2 ** 32), null);
  });

  it("leaves a setting unchanged when the setter rejects a number", () => {
    const metadata = new Metadata({ defaultWordCost: 100 });
    assert.throws(() => {
      metadata.defaultWordCost = 100000;
    });
    assert.strictEqual(metadata.defaultWordCost, 100);
  });
});
