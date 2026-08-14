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


def test_tokenize_surfaces_matches_tokenize():
    dictionary = load_dictionary("embedded://ipadic")
    tokenizer = Tokenizer(dictionary, mode="normal")

    for text in ["すもももももももものうち", "関西国際空港限定トートバッグ", ""]:
        expected = [token.surface for token in tokenizer.tokenize(text)]
        surfaces = tokenizer.tokenize_surfaces(text)
        assert surfaces == expected
        assert all(isinstance(surface, str) for surface in surfaces)


def test_tokenize_repeated_calls_are_stable():
    # The tokenizer reuses an internal lattice across calls; repeated calls
    # on one instance must keep producing identical output.
    dictionary = load_dictionary("embedded://ipadic")
    tokenizer = Tokenizer(dictionary, mode="normal")

    text = "すもももももももものうち"
    first_tokens = [(t.surface, t.byte_start, t.byte_end) for t in tokenizer.tokenize(text)]
    first_surfaces = tokenizer.tokenize_surfaces(text)
    for _ in range(100):
        assert [(t.surface, t.byte_start, t.byte_end) for t in tokenizer.tokenize(text)] == first_tokens
        assert tokenizer.tokenize_surfaces(text) == first_surfaces
