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

- **Remora / Bohrium / Lift / MLIR Linalg — adjacent array-language / IR compiler references**
  - Remora is a comparison point for rank polymorphism, frame/cell semantics, and implicit lifting in the J/APL family. Source: https://arxiv.org/abs/1907.00509
  - Bohrium is a precedent for collecting existing NumPy-style array programs into a delayed intermediate representation and deciding fusion, materialization, and heterogeneous realization later. Publications: https://bohrium.readthedocs.io/publications.html
  - Lift is a comparison point for separating high-level map/reduce rewrites from hardware mapping. Source: https://doi.org/10.1109/CGO.2017.7863730
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
- [ ] Implement left-bound `tNVc`, noun-input adverbs, successive adverb/derived conjunction/trident action semantics. `(+ ("-)) i.4` and `(+ (@:-)) i.4` have supported frontend construction but lack existing verb-valued rank/Atop executors. Report exact sources/C results/Rust `Unsupported` as runtime boundaries rather than arbitrary mismatch exemptions. Row 6 immediate applications and locale/explicit-local/definition scopes remain pending.

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

### 5.2 Names and binding

J Name, BindingVersion, semantic FunctionEntity, and Logical SSA ValueId are distinct concepts.

Late-bound function names must retain J semantics unless specialization is justified by a binding/version witness or runtime guard.

Names must not be globally snapshotted at sentence start if doing so changes J's observable right-to-left lookup or assignment behavior.

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

**WI1 gate:** native Windows default/portable each **396 passed / 17 ignored**; fmt/clippy/build pass. Python **27 passed**. Each j64/AVX2 runtime route: **4,942 cases / 4,938 passed / 4 existing runtime boundaries / 0 failed**; stages **10,163 checks / 0 failed**; words **6,618 cases / 0 failed**. Keep 164 capture-graph and 2 static boundaries separate. Recheck all ten report binary/source hashes. This does not complete with execution, symbolic dimensions, training/AD, CUDA or optimizer execution. Full upstream/private C trace and ignored definition acceptance remain unverified; Linux/GitHub CI were not run. The subsequent ordinary-reference step resolves those NAME restrictions; implicit locatives/operators remain pending. The with adapter is not a mandatory frontend prerequisite.

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

<a id="syntax-graph-hints"></a>

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
- latest frontend validation (ordinary references): Windows default/portable each **398 passed / 17 ignored**; fmt/clippy/build pass. Python **27 passed**. Each j64/AVX2 runtime route: **4,963 cases / 4,959 passed / 4 existing runtime boundaries / 0 failed**; stages **10,208 checks**; words **6,618 cases / 0 failed**. Keep 171 capture-graph and 2 static boundaries separate. All ten report binary/source hashes verified. Source/DLL pins match JE2 namespace; full upstream/definition acceptance/private C trace equivalence remain unverified.
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
- [ ] x/y operator definitions require deferred callable invocation. Unbound special names remain Unsupported rather than falling through to globals.
- [ ] Recursive shared-parser invocation currently has an **8-level Windows stack bound**, returning LimitError. Verify depth restoration after failure; a general explicit-frame/trampoline executor and wider depth remain pending.
- [ ] Connect body graph/source frames and complete local/locale/definition invocation. Keep 17 ignored definition acceptance tests incomplete.

**Explicit-modifier gate:** Windows default/portable each: **386 passed / 17 ignored**; fmt/clippy/build pass. Python: **27 passed**. Each j64/AVX2 direct/semantic-reference/parser-capture route: **4,911 cases / 4,907 passed / 4 existing runtime boundaries / 0 failed**; stages: **10,091 checks**; words: **6,618 cases / 0 failed**. Keep **147 capture-graph boundaries** (19 invocation, 109 modifier-value, 19 ordered-effect) and 2 static boundaries separate. All ten report binary/source hashes verified. Reviewed source is 13994ffa1ed5f06f79fad6e9822a7ed2d29b1528; executed DLL release is ded7793fe5795d79eda8e7138dce94aa056edf78. Full upstream tests, definition invocation acceptance and private C trace equivalence remain unverified. Linux/GitHub CI/CUDA checks were not run.

