//! Analysis-only JSON-line observation: noun expression TAB applied verb.
//! Executable prefix coverage is reported separately from Scan recognition.
use rustj::{Engine, facts::TypeFact, types::DType};
use std::io::{self, BufRead};

fn main() {
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        let Some((noun, verb)) = line.split_once('\t') else {
            panic!("noun TAB verb required")
        };
        let mut engine = Engine::new();
        engine.eval(&format!("scaninput=: {noun}")).unwrap();
        let input = engine.eval("scaninput").unwrap().unwrap();
        let source = format!("{verb} scaninput");
        let graph = engine.analyze_j_graph(&source).unwrap();
        let analysis = graph.scan_analysis().unwrap();
        analysis.verify(&graph).unwrap();
        let contract = analysis.candidates.first().map_or("null".into(), |candidate| {
            let c = &candidate.contract;
            let dtype = match c.output_facts.dtype {
                TypeFact::Exact(DType::Bool) => 1,
                TypeFact::Exact(DType::Int) => 4,
                _ => panic!("unexpected Scan output type"),
            };
            format!("{{\"type\":{dtype},\"shape\":{:?},\"items\":{},\"atoms\":{},\"parallel_authorized\":{},\"inject_identity\":{},\"returns_input_atoms\":{}}}",
                c.output_facts.shape.as_ref().unwrap(), c.items, c.atoms,
                c.parallel_prefix_authorized, c.inject_identity, c.returns_input_atoms)
        });
        let reasons = analysis
            .boundaries
            .iter()
            .map(|b| format!("\"{:?}\"", b.reason))
            .collect::<Vec<_>>()
            .join(",");
        let runtime = match engine.eval(&source) {
            Ok(Some(value)) => value.json(),
            Ok(None) => "{\"silent\":true}".into(),
            Err(error) => format!("{{\"error\":\"{}\"}}", error.kind()),
        };
        println!(
            "{{\"input\":{},\"contract\":{contract},\"boundaries\":[{reasons}],\"runtime\":{runtime}}}",
            input.json()
        );
    }
}
