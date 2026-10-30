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
