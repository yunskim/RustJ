# Futhark 컴파일러 선행연구: RustJ Graph/Logical IR 책임과 구현 참고

> 검토일: 2026-11-13. **문헌·설계 분석이며 RustJ 구현/테스트 완료가 아니다.**
> 아키텍처 불변조건: 현재 **J Graph IR \`j_graph_ir::Plan\` → Verified Logical Execution IR \`logical_ir::Plan\` → Route/Schedule/Physical** 경계를 유지한다. Futhark 전용 IR, 제3의 canonical IR, 기본 autograd, Futhark 필수 backend를 추가하지 않는다. [정본 §2.1](../project/chapters/03-architecture-p01.ko.md#두-ir-사이의-최적화-책임), [공통배열대수 선행연구](common-array-algebra-prior-art.ko.md), [einfold 검토](xarray-sql-attention-review.md).

## 1. 조사 범위와 증거 수준

2026-11 Reddit r/apljk에서 소개한 [Futhark-J Bridge](https://web.liminal.cafe/~byakuren/j/futhark/)는 **J 소스를 RustJ처럼 자동 컴파일하는 시스템이 아니다.** J에 포함한 **별도 Futhark 소스**를 \`futhark multicore --library\`로 C 라이브러리로 빌드하고 J FFI로 호출하며, 캐시·persistent context를 관리하는 bridge다. 원문 v1.7.0의 123-test와 1,525배 Mandelbrot 가속 등은 개발자가 보고한 결과다. 성능 수치는 참조 구현, 입력·오라클·스레드 수·컴파일/FFI 비용 등을 통제한 RustJ 측 재측정 전에는 받아들이지 않는다. Dot product처럼 J가 더 빠른 예도 있다.

**초점은 Bridge가 아니라 Futhark compiler 연구다.** 다음 다섯 축을 구분한다.

| 연구 | 검증한 원문 | 검토 가능한 주장 | RustJ에서 결정할 주체 |
|---|---|---|---|
| Redomap (Henriksen, Larsen, Oancea, ARRAY 2016) | [저자 공개 논문](https://www.futhark-lang.org/publications/array16.pdf) | producer–consumer·horizontal fusion, map/reduce 결합, 순서 제약과 coalescing | Graph 후보와 합법성 → Logical 의미 계약 → Physical 구현 |
| Moderate/regular flattening (Henriksen 외, PLDI 2017; Futhark 교재 §8) | [PLDI DOI](https://doi.org/10.1145/3062341.3062354), [교재](https://futhark-book.readthedocs.io/en/latest/regular-flattening.html) | 규칙적인 중첩 map/reduce/scan을 segmented 연산·커널로 매핑 | Logical의 인덱스·세그먼트 의미와 Schedule의 병렬화 |
| Incremental flattening (Henriksen, Thorøe, Elsman, Oancea, PPoPP 2019) | [논문](https://futhark-lang.org/publications/ppopp19.pdf), [DOI](https://doi.org/10.1145/3293883.3295707) | 병렬화 정도가 다른 다수 code version + runtime guard/autotuning | Schedule/Physical의 버전·가드·비용 |
| Full flattening (Henriksen, 2026-07-31) | [공식 구현 해설](https://futhark-lang.org/blog/2026-07-31-full-flattening.html) | 불규칙한 중첩 병렬성을 segment/packing 등으로 처리하는 재설계 | Logical의 domain/shape 정당성 + 선택적 physical mapping |
| Parallel accumulators (Henriksen, 2026-02-23) | [공식 IR 설계 해설](https://futhark-lang.org/blog/2026-02-23-accumulators.html) | 선형·write-only accumulator로 병렬 불규칙 scatter-accumulate 표현 | Logical의 effect/alias/누적 계약, Physical의 atomic/segmented 실행 |

위 2026년 블로그의 설명은 **연구가 제시한 설계·당시 상태**다. 특히 7월 full flattener 글은 해당 구현을 “soon-to-be-merged”로 설명하므로, 현재 RustJ가 같은 기능을 구현했다거나 모든 비균일 입력에서 고성능이 보장된다고 말하지 않는다.

## 2. IR 책임을 그대로 유지하면서 차용할 핵심

| 기존 RustJ 계층 | 보존하는 것 | Futhark에서 연구할 방식 | **넘기지 말아야 할 결정** |
|---|---|---|---|
| Frontend / J Semantic Construction | J의 품사·derived verb·rank conjunction, name/version, 효과, source provenance | J source를 직접 Futhark SOAC로 취급하지 않음 | 새로운 frontend grammar나 자동 \`map\` 의미 강제 |
| **J Graph IR** | Applied J operation topology, 고수준 \`StructuredConv\` / Fold / Scan, GraphFacts, Algebraic View, 원본 graph/provenance | map-map, producer-consumer/horizontal fusion 등 **대수적 동치 후보**; 비파괴 Algebraic Expansion; 동치 검증에 필요한 shape·reduction 법칙 | kernel, thread, tile, buffer, layout, materialization |
| **Verified Logical Execution IR** | 선택된 Graph의 normalized logical operation/SSA, \`CellApply\`, \`IndexMap\`, \`Reduce/Contract\`, \`SemanticCheck\`, error/effect order, \`j_origin\` | 선택적 Logical Decomposition, segment flags/offsets, reduction domain, 연산별 dependency/alias/accumulation 증거, 후보별 legality | target-specific block/workgroup 크기 또는 실제 atomic 코드 선택 |
| Route / Schedule / Transform / Physical | target capabilities, memory space, schedule, tile, kernel, barriers, buffers | incrementally flattened alternatives, vectorization avoidance, loop interchange 적용 여부, segmented kernel, atomic vs deterministic reduction | J 언어 의미를 바꿀 새 primitive identity |
| Executor / backend | 검증된 물리 실행과 fallback | 벤치마크, runtime guard, 자원·성능 선택 | 최적화가 실패해도 J 의미가 달라지는 fallback |

**주의:** Futhark의 IR 구분을 RustJ에 일대일로 가져오지 않는다. Futhark가 \`map\`, \`redomap\`, \`segscan\`을 IR에 두었다고 해서 RustJ Graph IR에 같은 variant를 즉시 추가해야 하는 것은 아니다. *연산의 수학적 동치 탐색*은 Graph, *인덱스·데이터 의존성·검사*는 Logical, *동일 계산의 실제 실행 형태 선택*은 Schedule/Physical의 책임이다.

## 3. Redomap과 fusion: 무엇을 하나의 연산으로 묶을 것인가 (2016)

### 3.1 Producer–consumer fusion

\`\`\`text
b = map g a
c = map f b
     ⇒
c = map (f ∘ g) a
\`\`\`

두 번의 배열 순회를 한 번으로 합칠 수 있다. \`b\`의 물질화가 사라질 가능성이 있지만, 이것이 곧 실행 시 버퍼를 없앴다는 뜻은 아니다. 원본 값 \`b\`를 다른 소비자가 사용할 때는 fanout, 재계산, 공유를 함께 따져야 한다. RustJ의 Graph IR은 합성 후보와 소비자 의존성을, Logical IR은 해당 후보의 값·검사·효과 순서 조건을 유지한다.

### 3.2 Map–reduce 결합과 redomap

\`\`\`text
s = reduce (+) 0 (map (\x -> h(x)) a)
\`\`\`

중간 \`map\` 배열을 생성하지 않고, 변환된 값을 읽으면서 축약할 수 있다. 2016 논문의 **\`redomap\`**은 단일 \`map\` 최적화만을 말하지 않는다. producer–consumer fusion과 공통 입력을 이용하는 horizontal fusion으로 map/reduce 조합을 나타내는 compiler 내부의 결합 연산이다. 논문은 병렬성의 일부를 순차화하면서도 메모리 coalescing을 유지하는 lowering을 논의한다.

**중요한 조건:** reduce의 결합성만으로 병렬 분할이 정당화되는 경우가 있고, 교환성은 별도로 필요할 수 있다. 순서를 바꾸지 않는 결합과 순서를 뒤바꾸는 교환을 혼동하지 않는다. IEEE 부동소수점 \`+\`는 일반적인 실수 덧셈처럼 결합적이지 않다. RustJ의 \`+/ . *\`, contraction, fold/scan, \`(+/)\`와 관련한 재결합은 Exact-J 동치 규칙으로 무조건 등록하지 않는다. J의 \`9!:18/19\`, tolerant equality는 비추이적 근사 관계이므로 e-class union의 근거가 될 수 없다.

### 3.3 Fusion과 fission은 반대 방향이지만 모두 필요하다

\`map f (map g xs)\`를 합치는 것은 fusion. \`map (\x -> let a=f x; b=g a in b) xs\`를 여러 map으로 **쪼개는 것**은 fission/loop distribution이다. Full flattening은 후자를 사용해 중첩 구조를 다룰 수 있는 단순 연산으로 나눈다. 지나친 fission은 kernel launch 및 전체 메모리 왕복을 증가시킨다. **컴파일러의 목표는 fusion만 최대화하거나 flattening만 극대화하는 것이 아니라, 양자의 균형을 결정하는 것**이다.

## 4. Moderate/regular flattening: 중첩 병렬성을 일부만 펴기 (2017)

중첩 연산 예:

\`\`\`text
map (\row -> reduce (+) 0 row) matrix
\`\`\`

외부 행과 내부 열을 모두 병렬 처리하는 알고리즘이 항상 가장 빠르지 않다. Futhark의 moderate flattening은 *모든 병렬성을 무조건 전개하지 않고*, 병렬화 경계와 메모리 지역성을 고려해 일부 수준만 펼친다.

- \`map(map f)\`: 중첩 차원을 일시적으로 합친 뒤 \`map f\`, 이후 원래 shape 복구가 가능한 규칙적 경우를 활용한다.
- \`map(scan op z)\`: 행별 scan을 **segmented scan**으로 변환할 수 있다. 각 segment 시작 위치가 표시되어 서로 다른 행의 누적이 섞이지 않는다.
- \`map(reduce op z)\`: segment별 reduction/관련 결합 연산으로 처리할 수 있다.
- \`map(loop ...)\`: 바깥 map의 여러 원소에서 내부 loop를 실행할지, 적법한 경우 loop와 map의 순서를 바꿀지 선택한다. 후자가 항상 더 빠르거나 항상 합법적인 것은 아니다.

### Segmented scan의 핵심 원리

정수 값 \`[1,2,3,4,5,6]\`, segment 시작 플래그 \`[T,T,F,T,F,F]\`라면 세 segment \`[1]\`, \`[2,3]\`, \`[4,5,6]\`의 prefix sum을 이어 붙인 결과는 \`[1,2,5,4,9,15]\`다. 일반 \`scan\`의 상태를 \`(value, start_flag)\` 쌍으로 확장하면, 새 segment의 시작에서 누적을 재설정하는 **결합 연산**으로 정의할 수 있다. 결합법칙 및 항등원이 성립하는지 연산자별 확인이 필요하다.

**RustJ 적용:** \`Rank/CellApply\`를 성급하게 실제 loop로 바꾸지 않는다. Logical IR에서 **frame/cell 경계, uniform cell length, segment boundary**를 구체적으로 증명하고, Physical planner가 row kernel / segmented scan / sequential inner loop 등을 선택한다. 이 사례는 Graph/Logical 분리를 지지한다.

## 5. Incremental flattening: 한 계산의 여러 물리 버전 (2019)

2019 PPoPP 논문은 중첩 병렬성의 깊이와 데이터 크기에 따라 어느 계층까지 병렬화할지 달라진다는 문제를 다룬다. 단 하나의 고정 schedule 대신, **유효한 병렬화 깊이가 다른 복수의 코드 버전**을 생성하고, 대상 GPU·데이터 크기에 맞춰 튜닝한 조건으로 버전을 고른다. 논문은 GPU 2종, 금융 애플리케이션 3종, Rodinia benchmark 6종을 평가하고, 경우에 따라 코드 크기가 수배 증가할 수 있음을 보고한다.

\`\`\`text
동일한 검증된 Logical 계산
    ├─ schedule A: outer map parallel / inner reduce sequential
    ├─ schedule B: outer map + segmented inner reduce
    └─ schedule C: map/loop interchange 또는 tile+local reduction
                  ↓ runtime-guard/cost
             선택된 physical plan
\`\`\`

위 A/B/C는 RustJ에서 가능한 **설명용 후보**이지 논문의 특정 버전을 그대로 복제한 표가 아니다. 핵심은 **하나의 canonical Logical IR을 그대로 두고 여러 물리 계획을 생성할 수 있다는 점**이다. Shape 범위, hardware limit, 메모리 상한, legality proof, 런타임 guard의 실패 경로를 별도로 다룬다. \`PhysicalPlan\`의 일부가 아닌 새 필수 IR 계층은 만들지 않는다.

## 6. Full flattening: 불균일한 중첩 길이까지 다루기 (2026)

기존 regular flattening은 같은 수준의 내부 배열 길이가 일정하다는 조건에서 비교적 간단하다.

\`\`\`text
map (\n -> reduce (+) 0 (iota n)) [1,3,2]
\`\`\`

내부 길이가 1, 3, 2로 **서로 다르다**. Futhark 언어/IR의 정규 배열은 행별 길이가 다른 ragged array를 그대로 허용하지 않는다. 새 full flattening 접근은 개념적으로 중첩 배열을 다음처럼 나타낸다.

\`\`\`text
lengths   = [1, 3, 2]
offsets   = [0, 1, 4]         # 배타 prefix sum
flat vals = [0, 0,1,2, 0,1]
flags     = [T, T,F,F, T,F]   # segment head
\`\`\`

이 표현으로 segment별 \`iota\`, \`scan\`, \`reduce\` 등으로 바꿔 **병렬 계산을 평탄한 구조에서 수행**할 수 있다. 이 중간 표현은 문헌을 설명하는 논리적 모델이며, RustJ에 필수 ragged array IR을 추가하자는 뜻이 아니다.

2026년 공식 해설의 주요 구현 아이디어:

1. **Fission + segmented rewriting:** 복합 \`map\`의 본문을 단순 연산으로 분해한 뒤 segmented 대응 연산으로 전환한다.
2. **Branch packing:** 분기 조건에 따라 lane을 분류해 각 분기를 처리한 후 원래 순서로 합친다. 재배치/패킹의 비용이 있다.
3. **Nonuniform loops:** 반복 종료 상태가 다른 lane들을 추적하며 수행할 수 있지만, 반복적인 필터·이동 때문에 비용이 클 수 있다.
4. **Vectorization avoidance:** 스칼라 문장 묶음은 쪼개지 않고 유지해 불필요한 중간 배열·메모리 왕복을 줄인다.
5. **Uniformity-specialized path:** 크기 타입(shape/size facts)이 균일성을 증명하면 segment metadata 없이 정규 다차원 배열 경로와 후속 tiling을 이용한다.
6. **Loop interchange legality:** \`map\`과 \`loop\`를 뒤집는 단순한 경우에도 \`f\`의 shape 보존과 입력의 규칙성을 확인한다.

따라서 **full flattening = 모든 중첩 병렬 계산이 언제나 빠르게 실행된다**는 주장이 아니다. 오히려 “의미상 다룰 수 있는 범위를 넓히고, regular fast path를 보존하며, 실행 품질은 별도 최적화로 개선한다”는 연구다. RustJ에서는 Graph의 고수준 구조를 버리지 말고, Logical에서 필요할 때 segment/domain 변환을 **조건부 표현**으로 검토한다.

## 7. Parallel accumulators: 역방향·scatter 누적을 어떻게 안전하게 표현하는가 (2026)

문제는 \`gather\`의 명시적 adjoint나 weight-gradient처럼 여러 결과가 하나의 입력 원소로 **간접 누적**될 때 나타난다.

\`\`\`text
# y[k] = x[index[k]]   (gather)
# explicit input-adjoint conceptual form:
dx = zeros_like(x)
for k:
    dx[index[k]] += dy[k]
\`\`\`

중복 인덱스가 있으면 여러 스레드가 같은 \`dx[i]\`에 쓸 수 있다. 모든 반복이 길이 \`size(x)\`인 희소 벡터를 만들고 마지막에 더하는 방식은 불필요한 O(n*m) 중간 데이터를 만들 위험이 있다. Futhark는 이를 **write-only accumulator**라는 내부 IR 개념으로 표현한다.

- \`with_acc\`: 원본 배열에 대한 accumulator 수명을 시작·끝냄.
- \`update_acc\`: 지정 index에 결합 연산에 따른 누적을 기록.
- \`acc(c,t)\`: 해당 \`with_acc\`에서 부여한 **고유 certificate \`c\`**를 갖는 accumulator 타입.
- **Linearity**: accumulator의 임의 복제·관찰·교차 반환을 금지하여 읽기/쓰기·alias 혼동을 막음.
- **Parallel map 특례**: \`map\`의 반복마다 논리적으로 누적 토큰을 분기했다가 합치도록 정의한다. 개별 iteration의 공유 효과를 무제한 허용하는 일반 효과 시스템과 다르다.

설계상 accumulator의 의미는 \`(index,value)\` 갱신 목록으로 설명할 수 있지만, 실제 코드 생성은 상황에 맞게 같은 배열의 atomic update 등으로 구현한다. Futhark 공식 글은 IR 안에서만 이 구조를 사용하며 source language에 안전하게 노출하는 문제는 해결되지 않았다고 한다.

**RustJ 적용 결론:** 새로운 사용자 노출 \`with\` 의미나 implicit autograd를 도입하지 않는다. 기존 **명시적 adjoint primitive**를 유지한다. Logical IR의 검증 가능한 누적/의존성/alias contract 또는 \`scatter-add\`의 제한된 logical operation 후보를 검토할 수 있다. 실제 atomics, segmented reduction, deterministic gather-then-reduce는 Schedule/Physical에서 선택한다. 이 둘은 **동일한 bitwise 결과를 보장하지 않을 수 있다.** 결합·교환법칙, 중복 인덱스, bounds, dtype, NaN/Inf, signed zero, overflow, 첫 오류, 결정론을 포함한 별도 proof가 필요하다.

참고: **RustJ의 기존 \`with\` conjunction과 Futhark \`with_acc\`는 전혀 다른 개념**이다. 이름이 비슷하다고 통합하거나 의미를 변경하지 않는다.

## 8. RustJ 식으로 연결한 세 가지 동작 예

### 8.1 Scan over cells

\`\`\`text
Graph:       CellApply(Scan(op), X)
Graph view:  원본 유지 + 균일한 segment별 scan 후보
Logical:     frame/cell index range, lengths, start flags,
             identity/associativity witness, J fill/error facts
Physical:    per-row scan | segmented scan | sequential fallback
\`\`\`

\`Scan\`의 합법적 segment 구분은 *연산의 값 의미*와 관련되고, segment flag의 구체적 저장·배치는 physical decision이다. 비균일한 J rank/cell 규칙을 단순한 ragged-array 형식으로 등치하지 않는다.

### 8.2 반복되는 원소별 logistic recurrence

\`\`\`text
for step:
  for element: x[element] = f(x[element])
\`\`\`

와

\`\`\`text
parallel for element:
  for step: x[element] = f(x[element])
\`\`\`

는 원소간 독립성과 동일한 각 원소의 반복 순서, effect/error 조건이 충족되면 같은 수학적 계산이다. 후자는 원소별 locality에 유리할 수 있고, 전자는 벡터화를 활용할 수도 있다. 어떤 쪽이 빠를지는 실측할 문제다. 이것은 **Graph IR에 새로운 primitive를 추가해야 한다는 예가 아니라 기존 Logical 계산의 schedule 비교 예**다.

### 8.3 Conv/Contraction의 explicit weight adjoint

\`\`\`text
Graph:      StructuredConvWeightAdjoint(X, dY, spec)   # 고수준 유지
Algebraic:  필요한 경우 중첩 sum-product 의미식 조회
Logical:    window/axis/reduction domain; logical sum contributions
Physical:   tiled gather-reduce | segmented reduce | guarded scatter-atomic
\`\`\`

여기서 \`WindowView\`/IndexMap을 Logical에서 설명했다고 물리적인 \`im2col\` 배열이 생성된 것은 아니다. Scatter-atomic도 항상 정당한 선택은 아니다.

## 9. 채택/보류 및 검증 체크리스트

| ID | 할 일과 검증용 반례 | 책임 / 상태 |
|---|---|---|
| **FUT-01** | Map→Map fusion의 fanout, J NAME/효과·first-error 및 원본 provenance 보존 | Graph + legality — [ ] 미구현 |
| **FUT-02** | Map→Reduce/Redomap 후보: associative/noncommutative, zero-length, dtype/overflow/IEEE 반례 | Graph + Logical — [ ] 미구현 |
| **FUT-03** | \`CellApply(Scan)\`의 frame/cell/segment 대응과 \`J-C\` 오라클 검증; empty frame·prototype 포함 | Logical — [ ] 미구현 |
| **FUT-04** | uniform/nonuniform shape facts, segment length/offset/flags 및 bounds witness | Logical — [ ] 미구현 |
| **FUT-05** | map/loop interchange 조건, iteration 별 shape/first-error/ordered effects의 음성 테스트 | Logical legality + schedule — [ ] 미구현 |
| **FUT-06** | fused vs fissioned kernel의 memory traffic·temp bytes·launch count 비교; 지나친 fission 차단 | Schedule/Physical — [ ] 미구현 |
| **FUT-07** | 동일 verified Logical 계획에서 2개 이상 physical schedules 및 실패 시 fallback; guard determinism | Schedule/Physical — [ ] 미구현 |
| **FUT-08** | 명시적 gather/Conv adjoint의 중복 index accumulate: alias, read-before-finish, race, bounds | Logical contracts — [ ] 미구현 |
| **FUT-09** | atomic vs deterministic segmented reduction의 exact/approximate 수치 정책; J tolerant equality로 e-graph union 금지 | Numeric policy + runtime — [ ] 미구현 |
| **FUT-10** | Bridge 벤치마크 재현: 동일 수학 알고리즘·동일 결과·J-native 배열식·compile/FFI 시간 분리, threads/scaling/size 측정 | Independent benchmark — [ ] 미구현 |
| **FUT-11** | Full flattening 알고리즘의 upstream 버전 고정 및 실제 compiler IR pass 확인 후 RustJ feature scope 재검토 | Research follow-up — [ ] 미검증 |
| **FUT-12** | Futhark/einfold/SPORES와의 중복 규칙 확인, 기존 \`j_graph_ir.rs\`·\`logical_ir.rs\`·\`fusion_planning.rs\`·\`physical_plan.rs\` 경계 유지 | Integration audit — [ ] 미구현 |

각 체크리스트의 완료는 코드/테스트·실제 commit/CI 또는 정형 증거를 동반해야 한다. **문서 작성만으로는 완료로 표시하지 않는다.** 변환 전후 값만 비교하지 말고 shape/rank·fill/prototype·first error·effect/name binding·numeric policy·memory·실행시간을 따로 측정한다.

## 10. 선행연구의 역사적 연결

Futhark의 flattening은 독립적인 새 발명이 아니다. **NESL / Blelloch**의 nested data parallel flattening과 segmented scan, **Data Parallel Haskell(DPH)**의 vectorization avoidance, **Accelerate** 등 functional array compilation의 성과를 잇는다. Futhark는 초기에는 uniform 부분집합·moderate flattening으로 실용적인 지역성을 중시했고, 2019년에는 incremental flattening으로 여러 스케줄을 다뤘으며, 2026년의 full flattening 재설계는 불균일한 중첩 병렬성을 segmented representation, branch packing, fission 제어로 확장했다. Accumulator 연구는 이와 별도로 역방향 계산의 누적 효과를 compiler IR에서 다루는 문제를 해결한다.

RustJ 관점에서 **SPORES/egg의 algebraic equality search와 Futhark의 flattening은 같은 단계의 대체물이 아니다.** 전자는 값 계산의 동등한 *Graph 형태*를 탐색하고, 후자는 선택된 계산의 합법적 *병렬 실행 형태*를 탐색하는 데 더 가깝다. \`einfold\`의 producer/consumer fold pipeline witness와도 보완 관계다. SQL 비유를 사용하면 Graph의 관계대수 rewrite, Logical의 접근·검증·축약 계획, Physical의 join/kernel/schedule 선택에 대응하되 **J가 순수 SQL이라는 뜻은 아니다.**

## 11. 원문 및 출처

1. [Futhark-J Bridge — 원문](https://web.liminal.cafe/~byakuren/j/futhark/) — J↔Futhark FFI 사용법, 구현, 성능 주장의 **원출처**.
2. Troels Henriksen, Ken Friis Larsen, Cosmin E. Oancea, **“Design and GPGPU Performance of Futhark's Redomap Construct”**, ARRAY 2016 — [공개 PDF](https://www.futhark-lang.org/publications/array16.pdf), [DOI](https://doi.org/10.1145/2935323.2935326).
3. Troels Henriksen 외, **“Futhark: Purely Functional GPU-programming with Nested Parallelism and In-place Array Updates”**, PLDI 2017 — [DOI](https://doi.org/10.1145/3062341.3062354).
4. **Parallel Programming in Futhark**, [Chapter 8: Regular Flattening](https://futhark-book.readthedocs.io/en/latest/regular-flattening.html) — segmented scan, map/map, map/scan, moderate flattening 예제.
5. Troels Henriksen, Frederik Thorøe, Martin Elsman, Cosmin Oancea, **“Incremental Flattening for Nested Data Parallelism”**, PPoPP 2019 — [PDF](https://futhark-lang.org/publications/ppopp19.pdf), [DOI](https://doi.org/10.1145/3293883.3295707).
6. Troels Henriksen, **“The why and how of parallel in-place random-access accumulation”**, 2026-02-23 — [공식 블로그](https://futhark-lang.org/blog/2026-02-23-accumulators.html).
7. Troels Henriksen, **“Full flattening of nested data parallelism”**, 2026-07-31 — [공식 블로그](https://futhark-lang.org/blog/2026-07-31-full-flattening.html).
8. [Futhark 공식 문서 — Bridges](https://futhark-lang.org/docs.html) — Futhark-J Bridge의 제3자 연결 및 C interface 위치.
