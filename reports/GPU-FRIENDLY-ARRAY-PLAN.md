# GPU 친화적 배열 구현 계획과 체크리스트

기준일: 2026-09-30. 사용자 요청에 따라 다른 기능 확장보다 이 계획을 최우선으로 실행한다.
전체 진행 상태는 이 문서의 체크리스트를 갱신한다. 기존 P0~P4 및 DEF 단계는 아래 작업을 직접 막는 결함 수정 외에는 후순위다.
CUDA 구현 보류, Windows 네이티브 검증, GitHub CI 생략 정책은 유지한다.

## 목표와 현재 출발점

J의 type/shape/ordered atoms와 rank/cell/frame/agreement 의미를 유지하면서 물리 저장 표현을 독립시킨다.
CPU에서 먼저 strides/offset view와 물리 계획을 검증한다. CUDA 하드웨어는 이 단계들의 선행 조건이 아니다.
현재 CpuStorage는 Inline/Owned/Shared를 지원하지만 Value가 저장소를 직접 갖고 ArrayView는 연속 CPU slice 중심이다.
Semantic IR, 보수적 primitive 계약, ValueId 기반 LogicalPlan과 rank facts는 출발점이며 완성된 physical executor가 아니다.

## 추가 조사 반영 — 2026-09-30

설계 근거와 결정 D1~D5는 [배열 프레임워크 추가 조사](ARRAY-FRAMEWORK-DESIGN-REVIEW.md)를 따른다.
조사와 문서 반영은 완료했지만 아래 구현·실측 항목은 모두 미완료다.

- [x] ndarray/Arrow API·생성/소유권 소스와 OpenXLA/Burn/ArrayFire 공식 문서를 추가 확인하고 결정·검증 사례를 기록했다.
- [ ] G1/D1: checked span·바이트 범위·dtype/backing 일치, empty/scalar 정책, stale BufferId와 read-only 기본 경계를 구현한다.
- [ ] G1~G2/D2: logical-order contiguous와 memory-contiguous를 구분하고 transpose ravel·reshape 순서 테스트를 추가한다.
- [ ] G3~G4/D3: 내부 겹침과 외부 live alias를 구분하고 부분/shared backing 회수와 조기 풀 반환을 차단한다.
- [ ] G4~G5/D4: 여러 consumer의 layout 충돌·반복 읽기·retained bytes를 계획/비용 검사에 포함한다.
- [ ] G4 이후/D5: 최소 CPU 실행 연결 뒤 제한된 JIT/fusion 실험과 bounded cache·guard·cold/warm 검증을 수행한다.

G1은 mutable arbitrary view를 제공하지 않는다. G2 첫 reshape view는 표준 논리 순서의 연속 입력으로 제한한다.
후속 JIT 실험은 G1~G4를 지연시키지 않으며 CUDA 보류도 유지한다.

## G1 — 논리 값과 물리 표현의 경계 (바로 다음 작업)

- [ ] ValueId와 별개의 BufferId 및 버퍼 소유자/등록 수명 모델을 정의한다. 논리 이름 버전과 버퍼 ID를 혼동하지 않는다.
- [ ] PhysicalArray descriptor: 버퍼 참조, shape, 원소 단위 signed strides, 원소 단위 offset, 논리 dtype를 구현한다.
- [ ] CPU 버퍼 길이/실제 정렬과 descriptor의 접근 범위를 검증한다. 정렬은 가정이 아니라 backing allocation에서 얻는다.
- [ ] rank/stride 길이 일치, 음수 stride, 0 stride, 빈 배열, scalar, checked 주소 계산을 검증한다.
- [ ] 동일 논리 값의 여러 물리 표현을 허용하며 CpuStorage를 CPU backing으로 재사용하는 adapter를 만든다.
- [ ] 단위 테스트: 유효/잘못된 descriptor, signed overflow, 빈 축, backing 경계, 공유 버퍼와 descriptor 수명.

완료 조건: CPU slice를 얻는 API는 연속성 및 CPU backing을 증명한 경우에만 제공한다.
장치 포인터를 CPU slice로 변환하는 API나 가짜 CUDA 저장소는 추가하지 않는다.
새 표현을 기존 Value에 연결하기 전부터 공개 생성 경계에서 불변식을 검사한다.

## G2 — 복사 없는 structural view

- [ ] transpose: 축과 strides 순열로 표현한다. J의 단항 transpose 의미와 기존 결과를 유지한다.
- [ ] reverse: 첫 축 offset/음수 stride로 표현한다. 빈 축에서 offset underflow가 없어야 한다.
- [ ] fill 없는 take/drop과 slice를 view로 표현한다. fill이 필요한 take는 명시적으로 materialize한다.
- [ ] 원소 순서가 보존되는 reshape만 metadata 변경을 허용한다. 반복/절단/비호환 순서 reshape는 기존 J 의미에 따라 materialize한다.
- [ ] J agreement 분석의 반복 mapping을 필요한 경우 zero stride로 낮춘다. NumPy broadcasting 규칙을 추가하지 않는다.
- [ ] logical-order materialization과 bounded 출력 할당을 구현한다.
- [ ] 테스트: 작은 배열의 모든 인덱스를 dense 기준과 비교하고 transpose→reverse→slice 조합, scalar/빈 축/높은 rank를 포함한다.
- [ ] 테스트: view 생성의 backing identity 유지와 payload 복사 없음, 원본 불변성, source scope 종료 후 공유 view 수명.

