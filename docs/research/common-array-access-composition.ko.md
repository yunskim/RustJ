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
| axis_permutation_composition_matches_two_stage_pullback | 독립적인 두 축 순열의 합성 확인 | 작은 모든 유효 좌표에서 직접 역추적과 합성 역추적이 일치 | 축 합성 방향 또는 좌표 매핑 오류 | CI 검증 대기 |
| axis_access_rejects_unproved_shape_and_index_relations | 불완전한 증거의 거부 | rank, bijection, 중간 shape, 좌표 범위 위반을 거부 | 잘못된 접근 관계를 정당한 증거로 허용 | CI 검증 대기 |
| axis_composition_preserves_scalar_and_zero_extent_shapes | scalar/빈 축 경계 유지 | rank 0, shape의 0 크기 축, 무효 좌표 검사 보존 | 빈 프레임/축 정보 손실 또는 없는 atom 접근 허용 | CI 검증 대기 |

## 다음에 필요한 독립 증거

- VerifiedFrontend/J Graph의 특정 StaticReindex 노드가 이 접근 관계를 실제로 만족하는지 증명하는 연결
- 메모리/I/O 지역에 대한 안전한 backward demand 전파
- J-C 오라클로 fill/prototype, check/error order 및 Rank 의미 동치 확인
- 두 연산 계열 이상에서 동일한 region transformer 재사용
- 그 뒤에만 GF4/FW-05–09의 Rewrite/EqGraph 근거 후보로 승격

**현재 증명 범위: 좌표와 shape.** 실제 J 동치나 성능 이득은 미검증.
