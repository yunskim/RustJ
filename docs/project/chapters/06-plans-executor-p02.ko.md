[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)

<a id="h-p--m3m4-구현-인수-패킷과-물리-검증-체크리스트-계약-2026-10-07-문서-전용"></a>
##### H-P — M3→M4 구현 인수 패킷과 물리 검증 체크리스트 계약 (2026-10-07; 문서 전용)

**사용법:** 이 절은 H-01~09, H-K, H-A(A0~A5), RB-01~08의 **실행 시점 판정표**다. 구현·PR 심사 때 각 행에 입력 증거, 검증 결과, 부정 사례와 증거 링크를 채운다. 새 필수 IR, `TaskId` 런타임, 멀티프로세서 전용 planner를 요구하지 않는다. **미제공/unknown은 승인으로 간주하지 않는다.** M3가 의미론·선택된 route의 합법성 증거를 제공하며 M4는 이 증거를 재작성하지 않고 선택한 물리 실현을 검증한다. 전역 A3 coverage는 M3 소유, native region의 실현 coverage는 M4 소유다.

**선행 단계가 전달해야 할 최소 패킷(개념적 묶음이며 새 Rust struct 요구 아님):**

| ID / 책임 | M3/상위 단계가 인계할 증거 | M4가 반드시 확인할 사항 | 증거 부족·불일치 시 |
|---|---|---|---|
| **HP-01 원본** | 별도 보존한 검증된 원본 A3, schema/primitive registry, 원본 op ID·span·J graph origin, 변경/재작성 대응과 증명의 적용 범위 | 단순 source **문자열** 일치가 아니라 **실제 원본 op payload/facts/Check/Write와 selected region provenance**를 대조. digest 단독·바뀐 A3의 `verify()` 성공만으로 원본 동일성 승인 금지 | stale/위조 source proof 거부 |
| **HP-02 범위** | 원본 모든 op·0-result Check·별도 `Plan.write`의 M3 global ownership, 각 region op 범위, live-in/out, final result, 교차 route bridge | 해당 native region에서 원본 op→physical task 관계가 **완전하고 증명된 매핑**인지 대조. 1:1을 강제하지 않으며 fusion/rewrite의 N:1·1:N 대응은 RB-01 등가 witness가 필수. 외부 구간과 live-out 누락 없음; `NoKernel`/`ValueOnly` 후보 분류만으로 binding 승인 불가 | coverage 누락·겹침·증거 없는 병합/중복 실행 거부 |
| **HP-03 호출과 recipe** | 원본 `OpKind`, callable POS/valence, derived Rank/CellApply/fit/tolerance/numeric policy, selected route와 **구체적 구현** capability/target/guard | 등록된 `ReferenceSequential` 등의 *후보*와 실제 구현 kernel/adapter를 구분; 지원 dtype/rank/shape/valence·CPU target 검증 | 지원되지 않는 native kernel은 실행 금지 |
| **HP-04 오류와 Check** | 각 `SemanticCheck`의 constraint/입력·기대 J error kind·원본 선후관계·execute / proven discharge / guard 중 유일한 owner | 결과 SSA 값이 없는 Check도 누락 금지. 실행/guard는 **해당 Check에 종속된** kernel·효과보다 앞서되, **원본에서 Check보다 먼저 발생해야 하는 다른 오류·효과보다 앞당겨서는 안 됨**. 입증된 discharge는 원본 error/order 조건과 불변인 입력 증거가 필요 | 누락·중복·선후관계·J error class 변조 또는 무근거 discharge 거부 |
| **HP-05 Name/Effect/Write** | noun read 시점의 value/binding-version/scope/locale 증거, function nameref의 late lookup/POS, effect edges, `Plan.write` 별도 commit owner | source statement 전체의 사전 snapshot 금지, Name 변동 뒤 guard 재확인, `Return`을 assignment commit으로 오인 금지; effect 후 무조건 replay 금지 | 미확정 binding·commit owner는 native 승인 보류 |
| **HP-06 logical value** | producer/def-use, dtype/rank/Shape/J atom order, boxed/sparse·0-cell Rank fill/prototype/assembly 의무, overflow·promotion·error witness | 물리화로 logical value가 사라지지 않으며 `ValueId`≠`PlanBufferId`≠runtime `BufferId`; 빈 배열 Shape와 J prefix frame 규칙 유지 | 변환 대신 semantic/reference route 또는 Unsupported |
| **HP-07 계획 버퍼** | 값별 storage/materialization 요구, readonly/externally owned/persistent/temporary 여부, alias·donation 허용 증거, 메모리 상한 | buffer encoding·capacity·alignment·memory space·view span/stride/offset·generation·실제 backing alias, overlapping writable view, last-use 뒤 재사용만 허용 | bounds/alias/ownership 모순 거부, input 불변 유지 |
| **HP-08 Task/Dependency** | 원본 data-flow·observable order·Check/guard/effect/first-error edge, 각 value ready와 region interface | `BindInput/Check/View/Materialize/Kernel/Return` 같은 task의 source 대응, 선행 task 및 ready 보장, DAG cycle/누락 edge 검출; 순차 CPU에서는 검증된 list order로 실현 가능 | 의존성 위반 task launch 거부 |
| **HP-09 Completion/수명** | task 사용 집합, 각 reader/writer·transfer의 완료 필요조건, 반환·임시 소유권 | `submitted`≠`completed`; **모든** 사용 완료 전 buffer 재활용·외부 반환 금지. v0 동기 CPU는 task 완료 시 즉시 completion으로 간주할 수 있지만 그 근거를 명시; 비동기 경로는 explicit event/timepoint/lease 필요 | completion 증거 없는 재사용·비동기 실행 금지 |
| **HP-10 동적 guard·fallback** | witness/proof provenance, 재검증할 runtime 조건, 해당 guard의 effect 이전 안전 지점, **기존에 적법한** 대체 route | guard 검사→사용 사이 stale version/shape 불가; failure 시 effect 이전에만 지원 route 선택; 부분 effect 이후 자동 재실행 금지. Backend 실패를 임의 J Domain/Rank/Length로 매핑 금지 | guard false면 native 실행 중지, safe fallback 없으면 명시적 실패 |
| **HP-11 장치·자원** | `TargetContext`/resolved capability 및 device/runtime availability, resource limits, 비용/자원 추정의 구분 | 첫 M4: CPU 실행·Host memory·단일 순차·zero transfer만 승인. 미래 GPU/멀티코어는 device placement, memory space, transfer readiness, intra-device scheduling 독립 검사; resource feasible ≠ profitable | 미지원 device/transfer 경로 승인 금지 |
| **HP-12 실행 결과 증거** | reference/Jsource 대응 가능 사례, 실행 route와 guard 기록, 검증할 dtype/Shape/atom·effect/error 결과 | 독립 RustJ semantic/A3 reference와 가능한 pinned J C reference, **실제 native route**의 3방향 비교; CI 명령·commit·artifact 연결 | 테스트 소스 존재/후보 선택만으로 M4 E2E 통과 주장 금지 |

