**English** | [한국어 (canonical)](README.ko.md)

# RustJ — A J Array Compiler

RustJ aims to be a Rust-based array compiler that preserves J's language and array semantics while treating CPU and GPU as peer execution targets.

It is not a line-by-line translation of `jsource`. RustJ is designed as a compiler system with explicit boundaries between the J frontend, semantic analysis, logical/physical planning, and backends.

`Jaxa` is not the name of a separate component in the current architecture. Ideas developed in the historical `jaxa-analyzer` research repository are incorporated into RustJ's middle-end design.

## JAXA design influence — “SQL for array operations”

Earlier JAXA documents used the phrases **“SQL for neural networks”** and **“SQL for array operations.”** RustJ now uses the latter as the more general design analogy.

The important principle is not SQL-like syntax:

> **J source is not an execution plan.**

J's rank, cell/frame semantics, derived entities, trains/composition, reductions/scans, and shape/reindex operations expose high-level information about **what is being computed**. RustJ tries to preserve that information for as long as possible and lets the compiler choose **how to realize it** subject to semantic legality, resource constraints, and cost.

```text
J source / semantics
    ↓
J Semantic IR / J Graph IR
    ↓
Logical Array / Execution IR
    ↓
logical rewrite / fusion
    ↓
execution planning
    ↓
CPU / SIMD / multicore / GPU / external route
```

As a relational database separates a query from its physical plan, RustJ separates logical computation from physical realization. Fusion, materialization, layout, scheduling, and device/thread mapping are therefore not fixed by the source expression unless J semantics require them.

Full J is not a purely declarative SQL-like language: names, assignment, effects, observable errors, and control semantics still matter. RustJ preserves **correct J semantics first**, then exploits this “SQL for array operations” optimization freedom inside analyzable array regions.

The long-term framing is therefore broader than **“J with GPU support”**: RustJ aims to be **a heterogeneous array compiler/runtime using J as a high-level array language**. The analyzer and logical/physical separation ideas developed in JAXA are inherited as prior design work rather than retained as a separate current component.

## Core array model — Logical Array vs Physical Array

RustJ explicitly separates **logical array semantics** from **physical array representation**. This is a core architecture decision.

```text
Logical Array / J noun
  dtype / J-visible type
  shape
  ordered logical atoms
  boxed / sparse and other J-visible semantics

        ≠

Physical Array / Representation
  buffer / storage
  strides / offset
  layout / tiling / alignment
  memory space
  CPU/GPU placement
  sharding / transfer
```

A single logical value may therefore have multiple physical representations, while a logical intermediate does not necessarily require its own materialized buffer. Operations such as reshape, transpose, reverse, and slice keep their **logical meaning separate from whether a copy/materialization is chosen**.

RustJ preserves J semantics first; downstream planning chooses representation, layout, buffers, and devices. See the **Logical vs Physical Array** sections in [PROJECT.md](PROJECT.md) and [FOUNDATIONS.md](FOUNDATIONS.md).

### JEntity is a semantic boundary carrier, not a common array type

Repeated comparison with current jsource leads RustJ to unify nouns and functions only at the **semantic RHS boundary**, not as one physical/runtime array representation.

```text
JEntity
├─ Noun
└─ Function
    └─ POS = Verb | Adverb | Conjunction
```

`JEntity` transports `Noun | Function` losslessly across parser, binding, assignment, and semantic-operand boundaries. It is not a universal base class that merges `Value`, `FunctionEntity`, or every IR node.

Verb/Adverb/Conjunction entities do not acquire noun-style shape/rank. jsource's common `A`/`AD` allocation header is an implementation carrier; function AN/AR fields do not define J array semantics.

Gerunds likewise do not justify a generic `EntityArray`. The J-visible source remains a boxed noun, and modifier contexts may create an operator-specific `GerundView` / `InterpretedEntitySequence`. A generic `EntityCollectionView` is extracted only if multiple independent J semantics demonstrate the same shaped-entity algebra.

- unify semantic transport/identity boundaries;
- keep noun values, function entities, lookup/reference state, and physical storage distinct;
- do not assign noun shape/rank to functions;
- defer generic EntityArray/EntityCollectionView;
- allow a minimal JEntity seam during M2 when it removes a duplicated, well-tested carrier.

```text
J Source
  ↓
Frontend / parser-time J semantics
  ↓
J Semantic Construction IR / FunctionEntity
  ↓
J Graph IR / Graph Analyzer
  ↓
Execution Semantic Lowering
  ↓
Verified Logical Execution IR / Plan
  ↓
Route Partition
  │
  ├─ region(s): RustJ native → Logical Optimizer → Schedule → Physical Plan → Executor
  ├─ region(s): MLIR family → LLVM/NVVM/ROCDL/SPIR-V
  ├─ region(s): StableHLO-compatible subset → external compiler
  └─ region(s): verified library/custom-kernel route
```

