# RustJ 구현 계획

현재 작업 순서·완료 조건·확인 상태는 [실행 체크리스트](IMPLEMENTATION-CHECKLIST.md)를 따른다 (2026-09-28). GitHub CI는 생략하고 로컬에서 검증한다. CUDA 구현은 재개 요청까지 보류한다.

## 채택된 목표 — compiler-first RustJ

2026-09-27부터 RustJ의 최종 목표를 **독립 Rust J 인터프리터**가 아니라 다음 compiler architecture로 수정한다.

```text
J Source
  → Parser
  → Semantic IR
  → Jaxa Analyzer
  → Logical Execution Plan
  → Logical Optimizer
  → Physical Planner / Optimizer
  → Physical Execution Plan
  → Backend Lowering / Code Generation
  → Runtime / Executor
```

CPU와 GPU는 같은 Logical Plan에서 출발하는 동등한 backend다. 현재 직접 평가형 CPU 엔진은 폐기하지 않고 semantic reference, differential oracle, 기존 CPU kernel 자산, 명시적 fallback의 기반으로 유지하면서 compiler pipeline을 옆에 세운다.

전체 설계 기준은 [RustJ 컴파일러 아키텍처](COMPILER-ARCHITECTURE.md)를 따른다.

### Jaxa Analyzer의 책임

Jaxa Analyzer는 Semantic IR을 **해석해서 Logical Execution Plan을 만드는 계층**이다. 실행하지 않는다.

주요 출력 정보:

- rank / cell / frame / agreement
- dtype / shape
- primitive contract
- dependency / effect / alias
- parallel domain
- map / reduce / scan / gather / structural 분류
- 합법적인 rewrite/fusion 후보
- semantic materialization boundary
- JAXA custom primitive 및 `with` conjunction binding

Physical Planner는 이 Logical Plan을 받아 device, physical layout, strides/view, tiling, fusion group, transfer, buffer reuse, work partition을 결정한다. Codegen은 이를 backend artifact로 만들고 Executor는 정해진 계획을 수행한다.

### GPU-friendly 배열에 대한 확정 결정

J noun의 논리 모델은 계속 **type + shape + ordered atoms/value**다. GPU를 위해 JArray에 physical 속성을 추가하지 않는다.

다음은 physical representation/planning에 둔다.

- storage / BufferId
- strides
- offset
- dense/tiled layout
- alignment
- CPU/GPU placement
- sharding
- device transfer
- stream/event/completion
- GPU block/thread/workgroup mapping

현재의 `CpuStorage`와 contiguous `ArrayView`는 CPU physical backend의 출발점으로 본다. 목표 `ArrayView`는 `storage + shape + strides + offset`을 표현할 수 있도록 일반화한다.

영향을 받은 결정:

- NumPy: arbitrary strided view
- CuPy: strided array representation의 GPU 적용
- PyTorch: logical dimensions와 memory format/layout 분리
- JAX: global logical array와 placement/sharding/local layout 분리
- ArrayFire: lazy graph와 안전한 kernel fusion
- Julia GPUArrays: backend abstraction
- J: rank/cell/frame/agreement를 parallel domain의 semantic source로 사용

가져오지 않는 결정:

- NumPy broadcasting으로 J agreement 대체
- PyTorch/JAX Tensor semantics로 J noun 재정의
- CUDA-specific 정보를 JArray/primitive semantics에 삽입
- 모든 배열을 tiled storage로 고정
- GPU 성능을 위해 J의 overflow/promotion/error/floating semantics 변경

### 현재 실행 순서 — 2026-09-30 변경

사용자 요청에 따라 GPU 친화적 배열 설계를 다른 기능 확장보다 우선한다.
기존 Semantic IR/LogicalPlan을 출발점으로 다음 순서를 실행한다.

