# Installation

## Prerequisites

- **Python 3.10 or later** (up to 3.14)
- **Rust toolchain** -- Install via [rustup](https://rustup.rs/)
- **maturin** -- Python package for building Rust-based Python extensions

Install maturin with pip:

```bash
pip install maturin
```

## Development Build

Build and install lindera-python in development mode:

```bash
cd lindera-python
maturin develop
```

Or use the project Makefile:

```bash
make python-develop
```

### Build with Training Support

The `train` feature enables CRF-based dictionary training functionality. It is enabled by default:

```bash
maturin develop --features train
```

### Build with Embedded Dictionaries

Embed dictionaries directly into the binary so no external dictionary files are needed at runtime:

```bash
maturin develop --features embed-ipadic
```

## Feature Flags

| Feature | Description | Default |
| --- | --- | --- |
| `train` | CRF training functionality | Enabled |
| `embed-ipadic` | Embed Japanese dictionary (IPADIC) | Disabled |
| `embed-unidic` | Embed Japanese dictionary (UniDic) | Disabled |
| `embed-ipadic-neologd` | Embed Japanese dictionary (IPADIC NEologd) | Disabled |
| `embed-ko-dic` | Embed Korean dictionary (ko-dic) | Disabled |
| `embed-cc-cedict` | Embed Chinese dictionary (CC-CEDICT) | Disabled |
| `embed-jieba` | Embed Chinese dictionary (Jieba) | Disabled |
| `embed-cjk` | Embed all CJK dictionaries (IPADIC, ko-dic, Jieba) | Disabled |

Multiple features can be combined:

```bash
maturin develop --features "train,embed-ipadic,embed-ko-dic"
```

## Verifying the Installation

After installation, verify that lindera is available in Python:

```python
import lindera

print(lindera.version())
```
