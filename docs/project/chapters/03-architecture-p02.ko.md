[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)

<a id="242-j-semantics-보존과-compiler-fact-축적-원칙"></a>
### 2.4.2 J semantics 보존과 compiler fact 축적 원칙

RustJ의 parser와 J Semantic IR은 **J language semantics를 보존하는 canonical layer**다. 이후 compiler 단계에서 필요한 정보를 얻기 위해 parser-produced entity를 target/optimization 친화적인 형태로 변형하지 않는다.

기본 원칙:

```text
J source
  ↓
jsource-compatible parsing / semantic construction
  ↓
J Semantic IR                         // J meaning is authoritative here
  │
  └─ FunctionEntity
       ├─ identity / POS / operands / span
       └─ semantic_info               // intrinsic, recursively reusable J semantics
            ├─ construction facts
            ├─ valence/rank contract
            ├─ effect/error summary
            ├─ binding/self dependency
            ├─ atomicity/latent policy
            └─ other target-independent intrinsic semantics
        ↓
Logical IR Node
  ├─ origin: FunctionEntityRef
  └─ resolved_facts                   // actual call/argument dependent
       ├─ type/shape
       ├─ effective rank / CellApply
       ├─ agreement/repetition
       ├─ access/effect/error requirements
       └─ constraints
        ↓
Optimizer / Route / Schedule / Physical Plan
```

즉 **J semantic graph는 원본 의미의 기준**이고, 각 stage의 node가 그 stage에서 안정적인 정보를 직접 소유한다. side table은 비싸거나 재계산 가능한 보조 분석의 cache로만 사용하며, 재귀적 semantic 분석에 필요한 intrinsic 정보의 주 저장소로 삼지 않는다.

정보를 다음 수명으로 분리한다.

1. **SourceSemanticIdentity**
   - parser result POS
   - primitive/derived identity
   - Hook/Fork construction
   - modifier semantic operands
   - name/binding semantics
   - source span/provenance

2. **FunctionSemanticInfo**
   - 각 `FunctionEntity`가 직접 소유하는 immutable intrinsic semantic information
   - modifier construction-time facts
   - valence/rank contract
   - effect/name/self/error/atomicity/latent policy summary
   - derived entity를 만들 때 이미 완성된 child node의 `semantic_info`를 재귀적으로 참조하여 bottom-up으로 계산
   - primitive는 `PrimitiveSpec`을 참조/요약하고, named/late-bound entity는 필요한 항목을 `Unknown/May...` 상태로 보존
   - target-independent

3. **ResolvedCallFacts**
   - 실제 valence와 argument facts가 있어야 결정되는 정보
   - effective rank
   - frame/cell split
   - prefix agreement/repetition
   - result type/shape constraints
   - access/effect/error requirements

4. **OptimizationFacts**
   - 의미를 바꾸지 않고 다시 계산 가능한 proof/candidate
   - uniform cell result
   - fusion candidate
   - CellApply absorption/fusion proof
   - materialization-elision candidate

5. **LoweringCapability / Target facts**
   - backend/architecture/device별 realization 정보
   - NN extension과 built-in primitive에 동일하게 적용
   - semantic identity에 포함하지 않음

6. **PhysicalDecision**
   - schedule, tile, workgroup, layout, memory space, buffer reuse, synchronization

중요한 불변조건:

- J semantic entity를 LogicalOp으로 조기에 치환하여 원래 modifier/train 구조를 잃지 않는다.
- `+/`는 semantic layer에서 `/`가 `+`에 적용된 completed J Verb로 남고, `Reduce(Add)`는 lowering 결과다.
- `u"r`는 semantic layer에서 `"` conjunction application으로 남고, effective rank/CellApply는 call analysis 결과다.
- `(+/ % #)`는 semantic layer에서 Fork graph를 유지하며, Mean fusion은 optimizer candidate/proof다.
- semantic node가 hardware target에 따라 달라지지 않는다.
- compiler fact가 필요하다는 이유로 parser에 migration boolean/target hint를 추가하지 않는다.
- 각 `FunctionEntity`를 따라 재귀적으로 내려가면 그 entity의 intrinsic J semantic contract/summary를 다시 외부 lookup 없이 참조할 수 있어야 한다.
- 반대로 target-dependent fact만으로 J semantic identity를 추정하지 않는다.

