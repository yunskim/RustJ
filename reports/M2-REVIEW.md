# M2 재검토 및 남은 CPU 작업

## 검토 결과와 수정

| 발견한 문제 | 조치 | 검증 |
|---|---|---|
| inline Value가 Token 전체 크기를 키워 파싱 메모리 증가 | scalar token과 boxed 배열 token 구분, 이름은 원문 차용, verb는 정적 문자열, scalar 숫자는 중간 문자열 목록 없이 파싱 | literal·rank·이름 의미 테스트와 C 차등 검증 |
| rank가 모든 셀 결과를 Vec<Value>에 보관한 후 조립 | 한 셀과 최종 결과 버퍼만 유지하는 CellBuilder 도입 | 이전 셀을 포함한 타입 승격, 빈 셀, shape 오류 테스트 |
| 정수 배열 reduction이 매 행마다 새 결과 버퍼 할당 | 차용 lhs와 단독 소유 rhs accumulator를 사용, add/sub의 역연산 overflow 복구 재사용 | 행 수 2/100 모두 accumulator 할당 1회; SIMD lane/tail 승격 검증 |
| ArrayView의 정수 변환이 Value와 달리 정수값 float를 거부 | 유한성·소수부·i64 범위 조건 일치 | NaN/무한대/극값/잘못된 index 검사 |
| 순차 조립 시 앞선 shape 오류가 뒤의 셀 계산 오류를 가릴 수 있음 | 셀 평가를 계속하고 조립 오류 보고를 지연 | 뒤의 Limit 오류가 앞선 padding 오류보다 먼저 보고되는 기존 순서 유지 |

CellBuilder는 초기 결과의 shape로 최종 원소 수를 확인하고 한 버퍼를 예약한다. 셀 타입이 나중에 올라가면 이전 결과도 함께 승격한다. 형상이 다른 셀을 padding하는 기능을 새로 지원한 것은 아니다. 메모리 할당 실패 시점까지 이전 구현과 같다고 보장하지는 않는다.

## 할당과 메모리

| 작업 | 초기 CPU 커밋 7add070 | M2 첫 구현 | 이번 재검토 후 |
|---|---:|---:|---:|
| `r =: a + 2`, 16원소 할당 횟수 | 17 | 14 | 6 |
| 같은 평가의 누적 요청 바이트 | 1,031 | 1,455 | 745 |
| 단독 소유 배열 add 추가 할당 | 4 | 0 | 0 |

메타데이터 읽기인 rank tally는 frame 수에 관계없이 최종 payload 하나만 할당한다. 이전의 셀별 Value 목록은 없다. 정수 배열의 행 방향 add/sub reduction도 승격이 없는 경우 accumulator 하나를 재사용한다. 이는 전용 메모리 풀을 도입한 결과가 아니라 불필요한 할당 자체를 제거한 결과다.

## 시간 측정

초기 CPU 커밋과 현재 구현을 같은 harness로 빌드하고 CPU 0에 고정했다. 두 차례 비교에서 버전 실행 순서를 뒤집었다. 한 항목은 7라운드 중앙값이며 마이크로초 단위다. 측정 중 다른 빌드·테스트를 실행하지 않았다. 시스템 전체의 WSL2 부하·주파수까지 통제한 것은 아니다.

| 작업 | 이전 1차 → 이후 1차 | 이전 2차 → 이후 2차 |
|---|---|---|
| 16원소 단독 소유 add | 0.129 → 0.063 | 0.129 → 0.076 |
| 4096원소 단독 소유 add | 1.477 → 1.141 | 1.465 → 1.273 |
| 4096행 rank tally | 932.823 → 227.335 | 712.658 → 164.278 |
| 4096행, 행당 8원소 rank sum | 4833.741 → 1484.094 | 4069.124 → 1490.712 |
| 파싱 포함 16원소 정수 scalar add | 0.598 → 0.425 | 0.640 → 0.419 |
| 파싱 포함 4096원소 정수 scalar add | 2.300 → 1.803 | 2.592 → 1.987 |
| 파싱 포함 100만 원소 정수 scalar add | 1355.550 → 1644.644 | 1552.643 → 1320.068 |

작은 배열·rank 경로는 개선됐지만 큰 배열은 여전히 실행 간 변동이 크다. 100만 원소의 같은 C 비교값은 1차 1753.227, 2차 1282.396이다. 모든 크기에서 C보다 빠르다고 결론내리지 않는다. 원시 기록·소스 해시는 [m2-review-measurements.json](m2-review-measurements.json)에 있다. `baseline`의 시간은 할당 wrapper를 포함하므로 위 시간 표는 uninstrumented `layout`/`comparison`만 사용했다.

재현 명령: `python3 tools/measure_m2.py --prefix m2-review`. 첫 M2 결과도 별도 파일로 보존했다.

## 정확성 검증

- rustfmt 및 Clippy(all targets, warnings as errors) 통과.
- 기본/portable 각각 21개 테스트와 수명 compile-fail 1개 통과.
- C j64avx2와 차등 검증을 439개에서 464개로 확대해 전부 통과. 전체 upstream J suite는 아니다.
- 추가 사례는 compact numeric token, rank 결과의 늦은 float 승격, 큰 cell의 add/sub overflow, 빈 cell과 문자 cell을 포함한다.

## CUDA 환경 확인과 남은 범위

현재 Windows 장치 조회에는 Intel Graphics가 표시됐고 WSL에서 `nvidia-smi` 및 통상 WSL NVIDIA 실행 파일을 찾지 못했다. 따라서 이 환경에서 CUDA 실기 실행을 검증할 수 없다. CUDA 코드나 지원 완료를 가장하는 stub을 추가하지 않았다. NVIDIA GPU가 제공되는 Linux/WSL 실행 환경에서 M3의 device storage·전송·완료 이벤트·커널을 검증해야 한다.

CPU에 남은 작업은 전용 scratch/output pool과 보유량 정책, 공유·탈출하는 slice의 backing 보유량 정책, 일반 strided view다. 현재 scope 차용과 직접 accumulator 재사용에는 별도의 persistent pool을 추가하지 않았다. 큰 배열 성능 원인 분석에는 allocator/페이지 동작과 시스템 변동을 분리한 추가 측정이 필요하다. JAXA 기반 primitive 계약·작은 Array IR는 아직 계획 단계다.

이번 변경은 M2의 확인된 메모리 증가와 중간 결과 문제를 해결한 것이며 전체 로드맵 또는 CUDA 목표의 완료를 뜻하지 않는다.
