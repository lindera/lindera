//! Measures a model file of lindera-dependency: its size, the time to read
//! it, and the sentences per second of chunking, of parsing and of both.
//!
//! The file is one that `train-chunker` or `train-parser` wrote. Every
//! sentence of the input is analyzed with what the file has: its morphemes
//! are chunked, its bunsetsu are parsed, and its morphemes are chunked and
//! parsed. Each is run several times, and the best round is reported.
//!
//! With a training file as the last argument, a chunker is also trained
//! with the defaults of `train-chunker` and run in memory, where its
//! feature strings are in a hash table of the standard library, in turn
//! with the same chunker on the bytes of a model file.
//!
//! Usage: model_speed <model> <input.lattice> <rounds> [<train.lattice>]

use std::ops::Range;
use std::process::ExitCode;
use std::time::Instant;

use lindera_dependency::Sentence;
use lindera_dependency::chunker::{self, Chunker, ChunkerData, TrainOptions};
use lindera_dependency::lattice::{self, LatticeMorpheme};
use lindera_dependency::model::{Model, ModelBuilder};

/// How often the file is read to time the reading.
const READS: u32 = 100;

/// The error of this program.
type Failure = Box<dyn std::error::Error>;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Error: {error}");
            ExitCode::FAILURE
        }
    }
}

/// The best time of some rounds of a function, in seconds.
fn best(rounds: usize, mut run: impl FnMut() -> Result<usize, Failure>) -> Result<f64, Failure> {
    let mut best = f64::INFINITY;
    for _ in 0..rounds {
        let started = Instant::now();
        std::hint::black_box(run()?);
        best = best.min(started.elapsed().as_secs_f64());
    }
    Ok(best)
}

/// Reads the model and the input, and measures.
fn run() -> Result<(), Failure> {
    let arguments: Vec<String> = std::env::args().collect();
    let (model, input, rounds, training) = match arguments.as_slice() {
        [_, model, input, rounds] => (model, input, rounds, None),
        [_, model, input, rounds, training] => (model, input, rounds, Some(training)),
        _ => {
            return Err(
                "usage: model_speed <model> <input.lattice> <rounds> [<train.lattice>]".into(),
            );
        }
    };
    let bytes = std::fs::read(model)?;
    let input = lattice::parse(&std::fs::read_to_string(input)?)?;
    let rounds: usize = rounds.parse()?;
    let morphemes: usize = input.iter().map(|sentence| sentence.morphemes.len()).sum();
    let bunsetsu: usize = input.iter().map(|sentence| sentence.bunsetsu.len()).sum();
    println!("bytes: {}", bytes.len());
    println!(
        "sentences: {}, bunsetsu: {bunsetsu}, morphemes: {morphemes}",
        input.len()
    );

    let started = Instant::now();
    for _ in 0..READS {
        std::hint::black_box(Model::new(std::hint::black_box(bytes.as_slice()))?);
    }
    println!(
        "reading: {:.0} microseconds",
        started.elapsed().as_secs_f64() * 1e6 / f64::from(READS)
    );

    let model = Model::new(bytes.as_slice())?;
    let chunker = model.chunker();
    let parser = model.parser();
    let per_second = |seconds: f64| input.len() as f64 / seconds;
    if let Some(chunker) = &chunker {
        let seconds = best(rounds, || chunk(chunker, &input))?;
        println!("chunking: {:.0} sentences per second", per_second(seconds));
    }
    if let Some(parser) = &parser {
        let ranges: Vec<Vec<Range<usize>>> = input
            .iter()
            .map(|sentence| {
                sentence
                    .bunsetsu
                    .iter()
                    .map(|bunsetsu| bunsetsu.morphemes.clone())
                    .collect()
            })
            .collect();
        let seconds = best(rounds, || {
            let mut count = 0;
            for (sentence, ranges) in input.iter().zip(&ranges) {
                count += parser.parse(&sentence.morphemes, ranges)?.len();
            }
            Ok(count)
        })?;
        println!(
            "parsing the bunsetsu of the input: {:.0} sentences per second",
            per_second(seconds)
        );
    }
    if let (Some(chunker), Some(parser)) = (&chunker, &parser) {
        let seconds = best(rounds, || {
            let mut count = 0;
            for sentence in &input {
                let ranges = chunker.chunk(&sentence.morphemes)?;
                count += parser.parse(&sentence.morphemes, &ranges)?.len();
            }
            Ok(count)
        })?;
        println!(
            "chunking and parsing: {:.0} sentences per second",
            per_second(seconds)
        );
    }

    if let Some(training) = training {
        let training = lattice::parse(&std::fs::read_to_string(training)?)?;
        let options = TrainOptions {
            threads: std::thread::available_parallelism().map_or(1, usize::from),
            ..TrainOptions::default()
        };
        let trained = chunker::train(&training, &options)?;
        let bytes = ModelBuilder::new("ipadic", options.schema.clone())?
            .chunker(&trained.model)?
            .to_bytes();
        let model = Model::new(bytes.as_slice())?;
        let on_bytes = model.chunker().ok_or("the model has no chunker")?;
        let in_memory = Chunker::new(trained.model);
        // The two in turn, the best round of each.
        let mut seconds = [f64::INFINITY; 2];
        for _ in 0..rounds {
            seconds[0] = seconds[0].min(best(1, || chunk(&in_memory, &input))?);
            seconds[1] = seconds[1].min(best(1, || chunk(&on_bytes, &input))?);
        }
        println!(
            "a chunker trained here, in memory: {:.0} sentences per second",
            per_second(seconds[0])
        );
        println!(
            "the same chunker on the bytes of a file of {} bytes: {:.0} sentences per second",
            bytes.len(),
            per_second(seconds[1])
        );
    }
    Ok(())
}

/// Chunks every sentence and counts the bunsetsu.
fn chunk<D: ChunkerData>(
    chunker: &Chunker<D>,
    sentences: &[Sentence<LatticeMorpheme>],
) -> Result<usize, Failure> {
    let mut count = 0;
    for sentence in sentences {
        count += chunker.chunk(&sentence.morphemes)?.len();
    }
    Ok(count)
}
