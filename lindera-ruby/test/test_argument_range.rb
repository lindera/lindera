# frozen_string_literal: true

require_relative 'test_helper'

# Integer arguments reject a value outside the range of their setting (#1209).
#
# The binding declares these parameters with the core's own types (i16, u16,
# i32, u64, usize), so magnus raises RangeError for a value the setting cannot
# hold instead of wrapping it, as the PHP, Node.js and WASM bindings used to.
class TestArgumentRange < Minitest::Test
  USIZE_MAX = (2**64) - 1

  # A path that does not exist: magnus converts the arguments before train runs.
  MISSING = '/nonexistent/issue1209'

  def self.metadata(cost: nil, left: nil, right: nil)
    Lindera::Metadata.new(nil, nil, cost, left, right, nil, nil, nil)
  end

  def self.tokenizer
    Lindera::Tokenizer.new(Lindera.load_dictionary('embedded://ipadic'), Lindera::Mode.normal, nil)
  end

  def self.train(max_iter, max_threads)
    Lindera.train(*([MISSING] * 7), 0.01, max_iter, max_threads)
  end

  # A call per integer parameter, with values outside its range.
  CASES = {
    'Metadata.new(default_word_cost)' => [->(v) { metadata(cost: v) }, [-32_769, 32_768, 100_000]],
    'Metadata.new(default_left_context_id)' => [->(v) { metadata(left: v) }, [-1, 65_536]],
    'Metadata.new(default_right_context_id)' => [->(v) { metadata(right: v) }, [-1, 65_536]],
    'Penalty.new(kanji_threshold)' => [->(v) { Lindera::Penalty.new(v, nil, nil, nil) }, [-1, USIZE_MAX + 1]],
    'Penalty.new(kanji_penalty)' => [->(v) { Lindera::Penalty.new(nil, v, nil, nil) }, [-(2**31) - 1, 2**31]],
    'Penalty.new(other_threshold)' => [->(v) { Lindera::Penalty.new(nil, nil, v, nil) }, [-1, USIZE_MAX + 1]],
    'Penalty.new(other_penalty)' => [->(v) { Lindera::Penalty.new(nil, nil, nil, v) }, [-(2**31) - 1, 2**31]],
    'Schema#get_field_name(index)' => [->(v) { Lindera::Schema.create_default.get_field_name(v) }, [-1]],
    'Token#get_detail(index)' => [->(v) { tokenizer.tokenize('東京')[0].get_detail(v) }, [-1]],
    'Tokenizer#tokenize_nbest(n)' => [->(v) { tokenizer.tokenize_nbest('東京', v, nil, nil) }, [-1]],
    'Lindera.train(max_iter)' => [->(v) { train(v, 1) }, [-1]],
    'Lindera.train(max_threads)' => [->(v) { train(1, v) }, [-1]]
  }.freeze

  CASES.each do |label, (call, values)|
    define_method("test_#{label.gsub(/\W+/, '_')}_rejects_values_outside_the_range") do
      skip 'train feature not available' if label.start_with?('Lindera.train') && !Lindera.respond_to?(:train)

      values.each do |value|
        assert_raises(RangeError, "#{label} accepted #{value}") { call.call(value) }
      end
    end
  end

  def test_an_accepted_value_is_kept_unchanged
    metadata = self.class.metadata(cost: -32_768, left: 65_535)
    assert_equal(-32_768, metadata.default_word_cost)
    assert_equal 65_535, metadata.default_left_context_id
    assert_equal(-(2**31), Lindera::Penalty.new(nil, -(2**31), nil, nil).kanji_penalty_length_penalty)
  end
end
