# RustJ 공통배열대수 선행 연구와 J 의미론의 대응표

> 상태: **문헌·설계 검토 완료(1차) / 개별 법칙의 RustJ 정형 증명·재작성 승인은 미완료**.
> `PROJECT.ko.md`가 아키텍처·상태의 정본이며, 이 문서는 기존 [R0 공통 접근 관계 실험](common-array-access-composition.ko.md)과 [수치 오차 정책](common-array-numeric-error-policy.ko.md)에 딸린 **비교 연구·검증 체크리스트**다. 별도의 새로운 필수 IR이나 완성된 보편 배열대수를 선언하지 않는다.

## 1. 연구 목표의 변경

**공통배열대수를 처음부터 독립적으로 발명한다고 가정하지 않는다.** 우선 MoA의 shape/인덱싱, APL의 연산자 calculus, Remora의 rank-polymorphism, Bird–Meertens의 fold 법칙, SPORES의 관계적 sum-product, Futhark의 SOAC 최적화, RISE의 가이드 등가 포화를 **서로 다른 전제 조건을 가진 기존 이론**으로 검토한다. 각 연산을 RustJ의 원본 J 의미론에 맞춰 연결하고, *조건을 증명한 경우에만* 후보 재작성의 합법성 심사로 넘긴다.

