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
| Boxed | Arc<Value>, retains the enclosed noun's shape | pending |

This is a representation milestone, not language support for new literals or verbs.
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
The current Scalar representation only references the Value types already supported;
recursive boxed arrays need a future Data::Boxed variant.

A future Data::Boxed storage should use CpuStorage<Arc<Value>>, not Vec<Scalar>
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
