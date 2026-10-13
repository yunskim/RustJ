//! Call-local comparison-policy snapshot for the implemented RustJ CPU subset.
//!
//! This is a semantic execution input, not an A3 hash/physical strategy.
//! Runtime equality now uses a pinned *default* J CCT snapshot (jsource
//! `TCMPEQ`); the historical Rust `near` rule remains only as an explicit
//! regression witness. A J-C-matching dynamic CCT state and immutable
//! snapshots exist for analysis but are NOT yet wired to J `9!:18`/`9!:19`,
//! `!.t`, comparison kernels or search execution. The pinned-default route
//! remains the only executable policy until call sites thread a snapshot.
//! The state version guards future runtime caches against stale comparison
//! results. It is never an optimizer numerical approximation budget.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum ComparisonPolicyIdentity {
    /// Existing CPU `kernels::near` contract; NOT a general J CCT witness.
    #[cfg(test)]
    FixedRustNearV0,
    /// Pinned jsource C `TCMPEQ`, with initial `cct = 1.0 - 2^-44`.
    PinnedJDefaultCctV0,
    /// Explicit runtime value and mutation generation; a snapshot is immutable.
    #[expect(
        dead_code,
        reason = "N1a: runtime snapshot producer is intentionally not wired"
    )]
    RuntimeJComplementaryCct { cct_bits: u64, version: u64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ComparisonPolicySnapshot {
    identity: ComparisonPolicyIdentity,
}

/// The exact jsrc/xa.c::jtcts upper-bound literal at the pinned revision.
/// Neither J tolerance nor this ceiling is an optimizer error allowance.
#[expect(dead_code, reason = "N1a: scalar CCT range only used by staged state")]
pub(crate) const JSOURCE_MAX_TOLERANCE: f64 = 5.820766091e-11;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(
    dead_code,
    reason = "N1a: state setter is not yet reached by interpreter"
)]
pub(crate) enum JComparisonStateError {
    Domain,
    VersionExhausted,
}

/// Engine-owned comparison state for a future foreign-verb implementation.
/// Values are stored as complementary tolerance exactly as jtcts does.
/// No process-global mutable state and no physical search cache is embedded.
#[derive(Clone, Copy, Debug)]
#[expect(dead_code, reason = "N1a: engine ownership comes in the next PR")]
pub(crate) struct JComparisonState {
    complementary_cct: f64,
    version: u64,
}

impl Default for JComparisonState {
    fn default() -> Self {
        Self {
            complementary_cct: 1.0 - 2f64.powi(-44),
            version: 0,
        }
    }
}

#[expect(dead_code, reason = "N1a: call-site CCT propagation is a follow-up")]
impl JComparisonState {
    /// Matches jtctq: return 1 - the stored complementary value. The result
    /// may be rounded, and is not necessarily the last requested input.
    pub(crate) fn tolerance(&self) -> f64 {
        1.0 - self.complementary_cct
    }

    pub(crate) fn snapshot(&self) -> ComparisonPolicySnapshot {
        ComparisonPolicySnapshot {
            identity: ComparisonPolicyIdentity::RuntimeJComplementaryCct {
                cct_bits: self.complementary_cct.to_bits(),
                version: self.version,
            },
        }
    }

    /// jtcts scalar numeric-conversion/rank checks belong to the future
    /// foreign primitive. This receives a prevalidated numeric scalar.
    /// Every successful setter gets a fresh generation, including no-op sets,
    /// so any future cached comparisons must be invalidated or re-guarded.
    pub(crate) fn set_scalar_tolerance(
        &mut self,
        tolerance: f64,
    ) -> Result<(), JComparisonStateError> {
        if !(0.0..=JSOURCE_MAX_TOLERANCE).contains(&tolerance) {
            return Err(JComparisonStateError::Domain);
        }
        let next_version = self
            .version
            .checked_add(1)
            .ok_or(JComparisonStateError::VersionExhausted)?;
        self.complementary_cct = 1.0 - tolerance;
        self.version = next_version;
        Ok(())
    }

    pub(crate) fn version(&self) -> u64 {
        self.version
    }
}

impl ComparisonPolicySnapshot {
    #[cfg(test)]
    pub(crate) const fn fixed_rust_near() -> Self {
        Self {
            identity: ComparisonPolicyIdentity::FixedRustNearV0,
        }
    }

    /// Current default J comparison policy, not dynamic CCT support.
    pub(crate) const fn pinned_j_default_cct() -> Self {
        Self {
            identity: ComparisonPolicyIdentity::PinnedJDefaultCctV0,
        }
    }

