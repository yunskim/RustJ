//! Bounded engine-local reuse of retired integer payloads.
use crate::{Data, Value};
#[derive(Default)]
pub(crate) struct OutputPool {
    buffers: Vec<Vec<i64>>,
    bytes: usize,
    limit: usize,
    hits: usize,
}
impl OutputPool {
    pub(crate) fn new(limit: usize) -> Self {
        Self {
            limit,
            ..Self::default()
        }
    }
    pub(crate) fn take(&mut self, n: usize) -> crate::Result<Vec<i64>> {
        if let Some(i) = self.buffers.iter().position(|v| v.capacity() == n) {
            let out = self.buffers.swap_remove(i);
            self.bytes -= out.capacity() * 8;
            self.hits += 1;
            return Ok(out);
        }
        crate::value::buffer(n)
    }
    pub(crate) fn retire(&mut self, value: Value) {
        if let Data::Int(storage) = value.data {
            if let Ok(mut v) = storage.try_into_vec() {
                let bytes = v.capacity() * 8;
                if bytes >= 1024
                    && bytes <= self.limit.saturating_sub(self.bytes)
                    && self.buffers.len() < 16
                {
                    v.clear();
                    self.bytes += bytes;
                    self.buffers.push(v);
                }
            }
        }
    }
    pub(crate) fn stats(&self) -> (usize, usize) {
        (self.bytes, self.hits)
    }
    pub(crate) fn clear(&mut self) {
        self.buffers.clear();
        self.bytes = 0;
    }
}
