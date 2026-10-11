[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)

> 문서 분할 안내: 기존 `PROJECT.ko.md`의 내용을 이동한 모듈입니다. 원문의 단일 파일 정본·신규 Markdown 금지 문구는 최신 [Agent 문서 규약](../../../AGENTS.md)보다 우선하지 않습니다.

# 4. J Graph Analyzer와 Execution Semantic Lowering — 계속 (2/2)

##### 4.1.2.6 다른 배열 컴파일 프레임워크와의 대조

[정본 내 프레임워크 비교](#framework-graph-hints)에 JAX/XLA, MLIR Linalg, TVM Relax, Halide의 공식 설명과 소스를 바탕으로 F1–F9를 기록했다. 추가 힌트는 fork 재결합 영역, consumer 수, 입력 수명, cell mapping, window 공유 영역, loop invariant, 값/effect live-out이다. 각 항목은 출처와 J 적용 추론을 분리한다.

##### 4.1.2.7 RustJ에 반영할 설계와 검증 체크리스트

제안하는 처리 순서:

```text
품사/binding 확인과 parsing
  → syntax 구조에서 graph + 최적화 후보 생성
  → leaf 계약·효과·오류·alias로 적법성 검사
  → shape/layout와 비용으로 실행 전략 선택
```

Graph 노드/영역의 설계 후보: `Fork`, `Hook`, `Compose`, `CellApply`, `BoundOperand`, `LoopRegion`, `SelectionRegion`. 결과 선택은 `Fork` 가운데 verb의 계약을 해석한 값 사용 정보로 둔다. 실제 구현 API 이름은 아니다.

각 후보에 source span, syntax origin, 적용 valence/rank, 데이터 edge, 효과 검사 상태를 남긴다. 예를 들어 `ParallelCandidate(origin=Fork)`는 **병렬 후보를 발견한 상태**이지 `ParallelSafe`나 실행 계획 확정 상태가 아니다. 이름이 verb인지 noun인지 모르는 상태에서 토큰 세 개만 보고 fork를 확정하지 않는다.

2026-10-02 프레임워크 비교 당시의 코드 대조 기준(`89b87b8`)에서는 `src/j_graph_ir.rs`가 `GraphForm::Pipeline/Hook/Fork/Reduce/PrefixInfix/Rank`와 `GraphHint::ParallelBranchCandidate` 등을 이미 보존한다. `classify_function()`은 leaf kernel 구현을 분석하기 전에 fork에서 병렬 후보를 만든다. 이 사실은 실행 병렬화 완료를 뜻하지 않는다. Bond/Compose/Power/Agenda/Fix의 일반 지원과 S1–S12 전체 최적화는 별도로 검증해야 한다. 자세한 현재 상태는 A1.5를 따른다.

- [x] 사용자 예의 parallel 후보가 바깥 fork syntax에서 나온다고 명시했다.
- [x] leaf를 black box로 두고도 얻는 syntax 힌트 S1–S12를 검토했다.
- [x] 일반 compiler 정보 분류와 J syntax 대응을 분리했다.
- [x] modifier 적용 구조와 leaf kernel 힌트를 구분했다.
- [x] 후보 생성과 적법성·비용 판단을 별도 단계로 정리했다.
- [ ] 구문 지원 시 named verb만으로 fork/hook/compose graph를 검사하는 회귀를 추가한다.
- [ ] fork 두 branch 사이에 불필요한 반환값 의존 edge가 생기지 않는지 확인한다.
- [ ] 사용자 예의 바깥 fork와 내부 hook이 별도 구조로 보존되는지 확인한다.
- [ ] effectful branch에서도 parallel 후보는 생성되되 안전성 실패 후 실행 순서가 보존되는지 검증한다.
- [ ] rank/valence·이름 재정의·Agenda·Power의 영역 의미를 검증한다.
- [ ] 후보 적용 전후 값·dtype·shape·오류·효과 순서를 Windows 네이티브에서 비교한다.

기존 M1–M6/frontend 이행 순서와 G1–G5 배열 설계 우선순위가 실행 계획의 기준이다. 이번 검토는 그 계획을 보강하며 조사한 모든 syntax 구현을 선행 조건으로 추가하지 않는다. 실제 CUDA 보류는 유지한다. 이번에는 문서만 수정했으며 Rust/C 실행 테스트, 성능 측정, 새 parsing 또는 병렬 실행 구현은 수행하지 않았다.

<a id="framework-graph-hints"></a>

#### 4.1.4 Candidate lifecycle와 proof-discharge contract

J Graph IR이 candidate를 발견한 뒤 실제 transformation으로 commit하기까지의 상태를 **하나의 `selected` bool로 표현하지 않는다.** legality, target feasibility, resource feasibility, cost, selection은 서로 다른 질문이며 서로 다른 evidence를 가진다.

개념적으로 candidate는 다음의 **직교한 evidence 축**을 가진다.

~~~text
CandidateEvidence
  provenance        Verified | Stale/Invalid
  equivalence       Unknown | Proven | Disproven | Guarded(GuardId)
  semantic_legality obligation별 Unknown | Proven | Disproven | Guarded(GuardId)
  target_feasibility Unknown | Supported | RequiresFacts | Unsupported
  resource_state    Unknown | Symbolic | Resolved | ExceedsHardLimit
  cost_state        Uncosted | Estimated(CostEstimate)
  selection         Unselected | Selected | Rejected(reason)
  lowering_state    NotLowered | Lowered(Transform/Route identity)
~~~

이 축을 하나의 선형 enum으로 저장할 필요는 없다. planner/UI가 다음과 같은 **derived lifecycle summary**를 만들 수는 있다.

~~~text
Discovered
   ↓ source/provenance verification
AwaitingProofs
   ├─→ Illegal
   └─→ Legal or GuardedLegal
          ↓ hard target/resource feasibility
       Feasible
          ↓ cost evidence
       Costed
          ↓ compatibility + global/local choice
       Selected / Rejected
          ↓ committed lowering
       Lowered
~~~

단, 이 화살표는 분석 pass의 실행 순서를 강제하지 않는다. resource/work-depth/cost 분석은 legality proof가 끝나기 전에도 **speculative side analysis**로 계산할 수 있다. 금지되는 것은 필요한 legality proof가 끝나기 전에 candidate를 실행 plan으로 **commit**하는 것이다.

##### Evidence owner

| Evidence / 질문 | 주 소유자 | 의미 | selection에 대한 규칙 |
|---|---|---|---|
| source topology / provenance | J Graph verifier + candidate registry | candidate가 현재 source/region/rule version에서 실제로 유도되었는가 | stale provenance면 즉시 폐기 |
| algebraic equivalence | rewrite/scan/fusion rule의 witness validator | source와 replacement/composite identity가 같은가 | 필요한 equivalence가 Unknown이면 commit 금지 |
| rank/cell/assembly | Execution semantic facts + candidate legality checker | CellApply/assembly/error 의미가 보존되는가 | Proven 또는 effect 이전 Guard 필요 |
| numeric / tolerance / reassociation | primitive/derived numeric contract | overflow, `!.`, tolerance, floating-order contract가 보존되는가 | semantic relaxation 없이는 임의 reassociation 금지 |
| effect / error ordering | effect/error/speculation analysis | observable write/error 순서를 바꾸어도 되는가 | Disproven이면 candidate illegal; guard가 effect 뒤라면 사용 불가 |
| fanout / retention / alias | graph use/liveness + alias/storage facts | producer 복제, retained value, reuse가 합법인가 | external use를 잃거나 alias proof 없으면 해당 transform 금지 |
| resource / work-depth | `j_graph_resource` / `j_graph_work_depth` | symbolic state, internal traffic, work/depth, hard resource need | cost와 분리. hard target limit 초과만 feasibility 거부 근거 |
| target/lowering capability | LoweringRegistry × resolved target | 해당 op/region을 실제 realization으로 내릴 수 있는가 | source-basis support만으로 fused/composite legality를 추정하지 않음 |
| empirical profitability | CostProfile / planner | legal candidates 중 무엇이 유리한가 | legal candidate를 단지 느리다는 이유로 semantic invalid로 만들지 않음 |
| final compatibility/selection | Schedule/Transform planner | 후보들의 겹침·순서·route를 함께 선택 | selected set 전체가 상호 호환되어야 함 |

##### Guarded legality

`Guarded(GuardId)`는 “증명하지 못했지만 일단 실행”이라는 뜻이 아니다. 다음 조건을 모두 만족해야 한다.

1. guard가 transformation이 의존하는 fact를 실제로 검증한다.
2. guard는 해당 region의 observable effect보다 먼저 실행된다.
3. guard miss의 대체 route가 명시되어 있다.
4. miss 후 원래 J semantics를 재실행해도 중복 effect가 생기지 않는다.
5. guard identity/provenance가 specialization/candidate cache와 연결된다.

effect가 이미 commit된 뒤에는 guard miss/Unsupported를 이유로 source region을 자동 replay하지 않는다.

##### Candidate overlap과 selection

candidate overlap은 곧바로 오류도 아니고 곧바로 composition 가능도 아니다. v0에서는 보수적으로 다음을 적용한다.

- 같은 source operation을 **대체**하는 두 rewrite는 동시에 select하지 않는다.
- rewrite와 fusion이 같은 source operations를 겹쳐 소유하면 registered compatibility/composition rule이 없는 한 동시에 commit하지 않는다.
- 둘 다 유용할 수 있으면 먼저 하나를 선택·적용해 **새 graph/version**을 만든 뒤 그 결과에서 candidate discovery를 다시 수행할 수 있다.
- candidate의 profitability 비교 때문에 원 source graph/witness를 파괴하지 않는다.
- `CandidateId`/analysis-local index는 특정 Plan/graph version에 귀속되며 source/registry version이 바뀌면 재검증한다.
- selection 결과는 semantic IR 자체가 아니라 별도 Transform/Schedule plan이다.

##### 현재 코드와 목표 계약의 대응

현재 구현은 이 전체 lifecycle의 일부만 갖는다.

~~~text
GraphRewriteCandidate
  provenance + equivalence witness
  target-independent

FusionCandidate
  proof obligations
  legality = Unknown
  resource_transfer_proven = false
  selected = false

RewritePlanningReport
  TargetUnsupported / NeedsCallFacts / NeedsResourceFacts / ReadyForCosting

FusionReadinessReport
  AwaitingSemanticProofs
  fused target query deferred
  selected = false
~~~

아직 공통 `CandidateEvidence`/`ProofBundle`, obligation별 proof discharge 결과, compatibility-aware selection plan, committed transform identity는 구현되어 있지 않다. 위 타입 이름은 목표 개념을 설명하며 현재 API 완료를 주장하지 않는다.

##### 구현 시 분리 순서

현재 M2 frontend 우선순위를 바꾸지 않는다. optimizer 단계에 착수할 때는 다음처럼 **한 번에 한 의미 + 한 verifier/test**로 추가한다.

1. proof 결과의 공통 state/provenance 표현 + stale evidence verifier
2. fusion obligation별 discharge 결과를 기록하되 selection은 하지 않음
3. rewrite/scan 후보에도 동일한 legality view를 투영
4. target hard-feasibility와 semantic legality를 합치지 않는 readiness view
5. candidate compatibility/overlap 검사
6. costed candidate set에서 별도 SelectionPlan 생성
7. selected candidate만 committed transform/lowering으로 넘기고 source provenance를 검증

### 4.2 Execution Semantic Lowering의 책임

- J Graph node를 explicit execution dataflow로 전개
- monad/dyad valence와 actual call instantiation 확정
- primitive semantic contract 적용
- dtype / shape / cell / frame / agreement / rank-result-assembly fact 전파
- iteration domain / axis semantics / access relation 도출
- CellApply / Reduce / Gather / Contract 등 normalized execution/basis operation 생성
- ConstraintSet / FactWitness / SemanticCheck 생성
- effect / alias / speculation legality 분석
- invariance / semantic storage requirement / representation-side fact의 경계 설정
- J observable error/evaluation ordering 보존
- J Graph node → execution op provenance(`j_origin`) 유지
- verified Logical Execution IR 생성

다음은 이 단계의 책임이 아니다.

- 특정 backend capability를 보고 schedule을 선택하는 일
- fusion region을 실제로 확정하는 일
- optional intermediate를 실제 buffer로 materialize하는 일
- tile/vector/workgroup/layout/device를 고르는 일
- target cost model로 후보를 ranking하는 일

이 결정들은 Route Partition/Export, 외부 compiler, 또는 RustJ-native Logical Optimizer/Physical Planner가 담당한다.

과거 `jaxa-analyzer` 문서에서 “analyzer가 hardware profile을 받아 fusion/resource를 결정한다”고 한 표현과 충돌하지 않도록 용어를 재해석한다.

```text
old "JAXA analyzer"
  ≈ current Semantic Analyzer
    + native Logical Optimizer
    + Schedule / Transform planning
    + Physical Planner / Resource analysis
```

현행 RustJ에서 **Semantic Analyzer라는 좁은 단계만** target-independent다. 과거 analyzer의 hardware-dependent 기능을 버린 것이 아니라 downstream planning 단계로 분리한 것이다.

### 4.3 두 middle-end가 공통으로 하지 않는 일

- source text tokenization
- parser stack 규칙의 재실행
- source를 다시 parse하여 의미를 복원
- 직접 CPU loop 실행
- 직접 CUDA kernel 실행
- Executor 단계에서 의미론을 다시 판단
- 알 수 없는 정보를 임의로 추측

즉 Graph Analyzer와 Execution Lowering은 scanner/parser stack mechanics를 재실행하지 않는다. 대신 parser가 완성한 FunctionEntity와 J Graph IR의 구조를 정식 compiler input으로 사용한다.

### 4.4 Execution semantic lowering 이후의 generic 경계

다른 frontend와 공유할 가능성이 높은 지점은 J Graph IR 이전이 아니라 **J graph analysis와 execution semantic lowering을 마친 뒤의 Logical Execution IR / Plan**이다. J Graph IR은 의도적으로 J-specific하다.

```text
J frontend / FunctionEntity
    ↓
J Graph IR + Graph Analyzer      ← intentionally J-specific
    ↓
Execution Semantic Lowering
    ↓
Logical Execution IR / Plan     ← generic compiler boundary
    ↓
Route partition / export
    ├─ RustJ-native optimizer/planner
    ├─ MLIR
    ├─ StableHLO-compatible subset
    └─ library/custom backend
```

향후 다른 array DSL frontend를 붙이고 싶다면 두 선택이 가능하다.

1. J semantic model을 의도적으로 공유하면 J Semantic Array IR을 생성한다.
2. J와 무관한 frontend라면 자기 semantic analyzer를 거쳐 Logical Array IR / Plan에 합류한다.

따라서 **middle-end를 generic tensor IR consumer처럼 만들기 위해 J의 구조를 일찍 버리지 않는다.**

#### 4.4.1 분석 fact는 typed lattice로 관리하고 semantic error와 분리한다

shape, alias, invariance, effect, binding, constraint 같은 서로 다른 분석 정보를 하나의 범용 `Unknown` 값으로 뭉개지 않는다.

각 fact domain은 자기 lattice를 정의한다. 모든 domain이 동일한 enum을 강제로 공유할 필요는 없지만 공통적으로 다음 개념을 갖는다.

```text
analysis state
  Uninitialized
  Known(T)
  Overdefined / Unknown
  domain-specific bottom/unreachable if needed
```

control-flow merge나 여러 predecessor에서 fact가 합쳐질 때는 domain별 monotonic `join`을 사용한다. MLIR data-flow framework처럼 lattice state는 **분석 지식의 상태**를 나타낸다.

예:

```text
ShapeFact
AliasFact
InvarianceFact
ConstraintFact
EffectFact
BindingFact
```

`Unknown`은 사실을 임의로 꾸며내지 않는다는 뜻이지 곧바로 실행 불가를 뜻하지 않는다. domain과 route에 따라 다음 중 하나가 된다.

- optimization barrier
- runtime witness/guard 필요
- conservative lowering
- external route rejection
- 재분석 조건

특히 `AccessRelation`은 모든 valid J op가 v0부터 완전한 affine/index-map contract를 가져야 한다는 뜻이 아니다.

```text
AccessFact
  Known(AccessRelation)
  Opaque / Unknown
```

로 둘 수 있다.

- `Known`: fusion, locality, vectorization, advanced scheduling 분석 가능
- `Opaque/Unknown`: J semantics 자체는 valid할 수 있으며, access-sensitive optimization의 barrier가 됨
- route가 full access contract를 요구할 때만 해당 route에서 reject/Unsupported
- conservative/native/runtime semantic path가 있으면 실행 자체를 금지하지 않음

따라서 **hardware-aware Logical IR은 hardware-relevant fact를 표현할 수 있어야 하지만, 모든 op가 v0부터 모든 fact를 Known으로 제공해야 한다는 뜻은 아니다.**

중요하게, **J semantic error는 lattice element가 아니다.**

```text
UnreachablePath / UnsatisfiableConstraint
        ≠
JSemanticError(domain/rank/length/value/...)
```

- `UnreachablePath`는 control-flow/constraint 분석 결과다.
- `JSemanticError`는 source semantics에 따라 진단하거나 runtime error behavior로 보존해야 하는 프로그램 의미다.
- 어떤 path에서 반드시 error가 발생한다고 증명되더라도 optimizer는 그 error의 관찰 가능한 순서를 `SpeculationSemantics`와 effect ordering에 따라 보존해야 한다.

따라서 분석 lattice의 bottom/top 개념과 사용자-visible error contract를 같은 `Invalid` 상태로 합치지 않는다.

<a id="read-physical"></a>

---

[← 이전 부분](./05-graph-execution-lowering-p01.ko.md)
