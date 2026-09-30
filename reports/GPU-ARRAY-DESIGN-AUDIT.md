# GPU 친화적 배열 설계 재검토

검토일: 2026-09-30. 기준 commit: 11fd190.
동일 검토자가 서로 다른 질문과 코드 근거로 세 차례 분리 검토한 뒤 교차 확인했다.
별도 모델/다른 사람의 독립 심사 또는 구현 테스트를 수행한 것은 아니다.

판정: 논리 의미/물리 표현 분리와 CPU부터 검증하는 방향은 적절하다.
다만 공개 API와 실행 경계의 세부 불변식이 빠져 있어 기존 계획 그대로는 구현 승인 조건이 충분하지 않다.
아래 수정은 설계 결정이며 관련 코드 구현은 아직 없다.

## 검토 A — 메모리·표현·소유권

출발 질문: 유효 descriptor가 이동·clone·풀 반환 후에도 같은 초기화된 값을 읽는가?
근거: src/storage.rs CpuStorage/Shape/ArrayView, src/value.rs Value::new/count, src/pool.rs retire, src/bit_storage.rs.

| 발견 | 구체 실패 시나리오 | 수정 결정 |
|---|---|---|
| A1: BufferId와 이동 가능한 inline 주소의 관계 미정 | registry Vec가 커져 Inline 값이 이동하면 보관된 raw pointer가 무효 | ID는 주소가 아니며 매 호출 안정된 owner에서 다시 resolve. registry 구조 변경 중 slice lease/실행 borrow는 유지하지 않음 |
| A2: dtype와 physical element encoding 구분 부족 | Bool이 u8와 packed u64를 모두 쓸 수 있는데 dtype만으로 stride byte 크기를 산출 | 첫 AffineDense는 BoolByte/Int64/Float64/Char8만 허용. 실제 encoding·initialized len 검증을 별도로 둠 |
| A3: stride-only 모델의 범위 미정 | tiled 주소와 sparse 좌표는 offset+sum(i*s) 하나로 표현되지 않음 | AffineDense를 첫 표현 종류로 한정. bit/tiled/sparse/boxed는 별도 representation mapping, 같은 descriptor로 가장하지 않음 |
| A4: 빈 배열 정책과 descriptor 수정 경계 미정 | [0,usize::MAX,usize::MAX]에서 불필요 product 계산 overflow; 검증 후 stride 필드 변경 | private descriptor와 checked 변환만 제공. empty offset=0/stride=0 canonicalization, shape 유지, 원소 주소 계산 없음 |

A4 정책: shape count는 기존 value::count처럼 먼저 0 축을 확인한다.
비어 있지 않으면 atom count와 모든 접근 span/byte 계산을 checked 한다.
길이 1 축의 stride는 0으로 정규화한다. scalar는 offset에 초기화된 원소 하나를 요구한다.
empty identity의 backing owner는 보존할 수 있으나 보유 메모리 비용에 포함한다.
validated descriptor는 backing generation/encoding/initialized length가 바뀌면 재사용할 수 없다.

A1 수명 모델: 계획 내부 BufferId는 실행 컨텍스트에 scoped하며 주소를 저장하지 않는다.
외부로 반환하는 view는 owning lease 또는 Rust borrow를 보존하고, ID 단독으로 값을 탈출시키지 않는다.
동일 shared allocation을 여러 번 등록한 ID가 달라도 alias할 수 있다. BufferId 불일치가 non-alias proof는 아니다.
allocation owner와 live leases를 바탕으로 재사용을 증명하며 첫 단계는 새 독점 출력만 허용한다.
inline scalar를 일반 view에 올리는 방법은 registry ownership 또는 borrow이고, 메타데이터 view 생성은 payload 전체 복사를 하지 않는다.

## 검토 B — J 의미·mapping

출발 질문: physical view를 유지할 때 J의 atom/cell 순서·agreement·오류가 보존되는가?
근거: src/kernels.rs agreement/ranked_dyad_ranks, src/facts.rs RankPlan, src/storage.rs cell, tests/semantics.rs.

