[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)

<a id="a152-jaxa의-static-memory-claim을-rustj에서-해석하는-방식"></a>
#### A1.5.2 JAXA의 static-memory claim을 RustJ에서 해석하는 방식

JAXA의 중요한 주장 중 하나는 **배열 연산을 J DSL로 정적으로 표현하면 graph를 실행하기 전에 필요한 메모리 구조를 상당 부분 결정할 수 있다**는 것이다. RustJ는 이 주장을 버리지 않되, `logical memory`와 `physical memory`를 구분한다.

J Graph 단계에서 정적으로 계산할 수 있는 것:

- 각 logical ArrayValue의 shape / rank / dtype fact
- shape가 known이면 atom count
- producer-consumer use-def와 fan-out
- graph-order lifetime / last-use
- Hook/Fork의 live-across value
- pipeline/branch 내부 intermediate
- 어떤 value가 materialization-elision 후보인지
- static reindex/view가 virtual하게 유지될 가능성
- reduction accumulator / temporary / working_state의 symbolic requirement
- representation model이 주어졌을 때 logical extent의 represented byte size

J Graph 단계에서 **아직 결정하지 않는 것**:

- 실제 register allocation
- register class별 사용량
- shared/LDS/scratchpad의 concrete byte 수
- tile/workgroup별 local storage
- packed-bool/box/sparse 등의 final representation
- alignment/padding/buffer offset
- spill/occupancy
- exact physical allocation/reuse

따라서:

~~~text
J syntax / J Graph
    ↓
Static logical memory analysis
    shape → atom count → use/lifetime → materialization opportunity
    ↓
Representation + Schedule + Target
    ↓
ResourceEstimate / bufferization
    register/shared/global bytes, peak physical memory, traffic
~~~

현재 구현:

- `src/j_graph_memory.rs`의 `StaticMemoryAnalysis`
- `LogicalExtent { shape, atoms, dtype }`
- `GraphOrderLiveRange { defined_at, last_use }`
- `PipelineIntermediate`, `RetainedAcrossBranch`, `BranchIntermediate`, `VirtualView` materialization opportunity
- explicit `AtomRepresentation`을 제공할 때만 byte size 평가
- 모든 logical value를 materialize한다고 가정한 `graph_order_peak_materialized_bytes()` 제공

이 conservative peak는 최종 resource estimate가 아니다. fusion/materialization selection 전의 upper-bound-like graph estimate이며, JAXA의 핵심인 **“graph에서 memory obligation을 정적으로 계산한다”**는 주장을 검증하기 위한 분석 결과다.

현재 `j_graph_resource` 최소 evaluator는 region별 `internal_atoms`, `elidable_materialization_atoms`, `retained_live_atoms`, `graph_order_peak_live_atoms`, `has_reduction_accumulator`, `has_unknown_resource_requirement`를 계산한다. 이것은 target-independent logical resource summary다.

향후 `ResourceCompositionRule` evaluator는 다음 불변조건을 따른다.

- Pipeline: stage temporary는 lifetime이 겹치지 않으면 재사용 가능하며 internal edge materialization을 제거할 수 있다.
- Reduction: 큰 producer result 대신 accumulator state로 직접 소비할 수 있는지를 표현한다.
- Branch: sibling branch의 live temporary와 retained input이 겹칠 수 있다.
- Join: 두 branch result가 join 시점에 동시에 live할 수 있다.
- memory traffic은 node resource의 단순 합이 아니라 **materialized edge의 write/read**를 중심으로 계산한다.
- register pressure는 primitive register 숫자의 합이 아니라 **simultaneously-live symbolic values**를 중심으로 계산한다.


<a id="a2--extension-primitive-registry와-analysis-contract"></a>
### A2 — Extension Primitive Registry와 analysis contract

> **구현 주의:** 아래 목록 전체는 A2의 장기 architecture inventory다. A3-v0/첫 CPU vertical slice를 막는 하나의 거대한 선행 milestone로 취급하지 않는다.
>
> **A2-v0 blocking subset**
> - built-in/extension이 공유하는 최소 semantic capability interface
> - valence별 rank + shape/type/effect/error 최소 contract
> - `ValueFacts`의 최소 Type/Rank/Shape domain + compile-time Witness
> - Map/Reduce 수준의 IterationDomain
> - `AccessFact = Known(simple) | Opaque`
> - lowering eligibility/coverage manifest
> - 첫 실행 op에 필요한 native CPU lowering
>
> **A2-later**
> - richer ArrayPropertyFacts + full morphology worklist/fixpoint
> - interprocedural morphology/specialization cache
> - GraphIndex/AnalysisIndex batch-analysis view
> - full TargetProfile/TargetQueries
> - target locale chain
> - ResourceEstimate/CostEstimate/CompiledResourceReport
> - mixed RoutePartition boundary bridge
> - richer Window/Scan/Gather/Scatter access/resource model



