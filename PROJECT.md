# RustJ 통합 프로젝트 문서

> 상태: **유일한 권위 문서(authoritative project document)**  
> 기준일: 2026-09-30  
> 앞으로 아키텍처, 설계 결정, 구현 계획, 지원 범위, 진행 상태, 검증 정책과 주요 검증 결과는 이 문서에 통합한다.  
> 개별 설계 보고서·진행 보고서·체크리스트 Markdown 파일은 새로 만들지 않는다. 기계가 생성한 측정 원자료(JSON/JSONL)는 `reports/`에 별도로 보존한다.

## 1. 프로젝트 목적

RustJ는 `jsource`의 C 구현을 줄 단위로 Rust로 번역하는 프로젝트가 아니다.

목표는 다음과 같다.

> **J의 언어·배열 의미론을 보존하면서, CPU와 GPU를 동등한 실행 대상으로 삼는 현대적인 배열 컴파일러를 Rust로 구현한다.**

`jsource`는 다음을 위한 reference implementation이다.

- J의 관찰 가능한 언어 의미
- primitive의 corner case
- rank/cell/frame/agreement
- 오류와 타입 승격
- 배열 동작과 이름 의미
- 차등 테스트 oracle
- CPU 구현·메모리 정책의 비교 대상

C J 엔진을 RustJ의 정상 실행 fallback으로 사용하지 않는다.

현재 구현은 목표 compiler pipeline 전체를 완성한 상태가 아니다. 제한된 J frontend와 CPU 직접 실행 경로, Semantic IR, 초기 분석/LogicalPlan, CPU storage/SIMD, sparse/boxed 기초, 읽기 전용 affine PhysicalArray가 함께 존재하는 **전환 단계**다.

---

## 2. 최상위 아키텍처

채택한 목표 파이프라인은 다음과 같다.

```text
J Source
   ↓
──────────────── RustJ frontend ────────────────
Scanner / Lexer / Parser
   ↓
J Semantic IR
   ↓
J semantic lowering
   - verb/adverb/conjunction
   - hook/fork/train
   - rank/cell/frame/agreement
   - J name/binding semantics
   - J error/promotion semantics
   ↓
Array IR
──────────────── architectural boundary ───────
   ↓
──────────────────── Jaxa ──────────────────────
Array analysis
   ↓
Logical Execution Plan
   ↓
Logical optimization
   ↓
Physical Planner / Optimizer
   ↓
Physical Execution Plan
──────────────── architectural boundary ───────
   ↓
────────────── Backend / Runtime ───────────────
Backend lowering / code generation
   ├─ CPU
   ├─ CUDA
   ├─ Metal
   ├─ Vulkan / SPIR-V
   └─ future backends
   ↓
Runtime / Executor
```

핵심 원칙은 **J의 의미, 배열 계산, 물리 실행을 서로 다른 계층으로 분리하는 것**이다.

### 2.1 RustJ와 Jaxa의 위상

RustJ와 Jaxa는 개발상 같은 저장소·같은 Cargo workspace에서 함께 구현해도 된다. 그러나 논리적 책임은 분리한다.

- **RustJ**: J 언어를 이해한다.
- **Array IR**: J 고유 표기를 제거한 배열 계산을 표현한다.
- **Jaxa**: 배열 계산을 분석·최적화하고 논리/물리 실행 계획으로 변환한다.
- **Backend/Runtime**: 계획을 backend code로 낮추고 실행한다.

이를 한 문장으로 정의하면:

> **RustJ는 J 언어 구현이고, Jaxa는 Array IR을 입력으로 받는 배열 컴파일러 middle-end다.**

Jaxa가 RustJ parser, J token, hook/fork, adverb/conjunction 같은 J frontend 구조에 의존해서는 안 된다.

의존성 방향은 다음을 지향한다.

```text
RustJ frontend
      ↓
   Array IR
      ↓
     Jaxa
      ↓
Backend / Runtime
```

역방향 의존성은 허용하지 않는다.

```text
Jaxa ─X→ RustJ parser
Jaxa ─X→ J token
Jaxa ─X→ hook/fork/adverb/conjunction
```

### 2.2 물리적으로 함께, 논리적으로 독립

초기에는 경계가 계속 바뀌므로 별도 GitHub repository로 분리하지 않는다.

