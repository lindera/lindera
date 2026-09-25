# frozen_string_literal: true

# Smoke test for an installed lindera gem: loads it through RubyGems, then
# loads a dictionary and tokenizes a sentence with it.
#
# Usage: ruby smoke_test.rb [DICTIONARY_URI]   (default: embedded://ipadic)

require 'lindera'

spec = Gem.loaded_specs['lindera']
abort 'lindera was not loaded from an installed gem' unless spec
puts "lindera gem #{spec.version} (#{spec.full_gem_path}), Lindera.version #{Lindera.version}"

dictionary = Lindera.load_dictionary(ARGV.fetch(0, 'embedded://ipadic'))
tokenizer = Lindera::Tokenizer.new(dictionary, 'normal', nil)
surfaces = tokenizer.tokenize('関西国際空港限定トートバッグ').map(&:surface)
puts surfaces.join(' / ')

expected = %w[関西国際空港 限定 トートバッグ]
abort "expected #{expected.inspect}, got #{surfaces.inspect}" unless surfaces == expected
puts 'OK'
