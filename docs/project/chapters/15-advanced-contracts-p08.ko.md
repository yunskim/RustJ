[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)

> 문서 분할 안내: 기존 `PROJECT.ko.md`의 내용을 이동한 모듈입니다. 원문의 단일 파일 정본·신규 Markdown 금지 문구는 최신 [Agent 문서 규약](../../../AGENTS.md)보다 우선하지 않습니다.

# 심화 기술 계약 — primitive, Rank, 하드웨어 계획, Flow–Storage — 계속 (8/10)

### 4.22 외부 compiler 조사에서 얻은 직접적인 설계 근거

- **MLIR DataLayout / TargetSystemSpec**: type layout과 heterogeneous device properties를 별도 target specification으로 둔다.
- **LLVM TargetTransformInfo**: vector register width와 instruction/reduction cost 같은 target-specific 정보를 IR 의미와 분리된 query interface로 제공한다.
- **OpenXLA GPU**: logical shape와 physical layout을 분리하고, layout conflict는 copy로 materialize하며, fusion 뒤 tile/warp/stage 같은 구체 schedule을 backend config로 둔다.
- **TVM TensorIR**: logical loop/block 위에 bind, vectorize, cache/storage scope, tensorize 같은 schedule을 별도로 적용한다.
- **Triton**: block shape, warps, pipeline stages, register cap을 kernel configuration으로 다루고 shape/stride/alignment를 memory access의 전제로 사용한다.
- **CUDA**: warp width 외에도 register file, per-thread/block register limit, shared-memory capacity, resident block/warp/thread limits가 함께 launch/occupancy를 결정한다.
- **ROCm/HIP**: VGPR/SGPR, LDS, wave slots, block size가 occupancy를 공동 제한하며 architecture에 따라 wavefront와 register organization이 달라진다.

따라서 RustJ hardware-aware IR의 중심은 특정 vendor 필드 복제가 아니라 **logical access/dependency structure + generic target capabilities + explicit physical schedule + derived resource model**이다.

### 4.23 old analyzer output schema는 Logical/Physical 계층으로 분해해 흡수한다

`japchae`의 analyzer output schema v0.1은 중요한 prototype이지만 logical 정보와 target-dependent resource 정보가 한 객체에 섞여 있었다.

**Logical Array IR / Plan**에는 op identity, logical shape, rank/cell/frame, dtype/numeric constraints, iteration domain, axis semantics, access relations, effect/alias/speculation, control-flow, constraint witness, semantic storage/lifetime requirement, target-independent rewrite/fusion constraints, provenance를 둔다.

**Route Partition 입력**은 Logical Array IR과 adapter/backend capabilities다. RustJ-native route를 선택한 경우에만 Physical Planner가 Logical Array IR + TargetProfile + optional CostProfile + backend/library capabilities를 입력으로 받는다.

**RustJ-native Physical Plan / Planning Report**에는 fusion/kernel region, materialization boundary, target placement, axis/thread/vector mapping, chosen layout, tiling, memory-space staging, pipeline stages, synchronization, accumulator realization, placement/transfer, buffer lifetime/reuse, backend strategy, derived ResourceEstimate와 optional CostEstimate를 둔다.

compile artifact에는 source revision, PrimitiveSpec registry version, input facts/spec, TargetProfile version, optional CostProfile version, compiler version을 provenance로 기록한다.

### 4.24 Flow–Storage: semantic storage requirement와 physical materialization을 분리한다

과거 `japchae`와 `jaxa-analyzer`에서 발전한 Flow–Storage 아이디어는 현재 RustJ의 `ValueId` / `BufferId` 분리와 결합한다.

> **Logical ArrayValue가 존재한다는 사실은 별도의 memory buffer가 존재한다는 뜻이 아니다.**

Logical IR에는 “반드시 독립적으로 관찰·보존되어야 하는가”라는 **semantic storage requirement**만 둔다.

```text
StorageRequirement
  EphemeralAllowed
  MustSurvive(region_or_effect_boundary)
  Persistent(resource_id)
  ExternalVisible
  ExplicitCheckpoint
```

반면 다음은 physical/backend 결정이다.

```text
MaterializationDecision
  KeepVirtual
  FuseAway
  RegisterResident
  WorkingStateResident
  Bufferize(memory_space)
  Rematerialize(recompute_source_or_region)
  ExternalResource
```

따라서 Logical IR에서 `Materialize`를 일반적인 실행 op처럼 남발하지 않는다. explicit checkpoint처럼 **J/RustJ semantics 자체가 저장을 요구하는 경우**에만 semantic storage op/requirement가 존재한다.