| 선행 연구 | 확인된 범위 | RustJ의 접점 | 그대로 가져올 수 없는 조건 |
|---|---|---|---|
| **MoA — A Mathematics of Arrays** (Lenore Marie Restifo Mullin, **1988**) | shape를 이용한 배열식 기술, 인덱싱 함수 **Ψ**, 차원을 확장하는 연산자 **Ω**, n차원 배열식의 상징적 검증과 하드웨어 검증 | `StaticAxisAccess`, `pullback_region`, `value_demand_trace`의 좌표/shape 기반 형식 명세 | 실제 J의 `{` 인덱싱, `|:` 전치, `# RustJ 공통배열대수 선행 연구와 J 의미론의 대응표

> 상태: **문헌·설계 검토 완료(1차) / 개별 법칙의 RustJ 정형 증명·재작성 승인은 미완료**.
> `PROJECT.ko.md`가 아키텍처·상태의 정본이며, 이 문서는 기존 [R0 공통 접근 관계 실험](common-array-access-composition.ko.md)과 [수치 오차 정책](common-array-numeric-error-policy.ko.md)에 딸린 **비교 연구·검증 체크리스트**다. 별도의 새로운 필수 IR이나 완성된 보편 배열대수를 선언하지 않는다.

## 1. 연구 목표의 변경

**공통배열대수를 처음부터 독립적으로 발명한다고 가정하지 않는다.** 우선 MoA의 shape/인덱싱, APL의 연산자 calculus, Remora의 rank-polymorphism, Bird–Meertens의 fold 법칙, SPORES의 관계적 sum-product, Futhark의 SOAC 최적화, RISE의 가이드 등가 포화를 **서로 다른 전제 조건을 가진 기존 이론**으로 검토한다. 각 연산을 RustJ의 원본 J 의미론에 맞춰 연결하고, *조건을 증명한 경우에만* 후보 재작성의 합법성 심사로 넘긴다.

| 선행 연구 | 확인된 범위 | RustJ의 접점 | 그대로 가져올 수 없는 조건 |
|---|---|---|---|
| **MoA — A Mathematics of Arrays** (Lenore Marie Restifo Mullin, **1988**) | shape를 이용한 배열식 기술, 인덱싱 함수 **Ψ**, 차원을 확장하는 연산자 **Ω**, n차원 배열식의 상징적 검증과 하드웨어 검증 | `StaticAxisAccess`, `pullback_region`, `value_demand_trace`의 좌표/shape 기반 형식 명세 |  reshape, 일반 gather, Rank·빈 결과의 fill/prototype·오류를 Ψ만으로 설명했다고 가정하지 않음 |
| **An Operator Calculus** (Iverson, Pesch, Schueler, **1984**) | APL rank, 합성, 미분·역연산의 상호 작용과 **빈 프레임 결과 shape** 논의 | J Rank/CellApply·derived verb의 적용 순서와 empty-frame 경계 | J와 APL의 연산자·관찰 가능 동작이 같다는 직접 증명 없음 |
| **Remora — The Semantics of Rank Polymorphism** (Slepak, Shivers, Manolios, **2019**) | 핵심 함수형 배열 언어의 동적 의미론, shape-indexed 타입, progress/preservation을 통한 **잘 타입된 Remora 프로그램**의 shape 안전성 | J Graph `GraphFacts`·rank/cell/frame, prefix frame agreement, `CellApply`, 정적 shape witness | Remora의 타입 건전성이 곧 **전체 J의 타입/shape 안전성**이라는 뜻이 아님. J NAME·late binding·동적 Rank·fill/empty-frame·오류 순서 차이 검증 필요 |
| **Bird–Meertens Formalism** (1980년대) | 리스트의 map/fold와 homomorphism 및 프로그램 변환 법칙·병렬 scan 관련 도출 | `Map`, `Reduce`, `Scan`의 수학적 후보와 병렬 분해 증거 | 항등원·결합법칙·도메인·순서가 성립해야 함. IEEE 부동소수점/첫 J 오류에 자동 적용 불가 |
| **Tensor algebra** | 축, contraction, sum-product 등의 수학적 구조 | tensor/NN extended primitive를 shape·axis·reduction 관계로 기술 | 일반 텐서 수학에는 J primitive·boxed/sparse·오류의 전체 의미론이 포함되지 않음 |
| **SPORES** (Wang 외, **2020**) | LA ↔ RA 변환, 관계대수 동치포화, **논문이 다루는 sum-product 표현 범위**의 완전한 규칙 집합 | `GraphBasis`, 접근 관계, 후보 EqGraph와 비용 추출의 연구 모델 | sum-product 외 모든 J primitive에 대한 완전성 증거가 아님; 행렬/관계 정합성과 J 타입/연산 순서 전제조건 필요 |
| **Futhark** (Henriksen 외, **2017**) | bulk-parallel 연산, fusion 규칙, flattening, 참조 투명성을 유지하는 in-place update 모델 | `Map/Reduce/Scan`, fusion candidate, logical/physical 분리와 병렬 schedule | 순수 함수형 언어와 J의 NAME/효과/first-error/prototype 의미가 다름 |
| **RISE / Guided Equality Saturation** (Koehler 외, **2021–2024**) | 함수형 배열 재작성에 e-graph를 적용; unguided 포화의 규모 문제를 sketch/guide로 제어 | 선택적 `egg`/EqGraph, rule-space 제한, 단계적 증거와 비용 탐색 | 후보 규칙의 J 관찰 동치 증명은 **별도**; 탐색 성공이 규칙 건전성을 대신하지 않음 |

**문헌 검증 범위:** Syracuse University의 MoA 학위 기록에서 **1988, Ψ, Ω, shape와 symbolic verification**을 확인했다. 해당 공식 저장소에는 초록만 공개되어 **논문 전체의 세부 정리·전제는 아직 전수 확인하지 않았다**. 반면 Remora·SPORES·RISE/Guided Equality Saturation과 Futhark는 공개 논문/학술 초록으로 각 주장 범위를 확인했다. 확인하지 않은 정리를 RustJ의 증명으로 표기하지 않는다.

## 2. 최소 공통 코어: 기존 정의를 *옮겨 쓰기* 전에 입증할 사항

기존 J Graph의 `ValueId`를 보존하고 다음 정보가 있는 경우에만 관계를 구성한다.

```text
LogicalArray = (dtype, shape, logical atoms, J-visible representation constraints)
IndexedAccess(source, output, index relation, domain/shape proof)
OperatorApplication(primitive/derived identity, valence, intrinsic rank,
                    cell/frame agreement, fill/prototype, effects/errors,
                    NAME binding/version, comparison/CCT/Fit state)
CandidateLaw(source expressions, target expressions, assumptions,
             shape/index witness, numerical/observational proof, origin)
