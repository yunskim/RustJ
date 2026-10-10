//! Local word-formation transport: hex input bytes -> raw `wordil`-equivalent spans.
//!
//! This intentionally uses `scanner::scan`, not parser-visible `parse_word_spans`:
//! jsource keeps a trailing `NB.` field in the raw boundary buffer and excludes
//! it only through the separate AM parse-word count.
use std::io::{self, BufRead};
fn main() {
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        let bytes: Vec<u8> = (0..line.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&line[i..i + 2], 16).unwrap())
            .collect();
        match rustj::scanner::scan(&bytes) {
            Ok(spans) => println!(
                "{{\"spans\":[{}]}}",
                spans
                    .iter()
                    .map(|s| format!("[{},{}]", s.start, s.end))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Err(_) => println!("{{\"error\":\"open quote\"}}"),
        }
    }
}
