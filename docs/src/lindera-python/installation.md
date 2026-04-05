# Installation

## Installing from PyPI

Pre-built wheels are available on [PyPI](https://pypi.org/project/lindera-python/):

```bash
pip install lindera-python
```

> [!NOTE]
> The PyPI package does not include dictionaries. See [Obtaining Dictionaries](#obtaining-dictionaries) below.

## Obtaining Dictionaries

Lindera does not bundle dictionaries with the package. You need to obtain a pre-built dictionary separately.

### Download from GitHub Releases

Pre-built dictionaries are available on the [GitHub Releases](https://github.com/lindera/lindera/releases) page. Download and extract the dictionary archive to a local directory:

```bash
# Example: download and extract the IPADIC dictionary
curl -LO https://github.com/lindera/lindera/releases/download/<version>/lindera-ipadic-<version>.zip
unzip lindera-ipadic-<version>.zip -d /path/to/ipadic
```

## Building from Source

If you need to build from source (e.g., to enable specific feature flags), the following prerequisites are required:

- **Python 3.10 or later** (up to 3.14)
- **Rust toolchain** -- Install via [rustup](https://rustup.rs/)
- **maturin** -- Python package for building Rust-based Python extensions

Install maturin with pip:

```bash
pip install maturin
```

### Development Build

Build and install lindera-python in development mode:

```bash
cd lindera-python
maturin develop
```

Or use the project Makefile:

```bash
make python-develop
```

#### Build with Training Support

The `train` feature enables CRF-based dictionary training functionality. It is enabled by default:

```bash
maturin develop --features train
```

## Feature Flags

| Feature | Description | Default |
| --- | --- | --- |
| `train` | CRF training functionality | Enabled |
| `embed-ipadic` | Embed Japanese dictionary (IPADIC) into the binary | Disabled |
| `embed-unidic` | Embed Japanese dictionary (UniDic) into the binary | Disabled |
| `embed-ipadic-neologd` | Embed Japanese dictionary (IPADIC NEologd) into the binary | Disabled |
| `embed-ko-dic` | Embed Korean dictionary (ko-dic) into the binary | Disabled |
| `embed-cc-cedict` | Embed Chinese dictionary (CC-CEDICT) into the binary | Disabled |
| `embed-jieba` | Embed Chinese dictionary (Jieba) into the binary | Disabled |
| `embed-cjk` | Embed all CJK dictionaries (IPADIC, ko-dic, Jieba) into the binary | Disabled |

Multiple features can be combined:

```bash
maturin develop --features "train,embed-ipadic,embed-ko-dic"
```

> [!TIP]
> If you want to embed a dictionary directly into the binary (advanced usage), enable the corresponding `embed-*` feature flag and load it using the `embedded://` scheme:
>
> ```python
> dictionary = load_dictionary("embedded://ipadic")
> ```
>
> See [Feature Flags](../development/feature_flags.md) for details.

## Verifying the Installation

After installation, verify that lindera is available in Python:

```python
import lindera

print(lindera.version())
```
