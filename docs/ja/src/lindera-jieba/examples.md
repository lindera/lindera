# 使用例

## 外部 Jieba でトークナイズ

```shell
% echo "可以进行中文形态学分析。" | lindera tokenize \
  --dict /tmp/lindera-jieba-0.1.0-20260310
```

## 埋め込み Jieba でトークナイズ

```shell
% echo "可以进行中文形态学分析。" | lindera tokenize \
  --dict embedded://jieba
```

注意: Jieba 辞書をバイナリに含めるには、`--features=embed-jieba` オプションを付けてビルドする必要があります。
