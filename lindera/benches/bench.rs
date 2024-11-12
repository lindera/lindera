use criterion::{criterion_group, criterion_main, Criterion};

#[allow(unused_variables)]
fn bench_constructor(c: &mut Criterion) {
    #[cfg(feature = "ipadic")]
    {
        use lindera::dictionary::load_dictionary_from_kind;
        use lindera::dictionary::DictionaryKind;
        use lindera::mode::Mode;
        use lindera::segmenter::Segmenter;
        use lindera::tokenizer::Tokenizer;

        c.bench_function("bench-constructor-ipadic", |b| {
            b.iter(|| {
                let dictionary = load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();
                let segmenter = Segmenter::new(
                    Mode::Normal,
                    dictionary,
                    None, // Assuming no user dictionary is provided
                );

                // Create a tokenizer.
                let tokenizer = Tokenizer::new(segmenter);
            })
        });
    }

    #[cfg(feature = "unidic")]
    {
        use lindera::dictionary::load_dictionary_from_kind;
        use lindera::dictionary::DictionaryKind;
        use lindera::mode::Mode;
        use lindera::segmenter::Segmenter;
        use lindera::tokenizer::Tokenizer;

        c.bench_function("bench-constructor-unidic", |b| {
            b.iter(|| {
                let dictionary = load_dictionary_from_kind(DictionaryKind::UniDic).unwrap();
                let segmenter = Segmenter::new(
                    Mode::Normal,
                    dictionary,
                    None, // Assuming no user dictionary is provided
                );

                // Create a tokenizer.
                let tokenizer = Tokenizer::new(segmenter);
            })
        });
    }

    #[cfg(feature = "ko-dic")]
    {
        use lindera::dictionary::load_dictionary_from_kind;
        use lindera::dictionary::DictionaryKind;
        use lindera::mode::Mode;
        use lindera::segmenter::Segmenter;
        use lindera::tokenizer::Tokenizer;

        c.bench_function("bench-constructor-ko-dic", |b| {
            b.iter(|| {
                let dictionary = load_dictionary_from_kind(DictionaryKind::KoDic).unwrap();
                let segmenter = Segmenter::new(
                    Mode::Normal,
                    dictionary,
                    None, // Assuming no user dictionary is provided
                );

                // Create a tokenizer.
                let tokenizer = Tokenizer::new(segmenter);
            })
        });
    }

    #[cfg(feature = "cc-cedict")]
    {
        use lindera::dictionary::load_dictionary_from_kind;
        use lindera::dictionary::DictionaryKind;
        use lindera::mode::Mode;
        use lindera::segmenter::Segmenter;
        use lindera::tokenizer::Tokenizer;

        c.bench_function("bench-constructor-cc-cedict", |b| {
            b.iter(|| {
                let dictionary = load_dictionary_from_kind(DictionaryKind::CcCedict).unwrap();
                let segmenter = Segmenter::new(
                    Mode::Normal,
                    dictionary,
                    None, // Assuming no user dictionary is provided
                );

                // Create a tokenizer.
                let tokenizer = Tokenizer::new(segmenter);
            })
        });
    }
}

