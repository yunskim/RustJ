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

The reference depends on the question being asked:

```text
J semantic correctness       → jsource
array graph/JIT fusion       → ArrayFire
J↔external GPU adapter       → jsoftware/math_arrayfire
array-compiler middle-end    → APEX / Co-dfns / TAIL-Futhark
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

---

## 3. JAXA design principle — “SQL for neural networks”

Earlier JAXA documents used the phrases **“SQL for neural networks”** and **“SQL for array operations.”**

The point was not to imitate SQL syntax or to declare a universal neural-network platform. The recurring idea was:

> **JAXA specifies logical array intent, not physical execution procedure.**

The SQL analogy concerns the separation between **logical intent** and **physical execution plans**.

```text
logical array intent
        ↓
legal logical / physical plans
        ↓
resource / cost evaluation
        ↓
selected execution plan
```

Users should express the computation and the semantic/storage obligations that matter, while the analyzer/compiler/backend decides matters such as:

- which equivalent graph form to use;
- whether to fuse or materialize;
- which execution basis and route to use;
- which memory/schedule strategy to use;
- which target-specific realization to select.

Another useful line from the earlier JAXA documents was:

> **JAXA does not execute fusion — the compiler does.**

RustJ inherits that separation in the following form:

```text
J semantics / FunctionEntity
        ↓
J Graph IR / Graph Basis
        ↓
rewrite / equivalence
        ↓
symbolic resource reasoning
        ↓
Logical Execution IR / Execution Basis
        ↓
physical planning / backend realization
```

This analogy is not intended to broaden RustJ's current product scope. RustJ's immediate goal remains **a J compiler/runtime that preserves full J semantics**.

“SQL for neural networks” is used here as historical design context for a few concrete principles:

1. Do not unnecessarily encode physical procedure in the source language.
2. When several semantically equivalent realizations exist, leave room for the compiler to choose.
3. Establish legality before cost.
4. Use resource/cost models to choose among legal plans, not to redefine semantics.
5. Keep logical intent reusable as optimizers and backends improve.

In RustJ, therefore, “SQL for neural networks” is best understood as an inherited compiler separation-of-concerns principle, not as a claim of a broad platform already delivered.

## 3.1 Core array-model decision — separate Logical Array from Physical Array

One of RustJ's most important architectural decisions is to **separate logical array semantics from physical array representation**.

The array observed by a J program is not the same thing as the way a particular CPU/GPU/backend stores, lays out, or accesses that array.

```text
Logical Array / J noun
    dtype / J-visible type
    shape
    ordered logical atoms / value
    J-visible representation semantics
      Dense / Boxed / Sparse / ...

            ≠

Physical Array / Representation
    BufferId / storage
    strides
    offset
    concrete layout / tiling
    alignment
    memory space
    CPU / GPU placement
    sharding
    transfer / synchronization
```

This is a semantic boundary, not just an implementation convenience.

Core invariants:

1. **Logical Array owns J meaning.** Shape, atom order, and J-visible boxed/sparse semantics remain in the logical/semantic layers.
2. **Physical Array owns realization.** Strides, offsets, concrete layout, buffers, memory spaces, and device placement belong downstream.
3. **One logical value may have multiple physical representations.** The same ValueId may exist on CPU/GPU or in different layouts.
4. **Several logical values may reuse one physical buffer** when their lifetimes do not overlap.
5. **A Logical ArrayValue does not imply a distinct materialized buffer.** Views, fused-away intermediates, and rematerialized values may exist without one.
6. The logical meaning of reshape/transpose/reverse/slice is separate from whether a copy/materialization is chosen.
7. J-visible representation classes such as sparse/boxed remain semantic, while concrete encodings such as CSR/COO or pointer/handle layouts are physical.

Conceptually:

```text
J noun / Logical ArrayValue
        ↓
semantic + graph + logical execution analysis
        ↓
RepresentationFacts / realization choice
        ↓
PhysicalArray
        ↓
