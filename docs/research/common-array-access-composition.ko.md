# R0 공통 배열 접근 대수: 정적 축 순열 합성

상태: 분석 전용 구현 후보. J 의미론의 동치 증명, Graph rewrite, EqGraph union, 실행 최적화는 **아직 허용하지 않는다**.
기준: PROJECT.ko.md의 공통배열대수 연구 가설과 SPORES 관계형 접근 설계(R0).

## 하나의 의미

논리 입력 배열의 shape S와 출력 축에서 입력 축으로 가는 순열 p를 갖는
StaticAxisAccess(S,p)를 정의한다. 모든 j에 대해 출력 shape[j]=S[p[j]]이며,
출력 좌표 o는 입력 좌표 i[p[j]]=o[j]에 대응한다.

첫 변환 p 다음에 q를 적용하려면 중간 출력 shape가 q의 입력 shape와 정확히
같아야 한다. 합성된 출력-입력 축 관계는 r[j]=p[q[j]]이다.
p와 q는 각각 축의 전단사 순열이어야 한다. rank 0과 0 크기 축도 보존한다.
데이터 레이아웃·stride·buffer는 이 정의에 포함되지 않는다.

이는 두 단계 **좌표 접근 관계**가 한 단계 관계와 같다는 구조적 사실이다.
J의 연산 자체가 동일하다는 결론이 아니다. 도중의 Rank/Cell/Frame,
프로토타입/Fill 검사, NAME, 효과 및 첫 오류 발생 순서를 제거해서는 안 된다.
정적 축 순열의 shape·순열 증거가 없으면 Unknown으로 남겨야 하며,
이 구현이 Dynamic Gather 또는 일반 Reshape를 처리한다고 주장하지 않는다.

## 테스트 의미 및 보장

| Test | Purpose | Guarantee | Failure meaning | Status |
|---|---|---|---|---|
| axis_permutation_composition_matches_two_stage_pullback | 독립적인 두 축 순열의 합성 확인 | 작은 모든 유효 좌표에서 직접 역추적과 합성 역추적이 일치 | 축 합성 방향 또는 좌표 매핑 오류 | PR #70 head CI PASS (2026-10-13) |
| axis_access_rejects_unproved_shape_and_index_relations | 불완전한 증거의 거부 | rank, bijection, 중간 shape, 좌표 범위 위반을 거부 | 잘못된 접근 관계를 정당한 증거로 허용 | PR #70 head CI PASS (2026-10-13) |
| axis_region_pullback_matches_composition_and_exact_requested_points | 정적 순열에서 출력 박스의 역방향 요구 영역 검증 | 합성·순차 역추적과 영역별 점 포함 관계가 정확히 일치하고 무효 범위를 거부 | 잘못된 데이터 영역 축소나 불필요한 축 삽입 | PR #71 및 병합 main CI PASS (2026-10-13) |
| axis_composition_preserves_scalar_and_zero_extent_shapes | scalar/빈 축 경계 유지 | rank 0, shape의 0 크기 축, 무효 좌표 검사 보존 | 빈 프레임/축 정보 손실 또는 없는 atom 접근 허용 | PR #70 head CI PASS (2026-10-13) |
| graph_monadic_transpose_projects_witnessed_axis_access | 실제 monadic `|:` J Graph의 역축 관계 추출 | 입력·출력 Shape/Rank 증거 및 원본 ValueId 보존, 두 번의 전치 합성 후 영역 역추적 확인 | 잘못된 Graph 노드를 정적 전치로 분류하거나 접근 축 방향을 잘못 계산 | PR #72 코드 커밋 `77badcd` CI PASS ([PR CI](https://github.com/yunskim/RustJ/actions/runs/38319113130)) |
| graph_transpose_access_refuses_unknown_and_unrelated_reindex_facts | 다른 Reindex·ReadNoun 및 불완전 facts 거부 | Reverse/Ravel/단순 명칭 및 Unknown/불일치 Shape·Rank에 witness를 부여하지 않음 | 추측한 의미론을 최적화의 근거로 허용 | PR #72 코드 커밋 `77badcd` CI PASS ([PR CI](https://github.com/yunskim/RustJ/actions/runs/38319113130)) |

| graph_monadic_map_and_transpose_share_region_pullback | 원본 Graph의 단항 Map→Transpose 공통 박스 역추적 확인 | 원본 ValueId·두 연산별 독립 증거·빈 값 영역·합성/순차 결과 일치 | 서로 다른 축 관계의 합성 오류 또는 중간 입력 경로 오인 | PR #75 코드 커밋 `a8e1f3e` PR CI PASS ([검증](https://github.com/yunskim/RustJ/actions/runs/38320754917)); main CI 미검증 |
| graph_unary_map_access_rejects_unsupported_calls_and_unknown_facts | 허용하지 않은 연산·모순된 Shape/Rank/계약 거부 | Dyad, Rank, Reverse, Ravel, Gather, Reduce, NAME 결과 및 불완전한 facts에서 임의 증거 미생성 | 잘못된 공통 접근 계약의 안전하지 않은 승인 | PR #75 코드 커밋 `a8e1f3e` PR CI PASS ([검증](https://github.com/yunskim/RustJ/actions/runs/38320754917)); main CI 미검증 |