#[allow(unused_variables)]
fn bench_constructor_with_simple_userdic(c: &mut Criterion) {
    #[cfg(feature = "ipadic")]
    {
        use std::path::PathBuf;

        use lindera::dictionary::{
            load_dictionary_from_kind, load_user_dictionary_from_csv, DictionaryKind,
        };
        use lindera::mode::Mode;
        use lindera::segmenter::Segmenter;
        use lindera::tokenizer::Tokenizer;

        c.bench_function("bench-constructor-simple-userdic-ipadic", |b| {
            b.iter(|| {
                let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../resources")
                    .join("ipadic_simple_userdic.csv");

                let dictionary = load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();
                let user_dictionary =
                    load_user_dictionary_from_csv(DictionaryKind::IPADIC, userdic_file.as_path())
                        .unwrap();
                let segmenter = Segmenter::new(
                    Mode::Normal,
                    dictionary,
                    Some(user_dictionary), // Assuming no user dictionary is provided
                );

                // Create a tokenizer.
                let tokenizer = Tokenizer::new(segmenter);
            })
        });
    }

    #[cfg(feature = "unidic")]
    {
        use std::path::PathBuf;

        use lindera::dictionary::{
            load_dictionary_from_kind, load_user_dictionary_from_csv, DictionaryKind,
        };
        use lindera::mode::Mode;
        use lindera::segmenter::Segmenter;
        use lindera::tokenizer::Tokenizer;

        c.bench_function("bench-constructor-simple-userdic-unidic", |b| {
            b.iter(|| {
                let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../resources")
                    .join("unidic_simple_userdic.csv");

                let dictionary = load_dictionary_from_kind(DictionaryKind::UniDic).unwrap();
                let user_dictionary =
                    load_user_dictionary_from_csv(DictionaryKind::UniDic, userdic_file.as_path())
                        .unwrap();
                let segmenter = Segmenter::new(
                    Mode::Normal,
                    dictionary,
                    Some(user_dictionary), // Assuming no user dictionary is provided
                );

                // Create a tokenizer.
                let tokenizer = Tokenizer::new(segmenter);
            })
        });
    }

    #[cfg(feature = "ko-dic")]
    {
        use std::path::PathBuf;

        use lindera::dictionary::{
            load_dictionary_from_kind, load_user_dictionary_from_csv, DictionaryKind,
        };
        use lindera::mode::Mode;
        use lindera::segmenter::Segmenter;
        use lindera::tokenizer::Tokenizer;

        c.bench_function("bench-constructor-simple-userdic-ko-dic", |b| {
            b.iter(|| {
                let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../resources")
                    .join("ko-dic_simple_userdic.csv");

                let dictionary = load_dictionary_from_kind(DictionaryKind::KoDic).unwrap();
                let user_dictionary =
                    load_user_dictionary_from_csv(DictionaryKind::KoDic, userdic_file.as_path())
                        .unwrap();
                let segmenter = Segmenter::new(
                    Mode::Normal,
                    dictionary,
                    Some(user_dictionary), // Assuming no user dictionary is provided
                );

                // Create a tokenizer.
                let tokenizer = Tokenizer::new(segmenter);
            })
        });
    }

    #[cfg(feature = "cc-cedict")]
    {
        use std::path::PathBuf;

        use lindera::dictionary::{
            load_dictionary_from_kind, load_user_dictionary_from_csv, DictionaryKind,
        };
        use lindera::mode::Mode;
        use lindera::segmenter::Segmenter;
        use lindera::tokenizer::Tokenizer;

        c.bench_function("bench-constructor-simple-userdic-cc-cedict", |b| {
            b.iter(|| {
                let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../resources")
                    .join("cc-cedict_simple_userdic.csv");

                let dictionary = load_dictionary_from_kind(DictionaryKind::CcCedict).unwrap();
                let user_dictionary =
                    load_user_dictionary_from_csv(DictionaryKind::CcCedict, userdic_file.as_path())
                        .unwrap();
                let segmenter = Segmenter::new(
                    Mode::Normal,
                    dictionary,
                    Some(user_dictionary), // Assuming no user dictionary is provided
                );

                // Create a tokenizer.
                let tokenizer = Tokenizer::new(segmenter);
            })
        });
    }
}

