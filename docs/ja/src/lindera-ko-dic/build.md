# ビルド

## システム辞書のビルド

mecab-ko-dic のソースファイルをダウンロード・展開し、辞書をビルドします:

```shell
% curl -L -o /tmp/mecab-ko-dic-2.1.1-20180720.tar.gz "https://Lindera.dev/mecab-ko-dic-2.1.1-20180720.tar.gz"
% tar zxvf /tmp/mecab-ko-dic-2.1.1-20180720.tar.gz -C /tmp
% lindera build \
  --src /tmp/mecab-ko-dic-2.1.1-20180720 \
  --dest /tmp/lindera-ko-dic-2.1.1-20180720 \
  --metadata ./lindera-ko-dic/metadata.json \
  --context-id-freq ./lindera-ko-dic/context_id_freq.txt
```

> [!TIP]
> `lindera-ko-dic/metadata.json` は `connection_id_mapping: true` を設定しているため、ビルダーは
> 連接コスト行列の文脈 ID を使用頻度順に付け替え、連接コスト参照時のキャッシュ局所性を改善します。
> `--context-id-freq` / `-f` に同梱の `context_id_freq.txt` ヒストグラムを渡すことで、この付け替えに
> 実際のコーパス頻度データを与えて ID をランク付けできます。このフラグを省略してもビルドは失敗せず、
> 精度の低いエントリ数ベースのフォールバックに黙って切り替わるだけです。いずれの場合もトークン化の
> 結果には影響しません -- この付け替えはコストを保つ全単射な再ラベル付けであり、ビルド時の最適化のみに
> 関わるものであって正確性には影響しません。

## ユーザー辞書のビルド

```shell
% lindera build \
  --src ./resources/user_dict/ko-dic_simple_userdic.csv \
  --dest ./resources/user_dict \
  --metadata ./lindera-ko-dic/metadata.json \
  --user
```

## 辞書の埋め込み

ko-dic 辞書をバイナリに直接埋め込むには、以下の feature フラグを付けてビルドします:

```shell
% cargo build --features=embed-ko-dic
```
