[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)

<a id="executionbasisexpansion"></a>
##### ExecutionBasisExpansion

**Convolution 상세 분석 계약:** [§7.10 Structured Conv/Index Access/Halo/SPORES·egg 수용](./11-roadmap-checklists-p04.ko.md#conv-graph-ir-optimization), [§7.10.7 Conv2d 고수준 blackbox + Graph 단계 SPORES 투영·선택적 Logical 분해](./11-roadmap-checklists-p04.ko.md#conv2d-graph-spores-semantic-blackbox). `Conv2d`의 J primitive 분해나 J 참조식은 필수가 아니며, Graph relation projection을 실행 가능한 Logical lowering으로 취급하지 않는다. 아래 Conv ↔ WindowView + Contract는 실제 실행·동등성 증명 완료를 의미하지 않는다.

큰 semantic/structured op가 basis graph와 동등하다고 알려져도 원래 identity를 삭제하지 않는다.

~~~text
ExecutionBasisExpansion
  source semantic/logical op
  applicability constraints
  expansion region/graph
  equivalence witness/rule id
  preserved numeric/error/order contract
  provenance
~~~

예:

~~~text
Conv
  ↔ WindowView + Contract

Pooling
  ↔ WindowView + Reduce

MatMul
  ↔ Contract

Mean
  ↔ Reduce(Add) + Tally + Divide
~~~

expansion은 mandatory lowering이 아니다. planner/optimizer는 original identity와 expansion 양쪽을 이용할 수 있다. refinement는 4.24.11의 성능 기준을 만족할 때 추가한다.

**명칭 구분:** 고수준 연산을 `WindowView/IndexMap`, `Multiply`, `ReduceSum/Contract` 등으로 선택적으로 분해하는 설계 동작을 **Logical Decomposition(논리적 연산 분해)**이라고 부른다. 기존 `ExecutionBasisExpansion`은 이 동작을 표현하는 내부 sidecar/계약이다. 이는 선택된 **Optimized J Graph**(변환 미선택 시 원본 **J Operation Graph**)를 Logical IR에서 실행 의미에 맞게 설명하는 선택적 동작이다. 그 결과의 operation/SSA 의존성 그래프를 **Logical Operation Graph**라 부르되, 이는 기존 `logical_ir::Plan`의 표현 관점이며 새로운 독립 IR이나 모든 고수준 노드의 강제 분해가 아니다. IndexMap은 데이터 접근 관계일 뿐 `CellApply`가 소유한 J implicit rank loop를 의미하지 않는다. Graph IR의 **Algebraic Expansion(대수적 확장)**은 고수준 Primitive를 비실행 sum-product로 투영해 주변 연산까지 재작성하여 *새로운 Graph 후보를 발견*하는 별도 과정이다. 두 과정 모두 중간 배열을 물질화하거나 실행 경로를 보장하지 않으며, 동일한 semantic/index 계약과 별도의 proof/guard를 사용한다. [Graph/Logical 책임](./03-architecture-p01.ko.md#두-ir-사이의-최적화-책임).

<a id="공통-logicalbasisop-contract"></a>
##### 공통 LogicalBasisOp contract

모든 basis operation은 conceptually 최소한 다음 query가 가능해야 한다.

~~~text
LogicalBasisOp
  identity
  operands / results
  ResolvedInstantiation
  IterationDomain
  AxisSemantics
  AccessFact[]                  // Known(...) | Opaque
  NumericSemantics
  ConstraintSet
  possible J error set / MayRaise
  EffectSummary
  SpeculationSemantics
  DestinationRelation
  StorageRequirement
  semantic/basis provenance
~~~

모든 field가 항상 Known일 필요는 없다. Unknown/Opaque는 language invalidity가 아니라 해당 optimization/route의 정보 부족이다.

<a id="lowering-capability-공통-형태"></a>
##### lowering capability 공통 형태

basis node는 implementation을 내장하지 않는다. registry가 다음 형태의 후보를 제공한다.

~~~text
ExecutionBasisLoweringCapability
  basis/op family
  applicability predicate over:
    ResolvedInstantiation
    ResolvedCallFacts
    ValueFacts / ArrayPropertyFacts
    RepresentationFacts
    TargetCapability
  semantic guarantees:
    exact numeric/error/order behavior
    supported representation
    supported access/alias form
  realization family id
  optional ResourceEstimate model
~~~

candidate legality와 candidate preference를 분리한다.

~~~text
legal(candidate, facts, target)
        ≠
preferred(candidate, CostProfile)
~~~

library가 존재하거나 specialized kernel이라는 이유만으로 자동 선택하지 않는다.

<a id="execution-basis별-최소-node-contract"></a>
##### execution basis별 최소 node contract

| Basis family | 최소 logical payload | 최소 facts / SemanticCheck | 최소 lowering capability family |
|---|---|---|---|
| IndexSpace / Generate | output shape/domain, index-to-value payload | shape extents nonnegative/representable, dtype/result shape | scalar loop, SIMD/SIMT index generation, constant/iota intrinsic, fused producer |
| Elementwise / MapN | scalar/cell payload, input access maps, output domain | dtype/promotion/domain, J agreement already resolved or wrapped by CellApply, error-order policy | scalar, SIMD, SIMT, fused elementwise, target intrinsic |
| CellApply | operand function graph, frame/cell split, valence, assembly policy | EffectiveCellRank, AgreementFact, RepetitionFact, FillAndEmptySemantics, AssemblyFact, length/domain/error order | GenericCellLoop, CanAbsorbCellApply, uniform MapCells/flat GPU route, batched structured op |
| RegularReindex / StaticReindex | output shape and result-index → source-index relation, fill/boundary policy | shape/item-count relation, index-map validity, view equivalence witness | metadata/view, consumer index-map fusion, layout absorption, materializing copy |
| Gather | source, index value, output domain, indexed axes | IndexVector role, bounds/rank checks, duplicate-read harmlessness, source representation | scalar/indirect gather, vector gather, GPU indexed read, fused consumer |
| Scatter / Amend | source/destination value, indices, updates, collision/order policy | bounds, alias/destination legality, duplicate-index ordering, J error order | out-of-place update, legal in-place update, GPU scatter, sorted/index-grouped route |
| ScatterCombine | destination/index/update plus combine op | combine algebraic facts, collision semantics, identity if required, bounds | atomic combine, privatized histogram, sort+segment reduce, sequential exact-order fallback |
| WindowView | source, output shape, window shape, stride/dilation/padding/fill, index relation | WindowSpec/Stride/Dilation/Padding roles, result shape, boundary/fill checks | virtual/index-only view, composed reindex, tiled/shared-local window, fused stencil/consumer |
| SegmentView | source plus offsets/lengths/flags descriptor and segment order | SegmentDescriptor validity, monotonic/range/coverage facts as applicable | virtual segmented view, offset/CSR-style traversal, segmented map/reduce/scan, materialized segment fallback |
| Permute | source plus permutation or permutation mapping | Permutation/Unique/KnownRange facts, validity check | view when representable, gather permutation, in-place cycle algorithm, out-of-place permutation |
| Reduce | reducer, axes, identity/empty policy, order/reassociation contract | reduced axes, dtype/accumulator, associativity/commutativity only when proven, FillAndEmptySemantics, MayRaise | serial ordered, SIMD tree, thread/tree, warp/subgroup, multi-stage, library reduction |
| Scan | operator, axis, direction, inclusive/exclusive/J prefix policy, identity if applicable | same numeric/algebraic facts as reduction plus exact prefix order contract | serial scan, parallel prefix, segmented scan when combined with SegmentView |
| Contract | input access maps, parallel axes, contraction axes, combine op, aggregate op | shape/axis compatibility, accumulator/numeric policy, reducer algebraic facts, empty contract | generic nested reduction, tiled CPU, GEMM/microkernel, tensor instruction, external library |
| Concat / Assemble | ordered inputs/cell results, assembly axis/policy | shape agreement, dtype promotion, heterogeneous/boxed assembly, FillAndEmptySemantics | virtual concat where legal, direct destination writes, memcpy/copy chain, producer-to-destination fusion |
| Replicate / Compact / Expand | source, counts/mask/positions, output-order policy | CountVector/boolean mask, nonnegative/integral counts, output-size overflow, stable order contract | serial expand, prefix-sum + scatter, GPU compaction, fused consumer |
| Grade | input, direction/order comparator contract, output permutation semantics | ordering/tolerance/fit, dtype/comparator legality, tie/order semantics | comparison sort, radix/key sort, small-array network, GPU sort, library route |
| Lookup / Classify | haystack/domain, query/items, mode(first/last/member/interval/self-classify), equality/order contract | equality/tolerance, sortedness/uniqueness/range facts when available | linear probe, hash table, direct-address/bitset, binary search, sort/merge lookup |
| GroupBy | keys, payload reference, group-order contract, descriptor result | equality/tolerance, group ordering semantics, key range/sortedness if known | hash grouping, sort grouping, direct bucket, reduce-by-index/atomic route, sequential stable fallback |
| NestedTraverse | boxed/nested source, traversal selector/path/level, visit function, reconstruction policy | nesting/box facts, path/level validity, heterogeneous assembly and errors | recursive/reference traversal, flattened leaves + offsets/segments, level-specialized traversal |
| LinearSolve | A/B operands, problem kind, transpose/least-squares/inverse semantic contract | rank/shape compatibility, numeric dtype, singularity/rank-deficiency/error semantics | generic reference solver, LU/QR/SVD family, dense CPU/GPU library, sparse solver route |
| StateMachine | transition table, input classifier, initial state, emission/output policy | table shape/type, state/input-class bounds, output assembly, error semantics | scalar sequential reference, table-specialized loop, transition composition/vector route when proven legal |

<a id="execution-basis-간-composition-rule"></a>
##### execution basis 간 composition rule

basis가 커졌다고 해서 nested structure를 즉시 평탄화하지 않는다.

~~~text
CellApply(Reduce(...))
WindowView → Contract
GroupBy → SegmentView → Reduce
NestedTraverse → CellApply(...)
~~~

는 유효한 Logical IR이다. optimizer는 다음 proof가 있을 때만 더 큰 realization으로 흡수한다.

- CellApply absorption / uniform assembly
- index-map composition
- reduction reassociation legality
- grouped-update collision semantics
- numeric/error-order equivalence
- representation compatibility

반대로 named high-level op를 basis graph로 열었더라도 optimizer가 library/tensor/solver route를 위해 original semantic identity를 이용할 수 있어야 한다.

<a id="a3-v0--v1-구현-범위"></a>
##### A3-v0 / v1 구현 범위

basis vocabulary 전체를 A3-v0의 선행조건으로 만들지 않는다.

~~~text
A3-v0 required executable core
  Elementwise
  CellApply
  Reduce
  RegularReindex/StaticReindex
  IndexSpace/Generate
  SemanticCheck
  + Opaque/Unknown access fallback

A3-v1 priority expansion
  Scan
  Gather / Scatter
  WindowView
  SegmentView
  Contract
  Concat / Assemble
  Replicate / Compact / Expand

later / workload-driven
  Grade
  Lookup / Classify
  GroupBy
  NestedTraverse
  LinearSolve
  StateMachine
  specialized ExecutionBasisExpansion rules
~~~

later로 둔 operation도 language semantics를 later까지 금지한다는 뜻이 아니다. 해당 basis lowering이 없으면 기존 native/runtime semantic path 또는 structured-op fallback이 correctness를 담당한다.

<a id="현재-a3-direct-lowering-구현-경계"></a>
##### 현재 A3 direct-lowering 구현 경계

M1 cutover 이후 `analysis::lower_graph()`가 canonical `logical_ir::Plan`을 직접 생성한다. 모듈/API와 완료 상태의 정본은 [§10 M1](./11-roadmap-checklists-p01.ko.md#architecture-migration-checklist), [§12](./13-current-status.ko.md#current-implementation-status)다. 아래는 basis 계약의 구현 경계다.

shared execution-semantic contract:
- `CallOp.execution_basis`: outer→inner `ExecutionBasis.layers`를 기록한다. 예: ranked reduction은 `[CellApply, Reduce]`를 보존한다.
- ResolvedInstantiation: target/valence/input-output dtype·rank/requested rank boundary를 기록한다.
- ValueRoleFacts: ShapeVector, IndexVector, CountVector, AxisPermutation 등 문맥상 role을 noun type과 분리한다.

A3 logical_ir 쪽:
- Operation과 SSA ValueData를 분리하여 zero-result operation을 표현할 수 있다.
- SemanticCheck는 실제 zero-result ordered op이며 PrefixAgreement, CellFrameAgreement, IndicesInBounds constraint를 우선 지원한다.
- ConstraintSet과 FactWitness로 static proof가 있는 check와 unresolved check를 구분한다.
- basis node는 family-specific ExecutionBasisPayload와 IterationDomain/axis role을 가진다. 현재 A3의 `OpKind::Basis.kind`는 routing을 위한 outer execution basis이고, `CallOp.execution_basis`가 전체 outer→inner composition을 보존한다.
- EffectSummary와 SpeculationSemantics는 conservative PrimitiveContract에서 초기화되며 이후 proof-driven refinement가 가능하다.
- src/lowering.rs의 ExecutionBasisLoweringCapability registry는 legality만 판정하며 cost/preference와 분리된다.
- native candidate가 없으면 RuntimeSemanticFallback으로 분류하며 invalid J로 취급하지 않는다.
- src/expansion.rs는 원 semantic op를 지우지 않는 optional multi-node ExecutionBasisExpansion sidecar를 제공한다.
- 첫 실제 expansion은 dyadic E. 이며 J Dictionary의 x E. y ↔ ($x) x&-: ;.3 y identity를 근거로 WindowView → CellApply(Match) graph를 제공한다.
  - reference: https://www.jsoftware.com/help/dictionary/decapdot.htm

아직 구현하지 않은 핵심:
- multi-block CFG와 branch/loop/exceptional control-flow (single Function/Region/Block/Return container는 구현됨)
- richer AccessRelation/AxisSemantics payload와 cell-rank가 완성된 ResolvedInstantiation
- general ConstraintSet lattice / runtime Guard
- SemanticCheck discharge/refinement witness가 SpeculationSemantics refinement로 이어지는 proof pass
- WindowView/SegmentView/Contract 등 v1 basis의 executable lowering capability
- CostProfile / preference ranking
- optimized native lowering과 reference executor 사이의 broad semantic equivalence corpus
- environment/state를 받는 general A3 executor

현재 correctness oracle로 src/logical_executor.rs의 closed-plan reference executor를 추가했다. 이 경로는 기존 semantic kernels를 재사용하며 physical schedule/bufferization을 하지 않는다. A3의 operation order, zero-result SemanticCheck, SSA wiring, basis payload가 기존 runtime과 같은 결과/error class를 내는지 검증하는 용도다.

zero-result SemanticCheck는 canonical A3 op/result-separated IR에서 표현한다. 제거된 transition container를 새 기능을 위해 복원하지 않는다.

<a id="basis-contract-검증"></a>
##### basis contract 검증

각 basis op에는 최소 세 종류의 test가 필요하다.

1. verifier negative tests
   - 잘못된 axis/index/shape/role/descriptor를 reject
2. semantic/reference equivalence tests
   - generic/reference path와 optimized lowering의 result/error equivalence
3. composition tests
   - CellApply+Reduce, Window+Contract, GroupBy+Reduce, sparse representation 조합처럼 실제 fusion boundary를 포함

J error가 있는 case는 값만 비교하지 않고 **error class와 observable ordering**까지 비교한다.





---


---

