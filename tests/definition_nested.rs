use rustj::{Engine, semantic::FunctionHead};

#[test]
fn nested_bodies_own_control_words_parameters_and_valence_separators() {
    for definition in [
        "outer=:{{\ninner=.{{\nif. y>0 do. y+1 else. 0 end.\n}}\ninner y\n}}",
        "outer=:3 : 0\ninner=.{{\nif. y>0 do. y+1 else. 0 end.\n}}\ninner y\n)",
        "outer=:{{\ninner=.3 : 'if. y>0 do. y+1 else. 0 end.'\ninner y\n}}",
    ] {
        let mut e = Engine::new();
        e.eval(definition).unwrap();
        assert_eq!(e.eval("outer 4").unwrap().unwrap().int_at(0).unwrap(), 5);
        assert_eq!(e.eval("outer _2").unwrap().unwrap().int_at(0).unwrap(), 0);
        assert!(e.binding_version("inner").is_none());
    }
    let mut e = Engine::new();
    // Inner x and ':' must not turn outer into a dyad or split its body.
    e.eval("outer=:{{\ninner=.{{\ny+1\n:\nx+y\n}}\n2 inner y\n}}")
        .unwrap();
    assert_eq!(e.eval("outer 4").unwrap().unwrap().int_at(0).unwrap(), 6);
    let mut e = Engine::new();
    assert_eq!(
        e.eval("outer=:{{\ninner=.3 : 0\ny+1\n)\ninner y\n}}")
            .unwrap_err()
            .kind(),
        "syntax error"
    );
    assert!(e.binding_version("outer").is_none());
}

#[test]
fn nested_definition_has_no_caller_local_capture_and_can_escape() {
    let mut e = Engine::new();
    e.eval("g=:10").unwrap();
    e.eval("outer=:{{\ng=.99\ninner=.{{t=.y+g\nt}}\ninner y\n}}")
        .unwrap();
    assert_eq!(e.eval("outer 2").unwrap().unwrap().int_at(0).unwrap(), 12);
    e.eval("g=:20").unwrap();
    assert_eq!(e.eval("outer 2").unwrap().unwrap().int_at(0).unwrap(), 22);
    assert!(e.binding_version("t").is_none());
    e.eval("outer=:{{\nprivate=.99\nescaped=:{{y+g}}\ny\n}}")
        .unwrap();
    e.eval("outer 0").unwrap();
    assert_eq!(e.eval("escaped 2").unwrap().unwrap().int_at(0).unwrap(), 22);
    assert!(e.binding_version("private").is_none());
}

#[test]
fn inner_names_do_not_contaminate_outer_name_plan_or_modifier_mode() {
    let parsed = rustj::semantic::parse("outer=:{{\ninner=.{{u+y}}\ny\n}}").unwrap();
    let expr = parsed.expression.unwrap();
    let rustj::semantic::ExprKind::VerbValue(function) = expr.kind else {
        panic!("verb")
    };
    let FunctionHead::ExplicitDefinition(code) = &function.entity.head else {
        panic!("definition")
    };
    assert_eq!(code.name_plan.monad.local_declarations, ["inner"]);
    for occurrence in &code.name_plan.monad.occurrences {
        assert_ne!(&code.body[occurrence.span.clone()], "u");
    }
    let mut e = Engine::new();
    e.eval("u=:10").unwrap();
    e.eval("outer=:{{\ninner=.{{u+y}}\ny\n}}").unwrap();
    assert_eq!(e.eval("outer 2").unwrap().unwrap().int_at(0).unwrap(), 2);
}

#[test]
fn a3_definition_reference_preserves_code_without_executing_or_binding() {
    use rustj::{execution_semantics::CallTarget, logical_ir::OpKind};
    let e = Engine::new();
    let plan = e.analyze_a3("f=:{{leaked=:y+1}}").unwrap();
    plan.verify().unwrap();
    let result = plan.result.unwrap();
    let operation = &plan.operations[plan.values[result.0].producer.0];
    let OpKind::VerbReference(callable) = &operation.kind else {
        panic!("function reference")
    };
    assert_eq!(callable.target, CallTarget::Definition);
    let FunctionHead::ExplicitDefinition(code) = &callable.semantic.head else {
        panic!("code")
    };
    assert_eq!(&*code.body, "leaked=:y+1");
    assert!(e.binding_version("f").is_none());
    assert!(e.binding_version("leaked").is_none());
    assert!(e.analyze_a3("{{y+1}} 4").is_err());
}
