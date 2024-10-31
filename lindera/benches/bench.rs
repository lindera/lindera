use criterion::{criterion_group, criterion_main, Criterion};

#[allow(unused_variables)]
fn bench_constructor(c: &mut Criterion) {
    #[cfg(feature = "ipadic")]
    {
        use lindera::dictionary::DictionaryKind;
        use lindera::mode::Mode;
        use lindera::tokenizer::{Tokenizer, TokenizerConfigBuilder};

        c.bench_function("bench-constructor-ipadic", |b| {
            b.iter(|| {
                let mut config_builder = TokenizerConfigBuilder::new();
                config_builder.set_segmenter_dictionary_kind(&DictionaryKind::IPADIC);
                config_builder.set_segmenter_mode(&Mode::Normal);

                // Create a tokenizer.
                let tokenizer = Tokenizer::from_config(&config_builder.build()).unwrap();
            })
        });
    }

    #[cfg(feature = "unidic")]
    {
        use lindera::dictionary::DictionaryKind;
        use lindera::mode::Mode;
        use lindera::tokenizer::{Tokenizer, TokenizerConfigBuilder};

        c.bench_function("bench-constructor-unidic", |b| {
            b.iter(|| {
                let mut config_builder = TokenizerConfigBuilder::new();
                config_builder.set_segmenter_dictionary_kind(&DictionaryKind::UniDic);
                config_builder.set_segmenter_mode(&Mode::Normal);

                // Create a tokenizer.
                let tokenizer = Tokenizer::from_config(&config_builder.build()).unwrap();
            })
        });
    }

    #[cfg(feature = "ko-dic")]
    {
        use lindera::dictionary::DictionaryKind;
        use lindera::mode::Mode;
        use lindera::tokenizer::{Tokenizer, TokenizerConfigBuilder};

        c.bench_function("bench-constructor-ko-dic", |b| {
            b.iter(|| {
                let mut config_builder = TokenizerConfigBuilder::new();
                config_builder.set_segmenter_dictionary_kind(&DictionaryKind::KoDic);
                config_builder.set_segmenter_mode(&Mode::Normal);

                // Create a tokenizer.
                let tokenizer = Tokenizer::from_config(&config_builder.build()).unwrap();
            })
        });
    }

    #[cfg(feature = "cc-cedict")]
    {
        use lindera::dictionary::DictionaryKind;
        use lindera::mode::Mode;
        use lindera::tokenizer::{Tokenizer, TokenizerConfigBuilder};

        c.bench_function("bench-constructor-cc-cedict", |b| {
            b.iter(|| {
                let mut config_builder = TokenizerConfigBuilder::new();
                config_builder.set_segmenter_dictionary_kind(&DictionaryKind::CcCedict);
                config_builder.set_segmenter_mode(&Mode::Normal);

                // Create a tokenizer.
                let tokenizer = Tokenizer::from_config(&config_builder.build()).unwrap();
            })
        });
    }
}

