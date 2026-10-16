"""Tests for the penalty of decompose mode (lindera/lindera#1215)."""

import pytest

from lindera import Penalty, Tokenizer, TokenizerBuilder, load_dictionary

TEXT = "関西国際空港限定トートバッグ"

DEFAULT_SURFACES = ["関西", "国際", "空港", "限定", "トートバッグ"]

# Without length penalties, decompose mode keeps the compound whole, as
# normal mode does.
NO_LENGTH_PENALTY_SURFACES = ["関西国際空港", "限定", "トートバッグ"]


def _no_length_penalty():
    return Penalty(kanji_penalty_length_penalty=0, other_penalty_length_penalty=0)


def _builder_surfaces(*args, **kwargs):
    """Returns the surfaces of TEXT for a tokenizer built with set_mode(*args)."""
    builder = TokenizerBuilder().set_dictionary("embedded://ipadic")
    tokenizer = builder.set_mode(*args, **kwargs).build()
    return [token.surface for token in tokenizer.tokenize(TEXT)]


def _constructor_surfaces(*args, **kwargs):
    """Returns the surfaces of TEXT for a Tokenizer built directly."""
    tokenizer = Tokenizer(load_dictionary("embedded://ipadic"), *args, **kwargs)
    return [token.surface for token in tokenizer.tokenize(TEXT)]


@pytest.mark.parametrize(
    ("args", "kwargs", "expected"),
    [
        (("decompose",), {}, DEFAULT_SURFACES),
        (("decompose", None), {}, DEFAULT_SURFACES),
        (("decompose",), {"penalty": Penalty()}, DEFAULT_SURFACES),
        (("decompose",), {"penalty": _no_length_penalty()}, NO_LENGTH_PENALTY_SURFACES),
        (("decompose", _no_length_penalty()), {}, NO_LENGTH_PENALTY_SURFACES),
    ],
)
def test_set_mode_penalty_reaches_the_segmenter(args, kwargs, expected):
    assert _builder_surfaces(*args, **kwargs) == expected


def test_set_mode_returns_the_builder():
    builder = TokenizerBuilder()
    assert builder.set_mode("decompose", _no_length_penalty()) is builder


@pytest.mark.parametrize(
    ("kwargs", "expected"),
    [
        ({"mode": "decompose"}, DEFAULT_SURFACES),
        ({"mode": "decompose", "penalty": None}, DEFAULT_SURFACES),
        (
            {"mode": "decompose", "penalty": _no_length_penalty()},
            NO_LENGTH_PENALTY_SURFACES,
        ),
    ],
)
def test_constructor_penalty_reaches_the_segmenter(kwargs, expected):
    assert _constructor_surfaces(**kwargs) == expected


def test_constructor_takes_penalty_positionally():
    assert (
        _constructor_surfaces("decompose", None, None, _no_length_penalty())
        == NO_LENGTH_PENALTY_SURFACES
    )


def test_set_mode_rejects_a_penalty_with_normal_mode():
    with pytest.raises(ValueError, match="decompose"):
        TokenizerBuilder().set_mode("normal", Penalty())


def test_constructor_rejects_a_penalty_with_normal_mode():
    with pytest.raises(ValueError, match="decompose"):
        Tokenizer(load_dictionary("embedded://ipadic"), penalty=Penalty())


@pytest.mark.parametrize("value", [0, "penalty", {"kanji_penalty_length_penalty": 0}])
def test_penalty_of_another_type_is_rejected(value):
    with pytest.raises(TypeError):
        TokenizerBuilder().set_mode("decompose", value)
    with pytest.raises(TypeError):
        Tokenizer(load_dictionary("embedded://ipadic"), "decompose", penalty=value)
