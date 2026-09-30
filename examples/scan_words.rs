//! Local test transport: hex input bytes -> JSON word spans, one case per line.
use std::io::{self, BufRead};
fn main() {
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        let bytes: Vec<u8> = (0..line.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&line[i..i + 2], 16).unwrap())
            .collect();
        match rustj::scanner::parse_word_spans(&bytes) {
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
