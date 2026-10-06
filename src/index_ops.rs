//! Index generation and item search. Floating search preserves tolerant equality.
use crate::{
    Data, Error, Result, Value,
    storage::{CpuStorage, Shape},
    value::{buffer, count},
};
use std::{borrow::Cow, collections::HashMap, sync::Arc};

pub(crate) fn steps(y: Value) -> Result<Value> {
    if !y.shape.is_empty() {
        return Err(Error::Unsupported("i: over array (result padding)".into()));
    }
    let d = y.float_at(0)?;
    let steps = 2.0 * d.abs();
    if !steps.is_finite() || steps.fract() != 0.0 {
        return Err(Error::Domain);
    }
    if steps >= i64::MAX as f64 {
        return Err(Error::Limit);
    }
    let n = (steps as usize).checked_add(1).ok_or(Error::Limit)?;
    let data = if d.fract() == 0.0 {
        let start = -(d as i64);
        Data::Int(CpuStorage::generate(n, |i| {
            if d < 0.0 {
                start - i as i64
            } else {
                start + i as i64
            }
        })?)
    } else {
        Data::Float(CpuStorage::generate(n, |i| {
            if d < 0.0 {
                -d - i as f64
            } else {
                -d + i as f64
            }
        })?)
    };
    Value::new([n], data)
}

pub(crate) fn indices(y: Value) -> Result<Value> {
    if y.shape.len() > 1 {
        return Err(Error::Unsupported(
            "I. over higher rank (result padding)".into(),
        ));
    }
    let mut n = 0usize;
    for i in 0..y.len() {
        let k = usize::try_from(y.int_at(i)?).map_err(|_| Error::Domain)?;
        n = n.checked_add(k).ok_or(Error::Limit)?;
    }
    let mut out = buffer(n)?;
    for i in 0..y.len() {
        out.extend(std::iter::repeat_n(i as i64, y.int_at(i)? as usize));
    }
    Value::ints([n], out)
}

pub(crate) fn atom_eq(a: &Value, ai: usize, b: &Value, bi: usize) -> bool {
    match (&a.data, &b.data) {
        (Data::Char(x), Data::Char(y)) => x[ai] == y[bi],
        (Data::Char(_), _) | (_, Data::Char(_)) => false,
        (Data::Float(_), _) | (_, Data::Float(_)) => {
            crate::kernels::near(a.float_at(ai).unwrap(), b.float_at(bi).unwrap())
        }
        _ => a.int_at(ai).unwrap() == b.int_at(bi).unwrap(),
    }
}


/// J Index-Of output intent; the search index is an implementation detail,
/// not a new J value or a new Graph IR operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LookupResult {
    First,
    Last,
    Membership,
}

/// A bounded direct-address table is profitable for narrow integer domains.
/// Other exact scalar domains still use a conventional hash table. Neither
/// route is legal for tolerant float/boxed comparisons.
#[derive(Clone)]
enum ExactScalarIndex {
    Direct { minimum: i64, positions: Vec<usize> },
    Hashed(HashMap<i64, usize>),
    ReverseHashed(HashMap<i64, usize>),
}

impl ExactScalarIndex {
    fn find(&self, key: i64, missing: usize) -> usize {
        match self {
            Self::Direct { minimum, positions } => key
                .checked_sub(*minimum)
                .and_then(|delta| usize::try_from(delta).ok())
                .and_then(|offset| positions.get(offset))
                .copied()
                .unwrap_or(missing),
            Self::Hashed(entries) | Self::ReverseHashed(entries) => {
                entries.get(&key).copied().unwrap_or(missing)
            }
        }
    }
}

/// Reverse hash query keys (not the indexed array) and use a single directional
/// walk of indexed values. A key is resolved exactly once, so first/last
/// representative and duplicate query semantics do not depend on hash order.
fn reverse_exact_index(
    indexed: &Value,
    items: usize,
    queries: &Value,
    result: LookupResult,
) -> Result<ExactScalarIndex> {
    let mut entries = HashMap::new();
    entries.try_reserve(queries.len()).map_err(|_| Error::Limit)?;
    for q in 0..queries.len() {
        entries.entry(queries.int_at(q)?).or_insert(items);
    }
    let mut remaining = entries.len();
    if result == LookupResult::Last {
        for i in (0..items).rev() {
            if let Some(position) = entries.get_mut(&indexed.int_at(i)?) {
                if *position == items {
                    *position = i;
                    remaining -= 1;
                    if remaining == 0 { break; }
                }
            }
        }
    } else {
        for i in 0..items {
            if let Some(position) = entries.get_mut(&indexed.int_at(i)?) {
                if *position == items {
                    *position = i;
                    remaining -= 1;
                    if remaining == 0 { break; }
                }
            }
        }
    }
    Ok(ExactScalarIndex::ReverseHashed(entries))
}

