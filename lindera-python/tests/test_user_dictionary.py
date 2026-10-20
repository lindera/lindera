import pytest

from lindera import (
    Mode,
    Tokenizer,
    build_user_dictionary,
    build_user_dictionary_for,
    load_dictionary,
    load_user_dictionary_for,
)

# Line 1 lies inside IPADIC's 1316 x 1316 connection matrix; line 2 has the
# context IDs 60000, far outside it.
OUT_OF_RANGE_CSV = (
    "東京タワー,1288,1288,-1000,名詞,固有名詞,一般,*,*,*,"
    "東京タワー,トウキョウタワー,トウキョウタワー\n"
    "東京スカイツリー,60000,60000,-1000,名詞,固有名詞,一般,*,*,*,"
    "東京スカイツリー,トウキョウスカイツリー,トウキョウスカイツリー\n"
)

SIMPLE_CSV = "東京スカイツリー,カスタム名詞,トウキョウスカイツリー\n"


def test_load_user_dictionary_for_names_the_csv_line_and_surface(tmp_path):
    dictionary = load_dictionary("embedded://ipadic")
    csv = tmp_path / "userdic.csv"
    csv.write_text(OUT_OF_RANGE_CSV, encoding="utf-8")

    with pytest.raises(ValueError) as excinfo:
        load_user_dictionary_for(str(csv), dictionary)

    message = str(excinfo.value)
    assert "line 2 (surface: '東京スカイツリー')" in message
    assert "left context id 60000 is out of range" in message


def test_load_user_dictionary_for_loads_a_csv_inside_the_matrix(tmp_path):
    dictionary = load_dictionary("embedded://ipadic")
    baseline = [
        token.surface
        for token in Tokenizer(dictionary, mode=Mode.normal()).tokenize(
            "東京スカイツリー"
        )
    ]
    assert len(baseline) > 1, f"fixture assumption: {baseline}"

    csv = tmp_path / "userdic.csv"
    csv.write_text(SIMPLE_CSV, encoding="utf-8")
    user_dictionary = load_user_dictionary_for(str(csv), dictionary)

    tokenizer = Tokenizer(
        dictionary, mode=Mode.normal(), user_dictionary=user_dictionary
    )
    surfaces = [token.surface for token in tokenizer.tokenize("東京スカイツリー")]
    assert surfaces == ["東京スカイツリー"]


def test_build_user_dictionary_for_names_the_csv_line_and_surface(tmp_path):
    dictionary = load_dictionary("embedded://ipadic")
    csv = tmp_path / "userdic.csv"
    csv.write_text(OUT_OF_RANGE_CSV, encoding="utf-8")
    output = tmp_path / "userdic.bin"

    with pytest.raises(ValueError) as excinfo:
        build_user_dictionary_for(str(csv), str(output), dictionary)

    message = str(excinfo.value)
    assert "line 2 (surface: '東京スカイツリー')" in message
    assert "left context id 60000 is out of range" in message
    assert not output.exists()


def test_build_user_dictionary_for_writes_the_output_file(tmp_path):
    dictionary = load_dictionary("embedded://ipadic")
    csv = tmp_path / "userdic.csv"
    csv.write_text(SIMPLE_CSV, encoding="utf-8")
    output = tmp_path / "out" / "userdic.bin"

    build_user_dictionary_for(str(csv), str(output), dictionary)

    assert output.is_file()
    user_dictionary = load_user_dictionary_for(str(output), dictionary)
    tokenizer = Tokenizer(
        dictionary, mode=Mode.normal(), user_dictionary=user_dictionary
    )
    surfaces = [token.surface for token in tokenizer.tokenize("東京スカイツリー")]
    assert surfaces == ["東京スカイツリー"]


def test_build_user_dictionary_writes_the_file_it_is_given(tmp_path):
    # `output_dir` is the path of the file to write, not a directory.
    dictionary = load_dictionary("embedded://ipadic")
    csv = tmp_path / "userdic.csv"
    csv.write_text(SIMPLE_CSV, encoding="utf-8")
    output = tmp_path / "userdic.bin"

    build_user_dictionary("ipadic", str(csv), str(output))

    assert output.is_file()
    load_user_dictionary_for(str(output), dictionary)
