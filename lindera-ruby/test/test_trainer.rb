# frozen_string_literal: true

require 'tmpdir'
require_relative 'test_helper'

# Lindera.export writes the files `lindera export` writes (#1169).
class TestTrainer < Minitest::Test
  # The files `lindera export` writes besides metadata.json.
  EXPORTED_FILES = %w[
    lex.csv matrix.def unk.def char.def feature.def rewrite.def left-id.def right-id.def
  ].freeze

  # The training input files and their contents.
  INPUTS = {
    'seed.csv' => "これ,0,0,0,名詞,代名詞,一般,*,*,*,これ,コレ,コレ\n" \
                  "は,0,0,0,助詞,係助詞,*,*,*,*,は,ハ,ワ\n",
    'char.def' => "DEFAULT 0 1 0\nHIRAGANA 1 1 0\n\n0x3041..0x3096 HIRAGANA\n",
    'unk.def' => "DEFAULT,0,0,0,名詞,一般,*,*,*,*,*,*,*\n" \
                 "HIRAGANA,0,0,0,名詞,一般,*,*,*,*,*,*,*\n",
    'feature.def' => "UNIGRAM U00:%F[0]\nBIGRAM B00:%L[0]/%R[0]\n",
    'rewrite.def' => "名詞,一般\tNOUN\n",
    'corpus.txt' => "これ\t名詞,代名詞,一般,*,*,*,これ,コレ,コレ\n" \
                    "は\t助詞,係助詞,*,*,*,*,は,ハ,ワ\n" \
                    "EOS\n"
  }.freeze

  def setup
    skip 'train feature not available' unless Lindera.respond_to?(:train)
  end

  # Writes INPUTS to a new temporary directory, trains a model from them and
  # yields the directory and the model file.
  def with_model
    Dir.mktmpdir('lindera-trainer-') do |dir|
      INPUTS.each { |name, content| File.write(File.join(dir, name), content, encoding: 'UTF-8') }
      inputs = %w[seed.csv corpus.txt char.def unk.def feature.def rewrite.def].map { |name| File.join(dir, name) }
      model = File.join(dir, 'model.dat')
      Lindera.train(*inputs, model, 0.01, 5, 1)
      yield dir, model
    end
  end

  def test_export_writes_the_files_of_lindera_export
    with_model do |dir, model|
      output = File.join(dir, 'exported')
      Lindera.export(model, output, nil)

      assert_equal EXPORTED_FILES.sort, Dir.children(output).sort
      EXPORTED_FILES.each { |name| assert_predicate File.size(File.join(output, name)), :positive?, name }
      # The definition files are those the model was trained with.
      %w[char.def feature.def rewrite.def].each do |name|
        assert_equal File.binread(File.join(dir, name)), File.binread(File.join(output, name)), name
      end
    end
  end

  def test_export_with_metadata_writes_metadata_json_too
    with_model do |dir, model|
      metadata = File.join(dir, 'metadata.json')
      File.write(metadata, '{"name": "test-dict", "version": "1.0.0", "encoding": "utf-8"}')
      output = File.join(dir, 'exported')
      Lindera.export(model, output, metadata)

      assert_equal (EXPORTED_FILES + ['metadata.json']).sort, Dir.children(output).sort
    end
  end
end
