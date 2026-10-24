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

  # Writes INPUTS to a new temporary directory and yields the directory and
  # the input files in the order Lindera.train takes them.
  def with_inputs
    Dir.mktmpdir('lindera-trainer-') do |dir|
      INPUTS.each { |name, content| File.write(File.join(dir, name), content, encoding: 'UTF-8') }
      inputs = %w[seed.csv corpus.txt char.def unk.def feature.def rewrite.def].map { |name| File.join(dir, name) }
      yield dir, inputs
    end
  end

  # Writes INPUTS to a new temporary directory, trains a model from them and
  # yields the directory and the model file.
  def with_model
    with_inputs do |dir, inputs|
      model = File.join(dir, 'model.dat')
      Lindera.train(*inputs, model, 0.01, 5, 1)
      yield dir, model
    end
  end

  # A nil lambda trains with 1.0, the L2 coefficient of MeCab's -c 1.0
  # (#1261): the model is byte-identical to one trained with 1.0.
  def test_train_with_nil_lambda_uses_one
    with_inputs do |dir, inputs|
      default = File.join(dir, 'default.dat')
      explicit = File.join(dir, 'explicit.dat')
      Lindera.train(*inputs, default, nil, 5, 1)
      Lindera.train(*inputs, explicit, 1.0, 5, 1)

      assert_predicate File.size(default), :positive?
      assert_equal File.binread(default), File.binread(explicit)
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

  # The training files of the repository, on which every setting below
  # trains a different model: the default converges before 100 iterations,
  # so eta: 0 changes where it stops (#1262).
  TRAINING = File.expand_path('../../resources/training', __dir__)

  # A path that does not exist: the settings are checked before any file.
  MISSING = '/nonexistent/issue1262'

  # Trains from TRAINING into `dir/name` with at most 100 iterations, the
  # positional `lambda` left out, and returns the model.
  def train_resources(dir, name, **settings)
    inputs = %w[seed.csv corpus.txt char.def unk.def feature.def rewrite.def].map { |file| File.join(TRAINING, file) }
    model = File.join(dir, name)
    Lindera.train(*inputs, model, nil, 100, 1, **settings)
    File.binread(model)
  end

  # regularization:, elastic_net_l1_ratio: and eta: reach the trainer, with
  # the CLI's defaults: giving the defaults trains the default model, a
  # ratio is ignored without elasticnet, and each other setting trains a
  # different model.
  def test_train_takes_the_regularization_type_the_ratio_and_eta
    Dir.mktmpdir('lindera-trainer-') do |dir|
      default = train_resources(dir, 'default.dat')

      assert_equal default,
                   train_resources(dir, 'explicit.dat', regularization: 'l2', elastic_net_l1_ratio: 0.5, eta: 5e-5)
      assert_equal default, train_resources(dir, 'l2-ratio.dat', regularization: 'l2', elastic_net_l1_ratio: 0.2)
      models = {
        'default' => default,
        'l1' => train_resources(dir, 'l1.dat', regularization: 'l1'),
        'elasticnet 0.2' => train_resources(dir, 'en02.dat', regularization: 'elasticnet', elastic_net_l1_ratio: 0.2),
        'elasticnet 0.8' => train_resources(dir, 'en08.dat', regularization: 'elasticnet', elastic_net_l1_ratio: 0.8),
        'eta 0' => train_resources(dir, 'eta0.dat', eta: 0)
      }
      same = models.keys.select { |name| models.values.count(models[name]) > 1 }

      assert_empty same, 'these settings trained the same model'
    end
  end

  # The positional arguments after the output path may be left out, as
  # before they had to be passed: seven arguments train with the defaults,
  # like ten with nil.
  def test_train_takes_seven_or_ten_positional_arguments
    with_inputs do |dir, inputs|
      seven = File.join(dir, 'seven.dat')
      ten = File.join(dir, 'ten.dat')
      Lindera.train(*inputs, seven)
      Lindera.train(*inputs, ten, nil, nil, nil)

      assert_predicate File.size(seven), :positive?
      assert_equal File.binread(seven), File.binread(ten)
    end
  end

  def train_missing(*positional, **settings)
    Lindera.train(*([MISSING] * 7), *positional, **settings)
  end

  # A setting the trainer would reject when training starts raises before
  # any file is read: an unknown type an ArgumentError, a number outside its
  # range a RangeError, as an integer outside its range does.
  def test_train_rejects_a_setting_before_reading_any_file
    ratio = 'elastic_net_l1_ratio must be a number between 0 and 1, got'
    [
      [ArgumentError, { regularization: 'l3' }, "regularization must be 'l1', 'l2' or 'elasticnet', got 'l3'"],
      [RangeError, [-1.0], 'lambda must be a finite number >= 0, got -1'],
      [RangeError, [Float::NAN], 'lambda must be a finite number >= 0, got NaN'],
      [RangeError, [Float::INFINITY], 'lambda must be a finite number >= 0, got Infinity'],
      [RangeError, { eta: -1 }, 'eta must be a finite number >= 0, got -1'],
      [RangeError, { eta: Float::NAN }, 'eta must be a finite number >= 0, got NaN'],
      [RangeError, { eta: Float::INFINITY }, 'eta must be a finite number >= 0, got Infinity'],
      [RangeError, { elastic_net_l1_ratio: -0.1 }, "#{ratio} -0.1"],
      [RangeError, { elastic_net_l1_ratio: 1.5 }, "#{ratio} 1.5"],
      [RangeError, { elastic_net_l1_ratio: Float::NAN }, "#{ratio} NaN"]
    ].each do |error, arguments, message|
      raised = assert_raises(error, arguments.inspect) do
        arguments.is_a?(Hash) ? train_missing(**arguments) : train_missing(*arguments)
      end

      assert_equal message, raised.message
    end
  end

  # The bounds, the other spellings and nil pass the checks, so the missing
  # seed file is what fails.
  def test_train_accepts_the_bounds_of_each_setting
    [
      [[0.0], {}],
      [[], { eta: 0 }],
      [[], { elastic_net_l1_ratio: 0 }],
      [[], { elastic_net_l1_ratio: 1 }],
      [[], { regularization: 'ElasticNet' }],
      [[], { regularization: 'elastic-net' }],
      [[nil, nil, nil], { regularization: nil, elastic_net_l1_ratio: nil, eta: nil }]
    ].each do |positional, settings|
      raised = assert_raises(ArgumentError, [positional, settings].inspect) { train_missing(*positional, **settings) }

      assert_includes raised.message, 'seed file does not exist'
    end
  end

  # An unknown keyword, an eleventh positional argument and a value of
  # another type are errors, not ignored or the default.
  def test_train_rejects_unknown_keywords_extra_arguments_and_other_types
    assert_raises(ArgumentError) { train_missing(lambda: 1.0) }
    assert_raises(ArgumentError) { train_missing(nil, nil, nil, nil) }
    assert_raises(TypeError) { train_missing(regularization: :l1) }
    assert_raises(TypeError) { train_missing(eta: '0') }
    assert_raises(TypeError) { train_missing(elastic_net_l1_ratio: '0.5') }
  end
end