/// Build a direct index only when table bytes are bounded and reasonably
/// proportional to the work. Use i128 when measuring the key span: subtracting
/// arbitrary i64 endpoints can overflow. An empty or tiny search needs no
/// indexing setup at all.
fn exact_scalar_index(
    values: &Value,
    items: usize,
    queries: usize,
    result: LookupResult,
    query_values: Option<&Value>,
) -> Result<Option<ExactScalarIndex>> {
    if items == 0 || queries == 0 || items.saturating_mul(queries) <= 32 {
        return Ok(None);
    }

    // Reverse hashing indexes the *smaller query set*. We still return indices
    // into the original indexed argument, respecting first/last occurrence.
    // It is legal only for exact scalar integers/booleans; callers enforce this.
    // Build a query-key table and scan indexed values in the direction needed
    // for the representative. Duplicate queries intentionally share a key.
    if let Some(query_values) = query_values {
        if items >= 64 && items / 2 > queries {
            return Ok(Some(reverse_exact_index(values, items, query_values, result)?));
        }
    }

    let mut minimum = i64::MAX;
    let mut maximum = i64::MIN;
    for i in 0..items {
        let key = values.int_at(i)?;
        minimum = minimum.min(key);
        maximum = maximum.max(key);
    }
    let span = i128::from(maximum) - i128::from(minimum) + 1;
    const MAX_DIRECT_ENTRIES: i128 = 65_536;
    let work_budget = items.saturating_add(queries).saturating_mul(4) as i128;
    let last = result == LookupResult::Last;

    if span <= MAX_DIRECT_ENTRIES && span <= work_budget {
        let width = span as usize;
        let mut positions = Vec::new();
        positions.try_reserve_exact(width).map_err(|_| Error::Limit)?;
        positions.resize(width, items);
        for i in 0..items {
            let offset = (i128::from(values.int_at(i)?) - i128::from(minimum)) as usize;
            if last || positions[offset] == items {
                positions[offset] = i;
            }
        }
        return Ok(Some(ExactScalarIndex::Direct { minimum, positions }));
    }

    let mut entries = HashMap::new();
    entries.try_reserve(items).map_err(|_| Error::Limit)?;
    for i in 0..items {
        let key = values.int_at(i)?;
        if last {
            entries.insert(key, i);
        } else {
            entries.entry(key).or_insert(i);
        }
    }
    Ok(Some(ExactScalarIndex::Hashed(entries)))
}


/// A physical algorithm is optional. Failure to allocate a *search table*
/// cannot introduce an observable Limit error if the original sequential J
/// lookup can execute without that table. Output-buffer errors remain errors.
fn optional_exact_scalar_index(
    values: &Value,
    items: usize,
    queries: usize,
    result: LookupResult,
    query_values: Option<&Value>,
) -> Result<Option<ExactScalarIndex>> {
    match exact_scalar_index(values, items, queries, result, query_values) {
        Err(Error::Limit) => Ok(None),
        result => result,
    }
}

/// One bounded, per-Engine, immutable exact-key prehash. Holding the shared
/// backing Arc makes pointer identity safe across binding replacement: a
/// redefined name with a new backing cannot reuse a stale index.
///
/// This is an implementation-side cache, not the J M. memo operation or an
/// A3 semantic assertion. There is deliberately no global cache.
#[derive(Default)]
pub(crate) struct ExactPrehashCache {
    prepared: Option<PreparedExactSearch>,
    builds: usize,
    hits: usize,
}

struct PreparedExactSearch {
    source: Value,
    representative: LookupResult,
    index: ExactScalarIndex,
}

fn same_shared_exact_source(a: &Value, b: &Value) -> bool {
    if a.shape != b.shape { return false; }
    match (&a.data, &b.data) {
        (Data::Int(CpuStorage::Shared(x)), Data::Int(CpuStorage::Shared(y))) =>
            Arc::ptr_eq(x, y),
        (Data::Bool(CpuStorage::Shared(x)), Data::Bool(CpuStorage::Shared(y))) =>
            Arc::ptr_eq(x, y),
        _ => false,
    }
}

