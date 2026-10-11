[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)

> 문서 분할 안내: 기존 `PROJECT.ko.md`의 내용을 이동한 모듈입니다. 원문의 단일 파일 정본·신규 Markdown 금지 문구는 최신 [Agent 문서 규약](../../../AGENTS.md)보다 우선하지 않습니다.

# 3. J Semantic Array IR — 계속 (3/4)

#### 3.7.1 Explicit-definition control-flow handoff example

control-flow handoff의 orientation 표본으로 다음 monadic explicit definition을 사용한다.

```j
f =: 3 : 'if. y do. 1 else. 0 end.'
```

이 예제의 목적은 `if.` 자체를 지금 compile한다고 주장하는 것이 아니라, **현재 frontend가 소유하는 정보와 future A3 CFG가 소유해야 할 정보를 분리하는 것**이다.

현재 경계:

~~~text
source
  f =: 3 : 'if. y do. 1 else. 0 end.'
      │
      ▼
[현재 구현] DefinitionInput / DefinitionCode
  source/body
  definition mode + monad/dyad body ranges
  sentence/source provenance
  monad_controls:
    If / Do / Else / End ...
  previous-result/control-flow metadata
      │
      ▼
[현재 구현] FunctionEntity
  head = ExplicitDefinition(DefinitionCode)
  POS / source / construction identity
      │
      ▼
[현재 runtime subset] per-call LocalFrame / straight-line invocation
  supported mode 1/2
  fresh LocalFrame per call
  x/y/u/v/m/n binding
  local-first → global lookup
  local/global assignment + cleanup
  control node가 Body 이외이면 Unsupported
      │
      ───────── compiler body-graph / CFG lowering stop line ─────────
      │
      ▼
[planned] definition-body semantic graph / CFG construction
  entry: evaluate condition sentence under J semantics
          ├─ true  → then body
          └─ false → else body
  merge: preserve J previous-result / return semantics
  local/name reads and writes remain explicit effects/resources where needed
      │
      ▼
[planned A3 control-flow lowering]
  Region
    Block(entry)
      ... condition ops ...
      CondBranch(...)      // future terminator, not current A3-v0 API
    Block(then)
      ... noun/result ...
      Branch(merge)
    Block(else)
      ... noun/result ...
      Branch(merge)
    Block(merge)
      block argument / phi-like selected result
      Return(result)
~~~

현재 `logical_ir::Plan`에 `Function → Region → Block` container는 이미 있지만 **A3-v0의 `Terminator`는 현재 `Return`만 가진다.** 따라서 Region/Block 타입이 존재한다는 사실을 `if./while./try.` CFG lowering이 구현되었다는 뜻으로 해석하지 않는다. `Branch`, `CondBranch`, block argument/phi-like merge는 위 그림에서 **planned concept**이다.

J Graph IR과 CFG도 같은 것으로 취급하지 않는다.

- 각 basic block 안의 analyzable array expression은 J semantic construction에서 **J Graph IR applied computation**으로 보존·분석할 수 있다.
- `if./while./try.`의 control edge와 invocation frame/namespace/effect semantics는 **definition control-flow / A3 Region-Block** 책임이다.
- 따라서 J Graph IR을 억지로 generic CFG로 확장하거나, 반대로 A3 CFG가 J combinator provenance를 지워서는 안 된다.

future CFG lowering이 추가될 때 필요한 최소 증명:

1. `DefinitionCode.monad_controls/dyad_controls`와 source/control ranges에서 branch/loop edge를 결정적으로 재구성한다.
2. 현재 runtime `LocalFrame`의 per-call 독립성, x/y/u/v/m/n binding, local-first lookup 의미를 compiler CFG에서도 보존하고, 향후 locale/path 지원이 추가될 때 같은 resource/effect contract로 확장한다.
3. branch merge에서 J의 previous-result/explicit `return.` 의미를 block-result/terminator로 정확히 표현한다.
4. local/public assignment와 error/throw/catch edge를 ordinary SSA value flow와 분리한다.
5. recursion은 Function body를 공유하더라도 invocation frame/state는 공유하지 않는다.
6. verifier가 illegal branch target, missing merge/result, effect-order break, source-control provenance drift를 거부한다.

현재 회귀와 runtime tests는 `DefinitionCode` construction/control metadata뿐 아니라 **지원 straight-line invocation의 LocalFrame·scope·assignment/effect 동작도 일부 검사한다.** 그러나 control-flow body를 J Graph/A3 CFG로 낮추는 planned compiler 경계의 완료 증거는 아니다.

#### 3.7.2 jsource NAME 정합성 감사 및 F1/P4 수용 조건 (2026-10-07; 설계·코드 조사)

