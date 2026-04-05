# Installation

> [!NOTE]
> lindera-ruby is not yet published to RubyGems. You need to build from source.

## Prerequisites

- **Ruby 3.1 or later**
- **Rust toolchain** -- Install via [rustup](https://rustup.rs/)
- **Bundler** -- Ruby dependency manager (`gem install bundler`)

## Obtaining Dictionaries

Lindera does not bundle dictionaries with the package. You need to obtain a pre-built dictionary separately.

### Download from GitHub Releases

Pre-built dictionaries are available on the [GitHub Releases](https://github.com/lindera/lindera/releases) page. Download and extract the dictionary archive to a local directory:

```bash
# Example: download and extract the IPADIC dictionary
curl -LO https://github.com/lindera/lindera/releases/download/<version>/lindera-ipadic-<version>.zip
unzip lindera-ipadic-<version>.zip -d /path/to/ipadic
```

## Development Build

Build and install lindera-ruby in development mode:

```bash
cd lindera-ruby
bundle install
bundle exec rake compile
```

Or use the project Makefile:

```bash
make ruby-develop
```

### Build with Training Support

The `train` feature enables CRF-based dictionary training functionality:

```bash
LINDERA_FEATURES="train" bundle exec rake compile
```

## Feature Flags

Features are specified through the `LINDERA_FEATURES` environment variable as a comma-separated list.

| Feature | Description | Default |
| --- | --- | --- |
| `train` | CRF training functionality | Disabled |
| `embed-ipadic` | Embed Japanese dictionary (IPADIC) into the binary | Disabled |
| `embed-unidic` | Embed Japanese dictionary (UniDic) into the binary | Disabled |
| `embed-ipadic-neologd` | Embed Japanese dictionary (IPADIC NEologd) into the binary | Disabled |
| `embed-ko-dic` | Embed Korean dictionary (ko-dic) into the binary | Disabled |
| `embed-cc-cedict` | Embed Chinese dictionary (CC-CEDICT) into the binary | Disabled |
| `embed-jieba` | Embed Chinese dictionary (Jieba) into the binary | Disabled |
| `embed-cjk` | Embed all CJK dictionaries (IPADIC, ko-dic, Jieba) into the binary | Disabled |

Multiple features can be combined:

```bash
LINDERA_FEATURES="train,embed-ipadic,embed-ko-dic" bundle exec rake compile
```

> [!TIP]
> If you want to embed a dictionary directly into the binary (advanced usage), enable the corresponding `embed-*` feature flag and load it using the `embedded://` scheme:
>
> ```ruby
> dictionary = Lindera.load_dictionary("embedded://ipadic")
> ```
>
> See [Feature Flags](../development/feature_flags.md) for details.

## Verifying the Installation

After installation, verify that lindera is available in Ruby:

```ruby
require 'lindera'

puts Lindera.version
```
