from lindera import Tokenizer, load_dictionary


def test_tokenize_with_ipadic():
    dictionary = load_dictionary("embedded://ipadic")
    tokenizer = Tokenizer(dictionary, mode="normal")

    text = "すもももももももものうち"
    print(text)

    # tokenize the text
    tokens = tokenizer.tokenize(text)

    assert tokens[0].surface == "すもも"
    assert tokens[1].surface == "も"
    assert tokens[2].surface == "もも"
    assert tokens[3].surface == "も"
    assert tokens[4].surface == "もも"
    assert tokens[5].surface == "の"
    assert tokens[6].surface == "うち"

    assert len(tokens) == 7
