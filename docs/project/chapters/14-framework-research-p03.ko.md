[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)

<a id="p3-현재-known-gaps--바로-다음-순서--m2-우선으로-재정렬-2026-10-06"></a>
###### P.3 현재 known gaps / 바로 다음 순서 — M2 우선으로 재정렬 (2026-10-06)

1. **M2 frontend 수렴이 우선:** word formation → enqueue → parser/name/POS/derived entity의 실제 미해결 의미론과 회귀 검증을 먼저 마무리한다. 검색 최적화 연구는 frontend 작업을 선점하지 않는다.
2. **독립적인 의미론 기준선 복구:** `eval_semantic_reference(...)`는 `pooled=false`지만, `kernels::dyad("i."/"i:"/"e.")`가 `index_ops::lookup(..., None)`을 호출하고 그 내부에서 **여전히 `optional_exact_scalar_index` → `plan_search_algorithm`**을 실행한다. 따라서 지금의 `semantic reference`는 **prehash를 피할 뿐 Direct/IndexedHash/ReverseQueryHash 선택에서 독립적이지 않다**. M2/M3 기준선으로 사용할 때는 순차·명세 중심 검색과 optional physical strategy를 실제로 분리하고 동일 J 입력에 두 경로가 일치하는지 검사해야 한다. 코드 변경 및 테스트 통과는 **아직 미실행**.
3. **J 언어 의미를 독립 검증:** `=`, `i.`, `i:`, `e.`, `E.`, `!.ct`, `9!:18/9!:19`의 원본 J C 결과를 수집하고 기본 비교식 `near`와 `TCMPEQ` 경계 차이, rank/cell/frame/empty/boxed/sparse/error를 참조 실행과 대조한다. 이 작업은 특수 해시 알고리즘 복제가 아니라 의미론 정확성 검증이다.
4. **Graph/A3/Registry 보존, Physical commitment 연기:** source topology·comparison intent·provenance·unknown proof facts와 실행 합법성 보고는 보존한다. P.1의 기존 Int/Bool 특수 실행은 현재 구현물로만 취급하고 후속 확장을 동결한다. BQN SIMD/작은 direct table/radix/충돌 전략, tolerant bucket 활성화, 휴리스틱 튜닝은 M2/M3 의미론 안정화와 M4 기준 CPU 실행·실측 뒤에만 재검토한다. `TolerantNeighborHash`는 `NeedsSemanticProof` 상태를 유지하며 Runtime/Physical 실행 경로에 올리지 않는다.

**검증 정책:** semantic-reference와 optimized 경로가 동일한 내부 planner를 공유하는 동안 두 결과의 일치는 독립적인 최적화 검증 증거가 아니다. 추후 확인된 C oracle 및 순차 semantics 기준선과의 **3방향 비교**, 오류/효과 순서, fallback, 실제 비용 측정을 별도 gate로 둔다.

<a id="p4-독립-검토-기록-1--제한된-float-search-후보-완전성-2026-10-06"></a>
###### P.4 독립 검토 기록 #1 — 제한된 float search 후보 완전성 (2026-10-06)

**의미론·수학 관점 (코드 구현과 독립):** 현재 RustJ `kernels::near`는 `a == b || finite(a,b) && |a-b| <= t * max(|a|,|b|)`이고 `t = 2^-44`. `a,b`가 0이 아닌 유한수이며 비교가 성공했다면 서로 부호가 다를 수 없다(반대 부호이면 차이는 두 절대값의 합으로 `t<1` 조건을 위반). `M=max(|a|,|b|)`, `m=min(|a|,|b|)`이면 `M-m <= t M`이므로 `m >= (1-t)M > M/2`(`t<1/2`). 따라서 이진 지수 `floor(log2 |a|)`와 `floor(log2 |b|)` 차이는 최대 1. **query와 동일 부호인 지수 `e-1,e,e+1` bucket은 성공 가능한 source 항목을 누락하지 않는 필요조건**이다. 결과는 bucket 대표값이 아니라 **모든 원본 index를 취합한 뒤 원래 `near`로 재검사**하고, `i.`은 최소 원본 index, `i:`는 최대 원본 index를 택한다. 비추이적 연쇄 `a≈b≈c`, `a≉c`에서 앞의 원소 하나로 bucket을 대표시키는 방식은 불허한다. +0/-0, 동부호 ±Infinity는 exact equality로 별도 취급하고 NaN은 불일치한다.

**수치 반증 탐색 (Rust/C 테스트 아님):** 동일한 IEEE-754 `f64` 의미를 JavaScript 숫자 모델로 모사해 **215개 값의 1,160개 일치 쌍**을 조사했다. 지수 경계를 넘는 일치 쌍은 132개, 같은 부호·지수차 ≤1 필요조건을 어긴 사례는 **0개**였다. `(1,1+0.75t,1+1.5t)`에서 근사 동등성은 `true,true,false`로 비추이성을 확인했다. **표본 확인은 일반 수학 증명·Rust 테스트 실행·J C 비교·성능 보증을 대신하지 않는다.** 이 절에서 확인한 것은 고정된 현재 Rust `near`에만 적용되며 J `!.ct`/복소수/박스/Rank 등에는 자동 일반화하지 않는다.

**계층·소스 독립 검토:** BQN 원문은 작은 인자 SIMD, sparse **table** initialization, reverse lookup과 large-input partitioning의 알고리즘 참고로만 사용한다. 실제 C의 tolerant 해시(`viavx2.c`)는 두 인접 tolerance 구간과 exact insert / tolerant probe를 활용하므로, 연구용 **부호·지수 3-bucket** 설계를 jsource의 bitmask 알고리즘과 동일하다고 주장하지 않는다. A3/Registry/Physical/Runtime/문서의 provenance/guard/비승격·체크리스트 항목 **13개 정적 확인을 통과**했다. `src/tolerant_search.rs`는 **`#[cfg(test)]` 등록만** 되어 있으며 실제 Runtime을 호출하지 않는다. **Cargo/CI/C differential/benchmark를 수행하지 않았으므로 P.1의 실행 수용 게이트는 계속 [ ]로 둔다.**


<a id="p5-독립-검토-기록-2--ieee-754-인접-비트원본-순서-보강-2026-10-06"></a>
###### P.5 독립 검토 기록 #2 — IEEE-754 인접 비트·원본 순서 보강 (2026-10-06)

**이번 변경과 완료 범위.** 연구 전용 `src/tolerant_search.rs`에 `fixed_near_candidate_filter_covers_ieee_neighbor_words_and_first_last` 회귀 테스트를 추가했다. subnormal/normal, 1·2의 거듭제곱 경계, 최대 유한수/Infinity/NaN, 부호별 인접 비트 패턴과 고정 seed 2,048개 임의 `f64` 비트 패턴을 포함한다. 각 query에 대해 (a) **원본 전체 배열**을 순차 `kernels::near` 비교한 참조 결과가 candidate 집합에 포함되는지, (b) 모든 candidate index가 유일·원본 순서대로 정렬되는지, (c) First/Last 반환값이 순차 참조와 같은지를 따로 확인하도록 작성했다. 이 테스트는 `#[cfg(test)]`로만 컴파일되는 기존 실험 모듈에 있으며 **실제 Rust 테스트 실행 완료를 의미하지 않는다**.

**별도 언어로 수행한 수치 반증 탐색.** 소스 버킷 함수와 독립적으로 작성한 Python IEEE-754 `double` 모델에 원본 원소 **4,224개**, query **310개**, 가능한 쌍 **1,309,440개**를 대입했다. `near` 일치 **674개** 중 서로 다른 버킷에 속한 쌍 **143개**를 포함하여 후보 누락 **0개**였으며, 전 query에서 순차 First/Last와 후보 기반 결과가 일치했다. 근사비교 비추이성 검산은 `true,true,false`였다. 이 결과는 **Python 모델의 유한 표본 검사**이며 Rust `cargo test`, J C oracle, 모든 `f64` 조합의 기계적 완전성 증명, 성능 측정이 아니다.

**독립적인 증명·안전성 검토.** `t=2^-44`는 `1/2`보다 압도적으로 작으므로 성공하는 비영 유한수 두 값은 부호가 같고 큰 절대값 `M`에 대해 작은 절대값 `m>(1/2)M`이어야 한다. 따라서 부호+이진 지수 차이 최대 1이라는 **필요조건만** bucket 필터에 사용한다. 비교에 사용한 실제 floating-point `near`를 후보마다 다시 평가하므로 거짓양성은 허용되지만 거짓음성은 허용되지 않는다. 계산된 `t*M`이 subnormal에서 0으로 underflow하면 일치 범위는 더 좁아질 뿐이며, `a==b` 경로는 signed zero와 Infinity를 별도 수용한다. 이 설명은 **현재 고정 함수**의 증명 개요이고, `!.ct`·동적 cct·복소수·박스·Sparse·Rank·셀 비교로 확대하는 정당화가 아니다. 단일 대표 원소 해싱과 tolerance 전역 cache는 여전히 금지한다.

**검증 상태 및 다음 게이트.** 이 환경에서 Rust/Cargo 실행 환경을 확보하지 못했으므로 `cargo test tolerant_search`, `cargo test --features portable tolerant_search`, `cargo fmt --check`, `cargo clippy`, jsource C 비교 및 성능 측정은 **미실행**으로 유지한다. P.1의 5–7, 12와 P.2의 C/E는 완료 처리하지 않는다. 다음 증거는 서로 다른 `!.ct` 설정, float/complex/boxed/Rank·empty 경계에 대한 **실제 C 결과**와 Rust 결과 차등, tolerance policy/version witness, 그리고 guard miss 시 원래 순차 의미를 보존하는 조건이다. `TolerantNeighborHash`의 `NeedsSemanticProof`는 변경하지 않았고 Physical 선택·런타임 실행에 연결하지 않았다.


<a id="p6-원본-j-비교-정책증명-경계--source-first-독립-검토-2026-10-06"></a>
###### P.6 원본 J 비교 정책·증명 경계 — source-first 독립 검토 (2026-10-06)

