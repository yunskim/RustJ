# RustJ 컴파일러 아키텍처

> 상태: **채택된 목표 아키텍처**  
> 기준일: 2026-09-27  
> 현재 저장소의 직접 평가형 CPU 엔진은 이 구조 전체를 구현한 것이 아니다. 기존 엔진은 J 의미 검증, 차등 테스트, 초기 CPU 커널과 향후 reference interpreter/fallback의 기반으로 유지한다.

## 1. 프로젝트 목표

RustJ의 목표는 `jsource`/csource를 Rust로 문자 그대로 번역한 인터프리터가 아니다.

**J의 언어 및 배열 의미론을 보존하면서, Jaxa 기반의 분석·계획 계층을 통해 CPU와 GPU를 대상으로 최적화된 코드를 생성하는 Rust 기반 J 컴파일러를 구현한다.**

`jsource`는 다음 목적의 reference implementation으로 사용한다.

- J의 관찰 가능한 언어 의미 확인
- 오류·승격·rank·agreement·배열 동작 차등 검증
- 성능·메모리 정책 비교
- 미묘한 primitive 동작 조사

C J 커널은 RustJ 실행의 fallback으로 사용하지 않는다.

## 2. 최상위 파이프라인

```text
J Source
   ↓
Tokenizer / Parser
   ↓
Semantic IR
   ↓
Jaxa Analyzer
   ↓
Logical Execution Plan
   ↓
Logical Optimizer
   ↓
Physical Planner / Optimizer
   ↓
Physical Execution Plan
   ↓
Backend Lowering / Code Generation
   ├─ CPU IR / native code
   ├─ CUDA kernel IR / kernel
   ├─ Metal
   ├─ Vulkan / SPIR-V
   └─ future backends
   ↓
Compiled Artifact
   ↓
Runtime / Executor
```

핵심 규칙은 **분석과 실행을 분리하는 것**이다.

- Jaxa Analyzer는 IR을 해석하고 계획을 만든다.
- Physical Planner는 하드웨어·메모리·layout 결정을 내린다.
- Codegen은 실제 backend code를 만든다.
- Executor는 이미 정해진 계획과 compiled artifact를 실행한다.
- Executor가 rank 의미를 다시 해석하거나 fusion·layout·device 선택을 다시 판단하지 않는다.

## 3. 인터프리터의 위치

인터프리터를 제품 아키텍처의 중심으로 두지 않는다.

기존 직접 평가 엔진은 장기적으로 다음 역할을 맡을 수 있다.

1. J 의미의 reference implementation
2. compiler differential test의 oracle
3. 디버깅 경로
4. 아직 compiler lowering이 없는 기능의 명시적 fallback

즉 기본 구조는

```text
parse → analyze → plan → optimize → compile → execute
```

이며,

```text
parse → verb dispatch → 즉시 실행
```

을 중심 구조로 확대하지 않는다.

## 4. J 고유 배열 모델을 보존한다

GPU 친화성을 위해 J의 noun을 일반 tensor framework의 Tensor 의미론으로 바꾸지 않는다.

논리적인 J 배열은 계속 다음 모델을 따른다.

```text
JArray
├─ type
├─ shape
└─ ordered atoms / logical value
```

개념적인 형태:

```rust
pub struct JArray {
    pub ty: JType,
    pub shape: Shape,
    pub value: ValueRef,
}
```

논리 JArray에 다음 정보를 넣지 않는다.

- stride
- physical offset
- CPU/GPU device
- tile shape
- memory format
- alignment
- sharding
- CUDA block/thread
- stream/event

이 정보들은 J의 의미가 아니라 **물리적 표현과 실행 계획의 선택**이다.

## 5. rank / cell / frame / agreement가 먼저다

GPU를 위해 NumPy식 broadcasting을 J 의미론으로 도입하지 않는다.

Jaxa Analyzer는 J의 기존 의미를 먼저 해석한다.

- argument rank
- verb rank
- frame
- cell
- dyadic agreement
- result rank / shape
- prototype / padding rules
- 오류와 타입 승격

예를 들어 argument shape가

```text
[64, 128, 256]
```

이고 적용되는 verb rank가 1이면 J 의미 수준에서

```text
frame      = [64, 128]
cell shape = [256]
cell count = 8192
```

가 된다.

이 정보는 GPU에서도 강력한 병렬 domain이지만, **GPU를 위해 만들어진 규칙이 아니라 J 의미론의 결과**다.

## 6. Semantic IR

Semantic IR은 J 프로그램이 무엇을 의미하는지를 나타낸다.

아직 다음 정보는 없어야 한다.

- CUDA block size
- physical strides
- tiling
- device transfer
- shared memory
- register allocation
- 특정 kernel launch

예:

```text
Reduce(+)
    │
   Add
  /   \
 Mul   c
/  \
a    b
```

Semantic IR은 source span과 name/version 정보를 보존하여 이후 진단이 원래 J source로 돌아갈 수 있어야 한다.

## 7. Jaxa Analyzer

Jaxa Analyzer는 **IR을 해석해서 실행 가능한 논리 계획을 세우는 계층**이다. 직접 실행하지 않는다.

주요 책임:

