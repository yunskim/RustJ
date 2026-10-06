# RustJ change validation

## Core architecture invariants

- Keep **Logical Array and Physical Array / Representation separate**. J-visible type/value/shape/atom order and J-visible boxed/sparse semantics belong to logical/semantic layers. BufferId, strides, offsets, concrete layout/tiling/alignment, memory space, device placement, sharding, transfer, and materialization belong to representation/physical planning. Do not equate ValueId with BufferId or logical-value existence with a materialized buffer.

## Documentation language policy

- `AGENTS.md` is internal maintainer/agent guidance and remains English-only.
- User-facing maintained Markdown uses a Korean canonical file plus an English mirror:
  - `README.ko.md` (canonical) ↔ `README.md` (English mirror)
  - `PROJECT.ko.md` (canonical) ↔ `PROJECT.md` (English mirror)
  - `FOUNDATIONS.ko.md` (canonical) ↔ `FOUNDATIONS.md` (English mirror)
  - `CLA.ko.md` (canonical) ↔ `CLA.md` (English mirror)
- Make architecture/design/progress/checklist edits in the `*.ko.md` canonical file first, then update the corresponding English `*.md` mirror in the same change whenever practical.
- For every major compiler stage or boundary, keep a concise contract answering: purpose, inputs, outputs, semantic information preserved, decisions forbidden at that stage, upstream/downstream contract, representative example, current implementation status, verifier/tests, and known gaps. Depth of prose does not substitute for a closed stage boundary.
- When changing a framework comparison or cross-cutting architecture claim, cross-search `FOUNDATIONS`, `PROJECT`, `README`, and `AGENTS` for stale duplicate wording before finishing the change.
- If the two versions ever disagree, the Korean `*.ko.md` file is authoritative.
- Rust source identifiers, comments, doc comments, diagnostics, tests, commits, and code-facing documentation remain English unless there is a specific reason otherwise.
- Do not create `AGENTS.ko.md`.
- Preserve the project licensing model: public distribution is GPL version 3 only; a separate commercial RustJ license may be offered only where the applicable RustJ copyright holder(s) possess all rights needed to grant it. If that commercial path depends on J SOURCE rights, the applicable Jsoftware commercial license and other upstream rights must also be satisfied. Keep Cargo metadata at `GPL-3.0-only`; do not use `GPL-3.0-or-later` unless the canonical project license policy is explicitly changed.
- External contributions require acceptance of `CLA.ko.md` / `CLA.md`. The CLA must preserve contributor ownership while granting rights sufficient for both GPL distribution and separate commercial relicensing; it must never be described as granting or expanding Jsoftware-owned rights.


Use `PROJECT.ko.md` as the single authoritative document for architecture, roadmap, support status, and validation policy. Use `FOUNDATIONS.ko.md` as the mandatory rationale/guardrail document for deciding whether an architecture change preserves J or regresses into a compiler-convenience subset.

- Do not introduce `Jaxa`/`JAXA` as a current RustJ compiler component name. Use `Semantic Analyzer / Lowering` for the target-independent middle-end; `Logical Optimizer`, `Schedule/Transform Plan`, and `Physical Planner` are RustJ-native-route stages, not mandatory stages for every execution route. The `JAXA`, `JAXA-complier`, `japchae`, and `jaxa-analyzer` repositories are historical research/prototype material only; new design decisions belong in RustJ `PROJECT.ko.md`.

For architecture changes affecting frontend semantics, J Semantic IR, interpreter/JIT/AOT boundaries, rank/CellApply, dynamic-name behavior, optimizer legality, or target semantics, review `FOUNDATIONS.ko.md` first. If a proposed change contradicts it, do not silently override the document: document the new evidence/rationale and update both `FOUNDATIONS.ko.md` and `PROJECT.ko.md` in the same design change.

For every implementation change:

- Update `PROJECT.ko.md` when architecture, boundaries, roadmap, checklist, supported semantics, or validation status changes.
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

### Array-compiler middle-end invariants

