# frozen_string_literal: true

require 'rbconfig'
require_relative 'test_helper'

# The nesting limit on arguments converted to JSON (#1062).
class TestArgumentDepth < Minitest::Test
  TOO_DEEP = /nested more than 128 levels deep/
  LIB_DIR = File.expand_path('../lib', __dir__)

  # The three builder methods that convert a Ruby value to JSON.
  CALLS = {
    'set_space_penalty' => ->(builder, value) { builder.set_space_penalty(value) },
    'append_character_filter' => ->(builder, value) { builder.append_character_filter('unicode_normalize', value) },
    'append_token_filter' => ->(builder, value) { builder.append_token_filter('lowercase', value) }
  }.freeze

  # The same calls as Ruby source, for the child process.
  CALL_SOURCES = {
    'set_space_penalty' => 'builder.set_space_penalty(value)',
    'append_character_filter' => "builder.append_character_filter('unicode_normalize', value)",
    'append_token_filter' => "builder.append_token_filter('lowercase', value)"
  }.freeze

  # Without the limit these overflowed the native stack: usually a
  # SystemStackError, but some runs hung, so they run in a child process
  # with a timeout.
  ISOLATED_VALUES = {
    'a value that contains itself' => "value = { 'rules' => [] }; value['self'] = value",
    '10,000 levels' => "value = {}; 9_999.times { value = { 'a' => value } }"
  }.freeze

  # Returns { 'a' => { 'a' => ... } } made of `levels` nested hashes.
  def nested_hashes(levels)
    value = {}
    (levels - 1).times { value = { 'a' => value } }
    value
  end

  # Returns { 'a' => [[...]] }: a hash holding `levels - 1` nested arrays.
  # The filter methods take a hash, so the arrays are wrapped in one.
  def nested_arrays(levels)
    value = []
    (levels - 2).times { value = [value] }
    { 'a' => value }
  end

  # Runs `CALL_SOURCES[method]` after `value_source` in a child
  # process that is killed after `timeout` seconds.
  #
  # Returns the process status and what the child printed.
  def run_isolated(method, value_source, timeout: 30)
    script = <<~RUBY
      require 'lindera'
      #{value_source}
      builder = Lindera::TokenizerBuilder.new
      begin
        #{CALL_SOURCES.fetch(method)}
        print 'returned'
      rescue ArgumentError => e
        print "raised: \#{e.message}"
      end
    RUBY
    reader, writer = IO.pipe
    pid = Process.spawn(RbConfig.ruby, '-I', LIB_DIR, '-e', script, out: writer, err: File::NULL)
    writer.close
    deadline = Process.clock_gettime(Process::CLOCK_MONOTONIC) + timeout
    status = nil
    until status
      _, status = Process.wait2(pid, Process::WNOHANG)
      next if status

      if Process.clock_gettime(Process::CLOCK_MONOTONIC) > deadline
        Process.kill(:KILL, pid)
        _, status = Process.wait2(pid)
      else
        sleep 0.05
      end
    end
    [status, reader.read]
  ensure
    reader&.close
  end

  CALLS.each_key do |method|
    define_method("test_#{method}_accepts_128_levels") do
      [nested_hashes(128), nested_arrays(128)].each do |value|
        CALLS[method].call(Lindera::TokenizerBuilder.new, value)
      rescue StandardError => e
        # set_space_penalty rejects the shape, but not for being too deep.
        refute_match TOO_DEEP, e.message
      end
    end

    define_method("test_#{method}_rejects_129_levels") do
      [nested_hashes(129), nested_arrays(129)].each do |value|
        error = assert_raises(ArgumentError) { CALLS[method].call(Lindera::TokenizerBuilder.new, value) }
        assert_match TOO_DEEP, error.message
      end
    end

    define_method("test_#{method}_keeps_the_builder_usable") do
      builder = Lindera::TokenizerBuilder.new
      assert_raises(ArgumentError) { CALLS[method].call(builder, nested_hashes(129)) }
      CALLS[method].call(builder, method == 'set_space_penalty' ? nil : {})
    end

    ISOLATED_VALUES.each do |name, value_source|
      define_method("test_#{method}_rejects_#{name.tr(' ,', '_')}_in_a_child_process") do
        status, output = run_isolated(method, value_source)
        assert status.success?, "child exited with #{status.inspect}: #{output}"
        assert_match(/\Araised: /, output)
        assert_match TOO_DEEP, output
      end
    end
  end
end
