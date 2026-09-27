# jaxa-analyzer에서 RustJ에 도입할 설계

검토 기준: [yunskim/jaxa-analyzer, 커밋 3eef39422ddb12183f8432f134cdc54a6419ef7e](https://github.com/yunskim/jaxa-analyzer/tree/3eef39422ddb12183f8432f134cdc54a6419ef7e). 해당 커밋의 문서와 Python 소스를 로컬에서 읽었다. 아래는 설계 채택 제안이며 JAXA 최적화기를 RustJ에 이식했다는 의미가 아니다.

## 2026-09-27 채택된 RustJ/Jaxa 경계

이 검토의 제안 중 의미/실행 분리를 RustJ의 공식 compiler architecture로 채택했다. 상세 기준은 [컴파일러 아키텍처](COMPILER-ARCHITECTURE.md)를 따른다.

```text
Semantic IR
    ↓
Jaxa Analyzer
    ↓
Logical Execution Plan
    ↓
Physical Planner / Optimizer
    ↓
Physical Execution Plan
    ↓
Backend Lowering / Codegen
    ↓
Executor
```

책임 경계를 다음처럼 고정한다.

- **Jaxa Analyzer**: rank/cell/frame/agreement, dtype/shape, primitive contract, dependency/effect/alias, parallel domain을 해석해 Logical Plan 생성.
- **Physical Planner**: strides/view, layout, tiling, device placement/sharding, transfer, fusion, materialization, buffer reuse, work partition 결정.
- **Codegen**: CPU/GPU-specific compiled artifact 생성.
- **Executor**: 이미 결정된 allocation/transfer/kernel/synchronization plan 수행. J 의미 재해석이나 최적화 판단을 하지 않음.

### Array model

J의 logical noun은 type + shape + ordered atoms/value로 유지한다. `strides`, `offset`, `layout`, `device`, `sharding`은 JArray의 semantic property가 아니라 physical representation이다.

```text
JArray / ValueId
       ↓
Logical Plan
       ↓
PhysicalArray
  storage
  shape
  strides
  offset
  layout
  placement
  sharding
```

현재 RustJ의 `CpuStorage`와 contiguous borrowed `ArrayView`는 CPU physical implementation의 기반으로 재해석한다. general physical view는 NumPy/CuPy식 shape + strides + offset을 사용할 수 있지만, J의 agreement를 NumPy broadcasting으로 바꾸지는 않는다.

참고한 설계의 범위:

- NumPy/CuPy: physical strided view
- PyTorch: logical shape와 memory format/layout 분리
- JAX: logical/global value와 placement/sharding/local layout 분리
- ArrayFire: lazy graph 및 합법적인 fusion
- Julia GPUArrays: 복수 backend abstraction
- J: rank/cell/frame/agreement를 parallel decomposition의 의미적 근거로 유지

특히 GPU parallel domain은 generic tensor axis에서 새로 정의하지 않고 Jaxa가 해석한 frame/cell에서 출발한다. 예를 들어 rank-1 적용에서 frame이 8192개 cell을 만든다면 Planner는 이를 GPU work domain으로 사용할 수 있지만 block/thread mapping은 physical decision으로 남긴다.


### JAXA 비표준 어휘 분류와 `with`

JAXA가 J에 없는 어휘를 제공한다는 점을 RustJ compiler 구조에 맞춰 다음처럼 분류한다.

| JAXA 어휘 | 분류 | RustJ/Jaxa 결정 |
|---|---|---|
| `relu` | custom computational primitive | 채택 |
| `linear` | custom computational primitive | 채택 |
| `conv` | custom computational primitive | 채택 |
| `cast_f32` | explicit dtype semantic op | 채택. backend의 암묵적 downcast와 구분 |
| user custom name, 예: `relu_custom_` | registered computational primitive | registry contract가 있으면 허용 |
| `conv_cuda_` | backend-specific primitive | 일반 semantic vocabulary에서는 제외. 필요 시 BackendIntrinsic으로 격리 |
| `load` | logical storage read intent | 채택 |
| `store` | logical storage write intent | 채택 |
| `emit` | side-output/storage effect | source sugar는 허용 가능. core IR에서는 Write/Accumulate로 일반화 |
| `cp` | checkpoint/availability intent | materialize 명령으로 고정하지 않고 CheckpointRequirement로 해석하는 방향 |
| **`with`** | **annotation/binding conjunction** | **채택** |

`with`는 계산 leaf가 아니라 **J 표기와 JAXA semantic metadata의 경계**다.

예:

```j
D1 =: (10 20 1 dense) with adam`dense_xadj`dense_wadj
```

Parser 단계에서는 `with`를 conjunction 구조로 보존하고, 오른쪽 이름들은 name resolution 및 registry binding 후 optimizer/adjoint/effect 등의 typed annotation으로 바꾼다.

허용되는 기본 방향:

```text
with
  ├─ optimizer/update semantics
  ├─ adjoint definitions
  ├─ primitive semantic contract selection
  ├─ semantic dtype requirement
  └─ effect/logical-storage relation
```

허용하지 않는 기본 방향:

```text
with
  ✗ CUDA block/thread count
  ✗ tile128 같은 physical tile policy
  ✗ shared-memory/register budget
  ✗ 특정 GPU/device 선택
  ✗ backend-specific memory layout
  ✗ stream/event schedule
```

후자의 정보는 Physical Planner/Backend Lowering에서 결정한다. `with` 하나에 optimizer, adjoint, CUDA spec, dtype, tile policy를 평평하게 섞지 않는다.

이 분리는 custom primitive registry에도 적용한다. primitive identity는 `conv`처럼 semantic 이름으로 유지하고, CPU/CUDA/Metal 구현은 lowering set으로 연결한다. `conv_cuda_`처럼 backend 이름을 semantic identity에 박는 방식은 일반 vocabulary가 아니라 explicit intrinsic/escape hatch로만 허용한다.


## 확인한 실제 범위

소스는 `tokens.py`, `tokenizer.py`, `jconsole_compare.py` 중심이다. Span을 보존하는 byte 기반 word formation과 J의 `;:` 결과 비교 도구가 있다. 독립 tokenizer 테스트 14개를 실행해 모두 통과했다. 이번 검토에서는 jconsole 차등 테스트를 실행하지 않았다.

Semantic AST, Array IR, resource planner, CUDA codegen은 문서에서 제시하는 방향이며 이 체크아웃에서 구현된 실행 엔진으로 확인하지 못했다. 문서의 ‘현재 단계’ 목록을 소스의 구현 완료 목록으로 해석하지 않는다.

JAXA는 J 문법을 사용하는 제한된 배열 언어와 custom primitive를 목표로 한다. RustJ는 독립적인 J 엔진이 목표이므로, JAXA의 제한 언어 정책을 RustJ 전체의 의미로 대체하지 않는다. 참고: [언어 범위와 IR 비전](https://github.com/yunskim/jaxa-analyzer/blob/3eef39422ddb12183f8432f134cdc54a6419ef7e/docs/jaxa_array_language_and_ir_vision.md#L78), [토크나이저 구현](https://github.com/yunskim/jaxa-analyzer/blob/3eef39422ddb12183f8432f134cdc54a6419ef7e/docs/tokenizer_implementation.md).

## 채택할 핵심

| 아이디어 | RustJ 적용 | 시점 |
|---|---|---|
| 의미와 실행 계획 분리 | J의 의미 표현 → 배열 연산 IR → 실행·메모리 계획 → CPU/CUDA 커널 | 작은 명세부터 시작, M3~M5 단계적 구현 |
| primitive별 계약 | dtype/shape/rank, 오류·승격, 부작용, alias, 허용 변환, scratch 요구량, backend 지원을 명시 | 첫 CUDA primitive와 함께 도입 |
| 논리 값과 실제 버퍼 분리 | IR의 값 하나마다 배열을 할당하지 않음. 마지막 사용과 외부 관찰 지점에서 materialization 판단 | M4 상주 연속 실행 |
| rank의 frame/cell 활용 | 차용 셀은 논리 영역, 병렬화·타일링은 별도 실행 결정 | M2 차용 뷰를 기반으로 M5 |
| 효과·상태 버전 추적 | 이름 바인딩의 읽기/쓰기와 GPU 완료 의존성을 추적 | M4~M5 |
| 하드웨어 프로파일 분리 | CPU/CUDA 지원 dtype·메모리·실행 제약을 문법 밖에 둠 | M3~M5 |
| Unknown을 명시 | 알 수 없는 effect/shape를 순수·정적이라고 추측하지 않음 | IR 진입 검증 |
| Span과 단계별 진단 | word formation, literal/name 분류, parsing, 지원 범위 검증의 책임을 구분 | 후속 frontend 확장 |

이 표의 GPU·IR 항목은 새 계획이며 런타임 구현 완료가 아니다. M2의 실제 코드 변경은 CpuStorage, Shape, ArrayView 및 이를 읽는 rank/reduction 경로다.

근거: [Semantic AST와 Array IR](https://github.com/yunskim/jaxa-analyzer/blob/3eef39422ddb12183f8432f134cdc54a6419ef7e/docs/jaxa_array_language_and_ir_vision.md#L358), [primitive 계약](https://github.com/yunskim/jaxa-analyzer/blob/3eef39422ddb12183f8432f134cdc54a6419ef7e/docs/jaxa_intent_language_and_execution_freedom.md#L552).

## RustJ에 맞춘 구체적인 경계

### 1. ValueId와 BufferId는 다르다

IR의 `ValueId`는 특정 계산 결과를 나타내고, 물리 계획의 `BufferId`는 실제 저장 공간을 나타낸다. 두 논리 값의 수명이 겹치지 않을 때 같은 버퍼를 사용할 수 있다. 반대로 하나의 값에 CPU 복사본과 CUDA 복사본이 있으면 두 버퍼와 최신 버전·완료 상태가 필요하다.

현재의 `CpuStorage`는 물리 저장소다. JAXA의 Flow/LogicalStorage와 같은 개념으로 이름만 대응시키지 않는다. Flow와 persistent storage는 값의 사용 역할이지 고정된 CPU 타입이나 GPU 메모리 종류가 아니다. 임시 값도 분기에서 오래 살아남을 수 있고 이름에 묶인 값도 마지막 사용 뒤에는 재사용 후보가 된다.

초기 최소 IR은 Input, Primitive, Bind를 표현하고, 별도 계획에 Allocate/Reuse, Upload/Download, Launch, Wait, Release를 둔다. 실제 버퍼의 할당/해제는 소유권과 완료 이벤트가 보장한다. 논리적 마지막 사용만으로 GPU 작업 중인 버퍼를 회수하지 않는다.

### 2. primitive 계약은 성능 정보보다 의미를 먼저 기술한다

개념적인 필드는 다음과 같다. 아직 공개 Rust API를 확정한 것은 아니다.

```text
PrimitiveContract
  value: dtype_rule, shape_rule, rank_rule
  errors: domain, shape, overflow/promotion, ordering
  effects: pure | reads/writes state | I/O | unknown
  alias: input/output overlap and in-place conditions
  rewrites: permitted fusion/reordering/recomputation
  resources: scratch/layout requirements or Unknown
  lowering: CPU/CUDA implementation and device requirements
```

예를 들어 i64 덧셈은 값에 따라 전체 배열을 f64로 승격시킨다. ‘원소별 순수 연산’이라고 선언해도 승격 경계를 무시한 fusion은 허용되지 않는다. float 연산에서도 FMA, 재결합, reduction 순서 변경은 별도 허용 조건이다. unknown effect는 최적화 장벽으로 남긴다.

외부 cuBLAS 같은 호출을 나중에 도입한다면 내부 구현은 opaque하더라도 타입·shape·메모리·stream·오류 계약은 제공해야 한다. 커널 내부를 볼 수 없다는 이유로 자원 비용을 0으로 보거나 자동 fusion하지 않는다.

### 3. J의 관찰 가능한 순서를 보존한다

`r =: (a + b) * c`에서 임시 합을 제거하려면 `(a + b)`의 전체 배열 승격과 후속 곱셈 의미를 보존해야 한다. 값 범위 증명, guard와 다시 계산하는 경로, 또는 별도 실행이 필요하다. 단순한 ‘커널 하나가 더 빠르다’는 이유로 결합하지 않는다.

`b =: a` 뒤에 `a`를 갱신해도 `b`의 기존 값은 보존한다. IR은 바인딩 버전을 구별하고, 물리적인 제자리 수정은 살아 있는 alias가 없으며 오류 시 상태 보존 조건을 만족할 때만 선택한다. 부작용·관찰 가능한 오류 순서를 바꾸지 않는다. 현재의 문장 단위 대입 성공 후 반영 정책도 유지한다.

### 4. 비용은 연산별 숫자의 합이 아니다

fusion하면 중간 배열 트래픽은 줄지만 register 수명, accumulator, 접근 layout과 occupancy는 악화될 수 있다. 비용 모델은 graph·schedule·device 조합을 입력으로 받고 실제 컴파일 결과와 측정으로 보정한다. 초기에는 transfer bytes, materialized bytes, launch 수, peak live bytes처럼 검증 가능한 항목부터 사용한다.

최초 검증은 작은 고정 타입의 두 연산 그래프에 대해 별도 실행과 안전한 결합 실행을 비교한다. 값·타입·오류·부작용 동등성과 전송 포함 시간·peak memory를 함께 확인한다. 처음부터 모든 J 식을 전역 최적화하려 하지 않는다.

근거: [논리 메모리 계획과 CUDA 물리 계획의 분리](https://github.com/yunskim/jaxa-analyzer/blob/3eef39422ddb12183f8432f134cdc54a6419ef7e/docs/flow_storage_scope_and_compiler_positioning.md#L257), [합성 자원 모델](https://github.com/yunskim/jaxa-analyzer/blob/3eef39422ddb12183f8432f134cdc54a6419ef7e/docs/primitive_blackbox_resource_composition.md#L540).

## 바로 가져오지 않을 부분

- JAXA의 primitive whitelist는 GPU/분석 가능한 영역 판정에만 응용한다. RustJ 전체에서 유효한 J 연산을 제한하는 근거로 쓰지 않는다. GPU 미지원과 언어 자체 미지원은 구별한다.
- Python registry는 필수 런타임 의존성으로 도입하지 않는다. 계약은 먼저 Rust 자료구조로 표현하고 향후 외부 도구용 직렬화는 별도 검토한다.
- JAXA 문서의 fp32 초기 대상은 RustJ의 i64/f64 의미를 낮추는 근거가 아니다. 첫 GPU 범위도 CUDA로 유지한다.
- load/store/emit/cp를 새 J 문법이나 확정 primitive로 추가하지 않는다. cp의 강제 materialization과 추상 availability 차이는 원문에서도 열린 질문이다.
- LeNet·학습·자동 미분은 현재 요구 범위에 추가하지 않는다. 일반 배열의 값 흐름·상태·수명 분석만 가져온다.
- byte 토크나이저의 Span·상태 기계는 frontend 개선에 참고하되 Latin-1 입력 제한이나 전체 소스를 그대로 가져오지 않는다. RustJ의 입력 인코딩과 진단 정책은 별도로 정의한다.

근거: [의도와 실행의 자유, cp의 미결정 사항](https://github.com/yunskim/jaxa-analyzer/blob/3eef39422ddb12183f8432f134cdc54a6419ef7e/docs/jaxa_intent_language_and_execution_freedom.md#L378), [Flow–Storage의 작업 모델 지위](https://github.com/yunskim/jaxa-analyzer/blob/3eef39422ddb12183f8432f134cdc54a6419ef7e/docs/flow_storage_scope_and_compiler_positioning.md#L3).

## 후속 완료 기준

1. 첫 CPU/CUDA primitive의 계약으로 dtype·shape·오류·장치 지원을 검사할 수 있다.
2. 작은 동일 논리 그래프에서 CPU와 CUDA 계획을 만들고 의미를 비교할 수 있다.
3. alias 또는 미완료 작업이 있는 버퍼의 재사용을 거부한다.
4. unknown effect와 승격 가능 연산을 잘못 결합하지 않는 음성 테스트가 있다.
5. 중간 배열·전송·실행 시간의 감소를 실측한다. AST/IR를 도입했다는 사실만으로 성능 성공을 선언하지 않는다.