- J primitive 의미 계약 적용
- rank / cell / frame / agreement 해석
- dtype·shape 추론
- 오류·승격 조건 추적
- name binding / effect / alias 분석
- dependency graph 구성
- independent parallel domain 식별
- map / reduce / scan / gather / structural operation 분류
- 합법적인 fusion·rewrite 후보 판정
- materialization이 관찰상 필요한 경계 식별
- backend 지원 가능성 및 Unknown 표시

### 7.1 Primitive contract

primitive마다 최소한 다음 의미 계약을 가진다.

```text
PrimitiveContract
  value:
    dtype_rule
    shape_rule
    rank_rule

  errors:
    domain
    rank
    length/shape
    overflow/promotion
    observable ordering

  effects:
    pure
    reads/writes state
    I/O
    unknown

  alias:
    input/output overlap
    in-place legality

  rewrites:
    fusion
    reordering
    recomputation
    reassociation

  parallel:
    map
    reduce
    scan
    gather
    scatter
    structural

  lowering:
    supported backend classes
```

GPU-specific block 크기 같은 것은 primitive contract에 넣지 않는다.

### 7.2 JAXA 확장 어휘와 `with` conjunction

JAXA가 J에 없는 이름을 제공한다고 해서 모든 확장을 같은 종류의 primitive로 취급하지 않는다. RustJ/Jaxa에서는 다음 taxonomy를 사용한다.

```text
JAXA extensions
├─ CustomComputationalPrimitive
│    relu
│    linear
│    conv
│    cast_f32
│    user-registered verbs
│
├─ Effect / Storage Operation
│    load
│    store
│    Write / Accumulate
│    (source-level emit은 여기로 lowering 가능)
│
├─ Semantic Annotation
│    CheckpointRequirement
│    (source-level cp의 권장 lowering)
│
├─ Annotation / Binding Conjunction
│    with
│
└─ Backend Intrinsic
     explicit escape hatch only
     (일반 semantic primitive와 구분)
```

#### `with`의 역할

`with`는 computational primitive가 아니라 **J로 표현된 계산 구조와 JAXA가 추가하는 semantic metadata/contract를 연결하는 conjunction**으로 취급한다.

예:

```j
D1 =: (10 20 1 dense) with adam`dense_xadj`dense_wadj
```

의 개념적 해석은 다음과 같다.

```text
With
├─ subject: dense operation/specification
└─ semantic annotations
     ├─ optimizer: adam
     ├─ input adjoint: dense_xadj
     └─ weight adjoint: dense_wadj
```

Tokenizer가 `with` 오른쪽의 개별 이름을 GPU 설정값으로 해석해서는 안 된다. Parser는 J의 conjunction 구조와 source span을 보존하고, name resolution/registry binding 이후 structured annotation으로 정규화한다.

`with`에 허용할 수 있는 정보의 기본 범주는 다음과 같다.

- optimizer 또는 update semantics
- adjoint / derivative definition
- custom primitive의 semantic contract 선택
- dtype requirement가 계산 의미의 일부인 경우 그 requirement
- effect / logical-storage relationship
- analyzer가 반드시 보존해야 하는 semantic obligation

반대로 다음은 `with`의 semantic metadata로 넣지 않는 것을 기본으로 한다.

- CUDA block/thread 수
- tile 크기
- shared-memory 크기
- register budget
- 특정 device 선택
- backend-specific physical layout
- stream/event 배치
- 특정 CUDA kernel 이름

이들은 Physical Planner/Backend Lowering의 책임이다.

따라서 다음과 같은 평평한 annotation은 피한다.

```text
dense with adam`dense_xadj`dense_wadj`cuda_spec`fp16`tile128
```

여기에는 optimizer/adjoint와 backend/tile 정책이 섞여 계층 경계를 무너뜨린다. 필요하다면 `with` 자체의 오른쪽 값을 typed annotation record로 정규화하고, physical policy는 별도의 planner hint 또는 실행 정책 계층으로 분리한다.

#### Backend-specific custom primitive

`conv_cuda_`처럼 backend 이름이 semantic primitive identity에 들어간 형태는 일반 core vocabulary로 채택하지 않는다.

권장 구조:

```text
semantic primitive: conv
    ↓
available lowerings
    ├─ CPU
    ├─ CUDA
    ├─ Metal
    └─ ...
```

backend-specific primitive가 꼭 필요하면 명시적인 `BackendIntrinsic`/escape-hatch 범주로 두고 portability와 optimization freedom이 제한된다는 점을 contract에 표시한다.

### 7.3 Unknown은 추측하지 않는다

shape, effect, alias, backend legality를 알 수 없으면 안전한 값으로 꾸며내지 않고 `Unknown`으로 남긴다.

Unknown은 필요하면 optimization boundary나 runtime guard가 된다.

## 8. Logical Execution Plan

Analyzer의 결과는 직접 실행 코드가 아니라 logical plan이다.

예시 operation:

```rust
pub enum LogicalOp {
    RankMap { /* J cell/frame mapping */ },
    Map { /* primitive */ },
    Reduce { /* primitive + axis/domain */ },
    Scan { /* primitive + axis/domain */ },
    Gather { /* index semantics */ },
    Scatter { /* if supported */ },
    Reshape { /* J semantics */ },
    Transpose { /* logical axis transform */ },
    Structural { /* take/drop/reverse/etc. */ },
}
```

### 8.1 RankMap을 first-class로 둔다

일반 tensor IR이 elementwise axis만 보는 것과 달리 RustJ의 logical plan은 J의 rank calculus를 보존한다.

```text
RankMap
  result frame
  argument frames
  cell shapes
  cell ranks
  agreement mapping
  verb
