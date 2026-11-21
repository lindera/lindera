# frozen_string_literal: true

require_relative 'test_helper'

# The batch methods of Tokenizer (#1402): every text is tokenized as a call of
# its own, on several threads, and the results come in the order of the
# texts, the same as the single calls'.
class TestTokenizeBatch < Minitest::Test
  BOCCHAN = File.expand_path('../../resources/bocchan.txt', __dir__)

  def setup
    @tokenizer = Lindera::Tokenizer.new(Lindera.load_dictionary('embedded://ipadic'), Lindera::Mode.normal, nil)
    # Enough input for a batch on several threads, and an empty text.
    @texts = File.read(BOCCHAN, encoding: 'UTF-8').split("\n") + ['']
  end

  def fields(tokens)
    tokens.map { |t| [t.surface, t.byte_start, t.byte_end, t.position, t.word_id, t.unknown?, t.details] }
  end

  def test_tokenize_batch_matches_tokenize
    expected = @texts.map { |text| fields(@tokenizer.tokenize(text)) }
    [nil, 1, 2, 8].each do |max_threads|
      batch = @tokenizer.tokenize_batch(@texts, max_threads)
      assert_equal expected, batch.map { |tokens| fields(tokens) }, "max_threads #{max_threads.inspect}"
    end
    assert_equal(expected, @tokenizer.tokenize_batch(@texts).map { |tokens| fields(tokens) })
  end

  def test_tokenize_surfaces_batch_matches_tokenize_surfaces
    expected = @texts.map { |text| @tokenizer.tokenize_surfaces(text) }
    assert_equal expected, @tokenizer.tokenize_surfaces_batch(@texts, 4)
    assert_equal expected, @tokenizer.tokenize_surfaces_batch(@texts)
  end

  def test_tokenize_nbest_batch_matches_tokenize_nbest
    some = @texts.first(60)
    expected = some.map do |text|
      @tokenizer.tokenize_nbest(text, 3, true, 5000).map { |tokens, cost| [fields(tokens), cost] }
    end
    [nil, 4].each do |max_threads|
      batch = @tokenizer.tokenize_nbest_batch(some, 3, true, 5000, max_threads)
      actual = batch.map { |results| results.map { |tokens, cost| [fields(tokens), cost] } }
      assert_equal expected, actual, "max_threads #{max_threads.inspect}"
    end
  end

  def test_tokenize_batch_takes_an_empty_array
    assert_equal [], @tokenizer.tokenize_batch([])
  end

  def test_tokenize_batch_rejects_a_string
    assert_raises(TypeError) { @tokenizer.tokenize_batch('東京') }
  end

  def test_tokenize_batch_rejects_a_wrong_number_of_arguments
    assert_raises(ArgumentError) { @tokenizer.tokenize_batch }
    assert_raises(ArgumentError) { @tokenizer.tokenize_batch(['東京'], 1, 2) }
  end
end
