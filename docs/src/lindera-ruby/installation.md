# Installation

From v6.1.0, lindera-ruby installs from [RubyGems](https://rubygems.org/gems/lindera) as `lindera`. It is a source gem: `gem install` compiles the native extension from the bundled Rust source, which takes a few minutes.

## Prerequisites

- **Ruby 3.1 or later**, with the tools needed to build native gems (a C compiler and `make`)
- **Rust toolchain 1.88 or later** -- Install via [rustup](https://rustup.rs/)
- **libclang** -- `libclang-dev` on Debian/Ubuntu; the Xcode command line tools on macOS
- **Bundler** -- only to build from source (`gem install bundler`)

## Install from RubyGems

```bash
gem install lindera
```

Or add it to your `Gemfile`:

```ruby
gem "lindera"
```

> [!NOTE]
> Use lindera v6.1.0 or later. The `lindera` gems published before v6.1.0 fail to compile during installation; for those versions, build from source as described below.

The gem compiles against the [crates.io](https://crates.io/crates/lindera) releases of `lindera` and `lindera-binding-core` of the same version, which it pins exactly. It ships no `Cargo.lock`, so their dependencies, including the other `lindera-*` crates, resolve to the latest compatible releases at install time. It builds with the default features (`train`) and no embedded dictionary; see [Obtaining Dictionaries](#obtaining-dictionaries). To enable other [feature flags](#feature-flags), set `LINDERA_FEATURES` when installing:

```bash
LINDERA_FEATURES="embed-ipadic" gem install lindera
```

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
make build-lindera-ruby
```

Run `make test-lindera-ruby` to run both the Rust unit tests and the Ruby
minitest suite.

### Building the Gem

`bundle exec rake build` (or `make package-lindera-ruby`) writes the source gem
to `lindera-ruby/pkg/`. It packages the crate as `cargo package` normalizes it,
so the gem compiles outside this repository, against the crates.io releases of
`lindera` and `lindera-binding-core` of the same version. Do not run `gem build`
directly: the crate's own `Cargo.toml` only resolves inside the Cargo workspace.

`make test-lindera-ruby-gem` (requires Docker) builds the gem, installs it with
`gem install` in a clean `ruby` container and tokenizes text with it. The test
compiles the `lindera` crates from the checkout, so it also works for versions
not yet released on crates.io.

### Build with Training Support

The `train` feature enables CRF-based dictionary training functionality. It is enabled by default:

```bash
LINDERA_FEATURES="train" bundle exec rake compile
```

## Feature Flags

Features are specified through the `LINDERA_FEATURES` environment variable as a comma-separated list, both for `bundle exec rake compile` and for `gem install`.

| Feature | Description | Default |
| --- | --- | --- |
| `train` | CRF training functionality | Enabled |
| `embed-ipadic` | Embed Japanese dictionary (IPADIC) into the binary | Disabled |
| `embed-unidic` | Embed Japanese dictionary (UniDic) into the binary | Disabled |
| `embed-sudachidict` | Embed Japanese dictionary (SudachiDict) into the binary | Disabled |
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
