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
   ├──────── Route A: RustJ-native planning/execution
   │              ↓
   │       Logical Optimizer
   │              ↓
   │       Physical Planner
   │              ↓
   │       Physical Execution Plan
   │              ↓
   │       RustJ CPU/GPU Runtime
   │
   ├──────── Route B: MLIR family
   │              ↓
   │       RustJ IR export adapter
   │              ↓
   │       tensor/linalg/scf/vector/gpu/...
   │              ↓
   │       LLVM / NVVM / ROCDL / SPIR-V
   │              ↓
   │       external/runtime execution
   │
   ├──────── Route C: StableHLO/OpenXLA-compatible subset
   │              ↓
   │       StableHLO adapter
   │              ↓
   │       XLA / IREE / other consumer
   │
   └──────── Route D: library / foreign backend call
                  ↓
          BLAS / vendor library / custom kernel
```

핵심 원칙은 **J의 고수준 배열 변환 구조를 Semantic Analyzer가 보기 전에 없애지 않고, analyzer/lowering 단계가 그 구조를 분석한 뒤 backend-independent logical dataflow로 낮추는 것**이다.

### 2.1 RustJ compiler stage의 위상

RustJ는 하나의 compiler system으로 개발한다. 별도 고유 컴포넌트명을 두기보다 각 compiler stage의 책임을 명확히 분리한다.

- **RustJ frontend**: source text를 읽고 J의 품사·결합·이름 의미를 보존한 `J Semantic Array IR`을 만든다.
- **J Semantic Array IR**: J의 배열 계산을 고수준에서 표현한다. hook/fork/train, adverb/conjunction으로 만든 derived verb, rank 같은 의미 구조를 보존한다.
- **Semantic Analyzer / Lowering**: 이 고수준 IR을 분석하여 explicit dataflow와 array operation으로 이루어진 `Logical Array IR / Logical Execution Plan`으로 낮춘다. 이 단계는 target-independent facts와 semantic legality를 만든다.
- **Route Selection / Export**: Logical Array IR 이후에는 RustJ-native planning, MLIR, StableHLO-compatible subset, library/custom-kernel 등의 검증된 경로 중 하나를 선택할 수 있다.
- **RustJ-native Physical Planner**: Route A에서만 layout, placement, materialization, buffer, transfer, scheduling 같은 물리 실행 결정을 내린다.
- **Backend/Runtime**: 선택된 route의 lower-level IR 또는 Physical Plan을 실행 가능한 artifact로 낮추고 실행한다.

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
Route selection / export
   ├─ RustJ Physical Plan
   ├─ MLIR family
   ├─ StableHLO-compatible subset
   └─ verified library/custom backend
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
  ir-verify
  ir-adapters
  native-schedule
  native-physical
  backend-cpu
  backend-gpu
  runtime
```

실제 crate 분리는 인터페이스가 안정된 뒤 진행한다. **repository 분리보다 dependency 방향과 API 경계가 우선**이다.

### 2.3 Logical IR은 실행 backend를 강제하지 않는다

RustJ의 목표는 J 의미를 정확히 분석하여 **좋은 IR과 정확한 계약을 만드는 것**이지, 모든 최적화와 code generation을 직접 다시 구현하는 것이 아니다.

따라서 `Logical Array IR`은 RustJ-native planner만의 내부 자료구조가 아니라 **여러 실행 경로가 소비할 수 있는 compiler boundary**로 설계한다.

원칙:

1. RustJ 자체 Physical Planner/Executor 경로를 유지한다. 이는 bootstrap, differential validation, target-specific 실험에 유용하다.
2. 지원 가능한 연산은 MLIR의 `tensor`/`linalg`/`scf`/`vector`/`gpu` 계층으로 낮출 수 있게 한다.
3. 더 낮은 단계에서는 LLVM IR, NVVM, ROCDL, SPIR-V 같은 성숙한 IR/backend를 활용할 수 있다.
4. StableHLO로 의미 보존이 가능한 subset은 StableHLO export를 허용하여 XLA/IREE 같은 외부 compiler를 사용할 수 있다.
5. matmul/conv/FFT 같은 연산은 경우에 따라 최적화된 library/custom-call 경로가 더 적절할 수 있다.
6. 어느 외부 IR도 J 전체 semantics를 자동으로 표현한다고 가정하지 않는다. boxed array, J-specific rank semantics, observable error ordering, effects처럼 표현력이 부족한 부분은 lowering 전에 명시적으로 해소하거나 해당 route를 사용하지 않는다.
7. 외부 route가 불가능하거나 의미 보존을 증명하지 못하면 해당 route를 거부한다. RustJ-native conservative path가 실제로 해당 op를 지원할 때만 fallback으로 사용할 수 있으며, 어떤 route도 지원하지 않으면 명시적인 unsupported 결과를 낸다.

즉:

```text
RustJ owns:
  J semantics
  semantic contracts
  semantic rewrite/lowering legality
  external-route preconditions
  provenance

External/native lower backends own:
  lower-level target legality
  instruction/resource legality
  backend-specific verification

RustJ may delegate:
  loop optimization
  vectorization
  tiling
  register allocation
  instruction selection
  GPU kernel lowering
  machine-code generation
```

이 원칙은 compiler의 책임을 포기하는 것이 아니라 **의미와 legality는 RustJ가 책임지고, 검증된 외부 최적화 infrastructure는 재사용한다**는 뜻이다.

### 2.4 IR export adapter의 계약

외부 IR로 내보내는 adapter는 단순 pretty-printer가 아니다.

각 adapter는 다음을 명시해야 한다.

```text
IrExportAdapter
  accepted_logical_ops
  required_shape_constraints
  required_numeric_relaxations
  required_effect_conditions
  required_alias_conditions
  dynamic-shape support
  custom-call / library escape hatch
  target capability requirements
  provenance mapping
```

adapter는 다음 중 하나를 반환한다.

```text
Lowered(external_ir)
Unsupported(reason)
Guarded {
  predicate,
  fast_route,
  fallback_route | Unsupported
}
```

외부 IR로 내릴 때 J semantic origin과 source span을 가능한 범위에서 metadata/provenance로 유지한다.

MLIR은 여러 abstraction의 dialect를 한 module 안에서 공존시키고 dialect conversion으로 점진적으로 lowering할 수 있으므로 RustJ의 multi-level IR 구조와 특히 잘 맞는다. StableHLO는 ML framework/compiler 사이의 portability layer를 목표로 하는 high-level op set이므로 NN/tensor subset의 선택적 export 대상으로 본다. LLVM IR/SPIR-V는 더 낮은 execution target으로 사용한다.


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
- iteration domain / axis semantics / access relation 도출
- control/data dependency graph 생성
- effect / alias / speculation legality 분석
- uniformity / symbolic constraint / mask fact 전파
- map / reduce / scan / gather / structural pattern 식별
- derived structure의 **target-independent** normalization/lowering
- semantic storage/lifetime requirement 도출
- target-independent rewrite legality와 fusion constraint 도출
- Logical Array IR / Logical Execution Plan 생성

다음은 이 단계의 책임이 아니다.

- 특정 backend capability를 보고 schedule을 선택하는 일
- fusion region을 실제로 확정하는 일
- optional intermediate를 실제 buffer로 materialize하는 일
- tile/vector/workgroup/layout/device를 고르는 일
- target cost model로 후보를 ranking하는 일

이 결정들은 Route Selection, 외부 compiler, 또는 RustJ-native Logical Optimizer/Physical Planner가 담당한다.

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
Logical Array IR / Plan  ← generic compiler boundary
    ↓
Route selection / export
    ├─ RustJ-native optimizer/planner
    ├─ MLIR
    ├─ StableHLO-compatible subset
    └─ library/custom backend
```

향후 다른 array DSL frontend를 붙이고 싶다면 두 선택이 가능하다.

1. J semantic model을 의도적으로 공유하면 J Semantic Array IR을 생성한다.
2. J와 무관한 frontend라면 자기 semantic analyzer를 거쳐 Logical Array IR / Plan에 합류한다.

따라서 **middle-end를 generic tensor IR consumer처럼 만들기 위해 J의 구조를 일찍 버리지 않는다.**

### 4.4 분석 fact는 typed lattice로 관리한다

shape, alias, uniformity, effect, binding, constraint 같은 서로 다른 분석 정보를 하나의 범용 `Unknown` 값으로 뭉개지 않는다.

각 fact domain은 자기 lattice를 가진다.

```text
Fact<T>
  Uninitialized
  Known(T)
  Overdefined / Unknown
  Contradiction / Invalid   // 해당 domain에서 의미가 있을 때
