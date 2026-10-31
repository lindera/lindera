# frozen_string_literal: true

require 'json'
require 'tmpdir'

require_relative 'test_helper'

class TestBuildDictionary < Minitest::Test
  # The source of a dictionary with a 4x4 connection matrix, whose
  # metadata.json enables connection_id_mapping and whose one entry has the
  # context IDs 2, with a histogram that counts 1, 3 and 2 for IDs 1, 2 and 3.
  FIXTURE = File.expand_path('../../resources/connection_id_mapping', __dir__)

  # Builds the fixture into a new temporary directory with the keyword
  # arguments `kwargs`, and returns the `context_id_map` of the built
  # metadata.json; the directory is removed afterwards.
  def built_context_id_map(**kwargs)
    Dir.mktmpdir('lindera-build-') do |dir|
      metadata = Lindera::Metadata.from_json_file(File.join(FIXTURE, 'metadata.json'))
      Lindera.build_dictionary(File.join(FIXTURE, 'source'), dir, metadata, **kwargs)
      JSON.parse(File.read(File.join(dir, 'metadata.json'), encoding: 'UTF-8'))['context_id_map']
    end
  end

  def test_build_dictionary_keeps_connection_id_mapping_of_the_metadata
    # Without a histogram, the IDs other than 0 are ranked by the count of
    # dictionary entries, ties by ID: 2 (the entry's), then 1 and 3.
    assert_equal({ 'left' => [0, 2, 1, 3], 'right' => [0, 2, 1, 3] }, built_context_id_map)
  end

  def test_build_dictionary_ranks_context_ids_by_the_histogram
    histogram = File.join(FIXTURE, 'context_id_freq.txt')
    assert_equal({ 'left' => [0, 3, 1, 2], 'right' => [0, 3, 1, 2] },
                 built_context_id_map(context_id_freq: histogram))
  end
end
