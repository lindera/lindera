//! Measures a model file of lindera-dependency: its size, the time to read
//! it, and the sentences per second of chunking.
//!
//! The chunker is trained on the first file with the defaults of
//! `train-chunker` and put into a model file. The second file is then
//! chunked with the chunker read on the bytes of the file, and with the
//! same chunker in memory, where the feature strings are in a hash table of
//! the standard library. The two are run in turn, and the best round of each
//! is reported.
//!
//! Usage: model_speed <train.lattice> <input.lattice> <rounds>

use std::process::ExitCode;
use std::time::Instant;

use lindera_dependency::Sentence;
use lindera_dependency::chunker::{self, Chunker, ChunkerData, TrainOptions};
use lindera_dependency::lattice::{self, LatticeMorpheme};
use lindera_dependency::model::{Model, ModelBuilder};

/// How often the file is read to time the reading.
const READS: u32 = 1000;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Error: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Trains, writes the model file in memory, and measures.
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = std::env::args().collect();
    let [_, training, input, rounds] = arguments.as_slice() else {
        return Err("usage: model_speed <train.lattice> <input.lattice> <rounds>".into());
    };
    let training = lattice::parse(&std::fs::read_to_string(training)?)?;
    let input = lattice::parse(&std::fs::read_to_string(input)?)?;
    let rounds: usize = rounds.parse()?;

    let options = TrainOptions {
        threads: std::thread::available_parallelism().map_or(1, usize::from),
        ..TrainOptions::default()
    };
    let trained = chunker::train(&training, &options)?;
    let bytes = ModelBuilder::new("ipadic", options.schema.clone())?
        .chunker(&trained.model)?
        .to_bytes();
    println!("feature strings: {}", trained.model.feature_count());
    println!("bytes: {}", bytes.len());

    let started = Instant::now();
    for _ in 0..READS {
        std::hint::black_box(Model::new(std::hint::black_box(bytes.as_slice()))?);
    }
    println!(
        "reading: {:.0} microseconds",
        started.elapsed().as_secs_f64() * 1e6 / f64::from(READS)
    );

    let model = Model::new(bytes.as_slice())?;
    let on_bytes = model.chunker().ok_or("the model has no chunker")?;
    let in_memory = Chunker::new(trained.model);
    let morphemes: usize = input.iter().map(|sentence| sentence.morphemes.len()).sum();
    println!("sentences: {}, morphemes: {morphemes}", input.len());

    let mut best = [f64::INFINITY; 2];
    let mut bunsetsu = [0; 2];
    for _ in 0..rounds {
        let started = Instant::now();
        bunsetsu[0] = chunk(&in_memory, &input)?;
        best[0] = best[0].min(started.elapsed().as_secs_f64());
        let started = Instant::now();
        bunsetsu[1] = chunk(&on_bytes, &input)?;
        best[1] = best[1].min(started.elapsed().as_secs_f64());
    }
    if bunsetsu[0] != bunsetsu[1] {
        return Err("the two chunkers do not give the same number of bunsetsu".into());
    }
    for (name, seconds) in [
        ("in memory", best[0]),
        ("on the bytes of the file", best[1]),
    ] {
        println!(
            "chunking {name}: {:.0} sentences per second",
            input.len() as f64 / seconds
        );
    }
    Ok(())
}

/// Chunks every sentence and counts the bunsetsu.
fn chunk<D: ChunkerData>(
    chunker: &Chunker<D>,
    sentences: &[Sentence<LatticeMorpheme>],
) -> Result<usize, lindera_dependency::Error> {
    let mut count = 0;
    for sentence in sentences {
        count += chunker.chunk(&sentence.morphemes)?.len();
    }
    Ok(count)
}
