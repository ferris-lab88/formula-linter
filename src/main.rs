mod lint;

use lint::Severity;
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

    let mut error_count = 0usize;
    let mut warning_count = 0usize;

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
            match finding.severity {
                Severity::Error => error_count += 1,
                Severity::Warning => warning_count += 1,
            }
            println!("{}", finding);
        }
    }

    let total = error_count + warning_count;
    if total == 0 {
        println!("no issues found");
    } else {
        println!(
            "{} issue{} ({} error{}, {} warning{})",
            total,
            if total == 1 { "" } else { "s" },
            error_count,
            if error_count == 1 { "" } else { "s" },
            warning_count,
            if warning_count == 1 { "" } else { "s" },
        );
    }

    // Warnings are worth reading but shouldn't fail a CI step on their own;
    // only a real error (a broken or empty formula) should trip the exit code.
    if error_count > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
