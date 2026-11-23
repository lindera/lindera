#!/usr/bin/env python3
"""Head rules for IPADIC against the content head of UD Japanese-GSD.

The script the rule of `Schema::ipadic()` was explored with, before it was
written in Rust: it shows examples by surface, which the report of
`lindera-dependency convert` does not, and it measures the variants that
were not taken. The rule that was taken is measured by `convert` itself
(convert_corpora.sh) and step by step by head_variants/.

The training data (lattice format, IPADIC) is laid over the CoNLL-U files by
character offsets. The content head of a bunsetsu is its word with
`BunsetuPositionType=SEM_HEAD`, or `ROOT`; the head by a rule agrees when it
is the morpheme that holds the first character of that word.

Usage:
    python3 -I -B head_rules.py <data directory> table
    python3 -I -B head_rules.py <data directory> disagreements [<rules>]
    python3 -I -B head_rules.py <data directory> symbols

    <data directory> holds gsd/ (fetch_corpora.sh) and lattice/
    (convert_corpora.sh).

table          The agreement on train, dev and test: for CaboCha's rule for
               IPA, for each addition of `Schema::ipadic()` in turn, and for
               the variants that were not taken.
disagreements  The most frequent disagreements on train, by part of speech
               and by surface, for the rules given: names of RULES separated
               by commas (the rule of `Schema::ipadic()` when left out).
symbols        The morphemes of train that are not 記号 and whose surface
               has no letter or digit, with whether the corpus has them as
               the head.

The symbols are recognized here by their Unicode category, and in
`Schema::ipadic()` by a list of characters; the two give the same counts on
these files.
"""

import collections
import sys
import unicodedata

# The function word patterns of CaboCha for IPA (src/selector_pat.h).
CABOCHA_FUNCTION_WORDS = [
    "助詞",
    "助動詞",
    "動詞,非自立",
    "動詞,接尾",
    "形容詞,非自立",
    "形容詞,接尾",
]

# What can be added to CaboCha's rule. The first six, in this order, are the
# rule of `Schema::ipadic()`.
RULES = {
    "symbols": "a symbol is recognized by its surface too (% and ° are units)",
    "suru": "する, できる and 出来る are function words",
    "dependent-noun": "名詞,非自立 is not a head",
    "teki": "the suffix 的 (名詞,接尾,形容動詞語幹) is not a head",
    "sa": "the suffix さ (名詞,接尾,特殊) is not a head",
    "verb-after-particle": "a 動詞,自立 right after a 助詞 is not a head",
    # Not taken.
    "every-suffix": "名詞,接尾 is not a head",
    "counters": "名詞,接尾,助数詞 is not a head",
    "adverbial-noun": "名詞,非自立,副詞可能 stays a head",
}
TAKEN = ["symbols", "suru", "dependent-noun", "teki", "sa", "verb-after-particle"]
NOT_TAKEN = ["every-suffix", "counters", "adverbial-noun"]


def strip(text):
    return "".join(c for c in text if not c.isspace())


def is_mark(surface):
    """Whether a surface is made of punctuation and symbols, units left out."""
    return bool(surface) and all(
        unicodedata.category(c)[0] in "PS" and c not in "%°" for c in surface
    )


def read_conllu(path):
    """The sentences of a file, each the (form, position type) of its words."""
    sentences, words = [], []
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.rstrip("\n")
            if not line:
                if words:
                    sentences.append(words)
                    words = []
                continue
            if line.startswith("#"):
                continue
            columns = line.split("\t")
            if "-" in columns[0] or "." in columns[0]:
                continue
            kind = [
                pair[len("BunsetuPositionType=") :]
                for pair in columns[9].split("|")
                if pair.startswith("BunsetuPositionType=")
            ]
            words.append((strip(columns[1]), kind[0] if kind else None))
    if words:
        sentences.append(words)
    return sentences


def read_lattice(path):
    """The sentences of a file, each the (surface, features) of its bunsetsu."""
    sentences, current = [], []
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.rstrip("\n")
            if line == "EOS":
                sentences.append(current)
                current = []
            elif line.startswith("* "):
                current.append([])
            else:
                surface, features = line.split("\t")[:2]
                current[-1].append((surface, features))
    return sentences


def positions(morphemes, rules):
    """The head and the function word of a bunsetsu, as findHead of CaboCha."""
    head = function = 0
    for index, (surface, features) in enumerate(morphemes):
        columns = features.split(",")
        before = morphemes[index - 1][1] if index > 0 else ""
        is_function = any(features.startswith(p) for p in CABOCHA_FUNCTION_WORDS)
        if (
            "suru" in rules
            and features.startswith("動詞,自立")
            and len(columns) > 6
            and columns[6] in ("する", "できる", "出来る")
        ):
            is_function = True
        if is_function:
            function = index
            continue
        if features.startswith("記号"):
            continue
        if "symbols" in rules and is_mark(surface):
            continue
        kept_adverbial = "adverbial-noun" in rules and features.startswith(
            "名詞,非自立,副詞可能"
        )
        if (
            "dependent-noun" in rules
            and features.startswith("名詞,非自立")
            and not kept_adverbial
        ):
            continue
        if "teki" in rules and features.startswith("名詞,接尾,形容動詞語幹"):
            continue
        if "sa" in rules and features.startswith("名詞,接尾,特殊"):
            continue
        if "every-suffix" in rules and features.startswith("名詞,接尾"):
            continue
        if "counters" in rules and features.startswith("名詞,接尾,助数詞"):
            continue
        if (
            "verb-after-particle" in rules
            and features.startswith("動詞,自立")
            and before.startswith("助詞")
        ):
            continue
        head = index
    function = max(function, head)
    return head, function


