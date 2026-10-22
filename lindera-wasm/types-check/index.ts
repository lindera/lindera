// Type-level regression guard for the TypeScript declarations wasm-pack
// generates (`pkg-node/lindera_wasm.d.ts`).
//
// The Rust tests exercise the bindings at run time only, so nothing checked
// how TypeScript callers see the API. #1065 typed the constructor's new
// `space_penalty` parameter with `unchecked_param_type`, which wasm-bindgen
// treats as required, so it also dropped the `?` from `mode` and
// `user_dictionary` and `new Tokenizer(dictionary)` stopped compiling (#1076).
//
// Compiling this file with `tsc --noEmit` (`make types-check-lindera-wasm`)
// fails on such a change. It is never executed; only type-checked.

import {
  loadDictionary,
  Mode,
  Tokenizer,
  Penalty,
  TokenizerBuilder,
  type Dictionary,
  type UserDictionary,
} from "../pkg-node/lindera_wasm.js";

// Every documented form of the constructor: the arguments after the
// dictionary are optional. The user dictionary is taken as a `JsValue`, so
// that it stays usable (#1238), and is typed `UserDictionary` all the same.
export function checkTokenizerConstructorForms(
  dictionary: Dictionary,
  userDictionary: UserDictionary,
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
    new Tokenizer(dictionary, null, userDictionary),
    new Tokenizer(dictionary, undefined, userDictionary, false),
    // @ts-expect-error -- a mode is a Mode, not its name (#1218).
    new Tokenizer(dictionary, "normal"),
    // @ts-expect-error -- a user dictionary is a UserDictionary, not a path.
    new Tokenizer(dictionary, null, "user_dict.csv"),
  ];
}

// A `Mode` holds the penalty of decompose mode, as the Rust `Mode` does
// (#1218); `setMode` and its alias take it.
export function checkModeForms(): void {
  const penalty = new Penalty(undefined, 0, undefined, 0);
  const modes: Array<Mode> = [
    Mode.normal(),
    Mode.decompose(),
    Mode.decompose(penalty),
    Mode.decompose(null),
    Mode.decompose(undefined),
    Mode.fromStr("decompose"),
  ];
  const name: string = modes[0].name;
  const modePenalty: Penalty | undefined = modes[1].penalty;
  const kinds: Array<boolean> = [
    modes[0].isNormal(),
    modes[0].isDecompose(),
    modes[0].equals(modes[1]),
  ];
  const text: string = modes[0].toString();
  void [name, modePenalty, kinds, text];

  const builder: TokenizerBuilder = new TokenizerBuilder()
    .setMode(Mode.normal())
    .setMode(Mode.decompose(penalty))
    .set_mode(Mode.decompose());
  // @ts-expect-error -- the penalty is a Penalty.
  Mode.decompose({ kanji_penalty_length_penalty: 0 });
  // @ts-expect-error -- a mode is a Mode, not its name.
  builder.setMode("decompose");
  // @ts-expect-error -- the penalty goes in the Mode.
  builder.setMode(Mode.decompose(), penalty);
  // @ts-expect-error -- the copy function is hidden from TypeScript.
  Mode.__copyOf(Mode.normal());
  void builder;
}

export function checkTokenize(text: string): void {
  const tokenizer: Tokenizer = new Tokenizer(
    loadDictionary("embedded://ipadic"),
  );
  const tokens: Array<unknown> = tokenizer.tokenize(text);
  const surfaces: Array<string> = tokenizer.tokenizeSurfaces(text);
  void [tokens, surfaces];
}

export function checkSpacePenaltyForms(): void {
  const builder: TokenizerBuilder = new TokenizerBuilder()
    .setSpacePenalty(null)
    .setSpacePenalty(undefined)
    .setSpacePenalty(true)
    .setSpacePenalty(false)
    .setSpacePenalty({ rules: [{ pos: ["JKS", "JX"], cost: 6000 }] });
  void builder;
}

// `setSkipWhitespace` and its alias type their argument with
// `unchecked_param_type`: a boolean, `null` or `undefined`, and required.
export function checkSkipWhitespaceForms(): void {
  const builder: TokenizerBuilder = new TokenizerBuilder()
    .setSkipWhitespace(true)
    .setSkipWhitespace(false)
    .setSkipWhitespace(null)
    .setSkipWhitespace(undefined)
    .set_skip_whitespace(false)
    .set_skip_whitespace(null);
  // @ts-expect-error -- the argument is required.
  builder.setSkipWhitespace();
  // @ts-expect-error -- the argument is required.
  builder.set_skip_whitespace();
  void builder;
}