**The canonical source for detailed architecture, compiler-stage boundaries, IR design, implementation planning, checklists, supported semantics, and validation policy is [PROJECT.ko.md](PROJECT.ko.md). Its English mirror is [PROJECT.md](PROJECT.md).**

**The mandatory design rationale and regression guardrails for preserving J semantics while adopting a compiler-oriented architecture are defined in [FOUNDATIONS.ko.md](FOUNDATIONS.ko.md). Its English mirror is [FOUNDATIONS.md](FOUNDATIONS.md). Review them before changing the frontend, Semantic IR, interpreter/JIT/AOT boundaries, rank/CellApply semantics, or target architecture.**

## Research foundations

In addition to jsource compatibility, RustJ's middle-end design draws from prior array-language compiler research.

- **APEX — The APL Parallel Executor**: array morphology, SSA, interprocedural specialization, and array-property/data-flow analysis.  
  Source: https://gitlab.com/bernecky/apex , https://www.snakeisland.com/ms.pdf
- **Aaron W. Hsu / Co-dfns**: Node Coordinate Matrix, columnar/inverted-table ASTs, data-parallel/nanopass compiler passes, and GPU critical-path/kernel-count/memory-traffic reasoning.  
  Paper: https://dl.acm.org/doi/10.1145/2935323.2935331  
  Source pin: https://github.com/Co-dfns/Co-dfns/tree/4e6d3e3002f2109360d24278776c5b5a4f65db0d
- **APL → TAIL → Futhark**: typed/rank-aware array IRs, explicit map/reduction nests, loop fusion, nested-parallelism flattening, and GPU lowering.  
  Dyalog'16: https://elsman.com/pdf/Dyalog16.pdf  
  FHPC'16: https://elsman.com/pdf/fhpc16futhark.pdf
- **Remora**: a comparison point for rank polymorphism, frame/cell semantics, and implicit lifting in the J/APL family.  
  Paper: https://arxiv.org/abs/1907.00509
- **Bohrium**: a precedent for lazily collecting NumPy-style array operations so fusion, allocation/materialization, host-device movement, and backend-specific execution can be delayed. It should not be read as a system that dynamically chooses CPU versus GPU for every operation.  
  Publications: https://bohrium.readthedocs.io/publications.html
- **Lift**: a comparison point for rewrite-driven progression from portable map/reduce patterns toward OpenCL-specific functional patterns and hardware mappings. It is not a strict separation of rewriting from hardware mapping.  
  Paper: https://doi.org/10.1109/CGO.2017.7863730
- **MLIR Linalg**: a reference for preserving structured operations and implicit iteration until later tiling/vectorization/lowering materializes loops.  
  Docs: https://mlir.llvm.org/docs/Tutorials/transform/Ch0/
- **JAX / jaxpr**: a contrast point for an explicitly typed, functional, first-order ANF that is convenient for transformations, not a precedent for preserving J source-combinator provenance.  
  Docs: https://docs.jax.dev/en/latest/601/jaxpr.html
- **XLA HLO Fusion**: a contrast point for a committed IR representation in which a fusion computation is already grouped. RustJ's `FusionCandidate` is an earlier pre-selection analysis object.  
  Docs: https://openxla.org/xla/operation_semantics#fusion

RustJ does not copy these compilers wholesale. It preserves **full J semantics first** and selectively adopts ideas such as:

```text
GraphIndex / AnalysisIndex
MorphologyEngine
ArrayPropertyFacts + FactWitness
SpecializationKey/cache
High-level Logical Parallel IR
Pure/effect region partition
ParameterizedLoweringRecipe
```

Restrictions such as static rank, static scope, no execute, or pure-only subsets in those research compilers are treated as **preconditions for particular compiler routes**, not as restrictions on the J language accepted by RustJ. See [FOUNDATIONS.md](FOUNDATIONS.md) Part XX for the rationale and [PROJECT.md](PROJECT.md) sections 4.24.3–4.24.10/A2/A3 for implementation contracts and checklists.

## Current status

The repository is currently in transition toward the target compiler pipeline.

