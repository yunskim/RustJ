[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)

> 문서 분할 안내: 기존 `PROJECT.ko.md`의 내용을 이동한 모듈입니다. 원문의 단일 파일 정본·신규 Markdown 금지 문구는 최신 [Agent 문서 규약](../../../AGENTS.md)보다 우선하지 않습니다.

## 15. 문서 정책

### 15.1 권위 문서

앞으로 사람이 유지하는 프로젝트 기준 문서는 **이 `PROJECT.ko.md`가 정본이며 `PROJECT.md`는 같은 설계의 영어 mirror**다. Syntax 힌트 검토와 프레임워크 비교도 §4.1.2–3에 통합했으며 별도 설계 보고서를 유지하지 않는다. 새 설계·구조 결정은 한국어 정본을 먼저 갱신하고 영어 mirror를 같은 변경에서 갱신한다.

정본 유지 규칙:

- 전체 구현 상태와 코드 검토 기준은 §12에서 요약하고, 완료/미완료 관문은 §10의 해당 stage 체크리스트에서 관리한다. 다른 절은 고유 계약만 설명하고 이 정본 위치를 참조한다.
- syntax/graph 연구는 §4.1, 확장 primitive inventory는 §4.6, cell 의미의 proof 표본은 §4.11.4, 공통 검증 정책은 §11에 둔다.
- 역사 감사와 과거 설계는 날짜/검토 기준을 명시한다. 이전 v0.1 또는 transition 단계의 상태를 현재 구현으로 서술하지 않는다.
- 절을 이동하면 한·영 heading/상호 참조를 함께 갱신하고, 고정 출처·후보/채택/구현 상태·고유 검증 조건을 보존한다.


변경 시 함께 갱신할 항목:

- 아키텍처 결정
- interface/boundary
- 구현 계획
- 체크리스트
- current status
- 지원 범위
- validation policy
- 주요 검증 결과

### 15.2 README

`README.md`는 다음만 담당한다.

- 프로젝트 한 줄 설명
- 빌드/실행 방법
- 현재 지원 범위의 짧은 요약
- `PROJECT.ko.md` 링크

README에 별도의 상세 설계 사본을 만들지 않는다.

### 15.3 reports/

`reports/`는 앞으로 **기계 생성 또는 실측 원자료** 중심으로 사용한다.

허용 예:

- benchmark JSON
- allocation JSONL
- conformance result
- measurement raw data

새로운 사람이 편집하는 설계/체크리스트/진행 Markdown 보고서는 만들지 않는다.

### 15.4 이번 통합에서 흡수한 기존 문서군

다음 범주의 기존 Markdown은 이 문서로 통합하고 별도 파일을 유지하지 않는다.

아키텍처·설계:
- COMPILER-ARCHITECTURE
- JAXA-REVIEW
- MEMORY-POLICY
- COMPILER-HINTS-J-SYNTAX — §4.1.2로 통합
- J-GRAPH-FRAMEWORK-OPTIMIZATION-REVIEW — §4.1.3로 통합
- ARRAY-FRAMEWORK-DESIGN-REVIEW
- GPU-ARRAY-DESIGN-AUDIT
- RUST-ARRAY-REFERENCES

계획·진행:
- IMPLEMENTATION-PLAN
- IMPLEMENTATION-CHECKLIST
- GPU-FRIENDLY-ARRAY-PLAN
- COMPILER-IR-PROGRESS
- C1-PROGRESS
- DEFINITION-IMPLEMENTATION-PLAN

언어·기능·감사:
- C-J-FEATURE-AUDIT
- LEXER-AUDIT
- NAME-SEMANTICS-AUDIT
- DEFINITION-PARSING-AUDIT
- ARRAY-VERBS
- INDEX-VERBS
- DYADIC-RANK
- SCALAR-TYPES
- SPARSE-ARRAYS
- BIT-STORAGE

검증·이력:
- VALIDATION-STRATEGY
- MILESTONE-1
- MILESTONE-2
- M2-POOL
- M2-REVIEW
- PERFORMANCE
- PHYSICAL-ARRAY-G1

세부 역사 원문은 Git history에서 계속 조회할 수 있고, 측정 원자료는 `reports/`에 남긴다.
### 15.5 외부 역사 저장소