`Rematerialize`는 semantic value를 삭제하는 것이 아니라 **동일 value를 필요 시 다시 계산하는 physical/schedule 선택**이다. 따라서 checkpoint requirement와 rematerialization decision을 분리한다. explicit checkpoint가 있으면 저장 의무가 생길 수 있지만, ordinary ephemeral value는 planner가 `Bufferize`와 `Rematerialize` 사이를 resource/cost 모델로 선택할 수 있다. 이 선택의 legality는 purity/effect/error-order/name-binding/state dependency를 증명해야 하며 단순히 계산량이 작다는 이유로 재계산하지 않는다.

persistent state는 primitive 내부 hidden state로 숨기지 않는다. scalar state도 J 의미상 rank-0 array로 취급한다.

Logical 단계에서는 `Read`, `Write`, `Accumulate`, `Load`, `Alias/View` 및 `StorageRequirement`를 표현하고, 실제 register/shared/device/host buffer, allocation, offset, reuse, copy는 bufferization/physical planning 또는 외부 compiler가 정한다.

이 원칙은 MLIR이 tensor-level optimization 뒤에 bufferization을 늦추는 구조와 IREE Stream이 tensor computation 뒤에 resource lifetime/allocation을 명시화하는 구조를 따른다.

#### 4.24.1 destination / alias relation은 buffer identity와 분리한다

late bufferization을 잘 지원하려면 op가 결과와 operand 사이의 **semantic alias/destination 가능성**을 설명할 수 있어야 한다.

```text
DestinationRelation
  FreshResult
  MayReuse(operand)
  EquivalentView(operand, mapping)
  OverlappingView(operand, relation)
  MustAlias(resource)
  Unknown
```

이 relation은 `BufferId`를 미리 배정하는 것이 아니다. native bufferization이나 MLIR One-Shot Bufferize 같은 후속 단계가 SSA use-def, liveness, conflict를 함께 보고 실제 in-place/out-of-place 결정을 내릴 수 있게 하는 contract다.

#### 4.24.2 mutable state는 primitive 내부에 숨기지 않는다

후기 `japchae` D-27/D-31의 결정은 현행 RustJ에서도 유지한다.

weight, gradient, optimizer state, checkpoint, routing map처럼 실행 사이에 관찰·재사용·갱신되는 array state는 특별한 “layer-owned field”가 아니라 **명시적인 array/resource identity**로 표현한다.

```text
StateResource
  logical array type/shape
  lifetime requirement
  access/effect permissions
  initialization / persistence semantics
  optional external visibility
```

역할 이름은 semantic kind를 결정하지 않는다.

```text
weight
gradient
optimizer m/v
step counter (rank-0 array)
checkpoint
routing map
```

모두 J 관점에서는 array/value/resource이며, 차이는 lifetime과 Read/Write/Accumulate effect 및 storage requirement다.

parameterized adverb가 stateful computation을 도출하더라도 mutable state를 derived verb 내부의 숨은 object field로 캡슐화하지 않는다.

```text
DerivedVerb
  references StateResource ids
  + pure/declared computation semantics

StateResource
  exists outside the verb
```

compile-time 고정 hyperparameter처럼 실행 중 변하지 않는 값은 immutable parameter/attribute로 derived verb에 캡처할 수 있다. **mutable runtime state와 compile-time constant를 구분**한다.

과거 “materialized array”라는 용어는 다음 두 개념을 한 단어로 묶었다.

```text
semantic side:
  StateResource / StorageRequirement / lifetime/effects

physical side:
  MaterializationDecision / BufferId / offset / memory space
```

현행 RustJ에서는 이를 분리한다. named state의 identity/lifetime이 compile time에 알려져도 physical fixed offset을 반드시 미리 정할 필요는 없다.

과거 Flow–Storage 연구의 “in-band memory vocabulary” 아이디어는 이 모델 위에 선택적으로 올릴 수 있다. `load/store/emit/cp` 같은 표기가 채택되더라도 그것은 physical buffer 명령이 아니라 **semantic resource/effect declaration**이어야 한다. external/native lowering이 이를 실제 load/store/copy/checkpoint로 어떻게 실현할지는 별개다.



#### 4.24.3 APEX/Co-dfns/TAIL 연구를 반영한 middle-end substrate

APEX, Aaron Hsu의 Co-dfns 연구, APL→TAIL→Futhark 연구를 비교한 결과, 현재 RustJ의 `Semantic IR → ResolvedCallFacts → Logical IR → Optimizer → Route/Schedule` 방향은 유지하되 **analysis substrate를 명시적으로 추가**한다.

