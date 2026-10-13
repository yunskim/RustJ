[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)

<a id="c-rustj-계층-경계"></a>
##### C. RustJ 계층 경계

~~~text
1. J Graph IR / graph facts
   - source/derived-verb provenance
   - rank/cell/frame topology
   - shape/rank/count facts
   - producer/consumer demand
   - logical view/materialization opportunity
   - operation semantic traits

2. Graph optimization
   - composition/reduction fusion candidate
   - rank-domain absorption candidate
   - shape/count simplification
   - box/open/materialization elimination
   - Mean / moving-average / dot-like / ranking / intersection idiom recognition
   - rewrite 후 candidate rediscovery

3. Execution Semantic Lowering / Physical planning
   - explicit loop/iteration realization
   - bufferization and inplace reuse
   - concrete view/alias representation
   - cache blocking/chunking
   - SIMD / multicore / GPU workgroup
   - library/custom-kernel selection
~~~

두 오류를 피한다.

1. **jsource의 special entry point를 Graph IR node 종류로 일대일 복제하지 않는다.** 예를 들어 `jtfslashatg` 자체가 semantic op가 아니다. RustJ에서는 source graph가 transform→reduce이고 fusion/materialization-elision candidate와 schedule choice가 분리된다.
2. **jsource runtime flag를 그대로 semantic property로 승격하지 않는다.** `WILLOPEN`, `USESITEMCOUNT`, `RANKATOP`의 의미를 producer-consumer demand, iteration-domain compatibility, result assembly/materialization contract 같은 일반 fact로 옮긴다.

<a id="d-구현-원칙"></a>
##### D. 구현 원칙

- jsource의 special case는 **최적화 아이디어의 source oracle**이지 RustJ rewrite의 correctness proof 자체가 아니다.
- 각 rewrite/idiom은 J-visible dtype, rank/frame/cell, empty/prototype, fit/tolerance, overflow/promotion, effect/error order를 포함한 별도 witness를 가진다.
- `Mean`, dot-like contraction, Ranking 같은 higher-level identity를 만들더라도 원 source graph/provenance를 검증 가능하게 유지한다.
- rewrite가 새 canonical identity를 만들면 후속 candidate discovery를 다시 실행할 수 있다. `+/%# → Mean → Window(Mean) → MovingAverage candidate`가 대표 사례다.
- view/materialization freedom은 alias/physical layout을 미리 결정하지 않는다.
- in-place와 cache/SIMD/GPU realization은 Graph IR correctness와 분리한다.
- RustJ가 jsource보다 일반적인 graph rule을 갖더라도 jsource differential oracle과 semantic tests를 통과하지 못하면 채택하지 않는다.

이 절은 **jsource 최적화를 전수 구현했다는 상태 보고가 아니다.** 향후 optimizer 작업에서는 각 catalog 항목을 `source pattern → semantic preconditions → graph candidate/rewrite → downstream realization → C differential regression` 형식의 구현 항목으로 전개한다.

<a id="e-독립-재검토-결과-2026-10-06"></a>
##### E. 독립 재검토 결과 (2026-10-06)

- **수정:** `jtfslashatg`를 범용 kernel fusion으로 설명하지 않는다. cell-at-a-time cache-footprint optimization과 full kernel fusion은 서로 다른 실행 선택이다. 이 구현은 빈/sparse·in-place 수익성·dtype mismatch 등에서 일반 경로를 택하고 overflow 발생 시에도 reversion할 수 있다.
- **수정:** `WILLOPEN/USESITEMCOUNT`를 임의의 소비자가 shape/count만 요구한다는 일반 의미론으로 등치하지 않는다. 여기서는 특히 rank Result Assembly와 다음 open/raze 사이의 cooperation이며 `USESITEMCOUNT`는 raze의 count/shape 검사 pass를 앞당기는 맥락이다.
- **수정:** `jtmean`, `jtmovavg`, `jtsumattymes1`는 상위 연산 의도를 드러내지만 J의 수치 계산 순서·fit·dtype·overflow·rank·empty semantics가 다른 일반 라이브러리 Mean/Dot과 자동으로 같다고 할 수 없다.
- **보강:** `ar.c` reduction, `ap.c` prefix/scan/window, `vi.c` prehash 및 sort/search, `cu.c` under/each, bound-constant numeric specialization을 조사 대상에 추가했다.
- **설계 경계:** J Graph에는 원 식과 후보/provenance를 남기고, Execution Semantic Lowering에서 call-dependent facts/equivalence를 discharge하며, Physical Planner가 buffer/cache/SIMD/GPU execution을 선택한다. source의 fast path가 존재한다는 사실은 RustJ에서 무조건 rewrite하라는 proof가 아니다.
- **범위:** 위 catalog는 조사한 source 영역에서 검증한 **대표 최적화 계열**이며 jsource 전체의 exhaustive inventory 또는 RustJ 구현 완료를 뜻하지 않는다. 이번 검토는 source/doc audit이고 Rust/C differential 또는 benchmark는 실행하지 않았다.

<a id="f-별도-기준으로-실시한-3회-검증-2026-10-06"></a>
##### F. 별도 기준으로 실시한 3회 검증 (2026-10-06)

| 독립 검증 | 출발 질문 | 확인한 근거·결론 | 남은 한계 |
|---|---|---|---|
| **1. 원본 구현 추적** | jsource의 각 경로가 실제 존재하며 언제 실행되는가? | `ca/cf/cr/va2/ar/ap/vi/cu`에서 construction → entrypoint → fallback을 대조했다. `jtintersect`의 `C_VIAVX` 조건, `jtiobs`의 `ct=0`/boxed 제한을 보완하고 `jtfslashatg` fallback 근거 링크를 확장했다. | 대표 source paths만 확인; 전체 jsource 특수화 목록은 아님 |
| **2. 의미론·반례 검증** | 동일한 graph 패턴이어도 J에서 불법인 변환은 무엇인가? | empty/sparse, rank/frame/cell, `!.`·numeric promotion·NaN·overflow, virtual alias/boxed Result Assembly, dynamic name/effect/error ordering을 독립적으로 반례 범주로 구성했다. 일반적인 Mean/Dot/Box→Open/fusion의 무조건 치환은 기각한다. | J/C differential oracle과 runtime tests는 미실행 |
| **3. 계층·문서 정합성** | 설계에서 누구에게 분석·증명·실현 책임이 있는가? | §4.1.3의 발견, §4.1.4의 candidate evidence/legality/selection, §4.2의 Execution Semantic Lowering, 후속 Physical Planner를 대조했다. Graph candidate와 실행 commitment가 분리돼 있고, 이 절의 Markdown table 단절을 수정했다. | 구체 RustJ 구현 상태나 성능 향상을 검증한 것은 아님 |

**Source revision 재확인:** 고정된 소스 검토 기준은 `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`이다. 2026-10-05의 jsource `master` HEAD `0a5101cfdd834b23a0b89d455e4f327310520a08`는 이 기준보다 20 commit 앞서고 `ap.c/ar.c/va2.c` 변경을 포함한다. 두 revision의 `jtmovavg`, `jtmovfslash`, `jtmean`, `jtreduce`, `jtfslashatg`, `jtsumattymes1`의 **검토한 진입·핵심 guard 발췌는 동일**했다. 이는 함수 전체, 모든 동작 경로 또는 jsource 전체에 변화가 없다는 증명이 아니다.

<a id="g-이후-optimizer-구현-시-필요한-차등-검증-항목--아직-미실행"></a>
##### G. 이후 optimizer 구현 시 필요한 차등 검증 항목 — 아직 미실행

| 대상 후보 | 반드시 비교할 경계 | 증명/실행 확인점 |
|---|---|---|
| `f/@:g` | 빈 배열·sparse, in-place 우세, result dtype 불일치, overflow reversion | source/result 동일, error order, effect 이전 fallback, 불법 재실행 없음 |
| `+/%#` 및 Window(Mean) | 길이 0/1/다중 cell, 정수·실수·NaN·overflow, window 길이 0/1/초과 | cell 길이·prototype·dtype·숫자 동등성 |
| `+/@:*"1 1` | mixed rank, empty/sparse, `!.0`·`!.1`, QP | rank/agreement, dtype/result shape, fallback |
| BOXATOP/WILLOPEN/USESITEMCOUNT | 중첩 박스·다양한 cell shape·raze·sparse·virtual alias | Result Assembly와 재귀화·usecount·검사 순서 |
| Search/Under/View | boxed rank/tolerance, prehash reuse, 이름 재바인딩, shared/in-place ravel | 검색 결과, inverse 시점, alias/pristinity |
| 모든 rewrite | source graph 버전, proof witness, effect/error dependency, guard miss | Unknown이면 commit 금지; side effect 이후 replay 금지 |

**결론:** jsource 소스는 optimization 후보를 발견하는 근거이지 RustJ rewrite의 correctness proof나 측정된 속도 향상의 증거가 아니다. 검증하지 않은 후보는 candidate로 남기며 선택된 transform으로 표시하지 않는다.

<a id="h-추가-전역-조사에서-발견한-누락-계열-2026-10-06"></a>
##### H. 추가 전역 조사에서 발견한 누락 계열 (2026-10-06)

기존 A절의 18개 항목은 jsource 전체를 대표하지 않는다. jsource 고정 revision의 `jsrc/` 파일 목록을 재확인하고, **기존 목록에 없거나 한 행에 과도하게 뭉쳐 있었던** 아래 계열을 별도 감사했다. 문서에 추가하는 것은 **설계 입력**이며 최적화 구현·성능 검증 완료를 뜻하지 않는다.

