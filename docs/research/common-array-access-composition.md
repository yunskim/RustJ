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

| graph_monadic_map_and_transpose_share_region_pullback | Validate shared region pullback from original unary Map→Transpose Graph | Preserve input ValueIds, independent operation witnesses, empty demand and composed/sequential agreement | Incorrect cross-family coordinate law or mismatched input occurrence | PR #75 merged · main `0c95dbf` CI PASS ([main CI 1](https://github.com/yunskim/RustJ/actions/runs/38321242397), [main CI 2](https://github.com/yunskim/RustJ/actions/runs/38321242471)) |
| graph_unary_map_access_rejects_unsupported_calls_and_unknown_facts | Reject unsupported operations and contradictory contracts/facts | No invented access for Dyad, Rank, Reverse, Ravel, Gather, Reduce, NAME result, missing/contradictory facts | Unsafe admission to the common access contract | PR #75 merged · main `0c95dbf` CI PASS ([main CI 1](https://github.com/yunskim/RustJ/actions/runs/38321242397), [main CI 2](https://github.com/yunskim/RustJ/actions/runs/38321242471)) |

| graph_value_demand_closure_across_witnessed_chain | Trace unary Map→Transpose→Map logical value-demand from original J Graph | Preserve every ValueId and exact output-to-source regions, empty demand, scalar, NAME source | Skipped graph edge, wrong map or lost source identity | PR #78 merged · main `e473168` CI PASS ([PR #38322201413](https://github.com/yunskim/RustJ/actions/runs/38322201413), [main #38322446856](https://github.com/yunskim/RustJ/actions/runs/38322446856), [main #38322446869](https://github.com/yunskim/RustJ/actions/runs/38322446869)) |
| graph_value_demand_stops_at_unproved_operation_and_invalid_box | Protect unproved internal operation and invalid box boundaries | Stop before reverse/dyad/reduction/rank, reject invalid rank/bounds or unknown result shape | Fabricated smaller input-demand through unsupported operation | PR #78 merged · main `e473168` CI PASS ([PR #38322201413](https://github.com/yunskim/RustJ/actions/runs/38322201413), [main #38322446856](https://github.com/yunskim/RustJ/actions/runs/38322446856), [main #38322446869](https://github.com/yunskim/RustJ/actions/runs/38322446869)) |

| graph_fanout_demands_remain_conservative_when_boxes_are_disjoint | Exact union of requests from two transpose consumers of a real fork | Keep each separately witnessed rectangular demand, never mislabel bounding hull as exact | Dropped fanout region or unsound exact-hull claim | PR #81 merged · main `9def7b8` CI PASS ([main #38323556118](https://github.com/yunskim/RustJ/actions/runs/38323556118), [main #38323556145](https://github.com/yunskim/RustJ/actions/runs/38323556145)) |
| graph_fanout_demand_union_rejects_unproved_join_and_invalid_requests | Reject unsupported join, unrelated target and malformed requests | Every demand must prove a path to the same original ValueId; reject join/bad bounds/empty input | Fabricated access across join or invalid set aggregation | PR #81 merged · main `9def7b8` CI PASS ([main #38323556118](https://github.com/yunskim/RustJ/actions/runs/38323556118), [main #38323556145](https://github.com/yunskim/RustJ/actions/runs/38323556145)) |

## Separate evidence still required

- R0.2 projects direct monadic `|:` J Graph nodes into coordinate access; other StaticReindex operations, ranked/dyadic transpose and full J execution equivalence still need independent witnesses.
- Sound backward region demands for memory/I/O.
- Pinned J-C oracle checks for fill/prototype, Rank and error/effect equivalence.
- R0.3 tests shared logical-region pullback for Map and Transpose; sound physical I/O savings and full J observational equality still require independent evidence.
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

## R0.3 Shared coordinate/box pullback: unary Map × Transpose (2026-10-13)

`Plan::static_value_access(ValueId)` exposes the **logical value-coordinate
projection only** for two independently recognized families from the original
J Graph:

- **Direct unary Map**: primitive `FunctionHead`, atomic monad,
  `Elementwise/CorrespondingElements` graph contract, primitive
  `Map/Pure/PreserveRight` classification, and equal known input/output
  Shape/Rank. Its axis relation is identity.
- **Direct monadic `|:`**: reuse the R0.2 witnessed reverse-axis projection.

Both families use the *same* `StaticAxisAccess::pullback_region`. For a
Map→Transpose graph, one composed access must equal successive region
pullback. This is a **cross-family coordinate value-demand law**, not proof
of equivalence, reordering, or permission to execute fewer calls.

Even a pure monadic Map may have J-visible errors in unrequested elements.
Prototype/fill, Rank/Cell/Frame, NAME/effect and first-error precedence
are separate obligations; this interface does not permit partial execution,
physical I/O pruning, fusion or EqGraph union. Dyads, ranked operations,
Gather, dynamic names and unknown shapes require independent evidence.

## R0.4 Conservative backward value demand across original Graph ValueIds (2026-10-13)

`Plan::value_demand_trace(output, box)` first verifies the full Plan and
checks the requested logical rectangular box against its result shape. It
follows only R0.3's separately witnessed direct unary Map and Transpose edges
backwards along **original ValueIds**, recording ordered
`GraphDemandAtValue { value, region }` steps.

- Input/literal/versioned NAME `ReadNoun` end at `Source`.
- Any other Apply (including Rank/Reduce/Reverse) ends at
  `UnprovedOperation` **on that operation's output**; input demand below
  the barrier is **unknown**, never inferred from the last recorded box.
- Invalid box/rank/unknown output shape fail closed to `None`.

This is only a **logical per-ValueId coordinate-demand trace**. Original
nodes, NAME ordering, error effects and all computation obligations remain
untouched; empty demand does not authorize skipping any fill/prototype work.
Fanout/branch closure, physical byte/I/O demands, costs, Exact-J observational
equality and EqGraph union require independent evidence.

## R0.5 Exact union of logical fanout value-demands (2026-10-13)

`Plan::witnessed_value_demand_union(target, requests)` validates each original
output ValueId and rectangular request through the R0.4 backward witness chain
**independently**, and accepts only requests proven to reach the same original
target ValueId. `GraphValueDemandUnion` preserves an exact `Vec<Rectangle>`
union. Overlapping or disjoint boxes are retained; no enclosing bounding
rectangle is silently claimed exact.

Test with the two independent transpose consumers in a concrete
`(|: + |:) a` fork. Its dyadic join is an unproved access barrier; unrelated
target, malformed rank/box and empty requests fail closed.

This is only an **exact set of logical per-value coordinate demands**.
It does not justify partially executing the enclosing fork/join, removing
observable NAME, errors, prototype/fill or effects, physical byte/IO pruning,
or EqGraph equality.
