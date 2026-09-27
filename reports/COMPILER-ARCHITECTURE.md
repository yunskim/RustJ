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

### 7.2 Unknown은 추측하지 않는다

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
