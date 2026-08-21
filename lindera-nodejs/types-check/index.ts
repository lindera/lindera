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
  loadDictionary,
  Tokenizer,
  type Token,
  type NbestResult,
} from "../index.js";

export function checkTokenShape(text: string): void {
  const dictionary = loadDictionary("embedded://ipadic");
  const tokenizer = new Tokenizer(dictionary, "normal");

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
