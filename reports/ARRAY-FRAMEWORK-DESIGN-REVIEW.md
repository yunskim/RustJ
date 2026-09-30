# 배열 프레임워크 추가 조사와 RustJ 설계 결정

조사일: 2026-09-30. 공식 API 문서와 공개 소스 일부를 확인했다.
이 보고서는 설계 근거이며 RustJ 구현·benchmark 또는 CUDA 실행 완료 증거가 아니다.
ndarray는 0.17.2를 확인했다. Arrow latest 및 웹 문서는 조회일 기준이며 의존성 도입 전 버전/commit을 고정해야 한다.
전체 repository 정밀 감사나 라이브러리 간 실측 비교는 아직 수행하지 않았다.

## 근거와 채택 범위

| 참고 대상·확인 자료 | 확인한 원칙 | RustJ에서 적용할 위치 |
|---|---|---|
| [ndarray ArrayBase 0.17.2](https://docs.rs/ndarray/0.17.2/ndarray/struct.ArrayBase.html) | 논리 순서의 slice와 메모리 순서의 slice가 구분된다. reshape API도 순서를 명시한다. | G1 slice API, G2 reshape |
| [ndarray view 생성 소스](https://docs.rs/ndarray/0.17.2/src/ndarray/impl_views/constructors.rs.html) | 안전한 slice 기반 생성은 접근 범위를 확인한다. raw-pointer 생성은 allocation·수명·정렬·주소 범위에 대한 호출자 증명이 필요하다. | G1 checked constructor |
| [Arrow Buffer API](https://docs.rs/arrow-buffer/latest/arrow_buffer/buffer/struct.Buffer.html), [소스](https://docs.rs/arrow-buffer/latest/src/arrow_buffer/buffer/immutable.rs.html) | 공유/offset과 할당 Layout은 소유권 회수의 제약이다. | G1 backing owner, G3 reuse |
| [OpenXLA HLO→Thunks](https://openxla.org/xla/hlo_to_thunks) | layout 충돌은 copy로 해소하며 dataflow·alias·수명을 바탕으로 실제 buffer slice를 배정한다. | G4 physical allocation |
| [OpenXLA GPU architecture](https://openxla.org/xla/gpu_architecture) | backend는 library 호출과 코드 생성을 선택한다. Triton은 일부 fusion의 codegen 경로다. | 후속 CUDA lowering 경계 |
| [Burn fusion](https://burn.dev/books/burn/performance/good-practices/kernel-fusion.html) | view indexing과 살아 있는 값은 fusion/vectorization에 영향을 준다. read/write 경계에서 elementwise 연산을 결합한다. | G4/G5 layout 비용·후속 fusion |
| [ArrayFire JIT](https://arrayfire.org/docs/jit.htm) | 지원 연산을 그래프로 모으고 평가 경계에서 kernel을 생성한다. | 후속 JIT 실험 |

NumPy/CuPy, PyTorch/JAX, GPUArrays, faer/dyn-stack, Polars에 대한 기존 조사도 유지한다.
이 보고서는 위 표의 자료를 추가 확인했으며 나머지 framework 소스를 새로 감사했다고 주장하지 않는다.

## D1 — 주소 범위 검증과 읽기/쓰기 분리 (G1)

초기 dense CPU descriptor는 typed initialized backing과 checked constructor로 만든다.
raw pointer를 공개 안전 API의 입력으로 받지 않는다. unsafe가 필요한 최종 kernel 경계는 별도 검증한다.

RustJ 결정: 비어 있지 않은 배열에 대해 각 축의 delta=(length-1)*stride를 checked 계산하고,
lo=offset+sum(min(delta,0)), hi=offset+sum(max(delta,0))가 backing 안에 있는지 확인한다.
원소→바이트 변환과 signed 연산도 checked로 수행한다. span 검증은 접근 가능성을 증명하며 비중첩을 증명하지 않는다.
빈 배열은 원소 주소를 계산하거나 포인터를 이동하지 않고 offset 정책을 별도로 정한다.
scalar는 원소 하나를 요구한다. length=1 축의 stride와 모든 축의 shape product overflow 정책을 명시한다.
ndarray raw constructor의 조건을 RustJ signed stride API에 그대로 복제하지 않는다.

공유·zero-stride·겹침 view는 read-only가 기본이다. 첫 mutable 경로는 독점 소유한 표준 연속 출력으로 제한한다.
주어진 shape/strides가 내부적으로 겹칠 수 있으므로 Arc 단독 소유만으로 일반 view 쓰기를 허용하지 않는다.
BufferId는 registry scope 또는 generation으로 stale ID 접근을 막는다. view는 owner 또는 실행 scope를 보존한다.

## D2 — 논리 순서와 메모리 연속성 (G1/G2)

RustJ 결정: standard logical-order contiguous와 memory-contiguous를 별도 판정한다.
기존 dense kernel에 넘기는 slice는 논리 순서가 일치한 경우만 허용한다.
다른 메모리 순서의 연속 slice를 사용할 경우 같은 index mapping이라는 별도 proof가 필요하다.
예: [2,3]의 transpose [3,2]는 backing이 하나여도 J ravel 순서는 0,3,1,4,2,5다.

첫 reshape view는 동일 atom count와 J ordered-atom 순서를 보존하는 표준 연속 입력에 한정한다.
비연속 stride의 축 병합/분할은 이후 별도 proof와 테스트로 확장한다.
J reshape의 반복·절단·fill 규칙은 ndarray same-size reshape와 구분한다.

## D3 — 버퍼 재사용은 명시적으로 증명 (G1/G3/G4)

RustJ 결정: 소유권 회수는 원래 할당 방식, 전체 영역, offset, 공유 owner, live view를 확인한다.
실패 시 숨은 COW 복사 대신 새 출력에 직접 기록한다. Vec에서 받은 메모리에 64-byte 정렬을 가정하지 않는다.
초기 backing은 기존 CpuStorage를 사용하며 custom aligned allocator는 실측 후 판단한다.
풀에 반환 가능한 마지막 사용 시점은 모든 alias/view를 포함한다. 마지막 logical ValueId 사용만으로 반환하지 않는다.
동기 CPU에서는 실제 실행 종료가 기준이며 CUDA 재개 후에는 제출 완료와 device 작업 완료를 구분한다.

## D4 — 작은 물리 계획과 비용 기록 (G4/G5)

RustJ 결정: 먼저 Buffer binding/View/Materialize/Kernel call/Output을 명시한다.
view 유지와 contiguous copy의 비교에는 consumer 수, 반복 읽기, index 비용, 복사 bytes와 backing retained bytes를 포함한다.
작은 slice가 큰 버퍼를 보존하는 경우는 detach 후보로 기록한다. 임의 threshold는 benchmark 전 확정하지 않는다.
여러 consumer가 서로 다른 layout을 요구할 수 있으며 논리 값 하나에 단일 물리 layout을 강제하지 않는다.
첫 reuse는 dtype/용량/정렬 호환과 겹치지 않는 수명 proof가 있는 버퍼로 제한한다.
XLA 전체 IR나 allocation arena를 처음부터 복제하지 않는다.

## D5 — JIT/fusion 확장 준비 (G4 이후 실험)

RustJ 결정: G1~G4 완료의 선행 조건으로 JIT를 넣지 않는다. 최소 물리 실행이 연결되면 작은 CPU 실험을 한다.
첫 후보는 f64 elementwise chain이며 integer 전체 결과 승격, 오류 순서, effect는 barrier로 보존한다.
부동소수점 연산 순서와 rounding을 유지하고 FMA/reassociation은 자동 허용하지 않는다.
CPU JIT는 contiguous/fixed/general stride별 특화와 bounded cache를 평가한다.
key에는 graph·semantic contract/version·dtype·rank/layout class·target capability를 포함하고 length는 가능하면 runtime 인자로 둔다.
변경 가능한 verb binding은 guard 또는 cache 무효화가 필요하며 raw buffer 주소는 cache identity가 아니다.
정확성, cold compile 비용, warm 실행, cache hit/miss·eviction과 총 코드 메모리를 별도로 측정한다.
단일 대형 덧셈은 대역폭 제한일 수 있어 JIT 우위를 가정하지 않는다.

CUDA 재개 시 Physical Plan 이후 library/backend codegen 경계를 유지한다.
Burn/CubeCL·Triton/XLA는 참고 후보이며 RustJ dependency/backend로 채택 확정하지 않는다.
NVIDIA library 사용 여부 및 Rust kernel 작성 목표와의 관계는 재개 시 별도 결정한다.

## 추가 검증 매트릭스 (아직 미실행)

- descriptor: scalar/empty/singleton/high rank, signed arithmetic·byte span overflow, dtype mismatch, stale BufferId.
- 주소 mapping: [2,3] transpose, reverse, offset slice, zero stride, [2,2] strides=[1,1] 내부 겹침.
- 논리 결과: transpose ravel 순서, view→reshape 거절/materialize, 여러 view 조합을 dense 인덱스 oracle와 비교.
- 소유권: source 해제 이후 owner 보존, 공유·부분 영역 회수 거절, live alias 중 풀 재사용 금지, 오류 경로 정리.
- 계획/비용: diamond graph의 여러 consumer, layout 충돌, shared output, 반복 읽기, tiny slice의 retained bytes.
- JIT 후속: guard 실패·이름 재정의·cache eviction, f64 bitwise 결과 및 J 오류/승격 barrier.

조회에서 일부 version URL은 실패해 Arrow latest 및 ndarray 실제 소스 링크로 확인했다.
모든 framework의 모든 안전성/성능 조건을 확인한 것은 아니다. 구현 시 선택 API의 버전 고정과 추가 소스 확인을 계속한다.

## 후속 설계 검토

[세 관점 설계 검토](GPU-ARRAY-DESIGN-AUDIT.md)에서 D1~D5의 미정 정책을 구체화했다. empty/singleton 정규화, affine mapping 범위와 encoding 분리, owner lease, fresh-plan 실행과 GPU ABI 경계는 해당 수정 결정을 따른다. 구현 검증은 아직 미완료다.
