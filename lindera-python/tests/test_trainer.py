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


# The training files of the repository, on which every setting below trains
# a different model: the default converges before 100 iterations, so eta=0
# changes where it stops (#1262).
TRAINING = Path(__file__).resolve().parents[2] / "resources" / "training"

# A path that does not exist: the settings are checked before any file.
MISSING = "/nonexistent/issue1262"


def _train_resources(output, **kwargs):
    import lindera

    lindera.train(
        seed=str(TRAINING / "seed.csv"),
        corpus=str(TRAINING / "corpus.txt"),
        char_def=str(TRAINING / "char.def"),
        unk_def=str(TRAINING / "unk.def"),
        feature_def=str(TRAINING / "feature.def"),
        rewrite_def=str(TRAINING / "rewrite.def"),
        output=str(output),
        max_iter=100,
        max_threads=1,
        **kwargs,
    )
    return Path(output).read_bytes()


@pytest.mark.parametrize(
    "regularization,lambda_,ratio",
    [("l1", 0.0, 0.5), ("elasticnet", 0.0, 0.5), ("elasticnet", 1.0, 0.0)],
)
def test_train_with_zero_l1_coefficient(tmp_path, regularization, lambda_, ratio):
    """A zero effective L1 coefficient trains through the shared Rust optimizer."""
    model = _train_resources(
        tmp_path / "zero_l1.dat",
        regularization=regularization,
        lambda_=lambda_,
        elastic_net_l1_ratio=ratio,
    )
    assert model


def test_train_takes_the_regularization_type_the_ratio_and_eta():
    """regularization, elastic_net_l1_ratio and eta reach the trainer, with
    the CLI's defaults (#1262): giving the defaults trains the default model,
    a ratio is ignored without elasticnet, and each other setting trains a
    different model."""
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir = Path(tmpdir)
        default = _train_resources(tmpdir / "default.dat")
        assert default == _train_resources(
            tmpdir / "explicit.dat",
            regularization="l2",
            elastic_net_l1_ratio=0.5,
            eta=5e-5,
        )
        assert default == _train_resources(
            tmpdir / "l2_ratio.dat", regularization="l2", elastic_net_l1_ratio=0.2
        )
        models = {
            "default": default,
            "l1": _train_resources(tmpdir / "l1.dat", regularization="l1"),
            "elasticnet 0.2": _train_resources(
                tmpdir / "en02.dat",
                regularization="elasticnet",
                elastic_net_l1_ratio=0.2,
            ),
            "elasticnet 0.8": _train_resources(
                tmpdir / "en08.dat",
                regularization="elasticnet",
                elastic_net_l1_ratio=0.8,
            ),
            "eta 0": _train_resources(tmpdir / "eta0.dat", eta=0.0),
        }
        same = sorted(
            name
            for name, model in models.items()
            if list(models.values()).count(model) > 1
        )
        assert not same, f"these settings trained the same model: {same}"


def _train_missing(**kwargs):
    import lindera

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


@pytest.mark.parametrize(
    ("kwargs", "message"),
    [
        (
            {"regularization": "l3"},
            "regularization must be 'l1', 'l2' or 'elasticnet', got 'l3'",
        ),
        ({"lambda_": -1.0}, "lambda_ must be a finite number >= 0, got -1"),
        ({"lambda_": float("nan")}, "lambda_ must be a finite number >= 0, got NaN"),
        (
            {"lambda_": float("inf")},
            "lambda_ must be a finite number >= 0, got Infinity",
        ),
        ({"eta": -1.0}, "eta must be a finite number >= 0, got -1"),
        ({"eta": float("nan")}, "eta must be a finite number >= 0, got NaN"),
        ({"eta": float("inf")}, "eta must be a finite number >= 0, got Infinity"),
        (
            {"elastic_net_l1_ratio": -0.1},
            "elastic_net_l1_ratio must be a number between 0 and 1, got -0.1",
        ),
        (
            {"elastic_net_l1_ratio": 1.5},
            "elastic_net_l1_ratio must be a number between 0 and 1, got 1.5",
        ),
        (
            {"elastic_net_l1_ratio": float("nan")},
            "elastic_net_l1_ratio must be a number between 0 and 1, got NaN",
        ),
    ],
)
def test_train_rejects_a_setting_before_reading_any_file(kwargs, message):
    """A setting the trainer would reject when training starts raises a
    ValueError that names it, before any file is read (#1262)."""
    with pytest.raises(ValueError) as excinfo:
        _train_missing(**kwargs)
    assert str(excinfo.value) == message


@pytest.mark.parametrize(
    "kwargs",
    [
        {"lambda_": 0.0},
        {"eta": 0.0},
        {"elastic_net_l1_ratio": 0.0},
        {"elastic_net_l1_ratio": 1.0},
        {"regularization": "ElasticNet"},
        {"regularization": "elastic-net"},
    ],
)
def test_train_accepts_the_bounds_of_each_setting(kwargs):
    """The bounds and the other spellings pass the checks, so the missing
    seed file is what fails."""
    with pytest.raises(ValueError) as excinfo:
        _train_missing(**kwargs)
    assert "seed file does not exist" in str(excinfo.value)