#[allow(unused_variables)]
fn bench_constructor_with_simple_userdic(c: &mut Criterion) {
    #[cfg(feature = "ipadic")]
    {
        use std::path::PathBuf;

        use lindera::dictionary::DictionaryKind;
        use lindera::mode::Mode;
        use lindera::tokenizer::{Tokenizer, TokenizerConfigBuilder};

        c.bench_function("bench-constructor-simple-userdic-ipadic", |b| {
            b.iter(|| {
                let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../resources")
                    .join("ipadic_simple_userdic.csv");

                let mut config_builder = TokenizerConfigBuilder::new();
                config_builder.set_segmenter_dictionary_kind(&DictionaryKind::IPADIC);
                config_builder.set_segmenter_mode(&Mode::Normal);
                config_builder.set_segmenter_user_dictionary_path(&userdic_file);
                config_builder.set_segmenter_user_dictionary_kind(&DictionaryKind::IPADIC);

                // Create a tokenizer.
                let tokenizer = Tokenizer::from_config(&config_builder.build()).unwrap();
            })
        });
    }

    #[cfg(feature = "unidic")]
    {
        use std::path::PathBuf;

        use lindera::dictionary::DictionaryKind;
        use lindera::mode::Mode;
        use lindera::tokenizer::{Tokenizer, TokenizerConfigBuilder};

        c.bench_function("bench-constructor-simple-userdic-unidic", |b| {
            b.iter(|| {
                let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../resources")
                    .join("unidic_simple_userdic.csv");

                let mut config_builder = TokenizerConfigBuilder::new();
                config_builder.set_segmenter_dictionary_kind(&DictionaryKind::UniDic);
                config_builder.set_segmenter_mode(&Mode::Normal);
                config_builder.set_segmenter_user_dictionary_path(&userdic_file);
                config_builder.set_segmenter_user_dictionary_kind(&DictionaryKind::UniDic);

                // Create a tokenizer.
                let tokenizer = Tokenizer::from_config(&config_builder.build()).unwrap();
            })
        });
    }

    #[cfg(feature = "ko-dic")]
    {
        use std::path::PathBuf;

        use lindera::dictionary::DictionaryKind;
        use lindera::mode::Mode;
        use lindera::tokenizer::{Tokenizer, TokenizerConfigBuilder};

        c.bench_function("bench-constructor-simple-userdic-ko-dic", |b| {
            b.iter(|| {
                let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../resources")
                    .join("ko-dic_simple_userdic.csv");

                let mut config_builder = TokenizerConfigBuilder::new();
                config_builder.set_segmenter_dictionary_kind(&DictionaryKind::KoDic);
                config_builder.set_segmenter_mode(&Mode::Normal);
                config_builder.set_segmenter_user_dictionary_path(&userdic_file);
                config_builder.set_segmenter_user_dictionary_kind(&DictionaryKind::KoDic);

                // Create a tokenizer.
                let tokenizer = Tokenizer::from_config(&config_builder.build()).unwrap();
            })
        });
    }

    #[cfg(feature = "cc-cedict")]
    {
        use std::path::PathBuf;

        use lindera::dictionary::DictionaryKind;
        use lindera::mode::Mode;
        use lindera::tokenizer::{Tokenizer, TokenizerConfigBuilder};

        c.bench_function("bench-constructor-simple-userdic-cc-cedict", |b| {
            b.iter(|| {
                let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../resources")
                    .join("cc-cedict_simple_userdic.csv");

                let mut config_builder = TokenizerConfigBuilder::new();
                config_builder.set_segmenter_dictionary_kind(&DictionaryKind::CcCedict);
                config_builder.set_segmenter_mode(&Mode::Normal);
                config_builder.set_segmenter_user_dictionary_path(&userdic_file);
                config_builder.set_segmenter_user_dictionary_kind(&DictionaryKind::CcCedict);

                // Create a tokenizer.
                let tokenizer = Tokenizer::from_config(&config_builder.build()).unwrap();
            })
        });
    }
}

