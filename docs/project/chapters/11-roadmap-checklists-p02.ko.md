[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)

<a id="je2-구현--namespace와-runtime-result의-jentity-수렴-2026-10-04-partial-je2"></a>
##### JE2 구현 — namespace와 runtime result의 JEntity 수렴 (2026-10-04, partial JE2)

- runtime `SymbolValue`를 제거하고 `Binding { value: JEntity, version: NameVersion }`으로 저장한다. 함수 binding에는 `Arc<FunctionEntity>`만 남으며 별도 Verb wrapper/span/target 복제가 없다. runtime final result도 동일 JEntity를 사용한다. `ExprKind`는 noun application/dependency 구조를 보존하는 parser representation으로 남는다.
- `commit_binding(name, JEntity) -> Result<JEntity>`가 version 증가 가능성을 먼저 검사하고, noun을 한 번 freeze한 뒤 namespace 저장/대입 반환 payload를 공유한다. 함수는 동일 DAG를 share한다. host assign은 이 함수에 위임하며 기존 noun replacement·OutputPool retirement 경로를 유지한다. automatic JEntity Clone을 추가하지 않고 freeze된 Value/Function Arc만 명시적으로 공유한다.
- noun lookup은 기존 값 snapshot을 내고 function lookup은 DAG의 result_pos를 전달한다. by-value nameless modifier, POS-known static alias, modifier resolution의 late NameRef chain·versions를 유지한다. 과거 enum variant가 암묵적으로 검사하던 Verb/Modifier class는 각 조회 경로의 명시적인 POS 검사로 대체했다. verb NameRef가 modifier로 바뀌면 같은 domain error/current_name을 보존한다.
- 더 이상 namespace에 Verb adapter가 필요하지 않아 `Verb::from_entity`와 그 전용 unit test를 제거했다. 보장하던 identity/span·POS 거부는 실제 commit/lookup 경로의 tests로 이행했다. parser occurrence wrapper와 `VerbTarget`은 아직 유지하며 namespace의 intrinsic Function identity와 분리한다. storage representation·Value 내부·gerund/view·optimizer/target policy는 바꾸지 않았다.

새 runtime unit tests 3개는 65,536-atom noun의 stored/returned pointer와 네 RHS class의 shared identity·versions, version overflow의 noun/function replacement 거부·기존 binding/pool/commit 보존, unified Function의 expected-POS/domain/current_name을 검증한다. integration regression은 cache limit 0/4096에서 noun→Verb→Adverb→Conjunction 교체 중 noun alias 생존과 마지막 alias 해제 뒤 bounded retirement·noun 재대입을 검증한다. 기존 adapter test 한 개를 제거하고 이 실제 경로 검사로 대체했다. C corpus/stage에는 noun/function replacement **11건**을 추가하여 entity boundary fixture는 **82건**이다.

**Namespace seam gate:** Windows default/portable 각각 **369 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture **4,821 cases / 4,817 passed / 기존 runtime 경계 4 / failed 0**, stage **10,001 checks**, words **6,618 cases**. capture graph 경계 108과 static 경계 2는 별도이며 full upstream suite·definition invocation acceptance·private C runtime trace 동등성은 미검증이다. report 10개의 실제 binary/source hash를 확인한다. conformance source pin은 `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`, DLL release는 `ded7793fe5795d79eda8e7138dce94aa056edf78`; JE0 source 감사 revision `0db94e768a845e2583c01d00538c3d16379677bb`는 새 DLL 검증으로 취급하지 않는다.

**남은 JE2:** parser completed-result transport와 deferred application 구조의 경계, 전체 local/locale/definition scope, explicit body invocation 및 static/runtime/capture 경로의 더 넓은 수렴을 검증한다. 이번 namespace seam 완료를 JE2 전체 완료로 표시하지 않는다. FunctionOperand view는 아래 단계에서 완료했으며 semantic nested DD와 기존 M2 gaps도 계속 추적한다. JE3+ collection·broader storage migration·optimization·CUDA·GitHub CI는 보류한다.

<a id="je2-구현--provenance를-보존하는-borrowed-operand-view-2026-10-04-partial-je2"></a>
##### JE2 구현 — provenance를 보존하는 borrowed operand view (2026-10-04, partial JE2)

`FunctionOperand::as_entity_ref()`는 noun과 모든 함수 POS를 공통 `JEntityRef`로 조회한다. `span()`은 noun operand의 저장된 source span 또는 함수 identity의 span을 빌려준다. 이후 application occurrence span과 합치지 않는다. 조회에서 Value 복사·Arc 증가·새 entity 할당이 없고, owning enum은 noun provenance와 함수 DAG 소유권을 보존하기 위해 유지한다. gerund collection이나 물리 배열 representation으로 확대하지 않는다.

실제 사용 경계는 nameless modifier의 by-value lookup 판정과 semantic binding의 function NameRef DAG 순회다. 기존 순회 순서·POS/lookup 정책은 유지한다. operand를 실행용 parser item으로 만드는 경계는 공유 Arc 소유권이 필요하므로 기존 materialization 경로를 유지한다.

회귀는 owned 65,536-atom noun의 pointer/span 보존을 추가하고, 기존 host 종료 후 noun snapshot·48단계 shared DAG·explicit Verb/Adverb/Conjunction tests를 공통 view로 확장하여 payload identity, 함수/DefinitionCode 참조 수 및 원본 위치를 확인한다. 새 J 문법은 추가하지 않아 기존 82건 entity-boundary C fixtures를 그대로 사용한다.

**Operand seam gate:** Windows default/portable 각각 **370 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture **4,821 cases / 4,817 passed / 기존 runtime 경계 4 / failed 0**, stage **10,001 checks**, words **6,618 cases / failed 0**. capture graph 경계 108과 static 경계 2는 별도다. report 10개의 실제 binary/source hash를 확인했다. source/DLL pin은 위 namespace gate와 같으며 full upstream suite·definition invocation acceptance·private C runtime trace 동등성은 미검증이다.

completed parser result와 deferred noun/application 구조를 구분한 공통 transport는 아래 단계에서 진행한다. 전체 local/locale/definition scope와 explicit body invocation은 미완료다. JE2 전체 완료로 표시하지 않는다.

<a id="je2-구현--completed-parser-result의-공통-transport-2026-10-04-partial-je2"></a>
##### JE2 구현 — completed parser result의 공통 transport (2026-10-04, partial JE2)

`CompletedParseResult { entity: JEntity, span, height, verb_adapter }`는 완료된 RHS의 이동 경계다. noun Value와 함수 Arc를 복사하지 않고 이동하며, source occurrence/height와 Verb의 span/target adapter는 immutable FunctionEntity identity 밖에 둔다. `from_item`은 literal 또는 grouped literal noun만 받아들이고, 미계산 call·ReadName은 기존 unsupported 경계를 유지한다. 이 helper는 계산하거나 이름을 재조회하지 않는다. JEntity에 Clone을 추가하지 않는다.

runtime rows 0–2는 기존 host.apply를 한 번 실행한 뒤 이 경계로 completed noun을 stack에 반환한다. host가 없는 analysis 경로는 기존 Expr 연산 구조를 보존한다. row 7은 같은 경계로 RHS를 host.assign에 전달하고 같은 반환 payload로 parser item을 복원한다. commit capture·provenance inheritance·POS·lookup timing·effect order는 기존 reduction pipeline이 처리한다. ParseValue의 Expr/Verb/Function/NAME/control variants, constructor 경로, final Program 구조 전체를 바꾸지는 않는다.

