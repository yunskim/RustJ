> Current local policy (2026-09-28): on this computer, run all tests,
> builds for validation, and static checks using native Windows tools only.
> Do not substitute WSL/Linux test runs. GitHub CI is skipped by user request.
> Earlier Linux results below are historical, not an instruction to run Linux CI.

# 지속 검증 전략

전체 C J 호환성은 아직 보장하지 않는다. 아래 검증은 구현된 부분집합의 회귀 방지 장치이며 upstream 전체 테스트 통과와 다르다.

## 모든 변경의 완료 조건

- 의미 변경에는 C 기준선과 비교하는 사례를 추가한다. 오류 수정은 실패했던 식과 필요한 대입 이력을 회귀 사례로 남긴다.
- 메모리/소유권 변경에는 별칭 불변성, 실패한 대입의 원상 보존, 보유 메모리 상한 검사를 추가한다.
- SIMD 변경은 0/1 원소, 벡터 폭과 tail 경계, 정수 극값·승격, 부호 있는 0 검사를 포함한다.
- rustfmt, clippy, 기본 및 portable Rust 테스트, 검증기 자체 테스트가 통과해야 한다.
- C 소스 revision을 고정하고 j64/j64avx2 각각에 Rust 기본/portable 결과를 비교한다. 예상 밖 차이·프로세스 오류·시간 초과·출력 누락은 실패다.
- 기능·성능 보고에 실행 환경, 지원 범위, 미실행 검사를 명시한다. 시험하지 않은 기능을 통과로 계산하지 않는다.

## 자동화한 검사

Linux CI는 push/PR/수동 실행에서 기존 680개 식과 seed 고정 생성 100라운드(800개 식)를 검증한다. 생성 사례는 SIMD 경계 길이를 포함하며 배열 생성→별칭→갱신→실패한 대입→값 재확인을 반복한다. 매일 UTC 18:23에는 날짜 seed와 1000라운드로 늘린다. GitHub의 일정 실행 시각은 지연될 수 있다.

타입·shape·오류는 정확히 비교한다. 유한 실수는 상대 오차 1e-14이며 0의 부호도 구분한다. NaN과 무한대는 JSON 표식으로 비교하므로 NaN payload 비트는 검사하지 않는다. 일부 Rust SIMD 단위 테스트의 비트 비교와 구분한다.

실패 보고서는 사례 index, 식, C/Rust 결과, seed, 실행 파일과 C 라이브러리 SHA256을 포함한다. 최초 실패까지의 전체 식 이력을 .repro.ijs로 저장한다. CI는 실패 시에도 보고서·C 빌드 manifest·로그를 artifact로 보관한다. 최소 실패식 자동 축소는 아직 없다.

## 알려진 차이의 관리

일반 C j64의 `(i.2 3) -"1 0 (i.2 3 4)` 타입 차이만 명시적인 --allow-known-j64로 분리한다. 정확한 식, 기준선 변형, 양쪽 타입·shape·값 조건을 모두 만족해야 하며 통과 수에 포함하지 않는다. 새 차이를 포괄 예외로 숨기지 않는다. 차이가 사라지면 예외 제거를 검토한다. AVX2 결과만 골라 전체 호환을 주장하지 않는다.

## 아직 추가해야 할 검증

- upstream test/*.ijs는 현재 미실행이다. J의 test harness·assert·제어문 지원을 조사해 원형 그대로 실행할 수 있는 파일부터 manifest에 편입한다. 추출·재작성한 사례는 upstream 파일 통과로 보고하지 않는다.
- 빈 frame, padding, boxed/complex/sparse, 함수·수정자 등 미지원 기능은 지원 범위 확대마다 별도로 편입한다.
- Miri는 portable 소유권/할당 코드부터, AddressSanitizer는 native SIMD 경로부터 별도 job을 구축한다. 현재 실행했다고 주장하지 않는다.
- default/portable의 직접 비트 단위 차등 실행, malformed-input 장시간 fuzzing, 다중 스레드 검증은 추가 과제다.
- 성능은 correctness 통과 후 동일 장비에서 교대 순서·반복 측정한다. 공유 CI runner 시간으로 엄격한 속도 gate를 걸지 않는다. 할당 횟수 등 결정적인 조건은 단위 테스트로 유지한다.
- CUDA는 보류 상태다. 재개 시 실제 장치의 결과·전송·완료 전 해제·오류를 검사하는 별도 gate를 추가한다.

## 운영

새 버그는 먼저 재현하고, 원인 수정과 회귀 테스트를 함께 제출한다. 기준선 업그레이드는 기능 변경과 분리해 두 C revision의 차이를 검토한다. 검증 실패를 허용하고 기능 추가를 계속하지 않는다. 이 문서와 CI 설정은 변경 시 함께 갱신한다.

## 2026-09-27 적용 검증 결과

- avx2: 1480개 중 일치 1480, 알려진 차이 0, 예상 밖 실패 0; seed 20260926.
- j64: 1480개 중 일치 1479, 알려진 차이 1, 예상 밖 실패 0; seed 20260926.
- portable-avx2: 1480개 중 일치 1480, 알려진 차이 0, 예상 밖 실패 0; seed 20260926.
- portable-j64: 1480개 중 일치 1479, 알려진 차이 1, 예상 밖 실패 0; seed 20260926.
- nightly: 8680개 중 일치 8680, 알려진 차이 0, 예상 밖 실패 0; seed 20260927.

Python 검증기 테스트 3개 통과. 이번 변경은 Rust 실행 코드 수정 없이 검증 도구·CI·문서에 적용했다. 원격 GitHub Actions 실행은 아직 확인하지 않았다.