권장 장기 형태는 같은 workspace 안의 모듈 또는 crate 경계다.

```text
rustj/
  rustj-frontend
  array-ir
  jaxa
  backend-cpu
  backend-gpu
  runtime
```

실제 crate 분리는 인터페이스가 안정된 뒤 진행한다. **repository 분리보다 dependency 방향과 API 경계가 우선**이다.

---

## 3. J Semantic IR과 Array IR

### 3.1 J Semantic IR

J Semantic IR은 “이 J 프로그램이 무엇을 의미하는가”를 표현한다.

여기에는 J 고유 의미가 남을 수 있다.

- verb / adverb / conjunction
- hook / fork / train
- rank
- cell / frame
- agreement
- name binding/version
- J의 오류 순서
- J의 승격 규칙
- source span

반대로 아직 다음 물리 정보는 없어야 한다.

- physical stride
- physical offset
- device placement
- tile size
- CUDA block/thread
- shared memory
- stream/event
- concrete buffer allocation

### 3.2 Array IR

Array IR은 RustJ frontend와 Jaxa 사이의 **정식 컴파일러 경계**다.

Array IR의 목표는 다음과 같다.

1. J의 구문을 제거한다.
2. 배열 계산의 의미는 보존한다.
3. backend의 물리적 선택은 포함하지 않는다.
4. J 이외의 frontend도 이론적으로 생성할 수 있는 형태를 유지한다.

대표 operation 예:

```text
Map
Zip / Elementwise
MapCells
Reduce
Scan
Reshape
Transpose
Reverse
Slice
Take / Drop
Concatenate
Gather
Scatter
Iota
Structural
CallCustomPrimitive
ReadState / WriteState
```

예를 들어 J의

```j
+/ *: y
```

는 J semantic lowering 이후 개념적으로

```text
Reduce(
  op = Add,
  input = Map(op = Square, y)
)
```

처럼 표현할 수 있다.

J의 rank 계산은 Array IR에 “J rank 문법”으로 남기기보다, frontend에서 계산한 명시적 cell/frame mapping을 `MapCells` 같은 일반 배열 연산으로 낮추는 것을 기본 방향으로 한다.

Array IR에 다음과 같은 J 구문 전용 operation을 넣지 않는다.

```text
JHook
JFork
JAdverbSlash
JConjunction
```

또한 다음 물리 정보도 넣지 않는다.

```text
CudaBlockSize
Tile128
GpuSharedMemory
PhysicalStride
DeviceTransfer
```

### 3.3 현재 구현과 목표 경계의 차이

현재 코드는 아직 이 경계를 완전히 물리적으로 구현하지 않았다.

현재 주요 모듈:

- `src/scanner.rs`: word formation
- `src/syntax.rs`: token 변환
- `src/semantic.rs`: Semantic IR과 binding 기초
- `src/contracts.rs`: primitive contract
- `src/facts.rs`: dtype/shape/rank facts
- `src/analysis.rs`: 현재 Semantic IR에서 LogicalPlan을 직접 생성하는 초기 분석기
- `src/physical.rs`: BufferId/BufferLease/PhysicalArray 기초
- `src/runtime.rs`: 제한된 직접 실행 경로
- `src/kernels.rs`, `src/numeric.rs`, `src/simd.rs`: CPU 실행
- `src/storage.rs`: CPU storage
- `src/sparse.rs`, `src/bit_storage.rs`: 추가 storage 표현

따라서 다음 compiler 구조 작업에서 `Semantic IR → Array IR → Jaxa` 경계를 명시적으로 만들고, 현재 `analysis.rs`의 J-specific lowering과 generic array planning 책임을 분리한다.

---

## 4. Jaxa

Jaxa는 **Array IR을 받아 실행 가능한 논리·물리 계획으로 바꾸는 배열 컴파일러 middle-end**다. Jaxa 자체는 실행기가 아니다.

### 4.1 Jaxa의 책임

- Array IR 검증
- dtype / shape / rank fact 전파
- dependency graph
- effect / alias 분석
- parallel domain 식별
- map / reduce / scan / gather / structural 분류
- fusion 가능성 분석
- materialization 경계 판단
- logical rewrite
- backend capability 확인
- cost input 생성
- Logical Execution Plan 생성
- Physical Planner와 함께 layout/placement/buffer 계획 수립

