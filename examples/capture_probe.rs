//! Differential-test adapter: ordinary result JSON with capture verification.
use rustj::Engine;
use std::{
    io::{self, BufRead},
    process::ExitCode,
};
fn main() -> ExitCode {
    let mut engine = Engine::new();
    let mut failed = false;
    let mut verified = 0;
    for line in io::stdin().lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::from(2);
            }
        };
        let report = engine.eval_captured(&line);
        if let Err(error) = report.capture.verify() {
            eprintln!("invalid capture for {line:?}: {error}");
            return ExitCode::from(2);
        }
        verified += 1;
        match report.result {
            Ok(Some(value)) => println!("{}", value.json()),
            Ok(None) => println!("{{\"silent\":true}}"),
            Err(error) => {
                println!("{{\"error\":\"{}\"}}", error.kind());
                failed = true;
            }
        }
    }
    eprintln!("verified captures: {verified}");
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
