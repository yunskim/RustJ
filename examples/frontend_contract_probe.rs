//! Read-only stage inspection and explicit runtime observations for boundary audits.
use rustj::{Engine, Error, semantic::ExprKind};
use std::io::{self, BufRead};

fn hex(text: &str) -> String {
    text.as_bytes().iter().map(|b| format!("{b:02x}")).collect()
}

fn failure(error: Error, source: &str, executing: bool) -> String {
    let context = error.context();
    let span = context
        .and_then(|c| c.span.as_ref())
        .map_or("null".into(), |s| format!("[{},{}]", s.start, s.end));
    let blame = context
        .and_then(|c| c.blame_word_index)
        .map_or("null".into(), |n| n.to_string());
    format!(
        "{{\"error\":\"{}\",\"category\":\"{}\",\"j_handler_eligible\":{},\"span\":{span},\"blame_word\":{blame},\"context_hex\":\"{}\",\"render_hex\":\"{}\"}}",
        error.kind(),
        error.category().name(),
        executing && error.is_j_catchable(),
        hex(&format!("{context:?}")),
        hex(&error.render("audit", source, 1))
    )
}

fn inspect(engine: &mut Engine, operation: &str, source: &str) -> rustj::Result<String> {
    match operation {
        "F" => {
            let program = engine
                .admit_frontend(source)
                .into_result()
                .map_err(|r| r.into_error())?;
            let context = program.frontend.as_ref().expect("frontend context");
            context.verify().map_err(Error::Verification)?;
            Ok(format!(
                "{{\"ok\":true,\"complete\":{},\"words\":{},\"reductions\":{}}}",
                context.complete,
                context.words.len(),
                context.reductions.len()
            ))
        }
        "H" => {
            let handoff = engine
                .admit_frontend_handoff(source)
                .into_result()
                .map_err(|r| r.into_error())?;
            Ok(format!(
                "{{\"ok\":true,\"requirements_hex\":\"{}\"}}",
                hex(&format!("{:?}", handoff.requirements()))
            ))
        }
        "P" => {
            let bound = engine
                .admit_semantic(source)
                .into_result()
                .map_err(|r| r.into_error())?;
            let p = bound.program;
            let context = p.frontend.as_ref().expect("analysis context");
            context.verify().map_err(Error::Verification)?;
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
                .admit_j_graph(source)
                .into_result()
                .map_err(|r| r.into_error())?
                .verify()
                .map_err(Error::Verification)?;
            Ok("{\"ok\":true}".into())
        }
        "L" => {
            engine
                .admit_logical(source)
                .into_result()
                .map_err(|r| r.into_error())?;
            Ok("{\"ok\":true}".into())
        }
        "R" => {
            let observed = engine.eval_captured(source);
            observed
                .capture
                .verify()
                .map_err(|e| Error::Verification(e.into()))?;
            let result = match observed.result {
                Ok(Some(value)) => value.json(),
                Ok(None) => "{\"silent\":true}".into(),
                Err(error) => failure(error, source, true),
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
            .unwrap_or_else(|error| failure(error, &source, matches!(operation, "R" | "E")));
        let stage = match operation {
            "F" => Some(rustj::admission::Stage::Frontend),
            "H" => Some(rustj::admission::Stage::FrontendHandoff),
            "P" => Some(rustj::admission::Stage::SemanticBinding),
            "G" => Some(rustj::admission::Stage::JGraph),
            "L" => Some(rustj::admission::Stage::Logical),
            _ => None,
        };
        if let Some(stage) = stage {
            println!(
                "{{\"inspection_stage\":\"{}\",\"execution_performed\":false,{}",
                stage.name(),
                &output[1..]
            );
        } else {
            println!("{output}");
        }
    }
}
