use std::{env, process::ExitCode};

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

fn main() -> ExitCode {
    let mut args = env::args_os();
    let _ = args.next();
    let Some(source) = args.next() else {
        eprintln!("usage: word_formation <source>");
        return ExitCode::from(2);
    };
    if args.next().is_some() {
        eprintln!("usage: word_formation <source>");
        return ExitCode::from(2);
    }
    let source = source.to_string_lossy();
    match rustj::scanner::word_texts(&source) {
        Ok(words) => {
            println!("OK");
            for word in words {
                println!("{}", hex(word.as_bytes()));
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            println!("ERR\t{}", error.kind());
            ExitCode::SUCCESS
        }
    }
}
