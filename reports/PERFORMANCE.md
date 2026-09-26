# CPU 배열 최적화 기록

2026-09-26의 기존 측정 결과를 정리했다. GPU 지원은 아직 구현하지 않았다.

## 변경

정수 계산과 overflow 검출을 한 순회로 합쳤다. 공유 입력은 새 결과에 직접 계산하며, 단독 소유 add/sub 버퍼는 재사용한다. overflow가 발생한 add/sub는 wrapping 역연산으로 원래 피연산자를 복원한 뒤 실수로 승격한다. 곱셈은 새 결과 버퍼를 사용한다. AVX2 정수 add/sub·scalar multiply와 float add/sub를 추가했고 portable 경로를 유지했다.

## 측정

Intel N150, Ubuntu 24.04/WSL2, CPU 0 고정. C 참조는 커밋 `e75016ca74b5e595dd323226e6a4990172f72ec6`, GCC 13.3.0의 j64avx2 빌드다. 할당 계측 없는 동일 프로세스에서 Rust 평가기와 native JDo를 호출했다. 동일 대입 문장을 4회 준비 실행한 뒤 7라운드에서 순서를 번갈아 측정했다. 아래는 중앙값이며 단위는 마이크로초다.

| 원소 수 | 연산 | Rust 개선 전 | Rust 개선 후 | C AVX2 |
|---:|---|---:|---:|---:|
| 4,096 | 정수 scalar add | 8.466 | 2.271 | 2.403 |
| 100,000 | 정수 scalar add | 154.351 | 67.199 | 78.470 |
| 1,000,000 | 정수 scalar add | 2,864.227 | 1,267.687 | 1,243.078 |
| 1,000,000 | 정수 vector add | 3,323.148 | 1,552.043 | 1,608.965 |
| 4,000,000 | 정수 scalar add | 10,735.901 | 6,311.372 | 25,574.159 |

전체 조건·소스 해시·결과는 [comparison.json](comparison.json), 개별 실행 결과는 [개선 전](comparison-before.jsonl)과 [개선 후](comparison-after.jsonl)에 있다. 새로 성능 측정을 실행한 결과가 아니라 저장된 실행 기록이다.

## 해석과 제한

Rust 내부 개선은 크지만 C에 대한 우위는 크기·연산마다 다르다. 작은 배열과 일부 연산에서는 C가 빠르며, 모든 배열 산술에서 우위를 확보한 상태는 아니다. 시간에는 파싱·할당·대입이 포함되므로 순수 SIMD 처리량으로 해석하지 않는다. 한 CPU와 WSL2에서의 결과이며 시스템 부하와 주파수를 완전히 통제하지 않았다.

400만 원소의 큰 차이는 할당 동작의 영향을 포함한다. 준비 실행 후 `r=:a+2` 20회에서 Rust/C minor faults는 0/156,260이었다. [별도 진단](page-faults.jsonl)은 할당·페이지 관리의 영향을 시사하지만 malloc 내부의 mmap 정책을 확정하지 않는다. 일반 C 배열은 malloc/free 경로를 사용한다.

## 게시 전 검증

현재 소스에서 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo test --features portable`를 재실행했다. 기본·portable 각각 15개 테스트가 통과했다. 첫 마일스톤의 11개 테스트·362개 차등 사례는 당시 상태의 기록이며 현재와 구분한다.