1. G1: 논리 ValueId/물리 BufferId 분리와 checked PhysicalArray descriptor.
2. G2: strides/offset 기반 structural view와 명시적 materialization.
3. G3: CPU kernel·rank 실행 연결과 contiguous SIMD 경로 유지.
4. G4: 최소 PhysicalPlan과 CPU executor 연결.
5. G5: Windows correctness·성능·복사/할당 비용 검증.
6. 이후 함수 정의와 추가 언어 기능, registry/with, 전체 codegen/JIT 확장을 재개한다.

세부 실행 및 완료 판정은 [GPU 배열 계획·체크리스트](GPU-FRIENDLY-ARRAY-PLAN.md)를 따른다.
실제 CUDA 구현은 계속 보류한다. 이 컴퓨터에서는 Windows 네이티브 검증만 수행하고 GitHub CI는 생략한다.
이 순서가 아래의 과거 마일스톤 순서보다 우선하며, 배열 구현을 막는 최소 결함 수정만 병행한다.

## 주요 목표와 현재 상태

1. C J 커널을 실행 fallback으로 사용하지 않는 독립 Rust J compiler/runtime를 만든다.
2. J의 rank/cell/frame/agreement와 타입·shape·오류·승격 의미를 Semantic IR/Jaxa Analyzer에서 보존한다.
3. 분석, 논리 계획, 물리 계획, codegen, execution을 분리한다.
4. CPU 배열 연산에서 복사·할당·중간 배열을 줄이고 SIMD와 소유권 기반 재사용으로 성능을 확보한다.
5. GPU에서는 동일 Logical Plan을 바탕으로 device placement, strided view, layout/tiling, fusion, transfer를 계획한다.
6. CPU와 GPU에서 지원되는 J 의미를 차등 검증하고, backend 미지원과 언어 미구현을 구별한다.

현재 구현은 제한된 CPU 직접 평가 엔진과 portable/AVX2 커널이다. M2의 inline scalar/shape, 단독·공유 CPU storage, 차용 rank-cell, compact token, rank 결과 순차 조립, 정수 add/sub reduction accumulator 재사용과 bounded output pool 등이 구현되어 있다. 이 구현 상태를 compiler architecture가 이미 완료되었다는 뜻으로 해석하지 않는다.

## M2 — CPU/CUDA를 수용하는 배열 기반

- 논리적 Array와 실제 Storage를 분리한다. CPU의 inline scalar, 단독·공유 dense buffer와 향후 CUDA device storage를 구분한다.
- 낮은 rank의 shape는 inline으로, scope 내부 rank-cell은 차용 뷰로 표현한다.
- CPU 커널은 typed slice를 사용한다. Device storage에는 CPU slice API를 제공하지 않는다. GPU 커널 역시 장치 주소로 직접 접근할 수 있다.
- 복사 없는 소유권 회수와 명시적 복사를 구분한다. 버퍼의 할당 Layout과 backing owner를 보존한다.
- 작업용 scratch와 반환용 output pool을 분리한다. 참조 카운트뿐 아니라 진행 중인 작업의 사용 여부도 반환 조건에 포함할 수 있도록 설계한다.
- C의 compact 배열 구조는 비교 대상으로 삼되 동일한 ABI·헤더·할당 정책은 요구하지 않는다. 수치 출력은 초기화된 범위를 추적해 불필요한 0 초기화를 피하고, 공유 입력은 먼저 복사하지 않고 새 출력에 직접 계산한다.
- 고정 크기 결과는 필요한 용량과 정렬 중심으로 할당하고, 성장하는 builder는 별도의 용량 증가 정책을 둔다. 기본 수치 배열은 연속 메모리를 유지한다.
- CPU scratch와 output pool에도 보유 바이트 상한과 큰 블록 반환 정책을 적용한다. 작은 뷰가 큰 backing buffer를 유지하는 경우 복사·실체화 비용과 retained bytes를 비교한다.

완료 기준: 기존 CPU 의미 검증 통과, 공유 입력 불변성·오류 경로·뷰 수명 검증, 작은 값과 rank-cell의 할당 감소 측정. GPU 지원을 위한 추상화 때문에 CPU inner loop에 원소별 backend 분기가 생기지 않아야 한다.

## M3 — 첫 CUDA 실행 (구현 보류)

