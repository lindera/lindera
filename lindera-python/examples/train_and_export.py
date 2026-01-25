#!/usr/bin/env python3
"""
Example: Train a morphological analysis model and export dictionary files

This example demonstrates how to:
1. Train a model from a corpus using lindera.train()
2. Export dictionary files from the trained model using lindera.export()

Note: This requires the 'train' feature to be enabled when building lindera-python:
    maturin develop --features train
"""

import tempfile
from pathlib import Path

import lindera.trainer


def create_training_data(tmpdir: Path):
    """Create minimal training data based on lindera/resources/training format"""

    # Create seed lexicon (vocabulary with initial costs)
    # Format: surface,left_id,right_id,cost,features...
    seed_file = tmpdir / "seed.csv"
    seed_file.write_text(
        "外国,0,0,0,名詞,一般,*,*,*,*,外国,ガイコク,ガイコク\n"
        "人,0,0,0,名詞,接尾,一般,*,*,*,人,ジン,ジン\n"
        "参政,0,0,0,名詞,サ変接続,*,*,*,*,参政,サンセイ,サンセイ\n"
        "権,0,0,0,名詞,接尾,一般,*,*,*,権,ケン,ケン\n"
        "これ,0,0,0,名詞,代名詞,一般,*,*,*,これ,コレ,コレ\n"
        "は,0,0,0,助詞,係助詞,*,*,*,*,は,ハ,ワ\n"
        "テスト,0,0,0,名詞,サ変接続,*,*,*,*,テスト,テスト,テスト\n"
        "です,0,0,0,助動詞,*,*,*,特殊・デス,基本形,です,デス,デス\n"
        "。,0,0,0,記号,句点,*,*,*,*,。,。,。\n"
        "形態,0,0,0,名詞,一般,*,*,*,*,形態,ケイタイ,ケイタイ\n"
        "素,0,0,0,名詞,接尾,一般,*,*,*,素,ソ,ソ\n"
        "解析,0,0,0,名詞,サ変接続,*,*,*,*,解析,カイセキ,カイセキ\n"
        "を,0,0,0,助詞,格助詞,一般,*,*,*,を,ヲ,ヲ\n"
        "行う,0,0,0,動詞,自立,*,*,五段・ワ行促音便,基本形,行う,オコナウ,オコナウ\n"
    )

    # Create character definition (defines character types)
    char_def_file = tmpdir / "char.def"
    char_def_file.write_text(
        "# Character definition for training\n"
        "DEFAULT 0 1 0\n"
        "HIRAGANA 1 1 0\n"
        "KATAKANA 1 1 0\n"
        "KANJI 0 0 2\n"
        "ALPHA 1 1 0\n"
        "NUMERIC 1 1 0\n"
        "\n"
        "# Character mappings (simplified)\n"
        "0x3041..0x3096 HIRAGANA\n"
        "0x30A1..0x30F6 KATAKANA\n"
        "0x4E00..0x9FAF KANJI\n"
        "0x0030..0x0039 NUMERIC\n"
        "0x0041..0x005A ALPHA\n"
        "0x0061..0x007A ALPHA\n"
    )

    # Create unknown word definition (for out-of-vocabulary words)
    unk_def_file = tmpdir / "unk.def"
    unk_def_file.write_text(
        "# Unknown word definitions\n"
        "DEFAULT,0,0,0,名詞,一般,*,*,*,*,*,*,*\n"
        "HIRAGANA,0,0,0,名詞,一般,*,*,*,*,*,*,*\n"
        "KATAKANA,0,0,0,名詞,一般,*,*,*,*,*,*,*\n"
        "KANJI,0,0,0,名詞,一般,*,*,*,*,*,*,*\n"
        "ALPHA,0,0,0,名詞,固有名詞,一般,*,*,*,*,*,*\n"
        "NUMERIC,0,0,0,名詞,数,*,*,*,*,*,*,*\n"
    )

    # Create feature definition (defines features for CRF training)
    feature_def_file = tmpdir / "feature.def"
    feature_def_file.write_text(
        "# Feature template definitions for training\n"
        "# These define how features are extracted from the morphological data\n"
        "\n"
        "# Unigram features (word-level features)\n"
        "UNIGRAM U00:%F[0]    # Part of speech\n"
        "UNIGRAM U01:%F[0],%F?[1]    # POS + sub-category\n"
        "UNIGRAM U02:%F[0],%F[1],%F?[2]    # POS hierarchy\n"
        "\n"
        "# Bigram features (transition features between words)\n"
        "# Format: BIGRAM label:%L[index]/%R[index]\n"
        "# %L = left context (previous word), %R = right context (next word)\n"
        "BIGRAM B00:%L[0]/%R[0]    # POS-to-POS transition\n"
        "BIGRAM B01:%L[0],%L?[1]/%R[0]    # Left POS hierarchy to right POS\n"
        "BIGRAM B02:%L[0]/%R[0],%R?[1]    # Left POS to right POS hierarchy\n"
        "BIGRAM B03:%L[0],%L[1],%L?[2]/%R[0]    # Detailed left to simple right\n"
    )

    # Create rewrite definition (for feature rewriting)
    rewrite_def_file = tmpdir / "rewrite.def"
    rewrite_def_file.write_text(
        "# Rewrite rules for feature normalization\n"
        "# Format: original_pattern\treplacement_pattern\n"
        "\n"
        '# Test rewrite: convert "名詞,一般" to "NOUN,GENERAL"\n'
        "名詞,一般\tNOUN,GENERAL\n"
        "\n"
        '# Test rewrite: convert "助詞,係助詞" to "PARTICLE,KAKUJOSHI"\n'
        "助詞,係助詞\tPARTICLE,KAKUJOSHI\n"
        "\n"
        "# Normalize numeric expressions\n"
        "数\tNUM\n"
    )

    # Create training corpus (annotated text)
    # Format: surface\tfeatures (tab-separated)
    # Each sentence ends with "EOS"
    corpus_file = tmpdir / "corpus.txt"
    corpus_file.write_text(
        "外国\t名詞,一般,*,*,*,*,外国,ガイコク,ガイコク\n"
        "人\t名詞,接尾,一般,*,*,*,人,ジン,ジン\n"
        "参政\t名詞,サ変接続,*,*,*,*,参政,サンセイ,サンセイ\n"
        "権\t名詞,接尾,一般,*,*,*,権,ケン,ケン\n"
        "EOS\n"
        "\n"
        "これ\t名詞,代名詞,一般,*,*,*,これ,コレ,コレ\n"
        "は\t助詞,係助詞,*,*,*,*,は,ハ,ワ\n"
        "テスト\t名詞,サ変接続,*,*,*,*,テスト,テスト,テスト\n"
        "です\t助動詞,*,*,*,特殊・デス,基本形,です,デス,デス\n"
        "。\t記号,句点,*,*,*,*,。,。,。\n"
        "EOS\n"
        "\n"
        "形態\t名詞,一般,*,*,*,*,形態,ケイタイ,ケイタイ\n"
        "素\t名詞,接尾,一般,*,*,*,素,ソ,ソ\n"
        "解析\t名詞,サ変接続,*,*,*,*,解析,カイセキ,カイセキ\n"
        "を\t助詞,格助詞,一般,*,*,*,を,ヲ,ヲ\n"
        "行う\t動詞,自立,*,*,五段・ワ行促音便,基本形,行う,オコナウ,オコナウ\n"
        "EOS\n"
    )

    # Create metadata for dictionary export
    metadata_file = tmpdir / "metadata.json"
    metadata_file.write_text(
        "{\n" '  "name": "custom-dict",\n' '  "version": "1.0.0",\n' '  "encoding": "utf-8"\n' "}\n"
    )

    return {
        "seed": seed_file,
        "char_def": char_def_file,
        "unk_def": unk_def_file,
        "feature_def": feature_def_file,
        "rewrite_def": rewrite_def_file,
        "corpus": corpus_file,
        "metadata": metadata_file,
    }


