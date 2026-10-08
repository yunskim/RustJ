//! Assemble rank results directly, retaining only one cell and the output.
use crate::{
    error::{Error, Result},
    storage::CpuStorage,
    value::{Data, Value, buffer},
};

enum Output {
    Bool(Vec<u8>),
    Int(Vec<i64>),
    Float(Vec<f64>),
    Char(Vec<u8>),
    Boxed(Vec<std::sync::Arc<Value>>),
}
pub(crate) struct CellBuilder {
    out: Output,
    capacity: usize,
}
impl CellBuilder {
    pub(crate) fn new(first: &Value, capacity: usize) -> Result<Self> {
        let out = match first.data() {
            Data::ExtendedInt(_) => {
                return Err(Error::Unsupported("extended rank assembly".into()));
            }
            Data::Sparse(_) => return Err(Error::Unsupported("sparse assembly".into())),
            Data::Bool(_) => Output::Bool(buffer(capacity)?),
            Data::Int(_) => Output::Int(buffer(capacity)?),
            Data::Float(_) => Output::Float(buffer(capacity)?),
            Data::Char(_) => Output::Char(buffer(capacity)?),
            Data::Boxed(_) => Output::Boxed(buffer(capacity)?),
        };
        let mut builder = Self { out, capacity };
        builder.push(first)?;
        Ok(builder)
    }
    pub(crate) fn push(&mut self, cell: &Value) -> Result<()> {
        if cell.is_extended() {
            return Err(Error::Unsupported("extended rank assembly".into()));
        }
        if cell.is_sparse() {
            return Err(Error::Unsupported("sparse assembly".into()));
        }
        let len = match &self.out {
            Output::Bool(v) | Output::Char(v) => v.len(),
            Output::Int(v) => v.len(),
            Output::Float(v) => v.len(),
            Output::Boxed(v) => v.len(),
        };
        if cell.len() > self.capacity - len {
            return Err(Error::Length);
        }
        if matches!(self.out, Output::Char(_)) != matches!(cell.data(), Data::Char(_)) {
            return Err(Error::Domain);
        }
        if matches!(self.out, Output::Boxed(_)) != matches!(cell.data(), Data::Boxed(_)) {
            return Err(Error::Domain);
        }
        // Promotion includes every earlier cell, including empty cells' types.
        if matches!(cell.data(), Data::Float(_)) && !matches!(self.out, Output::Float(_)) {
            let mut v = buffer(self.capacity)?;
            match &self.out {
                Output::Bool(old) => v.extend(old.iter().map(|&x| x as f64)),
                Output::Int(old) => v.extend(old.iter().map(|&x| x as f64)),
                _ => unreachable!(),
            }
            self.out = Output::Float(v);
        } else if matches!(cell.data(), Data::Int(_)) && matches!(self.out, Output::Bool(_)) {
            let Output::Bool(old) = &self.out else {
                unreachable!()
            };
            let mut v = buffer(self.capacity)?;
            v.extend(old.iter().map(|&x| x as i64));
            self.out = Output::Int(v);
        }
        match (&mut self.out, cell.data()) {
            (Output::Bool(out), Data::Bool(v)) | (Output::Char(out), Data::Char(v)) => {
                out.extend_from_slice(v)
            }
            (Output::Boxed(out), Data::Boxed(v)) => out.extend_from_slice(v),
            (Output::Int(out), Data::Int(v)) => out.extend_from_slice(v),
            (Output::Float(out), Data::Float(v)) => out.extend_from_slice(v),
            (Output::Int(out), Data::Bool(v)) => out.extend(v.iter().map(|&x| x as i64)),
            (Output::Float(out), Data::Bool(v)) => out.extend(v.iter().map(|&x| x as f64)),
            (Output::Float(out), Data::Int(v)) => out.extend(v.iter().map(|&x| x as f64)),
            _ => unreachable!(),
        }
        Ok(())
    }
    pub(crate) fn finish(self) -> Data {
        match self.out {
            Output::Bool(v) => Data::Bool(CpuStorage::new(v)),
            Output::Int(v) => Data::Int(CpuStorage::new(v)),
            Output::Float(v) => Data::Float(CpuStorage::new(v)),
            Output::Char(v) => Data::Char(CpuStorage::new(v)),
            Output::Boxed(v) => Data::Boxed(CpuStorage::new(v)),
        }
    }
}
