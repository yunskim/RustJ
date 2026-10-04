**English mirror** | [한국어 — canonical](FOUNDATIONS.ko.md)

# RustJ Design Foundations: Why Compile J?

> Document role: **design constitution / mandatory architecture review**
>
> Canonical source: [FOUNDATIONS.ko.md](FOUNDATIONS.ko.md)
>
> Current implementation architecture and roadmap: [PROJECT.md](PROJECT.md), mirrored from [PROJECT.ko.md](PROJECT.ko.md)
>
> If this English document and the Korean canonical document disagree, the Korean `*.ko.md` document governs.

This document is not an implementation checklist. It explains why RustJ adopts a compiler-oriented architecture, which parts of J's historical interpreter model must be preserved, and when a compiler design would become a semantic regression rather than a modernization.

Review these foundations before changing:

- word formation / enqueue / parser behavior;
- J Semantic IR / FunctionEntity / JEntity boundary;
- interpreter / JIT / AOT boundaries;
- rank / CellApply / hook / fork / modifier lowering;
- name binding / locale / execute / dynamic semantics;
- Logical IR / fusion / route partitioning;
- CPU/GPU target semantics;
- unsupported-form policy;
- graph rewrite / resource analysis when semantic observability may change.

---

# Part I — The central question

## 1. Is compiling J itself a regression?

APL and J historically chose interpreter-oriented implementations even when computing resources were far more limited than today.

That raises a serious design question:

> If APL/J were intentionally interpretive, does putting a compiler at the center of RustJ risk turning J into a different, more static language?

The answer is:

> **Choosing compilation is not the regression.**
>
> **Changing J semantics to make compilation easier is the regression.**

RustJ must therefore avoid this failure mode:

```text
J-like syntax
    ↓
static restrictions introduced for compiler convenience
    ↓
something resembling J but no longer implementing J
```

The target architecture is instead:

```text
J semantics
    ↓
shared frontend + semantic model
    ↓
depending on available proof/capability
    ├─ AOT compilation
    ├─ JIT specialization
    ├─ runtime semantic execution
    ├─ external compiler route
    └─ verified library/custom-kernel route
```

### 1.1 Core invariant — Logical Array is not Physical Array

One of the boundaries this document protects is the separation between **the array semantics observed by J** and **the physical representation used by a backend**.

```text
Logical Array
    J-visible type/value
    shape
    ordered atoms
    boxed/sparse and other J-visible semantics

        ≠

Physical Array / Representation
    buffer/storage
    strides
    offset
    layout/tiling
    alignment
    memory space
    device placement
    sharding/transfer
```

The reason is fundamental:

**J semantics are not defined by a particular memory layout or device.**

For example, transpose is a logical axis/order transformation. Whether it remains a stride-only view, is absorbed into a consumer index map, or is materialized as a copy depends on representation, schedule, and target. Making that choice part of J noun identity would leak implementation details back into language semantics.

Therefore:

- do not equate logical shape/order/type with physical stride/layout;
- do not equate `ValueId` with `BufferId`;
- do not equate existence of a logical value with existence of a materialized buffer;
- do not make CPU/GPU placement part of J value identity;

### 1.2 Do not confuse a common J entity carrier with array semantics

jsource's common `A` handle is an important precedent, but it is not evidence that nouns and functions share one J-visible array semantics.

```text
semantic RHS        JEntity = Noun | Function
function POS        Verb | Adverb | Conjunction
array semantics     Noun J-visible type / shape / ordered atoms
function semantics  FunctionEntity construction / POS / operands / contracts
reference/control   lexical NAME / NameRef / binding version / provenance
physical            buffer / layout / device / allocator
```

Mandatory invariants:

