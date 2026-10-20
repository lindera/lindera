"""Tests for Mode, the counterpart of the Rust lindera::mode::Mode (#1218)."""

import pytest

from lindera import Mode, Penalty


def _short_katakana_threshold():
    return Penalty(other_penalty_length_threshold=4, other_penalty_length_penalty=3000)


def _assert_default_penalty(penalty):
    assert penalty.kanji_penalty_length_threshold == 2
    assert penalty.kanji_penalty_length_penalty == 3000
    assert penalty.other_penalty_length_threshold == 7
    assert penalty.other_penalty_length_penalty == 1700


def test_normal():
    mode = Mode.normal()
    assert mode.name == "normal"
    assert str(mode) == "normal"
    assert repr(mode) == "Mode.normal()"
    assert mode.is_normal()
    assert not mode.is_decompose()
    assert mode.penalty is None


def test_decompose_with_the_default_penalty():
    for mode in [Mode.decompose(), Mode.decompose(None), Mode.decompose(penalty=None)]:
        assert mode.name == "decompose"
        assert str(mode) == "decompose"
        assert repr(mode).startswith("Mode.decompose(Penalty(")
        assert mode.is_decompose()
        assert not mode.is_normal()
        _assert_default_penalty(mode.penalty)


def test_decompose_keeps_the_penalty():
    penalty = Mode.decompose(_short_katakana_threshold()).penalty
    assert penalty.kanji_penalty_length_threshold == 2
    assert penalty.kanji_penalty_length_penalty == 3000
    assert penalty.other_penalty_length_threshold == 4
    assert penalty.other_penalty_length_penalty == 3000


def test_the_penalty_is_a_copy():
    mode = Mode.decompose()
    penalty = mode.penalty
    penalty.kanji_penalty_length_penalty = 0
    assert mode.penalty.kanji_penalty_length_penalty == 3000

    given = Penalty()
    mode = Mode.decompose(given)
    given.kanji_penalty_length_penalty = 0
    assert mode.penalty.kanji_penalty_length_penalty == 3000


def test_from_str():
    assert Mode.from_str("normal") == Mode.normal()
    assert Mode.from_str("decompose") == Mode.decompose()
    assert Mode.from_str(name="decompose") == Mode.decompose()


@pytest.mark.parametrize("name", ["Normal", "Decompose", "DECOMPOSE", "", "invalid"])
def test_from_str_rejects_other_names(name):
    # Case sensitive, as the Rust Mode parses; the ValueError that
    # set_mode("invalid") raised in v6.
    with pytest.raises(ValueError, match="Invalid mode"):
        Mode.from_str(name)


@pytest.mark.parametrize("value", [0, None, Penalty()])
def test_from_str_takes_a_string(value):
    with pytest.raises(TypeError):
        Mode.from_str(value)


def test_equality_compares_the_name_and_the_penalty():
    assert Mode.normal() == Mode.normal()
    assert Mode.decompose() == Mode.decompose(Penalty())
    assert Mode.decompose(_short_katakana_threshold()) == Mode.decompose(
        _short_katakana_threshold()
    )
    assert Mode.decompose() != Mode.decompose(_short_katakana_threshold())
    assert Mode.normal() != Mode.decompose()


@pytest.mark.parametrize("other", ["normal", 0, None, Penalty()])
def test_a_mode_is_not_equal_to_another_type(other):
    assert Mode.normal() != other
    assert (Mode.normal() == other) is False


def test_a_mode_is_not_hashable():
    # As the Rust Mode is Eq but not Hash.
    with pytest.raises(TypeError):
        hash(Mode.normal())


def test_a_mode_has_no_constructor():
    with pytest.raises(TypeError):
        Mode()
    with pytest.raises(TypeError):
        Mode("normal")


def test_a_mode_cannot_be_changed():
    mode = Mode.normal()
    with pytest.raises(AttributeError):
        mode.name = "decompose"
    with pytest.raises(AttributeError):
        mode.penalty = Penalty()


def test_the_enum_members_are_gone():
    assert not hasattr(Mode, "Normal")
    assert not hasattr(Mode, "Decompose")