다음 저장소의 설계 내용은 2026-09-30 기준으로 이 문서에 흡수했다.

- `yunskim/JAXA`
- `yunskim/JAXA-complier`
- `yunskim/japchae`
- `yunskim/jaxa-analyzer`

앞으로 새 설계 결정을 이 네 저장소 중 하나에 먼저 기록하고 나중에 RustJ로 옮기는 workflow를 사용하지 않는다. **RustJ `PROJECT.ko.md`가 최초 기록 장소이자 최종 권위 문서**다.

기존 저장소는 prototype 코드, 연구 이력, 참고 구현을 확인할 때만 사용한다.

### 15.5.1 current jsource semantic cross-check (2026-09-30)

기준 revision: `jsoftware/jsource@13994ffa1ed5f06f79fad6e9822a7ed2d29b1528` (2026-09-30 current master at this review).

구현 기법을 그대로 복제할 필요는 없지만 다음 source-level semantic facts는 RustJ 설계 제약으로 채택한다.

| jsource 확인점 | RustJ 설계 결론 |
|---|---|
| `w.c` enqueue는 alphabetic word를 ordinary NAME으로 만들고 lookup flag/hint를 붙인다 | extension도 enqueue에서 ADV/VERB로 고정하지 않는다 |
| `p.c` parser가 NAME을 stack할 때 local/locale lookup하고 noun은 value, 일반 ACV는 nameref로 처리한다 | noun snapshot과 function-name late binding을 분리한다 |
| `sc.c` nameref 실행은 현재 lookup value의 part of speech가 reference 생성 시 기대한 품사와 같은지 검사한다 | `NameRef.expected_part_of_speech`를 보존하고 mismatch는 domain error로 처리한다 |
| `p.c`는 parse reduction 중 name lookup/verb execution/assignment를 수행한다 | 문장 전체 name snapshot을 만들지 않고 J의 우측→좌측 observable sequencing을 effect/name dependency로 보존한다 |
| `j.h::FORK1/FORK2`는 일반 fork에서 right tine을 먼저 실행하고 이후 left tine, middle verb 순으로 실행한다 | train value graph를 자동 병렬 독립으로 보지 않고 effect/name/error order edge를 proof 전까지 보존한다 |
| `wc.c`/`cx.c`는 `try./catch./catchd./catcht./throw.`를 linked control flow로 실행하고 error/throw를 handler로 전달한다 | J-visible errors를 항상 fatal diagnostic으로 취급하지 않고 exceptional CFG/control effect로 보존한다 |
| `c.c::ad12`의 `u::v`는 ordinary failure에서 fallback `v`를 실행/반환하지만 throw/exit는 전파한다 | adverse derived verb를 expression-level error-handler semantics로 보존한다 |
| `c.c::jtobverse`와 inverse logic은 `u:.v`의 second operand를 inverse semantics에 사용한다 | forward dataflow가 같아 보여도 obverse metadata/operand를 dead-code로 제거하지 않는다 |
| `cf.c`의 bident/trident tables는 noun/verb/adverb/conjunction 조합에서 verb 외에 derived adverb/conjunction도 생성한다 | Semantic IR을 DerivedVerb 중심으로 제한하지 않고 result POS를 가진 `DerivedEntity`로 일반화한다 |
| `p.c::cases[]` row 3/4/5/6은 adverb application, conjunction application, fork, hook의 parser reduction shape를 정의한다 | RustJ Semantic Function DAG의 parent/operand shape도 이 parse production을 기준으로 만든다 |
| `jtype.h`/`ja.h`의 `V` + `fdef/fdeffill`은 parser reduction 결과를 shared function block으로 realization하고 specialized executor를 선택한다 | shared graph 원칙과 semantic/executor 분리는 채택하되, `fgh`의 execution auxiliary를 source DAG child로 자동 복제하지 않는다 |
| rank/insert/fit/power/under 등은 parser의 ADV/CONJ application production을 통해 derived function이 되고, jsource execution object는 form별 auxiliary metadata를 추가할 수 있다 | RustJ semantic DAG는 operator identity + parser operands를 보존하고 execution-only auxiliary는 downstream attachment로 분리한다 |
| `cf.c`는 boxed noun을 특정 modifier 문맥에서 gerund로 해석한다 | gerund를 별도 전역 noun type으로 만들지 않고 boxed noun + contextual GerundSemantics로 보존한다 |
| parser assignment reduction은 assigned J entity를 parse stack/result에 남기면서 symbol table을 갱신한다 | assignment를 entity-producing effectful expression으로 모델링한다 |
| `cr.c`/rank conjunction은 negative requested rank를 argument rank에 상대적으로 resolve하고 infinite rank를 별도로 다룬다 | rank IR을 nonnegative integer 하나로 축소하지 않고 Infinite/Absolute/Relative `RankSpec`을 둔다 |
| `cr.c::jtqq`는 `Verb"RankNoun` 외에도 right Verb rank extraction과 left Noun gerund/constant-verb form을 처리한다 | J Semantic IR의 rank node를 verb+integer pair로 제한하지 않고 original entity operands를 보존한다 |
| `cr.c` rank dyad는 frame prefix agreement를 검사하고 residual frame에 cell을 반복한다 | NumPy broadcasting으로 대체하지 않는다 |
| `cr.c`는 zero cells에서 fill-cell을 실행해 result cell type/shape를 정한다 | zero-trip elimination 전에 fill/prototype semantics를 해결한다 |
| `result.h`는 rank/modifier의 cell results가 type/shape 불일치하면 homogeneous fast path에서 assembly path로 전환하고 type/shape join + framing fill을 수행한다 | rank map을 항상 static uniform tensor map으로 가정하지 않고 `RankAssemblySemantics`를 보존한다 |
| `cv.c`의 `!.`는 comparison tolerance 또는 fill을 바꾸는 derived verb를 만든다 | tolerance/fill override를 semantic contract로 보존한다 |
| `ar.c`는 일반 float reduction에 SIMD/multiple-accumulator 경로를 사용할 수 있고 `+/!.0`에는 compensated summation 경로가 있다 | exact source operation order를 blanket semantic invariant로 만들지 않고 primitive/fit별 numeric policy를 보존한다 |
| `va2.c`는 unsupported argument-type pair라도 empty operand가 있으면 notional safe type로 재해석하는 경로를 가진다 | type/domain inference를 dtype pair만으로 고정하지 않고 emptiness/fill context를 입력으로 받는다 |
| `va2.c`는 agreement/rank-shape 검사와 domain/type/value error의 precedence를 의도적으로 관리한다 | GPU parallel error reporting도 J error contract를 따른다 |
| `va2.c`는 retryable overflow를 retry/repair하고 result type consistency를 유지한다 | primitive/type별 overflow promotion을 lane-local 임의 처리로 바꾸지 않는다 |
| sparse가 AT/type 및 `$.`를 통해 J-visible하고 axes/element를 가진다 | sparse를 단순 physical compression format으로 보지 않는다 |
| BOX는 noun type이고 boxed atom이 J value를 담는다 | boxed를 physical encoding 목록에서 제외한다 |

