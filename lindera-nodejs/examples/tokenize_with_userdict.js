const path = require("path");
const {
  loadDictionary,
  loadUserDictionary,
  Tokenizer,
} = require("lindera-nodejs");

const projectRoot = path.resolve(__dirname, "..");

function main() {
  // load the dictionary
  const dictionary = loadDictionary("embedded://ipadic");

  const metadata = dictionary.metadata();

  // load the user dictionary
  const userDictionaryPath = path.join(
    projectRoot,
    "resources",
    "ipadic_simple_userdic.csv",
  );
  const userDictionary = loadUserDictionary(userDictionaryPath, metadata);

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