<a id="25-route-partition은-whole-program-exclusive-choice가-아니다"></a>
### 2.5 Route partition은 whole-program exclusive choice가 아니다

Route partition은 parser 직후나 J Semantic IR에서 하지 않는다. **Semantic Analyzer가 의미를 확정하고 Logical IR verifier를 통과한 뒤**, 최소 target-independent canonicalization을 거친 representation에 적용한다.

한 프로그램의 모든 op를 같은 backend/IR로 보내야 한다고 가정하지 않는다.

개념 모델:

```text
Logical Module
  ↓ capability matching / legality
RoutePartition
  ├─ Region A → MLIR
  ├─ Region B → verified library
  ├─ Region C → RustJ native
  └─ Region D → Unsupported
```

architecture 차원에서는 mixed route를 허용하지만, **첫 구현에서는 verified single-block Function/Region 전체를 하나의 route로 보낸다.** 이 단계에서는 boundary bridge가 없다.

그 다음 단계에서만 **single-block contiguous subgraph** partition과 boundary value bridge를 추가하고, 이후 Function/Region 단위의 richer partition으로 확장한다.

즉:

```text
Route-v0
  one verified single-block region
    → exactly one route
    → Lowered | Unsupported

Route-v1
  one verified region
    → contiguous subgraphs
    → mixed route + explicit boundary bridge
```

mixed-route 가능성은 architecture invariant이지만 **A1/A3-v0를 증명하기 위한 선행 구현 요구사항은 아니다.**

`RoutePartition`은 Logical IR의 semantic identity가 아니라 **compilation plan/view**다. 같은 verified Logical IR에 대해 target/backend availability나 cost model이 달라지면 다른 partition plan을 만들 수 있다. canonical Logical IR을 destructive하게 route-specific op로 덮어쓰지 않는다.

각 route region은 다음을 가진다.

```text
RouteRegion
  ops / values
  chosen route
  adapter preconditions
  required witnesses/guards
  boundary inputs/outputs
  semantic provenance
```

partition legality는 단순 op coverage가 아니라 shape/numeric constraint, effect ordering, alias/storage requirement, speculation safety를 함께 본다. effect/token/witness edge를 안전하게 보존할 수 없는 경계에서는 partition하지 않는다.

중요한 원칙:

- route partition 자체는 physical buffer/layout/transfer를 확정하지 않는다.
- 서로 다른 route 사이의 value bridge는 semantic value boundary로 먼저 표현한다.
- 실제 host/device transfer, layout conversion, buffer copy는 각 lower-level route가 필요한 representation을 정한 뒤 bridge lowering에서 구체화한다.
- adapter가 일부 op만 지원하면 지원되는 maximal legal region만 offload할 수 있다.
- unsupported region은 다른 route가 지원하면 그 route로 갈 수 있고, 아무 route도 지원하지 않으면 명시적 `Unsupported`다.

TVM BYOC처럼 external codegen 대상 subgraph를 partition하고 나머지 graph를 기본 pipeline에 남기는 구조를 참고한다. MLIR의 mixed-dialect module도 여러 abstraction/lowering state가 동시에 존재할 수 있다는 점에서 같은 방향을 지원한다.

<a id="251-routeregion-boundary-contract"></a>
#### 2.5.1 RouteRegion boundary contract

mixed route의 핵심은 “어떤 op를 어느 backend로 보낼 것인가”보다 **region 경계에서 J semantics를 어떻게 끊김 없이 보존하는가**다. RoutePartition은 physical buffer plan이 아니므로 boundary는 먼저 logical/effect contract로 표현한다.

개념적으로:

~~~text
RouteBoundary
  producer_region
  consumer_region
  live_in / live_out ValueId
  effect_order_in / effect_order_out
  semantic_checks crossing or anchored
  required witnesses / guards
  logical representation requirements
  source / J Graph provenance
  bridge-lowering responsibility
~~~

`logical representation requirements`는 `Dense`, sparse/boxed semantics, dtype/shape/rank 같은 **J-visible 또는 route-precondition 수준**의 요구를 뜻한다. 구체 `BufferId`, device pointer, stride/alignment, host↔device copy는 RoutePartition이 아니라 bridge lowering / Physical Plan이 결정한다.