#[allow(unused_variables)]
fn bench_tokenize(c: &mut Criterion) {
    #[cfg(feature = "ipadic")]
    {
        use lindera::dictionary::DictionaryKind;
        use lindera::mode::Mode;
        use lindera::tokenizer::{Tokenizer, TokenizerConfigBuilder};

        let mut config_builder = TokenizerConfigBuilder::new();
        config_builder.set_segmenter_dictionary_kind(&DictionaryKind::IPADIC);
        config_builder.set_segmenter_mode(&Mode::Normal);

        // Create a tokenizer.
        let tokenizer = Tokenizer::from_config(&config_builder.build()).unwrap();

        c.bench_function("bench-tokenize-ipadic", |b| {
            b.iter(|| tokenizer.tokenize("検索エンジン（けんさくエンジン、英語: search engine）は、狭義にはインターネットに存在する情報（ウェブページ、ウェブサイト、画像ファイル、ネットニュースなど）を検索する機能およびそのプログラム。"))
        });
    }

    #[cfg(feature = "unidic")]
    {
        use lindera::dictionary::DictionaryKind;
        use lindera::mode::Mode;
        use lindera::tokenizer::{Tokenizer, TokenizerConfigBuilder};

        let mut config_builder = TokenizerConfigBuilder::new();
        config_builder.set_segmenter_dictionary_kind(&DictionaryKind::UniDic);
        config_builder.set_segmenter_mode(&Mode::Normal);

        // Create a tokenizer.
        let tokenizer = Tokenizer::from_config(&config_builder.build()).unwrap();

        c.bench_function("bench-tokenize-unidic", |b| {
            b.iter(|| tokenizer.tokenize("検索エンジン（けんさくエンジン、英語: search engine）は、狭義にはインターネットに存在する情報（ウェブページ、ウェブサイト、画像ファイル、ネットニュースなど）を検索する機能およびそのプログラム。"))
        });
    }

    #[cfg(feature = "ko-dic")]
    {
        use lindera::dictionary::DictionaryKind;
        use lindera::mode::Mode;
        use lindera::tokenizer::{Tokenizer, TokenizerConfigBuilder};

        let mut config_builder = TokenizerConfigBuilder::new();
        config_builder.set_segmenter_dictionary_kind(&DictionaryKind::KoDic);
        config_builder.set_segmenter_mode(&Mode::Normal);

        // Create a tokenizer.
        let tokenizer = Tokenizer::from_config(&config_builder.build()).unwrap();

        c.bench_function("bench-tokenize-ko-dic", |b| {
            b.iter(|| tokenizer.tokenize("검색엔진(search engine)은컴퓨터시스템에저장된정보를찾아주거나웹검색(web search query)을도와주도록설계된정보검색시스템또는컴퓨터프로그램이다. 이러한검색결과는목록으로표시되는것이보통이다."))
        });
    }

    #[cfg(feature = "cc-cedict")]
    {
        use lindera::dictionary::DictionaryKind;
        use lindera::mode::Mode;
        use lindera::tokenizer::{Tokenizer, TokenizerConfigBuilder};

        let mut config_builder = TokenizerConfigBuilder::new();
        config_builder.set_segmenter_dictionary_kind(&DictionaryKind::CcCedict);
        config_builder.set_segmenter_mode(&Mode::Normal);

        // Create a tokenizer.
        let tokenizer = Tokenizer::from_config(&config_builder.build()).unwrap();

        c.bench_function("bench-tokenize-cc-cedict", |b| {
            b.iter(|| tokenizer.tokenize("搜索引擎（英語：search engine）是一种信息检索系统，旨在协助搜索存储在计算机系统中的信息。搜索结果一般被称为“hits”，通常会以表单的形式列出。网络搜索引擎是最常见、公开的一种搜索引擎，其功能为搜索万维网上储存的信息。"))
        });
    }
}

