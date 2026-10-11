[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)

> 문서 분할 안내: 기존 `PROJECT.ko.md`의 내용을 이동한 모듈입니다. 원문의 단일 파일 정본·신규 Markdown 금지 문구는 최신 [Agent 문서 규약](../../../AGENTS.md)보다 우선하지 않습니다.

# 10. 구현 계획과 체크리스트 — 계속 (4/5)

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

#### 진행 규칙

- F0–F2 + P0–P7을 frontend/parser migration의 **single source of truth**로 사용한다. P8은 기존 A1/A2/A3 체크리스트와 함께 추적한다.
- 이후 frontend 작업의 진행상황 보고는 반드시 이 체크리스트의 phase/item 기준으로 보고한다. 새 작업이 생기면 임시 TODO를 코드에만 남기지 않고 먼저 해당 phase에 checklist item으로 추가한다.
- jsource와의 차이를 발견하면 "RustJ 구현 편의"로 봉합하지 않고 해당 phase의 compatibility defect로 기록한다. observable semantics가 같다는 differential proof가 있기 전에는 의도적 차이로 간주하지 않는다.
- frontend 구현은 원칙적으로 F0 → F1 → F2 → P1 → P2 → P3 → P4 → P5 → P6 → P7 순으로 진행하되, 앞 phase interface를 깨지 않는 oracle/test 작업은 병행할 수 있다.
- 완료 즉시 같은 변경에서 `[ ] -> [x]`로 갱신한다.
- 부분 구현을 완료로 표시하지 않는다. 각 phase의 완료 조건을 만족해야 phase 완료로 본다.
- jsource와 의도적으로 다른 observable parser behavior가 필요해지면 구현 전에 rationale과 semantic impact를 이 문서에 기록한다.
- downstream IR 요구사항은 P8/A1/A2/A3에 추가하고 parser entity에 임시 compiler field로 밀어 넣지 않는다.


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

### A1.5 — J Graph IR / JAXA Array Operation Graph IR

**목표:** JAXA의 핵심 연구 표면을 first-class compiler IR로 만든다. parser가 만든 immutable FunctionEntity를 actual noun application과 결합하여, J 문법 자체가 제공하는 graph topology와 optimization hint를 잃지 않는 applied operation graph를 만든다.

> **현재 위상:** `j_graph_ir` v0.10은 **explicit applied-operation graph + access-pattern basis + witnessed rewrite/resource analysis** 단계다. `@:`/Hook/Fork 내부 stage/branch가 실제 `ValueId` node로 전개되고, `/`, `"`, `\`의 Reduce/CellApply/Window 구조가 graph-level basis/resource identity로 보존된다. stage별 GraphFacts/use-count/analyzability와 `ResourceExprGraph`, witnessed rewrite candidate, conservative source-vs-replacement resource evaluation, existing `LoweringRegistry + TargetCapabilities`에 대한 target-only feasibility bridge가 존재한다. 아직 없는 것은 full rewrite-specific shape algebra, executable WindowView lowering, fusion-candidate별 lifetime extension, resolved TargetProfile/ResourceEstimate/CostProfile, 실제 candidate selection/partition이다.

<a id="graph-prior-art-followup"></a>

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

**GF 후속 체크리스트 — M2/M3 및 기존 A1.5 순서를 유지**
- [x] GF0: 선행연구 설명과 실제 GraphBasis 코드의 차이를 감사하고 Scan의 독립 basis 설계 결정을 정정한다.
- [ ] GF1: 업로드 원문을 대조하고 위 복원 내용의 누락/차이를 확인한다.
- [x] GF2: analysis-only CompositionRelation sidecar와 verifier를 추가했다. pipeline/ordinary/capped fork의 wiring·observable dependency, noun-left 미전개 경계, nested rank/window/reduction 및 Copy Rank의 header-only RHS를 회귀 검증한다. noun-left의 실제 h→g graph 전문화는 기존 후속 항목이며 이번 완료에 포함하지 않는다.
- [x] GF3: first-class Scan basis 및 Boolean `+`/`*` insert-prefix의 identity/contract witness와 conservative recognizer를 추가했다. 원본 Window→operand는 보존하며 일반 prefix/infix·unknown reducer·numeric/rank 경계를 C 기본·AVX2와 대조한다. 실행 prefix 지원을 주장하지 않는다.
- [ ] GF3a: 관련 value-property/rank/representation witness를 갖춘 integer/float·일반 reducer·nested rank·sparse 경로로 Scan 인식을 확장한다. unknown을 proof로 취급하지 않는다.
- [x] GF4: 기존 GraphRewriteProvenance와 composition/Scan witness 위에 versioned fusion rule schema·registry·candidate envelope·verifier를 연결했다. 4개 research pattern의 발견만 지원하며 legality/resource/target/profitability/selection은 후속이다.
- [x] GF5: 독립 symbolic WorkDepthExpr DAG, ordered successful-path node/region 모델, 별도 ordered Scan identity 모델, fusion source 비교와 단일 연산 duplication 가설을 추가하고 verifier/회귀로 검사했다.
- [ ] GF5a: effective rank/cell/segment 및 일반 window/reducer 모델을 확장하고, 법적 독립성·numeric/error witness를 얻은 경우에만 parallel Depth/max/tree 모델을 추가한다.
- [ ] GF6: 실행 가능한 lowering과 lifetime/resource/cost 비교가 갖춰진 후보만 선택/partition에 연결한다.
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


---

[← 이전 부분](./11-roadmap-checklists-p03.ko.md) | [다음 부분 →](./11-roadmap-checklists-p05.ko.md)
