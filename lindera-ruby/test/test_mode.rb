# frozen_string_literal: true

require 'open3'
require 'rbconfig'

require_relative 'test_helper'

# Lindera::Mode, the counterpart of the Rust lindera::mode::Mode (#1218).
class TestMode < Minitest::Test
  def short_katakana_threshold
    Lindera::Penalty.new(nil, nil, 4, 3000)
  end

  def assert_default_penalty(penalty)
    assert_equal 2, penalty.kanji_penalty_length_threshold
    assert_equal 3000, penalty.kanji_penalty_length_penalty
    assert_equal 7, penalty.other_penalty_length_threshold
    assert_equal 1700, penalty.other_penalty_length_penalty
  end

  def test_normal
    mode = Lindera::Mode.normal
    assert_equal 'normal', mode.name
    assert_equal 'normal', mode.to_s
    assert_equal '#<Lindera::Mode: normal>', mode.inspect
    assert mode.normal?
    refute mode.decompose?
    assert_nil mode.penalty
  end

  def test_decompose_with_the_default_penalty
    [Lindera::Mode.decompose, Lindera::Mode.decompose(nil)].each do |mode|
      assert_equal 'decompose', mode.name
      assert_equal 'decompose', mode.to_s
      assert_match(/\A#<Lindera::Mode: decompose Penalty\(/, mode.inspect)
      assert mode.decompose?
      refute mode.normal?
      assert_default_penalty(mode.penalty)
    end
  end

  def test_decompose_keeps_the_penalty
    penalty = Lindera::Mode.decompose(short_katakana_threshold).penalty
    assert_equal 2, penalty.kanji_penalty_length_threshold
    assert_equal 3000, penalty.kanji_penalty_length_penalty
    assert_equal 4, penalty.other_penalty_length_threshold
    assert_equal 3000, penalty.other_penalty_length_penalty
  end

  def test_decompose_rejects_another_type_and_extra_arguments
    ['penalty', 0, Lindera::Mode.normal].each do |value|
      assert_raises(TypeError, value.inspect) { Lindera::Mode.decompose(value) }
    end
    assert_raises(ArgumentError) { Lindera::Mode.decompose(nil, nil) }
  end

  def test_from_str
    assert_equal Lindera::Mode.normal, Lindera::Mode.from_str('normal')
    assert_equal Lindera::Mode.decompose, Lindera::Mode.from_str('decompose')
  end

  def test_from_str_rejects_other_names
    # Case sensitive, as the Rust Mode parses; the RuntimeError that
    # set_mode('invalid') raised in v6.
    ['Normal', 'Decompose', 'DECOMPOSE', '', 'invalid'].each do |name|
      error = assert_raises(RuntimeError, name.inspect) { Lindera::Mode.from_str(name) }
      assert_match(/Invalid mode/, error.message)
    end
    assert_raises(TypeError) { Lindera::Mode.from_str(0) }
  end

  def test_equality_compares_the_name_and_the_penalty
    assert_equal Lindera::Mode.normal, Lindera::Mode.normal
    assert_equal Lindera::Mode.decompose, Lindera::Mode.decompose(Lindera::Penalty.new(nil, nil, nil, nil))
    assert_equal Lindera::Mode.decompose(short_katakana_threshold), Lindera::Mode.decompose(short_katakana_threshold)
    refute_equal Lindera::Mode.decompose, Lindera::Mode.decompose(short_katakana_threshold)
    refute_equal Lindera::Mode.normal, Lindera::Mode.decompose
    assert Lindera::Mode.normal != Lindera::Mode.decompose
  end

  def test_a_mode_is_not_equal_to_another_type
    ['normal', :normal, 0, nil, Lindera::Penalty.new(nil, nil, nil, nil)].each do |other|
      refute Lindera::Mode.normal == other, other.inspect
    end
  end

  def test_eql_and_hash_stay_by_identity
    # As the Rust Mode is Eq but not Hash, only == compares by value.
    refute Lindera::Mode.normal.eql?(Lindera::Mode.normal)
  end

  def test_a_mode_has_no_new
    assert_raises(TypeError) { Lindera::Mode.new }
    assert_raises(TypeError) { Lindera::Mode.allocate }
  end

  # The allocator must be undefined before the first Mode is created:
  # magnus undefines it only when the class first wraps a value, and
  # minitest runs the tests of one process in random order.
  def test_a_mode_has_no_new_in_a_fresh_process
    script = <<~RUBY
      require 'lindera'
      begin
        Lindera::Mode.new
      rescue TypeError
        exit 0
      end
      exit 1
    RUBY
    lib = File.expand_path('../lib', __dir__)
    _out, err, status = Open3.capture3(RbConfig.ruby, '-I', lib, '-e', script)
    assert status.success?, "Lindera::Mode.new did not raise TypeError: #{err}"
  end
end
