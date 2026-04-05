# Installation

## Prerequisites

- **Ruby 3.1 or later**
- **Rust toolchain** -- Install via [rustup](https://rustup.rs/)
- **Bundler** -- Ruby dependency manager (`gem install bundler`)

## Development Build

Build and install lindera-ruby in development mode:

```bash
cd lindera-ruby
bundle install
LINDERA_FEATURES="embed-ipadic" bundle exec rake compile
```

Or use the project Makefile:

```bash
make ruby-develop
```

### Build with Training Support

The `train` feature enables CRF-based dictionary training functionality:

```bash
LINDERA_FEATURES="embed-ipadic,train" bundle exec rake compile
```

### Build with Embedded Dictionaries

Embed dictionaries directly into the binary so no external dictionary files are needed at runtime:

```bash
LINDERA_FEATURES="embed-ipadic" bundle exec rake compile
```

## Feature Flags

Features are specified through the `LINDERA_FEATURES` environment variable as a comma-separated list.

| Feature | Description | Default |
| --- | --- | --- |
| `train` | CRF training functionality | Disabled |
| `embed-ipadic` | Embed Japanese dictionary (IPADIC) | Disabled |
| `embed-unidic` | Embed Japanese dictionary (UniDic) | Disabled |
| `embed-ipadic-neologd` | Embed Japanese dictionary (IPADIC NEologd) | Disabled |
| `embed-ko-dic` | Embed Korean dictionary (ko-dic) | Disabled |
| `embed-cc-cedict` | Embed Chinese dictionary (CC-CEDICT) | Disabled |
| `embed-jieba` | Embed Chinese dictionary (Jieba) | Disabled |
| `embed-cjk` | Embed all CJK dictionaries (IPADIC, ko-dic, Jieba) | Disabled |

Multiple features can be combined:

```bash
LINDERA_FEATURES="train,embed-ipadic,embed-ko-dic" bundle exec rake compile
```

## Verifying the Installation

After installation, verify that lindera is available in Ruby:

```ruby
require 'lindera'

puts Lindera.version
```
