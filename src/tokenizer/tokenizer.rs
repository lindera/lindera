use byteorder::ByteOrder;
use byteorder::LittleEndian;
use serde::Serialize;

use lindera_core::core::character_definition::CharacterDefinitions;
use lindera_core::core::connection::ConnectionCostMatrix;
use lindera_core::core::prefix_dict::PrefixDict;
use lindera_core::core::unknown_dictionary::UnknownDictionary;
use lindera_core::core::viterbi::{Lattice, Mode, Penalty};
use lindera_core::core::word_entry::{WordDetail, WordId};
use lindera_ipadic;
use lindera_dictionary;

pub fn word_detail(word_id: WordId, words_idx_data: &[u8], words_data: &[u8]) -> WordDetail {
    if word_id.is_unknown() {
        return WordDetail {
            pos_level1: "UNK".to_string(),
            pos_level2: "*".to_string(),
            pos_level3: "*".to_string(),
            pos_level4: "*".to_string(),
            conjugation_type: "*".to_string(),
            conjugate_form: "*".to_string(),
            base_form: "*".to_string(),
            reading: "*".to_string(),
            pronunciation: "*".to_string(),
        };
    }

    let idx = LittleEndian::read_u32(&words_idx_data[4 * word_id.0 as usize..][..4]);
    let data = &words_data[idx as usize..];
    let word_detail = bincode::deserialize_from(data).unwrap();
    word_detail
}

#[derive(Serialize)]
pub struct Token<'a> {
    pub text: &'a str,
    pub detail: WordDetail,
}

pub struct Tokenizer {
    dict: PrefixDict<Vec<u8>>,
    cost_matrix: ConnectionCostMatrix,
    lattice: Lattice,
    char_definitions: CharacterDefinitions,
    unknown_dictionary: UnknownDictionary,
    words_idx_data: Vec<u8>,
    words_data: Vec<u8>,
    mode: Mode,
    offsets: Vec<(usize, WordId)>,
}

impl Tokenizer {
    pub fn default(mode: Mode) -> Tokenizer {
        Tokenizer {
            dict: lindera_ipadic::prefix_dict(),
            cost_matrix: lindera_ipadic::connection(),
            lattice: Lattice::default(),
            char_definitions: lindera_ipadic::char_def(),
            unknown_dictionary: lindera_ipadic::unknown_dict(),
            words_idx_data: lindera_ipadic::words_idx_data(),
            words_data: lindera_ipadic::words_data(),
            mode,
            offsets: Vec::new(),
        }
    }

    pub fn external(dir: &str, mode: Mode) -> Tokenizer {
        Tokenizer {
            dict: lindera_dictionary::prefix_dict(dir),
            cost_matrix: lindera_dictionary::connection(dir),
            lattice: Lattice::default(),
            char_definitions: lindera_dictionary::char_def(dir),
            unknown_dictionary: lindera_dictionary::unknown_dict(dir),
            words_idx_data: lindera_dictionary::words_idx_data(dir),
            words_data: lindera_dictionary::words_data(dir),
            mode,
            offsets: Vec::new(),
        }
    }

    pub fn default_normal() -> Tokenizer {
        Self::default(Mode::Normal)
    }

    pub fn default_decompose() -> Tokenizer {
        Self::default(Mode::Decompose(Penalty::default()))
    }

    pub fn external_normal(dir: &str) -> Tokenizer {
        Self::external(dir, Mode::Normal)
    }

    pub fn external_decompose(dir: &str) -> Tokenizer {
        Self::external(dir, Mode::Decompose(Penalty::default()))
    }

    /// Returns an array of offsets that mark the beginning of each tokens,
    /// in bytes.
    ///
    /// For instance
    /// e.g. "僕は'
    ///
    /// returns the array `[0, 3]`
    ///
    /// The array, always starts with 0, except if you tokenize the empty string,
    /// in which case an empty array is returned.
    ///
    /// Whitespaces also count as tokens.
    pub(crate) fn tokenize_offsets(&mut self, text: &str) -> &[(usize, WordId)] {
        if text.is_empty() {
            return &[];
        }
        self.lattice.set_text(
            &self.dict,
            &self.char_definitions,
            &self.unknown_dictionary,
            text,
            &self.mode,
        );
        self.lattice
            .calculate_path_costs(&self.cost_matrix, &self.mode);
        self.lattice.tokens_offset(&mut self.offsets);
        &self.offsets[..]
    }

    fn tokenize_without_split<'a>(&mut self, text: &'a str, tokens: &mut Vec<Token<'a>>) {
        let words_idx_data = self.words_idx_data.clone();
        let words_data = self.words_data.clone();

