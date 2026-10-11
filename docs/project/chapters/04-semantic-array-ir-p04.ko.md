[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)

> 문서 분할 안내: 기존 `PROJECT.ko.md`의 내용을 이동한 모듈입니다. 원문의 단일 파일 정본·신규 Markdown 금지 문구는 최신 [Agent 문서 규약](../../../AGENTS.md)보다 우선하지 않습니다.

# 3. J Semantic Array IR — 계속 (4/4)

### 3.8 sentence evaluation order와 namespace mutation

J의 parser는 conventional frontend처럼 “문장 전체 AST를 만든 뒤 모든 name을 한 번에 resolve”하는 것으로 의미를 모델링하면 안 된다. current jsource의 `p.c`는 queue를 stack하면서 name lookup, parse reduction, verb execution, assignment를 한 sentence 안에서 진행하며 J의 **우측→좌측 평가 의미**를 실현한다.

따라서 RustJ가 parser/evaluator 구현 방식 자체는 바꾸더라도 다음 observable order를 보존한다.

```text
sentence
  semantic right-to-left evaluation / reduction order
  + name lookup at the point required by J semantics
  + assignment/locale mutation at its semantic execution point
```

특히 다음 shortcut을 금지한다.

```text
문장 시작 환경을 snapshot
→ 모든 NameRef를 그 snapshot으로 resolve
→ 이후 assignment를 한꺼번에 commit
```

이 방식은 같은 sentence 안의 assignment, expunge, locale change, dynamic execution, side effect가 이후/이전 subexpression의 lookup에 미치는 의미를 바꿀 수 있다.

compiler IR에서는 이를 반드시 source-order instruction list로 복제할 필요는 없다. 대신:

- pure subexpression은 dependency/effect proof 뒤 graph로 재배열 가능
- name lookup, assignment, locale mutation, dynamic execute, I/O 등은 `EffectToken` 또는 동등한 semantic sequencing edge를 가져야 함
- noun value snapshot과 function nameref late lookup의 시점 차이를 보존
- assignment는 namespace write effect와 동시에 assigned J entity를 산출하므로 entity/value-flow와 effect-flow 양쪽에 나타남
- optimization이 name/effect boundary를 넘어갈 때 legality proof가 필요

즉 **J의 우측→좌측 parser implementation을 복제하는 것이 목표가 아니라, 그 구현이 만들어내는 observable evaluation/binding order를 IR에 보존하는 것**이 목표다.


---

<a id="dynamic-semantic-boundaries"></a>

### 3.9 동적 의미와 컴파일 경계 계약

이 절은 §3.4/§3.7/§3.8의 순서·이름 의미와 FOUNDATIONS §21/§33을 실행 경로 선택의 조건으로 모은 **설계 계약**이다. 구문별 영구 컴파일 금지 목록을 만들지 않는다. J-valid 여부, graph 구성/부분 분석 가능 여부, 특정 변환의 적법성, 특정 backend의 실행 capability는 서로 다른 판정이다. frontend가 실제 constructor/POS를 아직 확정하지 못하면 source·원인·시점의 경계를 보고하며 가짜 completed graph를 만들지 않는다.

#### 3.9.1 경계별 필요한 근거와 처리

