//! Call-local comparison-policy snapshot for the implemented RustJ CPU subset.
//!
//! This is a semantic execution input, not an A3 hash/physical strategy.
//! The interpreter currently exposes only the legacy fixed `near` policy:
//! no J `!.t` fit or `9!:19` global tolerance change is implemented.
//! Keeping an explicit snapshot prevents callers from mistaking a float
//! dtype or an A3 SearchDescriptor for permission to reuse tolerant tables.
//! A future dynamic policy must carry a runtime version and be guarded before
//! introducing optimized tolerant lookup.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum ComparisonPolicyIdentity {
    /// Existing CPU `kernels::near` contract; NOT a general J CCT witness.
    FixedRustNearV0,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ComparisonPolicySnapshot {
    identity: ComparisonPolicyIdentity,
}

impl ComparisonPolicySnapshot {
    pub(crate) const fn fixed_rust_near() -> Self {
        Self {
            identity: ComparisonPolicyIdentity::FixedRustNearV0,
        }
    }

    /// Keep the existing floating predicate *bit-for-bit* on the supported
    /// path. In particular, do not replace it with a divide, min/max ratio,
    /// nontransitive hash representative, or jsource's masked-bucket probe.
    pub(crate) fn float_equal(self, a: f64, b: f64) -> bool {
        match self.identity {
            ComparisonPolicyIdentity::FixedRustNearV0 => {
                a == b
                    || (a.is_finite()
                        && b.is_finite()
                        && (a - b).abs() <= 2f64.powi(-44) * a.abs().max(b.abs()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ComparisonPolicySnapshot;

    #[test]
    fn fixed_policy_matches_existing_special_values_and_nontransitive_chain() {
        let policy = ComparisonPolicySnapshot::fixed_rust_near();
        let t = 2f64.powi(-44);
        let (a, b, c) = (1.0, 1.0 + 0.75 * t, 1.0 + 1.5 * t);
        assert!(policy.float_equal(a, b));
        assert!(policy.float_equal(b, c));
        assert!(!policy.float_equal(a, c));
        assert!(policy.float_equal(-0.0, 0.0));
        assert!(policy.float_equal(f64::INFINITY, f64::INFINITY));
        assert!(policy.float_equal(f64::NEG_INFINITY, f64::NEG_INFINITY));
        assert!(!policy.float_equal(f64::INFINITY, f64::NEG_INFINITY));
        assert!(!policy.float_equal(f64::NAN, f64::NAN));
        assert!(!policy.float_equal(f64::from_bits(1), 0.0));
    }
}