- [x] extension name을 parser keyword로 만들지 않고 ordinary name binding으로 등록한다. **F1/P4 현재 구현:** extension catalog가 있어도 enqueue는 NAME으로 유지하고 parser/runtime name environment가 binding/POS를 해석한다.
- [x] Enqueue는 extension도 ordinary NAME/lookup metadata로 처리하고, parser-time normal name lookup이 현재 binding의 품사를 결정하게 한다. `tests/enqueuer.rs`의 extension-like spelling 회귀와 F1 checklist를 근거로 한다.
- [ ] parameterized adverb(`conv`, `linear` 등)와 그 결과 derived computational verb/op identity를 분리한다.
- [ ] built-in과 extension-derived computational entity가 공유하는 semantic capability interface를 정의한다.
- [ ] extension builder(adverb/conjunction/verb) identity와 derived computational entity identity를 분리한다.
- [ ] `PrimitiveSpec`을 semantic identity/version record로 축소하고 semantic capability interface와 lowering/realization registry를 분리한다.
- [ ] monad / dyad-left / dyad-right별 innate RankSpec과 cell axis-role contract를 정의한다.
- [ ] `IterationDomain`, `AccessRelation`, `InvarianceFact`, `ConstraintSet`, `SemanticMaskSemantics`를 정의하여 leading axis보다 일반적인 hardware-relevant logical contract를 만든다.
- [ ] access-pattern taxonomy(Map/Reduce/WindowReduce/Scan/StaticReindex/Gather/Scatter)를 최소 형태로 정의한다.
- [ ] shape/dtype/effect/alias/semantic-reference 계약을 정의한다.
- [ ] `ValueFacts`를 Type/Rank/Shape/ItemCount/Constant/ArrayProperty/Constraint의 abstract-domain 집합으로 정의한다.
- [ ] `ArrayPropertyFacts` domain(IntegralValued/NonNegative/Unique/Sorted/Permutation/KnownRange 등)의 최소형과 primitive transfer rule interface를 정의한다.
- [ ] optimizer가 사용하는 추론 fact에 `FactWitness`/provenance를 연결하는 최소 contract를 정의한다.
- [ ] logical `ConstraintSet`과 downstream `RepresentationFacts`를 분리한다.
- [ ] `CompilationTarget = BackendFamily + ArchitectureTarget + DeviceProfile + RuntimeProfile`을 정의하고, 기존 `TargetProfile`은 resolved effective view로 사용한다.
- [ ] compile invocation 시작 시 `CompilationTargetLocale` / `TargetContext`를 확정하고 lowering lookup의 root로 사용한다.
- [ ] AOT CLI와 향후 JIT API가 동일한 `TargetSelector/TargetOptions` contract를 사용하게 한다. JIT는 runtime discovery를 추가 evidence로만 사용하고 별도 target-selection 체계를 만들지 않는다.
- [ ] `target=auto`와 explicit target constraint의 precedence를 정의하고 AOT/JIT 양쪽에서 동일하게 테스트한다.
- [ ] built-in primitive와 extension-derived op가 source identity와 무관하게 동일 active target locale/path에서 lowering/capability를 조회하는 테스트를 추가한다.
- [ ] execution hierarchy/register allocation rules/memory & resource coupling/compute & execution scope/sync & memory ordering/data movement/execution mode/ABI capability를 architecture/device profile에 올바르게 분리한다.
- [ ] compiler target locale chain(device → architecture → family → backend → cpu/gpu → generic)을 정의한다.
- [ ] built-in J primitive도 extension과 동일하게 target lowering binding을 locale chain에서 조회한다.
- [ ] lowering lookup key에 primitive/source identity뿐 아니라 resolved valence와 derived rank/fit/numeric semantics를 포함한다.
- [ ] hard target facts와 empirical `CostProfile`을 분리한다.
- [ ] Physical Plan에 logical-axis mapping/tile/vector-subgroup-workgroup/memory-space/layout/pipeline 정보를 기록한다.
- [ ] `ResourceEstimate`를 graph + schedule + TargetProfile의 함수로 계산하고 `CostEstimate`를 별도 계층으로 둔다.
- [ ] backend가 실제 register/spill/shared-memory 결과를 돌려주는 `CompiledResourceReport`와 re-plan 경로를 정의한다.
- [ ] `TargetProfile`을 stable facts와 architecture-specific `TargetQueries`로 분리한다.
- [ ] Logical Array IR → MLIR export adapter의 최소 contract를 설계한다.
- [ ] whole-program route 선택이 아니라 subgraph/region 단위 `RoutePartition`과 boundary value bridge를 정의한다.
- [ ] StableHLO로 안전하게 내릴 수 있는 subset을 명시하고 unsupported semantics를 거부하는 규칙을 만든다.
- [ ] resource 함수는 고정 숫자가 아니라 fusion context/target에 대한 함수로 둔다.
- [ ] register estimate는 primitive별 합이 아니라 scheduled liveness peak로 계산한다.
- [ ] accumulator requirement(logical)와 accumulator realization(schedule/target)을 분리한다.
- [ ] scratchpad/shared usage를 tile/reuse/pipeline-stage 함수로 계산한다.
- [ ] 첫 extension set(`relu`, `linear`, `conv2d`, `flatten`, reduction/pool)을 port한다.
- [ ] noun snapshot과 verb/adverb/conjunction nameref late lookup, alias/shadow/rebind, `f.` fix semantics를 구분하는 테스트를 추가한다.
- [ ] mutable extension state가 hidden verb field가 아니라 explicit StateResource로 나타나는 테스트를 추가한다.
- [ ] standard-J reference definition이 가능한 extension은 차등 oracle test를 추가한다.

완료 조건: 새 NN primitive 하나를 추가할 때 scanner/parser 수정 없이 registry/spec/lowering만 추가하면 되고, Semantic Analyzer가 rank·iteration domain·axis semantics·access relation·numeric/dependency/effect contract를 읽을 수 있으며, RustJ-native route에서는 별도 TargetProfile을 이용해 schedule/ResourceEstimate를 만들고 external route에서는 adapter가 같은 Logical IR contract를 검증해 lowering할 수 있다.