<a id="boundary-legality"></a>
##### boundary legality

region 경계는 다음 조건을 모두 만족할 때만 허용한다.

1. boundary live-in/out이 모두 explicit SSA/logical value로 표현된다.
2. value가 없어도 관찰 가능한 effect/error dependency가 있으면 explicit ordering edge로 남는다.
3. `SemanticCheck`를 잘라내거나 두 route에서 중복 실행해 error precedence를 바꾸지 않는다.
4. producer와 consumer의 representation/precondition 사이에 합법적인 bridge가 존재하거나, 아직 미결이면 partition을 commit하지 않는다.
5. dynamic guard가 필요하면 guard는 region의 observable effect보다 먼저 평가된다.
6. region 전체가 선택된 route의 semantic capability를 만족한다. op별 support의 단순 교집합만으로 region legality를 선언하지 않는다.
7. external/runtime route 실패를 이유로 이미 effect를 수행한 producer region을 자동 replay하지 않는다.

<a id="live-out은-value만이-아니다"></a>
##### live-out은 value만이 아니다

다음 두 경우를 구분한다.

~~~text
value live-out
  producer result가 다음 region의 input으로 사용됨

effect live-out
  반환값은 쓰이지 않아도 write / I/O / error-order dependency가 다음 region의 관찰 결과에 영향을 줌
~~~

fork/selector에서 선택되지 않은 branch의 값이 dead여도 effect/error가 live-out일 수 있다. RoutePartition은 pure dataflow liveness만 보고 branch나 check를 제거하면 안 된다.

<a id="bridge-lowering"></a>
##### bridge lowering

RoutePartition 이후 별도 bridge lowering이 실제 representation 이동을 만든다.

~~~text
Logical ValueId
    ↓ route boundary requirement
Bridge Lowering
    ├─ no-op compatible handoff
    ├─ materialize
    ├─ layout/encoding conversion
    ├─ host ↔ device transfer
    ├─ external handle wrap/unwrap
    └─ synchronization / completion edge
    ↓
consumer route representation
~~~

bridge 비용은 route selection의 cost input이 될 수 있지만, 이미 선택한 route를 정당화하기 위해 semantic constraint를 바꾸면 안 된다.

<a id="현재-코드와-목표의-차이"></a>
##### 현재 코드와 목표의 차이

현재 `lowering.rs::partition_plan`은 operation별 `RouteDecision`을 `ValueOnly / SemanticCheck / PureArray / RuntimeSemantic` class로 나눈 뒤 **인접한 같은 class를 contiguous range로 묶는 v0 분석 도구**다.

현재 `RouteRegion`은:

~~~text
class
operations: Range<usize>
~~~

만 가진다. 아직 chosen external route, boundary live-in/out, bridge, effect edge, region-wide legality proof를 소유하지 않는다. 따라서 현재 `partition_plan` 결과를 최종 mixed-route execution plan으로 해석하지 않는다.

<a id="구현검증-순서"></a>
##### 구현/검증 순서

mixed-route 구현에 착수할 때는 다음 순서로 확장한다.

1. RouteRegion live-in/live-out 계산 + verifier
2. value live-out과 effect live-out을 분리하는 회귀
3. SemanticCheck/order edge가 boundary에서 보존되는지 검증
4. route-wide legality aggregate와 guard ownership
5. representation-neutral BridgeRequirement
6. bridge lowering에서 concrete transfer/materialization 생성
7. same Logical IR에 target별 다른 RoutePartition을 만들어 semantic result/error가 같은지 differential 검증


<a id="m3-rb--a3routeboundary-의미-보존-증명-및-검증-계약-2026-10-07-문서-설계만"></a>
##### M3-RB — A3→RouteBoundary 의미 보존 증명 및 검증 계약 (2026-10-07; 문서 설계만)

**범위:** 기존 §2.5.1 RouteBoundary 개념에 검증 가능한 승인 조건을 부여한다. 현행 `lowering.rs::partition_plan`은 `RouteRegion { class, operations: Range<usize> }`라는 **연속 구간 후보 분류기**이지 실행 승인기나 혼합장치 스케줄러가 아니다. 새 필수 Route IR, CPU Parallel IR, Data Movement IR은 만들지 않고, 추후 기존 route contract에 연결되는 검증 증거(sidecar 또는 동등한 형태)를 사용한다.