```

이 정보로 CPU/GPU 모두 같은 logical plan에서 출발한다.

## 9. 논리 배열과 물리 배열을 분리한다

논리적인 `ValueId`와 실제 메모리의 `BufferId`는 다르다.

```text
ValueId
  = J 계산의 논리 결과

BufferId
  = 특정 backend 메모리의 실제 저장 공간
```

하나의 ValueId가 CPU/GPU 양쪽 representation을 가질 수도 있고, 서로 수명이 겹치지 않는 여러 ValueId가 같은 BufferId를 재사용할 수도 있다.

### 9.1 PhysicalArray

물리 representation은 대략 다음 정보를 가진다.

```rust
pub struct PhysicalArray {
    pub storage: StorageId,
    pub view: ArrayView,
    pub placement: Placement,
    pub layout: PhysicalLayout,
}
```

```rust
pub struct ArrayView {
    pub shape: Shape,
    pub strides: Strides,
    pub offset: isize,
}
```

이것은 JArray가 아니라 backend execution representation이다.

### 9.2 구현 전 불변식 보완 (2026-09-30)

위 구조 예시는 개념 모델이며 Rust object/device ABI 선언이 아니다.
세 차례 분리 검토의 상세 근거는 [GPU 배열 설계 검토](GPU-ARRAY-DESIGN-AUDIT.md),
현재 실행 범위는 [GPU 배열 체크리스트](GPU-FRIENDLY-ARRAY-PLAN.md)를 따른다.

- 첫 physical mapping은 immutable checked `AffineDense`다. strides/offset은 이 mapping의 속성이다.
- packed bit, tiled, sparse, boxed는 별도 encoding/mapping이며 affine byte-stride 모델로 강제하지 않는다.
- BufferId는 주소와 allocation alias identity가 아니다. scoped/generation ID를 실제 owner에서 resolve하며 탈출 view는 owner lease/borrow를 보존한다.
- inline 이동 및 registry 성장 중 raw pointer를 보관하지 않는다. slice lease 동안 backing 이동·교체를 금지한다.
- CPU slice의 논리 순서 연속성과 메모리 연속성을 구분한다. 초기 mutable 경로는 독점 표준 연속 출력만 허용한다.
- empty descriptor는 shape를 보존하고 offset/strides를 0으로 정규화한다. 주소 연산을 하지 않는다. singleton stride도 0으로 정규화한다.
- 초기 encoding은 BoolByte/Int64/Float64/Char8이며 backing의 dtype·initialized length·span을 checked 검증한다.
- representation record는 논리 값 identity와 layout/encoding/location/readiness를 연결한다. mutable binding version은 별도다.
- GPU ABI lowering은 고정 폭 index/address와 target capability를 검증한다. host usize/isize/Arc layout을 그대로 넘기지 않는다.
- 첫 CPU physical 실행은 fresh binding snapshot에 한정한다. reusable plan은 버전·dtype/shape·동적 verb guards 및 재분석을 요구한다.
- 실제 CUDA 완료/수명 구현은 보류하며 제출 완료를 값 Ready나 버퍼 재사용 증명으로 쓰지 않는다.

## 10. GPU 친화적 배열 설계

### 10.1 stride/view

NumPy/CuPy와 유사하게 physical view는 shape + strides + offset을 사용할 수 있다.

이를 통해 다음 operation을 가능한 한 data movement 없이 표현한다.

- transpose
- slice
- reverse
- compatible reshape
- 반복 access

예:

```text
before transpose
shape   [100, 200]
strides [200, 1]

after transpose view
shape   [200, 100]
strides [1, 200]
```

logical J transpose와 physical copy는 같은 것이 아니다.

### 10.2 J agreement와 zero stride

J 의미는 먼저 agreement로 확정한다.

그 결과 같은 atom/cell을 반복 읽는 physical lowering이 가능하다면 zero-stride view를 사용할 수 있다.

```text
J agreement
    ↓
logical mapping
    ↓
physical zero-stride implementation (optional)
```

zero stride가 J broadcasting 규칙을 정의하는 것은 아니다.

### 10.3 dense와 tiled layout

```rust
pub enum PhysicalLayout {
    Dense(DenseLayout),
    Tiled(TileLayout),
}
```

J는 reshape/transpose/take/drop/ravel/reverse 같은 structural operation이 많으므로 기본 representation을 무조건 tiled로 고정하지 않는다.

Planner가 workload에 따라 선택한다.

- strided dense view 유지
- contiguous copy
- matrix/tensor용 tiled temporary
- GPU shared-memory tile
- backend-specific swizzle가 필요하면 추후 확장

### 10.4 placement와 sharding

JAX류 시스템의 장점을 참고해 logical shape와 device placement를 분리한다.

```rust
pub enum Placement {
    Cpu,
    Device(DeviceId),
    Sharded(ShardingId),
}
```

예를 들어 logical J noun이

```text
[8192, 4096]
```

이어도 4개 GPU에서 physical shard가

```text
GPU0  rows    0..2048
GPU1  rows 2048..4096
GPU2  rows 4096..6144
GPU3  rows 6144..8192
```

일 수 있다.

J 사용자에게는 여전히 하나의 동일한 배열이다.

## 10.5 기존 J/RustJ 배열과 GPU-friendly physical array의 차이

이 설계에서 **J 배열의 언어적 의미는 바뀌지 않는다.** 달라지는 것은 그 값을 실행기에 전달하는 물리 표현의 자유도다.

현재 RustJ의 CPU 구현은 대략 다음 구조다.

```text
Value
├─ Shape
└─ Data
   ├─ Bool(CpuStorage<u8>)
   ├─ Int(CpuStorage<i64>)
   ├─ Float(CpuStorage<f64>)
   └─ Char(CpuStorage<u8>)

