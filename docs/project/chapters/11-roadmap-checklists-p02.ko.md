[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)

> 문서 분할 안내: 기존 `PROJECT.ko.md`의 내용을 이동한 모듈입니다. 원문의 단일 파일 정본·신규 Markdown 금지 문구는 최신 [Agent 문서 규약](../../../AGENTS.md)보다 우선하지 않습니다.

# 10. 구현 계획과 체크리스트 — 계속 (2/5)

##### JE2 구현 — namespace와 runtime result의 JEntity 수렴 (2026-10-04, partial JE2)

- runtime `SymbolValue`를 제거하고 `Binding { value: JEntity, version: NameVersion }`으로 저장한다. 함수 binding에는 `Arc<FunctionEntity>`만 남으며 별도 Verb wrapper/span/target 복제가 없다. runtime final result도 동일 JEntity를 사용한다. `ExprKind`는 noun application/dependency 구조를 보존하는 parser representation으로 남는다.
- `commit_binding(name, JEntity) -> Result<JEntity>`가 version 증가 가능성을 먼저 검사하고, noun을 한 번 freeze한 뒤 namespace 저장/대입 반환 payload를 공유한다. 함수는 동일 DAG를 share한다. host assign은 이 함수에 위임하며 기존 noun replacement·OutputPool retirement 경로를 유지한다. automatic JEntity Clone을 추가하지 않고 freeze된 Value/Function Arc만 명시적으로 공유한다.
- noun lookup은 기존 값 snapshot을 내고 function lookup은 DAG의 result_pos를 전달한다. by-value nameless modifier, POS-known static alias, modifier resolution의 late NameRef chain·versions를 유지한다. 과거 enum variant가 암묵적으로 검사하던 Verb/Modifier class는 각 조회 경로의 명시적인 POS 검사로 대체했다. verb NameRef가 modifier로 바뀌면 같은 domain error/current_name을 보존한다.
- 더 이상 namespace에 Verb adapter가 필요하지 않아 `Verb::from_entity`와 그 전용 unit test를 제거했다. 보장하던 identity/span·POS 거부는 실제 commit/lookup 경로의 tests로 이행했다. parser occurrence wrapper와 `VerbTarget`은 아직 유지하며 namespace의 intrinsic Function identity와 분리한다. storage representation·Value 내부·gerund/view·optimizer/target policy는 바꾸지 않았다.

새 runtime unit tests 3개는 65,536-atom noun의 stored/returned pointer와 네 RHS class의 shared identity·versions, version overflow의 noun/function replacement 거부·기존 binding/pool/commit 보존, unified Function의 expected-POS/domain/current_name을 검증한다. integration regression은 cache limit 0/4096에서 noun→Verb→Adverb→Conjunction 교체 중 noun alias 생존과 마지막 alias 해제 뒤 bounded retirement·noun 재대입을 검증한다. 기존 adapter test 한 개를 제거하고 이 실제 경로 검사로 대체했다. C corpus/stage에는 noun/function replacement **11건**을 추가하여 entity boundary fixture는 **82건**이다.

**Namespace seam gate:** Windows default/portable 각각 **369 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture **4,821 cases / 4,817 passed / 기존 runtime 경계 4 / failed 0**, stage **10,001 checks**, words **6,618 cases**. capture graph 경계 108과 static 경계 2는 별도이며 full upstream suite·definition invocation acceptance·private C runtime trace 동등성은 미검증이다. report 10개의 실제 binary/source hash를 확인한다. conformance source pin은 `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`, DLL release는 `ded7793fe5795d79eda8e7138dce94aa056edf78`; JE0 source 감사 revision `0db94e768a845e2583c01d00538c3d16379677bb`는 새 DLL 검증으로 취급하지 않는다.

**남은 JE2:** parser completed-result transport와 deferred application 구조의 경계, 전체 local/locale/definition scope, explicit body invocation 및 static/runtime/capture 경로의 더 넓은 수렴을 검증한다. 이번 namespace seam 완료를 JE2 전체 완료로 표시하지 않는다. FunctionOperand view는 아래 단계에서 완료했으며 semantic nested DD와 기존 M2 gaps도 계속 추적한다. JE3+ collection·broader storage migration·optimization·CUDA·GitHub CI는 보류한다.

##### JE2 구현 — provenance를 보존하는 borrowed operand view (2026-10-04, partial JE2)

`FunctionOperand::as_entity_ref()`는 noun과 모든 함수 POS를 공통 `JEntityRef`로 조회한다. `span()`은 noun operand의 저장된 source span 또는 함수 identity의 span을 빌려준다. 이후 application occurrence span과 합치지 않는다. 조회에서 Value 복사·Arc 증가·새 entity 할당이 없고, owning enum은 noun provenance와 함수 DAG 소유권을 보존하기 위해 유지한다. gerund collection이나 물리 배열 representation으로 확대하지 않는다.

실제 사용 경계는 nameless modifier의 by-value lookup 판정과 semantic binding의 function NameRef DAG 순회다. 기존 순회 순서·POS/lookup 정책은 유지한다. operand를 실행용 parser item으로 만드는 경계는 공유 Arc 소유권이 필요하므로 기존 materialization 경로를 유지한다.

