#![deny(unsafe_code)]

mod array_ops;
mod assembly;
mod index_ops;

pub mod analysis;
pub mod contracts;
pub mod error;
pub mod facts;
pub mod kernels;
mod numeric;
mod pool;
pub mod primitive;
pub mod runtime;
pub mod scanner;
pub mod semantic;
#[allow(unsafe_code)]
mod simd;
pub mod storage;
pub mod syntax;
pub mod value;

pub use error::{Error, Result};
pub use runtime::Engine;
pub use value::{Data, Value};