CpuStorage
├─ Inline
├─ Owned(Vec)
└─ Shared(Arc<Vec>)

ArrayView
├─ shape
└─ borrowed contiguous CPU slice
```

이 모델은 CPU dense array에는 단순하고 효율적이지만, physical view가 사실상 **연속 CPU slice**라는 가정을 갖는다. 현재 transpose/reverse/take/drop 같은 structural operation은 일반적으로 새 dense output을 만들어야 하고, rank cell도 contiguous suffix slice로 잘라 사용하는 구조다.

목표 구조는 다음처럼 책임을 나눈다.

```text
J semantic value
    JArray
    ├─ type
    ├─ shape
    └─ ordered atoms / logical value

            ↓ compiled representation

PhysicalArray
├─ StorageId / BufferId
├─ shape
├─ strides
├─ offset
├─ PhysicalLayout
├─ Placement
├─ optional Sharding
└─ completion/lifetime state
```

차이는 다음과 같다.

| 항목 | 현재 J/RustJ CPU 모델 | GPU-friendly 목표 모델 |
|---|---|---|
| J 의미 | type + shape + ordered atoms | **동일** |
| rank/cell/frame/agreement | J 의미 및 CPU 실행에 직접 사용 | **동일한 J 의미**, Analyzer가 Logical Plan으로 명시 |
| 저장 위치 | CPU | CPU / GPU / multi-device |
| 기본 데이터 표현 | dense contiguous CPU buffer | dense contiguous를 기본 후보로 하되 strided/tiled 등 허용 |
| view | shape + contiguous borrowed slice | storage + shape + strides + offset |
| transpose | 대체로 새 dense 결과 | 우선 stride permutation view, 필요할 때만 materialize |
| reverse | 대체로 새 dense 결과 | negative stride/offset view가 합법하면 view |
| slice | contiguous slice 중심 | arbitrary strided view 가능 |
| reshape | dense ordering에 직접 의존 | layout-compatible하면 metadata-only, 아니면 relayout |
| agreement 반복 접근 | 직접 실행 규칙 | J agreement를 먼저 확정한 후 zero-stride 등의 physical lowering 허용 |
| tile | 없음 | planner가 workload에 따라 선택 |
| device | 없음 | Placement로 분리 |
| sharding | 없음 | logical shape와 독립된 physical property |
| 중간 값 | Value마다 배열이 생기기 쉬움 | ValueId와 BufferId를 분리해 materialization/fusion/reuse 계획 |
| 실행 선택 | primitive가 CPU storage를 직접 처리 | Logical Plan → Physical Plan → codegen → Executor |

따라서 새 `ArrayView`는 현재 차용 CPU slice의 단순 확장이 아니라, **논리 배열과 물리 저장소 사이의 일반적인 indexing mapping**이 된다.

```rust
pub struct ArrayView {
    pub storage: StorageId,
    pub shape: Shape,
    pub strides: Strides,
    pub offset: isize,
}
```

CPU backend도 이 representation을 사용할 수 있으므로 GPU 지원을 위해 CPU를 별도 세계로 만들 필요가 없다. 다만 hot loop에서는 generic stride 계산을 매 atom마다 수행하지 않고 Planner/Codegen이 contiguous, fixed-stride, broadcast, reversed 등의 경우를 specialization해야 한다.

### 10.5.1 현재 `CpuStorage`는 폐기 대상이 아니라 physical backend 구현의 시작점이다

현재 `CpuStorage<T>`의 Inline / Owned / Shared 구분과 소유권 기반 재사용은 계속 가치가 있다. 다만 장기적으로는 J semantic `Value`와 직접 결합된 유일 저장소가 아니라 CPU physical storage 구현으로 위치를 명확히 한다.

```text
JArray / ValueId
      ↓
PhysicalArray
      ↓
StorageId
      ├─ CpuStorage
      ├─ CudaStorage
      ├─ MetalStorage
      └─ ...