## 다음에 필요한 독립 증거

- 실제 monadic `|:`의 Graph→정적 축 접근 투영은 R0.2에서 도입. 다른 StaticReindex, dyadic/ranked `|:` 및 전체 실행 의미론은 별도 증거 필요
- 메모리/I/O 지역에 대한 안전한 backward demand 전파
- J-C 오라클로 fill/prototype, check/error order 및 Rank 의미 동치 확인
- 두 연산 계열(Map·Transpose)의 동일한 논리 영역 역추적은 R0.3에서 시험; 실제 물리 I/O 절감과 J 관찰 동치는 아직 별도 증거 필요
- 그 뒤에만 GF4/FW-05–09의 Rewrite/EqGraph 근거 후보로 승격

**현재 증명 범위: 좌표와 shape.** 실제 J 동치나 성능 이득은 미검증.

## R0.1 직사각형 데이터 영역 역추적 (2026-10-13)

각 출력 축의 반열린 범위 R[j]가 주어지면 정적 전단사 순열 p에서
입력 축 p[j]의 요구 범위는 정확히 R[j]다. 범위의 길이와 순서는 바꾸지 않는다.
출력 rank, 시작/끝 순서, 실제 출력 extent를 검증하며, 빈 범위도 그대로 보존한다.
합성 순열에 적용한 결과는 두 변환에 차례로 적용한 결과와 일치해야 한다.

이 증명은 **논리적 value-region**에만 적용한다. 빈 결과가 프로토타입 계산,
오류 검사, 효과 실행을 생략할 수 있다는 증거가 아니며, 실제 저장소 청크,
실행 스케줄 및 파일 byte 범위를 허용하지 않는다. GF5a/IO-09·10의 실제
Graph access witness와 J 의미론 검증이 이어져야 한다.

## R0.2 J Graph monadic Transpose 연결 (2026-10-13)

`Plan::static_transpose_access(ValueId)`는 **원본 J Graph**에서
`PrimitiveVerb(Cant)`의 monadic atomic Apply이며 Graph Basis가
`StaticReindex`, Access가 `IndexTransform`이고, 입력·출력 Shape/Rank가
정확히 주어져 결과 Shape가 입력 Shape의 역순일 때만
`GraphTransposeAccess { input, output, access }`를 반환한다.
`Plan::verify()` 실패 시에도 실패 폐쇄 방식으로 `None`을 반환한다.

이 경로는 J 원본 그래프와 NAME 읽기, 에러/검사 순서를 그대로 유지하는
**분석용 값 좌표 투영**이다. 그래프 노드를 교체하거나 물리 read region,
EqGraph E-Class union 및 check 제거 권한을 부여하지 않는다.
`|.`, `,`, 동적/불일치 Shape, Rank가 적용된 전치, dyadic `|:`에는
동일한 근거를 추측해서 붙이지 않는다.

## R0.3 공통 좌표·영역 역추적: Unary Map × Transpose (2026-10-13)

`Plan::static_value_access(ValueId)`는 원본 J Graph에서 다음 두 연산 계열의
**논리 값 좌표 접근 관계만** 같은 `StaticAxisAccess`로 투영한다.

- **직접 단항 Map**: primitive `FunctionHead`, Atomic Monad, 정확한
  `Elementwise/CorrespondingElements` 계약, primitive의 `Map/Pure/PreserveRight`
  규칙, 알려진 일치 Shape/Rank가 모두 필요하다. 축 관계는 항등이다.
- **직접 단항 Transpose (`|:`)**: R0.2의 기존 증거를 사용한다. 축 관계는
  역순 전단사 순열이다.

출력 값 영역의 역추적은 두 계열 모두 동일한 `pullback_region`을 적용하며,
Map→Transpose의 역추적은 각 단계에 적용한 역추적과 합성 접근에 적용한
역추적이 일치해야 한다. 이것은 **공통 value-demand 좌표 전달 법칙**이며,
두 primitive 함수의 동치 또는 재배열 가능성에 대한 증명이 아니다.

단항 Map의 모든 요소가 오류 없이 평가된다고 보장하지 않는다.
선택되지 않은 요소에서 발생할 수 있는 J 오류, prototype/fill,
Rank/Cell/Frame, NAME·효과의 최초 관찰 순서 때문에 이 결과만으로
실제 부분 평가·I/O 축소·fusion·EqGraph 병합을 허용할 수 없다.
Dyad, Rank, Gather, 동적 이름, 불명확한 Shape는 해당 증거 없이 확장하지 않는다.