```

control-flow merge나 여러 predecessor에서 fact가 합쳐질 때는 domain별 monotonic `join`을 사용한다. 이는 MLIR data-flow framework의 lattice 방식과 같은 원칙이다.

예:

```text
ShapeFact
AliasFact
UniformityFact
ConstraintFact
EffectFact
BindingFact
LayoutFact
```

`Unknown`은 사실을 임의로 꾸며내지 않는다는 뜻이지 곧바로 실행 불가를 뜻하지 않는다. domain과 route에 따라 다음 중 하나가 된다.

- optimization barrier
- runtime guard / witness
- conservative lowering
- external route rejection
- 재분석 조건

반면 `Contradiction/Invalid`은 rank/shape/domain error처럼 semantic error가 증명된 경우와 구분한다.

### 4.5 Primitive contract

semantic analysis에서 모든 primitive는 공통 `PrimitiveContract` interface를 통해 해석한다. built-in J primitive와 name 기반 extension primitive의 **등록 경로는 달라도 분석 interface는 같다.**

이 절의 contract는 “분석기가 반드시 물어볼 수 있어야 하는 질문”을 정의하고, 구체적인 저장 구조는 4.10의 `PrimitiveSpec = Identity + Analysis + Realization` 분리를 따른다.

semantic 쪽에서 최소한 다음을 표현하거나 명시적으로 `Unknown`으로 둘 수 있어야 한다.

```text
identity / part of speech / valence
innate rank
shape rule
dtype / promotion rule
axis-role / access-pattern rule
error contract
  domain
  rank / shape
  overflow / promotion
  observable error ordering
effect contract
alias / mutation legality
safe rewrite / reassociation constraints
semantic reference definition (optional)
```

fusion cost, accumulator realization, register/shared-memory 양, concrete layout, tile 크기, device-specific intrinsic은 semantic identity 자체가 아니다. 이들은 AnalysisContract의 요구사항과 TargetProfile을 바탕으로 realization/physical planning에서 결정한다.

따라서 `PrimitiveContract`는 analyzer가 보는 공통 interface이고, `PrimitiveSpec`은 그 contract를 실제로 제공하는 versioned registry record라는 관계로 사용한다.

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

### 4.7 과거 JAXA/Japchae 저장소 통합 기준

2026-09-30에 다음 네 저장소의 최신 내용을 다시 대조했다.

| 저장소 | 검토 기준 | 이 문서에 흡수하는 핵심 |
|---|---|---|
| `yunskim/JAXA` | main `12bc0659`, 2026-03-24 | 정적 graph, fusion, J 조합 구조를 이용한 NN 표현이라는 초기 문제의식 |
| `yunskim/JAXA-complier` | main `ceba0589`, 2026-04-26 | custom primitive registry prototype, rank/shape/analyzer metadata의 실제 자료구조 |
| `yunskim/japchae` | main `510c31b5`, 2026-06-19 | primitive identity/realization 2층 모델, resource function, materialized arrays, analyzer output schema |
| `yunskim/jaxa-analyzer` | substantive baseline `3eef3942`, 2026-07-28; 2026-09-30에는 RustJ 이관 상태 주석 추가 | J frontend/vocabulary/name extension, semantic AST, primitive contract, Flow–Storage/resource 분석의 최신 정리 |

앞으로 위 저장소들은 **historical research/prototype source**다. 설계 결정을 수정할 때 원본을 다시 고쳐 여러 갈래를 유지하지 않고 이 `PROJECT.md`를 갱신한다.

결정 충돌 시 단순한 repository 날짜보다 **같은 주제에 대한 후속 결정**을 우선한다. 대표적인 예:

- 초기 `JAXA`의 complex rank/precision 표기 `"RjP`는 표준 J와 호환되지 않으므로 채택하지 않는다.
- J의 rank conjunction `"`은 표준 J 의미 그대로 유지한다.
- dtype/precision, accumulator precision, layout, hardware resource는 rank 문법에 억지로 넣지 않고 각각 semantic contract와 planning 계층에 둔다.
- 4월 prototype의 고정 `memory_layout`, `tiling_axis`, register 숫자는 최종 semantic identity가 아니다. 6~7월 설계처럼 identity contract + realization function + target profile로 일반화한다.

### 4.8 확장 primitive는 name binding + registry contract로 추가한다

NN/array extension primitive는 J의 새로운 keyword나 punctuation을 추가하지 않고 **name**으로 추가한다.

```text
source name "conv"
      ↓
J word / NameRef
      ↓
name / extension resolution
      ↓
ExtensionPrimitiveRef(Conv)
      │
      └───────────────┐
                      ↓
              Primitive Registry
                      ↓
                 PrimitiveSpec
                      ↓
J Semantic Array IR + analysis contract
```

중요한 불변식:

1. parser가 `conv`라는 문자열을 특별 취급하지 않는다.
2. name이 primitive로 해소되면 문자열이 아니라 안정적인 `PrimitiveId`/entity identity를 가진다.
3. `f =: conv`처럼 alias/binding을 거쳐도 같은 primitive identity와 spec이 보존되어야 한다.
4. registry는 spelling table이 아니라 **분석 계약의 single source of truth**다.
5. custom primitive 추가 때문에 scanner/parser 코드를 수정하지 않는다.
6. J built-in primitive와 extension primitive는 출처는 달라도 analyzer에서는 공통 `PrimitiveContract` interface로 다룬다.

기존 `JAXA-complier` prototype은 built-in lookup 실패 후 custom registry를 검사했다. RustJ에서는 정확한 enqueue 시점 구현보다 **J name semantics를 보존하면서 semantic resolution 결과가 registry-backed primitive entity가 된다**는 점을 아키텍처 불변식으로 둔다.

### 4.9 Vocabulary는 이름 목록이 아니라 form + contract다

과거 연구의 중요한 결론은 primitive vocabulary가 단순 whitelist가 아니라는 점이다.

Structural vocabulary에는 primitive verb/adverb/conjunction, hook/fork/train, `@:`, `@`, `&`, rank `"`, insert/reduce `/`, scan `\` 및 기타 derived-verb builder가 포함된다.

Computational vocabulary에는 표준 J primitive와 name 기반 extension primitive(`conv`, `linear`, `relu` 등)가 포함된다.

Structural form에는 필요하면 operand/result 품사와 semantic constraint를 둔다. RustJ 자체는 장기적으로 J 전체 의미를 구현하므로 다음을 구분한다.

```text
valid J semantics
    ≠
advanced optimization contract available
```

J built-in의 resource contract가 Unknown이면 보수적인 plan으로 실행할 수 있다. 반면 RustJ 고유 extension primitive는 최소 semantic contract와 실행/lowering 경로가 없으면 등록 완료로 보지 않는다.

### 4.10 PrimitiveSpec은 semantic record이고, realization은 별도 registry/interface다

과거 prototype처럼 primitive 하나에 rank, shape, layout, register, tiling, backend implementation을 모두 넣지 않는다.

현행 모델은 **semantic record + capability interfaces + lowering/realization registry**로 나눈다.

```text
PrimitiveSpec
├─ identity / part of speech / valence
├─ innate rank
├─ parameter schema
├─ semantic reference definition (optional)
└─ version / provenance

Primitive semantic interfaces
├─ ShapeInference
├─ TypePromotion
├─ AxisAndIterationSemantics
├─ AccessPattern
├─ NumericSemantics
├─ EffectSemantics
├─ AliasSemantics
├─ SpeculationSemantics
└─ RewriteLegality

Lowering / realization interfaces
├─ RustJNativeLowering
├─ MlirLowering
├─ StableHloLowering
├─ LibraryLowering
└─ TargetSpecificLowering
```

이 방식은 MLIR의 operation interface 원칙과 유사하다. 분석기와 변환기는 concrete op 이름을 일일이 special-case하기보다 필요한 capability interface를 질의한다.

예를 들어 conv extension이 등록될 때 semantic registry에는 다음이 들어갈 수 있다.

```text
PrimitiveId::Conv2d
  ShapeInference
  AxisAndIterationSemantics
  AccessPattern
  NumericSemantics
  EffectSemantics
```

반면 NVIDIA/AMD/CPU의 실제 구현 선택은 같은 record 안에 target 숫자로 박지 않고 lowering registry가 제공한다.

```text
LoweringRegistry.lookup(
    primitive,
    route,
    target_capabilities
) -> candidates
```

과거의 `register_fn`, `shared_memory_fn`, `accum_fn`은 semantic primitive property가 아니라 native planner용 realization/resource model 구현으로 옮긴다.

```text
ResourceUsage
  = R(LogicalGraph, PhysicalSchedule, TargetProfile)