**명세 우선순위:** 고정 `jsource@13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`의 [`sn.c::vnm/vlocnm/nfs`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sn.c), [`w.c::jtenqueue`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/w.c), [`p.c::jtparsea`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c), [`s.c::jtsyrd/jtsymbis`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/s.c), [`sl.c`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sl.c) 및 [J Dictionary의 NAME/locative/assignment 관찰 의미](https://jsoftware.com/help/dictionary/dicti.htm)를 기준으로 한다. RustJ 고유 NAME **문법·의미 제한을 추가하지 않는다.** 허용되는 차이는 내부 carrier·hash layout·lookaside·refcount·JIT cache 구현 등 외부에서 보이지 않는 방식뿐이다. 아래는 **현재 main의 정적 감사**이며 실제 differential PASS 기록이 아니다.

**NJ-03 오류 전달 경계의 제한된 구현:** `RuntimeParserHost::lookup`은 `Result<Option<ParserNameBinding>>`으로 조회 실패와 미정의 이름을 구별한다. parser는 실패의 오류 종류를 유지하고 해당 NAME span과 원래 enqueue 단어 번호를 추가하며 이후 조회·version 관측·실행을 중단한다. 오류 전에 완료된 우측 호출의 효과와 capture는 보존하며 replay하지 않는다. 성공적인 read capture를 합성하지 않는다. 일반 NAME의 flat namespace 조회는 유지하며 base noun locative는 별도 scope 선택과 오류 전달을 사용한다. 일반 locale/path/holder 해석과 완전한 resolution witness는 여전히 미구현이다. NJ-03 전체·NJ-04 및 L3 완료를 뜻하지 않는다.

NAME 감사 도구는 `--reference-root .reference`로 Linux/WSL의 `build_reference.py` 산출물을 사용할 수 있다. 빌드 manifest의 고정 commit·platform·variant와 실제 library 존재를 검사하고 binary SHA256을 기록한다. 없는 variant는 미실행으로 남으며, unsupported/mismatch를 통과로 계산하지 않는다. 기존 Windows `--assets-root` 경로도 유지한다.

일반 locative는 이제 Simple/DirectLocative/IndirectLocative/BaseLocative 어휘 메타데이터를 가진 NAME으로 enqueue된다. env=2에서 locative NAME 대입 대상의 지역 copula는 전역으로 승격되며 env=0은 특수화하지 않는다. 원본 span·queue index를 보존한다. 정적 parser, gerund lookup, catalog 선언, 반복 변수는 일반 namespace 접근 전에 locative 실행을 계속 거부한다. 런타임의 base 및 named-own noun만 아래 제한된 경계로 허용한다. computed locative와 아래 경계 밖 abandon locative는 Unsupported다. #14에서 현재 코드로 이식한 NJ-01 분류 단위이며 NJ-04 locale/path 해석이나 L3 완료가 아니다. 회귀 검사는 세 환경·두 copula, malformed/limit 우선순위와 비변경 경계를 포함한다. 고정 소스/public C stage 비교는 어휘 문법과 값 probe를 검사하며 C 내부 queue flag는 소스 기반 golden이지 exported runtime trace가 아니다.

**고정 C locale 오류 전달:** 오라클은 jsource EVLOCALE(30)을 `locale error`로 전달하여 미정의 이름의 `value error`, 잘못된 holder 형태의 `rank error`와 구분한다. 잘못된 숫자 holder와 생성되지 않은 번호 locale을 고정 C 라이브러리로 검증하며, 알 수 없는 오류 번호의 기존 fallback은 유지한다. 검증 하네스 수정이며 indirect/numbered locale 구현이나 NJ-04/L3 완료를 뜻하지 않는다.

**Indirect 명사 검증 선행 행렬:** 자동 NAME 차분에 boxed holder 재대입, named own-miss z 조회와 own 쓰기, snapshot, 실패 RHS 후 값 보존, holder 없음/숫자/비박스 오류 및 오른쪽 오류 우선순위를 추가한다. Rust 미지원 결과는 unsupported gap으로 남기며 locale/value 오류 혼동은 의미론 불일치로 분류한다. C 관찰은 내부 holder/version witness의 증거가 아니며 indirect runtime 구현과 full NJ-04/L3는 미완료다.

**전체 primitive 이름 catalog:** 고정 jsource ws.c의 공개 spellin 표기 139개와 상수 동사 계열의 _1:…_9:·__:를 포함한 149개 표기에 PrimitiveName/PrimitiveNames를 부여한다. CPLUS 같은 기호별 코드 이름과 t.c의 monad/dyad handler 이름을 별도 metadata로 기록한다. 같은 기호의 valence 이름은 구분하며 atomic 공용 handler 이름은 의미 계약으로 추론하지 않는다. 기존 core identity의 names()로 실행/recognition-only 구분 없이 접근한다. noun·copula·괄호 및 미설치 spellin 항목도 이름을 기록하지만 함수 수용을 넓히지 않는다. 숫자 C opcode·ds 객체·kernel 재사용을 복제하지 않으며 registry/IR identity·rank·effect·실행 지원은 유지한다. pinned source 표 전체 대조와 core/Vocabulary exhaustive 회귀로 검증한다. 실행용 PrimitiveId·AdverbId·ConjunctionId와 frontend, J Graph/Logical IR, 분석, runtime, 예제, 테스트 및 문서의 primitive 참조도 동일한 기호 이름을 사용한다(Plus, Dollar, Slash, Qq, Atco 등). 의미별 별칭 Add/Shape/Insert/Rank/Atop은 primitive 식별자에서 제거한다. valence별 함수 이름과 논리 연산·커널 이름은 각 계층의 의미를 유지한다. 공개 Rust enum/API 및 Debug 이름은 변경되지만 표기, enum 순서, 의미론과 registry version 10은 유지한다. 모든 실행 identity 이름과 카탈로그 이름의 일치 및 lexer→semantic→Logical IR 전파를 회귀 검증한다.

**현재 rank 63 표현:** 현재 PrimitiveId::innate_ranks와 rank 정규화는 jsource 호환 정수 표현 63을 유지한다. 고정 jtype.h의 RANKT는 8비트이며 플래그 2비트를 제외한 6비트의 최대값 RMAX = 63이다. 이는 실제 배열 rank의 허용 최대값이기도 하므로 실제 rank 63은 63차원을 뜻한다. 동사의 innate rank 63은 모든 허용 인수를 전체 cell로 받는 사실상 무한 rank 역할을 하며, 적용 cell rank는 인수 rank를 넘지 않는다(인수 rank 3이면 cell rank 3). 배열 rank와 동사 rank의 역할을 혼동하지 않는다. 명시적 Infinite 표현은 장기 IR 설계이며 이번 변경에서 도입하지 않는다. [jsource 정의](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/jtype.h#L68-L79)

**Indirect holder 오류 조회 경계:** 단일 일반 holder 이름의 indirect 읽기는 parser-stack 조회 시 local → base → z 명사 holder를 확인한다. 미정의 holder는 value error, 비원자 holder는 rank error, 현재 생성 불가능한 비음수 정수 번호 locale는 locale error를 반환한다. 오른쪽 완료 효과와 최초 오류 순서를 보존하며 실패 조회에 flat binding witness를 만들지 않는다. scalar 비박스 타입은 domain error이며 비음수 scalar Boolean/boxed 정수·Boolean 번호 locale도 locale error로 구분한다. scalar box 내부를 rank → 비어 있음 → literal 타입 → locale 이름 순서로 검증하여 각각 rank/length/domain/ill-formed name 오류를 보존한다. 오류 조회는 오른쪽 완료 효과를 유지하고 왼쪽 효과를 실행하지 않으며 성공 binding witness를 만들지 않는다. scalar 정수 또는 boxed scalar 정수의 음수 debug 주소도 정지된 debugger call frame이 없는 현재 runtime에서는 locale error를 반환한다. 실행 중인 일반 local frame을 debug frame으로 대체하지 않으며 실패 후 frame 복원·전역 holder 보존을 검증한다. 실제 debugger/suspension·복합 holder·함수·static/compiler는 미지원 유지; 아래 제한된 명사 대입·abandon은 별도 경계다. 아래의 제한된 정상 boxed holder 읽기 외 일반 indirect witness와 full NJ-04/L3는 미완료다.

**Boxed holder base/named 명사 읽기 경계:** 단일 일반 holder가 scalar box 안의 rank 0/1 문자 named locale를 제공하면 해당 locale own table과 기본 z 경로에서 명사를 읽는다(번호 locale 제외). boxed base holder는 Engine base table을 공유하며 own hit는 Global(engine), z hit는 Locale(z)로 구별한다. 대상 조회는 호출 local을 우회하지만 holder 선택은 local 우선이며 named↔base 재대입·snapshot과 wrong Global/Locale witness 음성 검사를 포함한다. local → base → z holder 선택, 조회 시 holder 재대입 반영과 배열 snapshot을 검증한다. capture는 시작/발견 locale, holder version/generation 및 읽은 명사의 version/generation을 분리하며 구조 verifier의 음성 검사를 포함한다. 이 관찰은 indirect SimpleNameGuard/LocativeNounGuard 또는 compiler/hoisting/replay proof를 허용하지 않는다. 함수 호출·복합 holder·일반 locale path는 후속 계약이다. 단일 boxed holder 명사 abandon은 아래 별도 경계에서 지원하며 full NJ-04/L3는 미완료다. indirect noun 읽기와 abandon의 capture 검증은 존재하는 holder binding을 가리킬 수 없는 버전 또는 generation 0을 거부한다.

**단일 boxed holder 명사 abandon 경계:** `name__holder_:`는 stack 조회 시 local → base → z holder를 선택하고 boxed base/named 시작 locale의 own → 기본 z 경로에서 찾은 명사를 반환한 뒤 발견 table에서만 삭제한다. locative 대상은 explicit local table을 우회한다. 새 named locale 준비는 삭제 전 관찰보다 먼저 수행하여, 처음 z hit를 삭제해도 시작/발견 scope·holder와 noun version/generation을 캡처에 보존한다. holder 자체 삭제도 삭제 전 증거를 유지한다. 고정 p.c의 undefname 분기에 따라 abandon의 holder 조회 실패(rank/domain/length/잘못된 이름/부재 numeric locale)는 value error로 보고하며, 일반 읽기·대입의 상세 오류는 유지한다. 실패는 base/named/z binding과 버전을 삭제하거나 변경하지 않는다. RHS 최초 오류는 삭제를 방지하지만 abandon 완료 후 왼쪽 오류는 삭제를 되돌리지 않는다. 배열 snapshot, own 삭제 후 z fallback, z 삭제 후 ordinary value error, local holder 우선과 event/NameUse 변조 음성 검증을 고정 C 및 두 실행 경로로 확인한다. indirect guard·J Graph/compiler 수용은 계속 거부하며 함수·future reference·복합/번호/debug holder·일반 path/full L3는 미완료다.

**명사 인수 선택 동사 `[`, `]` runtime 경계:** `t.c`의 CLEFT/CRIGHT와 `v.c::jtright1/jtleft2/jtright2`에 따라 두 동사의 monad는 y, dyad `[`는 x, dyad `]`는 y를 그대로 반환한다. innate rank는 모두 RMAX(63)이며 operand는 공유 parser가 오른쪽부터 모두 평가한 뒤 선택한다. 무시되는 operand의 오류·효과를 생략하지 않는다. 수치 변환/shape agreement를 적용하지 않고 선택된 명사의 dtype·shape·boxed/sparse/extended 값과 snapshot을 보존한다. 기존 VocabularyPrimitive recognition에서 PrimitiveId::Left/Right로 실행 identity를 이관하고 registry version을 갱신한다. runtime noun 실행을 검증했으며 compiler alias/evaluation proof 계약은 unknown으로 유지한다. 함수 이름 이관·최적화 수용·일반 rank prototype 및 full L3 완료를 뜻하지 않는다. 고정 C 차분과 default/portable·semantic-reference 회귀는 noun 종류·RHS holder 변경·양 operand 오류 순서를 검증한다.

**단일 boxed holder 명사 대입 경계:** RHS 평가 완료 후 holder를 local → base → z에서 선택하고 boxed base/named locale의 own table에 명사를 기록한다. z에서 읽은 버전은 대입 previous version이 아니다. `=.`/`=:` 모두 explicit local target을 우회한다. RHS 실패는 target locale/binding을 만들거나 바꾸지 않으며, RHS holder 재대입은 완료 시점 대상에 반영한다. commit이 holder 자체를 바꾸어도 캡처 버전은 commit 직전 대상 이름을 고정하여 기록한다. 읽기와 대입은 동일 holder 오류 검증을 공유한다. pinned C 차분은 named/base/z 보존, local bypass, RHS 오류 우선순위와 자기 holder 재대입을 검증한다. 이 bounded runtime/capture 구현은 indirect compiler/guard/Graph 증명이 아니며 함수·복합 holder·computed·일반 path와 full NJ-04/L3는 미완료다. 단일 boxed holder 명사 abandon은 위 별도 경계에서 지원한다.

**NJ-04 base 명사 경계:** 런타임 `a__` 조회·명사 대입은 Engine의 base namespace를 명시적으로 선택하고 호출 지역 table을 우회한다. `a__`라는 별도 flat key를 만들지 않으며 기존 base `a`의 generation/version과 동일한 저장소를 사용한다. noun은 parser stack 진입 시 snapshot하고 대입·오류·효과 순서는 공유 parser로 보존한다. `BaseLocaleOnly`/`Bypassed` 관찰은 실제 검색과 lexical form을 verifier에서 대조한다. 일반 simple-name guard로 승격하지 않으며 locative capture의 J Graph 변환도 namespace dependency 증명 전까지 거부한다. 함수 locative, 미정의 future reference, computed/abandon locative, numbered/indirect locale 및 path·실행 locale 전환은 Unsupported로 남긴다. named locale 자체 table의 명사는 아래 별도 단위로 지원한다. 회귀·고정 C 차분에는 base 공유/배열 snapshot, 지역 shadow 우회·대입, 우측부터의 조회와 실패 대입의 상태 보존을 포함한다. NJ-04 전체·L3는 계속 미완료다.

**NJ-04 named-own 명사 경계:** `a_probe_` 같은 직접 locative의 simple name과 locale spelling을 분리하고 `Engine.named_locales`의 독립 table·ScopeInstanceId로 조회/대입한다. 명사 lookup 때 locale이 없으면 생성하며, RHS 실패 전에 대상 locale을 미리 만들지 않는다. `a_base_`는 기존 base binding을 사용한다. `DirectLocaleOnly(start)`와 실제 `FoundScope::Locale(hit)`을 구분하여 verifier가 일치·POS·local 우회를 검사한다. 각 locale의 version이 같더라도 scope/generation을 혼동하지 않는다. named 함수/미정의 future reference, numbered locale, 임의 path 검색, path 변경/indirect/호출 locale 전환, computed 및 아래 경계 밖 abandon·컴파일 수용은 별도 Unsupported 경계다. C 감사의 named/base z 명사 fixture는 해당 단위에서 검증하며 전체 path 수용으로 간주하지 않는다. 일반 NJ-04/NJ-05 및 L3는 미완료다.



**PR #14 이력 통합:** NAME lexical 분류·locative copula는 #25에서 현재 frontend에 이식되고, 오류 전달(#22/#23)·base/named-own noun(#26/#27)·대입 버전(#28)이 후속 구현됐다. #14의 오래된 parser 전면 거부는 병합 시 현재 bounded runtime 지원을 보존하는 것으로 대체한다. 단순 by-value/abandon의 기존 지원도 유지한다. full locale/path/function/by-value locative 및 플랫폼 NAME 수용은 여전히 미완료다.

**NJ-04 direct own-table witness 검증 보강:** `DirectLocaleOnly`의 시작 ID와 발견 ID를 함께 변조하여 named locale을 base로, 또는 `a_base_`를 named locale으로 위장하는 capture를 거부한다. NAME의 실제 `_base_` 별칭 표기와 engine scope 선택을 대조하며 정상 base/named noun snapshot은 유지한다. 회귀는 양쪽 위장과 정상 capture를 검사한다. 이는 own-table 역할 검증이며 default z-path·일반 locale identity 매핑·full resolution witness 또는 L3 완료를 의미하지 않는다.

**NJ-04 검색 종류 witness 보강:** base/direct locative의 조회 기록을 `GlobalOnly` 또는 `CurrentFrameThenGlobal` 일반 이름 검색으로 위장하면 verifier가 거부한다. 일반 검색은 lexical Simple NAME만 허용하며 정상 base/named capture는 유지한다. 세 locative 표기의 변조 회귀로 경계를 검증한다. default z-path·완전한 namespace witness 및 L3 완료는 별도 미완료 계약이다.

**NJ-04 z own-table 선행 단위:** `a_z_` 직접 명사 조회·대입을 독립 z locale table과 ScopeInstanceId로 지원한다. 다른 named-own과 같은 snapshot·버전·local 우회·실패 RHS 보존을 사용하며 함수·미정의 참조와 컴파일 수용은 계속 거부한다. 기본 z noun 검색은 아래 단위에서 지원하며 임의 path 변경은 계속 미지원이다. 회귀·고정 C 감사는 z 자체 값, 배열 snapshot, local shadow 우회 및 실패 대입 이후 값을 검증한다. NJ-04 전체/L3 미완료.

**NJ-04 named default z 명사 경계:** named direct locative의 own table에 명사가 없으면 기본 z table에서 조회한다. `NamedDefaultZ { start, z }`와 실제 `FoundScope::Locale(z)`는 시작·발견 ID를 분리하고 local 우회·noun POS를 검증한다. read version은 발견 table을 사용하고 assignment version/commit은 지정 named own table만 사용한다. 첫 own 대입은 z 버전을 이전 값으로 가져오지 않는다. 최초 named locale 생성 전에는 ID를 가정하지 않으며 성공 lookup 후 생성된 scope 관찰을 capture에 기록한다. snapshot·실패 RHS·local shadow 우회 회귀 및 고정 C 감사가 해당 경계를 검증한다. path 변경, path 변경/epoch guard, full namespace witness와 NJ-04/L3 전체는 미완료다.

**NJ-04 explicit base default z 명사 경계:** `a__`와 `a_base_`는 base own miss 시 z noun을 읽고 `BaseDefaultZ { z }`/`FoundScope::Locale(z)`로 base 시작과 z 발견을 분리한다. local table을 우회하고 base own 대입 version/commit을 사용하므로 첫 대입 이전 버전은 없음이며 z 값은 보존한다. 두 별칭의 동일 값, snapshot, local shadow 우회, 실패 RHS 보존 및 잘못된 found ID 거부를 회귀·고정 C 감사로 검증한다. path 변경, path epoch guard·임의 path·함수/indirect/numbered와 full NJ-04/L3는 미완료다.

**NJ-04 일반 NAME default z 명사 경계:** 일반 이름은 local → base own → z noun → extension registry 순서로 조회한다. `SimpleDefaultZ { z }`/실제 found locale·generation·version을 기록하며 base/지역 대입은 own 대상 버전만 사용한다. 일반 z noun path 관찰의 제한된 guard는 아래 경계에서 지원한다. 일반 z path noun abandon은 아래 경계에서 지원한다. 일반/base/named 일관성, noun snapshot, local shadow/첫 지역 대입, 실패 RHS와 z 보존을 회귀·고정 C 감사로 검증한다. 임의 path/epoch guard·호출 locale 전환·함수/indirect/numbered와 full NJ-04/L3는 미완료다.

**NJ-04 일반 z-path noun abandon:** 일반 `a_:`는 local/base own 우선순위를 유지하고 fallback으로 찾은 z 항목을 반환·삭제한다. 단일 지역 word의 local binding 비삭제 규칙은 유지하며, 지역 miss의 z 발견은 삭제한다. snapshot 수명, 오른쪽 오류 전 삭제 없음, 삭제 후 오류에서도 삭제 유지, own-table 우선순위와 실제 발견 locale capture를 회귀·고정 C 감사로 검증한다. ordered effect/guard 및 명시적 locative abandon·임의 path는 계속 별도 미완료 계약이며 full NJ-04/L3를 완료하지 않는다.

**NJ-04 z abandon 관찰 검증 보강:** 삭제 event의 독립 pre-action lookup도 frame/search-state 계약을 검증한다. 프레임 없음은 NoFrame, 프레임 있음은 engine/z와 다른 ID 및 Absent/DeclaredUnbound여야 한다. frontend NAME 관찰을 그대로 둔 채 event frame을 engine/z로 위장하거나 local state를 바꾸는 회귀가 이전 검증 우회를 재현하며 이제 거부된다. 기존 값·삭제 의미와 guard/compiler 수용 범위는 유지한다. 완전한 namespace witness 및 NJ-04/L3 전체는 미완료다.

**NJ-01 locative abandon enqueue 선행 경계:** 적법한 `a___:`, `a_base__:`, `a_probe__:`, `a__holder_:`는 NAME 주소 형식과 by-value/abandon 정책을 독립적으로 보존한다. NAME payload는 inflection을 제거한 주소, 원본 span/word index와 lookup/abandon flag는 유지한다. malformed/limit 검증은 이름 문법 단계에 남는다. static parser와 runtime은 아래 explicit-base 및 named own noun 경계 밖 미구현 locative 삭제를 명시적으로 Unsupported로 거부하여 flat key 조회나 기존 table 삭제로 잘못 해석하지 않는다. enqueue 성공은 locale 삭제 실행 또는 NJ-01/NJ-04/L3 전체 완료가 아니다. base own noun 실행은 아래 경계에서 지원하며 z fallback 삭제는 아래 경계에서 지원한다.

**NJ-04 explicit-base own noun abandon:** `a___:`/`a_base__:`는 local을 우회하여 base own noun을 반환·삭제한다. own 함수 및 미발견 path/future 삭제는 Unsupported로 유지하며 상태를 변경하지 않는다. noun snapshot 수명, RHS 오류 전 삭제 없음, 삭제 후 오류에도 삭제 유지, 양쪽 실행 경로 local shadow 우회 및 실제 base found capture를 회귀·고정 C 감사로 검증한다. static/deferred parser와 Graph/guard 수용은 별도 미지원이며 full NJ-04/L3는 미완료다.

**NJ-04 explicit-base default z noun abandon:** base own miss의 `a___:`/`a_base__:`는 실제 발견 z noun을 반환·삭제한다. local 우회, BaseDefaultZ/실제 found locale 및 삭제 전 version/generation을 보존한다. snapshot·오류 순서·지역 shadow 우회·두 alias 일관성과 event search/found 변조 거부를 회귀·고정 C 감사로 검증한다. own 함수·미발견 path/future 삭제 및 static/Graph/guard 수용은 별도 미지원이며 full NJ-04/L3는 미완료다.

**NJ-04 named own noun abandon:** `a_probe__:`/`a_z__:` 등 direct named 주소는 local을 우회하여 해당 locale own noun을 반환·삭제한다. 다른 table과 noun snapshot을 보존하고 최초 오류·삭제 후 오류·지역 shadow 및 실제 found capture/변조 거부를 회귀·고정 C 감사로 검증한다. 미발견 path/future 삭제·함수·indirect/numbered 및 static/Graph/guard 수용은 Unsupported로 유지하며 full NJ-04/L3는 미완료다.

**NJ-04 named default z noun abandon:** direct named own miss의 `a_probe__:`는 실제 발견 z noun을 반환·삭제한다. 시작 named locale과 found z·local 우회·삭제 전 version/generation을 분리하며 첫 locale 조회·snapshot·최초 오류·삭제 후 오류와 start/found/frame 변조 거부를 회귀·고정 C 감사로 검증한다. 임의 path/future·함수·indirect/numbered 및 static/Graph/guard 수용은 미지원이며 full NJ-04/L3는 미완료다.

**NJ-04 일반 z noun guard:** 일반 NAME의 SimpleDefaultZ noun hit도 제한된 SimpleNameGuard를 만들 수 있다. 검사 때 전체 lookup observation을 재확인하여 base shadow·z 재대입·삭제·동일 값 재생성 ABA·다른 engine을 거부하며 무관한 z 이름 대입은 허용한다. 이는 검사 시점의 사실이며 read hoisting·effect replay·컴파일 실행 허용이 아니다. abandon/function path guard와 임의 path epoch 및 full NJ-04/L3는 미완료다.

**NJ-04 locative noun guard:** 검증된 runtime base/direct named own 및 기본 z noun 관찰에서 별도 LocativeNounGuard를 만든다. 전체 lookup 재검사로 own shadow·재대입·삭제 후 동일 값 재생성 ABA·engine/frame 변경을 거부하며 무관한 이름 대입은 보존한다. SimpleNameGuard의 일반 이름 경계는 확장하지 않고 abandon/function/indirect/numbered·임의 path와 compiler/hoisting/effect replay 수용은 계속 미지원이다. 이는 check-time 사실이며 full NJ-04/L3 완료가 아니다.

**NJ-07 대입 버전 경계:** parser commit의 이전/새 버전은 `assignment_version(name, local)`로 실제 대입 대상 테이블에서 관찰한다. 지역 선언이 아직 없을 때 RHS가 전역 값을 읽더라도 지역 첫 commit의 이전 버전은 없음이며, 다음 지역 commit은 지역 버전을 사용한다. base/named-own locative는 지역 flag와 무관하게 해당 own table을 사용한다. 읽기용 `version`은 기존 조회 의미를 유지한다. 실패한 RHS는 commit을 만들지 않는다. 이 경계는 default z path의 읽기 발견 테이블과 쓰기 대상 테이블을 분리하기 위한 선행 단위이며, path 지원·완전한 namespace commit witness·L3 완료를 뜻하지 않는다. 회귀는 지역 첫/반복 commit, 전역 값 보존, 전역 첫 commit과 RHS 오류를 검사한다.

| ID | jsource 관찰 계약 | RustJ 현재 구조/차이 | 수용 조건 |
|---|---|---|---|
| **NJ-01 NAME 인식** | 단순 이름(내부 `_` 허용), direct `name_locale_`, indirect `name__holder`, base alias `name__`; 적법한 `name_:`는 `NAMEBYVALUE/NAMEABANDON` 별도 inflection. `nfs` 길이/오류 precedence, `vnm` 문법 유지 | 일반 locative 어휘 분류와 단순 by-value/abandon 경계는 구현됨. locative by-value/abandon 및 32-bit numeric locale 제한 등 플랫폼 경계 미검증 | 모든 jsource-valid NAME을 정상 parse queue에 전달하고 invalid/limit/spelling만 해당 J 오류로 구분. `Unsupported`를 최종 NAME 구문 결과로 인정하지 않음 |
| **NJ-02 enqueue 환경** | `jtenqueue env=0/1/2`, NAME to-lookup/assignment flags, copula 분류. locative 대상 `=.`는 explicit body에서도 **global** | env=0/1/2 일반·locative copula 분류와 locative global 승격은 소스 golden/public C로 검증됨. locative by-value flag와 완전한 내부 trace는 미검증 | 원본 queue 분류를 C 소스 유도 golden과 실행 가능한 J oracle에서 검증 |
| **NJ-03 조회 결과/오류 API** | `syrd`는 **미정의 이름**과 **locale 해석 실패**를 구분하고, 실제 lookup에는 local 여부·찾은 locale·이름의 POS·동적 error가 개입 | `RuntimeParserHost::lookup(&str)->Result<Option<ParserNameBinding>>`은 오류/성공/미정의를 구별한다. base 및 named-own noun의 명시 scope 선택·관찰을 제한적으로 구현; 일반 locale/path/holder 해석과 witness는 미구현. `binding_version(name)`도 전역 flat key 중심 | 최소 `Result<Option<...>>`에 해당하는 **오류 있는 lookup 경계**와 actual resolution witness(local/locale/path/context/version)를 분리하여 보존. 구체 타입 이름은 구현 때 선택 |
| **NJ-04 실제 namespace** | 일반 이름: invocation local → 현재 locale → **그 locale의 명시 path** (path의 path는 재귀 탐색하지 않음). Direct는 지정 locale부터, indirect는 holder의 현재 값으로 시작 locale 결정. 숫자 locale·locale 생성/변경/삭제·path 영향 | `Engine.names`의 base 명사 조회·대입과 마지막 `LocalFrame`을 우회하는 `a__` runtime 경계만 구현. named locale별 own-noun table은 별도 단위로 구현. 기본 z noun path는 아래 단위로 구현. 임의 path·검색 path 변경·current locale 상태/전환은 없음 | namespace를 locale-scoped로 구현하고 local, current, path, direct/indirect, name shadowing을 같은 의미·오류 순서로 수행 |
| **NJ-05 locative 호출 문맥** | `f_locale_` 호출 시 global name 실행의 **current locale**가 시작 locale로 전환되고 복원됨. 값이 경로상의 다른 locale에서 발견돼도 **검색한 locale ≠ 실행 locale** 가능 | Function `NameRef(String)`와 runtime 호출이 locale context·search hit locale·restore obligation을 별도로 모델링하지 못함 | lookup 시작 locale, 실제 발견 locale, 호출 실행 locale, 복원 이벤트를 구분. caller의 local frame이 callee로 잘못 전파되지 않게 함 |
| **NJ-06 값 vs NameRef** | noun은 stack 진입 시 값으로 snapshot, 일반 Verb/Adverb/Conjunction은 당시 POS에 따른 nameref와 호출 시 late lookup; locative 함수는 호출 locale context 필요. `name_:`는 by-value 후 binding abandon/delete. undefined 일반 NAME의 파서 fallback 경로는 즉시 오류와 동일하지 않음 | noun snapshot, Function `NameRef`, 일부 POS mismatch/domain은 존재. 하지만 `name_:` 삭제·full modifier/locative NameRef·원본 undefined 경로 미완료 | 재바인딩/품사 변경/특수 by-value-name과 실제 NameRef/실행 context에 대한 차분 수용 |
| **NJ-07 대입 대상** | local `=.`, global `=:`; locative는 모두 global. 중간·연속 대입, **계산된 단일 NAME / 여러 NAME**(문자열·boxed list), locally-defined name에 대한 public collision의 J Domain 오류 | parser `PendingAssignment`와 `assign_scoped(&str,..)`가 주로 단일 raw 이름을 전제로 함; 다중/계산된 name·locale table 선택 일반화 미완료 | RHS 평가 시점/결과 JEntity, name 계산, 해당 table commit, 첫 오류 및 side-effect ordering과 rollback 경계 대조 |
| **NJ-08 전체 문장 효과** | `p.c` stack/reduction에 따른 right-to-left lookup, assignment/locale mutation, lookup 후 함수 호출의 dynamic context. `18!:` locale/path 변동은 재조회·guard에 영향 | 일부 right-to-left runtime action·local frame은 구현. compiler proof는 단일 name/version으로 current locale/path 변경을 증명하지 못함 | 전체 문장 선행 snapshot/잘못된 함수 eager capture 금지. namespace-version뿐 아니라 관련 locale/path·local shadow·holder 재바인딩 witness를 필요한 시점에 재검증 |
| **NJ-09 compiler target** | 사용자 J locale이 언어 의미의 namespace | `CompilationTargetLocale`은 설계상 **별도 compiler namespace**로 이미 분리되어 있어 문제가 아님 | compiler target locale을 J language locale 조회·대입으로 오인하거나 두 binding table을 공유하지 않음 |

**구현 선행순서:** (1) NJ-01/02 NAME 블록과 enqueue의 원본 동등성 → (2) NJ-03/04 **오류를 전파할 수 있는 lookup·locale namespace** → (3) NJ-05/06 locative 호출·by-value/abandon·NameRef → (4) NJ-07/08 대입/오류/재바인딩/평가 순서 → (5) P6 C `j64`/`j64avx2`와 Rust reference의 독립 차분. F1/P4를 문법 인식만으로 완료 처리하지 않는다. unsupported는 **현재 구현 미수용**만 나타내며 J NAME feature를 제외하는 설계 선택이 아니다.

**부정·긍정 수용 시험 등록부:** `NJ-V01` simple/internal underscore·valid direct/indirect/base alias·`name_:` vs invalid/limit/spelling; `NJ-V02` env=0/1/2+locative `=.` 플래그; `NJ-V03` local shadow/unbound fallback/locale path 한 단계와 missing/invalid locale error; `NJ-V04` indirect holder의 값·재바인딩·오류, 숫자/소멸 locale; `NJ-V05` locative function의 실행 current locale, 경로에서 발견된 function, recursive/frame restoration; `NJ-V06` noun snapshot vs late function NameRef·POS 변화 및 `name_:`의 삭제; `NJ-V07` 계산된/복수 assignment·local/global 충돌·first J error/effect 순서. 기록: `source revision | 실제 C binary hash/variant | Rust commit | positive + one-mutated-negative | value/POS/locale/path/effect/error | pass/fail/unrun`. 현재 **모두 미수용**이며 기존 소규모 enqueue 테스트의 통과를 full name compliance로 승격하지 않는다.

#### 3.7.3 NAME 의미론적 조회와 최적화 provenance 분리 (2026-10-07; 설계 전용)

**결정:** `jsource`의 NAME 조회 **시점**은 그대로 두면서, NAME이 값으로 치환되더라도 **원본 NAME occurrence와 조회 사건·증거**를 J Graph/A3 및 최적화기에 남긴다. Compiler가 편하다는 이유로 noun을 늦게 조회하거나 verb를 미리 실행 함수로 확정하지 않는다. 새 실행용 NAME IR을 강제하지 않고 기존 `EnqueuedWord`, `ParseCapture`, `FunctionHead::NameRef`, J Graph `ReadNoun`, A3 `ReadNoun/VerbReference`를 우선 확장한다. 여기의 명칭은 **제안된 개념 계약**이지 구현된 Rust 타입이 아니다.

**jsource 기준:** 고정 [w.c `jtenqueue`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/w.c), [p.c `jtparsea`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c), [s.c `jtsyrd/jtsymbis`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/s.c), [sn.c `vnm/nfs`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sn.c). `p.c`는 NAME을 parser stack으로 들여올 때 **noun이면 그 순간의 값으로 치환**, 일반 verb라면 **당시 POS로 parser row를 결정하되 `nameref`를 보존하고 실제 호출에서 이름을 재조회**한다. Adverb/Conjunction과 gerund/fork는 생성 시점의 실제 modifier/operand 조회가 있으므로 “모든 function은 late”로 단순화하지 않는다. Direct/indirect locative, `name_:` by-value/abandon, `mnuvxy` 역시 별개의 J semantics다.

| source 역할 | J 실행이 사용하는 값 | 최적화기에 계속 남길 정보 |
|---|---|---|
| **Noun NAME** | 스택 진입 **시점**에 확보한 noun snapshot. 뒤의 재대입은 이 값에 영향 없음 | NAME spelling·syntax form·word/span·발생 ID, 실제 parser read 지점, local/locale/path resolution과 binding identity, 해당 snapshot의 dtype/Shape/rank |
| **일반 Verb NAME** | parser-time POS에 맞는 NameRef; **call-time** 실제 callable 조회·POS 오류 | 원래 NAME, 기대 POS, 관찰된 primitive/innate-rank **후보 힌트**와 재조회 의무. 미증명 함수 고정 금지 |
| **Modifier/gerund/fork NAME** | 각 constructor가 **생성 시점 실제로 읽거나 고정한** operand/function, 또는 별도 late ref | 생성 시점 snapshot 이벤트와 호출 시점 NameRef의 provenance·불변 범위 구분 |
| **Locative/특수 NAME** | direct/indirect locale 조회, 실행 locale 전환·복원, `name_:` abandon 효과 | 원 표기·holder read, local shadow, 검색 시작/실제 발견/함수 실행 locale, path·version·generation·원본 오류 순서 |

**독립 identity(한 sidecar/기존 IR에 구현해도 됨):**

~~~text
NameOccurrence {
  id, original_spelling, address_form: Simple|Direct|Indirect,
  read_policy: Ordinary|ByValue|ByValueAndAbandon, special_operand?,
  enqueue_word_index, source_span, parser_use_site
}
NameResolutionEvent {
  occurrence_id,
  time: StackNounRead|StackPOS|ConstructorRead|CallLookup|
        AssignmentTarget|LocaleRead|Abandon,
  outcome: NounSnapshot(value_id)|FunctionRef(expected_pos)|
           ConstructorCaptured(entity)|MissingNameRef|WriteResult|JError,
  lookup_witness?: {call_frame, local_present_or_absent,
                   start_locale, search_path_order, found_locale, execution_locale,
                   symbol_generation, binding_version, actual_pos,
                   indirect_holder_reads, locale_path_epochs},
  effect/error_order_edges, source_diagnostics
}
OptimizationNameEvidence {
  event_id, observed_pos?, candidate_callable_family?,
  observed_dtype_shape_rank?, constructor_fixed_facts?,
  status: Observed|ProvenStable|GuardRequired|Unknown,
  proof_or_guard_footprint?
}
~~~

**절대 동일시하지 않을 것:** source `NameOccurrence` ≠ 실제 `ResolutionEvent` ≠ `JEntity/SSA ValueId` ≠ 분석용 `OptimizationNameEvidence` ≠ 물리 `BufferId`. 같은 문자열 NAME이라도 occurrence, frame, locale, path, local 유무에 따라 다른 심볼이다. `NameVersion` 하나로 **조회에서 local이 없었다는 부정 증거**, path 무변화, indirect holder 재바인딩, locale 삭제·재생성을 증명할 수 없다.

**실행과 재사용의 시간 차이 예:**

~~~text
a =: 1 2 3
... a ...    J stack-read(a) -> noun v0
             capture event#A(name="a", resolved_binding, value=v0)
             J Graph ReadNoun(a,event#A) -> A3 SSA v0 + source link
a =: 9       이미 읽은 v0는 불변; 이후 읽기는 새로운 event.

f =: +
... f ...    parser POS=Verb, NameRef("f"), expected_pos=Verb
             optimization: observed Add-family HINT (not a frozen target)
f =: *
... f ...    새 호출은 현재 f를 읽거나 call-time guard로 적법성을 입증해야 함
~~~

이 예의 noun snapshot은 **실제 J parser read 시점**의 값이지 compiler가 사전 분석 시 얻은 값을 모든 미래 실행에서 캐시하라는 뜻이 아니다. 실행을 재사용하는 경우 매 invocation에서 올바른 read boundary의 값을 공급하거나 안정성 proof/guard 및 효과·오류 순서 증거가 필요하다. 반대로 이미 read를 수행한 뒤에는 같은 event를 재조회해서도 안 된다.

**Stage별 소유권과 최소 구현 변경:**

1. **F1 Enqueuer:** NAME의 문법/locative/by-value 분류와 source identity만 유지한다. lexical 단계에서 POS·binding·primitive target을 고정하지 않는다.
2. **P4 runtime parser:** `resolve_stack_item`의 noun value snapshot은 기존과 같은 타이밍으로 실행하되 origin event를 기록한다. 함수는 POS 검사와 미래 NameRef를 분리. 현재 `RuntimeParserHost::lookup(&str)->Result<Option<ParserNameBinding>>`은 **오류 전달 조회** 경계를 제공한다. 실제 locale 해석과 witness 연결은 후속 구현이다. `start_locale`·`found_locale`·`execution_locale`는 별개.
3. **Semantic/JEntity:** `NameUse`·`BoundProgram`·`ModifierSnapshot`·`NameRankSnapshot`의 관찰값을 guard와 혼동하지 않는다. frozen constructor operand는 별도 event이고 일반 NameRef는 late.
4. **J Graph:** `NodeKind::ReadNoun{name,version}`와 `VerbValue/Apply(NameRef)`에 event/provenance 링크를 유지. `GraphAnalyzability::RequiresSpecialization`은 미증명 dynamic function에서 유지. 값만 남기고 원본 name을 지우지 않는다.
5. **A3:** `OpKind::ReadNoun{symbol,version}`, `VerbReference(Callable)`, `Write`, `SemanticCheck`, `order_after`와 J Graph/source event의 연결을 검증. `Symbol{name,scope=CurrentGlobal|LocalFrame}`은 아직 locale/path 전체를 표현하지 못한다. namespace semantic effects를 단순 SSA 값·literal로 지우지 않는다.
6. **M3+ optimizer:** known POS/primitive/Shape/rank는 *transform candidate*만 부여한다. `ProvenStable` 및 `GuardRequired`의 proof에는 actual use 시점까지 local absence/shadow, current/search locale/path, indirect holder, binding versions, statement 내 namespace writes, first error/effect order를 포함. guard miss는 J 오류가 아니며 effect 이후 문장 전체 replay 금지.

**프레임워크 비교:**

| 참조 | 빌릴 부분 | 빌리면 안 되는 부분 |
|---|---|---|
| [MLIR SymbolRef/SymbolTable](https://mlir.llvm.org/docs/SymbolsAndSymbolTables/) | symbolic NAME uses와 SSA value 흐름 분리 | 정적 MLIR symbol table을 J의 동적 locale/path로 대체 |
| [LLVM MemorySSA](https://llvm.org/docs/MemorySSA.html) | read/write clobber, hoist legality와 순서 의존성 | J NameRef/POS/locale/first error를 일반 메모리 load 취급 |
| [Truffle Assumption](https://www.graalvm.org/truffle/javadoc/com/oracle/truffle/api/Assumption.html) | speculate한 바인딩·함수의 무효화 가능 guard | 단일 전역 boolean/flat version으로 path·holder 무변화를 주장 |
| [JAX captured constants](https://docs.jax.dev/en/latest/internals/constants.html) 및 [JIT global caveat](https://docs.jax.dev/en/latest/notebooks/Common_Gotchas_in_JAX.html) | captured value와 runtime input 분리 | JAX의 trace-time global 고정을 J의 dynamic name semantics에 이식 |

**수용 단계(구현/검증은 모두 UNRUN):**

- [ ] **NP-01:** NAME occurrence ID·form·span/word·중복 출현 원본 보존. **NP-V01:** 다른 occurrence/locale를 문자열만으로 병합하면 실패.
- [ ] **NP-02:** noun stack-read, function parser POS/call NameRef, modifier constructor snapshot을 정확한 시점에 분리. **NP-V02:** noun 재조회 지연이나 verb 조기 freeze 거부.
- [ ] **NP-03:** 오류 있는 locale-aware lookup, local presence/**absence**·path/holder witness·실행 locale. **NP-V03:** shadow/path/holder 바뀌었는데 cached binding 재사용 거부.
- [ ] **NP-04:** Parser capture→Semantic→J Graph `ReadNoun/NameRef`→A3 `ReadNoun/VerbReference` source event 연결. **NP-V04:** noun literal fold로 NAME provenance 소실하면 거부.
- [ ] **NP-05:** `Observed/ProvenStable/GuardRequired/Unknown` 구분과 guard footprint/first error·effect-order 증명. **NP-V05:** `Observed` 단독으로 Add kernel 특수화 거부.
- [ ] **NP-06:** locative NameRef, `name_:`, gerund/fork/Rank constructor, assignment·rebinding·POS 변화의 조회/고정 시점 보존. **NP-V06:** 조기/후기 조회 혼동 및 error 순서 변경 거부.
- [ ] **NP-07:** pinned J C `j64/j64avx2` ↔ Rust semantic reference·event ↔ Graph/A3·실제 route의 차분. **NP-V07:** J 값·POS·첫 오류·namespace effect 불일치를 실패로 판정.

등록 양식: `source revision | NAME occurrence/event→SSA IDs | oracle/binary | positive + one-invariant-negative | proof/guard owner | PASS/FAIL/UNRUN | CI SHA`. **문서 설계 완료가 F1/P4/P5/M3 구현 완료를 뜻하지 않는다.**

#### 3.7.4 NAME 최적화 캐시와 멀티프로세서 인계 계약 (2026-10-07; 설계 전용)

§3.7.3과 **NP-01~07이 유일한 수용 체크리스트**다. 이 절은 최신 NAME 감사(`cd6ac8c`)에 실행 수명·동시성·캐시 계약을 보강하며 새로운 NAME 구현이 완료됐다는 뜻이 아니다. 이전 채팅의 인계는 저장소의 M3→M4 계약을 기준으로 이어간다. 첫 구현은 단일 CPU로 닫고, 동일 계약을 이후 worker/device 실행으로 확장한다.

**Identity와 저장 비용(NP-01/03/04):** Engine 소유 interner의 `NameId`는 spelling 저장·해시 비용만 줄인다. 전역 의미 identity가 아니다. 제안 `BindingStamp = (EngineIdentity, ScopeInstanceIdentity, SymbolGeneration, Revision)`은 frame 재사용, expunge 후 같은 이름 재생성, locale 삭제·재생성에 따른 ABA를 막는다. 현재 per-name `NameVersion`을 이 stamp 전체로 간주하지 않는다. 요청한 NAME/검색 시작점과 실제 발견된 binding key도 구분한다. occurrence ID는 span과 expanded-word 위치를 포함하며 같은 spelling의 중복 출현을 병합하지 않는다. Provenance sidecar는 ID·요약·증거만 보유한다. 모든 중간 noun을 `Arc<Value>`로 붙잡으면 liveness와 버퍼 재사용이 훼손되므로 payload 수명은 실제 dataflow와 실행 lease가 소유한다.

**Guard에서 실행으로의 인계(NP-03/05):** guard 성공 후 포인터만 가져오는 방식은 검사와 사용 사이 재바인딩을 허용한다. 의미론적 lookup 지점에서 namespace owner가 lookup/POS/guard 검증과 immutable entity/value lease 확보를 하나의 일관된 작업으로 수행한다. 최초 단일 owner에서는 순서화된 실행으로, 향후 공유 namespace에서는 lock 또는 검증된 snapshot protocol로 구현할 수 있다. 이 lease는 **이미 수행한 read/call lookup 하나**를 보호하며 함수 본문의 다른 동적 NAME까지 고정하지 않는다. Guard footprint에는 local absence, path epoch, indirect holder와 실행 locale가 포함된다. Unknown namespace effect/foreign call은 증거 갱신 또는 분석 장벽이며, guard miss 후 이미 일어난 효과를 문장 전체 replay로 반복하지 않는다.

**최적화에 사용하는 방법(NP-04/05):** source NAME → resolution event → semantic function/value identity → relevant dtype/rank/shape/effect facts → proof/guard → specialization 순서를 유지한다. 예를 들어 `f`가 Add로 관찰되면 fusion 후보를 만들 수 있지만 call-time target/POS 및 lookup 의존성이 검증돼야 Add로 실행할 수 있다. `f`의 spelling을 primitive dispatch key로 사용하지 않는다. Summary cache와 executable cache는 분리하고, 관련 binding stamp·lookup dependencies·semantic identity·필요한 argument facts를 키로 삼는다. 실행 코드 캐시는 추가로 target/numeric policy를 포함한다. Shape와 상수를 무조건 전부 키에 넣지 않으며 bounded cache와 merge/widening을 적용한다. 동일 구조의 함수는 적법한 범위에서 코드를 공유하되 occurrence/source 진단은 별도 mapping으로 보존한다.

| 배열 compiler의 실제 구조 | RustJ에 적용할 설계 판단 |
|---|---|
| [JAX jaxpr](https://docs.jax.dev/en/latest/601/jaxpr.html)의 typed input/constant/intermediate 변수 | lookup으로 확보한 noun은 explicit SSA input/constant carrier로 전달하고 원 NAME/event는 sidecar에 남긴다. JAX의 trace-time global 고정은 가져오지 않는다. |
| [PyTorch Dynamo](https://docs.pytorch.org/docs/2.14/user_guide/torch_compiler/compile/programming_model.dynamo_core_concepts.html)의 guards·graph breaks | 동적 NAME 구간과 검증된 배열 region의 경계를 명시한다. J의 POS·locale·오류 순서를 보존하는 continuation은 RustJ가 별도로 소유한다. |
| [OpenXLA HLO→Thunks](https://openxla.org/xla/hlo_to_thunks)의 schedule 이후 buffer assignment | NAME/SSA identity로 주소나 버퍼 재사용을 결정하지 않는다. alias/liveness/readiness 증명 후 physical allocation과 실행 수명을 결정한다. |

**Worker/device 경계(NP-05):** namespace owner가 원래 J 시점에 확보한 immutable value versions, 검증된 callable 또는 닫힌 array region, readiness/effect/error dependencies와 leases를 physical 실행에 넘긴다. CPU worker가 임의 시점에 원 NAME을 재조회하지 않는다. Region 안에 더 늦은 동적 조회가 필요하면 semantic boundary를 유지하고 그 지점에서 owner로 돌아간다. Lease는 enqueue가 아닌 실제 작업 완료까지 유지한다. 별도 process에서는 raw pointer/Engine-local NameId를 직렬화하지 않고 value/region transport와 수신 측 identity remapping을 명시한다. 이것은 shared mutable namespace를 프로세스 간 자동 복제하는 설계가 아니다. CUDA 구현은 계속 보류다.

**검증 증거와 남은 구현(NP-07):** [native Windows C probe](../../../tools/name_system_research.py)와 [기계 판독 결과](../../../reports/name-system-research-windows.json)를 추가했다. noun snapshot, verb late lookup, undefined→defined, 기대 POS 오류, 문장 내 대입/조회 순서, direct locative, `f.` 고정, `name_:` abandon의 **8건 × C j64/AVX2 = 16/16 PASS**다. Source pin `13994ffa…`와 reference asset pin `ded7793…`은 별개이고 JSON에 실제 파일 SHA256을 기록한다. 재현: native Windows Python으로 `tools/name_system_research.py --assets-root <reference-assets-checkout>` 실행. 이 결과는 **C 의미 조사**이며 RustJ locative/abandon 구현이나 NP-07 전체 차분 통과가 아니다. 통합 기준 `cd6ac8c`의 깨끗한 작업 트리에서는 `cargo test --locked [--features portable] --test semantic --test parser_capture name`으로 default/portable 각각 **17/17 PASS**, `test_oracle_protocol.py` **9/9 PASS**, `git diff --check`를 확인했다. 전체 Rust suite·fmt/clippy·Linux·GPU·GitHub CI는 이번 설계 변경에서 실행하지 않았다.

NP-03 부정 테스트에 frame/locale/symbol 삭제·재생성과 stale stamp, NP-04에 최적화 후 provenance 및 중간 payload 비보유, NP-05에 guard→lease 사이 재바인딩·worker 완료 전 buffer reuse·효과 이후 guard miss를 추가한다. 구현 순서는 NP-01/02 occurrence/event → NP-03 namespace/stamp → NP-04 IR 연결 → NP-05 guard/lease → NP-06 특수 NAME → NP-07 독립 차분이다. **모든 전체 수용 게이트는 계속 미완료**이며 C probe 일부 통과로 체크하지 않는다.

#### 3.7.5 추가 배열 compiler 비교와 NAME 호환성 재검토 (2026-10-07)

**판정:** §3.7.3/4의 NAME carrier는 필요한 출발점이지만 충분한 compiler 계약이나 jsource 호환 구현은 아니다. 이름 출처 보존에 더해 **SSA use-def/CFG 합류, namespace effects, alias·escape·liveness, shape facts의 범위, transformation provenance와 verifier**가 있어야 최적화의 적법성을 검증할 수 있다. 다음은 upstream 자료에서 관찰한 구조와 RustJ에 대한 설계 판단을 구분한 보강이다. 새 IR 계층을 의무화하지 않는다.

| 추가 참조 / upstream 관찰 | RustJ에 채택할 부분 | 그대로 채택할 수 없는 부분 |
|---|---|---|
| [Futhark language reference §3.10/3.11](https://futhark.readthedocs.io/en/latest/language-reference.html): size variables, branch/loop alias 합류와 fixed point, consuming parameter와 alias-free return | 함수 요약에 argument/result alias·escape·read/write 정보; CFG 합류와 loop fixed point; 실제 입력 값의 shape에서 유도한 size identity | consuming type과 higher-order 제약을 J 문법 제한으로 추가하지 않는다. 일반 J는 alias가 살아 있으면 copy/다른 적법한 실행을 선택한다. |
| [TVM Relax](https://tvm.apache.org/docs/deep_dive/relax/learning.html), [Var/DataflowVar/MatchCast API](https://tvm.apache.org/docs/reference/api/python/relax/relax.html): visible/dataflow variable 구분, symbolic shape, runtime structural check, pure dataflow region | J NAME와 SSA 내부 변수 구분; shape symbol을 source spelling과 분리; proof로 닫힌 array region과 runtime semantic region의 명시 경계 | pure라는 분류만으로 J error·NAME lookup의 reorder가 허용되지는 않는다. `MatchCast` 스타일 검사는 J에 맞는 오류 시점/guard failure 계약이 필요하다. |
| [MLIR One-Shot Bufferization](https://mlir.llvm.org/docs/Bufferization/): tensor SSA use-def와 read-after-write conflict 분석 후 in-place/copy 선택 | noun snapshot의 alias와 생존 reader를 추적하고 마지막 semantic use 및 worker completion을 만족할 때만 버퍼 재사용 | binding version·NameId·reference count 하나를 uniqueness/alias/liveness 증명으로 사용하지 않는다. |

**다음 단계로 넘길 carrier의 보완(NP-01~06):**

1. **문법 축을 직교화:** §3.7.3의 개념 예시에서 `ByValueAbandon`을 locative와 배타적인 form으로 두었던 표기를 수정했다. `address_form`(simple/direct/indirect와 holder/terminal selector), `read_policy`(ordinary/by-value/by-value+abandon), special operand/context flags를 따로 보존한다. 원본 spelling을 없애거나 parser 전에 실제 binding을 고정하지 않는다. Numeric debug-frame locative와 implicit operand의 정확한 분류는 pinned source와 차분으로 닫는다.
2. **SSA/CFG와 namespace를 별도 합류:** `ReadNoun(event) -> ValueId`는 실행 시점의 read이며 compile-time 환경 캡처가 아니다. branch는 도달 가능한 값·facts·alias를 block arguments/phi 상당으로 합류하고, loop는 widening/fixed point를 적용한다. `NameRef`와 namespace state는 별도 효과 의존성으로 유지한다. 한쪽 branch의 binding proof를 합류 이후 무조건 적용하지 않는다. 현재 A3는 single-block/Return-only이므로 일반 NAME CFG 특수화는 미구현이다.
3. **요약을 값과 호출에 연결:** dtype/rank/shape constraints, alias/escape, effect/error 및 nested dynamic NAME dependency는 관련 value/call 분석에 둔다. Source NAME spelling이나 불변 FunctionEntity identity에 호출별 관찰을 넣지 않는다. Named function target guard가 성공해도 함수 본문의 dynamic name이 자동으로 stable/pure가 되지 않는다. Unknown call/execute는 보수적인 effect summary를 가진다.
4. **Provenance는 일대일이 아님:** fold/CSE/fusion/inlining 후 한 노드가 여러 occurrence/event에서 유래할 수 있다. Interned origin-set/공유 derivation DAG 상당의 many-to-many mapping으로 보존하되 실제 runtime lookup 사건을 중복 실행하지 않는다. 제거된 체크/조회에는 proof·guard·order witness를 연결한다. Source metadata만으로 eliminated operation의 적법성을 증명하지 않는다.
5. **Semantic state와 analysis state 구분:** 실제 조회 결과는 noun snapshot, expected-POS NameRef, constructor capture, 미정의 일반 function reference, 또는 J 오류다. `Unknown/Observed/GuardRequired/ProvenStable`은 별도 분석 상태다. 미정의 일반 NAME를 항상 즉시 Value 오류로 바꾸거나 `Unknown`을 J 오류처럼 실행하지 않는다.

**실제 구현을 C와 비교한 범위:** [audit 도구](../../../tools/name_compatibility_audit.py)와 [Windows 결과](../../../reports/name-compatibility-windows.json)는 `c151f36` runtime source의 **26개 독립 stateful fixture × j64/AVX2 × direct/semantic-reference = 104건**을 비교했다. **64 matched, 40 unsupported_gap, semantic mismatch 0**이며 각 조합은 같은 **16개 fixture 일치 / 10개 미지원**이다. 성공률을 전체 J NAME 호환율로 해석하지 않는다. 모든 setup 문장과 관찰한 query의 dtype/shape/data 또는 오류 종류를 비교하고, CLI가 unsupported definition에서 입력을 중단한 경우 미관찰 suffix를 별도 기록한다. C adapter는 기존 `Oracle.eval`의 assignment/result 관찰을 사용하므로 내부 lookup 사건·POS reflection·namespace 전체 상태의 동등성을 검사한 것은 아니다.

| NAME 기능 | 이 audit 및 코드 조사에서 확인한 현재 경계 |
|---|---|
| 일반 noun snapshot, array alias/reshape 후 기존 값 유지, 실패한 RHS의 binding 보존 | 해당 fixture들은 두 DLL·두 runtime 경로에서 일치. 모든 alias/storage encoding을 검증했다는 뜻은 아님 |
| 일반 verb late lookup·alias chain·undefined→defined·미정의 호출·기대 POS 오류, 문장 내 대입/조회 순서 | 해당 fixture 일치. rank/header 변화·recursive/self·모든 train/constructor 순서는 추가 수용 필요 |
| named adverb construction, nameless modifier alias, conjunction alias | 선정 fixture 일치. **모든 function은 동일한 late 정책**이라고 일반화하지 않음 |
| straight-line explicit **modifier**의 local shadow/unbound fallback | 해당 fixture 일치. `3 :` 일반 explicit verb의 local 호출 두 fixture는 Unsupported; local frame 전체 미구현이라고도 하지 않음 |
| direct/base/indirect locative, 실행 locale 문맥 | fixture Unsupported. source grammar 일부 검사는 있으나 locale namespace/path·실행 전환/복원 호환은 미완료 |
| `name_:`, `f.`, 계산된 단일/복수 NAME 대입 | fixture Unsupported. C 조사 통과를 Rust 구현 통과로 승격하지 않음 |
| locale 생성/삭제·path 한 단계·numeric/debug-frame locative·special `m n u v x y`의 전체 조합 | 이번 실행 audit 미검증. NJ/NP 수용 게이트 유지 |
| name reflection `4!:`/`5!:`, execute `".`, recursion/escape/fix, namespace mutation 후 guard invalidation | 전체 호환 미검증. 관찰 가능한 name class/list/representation 및 first error·effect trace까지 NP-06/07 확대 필요 |
| provenance 기반 compiled specialization, CFG guard, native M4 및 worker/device 인계 | 설계 단계/전체 수용 미완료. `--semantic-reference`는 컴파일된 CPU route가 아님 |

**검증과 구현 순서:** NP-01/02 event+직교 NAME syntax → NP-03 error-bearing locale namespace/stamp → NP-04 use-def+many-origin mapping → NP-05 effect/alias summaries·guard/lease verifier → NP-06 특수 NAME/reflective semantics → NP-07 runtime/event/Graph/A3/실제 compiled route 차분. 각 NP-V 부정 테스트에 branch-only proof escape, loop namespace clobber, merged origin 유실, 살아 있는 noun alias overwrite, nested NameRef 의존성 누락을 연결한다. 단일 CPU 경로의 수용 전에 병렬 worker가 namespace를 재해석하게 만들지 않는다.

이번 변경은 **설계·감사 도구만 추가**했으며 Rust semantic 구현은 수정하지 않았다. Native Windows의 enqueuer 19 + modifier_scope 8 + parser_capture 41 + semantic 54 = **122 tests를 default/portable 각각 통과**, audit 분류 회귀 **6건 통과**. 조사한 C source pin과 DLL asset pin은 서로 다르므로 실제 oracle은 binary hash로 구별하며, pinned source에서 같은 DLL을 재빌드한 증거는 없다. 전체 upstream J suite·전체 reflection/locale suite·Miri·GPU·Linux·GitHub CI는 실행하지 않았다. NP 전체 수용 게이트는 계속 미완료다.

#### 3.7.6 Parser→compiler의 구체 전달 계약: 지연 noun과 concrete JEntity (2026-10-07)

**정정·결정:** parser reduction과 kernel execution은 같은 동작이 아니다. Compiler 경로의 기본 출력은 **아직 배열 연산을 실행하지 않은 semantic computation program**이다. noun-producing row 0–2를 줄인다는 것은 noun-result expression/reference를 stack에 넣는 것이며, 결과 POS가 Noun이라는 이유로 데이터를 계산하지 않는다. 별도의 unreduced AST와 계산된 결과를 항상 함께 보관하는 방식도 아니다. 아래 계약은 기존 단계들을 명확하게 한 것이며 새 mandatory IR pass를 추가하지 않는다.

**현재 코드의 두 realization:** `parser::runtime_noun`은 host가 없으면 `Expr::Monad/Dyad`를 그대로 남기지만 host가 있으면 `RuntimeParserHost::apply`로 계산하여 concrete noun을 반환한다. `parse_analysis`/`StaticAnalyzer`는 전자, `parse_runtime_host`와 현재 semantic-reference runtime은 후자다. 후자는 J 동작을 확인하는 실행 경로이지 compiler가 모든 배열을 먼저 계산해야 한다는 설계가 아니다. `ParseCapture -> J Graph`는 실행 관찰·검증 경로이며 **실행을 선행해야만 graph를 만들 수 있는 필수 compiler 경로가 아니다**. 분석 경로에는 `Program -> BoundProgram -> j_graph_ir::Plan::from_bound_with_graph_facts`가 이미 있다.

| 경계 | 다음 단계에 넘기는 실제/목표 형태 | 실행·저장 제한 및 미완료 |
|---|---|---|
| Word Formation/Enqueue → Parser | `EnqueuedWord` queue: spelling/payload/class, NAME lookup flags, original word/span | 이름의 concrete binding/POS·배열 계산을 lexical 단계에서 고정하지 않음 |
| Parser+Semantic Construction → J Graph builder | 현재 `semantic::Program`: root `Expr`, 공유 `FunctionEntity` DAG, assignment metadata, reductions와 name/constructor observations. 지연 noun은 `Expr::{ReadName,Monad,Dyad}` 상당; literal·이미 확보한 noun만 concrete `Value` | `JEntity::{Noun(Value),Function(Arc<...>)}`는 **concrete entity transport**. Deferred expression을 `JEntity::Noun`인 척 넘기지 않음. 일반 ordered writes·full NAME events·continuation 확장은 아직 미완료 |
| Binding/analysis → J Graph | `BoundProgram`의 읽기 의존성·pending write와 `j_graph_ir::Plan`: `ReadNoun`, `Apply(FunctionEntity, argument ValueIds)`, regions/source mapping와 graph facts | catalog NameVersion/shape는 분석 관찰·입력 계약 후보이지 실행 binding guard가 아님. `ValueId`는 계산 결과의 identity이며 버퍼·materialized noun이 아님 |
| J Graph → Execution Semantic Lowering/A3 | `logical_ir::Plan`: operations/SSA values, `ReadNoun`, `VerbReference`, Basis/SemanticCall, SemanticCheck, writes 및 effect/error order | 고수준 J parent/operand·region origin 연결을 유지. 현재 single-block/Return-only; 일반 CFG·namespace/guard·runtime bridge가 완성된 것은 아님 |
| A3 → native physical route 또는 검증된 external adapter | 검증한 computation region + 입력 binding/guard 계약, 이후 physical plan의 buffer/lease/readiness·target realization | native route의 schedule/representation/codegen은 downstream. external route에 native optimizer/scheduler를 강제하지 않음. 실제 CPU M4 연결과 fallback capability는 별도 수용 대상 |
| 실행 완료 → namespace/caller | concrete noun 또는 function 결과와 해당 semantic point의 assignment commit/observable effects | 이때 실제 결과가 생긴다. 지연이 문장 사이에서 관찰 가능한 대입·오류를 임의로 늦추는 lazy-language 변경을 뜻하지 않음 |

**고정 예: `b =: a + c * d`** — 이 예는 `a,c,d`가 Noun이라는 입력 계약/POS가 확보된 구간이다. 일반 이름의 POS를 모른 채 같은 parse graph를 무조건 확정하는 예가 아니다.

~~~text
Parser semantic output (not evaluated):
  assignment target = b
  root = Dyad(+, ReadName(a), Dyad(*, ReadName(c), ReadName(d)))
  callable identities = primitive +, primitive *
  source/reduction/name-use metadata = linked, not a second AST

J Graph (illustrative IDs, right-to-left construction):
  v0 = ReadNoun(d, eventD)
  v1 = ReadNoun(c, eventC)
  v2 = Apply(*, left=v1, right=v0)
  v3 = ReadNoun(a, eventA)
  v4 = Apply(+, left=v3, right=v2)
  pending WriteName(b, v4)

A3: resolved/guarded call semantics + checks + data/effect/error dependencies
Physical: legal fused kernel or separate kernels + actual buffer/lease decisions
Runtime: supply values at semantic read points, execute, commit b at its write point
~~~

이 그림의 `eventD` 등은 NP의 제안 provenance 연결을 나타내는 개념 표기이며, 현재 `ReadNoun{name,version}`에 event 필드가 구현됐다는 뜻이 아니다.

이 graph의 read/apply/write는 **실행할 의미**이지 compiler가 지금 수행한 사건이 아니다. 외부 noun input은 실행 시 해당 read의 immutable value를 공급하거나 동등성을 증명한 입력 bridge로 전달한다. 이미 read된 snapshot을 후속 rebind로 바꾸지 않으며, 아직 실행하지 않은 read를 compile-time noun capture로 바꾸지도 않는다. 분석기로 이미 read/compute된 결과를 넘겨 graph를 복원하는 capture 경로와 혼동하지 않는다. NAME witness와 semantic sequencing을 보강하는 NP-01~07은 계속 미완료다.

**파싱이 실제 값/POS에 의존하면:** literal rank operand 같은 이미 알려진 작은 값은 정상 constructor에 사용한다. 아직 계산하지 않은 값이 constructor result POS·후속 lookup·동적 source를 결정하면, **닫힌 prefix/region + 필요한 값/클래스 요청 + 정확한 parser continuation**을 semantic runtime/JIT 경계로 넘기는 것이 목표다. 이는 새로운 concrete Rust enum이 아니라 필요한 handoff 계약이다. 그 요청을 충족하기 위해 필요한 의존 부분을 실행하고, 원래 frame/locale·queue/stack·효과 상태에서 계속한다. 현재는 그 일반 continuation이 미구현이며 지원되지 않는 분석 경계를 명시한다. 가짜 noun/FunctionEntity로 파싱을 계속하거나 문장 전체를 다시 실행하지 않는다. 런타임 지원 경로가 있다는 이유로 분석 가능한 전체 구간을 항상 eager 실행하는 것을 compiler 기본 경로로 채택하지 않는다.

**최소 수용 작업:** 기존 NP-02/04/05/07 및 M3→M4 게이트 안에서 (a) analysis→Program→Graph→A3까지 kernel 호출/배열 allocation 없이 연결되는 예, (b) runtime input binding과 source origin의 대응, (c) 이미 계산한 concrete 값과 deferred noun의 오용 거부, (d) guarded POS/NAME 변경·동적 constructor에서 정확한 boundary 보고를 검증한다. 새 병행 AST 정본이나 경쟁 checklist는 만들지 않는다.

**이번 확인:** native Windows `static_explain 'data + data * data'`를 `data: Float[1000000000000]` metadata만으로 실행해 `ReadNoun` 3개·`Apply` 2개의 graph를 생성했다. 데이터 배열이나 해당 multiply/add 결과를 계산한 것이 아니다. 이것은 현재의 지연 graph 구성 증거이며 full runtime binding guard·allocation 계측·M4 end-to-end 검증은 아니다. 기존 execution-free semantic 회귀 2건은 default/portable 각각 통과했고 `git diff --check`도 통과했다. 이번에는 문서만 변경했으며 전체 suite·fmt/clippy·GitHub CI를 실행하지 않았다. 다음 구현 우선순위는 이 지연 경로를 실제 CPU 실행으로 닫는 것이며, eager capture를 compiler의 유일한 입력으로 만드는 것이 아니다.

#### 3.7.7 SemanticHandoff v1 논리 데이터 구조 확정 (2026-10-07; 논리 계약·첫 구현)

**현재 계획의 부족과 이번 결정:** 앞 절들은 정보·순서·경계 계약이었으며 구체 carrier가 구현되지는 않았다. 이제 parser→analyzer 전달 단위를 **`SemanticHandoff` 하나**로 정한다. `program`은 reduction으로 구성한 의미 결과, `context`는 reduction 전 입력·해석 맥락, `origins`는 둘의 연결이다. 아래는 **논리 계약을 표현한 목표 Rust schema**다. 첫 구현은 기존 `Program` + `frontend: Option<Arc<FrontendContext>>`에 대응시키며 실제 필드·검증 범위는 이 절 끝의 구현 표에 명시한다. arena로 Program 전체를 이미 교체했다는 뜻은 아니다. 변경은 NP-01/02/04의 단계적 이행으로 수행하고, 같은 stage에 두 의미 정본을 영구 유지하지 않는다.

**확정 범위:** 아래 필드와 참조 관계는 논리 계약이다. `Vec/Box/Arc`, 정수 ID 폭, arena 배치, 직렬화 형식은 구현 예시이며 고정하지 않는다. 의미 결과와 reduction 이전 맥락을 모두 보존하되, 원본 AST를 별도로 다시 구성하거나 중간 배열 값을 계산할 의무는 없다.

~~~rust
struct SemanticHandoff {
    schema: u32,                         // 1; readers reject unknown versions
    unit: FrontendUnitId,                 // owner of all local IDs
    program: Program,                     // sole semantic program
    context: Arc<FrontendContext>,
    origins: Vec<NodeOrigin>,
    status: HandoffStatus,                // Complete or Blocked(DemandId)
}

// Target shape of the existing Program, not an additional compiler IR stage.
struct Program {
    nodes: Vec<SemanticNode>,             // NodeId indexes this arena
    root: Option<NodeId>,
}
enum SemanticNode {
    Literal(Value),                       // source/required concrete noun only
    ReadNoun(NameUseId),                   // deferred semantic read
    FunctionUse(Arc<FunctionEntity>),      // shared completed J function DAG
    Monad { function: NodeId, argument: NodeId },
    Dyad { function: NodeId, left: NodeId, right: NodeId },
    WriteName { target: NameUseId, value: NodeId, event: EventId },
}

struct FrontendContext {
    sources: Vec<SourceRecord>,            // original and expansion sources
    words: Vec<WordRecord>,               // WordId = expanded enqueue index
    items: Vec<ItemRecord>,               // immutable parser occurrences
    reductions: Vec<ReductionRecord>,
    names: Vec<NameSyntax>,
    name_uses: Vec<NameUseRecord>,
    observations: Vec<NameObservation>,
    events: Vec<SemanticEvent>,
    demands: Vec<BoundaryDemand>,
    function_origins: Vec<FunctionOperandOrigin>,
}
struct ItemRecord {
    producer: ItemProducer,               // Word | NameUse | Reduction | Mark
    class: ParseClass,
    semantic: Option<NodeId>,             // None for syntax/control items
    word_range: Range<usize>,
    blame_word: Option<WordId>,
}
struct ReductionRecord {
    row: ParseRow,
    window: [Option<ItemId>; 4],          // inspected stack window before action
    consumed: Box<[ItemId]>,
    produced: ItemId,
    name_uses: Box<[NameUseId]>,
    events: Box<[EventId]>,
}
struct NameUseRecord {
    syntax: NameSyntaxId,
    input_item: ItemId,
    phase: NameUsePhase,
    expected_pos: Option<JPos>,
    policy: NamePolicy,
    event: EventId,
    observation: Option<ObservationId>,
}
struct NodeOrigin {
    node: NodeId,
    items: Box<[ItemId]>,
    reductions: Box<[ReductionId]>,
    name_uses: Box<[NameUseId]>,
}
~~~

**보조 타입도 v1 논리 계약에 포함한다.** 아래 필드와 variant로 연결 관계를 명시한다.

| 타입 | v1 필드/variant와 의미 |
|---|---|
| ID들 | `FrontendUnitId`는 비재사용 unit identity. `SourceId/NodeId/WordId/ItemId/ReductionId/NameSyntaxId/NameUseId/ObservationId/EventId/DemandId`는 서로 구분되는 local identity. 모든 외부 link는 `(unit, local_id)`. Expr의 이동 위치·Arc 주소·source spelling을 ID로 사용하지 않음. ID 폭과 저장 방식은 구현 선택 |
| `SourceRecord`, `SourceSpan` | source는 `text`, `parent: Option<SourceSpan>`을 소유. 최초 source의 parent는 None, DD 등 생성 source는 생성 원인 span을 가리킴. 모든 source span은 `source: SourceId`와 `bytes: Range<usize>`로 구성하여 원문/확장문을 구분 |
| `WordRecord` | `span: SourceSpan`, `class: EnqueueClass`, `flags: EnqueueFlags`, `environment: EnqueueEnvironment`, `expansion_origin: Option<WordId>`; spelling은 해당 immutable source로 읽음. DD 확장에서 같은 span인 word도 다른 WordId. 확장문은 별도 SourceRecord와 parent mapping 유지 |
| `ItemProducer` | `Word(WordId)`, `NameUse(NameUseId)`, `Reduction(ReductionId)`, `Mark`; NAME lookup으로 품사가 바뀌면 이전 ItemRecord를 수정하지 않고 새 item 생성. 실제 reduction window의 context item과 consumed item을 구분 |
| `NameSyntax` | `word: WordId`, `address: Simple/Direct/Indirect`, `components: Box<[NameComponent]>`, `read_mode: Ordinary/ByValue/ByValueAndAbandon`, `special_operand: Option<ImplicitOperand>`; component는 `kind: SimplePart/NamedLocale/NumericLocale/HolderName/DebugFrame` + 원 source span. direct/indirect와 read_mode는 직교. 실제 locale/frame identity는 여기에 넣지 않음 |
| `JPos`, `NameUsePhase`, `NamePolicy` | POS=`Noun/Verb/Adverb/Conjunction`. Phase=`StackRead/StackPOS/ConstructorRead/CallLookup/AssignmentTarget/Abandon`. Policy=`CaptureAtRead/LateAtCall/ResolveAtConstruction/CaptureAtConstruction/Assignment/ReadAndAbandon`. `expected_pos=None`는 미확정 요구이고 Noun 가정이 아님. 하나의 NAME에 stack POS와 call lookup 등 여러 use 존재. 아래 첫 구현의 stack policy는 앞의 세 가지에 한정 |
| `NameObservation` | `origin: Catalog/Runtime`, `outcome: Noun/Function/Missing/JError`, `pos: Option<JPos>`, `lookup: Option<LookupWitness>`, constructor-fixed ranks/POS 등 작은 metadata. 실제 noun은 필요한 semantic Literal/constructor operand가 소유하며 observation에 payload를 복제하지 않음. Catalog/Runtime 어느 관찰도 자동 proof가 아님 |
| `LookupWitness` | `engine`, `frame_instance`, `start_locale`, `found_locale`, `execution_locale`, `binding_stamp`, `dependencies`. stamp=`scope_instance/symbol_generation/revision`; dependencies=`LocalPresent/LocalAbsent/LocalePathEpoch/HolderRead/NamespaceEpoch` typed records. 미확보 witness는 None으로 드러내고 flat NameVersion으로 채우지 않음. 실제 runtime 관찰과 future GuardRecipe는 다른 타입/소유자 |
| `SemanticEvent` | `kind: NameUse/Constructor/Call/Write/Check`, `owner: ItemId`, `node: Option<NodeId>`, `after: Box<[EventId]>`, `effect: NamespaceRead/NamespaceWrite/ArrayCall/UnknownCall/Constructor`, `realization: Planned/Observed`. compile-time 계획과 runtime 실행 사건 구분; constructor·call의 potential error 순서도 edge로 유지. 기본 보수적 순서 이후 완화는 별도 proof 필요 |
| `BoundaryDemand` | `kind: NeedNameClass/NeedConstructorValue/NeedConstructorResult/NeedNamespaceAction/NeedDynamicSemantics/NeedCallableProof`, `at_item: ItemId`, `row: Option<ParseRow>`, `required_nodes: Box<[NodeId]>`, `after: Box<[EventId]>`, `reason: typed reason`; v0에서 runtime continuation을 갖춘 척하지 않음 |
| `HandoffStatus` | `Complete`는 semantic construction complete일 뿐 execution admitted가 아님. `Blocked(DemandId)`는 prefix를 담은 inspection artifact이며 complete graph/실행 계획으로 낮추지 않음. actual JError는 별도 진단 결과; demand/Unknown과 혼합하지 않음 |
| `FunctionOperandOrigin` | `owner: NodeId`, `operand_path: [operand index]`, `items: [ItemId]`, `name_uses: [NameUseId]`; shared FunctionEntity DAG 안의 특정 operand 사용을 구별. 같은 함수 identity를 공유해도 owner/path별 NAME 맥락은 합치지 않음. 빈 path는 함수 root 사용 |

**대입·그룹·미완성 reduction:** `WriteName`은 문장 끝뿐 아니라 표현식 내부에도 존재하며 assigned entity를 결과로 산출한다. event가 namespace write의 의미 시점을 보존한다. 괄호는 의미 중립이면 새 semantic node를 강제하지 않지만 Item/Reduction/origin에는 남긴다. ReductionRecord는 완료된 reduction만 기록한다. 실패/중단 직전 window는 BoundaryDemand의 `window: [Option<ItemId>; 4]`에 보존하며, Blocked 상태에서 완성된 결과를 꾸며내지 않는다.

**Noun capture의 의미:** `CaptureAtRead`는 J의 의미적 read 시점에 binding/value identity를 고정한다는 계약이지 compiler에서 배열 내용을 즉시 계산하라는 지시가 아니다. 읽은 noun이 deferred computation의 결과이면 그 값의 semantic identity와 순서 의존성을 보존한다. 이후 rebinding으로 그 read의 대상을 바꾸지 않는다. 함수의 `LateAtCall`은 호출 시점 lookup이며 둘을 같은 name 문자열만으로 합치지 않는다.

**NodeId와 함수 identity:** arena는 기존 `Expr` tree의 저장 방식을 점진적으로 대체하는 것이며 함수 DAG를 복제하지 않는다. 같은 `Arc<FunctionEntity>`를 두 번 사용하는 경우 `FunctionUse` NodeId와 parser ItemId는 각각 별개다. 함수의 intrinsic operand identity에 call-site NAME/guard/target 정보를 넣지 않는다. Noun 연산 결과는 Monad/Dyad node이지 `JEntity::Noun`의 가짜 concrete Value가 아니다. NodeOrigin은 many-to-many여서 parentheses/NAME 치환처럼 여러 item이 같은 결과를 지시하는 경우도 표현한다.

**고정 연결 예(`a+c*d`, 각 입력 Noun POS 계약):** words는 W0=a,W1=+,W2=c,W3=*,W4=d. queue-origin NAME item과 stack-read 후 Noun item은 다른 ItemId이고 NameUse로 연결한다. R0은 `DyadNVN`, consumed=`[c의 noun item, * item, d의 noun item]`, produced=P0(Noun, semantic=Nmul). R1은 consumed=`[a의 noun item,+ item,P0]`, produced=P1(Noun, semantic=Nadd). Nmul origin은 R0/관련 NAME uses, Nadd origin은 R1/관련 NAME uses를 가리킨다. **span이 같다는 이유로 항목을 연결하거나 reduction 이전 맥락을 덮어쓰지 않는다.**

**Downstream 계약:** Graph/A3는 `(FrontendUnitId, NodeId)`를 origin key로 사용한다. 각 adapter가 target node → **원 semantic NodeId 집합**의 mapping을 산출하고 같은 `Arc<FrontendContext>`를 공유한다. Fusion/CSE는 origin 합집합, inlining은 caller/callee unit mapping을 유지한다. NodeOrigin과 NAME use를 원 spelling으로 다시 추측하지 않는다. Layout/BufferId·schedule·GPU placement·CostProfile은 이 schema에 들어가지 않는다. 변환 후 필요한 semantic program은 검증용으로 참조/보존하되 원 AST·runtime 중간 배열 전체를 복제하지 않는다.

**v1 verifier와 이행 계획(기존 NP/DB 게이트):**

1. **NP-01/04: 타입·저장소·verifier를 먼저 구현.** ID 범위/unit, producer/output 역참조, 이전 입력 occurrence 보존, 원 words coverage와 실제 matcher window/row, source/expansion span, callable/noun edge 종류, roots/origins coverage, event edge cycle, unresolved demand와 Complete의 모순을 검사한다. fabricated producer/node link 하나만 바꿔도 거부하는 부정 테스트 필수. 이 검사는 J 의미의 독립 증명은 아님.
2. **NP-02/04: parser emission.** 같은 matcher/action에서 Word→Item→NameUse→Reduction→SemanticNode를 함께 기록한다. 기존 Program/ParseReduction/NameUse/ParseCapture 소비자는 migration adapter를 거쳐 비교한다. Capture는 runtime 검증용으로 유지하고 compiler context를 만들기 위해 eager 실행하지 않는다. Arena 전환 시 기존 Expr를 두 번째 정본으로 남기지 않는다.
3. **NP-03/05: 실제 lookup API와 witness 연결.** catalog 관찰과 runtime witness를 구별하고 error-bearing locale lookup을 연결. 없는 locale/path/frame 증거를 가짜 값으로 채우지 않는다. concrete lookup support 미완료인 form은 demand/coverage 경계로 유지한다.
4. **NP-04/05: Graph/A3 origin adapter.** parent/operand/function-use와 각 name use가 다음 단계까지 이어지는 golden 및 fold/CSE origin 유실 부정 테스트. admission verifier 미완료 시 Complete handoff만으로 NAME 특수화를 실행하지 않는다.
5. **NP-06/07: C differential.** noun/verb/modifier·rank/gerund/fork·assignment/locative의 POS/값/오류/namespace 결과와 Rust handoff/context projections 비교. C에서 관찰할 수 없는 내부 queue/lookup는 source-derived fixture 또는 별도 계측 증거로 표시하며 값 비교 통과를 내부 trace 동등성으로 승격하지 않는다.

**최우선 구현 slice — parser 결과와 NAME scope (2026-10-07):** 논리 handoff의 실제 carrier는 기존 `semantic::Program`이다. 그 `expression/assignment/FunctionEntity`가 유일한 의미 정본이고, `frontend_context::FrontendContext`는 동일 parser action에서 만든 immutable 구조·출처 index다. index의 `Literal`에는 배열 payload가 없으며 독립 executor가 아니다. `Expr.origin`이 두 구조를 직접 연결한다. source span으로 결과를 재구성하거나 두 번째 parsing을 수행하지 않는다.

~~~text
Program
  expression: Expr { origin: NodeId?, kind: J semantic expression }
  assignment / constructor snapshots / shared FunctionEntity DAG
  frontend: Arc<FrontendContext>?

FrontendContext
  schema=3, unit: FrontendUnitId, realization: Deferred | Observed
  source, source_origin: SourceOrigin?, words: WordRecord[]
  items: ItemRecord[], nodes: NodeRecord[], origins: NodeOrigin[]
  name_uses: NameUseRecord[], reductions: ReductionRecord[]
  steps: Stack | FrontMark | Reduce, pending: PendingAction?, root: ItemId?
  complete: bool
~~~

| 확인할 조건 | 실제 구현·검증과 제한 |
|---|---|
| reduction 전 맥락과 결과 모두 보존 | WordId/ItemId/NodeId/ReductionId/NameUseId를 구분. NAME 치환은 새 item. reduction은 실제 4-slot window, consumed IDs, produced ID를 기록. 없는 suffix slot은 virtual Mark로 해석. parentheses는 같은 semantic node에 여러 item/reduction origin을 연결 |
| parsing 중 사용 가능 | 현재 matcher/action에서 기록하며 `FrontendContext::verify`가 right-to-left queue→stack과 FRONT MARK 시점, row 선택, 입력·출력·node 연결을 replay. `parse_frontend` 실패는 `FrontendFailure {error, context}`로 성공 prefix와 pending row/window 보존. 성공하지 않은 action의 가짜 output 없음. 이는 runtime continuation이 아님 |
| noun·verb·modifier NAME 시점 | `NameResolution=NounValue/FunctionValue/FunctionReference`, `NamePolicy=CaptureAtRead/LateAtCall/ResolveAtConstruction`. 실제 resolve branch가 정책을 기록. `u`가 또 다른 NameRef 값을 치환한 경우도 FunctionValue이며 u 자체를 late lookup으로 바꾸지 않음. CatalogClass/RuntimeClass/DiagnosticAssumption 및 optional binding version/lookup 관찰 구분. 진단용 noun 가정은 실행 POS proof가 아님 |
| local/global 실제 관찰 | `LookupObservation {engine, frame, search, local_state, found, binding_version, binding_generation, binding_class}`. Search=GlobalOnly/CurrentFrameThenGlobal; local_state=NoFrame/Bound/DeclaredUnbound/Absent; found=Local(scope)/Global(scope)/Extension/Missing. Engine/global namespace와 invocation frame은 비재사용 ScopeInstanceId. 같은 spelling·version=1이어도 다른 frame의 binding을 합치지 않음 |
| explicit/direct definition local name | `DefinitionCode.name_plan`의 monad/dyad별 `DefinitionScopePlan`에 local 선언 목록, 원 sentence/word/span과 ReadCurrentFrameThenGlobal/LocalAssignmentTarget/GlobalAssignmentTarget, dynamic assignment target 위치를 보존. 생성 시 body lookup/POS/값 계산 없음. actual invocation은 이 local 선언 목록을 사용하며 u/v/m/n/x/y는 invocation operand/argument로 따로 공급 |
| 선언과 실제 binding 구분 | `t=.t+u`에서 미초기화 local t의 RHS는 global fallback 가능하고 대입 뒤 읽기는 local. 선언은 read를 local SSA로 고정하는 증거가 아님. ordinary local function NAME은 late reference를 보존하며 반환 후 caller frame을 closure처럼 캡처하지 않음. implicit operand의 function value 치환과 구별 |
| 후속 최적화 입력 | J Graph의 각 node에 `parser_origins: [NodeId]`, 같은 context Arc 유지. A3 `parser_provenance`는 기존 operation.j_origin→Graph ID→parser NodeId 체인을 보존. derived-function 확장의 생성 node는 보수적인 enclosing call origins를 유지하므로 가장 작은 source occurrence의 유일 매핑으로 주장하지 않음. Context/Graph/A3 verifier는 누락·범위 오류·source 불일치를 거부 |
| 메모리·실행 경계 | analysis는 context를 항상 보존, runtime은 capture를 요청한 경우에 기록. 일반 runtime body는 추적 비용을 강제하지 않음. 함수는 기존 Arc 공유, 중간 배열을 context에 복제하지 않음. gerund singleton boxed header는 반복적으로 벗겨 기존 depth/rank/empty 순서와 limit를 유지하면서 Windows 재귀 stack 증가를 방지 |

**NAME 관련 최적화 금지 규칙:** `DefinitionScopePlan`은 source inventory이지 닫힌 local 집합·현재 POS·값 proof가 아니다. computed assignment, execute, 제어 흐름이 만들 수 있는 추가 이름을 배제하지 않는다. local 부재·global fallback을 lookup 삭제/hoist에 활용하려면 frame liveness, symbol generation, namespace/local epoch, locale/path 및 의미적 read 시점까지 별도 witness/guard가 필요하다. 현재 `LookupObservation`이나 flat binding version만으로 그 최적화를 허용하지 않는다. actual C name lookup의 buckets/lookaside는 저장 방식이며 RustJ가 복제할 계약은 lookup 결과·시점·fallback이다.

**수용 체크리스트:**

- [x] 논리 결과/입력/NAME ID 관계와 소유 범위 확정; context schema/unit 검증.
- [x] 기존 9개 action에서 emission; action 전 window와 완료 result 연결; failure prefix와 pending action.
- [x] source-derived C `p.c::cases` literal mask 표와 9^4=6,561개 class window의 first-row 선택 비교. C 내부 실행 trace를 계측한 검증과 구별.
- [x] 같은 구조의 queue/stack replay, 잘못된 producer/operand/origin·scope 관찰 부정 테스트.
- [x] Graph/A3까지 출처 전달 및 origin 유실 거부; metadata만 있는 10^12-element noun의 deferred 분석.
- [x] explicit/direct local declaration·global fallback·shadowing·global write·반환된 ordinary function NAME·implicit operand 치환 회귀.
- [x] 지원 중인 simple NAME의 runtime observation에 재사용 없는 BindingGeneration/POS 추가; 실제 NAME use ID에서 SimpleNameGuard 생성, Engine에서 검색 전체 재검사 및 typed invalidation 결과 제공.
- [ ] 완전한 LookupWitness/GuardRecipe, constructor별 모든 NAME use의 독립 event ID, function operand path별 provenance, multi-unit inlining origin merge.
- [ ] source expansion entry 전체와 typed BoundaryDemand/정확한 runtime continuation, Program 전체 arena 저장 전환. 현 실패 context를 실행 가능한 continuation으로 사용하지 않음.

이 slice는 **지원 중인 parsing action과 compilation 경로의 구조 보존**을 닫는다. full-J/locale/control executor 또는 NAME 특수화 실행 적법성 완료를 의미하지 않는다. 상세 실행 검증 결과는 아래에 기록한다.

**Simple NAME guard 첫 구현:** `SimpleNameGuard::from_name_use(context, NameUseId)`는 검증된 완료 context의 runtime bound simple NAME만 받으며 `(FrontendUnitId, NameUseId)`를 보존한다. catalog/diagnostic 가정, missing/extension, locative/by-value form은 이 recipe의 수용 범위 밖이다. `Engine::check_name_guard`는 원래 engine·현재 frame 정체성을 먼저 확인하고 current-frame→global→기본 z noun 검색을 다시 수행하여 local bound/declared-unbound/absent 상태, found scope, binding version·generation·POS를 대조한다. 결과는 `ValidAtCheck/EngineChanged/FrameChanged/LookupChanged`이며 J 오류나 자동 fallback이 아니다. 성공한 할당마다 `BindingGeneration`을 새로 발급하여 삭제·재생성 후 version/POS가 같아지는 ABA도 거부한다. 현재 expunge 언어 지원을 추가한 것은 아니며 ABA 회귀는 내부 table 삭제로 재현한다.

이 recipe는 검색을 생략하는 epoch fast path가 아니다. 같은 이름의 재할당과 fallback을 가리는 local 생성, invocation 변경은 무효화하고 관계없는 binding 변경은 허용한다. 실패한 RHS는 기존 guard를 유지한다. noun snapshot을 새 값으로 갱신하거나 ordinary verb의 late lookup을 고정하지 않는다. `ValidAtCheck` 이후 effect 또는 다른 mutable engine 접근을 넘는 유효성, locale/path, concurrent lease, compiled dispatcher와 effect 이전 route admission은 여전히 미구현이다. 따라서 full LookupWitness/GuardRecipe 체크리스트는 열린 상태로 유지한다.

`g=:f` 뒤 f만 변경하면 g의 guard는 여전히 `ValidAtCheck`일 수 있지만 g의 실행 결과는 달라진다. 이는 의도한 직접 binding 검증이며 전이적 function DAG의 실행 동일성을 증명하지 않는다. 후속 특수화는 고정하려는 각 late read의 별도 guard와 의미적 시점·효과 유효성 근거를 요구한다.

**전이적 alias-call guard와 단일 순수 호출 lease (2026-10-07):** `name_guards::AliasCallGuard`는 runtime context의 ordinary `LateAtCall` NAME에서 시작해 **단순 별칭 체인 → operand 없는 `+/-/*/%` Verb**만 수용한다. `Engine::prepare_alias_call_guard(&Arc<FrontendContext>, NameUseId)`는 기존 root 관찰의 유효성을 먼저 확인하고 각 alias의 spelling, 실제 LookupObservation, shared immutable target Arc를 `AliasRead[]`에 보존한다. `h→g→f`에서 f만 바뀌어도 read index 2로 무효화를 보고한다. literal noun snapshot, constructor-time modifier, implicit function-value substitution을 ordinary call-time 조회로 바꾸지 않는다. derived operands·hook/fork·rank/reduce·definition body를 미리 순회하거나 실행하지 않는다. 미정의/POS 변경/순환/깊이 한계는 typed admission 결과이며 원래 J 오류보다 먼저 자동 노출하는 dispatcher는 없다.

원 `FrontendContext` Arc와 `(unit, NameUseId)`를 guard가 공유하여 수명을 보장한다. `AliasCallGuard::verify`는 원 use와 root observation 연결, chain next-name/target, terminal primitive, operand 부재, generation 중복·누락·빈/과대 chain을 검사한다. `Engine::check_alias_call_guard`는 실제 namespace/frame과 모든 read를 재검사한다. 이 guard가 보존하는 것은 derived runtime evidence이며 별도 executable function IR이 아니다.

입력은 의미적 순서로 먼저 계산하고 **호출 직전** `validate_alias_call_guard`를 실행한다. 성공 시 `ValidatedAliasTarget<'engine>`가 Engine과 guard를 불변으로 빌리며, token을 소비하는 `apply_monad/apply_dyad`가 기존 산술 kernel을 한 번 실행한다. 살아 있는 token과 Engine mutation을 함께 사용하는 코드는 compile-fail doctest로 거부함을 확인한다. 이 slice는 기존 parser/runtime/Logical executor의 자동 경로 선택을 바꾸지 않으며 새 CLI 실행 모드도 아니다. raw kernel 오류를 반환하는 명시적 호출 API다. 실패 시 kernel을 실행하거나 문장/인자 효과를 replay하지 않는다. caller는 이미 보관한 noun으로 지원되는 Rust 실행을 명시적으로 선택할 수 있다. 전체 region admission, CFG continuation, arbitrary effectful function·locale/path·concurrent Engine lease는 계속 미구현이다.

`ParseCapture.frontend`는 같은 parser가 만든 `Program.frontend` Arc를 공개하고, 실패 시 완료된 prefix를 보존한다. 두 번째 parsing이나 event→context 재구성을 수행하지 않는다. 실패 context는 여전히 실행 가능한 continuation이 아니며 guard admission은 완료 context만 받는다. explicit invocation frame에서 global fallback이 local 생성으로 가려지거나 frame이 끝나는 경우의 전이적 guard 회귀도 포함한다.

- [x] ordinary alias chain의 전이적 조회·원 parser context 소유·typed invalidation 및 verifier.
- [x] 지원 산술의 단일 call-time borrow lease, 실제 kernel 실행, effect 재실행 금지 회귀.
- [x] 공개 capture의 성공/실패 context 보존, 순환·깊이·잘못된 recipe·local fallback/frame 종료 경계 회귀.
- [x] 계산된 인자를 move로 소유하는 AliasCall 및 typed miss; 동일 engine/frame의 명시적 단일 semantic call 이어 실행. 다른 engine/frame·invalid recipe에서는 소유권 반환.
- [ ] derived function operand별 의미적 lookup 시점·origin path와 일반 effect/region admission. 현 alias chain을 이 전체 영역의 proof로 사용하지 않음.

**계산 완료된 단일 호출 경계:** `AliasCall { guard: Arc<AliasCallGuard>, x: Option<Value>, y: Value }`는 원 parser context의 callee와 이미 계산한 논리 값들을 유지한다. `try_alias_call(&self, call)`은 `Executed(Result<Value>)` 또는 `Miss(AliasCallMiss { check, call })`를 반환한다. kernel의 length/domain 오류는 Executed의 J 오류이며 guard miss로 바꾸지 않는다. miss는 인자를 그대로 반환하고 함수나 인자를 실행하지 않는다. caller가 `resume_alias_call(&mut self, miss)`를 명시적으로 호출하면 현재 engine/frame을 다시 확인하고 **원 NameRef Arc + 저장한 x/y**를 기존 `call_entity`로 전달한다. 현재 binding의 POS/값 변경은 원 Rust 의미 경로가 Domain/Value 등으로 처리한다. 원 callee는 source span에서 재구성하지 않는다.

이어 실행 전에 engine/frame이 다르거나 recipe가 손상되면 `Miss`로 인자 소유권을 다시 돌려주며 실행하지 않는다. RHS/인자 계산을 다시 파싱하거나 namespace에서 noun을 재조회하지 않는다. 따라서 인자 계산 중 count 증가·함수 재대입이 있어도 count는 한 번만 증가하고 이미 읽은 noun은 이후 재대입에 영향받지 않는다. 이는 **이미 인자를 계산한 ordinary call 지점**에 한정한다. pending parser/constructor, control block, 문장 중간의 임의 위치를 재개하는 continuation 또는 자동 compiled dispatcher를 구현한 것이 아니다. lease API는 raw kernel 오류를, semantic resume는 기존 runtime 진단을 반환하며 두 경로의 전체 human diagnostic 동등성은 아직 검증하지 않았다.

**Alias/call-ready slice 실행 검증 (2026-10-07, Windows native):** default/portable 각각 **578 passed / 17 ignored / 0 failed**. library 75개, `tests/name_guards.rs` 10개 및 살아 있는 lease와 Engine mutation의 충돌을 거부하는 compile-fail doctest를 포함한다. fmt/clippy 통과. Python alias audit 3개·NAME audit 8개·oracle protocol 9개 통과. `examples/alias_guard_probe.rs`가 실제 lease kernel 호출과 miss 후 저장한 인자로 `resume_alias_call`을 실행한다. `tools/alias_guard_audit.py`의 27 fixture × C j64/AVX2 = **54 observation 모두 일치** (`reports/alias-call-guard-windows.json`). 배열/빈 배열/scalar 확장/length 오류, 인자 효과 1회, target noun·미정의 NAME의 Domain/Value를 포함한다. guard 상태는 Rust 자체 assertion이며 C에서 계측한 event가 아니다. C oracle 출력 dtype 범위 내 유한 비교이고 전체 numeric category/성능 증명도 아니다. 기존 scope 비교 **40 일치**, NAME **64 일치 / 40 미지원**, boundary **36 일치 / 4 미지원**도 재실행했다. 전체 diagnostic, GPU/Linux/remote CI 검증은 수행하지 않았다. 아래 guard 565건/parser 560건 기록은 이전 slice 결과다.

**Guard slice 실행 검증 (2026-10-07, Windows native):** default/portable 각각 **565 passed / 17 ignored / 0 failed** (library 73개, 신규 guard 회귀 5개 포함). 별칭의 전이적 late target에 대한 assertion 추가 후 관련 library scope 회귀 7개를 두 설정에서 재실행해 통과했다. fmt/clippy 통과; Python audit 8개·oracle protocol 9개 통과. C j64/AVX2 × 두 Rust 경로 재비교는 scope **40/40 일치**, 기존 NAME **64 일치 / 40 미지원**, semantic boundary **36 일치 / 4 미지원**이며 의미 불일치는 없다. C 값/오류 비교는 Rust guard 내부 적법성의 독립 증명으로 간주하지 않는다. 아래 560건 검증 기록은 앞선 parser-context slice의 결과다.

**실행 검증 (2026-10-07, Windows native):** default와 portable 각각 **560 passed / 17 ignored / 0 failed**. 신규 frontend 통합 회귀 15개와 library 전체 68개를 포함한다. `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings` 통과. Python audit 분류·전달 회귀 8개와 oracle protocol 회귀 9개 통과. C j64/AVX2 × Rust direct/semantic-reference 비교에서 explicit/direct NAME scope 10 fixture의 **40 observation 모두 일치** (`reports/name-scope-handoff-windows.json`); 기존 NAME 26 fixture는 **64 일치 / 40 미지원**, semantic boundary 10 fixture는 **36 일치 / 4 미지원**. 각 observation은 setup/error 후 read를 포함하는 문장 sequence이며 C 내부 event trace나 compiled route 검증이 아니다. 여러 줄 explicit block setup만 C script 입력으로 전달하고 양쪽 원문은 동일하게 보존한다. 보고서는 binary/DLL hash와 source/asset revision 차이를 기록한다. 미지원 locale·execute 등은 통과로 계산하지 않는다. GPU/Linux/remote CI 검증은 실행하지 않았다.


---

[← 이전 부분](./04-semantic-array-ir-p02.ko.md) | [다음 부분 →](./04-semantic-array-ir-p04.ko.md)
