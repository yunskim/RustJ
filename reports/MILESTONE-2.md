# M2 배열 저장소와 차용 뷰

이 문서는 첫 M2 구현의 측정 기록이다. 이후 토큰 메모리 증가와 rank 결과 보관, reduction 누적 버퍼를 개선했다. 최신 결과와 남은 범위는 [M2 재검토](M2-REVIEW.md)를 참조한다.

기준 커밋은 `7add0706c652cbc935931db40a257dd80f5727a4`다. Linux/WSL2에서 현재 코드의 의미·할당·성능을 비교했다. CUDA backend는 아직 없다.

## 구현한 내용

- `CpuStorage<T>`: Inline, Owned(Vec), Shared(Arc<Vec>)로 구분한다. 이름에 저장할 때 `into_shared`를 적용하고 중간 결과는 단독 소유로 유지한다.
- `Shape`: rank 4까지 내부 저장하고 그 이상은 별도 저장한다. rank 0과 작은 rank를 복제할 때 heap 할당이 없다.
- `ArrayView<'a>`: shape와 typed slice를 차용하며 owner보다 오래 살 수 없다. rank-cell 생성은 입력 데이터 복사와 참조 카운트 조작을 하지 않는다.
- rank의 metadata/산술 및 reduction 경로가 차용 입력을 읽는다. 결과는 독립적인 소유 값으로 반환한다. identity/ravel 등 소유 출력이 필요한 경로의 복사와 결과 조립은 남아 있다.
- 정수/실수 scalar 산술은 inline 출력 경로를 사용한다. 단독 소유 배열의 정수 add/sub는 기존 버퍼를 유지하며 제어 블록과 shape의 추가 할당도 제거한다.
- parser는 literal 값을 토큰에서 이동한다. 공유 입력을 처리하는 혼합 수치 경로도 입력을 복제한 뒤 덮어쓰지 않고 새 출력에 직접 계산한다.
- GPU에서 사용할 수 없는 CPU slice API를 CpuStorage/CpuView에 한정했다. device pointer를 host slice처럼 취급하는 가짜 CUDA variant는 추가하지 않았다. 실제 device storage와 dispatch는 M3에서 구현한다.

공개 Rust API 변경: `Data`의 내용은 `Arc<Vec<T>>` 대신 `CpuStorage<T>`다. 소유 값의 `clone()`은 데이터 복사이며, 복사 없이 공유하려면 `value.into_shared()` 후 clone한다. J 이름 바인딩과 별칭의 외부 의미는 동일하다. `Value::new/ints`는 기존 Vec shape와 배열 형태의 shape를 모두 받는다.

## 검증

- rustfmt, Clippy 모든 target의 경고 오류 처리 통과.
- 기본 및 portable 모드 각각 기존 15개 + 저장소 2개 테스트 통과.
- owner보다 오래 사는 차용 뷰가 컴파일되지 않는 rustdoc 테스트 통과.
- C j64avx2 기준선과 439개 차등 사례 통과, 실패/제외 0. 전체 upstream J 테스트를 실행한 것은 아니다.
- 저장소 테스트는 단독 버퍼 포인터 재사용, 공유 입력 불변성, 마지막 owner 회수, view offset/bounds, 빈 셀, rank 5, 전체 셀 overflow 승격, 우측 fold를 검증한다.

## 할당 결과

`baseline` harness의 별도 계측과 저장소 테스트로 확인했다.

| 작업 | 이전 | 이후 |
|---|---:|---:|
| 단독 소유 배열에 scalar add 한 번 | 4회 / 96바이트 | 0회 / 0바이트 |
| `r =: a + 2` 전체 평가 | 17회 | 14회 |
| Value scalar 생성과 정수 add | — | 0회 |
| 차용 rank-cell 생성 | — | 0회 |

단, 파싱 포함 16원소 대입의 누적 할당 바이트는 1,031에서 1,455로 늘었다. inline 저장 때문에 Value와 이를 포함한 token/item이 커지는 비용이다. 할당 횟수 감소와 메타데이터 크기 증가는 구분해야 한다. 토큰의 compact 표현은 후속 과제이며 이번 단계에서 메모리 사용량이 모두 줄었다고 주장하지 않는다.

## 시간 비교

동일 benchmark를 이전/이후 소스에 각각 빌드했다. 빌드를 모두 마친 뒤 CPU 한 개에 고정해 두 번 측정했고 두 번째에는 버전 실행 순서를 뒤집었다. 각 시간은 7라운드 중앙값이다. 아래 단위는 마이크로초다.

| 작업 | 이전 1차 → 이후 1차 | 이전 2차 → 이후 2차 |
|---|---|---|
| scalar integer add | 0.158 → 0.083 | 0.141 → 0.083 |
| 16원소 단독 소유 add | 0.133 → 0.066 | 0.109 → 0.066 |
| 4096원소 단독 소유 add | 1.527 → 1.264 | 1.202 → 1.126 |
| 4096행 rank tally | 842.279 → 255.758 | 820.022 → 157.648 |
| 4096행, 행당 8원소 rank sum | 4063.598 → 1907.104 | 5557.252 → 2021.704 |
| 100만 원소 단독 소유 add | 948.994 → 1160.670 | 802.174 → 760.719 |

작은 값과 rank 경로는 두 측정 모두 개선됐다. 큰 배열은 변동이 크고 회귀처럼 보이는 측정도 있어, 전반적인 속도 향상이나 C 대비 우위를 선언하지 않는다. 파싱 포함 4096원소 정수 scalar add도 1차에는 개선(2.390→1.784), 2차에는 악화(2.540→3.157)되어 추가 통제가 필요하다. CPU의 전체 성능 목표는 아직 미완료다.

원시 결과와 소스·harness 해시는 [m2-measurements.json](m2-measurements.json)에 있다. 시간 비교의 `layout`/`comparison`은 할당 계측이 없고, `baseline`의 시간은 계측 wrapper가 있어 주 성능 판정에 사용하지 않았다. 1차 실행 중 참고 저장소의 clone·문서 읽기가 있었으므로 두 측정 모두 완전히 유휴 상태였다고 주장하지 않는다. WSL2의 시스템 부하·주파수 변동을 완전히 통제하지 않았다.

재현: `python3 tools/measure_m2.py`. pinned j64avx2 참조 라이브러리가 필요하다. 도구는 이전 커밋의 별도 worktree에 동일한 layout harness를 추가하고 순차 측정한다.

## 남은 작업

M2의 저장소·inline·차용 뷰 핵심은 구현했지만, 실제 scratch allocator/output pool과 그 상한, 큰 backing buffer를 유지하는 뷰의 자동 실체화는 미구현이다. 현재 view는 scope 차용만 제공하며 공유·탈출 가능한 slice storage도 후속 설계다. compact token, 일반 strided view, rank 결과 직접 조립, CPU 병렬 실행도 남아 있다.

JAXA 검토에서 제안한 논리 Array IR와 물리 Memory Plan 분리는 [별도 검토](JAXA-REVIEW.md)에 기록했다. 이는 현재 CpuStorage 구현과 구별되는 후속 compiler 계층이며, M2 완료를 위해 전체 최적화 compiler를 구현했다는 의미가 아니다.