- `JEntity` is a **thin boundary carrier** for parser, binding, assignment, and semantic operands. It does not merge the internal models of `Value` and `FunctionEntity`.
- lexical NAME/unresolved reference is not a JEntity POS. An executable function nameref produced by lookup may remain a `NameRef` identity inside FunctionEntity.
- a common carrier must not erase the observable timing difference between noun-value lookup/snapshot behavior and function nameref late lookup.
- Verb/Adverb/Conjunction entities do not receive noun-style shape/rank.
- jsource's gerund fake-BOX/function-payload carrier is not semantic evidence for `EntityArray`.
- gerund-like higher-order semantics first use operator-specific `GerundView` / `InterpretedEntitySequence` representations.
- a generic `EntityCollectionView` is extracted only after the same shaped-entity law appears in multiple independent J semantics.
- JEntity migration is incremental, one duplicated carrier seam at a time, not a broad frontend rewrite.
- do not replace J agreement/rank semantics with backend broadcasting/layout semantics;
- distinguish J-visible sparse/boxed semantics from concrete CSR/COO or pointer/handle encodings;
- changing physical realization must preserve the same logical array semantics.

This separation is what gives the compiler freedom to optimize safely:

```text
J logical semantics
        ↓
analysis / legality
        ↓
representation choice
        ↓
physical planning
        ↓
buffer / layout / device
```

If RustJ reverses this order and starts adapting J semantics to a preselected physical representation, that is an architectural regression.

---

# Part II — Why APL was interpretive

## 2. Historical interpretation was a language-design choice

Early APL implementations were not merely interpreters because compilers were impossible.

Important historical motivations included:

- keeping implementation experimental and interactive;
- preventing machine representation from forcing changes to language design;
- avoiding declarations that duplicated information already present in array values;
- exploiting the fact that array primitives perform large units of useful work, reducing interpretation overhead relative to scalar languages.

That history provides a direct warning for RustJ.

Do **not** reason like this:

```text
GPU compiler needs shape
    ↓
require user shape declarations

optimizer wants static type
    ↓
add static type syntax to J

backend wants stable function identity
    ↓
freeze names earlier than J does
```

Instead:

```text
J keeps its dynamic semantics
    ↓
compiler infers facts when possible
    ↓
guards/specializes where justified
    ↓
falls back to a more semantic route where proof is unavailable
```

---

# Part III — The fundamental preservation rule

## 3. Compiler capability must be a subset of language capability

RustJ's compiler routes may have restrictions.

RustJ the language implementation must not silently redefine J around those restrictions.

For any J form:

```text
valid J
   ↓
Can current route prove enough?
   ├─ yes → compile/specialize
   └─ no  → another correct route, guard, or explicit implementation gap
```

"Not currently analyzable by this route" is not the same as "not valid J."

This distinction is foundational.

---

# Part IV — Frontend fidelity

## 4. The frontend is language semantics, not preprocessing convenience

Word formation, enqueue, parser reduction, name lookup timing, assignment behavior, and derived-function construction are observable parts of J semantics.

RustJ must treat current jsource behavior through:

```text
word formation
    ↓
enqueue
    ↓
parse
```

as the frontend compatibility oracle.

A Rust representation may differ internally. Observable behavior must not.

### 4.1 Word formation

The scanner should follow J's actual state-machine behavior rather than a simplified regex grammar.

It must preserve:

- token/word boundaries;
- comments;
- quotes;
- numeric forms;
- punctuation transitions;
- control-word behavior.

### 4.2 Enqueue

Do not turn extension names into parser keywords.

Ordinary names remain ordinary names.

Enqueue may attach lookup/control metadata, but the actual J part of speech of an ordinary name may depend on parser-time lookup.

### 4.3 Parser

Do not replace jsource parser semantics with a convenient AST grammar that happens to accept common examples.

The parser must preserve:

- reduction eligibility/order;
- parser-time name/POS lookup;
- modifier construction;
- hook/fork/train construction;
- assignment and parentheses;
- J error class;
- completed derived-entity boundaries.

The semantic graph produced by parsing should retain construction topology.

---

# Part V — Semantic identity before optimization

## 5. Preserve J function construction

A J-derived function can be large. It should be represented as a shared immutable FunctionEntity/JEntity graph rather than recursively copied values.

The parser should create semantic entities reflecting actual J construction.

Do not replace:

```text
Rank(Insert(+), 1)
```

and

```text
Insert(Rank(+, 1))
```

with the same flat set of flags.

They are structurally different J-derived functions and can behave differently.

### 5.1 Source operator remains semantic parent

Applied `/` is an adverb construction.

Applied `"` is a conjunction construction.

Semantic IR should preserve those identities.

Execution notions such as:

- Reduce;
- CellApply;
- WindowView;
- Gather;
- Contract;

