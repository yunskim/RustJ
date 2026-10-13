//! Numerical proof and error-budget *policy* for candidate optimizations.
//!
//! J language comparison tolerance is a semantic input, not an optimization
//! error allowance. Mathematical coordinate/algebra identities are also not
//! proof that a floating evaluation preserves J-observable results.
//!
//! This module is an analysis-only vocabulary and conservative numeric
//! precheck. It is NOT wired to an optimizer or an EqGraph, and passing the
//! numeric precheck is not authorization for an e-class union or J rewrite.

/// A claim's semantic scope, independent of approximate numeric closeness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericEqualityClaim {
    /// Axis, coordinate or logical value-demand relations only.
    CoordinateIdentity,
    /// Equal values over ideal mathematical reals, not binary64 evaluation.
    RealAlgebraicIdentity,
    /// The claimant asserts the full J-visible behavior was proven equal.
    /// This variant itself does not contain or validate such a proof.
    ExactJObservational,
}

/// Optimization-specific error budget; never inferred from J comparison CCT.
///
/// Exact is the default. A bounded budget is an explicit *separate* choice
/// for approximate candidates and cannot qualify an EqGraph equality union.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum OptimizationErrorBudget {
    #[default]
    Exact,
    /// Magnitudes describe only a proposed budget; no compositional error
    /// theorem, error metric/norm, or executable approval is implicit.
    Bounded {
        max_absolute: f64,
        max_relative: f64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericBudgetError {
    NonFinite,
    Negative,
    ZeroRequiresExact,
}

impl OptimizationErrorBudget {
    /// Construct only an *explicit* finite, nonnegative optimization budget.
    /// Neither parameter is the J 9!:18/9!:19 comparison tolerance.
    pub fn bounded(
        max_absolute: f64,
        max_relative: f64,
    ) -> Result<Self, NumericBudgetError> {
        if !max_absolute.is_finite() || !max_relative.is_finite() {
            return Err(NumericBudgetError::NonFinite);
        }
        if max_absolute < 0.0 || max_relative < 0.0 {
            return Err(NumericBudgetError::Negative);
        }
        if max_absolute == 0.0 && max_relative == 0.0 {
            return Err(NumericBudgetError::ZeroRequiresExact);
        }
        Ok(Self::Bounded {
            max_absolute,
            max_relative,
        })
    }

    pub const fn is_exact(self) -> bool {
        matches!(self, Self::Exact)
    }
}

/// This is a necessary *numeric* check, never a sufficient J equivalence
/// witness. Other mandatory proofs include evaluation order, error precedence,
/// NAME/effects, Rank/Cell/Frame and empty prototype/fill behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EqGraphNumericPrecheck {
    Rejected,
    RequiresOtherJProofs,
}

/// Block approximate or merely ideal-mathematical relations as e-class unions.
///
/// In particular, J tolerant equality is not an equivalence relation
/// (transitivity fails), and a bounded optimization error cannot repair that.
pub fn eqgraph_numeric_precheck(
    claim: NumericEqualityClaim,
    budget: OptimizationErrorBudget,
) -> EqGraphNumericPrecheck {
    if claim == NumericEqualityClaim::ExactJObservational && budget.is_exact() {
        EqGraphNumericPrecheck::RequiresOtherJProofs
    } else {
        EqGraphNumericPrecheck::Rejected
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EqGraphNumericPrecheck, NumericBudgetError, NumericEqualityClaim,
        OptimizationErrorBudget, eqgraph_numeric_precheck,
    };
    use crate::comparison_policy::ComparisonPolicySnapshot;

    #[test]
    fn optimization_error_budget_defaults_to_exact_and_rejects_invalid_proposals() {
        assert_eq!(
            OptimizationErrorBudget::default(),
            OptimizationErrorBudget::Exact
        );
        for (a, r, problem) in [
            (f64::NAN, 0.0, NumericBudgetError::NonFinite),
            (0.0, f64::INFINITY, NumericBudgetError::NonFinite),
            (-1.0, 0.0, NumericBudgetError::Negative),
            (0.0, -0.1, NumericBudgetError::Negative),
            (0.0, 0.0, NumericBudgetError::ZeroRequiresExact),
        ] {
            assert_eq!(OptimizationErrorBudget::bounded(a, r), Err(problem));
        }
        let explicit = OptimizationErrorBudget::bounded(0.0, 2f64.powi(-44)).unwrap();
        assert!(!explicit.is_exact());
        assert!(OptimizationErrorBudget::Exact.is_exact());
    }

    #[test]
    fn j_tolerant_comparison_and_real_algebra_do_not_admit_eqgraph_union() {
        let j_default = ComparisonPolicySnapshot::pinned_j_default_cct();
        let tolerance = 2f64.powi(-44);
        let (a, b, c) = (1.0, 1.0 + 0.75 * tolerance, 1.0 + 1.5 * tolerance);
        assert!(j_default.float_equal(a, b));
        assert!(j_default.float_equal(b, c));
        assert!(!j_default.float_equal(a, c));

        let exact = OptimizationErrorBudget::Exact;
        for claim in [
            NumericEqualityClaim::CoordinateIdentity,
            NumericEqualityClaim::RealAlgebraicIdentity,
        ] {
            assert_eq!(
                eqgraph_numeric_precheck(claim, exact),
                EqGraphNumericPrecheck::Rejected
            );
        }
        let approximate_budget = OptimizationErrorBudget::bounded(0.0, tolerance).unwrap();
        assert_eq!(
            eqgraph_numeric_precheck(
                NumericEqualityClaim::ExactJObservational,
                approximate_budget
            ),
            EqGraphNumericPrecheck::Rejected
        );
        assert_eq!(
            eqgraph_numeric_precheck(NumericEqualityClaim::ExactJObservational, exact),
            EqGraphNumericPrecheck::RequiresOtherJProofs
        );
    }
}
