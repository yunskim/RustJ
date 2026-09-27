# CPU dyadic rank — scalar rank 단계

2026-09-26. `x u"n y`에서 스칼라 정수 n을 지원한다. 음수는 각 인수의 rank에서 상대적으로 계산하고, 양수는 각 인수 rank까지 제한한다. frame은 prefix가 일치해야 하며 짧은 frame의 셀은 긴 frame의 나머지 축에 반복된다. 입력 셀은 ArrayView로 차용한다.

산술 + - * %는 차용 셀에 직접 계산한다. 나머지 기존 dyad는 셀을 소유 값으로 만든 후 기존 구현을 호출하므로 복사 비용과 기존 지원 한계가 남는다. 결과는 CellBuilder로 순차 조립하고 후속 셀의 실수 승격을 앞선 결과에도 적용한다. 셀별 계산 오류를 조립 오류보다 먼저 보고한다.

이번 차등 검증 범위는 산술·비교 연산, 같은/다른 frame 길이, frame 불일치, 음수 rank, scalar 확장, 빈 셀, overflow 승격이다. 빈 frame의 prototype 추론, dyadic reduction, 서로 다른 결과 shape의 padding은 아직 지원하지 않는다. 일반 rank 산술 경로는 현재 scalar typed loop이며 dense atomic의 AVX2 경로와 구분된다. 모든 rank 연산이 SIMD 가속된다고 주장하지 않는다.

C 참조는 고정된 jsource의 jsrc/cr.c rank 실행 및 별도 프로세스 oracle을 사용한다. 재현은 tools/conformance.py와 tests/semantics.rs의 dyadic_scalar_rank_frame_repetition_and_errors에서 가능하다. CUDA 코드는 추가하지 않았다.

## rank 목록 추가

1개는 monad/left/right에 같은 rank, 2개 `l r`은 monad=r 및 left=l/right=r, 3개 `m l r`은 각각 지정한다. 음수도 인수별로 적용한다. 현재는 직접 쓴 숫자 literal만 지원하고 rank를 계산하는 표현식·동사 operand·무한 rank는 추가 작업이다. 4개 이상은 length error다. 결과 padding과 빈 frame prototype 제한은 유지한다.

기본 및 portable 각각 일반 테스트 25개와 문서 compile-fail 1개, clippy 검증을 수행한다. C 차등 corpus는 680개로 확대했다.

기준선 차이: `(i.2 3) -"1 0 (i.2 3 4)`는 고정된 C j64 빌드에서 Float, j64avx2에서는 Int다. 각각 새 프로세스에서 3회 확인했으며 값과 shape는 같다. RustJ는 Int를 유지한다. j64 비교는 이 dtype 차이 1개가 남으며 AVX2 기준선을 별도로 비교한다. 원인이 규명되지 않은 C 빌드 간 차이를 RustJ의 불필요한 승격으로 모방하지 않았다. 차등 보고서에 실제 reference library 경로를 기록하도록 개선했다.
