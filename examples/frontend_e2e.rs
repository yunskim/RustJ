//! Reproducible observation of the real frontend, not an alternate parser.
//! Run on Windows: cargo run --example frontend_e2e
use rustj::{Engine, frontend_context::NodeKind, parser_capture::CaptureEvent, semantic::*};

fn function(f: &FunctionEntity) -> String {
    match &f.head {
        FunctionHead::ExplicitDefinition(code) => format!(
            "ExplicitDefinition {{ pos: {:?}, form: {:?}, mode: {}, source_span: {:?}, body: {:?}, monad: {:?}, dyad: {:?}, names: {:?}, monad_controls: {:?}, dyad_controls: {:?} }}",
            f.result_pos,
            code.form,
            code.mode,
            code.source_span,
            code.body,
            code.monad,
            code.dyad,
            code.name_plan,
            code.monad_controls,
            code.dyad_controls,
        ),
        FunctionHead::DefinitionConstructor(source) => format!(
            "DefinitionConstructor {{ form: {:?}, source_span: {:?} }}",
            source.input.form, source.input.span,
        ),
        head => format!("{head:?}"),
    }
}

fn expression(e: &Expr) -> String {
    match &e.kind {
        ExprKind::Literal(value) => format!("Literal({})", value.json()),
        ExprKind::ReadName(name) => format!("ReadName({name:?})"),
        ExprKind::TakeName { name, single_word } => {
            format!("TakeName {{ name: {name:?}, single_word: {single_word} }}")
        }
        ExprKind::VerbValue(verb) => format!("VerbValue({})", function(&verb.entity)),
        ExprKind::ModifierValue(f) => format!("ModifierValue({})", function(f)),
        ExprKind::Group(inner) => format!("Group({})", expression(inner)),
        ExprKind::Monad { verb, argument } => format!(
            "Monad {{ function: {}, argument: {} }}",
            function(&verb.entity),
            expression(argument),
        ),
        ExprKind::Dyad { verb, left, right } => format!(
            "Dyad {{ function: {}, left: {}, right: {} }}",
            function(&verb.entity),
            expression(left),
            expression(right),
        ),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = Engine::new();
    for source in [
        "a=:1 2 3",
        "a+2*3",
        "g=:10",
        "explicit=:3 : 0\nt=.y+g\nt\n)",
        "direct=:{{ t=.y+g\nt }}",
        "explicit 2",
        "direct 2",
        "g=:20",
        "explicit 2",
        "direct 2",
        "pair=:4 : 'x+y'",
        "2 pair 3",
        "ddpair=:{{ x+y }}",
        "2 ddpair 3",
    ] {
        println!("SOURCE {source:?}");
        println!("TOKENIZER {:?}", rustj::tokenizer::word_texts(source)?);
        println!("FRAME {:?}", rustj::parser::frame_definition_input(source)?);
        for word in rustj::enqueuer::enqueue(source)? {
            let payload = match &word.payload {
                rustj::enqueuer::EnqueuedPayload::Function(f) => function(f),
                rustj::enqueuer::EnqueuedPayload::Noun(value) => value.json(),
                other => format!("{other:?}"),
            };
            println!(
                "QUEUED_WORD {} span={:?} class={:?} flags={:?} payload={payload}",
                word.word_index, word.span, word.class, word.flags
            );
        }
        let before = ["a", "g", "explicit", "direct", "pair", "ddpair", "t"]
            .map(|name| engine.binding_version(name));
        match engine.prepare_semantic_diagnostic(source) {
            Ok(bound) => {
                let program = bound.program;
                println!(
                    "ANALYSIS_PROGRAM assignment={:?} assignment_source={:?} expression={}",
                    program.assignment,
                    program.assignment_source,
                    program
                        .expression
                        .as_ref()
                        .map(expression)
                        .unwrap_or_else(|| "None".into())
                );
                if let Some(context) = &program.frontend {
                    context.verify()?;
                    println!(
                        "ANALYSIS_CONTEXT realization={:?} complete={} root={:?}",
                        context.realization, context.complete, context.root
                    );
                }
            }
            Err(error) => println!("ANALYSIS_BOUNDARY {error}"),
        }
        assert_eq!(
            before,
            ["a", "g", "explicit", "direct", "pair", "ddpair", "t"]
                .map(|name| engine.binding_version(name)),
            "analysis must not commit"
        );
        let observed = engine.eval_captured(source);
        observed.capture.verify()?;
        let context = observed
            .capture
            .frontend
            .as_ref()
            .ok_or("missing frontend")?;
        context.verify()?;
        println!("ENQUEUE {:?}", context.words);
        println!(
            "PARSER realization={:?} complete={} root={:?}",
            context.realization, context.complete, context.root
        );
        println!("ITEMS {:?}", context.items);
        println!("STEPS {:?}", context.steps);
        println!("REDUCTIONS {:?}", context.reductions);
        println!("NAME_USES {:?}", context.name_uses);
        for (i, node) in context.nodes.iter().enumerate() {
            let summary = match &node.kind {
                NodeKind::Function(f) => function(f),
                NodeKind::Construct {
                    row,
                    inputs,
                    function: Some(f),
                } => format!(
                    "Construct {{ row: {row:?}, inputs: {inputs:?}, function: {} }}",
                    function(f)
                ),
                other => format!("{other:?}"),
            };
            println!("NODE {i} {:?} {summary}", node.class);
        }
        println!("ORIGINS {:?}", context.origins);
        for event in &observed.capture.events {
            match event {
                CaptureEvent::ApplyAttempt {
                    function: f,
                    left,
                    right,
                    ..
                } => println!("APPLY {} x={left:?} y={right:?}", function(f)),
                CaptureEvent::ApplySuccess { facts, .. } => println!("APPLY_SUCCESS {facts:?}"),
                CaptureEvent::Commit {
                    name,
                    version,
                    class,
                    function: f,
                    ..
                } => {
                    println!("COMMIT name={name:?} version={version:?} class={class:?}");
                    if let Some(f) = f {
                        println!("COMMITTED_FUNCTION {}", function(f));
                        if let FunctionHead::ExplicitDefinition(code) = &f.head {
                            println!("DEFINITION_BODY_WORDS {:?}", code.sentences);
                        }
                    }
                }
                _ => {}
            }
        }
        println!(
            "RESULT {}",
            match observed.result? {
                Some(value) => value.json(),
                None => "{\"silent\":true}".into(),
            }
        );
        assert!(
            engine.binding_version("t").is_none(),
            "definition local must not escape"
        );
        println!();
    }
    // Once the frontend E2E pathway is inspected, an opt-in PyTorch-aligned
    // profile reports catalog identity and POS without claiming callable kernels.
    // Normal Engine::new() keeps the core-J-only default namespace unchanged.
    let nn_engine = Engine::with_primitive_context(
        rustj::nn_extensions::frontend_preview_context(),
    );
    for spec in rustj::nn_extensions::NAMES
        .iter()
        .filter(|spec| spec.frontend_preview)
    {
        let parsed = nn_engine.parse_frontend(spec.spelling)?;
        let frontend = parsed.frontend.as_ref().ok_or("missing NN frontend")?;
        frontend.verify()?;
        let family = rustj::nn_extensions::family(spec.family_id)
            .ok_or("missing NN family contract")?;
        println!(
            "NN_EXTENSION_FRONTEND id={} name={} identity={} pytorch={:?} pos={:?} intrinsic={:?} derived={:?} shape={:?} dtype={:?} effect={:?} route={:?} empty={:?} errors={:?} status=RECOGNIZED_ONLY",
            spec.family_id,
            spec.spelling,
            spec.semantic_identity,
            spec.pytorch_api,
            spec.pos,
            spec.intrinsic_ranks,
            spec.derived_verb_ranks,
            family.shape_rule,
            family.dtype_rule,
            family.effect_rule,
            family.logical_route_candidate,
            family.empty_fill_proof,
            family.error_order_proof,
        );
    }
    Ok(())
}
