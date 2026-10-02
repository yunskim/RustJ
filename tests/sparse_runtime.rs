use rustj::{
    Data, Engine, Error, Value,
    facts::{DType, Facts, TypeFact},
};
use std::sync::Arc;
fn eval(e: &mut Engine, text: &str) -> Value {
    e.eval(text).unwrap().unwrap()
}

#[test]
fn dollar_dot_converts_and_exposes_components() {
    let mut e = Engine::new();
    e.eval("s=:$.2 3$0 1 0 2 0 3").unwrap();
    let s = eval(&mut e, "s");
    assert!(s.is_sparse());
    assert_eq!(s.type_code(), 4096);
    assert_eq!(s.len(), 6);
    assert_eq!(s.view().len(), 6);
    assert_eq!(s.view().to_owned().unwrap().json(), s.json());
    assert_eq!(eval(&mut e, "$s").json(), eval(&mut e, "2 3").json());
    assert_eq!(eval(&mut e, "#s").int_at(0).unwrap(), 2);
    for (source, expected) in [
        ("2$.s", "i.2"),
        ("3$.s", "0+0"),
        ("4$.s", "3 2$0 1 1 0 1 2"),
        ("5$.s", "1 2 3"),
        ("7$.s", "3"),
        ("0$.s", "2 3$0 1 0 2 0 3"),
    ] {
        assert_eq!(
            eval(&mut e, source).json(),
            eval(&mut e, expected).json(),
            "{source}"
        );
    }
    assert_eq!(eval(&mut e, "$.s").json(), s.json());
    assert_eq!(eval(&mut e, "0$.2 3$0 1 0 2 0 3").json(), s.json());
    assert_eq!(Facts::of(&s).dtype, TypeFact::Exact(DType::Int));
    assert!(s.json().contains("\"sparse\""));
}

#[test]
fn scalar_empty_bool_float_and_fill_creation() {
    let mut e = Engine::new();
    for source in ["42", "1", "'a'", "<42"] {
        assert_eq!(
            eval(&mut e, &format!("$.{source}")).json(),
            eval(&mut e, source).json()
        );
    }
    for source in ["i.0", "0 1 0 1", "0.0 1.5 0.0", "2 0$i.0"] {
        assert_eq!(
            eval(&mut e, &format!("0$.$.{source}")).json(),
            eval(&mut e, source).json()
        );
    }
    let zeros = eval(&mut e, "1$.2 3");
    assert_eq!(zeros.type_code(), 8192);
    let Data::Sparse(v) = zeros.data() else {
        panic!()
    };
    assert_eq!(v.stored_rows(), 0);
    assert_eq!(v.fill().float_at(0).unwrap(), 0.0);
    assert_eq!(
        eval(&mut e, "0$.1$.2 3").json(),
        eval(&mut e, "2 3$0.0").json()
    );
    assert_eq!(eval(&mut e, "2$.1 2 3").json(), eval(&mut e, ",0+0").json());
}

#[test]
fn sparse_aliases_transactions_and_materialization_limit() {
    let mut e = Engine::new();
    e.eval("s=:$.0 2 0 4").unwrap();
    e.eval("t=:s").unwrap();
    let s = eval(&mut e, "s");
    let t = eval(&mut e, "t");
    let (Data::Sparse(a), Data::Sparse(b)) = (s.data(), t.data()) else {
        panic!()
    };
    assert!(Arc::ptr_eq(a, b));
    e.eval("v=:5$.s").unwrap();
    e.eval("v=:v+10").unwrap();
    assert_eq!(eval(&mut e, "5$.s").json(), eval(&mut e, "2 4").json());
    assert!(matches!(e.eval("s=:s+1"), Err(Error::Unsupported(_))));
    assert_eq!(eval(&mut e, "s").json(), s.json());
    e.eval("huge=:1$.1000000 1000000").unwrap();
    assert_eq!(eval(&mut e, "7$.huge").int_at(0).unwrap(), 0);
    assert!(eval(&mut e, "huge").json().len() < 200);
    assert!(matches!(e.eval("0$.huge"), Err(Error::Limit)));
}

#[test]
fn unsupported_paths_do_not_silently_densify_or_panic() {
    let mut e = Engine::new();
    e.eval("s=:$.1 0 2").unwrap();
    for source in [
        "s+s", "s=s", "i.s", ",s", "<s", ">s", "0{s", "2$s", "+/s", "+\"0 s", "s i. s", "s e. s",
        "8$.s",
    ] {
        assert!(
            matches!(e.eval(source), Err(Error::Unsupported(_))),
            "{source}"
        );
    }
    for (source, error) in [
        ("3$.1 2", Error::Domain),
        ("6$.s", Error::Domain),
        ("1$._1 2", Error::Domain),
        ("1$.i.0", Error::Length),
    ] {
        assert_eq!(e.eval(source).unwrap_err(), error, "{source}");
    }
    let mut ir = Engine::new();
    let mut direct = Engine::new();
    for source in ["s=:$.0 1 0 2", "t=:s", "5$.t", "0$.s", "s=:s+1", "0$.s"] {
        let normalize = |r: rustj::Result<Option<Value>>| r.map(|v| v.map(|x| x.json()));
        assert_eq!(
            normalize(direct.eval(source)),
            normalize(ir.eval_semantic_reference(source)),
            "{source}"
        );
    }
}

#[test]
fn analysis_separates_element_type_from_sparse_layout() {
    use rustj::facts::RepresentationClassFact;
    let mut e = Engine::new();
    e.eval("s=:$.0 2 0 4").unwrap();
    let plan = e.analyze_a3("s").unwrap();
    let facts = &plan.values[plan.result.unwrap().0].facts;
    assert_eq!(facts.dtype, TypeFact::Exact(DType::Int));
    assert_eq!(
        facts.representation_class,
        RepresentationClassFact::AxisSparse
    );
    let plan = e.analyze_a3("$.1 2 3").unwrap();
    let facts = &plan.values[plan.result.unwrap().0].facts;
    assert_eq!(
        facts.representation_class,
        RepresentationClassFact::AxisSparse
    );
    assert_eq!(facts.shape, Some(vec![3]));
    let plan = e.analyze_a3("$.42").unwrap();
    assert_eq!(
        plan.values[plan.result.unwrap().0]
            .facts
            .representation_class,
        RepresentationClassFact::Dense
    );
}
