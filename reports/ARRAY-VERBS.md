# 배열 조작 verb 추가 — 2026-09-27

reverse, scalar rotate/take/drop, monadic transpose를 지원한다. dtype과 나머지 cell shape를 보존하고 음수 개수·빈 배열·scalar를 처리한다. take 확장 시 숫자 0/문자 공백을 사용한다. 최대 음수 정수는 unsigned_abs로 처리해 오버플로하지 않으며 표현할 수 없는 출력 크기는 limit error로 거절한다.

중간 인덱스 배열을 만들지 않고 원본 위치를 계산해 typed output에 직접 기록한다. 공유 입력을 변경하지 않는다. 전치 두 번/뒤집기 두 번의 복원, 행 단위 처리, 채움 위치, 실패한 대입 보존을 Rust 테스트로 확인한다.

지원 범위: 회전/take/drop은 스칼라 왼쪽 인수만. count list, dyadic transpose, monadic head/tail, custom fill은 미지원이다. 일반 결과는 복사하며 strided view 최적화는 후속이다.

차등 corpus에 253개를 추가하여 기본 933개 + 상태 기반 생성 800개 = 1733개다. C AVX2와는 전부 일치하며 C j64에서는 기존 타입 차이 1개를 별도로 보고한다. Rust default/portable 각각 27개 일반 테스트와 문서 수명 테스트를 실행한다. 원격 CI·upstream 전체·sanitizer/Miri 검증은 이번에 수행하지 않았다.