- CUDA 도구는 CubeCL의 CUDA 경로와 Rust 커널 컴파일 도구를 소규모 실험으로 평가한다. cudarc는 호스트 제어 후보이며 그 자체가 Rust GPU 커널 컴파일러는 아니다. i64/f64, overflow 검출, Linux 빌드·배포 조건을 확인해 선택하고 버전을 고정한다.
- 선택적 CUDA 빌드, 장치 탐색, device buffer, CPU↔GPU 명시적 전송, stream/event와 완료 후 해제를 구현한다. 실제 사용자 API 이름은 이 단계에서 확정한다.
- 첫 연산은 연속 i64/f64 배열의 동일 shape 덧셈과 scalar 확장으로 한정한다. 타입 지원 여부를 장치와 도구 양쪽에서 검사하며 조용히 f32로 축소하지 않는다.
- i64 overflow는 전체 결과의 f64 승격으로 처리한다. 첫 구현은 원본 입력을 보존하고 overflow flag 확인 후 필요하면 다시 계산한다. 이 경로의 동기화 비용도 기록한다.
- GPU 접근 실패와 실행 오류를 RustJ 오류로 전달한다. 명시적인 GPU 요청에서 조용히 CPU로 실행하지 않는다.
- JAXA 검토를 반영해 첫 primitive부터 dtype·shape·rank·오류/승격·effect·alias·CPU/CUDA 지원 계약을 명시한다. 하드웨어 수치와 backend 선택은 J 문법 및 parser 밖에서 관리한다.

완료 기준: 실제 NVIDIA GPU의 Linux 환경에서 CPU RustJ 및 지원 범위의 C 기준선과 비교. 0/1 원소, SIMD/블록 경계 길이, 큰 배열, 정수 극값·승격, 공유 입력, 장치 메모리 부족 및 전송 실패를 검증한다. 하드웨어를 확보하지 못하면 CUDA 실행 검증은 미완료로 기록한다.

## M4 — GPU 상주와 비동기 수명 관리 (구현 보류)

- GPU에서 생성한 결과를 다음 GPU 연산에 바로 전달하고, 출력·CPU 연산 진입 등 필요한 경계에서만 다운로드한다.
- device ID, 데이터 위치, pending completion을 추적한다. host/device 복사본을 함께 보관한다면 어느 쪽이 최신인지 명확히 관리한다.
- stream 순서와 event 의존성을 보존한다. 작업 제출 후 마지막 사용자 참조가 없어져도 실행 완료 전에는 버퍼를 해제하거나 풀에서 재사용하지 않는다.
- 대기 중인 작업은 개수와 총 바이트를 제한한다. 취소는 즉시 메모리를 반환한다는 뜻이 아니며 실제 완료와 정리까지 추적한다.
- 장치별 output pool과 scratch에 메모리 상한을 둔다. 완료된 작업이 사용하던 유일 소유 버퍼만 재사용한다.
- 외부 async API는 완료 통지를 제공할 수 있지만 CPU 수치 커널을 async로 바꾸지는 않는다. pinned host staging은 전송 중첩 효과와 메모리 비용을 측정한 뒤 한도 내에서 적용한다.
- 작은 Array IR의 ValueId와 물리 Memory Plan의 BufferId를 구분한다. 논리 값마다 즉시 배열을 할당하지 않고, 마지막 사용·alias·관찰 지점·완료 이벤트에 따라 materialization과 재사용을 결정한다. dynamic/unknown 의미를 추측해 최적화하지 않는다.

완료 기준: 연속 연산 사이 불필요한 CPU 왕복 없음, 진행 중 해제·취소·오류에서 use-after-free 없음, 긴 반복 실행에서도 보유 메모리 상한 준수. 작은 뷰가 큰 GPU backer를 유지하는 비용도 기록한다.

## M5 — 의미 확장과 실행 선택

