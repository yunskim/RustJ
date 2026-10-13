# RustJ common array algebra: prior art and J-semantics mapping

> **Status:** initial literature/design review documented; formal RustJ translations, J-differential proofs and executable rewrites **NOT YET VERIFIED**. `PROJECT.ko.md` is the canonical architecture/status authority. This is a companion research checklist to [R0 access composition](common-array-access-composition.md) and [numeric policy](common-array-numeric-error-policy.md), not a new mandatory IR or a claimed universal algebra.

## 1. Revised research position

**Do not assume that Common Array Algebra must be invented from scratch.** Evaluate and reconcile existing formal systems: MoA for shape/indexing, APL's operator calculus, Remora's rank polymorphism, Bird–Meertens algebraic transformations, SPORES sum-product/relational rewrites, Futhark parallel-array compilation and RISE's guided equality saturation. Transfer definitions and proven laws only with their assumptions, then establish **independent J-specific obligations** before a rule is admitted.

| Prior art | Established scope | RustJ connection | Explicit transfer limitation |
|---|---|---|---|
| **MoA, A Mathematics of Arrays** (Lenore Marie Restifo Mullin, **1988**) | Shape-based array expressions, index function **Ψ**, higher-order dimension operator **Ω**, symbolic verification of n-D expressions and hardware-oriented applications | `StaticAxisAccess`, `pullback_region`, `value_demand_trace` coordinate relations | Does not automatically cover every J Rank/Gather/Reshape, fill/prototype or first-error behavior |
| **An Operator Calculus** (Iverson, Pesch, Schueler, **1984**) | Interactions of APL rank, composition, derivative/inverse operators and shapes of results on **empty frames** | J Rank/CellApply and derived-verb semantics | Do not assume J and historical APL operator semantics are identical |
| **Remora, The Semantics of Rank Polymorphism** (Slepak, Shivers, Manolios, **2019**) | Dynamic rank-polymorphic semantics, shape-indexed type system, progress/preservation establishing shape safety for **well-typed Remora programs** | J Graph `GraphFacts`, rank/cell/frame, prefix agreement and `CellApply` | No theorem implies all J programs are shape-safe. Check dynamic NAME, rank, empty frame/fill and error precedence separately |
| **Bird–Meertens Formalism** (1980s onward) | map/fold, list homomorphisms and algebraic derivation of parallel scan | Map/Reduce/Scan candidate identities | Need operator-domain associativity, identities and ordered J error/IEEE constraints |
| **Tensor algebra** | Axis transformations, contraction and sum-product | Tensor/NN extended-primitive contracts | Does not supply J language-level semantics by itself |
| **SPORES** (Wang et al., **2020**) | LA→RA translation, equality saturation and rewrite completeness **within its analyzed sum-product language** | GraphBasis, indexed relational representation, candidate proof/cost model | Not a completeness theorem for all J verbs, numeric domains or error behavior |
| **Futhark** (Henriksen et al., **2017**) | Bulk-parallel operators, fusion laws, flattening and referentially transparent in-place updates | Map/Reduce/Scan, fusion candidates, logical/physical separation | Pure functional semantics are not J NAME/effect/prototype semantics |
| **RISE / Guided Equality Saturation** (Koehler et al., **2021–2024**) | e-graph-guided array rewriting; sketches/guides address unguided search explosion | Optional `egg`/EqGraph candidate exploration and cost-based extraction | A found optimization is not evidence that its laws preserve Exact-J behavior |

**Evidence level:** Syracuse University's official dissertation record verifies the 1988 MoA author/date, Ψ/Ω and shape/symbolic verification, but its page exposes **an abstract only** (full manuscript behind restricted access); do not claim its entire theorem set was audited. Public Remora/SPORES/RISE/Futhark texts or scholarly abstracts support their respective scoped claims. The mapping above is **comparative analysis**, not machine-checked porting.

## 2. Minimal shared formal vocabulary (candidate, not a new IR)

Keep original Graph ValueIds and propose the following *proof specification* rather than prematurely imposing a `CommonArrayIR`:

```text
LogicalArray(dtype, shape, logical atoms, J-visible representation)
IndexedAccess(source, result, coordinate relation, shape/domain witness)
OperatorApplication(primitive or derived identity, valence, intrinsic rank,
                    cell/frame agreement, fill/prototype, errors/effects,
                    NAME version, CCT/Fit state)
CandidateLaw(source, replacement, domain assumptions, shape/index evidence,
             numerical/observational proof, provenance)
```

MoA's Ψ index expression can be compared with an output-to-input access relation; R0's axis permutation is only a *subset*. General Gather, Reshape, dynamic rank and reductions need independent proofs.

Distinguish three **non-interchangeable** evidence layers:

1. **Coordinate/shape identity** (the R0 access algebra covers proved fragments).
2. **Mathematical value identity** under an explicit algebra/domain.
3. **Exact-J observational identity**, additionally preserving dtype/rank/cell/frame, empty-frame fill/prototype, first observable error, NAME/effects, evaluation order and runtime comparison semantics.

Layer 1 or 2 **never** automatically establishes layer 3. J's `2^-44` default tolerant comparison is not an optimizer error allowance, and nontransitive tolerant equality must not be used for `egg`/EqGraph E-Class unions. Follow the [numeric error policy](common-array-numeric-error-policy.md).

## 3. Mapping to existing RustJ results

