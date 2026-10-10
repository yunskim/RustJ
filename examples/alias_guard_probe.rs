//! Finite oracle probe for the explicit guarded call API, not a CLI route.
use rustj::{
    Engine, Result, Value,
    frontend_context::NameUseId,
    name_guards::{AliasCall, AliasCallAttempt},
};

fn outcome(result: Result<Option<Value>>) -> String {
    match result {
        Ok(Some(value)) => value.json(),
        Ok(None) => "{\"silent\":true}".into(),
        Err(error) => format!("{{\"error\":\"{}\"}}", error.kind()),
    }
}

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 4, "primitive, monad|dyad|effect-miss, x, y");
    assert!(["+", "-", "*", "%"].contains(&args[0].as_str()));
    let mut engine = Engine::new();
    for source in [format!("f=:{}", args[0]), "g=:f".into(), "h=:g".into()] {
        engine.eval(&source).unwrap();
    }
    let observed = engine.eval_captured("h 2");
    observed.result.unwrap();
    let guard = engine
        .prepare_alias_call_guard(observed.capture.frontend.as_ref().unwrap(), NameUseId(0))
        .unwrap();
    if args[1].ends_with("-miss") {
        match args[1].as_str() {
            "effect-miss" => {
                engine.eval("count=:0").unwrap();
                engine
                    .eval("change=:{{ count=:count+1\nf=:*\nu }}")
                    .unwrap();
                engine.eval("argument=:_2 change").unwrap();
            }
            "noun-miss" | "missing-miss" => {
                engine
                    .eval(if args[1] == "noun-miss" {
                        "f=:7"
                    } else {
                        "f=:later"
                    })
                    .unwrap();
                engine.eval("argument=:_2").unwrap();
            }
            _ => panic!("unknown miss probe"),
        }
        let y = engine.eval("argument").unwrap().unwrap();
        let AliasCallAttempt::Miss(miss) =
            engine.try_alias_call(AliasCall::new(std::sync::Arc::new(guard), None, y))
        else {
            panic!("expected miss")
        };
        let AliasCallAttempt::Executed(result) = engine.resume_alias_call(miss) else {
            panic!("unexpected scope refusal")
        };
        let result = outcome(result.map(Some));
        if args[1] == "effect-miss" {
            let count = outcome(engine.eval("count"));
            println!("{{\"guard\":\"miss\",\"result\":{result},\"count\":{count}}}");
        } else {
            println!("{{\"guard\":\"miss\",\"result\":{result}}}");
        }
        return;
    }
    // Probe inputs are finite pure expressions. Evaluate y before x, then
    // acquire the lease immediately before its one kernel call.
    let y = engine.eval(&args[3]).unwrap().unwrap();
    let result = if args[1] == "monad" {
        engine
            .validate_alias_call_guard(&guard)
            .unwrap()
            .apply_monad(y)
    } else {
        assert_eq!(args[1], "dyad");
        let x = engine.eval(&args[2]).unwrap().unwrap();
        engine
            .validate_alias_call_guard(&guard)
            .unwrap()
            .apply_dyad(x, y)
    };
    let result = outcome(result.map(Some));
    println!("{{\"guard\":\"valid\",\"result\":{result}}}");
}
