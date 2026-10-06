//! FW-02 independent, sequential J item-search reference.
//!
//! This module must NOT call index_ops, physical search planning, hash tables,
//! prehash caches, or graph search descriptors. It owns the control flow for
//! first/last index and membership, while sharing the explicit semantic
//! comparison-policy snapshot (currently fixed RustJ near, not full J CCT).
//! It is a supported-dense-subset oracle, not a proof of boxed/sparse J semantics.

use crate::{
    comparison_policy::ComparisonPolicySnapshot,
    storage::{CpuStorage, Shape},
    value::{count, Data},
    Error, Result, Value,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SearchMode {
    First,
    Last,
    Membership,
}

fn supported_dense(value: &Value) -> bool {
    matches!(
        value.data,
        Data::Bool(_) | Data::Int(_) | Data::Float(_) | Data::Char(_)
    )
}

fn same_atom(
    indexed: &Value,
    index_position: usize,
    query: &Value,
    query_position: usize,
    comparison: ComparisonPolicySnapshot,
) -> bool {
    match (&indexed.data, &query.data) {
        (Data::Char(a), Data::Char(b)) => a[index_position] == b[query_position],
        (Data::Char(_), _) | (_, Data::Char(_)) => false,
        (Data::Float(_), _) | (_, Data::Float(_)) => comparison.float_equal(
            indexed.float_at(index_position).expect("supported dense"),
            query.float_at(query_position).expect("supported dense"),
        ),
        _ => {
            indexed.int_at(index_position).expect("supported dense")
                == query.int_at(query_position).expect("supported dense")
        }
    }
}

fn lookup(indexed: &Value, query: &Value, mode: SearchMode) -> Result<Value> {
    // Item axis / query frame are semantic information, not physical routing.
    let items = indexed.shape().first().copied().unwrap_or(1);
    let item_shape = indexed.shape().get(1..).unwrap_or(&[]);
    let frame_rank = query.shape().len().saturating_sub(item_shape.len());
    let result_shape = Shape::from(&query.shape()[..frame_rank]);
    let query_items = count(&result_shape)?;
    let compatible = query.shape().len() >= item_shape.len()
        && &query.shape()[frame_rank..] == item_shape;
    let cell_atoms = count(item_shape)?;
    // Match the current supported-dense contract; do not pretend that
    // recursive boxed equality or sparse-item search has been validated.
    if !supported_dense(indexed) || !supported_dense(query) {
        return Err(Error::Unsupported("reference boxed/sparse index-of".into()));
    }

    let comparison = ComparisonPolicySnapshot::fixed_rust_near();
    let first_or_last = |q: usize| -> usize {
        if !compatible {
            return items;
        }
        let matches = |i: usize| -> bool {
            let left = i * cell_atoms;
            let right = q * cell_atoms;
            (0..cell_atoms).all(|offset| {
                same_atom(indexed, left + offset, query, right + offset, comparison)
            })
        };
        match mode {
            SearchMode::Last => (0..items).rev().find(|&i| matches(i)),
            SearchMode::First | SearchMode::Membership => {
                (0..items).find(|&i| matches(i))
            }
        }
        .unwrap_or(items)
    };
    match mode {
        SearchMode::Membership => Value::new(
            result_shape,
            Data::Bool(CpuStorage::generate(query_items, |q| {
                (first_or_last(q) != items) as u8
            })?),
        ),
        SearchMode::First | SearchMode::Last => Value::new(
            result_shape,
            Data::Int(CpuStorage::generate(query_items, |q| first_or_last(q) as i64)?),
        ),
    }
}

pub(crate) fn index_of(indexed: &Value, query: &Value, last: bool) -> Result<Value> {
    lookup(
        indexed,
        query,
        if last {
            SearchMode::Last
        } else {
            SearchMode::First
        },
    )
}

pub(crate) fn member(query: &Value, indexed: &Value) -> Result<Value> {
    lookup(indexed, query, SearchMode::Membership)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index_ops;

    fn equal_results(reference: Result<Value>, optimized: Result<Value>) {
        match (reference, optimized) {
            (Ok(a), Ok(b)) => assert_eq!(a.json(), b.json()),
            (Err(a), Err(b)) => assert_eq!(a.kind(), b.kind()),
            (a, b) => panic!("reference/optimized outcome mismatch: {a:?} vs {b:?}"),
        }
    }

    fn verify(indexed: Value, queries: Value) {
        equal_results(
            index_of(&indexed, &queries, false),
            index_ops::index_of(indexed.clone(), queries.clone(), false),
        );
        equal_results(
            index_of(&indexed, &queries, true),
            index_ops::index_of(indexed.clone(), queries.clone(), true),
        );
        equal_results(
            member(&queries, &indexed),
            index_ops::member(queries, indexed),
        );
    }

    #[test]
    fn sequential_reference_independently_checks_hash_direct_reverse_and_empty_shapes() {
        for items in [0usize, 1, 3, 33, 128] {
            for queries in [0usize, 1, 2, 17, 130] {
                let keys = (0..items).map(|i| (i % 7) as i64 - 3).collect();
                let probes = (0..queries).map(|i| (i % 11) as i64 - 5).collect();
                verify(Value::ints([items], keys).unwrap(), Value::ints([queries], probes).unwrap());
            }
        }
        verify(
            Value::ints([4], vec![i64::MIN, 0, i64::MAX, i64::MIN]).unwrap(),
            Value::ints([4], vec![i64::MAX, i64::MIN, 1, 0]).unwrap(),
        );
        verify(
            Value::ints([3, 2], vec![1, 2, 1, 2, 3, 4]).unwrap(),
            Value::ints([3, 2], vec![1, 2, 4, 3, 3, 4]).unwrap(),
        );
        verify(
            Value::ints([3, 2], vec![1, 2, 1, 2, 3, 4]).unwrap(),
            Value::ints([2, 3], vec![1, 2, 3, 4, 5, 6]).unwrap(),
        );
        verify(
            Value::ints([3, 0], Vec::new()).unwrap(),
            Value::ints([2, 0], Vec::new()).unwrap(),
        );
        verify(
            Value::ints([], vec![7]).unwrap(),
            Value::ints([3], vec![7, 8, 7]).unwrap(),
        );
    }

    #[test]
    fn sequential_reference_preserves_nontransitive_float_representatives() {
        let t = 2f64.powi(-44);
        for items in [0usize, 1, 3, 32, 100] {
            let pool = [
                1.0,
                1.0 + 0.75 * t,
                1.0 + 1.5 * t,
                0.0,
                -0.0,
                f64::NAN,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::from_bits(1),
            ];
            let data = (0..items).map(|i| pool[i % pool.len()]).collect();
            let indexed = Value::new([items], Data::Float(CpuStorage::new(data))).unwrap();
            let queries = Value::new(
                [pool.len()],
                Data::Float(CpuStorage::new(pool.to_vec())),
            )
            .unwrap();
            verify(indexed, queries);
        }
    }

    #[test]
    fn sequential_reference_covers_boolean_integer_and_character_cross_types() {
        verify(
            Value::new([4], Data::Bool(CpuStorage::new(vec![0, 1, 1, 0]))).unwrap(),
            Value::ints([4], vec![1, 2, 0, -1]).unwrap(),
        );
        verify(
            Value::new([4], Data::Char(CpuStorage::new(b"abba".to_vec()))).unwrap(),
            Value::new([3], Data::Char(CpuStorage::new(b"abc".to_vec()))).unwrap(),
        );
        verify(
            Value::new([3], Data::Char(CpuStorage::new(b"abc".to_vec()))).unwrap(),
            Value::ints([3], vec![97, 98, 99]).unwrap(),
        );
    }
}
