#!/usr/bin/env ruby
# frozen_string_literal: true

# Per-call benchmark for lindera-ruby (interleaved A/B harness member).
#
# Prints one TSV line per workload: "<workload>\t<min microseconds/call>".
# With --verify, prints the token surfaces instead (for the byte-identical
# correctness gate). Run from a shell where the compiled extension is on
# the load path (e.g. `bundle exec ruby scripts/benchmarks/bench_ruby.rb`
# from lindera-ruby/). See scripts/benchmarks/README.md for the protocol.

require 'lindera'

TEXT = ENV.fetch('BENCH_TEXT', 'すもももももももものうち')
CALLS = ENV.fetch('BENCH_CALLS', '2000').to_i
INNER = ENV.fetch('BENCH_INNER', '10').to_i

def us_per_call(calls)
  start = Process.clock_gettime(Process::CLOCK_MONOTONIC)
  calls.times { yield }
  (Process.clock_gettime(Process::CLOCK_MONOTONIC) - start) / calls * 1_000_000
end

def min_us(calls, inner, &block)
  Array.new(inner) { us_per_call(calls, &block) }.min
end

dictionary = Lindera.load_dictionary('embedded://ipadic')
tokenizer = Lindera::Tokenizer.new(dictionary, 'normal', nil)
has_surfaces = tokenizer.respond_to?(:tokenize_surfaces)

if ARGV.include?('--verify')
  puts "tokenize\t#{tokenizer.tokenize(TEXT).map(&:surface).join("\t")}"
  puts "surfaces\t#{tokenizer.tokenize_surfaces(TEXT).join("\t")}" if has_surfaces
else
  puts format("tokenize\t%.3f", min_us(CALLS, INNER) { tokenizer.tokenize(TEXT) })
  if has_surfaces
    puts format("surfaces\t%.3f", min_us(CALLS, INNER) { tokenizer.tokenize_surfaces(TEXT) })
  end
end
