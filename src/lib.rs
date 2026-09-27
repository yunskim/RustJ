#![deny(unsafe_code)]

mod array_ops;
mod assembly;
mod index_ops;

pub mod error;
pub mod kernels;
mod numeric;
mod pool;
pub mod runtime;
#[allow(unsafe_code)]
mod simd;
pub mod storage;
pub mod syntax;
pub mod value;

pub use error::{Error, Result};
pub use runtime::Engine;
pub use value::{Data, Value};