```

이렇게 하면 extension primitive를 추가해도 모든 분석/optimizer/backend가 그 이름을 직접 알 필요가 없다.

### 4.11 rank와 axis role은 서로 다른 정보다

J rank가 알려주는 것은 argument를 frame과 cell로 어떻게 나누어 verb를 적용하는가이다. PrimitiveSpec의 axis-role contract는 그 cell 내부의 각 축이 연산에서 어떤 역할을 하는가를 알려준다.

```text
full argument shape
        ↓ J rank semantics
frame axes | cell axes
             ↓ PrimitiveSpec.axis_roles
       semantic axis roles
```

예를 들어 conv2d:

```text
full shape: [B, C, H, W]
innate cell rank: 3

frame: [B]
cell : [C, H, W]

cell axis 0 = channel / reduction input
cell axis 1 = spatial H
cell axis 2 = spatial W
```

따라서 흔히 말하는 **leading axis**도 전체 배열의 고정된 의미로 하드코딩하지 않는다. full array의 leading axis가 frame일 수 있고, primitive가 분석에 사용하는 것은 cell 내부 axis role이다. 필요하면 `cell axis 0`이 channel/reduction이라는 사실을 spec에 명시한다.

conv 계열의 innate rank는 다음처럼 정리한다.

```text
conv1d cell = [C, W]       innate rank 2
conv2d cell = [C, H, W]    innate rank 3
conv3d cell = [C, D, H, W] innate rank 4
```

일반적으로 `conv_kd innate rank = k + 1`이다. 공간 차원 외에 channel 축이 cell 안에 있어야 channel accumulation을 표현할 수 있기 때문이다.

개념적인 `AxisRoleSpec`은 cell axis role, reduction axes, parallel axes, window axes, preserved axes, output-axis mapping을 가진다. `C/H/W` 같은 이름은 사람이 읽기 위한 label이고 analyzer는 reduction/parallel/window/static-reindex 같은 역할을 사용한다.

가변 reduction인 표준 J `+/` 같은 연산은 rank/cell 구조에서 axis가 유도된다. 반대로 conv처럼 축 역할이 연산 정체성에 고정된 primitive는 registry spec이 그 역할을 제공한다.

### 4.12 access pattern은 fusion 분석의 semantic lower bound다

`japchae` D-24에서 primitive를 scalar arithmetic까지 지나치게 분해하면 matmul/conv 구조 정보가 사라져 다시 pattern recognition을 해야 한다는 문제가 확인되었다. fusion/array planning 관점의 유용한 lower bound는 **memory/access pattern**이다.

초기 분류:

```text
Map
Reduce
WindowReduce
Scan
StaticReindex
DynamicGather
Scatter
```

`StaticReindex`는 reshape/transpose/flatten/broadcast처럼 index mapping이 compile-time known인 경우다. 기본적으로 materialization이 아니라 view/remap 후보다. 실제 copy 여부는 downstream layout과 target을 본 physical planning에서 결정한다.

### 4.13 semantic reference definition과 implementation을 분리한다

name 기반 custom primitive의 구현은 black box여도 되지만 의미까지 black box여서는 안 된다. 표준 J로 표현 가능한 extension primitive는 가능하면 **기존 J primitive만으로 reference definition**을 제공한다.

reference definition은 execution implementation도 resource model도 아니며 optimization decomposition을 강제하지 않는다. 원래 J 엔진을 semantic oracle로 사용할 수 있게 하는 명세다.

```text
semantic validation
  extension result == reference J result

resource validation
  predicted resource/cost ~= measured backend behavior
```

reference definition을 실제 lowering으로 사용하는 것은 별도 검증을 통과한 뒤에만 허용한다.

### 4.14 Hardware-aware IR 설계 원칙: 네 층을 분리한다

leading axis, reduction axis, contiguity 같은 정보는 중요하지만 **하드웨어 분석에 필요한 전체 정보의 일부**다.

2026-09-30에 MLIR DataLayout/TargetSystemSpec, LLVM TargetTransformInfo, OpenXLA GPU pipeline, TVM TensorIR scheduling, Triton kernel configuration, NVIDIA CUDA, AMD ROCm/HIP 문서를 다시 검토했다.

공통적으로 확인되는 구조는 다음과 같다.

1. 고수준 연산 의미와 target hardware의 고정 사양을 분리한다.
2. logical computation에는 planner가 병렬화·locality·reduction·layout 가능성을 추론할 수 있는 정보가 있어야 한다.
3. tile size, thread/workgroup mapping, memory-space 배치 같은 값은 target을 본 뒤 schedule/physical 단계에서 결정한다.
4. register 사용량, occupancy, memory traffic 같은 값은 primitive의 고정 상수가 아니라 **graph + schedule + target**의 함수다.
5. hardware의 hard limit와 측정 기반 performance cost를 같은 것으로 취급하지 않는다.

RustJ에서는 이를 네 층으로 나눈다.

```text
A. Hardware-relevant semantic/logical facts
   "이 계산은 어떤 구조인가?"

B. TargetProfile
   "이 하드웨어는 무엇을 제공하고 무엇을 제한하는가?"

C. Physical Schedule / Realization
   "이 계산을 이 target에 어떻게 매핑할 것인가?"

D. Resource / Cost Estimate
   "그 매핑의 자원 사용량과 예상 비용은 얼마인가?"
```

이 네 층을 하나의 `PrimitiveSpec` 또는 하나의 거대한 IR node에 섞지 않는다.

예를 들면 `leading axis`, `reduction axis`, `access relation`, `iteration dependency`는 A에 속하고, `warp/wave width`, register-file capacity, shared/LDS capacity, supported matrix instruction은 B에 속한다. `tile=64x128`, `num_warps=4`, `vector_width=8`, memory-space 선택은 C이며, `registers/thread=72`, occupancy, HBM bytes, spill risk는 D다.

### 4.15 Logical Array IR이 가져야 하는 hardware-relevant contract

Logical Array IR은 hardware-independent여야 하지만 **hardware-blind여서는 안 된다.** CUDA의 `threadIdx.x`를 몰라도 planner가 어떤 logical axis를 thread/lane/vector에 배치할 수 있는지는 알아야 한다.

최소 모델:

```text
LogicalOp
├─ IterationDomain
├─ AxisSemantics
├─ AccessRelations
├─ NumericSemantics
├─ Dependency / SynchronizationRequirements
├─ Effect / Alias
└─ Materialization / LifetimeRequirements
```

#### 4.15.1 IterationDomain

연산이 어떤 index 공간 위에서 정의되는지를 표현한다.

```text
IterationAxis
  id
  extent
  kind:
    Parallel
    Reduction
    Scan
    Window
    SerialDependency
    Broadcast
    StaticReindex
    Gather
    Scatter
```

이것은 GPU thread axis가 아니다. logical iteration axis다. 이후 planner가 CPU thread, SIMD lane, GPU workgroup/subgroup/lane 등에 매핑한다.

#### 4.15.2 AxisSemantics

J rank가 frame/cell을 정하고, primitive contract가 cell 내부 axis role을 정한다. 필요한 정보는 단순한 `leading_axis` 하나보다 넓다.

```text
AxisSemantics
  frame_axes
  cell_axes
  input_axis_roles
  output_axis_roles
  reduction_axes
  preserved_axes
  introduced_axes
  contracted_axes
  window_axes
  broadcast_axes
  scan_axes
  output_axis_mapping
```

conv2d라면 input cell `[C_in,H,W]`, weight `[C_out,C_in,KH,KW]`, output cell `[C_out,OH,OW]`이고 output/parallel axes는 `[C_out,OH,OW]`, reduction axes는 `[C_in,KH,KW]`, window axes는 `[KH,KW]`다.

`leading axis`가 필요하다면 이 구조 안에서 `cell axis 0` 또는 특정 access map의 major/minor logical axis로 표현한다. standalone 특별 규칙으로 두지 않는다.

#### 4.15.3 AccessRelations

성능 분석에는 axis 이름보다 **실제로 각 operand를 어떤 index로 읽고 쓰는가**가 더 중요하다. 따라서 가능한 범위에서 input/output index relation을 표현한다.

```text
AccessRelation
  operand
  mode: Read | Write | ReadWrite
  index_map(iteration_axes, reduction_axes) -> operand_indices
  regularity:
    Affine
    StaticPermutation
    Windowed
    Indirect
    DataDependent
  reuse_axes
  known_stride_facts
  known_alignment_facts
  bounds / masking requirement
```

conv2d의 개념적 관계:

```text
Y[oc, oh, ow]
X[ic, oh*stride_h + kh - pad_h, ow*stride_w + kw - pad_w]
W[oc, ic, kh, kw]
```

이 정보로 planner는 contiguous/vectorized access 후보, tile 내부 reuse, reduction/output parallelism, shared/LDS/cache staging, static reindex, gather/scatter 제약을 판단할 수 있다. 따라서 향후 `leading_axis`보다 더 일반적인 핵심 표현은 **axis-role + access relation**이다.

#### 4.15.4 NumericSemantics

하드웨어 mapping을 결정하려면 dtype만으로 부족하다.

```text
NumericSemantics
  input dtypes
  result dtype
  accumulator requirement
  widening / narrowing rules
  exact overflow behavior
  reassociation allowed?
  FMA contraction allowed?
  reduction order observable?
  NaN / signed-zero constraints