    /// Keep the existing floating predicate *bit-for-bit* on the supported
    /// path. In particular, do not replace it with a divide, min/max ratio,
    /// nontransitive hash representative, or jsource's masked-bucket probe.
    pub(crate) fn float_equal(self, a: f64, b: f64) -> bool {
        match self.identity {
            #[cfg(test)]
            ComparisonPolicyIdentity::FixedRustNearV0 => {
                a == b
                    || (a.is_finite()
                        && b.is_finite()
                        && (a - b).abs() <= 2f64.powi(-44) * a.abs().max(b.abs()))
            }
            ComparisonPolicyIdentity::PinnedJDefaultCctV0 => {
                let cct = 1.0 - 2f64.powi(-44);
                (a > cct * b) != (b <= cct * a)
            }
            ComparisonPolicyIdentity::RuntimeJComplementaryCct { cct_bits, .. } => {
                let cct = f64::from_bits(cct_bits);
                (a > cct * b) != (b <= cct * a)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ComparisonPolicySnapshot, JComparisonState, JComparisonStateError, JSOURCE_MAX_TOLERANCE,
    };

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
        let pinned = ComparisonPolicySnapshot::pinned_j_default_cct();
        assert_eq!(
            pinned.float_equal(a, on_lower_boundary),
            source_cct_macro_model(a, on_lower_boundary, cct)
        );
        assert_eq!(
            pinned.float_equal(a, on_upper_boundary),
            source_cct_macro_model(a, on_upper_boundary, cct)
        );

        for (x, y) in [
            (0.0, -0.0),
            (f64::INFINITY, f64::INFINITY),
            (f64::NEG_INFINITY, f64::NEG_INFINITY),
            (f64::NAN, f64::NAN),
        ] {
            assert_eq!(legacy.float_equal(x, y), source_cct_macro_model(x, y, cct));
            assert_eq!(pinned.float_equal(x, y), source_cct_macro_model(x, y, cct));
        }
    }

    #[test]
    fn pinned_default_cct_matches_source_model_for_tolerant_chain() {
        let t = 2f64.powi(-44);
        let cct = 1.0 - t;
        let policy = ComparisonPolicySnapshot::pinned_j_default_cct();
        let points = [
            1.0,
            1.0 + 0.75 * t,
            1.0 + 1.5 * t,
            -1.0,
            0.0,
            -0.0,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
            f64::from_bits(1),
        ];
        for &left in &points {
            for &right in &points {
                assert_eq!(
                    policy.float_equal(left, right),
                    source_cct_macro_model(left, right, cct),
                    "default CCT mismatch: {left:?} vs {right:?}"
                );
            }
        }
        // CCT is not a transitive equivalence relation; exact hash is illegal.
        assert!(policy.float_equal(points[0], points[1]));
        assert!(policy.float_equal(points[1], points[2]));
        assert!(!policy.float_equal(points[0], points[2]));
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

    #[test]
    fn j_cct_state_setter_matches_source_range_rounding_and_is_transactional() {
        let mut state = JComparisonState::default();
        let initial = state.tolerance();
        assert_eq!(initial, 2f64.powi(-44));
        assert_eq!(state.version(), 0);
        for invalid in [
            -1.0,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            JSOURCE_MAX_TOLERANCE * 2.0,
        ] {
            assert_eq!(
                state.set_scalar_tolerance(invalid),
                Err(JComparisonStateError::Domain)
            );
            assert_eq!(state.tolerance(), initial);
            assert_eq!(state.version(), 0);
        }
        state.set_scalar_tolerance(JSOURCE_MAX_TOLERANCE).unwrap();
        assert_eq!(state.tolerance(), 1.0 - (1.0 - JSOURCE_MAX_TOLERANCE));
        assert_eq!(state.version(), 1);
        state.set_scalar_tolerance(0.0).unwrap();
        assert_eq!(state.tolerance(), 0.0);
        state.set_scalar_tolerance(-0.0).unwrap();
        assert_eq!(state.version(), 3);
        assert_eq!(state.tolerance(), 0.0);

        state.version = u64::MAX;
        let before = state.snapshot();
        assert_eq!(
            state.set_scalar_tolerance(2f64.powi(-44)),
            Err(JComparisonStateError::VersionExhausted)
        );
        assert_eq!(state.snapshot(), before);
    }

    #[test]
    fn j_cct_snapshots_are_immutable_and_do_not_authorize_egraph_equivalence() {
        let mut state = JComparisonState::default();
        let default = state.snapshot();
        let t = 2f64.powi(-44);
        let (a, b) = (1.0, 1.0 + t * 0.75);
        assert!(default.float_equal(a, b));
        state.set_scalar_tolerance(0.0).unwrap();
        let strict = state.snapshot();
        assert_ne!(strict, default);
        assert!(default.float_equal(a, b));
        assert!(!strict.float_equal(a, b));
        assert!(strict.float_equal(0.0, -0.0)); // numeric, not bitwise identity
        assert!(strict.float_equal(f64::INFINITY, f64::INFINITY));
        assert!(!strict.float_equal(f64::NAN, f64::NAN));
        assert_eq!(strict.float_equal(a, b), source_cct_macro_model(a, b, 1.0));
        state.set_scalar_tolerance(t).unwrap();
        let restored = state.snapshot();
        assert_ne!(restored, default); // distinct runtime version
        assert_eq!(restored.float_equal(a, b), default.float_equal(a, b));
    }
}
