[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)

<a id="최적화에-사용할-frontend-정보-보존-2026-10-03"></a>
##### 최적화에 사용할 frontend 정보 보존 (2026-10-03)

목표는 SQL 구현 방식을 복제하는 것이 아니라 **실행 전 분석·최적화에 사용할 J tokenizer/enqueuer/parser**이다. 현재 우선순위는 이 세 단계의 정보 보존과 jsource와의 논리적 호환성이다. 이번 변경은 최적화 변환·실행 순서 변경·kernel 선택을 수행하지 않는다.

`static_analysis::StaticAnalyzer`는 입력 이름의 noun/function 품사와 `GraphFacts`를 선언받아 기존 tokenizer → enqueuer → parser → bind → J Graph 경로를 비실행으로 연결한다. parser의 `AbstractNoun`은 실제 `Value`가 아닌 분석용 noun 분류이며, concrete 실행에 들어가면 거부한다. 실제 입력 배열을 할당하거나 이름의 함수를 호출하지 않고 noun을 중간에 사용하는 식도 연산 구조로 남긴다. 리터럴은 기존처럼 실제 상수 payload를 구성하므로 '모든 allocation 없음'을 뜻하지 않는다.

- [x] trillion-element 입력을 metadata만으로 선언하고 `x+y*z`, `(x+y)*z`의 다른 operand 구조와 fork region을 보존하는 regression을 추가했다.
- [x] `SourceWord` sidecar로 tokenizer span, enqueue 품사·original word index·name lookup/copula flags를 분석 결과에 보존한다. `ParseReduction`이 지원되는 각 reduction의 operand word range·result origin과 연결한다. 미지원 semantic action의 runtime trace 완료를 뜻하지 않는다.
- [x] 미정 shape는 Unknown, 이름의 함수는 specialization 경계로 유지한다. 품사가 없는 이름과 값이 필요한 미지원 constructor는 분석 coverage 오류로 반환한다.
- [x] assignment는 proposed graph write만 남기며 input catalog를 변경하지 않는다. runtime에서 domain error인 식도 분석 중 실행하지 않는 regression을 추가했다. 분석 성공이 runtime 오류 없음의 증명은 아니다.
- [x] `examples/static_explain.rs`로 데이터 없이 graph와 logical memory 정보를 확인한다. catalog version은 runtime guard가 아니며 결과는 실행 가능한 compiled plan이 아니다.
- [x] 이름 조회를 오른쪽부터 stack entry로 옮기고, 지원되는 각 reduction의 provenance와 final assignment copula를 전달한다.
- [x] 지원되는 실제 noun reduction·별도 capture·top-level single-name non-final assignment를 구현한다.
- [ ] 미지원 constructor, explicit-local/locale/definition scope와 effect coverage는 F2/P2–P6에 따라 확장한다.
- [ ] frontend 검증 후 별도 단계에서 effect/error ordering 증명과 최적화 변환·lowering·실행을 연결한다.

논리적 extent/live range/resource 보고는 기존 분석기를 재사용하며 최적화는 하지 않는다. logical atom 합계는 peak allocation이 아니다. 전체 J, upstream suite, CUDA 실행을 지원·검증했다는 의미는 아니다. 이번 Windows 검증: Rust default/portable 각각 228 passed, 17 ignored; fmt/clippy 통과; Python harness 18 passed; j64/AVX2 각각 direct·semantic-reference 2,063문장 중 2,061 passed, runtime coverage boundary 2개, failed 0; stage 7,014 checks와 words 6,618 cases에서 failed 0. 신규 정적 frontend regression은 7개다. metadata-only 10^12-element 예제도 실행했다. C DLL release metadata는 `ded7793fe5795d79eda8e7138dce94aa056edf78`, source 검토 pin은 `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`이며 source pin으로 빌드한 DLL이라는 주장은 하지 않는다. 신규 정적 frontend regression과 기존 Windows default/portable·C j64/AVX2 frontend 비교를 함께 검증한다.

<a id="runtime-noun-reduction--별도-capture-v0-2026-10-03"></a>
##### Runtime noun reduction + 별도 capture v0 (2026-10-03)

- [x] P2/P4 지원 범위에서 `RuntimeParserHost`가 같은 9-row parser의 stack-entry 조회와 rows 0–2 invocation을 제공한다. 각 application은 즉시 실제 `Value`로 reduce되어 같은 stack에 재삽입된다. final noun은 계산을 다시 실행하지 않는다. 분석 context는 host가 없으므로 kernel을 실행하지 않는다.
- [x] P2/P5 `Engine::eval_captured`가 선택적인 `ParseCapture`를 반환한다. record off/on 모두 같은 parser/kernel을 실행하며 input occurrence, 원래 FunctionEntity를 가진 apply attempt, success facts 또는 failure kind/context, constructor의 noun-input 연결, final commit을 순서대로 기록한다. 실패한 경우에도 partial capture를 돌려준다.
- [x] P3/P5 `f=:+"(1+0)`와 `f=:(1+2) + *`의 computed noun constructor를 runtime에서 지원한다. 값 생산 occurrence와 constructor 입력을 연결한다. 일반 fork 호출 executor 미지원과 static computed-constructor coverage는 별도 경계이며 fake constant로 통과시키지 않는다.
- [x] recording parity, 실패 후 기존 binding/version 보존, 괄호 전후 occurrence 유지, 큰 입력의 facts-only 기록, row invocation과 이후 lookup의 순서 및 final Literal 반환을 회귀 검증했다.
- [x] P5/P8 `j_graph_ir::Plan::from_capture`가 성공 capture를 기존 J Graph로 변환하고 verifier를 통과시킨다. `CapturedGraph`는 occurrence→ValueId, constructor의 computed noun 입력, observed facts를 별도 sidecar로 보존한다. 실패 capture는 완료 graph로 변환하지 않는다.
- [x] P4 일부: row 7에서 top-level single-name non-final assignment를 즉시 수행하며 이후 stack-entry 조회·RHS POS·후속 오류 이전에 완료한 대입을 보존한다.
- [ ] P4 explicit-local/locale/definition/effect 및 전체 constructor/result-POS coverage를 확장한다. 17개 definition acceptance ignored는 여전히 미구현이다.

`parser_capture.rs`는 canonical IR을 대체하지 않는 observation log다. input/intermediate 배열 snapshot을 저장하지 않고 dtype/shape·source span·occurrence edge를 저장한다. shared FunctionEntity는 J 의미에 필요한 intrinsic noun operand를 소유하므로 그 lifetime은 capture로 연장될 수 있다. 이를 payload 복사나 buffer/physical scheduling과 혼동하지 않는다. 기록은 실제 한 실행의 관찰이며 purity/binding/value/error guards 없는 compiled replay의 증명이 아니다. capture 켠 상태에서 parser가 실제 값을 계산하는 것과 static analyzer가 실행하지 않는 것은 서로 다른 API 계약이다.


**Capture adapter 범위:** source는 capture가 읽기 전용으로 소유한다. source literal은 enqueue payload에서 다시 구성하며, named noun은 관찰 당시 version을 가진 ReadNoun으로 남겨 현재 workspace 값을 다시 읽지 않는다. apply는 기존 Builder를 사용하고 FunctionEntity 및 NameRef를 보존한다. inferred graph facts와 runtime observed facts를 분리하며, 함수 참조는 실제 호출이 성공했더라도 specialization 경계를 유지한다. constructor의 computed noun 의존 관계는 `ConstructorOrigin.noun_inputs`로 보존한다. 이 sidecar를 제외한 일반 graph memory 분석만으로 constructor operand의 완전한 lifetime/physical peak를 추정하지 않는다. 여러 effect·runtime guard·실패 후 continuation·modifier-value graph lowering과 재사용 가능한 실행 계획은 아직 범위 밖이다.

**Parser-time assignment 범위:** 위 reviewed source pin의 `p.c` row 7을 따라 `x+(x=:2)`는 오른쪽 대입을 완료한 뒤 왼쪽 이름을 조회한다. chained/parenthesized assignment도 같은 matcher를 사용하며 explicit local scope가 없는 top-level `=.`은 enqueue에서 global로 분류한다. outer row 7에 도달하기 전 실패하면 그 binding은 유지하며, 이미 수행한 대입은 모두 남긴다. runtime final assignment도 성공적인 parser exit 이후가 아니라 row 7에서 commit한다. `(x=:2`와 `x=:2)`는 syntax error지만 `x=2`를 남기는 C 동작을 따른다. final assignment reduction 후 추가 row 처리를 중단하고 exit validation을 진행하며 runtime에서 다시 commit하지 않는다. 배열 RHS는 shared로 전환한 뒤 반환용 별칭을 만든다. capture는 occurrence/function identity·실제 POS·copula provenance·previous/proposed binding version·final 여부만 기록하며 input/intermediate 배열 snapshot을 추가하지 않는다. static 경로는 실행 없이 non-final assignment를 분석 경계로 거부한다. 일반 locale, explicit local environment, noun/multiple assignment target은 미지원이다.

- [x] P4/P5: noun·verb·adverb·conjunction 중간 대입은 실제 result POS/FunctionEntity를 보존한다. capture verifier는 RHS identity/class·copula scope·Engine-local version 증가·final event 순서를 검사한다.
- [x] P4/P6: unmatched control이 도달 가능한 대입을 선제 차단하지 않는다. SyntaxError에는 정확한 control span을 보존하며 앞선 runtime error class를 대체하지 않는다. C 관찰에서 `a=:missing + )`가 exit 오류 이후 hook binding을 남김을 확인하고 기존 일괄 rollback 테스트를 수정했다.
- [x] P5/P8: terminal enqueue/parse/runtime failure kind/context를 별도로 보존하여 ApplyFailure가 없는 partial capture도 completed graph로 오인하지 않는다. 차등 harness는 full-prefix 비교가 성공하면 오래된 repro 파일을 제거한다.

- [x] P3/P4/P5: runtime named primitive modifier는 row 3/4 구성 시점에 expected POS를 확인하고 실제 completed FunctionEntity를 만든다. 일반 modifier alias는 row 7 대입 시 실제 modifier를 snapshot하므로 원래 이름의 후속 재대입이 alias를 바꾸지 않는다. 일반 verb alias의 late reference는 유지한다. C `5!:1`/call 관찰에서 `adv=:/; f=:+adv; adv=:1` 이후에도 `f`는 완성된 insert이고 `f i.3`은 3임을 확인했다. named verb의 late reference는 유지한다. `ModifierResolved`/`CapturedGraph.modifier_bindings`는 모든 modifier 대입/구성 시점 name/version/expected-POS witness·resolution row와 현재 source-use span을 보존하며 graph에도 의존성을 남긴다. 이는 executable reuse guard가 아니다. operand-free primitive modifier alias 조회를 지원하며 arbitrary derived modifier executor와 unknown static modifier action은 별도 coverage 작업이다.

**Ordered-effect graph 경계:** 새 runtime 문장도 capture verifier를 통과시키지만 현재 J Graph adapter에는 final write 슬롯 하나만 있다. non-final write가 있는 capture는 명시적으로 거부하며 commit이 마지막 event인 `(x=:2)`도 포함한다. standalone modifier-value lowering도 명시적인 graph 경계다. parser-capture 차등 보고서의 `graph_coverage_boundaries`에 exact source와 reason을 별도로 기록하며 값/error 불일치를 면제하지 않는다. ordered write/read/effect IR과 replay 합법성은 후속 작업이며 이번 단계에서 최적화를 수행하지 않는다.

Windows 검증: default/portable 각각 **250 passed / 17 ignored**, fmt/clippy 통과, Python 20 passed. j64/AVX2 각각 direct·semantic-reference·parser-capture 세 경로의 **2,139문장 중 2,137 passed + 명시적 runtime 경계 2개, failed 0**; stage 7,014 checks와 words 6,618 cases 통과. `examples/capture_probe.rs`는 모든 문장의 capture association/attempt-outcome 순서를 검증하며 성공 문장은 명시적으로 보고한 graph 경계 13건(ordered-effect 9건, modifier-value 4건)을 제외하고 J Graph adapter/verifier도 통과시킨다. C oracle은 C word formation으로 outer copula와 inner copula/literal/comment를 구분하며 harness 회귀 테스트 2개로 보호한다. parser-capture JSON 보고서도 저장한다. 기존 oracle/source pin 구분과 GitHub CI 생략 방침을 유지한다.

<a id="정적-modifier-구성과-분석-의존성-2026-10-03"></a>
##### 정적 modifier 구성과 분석 의존성 (2026-10-03)

- [x] P3: modifier의 입력 품사만 알려져 있을 때 application 결과를 Verb로 추측하는 경로를 제거했다. `declare_function(name, Adverb/Conjunction)`은 품사 정보만 선언한다. 이 이름을 구성에 사용하면 현재 이름·source span을 가진 `Unsupported` 경계를 반환한다. 단독 modifier의 품사 관찰과 실제 application은 구분한다.
- [x] P3/P5: `StaticAnalyzer::declare_primitive_modifier`로 core primitive modifier의 실제 의미를 선언한다. `Engine::prepare_semantic/analyze_j_graph`도 현재 workspace의 operand-free primitive modifier와 그 별칭을 읽어 같은 row 3/4 constructor로 구성한다. 알려진 `/`, `\`, `"`, `@:` 등도 각 constructor가 지원하는 operand legality/result POS/error 규칙을 따르며, arbitrary derived/extension modifier 지원을 뜻하지 않는다.
- [x] P5: `Program.modifier_snapshots`와 J Graph schema **0.4**의 `Plan.modifier_snapshots`에 이름, catalog/Engine-local version, expected POS, 공유 FunctionEntity, 현재 사용 위치를 보존한다. verifier는 source-use span, 이름, version과 primitive modifier 품사를 검증한다. 이 의존성은 intrinsic 함수 identity나 physical allocation 정보에 섞지 않는다. runtime capture의 `ModifierResolved` 관찰과 정적 분석의 binding 가정은 별도 sidecar다. 둘 모두 executable reuse guard가 아니다.
- [x] P6: 큰 배열은 metadata만으로 분석하고 reduction kernel을 실행하지 않는다. 알려진 modifier 별칭의 의미는 원래 이름의 재대입 후에도 유지된다. 별칭 자체를 바꾸면 새 분석의 version/구성이 달라지지만 기존 graph는 유지된다. 일반 verb 이름은 late NameRef로 남긴다. 최종 대입은 pending proposal이며 workspace/catalog를 변경하지 않는다.
- [x] P3/P6: Windows stage probe에 별도 setup/읽기 전용 분석 모드를 추가했다. C `5!:1`/`4!:0`과 구성·품사·domain/length 오류를 비교하고, 분석 후 target version 및 modifier 의존성을 검사한다. `candidate=: + analysisrank (#1 2)`는 C가 실행하면 성공하지만 정적 constructor에는 concrete noun이 필요하므로 정확한 source/reason을 `analysis_coverage_boundaries`에 남긴다. 성공 검사에 포함하지 않는다.
- [ ] arbitrary derived modifier의 실제 의미/result POS, explicit-local/locale/definition scope, ordered-effect graph와 재사용 guard를 확장한다. 값 계산이 필요한 constructor를 fake noun으로 통과시키지 않는다. 이번 단계는 tokenizer/enqueuer/parser의 분석 가능 범위를 확장하며 최적화 변환을 실행하지 않는다.

