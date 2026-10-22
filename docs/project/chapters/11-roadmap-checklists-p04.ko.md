[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)

<a id="p2--하나의-9-row-reduction-engine으로-전환"></a>
#### P2 — 하나의 9-row reduction engine으로 전환

- **jsource invariant:** conjunction chain은 parse-table 구조상 left-to-right로 결합한다. 예: `u @: v @: w`의 semantic graph는 `(u @: v) @: w`이다.
- [x] row 0 `EDGE VERB NOUN ANY`를 first-match로 선택한다. analysis/no-host 경로는 monadic `Expr`를 보존하고, runtime-host 경로는 그 자리에서 실행해 completed noun `Value`를 같은 stack에 재삽입한다.
- [x] row 1 `EDGE+AVN VERB VERB NOUN`의 정확한 four-class eligibility/reduction extent를 구현했다. row 0과 마찬가지로 analysis는 application structure를 보존하고 runtime host는 actual noun으로 reduce/reinsert한다.
- [x] row 2 `EDGE+AVN NOUN VERB NOUN`을 production stack reducer에서 선택한다. analysis/no-host 경로는 dyadic `Expr`를 보존하고 runtime-host 경로는 actual noun을 계산해 같은 stack에 재삽입한다.
- [ ] row 3 `EDGE+AVN (VERB|NOUN) ADV ANY`를 modifier semantic constructor 호출로 구현한다. 지원 adverb/gerund 생성은 구현되었으며 전체 primitive·explicit modifier application은 미완료다.
- [ ] row 4 `EDGE+AVN (VERB|NOUN) CONJ (VERB|NOUN)`를 modifier semantic constructor 호출로 구현한다. rank/@:/지원 DefinitionConstructor 경계는 구현되었으며 전체 conjunction 및 invocation은 미완료다.
- [x] row 5 `EDGE+AVN (VERB|NOUN) VERB VERB`의 production Fork construction을 구현했다. VVV ordinary/capped 판정과 noun-left fork의 지원 construction을 같은 row action에서 처리하며, 전체 noun/value-dependent/static coverage는 P3에서 계속 추적한다.
- [x] row 6 `EDGE CAVN CAVN ANY`의 production Hook/bident/trident disposition dispatch를 구현했다. basic Hook과 non-executing modifier train을 구성하고 earlier row가 소유해야 할 immediate action을 invariant로 거부한다. 전체 primitive/definition executor coverage는 P3의 미완료 범위다.
- [ ] row 7 `(NAME|NOUN) ASGN CAVN ANY` assignment reduction과 effect/result semantics를 구현한다. top-level single-name의 네 RHS class와 중간/연속 대입은 구현되었다. 아래 F1/P4 보완에서 문자열 noun/multiple-name target 실행도 추가했다. boxed/AR target·ordered-write lowering·전체 scope는 미완료다.
- [x] row 8 `LPAR CAVN RPAR ANY`를 production stack action으로 구현하고 recursive parenthesis parser를 제거했다. grouped noun은 `ExprKind::Group`/depth를, grouped function은 semantic identity를 유지한 채 parser provenance span을 괄호 전체로 보존한다.
- [x] 구현된 각 reduction 결과를 같은 parser stack에 되돌리고 동일한 match_parse_row로 다시 scan/reduce한다. 미지원 semantic form은 해당 action의 오류/coverage 경계로 남긴다.
- [ ] row action abstraction이 `ReadyParseValue`와 `RequiresRuntimeSemanticParse`를 구분할 수 있게 하여, 정적 compiler path가 parser-visible runtime dependency를 숨기지 않게 한다.
- [x] runtime semantic host와 analysis가 동일 parse_context/row matcher/action engine을 사용한다. host가 필요한 미지원 form 전체의 executor는 별도 미완료다.
- [x] 기존 flat-vector modifier/train/application reducer를 삭제하고 production expression reduction을 right-to-left stack + ordered `match_parse_row`로 cutover했다. 아직 미구현 semantic form은 해당 row action에서 명시적으로 남긴다.
- [ ] one-word sentence의 별도 jsource path와 관찰 가능한 결과가 동일하도록 테스트한다.

**P2 완료 조건:** 모든 parser reduction 선택을 jsource row 번호와 input class 조합으로 설명할 수 있고, deferred semantic action과 runtime semantic action이 동일한 parser engine을 공유한다. parser-visible effect/value dependency를 무시한 정적 진행 경로가 없다.

<a id="p3--modifierhookforkbidenttrident-construction-semantics"></a>
#### P3 — modifier/Hook/Fork/bident/trident construction semantics

- [ ] rows 3–4에서 result POS를 RustJ가 임의로 고정하지 않고 **modifier semantic constructor가 반환한 실제 POS**를 다음 parser class로 사용한다.
- [ ] row 3의 `VERB ADV`와 `NOUN ADV`를 각 adverb의 J construction semantics에 따라 처리한다.
- [ ] row 4의 `(VERB|NOUN) CONJ (VERB|NOUN)` 전체 parser form을 각 conjunction의 J construction semantics에 따라 처리한다.
- [x] `cf.c::bidents[]`를 `SyntaxError | ImmediateSemanticApply | BuildDerivedModifier(result_pos)`의 semantic disposition으로 옮겼다. row 6의 non-executing modifier 구성과 AR/derived modifier 내부의 지원된 immediate action은 구현되어 있다. 전체 primitive/definition executor 지원과는 구분한다.
- [x] `cf.c::tridents[]`를 `SyntaxError | ImmediateSemanticApply | BuildFork | BuildDerivedModifier(result_pos)`의 semantic disposition으로 옮겼다. row 5 fork와 row 6 non-fork modifier를 구분한다. AR/derived modifier의 지원된 immediate action은 구현되어 있으나 전체 primitive/definition executor는 미완료다.
- [ ] VV Hook과 NVV/VVV Fork의 construction boundary를 jsource와 동일하게 만든다.
- [ ] 긴 train은 별도 `LongTrain` algorithm이 아니라 row 5/6 반복 reduction의 결과로만 형성한다.
- [ ] modifier application마다 completed entity 하나를 만들고 후속 reduction은 그 entity ref만 보게 한다.
- [ ] parser semantic operand와 jsource execution auxiliary(`fgh` helper slot, `localuse`, cached executor)를 구분한다.

**P3 완료 조건:** derived Verb/Adverb/Conjunction과 immediate semantic application의 구분까지 jsource parser construction behavior와 일치한다.

<a id="p4--parser-time-name-resolution과-assignment-sequencing"></a>
#### P4 — parser-time name resolution과 assignment sequencing

- [x] enqueue는 ordinary NAME과 lookup metadata를 전달하고 extension 이름을 keyword로 만들지 않는다. F1의 ordinary-name/extension POS 회귀와 같은 계약이다.
- [ ] parser가 ordinary name을 stack에 넣기 직전에 현재 local/locale binding을 조회해 noun/verb/adverb/conjunction class를 얻는다.
- [ ] noun name의 by-value resolution과 일반 function/modifier name의 nameref semantics를 구분한다.
- [ ] jsource의 nameless modifier by-value 최적화는 언어 semantics와 분리하고 RustJ에서 필수로 복제하지 않는다.
- [ ] named Verb/Adverb/Conjunction이 primitive entity와 동일한 row 0–6 경로에 참여하게 한다.
- [ ] sentence 시작 시 전체 binding snapshot을 만들지 않고 observable right-to-left lookup/assignment sequencing을 보존한다.
- [x] assignment-target NAME을 enqueue to-name flags와 ParseValue::NameTarget으로 ordinary lookup과 분리한다. 전체 noun/multiple-name assignment와 scope 지원은 별도 미완료다.
- [ ] `=.` / `=:`의 symbol-table 선택과 assignment result semantics를 테스트한다.
- [ ] current POS를 가진 nameref가 later resolution 시 다른 POS로 바뀐 경우의 J-compatible error contract를 보존한다.
- [ ] extension builder(`conv` 등)의 shadow/rebind도 ordinary J name semantics를 따르게 한다.
- [ ] row 0–2 실행이 같은 sentence의 이후 parser-time name/locale/POS lookup에 영향을 주는 사례를 식별하고 parser-visible effect로 분류한다.
- [ ] deferred noun value가 뒤 row 3/4 modifier construction의 실제 operand value로 필요한 경우 정적 placeholder로 construction을 완료하지 않는다.
- [ ] v0에서는 이러한 dynamic parse dependency를 `RuntimeSemanticParse`/coverage fallback으로 보내고, 정적 compile 성공으로 오인하지 않는다.
- [ ] 추후 guard/multiversion을 추가하더라도 observable reduction/order/error semantics가 runtime semantic baseline과 같음을 요구한다.

- [ ] **P4-locative J 실행 의미(위 F1 연계):** direct는 지정 locale의 symbol을 읽고/쓴다; indirect는 현재 binding의 boxed locale string을 **해당 조회·대입 시점**에 읽는다; `name__`은 `name_base_`로 귀착한다. locative `=.`/`=:`는 explicit definition 안에서도 global이다. RHS는 원래 실행 locale에서 평가하고 지정 locale에 기록한다. locative로 호출한 function의 current-locale 교체·복원, locale search path, local frame 분리, observable NAME/POS/error/effect/assignment 순서를 테스트한다. 미정의 이름·잘못된 locale 값은 J의 실제 오류를 C oracle로 판정하며 미지원으로 포장하지 않는다. **수용:** pinned J C의 positive/negative·rebind·nested explicit-definition 차분; direct/indirect·읽기/대입·출처 span을 모두 보전해야 한다. 문법만 인정하거나 구문 뒤 일괄 `Unsupported`를 던지는 방식은 P4 통과 아님.

- [ ] **NP-03/06 (§3.7.3):** error-bearing lookup, local/locale/path·holder witness, late NameRef와 constructor/assignment 오류 순서를 검증한다(NP-V03/06).

**P4 완료 조건:** parser 결과가 spelling이 아니라 그 시점의 J binding, assignment state, parse row에 의해 결정된다.

- [ ] **NP-04/05 (§3.7.3):** J Graph/A3까지 name event 출처와 Observed/ProvenStable/GuardRequired/Unknown을 유지한다(NP-V04/05).

<a id="p5--construction-time-j-semantics와-compiler-analysis-facts-분리"></a>
#### P5 — construction-time J semantics와 compiler-analysis facts 분리

**구현 방향:** parser/J Semantic Construction IR의 `FunctionEntity` 자체를 compiler convenience를 위해 변형하지 않는다. 대신 그 immutable graph에서 **별도 J Graph IR을 파생**하여 applied topology와 syntax-derived optimization hint를 표현한다. J Graph IR은 graph analysis에 필요한 조기 abstract `GraphFacts(dtype/shape/rank)`를 가질 수 있지만, 이것은 실행 계약의 canonical resolved fact가 아니다. actual valence/effective-rank/cell/frame/agreement/check/effect/error/access/representation을 포함한 최종 call-time 사실은 Logical Execution IR의 `ResolvedCallFacts`/`ResolvedInstantiation`이 소유한다. lowering은 두 층이 공유하는 dtype/shape/rank가 일치하는지 검증한다. target 이후 정보는 `LoweringCapability/TargetFacts -> PhysicalDecision`에 둔다.

parser에서 **모든 의미 해석을 제거하지 않는다.** jsource modifier application이 그 자리에서 검증하고 result entity를 만드는 의미는 그대로 수행한다. 제거 대상은 target/call-dependent compiler facts다.

- [ ] pure/semantic `ModifierSemanticConstructor` interface를 두어 parser row 3/4가 J-defined construction validation과 result POS/entity 생성을 요청하게 한다. operand value가 runtime-dependent하면 같은 action을 runtime semantic parser에서 수행할 수 있어야 한다.
- [ ] parser-produced `FunctionEntity`는 identity/result POS/source operands/span과 **intrinsic `FunctionSemanticInfo`**를 immutable하게 소유한다.
- [ ] modifier construction facts는 해당 completed `FunctionEntity.semantic_info.construction`에 보존하여 재귀 traversal에서 바로 참조할 수 있게 한다.
- [ ] `"` constructor는 jsource `jtqq`와 동일한 noun/verb operand legality, rank/length/domain validation 및 requested-rank normalization을 **modifier application 시점의 construction semantics**로 처리한다. 이는 반드시 compile-time이라는 뜻은 아니며 runtime parser fallback에서도 같은 규칙을 사용한다.
- [ ] normalized requested rank 같은 node-intrinsic construction fact와 **actual argument rank를 이용한 effective rank/cell/frame 계산**을 분리한다. 후자는 call/Logical IR node의 `ResolvedCallFacts`에 둔다.
- [x] applied `/`는 completed derived entity로 만들고 `Verb.reduce` 같은 compiler migration boolean을 semantic identity에서 제거했다.
- [x] `Verb.reduce` 사용처를 제거했다. reduction identity/basis/fact inference는 completed `FunctionEntity`의 Insert 구조에서 Semantic Analyzer/Lowering이 유도한다.
- [ ] `Verb.rank` 필드/사용처 제거는 완료했다. 남은 일은 requested-rank를 construction fact로 정규화하고 call-time `ResolvedRankContract/CellApply`와 명시적으로 분리하는 것이다.
- [x] `Callable.reduce` / `Callable.rank` migration field를 제거했다. Analyzer는 shared `FunctionEntity` 구조를 직접 따라 execution basis/facts/outer rank boundary를 유도한다.
- [ ] innate rank, effective rank, frame/cell split, agreement/repetition, access, optimizer proofs가 parser entity 필드에 들어가지 않게 한다.
- [ ] architecture/device/lowering/cost metadata가 parser/`FunctionEntity.semantic_info`에 들어가지 않게 한다.

**P5 완료 조건:** parser는 J entity construction의 성공/실패와 completed semantic identity를 정확히 결정하지만, call-dependent 및 target-dependent compiler facts는 소유하지 않는다.

**2026-10-02 구현·검증 증거 (F0/P3/P5의 부분 완료):**

- [x] rank noun의 construction 검사를 rank → length → numeric audit 순서로 통일했다. scalar/vector만 허용하며, empty/4개 이상 operand와 잘못된 numeric operand의 J error class를 보존한다.
- [x] `semantic::rank_noun_contract`를 parser, analysis/facts, interpreter와 Logical IR reference executor가 공유한다. 고정 fuzz 정수 audit, 무한대/범위 밖 실수/`_.`의 jsource `vib` 처리 및 ±63 requested-rank clamp를 적용하되 FunctionEntity에는 원래 noun을 보존한다.
- [x] 중첩 괄호의 completed literal noun을 conjunction 및 noun-left fork construction에서 보존한다. 계산이 필요한 noun operand는 여전히 runtime semantic parsing coverage boundary다.
- [x] noun `/` 및 noun operand의 `@:`는 unreduced-stack syntax error 대신 constructor domain error를 반환한다. 합법적인 noun-left rank 등 미구현 form은 UnsupportedImplementation으로 구분한다.
- [x] 큰 diagnostic ErrorContext를 box로 분리했다. 기존 kind/span/blame/inner-context 우선순위는 유지하며 Error 크기 ≤32 bytes를 회귀 검사한다.
- [x] native Windows default/portable는 각각 208 passed / 17 ignored이며, fmt, clippy `-D warnings`, Python harness 11 tests를 통과했다. 17개 pending definition acceptance tests는 완료로 계산하지 않는다. 기존 formatting drift도 정리했다.
- [x] Windows 공식 C 배포본 일반·AVX2 각각에서 2,050문장 × direct/semantic-reference 경로가 불일치 0 / known deviation 0이다. word formation은 각각 6,618건(seed `20260927`, open quote `1109`) 불일치 0이다.

이번 실행 기준선은 공식 `build/w64.zip`의 release commit metadata `ded7793fe5795d79eda8e7138dce94aa056edf78`이다. `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`은 source-level parser 검토 기준으로 유지한다. 그 고정 소스의 Windows MSVC 빌드는 GNU C 확장 때문에 실패했으므로 이번 배포본 비교를 pinned-build 성공으로 표시하지 않는다. 각 `reports/frontend-*-windows.json`은 실제 DLL/실행 파일 SHA-256과 revision/platform을 기록하며, word harness는 revision을 하드코딩하지 않는다.

재실행: native Windows에서 `tools/check-windows.ps1` 후 `tools/check-frontend-windows.ps1 -ReferenceDirectory <j.dll/javx2.dll 폴더> -ReferenceRevision <확인한 40자리 commit> -SourceDirectory <jsource checkout> -SourceRevision <검토한 40자리 commit> -Avx2`를 실행한다. 기본 Python 3.13 경로는 `-Python`으로 변경할 수 있다. GitHub CI와 Linux tests는 실행하지 않았다. upstream 전체 suite와 CUDA 검증도 수행하지 않았다.

**현재 남은 gate:** intrinsic FunctionSemanticInfo의 최종 수렴, full noun/verb modifier·immediate constructor semantics, 모든 result POS/primitive coverage, full runtime `ptcol` reachable-state trace, 일반 locale/locative/definition-control scope, noun/multiple assignment target과 static/runtime dynamic-boundary 수렴이 남아 있다. **P2 rows 0–2 RuntimeParserHost/reinsertion과 지원 범위의 우측→좌측 name/assignment sequencing 자체는 이미 구현되었으므로 이를 미구현 항목으로 다시 세지 않는다.** 이 증거는 M2 전체 완료를 뜻하지 않는다.

- [ ] **NP-07 (§3.7.3):** J C와 Rust runtime·capture·Graph/A3 차분 및 효과·오류 순서 부정 테스트를 통과한다(NP-V07).

<a id="p6--differentialconformance-test-matrix"></a>
#### P6 — differential/conformance test matrix

- [x] 단계별 probe와 의미 정규화·source-table 비교 harness를 추가한다 (`frontend_probe`, `frontend_stage_conformance`).
- [x] tokenizer/enqueuer/parser 구현 파일을 분리하고 기존 scanner/semantic parser API는 compatibility export로 유지한다.
- [x] 단독/괄호 adverb·conjunction 및 최종 이름 할당을 syntax error 없이 `ModifierValue`와 실제 POS로 보존한다. named modifier 실행·전체 derived POS는 계속 미완료다.
- [x] normalization이 completed modifier 경계·ordered operand·boxed noun type/shape를 보존하고 모르는 atomic encoding을 거부하는 unit tests를 추가한다.
- [x] 새 stage suite의 native Windows j64/AVX2 실행 결과를 기록한다. 각각 7,014개 검사(선언 row 6,561조합, 함수/POS 구조 111건 포함), 불일치 0이다.
- [ ] runtime `ptcol`의 reachable stack context와 row actions/provenance trace를 C와 비교한다. 선언 `cases[]` 6,561조합 검사는 이 항목을 대체하지 않는다.

- [ ] 9개 parse row 각각의 최소 positive sentence를 jsource와 differential 비교한다.
- [ ] row precedence가 충돌할 수 있는 competing-pattern 문장을 추가한다.
- [ ] `+/ % #`, 2-verb Hook, 3/4/5개 이상 train을 구조 golden으로 비교한다.
- [ ] 연속 adverb/conjunction 및 derived modifier가 다시 modifier operand가 되는 사례를 추가한다.
- [ ] modifier constructor의 construction-time rank/length/domain error를 jsource와 비교한다.
- [ ] Verb/Adverb/Conjunction을 name에 할당한 뒤 사용하는 사례를 추가한다.
- [ ] sentence 중간 assignment/name lookup이 뒤 reduction의 class에 영향을 주는 사례를 추가한다.
- [ ] row 0–2의 effectful 실행이 왼쪽의 name/locale/POS resolution을 바꾸는 문장을 differential corpus에 포함한다.
- [ ] runtime-dependent noun이 modifier operand가 되어 construction success/error/POS가 runtime에 결정되는 사례를 포함한다.
- [ ] parentheses가 reduction boundary를 바꾸는 사례를 추가한다.
- [ ] one-word sentence path를 별도 regression으로 둔다.
- [ ] deterministic noun result와 J error class를 비교한다.
- [ ] assigned entity의 POS는 `4!:0`, 유용한 derived structure는 `5!:1`/ `5!:5`를 oracle로 비교한다.
- [ ] RustJ ParseTrace로 row id/input classes/span/result class를 golden화하되 jsource 내부 bitmask/주소/flags와 비교하지 않는다.
- [ ] 아직 lowering하지 못하는 합법 J form의 **parser success**와 이후 `UnsupportedImplementation`을 syntax error와 구분한다.
- [x] 현재 지원 범위의 differential suite를 native Windows local gate로 실행할 도구를 추가했다. GitHub CI는 사용자 지시에 따라 생략한다.

**2026-10-03 Windows 실행 기록:** 파일 분리와 `=.`/ill-formed name·number/modifier result POS 수정 후 default/portable 각각 **212 passed, 17 ignored**; fmt/clippy 통과, Python harness **17 passed**. 일반/AVX2 C oracle 각각 direct/semantic-reference **2,050문장**, raw word **6,618건**, 새 stage suite **7,014개 검사**에서 불일치 0이다. 17개 ignored definition tests와 stage report의 pending 목록은 완료가 아니다. 새 보고서는 `reports/frontend-{j64,avx2}-stages-windows.json`이며 기존 6개 value/word 보고서도 현재 binaries의 hash로 갱신했다. source review pin과 공식 Windows DLL revision은 앞 실행 기록처럼 서로 다르다.

**P6 완료 조건:** 지원 parser surface의 변경은 jsource observable differential + RustJ row trace golden 없이 merge되지 않는다.

<a id="p7--cutover와-legacy-parser-제거"></a>
#### P7 — cutover와 legacy parser 제거

- [ ] 새 9-row engine이 기존 parser/semantic golden을 모두 통과한다.
- [ ] Analyzer golden(`(+/ % #) y` 포함)이 새 parser output에서도 동일한 completed entity graph를 입력으로 받는다.
- [x] old `reduce_modifier_applications` 함수는 현재 source에 없다. row engine이 modifier reductions를 소유한다.
- [x] old `collapse_verb_trains` 함수는 현재 source에 없다. row engine이 train reductions를 소유한다.
- [x] 과거 flat-vector noun/verb application loop를 제거했다. 현재 Expr application 생성은 동일 9-row semantic action 안에 있으며 필요한 의미 구조라 유지한다.
- [ ] parser-only migration fields와 dead compatibility code를 제거한다.
- [x] parse/parse_analysis/parse_runtime_host가 동일 parse_context와 reduction engine을 공유한다. cfg(test) parse_runtime도 같은 parse_with→parse_context를 사용한다.
- [ ] parser 전환 후 전체 native Windows local 검증을 통과시킨다. GitHub CI는 생략한다.

**P7 완료 조건:** RustJ의 J parser semantics를 정의하는 코드 경로가 하나이며, 그 경로는 jsource-compatible class resolution + 9-row reduction + semantic constructor contract를 따른다.

<a id="p8--a1a2a3로의-integration-handoff"></a>
#### P8 — A1/A2/A3로의 integration handoff

P8은 **parser migration 선행 게이트가 아니다.** P0–P7에서 얻은 clean parser output을 기존 compiler milestones가 소비하도록 연결하는 후속 작업이다.

- [ ] PrimitiveSpec/ExtensionSpec의 target-independent semantic contract와 parser entity identity를 연결한다.
- [x] P8에서 J Graph IR을 parser output과 execution IR 사이의 명시적 compiler boundary로 추가한다.
- [x] `GraphForm/GraphHint`에서 J syntax-derived topology를 기록하고 execution lowering이 이를 소비하도록 한다.
- [ ] `GraphRuleRefs`를 PrimitiveSpec/derived-composition rule registry와 연결한다.
- [ ] `FunctionEntity.semantic_info`의 intrinsic facts와 Logical IR node의 `ResolvedCallFacts` 책임을 분리한다.
- [ ] `ResolvedCallFacts`에서 valence/effective-rank/cell/frame/agreement/repetition/type/shape/effect/error/access를 계산한다.
- [ ] `+/ % #` matrix golden에서 최종 `%`의 implicit CellApply2를 명시적으로 만든다.
- [ ] Logical Optimizer가 semantic graph를 보존한 채 `FusionCandidate::Mean` 등을 별도 proof/candidate로 만든다.
- [ ] built-in과 NN/array extension op가 동일한 LogicalOp/lowering interface로 진입하게 한다.
- [ ] architecture-specific 구현 정보는 `LoweringRegistry × ArchitectureTarget`에서만 결합한다.
- [ ] concrete GPU model 정보는 `DeviceProfile`, 후보 선택 성능 정보는 `CostProfile/RuntimeProfile`로 분리한다.

- [ ] P8 지원 범위별 Program→J Graph→A3 admission을 검증하고, 본문 재분석 경계와 NAME proof/guard 요구를 명시한다. ControlNode.go만으로 CFG를 만들지 않는다. 이번 재감사 절과 연결한다.

**P8 완료 조건:** parser를 다시 변경하지 않고 A1/A2/A3의 semantic analysis, CPU/GPU lowering, NN extension을 확장할 수 있다. dynamic J parsing이 필요한 form은 compiler coverage와 runtime semantic fallback의 명시적 경계로 남는다.

<a id="진행-규칙"></a>
#### 진행 규칙

- F0–F2 + P0–P7을 frontend/parser migration의 **single source of truth**로 사용한다. P8은 기존 A1/A2/A3 체크리스트와 함께 추적한다.
- 이후 frontend 작업의 진행상황 보고는 반드시 이 체크리스트의 phase/item 기준으로 보고한다. 새 작업이 생기면 임시 TODO를 코드에만 남기지 않고 먼저 해당 phase에 checklist item으로 추가한다.
- jsource와의 차이를 발견하면 "RustJ 구현 편의"로 봉합하지 않고 해당 phase의 compatibility defect로 기록한다. observable semantics가 같다는 differential proof가 있기 전에는 의도적 차이로 간주하지 않는다.
- frontend 구현은 원칙적으로 F0 → F1 → F2 → P1 → P2 → P3 → P4 → P5 → P6 → P7 순으로 진행하되, 앞 phase interface를 깨지 않는 oracle/test 작업은 병행할 수 있다.
- 완료 즉시 같은 변경에서 `[ ] -> [x]`로 갱신한다.
- 부분 구현을 완료로 표시하지 않는다. 각 phase의 완료 조건을 만족해야 phase 완료로 본다.
- jsource와 의도적으로 다른 observable parser behavior가 필요해지면 구현 전에 rationale과 semantic impact를 이 문서에 기록한다.
- downstream IR 요구사항은 P8/A1/A2/A3에 추가하고 parser entity에 임시 compiler field로 밀어 넣지 않는다.


<a id="a06--structured-diagnostic-context"></a>
### A0.6 — Structured diagnostic context

- [x] J-compatible machine error와 diagnostic provenance/context를 분리한다.
- [x] `ErrorContext`에 phase/span/original word index/current name/operation/valence/argument summary/notes를 표현할 수 있게 한다.
- [x] inner context 우선 merge 규칙을 만들어 outer stage가 더 정확한 span/blame/context를 덮어쓰지 않게 한다.
- [x] `DiagnosticAnalyzer`와 Python-style renderer를 분리한다.
- [x] enqueue word에 original word index를 보존하고 parser diagnostic으로 전달하기 시작한다.
- [x] runtime monad/dyad failure에 executing primitive, valence, x/y type/shape/rank summary를 붙인다.
- [x] interpreter execution과 compiler `analyze_diagnostic`이 동일 context/error infrastructure를 사용한다.
- [x] stable `eval()/analyze()/parse()` machine API는 context wrapper를 제거하고 기존 J error variant/kind를 유지한다.
- [x] F1 `EnqueuedWord`가 byte span과 original word index를 canonical provenance로 직접 소유한다.
- [ ] F2 parser stack entry가 original word index를 모든 reduction 동안 보존하고 jsource `infererrtok`에 대응하는 blame inference를 구현한다.
- [ ] Hook/Fork/derived modifier 실행 시 failing semantic entity의 compact linear representation을 diagnostic context에 넣는다.
- [ ] rank/agreement failure analyzer가 effective cell/frame facts를 사용해 J처럼 어느 frame/shape가 불일치하는지 구조적으로 설명한다.
- [ ] index error analyzer가 offending selector/index/path를 작은 structured detail로 보존하고 설명한다.
- [ ] domain error analyzer가 primitive contract와 argument dtype/value summary를 이용해 구체적 원인을 설명한다.
- [ ] assembly error analyzer가 cell-result type/shape join failure 위치를 설명한다.
- [ ] lowering/backend failure도 Logical IR source origin + semantic operation context로 동일 renderer에 연결한다.
- [ ] diagnostic context가 큰 noun payload를 소유/복사하지 않는지 테스트한다.
- [ ] 최신 J error corpus의 대표 사례를 RustJ diagnostic golden으로 추가하되 문구 자체보다 semantic information completeness를 검증한다.

**완료 조건:** AOT/interpreter/JIT/backend 어느 경로에서 실패해도 J error class는 안정적으로 유지되고, 동일한 structured context → analyzer → renderer 경로로 source 위치와 semantic 원인을 설명할 수 있다.

- [x] 정의 body 원문 좌표와 caller 좌표를 분리한 source-unit/map/frame chain과 내부 실패 위치·호출 chain 회귀. 아래 A0.6 후속 절의 named file/root provenance 계약을 따른다.
- [x] 현재 frontend 연결 경계의 J failure/admission/verifier/backend 분류와 handler/replay 권한. 아래 A0.6/P8 절의 범위와 남은 legacy/backend 확장을 따른다.

<a id="a1--j-semantic-construction-ir--functionentity-경계"></a>
### A1 — J Semantic Construction IR / FunctionEntity 경계

- [ ] 현재 `semantic.rs`가 noun/verb/adverb/conjunction과 derived composition을 얼마나 보존하는지 감사한다.
- [x] jsource `p.c::cases[]`의 parser function-construction rows를 기준으로 immutable shared `FunctionEntity` graph를 만든다.
- [x] `/` ADV와 `"` CONJ를 source operator identity로 보존하고 parser application 결과의 parent로 사용한다. modifier별 `Insert/Rank/...` semantic node enum을 만들지 않는다.
- [x] 큰 derived function/train의 `Arc<FunctionEntity>` sharing test로 subtree deep-copy가 없음을 검증한다.
- [x] semantic FunctionEntity와 runtime/backend executor specialization의 층을 분리한다.
- [ ] primitive verb identity와 monad/dyad valence를 명시한다.
- [x] Hook / Fork parser-production parent를 first-class semantic identity로 표현하고, train은 별도 `Train` node 없이 shared Hook/Fork graph로 구성한다.
- [ ] adverb/conjunction/hook/trident application으로 생긴 DerivedEntity와 result part of speech(Verb/Adverb/Conjunction)를 보존한다.
- [ ] boxed noun의 ordinary-data 사용과 modifier-context gerund interpretation을 구분한다.
- [ ] `::` adverse와 `:.` obverse처럼 forward graph 밖의 latent error/inverse semantics를 보존한다.
- [ ] rank-derived verb와 cell/frame 의미를 Semantic Analyzer가 분석할 수 있게 표현한다.
- [ ] rank conjunction의 verb"rank-noun, verb"verb, noun/gerund"rank forms를 source operand 품사 손실 없이 표현한다.
- [ ] Infinite/Absolute/Relative RankSpec과 monad/left/right rank triple을 보존한다.
- [ ] name reference/binding/version과 source span을 필요한 범위에서 연결한다.
- [ ] `NameRef.expected_part_of_speech`와 runtime lookup POS mismatch의 domain error를 모델링한다.
- [ ] sentence 전체의 name environment를 선행 snapshot하지 않고 우측→좌측 assignment/name lookup sequencing을 보존한다.
- [ ] explicit definition의 DefinitionCode와 invocation CallFrame을 분리한다.
- [ ] local slot hint와 실제 local binding을 구분하고 unbound local candidate의 locale fallback을 보존한다.
- [ ] `=.` local assignment와 `=:` public/locale assignment를 구분한다.
- [ ] assignment가 namespace write effect와 assigned-entity result(noun/verb/adverb/conjunction)를 동시에 갖는지 테스트한다.
- [ ] primitive contract를 semantic node에 연결한다.
- [ ] J dyadic rank의 prefix frame agreement와 residual-frame repetition을 명시적으로 테스트한다.
- [ ] zero-cell rank execution의 fill-cell/prototype result type·shape semantics를 테스트한다.
- [ ] rank cell 결과의 type/shape가 다른 경우 J result assembly(type join, shape join, framing fill, assembly error)를 테스트한다.
- [ ] uniform cell-result proof가 있을 때만 rank map을 고정-shape parallel output으로 낮춘다.
- [ ] boxed와 sparse를 physical encoding이 아닌 J-visible semantic representation으로 보존한다.
- [ ] comparison tolerance/`!.` fit context와 J error precedence를 semantic contract에 포함한다.
- [ ] empty operand에서의 context-sensitive type/domain semantics를 dense atomic dyad golden test로 검증한다.
- [ ] Semantic Analyzer가 source parser 없이 J Semantic Array IR만으로 분석 가능하게 한다.
- [ ] Semantic Analyzer / Lowering이 semantic structure를 Logical Array IR / Plan으로 낮추는 테스트를 작성한다.
- [ ] fork/hook의 J-compatible observable execution order를 보존하고 pure/speculatable proof가 있을 때만 branch 병렬화를 허용하는 golden test를 둔다.
- [ ] reduction derived verb와 rank-conjunction-derived verb를 대표 golden test로 둔다.

완료 조건: Semantic Analyzer를 scanner/parser 없이 테스트할 수 있으면서도 hook/fork/train/rank 및 derived verb/adverb/conjunction의 의미 구조가 분석 입력에 남아 있다.

<a id="j-graph-implementation-checklist"></a>

<a id="a15--j-graph-ir--jaxa-array-operation-graph-ir"></a>
### A1.5 — J Graph IR (과거 JAXA Array Operation Graph 연구의 계승)

**목표:** JAXA의 핵심 연구 표면을 first-class compiler IR로 만든다. parser가 만든 immutable FunctionEntity를 actual noun application과 결합하여, J 문법 자체가 제공하는 graph topology와 optimization hint를 잃지 않는 applied operation graph를 만든다.