| 발견 | 구체 실패 시나리오 | 수정 결정 |
|---|---|---|
| B1: 단순 zero stride 예시만으로 frame/cell mapping을 다 표현할 수 없음 | frame=[2]가 [2,3]에 반복될 때 cell 축을 함께 늘리면 다른 atom을 읽음 | logical agreement mapping과 physical strides를 분리하고 증명된 affine subset만 zero stride로 낮춤 |
| B2: 안전한 입력 순회와 J 오류 순회 구분 부족 | locality 때문에 cell 순서를 바꾸면 먼저 보고하는 오류가 달라짐 | 관찰 가능한 cell 평가 순서 유지. 순회 재정렬은 계약·proof가 있을 때만 |
| B3: G2 view가 실제 언어 경로에 연결되는 시점 미정 | helper에서 transpose view만 만들고 기존 evaluator는 계속 복사 | G4 완료는 source의 structural→산술 식이 view를 전달했다는 계획·복사 계측 증거를 요구 |

B1 예: scalar-cell rank에서 왼쪽 shape=[2] strides=[1], 오른쪽 shape=[2,3] strides=[3,1]이면
왼쪽은 결과 frame mapping에서 strides=[1,0]으로 읽어 [a,a,a,b,b,b]를 얻는다.
vector cell [4]를 가지는 frame=[2]가 frame=[2,3]에 반복되면 왼쪽 접근 strides는 [4,0,1]이다.
cell 축의 J agreement는 별도로 분석한다. 모든 RankMap/Gather를 하나의 affine descriptor로 축소하지 않는다.

B2/B3 첫 지원표: G1/G2/G3 AffineDense는 byte Bool, i64, f64, byte Char의 read-only 구조 변환이다.
첫 산술 연결은 기존 지원 numeric 덧셈이며 Char 산술 지원을 암시하지 않는다.
빈-frame prototype/padding 미지원은 유지하며 빈 결과라는 이유로 원래 오류/shape 추론을 건너뛰지 않는다.
reshape는 논리 순서 보존 여부를 먼저 검사하고 J의 반복/절단 semantics는 필요 시 materialize한다.
형태 변환과 계획이 metadata만 바뀌는지, 출력/JSON 경계에서 복사가 발생하는지는 각각 계측한다.

## 검토 C — compiler·GPU 실행·성능

출발 질문: 현재 inspection IR을 실행기에 연결하고 나중에 GPU를 추가해도 값·주소·완료 상태를 혼동하지 않는가?
근거: src/analysis.rs의 inspection-only 설명, src/runtime.rs analyze/prepare_semantic/commit_binding,
reports/COMPILER-ARCHITECTURE.md 9/10/15장, reports/ARRAY-FRAMEWORK-DESIGN-REVIEW.md.

| 발견 | 구체 실패 시나리오 | 수정 결정 |
|---|---|---|
| C1: inspection LogicalPlan을 재사용 실행 가능하다고 해석할 위험 | analyze 뒤 noun 재정의로 shape/version이 바뀌어 예전 facts에 새 버퍼를 연결 | G4 첫 실행은 fresh bound plan과 입력 snapshot lease로 제한. reusable plan은 guards·rebind/reanalysis가 있어야 허용 |
| C2: 여러 representation의 값 버전/유효 상태 계약 미정 | CPU와 GPU representation이 같은 ValueId여도 한쪽은 이전 binding 값 또는 미완료 결과 | 논리 값 identity와 binding version을 분리하고 immutable 값의 representation record에 encoding/layout/location/readiness를 둠 |
| C3: CPU descriptor를 그대로 GPU ABI로 사용하려는 위험 | host isize/usize·Arc/raw pointer를 device kernel 인자로 전송 | backend descriptor를 별도 생성. 고정 폭 주소/stride·범위·target encoding 검증 후 변환, Rust object layout은 ABI 아님 |
| C4: view면 GPU에서 빠르다는 가정과 index 병목 | transpose view가 불규칙 접근을 만들어 contiguous copy보다 느림 | mapping 비용, coalescing 후보, 소비 횟수, 전송량을 계획 비용에 기록. GPU 실측 전 우위 판단 금지 |