def main():
    """Main training and export workflow"""
    print("=== Lindera Training and Export Example ===\n")

    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir = Path(tmpdir)
        print(f"Working directory: {tmpdir}\n")

        # Step 1: Create training data
        print("Step 1: Creating training data...")
        files = create_training_data(tmpdir)
        print("✓ Training data created\n")

        # Step 2: Train model
        print("Step 2: Training model...")
        model_file = tmpdir / "model.dat"

        lindera.trainer.train(
            seed=str(files["seed"]),
            corpus=str(files["corpus"]),
            char_def=str(files["char_def"]),
            unk_def=str(files["unk_def"]),
            feature_def=str(files["feature_def"]),
            rewrite_def=str(files["rewrite_def"]),
            output=str(model_file),
            lambda_=0.01,  # L1 regularization
            max_iter=10,  # Number of training iterations
            max_threads=None,  # Auto-detect CPU cores
        )

        print(f"✓ Model trained and saved to: {model_file}\n")

        # Step 3: Export dictionary files
        print("Step 3: Exporting dictionary files...")
        export_dir = tmpdir / "exported_dict"

        lindera.trainer.export(
            model=str(model_file),
            output=str(export_dir),
            metadata=str(files["metadata"]),
        )

        print(f"✓ Dictionary files exported to: {export_dir}\n")

        # Step 4: List exported files
        print("Step 4: Exported files:")
        exported_files = sorted(export_dir.glob("*"))
        for file in exported_files:
            size = file.stat().st_size
            print(f"  - {file.name} ({size:,} bytes)")

        print("\n✓ Training and export completed successfully!")


if __name__ == "__main__":
    main()