- Limited J frontend and direct CPU execution path
- Shared immutable `FunctionEntity` semantic DAG preserving hook/fork/train/derived-modifier/rank structure
- J Graph IR as a separate canonical analysis surface with Graph Basis plus initial rewrite/resource analysis
- **M1 complete:** J Graph lowering builds the canonical A3 `logical_ir::Plan` directly; the transition IR/container has been removed
- A3-v0 SSA ValueIds, Execution Basis, semantic checks/constraints/effect/error contracts, verifier, and reference executor
- Explicit/direct source/control/NAME metadata, ordinary mode-3/4 calls, supported if/while/for/try, nested direct/string-explicit local scopes and A3 definition references are implemented. Frontend E2E is verified for the supported subset. Statement/control, pre-execution admission and return-check failures preserve distinct source-owned definition and nested-call frames. Named source APIs/file CLI trace nested definitions back to the original file. Full J expressiveness, body Graph/Logical analysis and compiled CFG, and general locales/locatives remain follow-up work. See PROJECT.md's frontend audit and remediation checklist.
- `VerifiedFrontend` checks NAME, assignment and definition links in the same `Program + FrontendContext`. Typed admission separates representation acceptance from execution authority and distinguishes J errors, unsupported capabilities, verifier defects and backend failures. Execution captures cannot become deferred replay inputs.
- Suffix-free decimal integer literals promote the entire numeric word to Float on signed 64-bit overflow, matching C. In-range Int precision and Float JSON round-trips are preserved; integer spelling also controls Bool/Int selection as in C (`1` is Bool, `01` is Int). Scientific real words narrow to Int only under C spelling masks and exact signed-range/integrality checks (`1e0` is Int, `1E0` is Float). Real-family decimal ratios follow C whole-word type and signed-zero rules (`1r2.0` is Float; `2r1 1e0` is an Int array). Exact rational, complex, extended and hexadecimal ratio payloads remain incomplete.
- Single/multiple string and runtime-computed string assignment targets preserve local/global scope, scalar extension, item/open and ordered partial failures. Multiple-write Graph/Logical lowering and boxed/atomic-representation targets remain unsupported.
- Enqueue preserves `name_:` flags/source without execution. Non-executing parsing emits noun `ExprKind::TakeName` or function `FunctionHead::TakeName`, retaining POS/source/sentence context. `Engine::parse_frontend` returns pre-binding transport/failure context without lookup effects or deletion. Runtime preserves noun/verb/adverb and explicit/non-nameless conjunction by-value lookup and local/global deletion order. Nameless conjunctions can transfer through another name for subsequent application. Direct application remains Unsupported after retaining deletion/nested commits, differing from C's valence error. Single-word locals are not deleted, matching C. General-path read-only loop index deletion and deferred-effect Graph/Logical lowering remain unsupported.
- `Engine::prepare_name_effects` / `execute_name_effects` execute an ordered semantic plan for top-level simple-name noun operations/TakeName, function transfer and a final single assignment. SSA values, successful effect tokens and actual scope/generation/version/deletion observations survive the boundary; failure never replays effects. Reuse checks POS first. Definition frames, modifier construction, dynamic verb calls and intermediate writes remain follow-ups.
- `prepare_name_arrays` / `execute_name_arrays` connect primitive Applies to J Graph/Logical regions with explicit Inputs and token boundaries. Applies and immutable value transport execute in one batch/SSA workspace, retaining internal error/success checkpoints. NAME effects remain in the parent plan; array ownership moves at last use. Unique storage reuse and retained external aliases are verified. Kernel fusion and Physical execution remain follow-ups. The existing pure Graph route still rejects deferred effects.
- CPU Inline/Owned/Shared storage and runtime AVX2 plus portable fallback
- Initial sparse/boxed/packed-bit support and read-only affine PhysicalArray (G1)
- M2 jsource-compatible frontend cutover, M3 logical/physical value-boundary convergence, and the M4 native Physical Planner/CPU executor remain incomplete
- CUDA backend is not implemented yet

The current priority is **M2 frontend convergence**: complete the jsource-compatible word-formation/enqueue/9-row-parser and definition/name/assignment semantics, then finish M3 logical/physical separation and connect the M4 `Verified Logical IR → Schedule/Physical Plan → CPU Physical Executor` vertical slice.

Execution after Logical Array IR is intentionally not restricted to one route. RustJ-native planning/execution can coexist with MLIR/LLVM-family lowering, StableHLO-compatible subsets, SPIR-V/NVVM/ROCDL, and verified external libraries/custom kernels. RustJ owns J semantics, legality, and lowering preconditions while reusing mature external compiler IRs and optimizers where appropriate.

RustJ's long-term **language-semantics goal is the full J language**. Not every J feature needs to enter the same hardware-aware optimization route. Forms with sufficient shape/rank/access/effect contracts can enter an `Analyzable Array Profile` and use Logical Array IR plus advanced planners or external compiler routes; other forms may retain semantics through native/runtime lowering or remain explicitly unsupported until implemented.

Extension primitives are ordinary J name bindings rather than reserved keywords. For example, the historical `conv` prototype is modeled as a parameterized adverb that produces a derived Conv computational verb/op after parameters are applied.

