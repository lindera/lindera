# Lindera

[English](README.md) · [日本語](README_ja.md) · **Türkçe**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Crates.io](https://img.shields.io/crates/v/lindera.svg)](https://crates.io/crates/lindera)

Rust ile yazılmış bir biçimbilimsel (morfolojik) çözümleme kütüphanesi. Bu proje [kuromoji-rs](https://github.com/fulmicoton/kuromoji-rs) deposundan çatallanmıştır.

Lindera, kurulumu kolay ve çeşitli Rust uygulamaları için özlü API'ler sunan bir kütüphane geliştirmeyi amaçlar.

## Belgeler

- [İngilizce Belgeler](https://lindera.github.io/lindera/)
- [Japonca Belgeler (日本語ドキュメント)](https://lindera.github.io/lindera/ja/)

```toml
[dependencies]
lindera = "7"
```

> **Not:** v6 sürümünden mi yükseltiyorsunuz? Ayrıntılar için [geçiş kılavuzuna](https://lindera.github.io/lindera/migration_v6_to_v7.html) bakın.

Önceden derlenmiş sözlükler [GitHub Releases](https://github.com/lindera/lindera/releases) sayfasında mevcuttur. Bir sözlük arşivi indirin (örn. `lindera-ipadic-*.zip`) ve yüklerken yolunu belirtin. CLI kullanırken `lindera download ipadic` komutu sözlüğü otomatik olarak indirip kurar.

## Performans

Lindera, donanıma ve Viterbi kafesinin çağrılar arasında yeniden kullanılıp kullanılmamasına bağlı olarak Japonca metinleri (IPADIC) tek iş parçacığında yaklaşık 10–20 MB/sn hızla belirteçlerine ayırır. Karşılaştırma paketini (`lindera/benches/`) kendi makinenizde yeniden üretmek için `make bench` çalıştırın; ayrıntılı yöntem ve karşılaştırma sayıları için [#875](https://github.com/lindera/lindera/issues/875) kaydına bakın.

## Python Bağlamaları

Lindera ayrıca Python bağlamaları da sunar. pip ile kurabilirsiniz:

```bash
pip install lindera
```

Daha fazla ayrıntı için [lindera-python](lindera-python/) dizinine bakın.

## Node.js Bağlamaları

Lindera ayrıca Node.js bağlamaları da sunar. npm ile kurabilirsiniz:

```bash
npm install lindera
```

Daha fazla ayrıntı için [lindera-nodejs](lindera-nodejs/) dizinine bakın.

## Ruby Bağlamaları

Lindera ayrıca Ruby bağlamaları da sunar. v6.1.0 sürümünden itibaren RubyGems üzerinden `lindera` adıyla kurulur. Gem kurulum sırasında yerel uzantısını derlediği için bir Rust araç zinciri ve libclang gerektirir:

```bash
gem install lindera
```

Daha fazla ayrıntı için [lindera-ruby](lindera-ruby/) dizinine bakın.

## PHP Bağlamaları

Lindera ayrıca bir PHP eklentisi olarak PHP bağlamaları sunar. v6.1.0 sürümünden itibaren Packagist üzerinde `lindera/lindera` adıyla yayınlanır ve [PIE](https://github.com/php/pie) ile kurulur:

```bash
pie install lindera/lindera
```

Ayrıntılar için [lindera-php](lindera-php/) dizinine bakın.

## WebAssembly Bağlamaları

Lindera ayrıca WebAssembly bağlamaları da sunar. npm ile kurabilirsiniz:

```bash
npm install lindera-wasm
```

Paket wasm-pack'in `web` hedefiyle derlenir; tarayıcılarda yerel olarak çalışabildiği gibi modern paketleyicilerle (Vite, Webpack 5) de uyumludur.

Daha fazla ayrıntı ve demo uygulama için [lindera-wasm](lindera-wasm/) dizinine bakın.

## Lisans

MIT
