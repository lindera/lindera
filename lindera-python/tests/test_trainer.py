import tempfile
from pathlib import Path

import pytest

# Skip all tests in this file if train feature is not available
pytestmark = pytest.mark.skipif(
    not hasattr(__import__("lindera"), "train"), reason="train feature not available"
)

# The files `lindera export` writes besides metadata.json, which
# `lindera.export` writes too (#1169).
EXPORTED_FILES = [
    "lex.csv",
    "matrix.def",
    "unk.def",
    "char.def",
    "feature.def",
    "rewrite.def",
    "left-id.def",
    "right-id.def",
]


def test_train_basic():
    """Test basic training functionality"""
    import lindera

    # Create temporary directory for test files
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir = Path(tmpdir)

        # Create minimal seed lexicon (based on lindera/resources/training format)
        seed_file = tmpdir / "seed.csv"
        seed_file.write_text(
            "これ,0,0,0,名詞,代名詞,一般,*,*,*,これ,コレ,コレ\n"
            "は,0,0,0,助詞,係助詞,*,*,*,*,は,ハ,ワ\n"
            "テスト,0,0,0,名詞,サ変接続,*,*,*,*,テスト,テスト,テスト\n"
        )

        # Create character definition
        char_def_file = tmpdir / "char.def"
        char_def_file.write_text(
            "DEFAULT 0 1 0\n"
            "HIRAGANA 1 1 0\n"
            "KATAKANA 1 1 0\n"
            "KANJI 0 0 2\n"
            "ALPHA 1 1 0\n"
            "NUMERIC 1 1 0\n"
            "\n"
            "0x3041..0x3096 HIRAGANA\n"
            "0x30A1..0x30F6 KATAKANA\n"
            "0x4E00..0x9FAF KANJI\n"
            "0x0030..0x0039 NUMERIC\n"
            "0x0041..0x005A ALPHA\n"
            "0x0061..0x007A ALPHA\n"
        )

        # Create unknown word definition
        unk_def_file = tmpdir / "unk.def"
        unk_def_file.write_text(
            "DEFAULT,0,0,0,名詞,一般,*,*,*,*,*,*,*\n"
            "HIRAGANA,0,0,0,名詞,一般,*,*,*,*,*,*,*\n"
            "KATAKANA,0,0,0,名詞,一般,*,*,*,*,*,*,*\n"
            "KANJI,0,0,0,名詞,一般,*,*,*,*,*,*,*\n"
            "ALPHA,0,0,0,名詞,固有名詞,一般,*,*,*,*,*,*\n"
            "NUMERIC,0,0,0,名詞,数,*,*,*,*,*,*,*\n"
        )

        # Create feature definition
        feature_def_file = tmpdir / "feature.def"
        feature_def_file.write_text(
            "UNIGRAM U00:%F[0]\nUNIGRAM U01:%F[0],%F?[1]\nBIGRAM B00:%L[0]/%R[0]\n"
        )

        # Create rewrite definition
        rewrite_def_file = tmpdir / "rewrite.def"
        rewrite_def_file.write_text("名詞,一般\tNOUN,GENERAL\n")

        # Create minimal training corpus (tab-separated format with EOS markers)
        corpus_file = tmpdir / "corpus.txt"
        corpus_file.write_text(
            "これ\t名詞,代名詞,一般,*,*,*,これ,コレ,コレ\n"
            "は\t助詞,係助詞,*,*,*,*,は,ハ,ワ\n"
            "テスト\t名詞,サ変接続,*,*,*,*,テスト,テスト,テスト\n"
            "EOS\n"
        )

        # Output model file
        model_file = tmpdir / "model.dat"

        # Test training
        lindera.train(
            seed=str(seed_file),
            corpus=str(corpus_file),
            char_def=str(char_def_file),
            unk_def=str(unk_def_file),
            feature_def=str(feature_def_file),
            rewrite_def=str(rewrite_def_file),
            output=str(model_file),
            lambda_=0.01,
            max_iter=5,  # Use small number for testing
            max_threads=1,
        )

        # Verify model file was created
        assert model_file.exists()
        assert model_file.stat().st_size > 0


def test_export_basic():
    """Test basic export functionality"""
    import lindera

    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir = Path(tmpdir)

        # First, create and train a model (reuse code from test_train_basic)
        seed_file = tmpdir / "seed.csv"
        seed_file.write_text(
            "これ,0,0,0,名詞,代名詞,一般,*,*,*,これ,コレ,コレ\n"
            "は,0,0,0,助詞,係助詞,*,*,*,*,は,ハ,ワ\n"
        )

        char_def_file = tmpdir / "char.def"
        char_def_file.write_text(
            "DEFAULT 0 1 0\nHIRAGANA 1 1 0\n\n0x3041..0x3096 HIRAGANA\n"
        )

        unk_def_file = tmpdir / "unk.def"
        unk_def_file.write_text(
            "DEFAULT,0,0,0,名詞,一般,*,*,*,*,*,*,*\n"
            "HIRAGANA,0,0,0,名詞,一般,*,*,*,*,*,*,*\n"
        )

        feature_def_file = tmpdir / "feature.def"
        feature_def_file.write_text("UNIGRAM U00:%F[0]\nBIGRAM B00:%L[0]/%R[0]\n")

        rewrite_def_file = tmpdir / "rewrite.def"
        rewrite_def_file.write_text("名詞,一般\tNOUN\n")

        corpus_file = tmpdir / "corpus.txt"
        corpus_file.write_text(
            "これ\t名詞,代名詞,一般,*,*,*,これ,コレ,コレ\n"
            "は\t助詞,係助詞,*,*,*,*,は,ハ,ワ\n"
            "EOS\n"
        )

        model_file = tmpdir / "model.dat"

        # Train model
        lindera.train(
            seed=str(seed_file),
            corpus=str(corpus_file),
            char_def=str(char_def_file),
            unk_def=str(unk_def_file),
            feature_def=str(feature_def_file),
            rewrite_def=str(rewrite_def_file),
            output=str(model_file),
            lambda_=0.01,
            max_iter=5,
            max_threads=1,
        )

        # Export dictionary
        export_dir = tmpdir / "exported"
        lindera.export(model=str(model_file), output=str(export_dir))

        # Verify exported files: those of `lindera export`, none empty
        assert sorted(p.name for p in export_dir.iterdir()) == sorted(EXPORTED_FILES)
        for name in EXPORTED_FILES:
            assert (export_dir / name).stat().st_size > 0, name

        # The definition files are those the model was trained with
        for source in [char_def_file, feature_def_file, rewrite_def_file]:
            exported = export_dir / source.name
            assert exported.read_bytes() == source.read_bytes(), source.name


