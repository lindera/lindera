# Architecture

## Module Structure

```text
lindera-trainer/src/
├── lib.rs                # Trainer: CRF training orchestration; public API re-exports
├── config.rs             # TrainerConfig: parses seed lexicon, char.def, feature.def, rewrite.def
├── corpus.rs              # Corpus, Example, Word: training data representation
├── feature_extractor.rs  # FeatureExtractor: feature template parsing and feature ID management
├── feature_rewriter.rs   # DictionaryRewriter: MeCab-compatible 3-section rewrite
└── model.rs               # Model, SerializableModel: trained model storage, serialization, dictionary output
```

## Key Components

### TrainerConfig

Parses the five training input files -- seed lexicon (`lex.csv`), character definition (`char.def`), unknown-word definition (`unk.def`), feature template (`feature.def`), and rewrite rules (`rewrite.def`) -- into the configuration consumed by `Trainer`. `TrainerConfig::from_readers` (or `from_paths`, its file-path convenience wrapper) extracts the surface/feature vocabulary from the seed lexicon, builds a `FeatureExtractor` from the parsed feature templates, builds a `DictionaryRewriter` from the rewrite rules, and assembles a minimal in-memory `lindera_dictionary::dictionary::Dictionary` (character definition, unknown-word categories, and a system lexicon) from the same input. It also exposes `surfaces()`, `surface_features()`, `get_features()`, a user lexicon accessible via `user_lexicon()` / `add_user_lexicon_entry()` / `load_user_lexicon_from_content()`, and `metadata()`.

### Corpus / Example / Word

Represent annotated training data. `Word` pairs a surface form with its (comma-joined) feature string. `Example` is one training sentence, built from a `Vec<Word>` whose surfaces are concatenated to reconstruct the original sentence text. `Corpus` is a collection of `Example`s; `Corpus::from_reader` parses tab-separated `surface<TAB>features` lines, treating blank lines or literal `EOS` lines as sentence boundaries.

### FeatureExtractor

Parses MeCab-compatible feature templates and manages the mapping from generated feature strings to interned, `NonZeroU32` feature IDs. Supported template placeholders are `%F[n]` / `%F?[n]` (feature field at index `n`, the `?` form skipped when the value is `*`), `%t` (character category), `%w` (surface form, unigram only), `%u` / `%l` / `%r` (full rewritten unigram/left/right feature string), and `%L[n]` / `%L?[n]` / `%R[n]` / `%R?[n]` for bigram left/right context fields. `extract_unigram_feature_ids[_with_ctx]`, `extract_left_feature_ids[_with_ctx]`, and `extract_right_feature_ids[_with_ctx]` apply the parsed templates to a feature vector (plus, for bigram extraction, an optional `TemplateContext` carrying surface/ufeature/lfeature/rfeature) and return the resulting feature IDs, creating new IDs on first use.

### DictionaryRewriter

Implements MeCab's 3-section `rewrite.def` format: `[unigram rewrite]`, `[left rewrite]`, and `[right rewrite]` sections, each holding an ordered list of `pattern<TAB>replacement` rules built into a `FeatureRewriter` prefix trie (via the internal `FeatureRewriterBuilder`). `DictionaryRewriter::from_reader` parses all three sections (falling back to treating an unsectioned file as legacy right-rewrite-only content, for backward compatibility with older Lindera `rewrite.def` files). `rewrite()` applies all three rewriters to a feature string and returns `(ufeature, lfeature, rfeature)`, passing a section through unchanged when no rule matches; `rewrite_cached()` memoizes results per input feature string (MeCab's `rewrite2` equivalent).

### Model / SerializableModel

`Model` is the trained model produced by `Trainer::train`. It owns the trained `lindera_crf::RawModel`, the `TrainerConfig` used to train it, the extracted feature weights and labels, and any user lexicon entries added afterward via `read_user_lexicon`. It converts learned CRF weights into MeCab-compatible integer costs (`tocost(weight, cost_factor) = clamp(-weight * cost_factor, -32767, 32767)`), with the cost factor computed to make full use of the `i16` range (`calculate_cost_factor`). `Model` can serialize itself (`write_model`) and export dictionary source files directly.

`SerializableModel` is the plain-data, `rkyv`-serializable counterpart returned by `Model::read_model` when loading a previously trained model back from a reader (as done by the `lindera export` CLI command). It carries the same trained information as `Model` -- feature weights, labels, POS info, connection cost matrix, unknown-word categories, preserved `char.def`/`feature.def`/`rewrite.def` content, cost factor, and left/right context ID maps -- as owned data, without a `TrainerConfig`, and exposes its own writer methods for producing the dictionary export files.

#### `Model` public methods

