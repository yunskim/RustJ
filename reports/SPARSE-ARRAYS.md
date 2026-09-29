# Sparse storage milestone — 2026-09-29

`SparseArray` is a storage API, not a Scalar variant. It is now also carried by
`Data::Sparse(Arc<SparseArray>)`; a guarded subset of J `$.` is implemented.

The pinned C source `jsrc/jtype.h:1070` defines P's four components:
axes (a), scalar fill (e), coordinate matrix (i), and corresponding dense
value cells (x). Rust stores the same information without relative pointers or
C allocation headers. Coordinates and axes use immutable Arc slices, while
Value buffers are explicitly frozen for sharing.

Construction checks sorted unique valid sparse axes, coordinate row width,
lexicographically sorted unique rows, index bounds, scalar fill, matching element
types, and values shape `[stored_rows] + non_sparse_dimensions`. An empty sparse
axis set is supported. The explicit constructor permits redundant fill cells. `from_dense` omits
fill-only cells while retaining each non-fill dense block. Current supported payloads are Bool, Int, Float and byte Char.
Complex needs its dense representation first. Boxed sparse payloads are rejected;
C's SPARSABLE set likewise excludes BOX/XNUM/RAT in the inspected revision.

Logical shape need not fit a dense allocation. `to_dense(max_atoms)` checks size
and the caller's explicit atom limit before allocating; there is no implicit
densification. The bound counts atoms, not bytes. Materialization handles arbitrary
sparse axes and dense value cells in row-major order. Nonzero fill, signed zero,
empty dimensions, all-fill arrays and scalar shape are tested. An empty dimension
now makes the common shape product zero even when preceding dimensions overflow.

## Remaining sequence

- [x] Validated axes/fill/coordinates/value-cell storage.
- [x] Shared immutable buffers and explicit bounded dense materialization.
- [x] Runtime array representation and distinct element-type/layout facts.
- [ ] J `$.` construction, component queries and error behavior.
- [x] Dense-to-sparse conversion, omitting fill-only cells without dense-sized scratch.
- [ ] Existing sparse-array compaction and axis changes.
- [ ] Elementwise scalar maps that transform both stored values and fill.
- [ ] Sparse/sparse merges, duplicate policy during construction, reductions,
      indexing and shape verbs, with operation-specific density decisions.
- [ ] Native Windows pinned C oracle comparison, including nonzero fill and
      partially sparse axes. No C equivalence claim yet.
- [ ] Storage/performance comparison against dense arrays; CUDA layouts deferred.

Windows MSVC validation: default and portable suites each pass 67 tests plus one
doctest (six sparse-specific tests). Clippy with denied warnings passes.
GitHub CI is skipped. No Linux verification was run for this milestone.

Dense conversion now round-trips every sparse-axis subset of a rank-three array.
It scans cells without building a full array of mapped indices. Float fill checks
use exact bits, preserving signed zero and NaN payloads: this is a storage
conversion policy, not J's tolerant comparison semantics. Runtime `$.` conversion
must be separately checked against C before claiming identical canonicalization.

## Runtime connection — 2026-09-29

Supported: monadic `$.` (numeric arrays, scalar/previously sparse identity),
`0 $.` toggle, `1 $. shape` with float-zero fill, scalar component queries
`2/3/4/5/7 $.`, `$` shape and `#` tally. Names share the sparse object and queries
share immutable value buffers. `Facts.layout` distinguishes Dense/AxisSparse
from the logical DType. JSON serializes sparse components without densifying.

Unsupported: boxed constructor/control descriptors, axis changes, fill replacement,
compaction controls, sparse arithmetic/comparison/search/index/rank/reduction.
Those paths reject sparse inputs before dense kernels. The two evaluator paths
use the same sparse error path rather than entering the dense buffer pool.

`0 $. sparse` currently limits dense expansion to 16,777,216 atoms. Callers of the
Rust storage API can supply another explicit cap. A runtime Value requires logical
count to fit usize; the standalone SparseArray API can represent larger shapes.
Numeric default-fill conversion treats -0.0 as zero; the independent exact-bit
storage conversion retains its previous policy. Character/boxed array conversion
is rejected following the pinned `vs.c` sparseit restrictions. Scalar identity
precedes this check, matching `jtsparse1`.

C source references: `vs.c:65` constructor default fill, `:126` sparseit,
`:322` monad, `:328` dyadic control dispatch; `xb.c` external sparse type codes.
Dense component-query cases were added to the C conformance corpus. They have
**not** been run against native Windows C because the reference build is blocked.

Latest validation: native Windows MSVC default/portable each passed 75 tests
plus one doctest; fmt and Clippy all-targets with warnings denied passed.
Python harness: 6 tests passed. No Linux tests or GitHub CI were run.
