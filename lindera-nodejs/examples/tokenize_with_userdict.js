const path = require("path");
const {
  loadDictionary,
  loadUserDictionaryFor,
  Tokenizer,
} = require("../index.js");

const projectRoot = path.resolve(__dirname, "..");

function main() {
  // load the dictionary
  const dictionary = loadDictionary("embedded://ipadic");

  // load the user dictionary for the dictionary
  const userDictionaryPath = path.join(
    projectRoot,
    "resources",
    "ipadic_simple_userdic.csv",
  );
  const userDictionary = loadUserDictionaryFor(userDictionaryPath, dictionary);

  // create a tokenizer
  const tokenizer = new Tokenizer(dictionary, "normal", userDictionary);

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
