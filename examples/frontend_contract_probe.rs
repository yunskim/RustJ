//! Read-only stage inspection and explicit runtime observations for boundary audits.
use rustj::{Engine, Error, semantic::ExprKind};
use std::io::{self, BufRead};

fn hex(text: &str) -> String {
    text.as_bytes().iter().map(|b| format!("{b:02x}")).collect()
}

fn failure(error: Error, source: &str) -> String {
    let context = error.context();
    let span = context
        .and_then(|c| c.span.as_ref())
        .map_or("null".into(), |s| format!("[{},{}]", s.start, s.end));
    let blame = context
        .and_then(|c| c.blame_word_index)
        .map_or("null".into(), |n| n.to_string());
    format!(
        "{{\"error\":\"{}\",\"span\":{span},\"blame_word\":{blame},\"context_hex\":\"{}\",\"render_hex\":\"{}\"}}",
        error.kind(),
        hex(&format!("{context:?}")),
        hex(&error.render("audit", source, 1))
    )
}

fn inspect(engine: &mut Engine, operation: &str, source: &str) -> rustj::Result<String> {
    match operation {
        "P" => {
            let bound = engine.prepare_semantic_diagnostic(source)?;
            let p = bound.program;
            let context = p.frontend.as_ref().expect("analysis context");
            context.verify().map_err(Error::Unsupported)?;
            let kind = match p.expression.as_ref().map(|e| &e.kind) {
                Some(ExprKind::VerbValue(_)) => "verb",
                Some(ExprKind::ModifierValue(_)) => "modifier",
                Some(ExprKind::Literal(_)) => "literal",
                Some(_) => "expression",
                None => "none",
            };
            Ok(format!(
                "{{\"ok\":true,\"kind\":\"{kind}\",\"complete\":{},\"words\":{},\"reductions\":{},\"names_hex\":\"{}\"}}",
                context.complete,
                context.words.len(),
                context.reductions.len(),
                hex(&format!("{:?}", context.name_uses))
            ))
        }
        "G" => {
            engine
                .analyze_j_graph_diagnostic(source)?
                .verify()
                .map_err(Error::Unsupported)?;
            Ok("{\"ok\":true}".into())
        }
        "L" => {
            engine.analyze_compilation_diagnostic(source)?;
            Ok("{\"ok\":true}".into())
        }
        "R" => {
            let observed = engine.eval_captured(source);
            observed
                .capture
                .verify()
                .map_err(|e| Error::Unsupported(e.into()))?;
            let result = match observed.result {
                Ok(Some(value)) => value.json(),
                Ok(None) => "{\"silent\":true}".into(),
                Err(error) => failure(error, source),
            };
            Ok(format!(
                "{{\"result\":{result},\"capture_hex\":\"{}\"}}",
                hex(&format!("{:?}", observed.capture))
            ))
        }
        "E" => Ok(match engine.eval_diagnostic(source)? {
            Some(value) => value.json(),
            None => "{\"silent\":true}".into(),
        }),
        _ => Err(Error::Syntax("unknown audit operation".into())),
    }
}

fn main() {
    let mut engine = Engine::new();
    for line in io::stdin().lock().lines() {
        let line = line.expect("stdin");
        let (operation, encoded) = line.split_once(' ').expect("operation and UTF-8 hex");
        let bytes: Vec<_> = encoded
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                u8::from_str_radix(std::str::from_utf8(pair).expect("hex"), 16).expect("hex byte")
            })
            .collect();
        let source = String::from_utf8(bytes).expect("UTF-8 source");
        let output = inspect(&mut engine, operation, &source)
            .unwrap_or_else(|error| failure(error, &source));
        println!("{output}");
    }
}