> **현재 위상:** `j_graph_ir` v0.10은 **explicit applied-operation graph + access-pattern basis + witnessed rewrite/resource analysis** 단계다. `@:`/Hook/Fork 내부 stage/branch가 실제 `ValueId` node로 전개되고, `/`, `"`, `\`의 Reduce/CellApply/Window 구조가 graph-level basis/resource identity로 보존된다. stage별 GraphFacts/use-count/analyzability와 `ResourceExprGraph`, witnessed rewrite candidate, conservative source-vs-replacement resource evaluation, existing `LoweringRegistry + TargetCapabilities`에 대한 target-only feasibility bridge가 존재한다. 아직 없는 것은 full rewrite-specific shape algebra, executable WindowView lowering, fusion-candidate별 lifetime extension, resolved TargetProfile/ResourceEstimate/CostProfile, 실제 candidate selection/partition이다.

<a id="graph-prior-art-followup"></a>

<a id="2026-10-05-선행연구-후속-작은-basis와-composition-algebra"></a>
#### 2026-10-05 선행연구 후속: 작은 basis와 composition algebra

**감사 기준:** GitHub `main`의 `b00f2263decb4777e84f7670cc3bbd2536618f80`에서 문서와 `src/j_graph_ir.rs`를 대조했다. 이전 대화의 선행연구 반영 커밋은 `84b8546`이다. 최초 감사는 설계 계약을 정리했으며 구현 완료 선언이 아니었다. 아래 GF2/GF3의 후속 구현과 검증은 별도로 기록한다. 업로드된 `붙여넣은 텍스트(1).txt`는 이 작업 환경의 로컬 경로/실행 도구 오류로 읽지 못했으므로, 복원한 대화와 사용자가 명시한 후속 항목을 기준으로 한다. 첨부 원문 대조는 미완료다.

| 항목 | 확인한 현재 상태 | 이번 결정 / 남은 구현 |
|---|---|---|
| 작은 Graph Basis + composition + witness | basis layers, Pipeline/Hook/Fork region, witnessed rewrite seam 있음 | 조합별 새 op를 증식시키지 않는다 |
| first-class Scan | GF3의 독립 GraphBasisKind::Scan 및 Boolean atomic prefix identity 후보/검증기 추가; 원본 Window→operand 보존 | numeric/rank/representation 확장 및 실행·reassociation·parallel-prefix 허가는 후속 |
| vertical / horizontal / nested | GF2의 공통 composition 분석 sidecar/verifier 추가; pipeline, branch/join 및 operand path 보존 | 법적 독립성 witness, noun-left graph 전문화와 실행 연결은 후속 |
| fusion algebra / registry | GF4의 4개 research schema와 registry/envelope/verifier 추가; 기존 E. witnessed rewrite와 provenance seam 공유 | 일반 equivalence proof·resource transfer·target query·실행 선택은 후속 |
| symbolic Work / Depth | GF5의 독립 symbolic domain과 ordered map/reduce/region 및 Scan identity 모델 추가 | 일반 rank/window·법적으로 입증된 parallel 모델·실제 target cost와 연결은 후속 |
| multiversion | specialization/guard 설계와 runtime baseline 있음 | 버전 선택·무효화·bounded cache 구현은 장기 후속 |
| streaming / inspector-executor | window/access/resource seam은 존재 | streaming 계약과 inspection plan은 장기 설계 후속; 실행 완료 아님 |

**Scan 계약.** Scan은 모든 prefix를 독립적으로 재계산하는 Window→Reduce와 다른 알고리즘 구조이므로 first-class Graph Basis로 보존한다. 원래 PrefixInfix FunctionEntity, valence, rank/cell boundary와 source provenance는 유지한다. 일반 `u\\`와 dyadic infix를 Scan으로 분류하지 않는다. insert-compatible monadic prefix조차 reducer, prefix 방향·길이·shape/assembly, empty/singleton, identity 사용, integer overflow/promotion, floating-point 결과, domain/error/effect 순서의 witness가 필요하다. Unknown이면 Window→operand 구조를 유지한다. associativity나 purity를 spelling만으로 추정하지 않는다. Scan identity 보존과 reassociation/parallel-prefix 허가는 별개다.

**Composition 분석.** Vertical은 producer→consumer edge, Horizontal은 동일 logical input을 사용하는 독립 consumer 관계, Nested는 rank/cell/segment 내부의 computation boundary다. 한 graph는 세 관계를 함께 가질 수 있으므로 배타적인 enum 하나나 `layers` 목록만으로 topology를 대체하지 않는다. sidecar는 node/region/edge identity, input occurrence, fan-out/use-count, live-across와 effect/error dependency를 참조한다. ordinary fork는 horizontal 후보일 수 있지만 h→f→g의 observable 순서와 constructor subtype을 보존한다. capped fork는 pipeline, noun-left fork는 retained noun + h로 구분한다. nested classification은 flattening 허가가 아니다.

**Fusion registry.** 기존 rewrite witness/verifier를 재사용하되 target lowering registry와 별도 역할로 둔다. 각 rule은 stable ID/version, source basis+composition pattern, replacement graph, relevant call facts, semantic proof obligations, witness/provenance mapping, fan-out/retained-value 변화, symbolic resource/work-depth transfer, target capability query를 선언한다. discovery → legality → target feasibility → profitability → selection을 분리한다. Unknown은 불법을 뜻하지도, 합법을 뜻하지도 않는다. Map→Map, Map→Reduce, Map→Scan 및 common-input Map+Map은 첫 후보 연구 목록이며 현재 지원 선언이 아니다. fusion이 shared producer를 복제하거나 lifetime을 늘릴 수 있으므로 cost 감소를 기본 가정하지 않는다. 실행 optimizer/새 executor 도입 없이 먼저 analysis-only seam을 만든다.

**Symbolic Work/Depth.** logical atom/state resource domain과 별도로 Work(총 연산량), Depth(의존 경로 길이)를 유지한다. 둘 다 symbolic extent와 operator 비용, Unknown/provenance를 보유한다. sequence는 Work/Depth를 합산한다. 법적으로 독립인 branch는 Work 합, Depth max + join을 사용할 수 있지만 effect/error dependency가 있으면 ordered dependency를 유지한다. Map은 cell work를 extent에 곱하고 nested cell depth를 보존한다. ordered Reduce/Scan baseline과 reassociation이 입증된 parallel candidate의 식을 별도로 계산한다. unit-cost associative operator라는 조건 아래 work-efficient tree candidate는 O(n) Work/O(log n) Depth일 수 있으나 이를 모든 J reducer에 부여하지 않는다. empty/singleton은 별도 case다. wall-clock latency·launch·traffic·transfer·synchronization은 CostEstimate이며 Work/Depth로 대체하지 않는다.

**장기 후속 계약.** multiversion은 relevant-fact key·binding dependency·guard·bounded cache/widening을 연결하며 guard miss 뒤 observable effect를 재실행하지 않는다. streaming은 chunk boundary, carry/state, ordering, termination, bounded memory와 materialization contract가 증명되는 region에서만 후보를 만든다. inspector-executor는 indirect access를 조사하는 비용·effect·binding/array mutation·alias invalidation과 검사 결과의 witness lifetime을 명시한다. 세 항목은 full-J restriction도 첫 M4 CPU slice의 선행조건도 아니다.

**선행연구를 적용하는 범위.** Futhark의 fusion 설명은 vertical/horizontal 분석의 직접 비교 자료이며, 2026 scan-scatter 작업은 fusion algebra가 확장될 수 있음을 보여 준다. 개별 compiler의 지원/금지 규칙을 RustJ의 영구 법칙으로 복사하지 않는다. Work/Span 자료는 분석 도메인의 비교 근거이며 J numeric/error semantics의 증명이 아니다.
- https://futhark.readthedocs.io/_/downloads/en/v0.25.4/pdf/
- https://futhark-lang.org/blog/2026-03-24-scan-scatter-fusion.html
- https://github.com/diku-dk/futhark-book/blob/master/parallel-cost-model.rst

<a id="common-array-algebra-hypothesis"></a>
<a id="2026-10-11-연구-가설-배열-계산의-공통-접근-대수와-증명-가능한-재작성"></a>
#### 2026-10-11 연구 가설: 배열 계산의 공통 접근 대수와 증명 가능한 재작성

> **위상: 연구 가설 / 분석·문서 단계.** 아래 `Domain–Access–Compute–Combine`(DACC)는 기존 학계가 정식으로 채택한 이름이나 RustJ의 확정 IR이 아니다. J의 언어 의미론 또는 현재 `GraphBasisKind`·`ExecutionBasisKind`를 대체하지 않는다. 목적은 **여러 배열 연산에서 재사용할 수 있는 공통 분석 관계와 동치 규칙이 실제로 존재하는지 반증 가능한 실험으로 확인**하는 것이다. `GF2~GF6`의 기존 graph composition, witness, rewrite 후보 및 work/depth 계약을 그대로 출발점으로 삼으며 새로운 필수 컴파일 단계, 별도 AST, 새 `AA-*` 체크리스트를 지금 만들지 않는다.

<a id="1-문제-정의-표현-가능성동치성최적화-유용성은-서로-다른-주장"></a>
##### 1. 문제 정의: 표현 가능성·동치성·최적화 유용성은 서로 다른 주장

- **표현 가능성(coverage):** 어떤 연산 `P`를 공통 구조의 항목으로 기술할 수 있는가? 극단적으로 모든 연산을 하나의 불투명한 `Compute(P)`로 처리하는 것은 참이지만 아무 최적화 지식도 제공하지 않으므로 성공이 아니다.
- **동치성(equivalence):** 원본 `P`와 대체 `Q`에 대해 모든 허용 입력에서 J가 관찰하는 결과값·타입·Shape·오류·효과·순서·상태가 같은가? 수학적 값의 동등성만으로는 부족하다.
- **재사용성(reuse):** 연산별 전용 패스 없이 같은 `Domain/Access` 분석기, 영역 역전파 규칙 또는 guarded rewrite가 **서로 다른 연산군 두 개 이상**에 적용되는가? 공유할 수 없는 예외 나열이라면 공통 대수라는 가설은 약해진다.
- **실용성(profitability):** 더 적은 수의 기저 연산으로 표현되더라도 알고리즘 정체성을 잃고 특수 라이브러리 경로가 막히면 실패다. 실행 가능성, 계산 Work/Depth, 물리 메모리·I/O 비용, target capability는 동치성 이후 별도로 평가한다.
- **완전성(completeness)은 별도 고난도 의제:** SPORES가 논의하는 자신의 선형대수 부분언어에서의 완전성 주장은 full-J의 동치 재작성에 대한 완전성 증명이 아니다. RustJ는 이를 선결 요구사항으로 두지 않는다.

**정확한 연구 질문:** “J의 Rank/Cell/Frame 및 observable semantics를 보존한 채, Map·Reindex·Reduce·Scan·Window/Convolution·Gather를 아우르는 *공통 구조적 접근 관계*를 얻고, 그 위에서 검증 가능한 여러 변환과 I/O 수요를 같은 규칙으로 유도할 수 있는가?”

<a id="2-논리적-배열-모델과-좌표--값-모델의-한계"></a>
##### 2. 논리적 배열 모델과 ‘좌표 → 값’ 모델의 한계

가장 작은 순수 Dense 후보 모델은 `A : Index(shape(A)) → Atom(dtype(A))`다. `Index(s)`는 각 차원이 `0..s_i-1`인 유한 좌표 집합이다. 이는 물리 `strides/offset/buffer/placement`와 무관하므로 transpose·indexing·shape 변환을 기술하기 좋다. 그러나 **이 정의만으로 J noun을 완전히 모델링하지 못한다.** 특히 크기가 0인 배열은 좌표·원소를 전혀 갖지 않아도 J-visible 타입·Shape·fill/prototype 관련 결과가 달라질 수 있으며, boxed/sparse 표현과 value semantics, tolerances/fit, 불리언과 숫자 promotion 및 오류 의미론이 남는다.

따라서 개념적 논리값은 `LogicalArray = (JType, Shape, OrderedAtoms, JVisibleRepresentation/FillFacts)`이고, `A(i)`는 **값 의미의 한 투영**일 뿐이다. Rank의 빈 Frame에서 virtual Cell을 이용한 결과 형식 결정은 원소가 없는 경우에도 별도의 **ResultAssembly/Prototype 의무**로 남겨야 한다. `JEntity`의 noun/function distinction과 arbitrary derived verb에 noun-style shape/rank를 부여하지 않는 규약은 불변이다.

<a id="3-공통-구조-후보-dacc--j-semantic-envelope"></a>
##### 3. 공통 구조 후보: DACC + J-semantic envelope

아래는 **분석 표면의 개념 스키마**로, 실제 Rust 구조체/새 IR 정의가 아니다.

~~~text
ArrayOperationContract (candidate analysis view)
  ResultDomain: ShapeExpr + result logical index domain
  Application:  Requested/RealizedRank + FrameDomain + CellDomain
  Iteration:    output, contraction, segment/window, ordered-prefix domains
  Access:       input-slot -> [index relation / region transformer]
  Compute:      scalar/cell payload OR retained structured primitive identity
  Combine:      identity, order, direction, state/carry, numeric contract
  Assembly:     J result cell assembly, fill/prototype, empty-frame obligations
  Effects:      source/name/alias/effect/error dependencies and provenance
  Evidence:     Known / Guarded / Unknown; witnessed facts and guards
~~~

- **Domain:** 결과 원소와 `Cell/Frame`이 존재하는 논리 좌표·분할; 반복 변수 중 출력축·계약축·Window축을 구분한다. `Rank` conjunction은 여전히 **J가 어떤 Cell에 함수를 적용하는지**를 정의한다. `Domain`과 `Shape`만으로 Rank를 대체하거나 MLIR 반복차원의 `parallel/reduction` 구분을 J Rank와 동일시하지 않는다.
- **Access:** 입력 occurrence 및 값/좌표 사이 관계. `CorrespondingElements`(Map), `IndexTransform`(Transpose/Slice), `ReductionAxis`, `WindowRelative`, `CellRelative`, `Indirect`, `SearchDependent`, `Unknown` 같은 기존 `GraphOperationContract::access` 분류를 재사용하되, 그것은 **정확한 역함수나 데이터 지역성 증명**이 아니다. 필요한 경우 별도 규칙과 witness를 부가한다.
- **Compute:** 원소/Cell의 값 변환. 모든 연산의 내부를 무조건 `Map+Reduce`로 평탄화하지 않는다. FFT·Sort·Search·Scatter·Dynamic Gather·Attention·Convolution은 알고리즘 정체성을 가진 구조적 primitive로 보존하고, 증명된 `access/decomposition contract`가 있으면 **후보로만** 펼친다.
- **Combine:** 접기/축약 방향, 시작값/identity, 연산 순서, Scan carry, 통합 규칙. 일반 J `u/`와 `u\\`에서 `u`를 결합법칙이 성립하는 연산이라고 가정하지 않는다. 삽입/reduction의 원래 J 평가 방향·승격·오류 순서를 보존하는 순서 지정과 병렬 결합 가능성은 다르다.
- **Envelope:** 의미론적 correctness의 경계. 원소값만 같아도 J의 Name late binding, Hook/Fork 순서, Fill, comparison tolerance, overflow/retry/promotion, error/side effect가 다르면 rewrite는 불법이다. 증거 없는 `Unknown`은 pass가 합법적이라고 해석할 수 없다.

현재 Graph IR `GraphBasis`, `CompositionRelation`, `GraphFacts`, `AccessContract`, `GraphRewriteProvenance`/`FactWitness`, A3 `ExecutionBasis`/`EffectSummary`/`SemanticCheck` 위의 **분석 sidecar / 계약 투영**으로 출발한다. Physical `BufferId`, concrete tile/strides, file byte offset, device schedule은 여기 넣지 않는다.

<a id="4-여섯-연산군에서-공통-구조를-유도하는-예"></a>
##### 4. 여섯 연산군에서 공통 구조를 유도하는 예

아래 `i,j,k` 표기는 **수학적 의사 표기**이며 J 구문 또는 실현된 RustJ lowering이 아니다.

| 연산군 | 결과 Domain / 입력 Access / Combine | 재사용할 수 있을 것으로 예상하는 분석 |
|---|---|---|
| Elementwise | `C[i,j] = f(A[i,j],B[i,j])`. 대응하는 입력 좌표, 독립 출력축 | 동일 위치 region transfer; guarded Map→Map fusion |
| Transpose / Reshape / Slice | `B[i,j] = A[j,i]`처럼 IndexTransform. Reshape는 Shape/atom-order 규칙, Slice는 범위·fill/error 조건 포함 | Index-map composition; 요청 출력 영역을 입력 영역으로 역상 추적 |
| Reduce | `B[i] = fold_ordered_k (A[i,k])`. 출력축 `i`와 결합축 `k`를 분리 | 필요한 reduction 축의 역방향 수요; 결합 방향·numeric law 증명 |
| Scan | `B[i,k] = prefix_ordered_{0..k}(A[i,*])`. Prefix dependency/carry 명시 | 공통 축 역할 분석; Window→반복 접기와 *다른* first-class Scan identity 보존 |
| Convolution / Window | `Y[n,o,h,w]`는 입력 채널과 kernel offset이 정의하는 유한 접근 집합에서 값/가중치를 읽음 | Access halo와 연산량·I/O 요구량, 조건부 WindowView/Contract decomposition |
| Gather / Indirect / Search | `B[i] = A[index[i]]`. 실제 index 값이 접근 영역을 결정; lookup/search 오류·중복 영향 | Known/Guarded/Unknown access, inspector-executor 후보, 보수적 full/opaque fallback |

**행렬곱 유도 예시.** `C[i,j] = ordered_sum_k A[i,k] * B[k,j]`를 결과축 `i,j`, 계약축 `k`, 입력 접근 `(i,k)` / `(k,j)`로 기술한다. `C[10:20,30:40]`의 데이터 결과만 요구하면 논리 수요는 `A[10:20,:]`, `B[:,30:40]`가 된다. 다만 실제 J `+/ . *` 등 내적의 숫자 형식·연산 순서·오류 의미론을 동일하다고 **가정하면 안 된다**. 위 식은 정적 접근 분석의 예이며 동치 rewrite 승인 증거가 아니다. Shape/Cell, 정렬된 `k` 순서, source-level checks가 별도다.

**Convolution 유도 예시.** 특정 출력 사각 영역이 필요하면 kernel/stride/dilation/padding·group/channel 역할에서 해당 입력 좌표 및 halo를 계산한다. Direct Conv, WindowView→Contract, im2col→GEMM은 서로 다른 후보일 뿐이고, padding/fill·boundary/stride·layout meaning·숫자 누적 순서·activation/save/adjoint semantics를 확인하기 전에는 동치라고 승인하지 않는다. 원본은 GraphBasis::Structured(Conv)로 남긴다.

**Rank 유도 예시.** `u"r`는 Frame 위에 각 Cell에 `u`를 적용하고 결과 Cell을 J 방식으로 조립한다. Frame 크기가 0일 때 데이터 셀을 실제 실행하지 않더라도 결과 dtype/shape/fill을 유도해야 하는 경우가 있으므로 `Demand(Value)=None`이 `Demand(Prototype/Checks)=None`을 뜻하지 않는다. Rank는 `Map`과 닮아도 **J의 적용·조립 연산자**로 독립적으로 검증한다.

<a id="5-공통-재작성-후보--모두-proof-obligation이-있음"></a>
##### 5. 공통 재작성 후보 — 모두 Proof Obligation이 있음

| 규칙/후보 | 기대 이득 | 필수 합법성 증거 / 실패 사례 |
|---|---|---|
| `Map(f, Map(g,A)) → Map(f∘g,A)` | 중간 배열 제거 / fusion | 순수성, 동일 evaluation multiplicity, 각 원소 error ordering, dtype/shape/overflow/provenance. 공유 `g(A)`가 다른 consumer에도 사용되면 계산 복제가 이득 아닐 수 있음 |
| `IndexMap(p,IndexMap(q,A)) → IndexMap(q∘p,A)` | 중간 view/reindex 단계 축소 | 정확한 합성 좌표·정의역/범위, negative/out-of-bounds, fill·Rank/Shape 계약, index-dependent errors |
| `Map(f,IndexMap(p,A)) ↔ IndexMap(p,Map(f,A))` | Materialization 생략 | `f` 순수·total이고 관찰 가능한 오류 순서/중복 적용이 바뀌지 않을 것, J array agreement 및 shape 조립 증명 |
| `Map→Reduce` / `Map→Scan` | 중간 buffer 없이 소비 | 동일 ordered fold/prefix와 callback/error 순서, accumulator/carry 및 empty/singleton, J numeric promotions; 자동 reassociation 불허 |
| Fold/Scan 병렬 Tree | Work/Depth 변화 | associativity/identity와 J-visible 부동소수점·정수 overflow/promotion·오류까지 **해당 dtype/모드에서** 입증; 단순한 수학적 결합법칙만으로 불충분 |
| Conv→WindowView/Contract 또는 im2col+GEMM | 특수 커널·접근 변환 | kernel geometry, padding, numeric order, dtype, alias/lifetime, output shape, partial read semantics 일치 증거 |
| Common producer read-region union | 동일 backing의 I/O 읽기 중복 감소 | source identity/version/snapshot, 순서/오류/효과 안전성, 실제 codec/read-chunk amplification 및 buffer lease |
| Dead input-region pruning | 필요한 byte-range 최소화 | 원래 J-visible check·effects·name binding·오류를 제거하지 않았는지; 데이터 결과가 필요 없다는 것과 읽기 효과가 없다는 것은 다름 |

**대표 반례 corpus(가설의 필수 반증 시도).** (a) floating-point `(a+b)+c ≠ a+(b+c)`; NaN/tolerance, (b) overflow/promotion 가능한 integer sum, (c) nonassociative `-/`와 order-sensitive Scan, (d) empty Frame에서 결과 prototype/shape와 virtual Cell, (e) boxed/sparse의 fill/representation difference, (f) dynamic gather의 index 값·bounds check, (g) J `1!:` read의 결과가 사용되지 않아도 error 발생, (h) Hook/Fork의 J right-to-left NAME/effect/error precedence, (i) aliased/mutable weight와 source-version 변경, (j) FFT/IFFT scale 및 adjoint 별도 정의. 하나라도 일반 규칙의 반례가 나오면 규칙 적용 조건을 좁히거나 기각한다. **반례를 예외로 숨기거나 J 의미를 제한해 맞추지 않는다.**

<a id="6-같은-대수에서-역방향-arrayregion-demand를-얻는-법"></a>
##### 6. 같은 대수에서 역방향 ArrayRegion demand를 얻는 법

필요한 결과 좌표 집합 `R_out`과 입력 slot `s`의 Access 관계 `Access_s(o) ⊆ InputIndices`를 안다면 **값 수요**의 sound over-approximation은 다음과 같다.

~~~text
Need_s(R_out) ⊇ Union_{o ∈ R_out} Access_s(o)
For multiple consumers: Need(source) = Union of their needs.
If relation is not justified: Unknown/Full (never unsafely narrower).
Additional obligations: dtype/shape/prototype, checks, errors,
effect and ordering liveness survive independently of value demand.
~~~

