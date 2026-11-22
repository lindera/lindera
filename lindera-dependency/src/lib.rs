//! Japanese bunsetsu dependency analysis for Lindera.
//!
//! A dependency analysis groups the morphemes of a sentence into bunsetsu
//! (文節) and finds, for each bunsetsu, the bunsetsu it depends on:
//!
//! ```text
//! 彼は 本を 読んだ。
//! 彼は ──────┐
//! 本を ──────┤
//! 読んだ。 ◀──┘   (root)
//! ```
//!
//! This crate holds, so far, what the analysis is built on and nothing of the
//! analysis itself:
//!
//! - [`Morpheme`], the trait of a morpheme: a surface and the feature columns
//!   of its dictionary. The crate depends on no tokenizer, and a type outside
//!   Lindera can implement the trait.
//! - [`Sentence`] and [`Bunsetsu`]: morphemes grouped into bunsetsu, each
//!   with its head, the type of the arc ([`ArcType`]), the positions of its
//!   head word and function word, and a score.
//! - [`lattice`]: reading and writing the lattice format of CaboCha
//!   (`cabocha -f1`), which CaboCha's and J.DepP's tools exchange.
//! - [`Sentence::violations`]: whether a sentence is the tree the parsers
//!   assume, with one root that is the last bunsetsu, every head to the
//!   right, and no crossing arcs.
//!
//! # Examples
//!
//! Read a sentence in the lattice format, look at its bunsetsu, and write it
//! back:
//!
//! ```
//! use lindera_dependency::lattice;
//! use lindera_dependency::Morpheme;
//!
//! let text = "\
//! * 0 2D 0/1 0.000000
//! 彼\t名詞,代名詞,一般,*,*,*,彼,カレ,カレ
//! は\t助詞,係助詞,*,*,*,*,は,ハ,ワ
//! * 1 2D 0/1 0.000000
//! 本\t名詞,一般,*,*,*,*,本,ホン,ホン
//! を\t助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
//! * 2 -1D 0/1 0.000000
//! 読ん\t動詞,自立,*,*,五段・マ行,連用タ接続,読む,ヨン,ヨン
//! だ\t助動詞,*,*,*,特殊・タ,基本形,だ,ダ,ダ
//! 。\t記号,句点,*,*,*,*,。,。,。
//! EOS
//! ";
//!
//! let sentences = lattice::parse(text)?;
//! let sentence = &sentences[0];
//!
//! // Each bunsetsu, as the surfaces of its morphemes, and its head.
//! let bunsetsu: Vec<(String, Option<usize>)> = sentence
//!     .bunsetsu
//!     .iter()
//!     .map(|bunsetsu| {
//!         let surface = sentence.morphemes[bunsetsu.morphemes.clone()]
//!             .iter()
//!             .map(|morpheme| morpheme.surface())
//!             .collect();
//!         (surface, bunsetsu.head)
//!     })
//!     .collect();
//! assert_eq!(
//!     bunsetsu,
//!     [
//!         ("彼は".to_string(), Some(2)),
//!         ("本を".to_string(), Some(2)),
//!         ("読んだ。".to_string(), None),
//!     ]
//! );
//!
//! // The base form of the head word of the last bunsetsu: its position is
//! // counted from the first morpheme of the bunsetsu.
//! let last = &sentence.bunsetsu[2];
//! let (head_word, _) = last.positions.unwrap();
//! let head_word = &sentence.morphemes[last.morphemes.start + head_word];
//! assert_eq!(head_word.feature(6), Some("読む"));
//!
//! // The sentence is a tree as the parsers assume.
//! assert!(sentence.violations().is_empty());
//!
//! // Writing gives back the bytes that were read.
//! let mut output = Vec::new();
//! lattice::write(&mut output, sentence)?;
//! assert_eq!(output, text.as_bytes());
//! # Ok::<(), lindera_dependency::Error>(())
//! ```

pub mod error;
pub mod lattice;
pub mod morpheme;
pub mod sentence;

pub use error::{Error, Result};
pub use morpheme::Morpheme;
pub use sentence::{ArcType, Bunsetsu, Sentence, Violation};
