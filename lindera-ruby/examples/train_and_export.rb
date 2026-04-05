# frozen_string_literal: true

# Example: Train a morphological analysis model and export dictionary files
#
# This example demonstrates how to:
# 1. Train a model from a corpus using Lindera.train
# 2. Export dictionary files from the trained model using Lindera.export
#
# Note: This requires the 'train' feature to be enabled when building lindera-ruby:
#   LINDERA_FEATURES="embed-ipadic,train" bundle exec rake compile

require 'tmpdir'
require 'json'
require 'lindera'

def create_training_data(tmpdir)
  # Create seed lexicon (vocabulary with initial costs)
  # Format: surface,left_id,right_id,cost,features...
  seed_file = File.join(tmpdir, 'seed.csv')
  File.write(seed_file, <<~CSV)
    外国,0,0,0,名詞,一般,*,*,*,*,外国,ガイコク,ガイコク
    人,0,0,0,名詞,接尾,一般,*,*,*,人,ジン,ジン
    参政,0,0,0,名詞,サ変接続,*,*,*,*,参政,サンセイ,サンセイ
    権,0,0,0,名詞,接尾,一般,*,*,*,権,ケン,ケン
    これ,0,0,0,名詞,代名詞,一般,*,*,*,これ,コレ,コレ
    は,0,0,0,助詞,係助詞,*,*,*,*,は,ハ,ワ
    テスト,0,0,0,名詞,サ変接続,*,*,*,*,テスト,テスト,テスト
    です,0,0,0,助動詞,*,*,*,特殊・デス,基本形,です,デス,デス
    。,0,0,0,記号,句点,*,*,*,*,。,。,。
    形態,0,0,0,名詞,一般,*,*,*,*,形態,ケイタイ,ケイタイ
    素,0,0,0,名詞,接尾,一般,*,*,*,素,ソ,ソ
    解析,0,0,0,名詞,サ変接続,*,*,*,*,解析,カイセキ,カイセキ
    を,0,0,0,助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
    行う,0,0,0,動詞,自立,*,*,五段・ワ行促音便,基本形,行う,オコナウ,オコナウ
  CSV

  # Create character definition (defines character types)
  char_def_file = File.join(tmpdir, 'char.def')
  File.write(char_def_file, <<~DEF)
    # Character definition for training
    DEFAULT 0 1 0
    HIRAGANA 1 1 0
    KATAKANA 1 1 0
    KANJI 0 0 2
    ALPHA 1 1 0
    NUMERIC 1 1 0

    # Character mappings (simplified)
    0x3041..0x3096 HIRAGANA
    0x30A1..0x30F6 KATAKANA
    0x4E00..0x9FAF KANJI
    0x0030..0x0039 NUMERIC
    0x0041..0x005A ALPHA
    0x0061..0x007A ALPHA
  DEF

  # Create unknown word definition (for out-of-vocabulary words)
  unk_def_file = File.join(tmpdir, 'unk.def')
  File.write(unk_def_file, <<~DEF)
    # Unknown word definitions
    DEFAULT,0,0,0,名詞,一般,*,*,*,*,*,*,*
    HIRAGANA,0,0,0,名詞,一般,*,*,*,*,*,*,*
    KATAKANA,0,0,0,名詞,一般,*,*,*,*,*,*,*
    KANJI,0,0,0,名詞,一般,*,*,*,*,*,*,*
    ALPHA,0,0,0,名詞,固有名詞,一般,*,*,*,*,*,*
    NUMERIC,0,0,0,名詞,数,*,*,*,*,*,*,*
  DEF

  # Create feature definition (defines features for CRF training)
  feature_def_file = File.join(tmpdir, 'feature.def')
  File.write(feature_def_file, <<~DEF)
    # Feature template definitions for training

    # Unigram features (word-level features)
    UNIGRAM U00:%F[0]    # Part of speech
    UNIGRAM U01:%F[0],%F?[1]    # POS + sub-category
    UNIGRAM U02:%F[0],%F[1],%F?[2]    # POS hierarchy

    # Bigram features (transition features between words)
    BIGRAM B00:%L[0]/%R[0]    # POS-to-POS transition
    BIGRAM B01:%L[0],%L?[1]/%R[0]    # Left POS hierarchy to right POS
    BIGRAM B02:%L[0]/%R[0],%R?[1]    # Left POS to right POS hierarchy
    BIGRAM B03:%L[0],%L[1],%L?[2]/%R[0]    # Detailed left to simple right
  DEF

  # Create rewrite definition (for feature rewriting)
  rewrite_def_file = File.join(tmpdir, 'rewrite.def')
  File.write(rewrite_def_file, <<~DEF)
    # Rewrite rules for feature normalization

    名詞,一般\tNOUN,GENERAL

    助詞,係助詞\tPARTICLE,KAKUJOSHI

    # Normalize numeric expressions
    数\tNUM
  DEF

  # Create training corpus (annotated text)
  corpus_file = File.join(tmpdir, 'corpus.txt')
  File.write(corpus_file, <<~CORPUS)
    外国\t名詞,一般,*,*,*,*,外国,ガイコク,ガイコク
    人\t名詞,接尾,一般,*,*,*,人,ジン,ジン
    参政\t名詞,サ変接続,*,*,*,*,参政,サンセイ,サンセイ
    権\t名詞,接尾,一般,*,*,*,権,ケン,ケン
    EOS

    これ\t名詞,代名詞,一般,*,*,*,これ,コレ,コレ
    は\t助詞,係助詞,*,*,*,*,は,ハ,ワ
    テスト\t名詞,サ変接続,*,*,*,*,テスト,テスト,テスト
    です\t助動詞,*,*,*,特殊・デス,基本形,です,デス,デス
    。\t記号,句点,*,*,*,*,。,。,。
    EOS

    形態\t名詞,一般,*,*,*,*,形態,ケイタイ,ケイタイ
    素\t名詞,接尾,一般,*,*,*,素,ソ,ソ
    解析\t名詞,サ変接続,*,*,*,*,解析,カイセキ,カイセキ
    を\t助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
    行う\t動詞,自立,*,*,五段・ワ行促音便,基本形,行う,オコナウ,オコナウ
    EOS
  CORPUS

  # Create metadata for dictionary export
  metadata_file = File.join(tmpdir, 'metadata.json')
  File.write(metadata_file, JSON.pretty_generate(
    name: 'custom-dict',
    version: '1.0.0',
    encoding: 'utf-8'
  ))

  {
    seed: seed_file,
    char_def: char_def_file,
    unk_def: unk_def_file,
    feature_def: feature_def_file,
    rewrite_def: rewrite_def_file,
    corpus: corpus_file,
    metadata: metadata_file
  }
