# frozen_string_literal: true

require 'open-uri'
require 'tmpdir'
require 'lindera'

project_root = File.expand_path('..', __dir__)

url = 'https://lindera.dev/mecab-ipadic-2.7.0-20070801.tar.gz'
filename = '/tmp/mecab-ipadic-2.7.0-20070801.tar.gz'

# Download dictionary source file
puts "Downloading #{url}..."
URI.parse(url).open('User-Agent' => "lindera-ruby/#{Lindera.version}") do |remote|
  File.binwrite(filename, remote.read)
end

# Extract the dictionary source file
puts 'Extracting...'
system("tar xzf #{filename} -C /tmp/") || abort('Failed to extract')

source_path = '/tmp/mecab-ipadic-2.7.0-20070801'
destination_path = '/tmp/lindera-ipadic-2.7.0-20070801'
metadata_path = File.join(project_root, 'resources', 'ipadic_metadata.json')

metadata = Lindera::Metadata.from_json_file(metadata_path)

# Build dictionary
puts 'Building dictionary...'
Lindera.build_dictionary(source_path, destination_path, metadata)

# List all files in the destination directory
puts "\nFiles created in #{destination_path}:"
Dir.glob(File.join(destination_path, '**', '*')).select { |f| File.file?(f) }.sort.each do |file|
  rel_path = file.sub("#{destination_path}/", '')
  size = File.size(file).to_s.reverse.gsub(/(\d{3})(?=\d)/, '\\1,').reverse
  puts "  #{rel_path} (#{size} bytes)"
end
puts
