# frozen_string_literal: true

require 'lindera'

# Load the dictionary
dictionary = Lindera.load_dictionary('embedded://ipadic')

# Create a tokenizer with decompose mode
tokenizer = Lindera::Tokenizer.new(dictionary, 'decompose', nil)

text = '関西国際空港限定トートバッグを東京スカイツリーの最寄り駅であるとうきょうスカイツリー駅で買う'
puts "text: #{text}\n\n"

# Tokenize the text
tokens = tokenizer.tokenize(text)

tokens.each do |token|
  puts token.surface
end