#[allow(unused_variables)]
fn bench_tokenize_with_simple_userdic(c: &mut Criterion) {
    #[cfg(feature = "ipadic")]
    {
        use std::path::PathBuf;

        use lindera::dictionary::DictionaryKind;
        use lindera::mode::Mode;
        use lindera::tokenizer::{Tokenizer, TokenizerConfigBuilder};

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ipadic_simple_userdic.csv");

        let mut config_builder = TokenizerConfigBuilder::new();
        config_builder.set_segmenter_dictionary_kind(&DictionaryKind::IPADIC);
        config_builder.set_segmenter_mode(&Mode::Normal);
        config_builder.set_segmenter_user_dictionary_path(&userdic_file);
        config_builder.set_segmenter_user_dictionary_kind(&DictionaryKind::IPADIC);

        // Create a tokenizer.
        let tokenizer = Tokenizer::from_config(&config_builder.build()).unwrap();

        c.bench_function("bench-tokenize-with-simple-userdic-ipadic", |b| {
            b.iter(|| {
                tokenizer.tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です")
            })
        });
    }

    #[cfg(feature = "unidic")]
    {
        use std::path::PathBuf;

        use lindera::dictionary::DictionaryKind;
        use lindera::mode::Mode;
        use lindera::tokenizer::{Tokenizer, TokenizerConfigBuilder};

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("unidic_simple_userdic.csv");

        let mut config_builder = TokenizerConfigBuilder::new();
        config_builder.set_segmenter_dictionary_kind(&DictionaryKind::UniDic);
        config_builder.set_segmenter_mode(&Mode::Normal);
        config_builder.set_segmenter_user_dictionary_path(&userdic_file);
        config_builder.set_segmenter_user_dictionary_kind(&DictionaryKind::UniDic);

        // Create a tokenizer.
        let tokenizer = Tokenizer::from_config(&config_builder.build()).unwrap();

        c.bench_function("bench-tokenize-with-simple-userdic-unidic", |b| {
            b.iter(|| {
                tokenizer.tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です")
            })
        });
    }

    #[cfg(feature = "ko-dic")]
    {
        use std::path::PathBuf;

        use lindera::dictionary::DictionaryKind;
        use lindera::mode::Mode;
        use lindera::tokenizer::{Tokenizer, TokenizerConfigBuilder};

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ko-dic_simple_userdic.csv");

        let mut config_builder = TokenizerConfigBuilder::new();
        config_builder.set_segmenter_dictionary_kind(&DictionaryKind::KoDic);
        config_builder.set_segmenter_mode(&Mode::Normal);
        config_builder.set_segmenter_user_dictionary_path(&userdic_file);
        config_builder.set_segmenter_user_dictionary_kind(&DictionaryKind::KoDic);

        // Create a tokenizer.
        let tokenizer = Tokenizer::from_config(&config_builder.build()).unwrap();

        c.bench_function("bench-tokenize-with-simple-userdic-ko-dic", |b| {
            b.iter(|| tokenizer.tokenize("하네다공항한정토트백."))
        });
    }

    #[cfg(feature = "cc-cedict")]
    {
        use std::path::PathBuf;

        use lindera::dictionary::DictionaryKind;
        use lindera::mode::Mode;
        use lindera::tokenizer::{Tokenizer, TokenizerConfigBuilder};

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("cc-cedict_simple_userdic.csv");

        let mut config_builder = TokenizerConfigBuilder::new();
        config_builder.set_segmenter_dictionary_kind(&DictionaryKind::CcCedict);
        config_builder.set_segmenter_mode(&Mode::Normal);
        config_builder.set_segmenter_user_dictionary_path(&userdic_file);
        config_builder.set_segmenter_user_dictionary_kind(&DictionaryKind::CcCedict);

        // Create a tokenizer.
        let tokenizer = Tokenizer::from_config(&config_builder.build()).unwrap();

        c.bench_function("bench-tokenize-with-simple-userdic-cc-cedict", |b| {
            b.iter(|| tokenizer.tokenize("羽田机场限定托特包。"))
        });
    }
}

#[allow(unused_variables)]
fn bench_tokenize_long_text(c: &mut Criterion) {
    #[cfg(feature = "ipadic")]
    {
        use std::fs::File;
        use std::io::BufReader;
        use std::io::Read;
        use std::path::PathBuf;

        use lindera::dictionary::DictionaryKind;
        use lindera::mode::Mode;
        use lindera::tokenizer::{Tokenizer, TokenizerConfigBuilder};

        let mut long_text_file = BufReader::new(
            File::open(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../resources")
                    .join("bocchan.txt"),
            )
            .unwrap(),
        );
        let mut long_text = String::new();
        let _size = long_text_file.read_to_string(&mut long_text).unwrap();

        let mut config_builder = TokenizerConfigBuilder::new();
        config_builder.set_segmenter_dictionary_kind(&DictionaryKind::IPADIC);
        config_builder.set_segmenter_mode(&Mode::Normal);

        // Create a tokenizer.
        let tokenizer = Tokenizer::from_config(&config_builder.build()).unwrap();

        // Using benchmark_group for changing sample_size
        let mut group = c.benchmark_group("tokenize-long-text-ipadic");
        group.sample_size(20);
        group.bench_function("bench-tokenize-long-text-ipadic", |b| {
            b.iter(|| tokenizer.tokenize(long_text.as_str()));
        });
        group.finish();
    }

    #[cfg(feature = "unidic")]
    {
        use std::fs::File;
        use std::io::BufReader;
        use std::io::Read;
        use std::path::PathBuf;

        use lindera::dictionary::DictionaryKind;
        use lindera::mode::Mode;
        use lindera::tokenizer::{Tokenizer, TokenizerConfigBuilder};

        let mut long_text_file = BufReader::new(
            File::open(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../resources")
                    .join("bocchan.txt"),
            )
            .unwrap(),
        );
        let mut long_text = String::new();
        let _size = long_text_file.read_to_string(&mut long_text).unwrap();

        let mut config_builder = TokenizerConfigBuilder::new();
        config_builder.set_segmenter_dictionary_kind(&DictionaryKind::UniDic);
        config_builder.set_segmenter_mode(&Mode::Normal);

        // Create a tokenizer.
        let tokenizer = Tokenizer::from_config(&config_builder.build()).unwrap();

        // Using benchmark_group for changing sample_size
        let mut group = c.benchmark_group("tokenize-long-text-unidic");
        group.sample_size(20);
        group.bench_function("bench-tokenize-long-text-unidic", |b| {
            b.iter(|| tokenizer.tokenize(long_text.as_str()));
        });
        group.finish();
    }
}

