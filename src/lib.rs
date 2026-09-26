#![deny(unsafe_code)]

pub mod error;
pub mod kernels;
mod numeric;
pub mod runtime;
#[allow(unsafe_code)]
mod simd;
pub mod syntax;
pub mod value;

pub use error::{Error, Result};
pub use runtime::Engine;
pub use value::{Data, Value};
