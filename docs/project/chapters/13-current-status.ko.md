[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)


<a id="current-implementation-status"></a>

<a id="read-status"></a>
<a id="12-현재-검증구현-상태-요약"></a>
## 12. 현재 검증·구현 상태 요약

이 절의 항목은 architecture·migration 상태를 개괄하는 목록이며 **2026-10-05 시점까지 축적한 기준선**이 포함되어 있다. 최신 실행 증거는 도입부의 **2026-10-08 Windows 기록**과 바로 뒤 **상세 구현·검증 기록**, §10 체크리스트를 함께 확인한다. 이력상의 테스트 수치와 단계별 gate를 현재 HEAD의 검증 완료 주장으로 읽지 않는다.

- 제한된 CPU J interpreter/runtime 경로가 동작한다.
- state-table word formation과 transitional Semantic IR parser가 존재한다.
- parser-produced shared `FunctionEntity`가 primitive, modifier application, hook/fork/train, rank/@: 구조를 보존한다.
- explicit/direct frontend는 immutable DefinitionCode, 제어/NAME metadata, multiple root/raw noun DD, source provenance를 보존한다. 지원 mode-3/4 verb 호출과 if/while/for/try, 중첩 direct/문자열 explicit의 독립 local scope 및 A3 정의 참조가 구현됐다. 현재 범위 E2E 완료와 full J 표현력/일반 locale·operator-wrapper/본문 Graph·Logical 분석 및 CFG 컴파일/본문-caller source-frame 미완료를 구분한다. 최신 경계와 오류 증거는 Frontend E2E 재감사 절을 따른다.
- J Graph IR이 별도 canonical analysis surface로 존재하고 Graph Basis, structural opportunity, graph rewrite/resource analysis 기초가 구현되어 있다.
- **M1 완료:** J Graph lowering이 `logical_ir::Plan`을 직접 생성한다. transition module/container/API는 제거했고 `Engine::analyze/analyze_a3`와 `CompilationAnalysis.logical`은 같은 canonical plan을 사용한다.
- A3-v0에는 SSA ValueId, Function/Region/Block/Return, Execution Basis payload, SemanticCheck, ConstraintSet/FactWitness, Effect/Speculation/PossibleErrors/DestinationRelation, verifier가 구현되어 있다.
- `SemanticCapabilityView`, `ParameterizedLoweringRecipe`, `LoweringRegistry`, 기본 target legality/candidate generation과 contiguous route partition prototype이 구현되어 있다.
- 현재 RoutePartition은 class + operation range 중심의 prototype이며 boundary values/preconditions/chosen external route/bridge representation은 아직 없다.
- `logical_executor::execute_closed`는 A3 correctness/reference executor이며 native Physical Executor는 아니다.
- Logical/Physical Array 분리 원칙은 문서와 테스트로 고정되어 있고, G1 read-only CPU affine `PhysicalArray`/BufferId/BufferLease가 구현되어 있다.
- `Value`의 dense payload가 아직 `CpuStorage`를 직접 소유하므로 runtime carrier는 완전한 logical/physical 분리 이전의 migration state다.
- `facts::RepresentationClassFact`는 Dense/AxisSparse J-visible representation class만 나타내며, stride/offset/device/buffer 같은 physical layout은 포함하지 않는다.
- sparse/boxed/packed-bit 기반 구현이 일부 있으나 semantic representation과 concrete backend encoding 경계는 추가 정리가 필요하다.
- G2~G5와 Schedule/Physical Planner/Physical Execution Plan/CPU native executor는 미완료다.
- frontend는 동일 ordered 9-row matcher와 runtime/analysis reduction engine을 사용하며 과거 flat modifier/train heuristic reducer는 제거했다. 지원 범위의 name/POS/assignment와 completed-result 경계가 구현되었지만 전체 enqueue/construction/local·locale·definition semantics의 M2 완료 gate는 남아 있다.
- 최신 frontend/numeric 검증은 **NV3d2b2a**다: Windows default/portable 각각 **474 passed / 17 ignored**, fmt/clippy/build 통과, Python **30 passed**. j64/AVX2의 기존 세 runtime 경로는 각각 **5,380 / 5,380 passed / failed 0**, stage **10,810**, words **6,623**을 유지한다. 양 DLL numeric syntax는 각각 **2,485 cases / failed 0**이며 accepted noun controls 182, lexical-error equality 1,244, valid payload boundary 850, integer conversion boundary 2, C reference precision boundary 200, quad construction boundary 1, NaN word-formation boundary 4를 기록한다. unresolved recognition/error 경계가 각 1건 남아 있으므로 이를 성공 실행이나 정확한 오류 동등성으로 세지 않는다. vocabulary POS 143 / bare binding·AR 140 / noun payload 3, capture graph 257, static 2, runtime prefix 285 / executable prefix passes 0은 별도다. 최신 graph-readiness 검증은 **GF6a(463 passed / 17 ignored)**이며 semantic-proof discharge·fusion selection·성능·GPU 실행 완료를 뜻하지 않는다. full upstream·definition acceptance·private C trace·Linux/GitHub CI/CUDA는 여전히 미검증/보류다.
- MLIR adapter, StableHLO adapter, ArrayFire external route는 아직 참고/설계 단계다.
- TargetProfile/CostProfile/ResourceEstimate/CostEstimate의 완전한 구현은 아직 없다.
- 실제 CUDA storage/kernel은 없다.
- **Linux CI/GitHub Actions 결과는 별도 요청이 없으면 구조 진행 판단과 완료 gate에서 생략한다.** 로컬/명시적으로 실행한 검증만 완료 기록에 사용한다.

기계 측정 원자료는 `reports/*.json`, `reports/*.jsonl`에 보존한다.

---


<a id="상세-구현검증-기록-2026-10-07"></a>
### 상세 구현·검증 기록 (2026-10-07)

여기부터는 설계 개요가 아닌 시점별 구현·실험 증거다. 최신 지원 범위와 합격 기준은 위 §12 요약 및 §10·§11 정책을 우선한다.

<a id="definition-호출과-frontend-e2e-확인-2026-10-07"></a>
#### Definition 호출과 frontend E2E 확인 (2026-10-07)

이 절은 과거의 "17 ignored" 기록보다 최신인 definition 실행 상태다. 처음 17개 수용 테스트를 모두 강제로 실행했을 때 **0 passed / 17 failed**였다. `DefinitionCode`와 modifier용 프레임이 존재했지만 일반 mode-3/4 verb가 호출 실행기로 연결되지 않았으므로 일반 explicit/direct verb 지원으로 계산하면 안 되는 상태였다.

- [x] 일반 explicit/direct verb 및 ordinary alias의 monad/dyad 호출을 공유 정의 실행기에 연결한다. 인자는 호출 전에 원래 parser 순서로 평가하고, 본문은 호출별 별도 `LocalFrame`에서 실행한다.
- [x] `=.` 지역 대입, `=:` 전역 대입, noun snapshot, 호출 시 전역 noun/verb 재조회, caller의 사적인 local을 캡처하지 않는 조회, 실패 후 프레임 복원을 검증한다. ordinary mode-3 verb의 미결합 `u`/`x`는 modifier 인자 누락으로 오판하지 않고 전역으로 fallback한다.
- [x] 기존 `ControlNode`의 감사된 jump를 사용해 `if./elseif./else.`, `while./whilst.`, `break./continue.`, `return.`, `try./catch./catchd.`를 실행한다. T-block 결과와 마지막 B-block 결과를 분리하며, 초기/오류 후 결과는 C의 Boolean 빈 행렬이다. `Unsupported`는 J 예외처럼 catch하여 성공으로 바꾸지 않는다.
- [x] 원래 수용 테스트 17개 중 **14개**를 통과시키고 ignore를 해제한다. C의 첫 atom 조건 판정·빈 결과·중첩 catch·scope/effect/error 복원 회귀 테스트 **4개**를 추가한다.
- [x] `examples/frontend_e2e.rs`에서 실제 tokenizer, enqueuer, 분석용 `Program`, 실행 parser의 `FrontendContext`/NAME/환원 기록과 결과를 관측하고 verifier를 통과시킨다. 분석이 binding을 commit하지 않으며 local `t`가 유출되지 않음을 확인한다.
- [x] `for./for_name.` 실행: leading-axis item, scalar/빈 iterator/zero-atom row, 읽기 전용 `_index`, noun snapshot, break/continue/예외 시 해제를 구현한다. C forinit 오류의 catch 불가 경계도 보존한다. named sparse iterator는 명시적 Unsupported이며 item 전달은 현재 소유 배열 복사다; zero-copy/GPU 실행 완료가 아니다.
- [x] 중첩 direct 및 문자열 explicit definition construction과 독립 local scope를 구현한다. 기존 input framer로 multiline unit을 수집하고 내부 control/colon/name을 outer preparse에서 격리한다. inner local은 caller-private local을 캡처하지 않으며 전역 NAME은 호출 시 재조회한다. 본문 안에 colon-zero block을 삽입하는 표기는 C와 같이 syntax error다; 외부 입력 스트림 소비 기능의 구현으로 계산하지 않는다.
- [x] 정의 값의 A3 callable projection: `VerbReference(Callable { target: Definition, semantic: Arc<FunctionEntity> })`가 immutable `DefinitionCode`와 이름/원문 정보를 보존한다. 배열 literal/type/shape를 꾸며내지 않고 분석 시 binding/body effect를 실행하지 않는다. verifier가 target/semantic 불일치와 Definition target의 직접 SemanticCall을 거부한다. **정의 본문의 CFG lowering/compiled execution은 미완료**이며 호출 분석의 structural lowering 경계는 유지한다.

**실제 전달 구조:** 분석 경로는 `Program { source, assignment, assignment_source, expression, frontend, reductions, ... }`이다. `expression`은 `Literal/ReadName/Monad/Dyad/VerbValue/...`이며 `frontend: Arc<FrontendContext>`가 expanded words/flags/span, items, semantic node links, origins, NAME observations, stack/reduce steps와 root를 연결한다. 관측 경로의 `CapturedEvaluation { result, capture }`는 실제 실행 결과와 같은 runtime parser의 context를 함께 제공한다. 관측 sidecar는 독립 실행 가능한 AST/continuation이 아니다. runtime parser는 noun을 실제로 reduce하며, 분석 경로의 지연 표현식을 runtime 결과로 바꿔 설명하지 않는다.

**Definition 경계:** 아래 두 정의는 생성 시 본문을 실행하지 않고 `VerbValue(FunctionEntity { head: ExplicitDefinition(Arc<DefinitionCode>), result_pos: Verb, ... })`를 만든다. `DefinitionCode`는 원문/form/span, decoded body, valence ranges, queued body words/flags, control nodes, `DefinitionNamePlan`을 보존한다. local `t`는 `LocalAssignmentTarget`; `y/g/t` 읽기는 `ReadCurrentFrameThenGlobal`이다. 이는 이름/제어의 preparse 결과이며 **본문 전체의 최적화 가능한 AST를 미리 완성한 것이 아니다**. 본문은 호출 시 공유 runtime parser로 해석된다. outer capture는 본문의 모든 내부 NAME event를 재귀적으로 export하지 않는다.

```j
a=:1 2 3
a+2*3                        NB. 7 8 9
g=:10
explicit=:3 : 0
t=.y+g
t
)
direct=:{{ t=.y+g
t }}
explicit 2                   NB. 12
direct 2                     NB. 12
g=:20
explicit 2                   NB. 22
direct 2                     NB. 22
pair=:4 : 'x+y'
2 pair 3                     NB. 5
ddpair=:{{ x+y }}
2 ddpair 3                   NB. 5
```

`a+2*3`의 분석 결과는 `Dyad(Add, ReadName("a"), Dyad(Multiply, Literal(2), Literal(3)))`이며 runtime의 noun NAME은 stack 시 snapshot한다. explicit outer queue는 `Name Assignment Noun(3) Conjunction(DefinitionConstructor) Noun(body)`다. direct queue에는 `(9 : body)`에 대응하는 괄호/constructor words가 생기며, `Conjunction → Parenthesis → Assignment`를 거쳐 이 예에서는 mode 3이 추론된다. `explicit 2`/`direct 2`의 분석 결과는 `Monad(NameRef(name), Literal(2))`이고 실제 NAME 관측은 `LateAtCall / FunctionReference / RuntimeClass`다. 두 dyad 예제는 `Dyad(NameRef(name), Literal(2), Literal(3))`이다.

native Windows 재현: `cargo run --example frontend_e2e`. 원자료는 `reports/frontend-e2e-windows.json`, C와의 bounded 비교는 `reports/definition-calls-windows.json`에 보존한다. 시연 14문장 결과는 C j64/AVX2 × direct/semantic-reference 네 경로와 일치했다. 확장 정의 사례 **31 fixtures / 124 observations / 124 matched**, 기존 NAME scope **10 fixtures / 40 observations / 40 matched**다. Python harness **67 tests**를 통과했으며 Windows 경로 quoting helper의 slash 정규화도 바로잡았다. CUDA/Linux/GitHub CI/전체 upstream 동등성 및 최적화된 정의 실행 완료를 주장하지 않는다. parser 중첩 기반 재귀 깊이는 현재 8로 제한되며 explicit 실행 프레임으로의 전환은 후속 작업이다.

**최종 회귀 검증:** Windows default/portable 각각 **596 passed / 3 ignored / 0 failed**. fmt와 clippy `--all-targets -D warnings`를 통과했다. 미완료 3개를 강제 실행하면 for/nested/A3 경계에서 실패하며 성공으로 계산하지 않는다. 보고서 3개의 source/binary hash는 최종 default build와 대조했다.

<a id="definition-반복중첩a3-함수-값-후속-검증-2026-10-07"></a>
##### Definition 반복·중첩·A3 함수 값 후속 검증 (2026-10-07)

위 세 실행 단위를 진행해 기존 `definition_acceptance` 21개 모두의 ignore를 해제했다. 반복문 회귀 5개와 중첩/함수 참조 회귀 4개를 추가했다. Source → Program/FunctionEntity → J Graph → A3의 함수 값 전달과 본문 CFG 실행은 구분한다. 현재 호출은 공유 runtime parser를 사용하며 최대 중첩 깊이 8이다. select/case/fcase, catcht/throw, goto/label, 일반 locale/locative, compiled CFG 및 CUDA 실행은 후속이다. 이전 절의 596/3 기록은 당시 결과다.

검증 원자료는 `reports/definition-loops-windows.json`과 `reports/definition-nested-windows.json`에 기록한다. 각 보고서는 두 Windows J DLL × 두 Rust evaluator 경로의 bounded 관측이며 upstream 전체 동등성 증명이 아니다. Windows `cargo test --all-targets`의 기존 comparison bench는 Linux `dl.lib` 링크 의존성 때문에 실행 불가다. 일반 Windows tests, portable tests, clippy 정적 검사와 구분해 기록한다.

최종 native Windows 검증: default/portable 각각 **608 passed / 0 ignored / 0 failed**, Python **67 passed**, fmt/clippy 통과. C 두 DLL × direct/semantic-reference 비교: for **23 fixtures / 92 matched**, nested **12 / 48**, 기존 definition **31 / 124**, NAME scope **10 / 40**, 합계 **304 observations / 304 matched**. 네 C 보고서의 source/binary SHA-256를 최종 default build와 대조했다. frontend-e2e-windows.json은 이전 시연의 역사적 capture이며 이번에 재생성하지 않았다.


<a id="frontend-e2e-재감사-표현력후속-전달오류-2026-10-07"></a>
#### Frontend E2E 재감사: 표현력·후속 전달·오류 (2026-10-07)

