# frozen_string_literal: true

require 'lindera'

project_root = File.expand_path('..', __dir__)

# Load the dictionary
dictionary = Lindera.load_dictionary('embedded://ipadic')

metadata = dictionary.metadata

# Load the user dictionary
user_dictionary_path = File.join(project_root, 'resources', 'ipadic_simple_userdic.csv')
user_dictionary = Lindera.load_user_dictionary(user_dictionary_path, metadata)

# Create a tokenizer
tokenizer = Lindera::Tokenizer.new(dictionary, 'normal', user_dictionary)

text = '関西国際空港限定トートバッグを東京スカイツリーの最寄り駅であるとうきょうスカイツリー駅で買う'
puts "text: #{text}\n\n"

# Tokenize the text
tokens = tokenizer.tokenize(text)

tokens.each do |token|
  puts token.surface
end