```

CPU vector reduction이나 GPU tree reduction은 J의 관찰 가능한 floating-point 순서를 바꿀 수 있으므로, `reduction`이라는 사실만으로 재배치를 허용하지 않는다.

#### 4.15.5 Dependency / synchronization requirement

logical op은 CUDA barrier 자체를 갖지 않는다. 대신 어떤 범위의 dependency가 필요한지 표현한다.

```text
DependencyRequirement
  Independent
  Reduction(axis_set)
  OrderedScan(axis_set)
  WindowDependency(axis_set)
  AtomicUpdate(resource_or_value)
  CrossValueOrdering(effect_or_resource)
  Collective(logical_participants)
```

여기서 `Subgroup`, `Warp`, `Workgroup` 같은 execution scope는 Logical IR에 넣지 않는다. 이들은 Physical Schedule이 logical participants를 target execution hierarchy에 매핑한 뒤 생긴다.

Physical Planner 또는 외부 compiler가 target의 barrier/shuffle/atomic/collective capability를 보고 구체적으로 실현한다.

#### 4.15.6 Uniformity / Divergence

SPMD target에서는 값과 control flow가 execution scope 안에서 uniform한지 varying한지가 중요한 hardware-relevant fact다.

```text
UniformityFact
  Uniform(over_logical_axes)
  Varying(over_logical_axes)
  Unknown
```

적용 대상:

- branch condition
- indirect index
- pointer/address calculation
- mask
- subgroup collective operand

Logical IR에서는 어떤 logical axes에 대해 값이 invariant인지 보존한다. Physical Schedule이 그 axes를 subgroup/lane에 매핑하면 backend가 실제 divergence와 broadcast/coalescing 가능성을 계산한다.

#### 4.15.7 Symbolic shape / divisibility / alignment constraints

dynamic extent는 단순 `Unknown`으로 버리지 않고 가능한 제약을 보존한다.

```text
ConstraintSet
  Equal(a, b)
  UpperBound(x, n)
  LowerBound(x, n)
  MultipleOf(x, n)
  DivisibleBy(x, n)
  PowerOfTwo(x)
  Alignment(value, bytes)
  ContiguousRun(axis, n)
  NonZero(x)
```

이 정보는 vector width, tensor/matrix instruction, tile size, unrolling, memory transaction legality를 결정할 때 사용한다.

제약이 compile-time에 증명되지 않더라도 runtime guard로 specialization할 수 있다.

```text
if N % 16 == 0
  → vectorized/tensor path
else
  → conservative path
```

#### 4.15.8 Predication / masking semantics

tail tile과 out-of-bounds window를 안전하게 다루려면 logical mask 의미가 필요하다.

```text
MaskSemantics
  predicate source
  affected accesses
  masked-load fill semantics
  masked-store semantics
  bounds relation
  side-effect suppression rule
```

mask는 GPU-specific 개념이 아니다. CPU masked vector instruction이나 scalar fallback에도 동일한 logical fact를 사용할 수 있다.

#### 4.15.9 SSA, region/block, control flow

현재 array dataflow만으로는 향후 direct/explicit definition의 조건분기·반복·호출을 충분히 표현할 수 없다.

Logical Array IR은 장기적으로 최소한 다음 구조를 허용한다.

```text
Function
  Region
    Block(args...)
      Op...
      Terminator(successors / return)
```

value는 SSA `ValueId`로 표현한다. pure array graph는 single-block graph region으로 표현할 수 있고, control flow가 필요한 경우 CFG region/block을 사용한다.

J의 hook/fork/train을 이 CFG로 일찍 풀라는 뜻은 아니다. 그것들은 J Semantic Array IR에서 보존한 뒤 semantic lowering 결과로 필요한 control/dataflow만 만든다.

#### 4.15.10 Constraint witness / runtime guard

`ConstraintSet`은 metadata 목록만으로 끝내지 않는다. 어떤 최적화가 특정 runtime assumption에 의존하는지 추적할 수 있어야 한다.

개념 모델:

```text
WitnessId
Assert(constraint) -> WitnessId
Assume(witness) {
  optimized region
}
Guard(constraint,
      fast_region,
      fallback_region)
```

compile-time에 증명된 constraint는 witness 없이 fact로 정착할 수 있다. 동적 constraint는 guard를 통해 specialization 경로와 fallback 경로를 동시에 보존한다.

이는 MLIR Shape dialect의 witness/assuming 아이디어와 같은 목적을 가진다.

#### 4.15.11 effect resource와 ordering token

SSA data dependency만으로는 I/O, mutable state, explicit storage update, runtime call의 관찰 가능한 순서를 모두 표현할 수 없다.

Logical IR은 effect를 두 수준으로 표현한다.

```text
EffectSummary
  resource
  Read | Write | Allocate | Free | IO | Unknown
  stage/order constraints

EffectToken
  explicit ordering edge when data dependency alone is insufficient
```

모든 pure op에 token을 붙이지 않는다. observable side effect나 external call처럼 순서가 의미에 포함되는 경우에만 explicit token/effect edge를 사용한다.

StableHLO의 side-effecting op token과 MLIR MemoryEffect/Speculation interface를 참고하되, J의 error ordering까지 포함할 수 있도록 `SpeculationSemantics`를 별도로 둔다.

#### 4.15.12 speculation / may-error semantics

J에서는 domain/rank/length/overflow 등의 오류 발생 순서도 관찰 가능할 수 있다. 따라서 “memory effect가 없다”와 “마음대로 speculative execution 가능”은 다르다.

```text
SpeculationSemantics
  AlwaysSafe
  SafeIf(constraints)
  MayRaiseObservableError
  MayNotTerminate
  HasNonLocalControlEffect
```

optimizer와 external adapter는 이 contract를 보고 hoist, duplicate, eliminate, reassociate 가능성을 판단한다.



### 4.16 TargetProfile: 하드웨어 hard facts와 capabilities

`TargetProfile`은 logical IR 밖의 **versioned target description**이다. compile invocation/plan에 연결되지만 J semantic value의 일부는 아니다. MLIR TargetSystemSpec처럼 여러 device를 기술할 수 있는 방향을 지향한다.

```text
TargetProfile
├─ TargetIdentity
├─ ExecutionHierarchy
├─ RegisterResources
├─ MemoryHierarchy
├─ ComputeCapabilities
├─ SynchronizationCapabilities
├─ LaunchAndSchedulingLimits
├─ TransferAndTopology
└─ ABI / DataLayout
```

#### 4.16.1 TargetIdentity

`vendor`, architecture, device family/feature set, backend target triple 또는 equivalent, driver/runtime capability version, profile schema version을 둔다. product name보다 capability query를 우선한다.

#### 4.16.2 ExecutionHierarchy

GPU 예는 `Device → ComputeUnit/SM → Workgroup/CTA → Subgroup/Warp/Wave → Lane/Thread`, CPU 예는 `Machine/NUMA → Core → Hardware thread → SIMD lanes`로 본다.

필요 후보는 compute-unit count, supported subgroup widths, resident subgroup/workgroup limits, threads/lanes per workgroup, workgroup/grid dimension limits, CPU SMT/NUMA facts다.

NVIDIA에서는 warp width뿐 아니라 register/shared-memory/thread-block 한도가 occupancy와 launch 가능성을 제한한다. AMD에서는 wavefront, VGPR/SGPR, LDS, wave slot과 block size가 함께 occupancy를 제한한다. 따라서 `warp_size` 하나만으로 GPU resource model을 만들지 않는다.

#### 4.16.3 RegisterResources

register를 단일 숫자로 가정하지 않는다.

```text
RegisterResource
  class:
    Scalar
    Vector
    Predicate
    Accumulator
    General
    TargetSpecific
  width_bits
  capacity_per_execution_unit
  max_per_lane_or_thread
  max_per_workgroup if applicable
  allocation_granularity
```

CPU에서는 scalar/fixed/scalable vector register width와 register class가 중요하고, AMD 계열에서는 VGPR/SGPR/accumulator class의 차이가 중요할 수 있다.

register/resource allocation은 연속적인 실수값이 아니라 target별 allocation granularity를 가진다. 따라서 capacity와 별도로 다음을 모델링한다.

```text
AllocationRule
  resource_class
  allocation_scope
  allocation_granularity
  rounding_rule
  max_per_lane_or_thread
  max_per_workgroup
  max_per_compute_unit
