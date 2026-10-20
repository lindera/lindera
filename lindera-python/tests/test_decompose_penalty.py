"""Tests for the penalty of decompose mode (lindera/lindera#1215, #1218)."""

import pytest

from lindera import Mode, Penalty, Tokenizer, TokenizerBuilder, load_dictionary

TEXT = "関西国際空港限定トートバッグ"

NORMAL_SURFACES = ["関西国際空港", "限定", "トートバッグ"]

DEFAULT_SURFACES = ["関西", "国際", "空港", "限定", "トートバッグ"]

# Without length penalties, decompose mode keeps the compound whole, as
# normal mode does.
NO_LENGTH_PENALTY_SURFACES = NORMAL_SURFACES

# With a katakana threshold of 4, the output differs from both normal mode's
# and the default penalty's, so a decompose mode that loses its penalty, or
# is read as normal mode, fails.
SHORT_KATAKANA_SURFACES = ["関西", "国際", "空港", "限定", "トー", "トバッグ"]


def _no_length_penalty():
    return Penalty(kanji_penalty_length_penalty=0, other_penalty_length_penalty=0)


def _short_katakana_threshold():
    return Penalty(other_penalty_length_threshold=4, other_penalty_length_penalty=3000)


MODES = [
    pytest.param(Mode.normal, NORMAL_SURFACES, id="normal"),
    pytest.param(Mode.decompose, DEFAULT_SURFACES, id="decompose"),
    pytest.param(lambda: Mode.decompose(None), DEFAULT_SURFACES, id="decompose-none"),
    pytest.param(
        lambda: Mode.decompose(Penalty()), DEFAULT_SURFACES, id="decompose-default"
    ),
    pytest.param(
        lambda: Mode.decompose(_no_length_penalty()),
        NO_LENGTH_PENALTY_SURFACES,
        id="decompose-no-length-penalty",
    ),
    pytest.param(
        lambda: Mode.decompose(penalty=_short_katakana_threshold()),
        SHORT_KATAKANA_SURFACES,
        id="decompose-short-katakana-threshold",
    ),
    pytest.param(
        lambda: Mode.from_str("decompose"), DEFAULT_SURFACES, id="from-str-decompose"
    ),
]


def _builder_surfaces(mode):
    """Returns the surfaces of TEXT for a tokenizer built with set_mode(mode)."""
    builder = TokenizerBuilder().set_dictionary("embedded://ipadic")
    tokenizer = builder.set_mode(mode).build()
    return [token.surface for token in tokenizer.tokenize(TEXT)]


def _constructor_surfaces(*args, **kwargs):
    """Returns the surfaces of TEXT for a Tokenizer built directly."""
    tokenizer = Tokenizer(load_dictionary("embedded://ipadic"), *args, **kwargs)
    return [token.surface for token in tokenizer.tokenize(TEXT)]


@pytest.mark.parametrize(("make_mode", "expected"), MODES)
def test_set_mode_penalty_reaches_the_segmenter(make_mode, expected):
    assert _builder_surfaces(make_mode()) == expected


@pytest.mark.parametrize(("make_mode", "expected"), MODES)
def test_constructor_penalty_reaches_the_segmenter(make_mode, expected):
    assert _constructor_surfaces(make_mode()) == expected
    assert _constructor_surfaces(mode=make_mode()) == expected


def test_one_mode_serves_several_tokenizers():
    mode = Mode.decompose(_short_katakana_threshold())
    assert _builder_surfaces(mode) == SHORT_KATAKANA_SURFACES
    assert _constructor_surfaces(mode) == SHORT_KATAKANA_SURFACES
    assert _builder_surfaces(mode) == SHORT_KATAKANA_SURFACES


def test_set_mode_returns_the_builder():
    builder = TokenizerBuilder()
    assert builder.set_mode(Mode.decompose()) is builder


@pytest.mark.parametrize(
    ("args", "kwargs"), [((), {}), ((None,), {}), ((), {"mode": None})]
)
def test_constructor_defaults_to_normal_mode(args, kwargs):
    assert _constructor_surfaces(*args, **kwargs) == NORMAL_SURFACES


@pytest.mark.parametrize(
    "value", ["decompose", "normal", 0, None, Penalty(), {"name": "decompose"}]
)
def test_set_mode_rejects_a_value_that_is_not_a_mode(value):
    with pytest.raises(TypeError):
        TokenizerBuilder().set_mode(value)


@pytest.mark.parametrize(
    "value", ["decompose", "normal", 0, Penalty(), {"name": "decompose"}]
)
def test_constructor_rejects_a_value_that_is_not_a_mode(value):
    with pytest.raises(TypeError):
        Tokenizer(load_dictionary("embedded://ipadic"), value)


def test_the_penalty_argument_is_gone():
    # The penalty goes in the Mode (#1218).
    with pytest.raises(TypeError):
        TokenizerBuilder().set_mode(Mode.decompose(), Penalty())
    with pytest.raises(TypeError):
        Tokenizer(load_dictionary("embedded://ipadic"), penalty=Penalty())