사용자가 확정한 **현재 지원 범위의 frontend E2E 완료**는 유지한다. 이는 full J 표현력이나 모든 Program의 J Graph/A3 admission, definition 본문 CFG/compiled execution 완료를 의미하지 않는다. verb 본문의 계산/구조화 영역은 후속 J Graph/Logical 단계에서 구체화하고 필요한 경로에서 CFG로 낮춘다. frontend에 완성 CFG를 요구하지 않는다. 이번 감사는 runtime 변경이 아니라 실제 경계와 필요한 계약의 재확인이다.

<a id="실제-범위와-재현"></a>
##### 실제 범위와 재현

`examples/frontend_contract_probe.rs`는 P=prepare_semantic_diagnostic, G=J Graph, L=CompilationAnalysis/A3의 독립 비실행 검사를 수행한 뒤 R=eval_captured를 수행한다. 분석의 성공을 실행 성공으로 세지 않는다. `tools/frontend_contract_audit.py --assets-root ../rustj-project-docs --probe target/debug/examples/frontend_contract_probe.exe --report reports/frontend-contract-audit-windows.json`으로 Windows 두 J DLL과 비교한다. 원자료에는 각 단계 admission, error context/render, 실제 outer capture, setup 및 오류 후 state를 포함한다.

**31 사례 × DLL 2 = 62 관측: runtime/post-state 38 matched / 24 runtime_gap.** 12개 사례의 알려진 미지원 차이를 통과로 계산하지 않았다. complex/extended/rational 3개는 C bridge가 값을 직렬화하지 못해 **C 수용 여부와 noun type만** 확인했다. 그 외는 값/오류 종류와 setup/post-state를 비교했다. C의 diagnostic 위치/문구/내부 parser state와의 동등성은 이번 감사에서 검증하지 않았다. P는 15 accepted / 14 Unsupported / syntax 1 / control 1, G는 14 / 15 / 1 / 1, L은 13 / 16 / 1 / 1이며 전체 upstream 비율로 일반화하지 않는다.

위 38/24와 단계별 수치는 보완 전 감사 기준선이다. 문자열 target 구현 후 최신 결과는 아래 F1/P4 실행 체크리스트에 기록한다.

| 질문/사례 | 실제 판정 | 소유 단계 |
|---|---|---|
| jsource의 전체 표현력 | **아직 아님**. 문자열 단일·다중/computed target과 bounded noun/verb/adverb/explicit conjunction abandon 및 nameless conjunction 이름 이관 실행을 추가했다. boxed/AR target, locative, nameless conjunction abandon의 직접 적용, deferred effect lowering, complex/extended/rational literal conversion 등은 미지원; decimal overflow는 아래 N1에서 보완 | F1/P4 및 기존 숫자·NAME 호환성 게이트; 아래 noun target·abandon 실행 체크리스트 참조 |
| `a+a=:2`, `a=:b=:1`, 계산된 rank | runtime은 C와 일치하나 비실행 P/G/L에서 Unsupported. chained assignment 전체가 runtime 미지원이라고 말하면 틀린다 | P 단계 동적 construction/effect 경계, P8 |
| `adv=:/` | Program의 ModifierValue와 POS는 보존하지만 J Graph가 modifier value lowering을 거부 | P8/A1/A2, frontend lexical 오류 아님 |
| `". '1+2'` | P/G는 수용하고 L/runtime은 미지원. primitive 인식과 실행 지원을 분리 | lowering/runtime capability |
| explicit/direct/nested 정의 값 | P/G/L 수용. L은 Definition 함수 참조이며 본문 계산 IR/CFG는 아님 | definition code 보존 → P8/A1/A2/A3 |
| select 등의 제어문 | preparse metadata 수용과 실제 호출 가능성은 별개. 이번 select 사례는 **정의 생성만** 검사 | 후속 definition semantics/lowering |

<a id="후속-단계에-전달되는-것과-계약의-한계"></a>
##### 후속 단계에 전달되는 것과 계약의 한계

| 정보 | 현재 산출물/근거 | 후속 소비 조건 |
|---|---|---|
| 환원 결과와 환원 전 맥락 | Program.expression + Arc<FrontendContext>의 words/items/nodes/origins/NAME/reductions/steps/root | Program/FunctionEntity가 semantic authority. sidecar를 별도 실행 AST/continuation으로 사용하지 않는다 |
| 함수의 J 구조 | FunctionEntity의 head/POS/operands, noun operand 값·span, hook/fork/derived identity 및 header ranks | source modifier 구조를 보존하고 이후에 분석·정규화한다. rank/POS snapshot은 late target 고정 증거가 아니다 |
| 이름 | NamePolicy(CaptureAtRead/LateAtCall/ResolveAtConstruction), NameEvidence, runtime LookupObservation(frame/global instance, binding generation/version/POS) | catalog/runtime 관측과 executable guard를 구분한다. 현재 guard는 simple name 한정이며 locale/path witness가 아니다 |
| 정의 본문 | DefinitionCode의 원문·decoded body·form/mode/valence, sentence/word span·class·flags, ControlNode와 name_plan | body words는 payload/본문 AST 전체가 아니다. 후속 단계가 정의별 문장 분석을 수행할 수 있는 입력이며 재-enqueue/semantic parse 경계를 명시한다 |
| 지역 scope | valence별 local declarations와 읽기/지역·전역 대입 role, 호출 시 별도 LocalFrame | declared-unbound local의 global fallback, caller-private local 비캡처를 보존. local 선언을 미리 값이 결합된 SSA로 치환하지 않는다 |
| 제어 | ControlKind/go/assertion/previous_result/named_target와 verifier | **go는 단독 정상 successor/CFG edge가 아니다.** C의 control/error target 역할을 kind와 함께 해석해야 한다. If/Loop/Try 영역과 합류·loop-carried value·exception edge는 후속 단계에서 생성한다 |
| 오류·효과 | ErrorContext, 실패 prefix/pending window, runtime CaptureFailure/ApplyFailure/Commit, A3 PossibleErrors/SemanticCheck/ordered operations | trace는 최적화 허가·effect replay·정확한 resume 증거가 아니다. 정의 본문의 effect/error graph와 handler 관계는 아직 outer A3에 연결되지 않았다 |

**판정:** 지원 사례의 source와 semantic identity를 보존하므로 후속 분석을 시작할 수 있다. 그러나 **모든 표현에 필요한 정보가 이미 분석 완료된 IR로 존재한다거나, 후속 단계가 재분석 없이 바로 최적화할 수 있다는 주장은 불가**다. raw source 보존과 ready-to-optimize handoff를 구분한다. Source map은 기존 body_error가 escaped quote 등을 원문으로 매핑하지만, 공개된 source-unit/frame chain 계약은 아래 후속 절에서 보완했다. 전체 J 표현력과 모든 단계별 admission의 완료를 뜻하지 않는다.

<a id="오류-처리-재검토--a06p8a3-연결"></a>
##### 오류 처리 재검토 — A0.6/P8/A3 연결

- J error class/diagnostic context/render의 분리, inner context 우선 merge, 실패 대입의 기존 binding 보존은 현재 구현되어 있다. 그러나 정의 경계는 source 좌표가 다르므로 `runtime.rs`의 invoke_definition_verb가 body span/blame를 지우고 parser가 caller 위치를 붙인다. explicit modifier 경계도 caller span으로 대체한다. 이는 잘못된 caller 좌표 사용을 막는 임시 처리이며 **본문 실패 위치 보존 완료가 아니다**.
- `f=:{{y+1 2 3}}` → `f 1 2`는 length error, primitive `+`, X shape [2]/Y shape [3]을 보존하지만 span은 caller의 `f`인 [0,1]이다. `outer → inner`도 outer 이름 [0,5]만 남는다. 본문 위치와 호출 chain을 같이 제공하지 못한다.
- `missing+(1 2+1 2 3)`은 오른쪽 length error가 먼저 관측된다. `(1 2+1 2 3)+a=:9`는 length error 뒤에도 a=9이며 static P는 Unsupported다. 분석 단계에서 이 오류를 먼저 발생시키거나 runtime 전체를 replay하면 의미가 달라질 수 있다.
- `a=:7` 뒤 실패한 `a=:1 2+1 2 3`은 a=7을 유지한다. 잘못된 control로 f 재정의 실패 시 f=42를 유지한다. 이는 문장 전체 rollback이 아니라 실패 대입의 commit 방지다. 이전에 완료한 대입 효과는 남는다.
- catch는 지원된 J 실행 오류만 처리한다. Unsupported를 잡아 42를 반환하면 미지원 실행을 성공으로 위장한다. forinit의 read-only 실패처럼 C의 **비catch 경계**도 별도로 보존한다. 일반 throw/catcht, 전체 오류 종류·handler 의미와 최적화 후 오류 순서 동등성은 미완료다.
- **다음 계약:** J semantic failure / route·analysis admission miss / compiler-verifier defect / backend implementation failure를 구분한다. 현재 Error::Unsupported는 여러 의미에 사용되므로 J catchable error와 같은 것으로 해석하지 않는다. backend failure의 무조건 재실행도 금지한다.
- **진단 계약:** immutable SourceUnit/definition origin, decoded-body↔original source map, primary failure site + callsite frame chain, semantic operation origin을 전달한다. 함수 인자 전체를 복사하지 않고 작은 type/shape 요약만 유지한다. 정적 오류·런타임 의미 오류·구현 실패의 시점과 처리 권한을 구분하고, 컴파일 단계에서 잠재적인 J runtime error를 임의로 앞당기지 않는다.

<a id="외부-프레임워크에서-적용할-부분"></a>
##### 외부 프레임워크에서 적용할 부분

아래는 구현 완료 주장이 아니라 기존 P8/A0.6/A3 계약의 구체화다.

