#![deny(unsafe_code)]

mod array_ops;
mod assembly;
mod index_ops;

pub mod analysis;
pub mod bit_storage;
pub mod contracts;
pub mod error;
pub mod facts;
pub mod kernels;
pub mod logical_ir;
mod numeric;
pub mod physical;
mod pool;
pub mod primitive;
pub mod runtime;
pub mod scanner;
pub mod semantic;
#[allow(unsafe_code)]
mod simd;
pub mod sparse;
pub mod storage;
pub mod syntax;
pub mod types;
pub mod value;

pub use error::{Error, Result};
pub use runtime::Engine;
pub use value::{Data, Value};
