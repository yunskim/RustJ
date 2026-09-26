# Rust 대용량 배열 프로젝트를 반영한 개선안

## GPU 범위 결정

사용자 결정: 첫 GPU 백엔드는 CUDA만 지원 대상으로 한다. 현재 GPU 실행은 미구현이며, CPU/CUDA storage 분리와 실행 완료까지의 버퍼 수명 관리를 향후 설계에 포함한다. CPU 커널의 직접 slice 접근과 CUDA device buffer 접근은 별도 API로 유지하고, 전송은 명시적인 경계에서 수행한다. GPU 상주 상태의 반복 연산과 CPU↔GPU 전송을 포함한 실행을 따로 측정한다. 다른 GPU 백엔드는 현재 범위에 포함하지 않는다.

조사일: 2026-09-26. 공식 API 문서와 공개 소스의 설계를 조사했다. 아래 성능 효과는 RustJ에서 검증할 가설이며, 해당 라이브러리들을 RustJ와 직접 비교 측정한 결과는 아니다. 런타임 코드를 변경하거나 의존성을 추가하지 않고 메모리 설계를 구체화했다.

## 참고 프로젝트와 채택 범위

### Apache Arrow Rust: 버퍼 소유권과 할당 레이아웃

Arrow의 `Buffer`는 payload 복사 없이 공유·슬라이싱할 수 있다. 수정 가능한 버퍼나 Vec로 되돌릴 때는 단독 소유뿐 아니라 offset, 원래 할당의 Layout, 외부 할당 여부 등 제약을 검사한다. `MutableBuffer`의 자체 할당 경로는 캐시라인 정렬과 정렬 단위 용량을 사용한다. Vec에서 가져온 메모리까지 무조건 64바이트 정렬이라고 가정해서는 안 된다.