- Treat J source and parser-produced semantic structure as computation meaning, not as a physical execution plan. Preserve rank/cell, modifier/derived-entity, train/composition, reduce/scan, and reindex/shape structure until semantic/logical lowering has extracted the relevant contracts.
- Treat `ValueFacts` as an extensible abstract-analysis domain, not a bag of optional metadata. Type/rank/shape/item-count/constant/array-property facts must support explicit merge/refinement semantics and may participate in worklist/fixpoint analysis.
- Keep `ArrayPropertyFacts` (for example IntegralValued, NonNegative, Unique, Sorted, Permutation, KnownRange) on values/call analysis, never as immutable FunctionEntity identity unless the property is truly intrinsic to the entity.
- When an inferred fact is used to remove a J-visible check, specialize a call, or choose a narrower route, retain a `FactWitness`/provenance or an explicit runtime guard. Never base legality on optimistic unknown facts.
- Distinguish J Name, BindingVersion, and Logical SSA ValueId. SSA is a post-semantic dataflow representation; it must not erase J namespace/locale/POS semantics.
- Represent call specialization with a cacheable `SpecializationKey` derived from only **relevant** argument facts. Do not put exact shape/constants into every key by default; define merge/widening policy to prevent specialization explosion.
- The canonical semantic/logical graph remains the source of truth. A columnar `GraphIndex`/`AnalysisIndex` inspired by Co-dfns Node Coordinate Matrix/inverted-table AST may be derived for batch passes, but must not replace semantic identity or force every pass into a matrix representation.
- Prefer small composable analysis/transform passes (nanopass style) while avoiding whole-IR cloning when fact tables/sidecars suffice.
- Preserve high-level parallel structure such as CellApply/Map/Reduce/Scan/Reindex/Loop until optimizer/scheduling. Do not scalarize these merely to simplify lowering; nested-parallel flattening is an optimization/schedule decision.
- Separate full-J semantic validity from external-route eligibility. Static-scope, static-rank, no-execute, and pure-array restrictions found variously across APEX/Co-dfns/TAIL/Futhark are route preconditions, not RustJ language restrictions; do not imply that every compared system has every restriction.
- Extract `PureArrayRegion`, `GuardedDynamicRegion`, `StatefulRegion`, and `RuntimeSemanticRegion` through effect analysis rather than rejecting whole programs because one external backend is pure/static.
- Model target realization as a `ParameterizedLoweringRecipe` when implementation depends on resolved rank/type/shape/property facts. One semantic op may yield view, fused, library, custom-kernel, or runtime fallback candidates.
- Evaluate optimization profitability with critical-path, kernel-launch, memory-traffic, temporary-materialization, transfer, and synchronization estimates in addition to arithmetic work.
- Do not remove conformability/bounds/domain/error checks merely to enable fusion. Eliminate or hoist checks only with proof/witness/guard while preserving J error semantics.