        let offsets = self.tokenize_offsets(text);

        for i in 0..offsets.len() {
            let (token_start, word_id) = offsets[i];
            let token_stop = if i == offsets.len() - 1 {
                text.len()
            } else {
                let (next_start, _) = offsets[i + 1];
                next_start
            };
            tokens.push(Token {
                text: &text[token_start..token_stop],
                detail: word_detail(word_id, words_idx_data.as_slice(), words_data.as_slice()),
            })
        }
    }

    fn tokenize_without_split_str<'a>(&mut self, text: &'a str, tokens: &mut Vec<&'a str>) {
        let offsets = self.tokenize_offsets(text);
        for i in 0..offsets.len() {
            let (token_start, _word_detail) = offsets[i];
            let token_stop = if i == offsets.len() - 1 {
                text.len()
            } else {
                let (next_start, _) = offsets[i + 1];
                next_start
            };
            tokens.push(&text[token_start..token_stop]);
        }
    }

    pub fn tokenize<'a>(&'a mut self, mut text: &'a str) -> Vec<Token> {
        let mut tokens = Vec::new();
        while let Some(split_idx) = text.find(|c| c == '。' || c == '、') {
            self.tokenize_without_split(&text[..split_idx + 3], &mut tokens);
            text = &text[split_idx + 3..];
        }
        if !text.is_empty() {
            self.tokenize_without_split(&text, &mut tokens);
        }
        tokens
    }

    pub fn tokenize_str<'a>(&'a mut self, mut text: &'a str) -> Vec<&'a str> {
        let mut tokens = Vec::new();
        while let Some(split_idx) = text.find(|c| c == '。' || c == '、') {
            self.tokenize_without_split_str(&text[..split_idx + 3], &mut tokens);
            text = &text[split_idx + 3..];
        }
        if !text.is_empty() {
            self.tokenize_without_split_str(&text, &mut tokens);
        }
        tokens
    }
}

#[cfg(test)]
mod tests {
    use crate::tokenizer::tokenizer::Tokenizer;
    use lindera_core::core::word_entry::WordId;

    #[test]
    fn test_empty() {
        let mut tokenizer = Tokenizer::default_decompose();
        let tokens = tokenizer.tokenize_offsets("");
        assert_eq!(tokens, &[]);
    }

    #[test]
    fn test_space() {
        let mut tokenizer = Tokenizer::default_decompose();
        let tokens = tokenizer.tokenize_offsets(" ");
        assert_eq!(tokens, &[(0, WordId(4294967295))]);
    }

    #[test]
    fn test_boku_ha() {
        let mut tokenizer = Tokenizer::default_decompose();
        let tokens = tokenizer.tokenize_offsets("僕は");
        assert_eq!(tokens, &[(0, WordId(132630)), (3, WordId(57063))]);
    }
    /*
    #[test]
    fn test_tokenize() {
        let mut tokenizer = Tokenizer::for_search();
        let tokens = tokenizer.tokenize_offsets("俺はまだ本気出してないだけ。");
        assert_eq!(tokens, &[0, 3, 6, 12, 18, 24, 27, 33, 39]);
    }

    #[test]
    fn test_tokenize2() {
        let mut tokenizer = Tokenizer::for_search();
        let tokens: Vec<&str> = tokenizer.tokenize_str("私の名前はマズレル野恵美です。");
        assert_eq!(tokens, vec!["私", "の", "名前", "は", "マズレル", "野", "恵美", "です", "。"]);
    }

    #[test]
    fn test_tokenize_junk() {
        let mut tokenizer = Tokenizer::for_search();
        let tokens: Vec<&str> = tokenizer.tokenize_str("関西国werwerママママ空港");
        assert_eq!(tokens, vec!["関西", "国", "werwer", "ママ", "ママ", "空港"]);
    }

    #[test]
    fn test_tokenize_search_mode() {
        let mut tokenizer = Tokenizer::for_search();
        let tokens: Vec<&str> = tokenizer.tokenize_str("関西国際空港");
        assert_eq!(tokens, vec!["関西", "国際", "空港"]);
    }

    #[test]
    fn test_tokenize_sumomomomo() {
        let mut tokenizer = Tokenizer::for_search();
        let tokens: Vec<&str> = tokenizer.tokenize_str("すもももももももものうち");
        assert_eq!(tokens, vec!["すもも", "も", "もも", "も", "もも", "の", "うち"]);
    }

    #[test]
    fn test_mukigen_search() {
        let mut tokenizer = Tokenizer::for_search();
        let tokens: Vec<&str> = tokenizer.tokenize_str("無期限に—でもどの種を?");
        assert_eq!(tokens, vec!["無", "期限", "に", "—", "でも", "どの", "種", "を", "?"]);
    }
    */

