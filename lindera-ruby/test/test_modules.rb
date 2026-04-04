# frozen_string_literal: true

require_relative 'test_helper'

class TestModules < Minitest::Test
  def test_version
    version = Lindera.version
    assert_kind_of String, version
    refute_empty version
  end

  def test_module_classes
    assert defined?(Lindera::Tokenizer)
    assert defined?(Lindera::TokenizerBuilder)
    assert defined?(Lindera::Dictionary)
    assert defined?(Lindera::UserDictionary)
    assert defined?(Lindera::Token)
    assert defined?(Lindera::Mode)
    assert defined?(Lindera::Penalty)
    assert defined?(Lindera::Metadata)
    assert defined?(Lindera::CompressionAlgorithm)
    assert defined?(Lindera::Schema)
    assert defined?(Lindera::FieldDefinition)
    assert defined?(Lindera::FieldType)
  end

  def test_module_functions
    assert Lindera.respond_to?(:load_dictionary)
    assert Lindera.respond_to?(:load_user_dictionary)
    assert Lindera.respond_to?(:build_dictionary)
    assert Lindera.respond_to?(:build_user_dictionary)
    assert Lindera.respond_to?(:version)
  end

  def test_schema_default
    schema = Lindera::Schema.create_default
    assert_equal 13, schema.field_count
    fields = schema.fields
    assert_equal 'surface', fields[0]
    assert_equal 'pronunciation', fields[12]
  end

  def test_schema_custom
    schema = Lindera::Schema.new(%w[surface reading pronunciation])
    assert_equal 3, schema.field_count
    assert_equal 0, schema.get_field_index('surface')
    assert_equal 1, schema.get_field_index('reading')
    assert_nil schema.get_field_index('nonexistent')
  end

  def test_schema_field_by_name
    schema = Lindera::Schema.create_default
    field = schema.get_field_by_name('surface')
    assert_kind_of Lindera::FieldDefinition, field
    assert_equal 0, field.index
    assert_equal 'surface', field.name
  end

  def test_metadata_default
    metadata = Lindera::Metadata.create_default
    assert_equal 'default', metadata.name
    assert_equal 'UTF-8', metadata.encoding
    assert_equal(-10_000, metadata.default_word_cost)
  end

  def test_metadata_to_hash
    metadata = Lindera::Metadata.create_default
    hash = metadata.to_h
    assert_kind_of Hash, hash
    assert_equal 'default', hash['name']
    assert_equal 'UTF-8', hash['encoding']
  end

  def test_mode
    mode = Lindera::Mode.new('normal')
    assert mode.normal?
    refute mode.decompose?
    assert_equal 'normal', mode.name

    mode2 = Lindera::Mode.new('decompose')
    refute mode2.normal?
    assert mode2.decompose?
  end

  def test_penalty
    penalty = Lindera::Penalty.new(2, 3000, 7, 1700)
    assert_equal 2, penalty.kanji_penalty_length_threshold
    assert_equal 3000, penalty.kanji_penalty_length_penalty
    assert_equal 7, penalty.other_penalty_length_threshold
    assert_equal 1700, penalty.other_penalty_length_penalty
  end
end
