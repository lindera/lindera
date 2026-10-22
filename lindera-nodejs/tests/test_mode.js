// Mode, the counterpart of the Rust lindera::mode::Mode (lindera/lindera#1218).
const assert = require("node:assert");
const { describe, it } = require("node:test");

const lindera = require("../index.js");
const { Mode } = lindera;

const DEFAULT_PENALTY = {
  kanjiPenaltyLengthThreshold: 2,
  kanjiPenaltyLengthPenalty: 3000,
  otherPenaltyLengthThreshold: 7,
  otherPenaltyLengthPenalty: 1700,
};
const SHORT_KATAKANA_THRESHOLD = {
  otherPenaltyLengthThreshold: 4,
  otherPenaltyLengthPenalty: 3000,
};

describe("Mode", () => {
  it("is exported as Mode, and not as JsMode", () => {
    assert.strictEqual(typeof Mode, "function");
    assert.strictEqual(lindera.JsMode, undefined);
  });

  it("normal", () => {
    const mode = Mode.normal();
    assert.ok(mode instanceof Mode);
    assert.strictEqual(mode.name, "normal");
    assert.strictEqual(mode.toString(), "normal");
    assert.strictEqual(`${mode}`, "normal");
    assert.strictEqual(mode.isNormal(), true);
    assert.strictEqual(mode.isDecompose(), false);
    assert.strictEqual(mode.penalty, null);
  });

  it("decompose with the default penalty", () => {
    for (const mode of [
      Mode.decompose(),
      Mode.decompose(undefined),
      Mode.decompose(null),
      Mode.decompose({}),
    ]) {
      assert.strictEqual(mode.name, "decompose");
      assert.strictEqual(mode.toString(), "decompose");
      assert.strictEqual(mode.isDecompose(), true);
      assert.strictEqual(mode.isNormal(), false);
      assert.deepStrictEqual(mode.penalty, DEFAULT_PENALTY);
    }
  });

  it("decompose keeps the penalty, with the fields left out at their defaults", () => {
    assert.deepStrictEqual(Mode.decompose(SHORT_KATAKANA_THRESHOLD).penalty, {
      ...DEFAULT_PENALTY,
      ...SHORT_KATAKANA_THRESHOLD,
    });
  });

  it("returns a copy of the penalty", () => {
    const mode = Mode.decompose();
    mode.penalty.kanjiPenaltyLengthPenalty = 0;
    assert.strictEqual(mode.penalty.kanjiPenaltyLengthPenalty, 3000);

    const given = { kanjiPenaltyLengthPenalty: 1000 };
    const fromGiven = Mode.decompose(given);
    given.kanjiPenaltyLengthPenalty = 0;
    assert.strictEqual(fromGiven.penalty.kanjiPenaltyLengthPenalty, 1000);
  });

  it("decompose rejects a penalty that is not an object or a field out of its range", () => {
    for (const value of [0, "penalty", true]) {
      assert.throws(
        () => Mode.decompose(value),
        { code: "InvalidArg", message: /Expect value to be Object/ },
        `value ${JSON.stringify(value)}`,
      );
    }
    for (const value of [
      { kanjiPenaltyLengthPenalty: "0" },
      { kanjiPenaltyLengthPenalty: null },
    ]) {
      assert.throws(
        () => Mode.decompose(value),
        { code: "NumberExpected", message: /kanjiPenaltyLengthPenalty/ },
        `value ${JSON.stringify(value)}`,
      );
    }
    assert.throws(() => Mode.decompose({ kanjiPenaltyLengthThreshold: -1 }), {
      code: "InvalidArg",
      message: /kanjiPenaltyLengthThreshold/,
    });
  });

  it("fromStr", () => {
    assert.ok(Mode.fromStr("normal").equals(Mode.normal()));
    assert.ok(Mode.fromStr("decompose").equals(Mode.decompose()));
  });

  it("fromStr rejects other names, case sensitive as the Rust Mode parses", () => {
    for (const name of ["Normal", "Decompose", "DECOMPOSE", "", "invalid"]) {
      assert.throws(
        () => Mode.fromStr(name),
        { message: /Invalid mode/ },
        `name ${JSON.stringify(name)}`,
      );
    }
    assert.throws(() => Mode.fromStr(0), { code: "StringExpected" });
  });

  it("equals compares the name and the penalty", () => {
    assert.ok(Mode.normal().equals(Mode.normal()));
    assert.ok(Mode.decompose().equals(Mode.decompose({})));
    assert.ok(Mode.decompose().equals(Mode.decompose(DEFAULT_PENALTY)));
    assert.ok(
      Mode.decompose(SHORT_KATAKANA_THRESHOLD).equals(
        Mode.decompose(SHORT_KATAKANA_THRESHOLD),
      ),
    );
    assert.ok(
      !Mode.decompose().equals(Mode.decompose(SHORT_KATAKANA_THRESHOLD)),
    );
    assert.ok(!Mode.normal().equals(Mode.decompose()));
    const mode = Mode.normal();
    assert.ok(mode.equals(mode));
    assert.notStrictEqual(Mode.normal(), Mode.normal());
  });

  it("equals rejects a value that is not a Mode", () => {
    for (const value of ["normal", 0, null, undefined, {}, DEFAULT_PENALTY]) {
      assert.throws(
        () => Mode.normal().equals(value),
        { code: "InvalidArg" },
        `value ${JSON.stringify(value)}`,
      );
    }
  });

  it("has no constructor", () => {
    assert.throws(() => new Mode());
  });

  it("cannot be changed", () => {
    const mode = Mode.normal();
    assert.throws(() => {
      "use strict";
      mode.name = "decompose";
    }, TypeError);
    assert.strictEqual(mode.name, "normal");
  });
});
