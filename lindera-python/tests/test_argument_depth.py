"""Tests for the nesting limit on converted arguments (lindera/lindera#1062)."""

import subprocess
import sys
import textwrap

import pytest

from lindera import TokenizerBuilder

TOO_DEEP = "nested more than 128 levels deep"

# The three builder methods that convert a Python value to JSON.
CALLS = {
    "set_space_penalty": lambda builder, value: builder.set_space_penalty(value),
    "append_character_filter": lambda builder, value: builder.append_character_filter(
        "unicode_normalize", value
    ),
    "append_token_filter": lambda builder, value: builder.append_token_filter(
        "lowercase", value
    ),
}


def nested_dicts(levels):
    """Returns {"a": {"a": ...}} made of `levels` nested dicts."""
    value = {}
    for _ in range(levels - 1):
        value = {"a": value}
    return value


def nested_lists(levels):
    """Returns {"a": [[...]]}: a dict holding `levels - 1` nested lists.

    The filter methods take a dict, so lists are wrapped in one.
    """
    value = []
    for _ in range(levels - 2):
        value = [value]
    return {"a": value}


@pytest.mark.parametrize("method", CALLS)
@pytest.mark.parametrize("make", [nested_dicts, nested_lists])
def test_accepts_128_levels(method, make):
    try:
        CALLS[method](TokenizerBuilder(), make(128))
    except ValueError as error:
        # set_space_penalty rejects the shape, but not for being too deep.
        assert TOO_DEEP not in str(error)


@pytest.mark.parametrize("method", CALLS)
@pytest.mark.parametrize("make", [nested_dicts, nested_lists])
def test_rejects_129_levels(method, make):
    with pytest.raises(ValueError, match=TOO_DEEP):
        CALLS[method](TokenizerBuilder(), make(129))


@pytest.mark.parametrize("method", CALLS)
def test_builder_stays_usable_after_rejecting_an_argument(method):
    builder = TokenizerBuilder()
    with pytest.raises(ValueError, match=TOO_DEEP):
        CALLS[method](builder, nested_dicts(129))
    CALLS[method](builder, None if method == "set_space_penalty" else {})


# Without the limit these overflowed the native stack and killed the
# interpreter, so they run in a separate process.
ISOLATED_VALUES = {
    "a value that contains itself": "value = {'rules': []}; value['self'] = value",
    "10,000 levels": textwrap.dedent(
        """
        value = {}
        for _ in range(9999):
            value = {'a': value}
        """
    ),
}


@pytest.mark.parametrize("method", CALLS)
@pytest.mark.parametrize("name", ISOLATED_VALUES)
def test_rejects_without_crashing_the_interpreter(method, name):
    call = {
        "set_space_penalty": "builder.set_space_penalty(value)",
        "append_character_filter": "builder.append_character_filter('unicode_normalize', value)",
        "append_token_filter": "builder.append_token_filter('lowercase', value)",
    }[method]
    script = "\n".join(
        [
            "from lindera import TokenizerBuilder",
            ISOLATED_VALUES[name],
            "builder = TokenizerBuilder()",
            "try:",
            f"    {call}",
            "    print('returned')",
            "except ValueError as error:",
            "    print('raised:', error)",
        ]
    )
    result = subprocess.run(
        [sys.executable, "-c", script],
        capture_output=True,
        text=True,
        timeout=60,
    )
    assert result.returncode == 0, result.stderr
    assert result.stdout.startswith("raised:"), result.stdout
    assert TOO_DEEP in result.stdout