    #[test]
    fn test_gyoi() {
        let mut tokenizer = Tokenizer::default_normal();
        let tokens: Vec<&str> = tokenizer.tokenize_str("御意。 御意〜。");
        assert_eq!(tokens, vec!["御意", "。", " ", "御意", "〜", "。"]);
    }

    #[test]
    fn test_demoyorokobi() {
        let mut tokenizer = Tokenizer::default_normal();
        let tokens: Vec<&str> = tokenizer.tokenize_str("〜でも喜び");
        assert_eq!(tokens, vec!["〜", "でも", "喜び"]);
    }

    #[test]
    fn test_mukigen_normal2() {
        let mut tokenizer = Tokenizer::default_normal();
        let tokens: Vec<&str> = tokenizer.tokenize_str("—でも");
        assert_eq!(tokens, vec!["—", "でも"]);
    }

    #[test]
    fn test_atodedenwa() {
        let mut tokenizer = Tokenizer::default_normal();
        let tokens: Vec<&str> = tokenizer.tokenize_str("後で");
        assert_eq!(tokens, vec!["後で"]);
    }

    #[test]
    fn test_ikkagetsu() {
        let mut tokenizer = Tokenizer::default_normal();
        let tokens: Vec<&str> = tokenizer.tokenize_str("ーヶ月");
        assert_eq!(tokens, vec!["ーヶ", "月"]);
    }

    #[test]
    fn test_mukigen_normal() {
        let mut tokenizer = Tokenizer::default_normal();
        let tokens: Vec<&str> = tokenizer.tokenize_str("無期限に—でもどの種を?");
        assert_eq!(
            tokens,
            vec!["無", "期限", "に", "—", "でも", "どの", "種", "を", "?"]
        );
    }

    #[test]
    fn test_demo() {
        let mut tokenizer = Tokenizer::default_normal();
        let tokens: Vec<&str> = tokenizer.tokenize_str("――!!?");
        assert_eq!(tokens, vec!["――!!?"]);
    }

    #[test]
    fn test_kaikeishi() {
        let mut tokenizer = Tokenizer::default_normal();
        let tokens: Vec<&str> = tokenizer.tokenize_str("ジム・コガン");
        assert_eq!(tokens, vec!["ジム・コガン"]);
    }

    #[test]
    fn test_bruce() {
        let mut tokenizer = Tokenizer::default_normal();
        let tokens: Vec<&str> = tokenizer.tokenize_str("ブルース・モラン");
        assert_eq!(tokens, vec!["ブルース・モラン"]);
    }

    #[test]
    fn test_tokenize_real() {
        let mut tokenizer = Tokenizer::default_normal();
        let tokens: Vec<&str> = tokenizer.tokenize_str(
            "本項で解説する地方病とは、山梨県における日本住血吸虫症の呼称であり、\
             長い間その原因が明らかにならず住民を苦しめた感染症である。",
        );
        assert_eq!(
            tokens,
            vec![
                "本",
                "項",
                "で",
                "解説",
                "する",
                "地方",
                "病",
                "と",
                "は",
                "、",
                "山梨",
                "県",
                "における",
                "日本",
                "住",
                "血",
                "吸",
                "虫",
                "症",
                "の",
                "呼称",
                "で",
                "あり",
                "、",
                "長い",
                "間",
                "その",
                "原因",
                "が",
                "明らか",
                "に",
                "なら",
                "ず",
                "住民",
                "を",
                "苦しめ",
                "た",
                "感染",
                "症",
                "で",
                "ある",
                "。"
            ]
        );
    }

    #[test]
    fn test_hitobito() {
        let mut tokenizer = Tokenizer::default_normal();
        let tokens: Vec<&str> = tokenizer.tokenize_str("満々!");
        assert_eq!(tokens, &["満々", "!"]);
    }

    #[test]
    fn test_tokenize_short() {
        let mut tokenizer = Tokenizer::default_normal();
        let tokens: Vec<&str> = tokenizer
            .tokenize("日本住")
            .into_iter()
            .map(|token| token.text)
            .collect();
        assert_eq!(tokens, vec!["日本", "住"]);
    }

    #[test]
    fn test_tokenize_short2() {
        let mut tokenizer = Tokenizer::default_normal();
        let tokens: Vec<&str> = tokenizer
            .tokenize("ここでは")
            .into_iter()
            .map(|token| token.text)
            .collect();
        assert_eq!(tokens, vec!["ここ", "で", "は"]);
    }
}
