from lindera import Tokenizer, TokenizerBuilder
from lindera import Token
import lindera

def test_tokenize_returns_token_objects():
    # Load dictionary explicitly to ensure it works
    try:
        # Use URI format for embedded dictionary
        dictionary = lindera.load_dictionary("embedded://ipadic")
        tokenizer = lindera.Tokenizer(dictionary)
    except Exception as e:
        print(f"Failed to load dictionary: {e}")
        # Fallback for testing environment if ipadic is not embedded
        raise e

    # TokenizerBuilder approach (alternative)
    # tokenizer = TokenizerBuilder().set_segmenter_dictionary("ipadic").build()
    
    # Tokenize text
    text = "関西国際空港"
    tokens = tokenizer.tokenize(text)
    
    # Verify we got a list of Token objects
    assert isinstance(tokens, list)
    assert len(tokens) > 0
    assert isinstance(tokens[0], Token)
    
    # Verify Token attributes
    # Expected tokens: 関西, 国際, 空港 (assuming IPADIC normal mode)
    # Note: Using default dictionary might result in different segmentation depending on version, 
    # but we just check the first token's structure.
    
    token = tokens[0]
    
    # Check surface attribute
    assert hasattr(token, "surface")
    assert isinstance(token.surface, str)
    assert token.surface == "関西" or token.surface.startswith("関西") # Basic sanity check
    
    # Check other attributes
    assert hasattr(token, "byte_start")
    assert isinstance(token.byte_start, int)
    
    assert hasattr(token, "byte_end")
    assert isinstance(token.byte_end, int)
    
    assert hasattr(token, "position")
    assert isinstance(token.position, int)
    
    assert hasattr(token, "word_id")
    assert isinstance(token.word_id, int)
    
    assert hasattr(token, "details")
    assert isinstance(token.details, list)
    assert len(token.details) > 0
    
    # Check get_detail method
    first_detail = token.get_detail(0)
    assert first_detail is not None
    assert first_detail == token.details[0]
    
    # Check out of bounds
    assert token.get_detail(9999) is None

    print("PyToken verify success!")

if __name__ == "__main__":
    try:
        from lindera import version
        print(f"Lindera version: {version()}")
        test_tokenize_returns_token_objects()
    except ImportError:
        print("Lindera package not installed or built correctly.")
    except Exception as e:
        print(f"Test failed: {e}")
        exit(1)