BufferId / memory space / layout / device
```

This is why GraphFacts and Logical IR do not own stride/offset/device state, and why Physical Planning decides concrete layout/buffer/placement. Backend convenience must not redefine J noun semantics.

The detailed model is described in the later logical/physical array section and in the RepresentationFacts boundary.

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

The existing special-purpose modifier/train reducers are transitional and should disappear once the unified parser engine covers them.

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

### 5.2 Names and binding

J Name, BindingVersion, semantic FunctionEntity, and Logical SSA ValueId are distinct concepts.

Late-bound function names must retain J semantics unless specialization is justified by a binding/version witness or runtime guard.

Names must not be globally snapshotted at sentence start if doing so changes J's observable right-to-left lookup or assignment behavior.

---

# Part III — Rank, cells, and array semantics

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

One ValueId may have several physical representations. Conversely, several non-overlapping ValueIds may reuse one BufferId.

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
- [ ] The historical name `facts::LayoutFact` currently means the J-visible `Dense / AxisSparse` representation class, not physical layout. Consider moving it toward a `RepresentationFact` name in a later cleanup.

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

Current schema: **v0.3**.

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

Whether Scan deserves a separate Graph Basis identity remains an open basis-taxonomy question.

---

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

## 15. Semantic validation

For semantic changes:

- add reproducer/regression coverage;
- compare against pinned jsource where available;
- test word formation, enqueue, parser, and execution at the appropriate layer;
- preserve J error class even when diagnostics become richer;
- do not claim validation that was not actually run.

The user has requested that current development not depend on remote CI; source-level review and direct/local checks should be reported accurately.

## 15.1 Differential frontend validation

Frontend compatibility should eventually provide separate differential gates for:

1. word formation;
2. enqueue classification/metadata;
3. parser reduction behavior and result topology.

A mismatch in a test harness must first be distinguished from a true semantic mismatch.

---

# Part XIII — Current implementation status

## 16. Completed or substantially implemented

- shared immutable FunctionEntity semantic DAG;
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

## 16.1 Transitional structure that must be removed

The main remaining M1 debt is the **compatibility transition output**, not A3 construction itself.

Canonical A3 is now built incrementally while J Graph execution-semantic lowering runs:

```text
J Graph IR
   ↓
Execution Semantic Lowering
   ├─ canonical logical_ir::Plan     // built incrementally
   └─ transition_ir::LogicalPlan     // crate-private compatibility output
```

The completed transition-plan → A3 conversion pass has been removed. M1 now needs to delete the compatibility output and its legacy API/tests.

Other transitional points:

- Value still owns dense CpuStorage directly;
- physical.rs is a representation foundation, not yet a Physical Planner;
- runtime.rs still contains interpreter-oriented flattening such as ResolvedVerb { reduce, rank, ... };
- the frontend still uses modifier/train heuristics rather than the completed jsource-compatible Enqueue + 9-row parser;
- RouteRegion is still a class + operation-range prototype;
- Schedule/Transform Plan, Physical Plan, native CPU physical executor, MLIR/StableHLO/ArrayFire routes are not yet complete.

## 16.2 Deferred / later

- production CUDA backend;
- broad AD/VJP transforms;
- aggressive resource pruning;
- mature multi-route partitioning;
- complete full-J implementation.

Linux/GitHub Actions CI is not a default architectural progress gate unless explicitly requested.

---

# Part XIV — Architecture convergence roadmap

## 17. Active migration checklist

The Korean canonical document contains the authoritative detailed M0–M6 checklist. The English mirror follows the same order:

```text
M0  Freeze module ownership and dependency boundaries
 ↓
M1  Make logical_ir::Plan the sole canonical execution IR
    A3 now builds incrementally; remove the compatibility transition output/API
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

Current transitional seams are explicitly temporary:

- `transition_ir.rs` is crate-private and deletion-bound;
- `analysis.rs` owns lowering mechanics, not shared execution vocabulary;
- `execution_semantics.rs` owns target-independent execution contracts;
- `compilation.rs` owns the cross-stage analysis bundle;
- `logical_ir::Plan` is the canonical execution IR.

New functionality must not add semantics to the compatibility transition IR.

Key ownership rules:

- parser/FunctionEntity owns J semantic construction, not target decisions;
- J Graph owns graph algebra and rewrite/resource analysis, not physical layout;
- Logical IR owns executable semantic dataflow/check/effect/error contracts, not buffers or devices;
- route/schedule/planner owns realization decisions;
- physical representation owns buffer/layout/device details;
- backend kernels do not define semantic legality;
- interpreter/reference-runtime flattening is not a canonical compiler model.

## 17.1 M1 completion gate

M1 is complete only when:

- J Graph IR lowering builds logical_ir::Plan directly (**done**);
- the completed-plan `Plan::from_transition` pass is gone (**done**);
- shared execution contracts no longer belong to the legacy plan (**done**);
- compatibility `transition_ir::LogicalPlan` and its ValueId/Node/Write/verifier are gone;
- `Engine::analyze` and remaining tests no longer expose/consume the transition IR;
- CompilationAnalysis contains the J graph/rewrite/resource views plus the canonical logical plan without a transition field;
- graph origin, source span, name/version, semantic checks and observable ordering survive the final cutover.

## 17.2 M4 first vertical slice

The first native planner is deliberately simple:

```text
verified logical_ir::Plan
  ↓
deterministic all-CPU schedule
  ↓
Bind / View / Materialize / Kernel / Return
  ↓
CPU Physical Executor
```

Correctness and boundary ownership come before a sophisticated cost model.

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

If a discrepancy exists, the Korean canonical file is authoritative.

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
