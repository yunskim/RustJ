# RustJ common array algebra: J comparison tolerance vs optimization error budgets

Status: **policy and analysis-only numeric precheck implemented**; dynamic J CCT / Fit execution, approximate optimizations, actual EqGraph admission and full J observational equality remain **unimplemented / unverified**.

## Three independent layers

| Layer | Semantics | Ownership | Required claim |
|---|---|---|---|
| **J language comparison** | Default tolerance and live `!.t`, `9!:18`/`9!:19` where supported, primitive-specific J outputs | Semantic/runtime policy snapshot with version/guards | Match actual J-visible comparison results |
| **Mathematical common-array algebra** | Shape, axes, access/demand relations and algebraic identities over explicitly stated domains | Graph proof and conditions | Mathematical proof **does not prove binary64 evaluation identity** |
| **Optimization numerical error budget** | Explicit metric, absolute/relative/ULP/norm and accumulation proof; no implicit allowance | Candidate / numeric legality pass | Budget satisfaction alone never licenses J rewrite or EqGraph union |

**J comparison tolerance is not the compiler's numerical approximation budget.** Default optimizer mode remains exact.

## J baseline and source-derived boundary

J Dictionary specifies default `t = 2^-44 ≈ 5.684341886080802e-14` and, for finite reals, the mathematical tolerance test:

```text
abs(a-b) <= t * max(abs(a), abs(b))
```

However, the J C path `jsrc/vcomp.h::TCMPEQ` uses a floating arithmetic comparison macro and a `cct = 1 - 2^-44` representation. The documented real-valued relation cannot simply replace the source-accurate binary64 branch at the boundary. RustJ already has a pinned-default CCT snapshot and legacy-near boundary differential tests in `src/comparison_policy.rs`.

`9!:18 ''` reads and `9!:19` changes the comparison tolerance; `=!.0` and general `!.t` are verb-specific Fit variants. **RustJ does not yet implement dynamic CCT or Fit**, so current default behavior must not be described as complete J comparison semantics. Zero-tolerance J comparison is not the same as bitwise floating identity, including signed zeros. Verify `-:` Match and search, boxed/sparse/rank, NaN/Inf and first-error semantics separately.

The `jsource/test/g421e.ijs` tolerance adjustment for reordered `+/` summation demonstrates a **test comparison policy**, not a license for arbitrary reassociation under that tolerance.

## EqGraph and rewrite guard

`src/numeric_error_policy.rs` introduces `NumericEqualityClaim`, `OptimizationErrorBudget` and a **numeric-only** `eqgraph_numeric_precheck`.

- `OptimizationErrorBudget::Exact` is the default; bounded budgets require explicit finite nonnegative bounds and are not derived from J CCT.
- Coordinate access proofs and identities over mathematical reals **fail** the E-Class numeric precheck.
- A bounded numeric budget always fails an **equality** union precheck even when equal to J default comparison tolerance.
- Only a claimed `ExactJObservational` proof with exact budget passes the numeric *precheck*, yielding `RequiresOtherJProofs`: this is **not** authorization to union or rewrite. Verification of that proof and Rank/Cell/Frame, NAME/effects, first-error, fill/prototype, dtype and runtime CCT/Fit remains mandatory.
- Approximate candidates must remain separate from exact EqGraph equivalence classes and use explicit bounds and compositional error analysis, if later introduced.

## Test / Purpose / Guarantee / Failure meaning / Status

| Test or task | Purpose | Guarantee | Failure meaning | Status |
|---|---|---|---|---|
| `optimization_error_budget_defaults_to_exact_and_rejects_invalid_proposals` | Default/constructor contract | Exact by default; nonfinite/negative/zero-bounded rejected | Accidental J tolerance → optimizer budget promotion | **PR #91 merged · main `aeee1ed` CI PASS** ([main CI 1](https://github.com/yunskim/RustJ/actions/runs/38326014035), [main CI 2](https://github.com/yunskim/RustJ/actions/runs/38326013963)) |
| `j_tolerant_comparison_and_real_algebra_do_not_admit_eqgraph_union` | Separate nontransitive tolerance from EqGraph equality | A≈B and B≈C do not prove A≈C; mathematical/proposed bounded similarity never merges | Unsound e-class equivalence closure | **PR #91 merged · main `aeee1ed` CI PASS** ([main CI 1](https://github.com/yunskim/RustJ/actions/runs/38326014035), [main CI 2](https://github.com/yunskim/RustJ/actions/runs/38326013963)) |
| `9!:18` / `9!:19` | Snapshot, version, range and mutation semantics | Match actual J runtime policy incl. cache invalidation | Treating fixed default as full dynamic semantics | **UNIMPLEMENTED / UNVERIFIED** |
| `=!.0`, general `!.t`, `-:` and dependent primitives | Pinned jsource C oracle, rank/boxed/sparse/edge-case validation | Retain comparisons, errors, signed zeros, NaN/Inf, exact boundaries | Replacing source semantics by idealized equation | **PARTIAL / oracle INCOMPLETE** |
| Associative/reduction numerical proofs | Model abs/rel/ULP bounds, domains, error accumulation and J observations | Domain-dependent numeric guarantee independent of J tolerant comparison | Treating `2^-44` as universal reassociation allowance | **UNIMPLEMENTED** |
| egg/EqGraph gate integration | Verify actual full Exact-J equivalence before union | Never turn tolerant nontransitive comparison into a congruence | False E-Class equivalence | **UNCONNECTED / UNVERIFIED** |

## Sources

- [J Dictionary: Equal](https://www.jsoftware.com/help/dictionary/d000.htm)
- [J Dictionary: Global Parameters](https://jsoftware.com/help/dictionary/dx009.htm)
- [J Dictionary: Fit](https://jsoftware.com/help/dictionary/d411.htm)
- [Pinned jsource `jsrc/vcomp.h`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vcomp.h), `jsrc/xa.c` for CCT and `test/g421e.ijs` for a summation-order validation example
- RustJ: `src/comparison_policy.rs`, `src/kernels.rs`, `src/index_ops.rs`, `tools/tolerance_audit.py`.

**Non-admission:** passing policy unit tests or this precheck is not proof of dynamic Fit, source-level J equivalence, validated approximate optimization, runtime pruning, or EqGraph merging.

## Follow-on issues

- [#92: Dynamic J CCT and Fit snapshot correctness](https://github.com/yunskim/RustJ/issues/92) — **unimplemented**.
- [#93: Numerical error propagation and actual EqGraph admission](https://github.com/yunskim/RustJ/issues/93) — **unimplemented**.