```

즉 기존 CPU 최적화를 버리고 새 tensor runtime으로 갈아엎는 것이 아니라, **현재 CPU storage를 compiler backend의 한 physical implementation으로 재배치**한다.

### 10.5.2 JArray에는 strides를 넣지 않는다

NumPy 계열에서는 ndarray 객체 자체가 shape/strides/data를 사용자-facing 배열 정체성으로 갖지만 RustJ에서는 J noun의 의미와 backend representation을 더 강하게 분리한다.

이유:

1. 같은 J value가 CPU와 GPU에서 서로 다른 layout을 동시에 가질 수 있다.
2. 같은 J value를 어떤 연산에서는 strided view로, 다른 연산에서는 tiled temporary로 사용할 수 있다.
3. multi-GPU shard가 있어도 J의 logical shape는 바뀌지 않는다.
4. optimizer가 physical transpose/materialization을 자유롭게 선택하려면 layout이 J value의 정체성에 고정되어서는 안 된다.

따라서 `strides`, `offset`, `layout`, `placement`는 **compiled physical representation의 속성**이다.

## 10.6 다른 배열/컴파일 프레임워크에서 받은 영향

이 설계는 여러 기존 시스템에서 검증된 physical/runtime 아이디어를 가져오지만 J의 언어 의미를 그 프레임워크의 tensor semantics로 대체하지 않는다.

| 출처 | 채택하는 결정 | RustJ에서의 위치 | 의도적으로 채택하지 않는 것 |
|---|---|---|---|
| **NumPy** | shape + strides + offset으로 같은 buffer의 여러 view를 표현 | PhysicalArray / ArrayView | NumPy broadcasting을 J agreement로 대체하지 않음 |
| **CuPy** | NumPy식 strided array representation을 GPU device memory에도 적용 가능하다는 모델 | GPU physical storage/view | CuPy ndarray를 J noun의 사용자 의미 모델로 삼지 않음 |
| **PyTorch** | logical dimensions와 physical memory format/layout을 분리 | PhysicalLayout / Planner | Tensor의 channels-last 등 특정 format을 J 언어 속성으로 노출하지 않음 |
| **JAX** | logical/global array와 device placement, sharding, device-local layout을 분리 | Placement / Sharding / PhysicalLayout | JAX tracing·broadcasting·functional array semantics를 J 의미로 가져오지 않음 |
| **ArrayFire** | expression을 바로 materialize하지 않고 fusion 가능한 graph로 유지 | Logical/Physical planning, codegen | 모든 J 연산을 무조건 lazy fusion하지 않음; J 오류·승격 의미가 우선 |
| **Julia GPUArrays** | 공통 array/algorithm abstraction 아래 여러 GPU backend를 붙이는 구조 | backend trait / codegen / executor | 특정 Julia array API를 노출하지 않음 |
| **J 자체** | rank/cell/frame/agreement와 ordered atom semantics | Semantic IR / Jaxa Analyzer / Logical Plan | 이 부분은 다른 tensor framework의 axis/broadcast 모델로 교체하지 않음 |

### 10.6.1 NumPy / CuPy — physical view

가장 직접적인 영향은 arbitrary strided view다.

```text
logical index (i,j,...)
      ↓
offset + Σ(index[k] * stride[k])
      ↓
physical storage location
```

이 모델 덕분에 transpose, reverse, slice, compatible reshape를 가능한 한 metadata operation으로 만들 수 있다.

하지만 RustJ에서는 순서가 중요하다.

```text
J semantics
   ↓
logical operation 확정
   ↓
Physical Planner
   ↓
strided view를 사용할지 materialize할지 선택
```

즉 physical view가 J의 의미를 결정하지 않는다.

### 10.6.2 PyTorch — memory format은 logical shape와 다르다

PyTorch가 같은 논리 tensor를 서로 다른 memory format/stride로 표현할 수 있다는 점에서 영향을 받았다.

RustJ의 대응은 더 일반적이다.

```text
Logical J shape
     ≠
PhysicalLayout
```

Planner는 연산별로 dense contiguous, transposed stride, tiled layout 등의 physical form을 선택할 수 있다.

### 10.6.3 JAX — placement, sharding, local layout의 분리

multi-device 확장성을 위해 가장 중요한 참고 중 하나다.

```text
logical global value
     ↓
placement / sharding
     ↓
device-local physical layout
```

이 구분을 채택하면 multi-GPU를 추가해도 JArray의 shape나 rank 의미를 수정할 필요가 없다.

### 10.6.4 ArrayFire — lazy expression과 fusion

J 표현식은 여러 array primitive를 연속으로 쓰는 경우가 많다. 중간 결과를 매 단계 materialize하면 GPU global-memory traffic과 kernel launch가 증가한다.

따라서 logical value를 즉시 buffer와 동일시하지 않고, 분석 후 합법적인 범위에서 fusion한다.

단, fusion의 legality는 성능보다 J semantics가 먼저다. 특히 다음은 barrier가 될 수 있다.

- 전체 배열 단위 정수 overflow/promotion
- 관찰 가능한 오류 순서
- alias/state effect
- floating-point reduction order
- FMA/reassociation 허용 여부

### 10.6.5 Julia GPUArrays — backend를 array semantics와 분리

RustJ compiler는 CUDA compiler 하나로 고정하지 않는다.

```text
Logical Plan
   ├─ CPU backend
   ├─ CUDA backend
   ├─ Metal backend
   ├─ Vulkan/SPIR-V backend
   └─ future backend
```

첫 GPU backend가 CUDA여도 frontend, Analyzer, Logical Plan은 CUDA API를 알지 않는다.

## 10.7 RustJ 고유 결정: J rank calculus를 GPU parallel domain으로 사용

다른 framework에서 가져온 physical 기술보다 더 중요한 것은 **J가 이미 갖고 있는 rank calculus를 병렬 decomposition의 출발점으로 사용하는 것**이다.

예:

```text
argument shape = [64, 128, 256]
verb rank      = 1