**권위 기준:** *변환 전 검증된 원본 A3 스냅샷*을 보존하여 비교한다. `Plan::verify`는 구조·schema/registry·SSA의 일부 검증기이며, 임의 수정된 `CallOp.effect`, `possible_errors`, `SpeculationSemantics`, `FactWitness`가 원래 J 의미에 참이라는 독립 인증서가 아니다. 최적화 후보가 수정한 A3를 자기 자신의 유일한 증거로 삼을 수 없다. 원본 source/primitive registry·op identity/`j_origin`/span과 생성자 증명을 연결한다. fingerprint는 stale 감지에는 유용하나 의미론 증명은 아니다.

**승인 인터페이스(개념, 새 Rust 타입 확정 아님):**

~~~text
Input:
  immutable verified source A3, proposed RouteRegion coverage
  chosen route/target, optional rewrite/fusion witnesses
  semantic facts/guards, required value/effect/bridge obligations

Output:
  source op/check/write mapping, region live-in/out, value+effect ordering
  per-region capability + discharged proofs or owned guards
  representation-neutral bridge obligations and commit/replay restrictions

Verdict:
  Verified      모든 의무를 이미 증명, 해당 route commit 가능
  GuardRequired 실행 전 올바른 guard/실패 경로 연결이 필요; 즉시 실행 허가 아님
  Rejected      위조·누락·미지원·미증명; 안전한 fallback 또는 Unsupported
~~~

**반드시 검증할 불변조건**

| ID / 소유 | 필요 증거와 검증 의무 | 거부해야 하는 반례 |
|---|---|---|
| **RB-01 op coverage** | 원본 단일 블록 `operations[0..N]`을 비중복·무간극·원래 순서의 region range로 **모두** 커버한다. 원본 OpId/span/`j_origin` 대응과 최종 result를 보존한다. Fusion/rewrite로 1:N 또는 N:1 대응이 생기면 별도의 *완전한 의미론적 등가 witness*를 요구한다 | 빠진 op, 중복·겹침 range, 역순, 유효하지 않은 ID, 원본에 없는 op provenance. N=0인 진짜 빈 A3에는 빈 partition을 허용 |
| **RB-02 value/live-in/out** | Call의 left/right, `SemanticCheck.constraint`의 입력, read/name/guard, 결과의 producer, 다른 region의 소비자, `Plan.result`까지 def-use closure를 계산한다. **Check는 출력 SSA가 없어도 값을 소비**한다 | 결과 값만 쫓아 Check 입력·cross-region producer를 유실, dangling live-in, stale value version, return value 누락 |
| **RB-03 Check obligations** | 원본의 zero-result `SemanticCheck`마다 **한 소유자**를 부여한다: (a) 원래 J error class·우선순위로 실행, (b) 안정된 입력/생성 경로가 검증된 `FactWitness`로 정당하게 discharge, (c) 오류·관찰 순서 동등성이 입증된 guard/check로 대체 | Check 탈락·중복·오류 class 변경·호출 뒤 이동·자기선언 witness. **증명된 Check 제거는 허용**하지만 Check의 무조건 삭제는 금지 |
| **RB-04 순서·첫 오류** | SSA def-use, `Operation.order_after`, Check→Call, observable effect/error edge, `Plan.write.after`의 순서 의무를 원본으로부터 보존한다. 첫 J 오류를 worker/GPU 완료 순서로 정의하지 않는다 | 값이 사용되지 않는 실패 가능 op·effect-live branch·Check 삭제, effect 앞뒤 재배열, 첫 오류 변경, error handler/try-catch 관찰 변화 |
| **RB-05 dynamic NAME와 Write** | `ReadNoun {symbol,version}`의 noun snapshot과 `CallTarget::Dynamic` / function NameRef의 late lookup을 분리한다. `Plan.write`는 **operations vector 밖의 독립 commit event**: symbol/value/previous/proposed/span/after를 반드시 보존 | op 전체를 커버했다는 이유로 Write 생략, 조기 function target 고정, 옛 binding snapshot 재사용, write 중복·순서 역전·effect 이후 무조건 replay |
| **RB-06 region-wide legality** | `LoweringRegistry::legal_candidates`의 *후보*를 넘어서, 선택 region 전체의 primitive/valence·Rank/CellApply/empty-frame fill, effect/error, target feature, runtime guards, alias/bridge 전제조건을 검증한다 | per-op support의 단순 합집합으로 전체 region 실행 승인, `ReferenceSequential`을 compiler-native kernel로 오인, Unknown effect/error를 Pure/NoError로 몰래 완화, tolerance/boxed/sparse/가변 cell 결과·재결합 미증명 승격 |
| **RB-07 bridge/fallback** | 경계에는 J-visible dtype/shape/rank/boxed/sparse와 value/effect ordering **요구사항**만 둔다. 구체 `BufferId`/stride/host↔device transfer/ready token은 후속 Physical/Bridge에서 결정한다. Guard는 observable effect 이전에 실행한다 | legal bridge 없는 cut, 뒤늦은 guard, committed effect 이후 producer replay, GPU 메모리 배치의 부재를 무시하고 verified 처리 |
| **RB-08 proof freshness/diagnostics** | 증거를 원본 A3/registry/op/version/guard 관찰 시점에 묶고, 검증 불가 의무와 Reject 이유를 source op/region에 귀속한다 | 다른 op의 proof 재사용, 바뀐 input/name version에 stale witness 재활용, 거짓 error summary, 미지원 backend/bridge를 Verified 처리 |

