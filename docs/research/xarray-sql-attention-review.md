# Attention benchmark implications for RustJ

Status: design review and implementation checklist; **not an implemented optimization**.

## Source and scope

- xqlsystems/xarray-sql, pinned commit `6debe2495dcb4e8fac20ab97df5b89baae2a31a6`, [benchmarks/ml/01_attention.py](https://github.com/xqlsystems/xarray-sql/blob/6debe2495dcb4e8fac20ab97df5b89baae2a31a6/benchmarks/ml/01_attention.py).
- Related projects referenced by that benchmark: [einfold](https://github.com/xqlsystems/einfold), [ddx](https://github.com/xqlsystems/ddx), [FlashAttention](https://arxiv.org/abs/2205.14135).
- The benchmark demonstrates and tests a claimed fused plan; the optimizer's implementation, cost model, and performance results require separate examination. Do not treat its comments as proof of a general rewrite.

## What the benchmark demonstrates

1. Causal attention is expressed as two indexed sum-product joins with a row-wise stable softmax: `softmax(Q Kᵀ / sqrt(d)) V`.
2. Two equivalent SQL spellings (window max versus grouped max joined back) target the same fused streaming plan.
3. A Nadaraya–Watson Gaussian-kernel weighted average is a non-attention control for structural rather than operator-name matching.
4. A causal mask is represented by *absent pairs*, not multiplication by zero. The infinity test detects invalid `0 * Inf` behavior.
5. The model and loss are separated by a named view; ddx computes gradients of Q/K/V and checks against JAX. This is evidence for a test methodology, **not** a reason to add implicit autograd to RustJ.

## RustJ design implications

### Graph IR

- Preserve structured `attention` / `softmax` / `contraction` primitives when available, alongside the graph-expansion alternative; do not require users to spell attention as scalar J loops.
- Normalize independently expressed row maximum, exponential, denominator reduction, weighted contraction, and causal predicate into an analyzable **normalized weighted reduction** candidate, without erasing observable J semantics.
- Pattern family: `output[i,f] = sum_j(exp(score(i,j)-m_i)*value(j,f))/sum_j(exp(score(i,j)-m_i))` with `m_i=max_j score(i,j)`; `score` may be a contraction or another function (e.g. Gaussian kernel).
- The candidate is an optimization *view*, not a new mandatory Graph IR node or proof of equivalence.

### Logical IR

- Represent index domains and predicates explicitly: `(i,j)` valid iff `j <= i` for causal attention. Distinguish absent domain entries from present zero values.
- Keep index-map expansion logical; do not materialize an `N×N` score tensor solely because the mathematical expression names one.
- Consider alternative execution plans: materialized scores; tiled score/softmax/value contraction; online max/sum/numerator accumulation.
- Online block merge (for compatible finite real inputs): each block has max `m`, denominator `l=sum exp(s-m)`, and vector numerator `a=sum exp(s-m)*v`. Merge two states with `M=max(m1,m2)`, `L=exp(m1-M)l1+exp(m2-M)l2`, `A=exp(m1-M)a1+exp(m2-M)a2`; output `A/L`. Specify empty/all-masked rows separately.
- Cost model must compare memory footprint, recomputation, tiling, layout, numerical error and target device. Never claim FlashAttention speed without measurement.

### Rewrite legality and numerical semantics

- Rewrites require J rank, shape, axis, dtype, error, missing/empty, NaN/Inf, and effect contracts.
- Online softmax changes summation order and may change floating results. J's tolerant equality is **not** a blanket license to claim exact program equivalence. Track an approximate/numerical-equivalence policy separately from Exact-J rewrites.
- Do not translate SQL NULL semantics into J without explicit semantics.
- For gradients, retain RustJ's **explicit adjoint primitives** policy. A fused forward primitive may have an explicit fused adjoint; do not add default autograd.

## Checklist

- [ ] Define a weighted-reduction structural matcher across graph spellings.
- [ ] Add legality facts for reduction axes, masks, empty domains, NaN/Inf and floating-point behavior.
- [ ] Model materialized and online tiled execution alternatives in Logical IR.
- [ ] Explore e-graph rewrites only after preconditions and equivalence classes are formalized.
- [ ] Add causal attention and Nadaraya–Watson as distinct positive test workloads.
- [ ] Add window-style vs grouped-max-style normalization tests.
- [ ] Add a masked infinity regression that distinguishes absent pairs from zero multiplication.
- [ ] Compare reference outputs and explicit Q/K/V adjoints with an independent numerical implementation.
- [ ] Benchmark peak memory, runtime, and numerical deviation over varying sequence lengths and devices.
- [ ] Inspect einfold's actual rule implementation before adopting any rule or performance claim.

## Architectural conclusion

Keep Graph IR and Logical IR separate: Graph IR preserves mathematical structure and alternative graph forms; Logical IR explores indexed execution strategies and avoids unnecessary materialization. The important reusable optimization target is **normalized weighted reduction**, not the string `attention`.


## Follow-up: actual einfold implementation review (2026-10-31)

The upstream implementation was inspected, rather than inferring it from the benchmark:

- [einfold README](https://github.com/xqlsystems/einfold/blob/main/README.md)
- [design, sections 2.2 and 9.6](https://github.com/xqlsystems/einfold/blob/main/docs/design.md)
- [engine-independent Fold IR](https://github.com/xqlsystems/einfold/blob/main/crates/einfold-ir/src/fold.rs)
- [engine-independent Pipeline IR](https://github.com/xqlsystems/einfold/blob/main/crates/einfold-ir/src/pipeline.rs)
- [DataFusion pipeline detector](https://github.com/xqlsystems/einfold/blob/main/crates/einfold-datafusion/src/detect_pipelines.rs)
- [DataFusion integration](https://github.com/xqlsystems/einfold/blob/main/crates/einfold-datafusion/src/lib.rs)

### Confirmed architecture and constraints

Current einfold 0.1 detects shapes in DataFusion's *optimized logical plan*, immediately before physical planning. `DetectFolds` / pipeline detection replace recognized regions with `FoldNode` / `PipelineNode`; `EinFoldExec` runs a selected kernel and retains a host-plan fallback. The engine-independent IR and kernels are separated from the DataFusion integration.

The `Fold` IR records dimension identities, operand keys/factors, output dimensions, row expressions, join conditions and optional output domain. This is not just an einsum: a **fold over a join** can aggregate an arbitrary expression. A semiring fold is a more restrictive subtype eligible for additional algebraic rewrites. Do **not** assume every fold permits reassociation/distribution.

The `Pipeline` IR has a producer fold, score scaling/shift, weight, value operand and consumer reduction. Its documented fusion precondition: each producer group must correspond to a consumer group together with dimensions the consumer reduces. The detector explicitly matches window-max and grouped-max softmax spellings, alternative numerator/denominator division placement, and requires copied score/weight computations to agree. It declines on unsupported forms.

**Implementation versus roadmap:** the upstream design describes egglog-based exploration, specialized contraction planning, memory-budget tiling and GPU hosts, but these are deferred. The current detector is deliberately narrow and rule-driven. RustJ should not cite einfold as an already implemented generalized equality-saturation optimizer.

### RustJ integration boundary

```text
J Frontend -> Graph IR
                 | graph rewrite / graph expansion
                 | discover FoldCandidate / FoldPipelineCandidate
                 | attach shape, rank, domain, effect and numerical facts
                 v
             Logical IR
                 | index domains, key mapping, output axes
                 | select materialized / streamed / tiled / sparse plan
                 | cost and memory budget + fallback
                 v
             backend kernels
```

- **Graph IR**: discover high-level fold and producer/consumer dependency patterns while retaining original primitives and witnesses. A fold candidate is an *analysis view*, not a forced lowering. E-graph equivalence applies only when the relevant J semantics and numeric policy justify it.
- **Logical IR**: make the producer/consumer axis mapping explicit and prove the fusion precondition; choose a fused tile pipeline or materialized alternative. Conditions define **domain membership** (e.g. causal `j<=i`), not a zero-valued multiply.
- **Runtime**: use fast dense positional indexing only if validated by coordinate/shape facts; otherwise choose a suitable sparse/indexed plan or safe fallback. Upstream's SQL NULL, duplicate-key and grouping rules are not J rules.
- **Numerical policy**: online stable softmax and changed reduction order need an explicit floating-point equivalence policy; fixed tile boundaries can support deterministic results but are not automatically Exact-J. Special cases: empty/all-masked groups, NaN/Inf, signed zero, overflow, rank and dtype.
- **Adjoints**: preserve explicit adjoint primitives. Fusion of the forward graph does not silently introduce ddx-like automatic differentiation.

### Implementation gates

- [ ] G1: Define `FoldCandidate` facts: operand IDs, dimension IDs, output axes, row expression, aggregation operator, predicate/domain, legality witness.
- [ ] G2: Classify fold vs semiring fold vs algebraic fold; permit distributive rewrites only for proven semiring contracts.
- [ ] G3: Define `FoldPipelineCandidate` producer/consumer axis correspondence and a checkable fusion proof obligation.
- [ ] G4: Normalize supported softmax spellings (window max/group max, numerator/denominator placement), without changing J-observable behavior.
- [ ] G5: Build Logical IR alternatives for materialized and tiled online accumulation; keep a correctness-preserving fallback.
- [ ] G6: Add property-based tests for axis mappings, causal/sliding masks, non-attention Gaussian weighted average, all-masked rows, NaN/Inf, and floating error policy.
- [ ] G7: Measure allocation peak, elapsed time, scaling, and repeatability; do not transfer upstream benchmark speedups to RustJ.
- [ ] G8: Revisit egg/egglog extraction and cost-guided tiling only after G1–G7 demonstrate a useful optimization.

**Recommended first implementation:** a narrow, semantics-checked FoldCandidate recognizer and a two-fold pipeline witness; defer general e-graph rule expansion until this path is independently tested.


## Executable work plan / progress tracker

**Tracking policy:** `[ ]` means not verified, `[x]` means completed with linked evidence. A design note alone does not count as implementation. Update this section whenever code or tests change. Keep a link to the relevant PR/commit beside each completed gate. Do not mark work done solely because the design has been described.

**Dependency order:** P0 contracts → P1 recognition → P2 fusion proof → P3 Logical IR alternatives → P4 execution and numerical validation → P5 optional equality-saturation expansion. Preserve the existing Graph IR / Logical IR split; do not create a third mandatory IR.

### P0 — Scope and contracts (design first)

- [ ] P0.1 Inventory existing Graph IR, Logical IR, FactWitness, e-graph, NN extension and adjoint interfaces; record concrete integration points and avoid duplicate abstractions.
- [ ] P0.2 Specify a `FoldCandidate` contract: source node IDs, operand/axis correspondence, reduction axes, row expression, output axes, domain predicate, dtype/rank/shape, effects, numeric policy, provenance.
- [ ] P0.3 Specify a `FoldPipelineCandidate` contract: producer and consumer IDs, kept/summed dimensions, scale/normalizer, value operand, matching witness, fallback.
- [ ] P0.4 Document legality and counterexamples: empty domains, rank/cell/frame, masked vs zero, NaN/Inf, signed zero, floating reduction order, J errors and effects.
- **Exit:** reviewable contracts and at least one rejected counterexample per relevant legality family.

### P1 — Graph IR fold recognition

- [ ] P1.1 Detect basic reduction and sum-of-products contraction without lowering to explicit scalar loops.
- [ ] P1.2 Classify generic fold, semiring fold and algebraic fold; allow distributivity only with an explicit law witness.
- [ ] P1.3 Recognize Gaussian pair-score weighted aggregation as a non-attention case.
- [ ] P1.4 Emit diagnostic evidence explaining why each candidate was accepted or rejected.
- **Exit:** positive and negative matcher tests; unchanged Graph IR execution behavior.

### P2 — Pipeline recognition and proof

- [ ] P2.1 Detect producer score fold → transform/normalizer → consumer weighted reduction.
- [ ] P2.2 Verify producer groups map to consumer groups plus eliminated dimensions; reject invalid joins/axis reuse.
- [ ] P2.3 Recognize mathematically compatible softmax spellings only when J semantic and numeric policies permit.
- [ ] P2.4 Preserve causal/sliding/band mask as domain restrictions, not zero multiplication.
- **Exit:** independently tested fusion witness, with causal attention and Nadaraya–Watson positive cases and adversarial negatives.

### P3 — Logical IR planning

- [ ] P3.1 Lower a validated candidate into indexed Logical IR with explicit domain and axis mapping.
- [ ] P3.2 Keep both materialized and fused/tiled alternatives; no eager `N×N` materialization requirement.
- [ ] P3.3 Define online max/denominator/numerator accumulator and merge semantics, including all-masked/empty cases.
- [ ] P3.4 Add device, memory-budget, layout and cost facts; reject infeasible plans.
- [ ] P3.5 Ensure fallback to the original valid execution path when recognition, legality or resource checks fail.
- **Exit:** plan comparison and reproducible selection rationale.

### P4 — Kernels, tests, measurements

- [ ] P4.1 Implement a minimal CPU reference executor and a fused tiled executor; do not assume GPU availability.
- [ ] P4.2 Differential tests versus independent reference for attention and Gaussian weighted average, including adversarial numerical cases.
- [ ] P4.3 Verify deterministic behavior under selected scheduling policy and document allowable numerical deviation.
- [ ] P4.4 Validate explicit adjoint primitive compatibility without adding implicit autograd.
- [ ] P4.5 Benchmark sequence length, tile size, memory peak, execution time and correctness; report failed/unsupported cases.
- **Exit:** measurable benefit on at least one workload without violating the selected J semantics policy; otherwise retain the unfused path.

### P5 — Optional e-graph expansion (after P4)

- [ ] P5.1 Add proven fold and pipeline rewrites to the existing egg/e-graph framework only after checking preconditions.
- [ ] P5.2 Compare bounded equality saturation against direct pattern detection in compilation cost and selected plan quality.
- [ ] P5.3 Decide whether generalized cost-guided extraction is justified; keep direct recognition if not.
- **Exit:** measured evidence for or against expanding the rule set.

### Progress log

| Date | Change | Evidence |
| --- | --- | --- |
| 2026-10-31 | Reviewed xarray-sql benchmark and einfold implementation; documented contracts and phased plan. No RustJ implementation or tests claimed. | Source links above and this document |

**Next actionable item:** P0.1, followed by P0.2–P0.4. Do not begin kernel implementation before the semantics and witness boundaries are specified.


## Research lineage: folds, relational algebra and tensor execution

This section distinguishes **documented upstream references**, **conceptual predecessors**, and **RustJ design proposals**. Similarity of ideas does not establish that einfold directly implements a named paper's algorithm.

### A. Functional folds and algebraic aggregation

Functional programming's `fold`/`reduce` abstracts iteration over a sequence using a state update. Parallel decomposition additionally needs a mergeable summary, often a **monoid** `(S, ⊕, e)`: closure, associativity and an identity. For example a mean can use summary `(sum,count)` with componentwise addition, followed by `sum/count`. This is a mathematical specification; IEEE floating-point addition is not strictly associative.

A general state transition `s' = f(s,x)` does **not** imply a merge operator exists. In particular a J Fold conjunction (`F.` family) is a language-level iteration construct, while einfold's fold is an indexed aggregate over related inputs. A RustJ J-Fold implementation must preserve iteration order, control/termination, collected outputs, errors and effects. Only a *proved* aggregate subset can use mergeable-fold optimizations. Do not conflate the two or require J Fold as an input to the optimizer.

References:
- [J vocabulary: Fold conjunction](https://code.jsoftware.com/wiki/Vocabulary/fcap)
- [Bird–Meertens formalism, overview](https://en.wikipedia.org/wiki/Bird%E2%80%93Meertens_formalism) (orientation only; prefer primary papers for laws)
- [Gray et al., Data Cube: A Relational Aggregation Operator (1997)](https://doi.org/10.1023/A:1009726021843) — distributive/algebraic/holistic aggregate distinctions.

### B. Database semirings, factorization and aggregate queries

**Semiring provenance** (Green, Karvounarakis & Tannen, *Provenance Semirings*, PODS 2007) provides a principled algebra for relational operations. The broader lesson is that join/product and union/sum laws depend on the chosen semiring, rather than on syntax alone. In RustJ, any proposed distribution, reassociation or elimination rule needs its own dtype/shape/identity/effect witness.

**Factorized databases** (Olteanu and collaborators) represent repeated relational structure compactly instead of expanding every join tuple. This motivates *avoiding materialization* but is not identical to tensor tile fusion: factorized representation, streamed join-aggregation and GPU tiling are distinct techniques.

**FAQ / InsideOut** (Abo Khamis, Ngo & Rudra, *FAQ: Questions Asked Frequently*, 2016) studies functional aggregate queries over variables and semiring-like operations; variable elimination order and structural width drive complexity. For RustJ, contracted/reduced axes are candidates for variable elimination, and Logical IR can explore alternative orders subject to legality and cost. This does not mean every J array program is an FAQ instance.

Primary sources:
- [Green et al., Provenance Semirings (2007)](https://doi.org/10.1145/1265530.1265535)
- [Abo Khamis, Ngo & Rudra, FAQ: Questions Asked Frequently (2016), arXiv](https://arxiv.org/abs/1504.04044)
- [Olteanu & Závodný, Factorised Representations of Query Results (2012)](https://doi.org/10.1145/2213836.2213869)

### C. Tensor-to-relational algebra and equality saturation

A tensor contraction `C[i,k] = Σ_j A[i,j]·B[j,k]` corresponds to matching the shared `j` axis and aggregating over it. In SQL this is an equality join plus `GROUP BY i,k` and `SUM(A.v*B.v)`. The 2023 Einstein-summation-in-SQL work is a **directly acknowledged source in einfold's design**. The equivalence of a mathematical contraction to a SQL query does not imply the SQL engine will select a competitive physical algorithm.

**SPORES** (*Sum-Product Optimization via Relational Equality Saturation*, 2020) uses a relational representation and equality saturation to explore alternative linear algebra expressions. This is conceptually complementary to FAQ's variable-elimination/order perspective: SPORES explores valid equivalent expressions; FAQ analyzes evaluation of aggregate queries. Neither is automatically a dependency of the current einfold implementation. einfold's design mentions egglog, but the implementation inspected above currently uses narrower pattern detectors and specialized kernels.

Primary sources:
- [Blacher et al., Efficient and Portable Einstein Summation in SQL (2023)](https://doi.org/10.1145/3589266)
- [Wang et al., SPORES (2020)](https://arxiv.org/abs/2002.07951)
- [einfold design and current-status distinction](https://github.com/xqlsystems/einfold/blob/main/docs/design.md)

### D. Online normalization and I/O-aware attention

**Memory-efficient attention** (Rabe & Staats, 2021) demonstrates attention without retaining a quadratic-size attention matrix. **FlashAttention** (Dao et al., 2022) exploits tiling and reduced memory traffic. An online stable softmax can maintain a row state `(m,l,a)` for maximum, shifted weight sum and weighted-value sum, then output `a/l`. The key opportunity is to fuse producer score computation with consumer weighted reduction so score tiles need not be materialized globally.

These works justify the *execution algorithm*, not an arbitrary compiler rewrite. The compiler must separately establish producer/consumer index correspondence, mask domain, legal normalization, numerical policy, and fallback. einfold's distinctive contribution in this comparison is detecting such opportunities in an existing SQL logical plan.

Primary sources:
- [Rabe & Staats, Self-attention Does Not Need O(n²) Memory (2021)](https://arxiv.org/abs/2112.05682)
- [Dao et al., FlashAttention (2022)](https://arxiv.org/abs/2205.14135)
- [einfold Fold/Pipeline IR](https://github.com/xqlsystems/einfold/blob/main/crates/einfold-ir/src/pipeline.rs)

### E. Comparative map for RustJ

| Research tradition | Core question | RustJ placement | Evidence / limitation |
| --- | --- | --- | --- |
| J Fold / functional fold | How does sequential state evolve? | Frontend semantics; Graph IR state-flow analysis | Not necessarily mergeable |
| Monoid / semiring / data cube | Which aggregates and rewrites are lawful? | Algebra contracts and FactWitness | Float and effects need explicit restrictions |
| Factorized databases | How can repeated intermediate data be represented or avoided? | Logical IR representation and execution | Distinct from automatic tile fusion |
| FAQ / InsideOut | Which index/variable should be eliminated first? | Logical IR alternative evaluation orders | Requires representable query structure |
| SPORES | Which equivalent algebraic expressions can be explored? | Graph IR/e-graph, with verified laws | Equality saturation is not a proof of an unvalidated rule |
| einfold | Can a fold/pipeline be recognized and mapped to kernels? | Graph IR candidates → Logical IR execution choices | Current implementation is narrower than its roadmap |
| Online softmax / FlashAttention | How to compute weighted reductions with low memory traffic? | Tiled Logical IR/backend | Numerical semantics and hardware costs matter |

### F. Research checklist to incorporate into the implementation gates

- [ ] R1. Read FAQ/InsideOut and document a concrete RustJ contraction with two legal variable-elimination orders; record estimated intermediate shapes.
- [ ] R2. Review factorized-query evaluation and distinguish **compact representation**, **streaming**, and **tiling** in the Logical IR design.
- [ ] R3. State the exact monoid/semiring assumptions behind each candidate rewrite; include counterexamples for floating point and effects.
- [ ] R4. Compare SPORES-style expression exploration with direct fold detection on one end-to-end example; do not assume egglog is already integrated upstream.
- [ ] R5. Document whether and when J `F.` can be recognized as a mergeable aggregate; include a nonmergeable recurrence counterexample.
- [ ] R6. Compare the materialized attention reference with online-softmax tile accumulation, including all-masked and nonfinite inputs.
- [ ] R7. Verify primary-source metadata and link stability before using these references in a formal publication.

**Recommended study order:** FAQ → semiring/aggregate laws → factorization → SPORES → einfold implementation → online softmax/FlashAttention. The first deliverable is an *index-elimination example with legality witnesses*, not an optimizer implementation.

### Progress log addition

- **2026-10-31:** Added a sourced research lineage and R1–R7 research checklist. These items are research tasks, not completed implementation.


## FAQ / InsideOut worked example: variable elimination in Logical IR

**Status:** design example only; no planner or proof checker implemented. This is a follow-up to research task R1.

### 1. Query and axes

Consider the chain contraction

```text
Y[i,l] = sum(j,k, A[i,j] * B[j,k] * C[k,l])
A: [I,J]     B: [J,K]     C: [K,L]     Y: [I,L]
free axes: i,l          reduced axes: j,k
```

In FAQ terms, each factor touches a subset of variables, and the optimizer may eliminate bound variables in different orders. The graph has edges `(i,j), (j,k), (k,l)`. The same mathematical expression admits multiple evaluation plans:

**Plan A — eliminate j first**

```text
T[i,k] = sum_j A[i,j] * B[j,k]     # T: [I,K]
Y[i,l] = sum_k T[i,k] * C[k,l]     # Y: [I,L]
```

**Plan B — eliminate k first**

```text
U[j,l] = sum_k B[j,k] * C[k,l]     # U: [J,L]
Y[i,l] = sum_j A[i,j] * U[j,l]     # Y: [I,L]
```

Both avoid materializing a four-dimensional `[I,J,K,L]` product. The elimination order changes intermediate shape and arithmetic cost. For dense matrix products, approximate multiply-add counts are `I*J*K + I*K*L` (A) versus `J*K*L + I*J*L` (B), ignoring constant factors and hardware behavior.

### 2. Concrete asymmetric dimensions

Let `I=4096, J=16, K=4096, L=16`.

| Metric | Plan A (j first) | Plan B (k first) |
| --- | ---: | ---: |
| First intermediate | `T[4096,4096]` | `U[16,16]` |
| Intermediate elements | 16,777,216 | 256 |
| Approx. multiply-adds | 269,484,032 | 1,048,576 |
| Intermediate float32 storage (if materialized) | 64 MiB | 1 KiB |

The counts assume dense operands and standard matrix-chain evaluation. They are **illustrative cost estimates**, not benchmark measurements. For different dimensions the preference may reverse. A streamed/tiled executor may reduce peak memory without changing the algebraic plan, so intermediate *logical shape* and *physical allocation* must be tracked separately.

### 3. Suggested Graph IR / Logical IR contract

Graph IR should retain a canonical contraction expression and candidate alternative factorizations, with provenance of the rewrite:

```text
Contraction {
  factors: [A(i,j), B(j,k), C(k,l)],
  free_axes: [i,l],
  reduce_axes: [j,k],
  combine: multiply,
  aggregate: sum,
  semantics_witness: ...
}
```

Logical IR can then explore `Eliminate(j) -> Eliminate(k)` versus `Eliminate(k) -> Eliminate(j)`, attaching:
- resulting axis sets and extents for each intermediate;
- dense/sparse/storage layout assumptions and optional materialization/tiling;
- estimated operations, bytes moved, peak live bytes, and backend capabilities;
- legality witness and a safe fallback to the original evaluation order.

This is a **proposed schema**, not an assertion about existing RustJ types. Check P0.1 before introducing types or new IR layers. The same planner can later consider a producer→normalizer→weighted-reduction pipeline, but normalizers and masks impose additional dependencies beyond a plain sum-product contraction.

### 4. Correctness boundary and counterexamples

For exact arithmetic over a suitable semiring, reassociating finite sums/products and eliminating independent variables in either order is legal. For J semantics, the optimizer must separately account for:
- floating-point reassociation/rounding, signed zero, NaN/Inf, overflow and the language's tolerance policy;
- noncommutative or order-sensitive combining/aggregation operators;
- missing values, masks, shape/rank behavior and zero-length dimensions;
- effects, errors, evaluation order and explicit adjoint contracts.

**A J approximate-equality threshold is not by itself a proof** that reassociation is legal or that results remain within tolerance for arbitrary input sizes. If the necessary witness is absent, do not generate the alternative plan.

### 5. FAQ versus matrix-chain optimization

This three-factor example is also classical **matrix-chain multiplication order optimization**. FAQ/InsideOut generalizes the idea to factor scopes, variable elimination and aggregate query structure; this example alone does not demonstrate FAQ's full complexity or width analysis. A subsequent test should use a branched factor graph and compare variable-elimination orders with different intermediate scopes.

### R1 completion criteria and follow-up

- [x] R1.a Write one contraction with two elimination orders and explicit intermediate shapes.
- [x] R1.b Calculate representative operation/storage estimates and note the assumptions.
- [x] R1.c Identify legality witnesses and numerical counterexamples to investigate.
- [ ] R1.d Cross-check the example against RustJ's current IR types and FactWitness implementation (P0.1).
- [ ] R1.e Add a branched-factor example and test legality/cost estimates in code.
- [ ] R1.f Read the FAQ paper's formal elimination definitions and record where this design diverges.

References:
- [FAQ: Questions Asked Frequently, Abo Khamis, Ngo, Rudra](https://arxiv.org/abs/1504.04044)
- [SPORES: Sum-Product Optimization via Relational Equality Saturation](https://arxiv.org/abs/2002.07951)
- [Classical matrix-chain multiplication](https://en.wikipedia.org/wiki/Matrix_chain_multiplication) (background only)


## P0.1 source inventory: actual RustJ IR boundaries (2026-10-31)

Inspected repository source rather than assuming the proposed FAQ schema exists:

| File | Observed source facts | Implication for FAQ |
| --- | --- | --- |
| [`src/j_graph_ir.rs`](../../src/j_graph_ir.rs) | `GraphBasisKind` already includes `Reduce`, `Window`, `CellApply`, `Structured`; `GraphBasis.layers` records composition; `NodeKind::Apply` preserves the function identity and operation contract. | Recognize aggregate/contraction candidates as analysis on existing graph nodes; do **not** add a new mandatory IR stage. |
| [`src/logical_ir.rs`](../../src/logical_ir.rs) | `IterationAxis` has `position`, optional `extent`, `kind`, `role`; `AxisRole` includes `Reduction`; `ExecutionBasisPayload::Reduce` currently stores `ReductionAxis`. | Existing axis model is a starting point, but no demonstrated factor-scope graph or multi-variable elimination schedule; introduce those only as an opt-in candidate/plan descriptor. |
| [`src/logical_ir.rs`](../../src/logical_ir.rs) | `FactWitness` currently covers `PrefixAgreement` and `CellFrameAgreement`. | It does **not** prove semiring laws, reassociation or numerical equivalence. Add distinct legality obligations rather than treating shape witnesses as algebraic proof. |
| [`src/fusion_planning.rs`](../../src/fusion_planning.rs) | Readiness reports retain unresolved `ProofObligation`, `AwaitingSemanticProofs`, and `selected: false`. | Candidate generation can reuse the readiness/obligation boundary; never claim a fused candidate is executable merely because source bases are supported. |
| [`src/physical_plan.rs`](../../src/physical_plan.rs) | Explicit physical buffers/views, execution device, memory space and physical operations exist. | Keep memory allocation and tiling decisions downstream of logical axis planning. |
| [`src/facts.rs`](../../src/facts.rs), [`src/contracts.rs`](../../src/contracts.rs) | Existing shape/rank and operation contract infrastructure. | Reuse facts for dimensions, but algebraic legality needs a stronger, separately recorded contract. |

### Decision and implementation boundary

**Decision:** Keep the existing Graph IR → Logical IR → physical plan architecture. Attach `AggregateFoldCandidate` / `EliminationOrderCandidate` as analysis artifacts rather than introducing another IR layer. Proposed descriptors must refer to existing graph value IDs and explicitly list factor axis scopes, free/reduced axes, operations, semantic assumptions and outstanding proof obligations. Logical IR may then carry a chosen elimination schedule, if proved legal, with per-step output scopes and cost estimates.

**Not yet present or demonstrated:** generalized contraction recognition across J expressions; FAQ variable-elimination enumerator; monoid/semiring legality witnesses; executable fold-pipeline fusion; cost-based plan selection. The source inventory is not evidence that these are implemented.

### Updated P0.1 checklist

- [x] P0.1.a Inspect Graph IR basis and application representation.
- [x] P0.1.b Inspect Logical IR axes, reduction payload and FactWitness.
- [x] P0.1.c Inspect fusion readiness and physical planning boundaries.
- [x] P0.1.d Record the minimal-extension decision without a new IR layer.
- [ ] P0.1.e Trace one real J contraction from frontend through both IRs; identify where factor scopes and legality information are lost.
- [ ] P0.2 Define candidate descriptor fields against concrete existing types.
- [ ] P0.3 Define producer/consumer pipeline scope and alignment checks.
- [ ] P0.4 Specify proof obligations and safe fallback for J numeric semantics.

**Next:** P0.1.e, then P0.2. No source code, runtime tests, or CI were changed/executed for this inventory.


## P0.1.e trace: concrete J reduction and the missing contraction metadata

**Source-level trace (not a runtime execution):** take the J expression `+/ 1 2 3`, a monadic reduction by addition. The code paths below were inspected; the example was **not** run through a parser or printed as an actual IR dump.

1. **Frontend handoff:** `src/frontend_handoff.rs` exposes `FrontendHandoff::from_program` and `lower_name_effects`. Graph IR has `Plan::from_bound` and `from_capture` constructors. These are distinct entrypoints; a verified end-to-end runtime trace is still pending.
2. **Graph IR classification:** `src/j_graph_ir.rs::classify_function` explicitly matches `PrimitiveAdverb(AdverbId::Slash)` and returns `GraphForm::Reduce { operand }` plus `GraphHint::ReductionStructure`. `GraphBasisKind::Reduce` records the basis while retaining the reducer's function identity. This is direct source evidence that `+/` is recognized as a reduction *family*.
3. **Logical IR representation:** `src/logical_ir.rs` represents `ExecutionBasisKind::Reduce` with `ExecutionBasisPayload::Reduce { axis: ReductionAxis::LeadingCellAxis }`. Its `IterationAxis` records role/extent but not named factor-variable incidences; `FactWitness` only covers prefix/frame agreement. The operation can be described as a reduction, but there is no demonstrated generalized factor-scope or elimination-order descriptor.
4. **Target lowering:** `src/lowering.rs` maps graph `Reduce` to execution `Reduce` and registers `OrderedReduction` for a CPU target. Ordered reduction is **not** permission to reassociate or parallelize it. Target feasibility and fusion legality remain distinct.

**Boundary of this trace:** `+/ 1 2 3` has only one reduced axis, so it cannot itself demonstrate alternative FAQ elimination orders. The three-factor expression `Σ_{j,k} A[i,j] B[j,k] C[k,l]` in the earlier example is mathematical pseudocode; we have **not** established a valid equivalent J source spelling or verified that RustJ's current frontend accepts and preserves its multi-factor contraction as one analyzable graph. Do not claim this example is currently executable.

**Observed information gap:** the existing reduction basis identifies a reducer and (in the logical payload) a leading-cell axis, but it does not expose a canonical set of factor axis scopes `{i,j}, {j,k}, {k,l}`, a list of free/reduced symbolic variables, or a legal variable-elimination schedule. These should be computed as **optional analysis descriptors** from actual graph structure and validated facts, rather than guessed from shape alone.

### Next executable acceptance test

- [ ] Build a frontend-to-Graph-IR-to-Logical-IR test for `+/ 1 2 3` and assert reduction identity, axis role and provenance.
- [ ] Identify a *currently supported* J expression for a two-axis contraction, then inspect its real graph and logical operations.
- [ ] Record the first transformation at which symbolic factor incidence is no longer recoverable, if any.
- [ ] Only then define and attach the optional `AggregateFoldCandidate` / `EliminationOrderCandidate` descriptor and algebraic legality obligations.

**P0.1.e status:** source trace and information-gap analysis complete; executable end-to-end trace remains open. No code or tests were run.


## Two-reduction executable regression test (pending execution)

Added [`tests/logical_ir.rs`](../../tests/logical_ir.rs) test `nested_reductions_preserve_two_distinct_reduction_operations` for `+/ (+/\"1) i.2 3`. It requests both `Engine::analyze_j_graph` and `Engine::analyze_a3`, verifies both IRs, checks graph-node provenance count and expects at least two Logical IR Reduce bases.

**Not yet verified:** parser acceptance, actual nested-reduction lowering and test pass/fail; no local Rust toolchain or CI was run. This test is a probe, not proof that the optimizer can recognize a multi-factor Einstein contraction. If the test fails, diagnose parsing versus IR decomposition rather than relaxing the semantic assertion without evidence.

- [x] Add an automated two-reduction IR preservation probe.
- [ ] Execute `cargo test --test logical_ir nested_reductions_preserve_two_distinct_reduction_operations -- --nocapture`.
- [ ] Inspect actual emitted IR, then distinguish sequential nested reductions from reorderable multi-factor contractions.
- [ ] Only after a passing trace, mark P0.1.e executable portion complete.