**최소 자료흐름/승인 프로토콜:**

~~~text
M3: immutable SourceA3 + selected RouteRegion + original op/Check/Write coverage
    + live-in/out + semantic proof/witness/guards + observable order
  -> RouteVerified  (A0/A1, 아직 실행 허가는 아님)
M4: task(s) + planned buffer/view + def-use/dependency + readiness/completion
    + selected real CPU realization + lease/ownership/resource checks
  -> PhysicalVerified  (A2/A3, 특정 native region에 한정)
Call time: name/input versions + guards + actual buffer generations/leases
  -> RuntimeReady -> Execute -> observed result/error/effect -> independent comparison (A4/A5)
~~~

**최종 심사 시점과 판정(기존 A0~A5의 통과 기준; 새 gate 아님):**

| 기존 gate / 판정 책임 | PASS에 반드시 필요한 증거 | FAIL/UNRUN 및 후속 |
|---|---|---|
| **A0 SourceA3 — Frontend/A3 소유** | 원본 `verify()` 및 스키마/registry/op payload·facts·span·origin의 **변경되지 않은 권위 스냅샷** 확인; 수정 후보는 원본을 덮어쓰지 않음 | 원본 부재·stale → FAIL; snapshot 대조 미실행 → UNRUN |
| **A1 RouteVerified — M3 소유** | RB-01~08의 **전역** op·Check·Write 커버리지, 실현 가능한 선택 region, 모든 effect/error/guard/Name 및 bridge 의무의 증명 또는 안전하게 owner 지정. `GuardRequired`를 무조건 `Verified`로 표시하지 않음 | 누락·증거 없는 fusion·불명 capability → FAIL/UNRUN; 다른 적법 route만 가능 |
| **A2/A3 ExecutableRoute/PhysicalVerified — M4 소유** | 실제 설치/구현된 selected realization + 해당 region의 증명된 source↔task 매핑, checked buffer/view/alias·의존성·동기 CPU 완료/자원 경계. A1 PASS를 M4 PASS로 복사 금지 | 물리 verifier·native kernel 미구현은 UNRUN/미지원. 미정의 buffer/edge·alias 위반은 FAIL |
| **A4 RuntimeReady — 호출 시점 Runtime/Executor 소유** | 해당 **호출마다** live input/name/version·guard·lease/generation·readiness 재검증; effect 전 fallback 준비 여부 | guard false이면 native 미실행, effect 전 합법 route로만 분기. 이미 commit된 effect 뒤 retry 금지 |
| **A5 AcceptedNativeE2E — 독립 차분 검증 소유** | 실제 native route 실행 기록과 semantic reference 및 가능한 pinned J C oracle의 **값·Shape·첫 J error·effect** 일치, positive/negative 테스트와 CI commit/run 기록 | reference/fallback 실행만 성공하거나 증거 미실행이면 UNRUN; 불일치 시 FAIL. C oracle 불가 항목은 불가 사유를 명시하고 3방향 검증 완료로 표시하지 않음 |

