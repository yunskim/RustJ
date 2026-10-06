//! Research-only tolerant Index-Of candidate-completeness harness.
//!
//! This is NOT connected to A3 code generation, the interpreter, or the
//! Physical Planner. It tests a necessary condition for the present fixed
//! CPU near predicate, not full J fit, dynamic cct, box, sparse, or rank.
//!
//! With 0 <= t < 1/2, finite nonzero a approximately equals b only if
//! they have the same sign and floor(log2(abs(a))) differs from
//! floor(log2(abs(b))) by at most one. Equal signed zero and same-sign
//! infinity are special cases; NaN never compares equal. Thus sign/exponent
//! buckets can only FILTER candidate positions: the original predicate must
//! verify matches. Keep *all* source positions because tolerant equality
//! is not transitive and cannot be quotiented into ordinary hash classes.

use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum ExponentBucket {
    Zero,
    PositiveInfinity,
    NegativeInfinity,
    Finite { negative: bool, exponent: i16 },
}

fn exponent_bucket(value: f64) -> Option<ExponentBucket> {
    if value.is_nan() {
        return None;
    }
    if value == 0.0 {
        return Some(ExponentBucket::Zero);
    }
    if value.is_infinite() {
        return Some(if value.is_sign_negative() {
            ExponentBucket::NegativeInfinity
        } else {
            ExponentBucket::PositiveInfinity
        });
    }
    let bits = value.to_bits();
    let raw_exponent = ((bits >> 52) & 0x7ff) as i32;
    let exponent = if raw_exponent != 0 {
        raw_exponent - 1023
    } else {
        let fraction = bits & ((1_u64 << 52) - 1);
        debug_assert_ne!(fraction, 0);
        63 - (fraction.leading_zeros() as i32) - 1074
    };
    Some(ExponentBucket::Finite {
        negative: value.is_sign_negative(),
        exponent: exponent as i16,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MatchOrder {
    First,
    Last,
}

struct TolerantExponentCandidateIndex {
    original: Vec<f64>,
    buckets: HashMap<ExponentBucket, Vec<usize>>,
}

impl TolerantExponentCandidateIndex {
    fn new(original: &[f64]) -> Self {
        let mut buckets: HashMap<ExponentBucket, Vec<usize>> = HashMap::new();
        for (position, &value) in original.iter().enumerate() {
            if let Some(bucket) = exponent_bucket(value) {
                buckets.entry(bucket).or_default().push(position);
            }
        }
        Self {
            original: original.to_vec(),
            buckets,
        }
    }

    // A superset of matching positions, restored to original source order.
    // This deliberately does not merge approximate-equal representatives.
    fn candidate_positions(&self, query: f64) -> Vec<usize> {
        let mut result = Vec::new();
        match exponent_bucket(query) {
            Some(ExponentBucket::Finite { negative, exponent }) => {
                for delta in [-1, 0, 1] {
                    let bucket = ExponentBucket::Finite {
                        negative,
                        exponent: exponent + delta,
                    };
                    if let Some(positions) = self.buckets.get(&bucket) {
                        result.extend_from_slice(positions);
                    }
                }
            }
            Some(bucket) => {
                if let Some(positions) = self.buckets.get(&bucket) {
                    result.extend_from_slice(positions);
                }
            }
            None => {}
        }
        result.sort_unstable();
        result
    }

    fn find(&self, query: f64, order: MatchOrder) -> usize {
        let candidates = self.candidate_positions(query);
        let predicate = |position: &usize| {
            crate::kernels::near(self.original[*position], query)
        };
        match order {
            MatchOrder::First => candidates.iter().find(|pos| predicate(pos)).copied(),
            MatchOrder::Last => candidates.iter().rev().find(|pos| predicate(pos)).copied(),
        }
        .unwrap_or(self.original.len())
    }
}

#[cfg(test)]
mod tests {
    use super::{exponent_bucket, MatchOrder, TolerantExponentCandidateIndex};
    use crate::kernels::near;

    fn reference(values: &[f64], query: f64, mode: MatchOrder) -> usize {
        let found = match mode {
            MatchOrder::First => values.iter().position(|&x| near(x, query)),
            MatchOrder::Last => values.iter().rposition(|&x| near(x, query)),
        };
        found.unwrap_or(values.len())
    }

    #[test]
    fn tolerant_equality_is_not_an_equivalence_relation() {
        let t = 2_f64.powi(-44);
        let a = 1.0;
        let b = 1.0 + 0.75 * t;
        let c = 1.0 + 1.50 * t;
        assert!(near(a, b));
        assert!(near(b, c));
        assert!(!near(a, c));
        let index = TolerantExponentCandidateIndex::new(&[a, b, c]);
        assert_eq!(index.find(c, MatchOrder::First), 1);
        assert_eq!(index.find(c, MatchOrder::Last), 2);
        assert_eq!(index.find(a, MatchOrder::First), 0);
        assert_eq!(index.find(a, MatchOrder::Last), 1);
    }

    #[test]
    fn ordered_candidates_preserve_first_and_last_across_exponent_boundary() {
        let t = 2_f64.powi(-44);
        let below = 2.0 * (1.0 - t / 4.0);
        assert!(below < 2.0);
        assert!(near(below, 2.0));
        assert_ne!(exponent_bucket(below), exponent_bucket(2.0));
        let values = [below, 2.0, below, 3.0];
        let index = TolerantExponentCandidateIndex::new(&values);
        assert_eq!(index.find(2.0, MatchOrder::First), 0);
        assert_eq!(index.find(2.0, MatchOrder::Last), 2);
        assert_eq!(index.find(4.0, MatchOrder::First), values.len());
    }

    #[test]
    fn zero_infinity_nan_and_subnormal_buckets_do_not_lose_matches() {
        let values = [
            0.0, -0.0,
            f64::from_bits(1), -f64::from_bits(1),
            f64::from_bits((1_u64 << 52) - 1), f64::MIN_POSITIVE,
            f64::INFINITY, f64::NEG_INFINITY, f64::NAN,
            f64::MAX, -f64::MAX,
        ];
        let index = TolerantExponentCandidateIndex::new(&values);
        for &query in &values {
            let candidates = index.candidate_positions(query);
            for (position, &source) in values.iter().enumerate() {
                if near(source, query) {
                    assert!(candidates.contains(&position),
                        "lost match {position}: {query:?}");
                }
            }
            for mode in [MatchOrder::First, MatchOrder::Last] {
                assert_eq!(index.find(query, mode), reference(&values, query, mode));
            }
        }
        assert_eq!(index.find(f64::NAN, MatchOrder::First), values.len());
        assert_eq!(index.find(-0.0, MatchOrder::First), 0);
        assert_eq!(index.find(0.0, MatchOrder::Last), 1);
    }

    // Additional IEEE-754 word-space coverage. This is deliberately an
    // independent linear oracle, not a comparison against a second hash.
    // It exercises adjacent representable words on both sides of powers of
    // two, including subnormal/normal and maximum-finite boundaries.
    #[test]
    fn fixed_near_candidate_filter_covers_ieee_neighbor_words_and_first_last() {
        let mut values = vec![
            0.0, -0.0, f64::INFINITY, f64::NEG_INFINITY,
            f64::NAN, f64::MAX, -f64::MAX,
        ];
        let anchors = [
            1_u64, (1_u64 << 52) - 1, 1_u64 << 52,
            0x3fef_ffff_ffff_ffff, 0x3ff0_0000_0000_0000,
            0x3fff_ffff_ffff_ffff, 0x4000_0000_0000_0000,
            0x7fef_ffff_ffff_ffff,
        ];
        for anchor in anchors {
            for offset in [-512_i64, -256, -1, 0, 1, 256, 512] {
                if let Some(bits) = anchor.checked_add_signed(offset) {
                    for sign in [0_u64, 1_u64 << 63] {
                        values.push(f64::from_bits(bits | sign));
                    }
                }
            }
        }

        // Fixed PRNG seed; no external crate, nondeterminism or platform
        // dependence. Include the bit patterns exactly (including NaNs).
        let mut state = 0x1a2b_3c4d_5e6f_7890_u64;
        for _ in 0..2_048 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            values.push(f64::from_bits(state));
        }

        let index = TolerantExponentCandidateIndex::new(&values);
        let mut queries = values.iter().take(120).copied().collect::<Vec<_>>();
        queries.extend(values.iter().skip(120).step_by(17).copied());
        for query in queries {
            let candidates = index.candidate_positions(query);
            // The reference enumerates *every* original source position
            // without looking at buckets. Missing positions use len().
            for (position, &source) in values.iter().enumerate() {
                if near(source, query) {
                    assert!(
                        candidates.binary_search(&position).is_ok(),
                        "lost near source at {position}: {source:?} vs {query:?}"
                    );
                }
            }
            assert!(
                candidates.windows(2).all(|w| w[0] < w[1]),
                "candidate positions must be unique and in source order"
            );
            for order in [MatchOrder::First, MatchOrder::Last] {
                assert_eq!(
                    index.find(query, order),
                    reference(&values, query, order),
                    "wrong ordered representative for query {query:?}"
                );
            }
        }
    }

    #[test]
    fn independent_linear_reference_agrees_on_adversarial_float_grid() {
        let t = 2_f64.powi(-44);
        let mut values = vec![0.0, -0.0, f64::INFINITY, f64::NEG_INFINITY, f64::NAN];
        for exponent in [-1074, -1023, -1022, -64, -1, 0, 1, 64, 1023] {
            let base = if exponent == -1074 {
                f64::from_bits(1)
            } else {
                2_f64.powi(exponent)
            };
            for sign in [-1.0, 1.0] {
                for perturb in [-1.5, -0.75, 0.0, 0.75, 1.5] {
                    let value = sign * base * (1.0 + perturb * t);
                    if value.is_finite() {
                        values.push(value);
                    }
                }
            }
        }
        let index = TolerantExponentCandidateIndex::new(&values);
        let mut queries = values.clone();
        queries.extend([1.0 + 0.5 * t, 2.0, f64::MIN_POSITIVE, f64::MAX, -f64::MAX]);
        for &query in &queries {
            let candidates = index.candidate_positions(query);
            for (position, &source) in values.iter().enumerate() {
                if near(source, query) {
                    assert!(candidates.binary_search(&position).is_ok(),
                        "missing near pair {position}, {source:?}, {query:?}");
                }
            }
            for mode in [MatchOrder::First, MatchOrder::Last] {
                assert_eq!(index.find(query, mode), reference(&values, query, mode),
                    "mismatched tolerant representative at {query:?}");
            }
        }
    }
}
