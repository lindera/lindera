# 使用例

## 外部 ko-dic でトークナイズ

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
할      XSV+ETM,*,T,할,Inflect,XSV,ETM,하/XSV/*+ᆯ/ETM/*
수      NNB,*,F,수,*,*,*,*
있      VV,*,T,있,*,*,*,*
습니다  EF,*,F,습니다,*,*,*,*
.       SF,*,*,*,*,*,*,*
EOS
```

## 埋め込み ko-dic でトークナイズ

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
할      XSV+ETM,*,T,할,Inflect,XSV,ETM,하/XSV/*+ᆯ/ETM/*
수      NNB,*,F,수,*,*,*,*
있      VV,*,T,있,*,*,*,*
습니다  EF,*,F,습니다,*,*,*,*
.       SF,*,*,*,*,*,*,*
EOS
```

注意: ko-dic 辞書をバイナリに含めるには、`--features=embed-ko-dic` オプションを付けてビルドする必要があります。