근거:
- APEX source/research: https://gitlab.com/bernecky/apex , https://www.snakeisland.com/ms.pdf
- Aaron W. Hsu, *The Key to a Data Parallel Compiler*, DOI 10.1145/2935323.2935331: https://dl.acm.org/doi/10.1145/2935323.2935331
- Co-dfns source pin `4e6d3e3002f2109360d24278776c5b5a4f65db0d`: https://github.com/Co-dfns/Co-dfns
- Dyalog'16: https://elsman.com/pdf/Dyalog16.pdf
- Henriksen et al., FHPC'16: https://elsman.com/pdf/fhpc16futhark.pdf
- 자세한 Source → Observation → RustJ Decision 매핑은 `FOUNDATIONS.ko.md` Part XX를 따른다.

권장 middle-end:

```text
J Semantic IR
      ↓
Binding / Effect Analysis
      ├─→ StructuralOpportunity extraction
      ↓
Logical SSA
      ↓
GraphIndex / AnalysisIndex
      ↓
MorphologyEngine
      ↓
Interprocedural Fixed Point
      ↓
SpecializationEngine
      ↓
High-level Logical Parallel IR
      ↓
Optimization Proofs
      ↓
RoutePartition
      ↓
Schedule / Physical Plan
```

이 구조는 APEX/Co-dfns/TAIL을 복제하는 것이 아니다. 각 연구에서 검증된 array-compiler insight를 **full J semantics + jsource-compatible frontend**라는 RustJ 경계에 맞게 재배치한 것이다.

#### 4.24.4 MorphologyEngine: ValueFacts를 lattice/fixpoint 분석으로 승격한다

APEX의 array morphology에서 가져오는 핵심은 type inference가 아니라 **array property의 정식 data-flow analysis**다.

```text
ValueFacts
  TypeFact
  RankFact
  ShapeFact
  ItemCountFact
  ConstantFact
  ArrayPropertyFacts
  ConstraintSet
```

각 fact domain은 최소한 Unknown/Proven 계층, join/merge, transfer/refinement를 가져야 하며 interprocedural summary와 specialization propagation을 worklist/fixpoint로 반복할 수 있어야 한다.

`FunctionEntity`에는 actual argument-dependent morphology를 넣지 않는다. call/value/Logical node fact table이 소유한다.

#### 4.24.5 ArrayPropertyFacts와 FactWitness

APEX의 array predicates와 fact-origin 문제를 반영해 다음을 정식 분석 domain으로 둔다.

```text
ArrayPropertyFacts
  IntegralValued
  NonNegative
  Unique
  SortedAscending / SortedDescending
  Permutation
  KnownRange
  AllEqual
  ...
```

그리고 optimization에 사용되는 fact는 가능한 한 provenance를 갖는다.

```text
Fact<T>
  abstract_value
  witness

FactWitness
  Constant
  SameAs(ValueId, relation)
  DerivedFrom(NodeId, RuleId)
  BindingVersion(...)
  RuntimeGuard(GuardId)
  CellApplyFrame(...)
  ...
```

check elimination, fusion legality, route eligibility, specialization은 witness 없는 optimistic fact에 의존하지 않는다.

#### 4.24.6 GraphIndex는 semantic DAG의 replacement가 아니라 derived analysis view다

Hsu의 Node Coordinate Matrix와 현재 Co-dfns의 inverted-table AST는 parent/depth/type/kind/binding/source-range를 columnar form으로 보관해 subtree/group analysis를 batch operation으로 처리하는 장점을 보여 준다.

RustJ canonical semantic representation은 계속 immutable shared DAG다. 다만 다음 sidecar를 만들 수 있다.

```text
GraphIndex
  NodeId[]
  parent[]
  depth[]
  preorder[]
  subtree_end[]
  opcode[]
  entity_id[]
  scope_id[]
  def/use index
  source-origin index
```

용도:
- op/entity/scope/specialization key별 grouping
- use-def/liveness
- bottom-up/top-down summary
- dependency-level parallel pass
- CPU SIMD/멀티코어 batch analysis

**금지:** semantic identity를 dense matrix/SoA 하나에 종속시키거나 모든 pass를 data-parallel form으로 강제하지 않는다.

#### 4.24.7 SpecializationEngine: call-site clone 대신 cacheable SpecializationKey

APEX의 call-site specialization과 TAIL의 explicit type/rank instantiation을 일반화한다.

```text
SpecializationKey
  FunctionEntityId
  binding version / guard
  valence
  relevant dtype classes
  effective rank facts
  relevant shape class
  relevant ArrayPropertyFacts
  fit/tolerance/rank policy
```

동일 key는 analyzed region/JIT artifact를 재사용한다.