```

occupancy/concurrency 계산은 단순 `capacity / usage`가 아니라 allocation rule을 적용한 뒤 계산한다.


#### 4.16.4 MemoryHierarchy

memory space를 `global/shared` 두 종류로 고정하지 않는다.

```text
MemorySpace
  id
  scope / visibility
  capacity
  addressability
  allocation granularity
  alignment requirements
  transaction granularity
  bank structure if explicit
  coherence / consistency facts
  async-copy support
```

cache levels, cache-line size/capacity, scratchpad/shared/LDS capacity, HBM/DRAM capacity, host/device address spaces, constant/read-only spaces도 필요에 따라 profile에 둔다. OpenXLA처럼 logical shape와 physical memory space/layout을 분리한다.

일부 target에서는 cache와 scratchpad/shared memory가 동일한 physical resource를 partition하거나 configurable mode를 가진다. 따라서 독립 capacity만 저장하지 않고 resource coupling도 표현할 수 있어야 한다.

```text
ResourceCoupling
  participants
  valid_configuration_modes
  capacity_relation
  selection_scope
```

또한 coalescing/vector-load legality를 위해 모든 memory transaction 규칙을 거대한 static table로 복사하기보다 target query interface를 허용한다.

```text
TargetMemoryQueries
  legal_vector_access(access, width)
  transaction_estimate(access_distribution)
  preferred_alignment(type, space)
  bank_conflict_estimate(access_pattern)
  cache_line_or_transaction_granularity(space)
```


#### 4.16.5 ComputeCapabilities

```text
supported scalar dtypes
supported vector widths / scalable-vector support
native arithmetic/reduction/shuffle/permute
atomic operation signatures
matrix/tensor/MMA instruction families
  input dtype combinations
  accumulator dtype
  result dtype
  legal tile shapes
  layout constraints
special instructions
```

`tensor_core=true` 같은 boolean 하나보다 지원되는 operation signature 집합이 낫다.

각 특수 instruction capability에는 **execution scope**를 포함한다.

```text
ExecutionScope
  Lane
  SIMDGroup
  Subgroup
  WarpGroup
  Workgroup
  ComputeUnit
```

예를 들어 matrix/tensor instruction은 tile shape와 dtype뿐 아니라 몇 lane이 협력하는지, 어떤 operand layout과 accumulator register class를 요구하는지까지 capability로 질의할 수 있어야 한다.


#### 4.16.6 SynchronizationCapabilities

barrier scopes, subgroup shuffle/reduce, workgroup barrier, cross-workgroup synchronization, atomic scopes/dtypes, async barrier/pipeline support를 capability로 둔다.

atomic/memory model은 operation 지원 여부뿐 아니라 scope와 ordering을 포함한다.

```text
MemoryOrderingCapability
  supported_scopes
  supported_orderings
  atomic_ops_by_dtype
  fence_capabilities
  coherent_spaces
```


#### 4.16.7 LaunchAndSchedulingLimits

max threads/workgroup, resident workgroups/compute-unit, subgroups/workgroup, grid limits, dynamic scratchpad/shared-memory limits, cluster/cooperative launch capabilities 등을 둔다.

비동기 data movement는 boolean 하나가 아니라 capability family로 둔다.

```text
DataMovementCapability
  source_space
  destination_space
  dimensionality
  alignment / granularity
  execution_scope
  async
  synchronization mechanism
  optional transform/reduction support
```

CPU scalable-vector/SME 같은 target을 막지 않도록 execution mode도 확장 가능하게 둔다.

```text
ExecutionModeCapability
  feature state
  fixed_or_scalable_vector
  vector_length_range
  special matrix/register state
  legal mode transitions
```


#### 4.16.8 TransferAndTopology

장기적으로 device memory capacity, host-device links, peer-to-peer connectivity, NUMA relation, collective capability, concurrent copy/compute capability를 표현한다. single-device MVP에는 필수가 아니지만 Placement/Sharding 확장을 막지 않도록 위치를 예약한다.

#### 4.16.9 ABI / DataLayout

endianness, type size/alignment, pointer/address-space width, vector alignment 같은 정보다. J logical dtype 의미가 아니라 backend representation constraint다.

### 4.17 CostProfile: hard limit와 측정 성능을 분리한다

effective memory bandwidth, instruction throughput, cache hit rate, launch overhead, interconnect bandwidth/latency, library-call overhead는 driver, clock, power state, workload, runtime version에 따라 바뀔 수 있다.

따라서 `TargetProfile`과 별도로 optional `CostProfile`을 둔다.

```text
CostProfile
  target_profile_id
  runtime/driver/compiler version
  measurement provenance
  bandwidth estimates
  latency estimates
  instruction throughput estimates
  launch overhead
  transfer costs
  calibrated library/kernel costs
```

legality는 `TargetProfile` hard facts로 판단하고, 후보 ranking은 `TargetProfile + CostProfile`로 한다. CostProfile이 없어도 conservative heuristic으로 합법적인 plan을 만들 수 있어야 한다.

### 4.18 Physical Plan이 결정해서 기록해야 하는 hardware mapping

TargetProfile의 정보를 Logical IR에 복사하지 않는다. Physical Planner가 선택한 결과를 Physical Plan에 기록한다.

```text
PhysicalRegion
├─ target device
├─ kernel / library boundary
├─ logical-axis mapping
├─ tile hierarchy
├─ vector/subgroup/workgroup mapping
├─ memory-space assignment
├─ physical layout / strides / padding / alignment
├─ async-copy / software-pipeline stages
├─ synchronization
├─ accumulator realization
├─ materialization points
├─ buffer bindings
└─ transfer / collective actions
```

대표적으로 `AxisMapping(logical_axis -> execution_level)`, tile sizes, vector/subgroup width, workgroup/grid shape, `ValueId -> MemorySpace`, layout/strides/padding/alignment, prefetch distance, async-copy stages, double/multi buffering 등을 기록한다.

Triton의 `BLOCK_SIZE_*`, `num_warps`, `num_stages`, `maxnreg`와 OpenXLA fusion backend config의 tile/warp/stage 설정은 이 계층의 사례다.

### 4.19 ResourceEstimate는 plan의 결과이지 primitive property가 아니다

다음은 Physical Plan 후보를 TargetProfile/CostProfile과 결합해 계산하는 derived information이다.

```text
ResourceEstimate
  register usage by class
  shared/LDS/scratchpad bytes
  spill risk
  resident workgroups/subgroups
  theoretical occupancy/concurrency
  global-memory bytes
  cache/scratchpad traffic estimate
  transaction/coalescing estimate
  bank-conflict estimate
  arithmetic intensity
  instruction/compute estimate
  synchronization count/cost
  launch count
  transfer bytes/cost
  peak live memory
```

핵심 식:

```text
ResourceEstimate
  = R(LogicalGraph, PhysicalSchedule, TargetProfile, optional CostProfile)
```

따라서 primitive registry에 `registers=32`처럼 넣지 않는다. primitive는 `output마다 accumulator가 필요`, `이 축은 reduction`, `이 input tile은 재사용됨`, `workgroup-local collective 필요` 같은 요구/구조를 제공한다.

#### 4.19.1 Backend compiled-resource feedback

register allocation과 spill은 최종 backend lowering의 영향을 크게 받으므로 planner의 사전 추정만으로 완전히 확정할 수 없다.

따라서 backend는 선택적으로 실제 compile 결과를 다시 planner에 제공한다.

```text
CompiledResourceReport
  target
  kernel/artifact id
  register usage by class
  spills / local-memory bytes
  static + dynamic scratchpad/shared bytes
  stack frame
  generated instruction summary
  launch attributes
  backend diagnostics
```

흐름:

```text
Logical Plan
  ↓
Physical candidate
  ↓
estimated ResourceEstimate
  ↓
backend lowering / codegen
  ↓
CompiledResourceReport
  ↓
accept
or re-plan / choose another schedule
```

즉 RustJ planner는 backend compiler와 한 번만 대화하는 구조로 고정하지 않는다.

#### 4.19.2 TargetProfile은 data + query interface다

TargetProfile을 모든 vendor 규칙을 정적으로 열거한 거대한 struct로 만들지 않는다.

```text
TargetFacts
  stable capacities / limits / identities

TargetQueries
  vectorization_legal(...)
  memory_transactions(...)
  occupancy_bound(...)
  matrix_instruction_candidates(...)
  atomic_support(...)
  async_copy_candidates(...)
  preferred_layout(...)
