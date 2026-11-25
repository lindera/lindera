"""Tests for the token filter japanese_dependency (lindera/lindera#1438).

The model is the one of the tests of lindera-analysis: a chunker over the
part of speech and a parser that makes every bunsetsu depend on the next.
"""

import json
from pathlib import Path

import pytest

from lindera import Token, TokenDependency, TokenizerBuilder

MODEL = (
    Path(__file__).resolve().parents[2]
    / "resources"
    / "dependency"
    / "test_model_ipadic.bin"
)
TEXT = "彼は本を読んだ。"


def _builder():
    """A builder with the embedded IPADIC."""
    return TokenizerBuilder().set_dictionary("embedded://ipadic")


def _tokenizer():
    """A tokenizer with the filter."""
    return (
        _builder()
        .append_token_filter("japanese_dependency", {"model": str(MODEL)})
        .build()
    )


def test_a_token_has_its_dependency():
    tokens = _tokenizer().tokenize(TEXT)
    written = [
        (
            token.surface,
            token.dependency.sentence,
            token.dependency.bunsetsu,
            token.dependency.head,
            token.dependency.role,
            token.dependency.score,
        )
        for token in tokens
    ]
    assert written == [
        ("彼", 0, 0, 1, "head", 1.0),
        ("は", 0, 0, 1, "function", 1.0),
        ("本", 0, 1, 2, "head", 0.0),
        ("を", 0, 1, 2, "function", 0.0),
        ("読ん", 0, 2, None, "head", 0.0),
        ("だ", 0, 2, None, "function", 0.0),
        ("。", 0, 2, None, "other", 0.0),
    ]
    assert isinstance(tokens[0], Token)
    assert isinstance(tokens[0].dependency, TokenDependency)
    assert isinstance(tokens[0].dependency.score, float)


def test_a_token_has_no_dependency_without_the_filter():
    tokens = _builder().build().tokenize(TEXT)
    assert [token.dependency for token in tokens] == [None] * 7
    assert tokens[0].to_dict()["dependency"] is None


def test_to_dict_holds_the_dependency():
    tokens = _tokenizer().tokenize(TEXT)
    first = {"sentence": 0, "bunsetsu": 0, "head": 1, "role": "head", "score": 1.0}
    last = {"sentence": 0, "bunsetsu": 2, "head": None, "role": "other", "score": 0.0}
    assert tokens[0].dependency.to_dict() == first
    assert tokens[0].to_dict()["dependency"] == first
    assert tokens[6].to_dict()["dependency"] == last
    # The dict of a token is plain data.
    assert json.loads(json.dumps(tokens[6].to_dict()))["dependency"] == last
    assert repr(tokens[0].dependency) == (
        "<TokenDependency sentence=0, bunsetsu=0, head=1, role='head', score=1>"
    )
    assert "head=None" in repr(tokens[6].dependency)


def test_a_dependency_cannot_be_changed():
    dependency = _tokenizer().tokenize(TEXT)[0].dependency
    with pytest.raises(AttributeError):
        dependency.head = 5


def test_the_batch_and_the_nbest_results_carry_it():
    tokenizer = _tokenizer()
    expected = [token.to_dict() for token in tokenizer.tokenize(TEXT)]
    batch = tokenizer.tokenize_batch([TEXT, TEXT])
    assert [token.to_dict() for token in batch[1]] == expected
    tokens, _cost = tokenizer.tokenize_nbest(TEXT, 1)[0]
    assert [token.to_dict() for token in tokens] == expected


def test_sentences_are_numbered():
    tokens = _tokenizer().tokenize("彼は本を読んだ。本を読んだ。")
    assert [token.dependency.sentence for token in tokens] == [0] * 7 + [1] * 5
    # The bunsetsu of the second sentence start at 0 again.
    assert tokens[7].surface == "本"
    assert tokens[7].dependency.bunsetsu == 0


def test_a_model_that_cannot_be_read_is_an_error_of_build():
    builder = _builder().append_token_filter(
        "japanese_dependency", {"model": "no/such/model.bin"}
    )
    with pytest.raises(Exception, match="no/such/model.bin"):
        builder.build()


def test_tokens_removed_by_an_earlier_filter_are_refused():
    tokenizer = (
        _builder()
        .append_token_filter("japanese_stop_tags", {"tags": ["助詞,係助詞"]})
        .append_token_filter("japanese_dependency", {"model": str(MODEL)})
        .build()
    )
    with pytest.raises(Exception, match="removed or joined"):
        tokenizer.tokenize(TEXT)