### 4.2 Jaxa가 하지 않는 일

- J source parsing
- J hook/fork/adverb/conjunction 해석
- J syntax 재해석
- 직접 CPU loop 실행
- 직접 CUDA kernel 실행
- Executor 단계에서 의미론을 다시 판단
- 알 수 없는 정보를 임의로 추측

### 4.3 Unknown 원칙

shape, effect, alias, backend legality, dynamic binding을 알 수 없으면 안전한 값으로 꾸며내지 않고 `Unknown`으로 유지한다.

Unknown은 다음 중 하나가 된다.

- optimization barrier
- runtime guard
- fallback to conservative plan
- 재분석 조건

### 4.4 Primitive contract

Array IR/Jaxa 경계에서 primitive는 최소한 다음 의미 계약을 가져야 한다.

```text
PrimitiveContract
  value:
    dtype_rule
    shape_rule
    cell/map rule

  errors:
    domain
    rank/shape
    overflow/promotion
    observable ordering

  effects:
    pure
    read state
    write state
    I/O
    unknown

  alias:
    overlap
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

GPU block 크기나 tile 크기는 semantic primitive contract에 넣지 않는다.

### 4.5 JAXA 확장 어휘와 `with`

기존 jaxa-analyzer에서 검토한 확장 어휘는 다음 원칙을 유지한다.

- `relu`, `linear`, `conv`: custom computational primitive
- `cast_f32`: explicit semantic dtype operation
- `load`, `store`: logical storage/effect operation
- `emit`: core IR에서는 일반적인 side effect/write로 정규화
- `cp`: 강제 materialize가 아니라 checkpoint/availability requirement로 해석하는 방향
- `with`: computational primitive가 아니라 semantic annotation/binding conjunction

`with`에 optimizer, adjoint, semantic dtype requirement, effect contract 같은 의미 정보는 연결할 수 있지만 다음 물리 정책은 넣지 않는다.

- CUDA block/thread
- tile 크기
- shared-memory/register budget
- 특정 device
- backend-specific layout
- stream/event 배치

backend 이름을 primitive identity에 박는 `conv_cuda_` 같은 방식은 일반 vocabulary로 사용하지 않는다. 필요하면 명시적 `BackendIntrinsic` escape hatch로 격리한다.

---

## 5. Logical Plan, Physical Plan, Executor

### 5.1 Logical Execution Plan

Logical Plan은 어떤 배열 계산을 어떤 의존관계로 수행해야 하는지를 나타낸다.

예:

```text
Input y
  ↓
Map Square
  ↓
Reduce Add
```

아직 특정 device buffer 주소나 CUDA launch parameter는 없다.

Logical Plan에서 보존할 수 있는 정보:

- ValueId
- operation
- dtype/shape facts
- explicit cell mapping
- dependency
- effect boundary
- materialization requirement
- fusion candidate
- backend support facts

### 5.2 Physical Plan

Physical Plan은 “어떻게 실행할 것인가”를 결정한다.

- CPU/GPU placement
- buffer binding
- view
- materialize
- contiguous/fixed/general stride specialization
- layout
- tiling
- transfer
- synchronization
- buffer reuse
- work partition
- backend kernel/library 선택

### 5.3 Executor

Executor는 이미 정해진 Physical Plan을 수행한다.

Executor가 다음을 다시 판단해서는 안 된다.

- J rank 의미
- hook/fork 의미
- fusion 여부
- layout 선택
- device 선택
- buffer reuse legality

---

## 6. 논리 배열과 물리 배열

### 6.1 논리 J 배열

J의 noun 의미는 다음으로 유지한다.

```text
JArray
  type
  shape
  ordered atoms / logical value
