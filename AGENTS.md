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
- Hardware planning must distinguish logical facts, CompilationTarget layers (BackendFamily / ArchitectureTarget / DeviceProfile / RuntimeProfile), the resolved TargetProfile view, PhysicalSchedule decisions, ResourceEstimate, and backend CompiledResourceReport feedback.

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

- Treat extension names as ordinary J bindings, not reserved keywords. Enqueue keeps them as ordinary NAMEs with lookup metadata; parser-time normal J name lookup obtains the current noun/verb/adverb/conjunction class. Tokenizer/enqueuer/parser code must not hard-code extension spellings.
- Keep parameterized extension builders (for example a conv adverb) distinct from the derived computational verb/LogicalOp produced after parameters are applied.
- Keep full J semantic validity separate from eligibility for the hardware-aware analyzable array profile.
- Keep mutable weights/gradients/optimizer/checkpoint state outside primitive hidden fields as explicit StateResource identities.
- Keep ephemeral SSA ValueId, semantic StateResource, and physical BufferId distinct even though all user-visible data obey J array semantics.

- Treat built-in J primitives and extension-derived operations uniformly for hardware lowering; existing primitives such as add/reduce/transpose must participate in the same lowering-capability architecture.
- Keep BackendFamily, ArchitectureTarget, DeviceProfile, RuntimeProfile, and empirical CostProfile distinct; do not collapse them into one target identity.
- Resolve target-specific lowering/capability bindings through a compiler-only locale/path chain (device -> architecture -> family -> backend -> cpu/gpu -> generic), separate from user J locales.
- Do not infer architecture feature inheritance from numeric version ordering; use explicit locale paths and capability queries.
- Locale lookup discovers lowering/capability candidates; legality/resource/cost analysis chooses the realization.

- Preserve ordinary function-name late binding/nameref semantics; only specialize a NameRef to a stable primitive/builder identity when binding/version proof or a guard makes that legal.
- Preserve J prefix frame agreement and residual-frame cell repetition; never substitute NumPy-style trailing broadcasting.
- Preserve zero-cell rank fill-cell/prototype semantics, including result type/shape and J-defined error suppression/propagation.
- Treat boxed and sparse as J-visible semantic representations; physical boxed/sparse encodings are separate backend choices.
- Preserve comparison tolerance and !. fit semantics as semantic inputs/contracts, including fill override where applicable.
- Preserve primitive-specific overflow retry/promotion and J error precedence; do not expose arbitrary first-lane GPU errors.
- Keep `with` as an ordinary-name conjunction extension for typed semantic contracts only; physical schedule/device policy remains outside it.

- Preserve the expected part of speech carried by a J nameref; late lookup that resolves to a different noun/verb/adverb/conjunction class must follow J's domain-error semantics.
- Do not pre-snapshot all name bindings at sentence start; preserve observable right-to-left name lookup, assignment, locale mutation, and effect sequencing.
- Do not lower ranked cell application to a fixed-shape parallel map unless uniform result-cell type/shape is proven; otherwise preserve J rank-result assembly, type/shape joining, framing fill, and assembly-error semantics.

- Preserve J-visible errors as potential control flow: try/catch/throw and adverse (`::`) semantics must not be collapsed into fatal diagnostics when a handler can observe them.
- Preserve hook/fork/train observable name/effect/error ordering until purity/speculation proof permits reordering or parallelization.
- Do not erase latent derived semantics such as adverse error fallback or obverse (`:.`) inverse metadata merely because forward dataflow appears equivalent.
- Model derived J entities with their resulting part of speech; parser construction can yield derived verbs, adverbs, or conjunctions.
- Treat gerunds as contextually interpreted boxed nouns, not as a new global noun type.
- Key hardware lowering by resolved semantic operation, including valence and relevant rank/fit/numeric/effect policies; raw primitive spelling/id is insufficient.
- Keep primitive innate rank contracts valence-specific (monad, dyad-left, dyad-right).

- Prioritize implementation proof in this order: A1 semantic preservation -> A3-v0 verified single-block Logical IR -> minimal native CPU end-to-end slice. Do not make the full TargetProfile/resource/mixed-route architecture a blocking prerequisite.
- Represent missing access knowledge explicitly as an Opaque/Unknown access fact; treat it as an optimization/route barrier, not automatically as a J semantic error.
- Stage routing: the first external-route implementation should send one verified single-block region wholly to one route; add mixed contiguous-subgraph partition and boundary bridges only after that works.
- Maintain an explicit compilation-coverage manifest/golden tests for which semantic forms lower to array Logical IR, runtime semantics, guarded late binding, or UnsupportedImplementation.