이 검토에서 발견된 차이는 **포기한 jsource implementation detail이 아니라 observable semantics에 영향을 주는 항목만** 반영했다.

### 15.6 설계 일관성 불변식

앞으로 문서를 수정할 때 다음 불변식을 독립적으로 점검한다.

1. **J semantic structure 보존** — hook/fork/train/derived verb/rank는 Semantic Analyzer가 보기 전에 불필요하게 소거하지 않는다.
2. **Semantic Analyzer는 target-independent** — target capability, cost, tile/layout/device 선택은 analyzer 책임이 아니다.
3. **Logical IR은 target-independent structural-fact rich, schedule-free** — iteration/access/dependency/constraint처럼 hardware planning에 유용한 구조적 fact를 표현하지만 특정 target/warp/tile/buffer id는 갖지 않는다. 기존 ‘hardware-aware’는 이 뜻의 약칭이다.
4. **Primitive semantics와 realization 분리** — PrimitiveSpec/capability interface에 vendor resource 숫자를 넣지 않는다.
5. **Storage requirement와 materialization 분리** — logical persistence 요구와 실제 buffer allocation/copy를 같은 개념으로 쓰지 않는다.
6. **Schedule과 payload IR 분리** — fusion/tile/vectorization 선택은 native Schedule Plan이나 external compiler가 담당한다.
7. **Physical Planner는 Route A 전용** — external route가 RustJ Physical Plan을 반드시 거친다고 쓰지 않는다.
8. **Native fallback은 보장 아님** — 지원되는 native path가 있을 때만 fallback이며, 없으면 Unsupported가 정상 결과다.
9. **Effect와 error ordering 명시** — pure data dependency만으로 표현되지 않는 ordering은 effect token/speculation contract로 보존한다.
10. **Dynamic assumption은 witness/guard로 추적** — optimization이 암묵적 shape 또는 representation 가정에 기대지 않는다.
11. **Logical constraint와 representation fact 분리** — divisibility/shape 관계와 stride/alignment/address-space를 같은 fact domain에 넣지 않는다.
12. **Semantic mask와 predication 분리** — source/operation 의미의 mask만 Logical IR에 두고 tail/vector/workgroup mask는 schedule에서 만든다.
13. **Conflict semantics와 atomic realization 분리** — conflicting update를 Logical IR에 표현하되 atomic instruction 사용은 downstream이 결정한다.
14. **External IR은 projection** — RustJ Logical IR을 MLIR/StableHLO의 표현력에 맞춰 축소하지 않는다.
15. **Late bufferization** — alias/destination contract는 logical에 둘 수 있지만 BufferId/materialization은 downstream에서 정한다.
16. **Verifier first** — 잘못된 IR을 downstream이 추측해서 복구하게 하지 않는다.
17. **Version boundary 명시** — external interchange를 시작하면 IR schema와 registry/compiler provenance를 기록한다.
18. **Async dependency는 explicit** — physical async execution에서 host statement order를 dependency로 암묵 사용하지 않는다.
19. **Resource와 cost 분리** — ResourceEstimate는 resolved TargetProfile hard facts에 의존하고, empirical CostProfile은 CostEstimate/ranking에만 사용한다.
20. **Route는 혼합 가능** — external/native route는 whole-program exclusive choice가 아니라 legal region/subgraph 단위로 partition할 수 있다.
21. **Analysis state와 semantic error 분리** — lattice의 unknown/unreachable과 J의 domain/rank/length error를 같은 상태로 표현하지 않는다.
22. **RoutePartition은 plan** — route 배정은 Logical IR semantic identity가 아니며 target/backend 조건에 따라 재계산 가능하다.
23. **Extension name은 keyword가 아니다** — ordinary J binding/품사 해소를 사용하고 tokenizer/parser spelling special-case를 만들지 않는다.
24. **Surface builder와 derived op를 구분** — parameterized adverb identity와 그 결과 computational verb/LogicalOp identity를 같은 것으로 취급하지 않는다.
25. **Full J semantics와 analyzable array profile 분리** — advanced compiler contract가 없다는 이유만으로 valid J semantics를 부정하지 않는다.
26. **Mutable state externalization** — weight/grad/optimizer/checkpoint 같은 mutable array state를 primitive/verb hidden field에 숨기지 않는다.
27. **SSA는 namespace의 대체물이 아니다** — runtime J name lookup/assignment가 필요한 곳을 무리하게 SSA binding으로 고정하지 않는다.
28. **Definition code와 invocation frame 분리** — 재귀/동시 호출이 local values를 공유하지 않게 한다.
29. **J validity와 compilation eligibility 분리** — advanced lowering이 없다는 이유로 valid J program을 semantic error로 분류하지 않는다.
30. **Prototype implementation language는 architecture가 아니다** — Python registry/CUDA-family field 같은 역사 구현 선택을 RustJ semantic boundary로 승격하지 않는다.
31. **Alias legality와 in-place realization 분리** — `MayReuse` 가능성과 실제 buffer reuse 결정을 같은 bool로 표현하지 않는다.
32. **Array value와 storage identity 분리** — 모든 J 데이터가 array라는 의미론과 SSA ValueId/StateResource/BufferId의 compiler identity를 혼동하지 않는다.
33. **Computational built-in도 hardware lowering 대상** — hardware-aware compilation 대상인 기존 J computational entity와 extension-derived op를 같은 lowering/capability architecture에서 다룬다. namespace/control/system-foreign semantics를 억지로 pure GPU op로 만들지는 않는다.
34. **Backend/architecture/device 분리** — CUDA/ROCm 같은 backend, ISA/microarchitecture target, exact device capacity를 하나의 profile identity로 뭉개지 않는다.
35. **Target locale path는 명시적** — device→architecture→family→backend→class→generic 순서는 compiler namespace의 explicit resolution path이며 숫자 버전 상속으로 추론하지 않는다.
36. **Locale lookup과 candidate selection 분리** — locale은 lowering/capability 후보를 찾고, 실제 realization 선택은 legality/resource/cost 분석이 한다.
37. **Ordinary name 품사는 parser-time lookup** — enqueue가 local/locale name의 noun/verb/adverb/conjunction class를 미리 고정하지 않는다.
38. **Function names preserve late binding** — verb/adverb/conjunction NameRef를 static proof 없이 현재 value identity로 얼리지 않는다.
39. **J agreement is prefix agreement** — NumPy trailing broadcasting으로 대체하지 않고 rank/cell/frame repetition 의미를 보존한다.
40. **Empty execution has fill-cell semantics** — zero-trip이라는 이유로 prototype/type/shape/error 의미를 생략하지 않는다.
41. **Boxed/sparse are J-visible semantics** — boxed hierarchy와 sparse axes/element를 단순 physical encoding으로 취급하지 않는다.
42. **Tolerance/Fit are semantics** — comparison tolerance와 `!.`에 의한 numeric/fill variation을 backend optimization에서 잃지 않는다.
43. **Floating order is contract-driven** — 일반 J float 연산의 exact scalar execution order를 전역 불변식으로 가정하지 않고, primitive/derived verb의 reassociation·accuracy·compensated semantics를 따른다.
44. **Error contract is observable semantics** — J가 정한 precedence/suppression/retry를 보존하고 parallel first-error를 임의로 노출하지 않는다.
45. **Nameref keeps expected POS** — late lookup은 허용하지만 reference 생성 시의 verb/adverb/conjunction 품사 계약을 버리지 않으며 mismatch는 J의 domain error semantics를 따른다.
46. **Sentence environment is not pre-snapshotted** — 우측→좌측 evaluation 중 name lookup/assignment/locale mutation의 observable sequencing을 보존한다.
47. **Rank map is not always fixed-shape** — per-cell result type/shape uniformity를 증명하지 못하면 J의 result assembly/type join/framing fill semantics를 보존한다.
48. **Assignment is entity + effect** — `=.`/`=:`를 void statement로 낮추지 않고 binding mutation과 assigned J entity result를 함께 보존한다.
49. **Type semantics may depend on emptiness** — dtype pair만으로 domain/promotion을 확정하지 않고 J의 empty/fill/prototype context를 반영한다.
50. **Rank is not just usize** — infinite rank와 argument-relative negative rank를 semantic RankSpec으로 보존하고 적용 시 effective cell rank를 resolve한다.
51. **Rank conjunction is entity-based** — `"`의 left/right operand를 verb+integer로 가정하지 않고 J의 noun/gerund/verb-rank forms를 semantic analysis 전까지 보존한다.
52. **Lowering key includes semantic valence/context** — raw primitive id/spelling만으로 backend lowering을 선택하지 않고 resolved valence와 derived numeric/rank/effect semantics를 포함한 operation key를 사용한다.
53. **Innate rank is valence-specific** — primitive rank를 단일 값으로 두지 않고 monad와 dyadic left/right rank contract를 분리한다.
54. **J errors may be control flow** — try/catch/throw 영역 안의 observable error를 fatal diagnostic으로 접지 않고 exceptional successor/동등 runtime semantics를 보존한다.
55. **Train graph does not imply branch independence** — hook/fork/train의 value graph가 병렬 가능해 보여도 J의 name/effect/error execution order를 proof 없이 제거하지 않는다.
56. **Forward equivalence is not full derived-verb equivalence** — `::`, `:.` 등 modifier가 붙인 error/inverse/latent semantics를 현재 forward dataflow가 같다는 이유로 소거하지 않는다.
57. **Derived entity is not always a verb** — parser가 생성할 수 있는 derived adverb/conjunction의 result POS와 operands를 J Semantic IR에서 표현한다.
58. **Gerund is contextual noun semantics** — boxed noun을 전역적으로 gerund type으로 바꾸지 않고 modifier가 요구할 때 gerund interpretation을 적용한다.
59. **Architecture inventory is not a blocking milestone** — TargetProfile/resource/mixed-route의 장기 설계를 유지하되 A1→A3-v0→최소 CPU vertical slice를 먼저 증명한다.
60. **Unknown access is an optimization barrier, not a semantic error** — AccessRelation이 Opaque여도 valid J semantics와 conservative/runtime lowering 가능성을 유지한다.
61. **Mixed route is staged** — architecture는 mixed route를 허용하지만 첫 external implementation은 verified single-block region 전체를 한 route로 보낸다.
62. **Parser rules define the Function DAG** — ADV/CONJ application과 hook/fork의 parent/operand shape는 jsource parser reduction 규칙을 따르며 modifier별 독자 AST shape를 발명하지 않는다.
63. **Parse operands and executor auxiliaries are separate** — semantic DAG에는 parser operands/J semantics를 보존하고 jsource `fgh/localuse`의 실행 최적화 보조 객체를 자동 semantic child로 승격하지 않는다.
64. **Semantic modifier syntax is not LogicalOp syntax** — `/`, `"` 등의 parser-produced operator DAG를 Semantic IR에서 `Reduce`/`CellApply`로 조기 치환하지 않는다. normalized op는 Semantic Analyzer 이후에만 만들며, `MapCells`는 uniform-result proof 뒤의 제한된 lowering form이다.
65. **Parser production coverage is explicit** — 미지원 jsource parse row/form을 임의의 대체 AST로 해석하지 않고 coverage manifest에 pending으로 남긴다.
66. **Rank conjunction and implicit loop are distinct** — `"`는 parent conjunction + two operands로 derived function을 만들고, cell iteration은 function application의 공통 semantics다.
67. **Rank boundaries are semantic until proven fusible** — nested explicit/innate rank boundaries를 effective rank 하나로 early collapse하지 않는다.
68. **CellApply is logical, not physical** — frame iteration/repetition/assembly를 logical CellApply로 표현하고 CPU/GPU loop/thread mapping은 downstream이 선택한다.
69. **IRS means loop absorption, not different semantics** — jsource IRS와 같은 fast path는 generic CellApply와 동등해야 하며 lowering capability로 표현한다.
70. **Implicit loop owns empty and assembly semantics** — fill-cell prototype과 heterogeneous result assembly는 primitive kernel의 우연한 동작이 아니라 CellApply contract다.