| 경계 | 필요한 근거 | 검사·사용 시점 | 근거가 없거나 무효일 때 | 현재 구현 상태 |
|---|---|---|---|---|
| **DB-N noun 입력** | J가 읽은 noun snapshot과 dtype/rank/shape; contents 조건은 별도 | parser가 요구하는 noun read 시점. 외부 분석 metadata는 실제 입력 연결 시 별도 확인 | 확보한 facts만 분석. payload를 상수로 추측하거나 후속 binding으로 기존 noun을 교체하지 않음 | noun snapshot과 WI1 metadata 검사는 존재. WI1은 atoms·binding·effect·storage 안전성 또는 실행 계획 재사용을 증명하지 않음 |
| **DB-F 이름으로 호출하는 함수** | 실제 lookup 환경, 현재 POS/callable, 전문화가 사용한 binding의 안정성 | J parser-time POS lookup과 call-time late lookup을 각각 보존. 생성 시 복사한 rank/cap 의미는 별도 constructor 사실 | NAME/source DAG와 동적 호출을 보존. 안정성 없는 inline/CSE/hoist/전문화를 거부 | late NAME/POS 처리와 constructor snapshot seam은 일부 구현. catalog version은 runtime witness가 아니며 통합 guard dispatcher 미구현 |
| **DB-L local/locale 검색·변경** | 호출 frame, local binding 유무, current locale/path, locative와 검색 결과의 유효성 | 실제 의미적 lookup 지점 및 영향 있는 namespace/context 변경 뒤 | runtime lookup/write와 순서를 유지. 단일 referent version만으로 전체 검색 결과를 고정하지 않음 | local/public assignment와 modifier frame 일부 구현. full locale/locative/path guard는 미구현 |
| **DB-X 문자열 실행 `".`** | 문자열 내용과 실행 환경/POS/효과 계약. 상수 문자열만으로 purity를 증명하지 못함 | 실행 지점. 동적 문자열은 지원되는 shared frontend/JIT 경로가 실제 존재할 때 처리 | opaque runtime 경계. 임의 상수 치환이나 문자열만의 cache key로 실행 환경을 생략하지 않음 | general execute/JIT/cache route는 이 계약으로 지원을 선언하지 않음; 구현·검증 후 capability 등록 |
| **DB-C 값 의존 modifier/definition 생성** | 필요한 noun 값, constructor 성공/오류, 실제 result POS와 생성 사실 | 해당 parser reduction 지점. semantic runtime action이 이후 이름/POS 해석에 영향을 주면 그 뒤 파싱도 같은 상태를 사용 | 생성 경계에서 shared semantic parsing 요구. 실행하거나 생성하지 않은 결과를 추측하지 않음 | shared parser/runtime constructor와 capture 일부 구현. static 값 부족·full definition/control coverage는 경계 유지 |
| **DB-E 효과·관찰 가능한 오류** | namespace/resource read/write, I/O/context 변경, error/throw 및 catch 가능성, speculation 계약 | 원래 의미적 실행 순서. 재배치/병렬화/생략 전 legality 확인 | ordering barrier 유지. effect unknown을 pure로, unused result를 unused effect로 간주하지 않음 | 일부 runtime 순서·오류 회귀와 초기 effect 계약 존재. full effect graph/optimizer legality·dispatch 완료를 뜻하지 않음 |
| **DB-A 값 의존 결과·boxed/sparse·rank assembly** | 실제 연산의 type/shape/구조·empty prototype·fill/assembly 계약 | 필요한 facts를 알 때 분석하고 실행 시 잔여 조건 확인 | unknown 또는 conservative lowering. 검증되지 않은 uniform tensor/affine access로 축소하지 않음 | foundations와 일부 실행/분석 존재. general prototype·heterogeneous assembly·full boxed/sparse 경계는 계속 미완료 |

이 표의 runtime 처리는 **존재하고 검증된 RustJ capability에 한정**한다. 가능한 fallback이 항상 있다고 가정하지 않으며 C 엔진으로 넘기지 않는다. 지원 경로가 없으면 implementation coverage 경계로 보고하고 J-invalid 오류와 구분한다.

#### 3.9.2 허가하지 않는 변환과 guard 실패 계약