| Method | Description |
| --- | --- |
| `read_user_lexicon<R: Read>(&mut self, rdr: R) -> Result<()>` | Loads a user-defined lexicon (same `surface,left_id,right_id,cost,feature...` CSV format as the seed lexicon) into the model so that later exports can assign it inferred connection IDs and costs. Must be called before writing the dictionary if a user lexicon is needed. |
| `write_model<W: Write>(&self, writer: &mut W) -> Result<()>` | Serializes the full trained model (feature weights, labels, POS info, connection matrix, unknown-word categories, preserved definition file contents, cost factor, left/right ID maps) to `rkyv` binary format. |
| `read_model<R: Read>(reader: R) -> Result<SerializableModel>` | Associated function. Deserializes a `SerializableModel` from data written by `write_model`, trying `rkyv` first and falling back to legacy JSON; backfills `feature_sets` from `feature_weights` for older models that predate that field. |
| `write_dictionary<W1, W2, W3, W4>(&self, lexicon_wtr, connector_wtr, unk_handler_wtr, user_lexicon_wtr) -> Result<()>` | Convenience method that writes the lexicon, connection cost matrix, unknown-word dictionary, and user lexicon in one call. |
| `write_lexicon<W: Write>(&self, writer: &mut W) -> Result<()>` | Writes `lex.csv`-style entries (`surface,left_id,right_id,cost,features...`) for every seed vocabulary entry, using the connection IDs and costs from the merged CRF model. |
| `write_connection_costs<W: Write>(&self, writer: &mut W) -> Result<()>` | Writes a dense `matrix.def`-style connection cost matrix over every `(right_id, left_id)` pair (including BOS/EOS); pairs never seen during training get the maximum penalty cost (`i16::MAX`) so Viterbi avoids untrained transitions. |
| `write_unknown_dictionary<W: Write>(&self, writer: &mut W) -> Result<()>` | Writes `unk.def`-style entries for each unknown-word character category, using the learned connection IDs/costs together with the category's feature string from `unk.def`. |
| `get_unknown_word_cost(&self, category: usize) -> i32` | Returns the configured cost for an unknown-word category index (from `unk.def`'s cost column), defaulting to `2000` if the category has no configured cost. |
| `num_features(&self) -> usize` | Number of feature weights held by the trained model. |
| `num_labels(&self) -> usize` | Number of labels (vocabulary surfaces plus unknown-word categories) in the trained model. |
| `raw_model(&self) -> &lindera_crf::RawModel` | Access to the underlying `lindera-crf` raw model, for advanced or lower-level operations. |
| `write_bigram_details<L: Write, R: Write, C: Write>(&self, left_wtr, right_wtr, cost_wtr) -> Result<()>` | Writes three diagnostic files describing bigram connection features and costs (left feature list, right feature list, and left/right feature-pair costs), for dictionary optimization and debugging. |
| `evaluate(&self, test_lattices: &[lindera_crf::Lattice]) -> f64` | Returns the mean absolute value of the raw model's feature weights as a simple evaluation score. The `test_lattices` argument is currently unused; it does not yet score the model against held-out data. |
| `write_dictionary_buffers(&self, lexicon, connector, unk_handler, user_lexicon: &mut Vec<u8>) -> Result<()>` | Serializes labels, feature weights, user-entry count, and surfaces into four raw `rkyv` byte buffers -- a lower-level alternative to the CSV-oriented `write_dictionary`. |
| `write_left_id_def<W: Write>(&self, writer: &mut W) -> Result<()>` | Writes `left-id.def`, mapping each learned left-context ID to its feature string, with `0 BOS/EOS` as the first line. |
| `write_right_id_def<W: Write>(&self, writer: &mut W) -> Result<()>` | Writes `right-id.def`, mapping each learned right-context ID to its feature string, with `0 BOS/EOS` as the first line. |

#### `SerializableModel` public methods

Available on the value returned by `Model::read_model`, for re-exporting dictionary source files from a previously trained and serialized model (used by the `lindera export` CLI command):

| Method | Description |
| --- | --- |
| `write_lexicon<W: Write>(&self, writer: &mut W) -> Result<()>` | Writes `lex.csv`-style entries from the stored `feature_sets`, `labels`, and `pos_info`, skipping the trailing unknown-word category labels. |
| `write_connection_costs<W: Write>(&self, writer: &mut W) -> Result<()>` | Writes the dense `matrix.def`-style connection cost matrix from the stored `connection_matrix`, `max_left_id`, and `max_right_id`. |
| `write_unknown_dictionary<W: Write>(&self, writer: &mut W) -> Result<()>` | Writes `unk.def`-style entries for each stored unknown-word category. |
| `write_char_def<W: Write>(&self, writer: &mut W) -> Result<()>` | Writes the `char.def` content preserved verbatim from the original training input. |
| `write_feature_def<W: Write>(&self, writer: &mut W) -> Result<()>` | Writes the `feature.def` content preserved verbatim from the original training input. |
| `write_rewrite_def<W: Write>(&self, writer: &mut W) -> Result<()>` | Writes the `rewrite.def` content preserved verbatim from the original training input. |
| `write_left_id_def<W: Write>(&self, writer: &mut W) -> Result<()>` | Writes `left-id.def` from the stored `left_id_map`, with `0 BOS/EOS` as the first line. |
| `write_right_id_def<W: Write>(&self, writer: &mut W) -> Result<()>` | Writes `right-id.def` from the stored `right_id_map`, with `0 BOS/EOS` as the first line. |
| `update_metadata_json<W: Write>(&self, base_metadata_path: &Path, writer: &mut W) -> Result<()>` | Reads a base `metadata.json`, updates it with training-derived values (`default_word_cost` estimated from the median feature weight, plus a `model_info` section with feature/label counts, context ID ranges, and training metadata), and writes the result. |

#### `ModelMetadata` and `FeatureSetInfo`

`ModelMetadata` records summary information about a training run: `version`, `regularization` (the cost coefficient used), `iterations` (max iterations configured), `feature_count`, and `label_count`. It is embedded in `SerializableModel::metadata`.

`FeatureSetInfo` holds the per-label connection information extracted from the merged CRF model after training: `left_id`, `right_id`, and `weight`. `SerializableModel::feature_sets` stores one entry per label, in the same order as `labels`.
