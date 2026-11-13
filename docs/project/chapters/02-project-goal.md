[한국어 정본](02-project-goal.ko.md) | [Project index](../../../PROJECT.ko.md)

<a id="read-goal"></a>
<a id="1-project-goal"></a>
## 1. Project goal

RustJ is **not** a line-by-line Rust translation of the C `jsource` implementation.

**Central mission:**

> **Make J's function composition and array algebra intelligible to a compiler, discover and validate equivalent computation forms and optimization opportunities that conventional execution might miss, and use those transformations while preserving J's observable semantics.**

RustJ is a Rust-based J array compiler/runtime to research and realize this goal. CPU and GPU remain possible long-term execution targets. This does **not** require all backends to be finished at the same time or require RustJ to outperform specialist GPU compilers in every workload.

<a id="11-mission-priorities-and-success"></a>
### 1.1 Mission, success criteria and feature-admission policy

| Category | What it means |
|---|---|
| **Differentiating research purpose** | Extract high-level computational structure from J's hook/fork/train, composition, Rank, adverbs/conjunctions, reduction and reindexing. Explore witnessed equivalent Graph IR candidates and connect selected computations to verified Logical/Physical execution. |
| **Non-negotiable requirement** | Preserve J-observable values, types, shapes/rank, empty arrays, prototype/fill, NAME/binding, effects, first errors and numerical semantics. Reject rewrites that cannot meet these obligations. |
| **First independently useful delivery** | Complete M2 frontend semantics → M3 logical/physical boundary → M4 **verified native CPU vertical slice**. It must be useful and correct for its explicitly supported J subset even without advanced optimizer passes. |
| **Optimization admission** | Start from a small J computation, retain original and rewritten graphs, establish equivalence/guards, and measure execution time and memory on real CPU workloads. Keep the original verified execution when the new path is unsupported, invalid or not beneficial. |
| **Enabling technologies** | SPORES/egg, Futhark, MLIR, StableHLO and external libraries are optional sources of research, search infrastructure or execution backends, not independent product goals. |

**Explicit non-goals:** independently reimplementing Futhark, XLA, MLIR and SPORES; adopting every paper's proposed feature; immediately matching the fastest specialized GPU kernels; or proving universal equivalence for every J primitive as prerequisites for useful releases. The aim is not the largest feature list, but **a growing set of small, fully evidenced cases in which J's algebra exposes beneficial optimization opportunities**.

**Feature-admission questions:** Before adopting a new optimizer/IR capability, ask (1) what it adds to understanding J computation structure, (2) whether existing Graph/Logical/Physical ownership already supports it, (3) whether semantic correctness and performance can be verified independently, and (4) whether existing backends can supply its implementation. Do not convert literature surveys into a mandatory RustJ roadmap. The existing [M2–M4 roadmap](11-roadmap-checklists-p01.ko.md#architecture-migration-checklist), [next tasks](18-next-tasks.ko.md), and [Futhark research](../../research/futhark-compiler-ir-prior-art.md) follow this mission.

`jsource` remains the reference implementation for:

- observable J language semantics and primitive corner cases;
- rank/cell/frame/agreement and shape behavior;
- types, errors, promotions and name/binding behavior;
- differential correctness testing and CPU implementation comparisons.

The C J engine is **not** RustJ's ordinary execution fallback.

<a id="12-logical-vs-physical-arrays"></a>
### 1.2 Core array-model decision — distinguish Logical Array and Physical Array

Logical Array owns J-visible type, shape, atom order and boxed/sparse semantics. Physical Array owns buffer, stride, offset, layout and placement. A logical value does not require a separate materialized buffer.

The detailed contract is in [§6](07-array-model.ko.md#logical-physical-array-model), with GPU placement and alias/lifetime constraints in the corresponding Korean canonical chapters.

<a id="13-naming"></a>
### 1.3 Naming policy

There is no current RustJ component named `Jaxa`. Historical `jaxa-analyzer` research has been incorporated into RustJ's semantic, graph, logical and physical design. Use the current component names, not `Jaxa` as an independent runtime or crate:

```text
Jaxa Analyzer      → J Graph Analyzer + Execution Semantic Lowering + downstream legality/resource analysis
Jaxa lowering      → J Graph IR → Logical Execution IR lowering
Jaxa optimizer     → J Graph algebraic optimizer + Execution Logical Optimizer
Jaxa physical plan → Physical Planner / Physical Plan
```