- 안정성 근거 없이 NAME을 inline하거나, 문장 시작 환경으로 모든 이름을 고정하지 않는다. `=.`/`=:`/locale 변경·dynamic execute가 lookup에 미치는 영향을 보존한다. unbound local에서 locale로 내려간 lookup도 이후 local binding 생성으로 무효가 될 수 있다.
- 검사한 binding/metadata가 **사용 시점까지 같은 의미를 갖는다는 근거**를 요구한다. region entry의 version 검사 하나로 충분하다고 가정하지 않는다. 필요한 변경 지점에서 다시 검사하거나, 분석으로 그 사이의 변경 불가능성을 증명한다. concurrency가 있으면 snapshot/lease/synchronization 등 실제 계약이 추가로 필요하다.
- constructor-fixed rank·cap 정보와 executable binding witness를 섞지 않는다. implicit operand를 fix하여 modifier를 재구성한 **새 entity**의 생성 사실은 원래 entity의 late lookup 변경과 다르다.
- topology만으로 fork를 병렬 실행하거나 효과·오류가 있는 호출을 CSE/hoist/speculate/delete하지 않는다. Graph IR을 보존·분석할 수 있다는 사실은 해당 변환의 허가가 아니다.
- specialization guard miss는 J semantic error가 아니다. effect 이전에 재분석하거나 지원 route를 선택한다. 명시 입력 계약 위반과 J 자체의 오류는 각각 별도 정책을 따른다. compiler 분석용 실패를 원래 J 오류 대신 사용자에게 먼저 노출하지 않는다.
- **effect 이후 문장 전체 자동 replay를 금지한다.** 후속 runtime 전환은 이미 수행한 write/I/O, live noun snapshot, frame/locale 및 parser queue/stack·reduction 위치를 포함하는 정확한 continuation이 구현·검증되었을 때만 허용한다. 그 전에는 effect 이전 경로 선택만 지원하고 중간 전환 capability를 선언하지 않는다.
- backend precondition miss는 그 route의 거부다. 의미를 검증하지 않은 다른 backend나 native path를 자동으로 성공 가능한 fallback처럼 사용하지 않는다.

#### 3.9.3 unknown 분류와 구현 경계

| 미확정 정보 | 가능한 정적 작업 | 추가로 필요한 조건 |
|---|---|---|
| shape/type/extent | 알려진 graph topology, 부분 shape/resource 식과 경계 보고 | 해당 kernel/메모리 분석이 사용하는 잔여 조건 |
| 함수 identity/POS 또는 constructor 결과 | source/provenance와 아직 확정 가능한 구조 보존 | 실제 lookup/constructor와 안정성 근거. POS가 미확정이면 completed parse를 추측하지 않음 |
| effect/error/alias | 보수적 ordering·reuse barrier 유지 | 해당 변환을 허용할 legality proof |
| physical layout/device/capability | Logical 의미와 route 후보 보존 | representation/target adapter 검증. logical facts에 physical 가정을 넣지 않음 |

현재 `GraphAnalyzability`의 Static / StaticWithUnknownFacts / RequiresSpecialization / DynamicSemanticFallback과 `AnalysisBoundary`는 초기 분류다. **Static은 pure/error-free/reorderable/executable을 뜻하지 않는다.** `validate_noun_inputs()` 성공도 함수·효과·전체 실행 안전성을 뜻하지 않는다. 위 분류를 full guard/continuation/dispatcher의 구현으로 오인하지 않는다.

#### 3.9.4 Fallback / guard miss / replay decision table

RustJ에서 `fallback`이라는 말을 하나의 의미로 쓰지 않는다. 최소 다음 네 개념을 분리한다.

~~~text
route fallback
  실행 시작 전 capability/precondition을 보고 다른 verified route를 선택

guard miss
  specialization/optimization 가정이 false임을 effect 전에 확인

replay
  이미 시작한 region/sentence를 처음부터 다시 실행

continuation / deopt
  이미 완료한 의미적 effect를 보존한 채 정확한 semantic point에서 재개
~~~

현재 RustJ가 architecture상 일반적으로 허용하는 것은 **실행 전 route fallback**이다. effect 이전 guard miss도 지원 capability가 실제 존재할 때 reanalysis/reselection으로 처리할 수 있다. 일반 replay나 exact continuation/deopt는 구현·검증된 capability가 아니다.