belong to later analysis/lowering.

---

# Part VI — Dynamic names are not compiler bugs

## 6. Name lookup must remain J-like

J's name semantics cannot be "fixed" by snapshotting everything early.

RustJ must distinguish:

- Name;
- binding/locale;
- BindingVersion;
- FunctionEntity;
- SSA ValueId.

A compiler may specialize a name only when a witness or guard justifies treating the binding as stable.

Otherwise the semantic model must retain late binding.

Right-to-left lookup/assignment effects and locale mutation must not be erased for optimization convenience.

---

# Part VII — Rank is a semantic system

## 7. Rank is not simply loop syntax

J rank semantics include:

- innate primitive rank;
- explicit rank conjunction;
- frame/cell decomposition;
- prefix frame agreement;
- cell repetition;
- empty-frame fill/prototype behavior;
- result-cell assembly;
- type/shape joining;
- error behavior.

Therefore:

> `CellApply` is a semantic lowering concept, not merely a for-loop.

Do not map CellApply directly to GPU threads until the required uniformity and error/order properties are proven.

### 7.1 Explicit rank vs implicit rank

The `"` conjunction must remain distinct from implicit rank application.

These can interact and nest.

Do not prematurely collapse nested rank boundaries into one "effective rank."

---

# Part VIII — Errors are observable behavior

## 8. J-visible errors are part of semantics

A compiler must preserve:

- error class;
- relevant precedence/order;
- interaction with try/catch/throw;
- adverse/error fallback;
- assembly-time failures;
- domain/rank/length/index behavior.

"Same successful result" is not sufficient equivalence if errors differ.

Parallel execution, reassociation, speculation, and fusion must therefore be gated by semantic proofs.

### 8.1 Diagnostics and semantics are different layers

Rich diagnostics may include:

- source span;
- line/column;
- blame word;
- operation;
- valence;
- small argument summaries.

But diagnostics must not replace the original J-compatible error kind.

---

# Part IX — Logical vs physical array semantics

## 9. Logical Array is not Physical Array

The logical noun is:

```text
type + shape + ordered atoms
```

Physical metadata such as:

- strides;
- offset;
- tiling;
- layout;
- alignment;
- memory space;
- device;
- sharding;
- buffer identity;

belongs to representation/planning.

Do not let physical layout become semantic identity unless J itself makes it observable.

### 9.1 Boxed and sparse

Boxed and sparse values are J-visible semantic representations.

Their backend encodings are separate design decisions.

Do not "optimize them away" by redefining semantics.

---

# Part X — Numeric behavior

## 10. Numeric semantics must match J

Compiler-oriented execution must preserve:

- integer overflow retry/promotion;
- primitive-specific conversion;
- tolerant floating comparison;
- fit (`!.`) policies;
- fills;
- domain behavior;
- error ordering.

SIMD/GPU lanes must not expose arbitrary lane-local error behavior if J defines a different observable result/order.

---

# Part XI — J Graph IR exists to preserve useful structure

## 11. Why have a graph layer above execution normalization?

J notation exposes useful algebraic structure.

If RustJ immediately lowers every expression to scalar arithmetic or backend-like loops, it loses:

- rewrite opportunities;
- access-pattern identity;
- fusion boundaries/opportunities;
- structured operation identity;
- symbolic resource reasoning.

Therefore J Graph IR should preserve J-derived graph meaning before execution-specific normalization.

---

# Part XII — Graph Basis vs Execution Basis

## 12. Two basis layers are intentional

### Graph Basis

Answers:

> What meaningful graph/access pattern does this J construction represent?

Examples:

- Elementwise
- Reduce
- Window
- StaticReindex
- DynamicGather
- Search
- Structured

### Execution Basis

Answers:

> What executable normalized family can implement this analyzed operation?

Examples:

- Elementwise
- CellApply
- Reduce
- WindowView
- Gather
- Contract
- LookupClassify

They must not be conflated.

A graph-level convolution can remain structured while later realizing as a library kernel or WindowView + Contract.

---

# Part XIII — Rewrite needs proof

## 13. Algebraic transformation is not free

A graph rewrite requires:

- a rule identity;
- provenance;
- a semantic equivalence witness;
- rewrite-specific fact derivation;
- verification that output semantics still match.

