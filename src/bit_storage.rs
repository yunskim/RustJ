//! Immutable packed booleans. This is a physical layout, not a new J atom type.
//! Runtime Bool arrays still use byte storage until dispatch integration is ready.
use crate::{Error, Result, value::buffer};
use std::{ops::Range, sync::Arc};

#[derive(Clone, Debug)]
pub struct BitStorage {
    words: Arc<[u64]>,
    offset: usize,
    len: usize,
}
impl BitStorage {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.iter().any(|&b| b > 1) {
            return Err(Error::Domain);
        }
        let mut words = buffer(bytes.len().div_ceil(64))?;
        for chunk in bytes.chunks(64) {
            let mut word = 0;
            for (bit, &b) in chunk.iter().enumerate() {
                word |= (b as u64) << bit;
            }
            words.push(word);
        }
        Ok(Self {
            words: words.into(),
            offset: 0,
            len: bytes.len(),
        })
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    /// Underlying allocation size; sliced views retain their parent's allocation.
    pub fn backing_bytes(&self) -> usize {
        std::mem::size_of_val(self.words.as_ref())
    }
    pub fn get(&self, index: usize) -> Option<bool> {
        if index >= self.len {
            return None;
        }
        let bit = self.offset + index;
        Some((self.words[bit / 64] >> (bit % 64)) & 1 != 0)
    }
    pub fn slice(&self, range: Range<usize>) -> Result<Self> {
        if range.start > range.end || range.end > self.len {
            return Err(Error::Index);
        }
        Ok(Self {
            words: self.words.clone(),
            offset: self.offset + range.start,
            len: range.end - range.start,
        })
    }
    pub fn to_bytes(&self, max_atoms: usize) -> Result<Vec<u8>> {
        if self.len > max_atoms {
            return Err(Error::Limit);
        }
        let mut out = buffer(self.len)?;
        for i in 0..self.len {
            out.push(self.get(i).unwrap() as u8);
        }
        Ok(out)
    }
    // Return a logical word with all unused tail bits cleared, even for slices.
    fn word(&self, index: usize) -> u64 {
        let logical = index * 64;
        let bit = self.offset + logical;
        let shift = bit % 64;
        let mut value = self.words[bit / 64] >> shift;
        if shift != 0 {
            value |= self.words.get(bit / 64 + 1).copied().unwrap_or(0) << (64 - shift);
        }
        let valid = (self.len - logical).min(64);
        if valid < 64 {
            value &= (1u64 << valid) - 1;
        }
        value
    }
    pub fn count_ones(&self) -> usize {
        (0..self.len.div_ceil(64))
            .map(|i| self.word(i).count_ones() as usize)
            .sum()
    }
    pub fn and(&self, rhs: &Self) -> Result<Self> {
        self.zip(rhs, |a, b| a & b)
    }
    pub fn or(&self, rhs: &Self) -> Result<Self> {
        self.zip(rhs, |a, b| a | b)
    }
    pub fn xor(&self, rhs: &Self) -> Result<Self> {
        self.zip(rhs, |a, b| a ^ b)
    }
    fn zip(&self, rhs: &Self, f: impl Fn(u64, u64) -> u64) -> Result<Self> {
        if self.len != rhs.len {
            return Err(Error::Length);
        }
        let n = self.len.div_ceil(64);
        let mut words = buffer(n)?;
        for i in 0..n {
            words.push(f(self.word(i), rhs.word(i)));
        }
        Ok(Self {
            words: words.into(),
            offset: 0,
            len: self.len,
        })
    }
}