완료 조건: metadata-only 작업과 실제 출력 복사를 테스트로 구별한다.
작은 view가 큰 backing을 유지하는 비용을 기록하고, detach/materialize는 명시적으로 선택한다.

## G3 — CPU kernel 및 rank 실행 연결

- [ ] contiguous / fixed-stride / general-stride 읽기 경로를 선택하고 contiguous SIMD 경로를 유지한다.
- [ ] 덧셈을 첫 연결 연산으로 삼고 transpose/reverse view 입력을 기존 dense 결과와 비교한다.
- [ ] rank cell/frame을 물리 view mapping으로 연결한다. 논리 cell 경계와 물리 연속성을 구분한다.
- [ ] alias proof 없이는 공유/겹침/zero-stride view에 쓰지 않는다. 독점 소유·비중첩 증명 후에만 in-place/reuse를 허용한다.
- [ ] dtype 승격, 전체 결과 overflow 승격, 오류 및 평가 순서를 유지한다.
- [ ] 테스트: SIMD 경계·tail, 겹친 입력, 음수/zero stride, 정수 극값, NaN/무한대/signed zero, 빈 frame과 현재 미지원 prototype 진단.

완료 조건: 최소 연결 연산이 실제 실행 경로에서 view를 소비한다. descriptor만 추가하고 연결 완료로 표시하지 않는다.
boxed/sparse/확장 scalar는 별도 표현으로 유지하며 dense strided payload로 무조건 취급하지 않는다.
지원하지 않는 layout/type은 명시적으로 materialize하거나 기존 Rust 경로로 실행하며 C fallback을 사용하지 않는다.

## G4 — 최소 Physical Plan과 실행기

- [ ] LogicalPlan에서 버퍼 바인딩, View, Materialize, CPU kernel 호출을 명시한 최소 PhysicalPlan을 생성한다.
- [ ] 논리 ValueId와 실제 BufferId 매핑, read/write 의존성, 마지막 사용 시점 및 출력 소유권을 기록한다.
- [ ] layout-compatible view를 유지하고 contiguous kernel이 필요한 경계에서만 materialize한다.
- [ ] CPU executor가 계획에 기록된 결정을 수행하도록 연결한다. 완성된 codegen과 구별한다.
- [ ] 테스트: 계획 검사와 실제 결과 비교, 불필요한 transpose 복사 없음, 오류 경로 정리, 공유 출력 보존 및 재사용 제한.

완료 조건: 지원 subset에서 source→logical→physical→CPU 실행이 연결되고 기존 참조 경로와 비교된다.
전체 custom registry/with, 전체 언어 지원, JIT/codegen 완성을 선행 조건으로 삼지 않는다.

## G5 — 성능 판정과 확장 경계

- [ ] Windows 기본/portable 전체 회귀, fmt, Clippy를 수행하고 실행 환경·revision을 기록한다.
- [ ] contiguous 덧셈의 전후 성능, view 생성 비용, structural→산술 조합의 복사/할당/peak·retained bytes를 측정한다.
- [ ] 일반 stride 루프의 indexing 비용과 필요 시 contiguous materialization 비용을 비교한다. 큰 배열과 작은 배열을 분리한다.
- [ ] 실행 가능한 Windows C oracle이 있으면 결과·오류·성능을 비교한다. 없으면 미실행으로 기록하고 C 동등성/우위를 주장하지 않는다.
- [ ] 지원 view/타입/연산 표와 미지원 범위를 갱신한다.
- [ ] 후속 tiled layout, placement, transfer/completion을 위한 확장 경계를 문서화한다. 구현 없는 지원 상태는 추가하지 않는다.

완료 조건: 수치 결과와 메모리 불변식 검증을 먼저 통과한 뒤 성능을 평가한다.
측정한 workload 범위에서 contiguous 회귀와 복사 감소 여부를 보고한다. 회귀가 있으면 원인 및 해결을 체크리스트에 남긴다.

## 보류 항목과 재개 조건

실제 CUDA storage/kernel, stream/event, GPU 상주, sharding/multi-device는 사용자 재개 요청과 검증 환경 확보 뒤 진행한다.
함수 정의 DEF-1~6, 추가 verb/scalar 확장, custom registry/with는 G1~G5 뒤 재개한다.
단, 현재 배열 작업의 정확성을 막는 최소 계약·이름·parser 결함 수정은 필요한 범위에서 함께 처리한다.

## 체크리스트 사용 규칙

- 한 단계씩 구현하고 검증 근거가 있는 세부 항목만 [x]로 바꾼다. 계획 작성은 구현 완료가 아니다.
- 매 변경 후 변경 파일, Windows 검증 명령/결과, 활성·보류 테스트, 지원 범위와 남은 실패를 기록한다.
- 함수 정의의 기존 ignored 수용 테스트 17개는 유지하며 GPU 배열 구현의 통과 증거로 계산하지 않는다.
- Python harness 변경 시에만 해당 Windows Python 테스트도 수행한다. Linux 테스트와 GitHub CI는 실행하지 않는다.
- 바로 다음 작업은 G1 descriptor와 불변식 테스트다. 다음 단계 진입 전에 해당 완료 조건을 확인한다.

작성 시 상태: G1~G5 모두 미완료. 이번 변경은 우선순위와 완료 조건을 확정한 문서 변경이며 테스트를 재실행하지 않았다.
