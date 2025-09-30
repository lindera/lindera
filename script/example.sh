# This script demonstrates how to train a custom Lindera dictionary
# using the IPADIC dictionary as a base and a corpus from Common Crawl.
# It requires the Lindera CLI and jq to be installed.

#!/bin/bash

# Download and extract IPADIC source files
curl -L -o ~/tmp/mecab-ipadic-2.7.0-20250920.tar.gz "https://Lindera.dev/mecab-ipadic-2.7.0-20250920.tar.gz"
tar zxvf ~/tmp/mecab-ipadic-2.7.0-20250920.tar.gz -C ~/tmp

lindera build -s ~/tmp/mecab-ipadic-2.7.0-20250920 -d ~/tmp/lindera-ipadic-2.7.0-20250920 -m ./lindera-ipadic/metadata.json

# Download corpus
aws s3 cp --no-sign-request s3://abeja-cc-ja/common_crawl_0.jsonl ~/tmp/common_crawl_0.jsonl

# Extract contents
[[ -f ~/tmp/contents.txt ]] && rm ~/tmp/contents.txt
jq -r '.content' ~/tmp/common_crawl_0.jsonl > ~/tmp/contents.txt

# Create corpus
[[ -f ~/tmp/corpus.txt ]] && rm ~/tmp/corpus.txt
lindera tokenize -d ~/tmp/lindera-ipadic-2.7.0-20250920 ~/tmp/contents.txt >> ~/tmp/corpus.txt

# Make seed file
[[ -f ~/tmp/seed.csv ]] && rm ~/tmp/seed.csv
cat ~/tmp/mecab-ipadic-2.7.0-20250920/*.csv > ~/tmp/seed.csv

# Train model
[[ -f ~/tmp/lindera.model ]] && rm ~/tmp/lindera.model
lindera train \
  --seed ~/tmp/seed.csv \
  --corpus ~/tmp/corpus.txt \
  --unk-def ~/tmp/mecab-ipadic-2.7.0-20250920/unk.def \
  --char-def ~/tmp/mecab-ipadic-2.7.0-20250920/char.def \
  --feature-def ~/tmp/mecab-ipadic-2.7.0-20250920/feature.def \
  --rewrite-def ~/tmp/mecab-ipadic-2.7.0-20250920/rewrite.def \
  --output ~/tmp/lindera.model \
  --lambda 0.01 \
  --max-iterations 100

# Export trained dictionary
[[ -d ~/tmp/lindera-dict ]] && rm -rf ~/tmp/lindera-dict
lindera export \
  --model ~/tmp/lindera.model \
  --metadata ./lindera-ipadic/metadata.json \
  --output ~/tmp/lindera-dict

# Build final dictionary
[[ -d ~/tmp/lindera-compiled-dict ]] && rm -rf ~/tmp/lindera-compiled-dict
lindera build \
  --src ~/tmp/lindera-dict \
  --dest ~/tmp/lindera-compiled-dict \
  --metadata ./lindera-ipadic/metadata.json

# Tokenize text using the compiled dictionary
lindera tokenize -d ~/tmp/lindera-compiled-dict <<< "すもももももももものうち"