end

puts "=== Lindera Training and Export Example ===\n\n"

Dir.mktmpdir('lindera-train-') do |tmpdir|
  puts "Working directory: #{tmpdir}\n\n"

  # Step 1: Create training data
  puts 'Step 1: Creating training data...'
  files = create_training_data(tmpdir)
  puts "Training data created\n\n"

  # Step 2: Train model
  puts 'Step 2: Training model...'
  model_file = File.join(tmpdir, 'model.dat')

  Lindera.train(
    files[:seed],
    files[:corpus],
    files[:char_def],
    files[:unk_def],
    files[:feature_def],
    files[:rewrite_def],
    model_file,
    0.01,  # lambda (L1 regularization)
    10,    # max_iter
    nil    # max_threads (auto-detect)
  )

  puts "Model trained and saved to: #{model_file}\n\n"

  # Step 3: Export dictionary files
  puts 'Step 3: Exporting dictionary files...'
  export_dir = File.join(tmpdir, 'exported_dict')

  Lindera.export(model_file, export_dir, files[:metadata])

  puts "\nStep 4: Exported files:"
  Dir.glob(File.join(export_dir, '*')).sort.each do |file|
    size = File.size(file).to_s.reverse.gsub(/(\d{3})(?=\d)/, '\\1,').reverse
    puts "  - #{File.basename(file)} (#{size} bytes)"
  end

  puts "\nTraining and export completed successfully!"
end