<a id="a3--logical-execution-ir-core-verification-scheduling-boundary"></a>
### A3 — Logical Execution IR core, verification, scheduling boundary

> **단계화:** APEX/Co-dfns/TAIL 반영 항목은 단계적으로 도입한다. 첫 verified single-block Logical IR(A3-v0)은 SSA ValueId + 최소 Type/Rank/Shape/Witness + verifier를 우선한다. full GraphIndex, full morphology fixpoint, interprocedural SpecializationKey cache, richer ArrayPropertyFacts는 A3-v0의 선행조건이 아니며 v1/later에서 추가한다.
>
- [x] A3-v0에 SSA `ValueId`와 explicit single Function/Region/Block/`Return` Terminator 최소 구조를 정의했다.
- [x] J name/symbol identity, `BindingVersion`, A3 SSA `ValueId`를 서로 다른 타입/field로 구분한다.
- [ ] immutable semantic/Logical DAG에서 유도되는 `GraphIndex` / `AnalysisIndex` sidecar(parent/depth/preorder/subtree/op/entity/scope/use-def/source-origin)를 정의한다.
- [ ] graph index는 derived analysis view이며 semantic DAG의 canonical identity를 대체하지 않는다는 verifier/invariant를 둔다.
- [ ] morphology transfer를 worklist/fixpoint로 실행할 최소 `MorphologyEngine` interface를 정의한다.
- [ ] interprocedural summary와 call-site specialization을 `SpecializationKey` + cache로 표현한다.
- [ ] specialization key에 포함할 fact relevance 정책과 code-explosion merge/widening 정책을 정의한다.
- [ ] PureArray/GuardedDynamic/Stateful/RuntimeSemantic region 분류를 EffectAnalysis/RoutePartition contract에 추가한다.
- [ ] `CellApply/Map/Reduce/Scan/Reindex/Loop` 같은 high-level parallel structure의 early scalarization을 금지하는 Logical IR invariant를 추가한다.
- [x] A3 `CallOp + ExecutionBasisPayload` 공통 contract와 `ExecutionBasisKind` identity를 정의하고, target-specific realization은 `ExecutionBasisLoweringCapability` registry로 분리했다. 이 vocabulary는 GraphBasis와 별도 계층이다.
- [x] J syntax-derived `StructuralOpportunity` sidecar를 추가했다. `@:`는 Pipeline, hook/fork는 BranchJoin topology와 live-across/shared-input provenance를 analysis/A3 IR에 보존한다.
- [x] StructuralOpportunity discovery와 semantic legality/target feasibility/physical fusion commitment을 서로 다른 단계로 분리했다.
- [ ] adjoint/VJP expansion이 생기면 data-adjoint/parameter-adjoint branch를 `ParallelFanOut` opportunity로 연결한다.
- [ ] name-bound derived verb의 FunctionEntity/topology summary를 binding version + SpecializationKey로 전파해 `@:`/hook/fork opportunity가 call boundary에서 사라지지 않게 한다.
- [ ] StructuralOpportunity와 use-def/GraphIndex를 결합해 pipeline intermediate materialization-elision 및 branch live-range 분석을 일반화한다.
- [ ] J Graph IR의 GraphForm/GraphHint vocabulary를 Cut/Window, Dot/Contract, Power/Iteration, Key/GroupBy 등 J graph algebra 전반으로 확장한다.
- [ ] primitive마다 J Graph IR용 shape/dtype/rank/effect/resource rule reference를 연결하고, 아직 모르는 항목은 명시적 Unknown으로 둔다.
- [ ] Graph basis verb 위 rewrite/equivalence rule을 J Graph IR에서 표현하여 동일 execution semantics를 갖는 여러 J graph 후보를 생성할 수 있게 한다.
- [ ] target ResourceEstimate/register/shared-memory model을 opportunity별 feasibility query로 연결하되 Logical IR payload에는 concrete hardware allocation을 넣지 않는다.
- [x] ResolvedInstantiation 최소 record를 정의하여 우선 target/valence/input-output dtype·rank/requested-rank instance를 기록한다. cell-rank/value-role/numeric-policy 확장은 후속 refinement다.
- [x] ValueRoleFacts 최소형을 추가했다. 현재 ShapeVector/AxisPermutation/IndexVector/CountVector를 실제 분석에서 생산하며 나머지 role enum은 후속 basis가 사용한다.
- [x] J-visible predicate failure를 표현하는 first-class zero-result SemanticCheck를 A3 IR에 정의하고 compiler assertion과 분리했다.
- [x] ExecutionBasisExpansion sidecar에 applicability ConstraintSet + equivalence witness를 두고 original semantic/structured identity를 보존한다. 첫 rule은 E. → WindowView + CellApply(Match)다.
- [x] A3-v0 correctness executor 범위를 Elementwise/CellApply/Reduce/StaticReindex/IndexSpace/SemanticCheck 중심으로 제한했다. `logical_executor::execute_closed`는 closed expression reference path이며 native Physical Executor와는 별개다.
- [x] A3 verifier negative tests, runtime/reference-equivalence tests, Rank/Reduce 및 E. expansion composition tests의 golden scaffold를 추가했다.
- [x] `ParameterizedLoweringRecipe` interface와 `LoweringRegistry` 후보 생성 경로를 정의해 resolved call facts + target capability에서 multiple realization 후보를 만들 수 있게 했다. cost ranking/schedule 선택은 아직 downstream 과제다.
- [ ] pure graph region과 CFG region을 구분한다.
- [x] v0 `ConstraintSet + FactWitness`를 정의하고 PrefixAgreement/CellFrameAgreement/IndicesInBounds를 우선 연결했다. runtime branching `Guard`는 v1로 유지한다.
- [x] v0 `EffectSummary + SpeculationSemantics` resolved-call interface를 정의했다. explicit `EffectToken`은 v1로 유지한다.
- [x] v0 `PossibleErrors { known, unknown }`와 first-class `SemanticCheck`로 MayRaise를 보존한다. primitive별 완전한 error-set refinement와 exceptional CFG edge는 후속이다.
- [x] A3 `DestinationRelation`을 정의해 logical alias/reuse legality seam과 physical `BufferId`를 분리했다. 현재 call 기본값은 보수적으로 `Unknown`이다.
- [x] A3 verifier가 schema/container/op-value producer/order/basis payload/instantiation/constraint/zero-result check invariants를 검증한다.
- [x] A3-v0 `SemanticCapabilityView`를 정의해 result facts(shape/type/rank 포함), iteration/axis domain, access, effect, speculation, possible errors, destination/alias seam을 공통 API로 노출한다. richer property/alias interface는 후속 확장한다.
- [ ] schedule/transform representation을 Logical payload IR과 분리한다.
- [ ] external adapter capability negotiation과 guarded lowering을 정의한다.
- [x] A3 `IrSchemaVersion`과 compiler version/primitive registry version provenance를 IR header에 추가했다.
- [x] v0 single-block graph/check/witness/verifier/reference-executor golden tests를 추가했다. branch/loop/effect-token/dynamic-guard tests는 v1로 유지한다.

