//! Explain graph/memory using metadata only; no data array is allocated.
use rustj::{
    facts::TypeFact, j_graph_ir::GraphFacts, static_analysis::StaticAnalyzer, types::DType,
};
fn main() -> rustj::Result<()> {
    let source = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "(+/ % #) data".into());
    let mut analyzer = StaticAnalyzer::new();
    analyzer.declare_noun(
        "data",
        GraphFacts {
            dtype: TypeFact::Exact(DType::Float),
            shape: Some(vec![1_000_000_000_000]),
            rank: Some(1),
        },
    )?;
    let report = analyzer.analyze(&source)?;
    println!("Source: {source}");
    println!("Input: data, Float, shape [1000000000000] (metadata only)");
    for (index, node) in report.graph.nodes.iter().enumerate() {
        println!("Value {index}: {:?}; facts {:?}", node.kind, node.facts);
    }
    println!(
        "Logical atoms across known values: {} (not peak memory)",
        report.memory.known_logical_atoms
    );
    println!("Graph-order live ranges: {:?}", report.memory.live_ranges);
    println!(
        "Materialization opportunities: {:?}",
        report.memory.opportunities
    );
    println!("Analysis boundaries: {:?}", report.boundaries);
    Ok(())
}
