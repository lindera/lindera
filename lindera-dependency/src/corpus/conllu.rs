//! Reading CoNLL-U, as far as the conversion needs it.
//!
//! A sentence is its comment lines and its words; of the ten columns of a
//! word, the form, the head, the relation and, from the last column, the
//! label that says whether the word starts a bunsetsu.

use std::io::BufRead;

use crate::error::{Error, Result};

/// The key of the bunsetsu label in the MISC column of UD Japanese.
const BUNSETSU_LABEL: &str = "BunsetuBILabel";

/// A word of a CoNLL-U sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdWord {
    /// The form (column 2).
    pub form: String,
    /// The head (column 7): the number of another word of the sentence, from
    /// 1, or 0 for the root.
    pub head: usize,
    /// The relation to the head (column 8), such as `conj`.
    pub deprel: String,
    /// Whether the word starts a bunsetsu: `BunsetuBILabel=B` in the MISC
    /// column (column 10).
    pub starts_bunsetsu: bool,
}

/// A sentence of a CoNLL-U file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdSentence {
    /// The number of the first line of the sentence in its file, from 1.
    pub line: usize,
    /// The comment lines of the sentence, each with its leading `#`.
    pub comments: Vec<String>,
    /// The words of the sentence.
    pub words: Vec<UdWord>,
}

impl UdSentence {
    /// The text of the sentence, from its `# text = ...` comment.
    ///
    /// # Returns
    ///
    /// The text, or `None` when the sentence has no such comment.
    pub fn text(&self) -> Option<&str> {
        self.comments.iter().find_map(|comment| {
            let rest = comment.strip_prefix('#')?.trim_start();
            let rest = rest.strip_prefix("text")?.trim_start();
            Some(rest.strip_prefix('=')?.trim())
        })
    }
}

/// Reads the sentences of a CoNLL-U file one by one.
///
/// Lines of multiword tokens (`1-2`) and of empty nodes (`1.1`) are skipped:
/// the conversion works on the words. A word without the bunsetsu label is
/// an error, as such a file cannot be converted.
pub struct ConlluReader<R> {
    /// The input.
    reader: R,
    /// The number of lines read so far.
    line: usize,
    /// Whether the input has ended, or an error was returned.
    done: bool,
}

impl<R: BufRead> ConlluReader<R> {
    /// Makes a reader.
    ///
    /// # Arguments
    ///
    /// * `reader` - The input, in CoNLL-U.
    ///
    /// # Returns
    ///
    /// The reader.
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            line: 0,
            done: false,
        }
    }

    /// Reads the next sentence.
    ///
    /// # Returns
    ///
    /// The sentence, `None` at the end of the input, or the error of the
    /// first line that cannot be read.
    fn read_sentence(&mut self) -> Result<Option<UdSentence>> {
        let mut comments = Vec::new();
        let mut words: Vec<UdWord> = Vec::new();
        let mut first_line = 0;
        let mut buffer = String::new();
        loop {
            buffer.clear();
            let read = self.reader.read_line(&mut buffer)?;
            if read > 0 {
                self.line += 1;
            }
            let line = buffer.trim_end_matches(['\n', '\r']);
            if read == 0 || line.is_empty() {
                // A blank line, or the end of the input, ends a sentence.
                if !words.is_empty() {
                    check_heads(&words, first_line)?;
                    return Ok(Some(UdSentence {
                        line: first_line,
                        comments,
                        words,
                    }));
                }
                if read == 0 {
                    return Ok(None);
                }
                // Comments followed by a blank line belong to no sentence.
                comments.clear();
                first_line = 0;
                continue;
            }
            if first_line == 0 {
                first_line = self.line;
            }
            if line.starts_with('#') {
                comments.push(line.to_string());
                continue;
            }
            if let Some(word) = parse_word(line, self.line, words.len() + 1)? {
                words.push(word);
            }
        }
    }
}

impl<R: BufRead> Iterator for ConlluReader<R> {
    type Item = Result<UdSentence>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        match self.read_sentence() {
            Ok(Some(sentence)) => Some(Ok(sentence)),
            Ok(None) => {
                self.done = true;
                None
            }
            Err(error) => {
                self.done = true;
                Some(Err(error))
            }
        }
    }
}