**확인 가능한 J 의미론 원본:** J Dictionary의 [Equal (=)](https://www.jsoftware.com/help/dictionary/d000.htm)은 유한 float/complex 비교 기본 허용오차를 `2^-44`로 설명하고 `!.t` 재정의를 명시한다. [Fit (!.)](https://www.jsoftware.com/docs/help806/dictionary/d411.htm)에는 `i.`, `i:`, `e.`, `E.` 등의 fit 대상이 열거되어 있고, [전역 매개변수 9!:18/9!:19](https://www.jsoftware.com/help/dictionary/dx009.htm)는 tolerance 조회/변경 계약이다. 즉 기본 값이 우연히 현재 `kernels::near`의 상수와 일치해도 **실행 시 적용되는 비교 정책의 동일성**을 추정할 수 없다.

**확인한 pinned C 알고리즘:** [jsource `viavx2.c`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx2.c)는 `TFINDXYT`에서 원본 버킷과 인접 tolerance 구간을 모두 읽고, `IOFT`에서 `PUSHCCT(1.0)`로 table을 **불허용오차 비교로 구성한 다음** 검색 시 허용오차를 사용한다. `IIDOT`·`IICO`는 서로 다른 representative selection(최소/최대 원본 위치)을 갖는다. 이것은 **인접 구간의 후보를 모두 확인해야 한다는 source 증거**이지, 연구 중인 3개 이진 지수 bucket 구현과 동일하다는 증거가 아니다. 단일 approximate-equivalence 대표로 중복 항목을 삭제하면 `a≈b`, `b≈c`, `a≉c`에서 `[a,b] i. c`의 유일한 일치 위치를 잃는다. 이 부정 사례를 별도 연구 테스트에 추가했다.

**제안하는 `ComparisonPolicy` 증명 입력(설계 전용, 미구현):** (1) 호출할 때 해석된 J primitive/valence와 `!.t` override 유무, (2) 현재 `9!:19` 정책 값 및 의미론적 버전/소유 경계(호출 전/후 변경 포함), (3) argument dtype과 rank/cell/frame, boxed/sparse 구조, (4) 사용한 equality predicate의 정체성과 NaN/±0/Infinity/복소수 규칙, (5) 허용되는 tolerance 범위와 검증 witness, (6) source 값·원본 인덱스·First/Last/Presence 결과 모드, (7) 관찰 가능한 오류·효과 경계 및 순차 fallback을 기록해야 한다. 정책·입력·임시 테이블 lifetime이 바뀌면 prepared index를 재사용하지 않는다. A3 `SearchDescriptor`에는 **의미·정책 출처만** 보존하고, 테이블 구조와 비용은 Physical 소유다.

**실행 승인 필요조건(미승인 유지):** CPU finite scalar float 검색에서조차 **비교 정책을 실제로 witness/guard**하고, 모든 가능 일치 위치가 후보로 보존된다는 proof가 있으며, 후보마다 정확히 동일한 semantic comparator로 재검사하고, 최소/최대 원본 위치 및 no-match sentinel을 유지해야 한다. 가드 실패는 observable effect 전에 일반 순차 검색으로 돌아가야 한다. `NaN`/무한대/underflow, 혼합 타입, rank/cell, boxed/sparse, C와 Rust 기본 비교 차이를 회피하는 **명시적 지원 범위**가 필요하다. 지금 단계에서 proof는 고정 Rust `near`에만 제한되고, C differential·동적 policy와 `!.t`의 등가는 **미검증**이다. 따라서 `LoweringRegistry`의 `TolerantNeighborHash=NeedsSemanticProof`와 테스트 전용 배선을 유지한다.


<a id="p7-고정-비교-정책-스냅샷-구현--dynamic-j-cct-미지원-경계-2026-10-06"></a>
###### P.7 고정 비교 정책 스냅샷 구현 — dynamic J CCT 미지원 경계 (2026-10-06)

> **역사적 상태: 아래 `FixedRustNearV0` 실행 설명은 첫 구현 당시 기록이다. 현재 기본 CPU 실행은 §P.9의 `PinnedJDefaultCctV0`로 교체됐다.**

**구현:** `src/comparison_policy.rs`에 내부 `ComparisonPolicySnapshot`과 유일하게 구성 가능한 `FixedRustNearV0` 식별자를 둔다. 이 스냅샷은 현재 `kernels::near`에서 사용하던 `a == b || finite(a,b) && |a-b| <= 2^-44 max(|a|,|b|)` 식을 그대로 소유한다. `kernels::near`는 이를 호출하도록 위임하며, `src/index_ops.rs::lookup`과 `find`는 **검색 호출당 한 번** 동일 정책을 포착해 각각 `i.`/`i:`/`e.`, `E.`의 atom comparisons에 전달한다. 외부 `expansion.rs` 사용을 위한 `atom_eq` 래퍼는 그대로 유지한다. Graph IR·A3 `SearchDescriptor`·Physical index/table 선택을 수정하지 않았고, float 검색은 여전히 **원본 순차 경로**를 사용한다. Int/Bool exact-only prehash는 float tolerance와 독립이며 변경하지 않았다.

**새 회귀 테스트(작성만, 실행 미확인):** `src/comparison_policy.rs`에서 비추이 연쇄, ±0, NaN, ±Infinity, 최소 subnormal 대 zero를 검사한다. `src/index_ops.rs`의 `fixed_float_policy_preserves_first_last_membership_and_find`는 다중 일치·근사적 비추이성·missing sentinel 및 4가지 검색 결과 `i.`·`i:`·`e.`·`E.`를 독립적으로 확인한다. 기존 연구 harness `src/tolerant_search.rs`는 추가한 정책 모듈과 별개로 test-only다. 입력형 불변성과 정책 동일성은 코드 검토로 확인하되 **Cargo fmt/test/clippy, 실제 jsource 실행, differential/benchmark는 미실행**이다.

**설계상 의미:** '비교 정책'을 실행 코드의 명시적 입력으로 캡처하는 첫 단계이고 **J의 `!.t`/전역 `9!:19`를 구현한 것이 아니다**. `FixedRustNearV0`는 현재 Rust CPU 함수의 정체성일 뿐, jsource `cct`-based 구현 동등성 witness가 아니다. 다음 기능을 활성화하려면 `Fit`-derived verb/동적 설정의 semantic ownership, 실제 호출 시점 policy identity + epoch, guarded supported-range proof, 전체 rank/cell·type·error 조건, 적절한 cache key/invalidation 및 독립 J C differential을 각각 충족해야 한다. 자동 TolerantNeighborHash 후보 선택, prepared float hash, LLVM/GPU 경로는 계속 금지한다. P.1 단계 5–7/12는 **[ ] 그대로**다.


<a id="p8-jsource-기본-비교식의-경계-불일치--미해결-의미론-차이-발견-2026-10-06"></a>
###### P.8 jsource 기본 비교식의 경계 불일치 — 미해결 의미론 차이 발견 (2026-10-06)

> **역사적 발견: 아래 8개 경계 불일치는 §P.9 실행 수정 이전의 상태다. 고정 기본 CCT는 이후 3방향 검증에서 수렴했지만 동적 CCT/Fit 구현을 뜻하지 않는다.**

**독립 source-first 재감사:** 고정한 jsource [`jsrc/vcomp.h::TCMPEQ`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vcomp.h), [`jsrc/i.c` 초기화](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/i.c), [`jsrc/viavx.h::jeqd`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx.h)를 추가 확인했다. 기본 `jt->cct=1.0-FUZZ`, `FUZZ=2^-44`이며, C 소스의 단일 float 비교식은 `(a > cct*b) != (b <= cct*a)`다. 검색 계열의 tolerant probe 역시 `jeqd`를 통해 `TCMPEQ`를 호출한다. [`jsrc/xa.c`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/xa.c)에서 `9!:19` 입력은 `0 <= t <= 5.820766091e-11`을 검사하고 `cct=1.0-t`로 저장한다. 유효 범위와 변경 수명은 이 원본에 맞춰야 한다.

**소스 식의 IEEE-754 모델을 통한 실제 반례(실행한 J C 바이너리 아님):** `t=2^-44`, `a=1.0`, `b=1.0-t`이면 현재 RustJ `near`의 `|a-b| <= t*max(...)`는 **true**, 위 jsource C macro를 이진64 연산으로 모델링한 식은 **false**다. `b=1.0+t`에서도 같은 불일치가 나타난다. 이는 반올림·경계의 **strict-vs-inclusive** 차이여서, 수학적으로 같은 허용오차 폭을 논의한 것만으로 기계 수준의 결과 동등성이 보장되지 않는다는 구체적 증거다. 별도 Python IEEE-754 반증 탐색은 **의도적으로 허용오차 경계에 집중한 120,000쌍에서 19,968개의 모델 불일치**를 찾았다. 이 빈도는 일반 입력의 J-vs-Rust mismatch rate가 아니며 C 원본 실행 결과도 아니다. 부호·크기·subnormal·다양한 target CPU는 별도 검증해야 한다.

**독립 C 재현(전체 J 실행 아님):** pinned `TCMPEQ`의 식을 포함한 작은 C 프로그램을 Debian GCC **14.2.0**, `cc -std=c11 -O0` 및 `-O2`로 각각 컴파일·실행했다. 두 빌드 모두 `(a,b)=(1,1-2^-44)` 및 `(1,1+2^-44)`에서 기존 RustJ 수식 모델은 **true**, upstream C 매크로 수식은 **false**를 반환했다. ±0·Infinity·NaN 검산 사례도 두 빌드에서 일치했다. [`reports/cct-macro-boundary-probe.json`](../../../reports/cct-macro-boundary-probe.json)에 입력·컴파일러·결과·미실행 범위를 기록했다. **Rust 컴파일·전체 jsource 프로그램 실행·J oracle 차등 테스트는 여전히 하지 못했다**(`cargo`, `rustc`, J 인터프리터가 현재 환경에 없음). 이 자료는 소스 매크로 재현이지 J 엔진 통합 검증이 아니다.

**코드·승인 상태:** `src/comparison_policy.rs`의 테스트 전용 `source_cct_macro_model` 및 `jsource_cct_macro_model_exposes_fixed_near_boundary_gap`이 이 반례를 기계적 회귀 입력으로 기록했다. `FixedRustNearV0`의 기존 실행 의미는 **의도적으로 변경하지 않았다**. 실제 J C binary/엔진을 사용한 `=, i., i:, e., E.` 경계 검증과 dtype/rank/fit·error 차이를 확정한 뒤, 공통 comparator를 source-equivalent로 교체할지 결정해야 한다. 임의로 float hash를 활성화하거나 source model을 완전한 J runtime oracle로 선언하지 않는다. **P.1 #5 및 #12는 여전히 미완료이며, 이전 P.4의 수학적 후보-상계 증명은 *구 Rust fixed-near comparator*에 한정된다.**



<a id="p9-기본-j-cct-비교-정책으로-실행-경로-정렬--fw-03-부분-증거-2026-10-06"></a>
###### P.9 기본 J CCT 비교 정책으로 실행 경로 정렬 — FW-03 부분 증거 (2026-10-06)

**변경 이유와 최초 관찰.** `P.8`의 반례를 고정 `jsource@13994ffa1ed5f06f79fad6e9822a7ed2d29b1528` C binary에서 직접 확인했다. [FW-03 진단 CI 37445080131](https://github.com/yunskim/RustJ/actions/runs/37445080131)은 j64·j64avx2 × Rust default·portable 네 조합 각각 **48개 중 40 일치 / 8 C-vs-Rust 불일치 / Rust baseline 대 optimized 불일치 0**을 기록했다. 8개는 `1.0 ± 2^-44`의 `=`, `i.`, `i:`, `e.`(상·하한 × 네 연산)였다. 원본 삼방향 값·pin·binary 해시는 CI artifact `reports/tolerance-audit-*.json`에 보존한다.

**최소 구현.** `src/comparison_policy.rs`에 `PinnedJDefaultCctV0`을 추가하고 `float_equal(a,b) := (a > cct*b) != (b <= cct*a)`, `cct = 1 - 2^-44`로 고정했다. `src/kernels.rs::near`(비교), `src/index_ops.rs`(최적화 검색), `src/search_reference.rs`(독립 순차 검색)에 각각 사용한다. 이전 `FixedRustNearV0`은 `#[cfg(test)]` 회귀 witness로만 보존된다. 원본 J Graph/A3, cost model, exact Int/Bool hash 선택, float 순차 검색 정책은 유지한다.

**실제 실행 검증.** [Linux milestone CI 37445891526](https://github.com/yunskim/RustJ/actions/runs/37445891526)에서 고정 C J j64·j64avx2 × Rust default·portable 네 조합 각각 **48/48 일치, 총 192/192, Rust baseline 대 optimized 불일치 0**. `tools/tolerance_audit.py`는 기존 8개 불일치를 조용히 통과시키지 않고 `previously_known_gaps_now_matching`으로 분류하며, 새 C-vs-Rust 불일치 또는 두 Rust 실행 경로 불일치가 생기면 CI 실패로 보고한다. 이는 **기본 CCT·dense scalar 경계 표본**의 수용 증거이며 full J 동등성 증명은 아니다.

**잔여 의무.** `9!:19` 동적 전역 설정·`!.t` Fit, boxed/sparse·복소수·Rank/Cell·오류 순서 및 J numeric 타입 전체는 검증/구현이 남았다. `FW-03`, `FW-04`, `JX-04`, `P.1 #5/#12`는 **[ ]** 유지. P.4의 *구 Rust near* 한정 tolerant hashing 후보·수학적 증명을 새 default J CCT 해시 정당화로 승격하지 않는다. 자동 tolerant hash, float prepared hash, GPU/LLVM 경로도 아직 미허가다.

<a id="p10-rank-zero-frame-fill-cell-실행검증--fw-04-부분-증거-2026-10-06"></a>
###### P.10 Rank zero-frame fill-cell 실행·검증 — FW-04 부분 증거 (2026-10-06)

**jsource 의미론과 문제 재현.** 고정 [`jsrc/cr.c`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c)의 `jtrank1ex`·`jtrank2ex`는 frame의 셀 수가 0이면 type-correct fill cell 또는 반복되지 않는 첫 실제 cell을 이용해 operand를 평가하고, 그 결과의 type·shape로 빈 배열을 조립한다. 계산 오류 발생 시 `WITHDEBUGOFF`/non-exigent error fallback 등의 추가 계약도 있다. 이전 RustJ는 `(i.0 3) (i."1 1) (i.0 3)`을 `Unsupported`로 거부했고, [기준 Rank 진단 CI 37446732381](https://github.com/yunskim/RustJ/actions/runs/37446732381)에서 각 변형 14건 중 **13 일치 / 1 이 의미 불일치**를 확인했다.

**구현 범위.** `src/value.rs::rank_fill_cell`과 `empty_rank_result`에서 supported dense Bool/Int/Float/Char 입력의 첫 실제 cell 또는 type-correct fill cell, 그리고 결과 type·cell shape + 빈 frame 조립을 제공한다. `src/logical_executor.rs::apply_ranked`의 dyad/monad 및 `src/kernels.rs::ranked_dyad_ranks`·`ranked`가 공통 Rank/CellApply 요구를 사용한다. 검색 전용 Graph node·임의 물리 전략·optimizer 승인은 추가하지 않았다. **`primitive_fill` 실제 primitive witness가 있는 경우에만 zero-frame 자동 fill 평가**를 허용한다. 사용자 정의/동적 동사의 empty-frame 호출은 원본 J에서 효과를 발생시킬 수 있으므로 `Unsupported`로 명시하고, 증거 없는 부작용 speculative 실행을 금지한다. Boxed/sparse fill과 computational error fallback도 미지원이다. `requires_empty_frame_prototype` 및 정적 `Facts::default`는 지우지 않고 보수적으로 보존한다.

**실제 3방향 증거.** [Linux CI 37449102885](https://github.com/yunskim/RustJ/actions/runs/37449102885)의 check·`j64`·`j64avx2` × Rust default/portable가 모두 성공. `python3 tools/ranked_search_audit.py --reference-revision 13994ffa1ed5f06f79fad6e9822a7ed2d29b1528 --report reports/ranked-search-audit-<variant>-<backend>.json` 명령의 20개 fixture에서 **각 20/20, 총 80/80 C J · 독립 Rust semantic reference · Rust 최적화 실행 일치, 불일치 0**. 빈 frame i./i:/e., 문자/float, primitive 덧셈·reduction, scalar/row/broadcast와 frame mismatch를 포함한다. [Basis compile probe 37449102980](https://github.com/yunskim/RustJ/actions/runs/37449102980)도 성공하여 Rust tests(기본/portable)와 Clippy 등의 검사 근거를 별도 제공한다. 새 의미 지원을 반영해 `tests/analysis.rs`·`tests/semantics.rs`의 오래된 Unsupported 기대를 정정했고, 사용자 정의 동사의 빈 frame 효과 미검증 경계는 `tests/empty_scope.rs`로 계속 보호한다.

**미완료 / 다음 게이트.** J의 일반적인 boxed·sparse filler, 계산 오류 억제·회복, 효과/이름·오류 순서, mixed-type cell·heterogeneous cell-result padding 및 rank-implicit-loop 전체는 이 20개 통과로 증명되지 않는다. `FW-04`·`JX-04` 및 Rank 관련 상위 게이트는 **[ ] 유지**한다. 다음 변경은 C oracle에 부정 사례를 먼저 고정하고, 효과/자원 한계를 보존하는 의미론 계약을 독립 검증한 뒤 수용한다.

<a id="rank-cellapply-followups"></a>

<a id="p11-rankcellapply-후속-이행-체크리스트--fw-04jx-04-세부-실행-계획-2026-10-06"></a>
###### P.11 Rank/CellApply 후속 이행 체크리스트 — FW-04·JX-04 세부 실행 계획 (2026-10-06)

**목적·수용 경계.** §P.10에서 구현한 zero-frame Rank의 dense subset을 출발점으로, J의 일반 Rank/CellApply 의미론까지 검증·확장할 작업표다. 상위 §O.5 FW-04와 §Q JX-04의 **하위 실행 목록**이며 별도 최적화 허가 체계가 아니다. **현황: 범위 한정 5/12 완료, FW-04·JX-04는 [ ] 유지.** 고정된 [jsource@13994ffa](https://github.com/jsoftware/jsource/tree/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc)의 cr.c (jtrank1ex/jtrank2ex)와 result.h를 기준으로 한다. 아래 완료 항목은 지원 subset에 한정한다.

| ID / 단계 | 체크 | 구현 소유자·완료 판정 | 반드시 보존할 검증 증거·부정 사례 |
|---|---|---|---|
| RK-01 / A·M2 | [x] **고정 C 원본의 빈 frame 계약 조사** | cr.c의 fill-cell/첫 실제 cell 평가 → 결과 타입·shape 조립 및 오류 fallback 차이를 §P.10에 명시 | [기준 CI 37446732381](https://github.com/yunskim/RustJ/actions/runs/37446732381) 4환경 각 13/14 일치, 빈 frame 1건 불일치 기록(수정 전) |
| RK-02 / A·M2 | [x] **Dense fill-cell 값·타입·shape 구성** | src/value.rs의 rank_fill_cell/empty_rank_result: Bool·Int·Float·Char 타입별 fill, 비어 있지 않은 인자의 첫 실제 cell, 빈 결과 dtype 유지 | 지원된 타입/shape 및 boxed 거부 단위 테스트. Boxed/sparse의 임의 0 치환 금지 |
| RK-03 / A·M2 | [x] **일반 Rank monad/dyad 배선** | src/logical_executor.rs, src/kernels.rs, src/runtime.rs: 검색 전용 예외가 아닌 공통 CellApply 실행. 정적 Facts와 requires_empty_frame_prototype를 보수적으로 유지 | tests/semantic.rs, tests/analysis.rs; 결과 shape·데이터 타입 검사 |
| RK-04 / A·M2 | [x] **미검증 사용자 효과 자동 실행 차단** | runtime primitive witness로만 빈 frame fill 실행 허가. 동적·명시 정의 본문은 독립 효과 계약 없이는 Unsupported | tests/empty_scope.rs의 카운터·바인딩·정의 본문 미호출 검사. 실제 사용자 정의 Rank 지원 완료 아님 |
| RK-05 / A·FW-04 | [x] **현 지원 subset 독립 3경로 회귀** | tools/ranked_search_audit.py, 테스트와 Linux CI에서 C·Rust 순차 기준·Rust 최적화 실행을 별도로 비교 | [Linux CI 37449102885](https://github.com/yunskim/RustJ/actions/runs/37449102885) j64·j64avx2 × default·portable 각 20/20, 총 80/80; [Basis probe 37449102980](https://github.com/yunskim/RustJ/actions/runs/37449102980) 성공 |
| RK-06 / B·M2 | [x] **빈 frame·Rank 적대 입력 확대** | tools/ranked_search_audit.py·tests/semantic.rs: frame 선두/중간 0, 한쪽만 빈 배열, 길이 불일치, 음수·과대 rank, 타입 불일치, 중첩 Rank의 실행/오류 비교 | 원본 C 사례 먼저 고정, 결과값·오류 종류·Rust Unsupported를 별도 기록. 새 입력을 4환경 3경로로 검증  **진단 증거(2026-10-06; [CI 37452153562](https://github.com/yunskim/RustJ/actions/runs/37452153562)):** pinned C j64/j64avx2 × Rust default/portable 4조합 각각 24개 중 22개 일치, 2개 `rust_semantic_mismatch`(`empty_type_mismatch`, `nested_rank_empty`). 전체 green은 이 적대 corpus가 탐색/비수용 게이트이기 때문. [`45af152`](https://github.com/yunskim/RustJ/commit/45af1526c41e33d9a5b3fda8bf7bfd5bf92d9b4e) 및 [`59bb6d8`](https://github.com/yunskim/RustJ/commit/59bb6d8adff86db0fd8c1d43bb41cc64bbc6dc52)에서 해당 3경로 관찰값을 로그에 노출하고 테스트를 추가. **당시에는 2건 미해결이었으며 아래 32건 엄격 재검증으로 해당 범위를 수용했다.**  **부분 수정·CI 검증(2026-10-06):** [`4d5f903`](https://github.com/yunskim/RustJ/commit/4d5f90348a91ece4c15139d0ae56affb1ade68c6)·[`edad61b`](https://github.com/yunskim/RustJ/commit/edad61bc17ebeb46198f1d1f050bcb9bc95f0a42)에서 *원시 동사→Rank→Rank*의 제한된 값 전용 fill-cell 증거를 독립 A3/Rust 실행기 양쪽에 반영; [`a4c5953`](https://github.com/yunskim/RustJ/commit/a4c5953349ea8fba34df7bdcaed3e683ee773b01)·[`f074620`](https://github.com/yunskim/RustJ/commit/f074620e6317e5f86ffb425f07fba00a00847cdb) 정상·사용자 효과 차단 테스트. [Linux CI 37453410364](https://github.com/yunskim/RustJ/actions/runs/37453410364) check + 4 reference jobs **5/5 성공**; 기존 24개 적대 corpus는 각 조합 **23 pass / 1 `rust_semantic_mismatch`(`empty_type_mismatch`)**로 감소. 중첩 Rank 1건 해결, **당시 RK-06 [ ]였으며 이후 32건을 별도 검증했다. Rank 전체·ZF-IR-03 증명 완료를 뜻하지 않는다.** |
| RK-07 / B·M2 — **다음 작업** | [ ] **계산 오류 억제·재시도·오류 우선순위** | cr.c의 non-exigent 오류→fill 결과 대체, exigent 오류 전달, 타입 불일치 재시도의 조건을 분리; src/error.rs 및 Rank executor 계약 갱신 | 계산 오류, 타입 충돌, overflow, shape 오류의 원본 동작·오류 순서 고정. 80/80에 포함되지 않은 범위 |
| RK-08 / B·M3·FW-07 | [ ] **사용자 정의 동사·효과·동적 이름** | src/runtime.rs에서 동적 이름 조회 시점, fill 동사 실행 횟수, 부작용, 실패 후 재실행 가능성 및 guarded fallback을 정의 | C oracle의 카운터 증가·이름 변경·nested user verb·오류 후 상태 비교. 사전 증명 없는 speculative 실행 금지 유지 |
| RK-09 / B·M3 | [ ] **boxed·sparse fill/prototype** | src/value.rs·src/storage.rs·src/sparse.rs: box 내부 fill, sparse 축·fill 값과 atom type·shape 보존 | 타입별 C 결과와 비교; 임의 dense/0으로 일반화하거나 미지원 실행을 통과로 세지 않음 |
| RK-10 / B·M3 | [ ] **이질적 결과 cell·타입 승격·padding** | src/assembly.rs·src/logical_executor.rs·src/kernels.rs: result.h 기준 혼합 dtype·shape, fill/padding, 전체 cell 평가 후 오류 순서를 분리 | mixed result shape, char/numeric, empty cells, size mismatch, error precedence를 C·두 Rust 경로 비교 |
| RK-11 / C·M3 | [ ] **Rank + 파생 동사/implicit loop 연동** | semantic/Logical/Graph의 원래 modifier 관계와 rank/cell/frame facts 보존; fork/hook/@:/중첩 rank·late name은 실행 가능성과 별개 | negative/oversized rank, modifier composition, frame 반복, late binding의 3경로 테스트. Graph 후보가 승인된 실행으로 자동 변환되지 않음. **빈 frame과 빈 cell의 Graph 의미론·회귀 하위 체크리스트 §P.12 ZF-IR-01~04 참조; RK-11 전체 미완료.** |
| RK-12 / D·FW-04/JX-04 | [ ] **지원 범위·수용 증거 최종 대조** | RK-06~11 각각에 J 기준·baseline 독립성·Guard·효과/오류·자원 한계 증거를 연결하고 미지원 범위를 명시 | pinned C commit·명령·4조합 CI·cases/실패 분류·reports JSON을 남긴 뒤 해당 단계의 완료 여부를 개별 판정. 상위 FW-04/JX-04는 다른 의무가 남으면 [ ] |

**반복 작업 순서와 상태 갱신 규칙.** RK-06 → RK-07 → RK-08 → RK-09 → RK-10 → RK-11 → RK-12로 진행한다. 매 작업은 ① C 원본의 정상/부정 입력부터 고정 ② C / Rust 독립 semantic reference / Rust 실행 차이를 세 범주로 분류 ③ 가장 작은 공통 의미론 수정 ④ Rust default·portable fmt/clippy/test와 C j64·j64avx2 차분 실행 ⑤ CI 링크·수치·커밋·불일치를 해당 RK 행에 기록한 뒤에만 [x] 처리. **Linux CI 통과만으로 미지원 Rank를 전체 합격 처리하지 않으며** 부정 사례와 실행 효과를 임의로 묵살하지 않는다. M2 프런트엔드 수렴 우선순위는 유지한다. GPU/Hash/Graph 후보 선택은 FW-05~FW-13의 독립 증명·가드·자원·비용 허가 전에 열지 않는다.


<a id="rank-graph-zero-frame"></a>

<a id="p12-빈-frame빈-cell의-j-graph-ir-의미론--구조-증거와-실행-생략-허가-분리-2026-10-06"></a>
##### P.12 빈 Frame·빈 Cell의 J Graph IR 의미론 — 구조 증거와 실행 생략 허가 분리 (2026-10-06)

**설계 결론.** 빈 배열은 별도의 `EmptyArray` 연산 노드가 아니라 *논리 Shape·타입·원래 Rank/CellApply 적용 관계*에 속한다. `0 3`과 `2 0`은 둘 다 원소 수가 0이지만, rank 1을 적용할 때 전자는 **frame=[0], cell=[3]**(실제 셀 호출 0회, J fill-cell 의미론 필요), 후자는 **frame=[2], cell=[0]**(빈 셀에 대한 실제 호출 2회)이다. 따라서 `element_count == 0`은 **연산 실행이나 효과·오류 평가가 0회**라는 증명이 아니다. 고정 jsource `jsrc/cr.c::jtrank1ex/jtrank2ex`와 §P.10의 fill 계약을 참조한다.

**구현 경계.** `src/facts.rs`의 `rank_plan_for_shapes`는 frame/cell 분해를 단일 구조적 의미론으로 제공하고, `RankPlan::frame_execution()`은 `ZeroFrameNeedsFill / CellsPresent / IncompatibleFrames`만 구별한다. `has_empty_input_cell()`은 별도 속성이다. `src/j_graph_ir.rs::Plan::rank_frame_plan(ValueId)`은 실제 `GraphForm::Rank`이며 requested rank와 입력 Shape을 알 때만 **읽기 전용** RankPlan을 반환한다. `src/facts.rs`의 J Graph projection과 A3 inference가 같은 rank/frame 분해를 사용한다. 기존 `GraphFacts`의 dtype/shape가 빈 frame의 결과에 대해 **Unknown**이면 그대로 남겨두고, J Graph에 물리적 버퍼·실행 결정이나 새로운 형식 노드를 추가하지 않는다.

**의무와 금지 사항.** 구조적 `ZeroFrameNeedsFill`은 `ProvenEmptyResult`도 `SafeToElide`도 아니다. 원본 함수의 부작용·동적 이름·computational/error fallback·boxed/sparse filler·결과 cell 타입/shape를 별도 증명하지 못하면 fill-cell 평가를 생략하는 후보를 선택하지 않는다. A3 `IterationDomain`에 0 크기 frame 축이 보이더라도 kernel 생략/할당 제거/Graph fusion을 자동 허가하지 않는다. 생략 후보와 실제 선택은 FW-05~FW-13의 증거·Guard·자원·비용 게이트에 종속된다.

**타 배열 컴파일러 비교와 적용 경계(2026-10-06).** 아래 네 모델은 동일한 J 의미론을 구현했다는 근거가 아니라 서로 다른 *분석·최적화 단계*의 참고 자료다.

| 참고 시스템 | 확인한 접근 | RustJ 적용과 한계 |
|---|---|---|
| [XLA ZeroSizedHloElimination](https://github.com/openxla/xla/blob/main/xla/hlo/transforms/simplifiers/zero_sized_hlo_elimination.h), [패스 설명](https://openxla.org/xla/hlo_passes) | 원소 수 0의 HLO 출력을 빈 상수로 대체 | **빈 결과의 의미론 및 생략 적법성을 먼저 증명한 경우**에만 유사한 후보 생성. J의 fill-cell·효과·오류 무시 금지 |
| [Futhark 크기 타입](https://www.futhark-lang.org/blog/2020-03-15-futhark-0.15.1-released.html) | `[n]a`처럼 배열 크기·원소 타입을 표현; `[0][2]i32`와 `[2][0]i32` 구별 | Rank frame 기하와 독립적으로 **결과 cell의 dtype/shape**를 증명하는 분석을 참고. J 동적 이름/효과/이질적 조립은 별도 |
| [MLIR Linalg](https://mlir.llvm.org/docs/Dialects/Linalg/) | 인덱싱·반복 공간과 region의 계산 본문을 분리 | `CellApply` 반복 구조를 실제 cell 적용·결과 조립과 분리해 보존. `tensor.empty`는 원소 수가 0이라는 뜻이 아니라 내용 미지정 텐서 생성 |
| [StableHLO reduce](https://openxla.org/stablehlo/spec#reduce) | 축·reducer·init_values·결과 형식을 명시 | 결과 형식이 계약으로 확정되는 reduction에만 별도 추론 적용. J 일반 `/`와 Rank fill-cell을 초기값 기반 reduction으로 무조건 치환하지 않음 |

**ZF-IR-03의 추가 상세 계약(계획·미구현).** `RankPlan::frame_execution()`은 작업 생략 허가가 아니다. 불변 원본 J Graph 옆에 재계산 가능한 증명 sidecar를 두고 (1) graph/value 출처·원본 버전/동적 이름, (2) 결과-cell J dtype/shape·boxed/sparse fill·조립, (3) fill-cell 결과를 재구성하는 의미론, (4) 효과·오류 종류/순서·핸들러, (5) 최초 관찰 가능 효과 이전 Guard와 정확한 semantic reference fallback을 **각각** `Unknown / Proven(witness) / Guarded(guard+fallback) / Disproven`으로 구별한다. 구조상 `frame=[0]`이거나 일부 결과 Shape을 안다는 사실만으로 `Unknown`이 `Proven`이 되지 않는다. `Guarded` 역시 guard/fallback 검증 전에 선택 불가. 원본은 불변, 실행 선택은 FW-05~FW-13과 ZF-IR-04 뒤에만 둔다.

**작업 순서.** RK-06의 C oracle 기준 적대 Rank/empty-frame fixture → RK-07~10 오류/효과/boxed/sparse/조립 검증 → ZF-IR-03 sidecar·Unknown/Guard negative tests → ZF-IR-04 3경로 CPU/물리 승인. M2 우선순위를 바꾸지 않으며 위 계획 문서화만으로 ZF-IR-03/04·FW-06/07의 [ ]를 변경하지 않는다.

**정확한 RK-06 불일치 입력과 결과(추가 검증).** [새 진단 CI 37452934704](https://github.com/yunskim/RustJ/actions/runs/37452934704)의 `j64/default` job 출력에서 `(0 3 $ 'abc') (+"1 1) (i.0 3)`은 pinned jsource의 **정수형(타입코드 4), shape `[0,3]`, 빈 데이터**에 대해 독립 Rust reference/optimized가 모두 **`domain error`**를 반환했다. `(i.0 3) ((+"0 0)"1 1) (i.0 3)`은 C의 **Int(타입코드 4), shape `[0,3]`, 빈 데이터**에 대해 Rust 양 경로가 모두 **`unsupported`**를 반환했다. J 원본의 fill-cell 계산 오류 처리(`cr.c::jtrank2ex`의 비필수 오류 억제·재시도 포함)와 계층적 Rank 재귀 의미론을 독립 확인해야 하며, **빈 결과를 전부 원래 입력 타입으로 처리하거나 단순히 `domain error`를 무시하는 수정은 금지한다.** 위 증거는 `j64/default` job의 실제 세 경로 출력이다.

**RK-06 추가 C-oracle 관찰 항목(작성 완료, CI 검증 대기).** [`a4ffb0c`](https://github.com/yunskim/RustJ/commit/a4ffb0c240e5a4271bb9f20b86627f0f5fca157f), [`b241c97`](https://github.com/yunskim/RustJ/commit/b241c9730e1993cda3a402b9256f1e184d9fc501), [`d5f8a03`](https://github.com/yunskim/RustJ/commit/d5f8a0312952e3d47ee53a48b7eb6b1a67c0d012)에서 `tools/ranked_search_audit.py --adversarial`을 기존 24→**32 입력**으로 확대했다. 새 8건은 char/int 방향, char/float·bool, 한쪽 인자만 빈 frame, 정상 frame의 실제 char/int 셀과 빈 셀, 중첩 Rank 비빈 셀을 포함한다. `jsrc/cr.c::jtrank2ex`의 `EVINHOMO` fill-cell 재시도(원래 데이터 비어 있는지에 따라 타입 선택), *재시도 후* 비필수 계산 오류의 scalar 0 대체, exigent 오류 전달을 **서로 다른 의미론 경계**로 관찰하기 위한 탐색 corpus다. **실제 고정 C oracle 교차 검증(2026-10-06):** [Linux CI 37453629236](https://github.com/yunskim/RustJ/actions/runs/37453629236)의 check + pinned j64/j64avx2 × Rust default/portable **5/5 jobs 성공**. 네 reference 조합에서 각각 **32건 중 25 pass / 7 `rust_semantic_mismatch`**, 합계 100/128 일치·28 불일치 관찰(독립된 7개 입력 × 4조합). 불일치 이름은 `empty_type_mismatch`, `empty_type_mismatch_reversed`, `empty_char_float_fill`, `empty_char_bool_fill`, `empty_char_left_real_right`, `empty_char_right_real_left`, `positive_frame_empty_char_cells`. 앞의 6개에서 pinned C는 정수형 shape `[0,3]`, 마지막 1개는 정수형 shape `[2,0]`을 반환하고, Rust 기준/최적화 두 경로는 모두 `domain error`. `nonempty_char_int_domain`과 `nested_rank_nonempty_cells`은 일치. 이 **green CI는 비수용 진단 실행 성공**이며 해당 7개를 성공·허용된 불일치로 계산하지 않는다. 위 25/32는 수정 전 이력이다. 아래에서는 RK-06의 **고정 32건 유한 집합만** 엄격 회귀 기준으로 수용한다. RK-07·ZF-IR-03은 [ ] 유지하며 기존 20건 엄격 회귀 gate도 변경하지 않는다.

**RK-06 유한 차분 집합 완료 판정(2026-10-06; RK-07 전체 완료는 아님).** [최종 엄격 CI 37458845392](https://github.com/yunskim/RustJ/actions/runs/37458845392)에서 **check + pinned C j64/j64avx2 × Rust default/portable 4개 reference job, 총 5/5 성공**. 동일 고정 C revision `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`에 대해 새 RK-06 **32건 × 4조합 = 128개의 3경로 비교 모두 `pass`, mismatch 0**. 새 `--adversarial --gate-adversarial`은 각 불일치에서 종료코드 1을 반환하고 정확한 C / Rust reference / Rust optimized 관찰을 출력·보고서로 남기며, [`365c3ad`](https://github.com/yunskim/RustJ/commit/365c3adcb9e716e34f3083f50c317e243a51ad23)에서 Linux CI 네 조합의 **차단 게이트**로 승격했다. 기존 20건 Rank 엄격 gate도 유지. [`1644644`](https://github.com/yunskim/RustJ/commit/1644644444d8e51ee2097f4acd659b1e4b893013)·[`b631f6f`](https://github.com/yunskim/RustJ/commit/b631f6fb488f449fcc4c35b44da6ac5bcca444d8)에서 부정 사례가 실패를 유발하는 계약도 시험했다. 이 **32개 고정·dense 입력 집합의 RK-06만 [x]**이며, 무한한 J Rank 일반 의미론을 승인한 것이 아니다.

**RK-07 현재 구현 경계와 잔여 수용 조건.** [`9ab2952`](https://github.com/yunskim/RustJ/commit/9ab2952100ae7b2ae1e6338a761ecf4fc798804f)에서 값 전용 Rank의 **결과 Frame=0인 fill-cell 계산 `Domain` 오류**에 한정해 integer-zero 복구를 추가했다. [`cbd13af`](https://github.com/yunskim/RustJ/commit/cbd13af0864a85452a2cface91f283bf07b9f4e2)·[`85f1732`](https://github.com/yunskim/RustJ/commit/85f1732bb29da8b1573781e6fb6b86711d91468e)·[`a5541b2`](https://github.com/yunskim/RustJ/commit/a5541b22705b4f53aeb8a2094d958d5d749ec557)에서 *원시 `+`의 내재적 Rank 0* 결과 셀 shape를 유지하여 [0,3]을 [0]으로 잘못 축소하지 않는다. [`a23bcf4`](https://github.com/yunskim/RustJ/commit/a23bcf40b104e359fe135da9d76c8d0ae521e672)·[`6be596a`](https://github.com/yunskim/RustJ/commit/6be596a81ccec1edb346fe2554a42e7811c406ef)에서 양수 바깥 Frame [2] 안의 원자 수 0인 `+` 셀도 정수형 [2,0]으로 처리한다. [`0d85f03`](https://github.com/yunskim/RustJ/commit/0d85f030b00122d2239150a8d868327ff8810f0c)·[`75a36eb`](https://github.com/yunskim/RustJ/commit/75a36eb7886ca2f68d9527412e41453b93d9af62)에는 위 7건 및 **실제 nonempty char/int의 Domain**, Frame 불일치의 Length, 미지원·자원 오류의 전달을 확인하는 회귀가 있다. 근거는 고정 원본 [`jsrc/cr.c::jtrank2ex` L397–418](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c#L397-L418), [`jtrank2ex0` L541–569](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c#L541-L569), [`jerr.h::EXIGENTERROR` L89–90](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/jerr.h#L89-L90). **잔여 [ ]:** 일부 `,`/dense의 `EVINHOMO` 타입 재시도는 아래와 같이 구현했으나, 다른 동사까지 일반화할 **명시적 내부 오류 분류와 provenance**, 나머지 non-exigent/exigent 오류 계층·오류 우선순위, 사용자 동사 효과·이름 버전·boxed/sparse 및 비균일 셀 조립은 미완료다. `Unsupported`·`Limit` 등을 임의 복구하지 않으며 zero-frame shape fact만으로 실행 생략 또는 GPU lowering을 허용하지 않는다. **RK-07·08~12, ZF-IR-03/04, FW-06/07은 [ ] 유지.**

**RK-07 — `EVINHOMO` 타입 재시도 1차 구현 및 고정 C 수용 경계 (2026-10-06).** 고정 원본 [`jsrc/cr.c::jtrank2ex` L397–418](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c#L397-L418)·[`jtrank2ex0` L541–569](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c#L541-L569)은 *fill-cell 첫 호출 → `EVINHOMO` 시 자료형 선택 → 값 변환이 아닌 해당 자료형의 기본 fill-cell 재생성 → 단 한 번 재시도 → 남은 non-exigent 오류는 정수 0, exigent 오류는 전달* 순서다. 원래 양쪽 배열 중 **원소가 존재하는 쪽**의 dtype이 우선이며 **모두 빈 배열**일 때만 [`j.h::TYPEPRIORITY`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/j.h#L2040-L2046)의 bool < char < int < float 우선순위를 따른다. 이 정보는 Graph IR의 Frame 크기만으로 얻을 수 없으므로 타입·원래 원자 수·실행 동사의 근거를 보존한다.

[`ae7258d`](https://github.com/yunskim/RustJ/commit/ae7258d2d391ee43cecb5477f1ccd52f96025e80)에서 RK-06의 기존 엄격 **32건을 건드리지 않고** 별도 `--retry-probes` 10건(빈 frame과 양수 frame 안 빈 cell, char/int 방향, char/float·bool, 단측 nonempty, 정상 오류/Frame 길이 불일치)을 추가했다. 처음 [탐색 CI 37460517931](https://github.com/yunskim/RustJ/actions/runs/37460517931)의 j64/default에서는 **3 pass / 7 불일치**였다. 예를 들어 C의 **int [0,6]** 또는 **float [0,6]** 결과에 Rust 양 경로는 **int [0]**을 반환했으며, 양수 바깥 frame의 혼합 빈 셀 C **int [2,0]**에 Rust는 Domain이었다. [`d192ad6`](https://github.com/yunskim/RustJ/commit/d192ad6889b6dbcdf654321b7668b040cde54ae9)은 입력 값을 버리고 목표 dtype과 shape만 보존하는 기본 filler 생성기를 추가했다. [`4e35d44`](https://github.com/yunskim/RustJ/commit/4e35d443eb4eb8f2c8fa4fc8528439fc18aa141e)·[`5485011`](https://github.com/yunskim/RustJ/commit/5485011de69fd2354d19059cd79612d1c4fcd3e8)·[`a6a254c`](https://github.com/yunskim/RustJ/commit/a6a254cac2cee70a963abd2b19a4ef2194f29632)은 **구체적 순수 원시 동사 `,`의 dense char/numeric 불균질 오류**에서만 1회 타입 재시도를 허용하고, [`48e8a6a`](https://github.com/yunskim/RustJ/commit/48e8a6a203e1f58f28063b0765f60a19ff59731e)은 양수 frame 내부 원자 수 0인 연결 셀의 출력 타입을 보존했다. [`b370ac8`](https://github.com/yunskim/RustJ/commit/b370ac855f6cc81b02a38c569541100f98f2356e)·[`177b18c`](https://github.com/yunskim/RustJ/commit/177b18c50c4c2ad70a83aa3d7aee793c86a065da)은 값·자료형·형상 및 1회 재시도, 실제 원소가 있는 일반 셀의 Domain, frame 불일치 Length, Limit 오류 비재시도를 회귀 테스트로 고정했다.

**수용 증거:** [Linux CI 37461525543](https://github.com/yunskim/RustJ/actions/runs/37461525543), 원본 C 고정 commit `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`. check(파이썬 테스트·fmt·clippy·Rust default/portable 테스트·빌드) + j64/j64avx2 × default/portable **5/5 작업 통과**. **RK-07 신규 10건 × 4조합 = 40/40 독립 C / Rust 의미 기준 / Rust 실행 비교 `pass`, 불일치 0**. [`37e04dc`](https://github.com/yunskim/RustJ/commit/37e04dc8109065e86e4c2bca333416d0c9b441f4)·[`706fa36`](https://github.com/yunskim/RustJ/commit/706fa367aa860760d5b4df3b17dbb9b26ccf2da3)·[`b245f9e`](https://github.com/yunskim/RustJ/commit/b245f9e8a27bac0ffd21b724a9ca8df0d18b941c)의 `--retry-probes --gate-retry-probes`가 각 불일치에서 CI를 실패시키며 C·두 Rust 경로의 실제 출력과 report JSON은 CI artifact로 보존한다. 기존 RK-06 32건/4조합 **엄격 게이트도 유지**한다.

| RK-07 하위 작업 | 상태 | 다음 검증 기준 |
|---|---|---|
| RK-07-E1 · `,` 원시 동사의 타입 재시도 | [x] 제한적 dense 구현 | 최초 실패 후 실제 원자 유무에 따른 타입 선택, 기본 fill-cell 생성, 1회 재시도. 10건 pinned C 차분 |
| RK-07-E2 · 증거의 지속성 | [x] 해당 10건의 엄격 게이트 | 4조합 40/40, 하나라도 차이면 CI 실패·report JSON 잔존 |
| RK-07-E3 · 내부 오류 종류 | [ ] | `EVINHOMO`의 일반화 가능한 내부 분류·실행 단계별 provenance, 다른 primitive의 비필수 오류와 구별; `Error::Domain`만 보고 임의로 재시도 금지  **부분 진전(2026-10-06, 전체 [ ]):** [`5aa7074`](https://github.com/yunskim/RustJ/commit/5aa7074762ca6eac1570bc17d728b9ee07b6c3fc)에서 `Option<VerifiedValueOnlyZeroFrame>` 증명 토큰을 `recover_zero_frame_fill_domain`에 요구한다. 일반 셀/미검증·효과 호출의 `Domain`은 복구 금지, 증명된 빈 frame 값 전용 경로에서만 기존 정수 0 대체. 독립 A3 reference와 일반 kernel 호출자가 모두 토큰을 전달하며 부정 회귀를 추가. `EVINHOMO` 내부 코드별 provenance/오류 계층은 여전히 미구현이고, ZF-IR-03·FW-06/07 실행 생략 허가 아님. [Linux CI 37467064363](https://github.com/yunskim/RustJ/actions/runs/37467064363) **check + j64/j64avx2 × default/portable 5/5 jobs 성공**. 각 환경 RK-06 32/32 + RK-07 retry 10/10 + error-precedence 8/8, **200/200 pinned-C 3경로 일치·불일치 0**. [Basis probe 37467064362](https://github.com/yunskim/RustJ/actions/runs/37467064362) 성공. E3의 원본 내부 오류 분류·E4 exigent provenance는 **[ ] 유지**. |
| RK-07-E4 · exigent/비필수 분리 | [ ] | 고정 [`jerr.h::EXIGENTERROR` L87–90](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/jerr.h#L87-L90) 비교; 특히 Rust `Limit`가 C `EVLIMIT`와 할당 실패를 함께 대표할 수 있으므로 근거 없는 오류 억제 금지; 오류 순서 및 overflow 테스트 |
| RK-07-E5 · 실행 가능한 범위 확장 | [ ] | 파생·동적 이름·부작용·boxed/sparse·비균질 셀 조립; 기존 값 전용 증거가 사용자 동사의 효과 생략·GPU 실행 승인으로 확대되지 않음 |

**판정:** 상기 10건의 `EVINHOMO` 지원 dense 부분집합만 통과. **RK-07 전체·RK-08~12·ZF-IR-03/04·FW-06/07은 계속 [ ]**. 나머지 미검증 결과를 일반적인 일치·최적화 허가로 취급하지 않는다.

**RK-07 — 오류 우선순위 독립 C-oracle 8건(2026-10-06).** [탐색 CI 37463314265](https://github.com/yunskim/RustJ/actions/runs/37463314265)에서 고정 원본 C j64/j64avx2 × Rust default/portable **5/5 작업 성공**, 각 환경 **8/8** 3경로 일치(총 32/32, mismatch 0). 기존 E1/E2의 `EVINHOMO` 10건과 RK-06의 32건을 변경하지 않고 `--error-probes`를 별도로 추가했다. 입력은 (1) 결과 Frame 0 아래의 inner-cell Length, (2) 빈 char/정수 혼합 셀 길이 불일치, (3) 양수 Frame 실제 셀 Length, (4) Frame prefix Length 선행, (5) 빈 Frame 및 실제 셀 Index, (6) Frame 유무별 나눗셈을 구분한다. [`a84a508`](https://github.com/yunskim/RustJ/commit/a84a5084f3a4318f1f26dc95b0428e10317064cd)·[`cce5dec`](https://github.com/yunskim/RustJ/commit/cce5decdbbec2d6577c415d5696f8d7775738b10)에서 corpus와 부정 증거 보존 테스트를 추가했다. [`ba90688`](https://github.com/yunskim/RustJ/commit/ba9068801554e5836d9b57111e8e937440c23172)·[`970f6b0`](https://github.com/yunskim/RustJ/commit/970f6b07c09ed3969cfc158005c12a2795ce5010)·[`c90701d`](https://github.com/yunskim/RustJ/commit/c90701de2b456e556fb256128e976a7556793930)에서 `--error-probes --gate-error-probes`를 고정 8건 엄격 CI 게이트로 승격했다. **최종 증거 [Linux CI 37463712382](https://github.com/yunskim/RustJ/actions/runs/37463712382): 5/5 작업 성공, 8건 × 4조합 = 32/32 pinned-C 3경로 일치, 불일치 0건.** 실행 증거는 CI의 각 reference job 및 `rank-error-*.json` artifact에 남는다. 해당 **8개 유한 사례의 엄격 게이트만 수용**하며 내부 오류 provenance·나머지 exigent/비필수 분리·효과 및 일반 Rank 의미론이 미완료이므로 **RK-07 전체 [ ]**를 유지한다.

**RK-07-E4 원본 exigent 오류 분리 원칙.** 고정 [`jsrc/jerr.h` L7–55, L87–90](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/jerr.h#L87-L90)에서 `EXIGENTERROR`는 `EVALLOC`, `EVWSFULL`, `EVVALENCE`, `EVNONCE`, `EVVALUE`, `EVTHROW`, `EVSTACK`, `EVSYSTEM` 등 반드시 전파해야 할 원본 C 코드의 **선별적 집합**이다. 반면 `EVDOMAIN`, `EVLENGTH`, `EVINDEX`, `EVRANK`, `EVLIMIT`, `EVINHOMO`는 그 목록 밖에 있지만, 이 사실만으로 모든 Rust 오류를 억제할 수는 없다. RustJ `Error::Limit`는 원본 `EVLIMIT`와 자원·할당 실패를 정확히 분리하지 못하고, `Error::Unsupported`도 원본의 `EVNONCE`와 동치가 아니기 때문이다. 그러므로 **오류 종류 + 발생 단계 + 원본 내부 코드 + 효과 가능성**을 별도 witness로 갖기 전에는 이 두 Rust 오류를 억제·재시도하지 않는다. 다음 구현 작업은 오류 provenance를 구별하는 내부 분류(E4), 오류 선행 순서·exigent 전파의 추가 C 테스트, 사용자 정의 동사의 효과/동적 이름(E5)이다. 8건의 유한 차분 일치를 모든 non-exigent 오류의 복구 증거 또는 Graph IR 실행 생략 허가로 확장하지 않는다.

**지속 체크리스트 (RK-11 및 FW-04/JX-04의 하위 항목).**

| ID | 현재 | 수정·검증 수용 기준 |
|---|---|---|
| ZF-IR-01 / M2 | [x] **Rank/frame 구조 의미론 공통화** | `rank_plan_for_shapes`와 `RankFrameExecution`; 동일 입력에 대해 J Graph와 A3의 frame·cell·빈 frame 분류가 일치해야 함. 코드 커밋 [911e113](https://github.com/yunskim/RustJ/commit/911e113c8761f3e7b25ca6b932f6c8ab56e0b398)·[fb9f884](https://github.com/yunskim/RustJ/commit/fb9f8842ac74ef289e0852927f3ef65d54574f11). [Linux CI 37451816951](https://github.com/yunskim/RustJ/actions/runs/37451816951) check + 4 reference jobs 성공 |
| ZF-IR-02 / M2 | [x] **J Graph 읽기 전용 조회 + 역방향 검증** | `Plan::rank_frame_plan`([89984f1](https://github.com/yunskim/RustJ/commit/89984f10aa6e3869e2f3d4f77730e2537030854d)), `tests/j_graph_ir.rs`([b52fd51](https://github.com/yunskim/RustJ/commit/b52fd51d2420c27d4425b920f534b8941d0f0871)): `0 3`·`2 0`·중간 0·불일치 frame, Graph와 A3 RankPlan 비교, 비-Rank 조회 None, 결과 미추론 보존. [Linux CI 37451816951](https://github.com/yunskim/RustJ/actions/runs/37451816951) 5 jobs 성공 |
| ZF-IR-03 / M3·FW-06/07 | [ ] **빈 결과 생략의 독립 증명 의무** | result-cell 타입·shape·순수성·오류 억제·효과/동적 이름·Guard·fallback을 구별; Unknown→생략 승인 및 frame 0→무조건 kernel 생략을 거부하는 negative test |
| ZF-IR-04 / M3/M4·FW-11/13 | [ ] **실행 선택·물리 최적화 검증** | pinned C/J, 독립 Rust semantic reference, Rust optimized의 3경로 회귀 및 CPU 측정 후에만 빈 커널/버퍼 생략을 개별 활성화. 2 0의 빈-cell 호출·오류 및 zero-frame fill 오류·효과를 비교. GPU 승인 없음 |

**현 단계 판정·검증(2026-10-06):** ZF-IR-01/02 **2/4 완료**. [Linux milestone run 37451816951](https://github.com/yunskim/RustJ/actions/runs/37451816951), HEAD [`e9f821d`](https://github.com/yunskim/RustJ/commit/e9f821d0658005a1545131d31c31fb53d3521b26)에서 **check(기본/portable 테스트·fmt·clippy) + pinned j64/j64avx2 × default/portable 총 5/5 jobs 성공**을 확인했다. 이 기존 차분 실행은 **변경되지 않은 실행기의 회귀**에 대한 결과이지 ZF-IR 기반 kernel-elision 최적화를 실행·검증했다는 뜻이 아니다. ZF-IR-03/04는 구현되지 않았고, 상위 RK-11·FW-04·JX-04 **[ ] 유지**. §P.11의 RK-06~12 순서는 변경하지 않는다.

<a id="jsource-optimization-migration"></a>

<a id="q-jsource-전계열-최적화-이행-계획체크리스트--검색-이외-포함-2026-10-06-살아-있는-작업표"></a>
##### Q. jsource 전계열 최적화 이행 계획·체크리스트 — 검색 이외 포함 (2026-10-06, 살아 있는 작업표)

**목적·정본.** §4.1.3.6 A–M은 jsource에서 확인한 **최적화 아이디어·원본 제약·RustJ 소유 계층의 조사 목록**이다. §O.5 **FW-01~FW-18**은 공통 증명·Guard·CPU 실행 게이트이고, §P.1은 **검색(i. family)의 세부 구현 게이트**다. 이 Q절은 그 **이외에도 존재하는 전체 source-derived family를 빠짐없이 작업 단위로 관리하려는 이행 추적표**이며, 세 표의 완료를 서로 대체하지 않는다. 별도 optimizer, C special entrypoint별 Graph node, 범용 registry를 선행 생성하지 않는다. 원본 검토 기준은 고정 jsource commit [13994ffa1ed5f06f79fad6e9822a7ed2d29b1528](https://github.com/jsoftware/jsource/tree/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc)이다.

**상태: 0 / 26 완료.** 아래의 모든 [ ]는 **코드 변경·실제 실행 검증까지 충족하는 수용 게이트**다. 기존 구현·후보 분석·작성된 테스트의 존재만으로 [x]를 부여하지 않는다. M2 일반 frontend와 **FW-01을 우선**하고, JX-01의 목록 정리처럼 정적 분류만 선행할 수 있다. 나머지는 A(기준 의미론) → B(M3 증명 계약) → C(연산군별 후보·구현) → D(M4/후속 실행 승인)의 종속성에 따른다. C단계의 **후보 표현과 proof 준비**는 선행 가능해도, 특수 실행의 **선택·활성화**는 FW-11~FW-13의 독립 CPU 기준선·자원·비용 게이트 이후다. 특히 JX-08이 입증되기 전에는 공통 family interface를 추출하지 않는다.

| ID / 게이트 | source-derived 계열과 직접 근거 | RustJ 수정·연결 위치 | 완료 수용 기준과 반드시 남길 negative/차분 테스트 |
|---|---|---|---|
| JX-01 / A | **원본·coverage 인벤토리:** ca/cf/cr/va2/ar/ap, ao/cip/gemm, vg/vgsort/vgranking, vi/viix, cc/cu/vfrom/vf/vrep/vcat, sparse/sc/a/vrand/am 등 §A/H/K/M | src/j_graph_jsource.rs의 catalog/source pin·coverage; §4.1.3.6 A–M | family마다 **원본 source form → entrypoint → guard/fallback → 의미론 → RustJ owner → 상태**를 연결하고 중복·미조사 항목을 명시. pinned revision 이후 소스 변경은 별도 diff로 취급. exhaustive라고 표시하지 않음 |
| JX-02 / A·FW-01 | **J 구문/derived entity coverage:** @:, fork/hook, rank, /., dot, grade, cut, under, M., ?/?. | src/tokenizer.rs, src/enqueuer.rs, src/parser.rs, src/semantic.rs, src/j_graph_ir.rs | 지원하는 valence/POS/operand/late-name만 source entity로 인식. 미지원은 AwaitingFrontendOrFacts; 파싱 불가 표현에서 가짜 candidate 생성 금지. C parser/oracle와 정상·부정 사례 비교 |
| JX-03 / A·FW-02 | **진짜 순차 기준 의미론:** Reduce, Scan, Search 및 추후 연산군 | src/runtime.rs, src/kernels.rs, src/index_ops.rs, semantic/runtime tests | buffer pooling과 최적화 허가를 분리. 지원 subset에서 candidate selection·prehash·특수 알고리즘 호출 0회인 Rust baseline을 계측; C는 외부 비교 oracle이지 정상 fallback이 아님 **진행(2026-10-06):** FW-02 `i./i:/e.` 독립 dense reference와 prehash 불변성 테스트 추가(`45ccebb`,`3436252`; 위 FW-02 CI 증거). Reduce/Scan 전체 및 호출 경로 계측 미완료로 수용 보류. |
| JX-04 / A·FW-03/04 | **J 의미론 fixture:** rank/cell/frame, tolerance/Fit, empty/prototype, boxed/sparse, numeric overflow·오류 우선순위 | tests/semantic.rs, tests/j_graph_jsource.rs, tests/index_ops.rs 및 기존 differential harness | 고정 C binary / 순차 Rust / 최적화 Rust의 **서로 독립된** 결과를 기록. 현재 미지원과 C 원본의 fast-path 이상 징후를 분리; Rust near를 J CCT의 oracle로 가장하지 않음 **진행(2026-10-06):** [`1f3d1cc`](https://github.com/yunskim/RustJ/commit/1f3d1cccfc5c30b385f5d886adc9cd9ac5872f22) Rust-only 4,840조합 fixture 추가. J C binary의 동일 입력·관찰 결과와 3방향 연결 전이므로 수용 보류. **2026-10-06 확장 (검증 대기):** [`bc648cc`](https://github.com/yunskim/RustJ/commit/bc648cc0ba43761e103cd42c58830147c30ebb9c)에 독립 프로세스 3방향 C/Rust baseline/Rust optimized harness 45문장 추가; [`4e8b4dd`](https://github.com/yunskim/RustJ/commit/4e8b4ddd1d4a5b9d065830fb0211137f9b894d26)에서 분류 테스트, [`09f9a3b`](https://github.com/yunskim/RustJ/commit/09f9a3ba7804fc769027b16c7c9f5abc630339ac)에서 j64·j64avx2 × default·portable CI gate 등록. 지원 dense subset만 검증하며 Rank/boxed/sparse/CCT는 미검증. **실행 증거(2026-10-06):** [Linux milestone run 37441174098](https://github.com/yunskim/RustJ/actions/runs/37441174098), 명령 `python3 tools/search_three_way.py --reference-revision 13994ffa1ed5f06f79fad6e9822a7ed2d29b1528 --report reports/search-three-way-<variant>-<backend>.json`; GitHub Actions Ubuntu Linux, J C `j64`/`j64avx2` × Rust default/portable 네 조합 **각 45 passed / 0 failed / 0 ignored, 합계 180 비교**. 실제 jsource pin `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`; 개별 fixture/해시/불일치 종류는 CI JSON artifact. **제한:** supported dense search 정수·bool·문자·정확 부동소수, 중복·빈 배열·multi-item·재검색; Rank-파생 우회/boxed/sparse/동적 CCT·Fit 미검증. **FW-04 전체 [ ] 유지.** **FW-03 tolerance 진단:** [CI 37445080131](https://github.com/yunskim/RustJ/actions/runs/37445080131), j64·j64avx2 × default·portable 각 48건에 8개 C-vs-Rust 의미 불일치, reference-vs-optimized 0개. GitHub reports/tolerance-audit 원자료에 구분 보존; semantic acceptance 아님. **[ ] 유지.** **후속 구현/검증:** [`9aab336`](https://github.com/yunskim/RustJ/commit/9aab3362ac542ed96c0bd3ba6eb2ab6c109f14f4) pinned 기본 CCT 적용. [CI 37445891526](https://github.com/yunskim/RustJ/actions/runs/37445891526) j64·j64avx2 × default·portable **각 48/48, 총 192/192 일치, Rust reference-vs-optimized 불일치 0**. 위 40/8은 변경 전 이력. 범위/미완료 의무는 §P.9를 따른다. **[ ] 유지.** **Rank 차분 진단(2026-10-06, 수용 아님):** [`5b4a7d6`](https://github.com/yunskim/RustJ/commit/5b4a7d6ddf0fdf8e966ee06f63173e71cdbce423) `tools/ranked_search_audit.py`, [`c300ff3`](https://github.com/yunskim/RustJ/commit/c300ff3295499f04e41c928a78e38cd3ed95d7fc) corpus 단위 테스트, [`d88c38b`](https://github.com/yunskim/RustJ/commit/d88c38b34530c84d7ed02b46a9cbf5a55186a50f) CI matrix 연결. [CI 37446732381](https://github.com/yunskim/RustJ/actions/runs/37446732381) Ubuntu Linux `j64`/`j64avx2` × Rust default/portable 각 **14 cases / 13 C·Rust reference·Rust optimized 일치 / 1 Rust 양 경로 Unsupported**, 합계 **52/56 일치(미해결 4개 관찰)**. 불일치 입력 `(i.0 3) (i."1 1) (i.0 3)`에서 C는 Int shape `[0,3]` 빈 결과, Rust 양 경로는 `Unsupported`(Rank empty-frame prototype inference). **이 사례를 성공/허용 불일치로 간주하지 않는다.** J Rank 결과 prototype·dtype·shape/오류 계약을 먼저 구현하고 3방향 재검증할 것. P.1/search exact optimization 증명으로 전용하지 않음; FW-04/JX-04 [ ] 유지. **P.10 zero-frame Rank 보강(2026-10-06):** [CI 37449102885](https://github.com/yunskim/RustJ/actions/runs/37449102885) pinned j64·j64avx2 × default·portable 4조합 각 20/20(총 80/80), supported dense typed fill-cell·monad/dyad·search/reference/optimized 3방향 일치. [Basis probe 37449102980](https://github.com/yunskim/RustJ/actions/runs/37449102980) 성공. Boxed/sparse, computational error fallback, 사용자 정의 effectful Rank 및 mixed-result padding 미지원으로 **[ ] 유지**. **후속 단위 작업:** [§P.11 RK-01~RK-12](#rank-cellapply-followups) 체크리스트에 실행 증거·진행 상태를 기록한다. **상위 [ ] 유지.** |
| JX-05 / B·FW-05 | **multi-region source anchor:** Cut–Scan–Raze, convolution, composition·중첩 rank | src/j_graph_ir.rs, src/j_graph_jsource.rs, src/analysis.rs, src/lowering.rs | 불변 원본 graph의 schema·source value·region·ordered A3 op 집합·rule/version을 sidecar로 검증. stale/forged/span 재사용·다른 region 혼동을 negative test로 거부 |
| JX-06 / B·FW-06/08/09 | **family별 proof obligations:** valence, rank, equality, empty, error, ownership, resource | src/j_graph_jsource.rs, src/lowering.rs 및 기존 proof/verifier 경계 | Unknown/Proven/Guarded/Disproven을 obligation별 witness와 함께 보존; 미충족 하나라도 있으면 commit/selected 금지. 기존 rewrite/fusion/search의 증거를 잘못 재사용하지 않는 테스트 |
| JX-07 / B·FW-07 | **Guard·fallback·효과 경계:** RNG, name lookup, memo, allocation, numeric retry | src/lowering.rs, src/runtime.rs, src/execution_semantics.rs | guard는 observable effect보다 먼저, miss는 명시된 정확한 reference path로. 오류·난수 소비·cache mutation 뒤 자동 재실행·중복 실행 금지. guard miss와 post-effect replay negative tests |
| JX-08 / B·FW-10 | **두 번째 독립 연산군 검증:** Reduce/Scan **또는** GroupReduce | src/j_graph_scan.rs 또는 GroupBy basis + src/j_graph_jsource.rs, src/lowering.rs | 검색과 서로 다른 numeric order/association 또는 group representative/order obligations로 JX-05~07 계약을 실제 사용. 그 전에는 공통 Candidate/SelectionPlan 추출 금지 |
| JX-09 / C | **composition·rank/MapReduce streaming:** ca.c, cf.c, cr.c, va2.c::jtfslashatg | src/j_graph_fusion.rs, src/j_graph_composition.rs, src/fusion_planning.rs | @:·capped fork의 source 정체성 및 inner/outer rank domain 유지; dense·empty·dtype·effect·in-place 비용이 맞을 때만 streaming. 기존 fusion 후보 중복 금지, generic fallback 차분 |
| JX-10 / C | **Reduce/Mean/shape fast path:** ar.c::jtreduce/jtmean, cf.c::jtfolk | src/j_graph_jsource.rs, src/j_graph_scan.rs, Reduce execution lowering | +/%#은 **단항 호출**에서만 Mean 후보. 길이 0·1·2, cell shape, promotion, FP 순서/overflow/prototype을 증명. rewrite로 1-pass fusion이나 산술 재결합을 자동 승인하지 않음 |
| JX-11 / C | **Prefix Scan / Infix Window / MovingAverage:** ap.c::jtpscan/jtmovfslash/jtmovavg | src/j_graph_scan.rs, src/j_graph_jsource.rs, semantic Window/Scan lowering | prefix와 sliding window의 결과·순서·NaN/overflow·window 길이·fallback 독립 검증. Mean 후보에서 다시 발견한 Window 후보의 source provenance 유지 |
| JX-12 / C | **Key/GroupBy + GroupReduce:** ao.c::jtkeyct/jtsldot | src/j_graph_jsource.rs, src/execution_semantics.rs의 GroupBy, src/analysis.rs | key equality/CCT, 그룹 첫 등장 순서, 대표원소, 타입·overflow, empty/sparse를 증명한 후에만 직접 집계 후보를 합법화. 단순 Search 해시 규칙 재사용 불가 |
| JX-13 / C | **Contraction·full Dot/GEMM·oblique convolution:** cip.c, gemm.c, ao.c::jtpolymult, cr.c/va2.c의 제한된 sum-times | src/execution_semantics.rs의 Contract, src/analysis.rs, src/lowering.rs | 일반 dot과 제한된 rank-1 sum-times, oblique convolution을 구분. rank/agreement, Fit, int overflow/retry, float order, sparse 및 BLAS capability 검증; 라이브러리 route는 후속 target 결정 |
| JX-14 / C | **Grade/Sort/Ranking·order statistic:** vg.c, vgsort.c, vgranking.c, vg.c::jtordstat | src/j_graph_jsource.rs, Grade/Ranking semantic basis, src/lowering.rs | tie/order, key/type/range, rank, index 오류를 증명. order-statistic의 내부 **RNG draw**가 후속 J 난수 상태에 미치는 영향 미확정이면 pure quickselect rewrite 금지; state trace 음성 테스트 |
| JX-15 / C·§P | **Search/IndexOf·Interval I. 별개:** vi.c, viavx.c, viix.c, viavx2.c | src/index_ops.rs, src/logical_ir.rs의 SearchDescriptor, src/physical.rs | 검색 세부 구현은 **§P.1과 FW-02/03/04/12**로 위임. dyadic I.의 정렬/interval witness, i./i:/e. 대표·결과 모드 및 E. window 구별; tolerance 미증명 최적화 금지 |
| JX-16 / C | **byte char-map LUT·Boolean/sparse mask→indices:** cf.c::jthkiota, v.c::jtcharmap | src/j_graph_jsource.rs, IndexSpace/Lookup/Gather semantic basis | byte alphabet, 256-entry 범위, 중복 첫 일치, invalid index의 오류와 mask/sparse-fill·rank 검증. source-form candidate에서 특수 LUT/compact 실행까지 단번에 연결 금지 |
| JX-17 / C | **Cut→Scan→Raze 및 Box+Append/Raze:** ca.c, cc.c::jtrazecut1/2, vo.c::jtjlink | src/j_graph_ir.rs, src/j_graph_fusion.rs, ConcatAssemble/Result Assembly 경계 | **zero-cut 시 source specialized 경로와 generic의 결과 축 불일치**를 별도 blocker로 남김. 원본 의미, fill/shape, boxed lifetime, effect 및 fallback 일치 전에는 변환 불허 |
| JX-18 / C | **Shape/Count 단축·상수 수치·Hook threshold:** ca.c, v.c, cf.c, va1.c, vx.c, vz.c | src/j_graph_ir.rs facts, src/analysis.rs, numeric lowering | #@, 및 bound constants는 shape/count와 계산 요구를 분리하되 error/prototype/sparse를 보존. floor-log/digits/power/deadband는 domain, Fit, FP 반올림, SIMD guard/NaN 음성 테스트 |
| JX-19 / C | **Under/Each·inverse precomputation:** cu.c::jtsunder 등 | src/semantic.rs, src/j_graph_ir.rs, src/execution_semantics.rs | forward→inner→inverse, dynamic name binding 시점, effect/alias, identity와 역함수 유효성 입증. stale inverse cache 또는 이름 재정의 시 reference fallback |
| JX-20 / C | **Virtual View/Gather/Reshape/Compress/Catenate/Result Assembly:** v.c, vfrom.c, vf.c, vrep.c, vcat.c, result.h | src/physical.rs, src/storage.rs, src/lowering.rs 및 result-assembly 경계 | Box→Open 보편 소거 금지. contiguous/noncontiguous, recursive box, view lifetime, alias/pristinity/usecount, rank/shape/error 검증; 원본을 변경해 관찰 가능한 값이 바뀌는 alias negative test |
| JX-21 / C | **sparse-specific execution:** cpdtsp.c, vgsp.c, visp.c, vfromsp.c | src/sparse.rs, tests/sparse_runtime.rs, src/execution_semantics.rs | sparse axes/fill/empty/prototype, boxed/numeric 타입과 density/resource, dense와 다른 오류·순서 검증. sparse를 불투명 dense 배열이라고 가장한 fuse/convert 금지 |
| JX-22 / C | **Amend/Scatter·buffer donation·lifetime:** am.c, m.c, p.c, cx.c | src/runtime.rs, src/storage.rs, src/physical.rs, effect/assignment 경계 | index/type/readonly, shared alias, recursive boxes, assignment commit/error 순서 및 failed-retry를 증명. SSA liveness만으로 inplace 허가하지 않음; source mutation negative tests |
| JX-23 / C | **동적 name/locale lookup cache:** sc.c, cx.c | src/semantic.rs, src/runtime.rs, name-version/binding runtime | late binding, locale epoch, invalidation, redefinition/reentrancy, thread safety를 검증하고 cache hit/miss 출력 동일성 비교. 이름 참조 cache를 Graph 상수 전파로 취급하지 않음 |
| JX-24 / C | **명시적 M. memo·RNG generate/shape composition:** a.c::jtmemo, vrand.c::jtrollk, vg.c stateful route | src/semantic.rs, src/runtime.rs, src/execution_semantics.rs의 effect/state contract | M.의 사용자 지정 memo 동작과 임의 CSE를 구분. cache key/효과/수명, RNG seed·draw 순서·state trace를 관찰 가능한 계약으로 증명. 미지원이면 RuntimeSemantic/Unknown 유지 |
| JX-25 / D·FW-11~17 | **Physical target·자원·실측 cost 승인:** jsource SIMD/AVX, GEMM, hash, view, inplace, memory | src/lowering.rs, src/physical.rs, src/j_graph_resource.rs, src/fusion_planning.rs | baseline CPU와 독립 3방향 검증 후 target capability, hard allocation bytes, measured latency/memory를 분리. 동일 의미·guard/fallback·실측 win 없으면 SIMD/BLAS/GPU route 및 캐시 임계값 변경 보류 |
| JX-26 / 최종·FW-18 | **반복 독립 검토·미조사 잔여:** primitive numeric/allocator/architecture-specific paths, 새로운 jsource commits | §4.1.3.6 A–M, 이 §Q, §O.5, §P.1, reports/ | **원본 C 진입·fallback / J 의미·반례 / Graph-A3 provenance / guard-effect·runtime / target-cost·bench**를 서로 독립 재검토. commit·OS/toolchain·실행 명령·pass/fail/ignored·oracle pin/범위·known gaps를 남기고 FW/P와 체크 동기화 |

<a id="q1-jx-01fw-01-정적-sourcefrontend-감사-2026-10-06"></a>
###### Q.1 JX-01/FW-01 정적 source/frontend 감사 (2026-10-06)

**상태:** JX-01 [ ], FW-01 [ ] 유지. 고정 jsource revision에서 현재 registry가 지목한 **C/H 원본 16/16 파일 존재**를 직접 확인했다. 대표 심볼·텍스트는 15개 파일에서 발견했다. `vcat.c`의 `boxed ownership transfer`는 실제 C 심볼이 아닌 설명이므로 일치로 세지 않았다. **파일/심볼 확인은 guard·fallback 전체, 실제 J 실행 또는 최적화 의미론의 증명이 아니다.**

`src/j_graph_jsource.rs::JSOURCE_FAMILY_RULES`의 16개는 **AnalysisOnly 7 / ExistingAnalyzer 1 / AwaitingFrontendOrFacts 4 / DownstreamOnly 4**로 나뉜다.

| family / 현재 발견 상태 | source → pinned C/H | 검증할 조건·reference 복귀 / RustJ 책임·연계 게이트 |
|---|---|---|
| `ReductionFastPath` / AnalysisOnly | `f/ y` → `ar.c::jtreduce` | empty/singleton/two-item, identity, overflow → Reduce; ExecutionAlgorithm / JX-10 |
| `MeanIdiom` / AnalysisOnly | 단항 `(+/ % #) y` → `cf.c::jtfolk`, `ar.c::jtmean` | dyad 제외, rank·FP order → 원 fork; ExecutionSemantics / JX-10 |
| `WindowAlgorithm` / AnalysisOnly | `f\ y`, `x f\. y` → `ap.c::jtmovfslash` | Scan≠Window, length·NaN/overflow → generic; ExecutionAlgorithm / JX-11 |
| `SearchAlgorithm` / AnalysisOnly | `i.` / `i:` / `e.` dyad → `vi.c::indexofsub` | first/last/member, CCT·rank·boxed/sparse → sequential; ExecutionAlgorithm / §P·JX-15 |
| `IntervalLookup` / AnalysisOnly | 이항 `x I. y` → `viix.c` | sortedness/type/empty, 단항과 구별 → baseline; ExecutionAlgorithm / JX-15 |
| `GatherCopyOrView` / AnalysisOnly | `x { y` → `vfrom.c::jtget1cell` | bounds/alias/contiguity → copy; PhysicalPlanner / JX-20 |
| `ReindexCopyOrView` / AnalysisOnly | `$` / `|.` / `|:` → `vf.c` | fill/shape/usecount → materialize; PhysicalPlanner / JX-20 |
| `MapReduceStreaming` / ExistingAnalyzer | `f/@:g` → `va2.c::jtfslashatg` | dense/type/empty/inplace/overflow → generic map/reduce; 기존 GraphFusion / JX-09 |
| `ResultAssemblyDemand` / AwaitingFrontendOrFacts | box/open/raze → `result.h` | recursive boxes·raze·effect → generic assembly; ExecutionSemantics / JX-17/20 |
| `GroupAggregate` / AwaitingFrontendOrFacts | `u/.`, `f//.` → `ao.c::jtkeyct/jtsldot` | CCT·group order·representative/type → generic group; ExecutionAlgorithm / JX-12 |
| `MatrixContraction` / AwaitingFrontendOrFacts | `+/ . *` → `cip.c::jtpdt`, `gemm.c` | rank/Fit/overflow/FP order/sparse → generic dot; ExecutionAlgorithm / JX-13 |
| `GradeRanking` / AwaitingFrontendOrFacts | `/:`, `\:` → `vg.c` | tie/order/type/axis → generic grade; ExecutionAlgorithm / JX-14 |
| `TolerantHash` / DownstreamOnly | tolerant search → `viavx2.c` | CCT 비추이성/±0/NaN → sequential; ExecutionAlgorithm / §P·JX-15 |
| `SparseAlgorithm` / DownstreamOnly | sparse dot/grade/search/from → `cpdtsp.c` 등 | axes/fill/empty/type → sparse reference; ExecutionAlgorithm / JX-21 |
| `BufferOwnership` / DownstreamOnly | boxed concat/reshape/compress → `vcat.c` 등 | alias/usecount/recursive box → allocating result; PhysicalPlanner / JX-20/22 |
| `NameLookupCache` / DownstreamOnly | dynamic name/locale → `sc.c::jtunquote` | epoch·locale·reentrancy/invalidation → actual lookup; RuntimeBinding / JX-23 |

**registry 외의 source backlog도 유지:** Cut→Scan→Raze(`cc.c`, JX-17), oblique convolution(`ao.c`, JX-13), char LUT(`v.c`, JX-16), boolean/sparse→indices(`cf.c`, JX-16), RNG order statistic(`vg.c`, JX-14), RNG shape(`vrand.c`, JX-24), Box+Append(`vo.c`, JX-17), 명시적 `M.` memo(`a.c`, JX-24), Under/Each(`cu.c`, JX-19), bound numeric/deadband(`vx.c/vz.c/va1.c`, JX-18), Amend/Scatter(`am.c`, JX-22), assignment/definition fast path(`p.c/cx.c`, JX-22). 이 목록은 §4.1.3.6 H/K/M의 조사 결과로, **새 실행 후보·전수 조사 완료를 의미하지 않는다.**

**FW-01 프런트엔드 구분:**
- **Word formation:** `src/tokenizer.rs::scan/parse_word_spans`와 `tests/syntax.rs`의 이전 F0 실행 이력은 이번 검사 결과로 재계산하지 않는다.
- **Enqueue/POS:** `src/primitive.rs`, `src/enqueuer.rs`, `tests/enqueuer.rs`. `/.`, `.`, `/:`, `\:`, `;.`, `&.`, `M.`, `?`, `?.`, `!.`의 기본 품사 분류는 실제 derived constructor·executor·최적화 허가가 아니다. locative/name-by-value·일부 numeric payload는 Unsupported다.
- **Parser/derived:** `src/parser.rs`, `src/semantic.rs::FunctionEntity`, `tests/semantic.rs`에서 `@:`, Hook/Fork, Rank, Insert/PrefixInfix 일부를 표현한다. Key/Dot/Cut/Under/Memo/Grade 및 동적 NAME·valence·POS·오류/효과 순서의 실행/차분 검증은 남았다.
- **Source opportunity:** `src/j_graph_jsource.rs::discover`는 단항 Mean만 인정하며 `E.` window와 search를 구분한다. 후보는 모두 아직 legal/selected가 아니다.
- **추가 회귀 소스:** [커밋 e364535](https://github.com/yunskim/RustJ/commit/e36453575430879e4bc546c62350107a3e698e84)에서 `tests/j_graph_jsource.rs::optimization_vocabulary_pos_is_not_a_compiler_optimization_license`를 추가했다. **Cargo/default·portable/실제 J C oracle은 아직 미확인**.

**실제 CI에서 확인한 미해결 결과(2026-10-06):** 신규 POS 테스트 커밋 [e364535](https://github.com/yunskim/RustJ/commit/e36453575430879e4bc546c62350107a3e698e84)의 [Basis compile probe](https://github.com/yunskim/RustJ/actions/runs/37428376593)는 `cargo test` 중 **기존 `tests/index_ops.rs::member_preserves_cell_shapes_and_empty_query_semantics` 실패**(실제 bool shape `[0]` vs 잘못된 기대 `[2]`)로 종료되어 새 Jsource 테스트 완료 근거가 아니다. J 공식 Dictionary의 `x e. y ↔ (#y)>y i. x` 계약에 따라 **빈 왼쪽 질의는 빈 결과**, **빈 오른쪽 lookup은 길이 2의 거짓 결과**로 분리한 [80ad4a7](https://github.com/yunskim/RustJ/commit/80ad4a73b14099431966d2f697fa73f98778e159)를 반영했다. 이는 테스트의 방향 오류 수정이며 실제 전체 J C binary 비교가 아니다. 같은 e364535의 [Linux milestone](https://github.com/yunskim/RustJ/actions/runs/37428376492)은 **기존 다수 Rust 소스의 `cargo fmt --check` 차이** 때문에 check job 실패, C reference 네 조합의 개별 job은 성공했다. **80ad4a7 후속 default/portable 결과와 신규 POS 테스트 통과는 이 기록 시점에 미확정**이므로 JX-01/FW-01/FW-04·전체 CI [x] 금지.

**다음 한 단계:** 새 테스트의 실제 default·portable 결과 및 pinned J C의 POS/derived syntax/error 차분을 확보한 뒤 **확인된 의미론 불일치 한 종류 + 회귀 하나**씩 수정한다. FW-01·JX-01 체크는 결과 명령·환경·commit·unsupported 범위가 기록되기 전까지 열어 둔다.

<a id="q2-fw-01--jx-01-실행-회귀-및-원본-비교-게이트--두-번째-이행-기록-2026-10-06"></a>
###### Q.2 FW-01 / JX-01 실행 회귀 및 원본 비교 게이트 — 두 번째 이행 기록 (2026-10-06)

**구현 판정: [ ] FW-01 · [ ] JX-01 · [ ] JX-10 유지.** 단항 Mean 후보를 실제로 놓치는 오류를 GitHub Actions [02854b6 진단 로그](https://github.com/yunskim/RustJ/actions/runs/37429620894)에서 재현했다. `(+/ % #) y`는 **전체 Fork 호출은 단항이지만, Graph region의 마지막 `g=%` Apply는 분기 결과 두 개를 받는 이항**이다. 이전 `src/j_graph_jsource.rs::discover`는 join의 `Valence::Monad`를 요구해 Mean 후보를 항상 버렸다. [05dae2d](https://github.com/yunskim/RustJ/commit/05dae2d9c250a466c1f0b89cfdc4b776e72d74c5)는 원래 `RegionKind::Fork` 및 `region.inputs.len()==1`로 **외부 호출의 valence**를 판정하고, join의 `Valence::Dyad`를 확인한다. Graph 실행·순서·원본 function entity는 변경하지 않는다. 고정 `jsrc/cf.c::jtfolk`는 `+/ % #` 패턴에 대해 `f1=jtmean`만 설정하므로 **단항 후보만** 발견하는 제한은 적절하다.

[268c83f](https://github.com/yunskim/RustJ/commit/268c83f656812b2a9fc951cb91c84e5e6f2a368b)는 임시 `MEAN_DIAG` 출력을 제거하고 **outer Fork=단항/inner join=이항** 및 이항 호출의 후보 부재를 테스트한다. 기존 `e.` empty-query 회귀도 [d818a3e](https://github.com/yunskim/RustJ/commit/d818a3e807453327d7cc4cd265dc503acf36d0ec)에서 **Boolean membership 결과**와 **Int `i.` missing sentinel**을 구분해 수정했다. `e.`의 빈 왼쪽 프레임 `[0]` 및 오른쪽 비었을 때 길이 2의 false 결과를 각각 확인한다.

**실제 CI 증거:** Ubuntu GitHub Actions, Rust stable, [268c83f의 Basis compile probe](https://github.com/yunskim/RustJ/actions/runs/37432213914)에서 `cargo test`, `cargo test --features portable`, `cargo build --release`는 **성공**했다. Clippy는 기존 `src/index_ops.rs:330`의 inclusive-range 표현 경고로 실패했다. [dd2c118](https://github.com/yunskim/RustJ/commit/dd2c118de8bb025a2f4fa3f21be9f22063c6be52)로 범위 검사를 동일한 의미의 `(64..=MAX_PREHASH_ITEMS).contains(&items)`로 바꿨고, [37432369878](https://github.com/yunskim/RustJ/actions/runs/37432369878)에서 **default + portable tests / release build / Clippy 모두 통과**했다. 이는 *Rust 회귀 승인*이지 J 언어 전체 의미론 인증이 아니다.

**남은 독립 게이트:** Linux milestone [37432213939](https://github.com/yunskim/RustJ/actions/runs/37432213939)의 `check`는 `cargo fmt --check`에서 실패했다. 포맷 차이는 13개 Rust 소스·테스트 파일에 걸쳐 있으므로 **CI 전체 성공으로 표시하지 않는다**. j64/j64avx2 C reference job 결과와 검사 스위트 성공 여부는 별도 기록한다. [a91dd57](https://github.com/yunskim/RustJ/commit/a91dd5741f298e40782cea9a73801ff6dad0863e)는 `tools/conformance.py::cases`에 **단항 Mean의 보통 배열/길이 1/빈 배열/다차원 배열**을 추가해 pinned J C와 Rust 결과를 차분할 준비를 했다. 이 신규 C 차분 결과는 **아직 승인되지 않았다**. Native J 그래프 의미론 검증·JX-10 특수 실행 선택은 미완료다. 후속 실행에서 실패가 나면 `case + oracle output + Rust output + pin + backend`를 기록해 의미론 차이와 미지원/상위 원본 특수경로 차이를 분리한다.

<a id="q3-fw-01jx-01-github-ci-녹색-검증과-기준선-기록-2026-10-06"></a>
###### Q.3 FW-01/JX-01 GitHub CI 녹색 검증과 기준선 기록 (2026-10-06)

**코드/검증 커밋:** [89bbfd0](https://github.com/yunskim/RustJ/commit/89bbfd0e55163c90b5059e90d10b0ba0bd87ded5). GitHub Actions [Linux milestone 37433098574](https://github.com/yunskim/RustJ/actions/runs/37433098574)의 **5개 job이 모두 성공**했다. `check`는 Python 도구 테스트, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo test --features portable`, `cargo build --release`, milestone 예제 실행을 모두 통과했다. [Basis compile probe 37433098567](https://github.com/yunskim/RustJ/actions/runs/37433098567)도 완료·성공했다. 이전 포맷 오류는 GitHub CI rustfmt 제안 104 hunk / 13 파일을 **의미론 변경 없이** 원자적으로 적용하여 제거했다.

**고정 J C 차분 실행:** 같은 Linux milestone의 참조 job 네 개 모두 성공했다. CI는 실제 `jsource` commit `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`을 fetch·빌드했으며, [ed56b33](https://github.com/yunskim/RustJ/commit/ed56b336dd198b661cb1e4e80814488b4d890a57)부터 `tools/conformance.py`와 `tools/word_conformance.py` 보고서의 `reference_revision`에도 이 SHA를 기록한다.

| 고정 C variant × Rust backend | 실행 케이스 | 일치(pass) | 예외로 이미 기록된 known deviation | 예상 밖 failure |
|---|---:|---:|---:|---:|
| j64 × default | 5,384 | 5,383 | 1 | 0 |
| j64 × portable | 5,384 | 5,383 | 1 | 0 |
| j64avx2 × default | 5,384 | 5,384 | 0 | 0 |
| j64avx2 × portable | 5,384 | 5,384 | 0 | 0 |

이 known deviation은 **`(i.2 3) -"1 0 (i.2 3 4)`에서 동일한 값·형상이나 J64가 Float, Rust가 Int를 내는 기존의 한정된 타입 차이**로, `tools/conformance.py::known`에 사례·type·shape guard가 박혀 있다. 신규 단항 `(+/ % #)`의 보통 길이·길이 1·빈 배열·rank/frame 네 차분 사례는 이 예외에 해당하지 않고 정상 통과했다. `failed=0`는 이 **supported subset**과 known deviation 분리 기준으로만 유효하다. J upstream 전체 테스트 스위트나 모든 numeric/locale/effect 의미론을 증명한 것은 아니다.

**체크리스트 상태는 [ ] FW-01 / [ ] JX-01 / [ ] JX-10을 그대로 유지한다.** 이번에 끝난 것은 해당 부분집합에 대한 Mean source-candidate 회귀 및 독립 C 차분 **하위 검증**이다. FW-01은 Key/Dot/Cut/Grade/Under/Memo 등 미구현 frontend constructor, rank/effect/error 및 다른 단일 단계의 C 차분 검토가 남아 있다. JX-01은 고정 원본 source family의 **각 guard·fallback 전수 범위**를 확정해야 하며, JX-10은 proof/Guard/순차-특수 3방향 검증과 성능 실측 전에는 최적화 실행을 승인할 수 없다.

**다음 작업:** FW-01/JX-02의 지원되지 않는 **Key `/.` 파생 동사 구성**부터 고정 J C parser/POS·valence와 RustJ core support tag 구분을 입력별로 조사한다. 비지원 구문은 `AwaitingFrontendOrFacts`로 유지하고, 반례와 정상/부정 테스트를 확보한 뒤 한 연산군씩 구현한다.

<a id="q4-fw-01jx-02의-첫-key--파생-동사-구성--실행과-최적화-허가는-분리-2026-10-06"></a>
###### Q.4 FW-01/JX-02의 첫 Key `/.` 파생 동사 구성 — 실행과 최적화 허가는 분리 (2026-10-06)

**현황:** `FW-01 [ ]`, `JX-02 [ ]`, `JX-12 [ ]`, `JX-01 [ ]` 유지. `/.`를 **word/POS 인식에서 동사 피연산자 파생 동사의 비실행 구성까지** 이행한 부분 단계다. Key·Oblique의 범용 연산, 명사 gerund의 구성, GroupBy 최적화는 아직 구현·승인되지 않았다.

**고정 원본 계약:** `jsrc/ao.c::jtsldot`는 `u/.`를 하나의 derived verb로 생성하면서 **단항 `jtoblique` / 이항 `jtkey`**를 별도 등록하고 세 innate rank 모두 `RMAX`로 설정한다. 동사 피연산자는 직접 보존하고, **명사 gerund 피연산자는 `fxeachv`로 별도 해석**하므로 '모든 명사 피연산자'가 유효하다는 뜻은 아니다. `jtkeyct`는 분류에 `CCT`와 `indexofsub(IFORKEY)`를 쓰고 group별 실행으로 넘어가며, sparse·boxed·특수 reduction 경로는 guard/fallback이 다르다. 따라서 Key 실행을 일반 해시 GroupBy로 단순 치환하면 안 된다. 이 기록은 고정 SHA `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`의 `ao.c` 및 `cf.c` 확인에 한정한다.

**이번 이행의 세부 체크리스트(전체 FW/JX 게이트와 분리):**
- [x] `src/primitive.rs`의 `AdverbId::Sldot` 추가, `/.`를 중복 `VocabularyPrimitive`에서 제거, `REGISTRY_VERSION = 9`. `EnqueueClass::Adverb` 유지. 단독 modifier는 여전히 실행하지 않는다.
- [x] `src/parser.rs::apply_adverb`의 기존 동사-left 구조 경로에서 `FunctionHead::PrimitiveAdverb(Key)`와 원래 함수 operand를 보존하는 파생 동사 구성. `src/semantic.rs::innate_ranks`에 `[63;3]` 반영.
- [x] `src/j_graph_ir.rs`에서 Key 파생 함수를 **불투명 `GraphForm::Modifier`**로 표현. Reduce/Window/GroupAggregate 후보를 합성하지 않으며 `JsourceFamily::GroupAggregate`는 `AwaitingFrontendOrFacts` 상태 유지.
- [x] 정상·부정 Rust 회귀: `tests/semantic.rs::key_derived_verb_keeps_operator_and_operand_without_licensing_execution`, `tests/j_graph_jsource.rs::key_construction_preserves_an_opaque_graph_boundary_without_groupby_selection`, vocabulary/POS 테스트 및 기존 `tests/primitive.rs` 확장. 단항·이항 syntax는 parse되지만 실행은 명시적 `unsupported`; 명사 `3/.`도 지원된다고 가장하지 않음.
- [x] 고정 J C에 대한 **상태 보존형 구성·binding/alias 6개 fixture**를 `tools/conformance.py`에 추가. [CI 37435150581](https://github.com/yunskim/RustJ/actions/runs/37435150581) **전체 5 job 성공**, [Basis probe 37435150506](https://github.com/yunskim/RustJ/actions/runs/37435150506) 성공. `cargo fmt --check`, Clippy, 기본/portable test, release build 및 Python 도구 테스트가 녹색.
- [ ] 명사 gerund `m/.` 생성에 필요한 `fxeachv`/AR decoding, 유효·무효 gerund 및 오류 우선순위의 pinned J C 비교.
- [ ] 단항 Oblique와 이항 Key의 **독립 순차 reference**, group order/representative, tolerance(CCT)·rank·empty·boxed·sparse·effect/error/overflow 차분을 구현.
- [ ] CPU 특수 recipe/prepare-lifetime·ProofBundle·guard/fallback/무효화·성능 근거. 검증 전에는 GroupBy 실행 경로 선택·최적화 후보 승격 **금지**.

**네 갈래 실제 차분 증거:** `jsource` pinned revision SHA `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`; source-backed OCI/Linux GitHub Actions, seed `20260926`, randomized rounds `100`. `tools/conformance.py`의 보고서에는 동일 pin과 binary/reference SHA256이 포함된다.

| C variant / Rust 구성 | 사례 | 동등 확인 | 기존 좁은 known deviation | 새 실패 |
|---|---:|---:|---:|---:|
| `j64` / default | 5,390 | 5,389 | 1 | 0 |
| `j64` / portable | 5,390 | 5,389 | 1 | 0 |
| `j64avx2` / default | 5,390 | 5,390 | 0 | 0 |
| `j64avx2` / portable | 5,390 | 5,390 | 0 | 0 |

`j64`의 known deviation 1건은 앞의 Q.3에서 기록한 별도 rank 연산의 **Int/Float 타입 차이**이며 Key 테스트의 실패가 아니다. 새 6개 fixture는 **파생 동사 생성·이름 바인딩만** 비교하고 `x u/. y` 또는 `u/. y` 실행을 비교하지 않는다. J upstream 전체 테스트 스위트도 실행하지 않았다.

**구현 단위 커밋:** [78ba45a](https://github.com/yunskim/RustJ/commit/78ba45a9bcbc47e15ef57299a77c7131be523cde) 기능·회귀·C fixture, [de5c4dc](https://github.com/yunskim/RustJ/commit/de5c4dc10b11570836794fda42d3adf84a0c10bf) fmt 정리, [d8db5f9](https://github.com/yunskim/RustJ/commit/d8db5f95991f6116e672d41eaee6876d5faa3f19) Graph 후보 부정 회귀, [a4f4dd0](https://github.com/yunskim/RustJ/commit/a4f4dd078c26d4fe157357dbb65f37b56375dff2) fmt 마무리.

**다음 우선순위:** FW-01/JX-02의 범위 밖까지 Key 구현을 성급히 확장하지 않는다. 프런트엔드 미지원 목록을 유지하고 **FW-02의 독립 sequential `i.` 검색 기준선**부터 착수하여 optimized search와 의미론 검증 경로를 분리한다. 이후 FW-10/JX-08의 두 번째 독립 연산 계열로 Key/Reduce를 검증해 공통 인터페이스 필요성을 판단한다.

**체크리스트 사용 규칙.** 각 JX 행은 **(1) C 원본 pin·조건 확인 → (2) J 의미론/unsupported 범위 확정 → (3) Graph 후보 및 source witness → (4) obligation별 proof/Guard·fallback → (5) 독립 reference·negative·C differential → (6) target/resource/실측 선택**의 여섯 열을 통과해야 완료한다. 실제 결과가 없으면 해당 행은 [ ]로 유지하며, 한 번에 **하나의 의미론 변경 + 해당 회귀/반례 하나**를 우선한다. 실패 또는 upstream drift가 발견되면 해당 연산군의 증명을 무효화하고 FW 관련 선행 게이트까지 되돌아간다. 각 완료 행에는 **JX-ID / code commit / 실행 명령·환경 / passed·failed·ignored / jsource commit·실제 oracle 범위 / fallback·negative 결과 / 측정값 / known gaps / 다음 게이트**를 기록한다. 당장은 **JX-01의 출처·범위 추적과 FW-01(M2)**부터 이어가며 특수 최적화를 새로 활성화하지 않는다.


<a id="ndarray"></a>
### ndarray

참고:

- logical order와 memory order 구분
- checked view construction
- shape/stride 기반 view

RustJ 적용:

- logical-order contiguous와 memory-contiguous 구분
- checked affine descriptor
- reshape compatibility proof

<a id="apache-arrow"></a>
### Apache Arrow

참고:

- buffer ownership
- offset/shared backing
- allocation layout

RustJ 적용:

- BufferLease
- shared backing 수명
- 부분 view와 allocation owner 구분

<a id="openxla"></a>
### OpenXLA

참고:

- logical IR과 buffer assignment 분리
- layout conflict의 copy
- backend-specific lowering

RustJ 적용:

- ValueId/BufferId 분리
- Logical/Physical Plan 분리
- layout conflict materialization

<a id="arrayfire--j-arrayfire-add-on--fusion-systems"></a>
### ArrayFire / J ArrayFire add-on / fusion systems

확인 기준: 2026-10-01.

주요 참고 자료:

- ArrayFire JIT: https://arrayfire.org/docs/jit.htm
- ArrayFire Unified Backend: https://arrayfire.org/docs/unifiedbackend.htm
- CUDA interoperability: https://arrayfire.org/docs/interop_cuda.htm
- Memory manager API: https://arrayfire.org/docs/group__memory__manager.htm
- Jsoftware ArrayFire add-on, pinned at `b0543c8278fe7a50e0ac9f938a936b4a84ee239b`:
  https://github.com/jsoftware/math_arrayfire/tree/b0543c8278fe7a50e0ac9f938a936b4a84ee239b
- J add-on manual:
  https://github.com/jsoftware/math_arrayfire/blob/b0543c8278fe7a50e0ac9f938a936b4a84ee239b/man.txt
- Alex Shroyer의 J GPU/ArrayFire prototype 자료:
  https://alexshroyer.com/papers/matmul_j_gpu.pdf

ArrayFire 자체에서 참고할 핵심:

- elementwise 연산을 즉시 실행하지 않고 AST/lazy expression graph로 누적한 뒤 필요할 때 한 kernel로 JIT fusion한다.
- explicit `eval` 또는 JIT가 지원하지 않는 consumer가 evaluation boundary가 되며, `sync`는 평가 시작과 완료 대기를 구분한다.
- CUDA/OpenCL/oneAPI/CPU backend를 공통 array API 뒤에 두며 backend 선택은 array 계산 의미와 분리한다.
- device pointer, stream, lock/unlock, custom memory manager를 명시해 외부 kernel/library와의 ownership·lifetime·synchronization 경계를 관리한다.
- JIT compilation cache가 있으므로 cold compile cost와 warm execution cost를 분리해 측정해야 한다.

Jsoftware의 `math_arrayfire` add-on에서 특히 참고할 부분:

- 이것은 J 전체를 GPU compiler로 바꾸는 구현이 아니라 J에서 ArrayFire C API로 들어가는 **library adapter/offload 사례**다.
- J array는 row-major, ArrayFire array는 column-major이므로 add-on은 `rcc` 변환을 사용한다. 이는 logical atom order와 physical layout을 동일시하면 adapter 경계에서 불필요한 전역 변환 비용이 생길 수 있다는 실제 사례다.
- `families.ijs`는 `af_add`, `af_mul`, `af_sum` 같은 concrete ArrayFire function family를 직접 매핑한다. J의 일반 `/`, `\\`, rank, derived verb 의미 전체가 자동으로 ArrayFire op로 번역되는 구조는 아니다.
- add-on은 `af_array` handle을 별도 추적하고 release/hold/device GC를 관리한다. RustJ의 ValueId와 외부 backend buffer/handle을 분리해야 한다는 근거로 사용할 수 있다.
- add-on의 shape/rank validation은 ArrayFire `dim4` 경계에 맞춰 사실상 rank 4 이하를 전제로 한다. 이는 backend capability/precondition이지 J 언어의 rank 제한이 되어서는 안 된다.
- CPU/CUDA/OpenCL backend를 바꿔 쓸 수 있지만, backend 변경 자체가 J noun의 semantic identity를 바꾸지는 않는다.

RustJ 적용:

1. **ArrayFire는 Graph/Execution optimizer의 선행 구현 사례로 참고한다.**
   - lazy graph, evaluation boundary, fusion trigger를 참고하되 J Semantic IR 자체를 lazy ArrayFire AST처럼 축소하지 않는다.
   - fusion 여부와 materialization은 semantic legality가 확정된 뒤 Logical/Physical planning에서 결정한다.

2. **J ArrayFire add-on은 external-library route의 adapter 사례로 참고한다.**
   - `J logical value → adapter capability check → external array handle → execution → logical result` 경계를 설계할 때 직접 비교한다.
   - op coverage, dtype/rank/shape/layout 조건은 route precondition으로 명시한다.

3. **row-major/column-major mismatch를 Physical Planner 검증 사례로 사용한다.**
   - RustJ logical array는 layout-neutral하게 유지한다.
   - ArrayFire route가 column-major representation을 요구하면 view/consumer absorption/copy 중 어느 것이 합법적이고 싼지 physical plan에서 선택한다.
   - adapter 편의를 위해 J logical atom order를 바꾸지 않는다.

4. **evaluation/synchronization을 Physical Plan의 별도 개념으로 둔다.**
   - lazy value의 존재, kernel submission, device completion은 서로 다른 상태다.
   - 향후 AsyncToken/Timepoint, transfer, external library call의 legality와 lifetime 검증에 ArrayFire의 `eval/sync` 및 interop 경계를 비교한다.

5. **cost model과 benchmark 방법론에 cold/warm JIT를 분리한다.**
   - compile latency, kernel-cache hit, host/device transfer, layout conversion, intermediate materialization을 별도 비용 항목으로 본다.
   - 단순 warm-kernel 수치만으로 route profitability를 판단하지 않는다.

6. **Graph Basis ↔ external capability matrix를 만들 때 실물 비교 대상으로 사용한다.**
   - Elementwise, Reduce, Scan, Gather/Index, MatMul, Conv, Sparse 등 RustJ basis family가 ArrayFire API에서 직접 지원되는지,
   - J 의미를 그대로 보존하는지,
   - adapter shim 또는 fallback이 필요한지를 구분한다.

중요한 비채택 사항:

- ArrayFire `af::array`를 RustJ Logical Array/J noun과 동일시하지 않는다.
- ArrayFire의 rank/dim4 제한, column-major layout, dtype 범위를 J semantics에 역류시키지 않는다.
- ArrayFire의 fixed reduction API를 J의 일반 adverb `/` 또는 `\\` 의미론과 동일시하지 않는다.
- ArrayFire JIT가 fuse할 수 있다는 사실만으로 RustJ Graph rewrite/fusion의 semantic legality가 증명되었다고 보지 않는다.
- J add-on의 FFI 함수 매핑을 RustJ compiler architecture 자체로 채택하지 않는다.

따라서 ArrayFire는 RustJ의 GPU backend 후보 하나라기보다, **lazy array execution, kernel fusion, external-library routing, physical layout mismatch, device-handle lifetime과 synchronization을 동시에 검증할 수 있는 비교 기준**으로 다룬다.

<a id="jax--multi-device-systems"></a>
### JAX / multi-device systems

참고:

- logical/global value와 placement/sharding 분리

RustJ 적용:

- logical J noun과 physical placement 분리

<a id="mlir-core--interfaces--regions"></a>
### MLIR core / interfaces / regions

참고:

- SSA value + Region/Block 구조로 pure graph와 control flow를 함께 표현
- verifier를 operation contract의 일부로 둠
- concrete op를 special-case하지 않고 operation/type/attribute interface를 통해 분석 capability를 질의
- side effect와 speculation safety를 별도 interface로 모델링
- data-flow analysis를 typed lattice와 monotonic join으로 구성

RustJ 적용:

- Logical IR에 SSA ValueId + Function/Region/Block/Terminator를 둔다.
- Primitive/LogicalOp capability를 Shape/Axis/Access/Effect/Alias/Speculation interface로 분리한다.
- 생성/변환 뒤 verifier를 필수 경계로 둔다.
- fact domain마다 typed lattice를 사용한다.

<a id="mlir-shape--dynamic-constraints"></a>
### MLIR Shape / dynamic constraints

참고:

- unknown shape와 invalid shape를 구분
- compile-time constraint가 증명되지 않으면 witness/assuming 구조로 의존성을 명시
- shape computation과 value computation을 분리하되 필요하면 runtime shape value로 reify

RustJ 적용:

- `ConstraintSet`만 metadata로 저장하지 않고 Witness/Guard를 둔다.
- dynamic specialization은 fast/fallback region으로 표현한다.
- semantic error가 증명된 Invalid와 단순 Unknown을 구분한다.

<a id="mlir-linalg--bufferization"></a>
### MLIR Linalg / Bufferization

참고:

- indexing map + iterator type으로 structured computation을 표현
- tensor-level tiling/fusion/vectorization을 먼저 하고 bufferization을 늦춤
- Destination-Passing Style과 alias relation을 buffer reuse 분석의 입력으로 사용
- bufferization은 SSA use-def와 conflict 분석을 통해 실제 in-place/out-of-place를 결정

RustJ 적용:

- `IterationDomain + AccessRelation`을 Logical IR의 핵심 contract로 둔다.
- `StorageRequirement`와 실제 `MaterializationDecision`을 분리한다.
- `DestinationRelation`은 BufferId가 아니라 후속 bufferization hint/contract다.

<a id="mlir-transform-dialect--tvm-tensorir-schedule"></a>
### MLIR Transform dialect / TVM TensorIR schedule

참고:

- payload IR과 transformation/schedule description을 분리
- 같은 semantic computation에 여러 schedule을 적용 가능
- TVM은 graph-level Relax와 lower-level TensorIR/schedule을 구분하고 external codegen도 허용

RustJ 적용:

- Logical IR에 tile/vector/workgroup 결정을 박지 않는다.
- native route의 `Schedule / Transform Plan`을 별도 표현으로 둔다.
- MLIR/TVM류 external optimizer를 재구현하지 않고 adapter를 통해 활용할 수 있게 한다.
- external backend는 whole-program 선택이 아니라 legal subgraph/region partition으로 적용할 수 있게 한다.

<a id="iree-flow--stream--hal"></a>
### IREE Flow / Stream / HAL

참고:

- tensor dataflow(Flow), async scheduling/resource lifetime(Stream), hardware abstraction(HAL)을 분리
- resource size와 lifetime을 명시적으로 추적
- async execution은 timepoint로 availability/order를 표현
- allocation/reuse는 scheduling 뒤에 구체화

RustJ 적용:

- Logical ArrayValue와 physical resource를 분리한다.
- native Physical Plan의 async dependency는 explicit Timepoint/AsyncToken으로 표현한다.
- resource lifetime과 buffer reuse는 physical timeline을 기준으로 판단한다.

<a id="stablehlo--vhlo"></a>
### StableHLO / VHLO

참고:

- portable high-level op set과 명시적 specification/verifier/type inference
- side-effecting op는 token으로 ordering 가능
- custom_call/composite로 확장 가능하지만 semantic contract가 필요
- portable artifact는 별도의 versioned VHLO/compatibility layer로 관리

RustJ 적용:

- StableHLO는 전체 J IR이 아니라 안전한 tensor/NN subset export target이다.
- external effect mapping은 token/adapter contract로 검증한다.
- RustJ Logical IR도 외부 interchange를 시작할 때 schema version과 migration 정책을 둔다.

<a id="triton"></a>
### Triton

참고:

- tensor/block program과 backend schedule configuration을 분리
- block size, warp 수, pipeline stage, register limit은 semantic op가 아니라 compilation configuration
- layout/access constraint가 codegen 품질에 직접 영향

RustJ 적용:

- tile/warp/stage/register cap은 Physical Schedule/TargetProfile 쪽에 둔다.
- Logical IR에는 이를 선택할 수 있게 하는 axis/access/constraint fact만 유지한다.

<a id="중요한-비채택-사항"></a>
### 중요한 비채택 사항

- NumPy broadcasting을 J agreement로 대체하지 않는다.
- tensor framework dtype policy를 J type semantics로 대체하지 않는다.
- backend layout을 J noun identity로 만들지 않는다.
- framework의 JIT 성공 사례를 RustJ 성능 증거로 취급하지 않는다.

---


---