| 발생 상황 | J semantic error인가? | 다른 route 선택 가능? | replay 가능? | 현재 원칙 |
|---|---|---|---|---|
| compile/lowering 시 target capability miss | 아니오 | **예**, 아직 실행 전이고 verified alternative가 있으면 | 필요 없음 | route miss로 처리; 없으면 개념상 UnsupportedImplementation(현재 concrete API는 `Error::Unsupported`) |
| specialization guard miss, observable effect 전 | 아니오 | **예**, reanalysis/verified fallback route가 있으면 | 원칙적으로 재실행보다 새 route 선택 | guard miss는 J error로 노출하지 않음 |
| explicit compiler/API contract violation | J 자체 오류와 별개 | contract가 허용한 정책에 따름 | 자동 replay 아님 | 잘못된 user/compiler contract와 J Domain/Rank 등을 구분 |
| A3 `SemanticCheck` 또는 semantic call이 내는 J Domain/Length/Rank/Index 등 | **예** | 아니오. 다른 backend로 바꿔 같은 J error를 회피하지 않음 | 아니오 | 원래 error class/precedence를 보고 |
| backend adapter precondition miss, region 실행 전 | 아니오 | **예**, verified alternative가 있으면 | 필요 없음 | 해당 route만 거부 |
| native/external implementation Unsupported, 아직 어떤 observable effect도 시작 전 | 아니오 | **조건부**. 전체 region이 untouched이고 대체 route가 검증된 경우만 | v0에서는 route 재선택으로 처리 | implementation coverage 경계 |
| kernel/external route가 일부 실행된 뒤 implementation failure | 보통 J semantic error가 아님 | 자동으로는 아니오 | **기본 금지** | cleanup 후 implementation failure/Unsupported를 보고; exact transactional contract 없이는 replay 금지 |
| namespace write/I/O/error-observable effect가 commit된 뒤 실패 | 경우에 따라 별도 J error가 이미 관찰 가능 | 자동으로는 아니오 | **금지** | exact continuation이 구현·검증되기 전에는 중간 fallback capability를 선언하지 않음 |
| async route가 일부 completion/token을 발행한 뒤 실패 | 자동 J error 아님 | token/resource 상태를 완전히 증명할 때만 | 기본 금지 | future async contract가 completion/effect frontier를 소유해야 함 |

##### `RuntimeSemanticFallback`의 정확한 뜻

현재 `lowering.rs::RouteDecision::RuntimeSemanticFallback`은:

> **이 operation에 현재 native ExecutionBasis realization이 없으므로 semantic/runtime route가 필요하다는 compile-time 분류**

다. 이것은 “native kernel을 실행하다 실패하면 언제든 interpreter로 되돌아간다”는 runtime deoptimization 보장이 아니다. 실제 runtime semantic executor가 해당 form을 지원하지 않으면 최종 결과는 개념상 `UnsupportedImplementation`, 현재 concrete API로는 `Error::Unsupported(...)` / kind `"unsupported"`일 수 있다.

##### fallback commit frontier

future dispatcher는 최소한 다음 frontier를 추적해야 한다.

~~~text
before_start
  아무 observable effect/consumer-visible transfer도 없음

guarded_but_uncommitted
  guards/checks는 수행했지만 replay-sensitive effect 없음

committed
  namespace write / I/O / externally visible mutation / non-rollback transfer 등이 발생
~~~

`before_start`와 `guarded_but_uncommitted`에서는 verified alternate route 선택이 가능하다. `committed` 이후에는 exact continuation/transaction rollback 증명이 없는 한 region-start replay를 금지한다.

##### 검증 요구

