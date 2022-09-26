#[cfg(any(
    feature = "ipadic",
    feature = "unidic",
    feature = "ko-dic",
    feature = "cc-cedict"
))]
use lindera::tokenizer::Tokenizer;
use lindera::{
    mode::Mode,
    tokenizer::{DictionaryConfig, TokenizerConfig},
    DictionaryKind, LinderaResult,
};

fn main() -> LinderaResult<()> {
    #[cfg(feature = "ipadic")]
    {
        let dic_config = DictionaryConfig {
            kind: DictionaryKind::IPADIC,
            path: None,
        };

        let config = TokenizerConfig {
            dictionary: dic_config,
            user_dictionary: None,
            mode: Mode::Normal,
        };

        // create tokenizer
        let tokenizer = Tokenizer::with_config(config)?;

        // tokenize the text
        let tokens = tokenizer.tokenize("日本語の形態素解析を行うことができます。")?;

        // output the tokens
        for token in tokens {
            println!("{}", token.text);
        }
    }

    #[cfg(feature = "unidic")]
    {
        let dic_config = DictionaryConfig {
            kind: DictionaryKind::UniDic,
            path: None,
        };

        let config = TokenizerConfig {
            dictionary: dic_config,
            user_dictionary: None,
            mode: Mode::Normal,
        };

        // create tokenizer
        let tokenizer = Tokenizer::with_config(config)?;

        // tokenize the text
        let tokens = tokenizer.tokenize("日本語の形態素解析を行うことができます。")?;

        // output the tokens
        for token in tokens {
            println!("{}", token.text);
        }
    }

    #[cfg(feature = "ko-dic")]
    {
        let dic_config = DictionaryConfig {
            kind: DictionaryKind::KoDic,
            path: None,
        };

        let config = TokenizerConfig {
            dictionary: dic_config,
            user_dictionary: None,
            mode: Mode::Normal,
        };

        // create tokenizer
        let tokenizer = Tokenizer::with_config(config)?;

        let tokens = tokenizer.tokenize("한국어의형태해석을실시할수있습니다.")?;

        // output the tokens
        for token in tokens {
            println!("{}", token.text);
        }
    }

    #[cfg(feature = "cc-cedict")]
    {
        let dic_config = DictionaryConfig {
            kind: DictionaryKind::CcCedict,
            path: None,
        };

        let config = TokenizerConfig {
            dictionary: dic_config,
            user_dictionary: None,
            mode: Mode::Normal,
        };

        // create tokenizer
        let tokenizer = Tokenizer::with_config(config)?;

        #[cfg(feature = "cc-cedict")]
        let tokens = tokenizer.tokenize("可以进行中文形态学分析。")?;

        // output the tokens
        for token in tokens {
            println!("{}", token.text);
        }
    }

    Ok(())
}