#[allow(unused_variables)]
fn bench_tokenize_details_long_text(c: &mut Criterion) {
    #[cfg(feature = "ipadic")]
    {
        use std::fs::File;
        use std::io::BufReader;
        use std::io::Read;
        use std::path::PathBuf;

        use lindera::dictionary::DictionaryKind;
        use lindera::mode::Mode;
        use lindera::tokenizer::{Tokenizer, TokenizerConfigBuilder};

        let mut long_text_file = BufReader::new(
            File::open(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../resources")
                    .join("bocchan.txt"),
            )
            .unwrap(),
        );
        let mut long_text = String::new();
        let _size = long_text_file.read_to_string(&mut long_text).unwrap();

        let mut config_builder = TokenizerConfigBuilder::new();
        config_builder.set_segmenter_dictionary_kind(&DictionaryKind::IPADIC);
        config_builder.set_segmenter_mode(&Mode::Normal);

        // Create a tokenizer.
        let tokenizer = Tokenizer::from_config(&config_builder.build()).unwrap();

        // Using benchmark_group for changing sample_size
        let mut group = c.benchmark_group("tokenize-details-long-text-ipadic");
        group.sample_size(20);
        group.bench_function("bench-tokenize-details-long-text-ipadic", |b| {
            b.iter(|| {
                let mut tokens = tokenizer.tokenize(long_text.as_str()).unwrap();
                for token in tokens.iter_mut() {
                    let _details = token.details();
                }
            });
        });
        group.finish();
    }

    #[cfg(feature = "unidic")]
    {
        use std::fs::File;
        use std::io::BufReader;
        use std::io::Read;
        use std::path::PathBuf;

        use lindera::dictionary::DictionaryKind;
        use lindera::mode::Mode;
        use lindera::tokenizer::{Tokenizer, TokenizerConfigBuilder};

        let mut long_text_file = BufReader::new(
            File::open(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../resources")
                    .join("bocchan.txt"),
            )
            .unwrap(),
        );
        let mut long_text = String::new();
        let _size = long_text_file.read_to_string(&mut long_text).unwrap();

        let mut config_builder = TokenizerConfigBuilder::new();
        config_builder.set_segmenter_dictionary_kind(&DictionaryKind::UniDic);
        config_builder.set_segmenter_mode(&Mode::Normal);

        // Create a tokenizer.
        let tokenizer = Tokenizer::from_config(&config_builder.build()).unwrap();

        // Using benchmark_group for changing sample_size
        let mut group = c.benchmark_group("tokenize-details-long-text-unidic");
        group.sample_size(20);
        group.bench_function("bench-tokenize-details-long-text-unidic", |b| {
            b.iter(|| {
                let mut tokens = tokenizer.tokenize(long_text.as_str()).unwrap();
                for token in tokens.iter_mut() {
                    let _details = token.details();
                }
            });
        });
        group.finish();
    }
}

criterion_group!(
    benches,
    bench_constructor,
    bench_constructor_with_simple_userdic,
    bench_tokenize,
    bench_tokenize_with_simple_userdic,
    bench_tokenize_long_text,
    bench_tokenize_details_long_text,
);
criterion_main!(benches);