```

hard fact는 versioned data로 보존하고, 복잡하거나 architecture-specific한 규칙은 query implementation으로 캡슐화한다.


### 4.20 CPU와 GPU에서 실제로 필요한 정보

공통으로 shape/dtype, iteration/dependency axes, reduction/scan semantics, access relation, stride/alignment facts, alias/effect, working-set/reuse structure, vectorization/reassociation legality, memory hierarchy, register/vector capacity, parallel execution capacity, cost model이 필요하다.

CPU에서 특히 중요한 target facts는 core/hardware-thread topology, SIMD fixed/scalable widths, vector register classes, cache hierarchy/cache-line size, NUMA, gather/scatter/reduction cost, prefetch capability다.

GPU에서 특히 중요한 target facts는 SM/CU count, warp/wave/subgroup widths, workgroup limits, resident workgroup/subgroup limits, register classes/capacity, shared/LDS, coalescing/transaction rules, bank organization, subgroup collectives, barriers/atomics, matrix/tensor instruction signatures, async copy/pipeline, grid/cluster limits다.

이 차이 때문에 target schema를 NVIDIA의 `SM/register/shared-memory` 세 필드에 맞춰 고정하지 않는다.

### 4.21 현재 RustJ에 권장하는 최소 구현 범위

처음부터 완전한 hardware database를 만들지 않는다.

```text
H0 Logical hardware contract
  IterationDomain
  AxisSemantics
  AccessRelation
  NumericSemantics
  DependencyRequirement

H1 generic TargetProfile MVP
  execution hierarchy
  subgroup/vector width
  parallelism limits
  register resource summary
  scratchpad/shared-memory summary
  memory spaces/alignment
  supported dtypes/operations
  ABI/data-layout

H2 Physical Schedule MVP
  target placement
  axis mapping
  tile size
  vector/subgroup/workgroup mapping
  memory-space choice
  layout/stride
  materialization
  synchronization

H3 ResourceEstimate MVP
  register estimate
  scratchpad/shared estimate
  occupancy/concurrency bound
  global-memory traffic
  peak materialized bytes
  launch count
```

그 다음 empirical `CostProfile`과 richer cache/topology model을 추가한다.

### 4.22 외부 compiler 조사에서 얻은 직접적인 설계 근거

- **MLIR DataLayout / TargetSystemSpec**: type layout과 heterogeneous device properties를 별도 target specification으로 둔다.
- **LLVM TargetTransformInfo**: vector register width와 instruction/reduction cost 같은 target-specific 정보를 IR 의미와 분리된 query interface로 제공한다.
- **OpenXLA GPU**: logical shape와 physical layout을 분리하고, layout conflict는 copy로 materialize하며, fusion 뒤 tile/warp/stage 같은 구체 schedule을 backend config로 둔다.
- **TVM TensorIR**: logical loop/block 위에 bind, vectorize, cache/storage scope, tensorize 같은 schedule을 별도로 적용한다.
- **Triton**: block shape, warps, pipeline stages, register cap을 kernel configuration으로 다루고 shape/stride/alignment를 memory access의 전제로 사용한다.
- **CUDA**: warp width 외에도 register file, per-thread/block register limit, shared-memory capacity, resident block/warp/thread limits가 함께 launch/occupancy를 결정한다.
- **ROCm/HIP**: VGPR/SGPR, LDS, wave slots, block size가 occupancy를 공동 제한하며 architecture에 따라 wavefront와 register organization이 달라진다.

따라서 RustJ hardware-aware IR의 중심은 특정 vendor 필드 복제가 아니라 **logical access/dependency structure + generic target capabilities + explicit physical schedule + derived resource model**이다.

### 4.23 old analyzer output schema는 Logical/Physical 계층으로 분해해 흡수한다

`japchae`의 analyzer output schema v0.1은 중요한 prototype이지만 logical 정보와 target-dependent resource 정보가 한 객체에 섞여 있었다.

**Logical Array IR / Plan**에는 op identity, logical shape, rank/cell/frame, dtype/numeric constraints, iteration domain, axis semantics, access relations, effect/alias/speculation, control-flow, constraint witness, semantic storage/lifetime requirement, target-independent rewrite/fusion constraints, provenance를 둔다.

**Route Selection 입력**은 Logical Array IR과 adapter/backend capabilities다. RustJ-native route를 선택한 경우에만 Physical Planner가 Logical Array IR + TargetProfile + optional CostProfile + backend/library capabilities를 입력으로 받는다.

**Physical Plan / Planning Report**에는 fusion/kernel region, materialization boundary, target placement, axis/thread/vector mapping, chosen layout, tiling, memory-space staging, pipeline stages, synchronization, accumulator realization, placement/transfer, buffer lifetime/reuse, backend strategy, derived ResourceEstimate를 둔다.

compile artifact에는 source revision, PrimitiveSpec registry version, input facts/spec, TargetProfile version, optional CostProfile version, compiler version을 provenance로 기록한다.

### 4.24 Flow–Storage: semantic storage requirement와 physical materialization을 분리한다

과거 `japchae`와 `jaxa-analyzer`에서 발전한 Flow–Storage 아이디어는 현재 RustJ의 `ValueId` / `BufferId` 분리와 결합한다.

> **Logical ArrayValue가 존재한다는 사실은 별도의 memory buffer가 존재한다는 뜻이 아니다.**

Logical IR에는 “반드시 독립적으로 관찰·보존되어야 하는가”라는 **semantic storage requirement**만 둔다.

```text
StorageRequirement
  EphemeralAllowed
  MustSurvive(region_or_effect_boundary)
  Persistent(resource_id)
  ExternalVisible
  ExplicitCheckpoint
```

반면 다음은 physical/backend 결정이다.

```text
MaterializationDecision
  KeepVirtual
  FuseAway
  RegisterResident
  ScratchpadResident
  Bufferize(memory_space)
  ExternalResource
```

따라서 Logical IR에서 `Materialize`를 일반적인 실행 op처럼 남발하지 않는다. explicit checkpoint처럼 **J/RustJ semantics 자체가 저장을 요구하는 경우**에만 semantic storage op/requirement가 존재한다.

persistent state는 primitive 내부 hidden state로 숨기지 않는다. scalar state도 J 의미상 rank-0 array로 취급한다.

Logical 단계에서는 `Read`, `Write`, `Accumulate`, `Load`, `Alias/View` 및 `StorageRequirement`를 표현하고, 실제 register/shared/device/host buffer, allocation, offset, reuse, copy는 bufferization/physical planning 또는 외부 compiler가 정한다.

이 원칙은 MLIR이 tensor-level optimization 뒤에 bufferization을 늦추는 구조와 IREE Stream이 tensor computation 뒤에 resource lifetime/allocation을 명시화하는 구조를 따른다.

#### 4.24.1 destination / alias relation은 buffer identity와 분리한다

late bufferization을 잘 지원하려면 op가 결과와 operand 사이의 **semantic alias/destination 가능성**을 설명할 수 있어야 한다.

```text
DestinationRelation
  FreshResult
  MayReuse(operand)
  EquivalentView(operand, mapping)
  OverlappingView(operand, relation)
  MustAlias(resource)
  Unknown
```

이 relation은 `BufferId`를 미리 배정하는 것이 아니다. native bufferization이나 MLIR One-Shot Bufferize 같은 후속 단계가 SSA use-def, liveness, conflict를 함께 보고 실제 in-place/out-of-place 결정을 내릴 수 있게 하는 contract다.


### 4.25 과거 custom primitive inventory는 후보 목록으로 보존한다

`JAXA-complier`의 마지막 prototype registry에는 `conv`, `depthwise_conv`, `linear`, `bn`, `ln`, `adam`, `cp`, `flatten`, `relu`, `gelu`, `softmax`, `scaled_dot_product_attn`, `crossentropy`, `avgpool2d`, `maxpool2d`, `dropout`이 있었다.

이 목록을 그대로 RustJ의 확정 vocabulary로 간주하지 않는다. **역사적 candidate inventory**다.

새 RustJ registry에 들어가려면 최소한 part of speech/valence, innate rank, parameter schema, shape/dtype/numeric rule, iteration domain, axis semantics, access relations, dependency/effect/alias contract, semantic reference 또는 충분한 semantic specification, conservative execution/lowering path, 필요한 realization/resource requirement model을 갖춰야 한다.

초기 구현 우선순위는 가장 작은 end-to-end 검증이 가능한 `relu`, `linear`, `conv2d`, `flatten/static-reindex`, `avgpool2d` 또는 단순 reduction으로 둔다. attention, optimizer, checkpoint/training-specific extension은 core registry 구조가 검증된 뒤 단계적으로 옮긴다.

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

- SSA ValueId와 region/block/control-flow 구조
- normalized array operation
- dtype/shape/rank/cell/frame facts
- iteration domain / axis semantics / access relation
- symbolic constraints와 witness/guard
- data dependency와 effect ordering token
- effect / alias / speculation facts
- uniformity / mask facts
- semantic StorageRequirement
- target-independent rewrite/fusion constraints
- source/semantic origin metadata

특정 backend support 여부, concrete fusion region, buffer allocation, tile/layout/device 결정은 Logical IR의 본질적 fact가 아니다.

아직 특정 device buffer 주소나 CUDA launch parameter는 없다.

### 5.2 Schedule / Transform Plan과 Physical Plan을 구분한다

RustJ-native route에서는 Logical IR을 바로 buffer plan으로 덮어쓰지 않는다.

```text
Logical Array IR
   ↓
