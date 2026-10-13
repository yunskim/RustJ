[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)


<a id="read-physical"></a>
<a id="5-logical-plan-physical-plan-executor"></a>
## 5. Logical Plan, Physical Plan, Executor

<a id="51-logical-array-ir--logical-execution-plan"></a>
### 5.1 Logical Array IR / Logical Execution Plan

이 계층은 Semantic Analyzer가 hook/fork/derived verb/rank 같은 고수준 의미 구조를 분석한 뒤 만든 **명시적 배열 dataflow**다.

여기서 `Reduce`, `CellApply`, `StaticReindex` 같은 이름은 **Logical IR에서 처음 등장하는 normalized operation**이다. J Semantic IR의 parser-produced function graph에는 이 이름으로 modifier application을 대체하지 않는다. `MapCells`는 `CellApply`의 uniform result/assembly 조건이 증명된 뒤 사용할 수 있는 더 제한적인 lowering form이다.

```text
Semantic IR
  / : Verb
  └─ + : Verb

       ↓ Semantic Analyzer

Logical IR
  Reduce(reducer=+)
```

마찬가지로:

```text
Semantic IR
  " : Verb                 // derived result POS
  head = PrimitiveConjunction(Rank)
  ├─ left:  u
  └─ right: r

       ↓ Semantic Analyzer

Logical facts / op
  resolved RankSpec
  frame/cell mapping
  CellApply
    └─ optional later MapCells-style lowering when UniformProven
```

현재 코드의 `Callable.reduce` / `Callable.rank`는 기존 analyzer/runtime와 연결하기 위한 **migration field**다. 최종 A3-v0 Logical IR에서는 parser-derived operator graph를 해석한 결과를 normalized logical operation/fact로 표현하고, 이 bool/array shortcut을 semantic identity로 사용하지 않는다.

예를 들어 고수준의

```text
Apply(
  Fork(
    AdverbApplication(operator=/, operand=+),
    %,
    #
  ),
  y
)
```

는 분석 후 개념적으로 다음과 같은 dataflow가 될 수 있다.

```text
            Input y
           /       \
  Reduce(Add)      Tally
           \       /
             Divide
```

여기서 일반 dataflow 실행 의미는 fork 표기 없이도 표현할 수 있지만, **원래 source가 fork/hook/@:였다는 topology provenance는 optimization 정보로 계속 보존한다.** §4.1.1의 StructuralOpportunity sidecar가 pipeline/branch-join/live-across 정보를 명시적으로 운반하므로 optimizer가 generic DAG에서 이를 다시 pattern-match할 필요가 없다. 진단·debug provenance이기도 하지만 그것에 한정되지 않는다.

Logical Plan에서 보존할 정보:

- SSA ValueId와 region/block/control-flow 구조
- normalized array operation
- dtype/shape/rank/cell/frame facts
- iteration domain / axis semantics / access relation
- symbolic constraints와 witness/guard
- data dependency와 effect ordering token
- effect / alias / speculation facts
- invariance / semantic-mask facts
- semantic StorageRequirement
- target-independent rewrite/fusion constraints
- source/semantic origin metadata

특정 backend support 여부, concrete fusion region, buffer allocation, tile/layout/device 결정은 Logical IR의 본질적 fact가 아니다.

아직 특정 device buffer 주소나 CUDA launch parameter는 없다.

<a id="52-schedule--transform-plan과-physical-plan을-구분한다"></a>
### 5.2 Schedule / Transform Plan과 Physical Plan을 구분한다

RustJ-native route에서는 Logical IR을 바로 buffer plan으로 덮어쓰지 않는다.

```text
Logical Array IR
   ↓
Logical Optimizer
   ↓
Schedule / Transform Plan
   ↓
Physical Planner / Bufferization
   ↓
Physical Plan
```



여기서 `Logical Optimizer`는 target-independent canonicalization/DCE/CSE와 semantic-preserving graph rewrites를 담당한다. 특정 tile/layout/device/resource를 선택하거나 target cost로 후보를 확정하는 일은 Schedule / Transform Plan 이후의 책임이다.

**Schedule / Transform Plan**은 payload semantics와 분리된 선택/변환 의도를 표현한다.

- fusion/grouping
- tile hierarchy
- loop/axis mapping
- vectorization
- unrolling
- tensorization/intrinsic selection 후보
- layout transform 요청
- memory-space staging 요청
- software pipeline/prefetch 전략

이는 MLIR Transform dialect나 TVM TensorIR schedule처럼 “무엇을 계산하는가”와 “어떻게 변환할 것인가”를 분리하는 역할이다.

Schedule Plan은 여러 후보를 가질 수 있고 CostProfile/autotuning/backend feedback에 의해 바뀔 수 있다. 따라서 Logical IR의 semantic identity가 아니다.

**Physical Plan**은 선택된 schedule을 실제 resource/buffer/execution 객체로 구체화한다.

- CPU/GPU placement
- buffer binding / ownership
- physical view
- concrete materialization/copy
- contiguous/fixed/general stride specialization
- physical layout / padding / alignment
- memory-space assignment
- transfer
- synchronization/timepoint
- buffer reuse
- work partition
- backend kernel/library 선택
- async lifetime/resource information

<a id="521-m4-최소-physicalplan-v0-contract"></a>
#### 5.2.1 M4 최소 PhysicalPlan v0 contract

M4의 첫 CPU vertical slice를 구현하기 전에 **compiler plan의 resource identity와 runtime handle을 분리한 최소 schema**를 문서로 고정한다.

가장 중요한 구분:

~~~text
Logical ValueId
    ≠
PlanBufferId        // compiler PhysicalPlan 안의 symbolic/planned storage slot
    ≠
physical::BufferId  // runtime BufferRegistry가 발급하는 checked handle
    ≠
raw address
~~~

현재 `physical::BufferId`는 registry identity + slot + generation을 가진 **runtime handle**이다. compiler가 lifetime/reuse를 계획하기 위해 사용하는 slot identity와 그대로 동일시하지 않는다. 실제 구현명은 달라질 수 있지만, plan-time identity와 executor-time lease/handle의 계층은 분리한다.

또한 하나의 planned buffer 위에 여러 physical view가 존재할 수 있으므로 buffer identity와 view identity도 분리한다.

~~~text
PlanBufferId
  storage/resource identity

PhysicalViewId
  PlanBufferId
  logical shape
  strides
  offset
  encoding
  access mode
~~~

<a id="v0-physicalplan"></a>
##### v0 PhysicalPlan

첫 M4 범위는 **verified single-block pure-array CPU region + 필요한 SemanticCheck**로 제한할 수 있다. assignment/name mutation 같은 stateful effect는 첫 slice에서 RuntimeSemantic region에 남겨도 되며, 이것은 J language restriction이 아니다.

개념 schema:

~~~text
PhysicalPlan
  source_logical_schema / provenance
  resolved CPU target
  planned buffers
  physical views
  ordered/dependency-aware ops
  outputs

PhysicalOp
  BindInput
  Check
  View
  Materialize
  Kernel
  Return

