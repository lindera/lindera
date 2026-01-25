import lindera

def test_module_structure():
    """Test that the new submodule structure is present and working."""
    # Verify submodules are accessible as attributes of the main module
    assert hasattr(lindera, "tokenizer")
    assert hasattr(lindera, "dictionary")
    assert hasattr(lindera, "token")
    assert hasattr(lindera, "mode")
    assert hasattr(lindera, "metadata")
    assert hasattr(lindera, "schema")
    assert hasattr(lindera, "segmenter")
    assert hasattr(lindera, "character_filter")
    assert hasattr(lindera, "token_filter")
    assert hasattr(lindera, "error")
    
    # Verify we can also import them directly (Python's submodule behavior)
    # PyO3 modules added via add_submodule are accessible as lindera.tokenizer
    
    # Verify classes inside submodules
    assert hasattr(lindera.tokenizer, "Tokenizer")
    assert hasattr(lindera.tokenizer, "TokenizerBuilder")
    assert hasattr(lindera.dictionary, "Dictionary")
    assert hasattr(lindera.dictionary, "UserDictionary")
    assert hasattr(lindera.dictionary, "load_dictionary")
    assert hasattr(lindera.token, "Token")
    assert hasattr(lindera.mode, "Mode")
    assert hasattr(lindera.mode, "Penalty")
    assert hasattr(lindera.metadata, "Metadata")
    assert hasattr(lindera.schema, "Schema")
    assert hasattr(lindera.segmenter, "Segmenter")
    assert hasattr(lindera.error, "LinderaError")

def test_submodule_api_usage():
    """Test using the API via submodules."""
    # Load dictionary using submodule function
    dictionary = lindera.dictionary.load_dictionary("embedded://ipadic")
    assert isinstance(dictionary, lindera.dictionary.Dictionary)
    
    # Create tokenizer using submodule class
    tokenizer = lindera.tokenizer.Tokenizer(dictionary)
    
    # Tokenize
    tokens = tokenizer.tokenize("関西国際空港")
    
    # Verify results
    assert len(tokens) > 0
    # Every token should be an instance of lindera.token.Token
    assert isinstance(tokens[0], lindera.token.Token)
    assert tokens[0].surface.startswith("関西")

def test_top_level_aliases():
    """Test that top-level aliases are still working for backward compatibility."""
    assert lindera.Tokenizer is lindera.tokenizer.Tokenizer
    assert lindera.Dictionary is lindera.dictionary.Dictionary
    assert lindera.load_dictionary is lindera.dictionary.load_dictionary
    assert lindera.Token is lindera.token.Token

if __name__ == "__main__":
    test_module_structure()
    test_submodule_api_usage()
    test_top_level_aliases()
    print("Module structure tests PASSED!")