**구체적인 테스트 설계 (아직 테스트 코드를 쓰거나 실행하지 않음):**

| A3 기준 소스 | 정상 조건 | 하나씩 위조하여 거부해야 할 사례 |
|---|---|---|
| `1+2` | 두 literal의 ValueOnly → Elementwise 후보, result/value live-in 보존; CPU `ReferenceSequential`은 후보이지 native kernel 완성 증거가 아님 | result op 누락, value-live-in 누락, target capability 불일치 |
| `1 2+3 4` | PrefixAgreement의 생성·입력 shape 증거가 유효하면 Check 불필요 | witness만 Some으로 바꾸거나 실제 shape/version을 변경해 Check 생략 |
| `1 2+1 2 3` | zero-result Length Check가 Call보다 먼저 발생하고 J Length 유지 | Check 탈락/중복, Return만 보존, Check 뒤로 이동 |
| `3 { 10 20 30` | Gather의 Index Check/오류 우선순위 유지 | Check를 제거·Length로 변조·커널 뒤로 보냄 |
| `future 3` | 미지원 native call은 RuntimeSemantic 후보로 남김 | 근거 없는 GPU/native route 승격 |
| `a`, `a=:1+2` | noun snapshot과 별도 Write commit 및 이름 버전·순서 보존 | write 미매핑, 미래 버전 선읽기, write 이후 무조건 replay |
| `+/\"1 (2 3$ i.6)` 및 zero-frame 사례 | frame/cell 구분, zero-frame virtual fill 및 결과 type/shape assembly 의무를 보존 | empty frame을 no-op 처리, CellApply를 무증명 고정형 parallel map으로 변경 |
| 같은 A3의 CPU/GPU/External 후보 | 세 후보 모두 **같은 원본**에 대해 값/타입/Shape/오류/효과 의무를 충족해야 함. GPU 실구현은 별도 보류 | GPU 실행기가 없는 상태에서 실행 완료·성능 검증으로 기록 |

**실행할 검증 단계와 수용 게이트:**

1. **RB-V0 coverage:** 유효 A3 + 빈 블록 포함 모든 op-range 전수 매핑·순서·빈 partition, gap/overlap/invalid ID 음성 검사.
2. **RB-V1 semantic edges:** value/check/`Plan.write`/result 모두의 live-in/out, `order_after`, first-error, cross-region effect-live mapping을 검사. Check마다 실제 실행/증명된 discharge/동등 guard 하나를 입증.
3. **RB-V2 legality & guards:** 원본 semantic facts와 witness provenance, target/region-wide legality, guard placement 이전의 effect frontier, fallback safety를 재검증. GuardRequired는 아직 실행 불가.
4. **RB-V3 differential:** 각 정상 계획과 **단 하나의 invariant를 위조한 계획**을 비교. J 기준 C oracle, RustJ closed A3 reference(지원 입력만), RustJ interpreter의 value/type/Shape/atom/error precedence/effect/name state. Closed A3 executor가 dynamic NAME/Write oracle이 아닌 점을 명시.
5. **RB-V4 multi-route:** 같은 원본에서 target별 RoutePartition 후보를 만들고 증거를 교차 비교. 미구현 GPU/External은 정적 Rejected/GuardRequired 확인까지만; 실제 device/transfer 성능은 M4/M5/M6 별도 수용으로 둔다.