The following modifier-scope stage implements local/global assignment dispatch and final-result/failure/effect ordering across straight-line sentences. x/y operator callables and body graph integration remain separate follow-ups. CUDA, optimizer implementation and GitHub CI remain deferred.

Sources: [cx.c invocation/local frame](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L259), [u/v and noun m/n installation](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L322), [p.c special-name resolution](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c#L616), [VXOPR executor selection](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L1316).

#### JE2/P3 implementation — modifier local/global assignment and straight-line bodies (2026-10-04, partial)

This stage extends the preceding single-sentence boundary. RuntimeParserHost supplies the enqueue environment and scoped assignment. Definition bodies use ExplicitDefinition rules, preserving local `=.` instead of promoting it under TopLevel rules. Engine keeps a per-call LocalFrame separate from globals; reads search **the current frame, then globals**, never caller frames. Install u/v and noun-only m/n there. Freeze nouns once before sharing; retain function Arcs and array payloads without copying the namespace.

Execute multiple Body sentences in the selected nonoperator mode 1/2 valence in order, returning the final actual JEntity/POS even when the last sentence assigns. A nonassignment function result followed by another sentence raises C's `noun result was required`; function assignments may continue. Restore local frames and depth on success/error. Earlier committed global effects survive later failure; failed RHS and outer assignment targets do not commit.

C comparisons distinguish these name rules:

- Global `=:` to a currently **bound private name** raises domain error. A declared but uninitialized local name permits global assignment; a later local assignment may then shadow it.
- Ordinary function NAMEs retain execution-time lookup inside bodies and after return. Do not recursively freeze them to the final local function value. After frame exit an absent global gives value error; a subsequently bound global supplies the value. Distinguish this from special u/v by-value substitution and the actual RHS returned by a final function assignment.
- C's exit-time fixing of implicit locatives u./v. is a separate mechanism, deferred with the x/y operator executor.

Checklist:

- [x] Separate simple NAME local/global assignment and current-frame-to-global lookup in nonoperator modifiers.
- [x] Verify straight-line bodies, final assignment result/POS, intermediate nonnoun errors, committed global effects and frame restoration after failure.
- [x] Compare direct definitions and real `1/2 : 0` block inputs. Supply blocks through C `0!:100` script delivery and pass the same source to Rust; do not simulate interactive blocks with a single JDo call.
- [x] Add **six Rust regressions**, **31 common C corpus cases** and **41 stage-only cases**, covering local nouns/functions/adverbs, ordinary NAME escape/rebinding, operands, failure/effect ordering and shared payload identity for a 65,536-atom array after frame exit.
- [x] The ordinary-reference step below resolves cross-frame operands, publication with local NAMEs, uninitialized local assignments and operand/local collisions. Actual implicit locatives remain a separate unsupported boundary.
- [ ] Control flow, nested definition framing, locales and x/y operator callables remain pending. Retain the 8-level bound and 17 ignored definition acceptance tests.
- [ ] Connect body dependency/effect graphs and separate body/caller diagnostic frames. Graph lowering still reports `explicit modifier body graph requires invocation scope`; execution success does not imply static graph completion.

**Modifier-scope gate:** native Windows default/portable each: **392 passed / 17 ignored**; fmt/clippy/build pass. Python: **27 passed**. Each j64/AVX2 direct/semantic-reference/parser-capture route: **4,942 cases / 4,938 passed / 4 existing runtime boundaries / 0 failed**; stages: **10,163 checks / 0 failed**; words: **6,618 cases / 0 failed**. Keep **164 capture-graph boundaries** (28 invocation, 117 modifier-value, 19 ordered-effect) and 2 static boundaries separate. All ten report binary/source hashes verified. Reviewed source: 13994ffa1ed5f06f79fad6e9822a7ed2d29b1528; executed DLL release: ded7793fe5795d79eda8e7138dce94aa056edf78. Full upstream tests, ignored definition acceptance and private C trace equivalence remain unverified. Linux/GitHub CI/CUDA checks were not run.

The following ordinary-reference step audits these cases against C and resolves ordinary NAME restrictions; x/y operator callables, body graph/source frames and control flow remain pending. Global effects may commit before a later Unsupported boundary, so it is not a safe automatic replay signal. Optimizer implementation, CUDA and GitHub CI remain deferred.

Sources: [p.c local/global lookup](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c#L631), [s.c bound-private global assignment guard](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/s.c#L718), [cx.c intermediate noun-result requirement](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L67), [cx.c implicit-locative fixing](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L679), [af.c implicit u/v handling](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/af.c#L53), [jerr.h EVNONNOUN](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/jerr.h), [i.c error message](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/i.c).

#### JE2/P3 implementation — ordinary NAME scope boundaries resolved (2026-10-04, partial)

- [x] Check ordinary NAME cross-frame operands, global publication, uninitialized local assignments and operand/local collisions against the native C oracle. Resolve the earlier conservative boundaries for this scope.
- [x] Remove four restrictions confusing ordinary NameRef with implicit locatives and the recursive name-membership audits. Preserve late references and expected POS. Calls read the current frame then globals without capturing/searching caller frames. Keep u/v operand substitution and noun snapshots.
- [x] After smf=.u, passing smf to an inner modifier does not freeze the caller's local smf; execution reads the inner local/global binding. Publishing smexport=:smf/ preserves the ordinary name and reflects global rebinding after exit. Uninitialized smf=.smf follows RHS noun/function POS: noun snapshot or function NameRef. Operand/local name collisions are valid too.
- [x] Replace one Unsupported scope golden with three behavioral Rust regressions. Verify rebinding, POS mismatch, undefined-to-defined references, failed outer target/version preservation, committed global publication and frame recovery. Add **21** common single-line C cases and **45** stage cases including them. Compare returned C atomic structures and results/errors. Include af.c in source hashes.
- [ ] Actual implicit u./v. locatives, x/y operators, control flow/nested scope and body graph/source diagnostic frames remain pending. Ordinary NAME support does not complete implicit-locative fixing or closures. Static/no-host modifier application remains an explicit boundary; execution success does not complete static graph analysis.

**Ordinary-reference gate:** native Windows default/portable each **398 passed / 17 ignored**; fmt/clippy/build pass. Python **27 passed**. Each j64/AVX2 runtime route: **4,963 cases / 4,959 passed / 4 existing runtime boundaries / 0 failed**; stages **10,208 checks / 0 failed**; words **6,618 cases / 0 failed**. Keep **171 capture-graph** and **2 static** boundaries separate. No new runtime waiver. All ten report binary/source hashes verified. Reviewed source 13994ffa1ed5f06f79fad6e9822a7ed2d29b1528 differs from executed DLL release ded7793fe5795d79eda8e7138dce94aa056edf78. Full upstream/ignored definition acceptance/private C trace equivalence remain unverified; optimizer/CUDA/GitHub CI stay deferred.

Next compare implicit locatives and deferred x/y operator callables against C, then continue nonexecuting definition body graphs/source frames. Do not automatically replay after Unsupported because effects may already have committed.

Sources: [p.c ordinary lookup / mnuvxy](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c#L616), [cx.c return-time implicit-locative fix](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L679), [af.c hasimploc / fix scope](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/af.c#L17).

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

`PROJECT.ko.md` is the authoritative architecture/design/progress document; `PROJECT.md` is its English mirror. The syntax-hint review and framework comparison are integrated in §7.3–7.4 here and §4.1.2–3 in Korean. Do not maintain separate design-review reports; keep measurement raw data in `reports/`. Future design/structure changes update the canonical document first and its English mirror in the same change.

Maintenance rules: keep the overall code-inspection/status summary in §16 and completion gates in §17/stage checklists. Put graph research in §7, extension vocabulary in §5.3, the matrix-cell proof in §17.1.1 and validation policy in §15. Date historical audits instead of presenting old v0.1/transition stages as current. Section moves update both languages and cross-links while preserving sources, adoption/support distinctions and unique validation conditions.

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
