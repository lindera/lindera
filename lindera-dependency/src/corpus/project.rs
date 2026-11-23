//! Projecting the bunsetsu of a corpus onto the morphemes of a dictionary.
//!
//! The corpus has its own words; the parser runs on the morphemes of the
//! dictionary it is used with. The text of the sentence is tokenized again
//! with that dictionary, and the bunsetsu are laid over the morphemes by
//! character offsets, as `tools/kc2cabocha.pl` of CaboCha does for the
//! Kyoto corpus.

use super::DropCause;
use super::conllu::UdSentence;
use super::convert::{BunsetsuTree, drop_causes};
use crate::morpheme::Morpheme;
use crate::sentence::{Bunsetsu, Sentence};

/// A sentence of the corpus on the morphemes of a dictionary.
#[derive(Debug, Clone, PartialEq)]
pub struct Projection<M> {
    /// The sentence: the morphemes, and the bunsetsu over them.
    pub sentence: Sentence<M>,
    /// The number of bunsetsu boundaries inside the sentence: one less than
    /// its bunsetsu in the corpus.
    pub boundaries: usize,
    /// The number of those boundaries that are also morpheme boundaries.
    pub boundaries_fit: usize,
    /// The number of bunsetsu lost by merging: the boundaries that are not
    /// morpheme boundaries.
    pub merged: usize,
    /// For each bunsetsu of the sentence, the morpheme that holds the
    /// first character of its content head in the corpus, counted from the
    /// first morpheme of the bunsetsu; `None` for a bunsetsu whose content
    /// head the corpus does not give. What the head word of a schema is
    /// measured against.
    pub content_heads: Vec<Option<usize>>,
}