완료 조건: Logical IR이 RustJ-native planner와 external adapter 양쪽에서 동일한 verifier/interface contract를 통해 소비될 수 있고, buffer/layout/schedule을 넣지 않아도 control/effect/dynamic constraint semantics를 잃지 않는다.

구현은 단계적으로 한다.

```text
A3-v0
  single Function
  single Region / single Block
  pure array ops
  SSA ValueId
  verifier
  shape/axis/numeric contracts
  simple Known access or explicit Opaque access fact
  EffectSummary/Speculation interface
  ConstraintSet + compile-time Witness의 최소형
  Basis core: Elementwise / CellApply / Reduce / StaticReindex / IndexSpace
  first-class SemanticCheck + possible J error set

A3-v1
  multi-block CFG
  branch / loop / runtime Guard
  try/catch/throw exceptional edges
  EffectToken
  richer alias/destination analysis

A3-v2
  portable serialization/version migration
  async/control-effect extensions as needed
```

즉 장기 IR이 Region/Block을 지원한다고 해서 첫 구현에서 전체 CFG framework를 완성할 필요는 없다.

<a id="g1--논리-값과-물리-표현의-경계"></a>
### G1 — 논리 값과 물리 표현의 경계

상태: **완료**

- [x] ValueId와 BufferId 분리 기초
- [x] generation/scoped BufferId
- [x] BufferLease
- [x] read-only affine PhysicalArray
- [x] shape/strides/offset
- [x] encoding/dtype/backing 검증
- [x] signed span checking
- [x] empty/scalar policy
- [x] standard logical layout 판정
- [x] Windows 기본/portable 회귀

아직 evaluator/LogicalPlan 실행 경로와 완전히 연결된 것은 아니다.

<a id="g2--복사-없는-structural-view"></a>
### G2 — 복사 없는 structural view

- [ ] transpose stride permutation
- [ ] reverse negative stride
- [ ] fill 없는 take/drop/slice view
- [ ] compatible reshape metadata-only
- [ ] agreement 결과의 zero-stride lowering
- [ ] logical-order materialization
- [ ] backing identity/payload copy 테스트
- [ ] transpose→reverse→slice 조합 테스트

<a id="g3--cpu-kernel-및-cell-mapping-연결"></a>
### G3 — CPU kernel 및 cell mapping 연결

- [ ] contiguous / fixed-stride / general-stride 경로
- [ ] add를 첫 실제 연결 operation으로 사용
- [ ] SIMD contiguous fast path 유지
- [ ] explicit cell mapping과 physical view 연결
- [ ] alias proof 없는 write/reuse 금지
- [ ] primitive-specific overflow/retry/promotion 및 error precedence 보존
- [ ] comparison tolerance/`!.` contract 보존
- [ ] 일반 float reduction과 compensated `+/!.0` 같은 derived numeric policy를 구분한다.
- [ ] NaN/Inf/signed zero/empty/fill-cell 테스트

<a id="g4--rustj-native-최소-physical-plan과-cpu-executor"></a>
### G4 — RustJ-native 최소 Physical Plan과 CPU Executor

- [ ] plan-time `PlanBufferId` / `PhysicalViewId` identity와 verifier
- [ ] `BindInput`
- [ ] `Check` — A3 SemanticCheck의 error kind/origin/order 보존
- [ ] `View`
- [ ] `Materialize`
- [ ] `Kernel` call — selected lowering recipe만 실행
- [ ] `Return` / output ownership
- [ ] last physical use
- [ ] buffer reuse proof/witness
- [ ] layout-compatible view 유지
- [ ] runtime `BufferLease/BufferId` binding과 plan-time identity 분리
- [ ] CPU executor
- [ ] source → J Semantic Array IR → Semantic Analyzer/Lowering → Logical Array IR/Plan → RustJ-native Schedule/Physical Plan → CPU end-to-end

<a id="g5--rustj-native-성능-및-physical-확장-경계"></a>
### G5 — RustJ-native 성능 및 physical 확장 경계