이 목록과 충돌하는 문장이 생기면 더 오래된 문장을 유지하지 말고 권위 설계를 이 불변식에 맞춰 갱신한다.

### 15.7 문서 완전성 감사 — stage contract가 닫혀 있는가? (2026-10-06)

RustJ 문서는 개별 주제의 깊이는 충분하지만, 설계가 커지면서 **각 stage 자체의 설명보다 stage 사이의 연결 계약이 흩어지는 문제**가 생겼다. 따라서 “문서가 길다/자세하다”와 “독자가 compiler를 처음부터 끝까지 재구성할 수 있다”를 같은 것으로 보지 않는다.

앞으로 주요 compiler stage는 최소한 다음 질문에 답해야 문서적으로 **닫힌(closed) contract**로 본다.

~~~text
1. 왜 이 stage가 존재하는가?
2. 입력은 무엇인가?
3. 출력은 무엇인가?
4. 반드시 보존해야 하는 semantic information은 무엇인가?
5. 이 stage가 결정하면 안 되는 것은 무엇인가?
6. 앞/뒤 stage와의 contract는 무엇인가?
7. 대표 source → IR/plan 예제가 있는가?
8. 현재 구현은 어디까지인가?
9. 어떤 verifier/test가 그 경계를 증명하는가?
10. 아직 미구현·미정인 부분은 무엇인가?
~~~

