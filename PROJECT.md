**English mirror** | [한국어 — canonical](PROJECT.ko.md)

# RustJ Integrated Project Document

> Status: **English mirror of the canonical Korean project document**
>
> Canonical source of truth: [PROJECT.ko.md](PROJECT.ko.md)
>
> Design rationale and regression guardrails: [FOUNDATIONS.md](FOUNDATIONS.md), mirrored from [FOUNDATIONS.ko.md](FOUNDATIONS.ko.md)
>
> If this English document and the Korean canonical document disagree, the Korean `*.ko.md` document governs.

RustJ is not a line-by-line Rust translation of `jsource`. Its goal is to preserve J's language and array semantics while building a modern compiler-oriented execution system in which CPU and GPU are peer targets.

The current implementation is transitional: a limited J frontend, direct CPU execution, Semantic IR, J Graph IR, early Logical IR, storage/SIMD code, sparse/boxed foundations, and experimental compiler analysis coexist while the final architecture is being made explicit.

---

### Quick guide — current priority and reading order

- **Goal and invariants:** a Rust kernel/compiler preserving full J semantics. C is the differential oracle, not the normal runtime fallback. Keep Logical Array and Physical Representation separate.
- **Current priority:** continue M2 tokenizer → enqueuer → parser convergence. Preserving graph structure/partial facts is distinct from permitting optimization/execution. Then close M3 boundaries and validate the M4 Native CPU vertical slice. Retain GPU-friendly design while deferring CUDA implementation. Open external routes incrementally where capability is proven.
- **Latest validation:** NV2 core recognition: Windows default/portable each 435 passed / 17 ignored; each C base/AVX2 runtime route 5,380 cases / zero failures. Vocabulary POS 143, bare function binding/AR 140 and three noun payloads match. Keep 257 capture-graph and two static boundaries separate; this is not full J execution support. See the NV2 gate and current validation summary.
- **Reading order:** rationale in [FOUNDATIONS.md](FOUNDATIONS.md); name/effect/route conditions in [dynamic semantic boundary contracts](#dynamic-semantic-boundaries); work and gates in the frontend/milestone checklists and validation policy. Historical gates are not current support claims. Keep the canonical design and checklists in this document pair.

## 1. Project goal

RustJ aims to implement:

> **A modern J array compiler/runtime in Rust that owns full J semantics and can compile analyzable regions to CPU, GPU, external compiler IRs, or verified library/custom-kernel routes without redefining the J language for compiler convenience.**

`jsource` remains the primary compatibility oracle for:

- observable J language semantics;
- word formation, enqueue, parser behavior, and name lookup timing;
- primitive corner cases;
- rank/cell/frame/agreement semantics;
- errors and error precedence;
- numeric promotion and tolerance;
- boxed/sparse semantics;
- reference differential testing.

The C J engine is not intended to become RustJ's normal fallback runtime.

### Reference implementations

RustJ does not treat all external implementations as having the same authority. They are used as **role-specific reference implementations**.

- **`jsoftware/jsource` — J semantic reference / oracle**
  - The authority for language semantics, parser/name behavior, primitive corner cases, rank/agreement, errors, and type semantics.
  - The primary reference for RustJ semantic correctness and differential testing.

- **ArrayFire — array execution / JIT fusion / multi-backend runtime reference**
  - A reference for lazy expression graphs, evaluation boundaries, kernel JIT fusion, CPU/CUDA/OpenCL/oneAPI backend selection, and device-memory/stream/synchronization handling.
  - Used when comparing RustJ Graph/Execution optimization, Physical Planner behavior, external-library routes, and cost models.
  - **It is not an oracle for J language semantics.**

- **`jsoftware/math_arrayfire` — J ↔ GPU-library adapter/offload reference**
  - A concrete adapter that passes J arrays into ArrayFire handles.
  - Useful for studying row-major J versus column-major ArrayFire conversion, backend capability/rank limits, external-handle lifetime, release, and device-GC boundaries.
  - It must not be interpreted as an implementation of arbitrary J rank/adverb/derived-verb semantics as a GPU compiler.

- **APEX / Co-dfns / TAIL→Futhark — array-compiler research implementations**
  - References for morphology/fact analysis, data-parallel compiler representation, high-level parallel IR, fusion, and GPU lowering.
  - Their restricted APL subsets are not inherited as restrictions on RustJ's J semantics.

- **Remora / Bohrium / Lift / MLIR Linalg — adjacent array-language / IR compiler references**
  - Remora is a comparison point for rank polymorphism, frame/cell semantics, and implicit lifting in the J/APL family. Source: https://arxiv.org/abs/1907.00509
  - Bohrium is a precedent for lazily collecting NumPy-style array operations so fusion, allocation/materialization, host-device movement, and backend-specific execution can be delayed. Do not overstate it as dynamically selecting CPU versus GPU for every operation. Publications: https://bohrium.readthedocs.io/publications.html
  - Lift is a comparison point for rewrite-driven progression from portable map/reduce patterns toward OpenCL-specific functional patterns and increasingly concrete hardware mappings. Do not describe it as a strict separation of rewriting from hardware mapping. Source: https://doi.org/10.1109/CGO.2017.7863730
  - MLIR Linalg is a comparison point for preserving structured operations and implicit iteration until later tiling/vectorization/lowering materializes loops. Source: https://mlir.llvm.org/docs/Tutorials/transform/Ch0/
  - None of these systems define RustJ's J semantics; they are evidence for compiler layering and optimization techniques.

The reference depends on the question being asked:

```text
J semantic correctness       → jsource
array graph/JIT fusion       → ArrayFire
J↔external GPU adapter       → jsoftware/math_arrayfire
array-compiler middle-end    → APEX / Co-dfns / TAIL-Futhark
rank/structured-IR comparison → Remora / Bohrium / Lift / MLIR Linalg
```

The detailed ArrayFire source observations, RustJ applications, and non-adoptions are recorded in §14.2, **ArrayFire and the J ArrayFire add-on**.

---

## 2. Target architecture

```text
J Source
   ↓
──────────── RustJ frontend ────────────
Word formation / tokenizer
   ↓
Enqueue / glyph-control-name classification
   ↓
Parser-time name/POS lookup
   ↓
J parser reductions
   ↓
J Semantic Construction IR / FunctionEntity DAG
   │
   │ nouns / verbs / adverbs / conjunctions
   │ primitive and derived entities
   │ hook / fork / train / @:
   │ rank and other modifier applications
   │ names / bindings / versions
   │ source provenance
   ↓
──────────── J Graph IR ────────────
Applied J operation graph
   │
   │ Graph Basis
   │ rewrite / equivalence candidates
   │ logical shape/type/rank facts
   │ logical liveness/resource expressions
   ↓
──────── Execution Semantic Lowering ────────
Verified Logical Execution IR / Plan
   │
   │ Execution Basis
   │ semantic checks
   │ effect / error / speculation facts
   ↓
Route Partition
   │
   ├─ RustJ-native
   │    ↓
   │  Logical Optimizer
   │    ↓
   │  Schedule / Transform Plan
   │    ↓
   │  Physical Planner / Bufferization
   │    ↓
   │  Physical Plan
   │    ↓
   │  Executor
   │
   ├─ MLIR family → LLVM / NVVM / ROCDL / SPIR-V
   ├─ StableHLO-compatible subset → external consumer
   └─ verified library / custom-kernel route
```

The semantic meaning of a J program must not depend on the selected backend.

## 2.1 Route-region boundary contract

Mixed routing is not mainly about assigning operations to backends; it is about preserving J semantics across region boundaries. RoutePartition is not a physical-buffer plan, so a boundary is first expressed as a logical/effect contract.

```text
RouteBoundary
  producer_region
  consumer_region
  live_in / live_out ValueId
  effect_order_in / effect_order_out
  anchored/crossing SemanticChecks
  required witnesses / guards
  logical representation requirements
  source / J Graph provenance
  bridge-lowering responsibility
```

`logical representation requirements` means J-visible or route-precondition information such as dense/sparse/boxed semantics, dtype, shape, and rank. Concrete BufferIds, pointers, strides/alignment, and host-device copies belong to bridge lowering or a Physical Plan.

A cut is legal only when: all live-ins/outs are explicit; effect/error dependencies survive even without value live-outs; SemanticChecks are neither dropped nor duplicated in a way that changes precedence; a legal representation bridge exists or remains uncommitted; dynamic guards run before observable effects; region-wide semantic capability is proven rather than inferred from per-op support alone; and a failed external/runtime route never causes automatic replay after producer effects have committed.

Value liveness and effect liveness are distinct. A fork/selector branch may have a dead result but still carry observable writes, I/O, or error ordering. Route partitioning must not delete such dependencies merely because pure dataflow liveness says the value is unused.

Bridge lowering happens after route partition:

```text
Logical ValueId
    -> route-boundary requirement
Bridge Lowering
    |- compatible no-op handoff
    |- materialize
    |- layout/encoding conversion
    |- host <-> device transfer
    |- external-handle wrap/unwrap
    `- synchronization/completion edge
    -> consumer-route representation
```

Bridge cost may influence route selection, but it must never change the semantic constraints merely to justify an already chosen route.

Current `lowering.rs::partition_plan` is only a v0 analysis helper: it classifies operations as `ValueOnly`, `SemanticCheck`, `PureArray`, or `RuntimeSemantic` and merges adjacent operations with the same class. Current `RouteRegion` contains only `class` and an operation range; it does not yet own a chosen external route, boundary live-ins/outs, a bridge, effect edges, or a region-wide legality proof. Do not treat it as the final mixed-route execution plan.

When mixed-route implementation starts, extend it in this order: compute/verify live-ins and live-outs; distinguish value live-out from effect live-out; preserve SemanticCheck/order edges; add region-wide legality and guard ownership; add representation-neutral BridgeRequirement; lower bridges into concrete transfers/materializations; then differential-test target-specific RoutePartitions from the same Logical IR.

---

## 3. JAXA design principle — “SQL for array operations”

Earlier JAXA documents used the phrases **“SQL for neural networks”** and **“SQL for array operations.”** RustJ carries the idea forward in the more general sense of J as a **high-level array language / array query language**.

The analogy is not a claim that J syntax resembles SQL or that full J is a purely declarative language. Full J has names, assignment, effects, observable errors, and control semantics. The relevant principle is:

> **J source is not an execution plan.**

J's array semantics and function composition should preserve **what is being computed** at a high level, while the compiler chooses **how to realize it** subject to J semantic legality.

~~~text
J source / J semantics
        ↓
J Semantic IR / J Graph IR
        ↓
Logical Array / Execution IR
        ↓
equivalence / fusion / logical optimization
        ↓
execution planning / route selection
        ↓
CPU / SIMD / multicore / GPU / external compiler / library
~~~

At the level of responsibilities, the SQL analogy is:

~~~text
SQL / relational system          RustJ
---------------------------      --------------------------------
query                             J array computation
logical query plan                J Graph + Logical Execution IR
logical rewrite                   J-algebra / logical rewrite
physical planner                  schedule / route / physical planner
execution engine                  CPU/GPU/runtime/external backend
~~~

Users should express the computation and semantic/storage obligations that matter, while the analyzer/compiler/backend decides matters such as:

- which equivalent graph form to use;
- whether to fuse or materialize;
- which execution basis and route to use;
- which memory/layout/schedule strategy to use;
- whether CPU, SIMD, multicore, GPU, or another realization is appropriate;
- whether a verified external compiler or library route should be used.

J is unusually useful as a frontend for this model because the source already carries optimization-relevant array structure:

- rank exposes cell/frame boundaries and implicit iteration domains;
- adverbs, conjunctions, and derived entities preserve reduction, scan, cell-application, and composition structure;
- hook/fork/train/@: expose producer/consumer, branch/join, and composition topology;
- reshape/transpose/take/drop allow logical shape/reindex meaning to remain separate from physical materialization;
- whole-array notation reduces the need to rediscover high-level array intent from scalar loop nests.

### Historical origin of JAXA Graph IR — read optimization topology from J notation

JAXA did not begin from the abstract goal of “building a graph compiler.” It began from a concrete observation:

~~~text
u@:v
    → input → v → u
    → producer/consumer chain
    → kernel-fusion candidate

(f g h) y
    → f and h branch from the same input and join at g
    → branch/join topology
    → branch parallelism / branch-local fusion candidate

(f g) y
    → the original input and g(y) both feed f
    → ordered dependency + input-lifetime relation
    → producer/consumer fusion candidate
~~~

J combinators such as `@:`, Hook, and Fork are therefore more than compact syntax. They expose **computation dependencies and topology without committing to a physical execution procedure**. JAXA's initial hypothesis was that a compiler should read and preserve this information before lowering it into scalar loops or backend kernels, so that fusion, parallelism, materialization, reuse, and lifetime candidates remain explicit.

That observation motivates RustJ's independent `J Graph IR` layer:

~~~text
J syntax / FunctionEntity
        ↓
syntax-derived computation topology
        ↓
J Graph IR
        ↓
candidate generation
  fusion / branch parallelism / materialization / reuse
        ↓
semantic legality
  effects / errors / names / alias / rank contracts
        ↓
profitability / resource / target choice
        ↓
Logical/Physical realization
~~~

Three questions must remain separate:

1. **Does the topology expose an optimization candidate?**
2. **Is the transformation legal under J semantics?**
3. **Is that legal strategy actually profitable on the chosen target?**

A Fork does not imply that its branches may always execute in parallel, and `@:` does not imply unconditional fusion. Source structure identifies candidates; effect/error/name/alias semantics establish legality; resource and cost models choose a realization.

The branch/join diagram describes an **ordinary VVV fork**. A constructor-fixed capped `[: g h` instead follows the sequential pipeline `input → h → g(monad)`; a noun-left fork passes `h(input)` and its fixed noun to g. Interpret constructor meaning and operand POS before assuming two executable branches from a Fork parser row/head. Preserve the original source Fork/NAME DAG.

RustJ does not claim that each ingredient is itself novel. Hook/Fork dataflow, function-level program transformation, graph-based fusion, and high-level array IR all have prior art. The distinctive architectural combination being explored by JAXA/RustJ is to **preserve J's tacit combinator algebra as an independent semantic graph layer, generate optimization candidates directly from that topology, and then separate full-J semantic legality from physical profitability**.

#### Related prior art and RustJ's position

There is direct prior art for the starting observation itself. RustJ therefore does **not** claim novelty for the general proposition that J syntax exposes optimization-relevant information. In particular, Bernecky's APL93 paper is strikingly close to the problem framing that motivated JAXA's use of `@:`, Fork, and Hook: parallel Fork arms, composition as a pipeline, and expression-level merging to reduce intermediate-array and storage overhead.

- **Robert Bernecky, _The Role of APL and J in High-performance Computation_ (APL93, 1993)**
  - explicitly observes that the `f` and `h` arms of a J tacit Fork can proceed in parallel, and argues that tacit definition simplifies data-flow/data-dependency analysis;
  - discusses expression-level **loop jamming / merging**, combining sequences of array primitives into interleaved execution, which directly anticipates temporary-elimination/fusion concerns;
  - describes J composition as a verb-to-verb **pipeline** and points out cell-level parallelism;
  - paper: https://www.snakeisland.com/aplhiperf.pdf
  - DOI: https://doi.org/10.1145/166197.166201

- **John Backus, _Can Programming Be Liberated from the von Neumann Style?_ (CACM, 1978)**
  - is an important function-level precedent for treating program-combining forms and their algebra as objects of program transformation;
  - https://research.ibm.com/publications/can-programming-be-liberated-from-the-von-neumann-style-a-functional-style-and-its-algebra-of-programs

- **Accelerate / Futhark / Lift / MLIR Linalg**
  - Accelerate and Futhark preserve high-level array operations and dependency structure for fusion and parallel lowering;
  - Lift uses the semantics of functional data-parallel patterns such as map/reduce for rewrite-rule optimization and GPU mapping;
  - MLIR Linalg preserves structured transformation-relevant semantics before loop/CFG lowering and separates transformation validity from profitability;
  - Accelerate: https://www.acceleratehs.org/publications.html
  - Futhark: https://futhark.readthedocs.io/
  - Lift: https://doi.org/10.1109/CGO.2017.7863730
  - MLIR Linalg: https://mlir.llvm.org/docs/Rationale/RationaleLinalgDialect/

The conservative novelty framing is therefore:

~~~text
J syntax exposes optimization-relevant structure
    → direct prior art exists

Fork/Composition/Rank expose parallelism or pipeline structure
    → direct J/APL prior art exists

high-level array operations are preserved for fusion/rewrite
    → prior art exists in Accelerate / Futhark / Lift / MLIR

preserve the full-J tacit combinator algebra
as an independent J Graph IR,
generate optimization candidates from that topology,
then separate full-J semantic legality
from physical profitability
    → the distinctive architectural combination explored by RustJ/JAXA
~~~

The historical JAXA question is therefore recorded as:

> **If J's function-composition notation already exposes computation topology, why destroy that intent into loops and try to rediscover it later?**

RustJ's J Graph IR is the current implementation answer to that question.

A central compiler rule follows: **do not destroy this information too early.** Modifier identity, rank boundaries, and derived structure remain in J Semantic IR/J Graph until Semantic Analyzer/Lowering can normalize them into logical operations such as `Reduce`, `CellApply`, `Scan`, and reindex forms. Explicit loops, threads, blocks, buffers, and device mappings are downstream schedule/physical choices.

Historical JAXA focused mainly on analysis and a restricted vocabulary. RustJ reuses that design work while extending it to **full-J frontend/semantic ownership with incremental optimized-backend coverage**. Analyzable array regions may use aggressive logical/physical planning, while dynamic or effectful regions can remain on semantics-preserving native/runtime routes.

The historical JAXA statements:

> **JAXA specifies logical array intent, not physical execution procedure.**

> **JAXA does not execute fusion — the compiler does.**

remain useful origin points. The more precise long-term RustJ framing is therefore not merely “J with GPU support,” but **a heterogeneous array compiler/runtime using J as a high-level array language**.

This framing does not claim that the product scope is already complete. RustJ's immediate goal remains **a J compiler/runtime that preserves full J semantics**; “SQL for array operations” is a design analogy for compiler layering and optimization freedom.

## 3.1 Core array-model decision — separate Logical Array from Physical Array

Logical Array owns J-visible type/shape/atom order and boxed/sparse semantics. Physical Array owns buffers, strides, offsets, layout and placement. A logical value need not have a distinct materialized buffer.

The model and completion gate are defined once in [§6](#logical-physical-array-model); introductory text does not duplicate its detailed structures or checklists.

## 3.2 Naming policy

`Jaxa` / `JAXA` is not the name of a current RustJ compiler component.

Historical repositories:

- `JAXA`
- `JAXA-complier`
- `japchae`
- `jaxa-analyzer`

are research/prototype sources. Their useful ideas are absorbed into RustJ's current components:

```text
Jaxa Analyzer      → J Graph analysis + Execution Semantic Lowering
Jaxa lowering      → J Graph IR → Logical Execution IR lowering
Jaxa optimizer     → J Graph algebraic optimizer + Logical Optimizer
Jaxa physical plan → Physical Planner / Physical Plan
```

New architecture decisions belong in `PROJECT.ko.md` first and are mirrored here.

---

# Part I — Frontend compatibility

## 4. Frontend principle

The frontend is not a place for RustJ-specific language simplification.

The compatibility model is:

```text
J Source
  ↓
word formation / tokenizer      ≈ current jsource w.c
  ↓
enqueuer                         ≈ jtenqueue
  ↓
parser                           ≈ current jsource p.c reduction system
  ↓
lossless FunctionEntity DAG
```

Representation may be Rust-native. Observable frontend semantics must remain J-compatible.

### 4.1 Word formation

The scanner must preserve:

- J word boundaries;
- character classes and scanner-state behavior;
- comments and quoted forms;
- numeric continuation/rewind rules;
- control words and punctuation boundaries.

Regex approximation is not the target architecture.

### 4.2 Enqueue

The enqueuer must preserve jsource-compatible behavior for:

- primitive lookup;
- noun/verb/adverb/conjunction classification;
- names and lookup metadata;
- assignment words;
- lookup timing;
- parser control classes.

Extension names remain ordinary J names and must not become tokenizer/parser keywords.

### 4.3 Parser

The parser must converge on the jsource reduction model rather than retain a separate RustJ convenience grammar.

The migration target is a single parser engine that handles, in J-compatible order:

- noun/verb application;
- adverb application;
- conjunction application;
- hook/fork/train construction;
- parentheses;
- assignment;
- name/POS lookup;
- parser-time constructor/error behavior.

Completed modifier applications are first-class J function entities before later train construction.

For example, the distinction between:

```text
Rank(Insert(+), 1)
```

and

```text
Insert(Rank(+, 1))
```

must survive parsing and downstream analysis.

### 4.4 Frontend migration checklist

Authoritative checklist: `PROJECT.ko.md`, section A0.5.

Current phase order:

```text
F0  Word formation differential compatibility
F1  Enqueuer separation / jtenqueue behavior
F2  Parse queue

P1  Parser stack/value model
P2  Unified jsource-compatible reduction engine
P3  Modifier/train construction semantics
P4  Name lookup / assignment sequencing
P5  Parser semantics vs compiler-fact separation
P6  Conformance gate
P7  Legacy convenience parser removal
P8  Handoff to compiler IR
```

The declarative class matcher is unified now. Full runtime semantic actions, modifier result-POS coverage, and name/effect sequencing remain incomplete; table matching alone does not complete the frontend.

#### Frontend file ownership

`src/tokenizer.rs` owns word formation; `src/enqueuer.rs` owns word interpretation and environment flags; `src/parser.rs` owns class matching, stack reductions, construction and parser-time name/POS resolution. `src/semantic.rs` owns semantic objects, intrinsic rank-construction contracts and binding/version models. `scanner` and the old `semantic::parse` APIs are compatibility re-exports, not duplicate grammars. No stage chooses a backend or schedule.

#### Priority and stage equivalence contract (2026-10-03)

Complete tokenizer → enqueuer → parser fidelity before other implementation work. Use the existing F0–F2/P0–P7 checklist; CUDA execution remains deferred. Rust-native representation is allowed, but semantic projections must match jsource.

| Stage | Compared projection | Evidence and limitation |
|---|---|---|
| Tokenizer | Raw word bytes, parser-visible comment cutoff, quote errors | C `;:` vs Rust spans; raw trailing `NB.` and parse count remain separate |
| Enqueuer | POS, literal type/shape/data, name/copula/lookup flags, original span/index | C value and `4!:0` for literals/primitives; source-derived `w.c::jtenqueue` goldens for control/name flags, not an exported C queue |
| Parser | First-match row, ordered completed modifier/hook/fork graph, result POS, construction errors | Read actual `p.c::cases[]` for all 9⁴ class tuples; normalize C `5!:1` and compare POS/errors. This tacit-translator table check does not prove runtime `ptcol` actions or effect sequencing |

`examples/frontend_probe.rs` serializes target-independent observations. `tools/frontend_stage_conformance.py` reports checks, mismatches, and pending coverage per stage. Atomic normalization preserves source operator identity and typed/boxed noun operands; unsupported encodings fail closed. DLL revision/hash and reviewed source revision/hash are recorded separately.

- [x] Add stage probe, source-table enumeration, literal/POS and atomic graph comparison harness.
- [x] Separate tokenizer/enqueuer/parser implementation files, preserving compatibility exports.
- [x] Preserve standalone/parenthesized adverb and conjunction results and final name assignments as `ModifierValue` with actual POS. Named modifier execution and complete derived-POS coverage remain pending.
- [x] Add projection tests preserving completed modifiers and boxed noun type/shape.
- [x] Restore `=.` enqueue metadata: top-level local copulas become global, explicit-definition enqueue keeps them local. Local body execution and locative upgrades remain pending.
- [x] Record Windows j64/AVX2 stage results: 7,014 checks each (including 6,561 declarative row tuples and 111 function/POS graphs), zero mismatches.
- [ ] Complete numeric/name/core spelling coverage, then runtime reachable-state/reinsertion traces.
- [ ] Complete constructor POS and runtime noun operands, then right-to-left name/effect/local assignment semantics.
- [ ] Pass full P6 coverage before declaring P7 frontend completion.

Run the stage harness with `--binary`, `--source-directory`, `--source-revision`, `--reference-revision`, and `--report`; set `J_LIBRARY` to the actual Windows DLL. The Windows local runner accepts the source parameters and runs the new stage gate alongside existing value/error and word-boundary checks. GitHub CI remains disabled.

Windows validation after file separation and fidelity fixes: default/portable each **212 passed / 17 ignored**, fmt/clippy passed, Python harness **17 passed**. Each ordinary/AVX2 C reference passed 2,050 sentences through both direct and semantic-reference paths, 6,618 word cases, and 7,014 stage checks with zero mismatches. The two new stage reports and six existing reports record current executable hashes. Ignored definition tests and pending runtime/parser coverage are not completion evidence. Reviewed source `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528` and official Windows DLL release metadata `ded7793fe5795d79eda8e7138dce94aa056edf78` remain distinct.

Source-grounded changes refer to [w.c](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/w.c), [sn.c::vnm](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sn.c), [wn.c::connum](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wn.c), and [p.c](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c). Malformed `foo_` and decimal fields now preserve ill-formed name/number errors with provenance. Unimplemented alternate numeric forms and valid locatives remain Unsupported.

<a id="noun-reduction-capture"></a>

#### Noun reduction and separate compilation capture — research and implementation plan (2026-10-03)

**Decision:** runtime parsing executes verb applications and reinserts actual nouns, as jsource does. An optional separate capture retains production operations and input/output dependencies. Tacit verb-oriented expression remains useful, but is not a language restriction. This plan elaborates F2/P2–P6 and does not introduce another canonical IR or competing roadmap.

**Remaining gap:** runtime rows 0–2 now invoke a host and reduce actual nouns; static contexts retain application Expr structure. Capture v0 retains source operations and occurrence edges separately; its adapter now maps successful observations to the existing J Graph. Top-level single-name non-final assignments are implemented; explicit-local/locale/definition/effect and full modifier POS remain incomplete. Arbitrary fork executor coverage is separate from frontend construction support.

##### Framework/language comparison

Official documentation and source were reviewed on 2026-10-03. Moving `main`/`stable`/nightly references are research sources, not RustJ's pinned compatibility oracle. The application column is our design inference.

| Reference | How it handles values and structure | RustJ application/limit | Sources |
|---|---|---|---|
| PyTorch `make_fx` / ProxyTensor | Real tracing executes tensor operations while recording graph nodes; `proxy_call` creates a proxy, invokes the operation and associates returned tensors through `track_tensor_tree`. Fake tracing is a separate mode | Closest model for concrete noun plus separate origin. Also retain J modifier/train identity rather than only tensor primitives | [make_fx](https://docs.pytorch.org/docs/stable/generated/torch.fx.experimental.proxy_tensor.make_fx.html), [ProxyTensor source](https://github.com/pytorch/pytorch/blob/main/torch/fx/experimental/proxy_tensor.py) |
| FX symbolic tracing / Dynamo | FX proxies record symbolic operations with dynamic-control limitations. Dynamo captures graphs plus residual code and validity guards, using graph breaks around unsupported code | Make unknown noun/POS/effect dependencies explicit. Graph breaks require an actually supported RustJ runtime; there is no guaranteed full-J fallback | [FX](https://docs.pytorch.org/docs/stable/fx.html), [Dynamo](https://docs.pytorch.org/docs/stable/user_guide/torch_compiler/compile/programming_model.dynamo_core_concepts.html) |
| JAX | Tracers record jaxpr; abstract tracers lack array contents. Static/concrete values differ from traced values, and ordinary Python effects are not automatically recorded | Reference for later static compilation; do not use abstract shape/dtype to satisfy value-dependent J constructors or lose effects on repeated execution | [Tracing](https://docs.jax.dev/en/latest/tracing.html), [JIT/effects](https://docs.jax.dev/en/latest/jit-compilation.html) |
| TensorFlow `tf.function` | Host Python runs during tracing, tensor operations enter a graph; supported control flow is transformed by AutoGraph. Host and graph-runtime effects differ | Explicit staging boundaries; do not execute J-visible effects once during tracing and omit them from subsequent calls | [tf.function](https://www.tensorflow.org/guide/function) |
| ArrayFire / Eigen | ArrayFire accumulates supported elementwise ASTs until explicit evaluation or a non-JIT consumer. Eigen uses lazy expressions with alias/cost-dependent temporaries | Reference for pure-array fusion, not permission to make every J parser noun lazy or postpone its errors | [ArrayFire](https://arrayfire.org/docs/jit.htm), [Eigen](https://libeigen.gitlab.io/eigen/docs-nightly/TopicLazyEvaluation.html) |
| Julia | Compiler SSA IR preserves instruction/results/control flow, without implying ordinary runtime values retain histories | Reference for subsequent SSA lowering after J semantic resolution; SSA alone does not solve dynamic J parsing | [Julia SSA IR](https://docs.julialang.org/en/v1/devdocs/ssair/) |

Runtime parsing remains grounded in [jsource p.c](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c). Lazy execution is a later optimization for proven regions, not the baseline parser replacement.

##### Ownership and capture contract

For `a+b*c`, runtime reductions yield actual nouns while capture retains `v1=Apply(*,b_read,c_read)` and `v2=Apply(+,a_read,v1)`. These are observation ids, not addresses.

- Parser noun carriers hold actual `Value` plus optional origin. Recording cannot change POS/class matching/errors; disabled recording allocates no graph or retained array history.
- Proposed `CompilerCapture` records application occurrences, input/output ids, shared `FunctionEntity`, valence, original spans/word indices, observed type/shape, sequence/effect dependencies, and outcomes. It is a sidecar input to existing J Graph, not a new execution IR. Intermediate arrays are not copied into every node; external nouns are input slots, literal constants have intentional immutable ownership, and constructor-selected concrete values carry explicit dependencies/guards.
- Equal values need not share producers: `2*3` and `1+5` produce distinct occurrences. Neither value equality nor storage pointers define identity. Map proposed `CaptureValueId` explicitly to J Graph `ValueId`; keep J names/versions and Physical `BufferId` distinct.
- Rows 3–6 retain completed `/`, rank, hook/fork entities and operand order. A computed constructor noun is validated as an actual value, with its origin separately linked to the operand occurrence. Observed argument/binding facts are reuse witnesses, not new intrinsic function fields.
- Resolve noun reads at actual queue-to-stack entry and record binding versions. Preserve NameRef expected POS and late binding; an observed named target is not automatically a constant. Workspace values with unavailable history are external inputs. Cross-sentence capture requires explicit scope/version tracking.
- Record attempts before invocation, outputs after success, and unchanged ErrorKind/ErrorContext on failure. Partial capture is not an executable complete plan. Recording failures do not overwrite J errors, retry effects, roll back effects that J already performed, or commit an unsuccessful outer assignment.
- Derive/augment `j_graph_ir::Plan`, verify data/effect/error dependencies and use existing lowering. `logical_ir::Plan` remains canonical execution IR. No target/device/schedule decisions enter the parser.

##### Execution, staging, reuse and costs

Capture-on/off uses the same class matcher and semantic row actions. Proposed `Engine::eval_with_capture(&mut self, source)` executes once and returns an outcome/capture report, including partial failure information. It is distinct from read-only `prepare_semantic/analyze_j_graph(&self, ...)`; analysis must not silently run IO/assignments. A successful trace must not require a second execution.

The default non-executing static path may use abstract actions of the same row engine for proven pure/static forms. Concrete-value constructors, unknown bindings/POS and effect/error boundaries require explicit dependencies and supported runtime/residual regions. No-execution AOT rejects unknown dependencies or represents residual execution; it does not resolve them by secretly running the program.

An observed path is not a universal program. Separate input dependencies, constants and validity guards for shape, bindings/POS, environment and constructor values. Invalid reuse falls back safely or recaptures without restarting already-performed effects. v0 capture supports inspection and does not enable unguarded compiled replay. Execute-and-capture still pays for the first array computation; small arena metadata and shared function references must avoid retaining all temporaries. Static/reuse/fusion is subsequent performance work; CUDA remains deferred.

##### Frontend information preserved for future optimization (2026-10-03)

The goal is a **J tokenizer/enqueuer/parser usable for analysis and optimization before execution**, rather than adopting SQL implementation techniques. Current priority is information preservation and logical compatibility with jsource across these three stages. This change performs no optimization transform, execution reordering, or kernel selection.

`static_analysis::StaticAnalyzer` accepts input-name noun/function POS and `GraphFacts`, connecting the existing tokenizer → enqueuer → parser → bind → J Graph path without execution. Parser `AbstractNoun` classifies an analysis-only noun, not an actual `Value`, and is rejected in concrete execution. Expressions containing intermediate nouns retain operation structure without allocating input arrays or calling named functions. Literal constants still construct their existing payloads; this is not a claim of zero allocations.

- [x] Regressions declare trillion-element inputs using metadata and preserve distinct operand structures for `x+y*z`, `(x+y)*z`, and fork regions.
- [x] A `SourceWord` sidecar retains tokenizer spans and enqueue POS, original word indices, name-lookup/copula flags. This provides source provenance alongside graph spans; `ParseReduction` connects supported reductions to operand word ranges and result origins. Unsupported semantic-action runtime traces remain pending.
- [x] Unknown shapes stay Unknown, named functions retain specialization boundaries, and missing POS/value-dependent unsupported constructors produce analysis-coverage errors.
- [x] Assignments retain proposed graph writes without mutating the catalog. A regression confirms analysis does not execute a runtime domain-erroring call. Successful analysis does not prove runtime error freedom.
- [x] `examples/static_explain.rs` exposes graph/logical-memory information without input data. Catalog versions are not runtime guards and reports are not executable compiled plans.
- [x] Resolve names at right-to-left stack entry and propagate supported reduction provenance/final copulas.
- [x] Implement supported concrete noun reductions, separate capture and top-level single-name non-final assignment.
- [ ] Extend unsupported constructors, explicit-local/locale/definition scope and effect coverage under F2/P2–P6.
- [ ] After frontend verification, connect effect/error-order proofs, optimization transforms, lowering, and execution as separate stages.

Logical extent/liveness/resource reports reuse existing analysis without optimizing. The logical atom total is not peak allocation. This does not claim full J, upstream-suite or CUDA coverage. Windows validation: Rust default/portable each 228 passed, 17 ignored; fmt/clippy passed; Python harness 18 passed; j64/AVX2 each examined 2,063 direct and semantic-reference sentences: 2,061 passed, two runtime coverage boundaries and zero failures. All 7,014 stage checks and 6,618 word cases passed. Seven new static-frontend regressions passed, as did the metadata-only 10^12-element example. C DLL release metadata is `ded7793fe5795d79eda8e7138dce94aa056edf78`; reviewed source pin is `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`. The DLL is not claimed to have been built from that source pin. Verify new static-frontend regressions together with existing Windows default/portable and C j64/AVX2 frontend comparisons.

##### Runtime noun reduction and separate capture v0 (2026-10-03)

- [x] P2/P4: `RuntimeParserHost` supplies stack-entry lookup and rows 0–2 invocation through the same nine-row parser for supported forms. Each application immediately reduces to an actual `Value` and re-enters the stack. Final nouns do not replay computation. Analysis contexts have no execution host.
- [x] P2/P5: `Engine::eval_captured` returns optional `ParseCapture` observations. Capture on/off uses the same parser/kernels, recording input occurrences, source FunctionEntity apply attempts, success facts or failure kind/context, constructor noun-input associations, and final commits. Failed evaluation retains partial capture.
- [x] P3/P5: runtime constructors accept computed nouns in `f=:+"(1+0)` and `f=:(1+2) + *`, linking producer occurrences to constructor inputs. Arbitrary fork-call executor coverage and static computed constructors remain separate boundaries; no fake constants.
- [x] Regressions cover capture parity, preserved bindings/versions after failure, parenthesis identity, facts-only large reads, invocation before later lookup, and final Literal results.
- [x] P5/P8: `j_graph_ir::Plan::from_capture` maps successful captures to verified existing J Graphs. `CapturedGraph` sidecars retain occurrence→ValueId associations, computed constructor inputs and observed facts. Failed captures cannot become completed graphs.
- [x] P4 subset: execute top-level single-name non-final assignment at row 7, preserving later stack-entry lookup, RHS POS and committed early writes after later errors.
- [ ] P4: extend explicit-local/locale/definition/effect and full constructor/result-POS coverage. The 17 ignored definition acceptance tests remain unimplemented.

`parser_capture.rs` is an observation log, not a replacement canonical IR. Input/intermediate snapshots are omitted; dtype/shape, source spans and occurrence edges are retained. Shared FunctionEntity objects own intrinsic noun operands required by J semantics, so capture may extend their lifetime without copying payloads. This is distinct from buffers/physical scheduling. An observed execution does not prove compiled replay legal without purity/binding/value/error guards. Runtime capture computes actual values; the separate static analyzer never invokes them.


**Capture-adapter scope:** capture owns read-only source text. Literals are reconstructed from enqueue payloads; named nouns become ReadNoun nodes with observed binding versions without rereading current workspace values. Applications use the existing Builder, preserving FunctionEntity and NameRef. Inferred graph facts remain distinct from observed runtime facts; successful named calls still require specialization. `ConstructorOrigin.noun_inputs` preserves computed operand dependencies. Ordinary graph-memory analysis alone must not be treated as complete constructor-operand liveness or physical peak analysis without those sidecars. Multiple effects, runtime guards, failure continuation, modifier-value graph lowering and reusable executable plans remain outside this gate.

**Parser-time assignment scope:** following `p.c` row 7 (reviewed source pin above), `x+(x=:2)` commits the RHS before the later left-name lookup. Chained assignments and parenthesized assignments use the same matcher; top-level `=.` is enqueued as global when no explicit local scope is active. A failure before outer row 7 leaves its binding untouched; any assignment already performed remains visible. Runtime final assignment also commits at row 7, rather than after successful parser exit: `(x=:2` and `x=:2)` report syntax errors but retain `x=2`, matching C. A final-assignment reduction ends row processing before exit validation; the runtime does not commit it again. Array RHS buffers become shared before making a returned alias; capture records only occurrence/function identity, actual POS, copula provenance, previous/proposed binding versions and whether assignment is final. No input/intermediate array snapshots are added. The static path rejects non-final assignments without executing them. General locales, explicit local environments and noun/multiple assignment targets remain unsupported.

- [x] P4/P5: intermediate noun, verb, adverb and conjunction writes preserve actual result POS/FunctionEntity; capture verifier checks RHS identity/class, copula scope, Engine-local version progression and final-event ordering.
- [x] P4/P6: unmatched controls do not preempt reachable assignment actions. Precise control spans remain in SyntaxError diagnostics without replacing earlier runtime error classes. C observation of `a=:missing + )` confirms a hook assignment survives the exit error; the old blanket rollback test is corrected.
- [x] P5/P8: terminal enqueue/parse/runtime failure kind/context is retained separately, so a partial capture lacking ApplyFailure cannot masquerade as a completed graph. The differential harness removes stale repro files after a successful full-prefix comparison.

- [x] P3/P4/P5: runtime named primitive modifiers resolve at row 3/4 construction with expected POS checks, producing the actual completed FunctionEntity. Ordinary modifier aliases snapshot the actual modifier at row 7; later rebinding the original name does not change that alias. This differs from ordinary verb aliases, whose late references remain intact. C `5!:1`/call observations confirm `adv=:/; f=:+adv; adv=:1` leaves `f` as the completed insert and `f i.3` returns 3. Named verbs remain late references. `ModifierResolved`/`CapturedGraph.modifier_bindings` retain all modifier assignment/construction-time name/version/expected-POS witnesses and resolution rows and source-use spans; the graph records those dependencies without treating them as executable reuse guards. Operand-free primitive modifier aliases are supported; arbitrary derived modifier executors and unknown static modifier actions remain separate coverage work.

**Ordered-effect graph boundary:** capture verification covers these new runtime sentences, but the existing J Graph adapter has one final write slot. It explicitly rejects captures containing non-final writes, including `(x=:2)` whose commit happens to be the last event. Standalone modifier-value lowering is also an explicit graph boundary. The parser-capture differential report separately lists exact sources and reasons under `graph_coverage_boundaries`; this never waives value/error mismatches. Ordered write/read/effect IR support and replay legality remain later work, not an optimization introduced here.

Windows validation: default/portable each **250 passed / 17 ignored**, fmt/clippy passed, Python 20 passed. Each j64/AVX2 variant examined **2,139 sentences across direct, semantic-reference and parser-capture: 2,137 passed, two explicit runtime boundaries, zero failures**; 7,014 stage checks and 6,618 word cases passed. `examples/capture_probe.rs` validates associations and attempt/outcome ordering for every sentence, and J Graph adaptation/verification for successful sentences except thirteen explicitly reported graph boundaries (nine ordered-effect, four modifier-value). The C oracle uses C word formation to distinguish outer copulas from inner copulas/literals/comments; two harness regressions protect this distinction. Save the parser-capture JSON reports. Preserve the existing DLL/source pin distinction and skip GitHub CI.

##### Static modifier construction and analysis dependencies (2026-10-03)

- [x] P3: remove the guess that applying a modifier with known input POS produces a Verb. `declare_function(name, Adverb/Conjunction)` declares POS only; construction using it returns an explicit `Unsupported` boundary with the current name and source span. Standalone modifier POS observation is distinct from applying it.
- [x] P3/P5: `StaticAnalyzer::declare_primitive_modifier` declares actual core modifier semantics. `Engine::prepare_semantic/analyze_j_graph` reads operand-free primitive modifiers and their aliases from the current workspace and uses the same row 3/4 constructors. Known `/`, `\`, `"`, `@:` and other core identities follow supported constructor operand legality/result POS/error rules; this does not establish arbitrary derived/extension modifier coverage.
- [x] P5: `Program.modifier_snapshots` and J Graph schema **0.4** `Plan.modifier_snapshots` retain name, catalog/Engine-local version, expected POS, shared FunctionEntity and current use span. Verification checks source-use spans, names, versions and primitive modifier POS. Keep dependencies outside intrinsic function identity and physical allocation information. Runtime capture `ModifierResolved` observations and static binding assumptions occupy distinct sidecars; neither is an executable reuse guard.
- [x] P6: analyze large arrays from metadata without running reduction kernels. A known modifier alias keeps its meaning after the original name is rebound; rebinding the alias changes subsequent analysis versions/construction while existing graphs remain unchanged. Ordinary verb names retain late NameRefs. Final assignments remain pending proposals and do not mutate the workspace/catalog.
- [x] P3/P6: add separate setup/read-only analysis modes to the Windows stage probe. Compare construction, POS and domain/length errors with C `5!:1`/`4!:0`; check target versions and modifier dependencies after analysis. `candidate=: + analysisrank (#1 2)` succeeds when C executes it, but static construction needs a concrete noun, so retain its exact source/reason in `analysis_coverage_boundaries`, outside successful checks.
- [ ] Extend actual arbitrary derived modifier semantics/result POS, explicit-local/locale/definition scope, ordered-effect graphs and reuse guards. Do not fake noun values to pass value-dependent constructors. This step extends tokenizer/enqueuer/parser analysis coverage without implementing optimization transformations.

Windows validation: default/portable each **256 passed / 17 ignored**, fmt/clippy passed, Python harness **20 passed**. Each j64/AVX2 variant examines **2,139 sentences through direct, semantic-reference and parser-capture: 2,137 passed, two runtime boundaries, zero failures**; all **7,056 stage checks** (including 42 new static setup/construction/error/read-only/dependency checks) and **6,618 word cases** pass. One static value-dependent boundary and thirteen existing capture graph boundaries are reported separately from successful checks. Add six static-analysis regressions. Preserve the DLL release/source review pin distinction, unfinished definition tests and GitHub CI exclusion.

##### Modifier train structure and resulting POS (2026-10-03)

- [x] P2/P3: following `p.c` row 6, choose a trident when the third operand is CAVN, otherwise a bident. Nonzero action dispositions in `cf.c::bidents[]/tridents[]` construct an actual Adverb/Conjunction without applying it. Keep SyntaxError and immediate semantic application distinct; an unimplemented immediate application does not become a successful structural approximation.
- [x] P3/P5: `FunctionHead::ModifierTrain` represents the semantic bident/trident identified by C `CADVF`. Preserve arity 2/3, ordered noun/function operands, actual result POS, spans and completed child DAGs. Compare against C `5!:1` representation `4`, distinct from verb Hook `2` and Fork `3`. Support `+"`, `"1`, `/\`, `@:/`, `/ / /`, `/ / +` and nested trains. Execution pointers/helper slots do not become semantic operands.
- [x] P2/P6: provenance/capture inherits the left token for a bident and middle token for a non-fork trident, retaining two/three inputs. Runtime computed nouns contribute actual values; `ConstructionAttempt.noun_inputs` links their producing occurrences. Named arrays obey noun by-value semantics and remain unchanged after rebinding the original name. Static analysis rejects computed noun dependencies without executing them.
- [x] P6: extend C atomic/POS comparisons by 20 sentences and add two SyntaxError cases. Extend runtime value/error coverage with construction/failure/array rebinding cases. Verify four analysis boundaries (one existing computed rank, three derived modifier applications) as exact-source C successes/Rust `Unsupported`, outside successful checks.
- [ ] Implement derived modifier application, named derived modifier lookup/alias construction, row 6 immediate semantic application and full locale/explicit-local/definition scopes. Preserving a standalone modifier in Semantic IR is separate from executable J Graph lowering. Compiler analysis/executors do not guess that a new train is a verb Hook/Fork or known primitive. This step implements neither optimization nor CUDA.

Windows validation: default/portable each **261 passed / 17 ignored**, fmt/clippy passed, Python harness **20 passed**. Each j64/AVX2 variant examines **2,165 sentences through direct, semantic-reference and parser-capture: 2,163 passed, two runtime boundaries, zero failures**; all **7,098 stage checks** and **6,618 word cases** pass. Add five regression tests. Report 34 capture graph boundaries (nine ordered-effect, 25 modifier-value) and four static-analysis boundaries separately. New modifier Semantic IR construction does not complete executable modifier-value graph lowering. Definition tests remain unfinished, the upstream suite was not run and GitHub CI remains excluded.

Implementation references: [p.c row 6](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c#L1057), [cf.c disposition tables and jthook](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L288). The Windows DLL release pin and source review pin remain distinct; no claim that the DLL was built from this reviewed source.

##### Right-bound modifier bident application (2026-10-03)

- [x] P3: follow C `cf.c::tcNV` by passing verb input `u (C n/v)` to the actual constructor for `u C n/v`. Support core `"`/`@:` conjunctions with noun/verb right operands. `+ ("1)` returns a completed Rank entity; rank validation occurs at application rather than train definition. Other train applications do not receive a guessed Verb result.
- [x] P4/P5: runtime named derived modifier lookup and row 7 alias assignment share immutable train objects. Rebinding the original name does not change an existing alias/constructed verb. Engine static lookup and J Graph verification accept known train identities while preserving name/version/POS/current-use spans in `modifier_snapshots`. Known identity does not guarantee all applications are implemented. POS-only unknown modifier boundaries remain intact.
- [x] P5/P6: completed results are Rank/Atop entities as in C. Keep inline row 6/row 3 provenance, capture construction records and named train shared identity/version witnesses separately. Anchor bound operands at current application uses without mutating original train objects. Train nouns use shared storage to avoid copying large payloads during construction. Sidecars are not executable cache guards.
- [x] P6: test alias changes, read-only target proposals, construction-time domain/length errors and capture witnesses. Analyze metadata-only 10^12-element `- ("1) x` without kernels. Extend C `5!:1`/`4!:0` function/POS comparisons and runtime value/error coverage.
- [ ] Implement left-bound `tNVc`, noun-input adverbs, successive adverb/derived conjunction/trident action semantics. Later rank/Atop stages support execution of `(+ ("-)) i.4` and `(+ (@:-)) i.4`. Remove their runtime waivers from the current conformance corpus without claiming complete modifier semantics. Row 6 immediate applications and locale/explicit-local/definition scopes remain pending.

Windows validation: default/portable each **266 passed / 17 ignored**, fmt/clippy passed, Python harness **20 passed**. Each j64/AVX2 variant examines **2,185 sentences through direct, semantic-reference and parser-capture: 2,181 passed, four runtime boundaries, zero failures**; all **7,128 stage checks** (30 new static checks) and **6,618 word cases** pass. Add five regression tests. Report 38 capture graph boundaries (nine ordered-effect, 29 modifier-value) and three static-analysis boundaries separately. This does not establish full J/upstream-suite coverage, optimization/CUDA execution or GitHub CI. Preserve the DLL release/source review pin distinction.

Source references: [cf.c tcNV and other modifier train actions](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L200), [p.c modifier application](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c#L969).

##### Left binding and successive adverb application (2026-10-03)

- [x] P3: follow `cf.c::tNVc` by passing `u (v C)` to the actual constructor for `v C u`. Handle noun inputs used as rank specifications, such as `1 (-")`. Row 3 receives N/V inputs and reinserts an `Item` carrying actual result POS into the same parser stack. Static computed nouns remain explicit boundaries rather than fake constants.
- [x] P3/P5: `taAV` for `(A A)`/`(A V)` applies the first adverb, then dispatches its actual result and the second operand as a bident. `taaa` for `/ / /` applies f→g→h. Preserve nested Insert in `+ (/ /)`, Hook in `+ (/ +)` and three Insert stages in `+ (/ / /)` as immutable completed DAGs, without summary booleans or collapsing to a single primitive. Apply the existing recursion depth limit.
- [x] P4/P6: verify left-bound alias snapshots, noun-input rank/length/domain errors, unchanged targets on failure, and computed noun occurrence links to row 3. Application spans refer to current syntax without modifying old train definitions. Supported static regions construct function structure without kernels.
- [x] P3/P6: do not confuse `(A C)` `tac` with `(A A/V)` `taAV`. `tac` needs the original input in addition to the first application result and remains unsupported. `3 (/@:)` fails first with `/` DomainError; valid `+ (/@:)` returns `Unsupported` rather than an incorrect Adverb result. Stage reports retain C success/actual POS and exact source.
- [ ] Implement noun-left rank/gerund, `tac`, derived conjunction/other trident actions, dynamic lookup/effect contracts for late modifier NameRefs inside trains and row 6 immediate application. Executing constructed nested Insert/Hook remains separate kernel/executor coverage. Locale/explicit-local/definition scopes remain incomplete.

Windows validation: default/portable each **270 passed / 17 ignored**, fmt/clippy passed, Python harness **20 passed**. Each j64/AVX2 variant examines **2,205 sentences through direct, semantic-reference and parser-capture: 2,201 passed, four existing runtime boundaries, zero failures**; all **7,168 stage checks** (40 new static checks) and **6,618 word cases** pass. Add four regression tests. Report 40 capture graph boundaries (nine ordered-effect, 31 modifier-value) and three static-analysis boundaries separately from successful checks. No upstream-suite, optimization/CUDA or GitHub CI execution claim. Preserve the DLL release/source review pin distinction.

References: [cf.c taAV/tNVc/tac/taaa](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L200), [p.c modifier application and reinsertion](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c#L969).

##### Adverbial hook and derived conjunction application (2026-10-03)

- [x] P3: follow `cf.c::tac`: `(A C)` first constructs `t=u A`, then supplies the original input to `t C u`. Verify that Insert and Atop in `+ (/@:)` reference the same input function object. This resolves the `tac` boundary recorded in the preceding milestone.
- [x] P3/P5: `tca` applies A to the result of `u C v`; `tcc` applies the first and second C to the same original inputs in order, then constructs a bident from their results. `taav` applies the two A operands to left/right inputs and constructs a trident with fixed V. Actual result POS determines hook, insert, fork or modifier construction; row 4 reinserts the completed `Item` into the parser stack. Left/right binding of known derived conjunctions uses this path too.
- [x] P4/P5: convert repeatedly used concrete nouns from Owned to Shared storage without copying payloads, including nouns inside Groups. Verify the original payload pointer, shared identity and lifetime after dropping the original for 65,536 integers. Deferred expressions are not executed. Keep function DAGs and prior definitions immutable, preserving current-use spans and named alias snapshots. Binding/version observations are not reuse authorization.
- [x] P6: compare C function representations and actual POS, first-error ordering that prevents subsequent actions, unchanged assignment targets after failure and alias identity after rebinding the original conjunction. Add five regression tests, 16 C comparison sentences and 36 static stage checks.
- [ ] Connect noun-left rank/gerund, other derived conjunction tridents (`tcVCc`, etc.), late modifier NameRef lookup/effect contracts inside trains and immediate noun execution in bident/trident actions. Constructed-function kernel execution coverage and locale/explicit-local/definition scopes remain separate incomplete items. Do not implement optimization yet.

Windows validation: default/portable each **275 passed / 17 ignored**, fmt/clippy passed, Python harness **20 passed**. Each j64/AVX2 variant examines **2,221 sentences through direct, semantic-reference and parser-capture: 2,217 passed, four existing runtime boundaries, zero failures**; **7,204 stage checks** and **6,618 word cases** pass. Report 42 capture graph boundaries (nine ordered-effect, 33 modifier-value) separately from successes. Two static-analysis boundaries remain: computed rank and `tcVCc` in `candidate=: + (@: + @:) -`. Resolve the prior milestone's `tac` and derived conjunction bident boundaries, and newly report actual C success/POS versus Rust Unsupported for another trident. The 9⁴ stage table comparison checks declarative eligibility rather than proving a complete parser action trace. Preserve the C DLL release/source review pin distinction; no full J/upstream-suite, CUDA or GitHub CI execution claim.

References: [cf.c tac/tca/tcc/taav and modifier train dispatch](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L200), [p.c conjunction application](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c#L969).

##### Modifier trident action expansion (2026-10-03)

- [x] P3/P5: connect `cf.c` actions `tNVvc`, `tcVCc`, `tcaa`, `tNVca`, `tNVcc`, `taVCNV`, `taca`, `tacc`, `tcVCNV` and `tcca`; integrate existing `taaa`/`taav` into the same trident dispatch. Preserve each action's intermediate results, original inputs, fixed operands and application order. Actual POS drives bident/trident construction while keeping immutable function DAGs.
- [x] P4/P6: verify shared original function inputs across `tcVCc` branches and first-constructor errors preventing subsequent actions/assignment. Named trident conjunction aliases retain identity after rebinding the original name. Intermediate noun execution and late modifier lookup inside trains remain separate boundaries.
- [x] P6: compare 68 sentences covering 20 derived trident POS productions using `/`, `@:`, `+`, `3` samples and noun/verb input combinations against C. These samples do not prove compatibility for every primitive/effect combination. Add three regression tests, 98 comparison sentences and 154 stage checks.
- [ ] Next: noun-left rank construction, immediate noun execution/capture, late modifier lookup and locale/definition scopes. The preceding `tcVCc` boundary is resolved. Keep tokenizer/enqueuer/parser first and defer optimization/CUDA.

Windows default/portable each **278 passed / 17 ignored**, fmt/clippy passed, Python **20 passed**. Each j64/AVX2 variant across three execution paths: **2,319 sentences, 2,315 passed, four runtime boundaries, zero failures**; **7,358 stage checks**, **6,618 word cases** passed. Report 44 capture graph boundaries (nine ordered-effect, 35 modifier-value) separately from successes. Two static boundaries are computed rank and noun-left rank in `candidate=: 3 (" /) 1`, replacing the prior `tcVCc` boundary with the latter. Preserve DLL release/source review pins, subset scope and actual validation coverage. Skip GitHub CI.

Reference: [cf.c modifier trident actions](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L200).

##### Noun-left rank construction and operand roles (2026-10-03)

- [x] P3/P5: construct ordinary noun-left rank following `cr.c::jtqq`. Audit the right rank's rank→length→numeric validity first; preserve left noun/verb and right noun/verb order, values and spans in an immutable Rank entity. Share noun storage. Derived modifier binding/trident applications use the same path.
- [x] P4/P6: associate runtime-computed `(i.4)` and rank noun occurrences with construction capture. Static analysis does not execute computed nouns and retains explicit boundaries. Failures preserve the existing assignment target/version.
- [x] P5/P6: for `3"+`, do not mistake right `+` for the executed left function or left `3` for a rank specification. Rank graph forms and shape/type inference require the supported left-function form; noun-left forms preserve structure as Modifier with unknown facts. Add five regression tests, 17 C comparison sentences and 21 stage checks.
- [ ] Connect boxed rank-1 noun gerund auditing (except all-maximum right ranks), constant-rank kernel execution/Logical lowering, immediate noun execution, late modifier lookup and locale/definition scopes. Plain noun-left constructor support does not imply all gerund/execution support. Defer optimization/CUDA.

Windows default/portable each **283 passed / 17 ignored**, fmt/clippy passed, Python **20 passed**. Each j64/AVX2 variant through direct, semantic-reference and parser-capture: **2,336 sentences, 2,332 passed, four runtime boundaries, zero failures**; **7,379 stage checks**, **6,618 word cases** passed. Separately report 44 capture graph boundaries (nine ordered-effect, 35 modifier-value); the static report now lists one computed-rank boundary. This does not remove unsupported gerund/other computed-noun regions. Add `cr.c` to stage source hashes to identify the actual reviewed rank-constructor source. Skip GitHub CI.

Reference: [cr.c jtqq](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c#L731).

##### Primitive gerund construction and audit order (2026-10-03)

- [x] P3/P6: follow `ap.c::jtbslash`→`cg.c::jtfxeachv(1)` for noun `\` application: audit rank→length→boxed type. Empty rank-2 inputs produce RankError, empty rank-1 inputs LengthError and nonempty nonboxed inputs DomainError. Resolve the previous blanket Unsupported within this audit scope.
- [x] P3/P5: audit `r.c::jtfx` character primitive leaves through the current core PrimitiveResolver, checking char rank→length→ASCII spelling and final verb POS. Construct primitive-string gerunds as completed PrefixInfix Verbs preserving original noun operands, shared storage and spans. Do not add execution-only decoded fgh as semantic children.
- [x] P3/P6: use the same audit for noun-left rank gerund candidates. Suppress definite J audit failures and retain the constant noun as `cr.c` does; do not suppress implementation boundaries for names/unregistered primitives/compound ARs. Verify gerund element order, first errors, unchanged target/version on failure and constant fallback.
- [x] P6: add three regression tests, 38 C comparison sentences and three stage error checks. Add `ap.c`, `cg.c`, `r.c` source hashes. Report static computed-gerund noun boundaries separately from runtime constructor successes.
- [ ] Next: name/compound atomic representation fx decoding and binding, gerund execution/Logical lowering, intermediate noun execution/capture, late modifier lookup and locale/definition scopes. This is not full gerund support or optimizer/CUDA implementation.

Windows default/portable each **286 passed / 17 ignored**, fmt/clippy passed, Python **20 passed**. Each j64/AVX2 variant across three execution paths: **2,374 sentences, 2,370 passed, four existing runtime boundaries, zero failures**; **7,382 stage checks**, **6,618 word cases** passed. Separately report 44 capture graph boundaries (nine ordered-effect, 35 modifier-value). Two static report boundaries are computed rank and computed gerund nouns, counted as neither passes nor C semantic mismatches. No full upstream-suite or GitHub CI execution claim.

References: [ap.c jtbslash](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ap.c#L940), [cg.c fxeachv](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cg.c#L101), [r.c fxchar/fx](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/r.c#L77).

##### Compound gerund atomic representation decoding (2026-10-03)

- [x] P3/P5: recursively decode boxed ARs following `r.c::jtfx`: single primitive heads, noun `0`, hook `2`, fork `3`, modifier `4`, primitive adverb/conjunction application and derived modifier application with boxed heads. Reuse parser bident/trident dispositions and constructor actions to return actual POS. Do not invent a new J grammar or modifier-specific semantic AST.
- [x] P3/P6: preserve boxed type→rank→length, header/operand-vector audits, fork f→g→h and modifier left→right audits. Hook/modifier train ARs explicitly decode h first, then g→f. The latter matches C argument evaluation observed in both supplied Windows DLLs; it is not a general C-language or Linux-compiler guarantee. Definite noun-left rank audit failures retain constant fallback.
- [x] P4/P5: preserve the J-visible gerund noun and current construction span without adding decoded execution auxiliaries as parent semantic children. Verify shared payload pointers and lifetime after dropping the original for a 65,536-integer noun AR. Apply the existing depth limit to recursive decoding/nested constructors. Use the outer operand span where internal AR byte offsets are unavailable.
- [x] P6: add four regression tests and 103 C comparison sentences. `frontend_probe` operation `R` explicitly observes runtime parser construction/capture and compares completed functions with C `5!:1`/`4!:0`; `A` retains read-only/static behavior. Add 39 constructor representation/POS, 18 error and 46 setup/existing-target stage checks. Invalid fixture setup fails the check rather than passing as a shared error.
- [ ] Next: gerund names/locatives and full spellin inventory, host/capture for serialized entities requiring immediate noun execution, late modifier lookup, gerund execution/Logical lowering and locale/definition scopes. Names/unregistered primitives/unsupported immediate execution remain explicit Unsupported without guessed POS or fake nouns. Defer optimization/CUDA.

Windows default/portable each **290 passed / 17 ignored**, fmt/clippy passed, Python **20 passed**. Each j64/AVX2 variant through direct, semantic-reference and parser-capture: **2,477 sentences, 2,473 passed, four existing runtime boundaries, zero failures**; **7,485 stage checks**, **6,618 word cases** passed. Separately report 44 capture graph boundaries (nine ordered-effect, 35 modifier-value) and two static boundaries (computed rank, computed gerund noun). AR decoding success does not imply all J primitive/name/execution support. Preserve DLL release/source review pin and declarative-table/actual-trace proof distinctions; skip GitHub CI.

References: [r.c fxchar/fx](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/r.c#L77), [cg.c fxeachv](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cg.c#L101), [cf.c hook and modifier dispatch](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L200).

##### Gerund name POS at construction time (2026-10-03)

- [x] P3/P4: follow `r.c::jtfxchar`→`a.c::jtswap`→`sc.c::jtnameref` to look up ordinary names in character ARs through the existing environment at construction time. Do not snapshot all sentence bindings in advance. Undefined names become Verb NameRefs; defined functions retain NameRefs with their actual current POS. Do not freeze a verb's current primitive value or expand an alias reference. Existing extension names use the same binding path.
- [x] P3/P6: names currently bound to nouns, adverbs or conjunctions fail the final gerund Verb audit with DomainError. Definite noun-left rank audit failures retain constant fallback. Verify redefinition, undefined→verb, verb→noun/modifier, verb aliases, unchanged target/version after failures and original gerund noun function representations.
- [x] P3/P6: character ARs starting with alpha and ending with neither `.` nor `:` take the name audit path. Reuse enqueuer classification/validation for ordinary names; invalid characters, spaces and trailing underscores produce IllFormedName. Keep primitive and boxed AR header spellin paths distinct. Missing name environments and required locative support remain Unsupported.
- [x] P6: add three regression tests and 30 shared C comparison sentences. Stage checks compare nine runtime constructor representations/POS, seven errors and 14 binding setup/existing-target executions. Add `a.c`, `sc.c` source hashes. Verify that decoded Verb NameRefs do not snapshot values and that nested forks using named nouns cannot succeed without the required noun snapshot.
- [ ] Next: actual named-noun snapshots/capture within compound ARs, gerund lookup binding/version observations, immediate noun execution/capture and late modifier lookup in trains. Current noun handling supports POS auditing only; nested constructors requiring values remain Unsupported. Preserving the original noun operand alone does not establish decoded execution-auxiliary snapshots or compiled reuse conditions. Gerund execution/Logical lowering, locative/locale/definition scopes and full primitive inventory remain separate gaps. Defer optimization/CUDA.

Windows default/portable each **293 passed / 17 ignored**, fmt/clippy passed, Python **20 passed**. Each j64/AVX2 variant through direct, semantic-reference and parser-capture: **2,507 sentences, 2,503 passed, four existing runtime boundaries, zero failures**; **7,515 stage checks**, **6,618 word cases** passed. Separately report 46 capture graph boundaries (nine ordered-effect, 37 modifier-value) and two static boundaries (computed rank, computed gerund noun). Current gerund name lookup is not a complete capture witness/reuse guard. Distinguish DLL release and reviewed source pins; do not claim full J/upstream-suite or GitHub CI execution.

References: [r.c fxchar](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/r.c#L77), [a.c swap](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/a.c#L21), [sc.c nameref](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sc.c#L395), [sn.c vnm](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sn.c#L9).

##### Gerund noun snapshots and lookup capture (2026-10-03)

- [x] P3/P4: decode named nouns in character ARs as their actual construction-time `Value`, held in a shared Literal. Noun-left fork/rank operands must not change after name redefinition. Do not invent abstract noun values; value-consuming constructors retain Unsupported for abstract nouns.
- [x] P4/P5: retain completed decoded functions in `FunctionEntity.decoded_gerund`. These immutable shared objects own intrinsic noun snapshots; the original boxed gerund noun remains the source operand. Do not add decoded functions as source semantic child edges of PrefixInfix/Rank. Retain decoded results only after successful gerund audits; discard partial results on quiet rank constant fallback. Retention does not imply gerund execution support.
- [x] P4/P6: record each construction-time lookup as `GerundNameResolved` with name, current binding version (absent for undefined names), actual POS, noun facts and outer AR span. Capture events retain no array payload. Preserve lookup order on success, failure and quiet fallback; extend verification to require events between the matching constructor attempt/outcome. `CapturedGraph.gerund_name_reads` is an observation sidecar, not a compiled reuse guard. Static internal-name dependency/guard contracts remain incomplete.
- [x] P6: add four regression tests covering shared payload pointers for 65,536 integers, lifetime after redefinition/original/Engine release, rank noun snapshots versus verb NameRefs, first errors and unchanged target/version, partial decode disposal and rejection of misplaced capture events. Public static analysis retains abstract bound-noun boundaries and performs no execution/assignment.
- [x] P6: add 31 shared C comparison sentences and 49 stage checks. `frontend_probe` operation `D` observes constructor-owned decoded function representations. Use C-only fix adverb `5!:0` followed by `5!:1` to compare six scalar/vector/boxed named-noun snapshots; separately check three C snapshot-retention cases after redefinition. Rust does not bypass construction through foreign execution. Preserve existing `R` comparisons of original function representation/POS.
- [ ] Next: immediate noun execution/capture in bident/trident ARs and late modifier lookup/effect contracts in trains. Gerund execution/Logical lowering, compiled reuse conditions, locative/locale/definition scopes and full primitive inventory remain separate gaps. Keep tokenizer/enqueuer/parser first and defer optimization/CUDA.

Windows default/portable each **297 passed / 17 ignored**, fmt/clippy passed, Python **20 passed**. Each j64/AVX2 variant through direct, semantic-reference and parser-capture: **2,538 sentences, 2,534 passed, four existing runtime boundaries, zero failures**; **7,564 stage checks**, **6,618 word cases** passed. Separately report 46 capture graph boundaries (nine ordered-effect, 37 modifier-value) and two static boundaries (computed rank, computed gerund noun). Distinguish C DLL release and reviewed source pins. Do not claim full J/upstream-suite, GPU or GitHub CI execution.

References: [r.c fxchar/fx and noun/fork decoding](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/r.c#L77), [sc.c nameref noun values](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sc.c#L395), [cg.c fxeachv decoded gerunds](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cg.c#L101), [cr.c gerund auditing and constant fallback](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c#L731).

##### Immediate noun calls inside bident/trident ARs and capture (2026-10-03)

- [x] P3/P4: follow `cf.c::jthook` immediate productions: execute a monad for `V N` bidents and a dyad for `N V N` tridents through the runtime semantic host. Keep common decoder/modifier construction dispositions and return actual noun Items. Pass these into outer noun-left forks and retain result values as shared Literals in the existing decoded gerund structure. Callable errors precede the final gerund Verb audit.
- [x] P4: borrow the host mutably for the actual constructor call. Later AR name lookup reads the updated environment; do not snapshot sentence-wide bindings. Static/no-host paths return Unsupported without execution. Support is bounded by existing runtime verb executor coverage; do not invent results for unsupported callables.
- [x] P4/P6: `ConstructorApply` records a completed call's function, valence-dependent input facts, span and successful result facts or error class/context. Events retain no argument/result array payload. Buffer calls together with name reads to preserve order before outer construction success/failure. Verify Verb calls only inside matching row 3/4 constructions. `CapturedGraph.constructor_calls` is an observation sidecar, not a replay plan or optimization guard. This does not capture every internal effect/name lookup inside an executed function.
- [x] P6: add four regression tests for actual monadic/dyadic values, final DomainError after a successful call, call DomainError/LengthError and quiet rank fallback, unchanged target/version and misplaced-call rejection. Check monadic `+` shared payload/lifetime for 65,536 integers after original/Engine release and Windows hook AR g→f lookup order. A mock host verifies that a call's binding change reaches subsequent lookups and static paths never call; this is a bridge contract, not proof of full J effect compatibility.
- [x] P6: add 44 shared C comparison sentences and 56 stage checks: 19 function representations/POS, eight errors, 17 setups, eight computed-snapshot setups and four decoded noun values. Cross-check decoded values through C-only `5!:0`→`5!:1` and Rust constructor observation `D`. Do not bypass Rust frontend construction through foreign execution.
- [ ] Next: late modifier NameRef lookup/application and effect contracts inside trains/ARs. Unsupported derived callables, gerund execution/Logical lowering, compiled reuse conditions, complete internal effect graphs, locative/locale/definition scopes and primitive inventory remain separate work. Keep tokenizer/enqueuer/parser first; do not implement optimization, CUDA or GitHub CI.

Windows default/portable each **301 passed / 17 ignored**, fmt/clippy passed, Python **20 passed**. Each j64/AVX2 variant through direct, semantic-reference and parser-capture: **2,582 sentences, 2,578 passed, four existing runtime boundaries, zero failures**; **7,620 stage checks**, **6,618 word cases** passed. Separately report 46 capture graph boundaries (nine ordered-effect, 37 modifier-value) and two static boundaries (computed rank, computed gerund noun). Distinguish supplied Windows DLL release and reviewed source pins; do not claim full J/upstream-suite, GPU, Linux or GitHub CI validation.

References: [cf.c immediate V N/N V N hook application](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L311), [r.c AR hook/fork decode order](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/r.c#L93), [cg.c final gerund Verb audit](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cg.c#L101), [cr.c quiet gerund audit](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c#L731).

##### Nameless modifier snapshots and late train references (2026-10-03)

- [x] P3/P4: ordinary modifier names do not always become NameRefs. Mirror C's `VALTYPENAMELESS` stack lookup: primitive modifiers and modifier trains containing only primitive ACVs/nouns share the current immutable function value at queue-to-stack entry. A train built with `adv=:/` retains that value after `adv=:1`. This is a frontend lookup policy, not a recursive name-free analysis or optimizer purity guarantee.
- [x] P3/P4: actual modifier NameRefs inside trains/ARs resolve through the existing runtime resolver when that child is applied. Same-POS rebinding takes effect; a mismatch against the stored POS raises DomainError. Undefined names and alias failures retain the existing resolver contract. Existing children remain immutable; no eager DAG rewriting occurs. Static/no-host paths retain Unsupported when actual resolution is required.
- [x] P4/P5: distinguish `ModifierStacked` from application-time `ModifierResolved`. `CapturedGraph.modifier_stack_snapshots` preserves name/version/POS/function/span without turning the snapshot name into a late verb reference. Actual resolution observes current binding versions and constructor row/span. Both sidecars are observations, not compiled reuse guards or replay contracts.
- [x] P3/P6: character AR names retain the `fxchar` NameRef path, without the ordinary stack nameless shortcut. Gerund creation-time POS reads and application-time modifier resolution remain distinct events.
- [x] P6: four regression tests cover snapshots, real late adverb/conjunction rebinding, stored POS checks, failed target/version preservation, immutable child sharing and AR read order. Existing snapshot identity tests now assert the actual stack phase. Add 63 common C sentences and 69 stage checks; include `s.c` and `jtype.h` source hashes.
- [ ] Next: review remaining modifier constructor/executor inventory and explicit/local/locative/definition frontend scope. Unsupported callables, internal effect/dependency and compiled reuse contracts, gerund execution/Logical lowering remain open. Keep tokenizer/enqueuer/parser priority; optimization, CUDA and GitHub CI remain deferred.

Native Windows default/portable each **305 passed / 17 ignored**; fmt/clippy/build pass; Python **20 passed**. Each j64/AVX2 direct, semantic-reference and parser-capture run has **2,645 cases / 2,641 passed / 4 existing runtime boundaries / 0 failed**. Stage: **7,689 checks**; words: **6,618 cases**. Report **71 capture graph boundaries (9 ordered-effect, 62 modifier-value)** and **2 static boundaries (computed rank, computed gerund noun)** separately. Additional modifier-valued graph boundaries are neither noun execution failures nor new runtime waivers. DLL release pin `ded7793fe5795d79eda8e7138dce94aa056edf78` differs from the reviewed source pin; this does not establish whole-J/upstream-suite equivalence.

Source: [p.c nameless stack lookup](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c#L693), [s.c binding classification](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/s.c#L739), [jtype.h primitive/nameless flags](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/jtype.h#L1334), [cf.c train classification](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L367), [sc.c stored POS check](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sc.c#L138), [r.c character AR](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/r.c#L77).

##### Exhaustive bident/trident constructor disposition checks (2026-10-03)

- [x] P3/P6: expose the actual `bident_disposition`/`trident_disposition` used by parser rows and the AR decoder through frontend probe `--constructors`. Read C `cf.c`'s `bidents[16]` and `tridents[64]` to compare **all 80 class combinations**: syntax error, immediate semantic application, hook/fork, derived modifier and result POS. Account for the preceding `V V` hook special case and `MARK` fork. No separate Rust golden dispatch is introduced.
- [x] P6: fail on missing tables, duplicate/unknown source entries, unknown action/result pairs and missing/duplicate probe combinations. Add three extractor regression tests. Declarative disposition agreement does not prove runtime ptcol sequencing or support for every operand value.
- [x] P3/P6: construct all 16 bidents and 64 tridents as boxed ARs, bypassing unrelated surface reductions. Add **160 common sentences** and stage comparisons for 80 AR setups, 75 errors and 5 final Verb successes. Compare the five successful decoded structures against C reference-only `5!:0`/`5!:1`. Distinguish impossible-production SyntaxError from a noun/modifier result rejected by the final gerund Verb audit with DomainError.
- [x] P6: a Rust regression checks invalid productions and `N V N` final-audit failure, preserved old function/binding version and valid capture. Static/no-host construction retains Unsupported instead of executing a required noun call. This extends validation of existing dispatch, without claiming new constructor support.
- [ ] Next: review explicit/direct definition input collection, execution-free frontend structure and local/locative/definition scope. Remaining callable inventory, internal effect/dependency and compiled reuse contracts, gerund execution/Logical lowering remain separate work. Keep tokenizer/enqueuer/parser priority and defer optimization, CUDA and GitHub CI.

Native Windows default/portable each **306 passed / 17 ignored**; fmt/clippy/build pass; Python **23 passed**. Each j64/AVX2 direct, semantic-reference and parser-capture run has **2,805 cases / 2,801 passed / 4 existing runtime boundaries / 0 failed**. Stage: **7,935 checks**; words: **6,618 cases**. Capture graph boundaries remain 71 (9 ordered-effect, 62 modifier-value); static boundaries remain 2. No new waivers. Keep DLL release and source-review pins distinct.

Source: [cf.c tables and hook dispatch](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L310), [cf.c immediate application and modifier construction](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L349), [r.c AR decode](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/r.c#L77).

##### Definition input collection and execution-free source framing (2026-10-03, partial DEF-1)

- [x] Add `definition_input.rs` with `DefinitionInput`/`InputFrame`, exposed through parser `frame_definition_input`. Distinguish ordinary sentences, incomplete input and completed definitions; preserve original operator/body byte spans and nested DD ranges. Reuse tokenizer word formation without looking up body names or reducing nouns. This is a pre-enqueue source frame, not yet Program DefinitionCode/FunctionEntity.
- [x] Collect ordinary `{{ ... }}` and literal modes 1–4 with quoted bodies or `m : 0`. Preserve decoded quote doubling, embedded LF, nested DDs and opaque quoted/comment delimiters. Following `colon0`, only a spaces-only standalone `)` terminates a block; a `)` inside a pending nested DD does not terminate the outer block. The source API preserves CRLF.
- [x] CLI stdin/scripts share physical-line collection. Supported direct/block forms wait for the closing line, then report the existing unsupported boundary once and stop input. EOF on incomplete input reports a source-spanned input error. CLI collection inserts LF between physical lines; original CRLF byte preservation belongs to the source API. Body lines never execute separately.
- [x] Verify body enqueue under `ExplicitDefinition`: local copulas remain local and future names retain Name payload/lookup flags. This does not implement local binding/frame/invocation. Source construction has no Engine/callback and does not execute future reads or side effects.
- [x] Add seven regression tests and one CLI waiting/termination test for nesting, quoting/comments, padded terminators/CRLF, EOF, LF quoted bodies, spans and local enqueue flags. A real CLI process gives no response before closure and rejects the completed definition without running its body. The **17 full-definition acceptance tests remain ignored**, not passing capabilities.
- [x] Add 14 source projections, 20 body word comparisons against C `;:`, five quoted body comparisons against C literal decoding and five input-boundary goldens. C validates complete direct fixtures and explicit string equivalents of block fixtures. This does not test native C `m : 0` input callbacks, full preparse/control-flow equivalence or Rust callable construction/invocation. Include `cx.c`, `wc.c`, `io.c` source hashes.
- [ ] Next: separate DefinitionCode from invocation frames, construct body/control-word structures and completed semantic definition entities for the existing nine-row parser. Tagged DDs such as `{{)n`, multiple root DDs in a sentence, computed/grouped colon operands, `define` aliases and modes 0/9/13 remain outside this source framing coverage. Do not guess global/static semantics for unknown scope/callables. Optimization, CUDA and GitHub CI remain deferred.

Native Windows default/portable each **314 passed / 17 ignored**; fmt/clippy/build pass; Python **23 passed**. Each j64/AVX2 direct, semantic-reference and parser-capture run retains **2,805 cases / 2,801 passed / 4 runtime boundaries / 0 failed**. Stage: **7,979 checks**; words: **6,618 cases**. Capture graph boundaries remain 71 (9 ordered-effect, 62 modifier-value), static boundaries remain 2. Input framing validation is distinct from full-J execution support; no new waivers.

Source: [cx.c colon0 input termination](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L796), [cx.c quoted body line splitting](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L841), [cx.c DD tokens/nesting](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L1345), [wc.c preparse](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wc.c#L385), [io.c definition input](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/io.c#L383).

##### DefinitionCode construction, valence and capture provenance (2026-10-04, partial DEF-1/2)

- [x] Add immutable `DefinitionCode` in `definition_code.rs`: original source/form/span, decoded body, physical lines and body-relative word spans/POS/enqueue flags. Preserve local copulas and future names without definition-time body lookup or execution. Per-call local frames remain separate and unimplemented.
- [x] Expand literal definitions to N C N and direct definitions to parenthesized `(9 : 'body')`, matching C. Construct Code at existing parser row 4 and commit at row 7. Match direct initial whitespace/LF normalization. Never eagerly construct in enqueue: an explicit colon RHS length error occurs before construction/commit.
- [x] Infer direct mode/POS from actual `u/m`, `v/n`, `x/y` body Names, excluding quoted/comment spellings. Preserve spaces-only `:` sections, mode 4 dyad selection, x/y operator status and default valence movement. A modifier without x/y and with two valences raises `ValenceError`, retaining its previous binding/version. Keep original body separate from the rearranged function representation.
- [x] Preserve completed entities as `FunctionHead::ExplicitDefinition`. CLI continues after a closing line and after ordinary stdin execution errors, retaining failure exit status. Incomplete/unsupported definitions still stop; body lines never execute separately.
- [x] Capture Input retains expanded enqueue word index. Generated mode/body nouns share a DD span, so capture→J graph validates full enqueue index/span/payload/facts instead of interpreting that span as one source word. Reject altered indices. Preserve Code VerbValues with unknown contracts; internal body analysis, modifier-value graphs and A3 callable lowering remain unsupported.
- [x] Add six Code regressions and one CLI error-continuation regression: source/local flags, unresolved future bindings, mode/POS/valence, no body execution, failure transactions, row 4 error order and generated capture verification. **17 full-definition acceptance tests remain ignored.**
- [x] Add 21 common C sentences and 28 stage checks. Compare j64/AVX2 constructed POS/atomic representation, aliases, source versus semantic body, multiline valence, unchanged counters and retained old functions after errors. Native C block-input callbacks and full preparse/control-flow equivalence remain untested.
- [ ] Next: implement `wc.c::getsen/conword/preparse` control partitioning/structures, nested/tagged definitions, multiple root DDs and computed/grouped colon operands. Invocation local frames, runtime name/POS lookup/scope, complete J graph body analysis and A3 lowering are separate stages. Defer optimization, CUDA and GitHub CI.

Native Windows default/portable each **321 passed / 17 ignored**; fmt/clippy/build pass; Python **23 passed**. Each j64/AVX2 direct, semantic-reference and parser-capture run has **2,826 cases / 2,822 passed / 4 existing runtime boundaries / 0 failed**. Stage: **8,007 checks**; words: **6,618 cases**. Capture graph boundaries: 78 (9 ordered-effect, 69 modifier-value); static boundaries: 2. Seven new modifier graph boundaries are recorded separately from successful execution comparison. Upstream full suite was not executed. DLL release `ded7793...` and reviewed source `13994ff...` are distinct revisions.

Source: [cx.c colon/valence split](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L1264), [cx.c xop/mode inference](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L1282), [cx.c direct-definition expansion](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L1456), [wc.c control-word preparse](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wc.c#L385).

##### Control-word partitioning and body diagnostics (2026-10-04, partial DEF-2)

- [x] Add `definition_control.rs` with `ControlWord`/`DefinitionPart`/`partition_line`. Following `wc.c::conword/getsen`, classify actual tokenizer words against 20 fixed controls and named `for_`/`goto_`/`label_` forms. Preserve executable fragments, original byte spans and spacing before controls; exclude leading/trailing space and NB. comments. Quoted control spellings are opaque. `if.x` forms `if.` + `x`, so it partitions as If followed by a fragment.
- [x] Audit ordinary `for_` names: invalid `for_.`/`for_1a.` produce IllFormedName. Locative loop names remain Unsupported. Goto/label target resolution and validation remain a subsequent audit, matching C's separation of classification from congoto.
- [x] Check partitions during DefinitionCode construction. Control bodies remain Unsupported without commit until control-flow auditing is implemented. The public partition API may split unmatched controls; it neither establishes validity nor executes the body.
- [x] Remap body tokenizer/enqueue/name diagnostics to original source bytes, including direct initial whitespace/LF normalization, physical block lines and quote doubling. Do not treat a body-local word index as an outer sentence index. Preserve error kind and diagnostic context.
- [x] Add five regressions for the full classification inventory, named/invalid names, spacing/quotes/comments/adjacent controls, unmatched structures and execution boundaries, retained bindings after failures, and direct/block/escaped quoted-body diagnostics. Add a Python regression rejecting unknown/duplicate/changed-length source table entries.
- [x] Read fixed inventory directly from pinned `wc.c` MATCHNAME8/length/control enum expressions. Compare 33 source projections using actual C `;:` words and the reviewed getsen algorithm, including UTF-8 quoted text and every fixed control. This does not export C's private getsen/preparse execution trace or prove full control-flow equivalence. Add four shared C sentences for two invalid-for definitions and retained old functions after failure.
- [ ] Next: implement and validate `preparse`/`conall`/`congoto` structural audits and jump/section metadata. Then continue per-call local frames/name/POS lookup, nested/tagged/multiple DDs, computed colon operands and J graph body analysis. Control-body invocation and full definition acceptance remain unsupported; defer optimization, CUDA and GitHub CI.

Native Windows default/portable each **326 passed / 17 ignored**; fmt/clippy/build pass; Python **24 passed**. Each j64/AVX2 direct, semantic-reference and parser-capture run has **2,830 cases / 2,826 passed / 4 existing runtime boundaries / 0 failed**. Stage: **8,045 checks**; words: **6,618 cases**. Capture graph boundaries remain 78 and static boundaries 2, reported separately. Upstream full suite and private C control-flow trace comparison were not executed. Source-review and DLL-release pins remain as above.

Source: [wc.c conword](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wc.c#L331), [wc.c getsen](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wc.c#L366), [wc.c preparse audit](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wc.c#L385), [sn.c vnm](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sn.c#L9).

##### Definition control structure and per-valence audit (2026-10-04, partial DEF-2)

- [x] Added control entries and jump metadata in `definition_flow.rs`, following `preparse/conall/conend`. Definition **construction** supports if/elseif/else, while/whilst/for, break/continue, assert/return/throw, try/catch/catchd/catcht, and select/case/fcase. Invocation frames and body execution remain unimplemented.
- [x] Preserve physical sentences, fragment word ranges, body-relative spans, original lines, per-valence targets and assertion origins. `go` is C's control/error target, not every normal CFG successor. Catcht runtime handling is not resolved statically.
- [x] Enqueue each valence before its control audit, and finish monad auditing before processing dyad. Literal mode 4 skips the discarded pre-divider monad. Construction performs no body assignment, name lookup or execution; errors retain existing bindings.
- [x] Apply C control-entry, sentence-word and total-word limits. Compare the control-entry boundary with native C. Verify valence ranges, target bounds, physical lines, word/source spans and assertion references. Large total-word boundary measurements and a complete CFG semantic proof remain outstanding.
- [x] Compare all **584 sequences** of eight control words at lengths 1–3 with C. Preserve noncanonical `while. if./while./whilst./for. end.` accepted by C's packed-code interval check, marking them with `analysis_barrier` rather than inferring structured-lowering eligibility. Separate Rust regression tests check nested loop/try/select targets against the reviewed pinned algorithm.
- [ ] Next: goto/label target and structure-entry audit, then `canend`/BBLOCKEND result-eligibility metadata. Invocation frames, runtime scope, nested/tagged/multiple DD, computed colon operands and Code body graph lowering are separate milestones. Optimization, CUDA and GitHub CI remain deferred.

Native Windows default/portable each: **340 passed / 17 ignored**; fmt/clippy/build pass; Python: **24 passed**. For both j64 and AVX2, direct/semantic-reference/parser-capture: **3,501 cases / 3,497 passed / 4 existing runtime boundaries / 0 failed**; stages: **8,677 checks**; words: **6,618 cases**. Record 78 capture-graph and 2 static boundaries separately. Comparisons cover construction outcomes, errors and atomic representation; **private C control/jump traces were not exported or compared**. The upstream full suite and 17 definition invocation acceptance tests remain unexecuted. All ten report binary and source hashes were verified. DLL release `ded7793fe5795d79eda8e7138dce94aa056edf78` differs from reviewed source `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`.

Sources: [wc.c conend / packed interval](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wc.c#L72), [wc.c try/select](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wc.c#L111), [wc.c conall](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wc.c#L177), [wc.c preparse](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wc.c#L385), [cx.c valence ordering](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L1264).

##### Previous B-block result eligibility metadata (2026-10-04, partial DEF-2)

- [x] Add `PreviousResult::{Unresolved, CanReturn, CannotReturn}`, preserving C conall's reverse fixed point and provisional bits. This describes the **previous B-block result**, not the current test value, purity, CFG reachability or optimization permission. Mixed successor outcomes and unconfirmed cycles remain Unresolved.
- [x] Preserve CBBLOCKEND as `before_fallthrough_end`: only a Body immediately before a non-select fallthrough end with subsequent code qualifies. Distinguish final ends, backward loop ends, select ends and assert/test nodes. The verifier rejects invalid markers.
- [x] Four regression tests cover loops, branches, assert/throw/return, independent valences, analysis barriers and corrupted references. Add six native C constructor cases. These are source-based metadata checks, not private C canend trace comparisons. No execution or optimization was added.
- [ ] Next: goto/label target and structure-entry audit, with native C comparisons of the upstream goto position matrix. Invocation frames, scope and remaining definition input support follow separately.

Windows default/portable each: **344 passed / 17 ignored**; fmt/clippy/build pass; Python: **24 passed**. For both j64 and AVX2, direct/semantic-reference/parser-capture: **3,513 cases / 3,509 passed / 4 existing runtime boundaries / 0 failed**; stages: **8,683 checks**; words: **6,618 cases**. All ten report binary hashes match. The 78 capture-graph and 2 static boundaries, distinct DLL/source review revisions and unexecuted upstream full suite remain unchanged.

[wc.c CBBLOCKEND and canend](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wc.c#L238).

##### Goto/label resolution and structure-entry audit (2026-10-04, partial DEF-2)

- [x] Follow `congotoblk/congoto/congotochk` **before conall**, building intervals from original control kinds and targeting the successor of the matching label. Preserve raw suffixes without name lookup; empty and digit-leading suffixes are not reclassified as ordinary J names.
- [x] Missing targets or referenced duplicate labels in the same valence produce ControlError. Unreferenced duplicates remain legal. Match suffixes exactly; reject entry into structures and sibling branches while allowing exits. Malformed intervals use safe bounds checks.
- [x] Verify named suffix/source consistency and label-successor references. Preserve original quoted-source duplicate-label diagnostics and existing bindings on errors. Body invocation and runtime branchout stack handling remain unimplemented.
- [x] Compare all **1,028 insertion-gap combinations** for label and goto in three templates from pinned `test/ggoto.ijs`: select/if, while/try and if/for/whilst. Both j64 and AVX2 agree on 332 successful constructions and 696 ControlErrors. This is not the full upstream suite or a private jump trace comparison.
- [x] The CLI corpus uses single-line equivalents; stage probes compare original multiline source and valence separators. Fix a transport mismatch from sending a multiline quoted definition as one CLI case, and add a regression guard rejecting CR/LF cases. Successful construction is not counted as invocation support.
- [ ] Next: nested/tagged/multiple direct-definition and computed/grouped colon framing/enqueue provenance. Invocation/local frames, scope, Code structural body graph and A3 lowering remain separate. Locative for names, optimization, CUDA and GitHub CI remain unsupported/deferred.

Windows default/portable each: **348 passed / 17 ignored**; fmt/clippy/build pass. Python: **25 passed**, including the final CLI transport regression. For both j64 and AVX2, direct/semantic-reference/parser-capture: **4,563 cases / 4,559 passed / 4 existing runtime boundaries / 0 failed**; stages: **9,724 checks**; words: **6,618 cases**. All ten report binary/source hashes match, including the added `test/ggoto.ijs` hash. Record 78 capture-graph and 2 static boundaries separately. DLL release and source review pins remain distinct as documented above.

Sources: [wc.c goto audit](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wc.c#L14), [j.h half-open intervals and DO loop index](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/j.h#L1065), [upstream goto position tests](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/test/ggoto.ijs).

##### Preserve multiple direct definitions in one sentence (2026-10-04, partial DEF-1/2)

- [x] `InputFrame::Definitions` preserves disjoint root DDs in source order, with separate delimiter/body spans and nested-range ownership. An incomplete later root makes the entire input NeedMore without committing an earlier root. Collecting nested ranges does not implement semantic nested Code construction.
- [x] Enqueue each root as an independent parenthesized `9 : body` constructor, preserving ordinary gap words. Constructors share source and primitive context through Arc. Gap diagnostics use expanded queue indices while retaining original source positions independently.
- [x] Reuse existing rows 4/7 and Hook/Fork construction, preserving separate DefinitionCodes as function operands. Do not resolve or execute/reduce body names. Static preparation never commits bindings; failed train construction retains old bindings.
- [x] Four regression tests cover quote/comment and nested ownership, incomplete collection, enqueue provenance/indices/Arc sharing, structural trains, no body execution, transactional bindings and both CLI paths. Add 15 native C outcome/atomic-representation/transaction cases and four stage input/incomplete projections.
- [ ] Next: semantic nested DDs, tagged DDs, computed/grouped colon operands and mixed literal-colon/DD framing in one sentence. Multiple ordinary root DD support is not complete definition-form support. Invocation/local scope, A3 and Code body graph lowering follow separately. Optimization, CUDA and GitHub CI remain deferred.

Windows default/portable each: **352 passed / 17 ignored**; fmt/clippy/build pass; Python: **25 passed**. For both j64 and AVX2, direct/semantic-reference/parser-capture: **4,578 cases / 4,574 passed / 4 existing runtime boundaries / 0 failed**; stages: **9,743 checks**; words: **6,618 cases**. All ten report binary/source hashes match. Record 78 capture-graph and 2 static boundaries separately. The full upstream suite, definition invocation acceptance and private C trace equivalence remain unverified. DLL release and source review pins remain distinct as documented above.

Sources: [cx.c repeated DD expansion](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L1456), [p.c parser reduction](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c).

##### Raw input and noun preservation for tagged noun DD (2026-10-04, partial DEF-1/2)

- [x] Add `DefinitionForm::NounDirect` for `{{)n ... }}`. This produces a **character noun**, not function Code. Quotes, NB., control spellings and `{{` in the raw body are not words or executable sentences. Any `}}` on the first physical line closes it; later lines require column-zero `}}`. An empty header omits the initial LF, while a nonempty header retains the separator before the next line.
- [x] Retain original delimiter/body byte spans while converting physical CRLF to logical LF. Rescan later roots separately so an unmatched raw quote cannot contaminate subsequent ordinary/noun DD scanning. Nested noun DD inside ordinary DD and other tags remain Unsupported.
- [x] Enqueue one Noun with original span/index, distinguishing a single-byte character scalar from empty/multiple-byte character arrays. Create no DefinitionConstructor or invocation frame. Reuse existing noun reduction, assignment, snapshot and constant-noun fork construction. Adjacent nouns retain C's N/N syntax error; no concatenation grammar is invented.
- [x] Six Rust regression tests cover raw values/shapes, multiline/column-zero delimiters, mixed roots, enqueue provenance, noun snapshots, static non-commit, both CLI paths with quotes/comments/CRLF, and UTF-8 byte-boundary panics. Framing safely leaves invalid ordinary primitive bytes for enqueue diagnostics.
- [x] Add **161 C corpus cases**, with 11 fixed bodies and 64 bodies generated with seed 20261004, value observations, snapshots and mixed trains. Add **176 stage checks**. Six multiline cases use C `0!:100` script input to compare actual physical-line handling of LF/CRLF, empty headers, embedded delimiters and raw quotes. A single multiline JDo call is not treated as collection evidence.
- [x] Observe the copula prefix when raw quotes make original-source `;:` observation fail, without executing the source twice. Set Windows subprocess transport explicitly to UTF-8 and compare Korean raw noun bytes. Add two adapter regression tests.
- [ ] Next: semantic nested DD/nested noun DD, other tagged/computed/grouped forms and mixed literal-colon/DD framing. Audit unfinished-input EOF error-class compatibility separately. Callable A3, body graph lowering, invocation/local scope and locative for names remain incomplete. Optimization, CUDA and GitHub CI stay deferred.

Windows default/portable each: **358 passed / 17 ignored**; fmt/clippy/build pass. Python: **27 passed**. For both j64 and AVX2, direct/semantic-reference/parser-capture: **4,739 cases / 4,735 passed / 4 existing runtime boundaries / 0 failed**; stages: **9,919 checks**; words: **6,618 cases**. All ten report binary/source hashes match, including the added `test/g0x.ijs` hash. Record 78 capture-graph and 2 static boundaries separately. The full upstream suite, definition invocation acceptance and private C trace equivalence remain unverified. DLL release `ded7793fe5795d79eda8e7138dce94aa056edf78` differs from reviewed source `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`.

Sources: [cx.c noun DD raw collection](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L1413), [io.c physical input normalization](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/io.c#L316), [io.c script-line input](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/io.c#L362), [upstream string-script execution](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/test/g0x.ijs#L32).

<a id="static-frontend-review"></a>

##### Static-analysis acceptance and existing frontend review (2026-10-03)

**User acceptance:** analysis of operation graphs and memory requirements without execution is sufficient initially. Static analysis stays the default. Concrete execution plus capture serves value-dependent semantics/dynamic observation; full capture is not a prerequisite for supported static analysis. Track full J runtime compatibility, capture and compiled reuse separately.

Static nouns carry Noun POS, graph origin, inferred facts and an optional constant. Concrete nouns carry actual `Value` and separate origin. Use the same matcher/row rules with explicit action/context differences. Do not invent known noun values/POS from Unknown. Constant folding must preserve J errors/effects/bindings.

| Existing structure | Keep | Required change |
|---|---|---|
| `tokenizer.rs` | Transition table, raw spans, comment cutoff, quote errors | No capture-driven change; no execution/target fields |
| `enqueuer.rs` | Literals, primitive POS, unresolved names, lookup/copula flags, provenance | No graph construction; forward environment/flags/provenance beyond parser entry |
| `ParseValue::Noun(Expr, usize)` | Noun class and semantic structure | Distinguish concrete Value from static facts/origin carriers; do not treat an Expr as an actual value or erase existing static graphs |
| `expression()` / queue drain | Common queue/stack and noun/function distinction | `resolve_stack_item` performs lookup at right-to-left stack entry, with analysis/runtime noun snapshots distinguished by `ParseContext`; runtime invocation and top-level single-name non-final assignment are implemented; explicit-local/locale/effect extensions remain pending; static analysis needs stable POS/bindings or an explicit boundary |
| Rows 0–2 / `eval_program` | Monad/dyad semantics and kernels | Static actions retain application Expr/graphs. Concrete host actions immediately return Values; final nouns do not replay computation. Unsupported executor/effect forms remain explicit boundaries |
| `completed_noun()` / rows 3–6 | Completed FunctionEntity, source operators and ordered operands | `completed_noun` extracts Literal/Group, but runtime rows 0–2 now produce actual Literals before constructors. Static value-dependent boundaries remain explicit. Static constructors require known/proven constants; concrete constructors use actual values and preserve origins |
| `Item` / row 7 / errors | Spans and enqueue assignment metadata | `Item` retains original-word ranges/inherited tokens and enqueue flags; row 7 retains target/copula provenance and flags in `AssignmentSource`. Static final writes remain proposals; runtime row 7 performs both final and non-final assignments. Do not silently interpret unsupported local execution as global |
| Row 8 / graph adapter | Parenthesis reduction boundaries | Preserve noun origin through grouping; no extra execution op for parentheses; `ParseReduction` retains production/operand/source provenance |
| APIs / runtime / analysis | Read-only `prepare_semantic/analyze_j_graph` | `ParseContext::{Analysis, Runtime}` replaces `snapshot: bool`; `ActionContext`/`RuntimeParserHost` now supplies lookup/invocation and optional capture; top-level assignment host actions are implemented; explicit-local/locale/definition extensions remain pending |

The file separation stands. Changes concentrate in parser payload/actions and metadata forwarding, not lexer grammar or physical Value storage. Evidence is the current Noun carrier, lookup loop, constructor extractor, `AssignmentSource` copula metadata and `eval_program`, cross-checked against [p.c](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c) and [w.c](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/w.c). This review has been updated for the parser changes; kernels/executors are unchanged.

`j_graph_memory.rs` currently reports known logical extents/atoms, graph-order live ranges and materialization opportunities, with byte counts evaluated under an explicit representation model. These are not actual allocations or guaranteed peak device memory. Unknown shapes remain unknown; symbolic expressions/conditions can extend coverage. Physical peak/residency depends on schedule, layout, aliases and variable-width boxed/sparse representation. Given structure and input facts, static analysis need not execute array elements; this does not claim full symbolic-shape/J-form implementation.

- [x] Review all three modules: keep tokenizer, forward enqueue metadata, change parser/context/result contracts.
- [x] Define minimum acceptance as static graph/logical memory analysis, separate from full capture/JIT/physical peak estimation.
- [ ] **P2/P5 static-first interfaces:** retain existing static graphs while introducing explicit static/concrete carriers and actions; regression-test that compilation executes no runtime effects.
- [x] **F2/P4 supported lookup/provenance:** retain original-word ranges/inherited tokens/final copula flags and resolve names at stack entry; verify supported structures and failure-stop behavior. Non-final assignments, locale changes and effects remain separately pending below.
- [ ] **P3/P6 constructor boundaries:** analyze known constants; report reasons for unknown concrete values/POS without changing unsupported analysis into J syntax errors.
- [ ] **P5/P6 memory gate:** infer graph/liveness/extents from input facts, retain Unknown, distinguish atoms/represented bytes/estimated physical peaks.

**Implemented provenance contract:** `Program.reductions` and static reports retain row ids, ordered operand word ranges, result ranges/POS, byte spans and inherited tokens without copying noun payloads. In jsource, modifiers/forks/bident hooks inherit the left operand's `.t`; non-fork tridents inherit the middle operand's `.t`; parentheses inherit `(`. Rows 0–2 retain the right noun token, which C describes as immaterial for non-executable nouns. Failure-operator blame and result inheritance are distinct. Rust indices are zero-based. Source-based regressions cross-check [stack entry](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c#L735), [noun results](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c#L922) and [modifier/train/parenthesis actions](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c#L1002); this does not export or compare actual C stack traces.

Nine regressions cover lookup order/per-entry noun snapshots, stopping unvisited lookups after constructor failure, named modifier POS, all nine rows' word provenance, grouped modifier/fork origins, final copulas, error tokens and named insert/fork analysis coverage. The C corpus adds 13 sentences. `(entryverb/ % #) entrynoun` and `(entryverb/ % #) entrycopy` preserve frontend structure but lack an existing runtime executor. Reports retain exact sources, actual C/Rust outputs and reasons in `coverage_boundaries`; these count as neither passes nor C baseline deviations. Other errors or unregistered failures remain ordinary failures. The remaining 2,061 value/error comparisons and stage/word checks passed.

These static-analysis gates do not wait for completion of all runtime-capture work. The capture checklist below remains a separate execution-path gate.

##### Checklist integrated with F2/P2–P6

- [x] Research real/symbolic tracing, lazy evaluation, graph breaks/guards and compiler SSA using official references and ProxyTensor source.
- [x] Adopt actual noun reduction plus separate compilation capture; no verb-only syntax restriction.
- [x] Record current implementation gaps and acceptance gates in both canonical documents.
- [ ] **P2/P4 runtime actions:** one parser semantic host/context supplies name lookup, invocation and constructor validation. Resolve at right-to-left stack entry; rows 0–2 return actual nouns. Do not claim unsupported effectful forms are implemented.
- [ ] **P2/P5 capture carrier:** add opt-in occurrence ids, associations and ordered attempt/success/failure events, without compiler fields in physical Value storage or primitive executors. Establish capture-on/off parity first.
- [ ] **P3/P5 construction origins:** connect completed FunctionEntity structure and computed noun operands. Cover `f=:+"(1+0)` and `f=:(1+2) + *` using actual values rather than placeholders and an explicit support manifest.
- [ ] **P4 names/effects:** capture supported same-sentence assignment, binding/POS/locale changes in semantic order; distinguish prior effects from pending outer commit and report incomplete coverage.
- [ ] **P5/P8 graph adapter:** translate input/constant/read/apply/constructor dependencies into verified existing J Graph. Keep opaque operations as optimization barriers with separate lowering coverage.
- [ ] **P6 validation:** Windows default/portable and ordinary/AVX2 C differential, including recorded coverage, revisions/hashes and pending/deviations; no GitHub CI.
- [ ] **Later P5/P8 static/reuse:** require purity, error ordering and binding/value guards before abstract actions, region compilation or replay. The capture execution path completes at runtime reduction + capture parity + verified J Graph; this is separate from minimum static-analysis acceptance and requires no production JIT/CUDA.

##### Regression/acceptance matrix

| Axis | Cases/method | Required evidence |
|---|---|---|
| Topology | Set `a=:2`, `b=:3`, `c=:4`; compare `a+b*c`, `(a+b)*c`, monad chains | C outcome parity; actual reduction order and producer/consumer edges distinguish parentheses |
| Equal values/distinct producers | `(2*3)+(1+5)` | Distinct origins for both intermediate sixes; final add references both |
| Constructor nouns | `f=:+"(1+0)`, `f=:(1+2) + *`, computed rank/length/domain failures | C `4!:0`/`5!:1`, results/errors match; production edges survive and construction errors are not postponed |
| Completed function structure | `+/ % #`, hook/fork, nested rank/atop | Source operators, ordered operands and completed modifier boundaries match atomic observations |
| Names/effects | Noun rebind, late function alias rebind, supported intermediate assignments | Snapshot/version and late POS binding preserved; capture-on/off sequencing matches |
| Partial failures | `1+('a'+2)`, `(1 2+1 2 3)+('a'+1)` and reversed failing branches | C error/precedence match; no output or outer commit after failure; incomplete graph rejected as executable |
| Exactly once | Semantic-host test double counts invocations/assignments; separate C comparisons of supported sentences | Capture does not double effects; simulated host tests do not claim full-J implementation |
| Memory | Large array chains, alias inputs, multiple statements, release capture and observe lifetimes | No full-array copy per node, no diagnostic retention, bounded facts/constants; buffer identities remain separate |
| Reuse | Changed shape/binding/POS/constructor values and both branches | Guard invalidation or semantic execution; one traced branch is not reused universally |

**Status:** research and plan only. Runtime actions, capture API and retention tests are not implemented/run by this change. Earlier 212 Rust tests and 7,014 stage checks are not evidence for this new capture implementation.


---

# Part II — Semantic identity

## 5. FunctionEntity DAG

Large J-derived functions are represented as shared immutable graph entities rather than recursively copied trees.

The parser owns J construction identity.

Examples:

- primitive verb;
- applied adverb;
- applied conjunction;
- hook;
- fork;
- train;
- name reference;
- derived extension function.

The source operator remains the parent semantic identity. For example, applied `/` and `"` are not replaced in Semantic IR by execution-specific `Reduce` or `CellApply` nodes.

Those execution concepts are derived later.

### 5.1 No modifier summary fields as semantic identity

Legacy summary fields such as:

```text
reduce: bool
rank: Option<...>
```

must not replace the FunctionEntity topology.

Current implementation has removed the former `Verb.reduce`, `Verb.rank`, `Callable.reduce`, and `Callable.rank` semantic-summary dependencies.

<a id="current-j-vocabulary"></a>

### Current J vocabulary audit — NuVoc and implementation status (2026-10-05)

Use [NuVoc](https://code.jsoftware.com/wiki/NuVoc), reviewed 2026-10-05 at [revision 60409](https://code.jsoftware.com/mediawiki/index.php?title=NuVoc&oldid=60409), as the current documentation inventory. The older Dictionary vocabulary is historical reference, not the current support list. Read the index and relevant pages, then cross-check pinned `ws.c` spelling codes, `t.c` POS/constructors and C base/AVX2 `4!:0`. Distinguish web revision, reviewed source pin and executed DLL release. Individual web fetches for `c.`, ObsoleteSyntax, `[.` and `$::` failed; use index/C evidence for those entries.

| Current forms | Correct interpretation / design impact | RustJ status |
|---|---|---|
| `t.` / `T.` | Task conjunction / thread-task-debug verb; pyx/open/error/context and synchronization effects, not Taylor families | Runtime/effect/resource design; task execution remains unsupported |
| `[.` / `].` / `]:` | Lev/Dex conjunctions return the selected noun/verb; Ident adverb returns its operand | Implemented through core enqueue/shared constructors in this step |
| `/..` | Key dyad passes the key as the operand dyad's left argument; distinct from `/.` | Inventory/contract candidate; execution unsupported |
| `c.` / `m.` | Precision conversion verb / modular arithmetic conjunction | Value/numeric and constructor contracts; execution unsupported |
| `f.` / `f:` | Fix family; do not assume identical fix modes | Scope/name transformation contracts; execution unsupported |
| `F.` `F..` `F.:` `F:` `F:.` `F::` / `Z:` | Fold recurrence/collection variants and status/termination | Control/assembly/effect contracts; do not automatically replace with Reduce/Scan |
| `$:` / `$::` | Self-reference / scope-shortening adverb | Recursion/scope contracts; `$::` is not an alias for `$:` |
| `{{ }}` / `u.` `v.` | Direct-definition framing / caller-context operands | Existing partial frontend/runtime support; not proof of full definitions |
| `@` / `@:` | Atop / At; rank-sensitive and infinite-rank composition differ | Existing internal `ConjunctionId::Atop` names `@:`; clarify its comment, do not register `@` as an alias |
| `d.` `D.` `D:` `t:` `..` `.:` `s:` `I:` | Historical spellings rejected by both supplied C variants | Exclude from current coverage; fixed spelling errors now match C through NV3a; full name/numeric grammar remains NV3 work |

`s:` is in NuVoc's obsolete section and is rejected by both supplied C DLLs. Internal Symbol storage does not imply current J `s:` support. Distinguish rejected `I:` from current `I.`/`i:`. Keep `E.` Find Matches distinct from `I.` Interval Index. Distinguish NuVoc behavioral rank descriptions from C `b.0` intrinsic headers/IRS behavior; do not replace verified headers solely from the wiki summary. `u"v` and `m"v` Copy Rank remain separate forms. This inventory is a design classification, not proof of full NuVoc frontend/runtime/backend coverage or of a complete execution basis.

**NV1 code change (previous stage):** register `[.`/`].`/`]:` with primitive registry **7** and Graph IR **0.7**. Return the selected entity, preserving original function/NAME late lookup and noun snapshots. Do not omit prior operand noun reduction/assignment/errors. Source/reduction/capture retains whole construction provenance; no new array kernel is introduced. The tokenizer state machine needs no change. `ConstructionNounSuccess.selected_input` connects a selected noun to its original dependency node; discarded calculation nodes remain. Assignment still requires the existing ordered-effect graph boundary. Static parsing explicitly reports Unsupported when it cannot preserve a discarded noun computation, rather than erase that computation and approve an executable graph or invent a J error.

`tools/vocabulary_audit.py` derives core spelling candidates from pinned `ws.c` and compares C POS with Rust enqueue. It does not establish complete structural/control/direct-definition/name coverage. Separate word-formation passes, verified enqueue POS, Unsupported coverage and runtime conformance. Store raw results in `reports/vocabulary-*-windows.json`. A nonzero `ws.c` code does not prove an installed primitive: `w.c` also checks permanent usecount in `ds(e)`. `?:`/`` `. `` are code candidates, absent from the reviewed `t.c` installation and rejected by both DLLs; do not count them as valid unsupported primitives or POS passes. This audit does not cover all numeric-grammar forms such as `__`/`_.`. Record counts in the NV gates below. The standard `tools/check-frontend-windows.ps1 -Avx2` runner also audits vocabulary for each DLL, keeping these records updated with subsequent frontend gates.

Sources: [Task](https://code.jsoftware.com/wiki/Vocabulary/tdot), [Threads](https://code.jsoftware.com/wiki/Vocabulary/tcapdot), [Ident](https://code.jsoftware.com/wiki/Vocabulary/squarertco), [Dex](https://code.jsoftware.com/wiki/Vocabulary/squarertdot), [Key](https://code.jsoftware.com/wiki/Vocabulary/slashdot), [Modular](https://code.jsoftware.com/wiki/Vocabulary/mdot), [Fold](https://code.jsoftware.com/wiki/Vocabulary/fcap), [Copy Rank](https://code.jsoftware.com/wiki/Vocabulary/quotev). Source review pin `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`; executed DLL release `ded7793fe5795d79eda8e7138dce94aa056edf78`. These revisions are not asserted equal.

<a id="nv2-core-recognition"></a>

### NV2 — core spelling/POS recognition separated from execution capability

`PrimitiveSemanticId::Vocabulary` / `FunctionHead::VocabularyPrimitive` retain a verified operand-free core spelling/POS identity. Add 108 private-field descriptors to the core resolver and carry them through `EnqueuedPayload::Function` and the shared parser. Existing executable primitives take priority; extensions remain ordinary NAMEs. Registry **8**, Graph IR **0.8**. Bare task/Fold/Key dyad/precision/scope/constant function binding is not execution support.

Unverified descriptor rank/shape/dtype/effect/resource contracts remain unknown: `innate_ranks()` returns None; graph rules are DynamicOrUnknown; execution lowering/executors and application of the new descriptor itself as a modifier report Unsupported. Existing rank/selector/train constructors may retain verified structure without creating a new kernel or claiming purity. Copy Rank never guesses an unknown RHS header. Primitive modifier value stacking follows reviewed `t.c` VF2NAMELESS metadata and is separate from purity. Ordinary verb aliases retain late lookup.

`a.` is the char vector of bytes 0–255; `a:` is a scalar box containing an empty bool vector. Bounded `OnceLock<Value>` immutable shared payloads avoid buffer copies across enqueue occurrences. This does not introduce a large input/intermediate snapshot cache or physical layout/device facts into logical nouns.

The audit compares C `4!:0`/`5!:1` and Rust bare parser bindings for 140 functions and type/shape/payload for three nouns. Decoding the actual `!` identity in a gerund can now construct a Prefix entity, while executing it remains Unsupported. Update that regression to distinguish successful construction from unimplemented execution and include its common C fixture. Core recognition does not establish complete modifier/control/name semantics; continue NV3–NV5.

Evidence: [ws.c spelling codes](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ws.c), [t.c permanent nouns/POS/VF2NAMELESS](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/t.c#L81), [w.c installed primitive check](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/w.c#L140). Reviewed source and executed DLL provenance remain separate.


### 5.2 Names and binding

J Name, BindingVersion, semantic FunctionEntity, and Logical SSA ValueId are distinct concepts.

Late-bound function names must retain J semantics unless specialization is justified by a binding/version witness or runtime guard.

Names must not be globally snapshotted at sentence start if doing so changes J's observable right-to-left lookup or assignment behavior.

<a id="dynamic-semantic-boundaries"></a>

### 5.2.1 Dynamic semantic boundary contracts

This **design contract** consolidates name/order semantics and FOUNDATIONS §21/§33 into route-selection conditions. Do not create permanent syntax-level compilation bans. J validity, graph construction/partial analysis, transformation legality and backend execution capability are distinct decisions. If the frontend cannot establish the actual constructor/POS, report source/reason/phase rather than inventing a completed graph.

| Boundary | Required evidence | Check/use point | Missing or invalid evidence | Current implementation |
|---|---|---|---|---|
| **DB-N noun inputs** | J-read noun snapshot and dtype/rank/shape; contents conditions separately | Semantic noun-read point; separately validate analysis metadata when actual inputs are bound | Analyze available facts only; neither invent constant payloads nor replace an existing snapshot with a later binding | Noun snapshots and WI1 metadata checks exist. WI1 does not prove atoms/bindings/effects/storage safety or executable-plan reuse |
| **DB-F named functions** | Actual lookup environment, current POS/callable and binding stability used by specialization | Preserve parser-time POS lookup and call-time late lookup separately; copied rank/cap facts belong to construction | Preserve NAME/source DAG and dynamic calls; reject unproven inline/CSE/hoist/specialization | Partial late-name/POS and constructor snapshots exist. Catalog versions are not runtime witnesses; integrated guard dispatch is pending |
| **DB-L local/locale lookup and mutation** | Call frame, local binding presence, current locale/path, locatives and search-result validity | Actual semantic lookup point and after relevant namespace/context mutations | Keep runtime lookup/write order; a referent version alone cannot fix the entire search result | Partial local/public assignment and modifier frames exist. Full locale/locative/path guards are pending |
| **DB-X string execution `".`** | String contents plus execution environment/POS/effects; constant text does not prove purity | Execute point; dynamic strings require an actually supported shared frontend/JIT route | Opaque runtime boundary; do not substitute arbitrary constants or omit environment from a string-only cache key | This contract does not declare general execute/JIT/cache support; register capability after implementation/validation |
| **DB-C value-dependent modifier/definition construction** | Required noun values, constructor success/errors, actual result POS and construction facts | Corresponding parser reduction; subsequent parsing must use state changed by semantic actions | Require shared semantic parsing at construction; do not guess unexecuted/unconstructed results | Partial shared parser/runtime constructors and capture exist. Static missing-value and full definition/control boundaries remain |
| **DB-E effects and observable errors** | Namespace/resource reads/writes, I/O/context changes, errors/throw/catch behavior and speculation contracts | Original semantic order; establish legality before reorder/parallelization/elision | Preserve ordering barriers; unknown effects are not pure and unused results do not imply unused effects | Partial runtime order/error regressions and early effect contracts exist; full effect graphs/optimizer legality/dispatch remain incomplete |
| **DB-A value-dependent results/boxed/sparse/rank assembly** | Operation type/shape/structure, empty prototype and fill/assembly contracts | Analyze known facts; validate residual conditions at execution | Unknown or conservative lowering; do not impose unverified uniform tensors/affine access | Foundations and partial execution/analysis exist; general prototype/heterogeneous assembly/full boxed/sparse boundaries remain |

Runtime handling in this table requires **existing, verified RustJ capability**. Do not assume a fallback always exists or invoke the C engine. Missing support is an implementation-coverage boundary, distinct from invalid J.

**Transformation and guard-failure rules**

- Do not inline unstable NAMEs or freeze all names against the sentence-entry environment. Preserve the lookup consequences of `=.`/`=:`/locale mutation/dynamic execute. An unbound-local lookup that falls through to a locale can become invalid when a local binding is subsequently created.
- Require evidence that checked bindings/metadata retain their meaning **until use**. One region-entry version check is not automatically sufficient. Recheck at relevant mutation points or prove intervening mutation impossible. Concurrent mutation additionally needs real snapshot/lease/synchronization contracts.
- Keep constructor-fixed rank/cap facts separate from executable binding witnesses. Fixing implicit operands and reconstructing a **new entity** differs from late lookup changing an existing entity.
- Topology alone permits neither fork parallelization nor CSE/hoist/speculation/deletion of effectful or observably failing calls. Graph preservation/analysis does not authorize these transforms.
- A specialization guard miss is not a J error. Reanalyze or select a supported route before effects. Explicit input-contract violations and J errors have distinct policies; compiler-analysis failures must not replace or prematurely expose the original J error.
- **Never automatically replay the whole sentence after effects.** A later runtime transition requires an implemented/verified exact continuation retaining completed writes/I/O, live noun snapshots, frame/locale and parser queue/stack/reduction position. Until then support only pre-effect selection and do not claim mid-execution transition capability.
- A backend precondition miss rejects that route. Do not assume an unverified alternative backend/native path is a successful fallback.

**Unknown facts and implementation boundaries**

| Unknown information | Remaining static work | Residual conditions |
|---|---|---|
| Shape/type/extent | Known topology, partial shape/resource expressions and boundary reports | Conditions actually used by the kernel/memory analysis |
| Function identity/POS or constructor result | Source/provenance and already justified structure | Actual lookup/construction and stability; never guess a completed parse when POS is unknown |
| Effects/errors/alias | Conservative order/reuse barriers | Legality proof for the particular transform |
| Physical layout/device/capability | Logical meaning and route candidates | Representation/target adapter validation; no physical assumptions in logical facts |

Existing `GraphAnalyzability` states (Static / StaticWithUnknownFacts / RequiresSpecialization / DynamicSemanticFallback) and `AnalysisBoundary` are initial classifications. **Static does not mean pure/error-free/reorderable/executable.** `validate_noun_inputs()` does not prove function/effect/whole-execution safety. These classifications are not a completed guard/continuation/dispatcher implementation.

### 5.2.2 Fallback / guard miss / replay decision table

`fallback` is not one operation. Distinguish:

```text
route fallback
  choose another verified route before execution starts

guard miss
  discover before effects that a specialization/optimization premise is false

replay
  restart an already-started region/sentence from its beginning

continuation / deopt
  resume at the exact semantic point while retaining already committed effects
```

RustJ generally permits **pre-execution route fallback**. A pre-effect guard miss may trigger reanalysis/reselection when a real verified capability exists. Generic replay and exact continuation/deoptimization are not currently implemented/verified capabilities.

| Situation | J semantic error? | Alternative route? | Replay? | Current rule |
|---|---|---|---|---|
| target capability miss during compile/lowering | no | yes, if execution has not started and a verified alternative exists | unnecessary | route miss; otherwise UnsupportedImplementation |
| specialization guard miss before observable effects | no | yes, if reanalysis/verified fallback exists | prefer route reselection rather than restart semantics | never expose the miss as a J error |
| explicit compiler/API contract violation | distinct from J semantics | according to that contract | not automatic | keep contract errors separate from Domain/Rank/etc. |
| A3 SemanticCheck or semantic call raises Domain/Length/Rank/Index/etc. | **yes** | no; do not switch backend to evade the same J error | no | preserve J class and precedence |
| backend-adapter precondition miss before region execution | no | yes, with a verified alternative | unnecessary | reject that route only |
| native/external Unsupported before any observable effect starts | no | conditional, only when the whole region remains untouched and the alternative is verified | v0 treats this as route reselection | implementation-coverage boundary |
| implementation failure after partial kernel/external execution | normally not a J semantic error | not automatically | **forbidden by default** | cleanup, then report implementation failure/Unsupported unless a transactional contract exists |
| failure after namespace write/I/O/other observable effect commit | may coexist with already observable J state | not automatically | **forbidden** | do not claim mid-execution fallback without verified exact continuation |
| async failure after completion/token/resource publication | not automatically a J error | only if completion/effect state is fully proven | forbidden by default | future async contract must own the completion/effect frontier |

Current `lowering.rs::RouteDecision::RuntimeSemanticFallback` means **compile-time classification that this operation needs a semantic/runtime route because no current native ExecutionBasis realization is available**. It does not promise that RustJ may run a native kernel, fail halfway, and jump back to the interpreter. If the semantic/runtime route itself does not support the form, the result may still be `UnsupportedImplementation`.

Future dispatch must distinguish at least these commit frontiers:

```text
before_start
  no observable effect or consumer-visible transfer

guarded_but_uncommitted
  guards/checks may have run, but no replay-sensitive effect

committed
  namespace write / I/O / visible mutation / non-rollback transfer occurred
```

Verified alternate-route selection is possible in the first two states. After `committed`, region-start replay is forbidden unless exact continuation or transactional rollback has been implemented and verified.

Required tests include: guard miss never becoming a J error; pre-effect route miss choosing a verified alternative; SemanticCheck errors not disappearing under backend changes; a forced Unsupported after a namespace write proving the write is not executed twice; and future async/transfer tests on both sides of the completion frontier.

Track implementation in the [DB0–DB7 migration checklist](#dynamic-boundary-checklist); boundary conditions and implementation status are defined above.

Evidence: [FOUNDATIONS §21/§33](FOUNDATIONS.md), pinned C [p.c parser](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c), [sc.c NAME constructor](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sc.c#L364), [cx.c return fix](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L684), [af.c reconstruction](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/af.c#L193). DB0–DB7 classification/sequencing is a RustJ design decision, not a claim to have copied a complete upstream guard system.


---

<a id="extension-primitive-inventory"></a>

## 5.3 Extension primitives — functions, rationale and adoption status

Updated **2026-10-02** after checking remote default-branch HEADs and reading documents/registries at those revisions. Later topic-specific decisions and current RustJ invariants take precedence; historical descriptions/registry records are not evidence of executable support.

| Repository | Verified latest HEAD | Evidence role |
|---|---|---|
| `yunskim/JAXA` | [`12bc0659`](https://github.com/yunskim/JAXA/tree/12bc0659a1e008597bf07449c70029bc61eff93b) · 2026-03-24 | Initial motivation; later decisions supersede it |
| `yunskim/JAXA-complier` | [`ceba0589`](https://github.com/yunskim/JAXA-complier/tree/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368) · 2026-04-26 | Prototype inventory with POS |
| `yunskim/japchae` | [`510c31b5`](https://github.com/yunskim/japchae/tree/510c31b5fe3d4bf6ca0c6fa9aeae8556ca134607) · 2026-06-19 | Pooling/dropout and state-design history |
| `yunskim/jaxa-analyzer` | [`7275d5ba`](https://github.com/yunskim/jaxa-analyzer/tree/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33) · 2026-09-30 | Later cast/effect/AD/Flow–Storage review; current authority transferred to RustJ |

**Source keys**: [P — prototype registry](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py); [T — historical training modes](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/README.md); [A — extension architecture §§1.10, 3.4–3.5, 4.3, 7.4–7.7](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md); [F — later Flow–Storage scope §8](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/flow_storage_scope_and_compiler_positioning.md); [W — Flow–Storage technical review](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/flow_storage_model_review_and_open_questions.md); [D — pooling/dropout decisions](https://github.com/yunskim/japchae/blob/510c31b5fe3d4bf6ca0c6fa9aeae8556ca134607/documents/decisions.md).

The rationale/minimum-contract column records RustJ requirements inferred from the source descriptions; it does not claim the prototypes implemented those contracts. [Matching Korean canonical inventory](PROJECT.ko.md#extension-primitive-inventory).

**Why extensions:** these computations are not necessarily inexpressible in standard J. Provide reference definitions where possible. An extension name supplies a parameter schema, shape/numeric/effect contract, high-level graph identity and verified library/native/external lowering seam. Without a supported execution route, registration is incomplete.

Source names are ordinary J bindings, not reserved keywords. Distinguish adverbs deriving verbs from parameter/verb operands, computational verbs, conjunctions deriving entities from two operands, and registration APIs declaring relations. Builder POS is not the derived computational verb’s valence/rank.

| Name/family | Surface POS/kind | Function | Rationale and minimum contract | RustJ status | Sources |
|---|---|---|---|---|---|
| `conv / conv_forward` | Adverb | Build a spatial convolution verb from a parameter noun. | Retain window/channel-contraction structure for reference, library and kernel routes; specify kernel, stride, padding, dilation, bias and explicit weight resources. | Candidate; execution unverified | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| `depthwise_conv / depthwise_conv_forward` | Adverb | Build a channel-wise convolution verb. | Expose different channel connectivity/reuse; specify multiplier and output shape. The prototype identity shape rule is insufficient for registration. | Candidate; execution unverified | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py) |
| `linear / linear_forward` | Adverb | Build an affine transform from input/output sizes and bias configuration. | Analyze contraction plus bias and distinguish layer resources; separate plain matmul from parameter/resource binding. | Candidate; execution unverified | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| `bn / bn_forward` | Adverb | Build batch normalization with channel statistics. | Expose batch reduction, training/inference differences and running-stat effects; specify epsilon, axes and update rules without fixing a hardware barrier. | Candidate; execution unverified | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| `ln / ln_forward` | Adverb | Build per-sample feature normalization. | Distinguish sample-local reduction from batch statistics; specify affine parameters, epsilon and axes. Sample independence does not remove internal reduction synchronization. | Candidate; execution unverified | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py) |
| `avgpool2d` | Adverb | Build spatial average pooling from window/stride parameters. | Preserve window overlap and reduction; specify padding denominator and empty-window behavior. Fusion remains a later decision. | Candidate; execution unverified | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [D](https://github.com/yunskim/japchae/blob/510c31b5fe3d4bf6ca0c6fa9aeae8556ca134607/documents/decisions.md) |
| `maxpool2d` | Adverb | Build window maximum pooling. | Expose reduction and backward-index requirements; specify NaNs, ties, padding, empty windows and backward selection. | Candidate; execution unverified | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [D](https://github.com/yunskim/japchae/blob/510c31b5fe3d4bf6ca0c6fa9aeae8556ca134607/documents/decisions.md) |
| `dropout` | Adverb | Build a random-mask transform from a probability parameter. | Make training/inference and RNG dependencies explicit to prevent invalid CSE/rematerialization; specify seed/state, scaling and mask reuse. | Candidate; execution unverified | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [D](https://github.com/yunskim/japchae/blob/510c31b5fe3d4bf6ca0c6fa9aeae8556ca134607/documents/decisions.md) |
| `flatten` | Verb | Flatten specified cell axes into a feature axis. | Connect spatial layers to linear inputs and preserve logical reindexing; specify J atom order/axes, without guaranteeing zero-copy. | Candidate; execution unverified | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py) |
| `relu` | Verb | Apply element-wise rectification. | Provide a small map/fusion validation target; compare domain, NaNs, signed zero and derivative-boundary policy against a reference. | Candidate; execution unverified | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py) |
| `gelu` | Verb | Apply Gaussian-error-based activation. | Use an explicit exact/approximate numeric contract for backend error checks; the spelling alone does not select an approximation. | Candidate; execution unverified | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py) |
| `softmax` | Verb | Normalize exponentials along a specified axis. | Expose map plus reduction; specify stabilization, axes, empties, infinities/NaNs without declaring a fixed fusion boundary. | Candidate; execution unverified | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py) |
| `scaled_dot_product_attn` | Verb | Compute scaled dot-product attention over Q, K and V. | Retain contraction–softmax–contraction structure for tiled/library routes; specify masks, scale, axes and dtype. This is not FlashAttention support. | Candidate; execution unverified | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py) |
| `crossentropy` | Verb (dyad) | Compute cross-entropy from labels and predictions. | Expose loss reduction and input derivatives; first specify logits/probabilities, label format, mean/sum and log stability. | Candidate; execution unverified | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| `conv_backward / depthwise_conv_backward / linear_backward / bn_backward / ln_backward` | Adverb (prototype) | Prototype factories creating parameterized backward verbs. | Keep training computation first-class; later design separates per-input data/parameter VJPs instead of hiding outputs/state inside one backward spelling. | Candidate; execution unverified | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| `adam` | Adverb | Build an Adam update verb from learning-rate/moment parameters. | Expose weight, gradient, moments and step dependencies/versions as explicit StateResource; in-place reuse, fusion and scheduling are later decisions. | Candidate; execution unverified | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| `cast_f32 / cast_*` | Adverb | Derive a verb incorporating conversion of its operand result. | Preserve producer-associated mixed-precision opportunities; specify rounding, overflow and NaNs. Adverb POS alone cannot force fusion/in-place. | Candidate; execution unverified | [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| `to_f32 / to_*` | Verb | Convert an input array to an explicit dtype. | Represent an independent conversion node; this is neither device transfer nor a forced separate kernel. Supported families and numeric rules remain to be specified. | Candidate; execution unverified | [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| `emit / emit_acc` | Adverb (research) | Derive a verb writing/accumulating a side output to a resource. | Preserve gradients, loss or statistics with explicit destinations/conflicts/order. Later review leaves standalone surface operation versus Write/Accumulate open. | Research/open; not implemented | [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md), [F](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/flow_storage_scope_and_compiler_positioning.md) |
| `cp` | Adverb (research) | Mark checkpoint requirements/candidates for later consumers. | Analyze backward-residual lifetime and storage/recompute trade-offs; distinguish required retention, candidate storage and materialization. Surface adoption remains open. | Research/open; not implemented | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md), [F](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/flow_storage_scope_and_compiler_positioning.md) |
| `store` | Adverb (research) | Derive storage of a computed value or control mapping into a resource. | Let distant forward/backward consumers reuse the same decision; specify destination, version, returned value and effect order. | Research/open; not implemented | [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md), [W](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/flow_storage_model_review_and_open_questions.md) |
| `load` | Adverb form in research sketches | Form a derived accessor for stored values/mappings in research sketches. | Reuse routing decisions at the same version instead of recomputing them; specify resource identity, read version and uninitialized errors. Full POS/operand schema remains open. | Research/open; not implemented | [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md), [W](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/flow_storage_model_review_and_open_questions.md) |
| `with` | Conjunction | Attach a typed semantic annotation/contract to a base entity. | Validate adjoint/resource/numeric/storage information with computation identity. Adopted in RustJ design, not completed runtime support; exclude device/tile/register/layout policy. | Design adopted; not implemented | [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| `adjoint relation registration` | Registration API; not a computational conjunction | Register relations between a forward entity and per-input adjoint/VJP entities. | Validate shape/type, linearization, residuals and shared resources, and supply future AD rules; not inverse/obverse or a parallel annotation. API spelling/schema remain to be settled. | Research/open; not implemented | [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| `grad` | Adverb (historical) | Historical training mode emitting weight gradients to a global buffer. | Records the earlier accumulation/separate-optimizer intent; fixed offsets and inverse-slot training are not adopted. Redesign using explicit write/accumulate contracts. | Historical; not directly adopted | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [T](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/README.md) |
| `consume` | Adverb (historical) | Historical mode for immediate optimizer consumption of gradients. | Retain reduced-gradient-storage as a candidate; do not make immediate weight updates default. First prove legality for readers of the old weight version and effects. | Historical; not directly adopted | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [T](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/README.md) |

### 5.3.1 Research cases and the core-J boundary

These later research cases are not completed entries in the prototype registry above.

| Research name/role | Function | Rationale and unresolved contract | Sources |
|---|---|---|---|
| `mp / MatMul` | Matrix contraction of two inputs. | A contract/lowering alias candidate with a standard-J reference such as `+/ .*`, not required new syntax. | [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| `FFT / IFFT` | Frequency transform and its inverse. | An exploratory array-language case; not registered until normalization, axes, complex dtype and adjoint relations are specified. | [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| `Embedding / input-specific *_adjoint or *_grad_*` | Indexed lookup and per-input VJPs. | Test cases with parameter gradients but no index derivative; require differentiability and scatter/accumulate contracts. | [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| `decision / capacity / weighted_sum` | MoE routing decisions, capacity handling and weighted combination. | Research role/example names; `@.` is standard J, not an extension. Set names/POS only after drop/pad, shared token/probability mapping and reassembly contracts. | [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |

`+`, `/`, `\`, `"`, `@:`, `[`, `]`, `@.`, `|:`, `:.`, `^:` are core J vocabulary, not duplicate extension registrations. Core and extension-derived operations share semantic/capability/lowering principles.

### 5.3.2 Applying later research to RustJ

1. **`emit`/`cp` remain open.** Later scope §8 revisits surface primitives versus core effects, annotations and planner concepts; earlier “basic vocabulary” does not establish adoption.
2. **`with` is adopted in current design.** It attaches only typed semantic contracts as an ordinary-name conjunction. Historical optimizer/gerund bundles do not automatically define the current operand schema.
3. **Adjoint is a relation.** Nonlinear VJPs require a linearization point/residual contract. Do not replace inverse semantics of `:.`/`^:_1`. In `(loss_adjoint [ (loss emit))`, the outer fork supplies the parallel candidate; if `emit` is bound as an adverb, `(loss emit)` is modifier application. It is a hook only if both names are verbs.
4. **Dtype/POS do not fix physical execution.** Historical forced-fusion adverbs, narrowing-in-place and widening-new-buffer rules are not current invariants. Choose realization after numeric, alias, use/liveness and target/cost analysis.
5. **State is explicit.** Do not copy hidden weights/moments, fixed offsets, barriers/streams or register counts into primitive identity. Keep `ValueId`, `StateResource` and `BufferId` separate.

### 5.3.3 Implementation status and checklist

Code inspection baseline: documentation checkout `89b87b8`. `src/primitive.rs` provides `ExtensionPrimitive`/`PrimitiveResolver::resolve_extension_binding`, and `src/runtime.rs` provides parser name-binding integration. These seams and test extension handles do not implement the listed NN/effect contracts or kernels. Existing `tests/semantic.rs::unknown_contracts_are_barriers` covers conservative unknown contracts for `conv` and `with`; it was read, not rerun in this documentation change.

- [x] Check latest HEADs of all four repositories and record functions, rationale, POS and adoption status with pinned sources.
- [x] Include missing forward/backward families, historical `grad`/`consume`, and later cast/storage/registration vocabulary.
- [x] Integrate Korean canonical and English mirror.
- [ ] Specify each candidate’s parameter schema, valence/innate rank, shape/dtype/numeric rules and reference.
- [ ] Verify effects/errors/alias, StateResource/version and access/reduction contracts.
- [ ] Connect at least one verified execution/lowering route before marking individual support complete.
- [ ] On native Windows, compare ordinary name rebinding/locale/POS, empty/exceptional inputs, values/shapes/dtypes/errors/effect order.

Small validation candidates are `relu` → `linear`/`flatten` → `conv`/`avgpool2d`. Existing M1–M6/frontend migration order remains authoritative; this inventory does not require all candidates as new prerequisites. Training/AD/state families come later; actual CUDA implementation remains deferred.

### 5.3.4 Pre-execution input information — ``with (X`Y)`` candidate and metadata validation

**User intent:** investigate how input information enables analysis/optimization before execution. with/gerunds are optional candidates, not a required or finalized surface schema. Facts can come from existing noun metadata, an external API/catalog, file/dataset headers/schemas, or optional annotations and converge at one analysis boundary. Header access remains actual I/O with effects. Some topology analysis needs no shapes; dtype/rank/extents/dimension relations enable more precise legality/resource analysis. Metadata does not prove content-dependent conditions. Frontend priorities and deferred optimizer execution remain unchanged.

**Source review (2026-10-04):** the pinned jaxa-analyzer architecture document at `7275d5ba`, §7.7-2 entry-point item, proposes `X =: 2 3 source with fp16`, the corresponding Y definition and ``run ... with (X`Y)``. Nearby items discuss annotation/adjoint/optimizer bundles and warn against mixing dtype, hardware and tile policies. These are research proposals, not completed execution support. Interpret X/Y as candidate source entities carrying contracts, not entire training arrays. Standard J tie packages verb atomic representations but concatenates noun operands; applying backtick to training nouns does not automatically create external-input handles. [Original JAXA proposal](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md#L861), [J tie](https://www.jsoftware.com/help/dictionary/d610.htm).

**RustJ design example, not implemented extension syntax:**

```j
X =: 2 3 source with fp16
Y =: 2 3 source with fp16
run =: graph with (X`Y)
```

Retain the original surface form using ordinary source/fp16/with bindings. Introduce no spelling-specific frontend rules. A with semantic adapter must preserve source gerund nouns, order, spans and NameRefs and inspect only registered descriptor/annotation capabilities. It must not invoke arbitrary verbs or load data to discover metadata. Unknown capabilities are analysis boundaries, not invalid J syntax. Real source invocation retains its I/O/state contract.

If with is selected, candidate descriptors carry schema version, input identity/slot, dtype/rank/shape facts and provenance. A dyadic input bundle explicitly validates left/right slots while preserving X/Y order. Two entries alone do not identify an input bundle; adjoint/optimizer bundles need distinct typed roles. Specializing an observed function target requires ordinary NAME late-binding semantics and separate runtime witnesses. Real noun inputs retain parser-time snapshots, not function-like delayed lookup.

The [input information inventory in §5.3.5](#preexecution-input-information) specifies categories, requirements and implementation boundaries.

Bind real nouns batch by batch. IR retains ReadNoun and facts rather than embedding dataset contents as literals. Matching batches may share an analysis structure; executable-plan reuse also requires function/effect/runtime guards. Parameters changing each step are not compile constants. Do not indiscriminately key compilation on contents, pointers or runtime noun versions. Design cache keys around graph/contracts/schema and required semantic/target specialization, checking actual NAME witnesses separately. Data-dependent result extents remain unknown/guarded. Training pipeline/optimizer execution is outside this stage.

**Implemented seam:** StaticAnalysis::validate_noun_inputs() checks used noun declarations against live borrowed Values. It reads metadata without reading, cloning or retaining atoms. Type/rank/extent mismatches return Domain/Rank/Length respectively; missing names return Value and duplicate/extra inputs return Domain. Unknown facts add no constraints. Catalog revisions are not runtime witnesses. This API neither validates function bindings/effects/alias/device/value bounds nor proves execution safety; it is not automatically inserted into execution. The with adapter remains unimplemented.

Checklist, preserving existing frontend priorities:

- [x] **WI0** Locate the original proposal and J tie semantics; integrate input/adjoint/numeric/physical boundaries in canonical documents.
- [x] **WI1** Add live noun metadata validation to data-free analysis reports. Four Windows regressions cover different batches; type/rank/extent and missing/duplicate/extra rejection; unknown/partial facts, empty arrays, unchanged function boundaries and payload pointer/refcount identity.
- [ ] **WI2 (optional candidate)** Define registered source/input descriptors and typed bundles; implement a with adapter preserving gerund source/order/lookup timing/provenance. Distinguish noun operands and malformed descriptors.
- [ ] **WI3** Independently of surface syntax, connect input facts to the static catalog/ReadNoun; add C/frontend comparisons for late binding, rebinding/POS changes and unknown capabilities. Retain inference without annotations.
- [ ] **WI4** Connect actual input binding/lifetime and guard failures before effects, checking batch replacement, partial failure and conflicting contracts. External buffers/mmap/device adapters remain separate physical work.
- [ ] **WI5** Add symbolic dimensions/equalities, cache preconditions and training state contracts later. Optimizer/CUDA/AD execution remains deferred.

Comparisons: [JAX abstract evaluation](https://docs.jax.dev/en/latest/601/jax-primitives.html) analyzes shapes/types without contents; [PyTorch export](https://docs.pytorch.org/docs/stable/export) distinguishes input/parameter signatures and dynamic shape constraints. These inform descriptor analysis without replacing J NAME timing.

<a id="preexecution-input-information"></a>

### 5.3.5 Pre-execution input information inventory (2026-10-04)

**Requirements depend on the optimization.** Do not require every declaration for full J execution. Known graph/callable structure permits some topology analysis without shapes. Shape/type-dependent lowering needs relevant facts or runtime conditions. At minimum link each graph input to its noun POS, identity/slot, known-versus-unknown state and fact provenance/scope; refine dtype/rank/shape when available. This schema is independent of with and can receive noun metadata, external API/catalog facts, file headers or optional annotations.

| ID / category | Information and examples | Analysis/optimization use | Acquisition/checking and current boundary |
|---|---|---|---|
| IN0 Identity | Graph input ID, NAME/slot, schema revision, noun snapshot timing | Connect edges and detect wrong input wiring | Catalog/call mapping; catalog versions are not runtime NAME witnesses. Used-input catalog and missing/duplicate/extra validation exist |
| IN1 Element type | J logical dtype/numeric category; precise external fp16/fp32 connected to explicit conversion/encoding | Type propagation, legal operations/kernels, storage size | Noun/header/API. TypeFact and dtype validation exist; CPU Value::Float uses f64, not implemented fp16/fp32 NN inputs |
| IN2 Rank/shape | rank 2, [256,784], scalar/empty/unknown | Rank/cell split, agreement, result shape/logical extents | Derive rank/count from shape. Concrete/unknown facts and rank/shape validation exist. Preserve J prefix agreement |
| IN3 Dimension relations/bounds | X=[B,K], Y=[B,N], W=[K,N]; shared B, 0≤B≤1024 | Global shape relations, dynamic sizes, memory bounds/guards | Shared symbolic scope/constraints and runtime metadata checks. Unknown differs from equal symbols; do not exclude 0/1/empty by default. Symbolic facts/guards pending |
| IN4 Logical representation/nesting | Dense/axis-sparse/boxed; sparse axes/fill schema/stored count and known child schemas | Sparse/boxed legality, assembly, cost/extent | Separate J-visible facts from encoding. Some representation facts exist, but WI1 does not check these. Samples are not structural proofs |
| IN5 Runtime values/small constants | Data/weights remain inputs; some axis/rank/window constructor nouns need actual values | Constant folding, modifier construction, relevant specialization | Supply only required constant values/witnesses. Do not embed datasets/weights or hash contents into general cache keys. Value-dependent constructor boundaries remain. [JAX static args](https://docs.jax.dev/en/latest/aot.html) |
| IN6 Roles/effects | Batch/data/label/parameter/gradient/state, reads/writes/accumulates, step dependencies, optional AD targets | Updates, training/AD graphs, effect legality | Roles do not follow from dtype or uppercase names. Connect actual contracts and StateResources; training/AD pending. [PyTorch signatures](https://docs.pytorch.org/docs/main/user_guide/torch_compiler/export/api_reference.html) |
| IN7 Value properties | Bounds, finite/nonnegative/sorted/unique, valid indices | Check elimination, search specialization, content-dependent shapes | Proof/explicit contract/runtime checks. Sample statistics are cost hints, not legality proofs; full checks may cost O(N). WI1 does not inspect contents |
| IN8 Numeric policy | Casts, accumulator precision, overflow/promotion, cct/fit, NaN/Inf, reassociation/determinism | Legal fusion/reduction changes and mixed precision | Acquire from operation/semantic policy, not input dtype alone. An fp16 declaration does not change J numeric/error semantics |
| IN9 Lifetime/ownership/alias | External owner/release, overlap, read-only/mutation, reuse/donation after calls | Buffer reuse/in-place, external import, retained memory | Adapter/runtime ownership evidence plus liveness. Separate logical identity/BufferId. WI1 does not validate these. [JAX donation](https://docs.jax.dev/en/latest/buffer_donation.html) |
| IN10 Physical representation/target | Device, encoding, strides/offset/layout/alignment, transfer/sharding, capabilities | Concrete kernels/routes/schedules and byte/transfer costs | Adapter and TargetProfile, outside semantic with. Guard any specialized observed layout. [Tensor guards](https://docs.pytorch.org/docs/main/user_guide/torch_compiler/torch.compiler_dynamo_overview.html) |
| IN11 Provenance/validity | Unknown/observed/inferred/declared/proven, scope/version, runtime guard/conflicts | Track preconditions and prevent stale specialization/cache reuse | Associate witnesses/provenance. Do not promote samples/observations into permanent constants. Distinguish NAME witnesses from noun snapshots; current catalog revisions are analysis-local |

IN0–IN5 describe input facts, IN6–IN8 connect operation/state contracts, IN9–IN10 belong to runtime/physical boundaries, and IN11 applies to all facts. Do not put call-dependent input facts into immutable FunctionEntity identity. Keep proofs in analyses and physical facts downstream. Optional axis labels such as batch/channel/feature help interpret descriptors but do not replace J rank/axis semantics. Device placement may be an input fact; tile/kernel selection is a derived plan.

| Optimization goal | Relevant information | When unknown |
|---|---|---|
| Topology/common-input/branch-join candidates | Graph/callable identity, inputs, source provenance | Analyze known structure and retain callable boundaries |
| Fusion/reordering legality | Structure plus operation effects/errors/numeric/alias and required shape/type relations | Do not reorder based on topology alone; retain unapplied candidates without proof/guards |
| Logical memory/resources | Type/rank/shape, nesting/sparse schema, use-def/liveness | Report expressions/bounds/unknowns, not logical extent sums as peak allocation |
| Kernel/SIMD/GPU scheduling | Legality plus representation/target capabilities | Use guarded or verified supported routes, never invented layouts/devices |
| Buffer reuse/in-place | Representation, ownership/alias/lifetime and liveness | Conservatively retain shared/external ownership; permission is not guaranteed reuse |

Conceptual training-step specification, not implemented dtype/symbolic/AD support:

```text
X: data,      float32[B,784]
Y: label,     float32[B,10]
W: parameter,float32[784,10]   // runtime contents change between steps
constraints: X.dim0 = Y.dim0, 0 <= B <= 1024
state: W read -> compute -> explicit update; optimizer state is separate
constants: only required axis/window/rank operands
witness: check metadata and specialized NAME/semantic policy before effects
physical inputs: adapter placement/layout/lifetime, outside this schema
```

Constraints must match the actual program/domain; B=0 follows existing J results/errors. A user-defined batch step can be analyzed without full dataset contents/size. Replacing full-dataset reductions or batch statistics with arbitrary minibatches needs separate equivalence proof. Weights can change despite unchanged contracts. Cache only facts actually used by a specialization. Data-dependent output sizes, such as filtered lengths, remain unknown or require subsequent shape computation.

Acquire facts from existing semantic contracts/literals, prepared noun/header/API metadata, necessary optional declarations, then required proof/runtime guards. Do not require content scans, alias proofs or GPU implementation for the base input schema. A specialization guard miss is not itself a J semantic error: distinguish supported reanalysis/runtime routing from explicit contract violations. WI1 validator errors are explicit check results, not automatic compiler dispatch policy.

Sources: [JAX AOT](https://docs.jax.dev/en/latest/aot.html) and [symbolic constraints](https://docs.jax.dev/en/latest/export/shape_poly.html), [TensorFlow signatures](https://www.tensorflow.org/guide/function), [TVM symbolic shapes](https://tvm.apache.org/docs/deep_dive/relax/learning.html), [PyTorch signatures](https://docs.pytorch.org/docs/main/user_guide/torch_compiler/export/api_reference.html) and [tensor guards](https://docs.pytorch.org/docs/main/user_guide/torch_compiler/torch.compiler_dynamo_overview.html). IN0–IN11 and their adoption priority are RustJ design decisions.

Checklist:

- [x] **WI0a** Document syntax-independent IN0–IN11, per-optimization requirements and framework sources.
- [ ] **WI3a** Verify fact provenance/refinement and unknown/conflicting inputs; symbolic equalities/bounds belong with WI5.
- [ ] **WI4a** Distinguish guard misses/contract violations and supported route selection; connect pre-effect checks, covering scalar/empty/boxed/sparse, changing batches/weights and NAME changes.

This is a design-document update; it claims no runtime/optimizer implementation or new validation gate after WI1.

#### Design influences and sources — framework ideas versus RustJ decisions

This table identifies **concepts that influenced the design**, not frameworks implementing RustJ's exact conditions. The conditions below specify future external-input/execution adapters. Retain J NAME snapshot/late-binding, scalar/empty, sparse/boxed and observable error semantics.

| Framework/research influence | Specific idea | Inventory | RustJ adoption and differences |
|---|---|---|---|
| JAX | Data-free shape/dtype descriptors and value-requiring static arguments | IN1–IN3, IN5 | Separate arrays from small compile-time constants; no fabricated input atoms. [ShapeDtypeStruct](https://docs.jax.dev/en/latest/_autosummary/jax.ShapeDtypeStruct.html), [AOT](https://docs.jax.dev/en/latest/aot.html) |
| JAX shape-polymorphic export | Symbolic dimensions/equality/bounds in a shared scope | IN3, IN11 | Distinguish shared B from independent unknowns; preserve scope/provenance and J empty extents. [Symbolic shapes](https://docs.jax.dev/en/latest/export/shape_poly.html) |
| TensorFlow | TensorSpec/input_signature, wildcard dimensions, incompatible-input checks | IN1–IN3, IN11 | Permit partial facts; distinguish wildcards from cross-input equalities; no mandatory full-J declarations. [tf.function](https://www.tensorflow.org/guide/function) |
| TVM Relax | Propagate shape/dtype/symbolic relations across inputs/operators | IN1–IN3, IN10 | Separate logical relations from lowering; do not import TVM agreement semantics into J. [Relax](https://tvm.apache.org/docs/deep_dive/relax/learning.html) |
| PyTorch Dynamo/export | Guards on observed metadata and input/parameter/state signatures | IN0–IN3, IN6, IN10–IN11 | Check specialized dtype/rank/size/layout; retain separate J NAME witnesses and noun snapshot timing. [Guards](https://docs.pytorch.org/docs/main/user_guide/torch_compiler/torch.compiler_dynamo_overview.html), [signatures](https://docs.pytorch.org/docs/main/user_guide/torch_compiler/export/api_reference.html) |
| JAX buffer donation | Permission to reuse unneeded input storage for compatible outputs | IN9 | Explicit transfer with unusable old handles; permission does not guarantee actual reuse. [Donation](https://docs.jax.dev/en/latest/buffer_donation.html) |
| MLIR One-Shot Bufferize | Use-def/alias, read-after-write conflicts and writability guide in-place decisions | IN9–IN10 | Prove no overwrite of live J snapshots/external aliases; otherwise use separate output/copy or supported-route boundary. [Bufferization](https://mlir.llvm.org/docs/Bufferization/) |
| Historical JAXA research | Source contracts tied to entry graphs using with/gerunds | IN0–IN3, IN6 | Optional surface candidate with registered capabilities/ordinary bindings, excluding physical policies. [Proposal](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md#L873) |

#### Input metadata conditions (M0–M8)

Apply these when using metadata as a justified optimization premise. Dtype/rank/shape alone do not prove buffer safety, function semantics or content immutability. Physical access checks are separate adapter obligations.

| Condition | Contract | Unknown/failure handling and current boundary |
|---|---|---|
| M0 Mapping | Connect each used noun ID/slot/name; distinguish missing, duplicate and wrong-POS inputs | WI1 checks missing/duplicate/extra nouns; function/locale reuse proofs remain separate |
| M1 Logical consistency | Known rank equals shape length; nonnegative valid extents; scalar rank 0/one atom, zero extent/no atoms; overflow-safe counts | Existing declaration/Value validation applies; do not silently normalize invalid/empty shapes |
| M2 Dtype/encoding | Verify external dtype, byte order and encoding against logical dtype; conversions are explicit and preserve numeric semantics | Never reinterpret fp32 bytes as f64 slices; WI1 checks logical Value dtype only; external import pending |
| M3 Storage bounds | Before direct access verify dense capacity or strided offset/stride/addressable bounds without overflow, plus alignment/device/backend requirements | Shapes alone do not prove strided safety. Distinguish CPU affine foundations from future external adapters; outside WI1 |
| M4 Unknown/symbolic | Unknown does not mean constant/positive/no-alias; use only justified same-scope symbolic conditions | Symbolic guards pending with WI5; retain conditional reports/supported runtime shape boundaries |
| M5 Provenance/validity | Attach source/proof/scope; headers/catalogs do not guarantee buffer existence, lifetime or freshness | Bind premises to actual Value/buffer generation or equivalent ownership lease, not catalog revision; integration pending |
| M6 Check-to-use stability | Checked metadata/NAME/storage must match execution; control external reshape/reallocation/mutation with snapshot/lease/synchronization | Check relevant facts on each new batch; reject specialization/import if stability is unresolved. WI1 success does not extend across later calls |
| M7 Partial/content facts | Refine only known facts; finite/sorted/range require separate proof/check; samples are not legality evidence | Keep data-free analysis and unknown boxed/sparse children/structure. WI1 does not check atoms/representation |
| M8 Cache/guard failure | Track only used metadata/function/semantic premises; changing batch/weight contents or pointers alone need not recompile | Distinguish contract errors from specialization misses; no automatic replay after effects. Supported route/reanalysis needs real capability; dispatcher pending |

#### Ownership and input-storage conditions (O0–O8)

| Condition | Contract | Permitted actions and limitations |
|---|---|---|
| O0 Import mode | Distinguish owned transfer, shared immutable and external borrow; raw pointers/Arc counts alone do not prove external exclusivity | Even unknown-owner read-only candidates require lifetime/synchronization evidence |
| O1 Lifetime | Retain owner/lease until all reads/writes/transfers complete; returned views extend backing lifetime | Future GPU/async paths retain until actual completion, not enqueue; these executors are pending |
| O2 External mutation | Data exposed as a J noun snapshot must not silently change through producer writes | Immutable lease, synchronized snapshot copy or explicit state/effects; a read-only wrapper does not stop external writers |
| O3 Alias/overlap | Consider input views, external aliases, live global/boxed snapshots and graph consumers | Without proof use out-of-place or verified overlap-safe implementations; user no-alias assertions alone cannot establish safe Rust references |
| O4 Writability | Backing must be writable and semantic writes authorized | Never overwrite readonly/mmap/protected backing; physical allocation choices differ from observable state writes |
| O5 Donation/reuse | Explicit transfer permission, no live old-value reads, ownership/alias proof, compatible output representation and preserved J errors/effects | Old handles/aliases must become unusable after transfer. Low observed refcounts do not justify consuming shared nouns; output/copy may still be necessary |
| O6 Release responsibility | Specify transfer/borrow boundaries and release owner; return/release each transferred owner reference exactly once | Handle validation/partial import/call failures and final output drop without leaks/double release; metadata-only declarations own no actual buffer |
| O7 Zero-copy preconditions | Direct import requires lifetime/mutation/alias/alignment/encoding/layout/backend compatibility | Otherwise explicitly choose supported value-preserving copy/conversion or report import boundary. Copies also need writer synchronization; no universal zero-copy guarantee |
| O8 Failure/consumption timing | Complete possible validation before calls/effects/transfer; specify transfer commit and post-failure handle validity | Distinguish pre-start failures from consumed/effectful failures; no generic rollback/replay promise. Preserve J assignment transactions/live snapshots |

M/O describe **adopted contracts and pending implementation requirements**. StaticAnalysis::validate_noun_inputs() currently checks mapping and borrowed Value dtype/rank/shape only. External import/leases, encoding/stride capacity, alias, donation and automatic dispatch are not implemented. Ownership remains downstream; semantic with grants no unsafe pointer-access permission.

- [x] **WI0b** Record per-item influences/sources and M0–M8/O0–O8 in both canonical documents.
- [ ] **WI4b** When wiring adapters/guards, validate stale metadata, external mutation, overlap, readonly backing, bad capacity, use after donation, failure release and view lifetimes. Async/device completion tests wait for those backends.

#### Static analysis first: derive facts from contracts without real arrays

**User requirement:** static analysis is the foundation for pre-execution optimization. Do not require every M/O fact to come exclusively from runtime buffer checks. Analyze noun schemas/facts and preserved computation structure without actual nouns. Metadata acquisition through an external API/header may involve I/O; distinguish that from subsequent data-free static graph analysis. Do not default to invoking source verbs or creating dummy/training arrays to discover shapes. with is an optional information carrier.

```text
Preserved J graph + input schemas + operation/ownership contracts
    -> static propagation / constraints / use-def / liveness
    -> reports and candidate legality premises
    -> residual obligations only for required unproven premises
    -> actual input binding + required guards/leases
    -> supported execution route
```

These are design stages, not completed optimizer/solver/dispatcher support. Data-free reports describe graph structure, known/unknown facts, extents, resource expressions/bounds, live values and unresolved conditions. Partial analysis remains possible with unknown facts. Keep unknown separate from unreachable and compile-route limits separate from invalid J.

| Facts/conditions | Before execution | Possible residual obligations at binding |
|---|---|---|
| IN0 / M0 | Schema/syntax determine input slots/POS, edges and used inputs | Supplied mapping and ordinary NAME binding/POS/version witnesses |
| IN1–IN3 / M1,M4 | Declared dtype/rank/shape, constants and shape rules derive results/relations; statically check known constraints | Actual metadata agrees with declarations and unproven equalities/bounds |
| IN4 / M7 | Known nested/sparse schemas support applicability/assembly analysis | Actual child schemas/stored structure not already proven; content-dependent shapes stay unknown |
| IN5–IN8 | Explicit configuration and numeric/effect/state contracts support legality/dependencies | Required actual function/policy identity and separate contents conditions |
| IN9 / O1–O5 | Closed-graph use-def/liveness identifies old-value reads/internal alias conflicts; use ownership contracts and managed Rust borrow/lifetime evidence | Adapter guarantees external leases/alias/mutation/writability/exclusive transfer; declarations alone cannot prove exclusivity |
| IN10 / M2,M3,O7 | Known target/representation contracts identify candidates and encoding requirements | Backing capacity/strides/device/alignment/lifetime. Proven static physical conditions need not be repeatedly checked |
| IN11 / M5,M6,M8,O6,O8 | Track provenance/scopes, boundary proof obligations, cache preconditions and ownership transfer/release flow | Unresolved witness validity and check-to-use stability; no replay after effects |

Distinguish the statically provable absence of later internal reads from external exclusive ownership. The latter requires adapter guarantees or managed ownership/lifetime evidence. Do not assume general runtime alias checking is always cheap or sufficient. Without proof retain out-of-place/supported-route boundaries. Compile-time proofs and residual guards may coexist.

Concrete shapes permit exact logical extents in supported cases; symbolic shapes permit expressions/proven bounds; unknown relations do not justify exact counts. Symbolic analysis is still static analysis. Without concrete layout/schedule do not assert physical peak allocation or performance. Dataset atoms need not be retained in the compiler proportional to dataset size.

**Current boundary:** StaticAnalyzer::declare_noun()/analyze() produce supported-syntax concrete/unknown-fact reports without arrays. WI1 validate_noun_inputs() is the later compatibility-check seam. Arbitrary per-dimension symbols/solvers, full ownership proof obligations, external leases, optimizer transforms and dispatch remain pending. Preserve frontend semantics without requiring data execution for static analysis.

- [x] **WI0c** Classify static facts/proofs versus residual obligations and the static roles of M/O.
- [ ] **WI3b** Report metadata-only shape/resource/use-def results and unresolved conditions. Cover no input allocation/kernel execution, preserved unknowns, compile-time constraint conflicts and partial-fact refinement.
- [ ] **WI4c** Connect static proofs and residual guards/adapter guarantees at execution boundaries without making analysis runtime-only. State ownership/lifetime proof scopes and unresolved external aliases.

#### Framework comparison for pre-execution input information (2026-10-04)

| Framework | Input information before tensor execution | Shape changes/reuse | RustJ lesson and boundary |
|---|---|---|---|
| JAX | ShapeDtypeStruct supports trace/lower/compile without actual atoms; static arguments need actual values | AOT artifacts specialize to signatures and reject mismatches; shape-polymorphic export is a separate route | Separate metadata from real nouns and distinguish value constants from array inputs. [AOT](https://docs.jax.dev/en/latest/aot.html), [shape polymorphism](https://docs.jax.dev/en/latest/export/shape_poly.html) |
| PyTorch | torch.compile uses observed metadata/guards; export records operations using data-free FakeTensors/Proxies | Compile guard failure may recapture/recompile; dynamic shapes use symbolic sizes/constraints; export has its own constraints | Separate observed facts from validity checks. Consider optional static-to-dynamic generalization while preserving J NAME witnesses. [Export model](https://docs.pytorch.org/docs/main/user_guide/torch_compiler/export/programming_model.html), [dynamic shapes](https://docs.pytorch.org/docs/main/user_guide/torch_compiler/torch.compiler_dynamic_shapes.html), [guards](https://github.com/pytorch/pytorch/blob/main/docs/source/user_guide/torch_compiler/torch.compiler_dynamo_overview.md) |
| TensorFlow | TensorSpec/input_signature; get_concrete_function(TensorSpec(...)) separates tracing from tensor graph execution | None dimensions permit trace reuse; incompatible fixed signatures reject inputs | Preserve partially unknown extents. Wildcards alone do not relate dimensions across inputs. [tf.function](https://www.tensorflow.org/guide/function) |
| TVM Relax | Input shape/dtype types and symbolic n propagate across operators/functions | Symbolic shape relations remain in IR; unknown shapes may use runtime shape expressions | Distinguish unknown from X.batch = Y.batch in a future abstract domain. [Relax](https://tvm.apache.org/docs/deep_dive/relax/learning.html), [shape API](https://tvm.apache.org/docs/reference/api/python/relax/relax.html) |

Tracing can execute frontend/host code; it does not mean no user code executes. Descriptors do not automatically prove content-dependent sizes, branches or errors. RustJ analyzes preserved graphs and known facts while retaining unknowns and J effect/name timing. Topology opportunities may exist without shape/type information; richer facts refine legality and logical resources. Actual optimization passes remain deferred.

**WI1 gate:** native Windows default/portable each **396 passed / 17 ignored**; fmt/clippy/build pass. Python **27 passed**. Each j64/AVX2 runtime route: **4,942 cases / 4,938 passed / 4 existing runtime boundaries / 0 failed**; stages **10,163 checks / 0 failed**; words **6,618 cases / 0 failed**. Keep 164 capture-graph and 2 static boundaries separate. Recheck all ten report binary/source hashes. This does not complete with execution, symbolic dimensions, training/AD, CUDA or optimizer execution. Full upstream/private C trace and ignored definition acceptance remain unverified; Linux/GitHub CI were not run. The subsequent ordinary-reference step resolves those NAME restrictions; implicit locatives/full operator scope remain pending; straight-line calls are supported in the operator-call step below. The with adapter is not a mandatory frontend prerequisite.

<a id="array-compiler-static-analysis"></a>

### 5.3.6 Static analysis targets in array compilers and framework comparison (2026-10-04)

**Targets include intermediate values, operations, uses, control flow and storage, as well as inputs.** IN0–IN11 in §5.3.5 seed analysis; SA0–SA8 below describe questions derived from those inputs and the preserved graph/contracts. Static analysis produces facts, constraints, proofs and unresolved obligations. Fusion, DCE, tiling and buffer placement are transformations consuming analysis. XLA explicitly distinguishes analyses from HLO transformation passes. [XLA analyses](https://openxla.org/xla/hlo_passes)

| ID / target | Static questions and outputs | Optimization premise / unresolved boundary |
|---|---|---|
| SA0 — type/rank/shape relations | Intermediate dtypes, ranks, extents, dimension equalities/bounds, possible emptiness | Compatibility, size expressions, specialization. Symbolic information remains static; unresolved relations stay unknown. [JAX symbolic shapes](https://docs.jax.dev/en/latest/export/shape_poly.html) |
| SA1 — constants/abstract value properties | Small compile-time operands and their dependency closure; proven ranges/predicates | Constant evaluation/branch simplification. Do not inspect training-array contents to obtain facts; sorted/unique/finite declarations require evidence. [TVM compile-time analysis](https://tvm.apache.org/docs/reference/api/python/relax/analysis.html#tvm.relax.analysis.computable_at_compile_time) |
| SA2 — use-def/fan-out/result demand | Producers, consumers, shared inputs, unused results, last uses | Sharing/fusion/elimination candidates and retained intermediates. Effects/errors may prevent removal despite unused results. [XLA dataflow](https://openxla.org/xla/hlo_passes) |
| SA3 — iteration/index/access dependence | Iteration domains, input/output index maps, parallel/reduction axes, read/write conflicts | Tiling/vectorization/parallelism legality. Indirect gather/scatter and nonaffine accesses require further facts. [MLIR Linalg](https://mlir.llvm.org/docs/Dialects/Linalg/), [Affine](https://mlir.llvm.org/docs/Dialects/Affine/) |
| SA4 — control/calls/effects/errors | Branch/region/call dependencies, state reads/writes, order and speculation safety | Motion/elimination/parallel execution premises. Effect-free does not imply safe speculation; preserve J errors rather than treating them as UB. [MLIR effects/speculation](https://mlir.llvm.org/docs/Rationale/SideEffectsAndSpeculation/) |
| SA5 — alias/mutation/ownership | Shared backing, old-value reads after overwrite, escape and external ownership obligations | In-place/reuse/copy premises. Internal no-later-read does not prove external exclusivity. [MLIR Bufferization](https://mlir.llvm.org/docs/Bufferization/) |
| SA6 — lifetime/materialization/memory | Live ranges, required intermediates, logical size expressions, live bytes under an assumed order | Materialization/recomputation/reuse opportunities. Separate graph-order estimates from physical peak after scheduling/layout. [XLA scheduling/buffers](https://openxla.org/xla/hlo_to_thunks) |
| SA7 — representation/layout/target suitability | Logical reshape/index relations, supported dtype/representation/access model; downstream stride/device/target constraints | View/copy and kernel-route premises. Logical reshape equivalence alone does not prove external zero-copy. [TVM reshape analysis](https://tvm.apache.org/docs/reference/api/python/relax/analysis.html#tvm.relax.analysis.has_reshape_pattern) |
| SA8 — resources/cost/IR consistency | Operation/transfer/storage expressions, definition/use/type/region invariants | Candidate comparison and invalid-IR detection. Estimates are not measured time or exact physical memory. [XLA cost/verifier](https://openxla.org/xla/hlo_passes) |

#### How frameworks address the same questions

These are different layers: JAX/PyTorch/TensorFlow include graph acquisition and input constraints, XLA is a backend, and MLIR supplies IR infrastructure. This compares verified methods and boundaries, not rankings or exhaustive capabilities. Analysis after tracing differs from analysis of RustJ's source-preserved graph.

| Framework / layer | Verified analysis representation/method | Boundary / RustJ lesson |
|---|---|---|
| JAX / frontend | Symbolic dimension expressions and constraints | Some comparisons remain inconclusive. Preserve relations and unresolved conditions. [Shape polymorphism](https://docs.jax.dev/en/latest/export/shape_poly.html) |
| OpenXLA / backend | HLO values/uses, must-alias, cost, verifier; scheduled buffer slices | Lifetime depends on order. Separate semantics, analysis and physical decisions. [Analyses](https://openxla.org/xla/hlo_passes), [buffers](https://openxla.org/xla/hlo_to_thunks) |
| PyTorch / capture frontend | Symbolic sizes, ShapeEnv and guards manage graph shape assumptions | Distinguish guarded specialization from data-dependent control. Keep J NAME witnesses/source semantics separately. [Dynamic shapes](https://docs.pytorch.org/docs/main/user_guide/torch_compiler/torch.compiler_dynamic_shapes.html) |
| TensorFlow / graph frontend | TensorSpec/input_signature and graph functions express dtype/shape assumptions | Wildcards do not express cross-input equalities. Connect input contracts to analysis without requiring tracing. [tf.function](https://www.tensorflow.org/guide/function) |
| TVM Relax / graph + kernel IR | Use-def, compile-time dependencies, reshape-index proofs, allocation estimates before/after planning | estimate_memory_usage sums allocations without control-flow/cross-function accounting and may overestimate; it is not exact peak. [Analysis API](https://tvm.apache.org/docs/reference/api/python/relax/analysis.html) |
| MLIR / structured IR | Linalg maps/iterators, Affine dependence, effects/speculation interfaces, bufferization | Nonaffine access and external aliases require additional evidence. Derive these relations after preserving J rank/cell/assembly semantics. [Linalg](https://mlir.llvm.org/docs/Dialects/Linalg/), [Affine](https://mlir.llvm.org/docs/Dialects/Affine/), [effects](https://mlir.llvm.org/docs/Rationale/SideEffectsAndSpeculation/), [bufferization](https://mlir.llvm.org/docs/Bufferization/) |
| Futhark / array language/compiler | Sized array types; scan/scatter fusion considers producer/consumer and control dependencies | Do not adopt regular-array restrictions as RustJ language limits. Filter result length remains content-dependent. [Language reference](https://futhark.readthedocs.io/en/latest/language-reference.html), [fusion case](https://futhark-lang.org/blog/2026-03-24-scan-scatter-fusion.html) |

**Information from J syntax:** The fork in `(loss_adjoint [ (loss emit))` exposes branches sharing arguments and a result-selection/combination relation. This feeds SA2/SA4; the name adjoint itself does not imply parallelism. Fork syntax alone does not authorize parallel execution: check effects, errors, name lookup and shared-state dependencies. Modifier/train structure can inform SA0/SA3 rank/cell/iteration relations, but concrete index maps and uniform assembly require proofs. These are RustJ design decisions, not claims of J support in those frameworks.

**Current implementation and boundaries:** Schema-driven static_analysis.rs, preserved graphs/concrete-or-unknown GraphFacts in j_graph_ir.rs, uses/liveness/logical extents/explicitly assumed graph-order memory in j_graph_memory.rs, and resource expressions in j_graph_resource.rs provide initial foundations. Canonical Logical IR checks/effects/witnesses/verifier support downstream contracts. Arbitrary per-axis symbolic solving, general loop/index dependence, complete alias/escape/external-ownership proofs, target schedules and physical peak analysis are incomplete. SA rows are not completion claims. Keep optimizer execution/CUDA deferred and prioritize faithful tokenizer/enqueuer/parser semantics and graph preservation.

Future reports connect each fact to source node/span, assumptions, proof scope, known/symbolic/unknown status and residual obligations. Distinguish logical extents, assumed-order live bytes and physical allocation estimates. Do not replace function identity, J prefix agreement, empty/fill/boxed/sparse semantics, numeric policy or observable errors/effects with generic tensor rules without proof.

- [x] **WI0d** Document SA0–SA8, framework methods/boundaries and current implementation with sources.
- [ ] **WI3c** Connect SA categories, proof scopes and unresolved conditions to frontend graph/facts reports; add regressions for supported portions first. Do not require general solvers or physical scheduling before frontend completion.

# Part III — Rank, cells, and array semantics

<a id="logical-physical-array-model"></a>

## 6. Logical and physical arrays — core architecture decision

### 6.1 Logical J array meaning

A J noun remains logically:

```text
type + shape + ordered atoms/value
```

J-visible representation semantics such as boxed and sparse are also part of the logical/semantic model when J can observe them.

Physical representation details such as:

- strides;
- offset;
- tiling;
- alignment;
- address space;
- device placement;
- sharding;
- transfer;
- buffer identity;

do not belong to J noun semantics.

### 6.2 ValueId and BufferId are different

`ValueId` identifies a logical computation result.

`BufferId` identifies a physical storage allocation.

One ValueId may have several physical representations. Conversely, several ValueIds may reuse one BufferId when lifetime, alias and effect conditions have been verified.

A Logical ArrayValue therefore does **not** imply a separate memory buffer.

### 6.3 PhysicalArray

A physical realization may carry:

```text
PhysicalArray
  storage / buffer
  shape
  strides
  offset
  encoding
  placement
  layout
```

The physical layer decides whether a logical view stays virtual, is absorbed by a consumer, or becomes an actual copy/materialization.

#### 6.3.1 Current implementation status and completion criteria

The Logical/Physical Array split is **architecturally decided**, but the runtime representation has not completed the migration yet.

Current status:

- [x] Logical execution `ValueId` and physical `BufferId` are separate identities.
- [x] `PhysicalArray` owns buffer/stride/offset metadata; Logical IR does not.
- [x] Regression coverage demonstrates that the same logical atom order can be realized with different physical backing/stride/offset layouts.
- [x] GraphFacts does not own physical stride/layout/device state.
- [ ] Dense runtime `Value` payloads still contain `CpuStorage` directly. This is transitional and is not the final Logical Array abstraction.
- [ ] Complete an explicit representation adapter/handle boundary between dense logical values and CPU/GPU backend storage.
- [x] Renamed `facts::LayoutFact` to `RepresentationClassFact` and `Facts.layout` to `Facts.representation_class`; `Dense / AxisSparse` remains a J-visible representation class, not a physical layout.

The split is complete when defining or analyzing a logical value no longer requires `CpuStorage`, strides, offsets, devices, or BufferId, and those facts appear only through a selected backend representation.

Accordingly, the current `Value { shape, Data::Int(CpuStorage<_>), ... }` representation is a migration bridge rather than the final semantic boundary.

## 6.4 Rank conjunction vs implicit cell application

The explicit rank conjunction `"` and implicit rank/cell iteration are different concepts.

- `"` is a J conjunction with operands and parser identity.
- implicit rank execution is function-application semantics driven by callable rank and argument rank.

Nested rank boundaries must not be flattened until fill/assembly/error equivalence is proven.

## 6.5 CellApply

Logical `CellApply` represents implicit J cell execution.

It must preserve:

- frame/cell decomposition;
- J prefix frame agreement;
- residual-frame repetition;
- fill/prototype behavior on empty frames;
- result-cell type/shape joining;
- assembly errors;
- observable evaluation/error behavior.

A fixed-shape parallel map may replace it only after the necessary uniformity proofs exist.

---

# Part IV — J Graph IR

## 7. Purpose

J Graph IR preserves algebraic graph structure for reasoning before execution-specific normalization erases useful J structure.

Current schema: **v0.9**. GF2/GF3 add composition and witnessed Scan identity analysis; source Window graphs and execution boundaries remain intact.

It includes:

- explicit applied-operation nodes;
- combinator regions;
- GraphFacts;
- Graph Basis;
- graph hints;
- symbolic operation/resource contracts;
- use counts and liveness;
- algebraic rewrite candidates;
- equivalence witnesses;
- resource-expression provenance.

## 7.1 Graph Basis and Execution Basis are different

RustJ intentionally has two basis layers.

### Graph Basis

Used for JAXA-style graph algebra and access-pattern reasoning.

Current vocabulary includes:

- Elementwise
- CellApply
- Reduce
- Window
- StaticReindex
- DynamicGather
- Search
- Structured

The lower bound is not scalar arithmetic. It should preserve algorithm/access identity far enough for rewrite, locality, fusion, and resource reasoning.

### Execution Basis

Used for executable normalization and lowering.

Examples include:

- Elementwise
- CellApply
- Reduce
- WindowView
- StaticReindex
- Gather
- Contract
- LookupClassify
- other execution families

A structured graph operation such as convolution may remain a black box in Graph Basis while later lowering to WindowView + Contract or to a library/custom kernel.

## 7.2 Window / Prefix-Infix

J `u\` is preserved as `GraphForm::PrefixInfix`.

Graph Basis exposes a Window access family.

For example:

```text
(+/)\
```

is represented conceptually as:

```text
Window
  ↓
Reduce
```

rather than being collapsed into one opaque special case.

The independent Scan basis is now a design decision (2026-10-05); GF3 now adds initial Boolean identity candidates; broader recognition remains under GF3a and execution remains separate.

---

<a id="syntax-graph-hints"></a>

<a id="graph-prior-art-followup"></a>

#### 2026-10-05 prior-art follow-up: small basis and composition algebra

**Audit baseline:** documentation and `src/j_graph_ir.rs` at GitHub main `b00f2263decb4777e84f7670cc3bbd2536618f80`. The preceding prior-art documentation commit is `84b8546`. The initial audit recorded design contracts, not implementation completion claims; GF2/GF3 implementation and validation are recorded separately below. The uploaded `붙여넣은 텍스트(1).txt` could not be read because local path/execution tools failed; this restoration uses the retrieved conversation and the user's explicit follow-up list. Attachment reconciliation remains open.

| Item | Verified state | Decision / remaining implementation |
|---|---|---|
| Small Graph Basis + composition + witness | Basis layers, Pipeline/Hook/Fork regions and witnessed rewrite seam exist | Avoid a new op for every combination |
| First-class Scan | GF3 adds independent GraphBasisKind::Scan with Boolean atomic-prefix identity candidates/verifier; source Window→operand survives | Numeric/rank/representation extension and execution/reassociation/parallel-prefix authorization remain open |
| Vertical / horizontal / nested | GF2 adds a common composition sidecar/verifier over pipeline, branch/join and operand paths | Independence witnesses, noun-left graph specialization and execution integration remain open |
| Fusion algebra / registry | GF4 adds four research schemas and registry/envelope/verifier; shares provenance with witnessed E. rewrites | General equivalence proof, resource transfer, target queries and executable selection remain open |
| Symbolic Work / Depth | GF5 adds an independent symbolic domain, ordered map/reduce/region and Scan identity models | General rank/window, legally witnessed parallel models and actual target-cost integration remain open |
| Multiversion | Specialization/guard design and runtime baseline exist | Version selection, invalidation and bounded cache are long-term work |
| Streaming / inspector-executor | Window/access/resource seams exist | Streaming contracts and inspection plans are long-term design work, not executable support |

**Scan contract.** Scan preserves a different algorithmic structure from independently recomputing each prefix as Window→Reduce. Keep the original PrefixInfix FunctionEntity, valence, rank/cell boundaries and provenance. Do not classify arbitrary prefix or dyadic infix as Scan. Even insert-compatible monadic prefix requires a witness covering reducer, direction, prefix lengths, shape/assembly, empty/singleton behavior, identity use, integer overflow/promotion, floating-point results and domain/error/effect order. Unknown retains Window→operand. Spelling alone proves neither associativity nor purity. Scan identity and permission for reassociation/parallel-prefix execution are separate.

**Composition analysis.** Vertical describes producer→consumer edges; Horizontal describes independent consumers sharing a logical input; Nested describes computation inside rank/cell/segment boundaries. Relations may overlap, so a mutually exclusive enum or layers list cannot replace topology. The sidecar references node/region/edge identity, input occurrence, use-count/fan-out, live-across values and effect/error dependencies. Ordinary fork may expose a horizontal candidate while retaining observable h→f→g order and constructor subtype. Capped fork is a pipeline; noun-left fork retains a noun plus h. Nested classification does not authorize flattening.

**Fusion registry.** Reuse existing rewrite witnesses/verifiers while keeping this role separate from target lowering registration. Each rule declares stable ID/version, source basis+composition pattern, replacement graph, relevant call facts, proof obligations, witness/provenance mapping, fan-out/retained-value changes, symbolic resource/work-depth transfer and target capability query. Separate discovery, legality, feasibility, profitability and selection. Unknown establishes neither legality nor illegality. Map→Map, Map→Reduce, Map→Scan and common-input Map+Map are initial research candidates, not supported rewrites. Fusion can duplicate shared producers or extend lifetimes; do not assume cost always decreases. Implement an analysis-only seam before execution optimization.

**Symbolic Work/Depth.** Maintain total operations and dependency-path length separately from logical atom/state resources, including symbolic extents, operator costs, Unknown and provenance. Sequence adds both. Legally independent branches sum Work and use max Depth plus join; observable dependencies retain ordered paths. Map multiplies cell work by extent and preserves nested cell depth. Compute ordered Reduce/Scan baseline separately from a proved reassociation candidate. For a unit-cost associative operator, a work-efficient tree candidate may have O(n) Work/O(log n) Depth; this is not a universal J reducer contract. Empty/singleton cases are explicit. Latency, launches, traffic, transfers and synchronization belong to CostEstimate.

**Long-term contracts.** Multiversion connects relevant-fact keys, binding dependencies, guards, bounded caches and widening; guard miss cannot replay observable effects. Streaming requires proved chunk-boundary, carry/state, ordering, termination, bounded-memory and materialization contracts. Inspector-executor declares inspection cost/effects, mutation/alias invalidation and inspection-witness lifetime. These are neither full-J restrictions nor prerequisites for the first M4 CPU slice.

**Prior-art scope.** Futhark fusion documentation supports comparison of vertical/horizontal relations; its 2026 scan-scatter work illustrates an extensible fusion algebra. A compiler's current support limits are not permanent RustJ laws. Work/Span material motivates an analysis domain, not a proof of J numeric/error semantics.
- https://futhark.readthedocs.io/_/downloads/en/v0.25.4/pdf/
- https://futhark-lang.org/blog/2026-03-24-scan-scatter-fusion.html
- https://github.com/diku-dk/futhark-book/blob/master/parallel-cost-model.rst

**GF follow-up checklist — preserve M2/M3 and existing A1.5 sequencing**
- [x] GF0: Audit documentation against GraphBasis implementation and resolve the independent-Scan design decision.
- [ ] GF1: Reconcile the uploaded original text with this restoration.
- [x] GF2: Add an analysis-only CompositionRelation sidecar and verifier. Regress pipeline/ordinary/capped wiring and observable dependencies, the opaque noun-left boundary, nested rank/window/reduction and Copy Rank's header-only RHS. Actual noun-left h→g graph specialization remains a separate existing follow-up.
- [x] GF3: Add first-class Scan basis and Boolean Add/Multiply insert-prefix identity/contract witnesses with conservative recognition. Retain original Window→operand and compare general prefix/infix, unknown reducer and numeric/rank boundaries with C default/AVX2. This does not claim executable prefix support.
- [ ] GF3a: Extend recognition to integer/float, general reducers, nested rank and sparse with relevant value-property/rank/representation witnesses; Unknown is not proof.
- [x] GF4: Add versioned fusion schemas, registry, candidate envelopes and verifier over existing GraphRewriteProvenance and composition/Scan witnesses. Discover four research patterns; legality/resource/target/profitability/selection remain open.
- [x] GF5: Add independent symbolic WorkDepthExpr DAG, ordered successful-path node/region models, separate ordered Scan identity models, fusion source comparison and single-operation duplication hypotheses with verifiers/regressions.
- [ ] GF5a: Extend effective-rank/cell/segment and general window/reducer models; add parallel Depth/max/tree models only with independence and numeric/error witnesses.
- [ ] GF6: Connect selection/partition only for executable lowering with lifetime/resource/cost comparison.
- [x] GF6a: Connect source-only target feasibility with fusion/WorkDepth witnesses in readiness reports. Candidates lacking semantic proof, fused capability or transformed resource/cost remain unselected; full GF6 is still open.
- [ ] GF7: Advance multiversion, streaming and inspector-executor as long-term stages.

**Initial documentation validation limit:** `e0204d6`/`aba88ff` changed contracts/checklists only; no new Rust/Python/C checks ran then. GF2 validation below is a separate run.

<a id="gf2-composition-review"></a>

**GF2 code review and implementation — 2026-10-05.** Changes from `b00f226` to `aba88ff` affect only the two PROJECT documents. Compare the new contracts with j_graph_ir, graph memory/resource/rewrite and frontend FunctionEntity. Implement the analysis-only composition seam first; GF3–GF7 Scan/fusion/WorkDepth/execution selection remain open, following FOUNDATIONS' graph/execution separation.

- `src/j_graph_composition.rs` and `Plan::composition_analysis()` derive relations from verified J graphs. Vertical records producer/consumer ValueIds and Left/Right input occurrences. HorizontalCandidate records ordinary-fork region/input/branch identities without proving independence. ObservableOrder preserves constructor subtype, whole-child-invocation completion order and live-across values. Use counts follow `Plan::use_counts()`, including result/write consumers.
- Nested references the applied owner and original FunctionEntity operand paths for rank/cell, reduction and prefix/window boundaries. It invents no inner applied ValueIds and authorizes no flattening. Copy Rank's right function supplies a constructor header; its body is not a nested computation of the call.
- Review found that opaque noun-left Modifier forms inherited ordinary-fork ParallelBranch/BranchJoin hints. Remove them and record a RetainedNounBoundary referencing the original snapshot operand. Capped forks remain Pipeline/CappedFork; ordinary forks preserve h→f→g. Noun-left boundaries invent no f invocation; the original entity still owns the snapshot.
- The sidecar verifier checks the original graph, then re-derives relations to reject extra/missing/stale edges, slots, order, constructors, nested paths and fan-out. Unknown branch effect/error facts retain ObservableOrder. This is not a binding guard, parallelism legality proof or physical schedule.
- Seven new regressions cover pipeline wiring, unknown-function fork order/fan-out, capped/noun-left distinction, nested rank/window/reduction, repeated input slots, Copy Rank RHS and corrupted sidecars/graphs. The C basis is pinned `jsrc/cf.c::jtfolk`'s nvv/vvv/capped distinction and the existing two-DLL frontend differential corpus; no new private C trace equivalence is claimed.

**GF2 validation:** native Windows default/portable each **442 passed / 17 ignored**, fmt/clippy/build passed; Python **27 passed**. For each j64/AVX2 DLL, all three runtime routes report **5,380 cases / 5,380 passed / failed 0**, stages **10,810 checks / failed 0**, words **6,623 / failed 0**. Vocabulary checks cover 143 POS classifications from 145 candidates, 140 bare-function bindings/ARs and 3 noun payloads, with zero coverage gaps and two code-only rejections. Capture graph boundaries **257** and static boundaries **2** remain separate. A native verifier confirmed binary/source/DLL hashes in all twelve reports. Source pin `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528` and DLL release `ded7793fe5795d79eda8e7138dce94aa056edf78` are distinct. This gate does not establish optimizer/parallel scheduling/CUDA/full-upstream equivalence. No Linux tests or GitHub CI ran.

<a id="gf6a-fusion-readiness"></a>

**GF6a downstream fusion readiness — 2026-10-05.** `src/fusion_planning.rs` / `LoweringRegistry::fusion_readiness(plan,rules,target)` connect GF4 candidates and GF5 models to target-dependent inspection. Keep target metadata out of intrinsic J Graph identity/grammar. Query each source basis layer through the existing LoweringRegistry×TargetCapabilities target-only interface. This is neither resolved-call execution legality nor whole-fused-kernel support.

Reports retain candidate/rule identity, source feasibility, unresolved proof obligations, DeferredUntilLegality fused-target queries, AwaitingSemanticProofs and selected=false. Compare original target/registry/fusion/work-depth witnesses to reject stale/forged selection, erased obligations and asserted cost improvement. Unknown establishes neither legality nor illegality; source Unsupported is not a J language error. Guards/check-to-use/ownership, semantic/error equivalence, actual transformed lowering, lifetime/resource bounds, empirical CostEstimate and selection/partition remain full-GF6 work. A readiness report is not an execution route or fallback/replay plan.

`WorkDepthAnalysis::fusion_envelope_batch()` shares source validation and clones one expression arena, adding only three expressions per candidate (Work sum, Depth sum, Unknown replacement). Preserve source/retained identities and Unknown replacements while avoiding candidate×whole-expression-graph storage. Retain single-envelope inspection; the batch verifier re-derives schema/provenance/source proofs.

Four native Rust regressions cover source capability without fusion selection, changed target/registry invalidation, multi-candidate arena sharing without Scan execution promotion, forged selection/obligations/profitability and no-candidate handling for an unknown valid J graph. The generic GPU target is a metadata-only unit query, not GPU execution/compilation validation.

**GF6a validation:** native Windows default/portable each **463 passed / 17 ignored**, fmt/clippy/build passed; Python **30 passed**. Each j64/AVX2 DLL retains three runtime routes at **5,380 cases / 5,380 passed / failed 0**, stages **10,810**, words **6,623**, vocabulary POS **143**/binding **140**/nouns **3**. Scan **285 cases / 274 identity checks / 11 rejected analysis checks / failed 0** and **285 runtime prefix boundaries / executable prefix passes 0** remain separate. Capture graph boundaries **257** and static boundaries **2** also remain separate; binary/source/DLL hashes in all fourteen reports were checked. This validates readiness, not actual fusion selection, measured performance or GPU execution. No Linux tests, GitHub CI or CUDA ran.

<a id="gf5-work-depth"></a>

**GF5 symbolic Work/Depth — 2026-10-05.** Add `src/j_graph_work_depth.rs` / `Plan::work_depth_analysis()`. A separate schema-version-1 expression DAG supports Constant, provenance-bearing Unknown, logical Atoms/LeadingItems/ItemAtoms, metric-specific OperatorCost, Sum/Max/Product/Predecessors/IfEmpty. Keep memory-resource expressions, bytes, strides, buffers, launches and wall-clock cost separate. Callers supply explicit abstract operator weights through OperatorCostModel. Unknown costs/extents and checked arithmetic overflow evaluate to None. Unit-weight regressions validate mathematical models, not measured performance.

The initial model is an **ordered successful-path logical baseline**. Direct elementwise core calls retain DispatchChecks + atoms×Element. Ordered Reduce with primitive dyadic Map reducers retains DispatchChecks + (leading items−1)×item atoms×ReducerPair and conditional empty identity. This neither removes source error checks nor predicts the work of an actual failed trace. Costs must remain Unknown when dtype/numeric-retry or other relevant facts are insufficient. Shape-changing/unknown reducers, opaque/name/definition calls, general rank/window and final assignment remain Unknown. Actual name/guard latency, representation/materialization and hardware cost are outside this domain.

Collect each region's source operations backward from its output, stopping at external inputs, then sum each once in original order. Do not add nested-region costs again on top of their same child operations. Total counts every original operation once. Preserve ordinary-fork h→f→g dependencies and sum Depth; syntax alone never generates Max. The current ordered scalar Map baseline does not claim the optimal parallel critical path. Keep rank/cell interiors Unknown until relevant call facts/models exist.

GF3 Boolean Scan identities receive **separate ordered Scan hypotheses**: DispatchChecks + (n−1)×item atoms×ReducerPair + output atoms×ResultAssembly, preserving original empty/scalar/singleton and identity contracts. The source PrefixInfix cost/execution remains Unknown. ResultAssembly is an abstract weight, not a forced physical buffer/copy. Grant no parallel scan permission or execution selection.

GF4 envelope comparison references verified source operations/retained values. Even when source Work/Depth evaluates, transformed replacement remains FusionTransferUnproven/Unknown with improvement_proven=false. A single-operation extra-call hypothesis multiplies only additional calls with existing inputs; it is not whole-upstream cloning or legal duplication. duplication_authorized=false. Both hypotheses re-derive source/fusion witnesses and reject forged permission/improvement.

Eight Rust regressions cover symbolic unknown/operator weights, ordered forks, nested-region nonduplication, empty/singleton/scalar/matrix Reduce and separate Scan, unknown rank/extent/assignment, extra-call duplication, source-vs-unproved fusion, cycles/invalid provenance/totals/arithmetic overflow. General failed-path cost, reassociated reducers, parallel Max/tree, general CellApply/Window and executable optimization remain open.

**GF5 validation:** native Windows default/portable each **459 passed / 17 ignored**, fmt/clippy/build passed; Python **30 passed**. Each j64/AVX2 DLL retains three runtime routes at **5,380 cases / 5,380 passed / failed 0**, stages **10,810**, words **6,623**, vocabulary POS **143**/binding **140**/nouns **3**, failed 0. Scan checks retain **285 cases / 274 identity checks / 11 rejected analysis checks / failed 0**, with **285 runtime prefix boundaries / executable prefix passes 0** separate. Capture graph boundaries **257** and static boundaries **2** remain separate. A native verifier confirmed binary/source/DLL hashes in all fourteen reports. This validates symbolic models/source semantics, not measured performance or parallel/fused execution. Keep source pin and DLL release distinct; no Linux tests, GitHub CI or CUDA ran.

<a id="gf4-fusion-registry"></a>

**GF4 fusion registry and source envelopes — 2026-10-05.** Add `src/j_graph_fusion.rs` and `Plan::fusion_analysis(registry)`. Register stable IDs/version 1 for Map→Map, Map→Reduce, Map→Scan and common-input Map+Map. Reject duplicate/conflicting typed patterns, invalid versions/patterns and missing rank-cell/assembly, numeric, observable effect/error order, fan-out/retention, resource/work-depth and target-capability obligations. This is not an arbitrary-pattern overlap solver.

The replacement is an **OrderedSourceEnvelope** preserving the source applied-operation subgraph, input occurrences, order and external outputs. It is an analysis region, not a fused kernel or new semantic op. Reuse GraphRewriteProvenance with complete GF2 composition and GF3 Scan witnesses, without borrowing E.'s equivalence proof for unrelated fusion. MapScan requires a separately witnessed Scan call. Noun-left/capped forks invent no horizontal branches.

Record original use counts, internal input occurrences, external consumers and retained values. Two dyadic slots sharing one producer create one candidate but retain both occurrences. Preserve externally consumed producers/outputs rather than erasing or duplicating them. Re-derive source span/basis/value, order, fan-out, relevant input/operation GraphFacts and witnesses. Candidate-local counts use a small sparse map, avoiding a full-graph scratch array per candidate.

All candidates have legality=Unknown, resource_transfer_proven=false, selected=false and DeferredUntilLegality target queries. Legal transformed replacements, actual lowering queries, resource/work-depth transfer proofs, profitability and selection/partition remain open. Discovery guarantees neither lower cost nor parallelism; it does not mutate the original graph. Five Rust regressions cover registry rejection, three vertical patterns, horizontal h→f/join/external retention, repeated slots/shared-producer external uses, unknown Scan/noun-left/capped boundaries and forged selected/span/order/retention.

**GF4 validation:** native Windows default/portable each **451 passed / 17 ignored**, fmt/clippy/build passed; Python **30 passed**. For each j64/AVX2 DLL, existing three runtime routes report **5,380 cases / 5,380 passed / failed 0**, stages **10,810**, words **6,623**, failed 0. Scan checks retain **285 cases / 274 identity checks / 11 rejected analysis checks / failed 0**, with **285 runtime prefix boundaries / executable prefix passes 0** separate. Existing capture graph boundaries **257**, static boundaries **2** and vocabulary POS/binding/noun checks remain separate. A native verifier confirmed binary/source/DLL hashes in all fourteen reports. This validates source-graph/discovery/verifier paths, not fused execution, reassociation or parallel scheduling. Keep source pin and DLL release distinct; no Linux tests, GitHub CI or CUDA ran.

<a id="gf3-scan-identity"></a>

**GF3 Scan identity and conservative recognition — 2026-10-05.** `src/j_graph_scan.rs` / `Plan::scan_analysis()` derive independent GraphBasisKind::Scan candidates with ExactBooleanAtomicPrefixV1 witnesses from verified J graphs. Registry remains 8; extend the enum vocabulary with Graph IR schema **0.9**. Preserve the original PrefixInfix FunctionEntity, span, valence, input ValueId and Window→Reduce basis. Candidate output facts remain separate and are not injected into execution lowering. `execution_basis_for_graph_basis(Scan)` returns None, granting no execution capability.

- **Initial proof scope:** direct monadic PrefixInfix of Insert with an operand-free core Add/Multiply reducer, exact Boolean input dtype, consistent shape/rank and checked atom extent. Each Boolean sum lane is bounded by the leading-item count, which must fit i64; Boolean products stay in {0,1}. NameRefs, arbitrary verbs/reducers and Int/Float/boxed/unknown inputs gain no witness. Recognition reads no array payload and copies no large noun to prove legality.
- **Prefix/assembly:** inclusive leading-axis prefix lengths are 1..n with no artificial identity. Scalars produce shape [1]. Zero/one items or zero total atoms preserve input atoms/type, matching C atomic scan. Singleton/empty Boolean sums therefore remain Bool; nonempty n≥2 sums produce Int. Boolean products remain Bool. Dyadic infix and nested calls lacking effective-rank/cell/frame/assembly facts retain explicit analysis boundaries.
- **Effects/errors/numerics:** inspect actual bare core identity and Boolean closure/overflow bounds, without guessing associativity from spelling or arbitrary reducers. Insert no new reducer/identity call on scalar/empty/singleton paths. Preserve source order with `parallel_prefix_authorized=false`. Float accumulation order, integer retry/promotion, name/locale effects, sparse/representation conditions and general rank assembly need separate proofs. The witness applies to this verified plan's input facts/provenance and does not replace execution-time binding/metadata guards.
- **Verification:** validate the source graph, then re-derive contracts, source/input/span, basis, facts, witness and boundaries to reject stale/missing/forged values. Four Rust regressions cover short/scalar/matrix cases, general prefix/infix/numeric/NameRef/rank boundaries, forged parallel permission/dtype/span and unknown/inconsistent/overflow extents.
- **C differential scope:** add `tools/scan_contract_conformance.py` and `examples/scan_contract_probe.rs` to the Windows runner. Separate **274 identity/type/shape checks** (all Boolean patterns of lengths 0..6, scalar, empty/matrix/3D shapes) from **11 rejected analysis checks** (general prefix/infix/rank, integer overflow, float cancellation/signed zero/infinity, char and boxed). Compare C values with an independent exact Boolean model. Report all **285 runtime prefix boundaries** and **executable prefix passes 0** separately; Unsupported is never an executable equivalence pass. Rejected numeric/error results are C oracle observations, not Rust prefix execution equivalence.

**Sources:** reviewed [ap.c::jtbslash/jtpscan and Boolean prefix kernels](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ap.c), [atomic type dispatch](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/va2.c), [insert semantics](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ar.c). Keep source pin distinct from actual DLL release. Follow FOUNDATIONS' graph/execution separation and J rank/error preservation.

**GF3 validation:** native Windows default/portable each **446 passed / 17 ignored**, fmt/clippy/build passed; Python **30 passed**. Each j64/AVX2 Scan report has **285 cases / 274 identity checks / 11 rejected analysis checks / failed 0**, with **285 runtime prefix boundaries / executable prefix passes 0** separate. Existing three runtime routes each retain **5,380 cases / 5,380 passed / failed 0**; stages **10,810**, words **6,623**, vocabulary POS **143**/binding **140**/nouns **3**, all failed 0. Existing capture graph boundaries **257** and static boundaries **2** remain separate. A native verifier confirmed binary/source/DLL hashes in all fourteen reports. Source pin `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528` and DLL release `ded7793fe5795d79eda8e7138dce94aa056edf78` are distinct. No Linux tests, GitHub CI, CUDA or new executable optimizer ran.

## 7.3 Syntax-derived graph optimization hints

Integrated review date: 2026-10-02. Korean canonical counterpart: [§4.1.2](PROJECT.ko.md#syntax-graph-hints). This review classifies compiler information first, then identifies what J syntax contributes to graph IR. It separates leaf-kernel knowledge from composition structure. It is a design/documentation update, not new parser, parallel execution, or CUDA implementation.

### 7.3.1 The fork itself exposes the parallel candidate

The parallel possibility in `(loss_adjoint [ (loss emit))` comes from the **outer fork with `[` as its middle verb**. Recognizing that candidate does not require knowing that one leaf computes an adjoint or how the loss kernel works.

For a monadic `(f [ h)` application, the same input feeds `f` and `h`; their results reach the middle verb. Neither branch's returned value is the other's direct input. The middle verb's selector meaning then identifies the left result as the returned value. Keep four stages distinct:

1. Syntax supplies fork topology and the parallel candidate.
2. The middle verb's semantics supplies result-selection information.
3. Effect, hidden-state, alias, binding, and observable-error analysis establishes legality.
4. Schedule/resource/cost analysis chooses serial, parallel, or fused execution.

Candidate discovery must not be confused with permission to execute concurrently. [J Trains](https://www.jsoftware.com/help/dictionary/dictf.htm)

If `loss` and `emit` are both verbs, the inner `(loss emit)` is a hook: `y loss (emit y)`. Its producer-consumer dependency and original-input reuse coexist with the outer fork. Resolve parts of speech before classifying the expression. `emit` effects come from the extension contract; the spelling is not a standard J keyword or a reserved RustJ component name.

### 7.3.2 Classify the graph information first

| Compiler-information category | What syntax can expose without inspecting leaf kernels |
|---|---|
| Dependence and parallelism | Fork branches, hook dependencies, composition and joins |
| Value flow and use | Input routing, selected result, retained original input |
| Fusion scope | Producer-consumer composition and cell boundaries |
| Work decomposition | Rank application, frame/cell, partition/window structure |
| Iteration and selection | Loop-carried edges, repeat counts, selected regions |
| Constants and specialization | Bound nouns, noun-valued train operands, explicit Fix |
| Reuse candidates | Repeated applications and shared graph inputs/consumers |

Purity, exact dtype/shape, associativity, allocation aliasing/alignment, SIMD width, and CUDA placement are not general consequences of these forms. General compiler categories were informed by [GCC attributes](https://gcc.gnu.org/onlinedocs/gcc-15.2.0/gcc/Common-Function-Attributes.html) and [LLVM LangRef](https://llvm.org/docs/LangRef.html); the mapping to J graph information is a RustJ design synthesis.

### 7.3.3 Syntax hints with black-box leaf verbs

“Syntax” here includes train grammar and modifier application. J also calls built-in modifiers primitives; the distinction is between **what a leaf computes** and **how computations are connected/applied**. The following information survives when `f/g/h/u/v` are black boxes. Rank/cell boundaries accompany the explanatory normalizations.

| ID | J form | Information supplied by structure | Optimization candidate |
|---|---|---|---|
| S1 | `(f g h) y` | Shared-input fan-out and middle-verb join | Branch parallelism, shared-input planning |
| S2 | `(f [ h) y`, `(f ] h) y` | Fork plus selected-result use | Parallel branches, reduced result storage |
| S3 | `(f g) y` | `g(y)` and original `y` feed `f` | Retain input lifetime, producer-consumer fusion |
| S4 | `([: u v) y`, `(u@:v) y` | Ordered composition | Intermediate materialization elision |
| S5 | `(u@v) y` versus `(u@:v) y` | Different rank/cell application boundaries | Cell-local versus whole-array fusion |
| S6 | `x (u&v) y` | Transform each argument with `v`, then join with `u` | Parallel transformations, code specialization |
| S7 | `(u"n) y` | Apply a verb to cells of specified rank | Cell decomposition, batching |
| S8 | `(m&u) y`, `(u&n) y`, `(N g h) y` | Bound noun operand or constant train branch | Bound-value specialization |
| S9 | `(u^:k) y` | Iteration region with loop-carried input | Small constant-loop expansion, lifetime planning |
| S10 | Gerund `@.` | Selector and selected verb/train region | Constant selection, isolated branch regions |
| S11 | Nested/grouped trains | Precise grouping and topology | Preserve optimization regions |
| S12 | `(f g f) y` | Repeated named expression applied to the same input | Common-expression candidate subject to binding/effect proof |

Sources: [Trains](https://www.jsoftware.com/help/dictionary/dictf.htm), [Atop](https://www.jsoftware.com/help/dictionary/d620.htm), [At](https://www.jsoftware.com/help/dictionary/d622.htm), [Compose](https://www.jsoftware.com/help/dictionary/d630v.htm), [Rank](https://www.jsoftware.com/help/dictionary/d600v.htm), [Bond](https://www.jsoftware.com/help/dictionary/d630n.htm), [Power](https://www.jsoftware.com/help/dictionary/d202n.htm), [Agenda](https://www.jsoftware.com/help/dictionary/d621.htm).

S1–S6 expose connectivity without leaf-array semantics. S7–S10 use the modifier's application contract and operand facts, not a leaf's arithmetic properties. The corresponding restrictions are:

- Fork topology does not prove the absence of global/slot effects or observable error-order constraints. Keep J-compatible branch order until proof permits reordering.
- An unselected returned value is not necessarily an unobservable branch. Preserve its effects and errors; eliminating its payload requires further analysis.
- Hook retains the original input for the downstream call and does not expose two independent stages.
- Composition exposes a fusion candidate, not proof that opaque kernels can actually fuse. Preserve rank, effects, and errors.
- Dyadic compose applies the same verb to different inputs; it does not imply one shared computed result. Distinguish verb compose from noun bond by operand POS.
- Rank does not fix dtype, cell shape, contiguity, purity, uniform results, or empty-frame prototypes. Preserve J agreement, negative-rank interpretation, and assembly/error semantics.
- Nonnegative integer Power exposes a dependent loop, not parallel iterations. Negative, infinite, boxed, and array powers require their own semantics; inverse/obverse is not adjoint.
- Scalar Agenda may define one selected branch; general selectors may construct trains. Do not eagerly execute unselected regions.
- Parentheses only contribute when grouping changes parsing. Redundant parentheses are not a fusion fence, sequencing directive, or materialization obligation.
- Repeated names do not authorize CSE without binding/input/effect/error equivalence.

### 7.3.4 Modifier structure versus leaf-specific optimization

| Form | Structure available without leaf knowledge | Additional leaf contract needed |
|---|---|---|
| `u/ y` | Insert a dyad between items | Identity, associativity, numeric reassociation |
| `u\ y` | Apply `u` to each prefix | Efficient scan recognition for suitable `u` |
| `k u\ y` | Window/chunk extent, overlap and final fragment | Valid rolling-aggregation transformation |
| `u;.n` | Cut/tile/segment application | Segment kernel and result specialization |
| `keys u/. values` | Apply `u` to key groups | Key comparison and grouped kernel contracts |
| Dyadic `u . v` | Combined contraction application | Concrete dot/GEMM pattern and numerical contract |

For `+/\`, prefix application is structural information; a summation scan kernel requires knowledge of `+` and Insert. Similarly, `+/ % #` combines fork topology with sum/tally/divide leaf information. Never infer reassociation, FMA, integer-overflow behavior, or parallel reduction permission merely from structure. Sources: [Insert/Table](https://www.jsoftware.com/help/dictionary/d420.htm), [Prefix/Infix](https://www.jsoftware.com/help/dictionary/d430.htm), [Cut](https://www.jsoftware.com/help/dictionary/d331.htm), [Key/Oblique](https://www.jsoftware.com/help/dictionary/d421.htm), [Dot](https://www.jsoftware.com/help/dictionary/d300.htm).

Explicit `u f.` adds name-reference freezing semantics, distinct from grouping alone; parts containing `$:` are exceptions. Do not automatically Fix ordinary late-bound verbs or infer purity from Fix. [J Fix](https://www.jsoftware.com/help/dictionary/dfdot.htm)

A generic graph containing only leaf calls may lose cell, partition, loop, and binding structure. A graph already carrying identical information gains no new facts merely by being written in J. Syntax remains the source of candidate discovery even when another IR can represent the same topology. Preserve this source/provenance rather than recovering it primarily from a flattened DAG.

### 7.3.5 Integration and verification status

At the integration baseline `89b87b8`, `src/j_graph_ir.rs` already preserves `GraphForm::Pipeline/Hook/Fork/Reduce/PrefixInfix/Rank`, explicit stages/branches, and hints including `ParallelBranchCandidate`. `classify_function()` emits the fork candidate before leaf-kernel analysis. This is not implemented concurrent execution, general support for every modifier in S1–S12, or a performance result.

Keep `FunctionEntity` immutable and derive applied J Graph IR rather than adding target/pass facts to parser entities. The processing order is:

```text
POS/binding and parsing
  → syntax-derived applied topology and GraphHint candidates
  → execution-semantic legality, witnesses/guards
  → target/resource/cost analysis and schedule/physical decisions
```

`ParallelBranchCandidate` records discovery; it is not a `ParallelSafe` proof. Retain source span, syntax origin, valence/rank, value edges, J-observable order, and analysis state at the appropriate existing layer. Do not make a guaranteed native fallback claim: use a supported conservative route or report unsupported implementation.

- [x] Classify compiler information and separate syntax topology from leaf-kernel facts.
- [x] Record S1–S12, the outer-fork origin of the user example, and framework sources below.
- [ ] Extend graph regression coverage with named black-box verbs as support is added; verify outer fork and inner hook separately.
- [ ] Verify no spurious branch-return dependency, original-input lifetime, binding changes, and rank/valence behavior.
- [ ] Verify that effectful forks retain candidate provenance but cannot execute concurrently without legality proof.
- [ ] Cover non-associative reduction, floating order, heterogeneous/empty rank results, Agenda selection, and Power variants.
- [ ] Compare values/dtypes/shapes/errors/effects and materialization costs on native Windows when execution changes land.

The existing migration order remains authoritative. This integration does not advance implementation checkboxes, require all missing syntax before the next vertical slice, or resume CUDA work.

<a id="framework-graph-hints"></a>

## 7.4 Source-based array-compiler framework comparison

Integrated review date: 2026-10-02. Korean counterpart: [§4.1.3](PROJECT.ko.md#framework-graph-hints). The comparison covers JAX, XLA, MLIR Linalg, TVM Relax, and Halide. Every finding below separates observed behavior from a proposed J application and includes direct sources. External GitHub `main` links are moving references, not pinned snapshots; pin and recheck revisions when adopting an implementation. RustJ's integration baseline is `89b87b8`. Sources were inspected, not built or executed.

### F1 — Reconvergence identifies whole-region candidates

TVM `FuseOps` uses post-dominator analysis and path checks for diamond-shaped fusion. In J, `(f g h) y` exposes branches and a middle join; `((f g h)@:p) y` exposes a shared producer followed by a diamond. This gives branch-parallel and whole-region fusion candidates without inspecting leaf kernels. A syntactic join is only a reconvergence candidate: external consumers, effects, selectors, and the actual graph determine dominance and legality.

Sources: [TVM fuse_ops.cc, algorithm commentary and GraphCreator](https://github.com/apache/tvm/blob/main/src/relax/transform/fuse_ops.cc), [J Trains](https://www.jsoftware.com/help/dictionary/dictf.htm), [J At](https://www.jsoftware.com/help/dictionary/d622.htm).

### F2 — Consumer count informs sharing, recomputation, and lifetimes

MLIR computes producer results that must survive fusion because other consumers use them; a specific reshape-fusion default also checks single use. This is not a universal single-consumer rule. J composition, a shared producer before a fork, and hook reuse of the original input expose relevant use edges. Analyze whole-graph consumers and region live-outs before choosing shared materialization, recomputation, or multi-output fusion. Do not free a hook input before its last use. Duplication requires binding/effect/error equivalence and profitability; logical sharing does not prove allocation aliasing.

Sources: [MLIR ElementwiseOpFusion.cpp, getPreservedProducerResults and defaultControlFn](https://github.com/llvm/llvm-project/blob/main/mlir/lib/Dialect/Linalg/Transforms/ElementwiseOpFusion.cpp), [J Trains](https://www.jsoftware.com/help/dictionary/dictf.htm).

### F3 — Parallel and fused execution are competing candidates

XLA priority fusion examines users and fusibility, compares estimated fused/unfused times, and considers multi-output fusion in some paths. A J fork can expose separate branch execution and common-input fusion candidates; composition exposes intermediate-elision candidates. Compare serial, parallel, shared, recomputed, and fused plans using work, rereads, and temporary extents. Black-box topology alone cannot estimate register pressure or establish kernel fusion. Use a supported conservative route when analysis is insufficient; CUDA remains deferred.

Sources: [XLA priority_fusion.cc, CalculateProducerPriority/EstimateRunTimes](https://github.com/openxla/xla/blob/main/xla/backends/gpu/transforms/priority_fusion.cc), [XLA Fusion and Buffer Assignment](https://openxla.org/xla/gpu_architecture).

### F4 — Preserve rank as iteration domains and argument mappings

MLIR Linalg carries iterator kinds and indexing maps separately from computational payload. Its fusion checks tensor semantics, iterators, maps, and loop-bound preservation. J rank plus J agreement can supply frame/cell iteration and per-argument logical mappings, enabling batching and cell-local composition candidates. Do not immediately label every frame a proven parallel iterator; check effects, uniform results, empty prototypes, and assembly. Do not substitute NumPy broadcasting or force all mappings to be affine.

Sources: [Linalg documentation](https://mlir.llvm.org/docs/Dialects/Linalg/), [LinalgStructuredOps.td](https://github.com/llvm/llvm-project/blob/main/mlir/include/mlir/Dialect/Linalg/IR/LinalgStructuredOps.td), [areElementwiseOpsFusable](https://github.com/llvm/llvm-project/blob/main/mlir/lib/Dialect/Linalg/Transforms/ElementwiseOpFusion.cpp), [J Rank](https://www.jsoftware.com/help/dictionary/d600v.htm).

### F5 — Composition plus proven mappings can avoid intermediate copies

MLIR fusion composes consumer iteration and producer indexing maps. XLA separates logical shape from physical layout and handles layout conflicts with copies. J composition supplies connectivity; concrete reindex operation contracts supply maps. Combining the two exposes intermediate-elision candidates. Preserve `@`/`@:` boundaries, non-affine cases, fill and logical atom order. Actual address span, alias, contiguity, and alignment remain physical facts. Composition alone does not prove a transpose is metadata-only.

Sources: [MLIR consumerToProducerLoopsMap](https://github.com/llvm/llvm-project/blob/main/mlir/lib/Dialect/Linalg/Transforms/ElementwiseOpFusion.cpp), [XLA Layout Assignment](https://openxla.org/xla/gpu_architecture), [J At](https://www.jsoftware.com/help/dictionary/d622.htm).

### F6 — Window overlap exposes locality and required-region information

Halide lesson 8 compares inlining, root computation, and computation at consumer scopes. Its example shows required producer regions, reuse, circular-buffer storage, and cases where parallel loops prevent those optimizations. J window/cut structure plus producer-consumer composition can expose overlapping input regions and fragment boundaries. Compare full materialization, tile-local storage, and bounded sequential reuse; separate computation scope from storage scope. J grouping is not a Halide schedule directive. Window overlap does not prove independent output writes or equivalent rolling floating sums, and tile size remains a schedule decision.

Sources: [Halide lesson 8](https://halide-lang.org/docs/tutorial/lesson_08_scheduling_2.html), [its C++ source](https://github.com/halide/Halide/blob/main/tutorial/lesson_08_scheduling_2.cpp), [J Prefix/Infix](https://www.jsoftware.com/help/dictionary/d430.htm), [J Cut](https://www.jsoftware.com/help/dictionary/d331.htm).

### F7 — Separate Power carry from invariant inputs

JAX loop code distinguishes constants from carry, checks scan carry types, and can move forwarded while-loop carry to constants under conditions. Nonnegative integer J Power supplies the loop region; bound nouns and unchanged inputs supply invariant candidates. Hoist only proven invariant preparation/binding/metadata work. J may change result shape/dtype between iterations: do not import JAX's fixed carry constraint as a language restriction. Stable specialized paths need proof/guards and a supported general route. Hidden state reads cannot be hoisted solely because the source name is unchanged.

Sources: [JAX loops.py, carry checking and forwarding](https://github.com/jax-ml/jax/blob/main/jax/_src/lax/control_flow/loops.py), [Jaxpr loops](https://docs.jax.dev/en/latest/601/jaxpr.html), [J Power](https://www.jsoftware.com/help/dictionary/d202n.htm), [J Bond](https://www.jsoftware.com/help/dictionary/d630n.htm).

### F8 — Preserve selected and nested regions rather than only a flat DAG

Jaxpr stores branch and loop bodies as sub-jaxprs; its conditional code combines branch effects. Relax If stores separate branch SeqExprs. Agenda and nested trains can supply selected regions, local facts, and topology for independent analysis. Preserve selector-specific semantics; general Agenda can construct trains. Do not eagerly evaluate unselected branches, turn all cases into binary if, or interpret parentheses as scheduling fences.

Sources: [Jaxpr](https://docs.jax.dev/en/latest/601/jaxpr.html), [JAX conditionals.py, _join_cond_effects](https://github.com/jax-ml/jax/blob/main/jax/_src/lax/control_flow/conditionals.py), [TVM expr.h, IfNode/SeqExprNode](https://github.com/apache/tvm/blob/main/include/tvm/relax/expr.h), [J Agenda](https://www.jsoftware.com/help/dictionary/d621.htm).

### F9 — Distinguish value live-out from effect live-out

Relax distinguishes ordinary binding blocks from pure dataflow blocks; MLIR preserves externally used producer results; JAX loops check allowed effects. A selection fork exposes parallel topology and a selected returned value, but its other branch may still have observable effects/errors. Separate external values from effect dependencies. Do not delete `loss emit` because its value is unselected, or declare every J fork a pure dataflow region. Syntax supplies the candidate and value flow; contracts establish purity and legality.

Sources: [Relax DataflowBlock API](https://tvm.apache.org/docs/reference/api/python/relax/relax.html), [MLIR getPreservedProducerResults](https://github.com/llvm/llvm-project/blob/main/mlir/lib/Dialect/Linalg/Transforms/ElementwiseOpFusion.cpp), [JAX loop effect checks](https://github.com/jax-ml/jax/blob/main/jax/_src/lax/control_flow/loops.py), [J Trains](https://www.jsoftware.com/help/dictionary/dictf.htm).

### 7.4.1 Applying the findings to the user example

For `(loss_adjoint [ (loss emit))`, the outer fork supplies F1/F3 parallel topology. If the inner form is a hook, it supplies F2 input retention and producer-consumer edges. The selector supplies F9 returned-value use. Keep topology, value use, and effect/error/alias legality distinct. Adjoint registration is not a parallel annotation; later AD expansion may expose new branches whose actual topology then supplies candidates. Interpretation sources: [J Trains](https://www.jsoftware.com/help/dictionary/dictf.htm), [TVM fusion](https://github.com/apache/tvm/blob/main/src/relax/transform/fuse_ops.cc), [MLIR fusion](https://github.com/llvm/llvm-project/blob/main/mlir/lib/Dialect/Linalg/Transforms/ElementwiseOpFusion.cpp).

### 7.4.2 Priorities and verification checklist

These are refinements of the existing migration plan, not new mandatory prerequisite stages:

| Priority | Information to retain/analyze | Finding and source |
|---|---|---|
| 1 | Syntax origin, topology, data/effect edges | F1/F9: [TVM fusion](https://github.com/apache/tvm/blob/main/src/relax/transform/fuse_ops.cc), [Relax API](https://tvm.apache.org/docs/reference/api/python/relax/relax.html) |
| 1 | Consumers, retained input, region live-outs | F2: [MLIR fusion](https://github.com/llvm/llvm-project/blob/main/mlir/lib/Dialect/Linalg/Transforms/ElementwiseOpFusion.cpp) |
| 1 | Cell/frame domains and logical input maps | F4/F5: [Linalg](https://mlir.llvm.org/docs/Dialects/Linalg/) |
| 2 | Reconvergence, actual post-dominance, external uses | F1: [TVM fusion](https://github.com/apache/tvm/blob/main/src/relax/transform/fuse_ops.cc) |
| 2 | Parallel/shared/recompute/fusion costs | F3/F6: [XLA source](https://github.com/openxla/xla/blob/main/xla/backends/gpu/transforms/priority_fusion.cc), [Halide source](https://github.com/halide/Halide/blob/main/tutorial/lesson_08_scheduling_2.cpp) |
| 3 | Carry/invariants and branch-local facts | F7/F8: [JAX loops](https://github.com/jax-ml/jax/blob/main/jax/_src/lax/control_flow/loops.py), [JAX conditionals](https://github.com/jax-ml/jax/blob/main/jax/_src/lax/control_flow/conditionals.py) |

Existing `GraphForm/GraphHint`, applied stages, consumer/liveness and symbolic-resource seams provide a starting point. Actual dominance analysis, general modifier support, schedule selection, and executable optimizations remain subject to their implementation gates. Do not copy foreign purity/static-shape restrictions into full J semantics or confuse graph hints with selected physical policy.

- [x] Inspect official documentation and relevant source for all five frameworks.
- [x] Record F1–F9 with observations, J-specific inferences, conditions, and sources.
- [x] Integrate the syntax review and framework comparison into the canonical/mirror project documents.
- [ ] Pin/recheck framework revisions before implementation adoption.
- [ ] Verify named black-box topology, consumers/live-outs, effectful-fork rejection, producer duplication, hook lifetime, and unselected Agenda regions.
- [ ] Verify rank mappings, empty prototypes, and shape-changing Power.
- [ ] On native Windows, compare values/errors/effects and copy/allocation/performance costs after execution integration.

No external framework builds, Rust/C runtime tests, performance measurements, or CUDA validation were performed for this documentation integration. Earlier validation records retain their original scope.

## 7.5 Candidate lifecycle and proof-discharge contract

A discovered candidate must not be represented conceptually by one `selected` boolean. Legality, target feasibility, hard-resource feasibility, cost, selection, and lowering answer different questions and carry different evidence.

Conceptually keep orthogonal evidence dimensions:

```text
CandidateEvidence
  provenance         Verified | Stale/Invalid
  equivalence        Unknown | Proven | Disproven | Guarded(GuardId)
  semantic_legality  per obligation: Unknown | Proven | Disproven | Guarded(GuardId)
  target_feasibility Unknown | Supported | RequiresFacts | Unsupported
  resource_state     Unknown | Symbolic | Resolved | ExceedsHardLimit
  cost_state         Uncosted | Estimated(CostEstimate)
  selection          Unselected | Selected | Rejected(reason)
  lowering_state     NotLowered | Lowered(transform/route identity)
```

A planner may derive a lifecycle summary such as:

```text
Discovered
  -> AwaitingProofs
       -> Illegal
       -> Legal / GuardedLegal
            -> Feasible
            -> Costed
            -> Selected / Rejected
            -> Lowered
```

This is a commit-gate order, not necessarily pass-execution order. Resource/work-depth/cost side analyses may run speculatively before semantic legality is complete; the forbidden action is committing the candidate to an execution plan before all required legality evidence is discharged.

Evidence ownership:

| Evidence | Primary owner | Selection rule |
|---|---|---|
| source topology/provenance | J Graph verifier + candidate registry | stale provenance invalidates the candidate |
| algebraic equivalence | rewrite/scan/fusion witness validator | required equivalence may not remain Unknown at commit |
| rank/cell/assembly | execution-semantic facts + candidate legality checker | prove or guard before effects |
| numeric/tolerance/reassociation | primitive/derived numeric contracts | no unlicensed reassociation/relaxation |
| effect/error ordering | effect/error/speculation analysis | disproven means illegal; post-effect guards cannot justify replay |
| fanout/retention/alias | graph use/liveness + alias/storage facts | external uses and alias obligations must survive |
| resource/work-depth | `j_graph_resource` / `j_graph_work_depth` | separate from cost; only hard-limit violations reject feasibility |
| target/lowering capability | LoweringRegistry × resolved target | source-basis support does not prove composite/fused legality |
| empirical profitability | CostProfile / planner | may reject a legal candidate without making it semantically invalid |
| compatibility/selection | Schedule/Transform planner | the selected candidate set must be mutually compatible |

`Guarded(GuardId)` requires a real guard, execution before observable effects, an explicit miss route, replay safety, and cache/provenance linkage. Do not replay a source region automatically after observable effects have committed.

For overlapping candidates, v0 is conservative: two rewrites that replace the same source operations cannot both be selected; overlapping rewrite/fusion candidates require a registered compatibility/composition rule; otherwise apply one candidate, create a new graph/version, and rediscover candidates. Selection belongs to a Transform/Schedule plan, not semantic IR.

Current code implements only part of this model:

```text
GraphRewriteCandidate
  provenance + equivalence witness

FusionCandidate
  proof obligations
  legality = Unknown
  resource_transfer_proven = false
  selected = false

RewritePlanningReport
  TargetUnsupported / NeedsCallFacts / NeedsResourceFacts / ReadyForCosting

FusionReadinessReport
  AwaitingSemanticProofs
  fused target query deferred
  selected = false
```

There is no common `CandidateEvidence`/`ProofBundle`, per-obligation discharge result, compatibility-aware SelectionPlan, or committed transform identity yet. Those names describe target architecture, not completed APIs.

When implementation reaches this area, preserve the project's one-meaning/one-test discipline: add proof-state provenance and stale-evidence verification first; then per-obligation fusion discharge without selection; then a shared legality view for rewrite/scan candidates; then target-hard-feasibility; overlap compatibility; a separate SelectionPlan; and finally committed lowering with provenance verification.


# Part V — Rewrite and equivalence

## 8. Optimization dependency order

The architecture fixes the following dependency:

```text
Basis discovery
    ↓
Rewrite candidate generation
    ↓
Equivalence validation
    ↓
Candidate resource evaluation
    ↓
Sound resource pruning
    ↓
Later costing/selection
```

Rewrite rules must not silently mutate the source J Graph node. They produce sidecar candidates with provenance.

## 8.1 Rewrite registry

Current minimum registry contains a witnessed rule for J Find:

```text
x E. y
   Search
     ⇅  J Dictionary equivalence witness
Window
  ↓
CellApply(Match)
```

The original Search node remains intact.

Each candidate records:

- source J Graph ValueId;
- source span;
- source Graph Basis;
- rule identity;
- equivalence witness;
- replacement graph;
- rewrite-specific fact rule.

The verifier recomputes rule-derived facts and rejects stale or invented facts.

## 8.2 Current Find rewrite path

For the currently supported RustJ `E.` subset:

- source rank and pattern rank are at most 1;
- result dtype is Bool;
- result shape preserves the right argument shape;
- the logical window family has one logical position per right-argument position;
- trailing non-fitting windows remain positions whose match result is false.

The candidate can therefore infer a logical Window shape such as:

```text
right shape [6]
pattern shape [3]
→ logical window family shape [6, 3]
```

while still leaving representation and physical realization downstream.

---

# Part VI — Graph facts and resource model

## 9. GraphFacts

GraphFacts is intentionally separate from execution `Facts`.

Graph IR owns target-independent logical facts:

- dtype;
- shape;
- rank;
- graph analyzability.

It does not import execution layout facts.

Primitive semantic transfer rules may be shared, but GraphFacts and execution Facts remain distinct containers/domains.

## 9.1 Static memory interpretation

JAXA's "static memory" claim is interpreted as logical determinability, not physical buffer assignment.

J Graph analysis may determine:

- logical extent;
- use count;
- value lifetime;
- materialization opportunity;
- retained values;
- logical storage obligation;
- symbolic state requirement.

It must not prematurely own:

- BufferId;
- byte offset;
- register count;
- shared-memory address;
- physical stride/layout;
- device allocation.

Those belong to later planning.

## 9.2 ResourceExprGraph

Current resource analysis uses symbolic expressions such as:

- Zero
- Unknown
- ValueAtoms(value)
- Requirement(value, kind)
- Sum(...)
- Max(...)
- Scale(...)

This preserves provenance for resource summaries rather than reducing everything to an unexplained number.

Tracked graph-level quantities include:

- internal logical edge volume;
- elidable materialization volume;
- retained live volume;
- graph-order peak live volume;
- baseline internal read+write traffic;
- potentially elidable traffic;
- temporary state;
- reduction accumulator state;
- window working-set state.

## 9.3 Resource-state lifetime

Temporary/accumulator/working-state requirements carry canonical graph-order lifetime records.

A state may be marked as potentially extending across fusion.

Region analysis keeps both:

- canonical graph-order peak state;
- a conservative all-child-state upper bound.

This avoids treating graph-order locality as proof that a fused realization has the same lifetime.

---

# Part VII — Resource-aware rewrites

## 10. Candidate resource evaluation

Graph rewrite candidates are evaluated in the same target-independent domain as their source:

- logical atoms;
- internal materialization;
- read/write traffic;
- symbolic state requirements.

Known and unknown are never conflated.

If implementation-local costs are unknown, resource metric ordering is `Incomparable`.

Current Find rewrite example:

- result extent is known;
- logical window extent can be known for the supported rank≤1 subset;
- WindowWorkingSet lower bound can be derived from pattern extent;
- structural/composite implementation state may remain unknown.

Therefore the rewrite can be resource-analyzable without claiming complete cost knowledge.

## 10.1 Sound pruning contract

Early resource pruning requires both:

```text
ResourceBoundLocality::Local
+
PruningMonotonicity::ProvenMonotone
```

Without both proofs, candidates remain available and are evaluated later.

The current Find rewrite does **not** satisfy this early-pruning proof contract.

---

# Part VIII — Execution lowering and target feasibility

## 11. Logical Execution IR

Execution Semantic Lowering derives executable dataflow from the semantic/J Graph representation.

Logical IR owns:

- explicit operation/value flow;
- semantic checks;
- effect summaries;
- possible-error facts;
- speculation/evaluation-order constraints;
- execution-basis identity;
- execution-basis payload;
- resolved call/rank facts.

It must not contain target-specific schedule decisions.

## 11.1 WindowView payload

`ExecutionBasis::WindowView` now has explicit semantic payload support:

```text
WindowView {
    source,
    shape: PatternShape { pattern }
}
```

Expansion verification checks that payload provenance agrees with actual inputs.

Standalone WindowView lowering is still incomplete.

## 11.2 Composite rewrite realization

A witnessed rewrite does not require every Graph Basis node to have a standalone kernel.

The Find rewrite currently has a CPU `ReferenceRewriteComposite` realization for the supported rank≤1 `E.` subset.

That route:

- preserves the Window → CellApply(Match) graph model;
- validates the same current RustJ Find semantics;
- can execute the whole rewrite as one reference composite;
- does not pretend that standalone WindowView is already generally lowerable.

GPU support for this rewrite remains unavailable until an appropriate realization exists.

## 11.3 Target-only feasibility

Existing `LoweringRegistry + TargetCapabilities` is reused.

Rewrite target feasibility reports:

- Supported
- RequiresCallFacts
- Unsupported

This is deliberately weaker than final legality.

It does not invent call-dependent facts such as:

- purity;
- observable error freedom;
- evaluation-order freedom;
- reassociation legality;
- precise access relation.

## 11.4 Planning reports

A `RewritePlanningReport` combines:

- target feasibility;
- resource evaluation;
- rule pruning permission.

Current readiness states:

- TargetUnsupported
- NeedsCallFacts
- NeedsResourceFacts
- ReadyForCosting

It does not select a winning candidate.

Actual selection requires later `TargetProfile / ResourceEstimate / CostProfile` integration.

---

# Part IX — Target architecture

## 12. Target model

Do not collapse target identity into one opaque object.

The target model is layered:

```text
BackendFamily
   ↓
ArchitectureTarget
   ↓
DeviceProfile
   ↓
RuntimeProfile
   ↓
resolved TargetProfile
```

Compiler-specific target lookup is separate from user J locales.

```text
device
  → architecture
  → architecture family
  → backend family
  → cpu/gpu class
  → generic
```

Lookup discovers candidates. Legality/resource/cost analysis chooses among them.

## 12.1 Built-ins and extensions

Hardware lowering applies uniformly to built-in J primitives and extension-derived operations.

Do not create a special hardware path only for extensions.

---

# Part X — Flow–Storage and materialization

## 13. Identity separation

Keep these identities distinct:

```text
SSA ValueId
≠ semantic StateResource
≠ physical BufferId
```

Likewise:

```text
semantic StorageRequirement
≠ physical MaterializationDecision
```

## 13.1 Materialization choices

Planning may eventually choose among:

- KeepVirtual
- FuseAway
- RegisterResident
- WorkingStateResident
- Bufferize(memory_space)
- Rematerialize(recompute_source_or_region)
- ExternalResource

Rematerialization is a physical/schedule choice, not deletion of semantic identity.

Its legality requires proofs for effects, errors, state dependencies, name bindings, and observable ordering.

---

# Part XI — Historical JAXA inheritance audit

## 14. Principles retained

Repeated review of `JAXA`, `JAXA-complier`, `japchae`, and `jaxa-analyzer` confirms that RustJ should preserve the following research ideas:

1. J notation creates useful graph structure that should survive parsing.
2. Basis should reflect meaningful access patterns rather than scalar arithmetic atoms.
3. Rewrite/equivalence reasoning belongs above physical realization.
4. Resource reasoning should be symbolic and compositional.
5. Flow and Storage are related but distinct analyses.
6. Static memory means logical determinability, not early physical offsets.
7. High-level structured operations may remain graph black boxes.
8. Backend realization may decompose or fuse them later.

## 14.1 Ideas intentionally not inherited as hard rules

Do not regress to superseded prototype assumptions such as:

- special `"RjP` precision syntax;
- rank change always being a fusion boundary;
- all reshape/transpose always being zero-copy;
- complete physical graph determination at parse time;
- fixed physical buffer offsets in semantic/J Graph IR;
- hardware register numbers embedded in primitive semantics;
- pre-resolving all names before the J parser uses them.

Current jsource remains the frontend semantic oracle.

## 14.2 ArrayFire and the J ArrayFire add-on

Checked: 2026-10-01.

Primary references:

- ArrayFire JIT: https://arrayfire.org/docs/jit.htm
- Unified Backend: https://arrayfire.org/docs/unifiedbackend.htm
- CUDA interoperability: https://arrayfire.org/docs/interop_cuda.htm
- Memory manager API: https://arrayfire.org/docs/group__memory__manager.htm
- Jsoftware `math_arrayfire`, pinned at `b0543c8278fe7a50e0ac9f938a936b4a84ee239b`:
  https://github.com/jsoftware/math_arrayfire/tree/b0543c8278fe7a50e0ac9f938a936b4a84ee239b
- J add-on manual:
  https://github.com/jsoftware/math_arrayfire/blob/b0543c8278fe7a50e0ac9f938a936b4a84ee239b/man.txt
- Alex Shroyer's historical J/ArrayFire GPU prototype:
  https://alexshroyer.com/papers/matmul_j_gpu.pdf

The useful ArrayFire lessons are narrower than "use ArrayFire as the RustJ GPU backend":

1. ArrayFire accumulates supported elementwise operations in a lazy AST and JIT-fuses them at evaluation boundaries. RustJ should compare this with its later Logical/Physical planning, not collapse J Semantic IR into an ArrayFire-like expression tree.
2. `eval` and `sync` separate evaluation/submission from completion. This is useful for future RustJ async-token/timepoint, lifetime, transfer, and external-library-call semantics.
3. ArrayFire's unified API hides CPU/CUDA/OpenCL/oneAPI backends without making backend placement part of array-language meaning. RustJ should preserve the same semantic/physical separation.
4. Device-pointer, stream, lock/unlock, and custom-memory-manager APIs are useful references for external-library ownership and synchronization boundaries.

The Jsoftware add-on is especially valuable as a concrete **library-adapter/offload case study**:

- J arrays are row-major while ArrayFire arrays are column-major, so the add-on performs an `rcc` conversion. RustJ should use this as a test case for keeping logical atom order separate from physical layout and choosing view/copy/consumer absorption only during physical planning.
- `families.ijs` maps concrete functions such as `af_add`, `af_mul`, and `af_sum`. It does not make arbitrary J adverbs, rank, or derived verbs automatically equivalent to ArrayFire operations. RustJ external routes therefore need explicit capability/precondition checks.
- The add-on tracks `af_array` handles and release/hold/device-GC state separately from J values. This supports RustJ's ValueId vs external buffer/handle separation.
- The add-on effectively inherits ArrayFire's `dim4` rank boundary. That is a backend capability limit, not a valid restriction on J semantics.

Do not inherit ArrayFire's physical limits upstream:

- do not identify `af::array` with a RustJ Logical Array/J noun;
- do not inherit rank<=4, column-major layout, or ArrayFire dtype coverage as J-language constraints;
- do not treat fixed ArrayFire reductions as the semantics of J's general `/` or `\\`;
- do not treat ArrayFire JIT fusibility as proof of RustJ graph-rewrite/fusion legality;
- do not treat the J add-on's FFI mapping as RustJ's compiler architecture.

For later adapter work, build a Graph Basis ↔ ArrayFire capability matrix for Elementwise, Reduce, Scan, Gather/Index, MatMul, Conv, Sparse, layout conversion, synchronization, and fallback. Benchmark cold JIT compile cost separately from warm cached execution, transfer, layout conversion, and materialization cost.

---

# Part XII — Validation policy

<a id="validation-policy"></a>

## 15. Semantic validation

For semantic changes:

- add reproducer/regression coverage;
- compare against pinned jsource where available;
- test word formation, enqueue, parser, and execution at the appropriate layer;
- preserve J error class even when diagnostics become richer;
- do not claim validation that was not actually run.

The user has requested that current development not depend on remote CI; source-level review and direct/local checks should be reported accurately.

### Semantic hard-case gate

Before optimizing a semantic feature, add corresponding differential/golden coverage for prefix agreement, zero-cell prototypes/heterogeneous assembly, expected-POS errors, assignment/lookup sequencing, fork/hook observable order, adverse/obverse, tolerance/fit and overflow/error precedence. Full-J completion is not a prerequisite for the first CPU slice. The matrix mean example below tracks cell semantics separately from physical execution.

## 15.1 Differential frontend validation

Frontend compatibility should eventually provide separate differential gates for:

1. word formation;
2. enqueue classification/metadata;
3. parser reduction behavior and result topology.

A mismatch in a test harness must first be distinguished from a true semantic mismatch.

---

# Part XIII — Current implementation status

<a id="current-implementation-status"></a>

## 16. Completed or substantially implemented

Code/document review baseline: the 2026-10-04 WI1 noun input metadata validation seam. Read the latest execution results and remaining boundaries together with the JE2 checklist. Earlier stage gates remain historical validation records.

- shared immutable FunctionEntity semantic DAG;
- explicit/direct-definition frontend support through immutable `DefinitionCode`, control-flow metadata, multiple root direct definitions, raw noun direct definitions, and UTF-8/source provenance; invocation/local frames, nested/other-tagged/computed forms, and Code-body J Graph/A3 lowering remain incomplete;
- J Graph IR as a separate analysis surface;
- Graph Basis / Execution Basis separation;
- structural opportunities plus initial graph rewrite/resource analysis;
- A3-v0 SSA Logical IR with Function/Region/Block/Return;
- shared target-independent execution contracts extracted into `execution_semantics.rs`;
- Execution Basis payloads, SemanticCheck, ConstraintSet/FactWitness, Effect/Speculation/PossibleErrors/DestinationRelation;
- A3 verifier and closed-plan correctness/reference executor;
- SemanticCapabilityView;
- ParameterizedLoweringRecipe + LoweringRegistry legality/candidate generation;
- prototype contiguous route partitioning;
- logical/physical array separation as an architectural invariant;
- G1 read-only CPU affine PhysicalArray, BufferId and BufferLease.

## 16.1 Remaining transitional structure

M1 is complete: `analysis::lower_graph()` builds `logical_ir::Plan` directly; `CompilationAnalysis.logical` and `Engine::analyze/analyze_a3` use that canonical plan. `transition_ir`, its container/API and `Plan::from_transition`/`TransitionProjection` have been removed. This does not complete frontend semantics, implicit cell application or physical execution.

Remaining transitions:

- `Value` still owns dense `CpuStorage` directly;
- `physical.rs` provides representation foundations, not a Physical Planner;
- callable/runtime `reduce/rank` summaries remain migration fields;
- frontend uses the same ordered 9-row matcher/runtime-analysis reduction engine; the old flat modifier/train heuristic reducer is removed. Supported name/POS/assignment and completed-result boundaries are implemented, but full enqueue/construction/local/locale/definition semantics still gate M2 completion;
- latest frontend validation (NV2 core recognition): Windows default/portable each **435 passed / 17 ignored**; fmt/clippy/build pass. Python **27 passed**. Each j64/AVX2 runtime route **5,380 cases / 5,380 passed / 0 runtime boundaries / 0 failed**; stages **10,810 checks**; words **6,623 cases / 0 failed**. Keep 257 capture-graph and two static boundaries separate. Of 145 vocabulary candidates, 143 verified POS / zero enqueue boundaries / two code-only rejected candidates, with 140 bare function bindings/ARs and three noun payloads compared. These are not execution coverage percentages. Keep ten frontend/two vocabulary reports and the separate recursion-oracle boundary record. Full upstream/definition acceptance/private C trace equivalence remain unverified.
- `RouteRegion` is a class + operation-range prototype;
- Schedule/Physical Plan, native CPU physical execution and external adapters are not complete.

## 16.2 Deferred / later

- production CUDA backend;
- broad AD/VJP transforms;
- aggressive resource pruning;
- mature multi-route partitioning;
- complete full-J implementation.

Linux/GitHub Actions CI is not a default architectural progress gate unless explicitly requested.

---

# Part XIV — Architecture convergence roadmap

<a id="architecture-migration-checklist"></a>

## 17. Active migration checklist

<a id="dynamic-boundary-checklist"></a>

### DB — dynamic semantic boundary migration (DB0–DB7)

Connect this checklist to M2→M3→M4 and WI3/WI4/M5–M8; it is not a parallel backend implementation program. **Continue M2 frontend convergence first.** Add regressions with each semantic implementation change; actual optimization belongs to a separately authorized stage.

- [x] **DB0 document contracts:** consolidate boundaries, check points, prohibited unproven transforms, failure handling and initial/pending implementation status in canonical/mirror documents.
- [ ] **DB1 structured boundary reports:** record source span, semantic phase, reason, required facts/witnesses, permitted analysis/rejected transforms and route capability. Distinguish unknown facts, invalid J, implementation coverage and route rejection while retaining valid-unsupported reasons.
- [ ] **DB2 witness validity:** separate parser POS, constructor snapshots and call-time lookup. Track binding/frame/locale/path/unbound-search dependencies and mutation between check and use. Connect actual noun metadata through WI4 and M5/M6.
- [ ] **DB3 effect/error boundaries:** connect lookup/write/context/I/O/resource/errors through ordered regions or equivalent explicit dependencies. Separate value/effect live-outs in forks/selectors, repeated named calls and assignment expressions.
- [ ] **DB4 initial execution route selection:** check necessary guards before effects in the Native CPU slice. On miss choose actual supported routing/reanalysis/coverage boundaries without invented J errors or replay. Extend external/CUDA routes separately after capability proof.
- [ ] **DB5 prerequisites for mid-execution transitions:** when needed, specify/verify continuation state, ownership, exactly-once effects and error positions first. Do not enable mid-execution fallback before completion or make full continuations an unconditional prerequisite for the first CPU slice.
- [ ] **DB6 Windows differential gates:** compare NAME rebinding/POS changes, unbound→bound locals, locale/path changes, noun snapshots followed by rebinding, value-dependent construction and errors/guard misses after effects with C base/AVX2. Check lookup timing, effect order, post-failure bindings and execution counts alongside value/type/shape. Report currently unsupported locale/execute cases separately as coverage.
- [ ] **DB7 later semantic/performance gates:** compare verified direct runtime with Logical/Physical execution on identical inputs. Include guard hit/miss and empty/boxed/sparse boundaries; measure performance/copies/allocations separately after semantic success. Frontend passes or metadata analysis do not establish backend execution/performance success.

<a id="nv3a-spelling-errors"></a>

**NV3a fixed spelling error classification — 2026-10-05.** Follow `jsrc/ws.c::spellin` and `jsrc/w.c::jtenqueue`: the installed, verified core dictionary takes precedence. An unregistered colon inflection or nonnumeric dot inflection is a spelling error. Numeric dots still enter numeric construction; strings and simple names retain their classification. One-digit constant functions pass through existing descriptors. Reviewed C rejects `99:`/`1.5:`/`_99:` as spelling errors. Remaining invalid characters/uninstalled primitives are spelling errors, not missing execution capabilities. Do not create arbitrary obsolete-word exception lists.

`name_:` is valid by-value/abandon lookup syntax. Validate its simple-name prefix, then retain a separate Unsupported boundary. A plainly malformed prefix such as `foo__:` after suffix removal is an ill-formed name. The subsequent NV3b validates bounded locative grammar. The subsequent NV3c covers NAME length limits/error precedence. Locale lookup/abandon effects and complex/extended/rational numeric grammar remain unfinished. NV3a completes the fixed spelling seam, with bounded name syntax covered separately by NV3b; full NV3/NV5 remain open. Invalid lexical spelling stays distinct from valid primitives' unsupported construction/execution. Preserve enqueue diagnostic phase, source span and word index.

`tools/spelling_conformance.py` compares a **651** matrix of 93 graphic ASCII bases (excluding quote) × 7 suffixes plus additional name/numeric boundaries with both DLLs. Verify error class, word formation and valid unsupported boundaries separately; do not count these as primitive execution or complete name/numeric grammar conformance. `tools/vocabulary_audit.py` now also requires Rust/C error equality for two code-only candidates and eight legacy words. The standard Windows runner includes spelling reports.

**NV3a validation:** native Windows default/portable each **465 passed / 17 ignored**, fmt/clippy/build passed; Python **30 passed**. Each DLL: spelling **667 cases / failed 0**, with **453** error checks (**426** spelling, **22** number, **1** name, **4** syntax), **208** accepted enqueue controls and **6** valid Unsupported boundaries. The matrix contributes **651** cases; quote grammar and full locative/numeric grammar remain outside this scope. Existing three runtime routes each retain **5,380 cases / 5,380 passed / failed 0**, stages **10,810**, words **6,623**, vocabulary POS **143**/binding **140**/nouns **3**. Scan **285 / failed 0** and **285 runtime prefix boundaries / executable prefix passes 0**, capture graph boundaries **257** and static boundaries **2** remain separate. Binary/source/DLL hashes in all sixteen reports were checked. Spelling passes do not establish execution support or GPU performance. No Linux tests, GitHub CI or CUDA ran.

<a id="nv3b-name-syntax"></a>

**NV3b bounded name syntax — 2026-10-05.** Extend the shared enqueue name validator from `sn.c::vnm/vlocnm`. ASCII names begin with a letter and contain alphanumerics/underscores. Distinguish simple names, trailing direct locatives/empty base locale `__`, indirect chains, final numeric debug-frame components and their negative form. Check leading zeros/x64's 18-digit numeric direct locale limit, malformed intermediate numbers/isolated underscores/excess underscores. Apply the same grammar to the underlying name after removing `name_:`'s suffix. The validator performs no locale/symbol lookup, constructs no noun/function and needs no additional heap allocation.

Valid locative/by-value names remain Unsupported; this does not implement locale lookup, debug-frame access or abandon effects. Malformed names produce ill-formed name with enqueue phase/span/word index. The NAME/full/simple-name/locale length limits and error precedence left open at NV3b are covered by the subsequent NV3c. Complete numeric grammar and locale execution remain NV3/DB2 work. Only bounded NV3b syntax is complete, not full NV3/DB2/locales. Definition `for_name.` classification also passes through shared enqueue, inheriting this bounded syntax validation.

`tools/name_syntax_conformance.py` generates short `a0_` combinations, direct/indirect/debug-frame controls, mixed-case/digit/underscore names with a fixed seed and each `name_:` form for both DLLs. Distinguish C enqueue errors from subsequent value/locale lookup errors. Those later errors establish lexical validity only, not runtime/lookup success. Retain `sn.c`/`w.c`/`ws.c`/`jerr.h`, probe and DLL hashes in reports and connect the audit to the standard Windows runner.

**NV3b validation:** native Windows default/portable each **466 passed / 17 ignored**, fmt/clippy/build passed; Python **30 passed**. Each DLL: name syntax **4,030 cases / failed 0**, with **1,133** simple names, **1,901** valid Unsupported names and **996** invalid names. Spelling **667 / failed 0**, vocabulary POS **143**/binding **140**/nouns **3**, three runtime routes each **5,380 cases / 5,380 passed / failed 0**, stages **10,810** and words **6,623** remain stable. Scan **285 / failed 0** and **285 runtime prefix boundaries / executable prefix passes 0**, capture graph boundaries **257** and static boundaries **2** remain separate; binary/source/DLL hashes in all eighteen reports were checked. Lexical validity does not establish locale runtime support or execution conformance. No Linux tests, GitHub CI or CUDA ran.

<a id="nv3c-name-limits"></a>

**NV3c NAME lengths/error precedence — 2026-10-05.** Add J-visible compatibility checks from `sn.c::nfs`. The underlying name's total byte length must satisfy **1 ≤ n < 32767**; otherwise report ill-formed name. The stored simple-name and locale portions must each be **at most 255 bytes**. Split direct locatives at the last locale separator, including empty/base locales. For indirect locatives, measure the **entire chain suffix** after the first `__`; bounding each component individually is insufficient. These are J compatibility conditions, not Rust allocator/storage/physical-layout limits.

Preserve check order: total length → final indirect numeric/debug-frame digit validation → simple/locale sizes → `vnm` grammar. Letters in the final numeric component produce ill-formed name even when another portion is oversized. Other malformed locatives can produce limit error before grammar failure because component sizes are checked first. For `name_:`, check the underlying name after removing the suffix. Primitive-inflection spelling checks retain earlier precedence. Rust performs these checks without allocation or introducing C NAME blocks/hash/symbol tables. Preserve enqueue diagnostic phase/span/word index.

Extend Windows name differentials with 254/255/256/257 and 32766/32767 boundaries, direct/indirect chains, simultaneous oversize/malformed text and by-value forms. Compare precise length errors and spelling precedence with C. Valid locale/by-value/debug lookup remains Unsupported; locale execution/abandon effects are not implemented. Complete numeric-notation validity remains NV3d. This closes NAME length/error convergence, not full NV3/NV5.

**NV3c validation:** native Windows default/portable each **467 passed / 17 ignored**, fmt/clippy/build passed; Python **30 passed**. Each DLL: name syntax **4,125 cases / failed 0**, with **1,135** simple names, **1,927** valid Unsupported names, **1,022** invalid names, **40** length-limit checks and **1** spelling-precedence check. Add **95** length/precedence fixtures to the previous 4,030 cases. Spelling **667 / failed 0**, vocabulary POS **143**/binding **140**/nouns **3**, three runtime routes each **5,380 cases / 5,380 passed / failed 0**, stages **10,810**, words **6,623** and Scan **285 / failed 0** remain stable. Keep **285 runtime prefix boundaries / executable prefix passes 0**, capture graph boundaries **257** and static boundaries **2** separate. Binary/source/DLL hashes in all eighteen reports were checked. Lexical/length comparisons do not establish locale runtime success. No Linux tests, GitHub CI or CUDA ran.

<a id="nv3d1-numeric-recognition"></a>

**NV3d1 numeric-word context/error validation — 2026-10-05.** `src/numeric_input.rs` checks whole-word dispatch from `wn.c::numcase/connum` during enqueue. Preserve the shared complex/based, extended integer, rational and precision conversion mode across every field of a numeric list. Per-field interpretation would miss differences between `1x` and `1.0 1x`, `1j2 1x` or `2b10 1x`. Validate suffixes/operands, rational infinities, rectangular/polar complex, based digits and p/x exponents before reporting malformed words as ill-formed number. Remove the heuristic that chose Unsupported merely from a numeric-family marker. Regress valid `1xr2`: C `numfd` reads the omitted numerator of `r2` as zero.

Separate Valid/Invalid/Unknown recognition. Validated extended/rational/complex/based payload construction remains Unsupported. Precision and incompletely verified platform-specific `strtod` hex/NaN-payload syntax retain distinct Unsupported reasons rather than guessed Invalid errors. Half/single and some quad combinations hit the supplied C engine's nonce boundary; count neither C success nor spelling/number failure. Ordinary integer/decimal words use the existing constructor directly without extra float parsing/normalized-string allocation. Preserve enqueue phase, word index and span; introduce no runtime target/array IR/physical allocation facts into numeric grammar.

`tools/numeric_syntax_conformance.py` compares scalar forms, crossed numeric lists, malformed suffixes/missing operands, infinities, 64-bit overflow, colon spelling precedence and precision/platform boundaries with both DLLs. Separate accepted noun controls, verified lexical errors, valid payload boundaries, unresolved recognition/error boundaries and C reference precision boundaries. The latter boundaries are not precise-error passes or numeric payload execution support. Connect the audit to the standard Windows runner; retain `wn.c`/`w.c`/`ws.c`/`jerr.h` and actual binary/DLL hashes in reports.

**NV3d1 validation:** native Windows default/portable each **469 passed / 17 ignored**, fmt/clippy/build passed; Python **30 passed**. Each DLL: numeric syntax **1,099 cases / failed 0**, with **110** accepted noun controls, **652** lexical-error equalities, **327** valid payload boundaries, **5** unresolved recognition boundaries, **4** C reference precision boundaries and **1** unresolved error boundary. That final boundary retains C ill-formed number versus Rust Unknown/Unsupported as unfinished validation, never a pass. Existing name syntax **4,125 / failed 0**, spelling **667 / failed 0**, vocabulary POS **143**/binding **140**/nouns **3**, three runtime routes each **5,380 cases / 5,380 passed / failed 0**, stages **10,810**, words **6,623** and Scan **285 / failed 0** remain stable. Keep **285 runtime prefix boundaries / executable prefix passes 0**, capture graph boundaries **257** and static boundaries **2** separate. Binary/source/DLL hashes in all twenty reports were checked. Numeric syntax checks do not establish exact/complex/based payload execution or full precision success. No Linux tests, GitHub CI or CUDA ran.

<a id="nv3d2a-quad-hex-recognition"></a>

**NV3d2a quad/Windows hex grammar validation — 2026-10-05.** `src/numeric_input.rs` preserves whole-word precision selection and validates `numfq` mantissas, fractional scale, lowercase `e`, 64-bit exponents, `fq` suffixes and infinity/NaN forms. `2fqz` now produces C's ill-formed number error. Distinguish an out-of-range exponent from signed overflow when combining exponent and scale; retain the latter as Unknown. This does not implement quad payload construction or arbitrary-precision allocation/resource errors.

Validate Windows `strtod` hex mantissas and optional binary exponents in Rust. C `numfd` does not terminate the nominal field and accepts `t >= s+n`. Consequently `0Xad90`/`0Xb1` can consume subsequent hex digits and be valid, whereas `_0X0ad90` has a negative nonzero magnitude and is rejected. `numbpx`, however, temporarily terminates at a `p`/`x` separator, so the preceding read window must be restricted. Pass nominal field lengths and actual read windows separately to preserve these distinctions. Add no C FFI or C kernel dependency.

Reject negative hex polar magnitudes only when their leading bit/exponent proves a nonzero value under the default IEEE binary64 environment. At this checkpoint, accept zero mantissas and retain potentially underflowing negative zero, combined-exponent overflow and unproved hex-ratio signs as Unknown. NV3d2b1 below subsequently verifies default-rounding negative zero and NaN word boundaries. Changed rounding/FTZ environments, parenthesized NaN-payload word formation, complete platform `strtod` extensions and numeric-construction resource/error equivalence remain **NV3d2b**. Valid complex/based/quad payload construction remains Unsupported.

Source: pinned [wn.c](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wn.c), specifically `numfd`, `numfq`, `numj` and `numbpx`; record Windows DLL and source revisions separately. NV3d1 counts are historical; its unresolved quad/hex boundaries are updated by this stage's results below.

**NV3d2a validation:** native Windows default/portable each **471 passed / 17 ignored**, fmt/clippy/build passed; Python **30 passed**. Each DLL: numeric syntax **2,078 cases / failed 0**, with **182** accepted noun controls, **1,084** lexical-error equalities, **607** valid payload boundaries, **2** integer conversion boundaries, **200** C reference precision boundaries and **3** unresolved recognition boundaries. Unresolved error boundaries are now **0**. `frontend_probe` exposes the original Unsupported reason in a separate diagnostic field. Classify validated grammar, integer-overflow conversion, C precision and unresolved grammar from actual reasons, rather than claiming validity from fixture scope alone. Do not sum boundaries as numeric execution successes. Existing frontend/runtime/name/spelling/vocabulary/Scan comparisons remain stable; check source/DLL/binary hashes in all twenty reports. Keep **285 runtime prefix boundaries / executable prefix passes 0**, capture graph boundaries **257** and static boundaries **2** separate. No Linux tests, GitHub CI or CUDA ran. Full NV3d/NV3d2 remain open.

<a id="nv3d2b1-rounding-nan-words"></a>

**NV3d2b1 default rounding and NaN word boundaries — 2026-10-05.** `hex_nonnegative` examines the leading mantissa bit and remaining bits in the actual read window. Under default IEEE binary64 round-to-nearest, ties-to-even, negative magnitudes at or below `2^-1075` round to `-0`, making them valid polar inputs. Magnitudes above that midpoint remain negative and nonzero, producing ill-formed number. `_0X1P_1075ad90` and `_0X1P_9999ad90` now have validated grammar with unsupported payload construction; `_0X1.00000000000001P_1075ad90` matches C's error. Preserve sticky bits even at the end of long mantissas. Validate only this sign condition without constructing numeric payloads or calling C FFI.

`1jNaN(1)`, `1jnan()`, `1jNAN(foo)` and `1j_nan(1)` split into a numeric prefix, parentheses and an optional inner word in both C and Rust. Do not introduce Windows `strtod` parenthesized NaN payload syntax into J numeric words. C reports syntax error for the full sentences, while Rust stops at unsupported complex noun construction. Therefore the four fixtures establish word-formation equality and a **payload/parser coverage boundary**, not syntax-error equivalence or parser execution success. Add unsigned/negative NaN, Infinity and ratio forms as separate numeric fixtures.

This stage verifies polar signs under the default rounding environment. Externally changed rounding/FTZ environments were not tested. For grammatically validated very large hex exponents, use i128 intermediate arithmetic and sign-preserving saturation to decide only the polar sign. On supported 64-bit hosts the mantissa-length adjustment is smaller than the i128 range, preserving ordering against the threshold. Do not use this as a numeric payload construction rule. Overflow when combining quad fractional scale and exponent, converted hex-ratio signs and arbitrary-precision payload allocation/resource-error equivalence remain **NV3d2b2**. Do not reproduce C signed overflow in Rust or exhaust system memory to guess resource errors. Source evidence: pinned [wn.c](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wn.c), specifically `numfd`, `numj`, `numfq` and `numxTEMP`; word formation follows [w.c](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/w.c).

**NV3d2b1 validation:** native Windows default/portable each **473 passed / 17 ignored**, fmt/clippy/build passed; Python **30 passed**. Each DLL: numeric syntax **2,201 cases / failed 0**, with **182** accepted noun controls, **1,140** lexical-error equalities, **672** valid payload boundaries, **2** integer conversion boundaries, **200** C reference precision boundaries, **1** unresolved recognition boundary and **4** NaN word-formation boundaries. Unresolved error boundaries are **0**. The remaining recognition case is scale/exponent overflow in `2.1e_9223372036854775808fq`, not an execution success or precise-error pass. Record C syntax error and Rust payload Unsupported separately for the four NaN cases. Existing three runtime routes each **5,380 cases / 5,380 passed / failed 0**, stages **10,810**, words **6,623**, name syntax **4,125**, spelling **667**, vocabulary POS **143**/binding **140**/nouns **3** and Scan **285 / failed 0** remain stable. Keep **285 runtime prefix boundaries / executable prefix passes 0**, capture graph boundaries **257** and static boundaries **2** separate. Verify source/DLL/binary hashes in all twenty reports. No Linux tests, GitHub CI, CUDA or changed floating-point environment validation ran.

<a id="nv3d2b2a-exact-hex-ratios"></a>

**NV3d2b2a exact hex ratios and quad construction boundaries — 2026-10-05.** A ratio in a polar magnitude cannot be validated from the source sign alone. Pass actual read windows to `real_value` and use exactly representable binary64 hex operands for internal sign validation. Accumulate a bounded u64 mantissa, remove trailing zero bits, and prove at most 53 significant bits with a normal exponent or an exact subnormal multiple. Only then construct an exact internal operand with `f64::from_bits`. This does not support J noun/complex/quad payload execution. Retain accumulated-mantissa overflow, additional rounding and numeric construction of hex overflow/underflow operands as conservative boundaries.

Follow C `numfd` ratio handling: a zero denominator produces signed zero or infinity using numerator/denominator sign xor; otherwise apply `0 <= magnitude` to the division result. `_0X1r2ad90` is invalid, `_0X1r_2ad90` is valid, and `_0X1P_1074r2ad90` rounds to `-0`, making it valid. Reject NaN results. Denominators may consume hex digits beyond their nominal fields, so do not assume the denominator of `0X1r0X0ad90` is zero. Add no C FFI; preserve enqueue error spans/indices and whole-word numeric modes.

Classify `2.1e_9223372036854775808fq` as a **quad scale/exponent construction boundary**, rather than unresolved lexical grammar. Verified mantissa/suffix/exponent grammar does not prove C signed overflow in combined scale, numeric construction or allocation/resource-error equivalence. Separate the Unsupported reason and report category; count neither precise J-error equality nor numeric execution success. Preserve ill-formed-number precedence when a malformed field is also present.

Source: pinned [wn.c](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wn.c), specifically `numfd` ratio/signed-zero handling, `numj` polar nonnegative checks, `numfq` scale arithmetic and `numxTEMP` resource errors. Non-exact hex ratios, changed rounding/FTZ environments, defined quad-scale overflow and payload allocation/resource-error equivalence remain **NV3d2b2b**. These are RustJ implementation boundaries, not J language restrictions.

**NV3d2b2a validation:** native Windows default/portable each **474 passed / 17 ignored**, fmt/clippy/build passed; Python **30 passed**. Each DLL: numeric syntax **2,485 cases / failed 0**, with **182** accepted noun controls, **1,244** lexical-error equalities, **850** valid payload boundaries, **2** integer conversion boundaries, **200** C reference precision boundaries, **1** quad construction boundary, **4** NaN word-formation boundaries, **1** unresolved recognition boundary and **1** unresolved error boundary. The latter two cases are `_0X1P_1075r1ad90` and `0X1P9999r0X1P9999ad90`; retain C success/ill-formed number versus Rust Unsupported explicitly. Do not turn unresolved boundaries into precise-error or execution passes. Add **280** crossed exact-operand ratio fixtures, **2** unresolved conversion fixtures and **2** malformed/quad-construction precedence fixtures to the existing corpus. Existing frontend/runtime/name/spelling/vocabulary/Scan comparisons remain stable; verify source/DLL/binary hashes in all twenty reports. Keep **285 runtime prefix boundaries / executable prefix passes 0**, capture graph boundaries **257** and static boundaries **2** separate. No Linux tests, GitHub CI, CUDA, changed FP environment or memory-exhaustion tests ran.

<a id="vocabulary-migration-checklist"></a>

### NV — current J vocabulary convergence

See the [current vocabulary audit](#current-j-vocabulary). Follow M2 frontend sequencing; do not launch simultaneous task/fold/GPU executor implementation.

- [x] **NV0** Read the current NuVoc index/relevant pages and distinguish source spelling/POS and C DLL provenance. Correct Taylor/obsolete entries and missing current forms in the older inventory.
- [x] **NV1** Support `[.`/`].`/`]:` through normal core enqueue/shared constructors; regress noun/verb results, NAME snapshot/late lookup, modifier trains and discarded noun effects/errors.
- [x] **NV2** Extend pinned core spelling/POS recognition separately from construction/execution capability. Add 108 descriptors and actual `a.`/`a:` nouns. Both DLLs match 143 accepted POS, 140 bare function bindings/ARs and three noun payloads. Inventory passes are not execution support.
- [ ] **NV3** Generalize precise invalid/obsolete spelling errors against C `spellin`/enqueue. Distinguish valid unsupported primitives from invalid spelling without arbitrary exception lists.
- [x] **NV3a** Generalize fixed ASCII spelling and unregistered inflection errors after current core dictionary lookup, without obsolete exception lists. Valid unsupported `name_:`/numeric families remain separate coverage boundaries; full NV3 is still open.
- [x] **NV3b** Validate bounded direct/indirect/debug-frame/by-value name grammar through shared enqueue. Valid lookup remains Unsupported; NV3c covers NAME lengths, while locale execution and numeric grammar remain open.
- [x] **NV3c** Verify NAME/full/simple-name/locale storage length limits and enqueue error precedence against C `nfs` and both Windows DLLs.
- [ ] **NV3d** Distinguish valid unsupported numeric families from genuinely ill-formed numbers against C `connum`/`wn.c`.
- [x] **NV3d1** Generalize whole-word numeric mode selection and validated extended/rational/complex/based errors. Distinguish unsupported payload construction from Unknown grammar.
- [ ] **NV3d2** Verify dedicated quad grammar, platform-specific `strtod` extensions and numeric-construction resource/error boundaries. Do not guess Unknown syntax is Invalid.
- [x] **NV3d2a** Verify bounded quad/Windows hex grammar and field/read-window distinctions against both DLLs, separately from payload execution support.
- [ ] **NV3d2b** Verify resource, scale-overflow, rounding/FTZ, NaN-payload and remaining platform-conversion boundaries.
- [x] **NV3d2b1** Verify default ties-to-even hex polar signs and NaN-parenthesis word boundaries against both DLLs. Separate very-large-exponent sign decisions from numeric payload construction.
- [ ] **NV3d2b2** Verify quad scale overflow, hex-ratio signs, changed FP environments and payload allocation/resource-error equivalence. Never convert missing construction support into a J error.
- [x] **NV3d2b2a** Verify exact-hex polar ratio signs, signed zero, denominator read windows and quad construction boundaries against both DLLs.
- [ ] **NV3d2b2b** Verify hex ratios needing extra rounding/overflow, defined quad scale, changed FP environments and payload allocation/resource-error equivalence.
- [ ] **NV4** Review remaining families' valence/rank/construction/effect/error contracts sequentially. Do not collapse Key dyad, Fold, task/pyx, precision or scope semantics to aliases/pure array kernels.
- [ ] **NV5** Reconcile complete NuVoc forms/structural/control inventories with the support matrix. Update Windows differential gates at each step; separate full J support from limited-corpus success.

**NV1 gate (historical, 2026-10-05):** native Windows default/portable each **431 passed / 17 ignored**; fmt/clippy/build pass; Python **27 passed**. With **37** new common statements, each C base/AVX2 runtime route: **5,368 cases / 5,368 passed / 0 runtime boundaries / 0 failed**; stages **10,810 checks**; words **6,623 cases**, zero failures. Keep **254 capture-graph** and **2 static** boundaries separate. Verify actual source/reference/binary hashes in ten frontend and two vocabulary reports. Of **145 vocabulary candidates**, **33 enqueue POS verified / 110 Rust Unsupported / two source-code-only rejected candidates (`?:`, `` `. ``)**; both DLLs reject eight historical/invalid spelling fixtures. These are not complete NuVoc/J runtime coverage percentages. At this historical gate NV2–NV5 and DB1–DB7 remained pending, including ordered effects and static discarded-noun preservation, unsupported task/fold/precision/general scope execution. Optimization/CUDA/Linux execution tests/GitHub CI remain deferred.

**NV2 gate (2026-10-05):** native Windows default/portable each **435 passed / 17 ignored**; fmt/clippy/build pass; Python **27 passed**. Add 12 common C statements: each DLL's three runtime routes **5,380 cases / 5,380 passed / 0 runtime boundaries / 0 failed**; stages **10,810 checks**, words **6,623 cases**, zero failures. Keep **257 capture-graph** and **2 static** boundaries separate. Of 145 vocabulary candidates, **143 enqueue POS verified / zero Rust enqueue boundaries / two code-only rejected candidates**, with **140 bare function parser bindings/ARs and three noun payloads** compared. These counts are not full NuVoc/J execution coverage. Verify actual DLL/binary/source hashes in twelve reports and mark NV2 complete. Next is NV3; NV4/NV5 and DB1–DB7 remain pending.

The Korean canonical document contains the authoritative detailed M0–M6 checklist. The English mirror follows the same order:

```text
M0  Freeze module ownership and dependency boundaries
 ↓
M1  Complete: logical_ir::Plan is the sole canonical execution IR
    Direct graph lowering; transition container/output/API removed
 ↓
M2  Cut over to the jsource-compatible
    Word Formation → Enqueue → 9-row Parser frontend
 ↓
M3  Finish code-level Logical/Physical Array separation
    and remove misleading representation/layout seams
 ↓
M4  Build the first compiler-native CPU vertical slice
    Logical IR → Schedule → Physical Plan → Executor
 ↓
M5  Add route/schedule/resource/cost selection
 ↓
M6  Add verified external routes such as ArrayFire/MLIR/StableHLO;
    production GPU work remains deferred until explicitly resumed
```

The ordering is intentional. Do not grow the optimizer or GPU backend while duplicate canonical IRs and frontend semantic uncertainty remain.

### JE0–JE6 auxiliary semantic track — JEntity / contextual higher-order views

This track does **not** insert a new milestone into the M0–M6 critical path.

- JE0 audit proceeds alongside M2.
- A minimal JE1 `JEntity` boundary carrier may be introduced during M2 when it replaces one duplicated carrier seam (`AssignedValue`/`SymbolValue`/`ParserNameBinding`/`FunctionOperand`) under differential tests. Do not wait for M2 to finish only to perform a broad migration later.
- JE2 parser/binding/assignment convergence may therefore be part of M2.
- JE3+ higher-order collection work waits until frontend semantics are stable; changes that interact with noun storage wait for the M3 logical/physical boundary.
- No JE stage should unnecessarily block the first M4 CPU vertical slice.

Goal: adopt the semantic idea behind jsource's common carrier specifically at the semantic `RHS = NOUN + FUNC` boundary without copying its C allocation/runtime layout. `JEntity` is a thin parser/binding/assignment/operand boundary sum type, not a universal base class for `Value`, `FunctionEntity`, or every IR node. Function entities do not acquire noun-style semantic shape/rank. Higher-order collections start as operator-specific interpretation views; a generic shaped entity collection is extracted only if multiple J semantics require the same abstraction.

#### JE0 — audit current semantic carriers and jsource correspondence
- [x] Fix the goal: `JEntity` is a semantic abstraction, not a common physical-allocation abstraction.
- [x] Confirm that current `Value`, `FunctionEntity`, and `FunctionOperand` can be mapped to the common-entity idea without copying jsource storage layout.
- [x] Recheck current jsource `RHS = NOUN + FUNC` and `FUNC = VERB + ADV + CONJ`; scope RustJ `JEntity` to semantic RHS values rather than all `A` block classes.
- [x] Confirm that current jsource does not use AN/AR for functions; do not assign noun-style array shape/rank to FunctionEntity.
- [x] Recheck assignment/name lookup: assignment can transport Noun/Verb/Adverb/Conjunction RHS values, while noun lookup and function nameref late lookup have different timing semantics.
- [x] Recheck bident/trident construction: some parser actions produce an immediate Noun, so the generic construction result is `JEntity`, not always Function.
- [x] Recheck gerund conversion: `cg.c::jtfxeachv/jtfxeach` copies source rank/shape into an internal BOX-tagged carrier whose slots may contain function-typed A values; jsource itself says the result only *claims* to be a box array. Treat this as a runtime realization trick, not a semantic precedent for EntityArray.
- [x] Audit current RustJ carrier duplication: `FunctionOperand`, `ParserNameBinding`, `AssignedValue`, runtime `SymbolValue`, and expression noun/function variants overlap enough that a minimal JEntity can reduce M2 rework.
- [x] Record that `Verb { target, entity }` and `FunctionEntity` have not fully converged; audit `VerbTarget` before fixing the payload of `JEntity::Function`.
- [x] Distinguish lexical NAME from executable function nameref: unresolved lexical NAME is not a JEntity, while a resolved function nameref can be a Function JEntity with NameRef identity.
- [x] Pin this independent audit to current jsource master `0db94e768a845e2583c01d00538c3d16379677bb` (2026-10-03).
- [x] Inventory `Value`, `FunctionEntity`, `Verb`/`VerbTarget`, `FunctionOperand`, parser stack items, `ParserNameBinding`, `AssignedValue`, runtime `SymbolValue`, binding results, `NameRef`, `DefinitionCode`, and gerund decode/views.
- [x] Keep runtime `p.c` and tacit-translator `pv.c` evidence distinct; do not use translator actions such as `pv.c::jtvis` as the sole oracle for runtime observable semantics.
- [x] Record representative jsource differential cases where nouns/functions cross the same parser/binding/assignment boundary.
- [x] Identify duplicate noun/function carrier enums and prevent current `CpuStorage` from becoming a canonical JEntity dependency.

#### JE0 carrier audit and first migration seam (2026-10-04)

The following inventory is the pre-JE1 audit snapshot. The JE1 record below owns the current API and removal of AssignedValue.

| Current carrier | Identity, ownership and lifetime | Metadata and migration decision |
|---|---|---|
| `Value` / `Data` | J noun type/shape/atom order; Arc boxed children and shared sparse semantics. Owned dense clones copy; frozen clones share | CPU backing is transitional. JEntity transports Value without adding CpuStorage, host slices, BufferId, layout or device APIs |
| `FunctionEntity` | Immutable Arc DAG owning POS/head/ordered operands, shared definition Code and intrinsic noun snapshots | Arc FunctionEntity suffices as the Function payload. Callable rank contracts are distinct from noun shape |
| `Verb` / `VerbTarget` | Span + target + Arc identity. Primitive/Named duplicate heads; Derived is a migration marker | Production execution resolves the entity DAG; direct target checks remain in a parser test host. Preserve occurrence spans separately rather than promoting target to semantic identity |
| `FunctionOperand` | Shared child function or frozen concrete noun with operand span | Payload overlaps JEntity, but provenance differs. Leave it out of the first seam; use a later zero-loss adapter |
| parser `Item` / `ParseValue` | Class/source/provenance/flags/occurrence/span override; noun Expr + height, function DAG/Verb wrapper | Deferred/abstract noun structure and parser controls are not concrete entities. Keep lexical NAME/target/control outside JEntity POS |
| `ExprKind` / `Program` | Literal/function results plus Group/ReadName/Monad/Dyad, reductions and writes | Computation structure is not a duplicated RHS carrier; preserve application graphs |
| `ParserNameBinding` | Concrete noun snapshot, abstract noun class, function POS, known modifier with version | Keep this lookup-observation enum. Unknown values/POS observations are not entity snapshots |
| `AssignedValue` | Concrete row-7 Noun/Verb/Modifier transport to/from the host | **First JE1 seam**: Noun/Function variants, DAG-owned POS, with occurrence/height/assignment provenance outside the entity |
| runtime `SymbolValue` / `Binding` | Frozen noun or shared function wrapper plus Engine-local NameVersion; replaced nouns retire to pool | Duplicates concrete RHS payload. First replace only the host boundary adapter; defer full symbol table/pool migration to JE2+ |
| binding results / `BoundProgram` | Analysis reads/versions/dynamic function refs/pending write, not a cached executable plan | Keep entity identity separate from binding proofs; prepare does not commit |
| lexical NAME / `FunctionHead::NameRef` | Unresolved queue spelling/flags versus executable expected-POS function identity after lookup | Preserve noun snapshots and function late lookup timing |
| `DefinitionSource` / `DefinitionCode` | Shared source/context/spans and immutable body/valence/control metadata; no invocation locals | Function owns Arc Code. Noun DD remains Value. Assigning an alias does not invoke the body |
| gerund noun / `decoded_gerund` | Boxed source noun and ordered decoded function Arc vector as execution auxiliary, not source edges | Vector retains order/snapshots but lacks source shape/lookup observations itself; use parent noun/span and capture observations. Audit operator-specific shape needs in JE3; introduce no EntityArray |

Duplicate decision: AssignedValue/SymbolValue is the first concrete RHS seam. FunctionOperand overlaps payload with different provenance. ExprKind, ParserNameBinding, stack/control and binding observations retain distinct roles. JE0 itself did not implement a JEntity API. Track completed JE1 and remaining JE2–JE6 boundaries below.

Evidence: at audit revision `0db94e768a845e2583c01d00538c3d16379677bb`, p.c L87–96 explicitly declares the tacit-translator cases table. Runtime ptcol dispatch and row 7 at L1006–1043 assign the stacked CAVN RHS and leave it on the stack; pv.c::jtvis L158 is a translator action, not a runtime assignment oracle. sc.c::jtnamerefacv L364–397 distinguishes noun values from expected-POS function namerefs. cf.c L292–308 includes immediate `{0,NOUN}` results. cg.c L101–121 uses a source-shaped internal BOX realization, not semantic EntityArray. Native Windows Python confirmed matching declarative row predicates/constructor dispositions between the new and existing reviewed sources; five file hashes are in reports/entity-carrier-source-audit.json. This is neither runtime ptcol trace equivalence nor validation of a DLL built from the new source revision.

Compatibility bug fixed during JE0: row 7 resolved every modifier for application before assignment, rejecting explicit adverb/conjunction aliases. It now transports the stacked RHS unchanged. Nameless modifiers were already stacked by value; nonnameless modifiers retain POS-bearing NameRefs. C 5!:1 confirms original-name heads for explicit/derived modifier aliases. Static prepare preserves POS-known alias assignment even when application semantics are unsupported. Assignment does not run a body or create invocation locals; explicit body execution remains a separately tracked gap.

**JE0 gate:** five Rust regressions cover grouped assignment result/POS/commit identity for all four RHS classes; noun snapshots, function late lookup and POS mismatch; Arc sharing/lifetime after host drop for a 48-level Hook DAG; shared payload/rebinding lifetime for a 65,536-atom noun snapshot; and explicit modifier alias static non-commit, runtime NameRef/POS/span, no body execution, late lookup and POS changes. Add **71 C corpus/stage cases**: 13 noun assignments, 19 noun values, 33 function class/atomic representations and 6 J errors. Valid explicit adverb/conjunction alias cases match C without Unsupported waivers.

Windows default/portable each: **363 passed / 17 ignored**; fmt/clippy/build pass. Python: **27 passed**. For each j64/AVX2 direct/semantic-reference/parser-capture route: **4,810 cases / 4,806 passed / 4 existing runtime boundaries / 0 failed**; stages: **9,990 checks**; words: **6,618 cases**. Record 106 capture-graph boundaries separately from 2 static boundaries. The new source audit pin `0db94e7...`, conformance source pin `13994ff...`, and actual DLL release `ded7793...` are distinct. No claim is made for a newly built revision DLL, full upstream suite, private runtime traces or explicit body invocation acceptance.

**JE0 handoff:** pass the AssignedValue/runtime-assignment seam to JE1. Existing M2 gaps, including semantic nested DD, remain. Neither a broad frontend rewrite nor a JE0/JE1 prerequisite for the first M4 CPU slice is introduced.

Sources: [runtime p.c](https://github.com/jsoftware/jsource/blob/0db94e768a845e2583c01d00538c3d16379677bb/jsrc/p.c#L1006), [translator pv.c](https://github.com/jsoftware/jsource/blob/0db94e768a845e2583c01d00538c3d16379677bb/jsrc/pv.c#L158), [nameref sc.c](https://github.com/jsoftware/jsource/blob/0db94e768a845e2583c01d00538c3d16379677bb/jsrc/sc.c#L364), [constructor cf.c](https://github.com/jsoftware/jsource/blob/0db94e768a845e2583c01d00538c3d16379677bb/jsrc/cf.c#L292), [gerund cg.c](https://github.com/jsoftware/jsource/blob/0db94e768a845e2583c01d00538c3d16379677bb/jsrc/cg.c#L101).

#### JE1 — introduce the minimum common JEntity identity
- [x] Design minimal `JEntity`/`JEntityRef` as a **boundary carrier**, with direct semantic variants `Noun` and `Function`, the latter carrying Verb/Adverb/Conjunction POS.
- [x] Do not use JEntity as a base class that merges `Value` and `FunctionEntity` internals. Start at exactly one well-tested parser/binding/assignment/operand seam.
- [x] Decide whether `Arc<FunctionEntity>` is sufficient for `JEntity::Function` or whether any current `Verb`/`VerbTarget` semantics must survive.
- [x] Keep lexical NAME, unresolved references, binding/version, and provenance in separate reference/control structures rather than inventing more JEntity POS variants.
- [x] Reuse shared `FunctionEntity`; do not duplicate Verb/Adverb/Conjunction payloads.
- [x] Keep noun identity logical and free of BufferId/layout/device state.
- [x] Separate entity identity from provenance/binding metadata where appropriate.
- [x] Add sharing, round-trip, POS-mismatch and error regressions.

#### JE1 implementation — minimum JEntity at assignment (2026-10-04)

This is the JE1 introduction snapshot. The JE2 record below owns removal of temporary SymbolValue/Verb adapters and the current namespace carrier.

- Add `semantic::JEntity::{Noun(Value), Function(Arc<FunctionEntity>)}` and borrowed `JEntityRef::{Noun(&Value), Function(&FunctionEntity)}`. Moving the owning carrier preserves payloads; as_ref inspects without copies, allocation or refcount updates. JEntity deliberately has no automatic Clone because cloning an Owned Value can copy the full noun payload. Borrowed Copy/Clone only copies references.
- `RuntimeParserHost::assign(name, JEntity) -> Result<JEntity>` transports all four row-7 RHS classes through one boundary. Remove AssignedValue. FunctionEntity owns actual POS; add no separate Verb/Modifier payload variants.
- Keep verb occurrence spans/compatibility targets, noun Expr height, source/provenance/occurrence/assignment flags outside the carrier. Capture Commit identity/class/source and binding versions stay in their existing paths. The host still freezes noun assignment and shares its returned/symbol payload, preserving replacement/pool-retirement policy.
- Keep SymbolValue as the first-seam compatibility adapter. Verb::from_entity checks Verb POS and reconstructs Primitive/Named/Derived targets and intrinsic span from the shared entity. It copies neither function DAG nor DefinitionCode and does not fix NameRefs to current bindings. Parser reinsertion reuses original occurrence-wrapper span/target.
- Keep lexical NAME, abstract noun/POS observations, ParserNameBinding, ExprKind, FunctionOperand, symbol table and gerund auxiliaries outside this seam. Add no BufferId/stride/layout/target/device/schedule APIs to JEntity and no noun shape/rank to Function. Existing Value CPU backing remains a migration artifact, not completed logical/physical separation.

Reuse all 71 JE0 differential cases for grouped/chained assignments, noun snapshots, function late lookup, explicit modifier aliases, POS errors and effect/provenance across the production boundary. Extend the 48-level Hook DAG test with JEntity move/borrow round-trip and unchanged refcounts. New tests cover a 65,536-atom Owned noun's payload pointer across move/borrow, explicit Verb/Adverb/Conjunction POS and shared DefinitionCode, and runtime Verb-adapter identity/span/modifier-POS rejection.

**JE1 gate:** Windows default/portable each: **366 passed / 17 ignored**; fmt/clippy/build pass. Python: **27 passed**. For each j64/AVX2 direct/semantic-reference/parser-capture route: **4,810 cases / 4,806 passed / 4 existing runtime boundaries / 0 failed**; stages: **9,990 checks**; words: **6,618 cases**. No new language form was added, so rerun all 71 JE0 cases and the complete existing corpus without expansion; verify all ten report binary/source hashes. Keep 106 capture-graph and 2 static boundaries separate. Full upstream tests, definition invocation acceptance, a newly built audit-revision DLL and private C trace equivalence remain unverified. Conformance sources use `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`, actual reference DLL release uses `ded7793fe5795d79eda8e7138dce94aa056edf78`, distinct from the JE0 audit pin `0db94e768a845e2583c01d00538c3d16379677bb`.

The minimum JE1 API and first boundary are complete; SymbolValue convergence is handed to JE2. Do not force deferred noun/application structure or lookup observations into concrete JEntity. Explicit body invocation/scope, semantic nested DD, JE3+ higher-order views, broader storage migration and full J conformance remain incomplete. Optimization, CUDA and GitHub CI stay deferred.

#### JE2 — converge parser/binding/assignment transport
- [x] Provide the common borrowed JEntityRef through `FunctionOperand::as_entity_ref()` and preserve provenance through `span()`. Retain the owning enum for noun spans and shared function ownership.
- [ ] Let parser stack/value transport use a common entity handle while preserving jsource 9-row POS/class rules. Runtime rows 0–2 and row 7 completed-result transport are implemented below; convergence of all stack variants remains separate.
- [x] Connect completed runtime nouns and all four row-7 RHS classes through CompletedParseResult/JEntity. Keep deferred Expr, NAME and control separate from concrete entities.
- [x] Move modifier-train Noun/Verb/Adverb/Conjunction operands through the same completed-result boundary, preserving noun source spans/freezing and function DAG identities.
- [x] Connect rank/@: conjunction operands through completed-result transport, preserving right-before-left audits, quiet gerund fallback and original Expr spans.
- [x] Connect noun-left fork constants and supported explicit/direct definition mode/body/results through completed-result transport. Definition invocation remains incomplete.
- [x] Audit remaining adapters after constructor/assignment migration and unify duplicate capture identity inspection through a borrowed helper.
- [x] Generalize assignment to write and return the same assigned `JEntity`. Binding.value and the runtime host now use JEntity; remove SymbolValue.
- [ ] Preserve expected-POS checks, late binding, binding versions, and observable effect order. Top-level runtime lookup/Verb/Modifier checks are implemented; full local/locale/definition scopes remain incomplete.
- [x] Preserve the jsource name-lookup asymmetry: noun names may deliver the looked-up value/snapshot, while function names may require a nameref resolved again at execution. JEntity bindings preserve timing/POS/version semantics under the existing 71 cases plus 11 noun/function replacement cases.
- [ ] Use the same boundary for explicit/direct definitions across static/runtime paths.

#### JE2 implementation — JEntity namespace and runtime results (2026-10-04, partial JE2)

- Remove runtime SymbolValue; store `Binding { value: JEntity, version: NameVersion }`. Function bindings own only Arc FunctionEntity without duplicated Verb wrapper/span/target. Runtime final results use the same JEntity. ExprKind remains parser computation/dependency structure.
- `commit_binding(name, JEntity) -> Result<JEntity>` checks version increment first, freezes nouns once, and shares namespace/returned payloads. Functions share the same DAG. Host assignment delegates here; preserve noun replacement/OutputPool retirement. Add no automatic JEntity Clone; explicitly share only frozen Values or function Arcs.
- Preserve noun lookup snapshots, DAG-owned function POS, by-value nameless modifiers, POS-known static aliases and late modifier NameRef chains/versions. Replace implicit enum-variant class checks with explicit POS checks at lookup. A Verb NameRef rebound to a modifier retains domain error and current_name context.
- Remove the now-unused Verb::from_entity namespace adapter and its dedicated unit test. Migrate its identity/span/POS guarantees to actual commit/lookup tests. Parser occurrence wrappers/VerbTarget remain separate from intrinsic function identity. Storage representation, Value internals, gerund/views and optimizer/target policy are unchanged.

Three new runtime unit tests verify stored/returned pointers for a 65,536-atom noun and shared identities/versions for all RHS classes; noun/function version-overflow rejection preserving bindings/pool/commit; and unified Function expected-POS/domain/current_name behavior. An integration regression covers cache limits 0/4096, noun→Verb→Adverb→Conjunction replacement with a retained noun alias, bounded retirement after the last alias drops, and noun reassignment. Replace the previous adapter-only test with these production-path checks. Add **11 C corpus/stage replacement cases**, bringing entity-boundary fixtures to **82**.

**Namespace seam gate:** Windows default/portable each: **369 passed / 17 ignored**; fmt/clippy/build pass. Python: **27 passed**. For each j64/AVX2 direct/semantic-reference/parser-capture route: **4,821 cases / 4,817 passed / 4 existing runtime boundaries / 0 failed**; stages: **10,001 checks**; words: **6,618 cases**. Keep 108 capture-graph and 2 static boundaries separate. Full upstream tests, definition invocation acceptance and private C runtime trace equivalence remain unverified. Verify all ten report binary/source hashes. Conformance source pin is `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`, actual DLL release is `ded7793fe5795d79eda8e7138dce94aa056edf78`; JE0 source audit `0db94e768a845e2583c01d00538c3d16379677bb` is not validation of a newly built DLL.

Remaining JE2: completed parser-result transport versus deferred applications, full local/locale/definition scopes, explicit body invocation and broader static/runtime/capture convergence. This namespace seam does not complete all JE2. The operand view is completed in the next record; keep semantic nested DD and existing M2 gaps visible. JE3+ collections, broader storage migration, optimization, CUDA and GitHub CI remain deferred.

#### JE2 implementation — borrowed operand view preserving provenance (2026-10-04, partial JE2)

`FunctionOperand::as_entity_ref()` inspects nouns and every function POS through the common JEntityRef. `span()` borrows the stored noun operand source span or function identity span, separately from later application occurrences. Inspection copies no Value, increments no Arc, and allocates no entity. Retain the owning enum to preserve noun provenance and function DAG ownership. Do not generalize it to gerund collections or physical array representations.

Apply the view to nameless-modifier by-value lookup classification and semantic binding's function NameRef DAG traversal. Preserve traversal order and POS/lookup policy. Parser-item materialization still needs the owned shared Arc and retains its existing path.

Add an owned 65,536-atom noun pointer/span regression. Extend existing tests for noun snapshots after host destruction, a 48-level shared DAG, and explicit Verb/Adverb/Conjunction definitions to verify common-view payload identity, function/DefinitionCode reference counts and original provenance. No new J syntax is introduced; reuse the existing 82 entity-boundary C fixtures.

**Operand seam gate:** Windows default/portable each: **370 passed / 17 ignored**; fmt/clippy/build pass. Python: **27 passed**. Each j64/AVX2 direct/semantic-reference/parser-capture route: **4,821 cases / 4,817 passed / 4 existing runtime boundaries / 0 failed**; stages: **10,001 checks**; words: **6,618 cases / 0 failed**. Keep 108 capture-graph and 2 static boundaries separate. Verify all ten report binary/source hashes. Source/DLL pins match the namespace gate above. Full upstream tests, definition invocation acceptance and private C runtime trace equivalence remain unverified.

The next record implements common transport for completed parser results distinguished from deferred noun/application structure. Full local/locale/definition scopes and explicit body invocation remain incomplete. This does not complete all JE2.

#### JE2 implementation — common completed parser-result transport (2026-10-04, partial JE2)

`CompletedParseResult { entity: JEntity, span, height, verb_adapter }` moves completed RHS payloads without copying noun Values or function Arcs. Keep occurrence/height and Verb span/target adapters outside immutable FunctionEntity identity. `from_item` accepts literal or grouped-literal nouns; uncomputed calls and ReadName retain the existing Unsupported boundary. It never evaluates expressions or performs name lookup. Add no JEntity Clone.

Runtime rows 0–2 invoke the existing host.apply exactly once and return the completed noun through this boundary. Without a host, analysis preserves Expr computation structure. Row 7 uses the same boundary for host.assign transport and reconstruction from the returned payload. The existing reduction pipeline owns commit capture, provenance inheritance, POS, lookup timing and effect order. Keep the broader ParseValue Expr/Verb/Function/NAME/control variants, constructor paths and final Program structure.

Source evidence is [runtime p.c row 7](https://github.com/jsoftware/jsource/blob/0db94e768a845e2583c01d00538c3d16379677bb/jsrc/p.c#L1006), which assigns/returns the stacked RHS. Keep this separate from pv.c tacit translation; this is not a claim to have built a DLL from the new source pin.

Three new unit regressions verify a grouped owned 65,536-atom noun's pointer/occurrence span/height; all function POS identities, NameRef and Verb occurrence adapters; and static call/name preservation plus one apply followed by inner/outer commits in chained assignment. Add **6 computed scalar-chain/grouped-array assignment and binding-read cases**, bringing entity-boundary fixtures to **88**. The invocation-count test observes the Rust host boundary, not private C trace equivalence.

**Completed-result gate:** Windows default/portable each: **373 passed / 17 ignored**; fmt/clippy/build pass. Python: **27 passed**. Each j64/AVX2 direct/semantic-reference/parser-capture route: **4,827 cases / 4,823 passed / 4 existing runtime boundaries / 0 failed**; stages: **10,007 checks**; words: **6,618 cases / 0 failed**. Keep 108 capture-graph and 2 static boundaries separate. Verify all ten report binary/source hashes. Source/DLL pins match the namespace gate above. Full upstream tests, definition invocation acceptance and private C runtime trace equivalence remain unverified.

Next review remaining parser value/constructor boundaries for duplicated concrete-result versus analysis-expression transport. Full stack-enum convergence, local/locale/definition scopes and explicit body invocation remain incomplete. Optimization, CUDA and GitHub CI remain deferred.

#### JE2 implementation — common modifier-constructor operand boundary (2026-10-04, partial JE2)

`CompletedParseResult::into_operand()` applies the existing one-time into_shared policy to a completed noun and moves it with its original occurrence span into FunctionOperand. Move the same function Arc without copying occurrence-only Verb adapters into semantic children. Replace duplicated Noun/Verb/Adverb/Conjunction transport in modifier_train, shared by production rows 5/6 and gerund AR construction. Result POS remains determined by the existing cf.c disposition/constructor. Raw NAME/control retain syntax errors; deferred calls/ReadName retain Unsupported boundaries.

Keep rank-conjunction right-first audits, noun-left forks, definition constructors and immediate bident/trident application separate because their validation/execution contracts differ. Introduce no generic entity collection or physical storage changes.

Two new regressions verify zero-copy freezing/source-span retention of a grouped owned 65,536-atom noun and its reuse at a later occurrence after train destruction. Check all function POS DAG identities/reference counts, cf.c disposition/result POS, deferred-noun rejection and control syntax errors. Rerun runtime noun-origin capture, named-array snapshot, nested-modifier tests and the existing 88 entity-boundary C fixtures.

**Constructor-operand gate:** Windows default/portable each: **375 passed / 17 ignored**; fmt/clippy/build pass. Python: **27 passed**. Each j64/AVX2 direct/semantic-reference/parser-capture route: **4,827 cases / 4,823 passed / 4 existing runtime boundaries / 0 failed**; stages: **10,007 checks**; words: **6,618 cases / 0 failed**. Keep 108 capture-graph and 2 static boundaries separate. Verify all ten report binary/source hashes. Source/DLL pins match the namespace gate above. Full upstream tests, definition invocation acceptance and private C runtime trace equivalence remain unverified.

Next lock rank/conjunction right-before-left error precedence and gerund audit contracts before deciding how to reuse common transport there. This does not complete JE2 or full J parsing.

#### JE2 implementation — rank/conjunction operands and error precedence (2026-10-04, partial JE2)

Connect apply_conjunction_at Noun/Verb operands through CompletedParseResult::from_item/into_operand. Check the right operand form first; @: noun-right domain errors precede deferred-noun Unsupported. For rank noun-right, audit rank→length→numeric domain before inspecting the left operand or auditing gerunds. Preserve the original Expr-span versus parser reinsertion-override distinction. Common transport owns existing noun freezing and function DAG moves.

Evidence is pinned [cr.c::jtqq](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c#L733): right-rank extraction precedes the noun-left branch. Only boxed rank-1 nouns are audited as gerunds; skip audits when all requested ranks are RMAX. J fx errors quietly fall back to the constant noun with no partial decoded list. Continue propagating RustJ Unsupported boundaries rather than pretending an unimplemented C feature succeeded. Retain verb-right as its original function operand, separately from noun-left function identity.

Two new unit tests verify pointer/Expr spans for an owned 65,536-atom constant and rank noun, right rank/length/domain precedence over a deferred left, and @: noun-right domain precedence. One capture regression checks no gerund lookup/commit and unchanged binding versions after right errors; quiet audits with valid rank/verb-right; infinite-rank audit skipping; and partial decode removal. Add **12 setup, error/retained-binding use, quiet/RMAX/verb-right construction cases** to compound-gerund C corpus/stages. Entity-boundary fixtures remain separately at 88.

**Rank-operand gate:** Windows default/portable each: **378 passed / 17 ignored**; fmt/clippy/build pass. Python: **27 passed**. Each j64/AVX2 direct/semantic-reference/parser-capture route: **4,839 cases / 4,835 passed / 4 existing runtime boundaries / 0 failed**; stages: **10,019 checks**; words: **6,618 cases / 0 failed**. Capture-graph boundaries total **110**, including two added fixtures; static boundaries remain 2. Record these separately from runtime passes. Verify all ten report binary/source hashes. Conformance source/DLL pins match the JE2 namespace gate. Full upstream tests, definition invocation acceptance and private C trace equivalence remain unverified.

Next JE2 candidates: noun-left fork and definition-constructor completed-value boundaries, first checking source spans, constructor errors and no-body-execution contracts. Full stack convergence, scopes and definition invocation remain incomplete.

#### JE2 implementation — noun-left fork and definition construction (2026-10-04, partial JE2)

CompletedParseResult::from_noun moves grouped literals under the existing completed_noun rules, preserving Expr span/height. General from_item shares this boundary and restores the existing Item occurrence override. Noun-left fork moves this result through into_operand, preserving source span and g/h DAG identities. Replace its separate owned-noun path with the common one-time freeze policy so later operand reuse shares large constants rather than copying whole arrays. The allocator/physical representation itself is unchanged.

Supported DefinitionConstructor checks both noun classes first, then extracts completed mode/body in the existing order. These transient inputs are neither frozen nor stored as FunctionEntity operands. Preserve mode/body origin matching and semantic code validation. Return the actual Verb/Adverb/Conjunction entity through common function→into_item transport; remove an unnecessary DefinitionCode Arc clone. Preserve code/source provenance without creating invocation/local frames or executing bodies. Computed-definition coverage is not expanded.

Two unit regressions verify pointer/source span and g/h identities for a grouped owned 65,536-atom fork constant, shared survival through two reuses after fork destruction, deferred-noun rejection, and definition class-guard precedence over deferred inputs with existing domain/Unsupported behavior. One integration regression checks explicit/direct construction and commit for every function POS, unchanged body-counter version/value and no body-name input observations. Add **12 C definition corpus/stage cases** covering the same six constructors and counter reads. Entity-boundary fixtures remain separately at 88.

**Fork/definition gate:** Windows default/portable each: **381 passed / 17 ignored**; fmt/clippy/build pass. Python: **27 passed**. Each j64/AVX2 direct/semantic-reference/parser-capture route: **4,851 cases / 4,847 passed / 4 existing runtime boundaries / 0 failed**; stages: **10,031 checks**; words: **6,618 cases / 0 failed**. Record **114 capture-graph boundaries** and 2 static boundaries separately from runtime passes. Verify all ten report binary/source hashes. Conformance source/DLL pins match the JE2 namespace gate. Full upstream tests, definition invocation acceptance and private C trace equivalence remain unverified.

Next audit remaining completed-result/function-wrapper boundaries, remove only unnecessary adapters, and reconcile frontend F/P checklists with supported versus unimplemented construction/execution. Full stack convergence, local/locale scopes and definition invocation remain incomplete. CUDA, optimizer implementation and GitHub CI remain deferred.

#### JE2 implementation — remaining adapter audit and frontend checklist reconciliation (2026-10-04, partial JE2)

ParseValue::function_entity() borrows completed Verb/Adverb/Conjunction Arcs. Remove duplicated construction-success/final-result capture branches; clone an Arc only when the event must retain function lifetime, as before. Route completed Verb moves through the existing function factory while preserving both Item occurrence and Verb adapter spans. Add no J syntax or execution coverage.

| Retained structure | Reason |
|---|---|
| Verb/VerbTarget | Parser occurrence span and runtime target adapter differ from intrinsic FunctionEntity identity |
| ParseValue/Item | Deferred Expr, lexical NAME/target/control, class/flags/word provenance/occurrence are not concrete JEntity |
| ParserNameBinding | Lookup observations distinguish noun snapshots, abstract nouns, function POS and known modifier/version |
| FunctionOperand | Own noun source spans and function DAG Arcs; inspect through borrowed JEntity views |
| ExprKind | Final Program preserves static computation/dependencies versus completed values/functions |
| CompletedParseResult | Concrete JEntity moves retain only height/span/Verb occurrence adapters |

AssignedValue/SymbolValue were removed earlier. Do not erase these differences just to reduce enum counts; full stack/scope/invocation convergence remains incomplete.

Reconcile canonical frontend checklist items: **F2** same-stack reinsertion; **P2** rescanning with the same matcher and shared runtime/analysis engine; **P4** ordinary extension NAME and assignment-target separation; **P7** removed flat application loop and shared entry points. These implementation boundaries are checked. **P2 rows 3/4/7 remain partial**: supported adverb/gerund, rank/@:/definition construction, and top-level single-name/chained assignment do not complete all primitive/modifier/target/scope semantics. Full runtime ptcol traces, complete modifier/immediate bident/trident actions, scope/invocation, intrinsic FunctionSemanticInfo and final cutover gates remain unchecked.

Extend the existing explicit/direct all-POS regression to verify the same FunctionEntity/DefinitionCode Arc across ConstructionSuccess→FunctionResult→Commit and final-result observation before final commit. Revalidate the existing 4,851 C cases.

**Adapter-audit gate:** Windows default/portable each: **381 passed / 17 ignored**; fmt/clippy/build pass. Python: **27 passed**. Each j64/AVX2 direct/semantic-reference/parser-capture route: **4,851 cases / 4,847 passed / 4 existing runtime boundaries / 0 failed**; stages: **10,031 checks**; words: **6,618 cases / 0 failed**. Keep 114 capture-graph and 2 static boundaries separate. Verify all ten report binary/source hashes. Conformance source/DLL pins match the JE2 namespace gate. Full upstream tests, definition invocation acceptance and private C trace equivalence remain unverified.

The following audit corrects this planned priority: supported immediate actions already existed; distinguish them from surface parser row reachability and missing primitive/definition executors.

#### JE2/P3 implementation — immediate constructor results and row reachability correction (2026-10-04, partial)

C cf.c::jthook immediately applies fn==0 V N / N/V A and N V N / N/V C N/V combinations created by invisible modifier execution. RustJ already supports these through construct_modifier_bident/trident in AR decoding and derived modifier execution. Noun calls invoke the runtime host once; modifier actions preserve actual returned POS. Correct the claim that all immediate execution was missing. Complete primitive/definition execution and P3 remain incomplete.

Surface rows 0/2/3/4 consume immediate combinations before rows 5/6. Exhaustively check all **6,561 four-class windows**: Fork selects only NVV/VVV; Hook cannot select immediate/fork dispositions. Replace unreachable Unsupported messages with row-invariant errors. This does not turn static value-dependent Unsupported boundaries into J errors or execute static calls.

Route ConstructionNames::apply_noun success through CompletedParseResult::noun(...).into_item(), preserving one-time freeze, span/height and success/failure observation order. A unit regression checks both bident/trident host calls with owned 256×256 results: exactly one call, actual Noun POS, pointer/shape/span preserved and no copy when retained as a FunctionOperand. Revalidate existing failure/effect/static-no-host coverage. Add **12 C corpus/stage cases** feeding immediate scalar/array noun results into rank construction and comparing domain/length errors.

- [x] Separate immediate actions from surface row eligibility and exhaustively check reachability.
- [x] Move host noun results through the common completed-result carrier.
- [x] Compare nested immediate-to-rank cases against C.
- [ ] Complete primitive/explicit modifier invocation and local/locale/definition scope. Keep 17 ignored definition acceptance tests outside completion evidence.

**Immediate-boundary gate:** Windows default/portable each: **383 passed / 17 ignored**; fmt/clippy/build pass. Python: **27 passed**. Each j64/AVX2 direct/semantic-reference/parser-capture route: **4,863 cases / 4,859 passed / 4 existing runtime boundaries / 0 failed**; stages: **10,043 checks**; words: **6,618 cases / 0 failed**. Keep 114 capture-graph and 2 static boundaries separate. All ten report binary/source hashes verified. Source/DLL pins match the JE2 namespace gate. Full upstream tests, definition invocation acceptance and private C trace equivalence remain unverified.

Next isolate genuinely unsupported explicit adverb/conjunction application with minimal C cases, then implement an invocation boundary preserving operand/local bindings and actual result POS, including noun body results. CUDA, optimizer implementation and GitHub CI remain deferred.

Sources: [cf.c tables](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L292), [immediate invisible-modifier execution](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L355), [p.c ordered parser rows](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c).

#### JE2/P3 implementation — first nonoperator explicit modifier invocation (2026-10-04, partial)

Rows 3/4 and AR/derived modifier execution apply ExplicitDefinition through RuntimeParserHost::apply_definition, returning actual JEntity. Support mode 1/2 without x/y references where the selected valence has one Body sentence. Execute literals and computations such as u/, m+n and global noun reads through the shared frontend and existing semantic kernels. Direct mode 1/2 definitions use the same DefinitionCode boundary. Constructing/assigning a definition still does not execute its body.

ModifierFrame borrows the parent Engine for global lookup/calls; it neither copies the namespace nor temporarily writes operands into globals. Bind u/v and noun-only m/n aliases. operand_function substitutes concrete functions only for these special names, following p.c mnuvxy by-value semantics and cx.c operand installation. Ordinary function names retain late lookup/alias redefinition. Compare the gerund special-name u case with C decoded structure; do not snapshot ordinary gerund names.

CompletedParseResult reinserts actual Noun/Verb/Adverb/Conjunction POS, retaining shared noun payload/function Arcs. Remove the capture assumption that every construction returns Function. Static prepare keeps runtime-required invocation Unsupported; explicit definitions are not static-known primitive modifiers.

Add ExplicitModifierApply markers and ConstructionNounSuccess occurrences/facts, verifying matching attempts, row/POS, sequential IDs and failure/assignment preservation. Body dependency/effect graphs are not yet connected to the outer graph: J Graph conversion reports an explicit invocation-scope boundary rather than claiming analysis completion. Preserve operation/argument diagnostic context but locate body failures at the outer invocation, avoiding body offsets interpreted as caller offsets. Separate body/caller diagnostic frames remain pending.

- [x] Execute single-sentence nonoperator adverbs/conjunctions, reinserting actual noun/function POS.
- [x] Verify u/v, noun-only m/n, global late lookup, escaped function reuse, errors/redefinition and preserved assignment targets.
- [x] Add three Rust regressions and **48 C corpus/stage cases** covering scalar/matrix/empty/boxed nouns, returned ADV/CONJ applications, gerund operands and domain/length failures.
- [x] Simple NAME assignment bodies and multiple straight-line sentences are implemented in the following modifier-scope stage. Control flow and nested definition scopes remain Unsupported.
- [x] The later operator-call step implements deferred x/y Verb construction and straight-line invocation. It does not add global fallback for unbound special names or complete scope support.
- [ ] Recursive shared-parser invocation currently has an **8-level Windows stack bound**, returning LimitError. Verify depth restoration after failure; a general explicit-frame/trampoline executor and wider depth remain pending.
- [ ] Connect body graph/source frames and complete local/locale/definition invocation. Keep 17 ignored definition acceptance tests incomplete.

**Explicit-modifier gate:** Windows default/portable each: **386 passed / 17 ignored**; fmt/clippy/build pass. Python: **27 passed**. Each j64/AVX2 direct/semantic-reference/parser-capture route: **4,911 cases / 4,907 passed / 4 existing runtime boundaries / 0 failed**; stages: **10,091 checks**; words: **6,618 cases / 0 failed**. Keep **147 capture-graph boundaries** (19 invocation, 109 modifier-value, 19 ordered-effect) and 2 static boundaries separate. All ten report binary/source hashes verified. Reviewed source is 13994ffa1ed5f06f79fad6e9822a7ed2d29b1528; executed DLL release is ded7793fe5795d79eda8e7138dce94aa056edf78. Full upstream tests, definition invocation acceptance and private C trace equivalence remain unverified. Linux/GitHub CI/CUDA checks were not run.

The following modifier-scope stage implements local/global assignment dispatch and final-result/failure/effect ordering across straight-line sentences. The operator-call step below supports straight-line x/y calls; body graph integration and full callable scope remain pending. CUDA, optimizer implementation and GitHub CI remain deferred.

Sources: [cx.c invocation/local frame](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L259), [u/v and noun m/n installation](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L322), [p.c special-name resolution](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c#L616), [VXOPR executor selection](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L1316).

#### JE2/P3 implementation — modifier local/global assignment and straight-line bodies (2026-10-04, partial)

This stage extends the preceding single-sentence boundary. RuntimeParserHost supplies the enqueue environment and scoped assignment. Definition bodies use ExplicitDefinition rules, preserving local `=.` instead of promoting it under TopLevel rules. Engine keeps a per-call LocalFrame separate from globals; reads search **the current frame, then globals**, never caller frames. Install u/v and noun-only m/n there. Freeze nouns once before sharing; retain function Arcs and array payloads without copying the namespace.

Execute multiple Body sentences in the selected nonoperator mode 1/2 valence in order, returning the final actual JEntity/POS even when the last sentence assigns. A nonassignment function result followed by another sentence raises C's `noun result was required`; function assignments may continue. Restore local frames and depth on success/error. Earlier committed global effects survive later failure; failed RHS and outer assignment targets do not commit.

C comparisons distinguish these name rules:

- Global `=:` to a currently **bound private name** raises domain error. A declared but uninitialized local name permits global assignment; a later local assignment may then shadow it.
- Ordinary function NAMEs retain execution-time lookup inside bodies and after return. Do not recursively freeze them to the final local function value. After frame exit an absent global gives value error; a subsequently bound global supplies the value. Distinguish this from special u/v by-value substitution and the actual RHS returned by a final function assignment.
- C's exit-time fixing of implicit locatives u./v. is a separate mechanism, pending separately from the straight-line x/y executor implemented below.

Checklist:

- [x] Separate simple NAME local/global assignment and current-frame-to-global lookup in nonoperator modifiers.
- [x] Verify straight-line bodies, final assignment result/POS, intermediate nonnoun errors, committed global effects and frame restoration after failure.
- [x] Compare direct definitions and real `1/2 : 0` block inputs. Supply blocks through C `0!:100` script delivery and pass the same source to Rust; do not simulate interactive blocks with a single JDo call.
- [x] Add **six Rust regressions**, **31 common C corpus cases** and **41 stage-only cases**, covering local nouns/functions/adverbs, ordinary NAME escape/rebinding, operands, failure/effect ordering and shared payload identity for a 65,536-atom array after frame exit.
- [x] The ordinary-reference step below resolves cross-frame operands, publication with local NAMEs, uninitialized local assignments and operand/local collisions. Actual implicit locatives remain a separate unsupported boundary.
- [ ] Control flow, nested definition framing, locales and full operator wrappers/scope remain pending. Straight-line x/y calls are implemented below. Retain the 8-level bound and 17 ignored definition acceptance tests.
- [ ] Connect body dependency/effect graphs and separate body/caller diagnostic frames. Graph lowering still reports `explicit modifier body graph requires invocation scope`; execution success does not imply static graph completion.

**Modifier-scope gate:** native Windows default/portable each: **392 passed / 17 ignored**; fmt/clippy/build pass. Python: **27 passed**. Each j64/AVX2 direct/semantic-reference/parser-capture route: **4,942 cases / 4,938 passed / 4 existing runtime boundaries / 0 failed**; stages: **10,163 checks / 0 failed**; words: **6,618 cases / 0 failed**. Keep **164 capture-graph boundaries** (28 invocation, 117 modifier-value, 19 ordered-effect) and 2 static boundaries separate. All ten report binary/source hashes verified. Reviewed source: 13994ffa1ed5f06f79fad6e9822a7ed2d29b1528; executed DLL release: ded7793fe5795d79eda8e7138dce94aa056edf78. Full upstream tests, ignored definition acceptance and private C trace equivalence remain unverified. Linux/GitHub CI/CUDA checks were not run.

The following ordinary-reference step audits these cases against C and resolves ordinary NAME restrictions; straight-line x/y calls are implemented below while body graph/source frames and control flow remain pending. Global effects may commit before a later Unsupported boundary, so it is not a safe automatic replay signal. Optimizer implementation, CUDA and GitHub CI remain deferred.

Sources: [p.c local/global lookup](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c#L631), [s.c bound-private global assignment guard](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/s.c#L718), [cx.c intermediate noun-result requirement](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L67), [cx.c implicit-locative fixing](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L679), [af.c implicit u/v handling](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/af.c#L53), [jerr.h EVNONNOUN](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/jerr.h), [i.c error message](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/i.c).

#### JE2/P3 implementation — ordinary NAME scope boundaries resolved (2026-10-04, partial)

- [x] Check ordinary NAME cross-frame operands, global publication, uninitialized local assignments and operand/local collisions against the native C oracle. Resolve the earlier conservative boundaries for this scope.
- [x] Remove four restrictions confusing ordinary NameRef with implicit locatives and the recursive name-membership audits. Preserve late references and expected POS. Calls read the current frame then globals without capturing/searching caller frames. Keep u/v operand substitution and noun snapshots.
- [x] After smf=.u, passing smf to an inner modifier does not freeze the caller's local smf; execution reads the inner local/global binding. Publishing smexport=:smf/ preserves the ordinary name and reflects global rebinding after exit. Uninitialized smf=.smf follows RHS noun/function POS: noun snapshot or function NameRef. Operand/local name collisions are valid too.
- [x] Replace one Unsupported scope golden with three behavioral Rust regressions. Verify rebinding, POS mismatch, undefined-to-defined references, failed outer target/version preservation, committed global publication and frame recovery. Add **21** common single-line C cases and **45** stage cases including them. Compare returned C atomic structures and results/errors. Include af.c in source hashes.
- [ ] Actual implicit u./v. locatives, full operator wrappers/scope, control flow/nested scope and body graph/source diagnostic frames remain pending. Straight-line x/y calls are implemented below. Ordinary NAME support does not complete implicit-locative fixing or closures. Static/no-host modifier application remains an explicit boundary; execution success does not complete static graph analysis.

**Ordinary-reference gate:** native Windows default/portable each **398 passed / 17 ignored**; fmt/clippy/build pass. Python **27 passed**. Each j64/AVX2 runtime route: **4,963 cases / 4,959 passed / 4 existing runtime boundaries / 0 failed**; stages **10,208 checks / 0 failed**; words **6,618 cases / 0 failed**. Keep **171 capture-graph** and **2 static** boundaries separate. No new runtime waiver. All ten report binary/source hashes verified. Reviewed source 13994ffa1ed5f06f79fad6e9822a7ed2d29b1528 differs from executed DLL release ded7793fe5795d79eda8e7138dce94aa056edf78. Full upstream/ignored definition acceptance/private C trace equivalence remain unverified; optimizer/CUDA/GitHub CI stay deferred.

The following operator-call step implements deferred straight-line calls; implicit locatives and nonexecuting body graphs/source frames remain pending. Do not automatically replay after Unsupported because effects may already have committed.

Sources: [p.c ordinary lookup / mnuvxy](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c#L616), [cx.c return-time implicit-locative fix](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L679), [af.c hasimploc / fix scope](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/af.c#L17).

#### JE2/P3 implementation — deferred x/y operator verbs and straight-line calls (2026-10-04, partial)

- [x] Like cx.c::jtxop2/VXOPR, applying mode 1/2 operator operands constructs a **Verb without executing its body**. Retain FunctionHead::ExplicitDefinition/shared DefinitionCode and ordered source operands. Distinguish code modifier POS from resulting Verb POS; introduce no modifier-specific AST or body noun reduction.
- [x] Direct/ordinary NAME calls install separate x/y and reuse the straight-line body executor/tokenizer/enqueuer/parser. Select monad/dyad from actual arguments and code sections, not modifier operand count. Empty sections raise ValenceError; final function results raise EVNONNOUN. Construction does not execute control bodies; their calls remain Unsupported.
- [x] Install/clean per-call local frames, u/v and noun-only m/n. Preserve current-frame/global lookup, late ordinary references, committed global effects and failed outer assignments. Rebinding the definition does not change captured code; rebinding an ordinary function operand name affects later calls. Primitive calls acquire no new function Arc copies.
- [x] Freeze noun operands into shared storage during deferred construction and share them across calls. Three Rust regressions verify a 65,536-atom pointer/lifetime after rebinding, code Arc identity, construction/call effect counts, valence/POS/noun-result errors and repeated-failure frame recovery. Keep the existing Windows eight-level recursion bound.
- [x] Add **38** common C cases and **67** stage cases including them: scalars/vectors/empty, noun snapshots, named operand rebinding, direct/block/two-valence definitions and effects on failure. Frontend probe/C 5!:1 projection preserve boxed operator heads and operand vectors, distinguishing bare modifiers from applied verbs. Keep previous nonoperator scope/capture tests.
- [ ] u./v. implicit-locative fixing, control flow/nested scope, full bare mode 3/4 verb invocation, operators under rank/insert wrappers, body graph/source diagnostic frames and Logical lowering remain pending. Static/no-host application stays explicit. Preserved code/operands do not complete body analysis or compiled reuse.

**Operator-call gate:** native Windows default/portable each **401 passed / 17 ignored**; fmt/clippy/build pass. Python **27 passed**. Each j64/AVX2 runtime route: **5,001 cases / 4,997 passed / 4 existing runtime boundaries / 0 failed**; stages **10,275 checks / 0 failed**; words **6,618 cases / 0 failed**. Keep **184 capture-graph** and **2 static** boundaries separate. No new runtime waiver. Ten report binary/reference/source hashes verified. Source pin 13994ffa1ed5f06f79fad6e9822a7ed2d29b1528 differs from DLL release ded7793fe5795d79eda8e7138dce94aa056edf78. Full upstream/ignored acceptance/private C trace equivalence remain unverified; Linux/GitHub CI/CUDA were not run.

**Separate oracle boundary:** Infinite operator recursion caused the j64 oracle's ctypes JDo to terminate with OSError: exception: stack overflow. This was not a normal J LimitError comparison; exclude it from the successful corpus and record reports/operator-recursion-oracle-boundary-windows.json. Only Rust depth-limit/frame-recovery regression passed; do not claim C equivalence for this case or count the failed harness run as a successful gate.

The implicit-operand step below implements return-time fixing. Direct calls with caller-scope switching, body graphs/source frames and control execution remain pending. Keep optimizer/CUDA/GitHub CI deferred.

Sources: [cx.c jtxop2](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L749), [cx.c operand extraction](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L259), [cx.c argument installation](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L269), [cx.c result audit](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L671), [cx.c executor selection](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L1316).

#### JE2/P3 implementation — return-time implicit operand fixing (2026-10-04, partial)

- [x] Enqueue `u.`/`v.` as VERB primitives, following C `t.c`; retain the distinction from ordinary NAME and extension lookup. Registry version is 4.
- [x] Count them as u/v when inferring direct-definition mode, following `cx.c::xop`. Lexical VERB class is distinct from the definition's adverb/conjunction POS.
- [x] On a straight-line modifier function return, replace the first implicit locative per branch with the departing frame's final u/v binding. Do not recurse into replacements or fix ordinary NameRefs. Preserve source operators, operand order and decoded gerunds; unchanged entities retain their shared Arc.
- [x] A noun returned through a verb locative produces DomainError. Returning an uninstalled operand can leave a C function reference and remains Unsupported; do not reinterpret it as ValueError. Do not fix previously committed global publications. Cover ordinary-name rebinding and frame recovery after failed construction.
- [x] Unresolved implicit primitives have unknown contracts/effects and DynamicOrUnknown graph rules; they do not provide pure-kernel or shape-preservation facts.
- [ ] **Follow-up:** the caller-scope step below implements direct `u./v.` and ordinary-alias calls, plus out-of-frame publication call errors. Raw calls inside wrappers, uninstalled operand return references, complete locale/control/body graphs and source frames remain pending. Ordinary lookup would misinterpret caller-local names.

Native Windows C probes confirmed returned `u.`/`u./`/`v.`, operand-local reassignment, ordinary-name rebinding and raw global publication. **Three Rust regression tests** cover lexical/definition mode, returned calls/errors/frame recovery, shared fork operand Arcs, preserved rank DAGs and unknown graph rules. Add **29 common runtime cases** and **45 stage cases including those**. Raw calls and uninstalled operand returns are not counted in the new successful runtime corpus.

C `5!:1` projects execution objects rather than source graphs: fixing u to + in `u. "0` allows C to elide redundant rank. Only the comparison probe elides exactly `[0,0,0]` rank around +; Semantic IR preserves the source rank parent. Compare both zero-rank + and ravel with retained rank, and verify rank-parent preservation in Rust capture. Decoded gerunds remain constructor auxiliaries rather than recursively fixed source edges. Operator-specific gerund AR reconstruction and full constructor specialization remain pending. [t.c + ranks](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/t.c#L113), [cr.c rank reconstruction](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c#L778).

**Implicit-return gate:** native Windows default/portable each **404 passed / 17 ignored**; fmt/clippy/build pass. Python **27 passed**. Each j64/AVX2 direct/semantic-reference/parser-capture route: **5,030 cases / 5,026 passed / 4 existing runtime boundaries / 0 failed**; stages **10,320 checks / 0 failed**; words **6,618 cases / 0 failed**. Keep **195 capture-graph** and **2 static** boundaries separate. No new runtime waiver. Ten actual binary/reference/source report hashes verified; add `t.c` to reviewed source hashes. Source pin `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528` differs from DLL release `ded7793fe5795d79eda8e7138dce94aa056edf78`. Full upstream/ignored acceptance/private C trace equivalence remain unverified. Optimizer/CUDA/Linux/GitHub CI were not run.

Sources: [t.c registration](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/t.c#L221), [cx.c mode inference](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L766), [cx.c return fix](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L679), [af.c first implicit reference](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/af.c#L117), [sc.c caller scope switch](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sc.c#L124).

#### JE2/P3 implementation — implicit operand calls in caller scope (2026-10-04, partial)

- [x] Confirm ordinary u versus implicit u. caller-local lookup, v., monad/dyad and noun/missing operand errors using `sc.c::unquote` and native Windows C probes.
- [x] For direct primitives and ordinary aliases, obtain the current frame's operand, execute in the caller environment and restore the current frame on success and every J error. Move noun arguments without copying and share the existing parser/runtime executor.
- [x] Add Rust and both C-variant comparisons for caller-local collisions, operand reassignment, global publication, frame restoration and committed global effects after errors. Keep graph contracts unknown.
- [ ] Raw implicit execution inside insert/rank/trains, uninstalled operand return references, complete locale/control/body graphs and source frames remain separate boundaries. This step does not implement full global locale-path switching.

**Two Rust regressions** cover ordinary u versus u. caller-local lookup, u/v monad/dyad, noun/missing operand errors, committed global effects and frame recovery after nested operator failure. Retain the existing implicit-return regression. Add **23 common runtime cases** and **47 stage cases including those**. **Caller-scope gate:** native Windows default/portable each **406 passed / 17 ignored**; fmt/clippy/build pass. Python **27 passed**. Each j64/AVX2 runtime route: **5,053 cases / 5,049 passed / 4 existing runtime boundaries / 0 failed**; stages **10,367 checks / 0 failed**; words **6,618 cases / 0 failed**. Keep **204 capture-graph** and **2 static** boundaries separate. No new runtime waiver. Ten actual binary/reference/source report hashes verified. Source pin `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528` differs from DLL release `ded7793fe5795d79eda8e7138dce94aa056edf78`. Full upstream/ignored acceptance/private C trace equivalence remain unverified. Optimizer/CUDA/Linux/GitHub CI were not run. **Calling** an uninstalled operand produces ValueError; **returning** its reference remains a separate boundary. Keep the existing Windows recursion bound, counting suspended callees toward invocation depth.

Sources: [sc.c local operand/caller switch](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sc.c#L122), [sc.c implicit primitive call](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sc.c#L433).

#### JE2/P3 implementation — scope-aware wrapper and train execution (2026-10-04, partial)

- [x] Review `ar.c::jtredg` right association, `j.h::FORK1/FORK2` right-branch-first order, rank cell/frame and prefix agreement.
- [x] Share A3 rank/reduction cell algorithms through callback seams. Preserve primitive kernel paths; route only unsupported compositions to runtime FunctionEntity execution. Do not flatten operator/rank DAGs.
- [x] For nonempty monadic insert, uniform nonempty rank and Hook/Fork/Atop monads/dyads, resolve names and implicit caller scope at each child invocation. Share retained fork inputs and stop before the left branch when the right branch errors.
- [x] Add Rust/both-C coverage for wrappers, caller-local collisions and branch effect/error order. Retain static unknown contracts and effect barriers.
- [ ] Audit empty identities/prototypes, heterogeneous fill/padding, sparse/dyadic insert and noun-left/capped trains separately. Unsupported callbacks do not authorize replay after committed effects.

**Wrapper/train gate:** Windows default/portable each **408 passed / 17 ignored**; fmt/clippy/build pass. Native Python **27 passed**. Each j64/AVX2 runtime route: **5,089 cases / 5,088 passed / 1 runtime boundary / 0 failed**; stages **10,424 checks / 0 failed**; words **6,618 cases / 0 failed**. Keep **222 capture-graph** and **2 static** boundaries separate. Resolve three existing runtime boundaries (Atop and two named-insert fork cases) and remove their waivers; add none. Ten binary/reference/source report hashes verified; add `ar.c` to reviewed sources. Distinguish source pin `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528` from DLL release `ded7793fe5795d79eda8e7138dce94aa056edf78`. Full upstream/ignored acceptance/private C trace equivalence remain unverified; optimizer/CUDA/Linux/GitHub CI were not run.

Two Rust regressions and the existing named-insert provenance regression cover source DAGs, right association, caller locals, rank shape and branch effect/error order. Add **36 common runtime cases** and **57 stage cases including those**. Add no function Arc copying to primitive calls; only the runtime fallback traverses compositions. Do not claim A3 Hook/Fork execution or full body graph lowering. Next compare empty reduction identities and empty rank prototypes with C.

Sources: [ar.c reduce](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ar.c#L513), [j.h fork execution](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/j.h#L1249), [cr.c rank](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c).

#### JE2/P3 implementation — empty identities and pure ravel prototypes (2026-10-04, partial)

- [x] Verify `+ - * %` identities with `ai.c::jtiden` and native Windows C. A user verb can execute once for empty-rank prototype inference and commit global effects.
- [x] Resolve primitive witnesses through current name/POS and implicit caller scope without executing or claiming analysis of definition bodies. Compute the four witnessed identities through existing kernels and restore caller frames.
- [x] Infer pure monadic ravel empty-rank output shape/type from logical cell shape, sharing the kernel between primitives and implicit wrappers. Cover negative ranks, multidimensional zero axes, character type and caller-local collisions.
- [x] Add Rust/both-C regressions; retain separate unknown-user-verb prototype and identity boundaries. General prototype effects/suppressed errors/fill, other primitives, sparse and dyadic insert remain pending.

**Empty-scope gate:** native Windows default/portable each **410 passed / 17 ignored**; fmt/clippy/build pass. Python **27 passed**. Each j64/AVX2 runtime route: **5,130 cases / 5,129 passed / 1 runtime boundary / 0 failed**; stages **10,474 checks / 0 failed**; words **6,618 cases / 0 failed**. Keep **230 capture-graph** and **2 static** boundaries separate. No new waiver. Ten binary/reference/source report hashes verified; add `ai.c` to reviewed sources. Add **two Rust regressions**, **41 common runtime cases** and **50 stage cases including those**.

An explicit empty-rank C body executed once and changed count to 1; an unknown explicit reduction identity produced DomainError with count still 0. Record these separate j64 probes in `reports/empty-prototype-oracle-windows.json`. Rust remains explicitly Unsupported; do not claim equivalence/purity for these cases. Runtime primitive witnesses do not imply compile-time binding proofs or change analyzer unknown contracts. Full upstream/ignored acceptance/private C trace remain unverified; optimizer/CUDA/Linux/GitHub CI were not run. Next implement noun-left forks.

Sources: [ai.c identities](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ai.c#L368), [ar.c empty reduction](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ar.c#L505), [cr.c rank execution](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c).

#### JE2/P3 implementation — noun-left fork calls and shared snapshots (2026-10-04, partial)

- [x] Verify the `j.h` NVV route and native Windows C: the left noun is a constructor snapshot; execute h before passing that value to g. Compare monads/dyads, rebinding and agreement errors.
- [x] Reuse the source Fork DAG and CompletedParseResult's shared noun. Invoke only the right child; do not turn the noun into a late name lookup or share inputs unnecessarily for two branches.
- [x] Check 65,536-atom snapshot pointer/lifetime after rebinding, implicit caller scope, right-side effects committed before join failure and frame recovery.
- [x] Run both C variants and existing frontend/portable gates. Capped forks, specialized noun-left GraphForm/Logical lowering and general prototype/control/body graphs remain separate work.

Sources: [cf.c noun fork](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L59), [j.h NVV execution](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/j.h#L1277).


**Noun-fork gate:** native Windows default/portable each **412 passed / 17 ignored**; fmt/clippy/build pass. Python **27 passed**. Each j64/AVX2 runtime route: **5,156 cases / 5,155 passed / 1 runtime boundary / 0 failed**; stages **10,511 checks / 0 failed**; words **6,618 cases / 0 failed**. Keep **234 capture-graph** and **2 static** boundaries separate. Ten actual binary/reference/source report hashes verified; no new waiver. Add **two Rust regressions**, update the existing computed-noun capture regression to verify execution, and add **26 common runtime cases** and **37 stage cases including those**. Full upstream/ignored acceptance/private C trace equivalence remain unverified; optimizer/CUDA/Linux/GitHub CI were not run.

#### JE2/P3 implementation — capped-fork construction and graph representation (2026-10-05, partial)

- [x] Reviewed C `t.c::CCAP`, `cf.c::jtcap/jtfolk` and `j.h`, and recorded finite Windows j64/AVX2 probes. Keep `reports/capped-fork-oracle-windows.json` as the historical **pre-implementation reference-only observation**, not a conformance pass report.
- [x] Register `[:` as a core VERB through the ordinary tokenizer/enqueue path (registry **5**). Standalone monad/dyad raise ValenceError regardless of argument type or emptiness. Its conservative unknown analysis contract does not claim a callable pure array kernel.
- [x] At fork construction inspect direct `[:` or the **single name's current direct binding**; do not chase an alias chain. Later first-name value/POS changes leave capped meaning intact, while ordinary-fork namerefs remain late. Explicit operand substitution, caller-local construction and gerund AR decoding use the same constructor seam.
- [x] Preserve `FunctionHead::Fork`, all three original operands and NAME/source provenance. `FunctionEntity.fork_semantics`, present only for Fork, is immutable constructor meaning (Ordinary/Capped), not an argument-dependent fact or optimizer proof. Keep first-name constructor read/version in `Program/Plan.fork_name_reads` and capture `ForkNameResolved` sidecars, never as executable cache guards. A no-host POS-only first name remains an Unsupported construction boundary without direct-binding evidence.
- [x] Execute h(x,y), then monadic g; never call the first operand. Retain the source Fork and derive Pipeline regions with h→g dataflow in Graph IR **0.5**. Do not assign ParallelBranchCandidate or RetainedValueCandidate. Preserve g/h late-name/POS and effect/error barriers; no optimizer execution is introduced.
- [x] Add regressions for direct/named cap, alias chains, rebinding/POS changes, monad/dyad, operand/local scope, committed effects/recovery, source DAG/dependencies/pipeline hints and gerund carrier provenance. Normalize C AR's first operand to `[:` only in the oracle projection, preserving the source NAME.

**Cap gate:** native Windows default/portable each **418 passed / 17 ignored**; fmt/clippy/build pass. Native Python **27 passed**. Each j64/AVX2 direct, semantic-reference and parser-capture route: **5,207 cases / 5,206 passed / 1 existing runtime boundary / 0 failed**; stages **10,586 checks / 0 failed**; words **6,618 cases / 0 failed**. Record **239 capture-graph** and **2 existing static** boundaries separately. No new runtime waiver. Add **five Rust integration regressions**, **one POS-only constructor-proof unit regression**, **51 common runtime cases** and **75 stage cases including those**. Verify actual binary/reference/source hashes in all ten frontend reports. Distinguish source pin `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528` from DLL release `ded7793fe5795d79eda8e7138dce94aa056edf78`. Full upstream/ignored definition acceptance/private C trace equivalence remain unverified; optimizer/CUDA/Linux/GitHub CI were not run.

**Next checklist:**

- [x] Audit the remaining verb-valued rank operand against C `cr.c` innate-rank/construction rules. The verb-valued rank stage below separates fixed constructor rank from dynamic operand binding.
- [ ] Noun-left GraphForm/Logical specialization, general empty prototypes with effects/error suppression, heterogeneous fill/padding, sparse/dyadic insert and full definition/control/body graphs remain incomplete. Keep the construction boundary for insufficient POS-only first-name evidence in external static catalogs.

Sources: [t.c cap primitive](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/t.c#L163), [cf.c single-name cap check](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L38), [j.h capped calls](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/j.h#L1249).

#### JE2/P3 implementation — separate verb-valued rank construction facts from executable names (2026-10-05, partial)

- [x] Review C `cr.c::jtqq`, `sc.c::jtnamerefacv`, `ja.h` accessors and core primitive/derived constructors. `u"v` copies the right function object's **monad/left/right header ranks without executing it**. Negative requested ranks differ from actual derived-function headers: `+"_1` requests -1 but has monadic header `_`; gerund rank-derived headers are all `_`.
- [x] Copy the current binding's header into immutable `FunctionEntity.name_ranks` when an ordinary NAME enters the parser stack. Read an alias's existing header without following its current target. An undefined ordinary name has all-`_` C headers. This metadata neither freezes executable lookup nor proves purity. Distinguish implicit `u.` headers from explicit actual operand `u` headers.
- [x] Preserve the original rank conjunction, both source operands and NAME/span. `requested_ranks()` reads the source noun spec or RHS verb header without runtime execution/name lookup. Preserve late LHS binding/POS checks, nested rank boundaries and existing prefix agreement/error order. Subsequent RHS NAME rebinding, including noun/adverb POS changes, does not alter a constructed rank.
- [x] Record constructor header read/version/span separately in `Program/Plan.name_rank_snapshots` and capture `FunctionNameRank` observations. Exclude the RHS rank operand from executable late-reference lists. These sidecars are not cache guards or purity proofs. At that stage primitive registry was **6**; Graph IR **0.6** added `GraphForm::Rank.requested_ranks`, retaining the distinction between source noun `rank_spec` and source RHS functions.
- [x] Static catalog `declare_primitive_verb` supplies header evidence. A POS-only RHS NAME remains valid J syntax with an **Unsupported construction-proof boundary**. Known headers do not freeze executable bindings. A regression analyzes ravel-cell output shape from `[1_000_000_000_000, 3]` input metadata without allocating input payloads.
- [x] Seven Rust regressions cover RHS nonexecution, aliases/undefined names/rebinding, late LHS execution, explicit/implicit operands, negative/asymmetric ranks, empty pure ravel, prefix errors/recovery, source/capture/Graph/A3 and large metadata analysis. Compare C `b.0` header projections separately from runtime values/errors. `reports/verb-rank-oracle-windows.json` records preimplementation **C reference-only observations**, separately from conformance reports.

**Verb-rank gate:** native Windows default/portable each **425 passed / 17 ignored**; fmt/clippy/build pass. Native Python **27 passed**. Each j64/AVX2 direct, semantic-reference and parser-capture route: **5,321 cases / 5,321 passed / 0 runtime boundaries / 0 failed**; stages **10,757 checks / 0 failed**; words **6,618 cases / 0 failed**. Keep **244 capture-graph** and **2 static** boundaries separate. Compare **39 primitive/derived headers** and the alias header with C `b.0`. Verify actual binary/reference/source hashes in all ten frontend reports, including `ja.h`. Distinguish source pin `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528` from DLL release `ded7793fe5795d79eda8e7138dce94aa056edf78`. Zero runtime boundaries in the current corpus does not mean full J support. Full upstream/ignored definition acceptance/private C trace equivalence remain unverified; optimizer/CUDA/Linux/GitHub CI were not run.

**Return-boundary follow-up:** `cx.c` fixes the first implicit locative when an explicit modifier returns a non-noun; `af.c::jtfixa` reruns modifiers with substituted operands to construct new derived entities. In-body `(,"u.) y` uses `u.` header `_`; returning `,"u.` and fixing `u=+` produces a new entity using RHS header 0. Ravel output shapes for `[2,3]` input are `[6]` and `[2,3,1]` respectively. Reconstruction is distinct from changing an existing entity's rank through late lookup. First compare ten return statements directly against C base/AVX2 and Rust, confirming identical results. Add a Rust regression and common runtime corpus to retain this distinction. No runtime implementation change was needed.

**Return-boundary gate:** native Windows default/portable each **426 passed / 17 ignored**; fmt/clippy/build pass; Python **27 passed**. Each C base/AVX2 runtime route: **5,331 cases / 5,331 passed / 0 runtime boundaries / 0 failed**; stages **10,767 checks**; words **6,618 cases**, all with zero failures. Keep **250 capture-graph** and **2 static** boundaries separate. Reverify actual source/reference/binary hashes in all ten reports. The ten new common statements participate in all three runtime routes and stage checks. Both the Verb-rank and Return-boundary gates are historical; the NV2 gate is the latest validation. Unverified scope and optimizer/CUDA/Linux/GitHub CI deferral are unchanged.

Sources: [cx.c modifier return fix](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L684), [af.c implicit operand](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/af.c#L117), [af.c reconstruct modifier](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/af.c#L193).

**Next checklist:**

- [ ] Extend noun-left rank/gerund runtime and noun-left GraphForm/Logical specialization against C constructor/call rules.
- [ ] General empty-prototype effects/error suppression, heterogeneous fill/padding, sparse, dyadic insert and full definition/control/body graphs remain incomplete. Header evidence must not bypass these execution boundaries.

Sources: [cr.c rank constructor](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c#L734), [sc.c NAME header copy](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sc.c#L364), [ja.h rank accessor](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ja.h#L745), [t.c primitive headers](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/t.c), [ap.c prefix/infix header](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ap.c#L965), [ar.c insert header](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ar.c#L1009).

#### JE3 — prove operator-specific higher-order views before a generic collection
- [ ] Default to operator-specific `GerundView` / `InterpretedEntitySequence`, not a generic EntityArray.
- [ ] Do not reproduce jsource's fake-BOX/function-payload carrier as RustJ `Value::Boxed` or a new J-visible noun type.
- [ ] Preserve source boxed-noun shape only where a specific operator's observable semantics require it.
- [ ] Audit whether current `decoded_gerund: Option<Vec<Arc<FunctionEntity>>>` loses required shape/order/name-binding information for each supported gerund operator.
- [ ] If an ordered sequence is enough, do not create a shape-bearing view.
- [ ] Extract a generic `EntityCollectionView` only after at least two independent J semantic use cases require the same shaped-entity algebra.
- [ ] Preserve Hook/Fork/train DAGs as shared FunctionEntity graphs and never invent arbitrary J-visible arrays of verbs.

Completion: jsource's internal representation convenience is not sufficient evidence for EntityArray. Generic collection structure requires a demonstrated common J semantic law.

#### JE4 — integrate gerund/boxed higher-order semantics
- [ ] Preserve gerund as boxed noun plus context-specific interpretation, not a global new POS/atom type.
- [ ] Create an operator-specific `GerundView`/`InterpretedEntitySequence` only where gerund interpretation requires it; retain source boxed-noun shape only when the operator needs it.
- [ ] Extract a generic `EntityCollectionView` only after JE3 proves cross-operator commonality.
- [ ] Do not copy container rank/shape onto function entities stored/referenced by the view.
- [ ] Preserve fix/late-binding/version rules for embedded names/functions.
- [ ] Compare the current `decoded_gerund` special case with the common entity view and remove it only if semantics remain exact.

#### JE5 — keep entity algebra separate from array-execution algebra
- [ ] Function entities remain semantic entities; an applied verb enters array-execution IR only when it consumes noun input(s) and produces a noun result.
- [ ] Validate monadic/dyadic verb application and modifier derivation through the common entity contract, including parser bident/trident actions that can immediately produce a Noun result.
- [ ] Keep CellApply/Reduce/Scan/Reindex in the applied array-computation layer.
- [ ] Keep effect flow orthogonal to entity/value flow.
- [ ] Ensure J Graph/Logical IR does not depend on EntityArray physical storage/layout.

#### JE6 — migration cleanup and cost validation
- [ ] Remove compatibility adapters and duplicate noun/function carriers.
- [ ] Stabilize ownership/lifetime/API documentation.
- [ ] Benchmark large derived functions, gerunds, and repeated bindings for deep-copy/refcount regressions.
- [ ] Extend the compilation-coverage manifest for entity-layer/runtime-fallback boundaries.
- [ ] Re-run frontend conformance and J Graph/A3 goldens after migration.
- [ ] Verify that BufferId/stride/device facts did not leak into the entity layer.

Completion rule: future progress reports for this work use JE0–JE6 item numbers. New requirements are added to this checklist first. Minimal JEntity work may land one M2 seam at a time; broad rewrites are prohibited. If JE3 does not prove a common shaped-entity algebra across real J semantics, generic `EntityArray`/`EntityCollectionView` remains deferred.

## 17.0 Module ownership baseline

M0 is complete. The intended dependency direction is:

```text
frontend
  ↓
semantic FunctionEntity
  ↓
J Graph IR
  ↓
execution-semantic contracts
  ↓
logical_ir::Plan
  ↓
route / schedule
  ↓
physical plan / representation
  ↓
backend / executor
```

Current ownership after M1 cutover:

- `analysis.rs` owns lowering mechanics, not shared execution vocabulary;
- `execution_semantics.rs` owns target-independent execution contracts;
- `compilation.rs` owns the cross-stage analysis bundle;
- `logical_ir::Plan` is the canonical execution IR.

Do not recreate the removed compatibility transition IR for new functionality.

Key ownership rules:

- parser/FunctionEntity owns J semantic construction, not target decisions;
- J Graph owns graph algebra and rewrite/resource analysis, not physical layout;
- Logical IR owns executable semantic dataflow/check/effect/error contracts, not buffers or devices;
- route/schedule/planner owns realization decisions;
- physical representation owns buffer/layout/device details;
- backend kernels do not define semantic legality;
- interpreter/reference-runtime flattening is not a canonical compiler model.

## 17.1 M1 completion gate — complete

- [x] J Graph lowering directly builds `logical_ir::Plan`.
- [x] Shared execution contracts belong to `execution_semantics.rs`.
- [x] `Plan::from_transition`, `TransitionProjection`, `transition_ir` and the legacy container/API have been removed.
- [x] `Engine::analyze/analyze_a3/analyze_diagnostic` return the canonical plan.
- [x] `CompilationAnalysis` contains J graph/rewrite/resource views plus `logical`, without a transition field.
- [x] Direct-A3 regressions preserve graph origin/span, name version, checks and observable write order.

M2–M4 retain their own completion gates; analysis-only success is not native compiler execution.

<a id="mean-proof-example"></a>

### 17.1.1 Matrix mean semantic proof case

For `y =: i. 2 3`, `(+/ % #) y` requires leading-axis reduction `[3 5 7]`, tally `2`, and scalar-rank division yielding `[1.5 2.5 3.5]`. Preserve the semantic Fork and h → f → g observable order. Matching the output shape is insufficient: innate ranks, prefix frame agreement and scalar repetition must be explicit in a shared cell-application contract.

A3 already provides basis payloads, verification and a closed-plan reference executor; the route prototype exists. General innate-rank/implicit-cell assembly and native Schedule/Physical Executor remain incomplete. The existing `canonical_mean_fork_lowers_to_reduce_tally_divide_in_jsource_order` test calls `analyze_a3 + verify` and checks ordering; it is not an execution/E2E test. Add matrix-cell golden/reference comparison before considering fused Mean, then connect the M4 physical route. [Detailed Korean proof case](PROJECT.ko.md#mean-proof-example).

## 17.1.2 M2 construction conformance progress (2026-10-02)

The frontend now shares `semantic::rank_noun_contract` across parser construction, analysis/facts, interpreter and Logical IR reference execution. It checks operand rank before length and numeric audit, implements fixed-fuzz integral conversion and jsource `vib` handling of infinities, out-of-range floats and NaN ranks, and clamps requested ranks to ±63 without replacing the source noun. Nested parentheses preserve completed noun operands in rank/noun-left-fork construction. Noun `/` and invalid `@:` operands report constructor domain errors; valid but unimplemented noun-left rank remains unsupported implementation.

Diagnostic context is boxed so Error stays at most 32 bytes; error kind/span/blame and inner-context precedence remain covered. Existing formatting drift was corrected. Native Windows default/portable each passed 208 tests with 17 pending definition acceptance tests ignored; fmt, clippy with warnings denied, and 11 Python harness tests passed. Ignored tests are not completion evidence. Each ordinary/AVX2 C reference passed 2,050 sentences through both direct and semantic-reference execution with zero mismatches or known deviations; each also passed 6,618 word cases (seed 20260927, 1,109 open quotes).

Execution used the official `build/w64.zip` release with commit metadata `ded7793fe5795d79eda8e7138dce94aa056edf78`. The parser source-review pin remains `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`; its Windows MSVC build failed on GNU C extensions, so these results are not a successful pinned build. `reports/frontend-*-windows.json` records actual library/binary SHA-256, revision and platform; the word harness no longer hard-codes a source revision.

To reproduce locally on Windows, run `tools/check-windows.ps1`, then `tools/check-frontend-windows.ps1 -ReferenceDirectory <DLL directory> -ReferenceRevision <verified 40-character commit> -SourceDirectory <jsource checkout> -SourceRevision <reviewed 40-character commit> -Avx2`. Override `-Python` if needed. GitHub CI, Linux tests, upstream full suite and CUDA validation were not run.

F0 evidence and P1 class/payload separation are complete; M2 remains incomplete. Remaining gates include intrinsic FunctionSemanticInfo storage, full modifier/result-POS semantics, shared runtime parser actions/fallback and right-to-left name/assignment sequencing. [Canonical detailed checklist](PROJECT.ko.md#architecture-migration-checklist).

## 17.2 M4 first vertical slice

The first native planner is deliberately simple:

```text
verified logical_ir::Plan
  ↓
deterministic all-CPU schedule
  ↓
Bind / Check / View / Materialize / Kernel / Return
  ↓
CPU Physical Executor
```

Correctness and boundary ownership come before a sophisticated cost model.

### 17.2.1 Minimal PhysicalPlan v0 contract

Plan-time resource identity must remain distinct from runtime handles:

```text
Logical ValueId
    !=
PlanBufferId        // symbolic/planned storage slot inside PhysicalPlan
    !=
physical::BufferId  // checked handle issued by runtime BufferRegistry
    !=
raw address
```

Likewise, buffer identity and view identity are separate. A conceptual `PhysicalViewId` names a checked shape/stride/offset/encoding/access view over a `PlanBufferId`.

The first M4 plan may be limited to one verified single-block pure-array CPU region plus required SemanticChecks. Stateful name/assignment effects may remain on RuntimeSemantic routes; that is an implementation-route limitation, not a restriction on valid J.

Conceptual v0 schema:

```text
PhysicalPlan
  source logical schema/provenance
  resolved CPU target
  planned buffers
  physical views
  ordered/dependency-aware ops
  outputs

PhysicalOp
  BindInput
  Check
  View
  Materialize
  Kernel
  Return

future/non-M4:
  Transfer
  Sync / AsyncToken
```

`BindInput` attaches a logical input/read to executor-owned runtime storage without implying a copy. `Check` executes an A3 SemanticCheck with the same J error kind/origin/order. `View` is metadata-only. `Materialize` performs an explicit copy/packing while preserving logical atom order. `Kernel` executes an already selected lowering recipe/realization and must not rediscover rank/train/fusion legality. `Return` establishes output ownership. Transfer/Sync are not required for the first CPU slice.

A planned buffer needs at least memory-space/CPU class, encoding, extent or size expression, alignment, ownership class (input/temporary/output), def/use/last-use, and optional reuse witness. A first slice may require fully resolved CPU extents; dynamic sizing remains a route capability rather than a language restriction.

Reuse requires completed last use, no outstanding observing view/lease, compatible size/encoding/alignment/memory space, alias/destination permission, and preserved J effect/error order.

The PhysicalPlan verifier must reject invalid IDs or use-before-def; out-of-bounds views; unbound buffers; dropped/duplicated or reordered SemanticChecks; target-incompatible selected kernels; unproved overlapping writable views; materializations that do not preserve logical atom semantics; reuse without last-use/alias/ownership evidence; dangling returned views; and hidden namespace/write effects in an M4 pure-region plan.

Error/cleanup contract: Check failures remain J semantic errors; backend implementation failures are not silently reclassified as J Domain/Rank/Length errors; executor-owned temporaries are cleaned up without destroying caller inputs; the first pure slice does not own namespace-assignment commit; and no transparent replay occurs after observable effects have committed.

Current `src/physical.rs` is only the G1 representation foundation (`BufferRegistry`, `BufferLease`, runtime `BufferId`, and checked read-only affine PhysicalArray). It is not a PhysicalPlan, planner, or physical executor. `logical_executor.rs` is an A3 semantic/reference executor, not the Physical Executor.

Implement in one-meaning/one-test steps: plan-time buffer/view IDs plus empty-plan verification; BindInput+Return identity E2E; View span verification; Check/error-order regression; one Kernel realization (Add) with capability verification; Materialize ownership/order tests; last-use/reuse witnesses with alias-negative tests; then a multi-op Logical IR → PhysicalPlan → CPU differential case.

---

# Part XV — Documentation policy

## 18. Korean canonical / English mirror

Maintained user-facing Markdown follows this policy:

- `README.ko.md` — Korean canonical
- `README.md` — English mirror
- `PROJECT.ko.md` — Korean canonical
- `PROJECT.md` — English mirror
- `FOUNDATIONS.ko.md` — Korean canonical
- `FOUNDATIONS.md` — English mirror
- `AGENTS.md` — English-only internal maintainer/agent guidance

Design work is authored in Korean canonical files first. English mirrors are updated to remain semantically aligned for external readers.

`PROJECT.ko.md` is the authoritative architecture/design/progress document; `PROJECT.md` is its English mirror. The syntax-hint review and framework comparison are integrated in §7.3–7.4 here and §4.1.2–3 in Korean. Do not maintain separate design-review reports; keep measurement raw data in `reports/`. Future design/structure changes update the canonical document first and its English mirror in the same change.

Maintenance rules: keep the overall code-inspection/status summary in §16 and completion gates in §17/stage checklists. Put graph research in §7, extension vocabulary in §5.3, the matrix-cell proof in §17.1.1 and validation policy in §15. Date historical audits instead of presenting old v0.1/transition stages as current. Section moves update both languages and cross-links while preserving sources, adoption/support distinctions and unique validation conditions.

If a discrepancy exists, the Korean canonical file is authoritative.

### 18.1 Documentation-completeness audit — are stage contracts closed? (2026-10-06)

RustJ already documents many individual topics in depth, but depth is not the same as an end-to-end architecture that a reader can reconstruct. The recurring gap is **the contract between stages**. A major compiler stage is considered documentation-complete only when the documentation answers:

```text
1. Why does the stage exist?
2. What are its inputs?
3. What are its outputs?
4. What semantic information must it preserve?
5. What decisions must it not make?
6. What are its upstream/downstream contracts?
7. Is there a representative source-to-IR/plan example?
8. What is implemented today?
9. Which verifier/tests prove the boundary?
10. What remains unimplemented or undecided?
```

This is a documentation-connectivity audit, not a score of design quality or implementation completion.

| Stage / boundary | Documentation status | Strong coverage today | Main remaining gap |
|---|---|---|---|
| word formation → enqueue → parser | **strong** | A0.5/F0–F2/P0–P8, jsource oracle, 9-row reductions, name/assignment sequencing, differential gates | A compact canonical trace from tokens through queue/reductions to completed `JEntity/FunctionEntity` would improve orientation |
| Semantic Construction / binding / dynamic semantics | **strong** | FunctionEntity/JEntity, late NameRef, assignment=value+effect, definition frames, gerund/rank/train preservation | Concrete explicit-definition control-flow handoff into A3 regions/blocks remains partial |
| J Semantic → J Graph IR | **strong** | GraphForm/GraphBasis/GraphHint, provenance, applied graph, `@:`/fork diagrams, Graph-vs-Execution distinction | Canonical graph examples for hook/rank/reduce/scan are still distributed across sections |
| graph analysis → candidate/proof | **partial** | rewrite witnesses, fusion proof obligations, memory/resource/work-depth analyses, target-feasibility separation | No closed proof-discharge owner/state machine, overlapping-candidate conflict policy, or canonical illegal-candidate representation; fusion is still `AwaitingSemanticProofs` with `selected=false` |
| J Graph → execution-semantic lowering → A3 | **mostly strong** | direct lowering, fact-drift checks, Execution Basis, SemanticCheck, effects/errors/speculation, verifier, schema header | A3-v0 is still effectively single-block; schema support must not be confused with completed control-flow lowering |
| route analysis / partition | **partial** | `RouteDecision`, capabilities/recipes, mixed-route principle, RuntimeSemanticFallback, current contiguous-class partition | Missing one closed region-boundary ABI for live-ins/outs, representation/transfer/materialization, effects/tokens, errors, and whole-region legality |
| schedule / Physical Planner | **partial — high priority** | logical/schedule/physical separation, target/resource/cost models, G1 representation foundation | No canonical `PhysicalPlan` schema/verifier yet; M4 needs the minimum Bind/View/Materialize/Kernel/Transfer/Sync/Return contract first |
| native executor | **partial** | clear non-responsibilities, logical reference executor, G4 goal | Physical-plan op semantics, cleanup/error/async completion, and E2E oracle are not yet closed |
| fallback / guard miss / replay | **partial — high priority** | language validity vs route eligibility, RuntimeSemanticFallback, scattered no-replay-after-effects rules | Need one decision table separating compile-time route miss, guard miss, runtime Unsupported, J semantic error, and post-effect failure |
| external route / GPU | **planned** | adapter responsibility, external IR as projection, target/lowering separation | First concrete adapter ABI, round-trip verifier, and unsupported diagnostics are not implemented; CUDA remains intentionally deferred |
| validation / versioning | **partial** | strong frontend differential gates; A3 verifier/schema/provenance fields exist | Candidate→route→physical negative verifier matrix and serialization migration policy remain future work |

Highest-priority documentation closures, without changing the current M2 implementation priority:

1. Define the candidate lifecycle and proof-discharge ownership: `Discovered → AwaitingProofs → Legal/Illegal → Costed → Selected/Rejected → Lowered`, while allowing speculative resource/cost analysis before legality but forbidding selection before required proofs.
2. Define the `RouteRegion` boundary contract: live-in/out values, effect/error ordering edges, representation and transfer/materialization obligations, and the fact that today's contiguous-class `partition_plan` is a v0 analysis helper rather than the final mixed-route planner.
3. Define the minimal M4 PhysicalPlan schema and verifier before implementing it. Keep the current G1 `physical.rs` representation foundation distinct from a planner/executor.
4. Consolidate fallback/no-replay semantics into one decision table. In particular, distinguish route miss and guard miss from semantic J errors, and forbid automatic replay after observable effects have committed.
5. Maintain one canonical end-to-end compiler trace—e.g. `(+/ % #) y` or `f @: g`—from source through semantic construction, J Graph IR, candidates/proofs, A3, route, minimal PhysicalPlan, and CPU result/error, explicitly marking unimplemented stages.

Maintenance rule: whenever a major compiler stage/type or framework-comparison claim changes, update the relevant contract summary and cross-search `FOUNDATIONS`, `PROJECT`, `README`, and `AGENTS` for stale duplicate claims. Keep these audit results inside the canonical project documents rather than spawning separate Markdown review reports.

Code identifiers, comments, doc comments, test names, diagnostics, and commit messages remain English unless a specific case requires otherwise.


---

## License policy

RustJ's public open-source distribution path is GNU General Public License version 3, `GPL-3.0-only`.

RustJ also preserves a separate commercial-licensing path in the same general direction as current `jsource`. A commercial RustJ license may be granted only to the extent that the applicable RustJ copyright holder(s) actually possess the rights needed to relicense the relevant material.

Where commercial use or distribution of RustJ depends on code or other rights derived from J SOURCE, the necessary Jsoftware commercial J SOURCE license and other upstream rights must also be obtained and complied with. RustJ's `LICENSE` does not itself grant Jsoftware-owned commercial rights.

Current policy:

- public RustJ distribution without the necessary Jsoftware commercial rights: `GPL-3.0-only`
- where the necessary Jsoftware commercial rights and RustJ-side rights are both available: a separate RustJ commercial license may be offered
- external Contributions must grant the project the rights in `CLA.ko.md` / `CLA.md` needed for GPL distribution and separate commercial relicensing
- the CLA does not transfer contributor copyright to the project
- the CLA does not expand any rights owned by Jsoftware or another third party

`LICENSE` is the governing notice and `COPYING` contains the full GNU GPL v3 text. Cargo metadata remains `license = "GPL-3.0-only"` for the public open-source option. Do not change it back to `GPL-3.0-or-later`.