- [ ] structural view 생성 비용
- [ ] copy/allocation/peak/retained bytes
- [ ] general-stride indexing 비용
- [ ] materialization 비용 비교
- [ ] contiguous 기존 성능 회귀 확인
- [ ] Windows default/portable 전체 회귀
- [ ] 지원 layout/type/operation 표 갱신
- [ ] tiled/placement/transfer/completion 확장 경계 확인


<a id="out-of-core-io-checklist"></a>

<a id="io--느린-ioout-of-core-실행-이행-계획수용-체크리스트-2026-10-06"></a>
### IO — 느린 I/O·Out-of-core 실행 이행 계획·수용 체크리스트 (2026-10-06)

**I/O 프레임워크 처리·실행 구조 상세:** [§8.5.2](./09-memory-alias.ko.md#io-framework-execution-comparison)에서 기존 30개 게이트의 계층별 근거를 설명한다. 완료 판정은 이 표만 사용한다.

**상태: 설계·작업표 작성, 구현/실행 검증 0/30 수용.**

**체크리스트 운영 규칙(작업 시마다 적용).** 이 IO-01~IO-30 표가 I/O 구현 및 검증 상태의 단일 원장이다. 기존 표의 ID를 바꾸거나 동일 작업을 별도 계획 파일·새 checklist로 복제하지 않는다. 진행 시 (1) 선행 조건이 충족된 **가장 작은 미완료 실행 단위**를 선정, (2) Jsource/J 애드온/외부 프레임워크의 원본 및 라이선스·적용 가능성을 확정, (3) baseline 의미·negative fixture, (4) 최적화 구현, (5) J C oracle / Rust sync / optimized의 독립 비교 및 메모리·I/O 계측, (6) 해당 행의 증거·미지원·차단 조건을 갱신한다. 소스 검토·계획 완료는 구현 완료가 아니다. 실패·미측정·검증 환경 부재 시 **[ ] 유지**하고 사유를 적는다. 관련 기존 FW/DB/G4/G5 수용 표와 의미·효과/자원 선행 관계를 유지한다.

**현재 다음 작업:** IO-01·IO-25 upstream source pinned 부분 감사와 IO-02 효과/오류 반례 목록은 [§8.5.4](./09-memory-alias.ko.md#io-a-source-audit)에서 기록했다. 원본 C의 15개 file-foreign + 6개 ordered-effect 사례를 기준으로 고정했다. 다음에는 파일 핸들·권한·flush/close와 JMF bootstrap·RO/RW/COW 매핑 oracle을 소규모 실행 fixture로 확장한다. **소스 조사만으로는 IO-* 항목을 수용하지 않으며 0/30 [ ] 유지.**
 기존 M2/frontend와 FW-01~04·Rank/CellApply 의미 수렴을 우선한다. IO-A의 조사/모형화는 병행 가능하지만 I/O 경로를 M4 첫 CPU vertical slice의 필수 조건으로 격상하지 않는다. 단계 순서: **IO-A 근거와 안전 계약 → IO-B 동기 reference → IO-C 접근 최소화 → IO-D bounded async → IO-E 재사용/배치 최적화 → IO-F 실증/확장**. 앞 단계 미통과 시 뒤 단계는 설계 후보만 허용한다. [ ]은 *수용 전*, [x]는 변경 commit·실행한 명령/환경·jsource oracle 적용 범위·결과/남은 제한을 같은 행에 기록하고 독립적인 semantic/negative test가 통과했을 때만 사용한다. **아래 작업의 구현·테스트는 아직 수행하지 않았다.**

| ID / 단계·시점 | 완료 체크 · 실행 단위 | 선행 조건 · 최소 수용/negative 검증 |
|---|---|---|
| IO-01 / A·M2 병행 | [ ] jsource/J 라이브러리 근거 pin (원본 코드 일부 검토, 실행 oracle 대기; §8.5.4) | `jmf.ijs` mapping modes, `xf.c` partial I/O, alias/in-place, boxed mapping 제약, Jd column/ptable/jmfx, Jfiles/keyfiles, JMF boxed 경로별 source pin·행동/미지원 표. C foreign oracle와 파일 fixture; 추측/확인 분리 |
| IO-02 / A·M2~M3 | [ ] J 파일·mapping의 의미/효과 계약 (C oracle 15개 독립+6개 순서 사례 일부 실행; 핸들·권한·JMF·Jd 미검증, 수용 전; §8.5.4) | read/write/resize/flush/close, 오류·effect order, alias/late file changes, read-only/COW, J boxed/sparse, zero-frame case 목록. reorder/observable read omission 금지 negative test |
| IO-03 / A·M3 | [ ] 기존 IR·공통 data-movement 계약과 identity verifier | J Graph의 source topology vs Verified Logical의 `Effect/Dependency`·`AccessRelation` vs Physical Plan의 buffer/file region·transfer·ready token을 분리. `ValueId≠BufferId≠StateResource≠StorageObject/Version`, foreign effect vs immutable backing read; 새 Data Movement IR 도입 금지. 잘못된 계층의 필드·unknown effect를 거부하는 verifier/negative tests. §8.5.3 연결; `with`의 입력 semantic contract와 source capability·binding witness를 구별하고 mismatch/unknown provenance가 최적화에 사용되지 않도록 검사; [§8.5.5](./09-memory-alias.ko.md#io-static-source-contracts) 참조 |
| IO-04 / A·M3 | [ ] storage capability matrix | local file, mapped, chunk, remote, GPU는 개별 capability; offset/alignment/EOF/seek/atomic write/consistency. Unknown은 route barrier; SequentialOnly/SinglePass vs RegionReadable/Repeatable, mutable vs snapshot/versioned, Hard/Soft 구분, 잘못된 schema/capability 선언·binding 거부를 테스트; [§8.5.5](./09-memory-alias.ko.md#io-static-source-contracts) 참조 |
| IO-05 / B·M4 이후 | [ ] `read_at`/`write_at` 동기 독립 기준 | 일반 파일 offset/length, short read/EOF/overflow/error/permission 포함; 구현 전에 J file foreign과 physical array input 구분. 실제 fixture·C oracle 대조 |
| IO-06 / B·M4 이후 | [ ] versioned dense chunk reader | shape/element type/endianness/alignment/checked address/last short chunk. 파일보다 작은 메모리 예산으로 1회 순차 scan; baseline과 결과/오류 일치 |
| IO-07 / B·M4 이후 | [ ] mapped dense array 최소 경로 | read-only/mutable/COW·header/shape·flush/unmap/reference lifetime; mapping과 read_at 결과 비교. non-jmf typed boxed/JMF boxed 경로와 sparse capability를 구분해 검증 |
| IO-08 / B·M4 이후 | [ ] bounded resident/retained memory 기준 | chunk reader·buffer lease·release; RAM보다 큰 데이터와 작은 예산 조건에서 peak bound, leak/early release/cancel 검사 |
| IO-09 / C·M4~M5 | [ ] logical access region→physical byte-range witness | select/slice/reindex의 원소 범위를 증명한 뒤에만 `BufferSlice`/`FileByteRange` 접근 최적화 후보 생성; dynamic rank·alias·외부 변경·관찰 가능한 file/error effect·empty prototype은 opaque/barrier. byte omission과 J semantics 독립 검증. §8.5.3 연결; 역방향 `None/Regions/Full/Unknown` demand·다중 소비자 합집합, slice/transpose/reduction/halo/gather의 witness 및 full/unknown fallback을 검증; [§8.5.5](./09-memory-alias.ko.md#io-static-source-contracts) 참조 |
| IO-10 / C·M5 | [ ] scan projection/slice pushdown | source+consumer legality proof/guard, I/O 바이트 수와 결과/오류 측정; Reduce/Rank/sparse/boxed 반례 포함 |
| IO-11 / C·M5 | [ ] shared scan/subplan reuse | 동일 external object+version/policy/byte range일 때만 cache; changed file/late binding/alias가 cache invalidation 유발 |
| IO-12 / C·M5 | [ ] immutable snapshot/version contract | file content/version/staleness 검출·error precedence; speculative read는 no-visible-effect 조건에서만 허용 |
| IO-13 / D·M5 | [ ] portable async facade + readiness 계약 | 동기 reference 위 blocking-I/O workers와 `Pending/Ready/Failed/Cancelled` 및 완료 토큰. 파일/메모리 transfer는 공통 스케줄러에서 의존성을 분석하되 오류/취소 의미는 별개; portable sync fallback·negative cases. §8.5.3 연결 |
| IO-14 / D·M5 | [ ] bounded prefetch / double buffer | `Read(n+1)`↔`Compute(n)` overlap, stage dependency/tokens·buffer lease 테스트; serial·parallel 결과 대조 |
| IO-15 / D·M5 | [ ] governor/backpressure | inflight bytes+queued jobs+resident+kernel temporary memory 공동 예산; slow consumer/high-latency I/O/oversubscription/OOM negative test |
| IO-16 / D·M5 | [ ] streaming barriers·cleanup | chunked elementwise와 reduction, rank/cell/zero-frame, producer failure/cancel/error precedence; non-streamable operator는 barrier/materialize |
| IO-17 / D·M5 | [ ] async completion·buffer lifetime·J effect proof | short/failed read, dirty mapping, cancel/retry, early release, file observable-error order 및 `1!:` 미사용 read 제거 금지; speculative internal immutable read도 오류 노출 위치 증명 요구. 전송 중 buffer 재사용/중복 side effect 금지. §8.5.3 연결 |
| IO-18 / E·M5 | [ ] mutable weight/checkpoint 정합성 | read snapshot vs write version; partial write, atomic publication/durability/recovery test. 학습 상태 복제·일관성 계약 검증 전 offload 금지 |
| IO-19 / E·M5 | [ ] weight/scan reuse physical schedule | FlexGen식 layer-block/batch ordering 후보; read bytes↓/cache hit↑, dependency·effect·order unchanged, latency–throughput 별도 평가 |
| IO-20 / E·M5 | [ ] 공통 physical data-movement·placement cost planner | 기존 Physical Plan에서 memory copy/CPU↔GPU 후보/file byte-range read/write/transfer를 위치·region·lifetime·ready dependency로 함께 계획. bytes/requests/seek/latency/peak/inflight/compute/overlap 분리; unknown capacity는 hard gate, heuristic cost는 미측정 표시. GPU 실행은 보류. 새 IR 필요성은 2개 실제 실패 사례가 있을 때만 재검토. §8.5.3 연결 |
| IO-21 / F·M5 | [ ] 파일 크기·access shape 측정 suite | cold/warm, sequential/random, small/large, dense/strided, disk-backed NN weights, sparse/boxed guarded, 0-sized; bytes/wait/peak/time/latency |
| IO-22 / F·M5 | [ ] 3방향 semantic·negative·movement 검증 | C reference(실제 J foreign 효과 있는 영역) / Rust sync / Rust optimized 독립 비교; skipped read vs error precedence, stale version, empty Rank virtual cell, boxed/sparse, alias/early-free/cancel, 중간 copy/전송 bytes 검증. pass/fail/ignored·unsupported 분리. §8.5.3 연결; 정적 schema mismatch, snapshot invalidation, capability 오선언, compressed read amplification, reduction streaming 반례 포함; [§8.5.5](./09-memory-alias.ko.md#io-static-source-contracts) 참조 |
| IO-23 / F·M5 이후 | [ ] mmap vs read_at vs async 실측 선택 | OS page cache/hard faults, IO bound/compute bound, storage/media/target 차이. 유의미한 실측 이득과 regression 없을 때만 기본 경로 |
| IO-24 / F·M6 이후 | [ ] 확장 후보 승인 게이트 | io_uring/direct I/O, remote object storage, compression, NVMe↔GPU/pinned DMA, multi-device, Jd adapter. portable implementation·검증 예산 확인 시 별도 소규모 작업으로 승격 |


**추가 게이트 IO-25~IO-30 (이행 단계 기준으로 실행; 번호는 추가 발견 순서).**

| ID / 단계·시점 | 완료 체크 · 실행 단위 | 선행 조건 / 수용 기준 |
|---|---|---|
| IO-25 / A·M2 병행 | [ ] Jd/jfiles/JMF boxed 원본 교차 감사 (소스 확인·JMF RW/RO/COW smoke 구축, 원본 mapping·boxed/Jd 실행 수용 검증 대기; §8.5.4) | jsource pin과 data_jd pin, J binary oracle, ptable pruning·keyfiles·JMF boxed 분기 검증. IO-01·02 연계 |
| IO-26 / B·M4 이후 | [ ] Typed array storage metadata 검증 | dtype/shape/order/endian/offset/length/version; invalid overlap/duplicate/off-end/overflow, empty/scalar, sparse/boxed 경계. IO-05·06 연계 |
| IO-27 / B·M4 이후 | [ ] Read chunk와 write shard 분리 | Zarr/HDF5를 참고해 access axis별 read amplification, coalescing, shard write cost 및 contiguous baseline 비교. IO-06·08 연계; Read/Write/CodecChunk, IoRequestRange, ComputeTile을 분리하고 서로 다른 분할 경계의 증폭 및 버퍼 비용을 검증; [§8.5.5](./09-memory-alias.ko.md#io-static-source-contracts) 참조 |
| IO-28 / B·M4 이후 | [ ] mmap·SIMD tail·lifetime 안전성 | EOF next-page, vector overfetch 금지, real page granularity, live lease/remap/unmap/readonly/COW/concurrent readers. IO-07·08 연계 |
| IO-29 / C·M5 | [ ] Bounded decoded-chunk cache | identity/version/encoding/range cache key, budget+lease-safe eviction, mutation invalidation, strided reuse/cache thrash. IO-09~12 연계 |
| IO-30 / F·M5 이후 | [ ] Workload별 load mode 선택 실측 | mmap vs buffered read vs async, cache warm/cold, NVMe/remote, NN weights, small/large working set; bytes/page faults/RSS/latency/throughput. IO-21·23 연계 |

**Acceptance log 양식:** `IO-ID | code commit | jsource/library pin | command + OS/target + storage | J C oracle / Rust synchronous / optimized counts | measured cold/warm bytes/time/peak | failures/unsupported | CI 여부 | next gate`. 단순 기법 소개나 파일 존재만으로 [x] 금지. 성능은 의미·자원 안전 통과 후 평가하고, 미지원은 조용한 fallback 성공으로 계산하지 않는다. [`8.5 소유권과 실행 계약](./09-memory-alias.ko.md#out-of-core-io-contract) 참조.

<a id="c--frontend--언어-의미-확장"></a>
### C — frontend / 언어 의미 확장

Semantic IR/Logical IR 경계의 정확성을 막는 frontend 결함은 즉시 수정한다. 일반적인 언어 기능 확장은 A1~A3의 core IR 경계를 먼저 안정화한 뒤 진행하며, G2~G5와는 필요 의존성에 따라 병행한다.

- [ ] primitive registry/binding contract 보완
- [ ] direct/explicit definition
- [ ] verb binding/train
- [ ] adverb/conjunction 확대
- [ ] dyadic rank/rank list
- [ ] scan
- [ ] boxed/sparse 전체 의미
- [ ] scalar type 확대
- [ ] system/runtime API

<a id="formal-ir-verification-checklist"></a>

<a id="fv--ir-정확성-형식-검증-장기-후속-체크리스트-2026-10-08-계획만-기록"></a>
### FV — IR 정확성 형식 검증: 장기 후속 체크리스트 (2026-10-08; 계획만 기록)

**상태와 우선순위:** IR 정확성을 수학적으로 검증할 수 있는 체계는 장기적으로 필요하다. **도구 도입·형식 모델 작성·정리 증명·SMT 실행·Rust 검증·CI 연동은 전부 미착수**이며 지금 진행하지 않는다. M2 frontend 의미 수렴 → M3 경계 → M4 첫 native CPU 실행 우선순위를 바꾸거나 초기 CPU slice의 선행 조건으로 추가하지 않는다. 기존 verifier/negative tests, jsource differential/golden 검증은 계속 별개로 필요하다.

**검증 목표:** (1) IR well-formedness와 SSA·참조·scope 불변식, (2) J source/frontend → FunctionEntity/J Graph → A3 Logical Execution IR의 의미 보존, (3) rewrite/fusion/route/lowering 및 CPU/GPU realization의 관찰 가능한 동치성. 관찰 범위에는 값뿐 아니라 dtype/shape/rank, boxed/sparse/empty frame의 fill-cell/prototype/assembly, name lookup 시점·binding version, 효과와 오류 종류·우선순위·순서, 숫자/overflow/fit 정책을 포함한다. 명세에서의 증명과 실제 Rust 코드의 정확성은 다른 주장이다.

**도구 후보(채택 결정 아님):** Lean 4 / Rocq(CoQ) / Isabelle/HOL은 일반 의미론·변환 정리, Z3 / cvc5는 제한된 최적화·Shape 제약의 반례 탐색과 동치성, Verus / Creusot / Kani는 지원 범위 내 Rust 구현 검증, Alive2는 LLVM IR까지 정확하게 lowering된 경로의 LLVM-level 최적화 검증에 검토한다. Alive2로 J Graph/A3 자체를 직접 증명했다고 주장하지 않는다.

- [ ] **FV-01 — 착수·범위 결정:** M2/M3/M4 진행과 별개로 비용 대비 이득, 고위험 변환, 전제와 검증 대상 및 필요한 인력/유지보수 비용을 재평가한다. 첫 proof pilot 하나를 명시적으로 승인하기 전에는 형식 검증을 일정의 필수 gate로 승격하지 않는다.
- [ ] **FV-02 — 관찰 가능 의미론 명세:** 참조 jsource revision과 RustJ의 지원/미지원 의미를 구분하고 Source/Frontend·J Graph·A3의 결과/오류/효과·상태 관찰을 정의한다. 모델 범위 밖의 J 기능을 '증명됨'에 포함하지 않는다.
- [ ] **FV-03 — IR 구조 불변식:** ValueId·region·block·SSA 정의/사용·dominance·provenance·registry/schema·effect/Check/Write·name/binding-version·buffer identity의 각 단계 검증 전제를 명시한다.
- [ ] **FV-04 — J 배열 특수 의미:** Rank prefix agreement, CellApply, zero-frame 가상 fill-cell과 empty-cell의 구별, prototype/shape/type/heterogeneous assembly, boxed/sparse·tolerance·오류 우선순위를 형식화하고 반례를 준비한다.
- [ ] **FV-05 — 단계 간 의미 보존:** 대표 J Graph → A3 lowering 정리와 동적 guard/Unsupported 경계를 정의한다. Frontend → J Graph와 Route/Physical lowering의 증명 범위를 각각 구분한다.
- [ ] **FV-06 — rewrite 합법성:** DCE/CSE/fusion/scan, 값 재사용/재계산, zero-trip elimination, name lookup과 effect/error 재정렬 금지에 대해 변환별 전제·사후조건·반례를 정의한다. Unknown을 동치성 증거로 사용하지 않는다.
- [ ] **FV-07 — SMT 소규모 실험:** Z3 또는 cvc5를 한정된 Shape/정수·SSA rewrite에 적용해 counterexample·SAT/UNSAT/UNKNOWN을 구별한다. bounded 검사 결과를 전 입력에 대한 증명으로 보고하지 않는다.
- [ ] **FV-08 — Rust 구현 연결:** Verus/Creusot/Kani 중 실제 RustJ 코드·지원 기능·proof burden에 맞는 방법을 실험하고, 추상 IR 모델과 production Rust 구현 사이 정제(refinement)/계약 검증 공백을 기록한다.
- [ ] **FV-09 — backend 한정 검증:** LLVM 경로에만 Alive2 적용 가능 범위를 시험하고, GPU 병렬 효과·오류·race/lifetime 및 부동소수점 재결합/오차 정책은 별도 모델과 실행 검사로 다룬다. GPU 도입 보류는 유지한다.
- [ ] **FV-10 — 도구와 신뢰 경계 결정:** Lean/Rocq/Isabelle·SMT·Rust verifier 가운데 최소 조합을 정하고 모델 인코딩, 솔버 가정, timeout/unknown, proof checker, 코드-명세 버전 결합, 지원 범위를 문서화한다.
- [ ] **FV-11 — 회귀·CI 수용:** 선택한 proof/SMT witness 및 실패 반례를 해당 IR schema/rule/compiler revision과 묶는다. negative tests와 pinned C differential을 유지하고, proof pass와 실제 runtime/differential pass를 따로 기록한다. CI 자동화는 별도 승인 후 진행한다.
- [ ] **FV-12 — 수용·유지보수 판정:** 대표 변환의 실제 증명 재실행 가능성, 코드 변경 시 증명 유지 비용, 미증명 단계와 trusted assumptions, failure/UNKNOWN 처리 방식을 확인한 후에만 '검증 완료' 표시를 허용한다.

**재개 기준:** 사용자의 명시적 재개 요청 또는 핵심 IR 의미/변환이 충분히 안정화된 뒤 고위험 최적화를 실제로 승인할 시점에 FV-01부터 검토한다. **2026-10-08 기록은 필요성과 체크리스트의 문서화만 완료한 것이며, 위 FV-01~12 항목은 모두 미완료다.**

---

<a id="cuda--보류"></a>
### CUDA — 보류

- [ ] CUDA storage
- [ ] transfer/completion
- [ ] stream/event
- [ ] kernel codegen
- [ ] GPU resident graph
- [ ] sharding/multi-device

재개 조건: 사용자 요청과 검증 가능한 GPU 환경 확보.

---
