# Examples

## Tokenize with external Jieba

```shell
% echo "可以进行中文形态学分析。" | lindera tokenize \
  --dict /tmp/lindera-jieba-0.1.0-20260310
```

## Tokenize with embedded Jieba

```shell
% echo "可以进行中文形态学分析。" | lindera tokenize \
  --dict embedded://jieba
```

NOTE: To include Jieba dictionary in the binary, you must build with the `--features=embed-jieba` option.
