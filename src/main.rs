use rustj::Engine;
use std::{
    env, fs,
    io::{self, BufRead, IsTerminal, Write},
    process::ExitCode,
};

fn run(
    engine: &mut Engine,
    line: &str,
    json: bool,
    semantic: bool,
    source_name: &str,
    line_number: usize,
) -> rustj::Result<()> {
    match if semantic {
        engine.eval_semantic_reference_diagnostic(line)
    } else {
        engine.eval_diagnostic(line)
    } {
        Ok(Some(v)) => {
            println!("{}", if json { v.json() } else { v.display() });
            Ok(())
        }
        Ok(None) => {
            if json {
                println!("{{\"silent\":true}}");
            }
            Ok(())
        }
        Err(e) => {
            if json {
                println!("{{\"error\":\"{}\"}}", e.kind());
            } else {
                eprintln!("{}", e.render(source_name, line, line_number));
            }
            Err(e)
        }
    }
}
fn run_input(
    engine: &mut Engine,
    lines: impl Iterator<Item = io::Result<String>>,
    json: bool,
    semantic: bool,
    source_name: &str,
    stop_on_error: bool,
    interactive: bool,
) -> ExitCode {
    use rustj::definition_input::{DefinitionCollector, InputFrame};
    let mut pending: Option<(DefinitionCollector, usize)> = None;
    let mut ok = true;
    for (index, line) in lines.enumerate() {
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::FAILURE;
            }
        };
        if pending.is_some() || rustj::syntax::has_definition_syntax(&line) {
            let (collector, start) =
                pending.get_or_insert_with(|| (DefinitionCollector::default(), index + 1));
            match collector.push_line(&line) {
                Ok(InputFrame::NeedMore) => {
                    if interactive {
                        eprint!("   ");
                        let _ = io::stderr().flush();
                    }
                    continue;
                }
                Ok(InputFrame::Definition(_) | InputFrame::Definitions(_)) => {
                    let result = run(
                        engine,
                        collector.source(),
                        json,
                        semantic,
                        source_name,
                        *start,
                    );
                    let succeeded = result.is_ok();
                    ok &= succeeded;
                    if succeeded
                        || (!stop_on_error
                            && result.as_ref().is_err_and(|e| e.kind() != "unsupported"))
                    {
                        pending = None;
                        if interactive {
                            eprint!("   ");
                            let _ = io::stderr().flush();
                        }
                        continue;
                    }
                }
                Ok(InputFrame::Sentence) => {
                    let _ = run(
                        engine,
                        collector.source(),
                        json,
                        semantic,
                        source_name,
                        *start,
                    );
                }
                Err(error) => {
                    print_input_error(&error, collector.source(), json, source_name, *start);
                }
            }
            eprintln!(
                "definition execution is not supported; stopping input before any body lines execute"
            );
            return ExitCode::FAILURE;
        }
        let succeeded = run(engine, &line, json, semantic, source_name, index + 1).is_ok();
        ok &= succeeded;
        if !succeeded && stop_on_error {
            return ExitCode::FAILURE;
        }
        if interactive {
            eprint!("   ");
            let _ = io::stderr().flush();
        }
    }
    if let Some((collector, start)) = pending {
        if let Err(error) = collector.finish() {
            print_input_error(&error, collector.source(), json, source_name, start);
        }
        eprintln!("unterminated definition; stopping input before any body lines execute");
        return ExitCode::FAILURE;
    }
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn print_input_error(
    error: &rustj::Error,
    source: &str,
    json: bool,
    source_name: &str,
    line: usize,
) {
    if json {
        println!("{{\"error\":\"{}\"}}", error.kind());
    } else {
        eprintln!("{}", error.render(source_name, source, line));
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
        return if run(&mut engine, &s, json, semantic, "<command-line>", 1).is_ok() {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }
    if let Some(path) = file {
        let source = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::FAILURE;
            }
        };
        return run_input(
            &mut engine,
            source.lines().map(|line| Ok(line.to_owned())),
            json,
            semantic,
            &path,
            true,
            false,
        );
    }
    let input = io::stdin();
    let interactive = input.is_terminal() && !json;
    if interactive {
        eprint!("   ");
        let _ = io::stderr().flush();
    }
    run_input(
        &mut engine,
        input.lock().lines(),
        json,
        semantic,
        "<stdin>",
        false,
        interactive,
    )
}
