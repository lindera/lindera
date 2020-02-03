# Lindera

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Join the chat at https://gitter.im/bayard-search/lindera](https://badges.gitter.im/bayard-search/lindera.svg)](https://gitter.im/bayard-search/lindera?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

A Japanese Morphological Analyzer Command Line Interface written in Rust. This project fork from fulmicoton's [kuromoji-rs](https://github.com/fulmicoton/kuromoji-rs).


## Usage

```rust
use lindera::tokenizer::tokenizer::Tokenizer;
use std::env;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader, BufWriter, Write};

fn main() -> io::Result<()> {
    let mut args_it = env::args();
    let _ = args_it.next().unwrap();
    let input = args_it.next().unwrap();
    let output = args_it.next().unwrap();
    let f = File::open(input)?;
    let buff = BufReader::new(f);

    let mut wtr = BufWriter::new(File::create(output)?);
    let mut tokenizer = Tokenizer::normal();
    let mut output_line = String::new();
    for line_res in buff.lines() {
        output_line.clear();
        let line = line_res?;
        for token in tokenizer.tokenize_str(&line) {
            output_line.push_str(token);
            output_line.push_str("---");
        }
        output_line.push_str("\n");
        wtr.write_all(&output_line.as_bytes())?;
    }
    wtr.flush()?;
    Ok(())
}
```

test test_tokenize ... bench:       7,666 ns/iter (+/- 25,545)  
test test_tokenize ... bench:       5,507 ns/iter (+/- 755)