impl ExactPrehashCache {
    /// Returns counters as (new table builds, successful reused probes).
    pub(crate) fn stats(&self) -> (usize, usize) {
        (self.builds, self.hits)
    }

    pub(crate) fn clear(&mut self) {
        self.prepared = None;
        self.builds = 0;
        self.hits = 0;
    }

    fn get_or_prepare(
        &mut self,
        indexed: &Value,
        items: usize,
        queries: usize,
        result: LookupResult,
    ) -> Result<Option<&ExactScalarIndex>> {
        const MAX_PREHASH_ITEMS: usize = 16_384;
        // A prehash requires immutable shared backing; never assume a name,
        // dtype or shape alone provides version/identity.
        if items < 64 || items > MAX_PREHASH_ITEMS || queries == 0
            || indexed.shape.len() != 1
            || !matches!(&indexed.data,
                Data::Int(CpuStorage::Shared(_)) | Data::Bool(CpuStorage::Shared(_)))
        {
            return Ok(None);
        }
        // Membership and First both need the first occurrence. Last is a
        // distinct cache key, even though the original value is the same.
        let representative = if result == LookupResult::Last {
            LookupResult::Last
        } else {
            LookupResult::First
        };
        let same = self.prepared.as_ref().is_some_and(|prepared|
            prepared.representative == representative
                && same_shared_exact_source(&prepared.source, indexed)
        );
        if same {
            self.hits = self.hits.saturating_add(1);
        } else {
            // This prehash intentionally builds the *indexed* set, even when
            // query-side reverse hash would be cheaper for a single query.
            // Reuse is justified only by subsequent calls to the same Arc.
            let Some(index) = optional_exact_scalar_index(
                indexed, items, queries, representative, None
            )? else {
                return Ok(None);
            };
            self.prepared = Some(PreparedExactSearch {
                source: indexed.clone(),
                representative,
                index,
            });
            self.builds = self.builds.saturating_add(1);
        }
        Ok(self.prepared.as_ref().map(|prepared| &prepared.index))
    }
}

fn result_from_positions(
    shape: Shape,
    queries: usize,
    items: usize,
    result: LookupResult,
    mut position: impl FnMut(usize) -> usize,
) -> Result<Value> {
    match result {
        LookupResult::Membership => Value::new(
            shape,
            Data::Bool(CpuStorage::generate(queries, |q| (position(q) != items) as u8)?),
        ),
        LookupResult::First | LookupResult::Last => Value::new(
            shape,
            Data::Int(CpuStorage::generate(queries, |q| position(q) as i64)?),
        ),
    }
}

/// Common search contract: the first argument is the index domain, the second
/// is the query collection. Output mode controls materialization: membership
/// returns booleans directly, never a temporary index vector.
fn lookup(indexed: Value, queries: Value, result: LookupResult, cache: Option<&mut ExactPrehashCache>) -> Result<Value> {
    let items = indexed.shape.first().copied().unwrap_or(1);
    let cell_shape = if indexed.shape.is_empty() {
        &[][..]
    } else {
        &indexed.shape[1..]
    };
    let frame = queries.shape.len().saturating_sub(cell_shape.len());
    let shape = Shape::from(&queries.shape[..frame]);
    let n = count(&shape)?;

    if queries.shape.len() < cell_shape.len() || &queries.shape[frame..] != cell_shape {
        return result_from_positions(shape, n, items, result, |_| items);
    }

    let cell = count(cell_shape)?;
    let exact_scalar = cell_shape.is_empty()
        && matches!(indexed.data, Data::Int(_) | Data::Bool(_))
        && matches!(queries.data, Data::Int(_) | Data::Bool(_));
    let exact: Option<Cow<'_, ExactScalarIndex>> = if exact_scalar {
        if let Some(cache) = cache {
            if let Some(cached) = cache.get_or_prepare(&indexed, items, n, result)? {
                Some(Cow::Borrowed(cached))
            } else {
                optional_exact_scalar_index(&indexed, items, n, result, Some(&queries))?
                    .map(Cow::Owned)
            }
        } else {
            optional_exact_scalar_index(&indexed, items, n, result, Some(&queries))?
                .map(Cow::Owned)
        }
    } else {
        None
    };
    result_from_positions(shape, n, items, result, |q| {
        if let Some(index) = &exact {
            return index.find(queries.int_at(q).unwrap(), items);
        }
        let equal = |i: usize| {
            (0..cell).all(|k| atom_eq(&indexed, i * cell + k, &queries, q * cell + k))
        };
        match result {
            LookupResult::Last => (0..items).rev().find(|&i| equal(i)),
            LookupResult::First | LookupResult::Membership => (0..items).find(|&i| equal(i)),
        }
        .unwrap_or(items)
    })
}

