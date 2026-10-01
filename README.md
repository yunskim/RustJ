**English** | [한국어 (canonical)](README.ko.md)

# RustJ — A J Array Compiler

RustJ aims to be a Rust-based array compiler that preserves J's language and array semantics while treating CPU and GPU as peer execution targets.

It is not a line-by-line translation of `jsource`. RustJ is designed as a compiler system with explicit boundaries between the J frontend, semantic analysis, logical/physical planning, and backends.

`Jaxa` is not the name of a separate component in the current architecture. Ideas developed in the historical `jaxa-analyzer` research repository are incorporated into RustJ's middle-end design.

```text
J Source
  ↓
Frontend
  ↓
J Semantic Array IR
  ↓
Semantic Analyzer / Lowering
  ↓
Verified Logical Array IR / Execution Plan
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
- Foundations of J Semantic IR
- Primitive contracts and dtype/shape/rank facts
- Initial LogicalPlan
- CPU Inline/Owned/Shared storage
- Runtime AVX2 plus portable fallback
- Initial sparse/boxed/packed-bit support
- Read-only affine PhysicalArray (G1)
- Shared FunctionEntity/hook/fork/train/derived-verb semantics exist, but the J Semantic Array IR → Semantic Analyzer/Lowering boundary is not yet fully separated
- CUDA backend is not implemented yet

The current compiler-architecture work builds on the shared FunctionEntity/hook/fork/rank-conjunction representation, adds valence-specific innate-rank contracts and logical `CellApply` planning, and makes the `J Semantic Array IR → Semantic Analyzer/Lowering → Logical Array IR/Plan` boundary explicit in code.

Execution after Logical Array IR is intentionally not restricted to one route. RustJ-native planning/execution can coexist with MLIR/LLVM-family lowering, StableHLO-compatible subsets, SPIR-V/NVVM/ROCDL, and verified external libraries/custom kernels. RustJ owns J semantics, legality, and lowering preconditions while reusing mature external compiler IRs and optimizers where appropriate.

RustJ's long-term **language-semantics goal is the full J language**. Not every J feature needs to enter the same hardware-aware optimization route. Forms with sufficient shape/rank/access/effect contracts can enter an `Analyzable Array Profile` and use Logical Array IR plus advanced planners or external compiler routes; other forms may retain semantics through native/runtime lowering or remain explicitly unsupported until implemented.

Extension primitives are ordinary J name bindings rather than reserved keywords. For example, the historical `conv` prototype is modeled as a parameterized adverb that produces a derived Conv computational verb/op after parameters are applied.

The actual part of speech of an ordinary name is not permanently fixed during enqueue. The parser performs normal J lookup against the current local/locale binding when the name is used, while preserving late-bound nameref semantics for verbs, adverbs, and conjunctions.

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