#[allow(unused_variables)]
fn bench_tokenize(c: &mut Criterion) {
    #[cfg(feature = "ipadic")]
    {
        use lindera::dictionary::{load_dictionary_from_kind, DictionaryKind};
        use lindera::mode::Mode;
        use lindera::segmenter::Segmenter;
        use lindera::tokenizer::Tokenizer;

        let dictionary = load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();
        let segmenter = Segmenter::new(
            Mode::Normal,
            dictionary,
            None, // Assuming no user dictionary is provided
        );

        // Create a tokenizer.
        let tokenizer = Tokenizer::new(segmenter);

        c.bench_function("bench-tokenize-ipadic", |b| {
            b.iter(|| tokenizer.tokenize("検索エンジン（けんさくエンジン、英語: search engine）は、狭義にはインターネットに存在する情報（ウェブページ、ウェブサイト、画像ファイル、ネットニュースなど）を検索する機能およびそのプログラム。"))
        });
    }

    #[cfg(feature = "unidic")]
    {
        use lindera::dictionary::{load_dictionary_from_kind, DictionaryKind};
        use lindera::mode::Mode;
        use lindera::segmenter::Segmenter;
        use lindera::tokenizer::Tokenizer;

        let dictionary = load_dictionary_from_kind(DictionaryKind::UniDic).unwrap();
        let segmenter = Segmenter::new(
            Mode::Normal,
            dictionary,
            None, // Assuming no user dictionary is provided
        );

        // Create a tokenizer.
        let tokenizer = Tokenizer::new(segmenter);

        c.bench_function("bench-tokenize-unidic", |b| {
            b.iter(|| tokenizer.tokenize("検索エンジン（けんさくエンジン、英語: search engine）は、狭義にはインターネットに存在する情報（ウェブページ、ウェブサイト、画像ファイル、ネットニュースなど）を検索する機能およびそのプログラム。"))
        });
    }

    #[cfg(feature = "ko-dic")]
    {
        use lindera::dictionary::{load_dictionary_from_kind, DictionaryKind};
        use lindera::mode::Mode;
        use lindera::segmenter::Segmenter;
        use lindera::tokenizer::Tokenizer;

        let dictionary = load_dictionary_from_kind(DictionaryKind::KoDic).unwrap();
        let segmenter = Segmenter::new(
            Mode::Normal,
            dictionary,
            None, // Assuming no user dictionary is provided
        );

        // Create a tokenizer.
        let tokenizer = Tokenizer::new(segmenter);

        c.bench_function("bench-tokenize-ko-dic", |b| {
            b.iter(|| tokenizer.tokenize("검색엔진(search engine)은컴퓨터시스템에저장된정보를찾아주거나웹검색(web search query)을도와주도록설계된정보검색시스템또는컴퓨터프로그램이다. 이러한검색결과는목록으로표시되는것이보통이다."))
        });
    }

    #[cfg(feature = "cc-cedict")]
    {
        use lindera::dictionary::{load_dictionary_from_kind, DictionaryKind};
        use lindera::mode::Mode;
        use lindera::segmenter::Segmenter;
        use lindera::tokenizer::Tokenizer;

        let dictionary = load_dictionary_from_kind(DictionaryKind::CcCedict).unwrap();
        let segmenter = Segmenter::new(
            Mode::Normal,
            dictionary,
            None, // Assuming no user dictionary is provided
        );

        // Create a tokenizer.
        let tokenizer = Tokenizer::new(segmenter);

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

        use lindera::dictionary::{
            load_dictionary_from_kind, load_user_dictionary_from_csv, DictionaryKind,
        };
        use lindera::mode::Mode;
        use lindera::segmenter::Segmenter;
        use lindera::tokenizer::Tokenizer;

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ipadic_simple_userdic.csv");

        let dictionary = load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();
        let user_dictionary =
            load_user_dictionary_from_csv(DictionaryKind::IPADIC, userdic_file.as_path()).unwrap();
        let segmenter = Segmenter::new(
            Mode::Normal,
            dictionary,
            Some(user_dictionary), // Assuming no user dictionary is provided
        );

        // Create a tokenizer.
        let tokenizer = Tokenizer::new(segmenter);

        c.bench_function("bench-tokenize-with-simple-userdic-ipadic", |b| {
            b.iter(|| {
                tokenizer.tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です")
            })
        });
    }

    #[cfg(feature = "unidic")]
    {
        use std::path::PathBuf;

        use lindera::dictionary::{
            load_dictionary_from_kind, load_user_dictionary_from_csv, DictionaryKind,
        };
        use lindera::mode::Mode;
        use lindera::segmenter::Segmenter;
        use lindera::tokenizer::Tokenizer;

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("unidic_simple_userdic.csv");

        let dictionary = load_dictionary_from_kind(DictionaryKind::UniDic).unwrap();
        let user_dictionary =
            load_user_dictionary_from_csv(DictionaryKind::UniDic, userdic_file.as_path()).unwrap();
        let segmenter = Segmenter::new(
            Mode::Normal,
            dictionary,
            Some(user_dictionary), // Assuming no user dictionary is provided
        );

        // Create a tokenizer.
        let tokenizer = Tokenizer::new(segmenter);

        c.bench_function("bench-tokenize-with-simple-userdic-unidic", |b| {
            b.iter(|| {
                tokenizer.tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です")
            })
        });
    }

    #[cfg(feature = "ko-dic")]
    {
        use std::path::PathBuf;

        use lindera::dictionary::{
            load_dictionary_from_kind, load_user_dictionary_from_csv, DictionaryKind,
        };
        use lindera::mode::Mode;
        use lindera::segmenter::Segmenter;
        use lindera::tokenizer::Tokenizer;

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ko-dic_simple_userdic.csv");

        let dictionary = load_dictionary_from_kind(DictionaryKind::KoDic).unwrap();
        let user_dictionary =
            load_user_dictionary_from_csv(DictionaryKind::KoDic, userdic_file.as_path()).unwrap();
        let segmenter = Segmenter::new(
            Mode::Normal,
            dictionary,
            Some(user_dictionary), // Assuming no user dictionary is provided
        );

        // Create a tokenizer.
        let tokenizer = Tokenizer::new(segmenter);

        c.bench_function("bench-tokenize-with-simple-userdic-ko-dic", |b| {
            b.iter(|| tokenizer.tokenize("하네다공항한정토트백."))
        });
    }

    #[cfg(feature = "cc-cedict")]
    {
        use std::path::PathBuf;

        use lindera::dictionary::{
            load_dictionary_from_kind, load_user_dictionary_from_csv, DictionaryKind,
        };
        use lindera::mode::Mode;
        use lindera::segmenter::Segmenter;
        use lindera::tokenizer::Tokenizer;

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("cc-cedict_simple_userdic.csv");

        let dictionary = load_dictionary_from_kind(DictionaryKind::CcCedict).unwrap();
        let user_dictionary =
            load_user_dictionary_from_csv(DictionaryKind::CcCedict, userdic_file.as_path())
                .unwrap();
        let segmenter = Segmenter::new(
            Mode::Normal,
            dictionary,
            Some(user_dictionary), // Assuming no user dictionary is provided
        );

        // Create a tokenizer.
        let tokenizer = Tokenizer::new(segmenter);

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

        use lindera::dictionary::{load_dictionary_from_kind, DictionaryKind};
        use lindera::mode::Mode;
        use lindera::segmenter::Segmenter;
        use lindera::tokenizer::Tokenizer;

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

        let dictionary = load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();
        let segmenter = Segmenter::new(
            Mode::Normal,
            dictionary,
            None, // Assuming no user dictionary is provided
        );

        // Create a tokenizer.
        let tokenizer = Tokenizer::new(segmenter);

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

        use lindera::dictionary::{load_dictionary_from_kind, DictionaryKind};
        use lindera::mode::Mode;
        use lindera::segmenter::Segmenter;
        use lindera::tokenizer::Tokenizer;

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

        let dictionary = load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();
        let segmenter = Segmenter::new(
            Mode::Normal,
            dictionary,
            None, // Assuming no user dictionary is provided
        );

        // Create a tokenizer.
        let tokenizer = Tokenizer::new(segmenter);

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

        use lindera::dictionary::{load_dictionary_from_kind, DictionaryKind};
        use lindera::mode::Mode;
        use lindera::segmenter::Segmenter;
        use lindera::tokenizer::Tokenizer;

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

        let dictionary = load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();
        let segmenter = Segmenter::new(
            Mode::Normal,
            dictionary,
            None, // Assuming no user dictionary is provided
        );

        // Create a tokenizer.
        let tokenizer = Tokenizer::new(segmenter);

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

        use lindera::dictionary::{load_dictionary_from_kind, DictionaryKind};
        use lindera::mode::Mode;
        use lindera::segmenter::Segmenter;
        use lindera::tokenizer::Tokenizer;

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

        let dictionary = load_dictionary_from_kind(DictionaryKind::UniDic).unwrap();
        let segmenter = Segmenter::new(
            Mode::Normal,
            dictionary,
            None, // Assuming no user dictionary is provided
        );

        // Create a tokenizer.
        let tokenizer = Tokenizer::new(segmenter);

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
