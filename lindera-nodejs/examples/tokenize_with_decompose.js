const { loadDictionary, Tokenizer } = require("lindera");

function main() {
  // load the dictionary
  const dictionary = loadDictionary("embedded://ipadic");

  // create a tokenizer with decompose mode
  const tokenizer = new Tokenizer(dictionary, "decompose");

  const text =
    "関西国際空港限定トートバッグを東京スカイツリーの最寄り駅であるとうきょうスカイツリー駅で買う";
  console.log(`text: ${text}\n`);

  // tokenize the text
  const tokens = tokenizer.tokenize(text);

  for (const token of tokens) {
    console.log(token.surface);
  }
}

main();