def bunsetsu_with_content_head(data, split):
    """Yields (morphemes, position of the content head) for each bunsetsu."""
    corpus = read_conllu(f"{data}/gsd/ja_gsd-ud-{split}.conllu")
    lattice = read_lattice(f"{data}/lattice/ja_gsd-ud-{split}.lattice")
    position = 0
    for sentence in lattice:
        text = "".join(
            strip(surface) for bunsetsu in sentence for surface, _ in bunsetsu
        )
        # The lattice file holds the sentences that were kept, in order.
        while "".join(word[0] for word in corpus[position]) != text:
            position += 1
        words = corpus[position]
        position += 1
        starts, offset = [], 0
        for word in words:
            starts.append(offset)
            offset += len(word[0])
        offset = 0
        for bunsetsu in sentence:
            morpheme_starts, end = [], offset
            for surface, _ in bunsetsu:
                morpheme_starts.append(end)
                end += len(strip(surface))
            # A merged bunsetsu takes the content head of the last one merged.
            content_heads = [
                starts[index]
                for index, word in enumerate(words)
                if offset <= starts[index] < end and word[1] in ("SEM_HEAD", "ROOT")
            ]
            where = None
            if content_heads:
                where = max(
                    index
                    for index, start in enumerate(morpheme_starts)
                    if start <= content_heads[-1]
                )
            yield bunsetsu, where
            offset = end


def short(features):
    return ",".join(features.split(",")[:2])


def table(data):
    variants = [("the rule of CaboCha for IPA", [])]
    for count in range(1, len(TAKEN) + 1):
        variants.append((f"+ {RULES[TAKEN[count - 1]]}", TAKEN[:count]))
    for name in NOT_TAKEN:
        variants.append((f"Schema::ipadic() and: {RULES[name]}", TAKEN + [name]))
    for split in ("train", "dev", "test"):
        agree = collections.Counter()
        measured = 0
        for bunsetsu, content_head in bunsetsu_with_content_head(data, split):
            if content_head is None:
                continue
            measured += 1
            for label, rules in variants:
                if positions(bunsetsu, rules)[0] == content_head:
                    agree[label] += 1
        print(f"{split}: {measured} bunsetsu")
        for label, _ in variants:
            print(f"  {100 * agree[label] / measured:.2f}% ({agree[label]})  {label}")


def disagreements(data, rules):
    kinds, examples, total = collections.Counter(), collections.Counter(), 0
    for bunsetsu, content_head in bunsetsu_with_content_head(data, "train"):
        if content_head is None:
            continue
        head = positions(bunsetsu, rules)[0]
        if head == content_head:
            continue
        total += 1
        side = "right" if head > content_head else "left"
        kinds[(short(bunsetsu[head][1]), side, short(bunsetsu[content_head][1]))] += 1
        text = " ".join(surface for surface, _ in bunsetsu)
        examples[(text, bunsetsu[head][0], bunsetsu[content_head][0])] += 1
    print(f"disagreements on train: {total}")
    for (rule_pos, side, corpus_pos), count in kinds.most_common(30):
        print(f"  {count}: {rule_pos} is the head word, to the {side} of {corpus_pos}")
    print("by surface (bunsetsu, head word by the rule, content head of the corpus):")
    for (text, rule_head, corpus_head), count in examples.most_common(40):
        print(f"  {count}: {text} | {rule_head} | {corpus_head}")


def symbols(data):
    seen = collections.Counter()
    for bunsetsu, content_head in bunsetsu_with_content_head(data, "train"):
        for index, (surface, features) in enumerate(bunsetsu):
            # Every surface without a letter or a digit, the units included.
            no_letter = bool(surface) and all(
                unicodedata.category(c)[0] in "PS" for c in surface
            )
            if no_letter and not features.startswith("記号"):
                seen[(surface, short(features), index == content_head)] += 1
    for (surface, pos, is_head), count in seen.most_common(60):
        role = "the head in the corpus" if is_head else "not the head in the corpus"
        print(f"  {count}: {surface} ({pos}), {role}")


def main(argv):
    data, command = argv[0], argv[1]
    if command == "table":
        table(data)
    elif command == "disagreements":
        rules = argv[2].split(",") if len(argv) > 2 else TAKEN
        unknown = [rule for rule in rules if rule and rule not in RULES]
        if unknown:
            sys.exit(f"unknown rules: {unknown}; the rules are {list(RULES)}")
        disagreements(data, rules)
    elif command == "symbols":
        symbols(data)
    else:
        sys.exit(f"unknown command {command}")


if __name__ == "__main__":
    main(sys.argv[1:])