/// Lays the bunsetsu of a sentence over morphemes.
///
/// Bunsetsu and morphemes are aligned by character offsets, white space
/// left out. Where a bunsetsu boundary falls inside a morpheme, the bunsetsu
/// are merged up to the next boundary that is a morpheme boundary; the
/// merged bunsetsu takes the head, the arc type and the content head of the
/// last one merged, and the heads are renumbered.
///
/// # Arguments
///
/// * `sentence` - The sentence of the corpus.
/// * `tree` - Its bunsetsu, from [`to_bunsetsu`](super::to_bunsetsu).
/// * `morphemes` - The morphemes of the text of the sentence.
///
/// # Returns
///
/// The projection, or the causes for which the sentence is dropped:
/// [`DropCause::Text`] when the morphemes do not spell the words of the
/// sentence, and otherwise every way the merged bunsetsu are not a tree.
pub fn project<M: Morpheme>(
    sentence: &UdSentence,
    tree: &BunsetsuTree,
    morphemes: Vec<M>,
) -> Result<Projection<M>, Vec<DropCause>> {
    // The end of each bunsetsu and of each morpheme and the start of each
    // word, in characters that are not white space, and the characters
    // themselves to compare.
    let mut corpus_text = String::new();
    let mut word_starts = Vec::with_capacity(sentence.words.len());
    let mut bunsetsu_ends = Vec::with_capacity(tree.bunsetsu.len());
    let mut offset = 0;
    for bunsetsu in &tree.bunsetsu {
        for word in &sentence.words[bunsetsu.words.clone()] {
            word_starts.push(offset);
            offset += push_without_white_space(&mut corpus_text, &word.form);
        }
        bunsetsu_ends.push(offset);
    }
    let mut morpheme_text = String::with_capacity(corpus_text.len());
    let mut morpheme_ends = Vec::with_capacity(morphemes.len());
    let mut offset = 0;
    for morpheme in &morphemes {
        offset += push_without_white_space(&mut morpheme_text, morpheme.surface());
        morpheme_ends.push(offset);
    }
    if corpus_text != morpheme_text || corpus_text.is_empty() {
        return Err(vec![DropCause::Text]);
    }

    // The new bunsetsu: one ends wherever a bunsetsu of the corpus ends on a
    // morpheme boundary. `sources` holds, for each, the last bunsetsu of the
    // corpus in it, which gives its head.
    let mut sources = Vec::with_capacity(bunsetsu_ends.len());
    let mut bunsetsu = Vec::with_capacity(bunsetsu_ends.len());
    let mut start = 0;
    let mut next = 0;
    let mut previous_end = 0;
    for (index, &end) in bunsetsu_ends.iter().enumerate() {
        if end == previous_end {
            // A bunsetsu of white space only.
            return Err(vec![DropCause::Text]);
        }
        previous_end = end;
        while morpheme_ends[next] < end {
            next += 1;
        }
        if morpheme_ends[next] != end {
            continue;
        }
        next += 1;
        // Morphemes of white space only stay with the bunsetsu before them.
        while next < morphemes.len() && morpheme_ends[next] == end {
            next += 1;
        }
        sources.push(index);
        bunsetsu.push(start..next);
        start = next;
    }

    let boundaries = bunsetsu_ends.len() - 1;
    let merged = bunsetsu_ends.len() - sources.len();
    let heads: Vec<Option<usize>> = sources
        .iter()
        .map(|&source| {
            // The new bunsetsu that holds the head: the first one whose last
            // bunsetsu of the corpus is the head or after it.
            tree.bunsetsu[source]
                .head
                .map(|head| sources.partition_point(|&source| source < head))
        })
        .collect();
    let causes = drop_causes(&heads);
    if !causes.is_empty() {
        return Err(causes);
    }

    // The morpheme that holds the first character of the content head word
    // of each bunsetsu: the first one that ends after that character.
    let content_heads = bunsetsu
        .iter()
        .zip(&sources)
        .map(|(morphemes, &source)| {
            let word = tree.bunsetsu[source].content_head?;
            let start = *word_starts.get(word)?;
            let morpheme = morpheme_ends.partition_point(|&end| end <= start);
            morphemes
                .contains(&morpheme)
                .then(|| morpheme - morphemes.start)
        })
        .collect();

    let bunsetsu = bunsetsu
        .into_iter()
        .zip(heads)
        .zip(&sources)
        .map(|((morphemes, head), &source)| {
            let mut bunsetsu = Bunsetsu::new(morphemes, head);
            bunsetsu.arc_type = tree.bunsetsu[source].arc_type;
            bunsetsu
        })
        .collect();
    // The bunsetsu cover the morphemes and the heads are in range by
    // construction; a failure here would be a defect of this function.
    let sentence = Sentence::new(morphemes, bunsetsu).map_err(|_| vec![DropCause::Text])?;
    Ok(Projection {
        sentence,
        boundaries,
        boundaries_fit: boundaries - merged,
        merged,
        content_heads,
    })
}

