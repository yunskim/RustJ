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