/// Reads every sentence of a CoNLL-U text.
///
/// # Arguments
///
/// * `input` - The text, in CoNLL-U.
///
/// # Returns
///
/// The sentences, or the error of the first line that cannot be read.
pub fn parse(input: &str) -> Result<Vec<UdSentence>> {
    ConlluReader::new(input.as_bytes()).collect()
}

/// Reads the line of a word.
///
/// # Arguments
///
/// * `line` - The line, without its line break.
/// * `number` - The number of the line, from 1.
/// * `expected_id` - The ID the word must have: one more than the words of
///   the sentence read so far.
///
/// # Returns
///
/// The word, `None` for the line of a multiword token or of an empty node,
/// or [`Error::Conllu`].
fn parse_word(line: &str, number: usize, expected_id: usize) -> Result<Option<UdWord>> {
    let columns: Vec<&str> = line.split('\t').collect();
    if columns.len() != 10 {
        return Err(Error::conllu(
            number,
            format!("expected 10 columns, found {}", columns.len()),
        ));
    }
    let id = columns[0];
    if id.contains('-') || id.contains('.') {
        return Ok(None);
    }
    if id.parse::<usize>().ok() != Some(expected_id) {
        return Err(Error::conllu(
            number,
            format!("expected the word ID {expected_id}, found `{id}`"),
        ));
    }
    let head = columns[6]
        .parse::<usize>()
        .map_err(|_| Error::conllu(number, format!("the head `{}` is not a number", columns[6])))?;
    let label = columns[9]
        .split('|')
        .find_map(|pair| pair.strip_prefix(BUNSETSU_LABEL)?.strip_prefix('='));
    let starts_bunsetsu = match label {
        Some("B") => true,
        Some("I") => false,
        Some(other) => {
            return Err(Error::conllu(
                number,
                format!("{BUNSETSU_LABEL} is `{other}`, not `B` or `I`"),
            ));
        }
        None => {
            return Err(Error::conllu(
                number,
                format!("no {BUNSETSU_LABEL} in the last column"),
            ));
        }
    };
    Ok(Some(UdWord {
        form: columns[1].to_string(),
        head,
        deprel: columns[7].to_string(),
        starts_bunsetsu,
    }))
}

