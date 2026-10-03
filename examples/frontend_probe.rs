//! Frontend-only observability. No backend, storage identity or schedule is serialized.
use rustj::{enqueuer::*, parser::*, semantic::*};
use std::io::{self, BufRead};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn function(f: &FunctionEntity) -> String {
    let head = match &f.head {
        FunctionHead::PrimitiveVerb(id) => id.spelling(),
        FunctionHead::PrimitiveAdverb(id) => id.spelling(),
        FunctionHead::PrimitiveConjunction(id) => id.spelling(),
        FunctionHead::Hook => "2",
        FunctionHead::Fork => "3",
        FunctionHead::NameRef(name) => name,
    };
    let operands = f
        .operands
        .iter()
        .map(|operand| match operand {
            FunctionOperand::Function(child) => function(child),
            FunctionOperand::Noun { value, .. } => format!("{{\"noun\":{}}}", value.json()),
        })
        .collect::<Vec<_>>()
        .join(",");
    // Hex avoids transport escaping affecting J operator/name identity.
    format!(
        "{{\"head_hex\":\"{}\",\"operands\":[{}]}}",
        hex(head.as_bytes()),
        operands
    )
}
fn rows() {
    use ParseClass::*;
    let classes = [
        Noun,
        Verb,
        Adverb,
        Conjunction,
        Name,
        Assignment,
        LParen,
        RParen,
        Mark,
    ];
    for a in classes {
        for b in classes {
            for c in classes {
                for d in classes {
                    let row = match_parse_row([a, b, c, d])
                        .map_or("null".to_owned(), |r| (r as u8).to_string());
                    println!(
                        "{{\"classes\":[\"{a:?}\",\"{b:?}\",\"{c:?}\",\"{d:?}\"],\"row\":{row}}}"
                    );
                }
            }
        }
    }
}
fn inspect(source: &str) -> rustj::Result<String> {
    let raw = rustj::tokenizer::scan(source.as_bytes())?;
    let visible = rustj::tokenizer::parse_word_spans(source.as_bytes())?;
    let raw_words = raw
        .iter()
        .map(|s| format!("\"{}\"", hex(&source.as_bytes()[s.clone()])))
        .collect::<Vec<_>>()
        .join(",");
    let visible_words = visible
        .iter()
        .map(|s| format!("\"{}\"", hex(&source.as_bytes()[s.clone()])))
        .collect::<Vec<_>>()
        .join(",");
    let queue = match enqueue(source) {
        Ok(words) => {
            let entries = words.iter().map(|w| {
                let noun = match &w.payload {
                    EnqueuedPayload::Scalar(s) => s.clone().into_value().map(|v| v.json()),
                    EnqueuedPayload::Noun(v) => Ok(v.json()),
                    _ => Ok("null".to_owned()),
                }?;
                Ok(format!("{{\"class\":\"{:?}\",\"span\":[{},{}],\"index\":{},\"lookup\":{},\"global\":{},\"local\":{},\"to_name\":{},\"noun\":{}}}", w.class, w.span.start,w.span.end,w.word_index,w.flags.lookup_name,w.flags.global_assignment,w.flags.local_assignment,w.flags.assignment_to_name,noun))
            }).collect::<rustj::Result<Vec<_>>>()?;
            format!("[{}]", entries.join(","))
        }
        Err(e) => format!("{{\"error\":\"{}\"}}", e.kind()),
    };
    let parsed = match parse(source) {
        Ok(p) => match p.expression.map(|e| e.kind) {
            Some(ExprKind::VerbValue(v)) => {
                format!("{{\"pos\":3,\"function\":{}}}", function(&v.entity))
            }
            Some(ExprKind::ModifierValue(f)) => format!(
                "{{\"pos\":{},\"function\":{}}}",
                match f.result_pos {
                    FunctionPartOfSpeech::Adverb => 1,
                    FunctionPartOfSpeech::Conjunction => 2,
                    FunctionPartOfSpeech::Verb => 3,
                },
                function(&f)
            ),
            _ => "{\"non_function\":true}".to_owned(),
        },
        Err(e) => format!("{{\"error\":\"{}\"}}", e.kind()),
    };
    Ok(format!(
        "{{\"raw_words\":[{raw_words}],\"visible_words\":[{visible_words}],\"enqueue\":{queue},\"parse\":{parsed}}}"
    ))
}
fn main() {
    if std::env::args().any(|a| a == "--rows") {
        rows();
        return;
    }
    for line in io::stdin().lock().lines() {
        let line = line.expect("stdin");
        let bytes: Option<Vec<u8>> = if line.len() % 2 == 0 {
            (0..line.len())
                .step_by(2)
                .map(|i| {
                    line.get(i..i + 2)
                        .and_then(|s| u8::from_str_radix(s, 16).ok())
                })
                .collect()
        } else {
            None
        };
        let output = match bytes.and_then(|b| String::from_utf8(b).ok()) {
            Some(source) => {
                inspect(&source).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e.kind()))
            }
            None => "{\"transport_error\":\"invalid UTF-8 hex\"}".to_owned(),
        };
        println!("{output}");
    }
}
