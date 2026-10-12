# R0 common array access algebra: static axis-permutation composition

Status: analysis-only implementation candidate. This is **not** an Exact-J equivalence proof, Graph rewrite, EqGraph union, or executable optimization.
Authority: PROJECT.md common array algebra research hypothesis and SPORES relational access R0.

## One semantic unit

Define StaticAxisAccess(S,p) with logical input shape S and a permutation p from
output axes to input axes. For each axis j, output_shape[j] = S[p[j]], and an
output coordinate o demands input coordinate i[p[j]] = o[j].

For transformation p followed by q, the intermediate output shape must equal
q's input shape exactly. The composed axis relation is r[j] = p[q[j]].
Both p and q must be bijective permutations, including valid rank-zero and
zero-extent logical shapes. This has no physical layout, stride or buffer meaning.

This establishes equality of **coordinate access projections**, not of complete
J operations. Rank/Cell/Frame application, empty-frame prototype/fill, NAME,
effects and first observable error ordering remain independent obligations.
Missing permutation/shape evidence remains Unknown. Dynamic Gather and general
Reshape are deliberately out of scope.

## Test purpose and guarantees

| Test | Purpose | Guarantee | Failure meaning | Status |
|---|---|---|---|---|
| axis_permutation_composition_matches_two_stage_pullback | Check two structural permutations | Exhaustive small-coordinate pullback equals two-step reference | Composition direction or index mapping is wrong | PR #70 head CI PASS (2026-10-13) |
| axis_access_rejects_unproved_shape_and_index_relations | Reject absent or contradictory facts | Reject invalid rank/bijection/shape/coordinate | Invalid access relation was admitted | PR #70 head CI PASS (2026-10-13) |
| axis_region_pullback_matches_composition_and_exact_requested_points | Pull back an output box through static permutations | Composed/sequential regions and exhaustive point membership agree; reject bad bounds | Unsound region pruning or index-axis mapping | PR #71 CI pending |
| axis_composition_preserves_scalar_and_zero_extent_shapes | Protect scalar and zero-extent boundaries | Preserve rank zero, zero extents, nonexistent atom access | Empty shape semantics or bounds lost | PR #70 head CI PASS (2026-10-13) |

## Separate evidence still required

- Verify that a concrete VerifiedFrontend/J Graph StaticReindex node really satisfies this access projection.
- Sound backward region demands for memory/I/O.
- Pinned J-C oracle checks for fill/prototype, Rank and error/effect equivalence.
- Reuse the same transfer law across at least two different operation families.
- Only then consider GF4/FW-05–09 witnessed rewrite or EqGraph admission.

**Proved scope at this stage: coordinates and shape only.** No verified J equivalence or performance gain.

## R0.1 Logical box demand pullback (2026-10-13)

For half-open ranges R[j] on the output axes of a bijective static permutation
p, the exact requested input range on axis p[j] is R[j]. Preserve axis order
and interval extents; verify the rank, interval ordering and output extents.
Empty ranges are valid and remain empty. Pulling a box back through a composed
permutation must equal successive pullback through the two separate maps.

This is a **logical value-region** result only. An empty value demand cannot
erase prototype checks, observable errors or effects. It grants no physical
storage chunks, byte offsets or execution schedule. GF5a/IO-09·10 still require
Graph access witnesses and J semantic validation.