The original graph should remain available until a planner actually chooses an alternative.

A rewrite candidate is not yet a lowering decision.

### 13.1 Basis before rewrite

Optimization dependencies should remain:

```text
Basis
  ↓
Rewrite
  ↓
Equivalence
  ↓
Resource/Cost evaluation
  ↓
Selection
```

Do not start with arbitrary rewrite search before defining what graph identities the rules preserve.

---

# Part XIV — Resource reasoning must remain symbolic

## 14. JAXA's static-memory idea is logical, not physical

A compiler may statically know:

- extents;
- use counts;
- logical lifetimes;
- retained values;
- storage obligations;
- candidate materialization volume.

That does not mean Semantic/J Graph IR should assign:

- physical addresses;
- final offsets;
- registers;
- shared-memory banks;
- concrete device buffers.

Those depend on schedule and target.

### 14.1 Unknown is not zero

This rule is critical.

If RustJ does not yet know the size/cost of a state or implementation, it must remain Unknown.

A graph-visible lack of intermediate values does **not** prove zero backend resource cost.

---

# Part XV — Flow and Storage

## 15. Value identity, state identity, and buffer identity are different

Keep:

```text
ValueId
≠ StateResource
≠ BufferId
```

Flow analysis asks how values depend on one another.

Storage analysis asks which values/state must persist and for how long.

Bufferization asks how those requirements are physically realized.

These should not be collapsed.

### 15.1 Rematerialization

Recomputation/rematerialization is a physical planning choice.

It is legal only when semantic equivalence covers:

- effects;
- errors;
- name/state dependencies;
- ordering.

---

# Part XVI — Resource-aware pruning must be sound

## 16. Early pruning is dangerous

A partially expanded graph may look expensive locally and become cheaper after later fusion or rewrite.

Therefore early pruning is allowed only with proof that:

1. the relevant bound is local to the candidate; and
2. the pruning predicate is monotone, or an equivalent soundness argument exists.

Otherwise:

> generate the candidate, then evaluate later.

Do not turn heuristics into semantic search pruning.

---

# Part XVII — Target modeling

## 17. Target selection must not alter J meaning

Target choice is downstream of semantic analysis.

The frontend and Semantic IR must not change because a user selected CUDA, CPU, or another backend.

Target modeling should remain layered:

```text
BackendFamily
→ ArchitectureTarget
→ DeviceProfile
→ RuntimeProfile
→ resolved TargetProfile
```

Target-specific capability discovery may use compiler-only locale/path lookup, separate from J user locales.

---

# Part XVIII — External compiler routes

## 18. RustJ does not need to own every optimizer/backend

It is valid to lower verified regions through:

- MLIR/LLVM;
- StableHLO-compatible consumers;
- SPIR-V/NVVM/ROCDL;
- libraries;
- custom kernels.

But external IR restrictions do not redefine RustJ semantics.

An adapter must:

- prove its preconditions;
- insert guards where valid;
- reject/fallback when semantics do not fit.

Do not weaken J just to fit an external IR.

---

# Part XIX — Native fallback is not semantic permission

## 19. "Fallback" must still be correct

Calling a path "runtime fallback" or "native fallback" does not exempt it from J semantics.

It must still preserve:

- names;
- effects;
- errors;
- rank;
- boxed/sparse values;
- numeric rules.

RustJ may temporarily lack an implementation route for valid J. That is an implementation limitation, not permission to invent new language behavior.

---

# Part XX — Research compilers are evidence, not language specifications

## 20. APEX, Co-dfns, TAIL/Futhark, Remora, Bohrium, Lift, MLIR Linalg, and JAXA

Research systems provide valuable evidence for different parts of the array-compiler problem:

- **APEX**: morphology, SSA, property inference, interprocedural specialization.
- **Co-dfns**: compact/columnar graph representation, nanopass/data-parallel compiler organization, GPU cost reasoning.
- **TAIL/Futhark**: typed/rank-aware high-level parallel IR, fusion, nested-parallel flattening, GPU lowering.
- **Remora**: rank polymorphism, frame/cell semantics, and implicit lifting as a formal array-language model.
- **Bohrium**: delayed collection of existing NumPy-style array operations so fusion, materialization, and heterogeneous realization can be chosen later.
- **Lift**: high-level map/reduce rewrite separated from hardware mapping.
- **MLIR Linalg**: structured operations and implicit iteration preserved until later tiling/vectorization/lowering materializes loops.
- **JAXA**: logical array intent, graph basis, symbolic resource reasoning, and separation of logical from physical execution.