근거: [Buffer](https://docs.rs/arrow/latest/arrow/buffer/struct.Buffer.html), [MutableBuffer](https://docs.rs/arrow/latest/arrow/buffer/struct.MutableBuffer.html). 조회 결과 문서 버전은 각각 59.3.0, 60.0.0이며 링크는 latest이므로 변경될 수 있다. 실제 의존성 도입 시 같은 버전으로 고정하고 소스를 재검증한다.

**RustJ 결정:** `OwnedBuffer<T>`가 base pointer, initialized length, capacity, allocation Layout을 책임진다. 공유 뷰는 backing owner와 offset/length를 별도로 보존한다. `try_into_owned`는 복사 없는 변환 가능 여부를 반환하며 숨은 복사를 하지 않는다. 초기 구현에서는 단독 소유이고 전체 영역을 나타내며 할당 방식이 호환되는 경우만 소유권을 회수한다. 실패하면 결과 버퍼에 직접 계산한다.

64바이트 정렬 할당은 후보로 비교하되 기존 Vec 버퍼도 표현할 수 있게 한다. 부분 뷰의 시작 주소는 정렬되지 않을 수 있으므로 SIMD에는 비정렬 경로나 prefix/tail 처리가 필요하다. 용량 올림은 2의 거듭제곱 대신 필요한 바이트 수를 정렬 단위로 올리는 방식을 우선 시험한다.

### ndarray: 소유·공유·차용 분리와 한 번의 순회

ndarray는 `Array`, `ArcArray`, `CowArray`, `ArrayView`를 구별한다. 공유 또는 차용 상태에 대한 변경은 복사를 유발할 수 있으며, `try_into_owned_nocopy`는 복사 없는 소유권 회수를 명시적으로 시도한다. `Zip`은 여러 배열을 함께 순회하며 같은 메모리 배치일 때 벡터화에 유리하다.

근거: [ArrayBase](https://docs.rs/ndarray/latest/ndarray/struct.ArrayBase.html), [CowArray](https://docs.rs/ndarray/latest/ndarray/type.CowArray.html), [Zip 0.17.2](https://docs.rs/ndarray/0.17.2/ndarray/struct.Zip.html).

**RustJ 결정:** scope 내부 rank-cell은 `ArrayView<'a, T>`로 차용한다. 쓰기 커널은 단독 소유 버퍼의 `&mut [T]` 또는 새 결과 버퍼를 받는다. 공유 입력에 대한 단순 덧셈은 COW로 입력 전체를 복제한 다음 수정하지 않고, 새 결과에 바로 기록한다. contiguous 커널과 일반 strided 커널은 구분한다.

여러 primitive의 순회를 합치는 최적화도 후속 후보로 추가한다. 다만 J의 정수 overflow는 전체 배열의 실수 승격을 유발할 수 있다. 일부 원소만 먼저 후속 연산까지 계산하면 의미가 달라질 수 있으므로 primitive 단위 승격·오류 순서를 보존하거나 다시 계산하는 경로가 필요하다. 부동소수점 재결합, FMA 도입, 병렬 reduction 역시 자동 허용하지 않는다. 차용 뷰 도입과 연산 결합은 별도 단계다.

### faer와 dyn-stack: 재사용 가능한 작업 공간

faer의 저수준 선형대수 루틴은 `_scratch` 함수로 필요한 작업 공간을 계산하고 호출자가 `MemStack`을 제공하게 한다. 함께 살아 있는 작업 공간 요구량과 순차 재사용 가능한 요구량을 조합해 한 할당을 재사용할 수 있다.

근거: [faer linalg 메모리 할당 설명과 소스](https://docs.rs/faer/latest/src/faer/linalg/mod.rs.html).

**RustJ 결정:** 기존의 포괄적인 ‘버퍼 캐시’ 제안을 두 계층으로 나눈다.

- `ScratchScope<'a>`: 한 연산 내부의 변환·정렬·행렬 작업 등에 사용. 계산된 크기와 정렬을 만족하는 공간을 실행 컨텍스트가 제공하며, 반환값이 이를 참조할 수 없다. 순차 작업은 같은 공간을 재사용하고 동시에 살아 있는 작업은 겹치지 않는다.
- `OutputBufferPool`: 사용자에게 반환할 배열의 소유 저장소. 마지막 owner가 해제한 버퍼만 반환할 수 있으며, 총 바이트·크기별 개수·최대 보관 크기를 제한한다.

scratch에도 보유량 상한과 큰 요청 후 축소/반환 정책이 필요하다. 재귀 평가에서는 활성 scope 사이의 중첩을 보존하고, 병렬 실행에서는 worker별 공간을 둔다. boxed 값에는 destructor 처리가 필요하므로 첫 적용은 plain numeric scratch로 제한한다. 단순 덧셈처럼 작업 공간이 필요 없는 커널에는 scratch 할당을 추가하지 않는다.

### Polars: 청크 방식의 장점과 비용

Polars의 `ChunkedArray`는 새 청크를 붙여 전체 데이터의 재할당 없이 append할 수 있다. 반면 청크가 많으면 랜덤 접근과 산술에 비용이 생기고, 문서도 연산 전 rechunk의 필요성을 설명한다.

근거: [ChunkedArray 공식 문서](https://docs.pola.rs/api/rust/dev/polars_core/chunked_array/struct.ChunkedArray.html).

**RustJ 결정:** 기본 numeric storage는 단일 연속 버퍼를 유지한다. 청크 방식은 향후 대량 입력·여러 조각의 연결을 위한 builder에 한정해 비교한다. 총 길이를 알면 처음부터 한 버퍼를 할당하고, 모르거나 스트리밍 입력이면 청크로 수집한 뒤 dense 연산 진입 시 한 번만 합친다. 매 primitive마다 rechunk하지 않는다. Arrow의 null bitmap이나 표 형태를 J 기본 배열에 그대로 도입하지 않는다.

## 수정된 구현 목표

| 단계 | 적용 대상 | 완료 판단 |
|---|---|---|
| 1 | inline scalar/shape, 소유권 구분 | scalar payload/shape 할당 제거, 배열 공유 시 payload 복사 없음 |
| 2 | 차용 rank-cell과 contiguous view | 읽기 전용 셀 순회에서 셀당 heap 할당과 데이터 복사 없음 |
| 3 | 출력 직접 기록과 명시적 소유권 회수 | 유일 입력은 재사용, 공유 입력은 그대로 보존, 숨은 COW 복사 없음 |
| 4 | scratch 요구량·scope 도입 | 같은 작업 반복 시 warm-up 뒤 scratch 추가 할당 없음, 결과 탈출 없음 |
| 5 | 정렬 할당·출력 풀 실험 | 시간·요청 바이트·RSS·minor faults 동시 비교, 캐시 상한 준수 |
| 6 | 제한된 연산 결합 | J 기준선과 overflow·오류·부동소수점 의미 일치, 중간 배열 감소 |

배열 길이 0/1, SIMD lane 전후, 각 할당 크기 경계, 100만/400만 원소를 포함한다. 추가로 offset 1인 뷰, 작은 뷰가 큰 backer를 유지하는 경우, 부분 초기화 후 실패, 겹치는 입력, 공유 입력, 반복 크기 변경을 검증한다. 성능 비교는 같은 CPU·스레드 수·정수 승격 규칙·결과 소비 방식으로 수행하고 커널 자체와 파싱/할당 포함 실행을 나누어 기록한다.

외부 라이브러리를 기본 런타임 의존성으로 즉시 추가하지 않는다. RustJ의 동적 타입·rank·승격 규칙에 맞는 작은 storage 계층을 먼저 설계하고, scratch 구현에는 dyn-stack, 후속 행렬 연산에는 faer를 실제 도입 후보로 평가한다. ndarray와 Arrow는 설계 참조 및 별도 비교 구현 후보로 활용한다. 이는 라이브러리 자체의 우열 판단이 아니라 현재 J 커널의 요구에 따른 범위 결정이다.
