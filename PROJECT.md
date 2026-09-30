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

### 1.1 이름 정책

현재 아키텍처에는 `Jaxa`라는 별도 compiler component 이름을 두지 않는다.

과거 `jaxa-analyzer` 저장소와 문서에서 발전한 아이디어는 RustJ의 semantic analysis, logical lowering, resource/fusion planning 설계에 흡수한다. 그러나 현행 설계에서 별도 제품·crate·subsystem의 정체성을 뜻하지 않는다.

현행 용어는 다음으로 통일한다.

```text
Jaxa Analyzer      → Semantic Analyzer
Jaxa lowering      → Semantic / Logical Lowering
Jaxa optimizer     → Logical Optimizer
Jaxa physical plan → Physical Planner / Physical Plan
```

필요할 때만 `jaxa-analyzer`를 **historical research/prototype repository**라는 의미로 언급한다.


---

## 2. 최상위 아키텍처

채택한 목표 파이프라인은 다음과 같다.

```text
J Source
   ↓
──────────────── RustJ frontend ────────────────
Scanner / Lexer / Parser
   ↓
J Semantic Array IR
   │
   │ noun / verb / adverb / conjunction
   │ primitive / derived verb
   │ hook / fork / train
   │ rank and other modifier applications
   │ name/binding/version
   │ source span
   ↓
──────── Semantic Analyzer / Lowering ──────────
Semantic analysis of array transformations
   ↓
Logical Array IR / Logical Execution Plan
   │
   │ explicit dataflow
   │ cell/frame mapping
   │ map / reduce / scan / gather / structural
   │ dependency / effects / alias facts
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

핵심 원칙은 **J의 고수준 배열 변환 구조를 Semantic Analyzer가 보기 전에 없애지 않고, analyzer/lowering 단계가 그 구조를 분석한 뒤 backend-independent logical dataflow로 낮추는 것**이다.

### 2.1 RustJ compiler stage의 위상

RustJ는 하나의 compiler system으로 개발한다. 별도 고유 컴포넌트명을 두기보다 각 compiler stage의 책임을 명확히 분리한다.

- **RustJ frontend**: source text를 읽고 J의 품사·결합·이름 의미를 보존한 `J Semantic Array IR`을 만든다.
- **J Semantic Array IR**: J의 배열 계산을 고수준에서 표현한다. hook/fork/train, adverb/conjunction으로 만든 derived verb, rank 같은 의미 구조를 보존한다.
- **Semantic Analyzer / Lowering**: 이 고수준 IR을 분석하여 explicit dataflow와 array operation으로 이루어진 `Logical Array IR / Logical Execution Plan`으로 낮춘다.
- **Physical Planner**: layout, placement, materialization, buffer, transfer, scheduling 같은 물리 실행 결정을 내린다.
- **Backend/Runtime**: 계획을 backend code로 낮추고 실행한다.

이를 한 문장으로 정의하면:

> **RustJ는 frontend부터 semantic analysis/lowering, logical optimization, physical planning, backend, runtime까지 포함하는 하나의 compiler system이다.**

Semantic Analyzer / Lowering은 J source text나 tokenizer/parser mechanics에 의존하지 않는다. 그러나 다음 J 의미 구조는 **분석 입력이며 제거 대상이 아니다.**

- noun / verb / adverb / conjunction
- primitive와 derived verb
- hook / fork / train
- rank 및 modifier application
- monad / dyad valence
- name/binding/version이 의미에 영향을 주는 경우의 metadata
- primitive semantic contract

의존성 방향은 다음을 지향한다.

```text
RustJ source frontend
        ↓
J Semantic Array IR
        ↓
Semantic Analyzer / Lowering
        ↓
Logical Array IR / Plan
        ↓
Physical Plan
        ↓
Backend / Runtime
```

금지하는 역방향 의존성은 source frontend 구현 세부에 대한 것이다.

```text
Semantic Analyzer ─X→ scanner implementation
Semantic Analyzer ─X→ token stream layout
Semantic Analyzer ─X→ parser stack mechanics
Semantic Analyzer ─X→ source-text reparsing
```

반대로 `Hook`, `Fork`, `Train`, `DerivedVerb`, `Rank` 같은 **semantic IR node를 Semantic Analyzer가 아는 것은 의도된 설계**다.

### 2.2 물리적으로 함께, 논리적으로 독립

초기에는 경계가 계속 바뀌므로 별도 GitHub repository로 분리하지 않는다.

권장 장기 형태는 같은 workspace 안의 모듈 또는 crate 경계다.

```text
rustj/
  rustj-frontend
  j-semantic-ir
  semantic-analysis
  logical-array-ir
  backend-cpu
  backend-gpu
  runtime
