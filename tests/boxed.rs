use rustj::{Data, Engine, Error, Value};
use std::sync::Arc;
fn eval(e: &mut Engine, text: &str) -> Value {
    e.eval(text).unwrap().unwrap()
}

#[test]
fn box_open_nesting_shape_and_aliases() {
    let mut e = Engine::new();
    let boxed = eval(&mut e, "< i. 2 3");
    assert_eq!(boxed.type_code(), 32);
    assert_eq!(boxed.shape(), &[]);
    let Data::Boxed(contents) = boxed.data() else {
        panic!()
    };
    assert_eq!(contents[0].shape(), &[2, 3]);
    assert_eq!(
        eval(&mut e, "> < i. 2 3").json(),
        eval(&mut e, "i. 2 3").json()
    );
    assert_eq!(eval(&mut e, "> < < 42").type_code(), 32);
    assert_eq!(eval(&mut e, "> > < < 42").int_at(0).unwrap(), 42);
    assert_eq!(eval(&mut e, "> 1 2 3").json(), eval(&mut e, "1 2 3").json());
    e.eval("a=:i. 4").unwrap();
    e.eval("b=:<a").unwrap();
    e.eval("a=:a+10").unwrap();
    assert_eq!(eval(&mut e, ">b").json(), eval(&mut e, "i.4").json());
    e.eval("c=:>b").unwrap();
    e.eval("c=:c+20").unwrap();
    assert_eq!(eval(&mut e, ">b").json(), eval(&mut e, "i.4").json());
    assert!(
        boxed
            .json()
            .contains("\"data\":[{\"type\":4,\"shape\":[2,3]")
    );
}

#[test]
fn boxed_arrays_rank_select_and_shared_contents() {
    let mut e = Engine::new();
    e.eval("b=: (< 1 2),< 'abc'").unwrap();
    let boxes = eval(&mut e, "b");
    assert_eq!(boxes.shape(), &[2]);
    assert_eq!(eval(&mut e, ">0{b").json(), eval(&mut e, "1 2").json());
    assert_eq!(eval(&mut e, ">1{b").display(), "abc");
    let reshaped = eval(&mut e, "2 2$b");
    let Data::Boxed(v) = reshaped.data() else {
        panic!()
    };
    assert!(Arc::ptr_eq(&v[0], &v[2]));
    assert_eq!(eval(&mut e, ">0{|.b").display(), "abc");
    assert_eq!(eval(&mut e, "<\"0 i.4").shape(), &[4]);
    assert_eq!(
        eval(&mut e, ">\"0 <\"0 i.4").json(),
        eval(&mut e, "i.4").json()
    );
    assert_eq!(
        reshaped
            .view()
            .cell(1, 0)
            .unwrap()
            .to_owned()
            .unwrap()
            .type_code(),
        32
    );
}

#[test]
fn unsupported_box_operations_are_explicit_and_transactional() {
    let mut e = Engine::new();
    e.eval("b=:<1 2").unwrap();
    for text in [
        "b+2",
        "b=b",
        "b i. b",
        "b e. b",
        ">( <1 2),<1 2 3",
        "3{.2$b",
    ] {
        assert!(matches!(e.eval(text), Err(Error::Unsupported(_))), "{text}");
    }
    assert!(e.eval("b=:b+2").is_err());
    assert_eq!(eval(&mut e, ">b").json(), eval(&mut e, "1 2").json());
    assert_eq!(eval(&mut e, "1<2").int_at(0).unwrap(), 1);
    assert_eq!(eval(&mut e, "1>2").int_at(0).unwrap(), 0);
}

#[test]
fn uniform_open_assembles_shapes_promotes_and_preserves_nested_boxes() {
    let mut e = Engine::new();
    for (source, expected) in [
        (">(<1 2),<3 4", "2 2$1 2 3 4"),
        (">(<1 2),<3.5 4.5", "2 2$1 2 3.5 4.5"),
        (">2 2$<1 2 3", "2 2 3$1 2 3"),
        (">2$<i.0", "2 0$i.0"),
        (">0$<3", "0$<3"),
        ("> > (< <1),< <2", "1 2"),
    ] {
        assert_eq!(
            eval(&mut e, source).json(),
            eval(&mut e, expected).json(),
            "{source}"
        );
    }
    assert_eq!(e.eval(">(<1 2),<'ab'").unwrap_err(), Error::Domain);
    let mut direct = Engine::new();
    let mut reference = Engine::new();
    for source in [
        "a=:i.4",
        "b=:<a",
        "a=:a+1",
        ">b",
        "<\"0 i.3",
        ">(<1 2),<3 4",
    ] {
        assert_eq!(
            direct.eval(source).unwrap().map(|v| v.json()),
            reference
                .eval_semantic_reference(source)
                .unwrap()
                .map(|v| v.json())
        );
    }
}