Windows 검증: default/portable 각각 **256 passed / 17 ignored**, fmt/clippy 통과, Python harness **20 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture 세 경로에서 **2,139문장 중 2,137 passed, runtime 경계 2개, failed 0**; stage **7,056 checks**(새 정적 구성·오류·대입 없음·의존성 검사 42건 포함)와 words **6,618 cases** 통과. 정적 값 의존 경계 1건과 기존 capture graph 경계 13건은 성공 수와 별도로 기록한다. 정적 분석 regression 6개를 추가했다. 기존 C DLL release/source review pin 구분, 미완료 definition 테스트 및 GitHub CI 생략 방침을 유지한다.

<a id="modifier-train의-구조품사-보존-2026-10-03"></a>
##### Modifier train의 구조·품사 보존 (2026-10-03)

- [x] P2/P3: `p.c` row 6처럼 stack의 세 번째 operand가 CAVN이면 trident, 그렇지 않으면 bident를 선택한다. `cf.c::bidents[]/tridents[]`의 nonzero action disposition은 실행 없이 실제 Adverb/Conjunction을 구성한다. SyntaxError와 즉시 semantic application disposition은 구별하며, 미구현 즉시 application을 성공 구조로 바꾸지 않는다.
- [x] P3/P5: `FunctionHead::ModifierTrain`이 C `CADVF`의 의미적 bident/trident를 나타낸다. arity 2/3, 원래 순서의 noun/function operands, 실제 result POS, source span과 completed child DAG를 보존한다. C `5!:1`의 `4` 표현과 비교하며 일반 verb Hook의 `2`/Fork의 `3`과 구분한다. `+"`, `"1`, `/\`, `@:/`, `/ / /`, `/ / +`, nested train을 지원한다. C의 executor pointer/helper slot을 의미적 operand로 복제하지 않는다.
- [x] P2/P6: source provenance/capture는 bident의 왼쪽 token과 non-fork trident의 가운데 token을 전달하고 입력 2/3개를 기록한다. runtime의 계산된 noun은 실제 값으로 구성되며 `ConstructionAttempt.noun_inputs`가 생산 occurrence를 연결한다. train이 보유한 named array는 noun by-value 규칙을 따르며 원래 이름을 재대입해도 값이 보존된다. 정적 경로는 계산된 noun을 실행하지 않고 명시적으로 거부한다.
- [x] P6: C atomic representation/POS 비교를 20문장 확장하고 SyntaxError 사례 2개를 추가했다. runtime value/error corpus에도 구성·실패·배열 이름 재대입 사례를 추가했다. 네 가지 분석 경계(기존 computed rank 1건, derived modifier application 3건)는 C 성공과 Rust `Unsupported`를 exact source로 확인해 성공 수에서 제외한다.
- [ ] derived modifier application, named derived modifier 조회/alias 구성, row 6 immediate semantic application, 전체 locale/explicit-local/definition scope를 연결한다. standalone modifier의 Semantic IR 보존과 J Graph로의 executable lowering은 별개다. compiler 분석·executor는 새 train을 일반 Hook/Fork나 알려진 primitive로 추측하지 않는다. 이번 단계에서 optimizer/CUDA를 구현하지 않는다.

Windows 검증: default/portable 각각 **261 passed / 17 ignored**, fmt/clippy 통과, Python harness **20 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture에서 **2,165문장 중 2,163 passed + runtime 경계 2개, failed 0**; stage **7,098 checks**와 words **6,618 cases**에서 failed 0이다. 회귀 테스트 5개를 추가했다. capture graph 경계 34건(ordered-effect 9건, modifier-value 25건)과 정적 분석 경계 4건은 별도로 보고한다. 새 modifier Semantic IR 구성은 executable modifier-value graph lowering의 완료를 뜻하지 않는다. 기존 미완료 definition 테스트·upstream suite 미실행·GitHub CI 생략 방침을 유지한다.

구현 기준: [p.c row 6](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c#L1057), [cf.c disposition tables and jthook](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L288). 기존 Windows DLL release pin과 source review pin은 서로 다르며 이 source로 DLL을 빌드했다는 주장은 하지 않는다.

<a id="오른쪽-operand를-묶은-modifier-bident-application-2026-10-03"></a>
##### 오른쪽 operand를 묶은 modifier bident application (2026-10-03)

- [x] P3: C `cf.c::tcNV`에 따라 verb 입력 `u (C n/v)`를 `u C n/v`의 실제 constructor에 전달한다. 현재 core `"`/`@:` conjunction과 noun/verb 오른쪽 operand를 지원한다. `+ ("1)`은 completed Rank entity이며 Rank 검증은 train 정의가 아니라 application 시점에 수행한다. 일반 train을 일괄 Verb로 추측하지 않는다.
- [x] P4/P5: runtime named derived modifier 조회와 row 7 alias 대입이 immutable train 객체를 공유한다. 원래 이름의 재대입은 이미 구성된 alias/verb를 바꾸지 않는다. Engine 정적 조회와 J Graph verifier도 알려진 train identity를 허용하며 `modifier_snapshots`의 이름/version/POS/current-use span을 유지한다. identity가 알려졌다는 사실은 모든 application을 실행할 수 있다는 보장이 아니다. 품사만 선언한 unknown modifier 경계는 유지한다.
- [x] P5/P6: 실제 완료 결과는 C처럼 Rank/Atop entity다. inline source의 row 6/row 3 provenance와 capture construction 기록, named train의 공유 identity/version witness를 별도로 보존한다. bound operand는 현재 application 위치에 연결하고 원래 train 객체를 수정하지 않는다. train의 noun은 shared storage로 보존하여 다시 구성할 때 큰 payload를 복사하지 않는다. 이 sidecar는 executable cache guard가 아니다.
- [x] P6: named alias 변경, target 대입 없음, construction-time domain/length 오류, capture witness를 테스트한다. metadata-only 10^12-element 배열의 `- ("1) x`도 kernel 없이 분석한다. C `5!:1`/`4!:0`의 함수 구조/POS 비교와 runtime value/error corpus를 확장한다.
- [ ] 왼쪽을 묶은 `tNVc`, noun-input adverb, 연속 adverb·derived conjunction·trident application의 각 action semantics를 연결한다. `(+ ("-)) i.4`와 `(+ (@:-)) i.4`는 후속 rank/Atop 단계에서 실행까지 지원했다. 현재 conformance corpus의 해당 runtime waiver는 제거했지만 전체 modifier 의미의 완료를 뜻하지 않는다. row 6 즉시 application과 locale/explicit-local/definition scope도 계속 남는다.

Windows 검증: default/portable 각각 **266 passed / 17 ignored**, fmt/clippy 통과, Python harness **20 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture에서 **2,185문장 중 2,181 passed, runtime 경계 4개, failed 0**; stage **7,128 checks**(새 정적 검사 30건)와 words **6,618 cases** 통과. 회귀 테스트 5개를 추가했다. capture graph 경계 38건(ordered-effect 9건, modifier-value 29건), static analysis 경계 3건은 성공 수와 별도로 보고한다. 전체 J/upstream suite, optimizer/CUDA 또는 GitHub CI를 구현·실행했다는 의미는 아니다. C DLL release와 source review pin의 구분을 유지한다.

기준 소스: [cf.c tcNV 및 다른 modifier train actions](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L200), [p.c modifier application](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c#L969).

<a id="왼쪽-binding과-연속-adverb-application-2026-10-03"></a>
##### 왼쪽 binding과 연속 adverb application (2026-10-03)

- [x] P3: `cf.c::tNVc`에 따라 `u (v C)`를 `v C u`의 실제 constructor에 전달한다. `1 (-")`처럼 noun 입력이 rank specification인 경우도 처리한다. row 3은 N/V 입력과 실제 결과 품사를 가진 `Item`을 받아 같은 parser stack에 재삽입한다. static computed noun은 계속 명시적 경계이며 noun을 fake constant로 바꾸지 않는다.
- [x] P3/P5: `taAV`의 `(A A)`/`(A V)`는 첫 adverb를 적용한 뒤 실제 결과와 두 번째 operand를 bident dispatch에 전달한다. `/ / /`의 `taaa`는 f→g→h 순서로 적용한다. `+ (/ /)`의 중첩 Insert, `+ (/ +)`의 Hook, `+ (/ / /)`의 3단계 Insert를 immutable completed DAG로 보존하며 요약 boolean이나 하나의 primitive로 축약하지 않는다. recursion에는 기존 depth limit을 적용한다.
- [x] P4/P6: left-bound named alias의 snapshot identity, noun-input rank/length/domain 오류, 실패 후 기존 target 유지, computed noun 생산 occurrence와 row 3의 연결을 검증한다. application의 source span은 현재 구문을 가리키며 이전 train 정의 객체를 수정하지 않는다. 지원된 정적 구간은 kernel 없이 함수 구조를 구성한다.
- [x] P3/P6: `(A C)`의 `tac`를 `(A A/V)`의 `taAV`로 오인하지 않는다. `tac`는 첫 적용 결과뿐 아니라 원래 입력도 필요하므로 미구현 경계로 남긴다. `3 (/@:)`는 첫 `/`의 DomainError가 먼저 발생하며, 유효한 `+ (/@:)`는 잘못된 Adverb 결과를 반환하지 않고 `Unsupported`를 낸다. C 성공/actual POS와 exact source를 stage 보고서에 기록한다.
- [ ] noun을 왼쪽에 고정한 rank/gerund, `tac`, derived conjunction 및 다른 trident action, train 내부 late modifier NameRef의 동적 조회·effect 계약, row 6 즉시 application을 연결한다. 구성된 nested Insert/Hook의 실행 지원은 기존 kernel/executor coverage와 별개다. locale/explicit-local/definition scope도 계속 미완료다.

Windows 검증: default/portable 각각 **270 passed / 17 ignored**, fmt/clippy 통과, Python harness **20 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture에서 **2,205문장 중 2,201 passed, 기존 runtime 경계 4개, failed 0**; stage **7,168 checks**(새 정적 검사 40건)와 words **6,618 cases** 통과. 회귀 테스트 4개를 추가했다. capture graph 경계 40건(ordered-effect 9건, modifier-value 31건)과 static analysis 경계 3건을 성공 수와 구분한다. upstream suite, optimizer/CUDA, GitHub CI 실행을 주장하지 않는다. C DLL release/source review pin 구분을 유지한다.

기준 소스: [cf.c taAV/tNVc/tac/taaa](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L200), [p.c modifier application 및 재삽입](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c#L969).

<a id="adverbial-hook-및-derived-conjunction-적용-2026-10-03"></a>
##### Adverbial hook 및 derived conjunction 적용 (2026-10-03)

- [x] P3: `cf.c::tac`에 따라 `(A C)`는 먼저 `t=u A`를 구성하고 `t C u`에 원래 입력을 전달한다. `+ (/@:)`의 Insert와 Atop이 같은 입력 함수 객체를 참조하는지 검증한다. 앞 절에서 기록한 `tac` 미지원 경계는 이 단계에서 해소했다.
- [x] P3/P5: `tca`는 `u C v` 결과에 A를 적용하고, `tcc`는 같은 원래 입력에 첫 C와 둘째 C를 순서대로 적용한 뒤 결과들을 bident로 구성한다. `taav`는 왼쪽·오른쪽 입력에 각각 A를 적용하고 고정 V와 trident를 구성한다. 실제 결과 품사로 hook·insert·fork 또는 modifier를 구성하며 row 4가 완료된 `Item`을 parser stack에 재삽입한다. 알려진 derived conjunction의 left/right binding도 이 경로를 사용한다.
- [x] P4/P5: 반복 사용되는 concrete noun은 Owned 저장소를 Shared로 전환해 payload를 복사하지 않는다. Group 내부 noun도 처리하며 65,536개 정수의 원래 payload pointer, 공유 identity, 원본 해제 후 수명을 검증한다. deferred expression은 실행하지 않는다. 함수 DAG와 이전 정의는 immutable로 유지하고 현재 사용 span 및 named alias snapshot을 보존한다. binding/version 관찰은 재사용 허용 조건이 아니다.
- [x] P6: C의 함수 표현·결과 품사, 첫 constructor 오류가 다음 action을 막는 순서, 실패 후 기존 assignment target 유지, 원래 conjunction을 재정의한 뒤 alias identity를 검증한다. 회귀 테스트 5개, C 비교 문장 16개, 정적 stage 검사 36개를 추가했다.
- [ ] noun-left rank/gerund, 다른 derived conjunction trident(`tcVCc` 등), train 내부 late modifier NameRef 조회·effect 계약, bident/trident의 즉시 noun 실행을 연결한다. 구성된 함수의 kernel 실행 범위와 locale/explicit-local/definition scope는 별도 미완료 항목이다. 최적화는 아직 구현하지 않는다.

Windows 검증: default/portable 각각 **275 passed / 17 ignored**, fmt/clippy 통과, Python harness **20 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture에서 **2,221문장 중 2,217 passed, 기존 runtime 경계 4개, failed 0**; stage **7,204 checks**, words **6,618 cases** 통과. capture graph 경계 42건(ordered-effect 9건, modifier-value 33건)은 성공 수와 구분한다. static analysis 경계는 computed rank와 `candidate=: + (@: + @:) -`의 `tcVCc` 2건이다. 앞 절의 3건 중 `tac`와 derived conjunction bident 경계는 해소했고, 다른 trident의 실제 C 성공/품사와 Rust Unsupported를 새로 보고한다. stage의 9⁴ 표 검사는 declarative eligibility 비교이며 전체 parser action trace의 증명이 아니다. C DLL release pin과 검토한 source pin을 구분하며 전체 J/upstream suite, CUDA, GitHub CI 실행을 주장하지 않는다.

기준 소스: [cf.c tac/tca/tcc/taav 및 modifier train dispatch](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L200), [p.c conjunction application](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c#L969).

<a id="modifier-trident-action-확장-2026-10-03"></a>
##### Modifier trident action 확장 (2026-10-03)

- [x] P3/P5: `cf.c`의 `tNVvc`, `tcVCc`, `tcaa`, `tNVca`, `tNVcc`, `taVCNV`, `taca`, `tacc`, `tcVCNV`, `tcca`를 연결하고 기존 `taaa`/`taav`도 동일한 trident dispatch에 통합했다. 각 action의 중간 결과·원래 입력·고정 operand와 적용 순서를 보존한다. 결과는 실제 품사로 bident/trident construction에 전달하며 immutable 함수 DAG를 유지한다.
- [x] P4/P6: `tcVCc`의 두 conjunction 결과가 원래 함수 입력을 공유하는지, constructor의 첫 오류가 뒤 action과 assignment를 막는지 검증했다. named trident conjunction alias는 원래 이름 재정의 후에도 기존 identity를 유지한다. 중간 noun 실행 및 train 내부 late modifier lookup은 여전히 별도 경계다.
- [x] P6: 20개 derived trident 품사 production을 `/`, `@:`, `+`, `3` 표본과 noun/verb 입력 조합으로 구성한 68문장을 C와 비교했다. 이 표본 검증은 모든 primitive 및 effect 조합에 대한 완전한 호환성 증명이 아니다. 회귀 테스트 3개, 비교 문장 98개, stage 검사 154개를 추가했다.
- [ ] 다음: noun-left rank construction, 즉시 noun 실행과 capture, train 내부 late modifier 조회, locale/definition scope를 구현한다. 앞 단계에서 미지원이던 `tcVCc`는 해소했다. tokenizer/enqueuer/parser 우선순위와 최적화·CUDA 보류를 유지한다.

Windows default/portable 각각 **278 passed / 17 ignored**, fmt/clippy 통과, Python **20 passed**. j64/AVX2 각각 세 실행 경로에서 **2,319문장 중 2,315 passed, runtime 경계 4개, failed 0**; stage **7,358 checks**, words **6,618 cases** 통과. capture graph 경계 44건(ordered-effect 9건, modifier-value 35건)은 성공 수와 구분한다. static 경계 2건은 computed rank와 `candidate=: 3 (" /) 1`의 noun-left rank이며 이전 `tcVCc` 경계를 후자로 교체했다. DLL release/source review pin, 지원 부분집합, 실제 실행한 검증 범위를 유지한다. GitHub CI는 생략했다.

기준 소스: [cf.c modifier trident actions](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L200).

<a id="noun-left-rank-구성과-operand-역할-보존-2026-10-03"></a>
##### Noun-left rank 구성과 operand 역할 보존 (2026-10-03)

- [x] P3/P5: `cr.c::jtqq`에 따라 ordinary noun-left rank를 구성한다. 오른쪽 rank의 rank→length→numeric audit를 먼저 수행하고 왼쪽 noun 또는 verb, 오른쪽 noun 또는 verb의 원래 순서·값·span을 immutable Rank entity에 보존한다. noun 저장소는 공유한다. derived modifier binding/trident를 통한 적용도 같은 경로를 사용한다.
- [x] P4/P6: runtime에서 계산된 `(i.4)` noun과 rank noun의 두 생산 occurrence를 construction capture에 연결한다. 정적 경로는 computed noun을 실행하지 않고 명시적 경계를 유지한다. 실패 시 기존 assignment target과 version을 유지한다.
- [x] P5/P6: `3"+`에서 오른쪽 `+`를 실제 실행 대상으로 오인하거나 왼쪽 `3`을 rank specification으로 오인하지 않는다. Rank graph form 및 shape/type 추론은 왼쪽 function operand가 있는 기존 지원 형태에만 적용하고 noun-left 형태는 구조를 보존한 Modifier와 unknown facts로 남긴다. 회귀 테스트 5개, C 비교 문장 17개, stage 검사 21개를 추가했다.
- [ ] boxed rank-1 noun의 gerund audit(오른쪽 rank가 모두 최대값인 경우 제외), constant-rank 함수의 kernel 실행·Logical lowering, 즉시 noun 실행, late modifier 조회 및 locale/definition scope를 연결한다. plain noun-left constructor 성공이 모든 gerund나 실행 지원을 뜻하지 않는다. 최적화/CUDA는 보류한다.

Windows default/portable 각각 **283 passed / 17 ignored**, fmt/clippy 통과, Python **20 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture에서 **2,336문장 중 2,332 passed, runtime 경계 4개, failed 0**; stage **7,379 checks**, words **6,618 cases** 통과. capture graph 경계 44건(ordered-effect 9건, modifier-value 35건)은 별도이며 static 보고 경계는 computed rank 1건으로 줄었다. gerund 및 다른 computed noun 구간의 미지원까지 없어졌다는 의미는 아니다. stage source hash에 `cr.c`를 추가해 실제 rank-constructor 검토 소스를 식별한다. GitHub CI는 생략했다.

기준 소스: [cr.c jtqq](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c#L731).

<a id="primitive-gerund-구성과-audit-순서-2026-10-03"></a>
##### Primitive gerund 구성과 audit 순서 (2026-10-03)

- [x] P3/P6: `ap.c::jtbslash`→`cg.c::jtfxeachv(1)`에 따라 noun에 `\`를 적용할 때 rank→length→boxed type 순서로 검사한다. 비어 있는 rank-2 입력은 RankError, 빈 rank-1 입력은 LengthError, nonempty nonboxed 입력은 DomainError로 처리한다. 이전 blanket Unsupported를 이 검사 범위에서 해소했다.
- [x] P3/P5: `r.c::jtfx`의 character primitive leaf를 현재 core PrimitiveResolver로 검증한다. char rank→length→ASCII spelling 및 최종 verb POS를 확인한다. primitive 문자열로 구성된 gerund는 PrefixInfix의 noun operand를 그대로 가진 completed Verb로 구성하고 공유 저장소·원래 span을 보존한다. execution-only decoded fgh는 semantic child로 추가하지 않는다.
- [x] P3/P6: noun-left rank의 gerund 후보는 동일 audit를 사용한다. 확실한 J audit 실패는 `cr.c`처럼 조용히 constant noun으로 되돌리고, 이름/미등록 primitive/compound AR에 대한 구현 미지원은 숨기지 않는다. gerund 원소 순서, 첫 오류, 실패 시 target/version 유지와 fallback을 검증했다.
- [x] P6: 회귀 테스트 3개, C 비교 문장 38개, stage 오류 검사 3개를 추가했다. source hash에 `ap.c`, `cg.c`, `r.c`를 추가했다. 정적 분석의 computed gerund noun 경계는 runtime constructor 성공과 별도로 보고한다.
- [ ] 다음: 이름 및 복합 atomic representation의 `fx` decoding/binding, gerund 실행·Logical lowering, 중간 noun 즉시 실행/capture, train 내부 late modifier 조회와 locale/definition scope를 진행한다. 이 단계는 전체 gerund 지원이나 optimizer/CUDA 구현이 아니다.

Windows default/portable 각각 **286 passed / 17 ignored**, fmt/clippy 통과, Python **20 passed**. j64/AVX2 각각 세 실행 경로에서 **2,374문장 중 2,370 passed, 기존 runtime 경계 4개, failed 0**; stage **7,382 checks**, words **6,618 cases** 통과. capture graph 경계 44건(ordered-effect 9건, modifier-value 35건)은 별도다. static 보고 경계 2건은 computed rank와 computed gerund noun으로, 성공 수나 C와의 의미 불일치로 세지 않는다. 완전한 upstream suite나 GitHub CI 실행을 주장하지 않는다.

기준 소스: [ap.c jtbslash](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ap.c#L940), [cg.c fxeachv](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cg.c#L101), [r.c fxchar/fx](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/r.c#L77).

<a id="복합-gerund-atomic-representation-해석-2026-10-03"></a>
##### 복합 gerund atomic representation 해석 (2026-10-03)

- [x] P3/P5: `r.c::jtfx`의 boxed AR을 재귀 해석한다. primitive 단일 head, noun `0`, hook `2`, fork `3`, modifier `4`, primitive adverb/conjunction 적용 및 boxed head로 구성된 derived modifier 적용을 지원한다. decoder는 기존 parser의 bident/trident disposition과 constructor action을 사용해 실제 결과 품사를 반환한다. 새 J 문법이나 modifier 전용 semantic AST를 만들지 않는다.
- [x] P3/P6: AR의 boxed type→rank→length, header와 operand vector의 audit, fork의 f→g→h 및 modifier의 왼쪽→오른쪽 audit를 보존한다. hook/modifier train AR은 명시적으로 h를 먼저 해석하고 g→f를 따른다. 마지막 순서는 두 제공된 Windows C DLL에서 확인한 C argument 평가 순서이며, C 언어 일반의 보장이나 Linux compiler 순서로 주장하지 않는다. noun-left rank의 확실한 audit 실패는 기존처럼 constant fallback으로 처리한다.
- [x] P4/P5: J-visible gerund noun과 현재 construction span을 그대로 보존하며, decoded execution auxiliaries를 parent의 semantic child로 추가하지 않는다. 공유된 65,536개 정수 noun AR의 payload pointer와 원본 해제 후 수명을 검증한다. 재귀 해석·중첩 constructor에는 기존 depth limit을 적용한다. AR 내부 byte offset을 모르는 경우 바깥 operand span을 사용한다.
- [x] P6: 회귀 테스트 4개, C 비교 문장 103개를 추가했다. `frontend_probe`의 `R`은 명시적 runtime parser construction/capture 관찰이며 completed function을 C `5!:1`/`4!:0`와 비교한다. `A`의 read-only/static 계약은 유지한다. stage에는 constructor 표현/품사 39건, 오류 18건, setup/기존 target 검사 46건을 추가했다. AR fixture 자체의 잘못된 setup은 공통 오류로 통과시키지 않고 검사 실패로 처리한다.
- [ ] 다음: gerund name/locative와 전체 spellin inventory, serialized entity가 즉시 noun 실행을 요구하는 경우의 host/capture 연결, train 내부 late modifier 조회, gerund 실행·Logical lowering 및 locale/definition scope를 진행한다. 이름·미등록 primitive·미지원 즉시 실행은 명시적 Unsupported이며 임의 품사나 fake noun으로 대체하지 않는다. 최적화/CUDA는 보류한다.

Windows default/portable 각각 **290 passed / 17 ignored**, fmt/clippy 통과, Python **20 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture에서 **2,477문장 중 2,473 passed, 기존 runtime 경계 4개, failed 0**; stage **7,485 checks**, words **6,618 cases** 통과. capture graph 경계 44건(ordered-effect 9건, modifier-value 35건)과 static 경계 2건(computed rank, computed gerund noun)은 별도다. 성공한 AR 해석이 모든 J primitive/name/실행을 지원한다는 뜻은 아니다. C DLL release/source review pin 구분과 declarative parse table/실제 trace 증명 범위 구분을 유지하며 GitHub CI는 생략했다.

기준 소스: [r.c fxchar/fx](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/r.c#L77), [cg.c fxeachv](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cg.c#L101), [cf.c hook 및 modifier dispatch](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L200).

<a id="gerund-이름의-생성-시점-품사-확인-2026-10-03"></a>
##### Gerund 이름의 생성 시점 품사 확인 (2026-10-03)

- [x] P3/P4: `r.c::jtfxchar`→`a.c::jtswap`→`sc.c::jtnameref`에 따라 character AR의 ordinary name을 기존 이름 환경에서 생성 시점에 조회한다. 문장 전체의 binding을 미리 snapshot하지 않는다. undefined 이름은 Verb NameRef로, 정의된 함수는 현재 실제 품사의 NameRef로 구성한다. verb의 현재 primitive 값을 고정하거나 alias의 참조를 펼치지 않는다. 기존 extension 이름도 동일한 binding 경로를 사용한다.
- [x] P3/P6: 현재 noun·adverb·conjunction인 이름은 gerund의 최종 Verb 검사에서 DomainError를 낸다. noun-left rank의 확실한 audit 실패는 기존 constant fallback을 유지한다. 이름 재정의, undefined→verb, verb→noun/modifier, verb alias, 실패 후 target/version 유지와 원래 gerund noun의 함수 표현을 검증한다.
- [x] P3/P6: alpha로 시작하고 마지막 문자가 `.`/`:`가 아닌 character AR은 이름 검사 경로로 보낸다. ordinary name의 분류·검증은 enqueuer를 재사용하며 잘못된 문자·공백·끝 underscore는 IllFormedName으로 처리한다. primitive 경로와 boxed AR header의 spellin 경로는 구분한다. 이름 환경이 없거나 locative 지원이 필요한 경우에는 Unsupported를 유지한다.
- [x] P6: 회귀 테스트 3개와 공통 C 비교 문장 30개를 추가했다. stage는 runtime constructor의 함수 표현/POS 9건, 오류 7건, binding 설정·기존 target 실행 14건을 비교한다. source hash에 `a.c`, `sc.c`를 추가했다. decoded Verb NameRef가 값 snapshot을 갖지 않는지, named noun을 사용하는 중첩 fork가 필요한 noun snapshot 없이 성공하지 않는지도 검증한다.
- [ ] 다음: 복합 AR 내부 named noun의 실제 값 snapshot 및 capture, gerund 이름 조회의 binding/version 관찰, 즉시 noun 실행·capture, train 내부 late modifier 조회를 연결한다. 현재 noun 판별은 품사 검사만 가능하며 값이 필요한 중첩 constructor는 Unsupported다. 원래 noun operand를 유지하는 것만으로 decoded execution auxiliary의 snapshot이나 compiled reuse 조건이 보존되었다고 주장하지 않는다. gerund 실행·Logical lowering, locative/locale/definition scope, 전체 primitive inventory는 별도 미완료다. 최적화/CUDA는 보류한다.

Windows default/portable 각각 **293 passed / 17 ignored**, fmt/clippy 통과, Python **20 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture에서 **2,507문장 중 2,503 passed, 기존 runtime 경계 4개, failed 0**; stage **7,515 checks**, words **6,618 cases** 통과. capture graph 경계 46건(ordered-effect 9건, modifier-value 37건), static 경계 2건(computed rank, computed gerund noun)은 별도 보고한다. 현재 gerund 이름 조회는 완전한 capture witness/재사용 guard가 아니다. DLL release pin과 검토 source pin을 구분하며 전체 J/upstream suite나 GitHub CI 실행을 주장하지 않는다.

기준 소스: [r.c fxchar](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/r.c#L77), [a.c swap](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/a.c#L21), [sc.c nameref](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sc.c#L395), [sn.c vnm](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sn.c#L9).

<a id="gerund-내부-noun-snapshot과-조회-capture-2026-10-03"></a>
##### Gerund 내부 noun snapshot과 조회 capture (2026-10-03)

- [x] P3/P4: character AR의 named noun을 생성 시점의 실제 `Value`로 가져와 shared Literal로 decode한다. noun-left fork 및 rank operand의 값이 뒤 이름 재정의에 따라 변하지 않도록 한다. abstract noun은 값을 추측하지 않으며 값이 필요한 constructor에서 기존 Unsupported를 유지한다.
- [x] P4/P5: `FunctionEntity.decoded_gerund`에 완료된 decoded 함수들을 보존한다. 이 정보는 intrinsic noun snapshot을 가진 immutable 공유 객체이며, 원래 boxed gerund noun은 source operand에 그대로 남는다. decoded 함수들을 원래 PrefixInfix/Rank의 semantic child edge로 추가하지 않는다. 성공한 gerund audit만 decoded 결과를 보존하고, rank의 quiet constant fallback에서는 부분 결과를 버린다. 이 보존은 gerund 실행 지원을 뜻하지 않는다.
- [x] P4/P6: 생성 중의 이름 조회마다 `GerundNameResolved`에 이름·현재 binding version(undef는 없음)·실제 품사·noun facts·outer AR span을 기록한다. 배열 payload는 capture event에 넣지 않는다. 성공·실패·quiet fallback에서 조회 순서를 유지하고, 해당 constructor attempt/outcome 사이에서만 유효하도록 verifier를 확장했다. `CapturedGraph.gerund_name_reads`는 별도 observation sidecar이며 compiled reuse guard가 아니다. static 분석의 내부 이름 dependency/guard 계약은 아직 완성하지 않았다.
- [x] P6: 회귀 테스트 4개를 추가했다. 65,536개 정수의 payload pointer 공유, 재정의·원본/Engine 해제 후 수명, rank noun snapshot과 verb NameRef의 차이, 첫 오류와 기존 target/version 보존, 부분 decode 폐기, 잘못 배치된 capture event 거절을 검증한다. 공개 정적 분석은 abstract bound noun 경계를 그대로 유지하며 실행·assignment를 수행하지 않는다.
- [x] P6: C 공통 비교 문장 31개 및 stage 검사 49건을 추가했다. `frontend_probe`의 `D`는 생성된 객체의 decoded 함수 표현을 관찰한다. C에만 적용한 fix adverb `5!:0`의 AR decode 결과를 `5!:1`로 읽어 scalar/vector/boxed named noun snapshot 6건을 비교하고, C 객체의 재정의 후 snapshot 유지 3건을 별도 확인한다. Rust에서는 외래 실행으로 우회하지 않는다. 기존 `R`의 원래 함수 표현/POS 비교도 유지한다.
- [ ] 다음: bident/trident AR의 즉시 noun 실행 및 capture, train 내부 late modifier 조회·effect 계약을 연결한다. gerund 실행/Logical lowering, compiled reuse 조건, locative/locale/definition scope 및 전체 primitive inventory는 별도 미완료다. tokenizer/enqueuer/parser 우선순위와 최적화/CUDA 보류를 유지한다.

Windows default/portable 각각 **297 passed / 17 ignored**, fmt/clippy 통과, Python **20 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture에서 **2,538문장 중 2,534 passed, 기존 runtime 경계 4개, failed 0**; stage **7,564 checks**, words **6,618 cases** 통과. capture graph 경계 46건(ordered-effect 9건, modifier-value 37건), static 경계 2건(computed rank, computed gerund noun)은 별도 보고한다. C DLL release pin과 검토 source pin을 구분한다. 전체 J/upstream suite·GPU·GitHub CI 실행은 주장하지 않는다.

기준 소스: [r.c fxchar/fx 및 noun/fork decode](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/r.c#L77), [sc.c nameref의 noun 값 반환](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sc.c#L395), [cg.c fxeachv의 decoded gerund 보존](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cg.c#L101), [cr.c gerund audit 및 constant fallback](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c#L731).

<a id="bidenttrident-ar-내부-noun-실행과-capture-2026-10-03"></a>
##### Bident/trident AR 내부 noun 실행과 capture (2026-10-03)

- [x] P3/P4: `cf.c::jthook`의 즉시 적용 production에 따라 `V N` bident는 monad를, `N V N` trident는 dyad를 runtime semantic host에서 실행한다. decoder와 modifier construction의 공통 disposition을 유지하며 실제 noun `Item`을 반환한다. 이를 바깥 noun-left fork 등의 operand로 전달하고 shared Literal 및 기존 decoded gerund 구조에 결과 값을 보존한다. 최종 gerund Verb 검사 전에 함수 실행 오류가 먼저 발생하도록 한다.
- [x] P4: constructor 동안 host를 잠깐 mutable로 빌려 실제 call을 실행한다. 이후 AR 이름 조회는 갱신된 환경을 읽으며 문장 전체 binding snapshot을 만들지 않는다. static/no-host 경로는 Unsupported를 반환하고 실행하지 않는다. 지원 범위는 기존 runtime verb executor 범위이며 미지원 callable을 임의로 계산하지 않는다.
- [x] P4/P6: `ConstructorApply`는 완료된 call의 함수 객체·valence에 따른 입력 facts·span·성공 결과 facts 또는 오류 class/context를 기록한다. 배열 argument/result payload는 event에 보관하지 않는다. 이름 조회 event와 같은 buffer를 사용해 실제 순서를 유지하고 바깥 construction 성공·실패 전에 전달한다. verifier는 해당 row 3/4 construction 구간 안의 Verb call만 허용한다. `CapturedGraph.constructor_calls`는 관찰 sidecar이며 재실행 계획이나 최적화 guard가 아니다. 실행된 함수 내부의 전체 효과/조회 trace까지 포착한 것은 아니다.
- [x] P6: 회귀 테스트 4개로 monadic/dyadic 실제 값, call 성공 뒤의 최종 DomainError, call의 DomainError/LengthError와 quiet rank fallback, 기존 target/version 유지, 잘못 배치된 call event 거절을 검증했다. monadic `+`의 65,536개 정수 payload 공유·원본/Engine 해제 후 수명, Windows hook AR의 g→f 조회 순서도 확인했다. mock host 검사는 첫 call 이후의 binding 변화가 다음 조회에 반영되고 static 경로는 call하지 않는다는 연결 계약을 검증하며 전체 J effect 호환성 증명으로 취급하지 않는다.
- [x] P6: 공통 C 비교 문장 44개와 stage 검사 56건을 추가했다. stage는 함수 표현/POS 19건, 오류 8건, setup 17건 및 computed snapshot setup 8건·decoded noun 값 4건을 비교한다. decoded 값은 C-only `5!:0`→`5!:1`과 Rust constructor 관찰 `D`로 교차 확인한다. Rust frontend를 외래 실행으로 우회하지 않는다.
- [ ] 다음: train/AR 내부 late modifier NameRef의 조회 및 적용·effect 계약을 연결한다. 미지원 derived callable, gerund 실행/Logical lowering, compiled reuse 조건, 전체 내부 효과 graph, locative/locale/definition scope와 primitive inventory는 계속 별도 과제다. tokenizer/enqueuer/parser 우선순위를 유지하고 최적화·CUDA·GitHub CI는 진행하지 않는다.

Windows default/portable 각각 **301 passed / 17 ignored**, fmt/clippy 통과, Python **20 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture에서 **2,582문장 중 2,578 passed, 기존 runtime 경계 4개, failed 0**; stage **7,620 checks**, words **6,618 cases** 통과. capture graph 경계 46건(ordered-effect 9건, modifier-value 37건), static 경계 2건(computed rank, computed gerund noun)은 별도 보고한다. 공급된 Windows DLL release pin과 검토 source pin을 구분하며 전체 J/upstream suite·GPU·Linux·GitHub CI 검증을 주장하지 않는다.

기준 소스: [cf.c hook의 V N/N V N 즉시 적용](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L311), [r.c AR hook/fork decode 순서](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/r.c#L93), [cg.c 최종 gerund Verb 검사](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cg.c#L101), [cr.c quiet gerund audit](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c#L731).

<a id="nameless-modifier-snapshot과-train-내부-지연-조회-2026-10-03"></a>
##### Nameless modifier snapshot과 train 내부 지연 조회 (2026-10-03)

- [x] P3/P4: ordinary modifier 이름은 모두 NameRef가 되는 것이 아니다. C의 `VALTYPENAMELESS` lookup에 맞춰 primitive modifier 및 primitive ACV/noun으로만 구성된 modifier train은 queue에서 stack으로 들어갈 때 현재 immutable 함수 값을 공유한다. `adv=:/`를 담은 train은 이후 `adv=:1`에도 기존 값을 유지한다. 일반적인 재귀적 이름 없음·순수성 판정으로 확대하지 않는다.
- [x] P3/P4: 실제 NameRef를 포함한 train/AR의 modifier child는 실제 adverb/conjunction 적용 시점에 기존 runtime resolver로 조회한다. 같은 품사의 재정의는 반영하고, 저장된 품사와 현재 binding의 품사가 다르면 DomainError를 낸다. undefined 이름 및 alias 오류도 기존 resolver 계약을 따른다. 기존 train의 child를 바꾸거나 전체 DAG를 미리 펼치지 않는다. static/no-host 경로는 필요한 조회에서 Unsupported를 유지한다.
- [x] P4/P5: stack 시점의 `ModifierStacked`와 실제 적용 시점의 `ModifierResolved`를 구분한다. `CapturedGraph.modifier_stack_snapshots`는 이름·version·품사·함수 값·span을 보존하며 snapshot 이름을 late verb reference로 기록하지 않는다. 실제 조회는 constructor row/span과 현재 binding version을 관찰한다. 두 sidecar 모두 observation이며 compiled reuse guard나 replay 계약이 아니다.
- [x] P3/P6: character AR의 이름은 `fxchar`의 NameRef 경로를 유지한다. ordinary stack의 nameless shortcut을 AR 이름 decode에 적용하지 않는다. gerund 이름의 생성 시점 품사 확인과 실제 modifier 적용의 지연 조회를 별도 event로 검증한다.
- [x] P6: 회귀 테스트 4개로 nameless snapshot 유지, nonnameless adverb/conjunction 재정의, 저장 품사 검사, 실패 후 target/version 보존, immutable child 공유 및 AR 조회 순서를 검증했다. 기존 snapshot identity 테스트도 실제 stack 시점에 맞췄다. C 공통 문장 63개와 stage 검사 69건을 추가했으며 source hash에 `s.c`, `jtype.h`를 포함했다.
- [ ] 다음: 남은 modifier constructor/executor inventory와 explicit/local/locative/definition scope의 frontend 계약을 검토한다. 미지원 callable 및 내부 effect/dependency·compiled reuse 계약, gerund 실행/Logical lowering은 남아 있다. tokenizer/enqueuer/parser에 집중하며 최적화·CUDA·GitHub CI는 보류한다.

Windows default/portable 각각 **305 passed / 17 ignored**, fmt/clippy/build 통과, Python **20 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture에서 **2,645문장 중 2,641 passed, 기존 runtime 경계 4개, failed 0**; stage **7,689 checks**, words **6,618 cases** 통과. capture graph 경계 **71건(ordered-effect 9, modifier-value 62)**과 static 경계 **2건(computed rank, computed gerund noun)**은 별도 보고한다. 추가 modifier-valued 문장의 graph 경계는 noun 실행 실패나 새 runtime waiver가 아니다. DLL release pin `ded7793fe5795d79eda8e7138dce94aa056edf78`과 검토 source pin을 구분하며 전체 J/upstream suite의 동등성을 주장하지 않는다.

기준 소스: [p.c nameless stack lookup](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c#L693), [s.c binding 분류](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/s.c#L739), [jtype.h primitive/nameless flags](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/jtype.h#L1334), [cf.c train 분류](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L367), [sc.c 저장 품사 검사](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sc.c#L138), [r.c character AR](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/r.c#L77).

<a id="bidenttrident-constructor-inventory-전체-조합-검증-2026-10-03"></a>
##### Bident/trident constructor inventory 전체 조합 검증 (2026-10-03)

- [x] P3/P6: 실제 parser row 및 AR decoder가 사용하는 `bident_disposition`/`trident_disposition`을 frontend probe의 `--constructors`에서 관찰한다. 별도 Rust golden dispatch를 만들지 않는다. C `cf.c`의 `bidents[16]`·`tridents[64]`를 읽어 syntax error, 즉시 semantic apply, hook/fork, derived modifier 및 결과 품사 분류를 **80개 조합 모두** 대조한다. 표 밖에서 먼저 처리하는 `V V` hook과 `MARK` fork도 구분한다.
- [x] P6: C 표의 누락·중복·알 수 없는 operand/action/result 및 probe의 누락·중복 조합을 실패로 처리한다. source extractor 회귀 테스트 3개를 추가했다. 80개 조합이 맞는다는 결과를 runtime ptcol 전체 실행 순서나 모든 operand 값의 지원으로 확대하지 않는다.
- [x] P3/P6: 모든 16개 bident 및 64개 trident를 boxed AR로 구성하여 surface parser의 다른 reduction과 혼동하지 않고 실제 constructor에 전달한다. 공통 비교 문장 **160개**를 추가했고, stage에서는 AR setup 80건·오류 75건·최종 Verb 성공 5건을 C와 비교한다. 추가로 성공 5건의 decoded 함수 구조를 C의 reference-only `5!:0`/`5!:1`와 비교한다. noun/modifier 반환 후 gerund의 최종 Verb 검사에서 발생하는 DomainError와 불가능한 production의 SyntaxError를 구분한다.
- [x] P6: Rust 회귀 테스트로 invalid bident/trident와 `N V N` 결과의 최종 audit 실패 후 기존 함수·binding version 및 capture 유효성을 확인한다. static/no-host 경로가 필요한 noun 실행을 임의로 수행하지 않고 Unsupported를 유지하는지도 검증한다. 이번 변경은 constructor 지원 확대를 주장하지 않으며 기존 dispatch의 검증 범위를 넓힌다.
- [ ] 다음: explicit/direct definition의 입력 수집·실행 없는 frontend 구조와 local/locative/definition scope를 검토한다. 남은 callable inventory, 내부 effect/dependency·compiled reuse 계약, gerund 실행/Logical lowering은 별도 작업이다. tokenizer/enqueuer/parser 우선순위와 최적화·CUDA·GitHub CI 보류를 유지한다.

Windows default/portable 각각 **306 passed / 17 ignored**, fmt/clippy/build 통과, Python **23 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture에서 **2,805문장 중 2,801 passed, 기존 runtime 경계 4개, failed 0**; stage **7,935 checks**, words **6,618 cases** 통과. capture graph 경계 71건(ordered-effect 9, modifier-value 62), static 경계 2건은 유지한다. 새 waiver는 없으며 C DLL release pin과 검토 source pin을 구분한다.

기준 소스: [cf.c bident/trident 표와 hook dispatch](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L310), [cf.c 즉시 적용 및 modifier 구성](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L349), [r.c AR decode](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/r.c#L77).

<a id="definition-입력-수집과-실행-없는-source-구조-2026-10-03-def-1-일부"></a>
##### Definition 입력 수집과 실행 없는 source 구조 (2026-10-03, DEF-1 일부)

- [x] `definition_input.rs`의 `DefinitionInput`/`InputFrame`과 parser의 `frame_definition_input` 경로를 추가했다. ordinary sentence, 추가 입력 필요, 완료된 definition을 구분하고 definition operator·본문의 원본 byte span과 중첩 direct definition 범위를 보존한다. tokenizer의 기존 word formation을 재사용하며 본문 이름을 lookup하거나 noun으로 reduce하지 않는다. 이 구조는 enqueue 이전의 source framing이며 아직 `Program`의 DefinitionCode/FunctionEntity가 아니다.
- [x] 일반 `{{ ... }}`와 literal mode 1–4의 `m : 'body'`, `m : 0`을 수집한다. quote doubling을 해제한 본문과 LF 포함 quoted body를 보존한다. 문자열·NB. 주석 안의 brace를 구분자로 보지 않고 nested direct definition을 수집한다. block은 C `colon0`에 따라 앞뒤 ASCII 공백만 있는 단독 `)` 줄에서 끝내며 nested DD 안의 `)`는 바깥 block을 닫지 않는다. source API는 CRLF를 보존한다.
- [x] CLI stdin/script가 같은 physical-line collector를 사용한다. 지원되는 block/direct 입력은 닫는 줄까지 모은 다음 기존 미지원 오류를 한 번 보고하고 입력을 중단한다. incomplete 입력은 EOF에서 source span을 가진 입력 오류를 보고한다. CLI physical-line API는 줄 사이에 LF를 넣으며 원본 CRLF byte 보존은 source API의 계약이다. 본문 문장을 따로 실행하지 않는다.
- [x] 본문 enqueue의 `ExplicitDefinition` 환경에서 local copula를 global로 승격하지 않고 future name을 Name payload/lookup flag로 유지하는지 확인했다. 이는 local frame·binding·invocation 구현을 뜻하지 않는다. callback/Engine 없이 source 구조만 만들며 definition 본문의 future name이나 side effect를 생성 시점에 실행하지 않는다.
- [x] 회귀 테스트 7개와 CLI 대기/종료 테스트 1개를 추가했다. nested brace·quote·comment, padded terminator/CRLF, EOF, quote 해제 및 LF body, source span, local enqueue flags를 검증한다. 실제 CLI 프로세스는 닫는 줄 전에는 응답하지 않으며 완료 후 미지원 오류를 내고 본문을 실행하지 않는다. 기존 full-definition acceptance 테스트 **17개는 계속 ignored**이며 성공 capability로 계산하지 않는다.
- [x] stage에 source projection 14건, C `;:` 대비 body words 20건, C literal decode 대비 quoted body 5건, 입력 경계 golden 5건을 추가했다. C에서는 완료된 direct fixture 및 block과 동등한 explicit string fixture의 구성 가능성을 확인한다. C `m : 0` 입력 callback이나 전체 preparse/control-flow 동등성을 검증했다고 주장하지 않는다. 실제 body 생성·호출 테스트를 통과했다고도 주장하지 않는다. source hash에 `cx.c`, `wc.c`, `io.c`를 추가했다.
- [ ] 다음: DefinitionCode와 invocation frame을 분리한 semantic constructor 및 body/control-word 구조를 설계·구현하고, 기존 9-row parser에 completed definition entity를 연결한다. `{{)n` 등 tagged DD, 같은 문장의 여러 root DD, computed/grouped colon operands, `define` alias, modes 0/9/13는 현재 source framing 지원 범위 밖이다. unknown scope·callable을 임의로 global/static하게 처리하지 않는다. 최적화·CUDA·GitHub CI는 보류한다.

Windows default/portable 각각 **314 passed / 17 ignored**, fmt/clippy/build 통과, Python **23 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture의 기존 **2,805문장 중 2,801 passed, runtime 경계 4개, failed 0**; stage **7,979 checks**, words **6,618 cases** 통과. capture graph 경계 71건(ordered-effect 9, modifier-value 62)과 static 경계 2건을 유지한다. definition 입력 framing의 비교는 full-J 실행 지원과 구분하며 기존 waiver를 늘리지 않았다.

기준 소스: [cx.c colon0 입력 종료](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L796), [cx.c quoted body line 분리](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L841), [cx.c DD token/nesting 처리](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L1345), [wc.c preparse](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wc.c#L385), [io.c definition 입력](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/io.c#L383).

<a id="definitioncode-생성valencecapture-provenance-2026-10-04-partial-def-12"></a>
##### DefinitionCode 생성·valence·capture provenance (2026-10-04, partial DEF-1/2)

- [x] `definition_code.rs`에 immutable `DefinitionCode`를 추가했다. 원본 source/form/span, decoded body, physical line 및 body-relative word span·품사·enqueue flags를 저장한다. local copula와 future name을 보존하며 본문 이름을 생성 시점에 조회하거나 본문을 실행하지 않는다. Code와 호출별 local value frame은 별개이고 후자는 아직 미구현이다.
- [x] enqueuer는 완료된 literal 정의를 N C N으로, direct definition은 C처럼 괄호가 있는 `(9 : 'body')`로 전개한다. 기존 parser row 4에서 Code를 생성하고 row 7에서 binding을 commit한다. direct body의 첫 word 앞 공백/초기 LF 정리도 C 기준을 따른다. enqueue에서 Code를 미리 만들어 오류 순서를 바꾸지 않는다. explicit colon 오른쪽의 length error가 먼저 발생할 때 생성·commit은 일어나지 않는다.
- [x] actual body Name의 `u/m`, `v/n`, `x/y`를 이용해 direct mode/POS를 추론한다. 문자열·주석의 철자를 이름으로 취급하지 않는다. spaces-only `:` 구역, mode 4 dyad 선택, operator x/y 여부와 default valence 이동을 보존한다. x/y 없는 modifier가 두 valence를 정의하면 `ValenceError`를 내고 기존 binding/version을 유지한다. 원본 본문과 valence 재배치 후 함수 표현을 분리한다.
- [x] 완료된 정의는 `FunctionHead::ExplicitDefinition`으로 보존되며 CLI가 닫는 줄 이후 계속 입력을 받을 수 있다. 일반 실행 오류 뒤 stdin 세션을 계속 읽고 실패 status는 유지한다. 미완료·미지원 정의는 여전히 중단하며 본문 줄을 별도 문장으로 실행하지 않는다.
- [x] parser capture Input에 expanded enqueue word index를 보존한다. generated mode/body noun은 같은 DD source span을 공유하므로 span만으로 하나의 원본 단어라고 재해석하지 않는다. capture→J graph는 전체 enqueue의 index/span/payload/facts를 검증한다. 잘못된 index는 거부한다. VerbValue의 Code를 unknown 계약으로 보존하지만 내부 본문 graph를 분석했다고 주장하지 않는다. modifier-value graph와 A3 callable lowering은 미지원이다.
- [x] Code 회귀 테스트 6개와 CLI 오류 후 continuation 테스트 1개를 추가했다. source/local flags, future binding 미조회, mode/POS·valence, 본문 미실행, 실패 transaction, row 4 오류 순서, generated capture 검증을 검사한다. full definition acceptance **17개는 계속 ignored**다.
- [x] 공유 C corpus에 21문장, stage에 28검사를 추가했다. j64/AVX2에서 생성된 함수 품사·atomic 표현, alias, source/semantic body 구분, multiline valence, 미실행 counter와 실패 후 기존 함수 보존을 비교한다. native C block input callback 및 full preparse/control-flow의 동등성 검증은 아직 아니다.
- [ ] 다음: `wc.c::getsen/conword/preparse`의 control-word 분할·구조와 nested/tagged 정의, 여러 root DD, computed/grouped colon operands를 구현한다. Code 호출용 local frame, runtime name/POS lookup·scope, complete J graph body 분석과 A3 lowering은 별도 단계다. optimizer·CUDA·GitHub CI는 진행하지 않는다.

Windows default/portable 각각 **321 passed / 17 ignored**, fmt/clippy/build 통과, Python **23 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture **2,826 cases / 2,822 passed / 기존 runtime 경계 4 / failed 0**; stage **8,007 checks**, words **6,618 cases** 통과. capture graph 경계 78(ordered-effect 9, modifier-value 69), static 경계 2를 별도 기록한다. 새 modifier 정의 7건의 graph 경계는 실행 비교의 실패/면제가 아니다. upstream full suite는 실행하지 않았다. DLL release `ded7793...`와 source review `13994ff...`는 다른 revision이다.

기준 소스: [cx.c colon·valence 분리](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L1264), [cx.c xop·mode 추론](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L1282), [cx.c direct definition 전개](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L1456), [wc.c control-word preparse](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wc.c#L385).

<a id="control-word-분할과-body-진단-위치-2026-10-04-partial-def-2"></a>
##### Control-word 분할과 body 진단 위치 (2026-10-04, partial DEF-2)

- [x] `definition_control.rs`의 `ControlWord`/`DefinitionPart`/`partition_line`을 추가했다. `wc.c::conword/getsen`을 따라 tokenizer가 만든 실제 word를 고정 control word 20개와 named `for_`·`goto_`·`label_`로 분류한다. control 사이의 실행 문장과 원본 byte span, control 직전 공백을 보존하며 leading/trailing 공백과 NB. comment는 제외한다. quoted text의 control 철자를 제어 구문으로 해석하지 않는다. `if.x`는 실제 word formation 결과가 `if.` + `x`이므로 If로 분할한다.
- [x] `for_`의 ordinary name을 검사하고 잘못된 `for_.`/`for_1a.`에는 IllFormedName을 반환한다. locative loop name은 Unsupported다. goto/label target 연결·유효성은 C와 같이 후속 audit 과제이며 이 분류기에서 완성했다고 주장하지 않는다.
- [x] DefinitionCode 생성 경로에서 partition을 먼저 검사한다. control-flow audit을 구현하기 전에는 control body를 Unsupported로 유지하고 binding을 commit하지 않는다. public partition API는 unmatched control도 분할할 수 있지만 정의가 유효하다고 판정하거나 실행하지 않는다.
- [x] body tokenizer/enqueue/name 오류의 위치를 원본 소스 byte span으로 변환한다. direct body의 공백/초기 LF 정리, block의 물리 줄, quoted body의 quote doubling을 반영한다. body-local word index를 outer sentence index로 잘못 보고하지 않는다. 오류 kind와 기존 diagnostic context는 보존한다.
- [x] 회귀 테스트 5개로 전체 분류 inventory, named/invalid name, 공백·quote·comment·adjacent control, 미완료 구조와 실행 경계, 실패 후 기존 binding 유지, direct/block/escaped quoted body의 진단 위치를 검증했다. Python에는 source table의 unknown/duplicate/length 변경 거부 테스트를 추가했다.
- [x] stage에서 pinned `wc.c`의 MATCHNAME8·length·control enum을 읽어 fixed inventory를 비교하고, C `;:`의 실제 words와 검토한 getsen 알고리즘을 사용하는 source projection 33건을 비교한다. UTF-8 quoted body와 모든 고정 control을 포함한다. 이는 C private getsen/preparse 실행 trace를 export한 것이 아니며 full control-flow 동등성 증거와 구분한다. 두 invalid-for definition과 실패 후 old-function 문장 4건도 공유 C 비교에 추가했다.
- [ ] 다음: `preparse`/`conall`/`congoto`의 control 구조 audit과 jump/section metadata를 구현하고 검증한다. 이후 호출별 local frame·name/POS lookup, nested/tagged/multiple DD, computed colon operand 및 J graph body 분석을 이어간다. Code의 control body 호출, 전체 definition acceptance, 최적화·CUDA·GitHub CI는 미지원/보류다.

Windows default/portable 각각 **326 passed / 17 ignored**, fmt/clippy/build 통과, Python **24 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture **2,830 cases / 2,826 passed / 기존 runtime 경계 4 / failed 0**; stage **8,045 checks**, words **6,618 cases** 통과. capture graph 경계 78과 static 경계 2는 그대로 별도 기록한다. full upstream suite 및 private C control-flow trace 비교는 실행하지 않았다. source review pin과 DLL release pin은 이전 절과 같다.

기준 소스: [wc.c conword 분류](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wc.c#L331), [wc.c getsen 문장 분할](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wc.c#L366), [wc.c preparse audit](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wc.c#L385), [sn.c vnm 이름 검사](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sn.c#L9).

<a id="definition-control-구조와-valence별-audit-2026-10-04-partial-def-2"></a>
##### Definition control 구조와 valence별 audit (2026-10-04, partial DEF-2)

- [x] `definition_flow.rs`에 control entry·jump metadata를 추가했다. `preparse/conall/conend`를 따라 if/elseif/else, while/whilst/for, break/continue, assert/return/throw, try/catch/catchd/catcht, select/case/fcase의 **정의 생성 단계**를 처리한다. 호출별 local frame과 본문 실행은 아직 구현하지 않았다.
- [x] physical sentence와 fragment별 word 범위를 함께 보존한다. body-relative span, physical line, valence별 target, assert marker 위치를 검증한다. `go`는 C의 control/error target이며 모든 정상 successor를 표현하는 CFG edge는 아니다. catcht의 runtime 처리를 정적 확정으로 해석하지 않는다.
- [x] 각 valence의 enqueue를 마친 뒤 control 구조를 검사하고, monad 검사를 끝낸 뒤 dyad를 처리한다. literal mode 4의 divider 이전 monad는 C처럼 검사에서 제외한다. 실패 시 기존 binding을 보존하고 본문의 assignment·name lookup·실행을 하지 않는다.
- [x] C의 control entry·sentence word·전체 word 한계를 반영했다. control entry 경계는 native C 비교로 검증했다. verifier는 valence 범위, jump 범위, physical line, word/source span 및 assert marker 참조를 검사한다. 전체 word 한계의 대규모 C 실측과 완전한 CFG 의미 증명은 미완료다.
- [x] 8개 control word의 길이 1–3 조합 **584개**를 C와 비교한다. C의 packed-code interval 검사로 허용되는 비정형 `while. if./while./whilst./for. end.`도 보존하고 `analysis_barrier`로 표시한다. 이를 structured lowering 대상으로 추론하지 않는다. nested loop·try·select target은 검토한 pinned C 알고리즘 기반의 별도 Rust 회귀 테스트로 확인한다.
- [ ] 다음: goto/label target·구조 진입 제한 audit, `canend`/BBLOCKEND 결과 자격 metadata. 호출 frame·runtime scope, nested/tagged/multiple DD, computed colon operand 및 Code 본문 graph lowering은 별도 단계다. 최적화·CUDA·GitHub CI는 보류한다.

Windows default/portable 각각 **340 passed / 17 ignored**, fmt/clippy/build 통과, Python **24 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture **3,501 cases / 3,497 passed / 기존 runtime 경계 4 / failed 0**, stage **8,677 checks**, words **6,618 cases** 통과. capture graph 경계 78과 static 경계 2는 별도 기록한다. 생성 결과·오류·atomic representation을 비교했으며 **private C control/jump trace를 export해서 비교한 것은 아니다**. upstream full suite와 definition 호출 acceptance 17건은 미실행이다. 보고서 10개의 실행 파일 및 source hash를 확인했다. DLL release `ded7793fe5795d79eda8e7138dce94aa056edf78`와 검토 소스 `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`는 서로 다른 revision이다.

Sources: [wc.c conend / packed interval](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wc.c#L72), [wc.c try/select](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wc.c#L111), [wc.c conall](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wc.c#L177), [wc.c preparse](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wc.c#L385), [cx.c valence ordering](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L1264).

<a id="이전-b-block-결과-자격-metadata-2026-10-04-partial-def-2"></a>
##### 이전 B-block 결과 자격 metadata (2026-10-04, partial DEF-2)

- [x] `PreviousResult::{Unresolved, CanReturn, CannotReturn}`를 추가하고 C `conall`의 역방향 고정점과 provisional bit 처리를 따라 기록한다. 대상은 **이전 B-block 결과**이며 현재 test 값, purity, CFG reachability 또는 최적화 허용 여부를 뜻하지 않는다. successor 결과가 일치하지 않거나 cycle이 확정되지 않으면 Unresolved를 유지한다.
- [x] CBBLOCKEND를 `before_fallthrough_end`로 보존한다. 후속 문장이 있는 non-select fallthrough end 바로 앞의 Body만 표시한다. 마지막 end, backward loop end, select end 및 assert/test를 구분하고 verifier에서 잘못 붙인 marker를 거부한다.
- [x] 회귀 테스트 4개로 loop·분기·assert/throw/return, valence 독립성, analysis barrier 보존 및 잘못된 참조를 검증했다. C corpus에 생성 사례 6개를 추가했다. 이는 source 기반 metadata 검증이며 C의 private canend trace 비교는 아니다. 실행과 최적화는 추가하지 않았다.
- [ ] 다음: goto/label target 및 구조 진입 제한을 audit하고 upstream goto 위치 matrix를 native C와 비교한다. invocation frame·scope 및 나머지 definition input 지원은 후속 단계다.

Windows default/portable 각각 **344 passed / 17 ignored**, fmt/clippy/build 통과; Python **24 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture **3,513 cases / 3,509 passed / 기존 runtime 경계 4 / failed 0**, stage **8,683 checks**, words **6,618 cases**. 보고서 10개의 binary hash를 확인했다. capture graph 경계 78과 static 경계 2, DLL/source review revision 구분 및 upstream full-suite 미실행 상태는 그대로다.

[wc.c CBBLOCKEND and canend](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wc.c#L238).

<a id="gotolabel-연결과-구조-진입-audit-2026-10-04-partial-def-2"></a>
##### Goto/label 연결과 구조 진입 audit (2026-10-04, partial DEF-2)

- [x] `congotoblk/congoto/congotochk`를 따라 **conall 이전**에 원래 control kind로 구조 interval을 만들고 target label의 다음 entry를 goto target으로 연결한다. label suffix는 원본 문자열로 보존하며 name lookup을 하지 않는다. 빈 suffix와 숫자로 시작하는 suffix도 일반 J name으로 재분류하지 않는다.
- [x] 같은 valence에서 target이 없거나 참조된 label이 중복되면 ControlError다. 참조되지 않은 duplicate label은 C처럼 허용한다. target prefix를 정확히 구분하고, 구조 안으로의 진입·sibling branch 이동은 금지하되 구조 밖으로의 이동은 허용한다. malformed interval은 안전한 bounds 검사로 오류 처리한다.
- [x] verifier가 named suffix와 원본 source 및 label successor 참조를 확인한다. 원본 quoted source의 duplicate label 위치를 보존하고 실패 후 binding을 유지한다. body 호출이나 branchout runtime stack 처리는 구현하지 않았다.
- [x] pinned `test/ggoto.ijs`의 select/if, while/try, if/for/whilst 3개 template에서 label과 goto를 넣을 수 있는 모든 gap **1,028개**를 비교했다. j64/AVX2 모두 생성 성공 332건·ControlError 696건으로 일치한다. upstream suite 전체 실행이나 private jump trace 비교는 아니다.
- [x] CLI 문장 단위 corpus에는 한 줄 equivalent를 사용하고, 원본 여러 줄·valence 구분은 stage probe에서 비교한다. 실제 여러 줄 quoted string을 CLI 한 case로 잘못 보내는 transport 문제를 발견해 수정했고, 이후 CR/LF 입력을 거부하는 regression guard를 추가했다. control 정의 생성의 성공을 호출 지원으로 계산하지 않는다.
- [ ] 다음: nested/tagged/multiple direct definition 및 computed/grouped colon operand의 framing·enqueue provenance를 확장한다. invocation/local frame·scope, Code 본문 structural graph와 A3 lowering은 별도 단계다. for locative name, 최적화·CUDA·GitHub CI는 현재 미지원/보류다.

Windows default/portable 각각 **348 passed / 17 ignored**, fmt/clippy/build 통과. Python **25 passed**(최종 CLI transport regression 포함). j64/AVX2 각각 direct·semantic-reference·parser-capture **4,563 cases / 4,559 passed / 기존 runtime 경계 4 / failed 0**, stage **9,724 checks**, words **6,618 cases**. 보고서 10개의 binary/source hash를 확인했으며 `test/ggoto.ijs` hash도 추가했다. capture graph 경계 78과 static 경계 2는 별도다. DLL release/source review pin 구분은 이전 절과 동일하다.

Sources: [wc.c goto audit](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wc.c#L14), [j.h half-open intervals and DO loop index](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/j.h#L1065), [upstream goto position tests](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/test/ggoto.ijs).

<a id="한-문장의-여러-direct-definition-보존-2026-10-04-partial-def-12"></a>
##### 한 문장의 여러 direct definition 보존 (2026-10-04, partial DEF-1/2)

- [x] `InputFrame::Definitions`로 disjoint root DD를 source 순서대로 보존한다. 각각의 delimiter/body span과 nested 범위 소유권을 분리한다. 뒤쪽 root가 미완성이면 전체가 NeedMore이며 앞쪽 root도 commit되지 않는다. nested 범위 수집은 semantic nested Code 지원을 뜻하지 않는다.
- [x] enqueuer는 각 root를 독립된 parenthesized `9 : body` constructor로 전개하고 gap의 ordinary word를 유지한다. 원본 source와 primitive context를 constructor끼리 Arc로 공유한다. gap 오류의 blame index도 전개된 queue index를 사용한다. body·constructor의 원본 위치는 전개 위치와 별개로 보존한다.
- [x] 기존 row 4·row 7과 Hook/Fork 처리를 사용하며 여러 DefinitionCode를 function operand로 유지한다. 본문 이름을 조회하거나 noun으로 실행/reduce하지 않는다. static prepare는 binding을 commit하지 않으며, 실패한 train 정의도 old binding을 보존한다.
- [x] 회귀 테스트 4개로 quote/comment·nested 소유권, incomplete collection, enqueue provenance/index·Arc 공유, structural train·body 미실행·binding 유지 및 CLI의 두 경로를 검증했다. C corpus에 15건을 추가해 실제 atomic representation과 오류·transaction을 비교했다. stage에는 input projection·incomplete 비교 4건을 더했다.
- [ ] 다음: semantic nested DD와 tagged DD, computed/grouped colon operands 및 한 문장의 mixed literal-colon/DD framing. 여러 ordinary root DD 지원을 모든 definition form 지원으로 확대 해석하지 않는다. invocation/local scope, A3 및 Code 본문 graph lowering은 후속 단계다. 최적화·CUDA·GitHub CI는 보류한다.

Windows default/portable 각각 **352 passed / 17 ignored**, fmt/clippy/build 통과, Python **25 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture **4,578 cases / 4,574 passed / 기존 runtime 경계 4 / failed 0**, stage **9,743 checks**, words **6,618 cases**. 보고서 10개의 binary/source hash를 확인했다. capture graph 경계 78과 static 경계 2는 별도이며, full upstream suite·definition invocation acceptance·private C trace 동등성은 미검증이다. DLL release와 source review pin은 이전과 같다.

Sources: [cx.c repeated DD expansion](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L1456), [p.c parser reduction](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c).

<a id="tagged-noun-dd의-raw-입력noun-보존-2026-10-04-partial-def-12"></a>
##### Tagged noun DD의 raw 입력·noun 보존 (2026-10-04, partial DEF-1/2)

- [x] `DefinitionForm::NounDirect`로 `{{)n ... }}`를 처리한다. 이는 함수 Code가 아니라 **문자 noun**이다. raw body의 quote·NB.·control spelling·`{{`를 word나 실행 문장으로 해석하지 않는다. 첫 물리 줄에서는 어디에 있는 `}}`도 종료하고, 다음 줄부터는 column zero의 `}}`만 종료한다. tag가 첫 줄 끝이면 초기 LF를 생략하고, 첫 줄에 body가 있으면 다음 줄 앞 LF를 보존한다.
- [x] 원본 delimiter/body byte span은 유지하면서 physical CRLF를 logical LF로 변환한다. 이후 root를 별도로 rescan하므로 raw body의 unmatched quote가 다음 ordinary/noun DD의 word formation을 오염시키지 않는다. ordinary DD 내부의 nested noun DD 및 다른 tag는 아직 Unsupported다.
- [x] enqueue는 raw noun을 원래 span·word index를 가진 Noun 한 개로 낸다. 길이 1의 char scalar와 빈/여러 byte char 배열을 구분한다. DefinitionConstructor나 local invocation frame을 만들지 않는다. 기존 noun reduction·assignment·snapshot 및 constant-noun fork 구성을 재사용한다. 두 noun의 단순 나열에는 C의 N/N syntax error를 유지하며 임의 concat 규칙을 추가하지 않는다.
- [x] Rust 회귀 테스트 6개로 raw 값/shape, multiline·column-zero 종료, mixed roots, enqueue provenance·noun snapshot·static 미commit, CLI 두 경로의 quote/comment·CRLF 및 UTF-8 byte-boundary panic을 검증했다. malformed 일반 primitive byte를 framing에서 잘못 slicing하지 않고 enqueue의 오류 경로로 넘긴다.
- [x] C corpus에 noun 사례 **161건**을 추가했다(고정 body 11개·seed 20261004의 body 64개, 값 관찰·snapshot·mixed train 포함). stage에는 **176 checks**를 추가했다. multiline 6건은 C `0!:100` script 경로로 실제 physical input을 공급해 LF/CRLF·빈 header·embedded delimiter·raw quote를 비교한다. 한 번의 multiline `JDo` 호출을 physical-line collection 증거로 사용하지 않는다.
- [x] oracle이 raw body의 unmatched quote 때문에 `;:` 관찰을 definition 실행 오류로 잘못 취급하지 않도록 copula prefix를 관찰한다. 원본 문장을 두 번 실행하지 않는다. Windows subprocess 입력의 UTF-8을 명시해 한글 raw noun의 byte 값도 비교한다. 두 adapter regression test를 추가했다.
- [ ] 다음: semantic nested DD와 nested noun DD, 다른 tagged/computed/grouped definition 및 mixed literal-colon/DD framing. unfinished input의 EOF error-class 정합도 별도 audit 대상이다. callable A3·본문 graph lowering·invocation/local scope, for locative name은 미완료다. 최적화·CUDA·GitHub CI는 보류한다.

Windows default/portable 각각 **358 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture **4,739 cases / 4,735 passed / 기존 runtime 경계 4 / failed 0**, stage **9,919 checks**, words **6,618 cases**. 보고서 10개의 binary/source hash를 확인했고 `test/g0x.ijs` hash도 기록했다. capture graph 경계 78과 static 경계 2는 별도이며 upstream full suite·definition 호출 acceptance·private C trace 동등성은 미검증이다. DLL release `ded7793fe5795d79eda8e7138dce94aa056edf78`와 source review `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`는 서로 다른 revision이다.

Sources: [cx.c noun DD raw collection](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L1413), [io.c physical input normalization](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/io.c#L316), [io.c script-line input](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/io.c#L362), [upstream string-script execution](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/test/g0x.ijs#L32).

<a id="static-frontend-review"></a>

<a id="정적-분석-수용-기준과-기존-frontend-구조-재검토-2026-10-03"></a>
##### 정적 분석 수용 기준과 기존 frontend 구조 재검토 (2026-10-03)

**사용자 수용 기준:** 실행 없이 연산 graph와 메모리 요구를 분석할 수 있으면 우선 충분하다. 정적 분석은 기본 경로로 유지한다. 실제 실행 + 별도 capture는 값 의존 구간의 의미 보존과 동적 관찰을 위한 경로이며, 지원되는 정적 분석을 제공하기 위한 전면 선행 조건이 아니다. 완전한 J runtime compatibility, capture, compiled reuse의 완료 상태는 별도로 추적한다.

정적 noun은 “J 품사 = Noun, graph origin, 추론된 facts, 선택적인 constant”를 가진다. concrete noun은 실제 `Value`와 별도 origin을 가진다. 같은 class matcher/row 규칙 아래 semantic action/context가 둘을 구분한다. Unknown 값이나 품사를 실제 값이 있는 Noun으로 가장하지 않는다. constant folding도 J 오류·효과·binding 의미를 보존하는 안전한 범위에 한정한다.

| 현재 파일/구조 | 그대로 유지 | 수정해야 하는 점 |
|---|---|---|
| `tokenizer.rs` | `w.c` transition table, raw spans, quote errors, parser-visible comment cutoff | capture 때문에 변경할 사항은 없다. runtime/capture/target 정보를 넣지 않는다 |
| `enqueuer.rs` | literal construction, core primitive POS, unresolved NAME, lookup/copula flags, word index/span | graph를 만들 필요는 없다. `EnqueueEnvironment`와 flags/provenance를 parser 입구 이후에도 전달하는 contract를 보완한다 |
| `parser.rs::ParseValue::Noun(Expr, usize)` | Noun class와 원본 표현/의미 구조 | concrete Value carrier와 static noun facts/origin carrier를 명시적으로 구분한다. Expr 하나를 concrete 값처럼 사용하지 않는다. capture origin은 값과 별도이며 보존된 static graph도 버리지 않는다 |
| `expression()` / queue drain | 같은 queue/stack 규칙과 name의 noun/function 구분 | `resolve_stack_item`이 실제 right-to-left queue→stack entry에서 조회한다. `ParseContext`가 analysis와 runtime noun snapshot을 구분한다. runtime invocation host와 top-level single-name non-final assignment는 구현했으며 explicit-local/locale/effect 확장은 미완료다. static은 안정된 binding/POS 정보만 사용하며 불명확하면 분석 경계로 남긴다 |
| rows 0–2 / `runtime.rs::eval_program` | monad/dyad 의미와 실제 kernel implementation | static action은 application graph와 facts를 만들고, concrete action은 그 지점에서 실행한 noun을 돌려준다. concrete reduction 후 전체 Expr를 다시 실행하여 중복 계산하지 않도록 runtime return contract를 함께 바꾼다 |
| `completed_noun()` / rows 3–6 | completed FunctionEntity DAG, source operator, ordered operands | `completed_noun`은 Literal/Group만 추출하지만 runtime rows 0–2가 먼저 실제 Literal로 reduce하므로 computed noun constructor도 처리한다. static context의 값 의존 boundary는 유지한다. static constructor는 필요한 값이 constant/proven이면 진행하고, 아니면 value-dependent 경계로 남긴다. concrete constructor는 실제 값과 origin을 받아 validation한다 |
| `Item` / row 7 / diagnostics | source spans, enqueue의 local/global/to-name 구분 | `Item`이 original-word range/inherited token과 enqueue flags를 보존하며 row 7은 `AssignmentSource`로 target/copula provenance와 flags를 남긴다. static final assignment는 proposal로 남기며 runtime row 7은 final/non-final assignment를 즉시 수행한다. local 실행 미지원 상태를 유지하면서 metadata를 조용히 global로 해석하지 않는다 |
| row 8 / graph adapter | 괄호에 따른 reduction boundary | grouping 전후 같은 noun origin을 유지한다. 괄호 자체를 추가 실행 operation으로 만들지 않는다. production 순서·operand slot·original word를 기존 J Graph adapter에 전달한다 |
| parser API / runtime / analysis | `prepare_semantic/analyze_j_graph(&self, ...)`의 비실행 성격 | `snapshot: bool`은 `ParseContext::{Analysis, Runtime}`으로 대체했다. `ActionContext`/`RuntimeParserHost`가 lookup/invocation과 optional capture를 통합한다. top-level assignment host action은 구현했으며 explicit-local/locale/definition 확장은 후속 작업이다. static 분석과 runtime capture는 동일 grammar를 사용하되 실행 권한과 반환 타입을 구분한다 |

이 변경은 tokenizer/enqueuer/parser 파일 분리를 되돌리는 작업이 아니다. 핵심은 **parser의 payload·semantic actions와 metadata 전달**이며, graph/capture를 lexer나 physical `Value`에 넣지 않는다. 근거는 현재 `parser.rs`의 `Noun(Expr, usize)`, `expression`, `completed_noun`, row 7의 `AssignmentSource`, `runtime.rs::eval_program` 및 [jsource p.c](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c)/[w.c](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/w.c)의 계약이다. 이전 구조 검토를 현재 parser 변경에 맞게 갱신했다. kernel/executor 변경은 포함하지 않는다.

**메모리 분석의 의미:** 현재 `j_graph_memory.rs`는 알려진 shape의 logical atom count, graph 순서의 live range, intermediate/view materialization 후보를 분석하고 명시적인 representation model로 byte 수를 평가한다. 이는 실제 allocation이나 peak device memory의 보장이 아니다. shape가 미정이면 Unknown/상징식/조건을 유지하고, physical schedule·layout·alias·variable-width boxed/sparse representation이 정해진 뒤 peak/residency를 별도 평가한다. source가 주는 구조와 사용자 input signature가 확보되면 배열 원소를 실행하지 않고 분석한다. 현재 모든 symbolic shape나 J form이 구현되었다는 의미는 아니다.

- [x] 세 파일의 재검토를 완료하고 tokenizer 유지, enqueuer metadata 전달 보완, parser/context/return contract 수정으로 범위를 좁혔다.
- [x] 최소 수용 기준을 static graph + logical memory analysis로 명시하고 full capture/JIT/physical peak 계산과 구분했다.
- [ ] **P2/P5 static-first interface:** 기존 static graph 경로를 유지하면서 static/concrete noun carrier와 같은 grammar의 actions를 정의한다. compile 요청이 runtime effects를 실행하지 않는 regression을 추가한다.
- [x] **F2/P4 supported lookup/provenance:** original word 범위와 inherited token·final copula flags를 유지하고 name resolve를 stack entry로 옮겼다. 지원되는 reduction의 구조와 실패 시 중단을 검증했다. top-level single-name non-final assignment는 후속 gate로 검증하며 explicit-local/locale/effect 동등성은 별도 미완료 항목이다.
- [ ] **P3/P6 value-dependent boundary:** constant constructor 사례는 분석하며 unknown 실제 값/품사에서는 경계와 reason을 반환한다. Unsupported analysis를 J syntax error로 바꾸지 않는다.
- [ ] **P5/P6 static memory gate:** input type/shape 또는 facts로 graph/liveness/extent를 분석하고 Unknown을 보존한다. 결과 보고에 logical atoms·represented bytes·추정 peak의 차이를 표시한다.

**구현한 provenance 계약:** `Program.reductions`와 정적 분석 결과의 `reductions`는 row id, 순서대로 나열한 operand word range, result word range/품사, byte span, inherited token을 가진다. actual noun payload는 복사하지 않는다. jsource `p.c`의 modifier·fork·bident hook은 왼쪽 operand의 `.t`, non-fork trident는 가운데 operand의 `.t`, 괄호는 `(`의 `.t`를 이어받는다. rows 0–2의 noun 결과는 오른쪽 noun token을 유지하며 C는 이를 non-executable noun에서 immaterial로 설명한다. 실패 operator의 blame token과 result token은 별도다. Rust의 index는 0부터 시작한다. 이 계약은 [p.c stack entry](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c#L735), [noun result](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c#L922), [modifier/train/parenthesis](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c#L1002)를 cross-check한 source 기반 regression이며 C 내부 stack trace를 직접 export한 검증은 아니다.

이번 regression 9개는 이름의 오른쪽→왼쪽 조회·각 stack entry noun snapshot, constructor 실패 후 미방문 이름 조회 중단, named modifier POS, 9개 row의 original-word 전달, grouping된 modifier/fork origin, final copula 보존, 원문 error token 및 named insert/fork 분석 경계를 검증한다. C corpus에도 13문장을 추가했다. `(entryverb/ % #) entrynoun`과 `(entryverb/ % #) entrycopy`는 frontend 구조를 보존하지만 기존 runtime executor가 처리하지 못한다. 보고서 `coverage_boundaries`에 exact source와 실제 C/Rust 결과·이유를 남기며 성공이나 C baseline deviation으로 세지 않는다. Rust가 다른 오류를 내거나 등록하지 않은 구문이 실패하면 일반 failure다. 나머지 2,061문장의 값/error 비교와 stage/word 검증을 통과했다.

위 static 분석 gate는 runtime capture 전부의 완료를 기다리지 않는다. 아래 capture 체크리스트는 별도 실행 경로의 완료 기준으로 유지한다.

<a id="구현-순서와-완료-체크리스트"></a>
##### 구현 순서와 완료 체크리스트

기존 F2/P2–P6에 속한 아래 항목을 순서대로 진행한다. 조사/계획 완료와 runtime 구현 완료를 혼동하지 않는다.

- [x] 공식 문서 및 ProxyTensor source에서 concrete/symbolic capture, lazy evaluation, graph break/guard, SSA 방식의 차이를 조사했다.
- [x] 실제 noun reduction + 별도 compilation capture를 결정하고 문법상의 verb-only 제한을 두지 않기로 했다.
- [x] 현재 deferred parser와 목표 runtime parser의 차이, 아래 implementation/test gate를 정본에 기록했다.
- [x] **P2/P4 — runtime row actions (지원 subset):** `RuntimeParserHost`가 같은 9-row engine에서 stack-entry name lookup과 rows 0–2 invocation을 수행하고 actual `Value`를 같은 stack에 재삽입한다. 미지원 effectful/locale/definition form은 별도 coverage boundary로 남긴다.
- [x] **P2/P5 — capture carrier:** opt-in `ParseCapture`, occurrence ids, input/output associations와 ordered attempt/success/failure events가 구현되었다. raw `Value`/primitive executor에 compiler identity를 넣지 않으며 capture parity 회귀를 유지한다.
- [x] **P3/P5 — construction provenance (지원 subset):** completed FunctionEntity DAG, construction attempt/success, computed noun occurrence origin을 capture에 연결했다. runtime actual noun과 static value-dependent boundary를 구분하며 전체 constructor vocabulary 지원 완료를 뜻하지 않는다.
- [x] **P4 — names/effects (지원 subset):** same-sentence name lookup/assignment/POS와 이미 commit된 effect 대 pending outer assignment의 순서를 capture/runtime 회귀로 보존한다. 일반 user locale/path와 미지원 effect form은 여전히 별도 경계다.
- [x] **P5/P8 — graph adapter (성공 capture subset):** capture의 input/constant/read/apply/construction dependency를 기존 J Graph로 변환하고 verifier를 통과시키는 adapter가 구현되었다. failed/opaque/dynamic boundary를 executable complete graph로 승격하지 않는다.
- [x] **P6 — 지원 corpus differential/capture gate:** Windows default/portable와 j64/AVX2 oracle에서 여러 후속 gate를 반복 실행했고 reports에 coverage boundary와 revision/hash를 남겼다. 이는 full runtime `ptcol` internal trace나 full-J/upstream/locale/definition acceptance 완료를 뜻하지 않는다.
- [ ] **후속 P5/P8 — static/reuse:** purity·error order·binding/value guards를 확보한 구간에서만 abstract actions, region compilation, safe reuse를 추가한다. capture 실행 경로의 completion gate는 runtime reduction + capture parity + verified J Graph다. 최소 static 분석 gate와 구분하며 production CUDA/JIT를 요구하지 않는다.

<a id="테스트-matrix와-수용-조건"></a>
##### 테스트 matrix와 수용 조건

| 검증 축 | 사례/방법 | 통과 조건 |
|---|---|---|
| Arithmetic topology | `a=:2`, `b=:3`, `c=:4` 후 `a+b*c`; `(a+b)*c`; monad chain | C 결과/오류 일치. capture에는 실제 reduction order와 producer-consumer edges가 남고 괄호 차이가 보존된다 |
| 동일 값, 다른 production | `(2*3)+(1+5)` | 두 6이 별도 origins이며 최종 add가 두 origin을 참조한다 |
| Constructor noun | `f=:+"(1+0)`; `f=:(1+2) + *`; 계산 rank/length/domain 오류 | C `4!:0`/`5!:1`, 결과 및 오류 일치. actual noun operand와 생산 graph가 연결되고 constructor-time 오류를 뒤로 미루지 않는다 |
| Completed verb structure | `+/ % #`, hook/fork, nested rank/atop | source operator·operand order·completed modifier 경계가 C atomic structure와 일치한다 |
| Naming/sequencing | noun assignment 이후 rebind, late function alias rebind, 지원되는 중간 assignment | noun snapshot/version과 function POS/late binding이 유지된다. trace 없는 eval과 effects/lookup 순서가 같다 |
| Failure and partial graph | `1+('a'+2)`, `(1 2+1 2 3)+('a'+1)` 및 두 실패 분기의 반대 배치 | C error class와 failure precedence 일치. 실패 노드 뒤 성공 output/outer commit이 없고 partial graph를 executable로 오인하지 않는다 |
| Exactly-once effects | runtime semantic-host test double로 invocation/assignment events 계수; C에서 지원된 J 문장 별도 비교 | capture on/off 실행 횟수 동일. recording failure나 replay가 effects를 중복시키지 않는다. host tests를 C full-J 지원 증거로 계산하지 않는다 |
| Memory/identity | 큰 array chain·alias 입력·복수 문장·capture 해제 후 temporary lifetimes 관찰 | per-node full-array copy와 diagnostic array retention 없음. BufferId로 semantic id를 생성하지 않는다. bounded metadata/constant policy 검증 |
| Reuse safety | shape/binding/POS/constructor 값이 바뀐 입력; branch 양쪽 | guard invalidation 또는 semantic region 실행. 한 번 trace한 branch를 universal program으로 재사용하지 않는다 |

**역사 상태 주의:** 이 문단을 처음 작성한 2026-10-03 시점에는 계획/참고자료 정리만 완료되어 있었다. 이후 runtime row actions, capture API, successful-capture→J Graph adapter와 지원 corpus differential은 구현·검증되었다. 현재 미완료는 full runtime `ptcol` 내부 trace 동등성, 일반 locale/definition/control/effect coverage, capture 기반 safe static reuse/guard, 그리고 전체 memory-retention/performance gate다. 아래 212 tests/7,014 stage 수치는 당시 gate의 역사 기록이지 최신 검증이 아니다.

<a id="f0--jsource-word-formation-이식"></a>
#### F0 — jsource word formation 이식

- [x] `w.c::state`의 character-class × state transition table을 Rust enum/table로 **직접 이식**한다. `src/tokenizer.rs::TRANSITIONS`가 SS..SDDD 16개 state와 CX/CDD/CDDZ/CU/CS/CA/CN/CB/C9/CD/CC/CQ transition을 명시적으로 보존한다.
- [x] 기존 handwritten `scanner::transition`을 제거하고 lookup-only `TRANSITIONS[state][class]`로 교체했다. follow-on numeric rewind와 UNDD 처리는 `jtwordil`의 별도 boundary action으로 유지한다.
- [x] numeric follow-on, quoted literal, `NB.`, `NB..`/`NB.:`, `{{`/`}}`, inflection word boundary를 differential corpus로 만든다. `tools/word_conformance.py`가 state-prefix × 256-byte sweep, 특수 사례, seeded random 2,000건을 포함한다.
- [x] unmatched quote를 jsource `EVOPENQ`에 대응하는 `open quote`로 분류하고 시작 quote byte span을 보존한다.
- [x] word/parser/runtime source span은 byte offset으로 보존하고 사용자 진단 시 Unicode line/column으로 변환한다.
- [x] `;:` 기반 word-formation oracle과 RustJ raw `wordil`-equivalent spans를 byte 단위로 비교하는 adapter/probe를 만든다 (`tools/word_conformance.py`, `examples/scan_words.rs`). trailing `NB.` raw field와 parser-visible count는 별도 contract로 유지한다.
- [x] pinned jsource `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528` against 6,618-case differential에서 **failed=0 / zero_mismatch=true**를 확인했다 (seed `20260927`, open-quote cases `1109`).

**F0 완료 조건:** supported source domain에서 word boundaries/comment cutoff/error가 pinned jsource `jtwordil`과 일치한다.

<a id="f1--jsource-enqueue--primitiveresolver-이식"></a>
#### F1 — jsource enqueue + PrimitiveResolver 이식

- [x] `syntax::lex_spanned`가 수행하던 word interpretation을 `enqueuer::enqueue`로 이동하고 `syntax`는 legacy Token adapter로 축소했다.
- [x] `EnqueuedWord { class, payload, span, word_index, flags }`와 `EnqueueClass`/`EnqueueFlags`를 정의해 parser-facing class/payload/provenance를 명시적으로 분리했다.
- [ ] enqueue의 classification order와 parser class/POS 결정 순서를 `jtenqueue`와 동일하게 유지한다. RustJ convenience lexer가 먼저 품사를 확정하지 않게 한다.
- [ ] core J primitive lookup을 jsource `spellin -> ds`와 같은 위치와 precedence로 구현한다.
- [x] `PrimitiveResolver`/`PrimitiveContext`를 만들고 enqueue용 core primitive resolution과 parser/name-binding용 extension lookup을 분리했다.
- [x] `PrimitiveHandle { semantic_id, source_origin, result_pos, semantic_info, lowering_key }`를 정의했다.
- [ ] compile 시작 시 `PrimitiveContext`와 `TargetContext`를 함께 확정하되 enqueuer는 semantic primitive resolution에 `PrimitiveContext`만 사용한다.
- [ ] built-in과 extension 모두 동일 `lowering_key -> active TargetContext` lookup protocol을 사용하게 한다.
- [x] extension-like spelling은 enabled extension catalog에 있어도 enqueue에서는 ordinary NAME + lookup metadata로 진행한다.
- [ ] numeric/string construction, name validation, assignment/copula classification을 jsource `jtenqueue` 순서대로 이식한다.
- [x] ordinary NAME은 처음 non-lookup으로 두고, trailing NAME/뒤에 non-assignment가 오는 NAME만 lookup으로 전환하며 copula 직전 assignment target NAME은 non-lookup으로 유지한다.
- [x] `EnqueueFlags`에 `global_assignment/local_assignment/assignment_to_name`을 분리했다. `=:`는 global이며 NAME 직후 copula는 to-name flag를 보존한다. `=.`는 TopLevel에서 global로 승격하고 ExplicitDefinition enqueue 환경에서는 local을 유지한다. explicit body의 local 실행과 locative 승격은 미완료다.
- [x] **F1 env=0/1/2 작은 수렴(2026-10-07):** 고정 [jsource `w.c::jtenqueue`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/w.c)의 `env=0` tacit translator는 copula의 `ASGNTONAME` 특수화를 하지 않고 `=.`를 일반 local로 둔다. RustJ에 `EnqueueEnvironment::TacitTranslator`를 추가하고 env=0/1/2에서 `=.`·`=:`의 global/local/assignment-to-name 및 NAME lookup·source span을 각각 검증하는 Rust 회귀를 추가했다. **이것은 세 환경의 enqueue flag 부분 구현만 수용**하며 locative에 의한 global 승격, tacit translator 전체 사용 경로, C runtime queue의 직접 차분 및 F1 전체 완료를 뜻하지 않는다.
- [x] one-word sentence는 Noun/Name/Verb/Adverb/Conjunction만 결과 가능 class로 허용하고 copula/괄호 단독 문장을 enqueue 단계에서 거부한다.
- [ ] jsource sentence-word refcount/inplacing flags와 special in-place sentence rewrites는 optimization-only로 명시적으로 제외한다.
- [x] parser-time NAME lookup이 extension binding의 Verb/Adverb/Conjunction POS를 얻은 뒤 core와 같은 modifier/parser class 경로에 참여하는 테스트를 만들었다.

- [ ] **F1↔P4 locative 정상 수용(2026-10-07 검토):** 유효한 J direct `name_locale_`, indirect `name__holder`, 호환 `name__`(`name_base_`)는 **문법 오류가 아니며 영구 `Unsupported` 처리 대상도 아니다**. `sn.c::vnm/nfs`로 이름을 검증하고 `w.c::jtenqueue`의 NMLOC/NMILOC·`=.`의 global promotion을 보존한다. 단순 이름의 문자열 key로 속여서 다른 locale과 충돌시키지 않는다. F1의 syntax/name queue와 P4의 locale lookup/assignment는 **연계 구현 후 수용**한다. 현재 RustJ의 유효 locative에 대한 `Unsupported`는 명백한 **미구현 상태**이지 승인된 최종 동작이 아니다.

- [ ] **NP-01/02 (§3.7.3):** NAME occurrence·source span과 noun stack-read·function POS/NameRef·constructor event를 J 시점에 연결한다(NP-V01/02).

**F1 완료 조건:** parser가 raw spelling을 다시 해석하지 않고 `EnqueuedWord` queue만으로 core/extension primitive, name lookup, assignment semantics를 결정할 수 있으며 hardware implementation 선택은 아직 일어나지 않는다.

<a id="f2--jsource-parse-queue-skeleton"></a>
#### F2 — jsource parse queue skeleton

- [x] semantic parser 입력을 legacy `Token`에서 `EnqueuedWord` queue로 바꾸고 span/word-index/flags를 parser 진입까지 보존한다.
- [x] `EnqueueClass`와 `EnqueuedPayload`를 분리하고 parser가 동일 `EnqueuedWord` carrier에서 둘을 함께 운반한다.
- [ ] jsource Mark/Edge sentinel을 명시적으로 표현한다.
- [x] ordinary lookup NAME은 `EnqueueFlags.lookup_name`을 확인한 뒤 parser item 생성 전에 `ParserNameBinding`으로 resolve한다.
- [x] pinned `cases[]`를 옮긴 `match_parse_row([ParseClass; 4])`가 semantic payload를 보지 않고 parser class만으로 eligibility와 first-match precedence를 결정한다.
- [x] `cases[]` 기반 row ordering/reduction extent, 동일 stack reinsertion/rescan과 legacy flat train/modifier heuristic 제거를 production parser에 적용했다. **남은 것은 C runtime `ptcol`의 reachable-state/internal trace 동등성 검증**이며 P6에서 별도로 추적한다.
- [x] 구현된 reduction 결과를 동일 Item stack에 재삽입한다. 원본 word provenance와 occurrence를 reduction pipeline에서 계승하며 전체 runtime ptcol trace 동등성은 P6에서 별도 미완료다.
- [x] `ParseClass`를 F2 row matcher와 application/modifier reduction의 공통 class domain으로 사용한다.

**F2 완료 조건:** parser는 jsource-compatible enqueue queue를 유일한 입력으로 받아 9-row engine으로 넘길 수 있다.

<a id="p0--기준선과-differential-oracle-고정"></a>
#### P0 — 기준선과 differential oracle 고정

- [x] jsource 9-row parsing rule(row 0–8)의 eligibility와 precedence를 `cases[]`와 runtime dispatch 양쪽에서 확인한다.
- [x] ordinary non-assignment NAME은 stack class matching 전에 lookup되며, noun은 value로, 일반 verb/adverb/conjunction은 현재 POS를 가진 nameref/value semantics로 들어감을 확인한다.
- [x] rows 3–4의 modifier action 결과 `yy`의 실제 `AT(yy)`가 다음 parser class가 됨을 확인한다.
- [x] modifier application 결과가 하나의 completed J entity로 stack에 재삽입된 뒤 후속 reduction에 참여함을 확인한다.
- [x] `+/ % #`에서 `+/`가 하나의 derived VERB entity로 만들어진 뒤 Fork의 `f` operand가 됨을 확인한다.
- [x] 과거 staged modifier/train helper의 의미 한계를 확인했다. 현재 matcher는 `match_parse_row`로 통합되었으며, runtime semantic action 및 전체 POS coverage는 P2–P4에서 계속 추적한다.
- [x] parser와 compiler-analysis 책임 경계를 고정한다.
- [x] differential oracle의 최소 contract를 정한다:
  - 성공/실패 및 J error class,
  - deterministic noun result,
  - assignment 후 name class(`4!:0`),
  - constructed function/modifier의 atomic/linear representation(`5!:1`, `5!:5`)이 유용한 경우,
  - RustJ 내부에서는 row id/input classes/span/result class를 기록하는 optional ParseTrace.
- [x] 위 contract를 실제 test harness API로 만든다. `tools/oracle.py` JSON-lines protocol이 `eval`, `sentence`, `name_class`, `representation(atomic|linear)`을 제공하며 기존 string eval 요청과 호환된다.

**P0 완료 조건:** **완료.** observable contract를 사용하는 oracle API가 존재하고, source review는 jsource revision `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`을 고정한다. 로컬 oracle DLL은 별도 revision/hash로 식별하며 source pin으로 빌드되었다고 가정하지 않는다. GitHub CI는 생략한다.

<a id="p1--parser-stack-model과-semantic-value-model-분리"></a>
#### P1 — parser stack model과 semantic value model 분리

- [x] parser class/control(`EnqueueClass`/`ParseClass`)과 semantic payload/entity(`EnqueuedPayload`/`ParseValue`/`FunctionEntity`)를 별도 타입으로 분리했다.
- [x] `ParseClass`가 `Noun | Verb | Adverb | Conjunction | Name | Assignment | LParen | RParen | Mark`를 명시적으로 표현한다.
- [x] ordinary lookup NAME은 stack item 생성 전에 `ParserNameBinding`으로 현재 noun/function POS를 resolve하며, unresolved ordinary name만 jsource식 late verb nameref로 남긴다.
- [x] Noun과 `FunctionEntity`는 별도 representation을 유지하면서 `Item/ParseClass`에서 동일한 parser class interface로 참여한다.
- [x] source primitive, named modifier, completed derived entity가 모두 `FunctionEntity.result_pos -> ParseClass` 경로로 parser에 참여한다.
- [x] `FunctionEntity.result_pos`를 completed function entity의 parser POS 근거로 사용한다.
- [x] enqueue lookup/to-name flags와 parser assignment path로 assignment-target NAME을 ordinary semantic `NameRef`와 분리했다.
- [x] `EnqueuedWord`의 byte span/word index를 parser 진입까지 보존하고 completed Expr/FunctionEntity reduction span 및 진단 blame provenance로 전달한다.

**P1 완료 조건:** parser control state와 J semantic entity가 분리되어 있으며 ordinary names, assignment names, nouns, verbs/modifiers를 jsource class rules대로 stack에 올릴 수 있다.