Logical Optimizer
   ↓
Schedule / Transform Plan
   ↓
Physical Planner / Bufferization
   ↓
Physical Plan
```

**Schedule / Transform Plan**은 payload semantics와 분리된 선택/변환 의도를 표현한다.

- fusion/grouping
- tile hierarchy
- loop/axis mapping
- vectorization
- unrolling
- tensorization/intrinsic selection 후보
- layout transform 요청
- memory-space staging 요청
- software pipeline/prefetch 전략

이는 MLIR Transform dialect나 TVM TensorIR schedule처럼 “무엇을 계산하는가”와 “어떻게 변환할 것인가”를 분리하는 역할이다.

Schedule Plan은 여러 후보를 가질 수 있고 CostProfile/autotuning/backend feedback에 의해 바뀔 수 있다. 따라서 Logical IR의 semantic identity가 아니다.

**Physical Plan**은 선택된 schedule을 실제 resource/buffer/execution 객체로 구체화한다.

- CPU/GPU placement
- buffer binding / ownership
- physical view
- concrete materialization/copy
- contiguous/fixed/general stride specialization
- physical layout / padding / alignment
- memory-space assignment
- transfer
- synchronization/timepoint
- buffer reuse
- work partition
- backend kernel/library 선택
- async lifetime/resource information

### 5.3 RustJ-native Executor

이 절은 Route A에만 적용한다. RustJ-native Executor는 이미 정해진 Physical Plan을 수행한다. 외부 compiler/runtime route는 각 시스템의 executor/runtime가 자체 lower-level scheduling을 수행할 수 있다.

Executor가 다음을 다시 판단해서는 안 된다.

- J rank 의미
- hook/fork 의미
- fusion 여부
- layout 선택
- device 선택
- buffer reuse legality

Physical Plan에서 비동기 실행을 허용할 경우 dependency는 implicit host order에 기대지 않고 `AsyncToken/Timepoint` 또는 동등한 explicit edge로 표현한다. resource의 사용 가능 시점과 lifetime은 이 timeline과 연결한다. IREE Stream의 timepoint/resource model과 MLIR Async의 explicit dependency token이 참고 모델이다.

### 5.4 실행 경로는 하나가 아니다

`Logical Array IR`을 만든 이후 반드시 RustJ의 Physical Planner를 거쳐야 하는 것은 아니다.

#### Route A — RustJ native

```text
Logical Array IR
  → RustJ Logical Optimizer
  → RustJ Physical Planner
  → Physical Plan
  → RustJ Executor
```

장점:

- J-specific semantics와 실험적 hardware model을 가장 직접적으로 제어
- reference/bootstrap path
- external compiler와 결과 비교 가능

#### Route B — MLIR

```text
Logical Array IR
  → RustJ-to-MLIR adapter
  → tensor/linalg/arith/scf
  → vector/gpu/memref
  → LLVM / NVVM / ROCDL / SPIR-V
  → execution
```

MLIR Linalg는 generic indexing map/iterator semantics를 이용해 tiling, fusion, vectorization, loop lowering, library/intrinsic lowering을 제공하도록 설계되어 있다. RustJ의 `IterationDomain + AccessRelation`은 이 계층으로 내리기 좋은 형태를 목표로 한다.

RustJ가 MLIR의 최적화 passes를 재구현할 이유가 없다. 다만 J의 observable semantics를 위반할 수 있는 reassociation, error-order 변경 등의 lowering은 adapter가 막거나 필요한 attributes/guards를 제공해야 한다.

#### Route C — StableHLO / OpenXLA-compatible subset

NN/tensor 중심의 일부 LogicalOp은 StableHLO로 자연스럽게 표현될 수 있다.

```text
Logical Array IR subset
  → StableHLO
  → XLA / IREE / compatible compiler
```

StableHLO에는 token 기반 side-effect ordering, send/recv, side-effecting `custom_call` 같은 기능이 존재한다. 그러나 이것이 arbitrary J state/effect/error semantics 전체를 표현한다는 뜻은 아니다. J-specific entity, boxed semantics, unusual numeric/error ordering, 지원되지 않는 effect/resource model은 이 route에 억지로 넣지 않는다.

필요하면 StableHLO `composite`나 `custom_call` 계열 escape hatch를 사용할 수 있지만, 그것이 semantic contract를 숨기는 수단이 되어서는 안 된다. adapter는 effect/token mapping을 명시적으로 검증한다.

#### Route D — Direct external library/kernel

```text
LogicalOp / PhysicalRegion
  → verified library mapping
  → BLAS / FFT / vendor NN library / custom kernel