future / non-M4:
  Transfer
  Sync / AsyncToken
~~~

각 op의 역할:

- **BindInput** — logical input/read value를 executor가 가진 runtime storage/lease와 연결한다. deep copy를 뜻하지 않는다.
- **Check** — A3 `SemanticCheck`를 J error kind/origin/order와 함께 실행한다. optimizer 편의를 위해 kernel 안으로 숨기거나 제거하지 않는다.
- **View** — transpose/reverse/slice 등 합법한 metadata-only physical view를 만든다. 새 backing allocation을 의미하지 않는다.
- **Materialize** — consumer requirement나 layout/alias 조건 때문에 logical atom order를 보존한 실제 copy/packing을 만든다. 이유/provenance를 남긴다.
- **Kernel** — 이미 선택된 `ParameterizedLoweringRecipe/RealizationFamily`를 입력/output view에 적용한다. rank/hook/fork/fusion legality를 executor에서 다시 판단하지 않는다.
- **Return** — 최종 physical view/ownership을 logical result로 넘긴다. temporary를 output으로 잘못 재사용하지 않게 ownership을 확정한다.
- **Transfer/Sync** — GPU/mixed-route에서만 필요할 수 있으며 M4 CPU v0의 필수 op가 아니다.

<a id="planned-buffer와-lifetime"></a>
##### planned buffer와 lifetime

각 planned buffer는 최소 다음 정보가 필요하다.

~~~text
BufferRequirement
  memory space / CPU class
  encoding
  extent or size expression
  alignment requirement
  ownership class: input | temporary | output
  def / physical uses / last use
  optional reuse witness
~~~

M4 첫 slice가 fully-resolved CPU extent만 지원해도 된다. dynamic extent 지원이 없다는 사실을 J semantic restriction으로 올리지 않고 route capability로 둔다.

buffer reuse는 별도 semantic transform이 아니라 physical decision이다. 같은 `PlanBufferId` 또는 storage slot을 재사용하려면:

1. 이전 physical value의 last use가 끝났고,
2. outstanding view/lease가 그 storage를 관찰하지 않으며,
3. encoding/size/alignment/memory-space requirement가 맞고,
4. alias/destination contract가 허용하며,
5. J-visible effect/error order를 바꾸지 않는다는

reuse witness가 필요하다.

<a id="physicalplan-verifier"></a>
##### PhysicalPlan verifier

executor는 invalid plan을 추측해서 고치지 않는다. 최소 verifier는 다음을 검사한다.

- 모든 buffer/view/op id가 유효하고 use가 definition 뒤에 있다.
- 모든 view의 shape/stride/offset span이 backing extent 안에 있고 encoding/dtype contract가 맞다.
- buffer가 bind/allocation되기 전에 사용되지 않는다.
- `Check`의 ordering edge와 source origin이 A3 SemanticCheck에서 유도되었고 누락/중복되지 않는다.
- Kernel의 chosen realization이 resolved target과 lowering capability에 맞고 필요한 input/output view contract를 만족한다.
- write 가능한 overlapping views가 proof 없이 동시에 사용되지 않는다.
- Materialize가 logical atom order/value semantics를 보존한다.
- reuse는 last-use + alias/ownership witness 없이는 허용하지 않는다.
- Return은 유효한 output ownership/view를 가리키고 temporary lifetime 이후 dangling view를 만들지 않는다.
- M4 pure-region plan에는 숨은 namespace/write effect가 없다.

<a id="error--cleanup-contract"></a>
##### error / cleanup contract

- `Check` 실패는 원래 J semantic error로 보고한다.
- Kernel/library 자체의 implementation failure를 임의의 J Domain/Rank/Length error로 바꾸지 않는다.
- plan 실패 시 executor-owned temporary lease/resource는 정리하되 caller-owned input은 파괴하지 않는다.
- first M4 pure slice에서는 namespace assignment commit을 Physical Executor가 소유하지 않는다. stateful write를 native route에 넣을 때 별도 effect/commit contract를 추가한다.
- observable effect가 commit된 뒤 transparent replay하는 fallback은 금지한다.

<a id="m4m5-구현-선행조건과-검증-프로토콜-2026-10-07-설계만-확정"></a>
##### M4→M5 구현 선행조건과 검증 프로토콜 (2026-10-07, **설계만 확정**)

**범위:** 실제 PhysicalPlan/Executor/View/Kernel/Transfer를 지금 구현하지 않는다. 본 절은 기존 §5.2.1의 계약, §17 M4 구현 항목, HE-01~HE-09, IO-20에 적용할 **수용 조건과 검증 증거**다. 별도 Parallel IR, Data Movement IR, 중복 체크리스트는 만들지 않는다. 기존 Draft PR #4의 제한적인 identity 구현은 검증된 전체 M4 경로로 취급하지 않으며 완료 수용과 구별한다.

**컴파일러 경계의 의무와 검증 방법**

