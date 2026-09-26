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
  Tokenizer,
  TokenizerBuilder,
  type Dictionary,
} from "../pkg-node/lindera_wasm.js";

// Every documented form of the constructor: the arguments after the
// dictionary are optional.
export function checkTokenizerConstructorForms(dictionary: Dictionary): Array<Tokenizer> {
  return [
    new Tokenizer(dictionary),
    new Tokenizer(dictionary, "normal"),
    new Tokenizer(dictionary, "decompose", null),
    new Tokenizer(dictionary, undefined, undefined, false),
    new Tokenizer(dictionary, "normal", null, { rules: [{ pos: ["JKS", "JX"], cost: 6000 }] }),
  ];
}

export function checkTokenize(text: string): void {
  const tokenizer: Tokenizer = new Tokenizer(loadDictionary("embedded://ipadic"));
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