/// Appends a string without its white space.
///
/// # Arguments
///
/// * `target` - The string to append to.
/// * `text` - The string to append.
///
/// # Returns
///
/// The number of characters appended.
fn push_without_white_space(target: &mut String, text: &str) -> usize {
    let mut count = 0;
    for character in text.chars().filter(|character| !character.is_whitespace()) {
        target.push(character);
        count += 1;
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus::tests::{morphemes, ud_sentence, with_content_heads};
    use crate::corpus::{DictionaryMorpheme, to_bunsetsu};

    /// Projects a sentence onto morphemes with the given surfaces.
    fn projected(
        words: &[(&str, usize, &str, bool)],
        surfaces: &[&str],
    ) -> Result<Projection<DictionaryMorpheme>, Vec<DropCause>> {
        let sentence = ud_sentence(words);
        let tree = to_bunsetsu(&sentence).unwrap();
        project(&sentence, &tree, morphemes(surfaces))
    }

    /// The bunsetsu of a projection, as `(surface, head, letter)`.
    fn bunsetsu(projection: &Projection<DictionaryMorpheme>) -> Vec<(String, Option<usize>, char)> {
        let sentence = &projection.sentence;
        sentence
            .bunsetsu
            .iter()
            .map(|bunsetsu| {
                let surface = sentence.morphemes[bunsetsu.morphemes.clone()]
                    .iter()
                    .map(|morpheme| morpheme.surface())
                    .collect();
                (surface, bunsetsu.head, bunsetsu.arc_type.as_char())
            })
            .collect()
    }

    #[test]
    fn lays_bunsetsu_over_morphemes_that_differ_from_the_words() {
        // The words 読ん だ are one morpheme, 本 を are 本 を: every boundary
        // of the bunsetsu is a morpheme boundary.
        let projection = projected(
            &[
                ("彼", 5, "nsubj", true),
                ("は", 1, "case", false),
                ("本", 5, "obj", true),
                ("を", 3, "case", false),
                ("読ん", 0, "root", true),
                ("だ", 5, "aux", false),
            ],
            &["彼", "は", "本", "を", "読んだ"],
        )
        .unwrap();
        assert_eq!(
            bunsetsu(&projection),
            [
                ("彼は".to_string(), Some(2), 'D'),
                ("本を".to_string(), Some(2), 'D'),
                ("読んだ".to_string(), None, 'D'),
            ]
        );
        assert_eq!(
            (
                projection.boundaries,
                projection.boundaries_fit,
                projection.merged
            ),
            (2, 2, 0)
        );
        assert!(projection.sentence.violations().is_empty());
    }

    /// The content heads of a sentence projected onto morphemes with the
    /// given surfaces, with the given words marked.
    fn content_heads(
        words: &[(&str, usize, &str, bool)],
        marked: &[usize],
        surfaces: &[&str],
    ) -> Vec<Option<usize>> {
        let sentence = with_content_heads(ud_sentence(words), marked);
        let tree = to_bunsetsu(&sentence).unwrap();
        project(&sentence, &tree, morphemes(surfaces))
            .unwrap()
            .content_heads
    }

    #[test]
    fn the_content_head_is_the_morpheme_with_the_first_character_of_the_word() {
        // お 茶 を | 東京都 で | 飲む, with 茶, 東京都 and 飲む marked.
        let words = [
            ("お", 2, "compound", true),
            ("茶", 6, "obj", false),
            ("を", 2, "case", false),
            ("東京都", 6, "obl", true),
            ("で", 4, "case", false),
            ("飲む", 0, "root", true),
        ];
        let marked = [1, 3, 5];
        // The same morphemes as the words: the position in the bunsetsu.
        assert_eq!(
            content_heads(&words, &marked, &["お", "茶", "を", "東京都", "で", "飲む"]),
            [Some(1), Some(0), Some(0)]
        );
        // お茶 is one morpheme, which holds 茶; 東京都 is two, and the
        // first holds its first character; 飲む is two too.
        assert_eq!(
            content_heads(
                &words,
                &marked,
                &["お茶", "を", "東京", "都", "で", "飲", "む"]
            ),
            [Some(0), Some(0), Some(0)]
        );
        // The word is the second of its bunsetsu and starts inside a
        // morpheme: を of お 茶 を, with 茶を as one morpheme.
        assert_eq!(
            content_heads(&words, &[2, 4, 5], &["お", "茶を", "東京都", "で", "飲む"]),
            [Some(1), Some(1), Some(0)]
        );
        // A bunsetsu without a content head in the corpus has none.
        assert_eq!(
            content_heads(&words, &[1], &["お", "茶", "を", "東京都", "で", "飲む"]),
            [Some(1), None, None]
        );
    }

    #[test]
    fn a_merged_bunsetsu_takes_the_content_head_of_the_last_one_merged() {
        // a | b c | 読む, with ab as one morpheme: the first two bunsetsu
        // are merged. The content head of the first is a, in the morpheme
        // ab; the one of the second is c, the second morpheme.
        let words = [
            ("a", 4, "dep", true),
            ("b", 3, "compound", true),
            ("c", 4, "dep", false),
            ("読む", 0, "root", true),
        ];
        assert_eq!(
            content_heads(&words, &[0, 2, 3], &["ab", "c", "読む"]),
            [Some(1), Some(0)]
        );
        // The last one merged has none: the merged bunsetsu has none,
        // whatever the first has.
        assert_eq!(
            content_heads(&words, &[0, 3], &["ab", "c", "読む"]),
            [None, Some(0)]
        );
    }

    #[test]
    fn white_space_does_not_move_the_content_head() {
        // a b | 読む, with a morpheme of white space between a and b.
        let words = [
            ("a", 2, "compound", true),
            ("b", 3, "dep", false),
            ("読む", 0, "root", true),
        ];
        assert_eq!(
            content_heads(&words, &[1, 2], &["a", " ", "b", "読む"]),
            [Some(2), Some(0)]
        );
    }

    #[test]
    fn merges_bunsetsu_when_a_boundary_is_inside_a_morpheme() {
        // その | 後 | 読む, with その後 as one morpheme. The first bunsetsu
        // depends on the third and the second on the third: the merged one
        // takes the head of the last one merged.
        let projection = projected(
            &[
                ("その", 4, "det", true),
                ("後", 3, "obl", true),
                ("本", 4, "obj", true),
                ("読む", 0, "root", true),
            ],
            &["その後", "本", "読む"],
        )
        .unwrap();
        assert_eq!(
            bunsetsu(&projection),
            [
                ("その後".to_string(), Some(1), 'D'),
                ("本".to_string(), Some(2), 'D'),
                ("読む".to_string(), None, 'D'),
            ]
        );
        assert_eq!(
            (
                projection.boundaries,
                projection.boundaries_fit,
                projection.merged
            ),
            (3, 2, 1)
        );
    }

    #[test]
    fn a_merged_bunsetsu_takes_the_arc_type_of_the_last_one_merged() {
        // a | b | c | 読む: a and b are conjuncts (a → b is `P`, b → 読む is
        // `D`), c → 読む. The morpheme `ab` merges a and b.
        let projection = projected(
            &[
                ("a", 4, "obj", true),
                ("b", 1, "conj", true),
                ("c", 4, "obl", true),
                ("読む", 0, "root", true),
            ],
            &["ab", "c", "読む"],
        )
        .unwrap();
        assert_eq!(
            bunsetsu(&projection),
            [
                ("ab".to_string(), Some(2), 'D'),
                ("c".to_string(), Some(2), 'D'),
                ("読む".to_string(), None, 'D'),
            ]
        );
    }

    #[test]
    fn renumbers_heads_after_a_merge() {
        // a | b | c | d | 読む with a → d, b → c, c → d, d → 読む, and the
        // morpheme `bc`: d becomes bunsetsu 2 and 読む bunsetsu 3. A head
        // inside the merged bunsetsu (none here) would become that one.
        let projection = projected(
            &[
                ("a", 4, "dep", true),
                ("b", 3, "dep", true),
                ("c", 4, "dep", true),
                ("d", 5, "dep", true),
                ("読む", 0, "root", true),
            ],
            &["a", "bc", "d", "読む"],
        )
        .unwrap();
        assert_eq!(
            bunsetsu(&projection),
            [
                ("a".to_string(), Some(2), 'D'),
                ("bc".to_string(), Some(2), 'D'),
                ("d".to_string(), Some(3), 'D'),
                ("読む".to_string(), None, 'D'),
            ]
        );
    }

    #[test]
    fn a_head_inside_a_merged_bunsetsu_becomes_the_merged_one() {
        // a | b | c | 読む with a → b, b → 読む, c → 読む and the morpheme
        // `bc`: a depends on the merged bunsetsu.
        let projection = projected(
            &[
                ("a", 2, "dep", true),
                ("b", 4, "dep", true),
                ("c", 4, "dep", true),
                ("読む", 0, "root", true),
            ],
            &["a", "bc", "読む"],
        )
        .unwrap();
        assert_eq!(
            bunsetsu(&projection),
            [
                ("a".to_string(), Some(1), 'D'),
                ("bc".to_string(), Some(2), 'D'),
                ("読む".to_string(), None, 'D'),
            ]
        );
    }

    #[test]
    fn merges_more_than_two_bunsetsu_and_up_to_the_end() {
        // One morpheme over the three bunsetsu.
        let projection = projected(
            &[
                ("a", 2, "dep", true),
                ("b", 3, "dep", true),
                ("c", 0, "root", true),
            ],
            &["abc"],
        )
        .unwrap();
        assert_eq!(bunsetsu(&projection), [("abc".to_string(), None, 'D')]);
        assert_eq!(projection.merged, 2);
    }

    #[test]
    fn leaves_white_space_out_of_the_alignment() {
        // The form `New York` holds a space, and the tokenizer may or may
        // not give a morpheme for a space.
        let words = [("New York", 2, "obl", true), ("行く", 0, "root", true)];
        let projection = projected(&words, &["New", "York", "行く"]).unwrap();
        assert_eq!(
            bunsetsu(&projection),
            [
                ("NewYork".to_string(), Some(1), 'D'),
                ("行く".to_string(), None, 'D'),
            ]
        );
        let projection = projected(&words, &["New", " ", "York", "　", "行く", " "]).unwrap();
        assert_eq!(
            bunsetsu(&projection),
            [
                ("New York　".to_string(), Some(1), 'D'),
                ("行く ".to_string(), None, 'D'),
            ]
        );
    }

    #[test]
    fn drops_a_sentence_whose_morphemes_do_not_spell_its_words() {
        let words = [("彼", 2, "nsubj", true), ("読む", 0, "root", true)];
        assert_eq!(
            projected(&words, &["彼", "読ん"]),
            Err(vec![DropCause::Text])
        );
        assert_eq!(projected(&words, &["彼"]), Err(vec![DropCause::Text]));
        assert_eq!(
            projected(&words, &["彼", "読む", "。"]),
            Err(vec![DropCause::Text])
        );
        assert_eq!(projected(&words, &[]), Err(vec![DropCause::Text]));
    }

    #[test]
    fn drops_a_sentence_with_a_bunsetsu_of_white_space() {
        let words = [
            ("彼", 3, "nsubj", true),
            (" ", 3, "dep", true),
            ("読む", 0, "root", true),
        ];
        assert_eq!(
            projected(&words, &["彼", "読む"]),
            Err(vec![DropCause::Text])
        );
    }

    #[test]
    fn drops_a_sentence_that_is_no_tree_after_merging() {
        // a | b | c | d | e with a → c, b → c, c → e, d → e. The morpheme
        // `cd` merges c and d; the merged bunsetsu takes the head of d, and
        // the tree holds. With the morpheme `de` instead, d and the root
        // merge, and every arc still ends on it.
        let words = [
            ("a", 3, "dep", true),
            ("b", 3, "dep", true),
            ("c", 5, "dep", true),
            ("d", 5, "dep", true),
            ("e", 0, "root", true),
        ];
        assert!(projected(&words, &["a", "b", "cd", "e"]).is_ok());
        assert!(projected(&words, &["a", "b", "c", "de"]).is_ok());

        // A tree given to `project` that is none, as a caller could build:
        // the check after merging reports it.
        let sentence = ud_sentence(&words);
        let mut tree = to_bunsetsu(&sentence).unwrap();
        tree.bunsetsu[1].head = Some(3);
        tree.bunsetsu[0].head = Some(2);
        assert_eq!(
            project(&sentence, &tree, morphemes(&["a", "b", "c", "d", "e"])),
            Err(vec![DropCause::CrossingArcs])
        );
    }
}
