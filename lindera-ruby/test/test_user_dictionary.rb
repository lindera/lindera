# frozen_string_literal: true

require 'tmpdir'

require_relative 'test_helper'

class TestUserDictionary < Minitest::Test
  # Line 1 lies inside IPADIC's 1316 x 1316 connection matrix; line 2 has the
  # context IDs 60000, far outside it.
  OUT_OF_RANGE_CSV =
    "東京タワー,1288,1288,-1000,名詞,固有名詞,一般,*,*,*,東京タワー,トウキョウタワー,トウキョウタワー\n" \
    "東京スカイツリー,60000,60000,-1000,名詞,固有名詞,一般,*,*,*,東京スカイツリー,トウキョウスカイツリー,トウキョウスカイツリー\n"

  SIMPLE_CSV = "東京スカイツリー,カスタム名詞,トウキョウスカイツリー\n"

  def setup
    @dictionary = Lindera.load_dictionary('embedded://ipadic')
  end

  # Writes `content` to a CSV file in a new temporary directory and yields
  # its path; the directory is removed afterwards.
  def with_csv(content)
    Dir.mktmpdir('lindera-userdic-') do |dir|
      csv = File.join(dir, 'userdic.csv')
      File.write(csv, content, encoding: 'UTF-8')
      yield csv
    end
  end

  def test_load_user_dictionary_for_names_the_csv_line_and_surface
    with_csv(OUT_OF_RANGE_CSV) do |csv|
      error = assert_raises(RuntimeError) { Lindera.load_user_dictionary_for(csv, @dictionary) }
      assert_includes error.message, "line 2 (surface: '東京スカイツリー')"
      assert_includes error.message, 'left context id 60000 is out of range'
    end
  end

  def test_load_user_dictionary_for_loads_a_csv_inside_the_matrix
    baseline = Lindera::Tokenizer.new(@dictionary, 'normal', nil).tokenize('東京スカイツリー').map(&:surface)
    assert baseline.length > 1, "fixture assumption: #{baseline}"

    user_dictionary = with_csv(SIMPLE_CSV) { |csv| Lindera.load_user_dictionary_for(csv, @dictionary) }
    tokenizer = Lindera::Tokenizer.new(@dictionary, 'normal', user_dictionary)
    assert_equal ['東京スカイツリー'], tokenizer.tokenize('東京スカイツリー').map(&:surface)
  end
end
