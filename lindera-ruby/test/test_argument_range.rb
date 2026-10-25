# frozen_string_literal: true

require_relative 'test_helper'

# Integer arguments reject a value outside the range of their setting with a
# RangeError that names the parameter and the range (#1209, #1216).
#
# The binding takes these parameters as Integers and checks them against the
# range of the core's type, so the message reads as in the PHP, Node.js and
# WASM bindings (`default_word_cost must be an integer between -32768 and
# 32767, got 100000`), not as magnus's, which names only the Rust type.
class TestArgumentRange < Minitest::Test
  I32 = [-(2**31), (2**31) - 1].freeze
  I64 = [-(2**63), (2**63) - 1].freeze
  # usize and u64, which are both 64 bits wide on the platforms CI runs on.
  U64 = [0, (2**64) - 1].freeze
  # The counts of training, which start at 1: the trainer rejected 0 only
  # after building every lattice (#1275).
  COUNT = [1, (2**64) - 1].freeze

  # Added to a bound, takes a value past the 128-bit integers the binding
  # widens an argument to.
  PAST_I128 = 2**128

  # A path that does not exist: train checks its integers before its files.
  MISSING = '/nonexistent/issue1216'

  def self.metadata(cost: nil, left: nil, right: nil)
    Lindera::Metadata.new(nil, nil, cost, left, right, nil, nil, nil)
  end

  def self.tokenizer
    @tokenizer ||= Lindera::Tokenizer.new(Lindera.load_dictionary('embedded://ipadic'), Lindera::Mode.normal, nil)
  end

  def self.train(max_iter, max_threads)
    Lindera.train(*([MISSING] * 7), 0.01, max_iter, max_threads)
  end

  # A case per integer parameter, keyed `Class.method(parameter)`: a call that
  # passes the value it is given to that parameter, and the smallest and
  # largest value the parameter accepts.
  CASES = {
    'Metadata.new(default_word_cost)' => [->(v) { metadata(cost: v) }, -32_768, 32_767],
    'Metadata.new(default_left_context_id)' => [->(v) { metadata(left: v) }, 0, 65_535],
    'Metadata.new(default_right_context_id)' => [->(v) { metadata(right: v) }, 0, 65_535],
    'Penalty.new(kanji_penalty_length_threshold)' => [->(v) { Lindera::Penalty.new(v, nil, nil, nil) }, *U64],
    'Penalty.new(kanji_penalty_length_penalty)' => [->(v) { Lindera::Penalty.new(nil, v, nil, nil) }, *I32],
    'Penalty.new(other_penalty_length_threshold)' => [->(v) { Lindera::Penalty.new(nil, nil, v, nil) }, *U64],
    'Penalty.new(other_penalty_length_penalty)' => [->(v) { Lindera::Penalty.new(nil, nil, nil, v) }, *I32],
    'Schema#get_field_name(index)' => [->(v) { Lindera::Schema.create_default.get_field_name(v) }, *U64],
    'Token#get_detail(index)' => [->(v) { tokenizer.tokenize('東京')[0].get_detail(v) }, *U64],
    'Tokenizer#tokenize_nbest(n)' => [->(v) { tokenizer.tokenize_nbest('東京', v, nil, nil) }, *U64],
    'Tokenizer#tokenize_nbest(cost_threshold)' => [->(v) { tokenizer.tokenize_nbest('東京', 1, nil, v) }, *I64],
    'Lindera.train(max_iter)' => [->(v) { train(v, 1) }, *COUNT],
    'Lindera.train(max_threads)' => [->(v) { train(1, v) }, *COUNT]
  }.freeze

  CASES.each do |label, (call, min, max)|
    name = label[/\((\w+)\)\z/, 1]
    slug = label.gsub(/\W+/, '_')

    define_method("test_#{slug}_rejects_values_outside_the_range") do
      skip_without_train(label)

      # The values next to the range, and Bignums past 128 bits.
      [min - 1, max + 1, min - PAST_I128, max + PAST_I128].each do |value|
        error = assert_raises(RangeError, "#{label} accepted #{value}") { call.call(value) }
        assert_equal "#{name} must be an integer between #{min} and #{max}, got #{value}", error.message
      end
    end

    define_method("test_#{slug}_accepts_the_bounds") do
      skip_without_train(label)

      [min, max].each { |value| assert_accepted(label, call, value) }
    end
  end

  def test_an_accepted_value_is_kept_unchanged
    metadata = self.class.metadata(cost: -32_768, left: 65_535)
    assert_equal(-32_768, metadata.default_word_cost)
    assert_equal 65_535, metadata.default_left_context_id

    penalty = Lindera::Penalty.new(U64[1], I32[0], nil, nil)
    assert_equal U64[1], penalty.kanji_penalty_length_threshold
    assert_equal I32[0], penalty.kanji_penalty_length_penalty
  end

  private

  def skip_without_train(label)
    skip 'train feature not available' if label.start_with?('Lindera.train') && !Lindera.respond_to?(:train)
  end

  # Calls the case with a value it must accept. Lindera.train takes the value
  # and then raises for its missing files.
  def assert_accepted(label, call, value)
    call.call(value)
  rescue ArgumentError => e
    raise unless label.start_with?('Lindera.train') && e.message.include?('does not exist')
  end
end
