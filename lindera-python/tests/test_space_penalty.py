"""Tests for the left-space penalty setting (Korean, lindera/lindera#1052)."""

import json
from pathlib import Path

import pytest

from lindera import Metadata, TokenizerBuilder, load_dictionary

KO_DIC_METADATA = (
    Path(__file__).resolve().parents[2] / "lindera-ko-dic" / "metadata.json"
)
# The binding's own copy, used like resources/ipadic_metadata.json in the
# examples.
KO_DIC_METADATA_RESOURCE = (
    Path(__file__).resolve().parents[1] / "resources" / "ko-dic_metadata.json"
)

RULES = {"rules": [{"pos": ["JKS"], "cost": 6000}]}

# IPADIC ships no rules, but explicit rules apply to it too: penalizing 助詞
# right after a space makes "に" in "東京 に 行く" lose its particle reading.
PARTICLE_RULES = {"rules": [{"pos": ["助詞"], "cost": 100000}]}


@pytest.mark.parametrize("value", [None, True, False, RULES, {"rules": []}])
def test_set_space_penalty_accepts_the_config_forms(value):
    builder = TokenizerBuilder()
    # Like the other setters, it returns the builder for chaining.
    assert builder.set_space_penalty(value) is builder


@pytest.mark.parametrize(
    "value",
    [
        1,
        1.5,
        "false",
        ["JKS"],
        {"rules": "JKS"},
        {"rules": [{"pos": ["JKS"]}]},
        # Non-finite floats must not be mistaken for None (reset).
        float("nan"),
        float("inf"),
        # Integers beyond i64 raise ValueError, not OverflowError.
        2**70,
        {"rules": [{"pos": ["JKS"], "cost": 2**70}]},
    ],
)
def test_set_space_penalty_rejects_other_values(value):
    with pytest.raises(ValueError):
        TokenizerBuilder().set_space_penalty(value)


def test_set_space_penalty_rejects_non_json_objects():
    with pytest.raises(TypeError):
        TokenizerBuilder().set_space_penalty(object())


def test_space_penalty_true_fails_on_ipadic():
    # IPADIC ships no left-space penalty rules, so requiring them fails.
    builder = (
        TokenizerBuilder().set_dictionary("embedded://ipadic").set_space_penalty(True)
    )
    with pytest.raises(ValueError, match="space_penalty"):
        builder.build()


def _particle_pos(*values):
    """Returns the POS of "に" in "東京 に 行く" after these setter calls."""
    builder = TokenizerBuilder().set_dictionary("embedded://ipadic")
    for value in values:
        builder.set_space_penalty(value)
    tokens = builder.build().tokenize("東京 に 行く")
    return {token.surface: token.details[0] for token in tokens}["に"]


@pytest.mark.parametrize(
    ("values", "expected_particle"),
    [
        ((None,), True),
        ((False,), True),
        ((PARTICLE_RULES,), False),
        # The last call wins: False and None each drop earlier rules...
        ((PARTICLE_RULES, False), True),
        ((PARTICLE_RULES, None), True),
        # ...and rules replace an earlier False.
        ((False, PARTICLE_RULES), False),
    ],
)
def test_space_penalty_reaches_the_segmenter_on_ipadic(values, expected_particle):
    assert (_particle_pos(*values) == "助詞") is expected_particle


def test_space_penalty_none_undoes_true():
    # None restores the default, so a previous True no longer fails the build.
    builder = (
        TokenizerBuilder()
        .set_dictionary("embedded://ipadic")
        .set_space_penalty(True)
        .set_space_penalty(None)
    )
    builder.build()


@pytest.mark.parametrize("path", [KO_DIC_METADATA, KO_DIC_METADATA_RESOURCE])
def test_ko_dic_metadata_exposes_space_penalty(path):
    metadata = Metadata.from_json_file(str(path))
    space_penalty = metadata.to_dict()["space_penalty"]
    assert "JKS" in space_penalty
    rules = json.loads(space_penalty)["rules"]
    assert any("JKS" in rule["pos"] for rule in rules)


def test_metadata_without_rules_omits_space_penalty():
    assert "space_penalty" not in Metadata().to_dict()
    assert (
        "space_penalty" not in load_dictionary("embedded://ipadic").metadata().to_dict()
    )
