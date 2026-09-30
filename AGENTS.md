# RustJ change validation

Use `PROJECT.md` as the single authoritative document for architecture, roadmap, support status, and validation policy.

- Do not introduce `Jaxa`/`JAXA` as a current RustJ compiler component name. Use `Semantic Analyzer / Lowering` for the target-independent middle-end; `Logical Optimizer`, `Schedule/Transform Plan`, and `Physical Planner` are RustJ-native-route stages, not mandatory stages for every execution route. The `JAXA`, `JAXA-complier`, `japchae`, and `jaxa-analyzer` repositories are historical research/prototype material only; new design decisions belong in RustJ `PROJECT.md`.

For every implementation change:

- Update `PROJECT.md` when architecture, boundaries, roadmap, checklist, supported semantics, or validation status changes.
- Do not create new design/progress/checklist Markdown reports. Keep machine-generated measurement data in `reports/` when useful.
- Add a reproducer and regression coverage for every semantic bug fix.
- Extend conformance coverage for newly supported semantics.
- Run Rust default and portable tests, fmt, clippy, and relevant Python harness tests.
- For semantic/kernel changes, compare available C reference variants when the environment supports them. Report known deviations separately from passes.
- For storage changes, check alias preservation, transactional assignment, allocation behavior, overlap, and retained-memory limits.
- For SIMD changes, cover tails, integer overflow/promotion, exceptional floating-point values, and portable fallback.
- State which checks actually ran. Do not claim upstream suite, Miri, sanitizer, GPU, Linux CI, or remote CI coverage unless executed.
- CUDA implementation remains deferred until the user requests resumption and a verifiable GPU environment is available.


- Do not require every Logical Array IR program to pass through a RustJ-owned optimizer/code generator. Keep verified external lowering routes open (especially MLIR/LLVM and supported StableHLO subsets).
- Do not weaken RustJ Logical IR semantics merely to match an external IR. External adapters must declare preconditions and reject or guard unsupported semantics.
- Hardware planning must distinguish logical facts, TargetProfile hard facts, PhysicalSchedule decisions, ResourceEstimate, and backend CompiledResourceReport feedback.

- Keep semantic StorageRequirement separate from physical MaterializationDecision/BufferId; bufferization stays downstream.
- Keep schedule/transform choices out of Logical Array IR payload semantics.
- Require Logical IR verification at compiler boundaries; dynamic assumptions must be explicit witnesses/guards.
- Model observable side-effect/error ordering explicitly; do not infer it only from source statement order.
- Treat native fallback as optional capability, not a guaranteed catch-all for unsupported external lowering.

- Keep semantic AccessRelation free of physical stride/alignment/address-space facts; those belong to downstream RepresentationFacts or guarded adapter preconditions.
- Keep logical InvarianceFact target-independent; subgroup/lane uniformity, tail predication, and atomic realization are derived after schedule/target mapping.

- Keep ResourceEstimate independent of empirical CostProfile; use a separate CostEstimate for performance ranking.
- Treat RoutePartition as a recomputable compilation plan and allow mixed native/external regions within one program.
- Keep analysis-lattice states (unknown/unreachable) separate from user-visible J semantic errors.

- Treat extension names as ordinary J bindings, not reserved keywords; enqueue/name classification may resolve their part of speech through injected registry/environment data, but tokenizer/parser code must not hard-code spellings.
- Keep parameterized extension builders (for example a conv adverb) distinct from the derived computational verb/LogicalOp produced after parameters are applied.
- Keep full J semantic validity separate from eligibility for the hardware-aware analyzable array profile.
- Keep mutable weights/gradients/optimizer/checkpoint state outside primitive hidden fields as explicit StateResource identities.
- Keep ephemeral SSA ValueId, semantic StateResource, and physical BufferId distinct even though all user-visible data obey J array semantics.
