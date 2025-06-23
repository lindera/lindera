#[cfg(feature = "unidic")]
use std::env;
#[cfg(feature = "compress")]
use std::ops::Deref;

#[cfg(feature = "compress")]
use lindera_dictionary::decompress::decompress;
use lindera_dictionary::dictionary::character_definition::CharacterDefinition;
use lindera_dictionary::dictionary::connection_cost_matrix::ConnectionCostMatrix;
use lindera_dictionary::dictionary::prefix_dictionary::PrefixDictionary;
use lindera_dictionary::dictionary::unknown_dictionary::UnknownDictionary;
use lindera_dictionary::dictionary::Dictionary;
use lindera_dictionary::LinderaResult;

macro_rules! decompress_data {
    ($name: ident, $bytes: expr, $filename: literal) => {
        #[cfg(feature = "compress")]
        static $name: once_cell::sync::Lazy<Vec<u8>> = once_cell::sync::Lazy::new(|| {
            let (compressed_data, _) =
                bincode::serde::decode_from_slice(&$bytes[..], bincode::config::legacy())
                    .expect(concat!("invalid file format ", $filename));
            decompress(compressed_data).expect(concat!("invalid file format ", $filename))
        });
        #[cfg(not(feature = "compress"))]
        const $name: &'static [u8] = $bytes;
    };
}

macro_rules! unidic_data {
    ($name: ident, $path: literal, $filename: literal) => {
        #[cfg(feature = "unidic")]
        decompress_data!(
            $name,
            include_bytes!(concat!(env!("LINDERA_WORKDIR"), $path)),
            $filename
        );
        #[cfg(not(feature = "unidic"))]
        decompress_data!($name, &[], $filename);
    };
}

unidic_data!(
    CHAR_DEFINITION_DATA,
    "/lindera-unidic/char_def.bin",
    "char_def.bin"
);
unidic_data!(CONNECTION_DATA, "/lindera-unidic/matrix.mtx", "matrix.mtx");
unidic_data!(DA_DATA, "/lindera-unidic/dict.da", "dict.da");
unidic_data!(VALS_DATA, "/lindera-unidic/dict.vals", "dict.vals");
unidic_data!(UNKNOWN_DATA, "/lindera-unidic/unk.bin", "unk.bin");
unidic_data!(
    WORDS_IDX_DATA,
    "/lindera-unidic/dict.wordsidx",
    "dict.wordsidx"
);
unidic_data!(WORDS_DATA, "/lindera-unidic/dict.words", "dict.words");

pub fn load() -> LinderaResult<Dictionary> {
    #[cfg(feature = "compress")]
    {
        Ok(Dictionary {
            prefix_dictionary: PrefixDictionary::load(
                DA_DATA.deref(),
                VALS_DATA.deref(),
                WORDS_IDX_DATA.deref(),
                WORDS_DATA.deref(),
                true,
            ),
            connection_cost_matrix: ConnectionCostMatrix::load(CONNECTION_DATA.deref()),
            character_definition: CharacterDefinition::load(&CHAR_DEFINITION_DATA)?,
            unknown_dictionary: UnknownDictionary::load(&UNKNOWN_DATA)?,
        })
    }
    #[cfg(not(feature = "compress"))]
    {
        Ok(Dictionary {
            prefix_dictionary: PrefixDictionary::load(
                DA_DATA,
                VALS_DATA,
                WORDS_IDX_DATA,
                WORDS_DATA,
                true,
            ),
            connection_cost_matrix: ConnectionCostMatrix::load(CONNECTION_DATA),
            character_definition: CharacterDefinition::load(CHAR_DEFINITION_DATA)?,
            unknown_dictionary: UnknownDictionary::load(UNKNOWN_DATA)?,
        })
    }
}