회귀는 owned 65,536-atom noun의 pointer/span 보존을 추가하고, 기존 host 종료 후 noun snapshot·48단계 shared DAG·explicit Verb/Adverb/Conjunction tests를 공통 view로 확장하여 payload identity, 함수/DefinitionCode 참조 수 및 원본 위치를 확인한다. 새 J 문법은 추가하지 않아 기존 82건 entity-boundary C fixtures를 그대로 사용한다.

**Operand seam gate:** Windows default/portable 각각 **370 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture **4,821 cases / 4,817 passed / 기존 runtime 경계 4 / failed 0**, stage **10,001 checks**, words **6,618 cases / failed 0**. capture graph 경계 108과 static 경계 2는 별도다. report 10개의 실제 binary/source hash를 확인했다. source/DLL pin은 위 namespace gate와 같으며 full upstream suite·definition invocation acceptance·private C runtime trace 동등성은 미검증이다.

completed parser result와 deferred noun/application 구조를 구분한 공통 transport는 아래 단계에서 진행한다. 전체 local/locale/definition scope와 explicit body invocation은 미완료다. JE2 전체 완료로 표시하지 않는다.

##### JE2 구현 — completed parser result의 공통 transport (2026-10-04, partial JE2)

`CompletedParseResult { entity: JEntity, span, height, verb_adapter }`는 완료된 RHS의 이동 경계다. noun Value와 함수 Arc를 복사하지 않고 이동하며, source occurrence/height와 Verb의 span/target adapter는 immutable FunctionEntity identity 밖에 둔다. `from_item`은 literal 또는 grouped literal noun만 받아들이고, 미계산 call·ReadName은 기존 unsupported 경계를 유지한다. 이 helper는 계산하거나 이름을 재조회하지 않는다. JEntity에 Clone을 추가하지 않는다.

runtime rows 0–2는 기존 host.apply를 한 번 실행한 뒤 이 경계로 completed noun을 stack에 반환한다. host가 없는 analysis 경로는 기존 Expr 연산 구조를 보존한다. row 7은 같은 경계로 RHS를 host.assign에 전달하고 같은 반환 payload로 parser item을 복원한다. commit capture·provenance inheritance·POS·lookup timing·effect order는 기존 reduction pipeline이 처리한다. ParseValue의 Expr/Verb/Function/NAME/control variants, constructor 경로, final Program 구조 전체를 바꾸지는 않는다.