- Model large J derived functions as shared immutable FunctionEntity/JEntityRef graphs (or an equivalent arena DAG), not recursively copied nested enum values.
- Let J parser reduction rules define FunctionEntity DAG shape. ADV application, CONJ application, hook, and fork must follow jsource parse productions; do not invent modifier-specific semantic AST shapes such as Insert(base) or Rank(base,r).
- Treat current jsource parser language behavior as the compatibility oracle: preserve its parse-row eligibility/order, parser-time name/POS resolution, result POS, assignment/parenthesis behavior, and completed modifier-entity boundaries. RustJ may replace jsource's C parser mechanics, but not invent a different J grammar/reduction model.
- Use PROJECT.md section A0.5 P0–P7 as the authoritative parser-migration checklist. P8 is the handoff into A1/A2/A3, not a prerequisite for starting them. Keep parser phases ordered unless a parallel test/harness change does not invalidate an earlier interface, and update checklist boxes in the same change that completes an item.
- Treat every completed modifier reduction as one first-class J function entity before any later hook/fork/train reduction. A fork operand referring to `+/`, for example, must reference the completed derived Verb entity `+/`, not retain separate `+` and `/` parser items or an unfinished modifier fragment.
- Parser row 3/4 must perform J-defined modifier-construction semantics far enough to determine success/error and the actual returned POS, matching jsource constructor behavior. Keep those target-independent construction facts separate from call-time ResolvedCallFacts and target-specific lowering metadata.
- Do not assume jsource rows 0–2 can always be replaced by deferred AST construction. If a row action can change later parser-time name/locale/POS resolution or provide a runtime value needed by a modifier constructor, use the shared runtime semantic parse action/fallback rather than continuing static parsing as if the effect/value were already known.
- The static compiler path and runtime semantic fallback must share the same parser class matcher/reduction rules; dynamic parsing is a coverage boundary, not a second J grammar.
- Keep semantic function identity/operands separate from runtime/backend executor specialization; optimized handlers must not erase J-visible derived structure.

- Treat jsource V/fgh as an execution-object cross-check, not as the authoritative source of semantic DAG edges; execution-only auxiliaries in f/g/h/local metadata must not become semantic children automatically.
- In Semantic IR, keep the source operator as the parent identity: applied `/` has its operand, applied `"` has left/right operands. Normalize to Reduce/CellApply only in Semantic Analyzer -> Logical IR lowering; use fixed-shape MapCells only after uniform result/assembly proof.

- Keep the rank conjunction (`"`) distinct from implicit cell iteration: `"` is a parent conjunction with left/right parser operands; implicit looping is common function-application semantics driven by callable rank and argument rank.
- Preserve nested explicit/innate rank boundaries until fill/assembly/error equivalence proves they can be fused; never collapse them to one effective rank by default.
- Model implicit rank execution as logical CellApply/CellApplicationPlan semantics, not as an early physical for-loop or GPU thread mapping.
- Treat jsource IRS/VIRS behavior as a lowering optimization that absorbs CellApply; it must remain semantically equivalent to the generic cell loop.
- Keep empty fill-cell/prototype behavior and heterogeneous result assembly in the CellApply contract, not hidden inside primitive kernels.
- Do not copy jsource execution flags wholesale into Semantic/Logical IR. Classify each datum as source semantic identity, static semantic contract, resolved analysis fact, optimizer proof, lowering capability, representation fact, or physical decision, and introduce it at the earliest stage where it is semantically justified.
- Keep architecture-specific primitive/NN implementation metadata out of semantic contracts. Bind resolved LogicalOps to hardware through LoweringRegistry × ArchitectureTarget; use DeviceProfile for concrete device resources and CostProfile/RuntimeProfile for implementation selection.
- Treat NN/array extension operations and built-in J primitives uniformly at hardware-lowering time. Extension names remain ordinary J bindings, and a semantic op may have multiple decomposed, fused, library, or custom-kernel realizations per target.
- Keep parser-produced FunctionEntity graphs minimal and immutable; cache derived name/self/effect/error/rank summaries in analysis side tables keyed by shared entity identity rather than recursively copying summaries into every derived node.
- Treat jsource WILLOPEN/USESITEMCOUNT/in-place/pristine/zappable style metadata as inspiration for use-def/liveness/materialization analyses; do not encode those runtime-memory-management bits as J semantic properties.

