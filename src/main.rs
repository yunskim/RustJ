use rustj::Engine;
use std::{
    env, fs,
    io::{self, BufRead, IsTerminal, Write},
    process::ExitCode,
};

fn run(engine: &mut Engine, line: &str, json: bool, semantic: bool) -> bool {
    match if semantic {
        engine.eval_semantic_reference(line)
    } else {
        engine.eval(line)
    } {
        Ok(Some(v)) => {
            println!("{}", if json { v.json() } else { v.display() });
            true
        }
        Ok(None) => {
            if json {
                println!("{{\"silent\":true}}");
            }
            true
        }
        Err(e) => {
            if json {
                println!("{{\"error\":\"{}\"}}", e.kind());
            } else {
                eprintln!("{e}");
            }
            false
        }
    }
}
fn main() -> ExitCode {
    let mut json = false;
    let mut semantic = false;
    let mut expr = None;
    let mut file = None;
    let mut args = env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--json" => json = true,
            "--semantic-reference" => semantic = true,
            "-e" => {
                expr = args.next();
                if expr.is_none() {
                    eprintln!("-e requires a sentence");
                    return ExitCode::from(2);
                }
            }
            "--help" | "-h" => {
                println!(
                    "rustj [--json] [--semantic-reference] [-e 'J sentence' | script.ijs]\nNo arguments: read one sentence per line from stdin.\nExperimental subset; no C engine fallback."
                );
                return ExitCode::SUCCESS;
            }
            _ if !a.starts_with('-') && file.is_none() => file = Some(a),
            _ => {
                eprintln!("unknown argument: {a}");
                return ExitCode::from(2);
            }
        }
    }
    if expr.is_some() && file.is_some() {
        eprintln!("choose -e or script, not both");
        return ExitCode::from(2);
    }
    let mut engine = Engine::new();
    if let Some(s) = expr {
        return if run(&mut engine, &s, json, semantic) {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }
    if let Some(path) = file {
        let source = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::FAILURE;
            }
        };
        for line in source.lines() {
            if !run(&mut engine, line, json, semantic) {
                return ExitCode::FAILURE;
            }
        }
        return ExitCode::SUCCESS;
    }
    let input = io::stdin();
    let interactive = input.is_terminal() && !json;
    if interactive {
        eprint!("   ");
        let _ = io::stderr().flush();
    }
    let mut ok = true;
    for line in input.lock().lines() {
        match line {
            Ok(line) => {
                let succeeded = run(&mut engine, &line, json, semantic);
                ok &= succeeded;
                if !succeeded && rustj::syntax::has_definition_syntax(&line) {
                    eprintln!(
                        "definition syntax is not supported; stopping input before any body lines execute"
                    );
                    return ExitCode::FAILURE;
                }
            }
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::FAILURE;
            }
        }
        if interactive {
            eprint!("   ");
            let _ = io::stderr().flush();
        }
    }
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