```

다음은 논리 JArray의 identity가 아니다.

- stride
- offset
- physical layout
- CPU/GPU device
- tile shape
- alignment
- sharding
- CUDA block/thread

### 6.2 ValueId와 BufferId

`ValueId`와 `BufferId`는 다르다.

- `ValueId`: 계산의 논리 결과
- `BufferId`: 특정 물리 저장 공간

하나의 ValueId가 CPU와 GPU의 여러 representation을 가질 수 있다.

반대로 서로 수명이 겹치지 않는 여러 ValueId가 같은 BufferId를 재사용할 수도 있다.

### 6.3 PhysicalArray

개념 모델:

```rust
PhysicalArray {
    storage / buffer,
    shape,
    strides,
    offset,
    encoding,
    placement,
    layout,
}
```

현재 `src/physical.rs`에는 다음 G1 기초가 구현되어 있다.

- scoped/generation `BufferId`
- `BufferLease`
- CPU backing registry
- dtype/encoding 확인
- signed strides / offset
- checked address span
- empty/scalar 처리
- read-only affine `PhysicalArray`
- standard-layout 판정
- logical slice의 제한적 노출

### 6.4 CpuStorage의 위치

현재 `CpuStorage<T>`의 Inline / Owned / Shared 구조는 버리지 않는다.

장기적으로는 J semantic Value의 유일한 저장 방식이 아니라 **CPU backend physical storage**로 재배치한다.

```text
ValueId
  ↓
Physical representation
  ↓
Storage
  ├─ CpuStorage
  ├─ CudaStorage
  ├─ MetalStorage
  └─ ...
```

---

## 7. GPU 친화적 배열 설계

GPU 친화성 때문에 J의 언어 의미를 tensor framework의 broadcasting 규칙으로 바꾸지 않는다.

### 7.1 stride / offset view

가능하면 다음 structural operation을 metadata-only view로 표현한다.

- transpose
- reverse
- slice
- fill 없는 take/drop
- 순서 호환 reshape

예:

```text
shape   [100, 200]
strides [200, 1]

transpose

shape   [200, 100]
strides [1, 200]
```

logical transpose와 physical copy는 같은 것이 아니다.

### 7.2 논리 순서와 메모리 연속성

다음 둘을 구분한다.

- logical-order contiguous
- memory-contiguous

transpose된 backing이 물리적으로 연속 영역을 공유하더라도 J ravel 순서가 다르면 기존 dense kernel에 그대로 slice로 넘길 수 없다.

### 7.3 zero stride

J agreement를 먼저 계산한다.

그 결과 같은 atom/cell을 반복 읽는 구현이 합법적이면 physical lowering에서 zero stride를 사용할 수 있다.

zero stride가 J agreement 규칙을 정의하는 것은 아니다.

### 7.4 dense / tiled / sparse / packed

모든 표현을 affine byte-stride 모델 하나로 강제하지 않는다.

구분한다.

- affine dense
- tiled
- sparse
- boxed
- packed bit
- backend-specific encoding

초기 G1은 read-only affine dense만 다룬다.

### 7.5 placement / sharding

logical shape와 physical placement를 분리한다.

```text
Placement
  Cpu
  Device(DeviceId)
  Sharded(ShardingId)
```

multi-GPU shard가 있어도 사용자에게 보이는 J noun은 하나의 logical value다.

### 7.6 GPU 실행 원칙

GPU에서는 primitive별 즉시 실행보다 전체 계산을 본 뒤 계획해야 한다.

예:

```j
+/ *: y
```

를

```text
square kernel
→ intermediate GPU array
→ reduction kernel
```

로 고정하지 않는다.

가능하면

```text
Reduce(Add, Map(Square, y))
  ↓
legal fusion analysis
  ↓
