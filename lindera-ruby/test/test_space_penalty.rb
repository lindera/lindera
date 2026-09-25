# frozen_string_literal: true

require 'json'
require_relative 'test_helper'

# Left-space penalty (Korean) settings: TokenizerBuilder#set_space_penalty and
# the rules a dictionary's metadata.json ships (#1052).
class TestSpacePenalty < Minitest::Test
  # ko-dic's metadata.json in this repository, which ships mecab-ko-dic's rules.
  KO_DIC_METADATA = File.expand_path('../../lindera-ko-dic/metadata.json', __dir__)

  RULES = {
    'rules' => [
      { 'pos' => %w[JC JKB JKC JKG JKO JKQ JKS JKV JX], 'cost' => 6000 }
    ]
  }.freeze

  # A rule IPADIC can observe: a particle (助詞) right after a space costs so
  # much that 'は' in TEXT is no longer read as one.
  PARTICLE_RULES = { 'rules' => [{ 'pos' => ['助詞'], 'cost' => 30_000 }] }.freeze
  TEXT = '私 は 学生 です'
  DEFAULT_TOKENS = ['私/名詞', 'は/助詞', '学生/名詞', 'です/助動詞'].freeze

  def ipadic_builder
    builder = Lindera::TokenizerBuilder.new
    builder.set_dictionary('embedded://ipadic')
    builder
  end

  # Builds an IPADIC tokenizer after calling set_space_penalty with each of
  # `values` in turn, and returns "surface/part-of-speech" for TEXT.
  def tokens_after(*values)
    builder = ipadic_builder
    values.each { |value| builder.set_space_penalty(value) }
    builder.build.tokenize(TEXT).map { |token| "#{token.surface}/#{token.details[0]}" }
  end

  def test_set_space_penalty_accepts_the_config_forms
    builder = Lindera::TokenizerBuilder.new
    [nil, true, false, RULES, { 'rules' => [] }].each do |value|
      assert_nil builder.set_space_penalty(value), "rejected #{value.inspect}"
    end
  end

  def test_set_space_penalty_rejects_other_values
    builder = Lindera::TokenizerBuilder.new
    [1, 'false', ['JKS'], { 'rules' => 'JKS' }, { 'rules' => [{ 'pos' => ['JKS'] }] }].each do |value|
      error = assert_raises(RuntimeError, "accepted #{value.inspect}") do
        builder.set_space_penalty(value)
      end
      assert_match(/space_penalty/, error.message)
    end
  end

  # Values JSON cannot hold are rejected, not turned into nil (which would
  # silently reset an earlier setting).
  def test_set_space_penalty_rejects_unconvertible_values
    builder = Lindera::TokenizerBuilder.new
    [Float::NAN, Float::INFINITY, -Float::INFINITY, Object.new].each do |value|
      assert_raises(TypeError, "accepted #{value.inspect}") do
        builder.set_space_penalty(value)
      end
    end
  end

  # IPADIC ships no rules: requiring them (true) makes build fail.
  def test_true_fails_to_build_without_shipped_rules
    builder = ipadic_builder
    builder.set_space_penalty(true)
    assert_raises(RuntimeError) { builder.build }
  end

  # Each accepted value reaches the segmenter: explicit rules change the
  # output, and false or nil after them restore it.
  def test_setting_reaches_the_segmenter
    assert_equal DEFAULT_TOKENS, tokens_after
    assert_equal DEFAULT_TOKENS, tokens_after(nil)
    assert_equal DEFAULT_TOKENS, tokens_after(false)
    assert_equal DEFAULT_TOKENS, tokens_after(RULES)

    penalized = tokens_after(PARTICLE_RULES)
    refute_includes penalized, 'は/助詞'
    assert_equal 4, penalized.length

    assert_equal DEFAULT_TOKENS, tokens_after(PARTICLE_RULES, false)
    assert_equal DEFAULT_TOKENS, tokens_after(PARTICLE_RULES, nil)
  end

  # nil after true restores the default, so a dictionary without rules builds.
  def test_nil_resets_a_previous_setting
    builder = ipadic_builder
    builder.set_space_penalty(true)
    builder.set_space_penalty(nil)
    assert_kind_of Lindera::Tokenizer, builder.build
  end

  def test_metadata_to_hash_omits_space_penalty_by_default
    refute Lindera::Metadata.create_default.to_h.key?('space_penalty')
  end

  def test_metadata_from_ko_dic_json_exposes_space_penalty
    hash = Lindera::Metadata.from_json_file(KO_DIC_METADATA).to_h

    assert_includes hash['space_penalty'], 'JKS'
    rules = JSON.parse(hash['space_penalty'])['rules']
    assert_equal 2, rules.length
    assert_includes rules[1]['pos'], 'JKS'
    assert_equal 6000, rules[1]['cost']
  end
end