근거는 [jsource runtime p.c row 7](https://github.com/jsoftware/jsource/blob/0db94e768a845e2583c01d00538c3d16379677bb/jsrc/p.c#L1006)의 stacked RHS 대입/반환이다. 이는 pv.c tacit translator와 구분하며 새 source pin으로 DLL을 빌드했다는 주장이 아니다.

새 unit regression 3개는 grouped owned 65,536-atom noun의 pointer/occurrence span/height, 세 함수 POS의 shared identity와 NameRef·Verb occurrence adapter, 정적 call/name 보존과 chained assignment의 apply 1회→inner→outer commit 2회를 확인한다. 기존 entity-boundary fixtures에 computed scalar chained assignment와 grouped computed array assignment 및 두 이름의 결과 조회 **6건**을 추가하여 **88건**으로 늘렸다. 이 실행 횟수 검사는 Rust host 경계의 관찰이며 private C trace 동등성 주장이 아니다.

**Completed-result gate:** Windows default/portable 각각 **373 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture **4,827 cases / 4,823 passed / 기존 runtime 경계 4 / failed 0**, stage **10,007 checks**, words **6,618 cases / failed 0**. capture graph 경계 108과 static 경계 2는 별도다. report 10개의 실제 binary/source hash를 확인했다. source/DLL pin은 위 namespace gate와 같다. full upstream suite·definition invocation acceptance·private C runtime trace 동등성은 미검증이다.

다음은 남은 parser value/constructor 경계에서 concrete completed result와 분석용 expression이 중복 전달되는 지점을 검토한다. 전체 stack enum 수렴, local/locale/definition scope 및 explicit body invocation은 미완료다. optimizer·CUDA·GitHub CI는 계속 보류한다.

##### JE2 구현 — modifier constructor operand의 공통 경계 (2026-10-04, partial JE2)

`CompletedParseResult::into_operand()`는 완료 noun을 기존처럼 한 번 `into_shared`하고 원본 occurrence span과 함께 FunctionOperand에 이동한다. 함수는 동일 Arc를 이동하며 occurrence용 Verb adapter를 semantic child에 복제하지 않는다. `modifier_train`의 Noun/Verb/Adverb/Conjunction 변환 중복을 이 경계로 대체했다. production rows 5/6과 gerund AR의 동일 modifier-train 생성 경로에 적용되며, result POS는 기존 cf.c disposition/constructor 결정으로 유지한다. raw NAME/control은 syntax error, deferred call/ReadName은 기존 unsupported로 남긴다.

conjunction rank/right-first audit, noun-left fork, definition constructor, immediate bident/trident application은 각각의 검증 순서·실행 의미가 있어 이번 변경에 합치지 않았다. generic entity collection이나 physical storage 변경도 없다.

새 회귀 2개는 grouped owned 65,536-atom noun이 복사 없이 freeze되고 원본 span을 유지하며 train 해제 뒤 application occurrence에서 재사용되는지 확인한다. 함수 세 POS의 DAG identity/참조 수, cf.c disposition과 result POS, 미계산 noun의 거부 및 control의 syntax error도 확인한다. 기존 runtime noun-origin capture·named array snapshot·nested modifier train tests와 88건 entity-boundary C fixtures를 재검증한다.

**Constructor-operand gate:** Windows default/portable 각각 **375 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture **4,827 cases / 4,823 passed / 기존 runtime 경계 4 / failed 0**, stage **10,007 checks**, words **6,618 cases / failed 0**. capture graph 경계 108과 static 경계 2는 별도다. report 10개의 실제 binary/source hash를 확인했다. source/DLL pin은 위 namespace gate와 같다. full upstream suite·definition invocation acceptance·private C runtime trace 동등성은 미검증이다.

다음은 rank/conjunction operand 경계에서 right-before-left 오류 우선순위와 gerund 감사 계약을 먼저 고정한 뒤 공통 transport 사용 여부를 검토한다. JE2 전체 완료와 전체 J parser 지원으로 표시하지 않는다.

##### JE2 구현 — rank/conjunction operand와 오류 우선순위 (2026-10-04, partial JE2)

`apply_conjunction_at`의 좌우 Noun/Verb operand 전달을 `CompletedParseResult::from_item/into_operand`로 연결했다. 먼저 right operand의 형태를 검사하며 @:의 noun-right domain error는 미계산 noun의 unsupported보다 앞선다. rank의 noun-right는 rank→length→numeric domain 순서로 검사한 뒤에만 left operand를 읽거나 gerund를 감사한다. 원본 Expr span과 parser reinsertion span override의 기존 구분을 유지하고, noun freeze와 함수 DAG 이동은 공통 경계를 사용한다.

근거는 [고정 jsource cr.c::jtqq](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c#L733)의 right rank 추출과 그 이후 noun-left 분기다. boxed rank-1 noun만 gerund 감사 대상이며 모든 요청 rank가 RMAX이면 감사를 생략한다. fx 감사의 J 오류는 quiet constant-noun fallback으로 처리하고 partial decoded list를 남기지 않는다. RustJ 구현 경계인 unsupported는 계속 전달하며 성공한 C 기능인 것처럼 fallback하지 않는다. verb-right는 원래 function operand로 유지하고 noun-left function identity로 뒤바꾸지 않는다.

새 unit 2개는 owned 65,536-atom constant와 rank noun의 pointer/Expr span, deferred left보다 오른쪽 rank/length/domain 오류가 먼저 나오는지 및 @: noun-right domain 우선순위를 확인한다. capture 회귀 1개는 right 오류 시 gerund name lookup/commit 부재와 기존 binding version 보존, valid rank/verb-right의 quiet 감사, 무한 rank 감사 생략 및 partial decode 제거를 확인한다. compound gerund C corpus/stage에 setup·오류 뒤 binding 사용·quiet/RMAX/verb-right construction **12건**을 추가한다. entity-boundary fixtures는 별도 88건으로 유지한다.

**Rank-operand gate:** Windows default/portable 각각 **378 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture **4,839 cases / 4,835 passed / 기존 runtime 경계 4 / failed 0**, stage **10,019 checks**, words **6,618 cases / failed 0**. capture graph 경계는 새 사례 2건을 포함한 **110건**, static 경계는 2건이며 runtime pass와 별도로 기록한다. report 10개의 실제 binary/source hash를 확인했다. conformance source/DLL pin은 JE2 namespace gate와 같고 full upstream suite·definition invocation acceptance·private C trace 동등성은 미검증이다.

다음 JE2 후보는 noun-left fork와 definition constructor의 완료 값 전달 경계다. 각각 source span·생성 오류·본문을 실행하지 않는 계약을 먼저 확인한다. 전체 stack variant 수렴, scope와 definition invocation은 여전히 미완료다.

##### JE2 구현 — noun-left fork와 definition 생성 경계 (2026-10-04, partial JE2)

`CompletedParseResult::from_noun`은 grouped literal을 기존 completed_noun 규칙으로 이동하며 Expr span/height를 보존한다. 일반 from_item은 이 경계를 공유하되 기존 Item occurrence override를 복원한다. noun-left fork는 이 결과를 into_operand로 옮겨 source span·g/h DAG identity를 유지한다. 이전 fork만의 owned noun 경로를 공통 one-time freeze 정책으로 바꾸어 큰 상수의 이후 operand 재사용이 전체 배열 복사를 만들지 않도록 했다. allocator/physical representation 자체는 바꾸지 않았다.

지원 DefinitionConstructor는 양쪽 noun class guard를 먼저 수행한 뒤 mode와 body를 기존 순서로 완료 값에서 추출한다. 이 transient 입력은 freeze하거나 FunctionEntity operand로 저장하지 않는다. mode/body 원본 일치와 semantic code validation은 그대로 수행한다. 생성 결과는 실제 Verb/Adverb/Conjunction POS를 가진 FunctionEntity를 공통 function→into_item 경계로 반환한다. DefinitionCode Arc의 불필요한 clone도 제거했다. 원본 code와 source provenance를 보존하며 invocation/local frame을 생성하거나 본문을 실행하지 않는다. computed definition의 미지원 범위는 확대하지 않았다.

새 unit 2개는 grouped owned 65,536-atom fork 상수의 pointer/source span·g/h identity, 두 번 재사용 후 fork 해제까지 공유 생존, 미계산 noun 거부를 검증한다. definition의 양쪽 class guard가 deferred input보다 먼저 적용되는지와 기존 domain/unsupported 우선순위도 검증한다. integration 1개는 explicit/direct 각 세 함수 POS의 생성·commit, 본문 counter의 version/value 불변과 본문 이름 input observation 부재를 확인한다. C definition corpus/stage에 같은 6개 생성과 counter 조회 **12건**을 추가하며 entity-boundary fixture는 별도 88건이다.

**Fork/definition gate:** Windows default/portable 각각 **381 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture **4,851 cases / 4,847 passed / 기존 runtime 경계 4 / failed 0**, stage **10,031 checks**, words **6,618 cases / failed 0**. capture graph 경계 **114건**과 static 경계 2건은 runtime pass와 별도로 기록한다. report 10개의 실제 binary/source hash를 확인했다. conformance source/DLL pin은 JE2 namespace gate와 같고 full upstream suite·definition invocation acceptance·private C trace 동등성은 미검증이다.

다음은 남은 completed-result/function wrapper 경계를 다시 감사하여 필요 없는 adapter만 제거하고, frontend F/P 체크리스트의 지원 범위와 미지원 생성/실행 범위를 정리한다. 전체 stack variant 수렴·local/locale scope·definition invocation은 여전히 미완료다. CUDA·optimizer 구현·GitHub CI는 계속 보류한다.

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

##### JE2/P3 구현 — ordinary NAME scope 경계 해소 (2026-10-04, partial)

- [x] C native oracle로 ordinary NAME의 cross-frame 전달·global publication·미초기화 local 대입·operand/local 이름 충돌을 확인했다. 이전 scoped-reference 보수적 경계는 아래 범위에서 해소했다.
- [x] ordinary NameRef를 implicit locative로 오인한 네 제한과 전체 함수 DAG의 이름 membership 재검사를 제거했다. 함수 이름은 expected POS를 가진 late reference로 유지한다. 호출은 현재 frame → global을 조회하며 caller frame을 캡처하거나 탐색하지 않는다. `u/v` operand substitution과 noun snapshot은 그대로다.
- [x] `smf=.u` 후 `smf`를 안쪽 modifier에 전달하면 안쪽에서 ordinary `smf` 실행은 안쪽 local 또는 global을 조회한다. caller의 지역 `smf`로 고정되지 않는다. `smexport=:smf/`도 ordinary 이름을 보존하고 frame 밖 global 재정의를 반영한다. 미초기화 `smf=.smf`는 RHS의 현재 noun/function 품사를 따르며 noun이면 snapshot, 함수면 NameRef이다. 같은 이름의 operand/local 대입도 유효하다.
- [x] Rust scope regression은 기존 Unsupported golden 1개를 실제 의미 검증 3개로 교체했다. 재정의·POS mismatch·undefined→defined, 실패 뒤 outer target/version 유지·이미 commit된 global publication과 frame 복구를 검증한다. 공통 single-line C corpus **21건**, stage에 이를 포함한 **45건**을 추가했다. 반환 함수의 C atomic representation과 실제 결과/오류를 비교한다. `af.c`를 검토 소스 hash 목록에 추가했다.
- [ ] 실제 implicit locative `u./v.`, 전체 operator wrapper/scope, control flow·nested scope와 body graph/source diagnostic frame은 후속 작업이다. x/y 직선 호출은 아래 operator-call 단계에서 지원한다. 일반 NAME 지원을 implicit-locative fix나 전체 closure 지원으로 확대하지 않는다. static/no-host modifier application은 계속 명시적 경계이며 실행 성공을 정적 분석 완료로 계산하지 않는다.

**Ordinary-reference gate:** native Windows default/portable 각각 **398 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 세 runtime 경로 **4,963 cases / 4,959 passed / 기존 runtime 경계 4 / failed 0**, stage **10,208 checks / failed 0**, words **6,618 cases / failed 0**. capture graph 경계 **171건**, static 경계 **2건**은 별도다. 새 runtime waiver는 없다. 보고서 10개의 실제 binary/source hash를 확인했다. reviewed source `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`, 실행 DLL release `ded7793fe5795d79eda8e7138dce94aa056edf78`를 구분한다. full upstream·ignored definition acceptance·private C trace 동등성은 미검증이며 optimizer·CUDA·GitHub CI는 보류한다.

이후 operator-call 단계에서 deferred callable의 직선 호출을 구현했다. implicit locative와 실행되지 않는 definition body graph/source frame은 후속 경계다. effect가 commit된 뒤 Unsupported가 날 수 있으므로 자동 replay하지 않는다.

Sources: [p.c ordinary lookup / mnuvxy](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c#L616), [cx.c return-time implicit-locative fix](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L679), [af.c hasimploc / fix scope](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/af.c#L17).

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

##### JE2/P3 구현 — implicit operand의 caller scope 호출 (2026-10-04, partial)

- [x] C `sc.c::unquote`와 Windows oracle에서 ordinary u와 implicit u.의 caller-local 조회 차이, v., monad/dyad, noun/missing operand 오류를 확인한다.
- [x] direct primitive 및 ordinary alias를 통한 implicit 호출에서 현재 frame의 operand를 확보한 뒤 caller 환경에서 실행하고, 성공·J 오류 모두에서 현재 frame을 복구한다. 전달 noun을 복사하지 않고 기존 parser/runtime executor를 공유한다.
- [x] Rust 회귀 및 C 양 버전 대조에 caller-local 충돌, operand 지역 재대입, 전역 publication, 오류 후 frame/전역 효과 복구를 추가한다. graph unknown 계약은 유지한다.
- [ ] `/`·rank·train 내부의 raw implicit 실행, 미설치 operand의 반환 참조, 전체 locale/control/body graph/source frame은 별도 후속 경계다. 이 단계는 global locale path 전환의 전체 구현이 아니다.

Rust 회귀 **2개**가 ordinary u와 u.의 caller-local 차이, u/v monad/dyad, noun·missing operand 오류, 안쪽 operator의 실패 뒤 global 효과 보존과 frame 복구를 확인한다. 기존 implicit-return regression도 유지한다. 공통 runtime **23건**, 이를 포함한 stage **47건**을 추가했다. **Caller-scope gate:** native Windows default/portable 각각 **406 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 세 runtime 경로 **5,053 cases / 5,049 passed / 기존 runtime 경계 4 / failed 0**, stage **10,367 checks / failed 0**, words **6,618 cases / failed 0**. capture graph 경계 **204건**, static 경계 **2건**은 별도다. 새 runtime waiver는 없다. frontend report 10개의 실제 binary/reference/source hash를 검증했다. source pin `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`과 DLL release `ded7793fe5795d79eda8e7138dce94aa056edf78`를 구분한다. full upstream·ignored definition acceptance·private C trace 동등성은 미검증이다. optimizer·CUDA·Linux/GitHub CI는 실행하지 않았다. 미설치 operand의 **호출**은 ValueError이며 **반환** 참조는 아직 별도 경계다. 재귀 depth 제한은 기존 Windows bound를 유지하며, suspend된 callee도 invocation depth에 포함한다.

Sources: [sc.c local operand와 caller 환경 전환](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sc.c#L122), [sc.c implicit primitive 호출](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sc.c#L433).

##### JE2/P3 구현 — wrapper와 train의 scope-aware 실행 (2026-10-04, partial)

- [x] C `ar.c::jtredg`의 오른쪽 결합, `j.h::FORK1/FORK2`의 오른쪽 가지 우선, rank의 cell/frame·prefix agreement를 검토한다.
- [x] A3 executor의 rank/reduction cell 알고리즘을 callback seam으로 공유한다. 기존 primitive kernel 경로는 유지하고, Unsupported composition에만 runtime FunctionEntity 실행을 연결한다. operator/rank DAG를 flatten하지 않는다.
- [x] `/`의 비어 있지 않은 monad, uniform nonempty rank, Hook/Fork/Atop의 monad/dyad에서 실제 child invocation마다 이름과 implicit caller scope를 조회한다. fork input은 공유 noun으로 보존하고 오른쪽 가지 오류 전에 왼쪽 가지를 실행하지 않는다.
- [x] Rust 및 C 양 버전 corpus에 wrapper·caller-local 충돌·branch effect/error 순서를 추가한다. static unknown 계약과 effect barrier는 유지한다.
- [ ] empty identity/prototype, heterogenous rank fill/padding, sparse·dyadic insert와 noun-left/capped train은 별도 확인한다. callback의 Unsupported는 commit된 효과가 없는 재실행 허가가 아니다.

**Wrapper/train gate:** Windows default/portable 각각 **408 passed / 17 ignored**, fmt/clippy/build 통과. native Python **27 passed**. j64/AVX2 각각 세 runtime 경로 **5,089 cases / 5,088 passed / runtime 경계 1 / failed 0**, stage **10,424 checks / failed 0**, words **6,618 cases / failed 0**. capture graph 경계 **222건**, static 경계 **2건**은 별도다. 기존 runtime 경계 3건(Atop 및 fork 안 named insert 2건)을 해소하고 해당 waiver를 제거했다. 새 waiver는 없다. frontend report 10개의 binary/reference/source hash를 확인했고 `ar.c`를 검토 소스 목록에 추가했다. source pin `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`, DLL release `ded7793fe5795d79eda8e7138dce94aa056edf78`를 구분한다. full upstream·ignored definition acceptance·private C trace 동등성은 미검증이며 optimizer·CUDA·Linux/GitHub CI는 실행하지 않았다.

Rust regression 2개와 기존 named-insert provenance regression은 source DAG 보존·오른쪽 결합·caller-local·rank shape와 branch effect/error 순서를 확인한다. 공통 runtime **36건**, 이를 포함한 stage **57건**을 추가했다. primitive 호출에는 함수 Arc 복사를 추가하지 않고, Runtime fallback만 composition을 순회한다. A3의 Hook/Fork executor와 전체 body graph lowering 완료를 주장하지 않는다. 다음은 empty reduction identity와 empty rank prototype을 C와 대조한다.

Sources: [ar.c reduce](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ar.c#L513), [j.h fork execution](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/j.h#L1249), [cr.c rank](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c).

##### JE2/P3 구현 — empty identity와 순수 ravel prototype (2026-10-04, partial)

- [x] C `ai.c::jtiden`과 Windows oracle로 `+ - * %`의 identity를 확인했다. empty rank에서 사용자 verb가 prototype 계산용으로 한 번 실행되어 global 효과가 남을 수 있다는 점을 확인했다.
- [x] primitive witness를 현재 이름/POS·implicit caller scope에서 조회하되 definition 본문을 실행하거나 분석한 것으로 계산하지 않는다. 확인된 네 identity를 기존 kernel 경로로 계산하며 caller frame을 복구한다.
- [x] 순수 monadic ravel은 cell shape만으로 empty rank output shape/type을 계산한다. primitive와 implicit wrapper가 같은 kernel을 사용한다. negative rank·다차원 zero axes·character type·caller-local collision을 검증한다.
- [x] Rust와 C 양 버전 회귀를 추가하고 일반 사용자 verb의 prototype 및 empty identity 경계를 별도로 유지한다. 일반 prototype의 효과·suppressed error·fill/padding, 다른 primitive prototype, sparse·dyadic insert는 후속 작업이다.

**Empty-scope gate:** native Windows default/portable 각각 **410 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 세 runtime 경로 **5,130 cases / 5,129 passed / runtime 경계 1 / failed 0**, stage **10,474 checks / failed 0**, words **6,618 cases / failed 0**. capture graph 경계 **230건**, static 경계 **2건**은 별도다. 새 waiver는 없다. report 10개의 binary/reference/source hash를 확인했고 `ai.c`를 검토 목록에 추가했다. Rust regression **2개**, 공통 runtime **41건**, 이를 포함한 stage **50건**을 추가했다.

일반 explicit verb의 empty rank에서는 C가 body를 한 번 실행해 count를 1로 만들었으며, unknown explicit reduction identity는 DomainError를 내고 count는 0이었다. 두 경계의 j64 probe는 `reports/empty-prototype-oracle-windows.json`에 별도 기록한다. Rust는 아직 명시적 Unsupported이며 이 두 사례의 동등성/순수성을 주장하지 않는다. runtime primitive witness는 compile-time binding proof를 뜻하지 않고 analyzer의 unknown 계약을 바꾸지 않는다. full upstream·ignored definition acceptance·private C trace는 미검증이며 optimizer·CUDA·Linux/GitHub CI는 실행하지 않았다. 다음은 noun-left fork이다.

Sources: [ai.c identities](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ai.c#L368), [ar.c empty reduction](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ar.c#L505), [cr.c rank execution](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c).

##### JE2/P3 구현 — noun-left fork 호출과 snapshot 공유 (2026-10-04, partial)

- [x] C `j.h` NVV 경로와 Windows oracle에서 왼쪽 noun은 생성 시 snapshot이고 오른쪽 h 실행 후 g에 넘겨진다는 것을 확인한다. monad/dyad와 이름 재대입·agreement 오류를 대조한다.
- [x] 기존 parser의 source Fork DAG와 CompletedParseResult의 공유 noun을 그대로 사용한다. 오른쪽 child만 호출하고 왼쪽 값은 지연 이름 조회로 바꾸지 않는다. 두 가지 입력 보존을 위한 불필요한 공유 변환은 하지 않는다.
- [x] 65,536-atom snapshot의 pointer 유지·이름 재대입 뒤 생존, implicit caller scope, join 실패 전에 commit된 오른쪽 효과와 오류 후 frame 복구를 검증한다.
- [x] C 양 버전 및 기존 frontend/portable gate를 통과한 뒤 기록한다. capped fork, noun-left GraphForm/Logical lowering 전문화, 일반 prototype/control/body graph는 별도 후속 작업이다.

Sources: [cf.c noun fork](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L59), [j.h NVV execution](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/j.h#L1277).


**Noun-fork gate:** native Windows default/portable 각각 **412 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 세 runtime 경로 **5,156 cases / 5,155 passed / runtime 경계 1 / failed 0**, stage **10,511 checks / failed 0**, words **6,618 cases / failed 0**. capture graph 경계 **234건**, static 경계 **2건**은 별도다. report 10개의 actual binary/reference/source hash를 확인했고 새 waiver는 없다. Rust regression **2개**와 기존 computed-noun capture regression을 실제 실행 결과 검증으로 갱신했다. 공통 runtime **26건**, 이를 포함한 stage **37건**을 추가했다. full upstream·ignored definition acceptance·private C trace 동등성은 미검증이며 optimizer·CUDA·Linux/GitHub CI는 실행하지 않았다.

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

##### JE4 — gerund/boxed higher-order semantics 통합

- [ ] gerund를 새 global atom/POS type으로 만들지 않고 **boxed noun + modifier-context interpretation**이라는 기존 J semantics를 유지한다.
- [ ] gerund interpretation이 필요할 때만 boxed noun에서 operator-specific `GerundView`/`InterpretedEntitySequence`를 만든다. source boxed noun의 shape는 해당 operator가 필요할 때만 view metadata로 보존한다.
- [ ] generic `EntityCollectionView`는 JE3의 공통성 증명이 끝난 뒤에만 추출한다.
- [ ] view 내부에 function entity ref가 있어도 그 function에 container의 rank/shape를 복사하지 않는다.
- [ ] gerund 내부 name/function reference의 fix/late-binding/version 규칙을 보존한다.
- [ ] ordinary boxed data와 gerund interpretation이 같은 payload에서 context에 따라 달라지는 golden test를 추가한다.
- [ ] 현재 `decoded_gerund: Option<Vec<Arc<FunctionEntity>>>` 특수 필드를 공통 entity view로 대체할 수 있는지 검토하고, 의미 손실이 있으면 유지한다.

**JE4 완료 조건:** gerund와 boxed data의 문맥적 차이를 잃지 않으면서 higher-order entity collection을 공통 abstraction으로 표현할 수 있다.

##### JE5 — entity algebra와 array-execution algebra의 경계

- [ ] `JEntity` layer와 `Logical Execution IR`의 역할을 분리한다: function entity 자체는 logical array value가 아니고, **적용된 verb가 noun input을 받아 noun result를 만드는 순간** array execution graph로 내려간다.
- [ ] monadic application을 `JEntity(Verb) × JEntity(Noun) → JEntity(Noun)`, dyadic application을 `Noun × Verb × Noun → Noun`의 semantic contract로 검증한다.
- [ ] adverb/conjunction application은 일반적으로 function entity derivation이지만, parser bident/trident semantic action이 immediate noun result를 만들 수 있는 경우까지 `JEntity` boundary가 표현한다. 어느 경우에도 parser result를 즉시 physical execution representation으로 고정하지 않는다.
- [ ] `CellApply`/Reduce/Scan/Reindex가 entity layer가 아니라 applied array-computation layer에 남는지 확인한다.
- [ ] effect flow(namespace/I/O/state)와 entity/value flow를 직교하게 유지한다.
- [ ] J Graph/Logical IR이 `JEntityArray`의 physical layout이나 entity-container storage를 알 필요가 없다는 verifier/invariant를 둔다.

**JE5 완료 조건:** `JEntity` 일반화가 현재의 “verb application = logical array computation” 모델을 흐리지 않고 오히려 그 경계를 명시적으로 만든다.

##### JE6 — migration cleanup과 비용 검증

- [ ] compatibility adapter와 중복 `Noun|Function` carrier를 제거한다.
- [ ] public/internal API 이름을 정리하고 `JEntity`/`EntityArray` ownership/lifetime 문서를 고정한다.
- [ ] large derived function, gerund, repeated binding에서 deep-copy/refcount churn이 악화되지 않는지 benchmark한다.
- [ ] compiler coverage manifest에 entity-layer 지원/late-binding/runtime fallback 경계를 추가한다.
- [ ] M2 frontend conformance corpus와 기존 J Graph/A3 golden을 전부 다시 통과시킨다.
- [ ] Logical/Physical Array invariant에서 BufferId/stride/device가 entity layer로 역류하지 않았는지 구조 검사를 추가한다.

**JE6 완료 조건:** 기존 observable J semantics와 compiler pipeline 결과가 유지되고, 중복 carrier를 줄였으며, 공통 entity abstraction이 storage/runtime coupling을 새로 만들지 않는다.

**운영 규칙:** 이후 `JEntity`/higher-order view 관련 진행 보고는 반드시 `JE0`–`JE6` 항목 번호로 보고한다. 새 요구사항은 임시 TODO로 분산시키지 않고 먼저 이 체크리스트의 적절한 단계에 추가한다. 최소 `JEntity`는 M2의 중복 carrier를 줄이는 작은 seam부터 허용하지만 broad rewrite는 금지한다. JE3에서 공통 shaped-entity algebra가 입증되지 않으면 generic `EntityArray`/`EntityCollectionView`를 구현 목표로 강제하지 않는다.

### A0 — 문서/아키텍처 경계

- [x] RustJ 내부 compiler stage의 논리적 경계를 확정한다.
- [x] Semantic Analyzer 입력 전에 hook/fork/train/derived verb/rank를 제거하지 않는 원칙을 확정한다.
- [x] `J Semantic Array IR`과 `Logical Array IR / Plan`을 구분한다.
- [x] generic boundary 후보를 semantic analysis 이후의 Logical Array IR로 이동한다.
- [x] 문서를 `PROJECT.ko.md`로 통합한다.
- [x] 현재 지원 subset의 source frontend → FunctionEntity → J Graph → canonical Logical IR 분석 경계를 연결했다. full frontend/execution 완료는 M2/M4에서 별도로 추적한다.

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

#### Frontend 파일 경계

| 파일 | 책임 | 입력 → 출력 |
|---|---|---|
| `src/tokenizer.rs` | `w.c::jtwordil` state machine, raw words·parse-visible comment cutoff | source bytes → byte spans |
| `src/enqueuer.rs` | `jtenqueue` 해석, primitive/literal/name/copula·환경별 flags | source + tokenizer spans → `EnqueuedWord` queue |
| `src/parser.rs` | parse class/9-row matcher, stack reduction, modifier/train construction, parser-time name/POS lookup | typed queue → `Program`/completed `FunctionEntity` |
| `src/semantic.rs` | target-independent 의미 객체·rank constructor 계약·binding/version model | parser 결과를 실행/분석 계층에 전달 |

Tokenizer·enqueuer·parser 구현은 각각 한 파일이 소유한다. 기존 `scanner` module과 `semantic::parse`/row API는 compatibility re-export만 남겨 기존 사용자를 보호하며 별도 grammar를 유지하지 않는다. parser는 enqueue 결과를 spelling으로 다시 분류하지 않는다. execution/target 선택을 이 세 파일에 넣지 않는다.

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

#### Noun reduction과 컴파일용 구조 보존 — 조사 및 구현 계획 (2026-10-03)

**결정:** runtime parser는 jsource처럼 verb application을 실행하여 실제 noun으로 reduce하고, 컴파일러는 별도 capture에서 생산 연산과 input/output 연결을 보존한다. noun이 된다는 이유로 provenance를 버리지 않는다. verb 중심의 tacit 표현은 구조를 노출하는 권장 방식이며 필수 언어 제한이 아니다. 이 절은 F2/P2–P6를 구체화하는 계획이고 별도 roadmap이나 두 번째 canonical IR을 만들지 않는다.

**현재 남은 차이:** runtime rows 0–2는 이제 host를 통해 실제 noun으로 reduce한다. static context는 연산 Expr를 보존한다. capture v0는 source operation과 occurrence 연결을 별도로 보존하며, 성공 capture를 기존 J Graph로 변환하는 adapter도 구현했다. top-level single-name non-final assignment는 구현했다. explicit-local/locale/definition/effect 및 전체 modifier POS는 계속 미완료다. 일반 fork executor 미지원도 frontend construction 지원과 구분한다.

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

##### 실행 경로와 재사용 경계

1. **Reference semantic execution:** capture on/off 모두 같은 parser class matcher와 row actions를 사용한다. rows 0–2의 runtime action은 그 시점에 실제 noun을 생성한다. record 여부로 실행 횟수·name lookup·오류 시점이 달라지면 안 된다.
2. **Execute-and-capture:** 제안 API `Engine::eval_with_capture(&mut self, source)`는 사용자 문장을 한 번 실행하고 outcome + capture를 반환한다. 이는 read-only `prepare_semantic/analyze_j_graph(&self, ...)`와 구분한다. 실패를 기록하려면 outcome을 필드로 가진 report가 필요하며 outer `Result` 때문에 partial trace를 잃지 않도록 API를 정한다. 분석 요청을 명분으로 IO/assignment를 몰래 실행하거나 성공 trace를 만들기 위해 두 번 실행하지 않는다.
3. **Static compilation:** 기본 비실행 경로의 pure/static 범위는 같은 parser row engine의 abstract actions로 graph를 만들 수 있다. 실제 값이 필요한 constructor, unknown POS/binding, effects/error boundary는 typed dependency와 coverage reason으로 드러내며 현재 지원 runtime action/region을 이용한다. no-execution AOT는 unknown dependency를 명시적으로 거부하거나 residual runtime region으로 나타내고 compile-time 실행으로 해결하지 않는다.
4. **Captured graph reuse:** trace 한 번으로 모든 입력/branch가 표현되었다고 주장하지 않는다. observed shape·data-dependent constructor/POS·binding/environment assumptions를 constants/input dependencies/guards로 구분한다. reuse 전에 검증하거나 재capture/semantic execution으로 되돌린다. 재capture는 이미 일부 effects를 실행한 지점에서 문장 처음부터 다시 시작하는 방식으로 구현하지 않는다. v0 capture는 inspection에 한정하고 재사용 실행을 기본 제공하지 않는다.
5. **효율:** execute-and-capture는 첫 실행의 array 계산 비용을 없애지 않는다. v0는 trace arena + shared function references + small facts를 기본으로 하여 capture 때문에 모든 temporary가 살아남지 않게 한다. pure-array region reuse/JIT/fusion이 subsequent execution의 성능 단계다. CUDA 구현은 계속 유예한다.


---

[← 이전 부분](./11-roadmap-checklists-p01.ko.md) | [다음 부분 →](./11-roadmap-checklists-p03.ko.md)
