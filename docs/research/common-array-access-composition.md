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
| axis_region_pullback_matches_composition_and_exact_requested_points | Pull back an output box through static permutations | Composed/sequential regions and exhaustive point membership agree; reject bad bounds | Unsound region pruning or index-axis mapping | PR #71 and merged-main CI PASS (2026-10-13) |
| axis_composition_preserves_scalar_and_zero_extent_shapes | Protect scalar and zero-extent boundaries | Preserve rank zero, zero extents, nonexistent atom access | Empty shape semantics or bounds lost | PR #70 head CI PASS (2026-10-13) |
| graph_monadic_transpose_projects_witnessed_axis_access | Project monadic `|:` from an actual J Graph | Preserve ValueIds and exact shapes/ranks; check double-transpose composition and region demand | Incorrect graph classification or axis orientation | PR #72 code commit `77badcd` CI PASS ([PR CI](https://github.com/yunskim/RustJ/actions/runs/38319113130)) |
| graph_transpose_access_refuses_unknown_and_unrelated_reindex_facts | Reject unrelated reindex and incomplete facts | Reverse/ravel/name/unknown or contradictory shape/rank cannot assert transpose access | Inferring a rewrite permission from an unproven relation | PR #72 code commit `77badcd` CI PASS ([PR CI](https://github.com/yunskim/RustJ/actions/runs/38319113130)) |
| graph_monadic_map_and_transpose_share_region_pullback | Reuse one Region pullback method for unary Map and Transpose | Real J Graph `|: - a` uses identity Map and reversed-axis Transpose; sequential and composed boxes agree | Cross-family composition direction or demand mapping broken | R0.3 CI pending |
| graph_unary_map_projection_rejects_non_pointwise_or_incomplete_facts | Fail closed outside exact unary Map scope | Dyadic Map, reindex, noun, unknown/mismatched facts do not earn a unary Map access witness | Invented pointwise identity from unsupported J operations | R0.3 CI pending |

## Separate evidence still required

- R0.2 projects direct monadic `|:` J Graph nodes into coordinate access; other StaticReindex operations, ranked/dyadic transpose and full J execution equivalence still need independent witnesses.
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

## R0.2 J Graph monadic transpose projection (2026-10-13)

`Plan::static_transpose_access(ValueId)` only recognizes an original atomic
monadic `PrimitiveVerb(Cant)` Apply whose Graph Basis is `StaticReindex`,
Access class is `IndexTransform`, whose input/output shape and rank facts
are known, and whose output shape exactly reverses the input shape.
It returns `GraphTransposeAccess { input, output, access }` with original
ValueIds; failed `Plan::verify()` or missing/conflicting facts return `None`.

This is a **value-coordinate analysis view**, not an Exact-J rewrite, NAME
snapshot proof, elimination of checks/effects/errors, physical byte-region
request, or EqGraph e-class union. Do not project `|.`, `,`, dynamic shapes,
rank-applied transpose, or dyadic `|:` without separate contracts.

## R0.3 Two-family shared region transfer experiment (2026-10-13)

`Plan::static_unary_map_access(ValueId)` projects only direct monadic,
pure pointwise primitives whose `OperationClass::Map`,
`ShapeRule::PreserveRight`, `CorrespondingElements`, and both
Shape/Rank facts support an identity logical axis access relation.
The existing monadic Transpose projection reuses the same
`StaticAxisAccess::pullback_region` implementation with reversed axes.

Test the real J Graph `|: - a` for shared region transfer and correct
sequential/composed demand. This is one cross-family implementation
example, **not** a universal algebra completeness result or a license to
rewrite J errors/effects. Ranked ops, dyadic broadcasting, mutable NAME
and dynamic Gather remain separate and unproven.