C1: 첫 실행에서 inputs는 같은 Engine 상태에서 resolve/freeze하고 결과 대입은 성공 뒤 commit한다.
입력 lease는 facts가 검증한 dtype/shape와 같아야 한다. planning/binding 진단이 실행 시 오류 우선순위를 바꾸지 않도록 검증한다.
현재 runtime은 eager binding이 우측 인자의 오류보다 먼저 실패하지 않도록 구분한다. 실행 계획 연결에서도 이 경계를 보존하며,
정확한 순서를 보장하지 못하는 미지원 부분은 기존 Rust 평가 경로로 명시적으로 처리한다. Dynamic verb는 실행 시 해석하며,
모르는 계약을 GPU/fusion 후보로 삼지 않는다. 현재 analyze의 inspection API를 그대로 execute API로 공개하지 않는다.

C2/C3는 G1에서 설계 경계만 고정한다. 실제 CUDA object/stream/event를 지금 구현하지 않는다.
출력은 실행 완료·오류 검증 후에만 visible/Ready가 된다. submission과 completion은 별개이며
cancel/drop만으로 device 작업의 버퍼를 반환하지 않는다. immutable logical value의 다른 값으로의 mutation은 새 identity를 만든다.

공식 추가 근거: [NVIDIA 비동기 실행](https://docs.nvidia.com/cuda/cuda-programming-guide/02-basics/asynchronous-execution.html),
[stream-ordered allocation](https://docs.nvidia.com/cuda/cuda-programming-guide/04-special-topics/stream-ordered-memory-allocation.html).
stream 간 dependency와 실제 작업 완료를 확인한 수명 관리가 필요하다는 근거이며 CUDA 실기 검증이 아니다.

G3 성능 결정: hot loop에서 원소마다 dtype/backend를 분기하지 않는다. 루프 진입 시 선택한다.
일반 stride 순회는 첫 correctness reference로 둔다. 고정 rank·inner contiguous·incremental cursor 특화와
metadata inline storage는 G5 실측에 따라 도입하며 매 atom div/mod와 heap metadata 비용을 따로 측정한다.

## 교차 확인과 구현 시작 조건

세 검토를 합치면 alias·값 identity·BufferId·표현 mapping·device 완료 상태를 독립적으로 관리해야 한다.
범위 검사만으로 alias를 증명하거나, 논리 값 버전만으로 GPU 완료를 증명할 수 없다.

- [x] 세 관점의 분리 검토와 구체 실패 시나리오 기록.
- [x] A1~A4/B1~B3/C1~C4의 수정 결정을 GPU 배열 계획과 아키텍처에 반영.
- [ ] G1 구현 전 BufferId scope/owner lease, closed encoding, immutable AffineDense 생성 API 확정.
- [ ] empty/singleton/scalar, zero/negative stride, overlapping backing ID, registry 성장과 lease 수명 테스트.
- [ ] 논리 agreement 사례와 view→dense 비교, 첫 오류·overflow/승격 회귀.
- [ ] 재정의 전후 snapshot, dynamic verb, eager binding과 우측 인자 오류 우선순위, 실패한 대입, 여러 physical representation 유효성 검증.
- [ ] 실언어 structural→산술 경로의 view 유지, 복사/할당·metadata 비용 계측.

최종 판정: 위 불변식을 포함한 G1 구현에 착수할 수 있다. 전체 구조의 안전성·성능·GPU 실행이 검증된 것은 아니다.
이번 변경은 문서만 수정했고 Windows 실행 테스트·C 차등·CUDA·CI를 수행하지 않았다.

## G1 구현 후 상태 (2026-09-30)

위 내용은 11fd190 설계 검토 당시 기록이다. 이후 A1~A4에 해당하는 CPU owning lease·immutable affine descriptor·encoding/empty 정책을 구현하고 [G1 Windows 검증](PHYSICAL-ARRAY-G1.md)을 완료했다. B/C의 연산·실행기·CUDA 항목은 여전히 후속이다.
