"""Tests for the break-characters setting (lindera/lindera#1353)."""

import pytest

from lindera import TokenizerBuilder

# "レノン・ジョン" is not in IPADIC. With "・" as a break character (the
# default), the unknown word stops at it; with none, the katakana run,
# "・" included, is one unknown word, as in MeCab.
TEXT = "レノン・ジョン"
SPLIT = ["レノン", "・", "ジョン"]
WHOLE = ["レノン・ジョン"]


def _surfaces(builder):
    """Returns the surfaces of TEXT for a tokenizer built from `builder`."""
    return [token.surface for token in builder.build().tokenize(TEXT)]


def _ipadic_builder(*values):
    """An IPADIC builder after calling set_break_characters with each value."""
    builder = TokenizerBuilder().set_dictionary("embedded://ipadic")
    for value in values:
        builder.set_break_characters(value)
    return builder


@pytest.mark.parametrize("value", ["・", "", None])
def test_set_break_characters_chains(value):
    builder = TokenizerBuilder()
    assert builder.set_break_characters(value) is builder


@pytest.mark.parametrize(
    ("values", "expected"),
    [
        ((), SPLIT),
        (("・",), SPLIT),
        (("",), WHOLE),
        # Another set replaces the default.
        (("ー",), WHOLE),
        # None goes back to the default, "・".
        (("", None), SPLIT),
        # The last call wins.
        (("・", ""), WHOLE),
    ],
)
def test_set_break_characters_reaches_the_segmenter(values, expected):
    assert _surfaces(_ipadic_builder(*values)) == expected


@pytest.mark.parametrize("value", [0, True, ["・"]])
def test_set_break_characters_rejects_non_strings(value):
    with pytest.raises(TypeError):
        TokenizerBuilder().set_break_characters(value)


def test_set_break_characters_overrides_the_configuration_file(tmp_path):
    config = tmp_path / "lindera.yml"
    config.write_text(
        'segmenter:\n  dictionary: "embedded://ipadic"\n  break_characters: ""\n',
        encoding="utf-8",
    )
    builder = TokenizerBuilder().from_file(str(config))
    assert _surfaces(builder) == WHOLE
    builder.set_break_characters("・")
    assert _surfaces(builder) == SPLIT