**오류 분류:** 잘못된 물리 계획/미구현 capability/`GuardRequired`는 J 프로그램의 Domain·Rank·Length 오류가 아니다. 반면 적법한 J 실행에서 먼저 발생한 `SemanticCheck`는 원래 J error class와 순서를 유지한다. 증거의 존재(문서·테스트 정의)는 실행된 PASS와 별개다. 각 gate의 PASS는 **상위 gate PASS를 자동 승계하지 않는다**. `PhysicalVerified`는 특정 선택된 native region에 한정되며 전체 프로그램이나 GPU까지 확장하지 않는다.

**단계별 부정 테스트 등록부(각 테스트에서 정확히 한 조건만 위조):**

| 시험 | 기준 사례 / 실패 유도 | 기대 판정 |
|---|---|---|
| **HP-V01 source** | 같은 source 문자열로 literal atom, SSA facts, rank 또는 `j_origin` 변경 | 검증 실패. Draft PR #4의 `LiteralSourceWitness`는 **단일 literal v0의 부분적 보완**이지 일반적인 M3 원본 증명이 아님 |
| **HP-V02 coverage** | `1+2` Add op 누락, `a=:1+2`의 독립 `Write` 유실, 중복 region | RouteVerified/PhysicalVerified 거부 |
| **HP-V03 checks** | `1 2+1 2 3`의 Length Check 제거·후행·중복·Index로 변조 | Add 실행 전에 거부, 또는 정확한 J Length 오류 |
| **HP-V04 guard/name** | `a`의 noun snapshot/version 변경, late verb NameRef 조기 고정, shape guard 값 변경 | revalidation 불합격; 효과 이전의 적법한 대체 경로만 허용 |
| **HP-V05 buffer/view** | **정상 대조군:** 유효한 음수 stride·singleton 0-stride·빈 Shape·read-only alias. **각각의 위조군:** 실제 span 범위 이탈, stale generation, 잘못된 encoding, 겹치는 독점 write, 불법 last-use 재활용 | 합법적인 stride·빈 Shape를 거부하지 않음; bounds/alias/수명 위반만 거부 |
| **HP-V06 dependency** | Check/guard→Kernel, producer→reader, previous effect→commit edge 하나 삭제하거나 순환 추가 | plan verifier 거부, arbitrary first-error 노출 금지 |
| **HP-V07 completion** | 비동기 copy/kernel을 enqueue만 하고 입력 release, reader 완료 전에 재사용 | 사용 가능·재활용 거부; 실제 completion 이후만 허용 |
| **HP-V08 replay/error** | J error가 가능한 작업의 순서 뒤집기, effect commit 뒤 guard failure fallback 재실행 | first-error/effect order 보존 실패로 거부, 중복 effect 금지 |
| **HP-V09 baseline** | 닫힌 literal identity와 이후 실제 구현한 순차 CPU 단일 op의 차분 비교. Boxed/sparse·Rank zero-frame처럼 **그 시점 native 구현에서 미지원**인 정당한 J 입력은 semantic/reference fallback으로 별도 검사 | native 지원/미지원 정확 분류; semantic fallback 자체는 J 성공이어도 native E2E 승인으로 세지 않음 |

**증거 기록 양식(구현 시 각 gate마다 1건):** `gate ID | source A3 revision/schema/registry | source op/region IDs | M3 proof·guard owner | chosen recipe/capability | physical task/dependency/buffer/view IDs | runtime version/completion/effect frontier | positive/one-invariant-negative test IDs | CI commit/run·reference artifact | PASS/FAIL/UNRUN | 남은 예외`. `UNRUN`은 PASS가 아니다.