specialization explosion을 막기 위해 exact shape/constant/property는 **algorithm/lowering 선택에 실제로 영향을 줄 때만** key에 넣는다. 필요하면 abstract-state widening/merge를 허용한다.

#### 4.24.8 High-level parallel structure를 Logical IR에 보존한다

TAIL→Futhark 연구는 map/reduction nests를 explicit하게 유지해야 fusion, nested-parallelism flattening, coalesced access optimization을 수행할 수 있음을 보여 준다.

따라서 다음은 early scalarization 금지 대상이다.

```text
CellApply
MapCells
Reduce
Scan
Window
Gather / Scatter
StaticReindex
OuterProduct
MatMul
Conv
Loop / Power
```

nested `CellApply(Reduce(...))`, `Reduce(CellApply(...))` 등은 Logical IR에서 보존한다. flattening/segmentation/thread mapping은 optimizer/schedule decision이다.

이 절의 `MatMul`, `Conv`, `OuterProduct` 같은 named high-level op 보존 원칙은 **Graph Basis identity 보존** 문제다. 4.24.11의 Execution basis vocabulary와 경쟁 관계가 아니다. named op는 Graph IR에서 algorithm/access/resource identity를 black box로 유지할 수 있고, execution lowering은 필요하면 `ExecutionBasisExpansion`을 통해 `WindowView + Contract` 같은 더 compositional한 실행 graph를 제공할 수 있다. 더 작은 expansion은 실제 성능상 이득이 있을 때만 추가한다.

#### 4.24.9 Pure region extraction과 route precondition

TAIL/Futhark와 Co-dfns는 실제 compiler subset을 제한한다. RustJ는 그 제한을 J language restriction으로 채택하지 않는다.

```text
Full J semantic graph
      ↓
EffectAnalysis
      ↓
RegionPartition
  PureArrayRegion
  GuardedDynamicRegion
  StatefulRegion
  RuntimeSemanticRegion
```

external backend의 static scope/static rank/no-execute/pure-array requirement는 **route precondition**이다.

```text
route cannot lower X
  ≠ X is invalid J
```

route가 실패하면 native route, guarded JIT, runtime semantic path를 선택한다.

#### 4.24.10 Parameterized Lowering Recipe

TAIL→Futhark가 static type/rank information에 따라 primitive code skeleton을 specialize하는 방식을 일반화한다.

```text
Resolved Semantic Op
+ ResolvedCallFacts
+ TargetCapability
        ↓
ParameterizedLoweringRecipe
        ↓
candidate realizations
```

예를 들어 `Take`는 sign/bounds/fill/view legality에 따라 ViewTake, DirectSlice, PadAndSlice, GenericTakeKernel, RuntimeSemanticFallback으로 분기할 수 있다.

semantic op 하나가 kernel 하나라는 가정을 두지 않는다.




#### 4.24.10a Graph Basis와 Execution Basis를 분리한다

`basis`는 하나의 계층을 뜻하지 않는다. RustJ에서는 최소한 두 개를 명시적으로 구분한다.

~~~text
J Semantic Function / Derived Verb
        ↓
J Graph IR
  GraphBasis / graph algebra / rewrite
        ↓
Execution Semantic Lowering
        ↓
Logical Execution IR
  ExecutionBasisKind / ExecutionBasisPayload
        ↓
target lowering / library / custom kernel / physical schedule
~~~

**Graph Basis**는 역사적 JAXA의 basis-verb 연구에 가까운 개념이다. 목적은 J 표기가 제공한 계산 구조를 대수적으로 다루고, rewrite/equivalence/resource/fusion 분석에 사용할 생성원과 구조 단위를 보존하는 것이다. 이 층의 granularity는 hardware instruction이나 실행 loop의 최소성으로 정하지 않는다.

특히 과거에 `conv`를 black box로 남기기로 한 판단은 **Graph IR에서 convolution identity를 성급하게 더 작은 graph primitive로 쪼개지 않는다**는 뜻이다. convolution이라는 algorithm/access/resource 구조를 Graph Basis에서 그대로 보존하면 rewrite, equivalence, fusion boundary, symbolic resource reasoning이 원래 identity를 이용할 수 있다.

이 결정은 Execution IR의 분해를 금지하지 않는다.

~~~text
GraphBasis::Structured(Conv)
   ├→ ExecutionBasis(WindowView → Contract)
   ├→ ExecutionBasis(StaticReindex → Elementwise → Reduce)
   ├→ library call
   └→ custom fused kernel
~~~

즉 **graph-level black box와 execution-level decomposition은 독립적인 결정**이다. lowering은 원 GraphBasis identity와 equivalence/provenance를 유지한 채 하나 이상의 ExecutionBasis op로 펼칠 수 있다.

