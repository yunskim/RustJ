# Futhark compiler prior art for RustJ's existing Graph/Logical IR

> Reviewed 2026-11-13. **Literature/design study only, not an implemented or verified RustJ optimization.** Keep the existing canonical J Graph IR (`j_graph_ir::Plan`) → Verified Logical Execution IR (`logical_ir::Plan`) → Route/Schedule/Physical pipeline. Do not introduce a mandatory Futhark IR, a third canonical IR, implicit autodiff, or a required Futhark backend.
> Detailed Korean primary study: [futhark-compiler-ir-prior-art.ko.md](futhark-compiler-ir-prior-art.ko.md). Architecture contract: [Graph/Logical responsibilities](../project/chapters/03-architecture-p01.ko.md#두-ir-사이의-최적화-책임).

## What the Futhark-J Bridge does and does not prove

The [Futhark-J Bridge](https://web.liminal.cafe/~byakuren/j/futhark/) compiles **separately authored Futhark functions** with `futhark multicore --library`, builds a native library, caches it, and invokes it through J's FFI. It is **not** automatic compilation or optimization of arbitrary J programs. The author's benchmark claims (e.g. Mandelbrot 1525x and dot product 0.3x relative speed) are not transferable to RustJ. They require comparable algorithms, exact output checks, execution time separated from FFI/compilation, equivalent threading, and independent measurement.

The useful research lies in the compiler's IR and transformation discipline:

| Source | Main insight | Mapping to RustJ |
|---|---|---|
| [Redomap, ARRAY 2016](https://www.futhark-lang.org/publications/array16.pdf) | Producer/consumer and horizontal fusion of map/reduce; `redomap` as an internal high-level operator; subsequent memory-coalesced lowering | Graph equivalence and fanout constraints; Logical reduction semantics; Physical kernel choice |
| [Futhark, PLDI 2017](https://doi.org/10.1145/3062341.3062354); [book Chapter 8](https://futhark-book.readthedocs.io/en/latest/regular-flattening.html) | Moderate flattening for regular nested map/reduce/scan; segmented scans and selective mapping to hardware | Logical cell/segment/domain witnesses; downstream scheduling |
| [Incremental Flattening, PPoPP 2019](https://futhark-lang.org/publications/ppopp19.pdf) | Multiple code versions for different depths of nested parallelism, guarded by hardware/data-tuned predicates | Multiple Schedule/Physical variants of **one** verified Logical computation |
| [Full flattening, July 2026](https://futhark-lang.org/blog/2026-07-31-full-flattening.html) | Fission, segmented representations, branch packing, nonuniform loops, vectorization avoidance, uniform fast paths | Optional shape/domain decomposition and proof obligations; no mandatory ragged-array IR |
| [Parallel accumulators, February 2026](https://futhark-lang.org/blog/2026-02-23-accumulators.html) | Linear, certificate-tagged write-only accumulation for reverse-mode-related scatter patterns | Logical dependency/alias/accumulate legality, physical atomic/segmented implementations |

### 1. Fusion is not flattening

Fusion merges traversals:

```text
map f (map g xs) => map (f ∘ g) xs
reduce op z (map f xs) => fused map/reduce candidate
```

Flattening maps *nested parallelism* to execution patterns supported by hardware. Fission is often needed to split complex map bodies into operations that can be flattened. Excessive fission can increase intermediate buffers and kernel traffic. Optimal compilation balances fusion, fission, locality, and available parallelism, rather than maximizing any one of them.

Redomap treats map/reduce combinations as compiler-level operators, not user syntax. A reduction may be associative without being commutative: preserving order is not the same as permitting arbitrary permutation. IEEE floating-point addition is not generally associative, and J's tolerant comparison does not establish exact e-class equality. Graph rewrites require separate Exact-J evidence or a distinct approximate-policy gate.

### 2. Moderate/regular flattening and segmented scans

```text
map (\row -> scan op identity row) rows
    -> regular segmented scan, when the segment mapping is proven
```

For values `[1,2,3,4,5,6]` and segment-head flags `[T,T,F,T,F,F]`, segmented addition scan produces `[1,2,5,4,9,15]`. A scan over pairs `(value,head_flag)` resets accumulation at a segment head; associativity and identity still need an operator-specific proof. Uniform nested map/map and map/scan cases have efficient flattening rules; nonuniform J rank/frame/cell semantics must not be assumed identical to Futhark's regular arrays.

A map-over-loop can either keep an inner sequential loop per parallel element or, when shape-preserving and legal, interchange map and loop. This choice belongs to Schedule/Physical after the Logical verifier preserves the iteration-domain and error/effect obligations.

### 3. Incremental flattening: physical multiversioning

The PPoPP 2019 work produces several valid code versions and runtime predicates tuned to dataset and hardware properties, with a code-size tradeoff. RustJ should preserve **one** verified logical operation graph and consider alternate physical plans (outer parallel / inner sequential; segmented reduction; tiled parallel kernels), with resource bounds, legality checks and fallback. No third canonical IR is necessary.

### 4. Full flattening: nonuniform nested parallelism

The 2026 design handles cases like:

```text
map (\n -> reduce (+) 0 (iota n)) [1,3,2]
lengths=[1,3,2], offsets=[0,1,4]
values=[0,0,1,2,0,1]
heads =[T,T,F,F,T,F]
```

This representation is **explanatory**, not a proposed mandatory RustJ IR. The Futhark design fissions map bodies, generates segmented counterparts, packs branch lanes and tracks live lanes in nonuniform loops. It avoids fission of scalar “super-statements” to preserve locality. Size types permit uniform fast paths without segment metadata, improving downstream analysis/tiling. The July 2026 post described a near-merge compiler implementation; it does not establish that every nonuniform workload is efficient or that RustJ already supports it.

### 5. Parallel accumulators: the IR's effect boundary

An explicit gather input-adjoint needs a scatter accumulation:

```text
y[k] = x[index[k]]
dx[index[k]] += dy[k]   # potential duplicate index
```

Naively generating an entire dense contribution array per iteration risks excessive memory. Futhark's **IR-only** `with_acc` / `update_acc` employs `acc(c,t)`: a certificate-tagged, linear, write-only view. The certificate ties an accumulator to its scope; linear use prevents accidental duplication or observation; a special `map` rule logically splits/joins accumulators. The denotational model is a list of index/value updates; actual execution may use atomic operations. Associativity, commutativity, bounds, aliasing, races and floating-point nondeterminism must be treated explicitly.

RustJ keeps **explicit adjoint primitives**, does not enable implicit autograd, and does not conflate Futhark `with_acc` with RustJ's existing `with` conjunction. Consider a restricted Logical IR accumulation contract and only then alternative physical implementations.

## Layer-by-layer contract

| Layer | Allowed | Not allowed |
|---|---|---|
| J semantic frontend | J function construction, rank, names, binding/version, effects | Eager SOAC rewriting or semantic changes |
| **Graph IR** | Preserve `StructuredConv`, reduction/scan, Algebraic Views, candidate rewrites with witness and provenance | Threads, buffers, tiles, device layout |
| **Verified Logical IR** | Optional `IndexMap`/Window/Contract decomposition, SSA/checks, segment domains, accumulate/alias/error facts, `CellApply` | Actual atomic instructions, hardware thread blocks |
| **Schedule/Physical** | Fusion/fission execution, loop interchange, segmentation, tiling, kernel variant choice and guards | Redefining J primitive semantics |
| Executor | Verified fallback, benchmarks, runtime guards | Silent changes to first-error or numerical contract |

The bridge to [SPORES/egg](common-array-algebra-prior-art.md) and [einfold](xarray-sql-attention-review.md) is complementary: e-graph search explores equivalent **graph computations**, while flattening/scheduling explores equivalent **execution realizations**, subject to independent J legality and numerical checks.

## Implementation/proof gates — all OPEN

- [ ] **FUT-01** Map/map fusion: fanout, NAME, effect/first-error, provenance and counterexamples.
- [ ] **FUT-02** Map/reduce/redomap: associative vs commutative, empty, IEEE, overflow, dtype.
- [ ] **FUT-03** `CellApply(Scan)` frame/cell/segment proof against J-C; empty/prototype.
- [ ] **FUT-04** Uniform/nonuniform shape witnesses, offsets/flags and bounds.
- [ ] **FUT-05** Loop/map interchange: shape-preserving iteration, first error and ordered effects.
- [ ] **FUT-06** Fusion/fission costs: memory traffic, temporary bytes, launches and locality.
- [ ] **FUT-07** Two or more valid physical variants + guards/fallback from one verified Logical IR.
- [ ] **FUT-08** Explicit gather/Conv adjoint accumulation: duplicate indices, alias, races, bounds.
- [ ] **FUT-09** Atomic vs deterministic segmented reduction and Exact-J vs approximate numerical policy.
- [ ] **FUT-10** Independently benchmark bridge workloads with equal algorithms, output checks and FFI separated.
- [ ] **FUT-11** Pin and inspect the actual upstream full-flattening compiler implementation before adopting details.
- [ ] **FUT-12** Confirm integration with existing `j_graph_ir.rs`, `logical_ir.rs`, `fusion_planning.rs` and `physical_plan.rs`.

Documentation alone never closes an implementation/testing gate.

## Primary bibliography

1. [Futhark-J Bridge](https://web.liminal.cafe/~byakuren/j/futhark/) — upstream project and published benchmark claims.
2. Henriksen, Larsen, Oancea, *Design and GPGPU Performance of Futhark's Redomap Construct*, ARRAY 2016, [PDF](https://www.futhark-lang.org/publications/array16.pdf), [DOI](https://doi.org/10.1145/2935323.2935326).
3. Henriksen et al., *Futhark: Purely Functional GPU-programming with Nested Parallelism and In-place Array Updates*, PLDI 2017, [DOI](https://doi.org/10.1145/3062341.3062354).
4. *Parallel Programming in Futhark*, [Chapter 8: Regular Flattening](https://futhark-book.readthedocs.io/en/latest/regular-flattening.html).
5. Henriksen, Thorøe, Elsman, Oancea, *Incremental Flattening for Nested Data Parallelism*, PPoPP 2019, [PDF](https://futhark-lang.org/publications/ppopp19.pdf), [DOI](https://doi.org/10.1145/3293883.3295707).
6. Henriksen, [*The why and how of parallel in-place random-access accumulation*](https://futhark-lang.org/blog/2026-02-23-accumulators.html), February 2026.
7. Henriksen, [*Full flattening of nested data parallelism*](https://futhark-lang.org/blog/2026-07-31-full-flattening.html), July 2026.
8. [Futhark documentation — Bridges](https://futhark-lang.org/docs.html).
