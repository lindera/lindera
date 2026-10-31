// Type-level regression guard for the generated `index.d.ts`.
//
// `napi build` regenerates `index.d.ts`, and CI already fails if the committed
// copy is stale. That check compares the file against a fresh regeneration, so
// it cannot catch a file that is internally inconsistent — it happily
// reproduces the same broken output. #959 was exactly that: `NbestResult.tokens`
// referenced a `JsTokenData` type the file never declared, so every TypeScript
// consumer of `tokenizeNbest` failed to compile while CI stayed green.
//
// Compiling this file with `tsc --noEmit` fails if any type the public API
// exposes does not resolve. It is never executed; only type-checked.

import {
  buildDictionary,
  loadDictionary,
  Metadata,
  Mode,
  Tokenizer,
  TokenizerBuilder,
  type Dictionary,
  type Token,
  type NbestResult,
  type Penalty,
} from "../index.js";

export function checkTokenShape(text: string): void {
  const dictionary = loadDictionary("embedded://ipadic");
  const tokenizer = new Tokenizer(dictionary, Mode.normal());

  // `tokenize` yields plain token objects (#953).
  const tokens: Array<Token> = tokenizer.tokenize(text);
  const token: Token = tokens[0];

  const surface: string = token.surface;
  const byteStart: number = token.byteStart;
  const byteEnd: number = token.byteEnd;
  const position: number = token.position;
  const wordId: number = token.wordId;
  const isUnknown: boolean = token.isUnknown;
  const details: Array<string> = token.details;

  void [surface, byteStart, byteEnd, position, wordId, isUnknown, details];

  // The surfaces fast path.
  const surfaces: Array<string> = tokenizer.tokenizeSurfaces(text);
  void surfaces;

  // `tokenizeNbest` is the API that #959 broke: its result type referenced an
  // undeclared type, so naming `NbestResult` here is the actual regression
  // guard.
  const nbest: Array<NbestResult> = tokenizer.tokenizeNbest(text, 3);
  const nbestTokens: Array<Token> = nbest[0].tokens;
  const cost: number = nbest[0].cost;

  void [nbestTokens, cost];
}

// `setSpacePenalty` declares its argument type by hand (`ts_args_type`), so
// check that the declaration parses and accepts every documented form.
export function checkSpacePenaltyForms(): void {
  const builder: TokenizerBuilder = new TokenizerBuilder()
    .setSpacePenalty(null)
    .setSpacePenalty(undefined)
    .setSpacePenalty(true)
    .setSpacePenalty(false)
    .setSpacePenalty({ rules: [{ pos: ["JKS", "JX"], cost: 6000 }] });
  void builder;
}

// `setSkipWhitespace` declares its argument type by hand too: a boolean, `null`
// or `undefined`, and the argument is required, as for `setSpacePenalty`.
export function checkSkipWhitespaceForms(): void {
  const builder: TokenizerBuilder = new TokenizerBuilder()
    .setSkipWhitespace(true)
    .setSkipWhitespace(false)
    .setSkipWhitespace(null)
    .setSkipWhitespace(undefined);
  // @ts-expect-error -- the argument is required.
  builder.setSkipWhitespace();
  void builder;
}

// Every documented form of the constructor: the arguments after the
// dictionary are optional. The WASM declarations lost their `?` once (#1076);
// this guards the Node.js ones the same way.
export function checkTokenizerConstructorForms(
  dictionary: Dictionary,
): Array<Tokenizer> {
  return [
    new Tokenizer(dictionary),
    new Tokenizer(dictionary, Mode.normal()),
    new Tokenizer(dictionary, Mode.decompose(), null),
    new Tokenizer(dictionary, null),
    new Tokenizer(dictionary, undefined, undefined, false),
    new Tokenizer(dictionary, Mode.normal(), null, {
      rules: [{ pos: ["JKS", "JX"], cost: 6000 }],
    }),
    // @ts-expect-error -- a mode is a Mode, not its name (#1218).
    new Tokenizer(dictionary, "normal"),
  ];
}

// The histogram of `buildDictionary` is optional, so the calls written before
// it was added keep compiling (#1309).
export function checkBuildDictionaryForms(metadata: Metadata): void {
  buildDictionary("/path/to/source", "/path/to/output", metadata);
  buildDictionary("/path/to/source", "/path/to/output", metadata, null);
  buildDictionary(
    "/path/to/source",
    "/path/to/output",
    metadata,
    "/path/to/context_id_freq.txt",
  );
  // @ts-expect-error -- the histogram is a path.
  buildDictionary("/path/to/source", "/path/to/output", metadata, 0);
}

// A `Mode` holds the penalty of decompose mode, as the Rust `Mode` does
// (#1218). A `Penalty` is a plain object whose fields are all optional
// (#1215).
export function checkModeForms(): void {
  const empty: Penalty = {};
  const full: Penalty = {
    kanjiPenaltyLengthThreshold: 2,
    kanjiPenaltyLengthPenalty: 3000,
    otherPenaltyLengthThreshold: 7,
    otherPenaltyLengthPenalty: 1700,
  };
  const modes: Array<Mode> = [
    Mode.normal(),
    Mode.decompose(),
    Mode.decompose(empty),
    Mode.decompose(full),
    Mode.decompose(null),
    Mode.decompose(undefined),
    Mode.fromStr("decompose"),
  ];
  const name: string = modes[0].name;
  const penalty: Penalty | null = modes[1].penalty;
  const kinds: Array<boolean> = [
    modes[0].isNormal(),
    modes[0].isDecompose(),
    modes[0].equals(modes[1]),
  ];
  const text: string = modes[0].toString();
  void [name, penalty, kinds, text];

  const builder: TokenizerBuilder = new TokenizerBuilder()
    .setMode(Mode.normal())
    .setMode(Mode.decompose(full));
  // @ts-expect-error -- a field is a number.
  Mode.decompose({ kanjiPenaltyLengthPenalty: "0" });
  // @ts-expect-error -- a mode is a Mode, not its name.
  builder.setMode("decompose");
  // @ts-expect-error -- the penalty goes in the Mode.
  builder.setMode(Mode.decompose(), full);
  void builder;
}
