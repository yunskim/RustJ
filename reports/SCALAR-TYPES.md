# Scalar type foundation

The logical type vocabulary is shared by `types::Scalar` and analysis `DType`.
Existing imports through `syntax::Scalar` and `facts::DType` remain available.

| Logical atom | Representation | Current CPU execution |
| --- | --- | --- |
| Bool / logical bit | bool | supported; byte array storage |
| Int | i64 | supported |
| Float | f64 | supported |
| Char | u8 | supported; wide characters pending |
| Complex | two f64 components | pending |
| ExtendedInt | Arc<BigInt> | pending |
| Rational | Arc<Rational>, reduced finite BigRational | pending |
| Symbol | immutable shared text, equality by contents | pending |
| Boxed | Arc<Value>, retains the enclosed noun's shape | basic runtime support |

Complex, extended integer, rational and symbol remain representation-only.
Boxed runtime support is described in the follow-up below.
`Scalar::into_value` now returns Result and explicitly rejects unavailable array
representations. It cannot silently narrow exact numbers, drop an imaginary part,
or open a box. Existing scalar tokens remain smaller than a full Value header.
num-bigint and num-rational implement exact storage and fraction normalization;
J-specific arithmetic, promotion, tolerance, infinities and errors need separate work.
Zero-denominator rationals return Unsupported rather than panic or invent J semantics.

## Source evidence

Inspected local jsource revision e75016ca74b5e595dd323226e6a4990172f72ec6:
`jsrc/jtype.h` defines B01, CMPX, BOX, XNUM and RAT. B01 has size sizeof(B),
not a packed-bit array. Symbol support must not be inferred from SYMB, which
also names interpreter symbol tables: in this checkout `jsrc/vsb.c` says unused
and the atom definitions do not expose the historical SBT type. RustJ's Symbol
is a planned language capability, not a verified match to this C revision.

## Box ownership and layout

A box is an atom whose payload is an entire noun, including its shape and type.
For example, boxing a 2 by 3 array produces a scalar outer box; the enclosed
noun still has shape 2 by 3. A boxed array needs its own outer shape and an
array of shared noun references. It can contain heterogeneous shapes and types.
Data::Boxed now supports recursive boxed arrays containing implemented Value types.

Data::Boxed uses CpuStorage<Arc<Value>>, not Vec<Scalar>
for all numeric arrays. Homogeneous numeric buffers must remain contiguous for
SIMD and future CUDA work. Boxing/opening should share buffers, with mutation
using copy-on-write on both the noun and its buffer. No raw C pointers or C
reference counters are required. Box graphs must remain immutable/acyclic unless
a separate cycle-management design is introduced. Host reference graphs cannot
be copied directly into GPU memory; device support needs an explicit layout.

## Follow-up checklist

- [x] Shared logical scalar/analysis types, including boxed nouns.
- [x] Exact large integers and normalized finite rationals without f64 conversion.
- [x] Reject unsupported CPU lowering explicitly.
- [ ] Array Data/storage/views/serialization for the new types.
- [ ] Complex, extended integer and rational lexical forms and vector promotion.
- [ ] Monadic < boxing / > opening, preserving existing dyadic comparisons.
- [ ] Boxed indexing, reshape, equality, empty prototypes and heterogeneous open.
- [ ] J rational non-finite semantics and exact numeric conversion/error rules.
- [ ] Symbol construction/interning/order semantics and reference-version policy.
- [ ] Packed-bit array storage, offsets, tail masking and byte/bit conversions.
- [ ] C differential cases for every newly executable operation and performance checks.

Validation (2026-09-29): Windows MSVC default and portable each passed
54 tests plus one doctest; fmt check and Clippy all-targets with warnings denied
passed. On this computer all subsequent verification uses native Windows tools
only. C differential verification of the new types remains pending because the
CPU runtime cannot yet evaluate them. GitHub CI is skipped.

## Boxed execution follow-up — 2026-09-29

Implemented monadic `<`, scalar `>`, identity opening of empty boxed arrays, uniform-shape non-scalar `>` (including
numeric promotion), nested boxes, catenate, reshape, select, rank cell assembly,
and rearrangement without fill. Input buffers are frozen before boxing so opening
shares rather than copies them. JSON recursively records each enclosed noun's
type/shape/data. Text display is a compact diagnostic notation, not J box drawing.
Semantic contracts distinguish monadic boxing's scalar result from dyadic less-than.

Still unsupported: heterogeneous-cell padding on open,
boxed fill, deep equality/order/search, and empty-frame rank prototypes. Unsupported
atomic/search paths reject boxes before numeric code can unwrap a conversion error.
Tests cover aliases, transaction failure, shape preservation, nested sharing and
both evaluator paths. C source references are `jsrc/vo.c` jtbox and jtope.

Windows default/portable each pass 62 tests plus one doctest; Clippy passes.
Windows Python comparator tests: 4 pass. Oracle extraction and conformance cases
now include nested boxes, but **actual new C differential cases are not run**:
a native Windows reference DLL is not available in this checkout. The MSVC
build attempt fails on GNU-style variadic macros in the pinned C headers;
Windows clang-cl was not found in the checked installation paths. Do not reuse
historical Linux differential counts as verification of these changes.
See [SPARSE-ARRAYS.md](SPARSE-ARRAYS.md) for the separate sparse storage milestone.