J analysis:
frame      = [64, 128]
cell shape = [256]
cell count = 8192
```

Jaxa Analyzer는 이 사실을 Logical Plan에 명시한다.

```text
RankMap
  8192 independent cell applications
  cell shape = [256]
```

Physical Planner는 그 다음에만 hardware mapping을 정한다.

```text
J frame cells
    ↓
GPU grid / workgroups

one or more J cells
    ↓
block / workgroup

atoms inside cell
    ↓
threads / lanes / SIMD
```

이 mapping은 고정 규칙이 아니다. cell이 작으면 block 하나가 여러 cell을 처리할 수 있고, cell이 크면 하나의 cell이 여러 block/단계로 나뉠 수도 있다. 중요한 점은 **parallel domain의 semantic origin이 generic tensor axis가 아니라 J의 frame/cell 분석**이라는 것이다.

reduction과 scan도 같은 방식이다.

```text
Jaxa:
  "1024 independent rank-1 cells, each 4096 atoms"

GPU planner:
  "1024 reduction domains"
  → warp/block/hierarchical algorithm 선택
```

따라서 GPU backend가 J의 rank conjunction이나 agreement 규칙을 다시 구현하지 않는다.

## 10.8 의도적으로 하지 않는 설계

GPU 친화성 때문에 다음 방향으로 가지 않는다.

1. **JArray를 PyTorch/JAX식 Tensor 언어 객체로 재정의하지 않는다.**
2. **NumPy broadcasting을 J agreement 대신 사용하지 않는다.**
3. **CUDA block/thread 정보를 primitive나 JArray에 저장하지 않는다.**
4. **모든 배열을 처음부터 tiled storage로 강제하지 않는다.**
5. **transpose/reshape를 항상 copy 또는 항상 view라고 고정하지 않는다. Planner가 선택한다.**
6. **GPU에서 유리하다는 이유로 J의 overflow, promotion, error ordering, floating-point 의미를 자동 변경하지 않는다.**
7. **CPU와 GPU용 semantic IR을 따로 만들지 않는다. 둘은 동일 Logical Plan에서 출발한다.**
8. **GPU 지원 범위를 RustJ 언어 지원 범위와 동일시하지 않는다.**

## 10.9 이 결정이 현재 구현에 주는 구체적 변경 방향

현재 코드에서 단계적으로 다음 이동을 한다.

```text
현재
Value(shape + Data<CpuStorage>)
        +
contiguous borrowed ArrayView
        +
primitive direct execution

          ↓

중간 단계
Semantic Value / ValueId
        +
CPU PhysicalArray
        +
general ArrayView(shape/strides/offset)
        +
Logical Plan / Physical Plan
        +
기존 CPU kernel adapter

          ↓

목표
J semantic value
        +
backend-independent Logical Plan
        +
planner-selected PhysicalArray
        +
CPU/GPU codegen
        +
Executor
```

첫 physical-view 구현에서는 다음을 검증한다.

- contiguous dense view가 현재 CPU 성능을 회귀시키지 않는가
- transpose/reverse/slice를 view로 만들었을 때 J semantics가 동일한가
- unsupported stride를 만났을 때 planner가 안전하게 contiguous materialization을 삽입하는가
- alias/lifetime 때문에 buffer reuse가 잘못 일어나지 않는가
- rank cell 추출이 contiguous-only 가정을 하지 않아도 동일한 logical cell을 가리키는가
- view optimization이 C reference와의 차등 결과를 바꾸지 않는가

이 검증이 끝난 뒤 GPU storage를 추가한다. GPU가 general strided view를 모두 빠르게 처리할 것이라고 가정하지 않는다. planner는 kernel별 access pattern과 cost를 보고 view 유지와 relayout 사이에서 선택한다.


## 11. Physical Planner / Optimizer

Logical Plan을 실제 하드웨어에서 어떻게 실행할지 결정한다.

주요 책임:

- CPU / GPU / 향후 backend 선택
- device placement
- device transfer
- physical layout
- stride/view 유지 여부
- physical transpose / relayout 여부
- tiling
- fusion group
- materialization
- buffer reuse와 liveness
- work partition
- reduction / scan 알고리즘 선택
- GPU grid/block/warp mapping
- shared-memory 사용 계획
- synchronization dependency

### 11.1 J frame/cell을 GPU에 연결한다

예:

```text
frame      = [64, 128]
cell shape = [256]
```

이면 Jaxa가 먼저

```text
8192 independent cells
```

를 알려준다.

GPU planner는 그 다음

```text
frame cells → grid/workgroups
cell(s)     → block
cell atoms  → threads / lanes
```

같은 mapping 후보를 만든다.

정확한 block size는 device profile, register use, occupancy, access pattern을 보고 Physical Planner가 정한다.

### 11.2 reduction

```j
+/ y
```

에서 analyzer가

```text
1024 independent cells
each cell = 4096 atoms
```

이라고 판단했다면 planner는 이를 warp/block/hierarchical reduction으로 내릴 수 있다.

Jaxa가 CUDA reduction algorithm을 알 필요는 없다.

### 11.3 scan

scan도 동일하다.

Analyzer:

```text
frame마다 독립 scan
```

Planner:

```text
warp scan / block scan / hierarchical scan
```

중에서 선택한다.

## 12. fusion과 materialization

ArrayFire 등 lazy array runtime의 장점을 참고하되, fusion은 J 의미 계약을 깨지 않는 경우에만 허용한다.

예:

```j
+/ a * b + c
```

logical graph:

```text
Reduce(+)
    │
   Add
  /   \
 Mul   c