fused map-reduction kernel
```

처럼 계획한다.

CUDA 실제 구현은 현재 보류 상태다. CPU에서 physical representation과 plan/executor 경계를 먼저 검증한다.

---

## 8. 메모리·alias·수명 원칙

### 8.1 읽기/쓰기 분리

공유, zero-stride, 내부 overlap 가능 view는 기본적으로 read-only다.

첫 mutable 경로는 다음을 만족하는 출력으로 제한한다.

- 독점 소유
- 표준 연속
- non-overlap proof
- dtype/용량 호환
- live alias 없음

### 8.2 주소 검증

비어 있지 않은 affine view는 각 축의 signed delta를 checked 계산하고 접근 가능한 최저/최고 backing index를 확인한다.

span 검증은 접근 가능성을 증명하지만 non-overlap을 증명하지 않는다.

empty view는 원소 주소를 계산하지 않는다.

### 8.3 buffer reuse

buffer reuse는 명시적으로 증명한다.

확인할 것:

- 원래 allocation
- 전체/부분 영역
- offset
- 공유 owner
- live view
- alias
- dtype
- capacity
- alignment
- last physical use

GPU 재개 후에는 kernel 제출과 실제 device completion을 구분한다.

### 8.4 관찰 가능한 J 의미

최적화 때문에 다음을 바꾸지 않는다.

- 전체 배열 overflow promotion
- 오류 순서
- binding의 이전 값 보존
- side effect 순서
- float 연산 순서

FMA, reassociation, reduction 순서 변경은 별도 허용 조건 없이는 자동 적용하지 않는다.

---

## 9. 언어 및 구현 범위

### 9.1 현재 지원하는 주요 값

- Boolean
- i64
- f64
- byte character array
- BigInt / rational / boxed / sparse에 대한 일부 기반 구현

추가 scalar/storage 기반은 존재하지만 전체 J 의미와 모든 primitive 연결이 완료된 것은 아니다.

### 9.2 현재 직접 실행 경로에서 지원하는 대표 기능

- 숫자 scalar/vector literal
- 밑줄 음수
- 소수/지수
- NaN/Infinity literal
- 작은따옴표 문자열
- 우측부터 평가
- 괄호
- `NB.` 주석
- noun binding `=:`
- 기본 `+ - * %`
- monadic `|`
- 비교 `= < >`
- monadic `i.`
- `$ # ,`
- 기본 reshape/index/catenate 일부
- scalar expansion과 제한된 agreement
- `+/ -/ */ %/`
- 정수 하나의 monadic rank
- 일부 array structural verbs
- 일부 index/search verbs
- domain/length/rank/index/value/limit/syntax 오류

### 9.3 배열 조작

구현된 범위에는 다음이 포함된다.

- `|. y`: 첫 축 reverse
- `n |. y`: 첫 축 rotate
- `|: y`: 축 역순 transpose
- `n {. y`: take
- `n }. y`: drop

여러 축의 count list, dyadic transpose, 일반 fill, 모든 고차원 규칙은 아직 완전하지 않다.

### 9.4 index/search 계열

구현된 범위에는 다음이 포함된다.

- `i. y`
- `x i. y`
- `x i: y`
- `i: n`
- `I. y`
- `e.`
- `E.`

다차원·고급 interval/search 의미는 아직 제한적이다.

### 9.5 이름과 품사

중요한 J 의미 원칙:

- undefined name이 항상 즉시 value error인 것은 아니다.
- noun은 binding 시점 snapshot 의미가 필요하다.
- verb 이름은 호출 시점 해석이 필요한 경우가 있다.
- name/version 정보를 IR과 plan guard에 반영해야 한다.
- parser가 깊은 식에서 임의의 작은 recursion/height 한계로 J 의미를 바꾸지 않도록 한다.

### 9.6 direct / explicit definition

direct/explicit definition의 parser·AST·binding·execution은 아직 완전 구현되지 않았다.

GPU 배열 작업과 compiler boundary 정리가 우선이며, 이후 다음 순서로 진행한다.

1. word formation / parser contract
2. definition AST
3. local/name binding
4. verb execution
5. control flow
6. conformance

### 9.7 아직 큰 미지원 영역

- 전체 boxed semantics
- 전체 sparse semantics
- complex
- 모든 extended numeric semantics
- Unicode 전체
- 전체 verb binding/train
- 전체 adverb/conjunction
- 전체 dyadic rank와 rank list
- scan 전체
- 모든 system foreign
- file API
- serialization
- embedding ABI
- 완전한 parallel execution
- 완전한 CUDA backend

---

## 10. 구현 계획과 체크리스트

이 절이 앞으로 유일한 구현 체크리스트다.

### A0 — 문서/아키텍처 경계

- [x] RustJ/Jaxa의 논리적 경계를 확정한다.
- [x] J Semantic IR과 Array IR을 구분한다.
- [x] Jaxa를 generic array compiler middle-end로 정의한다.
- [x] 문서를 `PROJECT.md`로 통합한다.
- [ ] 실제 코드 dependency에서도 RustJ frontend → Array IR → Jaxa 경계를 만든다.

### A1 — 명시적 Array IR