- Keep BackendFamily, ArchitectureTarget, DeviceProfile, RuntimeProfile, and empirical CostProfile distinct; do not collapse them into one target identity.
- Resolve target-specific lowering/capability bindings through a compiler-only locale/path chain (device -> architecture -> family -> backend -> cpu/gpu -> generic), separate from user J locales.
- Establish the active compiler TargetContext/target locale at compile-invocation start, but never let that target choice alter parser or J Semantic IR meaning. All hardware-lowerable built-ins and extensions must query the same selected target-locale chain.
- Use one target-selection contract for both AOT and future JIT compilation. JIT may add runtime/device discovery when resolving `auto`, but must normalize to the same TargetContext/target-locale chain and honor the same explicit target constraints.
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
- Use `PROJECT.ko.md` JE0–JE6 as the authoritative checklist for common noun/function carrier work. JE0 runs during M2. A minimal JE1/JE2 `JEntity` boundary may land during M2 only when it replaces one duplicated, well-tested parser/binding/assignment/operand carrier seam; do not perform a broad representation rewrite. Higher-order collection work waits for frontend stability, and storage-affecting changes wait for M3. Do not make JE0–JE6 a prerequisite for the first M4 CPU vertical slice.
- A common `JEntity` layer may unify semantic transport/identity at the jsource-like `RHS = NOUN + FUNC` boundary: direct semantic variants are Noun and Function(POS=Verb/Adverb/Conjunction). Do not model every jsource `A` block class as a JEntity.
- Keep JEntity thin and boundary-oriented. Do not make it a universal IR base class or merge `Value` and `FunctionEntity` internals merely to reduce enum count. Audit current `Verb`/`VerbTarget` before fixing the canonical Function payload.
- Keep runtime `p.c` and tacit-translator `pv.c` evidence distinct when using jsource as an oracle.
- Keep lexical NAME/unresolved reference/binding-version/provenance separate from JEntity POS. Preserve the jsource asymmetry where noun lookup can yield a value snapshot while function names may remain executable namerefs for later lookup.
- Do not give FunctionEntity noun-style semantic shape/rank merely because jsource functions share an `AD` allocation header; current jsource explicitly does not use function AN/AR fields.
- Do not assume every parser construction returns Function: jsource bident/trident actions may return/execute to a Noun, so common parser-action results must support JEntity.
- Do not treat jsource's gerund fake-BOX carrier as a semantic EntityArray precedent. Prefer operator-specific `GerundView` / `InterpretedEntitySequence`; extract a generic `EntityCollectionView` only after multiple independent J semantics require the same shaped abstraction.
- Keep RustJ `Value::Boxed` noun-only even if jsource uses an internal BOX-tagged carrier whose slots may contain function-typed A values. Preserve source-container shape separately when operator semantics requires it.
- Audit `decoded_gerund: Vec<FunctionEntity>` for lost source shape/order/binding information before generalizing it.
- Let J parser reduction rules define FunctionEntity DAG shape. ADV application, CONJ application, hook, and fork must follow jsource parse productions; do not invent modifier-specific semantic AST shapes such as Insert(base) or Rank(base,r).
- Treat current jsource frontend behavior through **word formation -> enqueue -> parse** as the compatibility oracle and source-level implementation specification.
- Preserve source provenance as byte spans across frontend/semantic nodes and preserve original enqueue-word indices through parser stack reductions. Attach the most precise known provenance/context to errors without changing their J-compatible error class. AOT, interpreter, JIT, and REPL must share one diagnostic pipeline; do not create execution-mode-specific error systems.
- Model diagnostics as `J ErrorKind + ErrorContext -> DiagnosticAnalyzer -> Renderer`. ErrorContext may contain phase, span, blame word, current name, semantic operation, valence, and **small** argument summaries; never retain/copy a large noun solely for diagnostics.
- Follow current jsource's error-system principle: preserve enough failure context during enqueue/parse/execution to explain the error later. Prefer structured primitive-specific analyzers (rank/agreement/index/domain/assembly) over hard-coded prose at arbitrary call sites, and never let diagnostic analysis replace or mask the original J error.
- Human-facing diagnostics should render source name, 1-based line/column, source excerpt, caret/range, and a readable class such as `SyntaxError` or `RankError`; machine-readable `Error::kind()` remains the J-style contract used by conformance tests. Port `w.c::state` transition behavior, `jtenqueue` classification/POS/lookup semantics, and `p.c` 9-row reduction behavior faithfully; representation may be Rust-native but observable/frontend semantics must not be reinterpreted. Preserve word boundaries, enqueue primitive/name/assignment classification and lookup flags, parse-row eligibility/order, parser-time name/POS resolution, result POS, assignment/parenthesis behavior, and completed modifier-entity boundaries. RustJ may replace C pointer tagging/refcount/in-place machinery, but not invent a different J frontend language model.
- Generalize jsource's enqueue-time `spellin -> ds` primitive lookup as a `PrimitiveResolver`: resolve core J primitives and compile-profile-enabled extension primitives there, not by adding parser grammar cases. Return target-independent semantic primitive handles plus stable lowering keys; select hardware implementations only after semantic/logical analysis through the active TargetContext.
- Use PROJECT.ko.md section A0.5 F0–F2 + P0–P7 as the authoritative frontend/parser migration checklist. P8 is the handoff into A1/A2/A3. Prefer F0 -> F1 -> F2 before replacing parser reductions so parser work consumes the jsource-compatible enqueue queue rather than today's convenience tokens.
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
- Keep parser-produced FunctionEntity graphs immutable and shared, but let each node own its intrinsic FunctionSemanticInfo (construction facts, valence/rank contract, binding/self/effect/error/atomicity/latent summaries). Compute derived summaries bottom-up from completed child nodes; use side tables only for pass-local/recomputable/use-def/target-dependent analyses.
- Treat the parser-produced J Semantic IR as the authoritative semantic layer. Do not rewrite away J modifier/train identity merely to make later IR easier. Store intrinsic target-independent semantics on the FunctionEntity itself; store call-dependent facts on call/Logical IR nodes, pass-dependent proofs separately, and target/physical information only downstream.
- A compiler fact must never become part of J semantic identity solely because a later pass needs it. Node-owned semantic information is limited to facts invariant for that entity across calls and targets. If a fact depends on actual arguments, optimization proof, architecture, device, cost, layout, or schedule, keep it on the appropriate downstream node/analysis.
- Treat jsource WILLOPEN/USESITEMCOUNT/in-place/pristine/zappable style metadata as inspiration for use-def/liveness/materialization analyses; do not encode those runtime-memory-management bits as J semantic properties.

