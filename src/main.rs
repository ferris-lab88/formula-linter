mod lint;

use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let path = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("usage: flint <file>   (use '-' to read from stdin)");
            return ExitCode::from(2);
        }
    };

    let reader: Box<dyn BufRead> = if path == "-" {
        Box::new(BufReader::new(io::stdin()))
    } else {
        match File::open(&path) {
            Ok(f) => Box::new(BufReader::new(f)),
            Err(e) => {
                eprintln!("flint: cannot open {}: {}", path, e);
                return ExitCode::from(2);
            }
        }
    };

    let mut found_any = false;

    // lines() yields one owned String at a time and drops the previous
    // one before reading the next, so memory use stays flat no matter
    // how many rows the input has.
    for (index, line) in reader.lines().enumerate() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("flint: read error at line {}: {}", index + 1, e);
                return ExitCode::from(2);
            }
        };

        for finding in lint::check_line(index + 1, &line) {
            println!("{}", finding);
            found_any = true;
        }
    }

    if found_any {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