/// Checks that the head of every word is the root or another word of the
/// sentence.
///
/// # Arguments
///
/// * `words` - The words of the sentence.
/// * `first_line` - The number of the first line of the sentence, from 1.
///
/// # Returns
///
/// `Ok(())`, or [`Error::Conllu`] with the first line of the sentence.
fn check_heads(words: &[UdWord], first_line: usize) -> Result<()> {
    for (index, word) in words.iter().enumerate() {
        if word.head > words.len() || word.head == index + 1 {
            return Err(Error::conllu(
                first_line,
                format!(
                    "the head of the word {} is {}, in a sentence of {} words",
                    index + 1,
                    word.head,
                    words.len()
                ),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A word line with the given ID, form, head, relation and MISC column.
    fn line(id: &str, form: &str, head: &str, deprel: &str, misc: &str) -> String {
        format!("{id}\t{form}\t_\t_\t_\t_\t{head}\t{deprel}\t_\t{misc}\n")
    }

    /// The error of a text that cannot be read, as a string.
    fn error(input: &str) -> String {
        parse(input).unwrap_err().to_string()
    }

    #[test]
    fn reads_sentences_with_their_comments_and_words() {
        let input = format!(
            "# sent_id = a\n# text = 彼は 読む\n{}{}{}\n# sent_id = b\n{}",
            line("1", "彼", "3", "nsubj", "BunsetuBILabel=B|SpaceAfter=No"),
            line("2", "は", "1", "case", "SpaceAfter=Yes|BunsetuBILabel=I"),
            line("3", "読む", "0", "root", "BunsetuBILabel=B"),
            line("1", "はい", "0", "root", "BunsetuBILabel=B"),
        );
        let sentences = parse(&input).unwrap();
        assert_eq!(sentences.len(), 2);

        let first = &sentences[0];
        assert_eq!(first.line, 1);
        assert_eq!(first.comments, ["# sent_id = a", "# text = 彼は 読む"]);
        assert_eq!(first.text(), Some("彼は 読む"));
        assert_eq!(
            first.words,
            [
                UdWord {
                    form: "彼".to_string(),
                    head: 3,
                    deprel: "nsubj".to_string(),
                    starts_bunsetsu: true,
                },
                UdWord {
                    form: "は".to_string(),
                    head: 1,
                    deprel: "case".to_string(),
                    starts_bunsetsu: false,
                },
                UdWord {
                    form: "読む".to_string(),
                    head: 0,
                    deprel: "root".to_string(),
                    starts_bunsetsu: true,
                },
            ]
        );

        let second = &sentences[1];
        assert_eq!(second.line, 7);
        assert_eq!(second.text(), None);
        assert_eq!(second.words.len(), 1);
    }

    #[test]
    fn reads_a_last_sentence_without_a_blank_line_and_crlf() {
        let input = line("1", "はい", "0", "root", "BunsetuBILabel=B").replace('\n', "\r\n");
        let sentences = parse(&input).unwrap();
        assert_eq!(sentences.len(), 1);
        assert_eq!(sentences[0].words[0].form, "はい");
        assert!(sentences[0].words[0].starts_bunsetsu);

        assert!(parse("").unwrap().is_empty());
        assert!(parse("# only a comment\n\n").unwrap().is_empty());
    }

    #[test]
    fn skips_multiword_tokens_and_empty_nodes() {
        let input = format!(
            "{}{}{}{}",
            line("1-2", "彼は", "_", "_", "_"),
            line("1", "彼", "2", "nsubj", "BunsetuBILabel=B"),
            line("1.1", "空", "_", "_", "_"),
            line("2", "読む", "0", "root", "BunsetuBILabel=B"),
        );
        let sentences = parse(&input).unwrap();
        let forms: Vec<&str> = sentences[0]
            .words
            .iter()
            .map(|word| word.form.as_str())
            .collect();
        assert_eq!(forms, ["彼", "読む"]);
    }

    #[test]
    fn refuses_a_malformed_line_with_its_number() {
        assert_eq!(
            error("# c\n1\t彼\t_\n"),
            "line 2: expected 10 columns, found 3"
        );
        assert_eq!(
            error(&line("1", "彼", "0", "root", "BunsetuBILabel=B\textra")),
            "line 1: expected 10 columns, found 11"
        );
        assert_eq!(
            error(&line("2", "彼", "0", "root", "BunsetuBILabel=B")),
            "line 1: expected the word ID 1, found `2`"
        );
        assert_eq!(
            error(&line("1", "彼", "x", "root", "BunsetuBILabel=B")),
            "line 1: the head `x` is not a number"
        );
        assert_eq!(
            error(&line("1", "彼", "0", "root", "SpaceAfter=No")),
            "line 1: no BunsetuBILabel in the last column"
        );
        assert_eq!(
            error(&line("1", "彼", "0", "root", "BunsetuBILabel=O")),
            "line 1: BunsetuBILabel is `O`, not `B` or `I`"
        );
    }

    #[test]
    fn refuses_a_head_outside_the_sentence_or_on_the_word_itself() {
        let outside = format!("# c\n{}", line("1", "彼", "2", "nsubj", "BunsetuBILabel=B"));
        assert_eq!(
            error(&outside),
            "line 1: the head of the word 1 is 2, in a sentence of 1 words"
        );
        let itself = line("1", "彼", "1", "nsubj", "BunsetuBILabel=B");
        assert_eq!(
            error(&itself),
            "line 1: the head of the word 1 is 1, in a sentence of 1 words"
        );
    }

    #[test]
    fn stops_after_an_error() {
        let input = format!(
            "1\tbroken\n\n{}",
            line("1", "彼", "0", "root", "BunsetuBILabel=B")
        );
        let mut reader = ConlluReader::new(input.as_bytes());
        assert!(reader.next().unwrap().is_err());
        assert!(reader.next().is_none());
    }
}