pub(crate) fn index_of(a: Value, b: Value, last: bool) -> Result<Value> {
    lookup(a, b, if last { LookupResult::Last } else { LookupResult::First }, None)
}

pub(crate) fn member(a: Value, b: Value) -> Result<Value> {
    lookup(b, a, LookupResult::Membership, None)
}


/// Interpreter-only, name-identity guarded prehash reuse. Reference execution
/// continues to call the ordinary index_of/member routines.
pub(crate) fn index_of_cached(
    a: Value, b: Value, last: bool, cache: &mut ExactPrehashCache
) -> Result<Value> {
    lookup(a, b, if last { LookupResult::Last } else { LookupResult::First }, Some(cache))
}

pub(crate) fn member_cached(
    a: Value, b: Value, cache: &mut ExactPrehashCache
) -> Result<Value> {
    lookup(b, a, LookupResult::Membership, Some(cache))
}

pub(crate) fn find(a: Value, b: Value) -> Result<Value> {
    if a.shape.len() > b.shape.len() {
        return Err(Error::Rank);
    }
    if a.shape.len() > 1 || b.shape.len() > 1 {
        return Err(Error::Unsupported("E. multidimensional pattern".into()));
    }
    let width = a.len();
    Value::new(
        b.shape.clone(),
        Data::Bool(CpuStorage::generate(b.len(), |i| {
            (width <= b.len() - i && (0..width).all(|k| atom_eq(&a, k, &b, i + k))) as u8
        })?),
    )
}


#[cfg(test)]
mod index_family_tests {
    use super::{ExactScalarIndex, LookupResult, exact_scalar_index};
    use crate::Value;

    #[test]
    fn narrow_domain_uses_direct_table_without_changing_duplicate_policy() {
        let source = Value::ints([5], vec![-2, -1, 0, -2, 1]).unwrap();
        let first = exact_scalar_index(&source, 5, 7, LookupResult::First, None)
            .unwrap()
            .unwrap();
        let last = exact_scalar_index(&source, 5, 7, LookupResult::Last, None)
            .unwrap()
            .unwrap();
        assert!(matches!(first, ExactScalarIndex::Direct { .. }));
        assert!(matches!(last, ExactScalarIndex::Direct { .. }));
        assert_eq!(first.find(-2, 5), 0);
        assert_eq!(last.find(-2, 5), 3);
        assert_eq!(first.find(1, 5), 4);
        assert_eq!(first.find(i64::MAX, 5), 5);
    }

    #[test]
    fn wide_or_overflowing_i64_domain_uses_hash_without_span_overflow() {
        let source = Value::ints([3], vec![i64::MIN, 0, i64::MAX]).unwrap();
        let indexed = exact_scalar_index(&source, 3, 12, LookupResult::First, None)
            .unwrap()
            .unwrap();
        assert!(matches!(indexed, ExactScalarIndex::Hashed(_)));
        assert_eq!(indexed.find(i64::MIN, 3), 0);
        assert_eq!(indexed.find(0, 3), 1);
        assert_eq!(indexed.find(i64::MAX, 3), 2);
        assert_eq!(indexed.find(4, 3), 3);
    }

    #[test]
    fn tiny_search_avoids_index_setup() {
        let source = Value::ints([3], vec![1, 2, 3]).unwrap();
        assert!(exact_scalar_index(&source, 3, 2, LookupResult::Membership, None)
            .unwrap()
            .is_none());
    }

    #[test]
    fn reverse_hash_uses_query_keys_and_preserves_duplicate_representatives() {
        let source = Value::ints(
            [100],
            (0..100).map(|i| if i % 9 == 0 { 7 } else { i as i64 }).collect(),
        ).unwrap();
        let queries = Value::ints([3], vec![7, 7, 1000]).unwrap();
        let first = exact_scalar_index(&source, 100, 3, LookupResult::First, Some(&queries))
            .unwrap().unwrap();
        let last = exact_scalar_index(&source, 100, 3, LookupResult::Last, Some(&queries))
            .unwrap().unwrap();
        assert!(matches!(first, ExactScalarIndex::ReverseHashed(_)));
        assert!(matches!(last, ExactScalarIndex::ReverseHashed(_)));
        assert_eq!(first.find(7, 100), 0);
        assert_eq!(last.find(7, 100), 99);
        assert_eq!(first.find(1000, 100), 100);
    }

