"""Integer arguments reject a value outside the range of their setting (lindera/lindera#1209).

The binding declares these parameters with the core's own types (i16, u16,
i32, u64, usize), so pyo3 raises OverflowError for a value the setting cannot
hold instead of wrapping it, as the PHP, Node.js and WASM bindings used to.
These tests pin that, and that the error names the parameter.
"""

import sys

import pytest

import lindera

USIZE_MAX = 2**64 - 1

# A path that does not exist: pyo3 checks the arguments before train() runs.
MISSING = "/nonexistent/issue1209"


def _train(**kwargs):
    lindera.train(
        seed=MISSING,
        corpus=MISSING,
        char_def=MISSING,
        unk_def=MISSING,
        feature_def=MISSING,
        rewrite_def=MISSING,
        output=MISSING,
        **kwargs,
    )


def _set(obj, name, value):
    setattr(obj, name, value)


def _tokenizer():
    return lindera.Tokenizer(lindera.load_dictionary("embedded://ipadic"))


def _has_embedded_ipadic():
    try:
        lindera.load_dictionary("embedded://ipadic")
        return True
    except ValueError:
        return False


# (parameter name, call taking the value, values outside the range)
CASES = {
    "Metadata(default_word_cost)": (
        "default_word_cost",
        lambda v: lindera.Metadata(default_word_cost=v),
        [-32769, 32768, 100000],
    ),
    "Metadata(default_left_context_id)": (
        "default_left_context_id",
        lambda v: lindera.Metadata(default_left_context_id=v),
        [-1, 65536],
    ),
    "Metadata(default_right_context_id)": (
        "default_right_context_id",
        lambda v: lindera.Metadata(default_right_context_id=v),
        [-1, 65536],
    ),
    "metadata.default_word_cost =": (
        "default_word_cost",
        lambda v: _set(lindera.Metadata(), "default_word_cost", v),
        [-32769, 32768],
    ),
    "metadata.default_left_context_id =": (
        "default_left_context_id",
        lambda v: _set(lindera.Metadata(), "default_left_context_id", v),
        [-1, 65536],
    ),
    "metadata.default_right_context_id =": (
        "default_right_context_id",
        lambda v: _set(lindera.Metadata(), "default_right_context_id", v),
        [-1, 65536],
    ),
    "Penalty(kanji_penalty_length_threshold)": (
        "kanji_penalty_length_threshold",
        lambda v: lindera.Penalty(kanji_penalty_length_threshold=v),
        [-1, USIZE_MAX + 1],
    ),
    "Penalty(kanji_penalty_length_penalty)": (
        "kanji_penalty_length_penalty",
        lambda v: lindera.Penalty(kanji_penalty_length_penalty=v),
        [-(2**31) - 1, 2**31],
    ),
    "Penalty(other_penalty_length_threshold)": (
        "other_penalty_length_threshold",
        lambda v: lindera.Penalty(other_penalty_length_threshold=v),
        [-1, USIZE_MAX + 1],
    ),
    "Penalty(other_penalty_length_penalty)": (
        "other_penalty_length_penalty",
        lambda v: lindera.Penalty(other_penalty_length_penalty=v),
        [-(2**31) - 1, 2**31],
    ),
    "penalty.kanji_penalty_length_threshold =": (
        "kanji_penalty_length_threshold",
        lambda v: _set(lindera.Penalty(), "kanji_penalty_length_threshold", v),
        [-1],
    ),
    "penalty.kanji_penalty_length_penalty =": (
        "kanji_penalty_length_penalty",
        lambda v: _set(lindera.Penalty(), "kanji_penalty_length_penalty", v),
        [2**31],
    ),
    "penalty.other_penalty_length_threshold =": (
        "other_penalty_length_threshold",
        lambda v: _set(lindera.Penalty(), "other_penalty_length_threshold", v),
        [-1],
    ),
    "penalty.other_penalty_length_penalty =": (
        "other_penalty_length_penalty",
        lambda v: _set(lindera.Penalty(), "other_penalty_length_penalty", v),
        [-(2**31) - 1],
    ),
    "FieldDefinition(index)": (
        "index",
        lambda v: lindera.FieldDefinition(v, "x", lindera.FieldType.Custom, None),
        [-1],
    ),
    "Schema.get_field_name(index)": (
        "index",
        lambda v: lindera.Schema.create_default().get_field_name(v),
        [-1],
    ),
}

if _has_embedded_ipadic():
    CASES.update(
        {
            "Token.get_detail(index)": (
                "index",
                lambda v: _tokenizer().tokenize("東京")[0].get_detail(v),
                [-1],
            ),
            "Tokenizer.tokenize_nbest(n)": (
                "n",
                lambda v: _tokenizer().tokenize_nbest("東京", v),
                [-1],
            ),
        }
    )

if hasattr(lindera, "train"):
    CASES.update(
        {
            "train(max_iter)": ("max_iter", lambda v: _train(max_iter=v), [-1]),
            "train(max_threads)": (
                "max_threads",
                lambda v: _train(max_threads=v),
                [-1],
            ),
        }
    )


@pytest.mark.parametrize("label", sorted(CASES))
def test_a_value_outside_the_range_raises_overflow_error(label):
    name, call, values = CASES[label]
    for value in values:
        with pytest.raises(OverflowError) as excinfo:
            call(value)
        if sys.version_info >= (3, 11):
            # pyo3 names the parameter in a note; a setter's note names its
            # argument, which is named after the property.
            assert f"while processing '{name}'" in getattr(
                excinfo.value, "__notes__", []
            )


def test_an_accepted_value_is_kept_unchanged():
    metadata = lindera.Metadata(default_word_cost=-32768, default_left_context_id=65535)
    assert metadata.default_word_cost == -32768
    assert metadata.default_left_context_id == 65535
    penalty = lindera.Penalty(kanji_penalty_length_penalty=-(2**31))
    assert penalty.kanji_penalty_length_penalty == -(2**31)