| RustJ work | Current supported claim | Prior-art use |
|---|---|---|
| R0–R0.2 axes, regions, graph transpose | Coordinate/shape pullbacks on witnessed shapes | Compare MoA indexing notation and counterexamples; no Exact-J rewrite |
| R0.3–R0.4 unary map/transpose, demand traces | Witnessed common logical value-demand propagation | Define limited indexed-access family and unsupported barriers |
| R0.5–R0.7 fanout union/use occurrences/coverage | Caller-selected logical demands can be incomplete | Specify mathematical rectangle-set unions without claiming complete *execution* demand |
| GraphFacts, GraphBasis, Rank/CellApply | Source-aware interpreted J graph facts | Translate only Remora assumptions demonstrated to match J semantics |
| j_graph_rewrite/fusion, optional egg design | Candidate research / retained graph provenance | Attach SPORES/RISE rule sources, assumptions, numerical guards, observational witnesses and cost evidence before union |

## 4. Falsifiable checklist (separate small experiments)

| ID / Test | Purpose | Guarantee / assumptions | Failure meaning | Status |
|---|---|---|---|---|
| **LIT-MOA-01** `moa_psi_static_axis_access_correspondence` | Compare Ψ projections with R0 coordinates | Scalar, zero extent, axis transpose exact shape/index mapping | Wrong index/axis correspondence | [ ] UNIMPLEMENTED / full thesis theorem audit pending |
| **LIT-RANK-02** `remora_rank_cell_frame_j_differential` | Compare Remora lifting vs J Rank/CellApply | Positive/negative rank, prefix agreement, empty-frame prototype, pinned J-C oracle | Illegitimate transfer of Remora theorem to all J | [ ] UNIMPLEMENTED / UNVERIFIED |
| **LIT-BMF-03** `fold_scan_law_has_domain_and_error_witness` | Collect map/fold/scan law candidates | Domain-specific associativity, identities, shape, dtype, numeric/error/effect evidence | Unsafe floating reassociation | [ ] UNIMPLEMENTED / UNVERIFIED |
| **LIT-SPORES-04** `sum_product_relational_rule_guard` | Translate eligible sum-product identities | Explicit schema/axis, sparsity, shape, J numeric/error conditions | Scope-limited completeness overgeneralized | [ ] UNIMPLEMENTED / UNVERIFIED |
| **LIT-RISE-05** `guided_rewrite_requires_exact_j_witness` | Gate guided e-graph rules | No EqGraph union without Exact-J proof | Treating successful search as sound rewrite | [ ] UNIMPLEMENTED / UNVERIFIED |
| **LIT-FUTHARK-06** `fusion_does_not_erase_j_obligations` | Guard fusion/flatten candidates | Preserve original Rank/NAME/first-error/fill/prototype | Allow unproved scheduling/partial evaluation | [ ] UNIMPLEMENTED / UNVERIFIED |
| **LIT-PRIM-07** Primitive law registry | Record sources and conditions per original J operation | `SourceRule → Domain → Shape/Rank → NumericPolicy → Errors/Effects → Proof/Unknown` | Unsupported rules claimed complete | [ ] UNIMPLEMENTED / UNVERIFIED |
| **LIT-SRC-08** Full MoA text audit | Verify theorem hypotheses beyond the official abstract | Clearly separate verified citations and unreviewed mathematics | Treating an abstract as a formal completeness proof | [ ] FULL TEXT NOT YET AUDITED |

**Prioritize** MoA coordinate formalization and Remora-vs-J rank semantics, then BMF/SPORES on guarded subsets, then RISE/EqGraph search. Keep pinned J-C differential tests and source provenance. For each step require separate test/Purpose/Guarantee/Failure meaning/Status, bilingual docs, exact PR-head CI and merged-main CI.

## 5. Primary bibliography

1. Lenore Marie Restifo Mullin, *A Mathematics of Arrays*, Syracuse University PhD, **1988** — [official dissertation entry/abstract](https://surface.syr.edu/eecs_etd/249/), full-text access restricted.
2. Kenneth E. Iverson, Roland Pesch, J. Henri Schueler, *An Operator Calculus*, **1984** — [ACM DOI](https://doi.org/10.1145/800058.801101).
3. Justin Slepak, Olin Shivers, Panagiotis Manolios, *The Semantics of Rank Polymorphism*, **2019** — [arXiv](https://arxiv.org/abs/1907.00509).
4. Bird–Meertens sources — [Oxford Algebra of Programming bibliography](https://www.cs.ox.ac.uk/activities/publications/bibtex/algprog.html); Sergei Gorlatch, *Extracting and implementing list homomorphisms*, **1999** — [DOI](https://doi.org/10.1016/S0167-6423(97)00014-2).
5. Yisu Remy Wang et al., *SPORES*, **2020** — [arXiv](https://arxiv.org/abs/2002.07951), [PVLDB full paper](https://www.vldb.org/pvldb/vol13/p1919-wang.pdf).
6. Troels Henriksen et al., *Futhark: Purely Functional GPU-programming with Nested Parallelism and In-place Array Updates*, **2017** — [PLDI DOI](https://doi.org/10.1145/3062341.3062354).
7. Thomas Koehler et al., *Sketch-Guided Equality Saturation*, **2021/22** — [arXiv](https://arxiv.org/abs/2111.13040); *Guided Equality Saturation*, **2024** — [POPL DOI](https://doi.org/10.1145/3632900).

This literature comparison does **not** imply formal re-proving of all cited theorems inside RustJ.
