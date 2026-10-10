//! Bounded audit adapter. Setup/check use J execution; effect executes only
//! the prepared ordered plan and never retries through the runtime parser.
use rustj::{Engine, Error};
use std::io::{self, BufRead};

fn main() {
    let mut engine = Engine::new();
    for line in io::stdin().lock().lines() {
        let line = line.expect("audit input");
        let result = match line.split_once('\t') {
            Some(("array", source)) => engine
                .prepare_name_arrays(source)
                .and_then(|plan| engine.execute_name_arrays(&plan).result),
            Some(("effect", source)) => engine
                .prepare_name_effects(source)
                .and_then(|plan| engine.execute_name_effects(&plan).result),
            Some(("eval", source)) => engine.eval(source),
            _ => Err(Error::Unsupported("audit input mode".into())),
        };
        match result {
            Ok(Some(value)) => println!("{}", value.json()),
            Ok(None) => println!("{{\"silent\":true}}"),
            Err(error) => println!("{{\"error\":\"{}\"}}", error.kind()),
        }
    }
}