근거는 [jsource runtime p.c row 7](https://github.com/jsoftware/jsource/blob/0db94e768a845e2583c01d00538c3d16379677bb/jsrc/p.c#L1006)의 stacked RHS 대입/반환이다. 이는 pv.c tacit translator와 구분하며 새 source pin으로 DLL을 빌드했다는 주장이 아니다.

새 unit regression 3개는 grouped owned 65,536-atom noun의 pointer/occurrence span/height, 세 함수 POS의 shared identity와 NameRef·Verb occurrence adapter, 정적 call/name 보존과 chained assignment의 apply 1회→inner→outer commit 2회를 확인한다. 기존 entity-boundary fixtures에 computed scalar chained assignment와 grouped computed array assignment 및 두 이름의 결과 조회 **6건**을 추가하여 **88건**으로 늘렸다. 이 실행 횟수 검사는 Rust host 경계의 관찰이며 private C trace 동등성 주장이 아니다.

**Completed-result gate:** Windows default/portable 각각 **373 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture **4,827 cases / 4,823 passed / 기존 runtime 경계 4 / failed 0**, stage **10,007 checks**, words **6,618 cases / failed 0**. capture graph 경계 108과 static 경계 2는 별도다. report 10개의 실제 binary/source hash를 확인했다. source/DLL pin은 위 namespace gate와 같다. full upstream suite·definition invocation acceptance·private C runtime trace 동등성은 미검증이다.

다음은 남은 parser value/constructor 경계에서 concrete completed result와 분석용 expression이 중복 전달되는 지점을 검토한다. 전체 stack enum 수렴, local/locale/definition scope 및 explicit body invocation은 미완료다. optimizer·CUDA·GitHub CI는 계속 보류한다.

<a id="je2-구현--modifier-constructor-operand의-공통-경계-2026-10-04-partial-je2"></a>
##### JE2 구현 — modifier constructor operand의 공통 경계 (2026-10-04, partial JE2)

`CompletedParseResult::into_operand()`는 완료 noun을 기존처럼 한 번 `into_shared`하고 원본 occurrence span과 함께 FunctionOperand에 이동한다. 함수는 동일 Arc를 이동하며 occurrence용 Verb adapter를 semantic child에 복제하지 않는다. `modifier_train`의 Noun/Verb/Adverb/Conjunction 변환 중복을 이 경계로 대체했다. production rows 5/6과 gerund AR의 동일 modifier-train 생성 경로에 적용되며, result POS는 기존 cf.c disposition/constructor 결정으로 유지한다. raw NAME/control은 syntax error, deferred call/ReadName은 기존 unsupported로 남긴다.

conjunction rank/right-first audit, noun-left fork, definition constructor, immediate bident/trident application은 각각의 검증 순서·실행 의미가 있어 이번 변경에 합치지 않았다. generic entity collection이나 physical storage 변경도 없다.

새 회귀 2개는 grouped owned 65,536-atom noun이 복사 없이 freeze되고 원본 span을 유지하며 train 해제 뒤 application occurrence에서 재사용되는지 확인한다. 함수 세 POS의 DAG identity/참조 수, cf.c disposition과 result POS, 미계산 noun의 거부 및 control의 syntax error도 확인한다. 기존 runtime noun-origin capture·named array snapshot·nested modifier train tests와 88건 entity-boundary C fixtures를 재검증한다.

**Constructor-operand gate:** Windows default/portable 각각 **375 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture **4,827 cases / 4,823 passed / 기존 runtime 경계 4 / failed 0**, stage **10,007 checks**, words **6,618 cases / failed 0**. capture graph 경계 108과 static 경계 2는 별도다. report 10개의 실제 binary/source hash를 확인했다. source/DLL pin은 위 namespace gate와 같다. full upstream suite·definition invocation acceptance·private C runtime trace 동등성은 미검증이다.

다음은 rank/conjunction operand 경계에서 right-before-left 오류 우선순위와 gerund 감사 계약을 먼저 고정한 뒤 공통 transport 사용 여부를 검토한다. JE2 전체 완료와 전체 J parser 지원으로 표시하지 않는다.

<a id="je2-구현--rankconjunction-operand와-오류-우선순위-2026-10-04-partial-je2"></a>
##### JE2 구현 — rank/conjunction operand와 오류 우선순위 (2026-10-04, partial JE2)

`apply_conjunction_at`의 좌우 Noun/Verb operand 전달을 `CompletedParseResult::from_item/into_operand`로 연결했다. 먼저 right operand의 형태를 검사하며 @:의 noun-right domain error는 미계산 noun의 unsupported보다 앞선다. rank의 noun-right는 rank→length→numeric domain 순서로 검사한 뒤에만 left operand를 읽거나 gerund를 감사한다. 원본 Expr span과 parser reinsertion span override의 기존 구분을 유지하고, noun freeze와 함수 DAG 이동은 공통 경계를 사용한다.

근거는 [고정 jsource cr.c::jtqq](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c#L733)의 right rank 추출과 그 이후 noun-left 분기다. boxed rank-1 noun만 gerund 감사 대상이며 모든 요청 rank가 RMAX이면 감사를 생략한다. fx 감사의 J 오류는 quiet constant-noun fallback으로 처리하고 partial decoded list를 남기지 않는다. RustJ 구현 경계인 unsupported는 계속 전달하며 성공한 C 기능인 것처럼 fallback하지 않는다. verb-right는 원래 function operand로 유지하고 noun-left function identity로 뒤바꾸지 않는다.

새 unit 2개는 owned 65,536-atom constant와 rank noun의 pointer/Expr span, deferred left보다 오른쪽 rank/length/domain 오류가 먼저 나오는지 및 @: noun-right domain 우선순위를 확인한다. capture 회귀 1개는 right 오류 시 gerund name lookup/commit 부재와 기존 binding version 보존, valid rank/verb-right의 quiet 감사, 무한 rank 감사 생략 및 partial decode 제거를 확인한다. compound gerund C corpus/stage에 setup·오류 뒤 binding 사용·quiet/RMAX/verb-right construction **12건**을 추가한다. entity-boundary fixtures는 별도 88건으로 유지한다.

**Rank-operand gate:** Windows default/portable 각각 **378 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture **4,839 cases / 4,835 passed / 기존 runtime 경계 4 / failed 0**, stage **10,019 checks**, words **6,618 cases / failed 0**. capture graph 경계는 새 사례 2건을 포함한 **110건**, static 경계는 2건이며 runtime pass와 별도로 기록한다. report 10개의 실제 binary/source hash를 확인했다. conformance source/DLL pin은 JE2 namespace gate와 같고 full upstream suite·definition invocation acceptance·private C trace 동등성은 미검증이다.

다음 JE2 후보는 noun-left fork와 definition constructor의 완료 값 전달 경계다. 각각 source span·생성 오류·본문을 실행하지 않는 계약을 먼저 확인한다. 전체 stack variant 수렴, scope와 definition invocation은 여전히 미완료다.

<a id="je2-구현--noun-left-fork와-definition-생성-경계-2026-10-04-partial-je2"></a>
##### JE2 구현 — noun-left fork와 definition 생성 경계 (2026-10-04, partial JE2)

`CompletedParseResult::from_noun`은 grouped literal을 기존 completed_noun 규칙으로 이동하며 Expr span/height를 보존한다. 일반 from_item은 이 경계를 공유하되 기존 Item occurrence override를 복원한다. noun-left fork는 이 결과를 into_operand로 옮겨 source span·g/h DAG identity를 유지한다. 이전 fork만의 owned noun 경로를 공통 one-time freeze 정책으로 바꾸어 큰 상수의 이후 operand 재사용이 전체 배열 복사를 만들지 않도록 했다. allocator/physical representation 자체는 바꾸지 않았다.

지원 DefinitionConstructor는 양쪽 noun class guard를 먼저 수행한 뒤 mode와 body를 기존 순서로 완료 값에서 추출한다. 이 transient 입력은 freeze하거나 FunctionEntity operand로 저장하지 않는다. mode/body 원본 일치와 semantic code validation은 그대로 수행한다. 생성 결과는 실제 Verb/Adverb/Conjunction POS를 가진 FunctionEntity를 공통 function→into_item 경계로 반환한다. DefinitionCode Arc의 불필요한 clone도 제거했다. 원본 code와 source provenance를 보존하며 invocation/local frame을 생성하거나 본문을 실행하지 않는다. computed definition의 미지원 범위는 확대하지 않았다.

새 unit 2개는 grouped owned 65,536-atom fork 상수의 pointer/source span·g/h identity, 두 번 재사용 후 fork 해제까지 공유 생존, 미계산 noun 거부를 검증한다. definition의 양쪽 class guard가 deferred input보다 먼저 적용되는지와 기존 domain/unsupported 우선순위도 검증한다. integration 1개는 explicit/direct 각 세 함수 POS의 생성·commit, 본문 counter의 version/value 불변과 본문 이름 input observation 부재를 확인한다. C definition corpus/stage에 같은 6개 생성과 counter 조회 **12건**을 추가하며 entity-boundary fixture는 별도 88건이다.

**Fork/definition gate:** Windows default/portable 각각 **381 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture **4,851 cases / 4,847 passed / 기존 runtime 경계 4 / failed 0**, stage **10,031 checks**, words **6,618 cases / failed 0**. capture graph 경계 **114건**과 static 경계 2건은 runtime pass와 별도로 기록한다. report 10개의 실제 binary/source hash를 확인했다. conformance source/DLL pin은 JE2 namespace gate와 같고 full upstream suite·definition invocation acceptance·private C trace 동등성은 미검증이다.

다음은 남은 completed-result/function wrapper 경계를 다시 감사하여 필요 없는 adapter만 제거하고, frontend F/P 체크리스트의 지원 범위와 미지원 생성/실행 범위를 정리한다. 전체 stack variant 수렴·local/locale scope·definition invocation은 여전히 미완료다. CUDA·optimizer 구현·GitHub CI는 계속 보류한다.

<a id="je2-구현--잔여-adapter-감사와-frontend-체크리스트-수렴-2026-10-04-partial-je2"></a>
##### JE2 구현 — 잔여 adapter 감사와 frontend 체크리스트 수렴 (2026-10-04, partial JE2)

`ParseValue::function_entity()`로 Verb/Adverb/Conjunction의 완료 함수 Arc를 빌려 읽는다. construction success와 final function result capture의 중복 분기를 제거했으며, observation event가 함수 수명을 실제로 보유해야 할 때만 기존처럼 Arc를 clone한다. completed Verb의 이동도 기존 function factory로 통합하되 Item occurrence span과 Verb adapter span을 각각 유지한다. 새 J 문법이나 실행 지원을 추가하지 않았다.

| 유지하는 구조 | 유지 이유 |
|---|---|
| Verb/VerbTarget | 현재 parser occurrence span과 runtime target adapter를 intrinsic FunctionEntity identity와 구분한다 |
| ParseValue/Item | deferred Expr, lexical NAME/target/control, class/flags/word provenance/occurrence는 concrete JEntity와 역할이 다르다 |
| ParserNameBinding | noun snapshot·abstract noun·function POS·known modifier/version은 lookup observation 계약이다 |
| FunctionOperand | noun source span과 함수 DAG의 owning Arc를 유지하며 borrowed JEntity view로 읽는다 |
| ExprKind | static computation/dependency와 completed value/function을 최종 Program에서 구분한다 |
| CompletedParseResult | concrete JEntity 이동에 필요한 height/span/Verb occurrence adapter만 둔다 |

AssignedValue/SymbolValue는 앞 단계에서 제거했다. enum 수를 줄이려고 위 차이를 지우지 않는다. 이 감사로 JE2 전체 stack/scope/invocation 완료를 주장하지 않는다.

F2의 same-stack 재삽입, P2의 같은 matcher 재순회·runtime/analysis engine 공유, P4의 ordinary extension NAME·assignment target 분리, P7의 legacy flat application loop 제거·entry-point 공유를 구현된 범위에 맞게 체크했다. P2 rows 3/4/7은 부분 지원을 명시하고 전체 완료 체크는 유지하지 않는다. full runtime ptcol trace, 전체 modifier/immediate bident/trident semantics, scope와 invocation, intrinsic FunctionSemanticInfo 및 최종 cutover gate는 미완료다. English mirror는 아래 단계 요약으로 같은 상태를 전달한다.

기존 explicit/direct 세 POS 회귀를 확장하여 ConstructionSuccess→FunctionResult→Commit에서 같은 FunctionEntity/DefinitionCode Arc를 유지하고 final result observation이 final commit 앞에 위치하는지 확인한다. C corpus는 기존 4,851건을 그대로 재검증한다.

**Adapter-audit gate:** Windows default/portable 각각 **381 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture **4,851 cases / 4,847 passed / 기존 runtime 경계 4 / failed 0**, stage **10,031 checks**, words **6,618 cases / failed 0**. capture graph 경계 114건과 static 경계 2건은 별도다. report 10개의 실제 binary/source hash를 확인했다. conformance source/DLL pin은 JE2 namespace gate와 같고 full upstream suite·definition invocation acceptance·private C trace 동등성은 미검증이다.

후속 감사에서 확인한 immediate action의 기존 지원과 surface parser row 도달성은 아래 기록을 따른다. 이 단계의 “미지원 immediate action” 우선순위는 기존 구현을 충분히 구분하지 못한 계획이었다.

<a id="je2p3-구현--immediate-constructor-결과-경계와-row-도달성-정정-2026-10-04-partial"></a>
##### JE2/P3 구현 — immediate constructor 결과 경계와 row 도달성 정정 (2026-10-04, partial)

C `cf.c::jthook`의 `fn == 0`은 invisible modifier 실행으로 생긴 V N / N/V A 및 N V N / N/V C N/V를 즉시 적용한다. RustJ의 `construct_modifier_bident`/`construct_modifier_trident`는 이미 AR decode와 derived modifier 실행에서 이를 지원한다. V N·N V N은 runtime host를 한 번 호출하여 실제 Noun을 반환하고, adverb/conjunction action은 해당 constructor가 반환한 실제 POS를 유지한다. “immediate executor가 전부 미구현”이라는 P3 설명을 정정한다. 지원 primitive/definition 범위를 넘어서는 실행은 여전히 Unsupported이며 전체 P3 완료를 뜻하지 않는다.

surface parser에서는 ordered rows 0/2/3/4가 이 즉시 적용 조합을 rows 5/6보다 먼저 소비한다. 현재 9개 ParseClass의 모든 6,561 stack window를 검사하여 row 5는 NVV/VVV fork만, row 6은 immediate/fork가 아닌 disposition만 선택함을 고정했다. 도달하지 않는 branch의 Unsupported 문구를 row invariant 오류로 바꿨다. static analysis가 실제 값이 필요한 호출을 임의로 실행하거나 Unsupported를 J 오류로 바꾸는 정책 변경이 아니다.

`ConstructionNames::apply_noun`의 성공 결과를 `CompletedParseResult::noun(...).into_item()`으로 통합한다. 기존 once-freeze, span/height와 ConstructorApply의 성공/오류 observation 순서는 유지한다. 새 unit regression은 bident/trident 두 경로에서 owned 256×256 배열 결과의 payload pointer·shape·span·실제 Noun POS와 host 호출 1회를 확인하고, 후속 FunctionOperand로 이동해도 복사 없이 살아 있음을 검증한다. 기존 실패/효과/정적 no-host 회귀를 함께 재검증한다.

- [x] immediate action과 surface row eligibility를 구분하고 전수 검사한다.
- [x] host Noun 결과를 공통 completed-result carrier로 이동한다.
- [x] 즉시 계산한 Noun을 rank constructor에 넣는 C corpus/stage 사례 12개를 추가한다. scalar/array 결과 및 domain/length 실패를 비교한다.
- [ ] 모든 primitive/explicit modifier 실행과 local/locale/definition invocation을 구현한다. 기존 ignored definition acceptance 17개를 완료 증거로 세지 않는다.

**Immediate-boundary gate:** Windows default/portable 각각 **383 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture **4,863 cases / 4,859 passed / 기존 runtime 경계 4 / failed 0**, stage **10,043 checks**, words **6,618 cases / failed 0**. capture graph 경계 114건과 static 경계 2건은 별도다. report 10개의 실제 binary/source hash를 확인했다. source/DLL pin은 JE2 namespace gate와 동일하다. full upstream suite·definition invocation acceptance·private C trace 동등성은 미검증이다.

다음 단계는 실제 미지원 explicit adverb/conjunction 적용을 최소 C 사례로 분리하고, operand/local name binding과 실제 반환 POS를 보존하는 invocation 경계를 구현하는 것이다. noun 본문 결과도 Function으로 강제하지 않는다. CUDA·optimizer 구현·GitHub CI는 계속 보류한다.

기준 소스: [cf.c bident/trident table](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L292), [cf.c invisible modifier의 즉시 적용](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L355), [p.c ordered parser rows](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c).

<a id="je2p3-구현--nonoperator-explicit-modifier의-첫-invocation-경계-2026-10-04-partial"></a>
##### JE2/P3 구현 — nonoperator explicit modifier의 첫 invocation 경계 (2026-10-04, partial)

rows 3/4와 AR/derived modifier 실행이 `ExplicitDefinition`을 적용할 때 runtime host의 `apply_definition`을 거쳐 실제 `JEntity` 결과를 받는다. `x/y`를 참조하지 않는 mode 1/2에서 선택된 valence의 **단일 Body 문장**을 공유 tokenizer/enqueuer/parser로 실행한다. literal 결과뿐 아니라 `u/`, `m+n`, global noun을 읽는 계산도 기존 semantic kernel 범위에서 실행한다. direct definition의 mode 1/2도 같은 DefinitionCode 경계를 사용한다. definition을 생성·대입하는 단계는 계속 본문을 실행하지 않는다.

`ModifierFrame`은 parent Engine을 빌려 global lookup/semantic calls를 수행하며 전체 symbol table을 복제하거나 global에 operand를 잠시 대입하지 않는다. `u/v`는 실제 operand이고 noun일 때만 `m/n` alias를 정의한다. `RuntimeParserHost::operand_function`은 이 특별 이름의 concrete function substitution만 허용한다. ordinary 함수 NAME의 late lookup·alias 재정의 동작은 유지한다. C `p.c`의 mnuvxy by-value 규칙과 `cx.c`의 operand 설치를 기준으로 했다. gerund의 특별 이름 `u`도 C decoded structure와 대조했다. 일반 gerund 이름을 snapshot으로 바꾸지 않는다.

실제 반환값은 공통 CompletedParseResult를 통해 **Noun/Verb/Adverb/Conjunction의 품사 그대로** 다음 stack reduction에 들어간다. noun 반환을 함수로 강제하거나 capture에서 “completed construction function”으로 가정하던 경로를 제거한다. immutable 함수 Arc 및 noun shared payload를 이동·공유한다. 정적 prepare는 runtime host가 필요한 호출을 실행하지 않고 Unsupported를 유지한다. explicit 정의를 static-known primitive modifier로 분류하지 않는다.

capture에는 `ExplicitModifierApply` invocation marker와 noun 반환의 `ConstructionNounSuccess` occurrence/facts를 추가했다. matching construction attempt·row/POS·sequential occurrence와 실패 후 대입 보존을 검증한다. 본문 내부 dependency/effect graph를 outer graph에 아직 연결하지 않았으므로 J Graph 변환은 `explicit modifier body graph requires invocation scope`라는 경계를 반환한다. **실행 성공은 graph 분석 완료를 뜻하지 않는다.** 본문 source 좌표를 caller source 좌표로 잘못 렌더링하지 않도록 operation/argument context는 유지하고 오류 위치는 outer invocation으로 매핑한다. 별도의 body/caller diagnostic frame은 후속 작업이다.

지원 경계:

- [x] 단일 문장 nonoperator adverb/conjunction의 실제 Noun/Function 반환과 후속 reduction을 지원한다.
- [x] `u/v`, noun 전용 `m/n`, global late lookup, 반환 함수의 frame 밖 재사용, 오류/재정의/대입 대상 보존을 검증한다.
- [x] Rust 회귀 3개와 C corpus/stage 사례 **48개**를 추가한다. scalar/matrix/empty/boxed noun, 반환 ADV/CONJ 재적용, gerund operand 및 domain/length 실패를 포함한다.
- [x] 본문의 simple NAME `=.`/`=:`와 여러 직선 문장은 다음 modifier-scope 단계에서 구현했다. control flow·nested definition scope는 계속 미지원이다.
- [x] 이후 operator-call 단계에서 x/y operator의 deferred Verb construction과 직선 호출을 구현했다. unbound 특별 이름의 global fallback 및 전체 scope 지원은 포함하지 않는다.
- [ ] 현재 recursive parser 기반 호출은 Windows stack 보호를 위해 **8중첩**에서 LimitError를 낸다. 일반 invocation executor의 explicit frame/trampoline과 더 넓은 depth는 후속 작업이다. 오류 뒤 depth가 복구됨을 검증했다.
- [ ] 본문 graph/source frame 및 전체 local/locale/definition 실행을 연결한다. 기존 ignored definition acceptance 17개는 계속 미완료다.

**Explicit-modifier gate:** Windows default/portable 각각 **386 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture **4,911 cases / 4,907 passed / 기존 runtime 경계 4 / failed 0**, stage **10,091 checks**, words **6,618 cases / failed 0**. capture graph 경계 **147건**(invocation 19·modifier value 109·ordered effect 19)과 static 경계 2건은 별도다. report 10개의 실제 binary/source hash를 확인했다. reviewed source `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`와 실행 DLL release `ded7793fe5795d79eda8e7138dce94aa056edf78`를 구분한다. full upstream suite·definition invocation acceptance·private C trace 동등성은 미검증이다. Linux/GitHub CI/CUDA 검증은 실행하지 않았다.

이후 modifier-scope 단계에서 local/global assignment dispatch와 여러 직선 문장의 마지막 결과·실패/효과 순서를 구현했다. x/y 직선 호출은 아래 operator-call 단계에서 지원한다. body graph 연결과 전체 callable scope는 후속 경계다. CUDA·optimizer 구현·GitHub CI는 보류한다.

기준 소스: [cx.c modifier 호출 및 local frame](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L259), [cx.c u/v와 noun m/n 설치](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L322), [p.c mnuvxy의 by-value resolution](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c#L616), [cx.c VXOPR executor 선택](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L1316).

<a id="je2p3-구현--modifier-지역전역-대입과-직선-본문-실행-2026-10-04-partial"></a>
##### JE2/P3 구현 — modifier 지역·전역 대입과 직선 본문 실행 (2026-10-04, partial)

이 단계는 앞 기록의 단일 문장 제한을 확장한다. `RuntimeParserHost`가 enqueue environment와 scoped assignment를 전달하고, definition 본문은 `ExplicitDefinition` 규칙으로 enqueue한다. 본문의 `=.`를 TopLevel의 전역 대입 규칙으로 승격하지 않는다. Engine은 global namespace와 별도의 호출별 `LocalFrame`을 유지하고 **현재 frame → global** 순서로 읽는다. caller의 다른 frame을 탐색하지 않는다. `u/v`와 noun 전용 `m/n`도 이 frame에 설치한다. noun은 공유 전에 한 번 freeze하며, 함수 Arc 및 큰 배열 payload를 유지한다. 전체 symbol table을 복제하지 않는다.

선택된 nonoperator mode 1/2 valence의 여러 `Body` 문장을 순서대로 실행하고 마지막 문장의 실제 `JEntity`/POS를 반환한다. 마지막 문장이 대입이어도 그 RHS noun/function 값을 반환한다. 뒤 문장이 있는 비대입 함수 결과에는 C와 같은 `noun result was required` 오류를 내지만 함수 대입은 계속 진행할 수 있다. 실패 시 local frame과 호출 depth는 복구된다. 앞서 완료된 global 대입은 유지하고, 실패한 RHS와 바깥 대입 대상은 commit하지 않는다.

C 비교로 구분한 이름 규칙:

- **현재 값이 있는 지역 이름**에 `=:`로 대입하면 domain error이다. 지역 이름이 선언만 되어 있고 아직 값이 없으면 global 대입이 허용되며, 그 이후 `=.`로 지역 값을 넣을 수 있다.
- ordinary 함수 NAME은 본문에서도 실행 시 lookup한다. 이를 반환할 때 마지막 지역 함수 값으로 재귀 치환하지 않는다. frame 종료 후 같은 이름의 global이 없으면 value error이고, 이후 global이 생기면 그 값을 조회한다. `u/v`의 by-value substitution 및 마지막 함수 대입의 실제 RHS 반환과 구별한다.
- C가 종료 시 fix하는 implicit locative `u./v.`는 ordinary NAME과 다른 경계다. 아래 x/y 직선 operator executor 구현과 구별하여 후속 구현한다.

체크리스트:

- [x] nonoperator modifier의 simple NAME `=.`/`=:` dispatch와 현재 지역→전역 lookup을 분리한다.
- [x] 여러 직선 문장, 마지막 대입 결과/POS, 중간 nonnoun 오류, committed global 효과와 실패 후 frame 복구를 검증한다.
- [x] 직접 정의와 실제 `1/2 : 0` block 입력을 비교한다. block은 C `0!:100` script delivery로 공급하고 Rust에는 같은 원문을 전달한다. JDo 단일 호출로 interactive block을 흉내 내지 않는다.
- [x] Rust regression **6개**, 공통 C corpus **31건**, stage 전용 **41건**을 추가한다. 지역 noun/function/adverb, ordinary NAME 반환 후 재조회, reserved operand, 대입 실패·효과 순서, 65,536-atom payload의 frame 종료 후 pointer 공유를 포함한다.
- [x] 아래 ordinary-reference 단계에서 cross-frame 함수 operand, local NAME을 포함한 global publication, 미초기화 local 함수 대입 및 operand/local collision 제한을 해소했다. 실제 implicit locative는 별도 미지원 경계다.
- [ ] control flow·nested definition framing·locale 및 전체 operator wrapper/scope는 후속 구현한다. x/y 직선 호출은 아래 operator-call 단계에서 지원한다. 기존 8중첩 제한과 ignored definition acceptance 17개는 유지한다.
- [ ] 본문 dependency/effect graph와 별도 body/caller diagnostic frame을 연결한다. graph 변환은 계속 `explicit modifier body graph requires invocation scope`를 반환한다. 실행 성공은 static graph 분석 완료를 뜻하지 않는다.

**Modifier-scope gate:** native Windows default/portable 각각 **392 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture **4,942 cases / 4,938 passed / 기존 runtime 경계 4 / failed 0**, stage **10,163 checks / failed 0**, words **6,618 cases / failed 0**. capture graph 경계 **164건**(invocation 28·modifier value 117·ordered effect 19)과 static 경계 2건은 별도다. report 10개의 binary/source hash를 확인했다. reviewed source는 `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`, 실행 DLL release는 `ded7793fe5795d79eda8e7138dce94aa056edf78`이다. full upstream suite·ignored definition acceptance·private C trace 동등성은 미검증이다. Linux/GitHub CI/CUDA 검증은 실행하지 않았다.

이후 ordinary-reference 단계에서 scoped-reference 사례를 C와 대조하고 ordinary NAME 제한을 해소했다. x/y 직선 호출은 아래 operator-call 단계에서 구현했으며, 본문 graph/source frame 및 control flow는 후속 경계다. Unsupported 전에 global 효과가 commit될 수 있으므로 이를 안전한 자동 재실행 신호로 사용하지 않는다. optimizer·CUDA 구현·GitHub CI는 보류한다.

기준 소스: [p.c 지역 조회와 global fallback](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c#L631), [s.c bound private name의 global 대입 금지](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/s.c#L718), [cx.c 중간 noun 결과 요구](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L67), [cx.c implicit locative fix](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L679), [af.c implicit u/v 처리](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/af.c#L53), [jerr.h EVNONNOUN](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/jerr.h), [i.c 오류 문구](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/i.c).

<a id="je2p3-구현--ordinary-name-scope-경계-해소-2026-10-04-partial"></a>
##### JE2/P3 구현 — ordinary NAME scope 경계 해소 (2026-10-04, partial)

- [x] C native oracle로 ordinary NAME의 cross-frame 전달·global publication·미초기화 local 대입·operand/local 이름 충돌을 확인했다. 이전 scoped-reference 보수적 경계는 아래 범위에서 해소했다.
- [x] ordinary NameRef를 implicit locative로 오인한 네 제한과 전체 함수 DAG의 이름 membership 재검사를 제거했다. 함수 이름은 expected POS를 가진 late reference로 유지한다. 호출은 현재 frame → global을 조회하며 caller frame을 캡처하거나 탐색하지 않는다. `u/v` operand substitution과 noun snapshot은 그대로다.
- [x] `smf=.u` 후 `smf`를 안쪽 modifier에 전달하면 안쪽에서 ordinary `smf` 실행은 안쪽 local 또는 global을 조회한다. caller의 지역 `smf`로 고정되지 않는다. `smexport=:smf/`도 ordinary 이름을 보존하고 frame 밖 global 재정의를 반영한다. 미초기화 `smf=.smf`는 RHS의 현재 noun/function 품사를 따르며 noun이면 snapshot, 함수면 NameRef이다. 같은 이름의 operand/local 대입도 유효하다.
- [x] Rust scope regression은 기존 Unsupported golden 1개를 실제 의미 검증 3개로 교체했다. 재정의·POS mismatch·undefined→defined, 실패 뒤 outer target/version 유지·이미 commit된 global publication과 frame 복구를 검증한다. 공통 single-line C corpus **21건**, stage에 이를 포함한 **45건**을 추가했다. 반환 함수의 C atomic representation과 실제 결과/오류를 비교한다. `af.c`를 검토 소스 hash 목록에 추가했다.
- [ ] 실제 implicit locative `u./v.`, 전체 operator wrapper/scope, control flow·nested scope와 body graph/source diagnostic frame은 후속 작업이다. x/y 직선 호출은 아래 operator-call 단계에서 지원한다. 일반 NAME 지원을 implicit-locative fix나 전체 closure 지원으로 확대하지 않는다. static/no-host modifier application은 계속 명시적 경계이며 실행 성공을 정적 분석 완료로 계산하지 않는다.

**Ordinary-reference gate:** native Windows default/portable 각각 **398 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 세 runtime 경로 **4,963 cases / 4,959 passed / 기존 runtime 경계 4 / failed 0**, stage **10,208 checks / failed 0**, words **6,618 cases / failed 0**. capture graph 경계 **171건**, static 경계 **2건**은 별도다. 새 runtime waiver는 없다. 보고서 10개의 실제 binary/source hash를 확인했다. reviewed source `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`, 실행 DLL release `ded7793fe5795d79eda8e7138dce94aa056edf78`를 구분한다. full upstream·ignored definition acceptance·private C trace 동등성은 미검증이며 optimizer·CUDA·GitHub CI는 보류한다.

이후 operator-call 단계에서 deferred callable의 직선 호출을 구현했다. implicit locative와 실행되지 않는 definition body graph/source frame은 후속 경계다. effect가 commit된 뒤 Unsupported가 날 수 있으므로 자동 replay하지 않는다.

Sources: [p.c ordinary lookup / mnuvxy](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c#L616), [cx.c return-time implicit-locative fix](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L679), [af.c hasimploc / fix scope](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/af.c#L17).

<a id="je2p3-구현--xy-operator의-deferred-verb와-직선-호출-2026-10-04-partial"></a>
##### JE2/P3 구현 — x/y operator의 deferred verb와 직선 호출 (2026-10-04, partial)

- [x] C `cx.c::jtxop2/VXOPR`처럼 mode 1/2 operator의 operand application은 **본문을 실행하지 않고 Verb를 구성**한다. 기존 `FunctionHead::ExplicitDefinition`과 공유 DefinitionCode를 유지하며 원래 순서의 operand를 붙인다. code의 modifier POS와 적용 결과의 Verb POS를 구별한다. 새로운 modifier 전용 AST나 definition 본문 noun reduction을 만들지 않는다.
- [x] 반환 verb의 직접/ordinary NAME 호출에 별도 x/y를 설치하고 기존 직선 본문 executor·tokenizer/enqueuer/parser를 공유한다. valence는 modifier operand 수가 아니라 실제 x/y 호출과 DefinitionCode의 monad/dyad section으로 선택한다. 빈 section은 ValenceError, 최종 함수 결과는 EVNONNOUN으로 처리한다. 생성 시 control-flow 본문을 실행하지 않으며, 해당 호출은 아직 Unsupported이다.
- [x] 호출별 local frame과 u/v 및 noun 전용 m/n을 설치·정리한다. 기존 current-frame→global 조회, ordinary NameRef의 late lookup, global 효과/실패한 outer 대입 보존을 유지한다. 정의 재대입은 이미 생성된 verb의 공유 code를 바꾸지 않고, 함수 operand의 ordinary 이름 재대입은 실행 시 반영한다. primitive 호출에는 새 함수 Arc 복사를 추가하지 않는다.
- [x] noun operand는 deferred construction에서 공유 저장소로 고정하고 이후 호출에서는 공유한다. 65,536-atom operand의 pointer 보존·이름 재대입 후 생존, code Arc 동일성, 생성/호출 효과 횟수, valence·POS·noun-result 오류와 반복 실패 후 frame 복구를 Rust regression **3개**로 확인했다. 재귀 제한은 기존 Windows **8중첩**을 유지한다.
- [x] 공통 C corpus **38건**, 이를 포함한 stage **67건**을 추가했다. scalar/vector/empty, noun operand snapshot, named operand 재정의, direct/block/two-valence 정의와 실패 효과를 비교한다. frontend probe와 C `5!:1` adapter는 boxed operator head + operand vector를 보존하여 적용 전 modifier와 적용 후 verb를 구분한다. 기존 nonoperator scope/capture 테스트도 유지한다.
- [ ] `u./v.` implicit-locative fix, control flow·nested scope, bare mode 3/4 verb invocation의 전체 지원, rank/insert 등 wrapper 안의 operator 실행, body graph/source diagnostic frame과 Logical lowering은 후속 작업이다. static/no-host application은 계속 명시적 경계다. DefinitionCode와 operand graph 보존은 본문 분석·compiled reuse 완료를 뜻하지 않는다.

**Operator-call gate:** native Windows default/portable 각각 **401 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 세 runtime 경로 **5,001 cases / 4,997 passed / 기존 runtime 경계 4 / failed 0**, stage **10,275 checks / failed 0**, words **6,618 cases / failed 0**. capture graph 경계 **184건**, static 경계 **2건**은 별도다. 기존 runtime waiver는 추가하지 않았다. 보고서 10개의 binary/reference/source hash를 검증했다. source pin `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`과 DLL release `ded7793fe5795d79eda8e7138dce94aa056edf78`는 다르다. full upstream·ignored definition acceptance·private C trace 동등성은 미검증이며 Linux/GitHub CI/CUDA는 실행하지 않았다.

**별도 oracle 경계:** 무한 재귀 operator 사례에서 j64 C oracle의 ctypes JDo가 `OSError: exception: stack overflow`로 종료했다. 정상 J LimitError 대조가 아니므로 위 성공 corpus에서 제외하고 `reports/operator-recursion-oracle-boundary-windows.json`에 기록한다. Rust depth-limit/frame-recovery regression만 통과했으며 이 사례의 C 동등성은 주장하지 않는다. 실패한 harness 실행을 successful gate로 계산하지 않았다.

아래 implicit-operand 단계에서 반환 시 fix를 구현했다. caller scope 전환을 포함한 직접 호출과 body graph/source frame·control executor는 후속 작업이다. optimizer·CUDA·GitHub CI 보류를 유지한다.

Sources: [cx.c jtxop2](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L749), [cx.c operator operand extraction](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L259), [cx.c x/y/u/v installation](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L269), [cx.c result audit](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L671), [cx.c executor selection](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L1316).

<a id="je2p3-구현--implicit-operand의-반환-시-고정-2026-10-04-partial"></a>
##### JE2/P3 구현 — implicit operand의 반환 시 고정 (2026-10-04, partial)

- [x] C `t.c`처럼 `u.`/`v.`를 VERB primitive로 enqueue한다. ordinary NAME이나 extension keyword로 바꾸지 않는다. registry version은 4이다.
- [x] `cx.c::xop`처럼 direct definition의 mode 판정에서 `u.`는 u, `v.`는 v 사용으로 계산한다. lexical VERB 분류와 adverb/conjunction definition POS는 구별한다.
- [x] modifier의 직선 본문이 함수를 반환할 때 departing frame의 최종 u/v binding으로 첫 implicit locative를 치환한다. 각 replacement 안으로 더 들어가지 않으며 ordinary NameRef는 유지한다. source operator/operand 순서·decoded gerund를 유지하고 바뀌지 않은 FunctionEntity는 Arc를 공유한다.
- [x] noun operand를 verb locative로 반환하면 DomainError이다. 미설치 operand의 반환은 C에서 함수 참조가 남을 수 있어 현재 Unsupported 경계이며 ValueError로 단정하지 않는다. 이미 commit된 global publication은 고정하지 않는다. 반환 뒤 ordinary 함수 이름 재정의와 실패 후 frame 복구를 회귀 검증한다.
- [x] unresolved implicit primitive의 contract는 unknown/effect barrier이며 graph rule은 DynamicOrUnknown이다. 이를 pure 배열 kernel 또는 shape 보존 힌트로 간주하지 않는다.
- [ ] **후속 진행:** 아래 caller-scope 단계에서 direct `u./v.` 호출과 ordinary alias 호출, publication 밖 호출 오류를 구현했다. wrapper 내부 raw 호출, 미설치 operand 반환 참조, 전체 locale/control/body graph/source diagnostic frame은 후속 경계다. 일반 함수 lookup으로 대체하면 caller-local 이름을 잘못 해석할 수 있다.

Windows C oracle에서 반환 `u.`/`u./`/`v.`, operand의 지역 재대입, ordinary NAME 재정의, 전역 raw locative publication을 확인했다. Rust regression **3개**는 lexical/definition mode, 반환 뒤 호출·오류·frame 복구, fork의 동일 operand Arc 공유와 rank DAG 보존, graph unknown 규칙을 검증한다. 공통 runtime corpus **29건**, 이를 포함한 stage **45건**을 추가했다. raw 호출과 미설치 operand 반환은 새 successful runtime corpus로 계산하지 않는다.

C `5!:1`은 source graph가 아닌 실행 객체의 표현이므로 `u. "0`에서 u를 +로 고정한 뒤 C는 redundant rank를 생략한다. 비교용 frontend probe만 +의 정확한 `[0,0,0]` rank를 생략하며 Semantic IR의 source rank parent는 보존한다. C의 zero-rank +와 rank를 유지하는 ravel을 함께 대조하고, Rust capture에서 rank parent 보존을 확인한다. decoded gerund는 source edge처럼 재귀 고정하지 않고 constructor auxiliary로 유지한다. gerund operator별 AR 재구성과 전체 constructor 특수화는 후속 감사 대상이다. [t.c +의 rank](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/t.c#L113), [cr.c rank 재구성](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c#L778).

**Implicit-return gate:** native Windows default/portable 각각 **404 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture **5,030 cases / 5,026 passed / 기존 runtime 경계 4 / failed 0**, stage **10,320 checks / failed 0**, words **6,618 cases / failed 0**. capture graph 경계 **195건**, static 경계 **2건**은 별도다. 새 runtime waiver는 없다. frontend report 10개의 actual binary/reference/source hash를 검증했으며 `t.c`를 source hash 목록에 추가했다. source pin `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`과 DLL release `ded7793fe5795d79eda8e7138dce94aa056edf78`를 구분한다. full upstream·ignored definition acceptance·private C trace 동등성은 미검증이다. optimizer·CUDA·Linux/GitHub CI는 실행하지 않았다.

Sources: [t.c primitive 등록](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/t.c#L221), [cx.c mode 판정](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L766), [cx.c 반환 fix](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L679), [af.c 첫 implicit reference fix](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/af.c#L117), [sc.c caller scope 전환](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sc.c#L124).

<a id="je2p3-구현--implicit-operand의-caller-scope-호출-2026-10-04-partial"></a>
##### JE2/P3 구현 — implicit operand의 caller scope 호출 (2026-10-04, partial)

- [x] C `sc.c::unquote`와 Windows oracle에서 ordinary u와 implicit u.의 caller-local 조회 차이, v., monad/dyad, noun/missing operand 오류를 확인한다.
- [x] direct primitive 및 ordinary alias를 통한 implicit 호출에서 현재 frame의 operand를 확보한 뒤 caller 환경에서 실행하고, 성공·J 오류 모두에서 현재 frame을 복구한다. 전달 noun을 복사하지 않고 기존 parser/runtime executor를 공유한다.
- [x] Rust 회귀 및 C 양 버전 대조에 caller-local 충돌, operand 지역 재대입, 전역 publication, 오류 후 frame/전역 효과 복구를 추가한다. graph unknown 계약은 유지한다.
- [ ] `/`·rank·train 내부의 raw implicit 실행, 미설치 operand의 반환 참조, 전체 locale/control/body graph/source frame은 별도 후속 경계다. 이 단계는 global locale path 전환의 전체 구현이 아니다.

Rust 회귀 **2개**가 ordinary u와 u.의 caller-local 차이, u/v monad/dyad, noun·missing operand 오류, 안쪽 operator의 실패 뒤 global 효과 보존과 frame 복구를 확인한다. 기존 implicit-return regression도 유지한다. 공통 runtime **23건**, 이를 포함한 stage **47건**을 추가했다. **Caller-scope gate:** native Windows default/portable 각각 **406 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 세 runtime 경로 **5,053 cases / 5,049 passed / 기존 runtime 경계 4 / failed 0**, stage **10,367 checks / failed 0**, words **6,618 cases / failed 0**. capture graph 경계 **204건**, static 경계 **2건**은 별도다. 새 runtime waiver는 없다. frontend report 10개의 실제 binary/reference/source hash를 검증했다. source pin `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`과 DLL release `ded7793fe5795d79eda8e7138dce94aa056edf78`를 구분한다. full upstream·ignored definition acceptance·private C trace 동등성은 미검증이다. optimizer·CUDA·Linux/GitHub CI는 실행하지 않았다. 미설치 operand의 **호출**은 ValueError이며 **반환** 참조는 아직 별도 경계다. 재귀 depth 제한은 기존 Windows bound를 유지하며, suspend된 callee도 invocation depth에 포함한다.

Sources: [sc.c local operand와 caller 환경 전환](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sc.c#L122), [sc.c implicit primitive 호출](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sc.c#L433).

<a id="je2p3-구현--wrapper와-train의-scope-aware-실행-2026-10-04-partial"></a>
##### JE2/P3 구현 — wrapper와 train의 scope-aware 실행 (2026-10-04, partial)

- [x] C `ar.c::jtredg`의 오른쪽 결합, `j.h::FORK1/FORK2`의 오른쪽 가지 우선, rank의 cell/frame·prefix agreement를 검토한다.
- [x] A3 executor의 rank/reduction cell 알고리즘을 callback seam으로 공유한다. 기존 primitive kernel 경로는 유지하고, Unsupported composition에만 runtime FunctionEntity 실행을 연결한다. operator/rank DAG를 flatten하지 않는다.
- [x] `/`의 비어 있지 않은 monad, uniform nonempty rank, Hook/Fork/Atop의 monad/dyad에서 실제 child invocation마다 이름과 implicit caller scope를 조회한다. fork input은 공유 noun으로 보존하고 오른쪽 가지 오류 전에 왼쪽 가지를 실행하지 않는다.
- [x] Rust 및 C 양 버전 corpus에 wrapper·caller-local 충돌·branch effect/error 순서를 추가한다. static unknown 계약과 effect barrier는 유지한다.
- [ ] empty identity/prototype, heterogenous rank fill/padding, sparse·dyadic insert와 noun-left/capped train은 별도 확인한다. callback의 Unsupported는 commit된 효과가 없는 재실행 허가가 아니다.

**Wrapper/train gate:** Windows default/portable 각각 **408 passed / 17 ignored**, fmt/clippy/build 통과. native Python **27 passed**. j64/AVX2 각각 세 runtime 경로 **5,089 cases / 5,088 passed / runtime 경계 1 / failed 0**, stage **10,424 checks / failed 0**, words **6,618 cases / failed 0**. capture graph 경계 **222건**, static 경계 **2건**은 별도다. 기존 runtime 경계 3건(Atop 및 fork 안 named insert 2건)을 해소하고 해당 waiver를 제거했다. 새 waiver는 없다. frontend report 10개의 binary/reference/source hash를 확인했고 `ar.c`를 검토 소스 목록에 추가했다. source pin `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`, DLL release `ded7793fe5795d79eda8e7138dce94aa056edf78`를 구분한다. full upstream·ignored definition acceptance·private C trace 동등성은 미검증이며 optimizer·CUDA·Linux/GitHub CI는 실행하지 않았다.

Rust regression 2개와 기존 named-insert provenance regression은 source DAG 보존·오른쪽 결합·caller-local·rank shape와 branch effect/error 순서를 확인한다. 공통 runtime **36건**, 이를 포함한 stage **57건**을 추가했다. primitive 호출에는 함수 Arc 복사를 추가하지 않고, Runtime fallback만 composition을 순회한다. A3의 Hook/Fork executor와 전체 body graph lowering 완료를 주장하지 않는다. 다음은 empty reduction identity와 empty rank prototype을 C와 대조한다.

Sources: [ar.c reduce](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ar.c#L513), [j.h fork execution](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/j.h#L1249), [cr.c rank](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c).

<a id="je2p3-구현--empty-identity와-순수-ravel-prototype-2026-10-04-partial"></a>
##### JE2/P3 구현 — empty identity와 순수 ravel prototype (2026-10-04, partial)

- [x] C `ai.c::jtiden`과 Windows oracle로 `+ - * %`의 identity를 확인했다. empty rank에서 사용자 verb가 prototype 계산용으로 한 번 실행되어 global 효과가 남을 수 있다는 점을 확인했다.
- [x] primitive witness를 현재 이름/POS·implicit caller scope에서 조회하되 definition 본문을 실행하거나 분석한 것으로 계산하지 않는다. 확인된 네 identity를 기존 kernel 경로로 계산하며 caller frame을 복구한다.
- [x] 순수 monadic ravel은 cell shape만으로 empty rank output shape/type을 계산한다. primitive와 implicit wrapper가 같은 kernel을 사용한다. negative rank·다차원 zero axes·character type·caller-local collision을 검증한다.
- [x] Rust와 C 양 버전 회귀를 추가하고 일반 사용자 verb의 prototype 및 empty identity 경계를 별도로 유지한다. 일반 prototype의 효과·suppressed error·fill/padding, 다른 primitive prototype, sparse·dyadic insert는 후속 작업이다.

**Empty-scope gate:** native Windows default/portable 각각 **410 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 세 runtime 경로 **5,130 cases / 5,129 passed / runtime 경계 1 / failed 0**, stage **10,474 checks / failed 0**, words **6,618 cases / failed 0**. capture graph 경계 **230건**, static 경계 **2건**은 별도다. 새 waiver는 없다. report 10개의 binary/reference/source hash를 확인했고 `ai.c`를 검토 목록에 추가했다. Rust regression **2개**, 공통 runtime **41건**, 이를 포함한 stage **50건**을 추가했다.

일반 explicit verb의 empty rank에서는 C가 body를 한 번 실행해 count를 1로 만들었으며, unknown explicit reduction identity는 DomainError를 내고 count는 0이었다. 두 경계의 j64 probe는 `reports/empty-prototype-oracle-windows.json`에 별도 기록한다. Rust는 아직 명시적 Unsupported이며 이 두 사례의 동등성/순수성을 주장하지 않는다. runtime primitive witness는 compile-time binding proof를 뜻하지 않고 analyzer의 unknown 계약을 바꾸지 않는다. full upstream·ignored definition acceptance·private C trace는 미검증이며 optimizer·CUDA·Linux/GitHub CI는 실행하지 않았다. 다음은 noun-left fork이다.

Sources: [ai.c identities](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ai.c#L368), [ar.c empty reduction](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ar.c#L505), [cr.c rank execution](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c).

<a id="je2p3-구현--noun-left-fork-호출과-snapshot-공유-2026-10-04-partial"></a>
##### JE2/P3 구현 — noun-left fork 호출과 snapshot 공유 (2026-10-04, partial)

- [x] C `j.h` NVV 경로와 Windows oracle에서 왼쪽 noun은 생성 시 snapshot이고 오른쪽 h 실행 후 g에 넘겨진다는 것을 확인한다. monad/dyad와 이름 재대입·agreement 오류를 대조한다.
- [x] 기존 parser의 source Fork DAG와 CompletedParseResult의 공유 noun을 그대로 사용한다. 오른쪽 child만 호출하고 왼쪽 값은 지연 이름 조회로 바꾸지 않는다. 두 가지 입력 보존을 위한 불필요한 공유 변환은 하지 않는다.
- [x] 65,536-atom snapshot의 pointer 유지·이름 재대입 뒤 생존, implicit caller scope, join 실패 전에 commit된 오른쪽 효과와 오류 후 frame 복구를 검증한다.
- [x] C 양 버전 및 기존 frontend/portable gate를 통과한 뒤 기록한다. capped fork, noun-left GraphForm/Logical lowering 전문화, 일반 prototype/control/body graph는 별도 후속 작업이다.

Sources: [cf.c noun fork](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L59), [j.h NVV execution](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/j.h#L1277).


**Noun-fork gate:** native Windows default/portable 각각 **412 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 세 runtime 경로 **5,156 cases / 5,155 passed / runtime 경계 1 / failed 0**, stage **10,511 checks / failed 0**, words **6,618 cases / failed 0**. capture graph 경계 **234건**, static 경계 **2건**은 별도다. report 10개의 actual binary/reference/source hash를 확인했고 새 waiver는 없다. Rust regression **2개**와 기존 computed-noun capture regression을 실제 실행 결과 검증으로 갱신했다. 공통 runtime **26건**, 이를 포함한 stage **37건**을 추가했다. full upstream·ignored definition acceptance·private C trace 동등성은 미검증이며 optimizer·CUDA·Linux/GitHub CI는 실행하지 않았다.

<a id="je2p3-구현--capped-fork의-생성-의미와-graph-표현-2026-10-05-partial"></a>
##### JE2/P3 구현 — capped fork의 생성 의미와 graph 표현 (2026-10-05, partial)

- [x] C `t.c::CCAP`, `cf.c::jtcap/jtfolk`와 `j.h`를 확인하고 Windows j64/AVX2의 finite probe를 기록했다. 기존 `reports/capped-fork-oracle-windows.json`은 **구현 전 reference-only 관찰**이며 conformance 통과 보고서로 바꾸지 않는다.
- [x] `[:`를 core VERB primitive로 등록한다(registry **5**). 일반 tokenizer/enqueuer 경로를 쓰고 standalone monad/dyad는 argument type/empty 여부에 관계없이 ValenceError를 낸다. primitive의 보수적 unknown analysis contract는 callable/pure array kernel 지원을 뜻하지 않는다.
- [x] fork 생성 시 직접 `[:` 또는 현재 **single name의 직접 binding이 `[:`**인지 판정한다. alias chain을 추적하지 않는다. 생성 뒤 첫 이름의 값/POS 변경에도 capped 의미를 유지하며 ordinary fork의 nameref는 계속 late lookup한다. explicit operand substitution과 caller-local constructor 환경, gerund AR decoder도 같은 constructor seam을 쓴다.
- [x] `FunctionHead::Fork`와 원래 세 operand DAG·NAME/source span을 보존한다. `FunctionEntity.fork_semantics`는 Fork에만 존재하는 불변 constructor 의미(Ordinary/Capped)이며 actual argument fact나 optimizer proof가 아니다. 생성 시점의 첫 이름 read/version은 `Program/Plan.fork_name_reads`와 capture `ForkNameResolved` sidecar에 분리한다. 이 sidecar는 cached execution guard가 아니다. no-host에서 POS만 알려진 이름은 direct-binding proof가 없으므로 Unsupported construction 경계다.
- [x] capped 호출은 h(x,y) 다음 g(monad)를 실행하고 첫 operand를 호출하지 않는다. source Fork는 그대로 두고 Graph IR **0.5**에서 Pipeline region과 h→g dataflow를 유도한다. ParallelBranchCandidate·RetainedValueCandidate를 주지 않는다. g/h의 late name/POS·effect/error barrier를 유지하며 optimizer는 실행하지 않는다.
- [x] direct/named cap, alias chain·재대입·POS 변경, monad/dyad, operand/local scope, 오류 뒤 효과/복구, source DAG·constructor dependency·pipeline hint와 gerund AR provenance 회귀를 추가했다. C AR의 첫 operand `[:` 정규화는 oracle projection에만 적용하며 source NAME을 지우지 않는다.

**Cap gate:** native Windows default/portable 각각 **418 passed / 17 ignored**, fmt/clippy/build 통과. native Python **27 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture 경로 **5,207 cases / 5,206 passed / 기존 runtime 경계 1 / failed 0**, stage **10,586 checks / failed 0**, words **6,618 cases / failed 0**. capture graph 경계 **239건**, 기존 static 경계 **2건**은 따로 기록한다. 새 runtime waiver는 없다. Rust integration regression **5개**와 POS-only constructor proof unit regression **1개**, 공통 runtime **51건**, 이를 포함한 stage **75건**을 추가했다. frontend report 10개의 actual binary/reference/source hash를 검증했다. source pin `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`과 DLL release `ded7793fe5795d79eda8e7138dce94aa056edf78`는 구분한다. full upstream·ignored definition acceptance·private C trace 동등성은 미검증이며 optimizer·CUDA·Linux/GitHub CI는 실행하지 않았다.

**다음 체크리스트:**

- [x] 남은 verb-valued rank operand를 C `cr.c`의 innate rank/constructor 규칙과 대조했다. 아래 verb-valued rank 단계에서 고정 constructor rank와 dynamic operand binding을 구분한다.
- [ ] noun-left GraphForm/Logical 전문화·일반 empty prototype의 effects/error suppression·heterogeneous fill/padding·sparse·dyadic insert·full definition/control/body graph는 계속 미완료다. 외부 static catalog의 POS-only first NAME은 불충분한 proof이므로 생성 경계를 유지한다.

Sources: [t.c cap primitive](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/t.c#L163), [cf.c single-name cap 검사](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L38), [j.h capped call](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/j.h#L1249).

<a id="je2p3-구현--verb-valued-rank의-생성-정보와-실행-이름-분리-2026-10-05-partial"></a>
##### JE2/P3 구현 — verb-valued rank의 생성 정보와 실행 이름 분리 (2026-10-05, partial)

- [x] C `cr.c::jtqq`, `sc.c::jtnamerefacv`, `ja.h`의 rank accessor와 core primitive/derived constructor를 검토했다. `u"v`는 오른쪽 verb를 실행하지 않고 **그 함수 객체의 monad/left/right header rank**를 복사한다. negative requested rank와 derived verb의 실제 header rank는 다르다. 예를 들어 `+"_1`의 requested monad rank는 -1이지만 header monad rank는 `_`다. gerund rank-derived verb의 header도 모두 `_`다.
- [x] ordinary NAME이 parser stack에 들어갈 때 현재 binding의 header rank를 immutable `FunctionEntity.name_ranks`에 복사한다. alias의 기존 header를 읽고 현재 alias target을 따라가지 않는다. 미정의 ordinary name의 C header는 모두 `_`다. 이 metadata는 executable NAME을 고정하거나 pure로 만들지 않는다. 기존 implicit `u.`의 header와 explicit actual operand `u`의 header도 구분한다.
- [x] 원래 rank conjunction과 두 source operand, NAME/span을 그대로 보존한다. `requested_ranks()`가 noun rank spec 또는 오른쪽 verb header를 읽으며 runtime/name lookup을 수행하지 않는다. 왼쪽 callable의 late binding/POS 검사, nested rank 경계와 기존 prefix agreement·오류 순서는 유지한다. 오른쪽 NAME의 후속 재정의·noun/adverb로의 POS 변경은 이미 생성된 rank를 바꾸지 않는다.
- [x] constructor header read/version/span은 `Program/Plan.name_rank_snapshots`와 capture `FunctionNameRank` observation으로 별도 기록한다. 오른쪽 rank operand는 executable late-reference 목록에서 제외한다. 이 sidecar는 cache guard나 purity proof가 아니다. 당시 primitive registry는 **6**, Graph IR은 **0.6**이며 `GraphForm::Rank.requested_ranks`를 추가한다. source noun인 `rank_spec`과 source RHS function을 혼동하지 않는다.
- [x] static catalog의 `declare_primitive_verb`가 header 근거를 제공한다. POS만 알려진 RHS 이름은 유효한 J 문법이지만 **Unsupported construction proof 경계**로 남긴다. known header만으로 executable binding을 동결하지 않는다. 입력 payload 없이 `[1_000_000_000_000, 3]` metadata로 ravel-cell 결과 shape를 분석하는 회귀를 추가했다.
- [x] Rust 회귀 7개는 RHS 비실행, alias·미정의 이름·재대입, lhs late execution, explicit/implicit operand, 음수·비대칭 rank, empty pure ravel, prefix agreement 오류 후 복구, source/capture/Graph/A3 및 대용량 metadata 분석을 검사한다. C `b.0` header projection과 runtime 값/오류를 각각 대조한다. `reports/verb-rank-oracle-windows.json`은 구현 전 **C reference-only 관찰**이며 conformance 보고서와 구분한다.

**Verb-rank gate:** native Windows default/portable 각각 **425 passed / 17 ignored**, fmt/clippy/build 통과. native Python **27 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture **5,321 cases / 5,321 passed / runtime 경계 0 / failed 0**, stage **10,757 checks / failed 0**, words **6,618 cases / failed 0**. capture graph 경계 **244건**과 static 경계 **2건**은 별도로 남는다. primitive/derived header **39건**과 alias header를 C `b.0`로 대조했다. frontend report 10개의 실제 binary/reference/source hash를 검증했다(`ja.h` 포함). source pin `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`과 DLL release `ded7793fe5795d79eda8e7138dce94aa056edf78`는 구분한다. 현재 corpus의 runtime 경계 0은 full J 지원을 뜻하지 않는다. full upstream·ignored definition acceptance·private C trace 동등성은 미검증이며 optimizer·CUDA·Linux/GitHub CI는 실행하지 않았다.

**반환 경계 후속 검토:** `cx.c`는 explicit modifier가 non-noun을 반환할 때 첫 implicit locative를 fix하고, `af.c::jtfixa`는 치환한 operand로 modifier를 다시 실행해 새 derived entity를 만든다. 따라서 본문에서 `(,"u.) y`를 즉시 실행하면 `u.` header `_`를 쓰지만, `,"u.`를 반환해 `u=+`로 fix한 뒤 실행하면 새 entity의 RHS header 0을 쓴다. `[2,3]` 입력의 ravel 결과는 각각 `[6]`과 `[2,3,1]`이다. 이 재구성은 기존 entity의 rank를 late lookup으로 바꾸는 것과 다르다. C 기본·AVX2와 Rust의 반환 구문 10건을 먼저 직접 대조했고, 동일 결과를 확인했다. 추가 Rust 회귀와 공통 runtime corpus로 이 차이를 보존한다. 런타임 구현 변경은 필요하지 않았다.

**Return-boundary gate:** native Windows default/portable 각각 **426 passed / 17 ignored**, fmt/clippy/build 통과, Python **27 passed**. C 기본·AVX2 각각 세 runtime 경로 **5,331 cases / 5,331 passed / runtime 경계 0 / failed 0**, stage **10,767 checks**, words **6,618 cases**, 실패 0. capture graph **250건**, static **2건** 경계는 별도다. report 10개의 실제 source/reference/binary hash를 다시 확인했다. 추가한 10개 공통 구문은 세 runtime 경로와 stage 모두에 포함한다. 위 Verb-rank gate와 이 Return-boundary gate는 이전 단계의 기록이다. **당시에는 NV2가 최신 gate였지만 현재 최신 검증은 §10의 NV3d2b2a이며, graph-readiness 이력은 GF6a까지 진행됐다.** 미검증 범위와 optimizer/CUDA/Linux/GitHub CI 유보는 동일하다.

Sources: [cx.c modifier return fix](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L684), [af.c implicit operand](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/af.c#L117), [af.c reconstruct modifier](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/af.c#L193).

**다음 체크리스트:**

- [ ] noun-left rank/gerund runtime과 noun-left GraphForm/Logical 전문화를 C constructor/call 규칙에 맞춰 확장한다.
- [ ] 일반 empty prototype의 effects/error suppression, heterogeneous fill/padding, sparse, dyadic insert 및 full definition/control/body graph는 계속 미완료다. header 정보로 이 실행 경계를 우회하지 않는다.

Sources: [cr.c rank constructor](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c#L734), [sc.c NAME header copy](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sc.c#L364), [ja.h rank accessor](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ja.h#L745), [t.c primitive headers](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/t.c), [ap.c prefix/infix header](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ap.c#L965), [ar.c insert header](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ar.c#L1009).

<a id="je3--operator-specific-higher-order-view-필요성-검증"></a>
##### JE3 — operator-specific higher-order view 필요성 검증

- [ ] **기본값은 generic `EntityArray`가 아니다.** 먼저 `GerundView` / `InterpretedEntitySequence`처럼 해당 J operator의 semantic interpretation을 직접 표현한다.
- [ ] jsource `jtfxeachv`의 fake-BOX/function payload representation을 RustJ `Value::Boxed`나 새로운 J-visible noun type으로 복제하지 않는다.
- [ ] view가 shape/rank를 보존해야 하면 source boxed noun에서 가져오고, 그 shape가 실제 selector/agenda/tie 등 observable semantics에 필요한지 operator별로 증명한다.
- [ ] current `decoded_gerund: Option<Vec<Arc<FunctionEntity>>>`가 shape/order/name-binding 정보를 잃는지 `@.`, grave/tie 계열 등 각 지원 operator에 대해 감사한다.
- [ ] 단순 ordered sequence면 충분하면 shape-bearing view를 만들지 않는다.
- [ ] 두 개 이상의 독립적인 J semantic use case가 동일한 shaped-entity algebra를 요구할 때만 generic `EntityCollectionView`를 추출한다.
- [ ] train/Hook/Fork DAG는 collection으로 flatten하지 않고 shared FunctionEntity graph로 유지한다.
- [ ] arbitrary verb array를 새 J language feature처럼 허용하지 않는다.

**JE3 완료 조건:** operator-specific view로 충분한지 먼저 판정하고, generic shaped collection은 실제 공통 semantic law가 발견된 경우에만 추출한다. jsource 내부 representation의 편의만으로 `EntityArray`를 만들지 않는다.

<a id="je4--gerundboxed-higher-order-semantics-통합"></a>
##### JE4 — gerund/boxed higher-order semantics 통합

- [ ] gerund를 새 global atom/POS type으로 만들지 않고 **boxed noun + modifier-context interpretation**이라는 기존 J semantics를 유지한다.
- [ ] gerund interpretation이 필요할 때만 boxed noun에서 operator-specific `GerundView`/`InterpretedEntitySequence`를 만든다. source boxed noun의 shape는 해당 operator가 필요할 때만 view metadata로 보존한다.
- [ ] generic `EntityCollectionView`는 JE3의 공통성 증명이 끝난 뒤에만 추출한다.
- [ ] view 내부에 function entity ref가 있어도 그 function에 container의 rank/shape를 복사하지 않는다.
- [ ] gerund 내부 name/function reference의 fix/late-binding/version 규칙을 보존한다.
- [ ] ordinary boxed data와 gerund interpretation이 같은 payload에서 context에 따라 달라지는 golden test를 추가한다.
- [ ] 현재 `decoded_gerund: Option<Vec<Arc<FunctionEntity>>>` 특수 필드를 공통 entity view로 대체할 수 있는지 검토하고, 의미 손실이 있으면 유지한다.

**JE4 완료 조건:** gerund와 boxed data의 문맥적 차이를 잃지 않으면서 higher-order entity collection을 공통 abstraction으로 표현할 수 있다.

<a id="je5--entity-algebra와-array-execution-algebra의-경계"></a>
##### JE5 — entity algebra와 array-execution algebra의 경계

- [ ] `JEntity` layer와 `Logical Execution IR`의 역할을 분리한다: function entity 자체는 logical array value가 아니고, **적용된 verb가 noun input을 받아 noun result를 만드는 순간** array execution graph로 내려간다.
- [ ] monadic application을 `JEntity(Verb) × JEntity(Noun) → JEntity(Noun)`, dyadic application을 `Noun × Verb × Noun → Noun`의 semantic contract로 검증한다.
- [ ] adverb/conjunction application은 일반적으로 function entity derivation이지만, parser bident/trident semantic action이 immediate noun result를 만들 수 있는 경우까지 `JEntity` boundary가 표현한다. 어느 경우에도 parser result를 즉시 physical execution representation으로 고정하지 않는다.
- [ ] `CellApply`/Reduce/Scan/Reindex가 entity layer가 아니라 applied array-computation layer에 남는지 확인한다.
- [ ] effect flow(namespace/I/O/state)와 entity/value flow를 직교하게 유지한다.
- [ ] J Graph/Logical IR이 `JEntityArray`의 physical layout이나 entity-container storage를 알 필요가 없다는 verifier/invariant를 둔다.

**JE5 완료 조건:** `JEntity` 일반화가 현재의 “verb application = logical array computation” 모델을 흐리지 않고 오히려 그 경계를 명시적으로 만든다.

<a id="je6--migration-cleanup과-비용-검증"></a>
##### JE6 — migration cleanup과 비용 검증

- [ ] compatibility adapter와 중복 `Noun|Function` carrier를 제거한다.
- [ ] public/internal API 이름을 정리하고 `JEntity`/`EntityArray` ownership/lifetime 문서를 고정한다.
- [ ] large derived function, gerund, repeated binding에서 deep-copy/refcount churn이 악화되지 않는지 benchmark한다.
- [ ] compiler coverage manifest에 entity-layer 지원/late-binding/runtime fallback 경계를 추가한다.
- [ ] M2 frontend conformance corpus와 기존 J Graph/A3 golden을 전부 다시 통과시킨다.
- [ ] Logical/Physical Array invariant에서 BufferId/stride/device가 entity layer로 역류하지 않았는지 구조 검사를 추가한다.

**JE6 완료 조건:** 기존 observable J semantics와 compiler pipeline 결과가 유지되고, 중복 carrier를 줄였으며, 공통 entity abstraction이 storage/runtime coupling을 새로 만들지 않는다.

**운영 규칙:** 이후 `JEntity`/higher-order view 관련 진행 보고는 반드시 `JE0`–`JE6` 항목 번호로 보고한다. 새 요구사항은 임시 TODO로 분산시키지 않고 먼저 이 체크리스트의 적절한 단계에 추가한다. 최소 `JEntity`는 M2의 중복 carrier를 줄이는 작은 seam부터 허용하지만 broad rewrite는 금지한다. JE3에서 공통 shaped-entity algebra가 입증되지 않으면 generic `EntityArray`/`EntityCollectionView`를 구현 목표로 강제하지 않는다.

<a id="a0--문서아키텍처-경계"></a>
### A0 — 문서/아키텍처 경계

- [x] RustJ 내부 compiler stage의 논리적 경계를 확정한다.
- [x] Semantic Analyzer 입력 전에 hook/fork/train/derived verb/rank를 제거하지 않는 원칙을 확정한다.
- [x] `J Semantic Array IR`과 `Logical Array IR / Plan`을 구분한다.
- [x] generic boundary 후보를 semantic analysis 이후의 Logical Array IR로 이동한다.
- [x] 문서를 `PROJECT.ko.md`로 통합한다.
- [x] 현재 지원 subset의 source frontend → FunctionEntity → J Graph → canonical Logical IR 분석 경계를 연결했다. full frontend/execution 완료는 M2/M4에서 별도로 추적한다.

<a id="a05--jsource-compatible-parser-이행-체크리스트"></a>
### A0.5 — jsource-compatible parser 이행 체크리스트

이 절의 **F0–F2 + P0–P7이 frontend/parser migration의 authoritative checklist**다. F0은 word formation, F1은 enqueue/primitive resolution, F2는 parse-queue skeleton을 담당한다. 그 뒤 P 단계에서 parser semantic construction/name-resolution/cutover를 완성한다. P8은 A1/A2/A3로 넘기는 integration handoff다.

> **운영 원칙 (2026-10-01 확정):** Tokenizer/word formation, Enqueuer, Parser는 RustJ 고유 frontend 문법을 새로 설계하지 않고 current jsource의 observable frontend semantics를 충실히 이식한다. representation은 Rust-native여도 되지만 word boundary, enqueue classification/lookup timing, parser row eligibility/order, modifier construction boundary/result POS/error semantics는 jsource가 기준이다. Parser가 만든 completed `FunctionEntity` DAG가 canonical semantic source이며 downstream이 이를 `reduce/rank` 같은 축약 필드로 대체해서는 안 된다.
>
> **현재 실행 순서:** F0 differential 0-mismatch 기록 → F1 Enqueuer 분리 → F2 parse queue → P1 stack/value model → P2 9-row engine → P3 construction semantics → P4 name/assignment sequencing → P5 semantic/compiler fact 분리 → P6 conformance gate → P7 legacy parser 제거. 각 단계는 아래 완료 조건을 만족한 경우에만 완료로 체크한다.

검토 기준은 2026-09-30의 `jsoftware/jsource` master(`13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`)이다. 특히 다음을 함께 oracle로 본다.

- `jsrc/p.c::cases[]`: 9-row J parsing rule의 선언형 기준. 현재 runtime parser가 직접 순회하는 테이블은 아니고 tacit translator에도 사용된다.
- `jsrc/p.c` runtime parser의 `ptcol`/bit-mask dispatch: 같은 row eligibility/order를 실제 parser hot path에서 구현한다.
- `jsrc/cf.c`의 Hook/Fork 및 bident/trident dispatch.
- 각 ADV/CONJ constructor(`jtslash`, `jtqq` 등): modifier application 시 J-defined construction validation과 실제 result POS를 결정한다.

RustJ가 그대로 맞춰야 하는 것은 **word/class resolution timing, row eligibility와 precedence, reduction extent, result parser class/POS, construction-time J errors, assignment/parenthesis/name-resolution semantics**다. C의 bit packing, refcount, in-place bookkeeping, cached function pointer, `localuse` 최적화는 이식 대상이 아니다.

RustJ는 compiler이지만 jsource parser가 실행과 분리된 정적 AST parser가 아니라는 점을 보존해야 한다. rows 0–2의 verb application은 **parser-visible effect/value dependency가 없다는 것이 증명된 경우에만** Noun-producing semantic application으로 defer할 수 있다. 그 실행이 이후 name/locale lookup, assignment state, modifier operand value, result POS 또는 construction-time error에 영향을 줄 수 있으면 정적 parser가 효과를 무시한 채 진행해서는 안 된다. v0 correctness baseline은 동일한 9-row engine의 runtime semantic action/fallback을 사용하고, 이후 guard/multiversion으로 정적 범위를 넓힌다. rows 3–4 역시 modifier application 시점에 필요한 J construction semantics를 수행하여 completed entity/POS/error를 결정해야 한다.

<a id="frontend-파일-경계"></a>
#### Frontend 파일 경계

| 파일 | 책임 | 입력 → 출력 |
|---|---|---|
| `src/tokenizer.rs` | `w.c::jtwordil` state machine, raw words·parse-visible comment cutoff | source bytes → byte spans |
| `src/enqueuer.rs` | `jtenqueue` 해석, primitive/literal/name/copula·환경별 flags | source + tokenizer spans → `EnqueuedWord` queue |
| `src/parser.rs` | parse class/9-row matcher, stack reduction, modifier/train construction, parser-time name/POS lookup | typed queue → `Program`/completed `FunctionEntity` |
| `src/semantic.rs` | target-independent 의미 객체·rank constructor 계약·binding/version model | parser 결과를 실행/분석 계층에 전달 |

Tokenizer·enqueuer·parser 구현은 각각 한 파일이 소유한다. 기존 `scanner` module과 `semantic::parse`/row API는 compatibility re-export만 남겨 기존 사용자를 보호하며 별도 grammar를 유지하지 않는다. parser는 enqueue 결과를 spelling으로 다시 분류하지 않는다. execution/target 선택을 이 세 파일에 넣지 않는다.

<a id="최우선-실행-계획과-단계별-논리-동등성-2026-10-03"></a>
#### 최우선 실행 계획과 단계별 논리 동등성 (2026-10-03)

Tokenizer → Enqueuer → Parser의 jsource 충실도를 다른 구현 작업보다 먼저 완성한다. 기존 F0–F2/P0–P7 체크리스트를 그대로 사용하며 새 병렬 로드맵은 만들지 않는다. CUDA 구현은 계속 계획에만 둔다. representation·주소·refcount는 비교 대상이 아니며, 다음 의미 투영과 실패 동작을 비교한다.

| 단계 | 비교 대상 | 검증 방법 | 현재 한계 |
|---|---|---|---|
| Tokenizer/F0 | raw word bytes, parser-visible comment cutoff, quote 오류 | C `;:`와 Rust raw spans 비교; 별도로 trailing `NB.`를 parse queue에서 제외 | 기존 256-byte sweep과 새 UTF-8 probe의 입력 범위를 구분한다 |
| Enqueuer/F1 | 품사, noun type/shape/data, primitive/name 구분, copula·lookup 플래그, 원 word index/span | literal/primitive를 C에 할당하여 값과 `4!:0` 비교; 이름/control 플래그는 `w.c::jtenqueue`에서 도출한 golden | C 내부 queue를 직접 export한 검증은 아니다. 전체 숫자 표기·locative·`_:`·env=0 미완료 |
| Parser/F2/P2/P3 | first-match row, completed modifier와 hook/fork의 ordered semantic operands, 최종 POS, construction error | 실제 source `p.c::cases[]`를 읽어 9⁴ 조합 비교; C `5!:1`을 의미 구조로 정규화; `4!:0`과 error class 비교 | `cases[]`는 tacit translator용 선언 테이블이다. runtime `ptcol`의 reachable state·reinsertion·effect/name sequencing 증명과는 별도다 |

`examples/frontend_probe.rs`는 backend-independent 관찰 adapter다. source operator가 DAG parent로 남고 noun operand는 type/shape/data와 boxed 구조를 보존한다. `tools/frontend_stage_conformance.py`는 각 단계의 검사 수·불일치·미지원 목록을 따로 보고한다. 알려진 미지원 문법을 성공으로 집계하거나 최종 값 일치만으로 parser 구조 동등성을 선언하지 않는다. source review revision과 실제 oracle DLL revision·hash도 별도로 기록한다.

실행 순서는 다음과 같다. 각 완료 표시는 아래 F/P 항목에만 적용한다.

1. F0 경계 검증을 유지하고 stage probe를 추가한다.
2. F1의 copula 환경부터 복원하고 literal/name/spelling 오류·전체 core primitive coverage를 넓힌다.
3. F2/P2의 선언 row 계약과 runtime dispatch·reduction extent를 따로 검증한다.
4. P3의 modifier/trains 구조와 result POS/construction errors를 C atomic representation으로 확장한다.
5. P4의 parser-time noun snapshot/late function lookup, 중간 assignment/locale/effect를 runtime semantic action과 검증한다.
6. P6에서 단계별 비교와 기존 값/error differential을 native Windows gate로 실행한 뒤 P7 cutover 완료를 판단한다. GitHub CI는 사용하지 않는다.

새 stage 검증 실행 예시(Windows, source checkout과 DLL revision은 실제 준비한 값을 사용):

```powershell
python tools/frontend_stage_conformance.py --binary target/windows-validation/debug/examples/frontend_probe.exe --source-directory target/jref --source-revision 13994ffa1ed5f06f79fad6e9822a7ed2d29b1528 --reference-revision ded7793fe5795d79eda8e7138dce94aa056edf78 --report reports/frontend-j64-stages-windows.json
```

이 명령은 `J_LIBRARY`에 실제 `j.dll`/`javx2.dll` 경로가 설정되어 있어야 한다. source pin이 다르거나 DLL이 다른 revision이면 보고서에 그 차이를 유지한다. Windows local runner에도 source-directory/source-revision을 지정하여 단계 검사를 함께 실행한다.

이번 source 기반 수정은 [w.c::jtenqueue](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/w.c), [sn.c::vnm](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sn.c), [wn.c::connum](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wn.c), [p.c](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c)를 참조했다. `foo_`는 locative 미지원이 아니라 ill-formed name이다. `1q`, `1e`, `1.2.3`, `3..`도 ill-formed number를 보존하며 미구현 숫자 표기·유효 locative는 Unsupported로 남긴다.

<a id="noun-reduction-capture"></a>

<a id="noun-reduction과-컴파일용-구조-보존--조사-및-구현-계획-2026-10-03"></a>
#### Noun reduction과 컴파일용 구조 보존 — 조사 및 구현 계획 (2026-10-03)

**결정:** runtime parser는 jsource처럼 verb application을 실행하여 실제 noun으로 reduce하고, 컴파일러는 별도 capture에서 생산 연산과 input/output 연결을 보존한다. noun이 된다는 이유로 provenance를 버리지 않는다. verb 중심의 tacit 표현은 구조를 노출하는 권장 방식이며 필수 언어 제한이 아니다. 이 절은 F2/P2–P6를 구체화하는 계획이고 별도 roadmap이나 두 번째 canonical IR을 만들지 않는다.

**현재 남은 차이:** runtime rows 0–2는 이제 host를 통해 실제 noun으로 reduce한다. static context는 연산 Expr를 보존한다. capture v0는 source operation과 occurrence 연결을 별도로 보존하며, 성공 capture를 기존 J Graph로 변환하는 adapter도 구현했다. top-level single-name non-final assignment는 구현했다. explicit-local/locale/definition/effect 및 전체 modifier POS는 계속 미완료다. 일반 fork executor 미지원도 frontend construction 지원과 구분한다.

<a id="다른-언어배열-프레임워크의-처리"></a>
##### 다른 언어·배열 프레임워크의 처리

아래 공식 문서/소스는 2026-10-03 확인했다. `main`/`stable`/nightly URL은 움직이는 참고 자료이며 RustJ 호환성 oracle revision을 대신하지 않는다. framework 동작과 RustJ 적용 판단을 구분한다.

| 사례 | 실제 처리 | RustJ에 참고할 요소와 한계 | 출처 |
|---|---|---|---|
| PyTorch `make_fx` / ProxyTensor | real tracing은 실제 tensor로 실행하며 operation graph도 수집한다. `proxy_call`은 proxy node를 만들고 실제 operation을 호출한 뒤 `track_tensor_tree`로 결과와 proxy를 연결한다. fake tracing도 별도 mode다 | 실제 noun과 graph reference를 별도로 보존하는 v0의 가장 가까운 사례. 다만 J modifier/train 의미는 tensor primitive tracing만으로 복원할 수 없으므로 parser construction identity를 함께 기록한다 | [make_fx API](https://docs.pytorch.org/docs/stable/generated/torch.fx.experimental.proxy_tensor.make_fx.html), [proxy_tensor.py source](https://github.com/pytorch/pytorch/blob/main/torch/fx/experimental/proxy_tensor.py) |
| PyTorch FX symbolic tracing / Dynamo | FX Proxy는 값을 대신해 연산을 기록하지만 input-dependent Python control flow에는 제한이 있다. Dynamo는 graph, residual code, validity guards를 만들고 unsupported 구간에서는 graph break 후 일반 실행을 이어간다 | graph가 모르는 noun 값·name/POS·effect를 static 성공으로 꾸미지 않는다. 경계와 재사용 조건을 명시한다. RustJ runtime에도 지원 범위가 있으므로 모든 미지원 J 문법을 처리하는 fallback이 있다고 가정하지 않는다 | [FX tracing limitations](https://docs.pytorch.org/docs/stable/fx.html), [Dynamo graph breaks/guards](https://docs.pytorch.org/docs/stable/user_guide/torch_compiler/compile/programming_model.dynamo_core_concepts.html) |
| JAX | tracer가 operation을 기록하여 jaxpr를 만든다. abstract tracer는 shape/dtype을 알지만 실제 data를 모른다. static/concrete 값과 traced 값의 경계를 구분하며 Python side effects가 일반 jaxpr에 자동으로 들어가지 않는다 | 이후 no-execution static 경로의 참고 모델. 실제 noun이 필요한 J constructor를 abstract shape/dtype만으로 처리하지 않는다. J observable effects를 trace 때 한 번 실행하고 compiled reuse에서 생략하는 정책은 채택하지 않는다 | [Tracing](https://docs.jax.dev/en/latest/tracing.html), [JIT and side effects](https://docs.jax.dev/en/latest/jit-compilation.html) |
| TensorFlow `tf.function` | tracing 때 Python은 실행하고 TensorFlow operations는 graph에 기록한다. AutoGraph가 지원 제어 흐름을 변환한다. Python effects와 TensorFlow runtime effects는 서로 다르다 | host/parser-time 작업과 graph runtime 작업의 staging 경계를 명시한다. J error/name/assignment 동작을 graph 밖에서 실행했다는 이유로 subsequent calls에서 누락시키지 않는다 | [tf.function tracing, AutoGraph, effects](https://www.tensorflow.org/guide/function) |
| ArrayFire / Eigen | ArrayFire는 지원 elementwise operations를 AST에 모으고 explicit `eval`이나 non-JIT consumer가 필요할 때 평가한다. Eigen은 expression templates와 alias/cost 규칙에 따라 평가를 지연하거나 temporary를 만든다 | pure-array fusion과 계산 경계의 참고 사례. 모든 J parser noun을 lazy array로 바꾸는 근거는 아니다. J가 요구하는 오류·효과 시점을 늦출 수 있는지는 별도 증명이 필요하다 | [ArrayFire JIT](https://arrayfire.org/docs/jit.htm), [Eigen lazy evaluation/aliasing](https://libeigen.gitlab.io/eigen/docs-nightly/TopicLazyEvaluation.html) |
| Julia compiler | compiler는 SSA-form IR에 instruction/result/control-flow 관계를 유지한다. 이는 ordinary runtime value가 생성 이력을 자동으로 갖는다는 의미가 아니다 | J 의미를 확보한 후 application 결과를 SSA value로 연결하는 후속 lowering의 참고 사례. SSA만 도입하면 동적 J parsing이 해결된다는 결론은 내리지 않는다 | [Julia SSA IR](https://docs.julialang.org/en/v1/devdocs/ssair/) |

**RustJ 적용 판단:** 위 사례에서 실행/값과 graph representation이 분리될 수 있다는 점을 취한다. parser-time 의미 보존은 [jsource p.c](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c)가 기준이다. pure-array lazy evaluation은 이후 증명된 구간에서 사용할 최적화이며, 이번 baseline parser의 replacement가 아니다.

<a id="값과-기록의-소유권"></a>
##### 값과 기록의 소유권

예: `a + b * c`에서 parser의 오른쪽 reduction은 `b * c`를 실행해 noun을 얻는다. 별도 기록은 다음 연결을 보존한다. `v1/v2`는 설명용 observation id이며 메모리 주소가 아니다.

```text
capture: v1 = Apply(*, b_read, c_read)   parser: actual noun result + origin(v1)
capture: v2 = Apply(+, a_read, v1)      parser: actual noun result + origin(v2)
```

- **Parser noun carrier:** 실제 `Value`와 optional capture origin을 가진다. J 품사는 계속 Noun이다. capture 여부가 class matching/POS/error를 바꾸지 않는다. recording-disabled carrier는 추가 array 보관·graph 할당을 하지 않는다.
- **별도 `CompilerCapture`(제안):** application occurrence별 input ids, shared `FunctionEntity`, monad/dyad valence, original spans/word indices, observed type/shape, sequence/effect dependency, success/failure를 기록한다. 구체적인 `Value` 전체를 모든 node에 복사하지 않는다. literal constants는 intentional immutable constant pool, external nouns는 input slots, 중간 결과는 ids와 facts로 표현한다. scalar 값이 실제 constructor 선택에 사용되었다면 그 의존성/guard를 명시한다.
- **동일 값과 동일 origin은 다르다:** `2*3`과 `1+5`가 모두 6이어도 다른 production occurrence다. 값 equality나 storage pointer로 node identity를 합치지 않는다. shared operand는 id를 공유할 수 있지만 compiler proof 없이 두 연산을 합치지 않는다. `CaptureValueId`는 기존 J Graph `ValueId`, J name/version, Physical `BufferId`와 구분하고 adapter에서 명시적으로 매핑한다.
- **Function construction:** rows 3–6의 `/`, `"`, hook/fork completed entity identity는 그대로 보존한다. computed noun을 constructor가 읽으면 실제 값으로 validation하고 capture에는 해당 noun-origin → constructor operand 연결을 별도로 기록한다. 원 operator와 operand 순서를 지우거나 일반 Reduce/Map으로 바꾸지 않는다. sample-dependent rank나 function specialization은 intrinsic function identity와 별개의 reuse witness다.
- **외부 이름·assignment:** noun은 실제 stack-entry lookup 시점의 값/version을 사용한다. function NameRef는 예상 POS와 late binding을 보존한다. named function 실행에서 관찰한 target은 observation/witness이지 무조건 상수화할 근거가 아니다. 이전 workspace 값의 생성 graph가 없으면 외부 입력으로 기록하며 과거 이력을 추측하지 않는다. 문장 간 capture는 명시적인 scope와 binding versions가 생긴 후 확장한다.
- **성공/오류/효과:** invoke 직전에 attempt/input edges를 기록하고, 성공 시 output origin을 연결한다. 실패 시 기존 J ErrorKind/ErrorContext를 그대로 반환하고 실패 node와 partial graph를 남길 수 있다. partial graph는 complete executable plan이 아니다. 이미 수행된 J-visible effects를 capture 실패 때문에 문장 전체 rollback하거나 재실행하지 않는다. 아직 성공하지 않은 바깥 assignment를 commit하지 않는 기존 의미를 보존한다. trace 내부 실패는 J 오류를 덮어쓰지 않고 capture 불완전 상태로 분리한다.
- **최적화 전달:** capture는 새 실행 IR이 아니라 기존 `j_graph_ir::Plan`을 생성/보강하는 입력 sidecar다. `logical_ir::Plan`이 canonical execution IR이라는 M1 원칙은 유지한다. J Graph verifier는 data edges와 effect/error sequencing을 검증한 후 기존 lowering으로 넘긴다. parser에는 target/device/schedule/fusion 결정을 넣지 않는다.

<a id="실행-경로와-재사용-경계"></a>
##### 실행 경로와 재사용 경계

1. **Reference semantic execution:** capture on/off 모두 같은 parser class matcher와 row actions를 사용한다. rows 0–2의 runtime action은 그 시점에 실제 noun을 생성한다. record 여부로 실행 횟수·name lookup·오류 시점이 달라지면 안 된다.
2. **Execute-and-capture:** 제안 API `Engine::eval_with_capture(&mut self, source)`는 사용자 문장을 한 번 실행하고 outcome + capture를 반환한다. 이는 read-only `prepare_semantic/analyze_j_graph(&self, ...)`와 구분한다. 실패를 기록하려면 outcome을 필드로 가진 report가 필요하며 outer `Result` 때문에 partial trace를 잃지 않도록 API를 정한다. 분석 요청을 명분으로 IO/assignment를 몰래 실행하거나 성공 trace를 만들기 위해 두 번 실행하지 않는다.
3. **Static compilation:** 기본 비실행 경로의 pure/static 범위는 같은 parser row engine의 abstract actions로 graph를 만들 수 있다. 실제 값이 필요한 constructor, unknown POS/binding, effects/error boundary는 typed dependency와 coverage reason으로 드러내며 현재 지원 runtime action/region을 이용한다. no-execution AOT는 unknown dependency를 명시적으로 거부하거나 residual runtime region으로 나타내고 compile-time 실행으로 해결하지 않는다.
4. **Captured graph reuse:** trace 한 번으로 모든 입력/branch가 표현되었다고 주장하지 않는다. observed shape·data-dependent constructor/POS·binding/environment assumptions를 constants/input dependencies/guards로 구분한다. reuse 전에 검증하거나 재capture/semantic execution으로 되돌린다. 재capture는 이미 일부 effects를 실행한 지점에서 문장 처음부터 다시 시작하는 방식으로 구현하지 않는다. v0 capture는 inspection에 한정하고 재사용 실행을 기본 제공하지 않는다.
5. **효율:** execute-and-capture는 첫 실행의 array 계산 비용을 없애지 않는다. v0는 trace arena + shared function references + small facts를 기본으로 하여 capture 때문에 모든 temporary가 살아남지 않게 한다. pure-array region reuse/JIT/fusion이 subsequent execution의 성능 단계다. CUDA 구현은 계속 유예한다.



## NAME/locale 선행 작업과 Conv1D 보류 체크리스트 (2026-10-21)

**우선순위 변경:** Conv1D/SPORES/adjoint Graph IR 통합은 NAME·locale 의미론이 정리될 때까지 보류한다. 기존 Conv1D symbolic AST 및 독립 프로토타입은 유지하되 실제 최적화 경로에 연결하지 않는다.

### NAME/locale — 선행
- [x] 실제 parser/runtime 경계 조사: `ParserNameBinding`에 `Noun`, `AbstractNoun`, `Function`, `KnownVerb`, `KnownModifier`가 존재한다.
- [x] direct/base locative 경로 조사: runtime parser host는 noun lookup을 지원하나 함수 참조는 명시적으로 Unsupported 처리한다.
- [x] `direct_read_binding`은 대상 locale 조회 후 `z` fallback을 시도한다. 이름의 출처·검색 경로를 별도 증거로 남겨야 한다.
- [ ] J locale 조회·함수 호출·동적 재바인딩의 원문 의미론과 기존 jsource oracle fixture를 대조하여 허용 범위 확정
- [x] parser host의 direct/base locative **function POS 조회** 지원: noun과 동일한 scope 탐색으로 함수 품사 분류 (2026-10-21, 실행 호출 미지원)
- [x] direct/base locative verb NAME의 **실행 시점 조회**를 `resolve_function_entity`와 verb indirection에 연결 (테스트 미실행)
- [ ] direct/base 함수 호출의 재바인딩·효과 순서·z fallback 및 modifier 경로 검증
- [ ] indirect locative의 noun/function 분기와 locale 생성/오류 순서 계약 확정
- [ ] local/global/base/named/z fallback에서 POS 및 NAME version/provenance 전달
- [ ] Graph IR `RequiresSpecialization` 해제 조건과 실행 가드·fallback 계약 확정
- [ ] 오류/순서/재바인딩 회귀 fixture 추가 (실행은 묶음 테스트 시)
- [ ] frontend → Graph → Logical IR 통합 검증 (실행은 묶음 테스트 시)

### Conv1D — NAME 완료 이후
- [x] NN-08 semantic identity 및 symbolic forward/input-adjoint/weight-adjoint view 프로토타입
- [x] Graph `ValueId`를 담는 비침습적 logical-view 준비 인터페이스
- [ ] NAME/locale에서 검증된 확장 primitive 바인딩 증거를 Graph에 보존
- [ ] shape/rank/effect/error/floating-point 합법성 및 재구성 조건
- [ ] 정식 Graph IR rewrite/SPORES와 A3 Logical IR lowering 연결
- [ ] 누적 테스트 일괄 실행, 실패 수정 및 검증 결과 기록

**실행 정책:** 사용자 요청에 따라 지금은 테스트를 실행하지 않고, 변경 사항과 검증 대상을 누적한다.

### 구현 기록: direct/base 함수 POS 조회 (2026-10-21)
`runtime.rs`의 parser host `lookup`에서 direct/base locative의 기존 함수 Unsupported 분기를 `ParserNameBinding::Function(function.result_pos)`로 변경했다. `direct_read_binding`의 기존 대상 locale→z fallback을 사용하며, noun 복사/lookup 경로는 그대로 유지한다. **이 변경은 파서 품사 판별만 허용한다.** 실제 `resolve_function_entity`는 여전히 `visible_binding` 경로를 사용하므로 locative 함수 호출의 의미론적 실행까지 완료됐다고 볼 수 없다. 테스트는 일괄 실행 시점까지 보류한다.


### 구현 기록: direct/base locative verb 실행 조회 (2026-10-21)

`runtime::Engine::visible_function_binding`을 추가했다. 직접 지정된 locale의 함수 NAME은 `direct_read_binding`을 통해 해당 locale→`z` fallback으로 조회하며, base locative는 base→`z` 순서다. 일반 NAME은 기존 `visible_binding`으로 처리한다. `resolve_function_entity` 및 verb indirection 경로에 연결했고, 최종 함수 NAME 존재 검사도 같은 조회로 맞췄다. **호출 시점 재조회**를 수행하므로 파서의 POS 관찰 결과를 고정된 함수 identity로 사용하지 않는다.

**미완료:** indirect locative 함수, modifier 적용 경로 전체, locale 함수 할당, path 일반화, 동적 바인딩 오류/순서 oracle 검증. 특히 `visible_function_binding`은 indirect locative를 `None`으로 돌려 현재 실행을 명시적으로 허용하지 않는다. 이 변경은 실제 Rust 테스트를 실행하지 않았으며 통과를 주장하지 않는다.


### 구현 기록: 함수 간접 참조 및 modifier 조회 경로 확장 (2026-10-21)

`runtime.rs`의 기존 `visible_function_binding`을 다음 함수 NAME 조회 경로로 확장했다: explicit operator probe, primitive witness, composite function call, parser host의 `stacked_modifier`. 각 경로는 direct/base locative의 own-locale→z 검색을 사용하고 일반 NAME은 기존 조회로 처리한다. `function_name_ranks`는 여전히 locative를 보수적으로 unknown 처리하며, `fork_cap_binding`과 gerund lookup 등은 별도 의미론 감사가 필요하다.

**범위 주의:** 이 변경은 lookup 통일이지 모든 modifier/locale 호출의 완성 선언이 아니다. locale 함수 할당과 indirect function lookup은 여전히 지원하지 않는다. 컴파일·테스트·jsource oracle은 실행하지 않았다.


### 구현 기록: gerund·fork cap NAME 조회 (2026-10-21)

Parser host `gerund_binding`에서 direct/base locative의 현재 바인딩을 `visible_function_binding`으로 분류하도록 연결했다. 일반 NAME은 기존 `parser_name_binding`을 유지하여 extension 후보 처리 정책을 바꾸지 않는다. `fork_cap_binding` 역시 locale-aware 함수 조회를 사용한다. 이 변경은 **lookup/POS 전달 범위**이며 gerund의 locative 실행·abandon·path 일반화를 보증하지 않는다.

**중요한 미해결 설계:** `commit_runtime_binding`은 direct/base/indirect locative에서 함수 할당을 명시적으로 거부한다. 이를 단순히 허용하도록 변경하기 전에 parser assignment transport, `JEntity` 소유권, `z` fallback의 쓰기 금지, locale 생성 순서, dynamic NAME resolution, jsource oracle을 함께 확인해야 한다. 현재는 의도적으로 기존 Unsupported를 유지한다. 테스트는 일괄 실행까지 보류한다.


### 구현 기록: direct/base locative 함수 할당 허용 (2026-10-21)

`runtime::Engine::commit_runtime_binding`에서 direct/base locative의 함수 할당을 막던 `JEntity::Noun` 제한을 제거했다. 기존 `store_binding`은 `JEntity`를 저장하므로 noun/function 모두 동일한 저장 경로를 사용한다. `f_loc_`은 지정 locale 자체에 기록하고, `f__`은 base에 기록한다. 읽기에서만 적용하는 `z` fallback을 쓰기에는 적용하지 않는다. 간접 locative의 함수 할당은 아직 허용하지 않는다.

**검증 전제:** locale 생성 시점, J parser assignment transport, adverb/conjunction RHS, 재정의 후 POS 변경, `z` shadowing, 오류 순서 및 jsource oracle과의 일치를 검증하기 전에는 완성으로 간주하지 않는다. 테스트·컴파일은 사용자 요청에 따라 실행하지 않았다.


### NAME 재정의·shadowing 감사 및 누적 검증 행렬 (2026-10-21)

코드 확인: `store_binding`은 동일 locale 심벌 테이블의 이전 바인딩 버전을 증가시키고, 새 `BindingGeneration`을 생성하며, noun→function 또는 function→noun 변경을 같은 `JEntity` 저장 경로로 처리한다. `direct_read_binding`은 **own binding 우선**이므로 own noun이 있으면 `z` function으로 재검색하지 않는다. `visible_function_binding`은 direct/base에 대해 local frame을 우회하며, 일반 NAME에 대해서만 기존 `visible_binding`을 사용한다. 현재 `visible_binding`의 일반 NAME `z` fallback은 noun으로 제한되므로 이를 locative 함수 조회의 정책과 혼동하지 않는다.

| 검증 항목 | 기대 계약 | 상태 |
| --- | --- | --- |
| `f_loc_`에 verb 할당 후 호출 | locale own 함수로 조회, 실제 호출 경로까지 확인 | 미검증 |
| 같은 locale에서 verb→noun 변경 | `z` verb로 fallback하지 않고 POS/domain 경계 유지 | 미검증 |
| 같은 locale에서 noun→verb 변경 | 재파싱/재호출 시 새 POS 및 generation 반영 | 미검증 |
| `z` verb만 존재하는 `f_loc_` | 대상 locale에 own이 없을 때만 fallback | 미검증 |
| `f__`과 일반 `f`의 local shadowing | base locative는 local frame을 우회 | 미검증 |
| explicit definition·derived modifier 안의 재바인딩 | 실행 시점 NAME 재조회, effect/error 순서 유지 | 미검증 |
| `f_loc_` 할당에서 대상 locale 미존재 | locale 생성과 오류 순서가 jsource와 일치 | 미검증 |
| direct/base modifier 할당 | adverb/conjunction RHS의 parser transport와 runtime 적용 확인 | 미검증 |
| indirect locative 함수 할당 | 아직 Unsupported 유지; 지원 범위 확정 전 성공으로 처리하지 않음 | 미구현 |

**검증 정책:** 위 항목은 테스트를 실행하지 않은 코드 감사 결과에서 도출한 예상 계약이다. J 정답(특히 POS 변화와 locale 생성 순서)은 jsource oracle과 대조하여 확정한다. 실패 또는 차이가 발견되면 기존 구현을 재검토한다. Conv1D Graph specialization은 이 검증을 통과할 때까지 보류한다.


### Direct definition 지역변수 구현 확인 (2026-10-21)

`src/definition_input.rs`는 `DefinitionForm::Direct` (`{{ ... }}`)를 별도로 인식한다. `src/definition_code.rs`는 direct body를 공통 definition code로 정규화하고 `DefinitionScopePlan.local_declarations`에 선택 valence의 literal simple-name `=.` 대입 대상을 기록한다. `src/runtime.rs::invoke_definition_body`는 호출마다 새 `LocalFrame`을 생성하고 해당 선언을 `declared`에 반영하며, `x/y` 및 modifier operand `u/v/m/n`을 호출 프레임에 바인딩한다. 즉 **direct definition의 기본 지역변수 지원은 이미 구현**되어 있다.

- [x] Direct definition과 explicit definition의 공통 지역변수 실행 경로 코드 감사
- [ ] `{{ t=.y+1 [ t }}` 등 direct definition 지역 대입·읽기·호출 격리 회귀 작성
- [ ] Direct definition 안의 지역 함수 NAME 재정의 및 전역 함수 shadowing 검사
- [ ] Direct definition 안의 `f__`/`f_loc_`가 지역 프레임을 우회하는지 검사
- [ ] 재귀·중첩 direct definition 및 monad/dyad 선택 시 scope lifetime 검사
- [ ] 위 회귀를 jsource oracle과 대조하고 일괄 테스트 실행

주의: `local_declarations`는 literal simple-name 대입을 다루며 computed/noun assignment target 전체를 증명하지 않는다. 미바인딩 지역 이름의 global fallback과 호출 프레임의 시점별 조회는 별도 검증 대상이다. 테스트는 아직 실행하지 않았다.


### Direct definition NAME scope 추가 검증: 기존 oracle 기록 재감사 (2026-10-21)

`reports/definition-name-scopes-windows.json`의 과거 Windows differential 기록을 확인했다. `fixture_set=name-scopes`, 10개 사례 × 4개 관측 = 40건이며 기록된 `counts.matched=40`이다. 사례: `explicit_local_fallback`, `direct_local_fallback`, `explicit_local_function_escape`, `direct_local_function_escape`, `explicit_global_write`, `direct_global_write`, `explicit_local_global_collision`, `direct_local_global_collision`, `top_level_local_copula_is_global`, `callee_does_not_capture_caller_local`. 관측은 당시 binary/source revision에 한정된다. **현재 HEAD 테스트를 실행하거나 재검증한 것이 아니다.**

추가 코드 감사: `definition_code::DefinitionScopePlan`이 literal `=.` 선언을 수집하고, `runtime::invoke_definition_body`가 valence별 선언 및 `x/y/u/v/m/n`을 새 `LocalFrame`에 설치한다. `store_binding`은 function과 noun을 모두 저장한다. 따라서 지역 함수 저장·읽기·전역 충돌에 대한 기본 실행 경로는 존재한다. 그러나 direct/base locative 함수 할당을 최근 허용한 변경과의 상호작용, 재귀/중첩 scope 및 재정의 후 POS 변경은 과거 보고서만으로 보증할 수 없다.

- [x] 과거 direct definition 지역변수·충돌·함수 escape differential 기록 확인 (과거 40/40, 현 HEAD 미검증)
- [ ] 현 HEAD에서 과거 10개 fixture 재실행
- [ ] 지역 함수 verb→noun 및 noun→verb 재정의 회귀 추가
- [ ] 지역 함수와 `f__`/`f_loc_` 충돌 및 `z` fallback 회귀 추가
- [ ] 중첩·재귀 direct definition의 지역 프레임 격리 및 오류 복구 회귀 추가

테스트 실행은 사용자 지정 일괄 테스트 단계까지 보류한다.


### 누적 NAME 회귀 코드 추가 (2026-10-21)

`src/runtime.rs::name_locale_regression_tests`에 실행 보류 상태의 테스트 3개를 추가했다: (1) direct locale 함수 쓰기·z fallback·own noun shadow·함수 재정의, (2) base locative가 local frame을 우회하고 재정의 후 noun으로 바뀌는지, (3) direct definition `{{ ... }}`의 `local=.`이 호출 이후 전역 값을 바꾸지 않는지. **테스트 작성은 검증 통과가 아니다.** 현 HEAD 컴파일 및 테스트는 실행하지 않았다.

잔여: 과거 name-scopes 10개 fixture 재실행, direct definition 지역 함수 POS 변화, nested/recursive 프레임, locale modifier, 실제 호출 시점 오류 순서 및 jsource oracle.


### 추가 회귀 대기열: 중첩·재귀 direct definition 및 지역 함수 POS (2026-10-22)

`tests/definition_acceptance.rs`에 기존 `define/expect` 검증 패턴을 재사용하여 아래 네 가지 **정적 기대 결과**를 가진 회귀 테스트를 추가했다. 현재 HEAD에서 컴파일·실행·jsource 대조는 하지 않았으며 성공으로 집계하지 않는다.

- [x] **중첩 direct definition 호출 프레임:** 바깥/안쪽 모두 `slot=.`을 쓰되, 안쪽 호출 후 바깥 `slot`과 전역 `slot`이 보존되는지 확인하는 테스트 작성; 중첩 정의의 지역 함수 `inner`가 호출 후 밖으로 노출되지 않는지도 확인
- [x] **재귀 direct definition 호출 프레임:** 각 재귀 깊이에서 `keep=.`을 바인딩하고 복귀 후 caller의 값 및 전역 `keep`이 보존되는지 확인하는 테스트 작성
- [x] **동일 지역 함수의 품사 재정의:** `verb→noun` 및 `noun→verb`를 별도의 direct definition에서 확인하고 전역 함수가 변경되지 않는지 확인하는 테스트 작성
- [x] `tools/name_compatibility_audit.py`의 SCOPE/NESTED fixture 그룹에 총 5개 J-vs-Rust 비교 사례 추가 (두 POS 방향 분리, 중첩 오류 복구 포함)
- [ ] 현재 HEAD에서 Rust 테스트 및 J oracle을 실행하고 각 기대 결과 검증
- [x] 중첩 호출 오류 후 전역 조회·지역 함수 비누출·후속 호출을 검사하는 회귀 및 oracle fixture 작성 (실행 보류)
- [ ] 오류/예외 복구 시 caller 지역변수 생존과 frame pop 순서의 실제 테스트·jsource 일치 확인
- [ ] 지역 함수와 direct/base/named locale NAME 충돌·수정 시점·재귀 재바인딩 오류 순서 검사
- [ ] direct definition의 monad/dyad 및 modifier `u/v/m/n`과 지역 이름의 shadowing 확장 검사

**주의:** 이 시점에는 테스트 준비와 검증 실행을 분리한다. 과거 oracle 결과와 신규 테스트의 결과를 합산하거나, 재귀·중첩 및 POS 변경의 호환성이 증명되었다고 표현하지 않는다.


### NAME/locale 검증 대기열 확장: direct 호출·재귀 재바인딩·modifier operand (2026-10-22)

이전 중첩·재귀 지역 프레임 테스트에 이어, 실제 parser→runtime NAME 호출 및 modifier 호출 경계를 겨냥한 검증을 **작성만** 했다. 현재 HEAD의 컴파일·Rust 테스트·J oracle·CI는 실행하지 않았다.

- [x] `tests/definition_acceptance.rs`: direct definition 내부의 local `op=.-`와 base `op__` 및 named `op_zone_` 함수가 독립적으로 조회되는지 검사하는 테스트 추가. 함수 정의 바깥에서 base/named 함수가 변경되지 않는지도 검사
- [x] `tests/definition_acceptance.rs`: 재귀 호출 안쪽에서 local `op`가 plus로 재바인딩되어도 바깥 프레임의 minus 및 global plus가 보존되는지 검사하는 테스트 추가
- [x] `tests/explicit_modifier.rs`: direct adverb `{{u+m}}` 및 direct conjunction `{{u+v+m+n}}`의 implicit operand/alias가 전역 `u/m/v/n`보다 우선하여 조회되지만 전역 바인딩의 값·버전은 변경하지 않는지 검사하는 테스트 추가
- [x] `tests/base_locative.rs`: 과거 function locative assignment를 무조건 Unsupported로 기대한 테스트를 런타임 write/call 및 정적 분석 제한으로 분리하여 최신 코드 계약에 맞춤
- [x] `tests/base_locative.rs`: named function의 `z` fallback→own noun shadow→own verb 재정의 동작을 parser 호출 경로에서 확인하는 테스트 추가
- [x] `tools/name_compatibility_audit.py`의 SCOPE fixture에 위 기능과 연관된 J-vs-Rust 비교 사례 4개 추가
- [ ] 현재 HEAD 컴파일·Rust 테스트·`--scope-fixtures-only` oracle 비교 실행, 실패 사례 분류·수정
- [ ] direct/base/named 함수의 실행 시점 재정의와 RHS 평가 효과·오류 순서 검증
- [ ] direct modifier의 noun/function operand 교체와 재귀·중첩 정의 구간에서 `u/v/m/n` 이름 충돌 검증
- [ ] 간접 locative function 지원 여부, scope stamp/guard 및 NAME provenance 검토 후 Conv1D 통합 재개

**검증 상태는 모두 미실행이며, 체크 표시([x])는 사례·코드 작성 완료만 뜻한다.** 기존 Windows oracle 보고서와 이번 HEAD 신규 사례의 통과 횟수를 합산하지 않는다.


### NAME 함수 재바인딩·첫 오류 순서 및 기존 회귀 기대값 정리 (2026-10-22)

직전 direct definition/local·locale fixture에 이어 **일반 J 문장 안의 함수 NAME 조회 시점**과 **실패한 RHS의 비커밋 경계**를 검증 대기열로 추가했다.

- [x] `tests/base_locative.rs`: `op_probe_ 4 + change 0`과 `change 0 + op_probe_ 4`를 대비하는 NAME 재바인딩 우측→좌측 평가 순서 회귀 작성. `change`는 정의 본문에서 named locale의 verb를 재할당하고 noun 인수를 반환
- [x] `tests/base_locative.rs`: function target에 length-error RHS를 대입하면 기존 함수가 유지되는지, 실패하는 RHS **안에서 이미 완료된 global counter 변경**은 유지되는지 검증하는 회귀 작성
- [x] `src/runtime.rs::name_locale_regression_tests`: 실패한 named function assignment가 기존 `NameVersion` 및 `BindingGeneration`을 보존하고, 뒤이은 정상 재정의는 둘을 갱신하는 회귀 작성
- [x] `tests/named_locale.rs`: 이전 noun-only 단계의 `a_probe_=:+` 및 `a_z_=:+` Unsupported 기대를 현재 함수 저장 허용 정책에 맞게 수정. 실패 RHS 후 기존 noun의 조회 관측이 그대로인지 검사하는 기존 단언은 유지
- [x] `tools/name_compatibility_audit.py`: named function 문장 내 재바인딩 순서, 함수 RHS 실패 및 side effect 순서에 관한 J oracle 비교 fixture 2개 추가
- [x] `docs/project/chapters/04-semantic-array-ir-p03.ko.md`: 과거 direct/base function lookup Unsupported라고 기술했던 낡은 구현 상태를 현재 범위에 맞게 갱신
- [ ] **현 HEAD 검사:** Rust fmt/compile/tests 및 named scope J oracle 실행 후 실패·성공·미지원 분류 (현재 사용자 지시에 따라 미실행)
- [ ] **확장 검증:** right-to-left에서 재바인딩과 parser POS 변경이 겹칠 때의 domain/lookup 오류 우선순위; indirect function locative와 locale/path 생성 실패
- [ ] **컴파일러 gate:** 이러한 NAME/effect/error 조건을 Graph specialization에 사용하려면 generation·locale provenance·effect boundary가 실행 시 검증되는 추가 증거 필요

**주의:** 새 단언은 결과의 **예상 계약**이지 검증 결과가 아니다. 지금은 코드를 추가했을 뿐, 현 HEAD 및 C oracle 결과의 성공 건수는 갱신하지 않는다.


### NAME 동일 문장 POS 변경·locale 조회 생성/오류 순서 (2026-10-22)

**이번 범위:** 이미 정의된 verb NAME을 RHS가 noun으로 바꾸거나 다시 verb로 되돌릴 때, J 파서의 품사 인지 시점과 런타임 함수 조회 시점이 달라져도 오류 우선순위·바인딩 보존이 일치해야 한다. 이름의 단순 문자 유사성이나 Graph 정규화 결과만으로 함수 identity를 고정하지 않는다.

- [x] `tests/base_locative.rs::named_function_pos_rebinding_observes_right_to_left_call_order` 작성: `call 4 + nounify 0`, `call 4 + restore 0`, `restore 0 + call 4`에서 RHS 변경·호출 시점·첫 오류를 분리한다.
- [x] `src/runtime.rs::name_locale_regression_tests::named_locale_creation_occurs_at_lookup_or_successful_commit_not_failing_rhs` 작성: 실패한 named 조회의 locale 생성과, 우측 RHS 오류로 앞의 조회·할당에 도달하지 않았을 때 locale 미생성을 분리한다.
- [x] `tests/named_locale.rs::malformed_indirect_holder_error_follows_successful_rhs_only` 작성: 유효하지 않은 boxed holder의 해석 오류보다 실패한 RHS가 먼저인지, 정상 RHS로 도달했을 때 목적지 기존 바인딩이 유지되는지 검사한다.
- [x] `tools/name_compatibility_audit.py`의 SCOPE 2개·ASSIGNMENT 1개 J differential fixture 추가. fixture는 J-C 기대 출력의 사전 결정이 아니라 실행 때 **차이를 관측**하는 용도다.
- [ ] 현재 HEAD에서 새 Rust 회귀의 실제 컴파일·실행 및 J-C oracle 대조.
- [ ] J-C 첫 오류가 `domain/length/ill-formed name` 등의 기대와 다른 경우, 테스트를 임의로 완화하지 말고 NAME 파싱·resolution 시점과 J oracle의 출처를 비교해 원인 규명.
- [ ] 성공적 함수 재정의 뒤의 parser POS, modifier POS 변경, locale 초기 생성·local/path/holder 재검색을 포함한 실행 가드와 Graph specialization 증거 확정.

**실행 상태:** 이번 항목의 `cargo fmt`, `clippy`, Rust default/portable, Python audit, J-C differential은 **UNRUN**이다(사용자 요청에 따른 일괄 테스트 보류). 소스·보고서의 과거 PASS를 새 HEAD의 PASS로 옮기지 않는다. `main` push에서 자동 시작되는 원격 워크플로가 있더라도 수동 테스트 실행이나 결과 확인과 구별한다. 이 체크리스트의 체크([x])는 **테스트 코드/fixture 작성만 완료**된 뜻이다.


### NAME POS 재바인딩 자동 CI 차이 기록 (2026-10-22)

- **관측 리비전:** `dc0c205f307c9e0295eac2a3d984b0297a3a29c8`, 자동 push-triggered `Basis compile probe` [run 39730634513](https://github.com/yunskim/RustJ/actions/runs/39730634513). `cargo test` 중 `tests/base_locative.rs`의 `named_function_pos_rebinding_observes_right_to_left_call_order`가 **FAIL**: `tests/base_locative.rs:146`에서 실제 `syntax error`, 테스트 가정 `domain error`. 같은 테스트 파일의 앞선 8개는 그 자동 실행에서 PASS. 이 로그는 전체 HEAD 검증이 아니다.
- **진단:** 현재 기록으로는 J 원본도 `syntax error`를 내는지 확인되지 않았다. RustJ 오류 분류를 임의 변경하거나 J 동작이라고 주장하지 않는다. POS의 **파싱 시점 분류**와 함수 NAME의 **호출 시점 재조회**가 충돌하는 경계로 분류한다.
- **격리:** 해당 테스트의 기대값을 바꾸지 않고 `#[ignore = "..."]`를 추가했다(`74b1631`). 이는 CI의 다른 회귀를 관찰하기 위한 **미해결 계약 격리**이며 통과(PASS)가 아니다.
- [ ] 고정 버전 J-C 양 변형에서 `named_function_pos_rebinding_first_error` fixture를 실행하여 첫 오류와 후속 바인딩 상태 확인.
- [ ] RustJ 파서 body 재파싱과 `resolve_function_entity` 호출 경로를 구분해 `syntax`/ `domain` 첫 오류 차이의 원인을 규명.
- [ ] J와 일치하도록 필요하면 런타임/파서 또는 **정확한 기대값**을 수정한 뒤 ignore 해제. 오류 종류를 확인하지 않고 단언을 `is_err()`로 완화하지 않는다.

**실행 정책 상태:** 수동 테스트·CI는 시작하지 않았다. GitHub push가 자동 트리거한 원격 CI 결과를 **관측**했으며, 이 항목은 현재 FAIL/원본 미검증으로 추적한다. 다른 fmt/clippy/portable/J differential의 PASS는 주장하지 않는다.


### 누적 회귀의 자료형 오라클 수정 (2026-10-22)

자동 push-triggered `Basis compile probe`의 [run 39730895158](https://github.com/yunskim/RustJ/actions/runs/39730895158)에서 `tests/definition_acceptance.rs::direct_local_function_does_not_shadow_base_or_named_locale_functions`의 첫 비교가 FAIL이었다. 관측된 RustJ 결과는 `{"type":4,"shape":[],"data":[1]}`(Int scalar)이고, 테스트가 `expect(..., "1")`로 생성한 기준은 `{"type":1,"shape":[],"data":[1]}`(Bool scalar)이었다. 두 값은 숫자상 1이지만 J 자료형은 다르므로 검사가 정확하게 차이를 포착했다.

- [x] 비교 검사의 JSON type/value/shape 엄격성을 유지하면서 예상값 표현을 `_1+2`(Int 산술 결과)로 변경 (`tests/definition_acceptance.rs`, `32fa8f1`).
- [ ] 수정 HEAD에 대한 실제 CI 결과 확인. 이전 실패 run은 변경 이전 SHA이며 새 HEAD의 PASS를 의미하지 않는다.
- [ ] POS-change 첫 오류 `syntax` vs `domain`의 J 원본 대조는 별개 미해결 항목으로 유지한다.

**주의:** 위 숫자 비교 오류 수정은 NAME 기능의 J 의미론을 새로 검증한 것은 아니다. 자동 CI 로그를 읽었을 뿐 수동 테스트 실행/재실행은 하지 않았다.


### NAME POS 재정의 오류 — alias와 direct body의 독립 계약 (2026-10-22)

**근거 재감사:** 고정된 과거 J 비교 기록 `reports/name-system-research-windows.json`과 `reports/name-compatibility-windows.json`에서 `f=:+`, `g=:f`, `f=:7`, `g 3`은 J `j.dll` 및 `javx2.dll` 모두 **domain error**였으며, 당시 RustJ 양 실행 경로도 일치했다. 이는 생성된 **함수 alias가 기대한 verb POS를 유지**하고 호출 시 실제 target의 POS가 달라진 경우를 다룬다. 이 기록으로 `call=:{{op_probe_ y}}` 같은 **direct definition 내부의 이름 재파싱**까지 domain error라 단정할 수 없다. J의 공개 오류 설명에서 잘못된 문장 품사 배열은 syntax error의 한 원인이다. 원본 설명: https://www.jsoftware.com/help/jforc/error_messages.htm . 그러나 그 일반 규칙은 아래 신규 fixture에 대한 실제 원본 실행 증거를 대체하지 않는다.

- [x] `tests/base_locative.rs::named_locative_alias_retains_verb_pos_and_rejects_noun_rebinding`: named-locative 함수 alias를 만든 뒤 target을 noun으로 바꾸면 `domain error`, 다시 verb로 바꾸면 후속 호출은 새 함수를 사용하는 독립 테스트 작성.
- [x] `tests/base_locative.rs::direct_definition_reparses_named_locative_after_pos_restoration`: direct definition을 만든 뒤 이름을 noun으로 바꾸고 다시 verb로 돌렸을 때 본문의 정상 호출이 회복되는지 별도 활성 테스트 작성.
- [x] `tools/name_compatibility_audit.py`: 위 두 범주를 분리한 J reference fixture `named_locative_alias_pos_mismatch`, `named_direct_definition_reparse_after_pos_restore` 추가.
- [ ] `named_function_pos_rebinding_observes_right_to_left_call_order`의 첫 실패(`syntax error` actual / `domain error` test expected)는 **원본 동일 문장 결과를 확인할 때까지 ignored**. 현재 재분류만으로 RustJ parser/error mapper를 바꾸거나 ignore를 해제하지 않는다.
- [ ] 현 HEAD 기본/portable Rust 테스트 및 고정 C j64/AVX2의 두 fixture + `named_function_pos_rebinding_first_error` 실제 실행. 파서 NAME lookup 시점과 alias FunctionEntity의 런타임 lookup 경로를 별도 로그로 대조.
- [ ] 같은 문장 내 `nounify` 효과가 먼저 반영된 후 좌측 direct-body가 parse되는지 및 첫 syntax/domain error가 C와 같은지 확인. 시점이 다를 경우 별도 의미론 수정과 재발 방지 사례 필요.

**실행 상태:** 이번 변경은 **추가 검증 코드 작성 및 과거 oracle 결과 분석**이며, 신규 Rust default/portable·Python·C oracle 검증은 UNRUN. 기존 alias 2종 DLL의 domain error 결과를 신규 direct-body 사례의 PASS라고 보고하지 않는다. 자동 push CI도 현 HEAD 증거를 확인한 다음에만 판단한다.

**실행 경로 코드 감사:** `src/runtime.rs::invoke_definition_body`는 각 실행된 body sentence에 대해 `parser::parse_runtime_source(body_origin.slice(...), &mut frame, None)`를 호출한다. 따라서 RustJ direct definition의 본문 NAME은 외부 함수 별칭을 생성할 때처럼 모두 선행 고정되는 것이 아니라 **호출 중 해당 문장을 처리할 때 조회·POS 분류**된다. `EngineParserHost::lookup`은 direct locative의 현재 noun을 `ParserNameBinding::Noun`으로, 현재 함수를 `ParserNameBinding::Function(result_pos)`로 전달한다. `resolve_function_entity`의 동적 domain-error 확인은 이미 구성된 **FunctionEntity::NameRef**의 호출 시점 조건이다. 이 두 경로를 하나의 오류 종류 계약으로 합치지 않는다.

**관측된 별도 baseline 장애:** push-triggered `Linux milestone` [run 39731149300](https://github.com/yunskim/RustJ/actions/runs/39731149300)의 `check` job은 `cargo fmt --check`에서 FAIL이었다. 로그에 `src/conv1d_algebraic_view.rs` 및 `tests/base_locative.rs` 등의 포맷 차이가 보고되었다. 이 실패는 **Rust 수치/NAME 테스트 결과와 별도**이며, 현 HEAD의 fmt PASS를 보장하지 않는다. 사용자 지시상 이 세션에서는 formatter·테스트·CI를 재실행하지 않았고, fmt 수정도 별도 일괄 정비 대상으로 남긴다.



### CI rustfmt 차이의 비실행 적용 (2026-10-22)

자동 push-triggered [Linux milestone run 39732129543](https://github.com/yunskim/RustJ/actions/runs/39732129543)의 `check` job이 `cargo fmt --check`에서 실패했고, 로그의 **35개 diff 구간**을 해당 리비전 `20c5a2a`의 파일 원문과 문자 단위로 대조했다. 변경 대상은 `src/conv1d_algebraic_view.rs`, `src/lib.rs`, `src/runtime.rs`, `tests/base_locative.rs`이다. 각 구간은 파일 내용에서 **정확히 한 번 일치**하고 구간끼리 겹치지 않아, CI가 제시한 rustfmt 변경만 소스에 반영했다. 연산 의미나 이름 조회 로직은 이 변경에서 수정하지 않는다.

- [x] CI 로그에 기록된 4개 파일의 서식 차이 35개 구간 적용 및 한국어 정본 체크리스트 동기화
- [ ] 현 HEAD `cargo fmt --check` 확인 — **UNRUN/자동 CI 결과 대기**
- [ ] 현 HEAD Rust default/portable, clippy, Python, pinned J differential 확인 — **수동 실행 보류**
- [ ] `named_function_pos_rebinding_observes_right_to_left_call_order`의 J 원본 `syntax/domain` 첫 오류 oracle 확인 후 ignore 해제 여부 결정

**주의:** 이전 run의 실제 format 실패를 해결하기 위한 **코드 변경**이지, 새 HEAD에서 format PASS를 관측했다는 뜻이 아니다. Push로 자동 CI가 시작되더라도 수동 재실행과 구분하여 정확한 SHA의 결과만 집계한다.


### NAME 첫 오류 진단용 fixture 선택 및 격리 범위 축소 (2026-10-22)

**확인된 과거 CI:** `2434d2e7f78077a6611a62db35e456def4bc4b71`의 push-triggered [Basis compile probe 39732789489](https://github.com/yunskim/RustJ/actions/runs/39732789489)는 성공. 같은 SHA의 [Linux milestone 39732789571](https://github.com/yunskim/RustJ/actions/runs/39732789571)은 확인 당시 `check` 및 4개 reference matrix job이 모두 성공한 것으로 관측되었다. 다만 이 workflow의 reference matrix는 일반 conformance/rank/file-I/O 계열로, `tools/name_compatibility_audit.py`의 신규 NAME scope differential을 실행하는 증거가 **아니다**.

- [x] `tools/name_compatibility_audit.py`에 **반복 가능한 `--case NAME` 선택 기능** 추가: 기존 `--scope-fixtures-only` 등 그룹 범위 안에서 요청한 실제 fixture만 실행, 고정된 원래 순서 유지, 중복 요청 중복 실행 방지, 이름 미존재·중복 정의 시 즉시 오류
- [x] 진단 부분집합의 JSON 보고서/표준출력에 `selection.partial=true` 및 선택한 case 목록 기록. 선택 없이 실행하는 기존 전체 그룹의 결과·형식은 유지. 부분집합 성공을 전체 범위 PASS로 집계하지 않는다
- [x] `tools/test_name_compatibility_audit.py`에 선택/그룹 순서/중복/알 수 없는 이름 거부 및 격리된 NAME fixture 선택 여부 회귀 코드 작성
- [x] `tests/base_locative.rs::right_hand_pos_restoration_allows_later_left_hand_direct_call`의 **오른쪽 함수 restore → 왼쪽 direct body 정상 호출** 경로를 무시된 첫 오류 테스트에서 분리하여 활성 회귀로 추가
- [ ] 아래 세 개의 direct/alias fixture를 고정 J j64/j64avx2와 RustJ 양 경로에서 대조하고, 실제 `domain/syntax` 오류 및 후속 함수 상태의 결과를 확인
- [ ] 일치한 정확한 오류 종류를 근거로 `named_function_pos_rebinding_observes_right_to_left_call_order`의 ignore 해제 또는 parser/runtime 수정·새 실패 회귀 보강
- [ ] Python unit, Rust 기본/portable 및 NAME differential은 **수동 실행 보류(UNRUN)**. 푸시에 따른 CI 자동 실행 결과는 해당 SHA에 대해서만 별도로 보고

일괄 검사 시 **해당 fixture만 고르는 명령 예시** (Linux, pinned source build가 `.reference`에 준비돼 있고 `target/debug/rustj` 실행 파일이 존재할 때):

```sh
python3 tools/name_compatibility_audit.py \
  --reference-root .reference \
  --binary target/debug/rustj \
  --rust-revision "$(git rev-parse HEAD)" \
  --report reports/name-pos-targeted.json \
  --scope-fixtures-only \
  --case named_locative_alias_pos_mismatch \
  --case named_direct_definition_reparse_after_pos_restore \
  --case named_function_pos_rebinding_first_error
```

이 명령은 **제안된 미래 실행 방법**이며 이번 대화에서 실행하지 않았다. 특히 기존 C 자료의 alias POS mismatch가 domain error라는 결과는 direct body 문장에 대한 실제 C 결과를 대신하지 않는다.


### NAME differential 엄격 검증 모드 및 정상화된 HEAD CI (2026-10-22)

이전 진단 전용 `tools/name_compatibility_audit.py`는 `semantic_mismatch`, `unsupported_gap`, `observation_gap`을 JSON으로 보고하더라도 프로세스 종료 코드는 **0**이었다. 이 동작은 고장 목록을 쌓는 데는 적합하지만 CI에서 의미론 통과로 오인될 수 있다. **명시적인 `--gate`를 도입**해 실행 목적에 따라 종료 상태를 분리한다.

- [x] `--gate`가 없는 기존 진단 모드는 유지: 결과가 불일치해도 정상적으로 전체 보고서를 기록하고 0 반환. `counts`와 `records`의 의미를 바꾸지 않음.
- [x] `--gate`는 `matched` 외의 어떤 상태(`unsupported_gap`, `semantic_mismatch`, `observation_gap`)도 PASS로 계산하지 않음. 관측 0건과 고정 C 원본의 기대 j64/j64avx2 변형 중 누락도 FAIL.
- [x] `--gate` 실행은 JSON의 `gate.scope`, `gate.passed`, `gate.reasons`에 별도 기록. `--case`와 함께 쓰면 `selection.partial=true`를 유지하며, **선택한 fixture만 통과**한 것이지 NAME 전체 합격이 아님.
- [x] `tools/test_name_compatibility_audit.py`에 `matched` 허용, reference variant 누락 거부, mismatch/unsupported/observation gap 거부, 관측 0건 거부 회귀 코드 작성.
- [ ] **실제 실행/증거 대기:** `--scope-fixtures-only --case named_locative_alias_pos_mismatch --case named_direct_definition_reparse_after_pos_restore --case named_function_pos_rebinding_first_error --gate`로 고정 J 양 변형과 양 Rust 경로 대조. 실행 전에는 아래 명령을 PASS/FAIL로 기록하지 않음.
- [ ] direct definition 내 `syntax` 대 `domain` 첫 오류가 고정 J의 결과와 일치한 경우에만 `#[ignore]`를 해제하고 정상 회귀로 승격. 차이가 확인되면 parser/runtime 오류 순서 원인을 국소 수정한 뒤 원본과 재검증.

직전 `main` SHA `38902e3ed7196370e1c0b4ef450d95c175ccae82`에 대한 자동 [Linux milestone run 39733826013](https://github.com/yunskim/RustJ/actions/runs/39733826013)은 **SUCCESS**를 확인했다. 이 결과는 현재의 `--gate` 코드 추가 전 HEAD에만 해당한다. Rust 테스트/default/portable, fmt/clippy, reference baseline을 포함한 해당 자동 워크플로가 NAME 전용 C oracle까지 확인했다는 뜻은 아니다.

일괄 실행 때의 타깃 검사 예시:

```sh
python3 tools/name_compatibility_audit.py \
  --reference-root .reference \
  --binary target/debug/rustj \
  --rust-revision "$(git rev-parse HEAD)" \
  --report reports/name-pos-targeted.json \
  --scope-fixtures-only \
  --case named_locative_alias_pos_mismatch \
  --case named_direct_definition_reparse_after_pos_restore \
  --case named_function_pos_rebinding_first_error \
  --gate
```

**보류 상태:** 이번 변경에서는 수동 Rust/Python 검증, 고정 J-C 실행, CI 트리거를 하지 않았다. 코드는 작성했지만 J 오류 종류의 정확한 판정은 미완료. `--gate`가 켜지면 정확한 관측·원본 변형이 모두 갖춰지지 않는 한 PASS라고 주장하지 않는다.
