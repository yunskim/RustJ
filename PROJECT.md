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
- **Current priority:** [§O.5 framework migration checklist](#framework-migration-checklist) and [§Q whole-jsource optimization checklist](#jsource-optimization-migration) tracks M2→M3→M4 acceptance gates; continue M2 tokenizer → enqueuer → parser convergence. Preserving graph structure/partial facts is distinct from permitting optimization/execution. Then close M3 boundaries and validate the M4 Native CPU vertical slice. Retain GPU-friendly design while deferring CUDA implementation. Open external routes incrementally where capability is proven.
- **Latest validation:** as of NV3d2b2a (2026-10-05), Windows default/portable each **474 passed / 17 ignored** and Python **30 passed**. Existing j64/AVX2 runtime routes remain **5,380 / 5,380 passed / zero failures**, with stages **10,810** and words **6,623**. Numeric syntax is **2,485 cases / zero failures** per DLL, but one unresolved recognition boundary and one unresolved error boundary remain and are not counted as execution/error-equivalence success. Keep 257 capture-graph, two static, and 285 runtime-prefix / zero executable-prefix-pass boundaries separate. The latest graph-readiness gate is GF6a and does not mean fusion selection or GPU execution is implemented. See the NV3d2b2a/GF6a gates and current validation summary.
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

#### Canonical J Graph example suite

| Source | semantic construction / preserved provenance | applied topology | candidate | never infer from syntax alone |
|---|---|---|---|---|
| `f @: g` | Atop-derived Verb + Pipeline provenance | `input → g → f` | fusion/materialization elision | fused kernel, target placement, check removal |
| `(f g h) y` ordinary fork | Fork with original f/g/h + observable branch order | shared-input fan-out into `h(y)` and `f(y)`, joined by dyadic `g` | parallel-branch, branch/join fusion, retained/live-across | actual concurrent execution or branch reordering |
| `([: g h) y` capped fork | source Fork plus immutable capped-construction fact | `h(y) → g(monad)`; no executable first branch | pipeline/materialization | ordinary-fork parallel/retained treatment or calling `[:` as a branch |
| `(f g) y` hook | Hook provenance + shared original input | `g(y)` and retained `y` feed dyadic `f` | retained-input/materialization, legal fusion | dropping the shared input or arbitrary reorder |
| `u"r y` | Rank-derived Verb, requested-rank provenance | outer CellApply around the inner operation basis | cell parallelism, nested CellApply absorption/fusion | physical loop/thread mapping or rank-boundary collapse |
| `u/ y` | Insert-derived Verb | Reduce basis with operand `u` provenance | reduction realization, legal map/reduce fusion | tree reassociation, altered empty/identity behavior, arbitrary parallel reduction |
| `u\ y` | Prefix/Infix-derived Verb | preserved prefix/window family structure | witnessed Scan candidate or window/reduce rewrite | immediate replacement by Scan without associativity/error/numeric proof |

The suite deliberately keeps **source construction identity and applied dependency graph together**. Two forms may happen to lower to similar SSA DAGs while differing in name/effect/error/constructor semantics. Current GraphForm/GraphBasis/hint support and capped-fork/scan regressions do not imply that the candidate proof/selection lifecycle is fully implemented.

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

#### 4.3.1 Canonical frontend sentence trace — `+/ y`

Assume `y` is already bound to a noun. This compact sentence crosses word formation, enqueue classification, parser-time name lookup, modifier construction, and monadic application:

```text
source: "+/ y"
  -> words: "+"  "/"  "y"
  -> enqueue:
       Verb(Add)
       Adverb(Insert)
       Name("y", lookup_name=true)
  -> parser stack entry:
       Name("y") -> current-environment Noun snapshot
  -> row 3 / Adverb:
       Verb(+) + Adverb(/)
       -> completed FunctionEntity
          POS=Verb
          head=PrimitiveAdverb(Insert)
          operand[0]=Function(Add)
  -> stack reinsert + rescan
  -> row 0 / MonadEdge:
       derived Verb(+/) + Noun(y)
       -> Expr::Monad / completed noun result
```

`/` is not `Reduce` at enqueue time; it is an Adverb, and row 3 constructs the derived verb. `y` is not pre-snapshotted at sentence start; its NAME lookup occurs at parser-stack entry. Only after monadic application does an applied noun computation exist that J Graph/A3 may later analyze as reduction structure and lower toward `Reduce(Add)`.

Runtime parser actions may execute the call and reinsert a noun value, while static/analysis mode retains application structure; both must share the same nine-row language semantics. Existing enqueuer/parser-provenance tests cover class/lookup/provenance and row-3 behavior, while `+/1 2` analysis coverage verifies that the derived semantic head remains `PrimitiveAdverb(Insert)` through lowering. This is an orientation trace, not proof that locatives, direct definitions, gerunds, and all value-dependent constructors are complete.

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

#### Explicit-definition control-flow handoff

Use `f =: 3 : 'if. y do. 1 else. 0 end.'` as the orientation example. Today DefinitionInput/DefinitionCode retain source/body, valence body ranges, sentence provenance, and control nodes/jump metadata; the semantic FunctionEntity retains `ExplicitDefinition(DefinitionCode)`. A **runtime straight-line subset already has per-call LocalFrame invocation**, while compiler body-graph/A3 CFG lowering remains the stop line.

Current/planned handoff:

```text
DefinitionCode
  source/body + monad/dyad ranges + control metadata
      |
      v
[current runtime subset] per-call LocalFrame / straight-line invocation
  supported mode 1/2
  x/y/u/v/m/n bindings
  local-first -> global lookup
  local/global assignment + cleanup
  non-Body control remains Unsupported
      |
      -------- compiler body-graph / CFG lowering stop line --------
      |
      v
[planned] body semantic CFG
  entry condition
    |- true  -> then block
    `- false -> else block
  merge preserves J previous-result / return semantics
      |
      v
[planned] A3 Region/Block CFG
  CondBranch / Branch + block-argument or phi-like result merge
  Return
```

Current A3 already has Function→Region→Block containers, but its v0 `Terminator` currently has only `Return`. Therefore existing Region/Block types do **not** imply that `if./while./try.` lowering is implemented. Branch/CondBranch and value-merge mechanisms above are planned concepts.

Do not turn J Graph IR into a generic CFG merely to host explicit definitions. Analyzable array expressions inside a basic block may use J Graph applied-computation/provenance analysis; control edges, compiler-visible invocation-frame/resource modeling, namespace/effect resources, and block merges belong to definition-control/A3 lowering. Current runtime LocalFrame behavior is semantic evidence that the compiler lowering must preserve, not a substitute for CFG lowering. Future verification must cover control-source provenance, branch targets, frame independence, current local-first lookup semantics plus future locale/path support, previous-result/return merges, and effect/error ordering.

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
- [x] **P2/P4 runtime actions (supported subset):** `RuntimeParserHost` now supplies stack-entry lookup and rows 0–2 invocation through the shared nine-row engine, returning actual Values to the same stack. Unsupported effectful/locale/definition forms remain explicit coverage boundaries.
- [x] **P2/P5 capture carrier:** opt-in ParseCapture occurrence ids, associations and ordered attempt/success/failure events are implemented without compiler identity in Value storage or primitive executors; capture parity is regression-tested.
- [x] **P3/P5 construction origins (supported subset):** completed FunctionEntity structure, construction attempt/success and computed-noun occurrence origins are connected; runtime values remain distinct from static value-dependent boundaries. This is not complete constructor-vocabulary support.
- [x] **P4 names/effects (supported subset):** supported same-sentence lookup/assignment/POS and committed-effect versus pending-outer-commit ordering are captured/regression-tested. General user locales/paths and unsupported effects remain boundaries.
- [x] **P5/P8 graph adapter (successful-capture subset):** successful capture dependencies translate into the existing J Graph and pass its verifier. Failed/opaque/dynamic boundaries are not promoted to executable complete graphs.
- [x] **P6 supported-corpus differential/capture gate:** later Windows default/portable plus j64/AVX2 gates repeatedly record coverage boundaries and revisions/hashes. This does not prove full runtime `ptcol` internal traces, full J, locales or definition acceptance.
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

**Historical-status note:** when this subsection was first written on 2026-10-03, it was research/plan only. Later work implemented runtime row actions, the capture API, successful-capture→J Graph adaptation and supported-corpus differentials. Remaining gaps include full runtime `ptcol` internal-trace equivalence, general locale/definition/control/effect coverage, safe capture-based static reuse/guards, and the full memory-retention/performance gate. The 212-test / 7,014-stage figures below are historical, not the latest validation.


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

**Terminology note:** `UnsupportedImplementation` is an architecture-level category in these documents, not a current Rust enum variant. The concrete code path today uses `Error::Unsupported(String)` and machine-readable kind `"unsupported"`. Use the conceptual term when distinguishing J-valid-but-unimplemented coverage from J semantic errors; use the concrete name when documenting current APIs/tests.

| Situation | J semantic error? | Alternative route? | Replay? | Current rule |
|---|---|---|---|---|
| target capability miss during compile/lowering | no | yes, if execution has not started and a verified alternative exists | unnecessary | route miss; otherwise conceptually UnsupportedImplementation (current concrete API: `Error::Unsupported`) |
| specialization guard miss before observable effects | no | yes, if reanalysis/verified fallback exists | prefer route reselection rather than restart semantics | never expose the miss as a J error |
| explicit compiler/API contract violation | distinct from J semantics | according to that contract | not automatic | keep contract errors separate from Domain/Rank/etc. |
| A3 SemanticCheck or semantic call raises Domain/Length/Rank/Index/etc. | **yes** | no; do not switch backend to evade the same J error | no | preserve J class and precedence |
| backend-adapter precondition miss before region execution | no | yes, with a verified alternative | unnecessary | reject that route only |
| native/external Unsupported before any observable effect starts | no | conditional, only when the whole region remains untouched and the alternative is verified | v0 treats this as route reselection | implementation-coverage boundary |
| implementation failure after partial kernel/external execution | normally not a J semantic error | not automatically | **forbidden by default** | cleanup, then report implementation failure/Unsupported unless a transactional contract exists |
| failure after namespace write/I/O/other observable effect commit | may coexist with already observable J state | not automatically | **forbidden** | do not claim mid-execution fallback without verified exact continuation |
| async failure after completion/token/resource publication | not automatically a J error | only if completion/effect state is fully proven | forbidden by default | future async contract must own the completion/effect frontier |

Current `lowering.rs::RouteDecision::RuntimeSemanticFallback` means **compile-time classification that this operation needs a semantic/runtime route because no current native ExecutionBasis realization is available**. It does not promise that RustJ may run a native kernel, fail halfway, and jump back to the interpreter. If the semantic/runtime route itself does not support the form, the result is conceptually `UnsupportedImplementation`; the current concrete API is `Error::Unsupported(...)` with kind `"unsupported"`.

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

## 7.4.3 jsource-derived optimization principles and RustJ placement

Review baseline: `jsoftware/jsource` source pin **`13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`**. This is not a plan to clone jsource's hand-written special entry points one-for-one. It mines accumulated J-specific optimization knowledge and relocates it into RustJ graph facts, candidate rewrites, and downstream execution planning. **J Graph discovers candidates; Execution Semantic Lowering/proof validates call-dependent legality; Physical Planning chooses concrete realization.** This catalog is representative, not exhaustive.

| jsource observation | Direct source | Generalized RustJ idea | Placement |
|---|---|---|---|
| Derived verbs propagate execution-relevant properties | `ca.c::jtatop/jtatco`; `jtype.h` `VF2*` | Distinguish intrinsic traits from engine-specific result-assembly/consumer-demand hints; propagated runtime flags are not themselves J semantic identity. | **J Graph provenance + call facts / assembly** |
| `@:`/capped-fork/special-form recognition | `ca.c` `SPECAT/SPECATCO`; `cf.c::jtfolk` | Canonicalize equivalent applied topology while retaining source provenance; expose composition as fusion/materialization candidates. | **J Graph canonicalization + discovery** |
| Atomic `f/@:g` can reduce intermediate materialization | `ca.c::jtatco`; `va2.c::jtfslashatg` | Cell-at-a-time execution is **not proof of a single fused kernel**; source checks dense/nonempty, type compatibility and whether inplace is more beneficial, and may fall back. | **Graph fusion/streaming candidate → guarded execution schedule** |
| Nested rank loops can be subsumed | `jtype.h` `VF2RANKATOP*/RANKONLY*`; `ca.c`; `cr.c` | Preserve cell/frame iteration domains and discover compatible rank-domain absorption/fusion. | **Graph/execution semantic analysis**, concrete loop later |
| `+/%#` gets `jtmean` | `cf.c::jtfolk`; `ar.c::jtmean` | Mean is a high-level idiom, not necessarily a fused pass: jsource computes reduce then divides by cell length. Require numeric/rank/error witnesses before replacement. | **Graph idiom → execution selection** |
| Mean under infix/window becomes moving average | `ap.c::jtbslash/jtmovavg/jtmovsumavg`, with generic fallback | Rewrite rediscovery matters, but so do sliding-window algorithm/dtype choices, NaN/overflow handling and exact numeric behavior. | **Graph idiom → Window planning** |
| `+/@:*"1 1` gets `jtsumattymes1` | `cr.c` choice; `va2.c::jtsumattymes1` | Dot-like only under specific rank/dtype/empty/sparse/fit (`!.0/!.1`) paths. Generic Dot substitution is not automatically valid. | **Graph idiom + witnessed numeric lowering** |
| `#@,`, `#@$`, `*/@$` shortcut to rank/atom count | `ca.c`; `v.c::jtrank/jtnatoms` (sparse has a shape-based route) | Use shape/rank/count demand while preserving sparse/empty/prototype and observable check/error behavior. | **Graph facts / shape rewrite** |
| `BOXATOP/WILLOPEN/ATOPOPEN/USESITEMCOUNT` coordinate result assembly and consumer | `ca.c` explanation; `cr.c` result assembly; `jtype.h`, `result.h` | This is **not universal algebraic Box→Open cancellation**: it controls virtual boxed contents, recursive assembly/EPILOG, and raze count/shape checks. | **Graph demand → guarded assembly/materialization** |
| Ravel can use virtual blocks/header reshaping | `v.c::jtravel` incl. `ASGNINPLACESGN`, `AFNJA` | Expose a logical view opportunity, but not every reshape/take/transpose is zero-copy or a simple stride view; alias/pristinity/ownership matter. | **Graph view fact → physical representation** |
| Comparison/search/set combinations get specialized algorithms | `ca.c` ranking and comparison forms; `cf.c::jtfolk` `jtintersect` under `#if C_VIAVX` | Promote recognized Ranking/Intersection/Search idioms, but keep build capability and tolerance/type restrictions distinct from semantic proof. | **Graph candidate → target/algorithm selection** |
| Use count and inplaceability drive storage reuse | `v.c::jtravel` incl. use count, pristine and incorpable checks; `JTINPLACE*` | SSA liveness alone is insufficient: require ownership/alias, recursive boxed contents, rank/result shape, and error/retry legality. | **Buffer planner / physical lowering** |
| Cache footprint/SIMD/special routines influence the execution path | `va2.c::jtfslashatg` and specialized entry points | Keep fusion/streaming freedom and logical extent in graph IR; select chunking, SIMD, GPU workgroups, and libraries later. | **Target lowering / schedule / cost** |
| Reduce has empty/singleton/two-item/type-specialized paths | `ar.c::jtreduce/jtslash` | Identify neutral/singleton shape candidates, but preserve prototype/numeric semantics and select small-cell routines downstream. | **Graph facts → Reduce lowering** |
| Scan/infix has multiple specialized sliding algorithms | `ap.c::jtpscan/jtmovfslash` | Distinguish Scan from Window; sliding sum/min/max/boolean/XOR paths need numeric, NaN, overflow, and fallback contracts. | **Graph Scan/Window → schedule** |
| Search/index-of can use prehash or sorting + binary search | `vi.c` IPH modes and `jtiobs` | Prehash modes do not guarantee applicability; `jtiobs` is limited to `ct=0` tolerance and selected boxed high-rank/numeric-box cases. Select hash/sort/generic based on facts and cost. | **Graph Search → execution algorithm selection** |
| Under/each has structural and specialized paths | `cu.c` `u&.>`, `jtsunder`, inverse caching for `nameless(wvb)` | Preserve inverse and dynamic-name binding timing, effects, aliases; structural and cached-inverse routes require legality proof. | **J Graph Under provenance → execution** |
| Bound constants enable specialized numeric algorithms | `ca.c` constant `2&^.` log2 and modular-power cases | Preserve bound constants as facts; demand full numeric-domain/fit/overflow witnesses. | **Graph constant candidate → numeric lowering** |

Pinned sources:

- [`jsrc/ca.c` — Result Assembly flags](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ca.c#L269-L289) · [`jtatop/jtatco` patterns](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ca.c#L293-L516)
- [`jsrc/cf.c` — `jtfolk`, capped-fork normalization, mean and intersection specializations](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L55-L198)
- [`jsrc/cr.c` — rank/IRS selection and `jtsumattymes1`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c#L779-L799)
- [`jsrc/ap.c` — infix dispatch](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ap.c#L940-L965) · [moving-average fallback](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ap.c#L769-L780)
- [`jsrc/va2.c::jtfslashatg` — cell-at-a-time execution and fallback](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/va2.c#L1806-L1850)
- [`jsrc/v.c` — rank/atom-count shortcuts, virtual ravel and inplace/header reuse](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/v.c#L8-L40)
- [`jsrc/jtype.h` — BOXATOP/WILLOPEN/USESITEMCOUNT/RANKATOP/RANKONLY contracts](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/jtype.h#L1280-L1320)
- [`jsrc/result.h` — WILLBEOPENED/COUNTITEMS result-assembly contract](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/result.h#L20-L36)

- [`jsrc/ar.c` — empty/singleton reduction, `jtslash`, `jtmean`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ar.c#L818-L849) · [Mean](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ar.c#L1018-L1025)
- [`jsrc/ap.c` — sliding sum/min/max/boolean/XOR paths](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ap.c#L901-L965) · [numeric fallback](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ap.c#L750-L780)
- [`jsrc/vi.c` — hash/prehash modes, sort+binary search](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vi.c#L142-L184) · [sort path](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vi.c#L804-L839)
- [`jsrc/cu.c` — Under/Each and structural-under special paths](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cu.c#L391-L439)
- [`jsrc/va2.c` — dot-like rank/type/fit boundary](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/va2.c#L1640-L1687)

The layer boundary is deliberate: Graph IR/facts own provenance, rank/cell/frame topology, shape/count, demand, logical-view opportunities and semantic traits; graph optimization owns fusion, rank-domain absorption, shape/count simplification, materialization elimination and idiom recognition; execution/physical planning owns explicit loops, buffer reuse, concrete views, cache blocking, SIMD/multicore/GPU mapping and library/custom-kernel selection.

Do not make each jsource special entry point a Graph IR node, and do not copy runtime flags such as `WILLOPEN` or `RANKATOP` as semantic identity. Each rewrite still requires its own witness for J-visible dtype, rank/frame/cell, empty/prototype, fit/tolerance, overflow/promotion, and effect/error order. jsource special cases are optimization-source evidence, not sufficient correctness proofs. This section records design input only; it does not claim these optimizations are implemented.

**Independent audit (2026-10-06):** Corrected overgeneralizations about kernel fusion, algebraic Box→Open cancellation and generic Mean/Dot equivalence; documented restricted fallback/type/rank/fit cases, plus reduction, sliding/scan, hash/search, Under/Each and constant-specialization omissions. This is a source/document review, not a RustJ implementation or differential/benchmark run.

### Independent verification using three separate criteria (2026-10-06)

| Review | Independent question | Findings | Limit |
|---|---|---|---|
| **1. Source trace** | Which exact constructor/entrypoint/fallback paths exist? | Rechecked pinned `ca/cf/cr/va2/ar/ap/vi/cu`, clarified `C_VIAVX` gate for `jtintersect`, `ct=0`/boxed restrictions for `jtiobs`, and expanded fallback source links. | Not an exhaustive jsource inventory |
| **2. Semantic counterexamples** | Which apparently similar graph transforms can change J behavior? | Independently checked empty/sparse, rank/frame/cell, `!.`/promotion/NaN/overflow, boxed assembly/virtual alias, dynamic names, effect/error order. Rejected unconditional Mean/Dot/Box→Open/fusion rewrites. | No runtime/differential tests run |
| **3. IR/document boundary** | Which layer owns discovery, proof and realization? | Compared graph discovery, §7.5 candidate evidence, execution semantic lowering and Physical Planner; their boundary is consistent. Repaired the broken Markdown table. | No implementation/performance claim |

**Revision check:** pinned source baseline `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528` is 20 commits behind jsource `master` at `0a5101cfdd834b23a0b89d455e4f327310520a08` (2026-10-05); intervening changes include `ap.c/ar.c/va2.c`. The inspected entry/guard excerpts for `jtmovavg`, `jtmovfslash`, `jtmean`, `jtreduce`, `jtfslashatg`, `jtsumattymes1` matched; this does not verify entire functions or all current-head paths.

### Pending differential-regression matrix (not executed)

| Candidate | Boundaries to test | Required legality result |
|---|---|---|
| `f/@:g` | empty/sparse, inplace, dtype mismatch, overflow reversion | value/type/error order and effect-safe fallback |
| `+/%#` / Window(Mean) | zero/one/multiple cells; int/float/NaN/overflow; window lengths | shape/prototype/dtype/numeric equivalence |
| `+/@:*"1 1` | mixed-rank/empty/sparse, `!.0`/`!.1`, QP | rank/agreement/result type and fallback |
| BOXATOP/WILLOPEN/USESITEMCOUNT | nested boxes, nonuniform shapes, raze, sparse and virtual aliases | assembly/usecount/error/order invariants |
| Search/Under/View | boxed/tolerance/prehash, rebinding, shared/inplace ravel | search identity, inverse binding timing, alias legality |
| Every rewrite | proof/source version, effect/error order, guard miss | never commit unknown; no replay after effects |

jsource code paths are sources for candidate discovery, not correctness proofs or performance measurements. Unverified candidates must not be marked selected or realized.

### H. Additional missing optimization families found in repository-wide audit (2026-10-06)

The preceding catalog of 18 representative observations was **not** an exhaustive account of jsource optimization. Reviewing the pinned `jsrc/` source tree and additional functional families revealed these **previously omitted or over-collapsed families**.

| Missing family | Pinned direct source | What jsource specializes and where it is legal | RustJ owner |
|---|---|---|---|
| **Key/Group-by plus aggregation** | [`ao.c::jtkeyct`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ao.c#L213-L260), [`jtsldot`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ao.c#L824-L850) | Selected `u/.`/`f//.` cases directly accumulate group sum/min/max/mean rather than universally materializing reorder→Cut→Reduce. Requires key equality/tolerance, group ordering, type and overflow witnesses. | Graph **Key/GroupReduce candidate** → group algorithm |
| **General inner product / matrix multiplication** | [`cip.c::jtpdt/jtdot`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cip.c#L715-L739), [size-dependent route](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cip.c#L925-L956), [`gemm.c`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/gemm.c#L923-L985) | Broader than the existing rank-1 dot-like `jtsumattymes1`. `+/ . *` selects small/cached/BLAS/in-house dgemm/zgemm/igemm routes with numeric fallback. | Graph **Contraction** → numeric witness + physical GEMM route |
| **Grade/Sort/Ranking strategy selection** | [`vg.c`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vg.c#L525-L557), [`vgsort.c`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vgsort.c#L120-L149), [`vgranking.c`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vgranking.c#L34-L69) | Count/histogram+prefix, radix, quick/merge and direct sorting depending on range, item length and size; optional AVX512 gated route. Requires J equality/order and tie stability contracts. | Graph **Grade/Ranking** → target-aware algorithm choice |
| **Tolerance-aware hash table algorithms** | [`viavx2.c` neighbor hash intervals](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx2.c#L10-L49), [`viavx.c` table layout](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx.c#L15-L43) | Naive exact hashing cannot implement J tolerance. Neighbor-bucket probes, signed-zero handling and packed/AVX table strategies are distinct from prehash entrypoints. | Search tolerance semantic witness → hash planner |
| **Interval Index `I.`** | [`viix.c` boolean/small-range](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viix.c#L18-L48), [binary search](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viix.c#L55-L86) | A separate ordered-interval operation from general `i.` membership/hash search; range and sortedness permit table or branchless search. | Graph **IntervalLookup** → search route |
| **Cut/substring virtual paths** | [`cc.c::jtrightcut0`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cc.c#L133-L147), [`jtboxcut0`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cc.c#L158-L210) | Selected one-dimensional segments can be virtual; consumer-will-open permits boxed virtual contents. Reverse/negative paths and lifetime may force copying/fallback. | Graph Cut/segment + demand → View/Result Assembly |
| **From/Gather copy-vs-view** | [`vfrom.c::jtget1cell`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vfrom.c#L39-L53), [AVX gather path](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vfrom.c#L70-L108) | Small cells may copy, large contiguous cell selections may return virtual blocks; arbitrary indexed gather is not a universal zero-copy view. | Graph access facts → Physical gather/layout |
| **Reshape/Compress/Catenate ownership and copying** | [`vf.c`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vf.c#L301-L331), [`vrep.c`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vrep.c#L53-L93), [`vcat.c`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vcat.c#L192-L221) | Conditional header reuse, virtual reshape, in-place compression, and recursive-box ownership transfer require alias, pristine/usecount, fill and lifetime checks. | Graph materialization facts → buffer/ownership planner |
| **Sparse-specific algorithms** | [`cpdtsp.c`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cpdtsp.c#L1-L43), [`vgsp.c`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vgsp.c#L1-L25), [`visp.c`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/visp.c#L1-L25), [`vfromsp.c`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vfromsp.c#L1-L35) | Specialized sparse inner product, grade, index-of and From do not imply dense fallback or unconditional dense fusion. | Sparse semantic facts → sparse execution route |
| **Dynamic name-reference / locale caching** | [`sc.c` lookup](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sc.c#L57-L100), [invalidation/locale guards](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sc.c#L140-L178) | Timestamp/invalidation, short- vs long-term cache, permanent locale and atomic cache updates optimize dynamic lookup *without changing J binding semantics*. | Name/Binding runtime cache, **not** a Graph algebraic rewrite |

These families need different layers: source syntax/graph identity permits candidate recognition, Execution Semantic Lowering owns rank/type/tolerance/error/effect proof, and physical planning owns sorting/hash/GEMM/view/sparse/memory route choice. Name lookup cache is a separate interpreter/runtime semantic boundary.

### I. Explicit coverage limitation

This additional review searched the pinned `jsrc/` source inventory and sampled source files in grade/sort, group/key, inner product/GEMM, tolerant hash/index, cut, data movement, sparse, and name caching. **It does not establish that no other jsource optimizations exist.** In particular, a complete inventory of `p.c` parser/assignment paths, `cx.c` explicit definitions, primitive numeric `va1/v0/v1/v2`, allocator/amend `m.c/am.c`, and all SIMD/assembly microkernels remains unverified. Suggested first research priorities are high-level Key/GroupReduce, Contraction, Grade/Ranking, and IntervalLookup identities, followed by their legality witnesses and target-specific algorithms. This was a source/document audit, **not** J/RustJ differential execution or a benchmark.

### J. Framework-native RustJ integration (2026-10-06)

This section distinguishes source-derived **design evidence** from actual RustJ code integration. No verified performance improvement or semantically committed transform is claimed.

| Source-derived idea | Existing RustJ owner | Code status | Gate before execution optimization |
|---|---|---|---|
| `f/@:g` Map→Reduce streaming | Existing `j_graph_fusion.rs::MapReduce` and `fusion_planning.rs` | Reuse the existing envelope; do not add a duplicate rewrite | cell/rank, numeric/type, effect/order, target feasibility, fallback |
| `+/%#` Mean fork | New `j_graph_jsource.rs` source analyzer | Recognize **monadically applied ordinary Fork of Insert(Add), Divide and Tally** as a source-backed **MeanIdiom candidate only** | shape/cell, empty, numeric order, effects; no Mean kernel yet |
| Reduction/window/scan | Existing `GraphForm::Reduce/PrefixInfix`, `j_graph_scan.rs` | Register ReductionFastPath/WindowAlgorithm opportunities, separate from Scan witness | small-cell/window algorithm; NaN/overflow, monad vs dyad |
| `i.` / `e.` / `E.` search | Primitive identity, existing FindViaWindowMatch rewrite | Dyadic SearchAlgorithm source opportunity; preserve existing Find rewrite | tolerance, hash/sort applicability, cost |
| Dyadic `I.` interval index | `PrimitiveId::Indices` | Distinct IntervalLookup candidate, not monadic index-space | order/shape/type/tolerance |
| Dyadic From and static reindex | `DynamicGather/StaticReindex` | GatherCopyOrView/ReindexCopyOrView candidates only, **not universal zero-copy** | bounds, alias/ownership, fill, stride/gather |
| GroupReduce, full dot/GEMM, Grade/Ranking | Future J Graph/source-identity support + Execution Semantic Lowering | Source provenance, owner and obligations registered; do not fabricate executable nodes | frontend semantic support and differential proof |
| Tolerant hash, sparse, buffer reuse and name cache | Execution/Physical planner and binding runtime | `DownstreamOnly` or `AwaitingFrontendOrFacts`; never mislabeled graph algebra | tolerance/sparse/alias/locale version, fallback |

- `src/j_graph_jsource.rs` owns pinned source references, stable IDs, owner, coverage and proof obligations. `Plan::jsource_opportunities()` reports source `ValueId`, span, basis and facts without mutating the graph.
- `CompilationAnalysis::jsource_opportunities` is populated in `runtime.rs::analyze_compilation_diagnostic`, alongside existing rewrites and Logical IR; it **does not select/execute** opportunities.
- All opportunities are `AwaitingSemanticProofs`, `selected=false`. `verify(&Plan)` checks derivability/provenance against the graph, **not full numeric equivalence**.
- Existing MapReduce fusion remains solely owned by `j_graph_fusion`; GroupAggregate, MatrixContraction and GradeRanking are registry-only until their constructors and proofs exist.
- `tests/j_graph_jsource.rs` covers stable source registry, derivation/provenance, forged candidates, exact Mean fork and negative matches, selected Graph patterns and isolation from existing Find rewrite/MapReduce fusion. **Test execution has not yet been verified**.

Deferred work: establish correct J constructor/operand/rank semantics for Key, Dot, Grade, Cut and Under; discharge semantic/effect/alias/numeric witnesses; add execution-specific GroupReduce/Contraction/GradeSort/IntervalLookup operations; select guarded CPU/GPU/sparse/BLAS routes only through target/cost planning; and validate against jsource for empty/sparse/tolerance/`!.`/overflow/rank/error/binding cases. Preserve the current frontend milestone priority. The optimization catalog is **not** an executable jsource-optimization port.

### K. Follow-up source audit: monadic Mean guard and runtime-only optimizations (2026-10-06)

**Concrete correction:** In the pinned [`cf.c::jtfolk`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L93-L101), `(+/ % #)` selects **`f1=jtmean` only**, not a dyadic `f2` specialization. Earlier RustJ discovery matched the derived fork but did not check applied valence, so it could falsely emit `MeanIdiom` for dyadic calls. `src/j_graph_jsource.rs` now requires a **monadic applied node**, and `tests/j_graph_jsource.rs` includes a dyadic negative regression. No mean kernel or numeric-equivalence proof is implied.

A second pinned-source pass also sampled previously unreviewed areas:

| Direct source | Observed mechanism | RustJ boundary |
|---|---|---|
| [`p.c` lines 10–24](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c#L10-L24) | Parse rows 0–2 support inplace execution, assignment `zombieval`, early parse completion | Frontend/runtime binding and lifetime, **not** graph-only donation |
| [`cx.c` lines 270–329](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L270-L329) | Reuse/clone explicit local symbol tables, precomputed x/y buckets, borrowed/abandoned argument handling | Explicit runtime/binding; respect dynamic scopes and aliasing |
| [`va1.c` lines 313–383](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/va1.c#L313-L383) | Monadic numeric dispatch, retry/promotion by exceptional condition, distinct sparse fallback | Numeric semantic witnesses then guarded execution |
| [`am.c` lines 55–89](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/am.c#L55-L89), [568–581](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/am.c#L568-L581) | Guarded Amend/scatter inplace paths checking indexing, sparse/type/read-only/alias/usecount | Amend semantics then Scatter/Buffer planning; do not conflate with Gather |
| [`m.c` lines 743–783](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/m.c#L743-L783) | Virtual/recursive storage and allocation/refcount lifetime | Physical allocator/representation; do not copy C flags as graph semantics |

Only the Mean false-positive is changed in graph discovery. Other observations remain source-evidence backlog assigned to their respective stages; they do not claim additional implemented graph transformations or exhaustive jsource coverage. `FOUNDATIONS.ko.md` remains consistent with the semantic/physical separation and needs no change. **Rust tests, differential execution and benchmarks have not been run for this change.**

### L. Connect jsource source opportunities to canonical A3 and existing lowering (2026-10-06)

`LoweringRegistry::jsource_planning_reports` first re-verifies the J Graph opportunity against its source plan. It validates both graph and A3 IR and source/node-count consistency, then maps the source `ValueId` to existing canonical A3 calls via `Operation.j_origin`. For an A3 `Basis` call it reports the *existing ordinary* `legal_candidates` for the selected target; for `SemanticCall` it reports the existing semantic-call boundary. **An ordinary CPU/GPU reference route is not a jsource-specialized implementation or an equivalence proof.**

`JsourcePlanningReport` exposes candidate family, source provenance, decision owner, linked `OpId`s, the **entire unresolved** `ProofRequirement` list, and fail-closed status `NeedsLogicalCallLink` or `NeedsSemanticProof`. Type/shape facts, matching source syntax, and legal baseline routes do not silently discharge proof obligations. This is the first conditional-lowering *gate*, not optimized lowering execution. It does not commit a transform, select jsource-specific kernels, insert runtime guards, or make physical layout decisions.

Next: typed family-specific equivalence witnesses and runtime guards (including effect/error/fallback order), parameterized lowering recipes, target/cost decisions, and differential tests. `tests/lowering.rs` adds cases for Reduce, monadic Mean, Gather, IntervalLookup, CPU/GPU route separation, stale provenance and missing A3 origins. These tests **were added but not executed**; CI, Cargo, C differential and performance validation remain unrun.

<a id="jsource-audit-m"></a>

### M. Re-audit of previously unlisted jsource optimization families (2026-10-06)

**Method and scope.** On pinned [jsource revision 13994ffa](https://github.com/jsoftware/jsource/tree/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc), separately inspected (1) the \`ca.c\` composition and \`cf.c\` hook/fork dispatch tables, (2) implementations and fallback guards in \`ao/cc/v/vi/vg/vrand/vx/vz/va1/vo/a.c\`, and (3) existing RustJ \`j_graph_ir\` / \`j_graph_jsource\` / A3 / lowering boundaries. These paths were missing or overly aggregated in the earlier A/H/K representative catalogs. Source fast paths do **not** establish semantic equivalence, RustJ support or measured gain.

| Newly separated specialization | Verified pinned source and key guard | Placement in current RustJ framework |
|---|---|---|
| **Oblique reduction/convolution** \`f//.@:(g/)\` | [\`ca.c\` dispatch](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ca.c#L367-L374) and [\`ao.c::jtpolymult\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ao.c#L109-L161): atomic dyads with VFUSEDOK2, dense/nonempty vector and dtype/operator guards, generic fallback and overflow retry. | J Graph Oblique/Reduce candidate → typed Contract or segment-reduction/access relation → downstream convolution routine. Not an unconditional GEMM alias. **Not implemented.** |
| **Cut → Scan/Window → Raze fusion** | [\`ca.c\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ca.c#L358-L368), [\`cc.c::jtrazecut2\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cc.c#L983-L1031): restricted Cut modes, atomic scan and dense input; **the upstream comment acknowledges an extra result axis in the no-cut case**. | Multi-region Graph candidate → SegmentView + Scan/Window + ConcatAssemble → materialization/assembly. **Blocked pending zero-cut semantics reconciliation.** |
| **Byte-character substitution LUT** \`y {~ x i. ]\` | [\`cf.c\` fork](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L59-L65), [\`v.c::jtcharmap\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/v.c#L165-L191): LIT bytes, 256-entry table, alphabet special case, first-match precedence and index-error fallback. | IndexOf→Gather idiom → Lookup/Gather → Physical byte LUT with byte/shape/errors guard. **Not implemented.** |
| **Boolean/sparse predicate direct indices** \`# i.@#\` | [\`cf.c::jthkiota\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L221-L234), plus [Key hooks](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L329-L339): dense boolean and guarded sparse boolean nonzero-index path. | IndexSpace + Compact/GroupBy candidate → direct mask/sparse index route. Preserve rank/empty/fill and original value semantics. **Not implemented.** |
| **Grade→scalar Gather order statistics** | [\`cf.c\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L337-L345), [\`vg.c::jtordstat/jtordstati\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vg.c#L779-L800): scalar index, int/float vector and size restrictions. \`jtordstat\` draws **random pivots via \`jtrollksub\`**. | Grade→Gather candidate → guarded order-statistic select; inspect observable RNG state and tie/index behavior. **Not implemented.** |
| **Shape + RNG generation fusion** \`?@#\`, \`?@$\`, \`?.@#\` | [\`ca.c\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ca.c#L353-L359), [\`vrand.c::jtrollksub/jtrollk\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vrand.c#L609-L678): binary, power-of-two and general range RNG routines with rank/type fallback. | Source Generate/shape-demand candidate → **stateful** RNG runtime route; preserve seed/stream/draw/effects; never classify as pure algebraic fusion. **Not implemented.** |
| **Direct boxed Append/Raze link** \`,<\`, \`;<\`, \`,&<\` | [\`cf.c\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L315-L326), [\`vo.c::jtjlink\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vo.c#L107-L144): boxed/virtual/recursive ownership, WILLOPEN and in-place guards, fallback. | Graph producer-consumer demand → ConcatAssemble/result assembly → guarded ownership/materialization. Not universal Box→Open cancellation. **Not implemented.** |
| **Explicit \`M.\` memoization** | [\`a.c::jtmemo12/jtmemo\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/a.c#L93-L185): memoizable scalar integer-like arguments, locked expandable cache of key/result pairs, nonmemoizable input executes normally. | Derived verb identity + stateful cache runtime; preserve explicit memo semantics, call-skipping, bindings and lifetime. Not generic Graph CSE. **Not implemented.** |
| **Numeric constant and exact-result idioms** | [\`ca.c\` dispatch](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ca.c#L367-L387); [\`vx.c::jtdigits10\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vx.c#L257-L290), [\`vz.c::jtexppi\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vz.c#L293-L305), [\`va2.c\` exponent 0.5/power-of-two residue](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/va2.c#L1937-L1940). Complex half-turn has exact-zero component handling. | Graph constant/derived numeric idiom → numeric/fit/error witness → typed backend recipe. No one-to-one C function Graph node. **Not implemented.** |
| **Hook comparison and deadband** | [\`cf.c\` abs/level comparison](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L319-L333), [\`cf.c\` dispatch](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L105-L115), [\`va1.c::jtdeadband\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/va1.c#L440-L462): AVX2 or emulated AVX2, dense floating-point, scalar-threshold and fallback guard. | Graph Hook/comparison candidate → typed numeric comparator → target-feature-gated SIMD. Preserve tolerance, NaN/signed-zero and error ordering. **Not implemented.** |

Do not duplicate H's GroupReduce/GEMM/Grade/Search/virtual/sparse family rules. This table separates **different topology, proof conditions and target/runtime ownership** hidden inside those broad topics. Do not fabricate missing parser constructors or auto-register these ten as executable \`JSOURCE_FAMILY_RULES\`; future registry-only entries should start as \`AwaitingFrontendOrFacts\` or \`DownstreamOnly\`.

#### M.1 Is a RustJ framework change necessary?

**No wholesale architecture or canonical IR replacement is necessary. Several local contract extensions are necessary before these optimizations may execute.**

| Current RustJ structure | Decision and necessary action |
|---|---|
| Shared \`FunctionEntity\`, J Graph regions, canonical A3 and basis vocabulary (Contract, Scan, GroupBy, Grade, Gather, ConcatAssemble, etc.) | **Keep.** Preserve operator topology and existing basis algebra. Fill in currently deferred family-specific \`ExecutionBasisPayload\` only with corresponding semantic support. Do not add C-entrypoint-shaped IR nodes. |
| \`JsourceOpportunity\` stores one source ValueId/span; \`jsource_planning_reports\` links calls by \`Operation.j_origin == source_value\` | **Extend sidecar provenance** with optional region identity, graph-version/source anchor and the exact ordered A3 operation set for multi-region idioms such as Cut→Scan→Raze or Oblique→Reduce. Multiple nested regions can share a result ValueId. Do not change parser-owned semantic identity. |
| \`JsourcePlanningState\` only has NeedsLogicalCallLink/NeedsSemanticProof; \`OpportunityLegality\` only AwaitingSemanticProofs, and family proofs remain unresolved | **Add proof discharge before enabling optimized execution.** Implement §4.1.4's per-obligation evidence/proof/guard/rejection contract; separate provenance, call/rank/empty/fit/numeric/error proof, target/resource/cost and final selection. A legal baseline lowering is not an optimization equivalence witness. |
| A3 \`EffectSummary\` currently only Pure/Unknown; RNG, explicit \`M.\` cache, name bindings and control errors require ordered state relations | **Add scoped state/effect-resource contracts when supported.** Prevent loss of observable RNG consumption or explicit memo behavior; until then retain Unknown/runtime-semantic fallback. No forced immediate conversion of all A3 ops into memory/effect SSA. |
| Logical \`SemanticCheck\` and constraints, physical target/resource/cost layers already separated | **Keep separation.** Handle zero-cut rank/fill, boolean/sparse, byte-LUT index errors, numeric guard, RNG draw ordering before observable effects, with specified source fallback. No replay after effects. Buffer/byte table/SIMD workgroup belongs downstream. |

Two source-level counterexamples block automatic equivalence:

1. [\`cc.c::jtrazecut2\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cc.c#L983-L993) explicitly notes that its no-cut case can return a different result axis than the generic route. A jsource special entry point **is not an equivalence oracle**. Independently settle J result/empty assembly semantics before committing fusion.
2. [\`vg.c::jtordstat\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vg.c#L779-L790) calls RNG to choose pivots. Check whether the RNG state is observable by subsequent J operations before classifying Grade→Select as pure. If observable, require an explicit state/trace policy and its proof.

#### M.2 Staged implementation and validation

1. Keep M2 frontend precedence: verify derived POS, operands, rank/valence, exact source pattern and fallback for \`/. /..\`, \`;.\`, \`M.\`, \`? / ?.\`, hook/fork/grade. Unsupported forms remain unregistered as applied candidates.
2. Preserve source region/operation anchors, effect/RNG/cache/error edges and per-candidate numeric/empty/assembly proof obligations.
3. Implement one narrow, guarded family (for example byte LUT or boolean-index) with existing generic reference fallback; defer no-cut segment fusion, randomized selection and memoization until their extra contracts are proved.
4. Extend \`LoweringRegistry\` with verified parameterized recipes, then target/resource/cost and overlap-aware selection. The physical planner owns buffers, cache/tiling and device realization.
5. Run differential comparisons for positive/negative guards, dtype/rank/boxed/sparse/empty/\`!.\`/tolerance/NaN/overflow/error precedence, RNG seed/trace, dynamic name binding and replay safety before declaring any family implemented.

**Validation limits:** This is a pinned-source plus RustJ design/code **static audit**. No Cargo, CI, J/C differential execution or benchmarks were run. Not an exhaustive audit of every source file, all assembly/architecture microkernels, all build variants or current jsource HEAD; never claim zero omissions or working optimizations based on this section alone.

<a id="jsource-index-family"></a>

### N. Roger Hui's Index-Of family: phased integration in RustJ (2026-10-06)

**Sources.** Roger Hui, *Index-Of, A 30-Year Quest* (J Conference 2014; [bibliographic evidence](https://www.sigapl.org/Articles/APL%20Since%201978_3386319.pdf)) and *Hashing for Tolerant Index-Of* ([Jsoftware, 2010](https://www.jsoftware.com/papers/Hashing.htm)). Actual dispatch, preconditions, fallback and mode ownership were examined in pinned [\`vi.c\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vi.c#L140-L185), [\`viavx.c\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx.c), [\`viavx2.c\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx2.c) and [\`visp.c\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/visp.c). Neither the historic paper nor the existence of a C specialized routine proves a RustJ optimization legal or profitable.

**Separate J operations from search algorithms.** Dyadic \`i.\` (first match), \`i:\` (last match), \`e.\` (membership), \`~.\` (nub), \`~:\` (nub sieve), \`-.\` (less), \`I.@e.\` (matching positions), and Key classification can share a lookup engine, but result representation, rank/cell/frame, duplicate representative, empty/prototype and tolerance differ. Dyadic \`I.\` is **interval lookup**, not ordinary index-of; \`E.\` is a **substring/window match** owned by existing \`FindViaWindowMatch\`; monadic \`i.\`/\`i:\` generate index spaces. Do not fuse these semantic identities merely because their C implementations share a source file.

| Upstream strategy | Guard and cost premise | RustJ ownership |
|---|---|---|
| Sequential scan | Tiny inputs where setup dominates ([\`vi.c\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vi.c#L1113-L1148)) | Execution algorithm/reference |
| Direct indexing, bit-packed byte/small integer range | Integer domain span, presence-vs-position table width, initialization and cache locality ([\`vi.c\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vi.c#L1148-L1238)) | Physical algorithm/cost |
| Hash and reverse hash | Index-vs-query relative cardinality, duplicate order ([\`viavx.c\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx.c#L738-L850)) | Physical algorithm/cost |
| Tolerant float/complex/boxed hashing | Runtime cct/\`!.\`, nontransitive approximate equality, +0/-0, NaN, exact insertion, neighboring intervals ([\`viavx2.c\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx2.c#L8-L78)) | Equality semantics/proof **before** target algorithm |
| Boxed sort→binary search | Source \`jtiobs\` is limited to \`ct=0\` and selected boxed shapes ([\`vi.c\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vi.c#L735-L840)) | Target algorithm after equivalence witness |
| Sparse and Key self-classification | Sparse fill/axes, stable first occurrence ([\`visp.c\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/visp.c#L68-L106), [\`ao.c\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ao.c#L213-L255)) | LookupClassify/GroupBy, representation-specific |
| Prehash reuse / fused result modes | Dictionary key/type/rank/tolerance/version and lifetime; output index/boolean/compact/count/any/all ([\`vi.c\` modes](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vi.c#L140-L185)) | Execution result intent and costed cache planning |

#### N.1 First implemented slice — preserve canonical IR

- \`src/index_ops.rs\`: \`LookupResult::{First,Last,Membership}\` and a shared \`lookup(indexed,queries,result)\`. Membership now **produces boolean results directly**, without a materialized index-position vector, preserving the prior frame/rank and incompatible-cell behavior.
- Exact **integer/boolean scalar-only** inputs may use \`ExactScalarIndex::Direct\` or \`Hashed\`; small queries remain sequential. The direct table has a fixed **65,536-entry** ceiling, provisional \`span <= 4 * (indexed items + queries)\` heuristic, and \`items * queries <= 32\` sequential cutoff. Compute key span via \`i128\` to avoid signed overflow; preserve first/last duplicates and not-found. These numbers are **initial heuristic bounds, not benchmark-tuned cost evidence**.
- Float/boxed/complex or non-scalar cells remain on generic sequential \`atom_eq\` fallback. No tolerance hash is enabled.
- \`src/j_graph_jsource.rs\`: include dyadic \`i:\` in \`SearchAlgorithm\`; remove \`E.\` from this family (existing FindViaWindowMatch rewrite owns it); keep dyadic \`I.\` and monadic \`i.\`/\`i:\` distinct. Graph candidate legality remains unproven/unselected.
- Add index tests for duplicate/negative/missing, boolean membership, bounded direct vs hash, extreme i64 key spans, and provenance separation. **Tests were added, not executed; this is static code integration only.**

This is a narrow improvement to the CPU reference path, **not** full upstream \`i.\` support and **not** Graph-driven fast-path commitment.

#### N.2 Necessary extensions, not an IR redesign

The first N.2 extension has since landed in §O: A3 `LookupClassify { search: SearchDescriptor }` preserves first/last/membership/interval/self-classify, indexed/probe origin and J comparison meaning. **Still missing** are runtime `!.ct` policy/version witnesses, grouped/compact/count search modes, sortedness/uniqueness proofs, full prepared-lookup keys and shared `CandidateEvidence`. Preserve original `FunctionEntity`, dynamic tolerance and Rank/CellApply throughout.

1. Validate duplicate order, rank/cells, mixed types, box/sparse/empty/fit and J-visible error semantics with a C J oracle before broadening algorithm options.
2. Extend direct byte/packed-index tables, reverse hash and explicit prehash in separate increments with target cost/alias/cache/key-invalidation evidence; handle GPU resource constraints separately.
3. Treat tolerant hashing as a dedicated research and equivalence gate: near equality is not generally transitive; matching float hashes and matching equality classes cannot be naively identified. Guard cct, signed zero, NaN, boxed recursion, first/last matches and pre-effect fallback.
4. Feed proven algorithm recipes into \`LoweringRegistry\` with independent target/resource/cost evidence. Do not treat legal ordinary execution routes as equivalence witnesses for newer specializations.
5. Require Rust/C differential plus benchmarks for claims of general legality or speed; CI, Cargo, differential runs and performance measurement were **not performed** in this implementation slice.



#### N.3 Second slice: query-side reverse hashing and per-Engine prehash (2026-10-06)

**Provenance.** [Pinned jsource \`viavx.c\` reverse-hash selection](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx.c#L738-L850) is conditional on indexed/query sizes and supported modes; [\`vi.c\` prehashed modes](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vi.c#L140-L185) include saved derived-verb table semantics. RustJ borrows **only narrowly scoped algorithms** here; this is not a port of all upstream prehashed derived verbs.

| Implementation | Eligibility and behavior | Explicit guardrail |
|---|---|---|
| **Query-side reverse hash** | \`src/index_ops.rs::reverse_exact_index\`: for exact scalar Int/Bool, at least 64 indexed items and indexed/items ratio over 2:1, hash distinct query values then scan the original indexed items once | Forward scan for First/Membership, backward for Last. Stop when all distinct query keys resolve. Duplicate queries share correct positions and absent keys retain the missing sentinel. No tolerance, boxed or multicell hashing |
| **Engine-local prepared search index** | Retain one \`ExactPrehashCache\` per \`Engine\`. Index only shared immutable Int/Bool vectors, length 64–16,384. First and Membership reuse the same prepared index; Last requires a distinct representative policy | A hit requires identical retained \`Arc\` allocation, dtype, shape and policy. Retaining a shared source clone prevents pointer recycling; new backing after rebinding cannot hit a stale index. Single-entry bounded cache, not a global name cache |
| **Interpreter integration** | \`runtime.rs::Engine::interpret_ir\` uses cache-aware exact-scalar \`i.\`, \`i:\`, \`e.\` for the regular pooled primitive path. \`index_prehash_stats()\` and \`clear_index_prehash()\` expose limited diagnostics | Other types, explicit rank and \`eval_semantic_reference\` use the existing route; no additional Graph/A3 source transform is committed |
| **Regression cases** | Unit tests for direct/hash/reverse/linear vs a reference, duplicates, extreme keys, immutable identity and stale-rebind rejection; Engine integration tests for first-to-membership sharing, last-policy separation, temporary reverse and reference bypass | **Added, not executed.** CI/Cargo, C differential and benchmarks were not run |

**Allocation fallback:** `Error::Limit` when allocating an optional reverse/direct/hash/prehash search table is treated as a missed optimization, so the original sequential lookup still executes. A genuine result-buffer allocation failure remains an execution error.

**Architecture decision:** no new canonical Graph IR or A3 basis node. This remains a narrowly guarded **CPU interpreter implementation path**. The fixed thresholds (64 items, 2:1 size ratio, 16,384 cache items) are provisional, **not** measured optimal costs. It does not assert that JsourcePlanningReport has discharged equivalence proofs or that LoweringRegistry selects these implementations.

**Remaining prehash gate:** explicit J derived prehash such as \`m&i.\` or \`e.&n\` requires a compiler-visible prepared-lookup descriptor, versioned dictionary/key equality/tolerance context, cache lifetime and invalidation, fallback/check ordering and target/cost evidence. Do not extend this immutable Arc-identity cache to dynamic name, locale, boxed, sparse or tolerance-aware domains without independent proofs. Run J/C differential and measurements before widening eligibility.


<a id="algorithm-planning-migration"></a>

### O. Algorithm-planning framework extension informed by MLIR, IREE, TVM, XLA and Futhark (2026-10-06)

**Architecture decision:** do not turn jsource's special C entry points into J Graph node kinds or leave their choice exclusively in `index_ops.rs`. Preserve `J semantic identity → A3 meaning → target legality / proof evidence → Physical cost/selection → guarded executor with fallback`. This is the first search-family slice, **not** completion of a whole-compiler optimizer or auto-tuner.

#### O.1 Comparison and adopted boundaries

| Framework / verified reference | Actual mechanism | Adopt / explicitly defer |
|---|---|---|
| **MLIR Dialect Conversion** ([official reference](https://mlir.llvm.org/docs/DialectConversion/)) | ConversionTarget marks Legal/Dynamic/Illegal per operation and may leave unsupported operations in partial conversion | Distinguish baseline legality, runtime guard, semantic proof, target rejection and mismatched J operation in `SearchAlgorithmReadiness`. Do **not** replace J function/locale semantics with dialect legality |
| **MLIR Transform dialect** ([official reference](https://mlir.llvm.org/docs/Dialects/Transform/)) | Transform/control IR acts on separate payload IR and distinguishes recoverable from irrecoverable failures | Leave J Graph/A3 semantics unchanged when reporting candidate/selection; keep guard miss separate from invalid transform. Do not import the dialect itself |
| **IREE Flow/Stream/HAL and Codegen** ([phases](https://github.com/iree-org/iree/blob/main/docs/website/docs/developers/general/developer-tips.md), [LoweringConfig](https://iree.dev/reference/mlir-dialects/IREECodegen/)) | Separate dispatch/stream semantics, backend lowering configs, tiling/vectorization and bufferization | Keep hash size, SIMD, buffers and GPU scheduling out of semantic `SearchDescriptor`; do not claim unsupported GPU search kernels |
| **TVM MetaSchedule** ([official tutorial](https://tvm.apache.org/docs/deep_dive/tensor_ir/tutorials/meta_schedule.html)) | SpaceGenerator, SearchStrategy, CostModel, Builder/Runner and measured tuning database are separate | Separate legal candidates, workload facts, selection and eventual measured feedback. Current bounds are **heuristics, not a tuned cost model or database** |
| **XLA GPU priority fusion** ([design discussion](https://github.com/openxla/xla/discussions/10065), [pass source](https://github.com/openxla/xla/blob/main/xla/backends/gpu/transforms/priority_fusion.h)) | Estimate compute/memory/kernel-launch impact and rank feasible fusion choices by modeled benefit | Cost/profitability does not legalize semantics; defer device cost ranking until the device route exists |
| **Futhark SOAC / incremental flattening** ([2026 design](https://www.futhark-lang.org/blog/2026-07-31-full-flattening.html), [fusion discussion](https://www.futhark-lang.org/blog/2026-03-24-scan-scatter-fusion.html)) | Retain high-level array dataflow and select among sequential/flattened/fused implementations using shape and machine constraints | Retain high-level `LookupClassify` and later GroupBy/Reduce identity, but do not assume J errors, dynamic names and fit semantics satisfy unrestricted functional fusion identities |

#### O.2 Implemented three-way ownership

1. **A3 Execution Semantic Lowering:** `ExecutionBasisPayload::LookupClassify { search: SearchDescriptor }` now retains original primitive/valence-derived `FirstIndex/LastIndex/MembershipMask/IntervalIndex/SelfClassify`, J equality vs ordered interval comparison, indexed/probe SSA `ValueId` and rank-boundary identity. For `i.`/`i:` and dyadic `I.`, the **left operand is indexed and the right queried**; for dyadic `e.`, **the right operand is indexed and the left queried**. A3 retains this semantic direction before any physical strategy; unknown or nonprimitive derived calls remain `Deferred`. `A3_SCHEMA_VERSION` increases **0.4→0.5**; `Plan::verify` rejects a descriptor that differs from the originating `CallOp`. `JEquality` refers to actual J comparison semantics, **not** permission for exact float hashing.
2. **LoweringRegistry:** a reference CPU capability for known pure `LookupClassify` calls, plus `SearchAlgorithm::{Sequential,DirectAddress,IndexedHash,ReverseQueryHash,PreparedHash,TolerantNeighborHash}` reports. `SearchAlgorithmReadiness` differentiates `Baseline`, `RequiresExactScalarGuard`, `NeedsSemanticProof`, `UnsupportedTarget`, `UnsupportedSearchForm`. Interval lookup and unsupported GPU/Tolerant Hash are not enabled by mere registration. The same algorithm report is exposed through `JsourceLinkedCall.search_algorithms`, without changing the `NeedsSemanticProof` or unselected source-candidate state. Existing `JsourcePlanningReport` source proofs remain unresolved and are **not** promoted to executable optimized transformations.
3. **Physical strategy:** `physical.rs::plan_search_algorithm` receives an explicit `SearchWorkload` (indexed/query counts, key span if measured, shared immutable backing, prehash eligibility, available reverse-query values) and target. It queries the registry and returns a guarded or reference `SearchPhysicalChoice` with **estimated temporary table entries, not a byte-accurate resource model or measured timings**. Initial thresholds: sequential ≤32 pair comparisons, direct ≤65,536 entries and ≤4× total work, reverse ≥64 indexed with size ratio over 2:1, prepared 64–16,384 immutable shared entries. `index_ops.rs` checks actual Int/Bool scalar types/cells before following the Physical selector. **Hot-path boundary:** runtime selection uses allocation-free `LoweringRegistry::search_algorithm_readiness`, not a rebuilt registry and candidate vector for each lookup; compiler diagnostics can still request `search_algorithm_reports`. Optional table-allocation failure returns to sequential reference, while real result allocation errors remain observable.

**No semantic shortcut:** approximate tolerance is not generally transitive; `!.ct`, complex/boxed and float values cannot be moved into exact hashing without a separate witness. `I.` interval is not an Index-Of hash family. These changes do not create final GPU codegen, native physical `BufferId` schedules or graph-rewrite commits.

#### O.2a Representative example — `3 1 3 i: 3 4`

- **J Frontend / Graph:** Preserve dyadic `i:` (last-match); 3 resolves to the last occurrence and 4 to the not-found sentinel.
- **A3:** `LookupClassify { search: SearchDescriptor { output: LastIndex, indexed: left SSA ValueId, queried: right SSA ValueId, comparison: JEquality, .. } }` retains meaning without encoding a hash table.
- **Registry:** CPU reports `Sequential=Baseline`, exact-scalar `DirectAddress/IndexedHash/ReverseQueryHash/PreparedHash=RequiresExactScalarGuard`, and `TolerantNeighborHash=NeedsSemanticProof`.
- **Runtime/Physical:** Runtime checks Int/Bool scalar item types. With `3 × 2 <= 32` the candidate is sequential and the expected answer is `2 3`; bigger inputs may use other algorithms while preserving first/last/missing semantics.
- **Verification:** The regression cases were added **but not executed**. This trace is a contract example, not a performance measurement.

#### O.3 Generalization and verification gates

~~~text
A3 SemanticDescriptor (meaning, operands, rank, comparison)
 → AlgorithmCandidateSet (legal target, proof/guard obligations)
 → VerifiedRuntimeFacts or proven static witness (Unknown is not true)
 → CostProfile/ResourceBudget (work, bytes, transfer/occupancy)
 → SelectionPlan (independent of canonical semantic IR)
 → Guarded CommittedLowering (fallback before effects/errors)
~~~

The reusable concept is *algorithm option + proof/guard state + resource/cost profile + fallback*. The common report shape `AlgorithmCandidate<Algorithm, Readiness>` and its search-specific alias `SearchAlgorithmReport` are implemented; actual candidate registries, proof discharge, CostEstimate and SelectionPlan for Reduce/Scan, GroupBy, Grade and Contract are deferred until M2/M3 semantic convergence and an M4 CPU baseline, and should be generalized only after a second independently validated operator family actually needs the same contract: each family needs its own associative/tolerance, representative, ordering and numeric-precision proof rather than reusing search-specific enums. Runtime tolerance/fit policy, versioned prehash keys and nested Rank/CellApply legality also remain to be discharged. Do not select a GPU route until implementation/target hard-resource legality exists.

Added A3 search-mode/origin and forged-payload verifier tests, registry CPU/GPU/proof tests and Physical planner selection/guard tests; existing search/rebinding tests are retained. **These are static repository changes. Cargo tests, CI, jsource/C differential checks and benchmarks were not run; no execution or speed claims are made.**



#### O.4 Index-of family as a framework stress test — four independent audits (2026-10-06)

**Question and finding.** We study jsource's `i.` family not mainly to copy its fast hash routines, but to ask **why one semantic operation permits multiple execution realizations, output demands and index lifetimes, and where RustJ should own each decision**. Existing stage separation is fundamentally sound, but the operational contracts between semantic meaning, candidate evidence, materialization lifetime, cost/selection and an independent execution oracle are incomplete. Do **not** invent a new search IR, create one IR op per `vi.c` mode, or prematurely implement a general optimizer. Close the existing §O.3 / §7.5 architecture incrementally.

**Independent audit A — source semantics first.** The pinned [jsource `vi.c`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vi.c) dispatches `IIDOT` (first), `IICO` (last), `IEPS` (membership), `INUB/INUBSV` (nub/nub-sieve), `IIFBEPS` (`I.@e.`), related aggregation/mask modes, and `IPH...` prehash variants through a related internal engine. **Transferable insight:** indexed domain, comparison policy, source-order representative, output demand and reusable physical index state are separable. **Not transferable as J meaning:** C mode integers and entrypoints, SIMD cases, specific hash layouts or the tolerant masks in `viavx2.c`. Monadic `i.` / `i:` generation, dyadic interval `I.`, window find `E.`, Nub and Key are distinct observable semantic contracts. Reuse a lower-level computation or multi-op region only when equivalence is witnessed.

**Independent audit B — compiler IR first.** `src/logical_ir.rs::SearchDescriptor` already retains input `ValueId` provenance, indexed/query roles, first/last/membership/interval/self-classify output intent, J equality versus ordered interval and rank boundary. `src/j_graph_jsource.rs` discovers opportunities but leaves them unproved and unselected. **Keep these boundaries.** `JEquality` alone does **not** prove `!.t`/global `cct` compatibility, type promotion, complex/boxed/sparse handling, frame/cell/rank/prototype behavior or error ordering. `E.` belongs to its separately modeled window rewrite; a shared C implementation is not an argument to coerce it into `LookupClassify`. `I.@e.` and membership-consumer fusion are **multi-node graph/consumer-demand candidates** with source-region provenance, not new primitive opcodes.

**Independent audit C — physical strategy first.** [BQN's search implementation notes](https://mlochbaum.github.io/BQN/implementation/primitive/search.html) distinguish normal/reverse/selectively initialized table lookup (which side is indexed/traversed), one-shot versus reused tables (build versus lifetime cost), direct-table versus hashing/SIMD (representation/target), and first/last/member outputs (observable result). The current `src/lowering.rs::SearchAlgorithm` flattens **different axes** into `DirectAddress/IndexedHash/ReverseQueryHash/PreparedHash/TolerantNeighborHash`. Retain the bounded CPU implementation, but do not generalize that monolithic search enum across primitives. When justified, describe candidate recipes with orthogonal **(1) traversal/build direction (2) table/key encoding (3) build/reuse/invalidation lifetime (4) output materialization/consumer demand (5) target/resource/cost**. Only register proven combinations, not their entire Cartesian product. Such recipes belong to a later optimizer/Physical plan, **not** the Graph IR or A3 schema.

**Independent audit D — adversarial verification first.** `src/index_ops.rs::lookup` with `cache=None` can still call `optional_exact_scalar_index` → `plan_search_algorithm`. Consequently `Engine::eval_semantic_reference` avoids prehash, but is **not** an independently sequential search oracle. The legacy Rust `near` vs pinned C `TCMPEQ` boundary discrepancy proves a comparator identity cannot be inferred from a nominal tolerance. Non-transitive tolerant equality cannot be collapsed to a single equivalence-class representative; first/last, Nub and grouping have independent order obligations. Name rebinding, backing change, dynamic policy change, target change, empty cells and guard miss all require separately witnessed state invalidation. Use **three independent paths**: actual jsource C oracle, strictly sequential Rust semantic baseline, and selected optimized RustJ route.

**Framework cross-check (comparison, not automatic adoption).** [MLIR Dialect Conversion](https://mlir.llvm.org/docs/DialectConversion/) offers **analysis conversion** that checks potential legalizability without modifying payload IR and **dynamic legality** based on properties of the concrete operation. The [MLIR Transform Dialect](https://mlir.llvm.org/docs/Tutorials/transform/) can keep transform/schedule instructions separate from the payload being transformed. These suggest RustJ's **discovery ≠ commitment**, **runtime/semantic evidence before permission**, and **canonical Graph/A3 ≠ selection plan** separation. MLIR dialect conversion success does not itself prove J `!.t`, late NAME resolution or observable error-order semantics, and this comparison does not authorize adopting MLIR Transform IR as a new required RustJ subsystem now.

**Incremental stage contracts — inputs, preserved facts and forbidden decisions**

| Stage | Minimum contract | Must not decide / near-term action |
|---|---|---|
| M2 parser / FunctionEntity | Primitive and derived identity; valence; source binding; `!.t` and global setting scope | Do not rewrite parse entities into C/CPU special routines; **finish general M2 frontend semantics first** |
| J Graph / Analyzer | Original topology, domain/query roles, producer/consumer fanout, source-region provenance | Do not erase output demand or commit multi-op fusion early |
| Execution semantics / A3 | **Distinct** first/last/member/interval/window/group results; frame/cell/rank/empty; comparator contract; error/effect meaning | No hash/direct/prehash/device as a J equality property or `SearchDescriptor` payload |
| Candidate discovery / proof (§7.5) | `RuleId`, graph version/region, per-obligation `Unknown/Proven/Disproven/Guarded`, exact witness | No source idiom or dtype alone counts as legal; discard stale observations |
| Target, resource and cost | Verified runtime/static dtype, cell size/count/range, hard table-byte bound, build/probe/reuse estimates, memory/transfer, supported target | Do not mislabel estimates as timings or unknown resource as zero/cheap |
| Selection / index-state lifetime | Candidate compatibility, representation/direction, one-shot/persistent backing+policy identity/epoch, invalidation, pre-effect fallback | Do not store cache selection in canonical Graph IR or key prepared state on a J name alone |
| Executor / oracle | Independent sequential baseline, guarded optimized execution and real J C oracle; result/error/effect equivalence | Reference must not silently call Physical planning; no replay after observable effects |

**Smallest justified framework extension and order (acceptance gates, not an immediate coding request):**

1. **M2:** Establish a strictly sequential search reference and actual J C differential for `i.`/family, `!.t`, global CCT, rank/shape/type/empty/errors. Preserve the existing `SearchDescriptor` source-direction verifier. Do not spend the M2 budget writing new hash schemes.
2. **M3:** Extract the *minimum* shared view of §7.5 **source provenance + per-obligation proof/guard + invalidation** into existing search/rewrite/fusion sidecars. Add stale-witness/Unknown-not-legal negative checks one at a time. Do **not** build a speculative all-family registry first.
3. **M4:** Verify the native CPU reference slice independently from existing guarded exact Int/Bool physical choices against the same J C oracle. Keep the current `SearchAlgorithm` enum and `PreparedHash` implementation for now; no unmeasured threshold tuning.
4. **When optimization becomes the task:** Demand a **second independently validated family** (e.g., Reduce/Scan reassociation or GroupBy representative selection) actually requiring the same evidence/lifetime/selection interface before extracting general `CandidateEvidence`, recipe or `SelectionPlan`. Never relabel a search-only enum as a generic operator interface.
5. **Later:** Commit one-shot/reused index, query-side index, output materialization elision, footprint, GPU/external routes only after full J legality, target and hard-resource checks, baseline differential and real measurements.

**Repeated independent adversarial questions:** (A) Does the descriptor still determine J-observable results without C mode labels? (B) Does disabling every optional optimization leave a genuinely separate correct reference? (C) Do comparison-policy/binding/version changes invalidate candidate/cache state? (D) Can CPU and external routes share unchanged canonical Graph/A3 meaning? (E) Can `I.@e.`/Nub/Key/consumer fusion reuse *proof infrastructure* without falsely sharing primitive meaning? **Current status:** (A) partially supported; (B) fails because reference still consults Physical planning; (C) dynamic policy unsupported; (D) designed but not executed across targets; (E) insufficient derived-family semantic coverage. Do not claim generic optimizer completion or search speedups.

**Audit scope:** Read-only code inspection, pinned upstream jsource, BQN and MLIR framework comparison. **No Rust/Cargo tests, integrated J C differential, benchmark or new optimizer API implementation occurred.** This reinforces §P.0's specialization freeze and §P.3's M2-first priority rather than replacing them.




<a id="framework-migration-checklist"></a>

#### O.5 Deferred-optimization framework migration plan and living checklist (2026-10-06)

**Purpose.** Make §O.4's independently reviewed architectural direction an **actionable, evidence-based checklist**. The `i.` family is the first *validation case*, not a template for copying C special cases. The common framework should eventually support **discovery → provenance + semantic proof/guard → hard target/resource feasibility → cost → selection → guarded execution**, without corrupting J meaning. This §O.5 owns the **cross-stage migration gate**; §P.1 continues to track *search algorithms* specifically. Do not introduce a new roadmap file, one Graph opcode per upstream mode, or an upfront all-purpose optimizer crate.

**Present status: plan committed; 0/18 implementation/verification gates accepted.** Existing `SearchDescriptor`, `AlgorithmCandidate<_,_>`, `GraphRewriteCandidate`, `FusionCandidate` and `ExactPrehashCache` are useful foundations, **not** completion of these new gates. General **M2 tokenizer → enqueuer → parser/POS/name/derived-entity semantic convergence** remains the top project task; this plan does not authorize early search-kernel work.

**Checkbox rule.** [ ] = **not accepted**, even if part of the code/test exists. [x] = code, **actually executed verification**, and recorded evidence satisfy the row. Record `commit SHA | exact command | environment/target | passed/failed/ignored | pinned jsource commit and actual binary oracle scope | remaining gaps` in that row when completed. Historical tests, static review, and comparing two Rust routes using the same planner do **not** count. Do not claim CI if not executed.

**Dependency gates:** A (**M2 reference and semantics**) → B (**M3 proof/guard evidence**) → C (**M4 independently validated CPU slice**) → D (**later measured Physical generalization**). A gate validates only explicitly supported J forms; leave unimplemented full-J cases `Unknown/Unsupported`. Start with outstanding **general M2 frontend** work before search reference fixes. Prefer **one semantic change and one corresponding regression/negative test** per increment.

| ID / phase | Checklist item | Files / implementation scope | Acceptance and evidence; current status |
|---|---|---|---|
| FW-01 / A·M2 | [ ] Resolve frontend semantic scope | `src/tokenizer.rs`, `src/enqueuer.rs`, `src/parser.rs`, `src/semantic.rs`; classify word/POS/name/derived-entity gaps | Maintain general M2 priority; compare supported parse/resolution with actual J C oracle, record unsupported separately. **Not executed** |
| FW-02 / A·M2 | [ ] Isolate true sequential semantic reference | `src/runtime.rs`, `src/kernels.rs`, `src/index_ops.rs`: split `pooled: bool` buffer ownership from optional search Physical policy; route supported `i.`/`i:`/`e.`, including rank/cell/derived calls, to a sequential reference | Instrument/negative-test **zero** `plan_search_algorithm` and prehash calls in `eval_semantic_reference`; no global mutable mode, new public API or canonical IR rewrite required. **Not executed** |
| FW-03 / A·M2 | [ ] Verify real J equality policy | `src/comparison_policy.rs`, `src/kernels.rs`, semantic contracts: fixed Rust `near` vs pinned J C `TCMPEQ`, global `cct`, `!.t` scope, exceptional values and errors | Run pinned **real C binary** on boundary cases; unsupported dynamic policy stays `Unknown/Unsupported` and cannot enable tolerant hash. **Not executed** |
| FW-04 / A·M2 | [ ] Establish three-way search regression fixtures | `src/index_ops.rs` and existing differential tests: first/last/member, duplicates, empty cells/frames, types, errors, rank | Independently compare **J C / strict sequential Rust / optimized Rust** and classify differences. Two routes sharing the same planner are not independent. **Not executed** **2026-10-06 partial evidence:** [4×20 C/Rust Rank gate](https://github.com/yunskim/RustJ/actions/runs/37449102885) all 80/80 matched; [Basis probe](https://github.com/yunskim/RustJ/actions/runs/37449102980) passed. Typed dense zero-frame Rank is implemented. User-defined effects, boxed/sparse fillers and error fallback remain unresolved; gate [ ] remains. **Follow-up unit gates:** [P.11 RK-01–RK-12](#rank-cellapply-followups) tracks evidence and state; parent remains [ ]. |
| FW-05 / B·M3 | [ ] Witness source and rule identity/version | `src/j_graph_ir.rs`, `src/j_graph_rewrite.rs`, `src/j_graph_fusion.rs`, `src/j_graph_jsource.rs`: bind existing origin/span/basis to graph/schema/rule identity or reliable revalidation | Verifier negative tests reject forged/stale graph, changed rule and invalid origin; span-only comparison is insufficient. **Not executed** |
| FW-06 / B·M3 | [ ] Record proof state per obligation | Expose minimal §7.5 evidence **sidecar/view or adapter** for existing search/rewrite/fusion: `Unknown / Proven(witness) / Guarded(guard,fallback) / Disproven` plus source provenance | No single selected bool, search enum or giant common IR as proof. Negative-test Unknown→legal/selected rejection. **Not executed** |
| FW-07 / B·M3 | [ ] Prove guard and effect ordering | `src/lowering.rs`, effect/error verifier, executor adapters: guards before observable effects and explicit baseline fallback | Reject duplicate effects, post-effect replay, unknown fallback, or Guarded-without-actual-guard. **Not executed** |
| FW-08 / B·M3 | [ ] Validate dynamic comparison/name/rank witnesses | Semantic contract surrounding `src/logical_ir.rs::SearchDescriptor`, `comparison_policy.rs`: `!.t`/CCT, late NAME binding, rank/cell/empty obligations | A `JEquality` tag or dtype alone cannot authorize exact hashing; unknown policy/rank forces reference/unsupported. **Not executed** |
| FW-09 / B·M3 | [ ] Project one legality view onto three candidate kinds | Preserve native shapes of `j_graph_rewrite.rs`, `j_graph_fusion.rs`, `j_graph_jsource.rs`; expose provenance/obligations through `fusion_planning.rs` as needed | Reject incompatible overlap, borrowing `E.` window proof for different search semantics, and unknown resource silently accepted. **Not executed** |
| FW-10 / B·M3 | [ ] Cross-validate a second operator family | Select one existing Reduce/Scan **or** GroupBy semantic candidate; exercise FW-05–09 proof/guard machinery | Preserve family-specific reassociation/representative/order proofs. Extract actual common implementation **only after** a second family needs it; no generalized registry upfront. **Not executed** |
| FW-11 / C·M4 | [ ] Validate native CPU semantic vertical slice | Complete a supported J Graph → verified A3 → baseline CPU execution path with source, error and effect ordering | Record actual C-binary oracle commands and compared coverage. **Not executed** |
| FW-12 / C·M4 | [ ] Independently validate existing guarded Int/Bool search | `src/index_ops.rs`, `src/physical.rs`: Sequential, Direct, Indexed, Reverse, Prepared; first/last/member, allocation-failure fallback | Three-way C/independent sequential/optimized comparison; log actual Rust default and portable tests separately. **Not executed** |
| FW-13 / C·M4 | [ ] Separate target, hard resource and cost status | `src/lowering.rs`, `src/physical.rs`, `src/j_graph_resource.rs`: runtime/static proof, table **byte** bounds, estimated work/allocations, capability, measured performance | Unknown hard limit is not zero; estimate does not equal measured cost; unbenchmarked thresholds remain heuristics. **Not executed** |
| FW-14 / D·later | [ ] Evaluate physical recipe axes | Only after two-family evidence and measurements, consider splitting `SearchAlgorithm` / `SearchWorkload/Choice` into build/traversal side, representation, lifetime, output demand and target | Require **two demonstrated shared use cases**, migration and semantic-equivalence tests. Never insert physical choices into Graph/A3. **Not executed** |
| FW-15 / D·later | [ ] Generalize prepared-state lifetime/invalidation | Compare `src/index_ops.rs::ExactPrehashCache` with a second family: immutable backing identity, first/last representative, relevant policy/version, binding epoch, target, retention | Do not put irrelevant CCT into present exact Int/Bool keys; require source rebinding/cache miss/policy-change negative tests. **Not executed** |
| FW-16 / D·later | [ ] Add a distinct SelectionPlan and compatibility check | When §7.5 proof, target, resources and cost have been evidenced, check candidate overlap/compatibility and separate plan identity from committed lowering | Source Graph/SSA and provenance remain unchanged; reject illegal choice, incompatible overlap, stale commit. **Not executed** |
| FW-17 / D·later | [ ] Gate target and algorithm tuning on measurements | Evaluate jsource/BQN small/SIMD/reverse/prehash variants and CPU/external routes only after baseline measurements; GPU remains deferred | Enable candidates individually only with semantic proof, hard memory bound, measured benefit, portable fallback and negative checks. **Not executed** |
| FW-18 / overall | [ ] Repeat independent audits and record regression evidence | Independently re-audit A) J meaning, B) Graph/A3 invariance, C) proof/invalidation, D) CPU/target/resources, E) C/strict sequential/optimized results and cost; tie to §P.2 | Reopen failed prior gates; retain actual pass/fail/ignored, environment, pinned upstream commit and benchmark details; reconcile §P.1 before acceptance. **Not executed** |

**Stop rules.** Do not accept FW-12 before a truly separate reference exists (FW-02). Do not change `TolerantNeighborHash` from `NeedsSemanticProof` until dynamic equality and its guards are validated (FW-03/08). FW-05–09 cannot count as a second operator family (FW-10). Do not start physical strategy refactoring, tuning or target expansion (FW-14–17) before FW-11–13 plus hard-resource and performance evidence. Correctness and semantic bug fixes may proceed at any time.

**Evidence-log format:** `FW-ID | code commit | command/environment | passed/failed/ignored | pinned J C revision + actual oracle scope | unsupported/known gaps | next gate`. Adding this documentation **does not** mean Rust/Cargo, C binary differential, CI or benchmark execution occurred. Update this table and its canonical Korean counterpart rather than a separate daily-status file.



<a id="index-family-roadmap"></a>

### P. Living Index-Of family plan and acceptance checklist — Roger Hui × Marshall Lochbaum (2026-10-06)

**Cross-reference:** [§O.5 framework migration checklist](#framework-migration-checklist) owns cross-stage architecture acceptance; §P.1 continues to track per-search algorithm tests. A code-presence check in §P.1 never counts as an implementation/verification acceptance mark in §O.5.

**Maintenance rule.** This is the authoritative **living checklist** under [§N](#jsource-index-family), [§O](#algorithm-planning-migration) and candidate lifecycle §7.5; do not create another tracking document. A checked box means code/document presence has been **statically verified**, **not** that tests passed or semantics/performance were validated. Record specific proof/commands/results when checking completion gates. **No Linux-specific milestone is required or created.**

**Independently researched inputs:** Roger Hui, [*Index-Of, a 30-Year Quest*](https://www.jsoftware.com/papers/indexof/indexof.htm) and [*Hashing for Tolerant Index-Of*](https://www.jsoftware.com/papers/Hashing.htm), grounded against pinned [jsource vi.c](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vi.c#L140-L185), [viavx.c](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx.c#L738-L850), [viavx2.c](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx2.c#L8-L98); Marshall Lochbaum, [*BQN: Implementation of search functions*](https://mlochbaum.github.io/BQN/implementation/primitive/search.html) for small-input SIMD, direct 1/2-byte tables, sparse **table initialization**, one-shot reverse hashing, cache-sensitive hashing, collision monitoring and radix partitioning. **BQN sparse lookup means selective initialization of a dense-address lookup table; it does not mean J axis-sparse array semantics.** BQN is a physical algorithm reference, never a J semantic oracle. §O's MLIR/IREE/TVM/XLA comparison supplies independent legality/target/resource/cost ownership rules.

#### P.0 Stage-aligned deferred optimization policy and M2 priority audit (2026-10-06)

**Finding: the boundaries mostly fit RustJ's design, but the work sequence and early execution specialization require correction.** The canonical project priority is **M2 word formation → enqueue → J parser/name/POS/derived-entity semantic convergence**, then M3 boundaries and an M4 native-CPU vertical slice. `FOUNDATIONS.ko.md` Part XX requires preserving high-level rank/train/reduce/scan and source provenance while separating logical legality from target-specific scheduling, profitability and materialization. **Delayed optimization** does not mean postponing semantic analysis or candidate discovery. It means **postponing commitment to a special executable algorithm until the semantic, target, resource and cost evidence exists.**

| Work area | Audit | Stage gate |
|---|---|---|
| J primitive/valence/rank/search semantics; `!.t` and global `cct`, error precedence, first/last, nontransitivity and source-macro counterexamples | **Current semantics work** | Resolve via M2 reference/differential tests and M3 semantic contracts; jsource is an **oracle**, not an architecture to replicate |
| J Graph topology, A3 `SearchDescriptor`, SSA provenance, `LookupClassify` | **Keep** | Record comparison, rank/frame, input direction, result intent and unknown facts; no physical buckets/hash/SIMD in canonical meaning |
| `JsourceOpportunity`, `SearchAlgorithmReadiness`, `ComparisonPolicySnapshot` | **Keep only as inert evidence/semantic seams** | Opportunities remain unselected; tolerant route stays `NeedsSemanticProof`. Fixed Rust snapshot does not constitute dynamic J `cct` support |
| Existing runtime CPU Int/Bool Direct/Hash/Reverse/Prehash | **Limited interpreter optimization; freeze expansion** | Retain only under exact scalar runtime guards and sequential fallback. This is not a verified general PhysicalPlan. Prioritize reverting to reference if a semantic regression appears |
| Research-only tolerant exponent buckets and BQN SIMD/small-range tables, open addressing, radix/partition, tuning or production tolerant hashing | **Deferred / frozen** | Reassess after M2 semantics, M3 verification, M4 reference CPU slice, J differential and cost measurements. Keep research only under `#[cfg(test)]` |

**Immediate work:** Compare real J C results with Rust reference for `=`, `i.`, `i:`, `e.`, `E.` across default/changed tolerance, rank/cells/frames, boxed/sparse/empty and error order. Record the observed `TCMPEQ` boundary mismatch as a **semantic parity obligation**, not a mandate to port `viavx2.c`. Do not let this research displace the general M2 frontend convergence tasks.

**Resume gate:** Only reconsider executable specializations after validated reference semantics, independently sourced differential evidence, runtime guard/fallback, an M4 CPU baseline, and hard-resource/profitability evidence. P.1 stages 8–11 are **removed from the near-term work queue**; unfinished 5–7/12 remain unchecked. Never introduce a new RustJ IR node or executor path for each jsource C special case.

**Reference-path independence gap discovered on review:** `eval_semantic_reference` disables pooling/cache, not the ordinary exact-scalar Physical search planner inside `index_ops::lookup`. The semantic reference is not yet an independent sequential oracle. See revised P.3.

#### P.1 Phased checklist and gates

| Stage | Checklist item | Owner and acceptance gate | Status |
|---|---|---|---|
| 0 | [x] Distinguish search identities | Graph/frontend distinguishes dyadic `i.`, `i:`, `e.`, monadic generators, interval `I.`, window `E.` | Source/§N presence only |
| 1 | [x] Keep search semantic descriptor in A3 | First/last/membership/interval/self, J comparison, index/query direction, SSA origins and Rank remain independent of table layout; schema 0.5/verifier | Code/tests added, not run |
| 2 | [x] Separate registered algorithm legality and Physical selection | Registry Baseline/Guard/NeedsProof/Unsupported and SearchWorkload/PhysicalChoice; Unknown never implies legal | Code exists, no benchmark |
| 3 | [x] Guarded basic search implementations | Linear, narrow integer Direct, Hash, reverse query hashing, immutable-Arc per-Engine Prehash, direct membership outputs, allocation-failure fallback | Code exists; C conformance pending |
| 4 | [x] Research-only nontransitivity/completeness harness **added and wired** | `src/tolerant_search.rs` under `#[cfg(test)]`: near chains, source-first/last, ±1 exponent buckets, ±0, NaN, infinities and subnormals, independently scanned indices | **Not executed; no optimized runtime path** |
| 5 | [ ] Resolve full J comparison policy | `!.ct`/dynamic cct/version, float/complex/boxed/axis-sparse, exact insert vs tolerant probe, Rank/cell/frame and effect/error precedence with provenance | No discharged proofs |
| 6 | [ ] Prove candidate-filter completeness | Present fixed `kernels::near` uses t=2^-44. Prove adjacent exponent/sign buckets include every possible match, then recheck all candidates using original comparison and choose min/max original index | Fixed-predicate research only |
| 7 | [ ] Implement versioned tolerant runtime guard/fallback | Missing tolerance/rank/binding witness → pre-effect sequential reference; prehash key/invalidation includes comparison policy and backing lifetime | Not implemented |
| 8 | [ ] Evaluate BQN small-array / small-range strategies | SIMD vector search, byte/2-byte direct, packed presence, sparse table initialization; target/memory/source-order guards | Unmeasured |
| 9 | [ ] Evaluate collisions and large-input fallbacks | Alternative open addressing/linear probing (do not replace current HashMap on faith), collision counters, sorted/radix fallback, cache partitioning | Unimplemented |
| 10 | [ ] Calibrate one-shot/prehash cost | Indexed/query ratio, distinct key count, initialization/retention, cache residency, repeated-use vs one-shot cost; ResourceEstimate != CostEstimate | Only provisional heuristics |
| 11 | [ ] Share algorithm candidate infrastructure | Expand Nub/Key/filtered index outputs then use **family-specific** witnesses for Reduce/GroupBy/Grade/Contract. Do not reuse search-only semantics | Partial generic interface |
| 12 | [ ] Final runtime/differential and benchmark gate | Rust default/portable, J C first/last/NaN/±0/empty/Rank/Boxed/Sparse/`!.ct`/error, independent reference vs fast paths, randomized/adversarial time & memory measures | **Not executed** |

#### P.2 Five independent repeated reviews

- [ ] **A — upstream-first:** separate Hui, pinned jsource dispatch/tolerant implementation and actual J errors/Rank/Fit; do not substitute BQN semantics.
- [ ] **B — proof-first:** independently show nontransitive approximate equality, sign/exponent bucket completeness under explicit \(0 ≤ t < 1/2\), IEEE-754 edges and first/last **source** index; never prove from the candidate implementation alone.
- [ ] **C — oracle-first:** compare J C reference, Rust sequential `near`, and candidate filtering separately. Matching the Rust oracle does **not** prove agreement with J C.
- [ ] **D — boundary-first:** recheck Graph provenance, A3 direction/schema verifier, Registry target/guard/proof, Physical resource/cost and runtime fallback; no GPU/interval/tolerant route enabled on unknown proofs.
- [ ] **E — performance/adversarial:** test repeated keys, pathological collisions, tolerant chains, nonmatches, small/wide ranges, cache pressure and allocation failure. Do not enable algorithms without measured benefit and acceptable worst cases.

After fixing a finding, independently re-run the relevant checks and record commands, counts, exact upstream revision and measurements before checking a gate. No Linux milestone. Never mark CI/tests passed without actual execution.

#### P.3 Next actions — reordered for M2 semantics (2026-10-06)

1. **Finish M2 first.** Converge J word formation, enqueue, parser, name/POS resolution and derived-entity semantics, with actual regression execution. Search-optimization research must not displace the general frontend milestones.
2. **Restore an independent reference interpreter path.** `eval_semantic_reference(...)` disables pooling, but `kernels::dyad("i."/"i:"/"e.")` calls `index_ops::lookup(..., None)`, which **still invokes `optional_exact_scalar_index` and `plan_search_algorithm`**. Thus the current semantic reference avoids prehash yet can still select Direct/IndexedHash/ReverseQueryHash. Separate strictly sequential, specification-oriented search from optional Physical search decisions and test both paths against the same J inputs. **No implementation change or passing test is claimed here.**
3. **Verify J semantics independently.** Compare actual J C oracle results for `=`, `i.`, `i:`, `e.`, `E.`, `!.ct`, `9!:18/9!:19`, default-CCT macro boundaries, rank/cell/frame/empty, boxed/sparse and error precedence. This is a semantic-parity obligation, not a reason to port jsource's specialized hash implementations.
4. **Retain Graph/A3/Registry evidence; defer Physical commitment.** Preserve source topology, comparison intent, provenance and unknown proof facts. Freeze expansion of existing Int/Bool runtime specializations. Reconsider BQN SIMD/small-range direct tables/radix/collision strategies, tolerance buckets and heuristic tuning only after M2/M3 semantics, M4 baseline CPU reference validation, differential evidence and measurements. `TolerantNeighborHash` remains `NeedsSemanticProof` and unselected.

**Independent-verification rule:** Matching an optimized path against a so-called semantic reference that shares its Physical planner is not a valid independent correctness gate. Require three-way comparison against a truly sequential semantic baseline and actual J C oracle, in addition to error/effect/fallback and resource/cost checks.

#### P.4 Independent review record #1 — restricted float candidate completeness (2026-10-06)

**Math-first, independent of the implementation:** Current RustJ `kernels::near` is `a == b || finite(a,b) && |a-b| <= t * max(|a|,|b|)` with `t=2^-44`. If finite nonzero a,b match, their signs cannot differ: opposite signs make the absolute difference at least the larger magnitude, violating `t<1`. Writing `M=max(|a|,|b|)`, `m=min(|a|,|b|)`, successful comparison implies `m >= (1-t)M > M/2` (`t<1/2`), hence their base-two exponent floors differ by at most one. Therefore **same-sign exponent e-1/e/e+1 buckets form a complete candidate superset** for this limited predicate. Retain *every source index*, validate each returned candidate with the original `near` predicate, and choose the minimum/maximum original position for `i.`/`i:`. Never collapse tolerant chains into equivalence classes. +0/-0 and same-sign infinities use exact equality special cases; NaN never matches.

**Independent numeric falsification attempt, NOT a Rust/C run:** An IEEE-754 JavaScript Number model compared **215 values yielding 1,160 near pairs**, including 132 cross-exponent near pairs and **zero** violations of same-sign/adjacent-exponent candidate coverage. Approximate equality on `(1, 1+0.75t, 1+1.5t)` produced `true,true,false`, confirming nontransitivity. Sampling cannot prove J semantic parity, Rust correctness or performance, and this argument does not authorize dynamic `!.ct`, boxed/complex or Rank-aware matching.

**Source/boundary cross-check:** Lochbaum's BQN work informs tiny-input SIMD, sparse **table** initialization, reverse hashing and large-array partitioning, not J semantics. Pinned jsource `viavx2.c` instead uses adjacent tolerance-aware masked intervals with exact insertion and tolerant probe. The research-only **sign/exponent 3-bucket** approach must **not** be represented as the exact jsource bitmask algorithm. Static provenance/guard/architecture/checklist review passed **13/13 checks**, and `src/tolerant_search.rs` is reachable only under `#[cfg(test)]`, not production runtime. **No Cargo, CI, J C differential or benchmark execution took place; all release/semantic execution gates stay unchecked.**


#### P.5 Independent audit record #2 — IEEE-754 neighbor words and ordered representatives (2026-10-06)

**Scope of the change.** Added the research-only Rust test `fixed_near_candidate_filter_covers_ieee_neighbor_words_and_first_last` to `src/tolerant_search.rs`. It generates representable neighbors on both sides of normal/subnormal, powers of two and maximum-finite boundaries; signed zero, both infinities and NaNs; and 2,048 deterministically generated random `f64` bit patterns. For every sampled query it compares candidates with the **full original linear `kernels::near` scan**, checks candidate positions remain distinct and source-ordered, and checks the First/Last answers against the independent linear position oracle. The module remains `#[cfg(test)]` only, with no executable fast path.

**Separate-language numerical falsification attempt.** An independently written Python binary64 model checked **4,224 indexed values × 310 queries = 1,309,440 pairs**; **674** pairs matched `near`, including **143 cross-bucket matches**. There were **zero missing matched positions**, and candidate-based First/Last agreed with linear reference for all 310 queries. The nontransitive chain again evaluated `true,true,false`. These are **finite Python-model results**, not a Rust/Cargo execution, a J C oracle, an exhaustive bit-pattern proof or a speedup measurement.

**Independent proof and safety boundary.** For the present fixed `t=2^-44`, successful finite, nonzero `near` comparison requires matching signs and `m > M/2` for `m=min(|a|,|b|)`, `M=max(|a|,|b|)`; therefore only the same-sign exponent bucket and its immediate neighbors can contain matches. Filtering is necessary-only: preserve *all* original source positions, recheck using the original floating `near` predicate, and select min/max original position rather than coalescing approximate-equivalent keys. An underflowed `t*M` narrows the accepted comparison window; signed zeros and same-sign infinities are handled by the exact `a==b` branch. This is a fixed-predicate proof sketch, **not** evidence for dynamic `!.ct`, arbitrary J comparison tolerance, complex/boxed/sparse comparisons, Rank, or an indexed-key cache.

**Verification ledger.** No available Rust/Cargo executable was found, so `cargo test tolerant_search`, `cargo test --features portable tolerant_search`, `cargo fmt --check`, Clippy, C differential and benchmarks are all **not run**. Do not mark P.1 stages 5–7/12 or P.2 C/E completed. The next independent gate is actual jsource C-versus-Rust conformance under `!.ct`, rank/cell/empty, float/complex/boxed and comparison-policy/version changes, followed by pre-effect runtime guard/fallback evidence. `TolerantNeighborHash` still has `NeedsSemanticProof`; neither Physical selection nor interpreter execution is enabled.


#### P.6 Upstream J comparison policy and proof boundary — source-first audit (2026-10-06)

**Normative J meaning.** The official J Dictionary [Equal (=)](https://www.jsoftware.com/help/dictionary/d000.htm) states that finite float/complex comparisons use default tolerance `2^-44` and may be changed by `!.t`. [Fit (!.)](https://www.jsoftware.com/docs/help806/dictionary/d411.htm) includes the search family `i.`, `i:`, `e.` and `E.`; [global parameters 9!:18 and 9!:19](https://www.jsoftware.com/help/dictionary/dx009.htm) query and change tolerance. A default numeric constant matching the current Rust `kernels::near` is **not proof of identical effective comparison policy at the call boundary**.

**Pinned upstream implementation evidence.** [jsource `viavx2.c`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx2.c) uses `TFINDXYT` to probe the main and adjacent tolerant intervals and `PUSHCCT(1.0)` in `IOFT` to build the table with **intolerant equality** before tolerant probes. `IIDOT` and `IICO` select distinct minimum/maximum source representatives. This is evidence that candidate completeness and ordering matter; it does **not** establish that RustJ's experimental three exponent buckets implement jsource's masked intervals. Collapsing an approximately equal chain would lose the only match in `[a,b] i. c` for `a≈b`, `b≈c`, `a≉c`; a test-only negative regression now fixes this design obligation.

**Proposed `ComparisonPolicy` evidence (design only; not implemented):** retain the resolved primitive/valence and any `!.t` override; the active `9!:19` tolerance plus policy lifetime/version; dtype, rank/cell/frame and boxed/sparse structure; precise comparator identity including NaN/signed-zero/infinity/complex rules; tolerance range witness; indexed source identity, original positions and First/Last/Presence intent; observable error/effect boundary and sequential fallback. Changing policy, source backing or table lifetime must invalidate any prepared tolerant index. A3 `SearchDescriptor` carries semantic provenance, not table representation, which remains Physical-owned.

**Promotion remains blocked.** Even a CPU scalar-float implementation requires an effective-policy witness/guard, complete candidate superset proof, per-candidate recheck with *the same* semantic comparator, stable min/max original index and no-match sentinel, and guard-miss fallback **before effects**. The support boundary must explicitly handle or reject NaNs, infinities, underflow, mixed dtypes, rank/cells, boxed/sparse and reference mismatch. Current proof covers only RustJ's fixed `near`, not full J or dynamic fit. Without C differential and policy evidence, retain `TolerantNeighborHash = NeedsSemanticProof` and no runtime wiring.


#### P.7 Fixed comparison-policy snapshot implementation — dynamic J CCT remains unsupported (2026-10-06)

**Implemented:** A crate-internal `ComparisonPolicySnapshot` in `src/comparison_policy.rs` currently has exactly one constructible identity, `FixedRustNearV0`. It owns the *unchanged* CPU predicate `a==b || finite(a,b) && |a-b| <= 2^-44 max(|a|,|b|)`; `kernels::near` now delegates to this snapshot. `src/index_ops.rs::lookup` captures one policy per search call for `i.`/`i:`/`e.`, while `find` captures one for `E.`; each individual atom comparison uses that shared snapshot. The `atom_eq` adapter used from `expansion.rs` remains present. This does not change J Graph IR, the A3 `SearchDescriptor`, or Physical search selection. Float lookups remain **sequential**, and Int/Bool exact-only prehash remains independent of tolerant comparison.

**Regression coverage authored, execution unverified:** New internal policy tests cover the nontransitive near chain, ±0, NaN, ±infinity, and the minimum subnormal against zero. The `fixed_float_policy_preserves_first_last_membership_and_find` test in `src/index_ops.rs` covers duplicate positions, nontransitive near, the missing sentinel, and all four search outputs. The experimental tolerant candidate harness stays test-only and separate. **Cargo fmt/test/clippy, live jsource, C-vs-Rust differential and benchmarks have not run.**

**Semantic boundary:** This is the first explicit capture of an *implemented* comparator identity, **not** an implementation of J `!.t` or global `9!:19`. The identity `FixedRustNearV0` does not witness equality with upstream jsource's runtime `cct` implementation. Runtime promotion requires fit-derived-verb semantics, global tolerance ownership and call-time policy generation, a supported-range proof and guards, rank/cell/type/error fidelity, cache identity and invalidation, and independent C comparison. TolerantNeighborHash remains unselected; float prehash and GPU/LLVM routes are not activated. P.1 gates 5–7/12 stay **unchecked**.


#### P.8 Upstream default CCT boundary divergence — unresolved semantic difference (2026-10-06)

**Independent source-first audit.** The pinned [`jsrc/vcomp.h::TCMPEQ`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vcomp.h), [`jsrc/i.c` initialization](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/i.c) and [`jsrc/viavx.h::jeqd`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx.h) show default `jt->cct=1.0-FUZZ`, `FUZZ=2^-44`, and the binary64 comparator `(a > cct*b) != (b <= cct*a)`. Tolerant float lookup probes call this macro via `jeqd`. The [`jsrc/xa.c` `9!:19` setter](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/xa.c) checks `0 <= t <= 5.820766091e-11` and assigns `cct=1.0-t`; a future dynamic-policy implementation must preserve its lifetime and limits.

**Concrete binary64 source-macro counterexample, not a native J C run.** With `t=2^-44`, `a=1.0` and `b=1.0-t`, current RustJ `near` reports **true** because it uses inclusive `|a-b| <= t*max(...)`; the pinned C macro *modeled with IEEE-754 double operations* reports **false**. The same discrepancy arises for `b=1.0+t`: at the threshold, different arithmetic and strict comparisons matter. A separate intentionally boundary-focused Python probe compared **120,000 pairs** and found **19,968 model disagreements**. This does not estimate general-input mismatch frequency, verify an actual jsource executable, or prove every target/subnormal behavior.

**Independent compiled C macro reproduction (NOT a full J execution).** A small C translation of the pinned `TCMPEQ` expression was built and run with Debian GCC **14.2.0**, once each using `cc -std=c11 -O0` and `-O2`. In both builds, the legacy RustJ *equation model* returned **true** and the upstream C macro expression returned **false** for `(a,b)=(1,1-2^-44)` and `(1,1+2^-44)`. Signed-zero, infinity and NaN controls matched across both builds. Inputs, environment, outputs and limitations are recorded in [`reports/cct-macro-boundary-probe.json`](reports/cct-macro-boundary-probe.json). **Neither Rust itself nor the full jsource engine was compiled or executed** (no `cargo`, `rustc` or J interpreter was available). A reproduced source macro is not an integrated J semantic oracle.

**Code and release gate.** A test-only `source_cct_macro_model` and `jsource_cct_macro_model_exposes_fixed_near_boundary_gap` in `src/comparison_policy.rs` now capture the unresolved counterexample. Production `FixedRustNearV0` behavior is deliberately unchanged. Do not switch the global comparator or enable float hashing on the strength of a source-level emulation alone: first verify real J C output for boundary `=`, `i.`, `i:`, `e.`, `E.` and collect type/rank/fit/error differences. P.1 stages 5 and 12 remain unchecked. P.4's candidate-bound proof continues to apply only to RustJ's **legacy fixed `near`**, not automatically to upstream J.



### P.9 Pinned default J CCT semantic cutover — FW-03 partial evidence (2026-10-06)

The preceding P.8 observations describe the **historical legacy implementation**. RustJ now executes pinned default J CCT equality, `(a > cct*b) != (b <= cct*a)` with `cct=1-2^-44`, in `src/comparison_policy.rs`, `kernels.rs`, `index_ops.rs` and `search_reference.rs`. Legacy fixed-near is retained as a test-only regression witness, not the active CPU rule. [Linux CI 37445891526](https://github.com/yunskim/RustJ/actions/runs/37445891526) ran the pinned J C `j64`/`j64avx2` oracle against independent Rust sequential and optimized routes in default/portable builds: **48/48 per configuration, 192/192 aggregate, zero route mismatches**. Dynamic `9!:19`, `!.t`, boxed/sparse, broader Rank/error semantics and tolerant hashing approval are not implied. FW-03 remains unchecked.

### P.10 Rank zero-frame fill-cell execution and FW-04 partial acceptance evidence (2026-10-06)

Pinned `jsrc/cr.c::jtrank1ex/jtrank2ex` provides type-correct fill cells when the result frame is empty, executes the underlying operation once to determine output cell type/shape, and assembles an empty result. RustJ previously returned `Unsupported` for `(i.0 3) (i."1 1) (i.0 3)`. The generic dense runtime now uses `src/value.rs::rank_fill_cell/empty_rank_result`, `src/logical_executor.rs::apply_ranked`, and `src/kernels.rs` monadic/dyadic Rank paths. It supports typed dense Bool/Int/Float/Char fills, without inventing an Index-Of-only special case or prematurely approving physical optimization. A concrete primitive identity is required to run fill-cell evaluation; user-defined effectful forms remain unsupported pending observable-effect and error contracts. Boxed/sparse fills, suppressible computational errors, heterogeneous cell-result padding, and general name/effect sequencing remain open. Conservative Rank analysis facts are unchanged.

[Linux milestone CI 37449102885](https://github.com/yunskim/RustJ/actions/runs/37449102885) succeeded across the check job and pinned `j64`/`j64avx2` × default/portable execution: `tools/ranked_search_audit.py` compared 20 cases per configuration in C J / independent Rust reference / optimized Rust, **20/20 each and 80/80 aggregate**. [Basis compile probe 37449102980](https://github.com/yunskim/RustJ/actions/runs/37449102980) also succeeded. The suite covers first/last/membership, scalar/row/broadcast, typed empty frames and primitive reduction/addition. These are supported-subset confirmations, not full Rank, JX-04 or FW-04 acceptance. Keep all broader gates unchecked.

<a id="rank-cellapply-followups"></a>

### P.11 Rank/CellApply follow-up migration checklist — FW-04/JX-04 subordinate gates (2026-10-06)

**Scope and acceptance.** This checklist tracks the move from the validated **dense zero-frame Rank subset** in §P.10 to full J Rank/CellApply semantics. It is subordinate to §O.5 FW-04 and §Q JX-04; it does not authorize a physical optimization. **Scoped progress: 5/12 checked; FW-04 and JX-04 stay [ ].** The pinned [jsource cr.c](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c) (jtrank1ex/jtrank2ex) and result.h are the semantic baseline.

| Gate / stage | Check | Owner and acceptance | Evidence and required negative cases |
|---|---|---|---|
| RK-01 / A·M2 | [x] Source-first empty-frame contract | P.10 records the fill/first-existing-cell evaluation and result type/shape assembly; source error fallback remains separate | Pre-fix [CI 37446732381](https://github.com/yunskim/RustJ/actions/runs/37446732381): 13/14 matched per variant, one empty-frame discrepancy |
| RK-02 / A·M2 | [x] Typed dense fill-cell + empty assembly | value.rs rank_fill_cell/empty_rank_result preserve Bool/Int/Float/Char types and real first cell when available | Tests for fill type, shape and boxed refusal; no guessed boxed/sparse semantics |
| RK-03 / A·M2 | [x] Shared monadic/dyadic Rank execution | logical_executor.rs, kernels.rs, runtime.rs use shared Rank semantics, not an Index-Of-specific output; keep static Facts conservative | semantic.rs and analysis.rs tests |
| RK-04 / A·M2 | [x] No unproven user-body execution | Primitive-witness guard allows known built-in fill evaluation only. Unsupported otherwise | empty_scope.rs effects and name scope regression; not full J user-defined Rank |
| RK-05 / A·FW-04 | [x] Pinned C / independent Rust / optimized Rust matrix | ranked_search_audit.py and Linux CI compare first/last/member, typed empty frames, addition/reduction and frame cases | [4×20 CI 37449102885](https://github.com/yunskim/RustJ/actions/runs/37449102885) 80/80 matched; [basis 37449102980](https://github.com/yunskim/RustJ/actions/runs/37449102980) passed |
| RK-06 / B·M2 | [x] Adversarial empty frame and rank corpus | ranked_search_audit.py + semantic tests: zero-leading/interior frames, only one operand empty, frame mismatch, negative/oversized rank, incompatible types, nested Rank | Pin C outcomes first. Distinguish value/error/Unsupported, test both C builds and Rust variants  **Diagnostic evidence (2026-10-06; [CI 37452153562](https://github.com/yunskim/RustJ/actions/runs/37452153562)):** each of j64/j64avx2 × default/portable had 22/24 matching pinned-C/Rust semantic/Rust optimized observations, with two `rust_semantic_mismatch` cases (`empty_type_mismatch`, `nested_rank_empty`). A green workflow is not conformance: this adversarial corpus intentionally remains non-acceptance. [`45af152`](https://github.com/yunskim/RustJ/commit/45af1526c41e33d9a5b3fda8bf7bfd5bf92d9b4e) and [`59bb6d8`](https://github.com/yunskim/RustJ/commit/59bb6d8adff86db0fd8c1d43bb41cc64bbc6dc52) expose full three-way observations and test diagnostic reporting. **At this earlier stage two cases diverged; see the subsequently accepted 32-case bounded gate below.**  **Partial fix with executed evidence (2026-10-06):** [`4d5f903`](https://github.com/yunskim/RustJ/commit/4d5f90348a91ece4c15139d0ae56affb1ade68c6) and [`edad61b`](https://github.com/yunskim/RustJ/commit/edad61bc17ebeb46198f1d1f050bcb9bc95f0a42) conservatively permit value-only fill through a Rank-of-Rank of an intrinsic primitive in the independent A3 and normal Rust executors, with positive [`a4c5953`](https://github.com/yunskim/RustJ/commit/a4c5953349ea8fba34df7bdcaed3e683ee773b01) and effectful negative [`f074620`](https://github.com/yunskim/RustJ/commit/f074620e6317e5f86ffb425f07fba00a00847cdb) tests. [Linux CI 37453410364](https://github.com/yunskim/RustJ/actions/runs/37453410364) **5/5 jobs success**, pinned-C three-way adversarial **23/24 matched, 1 `rust_semantic_mismatch` (`empty_type_mismatch`) in each of four variants**. The nested-Rank gap was closed, but this is **not full Rank conformance or ZF-IR-03 evidence; RK-06 remained [ ] at this earlier checkpoint.** |
| RK-07 / B·M2 — **next** | [ ] Non-exigent/exigent error and retry semantics | Capture cr.c computational-error recovery, type retry, error precedence, and effect ordering in error.rs and Rank execution | C comparison for overflow, domain, incompatible type, shape error and order |
| RK-08 / B·M3/FW-07 | [ ] User-defined effects, dynamic name resolution | Runtime witness, lookup time, observable prototype call count, post-effect replay and guarded fallback contracts | C-vs-Rust counter/binding/late-name tests; never speculate user code |
| RK-09 / B·M3 | [ ] Boxed and sparse filler/prototype | value.rs/storage.rs/sparse.rs implement J prototype, fill value, axes and dtype semantics | Pinned C positive/negative suite; no zero-filled fake support |
| RK-10 / B·M3 | [ ] Heterogeneous cell result assembly | assembly.rs/logical_executor.rs/kernels.rs: promotions, shape padding, error precedence as in result.h | Mixed type/shape, empty cells, char/numeric, errors and effect order |
| RK-11 / C·M3 | [ ] Derived Rank and implicit loops | Preserve semantic operands and rank/cell/frame on fork/hook/@:/nested Rank/late names; no premature Graph route selection | Rank, frame repeat, late binding and modifier three-way fixtures. See §P.12 ZF-IR-01–04 for the narrower Graph empty-frame contract; broader RK-11 remains unchecked. |
| RK-12 / D·FW-04/JX-04 | [ ] Scoped final semantic acceptance | For supported RK-06–11 claims: independent C / Rust reference / runtime, guard/effect/error/resource proof and explicit unsupported scope | Record pinned commit, 4 CI variants, command, case counts, classifications and JSON artifacts before marking each subgate complete; broader FW/JX remain [ ] where duties remain |

**Iteration order and checklist use.** RK-06 → RK-07 → RK-08 → RK-09 → RK-10 → RK-11 → RK-12. For every gate: (1) pin positive and negative C cases, (2) classify C vs independent Rust baseline vs execution, (3) make the smallest shared semantic change, (4) run default/portable fmt, Clippy and tests plus both pinned C builds, and (5) append evidence to that RK row *before* ticking it. A passing subset cannot waive failures or turn on hash/GPU/Graph candidates without separate FW-05–FW-13 proof/guard/resource/cost authorization. M2 frontend convergence remains the overall priority.


<a id="rank-graph-zero-frame"></a>

### P.12 Zero-frame versus empty-cell semantics in J Graph IR — structural facts are not permission to elide evaluation (2026-10-06)

**Decision.** Empty arrays remain ordinary logical arrays with J-visible type, shape and original Rank/CellApply semantics, not a new `EmptyArray` operation. Both `0 3` and `2 0` have zero atoms, but at rank 1 the former has **frame=[0], cell=[3]** (no ordinary cell iterations, but J fill-cell semantics apply); the latter has **frame=[2], cell=[0]** (two actual calls on empty cells). Therefore `element_count == 0` never by itself proves that there are no observable computations, errors or effects. Consult pinned `jsrc/cr.c::jtrank1ex/jtrank2ex` and §P.10.

**Implementation boundary.** The shared pure `src/facts.rs::rank_plan_for_shapes` produces frame/cell structure; `RankPlan::frame_execution()` distinguishes `ZeroFrameNeedsFill / CellsPresent / IncompatibleFrames`; `has_empty_input_cell()` is a separate structural fact. `src/j_graph_ir.rs::Plan::rank_frame_plan(ValueId)` exposes a **read-only** view only for an applied `GraphForm::Rank` with known requested ranks and argument shapes. J Graph projection and A3 analysis use the same frame decomposition. Unknown output-cell dtype/shape stays unknown. No physical buffer identity, execution selection, extra graph node or premature optimizer is introduced.

**Safety barrier.** `ZeroFrameNeedsFill` is neither `ProvenEmptyResult` nor `SafeToElide`. The fill-cell can establish result dtype/shape, raise observable errors or execute effectful/named functions. A zero axis in the A3 `IterationDomain` does not automatically allow kernel skipping, fusion or buffer omission. A later optimization must separately prove result-cell shape/type, purity, error and dynamic-name behavior, guards and fallback; actual selection must pass FW-05–FW-13 resource/cost/execution gates.

**Array-compiler comparison and transfer boundaries (2026-10-06).** These references provide mechanisms for separate RustJ layers, **not** evidence that another compiler already implements J's zero-frame fill-cell semantics.

| Reference | Mechanism | RustJ transfer and limitation |
|---|---|---|
| [XLA ZeroSizedHloElimination](https://github.com/openxla/xla/blob/main/xla/hlo/transforms/simplifiers/zero_sized_hlo_elimination.h), [pass guide](https://openxla.org/xla/hlo_passes) | Replaces zero-element HLO results with empty constants | Generate analogous rewrites **only after** result semantics and elision legality are proved. Never suppress J fill-cell evaluation, effects or errors merely because item count is zero |
| [Futhark size types](https://www.futhark-lang.org/blog/2020-03-15-futhark-0.15.1-released.html) | Encodes element type and sizes as `[n]a`; distinguishes `[0][2]i32` from `[2][0]i32` | Prove result-cell dtype/shape separately from Rank frame geometry; do not erase J dynamic names/effects or heterogeneous result assembly |
| [MLIR Linalg](https://mlir.llvm.org/docs/Dialects/Linalg/) | Separates iteration/indexing space from compute payload region | Preserve `CellApply` iteration structure separately from cell computation and result assembly. `tensor.empty` creates an uninitialized-content tensor of a specified shape; it does **not** mean a zero-element tensor |
| [StableHLO reduce](https://openxla.org/stablehlo/spec#reduce) | Explicit reduction axes, reducer computation, init_values and result types | Apply known-result inference only to the relevant proved subset; do not assume an initialized StableHLO reduction is equivalent to arbitrary J `/` or Rank fill-cell semantics |

**ZF-IR-03 independent evidence ledger (planned; not implemented).** Keep the original J Graph immutable. A recomputable sidecar must independently track (1) graph/value identity, version and dynamic-name validity; (2) J result-cell dtype/shape, boxed/sparse fill and assembly; (3) semantic equivalence of fill-cell evaluation versus reconstructed output; (4) observable effects, errors, precedence and handlers; and (5) a guard ordered before the first observable effect with exact semantic-reference fallback. Each obligation distinguishes `Unknown / Proven(witness) / Guarded(guard+fallback) / Disproven`. Neither `frame=[0]` nor partially known shape upgrades an Unknown obligation to Proven. Guarded by itself never enables selection. FW-05–FW-13 and ZF-IR-04 remain the separate execution-selection gates.

**Work order.** Start with RK-06 adversarial C-oracle Rank/zero-frame fixtures; establish RK-07–10 error/effect/boxed/sparse/assembly semantics; then implement ZF-IR-03 sidecar and Unknown/Guard negative tests; finally validate ZF-IR-04 with independent three-way CPU/physical measurements. This comparison does not advance M3 ahead of M2, or tick ZF-IR-03/04 or FW-06/07.

**Exact RK-06 divergent inputs and outputs.** [Diagnostic CI 37452934704](https://github.com/yunskim/RustJ/actions/runs/37452934704), `j64/default` job: `(0 3 $ 'abc') (+"1 1) (i.0 3)` gives pinned J **integer type code 4**, shape `[0,3]`, empty data, but both Rust routes return **`domain error`**. `(i.0 3) ((+"0 0)"1 1) (i.0 3)` gives pinned J **integer type code 4**, shape `[0,3]`, empty data, but both Rust routes return **`unsupported`**. The full `jsrc/cr.c::jtrank2ex` fill-call/error-recovery contract and nested Rank semantics need separate verification. **Do not fix this by automatically inheriting an input type or indiscriminately suppressing errors.** The stated outputs come from the actual three-way diagnostic log.

**Additional RK-06 pinned-C observations (authored; CI verification pending).** Commits [`a4ffb0c`](https://github.com/yunskim/RustJ/commit/a4ffb0c240e5a4271bb9f20b86627f0f5fca157f), [`b241c97`](https://github.com/yunskim/RustJ/commit/b241c9730e1993cda3a402b9256f1e184d9fc501), [`d5f8a03`](https://github.com/yunskim/RustJ/commit/d5f8a0312952e3d47ee53a48b7eb6b1a67c0d012) extend `tools/ranked_search_audit.py --adversarial` from 24 to **32 cases**. Eight new probes distinguish char/int order, char/float and bool fillers, one-sided empty frames, nonempty char/int cells, positive frames of empty cells, and nonempty nested Rank. In pinned `jsrc/cr.c::jtrank2ex`, `EVINHOMO` fill-cell retry selects a target type based on which original arguments have atoms; only *after retry* are non-exigent computational errors replaced with an integer-zero scalar while exigent errors propagate. **Four-way pinned-C evidence (2026-10-06):** [Linux CI 37453629236](https://github.com/yunskim/RustJ/actions/runs/37453629236) completed **5/5 jobs successfully** (check + pinned C j64/j64avx2 × Rust default/portable). Each reference combination reports **25 pass / 7 `rust_semantic_mismatch` out of 32**, i.e. 100/128 matched observations and 28 mismatched observations across four runs (seven distinct sources). The seven open sources are `empty_type_mismatch`, `empty_type_mismatch_reversed`, `empty_char_float_fill`, `empty_char_bool_fill`, `empty_char_left_real_right`, `empty_char_right_real_left`, and `positive_frame_empty_char_cells`. For the first six, pinned J yields integer shape `[0,3]`; for the last, integer shape `[2,0]`. Both independent Rust semantic reference and optimized Rust report `domain error` in all seven. The nonempty char/int domain case and nonempty nested-Rank case match. This **green CI validates diagnostic execution, not J conformance**; none of the seven mismatches is accepted or silently waived. This 25/32 result is historical pre-fix evidence. Only the finite 32-case RK-06 corpus is now promoted to a strict gate below; RK-07 and ZF-IR-03 remain unchecked. The preexisting 20-case strict rank gate remains unchanged.

**RK-06 bounded acceptance (2026-10-06; not general RK-07 acceptance).** [Final strict Linux CI 37458845392](https://github.com/yunskim/RustJ/actions/runs/37458845392) completed **5/5 jobs**: check and pinned C j64/j64avx2 × Rust default/portable. Against the exact pinned C revision `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`, **32 cases in each of four configurations, 128/128 three-way classifications `pass`, zero mismatches**. `--adversarial --gate-adversarial` now returns nonzero when any C/independent-Rust-reference/optimized-Rust comparison differs, while printing and retaining the exact offending observations. [`365c3ad`](https://github.com/yunskim/RustJ/commit/365c3adcb9e716e34f3083f50c317e243a51ad23) installs this **blocking four-way Linux CI gate**; [`1644644`](https://github.com/yunskim/RustJ/commit/1644644444d8e51ee2097f4acd659b1e4b893013) and [`b631f6f`](https://github.com/yunskim/RustJ/commit/b631f6fb488f449fcc4c35b44da6ac5bcca444d8) test its failure behavior. The existing 20-case strict ranked-search gate remains intact. **Only the pinned, finite, supported-dense RK-06 corpus is marked [x]**; no full-J-Rank claim follows from these witnesses.

**RK-07 current boundary and open gates.** [`9ab2952`](https://github.com/yunskim/RustJ/commit/9ab2952100ae7b2ae1e6338a761ecf4fc798804f) implements integer-zero replacement of a **Domain failure from a value-only zero-result-frame fill-cell computation**. [`cbd13af`](https://github.com/yunskim/RustJ/commit/cbd13af0864a85452a2cface91f283bf07b9f4e2), [`85f1732`](https://github.com/yunskim/RustJ/commit/85f1732bb29da8b1573781e6fb6b86711d91468e), and [`a5541b2`](https://github.com/yunskim/RustJ/commit/a5541b22705b4f53aeb8a2094d958d5d749ec557) preserve the intrinsic atom-rank-zero result-cell shape of primitive `+`, avoiding an incorrect collapse of [0,3] to [0]. [`a23bcf4`](https://github.com/yunskim/RustJ/commit/a23bcf40b104e359fe135da9d76c8d0ae521e672) and [`6be596a`](https://github.com/yunskim/RustJ/commit/6be596a81ccec1edb346fe2554a42e7811c406ef) handle zero-atom `+` cells inside a positive outer Rank frame [2], preserving integer [2,0]. [`0d85f03`](https://github.com/yunskim/RustJ/commit/0d85f030b00122d2239150a8d868327ff8810f0c) and [`75a36eb`](https://github.com/yunskim/RustJ/commit/75a36eb7886ca2f68d9527412e41453b93d9af62) regress the original seven cases, nonempty char/int Domain and mismatched-frame Length; unit tests keep unsupported/resource errors intact. Pinned references: [`jsrc/cr.c::jtrank2ex` L397–418](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c#L397-L418), [`jtrank2ex0` L541–569](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c#L541-L569), [`jerr.h::EXIGENTERROR` L89–90](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/jerr.h#L89-L90). **Still open:** the scoped dense primitive-`,` `EVINHOMO` retry below is implemented, but generic internal error classification/provenance for other primitives, the remaining non-exigent/exigent taxonomy and precedence, user-function effects/name versions, boxed/sparse fill, and heterogeneous result assembly are not. Unknown/Unsupported/Limit must not be silently recovered; Graph IR frame geometry alone never authorizes skipped execution or GPU lowering. **RK-07/08–12, ZF-IR-03/04, and FW-06/07 remain [ ].**

**RK-07 — bounded `EVINHOMO` typed-retry implementation and C-oracle evidence (2026-10-06).** Pinned [`jsrc/cr.c::jtrank2ex` L397–418](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c#L397-L418) and [`jtrank2ex0` L541–569](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c#L541-L569) prescribe: first invoke the verb on typed fill-cells; on `EVINHOMO`, choose the type of an **original argument that has actual atoms**, otherwise the higher-ranked type when both originals are empty; **regenerate default fill-cells, rather than casting their contents**; retry once; finally substitute integer scalar zero for remaining non-exigent computational errors, but propagate exigent errors. The dense-type priorities bool < char < int < float come from pinned [`j.h::TYPEPRIORITY` L2040–2046](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/j.h#L2040-L2046). This decision is not derivable from a zero-frame shape alone: retain verb identity, original-argument atom presence and dtype through execution.

[`ae7258d`](https://github.com/yunskim/RustJ/commit/ae7258d2d391ee43cecb5477f1ccd52f96025e80) added **10 separate RK-07 C-oracle probes**, without changing RK-06's 32 strict tests: heterogeneous char/int in both orders, char/float, char/bool, one-sided nonempty arguments, positive frames with zero-atom cells and regular-error/length-error controls. Initial [exploratory run 37460517931](https://github.com/yunskim/RustJ/actions/runs/37460517931) j64/default showed **3/10 pass, 7 mismatches** (for example pinned int or float `[0,6]` versus Rust int `[0]`, and int `[2,0]` versus Rust Domain). [`d192ad6`](https://github.com/yunskim/RustJ/commit/d192ad6889b6dbcdf654321b7668b040cde54ae9) implemented target-typed fill regeneration without source data. [`4e35d44`](https://github.com/yunskim/RustJ/commit/4e35d443eb4eb8f2c8fa4fc8528439fc18aa141e), [`5485011`](https://github.com/yunskim/RustJ/commit/5485011de69fd2354d19059cd79612d1c4fcd3e8), and [`a6a254c`](https://github.com/yunskim/RustJ/commit/a6a254cac2cee70a963abd2b19a4ef2194f29632) allow exactly one typed retry for the **concrete pure primitive `,` in a witnessed dense char/numeric mismatch**, not for arbitrary Domain failures. [`48e8a6a`](https://github.com/yunskim/RustJ/commit/48e8a6a203e1f58f28063b0765f60a19ff59731e) preserves the output type for zero-atom catenate cells within nonempty outer frames. [`b370ac8`](https://github.com/yunskim/RustJ/commit/b370ac855f6cc81b02a38c569541100f98f2356e) and [`177b18c`](https://github.com/yunskim/RustJ/commit/177b18c50c4c2ad70a83aa3d7aee793c86a065da) pin expected type/shape, single retry, real nonempty-cell Domain, frame Length and no retry on Limit.

**Executed acceptance boundary:** [final Linux CI 37461525543](https://github.com/yunskim/RustJ/actions/runs/37461525543), pinned C revision `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`. Python tests, fmt, clippy, Rust default/portable tests/build (check) plus C j64/j64avx2 × Rust default/portable comparisons **5/5 jobs green**. The **10 RK-07 probes × 4 configurations = 40/40 independent C/Rust-semantic-reference/Rust execution comparisons passed, 0 mismatches**. [`37e04dc`](https://github.com/yunskim/RustJ/commit/37e04dc8109065e86e4c2bca333416d0c9b441f4), [`706fa36`](https://github.com/yunskim/RustJ/commit/706fa367aa860760d5b4df3b17dbb9b26ccf2da3) and [`b245f9e`](https://github.com/yunskim/RustJ/commit/b245f9e8a27bac0ffd21b724a9ca8df0d18b941c) install the blocking `--retry-probes --gate-retry-probes` four-way CI gate with unwaived C/reference/optimized observations in archived JSON. The original RK-06 32-case strict gate remains intact.

| RK-07 subtask | Status | Remaining acceptance requirement |
|---|---|---|
| RK-07-E1: type-specific catenate retry | [x] bounded dense subset | Original atom-presence priority, typed default filler regeneration, one retry, C 10-case differential |
| RK-07-E2: durable evidence | [x] strict 10-case gate | 4 configurations, 40/40, CI fails any mismatch and retains JSON |
| RK-07-E3: internal error provenance | [ ] | Distinguish `EVINHOMO` from generic Domain across other primitives and failure phases; never infer retry authority from Domain alone  **Partial implementation (2026-10-06; still [ ]):** [`5aa7074`](https://github.com/yunskim/RustJ/commit/5aa7074762ca6eac1570bc17d728b9ee07b6c3fc) requires an `Option<VerifiedValueOnlyZeroFrame>` proof token for `recover_zero_frame_fill_domain`. Domain failures from ordinary cells and unresolved/effectful calls are not recoverable; the previously verified value-only synthetic zero-frame path retains integer-zero fallback. Both independent A3 reference and primitive kernels supply the token, with negative tests. A distinct general internal `EVINHOMO` cause/provenance and C exigent-error classification remain unimplemented; this does not authorize ZF-IR-03 or FW-06/07 optimization. [Linux CI 37467064363](https://github.com/yunskim/RustJ/actions/runs/37467064363) **all 5/5 jobs passed**, including check and pinned j64/j64avx2 × Rust default/portable. Per configuration RK-06 32/32 + RK-07 retry 10/10 + error-precedence 8/8, thus **200/200 strict three-way C/Rust comparisons matched, 0 mismatches**. [Basis probe 37467064362](https://github.com/yunskim/RustJ/actions/runs/37467064362) passed. The general internal-error cause classification (E3) and exigent provenance (E4) **remain [ ]**. |
| RK-07-E4: exigent/non-exigent classification | [ ] | Pinned [`jerr.h::EXIGENTERROR` L87–90](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/jerr.h#L87-L90), overflow and error precedence; Rust Limit may conflate J EVLIMIT with actual allocation/resource failure, so do not suppress without provenance |
| RK-07-E5: semantic coverage expansion | [ ] | Derived/dynamic/effectful calls, boxed/sparse and nonuniform result assembly; structural frame facts must never license skipped effects or GPU lowering |

**Decision:** Only the finite dense primitive-catenate retry subset is accepted. **Parent RK-07, RK-08–12, ZF-IR-03/04 and FW-06/07 remain [ ]**. Neither general J Rank conformance nor optimization eligibility follows.

**RK-07 — independent eight-case pinned-C error-precedence corpus (2026-10-06).** [Diagnostic Linux CI 37463314265](https://github.com/yunskim/RustJ/actions/runs/37463314265) succeeded on all **5/5 jobs**, checking pinned C j64/j64avx2 × Rust default/portable, with **8/8 three-way matches in each configuration (32/32 total; zero mismatches)**. `--error-probes` independently covers (1) inner cell Length under a zero result frame, (2) mixed empty char/numeric cells of different length, (3) genuine Length with a positive frame, (4) prefix-frame Length before fill execution, (5) empty-frame versus populated-cell Index, and (6) division with zero versus positive frames. The existing strict 32-case RK-06 and 10-case RK-07 `EVINHOMO` gates are unchanged. [`a84a508`](https://github.com/yunskim/RustJ/commit/a84a5084f3a4318f1f26dc95b0428e10317064cd) and [`cce5dec`](https://github.com/yunskim/RustJ/commit/cce5decdbbec2d6577c415d5696f8d7775738b10) add the C corpus and unwaived-difference tests. [`ba90688`](https://github.com/yunskim/RustJ/commit/ba9068801554e5836d9b57111e8e937440c23172), [`970f6b0`](https://github.com/yunskim/RustJ/commit/970f6b07c09ed3969cfc158005c12a2795ce5010), and [`c90701d`](https://github.com/yunskim/RustJ/commit/c90701de2b456e556fb256128e976a7556793930) promote `--error-probes --gate-error-probes` to a blocking bounded CI regression. **Final evidence: [Linux CI 37463712382](https://github.com/yunskim/RustJ/actions/runs/37463712382), 5/5 jobs successful, eight pinned-C three-way matches per reference configuration (32/32 total, zero mismatches)**. The exact source/result observations remain in each reference job and `rank-error-*.json` artifact. Only this **finite eight-case strict regression** is accepted; **overall RK-07 remains unchecked** because internal error provenance, complete exigent/non-exigent taxonomy, effects and general Rank semantics remain unproven.

**RK-07-E4 exact exigent-error boundary.** In pinned [`jsrc/jerr.h` L7–55 and L87–90](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/jerr.h#L87-L90), `EXIGENTERROR` contains selected internal J error codes including allocation/workspace, valence, nonce, value, throw, stack, and system failures that must propagate even for fill-cell execution. Although `EVDOMAIN`, `EVLENGTH`, `EVINDEX`, `EVRANK`, `EVLIMIT`, and `EVINHOMO` are not in that mask, it does **not** authorize suppressing arbitrary Rust errors. RustJ's `Error::Limit` does not yet distinguish J `EVLIMIT` from allocator/resource failure, and `Error::Unsupported` is not equivalent to J `EVNONCE`. A separate, proven witness for **internal error code, phase, provenance and possible effects** is required before broadening retry/suppression. Next steps are internal error provenance classification (E4), independently pinned precedence/exigent cases, and effectful user-function/dynamic-name boundaries (E5). The finite 8-case matches are neither all-rank-error proof nor permission for Graph IR zero-frame elision.

**Living sub-checklist (under RK-11 and FW-04/JX-04).**

| Gate | Status | Required evidence |
|---|---|---|
| ZF-IR-01 / M2 | [x] **Common Rank geometry** | Shared frame split and `RankFrameExecution` with Graph/A3 structural agreement. Implemented in [911e113](https://github.com/yunskim/RustJ/commit/911e113c8761f3e7b25ca6b932f6c8ab56e0b398) and [fb9f884](https://github.com/yunskim/RustJ/commit/fb9f8842ac74ef289e0852927f3ef65d54574f11); [Linux CI 37451816951](https://github.com/yunskim/RustJ/actions/runs/37451816951) check and four oracle jobs passed |
| ZF-IR-02 / M2 | [x] **Read-only Graph query and negative tests** | `Plan::rank_frame_plan` [89984f1](https://github.com/yunskim/RustJ/commit/89984f10aa6e3869e2f3d4f77730e2537030854d), tests [b52fd51](https://github.com/yunskim/RustJ/commit/b52fd51d2420c27d4425b920f534b8941d0f0871) distinguishing `0 3`, `2 0`, inner zero and incompatible frame; Graph=A3; non-Rank=None; output remains unknown |
| ZF-IR-03 / M3·FW-06/07 | [ ] **Independent proof for empty-result elision** | Result-cell dtype/shape, effects, errors, names, guards and fallback; prohibit Unknown→skip and frame-zero→automatic kernel skip with negative tests |
| ZF-IR-04 / M3/M4·FW-11/13 | [ ] **Executed lowering and resource proof** | Compare pinned C, independent Rust semantic reference and optimized Rust, then measure CPU cost before enabling any individual kernel/buffer elision; preserve nonempty frames of empty cells. No GPU permission |

**Status (2026-10-06):** ZF-IR-01/02 **2/4 checked**, supported by [Linux milestone run 37451816951](https://github.com/yunskim/RustJ/actions/runs/37451816951) at [`e9f821d`](https://github.com/yunskim/RustJ/commit/e9f821d0658005a1545131d31c31fb53d3521b26): **all 5/5 jobs passed**, including default/portable Rust checks and j64/j64avx2 × default/portable differential jobs. This is a regression of the unchanged execution path, NOT proof or execution of zero-frame kernel elision. ZF-IR-03/04 are unimplemented. RK-11, FW-04 and JX-04 remain [ ]; §P.11 RK-06–RK-12 precedence is unchanged.

<a id="jsource-optimization-migration"></a>

### Q. Whole-jsource optimization migration plan and living checklist — beyond Index-Of (2026-10-06)

**Scope and authority.** §7.4.3 A–M catalogues audited jsource optimization **ideas, original guard/fallback constraints, and RustJ stage ownership**. §O.5 **FW-01–FW-18** owns shared semantics/evidence/guard/CPU milestones, and §P.1 owns **Index-Of-specific implementation**. This section is the complementary **per-family migration tracker**; passing one tracker does not complete another. Do not create another generic optimizer, translate C special entrypoints one-for-one into Graph IR nodes, or pre-build a common registry. Source audit pin: [jsource 13994ffa1ed5f06f79fad6e9822a7ed2d29b1528](https://github.com/jsoftware/jsource/tree/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc).

**Status: 0 / 26 complete.** Every [ ] is an **executed acceptance gate**, not a statement that a source family, candidate analyzer or test source exists. Keep M2 frontend and **FW-01 first**; source classification can proceed statically. Follow A (semantic baseline) → B (M3 proof/guard boundary) → C (family candidates and implementations) → D (M4 and later execution approval). C-stage candidate/proof research may precede FW-11–13, but **selecting or enabling specialized execution** requires the independently validated CPU baseline and separated target/resource/cost evidence. Do not extract a shared family interface before the second-family demonstration JX-08.

| ID / gate | Source family and jsource evidence | RustJ change boundary | Acceptance / negative and differential evidence required |
|---|---|---|---|
| JX-01 / A | **Pinned source and coverage inventory:** ca/cf/cr/va2/ar/ap, ao/cip/gemm, vg/vgsort/vgranking, vi/viix, cc/cu/vfrom/vf/vrep/vcat, sparse/sc/a/vrand/am in §A/H/K/M | src/j_graph_jsource.rs catalogue/source pin/coverage and §7.4.3 A–M | For each family map **source form → entrypoint → guard/fallback → semantic conditions → RustJ owner → status**. Track duplicates and unaudited paths. Diff future upstream revisions separately; never claim exhaustive coverage |
| JX-02 / A·FW-01 | **Frontend and derived entity coverage:** @:, fork/hook, rank, /., dot, grade, cut, under, M., ?/?. | src/tokenizer.rs, src/enqueuer.rs, src/parser.rs, src/semantic.rs, src/j_graph_ir.rs | Source/valence/POS/operand/late-binding must be supported before creating candidates. Unsupported forms stay AwaitingFrontendOrFacts; compare positive and negative parse/resolve cases with a real J C oracle |
| JX-03 / A·FW-02 | **Truly sequential semantic baseline:** Reduce, Scan, Search and later families | src/runtime.rs, src/kernels.rs, src/index_ops.rs, semantic/runtime tests | Separate pool ownership from optimization authorization. Instrument zero candidate selection/prehash/specialized dispatch for supported strict Rust reference; external J C is a differential oracle, not normal fallback |
| JX-04 / A·FW-03/04 | **J semantic fixture matrix:** rank/cell/frame, Fit/CCT, empty/prototype, boxed/sparse, numeric overflow, error precedence | tests/semantic.rs, tests/j_graph_jsource.rs, tests/index_ops.rs, existing differential harness | Record **independent pinned C / strict Rust / optimized Rust** outputs, supported coverage and unresolved issues. Distinguish upstream special-path oddities; never claim legacy Rust near is J CCT **2026-10-06 partial evidence:** [4×20 C/Rust Rank gate](https://github.com/yunskim/RustJ/actions/runs/37449102885) all 80/80 matched; [Basis probe](https://github.com/yunskim/RustJ/actions/runs/37449102980) passed. Typed dense zero-frame Rank is implemented. User-defined effects, boxed/sparse fillers and error fallback remain unresolved; gate [ ] remains. **Follow-up unit gates:** [P.11 RK-01–RK-12](#rank-cellapply-followups) tracks evidence and state; parent remains [ ]. |
| JX-05 / B·FW-05 | **Versioned multi-region source anchors:** Cut–Scan–Raze, convolution, composition/nested rank | src/j_graph_ir.rs, src/j_graph_jsource.rs, src/analysis.rs, src/lowering.rs | Verify immutable graph schema, source value, region, ordered A3 operation set, rule/version via sidecar. Reject stale, forged, span-reused and wrong-region witnesses |
| JX-06 / B·FW-06/08/09 | **Family-specific proof obligations:** valence, rank, equality, empty, error, ownership, resources | src/j_graph_jsource.rs, src/lowering.rs, current proof/verifier boundary | Keep per-obligation Unknown/Proven/Guarded/Disproven with witnesses. Any unresolved obligation blocks commit/selection; negative-test cross-family misuse of rewrite/fusion/search evidence |
| JX-07 / B·FW-07 | **Guard, fallback and effect boundaries:** RNG, name lookup, memo, allocation, numeric retry | src/lowering.rs, src/runtime.rs, src/execution_semantics.rs | Guards precede observable effects, and misses fall back to an explicit baseline route. Reject replay after cache mutation, RNG draws, effects or observable errors; negative-test guard misses and post-effect reexecution |
| JX-08 / B·FW-10 | **Second genuinely independent family:** Reduce/Scan **or** GroupReduce | src/j_graph_scan.rs or GroupBy basis, src/j_graph_jsource.rs, src/lowering.rs | Exercise JX-05–07 on numeric association/order or group representative/order obligations distinct from Search. Do not extract common Candidate/SelectionPlan APIs beforehand |
| JX-09 / C | **Composition, rank absorption, MapReduce streaming:** ca.c, cf.c, cr.c, va2.c::jtfslashatg | src/j_graph_fusion.rs, src/j_graph_composition.rs, src/fusion_planning.rs | Retain @:/capped-fork provenance and rank domains; dense/empty/type/effect/inplace-cost guards. Do not duplicate existing fusion candidates; compare with generic fallback |
| JX-10 / C | **Reduce, Mean, shape-driven fast paths:** ar.c::jtreduce/jtmean, cf.c::jtfolk | src/j_graph_jsource.rs, src/j_graph_scan.rs, Reduce lowering | +/%# is a **monadic** Mean opportunity only. Verify lengths 0/1/2, cell shape, prototype, promotion, FP order, overflow. Do not infer single-pass fusion or general reassociation |
| JX-11 / C | **Prefix Scan, Infix Window, MovingAverage:** ap.c::jtpscan/jtmovfslash/jtmovavg | src/j_graph_scan.rs, src/j_graph_jsource.rs, Window/Scan lowering | Validate prefix vs sliding order, NaN/overflow, window lengths and fallback; retain original graph provenance on rediscovered Mean→Window candidates |
| JX-12 / C | **Key/GroupBy plus GroupReduce:** ao.c::jtkeyct/jtsldot | src/j_graph_jsource.rs, GroupBy semantic basis, src/analysis.rs | Prove key equality/CCT, first-appearance group ordering, representatives, types, overflow, empty/sparse before direct aggregation; generic Search proof is insufficient |
| JX-13 / C | **Contraction, full Dot/GEMM and oblique convolution:** cip.c, gemm.c, ao.c::jtpolymult; rank-1 sum-times in cr.c/va2.c | Contract basis in src/execution_semantics.rs, src/analysis.rs, src/lowering.rs | Distinguish general dot, restricted sum-times and oblique reductions; prove rank/agreement/Fit/overflow-retry/FP order/sparse and BLAS capability, then choose the library route downstream |
| JX-14 / C | **Grade/Sort/Ranking and order statistics:** vg.c, vgsort.c, vgranking.c, vg.c::jtordstat | src/j_graph_jsource.rs, Grade/Ranking basis, src/lowering.rs | Prove ties/order, key/type/range, rank/index errors. **RNG draws inside order-statistic selection** may affect later J state; no pure quickselect rewrite until seed/trace effects are resolved |
| JX-15 / C·§P | **Search/Index-Of versus interval I.:** vi.c, viavx.c, viix.c, viavx2.c | src/index_ops.rs, SearchDescriptor in src/logical_ir.rs, src/physical.rs | Delegate detailed index-family work to **§P.1 and FW-02/03/04/12**. Prove dyadic I. sorted interval semantics, distinguish i./i:/e. result/representative modes and E. window; no unproven tolerant optimization |
| JX-16 / C | **Byte char-map LUT and Boolean/sparse mask to indices:** cf.c::jthkiota, v.c::jtcharmap | src/j_graph_jsource.rs, IndexSpace/Lookup/Gather basis | Prove byte alphabet, 256-entry limits, first duplicate match, invalid-index errors, mask/sparse fill and rank; do not jump from source detection directly to LUT/compact execution |
| JX-17 / C | **Cut→Scan→Raze and Box+Append/Raze:** ca.c, cc.c::jtrazecut1/2, vo.c::jtjlink | src/j_graph_ir.rs, src/j_graph_fusion.rs, ConcatAssemble/Result Assembly | Treat the **zero-cut result-axis difference between source optimized and generic paths** as a blocker. Preserve intended J semantics, fill/shape, boxed lifetime, effects and correct fallback before transforming |
| JX-18 / C | **Shape/Count shortcuts, bound constants, Hook thresholds:** ca.c, v.c, cf.c, va1.c, vx.c, vz.c | src/j_graph_ir.rs facts, src/analysis.rs, numeric lowering | Separate shape/count demands from value computation without suppressing errors/prototypes/sparse cases. Prove domain, Fit, FP rounding and SIMD/NaN guards for floor-log/digits/power/deadband |
| JX-19 / C | **Under/Each and inverse precomputation:** cu.c::jtsunder etc. | src/semantic.rs, src/j_graph_ir.rs, src/execution_semantics.rs | Preserve forward→inner→inverse, dynamic binding timing, effects/alias and inverse validity; reject stale cached inverses after rebinding |
| JX-20 / C | **Virtual View/Gather/Reshape/Compress/Catenate/Result Assembly:** v.c, vfrom.c, vf.c, vrep.c, vcat.c, result.h | src/physical.rs, src/storage.rs, src/lowering.rs, result assembly | No universal Box→Open cancellation. Verify contiguous/noncontiguous copy, recursive boxes, lifetime, alias/pristine/usecount, rank/shape/error; negative-test visible mutation through shared inputs |
| JX-21 / C | **Sparse-specific execution:** cpdtsp.c, vgsp.c, visp.c, vfromsp.c | src/sparse.rs, tests/sparse_runtime.rs, src/execution_semantics.rs | Validate sparse axes/fill/empty/prototype, boxed/numeric types, density/resources and distinct error/ordering; never treat dense fusion as a universal sparse fallback |
| JX-22 / C | **Amend/Scatter, donation and lifetime:** am.c, m.c, p.c, cx.c | src/runtime.rs, src/storage.rs, src/physical.rs, effect/assignment boundary | Prove index/type/readonly, alias, recursive boxes, commit/error order and failed retry. SSA liveness alone is insufficient for inplace; negative-test source mutation |
| JX-23 / C | **Dynamic name and locale lookup cache:** sc.c, cx.c | src/semantic.rs, src/runtime.rs, name/binding version runtime | Validate late binding, locale epoch, invalidation, rebinding/reentrancy and thread safety; compare cache hit/miss. Do not turn a name cache into a Graph constant rewrite |
| JX-24 / C | **Explicit M. memo and stateful RNG generate/shape:** a.c::jtmemo, vrand.c::jtrollk, vg.c | src/semantic.rs, src/runtime.rs, effect/state contracts in src/execution_semantics.rs | Keep requested M. semantics separate from unrestricted CSE. Prove cache key/lifetime/effects and observable RNG seed/draw order/state trace; unsupported remains RuntimeSemantic/Unknown |
| JX-25 / D·FW-11–17 | **Physical target, hard resources and measured costs:** jsource SIMD/AVX, GEMM, hash, view, inplace | src/lowering.rs, src/physical.rs, src/j_graph_resource.rs, src/fusion_planning.rs | After independent baseline/three-way testing, separate capabilities, hard allocation bytes and measured latency/memory. Defer SIMD/BLAS/GPU/cache-tuning without semantic proof, guard/fallback and measured gain |
| JX-26 / final·FW-18 | **Independent repeated audit and uncovered upstream paths:** numeric primitive/allocator/architecture-specific paths, new jsource commits | §7.4.3 A–M, §Q, §O.5, §P.1, reports/ | Separately audit **pinned C dispatch/fallback, J counterexamples, Graph-A3 provenance, guard-effect/runtime, target-cost/bench**. Record commit, environment/commands, pass/fail/ignored, pinned oracle scope and gaps; synchronize FW/P state |

#### Q.1 Initial JX-01/FW-01 static source/frontend audit (2026-10-06)

**Status: JX-01 [ ], FW-01 [ ] remain open.** We fetched all **16/16 pinned upstream C/H files** referenced by the current family registry. Representative text/symbols were found in 15; the `vcat.c` entry `boxed ownership transfer` is a descriptive label, **not** a literal C symbol. File/symbol presence does not prove the full guards, fallbacks, J semantics, runtime correctness, or performance.

`src/j_graph_jsource.rs::JSOURCE_FAMILY_RULES` includes **16 families**: **AnalysisOnly 7 / ExistingAnalyzer 1 / AwaitingFrontendOrFacts 4 / DownstreamOnly 4**.

| Family / discovery state | J form → pinned jsource evidence | Obligations and baseline fallback / RustJ owner·JX gate |
|---|---|---|
| `ReductionFastPath` / AnalysisOnly | `f/ y` → `ar.c::jtreduce` | empty/singleton/two-item, identity, overflow → Reduce; ExecutionAlgorithm / JX-10 |
| `MeanIdiom` / AnalysisOnly | monadic `(+/ % #) y` → `cf.c::jtfolk`, `ar.c::jtmean` | exclude dyad; rank/FP order → original fork; ExecutionSemantics / JX-10 |
| `WindowAlgorithm` / AnalysisOnly | `f\ y`, `x f\. y` → `ap.c::jtmovfslash` | Scan vs Window, length/NaN/overflow → generic; ExecutionAlgorithm / JX-11 |
| `SearchAlgorithm` / AnalysisOnly | dyadic `i.` / `i:` / `e.` → `vi.c::indexofsub` | first/last/member, CCT/rank/boxed/sparse → sequential; ExecutionAlgorithm / §P·JX-15 |
| `IntervalLookup` / AnalysisOnly | dyadic `x I. y` → `viix.c` | sortedness/type/empty, not monad → baseline; ExecutionAlgorithm / JX-15 |
| `GatherCopyOrView` / AnalysisOnly | `x { y` → `vfrom.c::jtget1cell` | bounds/alias/contiguity → copying; PhysicalPlanner / JX-20 |
| `ReindexCopyOrView` / AnalysisOnly | `$` / `|.` / `|:` → `vf.c` | fill/shape/usecount → materialize; PhysicalPlanner / JX-20 |
| `MapReduceStreaming` / ExistingAnalyzer | `f/@:g` → `va2.c::jtfslashatg` | dense/type/empty/inplace/overflow → generic map-reduce; existing GraphFusion / JX-09 |
| `ResultAssemblyDemand` / AwaitingFrontendOrFacts | box/open/raze → `result.h` | recursive boxes/raze checks/effects → generic assembly; ExecutionSemantics / JX-17/20 |
| `GroupAggregate` / AwaitingFrontendOrFacts | `u/.`, `f//.` → `ao.c::jtkeyct/jtsldot` | CCT/group order/representative/type → generic group; ExecutionAlgorithm / JX-12 |
| `MatrixContraction` / AwaitingFrontendOrFacts | `+/ . *` → `cip.c::jtpdt`, `gemm.c` | rank/Fit/overflow/FP order/sparse → generic dot; ExecutionAlgorithm / JX-13 |
| `GradeRanking` / AwaitingFrontendOrFacts | `/:`, `\:` → `vg.c` | ties/order/type/axis → generic grade; ExecutionAlgorithm / JX-14 |
| `TolerantHash` / DownstreamOnly | tolerant search → `viavx2.c` | CCT nontransitivity/±0/NaN → sequential; ExecutionAlgorithm / §P·JX-15 |
| `SparseAlgorithm` / DownstreamOnly | sparse dot/grade/index/from → `cpdtsp.c` etc. | axes/fill/empty/type → sparse reference; ExecutionAlgorithm / JX-21 |
| `BufferOwnership` / DownstreamOnly | boxed concat/reshape/compress → `vcat.c` etc. | alias/usecount/recursive boxes → allocate; PhysicalPlanner / JX-20/22 |
| `NameLookupCache` / DownstreamOnly | late name/locale → `sc.c::jtunquote` | epoch/locale/reentrancy/invalidation → actual lookup; RuntimeBinding / JX-23 |

**Tracked source families deliberately outside the registry:** §7.4.3 H/K/M includes Cut→Scan→Raze (`cc.c`, JX-17), oblique convolution (`ao.c`, JX-13), char-map LUT (`v.c`, JX-16), boolean/sparse→indices (`cf.c`, JX-16), RNG-pivot order statistics (`vg.c`, JX-14), RNG shape (`vrand.c`, JX-24), Box+Append (`vo.c`, JX-17), explicit `M.` memo (`a.c`, JX-24), Under/Each (`cu.c`, JX-19), bound numeric/deadband (`vx.c/vz.c/va1.c`, JX-18), Amend/Scatter (`am.c`, JX-22), assignment/explicit-definition fast paths (`p.c/cx.c`, JX-22). These are **source evidence backlogs**, not 16 additional executable or exhaustively audited families.

**FW-01 frontend boundary:**
- **Word formation:** `src/tokenizer.rs::scan/parse_word_spans` and `tests/syntax.rs`. Previous F0 differential results are history, **not** a rerun.
- **Enqueue/POS:** `src/primitive.rs`, `src/enqueuer.rs`, `tests/enqueuer.rs` recognize `/.`, `.`, `/:`, `\:`, `;.`, `&.`, `M.`, `?`, `?.`, `!.` as vocabulary POS, **not** as proof of supported derived constructor, runtime, or optimization. Locatives/name-by-value and some numeric payloads remain Unsupported.
- **Derived parser:** `src/parser.rs`, `src/semantic.rs::FunctionEntity`, `tests/semantic.rs` represent portions of `@:`, Hook/Fork, Rank, Insert/PrefixInfix. Key/Dot/Cut/Under/Memo/Grade construction/execution, late NAME, valence/POS and error/effect order require C differential evidence.
- **Source opportunities:** `src/j_graph_jsource.rs::discover` limits Mean to the monad and keeps `E.` window separate from ordinary index search. Candidate discovery does not authorize execution.
- **Regression source added:** [commit e364535](https://github.com/yunskim/RustJ/commit/e36453575430879e4bc546c62350107a3e698e84), `tests/j_graph_jsource.rs::optimization_vocabulary_pos_is_not_a_compiler_optimization_license`. **No Rust default/portable or real pinned C oracle pass yet verified**.

**Observed GitHub Actions failures (2026-10-06):** [Basis compile probe](https://github.com/yunskim/RustJ/actions/runs/37428376593) for [e364535](https://github.com/yunskim/RustJ/commit/e36453575430879e4bc546c62350107a3e698e84) failed during `cargo test` in the **pre-existing `tests/index_ops.rs::member_preserves_cell_shapes_and_empty_query_semantics`**: actual boolean shape `[0]` versus the erroneous expected `[2]`, before the new Jsource test could be accepted. J's `x e. y ↔ (#y)>y i. x` contract means an empty left query stays empty, while an empty right lookup gives false for each left query. [80ad4a7](https://github.com/yunskim/RustJ/commit/80ad4a73b14099431966d2f697fa73f98778e159) corrects the test orientation only; it is **not** full pinned C-binary verification. The [Linux milestone](https://github.com/yunskim/RustJ/actions/runs/37428376492) for e364535 failed `cargo fmt --check` due to pre-existing formatting differences across multiple Rust files; its four separate C-reference jobs succeeded. **Later default/portable and new POS-test acceptance remain unconfirmed as of this note**, so do not mark JX-01, FW-01, FW-04 or the full CI as complete.

**Next acceptance increment:** execute that regression under default/portable Rust, gather real pinned J C POS/derived syntax/error fixtures, then fix **one confirmed semantic discrepancy with one negative regression**. Do not check off JX-01/FW-01 before recording actual commands, environment, source pin, results, and unsupported boundaries.

#### Q.2 FW-01 / JX-01 executed Rust regressions and pending C differential — second migration record (2026-10-06)

**Acceptance: FW-01 [ ], JX-01 [ ], JX-10 [ ] remain open.** [02854b6 CI diagnostic](https://github.com/yunskim/RustJ/actions/runs/37429620894) reproduced a missed Mean candidate: in `(+/ % #) y`, the **outer fork call has one monadic input**, but the internal `g=%` join applies **dyadically** to the two monadic branch results. The previous `discover` checked `Valence::Monad` on this join node and therefore discarded all valid Mean regions. [05dae2d](https://github.com/yunskim/RustJ/commit/05dae2d9c250a466c1f0b89cfdc4b776e72d74c5) now checks original `RegionKind::Fork` and `region.inputs.len()==1`, while verifying that the internal join is dyadic. This changes **analysis discovery only**, not execution or effects. Pinned `jsrc/cf.c::jtfolk` installs `jtmean` as `f1` for `+/ % #`, not as `f2`: the monadic gate remains required.

[268c83f](https://github.com/yunskim/RustJ/commit/268c83f656812b2a9fc951cb91c84e5e6f2a368b) removed temporary `MEAN_DIAG` prints and asserts **monadic outer Fork / dyadic internal join** plus the absence of a dyadic Mean opportunity. [d818a3e](https://github.com/yunskim/RustJ/commit/d818a3e807453327d7cc4cd265dc503acf36d0ec) separately fixed `e.` empty-query tests to distinguish **Boolean membership** from **integer `i.` missing sentinel**, including left empty frame `[0]` and false results for a right-empty lookup.

**Executed CI evidence:** Ubuntu GitHub Actions with Rust stable. [268c83f Basis compile probe](https://github.com/yunskim/RustJ/actions/runs/37432213914): `cargo test`, `cargo test --features portable`, `cargo build --release` **passed**, while Clippy initially failed on pre-existing `src/index_ops.rs:330` manual inclusive range. [dd2c118](https://github.com/yunskim/RustJ/commit/dd2c118de8bb025a2f4fa3f21be9f22063c6be52) expressed the same gate as `(64..=MAX_PREHASH_ITEMS).contains(&items)`; [37432369878](https://github.com/yunskim/RustJ/actions/runs/37432369878) confirms **default + portable tests, release build and Clippy all passed**. Passing the Rust suite is not proof of full J semantic equivalence.

**Remaining independent gates:** [Linux milestone 37432213939](https://github.com/yunskim/RustJ/actions/runs/37432213939) check still fails `cargo fmt --check`, with formatting differences across 13 Rust files. Do not claim entire CI green; record j64/j64avx2 reference jobs separately. [a91dd57](https://github.com/yunskim/RustJ/commit/a91dd5741f298e40782cea9a73801ff6dad0863e) adds `tools/conformance.py::cases` for **ordinary, singleton, empty and framed monadic Mean** C-versus-Rust differentials. Their **new pinned J C results have not yet been accepted**. Record case, C result, Rust result, exact pin, variant, backend and limitations before closing any FW/JX gate. No new specialized execution is enabled.

#### Q.3 FW-01/JX-01 green CI and pinned J C differential evidence (2026-10-06)

**Code/test commit:** [89bbfd0](https://github.com/yunskim/RustJ/commit/89bbfd0e55163c90b5059e90d10b0ba0bd87ded5). All **five jobs** in [Linux milestone 37433098574](https://github.com/yunskim/RustJ/actions/runs/37433098574) completed **successfully**. The check job passed Python tooling tests, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, default/portable `cargo test`, release build, and milestone execution. [Basis compile probe 37433098567](https://github.com/yunskim/RustJ/actions/runs/37433098567) passed as well. Formatting debt was resolved by applying exactly 104 rustfmt CI hunks across 13 Rust source/test files in one semantics-preserving commit.

**Pinned J C differential:** All four Linux reference jobs succeeded against built pinned `jsource` commit `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`. Since [ed56b33](https://github.com/yunskim/RustJ/commit/ed56b336dd198b661cb1e4e80814488b4d890a57), the `conformance.py` and `word_conformance.py` reports explicitly carry the oracle `reference_revision` SHA.

| C variant × Rust backend | Cases | Exact passes | Narrow known deviation | Unexpected failures |
|---|---:|---:|---:|---:|
| j64 × default | 5,384 | 5,383 | 1 | 0 |
| j64 × portable | 5,384 | 5,383 | 1 | 0 |
| j64avx2 × default | 5,384 | 5,384 | 0 | 0 |
| j64avx2 × portable | 5,384 | 5,384 | 0 | 0 |

The one pre-existing J64-specific allowance in `tools/conformance.py::known` is the fixed case `(i.2 3) -"1 0 (i.2 3 4)`: identical values/shape but Float in J64 versus Int in Rust. It is constrained by exact source, dtype and shape, not a blanket exception. **New monadic Mean Fork fixtures** (ordinary, singleton, empty, rank/frame) do **not** match that allowance and passed. The `failed=0` claim is limited to this **supported subset** and the explicitly separated known deviation; upstream's entire J test suite and all numeric/locale/effect semantics were not proven.

**Gates stay open: FW-01 [ ], JX-01 [ ], JX-10 [ ].** The Mean region-candidate fix and J C differential are valuable **sub-gate evidence**, not complete M2 frontend semantics or enabled optimized Mean execution. FW-01 still needs unsupported Key/Dot/Cut/Grade/Under/Memo derived construction, rank/effect/error precedence, and real differential coverage beyond the subset. JX-01 needs a bounded audit of upstream guards and fallbacks per source family; JX-10 needs obligation proofs, guards, sequential-vs-specialized testing, and measured benefit before selection.

**Next increment:** investigate **Key `/.` derived-verb construction** under FW-01/JX-02 against pinned J C parser/POS/valence while preserving the existing Rust vocabulary-only boundary. Keep unimplemented forms `AwaitingFrontendOrFacts`, add positive/negative regression cases, and advance one semantic family at a time.

#### Q.4 First Key `/.` derived-verb construction under FW-01/JX-02 — no execution license (2026-10-06)

**Gates stay open: FW-01 [ ], JX-02 [ ], JX-12 [ ], JX-01 [ ].** This increment promotes `/.` from POS-only vocabulary recognition to **non-executing construction for verb-left `u/.`**. It does **not** implement general Key/Oblique execution, noun-gerund construction, or GroupBy optimization.

**Pinned source contract:** `jsrc/ao.c::jtsldot` constructs one derived verb with **monadic `jtoblique` and dyadic `jtkey`**, all intrinsic ranks `RMAX`. A verb operand is retained directly; a noun gerund is separately decoded using `fxeachv`. This does not authorize arbitrary noun operands. `jtkeyct` performs CCT-sensitive classification via `indexofsub(IFORKEY)` and separately applies the grouped verb; sparse, boxed and specialized reductions have distinct legality and fallback paths. Pinned revision: `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528` (`ao.c`, `cf.c`).

**Incremental acceptance checklist (separate from overall FW/JX closure):**
- [x] Add `AdverbId::Key` to `src/primitive.rs`, remove duplicate `/.` vocabulary-only descriptor, bump `REGISTRY_VERSION` to 9; keep its POS Adverb, with no runtime-execution assertion.
- [x] Reuse the existing verb-left `src/parser.rs::apply_adverb` construction path and preserve `FunctionHead::PrimitiveAdverb(Key)` with its function operand. Reflect `[63; 3]` intrinsic ranks in `src/semantic.rs`.
- [x] Classify the derived verb as **opaque `GraphForm::Modifier`** in `src/j_graph_ir.rs`; never fabricate Reduce, Window or GroupAggregate candidates. Keep `GroupAggregate` at `AwaitingFrontendOrFacts`.
- [x] Positive/negative tests in `tests/semantic.rs::key_derived_verb_keeps_operator_and_operand_without_licensing_execution`, `tests/j_graph_jsource.rs::key_construction_preserves_an_opaque_graph_boundary_without_groupby_selection`, vocabulary/POS and primitive tests. Both monadic and dyadic calls parse but runtime returns explicit unsupported; noun `3/.` is not falsely accepted.
- [x] Six stateful constructor/binding/alias J C differential fixtures added to `tools/conformance.py`. [Linux milestone 37435150581](https://github.com/yunskim/RustJ/actions/runs/37435150581) passed all **five jobs**, with formatting, Clippy, Rust default/portable tests, release build and Python tooling green. [Basis probe 37435150506](https://github.com/yunskim/RustJ/actions/runs/37435150506) also passed.
- [ ] Support upstream noun-gerund `m/.` construction through appropriate `fxeachv`/AR decoding and validate malformed gerunds/error precedence against pinned J C.
- [ ] Implement independent **sequential** monadic Oblique and dyadic Key semantics; cover CCT nontransitivity, group order/representative, rank, empties, boxed/sparse, effects, error ordering and overflow.
- [ ] Prove any specialized execution recipe with provenance, ProofBundle, guards, fallback, invalidation and resource measurements; no GroupBy selection until those obligations pass.

**Executed four-way pinned J C differential:** pin `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`, GitHub Actions Linux, seed `20260926`, randomized rounds `100`. Reports record reference revision plus Rust and J C binary SHA256.

| C variant / Rust build | Cases | Matches | Pre-existing narrowly known deviation | New failures |
|---|---:|---:|---:|---:|
| `j64` / default | 5,390 | 5,389 | 1 | 0 |
| `j64` / portable | 5,390 | 5,389 | 1 | 0 |
| `j64avx2` / default | 5,390 | 5,390 | 0 | 0 |
| `j64avx2` / portable | 5,390 | 5,390 | 0 | 0 |

The existing J64 deviation is Q.3's unrelated Rank result dtype issue, **not a Key failure**. The six new cases cover **constructor/assignment/alias behavior only**, not evaluation of `x u/. y` or `u/. y`; no claim about the entire upstream J test suite is made.

**Commits:** [78ba45a](https://github.com/yunskim/RustJ/commit/78ba45a9bcbc47e15ef57299a77c7131be523cde) Key constructors and regression fixtures; [de5c4dc](https://github.com/yunskim/RustJ/commit/de5c4dc10b11570836794fda42d3adf84a0c10bf) formatting; [d8db5f9](https://github.com/yunskim/RustJ/commit/d8db5f95991f6116e672d41eaee6876d5faa3f19) opaque Graph negative test; [a4f4dd0](https://github.com/yunskim/RustJ/commit/a4f4dd078c26d4fe157357dbb65f37b56375dff2) final formatting.

**Next priority:** Keep the unsupported frontend inventory explicit and implement **FW-02's independent sequential `i.` reference**, separating semantic validation from existing optimized search strategy. Only later use Key/Reduce as the second independent operation family for FW-10/JX-08 before extracting common optimization interfaces.

**Operating rules.** Each JX gate requires **(1) pinned C source and guards → (2) J semantics/support boundaries → (3) graph provenance and candidates → (4) per-obligation proof/guard/fallback → (5) independent Rust reference, negative tests and real C differential → (6) target/resource/measured-cost decision**. Keep [ ] without actual execution evidence. Prefer **one semantic change plus one related regression/counterexample** at a time. A regression or upstream drift invalidates affected proofs and reopens prior FW gates. For each completed row record **JX-ID / code commit / commands and environment / passed-failed-ignored / jsource commit and executed oracle scope / fallback-negative results / measured metrics / known gaps / next gate**. Next actionable work remains **JX-01 source coverage and FW-01 M2**, not enabling new specializations.

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

## 12.2 External adapter boundary contract

An external adapter is a projection of a verified RouteRegion/Logical IR, not a replacement for RustJ semantics. Its input includes region live-ins/outs, ordered operations and SemanticChecks, effect/error edges, provenance, discharged legality evidence/guards, resolved TargetContext, and logical representation requirements.

Capability queries must distinguish operation/ExecutionBasis/valence, dtype, rank/shape/dynamic-shape conditions, representation/layout preconditions, numeric/tolerance/reassociation policy, effect/token support, error/check representation, alias/mutation, and async/completion semantics. `supports Add` alone is not a sufficient contract.

An adapter result must retain a verifiable projection record: adapter/schema identity, source RouteRegion/A3 provenance, emitted external operations/module, host-side checks, mapped effect/token edges, BridgeRequirements, external handles, completion/ownership contract, and failure classification.

Every A3 SemanticCheck must either run on the RustJ side before launch in the same observable order, lower to an external form proven to preserve the same J error class/precedence, or make the region ineligible for that route. Backend traps/assertions/compile failures are not automatically J Domain/Rank/Length errors.

Pure regions may require no token. Stateful/effectful regions may be projected only when equivalent ordering/resources can be represented. Representation/layout/device choices remain bridge/physical concerns and must not change logical dtype/shape/atom order.

The adapter round-trip verifier checks that every source semantic operation maps to translated work, a retained host check, or an explicit bridge/effect action; live-in/out contracts survive; no checks/effect/error edges are dropped or reordered; emitted forms match declared capabilities; output ownership/completion precedes consumer use; provenance maps back to A3/J Graph/source; and unsupported partial modules are never returned as executable success plans.

Keep failure classes distinct: AdapterUnsupported, AdapterCompileFailure, AdapterRuntimeFailure, and actual JSemanticError. Pre-execution failures may choose a verified alternate route under the no-replay contract; failures after committed effects/transfers do not trigger automatic replay.

This is an implementation gate for future MLIR/StableHLO/ArrayFire/library routes. It does not claim that a production external adapter exists today, and it does not resume CUDA work.

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


<a id="out-of-core-io-contract"></a>

## 13.2 Slow I/O / out-of-core array execution (2026-10-06; design candidate, not implemented)

**Goal.** Evaluate arrays/NN weights larger than RAM/GPU capacity without changing J-visible values, type/shape/atom order, Rank zero-frame prototype behavior, errors or effects. Avoid unnecessary source reads, then overlap bounded I/O with compute. This is a physical planning/scheduling/runtime extension, governed by the canonical §8.5 contract, **not a new J language construct, nor a prerequisite for the first M4 native CPU vertical slice**. The single acceptance checklist is [§17 IO](#out-of-core-io-checklist).

| Source | What is borrowed | What is NOT implied |
|---|---|---|
| [Jsource jmf.ijs](https://github.com/jsoftware/jsource/blob/master/jlibrary/addons/data/jmf/jmf.ijs) | mapped J noun, read-write/read-only/COW maps, header/shape and unmap reference constraints | mmap does not provide automatic async prefetch; non-jmf typed boxed mapping is rejected, but JMF-backed boxed regression fixtures exist; scope must be tested per route |
| [Jsource xf.c](https://github.com/jsoftware/jsource/blob/master/jsrc/xf.c), J `1!:11`/`1!:12` | indexed byte-range read/write, sequential baseline based on `fread/fwrite` | effectful foreign I/O is not a pure logical scan and must not be silently rewritten |
| Jsource in-place/alias machinery | ownership-proved buffer reuse and copy elimination | mapped mutation is not automatically safe in-place reuse |
| Jd (J data add-on) | Verified on-demand file-backed columns and partition-column selective reads | Borrow storage/layout and pruning techniques only; do not assume arbitrary J effects or queries can be reordered |
| [DuckDB async I/O](https://duckdb.org/2026/07/31/asynchronous-io) | independent async blocking-I/O pool, read-ahead, memory-governed queued jobs, park/resume | do not copy a full database engine |
| [Polars lazy](https://docs.pola.rs/user-guide/lazy/optimizations/) | projection/predicate/slice pushdown, common subplan scan reuse | only with J-compatible access/effect/error proofs; not arbitrary verbs/reductions |
| [Apache Arrow Scanner](https://arrow.apache.org/docs/python/generated/pyarrow.dataset.Scanner.html) | distinct batch/fragment read-ahead, bounded batches and metadata pruning | no blanket conversion to Arrow representation |
| [Ray Data streaming](https://docs.ray.io/en/latest/data/data-internals.html) | block streams, bounded queues, backpressure/spilling accounting | shuffle/reduce barriers remain |
| [DeepSpeed ZeRO-Infinity](https://www.deepspeed.ai/tutorials/zero/) | NVMe/CPU/GPU staging and transfer/compute overlap | CUDA work remains deferred |
| [FlexGen (ICML 2023)](https://proceedings.mlr.press/v202/sheng23a.html) | version-stable weight reuse and layer/batch-block schedules under capacity and latency/throughput constraints | must not reorder visible J effects, late bindings, or failures |
| TensorFlow `tf.data` | prefetch+parallel data preparation as an additional comparison candidate | not proof of full-J compatibility |

**Ownership and stage contract.** J Semantic/J Graph IR owns J semantics, data+effect dependencies and unknown/opaque facts, but not file offsets, chunk sizes or queues. Verified Logical IR may produce guarded/witnessed byte-access and reuse candidates only if exact J semantics permit. Native Physical Planner/Schedule owns storage placement, byte ranges, materialization, chunk and job boundaries, prefetch, transfer, memory budget, cost and completion edges. Runtime initially owns synchronous `read_at`/`write_at` and chunk iteration; later it owns request pending/ready/error/cancel, exact lease lifetime, finite queues, and backpressure. An external adapter must declare the actual I/O/effect/ownership capabilities and decline unsupported routes.

**Keep identities separate.** `ValueId` is logical SSA; `StateResource` is a mutable semantic resource; `BufferId/BufferLease` owns runtime memory; proposed `StorageObjectId/Version` identifies external backing bytes and consistency; proposed `IoRequestId/CompletionToken` identifies I/O completion. The latter names are not accepted Rust APIs. Semantic `StorageRequirement` is not Physical `MaterializationDecision`.

**Legality.** Preserve J Rank/CellApply empty-frame/prototype, boxed/sparse, tolerance, error precedence, late-bound names and observable foreign I/O. Use checked offset/extent/shape arithmetic. Define short-read, EOF, permission, stale-version, non-atomic file update and cancellation behavior. Unknown access or mutability is an optimization barrier; no speculative I/O that reorders an observable failure/effect, no transparent replay after committed effects. A zero-byte data read never excuses required zero-cell J shape/type derivation.

**Execution route.** Baseline is portable synchronous file/chunk CPU execution. Proven projection/slice/range pruning comes next. Then memory-reserved bounded async read-ahead, double buffering, `Read(n+1)` overlapped with `Compute(n)`, explicit dependency/completion and release; memory pressure reduces depth. Use a scheduling/weight-reuse candidate only when input data is read-only/version-stable and reordering is semantically legal. mmap competes with `read_at`, not universally replaces it; page faults and cache behavior are measured. `io_uring`, remote storage, direct I/O and device DMA remain optional later capabilities.

**Writes/checkpoints.** A J foreign file write, shared mapped mutation and an optimizer checkpoint have different visible effects. Checkpoint design needs explicit immutable version capture, temporary write, platform-specific flush/durability, publication/recovery, cancellation and partial-write behavior. Never declare save success before the required durability level, or change J-visible effect/error timing silently.

**Cost and evidence.** Keep `ResourceEstimate` (peak/resident/inflight bytes, handles, queue budget) separate from `CostEstimate` (bytes, seeks/requests, bandwidth/latency, compute time, transfer/overlap). Compare cold vs warm cache, byte counts, wait, CPU compute, page faults, peak+retained memory, spilling, throughput *and* per-input latency. A feature exists only after code+independent semantic/negative tests+recorded commands and J C oracle coverage, as specified in the `17 checklist.


## 13.3 Independent source re-audit: J storage libraries, physical file formats, model loaders (2026-10-06)

**New gap identified.** The first I/O plan concentrated on async read-ahead; separate contracts for physical storage encoding, read-chunk vs write-shard granularity, mapped SIMD tails, remap/refcounts, and cache invalidation were under-specified. Evidence from upstream projects is not evidence of RustJ feature completion. All of the following is proposed pending [§17 IO acceptance](#out-of-core-io-checklist).

| Primary source | Verified mechanism | Adopt or defer |
|---|---|---|
| [J jfiles/keyfiles](https://github.com/jsoftware/jsource/tree/0a5101cfdd834b23a0b89d455e4f327310520a08/jlibrary/addons/data/jfiles) | serialized component storage using byte-range indexed read and keyed components | Keep serialized arbitrary J noun distinct from typed dense mmap, avoid copying full Jfiles format |
| [Jd column.ijs](https://github.com/jsoftware/data_jd/blob/0492991263a05bafa84ceca15f0f8249cfc62dcf/base/column.ijs) | on-demand column mapping, remap/resize; source calls out multi-process reference-count dangers | Verify lease/alias/remap/ownership safety, not a universal global DB lock |
| [Jd api_read.ijs](https://github.com/jsoftware/data_jd/blob/0492991263a05bafa84ceca15f0f8249cfc62dcf/api/api_read.ijs) · [ptable tutorial](https://www.jsoftware.com/jd_tuts.html) | partition-column-based table pruning | Use as selective array/partition-access inspiration only with J access, effect and error witnesses |
| [Jd jmfx.ijs](https://github.com/jsoftware/data_jd/blob/0492991263a05bafa84ceca15f0f8249cfc62dcf/base/jmfx.ijs) | padding file endings to avoid SIMD overfetch faults; historical 4-KiB page assumption | Cautionary case: require safe vector tails, checked mapping spans, actual OS granularity and lifecycle validation; do not copy hard-coded padding |
| [Zarr 3](https://zarr.readthedocs.io/en/stable/user-guide/arrays/) · [HDF5 cache](https://docs.h5py.org/en/stable/high/file.html) | independent read chunk and write shard layout, chunk cache/eviction | Physical shape and IO granularity need not equal J logical shape; benchmark amplification and per-workload layouts |
| [Safetensors](https://github.com/safetensors/safetensors/blob/main/README.md) | tensor dtype/shape/byte offsets and optional slice access, zero-size payload | Validate metadata/offsets/endian/empty/scalar. Do not make a new canonical RustJ disk format mandatory |
| [llama.cpp loader](https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md) | mmap vs no-mmap/mlock/direct I/O/NUMA/lazy tensor row policies | Choose storage load strategy from page faults, RAM pressure and access/reuse costs, not global mmap dogma |
| [IREE Stream dialect](https://iree.dev/reference/mlir-dialects/Stream/) | parameter.load/read/gather/write, AsyncTransfer and Timepoints encode loading/storing parameter byte ranges plus readiness and resource lifetime | **Most direct IR precedent:** read-only weights can use Physical ParameterRead/Load → Await → consumer; checkpoint write still needs a separate durability contract. Not equivalent to observable J file-foreign effects |
| DuckDB, Polars, Arrow, Ray, ZeRO-Infinity, FlexGen | skip unnecessary bytes, bounded job/batch read-ahead, backpressure, staged weights and reuse | Physical optimization order: prune bytes → version and alias safety → bounded scheduling → overlap → proven reuse |

**Corrected JMF boxed scope.** [jmf.ijs](https://github.com/jsoftware/jsource/blob/0a5101cfdd834b23a0b89d455e4f327310520a08/jlibrary/addons/data/jmf/jmf.ijs) rejects *non-JMF typed boxed* mapping, while [JMF-backed boxed tests](https://github.com/jsoftware/jsource/blob/0a5101cfdd834b23a0b89d455e4f327310520a08/test/gmbx.ijs) exist and [mbx.c](https://github.com/jsoftware/jsource/blob/0a5101cfdd834b23a0b89d455e4f327310520a08/jsrc/mbx.c) says “not supported.” Neither blanket all-boxed-supported nor all-boxed-unsupported is justified without a C-oracle check of the specific representation and operations.


**IREE Stream dependency precedent:** `stream.async.parameter.load/read/gather/write`, `stream.async.transfer`, and `stream.timepoint` make parameter byte ranges, readiness, and resource lifetime explicit in a physical schedule. Speculatable parameter loads are not a license to reorder observable J file foreign effects; preserve availability, cancellation and error ordering. Reference for IO-13–18 and IO-20. [Official Stream dialect](https://iree.dev/reference/mlir-dialects/Stream/).

**Clarified abstraction.** Distinguish logical ValueId, physical external StorageObject/Version, StorageEncoding (typed contiguous, typed chunked, serialized components, external adapters), ReadChunk, WriteShard, BufferLease and IoCompletion. These are concepts, *not* committed Rust APIs. Observable J foreign-file I/O cannot be silently rewritten as a pure read of versioned immutable array backing. An empty data region can require no file bytes and still require J Rank fill-cell/shape inference. Cache keys need source/version/range/encoding; memory budgeting must account for decoded/pinned/inflight/kernel buffers while reporting OS page cache/RSS separately.


<a id="io-framework-execution-comparison"></a>

## 13.4 Framework I/O optimization mechanisms and RustJ graph-to-physical scheduling (2026-10-06)

**Five independent concerns.** Efficient out-of-core execution combines (1) skipping unnecessary reads, (2) choosing physical read/storage granularity, (3) overlapping reads with computation, (4) reusing already loaded bytes and (5) memory, readiness and failure control. These concerns are owned by different stages. J Semantic/J Graph/Verified Logical IR defines observable J semantics, access and legality proof; Physical Planning/Scheduling/Execution chooses byte ranges, chunks, transfers, buffer lifetime, inflight requests and resource budgets. The table is a source-informed set of design candidates, **not evidence of implemented RustJ functionality**.

| Framework / primary documentation | Mechanism and owning layer | RustJ application and limits |
|---|---|---|
| [Polars lazy optimizer](https://docs.pola.rs/user-guide/lazy/optimizations/) | Predicate, projection and slice pushdown into scans; common subplan/file scan elimination in logical planning | IO-09–12: only prune bytes when `AccessRelation` and witnesses preserve J semantics; otherwise opaque fallback for Rank, dynamic lookup, errors and effects |
| [DuckDB async I/O announcement (2026-07-31)](https://duckdb.org/2026/07/31/asynchronous-io) | Parquet row-group / CSV scan jobs and range fetch tasks; separate `REGULAR` compute and primarily blocking `ASYNC` I/O pools; parked consumers resume at completion. Read-ahead depth negotiates with the temporary-memory manager | IO-13–17: bounded requests, readiness/wakeups, adaptive prefetch under pressure. Announced development/version behavior must not be assumed present in every release |
| [IREE Stream dialect](https://iree.dev/reference/mlir-dialects/Stream/) | `stream.async.parameter.load` creates a resource; `read` fills an allocation; `gather` assembles several parameter archive ranges. Timepoints and await dependencies encode availability and ordering in the physical resource graph | IO-13–18/20: physical `ParameterLoad/Gather → Await → Consumer` for immutable weights; IREE's hoistable parameter loads do not authorize speculation of observable J `1!:` file foreign calls |
| [Apache Arrow Scanner](https://arrow.apache.org/docs/python/generated/pyarrow.dataset.Scanner.html) | Separate `batch_readahead` and `fragment_readahead` scan concurrency | IO-14–16: independently bound read-ahead at chunk/fragment levels and account for decoded buffers |
| [Ray Data internals](https://docs.ray.io/en/latest/data/data-internals.html) | Stream block references through operator queues; schedule only when resources and backpressure permit; spill when needed | IO-15–17: bounded queues, slow-consumer pressure, distinction between spill, workers' working memory and nonstreamable barriers |
| [Zarr arrays/sharding](https://zarr.readthedocs.io/en/stable/user-guide/arrays/) / [HDF5 chunk cache](https://docs.h5py.org/en/stable/high/file.html) | Chunk/shard layout and decoded chunk caching trade read amplification, file count and reuse | IO-26–29: `LogicalShape`, `ReadChunk` and `WriteShard` remain independent. Small slices may still require decoding entire chunks |
| [llama.cpp](https://github.com/ggml-org/llama.cpp) | Mapped vs unmapped model loading and residency tradeoffs | IO-23/30: measure page faults, cold/warm cache, resident memory, local/remote throughput rather than mandate mmap |
| [DeepSpeed ZeRO](https://www.deepspeed.ai/tutorials/zero/) / [FlexGen](https://proceedings.mlr.press/v202/sheng23a.html) | Parameter/optimizer-state offload and prefetch between NVMe, CPU and GPU; layer/batch scheduling to reuse weights at throughput/latency tradeoffs | IO-18–20: only legally reorder loads of immutable/version-stable weights; mutable gradients, checkpointing and J effects need different contracts |
| [TensorFlow tf.data guide](https://www.tensorflow.org/guide/data_performance) | Input prefetch and parallel-map overlap producer and consumer | IO-14–16: helpful producer/consumer precedent, not permission to reorder arbitrary J cell/verb evaluation |
| [PyTorch Distributed Checkpoint](https://pytorch.org/docs/stable/distributed.checkpoint.html) | Stage mutable state and perform asynchronous persistence | IO-18: snapshot/version/write completion/publish/recovery, not success merely because an asynchronous operation was submitted |

**Two disk-backed weight layers — meaning versus realization.** The logical array computation is `X → MatMul(W1) → Activation → MatMul(W2) → Y`. Provided W1 and W2 are proven immutable/version-stable physical inputs, the physical scheduler may overlap a read of W2 with computing Layer 1. It cannot start Layer 2 before both the activation result and W2's readiness token are available. This is a candidate, not the current implementation.

~~~text
Logical (J semantics)
X ---> MatMul(W1) ---> Activation ---> MatMul(W2) ---> Y

Physical candidate
Reserve W1 -> Read W1 -> Ready W1 -> Compute L1 -> Activation --+
Reserve W2 -> Read W2 -> Ready W2 ------------------------------+
                                                               |
                                                          Compute L2 -> Y
Read W2 may overlap Compute L1, within the resource budget.
Compute L2 awaits BOTH Activation and Ready W2.
Release a buffer only after its last user and all pending I/O/transfers complete.
~~~

**Why these optimizations compose rather than replace one another.** Polars minimizes the requested bytes; IREE makes physical data movement/readiness explicit; DuckDB/Arrow/Ray manage request scheduling and backpressure; DeepSpeed/FlexGen seek profitable placement and reuse. Read-ahead can hide wait time but does not intrinsically reduce bytes, and merging many small byte ranges trades fewer requests for potential over-reading.

**Semantic and failure guardrails.** Distinguish (A) an internal read of an immutable/versioned storage object, (B) observable J foreign I/O such as `1!:11`/`1!:12`, and (C) mutable weight/checkpoint persistence. Only A admits a proof/guard-authorized speculative prefetch or pruning. B preserves J effect and error ordering; C additionally needs snapshot, version, commit, publication and durability semantics. A zero-byte read does not eliminate J Rank zero-cell fill/prototype, dtype, shape or error obligations. Boxed/sparse, dynamic NAME/Rank, alias changes, stale files, EOF/short reads and premature exposure of speculative failures remain negative-test barriers.

**Resource and cost accounting.** Bound queued + in-flight + decoded + pinned + temporary + output/retained buffers; report OS page cache and allocator RSS independently of runtime reservations. For cold/warm runs record actual bytes and I/O requests, seek/latency, blocking wait, CPU/GPU work, page faults, overlapped time, peak/retained memory, spilling, batch throughput and single-call latency. Approval order stays *semantic conformance → resource safety → measured cost-based route selection*.

**Existing acceptance checklist mapping (no new checklist):** IO-09–12/29 for pruning and cache; IO-05–08/26–28 for storage and chunk/shard representation; IO-13–17/20 for async, transfer and backpressure; IO-18–19 for weights/checkpoint; IO-21–24/30 for comparative measurements; IO-01–04/25 for original-source and semantic contracts. Continue using the [IO-01–IO-30 single acceptance ledger](#out-of-core-io-checklist); writing this design section **does not advance implementation acceptance beyond 0/30**.


<a id="unified-data-movement-contract"></a>
## 13.5 Unified memory I/O and disk I/O planning contract — no new mandatory IR layer (2026-10-06)

**Decision.** Treat memory access/copies, prospective CPU↔GPU transfers, and disk/file-backed range reads/writes as **joint data-movement planning and scheduling concerns of the existing Physical Planner / Physical Execution Plan**. Do not introduce a stand-alone `Data Movement IR` at this stage. Retain independently owned access-region proofs, effect/dependency contracts, and physical location/transfer/readiness/lifetime state at their established boundaries. Reconsider a separate resource/stream execution IR only when at least two concrete asynchronous or mixed-route use cases demonstrate that the existing Physical Plan cannot safely encode dependencies, buffer lifetime, or scheduling. This does **not** collapse the existing J Graph IR → Verified Logical Execution IR → Physical Plan boundaries.

| Existing stage | Owned facts and decisions | Must not own |
|---|---|---|
| J Semantic IR / J Graph IR | Source topology; ValueId, Rank/CellApply, empty-frame semantics, source provenance; optional pass-local access-candidate sidecars | BufferId, file offsets, DMA, concrete transfers, physical layouts, or EffectSummary/committed selection embedded into GraphFacts |
| Verified Logical Execution IR / analysis | Observable J file/namespace/state effects; error, ordering and guard dependencies; value/effect liveness; logical AccessRelation with Unknown/Proven/Guarded evidence | Reclassifying arbitrary file I/O as a pure array load or approving unsound read omission/reordering |
| Existing Physical Planner / Representation / Schedule | Distinct StorageObject/Version and BufferId/Lease identities, physical regions (buffer slices vs file byte ranges), Read/Write/Copy/Transfer/Materialize/Release, completion/readiness edges, placement/layout and byte/resource/cost estimates | Mandating a physical copy for every access, unifying file/memory semantics, or selecting unsupported device transfers |
| Executor / Backend | Independent synchronous CPU reference for read_at/write_at/buffer copies first; later separately verified async tokens, queues/backpressure, target implementations | Releasing buffers before transfer completion, replaying visible file effects, or claiming currently deferred GPU implementation |

**Shared interface, distinct meaning.** `AccessRegion` identifies which logical elements are needed. Concrete `BufferSlice` and `FileByteRange` are distinct kinds of physical region; they must not be treated as one alias domain, address space or failure contract. Shared analyses may inspect interval/producer/consumer, placement, lifetime, alias constraints, dependency, capability and cost. Continue distinguishing logical `ValueId`, mutable `StateResource`, external `StorageObjectId/Version`, physical `BufferId/BufferLease`, and `IoRequestId/CompletionToken`. These are proposed conceptual names, not committed Rust APIs.

**Compile time versus runtime.** Compile-time proofs may determine stable shape/dtype/access patterns, users/lifetimes, removable intermediates/copies, buffer-reuse opportunities, required regions and tentative streaming/transfer plans. Runtime must still establish actual file bytes, external modification, EOF/permissions, data sizes, available RAM/GPU capacity, cache/bandwidth conditions and completion, applying explicit guards and hard resource limits. Keep memory bytes, file requests/seeks, transfers, launch/synchronization, overlap, peak/inflight/retained bytes and cold/warm latency as distinct cost dimensions. Unknown is neither zero cost nor permission to reorder.

**Effect safety.** Distinguish (A) an internally accessed, proven immutable/version-stable backing object, (B) J-observable `1!:` foreign file I/O, and (C) mutable-state/checkpoint publication. Even A may expose failures; speculative scheduling must separately prove that user-visible error timing/order is preserved. An unused result of B does not license deleting file existence/permission/error effects. An asynchronous write submission for C is not a durability-completion guarantee. A zero-byte/empty-frame result still has J virtual fill-cell, dtype/shape and applicable error obligations.

**External framework precedents (not imported semantics).** [IREE Stream](https://iree.dev/reference/mlir-dialects/Stream/) provides physical resource, async transfer, parameter/file read/write and readiness scheduling. [MLIR Bufferization](https://mlir.llvm.org/docs/Bufferization/) and [Memory Effects](https://mlir.llvm.org/docs/Rationale/SideEffectsAndSpeculation/) motivate separating logical values from buffers and using resource-aware effect interfaces. [XLA GPU architecture](https://openxla.org/xla/gpu_architecture) shows staged fusion/buffer assignment/layout/transfer planning, not general J file I/O. [TVM](https://tvm.apache.org/docs/) informs device placement; [DataFusion](https://datafusion.apache.org/) informs file scan and proven filter/projection pushdown. None automatically enforces full-J foreign error ordering or Rank prototype semantics.

**Trace example.** A proven immutable `File(A) → Slice → Elementwise → Consumer` may lower to a physical byte-range read followed by fused CPU work once an access witness is valid. An unused `1!:1`/`1!:11` read cannot be eliminated solely by dead-value analysis when file-open/EOF/errors are observable. CPU↔GPU copies are candidates for the same physical planner, while CUDA execution remains deferred.

**Single existing acceptance ledger.** Do not create DM-* work items. Refine IO-03 (stage and identity/effect interface), IO-09 (access-region proof), IO-13/14 (sync-to-async readiness), IO-17 (effect/failure/lifetime), IO-20 (shared memory/file/transfer physical cost planning), and IO-22 (independent three-way and negative tests) in the existing [IO checklist](#out-of-core-io-checklist). Reuse FW-05–09 provenance/guard and DB effect evidence as prerequisites. **Status: architecture decision recorded; no implementation or execution evidence; IO acceptance remains 0/30. Not a prerequisite for M2 frontend convergence or the first M4 native CPU vertical slice.**

<a id="io-a-source-audit"></a>
## 13.6 IO-A pinned source cross-audit and executable witnesses (2026-10-06–07)

**Scope/status.** Reviewed pinned implementation source for IO-01/IO-25 and identified negative tests required by IO-02. **Source inspection is partial progress, not acceptance**: file-foreign fixtures have run against pinned J C binaries, but mapped/Jd and broader effects have not been verified, so IO-01/02/25 remain [ ] and the I/O implementation ledger stays at 0/30.

| Pinned source / location | Directly observed in source | Still to prove for RustJ |
|---|---|---|
| [jsource `xf.c` @0a5101c](https://github.com/jsoftware/jsource/blob/0a5101cfdd834b23a0b89d455e4f327310520a08/jsrc/xf.c), `jtjiread/jtjiwrite/jtixin` | `1!:11` opens/inspects the file before validating start and length against file size; read enforces `j≤size, j+length≤size, length≥0`. `1!:12` passes null for output length, so it checks nonnegative starting index rather than imposing the same read end-bound, then writes. Negative start offsets are adjusted by file size. `jtrd/jtwa` use synchronous `fread/fwrite`. | Actual precedence of open/index/permission errors, EOF, short I/O, writing past EOF, concurrent mutation, and no replay of visible effects. Source alone does not establish atomicity or concurrent-truncation behavior |
| [jsource `jmf.ijs` @0a5101c](https://github.com/jsoftware/jsource/blob/0a5101cfdd834b23a0b89d455e4f327310520a08/jlibrary/addons/data/jmf/jmf.ijs) and [`gmbx.ijs`](https://github.com/jsoftware/jsource/blob/0a5101cfdd834b23a0b89d455e4f327310520a08/test/gmbx.ijs) | Modes 0/1/2 distinguish RW/RO/COW; live refs may prevent unmap. `additem` rejects type 32 boxed, whereas `gmbx.ijs`, despite its mapped-boxed label, contains `'' -: q` and `'' -: r` assertions, not a proof that arbitrary boxed payload read/write works. | Format/type/mode-specific boxed support via executable oracle; never equate COW with committed shared write |
| [data_jd `column.ijs` @0492991](https://github.com/jsoftware/data_jd/blob/0492991263a05bafa84ceca15f0f8249cfc62dcf/base/column.ijs) and [`jmfx.ijs`](https://github.com/jsoftware/data_jd/blob/0492991263a05bafa84ceca15f0f8249cfc62dcf/base/jmfx.ijs) | Mapping/remapping and cross-process reference-count hazards are documented, with DB locks providing a restricted usage assumption. The SIMD-tail padding workaround assumes `PAGESIZE=:4096`. | OS-independent checked spans/masked tails, real page size and safe remap/lease tests; do not generalize DB locks to J semantics |
| [jsource `jfiles.ijs` @0a5101c](https://github.com/jsoftware/jsource/blob/0a5101cfdd834b23a0b89d455e4f327310520a08/jlibrary/addons/data/jfiles/jfiles.ijs) and [data_jd `api_read.ijs` @0492991](https://github.com/jsoftware/data_jd/blob/0492991263a05bafa84ceca15f0f8249cfc62dcf/api/api_read.ijs) | jfiles `j_read` uses `3!:2 @ (1!:11)`, reading serialized components from offset-indexed directory entries. Jd `readptable` selects underlying partitions using predicate results. | Distinguish typed contiguous backing from serialized components. Do not generalize Jd's selective reads into arbitrary J/foreign-I/O rewrites |

**Remaining executable fixtures (not yet fully run).** With a fixed J binary and controlled temporary files, observe normal/edge `1!:11` ranges, negative offsets, `1!:12` writes beyond EOF, nonexistent files, permissions, concurrent changes/short reads, named files versus numeric handles, error precedence and close/unlock behavior. Separately test JMF RW/RO/COW and live-reference unmap, exact mapped-boxed payload cases, resize/remap, and Jd/jfiles partition/keyed component access. Derive expected observable behavior from the executable C oracle rather than guessing it from source. Record pin, command/environment, pass/fail/unsupported and blockers in the existing IO ledger.

**IO-A executable diagnostic harness (2026-10-06).** Added 15 isolated temporary-file C-oracle cases in [tools/file_io_audit.py](tools/file_io_audit.py): whole/range reads, file size, negative offsets, zero-length EOF reads, read bounds, partial/beyond-EOF writes, missing files and a discarded read whose failure remains observable. [tools/test_file_io_audit.py](tools/test_file_io_audit.py) checks fixture uniqueness, quoted names, temporary-path isolation and mismatch reporting without a J binary. [Linux CI](.github/workflows/linux.yml) now builds the pinned `j64/j64avx2 × default/portable` J libraries and records non-acceptance diagnostic JSON as an artifact; the default diagnostic mode reports expectation mismatches rather than converting them into a passing J-semantic gate. The audit source pin `0a5101cf` and CI oracle revision `13994ffa` have identical Git blob SHAs for `xf.c`, `jmf.ijs`, `gmbx.ijs` and `jfiles.ijs`. Do not predeclare J error precedence, platform-specific file-hole contents or mapped-boxed behavior before executing oracle fixtures. **IO-01/02/25 remain unchecked; no accepted RustJ I/O implementation is implied.**

**Initial CI diagnostic correction (non-acceptance).** [Linux CI 37470524210](https://github.com/yunskim/RustJ/actions/runs/37470524210) initially showed 9 of 15 expected outcomes matching and six requiring review on both `j64` and `j64avx2`. All six initially reported `length error`: this was traced to **incorrectly pre-boxing the file name** in the indexed-foreign fixture, not an established J semantic disagreement. The [J Files manual](https://www.jsoftware.com/help/dictionary/dx001.htm) distinguishes whole-file `1!:1 <'name'` from indexed `1!:11 'name';offset length` and `x 1!:12 'name';offset`. [Fix 843ae79](https://github.com/yunskim/RustJ/commit/843ae79d922d57f40d8378f56d70e9cbb251b41a) uses the unboxed name on indexed calls and prevents `length/rank/syntax` errors from counting as real missing-file effects. [Unit-test update bfcfe7e](https://github.com/yunskim/RustJ/commit/bfcfe7e9e1dffaefa095a837f641c18fdda7c1fe) follows. Pending re-execution, 9/15 is not J-semantic acceptance and IO-01/02/25 and the overall 0/30 remain unchecked.

**Executed pinned J evidence (2026-10-06).** In [Linux CI 37470839455](https://github.com/yunskim/RustJ/actions/runs/37470839455), the general `check` job passed and the observed `j64/default`, `j64/portable`, and `j64avx2/default` runs each matched **15/15 C file-foreign fixture expectations, zero review items**. `j64avx2/portable` was still running at this checkpoint and is not counted. [Workflow commit 3d476f6](https://github.com/yunskim/RustJ/commit/3d476f6f4f8bd1c925e2216ee856cfdfcd5fc9d2) enables `--gate` to enforce these C-oracle fixture expectations in subsequent runs. This **J reference fixture gate** is not independent RustJ three-way verification and does not prove JMF/Jd/boxed or async I/O; IO-01/02/25 and the 0/30 acceptance ledger remain unchecked.

**Expanded IO-02 ordered J-reference witnesses (2026-10-07; partial oracle evidence).** [tools/file_io_audit.py](tools/file_io_audit.py) now adds **six ordered-effect cases / 14 J steps** alongside the prior 15 independent file-foreign fixtures. With one J interpreter and one isolated temporary file per case, it inspects file bytes after *every* JDo: (1) a discarded write result still has an observable effect, (2) a failing right read prevents a left write in J evaluation order, (3) a subsequent error does not roll back an earlier completed write, (4) a bad indexed write preserves a preceding completed write, (5) append followed by tail read, and (6) truncate followed by full read. [Offline tests](tools/test_file_io_audit.py) inject a forbidden early write and ensure it is detected. In [CI 37537959953](https://github.com/yunskim/RustJ/actions/runs/37537959953), the general `check` job and pinned `j64/default` oracle passed with **15/15 independent and 6/6 ordered (14 steps), zero mismatches**; other matrix variants were unresolved at the time of this checkpoint. This is a **J C-reference semantic witness**, not native/optimized RustJ I/O equivalence or mapped/foreign coverage: IO-02 and overall IO remain **[ ] / 0 of 30 accepted**.

**Next IO-02/IO-25 executable slices (within the existing ledger).** (a) Extend reference fixtures to named versus numeric handles and permissions/close/flush, EOF/short reads and concurrent truncation for `1!:1/2/3/4/11/12`; (b) independently verify loading the pinned J standard/add-on library before smoke-testing JMF RW/RO/COW, live-reference unmap/resize and typed/boxed variants; (c) enable Jd/jfiles executable probes only after their fixture and pinned dependency setup is demonstrably reproducible. Do not classify a missing J add-on bootstrap as J semantics, or promote the C-only 15+6 tests to RustJ implementation acceptance.

**Pinned C matrix confirmation for IO-02 (2026-10-07).** [Linux run 37537959953](https://github.com/yunskim/RustJ/actions/runs/37537959953) completed successfully in the generic check job and all four `j64/j64avx2 × default/portable` reference jobs. Each recorded **15/15** independent C file-foreign fixtures and **6/6** ordered-effect cases (**14 JDo steps**), with zero cases requiring review. This is a repeated C-source semantic witness, not RustJ three-way I/O execution equivalence; the IO implementation acceptance ledger stays unchanged.

**First independent IO-25 JMF smoke attempt (not accepted).** [tools/jmf_smoke.py](tools/jmf_smoke.py) attempts to bootstrap the pinned `jlibrary/bin/profile.ijs` and `load 'jmf'` under an isolated temporary HOME and J C binary, then runs RW(0)→RO(1)→COW(2) **map → empty noun check → unmap-result-zero** on a temporary JMF backing file. [Offline plan checks](tools/test_jmf_smoke.py) and a separate [Linux CI](.github/workflows/linux.yml) non-acceptance diagnostic step are added. Missing library bootstrap or JMF execution is reported as `blocked`, not a J semantic mismatch; CI results were not finalized at this checkpoint. This smoke does **not** validate boxed payloads, write durability/RO-COW mutation behavior, refcount-denied unmap, resize/remap, or Jd partitions. IO-25 remains [ ].

**Initial JMF bootstrap observation (2026-10-07; not accepted).** In [Linux 37538470000](https://github.com/yunskim/RustJ/actions/runs/37538470000), the pinned `j64/default` and `j64/portable` non-gating JMF probes recorded `status=blocked`, `stage_count=2`: setting `BINPATH_z_` succeeded but `0!:0 <.../jlibrary/bin/profile.ijs` returned `domain error`. **The probe never reached map/unmap.** This is a bootstrap blocker, not evidence that JMF RW/RO/COW semantics fail. [Diagnostic follow-up 541be39](https://github.com/yunskim/RustJ/commit/541be39f42f1df655e7af5bde86d928321f4c9b3) first checks loading a trivial standalone J script and captures `13!:12` on errors, to isolate script loading from full profile setup; the subsequent CI result was not yet confirmed at this checkpoint. IO-25 remains [ ].

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

## 15.2 Cross-stage negative verifier matrix

Positive E2E tests are insufficient. Each stage must reject invalid states owned by that stage:

| Stage | Required rejection examples | Status |
|---|---|---|
| J Graph `Plan::verify` | schema/primitive-registry mismatch, invalid IDs/regions, stale region results/stages, malformed pipeline/fork/hook topology, provenance drift | implemented; current schema exact-matches J Graph 0.9 |
| rewrite candidate verifier | stale source span/basis, unregistered rule/witness mismatch, invalid replacement DAG/facts/output semantics | implemented |
| scan/fusion analysis verifier | forged order/rule version/witness/retention/fanout or unsupported selected state | partially implemented; proof-discharge/selection verification remains future |
| A3 `Plan::verify` | schema/registry mismatch, invalid references/use-before-def, source/j_origin drift, malformed constraints/checks/effect/error/speculation/result/terminator | implemented; current schema exact-matches A3 0.5 |
| CandidateEvidence / SelectionPlan | stale evidence, Selected with required proof Unknown, selected Illegal candidate, incompatible overlapping candidates | planned — §7.5 |
| RouteRegion / RouteBoundary | missing live-ins/outs, dropped effect-live dependency, duplicated/dropped/reordered SemanticCheck, unproven region capability, post-effect guard, missing bridge | planned — §2.1 |
| PhysicalPlan | invalid buffer/view/op IDs, use-before-bind, out-of-bounds view, incompatible kernel, unordered Check, unproved overlap/reuse, dangling Return | planned M4 — §17.2.1 |
| ExternalRegionPlan | missing source op, dropped check/effect edge, capability/emission mismatch, incomplete completion/ownership, partial unsupported module reported as success | planned external-route gate — §12.2 |

Use one-mutated-invariant negative tests: build a valid artifact, clone it, break exactly one invariant, require that stage's verifier to reject it, and never invoke a later planner/executor. Do not make downstream layers repair invalid upstream artifacts.

## 15.3 Serialization / schema migration policy

Current J Graph/A3 artifacts are primarily in-process and do not promise long-term portable binary compatibility. Today the verifiers require exact schema and primitive-registry provenance:

```text
J Graph schema 0.9   exact match
A3 schema 0.5        exact match
PrimitiveRegistry    current REGISTRY_VERSION provenance
```

Do not assume minor-version compatibility implicitly. When external storage/interchange is introduced, decoding and migration are separate: decode using the artifact's known schema; apply an explicit version-to-version migration chain only when implemented; then run the current verifier. Unknown fields/ops/rules are never guessed into current meaning.

Downgrade is allowed only through an explicit lossless writer. If an older schema cannot represent a newer semantic field/op/effect, reject the downgrade rather than silently dropping it. PrimitiveRegistry version migration is separate from IR schema migration; identical spelling does not justify ignoring contract/ID changes.

Compiler version is provenance; schema/registry plus explicit migration contracts define compatibility. Rewrite/fusion/scan rule/witness registries also need versioned provenance so cached candidates become stale when meanings change. A future portable PhysicalPlan cache has its own schema plus target/device/runtime/capability fingerprint, not the Logical IR schema.

Unsupported schema/registry/migration is a compiler/artifact diagnostic, never a J Domain/Rank/Length error. Before 1.0, schemas may change frequently, but semantic field meaning must not change without a version bump.

---

# Part XIII — Current implementation status

<a id="current-implementation-status"></a>

## 16. Completed or substantially implemented

Code/document review baseline: the 2026-10-04 WI1 noun input metadata validation seam. Read the latest execution results and remaining boundaries together with the JE2 checklist. Earlier stage gates remain historical validation records.

- shared immutable FunctionEntity semantic DAG;
- explicit/direct-definition frontend support through immutable `DefinitionCode`, control-flow metadata, multiple root direct definitions, raw noun direct definitions, and UTF-8/source provenance; supported mode-1/2 straight-line invocation uses per-call local frames with x/y/u/v/m/n bindings and local/global assignment, while control-flow/nested scope, full locale/locative/operator-wrapper semantics, other tagged/computed forms, and Code-body J Graph/A3 CFG lowering remain incomplete;
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
- `physical.rs` provides representation foundations and a narrow CPU search-strategy selector, not the complete Physical Planner;
- callable/runtime `reduce/rank` summaries remain migration fields;
- frontend uses the same ordered 9-row matcher/runtime-analysis reduction engine; the old flat modifier/train heuristic reducer is removed. Supported name/POS/assignment and completed-result boundaries are implemented, but full enqueue/construction/local/locale/definition semantics still gate M2 completion;
- latest frontend/numeric validation is **NV3d2b2a**: Windows default/portable each **474 passed / 17 ignored**; fmt/clippy/build pass; Python **30 passed**. Existing j64/AVX2 runtime routes remain **5,380 / 5,380 passed / zero failures**, stages **10,810**, words **6,623**. Each DLL has **2,485 numeric-syntax cases / zero failures**, including 182 accepted noun controls, 1,244 lexical-error equalities, 850 valid payload boundaries, two integer-conversion boundaries, 200 C-reference precision boundaries, one quad-construction boundary, and four NaN word-formation boundaries. One unresolved recognition and one unresolved error boundary remain and are not counted as execution or precise-error-equivalence passes. Keep vocabulary POS 143 / bare bindings-AR 140 / noun payloads 3, capture graph 257, static 2, and runtime prefix 285 / executable prefix passes 0 separate. The latest graph-readiness gate is **GF6a (463 passed / 17 ignored)** and does not mean semantic-proof discharge, fusion selection, performance, or GPU execution is complete. Full upstream/definition acceptance/private C trace/Linux/GitHub CI/CUDA remain unverified or deferred.
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

**I/O tracking:** All storage, slow-I/O and out-of-core acceptance work belongs to the [IO-01–IO-30 checklist](#out-of-core-io-checklist). M2→M3→M4 semantic/CPU baseline remains the project priority; IO-A primary-source audits may proceed concurrently. Do not create another checklist.

<a id="heterogeneous-execution-checklist"></a>

### M4-P0 — Verified host identity PhysicalPlan (2026-10-07; partial code, tests not run)

- [x] **Code added:** `src/physical_plan.rs` introduces `PlanBufferId` distinct from runtime `BufferId`, `PhysicalViewId`, independent `ExecutionDevice` / `MemorySpace`, plan buffer/view descriptors, `PhysicalOp` variants, and a fail-closed v0 verifier. `empty_from_a3` supports genuinely empty blocks. `identity_literal` / `execute_identity` use `BufferRegistry` and a checked CPU `PhysicalArray` to realize only a single closed dense literal with `BindInput → Return`, no kernel or transfer.
- [ ] **Validation pending:** `tests/physical_plan.rs` contains independent logical-executor comparison and one-invariant-at-a-time invalid-plan cases (stale provenance, view, buffer extent/encoding, ownership, order, result, Check/Kernel/stateful A3). Rust fmt/clippy/default/portable tests, C oracle comparison and CI have **not** been run for this change. This is *not* general PhysicalPlan/CPU kernel execution, nor HE-01 acceptance.
- [ ] **Next slice:** view span and metadata-only reshape/reindex where semantics are proven; ordered A3 Check; selected Add Kernel/capability; materialization and lifetime/reuse witnesses, followed by end-to-end source→PhysicalPlan CPU differential testing.

---

### HE — Heterogeneous CPU/GPU execution planning (2026-10-07; links M4→M6)

**Decision.** RustJ is a heterogeneous array compiler, not a CPU thread-parallel compiler. CPU workers are a *device-local physical realization*, not a top-level canonical `Parallel IR` or `Parallel Physical Planner`. Retain `J Graph IR → verified logical_ir::Plan → RoutePartition → Schedule/Transform → Physical Planner → Physical Execution Plan → Executor`. Keep this checklist inside §17; do not fork the canonical roadmap or create an additional required IR. **This design decision does not lift the existing CUDA hold.**

**Primary implementation comparisons (inspiration, not adoption of whole dependencies):**

| Reference | Evidence | RustJ adaptation and limit |
|---|---|---|
| [IREE Stream](https://iree.dev/reference/mlir-dialects/Stream/) and [passes](https://iree.dev/reference/mlir-passes/Stream/) | flow → stream → HAL differentiates executable regions, target affinity/resource readiness, async scheduling and backend execution | Plan for device-affinity, readiness/completion and lifetime downstream, not a mandatory new Stream IR |
| [Kokkos View](https://kokkos.org/kokkos-core-wiki/ProgrammingGuide/View.html) and [Memory Spaces](https://kokkos.org/kokkos-core-wiki/API/core/memory_spaces.html) | ExecutionSpace is independent of MemorySpace; coherence/accessibility need fences | Do not equate executor location and memory residency, and never assume managed memory makes movement free |
| [MLIR scf.forall](https://mlir.llvm.org/docs/Dialects/SCFDialect/) / [tensor.parallel_insert_slice](https://mlir.llvm.org/docs/Dialects/TensorOps/) | iteration topology, target mapping and disjoint result slices are separate; side effects may be unordered | Preserve A3 iteration shape; require explicit J legality proof for concurrent effects/errors and output assembly |
| [XLA parallel task assigner](https://github.com/openxla/xla/blob/main/xla/service/cpu/parallel_task_assignment.cc) | internal CPU task count driven by FLOPs, bytes and overhead | Device-local CPU cost input only, not a global mixed-device scheduling model |
| [Futhark multicore scheduler](https://github.com/diku-dk/futhark/blob/master/rts/c/scheduler.h) / [Rayon](https://docs.rs/rayon/latest/rayon/) | chunk/task amortization and worker scheduling | CPU backend implementation detail; no thread counts in Graph/Logical IR |

**Stage and identity contracts.** (1) `logical_ir.rs` retains J value/shape/rank/CellApply, empty-frame virtual fill, late name/version, comparison/fit, effect and ordered observable errors. `IterationAxisKind::Parallel` is an *opportunity*, never a legality proof. (2) The existing `lowering.rs` legality/witness/guard boundary governs splitting, reordering and concurrency; Unknown is not proof. (3) `RouteRegion` placement (CPU/GPU/external) and the intra-device execution mode (sequential/SIMD/workers/GPU grid) are separate axes. A deterministic all-CPU, sequential, zero-transfer plan is valid. (4) Semantic `ValueId`, plan buffer/version identities, runtime leases/BufferId, memory-space residency and readiness are distinct. (5) Future Physical Execution Plans must represent transfer, computation, synchronization, completion, effect/error order, and ownership/lifetime edges. No concurrent overlapping outputs, check-after-effect, arbitrary first-lane error, or opportunistic resource release. (6) ResourceEstimate, CostEstimate, and empirical CostProfile remain separate; consider critical-path work/depth, memory capacity/bandwidth/residency, bytes and latency of transfers, compute/launch/synchronization overhead, peak in-flight bytes, and overlap only where dependencies allow it. Unknown resource is not feasible, unknown cost is not zero. (7) Native all-CPU M4 first; external MLIR/StableHLO remains independently eligible; CUDA/multi-device and async execution require later verified hardware and an explicit resumption request.

**Single integrated checklist:**

| Gate / phase | State | Acceptance |
|---|---|---|
| HE-00 / concurrent with M2 | [x] Architecture comparison and boundary decision recorded | Design only: this section, `FOUNDATIONS`, `AGENTS`; does **not** establish executable parallel/GPU support |
| HE-01 / M4 | [ ] Single-device, all-CPU, zero-transfer verified PhysicalPlan and end-to-end execution (**P0 identity code exists; acceptance pending**) | Original `BindInput/Check/View/Materialize/Kernel/Return` sequential slice vs `logical_executor` and actual J reference |
| HE-02 / M4→M5 | [ ] Distinct execution device / memory space / intra-device scheduler contracts and verifier | Graph/A3 unchanged, unknown feasibility rejected, live resources and disjoint output checks |
| HE-03 / M5 | [ ] Semantic ExecutionLegality report and witnesses/guards through existing lowering | Dynamic names, error/effect order, alias, Rank fill, guarded fallback and negative tests |
| HE-04 / M5 | [ ] CPU sequential/SIMD/worker candidates and observed cost model | Work size, bandwidth, launch/pool overhead, nested oversubscription and output parity |
| HE-05 / M5 | [ ] Device-memory placement and transfer readiness/cost | Buffer versions, residency, capacity, copy/prefetch/migrate, sync, `IO-20` shared contract |
| HE-06 / after M5 | [ ] Benchmark/compare a single-CPU baseline against modeled mixed candidates | Distinguish cold/warm, work, copies, transfer and actual runtime; do not claim a GPU implementation |
| HE-07 / after M6 | [ ] Actual CPU+GPU completion/transfer runtime and end-to-end checks | **Deferred** pending explicit restart and verified device; versions, failures, async, unsupported route |
| HE-08 / after M6 | [ ] J semantic tests for CellApply/reduce/scan parallelizations | Zero frame vs empty cell, type/shape joining, boxed/sparse, tolerance, reassociation and ordered errors |
| HE-09 / after M5 | [ ] Explain and validate schedule decisions/measurements | Choice/witness/guard reasons, bytes moved, synchronization, peak residency and J equivalence |

**Sequence:** HE-00 design → existing M2/M3 convergence → M4/HE-01 CPU baseline → HE-02/03 legality and identities → HE-04/05/06 backend-local worker/placement/cost candidates → HE-07/08/09 only when ready. Do not create a duplicate movement model: join the existing IO-20 planner scope. Follow §11 testing and never mark HE-01–09 complete based only on documentation.

---

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

**Return-boundary gate:** native Windows default/portable each **426 passed / 17 ignored**; fmt/clippy/build pass; Python **27 passed**. Each C base/AVX2 runtime route: **5,331 cases / 5,331 passed / 0 runtime boundaries / 0 failed**; stages **10,767 checks**; words **6,618 cases**, all with zero failures. Keep **250 capture-graph** and **2 static** boundaries separate. Reverify actual source/reference/binary hashes in all ten reports. The ten new common statements participate in all three runtime routes and stage checks. Both the Verb-rank and Return-boundary gates are historical. **NV2 was the latest gate at that point; the current latest validation is NV3d2b2a, while graph-readiness history has progressed through GF6a.** Unverified scope and optimizer/CUDA/Linux/GitHub CI deferral are unchanged.

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


<a id="out-of-core-io-checklist"></a>

### IO — Slow I/O / out-of-core migration acceptance checklist (2026-10-06)

**Framework execution comparison:** [§13.4](#io-framework-execution-comparison) explains the stage-by-stage optimization mechanisms; all acceptance evidence stays in the 30-row ledger below.

**Status: documented; 0/30 implementation acceptance gates passed.**

**Checklist operating protocol.** IO-01–IO-30 is the single acceptance ledger for slow-I/O/out-of-core work; retain stable IDs and do not duplicate in new roadmaps. For each iteration: (1) choose the smallest ready unchecked unit by prerequisites, (2) pin original Jsource/add-on and comparison-framework evidence plus rights/capability boundaries, (3) establish baseline semantic and negative fixtures, (4) implement only the legally allowed physical change, (5) compare pinned J C oracle / independent Rust synchronous / optimized routes and measure memory and I/O, and (6) record commit, actual command/environment, results, gaps and blockers in the corresponding row. A source review or design is not implementation acceptance. Preserve [ ] on unexecuted/failed/unsupported gates and cross-link existing FW/DB/G4/G5 gates.

**Next execution (not accepted):** IO-01/25 source pins and preliminary IO-02 effect/error counterexamples are recorded in [§13.6](#io-a-source-audit). The 15 independent file-foreign plus six ordered-effect C-oracle cases now provide a preliminary baseline. Extend named/numeric handles, permissions, close/flush and JMF bootstrap/RW/RO/COW before considering acceptance. Source review alone is not acceptance: **0/30**.
 Sequence: IO-A primary-source/semantic contract (may proceed during M2) → IO-B synchronous reference (after initial M4 CPU slice) → IO-C proven read minimization → IO-D bounded async → IO-E weight reuse/placement → IO-F measurement/expansion. Do not make this a prerequisite of M2, the generic FW checklist, or the first native CPU vertical slice. [ ] = not accepted even if partial code exists; [x] requires actual change SHA, commands/environment, tests including negative cases, J oracle coverage where relevant, unsupported limits and CI status.

| ID / stage | Checklist | Acceptance evidence / prerequisite |
|---|---|---|
| IO-01 / A, M2 parallel | [ ] Pin Jsource and J add-on source behavior (partial source review, executable oracle pending; §13.6) | jmf/xf.c/alias/JMF-boxed branches plus pinned Jd column/partition/jmfx and Jfiles/keyfiles source, confirmed by J binary foreign fixtures |
| IO-02 / A, M2–M3 | [ ] Define mapped/file J effect and error contract (C oracle 15 independent + 6 ordered cases partially executed; handles/permissions/JMF/Jd outstanding; §13.6) | read/write/resize/flush/close, order, alias, COW and empty Rank; rejection tests for illegal reordering |
| IO-03 / A, M3 | [ ] Verify existing IR and shared movement/identity/effect boundary | J Graph topology vs Verified Logical effect/dependency/AccessRelation vs Physical Plan buffer/file region, transfer and readiness; ValueId ≠ BufferId ≠ StateResource ≠ external object/version. No new mandatory Data Movement IR. Verifier rejects misplaced physical fields and unknown effects (§13.5) |
| IO-04 / A, M3 | [ ] Establish per-storage capability matrix | offset/alignment/EOF, snapshot, consistency, write durability, unknown as route barrier |
| IO-05 / B, after M4 | [ ] Independent synchronous read_at/write_at baseline | offsets, short read/EOF, overflow, permission/error tests; J file foreign semantics not conflated |
| IO-06 / B, after M4 | [ ] Versioned dense chunk reader | dtype/shape/order/endian/checked offsets, final partial chunk, bounded large scan |
| IO-07 / B, after M4 | [ ] Minimal mapped dense route | RO/RW/COW, header/shape, unmap/refcounts, read_at equivalence, JMF-backed boxed/non-jmf typed boxed capability distinction, plus sparse limitations |
| IO-08 / B, after M4 | [ ] Resident/retained-memory bound | buffer leases and release, lower budget than dataset, early-release/leak/cancel tests |
| IO-09 / C, M4–M5 | [ ] Prove logical access regions before physical byte pruning | select/slice/reindex AccessRegion witnesses before BufferSlice/FileByteRange optimization; unknown Rank, alias, mutation, observable errors/effects and empty prototype require opaque/barrier fallback. Independent byte-elision/semantic tests (§13.5) |
| IO-10 / C, M5 | [ ] Proven projection/slice pushdown | reduce bytes without value/error changes; reduction/boxed/sparse counterexamples |
| IO-11 / C, M5 | [ ] Reusable/versioned scan cache | same object/version/policy/range only; invalidation on mutation/rebinding |
| IO-12 / C, M5 | [ ] Immutable snapshot and stale-detection policy | no speculative external reads with observable side effects |
| IO-13 / D, M5 | [ ] Portable async facade and readiness contract | Blocking-I/O workers above independent sync baseline, Pending/Ready/Failed/Cancelled/completion tokens; common scheduling dependencies for memory/file transfers while preserving distinct error and cancellation semantics, plus sync fallback (§13.5) |
| IO-14 / D, M5 | [ ] Bounded prefetch/double buffer | overlap and token/lease correctness, async vs sync differential |
| IO-15 / D, M5 | [ ] Memory governor/backpressure | account queued, in-flight and runtime temp bytes; slow-consumer/OOM tests |
| IO-16 / D, M5 | [ ] Streaming legality/barriers | reduction, rank, zero-frame, nonstreamable materialization, ordering/cancel |
| IO-17 / D, M5 | [ ] Completion, lease, error/effect-order proof | short/failed reads, dirty mappings, cancel/retry, early release; unused `1!:` file read cannot be dropped. Speculative immutable backing reads also require visible-error-order proof; no double effects or buffer reuse during pending transfer (§13.5) |
| IO-18 / E, M5 | [ ] Mutable weights/checkpoint consistency | version capture, atomic publish/durability/recovery/partial writes |
| IO-19 / E, M5 | [ ] Legal weight/scan reuse scheduling | reuse helps bytes/cache, no effect/order changes, latency/throughput separated |
| IO-20 / E, M5 | [ ] Unified physical movement/placement cost planner | Existing Physical Plan jointly considers memory copy, prospective CPU↔GPU, file byte-range read/write and transfer, with separate region/location/lifetime/ready dependencies. Distinguish bytes/requests/seeks/latency/peak/inflight/compute/overlap, hard unknown-resource gates and unmeasured cost heuristics. GPU remains deferred; revisit new IR only after two concrete expressiveness failures (§13.5) |
| IO-21 / F, M5 | [ ] Cold/warm benchmark matrix | sequential/random, large/small, NN weights, views, boxed/sparse/empty |
| IO-22 / F, M5 | [ ] Independent three-way semantic and movement tests | Pinned C oracle where J foreign semantics apply / independent Rust sync / optimized. Test skipped-read errors, stale versions, empty-Rank virtual cells, boxed/sparse, aliases, early free/cancel, intermediate copies/transferred bytes. Report pass/fail/ignored/unsupported (§13.5) |
| IO-23 / F, M5 | [ ] Benchmark-backed mmap/read_at/async selection | page faults, bytes/requests, peak RAM, regressions, storage differences |
| IO-24 / F, after M6 | [ ] Optional extension approval gate | io_uring/direct I/O, object store, compression, DMA/GPU, multiple devices and Jd adapters after capabilities and portability proven |


**Additional acceptance gates IO-25–IO-30** (added in discovery order, executed by prerequisite stage; retain IO-01–IO-24):

| ID/stage | Checklist | Prerequisite / acceptance evidence |
|---|---|---|
| IO-25 / A, M2 parallel | [ ] Cross-audit Jd/jfiles/JMF boxed paths (source pinned, standalone RW/RO/COW smoke added; mapping/boxed/Jd executable acceptance pending; §13.6) | Pinned jsource and data_jd, executable J oracle for Jd partitions, keyed components, typed vs JMF boxed cases; IO-01/02 |
| IO-26 / B, after M4 | [ ] Verify typed array storage manifest | dtype, shape/order/endian, offsets/length/version, duplicate/overlap/off-end/overflow, empty/scalar and boxed/sparse capability; IO-05/06 |
| IO-27 / B, after M4 | [ ] Separate read chunk/write shard/layout | Access-axis-specific amplification, coalescing, file count, shard-write cost and contiguous fallback vs Zarr/HDF5; IO-06/08 |
| IO-28 / B, after M4 | [ ] Prove mmap/SIMD tail/lease safety | EOF page guard, no unproved vector overfetch, OS page granularity, live references on remap/unmap, RO/COW and concurrency; IO-07/08 |
| IO-29 / C, M5 | [ ] Bounded decoded chunk cache | Identity/version/range key, lease-safe eviction, invalidation on mutation and strided/cache-thrash tests; IO-09–12 |
| IO-30 / F, after M5 | [ ] Benchmark workload-dependent loading | mmap vs buffered read/async, cold/warm local/remote, NN weights, major faults/RSS/latency/throughput and budget stress; IO-21/23 |

**Acceptance log:** `IO-ID | code commit | pinned source | command/OS/target/storage | C oracle / Rust sync / optimized counts | cold/warm bytes/wall time/peak | failures/unsupported | CI evidence | next gate`. No [x] based solely on design prose or file existence.

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

A3 already provides basis payloads, verification and a closed-plan reference executor; the route prototype exists. General innate-rank/implicit-cell assembly and native Schedule/Physical Executor remain incomplete. The existing `canonical_mean_fork_lowers_to_reduce_tally_divide_in_jsource_order` test calls `analyze_a3 + verify` and checks ordering; it is not an execution/E2E test.

Use this same source as the canonical stage-by-stage compiler trace:

```text
J source
  (+/ % #) y
      |
      v
[implemented] parser / Semantic Construction
  Fork
    f = +/        // Insert(+) derived Verb
    g = %
    h = #
  preserve source span / operand identity / observable fork order
      |
      v
[implemented] J Graph IR
  input y
      +-- h branch: Apply Tally(y) ----------------+
      `-- f branch: Apply Reduce(Add, y) ----------+
                                                   v
                                         Apply Divide(f, h)
  + Fork region/provenance
  + branch/join/use/liveness facts
  + Graph Basis / GraphHint
      |
      +--> [implemented analysis] rewrite/fusion/resource/work-depth candidates/side analyses
      |       candidates do not replace the source graph or commit execution
      |
      v
[implemented] Execution Semantic Lowering -> canonical A3
  observable order: h -> f -> g
  Tally(y)
  Reduce(Add, y)
  Divide(left=reduce, right=tally)
    + valence/rank/cell/frame facts
    + prefix-agreement/repetition constraint or witness
    + SemanticCheck only when a required constraint remains unresolved
    + effect/error/speculation/order metadata
      |
      v
[implemented] A3 verification/reference capability
  Plan::verify()
  logical_executor::execute_closed() on its supported closed subset
      |
      v
[prototype] route analysis
  LoweringRegistry::route_operation / partition_plan
  an op without a native realization may classify as RuntimeSemanticFallback
  contiguous class grouping is not the final mixed-route plan
      |
      -------- current stop line for compiler-native physical execution --------
      |
      v
[planned M4] deterministic CPU Schedule / PhysicalPlan
  BindInput(y)
  Check(...)             // only for unresolved A3 SemanticChecks
  Kernel Tally
  Kernel Reduce(Add)
  View/iteration mapping // repeat the right scalar according to J cell semantics
  Kernel Divide
  Return(result)
      |
      v
[planned M4 validation]
  result = 1.5 2.5 3.5
  + value/error/order differential against jsource
  + allocation/view/reuse invariants
```

The generic `Tally + Reduce + CellApply/Divide` path is the correctness baseline. A Mean-style fused/composite form is only a later optimization candidate and must separately satisfy equivalence/rank-cell assembly, numeric/error/effect ordering, fanout/retention, resource/work-depth, target capability, and profitability. Do not read today's `j_graph_fusion`/`fusion_planning` as already selecting or lowering the whole Mean specialization.

Provenance must remain traceable: parser FunctionEntity Fork/operands/span -> J Graph region/value -> A3 `j_origin`; candidates retain source graph/rule/witness provenance; route and PhysicalPlan reference rather than destructively overwrite canonical A3; failures should map back through A3/J Graph provenance to source spans where possible.

Add matrix-cell golden/reference comparison before considering fused Mean, then connect the M4 physical route. [Detailed Korean proof case](PROJECT.ko.md#mean-proof-example).

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
| word formation → enqueue → parser | **documentation contract closed / convergence ongoing** | A0.5/F0–F2/P0–P8, jsource oracle, 9-row reductions, sequencing/gates, §4.3.1 `+/ y` canonical trace | actual support for locatives/definitions/gerunds/value-dependent constructors remains checklist-driven; orientation ownership is closed |
| Semantic Construction / binding / dynamic semantics | **documentation contract closed / general CFG implementation incomplete** | FunctionEntity/JEntity, late NameRef, assignment=value+effect, definition metadata, **runtime straight-line per-call LocalFrame/invocation subset**, gerund/rank/train preservation, explicit-definition handoff | Runtime frames are not wholly planned. What remains is general control-flow/nested/locale-locative support plus compiled Branch/CondBranch/value-merge CFG lowering; current A3 Terminator remains Return-only |
| J Semantic → J Graph IR | **documentation contract closed** | GraphForm/GraphBasis/GraphHint, provenance/applied graph, Graph-vs-Execution distinction, canonical suite for `@:`/ordinary+capped fork/hook/rank/reduce/prefix-infix | remaining gaps are implementation/test coverage per form, not missing stage ownership |
| graph analysis → candidate/proof | **documentation contract strengthened / implementation partial** | §7.5 defines orthogonal evidence, derived lifecycle, evidence owners, guarded legality, overlap/selection rules | Common `CandidateEvidence/ProofBundle`, per-obligation discharge, and SelectionPlan are **not implemented**; individual proof algorithms land with their verifier/tests |
| J Graph → execution-semantic lowering → A3 | **documentation contract mostly closed / CFG implementation incomplete** | direct lowering, fact-drift checks, Execution Basis, SemanticCheck, effects/errors/speculation, verifier, schema header, canonical mean trace, explicit-definition current/planned handoff example | A3-v0 is still single-block/Return-only. What remains is executable Branch/CondBranch/block-merge lowering plus differential E2E, not a missing documentation example |
| route analysis / partition | **documentation contract strengthened / implementation partial** | §2.1 defines live-ins/outs, effect live-outs, SemanticChecks, guards, representation-neutral bridges, and region legality | Current `RouteRegion { class, operations }` plus contiguous grouping remains a v0 helper; real bridge/region-wide verification and mixed-route execution are unimplemented |
| schedule / Physical Planner | **M4-v0 full planner unimplemented / i. family strategy selector partially implemented** | §17.2.1 defines `PlanBufferId != runtime BufferId`, PhysicalView, BindInput/Check/View/Materialize/Kernel/Return, lifetime/reuse/verifier/error-cleanup | Actual PhysicalPlan types, full planner, and executor remain unimplemented; the runtime-guarded i. strategy selector in physical.rs is a prototype. Transfer/Sync/async are post-M4 |
| native executor | **M4-v0 contract mostly closed / unimplemented** | §§17.2/17.2.1 cover op roles, verifier, cleanup/errors, executor non-responsibilities, and the canonical mean planned route | No real Physical Executor or differential E2E test yet; stateful/async execution remains later work |
| fallback / guard miss / replay | **documentation contract strengthened / dispatcher unimplemented** | §5.2.2 defines route fallback vs guard miss vs replay/continuation, the decision table, commit frontier, and precise RuntimeSemanticFallback meaning | Integrated guard dispatcher, exact continuation, and transactional rollback remain unimplemented and must not be claimed as capabilities |
| external route / GPU | **boundary contract fixed / implementation deferred** | §12.2 defines adapter input/capabilities/output, check/error/effect/token mapping, bridge/ownership, round-trip verification and failure classes | production adapters remain unimplemented; CUDA remains intentionally deferred |
| validation / versioning | **documentation contract strengthened / implementation follows stages** | frontend gates, exact J Graph 0.9/A3 0.5 schema+registry verification, §15.2 negative matrix, §15.3 migration/downgrade policy | Candidate/Route/Physical/External negative verifiers land with their stage implementations; portable serialization is not yet offered |

**First-pass documentation closures completed on 2026-10-06** without changing the current M2 implementation priority:

- [x] candidate lifecycle and proof-discharge ownership — §7.5
- [x] RouteRegion boundary contract — §2.1
- [x] minimal M4 PhysicalPlan v0 schema/verifier/cleanup contract — §17.2.1
- [x] fallback/guard-miss/no-replay decision table — §5.2.2
- [x] canonical end-to-end compiler trace — §17.1.1 and the detailed Korean proof case

These are documentation-contract completions, not implementation-completion claims. The canonical trace includes an explicit current stop line before planned M4 Schedule/PhysicalPlan execution.

**Second-pass documentation closures completed:**

- [x] compact frontend canonical sentence trace — §4.3.1
- [x] explicit-definition control-flow handoff example — frontend section
- [x] canonical J Graph example suite — JAXA/J Graph section
- [x] external-adapter boundary contract — §12.2
- [x] cross-stage negative-verifier matrix and serialization migration policy — §§15.2–15.3

These remain documentation contracts, not implementation-completion claims. Branch/CondBranch, CandidateEvidence/SelectionPlan, concrete RouteBoundary, PhysicalPlan, and ExternalRegionPlan are target concepts where the corresponding Rust APIs do not yet exist.

Remaining gaps are now mostly **implementation-driven details**: actual proof algorithms/cached-evidence invalidation; concrete explicit-definition CFG types and loop/try/select lowering; concrete RouteBoundary/BridgeRequirement types and executor integration; PhysicalPlan op structs/planner/reuse implementation; per-adapter capability/emission code; intentionally later async/stateful/exact-continuation semantics; and a wire format only when portable artifacts are actually exported. Do not freeze speculative APIs early; derive each from the stage contract using one meaning + one verifier/negative test at a time.

Maintenance rule: whenever a major compiler stage/type or framework-comparison claim changes, update the relevant contract summary and cross-search `FOUNDATIONS`, `PROJECT`, `README`, and `AGENTS` for stale duplicate claims. Keep these audit results inside the canonical project documents rather than spawning separate Markdown review reports. When a new validation gate becomes the current baseline, update the top-level Latest validation and current-status summary in the same change; historical gates that say “latest” must be marked as time-local history.

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