    #[test]
    fn reverse_hash_can_short_circuit_after_all_query_keys_resolve() {
        let source = Value::ints([200], (0..200).map(i64::from).collect()).unwrap();
        let queries = Value::ints([2], vec![1, 2]).unwrap();
        let first = exact_scalar_index(&source, 200, 2, LookupResult::Membership, Some(&queries))
            .unwrap().unwrap();
        assert_eq!(first.find(1, 200), 1);
        assert_eq!(first.find(2, 200), 2);
    }

    #[test]
    fn prehash_reuses_immutable_shared_backing_and_first_for_membership() {
        let source = Value::ints([128], (0..128).map(i64::from).collect())
            .unwrap().into_shared();
        let mut cache = super::ExactPrehashCache::default();
        let first = cache.get_or_prepare(&source, 128, 2, LookupResult::First)
            .unwrap().unwrap();
        assert_eq!(first.find(17, 128), 17);
        assert_eq!(cache.stats(), (1, 0));
        let copied = source.clone();
        let member = cache.get_or_prepare(&copied, 128, 2, LookupResult::Membership)
            .unwrap().unwrap();
        assert_eq!(member.find(17, 128), 17);
        assert_eq!(cache.stats(), (1, 1));

        let last = cache.get_or_prepare(&source, 128, 2, LookupResult::Last)
            .unwrap().unwrap();
        assert_eq!(last.find(17, 128), 17);
        assert_eq!(cache.stats(), (2, 1));
    }

    #[test]
    fn prehash_rebinding_and_unshared_storage_never_hit_stale_tables() {
        let source = Value::ints([128], (0..128).map(i64::from).collect())
            .unwrap().into_shared();
        let replaced = Value::ints([128], (1000..1128).map(i64::from).collect())
            .unwrap().into_shared();
        let mut cache = super::ExactPrehashCache::default();
        cache.get_or_prepare(&source, 128, 2, LookupResult::First)
            .unwrap().unwrap();
        let new_index = cache.get_or_prepare(&replaced, 128, 2, LookupResult::First)
            .unwrap().unwrap();
        assert_eq!(new_index.find(17, 128), 128);
        assert_eq!(new_index.find(1017, 128), 17);
        assert_eq!(cache.stats(), (2, 0));
        let unshared = Value::ints([128], (0..128).map(i64::from).collect()).unwrap();
        assert!(cache.get_or_prepare(&unshared, 128, 2, LookupResult::First)
            .unwrap().is_none());
        cache.clear();
        assert_eq!(cache.stats(), (0, 0));
    }

    #[test]
    fn all_exact_strategies_match_linear_reference_on_duplicate_heavy_inputs() {
        use super::{index_of, member};
        for &(items, nqueries) in &[(3, 10), (10, 10), (70, 3), (90, 80), (110, 25)] {
            let keys = (0..items).map(|i| (i % 11) as i64 - 5).collect::<Vec<_>>();
            let probes = (0..nqueries)
                .map(|i| (i % 17) as i64 - 7)
                .collect::<Vec<_>>();
            let source = Value::ints([items], keys.clone()).unwrap();
            let query = Value::ints([nqueries], probes.clone()).unwrap();
            for last in [false, true] {
                let found = index_of(source.clone(), query.clone(), last).unwrap();
                for (q, &key) in probes.iter().enumerate() {
                    let expected = if last {
                        keys.iter().rposition(|&v| v == key)
                    } else {
                        keys.iter().position(|&v| v == key)
                    }.unwrap_or(items) as i64;
                    assert_eq!(found.int_at(q).unwrap(), expected,
                        "items={items} probes={nqueries} q={q} last={last}");
                }
            }
            let member_result = member(query, source).unwrap();
            for (q, &key) in probes.iter().enumerate() {
                let expected = keys.contains(&key) as i64;
                assert_eq!(member_result.int_at(q).unwrap(), expected,
                    "membership items={items} probes={nqueries} q={q}");
            }
        }
    }
}