```

실제 crate 분리는 인터페이스가 안정된 뒤 진행한다. **repository 분리보다 dependency 방향과 API 경계가 우선**이다.

---

## 3. J Semantic Array IR

### 3.1 J의 계산 모델

J의 계산은 배열 중심이다.

- noun은 array value다.
- monadic verb는 개념적으로 `Array → Array`다.
- dyadic verb는 개념적으로 `Array × Array → Array`다.
- adverb와 conjunction은 verb 등 J entity를 받아 새로운 derived entity를 만든다.
- hook/fork/train은 verb를 합성하여 새로운 verb를 만든다.

따라서 compiler가 보존해야 하는 것은 단순한 배열 값뿐 아니라 **array transformer의 합성 구조**다.

예를 들어:

```j
(+/ % #) y
```

를 semantic analysis 전에 곧바로

```text
Divide(Reduce(Add, y), Tally(y))
```

로만 바꾸면 최종 데이터 의존성은 남지만, 원래의 fork 구조가 가진 분석 정보를 잃는다.

고수준 IR에서는 개념적으로 다음처럼 보존할 수 있다.

```text
Apply
├─ verb:
│   Fork
│   ├─ DerivedVerb(Insert, Add)
│   ├─ Divide
│   └─ Tally
└─ argument:
    y
```

Semantic Analyzer가 이 구조를 분석한 뒤에야 explicit dataflow로 낮춘다.

### 3.2 jsource에서 확인한 근거

확인 기준: `jsoftware/jsource` master `ce65ed97ec57d95910e9bab4a652e2991d294626` (2026-09-30 확인), 특히 `jsrc/jtype.h`, `jsrc/cf.c`, `jsrc/jc.h`.

현재 jsource의 내부 구현도 derived verb의 구조를 실행 전까지 보존한다.

`jsrc/jtype.h`의 핵심 구조:

```c
typedef struct AD AD;
typedef AD *A;
```

noun, verb, adverb, conjunction 등 J entity는 공통 `A` block 체계에 존재하고 type bit로 품사를 구별한다.

```text
NOUN = numeric + character + box
FUNC = VERB + ADV + CONJ
```

verb/adverb/conjunction의 payload는 `FAV(x)`로 `V` 구조를 읽는다. `V`에는 개념적으로 다음이 있다.

```text
V
├─ fgh[3]          operands / components
├─ valencefns[2]   monad / dyad entry
├─ monad/dyad rank
├─ id
└─ optimization / execution metadata
```

jsource 주석은 `fgh[3]`의 `h`가 fork에 사용된다고 명시한다. `jtfolk`는 “derived verb for a fork”를 만들고 `jthook`도 hook/trident 구조를 derived verb로 만든다. `CFORK`, `CHOOK`, `CADVF` 같은 identity도 유지된다.

또한 function block의 `AN`, `AR` field는 사용하지 않는다고 명시되어 있다. 즉 **noun array와 verb는 같은 J entity allocation 체계를 공유하지만 동일한 noun representation은 아니다.** RustJ는 이 물리 표현을 복제할 필요는 없지만, **derived verb composition을 first-class semantic structure로 보존한다는 점은 중요한 reference**다.

### 3.3 IR이 보존해야 하는 구조

개념 모델:

```text
JEntity
├─ Noun
│   └─ ArrayValue
├─ Verb
│   ├─ PrimitiveVerb
│   ├─ DerivedVerb
│   │   ├─ Hook
│   │   ├─ Fork
│   │   ├─ Train
│   │   ├─ AdverbDerived
│   │   ├─ ConjunctionDerived
│   │   ├─ RankDerived
│   │   └─ other modifier-derived forms
│   ├─ ExplicitVerb
│   ├─ NameRef
│   └─ CustomVerb
├─ Adverb
└─ Conjunction
```

실제 Rust enum을 이 모양 그대로 만들라는 뜻은 아니다. 중요한 것은 다음 invariants다.

1. primitive와 derived verb의 identity를 보존한다.
2. hook/fork/train의 operand 관계를 보존한다.
3. modifier와 operand의 관계를 보존한다.
4. monad/dyad valence를 보존한다.
5. rank가 계산 의미에 미치는 정보를 Semantic Analyzer가 볼 수 있어야 한다.
6. name reference와 binding/version이 의미에 영향을 주면 분석 가능한 형태로 보존한다.
7. source span은 진단을 위해 유지한다.

### 3.4 너무 이른 정규화를 금지한다

다음 변환은 **semantic analysis 전에 무조건 수행하지 않는다.**

```text
Fork(f,g,h)
    → g(f(y), h(y))

Hook(f,g)
    → fully expanded expression

DerivedVerb(/, +)
    → Reduce(Add)

RankDerived(f, r)
    → generic MapCells only
```

이런 정규화는 합법성과 분석 이득이 확인된 뒤 semantic lowering/rewrite 단계에서 수행한다.

이유:

- fork branch의 독립성
- 공통 argument 사용
- derived verb identity
- primitive composition
- rank propagation
- fusion 후보
- custom semantic annotation
- name/binding semantics

을 분석에 사용할 수 있기 때문이다.

### 3.5 물리 정보는 넣지 않는다

고수준 semantic IR에 다음 physical decision은 넣지 않는다.

```text
CudaBlockSize
Tile128
GpuSharedMemory
PhysicalStride
PhysicalOffset
DeviceTransfer
ConcreteBufferId
StreamEvent
```

이들은 Physical Planner/Backend의 책임이다.

### 3.6 현재 구현과 목표 경계의 차이

현재 코드는 아직 이 구조를 완성하지 않았다.

현재 주요 모듈:

- `src/scanner.rs`: word formation
- `src/syntax.rs`: token 변환
- `src/semantic.rs`: 현재 Semantic IR과 binding 기초
- `src/contracts.rs`: primitive contract
- `src/facts.rs`: dtype/shape/rank facts
- `src/analysis.rs`: 현재 Semantic IR에서 LogicalPlan을 직접 생성하는 초기 분석기
- `src/physical.rs`: BufferId/BufferLease/PhysicalArray 기초
- `src/runtime.rs`: 제한된 직접 실행 경로
- `src/kernels.rs`, `src/numeric.rs`, `src/simd.rs`: CPU 실행
- `src/storage.rs`: CPU storage
- `src/sparse.rs`, `src/bit_storage.rs`: 추가 storage 표현

다음 compiler 구조 작업에서는 현재 `semantic.rs`의 표현이 **verb composition을 충분히 보존하는지** 먼저 점검하고, 필요하면 `J Semantic Array IR`을 명시한다. 그 다음 `analysis.rs`를 Semantic Analyzer / Lowering 역할로 정리하여 이 IR을 직접 분석하게 한다.

---

## 4. Semantic Analyzer / Lowering

Semantic Analyzer / Lowering은 **J Semantic Array IR을 분석하여 Logical Array IR / Logical Execution Plan으로 낮추는 RustJ compiler middle-end**다. 이 단계 자체는 실행기가 아니다.

### 4.1 Semantic Analyzer / Lowering의 책임

- noun/verb/adverb/conjunction 품사와 적용 관계 분석
- primitive / derived verb 분석
- hook / fork / train 구조 분석
- modifier application과 rank semantics 분석
- monad / dyad valence 결정
- primitive semantic contract 적용
- dtype / shape / cell / frame / agreement fact 전파
- dependency graph 생성
- effect / alias 분석
- parallel domain 식별
- map / reduce / scan / gather / structural pattern 식별
- derived structure의 합법적인 normalization/lowering
- fusion 가능성 분석
- materialization 경계 판단
- logical rewrite
- backend capability 확인
- cost-model input 생성
- Logical Array IR / Logical Execution Plan 생성

### 4.2 Semantic Analyzer / Lowering이 하지 않는 일

- source text tokenization
- parser stack 규칙의 재실행
- source를 다시 parse하여 의미를 복원
- 직접 CPU loop 실행
- 직접 CUDA kernel 실행
- Executor 단계에서 의미론을 다시 판단
- 알 수 없는 정보를 임의로 추측

즉 Semantic Analyzer는 **J syntax mechanics는 모르지만 J semantic structure는 안다.**

### 4.3 Semantic analysis 이후의 generic 경계

다른 frontend와 공유할 가능성이 높은 지점은 semantic analysis 입력 전이 아니라 **고수준 J 구조를 분석한 뒤 생성하는 Logical Array IR / Plan**이다.

```text
J frontend
    ↓
J Semantic Array IR
    ↓
Semantic Analyzer / Lowering
    ↓
Logical Array IR / Plan  ← generic boundary 후보
    ↓
Physical Planner
```

향후 다른 array DSL frontend를 붙이고 싶다면 두 선택이 가능하다.

1. J semantic model을 의도적으로 공유하면 J Semantic Array IR을 생성한다.
2. J와 무관한 frontend라면 자기 semantic analyzer를 거쳐 Logical Array IR / Plan에 합류한다.

따라서 **middle-end를 generic tensor IR consumer처럼 만들기 위해 J의 구조를 일찍 버리지 않는다.**

### 4.4 Unknown 원칙

shape, effect, alias, backend legality, dynamic binding을 알 수 없으면 안전한 값으로 꾸며내지 않고 `Unknown`으로 유지한다.

Unknown은 다음 중 하나가 된다.

- optimization barrier
- runtime guard
- fallback to conservative plan
- 재분석 조건

### 4.5 Primitive contract

semantic analysis에서 primitive는 최소한 다음 의미 계약을 가진다.

```text
PrimitiveContract
  identity / valence
  value:
    dtype_rule
    shape_rule
    rank_rule

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

### 4.6 과거 jaxa-analyzer 연구에서 가져오는 확장 어휘와 `with`

`jaxa-analyzer` 연구/prototype 저장소에서 검토한 확장 어휘는 RustJ에 흡수할 때 다음 원칙을 유지한다. `JAXA`는 여기서 역사적 연구명일 뿐 현재 RustJ 아키텍처의 별도 컴포넌트명이 아니다.

- `relu`, `linear`, `conv`: custom computational primitive/verb
- `cast_f32`: 품사와 결합 의미를 보존하여 분석
- `load`, `store`: logical storage/effect operation 또는 derived semantic operation
- `emit`, `cp`: 품사와 derived-verb 구조가 분석 의미를 가진다면 IR에 보존
- `with`: 계산 primitive가 아니라 semantic annotation/binding 관계

중요한 원칙은 **확장 어휘도 품사와 composition structure가 분석 정보라면 너무 일찍 평평한 operation으로 만들지 않는 것**이다.

반대로 다음 physical policy는 semantic annotation에 섞지 않는다.

- CUDA block/thread
- tile 크기
- shared-memory/register budget
- 특정 device
- backend-specific layout
- stream/event 배치

backend-specific implementation identity와 semantic verb identity도 분리한다.

---

## 5. Logical Plan, Physical Plan, Executor

### 5.1 Logical Array IR / Logical Execution Plan

이 계층은 Semantic Analyzer가 hook/fork/derived verb/rank 같은 고수준 의미 구조를 분석한 뒤 만든 **명시적 배열 dataflow**다.

예를 들어 고수준의

```text
Apply(Fork(Insert(+), %, #), y)
```

는 분석 후 개념적으로 다음과 같은 dataflow가 될 수 있다.

```text
            Input y
           /       \
  Reduce(Add)      Tally
           \       /
             Divide
```

여기서부터는 원래 source가 fork였다는 사실이 실행에 불필요할 수 있다. 다만 진단·debug·rewrite provenance가 필요하면 origin metadata로 연결할 수 있다.

Logical Plan에서 보존할 정보:

- ValueId
- normalized array operation
- dtype/shape facts
- explicit cell/frame mapping
- dependency
- effect boundary
- alias facts
- materialization requirement
- fusion candidate
- backend support facts
- source/semantic origin metadata

아직 특정 device buffer 주소나 CUDA launch parameter는 없다.

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

### 6.1 논리 J 배열과 verb

J의 noun 의미는 다음으로 유지한다.

```text
JArray
  type
  shape
  ordered atoms / logical value
```

verb는 이 noun array를 입력받아 noun array를 반환하는 array transformer이며, J Semantic Array IR에서 first-class semantic entity로 표현한다. verb의 hook/fork/train/modifier composition은 semantic analysis 전에 보존한다.

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

고수준 IR에서는 먼저 derived-verb 구조를 보존한다.

```text
Apply(
  DerivedVerb(Insert, Add),
  Map(Square, y)
)
```

Semantic Analyzer가 이를 분석하여 reduction이라는 logical operation을 식별한 뒤,

```text
Reduce(Add, Map(Square, y))
  ↓
legal fusion analysis
  ↓
fused map-reduction kernel
```

처럼 계획할 수 있다.

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

- [x] RustJ 내부 compiler stage의 논리적 경계를 확정한다.
- [x] Semantic Analyzer 입력 전에 hook/fork/train/derived verb/rank를 제거하지 않는 원칙을 확정한다.
- [x] `J Semantic Array IR`과 `Logical Array IR / Plan`을 구분한다.
- [x] generic boundary 후보를 semantic analysis 이후의 Logical Array IR로 이동한다.
- [x] 문서를 `PROJECT.md`로 통합한다.
- [ ] 실제 코드 dependency에서도 source frontend → J Semantic Array IR → Semantic Analyzer/Lowering → Logical Plan 경계를 만든다.

### A1 — J Semantic Array IR와 Semantic Analyzer 경계

- [ ] 현재 `semantic.rs`가 noun/verb/adverb/conjunction과 derived composition을 얼마나 보존하는지 감사한다.
- [ ] primitive verb identity와 monad/dyad valence를 명시한다.
- [ ] Hook / Fork / Train을 first-class semantic node로 표현한다.
- [ ] adverb/conjunction application으로 생긴 DerivedVerb 구조를 보존한다.
- [ ] rank-derived verb와 cell/frame 의미를 Semantic Analyzer가 분석할 수 있게 표현한다.
- [ ] name reference/binding/version과 source span을 필요한 범위에서 연결한다.
- [ ] primitive contract를 semantic node에 연결한다.
- [ ] Semantic Analyzer가 source parser 없이 J Semantic Array IR만으로 분석 가능하게 한다.
- [ ] Semantic Analyzer / Lowering이 semantic structure를 Logical Array IR / Plan으로 낮추는 테스트를 작성한다.
- [ ] fork branch 독립성, reduction derived verb, rank-derived verb를 대표 golden test로 둔다.

완료 조건: Semantic Analyzer를 scanner/parser 없이 테스트할 수 있으면서도 hook/fork/train/rank/derived verb의 의미 구조가 분석 입력에 남아 있다.

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
- [ ] source → J Semantic Array IR → Semantic Analyzer/Lowering → Logical Array IR/Plan → physical → CPU end-to-end

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
- 명시적인 `J Semantic Array IR → Semantic Analyzer/Lowering → Logical Array IR/Plan` 경계는 아직 코드에서 완전히 분리되지 않았다.
- 현재 `analysis.rs`가 Semantic IR에서 LogicalPlan을 직접 만들고 있어 semantic analysis와 lowering 경계를 재정리해야 한다.
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

- 고수준 semantic graph를 분석한 뒤 logical dataflow 생성
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

RustJ frontend
  J source
    ↓
  J Semantic Array IR
    ↓
  Semantic Analyzer / Lowering
    ↓
  Logical Array IR / Plan
    ↓
  CPU / GPU physical plans
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

현재 가장 먼저 해야 할 compiler architecture 작업은 **J의 verb composition을 보존하는 semantic IR과 Semantic Analyzer / Lowering 경계를 코드에서 명시하는 것**이다.

순서:

1. 현재 `semantic.rs`, `analysis.rs`, `facts.rs`, `contracts.rs`의 책임을 다시 분류한다.
2. `semantic.rs`가 noun/verb/adverb/conjunction, hook/fork/train, derived verb, rank를 얼마나 보존하는지 감사한다.
3. 부족한 구조를 `J Semantic Array IR`로 명시한다.
4. Semantic Analyzer가 이 IR을 입력으로 받아 composition 구조를 분석하도록 `analysis.rs`를 재정리한다.
5. semantic lowering 결과로 `Logical Array IR / Logical Execution Plan`을 만든다.
6. fork, reduction derived verb, rank-derived verb를 Semantic Analyzer 단독 golden test로 검증한다.
7. 기존 LogicalPlan 결과와 의미 동등성을 비교한다.
8. 그 경계를 유지하면서 G2 structural view 작업을 계속한다.

특히 hook/fork/train/adverb/conjunction 정보를 “generic하게 만들기 위해” semantic analysis 이전에 소거하는 shortcut을 추가하지 않는다.
