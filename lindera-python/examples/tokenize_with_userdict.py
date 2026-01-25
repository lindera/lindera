from pathlib import Path

from lindera.dictionary import load_dictionary, load_user_dictionary
from lindera.tokenizer import Tokenizer

project_root = Path(__file__).resolve().parent.parent


def main():
    # load the dictionary
    dictionary = load_dictionary("embedded://ipadic")

    metadata = dictionary.metadata()

    # load the user dictionary
    user_dictionary_path = str(project_root / Path("./resources/ipadic_simple_userdic.csv"))
    user_dictionary = load_user_dictionary(user_dictionary_path, metadata)

    # create a tokenizer
    tokenizer = Tokenizer(dictionary, mode="normal", user_dictionary=user_dictionary)

    text = "関西国際空港限定トートバッグを東京スカイツリーの最寄り駅であるとうきょうスカイツリー駅で買う"
    print(f"text: {text}\n")

    # tokenize the text
    tokens = tokenizer.tokenize(text)

    for token in tokens:
        print(token.surface)


if __name__ == "__main__":
    main()