```

library call은 하나의 backend realization이며 primitive identity와 분리한다.

### 5.5 외부 IR을 사용할 때 RustJ가 끝까지 책임지는 것

외부 compiler에 넘긴다고 해도 다음 책임은 RustJ에 남는다.

- J source semantics
- rank/cell/frame/agreement
- primitive/derived-verb identity의 올바른 해석
- dtype/promotion/error contract
- effects/alias legality
- numeric relaxation/reassociation 허용 여부
- dynamic shape guard
- external lowering precondition
- unsupported case detection
- provenance와 differential validation

반대로 register allocation, instruction selection, generic tiling/vectorization, machine-code generation처럼 이미 성숙한 외부 compiler가 더 잘하는 부분은 위임할 수 있다.

### 5.6 외부 IR 선택 원칙

하나의 외부 IR에 전체 RustJ를 맞추지 않는다.

- **MLIR**: 가장 일반적인 multi-level lowering 후보. custom dialect도 가능하고 Linalg/Vector/GPU/LLVM/SPIR-V 등으로 점진 lowering 가능.
- **LLVM IR**: CPU 및 low-level codegen target. J의 high-level array semantics를 직접 담는 주 IR로 사용하지 않는다.
- **SPIR-V**: Vulkan/OpenCL 계열 compute target용 low-level portable binary IR.
- **NVVM / ROCDL**: NVIDIA/AMD-specific LLVM-level GPU lowering.
- **StableHLO**: ML/tensor op subset의 portable high-level interchange. J 전체 semantic IR의 대체재로 보지 않는다.

external route의 존재 때문에 RustJ Logical IR을 외부 IR의 최소공배수로 축소하지 않는다. **RustJ IR이 더 풍부하고, adapter가 필요한 subset을 projection하는 구조**를 유지한다.



### 5.7 Logical IR은 verifier·interface·version 경계를 가진다

Logical Array IR이 여러 route의 compiler boundary라면 단순 Rust struct 집합으로 끝내지 않는다.

#### 5.7.1 Verifier

각 operation은 생성/변환 후 최소 다음을 검증할 수 있어야 한다.

```text
structural verifier
type/dtype verifier
rank/shape verifier
region/block/terminator verifier
effect/token verifier
constraint/witness verifier
op-specific semantic verifier
```

invalid IR을 downstream optimizer가 추측해서 고치게 하지 않는다.

#### 5.7.2 Capability interfaces

분석/변환은 concrete op 이름의 거대한 switch보다 capability interface를 우선한다.

```text
ShapeInference
AxisAndIterationSemantics
AccessPattern
EffectSemantics
AliasSemantics
SpeculationSemantics
TilingCapability
BufferizationCapability
ExternalLoweringCapability
```

모든 op가 모든 interface를 구현할 필요는 없다. interface가 없으면 해당 optimization/route가 conservative하게 거부되거나 fallback 후보를 찾는다.

#### 5.7.3 Canonicalization과 rewrite provenance

canonicalization은 semantic-preserving rewrite만 포함한다. J-specific structure를 없애는 rewrite와 target-specific optimization을 같은 canonicalization 단계로 섞지 않는다.

각 nontrivial lowering/rewrite는 가능하면 source/semantic origin을 추적하여 differential debugging이 가능하게 한다.

#### 5.7.4 IR serialization/versioning

현재 개발 단계에서는 RustJ Logical IR의 장기 binary compatibility를 약속하지 않는다. 그러나 외부 tool/process와 IR을 교환하기 시작하면 schema version을 명시한다.

```text
IrSchemaVersion
PrimitiveRegistryVersion
producer/compiler version
feature set
```

portable artifact를 만들 경우 text/debug syntax와 portable serialization contract를 분리하고, version upgrade/downgrade 또는 unsupported-version 진단을 제공한다.

MLIR bytecode의 dialect versioning과 StableHLO/VHLO의 versioned portable artifact 방식이 참고 모델이다. compatibility를 약속하기 전에도 **version field와 verifier를 처음부터 두는 것**이 migration 비용을 줄인다.

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
### A2 — Extension Primitive Registry와 analysis contract

- [ ] extension name을 parser keyword로 만들지 않고 name resolution을 통해 `PrimitiveId`로 해소한다.
- [ ] built-in과 extension이 공유하는 `PrimitiveContract` interface를 정의한다.
- [ ] `PrimitiveSpec`을 semantic identity/version record로 축소하고 semantic capability interface와 lowering/realization registry를 분리한다.
- [ ] innate rank와 cell axis-role contract를 정의한다.
- [ ] `IterationDomain`, `AccessRelation`, `UniformityFact`, `ConstraintSet`, `MaskSemantics`를 정의하여 leading axis보다 일반적인 hardware-relevant logical contract를 만든다.
- [ ] access-pattern taxonomy(Map/Reduce/WindowReduce/Scan/StaticReindex/Gather/Scatter)를 최소 형태로 정의한다.
- [ ] shape/dtype/effect/alias/semantic-reference 계약을 정의한다.
- [ ] `TargetProfile`을 primitive registry와 분리하고 execution hierarchy/register allocation rules/memory & resource coupling/compute & execution scope/sync & memory ordering/data movement/execution mode/ABI capability를 최소 schema로 만든다.
- [ ] hard target facts와 empirical `CostProfile`을 분리한다.
- [ ] Physical Plan에 logical-axis mapping/tile/vector-subgroup-workgroup/memory-space/layout/pipeline 정보를 기록한다.
- [ ] `ResourceEstimate`를 graph + schedule + target의 함수로 계산한다.
- [ ] backend가 실제 register/spill/shared-memory 결과를 돌려주는 `CompiledResourceReport`와 re-plan 경로를 정의한다.
- [ ] `TargetProfile`을 stable facts와 architecture-specific `TargetQueries`로 분리한다.
- [ ] Logical Array IR → MLIR export adapter의 최소 contract를 설계한다.
- [ ] StableHLO로 안전하게 내릴 수 있는 subset을 명시하고 unsupported semantics를 거부하는 규칙을 만든다.
- [ ] resource 함수는 고정 숫자가 아니라 fusion context/target에 대한 함수로 둔다.
- [ ] 첫 extension set(`relu`, `linear`, `conv2d`, `flatten`, reduction/pool)을 port한다.
- [ ] alias를 거쳐도 primitive identity/spec이 보존되는 테스트를 추가한다.
- [ ] standard-J reference definition이 가능한 extension은 차등 oracle test를 추가한다.

완료 조건: 새 NN primitive 하나를 추가할 때 scanner/parser 수정 없이 registry/spec/lowering만 추가하면 되고, Semantic Analyzer가 rank·iteration domain·axis semantics·access relation·numeric/dependency/effect contract를 읽을 수 있으며, RustJ-native route에서는 별도 TargetProfile을 이용해 schedule/ResourceEstimate를 만들고 external route에서는 adapter가 같은 Logical IR contract를 검증해 lowering할 수 있다.

### A3 — Logical IR core, verification, scheduling boundary

- [ ] SSA `ValueId`, Function/Region/Block/Terminator 최소 구조를 정의한다.
- [ ] pure graph region과 CFG region을 구분한다.
- [ ] `ConstraintSet + Witness/Guard`를 정의한다.
- [ ] `EffectSummary + EffectToken + SpeculationSemantics`를 정의한다.
- [ ] `DestinationRelation`을 정의하여 bufferization contract와 BufferId를 분리한다.
- [ ] op verifier framework를 만든다.
- [ ] semantic capability interfaces(Shape/Axis/Access/Effect/Alias/Speculation)를 trait/API로 정의한다.
- [ ] schedule/transform representation을 Logical payload IR과 분리한다.
- [ ] external adapter capability negotiation과 guarded lowering을 정의한다.
- [ ] `IrSchemaVersion`과 registry/compiler provenance를 IR header에 둔다.
- [ ] pure graph, branch, loop, effect token, dynamic guard를 각각 verifier golden test로 만든다.

완료 조건: Logical IR이 RustJ-native planner와 external adapter 양쪽에서 동일한 verifier/interface contract를 통해 소비될 수 있고, buffer/layout/schedule을 넣지 않아도 control/effect/dynamic constraint semantics를 잃지 않는다.

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
  route selection
    ├─ RustJ native Physical Plan
    ├─ MLIR / LLVM / GPU dialects
    ├─ StableHLO-compatible route
    └─ verified library/custom backend
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
### 15.5 외부 역사 저장소

다음 저장소의 설계 내용은 2026-09-30 기준으로 이 문서에 흡수했다.

- `yunskim/JAXA`
- `yunskim/JAXA-complier`
- `yunskim/japchae`
- `yunskim/jaxa-analyzer`

앞으로 새 설계 결정을 이 네 저장소 중 하나에 먼저 기록하고 나중에 RustJ로 옮기는 workflow를 사용하지 않는다. **RustJ `PROJECT.md`가 최초 기록 장소이자 최종 권위 문서**다.

기존 저장소는 prototype 코드, 연구 이력, 참고 구현을 확인할 때만 사용한다.

---

## 16. 다음 작업

현재 가장 먼저 해야 할 compiler architecture 작업은 **J의 verb composition을 보존하는 semantic IR을 확정하고, hardware-aware Logical IR을 외부 compiler와도 공유 가능한 경계로 만드는 것**이다.

순서:

1. 현재 `semantic.rs`, `analysis.rs`, `facts.rs`, `contracts.rs`의 책임을 다시 분류한다.
2. `semantic.rs`가 noun/verb/adverb/conjunction, hook/fork/train, derived verb, rank를 얼마나 보존하는지 감사한다.
3. 부족한 구조를 `J Semantic Array IR`로 명시한다.
4. extension primitive registry의 최소 schema를 Identity / Analysis / Realization로 정의한다.
5. Logical IR의 `IterationDomain`, `AxisSemantics`, `AccessRelation`, `UniformityFact`, `ConstraintSet`, `MaskSemantics`, `NumericSemantics`, `DependencyRequirement` 최소 타입을 정의한다.
6. `TargetProfile` MVP를 execution hierarchy, allocation granularity, resource coupling, register/memory/compute/sync/data-movement/execution-mode/data-layout capability로 정의하고 `CostProfile`과 분리한다.
7. `TargetFacts + TargetQueries` interface를 정의한다.
8. `relu`, 단순 reduction, `linear`, `conv2d` 순으로 logical contract를 작성한다. conv2d에서는 output/reduction/window axes와 X/W/Y access relation을 golden reference로 삼는다.
9. Semantic Analyzer가 semantic IR과 PrimitiveSpec을 함께 읽어 composition/rank/shape/iteration/access/uniformity/constraint/dependency facts를 생성하게 한다.
10. semantic lowering 결과로 `Logical Array IR / Logical Execution Plan`을 만든다.
11. **첫 외부 경로로 MLIR adapter prototype**을 만든다. 최소 목표는 elementwise + reduction + static reindex를 `tensor/linalg/scf` 계층으로 내리고 MLIR verifier를 통과시키는 것이다.
12. LLVM/MLIR ExecutionEngine을 이용한 CPU 실행 경로를 실험하여 RustJ-native CPU executor와 결과를 비교한다.
13. Physical Planner가 Logical Plan + TargetProfile을 받아 axis mapping, tiling, memory-space, layout, materialization, synchronization을 선택하는 최소 `PhysicalRegion`을 만든다.
14. `ResourceEstimate` MVP로 register/scratchpad/concurrency/global-memory traffic/peak materialized bytes/launch count를 계산한다.
15. backend compile 결과를 `CompiledResourceReport`로 받아 accept/re-plan할 수 있는 interface를 만든다.
16. StableHLO export는 `relu/linear/conv/reduction`처럼 의미가 명확히 맞는 subset부터 별도 adapter로 검토한다.
17. fork, reduction derived verb, rank-derived verb, extension alias, conv2d access relation, MLIR roundtrip/verification을 golden test로 검증한다.
18. 기존 LogicalPlan 결과와 의미 동등성을 비교한다.
19. 그 경계를 유지하면서 G2 structural view 작업을 계속한다.

특히 두 가지 shortcut을 금지한다.

- hook/fork/train/adverb/conjunction 정보를 “generic하게 만들기 위해” semantic analysis 이전에 소거하지 않는다.
- 외부 IR을 쓰기 쉽도록 RustJ Logical IR을 외부 IR의 표현력에 맞춰 축소하지 않는다. RustJ IR이 의미의 superset이고 adapter가 안전한 subset을 projection한다.
