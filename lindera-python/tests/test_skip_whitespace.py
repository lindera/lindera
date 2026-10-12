"""Tests for the whitespace-skipping setting (lindera/lindera#1137)."""

import pytest

from lindera import TokenizerBuilder

# With whitespace skipped (the default for IPADIC), "が" connects to the word
# before the space and reads as a particle (助詞), as in MeCab; with the
# whitespace node in the lattice it reads as a conjunction (接続詞).
TEXT = "Google が 新しい"


def _pos_of_ga(builder):
    """Returns the POS of "が" in TEXT for a tokenizer built from `builder`."""
    tokens = builder.build().tokenize(TEXT)
    return {token.surface: token.details[0] for token in tokens}["が"]


def _ipadic_builder(*values):
    """An IPADIC builder after calling set_skip_whitespace with each value."""
    builder = TokenizerBuilder().set_dictionary("embedded://ipadic")
    for value in values:
        builder.set_skip_whitespace(value)
    return builder


@pytest.mark.parametrize("value", [True, False, None])
def test_set_skip_whitespace_chains(value):
    builder = TokenizerBuilder()
    assert builder.set_skip_whitespace(value) is builder


@pytest.mark.parametrize(
    ("values", "expected"),
    [
        ((), "助詞"),
        ((True,), "助詞"),
        ((False,), "接続詞"),
        # None goes back to the dictionary's default, which skips for IPADIC.
        ((False, None), "助詞"),
        # The last call wins.
        ((True, False), "接続詞"),
    ],
)
def test_set_skip_whitespace_reaches_the_segmenter(values, expected):
    assert _pos_of_ga(_ipadic_builder(*values)) == expected


@pytest.mark.parametrize("value", ["false", 0, 1])
def test_set_skip_whitespace_rejects_non_booleans(value):
    with pytest.raises(TypeError):
        TokenizerBuilder().set_skip_whitespace(value)


def test_set_skip_whitespace_overrides_the_configuration_file(tmp_path):
    config = tmp_path / "lindera.yml"
    config.write_text(
        'segmenter:\n  dictionary: "embedded://ipadic"\n  skip_whitespace: false\n',
        encoding="utf-8",
    )
    assert _pos_of_ga(TokenizerBuilder().from_file(str(config))) == "接続詞"

    builder = TokenizerBuilder().from_file(str(config)).set_skip_whitespace(True)
    assert _pos_of_ga(builder) == "助詞"

    # None removes the file's setting too, so the dictionary's default applies.
    builder = TokenizerBuilder().from_file(str(config)).set_skip_whitespace(None)
    assert _pos_of_ga(builder) == "助詞"


def test_set_skip_whitespace_has_no_effect_with_kept_whitespace():
    kept = _ipadic_builder().set_keep_whitespace(True)
    surfaces = [token.surface for token in kept.build().tokenize(TEXT)]
    assert " " in surfaces

    kept_and_skipped = _ipadic_builder(True).set_keep_whitespace(True)
    assert [
        token.surface for token in kept_and_skipped.build().tokenize(TEXT)
    ] == surfaces