The actual part of speech of an ordinary name is not permanently fixed during enqueue. The parser performs normal J name-environment lookup when the name is used, while preserving late-bound nameref semantics for verbs, adverbs, and conjunctions. **The current definition-runtime subset implements current `LocalFrame` → global-namespace lookup; general user locale/path lookup is still incomplete.**

`with` remains a conjunction extension bound to an ordinary J name and carries typed semantic annotations/contracts only. Physical policy such as tile, device, and register choices does not belong in `with`.

Hardware lowering applies to built-in J primitives as well as extensions. Existing primitives such as `+`, `*`, `+/`, and `|:` participate in the same target-independent semantic capability and backend/architecture-specific lowering architecture. Target-specific tile/register numbers are not embedded in primitive meaning; lowering is looked up by primitive/op identity.

CPU/GPU target modeling is layered rather than collapsed into one `TargetProfile` blob:

```text
BackendFamily → ArchitectureTarget → DeviceProfile → RuntimeProfile
                                      ↓
                              resolved TargetProfile
```

Architecture/device lowering capabilities and overrides are resolved through a compiler-only target locale chain, separate from user J locales:

```text
device
  → architecture
  → architecture family
  → backend family
  → cpu/gpu class
  → generic
```

Locale lookup discovers candidate lowering/capability providers. Legality, `ResourceEstimate`, and `CostEstimate` decide the actual schedule/realization.

## Build and run

```sh
cargo build --release
./target/release/rustj -e '+/"1 i. 2 3'
./target/release/rustj examples/milestone.ijs
./target/release/rustj
```

JSON output:

```sh
printf 'a =: i. 4\na + 2\n' | ./target/release/rustj --json
```

## Basic validation

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test --features portable
```

After regenerating the current frontend audit reports, run `python tools/verify_frontend_reports.py --assets-root ../rustj-project-docs` to verify source/binary/C DLL hashes. File integrity does not resolve known unsupported cases.

Additional conformance, performance, and memory validation policy is defined in [PROJECT.md](PROJECT.md).

## Documentation policy

Maintained user-facing Markdown follows a Korean-canonical / English-mirror policy.

- Korean canonical project architecture, plans, and checklists: `PROJECT.ko.md`
- English mirror: `PROJECT.md`
- Korean canonical design rationale and regression guardrails: `FOUNDATIONS.ko.md`
- English mirror: `FOUNDATIONS.md`
- Korean canonical entry point: `README.ko.md`
- English entry point: `README.md`
- Internal maintainer/agent guidance: `AGENTS.md` (English-only)
- Contributor License Agreement: `CLA.ko.md` (canonical) / `CLA.md` (English mirror)
- Machine-generated measurements: `reports/*.json`, `reports/*.jsonl`

Design and implementation work is authored in the Korean `*.ko.md` canonical documents first. English `*.md` files are mirrors for readers who prefer English. If the two versions disagree, the Korean canonical document governs.

Historical details from earlier individual Markdown reports remain available through Git history.

## Historical design repositories

The primitive vocabulary, resource model, materialized-array/Flow–Storage model, and analyzer research previously spread across `JAXA`, `JAXA-complier`, `japchae`, and `jaxa-analyzer` were consolidated into the RustJ project documents as of 2026-09-30. New design decisions now belong in RustJ's Korean canonical project document, `PROJECT.ko.md`.


## License

RustJ's public open-source distribution path is **GNU General Public License version 3 (GPL-3.0-only)**.

At the same time, RustJ preserves a **separate commercial-licensing path** in the same general direction as `jsource`. A commercial RustJ license can be granted only to the extent that the applicable RustJ copyright holder(s) actually hold the rights needed to grant it.

In particular, where commercial use or distribution of RustJ depends on code or other rights derived from J SOURCE, the necessary **Jsoftware commercial J SOURCE license and other upstream rights** must also be obtained and complied with. RustJ's own `LICENSE` does not itself grant commercial rights in Jsoftware material.

Operationally:

- public RustJ distribution without the necessary Jsoftware commercial rights: **GPL-3.0-only**
- where the necessary Jsoftware commercial rights and RustJ rights are both available: a **separate RustJ commercial license may be offered**

See [LICENSE](LICENSE) for the governing notice and [COPYING](COPYING) for the full GNU GPL v3 text.

External contributors must agree to the [CLA.ko.md](CLA.ko.md) (canonical) / [CLA.md](CLA.md) so that RustJ can preserve both GPL distribution and a future commercial-licensing path where legally permitted.

Cargo metadata remains `GPL-3.0-only` for the public open-source option. The conditional commercial path is described in `LICENSE` rather than encoded as an SPDX expression.