| 경계 | 실행 전 필요한 조건·증거 | 수용/거부 검증 | 조건 미충족 시 |
|---|---|---|---|
| **A3 → Route/Physical** | \`logical_ir::Plan::verify\` 성공, schema/primitive-registry/source origin 일치, 한 region 내 포함된 모든 value/check/effect/order edge 목록, 해당 route의 capability·guard/witness 확보 | A3 op 하나씩 추적하는 source→physical mapping; plan에서 Check 하나 삭제·복제·재배열, producer 위조, stale version/guard 위조 시 verifier가 **반드시 거부** | Unknown을 Supported로 치환하지 않음. 해당 region을 합법한 reference/runtime route로 유지하거나 Unsupported |
| **장치·메모리 독립성** | \`ExecutionDevice\`, \`MemorySpace\`, intra-device schedule은 서로 독립한 물리 결정. plan-time \`PlanBufferId\`/view와 runtime \`BufferId\`/lease, logical \`ValueId\` 분리. M4는 CPU+Host+zero-transfer | logical/semantic IR에 thread 수·CUDA stream·buffer address가 없는지 구조 검토; 서로 다른 메모리 위치와 동일 execution device를 **미래 확장 가능**하게 식별; M4 plan에는 Transfer/Sync가 없음을 검사 | 혼합 실행 능력이 없으면 단일 CPU 경로만 지원; J 의미를 바꾸지 않음 |
| **BindInput** | 실제 dense noun의 dtype·shape·encoding·extent·version/owner가 source value와 일치, readonly lease 유효, alias 여부 추적, implicit deep copy 없음 | 바뀐 binding version·stale buffer generation·미등록 BufferId·타인 registry·잘못된 encoding을 각각 하나씩 위조하여 reject; 입력 버퍼 원본 불변 확인 | 입력을 파괴하거나 임의 재바인딩하지 않고 route 거부 |
| **View** | 대상 primitive/Rank·axis 의미가 증명됨, logical shape/atom order 유지; \`shape×stride+offset\`의 최소/최대 backing byte 범위를 부호 있는 checked 연산으로 계산하고 encoding/alignment 적합성 확인. 빈 frame과 빈 cell은 구분 | scalar, rank 1/2/3, \`0 3\`·\`3 0\`·singleton, negative stride, transpose/reverse, overflow·zero stride·OOB 위조. **View는 실제 메모리를 복사하거나 쓰지 않았는지** 확인 | 불확실한 view를 실행하지 않음; 합법한 Materialize/다른 route를 별도 선택 |
| **SemanticCheck / 오류** | A3 Check마다 유일한 provenance, 실행 위치, 의존성, 원래 J error kind 및 error precedence. Guard는 관찰 가능 effect 전 평가; 복구/handler가 오류를 볼 수 있음 | 두 개 이상 Check의 순서 변경, 한 Check 누락·중복, invalid Rank/Length/Index, effect/NAME rebinding/try-catch 관찰 사례. Reference executor와 **오류 종류·첫 오류·부수효과 횟수·상태** 비교 | 물리화 대상 제외; arbitrary first-thread error 또는 GPU trap→J error 임의 변환 금지 |
| **Kernel (첫 Add)** | \`ParameterizedLoweringRecipe\`/registered realization이 resolved target에 합법, valence·rank/cell·prefix agreement·J 타입 승격·overflow/fit·alias·stride 및 전처리 Check 충족. Logical IR의 Parallel 축은 동시 실행 허가 증거 아님 | scalar/vector/matrix, 서로 다른 dtype, bool/int/float, integer overflow/promotion, NaN/Inf, length error, empty arrays, SIMD tail, stride mismatch. 등록하지 않은 recipe 또는 shape/target capability mismatch는 verifier reject; **기존 semantic interpreter를 호출한 결과를 native kernel 통과로 세지 않음** | 기존 reference/지원 route 선택, kernel 미지원은 J Domain error가 아님 |
| **Materialize / Return** | view의 **J 논리 원소 순서**로 allocation·copy, output type/shape 보존; backing/lease와 output ownership, temp last-use 확인 | transpose/reverse의 physical address order와 J atom order 비교, empty/boxed/sparse(지원하지 않는 경우 reject), output dangling/temporary 조기 해제, read-only input mutation 및 조립 에러 비교 | incomplete result를 반환하지 않음; 실패 시 호출자 입력 보존 |
| **Reuse / resource** | def/use/last-use, outstanding view/lease, non-overlap proof, encoding/size/alignment/memory-space 호환, check/effect precedence, budget 상한이 모두 충족 | reuse 중 live alias·overlap write·buffer size 부족·allocation 실패·exception cleanup 등 각 invariant 하나씩 위조; reclaimed resource에 대한 subsequent use 거부 | reuse 비활성화, 안전한 별도 버퍼 사용 또는 route 거부 |
| **CPU↔GPU / I/O 후속(M5+)** | region/device placement, memory residency/version, transfer bytes, ready/completion, sync, ownership/lifetime, host effect/error 의존성 및 capacity. 외부 backend도 계약을 유지 | 가상 plan verifier 수준에서 stale data/transfer 누락/copy-before-producer/read-before-ready/transfer-after-free/중복 commit을 reject. 실제 GPU 결과 동등성·성능은 **검증 가능한 실장치와 명시적 재개 조건** 이후만 수행 | CPU/단일 장치 계획만 합법; 복합 장치 실행을 완료로 주장하지 않음 |

**공통 검증 절차와 완료 판정**

1. **A3 정상 계획 생성·검증 → Physical 계획 생성·검증 → 제한된 CPU 실행** 순서를 지키고, 검증되지 않은 plan은 실행기까지 도달할 수 없게 한다. 정상 사례를 복제해 **불변 조건 하나만** 훼손하는 음성 검증을 기본 패턴으로 삼는다. verifier 실패 이유와 source op/provenance를 기록한다.
2. **3자 의미론 비교:** 동일 입력에 대해 (a) jsource C 기준 실행, (b) RustJ \`logical_executor::execute_closed\`/해당 의미론 기준 경로, (c) 신규 Physical Executor의 값·dtype·Shape·원소 순서·J error class/precedence를 비교한다. C 기준의 해당 기능이 현지 환경에 없으면 미검증으로 남기며 RustJ 내부 2자 비교로 대체 완료 처리하지 않는다.
3. **범위·부정 사례:** zero-frame Rank virtual fill vs positive-frame empty cell, 복합 CellApply 결과 type/shape join, boxed/sparse, tolerance/fit, numeric reassociation, late NAME/assignment/effects, 예외·resource cleanup은 독립 regression으로 분리한다. 미지원 subtype은 **지원 범위 밖**으로 보고하며 J 문법/의미론 제한으로 바꾸지 않는다.
4. **기계적 테스트:** 구현 시 \`cargo fmt --check\`, default/portable \`cargo test\`, \`cargo clippy\`와 재현 가능한 Linux/Windows 경로를 실행하고 커밋 SHA·플랫폼·스레드/feature·케이스 수·passed/failed/ignored를 기록한다. 실행하지 않은 테스트와 CI 결과는 성공으로 보고하지 않는다.
5. **비용은 의미론 통과 후:** Sequential CPU를 zero-transfer baseline으로 두고, 실행 시간 외 bytes read/written/moved, allocation/peak residency, kernel/transfer launch, sync, guard misses, fallback 횟수, critical path를 별도 측정한다. \`ResourceEstimate\`와 \`CostEstimate\`와 실측 \`CostProfile\`을 혼합하거나 Unknown을 0으로 두지 않는다.
6. **M4 수용 게이트:** Check·View·Kernel(Add)·Materialize 중 최소 하나의 **실제 compiler-native 연산 경로가 필요 Check를 포함하여** A3→verified PhysicalPlan→CPU에서 실행되고, 음성 verifier와 jsource differential이 통과해야 한다. 단일 literal \`BindInput→Return\`, 설계 문서 또는 테스트 소스만으로는 M4 전체/HE-01을 완료하지 않는다. M5의 mixed placement와 M6 GPU·async 실구현은 M4의 선행 필수 조건이 아니다.

**상태 (2026-10-07):** 본 절의 **조건·검증 프로토콜만 문서 확정**. 모든 새로운 실행/성능 수용은 아직 미검증이며, M4/HE-01~09 미완료 체크박스를 유지한다. 구현 재개 시 각 수용 결과를 기존 §17/HE 체크리스트에 증거 링크로 기록한다.

**연계 체크리스트:** [sw-MLPL 실행·검증 참고 SW-03/04/06](./03-architecture-p02.ko.md#sw-mlpl-reference-checklist)는 위 M4 수용 게이트의 보조 구현 패턴이며, 이 문서 추가만으로 체크박스를 닫지 않는다.

<a id="기존-아키텍처-대조-감사-m4-준비도와-검증-공백-2026-10-07-설계-검증-전용"></a>
##### 기존 아키텍처 대조 감사: M4 준비도와 검증 공백 (2026-10-07, 설계 검증 전용)

**결론:** J Graph → verified A3 → target-dependent lowering 후보 → Physical Representation이라는 기존 계층은 **그대로 확장 가능**하다. CPU 전용 Parallel Planner/IR, 새로운 Data Movement IR, GPU 실구현 선행은 필요하지 않다. 단, **A3 구조 검증**, **후보별 적법성 평가**, **원본 A3에서 Physical Plan까지 의미 보존의 증명**은 서로 다르다. 앞 단계의 성공이 뒤 단계의 완전한 실행 허가는 아니다. 아래는 현재 main의 실제 코드와 기존 테스트를 대조한 **정적 감사**이며 신규 테스트를 실행한 것이 아니다.

| 감사 ID / 소유자 | 실제 코드·테스트에서 확인 | 공백과 단계 구분 | 추후 검증 조건 |
|---|---|---|---|
| CA-01 / A3 구조·의미 | src/logical_ir.rs의 Plan::verify는 schema·registry provenance, single block, SSA producer/use-before-def, order_after가 앞선 op를 가리키는지, Check가 result를 만들지 않는지, 일부 rank/shape를 검증한다. tests/logical_ir.rs에 관련 회귀가 있다 | **M3→M4의 의미론 증명 미완료.** A3 verifier는 모든 effect/error summary를 J 원본에서 독립 재계산하거나, 삭제된 Check가 없는지 Physical Plan까지 대조하지 않는다. IterationAxisKind::Parallel은 실행 허가가 아니다 | 정상 A3 하나를 복제하여 Check 삭제·중복, order_after 삭제, effect/possible_errors/speculation의 근거 없는 완화, source origin·binding version 위조를 각각 주입한다. 구조 위조와 **구조는 통과할 수 있는 의미론 위조**를 구분하고, 후속 선택·검증에서 모두 차단 |
| CA-02 / Lowering | src/lowering.rs의 Requirement::satisfied·legal_candidates는 CPU/GPU family·feature, Pure, NoObservableError, EvaluationOrderRelaxed, known access/rank, 재결합 허용을 판정한다. tests/lowering.rs는 안전 조건이 없으면 SIMD/GPU·tree reduce를 닫는 검사를 포함 | **M3 증거 연결 미완료.** 후보 등록·CallOp에 적힌 사실이 곧 guard discharge/전체 Physical Plan의 적법성 인증서는 아니다. ReferenceSequential 후보는 실제 native CPU kernel 완성을 의미하지 않는다 | 근거 없는 effect/error/guard/witness 완화가 최종 선택에서 거부되는지 확인. target·valence·shape·stride·recipe와 proven semantic facts의 교차 검증이 필요 |
| CA-03 / RoutePartition | lowering.rs의 partition_plan은 ValueOnly/PureArray/SemanticCheck/RuntimeSemantic가 같은 연속된 op를 구간으로 묶는다 | **RouteBoundary verifier 미구현(M3→M4).** 구간 class/range만으로 source op 전수 대응, live-in/out, zero-result Check, effect-live/order edge, guard-before-effect, commit frontier, replay 안전성은 증명되지 않는다 | source A3 모든 op의 정확한 대응(적법한 rewrite/fusion에는 witness 필요), Check·effect·name version·order edge 보존, cross-region consumer readiness를 검증. Check 유실, 순서 반전, dangling live-in, stale version, effect 이후 replay 위조 시 거부 |
| CA-04 / Physical Representation | src/physical.rs에는 registry/slot/generation BufferId, read-only BufferLease, checked span을 가진 PhysicalArray가 있다. tests/physical.rs는 lease 생존, stale ID, alias, 음수 stride, transpose, zero/singleton axis를 검증 | **M4 구현 미완료.** main에는 plan-time PlanBufferId/PhysicalViewId, mutable output ownership, last-use/reuse proof, PhysicalPlan verifier, memory residency/ready/transfer plan이 없다. 현재 BufferId를 compiler slot으로 재사용하지 않는다 | buffer/view span overflow, 다른 ID 사이 alias, stale generation, overlapping mutable outputs, dangling Return, temporary early free, OOM cleanup과 reuse-after-free를 각각 독립 negative test로 검증. 쓰기·재사용은 증명 전 비활성 |
| CA-05 / Rank·오류 | src/facts.rs의 RankFrameExecution은 ZeroFrameNeedsFill과 CellsPresent를 구분한다. logical_executor::execute_closed는 Check→Call의 closed-noun 순차 reference다 | **M4/M5 의미론 조건.** reference는 동적 NAME/state에 대한 전체 oracle가 아니다. 일반 CellApply를 uniform map으로 가정하거나 first-error 순서를 worker 완료 순서로 바꾸면 오류 | zero-frame 가상 fill cell, positive-frame empty cell, result-cell dtype/shape join, boxed/sparse, tolerance/fit, overflow·numeric reassociation, first J error, try/catch/effect를 C jsource+RustJ baseline과 비교. 불명확한 연산은 순차 유지 |
| CA-06 / Device·memory·cost | TargetCapabilities는 CPU/GPU family와 feature를 제공한다. j_graph_work_depth.rs는 successful-path symbolic work/depth, j_graph_resource.rs는 logical resource 정보를 계산한다 | **M5 이후 구현 미완료.** 이 타입만으로 device placement ≠ memory placement, residency/BufferVersion, transfer/sync/timepoint, critical path/실측 비용을 표현·검증했다고 말할 수 없다 | 첫 Physical Plan은 단일 CPU/Host/zero-transfer로 충분. 이후 ready-before-read, copy-after-producer, transfer-before-free, stale version, memory capacity, cost Unknown!=0 및 실제 overlap 증명을 negative test로 확인 |

**독립 교차 감사(두 경로, 정적 코드 검토):**

- 정방향: logical_ir.rs → lowering.rs → physical.rs → facts.rs → j_graph_work_depth.rs, j_graph_resource.rs의 현재 API와 각 단계가 *실제로 보유하는 정보*를 대조했다.
- 역방향: tests/logical_ir.rs, tests/lowering.rs, tests/physical.rs, logical_executor.rs가 이미 통과하도록 설계된 범위와 아직 부정 검증을 제공하지 못하는 *단계 간 관계*를 대조했다.
- 두 검토의 일치된 결론은 **대규모 아키텍처 재설계 불필요, RouteBoundary 전체 coverage / guard discharge / PhysicalPlan verifier는 미완료**다. 기존 코드 테스트를 새로 실행하거나 통과 확인했다는 주장이 아니다.

**지금 확정할 계약(실제 구현·새 IR 도입은 보류):**

1. **A3/M3:** verified A3의 구조적 유효성, optimizer가 사용할 semantic witness/guard의 충족, selected physical plan의 전체 적법성을 세 단계로 분리한다. A3에 device/thread/memory layout을 넣지 않는다.
2. **M3→M4 Route 경계:** A3 source op 전수 매핑, live-in/out, Check/effect/order/late-NAME, guard-before-effect와 commit/replay frontier를 별도의 *검증 결과(sidecar 또는 동등한 기존 route 계약)*로 소유한다. 해당 표현의 Rust 타입/새 계층은 현재 고정하지 않는다.
3. **M4 Physical:** PlanBufferId와 런타임 BufferId, PhysicalViewId를 분리한다. view/span·buffer ownership/lease/last-use, selected realization, Check coverage, Return을 Physical verifier가 확인해야 한다. ExecutionDevice / MemorySpace / device-local schedule은 서로 다른 결정이다. M4는 CPU/Host/Sequential만 실구현해도 된다.
4. **M5 이후:** ResourceEstimate, predicted CostEstimate, measured CostProfile, 성공 경로의 work/depth 및 오류/효과 경로를 분리한다. device-memory migration/async overlap은 명시적 의존성·실장치 증명 후에만 비용 선택에 반영한다.

**결정 및 우선순위:** (1) M3의 원본 A3→Route Check/effect/witness 증거 의무 확정 → (2) M4의 CPU/Host/zero-transfer PhysicalPlan verifier·실제 native Add 경로 → (3) M5 배치·자원·비용 후보 → (4) M6+ 별도 재개된 GPU/비동기 실행. 설계 자체의 구조적 모순은 **발견되지 않았음**; 그러나 위 증거가 없을 때의 fail-closed 실행 경로는 반드시 필요하다. 기존 M2→M3→M4 우선순위, HE-01~09의 미완료 상태, 별도 CUDA 보류는 유지한다.

<a id="m3m4-route-to-physical-인계-계약-2026-10-07-설계만-완료--구현-없음"></a>
##### M3→M4 Route-to-Physical 인계 계약 (2026-10-07, **설계만 완료 / 구현 없음**)

**목적:** 위 §2.5.1의 `M3-RB/RB-01~08`과 이 §5.2.1의 `PhysicalPlan` verifier를 하나의 **증거 체인**으로 연결한다. M3가 J 의미의 적법성을 판단하고, M4가 **이미 허가받은 의미를 특정 buffer/view/kernel 순서로 실현해도 안전한지** 검증한다. M4가 누락된 Check/guard, Rank·오류 순서, name/version 증거를 새로 추측하여 M3 판정을 뒤집어서는 안 된다. M3의 합격이 M4의 실제 실행 성공을 보장하지도 않는다.

**입력·출력·소유권 표(새 필수 IR이나 Rust 자료형을 확정하지 않는 개념 계약):**

| 인계 항목 | M3가 제공·보증할 것 | M4가 독립적으로 검증·구체화할 것 | 불충분한 경우 |
|---|---|---|---|
| **H-01 원본 기준** | immutable source A3 identity, schema/primitive-registry, source op/`j_origin`, version 및 정당한 rewrite/fusion mapping | 받은 계획이 **같은 원본**에서 나온 것인지, selected physical op가 허용된 mapping에 매달리는지 대조 | stale/바뀐 증거를 사용하지 않고 plan reject |
| **H-02 Route 확정** | region 전체 coverage, 각 region 선택 route 및 CPU/GPU/external target capability와 선정 이유; 단순 `legal_candidates` 목록과 다름 | 각 `Kernel`/`View`/`Materialize`가 선택된 recipe·target·region에 속하고 *진짜 구현된* realization인지 검사 | 미구현 native kernel/외부 route를 성공으로 위장하지 않고 다른 합법 route나 Unsupported |
| **H-03 값 경계** | `ValueId`의 producer, region live-in/live-out, `Plan.result`, dtype/shape/rank·J-visible boxed/sparse와 value snapshot identity | plan-time `PlanBufferId`와 `PhysicalViewId` 바인딩, physical encoding·extent/stride/span, 최종 Return의 논리 dtype/shape/order와 input/output ownership 검증 | missing value, 모순된 backing/encoding/shape, dangling return reject |
| **H-04 Check 의무** | 원본의 zero-result `SemanticCheck`마다 실행·검증된 증거에 의한 discharge·동등 guard 대체 **중 정확히 하나**, J error kind와 original order | 반드시 실행할 Check를 `PhysicalOp::Check` 또는 의미 보존 equivalent에 매핑. **discharged Check를 임의 재삽입하거나 누락된 Check를 추론하지 않음** | coverage/증거 불일치, duplicate/late Check, J error priority 변경 reject |
| **H-05 효과·이름·Write** | `order_after` 및 effect/error edges, noun read snapshot vs late function NameRef, `Plan.write`(별도 commit), guard-before-effect 및 replay 경계 | 명시된 순서로 물리 Check/Kernel/Return/commit handoff. 첫 M4 pure CPU slice에서는 stateful commit을 **실행기 소유로 만들지 않음** | 증명 안 된 NAME/Write는 RuntimeSemantic 경로; effect 이후 재실행 금지 |
| **H-06 Guard/실행 준비** | 각 witness 의존 입력 버전과 `GuardRequired` 의무, 평가 시점, 실패 시 합법한 fallback 및 commit frontier | 물리적 plan에서 guard가 해당 연산·효과보다 **먼저** 배치되고 그 실행·결과·binding freshness가 보장되었는지 검사 | `GuardRequired`를 `Verified/Ready`로 자동 승격 금지, 가드 없는 실행 불가 |
| **H-07 데이터 의존성** | SSA def→use, region cross-value dependencies, Check/effect-first-error precedence와 effect-live edges | BindInput/alloc → Check → View/Materialize/Kernel → Return의 실제 use-before-def, buffer readiness, lease/last-use, overlap/reuse permission. 비동기면 completion edge 추가 | 순차 계획이라도 부적절한 재배열·조기 free·중복 쓰기 reject |
| **H-08 표현·장치** | J-visible value type/shape/rank/representation 및 허용 target의 **제약**만 제공; source A3에는 device pointer/stride/physical memory 없음 | 실행 장치, 메모리 공간, device-local schedule과 buffer residency를 **각각** 선택. M4 = CPU/Host/Sequential/zero-transfer, M5+에서 transfer/ready/sync 확장 | GPU·외부 backend 또는 residency가 없으면 CPU/합법 fallback, 무근거 transfer 가정 금지 |
| **H-09 비용·진단** | semantics/legality 결과와 선택 제약; cost 추정이 proof를 대신하지 않음 | physical feasibility/budget/peak bytes, predicted cost 및 선택 근거를 별도 기록, 실패 유형을 source op/region/plan buffer와 연결 | cost Unknown을 무료/실행 가능으로 취급하지 않고 선택 보류 |

**승인 단계와 책임의 단방향성:**

~~~text
원본 source A3 (Plan::verify 성공 / 독립 의미론 권위)
  → M3 Route candidate (partition_plan은 아직 후보 분석)
  → M3 RouteVerified + [원본 연산/Check/Write/순서/guard/bridge 증거]
      ├─ Rejected: 증거 없음 → 다른 합법 route / Unsupported
      ├─ GuardRequired: guard와 안전한 실패 경로가 먼저 충족될 때까지 실행 불가
      └─ Verified: 해당 region의 의미론·Route 적법성만 통과
  → M4 Physical candidate (선택 recipe / buffer / view / ordered ops)
  → M4 PhysicalVerified (타깃·storage·순서·resource·coverage 검증)
  → RuntimeReady (동적 guard, input versions, 실제 lease, capacity 재확인)
  → 실행 / 오류 / 자원 정리
~~~

`RouteVerified`, `PhysicalVerified`, `RuntimeReady`는 **개념적 승인 상태**이며 신규 enum/IR 구현 지시가 아니다. 정적으로 `Verified`여도 입력 name/version, guard, buffer lease 등이 실행 시 바뀌면 실행 전 재검증이 필요하다. 정당한 RuntimeSemantic/외부 route는 RustJ-native M4 physical executor를 **반드시** 거칠 필요가 없지만 동일한 경계 의미론 의무를 지켜야 한다.

**검증기 책임 분할(중복 검증과 허위 위임 방지):**

- **M3 Route 검증기:** 원본 A3와 후보 매핑, check coverage/discharge, 순서·오류·effect·Write/NAME, target semantics, guard ownership 및 representation-neutral bridge의 *합법성*을 인증한다. Physical stride·allocation 위치는 결정하지 않는다.
- **M4 Physical verifier:** M3의 서명 없는 제안/단순 class를 실행권으로 간주하지 않는다. 승인된 source mapping·Check/guard/order obligations와 선택된 physical ops의 **정확한 연결**을 재대조하고, selected recipe·resolved CPU capability, checked affine bounds, buffer/version/ownership/lease, output ownership, temporal safety와 resource feasibility를 확인한다. J semantic proof를 스스로 만들어내지 않는다.
- **Runtime admission/Executor:** 실제 name/input snapshot/guard outcome, lease generation/readiness, 장치 가능 여부를 **사용 시점**에 확인한다. verifier를 통과하지 않은 계획 실행, 관찰된 효과 뒤의 transparent replay, backend 내부 오류를 근거 없이 J Domain/Rank/Length로 바꾸기는 금지한다.
- **Cross-route 책임:** 첫 native CPU slice 밖의 runtime/external 부분을 하나의 PhysicalPlan으로 강제로 합칠 필요는 없다. 다만 각 boundary handoff의 value/version/check/error/effect 계약은 통합 검증 기록으로 추적한다.


<a id="h-k--a3-opkind별-m3m4-인계-규칙-2026-10-07-설계만"></a>
##### H-K — A3 OpKind별 M3→M4 인계 규칙 (2026-10-07; 설계만)

**구체적 공백:** 현행 `lowering.rs::route_operation`은 `Literal`, `ReadNoun`, `VerbReference`를 모두 `NoKernel`로, `partition_plan`은 그 연속 구간을 `RouteRegionClass::ValueOnly`로 합친다. 이 분류는 **실행 후보 grouping**이지 세 OpKind가 동일한 버퍼·호출·이름 의미를 가진다는 뜻이 아니다. M4 승인 여부는 region class뿐 아니라 원본 A3 **OpKind마다** 다르게 확인해야 한다.

| 원본 A3 대상 | M3가 책임지는 증거 | M4가 할 수 있는 일 / 금지되는 일 |
|---|---|---|
| `Literal(value)` | 원본 payload, dtype/shape, producer OpId와 result ValueId, source origin | 지원하는 dense CPU literal은 읽기 전용 입력 버퍼로 바인딩 가능. 이를 항상 deep copy할 필요는 없고, 타입·Shape를 임의 변경할 수 없음 |
| `ReadNoun {symbol,version}` | 원래 J read 시점과 noun snapshot, namespace/scope/버전, observable ordering | `ValueOnly`여도 literal로 간주하거나 나중 값으로 바꿔 읽지 않는다. 런타임 snapshot/guard가 없으면 합법한 RuntimeSemantic 경로에 남긴다 |
| `VerbReference(Callable)` | FunctionEntity, function POS, primitive/derived callable identity와 동적 NAME lookup | J 함수 참조는 noun의 dense 배열 버퍼가 아니다. `NoKernel`만 보고 `BindInput`용 값이나 kernel operand로 취급 금지; 별도의 function semantics 지원 필요 |
| `Basis { kind,payload,call }` | 원본 primitive/derived target·valence·Rank/CellApply·fact/error/effect, selected recipe 및 Check/guard 의무 | M3가 승인한 선택 target에 해당 recipe/실제 native 구현이 **존재할 때만** Kernel/View 계획 가능. `legal_candidates`나 `ReferenceSequential`만으로 native 준비 완료 주장 금지 |
| `SemanticCall(call)` | 정규화되지 않은 호출 의미, dynamic binding, 효과/오류 | 현행은 RuntimeSemanticFallback. 별도의 동등성·지원 증거 없이 임의로 native Basis/Kernel로 승격 금지 |
| `SemanticCheck(check)` | producer/liveness, zero-result constraint, 원본 error kind/span/order 및 실행·증명된 discharge·동등 guard 중 하나의 소유자 | output SSA가 없더라도 제거 금지. 실행 의무가 있으면 Check를 원래 순서에 반영; 증명된 discharge만 무실행 허용 |
| `Plan.result / Return` | 마지막 result ValueId, logical type/shape/atom order, cross-region producer | 유효한 output ownership/lease로 Return. Return이 Write commit 완료라는 뜻은 아님 |
| `Plan.write` (operations 밖) | symbol/value/previous/proposed/span/after와 commit/effect owner | 모든 op range를 포함해도 Write는 자동 포함되지 않는다. 초기 M4 pure slice가 assignment를 commit하거나 Write를 무시·반복 실행하는 일 금지 |

**전역·국소 검증의 구분:** RB-01은 원본 A3 **전체**의 모든 operation과 별도의 Write 사건에 적용된다. 하나의 RustJ-native PhysicalPlan은 이 가운데 **M3가 승인한 native region**만 담당할 수 있다. 전역 coverage는 M3가, 각 native region의 선택 recipe/Buffer·Check·Return coverage는 M4가 책임진다. 다른 RuntimeSemantic/External region은 억지로 동일한 PhysicalPlan에 집어넣지 말고 경계 ValueId·ordering·effect와 연결한다.

**인계 시 세 번 확인할 사실(새 필수 IR이나 데이터 계층 추가 없음):**

1. **M3 RouteVerified:** immutable source A3의 원래 OpKind/Check/Write/ValueId/order와 region의 실제 연산이 일치하는가? GuardRequired는 실행 허가가 아니며 guard 위치·실패 경로를 확정해야 한다.
2. **M4 PhysicalVerified:** 승인된 native op 각각에 실제 target-compatible recipe 및 물리 Bind/Check/View/Kernel/Materialize/Return 대응이 있는가? `PlanBufferId ≠ PhysicalViewId ≠ runtime BufferId`, affine span, lease, encoding, ownership/last-use가 증명되는가? M4가 M3의 의미 증거를 새로 만들어내지는 않는다.
3. **RuntimeReady:** 실제 read/name snapshot, input version, guard outcome, lease generation 및 capacity가 **사용 시점**에도 유효한가? 실패하면 observable effect commit **이전**의 안전한 fallback만 허용하고, effect 이후 자동 replay 금지.

**부정 검증 계획 (H-KV1~5; 실행 테스트 미작성):**

- **H-KV1:** `1+2`에서 Literal/Kernel input 대응을 유지해야 함. `VerbReference`를 `ValueOnly`라는 이유로 dense `BindInput`으로 취급하거나 literal producer를 바꾼 경우 거부.
- **H-KV2:** `a`의 snapshot/version/read 시점, `a=:1+2`의 별도 Write/after를 한 가지만 변형해 거부. 결과 Value만 맞고 Write가 유실돼도 거부.
- **H-KV3:** `1 2+1 2 3`의 zero-result Length Check는 Call 전에 관찰되어야 함. Check를 버리거나 중복·후행 배치한 physical 계획 거부.
- **H-KV4:** `future 3`의 SemanticCall을 새 의미론 증거 없이 native 실행으로 가장하거나 `ReferenceSequential` 후보만으로 native kernel 성공 주장 시 거부.
- **H-KV5:** 같은 원본 A3가 CPU-native와 RuntimeSemantic으로 분할될 때 M3 global coverage와 M4 native-local coverage를 각각 검사; 중간 live-out, 첫 오류, effect-live, final Return 손실 거부.

**상태:** H-K는 기존 H-01~09/HM-V0~V4를 OpKind별로 구체화한 **설계 수용 보조 규칙**이다. 별도 canonical IR, CPU Parallel 레이어나 runtime 구현을 요구하지 않는다. 실제 validator/negative test/차분 결과가 없는 동안 M3/M4/HE-01 완료 항목은 열린 상태로 유지한다.

**검증 표본(각 정상이 확인된 후 불변조건 1개만 위조하는 계획):**

| 사례 | 정상 인계 증거 | 거부·실패로 판정할 변형 |
|---|---|---|
| `1+2` | 원본 SSA value/literal → 선택 CPU Elementwise route → 별도 verified physical input/Return mapping | M3에 `ReferenceSequential` 후보만 있는데 M4가 구현되지 않은 native Add를 실행 가능이라고 승인; ValueId/PlanBufferId를 혼동 |
| `1 2+1 2 3` | zero-result Length Check의 원본 provenance/order를 M3가 전달, M4가 Check→Kernel 이전 실행을 보장 | M3 Check는 승인했으나 M4 plan에서 삭제·중복·Call 뒤 배치; J Length 오류를 구현 오류로 바꿈 |
| `1 2+3 4` | 입력 shape에 맞는 PrefixAgreement witness가 원본에 고정돼 있다면 합법적 Check discharge | input shape/NAME version 변경 후 과거 witness로 PhysicalVerified; 증거 없는 discharge를 M4가 자체 승인 |
| `3 { 10 20 30` | Index Check 원래 오류 종류·우선순위와 물리 검사의 대응 | Check를 M4 materialize/Kernel 뒤로 옮기거나 Index 대신 Length로 변경 |
| `a` 및 `a=:1+2` | name snapshot과 별도 Write event가 M3에 남고, 지원되지 않는 stateful 부분은 RuntimeSemantic 소유 | Return만 물리화하며 Write를 drop, speculative name read, committed assignment 이후 fallback replay |
| 2D reverse/transpose / empty frame | 의미상 Shape/atom order와 zero-frame fill 의무를 M3가 소유, M4는 signed span/stride/encoding·복사 순서 확인 | 음수 stride OOB, zero-frame CellApply를 아무 계산 없이 결과형 가정, incompatible cell-result join |
| 같은 source의 CPU/GPU/External 후보 | 공통 A3/guard/error 계약, 배치·storage contract는 route별로 독립 | CPU-only 테스트로 GPU/transfer Ready 주장, 준비되지 않은 메모리에서 read, 미승인 overlap/async error reorder |

**인계 수용 테스트 순서 (테스트 작성·실행은 보류):**

1. **HM-V0 정적 인터페이스:** M3-RB RB-V0~V2 승인 증거의 필드/원본 binding을 M4 입력 계약과 하나씩 대조한다. 누락·중복·stale op/Check/Write/guard, 다른 source의 승인 보고서 재사용을 차단한다.
2. **HM-V1 타입·자원:** CPU/Host/no-transfer M4 후보의 `ValueId ↔ PlanBufferId ↔ PhysicalViewId ↔ BufferLease` 관계, affine span·encoding·ownership·last-use·output validity·OOM cleanup을 정적/실행 시점으로 나누어 확인한다.
3. **HM-V2 순서·오류:** zero-result Check, observable effect/order, guarded fallback/commit과 첫 J 오류를 M3→M4→Runtime 세 지점에서 교차 검증한다. guard failure는 실행 전 대체 경로로만 넘긴다.
4. **HM-V3 3자 차분:** 동일 원본 A3를 기준으로 jsource C(사용 가능할 때), RustJ semantic/reference 경로, native Physical 경로의 값/dtype/Shape/atom order/J error class/우선순위/관찰 state를 비교한다. C 또는 native 경로가 없다면 **미검증**, 대체 통과 처리하지 않는다.
5. **HM-V4 단계 승인:** M3 `RouteVerified` 증거가 있더라도 별도 `PhysicalVerified`·`RuntimeReady`가 없으면 미승인. M4 완료는 구현된 native Add/Check 등을 통한 E2E와 negative tests가 실제 통과한 후에만; GPU/async/transfer는 M5·M6+ 별도 검증.

**정리:** 별도 새 compiler layer는 필요하지 않다. ***M3는 의미론적 적법성의 증거를 소유하고 M4는 그 증거를 변경하지 않고 물리적으로 실현했는지 검증한다.*** 이 명세만으로 M3 검증기/M4 Physical Executor가 만들어졌거나 테스트가 PASS한 것은 아니다. §10의 M3-RB·M4 및 HE-01~09는 **설계 완료와 구현/검증 미완료를 계속 분리**한다.

<a id="h-a--m3m4-첫-cpu-경로-실행-승인-판정표-2026-10-07-명세만"></a>
##### H-A — M3→M4 첫 CPU 경로 실행 승인 판정표 (2026-10-07; 명세만)

**이 절의 역할:** 기존 H-01~09, H-K, HM-V0~V4의 반복이 아니라, 실제 `lowering.rs`의 `RouteDecision`/`RouteRegionClass`와 미래 M4 compiler-native CPU 경로 사이에 **실행 가능한 것과 단지 합법적 후보인 것**의 경계를 확정한다. 현재 `main`에는 `src/physical_plan.rs`가 없으며, `src/physical.rs`는 read-only 표현 기반이다. Draft PR #4의 제한적인 literal identity 코드를 완료된 native Add 실행기로 계산하지 않는다. 구현·테스트 추가 없음.

**기본 승인 흐름(기존 단계의 판정 질문):**

| 순서 | 확인 대상 | 승인하려면 | 증거 부족 시 |
|---|---|---|---|
| **A0: SourceA3** | 변환 전 원본 `logical_ir::Plan` | `verify()`, source/provenance와 original Check/Write/오류 의무를 고정하고 변환 후보와 독립 보관 | 손상된 원본/다른 registry → 거부 |
| **A1: RouteVerified** | `partition_plan`이 분류한 모든 원본 op와 선택할 *특정* region | RB-01~08 coverage, source op/operand/live-in/out, Check 실행·proof discharge·guard 소유, Write/Name/error-order 및 region-wide capability 증거 | `NoKernel`·`NativeExecutionBasis`·`RuntimeSemanticFallback`이라는 분류 자체는 허가 아님 |
| **A2: ExecutableRoute** | M4가 인수하는 native region | M3가 승인한 **구체적인** realization과 **실제 존재하는 CPU kernel/adapter**, dtype/rank/valence 및 guard 지원; `ReferenceSequential`라는 후보 이름만으로 native 구현을 추정하지 않음 | 유효한 semantic/reference route만 사용하거나 Unsupported. *native 실행 가능*으로 표시 금지 |
| **A3: PhysicalVerified** | 물리 연산·버퍼·View·Check·Return | 허가된 region/source mapping과 1:1 또는 등가 증명 대응. BindInput/Check/Kernel/Materialize 순서, PlanBufferId≠BufferId, span/encoding/ownership/lease/last-use, CPU Host capacity 검증 | invalid physical plan은 실행 직전에 임의 보정하거나 J 오류로 바꾸지 않고 거부 |
| **A4: RuntimeReady** | 실제 호출 시점 | input/name snapshot 버전 및 guard 판정, registry lease/generation, capacity·readiness 유효. effect 이전 대체 경로 존재 | stale/guard failure는 **effect 이전에만** 안전한 경로로 변경, commit 이후 자동 재실행 금지 |
| **A5: AcceptedNativeE2E** | 출력과 관찰 결과 | 실제 compiler-native 결과·dtype·Shape·atom order·첫 J error/effect가 RustJ 순차 및 가능할 때 jsource 기준과 일치, 관련 부정 사례 테스트 통과 | A3 분석 통과, reference route 결과, 문서·테스트 소스 존재는 M4 완료 증거가 아님 |

**같은 A3에서 계획을 구성하는 구체 추적 표본**

~~~text
J: 1+2
  Source A3: Literal(1) → ValueId(0)
             Literal(2) → ValueId(1)
             Basis(Elementwise, Add) → ValueId(2), result=ValueId(2)
  Route candidate: ValueOnly(리터럴 구간) → PureArray(Add 구간)
  M3: 원본 Add, value 연결, PrefixAgreement witness/guard, Check 의무를 대조
  현행 CPU Lowering: Elementwise의 ReferenceSequential 후보 존재
  M4 실행 판정: 실제 compiler-native Add 구현/물리 검증/실행 결과 전에는 A2~A5 미승인

J: 1 2+1 2 3
  Source A3: 두 Literal → zero-result SemanticCheck(Length) → Basis(Add)
  Route candidate: ValueOnly → SemanticCheck → PureArray
  M3: Length Check와 Check→Call ordering, operand live-in을 전수 인계
  M4: Check 실행 또는 독립 증명된 동등 대체 없이 Kernel 선실행 불가
  기대 관찰: Length error가 원래 시점에 보고되어야 함. 수행되지 않은 물리 경로는 미검증

J: a=:1+2
  Source A3: 계산 op + 별도 Plan.write {symbol,value,previous,proposed,after}
  M3: 계산 결과와 Write commit을 서로 다른 의무로 인계
  M4: 순수 Add region만 인수 가능. 반환값 생성이 assignment commit을 뜻하지 않음
  전체 문장 완료: 책임이 분명한 RuntimeSemantic/향후 effect owner 필요
~~~

**지원 상태 판정표:** 현재 코드에 `ReferenceSequential`, `GenericCellLoop`, `MetadataOrIndexReindex` 등 *등록된 후보*가 있다고 해서 CPU Physical Executor가 구현된 것이 아니다. `RouteDecision::NoKernel`은 literal/read/verb-ref가 **같은 종류의 값**이라는 뜻이 아니다(H-K 참고). `TargetCapabilities::gpu_generic()`의 **기능 묘사**는 실제 GPU 장치·런타임 가용성 또는 실행 허가가 아니다. M4-v0 정책은 CPU/Host/Sequential/zero-transfer만 선택하도록 제한하되, 그 제한을 상위 J 언어 의미론/Logical IR에 역류시키지 않는다.

**검증 스펙 HA-V1~HA-V7 (실행은 보류):**

| 테스트 ID | 정상 승인 조건 | 한 항목씩 바꾸어 확인할 거부 조건 |
|---|---|---|
| HA-V1 **A3→Route** | `1+2`의 op/region 커버리지, 정확한 result/operands, source origin | native Add op 유실, 두 region overlap, 타 A3 origin을 같은 것으로 가장 |
| HA-V2 **후보≠커널** | `ReferenceSequential` 후보를 그대로 분석 후보로 표시하고 실제 native 구현이 존재할 때만 A2 승격 | 등록 후보만으로 PhysicalVerified/RuntimeReady/NativeE2E 선언 |
| HA-V3 **Check→Kernel** | `1 2+1 2 3`의 원본 Length Check를 선행, `3 { 10 20 30`의 Index Check를 선행; first error class 보존 | Check 삭제/중복/후행 배치, Length↔Index 교체, Kernel-first schedule |
| HA-V4 **witness/guard** | `1 2+3 4`처럼 원래 shape witness가 유효하면 Check 없이 가능; guard 필요 시 효과 전에 완료 | 바뀐 input shape/name version에 과거 증명 재활용, GuardRequired를 실행 승인으로 오인 |
| HA-V5 **Name/Write** | `a`는 read snapshot 검증, `a=:1+2`는 별도 commit owner 확인 | `ReadNoun`을 상수 literal로 변경, late function lookup 조기 고정, `Plan.write` 삭제/중복·effect 후 replay |
| HA-V6 **Storage/Return** | 음수 stride/transpose/빈 Shape의 논리 순서·encoding·span, output ownership/lease generation | `ValueId==PlanBufferId` 간주, 미등록/조기 해제 버퍼, overlapping writable view, 잘못된 atom order·dangling Return |
| HA-V7 **cross-route** | native 부분과 RuntimeSemantic 부분의 M3 global coverage + M4 native local coverage, 오류/effect edge 보존 | native region만 정상이라 전체 표현식 승인, 외부 bridge 미구현 상태에서 M4 전체 실행 성공 주장, GPU 완료 허위 기록 |

**구현 우선순위와 수용 상태:** (1) 기존 M3-RB validator와 독립 source A3 기준의 HA-V1/3/4/5 논리 검증 → (2) M4 선택 recipe의 *실제* 구현 여부·CPU buffer/Check/Return verifier(HA-V2/6) → (3) 독립 reference/jsource의 HA-V7 및 native E2E 수용 → (4) M5/M6의 device/memory/transfer 비용·비동기 검증. **이번에 완료한 것은 이 판정표와 테스트 설계뿐이며**, 현행 코드의 M3/M4/HE-01 실행 승인 체크박스는 그대로 열어 둔다.