Derived verb도 같은 순서를 따른다.

~~~text
Derived Verb
  → GraphBasis composition
  → ExecutionBasis composition
  → realization
~~~

예를 들어 ranked reduction은 Graph IR에서 outer-to-inner `CellApply → Reduce` 구조를 보존할 수 있다. 이를 하나의 effective execution primitive로 일찍 평탄화하지 않는다. 이후 Execution Semantic Lowering이 J의 fill/assembly/error semantics를 증명한 범위에서 별도의 `ExecutionBasisKind::CellApply`, `ExecutionBasisKind::Reduce` 또는 흡수된 realization으로 내린다.

코드에서는 두 층의 이름을 공유하지 않는다.

- J Graph IR: `GraphBasis`, `GraphBasisKind`
- Logical Execution IR: `ExecutionBasisKind`, `ExecutionBasisPayload`
- Execution lowering registry: `ExecutionBasisLoweringCapability`
- optional Execution-IR refinement: `ExecutionBasisExpansion`

같은 단어인 Map/Reduce/Reindex가 양쪽에 나타날 수 있지만 **타입, 소유 fact, rewrite 법칙, granularity 조건이 다르다.**

#### 4.24.11 Execution basis vocabulary v0.2: 최소성이 아니라 성능상 유효한 실행 최적화 경계

이 절의 **Execution basis operation**은 “더 이상 분해할 수 없는 최소 primitive”를 뜻하지 않는다. Graph Basis의 최소성/생성원 문제와도 별개다. **현재 수준의 semantic/array identity를 보존했을 때 optimizer와 planner가 실제 성능상 이득을 얻는 연산 단위**를 뜻한다.

따라서 어떤 basis op A가 더 작은 B + C + ... 로 표현 가능하다는 사실만으로 A를 제거하지 않는다. refinement는 다음 중 하나 이상의 이득이 입증되거나 강하게 예상될 때만 채택한다.

- producer/consumer fusion 범위 확대
- intermediate materialization 제거
- access/index-map 합성
- SIMD/SIMT/nested-parallel decomposition 개선
- tensor/GEMM/warp/shared-memory 등 target-specific lowering 개방
- memory traffic 또는 synchronization 감소
- 여러 semantic op 사이 optimizer pass 재사용
- cost model이 의미 있는 realization 선택지를 추가로 얻음

반대로 분해 후 planner가 원래의 구조를 다시 pattern-match해야 하거나 algorithm identity를 잃어 specialized implementation 선택이 어려워지면 큰 basis를 그대로 유지한다. **basis refinement는 version-up 가능한 optimization contract**이며 language semantics의 조건이 아니다.

잠정 v0.2 vocabulary:

~~~text
Iteration / application
  IndexSpace / Generate
  Elementwise / MapN
  CellApply

Access / views / update
  RegularReindex
  Gather
  Scatter / Amend
  ScatterCombine
  WindowView
  SegmentView
  Permute

Collective / algebra
  Reduce
  Scan
  Contract

Structural
  Concat / Assemble
  Replicate / Compact / Expand

Ordering / classification
  Grade
  Lookup / Classify
  GroupBy

Nested value structure
  NestedTraverse

Large structured kernels
  LinearSolve
  StateMachine          // provisional structured kernel
~~~

다음은 같은 목록에 섞지 않는다.

~~~text
Representation axis
  Dense / Sparse / future tiled or compressed layouts

Control / function semantics
  Power/Iterate, Agenda, Under, Atop, Bond, gerund,
  explicit definition, adverse/obverse, memoization, ...

Value construction / runtime
  Box/Open, assignment/binding, execute/format,
  foreign, random/state, symbol interning, ...
~~~

Box/Open은 nested computation 자체보다 value construction/representation에 가깝다. 반면 boxed tree를 일정 level에서 방문하는 NestedTraverse는 traversal/parallelization 선택을 보존할 가치가 있으므로 basis 쪽에 둔다.

LinearSolve, Grade, GroupBy는 더 작은 operation으로 구현할 수 있어도 지금 분해하지 않는다. solver/ordering/grouping identity 자체가 algorithm, library, sparse/dense, CPU/GPU realization 선택에 직접 유용하기 때문이다. StateMachine은 ;: 의 일반 sequential-machine semantics를 잃지 않기 위한 provisional structured kernel이며, 첫 구현은 generic sequential route여도 된다.


---

[← 이전 부분](./15-advanced-contracts-p07.ko.md) | [다음 부분 →](./15-advanced-contracts-p09.ko.md)