1. **MLIR SCF:** if/while/for와 yield/loop-carried value를 구조화 영역으로 유지하고 필요할 때 cf/CFG로 낮춘다. RustJ에서는 J의 조건 판정·이름·return/exception 의미를 유지해야 하며 scf 규칙으로 J를 제한하지 않는다. [SCF](https://mlir.llvm.org/docs/Dialects/SCFDialect/)
2. **MLIR verifier/conversion:** 단계별 합법 op와 dynamic legality를 분리한다. syntax/semantic validity와 특정 route lowering eligibility를 별도로 검증한다. [Toy verifier](https://mlir.llvm.org/docs/Tutorials/Toy/Ch-2/), [Dialect conversion](https://mlir.llvm.org/docs/DialectConversion/)
3. **MLIR source diagnostics:** operation origin과 callsite location/notes를 연결한다. 단일 span을 덮어쓰는 대신 primary body location과 호출 위치를 모두 보존한다. [Diagnostics](https://mlir.llvm.org/docs/Diagnostics/), [Builtin locations](https://mlir.llvm.org/docs/Dialects/Builtin/)
4. **MLIR effects/speculation:** memory effect와 이동·speculation 허가를 분리한다. 해당 문서도 non-local control flow 모델의 한계를 명시하므로 try/throw 및 J first-error를 MemoryEffects만으로 처리하지 않는다. [Side effects](https://mlir.llvm.org/docs/Rationale/SideEffectsAndSpeculation/)
5. **JAX jaxpr:** 명시적 입력/결과·상수와 하위 계산 영역을 구분하는 표현을 참고한다. J의 late global lookup을 JAX식 trace-time constant capture로 바꾸지 않는다. 해당 문서의 tracing 중 Python control flow 실행을 RustJ의 지연 정의 의미로 그대로 가져오지 않는다. [jaxpr](https://docs.jax.dev/en/latest/601/jaxpr.html)

Co-dfns/APEX/TAIL-Futhark 기존 연구는 FOUNDATIONS §§60–68의 이력으로 유지한다. 이번 Co-dfns pinned manual URL은 열리지 않아 새 소스 재검증 완료로 세지 않았다. 이번 신규 확인의 직접 근거는 MLIR/JAX 공식 자료다.

이번 감사 도구 검증: Windows default/portable 각각 **608 passed / 0 failed / 0 ignored**, Python **67 passed**, fmt/clippy 통과. runtime 변경은 없다. 새 62개 관측의 미지원 차이는 기존 304 matched 보고서의 수용 사례와 별도로 유지한다.


<a id="frontend-감사-보완-실행-계획-2026-10-07"></a>
#### Frontend 감사 보완 실행 계획 (2026-10-07)

기존 F1/P4/P8/A0.6/A1–A3 체크리스트를 원장으로 사용한다. 현재 지원 subset E2E 완료는 유지하며 frontend에서 CFG를 만들지 않는다. 아래 순서는 기존 항목의 실행 순서이며 각 단계의 구현·독립 검증 후에만 완료로 표시한다.

| 순서 | 기존 소유 항목 | 실행 범위·수용 조건 | 상태 |
|---|---|---|---|
| 1 | A0.6 definition source frames | immutable 원문·decoded body source map, 본문 실패 site와 호출 chain, ordinary/modifier/nested 경계 보존. escaped quote/UTF-8/CRLF·실패 후 재정의·catch/effect 회귀 및 C error-kind/post-state 비교 | 본문·실행 전·반환 frame 및 source-unit/file provenance 구현·검증 완료 |
| 2 | A0.6 error category / P8 admission | J 오류와 분석/route 미지원·verifier/backend 실패의 처리 권한을 구분하고 단계별 admission을 구조적으로 전달. Unsupported의 J catch 금지·실행 replay 금지 유지 | 현재 frontend 경계 구현; 최종 검증 중 |
| 3 | P8 / A1–A3 handoff | NAME policy·scope·version 관측과 executable guard를 구분. modifier value 전달, 계산된 noun constructor와 문장 내 effect의 명시적 경계; 본문 구조화/CFG는 후속 IR에서 수행 | verified handoff/NAME adapter 구현; 본문 lowering은 후속 |
| 4 | F1/P4 NAME compatibility | computed/noun/multiple targets → abandon → direct/indirect locatives 순으로 독립 실행 단위 선정. local/global·POS 변경·효과/첫 오류를 C와 비교 | 문자열 target·bounded abandon/nameless 이름 이관 runtime과 top-level NAME 효과·배열 Graph 연결 구현; nameless 직접 적용/일반 locatives 미완료 |
| 5 | F1 numeric compatibility | complex/extended/rational·큰 정수 literal 변환. recognition/type/value/error를 분리하고 C bridge 지원부터 확장 | 미완료 |
| 6 | P 단계 modifier inventory | 아직 미지원 core conjunction/derived modifier를 원본별로 추가. vocabulary 인식과 construction/runtime/lowering 수용을 별도로 검증 | 미완료 |

1단계 데이터 계약: ErrorContext에 source-owned 진단 frame chain을 추가하고 caller-relative span/word-index의 기존 의미는 유지한다. primary body 위치는 DefinitionCode의 sparse quote source map으로 원문에 대응한다. source는 Arc로 공유하고 noun payload를 진단에 복사하지 않는다. frame chain은 진단 정보이며 J semantic identity·CFG·resume 토큰이 아니다. 렌더러는 body 실패와 각 정의 호출 위치, 마지막 외부 caller를 함께 표시한다. 단계 1의 완료는 다른 단계의 미지원을 해결했다는 뜻이 아니다. Windows default/portable, fmt/clippy, Python 및 bounded C 감사로 검증하고 기계 원자료를 reports에 남긴다. CUDA/GitHub CI는 계속 유보한다.

A0.6 실행 체크리스트 — 첫 수정 범위:

- [x] `DefinitionSourceMap`: decoded body byte range를 원문으로 대응. doubled quote만 sparse 위치 목록으로 보존하고 전체 body/source 일치를 verifier에서 확인한다.
- [x] `DiagnosticSourceFrame`: `kind`, 공유 `source: Arc<str>`, `definition_span`, 원문 `span`, fragment queue 기준 `blame_word_index`를 보존한다. `ErrorContext.source_frames`는 가장 안쪽 실패 → 바깥 정의 호출 순서다. 기존 caller span/word-index 의미는 유지한다.
- [x] ordinary/modifier/nested 정의의 statement/control 실행 실패를 source frame으로 전달하고 렌더링한다. escaped quote·UTF-8·CRLF·source 재정의 수명·실패한 대입·catch 회귀를 추가한다.
- [x] 최종 소스 기준 native Windows default/portable 각각 **612 passed / 0 failed / 0 ignored**, fmt/clippy 통과, Python **67 passed**. 정의 호출/loop/nested/NAME scope C 비교 **304/304 일치**. Frontend 감사 **38 matched / 24 runtime_gap**는 기존 미지원 경계를 유지한다. `reports/definition-*-windows.json`, `reports/frontend-contract-audit-windows.json`에 바이너리·소스 hash와 결과를 기록했다. C 오류 위치/문자열 동등성은 검증하지 않았으며 source frame은 Rust 회귀 테스트로 검증했다.
- [x] 정의 실행 전 admission 오류, 최종 noun-result 검사/implicit-return fixing 오류처럼 statement 경로 밖의 실패에도 해당 경계에 맞는 frame을 추가한다. 제어/문장 site가 없으면 정의 span을 사용하며 body 위치를 만들지 않는다.
- [x] source-unit/file identity와 nested source의 최상위 원문 provenance를 확장한다. named source API와 파일 CLI는 immutable 파일 전체 원문에 연결하고, 익명 입력은 해당 입력 단위에 연결한다.
- [x] 순서 2: 현재 frontend 경계의 오류 분류와 typed stage admission 계약. 아래 A0.6/P8 절의 구현 범위/legacy 제한을 따른다.
- [x] 순서 3: 검증된 Program/context handoff, NAME·effect·modifier 전달 의무와 bounded NAME adapter. 일반 body/modifier lowering은 후속 IR 범위다.
- [ ] 순서 4–6: NAME 표현력, 수치 literal, modifier 호환성을 독립 C 비교와 함께 확장한다.

F1/P4 noun target 우선 실행 (2026-10-07): 문자열 단일·다중 대입을 먼저 구현한다. `p.c::jtis`에 따라 단일 문자열 이름은 RHS 전체를 보존하고, 다중 이름은 atom 확장 또는 선두 축 item 선택 후 한 번 open하여 왼쪽부터 대입한다. 이름 오류나 read-only/global 충돌은 앞서 성공한 대입을 되돌리지 않는다. 이름 개수 불일치는 대입 전에 검출한다. 따라서 다중 대입을 하나의 transactional write로 축약해서는 안 된다.

- [x] 단일 문자열 및 runtime 계산된 문자열 target, local/global·함수 RHS 처리.
- [x] 다중 문자열 target의 scalar 확장·item/open·중복 이름·부분 실패와 capture 계약.
- [x] 원래 noun target과 row-7 provenance 보존; ordered-write IR이 없는 Graph/Logical 경계는 명시적 Unsupported 유지.
- [x] Windows default/portable 각각 **617 passed / 0 failed / 0 ignored**, Python **67 passed**, fmt/clippy 통과. 새 대입 **28 fixtures × DLL 2 × route 2 = 112/112 일치**, 기존 정의/loop/nested/NAME scope **304/304 일치**, 합계 **416/416**. `reports/string-assignment-windows.json`과 기존 보고서를 최종 실행 파일로 갱신했다.
- [ ] boxed target, atomic-representation assignment, locative는 별도 후속 범위로 유지.

Frontend 감사 갱신: **31 cases / 62 observations = 42 matched / 20 runtime_gap**. `computed_target` (`'a'=:7`)와 `multiple_target` (`'a b'=:3 4`)의 runtime/post-state 차이 4건을 해소했다. 고유 미지원 사례는 **12 → 10**이며 나머지 20건은 통과로 계산하지 않는다. P는 **16 accepted / 13 Unsupported / syntax 1 / control 1**, G는 **15 / 14 / 1 / 1**, L은 **14 / 15 / 1 / 1**. P=prepare에는 binding도 포함하므로 parser가 다중 target을 구조화했다는 사실이 P admission을 의미하지 않는다. 다음 NAME 구현 단위는 `name_:` abandon이며 locale/locative와 boxed/AR target은 별도 게이트다.

전달 계약: `Program.noun_assignment: Option<NounAssignment>`는 원래 `target: Expr`와 word formation으로 얻은 `names: Vec<String>`을 가진다. `Program.assignment`는 단일 이름 write 호환 필드이며 다중/빈 target을 대표하지 않는다. 최종 대입 여부는 `has_assignment()`로 확인한다. `FrontendContext::WriteName.target`은 원래 NAME 또는 NOUN item을 가리키며 NOUN의 semantic edge도 검증한다. Runtime에서 완성한 target 값은 원래 enqueue/reduction context와 함께 보존한다. `AssignmentSource.selection`은 capture의 RHS occurrence에서 선두 축 item 선택 또는 atom 확장 후 한 번 open한 결과가 commit되었음을 표시한다. 각 commit의 이름/version/순서/실패까지 보존하며 이 capture는 재실행 계획이 아니다.

여기서 `target: Expr`는 row-7 시점의 완료된 표현이다. runtime 계산 후에는 Literal일 수 있으며 계산 전 구조는 `FrontendContext`의 원래 item/node/reduction link로 찾는다. `AssignmentSource.noun_target`은 computed noun target임을 명시한다. 현재 capture→Graph는 단일 literal을 포함한 noun-target commit을 모두 거절한다. 관측한 이름을 고정 write로 바꾸려면 target 의존성과 guard가 필요하기 때문이다. 단일 literal의 비실행 static Graph 경로와 구별한다.

`parse_frontend`는 literal 문자열 다중 target을 구조화하지만 `prepare_semantic`의 binding과 Graph/Logical은 다중·빈 target을 ordered-write IR 미지원으로 거절한다. 실제 값이 필요한 계산된 target은 static parser에서 실행하지 않고 runtime 경계로 남긴다. 단일 literal 문자열 target은 기존 단일 write 분석 경로로 전달한다. 기존 일반 NAME 대입에는 name-list heap allocation이나 재검증을 추가하지 않는다. 다중 RHS는 한 번 공유 상태로 전환하고 item만 선택한다. 현재 item 선택은 복사이며 zero-copy/GPU buffer view 구현을 뜻하지 않는다. open의 padding·sparse 및 `;` 등 별도 primitive 미지원은 그대로 유지한다.

<a id="f1p4-name_-실행-체크리스트"></a>
##### F1/P4 `name_:` 실행 체크리스트

후속 실행 계획 (2026-10-08): conjunction을 전부 거절하던 경계를 재검토한다. 두 DLL에서 primitive conjunction의 직접 적용은 valence error지만 explicit conjunction의 직접 적용과 primitive의 별도 이름 이관 후 적용은 정상임을 확인했다. 먼저 explicit/non-nameless conjunction 실행을 허용한다. nameless conjunction은 복사와 직접 적용의 차이를 후속 parser action 계약으로 구분하기 전까지 보수적으로 거절한다. 함수형 abandon의 비실행 parser 산출물도 `FunctionHead::TakeName { name, single_word }`로 구체화하여 POS·원문·abandon 정책을 보존하되 binding/lowering은 아직 실행하지 않는다.

- [x] non-nameless conjunction abandon의 global/local·explicit/direct·실패 후 삭제 순서를 C와 비교한다.
- [x] 함수형 deferred TakeName과 binding/Graph/Logical의 명시적 거절, 기존 noun/late NameRef와의 구분을 테스트한다. cap 판정과 수동 구성 capture/Graph 경계도 검증한다.
- [x] Windows default/portable, fmt/clippy/Python 및 최종 C 감사를 갱신한다. 아래 알려진 gap은 통과로 세지 않는다.

함수형 전달 계약: `Engine::parse_frontend(&self, source)`는 현재 Engine의 읽기 전용 catalog를 사용하여 binding 전 `Program` 또는 원래 queue/reduction/NAME/pending action을 가진 `FrontendFailure`를 반환한다. abandon NAME의 class와 가능한 version은 분석 가정으로 관측하며 kernel·정의 본문·대입·삭제는 실행하지 않는다. 일반 modifier의 기존 정적 identity 관측 계약은 유지한다. 기존 `prepare_semantic`은 parser 이후 binding까지 수행하는 별도 API다. catalog 없는 `parser::parse_frontend`는 기존처럼 noun을 기본 가정하며 모든 함수 POS를 추론하는 API가 아니다.

예: `f=:+` 이후 `Engine::parse_frontend("g=:f_:")`의 최종 식은 `VerbValue`이고 `entity.result_pos=Verb`, `entity.head=TakeName { name: "f", single_word: false }`, `entity.operands=[]`다. 실제 `+` 함수 본문을 고정한 값도, 실행 때마다 다시 찾는 일반 `NameRef("f")`도 아니다. `FrontendContext`에는 원래 `g`, `=:`, `f_:`의 span/word index, `CaptureAndAbandon` 정책, `CatalogClass` 근거, 선택적인 관측 version과 NAME→함수 node 연결이 남는다. local/global 의미는 정의의 NAME scope plan과 실행 frame 경계에서 정해지며 catalog 관측은 실행 guard가 아니다.

비실행 modifier 적용과 fork 첫 항의 cap 판정에 실제 abandon 결과가 필요한 경우에는 값을 추측하지 않는다. `h=:-c_:+`는 NAME 정보와 pending Conjunction row를, `f=:(cap_: + *)`는 pending Fork row를 보존하고 Unsupported로 멈춘다. runtime parser는 실제 값을 가져온 뒤 해당 생성 과정을 정상 수행할 수 있다. 아직 deferred modifier constructor IR이 구현됐다는 뜻은 아니다. binding과 Graph builder/verifier는 함수 DAG 안의 TakeName도 찾아 거절하고, 사실 추론·effect 분류는 Unknown을 유지한다. 수동 구성한 BoundProgram이나 capture로도 이를 순수 함수 상수/호출로 승격할 수 없다.

사용자 확인: tokenizer/enqueuer는 `name_:`를 즉시 조회·삭제하지 않는다. 기본 이름·abandon flag·원래 span/word index를 넘긴다. 비실행 parser는 noun의 `ExprKind::TakeName { name, single_word }`를 만들어 일반 ReadName과 구분한다. ordered NAME effect IR이 없는 기존 binding/Graph/Logical 경계는 거절하며 조회·삭제를 실행하지 않는다. 기본 Runtime semantic parser는 C와 같은 stack-entry 순서로 값을 가져오고 실제 발견한 scope를 삭제한다. 아래의 새 명시적 NAME 효과 계획도 해당 순서를 보존하며, 삭제를 옮기는 compiled route는 동일한 lookup/effect/error 순서를 입증해야 한다.

- [x] enqueue flag, deferred noun expression, NAME policy와 원래 context 연결.
- [x] runtime noun/verb/adverb by-value 반환, 실제 local/global scope 삭제, missing/error 순서, 삭제 후 재대입 ABA guard 회귀. 함수 값 내부의 별도 late nameref는 그대로 유지한다.
- [x] pinned C의 단일-word local fast path는 값을 반환하되 삭제하지 않는 동작을 별도 확인. `name_:`가 대입 target이면 lookup/delete하지 않고 기본 이름을 대입한다. explicit/direct local 선언도 기본 이름으로 기록한다.
- [x] capture에서 관측 전 lookup과 실제 삭제 여부를 보존하고 순서 있는 effect를 Graph의 순수 read로 승격하지 않는다. by-value 이름에 ordinary SimpleNameGuard를 만들 수 없다.
- [x] 두 C DLL·두 실행 경로 및 Windows default/portable·Python·fmt/clippy 검증. 아래 미지원 결과는 통과로 세지 않는다.
- [ ] locale/locative, execute 내부의 특수 abandon 처리와 deferred function entity lowering은 별도 후속 범위.

초기 `84782fd`의 경계: conjunction abandon을 전부 Unsupported로 거절했다. 2026-10-08 후속 수정은 explicit/non-nameless conjunction을 허용하고 nameless conjunction만 보수적으로 거절한다. nameless primitive의 직접 적용은 두 DLL의 valence error와 차이가 있어 이를 C 호환 성공으로 세지 않는다. primitive를 별도 이름으로 이관한 뒤 적용하는 C 경로는 정상임을 확인했지만 아직 Rust admission에 포함하지 않는다. 읽기 전용 loop index의 일반 삭제는 별도 `j.dll` probe에서 DLL 접근 오류가 관측되어 Rust는 Unsupported로 거절한다. 해당 C probe는 두 DLL 감사의 통과 집계에 포함하지 않는다. 단일-word local index의 비삭제 경로는 독립적으로 검증한다. extension registry binding의 abandon과 일반 function-result display도 미지원이다.

지연의 단위는 조회와 삭제를 결합한 의미 연산이다. suffix를 지운 ReadName이나 이미 계산한 Literal만 전달하면 안 된다. `TakeName.single_word`는 C의 local fast-path 맥락을 보존한다. 분석 단계에서 부작용을 일으키지 않으며, 추후 effect IR은 발견한 scope/binding identity, by-value 결과, 삭제 여부, 오류·다른 NAME 조회와의 순서를 함께 표현해야 한다. `CaptureEvent::Abandon`은 실행 전 lookup observation과 실제 `deleted`를 보존하는 관측 기록이며 실행 계획이 아니다.

2026-10-07 최종 검증: Windows default/portable 각각 **625 passed / 0 failed / 0 ignored**, Python **67 passed**, fmt/clippy 통과. `tests/name_abandon.rs` 8개 회귀는 비실행 분석, 원문·정책, 값·alias 수명, 평가 순서·실패 후 삭제, explicit/direct local의 단일-word 특례, declared-unbound global fallback, caller-private 비캡처, 함수 내부 late alias, guard ABA와 미지원 non-catchable 경계를 다룬다. `reports/name-abandon-windows.json`은 **17 fixtures × DLL 2 × route 2 = 68 observations: 64 matched / 4 unsupported_gap**이며 네 gap은 conjunction 한 사례다. 기존 문자열 대입·정의/loop/nested/scope는 최종 바이너리에서 **416/416 일치**. 합계 **480 matched / 4 unsupported_gap**이며 full J conformance를 뜻하지 않는다. C 오류 위치/문자열 동등성은 검증하지 않았다.

Frontend 감사 최종 갱신: **31 cases / 62 observations = 44 matched / 18 runtime_gap**. 앞의 문자열 대입 단계 **42/20**에 이어 `abandon_name`의 runtime/post-state 차이 2건을 해소했다. 고유 gap은 **10 → 9**. P/G/L admission은 이전 단계와 같으며 deferred effect의 binding/lowering은 여전히 Unsupported다. `reports/frontend-contract-audit-windows.json`과 위 비교 보고서에 최종 소스·실행 파일 hash를 기록했다. 참고 C 소스 pin은 `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`, 실제 DLL release는 `ded7793fe5795d79eda8e7138dce94aa056edf78`이며 같은 소스를 재빌드했다는 뜻은 아니다.

2026-10-08 후속 최종 검증: Windows default/portable 각각 **635 passed / 0 failed / 0 ignored**, Python **67 passed**, fmt/clippy 통과. 함수형 transport·verifier 4개 unit 회귀와 실행/공개 frontend API 6개 회귀를 추가했다. `reports/name-abandon-windows.json`은 **27 fixtures × DLL 2 × route 2 = 108 observations: 104 matched / 4 unsupported_gap**. explicit/direct conjunction, local bare-result 오류의 catch 후 binding 유지, global fallback, 생성 실패 후 삭제·catch, 실제 abandon 값이 cap인 fork를 포함한다. 네 gap은 기존 nameless conjunction 직접 적용 사례다. primitive의 별도 이름 이관 경로는 C-only 조사에서만 확인했고 Rust 지원/비교 통과로 세지 않았다. 기존 문자열 대입·정의/loop/nested/scope는 최종 바이너리에서 **416/416 일치**; 합계 **520 matched / 4 unsupported_gap**다.

Frontend 감사는 verb/adverb 이관·explicit conjunction 세 사례를 추가하여 **34 cases / 68 observations = 50 matched / 18 runtime_gap**이다. 이전 31-case 집합의 고유 gap 9건은 그대로이며 이번 수치 증가는 추가한 정상 runtime 사례 6건이다. P는 **16 accepted / 16 Unsupported / syntax 1 / control 1**, G는 **15 / 17 / 1 / 1**, L은 **14 / 18 / 1 / 1**. 새 `Engine::parse_frontend` 성공과 binding을 포함한 P admission을 구분한다. machine report의 source/binary/DLL hash를 최종 실행 파일로 갱신했다. full J·C 오류 위치/문자열·Linux·GPU·GitHub CI 검증을 주장하지 않는다.

다음 NAME 게이트: nameless conjunction의 별도 이름 이관과 직접 적용 차이를 parser action 계약으로 구분하고, noun/function TakeName을 실제 순서 있는 NAME effect IR로 lowering한다. 발견한 scope/binding identity, by-value 결과, 삭제 여부, 관측 오류 순서를 보존해야 하며 그 전에는 기존 Graph/Logical admission을 완화하지 않는다. locale/locative, boxed/AR target과 나머지 수치/primitive gap은 독립 후속 게이트다.

2026-10-08 이름 이관 실행 계획:

- [x] nameless conjunction의 abandon lookup/delete를 허용하고 parser Item에 적용 미지원 상태를 별도로 보존한다. semantic POS와 FunctionEntity를 변조하지 않는다.
- [x] 대입·괄호는 상태를 유지하며 이름에 저장한 함수 자체에는 상태를 넣지 않는다. 이후 새 문장의 일반 조회는 정상 적용 가능하다. 문장 내부 대입 직후 직접 적용은 계속 Unsupported로 거절한다.
- [x] global/local·연쇄/괄호 대입·single-word local 비삭제·실패 후 삭제/commit·capture 순서를 회귀 및 두 C DLL로 검증한다. C valence error와 Rust Unsupported 차이는 별도 gap으로 보고한다.
- [x] Windows default/portable·fmt/clippy·Python 및 최종 비교 보고서를 갱신한다. ordered NAME effect IR은 다음 독립 구현 단위다.

이 단계의 실제 계약은 `Item.abandoned_nameless_conjunction: bool`이다. 실제 abandon 결과가 nameless conjunction일 때만 설정하는 runtime parser 상태이며 FunctionEntity의 immutable identity·POS·저장된 binding에는 넣지 않는다. row 7 대입과 row 8 괄호는 이 상태를 그대로 전달한다. 그 외 소비 action은 실제 lookup/delete 및 이미 끝난 내부 대입 이후에 Unsupported로 중단하고 pending action·실패 capture를 보존한다. 새 일반 NAME lookup은 새 Item이므로 정상 conjunction 적용을 허용한다. 이는 C의 pointer tag를 Rust 함수 타입으로 흉내 낸 것이 아니며, 아직 C valence error를 구현했다는 뜻도 아니다. Unsupported는 J catch로 숨길 수 없다.

예: `c=:@:` 이후 `d=:(c_:)` → `c` 삭제, `d`에 `@:` 저장; 다음 `h=:-d+`와 `h 3` → `-3`. 반면 `h=:- (d=:c_:) +`는 `c` 삭제·`d` 대입 후 Unsupported이며 `h`는 생성하지 않는다. 이후 `h=:-d+`는 정상이다. runtime capture는 삭제 전 binding observation, 실제 삭제 및 내부 commit 순서를 보존하고 ordered effect Graph 미지원으로 거절한다. 비실행 frontend의 `FunctionHead::TakeName` 전달과 binding/Graph/Logical 거절 계약은 변경하지 않았다.

이름 이관 단계 최종 검증: Windows default/portable 각각 **638 passed / 0 failed / 0 ignored**, Python **67 passed**, fmt/clippy 통과. `tests/name_abandon.rs`는 17개 테스트이며 이번에 세 회귀 테스트를 추가했다. `reports/name-abandon-windows.json`은 **37 fixtures / 148 observations = 132 matched / 16 unsupported_gap**이다. 새 정상 이관·local bare 사례 7개는 두 DLL·두 route의 **28/28** 일치다. 기존 직접 적용 4건과 추가 괄호/내부 대입/local 직접 적용 12건을 합친 네 종류의 오류 차이는 통과로 세지 않는다. 기존 별도 감사 **416/416**과 합하면 **548 matched / 16 unsupported_gap**다. frontend 감사는 **34 cases / 68 observations = 50 matched / 18 runtime_gap**으로 유지된다. report는 최종 소스·실행 파일/DLL hash를 기록한다. full J·C 오류 위치·Linux·GPU·GitHub CI 검증은 수행하지 않았다.

다음 독립 구현 단위는 noun/function TakeName의 ordered NAME effect IR이다. lookup 시점의 scope/binding identity, by-value 결과, 실제 삭제와 single-word 특례, 후속 조회·대입·실패 순서 및 nameless 적용 admission을 명시적으로 표현한 뒤 Graph/Logical admission을 확대한다.

<a id="ordered-name-effect-ir-첫-실행-단위"></a>
##### Ordered NAME effect IR 첫 실행 단위

- [x] `Program`과 명시적인 parser step/item/node 연결에서 조회·TakeName·primitive apply·최종 단일 NAME write를 순서 있는 의미 계획으로 낮춘다. AST 순회나 span 정렬로 실행 순서를 추측하지 않는다.
- [x] SSA 값과 성공 효과 token을 분리한다. 실패 시 다음 token/연산을 실행하지 않으며 이미 끝난 삭제를 replay/rollback하지 않는다. lookup의 실제 scope·generation·version 및 삭제 결과는 실행 관측으로 보존한다.
- [x] 초기 실행 범위는 top-level 단순 NAME, noun primitive 연산, 함수형 TakeName 값 이관 및 최종 단일 대입이다. catalog POS 재사용 조건은 실행 전 검사하고 TakeName의 missing 오류는 실제 조회 순서에서 발생시킨다. ordinary noun의 초기 missing은 POS admission 실패다. 실제 값/version은 고정하지 않는다.
- [x] 위조/역순 token·잘못된 값 edge·효과 누락을 verifier로 검출하고 순서·오류·계획 재사용·함수 이관·무부작용 admission을 테스트한다. 기존 J Graph/Logical의 pure-route admission은 확대하지 않는다.
- [x] Windows default/portable·fmt/clippy·Python과 두 C DLL 비교 후 체크리스트·결과를 갱신한다. local definition frame/locative·modifier 생성·동적 verb 호출·중간 write는 다음 확장 범위다.

구체적인 전달 계약:

| 항목 | 계약 |
| --- | --- |
| 목적·입력 | `Engine::prepare_name_effects`가 읽기 전용 catalog를 사용한 비실행 `Program`을 받아 `name_effect_ir::Plan`으로 lowering한다. 준비 과정에서 kernel·정의 본문·조회/삭제 효과를 실행하지 않는다. |
| 출력 | private immutable Plan은 원래 Program, 공유 literal/function payload, `Step { operation, output: Option<ValueId>, before: EffectToken, after: EffectToken, parser_step, span, blame }` 및 결과 ValueId를 보존한다. `Operation`은 Literal/Function/Read/Take/Apply/Commit이다. |
| 순서 | `FrontendContext.steps`의 명시적 Stack/Reduce 순서에서 lowering한다. 괄호는 값 alias, 최종 단일 대입은 Commit이다. 원래 item/node 연결을 따르고 source span은 진단용으로만 사용한다. function/literal payload의 의미 권위는 Program이다. FrontendContext는 실행하지 않는다. |
| 실행·오류 | `Engine::execute_name_effects(&Plan)`은 검증과 POS admission 후 각 step을 한 번 실행한다. `Execution { result, completed: EffectToken, names: Vec<NameObservation> }`을 반환한다. 실패 token 이후의 연산을 실행하거나 parser로 replay하지 않는다. 관측에는 step, 조회 전/후 LookupObservation과 deleted가 있으며 배열 payload는 보관하지 않는다. |
| 재사용 조건 | 준비 당시 품사는 실행 전 조건이다. 현재 ordinary noun 이름이 없거나 품사가 바뀌면 효과 전에 Unsupported다. TakeName이 실행 전에 사라졌다면 missing 오류는 실제 Take step까지 미룬다. value·shape·version·generation을 상수로 고정하지 않는다. 준비 시 catalog 조회 실패도 실행된 value error로 보고하지 않고 dynamic parsing이 필요한 admission 실패로 구분한다. |
| 예시 | `a=:7` 이후 `a_:+a` → Read(a), primitive + 값, Take(a), Apply(+), 결과 14. 먼저 오른쪽 a를 snapshot하고 왼쪽 abandon이 실제 binding을 삭제한다. `a_:+1 2+1 2 3`은 먼저 length error로 끝나므로 Take는 실행되지 않는다. |
| 초기 범위 | top-level simple NAME의 noun snapshot/Take와 primitive noun 연산, 함수형 Take의 별도 이름 이관, 마지막 단일 NAME 대입. 함수 이관은 실제 실행 시 함수 값을 저장하고 내부 late NameRef는 유지한다. |
| 금지·후속 | local definition frame·locale/locative·modifier constructor·dynamic verb application·중간/다중/noun target write는 admission하지 않는다. nameless conjunction의 직접 적용도 이 계획으로 실행하지 않는다. 이 경로는 명시적 API이며 기본 eval이나 기존 pure J Graph/Logical의 admission을 확대하지 않는다. 다음에는 효과 사이의 pure array 영역을 Graph/Logical로 연결한다. |
| 소유권·검증 | literal은 한 번 공유 상태로 만들고 Program과 계획이 공유한다. 값 사용 횟수에 따라 마지막 사용은 move하고, 여러 사용에만 공유 상태를 만든다. `Plan::verify`는 원래 parser 연결에서 명령을 다시 유도하여 효과 누락/재배열, token·SSA edge·provenance 불일치를 검출한다. 검증은 J kernel이나 parser를 재실행하지 않는다. |

품사 경계의 중요한 예는 `a=:7`일 때 준비한 `b=:a`다. 준비 후 a를 삭제하면 기존 noun 계획은 실행 전에 거절한다. 새 J parse의 `b=:a`는 missing ordinary NAME을 verb 참조로 저장할 수 있기 때문이다. 이 admission 거절은 J 오류를 흉내 낸 것이 아니며, 실행 전 다른 경로를 선택할 수 있는 명시적 경계다. 반면 실제 계획 안에서 `a+a_:`의 오른쪽 Take 후 조회 실패는 실행된 value error이며 삭제를 되돌리지 않는다. 두 C DLL의 bare·대입 문장도 해당 결과와 삭제 상태를 확인했다.

2026-10-08 최종 검증: Windows default/portable 각각 **647 passed / 0 failed / 0 ignored**, Python **69 passed**, fmt/clippy(all-targets) 통과. 새 Rust 회귀는 8개 integration + 1개 verifier unit이며 Python 감사 어댑터 회귀 2개를 추가했다. `reports/name-effects-windows.json`: **19 fixtures × C DLL 2 = 38/38 matched**. effect로 표시한 한 문장만 새 계획으로 실행하며 setup/check는 일반 eval이다. 값·오류·삭제/실패한 최종 대입·array alias/reshape/search·verb/adverb/conjunction 및 explicit 정의 값 이관·내부 late alias를 비교했다. 실패 시 ordinary eval로 재시도하는 fallback은 없다.

기존 감사도 최종 바이너리로 갱신했다. abandon **132 matched / 16 unsupported_gap**, 문자열 대입·정의/loop/nested/scope **416/416 matched**, frontend **50 matched / 18 runtime_gap**이다. 기존 미지원 건은 줄었다고 주장하지 않는다. 새 계획 감사는 기존 runtime 두 route와 구분한 별도 38건이다. source/binary/probe/DLL hash를 보고서에 기록하며 참고 C 소스 pin과 실제 DLL release가 같은 소스 재빌드를 뜻하지 않는 기존 제한을 유지한다. Linux·GPU·full J·C 오류 위치·GitHub CI는 검증하지 않았다.

이 단계에서 예정했던 pure array의 J Graph/Logical 연결은 아래 후속 계약으로 구현했다. 다음 실행 단위는 내부 성공 위치를 보존하는 다중 Apply batching이다. local frame·중간 write·modifier construction 확대는 별도 gate이며, array optimizer가 NAME 효과를 삭제·이동·중복할 수 없도록 검증해야 한다.

<a id="name-효과와-배열-ir-연결-실행-체크리스트"></a>
### NAME 효과와 배열 IR 연결 실행 체크리스트

- [x] J Graph/Logical에 이름 조회와 구분한 명시적 배열 Input을 추가한다. 값은 효과 token을 통과한 snapshot으로 공급하며 가짜 이름·version·literal로 변환하지 않는다.
- [x] 각 primitive Apply를 입력 SSA 매핑·원래 parser step·진입/성공 token을 가진 검증된 배열 region으로 낮춘다. 초기에는 한 Apply 단위로 오류 경계를 유지한다.
- [x] Logical open-input 실행을 추가하고 NAME read/write/dynamic call을 region 안에서 거절한다. 실행 전에 모든 region을 검증하며 실패 후 다른 경로로 replay하지 않는다.
- [x] 기존 순서 있는 semantic route와 새 Logical route의 값·오류·삭제·실패 token을 비교하고 두 C DLL의 공통 사례를 갱신한다.
- [x] 후속으로 연속 pure Apply의 다중 결과 region 및 내부 성공 위치를 표현하여 batching을 검토한다. token 경계 밖의 fusion/hoisting은 허용하지 않는다.


<a id="name-배열-region의-구현-계약--2026-10-08"></a>
### NAME 배열 region의 구현 계약 — 2026-10-08

다중 Apply batching 실행 체크리스트:

- [x] Apply와 그 사이의 immutable literal/primitive-function 전달만 하나의 Graph/Logical batch로 묶고 Read/Take/Commit·그 외 parser step은 넘지 않는다. 외부 입력과 batch 밖에서 필요한 결과를 명시한다.
- [x] 각 Apply의 Logical operation 범위와 원래 step·진입/성공 token을 checkpoint로 보존한다. literal/function 전달은 zero-operation checkpoint다. 오류 시 내부 완료 위치에서 부모 오류·성공 token을 복원하며 replay하지 않는다.
- [x] 외부 입력은 한 번 이동하고 내부 SSA 수명·외부 alias·live-out을 보존한다. batch 생성은 비실행이며 검사 순서를 이동하지 않는다.
- [x] 성공·첫/후속 Apply 실패·NAME 경계·live-out·변조 거절을 회귀 및 Windows default/portable·두 C DLL로 검증한다.

`Engine::prepare_name_arrays`는 검증된 ordered NAME plan의 각 primitive Apply를 `ArrayRegion`으로 낮춘다. `ArrayPlan`은 원래 effect plan과 immutable region 목록을 보유하며, 각 region은 `step`, 진입/성공 `EffectToken`, 입력 NAME SSA `ValueId` 목록, 출력 `ValueId`, J Graph와 Logical plan을 가진다. 원래 parser step·span·blame은 부모 effect step에 남는다. Graph Input의 index는 이 입력 목록의 순서이며 NAME 조회나 저장된 literal이 아니다. 실제 조회/Take 결과는 해당 token에 도달한 실행에서 공급한다.

`Engine::execute_name_arrays`는 모든 region과 POS admission을 효과 실행 전에 확인한다. NAME read/delete/commit은 부모 executor만 수행하고 region은 값만 계산한다. 오류 시 부모 step의 진단 위치와 마지막 성공 token·삭제 관측을 보존하며 다른 경로로 재실행하지 않는다. function transfer는 배열 Apply가 없어 region 0개로 처리할 수 있다. 현재는 canonical unspecialized lowering만 허용하며, region verifier는 원래 FunctionEntity·SSA 연결·token·오류 검사와 순서를 확인한다. 최적화된 region은 추후 별도 등가성 witness 계약이 필요하다.

Graph schema **0.10**, A3 schema **0.6**에 명시적 `Input { index }`를 추가했다. `logical_executor::execute_with_inputs`는 실행 전 입력 개수와 알려진 dtype/shape/rank를 확인한다. 모든 값의 사용 횟수에는 semantic check와 최종 결과도 포함한다. 마지막 사용에서는 소유권을 이동하고, 중복 사용에서만 shared handle을 만든다. 4,096개 integer 배열의 +1 회귀는 고유 입력의 실제 data pointer 재사용과 외부 alias가 남은 경우의 원본 보존을 확인한다. 이는 복사 제거의 증거이며 C 대비 속도 우위를 측정한 결과는 아니다.

최초 배열 연결 단계의 지원 범위는 부모 ordered NAME plan과 같다. local definition frame·locative·중간 대입·modifier 생성·dynamic verb call은 별도 게이트다. 기본 eval 및 기존 pure Graph admission을 넓히지 않았다. 최초 단계는 Apply 하나의 오류 경계를 유지했으며, 다중 Apply batching은 아래 후속 계약으로 구현했다. fusion·GPU 실행·비동기 스케줄은 도입하지 않는다.

배열 연결 단계 최종 검증: Windows default/portable 각각 **656 passed / 0 failed / 0 ignored**, Python **69 passed**, fmt/clippy(all-targets) 통과. 이번 Rust 회귀는 integration 8개와 verifier unit 1개다. region 경계/입력 SSA·검사 변조 거절, 실제 저장소 재사용·alias 보존, shared fanout, shape/generation이 바뀐 계획 재사용, 오류 span/parser blame·실패 token을 확인했다. `reports/name-effects-windows.json`은 **19 fixtures × DLL 2 × route 2 = 76/76 matched**다. semantic/Logical 두 경로 모두 표시된 한 문장만 계획으로 실행하며 setup/check는 일반 eval이다.

기존 최종 바이너리 감사는 **548 matched / 16 unsupported_gap**, 별도 frontend 감사는 **50 matched / 18 runtime_gap**으로 유지됐다. 이 gap을 해결했다고 주장하지 않는다. 8개 보고서의 소스·바이너리·DLL hash **520건**이 최종 파일과 일치했다. C 소스 pin과 실제 DLL release는 앞 단계와 같으며 same-source rebuild/full J conformance·C 오류 위치/문구 동등성을 뜻하지 않는다. 테스트는 이 컴퓨터의 Windows에서만 실행했고 GitHub CI·Linux·GPU 검증은 하지 않았다.

<a id="다중-apply-batch-전달실행-계약--2026-10-08"></a>
### 다중 Apply batch 전달·실행 계약 — 2026-10-08

`ArrayPlan::batches()`는 private immutable `ArrayBatch` 목록을 반환한다. 기존 per-Apply `regions()`는 원래 경계를 검토할 수 있도록 남긴다. 실행에는 batch의 통합 J Graph/Logical plan과 SSA 작업 공간을 사용한다. `Read/Take/Commit`과 미지원 step을 넘어 묶지 않으며, 처음·마지막 Apply 사이의 immutable Literal/사용되지 않는 Function 값 전달만 내부에 포함한다. source span에서 순서를 추측하지 않고 ordered effect step을 따른다.

| 전달 항목 | 계약 |
| --- | --- |
| steps | 원래 effect plan의 연속 step 범위. 각 Apply와 사이의 값 전달 step 모두 포함한다. |
| inputs | `(부모 ValueId, 대체하는 사용 횟수)` 목록. 실제 조회 결과는 한 번 import하고, 내부 fanout은 Logical SSA가 관리한다. |
| constants | `(부모 ValueId, 원래 Program literal NodeId)` 목록. 일반 inputs 뒤의 Graph/Logical Input으로 공급한다. 고정된 immutable payload handle만 미리 공유하며 NAME 조회·동적 noun 계산·정의/constructor 실행은 하지 않는다. |
| outputs | batch 밖에서 사용되는 `(부모 ValueId, Logical ValueId)` 목록. 내부 전용 중간값은 export하지 않는다. 이 계약은 다중 결과를 표현할 수 있으며 기존 Logical plan의 단일 result/스키마를 변경하지 않는다. |
| checkpoints | 각 원래 step의 `step`, Logical operation range, 진입/성공 token. Apply의 range는 해당 semantic check와 call을 포함한다. Literal/Function 전달은 zero-operation range로 원래 성공 위치를 유지한다. |
| failure | Logical executor는 성공한 operation prefix 길이를 반환한다. 부모는 해당 checkpoint에서 실패 span/parser blame과 마지막 성공 effect token을 복원한다. 이전 NAME 삭제는 유지하고 실패 뒤 Commit은 실행하지 않는다. 다른 경로로 replay하지 않는다. |
| verification | 실행 전 원래 effect plan에서 canonical batch를 다시 유도해 입력·literal payload 연결·exports·checkpoint·원래 FunctionEntity·SSA·facts·검사/순서를 검증한다. 임의 최적화 계획을 받아들이는 witness 계약은 아직 아니다. |

예: `b=:1+2+a_:`는 Take(a) 뒤 두 덧셈을 하나의 batch로 실행한다. 첫 결과는 내부 SSA이고 마지막 결과만 b의 Commit으로 넘긴다. `b=:a_:+1+2+3`은 오른쪽 덧셈 batch와 Take 뒤 왼쪽 덧셈 batch로 나뉜다. `b=:1 2+a_:+3`은 Take와 첫 덧셈 성공 뒤 두 번째 덧셈에서 length error가 나면 삭제를 유지하고 b를 보존한다. 실패 위치는 batch 전체가 아니라 실제 실패한 Apply다.

실행은 기존 semantic kernel을 사용한다. 마지막 사용의 move, shared fanout 및 export 수명을 함께 계산하며, 중간 결과를 export하면 후속 in-place kernel이 그 결과를 변조할 수 없다. 순차 CPU이며 kernel fusion·physical schedule·GPU 실행은 하지 않는다. 다음 단계는 verified batch에서의 합법적인 fusion/bufferization 후보 분석과 비용·오류 순서 witness이며, local frame·중간 대입·modifier 확장은 독립 게이트로 유지한다.

batch 단계 최종 검증: Windows default/portable 각각 **663 passed / 0 failed / 0 ignored**, Python **69 passed**, fmt/clippy(all-targets) 통과. Rust 회귀는 integration 5개와 unit 2개를 추가했다. 내부 checkpoint·상수/입력 매핑·외부 alias·현재 shape 재조회·실제 고유 data pointer의 두 연산 연속 재사용, 첫/후속 검사·kernel 실패 후 정확한 부모 token/span/parser blame·NAME 상태, 다중/중복 export 및 변조 거절을 확인했다. Pointer 재사용은 복사 제거의 증거이며 성능 벤치마크나 C 대비 우위는 주장하지 않는다.

`reports/name-effects-windows.json`: **26 fixtures × DLL 2 × semantic/Logical route 2 = 104/104 matched**. 새 batch 정상·NAME 경계·첫/후속 length·삭제 후 length/domain·alias 사례 7개는 **28/28** 일치다. 기존 감사 **548 matched / 16 unsupported_gap**, frontend **50 matched / 18 runtime_gap**을 유지했고 보고서의 소스/바이너리/DLL hash **528건**이 일치했다. source pin과 DLL release는 앞 단계와 같다. full J·C 진단 문구/위치·성능·Linux·GPU·GitHub CI 검증은 아니다. 기본 eval 및 부모 NAME plan의 admission 범위는 확대하지 않았다.

<a id="frontend-a06--정의-실행-전반환-오류-경계-보완"></a>
### Frontend A0.6 — 정의 실행 전·반환 오류 경계 보완

frontend E2E 보완을 현재 우선 작업으로 복원한다. 배열 batching/fusion 확장은 이 게이트의 완료 조건이 아니다. 현재 지원 subset E2E 완료 판정은 유지하되, 감사의 runtime gap을 모두 tokenizer/enqueuer/parser 결함으로 해석하지 않는다. NAME 표현력·수치 literal·modifier 실행 미지원과 frontend 정보 전달 누락을 구분한다.

- [x] 실행 전 valence·미지원 control/framing·호출 깊이 및 frame 준비 실패에 `DefinitionAdmission` frame을 붙인다. 제어/문장 위치를 아는 경우 source map으로 대응하고, 그 외에는 실제 정의 span으로 표시한다.
- [x] 최종 noun-result 검사와 implicit-return fixing 실패에 `DefinitionReturn` frame을 붙인다. 마지막 결과를 만든 body fragment를 보존하며 존재하지 않는 queue blame을 만들지 않는다.
- [x] 기존 `DefinitionBody/DefinitionCall` chain과 caller-relative span/blame을 유지한다. 반환 실패는 본문 catch 바깥에 남고, 이전 효과·실패한 대입·local frame 정리를 바꾸지 않는다.
- [x] Windows default/portable·fmt/clippy·Python과 두 C DLL의 오류 종류/실패 후 상태를 확인한다. C 진단 문구/위치 동등성은 주장하지 않는다.
- [x] 다음 독립 단위였던 source-unit/file identity와 중첩 정의의 최상위 원문 provenance 구현. 아래 후속 절의 검증/제한을 따른다.

구체적인 산출물은 `ErrorContext.source_frames`의 기존 `DiagnosticSourceFrame { kind, source: Arc<str>, definition_span, span, blame_word_index }`다. `kind`에 `DefinitionAdmission/DefinitionReturn`을 추가했고 나머지 전달 구조는 유지한다. `DefinitionCode::diagnostic_error`는 해당 원문 frame만 추가하며 J 오류 종류나 caller 위치를 대체하지 않는다. renderer는 각각 `before definition execution`과 `returning from definition`으로 표시한다. source map은 doubled quote/UTF-8/CRLF 원문 위치를 유지하고 재정의 후에도 공유 원문 수명이 보존된다.

참고 근거는 고정 C 소스 `cx.c`의 본문 실행 후 noun 검사·`FIXAFIRSTIMPLOCONLY` 처리다. 예: `f=:3 : '+'`의 `f 0`은 본문 평가 뒤 noun-result 오류다. `f=:3 : 'count=:count+1\ntry. local=.+ catch. 42 end.'`의 반환 오류는 본문 catch로 처리하지 않으며 count 증가를 유지하고 호출자의 실패한 대입은 commit하지 않는다. 미지원 select 실행은 본문 실행 전에 거절하므로 앞 문장의 전역 변경도 실행하지 않는다. 이는 RustJ admission 정책이며 select를 C처럼 실행한다는 뜻이 아니다.

최종 검증: Windows default/portable 각각 **668 passed / 0 failed / 0 ignored**, Python **69 passed**, fmt/clippy(all-targets) 통과. 새 회귀 5개를 추가해 `definition_diagnostics`는 9개다. 실행 전 control admission·valence, explicit/direct 최종 noun 검사, implicit return의 domain/Unsupported, 중첩 호출 frame 순서·반복 실패 후 정리, 반환 오류와 catch/이전 전역 효과/실패한 대입·local 수명, escaped quote·UTF-8·CRLF·재정의 후 원문 수명을 확인했다.

정의 호출 감사는 **36 fixtures / 144 observations = 144/144 matched**이며 새 5개 사례는 두 DLL·두 route에서 **20/20** 일치다. multiline body를 CLI에 공급할 때는 명시적 `3 : 0` block을 사용하고 API/hex probe에서는 string form을 검증한다. 기존 NAME·문자열 대입·정의/loop/nested/scope 합계는 **568 matched / 16 unsupported_gap**다. Frontend 감사는 4개 정상 오류/실패 후 상태 사례를 추가하여 **38 cases / 76 observations = 58 matched / 18 runtime_gap**이며 고유 gap 9개는 그대로다. 별도 NAME 효과/배열 감사는 **104/104 matched**를 유지했다. 8개 보고서의 소스·바이너리·DLL hash **528건**이 일치했다. C source pin `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`과 DLL release `ded7793fe5795d79eda8e7138dce94aa056edf78`은 서로 같은 소스 재빌드를 뜻하지 않는다. full J·C 진단 위치/문구·Linux·GPU·GitHub CI는 검증하지 않았다.

이로써 A0.6의 실행 전·반환 frame 누락 항목을 닫는다. A0.6 전체 완료나 full frontend conformance를 주장하지 않으며, source-unit/file identity와 nested source → 최상위 원문의 provenance는 아래 후속 절에서 처리한다. 그 다음은 오류 분류/단계별 admission 계약이다.

<a id="frontend-a06--파일-원문-식별과-중첩-정의-provenance"></a>
### Frontend A0.6 — 파일 원문 식별과 중첩 정의 provenance

목적은 tokenizer → enqueuer → parser의 조각 좌표를 원래 입력 revision에 연결하는 것이다. 입력은 immutable `SourceUnit { id, name, text }`와 checked `SourceOrigin`이며, 출력은 `FrontendContext.source_origin`(schema 3), `DefinitionSource.origin`, `DefinitionCode.origin`, `DiagnosticSourceFrame.origin`이다. `SourceUnitId`는 process-local 진단용 식별자이고 같은 경로/내용을 다시 읽어도 별도 revision이다. FrontendUnitId·scope·binding version·SSA ValueId가 아니며 함수의 구조적 동등성에 포함하지 않는다.

`SourceOrigin`은 공유 원문 slice → decoded body sparse quote map → statement slice를 합성한다. 중첩 정의가 본문 조각에서 다시 만들어져도 parent chain을 보존한다. slice는 원문 Arc/range를 공유하고 전 byte map·원문 검색·noun payload 복사로 위치를 복구하지 않는다. decoded body map은 호출당 한 번 공유한다. UTF-8 경계/범위 오류는 거절하며 verifier는 parser text와 origin text의 일치를 검사한다. 이름과 원문 byte offset은 보존하고 CFG·배치·물리 메모리·실행 replay를 결정하지 않는다.

공개 경계: `SourceUnit::new(...).origin().slice(range)`, `parser::parse_frontend_source`, `Engine::eval_source_diagnostic`, `Engine::eval_source_captured`. 기존 문자열 API는 `<input>` 익명 단위를 만든다. CLI 파일은 원본 파일을 한 번 보유하고 collector는 framing에만 사용한다. 실제 실행은 원본 byte range를 사용하여 CRLF/UTF-8와 파일 line을 보존한다. 본문/call/admission/return frame의 기존 `source`/`span`/`definition_span`은 로컬 단위 좌표로 유지하고 `origin.root_span(...)`으로 파일 좌표를 얻는다. caller span과 queue blame은 바꾸지 않는다. renderer는 파일 이름과 root line/column을 사용한다.

예: `outer=:3 : 0` 본문 안의 `inner=:{{ missing+y }}`를 실행하면 inner 실패 frame과 outer call frame은 모두 정의 파일의 각 위치를 가리키고 마지막 외부 caller는 별도로 렌더링된다. global로 남은 inner는 outer를 재정의하고 입력 handle을 버린 뒤에도 같은 source revision을 보존한다.

- [x] immutable input identity와 checked slice/body map 합성, 함수 의미 동등성과 분리.
- [x] enqueue constructor → parser 결과/capture → 재파싱된 중첩 정의 → 진단 frame에 provenance 전달.
- [x] 파일 CLI가 raw LF/CRLF 원문을 사용하고 익명 API는 입력 단위를 명시.
- [x] 회귀 6개: explicit/direct nested·escaped quote·UTF-8·escaped 함수/재정의 수명, admission/return, 같은 파일명 revision, verifier 위조 거절, frontend 성공/실패/capture, raw LF/CRLF 파일 CLI의 두 route.
- [x] 최종 Windows default/portable 각각 **674 passed / 0 failed / 0 ignored**, fmt/clippy(all-targets), Python **69 passed**. 기존 C 감사 **568 matched / 16 unsupported_gap**, frontend **58 matched / 18 runtime_gap**, NAME 효과 **104/104 matched**를 유지했다. 8개 보고서의 소스·실행 파일·DLL hash **536건** 일치. 고유 frontend runtime gap 9개는 그대로이며 새 provenance 회귀는 Rust에서 검증했다. C pin/DLL release는 직전 절과 같고 동일 소스 재빌드를 뜻하지 않는다. C 진단 문구/위치·full J·Linux·GPU·GitHub CI는 검증하지 않았다.

제한: 원문 수명 때문에 정의가 살아 있는 동안 해당 파일 전체 Arc가 유지된다. 편집 가능한 source manager, 영속/분산 ID, dynamic execute 원문 연결은 미구현이다. stdin은 아직 입력 단위의 익명 원문이며 세션 전체 파일 좌표를 주장하지 않는다. 오류 위치는 RustJ의 보존 계약이며 C 진단 문구/좌표 동등성을 뜻하지 않는다. A0.6의 다음 작업은 오류 분류/단계별 admission이다. NAME/수치/modifier의 잔여 호환성과 후속 IR admission도 별도 미완료이므로 전체 frontend E2E 완료를 주장하지 않는다.

<a id="read-references"></a>

<a id="frontend-a06p8--오류-소유권과-검증된-handoff"></a>
### Frontend A0.6/P8 — 오류 소유권과 검증된 handoff

두 보완 단위를 연결한다. `FailureCategory`는 `JLanguage`, `UnsupportedCapability`, `VerifierDefect`, `BackendFailure`를 구분한다. 기존 J 오류의 `kind()`는 유지하며 내부 IR 오류는 `Error::Verification`/`verifier failure`, admitted 구현 실패는 `Error::Backend`/`backend failure`로 전달한다. `Error::is_j_catchable`은 실제 실행에서 발생한 J 오류에만 적용하고 정의의 catch 경로는 이 공통 정책을 사용한다. Unsupported·verifier·backend 실패는 catch로 성공값이 되지 않고, 앞서 발생한 효과를 되돌리거나 다른 경로로 자동 replay하지 않는다. backend 종류는 계약/음성 회귀로 검증하며 CUDA나 새 backend 실행 구현을 뜻하지 않는다.

Read-only 경계는 `admission::{Stage, Admission<T>, Rejection}`과 `Engine::admit_frontend/admit_frontend_handoff/admit_semantic/admit_j_graph/admit_logical/admit_name_effects/admit_name_arrays`다. 입력은 현재 catalog를 이용한 J source, 출력은 해당 단계 표현 또는 requested stage·원래 Error/category·알려진 진단 맥락이다. frontend/handoff parsing 실패는 기존 `FrontendFailure.context` prefix도 유지한다. 단계 이름은 요청한 경계이고 실제 실패 phase는 ErrorContext에 따로 유지한다. 분석이 발견한 syntax/domain 등도 실행된 J 오류가 아니므로 `Rejection::is_j_handler_eligible`은 false다. `may_inspect_another_route`는 미지원일 때 다른 read-only 분석을 시도할 수 있음을 뜻하며 실행 허가·자동 fallback·재실행 권한이 아니다. Accepted는 해당 표현을 만들었다는 의미이고 후속 IR/target/NAME guard admission을 보장하지 않는다.

`frontend_handoff::VerifiedFrontend`는 새 IR이 아니라 기존 `Arc<Program>`의 immutable 검증 경계다. 같은 `Arc<FrontendContext>`에 원래 word/item/reduction/NAME policy·resolution·evidence·binding 관측·source origin을 보존한다. verifier는 완결된 Deferred parsing, Program/context source·root, expression과 occurrence의 NAME/함수 Arc/operand edges, final simple/noun assignment 대상·copula flags·resolved names·abandon single-word 규칙, DefinitionCode의 body/name-plan/control/source-map 계약을 확인한다. 변경된 literal 값 전체를 원문으로 재계산해 증명하는 verifier가 아니며, parser가 만든 Program의 값이 의미 정본이다. observed capture·실패 prefix를 지연 실행으로 재사용하지 않는다.

출력 `Requirements`는 name environment, ordered NAME effects, late function lookup, construction observations, deferred definition bodies를 명시한다. 이들은 후속 분석의 의무이며 POS/shape/purity/lookup guard를 충족했다는 증거가 아니다. modifier value/POS와 constructor snapshots를 유지하고 explicit/direct의 monad/dyad local 선언·global 대입·unbound-local fallback 맥락은 기존 DefinitionCode.name_plan에 남긴다. 본문 실행·body CFG·local SSA binding을 frontend에서 결정하지 않는다. literal payload는 한 번 shared ownership으로 전환하고 `lower_name_effects()`는 같은 Program/context를 통해 기존 bounded ordered NAME IR로 내린다. 현재 `prepare_name_effects`도 이 경계를 사용한다.

예: a=7일 때 `b=:1+a_:`의 handoff를 만든 뒤 a=20으로 바꿔도 생성/NAME lowering은 삭제나 계산을 하지 않는다. 명시적 executor가 실행할 때만 현재 a를 읽고 삭제하며 b=21을 쓴다. `b=:a+a_:`는 앞선 Take 뒤의 value 오류와 삭제 상태를 유지한다. 일반 binding/Graph admission 거절과 ordered NAME route 수용을 구분한다. 저장된 capture로 이 문장을 다시 실행할 수는 없다.

- [x] 네 오류 범주, context/unlocated 보존, J catch 권한과 read-only rejection 구분.
- [x] Graph→A3·capture→Graph·Logical executor·ordered NAME/array/batch의 명시적 verifier 실패를 미지원에서 분리. 잘못된 입력 arity/facts와 유효한 route의 capability miss는 Unsupported 유지.
- [x] typed stage admission, frontend/handoff 실패 prefix, accepted representation과 실행/guard 권한 분리.
- [x] 동일 Program/context의 handoff 검증 및 bounded NAME adapter 연결. NAME·modifier·explicit/direct local/global 맥락 보존과 observed replay 거절.
- [x] 새 Rust 회귀 15개와 Python 감사 계약 회귀 4개 추가. F(frontend)/H(handoff)/P(binding)/G(Graph)/L(Logical)/R(actual execution)를 구분하고 오류 category·handler 권한을 기계 보고한다. P 실패를 tokenizer/parser 실패라고 세지 않는다. verifier/backend 실패가 관찰되면 감사는 runtime_gap으로 숨기지 않고 실패한다.
- [x] Windows default/portable 각각 **689 passed / 0 failed / 0 ignored**, fmt/clippy(all-targets) 및 Python **73 passed**. 두 C DLL의 기존 NAME/정의 감사 **568 matched / 16 unsupported_gap**, NAME 효과/배열 **104/104 matched**; frontend 감사 **58 matched / 18 runtime_gap**. 38개 독립 frontend 사례에서 F/H 각각 24 accepted·12 unsupported·2 J 진단, P 20·16·2, G 19·17·2, L 18·18·2이며 verifier/backend 실패는 0이다. F/H 수용은 전체 J 호환율이 아니다. 8개 report의 **552 source/binary/DLL hash** 검증 완료. C 진단 위치·문구, Linux·GPU·GitHub CI 검증은 수행하지 않았다.

제한/후속: 이 계약은 현재 frontend와 연결된 verifier 경계의 구현이다. 모든 legacy Unsupported callsite와 미래 backend를 전역적으로 전환했다는 뜻은 아니다. 일반 locative/locale·boxed/AR target·complex/extended/rational payload·N1 밖의 숫자 변환·미지원 modifier는 독립 호환성 범위이며 아직 미완료다. non-final/runtime-dependent parsing은 현재 semantic runtime 경로의 경계로 유지하고 정적 prefix를 실행 가능한 continuation으로 주장하지 않는다. 본문 Graph/Logical/CFG 및 일반 modifier effect lowering도 후속 단계다. 다음 frontend 호환성 단위는 numeric recognition과 typed literal payload 변환을 분리해 exact/complex family를 단계적으로 전달하는 것이다.

<a id="frontend-숫자-변환-n1--decimal-integer-overflow"></a>
### Frontend 숫자 변환 N1 — decimal integer overflow

범위는 접미사 없는 십진 정수 word다. C `wn.c::jtnumi/jtconnum`은 signed 64-bit 경계를 허용하고, 한 atom이라도 범위를 넘으면 word 전체를 다시 Float으로 읽으며 INT/Bool 축소를 금지한다. 이때 같은 word의 다른 정수도 Float 반올림을 따른다. `x` exact 정수와 `r` rational은 이 규칙의 대상이 아니며 아직 Unsupported다. 새 IR이나 host/device storage 선택을 frontend에 넣지 않는다.

- [x] pin된 C source의 전체 word promotion 규칙과 Windows DLL 관찰 확인.
- [x] scalar/array의 overflow를 Float payload로 변환하되 범위 내 Int의 정확도 유지.
- [x] 경계값·음수·긴 decimal·mixed word·뒤쪽 ill-formed 숫자와 정의/대입 실패 상태 회귀.
- [x] frontend/handoff/Graph/Logical/실행 및 두 C DLL 비교, 기존 8개 감사 재검증.
- [x] 전체 Windows default/portable·Python·fmt/clippy·hash 결과 기록 및 push.

엄격한 C 비교에서 Float JSON의 기존 표현 문제도 발견했다. `f64::to_string`의 `9223372036854776000` 같은 shortest-roundtrip 문자열을 JSON decoder가 정수로 읽으면 실제 Float과 다르게 비교한다. finite Float에는 소수점/지수 표기가 반드시 남게 하고 `-0.0`도 보존한다. dtype=8을 정수 dtype으로 바꾸거나 비교 tolerance를 늘리지 않는다. `tests/numeric_overflow.rs`는 JSON numeric spelling의 비트 왕복도 검증한다.


실행 결과: Windows default/portable 각각 **696 passed / 0 failed / 0 ignored**, fmt/clippy(all-targets) 및 Python **79 passed**. 새 numeric corpus는 **18 fixtures × DLL 2 = 36/36 matched**, 실제 오류·대입 후 상태를 비교하며 gap 허용은 없다. 각 DLL의 숫자 문법 감사 **2,485 cases / 0 failed**는 별도 recognition 검사이며 payload 미지원·precision 경계는 통과로 계산하지 않는다. 기존 frontend 감사는 **60 matched / 16 runtime_gap**로 개선되어 8개 독립 runtime gap이 남는다. NAME/정의 감사 **568 matched / 16 unsupported_gap**, NAME 효과/배열 **104/104 matched**는 유지됐다. **11개 report / 631 source·binary·DLL hash**를 `tools/verify_frontend_reports.py --assets-root ../rustj-project-docs`로 검증했다. source/DLL pin 차이와 bounded 비교의 제한은 기존 계약을 따른다. JSON 수정은 machine 출력 호환성 변경이며 kernel 성능 개선의 증거는 아니다. Linux·GPU·GitHub CI는 실행하지 않았다.

다음 수치 호환성 게이트는 decimal spelling에 따른 Bool/Int/Float 최소 dtype 선택과 real-family ratio conversion, 이후 exact/complex payload와 Value 저장 계약이다. 새 literal family를 단순 Float으로 바꾸거나 NAME parsing을 실행해 보충하지 않는다. general locative/locale·boxed/AR target·modifier 경계와 본문 Graph/Logical/CFG 작업도 미완료로 유지한다.


<a id="frontend-숫자-변환-n2a--integer-spelling과-bool-축소"></a>
### Frontend 숫자 변환 N2a — integer spelling과 Bool 축소

C `wn.c::jtconnum`의 `bcvtmask`는 연속된 두 비공백 문자가 있으면 Bool 축소를 금지한다. 단일 digit fast path와 두 글자 음수 fast path의 `_0`은 예외다. 따라서 `1`은 Bool, `01`은 Int, `_0`은 Bool, `0 _0`은 Int 배열이다. 이번 단위는 접미사 없는 정수 표기만 대상으로 하며 scientific real→Int 축소와 real ratio 변환은 별도 미완료다.

- [x] C source와 두 Windows DLL의 scalar/word 타입 차이를 확인.
- [x] scalar/word 전체 표기를 사용해 Bool 축소 여부를 결정.
- [x] tokenizer→enqueue→parser/handoff→Graph/Logical 및 runtime·정의·대입 결과 타입 회귀.
- [x] 독립 strict C corpus와 기존 감사·Windows 전체 회귀·hash 검증 후 push.

실행 결과: Windows default/portable 각각 **698 passed / 0 failed / 0 ignored**, fmt/clippy(all-targets), Python **79 passed**. `numeric-integer-dtype-windows.json`의 **18 fixtures × 두 DLL = 36/36 matched**, N1 overflow **36/36 matched**. 기존 frontend **60 matched / 16 runtime_gap**, NAME/정의 **568 matched / 16 unsupported_gap**, NAME 효과/배열 **104/104 matched** 유지. 두 DLL 숫자 문법 감사는 각각 **2,485 cases / 0 failed**이며 미지원 payload/precision 경계는 그대로 별도 집계한다. `tools/verify_frontend_reports.py --assets-root ../rustj-project-docs`로 **12개 report / 698 source·binary·DLL hash** 확인 완료. C 진단 위치, 전체 J, Linux·GPU·GitHub CI 검증은 아니다.

N2a 완료 시 남았던 scientific real의 Int 축소와 whole-word dtype 결합은 아래 N2b에서 별도 구현·검증한다. 당시 `1e0`은 C의 Int와 달리 RustJ에서 Float이었다. real ratio conversion, exact/complex payload 및 일반 NAME/locale 경계는 N2b 이후에도 미완료로 유지한다.


<a id="frontend-숫자-변환-n2b--scientific-real의-정확한-int-축소"></a>
### Frontend 숫자 변환 N2b — scientific real의 정확한 Int 축소

입력은 현재 real constructor가 읽는 숫자 word이며 출력은 같은 Scalar/Value literal이다. C `wn.c::jtnumcase/jtconnum`과 `k.c::jtbcvt/jtIfromD`, `j.h::ISFTOIOKFZ`를 따른다. `CVTNOFUZZ`이므로 정확히 정수인 Float만 signed 64-bit 범위 `[-2^63, 2^63)`에서 축소한다. word 전체에 소문자 e가 있고 dot이 없을 때 Float 경로에서 축소 가능하다. 대문자 E만 있으면 INT reader 실패 후 overflow suppression mask가 설정되어 Float 유지; 소문자 e가 함께 있으면 word 전체가 real 경로다. 원문 정수가 real word에 섞이면 먼저 모두 Float으로 읽은 뒤 축소하므로 `9007199254740993 1e0`의 첫 값은 9007199254740992다. Bool 축소, exact/ratio payload, NAME 실행이나 physical 저장 결정은 하지 않는다.

- [x] pin된 C source와 두 DLL의 경계/표기/whole-word 동작 확인.
- [x] 비허용 값 하나가 있으면 word 전체 Float 유지, 정확한 범위 검사 후 Int 축소.
- [x] parser/handoff/Logical·실행·정의·대입 및 실패 원문 회귀.
- [x] 독립 strict scientific corpus, 기존 감사, Windows default/portable·Python·fmt/clippy·hash 검증 후 push.

실행 검증: Windows default/portable 각각 **703 passed / 0 failed / 0 ignored**, fmt/clippy(all-targets), Python **79 passed**. `tests/numeric_scientific.rs` 신규 회귀 5개는 정확한 범위/정수성·표기 mask·whole-word 반올림·handoff/Logical·정의/대입 실패 상태를 다룬다. `numeric-scientific-windows.json`은 고정 40개와 mantissa 7 × exponent 7 × e/E 2 × scalar/real-word/dot-word 3 조합 294개, 합계 **334 fixtures × C DLL 2 = 668/668 matched**다. 각 frontend/handoff/binding/Graph/Logical inspection은 332 representation 수용·2개 J 입력 진단이며 내부 verifier/backend 실패는 0이다. 표현 수용과 실제 C 결과 비교, native compiled 실행은 다른 주장이다.

기존 N1 overflow 및 N2a 정수 타입은 각각 **36/36 matched**, frontend **60 matched / 16 runtime_gap**, NAME/정의 **568 matched / 16 unsupported_gap**, NAME 효과/배열 **104/104 matched** 유지. 각 C DLL의 numeric syntax 감사 **2,485 cases / 0 failed**는 recognition 검사이며 미지원 payload/precision 경계는 계속 별도로 기록한다. **13개 report / 765 source·binary·DLL hash**를 지속 검증 도구로 확인했다. 실제 DLL release와 검토한 C source pin이 다른 제한, C 진단 위치/문구·full J·Linux·GPU·GitHub CI 미검증은 유지한다.

N2b 당시 후속이던 real-family decimal ratio는 아래 N2c에서 구현·검증한다. 여전히 미완료인 exact/complex payload·일반 locale/locative·boxed/AR target·modifier 경계 및 본문 Graph/Logical/CFG는 독립 후속 과제다. N2b는 숫자 literal 의미 보완이며 성능 향상이나 전체 frontend 완성을 주장하지 않는다.

<a id="n2c--decimal-real-family-ratio-변환-2026-10-08"></a>
### N2c — decimal real-family ratio 변환 (2026-10-08)

계약: whole-word numeric mode/grammar 검증 이후에만 decimal ratio의 두 피연산자를 f64로 읽는다. `1r2.0`은 Float, `2r1 1e0`은 기존 exact Int narrowing 규칙을 따른다. exact rational `1r2`와 hexadecimal ratio payload는 계속 명시적 Unsupported 경계다. 이름 lookup, 지연 계산, 물리 메모리 계획을 추가하지 않는다.

- [x] pinned `wn.c::jtnumfd` 및 Windows C 실행으로 타입 mask와 signed zero/zero denominator 규칙 확인.
- [x] decimal ratio 변환 및 J의 signed 0/0·infinity 규칙 구현.
- [x] enqueue→handoff→Logical→capture·정의 local/global·실패 대입 회귀 검증.
- [x] C DLL 두 종 strict corpus·기존 감사 재실행 및 보고서 hash 검증.
- [x] Windows default/portable·fmt/clippy·Python 검사 후 commit/push.
검증 결과: native Windows default/portable 각각 **708 passed / 0 failed / 0 ignored**, fmt/clippy(all-targets), Python **80 passed**. 신규 회귀 5개 및 기존 미지원 기대값 2곳을 지원 상태에 맞게 갱신했다. strict ratio 감사는 고정 22개 + numerator 9 × denominator 7 × dot/scientific word 2 조합 126개, 총 **148 fixtures × C DLL 2 = 296/296 matched**다. signed zero를 구분하는 비교로 값·타입·shape·오류 종류·실패 대입 후 상태를 검증했다. 각 frontend/handoff/binding/Graph/Logical inspection은 143개 수용·5개 J 입력 진단이고 verifier/backend 실패는 0이다.

기존 N1 overflow·N2a integer dtype 각각 **36/36**, N2b scientific **668/668**, NAME 효과/배열 **104/104 matched** 유지. frontend **60 matched / 16 runtime_gap**, NAME/정의 **568 matched / 16 unsupported_gap**는 미지원으로 남는다. 숫자 문법 감사는 각 DLL **2,485 cases / 0 failed**, noun 수용 184→198·valid payload boundary 850→836이며 다른 precision/recognition 경계는 그대로 별도 집계한다. **14 reports / 832 source·binary·DLL hashes**를 지속 검증 도구로 확인했다.

N2c 다음 작업인 exact payload/Value 저장 계약 중 유한 extended integer는 아래 N3a에서 다룬다. exact rational/complex는 후속이다. decimal ratio는 f64 의미이며 exact rational 구현을 대신하지 않는다. hexadecimal ratio, 일반 locale/locative, boxed/AR target, modifier 경계, 정의 본문 Graph/Logical/CFG는 미완료다. 검토한 C source pin과 DLL release가 다른 기존 제한은 유지하며, 전체 J·C 진단 문구/위치·native compiled execution·Linux·GPU·GitHub CI 검증을 주장하지 않는다.


<a id="n3a--유한-extended-integer-payload와-저장-계약-2026-10-08"></a>
### N3a — 유한 extended integer payload와 저장 계약 (2026-10-08)

첫 exact payload 단위는 유한 decimal `x` family다. whole-word mode/grammar가 Extended임을 증명한 경우에만 모든 atom을 BigInt로 읽는다. `1x 9007199254740993`의 precision과 dtype 64를 유지하고 rational/complex 모드에 속한 word는 이 경로로 가져오지 않는다.

저장 계약: `Data::ExtendedInt(CpuStorage<Arc<BigInt>>)`는 immutable exact atom을 공유한다. scalar는 Inline, 배열은 Owned, NAME 경계는 Shared buffer를 사용한다. selection/view/rearrangement는 atom Arc를 복제하고 limbs를 복제하지 않는다. 이것은 CPU runtime carrier이며 GPU 주소·BufferId·물리 layout을 논리 IR에 추가하지 않는다. JSON은 exact decimal string을 사용한다. native affine physical adapter는 아직 이 타입을 거부한다.

- [x] whole-word typed mode 결과와 유한 extended payload construction.
- [x] Value/view/type facts/공유·selection·empty fill 및 구조 연산 연결.
- [x] exact 기본 산술과 미구현 연산의 Unsupported 경계 검증.
- [x] NAME alias/정의 local/global·실패 대입·정밀도 및 downstream 회귀.
- [x] C DLL 두 종 비교, 기존 감사·hash, Windows default/portable·fmt/clippy·Python 검증.
- [x] 한영 문서 갱신 후 commit/push; GitHub CI 생략.
현재 구현 범위: suffix 없는 정수를 포함한 Extended-mode word 전체, scalar/vector, Bool/Int와의 exact `+`·`-`·`*`·`=`·`<`·`>`, monadic `+`·`-`·`*`·`|`, `$`·`#`·ravel·reshape·reverse·transpose·from·scalar take/drop/rotate·scalar box/open. `$`·`#`·signum의 Semantic/Logical type facts도 dtype 64를 보존한다. 동적 NAME의 dtype이 미정이면 이 결과를 일반 Int로 단정하지 않고 Unknown으로 유지한다. 미구현 extended reduction에도 일반 Bool/Int 결과를 확정하지 않는다. 구조 연산은 limbs를 복제하지 않으며, absolute value의 비음수 atom도 재사용한다. rational/complex 모드 word는 해당 경계에 그대로 남긴다.

C 비교 도구는 private GMP 포인터를 읽지 않고 public J formatting으로 각 exact decimal atom을 추출하고 원래 shape를 보존한다. dtype 64 JSON은 decimal string 배열이며 Python의 Float 변환이나 정수 자릿수 제한을 거치지 않는다. frontend 감사는 이 oracle 코드의 hash도 기록한다. 신규 strict corpus **63 fixtures × C DLL 2 = 126/126 matched**, 각 frontend/handoff/binding/Graph/Logical inspection은 **60 representation 수용 / 3 J 입력 진단**, 내부 verifier/backend 실패는 0이다.

남은 범위: exact rational·complex payload, extended의 rational division·Float 혼합·i.-family·catenate·일반 reduce/rank assembly·sparse/native physical adapter. 미지원 연산은 J language error로 위장하지 않는다. BigInt limb/Arc의 allocator 실패를 C workspace-full로 변환하는 계약과 대규모 메모리/성능 측정은 아직 검증하지 않았다. 기존 source pin/DLL release 차이, full J·C 진단 문구/위치·native compiled execution·Linux·GPU 미검증 제한은 유지한다.

최종 실행 검증: Windows default/portable 각각 **716 passed / 0 failed / 0 ignored**, fmt/clippy(all-targets), Python **82 passed**. 신규 회귀는 exact storage/precision·Arc 공유/선택/해제·구조 연산·handoff/Logical/capture·정의/대입·미지원 경계 7개와 dtype/동적 Unknown 추론 1개다. C bridge에는 매우 긴 정수·부호·empty·잘못된 decimal 출력에 대한 Python 회귀 2개를 추가했다.

기존 numeric strict 감사는 overflow **36/36**, integer dtype **36/36**, scientific **668/668**, real ratio **296/296 matched** 유지. frontend 감사는 **62 matched / 14 runtime_gap**로 개선했으며, NAME/정의 **568 matched / 16 unsupported_gap**, NAME 효과/배열 **104/104 matched** 유지. 숫자 문법 감사는 각 DLL **2,485 cases / 0 failed**, noun 수용 198→224·valid payload boundary 836→810이며 나머지 precision/recognition 경계는 별도다. **15 reports / 918 source·binary·DLL hashes** 검증 완료. GitHub CI는 실행하지 않았다.

N3a 당시 다음 exact payload 단계였던 rational의 유한/비유한 의미와 저장 계약은 아래 N3b에서 구현했다. complex와 extended의 나머지 연산은 독립 체크리스트로 확장하며, CUDA는 계속 계획에만 둔다.


<a id="n3b--rational-literal비유한-값공유-저장-2026-10-08"></a>
### N3b — rational literal·비유한 값·공유 저장 (2026-10-08)

계약: whole-word mode/grammar가 Rational임을 확인한 뒤 exact 분자/분모를 정규화한다. 유한 값은 기약분수와 양수 분모, 0은 0/1, 무한대는 ±1/0이다. C `wn.c::jtnumr`/`vq.c::qstd` 및 Windows DLL에서 `0r0`→0, `1r0`→무한대, `1r_`→0, `_r_0`→음의 무한대를 확인했다. Rational word 전체의 dtype 128은 정수값이라도 보존하며 real/complex 모드의 ratio를 가져오지 않는다.

저장: `Data::Rational(CpuStorage<Arc<Rational>>)`로 정규화된 exact atom을 공유한다. scalar Inline·배열 Owned·NAME Shared, selection/view/rearrangement는 atom 내용을 복제하지 않는다. JSON atom은 decimal string 분자/분모 객체다. CPU runtime carrier만 확장하며 physical adapter/GPU layout과 rational 산술은 별도 후속 범위다.

실행 범위: 유리수 literal과 integer/extended 혼합 word, `$`·`#`(dtype 64), ravel·reshape·reverse·transpose·from·scalar take/drop/rotate·scalar box/open. `Rational::new`는 기존 Result API를 유지하면서 0/0→0/1, 비영 분자/0→±1/0으로 확장했다. JSON 예: `{"numerator":"9007199254740993","denominator":"2"}`. 기약분수 계산은 finite BigRational에서만 수행하며 기존 atom의 재배열은 Arc만 복제한다.

C 검증: 225개 fixture × `j.dll`/`javx2.dll` = **450/450 matched**. pinned `j.dll`은 200자리 분자/0 조합에서 `vq.c` 내부 trap(0xc000001d)이 발생해 해당 6개 source를 양쪽 비교 집합에서 제외했고 report의 `reference_exclusions`에 기록했다. 제외를 통과로 세지 않는다. 검토한 source `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`와 DLL release `ded7793fe5795d79eda8e7138dce94aa056edf78`는 동일 source rebuild가 아니다.

제한/다음: rational 산술·비교·Float/Complex 혼합·rational count/index coercion·catenate·일반 reduce/rank assembly·sparse/native physical adapter는 별도 Unsupported 경계다. BigInt/Arc allocation 실패의 workspace-full 매핑, 대용량 메모리 한계·성능, native compiled execution, Linux/GPU는 검증하지 않았다. N3b의 산술/비교 후속은 아래 N3c에서 구현했으며 CUDA는 계획만 유지한다.

- [x] 유한·무한대·zero 및 whole-word construction 구현.
- [x] Value/view/type facts/공유·selection·empty fill·구조 연산 연결.
- [x] NAME alias·explicit/direct local/global·실패 대입·handoff/Logical/capture 회귀.
- [x] 미지원 rational 산술·혼합·reduce/rank/native adapter 경계 유지.
- [x] C DLL 두 종 strict 비교 및 기존 감사/hash, Windows default/portable·fmt/clippy·Python.
- [x] 한영 문서와 체크리스트 갱신 후 commit/push; GitHub CI 생략.

검증 완료: Windows default/portable 각각 **725 passed / 0 failed / 0 ignored**, fmt·clippy(all-targets) 통과, Python **85 passed**. 신규 Rust 회귀 8개와 type fact 회귀 1개, Python exact formatter 회귀 3개를 포함한다. alias/overlap·Weak 해제·실패 대입 상태·explicit/direct local/global·enqueue/handoff/Logical/capture와 미지원 경계를 검사했다.

기존 strict numeric 감사는 overflow **36/36**, integer dtype **36/36**, scientific **668/668**, real ratio **296/296**, extended **126/126 matched**를 유지한다. frontend 감사 **64 matched / 12 runtime_gap**, NAME/정의 **568 matched / 16 unsupported_gap**, NAME 효과/배열 **104/104 matched**. 각 DLL 숫자 문법 **2,485 cases / 0 failed**, noun 수용 **313**, valid payload boundary **721**이며 precision/recognition 등은 계속 별도다. **16 reports / 1,001 source·binary·DLL hashes**를 검증했다. GitHub CI는 생략했다. full J나 C diagnostic text/location 동등성을 주장하지 않는다.

<a id="n3c--exact-rational-산술비교-2026-10-08"></a>
### N3c — exact rational 산술·비교 (2026-10-08)

계약: N3b의 canonical rational을 입력으로 `+`·`-`·`*`·`%`·`=`·`<`·`>`와 monadic `-`·`|`·`*`·`%`를 CPU runtime에 연결한다. Bool/Int/ExtendedInt 혼합은 exact로 승격하고 Float/Complex 혼합은 별도 미지원 경계를 유지한다. 유한 연산은 num-rational의 exact 연산을 사용하고 무한대는 C `vq.c`의 명시적 규칙을 먼저 적용한다. 반대 무한대 덧셈·동일 무한대 뺄셈·무한대 나눗셈은 J `NaN error`이며 기존 진단/try 처리와 실패 대입 원자성을 유지한다. 0×무한대와 0÷0은 rational zero이다. parser 지연 의미·물리 adapter·GPU 배치를 변경하지 않는다.

구현/검증 경계: arithmetic 결과는 정수값이어도 dtype 128, 비교는 Bool, monadic signum은 ExtendedInt(dtype 64)이다. Bool/Int/ExtendedInt와 rational이 한쪽에 있으면 exact로 처리한다. 두 ExtendedInt만의 나눗셈은 이번 단계에 포함하지 않는다. 유한 계산은 canonical 입력을 BigRational로 옮겨 num-rational 연산의 약분과 exact BigInt 계산을 이용하고, 결과는 `into_raw`로 이동해 재약분하지 않는다. 입력 atom은 빌리고 유한 계산용 numerator/denominator는 복제하므로 산술의 limb 복제·할당은 존재한다. 기존 배열/NAME alias는 수정하지 않고 비음수 absolute atom은 Arc를 재사용한다. 성능 우위나 allocation-free 산술은 주장하지 않는다.

C 특이 규칙: 독립 초기화한 두 DLL은 `1r0%_1r2`→양의 무한대, `_1r0%_1r2`→음의 무한대를 반환한다. `vq.c::qdiv`/`QdivQQ`의 zero denominator 결과가 finite 제수의 부호를 잃는 동작에 맞춰 무한 분자의 부호를 유지했다. 이를 일반 실수 나눗셈 규칙으로 바꾸지 않았으며 strict Cartesian 회귀에 포함했다. C source pin과 DLL release가 다른 기존 제한은 유지한다.

남은 범위: Float/Complex/Char 혼합, rational count/index coercion, catenate·일반 reduce/rank assembly·i.-family·sparse/native physical adapter. limb/Arc OOM의 J workspace-full 대응과 대용량 성능·메모리 측정은 미검증이다. CUDA는 계속 계획만 유지한다. N3c의 후속 중 ExtendedInt 나눗셈은 아래 N3d에서 구현했으며 유리수 reduction/assembly는 N3e/N3f 게이트로 남긴다.

- [x] C source/DLL finite/non-finite·promotion·error 규칙 확인.
- [x] exact 산술/비교·unary·type facts 및 공통 진단 연결.
- [x] 독립 기대값·broadcast/empty/alias·실패 대입/try·정의와 IR 회귀.
- [x] 두 C DLL strict 비교·기존 보고서 재검증·Windows default/portable·fmt/clippy·Python.
- [x] 한영 문서와 체크리스트 갱신, commit/push; CI 생략.

실행 검증: Windows default/portable 각각 **731 passed / 0 failed / 0 ignored**, fmt·clippy(all-targets), Python **85 passed**. 신규 Rust 회귀 6개와 기존 type-fact·handoff/Logical/capture 회귀 확장으로 정확한 200자리 상쇄·promotion·무한대/NaN·prefix agreement/empty·alias/절댓값 Arc 공유·실패 대입·try·explicit/direct local 및 공통 진단을 확인했다.

유리수 strict 감사는 기존 construction을 포함한 **649 fixtures × DLL 2 = 1,298/1,298 matched**이다. N3b의 C trap 6개 source 제외는 유지하며 report에 기록했고 통과로 세지 않았다. 이전 숫자 strict 감사(overflow 36, integer dtype 36, scientific 668, real ratio 296, extended 126)는 모두 matched 유지. frontend **64 matched / 12 runtime_gap**, NAME/정의 **568 matched / 16 unsupported_gap**, NAME 효과/배열 **104/104 matched**, 숫자 문법은 각 DLL **2,485 cases / 0 failed**를 유지했다. **16 reports / 1,001 hashes**를 재검증했다. native compiled execution·전체 J·C 진단 위치/문구·Linux/GPU/CI 동등성은 주장하지 않는다.

<a id="n3d--extendedint-exact-나눗셈역수와-다음-3단계-2026-10-08"></a>
### N3d — ExtendedInt exact 나눗셈·역수와 다음 3단계 (2026-10-08)

현재 exact numeric 후속 묶음은 3개 acceptance 단위다. ① N3d ExtendedInt 나눗셈·역수의 조건부 Rational 승격(이번 작업), ② N3e 유리수 right-fold reduction/empty identity/type·error 순서, ③ N3f 유리수 rank 결과 조립/혼합·fill·empty frame. ①을 완료하면 이 묶음은 2개 남는다. 이는 full J나 전체 RustJ의 잔여 단계 수가 아니다. 전체 범위에는 아직 숫자/primitive·locale/modifier·정의 본문 분석, M3 경계 검증, M4 native CPU, 최적화/성능과 미래 GPU 등이 있으며 이를 단순 체크박스 수로 합산하지 않는다.

계약: 한쪽 이상이 ExtendedInt이고 양쪽이 Bool/Int/ExtendedInt인 `%` 및 monadic `%`를 지원한다. C `vx.c::jtxdiv`/`va2.c`에 따라 모든 결과가 exact integer이면 dtype 64, 한 atom이라도 분수 또는 무한대이면 결과 전체 dtype 128이다. 0÷0은 integer zero, 비영수÷0은 rational infinity, empty 결과는 dtype 64이다. Rational 입력의 기존 dtype 128 보존과 일반 Int÷Int의 Float 규칙은 변경하지 않는다. 정적 타입은 분모/값을 증명하지 못하면 Unknown으로 남기며 실제 capture 결과가 정확한 dtype을 기록한다. num-integer의 단일 div_rem으로 몫/나머지를 구하고 첫 승격 이전 몫을 이동해 재사용하며 입력·NAME alias는 수정하지 않는다.

구현 세부/제한: 이미 transitive dependency였던 num-integer 0.1.47을 direct dependency로 명시해 BigInt `div_rem`을 사용한다. 나누어떨어지는 prefix의 private 몫은 Arc에서 꺼내 Rational numerator로 이동하며 입력 limbs를 다시 읽어 계산하지 않는다. 승격 시 두 output vector가 일시적으로 공존할 수 있고 Rational 구성·limb/Arc allocation은 남아 있다. OOM→J workspace-full 매핑, 대규모 peak memory와 성능 우위는 아직 검증하지 않았다. Float/Char/Complex 혼합은 기존 Unsupported 경계이며, general reduce/rank·search·native physical adapter·GPU를 이번 단계의 완료로 주장하지 않는다.

- [x] N3d: C 두 DLL의 divisible/nondivisible/zero·배열 전체 승격·empty·혼합/역수 규칙 확인.
- [x] N3d: CPU 구현과 독립 기대값·broadcast/alias/실패대입·정의/handoff/Logical/type-fact 회귀.
- [x] N3d: 기존 strict 보고서 확장/재검증, Windows default/portable·fmt/clippy·Python, 문서/commit/push(CI 생략).
- [x] N3e: 유리수 reduction의 right-fold·empty identity·오류/타입 보존 구현과 독립 C 게이트.
- [x] N3f: 균일 셀의 유리수 rank 결과 조립·exact 혼합/fill·empty-frame 계약과 독립 C 게이트(비균일 padding은 별도).

N3d 실행 결과: Windows default/portable 각각 **736 passed / 0 failed / 0 ignored**, fmt·clippy(all-targets), Python **85 passed**. 신규 Rust 회귀 5개와 기존 handoff/Logical/capture/type-fact 회귀 확장으로 정수/분수 승격·역수·200자리 값·prefix agreement/empty·NAME alias·local/실패대입을 검증했다.

확장된 rational/exact-division strict 감사는 **822 fixtures × DLL 2 = 1,644/1,644 matched**(N3c 대비 신규 173 fixtures/346 observations)이다. 기존 C trap 6개 source 제외는 report에 별도 유지하며 통과로 세지 않는다. 기존 overflow **36**, integer dtype **36**, scientific **668**, real ratio **296**, extended **126**은 모두 matched 유지, frontend **64 matched / 12 runtime_gap**, NAME/정의 **568 matched / 16 unsupported_gap**, NAME 효과/배열 **104/104 matched**, 숫자 문법 각 DLL **2,485 cases / 0 failed** 유지. **16 reports / 1,001 hashes**를 재검증했다. N3d 당시 이 exact numeric 후속 묶음은 **N3e·N3f 2개 미완료**였으며, 아래 N3e 완료 후 N3f만 남는다. GitHub CI는 생략했으며 full J·C 진단 문구/위치·native compiled execution·Linux/GPU 검증을 주장하지 않는다.

<a id="n3e--rational-primitive-right-fold-reduction-2026-10-08"></a>
### N3e — rational primitive right-fold reduction (2026-10-08)

계약: Rational 입력의 monadic `+/`·`-/`·`*/`·`%/`를 leading item 축을 따라 오른쪽부터 접는다. scalar/단일 item은 입력 dtype과 atom을 보존하고, 일반 비어 있지 않은 결과는 dtype 128이다. C `ar.c::jtreduce`/`ai.c::jtiden`은 zero item과 positive item/zero atom을 구별한다: zero item은 Bool 0/1 항등값으로 tail shape를 채우며, 두 item 이상이고 tail에 0이 있으면 `+/-`는 빈 Int, `*`는 빈 Bool, `%`는 빈 Float이다. 단일 empty item은 Rational을 유지한다. Static Semantic/Logical dtype facts도 이 shape 계약을 따르되 unknown shape는 Unknown으로 둔다.

CPU는 입력 Arc atom을 빌리고 마지막 item만 output으로 공유 복제한 뒤 private output의 atom을 오른쪽 fold 순서로 갱신한다. intermediate whole-array 복제를 매 row마다 만들지 않고 합법성 증명 없는 재결합/병렬화를 도입하지 않는다. J NaN 오류·실패 대입·try와 NAME alias를 보존하며 primitive identity/Rank/body CFG/physical/GPU 계약을 확장하지 않는다.

제한/다음: 일반 derived reduction의 empty prototype·dyadic insert·extended-only reduction·sparse·미지원 verb는 이번 범위 밖이다. N3f rank 결과 조립도 미완료이므로 primitive reduce 성공을 일반 rank 실행 완료로 해석하지 않는다. Rational finite arithmetic의 limb 복제와 atom별 Arc allocation은 유지하며 OOM→workspace-full, 대용량 peak memory/성능 우위는 검증하지 않았다. reviewed source와 DLL release 차이, native compiled execution·C 진단 위치/문구·Linux/GPU 미검증 제한을 유지한다.

- [x] C 두 DLL의 finite/non-finite·right fold·scalar/one/zero item·empty tail 결과 확인.
- [x] 공통 CPU primitive reduce와 Semantic/Logical type facts 구현.
- [x] exact 독립 기대값·shape/empty·alias/해제·실패 대입/try·정의·handoff/Logical/capture 회귀.
- [x] strict C 감사 확장·기존 보고서/hash·Windows default/portable·fmt/clippy·Python.
- [x] 문서·체크리스트 갱신 및 commit/push(CI 생략); N3f rank 결과 조립은 후속.

N3e 실행 결과: Windows default/portable 각각 **741 passed / 0 failed / 0 ignored**, fmt·clippy(all-targets), Python **85 passed**. 신규 Rust 회귀 5개 및 기존 type-fact/handoff/Logical/capture 회귀 확장으로 exact right-fold·200자리 cancellation·scalar/단일 item·zero item/empty tail·shape·무한대 NaN·공유/해제·실패 대입/try·explicit/direct local을 검증했다.

확장된 strict rational 감사는 **1,395 fixtures × DLL 2 = 2,790/2,790 matched**(N3d 대비 신규 573 fixtures/1,146 observations). 기존 C trap source 6개 제외는 report에 별도 유지하며 통과로 세지 않는다. overflow **36**, integer dtype **36**, scientific **668**, real ratio **296**, extended **126** 모두 matched 유지. frontend **64 matched / 12 runtime_gap**, NAME/정의 **568 matched / 16 unsupported_gap**, NAME 효과/배열 **104/104 matched**, 숫자 문법 각 DLL **2,485 cases / 0 failed** 유지. **16 reports / 1,001 hashes** 재검증. GitHub CI는 생략했다. 현재 3개 exact numeric 후속 묶음은 N3d·N3e 완료, **N3f rank 결과 조립 1개 미완료**이며 전체 RustJ 잔여 단계 수로 해석하지 않는다.

<a id="n3f--exact-rank-결과-조립-2026-10-08"></a>
### N3f — exact rank 결과 조립 (2026-10-08)

범위는 **균일한 결과 셀**의 Rational/ExtendedInt 조립과 Bool→Int→ExtendedInt→Rational 정확 승격이다. scalar rank, primitive reduce rank, 양수 frame의 explicit/direct definition 결과를 기존 CellBuilder와 동일한 순서로 조립한다. 먼저 나온 셀과 빈 셀의 타입도 승격에 포함한다. `(%"0)1x 2x 0x`는 Rational `[1/1,1/2,1/0]`을 낸다. Float 혼합은 반올림하지 않고 capability 오류로 남긴다.

메모리: 모든 결과 셀을 수집하지 않고 기존 streaming 경로의 셀 하나+출력 buffer 정책을 유지한다. 기존 Rational/ExtendedInt atom은 Arc를 공유하며 입력 배열 backing 자체는 보유하지 않는다. 정수→Rational은 분모 1을 직접 구성하여 GCD/normalization을 생략한다. 타입 승격은 이미 조립한 prefix를 한 번 변환하며 ExtendedInt limb 복제, atom Arc allocation, 승격 시 old/new buffer 공존은 남는다. 성능 우위·OOM→workspace-full·대용량 peak memory는 아직 검증하지 않았다.

빈 frame은 일반 fill-cell 규칙과 **primitive insert의 empty-total-atom dispatch**를 구분한다. `(+/"1)0 3$2r3`와 `(-/"1)0 3$2r3`는 Int, `*/`는 Bool, `%/`는 Float 빈 결과다. item 1은 Rational, item 0은 Bool identity를 유지한다. kernel 및 concrete primitive insert semantic/composite 경로에 같은 검증된 shortcut을 적용하며 임의의 함수나 dtype만으로 이를 허용하지 않는다. 양수 frame의 empty cell은 기존 reduction identity 규칙을 따른다. 검토 근거는 pinned `jsrc/ar.c`의 primitive dispatch와 `jsrc/cr.c`의 fill/result 조립이다. reviewed source pin과 DLL release pin은 서로 다르며 동일 소스 rebuild 검증을 주장하지 않는다.

- [x] 균일 셀 Rational/ExtendedInt 조립 및 Bool/Int/X→R 정확 승격; Float 혼합은 명시적 미지원 유지.
- [x] typed fill·empty frame·empty cell 타입, NAME alias/실패 대입·오류를 회귀 및 C DLL 2종으로 대조.
- [x] 비균일 셀 padding의 기존 미지원 경계를 별도 기록하며 완료 범위를 과장하지 않는다.
- [x] Windows default/portable·fmt/clippy·Python·전체 감사 및 hash 검증 후 commit/push(CI 생략).

N3f 실행 결과: Windows default/portable 각각 **746 passed / 0 failed / 0 ignored**, fmt·clippy(all-targets), Python **85 passed**. Rust 회귀 5개를 추가하여 exact 승격의 양방향 순서, 큰 정수/유리수, 빈 셀 타입·빈 frame, 공유/해제, explicit/direct local·alias·실패 대입과 NaN 오류를 검증했다.

strict rational 감사는 **1,472 fixtures × DLL 2 = 2,944/2,944 matched**(N3e 대비 신규 77 fixtures/154 observations). 기존 C trap source 6건 제외는 별도 유지하며 통과로 세지 않는다. F/H/P/G는 1,467 accepted-representation / 5 J 오류, L은 1,455 accepted-representation / 5 J 오류 / 12 unsupported-capability다. 정의 내부 조건 분기 결과를 rank로 조립하는 신규 12건은 runtime 값이 C와 같지만 Logical IR lowering은 미지원이다. runtime 일치를 compiled 실행 지원으로 해석하지 않는다.

기존 overflow **36**, integer dtype **36**, scientific **668**, real ratio **296**, extended **126** matched 유지. Frontend **64 matched / 12 runtime_gap**, NAME/정의 **568 matched / 16 unsupported_gap**, NAME 효과/배열 **104/104 matched**, 각 DLL 숫자 문법 **2,485 cases / 0 failed** 유지. **16 reports / 1,001 hashes** 재검증. GitHub CI는 생략했다. 이로써 N3d–N3f의 **명시한 범위의 exact numeric 3개 단위**를 마친다. 일반 rank·full J·C 진단 문구/위치·native compiled execution·Linux/GPU 완료를 뜻하지 않는다.

남은 독립 범위: 비균일 결과 셀의 padding, effectful/user-defined empty-frame 실행, boxed/sparse fill, Float/Complex 혼합, 일반 derived reduce·extended-only reduce 및 native physical adapter. 이번 구현을 일반 rank·전체 frontend E2E·full J 완료로 해석하지 않는다. CUDA는 계획만 유지한다.