**완료 판정:** RB-01~08/RB-V0~V4가 정본·영어 미러에 들어간 것은 **M3-RB 설계 명세 완료**뿐이다. **구현/검증 완료**는 실재하는 verifier, 각 negative test의 PASS, 3자 의미론 차분 결과와 증거 기록이 필요하다. `partition_plan`의 range/class, A3 `verify` 통과, 문서 수정만으로 M3·M4·HE-01을 완료 처리하지 않는다.


<a id="sw-mlpl-reference-checklist"></a>
<a id="외부-구현-사례-sw-mlpl--rustj-적용검증-체크리스트-2026-10-08-설계-조사만"></a>
##### 외부 구현 사례: sw-MLPL → RustJ 적용·검증 체크리스트 (2026-10-08, 설계 조사만)

**검토 대상 및 성격:** [sw-MLPL](https://github.com/sw-ml-study/sw-mlpl/tree/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f)의 2026-10-03 `main` snapshot (`d53c35f810aa`)에서 구현·테스트·설계 문서를 대조했다. 이 절은 **기존 M3 RouteBoundary / M4 Physical CPU / M5+ heterogeneous planning의 구현 참고와 수용 항목**이며 새로운 필수 IR, 실행 경로, RustJ 언어 문법, 별도 진행 문서를 추가하지 않는다. RustJ 기존 RB-01~08, RB-V0~V4, M4→M5 검증, §17/HE 체크리스트가 우선한다. **[x] 외부 설계·소스 검토 / [ ] RustJ 도입·검증은 아직 미실시**이다. 이 문서 갱신을 기능 구현/테스트 통과로 세지 않는다.

**비교로 확정한 경계:** sw-MLPL은 Rust 인터프리터, NumPy식 trailing-axis broadcasting, named-axis metadata, 일부 compile-to-Rust, MLX resident tape 및 제한된 CUDA 경로를 갖는다([S0], [S2], [S3]). RustJ는 J의 Rank/CellApply 및 zero-frame virtual fill, NAME/locale/`".`, J dtype·boxed·sparse·fit·오류·효과 우선순위를 보존한다. sw-MLPL의 `AxisSpec`/device handle/컴파일 가능 subset을 RustJ의 J semantic identity, Logical `ValueId` 또는 전체 J 지원 범위로 **그대로 치환하지 않는다**. 특히 `[0,3]`의 zero frame과 `[2,0]`의 positive frame/empty cell은 원소 수가 같아도 실행·prototype 의무가 다르다(§P.12).

**실행 체크리스트** — 한 항목을 닫을 때 *변경 코드 위치 + PR/commit + 재현 입력/음성 케이스 + 실행한 테스트 명령과 실제 결과 + 해당 source 근거*를 §17의 기존 단계 체크리스트에 연결한다. CI 대기, 테스트 미실행, 특정 backend 부재는 통과가 아니다.

| ID / 단계 | RustJ에 채택할 방법 (기존 계약에 추가) | 수용 게이트 / 실패해야 할 반례 | 근거 |
|---|---|---|---|
| **SW-01 / M3** | [ ] `LoweringRegistry`의 *기계 판독 가능한 coverage/capability 표*를 primitive + valence + rank/cell + J dtype/representation + effect/error + route/target/guard 축으로 점검한다. supported, runtime/guarded, 구현 미지원 후보를 구분하되 **op별 supported ≠ region Verified**로 고정한다. 문서의 범위 선언과 테스트의 registry 목록이 일치하게 한다. | 새 lowering 등록/제거 시 coverage gate 갱신 요구; 미등록 op, 틀린 valence, blocked boxed/sparse, stale witness, zero-frame 의무 누락 및 `ReferenceSequential`을 native로 오인한 계획은 `Rejected`/`GuardRequired`; RB-06/08, RB-V2~4까지 통과해야 완료. | [S4], [S5] |
| **SW-02 / M3** | [ ] `RankPlan`/A3 `IterationDomain`/SemanticCheck에 이미 있는 shape, frame/cell/reduction-axis, 결과 cell 조립, 추론 증거를 **공통 구조화 진단/설명**으로 제공한다. axis 선택 규칙을 중복 구현하지 않고 source span/provenance 및 `ErrorKind + ErrorContext`를 유지한다. 사용자 문법에 MLPL named axes를 이식하지 않는다. | 동일한 Rank/Shape 문제의 A3/diagnostic 표시가 일관되며, 잘못된 축·중복/범위 오류·`[0,3]` 대 `[2,0]`·불명확한 결과 cell 조립·원래 첫 J 오류가 보존되는지 회귀 검증; diagnostic이 J 오류를 가리거나 변경하면 실패. | [S1], [S2], [S7] |
| **SW-03 / M4** | [ ] 여러 compiler-native 작은 사례를 묶어 컴파일 비용을 분산하는 **batched parity harness**를 기존 3자 차분(jsource C, RustJ semantic reference, 실제 Physical/native route)의 보조 도구로 사용한다. 지원 범위 밖 입력은 명시적으로 분류한다. | 수치 scalar 하나만이 아니라 dtype·shape·ordered atoms·boxed/sparse(해당 지원 범위)·J error class/first error·effect/name 상태까지 비교. 가상 Cell/fill, Checked overflow, fit/tolerance, 동적 NAME, unsupported route negative를 포함. compiler가 reference evaluator를 호출한 것은 native 통과가 아님. | [S6], [S4] |
| **SW-04 / M4** | [ ] parse/compile/실행 시간, allocation/peak memory, materialization, 읽기·쓰기 bytes를 나누고 **동일한 실제 lowering 결과**를 기준으로 micro/macro benchmark를 남긴다. 변경 blast radius에 맞는 빠른 proxy test와 느린 release gate를 구분한다. | CPU reference/native 동일 입력 비교, scalar/short array/reduce/reindex/빈 frame·복합 rank/effect-boundary 별도 기록; 결과 검증이 선행하며 환경/커밋/설정·median·회귀 기준 기록. 실행하지 않은 benchmark는 증거가 아님. | [S8], [S9] |
| **SW-05 / M5+** | [ ] CPU/GPU/External의 실제 Physical/Bridge 경계에 upload/download, materialize, submit, fallback, transfer bytes, sync/ready wait, guard miss를 **측정 이벤트**로 정의한다. 실제 storage/lease/version과 event를 연결하되 A3 `ValueId`에 device handle을 넣지 않는다. | zero-transfer CPU baseline; 같은 J값의 physical residency 변화가 J-visible 의미를 바꾸지 않음; transfer-before-produce/read-before-ready/stale version/double-commit 거부. GPU 실장치가 없으면 설계/정적 verifier만 기록하고 resident 실행·성능을 성공으로 표시하지 않음. | [S3], [S10] |
| **SW-06 / M4→M5+** | [ ] source span, original A3 OpId/`j_origin`, dtype/shape, effect/error check, physical route·materialization·fallback을 연결하는 **선택적 structured trace**를 만든다. 대형 noun 본문은 복사하지 않고 제한된 요약/ID만 기록하며 비용을 별도 측정한다. | trace enable/disable 결과가 J 값·오류·효과·순서에 영향을 주지 않는 회귀; source→A3→physical 대응 및 reject 이유 확인; 구조 진단과 runtime 관찰을 혼동하지 않는다. | [S7], [S11] |

**진행 의존성 및 도입 금지 사항**

- **M2 우선순위 유지:** jsource 기반 frontend/Rank/empty-frame 및 C oracle mismatch 수렴을 먼저 수행한다. SW-01/02가 M2 의미론 수정을 우회하는 shortcut이 되어서는 안 된다.
- **M3 → M4 → M5(→ M6):** SW-01/02는 RB 검증기에 붙이고, SW-03/04는 실제 native CPU vertical slice가 있을 때 측정한다. SW-05는 M5의 transfer/residency 모델 이후, GPU 실제 실행은 별도 M6/재개 승인 뒤에만 한다. SW-06은 관찰 필요 시 단계적으로 추가한다.
- **비채택:** NumPy broadcasting으로 J Rank 대체, J의 `".`/동적 이름을 금지하는 closed-world AOT, MLPL `Value`/`TensorHandle`을 J noun/FunctionEntity/A3 `ValueId`와 통합, autograd tape를 범용 J SSA로 대체, sw-MLPL의 숫자 하나 parity를 전체 J 적합성으로 간주하는 것은 모두 금지한다.
- **완료 상태 (2026-10-08):** [x] 자료 조사·적용 경계 결정 및 출처 고정, [ ] SW-01, [ ] SW-02, [ ] SW-03, [ ] SW-04, [ ] SW-05, [ ] SW-06. 각 항목은 **실제 구현 + 해당 negative/conformance test PASS + 출처·명령·commit/CI 증거**가 있기 전까지 열린 상태다.

**원본 자료 (모두 동일 commit으로 고정; 설계/문서와 실제 구현을 구분)**

- **[S0]** [sw-MLPL README: 언어·backend와 실험적 범위](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/README.md).
- **[S1]** [AxisSpec 공통 axis resolver 구현](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/components/array/crates/mlpl-axes/src/axis_spec.rs).
- **[S2]** [trailing-axis broadcast 구현](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/components/array-element/crates/mlpl-array-ops-element/src/broadcast.rs), [shape/empty 회귀 테스트](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/components/array-element/crates/mlpl-array-ops-element/tests/broadcast_shape_tests.rs).
- **[S3]** [CPU/device TensorHandle](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/components/array/crates/mlpl-tensor-handle/src/handle.rs), [DeviceOps trait](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/components/array/crates/mlpl-tensor-handle/src/ops.rs), [upload/download/submit/fallback counters](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/components/array/crates/mlpl-tensor-handle/src/metrics.rs).
- **[S4]** [컴파일러 coverage boundary 테스트](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/components/syntax-codegen/crates/mlpl-lower-rs/tests/coverage_boundary_tests.rs), [coverage 가이드](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/docs/compiler-coverage.md).
- **[S5]** [supported lowering dispatch coverage 테스트](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/components/syntax-codegen/crates/mlpl-lower-rs/tests/dispatch_coverage_tests.rs).
- **[S6]** [interpreter vs compiled parity harness](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/components/dev-tools/crates/mlpl-parity-tests/tests/parity_tests.rs) — 현재 수치 scalar 중심 비교이므로 RustJ 전체 오라클의 대체가 아니다.
- **[S7]** [typed TraceEvent 구조](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/components/autograd/crates/mlpl-trace/src/event.rs).
- **[S8]** [actual lowering 기반 bench 방식과 수치](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/docs/benchmarks.md), [benchmark harness](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/components/dev-tools/crates/mlpl-bench/benches/interp_vs_compiled.rs).
- **[S9]** [blast-radius별 빠른/느린 테스트 cadence](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/docs/testing-cadence.md).
- **[S10]** [sw-MLPL architecture: resident device seam과 GPU 범위](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/docs/architecture.md).
- **[S11]** [구조적 dataflow renderer 설계(도입 의무 아님)](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/docs/dataflow-renderer-design.md).


MLIR은 여러 abstraction의 dialect를 한 module 안에서 공존시키고 dialect conversion으로 점진적으로 lowering할 수 있으므로 RustJ의 multi-level IR 구조와 특히 잘 맞는다.

향후 RustJ Logical IR 전체를 MLIR tooling 안에서 보존할 필요가 생기면 **RustJ-specific MLIR dialect**를 fidelity-preserving bridge로 둘 수 있다. 다만 v0의 필수 구현은 아니다. 초기 adapter는 안전하게 표현 가능한 op만 기존 `tensor/linalg/arith/scf` 등으로 직접 lowering하고, 의미 손실이 생기는 op는 거부한다. StableHLO는 ML framework/compiler 사이의 portability layer를 목표로 하는 high-level op set이므로 NN/tensor subset의 선택적 export 대상으로 본다. LLVM IR/SPIR-V는 더 낮은 execution target으로 사용한다.


---
