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

    // Independent model of pinned jsource jsrc/vcomp.h::TCMPEQ.
    // This is a source-derived numerical oracle, NOT an executed J C binary.
    fn source_cct_macro_model(a: f64, b: f64, cct: f64) -> bool {
        (a > cct * b) != (b <= cct * a)
    }

    #[test]
    fn jsource_cct_macro_model_exposes_fixed_near_boundary_gap() {
        let t = 2f64.powi(-44);
        let cct = 1.0 - t; // jsrc/i.c initializes cct = 1.0 - FUZZ.
        let legacy = ComparisonPolicySnapshot::fixed_rust_near();
        let a = 1.0;
        let on_lower_boundary = a - t;
        let on_upper_boundary = a + t;

        assert!(legacy.float_equal(a, on_lower_boundary));
        assert!(legacy.float_equal(a, on_upper_boundary));
        assert!(!source_cct_macro_model(a, on_lower_boundary, cct));
        assert!(!source_cct_macro_model(a, on_upper_boundary, cct));

        for (x, y) in [
            (0.0, -0.0),
            (f64::INFINITY, f64::INFINITY),
            (f64::NEG_INFINITY, f64::NEG_INFINITY),
            (f64::NAN, f64::NAN),
        ] {
            assert_eq!(legacy.float_equal(x, y), source_cct_macro_model(x, y, cct));
        }
    }

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
