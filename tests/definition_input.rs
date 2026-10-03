//! DEF-1 input framing only. No callable, symbol lookup or body execution.
use rustj::definition_input::{DefinitionCollector, DefinitionForm, InputFrame, frame};

fn definition(source: &str) -> rustj::definition_input::DefinitionInput {
    let InputFrame::Definition(input) = frame(source).unwrap() else {
        panic!("{source}")
    };
    input
}

#[test]
fn direct_nesting_quotes_comments_and_source_spans_are_preserved() {
    let source = "f=:{{\nNB. }} {{ ignored\ninner=.{{y+1}}\n'it''s }}'\ninner y\n}} trailing";
    let input = definition(source);
    assert_eq!(input.form, DefinitionForm::Direct);
    assert_eq!(
        &source[input.span.clone()],
        &source[3..source.find(" trailing").unwrap()]
    );
    assert_eq!(input.nested.len(), 1);
    assert_eq!(&source[input.nested[0].clone()], "{{y+1}}");
    assert!(input.body_text(source).unwrap().contains("inner=.{{y+1}}"));
}

#[test]
fn explicit_input_requires_spaces_only_terminator_and_preserves_crlf() {
    let source = "f=:3 : 0 NB. header\r\ny+1\r\n) NB. body\r\n\t)\r\n : \r\nx+y\r\n  )  \r\n42";
    let input = definition(source);
    assert_eq!(input.form, DefinitionForm::ExplicitBlock(3));
    assert_eq!(
        input.body_text(source).unwrap(),
        "y+1\r\n) NB. body\r\n\t)\r\n : \r\nx+y\r\n"
    );
    assert_eq!(&source[input.span.end..], "\r\n42");
}

#[test]
fn explicit_quoted_body_unescapes_without_evaluating_the_body() {
    for mode in 1..=4 {
        let source = format!("f=:{mode} : 'leaked=:99 NB. it''s {{ }}'");
        let input = definition(&source);
        assert_eq!(input.form, DefinitionForm::ExplicitString(mode));
        assert_eq!(input.body_text(&source).unwrap(), "leaked=:99 NB. it's { }");
        let engine = rustj::Engine::new();
        assert!(engine.binding_version("leaked").is_none());
        assert!(engine.prepare_semantic(&source).is_ok());
        let multiline = format!("f=:{mode} : 'leaked=:99\ny+1'");
        assert_eq!(
            definition(&multiline).body_text(&multiline).unwrap(),
            "leaked=:99\ny+1"
        );
    }
}

#[test]
fn collector_waits_for_whole_definition_and_reports_eof() {
    for lines in [vec!["f=:{{", "y+1", "}}"], vec!["f=:3 : 0", "y+1", ")"]] {
        let mut collector = DefinitionCollector::default();
        for line in &lines[..2] {
            assert_eq!(collector.push_line(line).unwrap(), InputFrame::NeedMore);
        }
        assert_eq!(collector.finish().unwrap_err().kind(), "syntax error");
        assert!(matches!(
            collector.push_line(lines[2]).unwrap(),
            InputFrame::Definition(_)
        ));
        assert_eq!(collector.source(), lines.join("\n"));
    }
}

#[test]
fn quoted_and_inflected_delimiters_are_not_definition_input() {
    for source in [
        "'{{ }} : define'",
        "NB. {{",
        "{{.",
        "3:",
        "a=:1",
        "3 : body",
    ] {
        assert_eq!(frame(source).unwrap(), InputFrame::Sentence, "{source}");
    }
    assert_eq!(
        frame("f=:{{ 'unfinished\n}} ").unwrap(),
        InputFrame::NeedMore
    );
    assert!(frame("f=:3 : 'unfinished").is_err());
    assert_eq!(frame("f=:{{)n raw }}").unwrap_err().kind(), "unsupported");
}

#[test]
fn collected_body_enqueue_keeps_local_copulas_and_defers_names() {
    use rustj::enqueuer::{EnqueueEnvironment, EnqueuedPayload, enqueue_in_environment};
    let source = "f=:3 : 0\ncopy=.future\n)";
    let input = definition(source);
    let body = input.body_text(source).unwrap();
    let queue = enqueue_in_environment(
        body.trim_end(),
        &rustj::primitive::PrimitiveContext::core(),
        EnqueueEnvironment::ExplicitDefinition,
    )
    .unwrap();
    assert!(queue[1].flags.local_assignment);
    assert!(!queue[1].flags.global_assignment);
    assert!(matches!(queue[2].payload, EnqueuedPayload::Name("future")));
    assert!(queue[2].flags.lookup_name);
}

#[test]
fn block_terminator_inside_nested_direct_definition_does_not_end_input() {
    let source = "f=:3 : 0\ninner=.{{\n)\ny+1\n}}\ninner y\n)";
    let input = definition(source);
    assert_eq!(input.span.end, source.len());
    assert_eq!(input.nested.len(), 1);
    assert_eq!(&source[input.nested[0].clone()], "{{\n)\ny+1\n}}");
    assert!(input.body_text(source).unwrap().contains("inner y"));
}

#[test]
fn multiple_roots_preserve_disjoint_source_order_and_nested_ownership() {
    let source = "f=:{{ '}}' [ y }} + {{ {{y+1}} y }} NB. {{ opaque";
    let InputFrame::Definitions(inputs) = frame(source).unwrap() else {
        panic!()
    };
    assert_eq!(inputs.len(), 2);
    assert!(inputs[0].span.end < inputs[1].span.start);
    assert!(inputs[0].nested.is_empty());
    assert_eq!(inputs[1].nested.len(), 1);
    assert_eq!(&source[inputs[1].nested[0].clone()], "{{y+1}}");
    let mut collector = DefinitionCollector::default();
    assert_eq!(
        collector.push_line("f=:{{y}} + {{").unwrap(),
        InputFrame::NeedMore
    );
    assert!(collector.finish().is_err());
    assert!(matches!(
        collector.push_line("y+1}}").unwrap(),
        InputFrame::Definitions(_)
    ));
    assert!(frame("{{y}} }}").is_err());
}

#[test]
fn each_root_enqueues_its_own_constructor_and_expanded_error_index() {
    use rustj::{
        enqueuer::{EnqueuedPayload, enqueue},
        semantic::FunctionHead,
    };
    let source = "{{y+1}} + {{y-1}}";
    let queue = enqueue(source).unwrap();
    assert_eq!(queue.len(), 11);
    let origins: Vec<_> = queue
        .iter()
        .filter_map(|word| match &word.payload {
            EnqueuedPayload::Function(function) => match &function.head {
                FunctionHead::DefinitionConstructor(origin) => Some(origin),
                _ => None,
            },
            _ => None,
        })
        .collect();
    assert_eq!(origins.len(), 2);
    assert_eq!(&source[origins[0].input.span.clone()], "{{y+1}}");
    assert_eq!(&source[origins[1].input.span.clone()], "{{y-1}}");
    assert!(std::sync::Arc::ptr_eq(
        &origins[0].source,
        &origins[1].source
    ));
    assert!(std::sync::Arc::ptr_eq(
        &origins[0].primitives,
        &origins[1].primitives
    ));
    for (i, word) in queue.iter().enumerate() {
        assert_eq!(word.word_index, i);
    }
    let error = enqueue("{{y}} + {{y}} 1 2e").unwrap_err();
    assert_eq!(error.context().unwrap().blame_word_index, Some(11));
}