/  \
a    b
```

planner가 다음 중 하나를 선택할 수 있다.

```text
kernel 1: mul + add
kernel 2: reduce
```

또는 안전하고 이득이 있으면

```text
kernel 1: mul + add + partial reduce
kernel 2: final reduce
```

하지만 다음 조건을 무시해 fusion하지 않는다.

- i64 overflow 후 전체 결과 승격
- floating-point reduction 순서
- FMA / reassociation 허용 여부
- alias 관찰 가능성
- effect / error ordering

### 12.1 MaterializationReason

물리 계획에서 materialization 이유를 설명 가능하게 유지한다.

```text
UserVisible
BackendRequirement
UnsupportedLayout
DeviceTransfer
AliasSafety
ExternalInterface
```

이는 `explain` 기능과 compiler debugging에 사용한다.

## 13. Physical Execution Plan

Physical Plan에는 Executor가 다시 판단하지 않아도 될 정도의 실행 결정이 들어간다.

예:

```rust
pub enum PhysicalStep {
    Allocate { /* buffer + device + layout */ },
    Transfer { /* source/target */ },
    ReLayout { /* input/output layout */ },
    LaunchKernel { /* compiled kernel + launch plan */ },
    Synchronize { /* dependency */ },
    ReleaseOrRecycle { /* after completion */ },
}
```

GPU launch plan은 이 단계에서 구체적일 수 있다.

```text
grid
block
shared_memory_bytes
stream/dependency
input/output buffer bindings
```

## 14. Backend lowering과 code generation

Physical Plan을 실제 backend code로 내린다.

```text
Physical Plan
   ├─ CPU lowering
   │    ↓
   │  loop/vector IR
   │    ↓
   │  native code
   │
   └─ GPU lowering
        ↓
      kernel IR
        ↓
      CUDA/PTX, Metal, SPIR-V, ...
```

정확한 CPU codegen 기술(직접 Rust kernel, Cranelift, LLVM 등)과 GPU kernel 기술은 별도 실험으로 결정한다. 아키텍처는 특정 compiler library를 전제로 하지 않는다.

## 15. JIT / AOT

컴파일러 형태는 AOT만 의미하지 않는다.

J처럼 동적인 언어에서는 **JIT specialization + cache**가 기본 후보다.

예:

```text
verb
+ input dtype
+ input rank
+ shape/layout class
+ backend/device capability
    ↓
CompileSignature
    ↓
cached compiled artifact
```

모든 정확한 shape를 specialization key로 고정하지 않는다. 동일 code가 여러 shape를 처리할 수 있으면 shape-polymorphic code를 사용한다.

향후 AOT 가능한 명시적 함수/모듈 범위는 별도로 추가할 수 있다.

## 16. Executor의 책임

Executor는 가능한 한 단순해야 한다.

해야 하는 일:

- buffer allocation / reuse
- transfer submit
- compiled kernel launch
- dependency wait / event handling
- completion 후 recycle
- runtime error 전달

하지 않는 일:

- J rank 해석
- shape inference
- fusion 판단
- device 선택
- layout 선택
- tile 선택
- kernel algorithm 선택

요약:

> **Planner thinks. Codegen builds. Executor acts.**

## 17. Explainability

분석과 실행 계획을 분리했으므로 단계별 inspection을 지원한다.

향후 예:

```text
explain '+/ a * b + c'

Semantic
  ...

J analysis
  frame: [1024]
  cell: [4096]
  reduction: last-axis
  fusion legality: ...

Logical plan
  Map(*)
  Map(+)
  Reduce(+)

Physical plan
  target: GPU0
  layout: dense contiguous
  kernel 1: fused multiply/add/partial reduce
  kernel 2: final reduce
  materialization: final output only