- CPU의 reduction·rank 중간 배열을 줄이고 빈 frame prototype, dyadic rank, 결과 padding을 확장한다.
- GPU는 곱셈·비교·지원되는 rank 연산부터 확대한다. reduction과 행렬 연산은 별도로 의미와 수치 정확도를 검증한다.
- 부동소수점 reduction의 순서 변경, FMA, 연산 재결합을 자동으로 허용하지 않는다. 엄격한 의미를 지킬 수 없는 연산은 명시된 CPU 경로를 사용하고 필요한 전송 비용을 공개한다.
- 연산 결합은 primitive별 정수 승격, 오류 순서, 입력 별칭 의미를 보존하는 경우에만 적용한다.
- graph·schedule·device를 분리하고, 결합 비용은 primitive별 숫자의 단순 합으로 계산하지 않는다. 초기에는 전송량·중간 배열량·launch 수·peak live bytes를 측정하며 register/occupancy 예측은 실측으로 검증한다.
- 초기 실행 선택은 명시적으로 한다. 자동 선택은 배열 크기뿐 아니라 현재 데이터 위치, 후속 연산, 전송량과 실제 장치 측정을 반영한 뒤 도입한다.
- verb·수정자·함수·boxed 지원도 계속 확장한다. boxed 등 일반 객체 그래프 전체를 첫 GPU 지원 범위로 삼지 않는다.

완료 기준: 지원 연산·타입·shape별 CPU/GPU 표와 제외 사유를 공개하고 차등 검증을 통과한다. GPU에 맞지 않는 연산도 독립 Rust CPU 엔진에서 처리하며 C J 커널로 fallback하지 않는다.

## 성능·배포 검증

동일 workload를 CPU SIMD, 전송 포함 CUDA, GPU 상주 CUDA의 세 조건으로 비교한다. 커널 컴파일·초기화의 cold 비용과 warm 실행을 분리한다. GPU 시간 측정은 제출 시간만 재지 않고 실제 완료를 포함한다.

측정 항목은 wall time, kernel time, 전송 바이트·횟수, allocation 수, host RSS, device peak/retained memory다. 단일 덧셈·연속 산술·타입 승격·행렬 연산을 분리하고 작은 배열부터 장치 용량 내 큰 배열까지 측정한다. 특정 GPU에서의 결과를 모든 GPU의 성능으로 일반화하지 않는다.

CPU CI는 GPU 없이 계속 실행한다. CUDA 빌드 검사와 실제 GPU 실행 검사를 구분하고, GPU 검증 환경에는 GPU 모델, driver/toolkit 및 도구 버전, 실행 옵션을 기록한다. CUDA를 켜지 않은 배포물에는 CUDA 설치를 요구하지 않는다.

참고 설계: [C 메모리 정책](MEMORY-POLICY.md), [Rust 배열 프로젝트 조사](RUST-ARRAY-REFERENCES.md). GPU는 기존 CPU 최적화를 대체하는 목표가 아니라 함께 검증할 주요 실행 경로다.

## 대화에서 합의한 요구 조건과 반영 위치

2026-09-26 보완. CUDA 주요 목표 추가와 별도로, 이전 대화의 요구를 이 계획의 추적 항목으로 통합했다. 아래 ‘확정 요구’는 사용자 요청이며 ‘설계 방향’은 이를 실현할 방법이다. 도구 이름이나 특정 구현 후보를 채택 확정으로 해석하지 않는다.