Together these systems reinforce several RustJ rules:

1. **J source is not an execution plan.**
2. Rank, derived entities, trains/composition, reduce/scan, and reindex/shape transforms are optimization-relevant high-level information.
3. Do not scalarize that structure merely to simplify lowering.
4. Separate full-J semantic validity from eligibility for any optimized or external route.
5. Separate logical rewrites from physical schedule/device/materialization choices.
6. Keep J as the frontend rather than shrinking the language into a compiler-convenience subset.

Restrictions in research systems—such as static scope, static rank, pure subsets, or no execute—are compiler-route preconditions, not RustJ language restrictions.

Direct comparison sources:

- Remora, *The Semantics of Rank Polymorphism*: https://arxiv.org/abs/1907.00509
- Bohrium publication index (NumPy CPU/GPU/cluster, vector VM, fusion lineage): https://bohrium.readthedocs.io/publications.html
- Lift, *A Functional Data-Parallel IR for High-Performance GPU Code Generation*: https://doi.org/10.1109/CGO.2017.7863730
- MLIR Linalg structured-operation primer / implicit-loop materialization: https://mlir.llvm.org/docs/Tutorials/transform/Ch0/

These are comparison evidence for compiler principles, not RustJ's J semantic specification.

The question is always:

> Can RustJ use the optimization idea while preserving J?

---

# Part XXI — Historical JAXA lessons

## 21. What RustJ should retain

Repeated review of the historical JAXA repositories supports these principles:

1. Preserve the graph implied by array notation.
2. Use meaningful access patterns as graph basis candidates.
3. Keep algebraic graph identity above backend realization.
4. Compose resource requirements symbolically.
5. Separate Flow from Storage.
6. Treat static memory as logical determinability.
7. Allow high-level structured operators to remain black boxes.
8. Let backend lowering choose decomposition/fusion.

## 21.1 What should not be inherited literally

Some early prototype ideas should not become current RustJ invariants:

- `"RjP` precision syntax;
- rank change always forcing a fusion boundary;
- all reshape/transpose being zero-cost;
- complete parse-time physical graph determination;
- physical offsets embedded in graph semantics;
- hard-coded architecture register counts in primitives;
- pre-resolving all names before parser use.

Later reasoning and current RustJ layering supersede those assumptions.

---

# Part XXII — Compiler legality before profitability

## 22. First prove correctness, then optimize

The ordering must be:

```text
semantic validity
   ↓
analysis facts
   ↓
legality
   ↓
resource feasibility
   ↓
cost/profitability
   ↓
selection
```

Do not use performance estimates as legality proofs.

Do not use an empirical CostProfile to decide whether a transformation is semantically valid.

---

# Part XXIII — Specialization

## 23. Specialization needs witnesses

Specialization can use:

- type;
- rank;
- shape;
- constants;
- array properties;
- binding versions.

But specialization keys should contain only facts relevant to the optimized behavior.

Otherwise specialization explodes combinatorially.

When a fact is used to remove a J-visible check or narrow a route, retain a witness or guard.

---

# Part XXIV — Parallelization

## 24. Parallelism is conditional

J's array semantics expose parallel opportunities, but not every syntactically array-shaped expression is freely parallelizable.

Before parallelizing/fusing/reassociating, consider:

- effects;
- names;
- error order;
- numeric reassociation;
- result assembly;
- tolerance;
- fill/prototype behavior;
- state resources.

The compiler may discover parallel structure early but may realize it in parallel only after legality proof.

---

# Part XXV — Hardware metadata belongs downstream

## 25. Keep schedule policy out of semantic annotations

Do not put physical policy such as:

- CUDA block size;
- tile dimensions;
- register counts;
- shared-memory layout;
- warp/subgroup assumptions;

inside J primitive semantics or ordinary semantic annotations.

Semantic contracts say what the operation means and which transformations may be legal.