```

이 기능은 optimizer 검증과 사용자 성능 진단 양쪽에 중요하다.

## 18. 테스트 전략

계층별로 독립 테스트한다.

### Frontend / Semantic

```text
source → Semantic IR
```

- token/span
- parsing
- J evaluation order
- name/version
- syntax/error location

### Analyzer

```text
Semantic IR → Logical Plan
```

- rank
- cell/frame
- agreement
- type/shape
- effect/alias
- operation class
- legal/illegal fusion

GPU가 없어도 테스트할 수 있다.

### Planner

```text
Logical Plan → Physical Plan
```

- device selection
- placement
- layout
- materialization
- transfer
- fusion
- work partition

실제 GPU 없이 mock device profile로 plan 생성 테스트가 가능해야 한다.

### Codegen

- generated IR/kernel structural tests
- compiled artifact smoke tests
- CPU/GPU backend별 edge case

### Executor

```text
Physical Plan + Compiled Artifact → result
```

- allocation lifetime
- completion before reuse
- transfer failure
- launch failure
- cancellation/cleanup

### Differential

지원 범위는 계속 고정한 C J reference와 비교한다.

compiler result와 reference interpreter result도 비교한다.

## 19. 참고할 기존 배열/컴파일 시스템

아이디어를 가져오되 J 의미를 대체하지 않는다.

- **NumPy**: shape/strides/offset 기반 view
- **CuPy**: NumPy식 배열 모델의 GPU device 적용
- **PyTorch**: logical tensor와 memory format/layout 선택의 분리
- **JAX**: logical global array, placement/sharding, device-local layout 분리
- **ArrayFire**: lazy expression과 kernel fusion
- **Julia GPUArrays**: 공통 array abstraction과 복수 GPU backend
- **J**: rank/cell/frame/agreement와 ordered array semantics

공식 참고:
- https://numpy.org/doc/stable/reference/arrays.ndarray.html
- https://docs.cupy.dev/en/stable/reference/generated/cupy.ndarray.html
- https://docs.pytorch.org/docs/stable/tensor_attributes.html
- https://docs.jax.dev/en/latest/notebooks/Distributed_arrays_and_automatic_parallelization.html
- https://arrayfire.org/docs/gettingstarted.htm
- https://juliagpu.github.io/GPUArrays.jl/stable/
- https://www.jsoftware.com/help/primer/frame_and_cell.htm

## 20. 구현 단계

현재 직접 평가 CPU 엔진을 한 번에 폐기하지 않는다. 동작을 유지하면서 compiler pipeline을 옆에 세운다.

### C0 — 현재 의미 기준선 유지

- 현 CPU 엔진과 C 차등 검증 유지
- J 기능 지원 확대 시 semantic regression 우선
- 현재 `Value`, `CpuStorage`, rank/reduction 구현에서 의미/물리 책임이 섞인 위치 기록

### C1 — Semantic IR과 primitive contract

- parser 결과와 실행을 분리
- span/name/version 보존
- primitive contract를 Rust 자료구조로 도입
- 기존 직접 실행과 IR 생성 결과를 함께 검증

### C2 — Jaxa Analyzer와 Logical Plan

- rank/cell/frame/agreement를 명시적 분석 결과로 생성
- `RankMap`, Map, Reduce, Scan, Gather, Structural logical op 도입
- analyzer가 실행 코드를 호출하지 않도록 경계 확립
- `explain logical` 수준 inspection 추가

### C3 — Physical Planner와 CPU compiler backend

- ArrayView(strides/offset) 및 logical/physical value 분리
- liveness / BufferId / materialization 계획
- 먼저 CPU를 대상으로 physical plan 생성
- 기존 CPU kernel을 compiled-plan backend로 재사용할 수 있는 adapter 도입
- 이후 loop/vector IR과 JIT codegen 후보 평가

이 단계에서 compiler pipeline이 실제 기본 실행 경로가 되는 것을 목표로 한다.

### C4 — GPU physical array와 GPU codegen

- device buffer / placement / transfer
- strided view와 planner-controlled layout
- dense/tiled 선택
- GPU kernel IR / codegen
- frame/cell 기반 work partition
- GPU 상주 연속 실행
- 실제 지원 GPU에서 correctness/performance gate

현재 GPU 구현 보류 정책이 유지되는 동안 C4는 설계·mock-plan 테스트까지만 진행할 수 있다.

### C5 — specialization / cache / multi-device

- JIT CompileSignature
- compiled artifact cache
- shape-polymorphic specialization
- multi-GPU sharding
- backend cost model 측정 보정
- remote/distributed backend 가능성 검토

## 21. 고정할 설계 규칙

1. JArray에는 GPU 실행 정보를 넣지 않는다.
2. J semantics를 tensor framework의 broadcasting 규칙으로 교체하지 않는다.
3. rank/cell/frame/agreement는 Semantic/Analyzer 계층에서 확정한다.
4. Jaxa Analyzer는 실행하지 않고 Logical Plan을 만든다.
5. logical ValueId와 physical BufferId를 구분한다.
6. strides/offset/layout/tiling/placement/sharding은 physical 계층에 둔다.
7. CPU와 GPU는 동일 Logical Plan에서 출발하는 동등한 backend다.
8. Physical Planner가 device/layout/fusion/materialization/work partition을 결정한다.
9. Codegen은 Physical Plan을 compiled artifact로 만든다.
10. Executor는 계획을 재해석하지 않고 실행한다.
11. interpreter는 reference/debug/fallback이며 중심 아키텍처가 아니다.
12. 안전하지 않은 fusion/reassociation으로 J의 오류·승격·수치 의미를 바꾸지 않는다.
13. 최적화의 성공은 IR 존재가 아니라 실제 correctness, traffic, allocation, memory, latency/throughput 측정으로 판단한다.
14. 현재 구현 상태와 목표 아키텍처를 문서에서 항상 구분한다.

## 22. 관련 문서

- [구현 계획](IMPLEMENTATION-PLAN.md)
- [JAXA 설계 검토](JAXA-REVIEW.md)
- [지속 검증 전략](VALIDATION-STRATEGY.md)
- [M2 재검토](M2-REVIEW.md)
- [Rust 배열 프로젝트 조사](RUST-ARRAY-REFERENCES.md)

이 문서가 이후 compiler/array/runtime 구조 변경의 기준 아키텍처다.