def test_export_with_metadata():
    """Test export with metadata update"""
    import lindera

    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir = Path(tmpdir)

        # Create minimal training setup
        seed_file = tmpdir / "seed.csv"
        seed_file.write_text("テスト,0,0,0,名詞,一般,*,*,*,*,テスト,テスト,テスト\n")

        char_def_file = tmpdir / "char.def"
        char_def_file.write_text(
            "DEFAULT 0 1 0\nKATAKANA 1 1 0\n\n0x30A1..0x30F6 KATAKANA\n"
        )

        # One line for each char.def category, as MeCab requires (#1195).
        unk_def_file = tmpdir / "unk.def"
        unk_def_file.write_text(
            "DEFAULT,0,0,0,名詞,一般,*,*,*,*,*,*,*\n"
            "KATAKANA,0,0,0,名詞,一般,*,*,*,*,*,*,*\n"
        )

        feature_def_file = tmpdir / "feature.def"
        feature_def_file.write_text("UNIGRAM U00:%F[0]\n")

        rewrite_def_file = tmpdir / "rewrite.def"
        rewrite_def_file.write_text("名詞\tNOUN\n")

        corpus_file = tmpdir / "corpus.txt"
        corpus_file.write_text("テスト\t名詞,一般,*,*,*,*,テスト,テスト,テスト\nEOS\n")

        model_file = tmpdir / "model.dat"

        # Train
        lindera.train(
            seed=str(seed_file),
            corpus=str(corpus_file),
            char_def=str(char_def_file),
            unk_def=str(unk_def_file),
            feature_def=str(feature_def_file),
            rewrite_def=str(rewrite_def_file),
            output=str(model_file),
            max_iter=3,
        )

        # Create base metadata
        metadata_file = tmpdir / "metadata.json"
        metadata_file.write_text(
            '{"name": "test-dict", "version": "1.0.0", "encoding": "utf-8"}'
        )

        # Export with metadata
        export_dir = tmpdir / "exported"
        lindera.export(
            model=str(model_file), output=str(export_dir), metadata=str(metadata_file)
        )

        # Verify metadata was created, with the files of `lindera export`
        assert sorted(p.name for p in export_dir.iterdir()) == sorted(
            EXPORTED_FILES + ["metadata.json"]
        )
        assert (export_dir / "metadata.json").stat().st_size > 0


def test_train_default_lambda_is_l2_coefficient_1():
    """Leaving lambda_ out trains with 1.0, the L2 coefficient of MeCab's
    -c 1.0 (#1261): the model is byte-identical to one trained with
    lambda_=1.0."""
    import lindera

    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir = Path(tmpdir)
        inputs = {
            "seed": (
                "seed.csv",
                (
                    "これ,0,0,0,名詞,代名詞,一般,*,*,*,これ,コレ,コレ\n"
                    "は,0,0,0,助詞,係助詞,*,*,*,*,は,ハ,ワ\n"
                ),
            ),
            "corpus": (
                "corpus.txt",
                (
                    "これ\t名詞,代名詞,一般,*,*,*,これ,コレ,コレ\n"
                    "は\t助詞,係助詞,*,*,*,*,は,ハ,ワ\n"
                    "EOS\n"
                ),
            ),
            "char_def": (
                "char.def",
                "DEFAULT 0 1 0\nHIRAGANA 1 1 0\n\n0x3041..0x3096 HIRAGANA\n",
            ),
            "unk_def": (
                "unk.def",
                (
                    "DEFAULT,0,0,0,名詞,一般,*,*,*,*,*,*,*\n"
                    "HIRAGANA,0,0,0,名詞,一般,*,*,*,*,*,*,*\n"
                ),
            ),
            "feature_def": (
                "feature.def",
                "UNIGRAM U00:%F[0]\nBIGRAM B00:%L[0]/%R[0]\n",
            ),
            "rewrite_def": ("rewrite.def", "名詞,一般\tNOUN\n"),
        }
        paths = {}
        for key, (name, content) in inputs.items():
            path = tmpdir / name
            path.write_text(content)
            paths[key] = str(path)

        def train(name, **kwargs):
            output = tmpdir / name
            lindera.train(
                **paths, output=str(output), max_iter=5, max_threads=1, **kwargs
            )
            return output.read_bytes()

        default = train("default.dat")
        explicit = train("explicit.dat", lambda_=1.0)
        assert default
        assert default == explicit
