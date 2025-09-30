#!/bin/bash

# Download and extract IPADIC source files
curl -L -o ~/tmp/mecab-ipadic-2.7.0-20250920.tar.gz "https://Lindera.dev/mecab-ipadic-2.7.0-20250920.tar.gz"
tar zxvf ~/tmp/mecab-ipadic-2.7.0-20250920.tar.gz -C ~/tmp

lindera build -s ~/tmp/mecab-ipadic-2.7.0-20250920 -d ~/tmp/lindera-ipadic-2.7.0-20250920 -m lindera-ipadic/metadata.json

# Download corpus
aws s3 cp --no-sign-request s3://abeja-cc-ja/common_crawl_0.jsonl ~/tmp/common_crawl_0.jsonl

# Extract contents
[[ -f ~/tmp/contents.txt ]] && rm ~/tmp/contents.txt
jq -r '.content' ~/tmp/common_crawl_0.jsonl > ~/tmp/contents.txt

# Create corpus
[[ -f ~/tmp/corpus.txt ]] && rm ~/tmp/corpus.txt
lindera tokenize -d ~/tmp/lindera-ipadic-2.7.0-20250920 ~/tmp/contents.txt >> ~/tmp/corpus.txt