- [ ] 최소 `ArrayProgram` / `ArrayOp` / `ArrayValueId`를 정의한다.
- [ ] J syntax 타입에 의존하지 않는 operation 집합을 정의한다.
- [ ] dtype/shape/effect/error contract를 연결한다.
- [ ] `semantic.rs`에서 Array IR lowering을 구현한다.
- [ ] rank/cell/frame/agreement를 explicit mapping으로 낮춘다.
- [ ] 현재 `analysis.rs`의 J-specific responsibility를 frontend lowering으로 옮긴다.
- [ ] Jaxa 분석 코드는 Array IR만 입력받게 한다.
- [ ] Array IR을 직접 구성한 단위 테스트로 Jaxa를 독립 검증한다.

완료 조건: Jaxa 모듈을 J parser/semantic AST 없이 테스트할 수 있다.

### G1 — 논리 값과 물리 표현의 경계

상태: **완료**

- [x] ValueId와 BufferId 분리 기초
- [x] generation/scoped BufferId
- [x] BufferLease
- [x] read-only affine PhysicalArray
- [x] shape/strides/offset
- [x] encoding/dtype/backing 검증
- [x] signed span checking
- [x] empty/scalar policy
- [x] standard logical layout 판정
- [x] Windows 기본/portable 회귀

아직 evaluator/LogicalPlan 실행 경로와 완전히 연결된 것은 아니다.

### G2 — 복사 없는 structural view

- [ ] transpose stride permutation
- [ ] reverse negative stride
- [ ] fill 없는 take/drop/slice view
- [ ] compatible reshape metadata-only
- [ ] agreement 결과의 zero-stride lowering
- [ ] logical-order materialization
- [ ] backing identity/payload copy 테스트
- [ ] transpose→reverse→slice 조합 테스트

### G3 — CPU kernel 및 cell mapping 연결

- [ ] contiguous / fixed-stride / general-stride 경로
- [ ] add를 첫 실제 연결 operation으로 사용
- [ ] SIMD contiguous fast path 유지
- [ ] explicit cell mapping과 physical view 연결
- [ ] alias proof 없는 write/reuse 금지
- [ ] overflow/promotion/error order 보존
- [ ] NaN/Inf/signed zero/empty 테스트

### G4 — 최소 Physical Plan과 CPU Executor

- [ ] Buffer bind
- [ ] View
- [ ] Materialize
- [ ] Kernel call
- [ ] Output ownership
- [ ] last physical use
- [ ] buffer reuse proof
- [ ] layout-compatible view 유지
- [ ] CPU executor
- [ ] source → semantic → array IR → Jaxa → physical → CPU end-to-end

### G5 — 성능 및 확장 경계

- [ ] structural view 생성 비용
- [ ] copy/allocation/peak/retained bytes
- [ ] general-stride indexing 비용
- [ ] materialization 비용 비교
- [ ] contiguous 기존 성능 회귀 확인
- [ ] Windows default/portable 전체 회귀
- [ ] 지원 layout/type/operation 표 갱신
- [ ] tiled/placement/transfer/completion 확장 경계 확인

### C — frontend / 언어 의미 확장

GPU 배열과 compiler boundary의 정확성을 막는 frontend 결함은 즉시 수정한다. 그 외 확장은 G1~G5와 A1 뒤에 진행한다.

- [ ] primitive registry/binding contract 보완
- [ ] direct/explicit definition
- [ ] verb binding/train
- [ ] adverb/conjunction 확대
- [ ] dyadic rank/rank list
- [ ] scan
- [ ] boxed/sparse 전체 의미
- [ ] scalar type 확대
- [ ] system/runtime API

### CUDA — 보류

- [ ] CUDA storage
- [ ] transfer/completion
- [ ] stream/event
- [ ] kernel codegen
- [ ] GPU resident graph
- [ ] sharding/multi-device

재개 조건: 사용자 요청과 검증 가능한 GPU 환경 확보.

---

## 11. 검증 정책

모든 구현 변경은 이 절을 따른다.