**현재 상태/우선순위:** `main`은 M3 RouteBoundary와 M4 full native verification을 아직 갖추지 않았다. Draft [PR #4](https://github.com/yunskim/RustJ/pull/4)의 제한된 literal identity 경로·source witness·CI 성공이 있더라도 Check, 실제 Add, 전체 source coverage, completion/비동기, GPU 지원을 증명하지 않는다. 먼저 **HP-01~06·08·10의 순차 CPU 의미/검증 계약**을 실현하고 HP-V01~04/06/08을 확인한다. 그 다음 실제 CPU Kernel과 HP-07/09 자원·수명 검증을 추가한다. HP-09의 비동기 completion, transfer, multi-device/worker scheduling은 이종 실행 단계에서 확장한다. M4-v0는 full async scheduler를 필수 선행조건으로 하지 않는다.

<a id="현재-코드와의-대응"></a>
##### 현재 코드와의 대응

현재 `src/physical.rs`는:

~~~text
BufferRegistry / BufferLease / runtime BufferId
checked read-only affine PhysicalArray
shape / strides / offset / encoding validation
~~~

을 제공하는 **G1 representation foundation**이다. 아직 `PhysicalPlan`, plan-time buffer slot, planner, physical executor가 아니다. `logical_executor.rs`도 A3 semantic/reference executor이지 Physical Executor가 아니다.

<a id="구현-순서--한-번에-한-의미--한-verifiertest"></a>
##### 구현 순서 — 한 번에 한 의미 + 한 verifier/test

1. plan-time `PlanBufferId`/`PhysicalViewId`와 empty plan verifier
2. `BindInput + Return`만으로 identity plan E2E
3. `View` + span verifier, transpose/reverse metadata-only 회귀
4. `Check` + J error class/order regression
5. `Kernel` 한 종류(Add) + selected lowering capability verification
6. `Materialize` + logical-order copy/ownership test
7. last-use + reuse witness, alias negative tests
8. 여러 op를 연결한 verified Logical IR → PhysicalPlan → CPU result differential test

이 순서는 G4를 구현할 때의 최소 vertical slice이며 full GPU resource model을 선행 조건으로 만들지 않는다.

<a id="53-rustj-native-executor"></a>
### 5.3 RustJ-native Executor

이 절은 Route A에만 적용한다. RustJ-native Executor는 이미 정해진 Physical Plan을 수행한다. 외부 compiler/runtime route는 각 시스템의 executor/runtime가 자체 lower-level scheduling을 수행할 수 있다.

Executor가 다음을 다시 판단해서는 안 된다.

- J rank 의미
- hook/fork 의미
- fusion 여부
- layout 선택
- device 선택
- buffer reuse legality

Physical Plan에서 비동기 실행을 허용할 경우 dependency는 implicit host order에 기대지 않고 `AsyncToken/Timepoint` 또는 동등한 explicit edge로 표현한다. resource의 사용 가능 시점과 lifetime은 이 timeline과 연결한다. IREE Stream의 timepoint/resource model과 MLIR Async의 explicit dependency token이 참고 모델이다.

<a id="54-실행-경로는-하나가-아니다"></a>
### 5.4 실행 경로는 하나가 아니다

`Logical Array IR`을 만든 이후 반드시 RustJ의 Physical Planner를 거쳐야 하는 것은 아니다.

<a id="route-a--rustj-native"></a>
#### Route A — RustJ native

```text
Logical Array IR
  → RustJ Logical Optimizer
  → RustJ Physical Planner
  → Physical Plan
  → RustJ Executor
```

장점:

- J-specific semantics와 실험적 hardware model을 가장 직접적으로 제어
- reference/bootstrap path
- external compiler와 결과 비교 가능

<a id="route-b--mlir"></a>
#### Route B — MLIR

```text
Logical Array IR
  → RustJ-to-MLIR adapter
  → tensor/linalg/arith/scf
  → vector/gpu/memref
  → LLVM / NVVM / ROCDL / SPIR-V
  → execution
```

MLIR Linalg는 generic indexing map/iterator semantics를 이용해 tiling, fusion, vectorization, loop lowering, library/intrinsic lowering을 제공하도록 설계되어 있다. RustJ의 `IterationDomain + AccessRelation`은 이 계층으로 내리기 좋은 형태를 목표로 한다.

RustJ가 MLIR의 최적화 passes를 재구현할 이유가 없다. 다만 J의 observable semantics를 위반할 수 있는 reassociation, error-order 변경 등의 lowering은 adapter가 막거나 필요한 attributes/guards를 제공해야 한다.

<a id="route-c--stablehlo--openxla-compatible-subset"></a>
#### Route C — StableHLO / OpenXLA-compatible subset

NN/tensor 중심의 일부 LogicalOp은 StableHLO로 자연스럽게 표현될 수 있다.

```text
Logical Array IR subset
  → StableHLO
  → XLA / IREE / compatible compiler
```

StableHLO에는 token 기반 side-effect ordering, send/recv, side-effecting `custom_call` 같은 기능이 존재한다. 그러나 이것이 arbitrary J state/effect/error semantics 전체를 표현한다는 뜻은 아니다. J-specific entity, boxed semantics, unusual numeric/error ordering, 지원되지 않는 effect/resource model은 이 route에 억지로 넣지 않는다.

필요하면 StableHLO `composite`나 `custom_call` 계열 escape hatch를 사용할 수 있지만, 그것이 semantic contract를 숨기는 수단이 되어서는 안 된다. adapter는 effect/token mapping을 명시적으로 검증한다.

<a id="route-d--direct-external-librarykernel"></a>
#### Route D — Direct external library/kernel

```text
LogicalOp / PhysicalRegion
  → verified library mapping
  → BLAS / FFT / vendor NN library / custom kernel
```

library call은 하나의 backend realization이며 primitive identity와 분리한다.

<a id="55-외부-ir을-사용할-때-rustj가-끝까지-책임지는-것"></a>
### 5.5 외부 IR을 사용할 때 RustJ가 끝까지 책임지는 것

외부 compiler에 넘긴다고 해도 다음 책임은 RustJ에 남는다.

- J source semantics
- rank/cell/frame/agreement
- primitive/derived-verb identity의 올바른 해석
- dtype/promotion/error contract
- effects/alias legality
- numeric relaxation/reassociation 허용 여부
- dynamic shape guard
- external lowering precondition
- unsupported case detection
- provenance와 differential validation

반대로 register allocation, instruction selection, generic tiling/vectorization, machine-code generation처럼 이미 성숙한 외부 compiler가 더 잘하는 부분은 위임할 수 있다.

<a id="551-external-adapter-boundary-contract"></a>
#### 5.5.1 External adapter boundary contract

external adapter는 RustJ semantic IR의 대체물이 아니라 **verified RouteRegion/Logical IR의 projection**이다. 첫 실제 MLIR/StableHLO/ArrayFire/library adapter를 구현하기 전에 모든 adapter가 공통으로 만족해야 할 boundary를 고정한다.

입력 계약:

~~~text
ExternalAdapterInput
  RouteRegion
    live_in / live_out
    ordered operations / SemanticChecks
    effect/error edges
    provenance
  discharged legality evidence / guards
  resolved TargetContext
  logical representation requirements
~~~

adapter는 unresolved legality를 backend optimizer가 알아서 해결할 것이라고 가정하지 않는다.

capability query는 최소 다음 축을 분리한다.

~~~text
AdapterCapability
  operation / ExecutionBasis / valence
  dtype classes
  rank / shape constraints
  dynamic-shape support
  representation / layout preconditions
  numeric/tolerance/reassociation policy
  effect/token support
  error/check representation
  alias / mutation contract
  async / completion semantics
~~~

`supports Add` 하나로는 충분하지 않다. 예를 들어 같은 Add라도 rank/cell mapping, dtype promotion, overflow/error semantics, layout/alias requirement가 다르면 다른 capability query가 필요할 수 있다.

출력은 단순 external module bytes가 아니라 RustJ가 검증할 수 있는 projection record를 포함해야 한다.

~~~text
ExternalRegionPlan
  adapter_id / adapter_schema_version
  source RouteRegion + A3 provenance
  translated external operations/module
  host-side checks retained
  mapped effect/token edges
  BridgeRequirements
  external input/output handles
  completion / ownership contract
  unsupported/compile-failure classification
~~~

<a id="semanticcheck--error-mapping"></a>
##### SemanticCheck / error mapping

각 A3 `SemanticCheck`는 다음 중 하나여야 한다.

1. external launch 전에 RustJ host/native side에서 원래 순서대로 실행,
2. external IR이 **같은 J-visible error class와 precedence**를 보장할 수 있을 때 명시적으로 lowering,
3. 그렇지 않으면 해당 region을 external route에서 거부.

backend assertion/trap을 무조건 J Domain/Rank/Length error로 바꾸지 않는다. external compiler crash/unsupported/kernel launch failure도 J semantic error가 아니다.

<a id="effect--token-mapping"></a>
##### Effect / token mapping

- pure region은 token 없이 projection할 수 있다.
- J-visible write/I/O/state ordering이 있는 region은 backend가 equivalent token/resource ordering을 표현할 수 있을 때만 projection한다.
- StableHLO token/custom_call 같은 escape hatch가 존재한다는 사실만으로 arbitrary J effect support를 선언하지 않는다.
- host-side effect와 external async operation이 섞이면 completion token이 §2.5.1 RouteBoundary와 §3.9.4 commit frontier에 연결되어야 한다.

<a id="representation-bridge--ownership"></a>
##### Representation bridge / ownership

adapter는 J logical value를 backend layout과 동일시하지 않는다.

~~~text
A3 Logical Value
  ↓ BridgeRequirement
Physical/External bridge lowering
  ↓
external tensor/array/handle
  ↓ completion + ownership
A3 live_out / next RouteRegion
~~~

row-major/column-major, dense/sparse encoding, device memory, alignment, zero-copy 가능성은 adapter precondition/bridge/Physical Plan 책임이다. semantic dtype/shape/order를 layout에 맞춰 바꾸지 않는다.

<a id="round-trip-verifier"></a>
##### Round-trip verifier

adapter output은 최소 다음을 검증할 수 있어야 한다.

- input RouteRegion의 모든 semantic operation이 translated op, host-side check, explicit bridge/effect action 중 정확한 대응을 가진다.
- live-in/out logical dtype/shape/rank/order contract가 projection 전후에 일치한다.
- dropped/reordered SemanticCheck/effect/error edge가 없다.
- adapter capability/witness가 실제 emitted external form의 requirement와 일치한다.
- external output handle의 ownership/completion이 다음 region이 사용하기 전에 확정된다.
- source A3 op/J Graph/source span으로 provenance를 역추적할 수 있다.
- unsupported form은 partial external module을 실행 가능한 성공 plan으로 반환하지 않는다.

이 verifier는 external compiler 자체의 optimizer correctness를 재증명하는 것이 아니라, **RustJ가 넘긴 의미와 adapter가 선언한 projection 사이의 계약**을 검증한다.

<a id="failure-classes"></a>
##### Failure classes

~~~text
AdapterUnsupported
  semantic/capability/precondition상 이 route를 만들 수 없음

AdapterCompileFailure
  backend compiler/API가 plan 생성에 실패

AdapterRuntimeFailure
  launch/execution/completion infrastructure 실패

JSemanticError
  RustJ SemanticCheck/operation contract가 정의한 실제 J error
~~~

앞의 세 항목을 임의로 `JSemanticError`로 재분류하지 않는다. 실행 전 failure이면 §3.9.4에 따라 verified alternate route를 선택할 수 있지만, observable effect/transfer commit 뒤에는 자동 replay하지 않는다.

<a id="첫-adapter-구현-gate"></a>
##### 첫 adapter 구현 gate

- 하나의 small pure-array region만 지원해도 되지만 capability matrix를 명시한다.
- unsupported dtype/rank/shape/layout가 fail-closed인지 test한다.
- host-side SemanticCheck가 external launch보다 먼저 같은 error를 내는지 differential test한다.
- representation copy/view bridge가 logical atom order를 보존하는지 test한다.
- output ownership/completion 후에만 consumer region이 접근하는지 test한다.
- adapter plan에서 source provenance가 round-trip되는지 test한다.
- external backend를 바꾸어도 같은 verified Logical IR의 J result/error가 유지되는지 비교한다.

현재 **실제 production external adapter가 이 계약을 완료했다는 뜻은 아니다.** M6 이전에는 이 절이 implementation gate 역할만 한다.

<a id="56-외부-ir-선택-원칙"></a>
### 5.6 외부 IR 선택 원칙

하나의 외부 IR에 전체 RustJ를 맞추지 않는다.

- **MLIR**: 가장 일반적인 multi-level lowering 후보. custom dialect도 가능하고 Linalg/Vector/GPU/LLVM/SPIR-V 등으로 점진 lowering 가능.
- **LLVM IR**: CPU 및 low-level codegen target. J의 high-level array semantics를 직접 담는 주 IR로 사용하지 않는다.
- **SPIR-V**: Vulkan/OpenCL 계열 compute target용 low-level portable binary IR.
- **NVVM / ROCDL**: NVIDIA/AMD-specific LLVM-level GPU lowering.
- **StableHLO**: ML/tensor op subset의 portable high-level interchange. J 전체 semantic IR의 대체재로 보지 않는다.

external route의 존재 때문에 RustJ Logical IR을 외부 IR의 최소공배수로 축소하지 않는다. **RustJ IR이 더 풍부하고, adapter가 필요한 subset을 projection하는 구조**를 유지한다.



<a id="57-logical-ir은-verifierinterfaceversion-경계를-가진다"></a>
### 5.7 Logical IR은 verifier·interface·version 경계를 가진다

Logical Array IR이 여러 route의 compiler boundary라면 단순 Rust struct 집합으로 끝내지 않는다.

<a id="571-verifier"></a>
#### 5.7.1 Verifier

각 operation은 생성/변환 후 최소 다음을 검증할 수 있어야 한다.

```text
structural verifier
type/dtype verifier
rank/shape verifier
region/block/terminator verifier
effect/token verifier
constraint/witness verifier
op-specific semantic verifier
```

invalid IR을 downstream optimizer가 추측해서 고치게 하지 않는다.

<a id="572-capability-interfaces"></a>
#### 5.7.2 Capability interfaces

분석/변환은 concrete op 이름의 거대한 switch보다 capability interface를 우선한다.

```text
ShapeInference
AxisAndIterationSemantics
AccessPattern
EffectSemantics
AliasSemantics
SpeculationSemantics
TilingCapability
BufferizationCapability
ExternalLoweringCapability
```

모든 op가 모든 interface를 구현할 필요는 없다. interface가 없으면 해당 optimization/route가 conservative하게 거부되거나 fallback 후보를 찾는다.

<a id="573-canonicalization과-rewrite-provenance"></a>
#### 5.7.3 Canonicalization과 rewrite provenance

canonicalization은 semantic-preserving rewrite만 포함한다. J-specific structure를 없애는 rewrite와 target-specific optimization을 같은 canonicalization 단계로 섞지 않는다.

각 nontrivial lowering/rewrite는 가능하면 source/semantic origin을 추적하여 differential debugging이 가능하게 한다.

<a id="574-ir-serializationversioning"></a>
#### 5.7.4 IR serialization/versioning

현재 개발 단계에서는 RustJ Logical IR의 장기 binary compatibility를 약속하지 않는다. 그러나 외부 tool/process와 IR을 교환하기 시작하면 schema version을 명시한다.

```text
IrSchemaVersion
PrimitiveRegistryVersion
producer/compiler version
feature set
```

portable artifact를 만들 경우 text/debug syntax와 portable serialization contract를 분리하고, version upgrade/downgrade 또는 unsupported-version 진단을 제공한다.

MLIR bytecode의 dialect versioning과 StableHLO/VHLO의 versioned portable artifact 방식이 참고 모델이다. compatibility를 약속하기 전에도 **version field와 verifier를 처음부터 두는 것**이 migration 비용을 줄인다.

<a id="575-cross-stage-negative-verifier-matrix"></a>
#### 5.7.5 Cross-stage negative verifier matrix

positive E2E test만으로는 compiler boundary를 보호할 수 없다. 각 stage는 “정상 plan이 통과한다”뿐 아니라 **그 stage가 책임지는 잘못된 상태를 반드시 거부한다**는 negative matrix를 가진다.

| Stage / verifier | 반드시 거부해야 하는 forged/invalid state | 현재/계획 상태 |
|---|---|---|
| J Graph `Plan::verify` | schema/primitive-registry mismatch, invalid ValueId/RegionId, stale region result/stage, malformed pipeline/fork/hook topology, source/fact/rule provenance drift | **현재 존재**. schema는 `J_GRAPH_SCHEMA_VERSION = 0.11`와 exact match |
| rewrite candidate `verify` | stale source span/basis, unregistered rule/witness mismatch, replacement DAG forward reference, fact-rule mismatch, output semantic facts drift | **현재 존재** |
| scan/fusion analysis verifier | forged source order, unsupported rule version, missing/incorrect witness, external-use/retention/fanout drift, candidate를 근거 없이 `selected`로 위조 | **현재 일부 존재**; proof discharge/selection verifier는 future |
| A3 `Plan::verify` | schema/registry mismatch, invalid op/value/block/region references, use-before-def, source/j_origin drift, malformed constraint/check/effect/error/speculation contract, result/write/terminator inconsistency | **현재 존재**. schema는 `A3_SCHEMA_VERSION = 0.6`와 exact match |
| CandidateEvidence / SelectionPlan | stale graph/version evidence, required proof Unknown인데 Selected, Illegal candidate 선택, overlapping incompatible candidates 동시 선택 | **planned** — §4.1.4 |
| RouteRegion / RouteBoundary | missing live-in/out, value-dead but effect-live dependency drop, SemanticCheck 중복/누락/순서변경, region-wide capability 미증명, guard가 effect 뒤에 배치, bridge requirement 누락 | **planned** — §2.5.1 |
| PhysicalPlan | invalid plan buffer/view/op id, use-before-bind, view span overflow, selected kernel capability mismatch, unordered Check, unproved writable overlap/reuse, dangling Return | **planned M4** — §5.2.1 |
| ExternalRegionPlan | source op 누락, check/effect edge drop, declared capability와 emitted op 불일치, incomplete output completion/ownership, unsupported partial module을 success로 표시 | **planned M6** — §5.5.1 |

negative test 이름과 타입은 구현과 함께 정하되, **검증 책임 자체는 stage contract의 일부**다. downstream이 invalid upstream artifact를 관대하게 보정하는 식으로 책임을 이동하지 않는다.

cross-stage forged test의 기본 패턴:

~~~text
valid source
  ↓ build valid artifact
clone artifact
  ↓ mutate exactly one invariant
stage.verify() must fail
  ↓
error identifies the violated boundary
  ↓
no later planner/executor is invoked
~~~

한 test에서 여러 invariant를 동시에 깨뜨리지 않는다. 어느 verifier가 어떤 invariant를 소유하는지 분명하게 유지한다.

<a id="576-serialization--schema-migration-policy"></a>
#### 5.7.6 Serialization / schema migration policy

현재 RustJ의 J Graph/A3는 주로 in-process compiler artifact이며 장기 portable serialization compatibility를 약속하지 않는다. 현재 verifier는:

~~~text
J Graph schema 0.10      exact match required
A3 schema 0.6           exact match required
PrimitiveRegistry       current REGISTRY_VERSION exact provenance required
~~~

를 기본으로 한다. **minor version이 다르다고 자동 호환으로 간주하지 않는다.** 외부 artifact reader가 생기기 전에는 exact-match fail-closed가 올바른 정책이다.

향후 저장/교환 format을 만들 때 다음 정책을 사용한다.

1. **Decode와 migrate를 분리한다.**
   - wire/file schema를 먼저 안전하게 decode한다.
   - source version별 explicit migration function이 있을 때만 current in-memory schema로 변환한다.
   - 알 수 없는 field/op/rule을 추측해 current 의미로 읽지 않는다.

2. **upgrade는 explicit chain만 허용한다.**

~~~text
v0.n artifact
   ↓ decode with v0.n schema
migrate_0_n_to_0_n1
   ↓
...
   ↓
current schema
   ↓
current verifier
~~~

migration 결과도 반드시 current verifier를 통과해야 한다.

3. **downgrade는 lossless writer가 있을 때만 허용한다.**
   - 새 semantic field/op/effect를 옛 schema가 표현하지 못하면 downgrade를 거부한다.
   - field를 조용히 drop해서 옛 artifact를 만들지 않는다.

4. **PrimitiveRegistryVersion mismatch는 schema mismatch와 별도다.**
   - primitive ID/contract mapping migration이 명시되어 있지 않으면 reject한다.
   - spelling이 같다는 이유만으로 semantic registry version을 무시하지 않는다.

5. **compiler version은 provenance, schema/registry가 compatibility key다.**
   - compiler version이 다르더라도 schema/registry+migration contract가 같을 수 있다.
   - 반대로 같은 compiler version 문자열만으로 compatibility를 보증하지 않는다.

6. **rule/witness registry도 versioned provenance를 유지한다.**
   - rewrite/fusion/scan witness meaning이 바뀌면 stale cached candidate를 재사용하지 않는다.

7. **PhysicalPlan portable cache는 별도 schema다.**
   - Logical IR schema와 같은 version으로 묶지 않는다.
   - target architecture/device/runtime/capability fingerprint를 함께 요구한다.
   - device-specific cached plan miss는 J error가 아니라 cache/route miss다.

8. **unsupported-version diagnostics는 semantic J error와 분리한다.**
   - `UnsupportedSchema/Registry/Migration` 계열 compiler diagnostic으로 보고한다.
   - Domain/Rank/Length 같은 J error로 위장하지 않는다.

pre-1.0 개발 단계에서는 schema를 자주 올릴 수 있다. 그 대신 version bump 없이 semantic field meaning을 바꾸는 것을 금지한다. portable artifact compatibility를 공식 약속하기 전에도 이 규율을 지킨다.

---
