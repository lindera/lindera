"""The batch methods of Tokenizer (#1402): every text is tokenized as a call
of its own, on several threads, and the results come in the order of the
texts, the same as the single calls'."""

from pathlib import Path

import pytest

from lindera import Mode, Tokenizer, load_dictionary

BOCCHAN = Path(__file__).resolve().parents[2] / "resources" / "bocchan.txt"


def _tokenizer():
    return Tokenizer(load_dictionary("embedded://ipadic"), mode=Mode.normal())


def _fields(tokens):
    return [
        (
            t.surface,
            t.byte_start,
            t.byte_end,
            t.position,
            t.word_id,
            t.is_unknown,
            t.details,
        )
        for t in tokens
    ]


def _texts():
    # Enough input for a batch on several threads, and an empty text.
    return BOCCHAN.read_text(encoding="utf-8").splitlines() + [""]


@pytest.mark.parametrize("max_threads", [None, 1, 2, 8])
def test_tokenize_batch_matches_tokenize(max_threads):
    tokenizer = _tokenizer()
    texts = _texts()
    expected = [_fields(tokenizer.tokenize(text)) for text in texts]
    batch = tokenizer.tokenize_batch(texts, max_threads=max_threads)
    assert [_fields(tokens) for tokens in batch] == expected


@pytest.mark.parametrize("max_threads", [None, 1, 4])
def test_tokenize_surfaces_batch_matches_tokenize_surfaces(max_threads):
    tokenizer = _tokenizer()
    texts = _texts()
    expected = [tokenizer.tokenize_surfaces(text) for text in texts]
    assert tokenizer.tokenize_surfaces_batch(texts, max_threads) == expected


@pytest.mark.parametrize("max_threads", [None, 1, 4])
def test_tokenize_nbest_batch_matches_tokenize_nbest(max_threads):
    tokenizer = _tokenizer()
    texts = _texts()[:60]
    expected = [
        [
            (_fields(tokens), cost)
            for tokens, cost in tokenizer.tokenize_nbest(text, 3, True, 5000)
        ]
        for text in texts
    ]
    batch = tokenizer.tokenize_nbest_batch(
        texts, 3, unique=True, cost_threshold=5000, max_threads=max_threads
    )
    assert [
        [(_fields(tokens), cost) for tokens, cost in results] for results in batch
    ] == expected


def test_tokenize_batch_takes_an_empty_list():
    assert _tokenizer().tokenize_batch([]) == []


def test_tokenize_batch_rejects_zero_threads():
    tokenizer = _tokenizer()
    for method in (
        lambda: tokenizer.tokenize_batch(["東京"], max_threads=0),
        lambda: tokenizer.tokenize_surfaces_batch(["東京"], max_threads=0),
        lambda: tokenizer.tokenize_nbest_batch(["東京"], 2, max_threads=0),
    ):
        with pytest.raises(
            ValueError,
            match=r"^max_threads must be an integer between 1 and \d+, got 0$",
        ):
            method()


def test_tokenize_batch_rejects_negative_threads():
    with pytest.raises(OverflowError):
        _tokenizer().tokenize_batch(["東京"], max_threads=-1)


def test_tokenize_batch_rejects_a_single_string():
    # A str is not taken as a list of one-character texts.
    with pytest.raises(TypeError):
        _tokenizer().tokenize_batch("東京")