| 구분 | 조건 | 반영·완료 확인 |
|---|---|---|
| 확정 요구 | 처음부터 작성하는 독립 Rust 엔진. 최종적으로 C J 커널 없이 실행 | 모든 마일스톤에서 C 엔진 fallback 금지. C는 별도 검증·성능 기준선으로 사용 |
| 확정 요구 | C의 내부 구조·메모리 관리 정책을 그대로 따를 필요 없음. 성능을 위해 알고리즘·표현 변경 가능 | M2에서 compact layout, 소유권, 할당 정책을 측정해 결정. 외부 J 의미와 내부 구현을 구분 |
| 확정 요구 | 단순 배열 연산의 성능을 핵심 성공 기준으로 취급 | 아래 CPU 성능 판정 적용. GPU 가속 결과로 CPU 성능 목표를 대신 충족했다고 판단하지 않음 |
| 확정 요구 | C 소스의 구현 아이디어와 실제 메모리 할당 정책을 근거로 개선 | 고정 C 커밋의 풀·뷰·재사용·참조 카운트와 Rust 경로를 비교하고 근거 문서 유지 |
| 확정 요구 | Rust 대용량 배열 프로젝트의 방법을 조사해 개선안에 적용 | Arrow의 storage 회수 조건, ndarray의 차용 뷰, faer의 scratch, Polars의 청크 비용을 M2~M5에 반영 |
| 확정 요구 | 지정 저장소를 사용 | 작업 저장소는 `git@github.com:yunskim/RustJ.git`. 구현·계획·검증 도구를 같은 저장소에서 관리 |
| 확정 요구 | Linux 우선, 첫 GPU 백엔드는 CUDA | Linux CPU 실행 유지, M3~M5는 실제 NVIDIA GPU에서 검증 |
| 설계 방향 | C와 비슷한 효율적인 Array 구조를 검토 | inline scalar/shape와 단독·공유 storage 분리부터 적용. 가변 길이 단일 할당 헤더는 측정 후보 |
| 설계 방향 | async가 유효한 경계에 활용 | 아래 동시 실행 정책과 M4 적용. 전체 엔진의 async 전환은 요구하지 않음 |

외부 라이브러리는 설계 참조와 실제 의존성 도입을 구분한다. ndarray·Arrow·Polars 전체를 즉시 런타임에 포함하는 것은 요구 사항이 아니다. dyn-stack·faer·CUDA 도구도 의미 호환, 성능, 빌드·배포 조건과 라이선스를 확인한 뒤 선택한다.

## CPU 성능 성공 기준 보완

단순 배열 산술에서 C 기준선에 대한 경쟁력과 측정 가능한 우위를 확보하는 것을 핵심 목표로 한다. 모든 크기·연산에서 우위를 보장한다고 미리 선언하지 않는다. 대표 workload에서 우위가 없거나 회귀가 남으면 해당 성능 목표는 미달성으로 기록하고 병목과 후속 조치를 남긴다.

- 비교 대상은 같은 하드웨어의 최적화된 C J 빌드다. 버전·컴파일 옵션·CPU 기능·스레드 수를 기록한다. 변경한 C 변형을 실험한다면 원본 기준선과 별도로 표시한다.
- scalar 확장과 배열끼리의 덧셈·뺄셈·곱셈을 작은 배열, 캐시 내 배열, 메모리 대역폭에 제한되는 큰 배열에서 비교한다. 유일 소유·공유 입력, 새 출력·버퍼 재사용을 구분한다.
- 파싱 포함 실행과 커널 자체를 별도로 측정한다. 할당 계측은 시간 측정과 분리하고 warm-up, 반복 분포, 결과 소비를 명시한다. 작은 차이가 측정 변동 범위에 있으면 우위라고 판정하지 않는다.
- 시간 외에 복사량·요청 용량·minor faults·peak/retained RSS를 기록한다. 할당 경계 전후, 100만/400만 원소, 반복 크기 변경을 포함한다.
- 페이지 폴트만으로 malloc 내부 정책이나 mmap 호출을 확정하지 않는다. 원인 주장은 소스 경로와 필요시 allocator/syscall 추적으로 확인한다.

## 동시 실행과 async의 적용 범위

CPU 계산은 동기 커널을 유지하고, CPU 병렬화는 별도의 worker 실행 정책으로 다룬다. 배열마다 무조건 병렬화하지 않고 크기·작업량 임계값을 측정한다. 스레드 간 이동·공유와 스레드별 풀의 반환 규칙을 먼저 정의하고, reduction의 순서·오류·승격 의미를 검증한다. M5의 실행 선택에 CPU 단일 스레드·CPU 병렬·CUDA를 단계적으로 포함한다.

