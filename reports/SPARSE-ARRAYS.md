# Sparse storage milestone — 2026-09-29

`SparseArray` is a separate Rust storage API, not a Scalar variant. It is not
yet a runtime `Value` representation and J `$.` syntax is not implemented.

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
- [ ] Runtime array representation and semantic/physical layout facts.
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