| 추가 계열 | 확인한 jsource 원본 | 실제 최적화와 필요한 적용 조건 | RustJ 책임 위치 |
|---|---|---|---|
| **Key / Group-by와 집계의 결합** | [`ao.c::jtkeyct` 분류·small-range 처리](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ao.c#L213-L260), [`jtsldot` 특수화](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ao.c#L824-L850) | `u/.`와 `f//.`에 대해 `+/`, 최소·최대, mean 등 허용된 그룹 집계를 일반적인 `reorder → Cut → Reduce` 경로와 달리 직접 그룹 결과에 누적한다. grouping의 **키 동등성·tolerance·그룹 순서·dtype·overflow** 및 지원 연산 종류가 제한이다. | **Graph Key/GroupReduce 후보 → typed group algorithm & storage planning** |
| **일반 inner product / matrix multiply 알고리즘 선택** | [`cip.c::jtdot/jtpdt`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cip.c#L715-L739), [`cip.c` small/cached/BLAS](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cip.c#L925-L956), [`gemm.c` 외부 BLAS/내장 경로](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/gemm.c#L923-L985) | 기존 문서의 **rank-1 dot-like** 특수 함수 `jtsumattymes1`보다 넓은 별도 계열이다. `+/ . *`에 대해 크기·type별 직접 계산, cache-aware multiply, dgemm/zgemm/igemm 경로 및 numeric fallback을 구분한다. **NaN·0×무한대·정수 promotion/overflow·빈 축** 검사 필수. | **Graph Contraction identity → Execution lowering / target-aware GEMM choice** |
| **Grade / Sort / Ranking의 값 범위·크기 기반 전환** | [`vg.c` quick/radix/small-range/merge 분기](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vg.c#L525-L557), [`vgsort.c` direct sort](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vgsort.c#L120-L149), [`vgranking.c` histogram+prefix](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vgranking.c#L34-L69) | Grade를 무조건 sort로 내리지 않는다. 값의 범위/키 길이/정렬 axis/규모에 따라 counting-histogram+prefix, radix, qsort, merge, direct item sort를 선택하고 ranking `/:@/:`는 별도 small-range 경로가 있다. `vgsortiqavx512.c`는 특정 CPU 빌드 조건. **안정적 동률 처리·J 비교 규칙**을 증명한다. | **Graph Grade/Rank idiom → order-contract + physical algorithm selection** |
| **tolerance-aware hash 및 테이블 전략** | [`viavx2.c` 인접 hash interval 확인](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx2.c#L10-L49), [`viavx.c` bit/packed table allocator](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx.c#L15-L43) | 기존 `vi.c`의 prehash/sort 항목을 확장한다. 근사 동등성을 단순 exact-key hash로 대체할 수 없어 인접 interval 및 signed-zero 등 J tolerance 계약을 반영한다. table layout/packed bits/AVX 최적화는 target 종속적이다. | **Search semantic witness + tolerance facts → Hash execution planner** |
| **Interval Index `I.`의 별도 알고리즘** | [`viix.c` boolean·small-range 경로](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viix.c#L18-L48), [branch-sensitive/branchless binary search](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viix.c#L55-L86) | 일반 `i.` 해시 검색과 다른 sorted-interval search다. 입력 ordering, type, range에 따라 lookup table 또는 binary-search 선택. 일부 AVX512 경로는 `C_FSGSBASE` 등의 빌드 가드를 사용한다. | **Graph IntervalLookup identity → search algorithm selection** |
| **Cut/Substring의 virtual slice 및 boxer 소비자 협력** | [`cc.c::jtrightcut0`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cc.c#L133-L147), [`cc.c::jtboxcut0` copy/virtual](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cc.c#L158-L210) | 일반적인 Cut을 무조건 모든 셀 복사/boxing으로 내리지 않는다. 제한된 1차원 segment에서 virtual result를 만들거나 다음 소비자가 열 것을 아는 경우 boxed substring의 복사를 피한다. 음수/역방향 범위, rank, lifetime, error/fallback 조건을 보존한다. | **Graph Cut/segment provenance + demand → View/Result Assembly lowering** |
| **Gather/From의 copy-versus-view와 SIMD gather** | [`vfrom.c::jtget1cell`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vfrom.c#L39-L53), [AVX2 gather 내부 경로](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vfrom.c#L70-L108) | 단일 cell은 크기가 작으면 복사, 크면 virtual block 선택 가능. 여러 index의 비연속 추출은 실제 gather/copy가 필요하며 모든 `{`를 view로 환원할 수 없다. index bounds와 결과 순서·alias를 보존한다. | **Graph Gather access facts → Physical copy/view/vector gather** |
| **Reshape/Compress/Catenate의 소유권·복사 비용 최적화** | [`vf.c` reshape inplace/virtual](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vf.c#L301-L331), [`vrep.c` compress inplace](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vrep.c#L53-L93), [`vcat.c` boxed ownership transfer](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vcat.c#L192-L221) | 기존 `jtravel` 항목만으로는 부족했다. result가 입력과 물리적으로 호환될 때 header/shape 변경, 일부 복사 회피, abandonment·pristinity·recursive contents의 소유권 이전 등 서로 다른 경로를 갖는다. alias, usecount, fill 및 identity 조건 검증. | **Graph layout/materialization facts → Buffer planner/ownership lowering** |
| **Sparse 자료형 전용 알고리즘 분기** | [`cpdtsp.c` sparse inner product](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cpdtsp.c#L1-L43), [`vgsp.c` sparse grade](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vgsp.c#L1-L25), [`visp.c` sparse index-of](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/visp.c#L1-L25), [`vfromsp.c` sparse From](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vfromsp.c#L1-L35) | sparse는 “dense 그래프의 느린 대체 경로”가 아니라 전용 표현과 계산 전략을 갖는다. sparse element/axis/shape, fill, 형 변환의 J-visible 의미가 다르므로 generic fusion legality를 가정하지 않는다. | **Sparse semantic facts/route eligibility → sparse-specific execution** |
| **동적 이름 참조와 locale lookup의 캐싱** | [`sc.c` cached lookup와 invalidation](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sc.c#L57-L100), [short/long-term cache와 locale 조건](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sc.c#L140-L178) | 자료흐름 rewrite가 아닌 **동적 J binding 해석의 실행 비용 절감**. short-term timestamp/할당 invalidation, long-term cacheable name 및 permanent locale 조건, multi-thread 원자성 등 세부 제약이 있다. cache hit를 name lookup semantics의 변경으로 혼동하지 않는다. | **Name/Binding semantic boundary → Runtime lookup cache/guard** |

**별도 계열로 분리한 이유:** `GroupReduce`는 단순 Search가 아니고, full `+/ . *`는 rank-1 dot idiom보다 넓으며, `Sort/Ranking`은 단순 graph-pattern recognition을 넘어 선택 가능한 여러 알고리즘의 family다. 반면 virtual·inplace·hash table·SIMD 구현은 Graph semantic node를 새로 늘리는 이유가 아니라 downstream planner의 결정 자료다.

<a id="i-범위-및-누락-여부에-대한-보수적-판정"></a>
##### I. 범위 및 누락 여부에 대한 보수적 판정

- **기존 catalog에는 중요 계열의 누락이 있었다.** 위 H절에서 10개 family를 보강했다. 항목 수는 jsource 전체의 완전한 optimization inventory나 구현 우선순위를 의미하지 않는다.
- **소스 확인 범위:** pinned `jsrc/` 소스 트리의 직접 C/H 파일 목록을 먼저 확보하고, 기존 composition/rank/reduction/array 영역 밖의 grade·sorting·group/key·inner product/GEMM·index/hash·cut·shape/data movement·sparse·name cache의 진입점과 적용 guard를 대조했다.
- **남은 미조사 영역:** `jsrc/p.c` parser/assignment fast paths, `jsrc/cx.c` explicit-definition execution, `jsrc/va1.c/v0.c/v1.c/v2.c` 원시 산술·수치 dispatch, `jsrc/m.c/am.c` allocator/index amend, 기타 assembly/SIMD microkernel의 전수 목록은 이번에는 exhaustive하게 검증하지 않았다. “더 이상 빠진 기법이 없다”고 판정하지 않는다.
- **우선 적용 순서 제안:** (1) 고수준 `Key/GroupReduce`, `Contraction`, `Grade/Ranking`, `IntervalLookup` 후보·provenance를 표현할 수 있는지 확인; (2) J rank/tolerance/group ordering/numeric error witness 정의; (3) `Hash/Search`, sparse, view/gather, physical buffer, name cache 등 execution-route 별로 독립 구현·jsource 차등 시험. current frontend/optimizer gate를 건너뛰어 착수했다는 뜻은 아니다.
- **검증 경계:** 이 절은 GitHub 소스/설계 문서 검사다. native J 실행, RustJ differential test, CPU/GPU benchmark를 수행하지 않았으며 각 효익·동등성·fallback 적법성은 미검증이다.

<a id="j-rustj-프레임워크에-맞춘-구현-연결-2026-10-06"></a>
##### J. RustJ 프레임워크에 맞춘 구현 연결 (2026-10-06)

이 절부터는 H–I의 source survey를 **현 RustJ 소스 코드 경계에 적용한 결과**를 기록한다. 전수 J semantics 증명이나 jsource 성능 재현을 의미하지 않는다.

| source-derived family / 아이디어 | 현재 RustJ 소유 경계 | 2026-10-06 코드 반영 | 실행 최적화 도입 전 남은 조건 |
|---|---|---|---|
| `f/@:g`의 Map→Reduce streaming | 기존 `src/j_graph_fusion.rs` `MapReduce` + `fusion_planning.rs` | **기존 분석을 재사용**, 중복 rewrite 미등록 | per-cell semantics, numeric/dtype, effects, target capability, fallback |
| `+/%#` mean fork 인식 | 신규 `src/j_graph_jsource.rs` source-pattern analyzer | `FunctionEntity`의 **단항 호출 ordinary Fork + Insert(Add) / Divide / Tally** 완전 일치 시 `MeanIdiom` *후보만 발견* | J rank/cell·empty·dtype·float order·error proof; 실제 `Mean` 실행 op 미구현 |
| reduction/window/scan 기법 | 기존 `GraphForm::Reduce/PrefixInfix`, `j_graph_scan.rs` + 신규 source opportunity | `ReductionFastPath`, `WindowAlgorithm` 후보 등록; 기존 Scan witness와 충돌하지 않음 | small-cell, moving aggregate, NaN/overflow, prefix vs infix proof 및 후속 알고리즘 |
| `i.` / `e.` / `E.` search | `GraphForm::Atomic`, `PrimitiveId`, 기존 `FindViaWindowMatch` rewrite | dyadic Search를 `SearchAlgorithm` source opportunity로 기록; `Find`의 기존 witnessed rewrite 유지 | tolerant equality, key type, prehash legality, algorithm cost |
| dyadic `I.` interval index | `PrimitiveId::Icap` | monadic indices와 혼동하지 않고 **dyad만 `IntervalLookup`** 후보 등록 | ordering·shape·type·tolerance, search route |
| dyadic `{` 및 static reindex | `GraphBasisKind::DynamicGather/StaticReindex`, physical buffer route | `GatherCopyOrView`, `ReindexCopyOrView` 후보 기록. **zero-copy를 단정하지 않음** | bounds, alias/ownership, result shape, fill, target stride/gather support |
| Key/GroupReduce, full `+/ . *`, Grade/Ranking | `J Graph`의 향후 source identity + Execution Semantic Lowering | source 파일·심볼·proof obligations만 **catalog 등록**, 가짜 applied graph node/실행 후보 생성 없음 | 해당 J constructor/typed semantic support, source graph canonicalization, jsource differential |
| tolerant hash, sparse, buffer reuse, name-ref cache | typed semantic facts / Execution/Physical planner / binding runtime | 발견 registry에 **`DownstreamOnly` 또는 `AwaitingFrontendOrFacts`** 표시; graph algebraic rewrite로 위장하지 않음 | tolerance/sparse/alias/locale-version, fallback 및 cost contracts |

**실제 코드 연결 및 불변 조건**

- `src/j_graph_jsource.rs`: pinned source evidence, family stable ID, 책임 계층, discovery coverage, proof requirement를 소유한다. `Plan::jsource_opportunities()`는 **원래 J Graph의 source id, span, basis, facts**를 보존한 sidecar만 산출하며 source graph를 변경하지 않는다.
- `src/compilation.rs`의 `CompilationAnalysis::jsource_opportunities`와 `src/runtime.rs::analyze_compilation_diagnostic`으로 분석 결과를 기존 rewrite/Logical IR과 **병렬 제공**한다. 최적화 실행 경로에 자동 연결하지 않는다.
- candidate의 legality는 `AwaitingSemanticProofs`, `selected=false`로 고정된다. `verify(&Plan)`는 source/provenance/evidence 상태를 재도출하여 변조·오래된 후보를 거부한다. 이 verifier는 **J 수치적 동등성 검증기가 아니다**.
- `MapReduceStreaming`은 이미 `j_graph_fusion`이 책임진다. 독립된 동일 후보를 자동 추가하지 않는다.
- `GroupAggregate`, `MatrixContraction`, `GradeRanking`의 source registry가 존재해도 해당 J 구문이 RustJ에서 파싱/실행 가능하다는 뜻은 아니다.
- `tests/j_graph_jsource.rs`는 registry 불변성, source provenance, 후보 변조 거부, mean fork exact-match/부정 사례, 일부 기본 graph 패턴, 기존 Find rewrite·MapReduce fusion의 분리를 검사하도록 추가했다. **실제 Rust 테스트 실행 결과는 아직 확인하지 못했다.**

**후속 구현 계획—현재 M2/frontend 단계 우선순위를 앞서지 않는다**

1. **source-coverage gate:** `Key (/.), Dot (.), Grade (/:, \:), Cut (;.), Under (&.)`의 정확한 품사·derived-verb operand·rank 계약이 구현되기 전에는 candidate를 임의 생성하지 않는다.
2. **witness gate:** source optimization마다 `Semantics + RankCellAssembly + DType/Numeric/!. + Sparse/Empty + Effect/Error + Alias/Fanout` proof를 작성한다. 실행 전에 guard miss는 source fallback이 있어야 하고 effect 이후 replay는 금지한다.
3. **algorithm basis gate:** `GroupReduce`, `Contraction`, `GradeSort`, `IntervalLookup`의 Execution Semantic operation/facts와 J-specific physical routes를 분리한다. J Graph에 `jtsort`, `jtfslashatg` 같은 C 함수 노드를 추가하지 않는다.
4. **target gate:** CPU vector/BLAS, sparse implementations, GPU library/custom-kernel 경로는 `LoweringRegistry`와 target/resource/cost에서 별도 선택한다. `Unknown`은 실패가 아니라 **보수적 미최적화 경로**다.
5. **regression gate:** jsource와 empty/sparse, boxed/tolerance, `!.0/!.1`, NaN/overflow, rank/frame, error/locale/cache-invalidating writes를 비교한다. differential과 성능 측정 없이 최적화 완료로 표시하지 않는다.

<a id="k-후속-원본-재검토--단항-mean-guard와-런타임-최적화-경계-2026-10-06"></a>
##### K. 후속 원본 재검토 — 단항 Mean guard와 런타임 최적화 경계 (2026-10-06)

**발견 및 수정:** 고정된 [`cf.c::jtfolk` 98행](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L93-L101)은 `(+/ % #)`에서 **`f1=jtmean`만** 지정한다. 이항 executor `f2`를 Mean으로 바꾸지 않는다. 이전 `src/j_graph_jsource.rs`는 fork의 구성만 확인하고 적용 valence를 보지 않아 이항 호출에도 `MeanIdiom`을 만들 수 있었다. 이제 **`NodeKind::Apply`가 `Valence::Monad`인 경우에만** mean 후보를 등록한다. `tests/j_graph_jsource.rs`에는 동일한 fork의 이항 호출에 대한 부정 회귀 사례를 추가했다. 이것은 후보 발견의 정확도 수정이며 mean 수치 동등성·kernel 구현의 증명이 아니다.

**이전 미조사 영역에 대한 2차 소스 감사:** 다음은 새 Graph rewrite를 무조건 등록하는 근거가 아니라 현재 계층에 대한 추가 설계 입력이다.

| jsource 추가 확인 | 확인 위치 | RustJ 해석 및 소유 계층 |
|---|---|---|
| parser 행 0–2의 inplace 실행, assignment `zombieval`, 조기 parse 종료 | [`p.c` 10–24행](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c#L10-L24) | **Frontend/Runtime의 이름·대입 시점과 lifetime 관리**. Graph liveness만으로 assignment buffer donation을 선언하지 않는다. |
| explicit-definition local symbol table 재사용/복제, x/y precomputed bucket, 버려진 인자의 usecount 특수화 | [`cx.c` 270–329행](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L270-L329) | **Explicit runtime/binding implementation**. 동적 scope·재진입·인자 alias 증명 전에는 Graph rewrite가 아니다. |
| 단항 numeric type-dispatch와 실패 유형별 정밀도 변경·재실행, sparse의 별도 fallback | [`va1.c` 313–383행](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/va1.c#L313-L383) | **Typed semantic error/promotion witness → Execution numeric route**. 단항 함수를 모두 무오류 SIMD kernel로 취급할 수 없다. retry는 effect/order 안전성 검증을 요구한다. |
| Amend/scatter의 index·sparse·dtype·read-only·usecount 조건부 inplace | [`am.c` 55–89행](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/am.c#L55-L89) · [568–581행](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/am.c#L568-L581) | **Amend semantic access/error contract → guarded Scatter/Buffer planner**. 단순 Gather/View 규칙과 구분하며 alias 및 transactional assignment를 보존한다. |
| recursive/virtual block과 refcount·allocation lifetime | [`m.c` 743–783행](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/m.c#L743-L783) | **Physical allocator/representation 관리**. C refcount flag를 semantic Graph property로 복제하지 않는다. |

**적용 판정:** 단항 Mean의 오검출은 즉시 수정한다. 나머지 다섯 경로는 기존 frontend/runtime, semantic numerical contract, Scatter/Buffer planner의 source-evidence backlog로 둔다. 새로운 J Graph node/실행 최적화가 구현된 것으로 표시하지 않는다. `FOUNDATIONS.ko.md`의 semantic/physical 경계 원칙과 모순되지 않아 해당 파일은 변경하지 않았다. **이번 변경은 pinned C source와 Rust 코드의 정적 검토이며, Rust 테스트·jsource 차등 실행·benchmark는 아직 실행하지 않았다.**

<a id="l-jsource-후보--a3-logical-ir--기존-loweringregistry-연결-2026-10-06"></a>
##### L. jsource 후보 → A3 Logical IR → 기존 LoweringRegistry 연결 (2026-10-06)

**이번 이행 단위:** `src/lowering.rs::LoweringRegistry::jsource_planning_reports`는 각 `JsourceOpportunity`의 출처를 `candidate.verify(&j_graph)`로 다시 확인한 뒤, `Operation.j_origin`으로 연결된 **canonical A3 호출**을 찾는다. `OpKind::Basis`는 **이미 등록된 일반 execution-basis lowering**에 `legal_candidates`로 질의하고, `OpKind::SemanticCall`은 기존 semantic-call 경계로 표시한다. J Graph/A3의 source 및 node-count 정합성과 양쪽 verifier 검사를 선행한다.

| 새 보고 내용 | 소유권 및 검증 경계 |
|---|---|
| `JsourcePlanningReport`의 family/source ValueId/span/decision owner | 기존 `JsourceFamilyRule` 및 J Graph provenance 유지 |
| `JsourceLinkedCall`의 A3 `OpId` 및 현재 대상의 기본 route | **기존 코드 경로 조사**일 뿐 해당 jsource 기법의 특수화 지원을 뜻하지 않음 |
| `unresolved_proofs` | `ProofRequirement` 전체가 **미충족**. type/shape 또는 기존 CPU reference route만으로 하나도 자동 면제하지 않음 |
| `NeedsLogicalCallLink` | canonical A3에서 대응 호출이 없는 후보. 실행 연결을 생성하지 않음 |
| `NeedsSemanticProof` | 대응 호출을 찾아도 equivalence/ordering/guard/fallback/cost가 증명되지 않으면 **최적화 실행 불허** |

이 연결을 통한 **조건부 lowering의 첫 gate**는 `source candidate validation → canonical logical call mapping → existing route legality`다. 단, 이를 *optimized implementation 선택*과 혼동하면 안 된다. **현재 추가한 결과에는 jsource-specialized kernel 선택, guard 삽입, rewrite commit, physical buffer 결정이 없다.** 실제 specialized lowering은 J-visible numerical/rank/empty/error/binding/alias 증명, 실패 전 source fallback, target-resource-cost 확인을 별도 통과한 뒤에만 후속 구현한다. 순서는 `ProofRequirement`별 witness/guard 도입 → 해당 family의 parameterized lowering recipe → guarded A3/Physical 선택 → C differential/benchmark다.

`tests/lowering.rs`에 Reduce, 단항 Mean, Gather, IntervalLookup의 J Graph→A3 origin 연결, CPU/GPU 기존 route와 미증명 분리, 오래된/조작 후보 및 누락된 logical origin을 검증하는 **정적 회귀 테스트를 추가했다. 테스트를 실행했다는 뜻은 아니다.** CI·Cargo·jsource differential·benchmark는 이 변경에서 실행하지 않았다.

<a id="jsource-audit-m"></a>

<a id="m-jsource-재감사-기존-catalog-밖의-구조적수치적상태적-최적화-2026-10-06"></a>
##### M. jsource 재감사: 기존 catalog 밖의 구조적·수치적·상태적 최적화 (2026-10-06)

**감사 방법과 범위.** 같은 [jsource 고정 revision](https://github.com/jsoftware/jsource/tree/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc)에서 (1) \`ca.c\`의 \`@/@:/&:\` 조합 dispatch 및 \`cf.c\`의 hook/fork construction, (2) \`ao/cc/v/vi/vg/vrand/vx/vz/va1/vo/a.c\`의 실제 실행·reversion 경로, (3) RustJ의 \`j_graph_ir.rs\`, \`j_graph_jsource.rs\`, \`analysis.rs\`, \`logical_ir.rs\`, \`execution_semantics.rs\`, \`lowering.rs\` 경계를 **서로 다른 출발점으로 대조**했다. 앞선 A/H/K의 대표 최적화 목록에는 다음 구조·소유권이 개별 항목으로 **빠져 있거나 충분히 분리되지 않았다.** 원본 dispatch의 존재는 RustJ parser 지원, 의미론적 등가성 또는 성능 개선의 증명이 아니다.

| 추가 확인한 계열 | pinned jsource의 직접 근거 및 실제 제약 | RustJ 기존 framework에 넣을 위치 / 도입 판정 |
|---|---|---|
| **Oblique reduction + convolution** (\`f//.@:(g/)\`) | [\`ca.c\` 특수 dispatch](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ca.c#L367-L374) → [\`ao.c::jtpolymult\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ao.c#L109-L161). 두 atomic dyadic operand에 \`VFUSEDOK2\`가 필요하고 실제 구현은 dense·nonempty vector, dtype/operation 조합별 분기와 generic fallback, 정수 overflow 시 재계산을 갖는다. | **J Graph의 Oblique/Reduce composition 후보 → Execution basis의 Contract/Segment-Reduce 또는 명시적 oblique access map → target-specific convolution algorithm**. GEMM의 단순 별칭으로 등록하지 않음. **미구현.** |
| **Cut → Scan/Window → Raze의 중간 boxing 제거** | [\`ca.c\` 인식](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ca.c#L358-L368) · [\`cc.c::jtrazecut1/2\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cc.c#L983-L1031): 한정된 \`;.n\`, atomic scan, dense input 등만 사용. **원본 주석은 cut이 하나도 없을 때 특수 경로가 일반 경로와 결과 축이 다를 수 있음을 명시한다.** | **J Graph의 multi-region producer/consumer fusion 후보 → SegmentView + Scan/Window + ConcatAssemble → result assembly/materialization**. 무조건 법칙으로 사용 금지. **zero-cut 의미론·fallback을 먼저 J oracle로 확정할 때까지 보류.** |
| **8-bit 문자 치환 LUT** (\`y {~ x i. ]\`) | [\`cf.c\` fork 인식](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L59-L65) → [\`v.c::jtcharmap\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/v.c#L165-L191). byte LIT, 256-entry table, \`x=a.\` shortcut, 중복 매핑의 첫 일치 우선순위, index error의 generic fallback. | **J Graph의 IndexOf→Gather idiom → Lookup/Gather execution → Physical byte LUT**. alphabet/encoding·rank·invalid index·duplicate semantics 증명 전에는 일반 \`i.\`를 대체하지 않음. **미구현.** |
| **Boolean/sparse predicate → indices 직접 생성** (\`# i.@#\`) | [\`cf.c::jthkiota\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L221-L234): boolean atom/vector는 \`ifb\` 경로, sparse boolean 1D는 fill과 sparse axes를 검사한 후 저장된 nonzero index 활용. [hook의 Key 인접 특수화](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L329-L339)도 존재. | **Graph IndexSpace + Compact(mask)/GroupBy 후보 → Execution masked-index or sparse-index route**. 일반 \`i.@#\` 중간 배열 제거 여부는 span/valence·shape/empty/sparse fill 증명 후 결정. **미구현.** |
| **Grade→scalar Gather의 order-statistic 선택** (\`x {/:~ y\`, \`x {/: y\`) | [\`cf.c\` hook dispatch](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L337-L345) → [\`vg.c::jtordstat/jtordstati\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vg.c#L779-L800). scalar order index, int/float vector 및 길이 조건에 제한된다. **\`jtordstat\`은 내부 pivot을 위해 \`jtrollksub\`를 호출한다.** | **Graph Grade→Gather idiom → order-statistic semantic contract → Physical select/quickselect**. 동률·정렬·index/error뿐 아니라 **난수 상태의 관찰 가능성**을 독립 확인해야 한다. generic Grade sort를 무조건 선택 연산으로 바꾸지 않음. **미구현.** |
| **Shape와 RNG 생성의 결합** (\`?@#\`, \`?@$ \`, \`?.@#\`) | [\`ca.c\` 선택](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ca.c#L353-L359) → [\`vrand.c::jtrollksub/jtrollk\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vrand.c#L609-L678). boolean·power-of-two·일반 range에서 난수 생성 알고리즘과 RNG draw 경로가 달라지고, \`jtrollk\`는 rank/type에 따라 generic으로 복귀한다. | **J Graph의 Generate/shape-demand 후보 → stateful RNG runtime/Execution Generate route**. 효과·난수 스트림·seed/engine/version 경계가 우선이며, pure fusion·일반 reassociation에 합류시키지 않음. **미구현.** |
| **Box + Append/Raze의 direct link** (\`,<\`, \`;<\`, \`,&<\`) | [\`cf.c\` hook 인식](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L315-L326), [\`vo.c::jtjlink\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vo.c#L107-L144). 특정 box 형식·recursive/virtual 보존·\`WILLOPEN\`·in-place, self-containment 회피와 fallback을 명시한다. | **Graph box/append demand → ConcatAssemble/boxed result assembly → ownership/materialization planner**. \`Box→Open\` 일반 소거로 해석하지 않음. **미구현.** |
| **명시적 \`M.\` memo cache** | [\`a.c::jtmemo12/jtmemo\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/a.c#L93-L185). 0-rank 정수형으로 변환 가능한 인자에 대한 두 인자 key, lookup/insert lock, 공유된 결과를 소유하는 동적 table; 다른 인자는 매번 원 함수를 실행한다. | **J derived verb의 explicit memo semantics + Stateful cache runtime**, 필요시 guarded call-specialization. 임의 Graph CSE나 모든 noun을 key로 쓰는 자동 memo와 구별; cache hit 시 호출이 생략되는 **사용자 요청된 \`M.\` 동작**과 동적 이름·효과·수명을 보존. **미구현.** |
| **수치 상수·정확도별 별도 algorithm family** | [\`ca.c\` digits/\`^@o.\`/constant modular power](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ca.c#L367-L387); [\`vx.c::jtdigits10\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vx.c#L257-L290), [\`vz.c::jtexppi\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vz.c#L293-L305), [\`va2.c\` exponent 0.5·power-of-2 modulus](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/va2.c#L1937-L1940). \`jtexppi\`는 특정 complex half-turn에서 exact-zero component를 만들어 일반 경로와 다르게 처리한다. | **Graph bound constant/composed numeric idiom → semantic numeric/promotion/fit witness → typed CPU/GPU specialized routine**. 수학적 항등식만으로 bitwise/오류 순서 등가를 주장하지 않음; backend routine은 새 Graph semantic op가 아니다. **미구현.** |
| **Hook의 비교·threshold 직접 실행** | [\`cf.c\` \`(compare |)\` 및 \`compare L.\` 인식](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L319-L333), [\`cf.c\` \`jtdeadband\` dispatch](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L105-L115), [\`va1.c::jtdeadband\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/va1.c#L440-L462). latter는 AVX2/EMU_AVX2 guard, float scalar threshold/dense-nonempty 조건 및 fallback을 가진다. | **Graph Hook-comparison candidate → typed semantic comparator/threshold contract → target-feature-gated SIMD**. tolerance, NaN, negative-zero, rank/effect 보존. **미구현.** |

**이미 H절에 있는 GroupAggregate/GEMM/Grade/Search/virtual/sparse와 중복 등록하지 않는다.** 이 표는 그 범주에 감춰졌던 **서로 다른 operator topology, legality, target/runtime mechanism**을 드러내는 출처 목록이다. 새로운 family ID를 \`JSOURCE_FAMILY_RULES\`에 자동으로 늘리거나 파싱되지 않는 J 구문을 조작해 applied Graph 후보를 생성하지 않는다. 소스 근거 등록 시에도 \`AwaitingFrontendOrFacts\` 또는 \`DownstreamOnly\` 상태로 시작한다.

<a id="m1-rustj-프레임워크-수정-필요성-판정"></a>
###### M.1 RustJ 프레임워크 수정 필요성 판정

**판정: canonical compiler architecture의 전면 변경은 불필요하다. 하지만 아래 경계의 국소적 확장은 최적화 실행을 시작하기 전에 필요하다.**

| 기존 구현에서 확인한 상태 | 판정 | 최소 변경 또는 유지할 원칙 |
|---|---|---|
| \`FunctionEntity\`, \`J Graph IR\` region, \`A3 Logical IR\` 및 \`ExecutionBasisKind::{Contract, Scan, GroupBy, Grade, Gather, ConcatAssemble, ...}\`가 분리되어 있음 | **재설계 불필요** | 새로운 C 특수 진입점별 Graph/SSA node를 만들지 말고 고수준 J topology와 기존 basis를 재사용한다. \`SegmentView/GroupBy/Contract\`의 \`ExecutionBasisPayload::Deferred\` 세부 payload는 실제 semantic 지원과 함께 보강한다. |
| \`JsourceOpportunity\`는 source \`ValueId\`·span·basis/facts를 보유하고, \`jsource_planning_reports\`는 A3 \`Operation.j_origin == source_value\`를 연결함 | **작지만 필요한 확장** | Cut–Scan–Raze, convolution 같은 **다중 연산/중첩 region 후보**를 위한 sidecar \`SourceAnchor\`(해당 graph version, optional \`RegionId\`, 원래 source value, A3 ordered operation set)와 검증기 설계. 동일 \`ValueId\`를 공유한 중첩 region을 오인하지 말 것. parser \`FunctionEntity\` identity는 그대로 둔다. |
| \`JsourcePlanningState\`는 \`NeedsLogicalCallLink/NeedsSemanticProof\`까지만 구별하고 \`unresolved_proofs\`는 family 전체를 반환; \`OpportunityLegality\`는 단일 \`AwaitingSemanticProofs\` | **실행 이전에 필요** | 기존 §4.1.4의 **obligation별 \`ProofBundle\`/witness/guard/failed state**를 실행 가능한 후보에 구현한다. source graph/version, valence, rank, empty/prototype, \`!.\`, numerical error, target feasibility를 각각 증명한다. \`selected\` bool 또는 합법적 baseline route만으로 증명 완료를 추정하지 않는다. |
| \`EffectSummary\`는 현재 \`Pure/Unknown\` 두 값뿐이며, \`M.\`, \`?\`, \`jtordstat\`의 RNG/cache와 error-control은 별도 상태 관계가 필요함 | **지원 시 국소 확장 필요** | \`StateResource\`/ordered effect dependency에 RNG state·explicit memo table·binding invalidation을 연결하는 규약을 추가한다. 단지 RustJ 수치 함수로 보인다는 이유로 RNG 소모·cache hit 부작용을 지워서는 안 된다. **지원 전에는 Unknown/RuntimeSemantic fallback**. 모든 현재 OpKind를 memory/effect SSA로 즉시 변경할 필요는 없다. |
| \`Logical IR\`은 \`SemanticCheck\`와 constraint/error origin을, 후속 계층은 target/cost/resource를 분리함 | **계층 유지** | zero-cut 결과 축·빈 segment/fill, sparse boolean fill, byte LUT error, random draw ordering의 semantic guard를 **effect 전에** 검사한다. guard miss는 지정된 J semantic route로, effect 이후 재시작은 금지. vector width, byte table, cache block, buffer ownership은 Physical Planner에서만 선택한다. |

**특별한 반례 두 개를 구현 gate로 고정한다.**

1. [\`cc.c::jtrazecut2\` 주석](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cc.c#L983-L993)은 **zero-cut에서 최적 경로와 일반 경로의 결과 축이 달라질 수 있다**고 명시한다. 따라서 jsource의 특수 구현을 **equivalence oracle로 사용하면 안 된다**. J 언어에서 요구되는 실제 결과·fill/assembly 정책을 독립적으로 확정하고 두 경로의 동작을 차등 분석해야 한다. 미해결이면 결합 최적화는 불허한다.
2. [\`vg.c::jtordstat\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vg.c#L779-L790)은 내부 pivot 생성에 \`jtrollksub\`를 부른다. 해당 RNG 상태 변화가 이후 J 호출에 관찰되는지 **확인하기 전**에 "Grade→Select는 pure rewrite"라고 선언하지 않는다. 난수 state를 관찰 가능하다고 확정한 경우에는 trace/seed 동등성 또는 J 사양상 허용된 차이를 별도로 증명해야 한다.

<a id="m2-이행-순서검증-범위"></a>
###### M.2 이행 순서·검증 범위

1. **M2/front-end coverage 우선 유지:** \`/. /.., ;., M., ?/?.\` 및 hook/fork/grade 조합의 정확한 POS·derived construction/valence·fallback을 먼저 인식한다. 아직 미지원인 형태는 \`AwaitingFrontendOrFacts\`.
2. **원본 출처와 J semantics 분리:** source form/region anchor와 expected Logical operation set을 남기고, 효과·RNG·cache·error/check dependency 및 숫자/비어 있음/shape 증명 의무를 정의한다.
3. **한 family씩 proof·guard·fallback:** 우선 byte-LUT 또는 boolean-index처럼 경계가 명료한 경로를 대상으로 source reference route와 optimized recipe를 동시에 비교한다. zero-cut·RNG·memo는 effect/assembly 증명 전 보류한다.
4. **후속 실현:** \`LoweringRegistry\`에 evidence를 소유한 parameterized recipe를 등록한 뒤 target/resource/cost, candidate overlap, 선택과 Physical Planner/Executor를 순차 연결한다.
5. **검증:** 각 특수 경로의 positive/negative guard, type/rank/empty/sparse/boxed/\`!.\`/tolerance/NaN/overflow/error precedence, RNG seed/trace, named binding 및 replay safety를 J/C differential과 Rust tests로 확인한다. **이번 M절은 source/architecture 정적 감사다. Cargo/CI/jsource 실행·benchmark를 수행하지 않았고, 확인한 10개 계열의 RustJ 구현·의미 등가성·성능을 주장하지 않는다.**

**미조사 잔여 범위:** assembly/architecture-specific microkernels 전수, 모든 primitive dispatch·runtime allocator, feature flags별 빌드 차이 및 현재 master 전체에 대한 일대일 diff는 완료하지 않았다. 이번 감사 결과를 "jsource 최적화 누락 없음" 또는 full-J support로 해석하지 않는다.

<a id="jsource-index-family"></a>

<a id="n-roger-hui의-index-of-family와-rustj-단계적-구현-2026-10-06"></a>
##### N. Roger Hui의 Index-Of family와 RustJ 단계적 구현 (2026-10-06)

**연구·원본 근거.** Roger Hui, *Index-Of, A 30-Year Quest* (J Conference, 2014; [후대의 서지 기록](https://www.sigapl.org/Articles/APL%20Since%201978_3386319.pdf)) 및 *Hashing for Tolerant Index-Of* ([Jsoftware 논문](https://www.jsoftware.com/papers/Hashing.htm), 2010). RustJ 설계의 실행 근거는 별도로 [고정 jsource \`vi.c\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vi.c#L140-L185), [\`viavx.c\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx.c), [\`viavx2.c\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx2.c), [\`visp.c\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/visp.c)의 dispatch/실행/guard이다. 발표의 역사적 중요성과 소스의 구현 상태를 구분한다.

**의미론상 연산 family / 알고리즘 family 구별.** J의 \`x i. y\`(first), \`x i: y\`(last), \`x e. y\`(membership), \`~. y\`(nub), \`~: y\`(nub sieve), \`x -. y\`(less), \`I.@e.\`(indices), \`u/. y\`(Key classification) 등은 equality/search를 공유할 수 있지만 **output convention, representative selection, rank/cell/frame, empty/prototype, tolerance/fit**은 서로 다르다. \`I.\` **이항**은 sorted interval index로 별도 \`IntervalLookup\`; \`E.\`는 연속 부분배열 window match이고 \`FindViaWindowMatch\`가 기존 소유자다. \`E.\`를 일반 \`i.\` 해시 후보로 분류하지 않는다. \`i. 4\`·\`i: 4\`의 단항 Generate도 dyadic 검색과 다른 의미다.

| jsource의 공유 검색 경로 | 선택의 관건 | RustJ 프레임워크 소유 계층 |
|---|---|---|
| Sequential scan | 아주 작은 검색 문제에서 해시 구축비 회피; [\`vi.c\` 후보 판정](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vi.c#L1113-L1148) | Execution algorithm/초기 CPU reference |
| Dense direct indexing, boolean/byte, bit-packed 및 좁은 integer range | \`[min,max]\` 값 범위, index-vs-presence table 폭, 초기화 비용·캐시 한도; [\`vi.c\` small-range 선택](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vi.c#L1148-L1238) | Physical algorithm과 cost, logical membership/position 분리 |
| General hash (first/last), reverse hashing | 입력·질의 상대 크기, duplicate representative, 해시 방식 및 저장 용량; [\`viavx.c\` reverse 선택](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx.c#L738-L850) | Execution algorithm/Physical strategy |
| Tolerance-aware float/complex/boxed search | \`!.ct\`/runtime \`cct\`, **근사동등성의 비추이성**, +0/-0, NaN, 두 인접 bucket 후보·exact insert vs tolerant probe; [\`viavx2.c\` dual-bucket 구현](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx2.c#L8-L78) | Semantic equality witness **후** target-specific implementation. 단순 표준 \`HashMap<f64, ..>\` 불가 |
| Boxed sort→binary search fallback | 실제 jsource \`jtiobs\`는 \`ct=0\`과 제한된 boxed 구성만; [\`vi.c\` guard](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vi.c#L735-L840) | Target algorithm, stable representative/비교 관계 증명 |
| Sparse search 및 Key self-classification | sparse fill/axes/empty, grouping first-occurrence order; [\`visp.c\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/visp.c#L68-L106), [\`ao.c::jtkeyct\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ao.c#L213-L255) | \`LookupClassify\` / \`GroupBy\` → sparse/group route |
| Prehash 및 result-mode fused loops | 재사용하는 dictionary의 key equality/rank/tolerance/version, saved-table lifetime, output \`first/last/presence/compact/count/any/all\`; [\`vi.c\` mode 목록](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vi.c#L140-L185) | A3 result intent/provenance → cache/algorithm planning; grouped/count output은 별도 유효성 필요 |

<a id="n1-첫-구현-단위--기존-ir-변경-없이-실행-경로-개선"></a>
###### N.1 첫 구현 단위 — 기존 IR 변경 없이 실행 경로 개선

2026-10-06 코드 연결:

- \`src/index_ops.rs\`: 내부 \`LookupResult::{First,Last,Membership}\`과 공통 \`lookup(indexed,queries,result)\`로 검색 의도와 materialized output을 분리한다. \`e.\`는 \`i.\`의 정수 위치 배열을 만들지 않고 Boolean 결과를 **직접 생성**한다. 출력 rank·frame과 불일치 셀의 missing/presence semantics는 기존 경로를 보존한다.
- 같은 파일: **integer/bool scalar**만 \`ExactScalarIndex::Direct\` 또는 \`Hashed\`를 허용한다. 최대 직접 테이블 **65,536 entry**, \`span <= 4 × (indexed items + queries)\`의 단순 memory/work heuristic, \`items × queries <= 32\`면 순차 검색. key span 계산은 \`i128\`으로 overflow를 회피하고, 최초/최후 일치 위치·not-found sentinel을 유지한다. **이 threshold는 측정된 최적 비용이 아니라 보수적인 시작값**이다.
- Float/complex/boxed/cell 전체 검색은 새 hash/direct 경로를 열지 않고 기존 \`atom_eq\` sequential fallback을 사용한다. 특히 비추이적 tolerant equality를 일반 key-based hash로 바꾸지 않는다.
- \`src/j_graph_jsource.rs\`: dyadic \`i:\`를 \`SearchAlgorithm\` 후보에 추가, \`E.\`는 제외하고 이미 있는 \`FindViaWindowMatch\` rewrite 책임을 유지. \`I.\` dyad와 단항 \`i./i:\`의 분리는 유지. candidate \`AwaitingSemanticProofs\`/not selected 정책 불변.
- \`tests/index_ops.rs\`·\`tests/j_graph_jsource.rs\`, \`src/index_ops.rs\` 내부 테스트에 scalar first/last/membership, negative/duplicate/missing, direct-vs-hash/extreme \`i64\`/small-query, Graph 후보 구분 회귀 사례를 추가했다. **이번에는 테스트를 실행하지 않았으며 정적 연결만 점검**한다.

이 변화는 **CPU reference implementation의 제한된 알고리즘 개선**이지 jsource의 모든 searching mode를 구현했거나 Graph 후보에 최적화 실행 권한을 준 것이 아니다. \`JSOURCE_FAMILY_RULES\`의 \`SearchAlgorithm\` 및 \`TolerantHash\`를 그대로 사용하며 새 Graph basis를 추가하지 않는다.

<a id="n2-다음-구현-gate--프레임워크-재설계-필요성"></a>
###### N.2 다음 구현 gate — 프레임워크 재설계 필요성

**N.2의 초기 확장 계획 중 기초 단계는 O절에서 구현되었다.** A3 `LookupClassify { search: SearchDescriptor }`가 first/last/membership/interval/self-classify 및 indexed/queried origin을 보유한다. 그러나 `!.ct`의 **runtime comparison-policy/version witness**, group/compact/count output, sortedness/uniqueness 및 prepared-lookup key의 완전한 의미론적 증명과 target-independent `CandidateEvidence`는 여전히 후속이다. `FunctionEntity`의 원래 구성·dynamic tolerance·Rank/CellApply 경계를 유지한다.

1. **Proof/semantics:** first/last/Key의 최초 등장 순서·rank/cell/frame/empty, 타입 혼합/box/sparse/tolerance/fit/error precedence를 J/C로 검증한다. 런타임 tolerance 변경은 cached hash를 무효화하거나 key에 포함한다.
2. **Algorithm extension:** 직결 출력의 경우에만 중간 materialization 제거 증명; 작은 범위의 1/2/4-byte packed/presence table, reverse hashing, 재사용 가능한 prehash를 **비용/메모리·alias·version guard와 함께** 하나씩 도입한다. GPU route는 전송·local memory·불균일 충돌 등의 physical resource 예산이 먼저다.
3. **Tolerant hashing 별도 연구 gate:** upstream 두 인접 interval 알고리즘을 그대로 이식하지 말고 \`cct\`/0/NaN/boxed·비추이성·일치 우선순위 증명을 갖춘 \`LookupEqualityWitness\` 아래서만 구현한다. guard miss는 효과 이전 generic J search로 돌아간다.
4. **선택/평가:** \`JsourcePlanningReport\`의 proven/unproven 구분과 결합, \`LoweringRegistry\`의 algorithm-route capability/target/empirical cost를 분리한다. 고정 65,536·32·4× 값은 benchmark와 실측 캐시 profile 전까지 튜닝값으로 취급한다.
5. **검증:** Rust reference, C J oracle, mixed rank/empty/sparse/box/exact+tolerant/negative-zero/large cardinality 및 differential/bench를 통과한 후에만 추후 search fast path의 일반 적용을 승인한다. **CI·Cargo·차등 실행·성능 측정은 이번 연결 작업에서 실행하지 않았다.**



<a id="n3-두-번째-구현-단위--reverse-hashing과-per-engine-prehash-2026-10-06"></a>
###### N.3 두 번째 구현 단위 — Reverse Hashing과 per-Engine Prehash (2026-10-06)

**대응 근거.** [jsource \`viavx.c::indexofsub\`의 reverse hash 선택](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx.c#L738-L850)은 indexed/query cardinality와 지원 모드에 따라 반대 방향에 검색 테이블을 세운다. [\`vi.c\`의 prehash mode](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vi.c#L140-L185)는 파생 함수의 저장 테이블과 특정 비교 계약을 포함한다. 아래 RustJ 구현은 그 **기본 개념을 제한적으로 차용**한 것이며 C의 prehashed derived verb 전체를 재현하지 않는다.

| 구현 항목 | 실제 경로와 조건 | 보존 계약 / 제한 |
|---|---|---|
| **질의측 Reverse Hashing** | \`src/index_ops.rs::reverse_exact_index\`: indexed 원소 ≥64, \`indexed.len()/2 > query count\`, 단항 셀 \`Int/Bool\` 양측만. 작은 **질의 집합**의 distinct key를 \`HashMap\`에 넣고 원 indexed 배열을 한 번 스캔 | First/Membership은 앞→뒤, Last는 뒤→앞. distinct query key가 모두 해결되면 조기 종료. 중복 질의·없는 값은 각각 같은 position/not-found sentinel을 받는다. float tolerance/boxed/multicell은 제외 |
| **저장소 identity 기반 per-Engine Prehash** | \`ExactPrehashCache\` 하나를 \`Engine\` 내부에 유지. 양측 exact \`Int/Bool\`, indexed rank=1, 64~16,384 items, indexed backing이 \`CpuStorage::Shared(Arc<_>)\`일 때만 사전 빌드. \`i.\`/Membership은 First 공용, \`i:\`는 Last 별도 | **같은 Arc allocation + dtype/shape + First/Last 모드**가 맞아야 hit. 원 source \`Value\`를 공유 clone으로 cache에 보유하여 포인터 재사용/수명 혼동을 차단. binding 재정의로 새 storage가 생기면 재빌드. 오직 한 table만 보존하므로 메모리 상한 제한 |
| **실행 경로 분리** | \`src/runtime.rs::Engine::interpret_ir\`에서 \`pooled\`인 일반 primitive dyad \`i.\`/\`i:\`/\`e.\`의 exact scalar 데이터만 cache-aware entry로 보내며 기타 입력·명시 Rank는 기존 \`kernels::dyad\`로 복귀 | \`eval_semantic_reference\`는 cache 사용하지 않음. \`index_prehash_stats() -> (builds,hits)\`, \`clear_index_prehash()\`는 J value 의미를 바꾸지 않는 관측·clear API |
| **테스트** | \`src/index_ops.rs\` unit: reverse 중복/순서·direct/hash/sequential 참조 대조·Arc identity·rebind; \`tests/index_ops.rs\` integration: 실제 Engine의 cache hit/miss, 멤버십 공유, Last 분리, 새 binding, reference 경로 및 temporary reverse | **작성만 완료.** Cargo/CI/C differential/benchmark 미실행; 정적 연결 확인과 실행 성공을 구분 |

**메모리 안전 fallback:** 선택적 Reverse/Direct/Hash/Prehash 테이블의 `Error::Limit`(준비용 메모리 확보 실패)은 최적화 불허로 처리하여 원래 순차 검색으로 복귀한다. 최종 결과 버퍼의 실제 할당 실패는 별도의 J 실행 오류로 유지한다.

**프레임워크 판정:** canonical \`J Graph → A3 LookupClassify → Physical Planner\`에 새 IR node를 추가하지 않았다. 이 첫 CPU 실행 경로의 비용 threshold(64, 2:1, 16,384)는 실험으로 최적화한 값이 아닌 보수적 초기값이다. 현재 cache는 physical interpreter-local 구현이고 compiler \`JsourcePlanningReport\`의 semantic witness, \`LoweringRegistry\` target-route/cost 선택, guarded optimized transform에 연결되었다고 주장하지 않는다.

**추가로 필요한 것:** \`m&i.\` 또는 \`e.&n\`처럼 J의 derived verb 수준으로 prehash 수명을 관리하려면 별도 \`PreparedLookup\`/versioned dictionary identity/semantic equality contract와 guard/miss fallback을 A3 sidecar에 구현해야 한다. 일반 이름/locale·\`!.ct\` tolerance 변경·sparse/boxed까지 이 캐시를 무조건 확대하면 안 된다. Prehash 준비 과정에서 관찰 가능한 error/effect를 건너뛰지 않는다는 증명도 선행해야 한다. 우선 실제 J/C 출력 차등 및 작은/큰 query 비용 측정 후 threshold와 API를 확정한다.


<a id="algorithm-planning-migration"></a>

<a id="o-타-프레임워크-대조를-통한-algorithm-planning-확장--i-family-첫-적용-2026-10-06"></a>
##### O. 타 프레임워크 대조를 통한 Algorithm Planning 확장 — i. family 첫 적용 (2026-10-06)

**프레임워크 결정:** jsource의 C 전용 특수 함수를 개별 J Graph IR 노드로 복제하거나, 그 선택을 모두 `src/index_ops.rs`에 넣지 않는다. `J semantic identity → A3 resolved operation/meaning → target legality + proof/guard evidence → Physical cost/choice → guarded executor/fallback`으로 분리한다. **이 구조는 검색에 대한 첫 구현이며 전체 optimizer의 SelectionPlan/자동 튜너 완성을 의미하지 않는다.** `PROJECT.ko.md`가 정본이고 `PROJECT.md`는 같은 구조의 영문 mirror다.

<a id="o1-비교한-프레임워크와-채택비채택-사항"></a>
###### O.1 비교한 프레임워크와 채택·비채택 사항

| 검토한 프레임워크·근거 | 해결 방식 (원본 근거) | RustJ에 채택한 원칙 / 하지 않은 것 |
|---|---|---|
| **MLIR Dialect Conversion** ([공식 Dialect Conversion](https://mlir.llvm.org/docs/DialectConversion/)) | `ConversionTarget`가 Legal/Dynamic/Illegal을 분리하고, op/operand/type별 합법성을 판정. `partial conversion`은 나머지를 유지 가능 | **채택:** `SearchAlgorithmReadiness`에서 ordinary baseline, exact-scalar runtime guard, 의미론 증명 필요, target 불가, 다른 연산을 분리한다. 특정 자료형을 안다고 `TolerantNeighborHash`를 승인하지 않는다. **비채택:** J binding/verb identity를 MLIR dialect로 교체하지 않음 |
| **MLIR Transform dialect** ([공식 문서](https://mlir.llvm.org/docs/Dialects/Transform/)) | transform/control IR을 payload IR과 분리하고 recoverable/irrecoverable 실패를 구별 | **채택:** 분석 및 선택 결과는 canonical A3를 변경하지 않는 별도 report/plan으로 유지; guard 실패와 invalid transform을 별개로 취급. **비채택:** Transform dialect IR 그대로 도입하지 않음 |
| **IREE Flow→Stream→HAL / Codegen lowering config** ([파이프라인](https://github.com/iree-org/iree/blob/main/docs/website/docs/developers/general/developer-tips.md), [Codegen LoweringConfig](https://iree.dev/reference/mlir-dialects/IREECodegen/), [LLVM CPU pass](https://iree.dev/reference/mlir-passes/CodegenLLVMCPU/)) | execution/dispatch와 device/backend translation 전략, tiling/vector/bufferization을 구분 | **채택:** A3 `SearchDescriptor`에 hash table 크기, buffer, SIMD, GPU device를 넣지 않으며 target 선택은 Registry/Physical로 미룬다. **비채택:** 지원하지 않는 GPU 검색 커널을 형식상 등록하지 않음 |
| **Apache TVM MetaSchedule** ([공식 tutorial](https://tvm.apache.org/docs/deep_dive/tensor_ir/tutorials/meta_schedule.html)) | `SpaceGenerator → SearchStrategy → CostModel → Builder/Runner → Database`를 분리해 실제 HW 측정 결과로 schedule을 고른다 | **채택:** 합법적인 algorithm 후보/비용 입력/선택 기록을 서로 다른 개념으로 유지. 현재 `SearchPhysicalChoice`의 비용은 **측정값이 아닌 초기 크기 기반 heuristic**. **비채택:** 아직 자동 탐색·벤치 DB·예측 모델은 구현하지 않음 |
| **XLA GPU Priority Fusion / Cost Model** ([설계 설명](https://github.com/openxla/xla/discussions/10065), [pass 원본](https://github.com/openxla/xla/blob/main/xla/backends/gpu/transforms/priority_fusion.h)) | fusion의 합법성과 별도로 estimated `time(unfused)-time(fused)`, memory/compute/launch overhead 등을 근거로 우선순위 결정 | **채택:** semantic proof와 profitability, target feasibility/임시 메모리 사용을 섞지 않음. **비채택:** GPU cost 추정을 임의 상수로 수치화하지 않음 |
| **Futhark SOAC / incremental flattening** ([2026년 개발 설명](https://www.futhark-lang.org/blog/2026-07-31-full-flattening.html), [scan-scatter fusion](https://www.futhark-lang.org/blog/2026-03-24-scan-scatter-fusion.html)) | 고수준 array/dataflow를 보존하고 다양한 flatten/fuse/순차 실행 버전을 target/workload에 따라 선택 | **채택:** 여러 낮은 수준 알고리즘으로 내릴 수 있는 고수준 `LookupClassify`/향후 `GroupBy`·`Reduce` 구조 유지. **비채택:** J의 오류 순서·동적 binding/tolerance 없이 functional fusion 동치식을 사용하지 않음 |

<a id="o2-실제-코드로-반영한-세-경계"></a>
###### O.2 실제 코드로 반영한 세 경계

1. **Execution Semantic Lowering / A3:** `src/logical_ir.rs`의 `ExecutionBasisPayload::LookupClassify { search: SearchDescriptor }`에 **original primitive와 valence에서 도출한** `SearchOutputKind::{FirstIndex,LastIndex,MembershipMask,IntervalIndex,SelfClassify}`, `SearchComparison::{JEquality,JOrderedInterval}`, indexed/queried SSA `ValueId`, original `rank_boundary`를 보유한다. `i.`/`i:` 및 이항 `I.`는 **왼쪽이 indexed set, 오른쪽이 query**이고, 이항 `e.`는 **오른쪽이 indexed set, 왼쪽이 query**다. A3는 이 방향을 이미 바르게 기록하며 Physical strategy가 인수 방향을 추측하지 않는다. 모호한/미지원 derived target은 `Deferred`로 남기고 새 실행 의미를 만들지 않는다. `A3_SCHEMA_VERSION`을 **0.4→0.5**로 올렸고 `Plan::verify`는 payload와 현재 `CallOp`이 일치하지 않는 변조를 거부한다. `JEquality`는 **runtime cct/`!.`를 준수할 의미**이지 float bit-exact hash 허가가 아니다.
2. **LoweringRegistry:** `src/lowering.rs`에서 기존 `ExecutionBasisKind::LookupClassify`에 **ordinary pure CPU reference** route를 등록하고, `SearchAlgorithm::{Sequential,DirectAddress,IndexedHash,ReverseQueryHash,PreparedHash,TolerantNeighborHash}`를 별도 알고리즘 후보로 열거한다. `search_algorithm_reports`는 `SearchDescriptor`에서 `Baseline/RequiresExactScalarGuard/NeedsSemanticProof/UnsupportedTarget/UnsupportedSearchForm` 상태를 계산한다. 현재 다른 J search form, GPU route, TolerantNeighborHash에 대해 근거 없는 실행 가능 상태를 만들지 않는다. `JsourceLinkedCall.search_algorithms`에도 같은 Registry 보고서를 노출하지만 `NeedsSemanticProof`와 candidate 미선택 상태는 그대로다. 기존 `JsourcePlanningReport`의 source provenance 및 미충족 `ProofRequirement`는 **그대로 유지**하며 실행 승인으로 승격시키지 않는다.
3. **Physical Search Planner:** `src/physical.rs::plan_search_algorithm`은 실제 runtime/target에서 얻은 `SearchWorkload`(indexed/query 수, `integer_span`, immutable shared index, prehash eligibility, reverse용 query 접근)를 받고 Registry에 후보 상태를 재질의한다. 순차, 좁은 정수 범위 direct, indexed hash, reverse hash, prepared hash를 provisional bound(32 comparisons; direct 최대 65,536 entries 및 `4×(items+queries)`; reverse 최소 64 indexed 및 크기비 2:1; prepared 64~16,384 immutable shared)에 따라 고른다. output의 index 위치/Boolean mask 의미는 변경하지 않고, `SearchPhysicalChoice`에는 selection basis와 estimated **table entries**만 보고한다. 이는 byte-accurate allocation, benchmarked cost, final BufferId/device schedule이 아니다. `src/index_ops.rs`는 경계에서 Int/Bool 단항 item/runtime dtype을 검사한 다음 planner 결정대로 table을 준비한다. **성능 경계:** 런타임은 매 검색마다 Registry 전체나 후보 Vec를 생성하지 않으며, `LoweringRegistry::search_algorithm_readiness`를 할당 없이 질의한다. 컴파일러 진단용 `search_algorithm_reports`만 후보 목록을 만든다. 선택적 table `Error::Limit`는 순차 fallback으로 되돌리지만 결과 버퍼 오류는 유지한다.

**경계 반례:** 근사 동등성은 비추이적일 수 있어 `!.ct`/floating/boxed를 exact hash로 보내지 않으며 `TolerantNeighborHash`는 `NeedsSemanticProof`. `I.` interval은 Index-Of 일반 hash 후보가 아니다. unknown J name/POS/locale·effect/error ordering·empty prototype은 sidecar/guard 없이 우회하지 않는다. 이 선택기는 **CPU interpreter exact scalar에 대한 검증 전제 실행 경로**이며 `J Graph candidate commit`, `PhysicalArray` 생성, GPU backend, 일반 A3-to-native codegen과 동일하지 않다.

<a id="o2a-동작-예시--3-1-3-i-3-4"></a>
###### O.2a 동작 예시 — `3 1 3 i: 3 4`

- **J Frontend / Graph IR:** 원문의 dyadic `i:`(Last index) semantics, 3의 마지막 위치 및 4의 not-found sentinel을 보존한다.
- **A3:** `LookupClassify { search: SearchDescriptor { output: LastIndex, indexed: left SSA ValueId, queried: right SSA ValueId, comparison: JEquality, .. } }`. 아직 해시 알고리즘이 아니다.
- **Registry:** CPU `Sequential=Baseline`, `IndexedHash/DirectAddress/ReverseQueryHash/PreparedHash=RequiresExactScalarGuard`, `TolerantNeighborHash=NeedsSemanticProof`로 보고한다.
- **Runtime/Physical:** 양측 Int/Bool 단항 셀임을 실행시 확인하며 `3 × 2 <= 32`이므로 순차 검색을 택한다. 예상 결과는 `2 3`이다. 같은 의미의 큰 입력에서는 검색 크기·범위·prehash 조건에 따라 다른 알고리즘을 고를 수 있지만, 의미론적 Last/missing은 불변이다.
- **검증 경계:** 해당 입력에 대한 A3/verifier·Registry·Physical 계획 테스트는 **추가만 했고 실행하지 않았다**. 이 예시는 코드 의도와 J 의미의 계약을 보여줄 뿐 실행 성공/성능 측정 증거는 아니다.

<a id="o3-앞으로-공통화할-설계-계약과-검증"></a>
###### O.3 앞으로 공통화할 설계 계약과 검증

이 첫 search-specific adapter에서 확인한 **재사용 가능한 최적화 질문**은 다음과 같다.

~~~text
SemanticDescriptor (A3: meaning, input/value origins, comparison, rank)
  ↓
AlgorithmCandidateSet (Registry: target legality, proof obligations, runtime guards)
  ↓
VerifiedRuntimeFacts (or proven static witnesses; no Unknown→true)
  ↓
CostProfile / ResourceBudget (Physical: work, bytes, occupancy/cache/transfer)
  ↓
SelectionPlan (separate from semantic IR; overlap + fallback)
  ↓
CommittedLowering (only witnessed/guarded, preserves errors/effects)
~~~

- **현재 구현:** 공통 보고서 자료형 `AlgorithmCandidate<Algorithm, Readiness>`와 검색 특화 alias `SearchAlgorithmReport`를 추가했다. 다만 Reduce/GroupBy/Grade/Contract용 **실제 후보 registry·obligation별 witness·공통 CostEstimate/SelectionPlan**으로 일반화하는 것은 **M2/M3 의미론 및 M4 기준선 검증 후, 둘 이상의 독립된 연산 가족에서 공통 계약이 실제로 요구될 때의 후속 작업**이다. Reduce/Scan의 reassociation, GroupBy의 equality/representative, Grade의 tie/order, Contract의 numeric accumulation/precision은 **각기 다른 semantic witness**를 요구한다. `LookupClassify` 전용 enum을 이들에 억지로 재사용하지 않는다.
- `SearchDescriptor`는 A3 J meaning만 표현하고, 향후 `ComparisonPolicy`를 `cct`/`!.` version guard가 붙은 contract로 구체화한다. Rank/CellApply가 외부 layer인 경우는 nested basis와 A3 call instantiation을 함께 검증한다. prehash 및 locale/name binder는 runtime key/version/lifetime에 귀속한다.
- target별 등록과 선택은 독립 검증한다: GPU/XLA식 cost model은 **GPU 실제 realization과 hard resource feasibility 등록 후에만** 합법 후보를 비교한다. out-of-budget/Unknown을 단순히 느린 비용으로 처리하지 않는다.
- 회귀 검사에 A3 `SearchDescriptor`의 mode·indexed/probe origin 및 forged payload 거부, Registry의 target·proof 상태, Physical의 5개 전략/false guard/GPU/Interval 차단을 추가했다. 기존 `i.` first/last/membership/empty/duplicate 및 prehash rebinding 테스트는 별도로 유지한다. **이번 변경에서는 Cargo·CI·jsource differential·benchmark를 실행하지 않았으므로 결과의 실행 검증과 성능 개선을 주장하지 않는다.**



<a id="o4-검색-계열을-통한-공통-프레임워크-역설계--독립-4관점-재검토-2026-10-06"></a>
###### O.4 검색 계열을 통한 공통 프레임워크 역설계 — 독립 4관점 재검토 (2026-10-06)

**검토 질문과 판정.** jsource `i.` family를 *어떻게 해시로 구현할까*보다 **왜 한 의미론적 연산에 여러 실행 알고리즘·결과 소비 형태·재사용 수명이 존재하며, RustJ 전체가 이를 어떤 경계로 표현해야 하는가**의 사례로 사용한다. **프레임워크의 큰 층 구분은 이미 옳다. 하지만 ‘검색 의미론 ↔ 후보의 증거 ↔ 자료구조 준비/수명 ↔ 비용/선택 ↔ 독립 실행 기준선’ 사이의 실행 가능한 연결 계약이 미완성**이다. 별도 검색 전용 IR, `vi.c` 모드별 연산, 즉각적인 범용 optimizer 또는 새로운 공통 crate를 추가하지 않는다. 기존 §O.3과 §4.1.4의 모델을 **구현 가능한 소규모 인터페이스 계약**으로 점진적으로 닫는다.

**검토 A — 원본 의미론에서 독립 출발.** pinned [jsource `vi.c`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vi.c)는 `IIDOT`(first), `IICO`(last), `IEPS`(membership), `INUB/INUBSV`(nub·nub sieve), `IIFBEPS`(`I.@e.`), 관련 집계/마스크 및 `IPH...` prehash 모드를 공통 검색 내부 엔진의 변형으로 다룬다. **재사용할 관찰:** domain을 검사하는 계산, 비교 정책, source-order 대표 위치, 결과 요구(output demand), 재사용 가능한 탐색 상태가 서로 구분된다. **재사용하지 않을 것:** C의 mode bit/entrypoint, 해시 배치, SIMD 분기, `viavx2.c`의 tolerant bitmask를 J 언어 의미나 Graph IR 구조로 고정하는 일. 특히 `i.`/ `i:`의 단항 생성형, `I.` 이항 interval, `E.` window find, Nub/Key는 **동일 verb가 아니라 별도의 observable J contract**다. 연산별 의미를 보존한 뒤 legality가 증명된 공통 하위 연산이나 multi-op region만 재사용한다.

**검토 B — RustJ 계층/IR에서 독립 출발.** 현재 `src/logical_ir.rs::SearchDescriptor`는 원본 `ValueId`, `indexed/queried` 방향, first/last/membership/interval/self-classify intent, `JEquality` 또는 ordered-interval 의미, rank boundary를 보유한다. `src/j_graph_jsource.rs`는 후보 발견만 하고 미증명·미선택 상태를 유지한다. 이것은 **보존**한다. 다만 `JEquality`라는 표식만으로 실제 `!.t`/전역 `cct`, 비교 타입 승격, boxed/complex/sparse, rank/cell/frame·empty prototype·error-order 증거가 성립하지 않는다. `E.`는 이미 별도 window rewrite candidate의 의미 영역이므로, C에서 내부 검색 루틴을 공유한다는 이유로 `LookupClassify`에 흡수하지 않는다. `I.@e.`와 membership-consumer 합성은 **복수 원본 graph node/consumer demand/provenance를 가진 region 후보**이며 primitive 하나를 새로운 특수 IR opcode로 대체하지 않는다.

**검토 C — 물리 실행/비용에서 독립 출발.** [BQN 검색 구현 설명](https://mlochbaum.github.io/BQN/implementation/primitive/search.html)에서 normal/reverse/sparse-table lookup은 **탐색 대상과 query의 어느 쪽에서 자료구조를 만들고 순회하는가**, one-shot vs persistent hash는 **준비 비용과 상태 수명**, direct lookup/hash/SIMD는 **실제 자료구조/타깃**, membership/index/first/last는 **관찰 결과**가 다르다. 현재 `src/lowering.rs::SearchAlgorithm`의 `DirectAddress/IndexedHash/ReverseQueryHash/PreparedHash/TolerantNeighborHash`는 첫 세 축을 **하나의 열거형에 혼합**하고 있다. 현행 제한된 CPU 구현은 유지할 수 있으나 범용화 시 ‘알고리즘 열거형을 모든 다른 primitive에 복사’하면 안 된다. 향후 후보 recipe의 직교한 선택 축을 **(1) 탐색·순회 방향 (2) 키/테이블 표현 (3) 구축·재사용/무효화 수명 (4) 출력 materialization/consumer (5) 대상·자원/비용**으로 *설명*하고, 실제 조합은 검증된 후보만 등록한다. 이것은 **후일 optimizer/Physical 전용 표현**이며 J Graph 또는 A3 스키마 변경 요구가 아니다. 임의 Cartesian-product 후보 폭발도 금지한다.

**검토 D — 반례/독립 oracle에서 독립 출발.** `src/index_ops.rs::lookup`의 `None` cache 경로도 `optional_exact_scalar_index` → `plan_search_algorithm`을 호출하므로 `Engine::eval_semantic_reference`는 **캐시 없는 reference일 뿐 엄밀한 sequential reference가 아니다**. `near`와 pinned C `TCMPEQ`의 이진64 경계 차이는 comparator identity 없이 최적화 동등성을 주장할 수 없다는 반례다. 비추이적인 tolerant equality는 대표 원소 하나의 해시 등가류로 축약할 수 없고, first/last와 nub/group 대표 선택의 proof도 다르다. 입력 변경·NAME rebinding·정책 변경·backend 전환·empty cell·guard 실패 시 어느 캐시/후보가 무효인지 증명 없이는 reuse하지 않는다. **언어 oracle=실제 jsource C**, **Rust 순차 semantic baseline**, **선택된 RustJ optimized route**의 세 경로가 독립적이어야 한다.

**프레임워크 대조 재검산(원본 비교, 자동 채택 아님).** [MLIR Dialect Conversion](https://mlir.llvm.org/docs/DialectConversion/)은 **analysis conversion**에서 IR을 고치지 않고 합법적으로 낮출 수 있는지 검사하고, **dynamic legality**는 실제 op 인자·속성에 따라 변환 허용을 결정한다. [MLIR Transform Dialect](https://mlir.llvm.org/docs/Tutorials/transform/)은 변환 대상(payload)과 별도 transform/schedule 표현을 사용할 수 있음을 보여 준다. RustJ에는 각각 **‘후보 발견≠채택’, ‘J semantics·runtime guard 없는 법적 허용 금지’, ‘원본 Graph/A3≠선택 계획’**으로 대응한다. 단, MLIR의 IR conversion에 성공했다고 J `!.t`·dynamic NAME·오류 순서를 증명한 것은 아니며, 지금 RustJ에 Transform Dialect나 MLIR 의존성을 설치해야 한다는 뜻도 아니다.

**계층별 증분 확장 계약 — 입력 / 유지할 정보 / 해당 계층에서 금지할 결정**

| 계층 | 유지·도출해야 할 최소 계약 | 금지 및 지금의 작업 |
|---|---|---|
| M2 Parser / FunctionEntity | primitive/derived identity, valence, source binding, `!.t`/동적 설정의 observable scope | C mode/CPU 특수 함수로 파싱 결과를 교체 금지; **M2 일반 parser 작업 우선** |
| J Graph / Analyzer | 원본 topology, query/domain role, producer/consumer·fanout·source provenance, composition 후보 | output demand를 잃거나 multi-op fusion을 일찍 확정하지 않음 |
| Execution semantics / A3 | first/last/member/interval/window/group의 **별도** 결과 의미, frame/cell/shape/empty, 비교 정책 계약, 오류·효과 | hash/direct/prehash/target을 J equality 또는 `SearchDescriptor` 안에 내장하지 않음 |
| Candidate discovery / proof (§4.1.4) | `RuleId`, source graph version/region, semantic obligation별 `Unknown/Proven/Disproven/Guarded`, exact runtime witness | observed source idiom이나 기본 dtype만으로 legal 선언 금지; stale candidate 폐기 |
| Target/resource/cost | runtime 또는 정적 근거로 검증한 dtype/item shape/count/range, table byte upper bound, build/probe/reuse, memory tier/transfer, target capability | 추정 work를 측정 시간으로 오인, unknown resource를 zero/cheap로 오인 금지 |
| 선택 계획 / index-state lifetime | candidate compatibility, direction/representation, 일회성·persistent 인덱스의 backing/policy identity/epoch, escape/invalidation, failure-before-effect fallback | J Graph를 변형해 cache/selection을 저장하거나 J 이름만으로 prepared state를 영속화하지 않음 |
| Executor / verification | 순차 reference, guarded route, J C oracle와 동일 결과/오류·효과 순서, 기록된 선택 원인 | 순차 oracle 내부에서 Physical 선택기 재호출 금지; guard miss 후 observable effect 재실행 금지 |

**최소 공통 프레임워크 확장 순서(현재 구현 지시가 아닌 수용 게이트):**

1. **M2:** `i.` family를 위한 새 hash가 아니라 search/reference의 *독립된 순차 baseline*과 실 J C differential, CCT·fit·Rank 등의 의미론 불일치 분류를 먼저 닫는다. 기존 `SearchDescriptor`의 방향·output 검증은 보존한다.
2. **M3:** §4.1.4의 **후보 provenance + obligation별 proof/guard + invalidation** 공통 evidence를 기존 search/rewrite/fusion 분석 sidecar에 투영할 최소 인터페이스를 정한다. `Unknown`을 완료로 해석하지 않는 verifier/negative test를 하나씩 만든다. **완전한 범용 registry를 먼저 설계하지 않는다.**
3. **M4:** 독립 CPU reference vertical slice와 기존 Int/Bool guard된 Physical route를 같은 J oracle로 검증한다. `SearchAlgorithm`·`PreparedHash` 등의 기존 enum을 일단 유지하며, 부하/표본 비용 없이 임계값을 바꾸지 않는다.
4. **최적화 착수 시:** 두 번째 독립 가족(예: Reduce/Scan의 reassociation, GroupBy의 대표 원소)에서 **실제 동일한 증거·수명·selection 인터페이스가 필요함을 증명한 후에만** 공통 `CandidateEvidence`/recipe/SelectionPlan으로 추출한다. `SearchDescriptor`나 search mode enum을 generic operator로 포장하지 않는다.
5. **이후:** one-shot vs reusable index, reverse-query 구축, materialization-elision, table footprint, GPU/외부 route 등의 물리 후보는 **full-J legality + target/resource hard gate + 실제 측정** 후에만 별도 plan으로 commit한다.

**반복 독립 검사와 실패 조건:** (A) jsource 실행 모드 이름을 제거해도 semantic descriptor가 J 결과를 결정하는가? (B) 모든 선택 후보를 비활성화해도 reference는 올바른가? (C) 비교 정책만 바꾸거나 source binding을 교체했을 때 캐시/guard가 무효로 되는가? (D) 동일 입력을 CPU/외부 adapter로 내릴 때 J Graph/A3가 불변인가? (E) 단일 search output이 아닌 `I.@e.`/Nub/Key/consumer fusion에서도 잘못된 범용화 없이 witness가 재사용되는가? **현재 답:** (A)의 의도/분리 일부 구현, (B) 독립 baseline 부재, (C) 동적 정책 미지원, (D) 구조 규정은 있으나 각 target 실행 미검증, (E) 해당 derived family semantic coverage 미완료. 따라서 공통 범용 optimizer ‘완료’나 search-specialized 성능 이득을 주장할 수 없다.

**이번 검토 범위:** 파일별 read-only 정적 점검 + pinned jsource 원본 + BQN/MLIR 설계 대조. **Rust/Cargo 실행, 전체 J binary differential, benchmark 또는 optimizer 새로운 API 구현은 하지 않았다.** 이 결정은 §P.0의 특수 코드 확장 동결 및 §P.3의 M2 우선순위를 **강화**하며 대체하지 않는다.




<a id="framework-migration-checklist"></a>

<a id="o5-지연-최적화-프레임워크-이행-계획체크리스트-2026-10-06-살아-있는-작업표"></a>
###### O.5 지연 최적화 프레임워크 이행 계획·체크리스트 (2026-10-06, 살아 있는 작업표)

**목적·범위.** §O.4의 네 관점 독립 검토를 **실제로 수정하고 검사할 수 있는 단일 체크리스트**로 전환한다. `i.` 계열은 최초 검증 사례이고, 최종 목표는 **J 의미론을 유지한 채 다른 연산군에도 적용 가능한 ‘발견 → 증명/가드 → 하드 자원/타깃 판정 → 비용 → 선택 → 실행’ 계약**이다. §P.1은 계속 **검색 알고리즘 자체의 상세 체크리스트**로 유지한다. **이 표가 단계 간 우선순위·완료 판정의 정본**이며, 새 로드맵 Markdown이나 새 범용 optimizer crate를 만들지 않는다.

**현재 진행 상태:** **이행 계획 문서화 완료 / 아래 구현·검증 게이트 0/18 수용.** 기존 `SearchDescriptor`, `AlgorithmCandidate<_,_>`, `GraphRewriteCandidate`, `FusionCandidate`, `ExactPrehashCache` 등은 **활용 가능한 현재 코드**이지 새 계약의 수용 완료를 뜻하지 않는다. **전체 프로젝트 우선순위는 M2 tokenizer → enqueuer → parser/POS/name/derived entity이며, M3/M4를 앞당기는 명분으로 검색 특수 알고리즘을 추가하지 않는다.**

**체크 상태 규칙.** [ ] = 수용 전(코드 일부 존재/작성/검증 대기 포함); [x] = **필요한 코드 + 실제 실행 검증 + 증거 링크를 모두 완료한 경우에만**. 단순 소스·테스트 파일 존재, 내부 Rust reference끼리의 일치, static assertion, 이전 milestone 수치는 완료 근거가 아니다. 각 항목을 완료할 때 같은 표의 **검증 기록**에 `commit SHA | 실제 명령 | 환경/target | pass/fail/ignored | jsource 고정 commit·실제 oracle 실행 범위 | 잔여 제한`을 적는다. CI 실행 여부는 별도 표시하며 실시하지 않았으면 ‘미실행’이라고 쓴다. 여기에 적는 날짜는 계획 작성일이지 완료일이 아니다.

**순서 및 게이트.** A(M2·기준 의미론) → B(M3·증거 인터페이스) → C(M4·독립 CPU 경로) → D(실측 이후 최소 공통 Physical 확장). 각 단계는 **자기 범위 내 이미 지원하는 J 의미론**에 대한 진짜 검증으로 통과시킬 수 있지만, 아직 지원하지 않는 J 형태는 `Unsupported/Unknown`으로 표시해야 한다. ‘full J 완료’를 허위 전제하지 않는다. A의 일반 frontend 미완료 항목이 첫 작업이고, search reference 작업은 이에 종속된 작은 수직 검증 과제로 진행한다. **한 번에 한 의미 + 하나의 회귀/negative test**를 기본 변경 단위로 한다.

| ID / 단계 | 완료 체크 | 수정 대상·실행 작업 | 선행 조건 / 수용 기준·검증 기록 |
|---|---|---|---|
| FW-01 / A·M2 | [ ] 프런트엔드 의미론 범위 고정 | `src/tokenizer.rs`, `src/enqueuer.rs`, `src/parser.rs`, `src/semantic.rs` 등 **실재 경로 확인 후** 미완료 syntax/POS/name/derived-entity 사건을 분류 | 일반 M2 우선순위를 유지. 기존 지원 subset의 parse/resolve 결과와 C oracle 대조; unsupported와 unresolved을 기록. **검증: 미실행** |
| FW-02 / A·M2 | [ ] 의미론 실행 모드 분리 | `src/runtime.rs`, `src/kernels.rs`, `src/index_ops.rs`: 현재 `pooled: bool`의 buffer pool 정책과 검색 physical 허가를 직교시킨다. reference `i.`/`i:`/`e.`는 `lookup`의 **순차 실행**만 통과; ranked/cell/derived call이 동작하는 범위에서 우회 없는지 확인 | `eval_semantic_reference`에서 `plan_search_algorithm`/prehash 호출이 **실제로 0회**임을 계측·negative test로 검증. 새 공개 API, 전역 mutable switch 또는 source IR 변경 불필요. **부분 구현/검증(2026-10-06):** `45ccebb`, `3436252`에서 `search_reference.rs` 순차 `i./i:/e.` 기준 경로와 prehash 미사용 회귀 검증. [Linux milestone 성공](https://github.com/yunskim/RustJ/actions/runs/37438389996), [Basis compile probe 성공](https://github.com/yunskim/RustJ/actions/runs/37438390025). **검증 추가:** [`80d0616`](https://github.com/yunskim/RustJ/commit/80d06163ea2d7966641bacd24cb1ac9ecc9e316c) 테스트 전용 same-thread planner 호출 카운터 및 최적화 경로 positive control. 서식·Clippy 수정 [`b86aafb`](https://github.com/yunskim/RustJ/commit/b86aafb0969c8e6b51fe827c6c83c1cce96e934b). [Linux milestone check 성공](https://github.com/yunskim/RustJ/actions/runs/37440725863), [Basis probe 테스트/Clippy 성공](https://github.com/yunskim/RustJ/actions/runs/37440725669); 전체 J C differential job은 별도. **미수용 [ ]:** rank/derived 우회 검증, C binary와의 3방향 의미론 비교, 전체 FW-02 수용 검증은 미완료. |
| FW-03 / A·M2 | [ ] 언어 비교 의미 정렬 | `src/comparison_policy.rs`, `src/kernels.rs`, 관련 semantic contract: 고정 Rust `near` vs C `TCMPEQ`, 전역 `cct`, `!.t` scope/override, 타입·오류·NaN/±0 등 지원 범위 분명히 한다 | 먼저 실제 **고정 jsource C binary oracle**로 경계 예제를 확인. 동적 설정 미지원이면 `Unknown/Unsupported` 처리하고 tolerant optimization 허가하지 않음. **FW-03 진단 기록(2026-10-06, 미수용):** 고정 C [`jsource@13994ffa`](https://github.com/jsoftware/jsource/tree/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528) `vcomp.h::TCMPEQ`/`i.c::cct=1-FUZZ`와 Rust `FixedRustNearV0` 사이의 binary64 경계를 [`tools/tolerance_audit.py`](https://github.com/yunskim/RustJ/blob/main/tools/tolerance_audit.py)로 실제 3경로 비교. [CI 37445080131](https://github.com/yunskim/RustJ/actions/runs/37445080131) Ubuntu Linux `j64`,`j64avx2` × Rust default/portable **각 48건 중 40 3-way 일치 / 8 C-vs-Rust 의미 불일치 / 0 Rust baseline-vs-optimized 불일치**(4 조합 총 192 관찰). `python3 tools/tolerance_audit.py --reference-revision 13994ffa1ed5f06f79fad6e9822a7ed2d29b1528 --report reports/tolerance-audit-<variant>-<backend>.json`; 모든 미일치는 report 원자료에 보존하며 CI 성공은 **진단 자체 성공**이지 J 동등성 합격이 아님. `9!:19`, `!.t`, Rank·boxed·sparse 및 전체 오류 우선순위는 미검증. **[ ] 유지.** **후속 구현/검증:** [`9aab336`](https://github.com/yunskim/RustJ/commit/9aab3362ac542ed96c0bd3ba6eb2ab6c109f14f4) pinned 기본 CCT 적용. [CI 37445891526](https://github.com/yunskim/RustJ/actions/runs/37445891526) j64·j64avx2 × default·portable **각 48/48, 총 192/192 일치, Rust reference-vs-optimized 불일치 0**. 위 40/8은 변경 전 이력. 범위/미완료 의무는 §P.9를 따른다. **[ ] 유지.** |
| FW-04 / A·M2 | [ ] 검색 reference 회귀·3방향 fixture | `src/index_ops.rs` 및 기존 differential tests: first/last, membership, 빈 셀·frame, 중복, 타입, 오류·Rank를 **C oracle / 순차 Rust / 최적화 Rust**로 분리한 케이스 구축 | 불일치 분류(언어 의미 vs optimized route), 실패 시 미지원/대체 실행 명시. 2개 Rust 경로가 동일 planner를 사용하면 3방향 증거로 세지 않음. **부분 구현(2026-10-06):** [`1f3d1cc`](https://github.com/yunskim/RustJ/commit/1f3d1cccfc5c30b385f5d886adc9cd9ac5872f22)의 exhaustive 3값 정수 4,840 `(indexed,query)` 조합에서 first/last/membership의 독립 위치 witness와 최적화 경로 비교 추가. **검증:** [`b86aafb`](https://github.com/yunskim/RustJ/commit/b86aafb0969c8e6b51fe827c6c83c1cce96e934b) Linux milestone check의 `cargo test` 성공. **C oracle/Rank/boxed/sparse/CCT 3방향 검증 미완료 [ ].** **2026-10-06 확장 (검증 대기):** [`bc648cc`](https://github.com/yunskim/RustJ/commit/bc648cc0ba43761e103cd42c58830147c30ebb9c)에 독립 프로세스 3방향 C/Rust baseline/Rust optimized harness 45문장 추가; [`4e8b4dd`](https://github.com/yunskim/RustJ/commit/4e8b4ddd1d4a5b9d065830fb0211137f9b894d26)에서 분류 테스트, [`09f9a3b`](https://github.com/yunskim/RustJ/commit/09f9a3ba7804fc769027b16c7c9f5abc630339ac)에서 j64·j64avx2 × default·portable CI gate 등록. 지원 dense subset만 검증하며 Rank/boxed/sparse/CCT는 미검증. **실행 증거(2026-10-06):** [Linux milestone run 37441174098](https://github.com/yunskim/RustJ/actions/runs/37441174098), 명령 `python3 tools/search_three_way.py --reference-revision 13994ffa1ed5f06f79fad6e9822a7ed2d29b1528 --report reports/search-three-way-<variant>-<backend>.json`; GitHub Actions Ubuntu Linux, J C `j64`/`j64avx2` × Rust default/portable 네 조합 **각 45 passed / 0 failed / 0 ignored, 합계 180 비교**. 실제 jsource pin `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`; 개별 fixture/해시/불일치 종류는 CI JSON artifact. **제한:** supported dense search 정수·bool·문자·정확 부동소수, 중복·빈 배열·multi-item·재검색; Rank-파생 우회/boxed/sparse/동적 CCT·Fit 미검증. **FW-04 전체 [ ] 유지.** **Rank 차분 진단(2026-10-06, 수용 아님):** [`5b4a7d6`](https://github.com/yunskim/RustJ/commit/5b4a7d6ddf0fdf8e966ee06f63173e71cdbce423) `tools/ranked_search_audit.py`, [`c300ff3`](https://github.com/yunskim/RustJ/commit/c300ff3295499f04e41c928a78e38cd3ed95d7fc) corpus 단위 테스트, [`d88c38b`](https://github.com/yunskim/RustJ/commit/d88c38b34530c84d7ed02b46a9cbf5a55186a50f) CI matrix 연결. [CI 37446732381](https://github.com/yunskim/RustJ/actions/runs/37446732381) Ubuntu Linux `j64`/`j64avx2` × Rust default/portable 각 **14 cases / 13 C·Rust reference·Rust optimized 일치 / 1 Rust 양 경로 Unsupported**, 합계 **52/56 일치(미해결 4개 관찰)**. 불일치 입력 `(i.0 3) (i."1 1) (i.0 3)`에서 C는 Int shape `[0,3]` 빈 결과, Rust 양 경로는 `Unsupported`(Rank empty-frame prototype inference). **이 사례를 성공/허용 불일치로 간주하지 않는다.** J Rank 결과 prototype·dtype·shape/오류 계약을 먼저 구현하고 3방향 재검증할 것. P.1/search exact optimization 증명으로 전용하지 않음; FW-04/JX-04 [ ] 유지. **P.10 zero-frame Rank 보강(2026-10-06):** [CI 37449102885](https://github.com/yunskim/RustJ/actions/runs/37449102885) pinned j64·j64avx2 × default·portable 4조합 각 20/20(총 80/80), supported dense typed fill-cell·monad/dyad·search/reference/optimized 3방향 일치. [Basis probe 37449102980](https://github.com/yunskim/RustJ/actions/runs/37449102980) 성공. Boxed/sparse, computational error fallback, 사용자 정의 effectful Rank 및 mixed-result padding 미지원으로 **[ ] 유지**. **후속 단위 작업:** [§P.11 RK-01~RK-12](./14-framework-research-p03.ko.md#rank-cellapply-followups) 체크리스트에 실행 증거·진행 상태를 기록한다. **상위 [ ] 유지.** |
| FW-05 / B·M3 | [ ] 후보 원본·버전 식별 계약 | `src/j_graph_ir.rs`, `src/j_graph_rewrite.rs`, `src/j_graph_fusion.rs`, `src/j_graph_jsource.rs`: 기존 source ID/span/basis에 **graph/schema/rule identity 또는 유효성 검증 수단**을 연결. original J Graph는 불변 | stale graph / changed rule / forged origin을 거부하는 verifier negative tests. 단순 span 일치를 유일한 유효성 증명으로 취급하지 않음. **검증: 미실행** |
| FW-06 / B·M3 | [ ] obligation별 증거 상태 | §4.1.4의 `CandidateEvidence` 목표를 search/rewrite/fusion **sidecar view 또는 최소 adapter**로 구체화: `Unknown / Proven(witness) / Guarded(guard,fallback) / Disproven`과 provenance를 분리 | 동일 `selected` bool·SearchAlgorithm enum·단일 큰 공통 IR로 합치지 않음. `Unknown`이 legal/selected로 승격되지 않는 negative tests. **검증: 미실행** |
| FW-07 / B·M3 | [ ] 가드·효과/오류 선후 관계 계약 | `src/lowering.rs`, effect/error/verifier 및 실행 adapter: guard를 **효과 발생 전**에 두고 miss 시 정확한 reference fallback 명시 | Observable effect 뒤 자동 재실행·중복 오류·fallback 누락을 거부. `Guarded` 상태만으로 선택 허가하지 않음. **검증: 미실행** |
| FW-08 / B·M3 | [ ] 비교 정책·이름·Rank의 동적 witness | `src/logical_ir.rs::SearchDescriptor` 주변 semantic policy 계약과 `comparison_policy.rs`: `!.t`/runtime CCT, dynamic NAME, rank/cell/empty 등이 후보별 어떤 증명을 요구하는지 표준화 | `JEquality`라는 라벨·dtype만으로 `exact hash`를 정당화하지 않음; 미확정 policy/Rank이면 선택 거부·reference 경로. **검증: 미실행** |
| FW-09 / B·M3 | [ ] 기존 3종 후보에 같은 legality view 투영 | `j_graph_rewrite.rs`, `j_graph_fusion.rs`, `j_graph_jsource.rs` + `fusion_planning.rs`에 provenance/obligation 상태를 **별도 원본 타입을 유지하며** 노출 | 후보 중복·기존 E. window witness를 검색 witness로 오용·unresolved resource를 legal로 처리하는 경우 거부. **검증: 미실행** |
| FW-10 / B·M3 | [ ] 두 번째 독립 가족 교차 검증 | 기존 Reduce/Scan **또는** GroupBy의 semantic candidate 하나를 선정해 FW-05~09의 증거/가드 인터페이스 적합성 확인 | 단지 두 가족에 enum 이름만 공유하는 것은 불합격. 다른 numeric/대표원소/order proof를 보존하면서 공통성이 실제 입증될 때만 구현 인터페이스 추출; 범용 registry 선행 구축 금지. **검증: 미실행** |
| FW-11 / C·M4 | [ ] Native CPU semantic vertical slice | 실제 supported subset의 J Graph → A3 verification → baseline CPU execution 경계를 닫고 reference 실행과 비교 | source/error/effect order를 유지. C binary oracle 범위·명령·환경·차분 결과를 기록. **검증: 미실행** |
| FW-12 / C·M4 | [ ] 기존 exact Int/Bool 경로 독립 검증 | `src/index_ops.rs`, `src/physical.rs`: Sequential / Direct / Indexed / Reverse / Prepared 별 output first/last/member와 allocation failure fallback 확인 | 같은 planner에 의존하지 않는 순차 reference 및 C oracle에 대해 3방향 검증. Rust default/portable는 각각 실제 결과 기재. **검증: 미실행** |
| FW-13 / C·M4 | [ ] target·hard resource·cost의 독립 승인 | `src/lowering.rs`, `src/physical.rs`, `src/j_graph_resource.rs`: static/runtime exact guard, table **byte** bound, estimated work·temporary allocation, target capability, measured cost를 서로 별도 상태로 보고 | hard limit `Unknown`은 값 0이 아니며 ‘느림’과 ‘불법/부적합’을 혼동하지 않음. 미측정 threshold는 heuristic으로 표시. **검증: 미실행** |
| FW-14 / D·후순위 | [ ] 물리 recipe의 선택 축 분리 | 충분한 두 가족 증거·실측이 나온 뒤 `SearchAlgorithm`과 `SearchWorkload/Choice`의 축(빌드/순회 방향, hash/direct/SIMD 표현, lifetime, output demand, target)을 재설계할 필요성 판정 | **구체적 공통 사용 사례 2개**와 기존 API migration·동등성 테스트 없는 대대적 분해 금지. J Graph/A3 schema에 물리 선택을 넣지 않음. **검증: 미실행** |
| FW-15 / D·후순위 | [ ] prepared-state 수명/무효화 계약 | `src/index_ops.rs::ExactPrehashCache`와 향후 다른 가족의 prepared state 비교: backing identity/immutability, first/last representative, policy/version, binding epoch, target, retention | 기존 exact Int/Bool 키에는 무관한 policy를 억지로 넣지 않음. 실제 영향을 주는 속성만 witness/key 포함; rebinding·cache miss·정책 변경 negative tests. **검증: 미실행** |
| FW-16 / D·후순위 | [ ] 별도 SelectionPlan·충돌/호환성 검사 | §4.1.4 target/semantic/resource/cost evidence로 검증된 후보 집합의 overlap·compatibility 검사와 별도 selection/committed lowering 식별자 | Graph/SSA source와 original witness 보존. legality 없는 선택, overlapping incompatible candidates, stale commit 거부. **검증: 미실행** |
| FW-17 / D·후순위 | [ ] 실측·타깃 확장 승인 | 독립 cost benchmarking 후에만 jsource/BQN 작은 배열·SIMD·reverse/prehash 조정 및 CPU/외부 route 검토. GPU는 기존 보류 정책 유지 | 동일 semantics + 메모리 상한 + 실측 win + portable fallback+negative tests 통과 시 **개별** 전략 활성화. **검증: 미실행** |
| FW-18 / 전체 수용 | [ ] 반복 독립 검토·회귀 기록 | A) 원본 J 의미 B) Graph/A3 불변 C) proof·invalidations D) CPU/target/resource E) C·순차·optimized 3방향/성능의 **서로 독립된 재검토**를 수행하고 §P.2와 연결 | 충돌 시 앞 단계 재개; pass/fail/ignored·실행 환경·원본 revision·bench 근거가 남고 기존 §P.1 체크 상태와 모순이 없어야 최종 수용. **검증: 미실행** |

**의존성/중단 규칙.** FW-02가 완료되기 전에는 “optimized/reference 통과”라는 내부 상호 비교만으로 FW-12를 완료하지 않는다. FW-03·08의 dynamic 비교 의미가 미완성인 동안 tolerant hash의 `NeedsSemanticProof`를 변경하지 않는다. FW-05~09에서 공통 증거를 정의했다고 FW-10의 **실제 두 번째 가족 검증**이 된 것은 아니다. FW-11~13이 완료되고 명시적인 메모리·타깃·실측 증거가 없으면 FW-14~17의 물리 전략/비용 임계값 변경을 시작하지 않는다. 버그 수정과 의미론 정확성 보완은 언제나 우선 가능하다.

**검증 진행 로그 템플릿(완료 시 해당 행에 복사):** `FW-ID | 코드 commit | 실행한 명령과 환경 | passed/failed/ignored | 사용한 jsource commit+실제 J 실행 범위 | 확인된 unsupported/known gaps | 다음 게이트`. 계획만 추가한 지금은 모든 실행 검증 항목이 **미실행**이다. 별도 일일 보고 파일 대신 이 절을 갱신하고 영문 §O.5와 체크 상태를 함께 맞춘다.



<a id="index-family-roadmap"></a>

<a id="p-i-family-통합-로드맵진행-체크리스트--roger-hui--marshall-lochbaum-2026-10-06"></a>
##### P. i. family 통합 로드맵·진행 체크리스트 — Roger Hui × Marshall Lochbaum (2026-10-06)

**교차 참조:** [§O.5 프레임워크 이행 계획](#framework-migration-checklist)은 **공통 구조·단계 게이트**를 관리하며, 여기 §P.1은 검색-family 알고리즘별 세부 검증을 관리한다. 두 표의 [x] 규칙은 다르므로 이행 계획의 [x]를 코드 존재만으로 변경하지 않는다.

**정본 운영 규칙.** 이 체크리스트는 [§N: i. 원본 및 1·2차 구현](#jsource-index-family), [§O: A3/Registry/Physical 경계](#algorithm-planning-migration), [§4.1.4: proof→selection lifecycle](../../../PROJECT.ko.md#)에 종속된 **살아 있는 구현 게이트**다. 항목마다 **근거 / 실행 계층 / 완료 조건 / 검증 상태**를 명시한다. [x]는 **코드·문서가 저장소에 존재함을 정적으로 확인**했음을 뜻할 수 있으며, 테스트 실행·jsource 동등성·성능 달성을 자동으로 의미하지 않는다. 별도 문서를 늘리지 않고 이 절에서 관리하며 완료될 때마다 실제 검증 근거를 기록한다. **Linux milestone은 별도로 만들거나 완료 조건에 넣지 않는다.**

**독립 참고 원본 및 적용 기준:**

- Roger Hui, [*Index-Of, a 30-Year Quest*](https://www.jsoftware.com/papers/indexof/indexof.htm) 및 [*Hashing for Tolerant Index-Of*](https://www.jsoftware.com/papers/Hashing.htm): J의 첫·마지막 일치, Nub/Key의 대표 원소, tolerant comparison 및 소스 실행 모드의 출발점. 실제 동작의 대조 원본은 pinned [jsource `vi.c`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vi.c#L140-L185), [`viavx.c`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx.c#L738-L850), [`viavx2.c`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx2.c#L8-L98). C fast path 자체를 동등성 증명으로 사용하지 않는다.
- Marshall Lochbaum, [*BQN: Implementation of search functions*](https://mlochbaum.github.io/BQN/implementation/primitive/search.html): 작은 배열 순차/SIMD, lookup table, one-shot hash, reverse hash, lazy/sparse **table initialization**, open-addressing/linear probing, collision observation, partitioning/radix 및 cache 비용을 비교하는 **알고리즘·물리 최적화 참고**. BQN과 J의 비교 동등성, Fit, Rank, Boxed/AxisSparse 의미가 같다고 가정하지 않는다. BQN의 *sparse lookup*은 **직접 주소 테이블 일부만 초기화하는 기법**이며 J sparse array와 다르다.
- 외부 프레임워크 [§O](#algorithm-planning-migration)의 MLIR dynamic legality, IREE target lowering, TVM measured cost, XLA resource/cost 분리는 **알고리즘 소유권 경계**로 재사용한다. 새로운 IR 계층이나 C 함수별 Graph node를 만들지 않는다.

<a id="p0-지연-최적화-정책과-m2-우선순위-검토-2026-10-06"></a>
###### P.0 지연 최적화 정책과 M2 우선순위 검토 (2026-10-06)

**판정: 구조적 분리는 대체로 적합하지만, 작업 우선순위와 일부 조기 실행 전문화는 조정해야 한다.** 프로젝트 최상단의 정본 우선순위는 **M2 word formation → enqueue → J parser/이름·POS·derived entity 의미론 수렴, 이후 M3 경계와 M4 Native CPU vertical slice**이다. `FOUNDATIONS.ko.md` Part XX는 J source를 실행 계획으로 취급하지 않고, 높은 수준의 rank/train/reduce/scan 구조와 source provenance를 보존하며, logical legality와 target-specific profitability/materialization을 분리하라고 규정한다. 여기서 **지연된 최적화**는 모든 분석을 뒤로 미루는 것이 아니라, **특수 실행 알고리즘의 확정·materialization·선택을 충분한 의미론/타깃/비용 증거 이후로 미루는 것**이다.

| 작업 분류 | 현재 판단 | 단계 경계 및 조치 |
|---|---|---|
| jsource 검색 family의 primitive/valence/Rank 의미, `!.t`/전역 `cct`, 오류·First/Last·비추이성·원본 C comparator 반례 | **현재 의미론 과제** | M2 의미론/differential 및 M3 semantic contract에서 해결한다. jsource는 **언어 oracle/반례 발견 수단**이며 특수 해시 알고리즘 복사 지시가 아니다 |
| J Graph source topology·A3 `SearchDescriptor`·원본 SSA provenance·`LookupClassify` | **유지** | 비교 정책/결과 intent/Rank·frame 및 아직 모르는 facts를 보존한다. 임시 table size, bucket, hashing, SIMD를 IR 의미로 승격하지 않는다 |
| `JsourceOpportunity`, `SearchAlgorithmReadiness`, `ComparisonPolicySnapshot` | **증거/경계에 한해 유지** | 후보 보고는 `selected=false`, 허용되지 않은 tolerance route는 `NeedsSemanticProof`; 현재 snapshot은 고정 Rust 비교식의 identity일 뿐 J 동적 policy 구현이 아니다 |
| 현재 동작 중인 CPU Int/Bool Direct/Hash/Reverse/Prehash | **기존 제한적 interpreter optimization; 확장 중지** | 실행 시 exact scalar guard·순차 fallback을 지키는 범위에서만 유지. benchmark와 전체 J differential 없이 일반 compiler PhysicalPlan의 완성이나 jsource 동등성이라고 부르지 않는다. 의미론 회귀가 발견되면 우선 reference로 회귀시킨다 |
| `src/tolerant_search.rs` 연구 bucket, BQN SIMD/작은 범위 table, open-addressing·radix/partition, 임계값 조정, 실전 tolerant hashing | **후순위 / 동결** | M2 의미론이 닫히고 M3 검증 계약, M4 CPU reference vertical slice 및 비교·비용 측정이 마련된 후 독립적으로 평가한다. 후보 연구 코드는 `#[cfg(test)]` 외 실행 경로로 올리지 않는다 |

**지금 당장 진행할 일:** `=, i., i:, e., E.`의 기본/변경 tolerance, Rank·cell/frame·boxed/sparse/empty 및 에러 순서에서 C oracle과 Rust reference 차이를 수집하고, J semantic model/IR 경계를 정리한다. 이미 확인한 `TCMPEQ` 경계 반례는 **의미론 수정이 필요한 검증 입력**이며, jsource의 `viavx2.c` 최적화 구현을 당장 따라 만들 근거가 아니다. M2의 일반 frontend 작업을 이 연구가 장기간 선점하지 않도록 한다.

**재개 조건:** 검증된 reference semantics + source/differential evidence + 명시적 target guard/fallback + M4 기준선 측정 + 상한 자원/비용 근거가 함께 있을 때만 실제 알고리즘 특수화를 검토한다. 이 조건을 충족하기 전 P.1의 8–11단계 작업을 **착수 대상에서 제외**하며, 5–7·12단계의 아직 검증되지 않은 항목을 완료 처리하지 않는다. 새 jsource C special-case마다 RustJ 고유 IR·executor branch를 만드는 설계는 금지한다.

**지연 최적화 재검토에서 드러난 기준선 혼합:** `eval_semantic_reference`는 캐시 사용을 끄지만 `index_ops::lookup`의 일반 exact scalar Physical search planner는 여전히 호출한다. 'reference path'라는 명칭이 최적화와 독립인 순차 oracle을 보장하지 않는다. P.3에서 M2 우선 과제로 분리한다.

<a id="p1-구현-단계와-수용-기준-계속-갱신"></a>
###### P.1 구현 단계와 수용 기준 (계속 갱신)

| 순서 | 체크 항목 | RustJ 소유 계층·수용 기준 | 현재 상태 |
|---|---|---|---|
| 0 | [x] J 검색 family를 원본 이름·valence·의도에 따라 분류 | J Graph/semantic identity: dyadic `i.`/ `i:`/ `e.`와 monadic generate, dyadic `I.` interval, `E.` window find를 혼동하지 않음 | 코드·§N 정적 확인, full-J 미완료 |
| 1 | [x] 검색 의미와 실행 전략 분리 | A3 `SearchDescriptor` first/last/member/interval/self; `Operation.j_origin`, rank boundary, J equality를 보유하고 Physical table 정보는 배제; A3 schema 0.6/verifier | 구현·회귀 테스트 추가; **실행 미확인** |
| 2 | [x] 알고리즘 후보·가드 상태와 비용 선택기 분리 | `LoweringRegistry`의 Baseline/Guard/Proof/Unsupported, `physical.rs::plan_search_algorithm`의 `SearchWorkload/Choice`. Unknown→legal 금지. CPU exact scalar에 한정 | 구현·정적 확인; **벤치마크 미실시** |
| 3 | [x] 기본 검색 전략과 optional cache 준비 | CPU 순차, 좁은 정수 Direct, Hash, query-side Reverse, immutable-Arc per-Engine Prehash; direct Boolean `e.` 결과와 table allocation 실패 시 순차 fallback | `index_ops.rs`/runtime 구현, 아직 conformance 미통과 |
| 4 | [x] Tolerant equality 반례와 후보 포괄성 **연구 테스트 작성·연결** | `src/tolerant_search.rs`(테스트 전용, `#[cfg(test)]`)에서 비추이성, first/last, 지수 bucket ±1, ±0, NaN, ±Inf, subnormal 및 독립 **linear search index** 대조 | **테스트 코드만 작성**, 실행·C oracle 검증 미완료; 런타임 미연결 |
| 5 | [ ] J 비교 계약 구체화 | `ComparisonPolicy`: `!.ct`/runtime cct/version, float/complex/boxed/AxisSparse, exact build vs tolerant probe, rank/cell/frame, first/last representative, error/effect precedence를 source-linked witness로 확정 | 의미론적 proof 미완료 |
| 6 | [ ] Tolerant candidate-filter 완전성 증명 | 현재 fixed `kernels::near(a,b)`의 **t = 2^-44** 조건에서 부호·절대값 지수의 인접 bucket 탐색이 가능한 모든 일치 위치를 포함함을 증명. exact match는 +0/-0·동부호 Infinity 별도, NaN 제외. **원래 predicate로 모든 candidate 재확인·원래 source index로 최솟값/최댓값 선택** | 조건부 수학 논증·연구 코드; 일반 J equality의 증명은 아님 |
| 7 | [ ] runtime tolerance-guard + miss fallback과 prehash invalidation | C reference/다중 precision·fit/rank·동적 변경에 대한 guard; 실패하면 효과 이전 기존 순차 경로. 인덱스의 `cct`/argument backing identity/version과 lifetime을 검증 | 실행 미구현·미검증 |
| 8 | [ ] BQN 기반 작은 인자·Small-range 고도화 | 작은 인자별 순차 양방향/SIMD, byte/2-byte 직접 테이블, packed presence, sparse **table initialization**; CPU feature·메모리 상한·first/last 결과가 기존과 같을 때만 선택 | 알고리즘·비용 검증 대기 |
| 9 | [ ] 대규모 해시 충돌·캐시 개선 | one-shot workload에 맞는 open-addressing/linear-probing **대안**(현재 Rust `HashMap`은 당장 교체 금지); 충돌 계수, 해시/비교 adversarial guard, high-collision sorting/radix fallback, partitioning/캐시 동작 비교 | 성능·구현 대기 |
| 10 | [ ] BQN reverse/Prehash 선택과 비용 보정 | query/indexed 크기 비율, uniq cardinality, table 초기화·clear 비용, cache residency, reuse 횟수, persistent vs one-shot 준비비를 `ResourceEstimate`와 `CostEstimate`로 분리. hard memory guard가 비용보다 우선 | fixed heuristic만 있음 |
| 11 | [ ] `i.` 외 가족으로 공통 알고리즘 선택 패턴 이식 | Nub/Key/`I.@e.`의 결과 모드와 순서 계약을 확장한 후, Reduce/GroupBy/Grade/Contract는 **각자 독립 semantic proof**를 둔 generic `AlgorithmCandidate`/SelectionPlan을 사용 | interface prototype 일부만 존재 |
| 12 | [ ] Release gate: Rust/C differential + 비용 실측 | 기본/portable Rust tests, J C 두 경로의 first/last/NaN/±0/empty/rank/boxed/sparse/`!.ct`/error, 참조 순차 vs 모든 활성 fast path, 무작위·충돌 유도 벤치 및 memory/time 회귀. **측정 전 threshold 고정 최적화 주장 금지** | 수행하지 않음 |

<a id="p2-독립-검증-루프-반복결과-기록"></a>
###### P.2 독립 검증 루프 (반복·결과 기록)

- [ ] **A — source-first 독립 검토:** Hui 논문 → 고정 jsource dispatch(`vi.c`)/tolerance(`viavx2.c`)/prehash 모드 → J 오류·Rank·Fit 계약을 추출. BQN 코드를 의미론적 oracle로 대체하지 않음.
- [ ] **B — mathematics-first 독립 검토:** `a == b || finite ∧ |a-b| ≤ t·max(|a|,|b|)`의 비추이성과 exponent bucket 포괄성, IEEE-754 rounding/underflow/overflow 및 대표 인덱스 순서를 **해시 코드와 별도** 증명. \(0 ≤ t < 1/2\) 같은 가정을 witness로 기록.
- [ ] **C — reference-first 독립 검토:** 모든 query에 대해 J C oracle, 현재 Rust 순차 near oracle, 후보 인덱스 검색을 3방향 비교한다. **Rust near와 일치한다**는 사실만으로 J C 일치가 성립하지 않음.
- [ ] **D — compiler-boundary 독립 검토:** Graph candidate → A3 descriptor/verifier → Registry dynamic legality → Physical cost/resource → runtime guard/fallback을 파일별로 읽고 stale origin, `e.` 인자 방향, GPU/Interval/Tolerant 미승인을 다시 확인한다.
- [ ] **E — adversarial/performance 독립 검토:** high collision, all-identical, near-chain, sorted/unsorted, tiny/massive range, no-match majority, cache-pressure, table build/miss/allocation과 source-order stable output을 계측. speed win이 없는 방안은 보류·폐기한다.

**반복 기준:** 한 번의 점검에서 실패 사례가 나오면 관련 코드·proof/fallback과 회귀 테스트를 수정하고 A~E 중 직접 관련되지 않은 관점에서도 다시 검증한다. [ ]를 [x]로 바꿀 때는 **실행 명령, 환경, 테스트 개수/결과, 비교 원본의 commit 또는 측정 데이터**를 같은 절에 기록한다. Linux 전용 milestone은 생성하지 않는다. CI를 실행하지 않았으면 통과라고 기록하지 않는다.