- Elementwise는 대응 좌표를 역전파한다. Transpose/Slice는 증명된 inverse index mapping이 있을 때만 좁힌다.
- Reduce는 요구 출력에 영향을 주는 **해당 reduction 축 전체**를 포함한다. Scan의 특정 prefix는 해당 prefix dependency/carry를 포함한다. 비결합적 Scan을 블록 간 독립 부분합으로 대체하지 않는다.
- Convolution은 입력 채널과 receptive-field halo를 포함한다. Gather는 index 데이터 자체와 실제 대상 좌표가 필요하며, index를 읽기 전에는 `Unknown`일 수 있다.
- 최종 `Full` 요구는 전체 논리 데이터를 언젠가 사용한다는 뜻이지 **전체 데이터를 한 번에 RAM에 올려야 한다는 의미가 아니다**. Out-of-core streaming/tiling의 가능성은 별도의 결합·순서·state·effect proof에서 판단한다.
- I/O Source 계약과 결합할 때에만 `ArrayRegion`을 실제 storage chunk/codec unit/byte ranges로 바꾼다. `RegionReadable`은 기능(capability)이지 빠른 랜덤 접근 성능 보장이 아니다. [8.5 I/O 계약](./09-memory-alias.ko.md#out-of-core-io-contract) 및 현재 논의한 `with` 의미론적 계약 경계를 따른다(후속 `8.5.5`가 병합되면 같은 원칙으로 연결).

이 분석은 **역방향 index-map 분석**이라는 측면에서 MLIR Linalg의 “iteration subset ↔ accessed data subset”과 연결되지만, J observable semantics와 정적 source-binding의 안전성은 RustJ가 별도로 책임진다.

<a id="7-선행연구-비교--동일성-주장이-아니라-부분적-증거"></a>
##### 7. 선행연구 비교 — 동일성 주장이 아니라 부분적 증거

| 연구/시스템 | 실제 입증·표현한 초점 | RustJ로 옮길 수 있는 부분과 차이 |
|---|---|---|
| [SPORES — Wang et al., 2020](https://arxiv.org/abs/2002.07951) | 선형대수를 관계대수로 변환해 equality saturation; 자기 대상 언어/규칙 범위에서 완전성 연구 | “배열의 SQL” 비유의 **구체적 선례**. full-J, ordered numeric fold, Rank, foreign effects까지 완전하다는 뜻 아님 |
| [Glenside — Smith et al., MAPS 2021](https://ztatlock.net/pubs/2021-maps-glenside/) | 순수 tensor Access Pattern과 rewrite, im2col/하드웨어 accelerator mapping 사례 | 순수 access relationship 중심 표현·generic rewriting. J의 동적 의미론/오류 및 효과 보존은 추가 의무 |
| [MLIR Linalg — Structured Ops](https://mlir.llvm.org/docs/Dialects/Linalg/) | iterator/indexing maps; iteration subset↔data subset 분석, fusion/tiling | Domain/Access 및 backward region 유도의 직접 근거. affine map 제약·buffer-oriented lowering을 J Rank 의미와 혼동 금지 |
| [TACO — Kjolstad et al., ASE 2017](https://compilers.stanford.edu/publications/ase17/) | dense/sparse tensor algebra의 index notation과 코드 생성 | index/contract 관점; full-J boxed/fill/error/sparse ordering 지원을 자동 계승하지 않음 |
| [Remora — Slepak/Shivers/Manolios, 2019](https://arxiv.org/abs/1907.00509) | Rank polymorphism, Frame/Cell 기반 lifting과 shape typing의 형식적 모델 | `u"r`의 의미론을 독립적으로 취급하는 비교 대상; Remora가 J의 모든 동작을 정의한 것은 아님 |
| [Futhark Scan–Scatter Fusion, 2026-03-24](https://www.futhark-lang.org/blog/2026-03-24-scan-scatter-fusion.html) | 기존 fusion 규칙이 취약한 scan/scatter 결합에 대한 표현·컴파일 방법 연구 | Scan/Scatter·Accumulator를 Map/Reduce 하나로 소거하지 않고 basis를 진화시키는 사례 |

이 자료들은 공통 구조의 **가능성을 보여 주는 서로 다른 부분적 실증**이지, 하나의 범용·완결된 Array Algebra가 발견되었다는 합의가 아니다. 선행연구가 이미 달성한 것은 novelty로 주장하지 않는다.

<a id="71-spores-원-논문-재검토--nn-적용-범위와-완전성의-정확한-의미-2026-10-11"></a>
##### 7.1 SPORES 원 논문 재검토 — NN 적용 범위와 완전성의 정확한 의미 (2026-10-11)

**사용자가 제공한 SPORES 심층 보고서의 검증 가능한 핵심.** SPORES(Wang·Hutchison·Leang·Howe·Suciu)는 선형대수(LA) DAG → 관계대수(RA) → equality saturation → 비용 기반 추출 → LA 복원의 설계를 제시한다. 논문의 중심 가설은 *복잡한 합·곱 수식의 Equivalent Form을 개별 하드코딩 패턴 대신 공통 대수와 작은 재작성 규칙으로 탐색*할 수 있다는 것이다. 이는 RustJ의 **Equivalent Form Discovery** 연구와 본질적으로 같은 질문의 제한된 선행 성공 사례다. 그 자체가 NN 전체를 최적화하거나 J Rank를 보존하는 실행 컴파일러는 아니다. 원문: [arXiv 2002.07951, 2020-12-24 v2](https://arxiv.org/pdf/2002.07951), [PVLDB 출판본](https://www.vldb.org/pvldb/vol13/p1919-wang.pdf).

**수학적 핵심을 정확히 한정한다.** 논문 Table 1의 LA grammar는 행렬곱, 원소별 곱·합, 행·열·전체 sum aggregation, transpose의 **일곱 연산**이다(뺄셈은 `A + (-1)B`). 이 부분언어를 실수 계수 `K`-relation으로 사상한다: 원소별 곱↔join, 원소별 합↔union, 합산↔aggregate, 행렬곱↔join 뒤 contracted-axis aggregate, transpose↔속성명/축 순서 변환. `R_LR`의 여섯 LA↔RA 번역 규칙과 `R_EQ`의 일곱 관계 대수 등식으로 정상형을 다룬다. 정리 2.3의 **completeness**는 이 LA/RA grammar와 수학적 모델에 속한 두 식이 **임의의 유효 차원·입력**에 대해 의미상 동치일 때 관계대수 재작성의 유한 경로로 연결된다는 것이다. *IEEE 754에서 bitwise 같은 결과, 임의의 기계학습 모델/연산, 모든 J verb의 완전 동치*를 증명하지 않는다. 정규형의 유일성 보조정리는 **특정 크기로 고정된 행렬**에 대해서가 아니라 임의 차원 전체에서 동치임을 전제로 한다. 또한 theorem의 **규칙 표현력**과 실제 샘플링·time/fuel-bound e-graph의 **모든 표현 탐색 완료**는 구분해야 한다.

**실제 최적화의 대표 사례와 올바른 비용 계산.** `L = sum((X - u v^T)^2)`는 실수 대수의 조건에서 다음 등가형으로 바꿀 수 있다.

~~~text
L = sum(X .* X) - 2 * (u^T X v) + (u^T u) * (v^T v)

X: 1,000,000 × 500,000 sparse
u: 1,000,000; v: 500,000
naive uv^T: 500,000,000,000 multiply + huge dense materialization
rewritten:
  u^T u / v^T v: about 1,500,000 entries
  u^T X v: additionally proportional to X's relevant nnz, access/traversal
  sum(X .* X): additionally proportional to stored nonzero entries
~~~

따라서 사용자 제공 보고서의 **“최종 전체 계산량이 150만 번의 곱셈뿐”**은 `u^Tu`와 `v^Tv` 비용만 센 **불완전한 비용 주장**이다. `X`를 스캔하고 `u^TXv`, `sum(X^2)`를 계산하는 비용도 포함해야 하며, 논문은 거대한 dense 외적 구체화를 피하는 성과를 보여준다. 본 변환은 수학적 등가이지만 연산 재결합/분배가 부동소수점 rounding, signed zero, NaN, overflow 및 J observable error/promotion 순서를 바꿀 수 있으므로 **RustJ Exact-J 후보 자동 승인 금지**.

**NN에서 유용한 범위와 경계.**

| 범위 | SPORES식 대수 탐색의 가치 | RustJ 판정 |
|---|---|---|
| Dense/Linear, MatMul chain, Gram·Low-rank loss, 선형 결합 및 Reduction | contraction 축·공통 부분식·factorization·분배, sparse-aware 알고리즘 순서 변경 | 순수한 합·곱 fragment를 추출할 가능성 높음. 실제 J numeric/order 증명 필요 |
| Convolution의 선형 합산 부분, projection, Attention `QK^T`와 `AV` | index/contract 분해 이후 공통 합·곱 최적화 기회 | padding/stride/window/mask/shape, softmax 경계 등의 additional rules 필요; conv identity 유지 |
| ReLU, GELU, Sigmoid, Softmax, LayerNorm, MaxPool, clipping | 비선형·max/div/exp/conditional 등은 원 논문 합·곱 grammar 밖 | `Opaque` 경계 또는 별도 sound algebra/primitive 계약. 임의 push-through·분배 금지 |
| FFT/IFFT, Sort, Scatter with duplicates, random Gather, stateful optimizer | 별도 알고리즘·접근 관계/갱신 순서 필요 | SPORES completeness 적용 불가, 효과/오류/alias proof 필요 |
| 명시적인 adjoint primitives | 각 primitive의 J 배열 계산이 합·곱 영역을 포함할 수 있음 | autodiff로 대체하지 않음; 선언된 adjoint는 별도 semantic graph이며 동일한 증명 게이트를 통과해야 함 |

**성능 수치는 버전/비교 기준을 함께 기록한다.** 확인한 arXiv v2 초록·본문은 기존 SystemML 대비 새로운 최적화의 성능 향상을 **1.2×~5×**, ALS에서 최대 **5×**로 기술한다. 출판본 및 공개 초록/2차 자료에는 **최대 10×**가 보고된다. 서로 다른 논문 버전·워크로드·baseline을 혼합하여 RustJ나 일반 NN에서 10×를 보장한다고 쓰지 않는다. 또한 SPORES는 SystemML에 존재했던 **84개 handwritten sum-product rewrite/heuristic**을 발견할 수 있었다고 보고하지만, '모든 NN 최적화가 일곱 규칙으로 충분'이라는 뜻이 아니다. E-graph는 phase ordering 문제를 줄이지만 비용 추출은 CSE 공유 고려 시 전역적이고, 원 논문의 ILP 추출은 비용이 크며 greedy 추출·규칙 sampling은 **완전탐색의 실용적 근사**다. 탐색이 포화하지 못한 사례도 논문에서 보고된다.

**RustJ에 반영하는 검증 가능한 설계 가설.** (1) `with`로 확인한 dtype/shape/property를 *증명 조건*으로 쓰고 Graph Basis에서 side-effect-free Sum/Product region만 분석적으로 식별한다. (2) 해당 fragment에만 optional `KRelation/IndexSumProduct` projection을 만들어 대안 식을 기존 `GF4 GraphRewriteCandidate`와 witness로 보관한다. (3) 결합·분배·aggregate pushdown은 `Exact-J`/guarded 또는 `math-only`로 분류하고 IEEE J numeric·효과·오류가 보존되지 않으면 selection 금지한다. (4) bounded alternative discovery와 resource/target/cost extraction을 분리하며 measured NN workloads/negative cases에 기록한다. **새 필수 relational IR·SPORES runtime·자동미분 도입 결정이 아니다.** 이 비교의 실행 검증은 기존 GF4/GF5/GF6 및 FW/IO 원장에 종속되며 **구현 수용 [ ]**다.

<a id="72-tensat-원-논문구현-재검토--equivalent-form-graph-search의-직접-선례-2026-10-11"></a>
##### 7.2 TenSat 원 논문·구현 재검토 — Equivalent Form Graph Search의 직접 선례 (2026-10-11)

**왜 직접 참고하는가.** [TenSat, MLSys 2021](https://proceedings.mlsys.org/paper_files/paper/2021/file/cc427d934a7f6c0663e5923f49eba531-Paper.pdf)은 TASO에서 생성·검증한 tensor graph rewrite 규칙을 `egg` 기반 **Equality Saturation**의 `exploration → extraction`에 옮겼다. 사용자가 JAXA에서 구상한 **Equivalent Form Discovery → 조건부 동치 검증 → 비용 기반 선택**의 구체적 시스템 사례다. SPORES가 제한된 선형대수의 관계형 완전성에 초점을 맞췄다면, TenSat은 multi-output/공유 DAG 형태의 **텐서 계산 그래프를 탐색하는 엔진**에 초점을 맞춘다. TenSat이 J에 대한 보편적인 Array Algebra나 모든 동치 규칙을 새로 발견한 것은 아니다. [원본 Rust 코드](https://github.com/uwplse/tensat)는 변환 검증기·rewrites·greedy/ILP extractor를 포함하지만 README는 optimizer를 진행 중 구현으로 표기하므로 논문의 실험 결과와 현 GitHub artifact의 완성도를 구분한다.

**핵심 기술 4개.** (1) 원본 계산을 파괴적으로 rewrite하는 대신 e-class가 여러 등가 후보를 함께 보유하여 phase ordering 의존도를 낮춘다. (2) 여러 결과 노드에 걸친 **multi-pattern rewrite**를 지원한다: `Y1=A@B, Y2=A@C`를 `T=A@concat(B,C); (Y1,Y2)=split(T)`로 표현해 공통 입력·MatMul 수행을 공유하는 후보를 만든다. 각 output이 나중에 동일하게 이 변환을 채택해야 이득을 얻으므로 **한 노드씩 탐욕적으로 고르는 추출**은 공유 이득을 놓칠 수 있다. (3) 텐서 shape/layout/split 메타데이터를 e-class analysis로 유지하고, 규칙 적용 시 shape 조건을 확인한다. (4) **ILP extraction**으로 각 e-node의 선택 비용과 공유 관계를 계산하고 실행 DAG의 acyclicity를 보장해야 한다. 유효한 등식으로 e-graph 안에 cycle이 생길 수 있어 TenSat은 pre/post filtering을 사용한다.

**논문 성능과 정확한 한계.** 7개 GPU inference model에서 기존 TASO 대비 최적화 결과 실행속도 최대 **16% 추가 향상**, 최적화 시간 평균 **48배 감소**를 보고했다. 수치는 해당 GPU/TASO 규칙 집합·벤치마크에 한정된다. 논문의 '최적 그래프'는 **탐색된 유한 e-graph와 노드별 독립 실행 시간 합이라는 비용 모델 내** 최적해이지, 모든 프로그램의 절대 최적이나 측정된 실제 전체 graph runtime의 보장이 아니다. 비용 모델 오차로 검색이 늘어도 실행이 느려지는 사례가 있고, multi-pattern 적용을 반복하면 e-node가 폭발적으로 늘어 1시간 solver timeout에 닿는 사례도 있다. node/time/iteration/multi-pattern budget은 실제 optimizer의 필수 설정이다.

**RustJ에 옮기면 먼저 수정해야 하는 점.** TenSat `src/rewrites.rs`에는 `ewadd-is-associative`, `matmul-is-associative`, `distributivity` 등의 대수적 rewrite가 들어 있다. 실수 대수의 동치가 **J floating point rounding·NaN·signed zero·정수 overflow/promotion·Rank Cell/Frame·오류·부작용 순서**의 동치는 아니다. 두 e-node를 같은 e-class로 합치기 전에 *정적 Shape/DType만 아니라* `Equivalent_J(P,Q; Γ)`의 검증 가능한 의미론 계약과 witness/guard가 필요하다. J 효과를 지닌 영역과 수치 동치가 증명되지 않은 후보는 원 graph를 유지하며 extraction에서 승인하지 않는다. `with`는 정적 semantic facts에 기여할 수 있으나 device/tile/cost를 `with`에 넣지 않는다.

**기존 GF/IO 원장에 연계할 실험(새 체크리스트 없음).** GF4의 versioned GraphRewriteCandidate/CompositionRelation에 (a) 단일 출력 `Map∘Map`·`IndexMap∘IndexMap`, (b) 두 output의 MatMul share/concat/split 규칙을 **analysis-only e-graph 후보**로 비교한다. GF5/6의 target feasibility·resource cost에서 greedy vs DAG-sharing-aware global extraction의 중복 계산/메모리·compile-time 차이를 측정한다. negative corpus로 floating-point reassociation, error visibility, Rank empty-frame/virtual cell, output-binding mismatches, cycle, stale witness, shared producer side effect를 검사한다. `egg` 또는 `egglog` 채택 여부, ILP 종속성, 실행 최적화 승인 여부는 별도 후속 결정이며 현재 RustJ의 GF4/GF6 구현 수용은 올라가지 않는다. 최신 [EqiForge 2026-09 preprint](https://arxiv.org/abs/2609.12330)도 동일 주제의 후속 연구 후보지만 RustJ 실제 성능 근거로 사용하지 않는다.

<a id="eqsat-successors-plan"></a>
<a id="73-spores-이후의-실용-기술-egg-relational-e-matching-egglog-2026-10-12"></a>
##### 7.3 SPORES 이후의 실용 기술: `egg`, relational e-matching, `egglog` (2026-10-12)

**통합 방향(§7.6–7.9의 확정 설계 계약).** SPORES의 관계형 LA 표현을 J의 보편 IR로 복제하는 것과, 그 **인덱스·접근 관계**를 여러 J 배열 연산에 재사용하는 것, 그리고 `egg`/후속 연구의 **등가 후보 압축·탐색·추출** 기술을 채택하는 것은 서로 다른 결정이다. RustJ는 기존 J Graph IR을 정본으로 유지하면서 **선택적 `RelAccessView` 표현은 넓게 실험**하고, **무조건적인 관계대수 동치 규칙은 실제 Exact-J 증명이 있는 영역부터** 적용한다. Frontend 기반 후보 규칙 자동 생성은 [§7.8](#frontend-equality-synthesis), 관계형 표현의 확대 범위와 R0/R1/R2 구현 계획은 [§7.9](#spores-relational-integration), 오류·효과 조건은 [§7.7](#eqgraph-error-semantics)에 연결한다. 각 후속 연구는 서로 동일한 시스템의 연속 버전이 아니다.

| 연구와 1차 자료 | 실제 기여 | RustJ에 적용할 범위와 도입 판단 |
|---|---|---|
| [SPORES, PVLDB 2020](https://doi.org/10.14778/3407790.3407799) | 제한된 LA→관계대수→등가 포화→비용 추출; 합·곱 최적화와 희소성·CSE 상호작용 | **동치 법칙 검증은** 순수 `IndexSumProduct`부터 시작. 관계형 **접근·인덱스 표현**은 다른 J 배열 연산으로 단계적으로 확장(§7.9); full-J 강제 관계형 변환은 불채택 |
| [`egg`, POPL 2021](https://doi.org/10.1145/3434304), [Rust 라이브러리](https://github.com/egraphs-good/egg) | e-graph의 rebuilding, e-class analysis, 조건부 rewrite 및 extraction용 범용 Rust 엔진 | **첫 실험 엔진 후보**. 기존 `GraphRewriteCandidate` 옆에서만 bounded exploration; Cargo 필수 의존성 추가는 실험 결과 이후 결정 |
| [TenSat, MLSys 2021](https://proceedings.mlsys.org/paper_files/paper/2021/file/cc427d934a7f6c0663e5923f49eba531-Paper.pdf) | 비지역적 다중 출력 규칙, shape-aware exploration, 공유 DAG의 비용/순환 제약을 가진 ILP extraction | 다중 consumer/합쳐진 출력에 대한 탐색·추출 **설계 참고**. TenSat 수치 규칙을 그대로 이식하지 않음 |
| [Relational e-matching, POPL 2022](https://doi.org/10.1145/3498696) | e-matching을 관계형 join 질의로 보고 pattern 구조에 맞는 실행 계획을 사용 | 여러 graph pattern이 실제 병목으로 확인된 뒤에만 검토. RustJ가 SQL 엔진을 가져야 한다는 뜻 아님 |
| [`egglog`, PLDI 2023](https://doi.org/10.1145/3591239), [구현](https://github.com/egraphs-good/egglog) | Datalog 방식의 점증적 사실 추론·lattice 분석과 등가 포화·congruence closure·추출 통합 | `Known/Guarded/Unknown`, shape/Rank/alias/region demand의 **규칙 기반 고정점 분석**이 `egg`의 e-class analysis보다 명확하게 필요할 때 비교 실험. 기본 엔진으로 즉시 확정하지 않음 |
| [Glenside, MAPS 2021](https://ztatlock.net/pubs/2021-maps-glenside/) | 순수 tensor access pattern과 등가 탐색을 분리 | DACC의 Access/Compute 투영과 Conv/im2col 후보를 참조; J의 Rank·prototype·효과는 별도 proof |

**`egg` 우선, `egglog` 비교 후 결정.** 현재 `src/j_graph_rewrite.rs`는 원본을 보존하는 candidate registry/`GraphEquivalenceWitness`/`GraphRewriteProvenance` 경계이며 범용 e-graph가 아니다. `egg`의 Rust API와 분석 확장성이 **최소한의 범위 제한 탐색기** 검증에 적합하다. SPORES 관계적 축 schema·접근 변환은 별도 optimizer가 아니라 **같은 EqGraph의 typed term/analysis 후보**로 연결하고, 정적 Exact-J 증거가 없는 조합은 e-class union하지 않는다. `egglog`는 여러 fact와 관계를 반복 정련하는 규칙이 필요할 때 장점이 있지만, 다중 분석 도메인과 규칙 고정점 엔진을 지금 필수 middle-end로 끌어들이면 M2/M3 범위가 커진다. 한 번의 실험에서 두 엔진을 동시에 프로덕션 의존성으로 추가하지 않는다.

<a id="74-rustj-등가-포화-adapter-계약--법적-등가성-구조적-탐색-추출을-분리"></a>
##### 7.4 RustJ 등가 포화 adapter 계약 — 법적 등가성, 구조적 탐색, 추출을 분리

~~~text
VerifiedFrontend / J Semantic IR / canonical J Graph IR
  → eligible pure closed region + source/semantic/error facts snapshot
  → two optional inputs to EqTerm:
       (a) Frontend-guided typed J rule candidates (§7.8)
       (b) SPORES-inspired RelAccessView / indexed axes (§7.9)
  → independently proven Exact-J rules + EvidenceEdge; guarded/unknown isolated
  → bounded egg E-Graph (source J terms + admitted relational alternatives)
  → verified shared-DAG / multi-root extraction, resources and cost
  → Verified Logical IR / ExecutionLegality / target route admission
  → RoutePartition / Schedule / Physical Plan
  (if proof, budget, extraction or admission fails: original verified graph)
~~~

- **탐색 영역:** 명시적으로 닫힌 순수 계산 region만 export한다. Name late binding, `1!:` I/O, mutable `StateResource`, 상태 의존적 adjoint, 오류 관찰 순서가 달라질 수 있는 소비자를 동일 e-class에 무조건 합치지 않는다. FunctionEntity·derived operator·NN extended primitive·독립 adjoint identity는 원본에서 유지한다. 순수 fragment 안에서도 J-visible dtype, Shape, Rank/Cell/Frame, prototype/fill, sparse/boxed, fit/tolerance, 정수 promotion, floating-point bit/exception 및 실행 횟수를 증명해야 한다.
- **e-class 합치기 전 legality:** 후보 규칙의 `rule_id/version`, source-span, input occurrence, original `ValueId`, fact snapshot, `FactWitness`와 필요 guard/fallback을 보관한다. *수학적 등식이나 dtype/shape 일치만으로 union 허가 금지.* `Exact-J Proven`의 정적 동치만 무조건 union하며, `Guarded`는 guard 영역에서만 성립하는 별도 조건부 후보로 유지한다. 기본 `egg` e-class 자체가 runtime guard, side effect, J error precedence를 증명하지 않는다. `Unknown/Disproven`은 합치기·선택 불가.
- **조건부 분석:** e-class의 shape·rank·access·dtype·array property·source-version fact merge는 기존 `ValueFacts`의 refinement 규율과 모순되면 거절하거나 `Unknown`으로 후퇴한다. 다른 타입이나 다른 result assembly의 e-node를 잘못 동일시하지 않는다. Proof/witness 유효성은 그래프/규칙 버전에 귀속시키고 stale witness·negative fact를 verifier가 거부한다. 선택한 후보는 Graph/Logical 경계에서 **재검증**한다.
- **탐색 예산·재현성:** e-node/e-class 수, rewrite iteration, pattern match, 다중 패턴 확장 및 wall-clock/solver budget을 독립 제한하고 정해진 규칙 순서와 seed로 재현 가능한 결과를 남긴다. budget/timeout/solver 부재 시 **원래 검증된 graph 경로**를 선택한다. Source Graph와 J semantics를 파괴하거나 새 필수 IR/pass로 대체하지 않는다.
- **추출은 단일 트리 최솟값이 아니다:** `egg`의 기본 tree extractor만으로 공유 producer의 재사용 vs 재계산, 여러 output, memory peak, read-region/I/O, transfer, target backend, fuse 가능성을 올바르게 평가할 수 없다. 우선 bounded 후보를 **공유 DAG**로 복원하고 `ResourceEstimate`와 별도 `CostEstimate`를 비교한다. greedy/tree, 공유-DAG-aware heuristic, 선택적 ILP(예: [`egg` LpExtractor](https://docs.rs/egg/latest/egg/struct.LpExtractor.html))를 구분한다. 실제 추출 DAG의 cycle·consumer 수·lifetime·재료화와 output binding을 검증한다. ILP/solver는 성공 전제나 필수 의존성이 아니다.
- **출력:** 등가 포화 탐색 결과는 *분석 후보*일 뿐, `LogicalOptimizer` 실행 권한이나 물리 kernel/배치 배정을 의미하지 않는다. 숫자적 재결합을 허용하는 별도의 비-J approximate 모드를 이 변경으로 만들지 않는다. 모든 경로에서 순서 있는 first error, empty-frame virtual Cell, full-J fallback이 살아 있어야 한다.

<a id="75-기존-gffwio-수용-원장에-속하는-실행-체크리스트"></a>
##### 7.5 기존 GF/FW/IO 수용 원장에 속하는 실행 체크리스트

아래 항목은 **기존 체크리스트의 하위 증거**이고 새로운 완료 단계나 독립적인 구현 상태표가 아니다. 전부 **미구현·미검증**이며 문서 반영만으로 GF4/GF6 수용 상태를 변경하지 않는다. M2 frontend, M3 legality, M4 single-CPU baseline보다 앞서도록 일정을 강제하지 않는다.

- [ ] **GF4 / FW-05–09:** `egg` proof-of-concept에서 원본 Graph IR과 adapter term의 왕복 보존, stable source/version, guarded rewrite와 stale-witness/Unknown 거절을 검증한다.
- [ ] **GF4:** 소규모 *정확한* `IndexMap∘IndexMap` 및 독립 순수 Map 사례를 우선 비교한다. 비법적 FP reassociation, error/effect 재배치, empty-frame prototype, sparse/boxed, dynamic NAME을 반드시 negative corpus에 넣는다.
- [ ] **GF4 / GF5a:** DACC/`RelAccessView` projection의 Window/Reduce/Scan/Conv와 동일 input-region pullback 규칙 재사용을 측정한다. **표현 가능한 연산군**과 **Exact-J rewrite 가능한 연산군**을 따로 계측하고 [§7.9](#spores-relational-integration)의 R0/R1/R2 확대/반증 조건을 따른다. 동일 규칙이 **서로 다른 두 연산군 이상**에서 sound하게 동작하지 못하면 일반화 주장을 기각한다.
- [ ] **GF4 / GF6:** 다중 출력·공유 producer 후보에서 tree/greedy와 shared-DAG-aware extraction을 비교하고 output/cycle, read-bytes, duplicated work, peak-materialization과 compile-time을 기록한다.
- [ ] **GF6 / IO-09·10:** 실제 executable CPU 후보가 생긴 후에만 고정 입력·동일 target에서 speed/Work/Depth/I/O/메모리 개선을 검증한다. 외부 route도 동일한 legality 및 semantics verification을 통과해야 한다.
- [ ] **GF4 / FW-05–09 (후순위):** `egg`와 `egglog`의 사실 정련·규칙 간 상호작용·구현 복잡도·컴파일 시간을 A/B 비교하고, 관계형 e-matching은 **pattern matching 병목 측정 후** 검토한다. 채택 여부와 crate/version/feature flag를 별도 결정 기록으로 남긴다.

**문서 검증 상태 (2026-10-12):** 논문 출처·기존 stage/원장과의 매핑만 검토했다. 새로운 optimizer 구현, corpus 실행, J-C oracle, Rust `fmt`/`clippy`/tests 및 성능 벤치마크는 이 문서 수정에서 **UNRUN**이다. 연구 결과를 실행 성능·J 의미 동치의 실증으로 표시하지 않는다.

<a id="eqgraph-architecture"></a>
<a id="76-아키텍처-결정-기존-j-graph-ir-옆에-e-graph를-실제로-둔다-2026-10-12"></a>
##### 7.6 아키텍처 결정: 기존 J Graph IR 옆에 E-Graph를 실제로 둔다 (2026-10-12)

**결정(설계 확정, 구현 미완료):** RustJ가 등가 포화(Equality Saturation)를 수행하려면 후보를 한 번씩 나열하는 `GraphRewriteCandidate`만으로는 부족하다. 동일 계산의 여러 표현을 e-class로 합치고, 대안의 조합을 탐색하고, 공유 DAG 기준으로 선택하는 **실제 E-Graph**가 필요하다. 따라서 순수하고 의미론적으로 닫힌 Graph region을 위한 별도 **`EqGraph`(가칭, `egg::EGraph` 기반 optimization representation)** 를 도입한다. 이것은 **컴파일러에서 정식으로 지원할 최적화 구조**이지만, 모든 J 실행 경로가 반드시 통과해야 하는 *canonical J Graph IR 대체물*이나 필수 lowering 단계는 아니다. 인터프리터·미지원/효과 있는 region은 원본 검증 경로를 유지한다.

~~~text
J Semantic IR / canonical J Graph IR (source preserved)
        ├── original verified lowering / fallback
        └── eligible pure region + source/fact/effect snapshot
                  ↓
             EqGraph adapter → egg::EGraph
                  ├── ENode(op, typed semantic signature, child EClassIds)
                  ├── EClass / conservative EClassFacts
                  ├── RuleId + EvidenceEdge / witness / provenance
                  └── source ValueId ↔ multi-root EClassId map
                         ↓ bounded saturation
                  ├── alternatives (not immediately committed)
                  └── shared-DAG-aware extraction + verifier
                         ↓ only when legal and realizable
             Verified Logical Array IR → existing routes
~~~

**E-Graph가 필요한 이유.** `GraphRewriteCandidate`는 원본→대체 후보와 provenance를 기록하기 좋지만, 재작성 결과에 또 다른 규칙을 적용하고, 서로 다른 경로에서 만난 동치식을 공유하고, 여러 출력의 공통 계산 비용을 비교하는 **congruence closure + 재구축(rebuilding) + equality saturation** 저장소는 아니다. 이 역할은 검증된 `egg` 자료구조/Runner에 맡긴다. RustJ는 J-specific term 변환, legality, witness, region policy, extraction을 소유한다. 새 union-find/e-class 엔진을 별도로 작성하지 않는다.

**최소 인터페이스/불변조건:**
1. `EqRegion::eligible`: FunctionEntity/Rank/Cell/Frame, dtype/shape/prototype/fill, observable checks/errors/effects, NAME/locale/source-version, input occurrences를 보존할 수 있는 **닫힌 순수 영역**만 export한다. 검증 불명은 원본 경로로 남는다.
2. `EqTerm/EqClassFacts`: 구조적 primitive/extended primitive/명시적 adjoint를 무조건 scalar Map/Reduce로 분해하지 않는다. 원소 타입·Shape·Rank·result assembly/ordered fold·numeric mode의 불일치가 있으면 e-class 합치기를 거부한다. 단순 `Unknown`으로 낮추는 일은 **합치기의 합법성 증명**이 아니다.
3. `EqRule + EvidenceEdge`: **정적 Exact-J 동치가 증명된 규칙에만** unconditional `union`을 사용한다. Guarded 변환은 guard가 region 전체의 동치 범위를 정적으로 보장하지 않으면 조건부 별도 후보로 보관하고 extraction/guarded lowering 시 검증한다. 근거 없는 실수 대수의 결합/분배, 오류·효과 순서 변경과 `Unknown` 증거의 union을 금지한다.
4. `RootMap/Extractor`: 원본 입력 occurrence와 각 결과 `ValueId`에 대응하는 **복수 root**를 보존한다. 단일 `RecExpr`의 AST-size 최소화를 다중 출력/공유 DAG 최적이라고 취급하지 않는다. multi-root consistency, output identity, acyclicity, liveness, shared producer materialization, resource/cost/target을 구분해 추출한다. ILP는 후속 선택 사항이다.
5. `EqBudget/Fallback`: iteration/e-node/e-class/match/time 및 다중 규칙의 예산을 제한한다. 탐색 실패·proof 실패·budget 초과·비용 모델 미정이면 원본 Graph IR을 파괴하지 않고 되돌아간다. 실제 Physical schedule/layout/device 배정은 이 구조 바깥에 둔다.

**단계별 구현 순서(기존 GF/FW/IO 체크리스트에 귀속):** (A) `GF4 / FW-05–09`: 별도 `src/j_graph_eq.rs` 또는 동등한 모듈에 `egg` adapter, 소규모 typed term·source/root map·검증기를 마련한다. crate/feature 선택과 직접 의존성 반영은 실제 구현 PR에서 결정한다. (B) `GF4`: 안전한 identity/IndexMap composition **한 가지**를 먼저 동치 포화하고 source-preserving round trip, empty-frame·오류·수치 반례를 검증한다. (C) `GF4 / GF5a`: 두 번째 규칙과 cross-family access 분석으로 누적 탐색의 가치가 있는지 확인한다. (D) `GF6 / IO-09·10`: 다중 출력 공유 DAG extraction을 실제 CPU/reference 실행 비용과 검증한다. (E) 충분한 사실 추론 병목 증거가 생긴 경우에만 `egglog`로 교체/혼합하는 A/B 검토를 수행한다.

**현황:** E-Graph **도입 결정과 계약의 문서화만 완료**. `EqGraph` 모듈·`egg` 의존성·실행 최적화·실증 성능은 아직 없다. 기존 GF4 후보 발견 구현과 GF6 추출 미완료 상태는 그대로다. M2/M3 correctness 및 M4 baseline을 희생하여 전면적인 saturation 엔진을 먼저 구현하지 않는다.

<a id="eqgraph-error-semantics"></a>
<a id="77-e-graph-오류실패-의미론-검토-오류는-계산의-관찰-가능한-결과다-2026-10-12"></a>
##### 7.7 E-Graph 오류·실패 의미론 검토: 오류는 계산의 관찰 가능한 결과다 (2026-10-12)

**검토 결론(설계 계약, 구현 미완료):** J 오류를 `egg`의 단순 rewrite 실패나 최적화 비용에 섞지 않는다. `x`와 `y`가 성공 시 같은 원소값을 만든다는 사실은 `Equivalent_J(x,y)`를 뜻하지 않는다. J의 관찰 가능한 **첫 오류의 종류·발생 조건·우선순위·원본 소스 귀속**, 오류 핸들러에 들어가는 사건과 그 전후 NAME/StateResource 상태까지 비교해야 한다. 특히 `Pure`는 `Total`이 아니다. 순수한 division, indexing, shape/rank agreement, Reduce/Rank cell application도 입력에 따라 실패할 수 있다. `GraphFacts`의 dtype/shape/rank는 의도적으로 오류·effect·평가 순서를 제외하므로 **GraphFacts만으로 E-Class union을 허가하지 않는다.**

**현재 코드 기반과 미비점(설계 참고):**
- `src/error.rs`의 `FailureCategory::{JLanguage,UnsupportedCapability,VerifierDefect,BackendFailure}`와 `ErrorContext`의 source span/word blame/definition frames를 재사용한다. `Error::Limit`는 실제 J 실행의 한 오류일 수 있으므로 *EqSat 탐색 메모리·시간 초과*를 `Error::Limit`로 바꾸지 않는다. 새 오류 enum 계층을 J 사용자에게 노출하지 않는다.
- `src/logical_ir.rs`의 `CallOp.effect`, `possible_errors`, `speculation`, `SemanticCheck`, `Operation.order_after` 및 `src/j_graph_composition.rs::ObservableOrder`가 이미 구분된다. `SemanticErrorKind`의 5종과 `PossibleErrors::unknown`은 **현재의 보수적 분석 요약**으로 보아야지 전 J 오류 분류를 열거하거나 완전한 순서 증명을 대체한다고 보지 않는다.
- `src/logical_executor.rs`는 zero-frame fill의 *증명된* 일부 Domain 복구 및 type-inhomogeneity 재시도만 수행한다. 이는 본래 의미론적 예외 처리이지 `egg`의 자유로운 오류 억제/재시도 권한이 아니다. `src/nn_extensions.rs`의 `error_order_proof=Unverified` 역시 자동으로 Total/NoError로 승격시키지 않는다.

**서로 다른 세 가지 실패 경로를 구분한다.**

| 구분 | 예시 | RustJ가 허용할 동작 |
|---|---|---|
| **J의 의미론적 실패** | Domain/Length/Rank/Index, NAME lookup, `1!:` I/O, 정수 승격과 관련된 실패·Rank fill의 공개 오류 | 원래 실행 위치·J error class·필요한 source blame·handler/side-effect/state trace를 보존한다. 최적화 실패로 간주해 원 식을 **오류 발생 뒤** 다시 실행하지 않는다 |
| **최적화 거절/예산 종료** | 규칙 증거 미비, 지원 밖 region, 제한된 e-node 수·시간·match budget, ILP solver 미채택 | J 실행 **전** 원본 verified route를 사용한다. 성공적 최적화가 없다는 내부 진단이지 J error나 user-facing Domain/Limit이 아니다 |
| **컴파일러/백엔드 실패** | E-Class facts 불일치, 잘못된 추출 DAG, stale witness를 통과시킨 verifier, admitted kernel 실행 실패 | `VerifierDefect`/`BackendFailure`로 분류·기록한다. 조용히 J Domain으로 바꾸지 않는다. 이미 J 효과나 오류가 관찰된 뒤 재실행 금지; rollback/snapshot 경계가 **독립적으로 증명된 경우**에만 재시작 가능 |

**오류 합법성 모델(선행 analysis sidecar, 새 필수 Graph IR payload 아님).** 원본 semantic execution을 기준으로 `ErrorEnvelope` / `ErrorFrontier`(개념적 이름)를 후보마다 구성한다.

~~~text
ErrorEnvelope for exported EqRegion
  failure_summary: ProvenTotal | MayFail(classes_or_unknown) | Unknown
  obligation_order: data edges + SemanticCheck.order_after
                    + complete J invocation / hook-fork / Rank cell order
  fault_sites: stable source ValueId / OpId, spans, word blame, frames
  observable_prefix: effects, NAME read/version, writes, state, handler events
  conditionality: value/shape/rank-dependent failures and guards
  multiplicity: whether a potentially failing call is executed 0/1/N times
  witness: exact rule/version, fact snapshot, guard/fallback, invalidation
~~~

**핵심은 `ErrorEnvelope` 자체가 증명이 아니라 검증할 의무의 목록이라는 점이다.** 후보는 성공 결과의 JType/Shape/Atoms/fill뿐 아니라, 실패하는 입력의 first observable error 및 그보다 먼저 완료된 side effects, 오류 핸들러의 선택, source diagnostic mapping까지 대조해야 한다. 동치성은 원본의 **전체 허용 입력/환경**에 대해 성립해야 한다. 정적 가드가 있다면 증명 범위를 해당 guard의 유효 영역으로 제한한다. 새로운 오류가 없다는 추론에는 `may_error=false`라는 레지스트리 플래그만으로는 부족하고 rule/operand/dtype/range/shape를 포함한 독립 `NoObservableError` witness가 필요하다.

**E-Graph 합치기 등급:**
1. **T0: 증명된 Total + effect-free + order-independent.** 원본 관찰 조건이 모두 같은 경우에만 일반적인 e-class union, CSE, 순서 변경/공유 후보를 실험한다. 초기 `egg` 구현은 **여기서 시작**하며 algebraic float reassociation은 여전히 numeric witness 없이는 불허한다.
2. **T1: MayFail이지만 정확한 error/effect trace가 증명된 경우.** 오류 발생 횟수, 첫 오류, 실행 순서·소스 귀속을 동일하게 유지하는 좁은 규칙만 허용한다. 원본 `SemanticCheck`/오류 barrier를 제자리 유지하거나 명시적인 ordered boundary로 투영해야 한다. 검증 전에는 **불투명하게 고정**하고 관련 입력 계산을 미리 성공적으로 평가한 *경계 밖 결과값*만 T0의 입력으로 사용할 수 있다.
3. **T2: Unknown/effectful/dynamic or guarded without complete proof.** e-class union과 규칙을 통한 소거·복제·재배치를 금지한다. Guarded rewrite는 불일치 시 **첫 J-visible action 이전** 또는 증명된 side-effect-free deoptimization safe point에서만 원본 경로로 간다. T2 실패를 T0/T1 사실로 낙관적으로 정련하지 않는다.

**금지할 반례:** `0 * index(A,i)`의 Index 오류를 대수적 zero-fold로 삭제; 두 개의 실패할 수 있는 순수 호출을 재배치해 첫 Domain/Index 오류를 뒤집음; J right-to-left·Hook/Fork의 완료 순서를 바꿈; shape precheck를 원래보다 앞당겨 다른 오류를 가림; 계산 CSE로 오류 횟수나 첫 fault site를 변경; dead output의 실제 `1!:` read를 삭제; 빈 Frame에서 virtual fill cell의 Domain·Rank 의미를 오류 없는 상수로 교체; `Error::Limit`의 원인과 우선순위를 EqSat 내부의 allocation failure와 혼동; guard를 효과 발생 후에 검사하고 원본을 재실행. 숫자 결과가 같아도 이런 변환은 불법이다.

**구현/검증 하위 작업 — 기존 GF4/FW/GF6 및 RK 작업에 귀속(모두 [ ]):**
- [ ] `GF4 / FW-05–09`: eligibility에서 `effect`, `possible_errors`, `speculation`, `SemanticCheck/order_after`, `ObservableOrder`를 **함께** 조회한다. NoError witness 미비는 T0 탈락이고, 별도 T1 증명 전에는 의미론적 fault barrier로 고정한다.
- [ ] `GF4 / FW-05–09`: rule별 `ErrorEnvelope`/증명 엣지 및 원본 source provenance 유효성 검증. `egg` explanation은 rewrite 경로의 **출처**이지 J 의미론의 자동 증명서가 아니므로 규칙 soundness를 따로 입증한다. 참고: [egg 설명과 unsafe division 예시](https://docs.rs/egg/latest/egg/tutorials/_03_explanations/).
- [ ] `GF4 / RK`: Rank empty-frame fill, inhomogeneous fill retry, boxed/sparse, ordered Reduce/Scan, Hook/Fork, failing NAME/foreign I/O 등 J-C oracle ↔ RustJ reference ↔ 추출 후보의 **성공·실패·오류 맥락·상태**를 구분해 비교한다. 오류 핸들러에 의해 관찰되는 실패도 포함한다.
- [ ] `GF6`: multi-output/parallel/accelerator 추출 전 **semantic order graph**와 per-lane error-priority를 검증하고, 다중 실패 시 원본의 first error만 J 사용자에게 보고한다. commit 이전 정당한 fallback만 허가하고 backend failure를 J error로 위장하지 않는다.
- [ ] `GF4 / GF6`: budget/timeout, unsupported, failed guard, verifier defect, backend error를 각각 별도 오류 소유권으로 기록하고, **J 효과 발생 후 비증명 재실행 금지** 음성 테스트를 추가한다.

**문서 변경의 검증 범위:** 이 계약은 연구/설계와 기존 코드 구조 대조에 한정된다. `ErrorEnvelope`, 실제 `EqGraph`, J-C differential corpus 및 runtime 검증은 이 PR에서 **미구현·UNRUN**이다. 해당 오류 불변조건을 충족하는 증거 없이는 E-Graph optimizer를 실행 경로로 승인하지 않는다.


<a id="frontend-equality-synthesis"></a>
<a id="78-frontend-기반-j-동치-규칙-자동-발견생성-구문-표면과-의미론적-계산의-분리-2026-10-12"></a>
##### 7.8 Frontend 기반 J 동치 규칙 자동 발견·생성: 구문 표면과 의미론적 계산의 분리 (2026-10-12)

**설계 결정(제안 확정, 구현 미완료).** 사용자가 수백 개의 J rewrite 규칙을 수작업으로 작성하는 것을 기본 전제로 삼지 않는다. 이미 구현된 RustJ Frontend의 **검증된 J Syntax + \`FunctionEntity\` + J Graph IR + Execution Semantic Facts**를 재사용해 타입/Rank/효과 조건이 맞는 식을 제한적으로 *합성(synthesis)*하고, 검증된 규칙만 \`egg\`의 equality saturation에 투입한다. **\`egg\`의 책임은 검증된 규칙을 조합해 동치 표현을 누적 탐색하는 것**이고, 새 규칙의 발명·전제 조건 추론·J 의미론 검증은 별도의 **Equality Synthesis sidecar**의 책임이다. 문자열 재작성기, 병렬 optimizer 또는 새 필수 Frontend 단계로 만들지 않는다. 실제 구현/성능 향상은 아직 없다.

**왜 naked J syntax보다 Frontend 결과가 유리한가?** 원문 구문은 J 단어의 형성과 우→좌 실행, tacit train(hook/fork), valence, conjunction/adverb에서 비롯되는 함수 생성, rank conjunction, fit, NAME/locale/late binding을 문맥 없이 판정할 수 없다. Frontend는 이미 토큰·구문·소스 origin과 함수 operand의 **구조적 정체성**을 기록하고, J Graph에는 단항/이항 적용, Graph Basis, 중첩 FunctionEntity, source input occurrence, region의 branch completion order가 있다. 다음 단계의 Logical IR은 \`CallOp\`의 access/Rank/Cell/Frame, \`possible_errors\`, \`speculation\`, \`SemanticCheck\`, \`order_after\`를 제공한다. 따라서 **파서를 복제하지 않고** 의미론적으로 가능한 후보만 열거하고 source provenance·오류 검증을 연결할 수 있다. 단, Frontend가 unknown shape/dtype/NAME·동적 오류를 알게 만들어 주는 것은 아니며, J Graph IR의 \`GraphFacts\`만으로 NoError/동치가 증명되지는 않는다.

**정보를 구별해 보존하는 계약.** \`ParsedSource/FrontendContext\`와 \`FunctionEntity\`는 **원래 J 구문의 증거·함수 정체성**이고, \`GraphBasis/Region\`은 **의미론적 계산 패턴**이며, \`Logical CallOp\`와 ordered checks는 **실행 합법성에 필요한 후행 사실**이다. 세 층을 합쳐 하나의 무조건적인 canonical AST로 붕괴시키지 않는다. 특히 동일한 글자열도 NAME version, locale, operand capture, valence에 따라 다른 의미를 가질 수 있다. \`FunctionEntity\`의 원본을 보존하고 동치 비교용 추상 term을 **별도로 export**한다. 최적 표현을 다시 J 문자열로 unparse하는 것은 선택적 진단 기능이지 추출/Logical IR lowering의 필수 조건이 아니다.

| 구분 | 입력 / 결과 | 동치 탐색에서의 지위 |
|---|---|---|
| **Syntactic desugaring** | direct definition·구문 sugar → 구성된 J 함수/정의 객체 | 같은 J 언어 의미로 읽는 Frontend 표준화 후보; 소스·진단/바인딩을 유지해야 함 |
| **Structural canonicalization** | resolved valence/primitive, train/Rank/composition → explicit semantic term/Graph | 동치 후보의 **표현 통일**, 곧바로 다른 알고리즘을 허용하는 rewrites가 아님 |
| **Algebraic equality** | \`A(x) ↔ B(x)\` with typed shape/rank/error/number assumptions | Exact-J 증명 후에만 \`egg\` e-class union; 조건부/미증명 후보는 분리 |
| **Idiom/implementation specialization** | \`(+/ % #)\` Fork → \`jtmean\` 같은 specialized executor | 소스 인식·성능 후보이지 자동 대수 동치 증명 또는 J Graph IR node 강제 치환이 아님 |
| **Argument/representation normalization** | tessellation argument의 표준형, copy/view, buffer layout | 논리 의미론/전제 검사와 분리. 물리 선택은 E-Graph 밖 |

**jsource에서 확인된 구체적인 추출 근거(기존 pinned revision \`13994ffa1ed5f06f79fad6e9822a7ed2d29b1528\`):**
1. [\`jsrc/cf.c::jtfolk\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c)는 ordinary \`+/ % #\` fork 형식을 검사해 \`f1=jtmean\`로 전용 구현을 선택한다. RustJ의 \`src/j_graph_jsource.rs::is_mean_fork\`는 이미 \`FunctionEntity::Fork\`/\`ForkSemantics::Ordinary\`와 \`PrimitiveAdverb(Slash)\`·\`PrimitiveVerb(Plus,Div,Pound)\`를 검사한다. **문자열 탐색이 아니라 Frontend가 보존한 operand DAG 구조**를 활용할 수 있다는 작동 중인 분석 예다. 다만 현재 결과는 \`OpportunityLegality::AwaitingSemanticProofs\`, \`selected=false\`이고, 일반 \`Mean\`으로의 무조건적 \`egg\` 합치기 허가가 아니다.
2. [\`jsrc/cc.c::jttesa\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cc.c)는 tessellation 인자를 rank/length/domain 검사와 변환 뒤 **표준형(start/length vector)**으로 만든다. 이는 표준형 비교/후보 생성의 근거지만, 입력 의미·오류/검사 시점·대상 배열 rank 조건을 지운 독립 항등식은 아니다.
3. [\`jsrc/cx.c\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c)의 direct definition 처리에는 \`{{...}}\` 구문을 \`9 : '...'\` 계열 표현으로 변환해 재파싱하는 경로가 있다. **구문 전개에 의한 공통 표현**의 예이지, 임의의 정의 실행/NAME/효과를 compile time에 평가해도 된다는 뜻이 아니다.
4. \`src/j_graph_jsource.rs\`의 **16개 jsource family**(mean, window, search, index, gather, rank/result assembly, copy/view, sparse/ownership 등)는 재사용할 provenance와 **pattern discovery seed**를 제공한다. \`DecisionOwner\`가 GraphFusion/ExecutionAlgorithm/PhysicalPlanner/RuntimeBinding으로 나뉘므로 **일괄적으로 \`egg\` rewrite로 변환하지 않는다**. 현재 family catalog는 analysis-only / existing-analyzer / awaiting-facts / downstream-only 항목을 포함한다.
5. \`src/j_graph_rewrite.rs\`의 \`FindViaWindowMatch\` / \`JFindCutMatchIdentity\`는 이미 source↔replacement 및 witness가 분리된 하나의 **기등록 동치 후보**다. 이것과 \`MeanIdiom\` recognition의 증거 수준은 같지 않다. Rule ID/FactRule/Scope/Proof를 통과한 경우에만 새 \`EqRule\` 후보로 승격한다.

**두 입력 표면을 쓰는 제한적 합성 경로:**

~~~text
Original J source
       │  VerifiedFrontend (Program + FrontendContext, source origins)
       ▼
J Syntax + FunctionEntity / derived verb identity
       ├── syntactic pattern extraction (jsource pinned idioms, desugaring)
       │      └── SyntaxPatternCandidate (not an equality proof)
       ▼
Canonical J Graph IR (GraphBasis, applied nodes, original regions)
       ├── typed semantic term export (not a replacement for source graph)
       └── bounded grammar/term enumeration + typed holes and occurrences
                     ▼
         CandidatePair / conditional rule schema
                     ▼
      staged oracle comparisons / counterexamples
                     ▼
       semantic legality + proof/guard review
             ├── disproved → reject with counterexample
             ├── unproved/guarded → pending/conditional registry (NO e-union)
             └── independently proven Exact-J → EqRule + witness
                                                    ↓
                                            egg::EGraph saturation
                                                    ↓
                                  verified multi-root/shared-DAG extraction
                                                    ↓
                                     Verified Logical Array IR / fallback
~~~

**개념적인 인터페이스(신규 필수 IR이 아닌 sidecar 계약).**

~~~text
TermSignature:
  sort=(noun | verb | adverb | conjunction | value), valence, primitive_id
  operand_structure/FunctionEntity, ordered_input_occurrences
  rank/cell/frame constraints, shape/dtype/JType, prototype/fill
  numeric tolerance/fit/overflow policy, exact rational/IEEE policy
  name/locale/source version, source origins
  effect/possible errors/ordered checks/observed state

SyntaxPatternCandidate:
  source_signature, normalized_semantic_pattern, jsource_rule/provenance
  phase = syntax_desugar | structural_recognize | algebraic | downstream
  owner = frontend | graph_rewrite | execution_semantics | planner
  proof_status = unknown | counterexample | guarded | independently_proven

EqRuleAdmission:
  lhs/rhs typed pattern, quantified variable bindings and valences
  required facts, legal context, exception/effect/numeric witness
  rule version + pinned J/JSOURCE revision + source provenance
  proof status = ExactJStatic (for unconditional union) | Other
~~~

실제 코드는 기존 \`FunctionEntity\`/\`GraphFacts\`/\`FactWitness\`/\`GraphRewriteCandidate\`/error fact를 **참조**하도록 만들고 이 schema를 사전 고정 API/필수 새로운 AST로 강제하지 않는다. "정규화 결과가 같다"를 단순 hash equality로 검증 완료 처리하지 않는다. 입력 occurrence가 다르거나 origin/version/witness가 stale이면 후보를 거절한다.

**자동 후보 생성 알고리즘(구현 순서):**
1. **시드**: J Dictionary의 명시적 항등식, \`GraphRewriteRule\`의 증명된 후보, pinned jsource의 **패턴 인식**을 분리해 수집한다. jsource의 specialized executor 자체를 \`LHS=RHS\` 항등식으로 복사하지 않는다. 동일한 일반화가 가능한 syntactic pattern만 provenance를 남기며 생성한다.
2. **유형 있는 문법**: 입력 \`x\`/\`y\`, 상수, 구조적 \`Apply\`, \`RankApply\`, \`Transpose\`, \`Ravel\`, \`Reshape\`, \`Gather\`, \`Reduce\`, \`Scan\`, 구조화된 \`Hook/Fork\`, 명시된 extended primitive/adjoint를 작은 depth부터 생성한다. 정적이 아닌 \`FunctionEntity\`를 분해하거나 \`Unknown\` type의 다형식을 임의의 구체 타입으로 만들지 않는다. 실제 구현 가능한 primitive 목록·합성 문법은 별도 버전으로 관리한다.
3. **초기 pruning**: sort/valence, 원소 타입·rank, symbolic shape/frame/cell, arity, primitive domain, result prototype/fill, ordered input occurrence를 검사해 구조적으로 불가능한 조합을 제거한다. input alias/NAME/side-effect 가능성이 있으면 닫힌 순수 region 밖에는 합성하지 않는다. 사실 충돌을 \`Unknown\`으로 격하해 합법화하지 않는다.
4. **관찰 서명**: 정해진 소규모 배열 코퍼스(스칼라·고Rank·empty-frame/empty-cell·boxed·sparse·mixed dtype·정수 승격·NaN/±0·tolerance/fit·유효/무효 index·동적 오류)를 pinned J-C oracle와 비교한다. **J value뿐 아니라 JType/Shape/Atoms, first error/handler, source blame, 효과와 사후 상태**를 구분한다. 무결한/격리된 fixture에서만 평가하며 효과 있는 후보 실행으로 호스트 상태를 오염시키지 않는다.
5. **동치 클래스 *가설***: 관찰 결과가 일치한 식들을 fingerprint로 bucket하고, 공통 부분식을 typed metavariable \`?x\` 등으로 anti-unify/generalize해 조건부 \`LHS ↔ RHS\`를 만든다. 결과가 같은 소수의 테스트는 **가설**이지 영구 \`egg\` e-class가 아니다. 입력값 의존 가드는 first J-visible action 이전에 검증할 수 있어야 한다.
6. **반례·증명**: 미사용 입력·Rank·dtype·fill·오류를 별도 differential suite에서 검사하고 반례가 나오면 scope/guard를 보강하거나 폐기한다. **한정된 입력 도메인**은 가능한 경우 완전 열거, 단순 구조 규칙은 규칙별 수학/타입/오류 순서 증명을 만든다. 일반 J 식의 완전 자동 동치 판정은 불가능하므로 oracle pass만으로 \`ExactJStatic\`으로 승격하지 않는다. 독립 검토/검증된 정적 항등식만 \`egg\`의 \`union\`에 허용한다.
7. **구현 연결**: \`RuleId\`, source revision, precondition, witness, numeric policy, effect/error trace를 serialize하거나 재현 가능한 registry에 기록한다. registry에서 승인된 \`EqRule\`만 bounded \`egg::Runner\`에 공급하고, 여러 rewrite 경로와 공유 DAG의 이득은 \`EqExtractor\`가 검증한다. cost/target/physical choice와 증명 status는 독립이다. Unknown/timeout이면 source graph를 유지한다.

**대표 세로 검증 사례:**
- **구조적**: \`(|: |: y)\` → \`y\`를 monadic transpose-twice *후보*로 자동 생성한다. 실제 J transpose/Rank 동작, 입력 평가 횟수와 negative case까지 증명 후에만 rule로 채택한다.
- **jsource-recognized idiom**: \`(+/ % #) y\`는 정확한 monadic ordinary Fork로 인식하며 \`MeanIdiom\` opportunity를 만들되, \`Mean(y)\`로 치환되는 rewrite는 numeric/empty/type/error/Rank 완전성 증명 전까지 보류한다. 단지 specialized execution algorithm을 비교할 경우는 E-Graph가 아니라 기존 ExecutionAlgorithm 측으로 보낸다.
- **Graph rewrite**: \`x E. y\` ↔ \`($x) x&-: ;.3 y\`는 현재 \`JFindCutMatchIdentity\` candidate와 source/fact witness를 유지하고, 실제 admissible scope를 검증한 다음 \`egg\` 변환을 검토한다.
- **조건부/음성**: \`0 * index(A,i)\`의 오류 소거, 실수 reassociation, NAME 재조회/중복 조회, side-effecting Fork 재배치, rank empty-frame fill 누락 등은 합성 결과의 관찰 값이 겉으로 같아도 **원본보다 일찍 J 오류를 가리거나 효과를 변경하면 거절**한다.

**재현성·예산·검증기:** enumeration의 depth/width/term 수, 타입 스키마 수, oracle case 수/시간, \`egg\` node/class/match/iteration 예산을 모두 제한하고 deterministic seed와 sorted rule IDs, generator version, \`JSOURCE_SOURCE_PIN\`, oracle J revision, parser/compiler version, discovery corpus hash를 기록한다. Pattern 발견과 프로덕션 Rule 승격을 다른 job/게이트로 운영한다. 단순 수치 fingerprint 충돌, alias/shape over-generalization, solver timeout, 새로운 upstream normalization으로 source map이 바뀐 경우 \`FAIL\` 또는 \`UNRUN\`로 남기고 \`egg\` 활성화를 차단한다. 발견율은 **후보/검증된 규칙/반례/미증명 규칙을 분리**해 보고하고, 최종 성능 주장은 실제 end-to-end 측정이 있을 때만 한다.

**기존 acceptance ledger에 연결할 미완료 체크리스트(새 milestone 아님):**
- [ ] **M2 / GF4** Frontend \`FunctionEntity\`와 original source mapping을 사용한 \`TermSignature\` adapter 구현: hook/fork, modifier, rank, valence, late NAME, repeated occurrence round trip 및 syntax-normalization/semantic-equality 분리 음성 테스트.
- [ ] **GF4 / FW-05–09** pinned \`cf.c::jtfolk\`, \`cc.c::jttesa\`, \`cx.c\` 변환을 세 부류로 분류하고 \`j_graph_jsource\` family registry와 연동한 provenance-bearing source pattern extractor; conditional syntax/algorithm 후보의 **금지된 e-union** verifier.
- [ ] **GF4 / GF5a** 정해진 typed grammar에 대한 bounded candidate enumerator + anti-unification/generalization 구현; ill-sorted/unknown/effectful 후보 early pruning, deterministic replay 및 검색 예산 테스트.
- [ ] **GF4 / RK / FW-05–09** pinned J-C oracle의 success/error/effect/shape/rank/empty/boxed/sparse/numeric 차등 코퍼스와 negative counterexample suite 구축; 발견 가설과 증명된 규칙을 서로 다른 registry로 관리.
- [ ] **GF4 / FW-05–09** \`EqRuleAdmission\`의 \`ExactJStatic\` 증거와 \`ErrorEnvelope\`/proof/guard 조건 검증기를 마련해 conditional 후보·불명확한 오류/효과/숫자 대수의 unconditional union 거절.
- [ ] **GF4 / GF6** 독립적으로 검증된 규칙 최소 한 개의 \`egg\` 등록, original/multi-root provenance 보존 및 extraction → Verified Logical IR round trip; 성공하지 못한 경우 원본 Graph fallback 증빙.
- [ ] **IO-09/10 / GF6** 생성된 여러 대안의 공유 producer/liveness/IO/resource/cost를 구분해 실행 검증 및 실측; optimizer 내부 budget 실패가 J \`Error::Limit\`이 되지 않는지 확인.
- [ ] **GF4 / GF5a** 기존 규칙만 적용한 \`egg\` baseline 대비 synthesized rule 추가의 별도 증가 효과를 측정하고, 새로운 확정 동치 규칙이 없거나 검증 비용이 과도하면 자동 합성 확대를 중단하는 반증 조건을 기록한다.

**현황(2026-10-12):** jsource pinned source 및 기존 RustJ Frontend/Graph 모듈 대조와 이 설계·체크리스트 문서화만 **PASS**. 자동 생성기/semantic equality oracle harness/independent proof/\`egg\` adapter, 성능 측정은 **미구현·UNRUN**이다. M2/M3 정확성, M4 단일 CPU baseline에 앞서 필수 stage로 올리지 않는다. \`FOUNDATIONS.ko.md\`의 원본 J 의미론과 Graph Basis/Execution Basis 구분에 어긋나지 않도록 이 연구는 **선택적 analysis/optimization sidecar**로 둔다.


<a id="spores-relational-integration"></a>
<a id="79-spores-관계형-접근-대수-통합-넓은-표현-범위와-제한된-동치-법칙을-분리-2026-10-12"></a>
##### 7.9 SPORES 관계형 접근 대수 통합: 넓은 표현 범위와 제한된 동치 법칙을 분리 (2026-10-12)

**범위 재평가(설계 결정, 구현 미완료).** SPORES는 단지 행렬곱 최적화가 아니라 **일반적인 배열 계산을 인덱스가 있는 관계로 보고 공통 규칙을 만드는 강력한 출발점**이다. 원 논문의 [Table 2/Figure 1](https://www.vldb.org/pvldb/vol13/p1919-wang.pdf)은 matrix multiply, elementwise multiply/add, row/column/total sum aggregation, transpose의 **7개 LA 연산**을 join/product·union/add·group-by sum과 축 이름 bind/unbind로 표현한다. §3.3은 square/minus/divide 등 사용자 함수·fused operator를 불투명하게 유지하거나 추가 규칙으로 연결하는 확장을 서술한다. §4.2는 선택된 SystemML 학습 workload에서 **sum-product 연산 실행 시간 비중 77.4–98.9%**를 보고한다. 이것은 J 전체 primitive 수나 J workload 전체의 커버리지 통계는 아니다. 다만 **연산 종류보다 실제 계산 비용 측면에서 폭넓은 적용 가능성**이 있다는 근거다.

**수정한 통합 결정:** RustJ는 (1) SPORES의 **엄밀히 증명된 관계형 sum-product 동치 규칙**과 (2) 그 원리를 일반화한 **관계형 접근/인덱스 표현 가능성**을 분리하여 둘 다 검토한다. 기존 \`GraphBasis\`, \`FunctionEntity\`, J Rank/Cell/Frame 및 논리 IR을 바꾸지 않고, 적격한 *pure closed region*에서만 **\`RelAccessView\`(개념명)**를 선택적인 J Graph → \`EqTerm\` adapter/analysis sidecar로 만든다. \`egg::EGraph\`가 Source Graph term ↔ RelAccess term의 **검증된 연결**을 보관하고 비용·공유 DAG 추출을 수행한다. 전체 J를 SQL/관계형 canonical IR로 변환하거나 새로운 필수 optimizer stage를 두지 않는다.

**표현의 최소 관계적 어휘와 J semantic envelope:**

~~~text
RelAccessView (optional EqGraph surface; conceptual, not implemented):
  Schema = (axis identities, finite extents, ordered axes, free/bound indices)
  Domain = (JType, Shape, ordered atoms, prototype/fill, boxed/sparse semantics)
  RelInput(array, source ValueId / FunctionEntity / occurrence)
  RenameAxis / Reindex / AccessMap / IndexJoin / Predicate (guarded)
  PointwiseApply(op) / Combine(op, reduction-order contract)
  Aggregate(bound axes, reducer identity and order)
  OpaqueBoundary(unknown J function, ordered error, effect, NAME, fill)
  Witness = (scope, input facts, rule proof, origin/version,
             J error/effect trace, numeric mode, fallback)
~~~

**중요한 의미론 조건:** SPORES의 \`K\`-relation은 실수의 \`(+,×)\` 반군/반환 의미론을 이용해 algebraic distributivity/reassociation을 적용할 수 있다. 그러나 J에서는 floating IEEE rounding/NaN/±0, tolerant equality/fit, overflow→whole-result promotion, 타입 혼합, Rank virtual fill, ordered reduction, observable first error/NAME 상태 때문에 같은 법칙이 일반적으로 성립하지 않는다. \`RelAccessView\`의 축/접근 관계가 *기술 가능*하다는 사실과 특정 rewrite가 **Exact-J 증명**을 갖는다는 사실을 혼동하지 않는다. 어떤 입력에서 sum aggregate의 순서가 자유롭다는 증명이 없으면 축을 재배열하는 관계대수 규칙을 적용하지 않는다. 원 논문의 수학적 완전성을 J의 오류·숫자 의미론으로 확대 주장하지 않는다.

**J 연산군별 통합 범위와 소유권(표현 가능성 ≠ 안전한 재작성):**

| J 연산군 / 예시 구조 | 관계형 표현/탐색 기회 | 초기 승격 정책 |
|---|---|---|
| **Elementwise map**: \`+\`, \`*\`, dyadic scalar primitive, broadcast | 공유 인덱스에 pointwise apply, join/align, shape-index/domain guard | 타입/반올림/fit/오류가 같은 연산만 Exact-J. 비합·비곱 함수는 opaque callable로 보존 가능 |
| **Sum/Reduce / dot / matrix contraction**: \`+/\`, \`+/ .*\`의 전형적 축 계약 등 | bound index에 대해 join → aggregate. 합·곱 부분영역은 SPORES의 핵심 | ordered fold·정수 승격·FP 재결합 불변성 증명 필요. 외부 BLAS/NN primitive는 별도 realization 후보로 보존 |
| **Transpose / permutation / broadcast axes**: \`|:\`, \`$:\`이 아닌 shape-dependent argument forms | 축 rename/reindex, free-index schema, 반복 축 대응 | 전치의 축 순서·Rank 조건과 atom order를 정확히 유지하는 구조적 변환부터 실험 |
| **Reshape / ravel / take / drop / gather**: \`$\`, \`,\`, \`{.\`, \`}.\`, \`{\` | index relation / affine-or-piecewise access, duplicate selection, bounds/fill domain | \`IndexJoin/AccessMap\`로 **표현 확장**. OOB, padding/fill, prototype, source occurrence 때문에 SPORES의 7-op 정리로 자동 합법화하지 않음 |
| **Window / convolution / sliding reduce**: \`;\.\`, \`\\\`, \`/\` 조합 / NN convolution | 입력 인덱스 + offset index / join + local map/aggregate; [Glenside access patterns](https://ztatlock.net/pubs/2021-maps-glenside/) 참고 | predicate/mask, boundary behavior, padding, reduction order, channel/groups, numerical error 검증 시에만 rule 활성 |
| **Scan / general Rank / cell assembly**: \`\\\`, \`"\`, hook/fork | prefix index relation, Cell/Frame partition, structured branch input | **단순 unordered sum-aggregate로 축약 금지**. 순서/빈 frame virtual cell/결과 조립에 대한 별도 primitive + evidence가 필요 |
| **Boxed/sparse/extended NN + adjoints** | index/value relations는 부분 설명 가능, sparse fill/default value·nested box 구조 또는 structural intrinsic 유지 | 순수한 내부 계산 일부만 export; opaque boundary/semantic witness 없는 전체 분해·병합 금지 |
| **NAME / dynamic locale / assignment / foreign I/O / RNG state** | 계산 결과가 array여도 관찰 순서·state 의존 | region barrier. 앞서 J-visible 효과가 있으면 원본 재실행을 통한 fallback 금지 |

**예: matrix contraction은 관계대수로 표기할 수 있으나 FP 변환까지 합법화되지는 않는다.**

~~~text
C[i,k] = ordered-sum_j (A[i,j] * B[j,k])

RelInput(A, free={i,j})
RelInput(B, free={j,k})
  → IndexJoin(shared j, output {i,j,k})
  → PointwiseMultiply
  → Aggregate(bound={j}, free={i,k}, ordered-J semantics)
  → C  [JType/Shape, origin, exception and numeric witness]
~~~

전치 \`Transpose(A)\`는 보통 축 label의 permutation으로, gather \`idx { A\`는 index relation과 bounds/fill 의무로 기술할 수 있다. 이 둘을 같은 관계적 어휘로 **분석**하는 것은 가능하지만 gather 오류를 소거하거나 matrix product 합산 순서를 바꾸는 것이 허용된다는 뜻은 아니다. 희소 배열의 missing tuple을 numeric zero로 해석하는 것도 J sparse fill/empty result dtype/prototype과 같다는 증명 전에는 불허한다.

**Frontend Equality Synthesis와 SPORES/egg의 역할 연결:**

~~~text
VerifiedFrontend + canonical J Graph
     ├─ original J term + source/effect/error guardrail
     ├─ typed J equality-synthesis candidate generator (§7.8)
     └─ optional RelAccessView from eligible pure regions
                │
       Syntax + shape/Rank/index schema and typed conditions
                ↓
       Candidate RelAccess identities / counterexample search
                ↓ independently reviewed Exact-J admission (§7.7)
       EqGraph(egg) holds both J terms and relational alternatives
                ↓ bounded saturation and legal multi-root extraction
       Verified Logical IR / unchanged original fallback
~~~

**세 단계의 확대 전략(완전성·성능은 실증할 때만 주장):**
1. **R0 / 첫 구현: 구조적 인덱스 의미론.** \`Transpose\`/axis rename, scalar or elementwise map의 **표현 왕복**부터; 자유/바운드 축·J dtype/shape·ordered atoms/입력 occurrence/오류 증거 유지. \`RelAccessView\`를 첫날부터 완전한 SQL 엔진이나 \`K\`-relation runtime으로 구현하지 않는다. 등가 e-union은 이미 증명된 identity 하나만 실험한다.
2. **R1 / SPORES core:** pure admissible \`IndexSumProduct\`에 elementwise add/multiply, join, constrained aggregate, matmul을 접목하고 일관된 shape/sparse/cardinality/work/IO cost를 비교한다. FP/overflow/zero-frame 등의 J 반례로 차단되는 수학적 규칙은 **guarded or rejected**로 남긴다. 실제로 두 개 이상의 서로 다른 J 연산군에서 동일 schema/access 법칙이 재사용될 때 공통 대수의 유용성이 입증된다.
3. **R2 / broader arrays:** gather/reindex/window/conv/NN extension/scan/Rank를 독립적 접근·순서 규칙이 입증된 범위에서 점진 추가. 이 단계는 SPORES 논문의 completeness 결과 **밖**이다. 규칙 발견 §7.8의 후보 생성기가 같은 schema를 쓰고, \`egg\` 추출기는 공유 DAG·sparse/fill 및 resource/cost의 실제 이익으로 채택 여부를 결정한다.

**다중 불변조건 및 verifier:** \`E-Class\`의 축 schema(이름 변경은 alpha-equivalence 검사), free/bound 축, JType/Shape/Rank/fill/zero-frame, source ValueId/FunctionEntity/origin/version, numeric mode와 \`ErrorEnvelope\`를 함께 검증한다. E-class facts 충돌을 \`Unknown\`으로 합쳐 union하지 않는다. \`Predicate\`/guard가 J 첫 오류보다 늦거나 효과가 발생한 뒤 fallback하면 거절한다. \`ResourceEstimate\`(가능성/메모리 상한)와 \`CostEstimate\`(예상 시간/IO)는 분리하고 추출 후 Logical IR 및 target route verification을 통과해야 한다.

**기존 GF/FW/IO/RK acceptance에 귀속할 미완료 작업:**
- [ ] **GF4 / FW-05–09** source Graph↔\`RelAccessView\` exporter/importer와 Free/Bound axis alpha-equivalence, source occurrences, J shape/type/fill/error witnesses round trip; 실패 시 기존 Graph 보존.
- [ ] **GF4 / GF5a** 7개 SPORES LA 연산의 관계적 표현 가능성 매트릭스와 J Map/Reduce/Transpose/Broadcast 분해를 직접 테스트해 **representation coverage / legal rewrite coverage**를 따로 계측.
- [ ] **GF4 / RK / FW** FP/NaN/±0, integer promotion, empty-frame virtual fill, boxed/sparse, ordered fold, gather bounds의 counterexample로 잘못된 \`K\`-relation union 차단.
- [ ] **GF4 / GF6** \`egg\`에서 source term↔relational term **witnessed** 변환과 작은 공유 DAG 추출, timeout/facts conflict의 검증된 원본 fallback 실험.
- [ ] **GF5a / IO-09/10** sparse/cardinality, materialization/read bytes, work/depth + IO의 실측 비교와 장기적으로 필요한 ILP extractor의 이득을 별도 평가.
- [ ] **GF4 / GF6** 두 개 이상의 서로 다른 연산군에서 동일 schema/access 변환 규칙이 *새롭고 적법한* 후보를 만드는지 반증 가능한 실험. 그렇지 않으면 순수 SPORES core만 채택하고 더 넓은 relational IR 강제 확대는 중단.

**현황:** SPORES 원문 Table 2/§3.3/§4.2 및 Glenside와 기존 RustJ 설계의 비교·문서화만 완료. \`RelAccessView\`, exporter, e-graph 연동, J 오류 증명, differential oracle, 성능 실험은 **미구현·UNRUN**. 이 계약은 §7.3의 초기 core-equality 범위를 부정하는 것이 아니라 **표현의 잠재적 커버리지와 합법적 최적화 커버리지를 분리**한다. M2/M3 correctness 및 M4 baseline은 우선이다.



<a id="conv-graph-ir-optimization"></a>
##### 7.10 Convolution의 Graph IR 최적화 — Structured Conv, Access/Halo, 동등성 후보와 수용 실험 (2026-10-13; 설계 전용)

**목표·현재 경계.** 이 절은 기존 공통배열대수·SPORES 관계형 접근 §7.9를 Convolution에 적용하는 세부 계약이다. 현재 **src/nn_extensions.rs**의 **NN-04 conv2d**, **NN-08 conv1d/conv3d**는 프론트엔드 인식·후보 명세 단계이고, Conv용 Graph 접근 sidecar, 실제 분해·fusion·egg/e-class, 실행 최적화는 미완료다. conv2d 표면 이름은 일반 J NAME이며 builder는 **Adverb**, 생성된 computational verb의 Rank/Valence/FunctionEntity는 별도이다. Frontend의 생성 시점/호출 시점 NAME 조회, 사용자 재정의와 오류·효과·원본 source occurrence를 바꾸지 않는다. Graph에서는 원본 **GraphBasisKind::Structured(Conv)** 노드를 남기고, SPORES가 사용하는 index/relational 투영은 **실행용 J 코드가 아닌 선택적 의미론 분석 sidecar**로 제공한다. **WindowView+Contract**, **im2col+GEMM**, 직접 Conv와 fusion은 비파괴적인 *후보*로만 둔다. **Convolution을 J primitive로 작성하거나 J 참조 정의를 제공할 의무는 없다.** J Graph에서 후보를 탐색하고, Logical IR은 합법적인 경우에만 선택적으로 분해한다. 새 필수 IR이나 J 문법 도입 금지. [확정 설계 §7.10.7](#conv2d-graph-spores-semantic-blackbox).

| 단계 | 입력/산출과 책임 | 다음 단계로 넘길 의무 / 아직 하지 않을 결정 |
|---|---|---|
| Frontend / J Semantic IR | builder/POS/NAME, 인자, derived verb, intrinsic/derived Rank | 바인딩·argument 평가 순서와 source 유지; Conv를 tokenizer keyword로 특별 처리하지 않음 |
| J Graph IR | Structured Conv의 원본 ValueId/FunctionEntity, dtype/shape/rank 사실, producer-consumer/fanout | graph access/fusion 기회만 기록; 커널·buffer·숫자 재결합 확정 금지 |
| 선택적 DACC / RelAccessView | 확정된 호출 geometry → output-to-input/index relation, contracted axes, tile demand, provenance/witness | 접근 기술 가능 ≠ J 동등성. 미상 geometry는 Unknown/Opaque |
| Rewrite / egg(향후) | 원본과 대안식에 RuleId/FactWitness/Guard 부착 | Exact-J observational proof 없으면 같은 e-class에 합치지 않음 |
| Logical IR + verifier | shape/Rank/empty/prototype, ordered check/effect/error, SSA 버전 | 수학적 같음만으로 evaluation order, dtype, 오류 제거 금지 |
| Physical/target planner | 합법 후보 + resource feasibility/cost + target capability | direct, GEMM, fused/library/tiling의 실제 실행 결정; fusion 이득 보장 금지 |

**7.10.1 Convolution의 인덱스 표현 — NCHW/grouped cross-correlation 참조 모델.** 예시 전제는 X:[N,Cin,H,W], K:[Cout,Cin/G,Kh,Kw], G>=1, Cin/Cout의 G 나눗셈 가능, stride (sh,sw)>0, dilation (dh,dw)>0, 비음수 대칭 padding (ph,pw), 선택적 bias b[o]이다. PyTorch식 Conv2d가 통상 사용하는 *kernel flip 없는 cross-correlation*을 기준으로 설명하며, 실제 J extension의 타입·빈 shape·오류·인자 순서·padding mode는 별도 규약과 오라클로 입증해야 한다.

~~~text
free axes:   n, o, y, x         (batch, output channel, output H/W)
bound axes:  c, r, t            (group-local input channel, kernel H/W)
g(o) = floor(o / (Cout/G))
ic = g(o)*(Cin/G) + c
ih = y*sh - ph + r*dh
iw = x*sw - pw + t*dw

Y[n,o,y,x] =
  ordered_sum_(c,r,t) ( padded_X[n,ic,ih,iw] * K[o,c,r,t] ) + b[o]

padded_X out-of-range coordinate => specified padding value;
never perform an out-of-bounds read of X.
~~~

비음수 대칭 padding, 유효 geometry/non-ceil 규약의 일반적인 Hout 식은 floor((H+2ph-dh*(Kh-1)-1)/sh)+1 이며 Wout도 같다. 이 식만으로 음수 분자·빈 결과·zero channel/kernel·integer overflow·J prototype/Rank/cell-frame·에러 발생 순서를 해결했다고 주장하지 않는다. **ordered_sum**을 실수의 교환 가능한 sum으로 자동 변경하지 않는다.

**7.10.2 Optional Conv access 분석 sidecar — 아직 구현된 Rust 타입 아님.**

~~~text
ConvSource { FunctionEntity, source ValueId, source span, binding version }
ResolvedConvCall { shapes/layout, ranks, groups, kernel/stride/dilation/
                   padding, dtype/numeric, error/effect/check contracts }
ConvAccessWitness {
  free_axes = {n,o,y,x}; bound_axes = {c,r,t}
  X_access -> Known(n,ic,ih,iw) | Padding(value) | Unknown
  K_access -> (o,c,r,t); bias_access -> o
  ordered_fold; source provenance; shape/Rank/fill/error guards
}
ConvDemand { requested outputs, exact input access set OR safe region cover,
             weight/bias demand, preserved semantic checks }
~~~

기존 **GraphBasisKind::Structured**, **Window/Reduce/StaticReindex**, **j_graph_access.rs**의 좌표 역추적 및 **j_graph_rewrite.rs**의 보존적 후보/증거 구조에 연결한다. geometry는 실제 호출로 결정되는 fact이므로 파서 FunctionEntity에 물리 tile/stride/buffer 결정을 넣지 않는다. 모델의 배열 layout(NCHW)은 해당 Conv의 축 명세이고, 물리 stride/layout은 Physical Plan 소유다. 축 alpha-renaming과 padding predicate를 확인하며 boxed/sparse fill을 무조건 숫자 0으로 바꾸지 않는다.

**7.10.3 역방향 수요(ArrayRegion pullback)와 Halo.** 결과 좌표 집합 Rout가 필요하면 실제 X 읽기 좌표는 Rout의 각 (n,o,y,x)와 모든 (c,r,t)에 위 접근식을 적용한 후 0<=ih<H, 0<=iw<W인 좌표들의 **합집합**이다. K 수요는 선택한 o 및 (c,r,t)에, bias 수요는 o에 한정된다. 출력 사각형 y∈[a,b), x∈[u,v), b>a, v>u에 대한 **안전한 bounding cover**는 다음과 같다.

~~~text
H_cover = [a*sh-ph, (b-1)*sh-ph+(Kh-1)*dh+1) intersect [0,H)
W_cover = [u*sw-pw, (v-1)*sw-pw+(Kw-1)*dw+1) intersect [0,W)
~~~

stride/dilation이 1보다 크면 cover에 실제로 읽지 않는 구멍이 생긴다. **cover는 정확한 최소 접근 집합이 아니다.** 정확한 합집합과 보수적 박스는 다른 fact로 보존한다. 예: Kh=Kw=3, stride=dilation=1, padding=1, 경계 안쪽 출력 타일 [10,18)×[10,18) (8×8)은 X의 공간 영역 [9,19)×[9,19) (10×10)을 요구한다. 경계에서 clipping, padding 값, group/channel subset을 별도 처리한다. Rout=empty라 값 읽기가 없어도 **J의 원본 argument evaluation, shape/prototype, check, effect, first-error까지 생략할 수 있다는 뜻은 아니다.**

**7.10.4 후보 생성·최적화 범위.**

| 후보 | 발견 조건 / 이점 | 독립적으로 증명·측정할 조건 |
|---|---|---|
| Conv→Bias→ReLU | 순수 single-consumer pipeline; intermediate materialization 생략 후보 | dtype/bias agreement, Rank, 오류·효과 순서, 다른 consumer/fanout, target resource |
| Conv→Transpose/Reindex | 소비자의 검증된 축 변환; index-map 합성 | J 논리 atom order/shape, 다른 사용자, 물리 view 또는 copy 필요성 |
| 1×1 Conv→(grouped) MatMul | Kh=Kw=1, 합법 stride/padding/groups/axis | weight packing, reshape bytes, 숫자 누적/FMA·Round-off |
| Conv→WindowView+Contract | offset/window, padding, reduction 축 확인 | 원래 Conv의 ordered fold/empty semantics·경계가 동일할 때만 executable |
| Conv→im2col+GEMM | input/kernel 좌표와 group 정합 | packing materialization/읽기 byte/재사용 vs 직접 kernel 비용 |
| Conv→Pooling 또는 Conv→Conv | 후속 연산 요구 타일을 전파 | halo 중복 계산, producer fanout, resource/동기화·오류 순서; 한 커널 fusion 보장 없음 |
| Inference Conv+BatchNorm folding | 불변 inference 통계·가중치·epsilon·cast 확인 | training BN 제외; FP reassociation은 Exact-J가 아닐 수 있음 |
| Conv+explicit VJP | 등록된 adjoint와 activation/state | input·weight gradient 접근과 activation lifetime; 자동미분 생성 아님 |

**7.10.5 SPORES/egg/실행의 경계.** 적격한 *순수* Conv 부분을 출력 자유축 {n,o,y,x}, 바운드축 {c,r,t}, **IndexJoin(X,K)→Multiply→ordered Aggregate**로 표현할 수 있다. 이 관계 표현은 **SPORES 식의 sum-product 탐색 후보**를 만든다는 뜻이지, 전체 Convolution/J 의미를 언제나 보존하는 대수적 rewrite 허가가 아니다. 실수 semiring의 결합/분배/aggregate pushdown은 J의 floating NaN/±0/FMA/rounding, 정수 overflow/promotion, ordered reduction, Rank/empty/prototype, NAME·효과·오류와 충돌할 수 있다. **Exact-J witness 없이는 egg e-class union 금지.** 근사 숫자 후보는 명시적 budget과 합성 오차 증명을 별도로 사용하고 J 비교 허용오차 2^-44를 최적화 기본 예산으로 사용하지 않는다.

한 가지 Structured(Conv)의 direct/GEMM/Winograd/FFT/library 구현은 **동일한 J 의미를 실현한다고 검증할 때만** backend realization으로 선택한다. 이것은 e-class를 합치는 것과 다른 결정이다. 경로는 Graph opportunity → access/geometry witness → legal rewrite/guard → Logical IR checks/effect/first error → target capability → ResourceEstimate + CostEstimate → Physical Plan이다. 최적화 이득을 판단할 때 연산량, memory traffic, peak temporary bytes, halo duplication, fanout, transfer, target occupancy/latency를 나누어 측정한다.

**7.10.6 최소 수용 실험 — 새 독립 로드맵 대신 기존 GF/FW/IO/NN 원장에 귀속.** M2→M3→M4 순차 CPU baseline을 최우선으로 두고, GF4·GF5a·GF6·FW-05–09·IO-09/10·NN-04/08에 다음 **Convolution 전용 test case**를 연결한다. 각 행은 Test/Purpose/Guarantee/Failure meaning/Status를 갖고, 현재 **모두 미구현/UNRUN**이다.

| Test / owner | Purpose | Guarantee / 반례 | Failure meaning | Status |
|---|---|---|---|---|
| conv_builder_binding / NN-04·GF4 | builder/derived verb·원본 보존 | NAME shadow/rebind, 호출/생성 시점, Rank/source span/effect | Frontend 의미 손실 | [ ] UNRUN |
| conv_index_geometry / NN-04·GF5a | X/K/result 축 검증 | 1×1, 3×3, 직사각형, stride/dilation/groups>1, depthwise, invalid geometry | 잘못된 source 좌표·shape | [ ] UNRUN |
| conv_region_halo / GF5a·IO-09 | output→input demand | 8×8→10×10, 경계 clipping, stride holes, channel subset, empty request | 읽기 누락/잘못된 exact cover | [ ] UNRUN |
| conv_padding_empty_error / FW-05–09 | padding/zero-frame/오류 | padding은 OOB read 아님; J prototype·required checks·first error | 관찰 순서/빈 결과 변화 | [ ] UNRUN |
| conv_numeric_rewrite_negative / GF4·FW | 허용 안 되는 변환 차단 | FP 재결합, NaN/±0/FMA, integer promotion, state/RNG, train BN, unknown | false Exact-J/e-graph union | [ ] UNRUN |
| conv_candidate_fallback / GF4·GF6 | 원본/후보 왕복 | proof/target/timeout 실패 시 원본 유지; 값/type/shape/error/effect 검사 | unverified 실행 또는 효과 재실행 | [ ] UNRUN |
| conv_cpu_cost_comparison / GF6·IO-10 | Direct vs GEMM vs tile | bytes/peak/work/halo duplication 및 CPU 실측 time 분리 | 측정 없는 성능 승인 | [ ] UNRUN |
| conv_vjp_lifetime / NN-16·GF5a | explicit adjoint 수요 | activation lease, input/weight gradients, state/error barrier | 자동미분 강제·상태 순서 변경 | [ ] UNRUN |

**증명·검증 분리:** J primitive로 작성한 Convolution 참조 정의는 **선택 사항이며 필수가 아니다**. 기존 J 연산의 언어 의미를 비교해야 할 때 pinned J-C는 그 부분의 오라클로 활용한다. **Conv2d 자체**는 독립적인 CPU indexed oracle이나 버전 고정 외부 구현·소형 수작업 정답을 사용할 수 있으며, extension의 수치·Rank·오류·빈 배열 계약에 맞게 검증해야 한다. 새 Conv extension의 의미 명세·독립 참조 구현, 접근 좌표의 정합성, 자원 예측의 실측은 각각 **별개의 검증**이다. 구조적 Index witness가 있어도 J observational equality, 실행 최적화, 성능 향상을 PASS 처리하지 않는다. 입력 반례와 PR-head·merged-main CI 증거를 별도로 기록한다. 구현되지 않은 Conv 경로는 Unknown/UnsupportedRoute이며 원본 의미 경로로 안전하게 fallback하거나 명시적으로 미지원 처리한다.

**현황:** 본 절 추가는 **문서 계약 작성만 완료**. Conv Access sidecar, EqGraph/SPORES exporter, 확장된 Graph rewrite, Conv CPU/GPU backend·fusion·성능 실증은 **미구현·미검증/UNRUN**이다. §7.9 R0→R1→R2와 기존 M2/M3/M4 우선순위는 변하지 않는다.


<a id="conv2d-graph-spores-semantic-blackbox"></a>
##### 7.10.7 설계 결정 — J로 풀지 않는 `Conv2d`와 Graph IR SPORES 처리 (2026-10-13; 설계/체크리스트)

**확정한 방향과 범위.** NN-04 `conv2d`는 J primitive 조합(`;._3`/Cut, Window/Multiply/Reduce 등)으로 **소스 코드나 Graph IR에 펼쳐서 정의하지 않는다**. J로 작성한 Conv 참조 구현 역시 **필수 조건이 아니다**. Frontend가 올바른 ordinary J NAME·POS 및 derived computational function을 해석한 뒤, **J Graph IR에는 하나의 고수준 `Structured(Conv2d)` semantic operation**을 유지한다. *Blackbox*는 구현 본문을 숨긴다는 뜻이지 shape·axis·Rank·입출력·배열 접근·effect를 모두 Unknown으로 처리한다는 뜻은 아니다. NN-08 Conv1d/Conv3d, adjoint, 다른 NN 연산이나 전체 NN 모델의 구현은 이번 결정·체크리스트 범위 밖이다. 기존 `7.10.4의 계층 간 fusion 후보는 장기 연구로 유지하며 다른 연산의 구현을 Conv2d 분석의 전제 조건으로 만들지 않는다.

**어디서 SPORES를 실행하는가.** `Graph IR optimization`에서 원본 `Structured(Conv2d)`에 붙은 **선택적 의미론 sidecar**를 이용해 SPORES 방식의 관계·sum-product *탐색용 투영(projection)*을 만든다. 이 투영은 **실제 J 소스도, 원본 Graph IR을 대체하는 필수 IR도, 즉시 실행되는 Logical IR도 아니다**. 원본 노드와 투영은 동일한 source/ValueId/FunctionEntity/parameter version·proof provenance로 연결된다. 투영에 필요한 fact가 미확정이면 `Opaque/Unknown`으로 남긴다. 기존 `j_graph_access.rs`·`j_graph_rewrite.rs`의 분석/보존적 후보 흐름에 통합하는 방향이며 지금 Rust API가 구현됐다는 뜻은 아니다.

~~~text
J ordinary NAME / derived function
        ↓
Graph IR: Structured(Conv2d) [source and weight/bias bindings retained]
        │
        ├─ semantic sidecar:
        │    Shape / Rank / axis and group geometry
        │    X, K, bias indexed AccessRelation / padding predicate
        │    free and contracted axes / ordered numeric reduction
        │    effect, first-error, empty/fill, NAME/version obligations
        │
        └─ projection for SPORES candidate exploration (analysis only)
                ↓
           candidate alternatives [original remains available]
                ↓
           equivalence evidence + guards
                │  unproved/math-only -> not an exact e-class union
                ↓
       Graph candidate registry / conditional cost evidence
                ↓
         Execution-semantic lowering → Logical IR
           ├─ original high-level Conv2d when supported
           └─ verified, selected WindowView/Contract/MatMul recipe
                ↓
          Physical route legality + measured resource/cost
~~~

**왜 소스 수준 J 분해가 필요 없는가.** `Conv2d`의 수학적 index relation(`7.10.1`의 자유축 `n,o,y,x`, 환원축 `c,r,t`, groups/stride/padding/dilation)은 sidecar에서 **의미 설명과 접근 관계**로만 사용한다. 이를 J 동사 나열이나 임의 `Gather/Multiply/Reduce` 노드 수천 개로 실체화하지 않는다. SPORES는 일반적으로 이 blackbox를 자동으로 이해하지 못하므로 RustJ가 **연산별 relation exporter와 검증 가능한 rewrite 조건**을 제공해야 한다. `Conv2d→WindowView+Contract`, 1×1 `Conv2d→(grouped) MatMul`, virtual im2col/GEMM, halo-tile은 모두 **탐색 후보**이며 원본 node 삭제·실행 최적화 확정이 아니다. 변환식이 J/Logical IR로 구현 가능하지 않으면 *분해하지 않아도 된다*.

**정확성 경계.** `Indexed access를 표현할 수 있음`, `수학적으로 동등함`, `J에서 관찰상 동등함`, `특정 타깃에서 실행·비용상 유리함`은 서로 독립적인 주장이다. SPORES의 semiring 등식은 부동소수점 NaN/±0/FMA·누산 순서·rounding, integer overflow/promotion, Rank/CellApply/empty/fill, 인자 평가와 first-error, NAME/rebind, effect/alias를 자동 보존하지 않는다. e-graph의 **exact e-class union**은 동일한 J 관찰 의미의 증거가 있을 때만 한다. 미검증/수학적 후보는 별도로 보관한다. Guard가 필요하면 관찰 가능한 effect보다 먼저 유효성·버전을 검사하고, 미증명 guard를 근거로 동등성을 선언하지 않는다. J의 `2^-44` 비교 허용오차를 무조건적인 근사 최적화 예산으로 전환하지 않는다.

**Graph / Logical / Physical 역할 계약.** Graph 단계는 `Conv2d` semantic identity, 원본 간선과 bindings, 후보 관계/증거, 효과·오류 의무 및 후보 비용 추정치를 보존한다. **실행 계획을 확정하거나 무조건 분해하지 않는다.** Logical 단계는 적법한 후보를 `WindowView/Contract` 등으로 **선택적으로 구체화**하거나 high-level Conv2d identity를 보존하고, SSA/Rank/shape/ordered SemanticCheck/first-error를 검증한다. 기존 Logical/Physical 경로가 해당 original/high-level op나 후보를 실행할 수 없으면 **실제 지원되는 경로만** 선택하거나 명시적 `UnsupportedRoute`로 남긴다. Physical 단계가 buffer/materialization/strides/schedule, target capability, hard resource feasibility 및 cost를 최종 판단한다. Graph의 낮은 비용 추정만으로 native kernel, 성공 실행 또는 성능 향상을 주장하지 않는다.

**동등성 오라클 정책.** J 언어의 일반 부분에 대한 pinned jsource differential은 그대로 중요하지만, 신규 `Conv2d` extension의 참조 정의를 **J primitive로 작성할 의무는 없다**. 독립적인 단순 CPU indexed convolution, 사양/버전을 고정한 외부 reference 구현, 손으로 검산한 소형 입력 등에서 **실제 extension의 서명·dtype·padding·numeric·empty/error 의미와 일치하는 범위**를 명시하여 differential corpus를 구축할 수 있다. 단순 수학/외부 프레임워크 결과 일치는 J observational proof가 아니며, 구현하지 않은 oracle 또는 미실행 테스트를 PASS로 기재하지 않는다.

**최소 구현·검증 체크리스트(`7.10.6 기존 테스트 원장을 증거로 사용; 별도의 NN-04 완료 체크박스 아님).**

| 단계 / owner | 필요한 산출·positive test | 반드시 실패시켜야 할 negative / 실패 의미 | 상태 |
|---|---|---|---|
| **GCS-00 / 문서** | Graph SPORES 투영 + blackbox 보존 + optional Logical 분해/비J oracle 정책을 한영 문서에 고정 | 문서 완료를 compiler/CI PASS로 취급하지 않음 | [x] 문서만 |
| **GCS-01 / GF4·NN-04** | `Structured(Conv2d)` 원본, FunctionEntity/ValueId/source·parameter binding/NAME·Rank 유지; `conv_builder_binding` | late rebind/동명이름·오류/효과가 다른 호출의 Conv 오인, source 소실 | [ ] UNRUN |
| **GCS-02 / GF5a·IO-09** | sidecar index/padding/group/ordered reduction 및 output→input exact set vs halo cover; `conv_index_geometry`/`conv_region_halo` | zero/invalid geometry·overflow·stride holes·empty/opaque에서 가짜 정확 witness | [ ] UNRUN |
| **GCS-03 / GF4·SPORES R1/R2** | **Conv2d-local** exporter 및 비파괴 원본/Window/MatMul/im2col/tile 후보와 RuleId/출처 보존; `conv_candidate_fallback` | exporter가 J 소스 분해를 강제, 원본 소거 또는 타 연산 구현 요구 | [ ] UNRUN |
| **GCS-04 / GF4·FW-05–09** | exact observation / math-only / unknown / reject와 guard 분리; `conv_numeric_rewrite_negative`·`conv_padding_empty_error` | floating 재결합, ±0·NaN, Rank/fill, first-error·effect·NAME 누락으로 false e-union | [ ] UNRUN |
| **GCS-05 / M3·GF6** | 검증된 후보만 Logical IR로 **선택적** expansion, 원본 high-level 경로·SSA/check·Unsupported 분리; `conv_candidate_fallback` | window/contract 전개 강제, 지원되지 않는 물리 경로에 조용히 우회 | [ ] UNRUN |
| **GCS-06 / GF6·IO-10** | legal 후보의 work/bytes/halo/peak-memory/resource, target 경로 여부, 측정 가능할 때 비용; `conv_cpu_cost_comparison` | unmeasured speedup, packing/target hard limit 누락 또는 모든 후보가 실행 가능하다는 주장 | [ ] UNRUN |
| **GCS-07 / NN-G6·GF6** | 독립 indexed oracle 및 가능하면 실제 지원되는 route의 전후 value/type/shape/error/effect differential, exact PR/head·main CI | 오라클 사양 차이·미실행 경로를 동등성 PASS나 실행 완료로 오인 | [ ] UNRUN |

**채택 기준.** GCS-01–03의 *분석 성공*은 Graph 후보 탐색만 허가한다. GCS-04의 J observational proof 없이는 exact e-class union/변환 확정 금지, GCS-05/06의 실제 route 및 물리 자원 증거 없이는 실행 최적화 완료 금지다. GCS-07은 구현된 해당 경로가 있을 때만 실행 결과/성능을 PASS로 기록한다. 이번 결정으로 **NN-04가 실행 가능해졌다는 뜻은 아니며**, 앞 절 `conv_vjp_lifetime`·다른 NN 연산 수용은 별도 후속이다. 전체 M2→M3→M4 baseline 우선순위를 그대로 둔다.

**현재 상태:** GCS-00 **문서 설계만 완료**. GCS-01–07, Conv relation exporter, SPORES/e-graph 통합, 실사용 최적화 및 검증은 **미구현·UNRUN**이다.


<a id="conv-virtual-expansion"></a>
**7.10.8 가상 전개(Virtual Expansion) — JAXA의 매개변수화 adverb를 유지하고 가중치 값은 노출하지 않음 (2026-10-14; 설계 전용).**

**실행 경계.** 원본 StructuredConv 보존은 실행 지원을 새로 제공하지 않는다. 실제 Logical/Physical 실행 경로가 존재할 때만 원본으로 실행할 수 있으며, 경로가 없으면 명시적으로 UnsupportedRoute를 반환한다. [§7.10.7 실행 경계](#conv2d-graph-spores-semantic-blackbox)를 함께 따른다.

**결정.** rank만 받는 adverb라는 대안은 폐기한다. 역사적 JAXA의 **shape/configuration noun → conv adverb → derived verb** 계약을 출발점으로 둔다. 입력·출력 채널 수, kernel 크기, stride, padding, bias 선택은 사용자가 *연산 정의의 매개변수*로 제공하지만 **학습 가능한 실제 weight 값**을 별도의 사용자 J noun 또는 conv 호출 인자로 제공하지 않는다. Graph IR에는 원본 Structured Conv가 남는다. SPORES 분석이 필요한 때에만 비실행·비파괴 sidecar에서 숨겨진 weight를 기호적 ParameterRef로 나타낸다. 이 과정은 J 소스를 다시 작성하거나 Conv를 J primitive 전체로 분해하는 것이 아니며, 새 필수 IR 단계를 추가하지 않는다.

**JAXA 원문의 정확한 인자 계약 (역사적 원형이지 현행 RustJ 실행 ABI가 아님).** 근거는 JAXA-complier의 Appendix B, docs/JAXA/primitives.py::_infer_conv, jaxa_enqueue.py::CUSTOM_PRIMITIVES다.

| 원형 builder | 왼쪽 noun: 정확한 인자 순서 | 입력 cell / 의미 |
|---|---|---|
| conv 2D | C_in C_out kH kW stride pad b_flag (**7개**) | [C_in,H,W], cell rank 3 |
| conv 3D | C_in C_out kD kH kW stride pad b_flag (**8개**) | [C_in,D,H,W], cell rank 4 |
| depthwise_conv | C 1 kH kW stride pad b_flag (**7개**) | [C,H,W], 채널별 filter |
| linear | N_in N_out b_flag (**3개**) | [N_in], weight shape [N_out,N_in] |

~~~j
NB. 과거 JAXA 문서 표기. 현행 RustJ에서 실행된다는 뜻이 아님.
C1 =: (1 6 5 5 1 0 1 conv)"3j32
C2 =: (6 16 5 5 1 0 1 conv)"3j32
DW =: (32 1 3 3 1 0 0 depthwise_conv)"3j32
F1 =: (400 120 1 linear)"1j32
~~~

C_in/C_out은 채널 수, kH/kW/kD는 kernel extents, stride/pad는 JAXA 원형에서 **공간축에 공통인 단일 scalar**이며 b_flag는 기본값 없이 명시하는 0/1이다. 유효한 2D geometry의 출력 H는 floor((H-kH+2*pad)/stride)+1이다. JAXA의 3/4 rank는 배치축을 제외한 cell에 적용한다. 과거의 RjP 표기(예: 3j32 = cell rank 3과 fp32 설정)는 JAXA 전용 연구 표기다. **일반 J의 rank conjunction(")에 precision을 새로 인코딩하지 않는다.** RustJ에서는 rank/cell·dtype/precision·numeric policy를 별도 의미 계약으로 두고, 원형 표기를 그대로 채택할지 여부는 미결정이다.

**JAXA와 RustJ 확장 범위를 구별한다.** 확인된 JAXA 일반 conv 원형은 단일 stride, 단일 pad, bias flag이며 dilation/groups 및 축별 비대칭 padding/stride 인자를 노출하는 계약은 없다. RustJ NN-04/08의 conv2d/conv1d/conv3d에는 dilation/groups를 포함하는 *확장 후보 스키마*가 별도로 존재한다. JAXA 7/8-field 입력은 호환/참조 계약으로 기록하고, RustJ의 확장 인자는 독립된 버전·형상·축·오류 의미론 및 oracle 검증을 거쳐야 한다. 누락된 configuration을 임의 기본값으로 채우지 않는다. RustJ의 공개 확장 NAME은 현재 conv2d이고, JAXA의 conv 명칭을 새 예약어나 기존 alias로 간주하지 않는다.

~~~j
NB. RustJ의 논의용 surface 후보; 실행 지원 확정 아님.
C1 =: (1 6 5 5 1 0 1 conv2d)"3
C2 =: (6 16 5 5 1 0 1 conv2d)"3
model =: C2 @: relu @: C1
~~~

여기서 7개 수는 **weight 값이 아니라 weight shape와 convolution 설정**이다. C1·C2의 layer instance는 서로 다른 weight를 가진다. J 이름은 일반 NAME으로 parser에서 해결하고 재정의/locale/late lookup을 보존한다. 실제 config, rank, input channel shape가 불일치하면 오류나 UnsupportedRoute를 명시한다. 사용자에게 직접 weight/gradient/optimizer noun을 관리하도록 강요하지 않는다.

**의미론과 상태의 소유권 (제안 분석 타입; 미구현).**

~~~text
J semantic / canonical Graph IR:
  ConvBuilder(config_noun, adverb_name_ref) -> DerivedFunction
  Apply(DerivedFunction, X) -> StructuredConv(call_id, X)
  retain FunctionEntity, NAME/locale/binding version, source span,
         Rank/CellApply, error/effect semantics

Semantic resources (not hidden mutable fields in a verb):
  LayerInstanceId -> StateResourceId(weight)
                   -> optional StateResourceId(bias)
  StateRead(weight, version=v) -> captured use/snapshot token
  StateWrite/Accumulate -> separate ordered effects
  ValueId != StateResourceId != BufferId

Analysis-only:
  virtual_expand(StructuredConv, resolved geometry, state versions)
    -> ConvAlgebraView {free_axes, bound_axes, index maps,
        IndexJoin(X, ParameterRef(instance, weight, version)),
        Multiply, OrderedAggregate, optional BiasRef,
        original_call_id, source/shape/rank/numeric/effect/error guards,
        proof obligations, expansion_budget}
    | Opaque(unknown configuration/resource/semantics)
~~~

LayerInstanceId는 사용자 NAME의 철자나 동일한 shape 자체가 아니다. DerivedFunction instance identity, 원본 binding/version 및 resource owner의 연결을 증명한다. C1과 C2의 shape가 같아도 weight는 합치지 않는다. 명시적으로 공유한 경우만 같은 StateResource를 참조하며, J NAME shadow/rebind, dynamic lookup, fixation(f.) 및 alias를 존중한다. JAXA의 과거 conv_forward/conv_backward 접미사 기반 weight 공유 관례는 참고 이력이며 RustJ에서 동일성을 증명하는 규칙으로 도입하지 않는다. 훈련 단계의 W@v와 W@(v+1)는 다른 관측 시점이다. 미확정 mutable read는 순수 값으로 추정하지 않으며, snapshot/ordered effect 없이 공통 부분식 소거나 e-class 병합을 허용하지 않는다.

**가상 전개의 절차와 각 경계.**

1. **요청 시 해석:** Conv의 config, output/input/weight ShapeSpec, Rank/Cell/frame, dtype, geometry, source FunctionEntity 및 state-read/version을 해석한다. 지원 범위 밖이거나 미상인 경우 Opaque로 유지한다. 이때 실제 weight 값을 읽거나 초기화하지 않는다.
2. **인덱스 전개:** 기존 §7.10.1–2의 NCHW/group/stride/pad/dilation 공식과 ConvAccessWitness로 free/bound axes, 유효 index/padding predicate, ordered fold를 산출한다. JAXA 원형은 계약이 확인될 때 groups=1, dilation=1, stride=(s,s), pad=(p,p)로 정규화할 수 있다.
3. **기호 parameter:** W는 ParameterRef(layer_instance, role=weight, resource_version, logical_shape)로, 선택적 bias는 별도 BiasRef로 나타낸다. 여기서 W는 J noun이나 physical buffer가 아니다. bias를 포함한 전체 affine Conv를 쌍선형이라고 부르지 않고, bilinear core와 bias Add를 분리한다.
4. **SPORES 후보:** IndexJoin(X,W) -> Multiply -> OrderedAggregate(+ optional Bias)로 export한다. 원본 축 이름·padding/fill·순서·provenance가 남는다. 단순 index relation의 기술 가능성과 실제 J 관측 동등성은 다르다. float reassociation, FMA, NaN/±0, integer overflow/promotion, empty/prototype, J error order, alias/state가 걸리면 Exact-J witness/guard 없이 e-class union을 금지한다. J 비교 허용오차 2^-44를 근사 rewrite 예산으로 자동 쓰지 않는다.
5. **복귀와 추출:** 후보는 original_call_id와 source/NAME/resource version 및 검증 의무를 함께 Graph rewrite candidate로 돌아온다. Logical IR에서 check/effect/error 및 shape·Rank를 검증하고, 실제 target lowering이 존재할 때만 실행 가능한 후보로 추출한다. 미지원 lowering은 비용만 높이는 방식이 아니라 추출 대상에서 제외한다.
6. **원본 보존/비용:** 증명 실패·불명·시간 초과·확장 비용 과다·타깃 미지원이면 원본 StructuredConv가 유지된다. sidecar 자체는 im2col, window, W buffer를 물리적으로 할당하지 않는다. Physical Planner가 나중에 buffer/parameter load, Direct Conv, GEMM, fused kernel과 스케줄을 결정한다.

~~~text
J source: (config conv2d)"rank -> derived function C1
             |
         Graph: StructuredConv(C1, X)   <-- authoritative
             |                 |
             |                 +--> semantic StateResource(weight@v, bias@v)
             v
       optional VirtualExpansion sidecar (non-executing)
         X + symbolic ParameterRef(C1, weight@v)
             -> index relation / ordered sum-product
             -> SPORES search (bounded)
             -> proof/guard + candidate extraction
             -> Logical IR verification
             -> selected physical route (or original Conv)
~~~

**명시적 adjoint.** 순수 bilinear core B_config(X,W)에 대해 내부 분석 연산자 BindRight(B_config, W@v) 및 BindLeft(B_config, X@snapshot)를 기호적으로 정의한다. 첫째의 Adj는 input adjoint(dX), 둘째의 Adj는 weight adjoint(dW)이며 bias adjoint는 별도 합산이다. 이는 사용자에게 bindL/bindR를 요구하거나 자동미분을 켜는 것이 아니다. 명시적으로 등록/호출한 adjoint/VJP만 Graph에 도출한다. dX에는 원본 input shape가 필요하다(stride>1에서 dY만으로 입력 크기가 유일하지 않을 수 있음). dW에도 원본 weight shape·X·dY·group/stride/pad/dilation이 필요하다. activation lifetime, gradient StateResource 누산, optimizer write의 효과와 버전을 보존한다. complex dtype은 inner product의 conjugation 계약을 별도로 정해야 한다.

**증분 처리·폭발 제어.** Conv algebra signature와 geometry witness는 식 자체를 복제하지 않는 bounded analysis sidecar로 캐시할 수 있다. key에는 relevant config/geometry, FunctionEntity/NAME binding, LayerInstanceId, state snapshot/version, dtype/numeric policy를 포함한다. 무효화·eviction을 제공하며 버전이 바뀐 weight를 같은 순수 parameter e-class로 간주하지 않는다. operator-level rewriting, index relation 탐색, relational saturation은 영역·node/time budget 및 rule whitelist로 제한한다. 정확한 합법성·numeric 검증 없이 operator identity만으로 Kernel fusion을 승인하지 않는다.

**수용 조건 (기존 NN-04/08, NN-G2, NN-AG, GF4/5a/6, FW/IO 체크리스트에 귀속; 새 병렬 로드맵 아님).**

| 시험 / 기존 담당 | 긍정·부정 사례 및 통과 의무 | 상태 |
|---|---|---|
| conv_jaxa_builder_schema / NN-04·08 | JAXA 2D 7개/3D 8개·depthwise/linear 인자, b_flag·rank/cell, 출력 shape. 누락·과잉·채널/geometry 오류와 unsupported dilation/groups 구분 | [ ] UNRUN |
| conv_parameter_identity / NN-G2·GF4 | 사용자/Graph에 weight 값 noun 요구 없음, C1/C2 별도 StateResource, 명시 공유/alias와 NAME rebind, update 전후 version 구분 | [ ] UNRUN |
| conv_virtual_export_roundtrip / GF4·GF5a | 원본 StructuredConv 보존; ParameterRef는 symbolic, index/padding/ordered fold/원본 source 보존, Unknown은 Opaque | [ ] UNRUN |
| conv_virtual_adjoint / NN-AG·GF5a | forward/dX/dW·bias shape, stride>1 크기 반례, activation snapshot과 gradient 누산/weight update 효과, 자동미분 부재 | [ ] UNRUN |
| conv_virtual_legality_extract / GF4·FW·GF6 | FP 재결합/NaN/±0/overflow, empty J 오류, state version mismerge 차단, 증명·lowering 부재시 원본 유지 | [ ] UNRUN |
| conv_virtual_budget_cost / GF6·IO | expansion/e-graph bounded, cache invalidation, legal direct/GEMM/fused 경로의 ResourceEstimate와 CostEstimate/CPU 시간 구분 | [ ] UNRUN |

**상태.** JAXA의 역사적 operand 원문과 구현 대조는 완료했지만, 이 절은 **설계 문서만 추가**한다. 현재 NN-04 parameter schema의 명시적 weight/bias 계약과 비노출 surface 간의 interface 정합, rank/dtype 구체 표기, Conv sidecar, SPORES exporter, e-graph union, adjoint lowering, 테스트/CI는 **미구현·UNRUN**이다. J 의미론을 변경하거나 기본 실행 경로를 바꾸지 않는다. M2→M3→M4와 §7.9 우선순위도 유지한다.

**출처:** [JAXA Appendix B](https://github.com/yunskim/JAXA-complier/blob/main/paper_draft_appendix_b.md), [JAXA shape inference](https://github.com/yunskim/JAXA-complier/blob/main/docs/JAXA/primitives.py), [JAXA custom adverb registry](https://github.com/yunskim/JAXA-complier/blob/main/jaxa_enqueue.py), [SPORES](https://www.vldb.org/pvldb/vol13/p1919-wang.pdf), [Glenside](https://ztatlock.net/pubs/2021-maps-glenside/), [RustJ StateResource](./15-advanced-contracts-p08.ko.md).


<a id="conv-virtual-expansion-examples"></a>
**7.10.9 Virtual Expansion 상세 예제집 — 계산값·관계형 전개·최적화 후보·거부 사례 (2026-10-14; 설계 전용).**

이 절의 J 표면 예제에서 conv2d builder와 해당 인자 배열은 **§7.10.8의 RustJ 제안 표기**이며 실행을 검증한 문법이 아니다. 과거 JAXA의 7-field configuration은 재사용하지만 JAXA의 RjP precision 확장을 일반 J rank 문법으로 가져오지 않는다. 배열 숫자 예제는 아래에 명시한 일반적 2D NCHW *cross-correlation* 및 정확한 실수 계산의 교육용 모델이다. 모든 X, W, Y의 단순화된 2D 표기에서는 크기 1인 batch·channel 축을 생략한다. 각 예제의 **virtual relation은 분석 산출물이지 실제 weight buffer, 추가 J noun, 변경된 Graph IR 또는 실행 검증의 증거가 아니다**.

**E1 — 3×3 입력, 2×2 kernel: 하나의 Structured Conv를 기호식으로 펼치기.**

~~~j
NB. 사용자 surface 후보; weight 값 선언 없음
C1 =: (1 1 2 2 1 0 0 conv2d)"3
model =: relu @: C1
~~~

* Configuration: C_in=1, C_out=1, kernel=(2,2), stride=1, pad=0, bias=0, cell rank=3.
* 예시 X (단일 채널): [[1,2,3],[4,5,6],[7,8,9]]; shape [1,3,3]. Y shape=[1,2,2].
* 원본 Graph: Input(X) → StructuredConv(C1,X) → ReLU → Output. 별도 user-level W 노드는 없다.
* State side table: LayerInstance(C1) → StateResource(weight_C1, logical_shape=[1,1,2,2], version=7). shape/version은 분석용 예시이며 실제 snapshot read가 아니다.

분석기가 VirtualExpansion(C1)을 요청하면 ParameterRef(weight_C1@v7)를 [[a,b],[c,d]]라는 **기호**로 표시할 수 있다. IndexJoin/Multiply/OrderedAggregate가 뜻하는 결과:

~~~text
Y[0,0] = 1a + 2b + 4c + 5d
Y[0,1] = 2a + 3b + 5c + 6d
Y[1,0] = 4a + 5b + 7c + 8d
Y[1,1] = 5a + 6b + 8c + 9d

free axes: output_h, output_w
bound axes: kernel_h, kernel_w
X_access = X[output_h + kernel_h, output_w + kernel_w]
W_access = ParameterRef(C1,weight@v7)[kernel_h,kernel_w]
index_join(X,W) -> multiply -> ordered_aggregate(kernel_h,kernel_w)
~~~

a,b,c,d는 실제 weight 값을 알거나 읽었다는 의미가 아니다. 검산용으로만 가상의 weight [[1,0],[0,-1]]을 대입하면 Y=[[-4,-4],[-4,-4]]가 된다. 실제 weight가 무엇이든 참조 관계는 동일하다. ReLU는 Conv의 ordered aggregate 바깥에 남으며 임의로 sum 안으로 이동하지 않는다.

**E2 — 동일 관계의 im2col/GEMM 후보: 관계식 생성과 물리 materialization은 다르다.**

E1의 네 output window를 *가상 행*으로 놓으면 다음 행렬 곱 후보가 만들어진다.

~~~text
VirtualPatchMatrix(X) =
  [[1,2,4,5],
   [2,3,5,6],
   [4,5,7,8],
   [5,6,8,9]]

VirtualWeightColumn(C1,version=7) = [[a],[b],[c],[d]]

Y_flat = VirtualPatchMatrix(X) @ VirtualWeightColumn(C1)
      = [a+2b+4c+5d, 2a+3b+5c+6d,
         4a+5b+7c+8d, 5a+6b+8c+9d]
~~~

이때 VirtualPatchMatrix는 **지표식의 표 보기**이지 별도 4×4 배열을 할당했다는 뜻이 아니다. 후보 경로는 DirectConv, WindowView+Contract, physical im2col+GEMM, fused on-the-fly patch GEMM 등이 가능하지만 모두 원래 Rank/cell, padding, 오류, numeric contract와 원소 순서를 검증한 뒤 cost 비교를 해야 한다. SPORES가 관계 동일성을 발견했다는 이유로 physical pack이나 GEMM 실행을 강제하지 않는다.

**E3 — 연속된 1×1 Conv 두 개: 실제로 찾을 수 있는 대수적 재작성 후보.**

~~~j
NB. 설명용 surface 후보; 두 layer 모두 bias 없음
C1 =: (1 2 1 1 1 0 0 conv2d)"3
C2 =: (2 1 1 1 1 0 0 conv2d)"3
model =: C2 @: C1
~~~

같은 공간 위치에서 W1=[a,b] (1→2), W2=[c,d] (2→1)라 하면 z0=a*x, z1=b*x, y=c*z0+d*z1. 가상 전개는 결합 가중치 k=(c*a+d*b)를 사용해 y=k*x인 1×1 Conv 후보를 제시한다.

~~~text
BEFORE: X -> Conv(C1, W1@v3) -> Z(2 channels) -> Conv(C2, W2@v8) -> Y
VIRTUAL: y = c*(a*x) + d*(b*x)
CANDIDATE: Y = Conv1x1(X, VirtualContract(W2@v8,W1@v3))
            effective_weight = c*a + d*b

AFTER only if legal:
  X -> FusedConv(C1,C2, resource dependencies preserved) -> Y
~~~

이 변환은 **실수대수에서만 무조건 성립**한다. 실제 floating 계산의 누산 순서/반올림/FMA가 바뀌므로 Exact-J 증명 없는 e-class union은 금지한다. 두 resource의 version과 쓰기·재바인딩 시점이 유지되어야 하며 훈련에서 각 weight gradient가 필요하다면 전개·materialization/activation 정책까지 보존해야 한다. 후보가 거부되더라도 원본 Conv→Conv는 그대로 실행 가능해야 한다. **입력 데이터가 바뀔 때마다 실물 k를 새 J weight noun으로 만들어 관리하도록 요구하지 않는다.**

**E4 — Conv→ReLU fusion은 가능해도 ReLU의 대수적 분배는 일반적으로 불가.**

~~~j
C1 =: (1 1 2 2 1 0 0 conv2d)"3
model =: relu @: C1
~~~

VirtualExpansion은 y=ReLU(Σ X*W)로 의미를 보존한다. ReLU(Σ X*W)와 Σ ReLU(X*W)는 일반적으로 같지 않다. 예를 들어 항이 2와 -3이면 전자는 ReLU(-1)=0이고, 후자는 2이다. 따라서 **ReLU를 contraction의 각 항으로 밀어 넣는 대수적 rewrite는 부정 사례**다. 반면 Conv와 ReLU를 하나의 physical kernel로 처리하면서 중간 Y의 저장을 생략하는 *fusion candidate*는 있다. 이 경우에도 다른 consumer의 fanout, numeric/Rank/error, target capability 및 resource feasibility를 따로 검증한다.

**E5 — Halo·부분 출력 demand: 전개하는 것은 필요한 인덱스 범위이지 곧바로 전체 tensor가 아니다.**

3×3 kernel, stride=1, dilation=1, padding=1이고 그림의 해당 타일이 실제 image 경계에서 충분히 떨어져 있다고 한다. output 좌표에서 y∈[10,18), x∈[10,18)인 **8×8 타일**을 소비한다면 (§7.10.3의 공식) 입력 X의 안전한 공간 bounding cover는 [9,19)×[9,19), 즉 **10×10**이다.

~~~text
Demand:    Y[10:18, 10:18]          -- 8x8
IndexMap:  X[y - 1 + r, x - 1 + s]  -- r,s = 0,1,2
Cover:     X[9:19, 9:19]            -- 10x10
Weight:    ParameterRef(C1, W@v7)   -- no materialization
~~~

stride/dilation이 1보다 크면 rectangular cover 안에 실제로 읽지 않는 hole이 있을 수 있다. **정확한 index union과 안전한 bounding cover는 서로 다른 fact**로 유지한다. 경계 clipping/zero fill은 실제 array 범위 밖 메모리를 읽는 것과 다르다. empty demand 역시 원래 J의 shape/prototype, required checks, first error 및 effect를 자동 제거하는 근거는 아니다.

**E6 — 명시적 input/weight adjoint: 같은 쌍선형 signature에서 유도하되 원본 ShapeSpec이 필수.**

bias를 뺀 B_config(X,W)에 대해 내부 분석 기호는 다음과 같다. 이는 사용자 J에 bindL/bindR를 강제하는 문법도, autograd 생성도 아니다.

~~~text
forward = B_config(X, W@v7)
L_W     = BindRight(B_config, W@v7)           : XSpec -> YSpec
R_X     = BindLeft(B_config, X@snapshot)      : WSpec -> YSpec

dX = Apply(Adj(L_W), dY)   : YSpec -> XSpec
dW = Apply(Adj(R_X), dY)   : YSpec -> WSpec
db = Reduce(dY over batch/spatial axes)      -- optional, separate affine bias
~~~

예: kH=kW=3, stride=2, pad=1이면 입력 H=5와 H=6 **모두 출력 H'=3**을 만들 수 있다. 따라서 dY와 W만으로 원본 dX의 크기를 유일하게 복원할 수 없다. 원래 XSpec과 weight ShapeSpec, group/dilation/stride/padding, batch/frame 및 activation snapshot이 필요하다. 실제 dX는 경계·stride를 반영한 gather/scatter/transpose kernel 후보이고 dW는 batch·공간축 누산 후보다. 두 후보는 오직 등록된 explicit adjoint/VJP 요청 경로에 대해서만 생성한다. W에 대한 gradient는 optimizer가 관리하는 StateResource 누산과 별개의 의미적 출력·효과 계약을 갖는다.

**E7 — 숨은 weight의 *동일성*과 값의 *동일성*은 다르다: 공유·업데이트 부정 사례.**

~~~text
C1: ConvConfig(in=1,out=2,k=1) -> LayerInstanceId(101)
C2: ConvConfig(in=1,out=2,k=1) -> LayerInstanceId(102)

ParameterRef(101,weight,v4) != ParameterRef(102,weight,v4)
     even when shapes and current numerical contents match

ParameterRef(101,weight,v4) != ParameterRef(101,weight,v5)
     when an optimizer update occurs between reads

Shared C1/C3 is legal only with witnessed common StateResourceId
and a compatible snapshot/version across the particular uses.
~~~

동일한 shape, 동일한 source spelling 또는 우연히 같은 값만으로 weight를 공유한다고 추론하지 않는다. optimizer 업데이트의 전후 Conv를 하나로 CSE하면 잘못된 계산이다. 명시적으로 같은 state resource를 공유하더라도 읽기 버전·오류·효과·사용 조건이 다르면 합칠 수 없다. 실제 가중치는 계속 외부 StateResource에 있으며 derived verb의 은닉 mutable field가 아니다.

**E8 — 한눈에 보는 sidecar 왕복, 반례 및 실행 fallback.**

~~~text
source:    C1 =: (1 1 2 2 1 0 0 conv2d)"3
           model =: relu @: C1
                 |
semantic: DerivedFunction(C1, config, rank), binding/version
                 |
Graph IR: Input -> StructuredConv(call_17,C1) -> ReLU
                 |  authoritative; do not mutate
                 +-- state table: C1 -> weight_C1@v7
                 |
                 v   [optional, lazy, non-executing]
sidecar:   ConvAlgebraView(call_17,
             X indexed join ParameterRef(weight_C1@v7),
             OrderedAggregate, source/geometry/check witnesses)
                 |
SPORES:    original + relational candidates
                 |
gate:      proven J semantics? correct versions/guards?
           executable Logical IR/target route? lower cost?
                 |
           +-----+------+
           |            |
         YES           NO / UNKNOWN / TIMEOUT / UNSUPPORTED
           |            |
           v            v
    accepted candidate  original StructuredConv
           |            |
           +-----+------+
                 |
          Logical IR / Physical Planner
~~~

| 상황 | 가상 전개 가능성 | 최적화 승인 |
|---|---|---|
| 유효한 k=2 Conv와 안정된 W snapshot | yes: E1/E2의 index relation | 실제 실행 rewrite는 별도 증명 |
| Conv→ReLU, 단일 consumer | yes: Conv 내부만 전개 | fused kernel은 후보, 분배법칙은 거부(E4) |
| 두 1×1 Conv와 불변 weight versions | yes: E3의 수학적 병합 후보 | 수치·학습·상태·kernel 검증 이후에만 |
| 업데이트 중 서로 다른 W versions | 버전을 분리하면 기술 가능 | 서로 다른 snapshot의 CSE/병합은 거부(E7) |
| shape/rank/dtype/parameter binding 미확정 | Opaque 또는 guarded analysis | 무증명 추출 거부 |
| proof 실패, e-graph 예산 초과, target 미지원 | 원본은 계속 존재 | 원본 StructuredConv로 fallback |

**테스트 수용 추가 (기존 §7.10.8 테스트 항목에 사례 연결, 모두 [ ] UNRUN).** E1 symbolic 4-cell 전개와 예제 W 검산, E2 가상 4×4 patch와 실제 할당 부재, E3 1×1 두 계층의 algebra candidate 및 IEEE negative, E4 ReLU 비분배, E5 8×8→10×10 exact/cover·빈 결과, E6 stride=2에서 H=5/6의 dX shape 부정 사례, E7 자원 identity/version/명시 공유, E8 original-call roundtrip·unsupported/fuel-exhaustion/target-failure를 각각 기존 conv_virtual_export_roundtrip, conv_virtual_legality_extract, conv_virtual_adjoint, conv_parameter_identity, conv_virtual_budget_cost로 귀속한다. 모든 시나리오는 documentation fixture일 뿐 실행 코드/CI·C oracle·target 검증 **UNRUN**이다.

**재사용 가능한 원칙.** VirtualExpansion은 Conv에만 하드코딩하는 실행 pass가 아니라, graph의 고수준 primitive가 선택적으로 제공하는 **AlgebraSignature → symbolic index/parameter relation → proven candidate → verified extraction**이라는 공통 분석 인터페이스로 일반화할 수 있다. Linear·MatMul·tensor contraction, attention의 *적격하고 효과가 분리된 contraction 구역*에도 확장 후보가 있으나, softmax, RNG/dropout, stateful normalization까지 임의의 sum-product 법칙으로 재작성할 수 있다는 뜻은 아니다. 원본 high-level identity가 기본 경로이고 sidecar는 요청할 때만 생성된다.


<a id="conv-virtual-expansion-related-work"></a>
**7.10.10 Virtual Expansion 선행 연구 대조 — 구현 시의 유사성과 대수 분석 계약의 경계 (2026-10-14; 문헌 검토·설계 전용).**

**요약.** **구조화된 고수준 표현을 유지하다가, 필요할 때 인덱스·관계·연산자 의미를 펼쳐 분석한다**는 발상은 새롭거나 RustJ만의 것이 아니다. 다음 연구들은 각각 가상 행렬을 실행 중 생성, 텐서 access pattern을 재작성, 고수준 그래프의 대체식을 탐색, 출력 수요에서 입력 범위를 유도, affine index를 모델링, DB view를 펼침, matrix-free 선형 변환을 구성하는 선례를 제공한다. 공통된 비유를 그대로 하나의 알고리즘/IR 또는 이미 증명된 RustJ 동등성으로 혼동해서는 안 된다. 아래 각 행의 **원문에서 확인한 성질**, **RustJ의 구체 적용**, **동일시할 수 없는 경계**를 분리했다.

| 선행 연구 / 근거 | 원문으로 확인한 기술 및 가장 가까운 예제 | RustJ에서 재사용할 아이디어 | 다른 점과 금지할 추론 |
|---|---|---|---|
| **NVIDIA CUTLASS implicit GEMM / implicit im2col** — [공식 알고리즘 문서](https://github.com/NVIDIA/cutlass/blob/main/media/docs/cpp/implicit_gemm_convolution.md), [Conv 매핑 코드](https://github.com/NVIDIA/cutlass/blob/main/include/cutlass/conv/convolution.h) | 일반 im2col의 **전체 복제 행렬**을 만들지 않고 GPU가 global memory에서 읽을 때 index/predicate로 GEMM에 필요한 **타일**을 구성한다. Forward/dgrad/wgrad 매핑도 제공한다. **E2**의 가장 직접적인 실행 대응 | Graph의 virtual patch relation에서 물리적 on-the-fly gather/implicit-GEMM lowering 후보 생성 | CUTLASS는 **커널 실행/물리적 타일 구성**이며 RustJ VirtualExpansion은 **컴파일러 분석용 sidecar**다. full im2col이 없다는 말은 shared-memory/register staging도 없다는 뜻이 아니다. CUTLASS 자체가 SPORES e-class 증명/원본 Graph 보존 정책을 제공한다는 주장 금지 |
| **Glenside** — Smith 외, *Pure Tensor Program Rewriting via Access Patterns* (MAPS 2021), [논문·저자 원문](https://ztatlock.net/pubs/2021-maps-glenside/) | 순수 tensor IR의 **access pattern**을 중심으로 layout-aware term rewrite를 수행하며 **im2col 변환의 자동 탐색**을 시연했다. **E1/E2**의 대수 표현 측면에서 가장 가까운 직접 선행 연구 | primitive별 optional AlgebraSignature를 access-pattern/index rewrite에 연결하고, EqGraph 후보를 비파괴적으로 탐색하는 연구 출발점 | Glenside의 자체 pure IR와 RustJ의 **full-J source/FunctionEntity + 외부 StateResource + optional sidecar**가 동일 구조는 아니다. RustJ의 모든 숫자·효과 관측 동등성은 별도 과제 |
| **TASO** — Jia 외, *TASO: Optimizing Deep Learning Computation with Automated Generation of Graph Substitutions* (SOSP 2019), [논문](https://web.stanford.edu/class/cs245/readings/taso.pdf), [프로젝트](https://catalyst.cs.cmu.edu/projects/taso.html) | 텐서 연산자의 명세/대수적 성질에서 graph substitution **후보를 생성하고 검사**하며 비용 기반 그래프 탐색을 수행한다. **E3/E4/E8**의 규칙 생성·승인에 선례 | 고수준 operator property를 단순한 kernel 정의와 분리하고, rewrite마다 적법성 witness/반례를 수집 | TASO는 이미 operator properties 및 검증을 사용했다. 따라서 **대수 signature를 primitive 계약으로 둔다는 발상 자체가 최초**라고 주장하지 않는다. TASO의 성질 검증을 곧바로 J Rank/NAME/오류/IEEE 동등성 증명이라고 간주하지 않는다 |
| **TenSat** — Yang 외, *Equality Saturation for Tensor Graph Superoptimization* (MLSys 2021), [학회 원문](https://proceedings.mlsys.org/paper_files/paper/2021/hash/cc427d934a7f6c0663e5923f49eba531-Abstract.html) | 순차적 치환의 phase-ordering 한계를 넘어 **e-graph equality saturation + extraction**으로 텐서 그래프를 탐색한다. **E3/E8**에 대응 | Rule whitelist, typed e-class/shape facts, budget, target-cost extraction 및 original candidate 보존 | TenSat의 tensor equality를 무조건 J **observational equality**로 옮길 수 없다. 미지원 residual expression은 **높은 비용**이 아니라 **추출 불가**이며, 근사 수치 후보는 별도 정책이 필요 |
| **Halide bounds inference** — [Halide Func::infer_input_bounds](https://halide-lang.org/docs/api/generated/Func.html), [BoundsInference pass](https://github.com/halide/Halide/blob/main/src/BoundsInference.cpp) | 주어진 출력 크기에서 필요한 입력 이미지/함수의 범위를 **호출 인덱스 관계로부터 역추론**한다. **E5의 8×8 output → 10×10 input halo**와 직접 대응 | output-demand→input-region pullback, halo, clipping, tile-dependent read set과 conservative cover 계산 | Halide의 bounds analysis 자체는 SPORES sum-product/e-class 증명이 아니다. 계산된 cover가 실제 최소 read 집합인 것도 아니며 J의 빈 셀·필요 검사·first error 제거 허가는 아니다 |
| **폴리헤드럴 컴파일(isl / Pluto / Tiramisu)** — [isl 공식 매뉴얼](https://libisl.sourceforge.io/user.html), [Tiramisu CGO 2019](https://arxiv.org/abs/1804.10694), [Tiramisu 구현](https://github.com/Tiramisu-Compiler/tiramisu) | affine 정수 제약의 **iteration domain, access relation, dependence 및 schedule**를 모델링한다. isl은 affine set/relation 및 schedule tree를 다루며 Tiramisu는 알고리즘/스케줄/배치의 분리를 제공한다. **E2/E5** 인덱스 관계에 선례 | h*stride + r*dilation - pad와 같은 (관련 값이 고정되거나 symbolic parameter인) affine access를 별도 분석 대상으로 표현; 복잡한 bounds/tiling 조건을 정확하게 계산 | 데이터에 따라 바뀌는 비-affine index, opaque predicate, effect가 있으면 일반 polyhedral 영역 밖일 수 있다. integer index legality는 J floating algebra legality와 다르며, schedule/physical layout을 Graph IR에 조기 주입하지 않는다 |
| **DB view unfolding** — [PostgreSQL 공식 Views and the Rule System](https://www.postgresql.org/docs/current/rules-views.html) | query rewriter가 view의 저장된 정의를 **query tree의 subquery로 확장**하고 원 질의의 관계와 함께 planner가 탐색하게 한다. **E8의 의미 정의 on-demand 공개**에 구조적 유사성 | opaque/high-level 연산에서 최적화용 관계 정의를 요청 시 내보내고 provenance와 역방향 매핑을 유지 | PostgreSQL은 실제로 query tree를 **재작성**한다. RustJ는 원본 StructuredConv를 authoritative로 남기는 **비파괴 sidecar**를 제안한다. DB view unfolding이 원본 Graph 자동 복귀·증명된 kernel extraction을 보장한다고 쓰지 않는다 |
| **Matrix-free 선형 연산자: SciPy LinearOperator / PyLops** — [SciPy 공식 API](https://docs.scipy.org/doc/scipy/reference/generated/scipy.sparse.linalg.LinearOperator.html), [PyLops 공식 API](https://pylops.readthedocs.io/en/stable/api/generated/pylops.LinearOperator.html) | 거대한 행렬을 만들지 않고 **matvec/apply**와 **rmatvec/adjoint**를 노출한다. 선형 연산자의 합성도 지연 정의할 수 있다. **E6의 BindRight/BindLeft/Adj**와 수학적으로 가깝다 | B(X,W)에서 W snapshot을 고정한 input-linear operator와 X snapshot을 고정한 weight-linear operator의 명시적 adjoint 계약 | 물리 행렬이 없다는 사실만으로 SPORES식 sum-product rewriting이나 J mutable weight/resource semantics가 구현되는 것은 아니다. complex에서 transpose와 Hermitian adjoint/conjugation을 구분 |
| **JAX linear_transpose / primitive transposition** — [공식 linear_transpose](https://docs.jax.dev/en/latest/_autosummary/jax.linear_transpose.html), [JAX primitive transposition 설명](https://docs.jax.dev/en/latest/601/jax-primitives.html), [JVP/VJP 설명](https://docs.jax.dev/en/latest/301/cookbook.html) | 선형이라고 **약속된 함수**를 transpose하고, JVP에서 등장하는 primitive에 대한 transpose rule을 조합해 역방향 선형 연산을 만든다. **E6의 선형화와 전치**에 직접 대응 | linearity/in-out ShapeSpec을 검사하는 Adj operator signature, registry 기반 transpose/lowering proof, transpose 복합 규칙 | JAX는 이 규칙을 **autodiff 경로**에 사용한다. RustJ는 사용자가 **명시한 adjoint/VJP만** 연결하고 원 함수의 자동 미분 생성을 강제하지 않는다. dtype/complex conjugation과 stride에 의한 원본 입력 shape 정보를 추가 계약으로 보존 |

**정확한 비교 관점 — 세 가지 '가상'을 구분한다.**

~~~text
1. Algebraic / compiler analysis: RustJ VirtualExpansion, Glenside access rewrites,
   Tensat candidate search, DB-like view unfolding analogy
   -> index and algebra relation is represented; not necessarily executed.

2. Demand / iteration-domain analysis: Halide bounds, isl/Pluto/Tiramisu
   -> required index set, halo, affine schedule/dependence are analyzed.

3. Runtime realization: CUTLASS implicit GEMM, physical matrix-free matvec
   -> addresses/tiles are generated when operations actually run.

RustJ combines (1) with (2) as an optional sidecar, and only after proof
and extraction may choose an implementation in (3). None is the other.
~~~

**공통 algebra signature의 권장 최소 인터페이스 (연구용, 아직 Rust 타입이 아님).** TASO/Glenside의 원칙을 참고하되, 기존 RustJ Graph IR/StateResource 모델과 충돌하지 않도록 **필수 기능**과 **선택적 분석용 정보**를 나눈다.

~~~text
AlgebraSignature {
  semantic_operation: FunctionEntity / StructuredOp ID
  operand_roles: runtime input, optional parameter references, scalar config
  type_and_axes: dtype, ShapeSpec, input/output rank/cell/frame, axis roles
  symbolic_access: output-to-input/index relation, optional padding/fill predicates
  fold_semantics: bound axes, fold identity/order, numeric policy
  operator_traits: linearity or bilinearity in SPECIFIED operand position;
                   optional explicit-adjoint registry key
  effects_and_provenance: required checks, error precedence, source occurrence,
                          StateResource read identity and snapshot/version guards
  available_views: optional relational/sum-product/index-only expansion
  admission: relation witness != EqGraph union proof != target-lowering proof
  fallback: original StructuredOp remains authoritative
}
~~~

여기서 **입력에 대한 선형성**과 **weight에 대한 선형성**은 bias를 분리한 bilinear core에 한정한다. ReLU/softmax/stateful normalization을 같은 trait로 표시하지 않는다. 정수·complex·J empty/prototype과 effective rank가 미확정이면 완전한 signature가 아니라 Unknown/Opaque 또는 동적 guard를 반환한다. 이 인터페이스는 RustJ의 새로운 강제 IR 노드가 아니라 **primitive가 제공할 수 있는 선택적 계약/분석 exporter**로 설계한다.

**관련 연구에서 얻는 구현 선택과 실험 연결 (모두 아직 미구현/UNRUN).**

| 선행 연구에서 유도되는 검증 질문 | 기존 §7.10.8 소유자/연결 예제 | 상태 |
|---|---|---|
| CUTLASS 대비: 가상 patch가 물리적으로 할당되지는 않았는가? 실제 Direct/implicit GEMM/packed GEMM bytes·latency를 구분했는가? | conv_virtual_budget_cost, E2 | [ ] UNRUN |
| Glenside/TASO/TenSat 대비: typed index/operator signature로 타당한 rewrite 후보를 만들며, unsupported numeric/error/state 가정은 union 없이 기각하는가? | conv_virtual_export_roundtrip / conv_virtual_legality_extract, E3/E4/E8 | [ ] UNRUN |
| Halide/isl 대비: 정확한 수요 집합·보수적 halo·empty/bounds/stride holes를 구분하는가? | conv_region_halo / conv_virtual_export_roundtrip, E5 | [ ] UNRUN |
| DB view unfolding 대비: sidecar를 버려도 원본 StructuredConv/NAME provenance가 그대로 실행 가능한가? | conv_candidate_fallback / conv_virtual_export_roundtrip, E8 | [ ] UNRUN |
| SciPy/PyLops/JAX 대비: input/weight 별도의 올바른 explicit adjoint, 원본 ShapeSpec·conjugation·snapshot, autograd 비강제 규약을 만족하는가? | conv_virtual_adjoint / conv_parameter_identity, E6/E7 | [ ] UNRUN |

**연구 기여 주장 범위.** 선행 연구가 이미 **연산자의 수학적 성질을 계약으로 주는 방법(TASO)**, **access pattern을 rewrite IR의 핵심으로 두는 방법(Glenside)**, **e-graph saturation/extraction(TenSat)**, **실행 시 가상 im2col을 만드는 방법(CUTLASS)**을 보여 주었다. 따라서 'Virtual Expansion 최초 발명'이나 '대수 signature를 1급으로 다루는 최초 컴파일러'라는 표현은 사용하지 않는다. RustJ의 **잠정적인 설계 조합**은 (a) J의 원본 modifier/derived verb와 StructuredOp identity 보존, (b) 사용자 가중치 값 비노출과 별도 명시적 StateResource/version, (c) 필요할 때만 sum-product/access/operator-view 제공, (d) Exact-J numeric·Rank·NAME·error/effect 관문 및 실현 가능한 후보만 추출, (e) explicit adjoint와 원본 Graph fallback을 **같은 계약**으로 연결하는 데 있다. 이것이 학술적으로 기존 연구와 구별되는 *새로운 기여인지*는 더 완전한 선행 문헌 조사와 구현/실측/반례 검증 없이는 단정할 수 없다.

**문헌 검증 상태:** 위 문헌의 논문 초록/원문, 작성자·공식 기술 문서/API로 각 대응 관계를 확인했다. 단, 모든 구현 세부에 대한 source-level 감사나 문헌의 기계 증명 재현을 수행한 것은 아니다. RustJ의 sidecar, rewrite/proof, target 실현/성능 결과는 이 절 추가로 구현되거나 검증된 것이 아니다. **설계 문서만 수정, 기존 M2→M3→M4 관문과 NN/GF/FW/IO의 [ ] UNRUN 상태 유지.**

**관련 출처:** [SPORES 원 논문](https://www.vldb.org/pvldb/vol13/p1919-wang.pdf), [Glenside](https://ztatlock.net/pubs/2021-maps-glenside/), [MLIR Linalg](https://mlir.llvm.org/docs/Dialects/Linalg/), [공통 접근 대수 R0](../../research/common-array-access-composition.ko.md), [ExecutionBasisExpansion](./15-advanced-contracts-p10.ko.md#executionbasisexpansion).



<a id="conv-virtual-expansion-walkthrough"></a>
**7.10.11 Virtual Expansion 입출력과 최적화 전 과정 — 분석 요청에서 후보 선택까지 (2026-10-14).**

이 절은 §7.10.8–10의 설계를 **입력 → 분석 산출물 → 후보 → 승인/거부 → 실행 경로** 순서로 읽는 안내다. 아래 요청 객체·RuleId·후보 이름은 설명용 의사 인터페이스이며 현재 Rust API가 아니다. 산술·배열 크기는 검산한 예제이고, RustJ Conv exporter/SPORES/커널 실행·성능 검증은 여전히 **UNRUN**이다. 기존 NN/GF/FW/IO 원장을 확장해 읽으며 새 완료 체크리스트를 만들지 않는다.

**A. 무엇을 입력으로 받는가?** 사용자 프로그램, compiler가 보유한 의미적 facts, 선택적 output demand, downstream의 물리 실행 조건은 서로 다른 입력이다. 실제 weight 원소를 받아야 index 관계를 만들 수 있는 것은 아니다.

| 제공자 | 이 예제의 입력 | 용도와 미확정 시 처리 |
|---|---|---|
| J source / parser | `(3 16 3 3 1 1 1 conv2d)"3`, 뒤의 `relu`; 원본 FunctionEntity·source span·ordinary NAME binding | 설명용 builder 후보. 품사/서명/NAME 확인이 안 되면 Conv라고 추측하지 않음 |
| call analysis | XShape=[1,3,64,64], WShape=[16,3,3,3], YShape=[1,16,64,64]; rank-3 cell, batch frame [1]; groups=1, dilation=1 | NCHW cross-correlation의 axis·bounds·padding 관계. 이 값들은 이 예제의 명시된 사양이며 현재 등록된 API의 자동 기본값이 아님 |
| semantic resource owner | layer L17, weight Rw@v7, bias Rb@v3, source/binding epoch; immutable snapshot 또는 ordered-read 의무 | 값 대신 기호 참조. 버전·공유·effect를 모르면 Opaque/guarded로 유지 |
| numeric/error contract | float32 storage, 명시된 accumulator/rounding·fold 순서, bias→ReLU 순서, empty/fill·first-error 의무 | dtype만으로 재결합/FMA 허가를 추론하지 않음. 이 예제는 c,r,s 순서를 명시하나 최종 RustJ extension 계약은 별도 수용 필요 |
| consumer demand | n=[0,1), o=[0,16), y=[10,18), x=[10,18) | 최종 출력의 8×8 타일. demand가 없으면 전체 출력; 작은 demand도 원래 오류·효과를 지울 권한이 아님 |
| downstream planner | 실제 target capability, layout, patch scratch 제한 8 KiB, 경험적 cost 정보 | **VirtualExpansion 뒤에** 후보를 거르는 물리 조건. source 의미/FunctionEntity identity의 일부가 아님 |

~~~text
ExpansionRequest (proposed):
  original_call=L17; operation=StructuredConv; input_value=Vx
  geometry=(Cin=3,Cout=16,k=3x3,stride=1,pad=1,groups=1,dilation=1)
  state=(ParameterRef(Rw,v7),BiasRef(Rb,v3))
  facts=(XShape,YShape,WShape,Rank,NumericContract,Checks,Effects)
  demand=Y[0:1,0:16,10:18,10:18]
  limits=(node_budget,time_budget); required_provenance=(source,binding)

ExpansionResult (proposed):
  original_call=L17                -- authoritative Graph is retained
  free_axes=(n,o,y,x); bound_axes=(c,r,s)
  X_access=(n,c,y-1+r,x-1+s); W_access=(o,c,r,s)
  predicates=(0<=c<3,0<=r,s<3,valid_X_or_zero_padding)
  fold=ordered(c,r,s); post_ops=(bias,ReLU)
  parameter_versions=(Rw@v7,Rb@v3)
  demand_pullback + witnesses + unknowns + proof_obligations
~~~

전개의 결과는 weight 값이나 patch 배열이 아니다. 입력 값 Vx와 기호 parameter를 결합하는 index relation, ordered reduction, 원본으로 돌아가는 provenance다. padding 밖 좌표는 해당 계약의 zero-fill이며 실제 범위 밖 load가 아니다. 출력 shape 검증·빈 cell 검사·ordered state read는 data demand와 별개의 의무로 남는다.

**B. Conv→ReLU, 8×8 출력 타일의 최적화 기록.** 전개된 식은 다음과 같다. bias 뒤 ReLU 적용을 유지한다.

~~~text
Z[n,o,y,x] = OrderedSum_(c,r,s)
    X_or_zero[n,c,y-1+r,x-1+s] * ParameterRef(Rw@v7)[o,c,r,s]
    + BiasRef(Rb@v3)[o]
Y[n,o,y,x] = ReLU(Z[n,o,y,x])

Demand(Y): [0:1,0:16,10:18,10:18]
Pullback(X): [0:1,0:3,9:19,9:19]        -- 300 atoms
Pullback(W): [0:16,0:3,0:3,0:3]        -- 432 atoms
Pullback(bias): [0:16]                 -- 16 atoms
~~~

이 interior 타일은 image 경계에 닿지 않고 stride/dilation=1이므로 입력 공간 union이 정확히 10×10이다. 다른 stride/dilation에서는 직사각형 cover와 실제 index 집합을 별도로 산출한다. 이것은 필요한 값의 영역이지 source의 모든 오류/효과를 생략해도 된다는 증명이 아니다.

| 항목 | 전체 출력 기준 | 요청 타일 기준 | 해석 |
|---|---|---|---|
| Conv 출력 원소 | 16×64×64=65,536 | 16×8×8=1,024 | bias/ReLU 산술은 별도로 계산 |
| multiply-accumulate 쌍 | 65,536×27=1,769,472 | 1,024×27=27,648 | 구현의 FLOP 수 또는 실행 시간 실측이 아님 |
| conceptual patch matrix | 4,096×27=110,592 float32, **432 KiB** | 64×27=1,728 float32, **6.75 KiB** | 실제 packing을 선택할 때만 allocation 후보; 분석 relation 자체는 0 bytes of patch storage |
| Z 중간 결과 저장 | 65,536×4=256 KiB | 1,024×4=4 KiB | sole-consumer Conv→ReLU fusion에서 생략 가능한 후보. fanout이면 liveness/소비자 의미를 재검사 |

여기서 8 KiB는 **patch scratch만**의 예산이다. input/weight/output, accumulators, shared/register staging와 lease를 포함한 총 peak-memory 한도가 아니다. 임시 배열 bytes 감소를 runtime latency 향상으로 환산하지 않는다.

| 후보 / 생성 근거 | 의미 관문 | 물리 관문과 이 예제의 결론 |
|---|---|---|
| Original StructuredConv | source·state·checks 보존 | 실제 지원되는 original route가 있으면 baseline으로 유지; 없으면 UnsupportedRoute |
| Full im2col→GEMM / 가상 window→matrix 관계 | fold 순서·FMA·padding/type/error 동등성 필요 | 432 KiB patch는 8 KiB patch 제한을 초과해 거부. 비용이 높다는 이유가 아니라 자원 부적격 |
| Tiled pack→GEMM / demand를 64개 row로 제한 | demand pushback·checks·numeric witness 필요 | patch 6.75 KiB는 해당 제한만 통과. 나머지 자원·kernel·정확성은 별도로 검사 |
| Direct tiled Conv→ReLU / 같은 fold와 post-op 보존 | 다른 Z consumer 없음, rounding·error·effect 동일, alias/lease 안전 | full patch 불필요. 실제 direct/fused route가 있고 관문을 통과할 때만 후보 선택 가능 |
| Implicit GEMM / 주소로 virtual patch를 생성 | GEMM 누산/타깃 numeric 계약 증명 필요 | full global patch는 없지만 tile staging·layout conversion 비용은 남음. CUDA 부재면 CPU direct의 대체 실행 경로로 취급하지 않음 |

설명용 candidate ledger의 결정은 `full_pack: Reject(Resource)`, `tile_pack: Needs(NumericWitness,Kernel)`, `direct_fused: Needs(Effect/NumericWitness,Kernel)`, `implicit_gemm: Reject(NoTargetRoute)`다. **현재 RustJ에서는 해당 Conv 경로가 구현되지 않았으므로 이 예제의 실제 실행 결론은 UnsupportedRoute/UNRUN이다.** 향후 가령 direct_fused만 검증된 CPU 경로이고 측정 비용도 낮다면 그 후보를 선택할 수 있다. 이것이 후보 생성, 적법성, 자원 검사와 최종 extraction을 분리하는 이유다.

**C. 값이 있는 재현 예와 값 없이 하는 분석.** §7.10.9 E1/E2의 작은 입력을 독립 Python 산술로 재현하면 아래와 같다. 값 W는 **문서 검산용 fixture**이며 RustJ 사용자 surface나 분석 요청이 W noun을 요구한다는 뜻이 아니다.

~~~python
X = [[1, 2, 3], [4, 5, 6], [7, 8, 9]]
W = [[1, 0], [0, -1]]
patches = [[X[y+r][x+s] for r in range(2) for s in range(2)]
           for y in range(2) for x in range(2)]
flat_w = [W[r][s] for r in range(2) for s in range(2)]
direct = [[sum(X[y+r][x+s] * W[r][s]
               for r in range(2) for s in range(2))
           for x in range(2)] for y in range(2)]
via_patch = [sum(a*b for a, b in zip(row, flat_w)) for row in patches]
assert direct == [[-4, -4], [-4, -4]]
assert via_patch == [-4, -4, -4, -4]
~~~

Python이 여기서 patches를 실제 list로 만드는 것은 독립 정답 검산을 위한 것이다. VirtualExpansion은 같은 row의 **좌표 매핑만** 저장하고 a,b,c,d는 기호로 남긴다. 이 정수 fixture 통과는 일반 float32 GEMM의 재결합·FMA·J error equivalence를 증명하지 않는다.

**D. 두 1×1 Conv의 실제 후보와 거부 입력.** config=(1→2→1), bias 없음, 같은 spatial domain, resource W1@v3=(a,b), W2@v8=(c,d)에 대해 `c*(a*x)+d*(b*x)`에서 `(c*a+d*b)*x`라는 후보를 생성한다. x=2,a=3,b=4,c=5,d=6이면 원본 5×6+6×8=78, 후보 k=39 및 k×2=78이다. 이는 수학적 후보를 설명하는 작은 정수 예다.

그러나 binary64 입력 `a=1e308,b=1e308,c=1,d=-1,x=2`에서는 원본의 두 첫 곱이 +Inf이고 다음 합은 `Inf + -Inf = NaN`인 반면, 후보는 k=0 및 0×2=0이다. 따라서 **같은 기호식 후보라도 Exact-J 승인에는 실패**한다. 이 Python/IEEE 반례는 재결합 위험의 검산이며 pinned J-C 실행 결과로 보고하지 않는다. weight write가 v3→v4를 만들거나 중간 Z의 다른 consumer/각 layer gradient가 필요하면 state/fanout/adjoint 관문도 독립적으로 실패할 수 있다. 승인 거부는 원본 graph 보존으로 이어지고 실행 가능한 원본 경로가 없으면 UnsupportedRoute다.

**E. 같은 입력을 다른 시스템이 어떻게 다루는가?** 다음 비교는 §7.10.10의 문헌을 이 요청에 적용한 해석이다. RustJ column은 **제안 설계**다. 라이브러리별 성능 순위가 아니다.

| 시스템 / 공식 근거 | 받는 입력과 실제 역할 | 이 예제에서 대응되는 부분 / RustJ의 독립 의무 |
|---|---|---|
| [SPORES, PVLDB 2020](https://www.vldb.org/pvldb/vol13/p1919-wang.pdf) | 선형대수 식을 관계대수로 옮겨 equality saturation 후 LA plan으로 추출 | IndexJoin·aggregate 기반 후보 탐색의 참고. opaque Conv를 자동 인식하거나 J state/IEEE/effect를 증명한다는 뜻이 아님 |
| [Glenside, MAPS 2021](https://ztatlock.net/pubs/2021-maps-glenside/) | pure tensor의 access pattern 표현과 rewrite; im2col 변환 탐색을 시연 | virtual patch의 인덱스 표현과 layout 후보 참고. NAME/resource version과 J 오류의 증거는 별도로 필요 |
| [CUTLASS implicit GEMM](https://github.com/NVIDIA/cutlass/blob/main/media/docs/cpp/implicit_gemm_convolution.md) | 실제 activation/filter tensor와 CUDA kernel configuration; load 때 convolution-matrix 타일을 형성 | B의 implicit_gemm 물리 후보와 대응. 분석-only 투영과 다르며 shared/register staging는 존재. 이 문서의 NCHW cross-correlation과 CUTLASS 예제의 layout/filter 방향도 adapter에서 확인 |
| [Halide bounds inference](https://github.com/halide/Halide/blob/main/src/BoundsInference.cpp) | pipeline의 호출 인덱스에서 필요한 함수 영역을 추론 | 8×8 demand→10×10 halo의 참고. bounds 추론만으로 ordered aggregate 재작성이나 e-class union을 승인하지 않음 |
| [SciPy LinearOperator](https://docs.scipy.org/doc/scipy/reference/generated/scipy.sparse.linalg.LinearOperator.html) | shape/dtype 및 matvec, 선택적 adjoint callable; matrix 없이 적용·지연 합성 | W snapshot을 고정한 Conv의 input-linear operator/explicit adjoint와 대응. callable만으로 sum-product 내부가 드러나지는 않음; 자동 e-graph 탐색기라고 간주하지 않음 |

RustJ 제안의 추가 연결은 **원본 FunctionEntity/StructuredConv ↔ source와 state version을 가진 분석 relation ↔ witnessed candidate ↔ 실제 supported Logical/Physical route**다. 사용자는 layer 구성과 X를 제공하며, compiler는 분석에 필요한 좌표·parameter identity를 공개하고 runtime은 선택된 경로에서 실제 parameter를 읽는다. 이 세 시점을 분리해야 값을 읽지 않는 가상 전개와 실제 실행 최적화를 혼동하지 않는다.

**검증 기록과 남은 작업.** 이 절의 정수 fixture, IEEE 반례, tile/patch bytes·MAC 쌍 산술 및 문서 링크는 별도로 검사한다. exporter roundtrip/demand witness는 `conv_virtual_export_roundtrip`·`conv_region_halo`, FP 반례는 `conv_virtual_legality_extract`, version/fanout는 `conv_parameter_identity`, resource/route/cost는 `conv_virtual_budget_cost`에 연결한다. 기존 구현 수용 항목은 모두 [ ] UNRUN이며, 문서 검사나 CI의 기존 J suite 통과가 새 Conv 실행을 수용하지 않는다.


<a id="8-rustj-적용-위치--검증-방법--합격반증-조건"></a>
##### 8. RustJ 적용 위치 / 검증 방법 / 합격·반증 조건

**아키텍처 배치.** J Syntax/FunctionEntity → 기존 J Graph IR(구조적 primitive identity 유지) → **후보 DACC/AccessRegion analysis sidecar** → `GraphRewriteCandidate + FactWitness/Guard` → Verified Logical Execution IR(Checks/Effects/Error order) → existing route legality/target/cost/Physical Plan 순이다. 이 배치는 새 mandatory algebra IR 또는 새 executor를 요구하지 않는다. Graph Basis의 `Scan/CellApply/Conv`는 남기고, 검증된 경우에만 ExecutionBasis expansion이나 외부 library path를 선택한다. `with`는 static dtype/shape/semantic boundary facts를 제공할 수 있으나 tile/device/layout은 physical 정책이다.

**최소 실험 프로토콜(기존 GF/IO 작업의 하위 검증 규정).**
1. Elementwise, StaticReindex, ordered Reduce, ordered Scan, Rank/CellApply, Window/Conv, DynamicGather를 각 **source 형태 + operand shape/type + expected J result/error**를 가진 작은 corpus로 고정한다. 처음 여섯 군을 중심으로 하되 Gather를 반드시 실패/unknown 반례에 포함한다.
2. 각 사례에 `ResultDomain/AccessClass/Compute/Combine/Assembly` 계약이 실제로 무엇을 알 수 있는지 `Known/Guarded/Unknown`으로 기록한다. 원본 FunctionEntity, input occurrence, target-independent facts와 witness를 유지한다.
3. **동일한 transfer API**를 두 개 이상의 군(예: Map·Transpose의 region pullback, Reduce·Conv의 contracted-axis/halo demand)에 적용하고, 각 부류의 정확성·unknown fallback을 검증한다. 연산별 예외만 늘어나면 실패로 기록한다.
4. 후보 rewrite마다 **원본 J C oracle ↔ RustJ semantic/reference ↔ 후보(구현된 경우)** 값을 비교하고 dtype/shape/fill·오류 종류/첫 관찰 순서·side effect/late binding을 함께 검사한다. 소스 매칭만으로 equivalence를 선언하지 않는다.
5. 결과 region 수요가 정확하거나 안전한 상위 근사인지 brute-force 작은 Shape와 기호 Shape를 교차 확인한다. 여분을 읽는 것은 비용 문제지만 필수 데이터를 놓치는 것은 correctness failure다. Empty/zero-rank, boxed/sparse, dynamic index, unknown side effects는 반드시 negative fixture를 만든다.
6. 합법적 후보에만 `Work/Depth`, region bytes, intermediate materialization, fanout, target capability를 비교한다. 실제 CPU 실행/측정 근거가 없으면 fusion 성능 이득을 **가설**로 남긴다.

**가설을 강화하는 증거:** (i) 6개 이상 연산군이 같은 구조 필드로 표현됨 **+** (ii) 서로 다른 군 사이에 재사용 가능한 일반 `Access/Rewrite` 규칙 둘 이상이 sound하게 적용됨 **+** (iii) 정확성/오류/빈 Frame/동적 Gather 반례 테스트 통과 **+** (iv) 최소 한 종류의 불필요한 intermediate 또는 I/O 영역 감소를 측정 가능한 실행 경로에서 실증. **기각·수정 조건:** 모두 opaque로 귀착, 적용 규칙이 매번 ad hoc, numeric/error/rank 의미 보존 실패, 더 낮은 basis로 내려가며 Conv/FFT/Scan identity와 합법적 lowering이 사라짐. 어떤 연구 결과도 full-J 완전성이나 “모든 배열 계산의 관계대수” 성립 증명으로 과장하지 않는다.

**기존 수용 원장만 사용한다.** `GF4`(후보 규칙/동치 witness), `GF5a`(Rank·Reduction·Scan·Window의 axis/Work-Depth 사실), `GF6`(실행 가능한 합법적 후보의 physical 이득), `FW-05~09`(FactWitness·rewrite correctness), `IO-09/10`(region demand·byte pruning)의 미완료 항목에 하위 증거를 축적한다. **이 문서화만으로 해당 항목에 새 `[x]`를 주지 않는다.** M2 frontend 우선순위 및 최초 M4 single-CPU vertical slice를 지연시키는 선행 게이트로 격상하지 않는다.

**GF 후속 체크리스트 — M2/M3 및 기존 A1.5 순서를 유지**
- [x] GF0: 선행연구 설명과 실제 GraphBasis 코드의 차이를 감사하고 Scan의 독립 basis 설계 결정을 정정한다.
- [ ] GF1: 업로드 원문을 대조하고 위 복원 내용의 누락/차이를 확인한다.
- [x] GF2: analysis-only CompositionRelation sidecar와 verifier를 추가했다. pipeline/ordinary/capped fork의 wiring·observable dependency, noun-left 미전개 경계, nested rank/window/reduction 및 Copy Rank의 header-only RHS를 회귀 검증한다. noun-left의 실제 h→g graph 전문화는 기존 후속 항목이며 이번 완료에 포함하지 않는다.
- [x] GF3: first-class Scan basis 및 Boolean `+`/`*` insert-prefix의 identity/contract witness와 conservative recognizer를 추가했다. 원본 Window→operand는 보존하며 일반 prefix/infix·unknown reducer·numeric/rank 경계를 C 기본·AVX2와 대조한다. 실행 prefix 지원을 주장하지 않는다.
- [ ] GF3a: 관련 value-property/rank/representation witness를 갖춘 integer/float·일반 reducer·nested rank·sparse 경로로 Scan 인식을 확장한다. unknown을 proof로 취급하지 않는다.
- [x] GF4: 기존 GraphRewriteProvenance와 composition/Scan witness 위에 versioned fusion rule schema·registry·candidate envelope·verifier를 연결했다. 4개 research pattern의 발견만 지원하며 legality/resource/target/profitability/selection은 후속이다. 현재 공통 배열 대수 가설에 대해서는 §공통 DACC 접근 관계·일반 rewrite·반례 검증§을 별도 하위 연구 대상으로 유지한다. 설계 문서만 추가됐으며 GF4 구현 수용 상태는 변하지 않는다. [공통 배열 대수 가설](#common-array-algebra-hypothesis) 참조.
- [x] GF5: 독립 symbolic WorkDepthExpr DAG, ordered successful-path node/region 모델, 별도 ordered Scan identity 모델, fusion source 비교와 단일 연산 duplication 가설을 추가하고 verifier/회귀로 검사했다.
- [ ] GF5a: effective rank/cell/segment 및 일반 window/reducer 모델을 확장하고, 법적 독립성·numeric/error witness를 얻은 경우에만 parallel Depth/max/tree 모델을 추가한다. [공통 배열 대수 가설](#common-array-algebra-hypothesis)의 Reduce/Scan/Conv의 축과 DataRegion 의무, Rank empty-frame prototype 반례를 포함해 검증한다.
- [ ] GF6: 실행 가능한 lowering과 lifetime/resource/cost 비교가 갖춰진 후보만 선택/partition에 연결한다. 공통 대수 기반 후보의 실행 이득 주장에는 동일 입력의 순수 기준 경로 대비 middle-IR 결과·오류 동등성과 Work/Depth·read bytes·materialization 계측을 요구한다. [가설 수용 조건](#common-array-algebra-hypothesis) 참조.
- [x] GF6a: source-only target feasibility와 fusion/WorkDepth witnesses를 연결한 선택 준비 상태 보고서를 추가했다. semantic proof·fused capability·변환 resource/cost가 없는 후보는 미선택이며 full GF6는 미완료다.
- [ ] GF7: multiversion·streaming·inspector-executor는 별도 장기 단계로 진행한다.

**최초 문서 변경의 검증 한계:** `e0204d6`/`aba88ff`는 문서 계약/체크리스트만 수정했고 당시 새 Rust/Python/C 검증을 실행하지 못했다. 아래 GF2 검증은 별도 실행 결과다.

<a id="gf2-composition-review"></a>

**GF2 코드 리뷰 및 구현 — 2026-10-05.** `b00f226`→`aba88ff`의 변경은 PROJECT 두 문서에 한정된다. 새 계약을 `j_graph_ir`, graph memory/resource/rewrite, frontend FunctionEntity와 대조했다. 당장 구현한 범위는 analysis-only composition seam이며 GF3–GF7의 Scan/fusion/WorkDepth/실행 선택을 완료로 표시하지 않는다. FOUNDATIONS의 graph/execution 분리 원칙을 따른다.

- `src/j_graph_composition.rs` 및 `Plan::composition_analysis()`는 verified J graph로부터 관계를 파생한다. Vertical은 producer/consumer ValueId와 Left/Right **입력 occurrence**를 기록한다. HorizontalCandidate는 ordinary fork의 region/input/branch identity를 기록하며 독립성 증명이 아니다. ObservableOrder는 region constructor, 전체 child invocation 완료 순서와 live-across를 보존한다. use-count는 result/write 소비를 포함한 기존 `Plan::use_counts()` 기준이다.
- Nested는 applied owner ValueId와 원래 FunctionEntity의 operand path로 rank/cell·reduction·prefix/window 경계를 참조한다. inner applied ValueId를 만들어 내거나 flattening을 허용하지 않는다. Copy Rank의 RHS 함수는 constructor header 공급원이므로 RHS 본문을 nested computation으로 열거하지 않는다.
- 리뷰에서 noun-left fork를 generic Modifier로 남기면서 ordinary fork의 ParallelBranch/BranchJoin 힌트를 전달하는 오류를 발견했다. 해당 힌트를 제거하고 RetainedNounBoundary로 미전개 noun snapshot operand를 참조한다. capped fork는 Pipeline/CappedFork, ordinary fork는 h→f→g이며 noun-left에는 가짜 f 호출을 만들지 않는다. snapshot 저장은 원래 entity가 소유한다.
- sidecar verifier는 원본 graph verifier를 먼저 호출하고 관계를 재유도해 extra/missing/stale edge·input slot·order·constructor·nested path·fan-out 차이를 거부한다. effect/error가 Unknown인 branch도 ObservableOrder를 유지한다. 이것은 binding guard, legal parallelism proof 또는 physical schedule이 아니다.
- 신규 회귀 7개는 pipeline wiring, unknown-function fork 순서와 fan-out, capped/noun-left 구분, nested rank/window/reduction, repeated input slots, Copy Rank RHS 및 손상된 sidecar/graph 거부를 검사한다. C oracle 근거는 pinned `jsrc/cf.c::jtfolk`의 nvv/vvv/capped 구분과 기존 양 DLL frontend 차분 corpus다. 새 private C trace 동등성을 주장하지 않는다.

**GF2 검증:** native Windows default/portable 각각 **442 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 세 runtime 경로 **5,380 cases / 5,380 passed / failed 0**, stage **10,810 checks / failed 0**, words **6,623 / failed 0**. vocabulary는 145 후보 중 143 POS, 140 bare-function binding/AR, 3 noun payload를 확인했으며 coverage 0/code-only rejected 2다. capture graph 경계 **257건**과 static 경계 **2건**은 별도다. 보고서 12개의 binary/source/DLL hash를 native 검증기로 확인했다. source pin `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`과 DLL release `ded7793fe5795d79eda8e7138dce94aa056edf78`는 구분한다. 실행 optimizer/parallel scheduling/CUDA/전체 upstream 동등성은 이 게이트의 검증 대상이 아니다. Linux/GitHub CI는 실행하지 않았다.

<a id="gf6a-fusion-readiness"></a>

**GF6a downstream fusion readiness — 2026-10-05.** `src/fusion_planning.rs`와 `LoweringRegistry::fusion_readiness(plan,rules,target)`가 GF4 후보와 GF5 모델을 target-dependent inspection으로 연결한다. J Graph의 intrinsic identity/grammar에 target 정보를 넣지 않는다. source 각 basis layer는 기존 LoweringRegistry×TargetCapabilities에 target-only metadata 질의를 수행한다. 이는 relevant CallFacts까지 검증한 실행 가능성 또는 전체 fused kernel의 지원을 뜻하지 않는다.

보고서는 candidate/rule identity, source feasibility, 미해결 proof obligations, DeferredUntilLegality fused-target query, AwaitingSemanticProofs 상태 및 selected=false를 보존한다. 원본 target/registry/fusion/work-depth witnesses와 비교하여 변경되거나 위조된 선택·의무 삭제·cost 개선을 거부한다. Unknown을 legal/illegal로 승격하지 않으며 source Unsupported도 J 언어 오류로 바꾸지 않는다. guard/check-to-use/ownership·semantic/error equivalence, 실제 transformed lowering, lifetime/resource bound, empirical CostEstimate와 selection/partition은 full GF6의 후속이다. 준비 보고서는 실행 route나 fallback/replay 계획이 아니다.

`WorkDepthAnalysis::fusion_envelope_batch()`는 source 검증을 공유하고 expression arena를 한 번만 복제한 뒤 후보당 Work sum/Depth sum/Unknown replacement 3개 식을 추가한다. 모든 후보의 source/retained operation identity와 Unknown replacement를 보존하면서 O(candidate×전체 expression graph) 저장량을 피한다. 단일 envelope inspection API도 유지한다. schema/provenance/source proof는 batch verifier에서 재유도한다.

신규 native Rust 회귀 4개는 source capability가 있어도 fusion을 선택하지 않음, target/registry 변경 무효화, 여러 후보에서 arena 공유와 Scan 실행 미승격, 위조 selected/obligation/profitability 및 unknown-valid J graph의 no-candidate 처리를 검사한다. GPU generic target은 metadata-only unit query이며 GPU 실행/컴파일 검증이 아니다.

**GF6a 검증:** native Windows default/portable 각각 **463 passed / 17 ignored**, fmt/clippy/build 통과; Python **30 passed**. j64/AVX2 각각 세 runtime 경로 **5,380 cases / 5,380 passed / failed 0**, stages **10,810**, words **6,623**, vocabulary POS **143**/binding **140**/noun **3**를 유지했다. Scan **285 cases / 274 identity checks / 11 rejected analysis checks / failed 0**와 runtime prefix 경계 **285 / executable prefix passes 0**은 별도다. capture graph 경계 **257**, static 경계 **2**도 별도로 유지했고 전체 보고서 14개의 binary/source/DLL hash를 확인했다. readiness 검증이며 실제 fusion 선택·성능·GPU 실행을 검증한 것은 아니다. Linux/GitHub CI/CUDA는 실행하지 않았다.

<a id="gf5-work-depth"></a>

**GF5 symbolic Work/Depth — 2026-10-05.** `src/j_graph_work_depth.rs` 및 `Plan::work_depth_analysis()`를 추가했다. schema version 1의 별도 expression DAG는 Constant, provenance-bearing Unknown, logical Atoms/LeadingItems/ItemAtoms, metric별 OperatorCost, Sum/Max/Product/Predecessors/IfEmpty를 갖는다. memory resource 식·바이트·strides·buffer·launch·wall-clock cost와 합치지 않는다. caller의 `OperatorCostModel`은 명시적인 abstract operator weight를 공급하며 Unknown/unknown extent/checked arithmetic overflow는 None을 반환한다. 단위 weight 회귀는 수학적 모델 검사이며 성능 측정값이 아니다.

초기 모델은 **ordered successful-path logical baseline**이다. direct elementwise core primitive는 DispatchChecks + atoms×Element 비용을 보존한다. primitive dyadic Map reducer의 ordered Reduce는 DispatchChecks + (leading items−1)×item atoms×ReducerPair 및 empty identity 조건을 모델링한다. source error check를 삭제하거나 failed trace의 실제 work를 예측하는 모델이 아니다. operator cost는 dtype/numeric retry 등 관련 사실이 부족하면 Unknown으로 공급해야 한다. shape-changing/unknown reducer·opaque/name/definition·일반 rank/window 및 최종 assignment는 Unknown이다. noun lookup/guard의 실제 지연, representation/materialization 및 hardware 비용은 이 domain의 대상이 아니다.

region은 external input boundary에서 역방향으로 source operation을 수집하고 원본 순서로 한 번씩 합산한다. nested pipeline/fork의 child region cost를 다시 더해 같은 operation을 중복 계산하지 않는다. total은 모든 원본 operation을 한 번씩 포함한다. 일반 fork는 h→f→g dependency를 유지하므로 Depth도 합산하며, 단순 syntax 관계로 Max를 생성하지 않는다. Map의 cell 작업을 ordered scalar baseline으로 세는 현재 모델은 최적의 parallel critical path를 주장하지 않는다. rank/cell 내부 경계는 필요한 call facts/model이 없으면 Unknown으로 보존한다.

GF3의 Boolean Scan identity에는 source Window 모델과 별도의 **ordered Scan hypothesis**를 만든다. 그 비용은 DispatchChecks + (n−1)×item atoms×ReducerPair + output atoms×ResultAssembly이며 empty/scalar/singleton 타입/identity 계약을 원래 witness에서 보존한다. 원본 PrefixInfix의 비용/실행은 여전히 Unknown이다. ResultAssembly weight는 abstract assembly 비용이며 물리적 materialized buffer/copy를 강제하지 않는다. parallel scan 허가와 실행 선택은 만들지 않는다.

GF4 envelope 비교는 검증된 source operation/retained value를 참조한다. source Work/Depth를 평가할 수 있어도 transformed replacement는 FusionTransferUnproven/Unknown이며 improvement_proven=false다. 단일 source operation의 extra-call duplication 가설은 기존 입력에서 추가 호출의 비용만 곱한다. 전체 upstream graph 복제 모델 또는 legal duplication proof가 아니며 duplication_authorized=false다. 두 가설도 source analysis/fusion witnesses로 재유도 검증하며 위조 permission/improvement를 거부한다.

신규 Rust 회귀 8개는 symbolic unknown/operator weights, ordered fork, nested region 중복 방지, empty/singleton/scalar/matrix Reduce와 별도 Scan, unknown rank/extent/assignment, extra-call duplication 및 source-vs-unproved fusion, expression cycle/잘못된 provenance·total·arithmetic overflow를 검사한다. 호출 error/effect 순서는 보존했으나 일반 failed-path cost, reassociated reducer, parallel Max/tree, 일반 CellApply/Window 및 실행 optimizer는 후속이다.

**GF5 검증:** native Windows default/portable 각각 **459 passed / 17 ignored**, fmt/clippy/build 통과; Python **30 passed**. j64/AVX2 각각 기존 세 runtime 경로 **5,380 cases / 5,380 passed / failed 0**, stages **10,810**, words **6,623**, vocabulary POS **143**/binding **140**/noun **3**이며 failed 0이다. Scan은 **285 cases / 274 identity checks / 11 rejected analysis checks / failed 0**, runtime prefix 경계 **285 / executable prefix passes 0**을 별도로 유지했다. capture graph 경계 **257**과 static 경계 **2**도 별도다. 전체 보고서 14개의 binary/source/DLL hash를 native 검증기로 확인했다. 이 게이트는 symbolic 모델·source semantics 회귀 검증이며 실제 성능 측정이나 parallel/fused 실행 검증이 아니다. source pin/DLL release는 구분하고 Linux/GitHub CI/CUDA는 실행하지 않았다.

<a id="gf4-fusion-registry"></a>

**GF4 fusion registry와 source envelope — 2026-10-05.** `src/j_graph_fusion.rs`와 `Plan::fusion_analysis(registry)`를 추가했다. stable ID/version 1의 Map→Map, Map→Reduce, Map→Scan, common-input Map+Map을 등록한다. typed pattern 중복/충돌, 잘못된 version/pattern 및 누락된 rank-cell/assembly·numeric·observable effect/error order·fan-out/retention·resource/work-depth·target capability 의무를 거부한다. 임의 pattern 언어의 일반 overlap solver를 구현한 것은 아니다.

후보의 replacement는 **OrderedSourceEnvelope**다. 원본 applied operation subgraph와 입력 occurrence, source 순서, 외부 output을 보존하는 분석용 영역이며 fused kernel이나 새로운 의미론 op가 아니다. 기존 `GraphRewriteProvenance`를 재사용하고, 전체 GF2 composition 및 GF3 Scan witness를 참조한다. E. identity의 equivalence witness를 다른 fusion의 증명으로 재사용하지 않는다. MapScan은 별도 Scan identity witness가 있는 call만 후보가 된다. noun-left/capped fork에는 가짜 horizontal 관계를 만들지 않는다.

각 envelope는 원본 use-count, 내부 입력 occurrence, 외부 소비자 및 retained value를 기록한다. 같은 producer가 dyadic 두 슬롯에 들어가면 candidate는 중복 등록하지 않되 occurrence 2개를 유지한다. 외부 소비자가 있는 producer/output을 지우거나 복제하지 않는다. 관련 input/operation GraphFacts를 기록하고, source span/basis/value, 순서, fan-out, call fact와 witness를 원본에서 재유도하여 검증한다. candidate 내부 use-count는 작은 sparse map으로 계산해 후보마다 전체 graph 크기의 scratch 배열을 만들지 않는다.

모든 후보는 `legality=Unknown`, `resource_transfer_proven=false`, `selected=false`이며 target query는 DeferredUntilLegality다. 아직 없음은 legal transformed replacement, 실제 lowering query, resource/work-depth transfer proof, profitability 및 selection/partition이다. discovery가 cost 감소나 병렬화를 뜻하지 않는다. 현재 immutable original graph를 변경하는 optimizer는 없다. 신규 Rust 회귀 5개는 registry 거부, 세 vertical pattern, horizontal h→f/join/external retention, repeated input 슬롯·shared producer의 외부 소비자, unknown Scan/noun-left/capped 경계 및 위조 selected/span/order/retention을 검사한다.

**GF4 검증:** native Windows default/portable 각각 **451 passed / 17 ignored**, fmt/clippy/build 통과; Python **30 passed**. j64/AVX2 각각 기존 세 runtime 경로 **5,380 cases / 5,380 passed / failed 0**, stage **10,810**, words **6,623**이며 failed 0이다. Scan identity 검사는 각각 **285 cases / 274 identity checks / 11 rejected analysis checks / failed 0**, runtime prefix 경계 **285 / executable prefix passes 0**을 유지했다. 기존 capture graph 경계 **257**, static 경계 **2** 및 vocabulary의 POS/binding/noun 검증은 별도다. 전체 보고서 14개의 binary/source/DLL hash를 native 검증기로 확인했다. 이 게이트는 source graph/발견/검증 경로를 검증하며 fused 실행·reassociation·parallel scheduling을 검증한 것은 아니다. source pin과 DLL release를 구분하며 Linux/GitHub CI/CUDA는 수행하지 않았다.

<a id="gf3-scan-identity"></a>

**GF3 Scan identity 및 보수적 인식 — 2026-10-05.** `src/j_graph_scan.rs` / `Plan::scan_analysis()`는 verified J graph 위에서 독립 `GraphBasisKind::Scan` 후보와 `ExactBooleanAtomicPrefixV1` witness를 생성한다. registry는 8을 유지하고 enum vocabulary 변경으로 Graph IR schema를 **0.9**로 올린다. 원래 PrefixInfix FunctionEntity·span·valence·input ValueId 및 Window→Reduce basis를 변경하지 않는다. candidate output facts는 source facts와 별도이며 실행 lowering에 주입하지 않는다. `execution_basis_for_graph_basis(Scan)`은 None으로 실행 capability를 암시하지 않는다.

- **초기 proof 범위:** 직접 monadic PrefixInfix의 operand가 Insert이고 reducer가 operand-free core Add/Multiply인 경우만 검사한다. input의 exact Boolean dtype, shape/rank consistency와 checked atom extent가 필요하다. Bool sum은 각 lane의 누적값이 leading item 수 이하이고 이 수가 i64 범위 안임을 확인한다. Bool product는 {0,1}에 닫혀 있다. NameRef·일반 verb·다른 reducer·Int/Float/boxed/unknown input은 witness를 얻지 않는다. 값 payload를 읽거나 대용량 배열을 복사해 proof를 만들지 않는다.
- **prefix/assembly:** leading-axis의 inclusive prefix 길이는 1..n이며 artificial identity를 삽입하지 않는다. scalar는 shape [1]로 바뀐다. item 수 0/1 또는 전체 atom 수 0이면 C atomic scan처럼 input atom/type을 유지한다. 따라서 Boolean sum의 singleton/empty 결과는 Bool이고, 일반 n≥2 비어 있지 않은 sum은 Int다. Boolean product는 Bool을 유지한다. dyadic infix와 nested rank의 effective cell/frame/assembly 사실이 부족한 호출은 명시적 analysis boundary다.
- **효과/오류와 수치 계약:** 실제 bare core identity와 Boolean 영역의 closure/overflow bound를 검사하며 spelling이나 arbitrary reducer의 associativity를 추측하지 않는다. scalar/empty/singleton 경로에서 reducer 또는 identity 실행을 새로 삽입하지 않는다. 후보는 source order를 유지하고 `parallel_prefix_authorized=false`다. Float accumulation 순서, integer retry/promotion, 이름/locale 효과, sparse/representation 조건과 일반 rank assembly는 별도 proof 대상으로 남긴다. witness는 검증된 plan의 input facts/provenance에 한정되며 실행 전 binding/metadata guard를 대체하지 않는다.
- **검증/검사 표면:** candidate verifier는 source graph를 검증한 뒤 contract·source/input/span·basis·fact·witness·boundary를 재유도해 stale/missing/forged 값을 거부한다. 새 Rust 회귀 4개는 empty/singleton/scalar/matrix, 일반 prefix/infix/numeric/NameRef/rank 경계, 잘못된 parallel permission·dtype·span, unknown/inconsistent/overflow extent를 검사한다.
- **C 차분 범위:** `tools/scan_contract_conformance.py`와 `examples/scan_contract_probe.rs`를 Windows runner에 연결했다. Boolean 길이 0..6 전체 패턴, scalar와 empty/matrix/3D shape를 포함한 **274 identity/type/shape checks**, 일반 prefix/infix/rank·integer overflow·float cancellation/signed zero/infinity·char/boxed **11 rejected analysis checks**를 분리한다. 독립 정확 Boolean 모델의 데이터와 C 결과도 비교한다. 모든 **285 prefix runtime 경계**와 **executable prefix passes 0**을 별도 기록하며 Unsupported를 실행 동등성 pass로 세지 않는다. rejected numeric/error 사례의 C 결과는 oracle 관찰이며 Rust prefix 실행의 동등성 주장이 아니다.

**근거:** 검토 소스 [ap.c::jtbslash/jtpscan, Bool prefix kernels](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ap.c), [atomic type dispatch](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/va2.c), [insert semantics](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ar.c). C source pin과 실제 DLL release를 구분한다. FOUNDATIONS의 graph/execution 분리·J error/rank 보존 가드레일을 따른다.

**GF3 검증:** native Windows default/portable 각각 **446 passed / 17 ignored**, fmt/clippy/build 통과; Python **30 passed**. j64/AVX2 각각 새 Scan report **285 cases / 274 identity checks / 11 rejected analysis checks / failed 0**이며 **runtime prefix 경계 285 / executable prefix passes 0**을 별도 기록했다. 기존 세 runtime 경로는 각각 **5,380 cases / 5,380 passed / failed 0**, stages **10,810**, words **6,623**, vocabulary POS **143**/binding **140**/noun **3**이며 failed 0이다. 기존 capture graph 경계 **257**, static 경계 **2**는 별도다. 보고서 14개의 binary/source/DLL hash를 native 검증기로 확인했다. source pin `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`과 DLL release `ded7793fe5795d79eda8e7138dce94aa056edf78`를 구분한다. Linux/GitHub CI/CUDA/새 실행 optimizer는 수행하지 않았다.

<a id="a151-과거-jaxa-역대조-감사"></a>
#### A1.5.1 과거 JAXA 역대조 감사

2026-10-01 `yunskim/JAXA`, `yunskim/JAXA-complier`, `yunskim/japchae`, `yunskim/jaxa-analyzer`를 현행 RustJ J Graph IR과 **여러 독립 관점으로 반복 대조**했다. 이번 감사는 (1) language/graph intent, (2) resource/Flow–Storage/static-memory, (3) basis/rewrite/equivalence, (4) frontend prototype, (5) superseded claim 역검토의 다섯 패스로 수행했다.

| 과거 JAXA 개념 | 현행 RustJ 상태 | 판정 |
|---|---|---|
| Semantic AST와 Array Operation IR 분리 | `FunctionEntity`와 `j_graph_ir::Plan`을 별도 boundary로 둠 | 반영 |
| J syntax에서 static graph 직접 유도 | `@:`, Hook/Fork, `/`, `"`를 `GraphForm`으로 분류 | 반영 |
| `@:` pipeline / Hook/Fork branch-join | `Pipeline`, `Hook`, `Fork` + GraphHint | 반영 |
| applied graph stage별 shape 전파 | stage/branch가 explicit `ValueId` node와 독립 `GraphFacts`를 보유하며 layout-independent `SemanticFacts`와 `facts::infer_semantic_projection`을 사용한다 | **초기 구현 — container 분리됨** |
| primitive shape/dtype/rank/effect contract | `GraphRuleRefs` + layout-independent `SemanticFacts` transfer를 J Graph build에서 적용. Execution `Facts` container와 GraphFacts container는 분리 | **초기 구현** |
| primitive symbolic resource contract | `GraphOperationContract`가 iteration/access/fusion/temporary/accumulator/working_state symbolic requirement를 가짐. 구체 resource expression registry는 미완성 | **부분 반영** |
| iteration/reduction/access pattern contract | `IterationContract`/`AccessContract`/`FusionStructure`를 J Graph op에 연결 | **초기 구현** |
| pipeline/reduction/branch/join별 resource composition | Region은 Pipeline/BranchJoin, Apply node는 Reduction/Window/CellMap/Structural composition identity를 직접 기록. `j_graph_resource`가 node/region identity를 노출하나 full symbolic evaluator는 후속 | **부분 반영** |
| intermediate edge materialization/traffic 분석 | `j_graph_memory`가 pipeline/branch/view materialization opportunity와 logical extent를 계산. traffic/selected materialization plan은 후속 | **초기 구현** |
| register/live-value pressure 분석 | J Graph use-def/live-range를 계산하고 branch `live_across`가 join까지 lifetime을 확장. register pressure로의 target mapping은 후속 | **초기 구현** |
| target profile과 graph resource demand 결합 | full TargetProfile/ResourceEstimate는 후속. 다만 Graph rewrite basis를 existing `LoweringRegistry + TargetCapabilities`에 투영하는 target-only feasibility query를 추가해 `Supported / RequiresCallFacts / Unsupported`를 구분한다 | **초기 연결** |
| fusion partition 산출 | fusion candidate/opportunity만 있으며 선택된 fusion partition은 없음. rewrite의 target-only feasibility bridge와 구분 | **미구현** |
| reshape/flatten/transpose를 virtual view로 취급 | Ravel/Reverse/Transpose에 `VirtualIndexingCandidate`를 J Graph에서 기록 | **초기 구현** |
| Flow–Storage | Logical Execution/Planner 쪽에 별도 모델로 보존 | **의도적으로 downstream — 적절** |
| checkpoint/rematerialization/reversible recovery | `StorageRequirement::ExplicitCheckpoint`와 logical/physical 분리는 설계됨. physical `Rematerialize` decision은 이번 반복 감사에서 명시적 planning option으로 보강했으나 구현은 없음 | **부분 반영 — 후속** |
| adjoint/VJP graph + parameter-adjoint fan-out | `ParallelFanOut` schema만 있고 transform 없음 | 연구/후속 |
| Graph basis → rewrite → equivalence algebra | Graph Basis와 첫 witnessed `E.` rewrite registry/candidate/verifier/resource 비교가 있음 | **초기 구현 — 일반 rule/equivalence 확장은 후속** |
| resource-aware rewrite pruning | 없음 | 과거에도 future work; 미구현 |
| basis access-pattern taxonomy | Graph Basis에 Window access family를 추가해 `u\`를 `PrefixInfix`로 보존하고 `(+/)\`를 `Window → Reduce`로 표현. 독립 Scan basis와 초기 Boolean identity 후보를 GF3에서 추가했다. 일반 수치/rank witness와 실행은 후속 | **초기 구현** |
| symbolic resource function/composition | `GraphOperationContract`와 `j_graph_resource`가 최소 합성을 수행한다. `ResourceExprGraph`가 ValueAtoms/Requirement/Sum/Max 식을 보존하고 Pipeline/BranchJoin의 internal/elidable/retained/peak-live provenance를 표현한다. PrefixInfix/Rank는 inner resource requirement를 합성한다. richer accumulator/window-size 함수와 target realization은 후속 | **초기 구현** |
| resource-aware pruning soundness | checklist에는 있으나 local/global resource 구분, monotonicity/soundness proof requirement가 명문화되지 않았음 | **설계 보강 필요** |
| Basis → Rewrite → Equivalence → Optimization 의존 순서 | 각 기능은 roadmap에 있으나 선행관계가 약하게 표현됨 | **설계 보강 필요** |
| static-analyzable subset / validation boundary | `GraphAnalyzability`로 Static / StaticWithUnknownFacts / RequiresSpecialization / DynamicSemanticFallback을 구분 | **초기 구현** |
| jsource-style graph normalization(capped fork→atop, tine simplification) | 현 `j_graph_ir`에는 별도 normalization pass 없음 | **미구현/확인 필요** |
| multi-device static partition | 없음 | future work |

**현재 판정:** v0.3에서 표기에서 얻는 topology를 stage-level applied graph, GraphFacts, use/liveness 및 symbolic resource seam으로 확장했고, v0.9에서 composition/Scan identity 분석을 추가했다. 부족한 것은 graph 부재가 아니라 rewrite-specific facts, 실행 lowering 및 실제 schedule/resource/cost 선택이다.

v0.2에서 explicit stage/branch graph를 도입했고, v0.3에서 Window access family, graph-only fact domain 경계, node-level Reduction/Window/CellMap resource composition을 추가했다.

1. `@:` stage, Hook/Fork branch/join은 이제 실제 J Graph `ValueId` node/edge다.
2. 원래 J combinator identity는 `Region(Pipeline/Hook/Fork)`으로 별도 보존한다.
3. stage별 `GraphFacts`, `GraphOperationContract`, `GraphAnalyzability`, use-count를 graph에서 질의할 수 있다.
4. `j_graph_memory`가 logical extent, graph-order live range, pipeline/branch/view materialization opportunity를 계산한다.

남은 핵심 부족은 **rewrite candidate를 실제 executable lowering/schedule/target resource model로 연결하는 단계**다. Graph 쪽에서는 `ResourceExprGraph`로 internal/elidable traffic, retained/peak-live, temporary/accumulator/window-state requirement와 canonical state lifetime을 표현하고, rewrite candidate도 동일 logical-atom/symbolic-state domain에서 비교한다. 다음 경계는 rewrite-specific facts의 확대, WindowView 등 execution lowering capability, fusion 선택에 따른 lifetime extension, resolved TargetProfile 기반 ResourceEstimate/CostEstimate, 그리고 그 뒤의 candidate selection/partition이다.

<a id="2026-10-01-반복-감사에서-추가로-확정한-jaxa-계승-원칙"></a>
#### 2026-10-01 반복 감사에서 추가로 확정한 JAXA 계승 원칙

1. **Graph Basis의 historical lower bound는 arithmetic atom이 아니라 access pattern 계층이다.** Japchae D-24의 핵심은 너무 작은 scalar `+`/`*`로 분해해 algorithm/access identity를 잃지 말라는 것이다. RustJ의 Graph Basis는 이 원칙을 유지한다. `Conv` 같은 structured op를 Graph Basis에서 black box로 유지하는 결정과 Execution Basis에서 필요 시 분해하는 결정은 독립적이다.
2. **Graph Basis vocabulary에는 windowing 계열이 필요하다.** historical 후보는 `map / reduce / window-reduce / static-reindex / dynamic-gather`이고 `scan`은 독립 패턴인지 열린 질문이었다. v0.3에서 `PrefixInfix`/Window access family와 resource identity를 추가했다. 일반 WindowReduce 실행 지원은 후속이다. ExecutionBasis::WindowView가 존재한다는 사실로 이 요구를 대체하지 않는다.
3. **Basis 연구가 rewrite/equivalence보다 선행한다.** 작업 의존은 `Basis → Rewrite → Equivalence → Optimization`으로 둔다. 완전한 최소 basis 증명까지 기다릴 필요는 없지만, rewrite rule은 어떤 Graph Basis identity를 보존/변환하는지 명시해야 한다.
4. **resource contract는 node별 고정 숫자도, 단순 enum 합도 아니다.** target-independent graph 층은 symbolic requirement/access/liveness/materialization 관계를 합성하고, schedule/target 이후 concrete register/shared/global resource를 계산한다. 현재 최소 `SymbolicResourceExpr`는 seam일 뿐 최종 모델이 아니다.
5. **resource-aware pruning은 매우 후순위다.** basis/rewrite/equivalence가 먼저 서야 하며, pruning은 (a) 해당 resource bound가 부분 graph에서 local하게 결정 가능한지, (b) pruning predicate가 monotone하거나 그 밖의 soundness proof를 갖는지 확인된 경우에만 허용한다. 그렇지 않으면 후보 생성 후 cost/resource evaluation만 수행한다.
6. **static memory claim은 logical determinability로 해석한다.** graph에서 extent/use/lifetime/storage obligation을 정적으로 알 수 있다는 주장은 유지하지만 physical offset/buffer/layout을 J Graph semantic fact로 올리지 않는다.
7. **adjoint/VJP는 basis/rewrite보다 앞서지 않는다.** historical 연구도 복합 graph의 AD는 basis/graph expansion 위에서 자연스럽게 닫히는 문제로 보았다. 현재 `ParallelFanOut` schema는 유지하되 실제 AD transform은 Graph Basis와 rewrite/equivalence surface가 더 성숙한 뒤 진행한다.
8. **frontend 역사 prototype은 current jsource보다 우선하지 않는다.** `JAXA-complier`의 tokenizer/enqueuer/parser Python prototype은 유용한 참고 구현이지만, name lookup timing과 parser behavior의 oracle은 current jsource `w.c/p.c/cf.c`다. 특히 전체 name 품사를 parser 전에 미리 확정하는 모델로 되돌아가지 않는다.
9. **초기 JAXA의 강한 구현 주장은 그대로 계승하지 않는다.** `"RjP`, rank 변화=항상 fusion boundary, 모든 shape op=항상 zero-copy, parse-time complete graph, fixed physical offset, primitive 고정 register 숫자는 후기 연구 또는 current RustJ 계층 분리와 충돌하므로 superseded다.


- [x] `src/j_graph_ir.rs`에 독립 J Graph IR을 추가하고 `Engine::analyze_j_graph()` inspection API를 제공한다.
- [x] `GraphBasis` / `GraphBasisKind`를 Execution basis 타입과 분리하고, derived rank/reduction처럼 outer→inner graph-basis composition을 보존하는 최소 seam을 추가했다.
- [x] Graph Basis에 Window access family를 추가하고 J `\`을 `GraphForm::PrefixInfix`로 보존한다. operand basis를 중첩해 `(+/)\`가 `Window → Reduce`가 되게 했다. 당시 Scan은 열린 질문으로 남겼으며, 2026-10-05 독립 basis 설계를 확정했다. GF3에서 초기 Boolean identity 후보를 추가했으며 일반 수치/rank witness와 실행은 후속이다.
- [x] 기존 `SymbolicResourceExpr` requirement leaf 위에 `ResourceExprGraph`를 추가해 logical `ValueAtoms`, symbolic requirement, `Sum`, `Max` composition을 표현한다. node requirement와 region internal/elidable/retained/peak-live 식 provenance를 보존하며 concrete target 숫자는 넣지 않는다.
- [x] GraphFacts inference에서 execution `Facts`/`LayoutFact` container seed/return adapter를 제거하고 layout-independent `SemanticFacts` domain/API를 사용한다. primitive shape/dtype rule source는 execution inference와 공유한다.
- [ ] Reduction/CellMap/Window resource composition identity를 실제 Apply node와 verifier/resource summary에 연결했고, Rank/PrefixInfix가 inner accumulator/working-state requirement를 보존하도록 합성했다. 남은 일은 이 node composition을 Pipeline/BranchJoin과 같은 symbolic lifetime/traffic evaluator까지 확장하는 것이다.
- [ ] Pipeline/BranchJoin의 edge traffic/retained/peak-live와 child resource state를 `ResourceExprGraph`로 합성했다. 각 temporary/accumulator/window-state에는 canonical graph-order `ResourceStateLiveRange`와 `may_extend_across_fusion`을 기록하고 region별 canonical peak와 conservative all-child-state upper bound를 모두 만든다. 남은 일은 실제 fusion candidate별 lifetime extension/overlap 제약과 target realization 함수까지 연결하는 것이다.
- [x] Graph Basis → rewrite candidate generation → equivalence validation → candidate resource evaluation → sound resource pruning 순서를 `GRAPH_OPTIMIZATION_ORDER`와 rule registry API에 반영했다.
- [x] resource-aware pruning은 `ResourceBoundLocality::Local` + `PruningMonotonicity::ProvenMonotone`가 모두 있는 rule에만 early pruning을 허용하도록 contract를 정의했다. 현재 `E.` rewrite는 global-context-dependent/unproven이라 pruning 불가다.
- [x] `GraphForm`으로 Atomic / Pipeline(`@:`) / Hook / Fork / Reduce(`/`) / PrefixInfix(`\`) / Rank(`"`) / generic Modifier를 구분한다.
- [x] `GraphHint`로 PipelineFusionCandidate / IntermediateMaterializationElision / BranchJoinFusionCandidate / RetainedValueCandidate / ParallelBranchCandidate / ReductionStructure / WindowStructure / CellParallelStructure를 기록한다.
- [x] `GraphRuleRefs`로 shape/dtype/rank-cell/effect rule source와 resource rule의 StructuralComposition/Unknown을 명시한다.
- [x] `Engine::analyze_compilation()`이 `j_graph`와 `logical` 두 IR을 함께 반환한다.
- [x] execution lowering은 BoundProgram을 직접 canonicalize하지 않고 J Graph IR을 소비한다.
- [x] execution node/A3 op가 `j_origin`으로 originating J Graph node를 보존한다.
- [x] Hook/Fork/@: topology 분류의 단일 소스를 `j_graph_ir::classify_function()`으로 두고 execution analyzer의 독립 pattern rediscovery를 제거한다.
- [ ] `\` Prefix/Infix의 graph vocabulary는 추가했다. 남은 Cut/Window(`;.`), Dot/Contract, Power/Iteration, Key/GroupBy 를 GraphForm/GraphHint로 확장한다. 독립 Scan basis의 초기 identity 분석은 GF3에 반영했고 확장은 GF3a에서 추적한다.
- [ ] `ExecutionBasis::WindowView` semantic payload는 `WindowShapeSpec::PatternShape`로 구현했고 expansion verifier가 inputs/payload 일치를 강제한다. `FindViaWindowMatch` 전체에 대해서는 current `E.` rank≤1 semantics와 동일한 CPU `ReferenceRewriteComposite` evaluator/capability를 추가했다. standalone WindowView value/kernel lowering은 아직 없으며 일반 Window rewrite를 위해 후속 구현한다.

- [x] current primitive/rank/reduce 범위에서 stage별 shape/dtype/rank facts를 J Graph build 중 전파한다. richer rule registry는 계속 확장한다.
- [x] `GraphOperationContract`로 iteration/access/fusion 및 temporary/accumulator/working_state symbolic requirement의 최소 seam을 추가했다.
- [x] graph-level use-def/common-input/live-range를 J Graph 및 `j_graph_memory`에서 계산한다.
- [x] logical extent(atom count)와 materialization opportunity를 J Graph에서 정적으로 계산한다.
- [x] `j_graph_resource`에서 Pipeline/BranchJoin의 internal/elidable/retained/peak-live atom volume과 reduction accumulator requirement를 합성하는 최소 evaluator를 구현했다. Reduction/CellMap 단독-region 및 traffic 식은 계속 확장한다.
- [ ] graph rewrite에 대해서는 existing `TargetCapabilities`/`LoweringRegistry`와의 초기 feasibility bridge를 추가했다. representation/schedule/full TargetProfile을 결합한 downstream `ResourceEstimate`는 여전히 후속이다.
- [x] 최소 graph rewrite registry/candidate sidecar를 추가하고 첫 rule로 `E.` Search → Window+CellApply(Match) 후보를 J Dictionary equivalence witness와 함께 생성한다. 일반 rule set 확장은 계속 필요하다.
- [x] graph candidate마다 source ValueId/span/basis provenance, registered semantic-equivalence witness, `RewriteFactRuleId`를 유지한다. verifier는 provenance/rule/witness뿐 아니라 rewrite-local GraphFacts를 rule로 재계산해 stale/invented facts도 거부한다.
- [ ] adjoint/VJP transform을 J Graph IR transform으로 추가하고 fan-out / accumulation topology를 explicit하게 만든다.
- [ ] name-bound derived verb의 graph summary를 binding version + SpecializationKey로 interprocedurally 전파한다.
- [ ] graph rewrite candidate를 source/replacement의 동일 logical-atom + symbolic-state resource domain에서 평가하고 rewrite-local GraphFacts를 보존한다. existing `LoweringRegistry + TargetCapabilities`로 replacement GraphBasis를 execution basis에 투영해 target-only feasibility도 질의한다. unknown cost는 `Incomparable`, call-dependent legality는 `RequiresCallFacts`로 남긴다. `RewritePlanningReport`가 resource + target readiness를 합쳐 `TargetUnsupported / NeedsCallFacts / NeedsResourceFacts / ReadyForCosting`까지만 판정하며 후보 선택은 하지 않는다. 현재 `E.` candidate는 CPU에서 standalone WindowView 없이도 whole-rule `ReferenceRewriteComposite`로 target-feasible하다. 다만 candidate-local Window extent/traffic이 아직 unknown이라 planning state는 `NeedsResourceFacts`이고, GPU/standalone WindowView route는 Unsupported다. 남은 일은 rewrite-specific Window shape/resource 식, 일반 WindowView lowering, full TargetProfile/ResourceEstimate/CostProfile과 연결하는 것이다.

**완료 조건:** 대표 J expressions(`@:`, Hook, Fork, Reduce, Rank, 이후 Window/Contract/Key/Power)가 generic execution DAG를 만들기 전에 J Graph IR에서 구조적으로 식별되고, graph optimizer가 source reparsing이나 execution-DAG pattern recovery 없이 fusion/lifetime/parallel/rewrite 후보를 만들 수 있다.

<a id="fgx-frontend-graph-logical-경계-검증-체크리스트-2026-10-22"></a>
### FGX — Frontend → J Graph IR → Logical IR 경계 검증 체크리스트 (2026-10-22)

**설계 정본:** [§2.1.2 Frontend/J Graph/Logical 경계 감사](./03-architecture-p01.ko.md#212-frontend-j-graph-logical-handoff-검증-계약). 이 목록은 A0.6 Frontend, A1/J Graph, A3 Logical 간 **검증 누락과 단계 선택 상태**만 담당한다. 기존 J Graph feature inventory나 [A3 본 체크리스트](./11-roadmap-checklists-p05.ko.md#a3--logical-execution-ir-core-verification-scheduling-boundary)를 복제하지 않는다.

**코드 감사 기준:** `main` `d1aa45065ce9e5ed50dbc725873f2bf8a1873b07`. 이 버전에서 `analyze_compilation_diagnostic`은 `parse_analysis → semantic::bind → from_bound_with_graph_facts → graph.verify → lower_graph(original.clone()) → A3.verify`를 거친다. `VerifiedFrontend::from_program`의 full handoff 대조는 주 경로에 **별도 호출되지 않으며**, `GraphRewriteCandidate`는 lowering되지 않는다. 확인한 것은 **코드 흐름**이고 수동 테스트 결과가 아니다.

#### FGX-00 — Baseline과 범위 확정 (감사 완료, 실행 검증 대기)

- [x] Frontend(`JEntity`/`FunctionEntity` + context), J Graph(적용된 topology), Logical(실행 operation/check/effect/error) 세 단계의 **서로 다른 소유권**을 확인했다. 세 IR을 하나로 합치지 않는다.
- [x] `j_graph_ir::Plan::verify`가 context 자체, source, parser origin을 검증하지만 `VerifiedFrontend::from_program`의 root·assignment·occurrence 교차 검증과 동일한지 보장할 수 없음을 확인했다.
- [x] Graph rewrite/resource 후보 분석과 원본 graph의 lowering을 분리해 기록했다. rewrite 자동 적용, CFG 지원, 전체 J 최적화 완료로 표기하지 않는다.
- [x] 한국어/영어 README 상세 pipeline 모두 **J Graph IR을 명시함**을 대조했다. README의 Graph 단계 전면 누락이라는 과거 지적은 정정한다.
- [ ] 변경 이후 `main` SHA와 정확한 CI 증거 및 unsupported/ignored 범위를 다시 확인한다. 본 항목의 코드 감사만으로 PASS라고 쓰지 않는다.

#### FGX-01 — P0: Graph/Execution fact drift를 verifier defect로 분류

**수정 대상/구현 기록:** `main` 감사 시점에는 `src/analysis.rs::lower_graph`의 `GraphFacts::agrees_with(...)` 실패가 `Error::Unsupported("J Graph/Execution fact drift ...")`를 반환했다. FGX-01 브랜치에서 `Error::Verification`으로 변경하고 기존 graph value index를 유지했다. 진짜 기능 미지원과 모순된 compiler-owned fact를 혼동하지 않는다.

- [x] **코드 작성:** 오류 반환값을 `Error::Verification`으로 변경. `Error::category() == VerifierDefect`, `kind() == "verifier failure"`의 unit assertion을 추가했다. **실행 검증은 아래 FGX-07로 별도 관리**.
- [x] **회귀 코드 작성:** `analyze_j_graph("1")`의 정상 lowering/verify와, literal node의 dtype fact를 Char로 변조한 뒤 `graph.verify()`가 통과하더라도 `lower_graph`가 verifier defect로 거부하는 경로를 추가. J 실행 오류를 흉내 내지 않는다. **실행 검증 대기**.
- [x] **Admission 회귀 코드 작성:** `Admission::<Plan>::inspected(Stage::Logical, Err(drift))`의 `VerifierDefect`, `may_inspect_another_route()==false`, `is_j_handler_eligible()==false`를 검증하며 `Unsupported`의 `may_inspect_another_route()==true`와 대조. **실행 검증 대기**.
- [x] **정상 회귀 코드 작성:** 동일한 원본 literal graph를 먼저 `lower_graph`하고 Logical verifier를 실행하는 positive control을 추가. 기존 고수준 lowering/regression suite의 default·portable 실제 실행 결과는 CI에서 확인한다.
- [ ] **승인 조건:** verifier 오류와 Unsupported 오류의 범주가 테스트로 구분되고 default/portable + fmt/clippy + 해당 구조·부정 테스트를 통과한다.

**FGX-01 구현 범위 (2026-10-22):** `src/analysis.rs`의 **오류 분류와 테스트만** 변경했다. GraphFacts 추론, graph 생성/검증, 실제 J semantics, handoff admission, Graph rewrite 선택·CFG에는 변경이 없다. 변경 전 `main` 기준 SHA `3d80fffbaaf09c8082b5728cc9f3b5fbe593c77b`. PR 정확한 HEAD/CI 결과는 테스트 완료 후 별도 보고·기록한다. pinned J-C 차분 검증은 내부 verifier 오류 분류를 검사하는 이 slice의 의미론적 필수 조건이 아니며, 사용자 요청대로 수동 테스트는 보류한다.

#### FGX-02 — P1: Compiler 주 경로의 VerifiedFrontend 검증 경계

**문제:** `prepare_semantic_diagnostic`은 `parse_analysis`와 `semantic::bind`를 직접 사용한다. `Graph::verify()`는 parser context를 검사하지만 Semantic Program의 root, assignment metadata, expression/occurrence edge를 `VerifiedFrontend::from_program`만큼 검사한다는 보장은 없다.

- [ ] `VerifiedFrontend::from_program`과 `j_graph_ir::Plan::verify`의 검사 항목을 **항목별 matrix**로 대조하고 주 경로에서 실제로 빠지는 invariant를 명시한다. 겹치는 검사를 "누락"으로 중복 계산하지 않는다.
- [ ] 주 컴파일 경로에 검증된 handoff를 전달하거나 **동등성까지 입증된** 별도 선행 검사를 배치한다. 선택한 API 경계·지원 범위·실패 category를 기록한다. `from_capture`와 context 없는 합법적 batch graph 생성자에 무조건 `VerifiedFrontend` 타입을 강제하지 않는다.
- [ ] **정상 회귀:** 리터럴, monad/dyad, hook/fork/rank, direct assignment, 허용된 noun assignment의 source/root/parser provenance가 frontend → J Graph → Logical에서 유지되는지 확인.
- [ ] **부정 회귀:** Program root, assignment span/source/copula, semantic occurrence edge를 각각 조작했을 때 검증이 거부하는지 확인. 지원되지 않는 동적 NAME/effect는 `Unsupported`로 보류하고 verifier 결함으로 오분류하지 않는다.
- [ ] **승인 조건:** 주 경로의 root/assignment/occurrence handoff 동등성 또는 명시적 좁은 적용 범위가 검증되고, 독립 graph 생성의 합법적 subset을 깨지 않는다.

#### FGX-03 — P1: `from_bound`와 `from_capture`의 제한된 동등성 검사

- [x] 정적 `from_bound`는 `j_graph_ir::Plan`, 실행 관측 기반 `from_capture`는 `CapturedGraph`를 생성한다. 반환 타입과 observation 부가정보가 달라 **전체 객체 동일성은 올바른 계약이 아님**을 기록했다.
- [ ] 실행 전후 효과가 없는 정적 J 표현의 공통 수용 교집합을 먼저 선정한다(예: literals, elementary dyadic expression, 구조가 양 경로에서 실제 승인된 hook/fork). 지원 여부가 불확실한 표현은 검사 범위로 확정하지 않는다.
- [ ] Graph의 공통 node/region, 적용 구조, producer-consumer dependency, source span 및 `j_origin` 대응을 비교한다. 실행 관측 값·constructor witness·name guard는 공통 구조 비교와 분리한다.
- [ ] **부정 회귀:** 실패 capture, 동적 name/locative, effect가 있는 문장을 자의적으로 정적 동등성 증거로 승격하지 않는지 검증한다.
- [ ] **승인 조건:** 선택된 순수 subset에서 graph 의미·구조가 일치하고, 서로 다른 관측 메타데이터를 부당하게 동일시하지 않는다.

#### FGX-04 — P2: 문서/컴포넌트 명칭 정합성

- [x] 현행 이름 `J Graph IR / Graph Analyzer`와 과거 **JAXA 연구명**의 구분을 아키텍처 정본에 반영했다. 그래프 단계의 존재 여부에 관한 README 오판을 정정했다.
- [ ] `docs/project`의 나머지 현행 stage 표제·설명과 축약 다이어그램을 검토하고, 역사적 JAXA 언급은 문맥을 남기되 현행 컴파일러 컴포넌트로 오해될 표현만 정리한다. 문서 anchor는 보존한다.
- [ ] 모듈 영어 미러 `03-architecture-p01.md`, `11-roadmap-checklists-p04.md`는 현재 **존재하지 않는다**. 프로젝트 문서 이관/미러 계획에 따라 전체 문서의 정합성을 보장할 방법을 정하고 후속 동기화한다. 불완전한 부분 파일을 대칭 미러인 것처럼 만들지 않는다.
- [ ] **승인 조건:** 한국어 정본·인덱스 링크·영어 미러 상태가 일관되고, README 상세 pipeline이 현재 Graph IR을 올바르게 기술한다.

#### FGX-05 — 이후: Selected J Graph → lowering / SPORES·E-Graph 연결

- [x] 현재 `analyze_compilation_diagnostic`의 rewrite 발견/자원 평가는 **analysis-only**, lowering 입력은 **원본 `j_graph.clone()`**임을 확인했다. 이는 의도된 비선택 상태이지 lowering이 rewrite를 잘못 적용하는 버그가 아니다.
- [ ] `source candidate → equivalence witness / Exact-J legality → guard/effect/error 조건 → target/resource/cost → selected graph → lowering`을 **서로 다른 상태**로 정의한다. `ReadyForCosting`이나 target feasibility만으로 선택·실행을 허가하지 않는다.
- [ ] Graph 후보를 실제 lowering 대상으로 전달할 때 proof provenance, error-first ordering, NAME/binding-effect witness, shape/rank/fill/numeric semantics와 실패 fallback을 검증한다. 미승인 후보는 원본 graph를 유지한다.
- [ ] SPORES/egg 기반 동등성 포화는 이 계약 위에 선택적으로 구성하며, 유사 수치 결과나 구현 용이성을 Exact-J 증명으로 간주하지 않는다.
- [ ] **승인 조건:** 정당한 후보 하나의 end-to-end 선택/재검증/Logical IR 생성과, 오류·효과 반례에 대한 선택 거부가 각각 테스트된다. candidate discovery만으로 완료 표시하지 않는다.

#### FGX-06 — 이후: A3-v0 single-block에서 explicit-definition CFG로 확장

- [x] `logical_ir::Plan::verify()`는 A3-v0에 function/region/block 각 1개와 Return 종결을 요구한다. 이는 **의도된 straight-line 실행 IR 범위**다.
- [ ] CFG 확장 전 function/region/block/terminator/block argument, SSA dominance, name/effect frontier, J first-error ordering, source provenance의 새 verifier 계약을 수립한다.
- [ ] explicit-definition interpreter의 제어 흐름 실행과 compiled CFG lowering 지원은 별도 판정한다. 미지원 body graph를 임의의 단일 block에 평탄화하지 않는다.
- [ ] **승인 조건:** 대표 if/while/for/try 중 선택한 작은 기능의 CFG가 올바르게 구성·검증되고, 잘못된 branch/SSA/effect 순서가 거부된다.

#### FGX-07 — 실행 증거/판정표 (이번 문서 작업 시점)

| 검사·증거 | 현재 상태 | 완료 기록 시 요구 사항 |
|---|---|---|
| 코드 구조 및 책임 경계 감사 | **확인(정적 감사)** | 감사 기준 `main` SHA `d1aa4506`를 유지 |
| 이번 변경의 fmt/clippy, 문서 검사 | **UNRUN** | 정확한 PR HEAD 자동 CI 또는 수동 결과·링크 |
| Rust default/portable 및 Python harness | **UNRUN** | 정확한 HEAD와 실행된 테스트명·ignored 별도 명시 |
| Frontend handoff 부정 verifier 테스트 | **UNRUN** | 변조 root/assignment/occurrence별 실패 category |
| Graph fact drift 분류 테스트 | **회귀 코드 작성, 실행 증거 대기** | `VerifierDefect`, no retry; Unsupported 대비; PR HEAD CI 정확히 기록 |
| 정적/관측 Graph 교집합 비교 | **UNRUN** | 공통 승인 subset/불일치 내역 기록 |
| J-C pinned j64/j64avx2 differential | **UNRUN(본 변경은 문서 전용)** | 향후 의미론 변경 시 해당 fixture 실제 관측 |
| Graph 선택 최적화 / compiled CFG | **BLOCKED(미구현)** | 개별 FGX-05/06 gate 충족 |

**진행 규칙:** P0(fact drift 오류 분류) → P1(handoff 완전성/두 graph 생성 경로의 의미 계약) → P2(문서 명칭) 순서로 작은 PR과 테스트 하나씩 처리한다. Graph rewrite 선택과 CFG는 **이후 독립 게이트**이며 P0/P1을 위해 선행 구현하지 않는다. 사용자 요청대로 이번 문서화 과정에서 수동 테스트·CI 재실행을 하지 않았으며, 기존 기록의 PASS를 새 코드의 PASS로 복사하지 않는다. 현행 [NAME 간접 함수 쓰기 draft PR #109](https://github.com/yunskim/RustJ/pull/109)는 별도의 J-C 검증 게이트에 의해 계속 보류되며 이 문서가 병합 승인을 제공하지 않는다.