Target lowering says how a particular machine may realize it.

---

# Part XXVI — What counts as a regression?

## 26. Regression criteria

A RustJ architecture change is a regression if it does any of the following without J-semantic justification:

- rejects valid J solely because a compiler route cannot handle it;
- changes word boundaries or parse reductions for implementation convenience;
- changes parser-time name/POS lookup timing;
- freezes dynamic names too early;
- replaces derived-function topology with flattened flags;
- substitutes NumPy broadcasting for J frame agreement;
- erases fill/prototype behavior;
- changes J-visible error class/order;
- treats Unknown as zero/false/safe;
- lets target choice alter frontend/Semantic IR meaning;
- exposes physical layout as semantic identity;
- requires declarations that J does not require;
- assumes a rewrite is equivalent without a witness;
- treats resource/cost preference as legality proof.

When any such change is proposed, stop and revisit the architecture rather than hiding the discrepancy in implementation details.

---

# Part XXVII — What modernization is encouraged?

## 27. Compiler-oriented improvements that preserve J

RustJ should aggressively modernize implementation where semantics remain intact.

Examples:

- Rust ownership/safety replacing C pointer/refcount machinery;
- shared immutable semantic DAGs;
- explicit SSA after semantic resolution;
- abstract interpretation/fact lattices;
- graph rewrite/equivalence infrastructure;
- symbolic liveness/resource models;
- JIT specialization with guards;
- vectorization;
- GPU lowering;
- mixed-route compilation;
- external compiler export;
- rematerialization;
- target-aware scheduling;
- verified custom/library kernels.

The standard is not "look like jsource internally."

The standard is "preserve J externally."

---

# Part XXVIII — Practical review checklist

## 28. Before changing the frontend

Ask:

- Does current jsource do this?
- Am I changing observable word/enqueue/parse behavior?
- Am I resolving a name earlier than J does?
- Am I inventing a new grammar construct for an extension?

## 28.1 Before changing Semantic IR

Ask:

- Is this field semantic identity or merely an analysis result?
- Could it be derived later?
- Does flattening lose modifier/train topology?
- Is source provenance preserved?

## 28.2 Before changing Logical IR

Ask:

- Is this target-independent?
- Are dynamic assumptions represented explicitly?
- Are semantic checks/effects/errors still visible?
- Have I turned schedule into semantics?

## 28.3 Before graph optimization

Ask:

- What Basis identity does the rule transform?
- What is the equivalence witness?
- Are rewrite-specific facts derived by a registered rule?
- Does Unknown remain Unknown?
- Am I pruning without a locality/monotonicity proof?

## 28.4 Before target lowering

Ask:

- Is the route semantically legal?
- Does target capability discovery remain downstream?
- Is this a standalone basis realization or a composite realization?
- Am I claiming support merely because the algebraic rewrite exists?

## 28.5 Before resource/cost selection

Ask:

- Which facts are logical?
- Which are schedule-dependent?
- Which are target-dependent?
- Is the metric resource feasibility or empirical profitability?
- Is an unknown implementation cost being mistaken for zero?

---

# Part XXIX — Core conclusion

## 29. RustJ's architectural contract

RustJ should be a compiler-oriented J implementation without becoming a compiler-convenience dialect of J.

The architecture is sound when it maintains this ordering:

```text
J semantics
  ↓
faithful frontend
  ↓
lossless semantic construction
  ↓
graph/algebraic analysis
  ↓
verified executable semantics
  ↓
target legality
  ↓
resource/cost planning
  ↓
physical realization
```

At every boundary:

> preserve meaning first, derive optimization freedom second.

That is the central rule against which major RustJ architecture changes should be judged.

---

# Documentation policy

Maintained user-facing documentation is Korean-canonical with English mirrors:

- `FOUNDATIONS.ko.md` — canonical design foundations
- `FOUNDATIONS.md` — English mirror
- `PROJECT.ko.md` — canonical project architecture/roadmap/checklist
- `PROJECT.md` — English mirror
- `README.ko.md` — canonical entry document
- `README.md` — English mirror
- `AGENTS.md` — English-only internal maintainer/agent instructions

Design work is authored in Korean canonical documents first.

If the Korean canonical and English mirror diverge, the Korean canonical version is authoritative.