async는 CUDA 완료 통지와 향후 파일·네트워크 입력 및 서버 요청 처리의 후보로 둔다. 파일·네트워크·서버 자체를 현재 마일스톤의 필수 구현으로 확대하지 않는다. 관련 기능 도입 시 입력→계산→출력 파이프라인에서 독립적으로 처리 가능한 블록만 중첩하고 작업 개수와 총 바이트에 backpressure를 적용한다.

C의 작업 큐·PYX 결과·오류 전파는 설계 참조로 사용한다. futex 대기 같은 blocking 호출은 async 실행 스레드에서 직접 수행하지 않는다. GPU 완료나 blocking I/O 대기에는 완료 통지 또는 별도 blocking worker 경계를 사용한다. CPU/GPU 모두 실행 중인 작업의 버퍼는 취소 요청만으로 회수하지 않는다.

## 진행 상태와 문서 관리

현재까지의 구현·측정 결과와 계획을 구분한다. GPU 실행, 메모리 재설계, CPU 병렬화 또는 async API를 문서에 추가한 사실만으로 지원 완료라고 표시하지 않는다. 마일스톤마다 지원 범위, 검증 환경, 통과·실패·미실행 항목과 성능 결과를 기록한다. 저장소에 연결했다는 사실과 커밋·원격 게시 여부도 별도로 보고한다.

## JAXA 설계 검토의 적용 범위

[jaxa-analyzer 검토](JAXA-REVIEW.md)의 primitive 계약과 의미/실행 분리를 M3~M5에 반영한다. 최소 계약과 작은 연산 그래프부터 구현하며 전체 compiler 완성을 첫 CUDA 커널의 선행 조건으로 만들지는 않는다.

후속 frontend는 Token/Span, 분류와 이름 해석, J 의미 AST, Array IR 진입 검증의 책임을 나눈다. GPU에서 분석·실행 가능한 영역의 제한을 RustJ 전체 언어의 제한으로 혼동하지 않는다. 지원되지 않는 backend와 미구현 J 기능은 다른 진단으로 남긴다.

JAXA의 fp32 우선 정책이나 자동 미분·학습 엔진 전체를 현재 구현 요구로 자동 승격하지 않는다. 다만 custom primitive registry와 `with` conjunction, load/store/emit/cp의 semantic contract는 compiler architecture의 설계 입력으로 명시적으로 검토한다. JAXA의 flow/storage는 논리적 역할이며 CpuStorage/향후 CudaStorage 같은 물리 저장소 타입과 동일시하지 않는다. 최초 검증은 같은 작은 논리 그래프에서 두 실행 계획의 값·오류·상태 변화가 같고 materialization·전송 비용이 달라지는지 확인하는 것이다.

## 변경마다 적용하는 검증

[지속 검증 전략](VALIDATION-STRATEGY.md)을 각 단계의 완료 조건으로 적용한다. 의미 변경은 C 차등 사례, 메모리 변경은 소유권·실패 경로 검사, SIMD 변경은 경계·승격 검사를 추가한다. CI는 기본/portable × j64/j64avx2, 상태 기반 생성 검사, 일일 확대 검사를 수행한다. upstream 전체 테스트·Miri·sanitizer는 미완료 검증 과제로 추적한다.

### C1 첫 구현 상태 (2026-09-27)

실행 없는 Semantic IR 생성과 보수적 primitive contract, 명시적 IR reference evaluator를 추가했다. C1 전체 완료가 아니다. [진행 기록](C1-PROGRESS.md)의 순서대로 lexer/scanner·byte span, name/version binding, registry/with를 먼저 완성한 뒤 C2로 진행한다. 기존 GPU 보류 정책은 유지한다.

### 컴파일러 IR 우선순위 수정 (2026-09-28)

C 내부 실행 구조 대신 의미를 보존하는 심볼·값·호출 분석 IR을 먼저 구축한다. 검사 전용 LogicalPlan 기초를 추가했으며 세부 범위와 남은 작업은 [컴파일러 IR 진행 기록](COMPILER-IR-PROGRESS.md)을 따른다. 위의 C1 완료 후 C2 착수 순서를 엄격한 선행 조건으로 적용하지 않는다.
