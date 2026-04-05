const { TokenizerBuilder } = require("lindera");

function main() {
  // create a tokenizer builder
  const builder = new TokenizerBuilder();

  // set mode and dictionary
  builder.setMode("normal");
  builder.setDictionary("embedded://ipadic");

  // append character filters
  builder.appendCharacterFilter("unicode_normalize", { kind: "nfkc" });
  builder.appendCharacterFilter("japanese_iteration_mark", {
    normalize_kanji: "true",
    normalize_kana: "true",
  });
  builder.appendCharacterFilter("mapping", {
    mapping: { リンデラ: "lindera" },
  });

  // append token filters
  builder.appendTokenFilter("japanese_katakana_stem", { min: 3 });
  builder.appendTokenFilter("japanese_stop_tags", {
    tags: [
      "接続詞",
      "助詞",
      "助詞,格助詞",
      "助詞,格助詞,一般",
      "助詞,格助詞,引用",
      "助詞,格助詞,連語",
      "助詞,係助詞",
      "助詞,副助詞",
      "助詞,間投助詞",
      "助詞,並立助詞",
      "助詞,終助詞",
      "助詞,副助詞／並立助詞／終助詞",
      "助詞,連体化",
      "助詞,副詞化",
      "助詞,特殊",
      "助動詞",
      "記号",
      "記号,一般",
      "記号,読点",
      "記号,句点",
      "記号,空白",
      "記号,括弧閉",
      "その他,間投",
      "フィラー",
      "非言語音",
    ],
  });
  builder.appendTokenFilter("lowercase");

  // build the tokenizer
  const tokenizer = builder.build();

  const text =
    "Ｌｉｎｄｅｒａは形態素解析ｴﾝｼﾞﾝです。ユーザー辞書も利用可能で、様々なフィルターも内包しています。Linderaはリンデラと読みます。";
  console.log(`text: ${text}\n`);

  // tokenize the text
  const tokens = tokenizer.tokenize(text);

  for (const token of tokens) {
    console.log(token.surface);
  }
}

main();