아래 평가는 **설계의 품질 평가가 아니라 문서 연결 완전성 평가**다. `충분`은 모든 구현이 끝났다는 뜻이 아니며, `부분`은 설계가 틀렸다는 뜻이 아니다.

| Stage / 경계 | 현재 문서 상태 | 이미 강한 부분 | 아직 닫히지 않은 부분 |
|---|---|---|---|
| word formation → enqueue → parser | **문서 계약 닫힘 / 구현 수렴 중** | A0.5/F0–F2/P0–P8, jsource oracle, 9-row reduction, name/assignment sequencing, differential gates, §3.3.2.2 `+/ y` canonical frontend trace | locative/definition/gerund/value-dependent constructor의 실제 지원 범위는 A0.5 checklist와 differential gate가 결정; orientation contract 자체는 닫힘 |
| Semantic Construction / binding / dynamic semantics | **현재 frontend E2E subset 완료 / 일반 compiled CFG 미완료** | FunctionEntity/JEntity, late NameRef, definition metadata, 지원 per-call LocalFrame·if/while/for/try·nested direct/string explicit, A3 definition reference | full J/locale·locative, body Graph·Logical analysis, source-frame diagnostics 및 compiled Branch/CondBranch/value-merge는 후속이다. A3 Terminator는 Return-only |
| J Semantic → J Graph IR | **문서 계약 닫힘** | GraphForm/GraphBasis/GraphHint, provenance, applied graph, Graph/Execution 분리, §4.1.0 canonical suite로 `@:`/ordinary+capped fork/hook/rank/reduce/prefix-infix를 동일 형식 비교 | 남은 gap은 form별 실제 lowering/test coverage이지 stage ownership 설명 부재가 아님 |
| Graph analysis → candidate/proof | **문서 계약 보강됨 / 구현 부분** | §4.1.4에 orthogonal evidence, derived lifecycle, evidence owner, guarded legality, overlap/selection 규칙을 통합 | 공통 `CandidateEvidence/ProofBundle`·obligation discharge·SelectionPlan은 **미구현**. 개별 proof algorithm의 세부 구현은 해당 optimizer 착수 시 verifier/test와 함께 확정 |
| J Graph → Execution Semantic Lowering → A3 | **문서 계약 대체로 닫힘 / CFG 구현 미완료** | direct lowering, Graph/Execution fact drift check, Execution Basis, SemanticCheck, effect/error/speculation, verifier, schema version, canonical mean trace, §3.7.1 planned/current control-flow handoff | A3-v0는 실제로 single-block/Return-only다. 남은 것은 문서 예제가 아니라 **Branch/CondBranch/block merge를 포함한 executable explicit-definition CFG lowering과 differential E2E** |
| Route analysis / partition | **문서 계약 보강됨 / 구현 부분** | §2.5.1에 live-in/out, effect live-out, SemanticCheck, guard, representation-neutral bridge, region legality 계약을 통합 | 현재 `RouteRegion { class, operations }`와 contiguous grouping은 v0 helper. 실제 bridge/region-wide verifier와 mixed-route executor는 미구현 |
| Schedule / Physical Planner | **M4-v0 본체 미완료 / i. family 선택기 일부 구현** | §5.2.1에 `PlanBufferId ≠ runtime BufferId`, PhysicalView, BindInput/Check/View/Materialize/Kernel/Return, lifetime/reuse/verifier/error-cleanup 계약을 정의 | `physical_plan`의 빈 A3/closed dense literal identity 타입·verifier·executor는 제한적으로 구현됨(#4). 일반 planner, Add/Check/View/Materialize/lifetime/reuse는 미구현. Transfer/Sync/async는 M4 이후 |
| Native Executor | **M4-v0 계약 대체로 닫힘 / 구현 미완료** | §5.2.1/§5.3에 op 역할, verifier, cleanup/error, executor non-responsibility, canonical mean planned route를 연결 | 실제 Physical Executor와 differential E2E test가 없음. stateful/async execution contract는 후속 |
| fallback / guard miss / replay | **문서 계약 보강됨 / dispatcher 미구현** | §3.9.4에 route fallback/guard miss/replay/continuation 구분, decision table, commit frontier, RuntimeSemanticFallback의 정확한 의미를 통합 | integrated guard dispatcher/exact continuation/transaction rollback은 미구현이며 capability로 주장하지 않음 |
| external route / GPU | **boundary contract 고정 / 구현 보류** | §5.5.1에 adapter input/capability/output, SemanticCheck/error/effect/token mapping, bridge/ownership, round-trip verifier, failure class를 정의 | production adapter는 미구현. M6까지 implementation gate로만 유지하며 CUDA는 의도적으로 보류 |
| validation / versioning | **문서 계약 보강됨 / 후속 verifier 구현 필요** | frontend differential gate, J Graph 0.10/A3 0.6 exact schema+registry verifier, §5.7.5 negative matrix, §5.7.6 explicit migration/downgrade policy | Candidate/Route/Physical/External negative verifiers는 각 stage 구현과 함께 추가; portable serialization 자체는 아직 미제공 |

#### 15.7.1 1차 감사에서 닫은 문서 계약

현재 구현 우선순위 M2를 바꾸지 않은 채, 2026-10-06 1차 감사에서 다음 다섯 빈칸을 **문서 계약 수준에서** 닫았다. 이는 구현 완료 체크가 아니다.

- [x] **Candidate lifecycle / proof discharge** — §4.1.4
- [x] **RouteRegion boundary contract** — §2.5.1
- [x] **M4 최소 PhysicalPlan v0 schema / verifier / cleanup contract** — §5.2.1
- [x] **Fallback / guard miss / no-replay decision table** — §3.9.4
- [x] **Canonical end-to-end compiler trace** — §4.11.4.12의 `(+/ % #) y`

특히 canonical trace에는 **현재 구현 stop line**을 넣어 route prototype 이후의 Schedule/PhysicalPlan/CPU result 구간을 `planned M4`로 표시했다. 목표 architecture 그림을 구현 완료 증거로 사용하지 않는다.

#### 15.7.2 2차 감사에서 닫은 문서 계약

- [x] **Frontend canonical sentence trace** — §3.3.2.2 `+/ y`
- [x] **Explicit-definition control-flow handoff** — §3.7.1
- [x] **Canonical J Graph example suite** — §4.1.0
- [x] **External adapter boundary contract** — §5.5.1
- [x] **Cross-stage negative verifier matrix + serialization migration policy** — §5.7.5–§5.7.6

이 단계에서도 구현하지 않은 타입/API를 구현 완료처럼 쓰지 않았다. 특히 Branch/CondBranch, CandidateEvidence/SelectionPlan, RouteBoundary concrete type, PhysicalPlan, ExternalRegionPlan은 **target concepts**이며 현재 코드에 모두 존재하는 API가 아니다.

#### 15.7.3 현재 남은 빈칸 — implementation-driven detail

1·2차 감사 후 남은 큰 항목은 architecture owner가 없는 빈칸이라기보다 **구현에 들어가야 구체 타입과 proof algorithm을 정할 수 있는 세부사항**이다.

- 각 fusion/rewrite/scan obligation의 실제 proof algorithm과 cached-evidence invalidation 구현
- explicit-definition CFG의 concrete Branch/CondBranch/block-argument API와 loop/try/select lowering
- RouteBoundary/BridgeRequirement의 concrete Rust 타입과 mixed-route executor integration
- PhysicalPlan concrete op structs, planner algorithm, Buffer reuse implementation
- 첫 external adapter별 actual capability matrix와 emitted-IR round-trip implementation
- async token/timepoint, stateful native route, exact continuation/transactional rollback처럼 아직 의도적으로 후순위인 semantics
- portable artifact를 실제 파일/프로세스 경계로 내보낼 때 선택할 wire format

이 항목은 지금 임의 타입을 미리 고정하지 않는다. 해당 구현 단계가 시작될 때 **현재 stage contract → 최소 타입 → verifier/negative test 하나 → 다음 의미** 순서로 구체화한다.

#### 15.7.4 문서 유지 규칙

- 새 major stage/type을 추가할 때 위 10개 질문 중 해당 항목을 함께 갱신한다.
- 구현 타입 이름과 문서의 개념 이름이 다르면 “현재 구현명 / 목표 개념명”을 명시한다.
- 목표 architecture 그림과 현재 구현 상태를 같은 시제로 쓰지 않는다.
- framework 비교를 수정하면 `FOUNDATIONS`, `PROJECT`, `README`, `AGENTS`에 같은 주장을 중복해 둔 곳이 없는지 교차 검색한다.
- 새 validation gate를 완료해 정본에 기록할 때 문서 상단 `최신 검증`과 §12 현재 요약도 같은 변경에서 갱신한다. historical gate의 “최신” 표현은 당시 시점임을 명시한다.
- 한 stage의 상세 절이 길어질수록 **입력/출력/금지/다음 경계** 요약을 절 앞이나 끝에 유지한다.
- 문서 감사에서 발견한 항목을 별도 Markdown 보고서로 분리하지 않는다. 이 절과 §10/§16의 기존 체크리스트에 흡수한다.

---