### 11.1 기본 완료 관문

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test --features portable
```

관련 Python harness가 바뀌면 해당 테스트도 실행한다.

### 11.2 semantic bug fix

- 재현 사례 추가
- regression test 추가
- 필요하면 conformance case 추가
- C reference와 비교 가능한 경우 비교
- known deviation과 pass를 분리

### 11.3 storage / memory 변경

확인 항목:

- alias preservation
- transactional assignment
- allocation/reuse
- retained memory
- empty/scalar
- stale BufferId
- overlap
- source scope 이후 owner 수명

### 11.4 SIMD 변경

- tail
- overflow/promotion
- exceptional float
- portable fallback
- runtime feature detection

### 11.5 실행했다고 주장할 수 있는 것만 기록

실제로 실행하지 않았다면 실행한 것으로 기록하지 않는다.

현재 정책상 다음은 자동으로 완료됐다고 간주하지 않는다.

- upstream 전체 J suite
- Miri
- sanitizer
- Linux CI
- GitHub Actions CI
- GPU test
- CUDA benchmark

### 11.6 C reference

참조 `jsource` revision은 검증 재현성을 위해 고정한다.

C reference는 별도 프로세스/벤치마크 경로에서 oracle로 사용하며 RustJ runtime dependency로 사용하지 않는다.

### 11.7 성능 해석

작은 배열에서 Python/FFI overhead가 섞인 숫자를 언어 성능으로 해석하지 않는다.

성능 비교에는 다음을 분리한다.

- parse/compile
- execution
- allocation
- copy bytes
- retained bytes
- peak live bytes
- cold/warm
- cache hit/miss
- transfer
- synchronization

`Rust가 J보다 빠르다` 같은 전체적 결론은 제한된 microbenchmark만으로 주장하지 않는다.

---

## 12. 현재 검증·구현 상태 요약

기준일: 2026-09-30.

- 제한된 CPU J 실행 경로가 동작한다.
- Semantic IR parser가 존재한다.
- source span과 name/version 기초가 있다.
- primitive contract와 dtype/shape/rank facts 기초가 있다.
- 초기 LogicalPlan 생성이 있다.
- CPU storage Inline/Owned/Shared가 있다.
- AVX2 runtime dispatch와 portable fallback이 있다.
- sparse/boxed/packed-bit 관련 기반 구현이 일부 있다.
- G1 read-only affine PhysicalArray가 구현되어 있다.
- G1 Windows default/portable 회귀와 Clippy 기록이 있다.
- G2~G5는 미완료다.
- 명시적 Array IR 경계는 아직 코드에 없다.
- Jaxa가 Array IR만 소비하는 dependency 경계도 아직 코드에 없다.
- 실제 CUDA storage/kernel은 없다.
- GitHub CI는 현재 사용하지 않는다.
- 이 컴퓨터에서는 Windows 네이티브 검증을 기준으로 한다.

기계 측정 원자료는 `reports/*.json`, `reports/*.jsonl`에 보존한다.

---

## 13. 프레임워크 조사에서 채택한 원칙

외부 프레임워크의 언어 의미를 가져오는 것이 아니라 검증된 구현 아이디어를 참고한다.

### ndarray

참고:

- logical order와 memory order 구분
- checked view construction
- shape/stride 기반 view

RustJ 적용:

- logical-order contiguous와 memory-contiguous 구분
- checked affine descriptor
- reshape compatibility proof

### Apache Arrow

참고:

- buffer ownership
- offset/shared backing
- allocation layout

RustJ 적용:

- BufferLease
- shared backing 수명
- 부분 view와 allocation owner 구분

### OpenXLA

참고:

- logical IR과 buffer assignment 분리
- layout conflict의 copy
- backend-specific lowering

RustJ 적용:

- ValueId/BufferId 분리
- Logical/Physical Plan 분리
- layout conflict materialization

### ArrayFire / fusion systems

참고:

- lazy graph
- evaluation boundary
- kernel fusion

RustJ 적용:

- Array IR graph
- 합법적인 fusion
- materialization 최소화

### JAX / multi-device systems

참고:

- logical/global value와 placement/sharding 분리

RustJ 적용:

- logical J noun과 physical placement 분리

### 중요한 비채택 사항

- NumPy broadcasting을 J agreement로 대체하지 않는다.
- tensor framework dtype policy를 J type semantics로 대체하지 않는다.
- backend layout을 J noun identity로 만들지 않는다.
- framework의 JIT 성공 사례를 RustJ 성능 증거로 취급하지 않는다.

---

## 14. jsource와 RustJ의 관계

현재 jsource는 CPU 실행을 전제로 한 C 구현이다.

문제는 C라는 언어 자체보다 다음 구조적 결합이다.

- C pointer 중심 배열 접근
- CPU address space 가정
- primitive 호출과 즉시 실행의 결합
- CPU allocation과 logical value의 결합
- 전체 graph 최적화 이전의 materialization
- backend-independent execution plan 부재

따라서 jsource를 그대로 Rust로 번역하면 주로 다음이 된다.

> CPU-oriented J interpreter written in Rust

RustJ의 목표는 다르다.

```text
jsource
  → semantic/reference oracle

RustJ
  J semantics
    ↓
  Array IR
    ↓
  Jaxa
    ↓
  CPU / GPU plans
```

jsource에서 적극적으로 가져올 것:

- tokenizer/parser behavior
- primitive semantics
- rank/agreement
- type promotion
- boxing/sparse rules
- error semantics
- corner cases
- tests

그대로 상속하지 않을 것:

- A block의 물리 메모리 정체성
- pointer 기반 primitive architecture
- CPU loop structure
- CPU-only allocation strategy
- primitive 즉시 실행 dispatch
- backend-specific in-place machinery

---

## 15. 문서 정책

### 15.1 권위 문서

앞으로 사람이 유지하는 프로젝트 기준 문서는 **이 `PROJECT.md` 하나**다.

변경 시 함께 갱신할 항목:

- 아키텍처 결정
- interface/boundary
- 구현 계획
- 체크리스트
- current status
- 지원 범위
- validation policy
- 주요 검증 결과

### 15.2 README

`README.md`는 다음만 담당한다.

- 프로젝트 한 줄 설명
- 빌드/실행 방법
- 현재 지원 범위의 짧은 요약
- `PROJECT.md` 링크

README에 별도의 상세 설계 사본을 만들지 않는다.

### 15.3 reports/

`reports/`는 앞으로 **기계 생성 또는 실측 원자료** 중심으로 사용한다.

허용 예:

- benchmark JSON
- allocation JSONL
- conformance result
- measurement raw data

새로운 사람이 편집하는 설계/체크리스트/진행 Markdown 보고서는 만들지 않는다.

### 15.4 이번 통합에서 흡수한 기존 문서군

다음 범주의 기존 Markdown은 이 문서로 통합하고 별도 파일을 유지하지 않는다.

아키텍처·설계:
- COMPILER-ARCHITECTURE
- JAXA-REVIEW
- MEMORY-POLICY
- ARRAY-FRAMEWORK-DESIGN-REVIEW
- GPU-ARRAY-DESIGN-AUDIT
- RUST-ARRAY-REFERENCES

계획·진행:
- IMPLEMENTATION-PLAN
- IMPLEMENTATION-CHECKLIST
- GPU-FRIENDLY-ARRAY-PLAN
- COMPILER-IR-PROGRESS
- C1-PROGRESS
- DEFINITION-IMPLEMENTATION-PLAN

언어·기능·감사:
- C-J-FEATURE-AUDIT
- LEXER-AUDIT
- NAME-SEMANTICS-AUDIT
- DEFINITION-PARSING-AUDIT
- ARRAY-VERBS
- INDEX-VERBS
- DYADIC-RANK
- SCALAR-TYPES
- SPARSE-ARRAYS
- BIT-STORAGE

검증·이력:
- VALIDATION-STRATEGY
- MILESTONE-1
- MILESTONE-2
- M2-POOL
- M2-REVIEW
- PERFORMANCE
- PHYSICAL-ARRAY-G1

세부 역사 원문은 Git history에서 계속 조회할 수 있고, 측정 원자료는 `reports/`에 남긴다.

---

## 16. 다음 작업

현재 가장 먼저 해야 할 compiler architecture 작업은 **명시적 Array IR 경계를 코드에 만드는 것**이다.

순서:

1. 현재 `semantic.rs`, `analysis.rs`, `facts.rs`, `contracts.rs`의 책임을 다시 분류한다.
2. 최소 Array IR 자료구조를 정의한다.
3. J Semantic IR → Array IR lowering을 구현한다.
4. Jaxa 분석기가 Array IR만 읽게 한다.
5. Jaxa 단독 unit test를 추가한다.
6. 기존 LogicalPlan 결과와 의미 동등성을 비교한다.
7. 그 경계를 유지하면서 G2 structural view 작업을 계속한다.

이 원칙을 깨는 임시 shortcut을 추가하지 않는다.