```

이것은 **검토용 명세 항목**이다. 새 자료구조를 곧바로 기존 Graph IR에 삽입하거나 별도의 `CommonArrayIR`을 필수 단계로 만들지 않는다. MoA Ψ를 `output coordinate → input coordinate` 관계와 비교할 수 있으나, `StaticAxisAccess`의 축 순열은 일부분이다. Gather/reshape/rank/reduction의 일반화는 아직 미증명이다.

등식 증거는 서로 다른 세 층으로 관리한다.

1. **좌표·shape 동치:** 역방향 demand/접근 좌표가 같은가? (R0 실험에서 일부 검증)
2. **순수 수학적 값 항등식:** 고정된 도메인, 결합/교환/분배 조건 아래 항등식이 성립하는가?
3. **Exact-J 관찰 동치:** 평가 결과·type/shape/rank·fill/prototype·첫 오류·NAME/효과·평가/합산 순서·현재 비교 허용 오차까지 보존하는가?

**1 또는 2만으로 3을 주장할 수 없다.** J의 비교 허용 오차(`9!:18`, `9!:19`, `!.t`)도 수치 최적화의 자동 예산이 아니다. 부동소수점 근사 동등성은 비추이적이므로 e-class `union`의 등호로 사용할 수 없다. [별도의 오차 정책](common-array-numeric-error-policy.ko.md)을 따른다.

## 3. RustJ 기존 구현과의 연결

| 기존 연구/기능 | 이미 확인된 사실 | 이번 문헌 통합으로 달라질 작업 |
|---|---|---|
| R0–R0.2 축 순열·박스 역추적·Graph 전치 | 좌표와 shape 관계의 분석/회귀 테스트 | MoA Ψ와의 정의 대응·반례를 작성; J 관찰 동치 승인 아님 |
| R0.3–R0.4 단항 Map/Transpose·Graph 역추적 | 일부 **증거가 있는** 연산 간 공통 demand 전달 | MoA식 접근 관계의 지원 도메인과 미지원 연산을 명시 |
| R0.5–R0.7 fanout 합집합·사용 간선·coverage | 원본 소비자별 논리 요구가 부분적일 수 있음을 보수적으로 보고 | 다중소비자/영역 합집합을 수학적 집합으로 정의; 전체 실행 요구의 증명은 여전히 별도 |
| J Graph `GraphFacts`·`GraphBasis` 및 Rank/CellApply | 이미 해석된 Function/valence/shape/프레임 정보가 있음 | Remora의 static/dynamic rank 전제와 J 원래 규칙을 하나씩 비교 |
| `j_graph_rewrite`, `j_graph_fusion` / egg 계획 | 후보 생성과 원본 보존 설계; 보편 EqGraph 완성 상태 아님 | SPORES/RISE 법칙을 등록할 때 도메인 증명·수치 정책·J-first-error/effect gate를 요구 |

## 4. 검증 체크리스트 (각기 독립된 작은 실험)

| ID / 후보 테스트 | Purpose | Guarantee / 전제 | Failure meaning | Status |
|---|---|---|---|---|
| **LIT-MOA-01** `moa_psi_static_axis_access_correspondence` | ψ 형태 인덱싱과 R0의 좌표 역추적 연결 | scalar/zero 축/2D transpose에서 shape·인덱스 대응 명시 | J/MoA 축 해석 오인 | [ ] 미구현 / 원문 정리 전수 미확인 |
| **LIT-RANK-02** `remora_rank_cell_frame_j_differential` | Remora 동적/정적 frame lifting과 J Rank/CellApply 비교 | positive/negative rank, prefix frame agreement, 0-cell/empty prototype 포함, **J-C 오라클** 기준 | Remora 타입 정리를 J 전체에 오적용 | [ ] 미구현 / 미검증 |
| **LIT-BMF-03** `fold_scan_law_has_domain_and_error_witness` | map/fold/scan 변환 법칙 후보 수집 | 연산별 결합법칙·항등원·shape·dtype, 숫자·오류/효과 전제 명시 | IEEE 재결합을 무조건 허용 | [ ] 미구현 / 미검증 |
| **LIT-SPORES-04** `sum_product_relational_rule_guard` | sum-product ↔ 관계형 법칙 대응 | 스키마/축·희소성·수치/shape/J 오류의 전제 명시 | 한정된 완전성을 전체 J로 확대 | [ ] 미구현 / 미검증 |
| **LIT-RISE-05** `guided_rewrite_requires_exact_j_witness` | e-graph guided 탐색 후보의 안전성 | 규칙/guide가 Exact-J 증거가 없는 경우 EqGraph union 거부 | 탐색 성공을 의미론적 동치로 오인 | [ ] 미구현 / 미검증 |
| **LIT-FUTHARK-06** `fusion_does_not_erase_j_obligations` | fusion/flattening 후보의 조건화 | 원본 Rank/NAME/first-error/fill/prototype 의무 유지 | 무증명 부분 실행·병렬화 승인 | [ ] 미구현 / 미검증 |
| **LIT-PRIM-07** primitive 법칙 등록부 | J primitive/derived 연산별 문헌·전제·증거 등록 | `SourceRule → Domain → Shape/Rank → NumericPolicy → Effects/Errors → Proof/Unknown` | 지원되지 않은 법칙을 완전성으로 표시 | [ ] 미구현 / 미검증 |
| **LIT-SRC-08** MoA 전체 정리 재검증 | 학위 초록 이상의 공식·법칙·가정을 전수 검증 | 증거와 검증하지 않은 주장을 구별 | 초록만 읽고 정리/완전성 과장 | [ ] 전체 본문 재검토 미실시 |

**우선순위:** LIT-MOA-01과 LIT-RANK-02를 선행하고, BMF/SPORES의 증거 있는 부분집합을 확장한 후 RISE/EqGraph 후보 생성·비용 추출을 연결한다. J-C 기준 의미론과 R0의 개별 테스트는 폐기하지 않는다. 각 체크는 한국어/영어 동기화, Test/Purpose/Guarantee/Failure meaning/Status, PR head/merged-main CI를 요구한다.

## 5. 서지 및 확인 가능한 원문

1. Lenore Marie Restifo Mullin, *A Mathematics of Arrays*, Syracuse University PhD dissertation, **1988** — [공식 학위 기록·초록](https://surface.syr.edu/eecs_etd/249/) (본문 공개 범위 제한).
2. Kenneth E. Iverson, Roland Pesch, J. Henri Schueler, *An Operator Calculus*, APL '84 — [ACM DOI](https://doi.org/10.1145/800058.801101).
3. Justin Slepak, Olin Shivers, Panagiotis Manolios, *The Semantics of Rank Polymorphism*, **2019** — [arXiv](https://arxiv.org/abs/1907.00509).
4. Richard Bird & Lambert Meertens 등의 1980년대 프로그램 대수 연구 — [Oxford Algebra of Programming 참고문헌](https://www.cs.ox.ac.uk/activities/publications/bibtex/algprog.html); Sergei Gorlatch, *Extracting and implementing list homomorphisms*, **1999** — [논문](https://doi.org/10.1016/S0167-6423(97)00014-2).
5. Yisu Remy Wang 외, *SPORES*, **2020** — [arXiv 논문](https://arxiv.org/abs/2002.07951), [PVLDB 전문](https://www.vldb.org/pvldb/vol13/p1919-wang.pdf).
6. Troels Henriksen 외, *Futhark: Purely Functional GPU-programming with Nested Parallelism and In-place Array Updates*, **2017** — [PLDI](https://doi.org/10.1145/3062341.3062354).
7. Thomas Koehler 외, *Sketch-Guided Equality Saturation*, **2021/22** — [arXiv](https://arxiv.org/abs/2111.13040); Thomas Koehler 외, *Guided Equality Saturation*, **2024** — [POPL 논문](https://doi.org/10.1145/3632900).

이 문서의 문헌 비교는 **원리·적용 전제의 검토**이지 각 논문 정리의 RustJ 번역을 모두 기계 검증한 결과가 아니다.
