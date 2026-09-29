# Packed boolean storage — 2026-09-29

`bit_storage::BitStorage` is an immutable physical representation of Bool atoms.
The J runtime continues to use byte Bool arrays; this module is not yet selected
by the planner or exposed through language verbs. No new J bit atom type or C
compatibility code has been invented.

Implemented: checked byte-to-bit conversion, Arc-backed slices with arbitrary
bit offsets, explicit bounded byte expansion, word-wise AND/OR/XOR and popcount.
Bits are stored least-significant first within u64 words. Inputs and output tails
are masked at the logical length, including unaligned slices. Operations require
equal lengths; J prefix agreement is a future dispatch concern.

Payload allocation is `ceil(n / 64) * 8` bytes, excluding Arc/header overhead.
A slice retains its parent's allocation. This is a storage-size property, not a
measured runtime speedup or an assertion that small bit arrays outperform bytes.
No SIMD/CUDA bit kernel or implicit packing policy is implemented.

Three tests compare against byte references over offsets 0 through 129 and
lengths around word boundaries, exercise binary operations with different
operand offsets, and reject invalid byte values, ranges and expansion limits.
Windows default/portable suites each pass 67 tests plus one doctest; Clippy
passes. Python validation harness tests (including synthetic boxed descriptors)
pass 6 tests. These synthetic tests do not replace the pending native C run.

Next: explicit physical-plan layout choice, conversions at kernel boundaries,
shape agreement, performance thresholds, and packed results in runtime storage.