- guard miss가 J semantic error code로 바뀌지 않는 test
- effect 이전 route miss가 alternate verified route를 선택하는 test
- SemanticCheck 실패에서 backend 변경이 error를 숨기지 않는 test
- namespace write 뒤 Unsupported를 강제로 만들고 write가 두 번 실행되지 않는 negative replay test
- async/transfer를 도입할 때 completion frontier 이전/이후 실패를 분리하는 test

이행 항목의 정본은 [§10 DB0–DB7 체크리스트](./11-roadmap-checklists-p01.ko.md#dynamic-boundary-checklist)에 둔다. 검사 대상과 구현 상태는 위 계약을 따른다.

#### 3.9.5 경계 판정 알고리즘과 검증 수용 조건 — 첫 CPU route (2026-10-07)

**적용 범위:** §3.7.6의 전달 계약을 첫 CPU route의 실행 전 admission에 적용한다. 일반 J의 언어 제한이 아니며, 다음 판정의 `Need...`는 J 오류가 아니라 **이 compiler route의 정보/구현 요구**다. 현재 `StaticAnalyzer`의 Unsupported/AnalysisBoundary는 일부 seam만 구현하며 아래의 구조화된 admission 결과와 dispatcher는 아직 없다. 문서만으로 실행 허가를 발행하지 않는다.

**판정 지점은 다음으로 고정한다.** 같은 9-row matcher를 쓰되 각 action 직전에 아래 정보를 검사한다. Deferred noun의 dtype/shape가 Unknown이라는 이유만으로 값을 계산하지 않는다.

| 판정 지점 / 코드 seam | Continue 조건 | 충족하지 못하면 반환할 요구 |
|---|---|---|
| queue NAME → stack, `resolve_stack_item` | 현재 의미적 lookup에 해당하는 POS가 입력 계약/증거로 정해졌고 앞선 namespace effect가 그 전제를 바꾸지 않음 | `NeedNameClass`: occurrence·lookup context·선행 effect 의존성. 누락된 catalog 항목을 J Value 오류로 바꾸지 않음; 실제 미정의 NAME의 J fallback과도 구분 |
| row 0–2, `runtime_noun`의 **분석 경로** | callable POS/valence와 noun-result 구조를 알 수 있음 | 기본은 `Expr::Monad/Dyad` 생성. 값 자체를 요구하지 않음. callable target 미확정이면 NameRef를 유지하고 별도 `NeedCallableProof`로 실행 admission을 보류 |
| row 3/4 및 noun-left fork/gerund/definition 생성, `completed_noun`/`CompletedParseResult`/`resolve_modifier` | constructor에 실제 필요한 operand가 concrete이거나 검증된 symbolic constructor가 의미·result POS·오류 조건을 완전히 표현 | `NeedConstructorValue` 또는 `NeedConstructorResult`: 필요한 operand/use, 검사할 값·POS, constructor identity. 현재 computed rank operand `(1+0)`도 이 분석 경계에 해당; 향후 검증된 constant evaluation은 이 요구를 해소할 수 있음 |
| row 7 대입, `apply_parse_row` | 단일 정적 target의 **최종 outer assignment**, RHS는 지연 graph에 남고 commit은 실행 성공 후 | 중간/계산된/복수 target·local/locale 변화는 `NeedNamespaceAction`. 현재 분석기는 non-final assignment에서 중단. symbolic namespace forwarding이 검증되기 전 기존 catalog로 후속 이름을 읽지 않음 |
| `".`·미확정 explicit modifier/opaque call 등 | 분석기가 해당 source/POS/효과 계약을 실제로 보유 | `NeedDynamicSemantics`: dynamic source/constructor/call와 원 위치. 임의 pure 가정 금지 |
| graph→A3→physical admission | 모든 요구가 해소됐고 실제 route가 지원되며 name/POS·nested-call 효과·error order·입력/수명 계약이 검증됨 | `NeedsGuardOrRoute`/`UnsupportedRoute`. graph가 만들어졌다는 사실만으로 실행하지 않음 |

제안 admission 결과에는 `kind | span/word/occurrence | parser row/use | required value/POS/callable | dependency/effect frontier | available evidence | selected action`을 담는다. **현재 Rust enum으로 구현됐다는 뜻은 아니다.** `Continue`는 graph 구성 허용이고 `ExecuteAllowed`는 별도 최종 판정이다. POS만 알아도 graph를 만들 수 있지만 함수 target·본문의 이름/효과·오류 계약 없이는 inline/fusion/native 실행을 허가하지 않는다.

**v0 dispatch는 실행 전에 한 번 결정한다.** 후보 문장 전체를 실제 workspace를 수정하지 않고 분석한다. 내부 namespace write/unknown-effect call/동적 구성 요구가 없고 필요한 proof·guard 및 target capability가 닫힌 구간만 compiled route에 수용한다. 유일하게 허용한 terminal write도 실제 commit은 실행 후다. 요구가 남으면 원 source를 **처음부터 아직 실행하지 않은 상태로** 지원되는 runtime route에 넘기거나 compiler coverage 거부를 반환한다. Runtime route의 실제 capability 확인은 필수이며 C fallback은 없다. Guard miss는 route 거부이지 J Domain/Value 오류가 아니다.

**v0에서 하지 않는 일:** prefix 실행 후 parser 중간 재개, 효과 이후 문장 전체 replay, 계산하지 않은 operand를 concrete JEntity로 운반, backend가 미지원인 call을 존재하지 않는 fallback에 위임. 따라서 처음에는 `a+a=:2` 같은 구간을 native compile하지 않아도 되지만, 지원되는 runtime 경로에서는 반드시 J 결과 `4`, 최종 `a=2`를 낸다. 이후 그 form의 compiler 수용은 ordered symbolic namespace/write와 verifier가 구현됐을 때 확장한다. 일반 continuation은 별도 후속 capability이며 state snapshot+exactly-once 테스트 전에는 활성화하지 않는다.

**수용은 세 층으로 분리한다.** 다음은 기존 NP/DB/M3→M4 게이트의 테스트 상세이며 경쟁 checklist가 아니다.

| 층 / 연결 게이트 | 통과해야 하는 관찰 | 반드시 실패시킬 단일 변이 | 현재 증거 |
|---|---|---|---|
| Analysis boundary / NP-02/04, DB-C | pure apply·unknown shape는 지연 graph, computed constructor·non-final write는 boundary, 분석 중 binding commit 없음 | deferred operand를 literal로 위장하거나 중간 write 뒤 이전 catalog read로 계속하기 | `tests/static_analysis.rs`의 기존 14 + 신규 3 회귀. API는 아직 구조화된 demand를 반환하지 않음 |
| Plan admission verifier / NP-03/05 | NAME/POS/constructor·입력·call effect 증거의 범위/시점, complete source mapping, 실행 전에 해소된 요구, error order와 수명, 지원되는 route | 하나의 guard 삭제, 다른 locale/frame stamp로 교체, nested NameRef dependency 누락, order edge 제거, 미해소 demand를 Ready로 변경 | **미구현/UNRUN**. `Plan::verify`의 구조 검사나 catalog version만으로 이 게이트를 대체할 수 없음 |
| C↔Rust 실제 compiled route / NP-07, DB-E, M4 | 값의 type/shape/data, 결과 POS·첫 오류, 효과 전후 namespace·alias와 source 관찰이 C/runtime/compiled 경로에서 동일 | `a+a=:2`를 3으로 계산, missing-left 오류를 right Length보다 먼저 노출, committed count를 0 또는 10으로 만들기, rebind 후 stale 함수 실행 | C↔현재 runtime 일부는 실행. **compiled route 및 NAME event/내부 effect trace 차분은 UNRUN** |

**oracle fixture를 먼저 고정했다:** [boundary audit](../../../tools/name_compatibility_audit.py)의 `--boundary-fixtures-only` 및 [Windows 결과](../../../reports/semantic-boundary-windows.json). 다음 10 fixtures × C j64/AVX2 × Rust direct/semantic-reference = **40건: 36 matched, 4 unsupported_gap**. 두 runtime 경로는 컴파일 경로가 아니다. Source와 DLL asset pin은 별개이고 실제 DLL/binary hash를 기록한다.

| fixture / 핵심 관찰 | C와 현재 runtime | compiler 수용에서의 요구 |
|---|---|---|
| `b=:a+a*a`, 기존 a 유지 | `[0,2,6]`, a=`[0,1,2]` 일치 | graph→A3에서 중간 배열 미실행; alias 보존; 실제 route 검증 |
| `n=:4` 뒤 `i.n` | `[0,1,2,3]` 일치 | unknown result shape는 parser value demand가 아님 |
| `(+/"1) i.2 3` / `(+/"(1+0)) i.2 3` | 둘 다 `[3,12]` 일치 | literal은 구성 가능, computed operand는 현재 analysis boundary |
| `a=:1` 뒤 `a+a=:2` | 결과 4, a=2 일치 | v0 중간 namespace write 경계; 기존 a=1로 parse를 계속하지 않음 |
| `adv=:1 : 'a=:u'` 뒤 `a+2 adv` | 결과 4, a=2 일치 | modifier 실행이 후속 lookup을 바꾸는 경계 |
| `missing+(1 2+1 2 3)` | **Length 오류**, missing Value보다 먼저 | semantic error order를 유지; admission miss가 J error를 먼저 발생시키지 않음 |
| `count=:0`, `adv=:1 : 'count=:count+u'`, `(1 2+1 2 3)+5 adv`, `count` | Length 오류 후 **count=5** 일치 | effect-before-error 보존, 실행 누락으로 0 또는 replay로 10을 만들면 실패 |
| `". '1+2'` | C=3, Rust Unsupported | dynamic semantic route 자체 미지원; 자동 fallback 성공 주장 금지 |
| `f=:+`, `g=:f`, `f=:7`, `g 3` | Domain 오류 일치 | POS/target guard와 정확한 semantic error/route-miss 구분 |

여기서 검증한 것은 **경계 입력에 대한 기존 runtime의 외부 동작**과 **현재 분석기의 중단/지연 동작**이다. 재개 state, typed demand API, admission verifier 및 compiled route가 통과했다고 기록하지 않는다. 후속 구현의 첫 완료 기준은 pure 입력 한 구간에서 `Deferred Program -> Graph -> A3 -> admitted CPU execution`이 kernel-call counter/할당 계측·C 값 비교로 닫히는 것이며, 이후 위의 한-변이 부정 테스트로 각 경계를 확장한다.

이번 Windows 검증: `static_analysis` **17/17 default 및 17/17 portable**, Python audit 분류 **6/6**·oracle protocol **9/9**, fmt·clippy(`--all-targets --all-features -D warnings`)·diff 검사 통과. 전체 Rust/upstream J suite·Linux·GPU·GitHub CI는 실행하지 않았다. NAME 처리의 위험을 후속 최적화에 떠넘기지 않기 위해, **admission verifier가 구현되고 위 부정 테스트를 통과하기 전에는 NAME 관찰 metadata만으로 특수화 실행을 수용하지 않는다.**

근거: [FOUNDATIONS §21/§33](../../../FOUNDATIONS.ko.md), pinned C [p.c parser](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c), [sc.c NAME constructor](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sc.c#L364), [cx.c return fix](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L684), [af.c reconstruction](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/af.c#L193). DB0–DB7의 분류·이행 순서는 RustJ 설계 판단이며 upstream의 완성된 guard 시스템을 복제했다는 뜻이 아니다.

<a id="read-graph-execution"></a>

---

[← 이전 부분](./04-semantic-array-ir-p03.ko.md)
