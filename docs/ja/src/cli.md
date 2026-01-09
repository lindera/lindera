# Lindera CLI

[Lindera](https://github.com/lindera-morphology/lindera) のための形態素解析コマンドラインインターフェースです。

## インストール

cargo経由でバイナリをインストールできます：

```shell script
% cargo install lindera-cli
```

または、以下のリリースページからバイナリをダウンロードすることもできます：

- [https://github.com/lindera-morphology/lindera/releases](https://github.com/lindera-morphology/lindera/releases)

## ビルド

### IPADIC（日本語辞書）を含めてビルド

"ipadic" 機能フラグを使用すると、LinderaにIPADICを含めることができます。

```shell script
% cargo build --release --features=embed-ipadic
```

### UniDic（日本語辞書）を含めてビルド

"unidic" 機能フラグを使用すると、LinderaにUniDicを含めることができます。

```shell script
% cargo build --release --features=embed-unidic
```

### ko-dic（韓国語辞書）を含めてビルド

"ko-dic" 機能フラグを使用すると、Linderaにko-dicを含めることができます。

```shell script
% cargo build --release --features=embed-ko-dic
```

### CC-CEDICT（中国語辞書）を含めてビルド

"cc-cedict" 機能フラグを使用すると、LinderaにCC-CEDICTを含めることができます。

```shell script
% cargo build --release --features=embed-cc-cedict
```

### 辞書なしでビルド

Linderaのバイナリサイズを削減するには、機能フラグを省略します。
これにより、辞書が含まれなくなるため、トークナイザーとトレーナーのみを含むバイナリになります。

```shell script
% cargo build --release
```

### 全機能を含めてビルド

```shell script
% cargo build --release --all-features
```

## 辞書のビルド

Linderaで使用するための形態素解析辞書をCSVソースファイルからビルド（コンパイル）します。

### 基本的なビルド方法

```bash
# システム辞書のビルド
lindera build \
  --src /path/to/dictionary/csv \
  --dest /path/to/output/dictionary \
  --metadata ./lindera-ipadic/metadata.json

# ユーザー辞書のビルド
lindera build \
  --src ./user_dict.csv \
  --dest ./user_dictionary \
  --metadata ./lindera-ipadic/metadata.json \
  --user
```

### ビルドパラメータ

- `--src` / `-s`: 辞書CSVファイルを含むソースディレクトリ（ユーザー辞書の場合は単一CSVファイル）
- `--dest` / `-d`: コンパイルされた辞書の出力先ディレクトリ
- `--metadata` / `-m`: 辞書構造を定義するメタデータ設定ファイル (metadata.json)
- `--user` / `-u`: システム辞書の代わりにユーザー辞書をビルドする（オプションフラグ）

### 辞書の種類

#### システム辞書 (System dictionary)

以下を含む完全な形態素解析辞書です：

- 語彙エントリ（単語定義）
- 接続コスト行列
- 未知語処理ルール
- 文字種定義

#### ユーザー辞書 (User dictionary)

システム辞書と一緒に動作する、カスタム単語のための補助辞書です。

### 例

#### IPADIC（日本語辞書）のビルド

```shell script
# IPADICソースファイルのダウンロードと展開
% curl -L -o /tmp/mecab-ipadic-2.7.0-20250920.tar.gz "https://Lindera.dev/mecab-ipadic-2.7.0-20250920.tar.gz"
% tar zxvf /tmp/mecab-ipadic-2.7.0-20250920.tar.gz -C /tmp

# 辞書のビルド
% lindera build \
  --src /tmp/mecab-ipadic-2.7.0-20250920 \
  --dest /tmp/lindera-ipadic-2.7.0-20250920 \
  --metadata ./lindera-ipadic/metadata.json

% ls -al /tmp/lindera-ipadic-2.7.0-20250920
% (cd /tmp && zip -r lindera-ipadic-2.7.0-20250920.zip lindera-ipadic-2.7.0-20250920/)
% tar -czf /tmp/lindera-ipadic-2.7.0-20250920.tar.gz -C /tmp lindera-ipadic-2.7.0-20250920
```

#### IPADIC NEologd（日本語辞書）のビルド

```shell script
# IPADIC NEologdソースファイルのダウンロードと展開
% curl -L -o /tmp/mecab-ipadic-neologd-0.0.7-20200820.tar.gz "https://lindera.dev/mecab-ipadic-neologd-0.0.7-20200820.tar.gz"
% tar zxvf /tmp/mecab-ipadic-neologd-0.0.7-20200820.tar.gz -C /tmp

# 辞書のビルド
% lindera build \
  --src /tmp/mecab-ipadic-neologd-0.0.7-20200820 \
  --dest /tmp/lindera-ipadic-neologd-0.0.7-20200820 \
  --metadata ./lindera-ipadic-neologd/metadata.json

% ls -al /tmp/lindera-ipadic-neologd-0.0.7-20200820
% (cd /tmp && zip -r lindera-ipadic-neologd-0.0.7-20200820.zip lindera-ipadic-neologd-0.0.7-20200820/)
% tar -czf /tmp/lindera-ipadic-neologd-0.0.7-20200820.tar.gz -C /tmp lindera-ipadic-neologd-0.0.7-20200820
```

#### UniDic（日本語辞書）のビルド

```shell script
# UniDicソースファイルのダウンロードと展開
% curl -L -o /tmp/unidic-mecab-2.1.2.tar.gz "https://Lindera.dev/unidic-mecab-2.1.2.tar.gz"
% tar zxvf /tmp/unidic-mecab-2.1.2.tar.gz -C /tmp

# 辞書のビルド
% lindera build \
  --src /tmp/unidic-mecab-2.1.2 \
  --dest /tmp/lindera-unidic-2.1.2 \
  --metadata ./lindera-unidic/metadata.json

% ls -al /tmp/lindera-unidic-2.1.2
% (cd /tmp && zip -r lindera-unidic-2.1.2.zip lindera-unidic-2.1.2/)
% tar -czf /tmp/lindera-unidic-2.1.2.tar.gz -C /tmp lindera-unidic-2.1.2
```

#### CC-CEDICT（中国語辞書）のビルド

```shell script
# CC-CEDICTソースファイルのダウンロードと展開
% curl -L -o /tmp/CC-CEDICT-MeCab-0.1.0-20200409.tar.gz "https://lindera.dev/CC-CEDICT-MeCab-0.1.0-20200409.tar.gz"
% tar zxvf /tmp/CC-CEDICT-MeCab-0.1.0-20200409.tar.gz -C /tmp

# 辞書のビルド
% lindera build \
  --src /tmp/CC-CEDICT-MeCab-0.1.0-20200409 \
  --dest /tmp/lindera-cc-cedict-0.1.0-20200409 \
  --metadata ./lindera-cc-cedict/metadata.json

% ls -al /tmp/lindera-cc-cedict-0.1.0-20200409
% (cd /tmp && zip -r lindera-cc-cedict-0.1.0-20200409.zip lindera-cc-cedict-0.1.0-20200409/)
% tar -czf /tmp/lindera-cc-cedict-0.1.0-20200409.tar.gz -C /tmp lindera-cc-cedict-0.1.0-20200409
```

#### ko-dic（韓国語辞書）のビルド

```shell script
# ko-dicソースファイルのダウンロードと展開
% curl -L -o /tmp/mecab-ko-dic-2.1.1-20180720.tar.gz "https://Lindera.dev/mecab-ko-dic-2.1.1-20180720.tar.gz"
% tar zxvf /tmp/mecab-ko-dic-2.1.1-20180720.tar.gz -C /tmp

# 辞書のビルド
% lindera build \
  --src /tmp/mecab-ko-dic-2.1.1-20180720 \
  --dest /tmp/lindera-ko-dic-2.1.1-20180720 \
  --metadata ./lindera-ko-dic/metadata.json

% ls -al /tmp/lindera-ko-dic-2.1.1-20180720
% (cd /tmp && zip -r lindera-ko-dic-2.1.1-20180720.zip lindera-ko-dic-2.1.1-20180720/)
% tar -czf /tmp/lindera-ko-dic-2.1.1-20180720.tar.gz -C /tmp lindera-ko-dic-2.1.1-20180720
```

## ユーザー辞書のビルド

### IPADICユーザー辞書（日本語）のビルド

ユーザー辞書フォーマットの詳細については、以下のURLを参照してください：

- [Lindera IPADIC Builder/User Dictionary Format](https://github.com/lindera-morphology/lindera/tree/main/lindera-ipadic-builder#user-dictionary-format-csv)

```shell
% lindera build \
  --src ./resources/user_dict/ipadic_simple_userdic.csv \
  --dest ./resources/user_dict \
  --metadata ./lindera-ipadic/metadata.json \
  --user
```

### UniDicユーザー辞書（日本語）のビルド

ユーザー辞書フォーマットの詳細については、以下のURLを参照してください：

- [Lindera UniDic Builder/User Dictionary Format](https://github.com/lindera-morphology/lindera/tree/main/lindera-unidic-builder#user-dictionary-format-csv)

```shell
% lindera build \
  --src ./resources/user_dict/unidic_simple_userdic.csv \
  --dest ./resources/user_dict \
  --metadata ./lindera-unidic/metadata.json \
  --user
```

### CC-CEDICTユーザー辞書（中国語）のビルド

ユーザー辞書フォーマットの詳細については、以下のURLを参照してください：

- [Lindera CC-CEDICT Builder/User Dictionary Format](https://github.com/lindera-morphology/lindera/tree/main/lindera-cc-cedict-builder#user-dictionary-format-csv)

```shell
% lindera build \
  --src ./resources/user_dict/cc-cedict_simple_userdic.csv \
  --dest ./resources/user_dict \
  --metadata ./lindera-cc-cedict/metadata.json \
  --user
```

### ko-dicユーザー辞書（韓国語）のビルド

ユーザー辞書フォーマットの詳細については、以下のURLを参照してください：

- [Lindera ko-dic Builder/User Dictionary Format](https://github.com/lindera-morphology/lindera/tree/main/lindera-ko-dic-builder#user-dictionary-format-csv)

```shell
% lindera build \
  --src ./resources/user_dict/ko-dic_simple_userdic.csv \
  --dest ./resources/user_dict \
  --metadata ./lindera-ko-dic/metadata.json \
  --user
```

## テキストのトークナイズ

様々な辞書を使用して、日本語、中国語、または韓国語のテキストに対して形態素解析（トークナイズ）を行います。

### 基本的なトークナイズ方法

```bash
# 辞書ディレクトリを指定してトークナイズ
echo "日本語の形態素解析を行うことができます。" | lindera tokenize \
  --dict /path/to/dictionary

# 埋め込み辞書を指定してトークナイズ
echo "日本語の形態素解析を行うことができます。" | lindera tokenize \
  --dict embedded://ipadic

# 出力形式を指定してトークナイズ
echo "日本語の形態素解析を行うことができます。" | lindera tokenize \
  --dict embedded://ipadic \
  --output json

# ファイルからテキストを読み込んでトークナイズ
lindera tokenize \
  --dict /path/to/dictionary \
  --output wakati \
  input.txt
```

### トークナイズパラメータ

- `--dict` / `-d`: 辞書のパスまたはURI（必須）
  - ファイルパス: `/path/to/dictionary`
  - 埋め込み: `embedded://ipadic`, `embedded://unidic`, etc.
- `--output` / `-o`: 出力形式 (デフォルト: mecab)
  - `mecab`: 品詞情報を含むMeCab互換形式
  - `wakati`: スペース区切りのトークンのみ
  - `json`: すべてのトークン情報を含む詳細なJSON形式
- `--user-dict` / `-u`: ユーザー辞書のパス（オプション）
- `--mode` / `-m`: トークナイズモード (デフォルト: normal)
  - `normal`: 標準的なトークナイズ
  - `decompose`: 複合語を分解する
- `--char-filter` / `-c`: 文字フィルタ設定 (JSON)
- `--token-filter` / `-t`: トークンフィルタ設定 (JSON)
- 入力ファイル: オプションのファイルパス (デフォルト: 標準入力)

### 外部辞書を使用した例

#### 外部IPADIC（日本語辞書）を使用したトークナイズ

```shell
% echo "日本語の形態素解析を行うことができます。" | lindera tokenize \
  --dict /tmp/lindera-ipadic-2.7.0-20250920
```

```text
日本語  名詞,一般,*,*,*,*,日本語,ニホンゴ,ニホンゴ
の      助詞,連体化,*,*,*,*,の,ノ,ノ
形態素  名詞,一般,*,*,*,*,形態素,ケイタイソ,ケイタイソ
解析    名詞,サ変接続,*,*,*,*,解析,カイセキ,カイセキ
を      助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
行う    動詞,自立,*,*,五段・ワ行促音便,基本形,行う,オコナウ,オコナウ
こと    名詞,非自立,一般,*,*,*,こと,コト,コト
が      助詞,格助詞,一般,*,*,*,が,ガ,ガ
でき    動詞,自立,*,*,一段,連用形,できる,デキ,デキ
ます    助動詞,*,*,*,特殊・マス,基本形,ます,マス,マス
。      記号,句点,*,*,*,*,。,。,。
EOS
```

#### 外部IPADIC NEologd（日本語辞書）を使用したトークナイズ

```shell
% echo "日本語の形態素解析を行うことができます。" | lindera tokenize \
  --dict /tmp/lindera-ipadic-neologd-0.0.7-20200820
```

```text
日本語  名詞,一般,*,*,*,*,日本語,ニホンゴ,ニホンゴ
の      助詞,連体化,*,*,*,*,の,ノ,ノ
形態素解析      名詞,固有名詞,一般,*,*,*,形態素解析,ケイタイソカイセキ,ケイタイソカイセキ
を      助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
行う    動詞,自立,*,*,五段・ワ行促音便,基本形,行う,オコナウ,オコナウ
こと    名詞,非自立,一般,*,*,*,こと,コト,コト
が      助詞,格助詞,一般,*,*,*,が,ガ,ガ
でき    動詞,自立,*,*,一段,連用形,できる,デキ,デキ
ます    助動詞,*,*,*,特殊・マス,基本形,ます,マス,マス
。      記号,句点,*,*,*,*,。,。,。
EOS
```

#### 外部UniDic（日本語辞書）を使用したトークナイズ

```shell
% echo "日本語の形態素解析を行うことができます。" | lindera tokenize \
  --dict /tmp/lindera-unidic-2.1.2
```

```text
日本    名詞,固有名詞,地名,国,*,*,ニッポン,日本,日本,ニッポン,日本,ニッポン,固,*,*,*,*
語      名詞,普通名詞,一般,*,*,*,ゴ,語,語,ゴ,語,ゴ,漢,*,*,*,*
の      助詞,格助詞,*,*,*,*,ノ,の,の,ノ,の,ノ,和,*,*,*,*
形態    名詞,普通名詞,一般,*,*,*,ケイタイ,形態,形態,ケータイ,形態,ケータイ,漢,*,*,*,*
素      接尾辞,名詞的,一般,*,*,*,ソ,素,素,ソ,素,ソ,漢,*,*,*,*
解析    名詞,普通名詞,サ変可能,*,*,*,カイセキ,解析,解析,カイセキ,解析,カイセキ,漢,*,*,*,*
を      助詞,格助詞,*,*,*,*,ヲ,を,を,オ,を,オ,和,*,*,*,*
行う    動詞,一般,*,*,五段-ワア行,連体形-一般,オコナウ,行う,行う,オコナウ,行う,オコナウ,和,*,*,*,*
こと    名詞,普通名詞,一般,*,*,*,コト,事,こと,コト,こと,コト,和,コ濁,基本形,*,*
が      助詞,格助詞,*,*,*,*,ガ,が,が,ガ,が,ガ,和,*,*,*,*
でき    動詞,非自立可能,*,*,上一段-カ行,連用形-一般,デキル,出来る,でき,デキ,できる,デキル,和,*,*,*,*
ます    助動詞,*,*,*,助動詞-マス,終止形-一般,マス,ます,ます,マス,ます,マス,和,*,*,*,*
。      補助記号,句点,*,*,*,*,,。,。,,。,,記号,*,*,*,*
EOS
```

#### 外部ko-dic（韓国語辞書）を使用したトークナイズ

```shell
% echo "한국어의형태해석을실시할수있습니다." | lindera tokenize \
  --dict /tmp/lindera-ko-dic-2.1.1-20180720
```

```text
한국어  NNG,*,F,한국어,Compound,*,*,한국/NNG/*+어/NNG/*
의      JKG,*,F,의,*,*,*,*
형태    NNG,*,F,형태,*,*,*,*
해석    NNG,행위,T,해석,*,*,*,*
을      JKO,*,T,을,*,*,*,*
실시    NNG,행위,F,실시,*,*,*,*
할      VV+ETM,*,T,할,Inflect,VV,ETM,하/VV/*+ᆯ/ETM/*
수      NNG,*,F,수,*,*,*,*
있      VX,*,T,있,*,*,*,*
습니다  EF,*,F,습니다,*,*,*,*
.       UNK
EOS
```

#### 外部CC-CEDICT（中国語辞書）を使用したトークナイズ

```shell
% echo "可以进行中文形态学分析。" | lindera tokenize \
  --dict /tmp/lindera-cc-cedict-0.1.0-20200409
```

```text
可以    *,*,*,*,ke3 yi3,可以,可以,can/may/possible/able to/not bad/pretty good/
进行    *,*,*,*,jin4 xing2,進行,进行,to advance/to conduct/underway/in progress/to do/to carry out/to carry on/to execute/
中文    *,*,*,*,Zhong1 wen2,中文,中文,Chinese language/
形态学  *,*,*,*,xing2 tai4 xue2,形態學,形态学,morphology (in biology or linguistics)/
分析    *,*,*,*,fen1 xi1,分析,分析,to analyze/analysis/CL:個|个[ge4]/
。      UNK
EOS
```

### 埋め込み辞書を使用した例

Linderaは、特定の機能フラグを指定してビルドすることで、バイナリに辞書を直接含めることができます。これにより、外部辞書ファイルなしでトークナイズが可能になります。

#### 埋め込みIPADIC（日本語辞書）を使用したトークナイズ

```shell
% echo "日本語の形態素解析を行うことができます。" | lindera tokenize \
  --dict embedded://ipadic
```

```text
日本語  名詞,一般,*,*,*,*,日本語,ニホンゴ,ニホンゴ
の      助詞,連体化,*,*,*,*,の,ノ,ノ
形態素  名詞,一般,*,*,*,*,形態素,ケイタイソ,ケイタイソ
解析    名詞,サ変接続,*,*,*,*,解析,カイセキ,カイセキ
を      助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
行う    動詞,自立,*,*,五段・ワ行促音便,基本形,行う,オコナウ,オコナウ
こと    名詞,非自立,一般,*,*,*,こと,コト,コト
が      助詞,格助詞,一般,*,*,*,が,ガ,ガ
でき    動詞,自立,*,*,一段,連用形,できる,デキ,デキ
ます    助動詞,*,*,*,特殊・マス,基本形,ます,マス,マス
。      記号,句点,*,*,*,*,。,。,。
EOS
```

注意: IPADIC辞書をバイナリに含めるには、`--features=embed-ipadic` オプションを使用してビルドする必要があります。

#### 埋め込みUniDic（日本語辞書）を使用したトークナイズ

```shell
% echo "日本語の形態素解析を行うことができます。" | lindera tokenize \
  --dict embedded://unidic
```

```text
日本    名詞,固有名詞,地名,国,*,*,ニッポン,日本,日本,ニッポン,日本,ニッポン,固,*,*,*,*
語      名詞,普通名詞,一般,*,*,*,ゴ,語,語,ゴ,語,ゴ,漢,*,*,*,*
の      助詞,格助詞,*,*,*,*,ノ,の,の,ノ,の,ノ,和,*,*,*,*
形態    名詞,普通名詞,一般,*,*,*,ケイタイ,形態,形態,ケータイ,形態,ケータイ,漢,*,*,*,*
素      接尾辞,名詞的,一般,*,*,*,ソ,素,素,ソ,素,ソ,漢,*,*,*,*
解析    名詞,普通名詞,サ変可能,*,*,*,カイセキ,解析,解析,カイセキ,解析,カイセキ,漢,*,*,*,*
を      助詞,格助詞,*,*,*,*,ヲ,を,を,オ,を,オ,和,*,*,*,*
行う    動詞,一般,*,*,五段-ワア行,連体形-一般,オコナウ,行う,行う,オコナウ,行う,オコナウ,和,*,*,*,*
こと    名詞,普通名詞,一般,*,*,*,コト,事,こと,コト,こと,コト,和,コ濁,基本形,*,*
が      助詞,格助詞,*,*,*,*,ガ,が,が,ガ,が,ガ,和,*,*,*,*
でき    動詞,非自立可能,*,*,上一段-カ行,連用形-一般,デキル,出来る,でき,デキ,できる,デキル,和,*,*,*,*
ます    助動詞,*,*,*,助動詞-マス,終止形-一般,マス,ます,ます,マス,ます,マス,和,*,*,*,*
。      補助記号,句点,*,*,*,*,,。,。,,。,,記号,*,*,*,*
EOS
```

注意: UniDic辞書をバイナリに含めるには、`--features=embed-unidic` オプションを使用してビルドする必要があります。

#### 埋め込みIPADIC NEologd（日本語辞書）を使用したトークナイズ

```shell
% echo "日本語の形態素解析を行うことができます。" | lindera tokenize \
  --dict embedded://ipadic-neologd
```

```text
日本語  名詞,一般,*,*,*,*,日本語,ニホンゴ,ニホンゴ
の      助詞,連体化,*,*,*,*,の,ノ,ノ
形態素解析      名詞,固有名詞,一般,*,*,*,形態素解析,ケイタイソカイセキ,ケイタイソカイセキ
を      助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
行う    動詞,自立,*,*,五段・ワ行促音便,基本形,行う,オコナウ,オコナウ
こと    名詞,非自立,一般,*,*,*,こと,コト,コト
が      助詞,格助詞,一般,*,*,*,が,ガ,ガ
でき    動詞,自立,*,*,一段,連用形,できる,デキ,デキ
ます    助動詞,*,*,*,特殊・マス,基本形,ます,マス,マス
。      記号,句点,*,*,*,*,。,。,。
EOS
```

注意: IPADIC NEologd辞書をバイナリに含めるには、`--features=embed-ipadic-neologd` オプションを使用してビルドする必要があります。

#### 埋め込みko-dic（韓国語辞書）を使用したトークナイズ

```shell
% echo "한국어의형태해석을실시할수있습니다." | lindera tokenize \
  --dict embedded://ko-dic
```

```text
한국어  NNG,*,F,한국어,Compound,*,*,한국/NNG/*+어/NNG/*
의      JKG,*,F,의,*,*,*,*
형태    NNG,*,F,형태,*,*,*,*
해석    NNG,행위,T,해석,*,*,*,*
을      JKO,*,T,을,*,*,*,*
실시    NNG,행위,F,실시,*,*,*,*
할      VV+ETM,*,T,할,Inflect,VV,ETM,하/VV/*+ᆯ/ETM/*
수      NNG,*,F,수,*,*,*,*
있      VX,*,T,있,*,*,*,*
습니다  EF,*,F,습니다,*,*,*,*
.       UNK
EOS
```

注意: ko-dic辞書をバイナリに含めるには、`--features=embed-ko-dic` オプションを使用してビルドする必要があります。

#### 埋め込みCC-CEDICT（中国語辞書）を使用したトークナイズ

```shell
% echo "可以进行中文形态学分析。" | lindera tokenize \
  --dict embedded://cc-cedict
```

```text
可以    *,*,*,*,ke3 yi3,可以,可以,can/may/possible/able to/not bad/pretty good/
进行    *,*,*,*,jin4 xing2,進行,进行,to advance/to conduct/underway/in progress/to do/to carry out/to carry on/to execute/
中文    *,*,*,*,Zhong1 wen2,中文,中文,Chinese language/
形态学  *,*,*,*,xing2 tai4 xue2,形態學,形态学,morphology (in biology or linguistics)/
分析    *,*,*,*,fen1 xi1,分析,分析,to analyze/analysis/CL:個|个[ge4]/
。      UNK
EOS
```

注意: CC-CEDICT辞書をバイナリに含めるには、`--features=embed-cc-cedict` オプションを使用してビルドする必要があります。

### ユーザー辞書の例

Linderaは、システム辞書と一緒にカスタム単語を追加するためのユーザー辞書をサポートしています。ユーザー辞書はCSVまたはバイナリ形式にすることができます。

#### ユーザー辞書の使用（CSV形式）

```shell
% echo "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です" | lindera tokenize \
  --dict embedded://ipadic \
  --user-dict ./resources/user_dict/ipadic_simple_userdic.csv
```

```text
東京スカイツリー        カスタム名詞,*,*,*,*,*,東京スカイツリー,トウキョウスカイツリー,*
の      助詞,連体化,*,*,*,*,の,ノ,ノ
最寄り駅        名詞,一般,*,*,*,*,最寄り駅,モヨリエキ,モヨリエキ
は      助詞,係助詞,*,*,*,*,は,ハ,ワ
とうきょうスカイツリー駅        カスタム名詞,*,*,*,*,*,とうきょうスカイツリー駅,トウキョウスカイツリーエキ,*
です    助動詞,*,*,*,特殊・デス,基本形,です,デス,デス
EOS
```

#### ユーザー辞書の使用（バイナリ形式）

```shell
% echo "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です" | lindera tokenize \
  --dict /tmp/lindera-ipadic-2.7.0-20250920 \
  --user-dict ./resources/user_dict/ipadic_simple_userdic.bin
```

```text
東京スカイツリー        カスタム名詞,*,*,*,*,*,東京スカイツリー,トウキョウスカイツリー,*
の      助詞,連体化,*,*,*,*,の,ノ,ノ
最寄り駅        名詞,一般,*,*,*,*,最寄り駅,モヨリエキ,モヨリエキ
は      助詞,係助詞,*,*,*,*,は,ハ,ワ
とうきょうスカイツリー駅        カスタム名詞,*,*,*,*,*,とうきょうスカイツリー駅,トウキョウスカイツリーエキ,*
です    助動詞,*,*,*,特殊・デス,基本形,です,デス,デス
EOS
```

### トークナイズモード

Linderaは2つのトークナイズモードを提供します：`normal` と `decompose` です。

#### Normal モード（デフォルト）

辞書に登録された単語に基づいて忠実にトークナイズします：

```shell
% echo "関西国際空港限定トートバッグ" | lindera tokenize \
  --dict embedded://ipadic \
  --mode normal
```

```text
関西国際空港    名詞,固有名詞,組織,*,*,*,関西国際空港,カンサイコクサイクウコウ,カンサイコクサイクーコー
限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
トートバッグ    UNK,*,*,*,*,*,*,*,*
EOS
```

#### Decompose モード

複合語をさらに分解してトークナイズします：

```shell
% echo "関西国際空港限定トートバッグ" | lindera tokenize \
  --dict embedded://ipadic \
  --mode decompose
```

```text
関西    名詞,固有名詞,地域,一般,*,*,関西,カンサイ,カンサイ
国際    名詞,一般,*,*,*,*,国際,コクサイ,コクサイ
空港    名詞,一般,*,*,*,*,空港,クウコウ,クーコー
限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
トートバッグ    UNK,*,*,*,*,*,*,*,*
EOS
```

### 出力形式

Linderaは3つの出力形式を提供します：`mecab`, `wakati`, `json`。

#### MeCab 形式（デフォルト）

品詞情報を含むMeCab互換形式で結果を出力します：

```shell
% echo "お待ちしております。" | lindera tokenize \
  --dict embedded://ipadic \
  --output mecab
```

```text
お待ち  名詞,サ変接続,*,*,*,*,お待ち,オマチ,オマチ
し  動詞,自立,*,*,サ変・スル,連用形,する,シ,シ
て  助詞,接続助詞,*,*,*,*,て,テ,テ
おり  動詞,非自立,*,*,五段・ラ行,連用形,おる,オリ,オリ
ます  助動詞,*,*,*,特殊・マス,基本形,ます,マス,マス
。  記号,句点,*,*,*,*,。,。,。
EOS
```

#### Wakati 形式

トークンテキストのみをスペース区切りで出力します：

```shell
% echo "お待ちしております。" | lindera tokenize \
  --dict embedded://ipadic \
  --output wakati
```

```text
お待ち し て おり ます 。
```

#### JSON 形式

すべてのトークン情報を含む詳細なJSON形式で出力します：

```shell
% echo "お待ちしております。" | lindera tokenize \
  --dict embedded://ipadic \
  --output json
```

```json
[
  {
    "base_form": "お待ち",
    "byte_end": 9,
    "byte_start": 0,
    "conjugation_form": "*",
    "conjugation_type": "*",
    "part_of_speech": "名詞",
    "part_of_speech_subcategory_1": "サ変接続",
    "part_of_speech_subcategory_2": "*",
    "part_of_speech_subcategory_3": "*",
    "pronunciation": "オマチ",
    "reading": "オマチ",
    "surface": "お待ち",
    "word_id": 14698
  },
  {
    "base_form": "する",
    "byte_end": 12,
    "byte_start": 9,
    "conjugation_form": "サ変・スル",
    "conjugation_type": "連用形",
    "part_of_speech": "動詞",
    "part_of_speech_subcategory_1": "自立",
    "part_of_speech_subcategory_2": "*",
    "part_of_speech_subcategory_3": "*",
    "pronunciation": "シ",
    "reading": "シ",
    "surface": "し",
    "word_id": 30763
  },
  {
    "base_form": "て",
    "byte_end": 15,
    "byte_start": 12,
    "conjugation_form": "*",
    "conjugation_type": "*",
    "part_of_speech": "助詞",
    "part_of_speech_subcategory_1": "接続助詞",
    "part_of_speech_subcategory_2": "*",
    "part_of_speech_subcategory_3": "*",
    "pronunciation": "テ",
    "reading": "テ",
    "surface": "て",
    "word_id": 46603
  },
  {
    "base_form": "おる",
    "byte_end": 21,
    "byte_start": 15,
    "conjugation_form": "五段・ラ行",
    "conjugation_type": "連用形",
    "part_of_speech": "動詞",
    "part_of_speech_subcategory_1": "非自立",
    "part_of_speech_subcategory_2": "*",
    "part_of_speech_subcategory_3": "*",
    "pronunciation": "オリ",
    "reading": "オリ",
    "surface": "おり",
    "word_id": 14239
  },
  {
    "base_form": "ます",
    "byte_end": 27,
    "byte_start": 21,
    "conjugation_form": "特殊・マス",
    "conjugation_type": "基本形",
    "part_of_speech": "助動詞",
    "part_of_speech_subcategory_1": "*",
    "part_of_speech_subcategory_2": "*",
    "part_of_speech_subcategory_3": "*",
    "pronunciation": "マス",
    "reading": "マス",
    "surface": "ます",
    "word_id": 68733
  },
  {
    "base_form": "。",
    "byte_end": 30,
    "byte_start": 27,
    "conjugation_form": "*",
    "conjugation_type": "*",
    "part_of_speech": "記号",
    "part_of_speech_subcategory_1": "句点",
    "part_of_speech_subcategory_2": "*",
    "part_of_speech_subcategory_3": "*",
    "pronunciation": "。",
    "reading": "。",
    "surface": "。",
    "word_id": 101
  }
]
```

## 高度なトークナイズ

Linderaは、文字フィルタ、トークナイザー、トークンフィルタを組み合わせた分析フレームワークを提供します。フィルタはJSONを使用して構成します。

### 文字フィルタとトークンフィルタの使用

```shell
% echo "すもももももももものうち" | lindera tokenize \
  --dict embedded://ipadic \
  --char-filter 'unicode_normalize:{"kind":"nfkc"}' \
  --token-filter 'japanese_keep_tags:{"tags":["名詞,一般"]}'
```

```text
すもも  名詞,一般,*,*,*,*,すもも,スモモ,スモモ
もも    名詞,一般,*,*,*,*,もも,モモ,モモ
もも    名詞,一般,*,*,*,*,もも,モモ,モモ
EOS
```

## 辞書の学習（実験的機能）

注釈付きコーパスデータから新しい形態素解析モデルを学習します。この機能を使用するには、`train` 機能フラグを有効にしてビルドする必要があります。（`train` 機能フラグはデフォルトで有効になっています。）

### 学習パラメータ

- `--seed` / `-s`: 重み付けを行うシード語彙ファイル（CSV形式）
- `--corpus` / `-c`: 学習用コーパス（注釈付きテキスト）
- `--char-def` / `-C`: 文字定義ファイル (char.def)
- `--unk-def` / `-u`: 未知語定義ファイル (unk.def) - 重み付けの対象
- `--feature-def` / `-f`: 素性定義ファイル (feature.def)
- `--rewrite-def` / `-r`: 書換えルール定義ファイル (rewrite.def)
- `--output` / `-o`: 出力モデルファイル
- `--lambda` / `-l`: L1正則化 (0.0-1.0) (デフォルト: 0.01)
- `--max-iterations` / `-i`: 学習の最大反復回数 (デフォルト: 100)
- `--max-threads` / `-t`: 最大スレッド数 (デフォルトはCPUコア数、データセットサイズに基づいて自動調整)

### 基本的なワークフロー

#### 1. 学習用ファイルの準備

**シード語彙ファイル (seed.csv):**

シード語彙ファイルは、CRFモデルの学習に使用される初期辞書エントリを含みます。各行はカンマ区切りのフィールドを持つ単語エントリを表します。具体的なフィールド構成は辞書フォーマットによって異なります：

- 表層形
- 左文脈ID
- 右文脈ID
- 単語コスト
- 品詞タグ（複数のフィールド）
- 原形
- 読み（カタカナ）
- 発音

注意: 正確なフィールド定義は辞書フォーマット（IPADIC, UniDic, ko-dic, CC-CEDICT）によって異なります。詳細は各辞書のフォーマット仕様を参照してください。

```csv
外国,0,0,0,名詞,一般,*,*,*,*,外国,ガイコク,ガイコク
人,0,0,0,名詞,接尾,一般,*,*,*,人,ジン,ジン
```

**学習用コーパス (corpus.txt):**

学習用コーパスファイルは、CRFモデルの学習に使用される注釈付きテキストデータを含みます。各行は以下で構成されます：

- 表層形（単語）とそれに続くタブ文字
- カンマ区切りの形態素素性（品詞タグ、原形、読み、発音）
- 文は "EOS" (End Of Sentence) マーカーで区切られます

注意: 形態素素性のフォーマットは辞書（IPADIC, UniDic, ko-dic, CC-CEDICT）によって異なります。詳細は各辞書のフォーマット仕様を参照してください。

```text
外国	名詞,一般,*,*,*,*,外国,ガイコク,ガイコク
人	名詞,接尾,一般,*,*,*,人,ジン,ジン
参政	名詞,サ変接続,*,*,*,*,参政,サンセイ,サンセイ
権	名詞,接尾,一般,*,*,*,権,ケン,ケン
EOS

これ	連体詞,*,*,*,*,*,これ,コレ,コレ
は	助詞,係助詞,*,*,*,*,は,ハ,ワ
テスト	名詞,サ変接続,*,*,*,*,テスト,テスト,テスト
です	助動詞,*,*,*,特殊・デス,基本形,です,デス,デス
。	記号,句点,*,*,*,*,。,。,。
EOS

形態	名詞,一般,*,*,*,*,形態,ケイタイ,ケイタイ
素	名詞,接尾,一般,*,*,*,素,ソ,ソ
解析	名詞,サ変接続,*,*,*,*,解析,カイセキ,カイセキ
を	助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
行う	動詞,自立,*,*,五段・ワ行促音便,基本形,行う,オコナウ,オコナウ
EOS
```

ファイルフォーマットや高度な機能の詳細については、[TRAINER_README.md](../TRAINER_README.md) を参照してください。

#### 2. モデルの学習

```bash
lindera train \
  --seed ./resources/training/seed.csv \
  --corpus ./resources/training/corpus.txt \
  --unk-def ./resources/training/unk.def \
  --char-def ./resources/training/char.def \
  --feature-def ./resources/training/feature.def \
  --rewrite-def ./resources/training/rewrite.def \
  --output /tmp/lindera/training/model.dat \
  --lambda 0.01 \
  --max-iterations 100
```

#### 3. 学習結果

学習済みモデルには以下が含まれます：

- **既存単語**: 新しく学習された重みを持つすべてのシード辞書レコード
- **新語**: シード辞書にはないがコーパスに含まれる単語（適切な重み付きで追加）

## 学習済みモデルの辞書へのエクスポート

学習済みモデルファイルをLindera辞書フォーマットのファイルにエクスポートします。この機能を使用するには、`train` 機能フラグを有効にしてビルドする必要があります。

### 基本的なエクスポート方法

```bash
# 学習済みモデルを辞書ファイルにエクスポート
lindera export \
  --model /tmp/lindera/training/model.dat \
  --metadata ./resources/training/metadata.json \
  --output /tmp/lindera/training/dictionary
```

### エクスポートパラメータ

- `--model` / `-m`: 学習済みモデルファイル（.dat形式）のパス
- `--output` / `-o`: 辞書ファイルの出力先ディレクトリ
- `--metadata`: オプションの metadata.json ファイル（学習済みモデル情報で更新されます）

### 出力ファイル

エクスポートコマンドは出力ディレクトリに以下の辞書ファイルを作成します：

- `lex.csv`: 学習された重みを持つ語彙ファイル
- `matrix.def`: 接続コスト行列
- `unk.def`: 未知語定義
- `char.def`: 文字種定義
- `metadata.json`: 更新されたメタデータファイル（`--metadata` オプションが指定された場合）

### 完全なワークフロー例

#### 1. モデルの学習

```bash
lindera train \
  --seed ./resources/training/seed.csv \
  --corpus ./resources/training/corpus.txt \
  --unk-def ./resources/training/unk.def \
  --char-def ./resources/training/char.def \
  --feature-def ./resources/training/feature.def \
  --rewrite-def ./resources/training/rewrite.def \
  --output /tmp/lindera/training/model.dat \
  --lambda 0.01 \
  --max-iterations 100
```

#### 2. 辞書フォーマットへのエクスポート

```bash
lindera export \
  --model /tmp/lindera/training/model.dat \
  --metadata ./resources/training/metadata.json \
  --output /tmp/lindera/training/dictionary
```

#### 3. 辞書のビルド

```bash
lindera build \
  --src /tmp/lindera/training/dictionary \
  --dest /tmp/lindera/training/compiled_dictionary \
  --metadata /tmp/lindera/training/dictionary/metadata.json
```

#### 4. 学習済み辞書の使用

```bash
echo "これは外国人参政権です。" | lindera tokenize \
  -d /tmp/lindera/training/compiled_dictionary
```

### メタデータ更新機能

`--metadata` オプションが指定されると、エクスポートコマンドは以下の処理を行います：

1. **ベースとなる metadata.json ファイルを読み込み**、既存の設定を保持します
2. **特定のフィールドを学習済みモデルの値で更新**します：
   - `default_left_context_id`: 学習済みモデルからの最大左文脈ID
   - `default_right_context_id`: 学習済みモデルからの最大右文脈ID
   - `default_word_cost`: 素性重みの中央値から計算された値
   - `model_info`: 以下の学習統計情報を含む：
     - `feature_count`: モデル内の素性数
     - `label_count`: モデル内のラベル数
     - `max_left_context_id`: 最大左文脈ID
     - `max_right_context_id`: 最大右文脈ID
     - `connection_matrix_size`: 接続コスト行列のサイズ
     - `training_iterations`: 実行された学習反復回数
     - `regularization`: 使用されたL1正則化パラメータ
     - `version`: モデルバージョン
     - `updated_at`: モデルがエクスポートされたタイムスタンプ

3. **既存の設定を保持**します（以下など）：
   - 辞書名
   - 文字エンコード設定
   - スキーマ定義
   - その他のユーザー定義設定

これにより、基本となる辞書設定を維持しながら、学習中に最適化されたパラメータを取り込むことができます。

## APIリファレンス

APIリファレンスは以下で公開されています：

- [lindera-cli](https://docs.rs/lindera-cli)
