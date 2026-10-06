#![deny(unsafe_code)]

mod array_ops;
mod assembly;
mod comparison_policy;
mod index_ops;
mod search_reference;
// Research-only tolerant lookup proof harness; never an executable fast path.
#[cfg(test)]
mod tolerant_search;

pub mod analysis;
pub mod bit_storage;
pub mod compilation;
pub mod contracts;
pub mod definition_code;
pub mod definition_control;
pub mod definition_flow;
pub mod definition_input;
pub mod enqueuer;
pub mod error;
pub mod execution_semantics;
pub mod expansion;
pub mod facts;
pub mod fusion_planning;
pub mod j_graph_composition;
pub mod j_graph_fusion;
pub mod j_graph_ir;
pub mod j_graph_jsource;
pub mod j_graph_memory;
pub mod j_graph_resource;
pub mod j_graph_rewrite;
pub mod j_graph_scan;
pub mod j_graph_work_depth;
pub mod kernels;
pub mod logical_executor;
pub mod logical_ir;
pub mod lowering;
mod numeric;
mod numeric_input;
pub mod opportunity;
pub mod parser;
pub mod parser_capture;
pub mod physical;
pub mod physical_plan;
mod pool;
pub mod primitive;
pub mod runtime;
pub mod scanner;
pub mod semantic;
#[allow(unsafe_code)]
mod simd;
pub mod sparse;
pub mod static_analysis;
pub mod storage;
pub mod syntax;
pub mod tokenizer;
pub mod types;
pub mod value;

pub use error::{Error, Result};
pub use runtime::Engine;
pub use value::{Data, Value};
