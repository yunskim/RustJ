[English](PROJECT.md) | **한국어 — 정본(canonical)**

# RustJ 통합 프로젝트 문서

> 상태: **유일한 권위 문서(authoritative project document)**  
> 기준일: 2026-09-30  
> 앞으로 아키텍처, 설계 결정, 구현 계획, 지원 범위, 진행 상태, 검증 정책과 주요 검증 결과는 이 문서에 통합한다.  
> [FOUNDATIONS.ko.md](FOUNDATIONS.ko.md)는 RustJ가 왜 compiler-oriented architecture를 택하는지, interpreter 전통에서 무엇을 보존해야 하는지, 어떤 compiler 설계가 J에서 회귀가 되는지를 규정하는 **필수 설계 기반 문서**다. frontend·Semantic IR·runtime/JIT/AOT 경계·rank/CellApply·target 설계를 변경하기 전 반드시 함께 검토한다.  
> 그 외 개별 설계 보고서·진행 보고서·체크리스트 Markdown 파일은 새로 만들지 않는다. 기계가 생성한 측정 원자료(JSON/JSONL)는 `reports/`에 별도로 보존한다.

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

### 참고 구현(reference implementations)

RustJ는 외부 구현을 하나의 동일한 권위로 취급하지 않고 **역할별 reference implementation**으로 구분한다.

- **`jsoftware/jsource` — J semantic reference / oracle**
  - 언어 의미, parser/name semantics, primitive corner case, rank/agreement, 오류·타입 의미의 기준이다.
  - RustJ의 semantic correctness와 differential test에서 최우선 reference다.

- **ArrayFire — array execution / JIT fusion / multi-backend runtime reference**
  - lazy expression graph, evaluation boundary, kernel JIT fusion, CPU/CUDA/OpenCL/oneAPI backend 선택, device memory·stream·synchronization 관리의 참고 구현이다.
  - RustJ의 Graph/Execution optimization, Physical Planner, external-library route, cost model을 설계할 때 비교한다.
  - **J 언어 의미의 oracle은 아니다.**

- **`jsoftware/math_arrayfire` — J ↔ GPU library adapter/offload reference**
  - J array를 ArrayFire handle로 넘기는 실제 adapter 구현이다.
  - row-major J와 column-major ArrayFire 사이의 layout conversion, backend capability/rank 제한, external handle lifetime과 release/GC 경계를 검토하는 참고 구현으로 사용한다.
  - J의 일반 rank/adverb/derived verb 전체를 GPU compiler로 구현한 사례로 해석하지 않는다.

- **APEX / Co-dfns / TAIL→Futhark 계열 — array-compiler research implementation**
  - morphology/fact analysis, data-parallel compiler representation, high-level parallel IR, fusion·GPU lowering을 비교하는 연구 구현이다.
  - 이들의 제한된 APL subset을 RustJ의 J semantics 제한으로 가져오지 않는다.

따라서 reference 우선순위는 목적별로 다르다.

```text
J semantic correctness       → jsource
array graph/JIT fusion       → ArrayFire
J↔external GPU adapter       → jsoftware/math_arrayfire
array-compiler middle-end    → APEX / Co-dfns / TAIL-Futhark
```

ArrayFire 관련 구체적인 Source → Observation → RustJ 적용·비채택 사항은 §13의 **ArrayFire / J ArrayFire add-on / fusion systems** 절을 따른다.

현재 구현은 목표 compiler pipeline 전체를 완성한 상태가 아니다. 제한된 J frontend와 CPU 직접 실행 경로, Semantic IR, 초기 분석/LogicalPlan, CPU storage/SIMD, sparse/boxed 기초, 읽기 전용 affine PhysicalArray가 함께 존재하는 **전환 단계**다.

### 1.1 JAXA에서 이어받은 설계 원칙 — “NN의 SQL”

JAXA 문서에서는 **“NN의 SQL”**, **“배열 연산의 SQL”**이라는 비유를 사용했다.

이 표현의 핵심은 SQL과 비슷한 문법을 만들거나 범용 신경망 플랫폼을 선언하는 것이 아니다. 과거 JAXA 문서에서 반복해서 강조한 핵심은 다음 한 문장에 가깝다.

> **JAXA specifies logical array intent, not physical execution procedure.**

즉 SQL과 닮은 것은 표면 문법이 아니라 **logical intent와 physical execution plan의 분리**다.

```text
logical array intent
        ↓
legal logical / physical plans
        ↓
resource / cost evaluation
        ↓
selected execution plan
```

사용자는 가능한 한 계산의 의미와 필요한 semantic/storage obligation을 표현하고, 다음 사항은 analyzer/compiler/backend에 맡긴다.

- 어떤 동등한 graph form을 사용할지
- fusion 또는 materialization을 할지
- 어떤 실행 basis와 route를 사용할지
- 어떤 memory/schedule 전략을 사용할지
- target별로 어떤 realization을 선택할지

과거 JAXA 문서의 또 다른 중요한 표현은 다음과 같다.

> **JAXA does not execute fusion — the compiler does.**

이 원칙은 current RustJ에서 다음 구조로 이어진다.

```text
J semantics / FunctionEntity
        ↓
J Graph IR / Graph Basis
        ↓
rewrite / equivalence
        ↓
symbolic resource reasoning
        ↓
Logical Execution IR / Execution Basis
        ↓
physical planning / backend realization
```

이 비유는 RustJ의 현재 제품 범위를 확대해서 주장하기 위한 것이 아니다. current RustJ의 직접 목표는 **full J semantics를 보존하는 J compiler/runtime**이다.

“NN의 SQL”은 다음을 설명하는 historical design direction으로만 사용한다.

1. source에서 physical procedure를 불필요하게 고정하지 않는다.
2. 의미적으로 동등한 실행 방법이 있다면 compiler가 선택할 수 있게 한다.
3. legality를 cost보다 먼저 판단한다.
4. resource/cost model은 의미를 바꾸는 것이 아니라 합법적인 plan 중에서 선택하는 데 사용한다.
5. backend나 optimizer가 발전해도 같은 logical intent를 다시 사용할 수 있게 한다.

따라서 이 프로젝트에서 “NN의 SQL”은 **장대한 범용 플랫폼 선언이 아니라, JAXA에서 이어받은 compiler separation-of-concerns 원칙을 설명하는 표현**으로 취급한다.

### 1.2 핵심 배열 모델 결정 — Logical Array와 Physical Array를 분리한다

RustJ의 가장 중요한 아키텍처 결정 중 하나는 **배열을 논리 계층과 물리 계층으로 분리하는 것**이다.

J 프로그램이 관찰하는 배열의 의미와, 특정 CPU/GPU/backend가 그 배열을 저장·배치·접근하는 방식은 같은 것이 아니다.

```text
Logical Array / J noun
    dtype / J-visible type
    shape
    ordered logical atoms / value
    J-visible representation semantics
      Dense / Boxed / Sparse / ...

            ≠

Physical Array / Representation
    BufferId / storage
    strides
    offset
    concrete layout / tiling
    alignment
    memory space
    CPU / GPU placement
    sharding
    transfer / synchronization
```

이 분리는 단순 구현 편의가 아니라 **semantic boundary**다.

핵심 불변조건은 다음과 같다.

1. **Logical Array는 J 의미를 소유한다.** shape, atom order, boxed/sparse와 같이 J 프로그램이 관찰할 수 있는 의미는 logical/semantic 계층에 남긴다.
2. **Physical Array는 realization을 소유한다.** stride, offset, concrete layout, buffer, memory space, device placement는 representation/physical 계층의 책임이다.
3. **하나의 logical value는 여러 physical representation을 가질 수 있다.** 같은 ValueId가 CPU/GPU 또는 서로 다른 layout의 representation으로 존재할 수 있다.
4. **여러 logical value가 하나의 physical buffer를 재사용할 수 있다.** lifetime이 겹치지 않으면 BufferId reuse가 가능하다.
5. **Logical ArrayValue가 존재한다고 해서 별도 buffer가 존재하는 것은 아니다.** view, fused-away intermediate, rematerialized value는 독립 buffer 없이 존재할 수 있다.
6. reshape/transpose/reverse/slice 같은 연산의 **논리적 의미와 copy/materialization 여부를 분리**한다. copy 여부는 downstream representation/planning이 결정한다.
7. sparse/boxed처럼 J가 관찰하는 representation class는 semantic 영역에 남기되, CSR/COO, pointer/handle 같은 구체 backend encoding은 physical 영역으로 내린다.

따라서 RustJ의 배열 경계는 개념적으로 다음과 같다.

```text
J noun / Logical ArrayValue
        ↓
semantic + graph + logical execution analysis
        ↓
RepresentationFacts / realization choice
        ↓
PhysicalArray
        ↓
BufferId / memory space / layout / device
```

이 원칙 때문에 GraphFacts와 Logical IR에는 stride/offset/device를 넣지 않고, Physical Planner가 실제 layout/buffer/placement를 결정한다. 반대로 backend 편의를 위해 J logical noun의 의미를 tensor framework의 physical layout이나 broadcasting 규칙으로 바꾸지 않는다.

상세 모델은 **§6 “논리 배열과 물리 배열”**과 §4.15.8의 RepresentationFacts 경계를 따른다.

### 1.3 이름 정책

현재 아키텍처에는 `Jaxa`라는 별도 compiler component 이름을 두지 않는다.

과거 `jaxa-analyzer` 저장소와 문서에서 발전한 아이디어는 RustJ의 semantic analysis, logical lowering, resource/fusion planning 설계에 흡수한다. 그러나 현행 설계에서 별도 제품·crate·subsystem의 정체성을 뜻하지 않는다.

현행 용어는 다음으로 통일한다.

```text
Jaxa Analyzer      → J Graph Analyzer + Execution Semantic Lowering + downstream legality/resource analysis
Jaxa lowering      → J Graph IR → Logical Execution IR lowering
Jaxa optimizer     → J Graph algebraic optimizer + Execution Logical Optimizer
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
Word formation / tokenizer
   ↓
Enqueue / glyph-control-name classification + lookup hints
   ↓
J parser-time name lookup / part-of-speech resolution
   ↓
semantic binding
   ↓
J Semantic Construction IR / FunctionEntity
   │
   │ noun / verb / adverb / conjunction
   │ primitive / derived verb
   │ hook / fork / train / @:
   │ rank and other modifier applications
   │ name/binding/version
   │ source span
   ↓
──────────── J Graph IR / Graph Analyzer ────────────
applied J operation graph + syntax-derived graph hints
   │ Pipeline / BranchJoin / Reduce / Rank / ...
   │ graph algebra / rewrite / fusion opportunity
   ↓
────────── Execution Semantic Lowering ──────────
explicit executable dataflow + J semantic facts/checks
   ↓
Verified Logical Execution IR / Plan
   ↓
target-independent execution canonicalization
   ↓
Route Partition / Export
   │
   ├──────── Region(s): RustJ-native
   │              ↓
   │       Logical Optimizer
   │              ↓
   │       Schedule / Transform Plan
   │              ↓
   │       Physical Planner / Bufferization
   │              ↓
   │       Physical Execution Plan
   │              ↓
   │       RustJ Runtime / Executor
   │
   ├──────── Region(s): MLIR family
   │              ↓
   │       RustJ IR export adapter
   │              ↓
   │       tensor/linalg/scf/vector/gpu/...
   │              ↓
   │       LLVM / NVVM / ROCDL / SPIR-V
   │
   ├──────── Region(s): StableHLO-compatible subset
   │              ↓
   │       StableHLO adapter
   │              ↓
   │       XLA / IREE / compatible consumer
   │
   └──────── Region(s): library / foreign backend
                  ↓
          BLAS / vendor library / custom kernel

route boundaries are bridged after representation requirements are known
```

핵심 원칙은 **J의 고수준 배열 변환 구조를 Semantic Analyzer가 보기 전에 없애지 않고, analyzer/lowering 단계가 그 구조를 분석한 뒤 backend-independent logical dataflow로 낮추는 것**이다.

### 2.1 RustJ compiler stage의 위상

RustJ는 하나의 compiler system으로 개발한다. 별도 고유 컴포넌트명을 두기보다 각 compiler stage의 책임을 명확히 분리한다.

- **RustJ frontend / J Semantic Construction IR**: source text를 J word로 나누고 J parsing/binding 의미를 보존한다. immutable `FunctionEntity` graph가 primitive, adverb/conjunction application, hook/fork/train, rank, `@:` 같은 **J 함수 구성 자체**를 표현한다. 이 층은 언어 의미의 canonical source다.
- **J Graph IR / JAXA Array Operation Graph IR**: 완성된 J function을 실제 noun input에 적용한 **배열 연산 graph를 J 문법의 대수로 표현하는 compiler analysis surface**다. `@: → Pipeline`, hook/fork → Branch/Join, `/ → Reduce`, `" → CellParallel`처럼 parser/semantic construction에서 정적으로 유도되는 topology와 optimization hint를 first-class로 기록한다. JAXA의 주 관심사는 이 층이며, 역사적 JAXA의 `basis verb`도 주로 이 층의 **Graph Basis**를 뜻한다.
- **Graph Analyzer / Algebraic Optimizer**: J Graph IR에서 fusion topology, fan-out/fan-in, common-input reuse, retained-value lifetime, reduction/cell parallelism, materialization-elision, 이후 adjoint/VJP fan-out 등을 찾는다. 가능한 경우 **GraphBasis** / rewrite / equivalence rule로 동등한 J graph 후보를 만들 수 있다. 여기서 생성되는 것은 target-independent opportunity와 graph candidate이지 concrete kernel schedule이 아니다.
- **Execution Semantic Lowering**: 선택된 J Graph IR을 explicit dataflow와 normalized array operation으로 이루어진 `Logical Execution IR / Logical Execution Plan`으로 낮춘다. 이 후자의 IR은 J observable semantics, facts/checks, **ExecutionBasis** operation, effect/error ordering, executable dependency를 정확히 표현한다. 여러 execution op가 하나의 J Graph node에서 나올 수 있으므로 모든 op는 J Graph origin을 보존한다.
- **Route Partition / Export**: verified Logical Execution IR 이후 프로그램 전체 또는 일부 region/subgraph를 RustJ-native planning, MLIR, StableHLO-compatible subset, library/custom-kernel 등 검증된 경로에 배정할 수 있다. 하나의 프로그램이 여러 route를 혼합할 수 있다.
- **RustJ-native Schedule / Transform Plan**: Route A에서 fusion/grouping, tiling, vectorization, axis mapping 같은 schedule 선택을 payload Logical IR과 분리해 기록한다.
- **RustJ-native Physical Planner / Bufferization**: 선택된 schedule을 바탕으로 placement, memory space, layout, concrete materialization/copy, buffer binding/reuse, transfer, synchronization을 구체화한다.
- **Backend/Runtime**: 선택된 route의 lower-level IR 또는 Physical Plan을 실행 가능한 artifact로 낮추고 실행한다.

이를 한 문장으로 정의하면:

> **RustJ는 J semantics와 Logical IR을 소유하고, RustJ-native optimizer/planner/runtime 경로와 검증된 external lowering 경로를 함께 제공하는 compiler system이다.**

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
J Semantic Construction IR / FunctionEntity
        ↓
J Graph IR / Array Operation Graph IR
        ↓
Graph Analysis / J-algebra optimization
        ↓
Execution Semantic Lowering
        ↓
Logical Execution IR / Plan
        ↓
Route partition / export
   ├─ RustJ native: Logical Optimizer → Schedule → Physical Plan
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

반대로 parser-produced `Hook`/`Fork` parent, 이들의 중첩으로 이루어진 train, result POS를 가진 derived entity, 그리고 `"` conjunction application 같은 **semantic structure를 Semantic Analyzer가 아는 것은 의도된 설계**다. 이는 별도 `Train` 또는 `Rank(u,r)` special semantic node를 둔다는 뜻이 아니다.


### 2.1.1 두 compiler IR은 목적이 다르며 둘 다 canonical boundary다

RustJ/JAXA에는 서로 다른 질문에 답하는 두 graph IR이 필요하다.

#### A. J Graph IR — “이 배열 계산은 J 대수로 어떤 graph인가?”

이 IR은 **표기/결합 구조에서 optimization 정보를 최대한 정적으로 추출하는 것**이 목적이다.

~~~text
J source / FunctionEntity
        ↓
Applied J Graph

@:          → Pipeline
hook/fork   → Branch / Join
/           → Reduction
"           → Cell application / frame parallelism
;. / window → neighborhood / segment topology   // 지원이 추가될 때
u . v       → contraction topology              // 지원이 추가될 때
adjoint     → reverse graph / fan-out            // 향후
~~~

node는 실제 argument ValueId와 함께 다음 종류의 intrinsic graph 정보를 가질 수 있다.

- J combinator/function identity와 source provenance
- graph form / topology
- J observable evaluation order
- shared/common inputs와 fan-out/fan-in
- producer-consumer chain
- syntactic reduction / rank-cell / window / contraction structure
- intermediate materialization-elision 후보
- retained-value / common-input reuse 후보
- dependency상 parallel branch 후보
- shape/dtype/rank/effect/resource **rule reference 또는 Unknown**
- basis/rewrite/adjoint rule reference가 있을 경우 그 identity

여기서 “hint”는 compiler에게 임의의 optimization을 권하는 pragma가 아니다. **J 문법으로부터 결정적으로 유도된 graph fact/opportunity**다. 다만 actual shape, runtime binding, J error order proof, target resource feasibility가 필요한 결론은 아직 내리지 않는다.

J Graph IR은 일반 SSA DAG의 단순 복사본이 아니다. 예를 들어 fork를 세 call로 즉시 잃어버리지 않고 `Fork/BranchJoin`이라는 algebraic form을 보존하며, `@:` chain을 generic producer-consumer 검색 없이 Pipeline으로 안다.

#### B. Logical Execution IR — “이 J graph를 정확히 실행하려면 어떤 operation/data dependency가 필요한가?”

이 IR은 **J 실행 의미와 compiler lowering contract를 정확히 표현하는 것**이 목적이다.

~~~text
J Graph IR
   ↓ lower/expand
Logical Execution IR

Pipeline
  → op → op → op

Fork
  → right branch ops
  → left branch ops
  → join op

Rank
  → CellApply + frame/cell facts

Insert
  → Reduce + ordering/empty/error semantics
~~~

여기서는 다음이 중심이다.

- explicit SSA-like data dependency
- ExecutionBasisKind/ExecutionBasisPayload
- ResolvedInstantiation / ValueFacts / ValueRoleFacts
- ConstraintSet / FactWitness / SemanticCheck
- EffectSummary / SpeculationSemantics
- AccessFact / DestinationRelation
- J observable error/evaluation ordering
- route/lowering legality의 입력

**한 J Graph node가 여러 Execution op로 펼쳐질 수 있으므로 1:1 대응을 가정하지 않는다.** `j_origin` provenance로 many-to-one 관계를 유지한다.

#### 두 IR 사이의 최적화 책임

~~~text
J Graph IR
  structural/algebraic optimization
  - pipeline / branch-join discovery
  - graph rewrite/equivalence
  - fusion candidate boundaries
  - common-input/lifetime opportunities
  - AD graph construction
        ↓
Logical Execution IR
  semantic/execution optimization
  - checks/proofs
  - basis expansion
  - access composition
  - legal fusion confirmation
  - representation-aware transforms
        ↓
Physical/Schedule
  target feasibility/profitability
~~~

따라서 RustJ는 “모든 최적화를 후자의 IR에서 한다”는 구조를 취하지 않는다. **JAXA가 주장하는 J 표기 대수의 이점은 전자의 IR에서 소비**하고, 후자는 그 결과를 정확히 실행 가능한 compiler contract로 만든다.

**소유권 불변조건:** GraphFacts는 조기 abstract fact이며 Execution Facts의 축약 복사본이 아니다. Graph IR에는 layout/representation, SemanticCheck, EffectSummary, error ordering, ResolvedInstantiation을 넣지 않는다. 공통 dtype/shape/rank transfer rule은 하나의 semantic rule source를 공유하고 lowering에서 drift를 검증한다.

현재 `StructuralOpportunity`는 이 두 층을 연결하는 bridge다. 앞으로는 J Graph IR의 `GraphForm/GraphHint`가 source이며, Execution lowering이 이를 실제 execution ValueId에 투영해 `StructuralOpportunity<ValueId>`를 만든다. 후자가 generic DAG를 다시 pattern-match해서 원래 J topology를 복원하는 경로는 보조 수단으로만 사용한다.

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

### 2.4.1 Semantic operation과 architecture-specific realization을 분리한다

J built-in primitive와 NN/array extension primitive는 **semantic operation 계층에서는 동일한 원칙**으로 관리한다.

예를 들어 `+`, `Reduce(+)`, `MatMul`, `Conv`, `Softmax`, `LayerNorm`, `Attention`은 모두 parser/semantic analysis 이후 target-independent operation으로 표현될 수 있다. RustJ extension primitive는 parser grammar에 하드코딩하지 않는다. **jsource `jtenqueue`가 `spellin(...) -> ds(e)`로 primitive를 가져오는 지점을 일반화한 `PrimitiveResolver`에서 core J primitive와 enabled extension primitive를 함께 resolve한다.** 해당 compile profile에서 primitive로 resolve되지 않은 valid spelling은 기존 J 규칙대로 name/noun/string 등 다음 enqueue classification으로 진행한다.

하드웨어별 정보는 semantic primitive 정의에 넣지 않는다.

```text
Primitive / Extension Semantic Contract
  - valence
  - rank/cell semantics
  - dtype / promotion / overflow
  - shape / axis semantics
  - fill / error / effect semantics
  - op-specific parameters
        ↓
Resolved Logical Operation
        ↓
LoweringRegistry × ArchitectureTarget
        ↓
DeviceProfile
        ↓
CostProfile / RuntimeProfile
        ↓
Schedule / Physical Plan
```

역할을 다음처럼 분리한다.

- **Semantic contract**: J-visible 또는 extension-visible 의미. hardware-independent.
- **ArchitectureTarget**: ISA/subgroup/warp-wave/memory hierarchy/synchronization/special-instruction 등 architecture family의 비교적 안정적인 capability.
- **LoweringCapability / LoweringRegistry**: 특정 resolved semantic operation을 특정 architecture에서 어떤 kernel/library/fused implementation으로 합법적으로 실현할 수 있는지.
- **DeviceProfile**: 같은 architecture 안에서도 device마다 다른 SM/CU 수, memory capacity/bandwidth, cache/resource limits 등.
- **CostProfile / RuntimeProfile**: 동일한 legal lowering 후보 사이의 실측 또는 추정 성능 정보.
- **Schedule / Physical Plan**: tile, vector width, workgroup/thread mapping, memory space, concrete layout/buffer/reuse/device placement.

따라서 같은 primitive라도 architecture에 따라 서로 다른 hardware metadata와 lowering 후보를 가질 수 있다.

```text
ResolvedOp::Reduce(Add, f32, axis=0)
  ├─ NVIDIA_SM100
  │    ├─ subgroup/warp reduction
  │    ├─ block reduction
  │    └─ library/custom kernel
  └─ AMD_CDNA4
       ├─ wave reduction
       ├─ LDS reduction
       └─ library/custom kernel
```

NN primitive도 동일하다.

```text
ResolvedOp::Conv(...)
  ├─ CPU        -> direct / im2col / library
  ├─ NVIDIA     -> tensor-core/custom/vendor library
  └─ AMD        -> MFMA/custom/vendor library
```

`Attention`처럼 하나의 semantic op가 여러 primitive sequence 또는 fused kernel로 realization될 수 있으므로 **semantic op 하나 = kernel 하나**로 가정하지 않는다. fused realization은 lowering candidate이며 semantic identity가 아니다.

### 2.4.2 J semantics 보존과 compiler fact 축적 원칙

RustJ의 parser와 J Semantic IR은 **J language semantics를 보존하는 canonical layer**다. 이후 compiler 단계에서 필요한 정보를 얻기 위해 parser-produced entity를 target/optimization 친화적인 형태로 변형하지 않는다.

기본 원칙:

```text
J source
  ↓
jsource-compatible parsing / semantic construction
  ↓
J Semantic IR                         // J meaning is authoritative here
  │
  └─ FunctionEntity
       ├─ identity / POS / operands / span
       └─ semantic_info               // intrinsic, recursively reusable J semantics
            ├─ construction facts
            ├─ valence/rank contract
            ├─ effect/error summary
            ├─ binding/self dependency
            ├─ atomicity/latent policy
            └─ other target-independent intrinsic semantics
        ↓
Logical IR Node
  ├─ origin: FunctionEntityRef
  └─ resolved_facts                   // actual call/argument dependent
       ├─ type/shape
       ├─ effective rank / CellApply
       ├─ agreement/repetition
       ├─ access/effect/error requirements
       └─ constraints
        ↓
Optimizer / Route / Schedule / Physical Plan
```

즉 **J semantic graph는 원본 의미의 기준**이고, 각 stage의 node가 그 stage에서 안정적인 정보를 직접 소유한다. side table은 비싸거나 재계산 가능한 보조 분석의 cache로만 사용하며, 재귀적 semantic 분석에 필요한 intrinsic 정보의 주 저장소로 삼지 않는다.

정보를 다음 수명으로 분리한다.

1. **SourceSemanticIdentity**
   - parser result POS
   - primitive/derived identity
   - Hook/Fork construction
   - modifier semantic operands
   - name/binding semantics
   - source span/provenance

2. **FunctionSemanticInfo**
   - 각 `FunctionEntity`가 직접 소유하는 immutable intrinsic semantic information
   - modifier construction-time facts
   - valence/rank contract
   - effect/name/self/error/atomicity/latent policy summary
   - derived entity를 만들 때 이미 완성된 child node의 `semantic_info`를 재귀적으로 참조하여 bottom-up으로 계산
   - primitive는 `PrimitiveSpec`을 참조/요약하고, named/late-bound entity는 필요한 항목을 `Unknown/May...` 상태로 보존
   - target-independent

3. **ResolvedCallFacts**
   - 실제 valence와 argument facts가 있어야 결정되는 정보
   - effective rank
   - frame/cell split
   - prefix agreement/repetition
   - result type/shape constraints
   - access/effect/error requirements

4. **OptimizationFacts**
   - 의미를 바꾸지 않고 다시 계산 가능한 proof/candidate
   - uniform cell result
   - fusion candidate
   - CellApply absorption/fusion proof
   - materialization-elision candidate

5. **LoweringCapability / Target facts**
   - backend/architecture/device별 realization 정보
   - NN extension과 built-in primitive에 동일하게 적용
   - semantic identity에 포함하지 않음

6. **PhysicalDecision**
   - schedule, tile, workgroup, layout, memory space, buffer reuse, synchronization

중요한 불변조건:

- J semantic entity를 LogicalOp으로 조기에 치환하여 원래 modifier/train 구조를 잃지 않는다.
- `+/`는 semantic layer에서 `/`가 `+`에 적용된 completed J Verb로 남고, `Reduce(Add)`는 lowering 결과다.
- `u"r`는 semantic layer에서 `"` conjunction application으로 남고, effective rank/CellApply는 call analysis 결과다.
- `(+/ % #)`는 semantic layer에서 Fork graph를 유지하며, Mean fusion은 optimizer candidate/proof다.
- semantic node가 hardware target에 따라 달라지지 않는다.
- compiler fact가 필요하다는 이유로 parser에 migration boolean/target hint를 추가하지 않는다.
- 각 `FunctionEntity`를 따라 재귀적으로 내려가면 그 entity의 intrinsic J semantic contract/summary를 다시 외부 lookup 없이 참조할 수 있어야 한다.
- 반대로 target-dependent fact만으로 J semantic identity를 추정하지 않는다.

### 2.5 Route partition은 whole-program exclusive choice가 아니다

Route partition은 parser 직후나 J Semantic IR에서 하지 않는다. **Semantic Analyzer가 의미를 확정하고 Logical IR verifier를 통과한 뒤**, 최소 target-independent canonicalization을 거친 representation에 적용한다.

한 프로그램의 모든 op를 같은 backend/IR로 보내야 한다고 가정하지 않는다.

개념 모델:

```text
Logical Module
  ↓ capability matching / legality
RoutePartition
  ├─ Region A → MLIR
  ├─ Region B → verified library
  ├─ Region C → RustJ native
  └─ Region D → Unsupported
```

architecture 차원에서는 mixed route를 허용하지만, **첫 구현에서는 verified single-block Function/Region 전체를 하나의 route로 보낸다.** 이 단계에서는 boundary bridge가 없다.

그 다음 단계에서만 **single-block contiguous subgraph** partition과 boundary value bridge를 추가하고, 이후 Function/Region 단위의 richer partition으로 확장한다.

즉:

```text
Route-v0
  one verified single-block region
    → exactly one route
    → Lowered | Unsupported

Route-v1
  one verified region
    → contiguous subgraphs
    → mixed route + explicit boundary bridge
```

mixed-route 가능성은 architecture invariant이지만 **A1/A3-v0를 증명하기 위한 선행 구현 요구사항은 아니다.**

`RoutePartition`은 Logical IR의 semantic identity가 아니라 **compilation plan/view**다. 같은 verified Logical IR에 대해 target/backend availability나 cost model이 달라지면 다른 partition plan을 만들 수 있다. canonical Logical IR을 destructive하게 route-specific op로 덮어쓰지 않는다.

각 route region은 다음을 가진다.

```text
RouteRegion
  ops / values
  chosen route
  adapter preconditions
  required witnesses/guards
  boundary inputs/outputs
  semantic provenance
```

partition legality는 단순 op coverage가 아니라 shape/numeric constraint, effect ordering, alias/storage requirement, speculation safety를 함께 본다. effect/token/witness edge를 안전하게 보존할 수 없는 경계에서는 partition하지 않는다.

중요한 원칙:

- route partition 자체는 physical buffer/layout/transfer를 확정하지 않는다.
- 서로 다른 route 사이의 value bridge는 semantic value boundary로 먼저 표현한다.
- 실제 host/device transfer, layout conversion, buffer copy는 각 lower-level route가 필요한 representation을 정한 뒤 bridge lowering에서 구체화한다.
- adapter가 일부 op만 지원하면 지원되는 maximal legal region만 offload할 수 있다.
- unsupported region은 다른 route가 지원하면 그 route로 갈 수 있고, 아무 route도 지원하지 않으면 명시적 `Unsupported`다.

TVM BYOC처럼 external codegen 대상 subgraph를 partition하고 나머지 graph를 기본 pipeline에 남기는 구조를 참고한다. MLIR의 mixed-dialect module도 여러 abstraction/lowering state가 동시에 존재할 수 있다는 점에서 같은 방향을 지원한다.


MLIR은 여러 abstraction의 dialect를 한 module 안에서 공존시키고 dialect conversion으로 점진적으로 lowering할 수 있으므로 RustJ의 multi-level IR 구조와 특히 잘 맞는다.

향후 RustJ Logical IR 전체를 MLIR tooling 안에서 보존할 필요가 생기면 **RustJ-specific MLIR dialect**를 fidelity-preserving bridge로 둘 수 있다. 다만 v0의 필수 구현은 아니다. 초기 adapter는 안전하게 표현 가능한 op만 기존 `tensor/linalg/arith/scf` 등으로 직접 lowering하고, 의미 손실이 생기는 op는 거부한다. StableHLO는 ML framework/compiler 사이의 portability layer를 목표로 하는 high-level op set이므로 NN/tensor subset의 선택적 export 대상으로 본다. LLVM IR/SPIR-V는 더 낮은 execution target으로 사용한다.


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
│   ├─ / : Verb
│   │   └─ + : Verb
│   ├─ % : Verb
│   └─ # : Verb
└─ argument:
    y
```

Semantic Analyzer가 이 구조를 분석한 뒤에야 explicit dataflow로 낮춘다.

### 3.2 jsource에서 확인한 근거

확인 기준: `jsoftware/jsource` master `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528` (2026-09-30 확인), 특히 `jsrc/jtype.h`, `jsrc/cf.c`, `jsrc/jc.h`.

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
│   ├─ Dense / Boxed / Sparse / ...
│   └─ GerundInterpretation when a modifier context requires it
├─ Verb
│   ├─ PrimitiveVerb
│   ├─ ExplicitVerb
│   └─ extension/custom verb
├─ Adverb
│   ├─ PrimitiveAdverb
│   └─ extension/custom adverb
├─ Conjunction
│   ├─ PrimitiveConjunction
│   └─ extension/custom conjunction
├─ NameRef(expected_part_of_speech)
└─ DerivedEntity
    ├─ result_part_of_speech
    ├─ Hook / Fork / Train
    ├─ AdverbApplication
    ├─ ConjunctionApplication
    ├─ RankConjunctionApplication
    ├─ Adverse / Obverse / Power / Agenda / Under / ...
    └─ operands: JEntity...
```

실제 Rust enum을 이 모양 그대로 만들라는 뜻은 아니다. 중요한 것은 다음 invariants다.

1. primitive와 **derived entity(verb/adverb/conjunction)**의 identity와 result part of speech를 보존한다.
2. hook/fork/train의 operand 관계를 보존한다.
3. modifier와 operand의 관계를 보존하고, modifier application 결과가 항상 verb라고 가정하지 않는다.
4. monad/dyad valence를 보존한다.
5. rank가 계산 의미에 미치는 정보를 Semantic Analyzer가 볼 수 있어야 한다.
   `"`의 left/right operand가 verb/noun/gerund/verb-rank form 중 무엇인지 분석 전에 보존한다.
6. name reference와 binding/version이 의미에 영향을 주면 분석 가능한 형태로 보존한다.
7. source span은 진단을 위해 유지한다.

#### 3.3.1 derived entity는 verb로 한정하지 않는다

current jsource의 hook/bident/trident construction(`cf.c`)은 input part-of-speech 조합에 따라 결과로 verb뿐 아니라 adverb/conjunction도 만든다.

따라서:

```text
derive(form, operands)
  -> JEntity {
       result_part_of_speech,
       form,
       operands
     }
```

가 기본 모델이고, `DerivedVerb`는 그중 result POS가 Verb인 경우의 convenience view로 본다.

예를 들어 semantic frontend가:

```text
ADV + ADV + VERB
CONJ + VERB + CONJ
NOUN + CONJ + ADV
...
```

같은 합법 조합을 parser 규칙에 따라 처리했을 때 그 구조와 결과 POS를 표현할 수 있어야 한다. 특정 compiler optimization이 지원하지 않는다고 해서 parser/semantic IR 단계에서 syntax-invalid로 축소하지 않는다.

#### 3.3.2 gerund는 새 atom type이 아니라 contextually interpreted noun이다

J gerund는 일반적으로 boxed noun 표현이며 특정 modifier(`@.`, `^:`, grave 계열, rank의 noun form 등)가 그 noun을 function/entity sequence로 해석한다.

따라서:

```text
Boxed Noun
  + modifier-specific gerund interpretation
      ↓
GerundView / GerundSemantics
  referenced entities / names
  ordering / selection semantics
```

로 다룬다.

boxed noun 자체를 전역적으로 `Gerund`라는 별도 J type으로 바꾸지 않는다. 같은 boxed value가 ordinary data로 쓰이는 문맥과 gerund로 해석되는 문맥을 구분한다.

gerund 안의 name/function reference도 J의 fix/late-binding 규칙을 잃지 않아야 한다.


### 3.3.2 Frontend 전체를 jsource-compatible pipeline으로 유지한다

RustJ는 적어도 **word formation -> enqueue -> parser까지 current jsource를 구현 명세에 가깝게 따른다.** 이 구간에서 compiler-friendly 문법을 새로 만들지 않는다.

```text
source bytes
  ↓
Word Formation
  jsource: jtwordil / w.c state table
  RustJ:   word_former
  ↓
Enqueue
  jsource: jtenqueue
  RustJ:   enqueuer
       ├─ spelling / primitive resolution
       ├─ numeric & string construction
       ├─ name validation / lookup flags
       └─ assignment classification
  ↓
Parse Queue
  ↓
Parser
  jsource: p.c 9-row reduction behavior
  RustJ:   same language/reduction semantics
  ↓
J semantic values / completed function entities
  ↓
RustJ Semantic IR / Analyzer
  ↓
Logical / Optimization / Target / Physical IR
```

RustJ가 달리 구현해도 되는 것은 포인터 low-bit tagging, refcount, in-place bookkeeping, cache layout, branch-prediction tricks 같은 C implementation detail이다. 반대로 **word boundary, enqueue classification, primitive/name distinction, parser lookup timing, parse reduction, result POS/error semantics**은 jsource compatibility 대상이다.

##### Enqueuer의 primitive-resolution hook

jsource `jtenqueue`는 symbol spelling primitive와 ordinary NAME을 구분한다. RustJ도 이 경계를 유지한다. **enqueue 단계에서 primitive로 고정되는 것은 core J primitive spelling뿐**이고, alphabetic extension 이름은 ordinary NAME + lookup metadata로 들어간다.

```text
source "+"
   ↓ enqueue core primitive resolution
PrimitiveHandle(Core(Add))
   ↓
existing parser rows

source "conv"
   ↓ enqueue
NAME("conv", lookup=true)
   ↓ parser-time name lookup
current binding / expected POS
   ↓
extension-derived semantic entity
```

`PrimitiveHandle`의 semantic 정보는 target-independent다. built-in과 extension-derived computational entity는 parser/name-binding 이후 공통 semantic capability와 `lowering_key` 체계로 수렴할 수 있지만, **extension visibility나 현재 binding을 enqueue가 keyword처럼 고정하지 않는다.**

따라서 compile invocation의 `PrimitiveContext`는 core primitive catalog와 extension binding catalog를 함께 가질 수 있어도 역할이 다르다.

- enqueuer: core J spelling만 `resolve_core_for_enqueue`로 해석한다.
- parser/name environment: ordinary NAME의 현재 binding을 lookup하고 extension entity/POS를 결정한다.
- target lowering: semantic entity의 stable lowering key를 active `TargetContext`에서 조회한다.

중요한 규칙:

- Core J primitive의 spelling/POS/semantic contract는 jsource와 호환되어야 한다.
- Extension name은 parser keyword가 아니며 enqueue에서 Verb/Adverb/Conjunction으로 고정하지 않는다.
- extension shadow/rebind는 ordinary J name semantics를 따른다.
- target hardware 정보 자체는 semantic entity에 복사하지 않는다.
- enqueuer에서 target-specific kernel을 선택하지 않는다.
- portable extension은 target implementation이 없더라도 semantic resolution 자체는 가능하며, lowering 단계에서 unsupported/fallback을 판단할 수 있다.
- 명시적으로 target-gated인 extension을 향후 지원하더라도 그 gating은 compile profile의 명시적 정책이어야 하며 core J semantics를 변경해서는 안 된다.

#### 3.3.2.1 Frontend compatibility invariant

`word formation -> enqueue -> parse` 구간은 RustJ가 새 문법을 설계하는 곳이 아니다. 이 세 단계는 **jsource frontend semantics를 충실히 이식**한다.

1. **Word formation / lexer**
   - `w.c::state` state machine을 기준으로 한다.
   - character class, state transition, word boundary emission, follow-on numeric 처리, quote/comment/DD(`{{ }}`) 처리까지 같은 규칙을 사용한다.
   - Rust enum/table로 표현은 바꿀 수 있지만 handwritten heuristic으로 별도 규칙을 만들지 않는다.

2. **Enqueue**
   - `jtenqueue`의 classification order를 기준으로 한다.
   - primitive lookup, constant construction, name validation, assignment classification, lookup-name marking, result parser class/POS를 같은 semantic 순서로 결정한다.
   - jsource의 pointer low-bit QC tagging은 Rust의 explicit enum/flags로 바꾸되 의미는 보존한다.
   - extension primitive는 이 단계의 primitive resolver를 확장해 넣는다. parser grammar에는 extension 전용 production을 추가하지 않는다.

3. **Parser**
   - `p.c::cases[]`와 runtime `ptcol` dispatch의 9-row rule을 기준으로 한다.
   - eligibility, precedence, reduction extent, result POS, reduction 후 stack 재삽입/rescan을 같은 규칙으로 구현한다.
   - Hook/Fork/bident/trident와 modifier construction도 jsource constructor semantics를 따른다.

```text
jsource
  wordil state machine
        ↓
  enqueue classification
        ↓
  9-row parser reduction
        ↓
  J semantic result

RustJ
  same frontend semantics
        ↓
  J Semantic IR
        ↓
  RustJ-specific analyzer / logical / target / physical IR
```

따라서 frontend 단계에서 차이가 허용되는 것은 **representation과 implementation technique**뿐이다. J-visible word formation/classification/parsing behavior는 compatibility 대상이다.

#### 3.3.2.2 Compiler/interpreter/JIT 공통 structured diagnostics

source provenance는 compiler 전용 부가기능이 아니라 frontend semantic infrastructure다. 최신 J의 `d.c` + `eformat_j_` 구조를 참고해 **J error class와 실패 당시의 semantic context를 분리하여 보존한 뒤, 별도 DiagnosticAnalyzer가 설명을 생성**한다. parser conformance 기준 revision은 그대로 유지하고, diagnostic design은 2026-09-30 current jsource master(`1d43f4eb7e43c8243f64dc4e6f31afde4e19d6a9`)의 error-context/eformat 구조도 참고한다.

```text
J-compatible ErrorKind
  syntax / domain / length / rank / ...
        +
ErrorContext
  phase
  SourceSpan
  original word index
  current name
  executing semantic operation
  valence
  small x/y summaries
  structured FailureDetail
        ↓
DiagnosticAnalyzer
  generic semantic explanation
  primitive-specific analyzer
  parser/enqueue explanation
        ↓
Diagnostic
        ↓
Python-style Renderer
  File / line / column
  source excerpt
  caret/range
  readable class
  executing fragment
  argument facts
  semantic explanation
        ↓
AOT compiler / interpreter / JIT / REPL
```

예:

```text
  File "model.ijs", line 27, column 14
    z =: x +"1 y
             ^~~

LengthError: length error
  during execution
  while executing dyad +"1
  x: type 4, rank 2, shape [2, 3]
  y: type 4, rank 2, shape [4, 3]
  argument shapes [2, 3] and [4, 3] do not conform
```

최신 J에서 가져올 핵심은 표시 모양 자체보다 다음 구조다.

- parser stack이 original token number를 유지하고 오류 시 blame token을 추론한다.
- enqueue/parse/execution/assembly 등 **실패 phase**를 구분한다.
- 실행 중인 entity와 monad/dyad valence를 보존한다.
- rank/shape/type/value/index 같은 실제 argument context를 이용해 terse error보다 구체적인 설명을 만든다.
- error formatter가 큰 noun을 복사하거나 hash table을 만들지 않도록 한다.
- formatting/analyzer 오류가 원래 J error를 덮어쓰지 않는다.

RustJ 원칙:

- 내부 provenance는 UTF-8 byte span을 canonical representation으로 사용하고 original enqueue-word index를 별도로 보존한다.
- 표시할 때 1-based line과 Unicode-scalar column으로 변환한다.
- 가장 안쪽 단계가 붙인 정확한 context를 outer compiler/runtime layer가 덮어쓰지 않는다.
- `ErrorContext.arguments`에는 전체 array가 아니라 type/shape/rank 등 작은 summary만 둔다.
- primitive-specific analyzer가 필요한 경우 문자열 메시지를 즉석에서 조립하지 않고 `FailureDetail` 같은 structured failure detail을 추가한다. `DiagnosticAnalyzer`가 이를 human explanation으로 변환하되 J machine error class는 바꾸지 않는다.
- JSON/conformance API와 기존 `eval()/analyze()`는 wrapper를 제거한 J-compatible machine error를 반환한다.
- `parse_diagnostic()/eval_diagnostic()/analyze_diagnostic()`은 같은 semantics를 실행하면서 context를 유지한다.
- runtime kernel error와 향후 physical backend error도 semantic/logical operation의 source origin으로 돌아갈 수 있어야 한다.
- 향후 multi-file/import/definition이 생기면 `SourceId + byte span`으로 일반화한다.
- interpreter/JIT용 별도 error system을 만들지 않는다.

#### 3.3.3 Parser language rules are jsource-compatible

RustJ는 parser 단계에서 별도의 언어 규칙을 발명하지 않는다. **token/word가 J parser에 들어온 뒤 어떤 fragment가 언제 reduction되고, 어떤 part of speech의 결과 entity가 다시 parser stack에 놓이는지는 current jsource의 parser 규칙을 기준으로 한다.**

즉 RustJ와 jsource의 차이는 parser language semantics가 아니라 **parser 이후의 representation/analysis/lowering**에서 만든다.

```text
J words / names
  ↓
jsource-compatible parser reductions
  - row 0: monad
  - row 1: monad after verb phrase
  - row 2: dyad
  - row 3: adverb application
  - row 4: conjunction application
  - row 5: fork
  - row 6: hook / bident / trident
  - row 7: assignment
  - row 8: parentheses
  ↓
J values / completed function entities
  ↓
RustJ Semantic Analyzer
  ↓
RustJ Logical IR / compiler facts
```

RustJ는 jsource의 C parser implementation details(bit-packed parse masks, refcount/inplacing mechanics, function pointers, cache tricks)를 복제할 필요는 없다. 그러나 **reduction eligibility, reduction ordering, result POS, parser-time name lookup semantics, completed modifier entity boundaries**는 호환되어야 한다.

현재의 `reduce_modifier_applications -> collapse_verb_trains -> noun/verb application` 식 staged helper는 구현 과도기이며, 최종 parser semantic model로 간주하지 않는다. 장기적으로는 jsource의 9-row parse behavior를 하나의 parser reduction engine에서 재현하고 differential tests로 검증한다.

#### 3.3.4 Function DAG의 shape는 J parser reduction rule이 결정한다

RustJ가 derived function의 DAG shape를 별도 IR 취향으로 새로 정의하지 않는다. **current jsource의 parser reduction table(`p.c::cases[]`)을 semantic DAG construction의 기준으로 삼는다.**

현재 jsource parse table에서 function construction에 직접 대응하는 핵심 row는 다음과 같다.

```text
row 3
  (VERB | NOUN)  ADV
    → adverb application

row 4
  (VERB | NOUN)  CONJ  (VERB | NOUN)
    → conjunction application

row 5
  (VERB | NOUN)  VERB  VERB
    → fork

row 6
  CAVN  CAVN
    → hook / bident
```

반면 row 0–2는 monad/dyad execution, row 7은 assignment, row 8은 parenthesis reduction이다.

따라서 semantic DAG에서 **modifier vocabulary별 node kind를 새로 발명하지 않는다.** source operator의 품사와 parser production이 parent/operand 관계를 결정한다.

예를 들어 insert는:

```text
source:
  + /

parser row 3:
  left operand = +
  operator     = /

Semantic Function DAG:

/ : Verb                 // applied adverb result
└─ + : Verb
```

이다.

즉 `Insert(+)`라는 별도 semantic node를 만들지 않는다. `/` 자체가 ADV operator identity이며, operand를 가진 derived result의 parent가 된다.

rank도 동일하다.

```text
source:
  u " r

parser row 4:
  left operand  = u
  operator      = "
  right operand = r

Semantic Function DAG:

" : Verb                 // applied conjunction result
├─ left:  u
└─ right: r
```

따라서 `Rank(u,r)` 같은 unary-like special node를 만들지 않는다. **`"` conjunction이 parent이고 양쪽 parser operand를 그대로 가진다.**

hook/fork는 source operator token이 따로 없으므로 parser production 자체가 parent identity다.

```text
Hook
├─ f
└─ g

Fork
├─ f
├─ g
└─ h
```

긴 train도 별도 `LongTrain` node를 만들지 않는다. jsource parser가 row 5/6 reduction을 반복해 hook/fork derived function을 만드는 것처럼 shared Hook/Fork node graph로 축약한다.

#### 3.3.5 jsource `V.fgh`는 parse DAG의 oracle이 아니라 realization cross-check다

jsource의 `V` block은 VERB/ADV/CONJ이 공유하는 execution/function representation이고 `fgh[3]`, valence entry points, rank, id, flags/local metadata를 가진다. 이는 RustJ가 **공통 shared function object**를 쓰고 subtree를 deep-copy하지 않아야 한다는 좋은 reference다.

그러나 `fgh[3]`의 모든 slot을 곧바로 source semantic DAG child로 복제하지 않는다.

이유는 일부 derived form에서 jsource가 parser operand 외에 **실행 최적화용 보조 function/object**도 `f/g/h` 또는 local metadata에 넣기 때문이다.

예:

- insert/reduction의 보조 rank/execution object
- under의 inverse/optimized execution auxiliary
- precomputed tables/hash/info 같은 realization metadata

따라서 authoritative 관계는:

```text
source words
  ↓
jsource parse reduction rule
  ↓
RustJ Semantic Function DAG       // semantic operands only
  ↓
Semantic Analyzer
  ↓
Logical IR
  ↓
runtime/backend specialization
```

이고, `V.fgh`는 **각 parse reduction 결과가 jsource에서 어떤 function object로 realization되는지 검증하는 자료**로 사용한다.

RustJ의 현재 `FunctionEntity` 원칙:

```text
FunctionEntity
  result_part_of_speech
  head:
    PrimitiveVerb
    PrimitiveAdverb
    PrimitiveConjunction
    NameRef
    Hook
    Fork
  operands:
    shared FunctionEntity ref
    semantic Noun operand
  source/provenance
```

같은 operator identity라도 operand가 없고 result POS가 ADV/CONJ이면 source operator entity이고, parser application 뒤에는 같은 operator identity가 operand를 가지며 result POS가 derived result의 품사가 된다.

현재 구현은 이 parser table의 **부분집합**만 지원한다. 따라서 지원 여부를 production 단위로 관리한다.

```text
ParseConstructionCoverage (현재)
  row 3: VERB ADV                  implemented
         NOUN ADV                  pending

  row 4: VERB CONJ NOUN            implemented for supported literal noun
         VERB CONJ VERB            structurally representable; lowering partial
         NOUN CONJ (VERB|NOUN)     pending

  row 5/6:
         edge-bounded pure verb phrase hook/fork
                                      implemented
         full mixed-sentence table   pending
```

미지원 production을 독자적인 다른 DAG로 대신 해석하지 않는다. 해당 production을 구현할 때 jsource parse rule과 result-POS rule을 추가한다.

예:

```text
/ : Adverb
  operands = []

/ : Verb
  operands = [+]
```

```text
" : Conjunction
  operands = []

" : Verb
  operands = [u, r]
```

이 구조는 modifier 수가 늘어도 semantic core에 `Insert/Rank/Power/Under/Fit/...` 식의 거대한 closed enum을 추가하지 않는다. 새 built-in/extension operator는 자신의 identity/POS/semantic contract를 registry에 추가하고, **DAG application shape는 기존 J parser production을 따른다.**

큰 derived expression은 immutable shared handle/`EntityId` DAG로 표현하여 assignment, alias, analysis plan 사이에서 subtree를 복사하지 않는다.

#### 3.3.6 modifier reduction 결과는 다음 parser reduction에서 하나의 function entity다

current jsource의 parser는 이 경계를 명확하게 가진다. `p.c`의 parser lines 3-4는 ADV/CONJ를 실제로 적용해 결과 `yy` function object를 만든 뒤 그 결과의 parsing type을 stack에 다시 기록한다. 따라서 이후 hook/fork/train reduction은 원래 modifier token과 operand token을 다시 보지 않고 **이미 만들어진 결과 entity 하나**를 operand로 받는다.

예를 들어:

```text
source words
  +  /  %  #

parser modifier reduction
  +  /
   ↓
  +/ : VERB entity E2

remaining function phrase
  E2  %  #

fork reduction
  ↓
  Fork(E2, %, #)
```

jsource에서 `ar.c::jtslash`는 실제로 새 function block `z`를 할당하고:

```text
id      = CSLASH
AT/type = VERB
fgh[0]  = original operand verb u
```

를 설치한다. `+/ `의 경우 `fgh[0]`은 primitive `+` object다. `fgh[2]`에는 dyadic execution을 위한 precomputed rank compound가 들어가지만, 이것은 parser-derived semantic operand가 아니라 execution auxiliary이므로 RustJ Semantic IR child로 복제하지 않는다.

그 뒤 `cf.c::jtfolk`는 `f`, `g`, `h`를 이미 완성된 J values/function objects로 받아 새 `CFORK/VERB` function block을 만들고 그대로 `fgh[0..2]`에 저장한다. `(+/%#)` specialization도 다음처럼 **f가 이미 CSLASH verb라는 전제**로 검사한다.

```text
f.id == CSLASH
g.id == CDIV
h.id == CPOUND
f.fgh[0].id == CPLUS
```

따라서 RustJ의 parser/semantic invariant는 다음으로 고정한다.

```text
ModifierApplicationReduction
  operands + modifier
      ↓
  new FunctionEntity(result_pos = ...)
      ↓
  one parser item / one J entity

Hook/Fork reduction
  operands = EntityRef to already-completed entities
```

즉 Fork 내부에 `AdverbApplication(/,+)`라는 아직 미완성 parser fragment가 들어가는 것이 아니라, **`+/`라는 완성된 derived Verb entity에 대한 reference가 들어간다.**

RustJ의 `FunctionEntity`는 이 점에서는 jsource `V` function block의 semantic subset을 직접 따른다.

```text
jsource V/function block            RustJ FunctionEntity
-------------------------           --------------------
AT = VERB/ADV/CONJ                  result_pos
id = CSLASH/CFORK/...               head / construction identity
semantic f/g/h operand              EntityRef operands
execution function pointer          제외: lowering/runtime registry
localuse/cache/rank helper          제외 또는 별도 analysis/lowering metadata
runtime memory flags                제외: downstream proof/physical state
```

이 구조에서:

```text
E1 = + : primitive Verb

E2 = +/ : derived Verb
     head = Insert(/)
     operands = [E1]

E3 = % : primitive Verb
E4 = # : primitive Verb

E5 = Fork : derived Verb
     operands = [E2, E3, E4]
```

가 canonical semantic representation이다.

이 원칙은 모든 modifier에 동일하게 적용한다. `u"r`, `u&v`, `u@v`, `u!.n` 등이 parser production을 완료하면 그 결과는 독립된 J function entity이고, 이후 train/modifier application은 그 entity reference를 operand로 사용한다. execution-only helper를 semantic operand로 승격시키지는 않는다.

semantic function identity/operands와 execution specialization도 분리한다. jsource가 동일한 parser-derived function object에 specialized executor를 선택할 수 있듯 RustJ도:

```text
Semantic FunctionEntity
  parser-derived identity + operands + J semantics

        ↓ analyze/lower

Logical operation / lowering candidate
  generic implementation
  optimized native implementation
  external IR lowering
  architecture-specific specialization
```

로 둔다.


### 3.4 너무 이른 정규화를 금지한다

다음 변환은 **semantic analysis 전에 무조건 수행하지 않는다.**

```text
Fork(f,g,h)
    → g(f(y), h(y))

Hook(f,g)
    → fully expanded expression

AdverbApplication(operator=/, operand=+)
    → Reduce(Add)

ConjunctionApplication(operator=", left_entity, right_entity)
    → logical CellApply after rank resolution
      (uniform result proof 전에는 fixed-shape MapCells로 축소하지 않음)
```

이런 정규화는 합법성과 분석 이득이 확인된 뒤 semantic lowering/rewrite 단계에서 수행한다.

이유:

- fork branch의 독립성/observable-order 판단 가능성
- 공통 argument 사용
- derived verb identity
- primitive composition
- rank propagation
- fusion 후보
- custom semantic annotation
- name/binding semantics

을 분석에 사용할 수 있기 때문이다.

#### 3.4.1 train dataflow와 observable execution order를 분리한다

current jsource의 fork/hook 실행(`j.h`의 `FORK1/FORK2`)은 일반 fork에서 **오른쪽 tine `h`를 먼저 실행하고, 그 다음 왼쪽 tine `f`, 마지막에 middle verb `g`**를 실행한다. hook/composition 계열도 해당 derived-verb 의미에 따른 실행 순서를 가진다.

pure computation만 보면:

```text
h(y) ─┐
      g
f(y) ─┘
```

라는 dataflow가 보이지만 이것만으로 `f`와 `h`가 semantic하게 병렬 독립이라는 뜻은 아니다.

다음이 있으면 observable-order edge를 보존한다.

- name/locale lookup 또는 mutation
- I/O / system foreign
- mutable `StateResource` effect
- catch 가능한 J error / throw
- runtime semantic context mutation
- 그 밖의 non-pure / non-speculatable behavior

개념적으로:

```text
ValueDependency
  h_result -> g.right
  f_result -> g.left

ObservableOrder
  h -> f -> g       // effectful/general fork compatibility order
```

두 tine이 pure하고, relevant late-bound name/effect/error dependency가 없으며, `SpeculationSemantics`가 허용한다고 증명된 경우에는 optimizer가 order edge를 제거하여 병렬 실행/fusion할 수 있다.

즉 source-level J 실행 순서를 physical serial schedule로 영구 고정하지는 않지만, **dataflow graph만 보고 순서 제약을 자동 폐기하지 않는다.**

#### 3.4.2 derived verb의 latent semantics를 forward dataflow로 소거하지 않는다

일부 J conjunction/modifier는 현재 forward call의 값 계산만 보면 없어 보여도 **향후 derived behavior**에 의미가 있다.

대표적인 예:

- `u :: v` adverse: `u`의 일반 오류를 잡아 `v`를 실행하지만 `throw.`/exit류는 같은 방식으로 잡지 않는다.
- `u :. v` obverse: forward 실행은 주로 `u`를 사용하지만 inverse 계산에서 `v`가 semantic하게 사용된다.
- rank/power/agenda/under 등도 operand entity와 derived metadata가 이후 변환·실행 의미에 영향을 줄 수 있다.

따라서:

```text
DerivedVerb
  forward semantics
  latent modifier semantics
  operand identities / NameRefs
  error/inverse/control contracts
```

를 필요한 범위에서 보존한다.

`u :. v`를 forward graph가 `u`와 같다는 이유로 그냥 `u`로 canonicalize하면 향후 inverse 의미가 달라진다. `u :: v`를 단순 `u` + unreachable fallback으로 취급하면 error behavior가 달라진다.

Logical lowering이 derived structure를 소거할 수 있는 것은 **그 latent semantics가 이후 프로그램에서 관찰되지 않거나 동등하게 다른 contract로 이전되었다는 proof**가 있을 때뿐이다.

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

이들은 downstream scheduling/bufferization/backend의 책임이다. RustJ-native route에서는 Physical Planner가 담당하고, external route에서는 해당 compiler의 lower-level IR/pass가 담당할 수 있다.

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

current jsource의 **parse reduction table을 DAG shape의 기준**으로 삼고, `V/fgh`는 shared execution-object realization의 cross-check로 사용한다. 현재 `semantic.rs`는 `/`를 ADV, `"`를 CONJ로 token/POS 보존하고 parser row 3/4가 operator-parent FunctionEntity를 만들도록 이전한다. `analysis.rs`는 이 graph를 직접 분석하며 legacy `reduce/rank` field는 runtime compatibility 기간에만 유지한다.

### 3.7 name resolution과 explicit definition의 호출 의미

과거 `jaxa-analyzer`의 jsource 조사에서 확인한 name/local-frame 의미를 현행 IR에서도 보존한다.

핵심 분리:

```text
DefinitionCode
  body / valence / local-layout hints / source

Invocation
  fresh CallFrame
  x/y/u/v/m/n bindings
  runtime local values
  current locale context
```

동일 definition의 재귀 호출은 code를 공유할 수 있지만 **local value frame은 호출마다 독립**이어야 한다.

J의 simple name lookup은 Python의 정적 LOCAL/GLOBAL 이분법과 다르다.

```text
simple name lookup:
  current invocation local table
    ↓ if no bound value
  current locale
    ↓
  locale path
    ↓
  value error
```

definition-time에 local slot/search hint가 존재한다는 사실은 그 name이 항상 local binding이라는 뜻이 아니다. slot이 아직 unbound이면 locale lookup으로 fallback할 수 있다.

assignment도 구분한다.

```text
=.   current invocation/local semantics
=:   locale/public assignment semantics
```

두 copula 모두 namespace write effect만 있는 void statement로 모델링하지 않는다. J sentence 안에서 assignment는 noun/verb/adverb/conjunction 등 **assigned J entity를 결과로 남겨 다른 표현과 결합될 수 있다.**

```text
AssignmentExpr
  target
  kind: Local (=.) | Public (=:)
  rhs: JEntity
  effect: namespace write
  result: assigned JEntity
```

따라서 assignment lowering은:

```text
entity = evaluate/derive RHS
write binding(target, entity)
return entity
```

라는 value+effect 의미를 함께 보존한다. physical in-place assignment optimization은 이 semantic result를 바꾸면 안 된다.

direct/explicit definition의 structured control flow는 장기 Logical IR의 Region/Block으로 낮출 수 있지만, 이 lowering 때문에 다음 의미를 잃으면 안 된다.

- monad/dyad body 구분
- `x y u v m n`의 valence/definition-kind별 binding
- 호출별 local frame 독립성
- local-first + locale fallback lookup
- direct/indirect locative semantics
- recursive call의 독립 frame
- source/control-flow diagnostics

따라서 SSA `ValueId`는 J namespace 자체의 대체물이 아니다. static binding이 증명된 경우에는 SSA value로 낮출 수 있지만, runtime name lookup/assignment semantics가 필요한 곳은 명시적인 name/resource/effect operation 또는 runtime lowering으로 보존한다.

### 3.8 sentence evaluation order와 namespace mutation

J의 parser는 conventional frontend처럼 “문장 전체 AST를 만든 뒤 모든 name을 한 번에 resolve”하는 것으로 의미를 모델링하면 안 된다. current jsource의 `p.c`는 queue를 stack하면서 name lookup, parse reduction, verb execution, assignment를 한 sentence 안에서 진행하며 J의 **우측→좌측 평가 의미**를 실현한다.

따라서 RustJ가 parser/evaluator 구현 방식 자체는 바꾸더라도 다음 observable order를 보존한다.

```text
sentence
  semantic right-to-left evaluation / reduction order
  + name lookup at the point required by J semantics
  + assignment/locale mutation at its semantic execution point
```

특히 다음 shortcut을 금지한다.

```text
문장 시작 환경을 snapshot
→ 모든 NameRef를 그 snapshot으로 resolve
→ 이후 assignment를 한꺼번에 commit
```

이 방식은 같은 sentence 안의 assignment, expunge, locale change, dynamic execution, side effect가 이후/이전 subexpression의 lookup에 미치는 의미를 바꿀 수 있다.

compiler IR에서는 이를 반드시 source-order instruction list로 복제할 필요는 없다. 대신:

- pure subexpression은 dependency/effect proof 뒤 graph로 재배열 가능
- name lookup, assignment, locale mutation, dynamic execute, I/O 등은 `EffectToken` 또는 동등한 semantic sequencing edge를 가져야 함
- noun value snapshot과 function nameref late lookup의 시점 차이를 보존
- assignment는 namespace write effect와 동시에 assigned J entity를 산출하므로 entity/value-flow와 effect-flow 양쪽에 나타남
- optimization이 name/effect boundary를 넘어갈 때 legality proof가 필요

즉 **J의 우측→좌측 parser implementation을 복제하는 것이 목표가 아니라, 그 구현이 만들어내는 observable evaluation/binding order를 IR에 보존하는 것**이 목표다.


---

## 4. J Graph Analyzer와 Execution Semantic Lowering

RustJ middle-end는 하나의 Analyzer가 모든 일을 하는 구조가 아니라 **두 서로 다른 IR과 두 분석 단계**를 가진다.

~~~text
J Semantic Construction IR
        ↓
J Graph IR
        ↓
J Graph Analyzer / Algebraic Optimizer
        ↓
Execution Semantic Lowering
        ↓
Logical Execution IR
~~~

JAXA의 핵심 연구 대상은 첫 번째 middle-end인 **J Graph Analyzer**다. Execution Semantic Lowering은 J 전체를 정확히 실행 가능한 compiler IR로 옮기기 위해 RustJ에서 강화된 두 번째 단계다.

### 4.1 J Graph Analyzer의 책임

- completed FunctionEntity를 실제 noun application과 결합하여 J Graph IR 생성
- primitive / derived verb / adverb / conjunction의 J graph form 분석
- `@:` chain을 Pipeline으로 식별
- hook / fork / train의 branch/fan-out/join topology 식별
- `/`, `"`, 이후 Cut/Dot/Key/Power 같은 J combinator의 graph structure 식별
- shared/common input, producer-consumer chain, retained-value lifetime 후보 도출
- intermediate materialization-elision / fusion / parallel branch 후보 도출
- J observable evaluation-order topology 보존
- primitive별 shape/dtype/rank/effect/resource rule reference 연결 또는 명시적 Unknown
- basis/rewrite/equivalence/adjoint rule을 사용할 수 있는 graph algebra surface 제공
- 동등한 J graph rewrite candidate 생성 및 target-independent graph-level legality 전처리

이 단계에서 **하지 않는 일**:

- actual target의 register/shared memory/tile 수치를 보고 fusion region 확정
- J-visible dynamic check를 제거했다고 가정
- generic execution SSA로 먼저 평탄화한 뒤 J topology를 다시 추측
- concrete bufferization/schedule 선택

### 4.2 Execution Semantic Lowering의 책임

- J Graph node를 explicit execution dataflow로 전개
- monad/dyad valence와 actual call instantiation 확정
- primitive semantic contract 적용
- dtype / shape / cell / frame / agreement / rank-result-assembly fact 전파
- iteration domain / axis semantics / access relation 도출
- CellApply / Reduce / Gather / Contract 등 normalized execution/basis operation 생성
- ConstraintSet / FactWitness / SemanticCheck 생성
- effect / alias / speculation legality 분석
- invariance / semantic storage requirement / representation-side fact의 경계 설정
- J observable error/evaluation ordering 보존
- J Graph node → execution op provenance(`j_origin`) 유지
- verified Logical Execution IR 생성

다음은 이 단계의 책임이 아니다.

- 특정 backend capability를 보고 schedule을 선택하는 일
- fusion region을 실제로 확정하는 일
- optional intermediate를 실제 buffer로 materialize하는 일
- tile/vector/workgroup/layout/device를 고르는 일
- target cost model로 후보를 ranking하는 일

이 결정들은 Route Partition/Export, 외부 compiler, 또는 RustJ-native Logical Optimizer/Physical Planner가 담당한다.

과거 `jaxa-analyzer` 문서에서 “analyzer가 hardware profile을 받아 fusion/resource를 결정한다”고 한 표현과 충돌하지 않도록 용어를 재해석한다.

```text
old "JAXA analyzer"
  ≈ current Semantic Analyzer
    + native Logical Optimizer
    + Schedule / Transform planning
    + Physical Planner / Resource analysis
```

현행 RustJ에서 **Semantic Analyzer라는 좁은 단계만** target-independent다. 과거 analyzer의 hardware-dependent 기능을 버린 것이 아니라 downstream planning 단계로 분리한 것이다.

### 4.3 두 middle-end가 공통으로 하지 않는 일

- source text tokenization
- parser stack 규칙의 재실행
- source를 다시 parse하여 의미를 복원
- 직접 CPU loop 실행
- 직접 CUDA kernel 실행
- Executor 단계에서 의미론을 다시 판단
- 알 수 없는 정보를 임의로 추측

즉 Graph Analyzer와 Execution Lowering은 scanner/parser stack mechanics를 재실행하지 않는다. 대신 parser가 완성한 FunctionEntity와 J Graph IR의 구조를 정식 compiler input으로 사용한다.

### 4.4 Execution semantic lowering 이후의 generic 경계

다른 frontend와 공유할 가능성이 높은 지점은 J Graph IR 이전이 아니라 **J graph analysis와 execution semantic lowering을 마친 뒤의 Logical Execution IR / Plan**이다. J Graph IR은 의도적으로 J-specific하다.

```text
J frontend / FunctionEntity
    ↓
J Graph IR + Graph Analyzer      ← intentionally J-specific
    ↓
Execution Semantic Lowering
    ↓
Logical Execution IR / Plan     ← generic compiler boundary
    ↓
Route partition / export
    ├─ RustJ-native optimizer/planner
    ├─ MLIR
    ├─ StableHLO-compatible subset
    └─ library/custom backend
```

향후 다른 array DSL frontend를 붙이고 싶다면 두 선택이 가능하다.

1. J semantic model을 의도적으로 공유하면 J Semantic Array IR을 생성한다.
2. J와 무관한 frontend라면 자기 semantic analyzer를 거쳐 Logical Array IR / Plan에 합류한다.

따라서 **middle-end를 generic tensor IR consumer처럼 만들기 위해 J의 구조를 일찍 버리지 않는다.**

### 4.4 분석 fact는 typed lattice로 관리하고 semantic error와 분리한다

shape, alias, invariance, effect, binding, constraint 같은 서로 다른 분석 정보를 하나의 범용 `Unknown` 값으로 뭉개지 않는다.

각 fact domain은 자기 lattice를 정의한다. 모든 domain이 동일한 enum을 강제로 공유할 필요는 없지만 공통적으로 다음 개념을 갖는다.

```text
analysis state
  Uninitialized
  Known(T)
  Overdefined / Unknown
  domain-specific bottom/unreachable if needed
```

control-flow merge나 여러 predecessor에서 fact가 합쳐질 때는 domain별 monotonic `join`을 사용한다. MLIR data-flow framework처럼 lattice state는 **분석 지식의 상태**를 나타낸다.

예:

```text
ShapeFact
AliasFact
InvarianceFact
ConstraintFact
EffectFact
BindingFact
```

`Unknown`은 사실을 임의로 꾸며내지 않는다는 뜻이지 곧바로 실행 불가를 뜻하지 않는다. domain과 route에 따라 다음 중 하나가 된다.

- optimization barrier
- runtime witness/guard 필요
- conservative lowering
- external route rejection
- 재분석 조건

특히 `AccessRelation`은 모든 valid J op가 v0부터 완전한 affine/index-map contract를 가져야 한다는 뜻이 아니다.

```text
AccessFact
  Known(AccessRelation)
  Opaque / Unknown
```

로 둘 수 있다.

- `Known`: fusion, locality, vectorization, advanced scheduling 분석 가능
- `Opaque/Unknown`: J semantics 자체는 valid할 수 있으며, access-sensitive optimization의 barrier가 됨
- route가 full access contract를 요구할 때만 해당 route에서 reject/Unsupported
- conservative/native/runtime semantic path가 있으면 실행 자체를 금지하지 않음

따라서 **hardware-aware Logical IR은 hardware-relevant fact를 표현할 수 있어야 하지만, 모든 op가 v0부터 모든 fact를 Known으로 제공해야 한다는 뜻은 아니다.**

중요하게, **J semantic error는 lattice element가 아니다.**

```text
UnreachablePath / UnsatisfiableConstraint
        ≠
JSemanticError(domain/rank/length/value/...)
```

- `UnreachablePath`는 control-flow/constraint 분석 결과다.
- `JSemanticError`는 source semantics에 따라 진단하거나 runtime error behavior로 보존해야 하는 프로그램 의미다.
- 어떤 path에서 반드시 error가 발생한다고 증명되더라도 optimizer는 그 error의 관찰 가능한 순서를 `SpeculationSemantics`와 effect ordering에 따라 보존해야 한다.

따라서 분석 lattice의 bottom/top 개념과 사용자-visible error contract를 같은 `Invalid` 상태로 합치지 않는다.

### 4.5 Primitive contract

semantic analysis에서 모든 primitive는 공통 `PrimitiveContract` interface를 통해 해석한다. built-in J primitive와 name 기반 extension primitive의 **등록 경로는 달라도 분석 interface는 같다.**

이 절의 contract는 “분석기가 반드시 물어볼 수 있어야 하는 질문”을 정의한다. 구체적인 저장 구조는 4.10의 **PrimitiveSpec semantic record + capability interfaces + lowering registry** 분리를 따른다.

semantic 쪽에서 최소한 다음을 표현하거나 명시적으로 `Unknown`으로 둘 수 있어야 한다.

```text
identity / part of speech / supported valences
innate ranks (monad / dyad-left / dyad-right)
shape rule
type / promotion rule (shape/empty/fill context 의존 가능)
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

fusion cost, accumulator realization, register/shared-memory 양, concrete layout, tile 크기, device-specific intrinsic은 semantic identity 자체가 아니다. target-independent semantic capability가 제공한 facts와 downstream TargetProfile/schedule을 바탕으로 native planner 또는 external compiler가 결정한다.

따라서 `PrimitiveContract`는 analyzer가 보는 공통 interface이고, `PrimitiveSpec`은 그 contract를 실제로 제공하는 versioned registry record라는 관계로 사용한다.

### 4.6 과거 jaxa-analyzer 연구에서 가져오는 확장 어휘

`jaxa-analyzer` 연구/prototype 저장소에서 검토한 확장 어휘는 RustJ에 흡수할 때 **표면 품사와 derived structure를 먼저 보존**한다. `JAXA`는 여기서 역사적 연구명일 뿐 현재 RustJ 아키텍처의 별도 컴포넌트명이 아니다.

예:

- `relu`, `flatten`, `softmax`: verb 후보
- `conv`, `linear`, `bn`, `avgpool2d`, `cp`: 역사 prototype에서는 parameterized adverb
- `cast_f32`: 실제 등록 품사와 결합 의미를 보존하여 분석
- `load`, `store`, `emit`, `cp`: 채택 시 storage/effect semantics와 derived structure를 명시
- `with`: **채택하는 ordinary-name conjunction extension**. base entity에 typed semantic annotation/contract를 결합한다.

중요한 원칙은 **확장 어휘도 품사와 composition structure가 분석 정보라면 너무 일찍 평평한 operation으로 만들지 않는 것**이다.

특히 parameterized adverb와 그 결과 verb를 구분한다.

```text
source binding: conv          // Adverb
parameter noun: 1 6 5 5
        ↓ adverb application
DerivedVerb(Conv, params)
        ↓ semantic analysis
LogicalOp::Conv2d(...)
```

즉 `PrimitiveId::Conv2d`는 source spelling `conv`의 품사 identity와 같은 것이 아니다.

`with` 자체는 유지한다. 다만 ordinary J conjunction name이므로 parser spelling special-case를 만들지 않고 일반 name lookup/nameref 의미를 따른다. `with`의 오른쪽은 typed semantic annotation/contract로 제한한다. 예를 들면 adjoint relation, StateResource binding, numeric policy, checkpoint/storage semantic policy, extension-specific semantic constraint는 허용할 수 있다. CUDA block, tile, register budget, 특정 device, physical layout 같은 schedule/physical policy는 `with`에 넣지 않는다.

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

앞으로 위 저장소들은 **historical research/prototype source**다. 설계 결정을 수정할 때 원본을 다시 고쳐 여러 갈래를 유지하지 않고 이 `PROJECT.ko.md`를 갱신한다.

결정 충돌 시 단순한 repository 날짜보다 **같은 주제에 대한 후속 결정**을 우선한다. 대표적인 예:

- 초기 `JAXA`의 complex rank/precision 표기 `"RjP`는 표준 J와 호환되지 않으므로 채택하지 않는다.
- J의 rank conjunction `"`은 표준 J 의미 그대로 유지한다.
- dtype/precision, accumulator precision, layout, hardware resource는 rank 문법에 억지로 넣지 않고 각각 semantic contract와 planning 계층에 둔다.
- 4월 prototype의 고정 `memory_layout`, `tiling_axis`, register 숫자는 최종 semantic identity가 아니다. 6~7월의 identity/realization 분리는 중요한 중간 단계였고, 현행 RustJ에서는 이를 더 분리하여 `PrimitiveSpec semantic record + capability interfaces + lowering registry + TargetProfile`로 사용한다.

#### 4.7.1 역사 저장소와 현행 RustJ의 충돌 해소표

과거 문서의 문장을 그대로 현재 설계로 읽지 않는다. 다음 항목은 명시적으로 재해석한다.

| 과거 JAXA/Japchae 주장 | 현행 RustJ 결정 |
|---|---|
| JAXA graph는 **parse time에 완전히 확정**된다 | parser만으로 충분하지 않다. resolvable name/품사 binding과 Semantic Analyzer를 거쳐 verified Logical IR이 만들어진다. dynamic constraint는 witness/guard로 남을 수 있다. |
| JAXA는 **J 전체가 아닌 제한된 vocabulary 언어**다 | RustJ의 언어 목표는 장기적으로 J 전체 의미다. 다만 hardware-aware Logical Array IR 및 advanced optimization route에 들어갈 수 있는 영역은 별도의 **analyzable array profile/subset**일 수 있다. |
| custom primitive spelling을 enqueue에서 built-in 실패 후 직접 가로챈다 | extension spelling은 reserved keyword가 아니다. ordinary J name environment에 binding으로 등록한다. enqueue는 ordinary NAME/lookup metadata만 만들고, parser가 정상 J name lookup을 통해 현재 binding과 품사를 얻는다. registry 추가 때문에 tokenizer/parser 구현을 수정하지 않는다. |
| `conv`, `linear`은 computational verb다 | 최신 prototype의 표면 품사는 parameterized **adverb**다. noun parameter를 받아 derived computational verb를 만든다. analyzer가 보는 Conv/Linear logical op identity는 이 derived verb에서 나온다. |
| prototype의 `rank_monad/rank_left/rank_right` 값을 builder의 영구 rank로 본다 | builder와 derived verb를 분리한다. rank는 parameter 적용 후 생성된 derived computational verb의 contract에서 확정한다. 예: derived `conv_kd`는 cell rank `k+1`. |
| rank가 같으면 fusion 가능하고 rank 변화가 fusion boundary다 | rank/cell/frame은 중요한 입력이지만 fusion legality의 충분조건이 아니다. access/dependency/effect/storage/speculation과 schedule/target까지 함께 본다. |
| flatten/reshape/transpose는 본질적으로 항상 stride remap이라 copy가 없다 | semantic level에서는 StaticReindex/view 후보일 뿐이다. 실제 view 유지, layout absorption, copy/materialization은 representation과 downstream consumer/target이 결정한다. |
| primitive가 `memory_layout`, `tiling_axis`, register/shared-memory 숫자를 가진다 | semantic primitive에는 axis/access/numeric/effect contract만 둔다. target-dependent resource/layout은 lowering/resource model + TargetProfile + Schedule에서 결정한다. |
| “JAXA analyzer”가 hardware profile을 받아 fusion/resource를 결정한다 | 옛 analyzer가 여러 단계를 한 이름으로 묶었다. 현행 RustJ의 **Semantic Analyzer는 target-independent**이고, hardware-dependent 부분은 RoutePartition 이후 native Schedule/Physical Planner/ResourceEstimate 또는 external compiler가 담당한다. |
| `PrimitiveResourceSpec`에 temporary/register/shared/instruction cost를 함께 둔다 | semantic requirement와 realization/cost를 분리한다. accumulator/reduction/reuse 같은 구조는 capability에, concrete resource expression은 lowering/resource model에, empirical performance는 CostEstimate에 둔다. |
| 모든 materialized array는 compile-time fixed offset을 가져야 정적 모델이 완성된다 | semantic resource identity/lifetime이 정적으로 알려질 수는 있지만 **physical offset은 semantic requirement가 아니다**. native late bufferization이나 external compiler가 실제 allocation/offset을 결정한다. |
| single stream 순서가 region 간 dependency의 기본 보장이다 | 특정 stream 가정은 architecture invariant가 아니다. physical async dependency는 explicit token/timepoint/event 또는 backend dependency로 표현한다. |
| fusion하지 않으면 중간값은 사실상 DRAM으로 간다 | 특정 GPU 구현의 직관일 뿐 architecture invariant가 아니다. cache, persistent kernel, producer-consumer scheduling, external backend가 다른 realization을 선택할 수 있다. 핵심은 logical value와 physical materialization을 분리하는 것이다. |
| mutable optimizer state를 stateful verb 내부에 둘 수 있다 | 후기 Japchae 결정대로 **mutable array state는 verb/primitive 밖의 explicit resource로 드러낸다.** weight, grad, optimizer state, checkpoint는 역할이 아니라 lifetime/effect/storage requirement로 구분한다. |
| weight/grad/activation을 모두 하나의 flat “materialized array object” 종류로 둔다 | J 의미상 모두 array라는 통찰은 유지하지만 IR identity는 분리한다. ephemeral computation result는 SSA `ValueId`, persistent/mutable named state는 `StateResource`, 실제 materialized storage는 downstream `BufferId`다. activation도 저장이 필요할 때만 StorageRequirement/BufferId를 얻는다. |
| `with`에 optimizer/adjoint/hardware/dtype/tile 정보를 모두 넣는다 | `with` **conjunction 자체는 채택**하되, 서로 다른 planning 층을 평평하게 섞는 방식은 채택하지 않는다. `with`는 ordinary J name에 binding된 conjunction으로 typed semantic annotation/contract만 결합하며 tile/device/register 같은 physical policy는 받지 않는다. |
| parameterized layer/verb는 weight storage 때문에 반드시 source name을 가져야 한다 | 계산 entity의 이름과 state resource identity를 분리한다. derived verb는 익명일 수 있고, 필요한 mutable state는 explicit `StateResource` identity로 참조한다. |
| graph를 남기려면 noun reduction/evaluation을 일반적으로 금지해야 한다 | RustJ 전체 J semantics에는 적용하지 않는다. J의 noun/value evaluation은 그대로 보존하고, compiler가 필요한 verb/adverb/conjunction composition을 `J Semantic Array IR`에서 별도로 first-class로 유지한다. |
| `load/store/emit/cp`는 JAXA의 기본 in-band memory vocabulary다 | core J semantics로 자동 채택하지 않는다. 필요한 경우 ordinary name/adverb extension으로 등록하고 `EffectSemantics + StorageRequirement/StateResource` contract를 갖춘 analyzable profile 기능으로 다룬다. |
| semantic/resource contract는 Python registry가 제공한다 | Python은 역사 prototype 구현 선택이다. 현행 RustJ architecture는 Rust capability interfaces, versioned data profiles, external adapter/registry를 사용하며 Python runtime dependency를 요구하지 않는다. |
| `requires_sync=true`가 primitive semantic property다 | logical contract에는 dependency/collective/conflicting-update 요구만 둔다. barrier/event/atomic 등 구체 synchronization은 schedule/target 이후 정한다. |
| `in_place=true`가 primitive의 고정 실행 property다 | semantic 쪽에는 alias/destination legality(`DestinationRelation`)만 둔다. 실제 in-place reuse는 liveness/conflict/bufferization 이후 결정한다. |
| `cuda_family`가 primitive identity 일부다 | backend/architecture 정보는 target lowering locale/registry의 metadata다. source/semantic primitive identity와 분리하고, BackendFamily/ArchitectureTarget/DeviceProfile을 각각 구분한다. |

이 표는 역사 저장소의 아이디어를 폐기한다는 뜻이 아니다. **어느 층에 속하는지를 현재 compiler architecture에 맞게 재배치**하는 기준이다.


### 4.8 확장 primitive는 ordinary name binding으로 추가하고 parser-time lookup을 따른다

NN/array extension은 J의 새로운 keyword나 punctuation을 추가하지 않고 **ordinary name**으로 추가한다.

current jsource(`p.c`, `w.c`)의 중요한 semantic 경계는 다음과 같다.

```text
source word "conv"
      ↓
Tokenizer / Enqueue
  ordinary NAME
  source/name metadata
  lookup-name flag / optional lookup hint
      ↓
J Parser
  name을 stack에 올릴 시점에
  current local/locale environment에서 lookup
      ↓
현재 binding의 실제 품사
  noun | verb | adverb | conjunction
      ↓
J reduction / derived entity construction
```

즉 **enqueue가 ordinary name의 품사를 미리 고정하지 않는다.** parser가 문장을 처리하면서 현재 binding을 lookup하고 그 value/type class를 사용한다.

extension registry의 역할은 reserved-word table이 아니라 **ordinary J name environment를 seed/register할 extension entity와 semantic/lowering metadata를 제공하는 것**이다.

예를 들어 초기 environment에:

```text
conv  -> ExtensionAdverb::Conv
with  -> ExtensionConjunction::With
relu  -> ExtensionVerb::Relu
```

를 binding할 수 있다. 그 뒤 parser는 extension 여부를 알 필요 없이 정상 name lookup을 한다.

중요한 불변식:

1. Tokenizer/Enqueuer는 `conv`, `with` 등의 spelling을 특별 token/POS로 hard-code하지 않는다.
2. Enqueue는 ordinary NAME 및 lookup metadata/hint를 만들 뿐 semantic 품사를 확정하지 않는다.
3. Parser가 해당 name을 사용할 때 **현재 local/locale binding을 lookup**하여 noun/verb/adverb/conjunction class를 얻는다.
4. 사용자가 정상 J binding 규칙으로 extension name을 shadow/rebind하면 그 현재 binding semantics가 우선한다.
5. noun name은 jsource처럼 value로 resolve될 수 있고, 일반 verb/adverb/conjunction name은 late binding을 보존하는 name-reference semantics가 필요하다.
6. 따라서 `f =: conv`를 항상 “현재 Conv builder identity의 정적 복사”라고 가정하지 않는다. J의 name-reference semantics를 보존하고, static binding이 증명된 경우에만 stable extension identity로 specialize한다.
7. parameterized adverb를 실제로 적용하여 derived verb가 만들어진 뒤에 그 derived computational entity의 semantic capability를 분석한다.
8. 새 extension을 추가할 때 tokenizer/enqueuer/parser의 **코드**를 수정하지 않고 ordinary binding data + semantic/lowering capability를 추가한다.

4월 `JAXA-complier` prototype의 “built-in lookup 실패 후 custom registry를 검사” 방식은 사용하지 않는다. extension도 J의 normal name-resolution 경로에 참여한다.

static specialization이 유용할 경우:

```text
NameRef
  name / locative semantics
  expected_part_of_speech
  + optional binding/version guard
  + optional proven ExtensionAdverb::Conv
      ↓
specialized derived entity
```

처럼 guard/proof를 남긴다. name을 compile time에 봤다는 이유만으로 향후 rebinding 가능성을 제거하지 않는다.

current jsource의 nameref execution(`sc.c`)은 lookup된 현재 value가 nameref 생성 시 기대한 품사와 같은지 검사하고, 달라졌으면 domain error를 낸다.

따라서:

```text
NameRef(expected = Verb)
  runtime lookup -> Verb        OK
  runtime lookup -> Adverb      DomainError
  runtime lookup -> Noun        DomainError
  runtime lookup -> undefined   ValueError when invoked/resolved as required
```

처럼 **late binding과 expected part-of-speech contract를 동시에 보존**한다.

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

J built-in이 valid J semantics를 가진다고 해서 반드시 hardware-aware Array Logical path에 들어갈 수 있는 것은 아니다.

현행 RustJ는 두 범위를 구분한다.

```text
RustJ language semantic coverage
  장기적으로 J 전체 의미

Analyzable Array Profile
  shape/rank/effect/access contract가 충분하여
  hardware-aware Logical Array IR과 advanced route에 안전하게 낮출 수 있는 영역
```

따라서 J built-in의 advanced resource/lowering contract가 부족하면 다음 중 하나다.

- semantic LogicalOp까지는 만들되 advanced optimization을 막고 conservative/native lowering 사용
- compiled runtime/effect call 경로로 낮춤
- 아직 구현되지 않은 feature라 명시적 Unsupported

어느 경우든 “유효한 J가 아니다”로 오해하지 않는다.

반면 RustJ 고유 extension primitive/adverb는 언어에 새 의미를 추가하는 것이므로 최소 semantic contract와 적어도 하나의 검증된 lowering/runtime 경로가 없으면 **등록 완료로 보지 않는다.**

Semantic Analyzer/validator는 syntax validity와 compilation eligibility를 구분한다.

```text
SemanticDisposition
  LowerToArrayLogical
  LowerToRuntimeSemantic
  KeepLateBound / RequiresGuard
  UnsupportedImplementation

JDiagnostic
  SyntaxError
  DomainError
  RankError
  LengthError
  ValueError
  ...
```

`UnsupportedImplementation`은 “J에서 잘못된 프로그램”이 아니라 **현재 RustJ 구현/route가 아직 실행하지 못한다**는 뜻이다.

extension registration 자체가 불완전한 경우에는 별도 registry/configuration diagnostic으로 본다.

```text
MissingSemanticContract
MissingLowering
InvalidExtensionRegistration
```

이 구분은 후기 `jaxa-analyzer`의 Validator 오류 분류를 RustJ의 full-J 목표에 맞게 일반화한 것이다.

#### 4.9.1 analyzable profile은 허용 목록/테스트로 고정한다

`Analyzable Array Profile`을 설명 문구로만 두지 않는다. 구현에서는 source/derived form별 compilation disposition을 **명시적 coverage manifest + golden test**로 고정한다.

개념적으로:

```text
CompilationCoverage
  semantic_form
  supported_valence/form constraints
  disposition:
    LowerToArrayLogical
    LowerToRuntimeSemantic
    KeepLateBound / RequiresGuard
    UnsupportedImplementation
  required_known_facts
  supported_routes
  reference/golden tests
```

v0 예:

```text
dyadic +
  → LowerToArrayLogical
  required: shape/type/agreement
  access: known elementwise map
  route: native-cpu

+/ on supported dense numeric cell
  → LowerToArrayLogical
  required: reduction/rank/numeric policy
  route: native-cpu

unknown dynamic named verb
  → KeepLateBound / RuntimeSemantic

system foreign / unsupported boxed-sparse form
  → RuntimeSemantic or UnsupportedImplementation
```

새 기능을 추가할 때 “advanced route인가?”를 문서 토론으로 매번 다시 결정하지 않고 이 manifest와 test를 갱신한다.

중요하게 이 목록은 **J 언어 validity 목록이 아니라 현재 compiler implementation/route eligibility 목록**이다.


### 4.10 PrimitiveSpec은 semantic record이고, realization은 별도 registry/interface다

과거 prototype처럼 primitive 하나에 rank, shape, layout, register, tiling, backend implementation을 모두 넣지 않는다.

현행 모델은 **semantic record + capability interfaces + lowering/realization registry**로 나눈다.

```text
PrimitiveSpec
├─ identity / part of speech / supported valences
├─ innate ranks
│   ├─ monad: RankSpec
│   ├─ dyad-left: RankSpec
│   └─ dyad-right: RankSpec
├─ parameter schema
├─ semantic reference definition (optional)
└─ version / provenance

Primitive semantic interfaces
├─ ShapeInference
├─ TypeSemantics / TypePromotion
├─ AxisAndIterationSemantics
├─ AccessPattern
├─ NumericSemantics
├─ FillAndEmptySemantics
├─ ErrorSemantics
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

parameterized extension은 **surface builder와 derived computational entity를 분리**한다.

예를 들어 source의 `conv` binding은 adverb builder다.

```text
ExtensionBuilderId::Conv
  part_of_speech = Adverb
  parameter_schema = ...
  derive(parameter_noun) -> DerivedPrimitive
```

parameter가 적용된 뒤의 derived entity가 실제 computational contract를 가진다.

```text
DerivedPrimitive::Conv2d(params)
  innate_rank = 3
  ShapeInference
  AxisAndIterationSemantics
  AccessPattern
  NumericSemantics
  EffectSemantics
```

따라서 source-level `conv`와 LogicalOp `Conv2d`를 같은 `PrimitiveId` 하나로 뭉개지 않는다.

반면 NVIDIA/AMD/CPU의 실제 구현 선택은 같은 record 안에 target 숫자로 박지 않고 lowering registry가 제공한다.

```text
LoweringRegistry.lookup(
    resolved_semantic_operation,
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

#### 4.10.1 기존 J primitive도 hardware lowering의 주체다

hardware-aware capability/lowering 체계는 extension primitive에만 적용하지 않는다.

```text
J built-in
  +
  *
  %
  +/
  |:
  {
  $
  ...

extension-derived operation
  Conv2d
  Linear
  ...
```

모두 동일한 원칙을 따른다.

예를 들어 dyadic `+`는 target-independent하게 다음 semantic capability를 제공한다.

```text
PrimitiveId::Plus
  part_of_speech = Verb
  monad contract  = Conjugate/identity-for-supported-real-types semantics
  dyad contract   = Add semantics
  rank / agreement
  ShapeInference
  TypeSemantics
  AxisAndIterationSemantics
  AccessPattern
  NumericSemantics
  EffectSemantics
  RewriteLegality
```

그러나 GPU에서 실제로 실행하려면 별도의 lowering capability가 필요하다.

```text
Lowering bindings for Add
  generic
  CPU family
  GPU family
  CUDA
  ROCm
  architecture-specific override
  MLIR
  StableHLO-compatible lowering
  ...
```

즉:

> **primitive가 hardware-relevant information을 가져야 한다**는 말은 semantic record 안에 특정 GPU의 warp/tile/register 숫자를 넣는다는 뜻이 아니다. primitive identity를 key로 하여 (1) target-independent hardware-relevant semantic capability와 (2) target/backend/architecture별 lowering capability를 조회할 수 있어야 한다는 뜻이다.

개념 흐름:

```text
ResolvedSemanticOp
  source = PrimitiveId::Plus
  valence = Dyad
  semantic_op = Add
  derived_policies = {...}
    │
    ├─ semantic capabilities
    │    iteration/access/numeric/effect
    │
    └─ target lowering lookup
           +
       CompilationTarget
           ↓
       legal lowering candidates
           ↓
       Schedule / Physical realization
```



lowering key는 raw spelling/primitive id 하나가 아니다.

```text
LoweringKey
  source_entity_identity
  resolved_valence
  semantic_operation_kind
  derived semantic policies
    rank/cell context as needed
    fit/tolerance/numeric mode
    storage/effect mode
  representation preconditions (separate)
```

예를 들어 같은 `+` glyph라도 monadic `+ y`와 dyadic `x + y`는 같은 lowering으로 가정하지 않는다. `+/`, `+!.0`, `+"r` 같은 derived form도 semantic analysis 뒤 각각 필요한 reduction/numeric/rank contract를 가진 resolved operation으로 lookup한다.

locale binding은 여러 수준으로 등록할 수 있다.

```text
source primitive + valence handler
semantic op-class handler (ElementwiseAdd, ReductionAdd, ...)
specific derived-op specialization
```

가장 구체적인 binding만 의미가 맞는 경우에 사용한다.

모든 primitive가 모든 architecture에 전용 implementation을 가질 필요는 없다.

```text
architecture-specific binding exists
      → use/consider specialized lowering

otherwise backend-family binding exists
      → use generic backend lowering

otherwise generic/external lowering exists
      → use it if legal

otherwise
      → UnsupportedImplementation for that route/target
```

예를 들어 `|:` transpose는 semantic capability에서 static permutation을 알려주고, target lowering은 consumer가 permutation을 absorb할지, view/layout으로 유지할지, 실제 transpose kernel/copy를 만들지를 결정한다.

따라서 **컴파일 가능한 J built-in computational entity도 extension-derived entity와 동일한 capability/lowering architecture에 참여**한다. 다만 name/locale/control/system foreign처럼 runtime/effect semantics가 중심인 built-in을 억지로 pure array LogicalOp/GPU kernel로 만들지는 않는다.


### 4.11 rank와 axis role은 서로 다른 정보다

J rank가 알려주는 것은 argument를 frame과 cell로 어떻게 나누어 verb를 적용하는가이다. `AxisAndIterationSemantics` capability는 그 cell 내부의 각 축이 연산에서 어떤 역할을 하는가를 알려준다.

```text
full argument shape
        ↓ J rank semantics
frame axes | cell axes
             ↓ AxisAndIterationSemantics
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

conv builder가 parameter noun을 받아 공간 차원을 확정한 뒤 생성하는 **derived conv verb**의 innate rank는 다음처럼 정리한다.

```text
conv1d cell = [C, W]       innate rank 2
conv2d cell = [C, H, W]    innate rank 3
conv3d cell = [C, D, H, W] innate rank 4
```

일반적으로 derived `conv_kd` verb의 innate rank는 `k + 1`이다. 공간 차원 외에 channel 축이 cell 안에 있어야 channel accumulation을 표현할 수 있기 때문이다. source adverb builder `conv` 자체에 하나의 고정 innate rank가 있다고 가정하지 않는다.

개념적인 `AxisRoleSpec`은 cell axis role, reduction axes, parallel axes, window axes, preserved axes, output-axis mapping을 가진다. `C/H/W` 같은 이름은 사람이 읽기 위한 label이고 analyzer는 reduction/parallel/window/static-reindex 같은 역할을 사용한다.

가변 reduction인 표준 J `+/` 같은 연산은 rank/cell 구조에서 axis가 유도된다. 반대로 derived conv verb처럼 축 역할이 연산 정체성에 고정된 연산은 `AxisAndIterationSemantics` capability가 그 역할을 제공한다.

#### 4.11.0 RankSpec은 absolute/infinite/relative rank를 표현한다

J의 rank를 단순 nonnegative `usize`로 모델링하지 않는다.

current jsource의 rank conjunction은 noun rank argument를 1~3개 값으로 해석하고, 음수 rank를 argument rank에 상대적으로 해석한다. infinite rank(`_`)도 별도 의미가 있다.

개념적으로:

```text
RankSpec
  Infinite
  Absolute(n)
  Relative(delta)   // negative rank: max(0, argument_rank + delta)
```

적용 시점:

```text
resolve_rank(RankSpec, argument_rank)
  → EffectiveCellRank
```

예:

```text
"_1 applied to rank-3 argument
  → effective cell rank 2

"_5 applied to rank-3 argument
  → clamp to 0

"_ applied to any argument
  → whole argument / infinite-rank semantics
```

monad/dyad rank list의 **source 원형은 `"` conjunction의 original operand에 보존**한다. Semantic Analyzer가 이를 해석한 뒤 별도의 analysis result로 resolved rank contract를 만들 수 있다.

```text
ResolvedRankContract
  monad_rank
  left_rank
  right_rank
```

`ResolvedRankContract`는 parser-produced Semantic IR node가 아니라 analysis 결과다.

`PrimitiveSpec`의 innate rank는 단수값이 아니라 **monad / dyad-left / dyad-right**별 `RankSpec`이다. 다만 current jsource primitive table의 innate rank는 nonnegative absolute rank 또는 infinite rank이므로, built-in `PrimitiveSpec.innate_rank`에는 `Absolute | Infinite`만 허용한다. `Relative`는 source `"`가 만드는 requested `RankBoundary`에 속한다. implementation integer sentinel과 동일시하지 않고 semantic `RankSpec`/resolved-rank abstraction을 사용한다.

important: jsource 내부의 `RMAX` 같은 sentinel은 implementation representation이다. RustJ IR에서는 `Infinite`를 명시적으로 표현하고 backend integer sentinel에 의존하지 않는다.

또한 source-level rank conjunction을 `RankDerived(verb, integer)`로 고정하지 않는다. current jsource의 `jtqq`는 operand form이 더 넓다.

```text
FunctionEntity
  head = PrimitiveConjunction(Rank)   // source operator identity = "
  operands = [left_operand, right_operand]
  result_pos = parser/operator semantics가 정한 POS
```

분석 가능한 대표 form:

```text
Verb " RankNoun
Verb " Verb            // right verb의 monad/left/right ranks를 사용

Noun " RankNoun
  ├─ well-formed gerund + applicable rank → cyclic-gerund derived verb
  └─ otherwise → noun-derived constant verb semantics
```

따라서 J Semantic Array IR에서는 **원래 left/right operand의 품사와 value/entity identity를 보존**하고, Semantic Analyzer가 J의 rank-conjunction form 규칙을 적용해 derived verb를 만든다.

```text
FunctionEntity(head=", operands=[left, right])
        ↓ J semantic analysis
ResolvedRankDerived {
  source_entity,
  monad/left/right RankSpec,
  execution/assembly semantics
}
        ↓ function application
Logical CellApply
```

`MapCells` 같은 fixed-shape parallel form으로 더 낮추는 것은 `CellApply`의 uniformity/assembly 조건이 증명된 subset에서만 한다.


#### 4.11.1 J agreement는 prefix frame agreement다

J의 dyadic rank agreement를 NumPy-style trailing-dimension broadcasting으로 해석하지 않는다.

current jsource의 rank/atomic dyad 경로는 shared frame prefix를 검사한다.

개념적으로:

```text
left frame  = P ++ LA
right frame = P ++ RA

shared prefix P는 shape가 같아야 한다.
한쪽 frame이 더 길면 짧은 쪽의 cell/frame을 residual frame에 대해 반복 적용한다.
```

explicit rank와 underlying verb rank가 함께 있으면 outer/inner frame 반복으로 세분되지만, 핵심 semantic invariant는 **prefix agreement + cell repetition**이다.

따라서 `AgreementFact`에는 최소한 다음을 보존한다.

```text
AgreementFact
  common_prefix_axes
  left_residual_frame
  right_residual_frame
  repeated_operand / repeated_cell relation
  agreement_error condition
```

zero stride는 이 agreement가 확정된 뒤의 physical realization일 뿐이다.

#### 4.11.2 empty frame은 “아무 일도 하지 않음”이 아니다

current jsource의 rank executor는 처리할 cell 수가 0이어도 단순히 verb 실행을 생략하지 않는다. **fill-cell을 만들어 verb를 의미적으로 실행**하여 결과 cell의 type/shape를 결정하는 경로가 있다.

따라서:

```text
empty iteration domain
  ≠ automatically return empty with guessed dtype/shape
```

이다.

RustJ semantic contract에는 다음을 둘 수 있어야 한다.

```text
FillAndEmptySemantics
  default_fill(type)
  sparse_element_if_any
  fit_fill_override_if_any
  empty-cell/prototype evaluation rule
  result-cell type/shape inference
  J-defined suppressed-vs-propagated error behavior
```

구현이 실제 scalar fill-cell execution을 하지 않아도 된다. static abstract evaluation이나 primitive-specific inference로 대체할 수 있지만 **jsource와 같은 observable result type/shape/error semantics**를 내야 한다.

특히 optimizer가 zero-trip loop를 제거하기 전에 결과 prototype/type/shape가 이미 J 규칙에 따라 확정되어 있어야 한다.

또한 type/domain inference를 dtype pair만의 함수로 만들지 않는다. current jsource의 atomic dyad에는 일반 argument-type 조합에 실행 routine이 없어도 operand가 empty이면 **notional safe type로 취급하여 empty execution이 domain error로 실패하지 않게 하는 경로**가 있다.

따라서:

```text
TypeSemantics(
  operand types,
  shapes / emptiness,
  rank/cell context,
  fill/fit context,
  primitive
) -> result type / conversion / semantic error
```

처럼 context-sensitive할 수 있어야 한다.

```text
TypePromotion(left_dtype, right_dtype) -> dtype
```

하나만으로 J의 empty semantics를 정의하지 않는다. 일반 nonempty domain error와 empty/prototype evaluation에서의 type handling을 분리한다.


#### 4.11.3 rank 결과 assembly는 고정-shape map보다 넓다

current jsource의 `result.h`는 rank/modifier가 여러 cell 결과를 모을 때 **모든 result cell의 type/shape가 첫 cell과 동일하다고 가정하지 않는다.**

정상 fast path는 homogeneous result cell이다.

```text
frame cells
  → f(cell_0) : type T, shape S
  → f(cell_1) : type T, shape S
  → ...
  → result shape = frame ++ S
```

하지만 뒤 cell의 type/shape가 달라지면 jsource는 assembly path로 전환한다.

관찰된 핵심 의미:

- compatible numeric/type 차이는 공통 type priority/promotion으로 assemble할 수 있다.
- result-cell rank/shape가 달라지면 common result-cell shape를 계산하고 필요한 framing fill을 사용한다.
- sparse result가 섞이면 별도 boxed/open assembly 경로가 필요할 수 있다.
- 서로 assemble할 수 없는 type/shape 조합은 assembly error가 된다.
- empty result cell의 type priority도 assembly 결과에 영향을 줄 수 있다.

따라서 RustJ의 rank lowering은 다음을 구분한다.

```text
UniformCellResult
  statically proven same result type/shape
  → regular MapCells / parallel map lowering 가능

DynamicCellResult
  type/shape may differ between cells
  → RankAssemble semantics를 보존
  → runtime semantic lowering 또는
     verifier가 보장된 dedicated assembly lowering 필요
```

개념 contract:

```text
RankAssemblySemantics
  frame
  per-cell result
  compatible-type join
  result-cell shape join
  framing-fill rule
  sparse/boxed interaction
  assembly-error condition
```

GPU에서 각 cell을 병렬 실행하더라도 이 assembly 의미가 없어지지 않는다. backend가 regular dense output을 직접 쓰려면 **cell result type/shape가 uniform하다는 proof**가 선행되어야 한다.



#### 4.11.4 Implicit loop는 모든 function application의 cell semantics다

J의 implicit loop를 `"` conjunction의 구현으로 한정하지 않는다.

current jsource의 구조는 다음 두 개념을 분리한다.

```text
explicit rank conjunction
  u " r
  → CQQ derived function
  → parent operator = "
  → left operand = u
  → right operand = r

implicit cell execution
  function application
  → callable rank contract + actual argument ranks
  → frame/cell split
  → repeated cell execution
  → result assembly
```

즉 `"`는 새 function을 만드는 conjunction이고, implicit loop는 primitive/derived function을 실제 noun argument에 적용할 때 공통으로 필요한 semantics다.

jsource에서 각 function block은 monad / dyad-left / dyad-right rank를 가지며, generic path는 `rank1ex/rank2ex`가 frame을 cell로 나누어 원래 function을 반복 호출한다.

##### 4.11.4.1 jsource generic rank loop의 semantic 단계

monad:

```text
argument rank
  ↓ resolve effective cell rank
frame = leading axes before cell
  ↓
if no frame:
    invoke once
else:
    iterate frame cells
      → logical cell view
      → invoke function
      → assemble results
```

dyad:

```text
left argument  → left frame + left cell
right argument → right frame + right cell
                     ↓
              prefix agreement
                     ↓
   shorter residual frame causes repetition
                     ↓
             invoke on cell pairs
                     ↓
               result assembly
```

current `rank2ex`는 explicit `"` rank와 underlying function rank가 함께 있을 때 **outer frame / inner frame을 따로 계산하고 residual repetition을 각각 보존**한다.

```text
argument
├─ outer frame        // explicit rank boundary 밖
└─ explicit-rank cell
   ├─ inner frame     // underlying action rank 밖
   └─ action cell
```

dyad에서는 left/right에 각각 이 구조가 존재하므로:

```text
outer common prefix
outer residual repeat
inner common prefix
inner residual repeat
action cell pair
```

가 생길 수 있다.

##### 4.11.4.2 nested rank boundary를 early collapse하지 않는다

jsource `cr.c`는 explicit rank와 underlying verb rank를 단순히 하나의 effective rank로 합치지 않는다. 중간 rank boundary의 fill/assembly가 결과를 바꿀 수 있기 때문이다.

따라서:

```text
(u " r1) " r2
```

를 곧바로 `min(r1,r2,innate_rank)` 같은 하나의 rank로 바꾸지 않는다.

semantic lowering은 nested cell-application boundary를 유지한다.

```text
CellApply(r2)
  body:
    CellApply(r1)
      body:
        Invoke(u)
```

`u` 자체의 innate rank가 더 낮아 별도 implicit application이 필요하면 내부에 또 CellApply가 생긴다.

```text
CellApply(explicit outer rank)
  ↓
CellApply(explicit inner rank)
  ↓
CellApply(innate action rank)
  ↓
InvokeCore(u)
```

인접 CellApply를 합치는 것은 허용하지만 다음 proof가 필요하다.

```text
RankLoopFusionLegality
  no intermediate fill distinction
  no dynamic assembly boundary
  no observable error/order difference
  same agreement/repetition semantics
```

##### 4.11.4.3 Logical IR에는 physical for-loop가 아니라 CellApply를 둔다

v0에서 implicit loop를 CFG loop나 GPU thread loop로 바로 낮추지 않는다.

```text
CellApply
  valence
  arguments
  requested/effective cell ranks
  frame semantics
  agreement/repetition plan
  body region:
    cell parameters
    semantic invoke
  empty/prototype policy
  assembly policy
```

monad:

```text
CellApply1
  frame_axes
  cell_axes
  body(cell) -> JValue
  assembly
```

dyad:

```text
CellApply2
  left_frame / left_cell
  right_frame / right_cell
  common_prefix
  left_residual / right_residual
  repeated_side
  body(left_cell, right_cell) -> JValue
  assembly
```

이것은 logical iteration semantics다. downstream은 같은 CellApply를 scalar CPU loop, SIMD, CPU thread-parallel loop, GPU grid/workgroup/lane mapping, primitive-integrated kernel, external compiler lowering 중 하나로 실현할 수 있다.

##### 4.11.4.4 cell projection은 copy가 아니라 view semantics다

jsource generic rank loop는 virtual A block을 만들어 backing array의 cell을 복사하지 않고 순회한다.

RustJ의 semantic/logical 표현:

```text
Argument Value
  ↓ Frame/Cell projection
CellView
  base ValueId
  logical cell shape
  logical index mapping
```

이 단계에서는 BufferId/byte offset을 확정하지 않는다. physical lowering에서 dense affine representation이 알려지면 offset/stride view가 될 수 있고, sparse/boxed는 각 representation에 맞는 cell projection을 사용한다.

##### 4.11.4.5 empty/prototype와 result assembly는 CellApply가 소유한다

jsource `rank1ex/rank2ex`는 cell count가 0이면 loop를 단순히 건너뛰지 않고 fill cell을 만들어 body를 의미적으로 실행해 result-cell type/shape를 정한다.

```text
zero result cells
  ↓
fill cell(s)
  ↓
prototype body invocation
  ↓
J-defined error suppression/propagation
  ↓
result-cell type/shape
  ↓
empty assembled result
```

따라서:

```text
CellApply.empty_policy = JFillCellPrototype
```

이다.

jsource `result.h`처럼 result assembly도 CellApply contract다.

```text
AssemblyPolicy
  UniformProven
  DynamicJAssembly
```

`UniformProven`이면 output을 미리 배치하고 병렬/GPU map으로 낮추기 쉽다.

`DynamicJAssembly`이면 type join, shape join, framing fill, sparse/boxed interaction, assembly error를 보존한다.

초기 GPU route는 `UniformProven` subset만 지원해도 된다.

##### 4.11.4.6 jsource IRS는 semantic feature가 아니라 loop absorption optimization이다

jsource의 `VIRS1/VIRS2` 및 `IRS1/IRS2`는 generic rank semantics와 다른 언어 기능이 아니다. rank 정보를 호출 linkage에 전달해 primitive/derived function이 implicit loop 일부를 내부에서 처리하는 fast path다.

RustJ는 pointer/rank encoding을 복제하지 않고 lowering capability로 일반화한다.

```text
CellApplyLoweringCapability
  GenericCellLoop
  CanAbsorbCellApply {
    valence
    supported rank forms
    agreement capability
    empty/prototype capability
    assembly guarantees
    representation constraints
  }
```

```text
Logical CellApply
  ├─ generic loop around core operation
  ├─ operation/kernel absorbs cell iteration
  └─ external compiler lowering
```

primitive가 loop를 흡수한다고 해서 Semantic IR/Logical semantics에서 implicit CellApply의 의미를 삭제하지 않는다. GPU에서 frame axes를 grid에 자연스럽게 흡수하는 것도 같은 종류의 lowering optimization이다.

##### 4.11.4.7 sparse path는 별도 lowering이어도 semantic CellApply는 같다

jsource가 dense generic rank loop와 `sprank1/sprank2`를 분리하듯 RustJ도 구현은 분리할 수 있다.

```text
CellApply semantics
  ├─ DenseAffine lowering
  ├─ Sparse lowering
  ├─ Boxed/runtime lowering
  └─ External lowering
```

sparse를 dense CellView로 강제하거나 sparse axes/element 의미를 잃지 않는다.

##### 4.11.4.8 구현 계획

**IL0 — parser/semantic boundary**

1. `"`를 special unary Rank node로 만들지 않는다.
2. parser row 4에 따라 `ConjunctionApplication(operator=", left, right)`를 만든다.
3. `"` parent와 양쪽 operand를 shared FunctionEntity DAG에 그대로 보존한다.
4. semantic parser graph의 legacy special-rank 표현은 generic conjunction application으로 교체한다. `Verb.rank`/`Callable.rank` 같은 남은 필드는 semantic identity가 아닌 downstream migration field로만 취급하고 제거한다.

**IL1 — rank contract**

5. 모든 callable에 valence별 innate `RankSpec` contract를 제공한다. primitive의 innate rank는 jsource primitive table처럼 nonnegative absolute rank 또는 infinite rank이며, source `"`의 negative requested rank와 같은 종류의 상태로 취급하지 않는다.
6. parser row 4의 `"` semantic constructor가 jsource `jtqq`와 같은 construction-time legality/rank/length/domain 검증을 수행하고, normalized requested `RankSpec`을 target-independent `ConstructionFacts`/`RankBoundary`로 기록한다. 이 requested rank만 `Absolute / Relative / Infinite`를 가질 수 있다. Analyzer는 이 검증을 처음 수행하는 단계가 아니라 해당 construction fact를 소비하는 단계다.
7. negative/infinite requested rank는 actual argument rank가 알려지는 call analysis에서 jsource `efr` 동등 규칙으로 effective cell rank를 resolve한 뒤 underlying callable의 innate rank와 별도 boundary로 적용한다.
8. jsource `rank2ex`처럼 explicit requested rank와 underlying innate rank의 outer/inner frame을 구분하고, nested `"` boundary를 하나의 rank triple로 평탄화하지 않는다.

**IL2 — pure CellApplication planner**

9. runtime/executor와 독립적인 planner를 만든다.

```text
plan_cell_application(callable, argument facts)
  -> DirectInvoke
   | CellApplyPlan
   | SemanticError
   | Unknown/RequiresRuntime
```

10. monad plan은 frame/cell split과 iteration extent를 계산한다.
11. dyad plan은 prefix agreement, common frame, residual frame, repeated side를 계산한다.
12. explicit rank + underlying rank는 nested outer/inner CellApply로 표현한다.

**IL3 — correctness-first CPU generic executor**

13. `CellApplyPlan`을 그대로 실행하는 reference-style CPU executor를 만든다.
14. dense cell은 copy하지 않고 view로 전달한다.
15. no-frame case는 direct invoke한다.
16. 먼저 optimization 없이 jsource differential result를 맞춘다.

**IL4 — empty/prototype + assembly**

17. zero-cell fill/prototype semantics를 추가한다.
18. homogeneous result fast path를 추가한다.
19. heterogeneous type/shape `DynamicJAssembly`를 추가한다.
20. sparse/boxed result와 assembly error를 golden test로 고정한다.

**IL5 — loop absorption**

21. generic CellApply와 의미 동등성이 확인된 primitive부터 `CanAbsorbCellApply`를 연다.
22. 첫 대상은 dense elementwise dyad와 단순 reduction으로 제한한다.
23. generic executor vs absorbed executor를 differential test한다.
24. nested CellApply fusion은 fill/assembly/error proof가 있을 때만 허용한다.

**IL6 — parallel/GPU lowering**

25. 첫 GPU route는 `UniformProven` CellApply로 제한한다.
26. frame iteration axes를 grid/workgroup/lane에 매핑한다.
27. agreement residual repetition은 logical index map/zero-stride equivalent로 lowering한다.
28. cell body reduction axis와 frame parallel axes를 구분한다.
29. dynamic assembly, catchable per-cell error, unsupported sparse/boxed는 GPU route barrier로 둔다.
30. architecture-specific locale lowering이 CellApply absorption/schedule 후보를 제공한다.

##### 4.11.4.9 첫 differential test matrix

```text
A. direct/no-frame
B. monadic frame iteration
C. dyadic equal-frame cells
D. dyadic prefix agreement + left repeat
E. dyadic prefix agreement + right repeat
F. explicit " rank
G. negative rank
H. nested " boundaries
I. empty frame fill/prototype
J. heterogeneous result-cell assembly
K. sparse argument
L. generic CellApply vs absorbed path equivalence
```

특히 nested rank는 jsource `cr.c`가 명시한 intermediate fill-boundary 사례를 포함한다. 이 테스트가 통과하기 전에는 adjacent rank boundaries를 자동 collapse하지 않는다.

##### 4.11.4.10 jsource 구현에서 가져올 구조적 아이디어

2026-09-30 current jsource master `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`의 `t.c`, `cr.c`, `ar.c`, `cf.c`, `result.h`, `jtype.h`를 기준으로 검토한다.

RustJ는 jsource의 C object layout, pointer/rank bit encoding, function-pointer dispatch를 복제하지 않는다. 대신 오랜 기간 검증된 다음 구조를 compiler architecture로 가져온다.

1. **primitive rank는 semantic contract다.** `t.c`의 monad / dyad-left / dyad-right rank table을 RustJ `PrimitiveSpec`의 기준 자료로 사용한다.
2. **explicit rank와 innate rank는 별도 boundary다.** `jtqq`가 source requested rank를 보존하고 `rank2ex`가 underlying rank와 outer/inner frame을 따로 처리하는 구조를 따른다.
3. **generic cell execution이 correctness baseline이다.** `rank1ex/rank2ex`에 대응하는 generic `CellApply`가 먼저 존재하고 IRS류는 loop-absorption optimization으로 취급한다.
4. **cell projection은 view다.** jsource virtual block처럼 logical cell을 복사하지 않고 projection/view로 표현한다.
5. **result assembly는 독립 semantics다.** `result.h`의 homogeneous fast path, type promotion, dynamic shape/fill, sparse/boxed, assembly error를 `CellApply` assembly contract로 옮긴다.
6. **empty frame은 zero-trip이 아니다.** fill cell/prototype invocation으로 result cell의 type/shape를 결정한다.
7. **reduce identity와 optimized reducer를 분리한다.** `jtslash`가 `u/` identity를 보존하면서 type/primitive별 reducer를 선택하는 것처럼, RustJ는 semantic `/(u)`에서 Logical `Reduce(u)`로 낮춘 뒤 realization을 선택한다.
8. **structural specialization은 원래 graph를 지우지 않는다.** `cf.c`가 `(+/ % #)`를 `jtmean`으로 specialize해도 `CFORK`와 f/g/h를 보존하는 방식을 따른다.
9. **sparse는 같은 semantics의 다른 realization이다.** dense와 sparse의 Logical `CellApply/Reduce` 의미는 공유하고 lowering/assembly 구현을 분리한다.
10. **jsource optimization flag는 의미를 재분류한 뒤에만 사용한다.** `VIRS*`, `VFUSEDOK2`, `WILLOPEN`, in-place/pristine flag를 그대로 Semantic IR bit로 가져오지 않는다.

대표 대응은 다음과 같다.

| jsource 개념 | RustJ에서의 의미 |
|---|---|
| primitive `mr/lr/rr` | `PrimitiveSpec.innate_rank` |
| `u"n` saved requested ranks | `RankBoundary` / requested `RankSpec` |
| `rank1ex/rank2ex` | Logical `CellApply` + generic CPU reference executor |
| `VIRS1/VIRS2` | `CanAbsorbCellApply` lowering capability |
| `VISATOMIC1/2` | atomic/cell semantic fact + realization capability |
| `VFUSEDOK2` | native lowering/kernel capability |
| `VNOLOCCHG` | effect summary의 `NoLocaleMutation` 증거 |
| `VNONAME/VNOSELF` | binding/self dependency summary의 참고 증거 |
| `VF2RANKATOP/RANKONLY` | Logical IR 이후 `CellApply` fusion candidate/proof |
| `VF2BOXATOP/ATOPOPEN/WILLOPEN` | explicit box/open semantics + use-def 기반 materialization-elision candidate |
| `VF2USESITEMCOUNT*` | shape/item-count fact 소비 capability |
| in-place/pristine/zappable flags | liveness/alias/bufferization 이후 physical reuse legality |
| pointer/rank bit encoding | 가져오지 않음 |

##### 4.11.4.11 이후 IR에서 사용할 정보의 provenance와 삽입 시점

jsource 조사에서 얻은 정보는 **가능한 한 이른 시점**이 아니라 **의미를 정확히 알 수 있는 가장 이른 시점**에 넣는다. Parser가 알 수 없는 fact를 Parser IR에 미리 넣거나, target-dependent hint를 Logical IR semantic payload에 박지 않는다.

정보 수명은 다음 일곱 종류로 구분한다.

```text
A. SourceSemanticIdentity
   parser가 직접 만든 J entity/operand/binding 구조

B. StaticSemanticContract
   primitive/extension registry가 제공하는 target-independent 의미

C. ResolvedSemanticFact
   actual valence/argument facts와 결합한 뒤 Analyzer가 계산한 의미

D. OptimizationProof
   graph/use-def 분석으로 얻는 재계산 가능한 proof/candidate

E. LoweringCapability
   어떤 backend/kernel이 어떤 semantic form을 직접 처리할 수 있는가

F. RepresentationFact
   선택된 representation/layout에 대해 알려진 사실

G. PhysicalDecision
   concrete schedule/buffer/device realization
```

핵심 규칙:

> **A/B/C는 의미 보존에 관여하므로 optimization 전에 존재해야 하고, D는 invalidation 가능한 분석 결과이며, E는 IR 의미가 아니라 registry 정보이고, F/G는 route/schedule 이후에만 생긴다.**

구체적인 정보와 최초 삽입 시점은 다음과 같다.

| 정보 | jsource에서 보이는 근거 | RustJ 표현 | 최초 생성 단계 | 이후 활용 |
|---|---|---|---|---|
| operator identity, POS, operands, Hook/Fork 구조, source span | parser reduction / `fgh` cross-check | `FunctionEntity` | **Parser → Semantic IR** | derived analysis, pattern matching, diagnostics |
| NameRef와 binding/version identity | nameref/cache/fix machinery | `NameRef + BindingVersion/Guard` | **semantic binding** | specialization legality, deopt/guard, observable lookup order |
| name/self dependency | `VNONAME`, `VNOSELF`, `VXOPR` | `BindingDependencySummary` | **Function summary pass** | fix/specialization, hoisting, parallelization barrier |
| locale/path mutation 가능성 | `VNOLOCCHG` | `EffectSummary::LocaleMutation` | **PrimitiveSpec + Function summary** | order-edge 제거, CSE, speculation |
| try/catch/adverse/error behavior | `VTRY1/2`, adverse/runtime paths | `ErrorSemantics`, `Catchability` | **Semantic Analyzer** | observable order, fusion/speculation legality |
| supported valence | primitive table/action routine | `ValenceContract` | **PrimitiveSpec registry** | call legality, lowering key |
| innate monad/l/r rank | `t.c mr/lr/rr` | valence별 `RankSpec` | **PrimitiveSpec registry** | CellApply planning |
| explicit requested rank | `jtqq` saved rank noun/verb | `ConstructionFacts::RankBoundary` / requested `RankSpec` | **parser modifier semantic construction** | derived summary, nested CellApply |
| effective rank | `efr`, argument rank | `EffectiveCellRank` | **call analysis** | frame/cell split |
| frame/cell split | `rank1ex/rank2ex` | `CellApplicationPlan` | **call analysis** | Logical CellApply, schedule axes |
| prefix agreement/common frame | `ASSERTAGREE` | `AgreementFact` | **CellApplicationPlanner** | legality, repetition/index maps |
| residual repeated side | rank loop repeat state | `RepetitionFact` | **CellApplicationPlanner** | zero-stride/index-map lowering |
| atomic/cellwise semantics | `VISATOMIC1/2` | `AtomicitySemantics` | **PrimitiveSpec / derived summary** | access relation, absorption/fusion proof |
| result shape/rank | primitive semantics + cell assembly | `ValueFacts.shape/rank` | **Analyzer dataflow** | verifier, route, schedule |
| result dtype/type classes | type dispatch/promotion tables | `TypeSemantics + TypeFact` | **PrimitiveSpec + Analyzer** | kernel selection, conversion insertion |
| overflow retry/promotion | `EWOV*` retry paths | `OverflowSemantics` | **PrimitiveSpec / resolved op** | vector/reduction legality |
| comparison tolerance / fit | runtime cct, `!.` constructor | construction policy + `NumericSemantics` | **modifier semantic construction + call resolution** | comparison kernel, guard/runtime input |
| neutral/fill/empty behavior | `red0`, filler paths | `FillAndEmptySemantics` | **PrimitiveSpec + derived policy** | empty CellApply/Reduce |
| result-cell assembly policy | `result.h` | `AssemblyFact` | **Analyzer**, proof refined by optimizer | fixed output allocation vs dynamic assembly |
| uniform result-cell proof | homogeneous `result.h` fast path analog | `UniformCellResultProof` | **Analyzer/Logical Optimizer** | MapCells/GPU eligibility |
| semantic axis roles | primitive/derived operation meaning | `AxisSemantics` | **resolved-op analysis** | reduction/parallel/window mapping |
| logical access relation | atomic/reduce/reindex/gather semantics | `AccessRelation` | **resolved-op analysis** | fusion/locality/route planning |
| shape constraints, item count, nonempty/divisibility | shape/value facts | `ConstraintSet`, item-count fact | **Analyzer** | specialization, loop bounds, allocation |
| dense/sparse/boxed semantic representation | sparse/boxed branches | `Layout/RepresentationFact` | **value analysis** | route/lowering selection |
| `BoxAtop/Open/Raze` producer-consumer opportunity | `VF2BOXATOP/ATOPOPEN/WILLOPEN` | explicit ops + `MaterializationElisionCandidate` | semantic ops은 **Analyzer**, candidate는 **Logical Optimizer use-def pass** | box/open elimination, producer-consumer fusion |
| item-count propagation opportunity | `VF2USESITEMCOUNT*` | `CanConsumeItemCountFact` | **lowering capability registry + use-def analysis** | avoid materializing producer result |
| rank-loop absorption | `VIRS1/VIRS2` | `CanAbsorbCellApply` | **lowering registry** | native/external lowering choice |
| special fused executor | `VFUSEDOK2`, `jtmean`, composition special cases | `FusionCandidate + LoweringCapability` | candidate는 **Logical Optimizer**, capability는 **registry** | fused kernel selection |
| cell-loop fusion | `RANKATOP/RANKONLY` subsumption | `CellApplyFusionProof` | **Logical Optimizer** | remove nested loop boundary only if legal |
| actual alias/in-place reuse | in-place/pristine/zappable | `Liveness/AliasProof` + `MaterializationDecision` | **Physical Planner / Bufferization** | buffer reuse |
| actual offset/stride/alignment | virtual/physical array layout | `RepresentationFacts` | **representation/physical lowering** | vectorization, coalescing, view realization |
| address space/device placement | 없음/CPU implementation detail | physical placement facts | **Physical Planner** | transfer/synchronization |
| tile/vector/workgroup mapping | implementation fast path | `PhysicalSchedule` | **Schedule stage** | codegen |
| resource/cost estimate | implementation-specific | `ResourceEstimate/CostEstimate` | **schedule + target 이후** | candidate ranking |

###### Semantic node는 intrinsic summary를 직접 소유한다

Parser-produced `FunctionEntity`는 source identity/operand 관계뿐 아니라 **그 entity 자체에 안정적으로 귀속되는 semantic summary를 직접 소유**한다. derived entity는 이미 완성된 child entity의 `semantic_info`를 재귀적으로 참조해 bottom-up으로 summary를 만든다.

```text
FunctionEntity                      // parser-owned, immutable, shared by Arc/EntityRef
  id
  identity / POS / operands / span
  semantic_info:
    construction
    valence_contract
    rank_contract
    binding_dependency
    effect_summary
    error_summary
    atomicity
    latent_semantics
```

primitive의 상세 규칙은 `PrimitiveSpec`을 authoritative source로 두되 node의 `semantic_info`는 이후 재귀 분석에 필요한 안정적인 contract/summary를 제공한다. 큰 derived function에서도 child subtree 자체는 shared reference이므로 복사되지 않으며, parent에는 **parent 자신의 summary 한 벌만** 저장된다.

side table/cache는 다음에만 사용한다.

- 비용이 큰 재계산 가능 분석
- 특정 pass에서만 유효한 proof
- use-def 전체를 봐야 하는 정보
- target/device/cost dependent 정보

즉 semantic traversal에서 매번 전역 table을 조회해야만 child의 의미를 알 수 있는 구조는 기본 모델로 삼지 않는다.

###### call-time에만 알 수 있는 정보는 ResolvedCallFacts로 모은다

rank agreement, actual frame/cell, repeated side, output constraints는 function 자체의 속성이 아니라 **function + valence + actual argument facts**의 속성이다.

```text
ResolvedCallFacts
  valence
  effective_cell_ranks
  CellApplicationPlan
  AgreementFact
  RepetitionFact
  AxisSemantics
  AccessRelations
  NumericSemantics
  FillAndEmptySemantics
  AssemblyFact
  Effect/Error/Alias requirements
  result ValueFacts
```

Semantic Analyzer가 이를 만든 뒤 필요한 부분을 verified Logical IR payload/annotation으로 내린다.

###### Logical IR payload와 재계산 가능한 analysis fact를 구분한다

다음은 semantic correctness를 위해 Logical IR에 남아야 한다.

```text
must-preserve
  operation kind / operands / valence
  CellApply boundary and agreement/repetition semantics
  rank/fit/tolerance/fill policy
  dynamic assembly requirement
  observable effect/error/order requirements
  storage/state-resource identity
```

반면 다음은 side analysis로 두고 graph rewrite 후 재계산 가능하게 한다.

```text
recomputable
  UniformCellResultProof
  FusionCandidate::Mean
  CellApplyFusionProof
  MaterializationElisionCandidate
  invariance
  item-count propagation opportunity
  route eligibility
```

외부 IR adapter가 correctness를 위해 필요한 fact는 adapter precondition/witness로 승격하며, 단순 optimizer hint는 export semantics에 포함하지 않는다.

###### jsource의 memory-management flags는 Physical IR로 바로 복사하지 않는다

`WILLOPEN`, in-place, pristine, zappable은 jsource의 reference-counted runtime에서 매우 중요한 정보지만 RustJ에서는 대부분 **use graph + liveness + ownership/alias + representation**으로 다시 유도하는 편이 낫다.

예를 들어:

```text
producer -> Box -> consumer Open
```

이 semantic graph가 있으면 Logical Optimizer가:

```text
MaterializationElisionCandidate
  producer result does not escape
  box/open semantics cancel under proof
```

를 만들고, 실제 buffer reuse 여부는 Physical Planner가 결정한다.

즉 jsource의 `WILLOPEN` bit를 parser/LogicalOp에 복사하지 않는다.

###### verifier는 fact의 출처와 가정을 구분한다

향후 analysis fact에는 필요하면 다음 provenance를 둔다.

```text
FactProvenance
  SourceSemantic
  PrimitiveSpec
  DerivedSummary
  InferredFrom(ValueId...)
  ProvenBy(pass)
  GuardedAssumption(GuardId)
```

특히 dynamic shape/rank/name binding을 compile-time fact로 사용했다면 `GuardedAssumption` 또는 runtime semantic dependency가 있어야 한다. 근거 없는 `Known` fact는 verifier가 허용하지 않는다.

###### 이 설계를 `(+/ % #) y`에 적용하면

`y =: i. 2 3`에서:

```text
Parser/Semantic IR
  Fork(
    +/ : derived Verb,
    %  : Verb,
    #  : Verb
  )

where the first fork operand is one shared derived-function entity:

  +/ : Verb
    head = PrimitiveAdverb(Insert)   // /
    operands = [ + : Verb ]

PrimitiveSpec
  + dyad rank = 0 0
  % dyad rank = 0 0
  # monad rank = Infinite

Function summary
  fork structure preserved
  pure/no-locale-change subset proven where applicable

Call analysis
  +/ y:
    Reduce(Add)
    reduction axis = leading axis of current cell
    result shape = [3]

  # y:
    Tally
    result shape = []

  (3 5 7) % 2:
    effective ranks = 0 / 0
    left frame = [3]
    right frame = []
    repeat right cell
    Logical CellApply2
    output shape = [3]

Logical Optimizer
  generic graph remains correctness form
  recognize FusionCandidate::Mean
  prove UniformCellResult if facts suffice

Route/lowering
  generic Reduce + Tally + CellApply(Divide)
  or verified fused mean kernel

Schedule/Physical
  choose SIMD/thread/GPU mapping
  choose buffers/views/reuse
```

따라서 `(+/ % #)` 예제에서 **mean이라는 정보는 Parser가 넣는 정보가 아니다.** Parser는 Fork를 보존하고, Analyzer는 rank/cell/access semantics를 넣고, Logical Optimizer가 mean pattern을 발견하며, target lowering이 실제 fused mean implementation을 선택한다.

###### 구현 순서에 반영

IL1/IL2 구현과 함께 다음 infrastructure를 추가한다.

1. `PrimitiveSpec`에 현재 지원 primitive의 valence별 innate rank/atomicity/effect/error/fill 핵심 contract를 넣는다.
2. `FunctionEntity.semantic_info`를 추가하여 shared DAG의 각 node가 자신의 name/self/effect/error/rank/atomicity/latent semantic summary를 직접 소유하게 한다. construction 시 child summary를 bottom-up으로 조합하고, pass-local 재계산 proof만 별도 cache/table로 둔다.
3. current `Node.facts + rank_plan + access`를 점진적으로 `ResolvedCallFacts / LogicalFacts` 구조로 모으되 한 번에 거대한 enum/struct로 바꾸지 않는다.
4. `CellApplicationPlanner`가 effective rank, frame/cell, agreement, repetition, empty policy를 생산하게 한다.
5. `LogicalPlan::verify`가 must-preserve fact의 정합성을 검증한다.
6. use-def 기반 `OptimizationFacts` pass를 추가하여 uniform-result, mean/fusion, CellApply fusion, materialization-elision candidate를 만든다.
7. `LoweringRegistry`에는 IRS/FUSEDOK에 대응하는 capability만 등록하고 semantic fact와 섞지 않는다.
8. 실제 stride/alignment/in-place/buffer reuse는 Logical IR 구현보다 뒤의 representation/physical 단계에서만 추가한다.


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
  ResourceEstimate ~= compiled resource report

performance validation
  CostEstimate ~= measured backend behavior
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

B. CompilationTarget / resolved TargetProfile
   BackendFamily + ArchitectureTarget + DeviceProfile + RuntimeProfile
   "이 target은 무엇을 제공하고 무엇을 제한하는가?"

C. Physical Schedule / Realization
   "이 계산을 이 target에 어떻게 매핑할 것인가?"

D. Resource / Cost Estimate
   "그 매핑의 자원 사용량과 예상 비용은 얼마인가?"
```

이 네 층을 하나의 `PrimitiveSpec` 또는 하나의 거대한 IR node에 섞지 않는다.

예를 들면 `leading axis`, `reduction axis`, `access relation`, `iteration dependency`는 A에 속한다. architecture instruction/subgroup/register-allocation rule과 exact device의 register/shared/LDS/cache capacity는 B에 속하되, ArchitectureTarget과 DeviceProfile 중 어디의 사실인지 구분한다. `tile=64x128`, `num_warps=4`, `vector_width=8`, memory-space 선택은 C이며, `registers/thread=72`, occupancy, HBM bytes, spill risk는 D다.

### 4.15 Logical Array IR이 가져야 하는 hardware-relevant contract

Logical Array IR은 hardware-independent여야 하지만 **hardware-blind여서는 안 된다.** CUDA의 `threadIdx.x`를 몰라도 planner가 어떤 logical axis를 thread/lane/vector에 배치할 수 있는지는 알아야 한다.

최소 모델:

```text
LogicalOp
├─ IterationDomain
├─ AxisSemantics
├─ AccessRelations
├─ NumericSemantics
├─ DependencyRequirements
├─ Effect / Alias
└─ Storage / LifetimeRequirements
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
  bounds / masking requirement
```

conv2d의 개념적 관계:

```text
Y[oc, oh, ow]
X[ic, oh*stride_h + kh - pad_h, ow*stride_w + kw - pad_w]
W[oc, ic, kh, kw]
```

이 정보로 planner는 tile 내부 reuse, reduction/output parallelism, static reindex, gather/scatter 구조를 판단할 수 있다. 실제 contiguous/vectorized access, memory transaction, alignment legality는 이후의 representation facts와 target mapping을 함께 봐야 한다. 따라서 향후 `leading_axis`보다 더 일반적인 핵심 표현은 **axis-role + semantic access relation**이다.

#### 4.15.4 NumericSemantics

하드웨어 mapping을 결정하려면 dtype만으로 부족하다.

```text
NumericSemantics
  input dtypes
  result dtype
  accumulator requirement
  widening / narrowing rules
  overflow retry / promotion behavior
  comparison tolerance mode / context
  fit (!.) numeric policy when applicable
  reduction algorithm / accuracy mode when specified
  reassociation allowed?
  FMA contraction allowed?
  reduction order/rounding constraints when specified
  NaN / signed-zero constraints
```

CPU vector reduction이나 GPU tree reduction을 무조건 금지하지 않는다. current jsource 자체도 일반 floating reduction에 SIMD/여러 accumulator를 사용할 수 있다. 대신 각 primitive/derived verb가 요구하는 **numeric accuracy/order contract**를 표현하고, 그 contract가 허용하는 범위에서만 reassociation/vector/tree reduction을 선택한다.

#### 4.15.4a comparison tolerance와 `!.`는 semantic input이다

J의 비교는 단순 IEEE bitwise comparison으로 고정되지 않는다. current jsource는 runtime comparison tolerance(`jt->cct`)를 사용하고, `u!.ct` derived verb는 자체 tolerance를 보존할 수 있다. `!.`는 일부 verb에서는 fill을 바꾸는 의미도 가진다.

따라서 compiled/GPU lowering은:

```text
ComparisonSemantics
  Exact
  DefaultTolerance(context/versioned runtime state)
  ExplicitTolerance(value)

FitSemantics
  numeric_tolerance_override
  fill_override
  primitive-specific fit behavior
```

를 잃지 않는다.

tolerance가 runtime mutable state에 의존하면 다음 중 하나가 필요하다.

- compile-time captured value + binding/runtime guard
- explicit semantic input
- runtime semantic lowering

GPU kernel 안에서 임의의 exact comparison으로 바꾸면 안 된다.

또한 current jsource는 일반 `+/`와 `+/!.0`를 같은 reduction algorithm으로 취급하지 않는다. `+/!.0`에는 compensated summation 경로가 있으므로, `!.`를 단순 optimizer hint로 버리면 안 된다.

```text
ReductionNumericPolicy
  DefaultJReduction
  Compensated
  ExactOrExtended
  PrimitiveSpecific(...)
```

처럼 derived verb의 numeric policy가 lowering까지 전달되어야 한다.

#### 4.15.4b overflow/promotion은 primitive contract의 retry semantics다

current jsource의 atomic arithmetic은 retryable overflow를 별도 error class로 반환하고, 경우에 따라 wider result path로 전체 연산을 retry하거나 이미 계산한 결과를 일관된 widened representation으로 repair한다.

따라서 “integer op이면 원소마다 overflow한 lane만 float로 바꾼다” 같은 realization은 허용하지 않는다.

```text
OverflowSemantics
  primitive/type combination
  retryable?
  promoted result type/path
  coherent-result requirement
  reduction/prefix-specific behavior
```

를 contract로 둔다.

“전체 배열 overflow promotion”이라는 기존 문구는 모든 primitive에 동일한 promotion 규칙이 있다는 뜻이 아니다. **해당 J primitive/type 조합이 정의하는 retry/promotion을 결과 array 전체의 일관된 J type semantics로 보존한다**는 뜻이다.


#### 4.15.5 Logical dependency requirement

logical op은 CUDA barrier 자체를 갖지 않는다. 대신 어떤 범위의 dependency가 필요한지 표현한다.

```text
DependencyRequirement
  Independent
  Reduction(axis_set)
  OrderedScan(axis_set)
  WindowDependency(axis_set)
  ConflictingUpdate(resource_or_value, combine_semantics)
  CrossValueOrdering(effect_or_resource)
  Collective(logical_participants)
```

여기서 `Subgroup`, `Warp`, `Workgroup` 같은 execution scope는 Logical IR에 넣지 않는다. 이들은 Physical Schedule이 logical participants를 target execution hierarchy에 매핑한 뒤 생긴다.

Physical Planner 또는 외부 compiler가 target의 barrier/shuffle/atomic/multi-stage reduction/collective capability를 보고 구체적으로 실현한다. `ConflictingUpdate`가 있다고 해서 atomic instruction 사용을 미리 결정하지 않는다.

#### 4.15.6 Logical invariance와 derived target uniformity를 분리한다

`uniform/divergent`는 보통 SPMD execution mapping이 정해진 뒤 의미가 생긴다. 따라서 target-independent Logical IR의 기본 fact 이름은 `InvarianceFact`로 둔다.

```text
InvarianceFact
  InvariantOver(logical_axis_set)
  KnownVariantAlong(logical_axis_set)
  Unknown
```

적용 대상:

- branch condition
- indirect index
- logical address/index expression
- mask
- collective operand

Physical Schedule이 logical axes를 SIMD lane/subgroup/thread에 매핑한 뒤:

```text
InvarianceFact + AxisMapping
  → TargetUniformity / DivergenceFact
```

를 파생한다.

즉 GPU subgroup uniformity를 Logical IR의 고정 의미로 저장하지 않는다. 이는 CPU vector lane invariance에도 같은 logical fact를 재사용할 수 있게 한다.

#### 4.15.7 Symbolic shape / divisibility constraints

dynamic extent는 단순 `Unknown`으로 버리지 않고 가능한 제약을 보존한다.

```text
ConstraintSet
  Equal(a, b)
  UpperBound(x, n)
  LowerBound(x, n)
  MultipleOf(x, n)
  DivisibleBy(x, n)
  PowerOfTwo(x)
  NonZero(x)
```

이 정보는 tile size, unrolling, matrix/tensor shape legality와 runtime specialization을 결정할 때 사용한다. 주소 alignment나 physical contiguity는 logical shape constraint와 분리한다.

제약이 compile-time에 증명되지 않더라도 runtime guard로 specialization할 수 있다.

```text
if N % 16 == 0
  → vectorized/tensor path
else
  → conservative path
```

#### 4.15.8 Representation facts는 Logical semantics와 분리한다

stride, byte alignment, concrete contiguity, address space는 J logical value의 의미가 아니다.

```text
RepresentationFacts
  storage_extents_if_known
  strides
  base_offset
  byte_alignment
  contiguous_dimensions
  address_space / memory_space
  external_abi_layout
```

`RepresentationFacts`는 canonical Logical IR의 semantic identity가 아니라 downstream planning/adapter가 Logical value에 대해 알고 있는 **representation-side analysis state**다.

이 facts는 다음 출처에서만 생긴다.

- 함수/FFI/external buffer ABI contract
- 이미 존재하는 PhysicalArray/view
- bufferization/physical planner의 결정
- runtime guard로 검증한 representation assumption

따라서 `AccessRelation`이나 semantic `ConstraintSet` 안에 stride/alignment를 섞지 않는다.

```text
LogicalOp + AccessRelation
          +
RepresentationFacts
          +
TargetProfile
       → physical access legality/cost
```

representation assumption에 의존하는 fast path는 witness/guard 또는 adapter precondition으로 명시한다. MLIR에서도 byte alignment는 tensor semantic이 아니라 memref/alloc/load/store 수준의 representation property로 다뤄진다.


#### 4.15.9 Semantic mask와 schedule predication을 분리한다

Logical IR에는 **연산 의미 자체가 조건부 access/value를 요구할 때만** semantic mask를 둔다.

```text
SemanticMaskSemantics
  predicate source
  affected logical accesses/results
  false-branch value semantics
  bounds relation
  side-effect suppression rule
```

예:

- data-dependent select/filter/gather safety
- window/padding 의미 때문에 논리적으로 out-of-domain access를 구분해야 하는 연산
- source semantics에 포함된 conditional effect

반대로 vector tail, partial tile, workgroup 경계 때문에 생기는 mask는 Logical IR의 의미가 아니다.

```text
Logical shape/access facts
        ↓
Schedule / vectorization / tiling
        ↓
PredicationPlan
  tail mask
  masked load/store
  boundary predicate
```

따라서 CPU AVX mask나 GPU lane predicate는 downstream schedule/backend 결정이다. 같은 Logical IR이 target에 따라 masked instruction, scalar remainder loop, padded tile 중 다른 realization을 선택할 수 있다.

#### 4.15.10 SSA, region/block, control flow

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

#### 4.15.11 Constraint witness / runtime guard

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

#### 4.15.12 effect resource와 ordering token

SSA data dependency만으로는 I/O, mutable state, explicit storage update, runtime call의 관찰 가능한 순서를 모두 표현할 수 없다.

Logical IR은 effect를 두 수준으로 표현한다.

```text
EffectSummary
  semantic_or_external_resource
  Read | Write | CreateResource | DestroyResource | IO | External | Unknown
  stage/order constraints

EffectToken
  explicit ordering edge when data dependency alone is insufficient
```

모든 pure op에 token을 붙이지 않는다. observable side effect나 external call처럼 순서가 의미에 포함되는 경우에만 explicit token/effect edge를 사용한다. 여기의 Create/Destroy는 **언어 또는 external-resource semantics에 실제로 존재하는 resource**에 한정하며, downstream buffer allocation/free를 뜻하지 않는다.

StableHLO의 side-effecting op token과 MLIR MemoryEffect/Speculation interface를 참고하되, J의 error ordering까지 포함할 수 있도록 `SpeculationSemantics`를 별도로 둔다.

#### 4.15.13 speculation / may-error semantics

J에서는 domain/rank/length/overflow 등의 오류 발생 순서도 관찰 가능할 수 있다. 따라서 “memory effect가 없다”와 “마음대로 speculative execution 가능”은 다르다.

```text
SpeculationSemantics
  AlwaysSafe
  SafeIf(constraints)
  MayRaiseObservableError(error_set)
  MayThrow
  MayNotTerminate
  HasNonLocalControlEffect
```

optimizer와 external adapter는 이 contract를 보고 hoist, duplicate, eliminate, reassociate 가능성을 판단한다.

#### 4.15.14 J error precedence와 internal semantic probes를 구분한다

backend 병렬 실행이 “먼저 발견한 lane의 오류”를 임의로 사용자 오류로 선택하면 안 된다.

current jsource의 atomic dyad 경로는 예를 들어 agreement/rank-shape 검사를 type/domain conversion보다 먼저 수행하며, source-level error class의 우선순위를 의도적으로 보존한다.

```text
ErrorSemantics
  possible_errors
  precedence / validation order where observable
  retryable_internal_conditions
  suppressed semantic-probe errors
  externally observable error
```

를 primitive/derived-op contract가 표현할 수 있어야 한다.

중요한 구분:

- J가 사용자에게 보고하는 domain/rank/length/value/... error
- overflow처럼 retry를 유도하는 internal condition
- empty fill-cell/prototype 평가에서 J가 의도적으로 suppress하는 computational error
- out-of-memory 같은 exigent error

따라서 “오류 순서 보존”은 **jsource가 정의한 observable error precedence와 suppression/retry semantics를 보존한다**는 뜻이다. 내부적으로 발생한 모든 임시 오류를 그대로 노출한다는 뜻이 아니다.

#### 4.15.15 J error는 exceptional control flow가 될 수 있다

explicit definition 안의 J error를 항상 process-level failure 또는 compile diagnostic으로 취급하지 않는다.

current jsource(`cx.c`, `wc.c`)는 `try.`, `catch.`, `catchd.`, `catcht.`, `throw.`를 control-word graph로 만들고, 실행 중 발생한 error/throw에 따라 handler target으로 이동한다.

따라서:

```text
JSemanticOutcome
  Normal(JEntity)
  Raise(JError)
  Throw
```

와 같은 개념을 CFG semantics가 표현할 수 있어야 한다.

장기 Logical/Control IR에서는:

```text
Op
  normal_successor
  exceptional_successor(s) when inside an active handler region

TryRegion
  body
  handlers:
    ordinary-error handler
    debug/error-class-specific handler as required by J semantics
    throw handler
```

형태 또는 동등한 runtime lowering을 허용한다.

중요한 원칙:

- `domain/rank/length/value/...` error는 handler가 없으면 외부로 전파되지만, active `try.` 안에서는 J control flow의 일부가 될 수 있다.
- `throw.`는 일반 return이 아니라 explicit exceptional control effect다.
- optimizer는 potentially-raising op를 catch boundary 밖으로 hoist하거나 handler를 건너 duplicate/eliminate하지 않는다. 그러려면 `SpeculationSemantics`/error proof가 필요하다.
- GPU/external region 안에서 생긴 J-visible error도 surrounding J handler semantics를 보존할 수 있어야 한다. adapter/backend가 typed error outcome을 되돌릴 수 없다면 해당 region offload를 거부하거나 error-free proof가 필요하다.
- retryable internal overflow/prototype probe error는 user-visible exceptional edge와 구분한다.

A3-v0의 pure single-block subset에는 full exceptional CFG를 요구하지 않는다. `try/catch/throw` lowering은 A3-v1의 multi-block CFG와 함께 구현하되, v0 operation contract는 `MayRaise`/error set을 잃지 않아야 한다.

explicit control word만 error handler인 것은 아니다. current jsource의 `u :: v` adverse conjunction은 derived verb 내부에 handler semantics를 만든다.

```text
AdverseDerived(u, v)
  run u
    normal -> return result
    ordinary catchable error -> clear/catch according to J semantics, run/use v
    throw / exit class -> propagate
```

따라서 `ErrorHandlerSemantics`는 function-level `try.`와 expression/derived-verb-level adverse 모두 표현할 수 있어야 한다. `v`가 noun인 경우 fallback value 자체가 결과가 될 수 있다는 점도 보존한다.





### 4.16 CompilationTarget은 backend / architecture / device를 분리한다

기존의 하나짜리 `TargetProfile`은 역할이 너무 넓다. 같은 CUDA/ROCm backend에서도 architecture가 다르면 instruction, register organization, subgroup/wave behavior, scratchpad 기능 등이 달라지고, 같은 architecture를 쓰는 device끼리도 compute-unit 수, memory/cache capacity 등이 다를 수 있다.

현행 모델은 다음을 구분한다.

```text
CompilationTarget
├─ BackendFamily
├─ ArchitectureTarget
├─ DeviceProfile
├─ RuntimeProfile
└─ resolved TargetProfile view
```

의미:

```text
BackendFamily
  어떤 compiler/runtime ecosystem인가?
  CUDA / ROCm / LLVM-CPU / SPIR-V / Metal / ...

ArchitectureTarget
  어떤 ISA/microarchitecture capability에 맞춰 code를 만들 수 있는가?
  architecture family + concrete compiler/ISA target + feature set

DeviceProfile
  그 architecture를 구현한 실제 device가 얼마만큼의 자원을 제공하는가?

RuntimeProfile
  현재 driver/runtime/toolchain에서 실제 사용할 수 있는 기능은 무엇인가?

TargetProfile
  위 profile fragments와 query providers를 resolution한
  compile invocation용 effective view
```

`CostProfile`은 여전히 별도다. 동일 device라도 runtime/clock/workload/calibration에 따라 달라지는 empirical 성능값이기 때문이다.

#### 4.16.1 BackendFamily

backend family는 source primitive identity가 아니라 lowering ecosystem이다.

예:

```text
generic
cpu
gpu
cuda
rocm
spirv
metal
llvm_cpu
```

BackendFamily가 제공할 수 있는 것:

- 공통 lowering interfaces
- ABI/data-layout family
- memory/execution model의 공통 query
- generic kernel/library strategy
- architecture target naming/selection 규칙

`CUDA`, `ROCm` 같은 이름만으로 concrete instruction legality를 전부 판단하지 않는다.

#### 4.16.2 ArchitectureTarget

architecture target은 **code-generation legality와 architecture-specific capability**의 핵심 단위다.

```text
ArchitectureTarget
  vendor
  architecture_family
  compiler_or_isa_target
  feature_set
  execution_hierarchy_rules
  register_model
  memory/scratchpad model
  instruction capabilities
  synchronization/memory-ordering capabilities
  allocation rules
```

예시적인 naming:

```text
nvidia family / sm target
amd family / gfx target
intel GPU family / target
cpu ISA + microarchitecture feature set
```

중요한 원칙:

- architecture version 숫자의 대소만으로 feature inheritance를 가정하지 않는다.
- capability는 명시적인 feature/query로 판정한다.
- family-level 공통 정보와 concrete architecture override를 둘 수 있다.
- architecture-specific instruction lowering은 이 수준에서 등록할 수 있다.

#### 4.16.3 DeviceProfile

DeviceProfile은 architecture identity가 아니라 **concrete capacity/topology**를 제공한다.

```text
DeviceProfile
  architecture_target
  compute-unit / SM / core count
  register capacities
  scratchpad/shared/LDS capacities
  cache capacities/topology
  memory capacity
  supported configurable resource modes
  link/topology facts
  launch/concurrency hard limits that are device-specific
```

같은 ArchitectureTarget을 공유하는 device라도 DeviceProfile은 다를 수 있다.

구분:

```text
ArchitectureTarget
  "어떤 code/instruction이 legal한가?"

DeviceProfile
  "이 legal code를 이 장치에서 어떻게 schedule하는 것이 가능한가?"
```

#### 4.16.4 RuntimeProfile

일부 capability는 silicon만으로 결정되지 않고 driver/runtime/compiler stack의 지원에도 의존한다.

```text
RuntimeProfile
  backend/runtime version
  driver capability
  compiler/toolchain feature availability
  library availability/version
  runtime launch/interop capability
```

ArchitectureTarget이 지원하는 instruction/feature라도 현재 toolchain/runtime route가 노출하지 않으면 해당 lowering candidate는 사용할 수 없다.

#### 4.16.5 resolved TargetProfile

기존 문서의 `TargetProfile`이라는 이름은 삭제하지 않고 **resolved effective target view**라는 뜻으로 좁힌다.

```text
TargetProfile =
  resolve(
    BackendFamily profile,
    ArchitectureTarget profile,
    DeviceProfile,
    RuntimeProfile,
    target query providers
  )
```

TargetProfile은 logical IR에 복사하지 않는다. compile invocation, route partition, schedule/physical planner, external adapter가 질의한다.

### 4.16.5 Compilation 시작 시 active target locale을 확정한다

각 compile invocation은 parser/J Semantic IR의 의미와 독립적으로 **active compiler target context**를 가진다. 단일-target MVP에서는 compile 시작 시 하나의 `CompilationTargetLocale`을 확정한다.

```text
CompilationSession
  semantic_context        // user J locale/binding semantics
  target_context:
    active_target_locale
    CompilationTarget
    DeviceProfile?
    RuntimeProfile?
```

`active_target_locale`은 user J locale과 절대 섞지 않는다. parser와 J Semantic IR은 target을 몰라도 동일한 의미를 만들어야 한다.

예:

```text
active target:
  compiler.device.h100_0

resolution path:
  compiler.device.h100_0
    -> compiler.arch.nvidia.sm90
    -> compiler.family.nvidia
    -> compiler.backend.cuda
    -> compiler.gpu
    -> compiler.generic
```

모든 hardware-lowerable semantic operation은 source가 built-in인지 extension인지와 무관하게 **같은 active target locale/path**에서 capability/lowering을 조회한다.

```text
+/          -> ResolvedOp::Reduce(Add) ----┐
conv2d ext  -> ResolvedOp::Conv2d ---------+-> same TargetLocale lookup
attention   -> ResolvedOp::Attention ------┘
```

따라서 primitive 자체가 architecture 정보를 갖는 것이 아니라:

```text
Resolved Semantic Op
  + active TargetLocale
  + resolved target/device facts
       ↓
Lowering candidates
```

가 된다.

중요한 규칙:

- target locale 선택은 **J parse/semantic validity에 영향을 주지 않는다**.
- target locale은 lowering/capability discovery의 시작점이다.
- built-in J primitive와 extension-derived op는 동일한 lookup protocol을 사용한다.
- locale lookup은 후보를 찾을 뿐 최종 kernel을 고르지 않는다. legality/resource/cost 분석이 후보 중 realization을 선택한다.
- device-specific locale에 binding이 없으면 explicit parent path를 따라 architecture/family/backend/class/generic으로 fallback한다.
- 숫자 architecture 버전으로 암묵적 상속을 추론하지 않는다.
- 단일-target MVP 이후 heterogeneous execution이 필요해지면 `CompilationSession`이 여러 `TargetContext`를 보유하고 RoutePartition이 region별 context를 선택하도록 확장한다. 이 경우에도 각 region의 built-in/extension은 선택된 동일 target locale chain을 사용한다.

### 4.16.5.1 AOT와 JIT는 동일한 target selection contract를 사용한다

AOT build와 향후 JIT compilation은 서로 다른 target-selection 시스템을 만들지 않는다. 둘 다 동일한 입력 contract를 사용한다.

```text
CompileOptions / JitOptions
  target_selector
  device_selector?
  target_profile?
  runtime_policy?
        ↓
TargetSelector::resolve(...)
        ↓
CompilationTargetLocale
TargetContext
        ↓
LoweringRegistry lookup
```

차이는 **resolution 시점과 available evidence**뿐이다.

- **AOT**: 명시적 CLI/config/default host target을 기준으로 target context를 만든다.
- **JIT**: 동일한 option model을 사용하되 실제 runtime device, driver/runtime capability, currently available backend를 추가 evidence로 사용할 수 있다.
- 사용자가 `--target`/동등 옵션을 명시하면 JIT도 그 constraint를 존중한다.
- `auto`/unspecified일 때만 JIT가 runtime discovery를 이용해 concrete target/device를 선택한다.
- JIT가 선택한 concrete target도 결국 동일한 compiler target locale/path로 normalize된다.
- built-in/extension 구분 없이 모든 operation은 해당 active target locale을 통해 lowering capability를 조회한다.

따라서:

```text
AOT:
  rustj build --target sm90
       ↓
  TargetSelector
       ↓
  compiler.arch.nvidia.sm90

JIT:
  jit(options: target=auto)
       ↓
  runtime discovery = H100
       ↓
  TargetSelector
       ↓
  compiler.device.<H100>
       -> compiler.arch.nvidia.sm90
       -> ...
```

JIT 전용 primitive registry나 JIT 전용 hardware metadata 체계를 만들지 않는다. cache key에는 semantic/IR version과 함께 resolved target identity 및 lowering-relevant runtime capability version을 포함할 수 있다.

### 4.16.6 Target locale chain: J locale 방식을 compiler lookup에 재사용한다

backend/architecture/device별 lowering과 capability override는 J의 **locale/path resolution 아이디어**를 compiler namespace에 재사용하면 단순하게 구현할 수 있다.

중요하게, **사용자 J locale과 compiler target locale은 namespace를 분리**한다. 의미와 lookup 방식은 재사용하지만 서로 binding을 섞지 않는다.

개념:

```text
User J Namespace
  base
  myapp
  ...

Compiler Target Namespace
  compiler.generic
  compiler.gpu
  compiler.backend.cuda
  compiler.family.nvidia.<family>
  compiler.arch.nvidia.<arch-target>
  compiler.device.<device-id>
```

target이 정해지면 explicit locale path를 구성한다.

예:

```text
TargetLocaleChain
  device-specific
    → concrete architecture
    → architecture family
    → backend family
    → device class (gpu/cpu)
    → generic
```

예시적인 형태:

```text
compiler.device.<exact-device>
  → compiler.arch.nvidia.<sm-target>
  → compiler.family.nvidia.<family>
  → compiler.backend.cuda
  → compiler.gpu
  → compiler.generic
```

또는:

```text
compiler.device.<exact-device>
  → compiler.arch.amd.<gfx-target>
  → compiler.family.amd.<family>
  → compiler.backend.rocm
  → compiler.gpu
  → compiler.generic
```

이 path는 **숫자 버전의 자동 상속이 아니라 명시적으로 구성한 resolution path**다.

#### 4.16.7 Lowering lookup도 locale resolution을 사용한다

각 target locale은 primitive/op에 대한 lowering binding을 가질 수 있다.

```text
compiler.generic
  ElementwiseAdd(Dyad) -> GenericAddLowering

compiler.gpu
  ElementwiseAdd(Dyad) -> GenericGpuElementwiseAdd

compiler.backend.cuda
  ElementwiseAdd(Dyad) -> CudaElementwiseAdd

compiler.arch.nvidia.<arch>
  Matmul(...) -> ArchitectureSpecificMatmul
```

lookup:

```text
resolve_lowering(
    key = ResolvedSemanticOp {
      source = PrimitiveId::Plus,
      valence = Dyad,
      semantic_op = ElementwiseAdd,
      derived_policies = ...
    },
    locale_chain = target.locales
)
```

검색 순서:

```text
most-specific device
  ↓
architecture
  ↓
family
  ↓
backend
  ↓
cpu/gpu class
  ↓
generic
```

가장 구체적인 legal binding이 우선하지만, 후보는 하나로 즉시 확정하지 않아도 된다. locale lookup은 **candidate discovery**를 담당하고, 실제 선택은 legality/resource/cost 평가가 한다.

```text
locale lookup
  → lowering candidates
  → capability/precondition verification
  → ResourceEstimate
  → optional CostEstimate
  → choose schedule/realization
```

device-specific lowering override는 허용하지만 남발하지 않는다. exact device locale은 주로 capacities/cost/profile override에 사용하고, code-generation 차이가 architecture에서 설명 가능하면 architecture locale에 둔다.

#### 4.16.8 Capability lookup도 locale fragment를 합성한다

lowering뿐 아니라 target information도 같은 mechanism을 사용할 수 있다.

```text
compiler.generic
  common ABI/query defaults

compiler.gpu
  generic subgroup/workgroup concepts

compiler.backend.cuda
  CUDA execution/memory model

compiler.arch...
  architecture instruction/register/allocation rules

compiler.device...
  exact capacities/topology
```

단순 key override로 표현하기 어려운 규칙은 `TargetQueries` provider binding을 locale에 등록한다.

```text
TargetFacts
  stable values / capacities / identities

TargetQueries
  legal_vector_access(...)
  memory_transaction_structure(...)
  occupancy_bound(...)
  matrix_instruction_candidates(...)
  atomic_support(...)
  async_copy_candidates(...)
  preferred_layout(...)
```

query resolution 역시 most-specific locale provider가 override하거나, 명시적인 composition rule을 통해 상위 provider를 호출할 수 있다.

#### 4.16.9 ExecutionHierarchy

GPU 예는 `Device → ComputeUnit/SM → Workgroup/CTA → Subgroup/Warp/Wave → Lane/Thread`, CPU 예는 `Machine/NUMA → Core → Hardware thread → SIMD lanes`로 본다.

필요 후보는 compute-unit count, supported subgroup widths, resident subgroup/workgroup limits, threads/lanes per workgroup, workgroup/grid dimension limits, CPU SMT/NUMA facts다.

이 정보의 일부는 architecture rule이고 일부는 exact device capacity다. 어느 profile fragment에 속하는지 구분해 저장한다.

#### 4.16.10 RegisterResources

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

register class/width/allocation rule은 주로 ArchitectureTarget에, concrete capacity/limit override는 DeviceProfile에 둘 수 있다.

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

#### 4.16.11 MemoryHierarchy

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

memory-space semantics/transaction/bank rules은 architecture/backend profile에, concrete cache/scratchpad/HBM capacity는 device profile에 두는 식으로 분리한다.

일부 target에서는 cache와 scratchpad/shared memory가 같은 physical resource를 partition한다.

```text
ResourceCoupling
  participants
  valid_configuration_modes
  capacity_relation
  selection_scope
```

#### 4.16.12 ComputeCapabilities

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
  execution scope
special instructions
```

`tensor_core=true` 같은 boolean 하나보다 operation signature/capability query가 낫다.

#### 4.16.13 Synchronization / memory ordering / data movement

architecture/backend profile은 다음 capability를 제공할 수 있다.

```text
MemoryOrderingCapability
  supported_scopes
  supported_orderings
  atomic_ops_by_dtype
  fence_capabilities
  coherent_spaces

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

구체적인 barrier/event/atomic/copy 선택은 schedule/physical lowering에서 한다.

#### 4.16.14 LaunchAndSchedulingLimits

max threads/workgroup, resident workgroups/compute-unit, subgroups/workgroup, grid limits, dynamic scratchpad limits, cluster/cooperative launch capability 등을 표현한다.

rule 자체가 architecture에 속하는지, exact numeric capacity가 device에 속하는지 분리한다.

#### 4.16.15 TransferAndTopology

device memory capacity, host-device links, peer-to-peer connectivity, NUMA relation, collective capability, concurrent copy/compute capability를 표현한다.

single-device MVP에는 필수가 아니지만 Placement/Sharding 확장을 막지 않도록 schema 위치를 예약한다.

#### 4.16.16 ABI / DataLayout

endianness, type size/alignment, pointer/address-space width, vector alignment 같은 정보다. J logical dtype 의미가 아니라 backend representation constraint다.

architecture/backend family의 ABI rule과 runtime/external ABI requirement를 분리할 수 있어야 한다.

### 4.17 CostProfile: hard limit와 측정 성능을 분리한다

effective memory bandwidth, instruction throughput, cache hit rate, launch overhead, interconnect bandwidth/latency, library-call overhead는 driver, clock, power state, workload, runtime version에 따라 바뀔 수 있다.

따라서 `CompilationTarget`의 hard facts와 별도로 optional `CostProfile`을 둔다.

```text
CostProfile
  backend_family
  architecture_target
  device_profile_id
  target_profile_resolution_id
  runtime/driver/compiler version
  measurement provenance
  bandwidth estimates
  latency estimates
  instruction throughput estimates
  launch overhead
  transfer costs
  calibrated library/kernel costs
  critical-path calibration/latency model
  kernel-launch aggregation/fusion benefit model
```

legality는 `TargetProfile` hard facts로 판단하고, 후보 ranking은 `TargetProfile + CostProfile`로 한다. CostProfile이 없어도 conservative heuristic으로 합법적인 plan을 만들 수 있어야 한다.

### 4.18 RustJ-native Physical Plan이 기록하는 hardware mapping

TargetProfile의 정보를 Logical IR에 복사하지 않는다. RustJ-native route에서는 Physical Planner가 선택한 결과를 Physical Plan에 기록한다. External route에서는 동등한 mapping을 해당 compiler의 lower-level IR/config가 소유할 수 있다.

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

### 4.19 ResourceEstimate와 CostEstimate를 분리한다

`TargetProfile`의 hard facts와 `CostProfile`의 경험적 성능치를 분리했으므로, plan 평가 결과도 두 종류로 나눈다.

#### ResourceEstimate

schedule과 target hard facts로부터 계산 가능한 자원/구조 추정이다.

```text
ResourceEstimate
  register usage by class
  shared/LDS/scratchpad bytes
  spill/local-memory risk
  resident workgroups/subgroups bound
  theoretical occupancy/concurrency bound
  global-memory bytes
  cache/scratchpad traffic amount
  transaction/coalescing count estimate
  bank-conflict structure
  arithmetic intensity
  instruction/work count
  critical-path depth / dependency-chain estimate
  synchronization count
  launch count
  transfer bytes
  temporary bytes / avoided materialization bytes
  peak live memory
```

```text
ResourceEstimate
  = R(LogicalGraph, PhysicalSchedule, resolved TargetProfile)
```

#### CostEstimate

실제 시간/성능 ranking에 필요한 추정이다.

```text
CostEstimate
  predicted latency
  predicted throughput
  launch overhead
  synchronization latency
  transfer time
  memory-service cost
  compute cost
  calibrated library/kernel cost
  confidence / provenance
```

```text
CostEstimate
  = C(ResourceEstimate,
      PhysicalSchedule,
      TargetProfile,
      optional CostProfile)
```

`CostProfile`이 없으면 CostEstimate를 생략하거나 conservative heuristic으로 후보를 ranking할 수 있다. **legality와 ResourceEstimate는 CostProfile이 없어도 계산 가능해야 한다.**

따라서 primitive registry에 `registers=32`처럼 넣지 않는다. primitive/capability는 `output마다 accumulator가 필요`, `이 축은 reduction`, `이 input tile은 재사용됨`, `logical collective가 필요` 같은 구조만 제공한다.

#### 4.19.0 ResourceEstimate derivation invariants

과거 `primitive_blackbox_resource_composition.md`의 핵심 원칙을 현행 resource model에 유지한다.

**register pressure는 primitive별 register 숫자의 합이 아니다.**

```text
register pressure
  = max over scheduled execution points(
      simultaneously-live register-class values
    )
```

따라서 다음이 필요하다.

- scheduled def/use와 last-use
- fused producer value의 lifetime
- branch/join에서 동시에 살아 있는 값
- reduction accumulator lifetime
- vector/tile temporaries
- backend lowering이 추가하는 temporary

두 primitive가 각각 register 10개를 쓴다고 해도 lifetime이 겹치지 않으면 peak는 20이 아니라 10에 가까울 수 있고, 반대로 producer value를 오래 유지하면 20을 넘을 수도 있다.

**accumulator requirement도 primitive 고정 숫자가 아니다.**

```text
AccumulatorRequirement
  logical reduction/output relation
  accumulator dtype constraints
  partial-reduction semantics

AccumulatorRealization
  outputs per lane/thread
  partial sums per output
  vector/matrix instruction choice
  register vs scratchpad staging
```

**scratchpad/shared/LDS requirement도 tile/staging의 함수**다.

```text
WorkingStateUsage
  = S(access reuse,
      tile shape,
      pipeline stages,
      buffering strategy,
      target allocation rules)
```

**memory traffic은 materialized logical edge와 physical representation을 함께 본다.** fused-away intermediate는 독립 external-memory write/read가 없을 수 있지만, cache/L2 hit 여부와 실제 DRAM transaction은 CostEstimate/measurement 영역이다.

따라서 resource model은 하나의 scalar `resource_cost`로 너무 일찍 축약하지 않는다.

```text
ResourceEstimate
  register classes
  scratchpad/shared
  live materialized bytes
  external-memory traffic amount
  synchronization/work counts
  occupancy/concurrency bounds
  ...
```

후보 ranking에서만 CostEstimate가 이 여러 축을 target-dependent performance metric으로 결합한다.


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

실제 runtime measurement는 `CompiledResourceReport`와 분리한다.

```text
ExecutionMeasurement
  artifact id
  input/workload signature
  elapsed time / throughput
  transfer time
  counters if available
  runtime/driver/clock provenance
```

측정값은 `CostProfile` calibration이나 autotuning database에 반영할 수 있지만, TargetProfile hard fact를 덮어쓰지 않는다.


#### 4.19.2 TargetProfile은 data + query interface다

TargetProfile을 모든 vendor 규칙을 정적으로 열거한 거대한 struct로 만들지 않는다.

```text
TargetFacts
  stable capacities / limits / identities

TargetQueries
  vectorization_legal(...)
  memory_transaction_structure(...)
  occupancy_bound(...)
  matrix_instruction_candidates(...)
  atomic_support(...)
  async_copy_candidates(...)
  preferred_layout(...)
```

hard fact는 versioned data로 보존하고, 복잡하거나 architecture-specific한 규칙은 query implementation으로 캡슐화한다.


### 4.20 CPU와 GPU에서 실제로 필요한 정보

공통으로 logical shape/dtype, iteration/dependency axes, reduction/scan semantics, access relation, alias/effect, working-set/reuse structure, vectorization/reassociation legality가 필요하다. Downstream planning에서는 여기에 RepresentationFacts(stride/alignment 등), memory hierarchy, register/vector capacity, parallel execution capacity, cost model을 결합한다.

CPU에서 특히 중요한 target facts는 core/hardware-thread topology, SIMD fixed/scalable widths, vector register classes, cache hierarchy/cache-line size, NUMA, gather/scatter/reduction cost, prefetch capability다.

GPU에서 특히 중요한 target facts는 SM/CU count, warp/wave/subgroup widths, workgroup limits, resident workgroup/subgroup limits, register classes/capacity, shared/LDS, coalescing/transaction rules, bank organization, subgroup collectives, barriers/atomics, matrix/tensor instruction signatures, async copy/pipeline, grid/cluster limits다.

이 차이 때문에 target schema를 NVIDIA의 `SM/register/shared-memory` 세 필드에 맞춰 고정하지 않는다.

### 4.21 현재 RustJ에 권장하는 최소 구현 범위

처음부터 완전한 hardware database를 만들지 않는다.

```text
H0-v0 Logical structural contract
  shape / dtype / valence
  IterationDomain: Map | Reduce 우선
  AxisSemantics: 필요한 최소 subset
  AccessFact: Known(simple AccessRelation) | Opaque
  NumericSemantics: 최소 contract
  DependencyRequirement: Independent | Reduction 우선

H0-later
  Window / Scan / Gather / Scatter / indirect access
  richer AccessRelation / masks / invariance / constraints

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

**Route Partition 입력**은 Logical Array IR과 adapter/backend capabilities다. RustJ-native route를 선택한 경우에만 Physical Planner가 Logical Array IR + TargetProfile + optional CostProfile + backend/library capabilities를 입력으로 받는다.

**RustJ-native Physical Plan / Planning Report**에는 fusion/kernel region, materialization boundary, target placement, axis/thread/vector mapping, chosen layout, tiling, memory-space staging, pipeline stages, synchronization, accumulator realization, placement/transfer, buffer lifetime/reuse, backend strategy, derived ResourceEstimate와 optional CostEstimate를 둔다.

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
  WorkingStateResident
  Bufferize(memory_space)
  Rematerialize(recompute_source_or_region)
  ExternalResource
```

따라서 Logical IR에서 `Materialize`를 일반적인 실행 op처럼 남발하지 않는다. explicit checkpoint처럼 **J/RustJ semantics 자체가 저장을 요구하는 경우**에만 semantic storage op/requirement가 존재한다.

`Rematerialize`는 semantic value를 삭제하는 것이 아니라 **동일 value를 필요 시 다시 계산하는 physical/schedule 선택**이다. 따라서 checkpoint requirement와 rematerialization decision을 분리한다. explicit checkpoint가 있으면 저장 의무가 생길 수 있지만, ordinary ephemeral value는 planner가 `Bufferize`와 `Rematerialize` 사이를 resource/cost 모델로 선택할 수 있다. 이 선택의 legality는 purity/effect/error-order/name-binding/state dependency를 증명해야 하며 단순히 계산량이 작다는 이유로 재계산하지 않는다.

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

#### 4.24.2 mutable state는 primitive 내부에 숨기지 않는다

후기 `japchae` D-27/D-31의 결정은 현행 RustJ에서도 유지한다.

weight, gradient, optimizer state, checkpoint, routing map처럼 실행 사이에 관찰·재사용·갱신되는 array state는 특별한 “layer-owned field”가 아니라 **명시적인 array/resource identity**로 표현한다.

```text
StateResource
  logical array type/shape
  lifetime requirement
  access/effect permissions
  initialization / persistence semantics
  optional external visibility
```

역할 이름은 semantic kind를 결정하지 않는다.

```text
weight
gradient
optimizer m/v
step counter (rank-0 array)
checkpoint
routing map
```

모두 J 관점에서는 array/value/resource이며, 차이는 lifetime과 Read/Write/Accumulate effect 및 storage requirement다.

parameterized adverb가 stateful computation을 도출하더라도 mutable state를 derived verb 내부의 숨은 object field로 캡슐화하지 않는다.

```text
DerivedVerb
  references StateResource ids
  + pure/declared computation semantics

StateResource
  exists outside the verb
```

compile-time 고정 hyperparameter처럼 실행 중 변하지 않는 값은 immutable parameter/attribute로 derived verb에 캡처할 수 있다. **mutable runtime state와 compile-time constant를 구분**한다.

과거 “materialized array”라는 용어는 다음 두 개념을 한 단어로 묶었다.

```text
semantic side:
  StateResource / StorageRequirement / lifetime/effects

physical side:
  MaterializationDecision / BufferId / offset / memory space
```

현행 RustJ에서는 이를 분리한다. named state의 identity/lifetime이 compile time에 알려져도 physical fixed offset을 반드시 미리 정할 필요는 없다.

과거 Flow–Storage 연구의 “in-band memory vocabulary” 아이디어는 이 모델 위에 선택적으로 올릴 수 있다. `load/store/emit/cp` 같은 표기가 채택되더라도 그것은 physical buffer 명령이 아니라 **semantic resource/effect declaration**이어야 한다. external/native lowering이 이를 실제 load/store/copy/checkpoint로 어떻게 실현할지는 별개다.



#### 4.24.3 APEX/Co-dfns/TAIL 연구를 반영한 middle-end substrate

APEX, Aaron Hsu의 Co-dfns 연구, APL→TAIL→Futhark 연구를 비교한 결과, 현재 RustJ의 `Semantic IR → ResolvedCallFacts → Logical IR → Optimizer → Route/Schedule` 방향은 유지하되 **analysis substrate를 명시적으로 추가**한다.

근거:
- APEX source/research: https://gitlab.com/bernecky/apex , https://www.snakeisland.com/ms.pdf
- Aaron W. Hsu, *The Key to a Data Parallel Compiler*, DOI 10.1145/2935323.2935331: https://dl.acm.org/doi/10.1145/2935323.2935331
- Co-dfns source pin `4e6d3e3002f2109360d24278776c5b5a4f65db0d`: https://github.com/Co-dfns/Co-dfns
- Dyalog'16: https://elsman.com/pdf/Dyalog16.pdf
- Henriksen et al., FHPC'16: https://elsman.com/pdf/fhpc16futhark.pdf
- 자세한 Source → Observation → RustJ Decision 매핑은 `FOUNDATIONS.ko.md` Part XX를 따른다.

권장 middle-end:

```text
J Semantic IR
      ↓
Binding / Effect Analysis
      ├─→ StructuralOpportunity extraction
      ↓
Logical SSA
      ↓
GraphIndex / AnalysisIndex
      ↓
MorphologyEngine
      ↓
Interprocedural Fixed Point
      ↓
SpecializationEngine
      ↓
High-level Logical Parallel IR
      ↓
Optimization Proofs
      ↓
RoutePartition
      ↓
Schedule / Physical Plan
```

이 구조는 APEX/Co-dfns/TAIL을 복제하는 것이 아니다. 각 연구에서 검증된 array-compiler insight를 **full J semantics + jsource-compatible frontend**라는 RustJ 경계에 맞게 재배치한 것이다.

#### 4.24.4 MorphologyEngine: ValueFacts를 lattice/fixpoint 분석으로 승격한다

APEX의 array morphology에서 가져오는 핵심은 type inference가 아니라 **array property의 정식 data-flow analysis**다.

```text
ValueFacts
  TypeFact
  RankFact
  ShapeFact
  ItemCountFact
  ConstantFact
  ArrayPropertyFacts
  ConstraintSet
```

각 fact domain은 최소한 Unknown/Proven 계층, join/merge, transfer/refinement를 가져야 하며 interprocedural summary와 specialization propagation을 worklist/fixpoint로 반복할 수 있어야 한다.

`FunctionEntity`에는 actual argument-dependent morphology를 넣지 않는다. call/value/Logical node fact table이 소유한다.

#### 4.24.5 ArrayPropertyFacts와 FactWitness

APEX의 array predicates와 fact-origin 문제를 반영해 다음을 정식 분석 domain으로 둔다.

```text
ArrayPropertyFacts
  IntegralValued
  NonNegative
  Unique
  SortedAscending / SortedDescending
  Permutation
  KnownRange
  AllEqual
  ...
```

그리고 optimization에 사용되는 fact는 가능한 한 provenance를 갖는다.

```text
Fact<T>
  abstract_value
  witness

FactWitness
  Constant
  SameAs(ValueId, relation)
  DerivedFrom(NodeId, RuleId)
  BindingVersion(...)
  RuntimeGuard(GuardId)
  CellApplyFrame(...)
  ...
```

check elimination, fusion legality, route eligibility, specialization은 witness 없는 optimistic fact에 의존하지 않는다.

#### 4.24.6 GraphIndex는 semantic DAG의 replacement가 아니라 derived analysis view다

Hsu의 Node Coordinate Matrix와 현재 Co-dfns의 inverted-table AST는 parent/depth/type/kind/binding/source-range를 columnar form으로 보관해 subtree/group analysis를 batch operation으로 처리하는 장점을 보여 준다.

RustJ canonical semantic representation은 계속 immutable shared DAG다. 다만 다음 sidecar를 만들 수 있다.

```text
GraphIndex
  NodeId[]
  parent[]
  depth[]
  preorder[]
  subtree_end[]
  opcode[]
  entity_id[]
  scope_id[]
  def/use index
  source-origin index
```

용도:
- op/entity/scope/specialization key별 grouping
- use-def/liveness
- bottom-up/top-down summary
- dependency-level parallel pass
- CPU SIMD/멀티코어 batch analysis

**금지:** semantic identity를 dense matrix/SoA 하나에 종속시키거나 모든 pass를 data-parallel form으로 강제하지 않는다.

#### 4.24.7 SpecializationEngine: call-site clone 대신 cacheable SpecializationKey

APEX의 call-site specialization과 TAIL의 explicit type/rank instantiation을 일반화한다.

```text
SpecializationKey
  FunctionEntityId
  binding version / guard
  valence
  relevant dtype classes
  effective rank facts
  relevant shape class
  relevant ArrayPropertyFacts
  fit/tolerance/rank policy
```

동일 key는 analyzed region/JIT artifact를 재사용한다.

specialization explosion을 막기 위해 exact shape/constant/property는 **algorithm/lowering 선택에 실제로 영향을 줄 때만** key에 넣는다. 필요하면 abstract-state widening/merge를 허용한다.

#### 4.24.8 High-level parallel structure를 Logical IR에 보존한다

TAIL→Futhark 연구는 map/reduction nests를 explicit하게 유지해야 fusion, nested-parallelism flattening, coalesced access optimization을 수행할 수 있음을 보여 준다.

따라서 다음은 early scalarization 금지 대상이다.

```text
CellApply
MapCells
Reduce
Scan
Window
Gather / Scatter
StaticReindex
OuterProduct
MatMul
Conv
Loop / Power
```

nested `CellApply(Reduce(...))`, `Reduce(CellApply(...))` 등은 Logical IR에서 보존한다. flattening/segmentation/thread mapping은 optimizer/schedule decision이다.

이 절의 `MatMul`, `Conv`, `OuterProduct` 같은 named high-level op 보존 원칙은 **Graph Basis identity 보존** 문제다. 4.24.11의 Execution basis vocabulary와 경쟁 관계가 아니다. named op는 Graph IR에서 algorithm/access/resource identity를 black box로 유지할 수 있고, execution lowering은 필요하면 `ExecutionBasisExpansion`을 통해 `WindowView + Contract` 같은 더 compositional한 실행 graph를 제공할 수 있다. 더 작은 expansion은 실제 성능상 이득이 있을 때만 추가한다.

#### 4.24.9 Pure region extraction과 route precondition

TAIL/Futhark와 Co-dfns는 실제 compiler subset을 제한한다. RustJ는 그 제한을 J language restriction으로 채택하지 않는다.

```text
Full J semantic graph
      ↓
EffectAnalysis
      ↓
RegionPartition
  PureArrayRegion
  GuardedDynamicRegion
  StatefulRegion
  RuntimeSemanticRegion
```

external backend의 static scope/static rank/no-execute/pure-array requirement는 **route precondition**이다.

```text
route cannot lower X
  ≠ X is invalid J
```

route가 실패하면 native route, guarded JIT, runtime semantic path를 선택한다.

#### 4.24.10 Parameterized Lowering Recipe

TAIL→Futhark가 static type/rank information에 따라 primitive code skeleton을 specialize하는 방식을 일반화한다.

```text
Resolved Semantic Op
+ ResolvedCallFacts
+ TargetCapability
        ↓
ParameterizedLoweringRecipe
        ↓
candidate realizations
```

예를 들어 `Take`는 sign/bounds/fill/view legality에 따라 ViewTake, DirectSlice, PadAndSlice, GenericTakeKernel, RuntimeSemanticFallback으로 분기할 수 있다.

semantic op 하나가 kernel 하나라는 가정을 두지 않는다.




#### 4.24.10a Graph Basis와 Execution Basis를 분리한다

`basis`는 하나의 계층을 뜻하지 않는다. RustJ에서는 최소한 두 개를 명시적으로 구분한다.

~~~text
J Semantic Function / Derived Verb
        ↓
J Graph IR
  GraphBasis / graph algebra / rewrite
        ↓
Execution Semantic Lowering
        ↓
Logical Execution IR
  ExecutionBasisKind / ExecutionBasisPayload
        ↓
target lowering / library / custom kernel / physical schedule
~~~

**Graph Basis**는 역사적 JAXA의 basis-verb 연구에 가까운 개념이다. 목적은 J 표기가 제공한 계산 구조를 대수적으로 다루고, rewrite/equivalence/resource/fusion 분석에 사용할 생성원과 구조 단위를 보존하는 것이다. 이 층의 granularity는 hardware instruction이나 실행 loop의 최소성으로 정하지 않는다.

특히 과거에 `conv`를 black box로 남기기로 한 판단은 **Graph IR에서 convolution identity를 성급하게 더 작은 graph primitive로 쪼개지 않는다**는 뜻이다. convolution이라는 algorithm/access/resource 구조를 Graph Basis에서 그대로 보존하면 rewrite, equivalence, fusion boundary, symbolic resource reasoning이 원래 identity를 이용할 수 있다.

이 결정은 Execution IR의 분해를 금지하지 않는다.

~~~text
GraphBasis::Structured(Conv)
   ├→ ExecutionBasis(WindowView → Contract)
   ├→ ExecutionBasis(StaticReindex → Elementwise → Reduce)
   ├→ library call
   └→ custom fused kernel
~~~

즉 **graph-level black box와 execution-level decomposition은 독립적인 결정**이다. lowering은 원 GraphBasis identity와 equivalence/provenance를 유지한 채 하나 이상의 ExecutionBasis op로 펼칠 수 있다.

Derived verb도 같은 순서를 따른다.

~~~text
Derived Verb
  → GraphBasis composition
  → ExecutionBasis composition
  → realization
~~~

예를 들어 ranked reduction은 Graph IR에서 outer-to-inner `CellApply → Reduce` 구조를 보존할 수 있다. 이를 하나의 effective execution primitive로 일찍 평탄화하지 않는다. 이후 Execution Semantic Lowering이 J의 fill/assembly/error semantics를 증명한 범위에서 별도의 `ExecutionBasisKind::CellApply`, `ExecutionBasisKind::Reduce` 또는 흡수된 realization으로 내린다.

코드에서는 두 층의 이름을 공유하지 않는다.

- J Graph IR: `GraphBasis`, `GraphBasisKind`
- Logical Execution IR: `ExecutionBasisKind`, `ExecutionBasisPayload`
- Execution lowering registry: `ExecutionBasisLoweringCapability`
- optional Execution-IR refinement: `ExecutionBasisExpansion`

같은 단어인 Map/Reduce/Reindex가 양쪽에 나타날 수 있지만 **타입, 소유 fact, rewrite 법칙, granularity 조건이 다르다.**

#### 4.24.11 Execution basis vocabulary v0.2: 최소성이 아니라 성능상 유효한 실행 최적화 경계

이 절의 **Execution basis operation**은 “더 이상 분해할 수 없는 최소 primitive”를 뜻하지 않는다. Graph Basis의 최소성/생성원 문제와도 별개다. **현재 수준의 semantic/array identity를 보존했을 때 optimizer와 planner가 실제 성능상 이득을 얻는 연산 단위**를 뜻한다.

따라서 어떤 basis op A가 더 작은 B + C + ... 로 표현 가능하다는 사실만으로 A를 제거하지 않는다. refinement는 다음 중 하나 이상의 이득이 입증되거나 강하게 예상될 때만 채택한다.

- producer/consumer fusion 범위 확대
- intermediate materialization 제거
- access/index-map 합성
- SIMD/SIMT/nested-parallel decomposition 개선
- tensor/GEMM/warp/shared-memory 등 target-specific lowering 개방
- memory traffic 또는 synchronization 감소
- 여러 semantic op 사이 optimizer pass 재사용
- cost model이 의미 있는 realization 선택지를 추가로 얻음

반대로 분해 후 planner가 원래의 구조를 다시 pattern-match해야 하거나 algorithm identity를 잃어 specialized implementation 선택이 어려워지면 큰 basis를 그대로 유지한다. **basis refinement는 version-up 가능한 optimization contract**이며 language semantics의 조건이 아니다.

잠정 v0.2 vocabulary:

~~~text
Iteration / application
  IndexSpace / Generate
  Elementwise / MapN
  CellApply

Access / views / update
  RegularReindex
  Gather
  Scatter / Amend
  ScatterCombine
  WindowView
  SegmentView
  Permute

Collective / algebra
  Reduce
  Scan
  Contract

Structural
  Concat / Assemble
  Replicate / Compact / Expand

Ordering / classification
  Grade
  Lookup / Classify
  GroupBy

Nested value structure
  NestedTraverse

Large structured kernels
  LinearSolve
  StateMachine          // provisional structured kernel
~~~

다음은 같은 목록에 섞지 않는다.

~~~text
Representation axis
  Dense / Sparse / future tiled or compressed layouts

Control / function semantics
  Power/Iterate, Agenda, Under, Atop, Bond, gerund,
  explicit definition, adverse/obverse, memoization, ...

Value construction / runtime
  Box/Open, assignment/binding, execute/format,
  foreign, random/state, symbol interning, ...
~~~

Box/Open은 nested computation 자체보다 value construction/representation에 가깝다. 반면 boxed tree를 일정 level에서 방문하는 NestedTraverse는 traversal/parallelization 선택을 보존할 가치가 있으므로 basis 쪽에 둔다.

LinearSolve, Grade, GroupBy는 더 작은 operation으로 구현할 수 있어도 지금 분해하지 않는다. solver/ordering/grouping identity 자체가 algorithm, library, sparse/dense, CPU/GPU realization 선택에 직접 유용하기 때문이다. StateMachine은 ;: 의 일반 sequential-machine semantics를 잃지 않기 위한 provisional structured kernel이며, 첫 구현은 generic sequential route여도 된다.

##### 다른 array-language/compiler basis와의 역대조

비교 자료:
- Futhark minimal basis: https://www.futhark-lang.org/blog/2019-04-10-what-is-the-minimal-basis-for-futhark.html
- Futhark SOACs: https://www.futhark-lang.org/docs/prelude/doc/prelude/soacs.html
- Accelerate: https://hackage.haskell.org/package/accelerate/docs/Data-Array-Accelerate.html
- Lift: https://lift-project.readthedocs.io/en/latest/lift-overview/
- SaC: https://sac-home.org/_media/docs%3Atutorial.pdf
- MLIR Linalg: https://mlir.llvm.org/docs/Dialects/Linalg/
- NESL: https://www.cs.cmu.edu/afs/cs/project/pscico/doc/nesl/manual/

| RustJ structure | 외부 비교에서 반복되는 근거 | v0.2 결정 |
|---|---|---|
| Elementwise/MapN | Futhark map, Accelerate map/zipWith, Lift map, NESL apply-to-each, Linalg parallel iterator | 유지 |
| Reduce | Futhark/Accelerate/NESL에서 구조를 직접 보존; generic reconstruction은 fusion/parallel reduction 기회를 잃을 수 있음 | 강하게 유지 |
| Scan | Futhark/NESL에서 독립 parallel prefix 구조 | 강하게 유지 |
| RegularReindex | Accelerate backpermute, Linalg indexing maps, SaC index expressions | 유지 |
| Gather | indexed/irregular read의 memory/coalescing 특성이 regular reindex와 다름 | 유지 |
| Scatter/ScatterCombine | Futhark scatter/reduce_by_index, Accelerate permute, NESL indexed write | 강하게 유지 |
| WindowView | Lift slide/pad, Accelerate stencil, LAILA pull-array/index-function 접근과 같은 neighborhood reuse 정보 | 유지 |
| SegmentView | segmented fold/scan, NESL flatten/partition과 연결 | 유지 |
| Contract | Linalg contract처럼 contraction identity가 GEMM/tensor/microkernel 선택에 유용 | 유지 |
| CellApply | J rank/cell/frame와 result assembly를 explicit하게 보존해야 하며 uniform proof 뒤에만 MapCells/flat parallel form으로 낮출 수 있음 | J-specific structured basis로 유지 |
| Grade | 다른 언어에서는 library/algorithm으로 구성 가능하지만 J primitive identity와 ordering algorithm 선택이 직접 중요 | 유지, refinement는 후속 |
| Lookup/Classify/GroupBy | Futhark reduce_by_index/segmented routes와 유사한 hash/sort/atomic/segment 선택 공간 | 유지 |
| NestedTraverse | NESL nested sequences가 보여 주듯 irregular nested traversal은 dense Map과 다른 구조 | 유지 |
| LinearSolve | named structured op/library-call granularity를 보존하는 것이 solver 선택에 유리 | 유지 |
| StateMachine | 비교 언어의 core array basis에서는 약한 대응; J ;: semantics 때문에 우선 structured kernel로 보존 | provisional |

Futhark가 보여 주는 중요한 경고는 **표현상 minimal basis와 optimization basis가 다르다**는 점이다. 많은 연산을 map+iota+scatter 류로 재구성할 수 있어도 reduce 같은 identity를 지우면 fusion/parallel implementation 선택을 잃을 수 있다. RustJ는 따라서 “분해 가능”을 refinement 이유로 사용하지 않는다.

#### 4.24.12 J primitive → provisional execution-basis coverage matrix

검토 기준:
- J 공식 Vocabulary: https://www.jsoftware.com/help/dictionary/vocabul.htm
- jsource 계열 special phrase inventory: https://jsoftware.com/help/dictionary/special.htm

이 표의 목적은 **J의 source primitive를 basis vocabulary로 대체하는 것**이 아니다. source/parser/Semantic IR에서는 원래 J primitive/derived identity를 보존하고, Semantic Analyzer가 call facts를 해석한 뒤 Logical IR에서 다음 basis graph를 만든다.

분류:
- **Direct**: 하나의 basis identity가 중심
- **Compose**: 둘 이상의 basis를 명시적으로 조합
- **Structured**: 큰 semantic/algorithm identity를 현재 유지
- **Control**: function/control IR이 basis graph의 조합/반복을 결정
- **Value/Rep**: value construction 또는 representation axis
- **Runtime**: stateful/dynamic/runtime semantic route
- **Cell kernel**: array traversal은 CellApply/Elementwise가 담당하고 내부 scalar/cell algorithm은 opaque 가능

| J primitive/form | 의미 요약 | provisional basis expansion | 필요한 facts/checks | 분류 / 성능상 이유 |
|---|---|---|---|---|
| = | Self-Classify / Equal | monad Classify; dyad Elementwise(Equal) | equality semantics, dtype, tolerance/fit | Direct; classification identity는 hash/sort/group route에 유용 |
| < | Box / Less Than | monad value Box; dyad Elementwise(Lt) | box/value type; comparison domain | Value/Rep + Direct |
| <. <: >. >: | floor/min, decrement/≤, ceiling/max, increment/≥ | Elementwise | dtype/promotion/domain | Direct |
| > | Open / Greater Than | monad value Open; dyad Elementwise(Gt) | boxed/open assembly; comparison domain | Value/Rep + Direct |
| + +. +: * *. *: - % %: ^ ^. &#124; | scalar arithmetic/logical/transcendental families | Elementwise(MapN) with scalar payload | dtype, promotion, domain, overflow/FP/error policy | Direct; scalar payload는 target intrinsic/SIMD 후보 |
| -. | Not / Less | monad Elementwise(Not); dyad Lookup(Membership in y) → Compact(items of x not present) | item-cell agreement, equality/tolerance, stable order | Compose; dyadic Less는 set/item exclusion이므로 scalar elementwise로 축소하지 않음 |
| -: | Halve / Match | monad Elementwise(Halve); dyad CellApply(MatchKernel), optionally shape/box check + Equal + Reduce(And) | shape, boxing, tolerant equality, rank/cell | Cell kernel/Compose; whole-cell Match identity를 보존 |
| %. | Matrix Inverse / Matrix Divide | LinearSolve or related structured linear-algebra kernel | rank/shape, singularity, numeric contract | Structured; LU/QR/SVD/library/sparse solver 선택을 보존 |
| $ | Shape Of / Reshape | monad ShapeFact; dyad RegularReindex with periodic/fill policy | target shape values, item count, fill/error | Compose; copy를 피하는 view/index-map 최적화 |
| $. | Sparse representation operations | same semantic basis op + RepresentationFacts::Sparse; explicit conversion/inspect is representation op | sparse axes/fill/index/value validity | Value/Rep; sparse는 별도 semantic basis가 아님 |
| ~. | Nub | Classify → Compact(first representative) | equality/tolerance, stable-first semantics | Compose; hash/sort specialization 가능 |
| ~: | Nub Sieve / Not-Equal | monad Classify → first-mask; dyad Elementwise(Ne) | equality/tolerance | Compose/Direct |
| &#124;. | Reverse / Rotate | RegularReindex | axis/rank, shift normalization | Direct; no-copy/index-map path |
| &#124;: | Transpose | RegularReindex(Permutation) | axis permutation validity | Direct; layout/coalescing optimization |
| . | Determinant / Dot Product | monad CellApply(DeterminantKernel); dyad Contract(u,v) | rank, contraction axes, reducer/combine semantics | Cell kernel + Direct; contraction identity enables GEMM/tensor routes |
| , | Ravel / Append | monad RegularReindex; dyad Concat/Assemble | shape agreement, dtype promotion | Direct |
| ,. ,: | Ravel Items/Stitch, Itemize/Laminate | RegularReindex and/or Concat/Assemble | rank/shape agreement | Compose |
| ; | Raze / Link | Open/boxed value traversal + Concat/Assemble | box/open validity, result assembly | Compose + Value/Rep |
| ;. | Cut | fret cuts SegmentView → CellApply; tessellation WindowView → CellApply | fret/window spec, boundaries, fill, assembly/errors | Compose; avoids building cut cells and exposes segment/window parallelism |
| ;: | Words / Sequential Machine | monad configured StateMachine; dyad general StateMachine | transition/input tables, state, emission, errors | Structured provisional; future SIMD/transition-composition possible |
| # | Tally / Copy | monad shape/item-count fact; dyad Replicate/Compact/Expand | counts nonnegative/integral, result size | Direct; prefix/compaction implementation 선택 가능 |
| #. | Base 2 / Base | Scan/Generate weights → Elementwise(Mul) → Reduce(Add) or preserved base kernel | radix/value domain, overflow | Compose; keep specialized kernel if cheaper |
| #: | Antibase 2 / Antibase | Generate/Scan radix weights + Elementwise(div/residue) + Assemble | radix/domain/result shape | Compose |
| ! | Factorial / Out Of | Elementwise scalar/cell combinatorial kernel | integer/general numeric domain | Cell kernel; internal algorithm need not be array-basis-expanded |
| !. | Fit | modifies semantic/numeric/error contract of operand graph | fit/tolerance/fill policy | Control/semantic annotation |
| !: | Foreign | runtime/foreign semantic route | foreign id, effects/capability | Runtime |
| / | Insert / Table | monad Reduce(u); dyad IndexSpace/FrameMap → CellApply(u) | reducer order/associativity/identity, rank/assembly | Direct/Compose; reduction identity retained even when non-reassociable |
| /. | Oblique / Key | oblique SegmentView/Reindex → CellApply; key Classify/GroupBy → SegmentView → CellApply, optionally grouped reduction | grouping equality/order, segment descriptors, assembly | Compose; avoids materialized groups and opens reduce-by-index route |
| /: \: | Grade Up/Down / Sort | Grade; sort result can be Grade → Gather | comparison order, stability/tolerance, dtype | Direct; radix/merge/small/GPU algorithm identity retained |
| \ | Prefix / Infix | insert-compatible prefix Scan; general prefix SegmentView(prefix family) → CellApply; infix WindowView/SegmentView → CellApply | window length, order, boundaries, assembly | Direct/Compose |
| \. | Suffix / Outfix | suffix Scan when legal or segment family; outfix SegmentView + Concat/Assemble → CellApply | same as above | Compose |
| [ ] [: | Same/Left, Same/Right, Cap | value projection / function-graph semantics | valence, provenance | Control/value; no new compute basis |
| { | Catalogue / From | monad Cartesian IndexSpace + Gather + Assemble; dyad Gather | index bounds, boxed catalogue shapes | Direct/Compose |
| {. {: }. }: | Head/Tail/Take/Drop/Behead/Curtail | RegularReindex or scalar Gather | bounds, fill, rank | Direct |
| {:: | Map / Fetch | NestedTraverse to produce leaf paths; fetch = path-guided nested Gather/Open | path validity, boxed structure | Direct + Value/Rep |
| } | Item Amend / Amend | Scatter/Amend, possibly ScatterCombine when combining update is explicit | bounds, overlap/update ordering, alias | Direct; in-place/atomic/scatter choices |
| " | Rank | resolve rank/cell/frame then CellApply | RankSpec, frame agreement, fill/empty/result assembly, error order | Control → Direct basis; central J optimization boundary |
| ". | Do / Numbers | parse/execute or numeric-conversion runtime semantic route | dynamic binding/parser state, numeric syntax | Runtime |
| ": | Default Format / Format | formatting runtime/cell kernel | locale/format spec, dtype | Runtime/Cell kernel |
| Tie/Evoke Gerund | gerund construction/selection | function/gerund semantic graph | POS/binding/gerund selection | Control |
| @ @: @. | Atop/At/Agenda | compose/select function graphs; resulting calls lower normally | valence, selected branch, effects | Control |
| & &: &. &.: | Bond/Compose/Appose/Under | function graph transformation; Under keeps inverse/obverse contract | inverse availability, rank/valence, effects | Control |
| ? ?. | Roll/Deal | stateful random generation; array shape may use Generate | RNG state/seed, domain, uniqueness for deal | Runtime + optional Generate |
| a. a: | Alphabet / boxed empty constant | constant/value construction | encoding/value type | Value |
| A. | Anagram Index / Anagram | permutation rank/unrank cell kernel; application Permute/Gather | permutation validity/range | Structured cell kernel + Direct |
| b. | Boolean/Basic | scalar/bitwise semantic kernel, optionally Elementwise/Reduce when derived | boolean function id, dtype | Cell kernel |
| C. | Cycle-Direct / Permute | permutation representation conversion + Permute/Gather | cycle/direct validity | Value/Rep + Direct |
| d. D. D: | Derivative/Secant family | function transformation producing a new semantic graph or specialized numeric kernel | derivative rules, function purity/domain | Control/Cell kernel |
| e. | Raze In / Member | monad value traversal/raze-in; dyad Lookup(Membership) | equality/tolerance, boxed semantics | Direct + Value |
| E. | Member of Interval/pattern occurrence | WindowView → Elementwise(Match) → Reduce/Match as applicable | pattern shape, equality, boundaries | Compose; explicit windows expose fusion |
| f. | Fix | function semantic specialization/fixing | binding versions | Control |
| H. | Hypergeometric | CellApply specialized numeric kernel | parameter/domain/numeric policy | Cell kernel |
| i. | Integers / Index Of | monad IndexSpace/Generate; dyad LookupFirst | shape/integer domain; equality/tolerance | Direct |
| i: | Steps / Index Of Last | monad Generate; dyad LookupLast | same | Direct |
| I. | Indices / Interval Index | monad Compact(IndexSpace,predicate); dyad ordered interval Lookup | ordering, bounds | Direct/Compose |
| j. r. o. | complex/polar/circle families | Elementwise scalar payload | numeric/domain | Direct |
| L. | Level Of | NestedTraverse/nested metadata computation | box tree shape | Direct; tree traversal distinct from dense map |
| L: | Level At | NestedTraverse(level selector) → CellApply(u) → nested reconstruct | level selector, reconstruction, assembly | Direct/Compose |
| M. | Memo | memoization/cache around semantic function graph | key equality, effects/purity | Control/runtime |
| p. p.. | Polynomial roots/evaluation/derivative/integral | specialized cell kernel; evaluation may use Contract/Reduce | coefficient dtype, numeric stability, output shape | Cell kernel; expand only when optimization pays |
| p: q: | Primes / factorization | CellApply specialized variable-result kernel | integer domain, dynamic result assembly | Cell kernel |
| s: u: x: | Symbol/Unicode/Extended Precision | representation/runtime conversion or Elementwise conversion | encoding/interning/numeric exactness | Value/Rep/Runtime |
| S: | Spread | NestedTraverse(level selector) → CellApply(u) → FlatAssemble | level selector, heterogeneous result/assembly | Direct/Compose |
| t. t: T. | Taylor families | function transformation or specialized numeric graph/kernel | order/domain/precision | Control/Cell kernel |
| constant functions | constant broadcast/Generate when array result required | Generate/constant | dtype/shape from call context | Direct/value |
| ~ | Reflex/Passive/Evoke | function semantic transformation/name resolution | valence/binding | Control |
| ^: | Power | Iterate/loop over analyzed basis graph; static count may unroll/fuse | count, fixed-point/inverse semantics, effects | Control; no new array basis |
| $: | Self-Reference | function/control recursion | binding/function identity | Control |
| : :. :: | Explicit/Monad-Dyad, Obverse, Adverse | function/control/exception semantic graph | POS, inverse/obverse, error semantics | Control |
| =. =: | local/global assignment | binding/effect operation | locale/scope/version/effect ordering | Runtime/control |
| _ _. _: | infinity/indeterminate constants | constant/value semantics | numeric type | Value |
| .. .: | Even/Odd conjunction forms | function/control semantics; resulting graph lowers normally | operand POS/valence | Control |
| NB. | Comment | frontend only | none | frontend, no IR op |

**Coverage conclusion v0.2:** official Vocabulary의 각 entry는 위 basis composition, structured kernel, control/function layer, representation/value layer, runtime semantic layer 중 하나로 분류된다. 현재 이 pass에서는 새로운 array-computation basis family가 필요하다는 반례가 나오지 않았다.

##### jsource special-code inventory와의 교차검증

| jsource special family | RustJ logical explanation |
|---|---|
| +/ .* | Contract(combine=*, aggregate=+) |
| $, 및 ravel-avoidance | RegularReindex/Reshape가 ravel materialization을 만들지 않음 |
| f;.n | SegmentView 또는 WindowView + CellApply(f) |
| f/;.n, +//., #/. | segment/group descriptors + Reduce/GroupBy; group cells를 만들지 않는 route |
| /:, /:~ | Grade |
| {/: | Grade → Gather의 fused candidate |
| +/\, =/\, f/\. | Scan 또는 ordered segment/window reduction |
| { | Gather |
| } / indexed amend phrase | Scatter/Amend |
| f"r | CellApply absorption/fusion candidate |
| i., i:, e. | Lookup/Classify |
| E. | WindowView + Match |
| +/%# mean family | Reduce(Add) + Tally + Divide, candidate fusion |

jsource가 phrase-by-phrase special code로 얻는 여러 이득을 RustJ에서는 **basis graph + capability/lowering rule**로 일반화할 수 있다.

#### 4.24.13 Derived forms closure check: 실제 execution-basis graph

primitive 한 개만 매핑해서는 J semantics coverage를 검증할 수 없다. modifier/derived form이 만들어 내는 implicit traversal과 control을 실제 basis graph로 내려 보아야 한다.

##### Rank

~~~text
u"r y
  ↓ ResolveRank(r, rank(y))
Cell/Frame partition
  ↓
Frame IndexSpace
  ↓
CellApply(u)
  ↓
J result Assemble
~~~

dyad:

~~~text
x u"(lr,rr) y
   ↓                         ↓
Resolve left cells      Resolve right cells
          \              /
           FrameAgreement
                 ↓
          SemanticCheck
        (length/error order)
                 ↓
          Frame IndexSpace
                 ↓
             CellApply(u)
                 ↓
              Assemble
~~~

CellApply(Elementwise), CellApply(Reduce), CellApply(Contract)는 각각 larger elementwise, batched/segmented reduction, batched contraction route로 흡수할 수 있다. semantic CellApply 자체는 optimization 전에도 보존한다.

##### Cut

~~~text
fret/mask
   ↓
SegmentDescriptor
   ↓
SegmentView(y)
   ↓
CellApply(u)
   ↓
Assemble
~~~

~~~text
window/tessellation spec
   ↓
WindowView(y)
   ↓
CellApply(u)
   ↓
Assemble
~~~

Cut 때문에 새 basis가 필요하지 않으며, 오히려 SegmentView와 WindowView를 구분할 근거가 된다.

##### Key / Oblique

~~~text
x
 ↓
Classify / GroupBy
 ↓
group descriptors ───── y
          \             /
           SegmentView
                ↓
            CellApply(u)
                ↓
             Assemble
~~~

u가 associative reduction으로 분석되면:

~~~text
GroupBy
   ↓
GroupedReduce / ScatterCombine
~~~

group array를 실제로 materialize하지 않는 것이 주요 성능 이점이다.

##### Power

~~~text
u^:n y
   ↓
Iterate {
  body = analyzed basis graph of u
  count = n
}
~~~

static small n에서는 unroll/cross-iteration fusion 후보가 될 수 있고 dynamic count는 loop로 남는다. fixed-point/inverse/boxed power의 J semantics는 control layer가 소유하며 새로운 array basis를 요구하지 않는다.

##### Boxed Level / Spread / Fetch

~~~text
u L:n y
  ↓
NestedTraverse(level=n)
  ↓
CellApply(u)
  ↓
NestedReconstruct
~~~

~~~text
u S:n y
  ↓
NestedTraverse(level=n)
  ↓
CellApply(u)
  ↓
FlatAssemble
~~~

~~~text
x {:: y
  ↓
path-guided NestedTraverse/Gather
  ↓
selected nested value
~~~

NestedReconstruct와 FlatAssemble는 output assembly policy이고 NestedTraverse가 traversal identity를 보존한다.

##### Sparse composition

Sparse는 basis를 복제하지 않는다.

~~~text
Reduce(+)
+ RepresentationFacts::Sparse
      ↓
sparse-aware lowering
~~~

~~~text
RegularReindex(Transpose)
+ RepresentationFacts::Sparse
      ↓
sparse index/value permutation
~~~

~~~text
CellApply(u)
+ RepresentationFacts::Sparse
      ↓
sparse cell route or conforming fallback
~~~

따라서 SparseReduce, SparseTranspose, SparseCellApply 같은 별도 semantic basis family를 만들지 않는다. sparse axes/fill/index/value는 representation facts이며 같은 J observable semantics를 다른 physical realization으로 실행한다.

##### closure 판정

Rank, Cut, Key/Oblique, Power, boxed Level/Spread/Fetch, sparse composition을 실제 graph로 내린 결과:

1. v0.2에 없는 새 array-computation basis family는 발견되지 않았다.
2. CellApply, WindowView, SegmentView, GroupBy/Classify, NestedTraverse의 필요성은 오히려 강화되었다.
3. StateMachine만은 다른 array compiler와의 대응 근거가 약하므로 **provisional structured kernel**로 유지한다.
4. basis를 더 작은 operation으로 refinement하는 작업은 이 closure의 선행조건이 아니다. 이후 benchmark/optimizer 구현에서 실질적 성능 이득이 확인되는 경우에만 ExecutionBasisExpansion rule을 추가한다.




#### 4.24.14 Execution-basis Logical IR node contract와 최소 analysis/lowering interface

4.24.11–4.24.13의 v0.2 vocabulary를 실제 A3 Logical IR로 옮길 때 basis 이름마다 임의의 struct를 따로 만들지 않는다. 모든 basis node는 공통 contract를 공유하고, 각 operation family가 필요한 추가 payload/fact/check를 명시한다.

핵심 경계:

~~~text
J Semantic entity / Resolved semantic op
        │
        ├─ original semantic identity
        ├─ ResolvedInstantiation
        └─ ResolvedCallFacts
                 │
                 ▼
        optional ExecutionBasisExpansion
                 │
                 ▼
           Logical basis graph
                 │
                 ├─ ValueFacts / ValueRoleFacts
                 ├─ IterationDomain / AxisSemantics
                 ├─ AccessFact
                 ├─ ConstraintSet / FactWitness
                 ├─ SemanticCheck
                 ├─ EffectSummary / SpeculationSemantics
                 ├─ DestinationRelation / StorageRequirement
                 └─ provenance
                         │
                         ▼
                LoweringRegistry query
                         │
                         ▼
              legal realization candidates
~~~

basis op는 target-independent다. warp width, SIMD width, tile, workgroup, shared memory, register budget, concrete layout, library handle 같은 것은 basis node payload에 넣지 않는다.

##### ResolvedInstantiation

polymorphic primitive/derived verb가 실제 call에서 어떤 instance가 되었는지를 ResolvedCallFacts와 specialization 사이에 명시적으로 기록한다.

~~~text
ResolvedInstantiation
  semantic_operation_id
  valence
  input dtype/rank/cell-rank instances
  output dtype/rank instance if known
  explicit/innate rank boundary
  relevant ValueRoleFacts
  numeric/fit policy identity
  source/binding provenance
~~~

이 구조는 semantic identity 자체가 아니다. 동일 FunctionEntity도 actual arguments에 따라 여러 instantiation을 가질 수 있다.

용도:
- IR dump/diagnostics
- differential test provenance
- SpecializationKey material 추출
- lowering lookup
- cache provenance

SpecializationKey에는 이 전체를 복사하지 않고 algorithm/lowering 선택에 실제 영향을 주는 field만 추출한다.

##### ValueRoleFacts

ValueFacts.shape는 “이 value 자체의 shape”다. 반대로 어떤 noun의 값이 다른 operation의 shape/axis/index/window specification으로 사용된다는 사실은 별도 role이다.

~~~text
ValueRoleFacts
  ShapeVector
  AxisVector
  AxisPermutation
  RankSpecifier
  IndexVector
  CountVector
  WindowSpec
  StrideSpec
  DilationSpec
  PaddingSpec
  SegmentDescriptor
  Permutation
  StateIdLike
  Unknown
~~~

ValueRoleFacts는 J type system에 새로운 nominal type을 추가하지 않는다. 같은 integer vector가 문맥에 따라 ordinary data 또는 ShapeVector가 될 수 있다. role은 call/basis analysis가 만든 fact이며 witness를 가질 수 있다.

##### first-class SemanticCheck

J-visible error condition을 compiler assertion과 구분한다.

~~~text
SemanticCheck
  predicate / required constraint
  J error kind
  semantic origin
  observable ordering dependency
  witness if already proven
~~~

예:
- shape agreement → length error
- index range → index error
- domain predicate → domain error
- permutation validity → index/domain error as defined by source primitive
- solver shape/singularity condition → corresponding J error contract

규칙:
1. compiler 내부 invariant assertion과 SemanticCheck를 같은 것으로 취급하지 않는다.
2. FactWitness가 predicate를 증명하면 check를 제거할 수 있다.
3. hoist/fuse/reorder는 SpeculationSemantics와 observable error order를 보존할 때만 허용한다.
4. A3-v0 single-block IR에서도 check는 MayRaise operation으로 존재할 수 있다. runtime branch/deoptimization이 필요한 Guard는 A3-v1의 책임이다.

현재 transition 구현의 src/analysis.rs::Node는 아직 “node 하나 = ValueId 하나”인 inspection snapshot이다. 따라서 이 구조에 zero-result SemanticCheck를 가짜 value-producing node로 억지로 넣지 않는다. 현재 단계에서는 direct ExecutionBasisKind, ResolvedInstantiation, ValueRoleFacts seam만 추가하고, first-class SemanticCheck는 A3의 operation/result 분리에서 zero-result operation으로 구현한다. 이것은 SemanticCheck를 후순위 의미로 낮추는 것이 아니라 잘못된 migration representation을 만들지 않기 위한 단계화다.

##### ExecutionBasisExpansion

큰 semantic/structured op가 basis graph와 동등하다고 알려져도 원래 identity를 삭제하지 않는다.

~~~text
ExecutionBasisExpansion
  source semantic/logical op
  applicability constraints
  expansion region/graph
  equivalence witness/rule id
  preserved numeric/error/order contract
  provenance
~~~

예:

~~~text
Conv
  ↔ WindowView + Contract

Pooling
  ↔ WindowView + Reduce

MatMul
  ↔ Contract

Mean
  ↔ Reduce(Add) + Tally + Divide
~~~

expansion은 mandatory lowering이 아니다. planner/optimizer는 original identity와 expansion 양쪽을 이용할 수 있다. refinement는 4.24.11의 성능 기준을 만족할 때 추가한다.

##### 공통 LogicalBasisOp contract

모든 basis operation은 conceptually 최소한 다음 query가 가능해야 한다.

~~~text
LogicalBasisOp
  identity
  operands / results
  ResolvedInstantiation
  IterationDomain
  AxisSemantics
  AccessFact[]                  // Known(...) | Opaque
  NumericSemantics
  ConstraintSet
  possible J error set / MayRaise
  EffectSummary
  SpeculationSemantics
  DestinationRelation
  StorageRequirement
  semantic/basis provenance
~~~

모든 field가 항상 Known일 필요는 없다. Unknown/Opaque는 language invalidity가 아니라 해당 optimization/route의 정보 부족이다.

##### lowering capability 공통 형태

basis node는 implementation을 내장하지 않는다. registry가 다음 형태의 후보를 제공한다.

~~~text
ExecutionBasisLoweringCapability
  basis/op family
  applicability predicate over:
    ResolvedInstantiation
    ResolvedCallFacts
    ValueFacts / ArrayPropertyFacts
    RepresentationFacts
    TargetCapability
  semantic guarantees:
    exact numeric/error/order behavior
    supported representation
    supported access/alias form
  realization family id
  optional ResourceEstimate model
~~~

candidate legality와 candidate preference를 분리한다.

~~~text
legal(candidate, facts, target)
        ≠
preferred(candidate, CostProfile)
~~~

library가 존재하거나 specialized kernel이라는 이유만으로 자동 선택하지 않는다.

##### execution basis별 최소 node contract

| Basis family | 최소 logical payload | 최소 facts / SemanticCheck | 최소 lowering capability family |
|---|---|---|---|
| IndexSpace / Generate | output shape/domain, index-to-value payload | shape extents nonnegative/representable, dtype/result shape | scalar loop, SIMD/SIMT index generation, constant/iota intrinsic, fused producer |
| Elementwise / MapN | scalar/cell payload, input access maps, output domain | dtype/promotion/domain, J agreement already resolved or wrapped by CellApply, error-order policy | scalar, SIMD, SIMT, fused elementwise, target intrinsic |
| CellApply | operand function graph, frame/cell split, valence, assembly policy | EffectiveCellRank, AgreementFact, RepetitionFact, FillAndEmptySemantics, AssemblyFact, length/domain/error order | GenericCellLoop, CanAbsorbCellApply, uniform MapCells/flat GPU route, batched structured op |
| RegularReindex / StaticReindex | output shape and result-index → source-index relation, fill/boundary policy | shape/item-count relation, index-map validity, view equivalence witness | metadata/view, consumer index-map fusion, layout absorption, materializing copy |
| Gather | source, index value, output domain, indexed axes | IndexVector role, bounds/rank checks, duplicate-read harmlessness, source representation | scalar/indirect gather, vector gather, GPU indexed read, fused consumer |
| Scatter / Amend | source/destination value, indices, updates, collision/order policy | bounds, alias/destination legality, duplicate-index ordering, J error order | out-of-place update, legal in-place update, GPU scatter, sorted/index-grouped route |
| ScatterCombine | destination/index/update plus combine op | combine algebraic facts, collision semantics, identity if required, bounds | atomic combine, privatized histogram, sort+segment reduce, sequential exact-order fallback |
| WindowView | source, output shape, window shape, stride/dilation/padding/fill, index relation | WindowSpec/Stride/Dilation/Padding roles, result shape, boundary/fill checks | virtual/index-only view, composed reindex, tiled/shared-local window, fused stencil/consumer |
| SegmentView | source plus offsets/lengths/flags descriptor and segment order | SegmentDescriptor validity, monotonic/range/coverage facts as applicable | virtual segmented view, offset/CSR-style traversal, segmented map/reduce/scan, materialized segment fallback |
| Permute | source plus permutation or permutation mapping | Permutation/Unique/KnownRange facts, validity check | view when representable, gather permutation, in-place cycle algorithm, out-of-place permutation |
| Reduce | reducer, axes, identity/empty policy, order/reassociation contract | reduced axes, dtype/accumulator, associativity/commutativity only when proven, FillAndEmptySemantics, MayRaise | serial ordered, SIMD tree, thread/tree, warp/subgroup, multi-stage, library reduction |
| Scan | operator, axis, direction, inclusive/exclusive/J prefix policy, identity if applicable | same numeric/algebraic facts as reduction plus exact prefix order contract | serial scan, parallel prefix, segmented scan when combined with SegmentView |
| Contract | input access maps, parallel axes, contraction axes, combine op, aggregate op | shape/axis compatibility, accumulator/numeric policy, reducer algebraic facts, empty contract | generic nested reduction, tiled CPU, GEMM/microkernel, tensor instruction, external library |
| Concat / Assemble | ordered inputs/cell results, assembly axis/policy | shape agreement, dtype promotion, heterogeneous/boxed assembly, FillAndEmptySemantics | virtual concat where legal, direct destination writes, memcpy/copy chain, producer-to-destination fusion |
| Replicate / Compact / Expand | source, counts/mask/positions, output-order policy | CountVector/boolean mask, nonnegative/integral counts, output-size overflow, stable order contract | serial expand, prefix-sum + scatter, GPU compaction, fused consumer |
| Grade | input, direction/order comparator contract, output permutation semantics | ordering/tolerance/fit, dtype/comparator legality, tie/order semantics | comparison sort, radix/key sort, small-array network, GPU sort, library route |
| Lookup / Classify | haystack/domain, query/items, mode(first/last/member/interval/self-classify), equality/order contract | equality/tolerance, sortedness/uniqueness/range facts when available | linear probe, hash table, direct-address/bitset, binary search, sort/merge lookup |
| GroupBy | keys, payload reference, group-order contract, descriptor result | equality/tolerance, group ordering semantics, key range/sortedness if known | hash grouping, sort grouping, direct bucket, reduce-by-index/atomic route, sequential stable fallback |
| NestedTraverse | boxed/nested source, traversal selector/path/level, visit function, reconstruction policy | nesting/box facts, path/level validity, heterogeneous assembly and errors | recursive/reference traversal, flattened leaves + offsets/segments, level-specialized traversal |
| LinearSolve | A/B operands, problem kind, transpose/least-squares/inverse semantic contract | rank/shape compatibility, numeric dtype, singularity/rank-deficiency/error semantics | generic reference solver, LU/QR/SVD family, dense CPU/GPU library, sparse solver route |
| StateMachine | transition table, input classifier, initial state, emission/output policy | table shape/type, state/input-class bounds, output assembly, error semantics | scalar sequential reference, table-specialized loop, transition composition/vector route when proven legal |

##### execution basis 간 composition rule

basis가 커졌다고 해서 nested structure를 즉시 평탄화하지 않는다.

~~~text
CellApply(Reduce(...))
WindowView → Contract
GroupBy → SegmentView → Reduce
NestedTraverse → CellApply(...)
~~~

는 유효한 Logical IR이다. optimizer는 다음 proof가 있을 때만 더 큰 realization으로 흡수한다.

- CellApply absorption / uniform assembly
- index-map composition
- reduction reassociation legality
- grouped-update collision semantics
- numeric/error-order equivalence
- representation compatibility

반대로 named high-level op를 basis graph로 열었더라도 optimizer가 library/tensor/solver route를 위해 original semantic identity를 이용할 수 있어야 한다.

##### A3-v0 / v1 구현 범위

basis vocabulary 전체를 A3-v0의 선행조건으로 만들지 않는다.

~~~text
A3-v0 required executable core
  Elementwise
  CellApply
  Reduce
  RegularReindex/StaticReindex
  IndexSpace/Generate
  SemanticCheck
  + Opaque/Unknown access fallback

A3-v1 priority expansion
  Scan
  Gather / Scatter
  WindowView
  SegmentView
  Contract
  Concat / Assemble
  Replicate / Compact / Expand

later / workload-driven
  Grade
  Lookup / Classify
  GroupBy
  NestedTraverse
  LinearSolve
  StateMachine
  specialized ExecutionBasisExpansion rules
~~~

later로 둔 operation도 language semantics를 later까지 금지한다는 뜻이 아니다. 해당 basis lowering이 없으면 기존 native/runtime semantic path 또는 structured-op fallback이 correctness를 담당한다.

##### 현재 A3 transition implementation seam

2026-10-01 현재 기존 src/analysis.rs::LogicalPlan은 compatibility/inspection plan으로 유지하고, 별도 src/logical_ir.rs에 A3-v0 single-block IR migration seam을 추가했다.

transition plan 쪽:
- Node.basis: 한 call을 추가 graph expansion 없이 직접 분류하되, 단일 enum이 아니라 outer→inner `ExecutionBasis.layers`를 기록한다. 예: ranked reduction은 `[CellApply, Reduce]`를 보존한다.
- ResolvedInstantiation: target/valence/input-output dtype·rank/requested rank boundary를 기록한다.
- ValueRoleFacts: ShapeVector, IndexVector, CountVector, AxisPermutation 등 문맥상 role을 noun type과 분리한다.

A3 logical_ir 쪽:
- Operation과 SSA ValueData를 분리하여 zero-result operation을 표현할 수 있다.
- SemanticCheck는 실제 zero-result ordered op이며 PrefixAgreement, CellFrameAgreement, IndicesInBounds constraint를 우선 지원한다.
- ConstraintSet과 FactWitness로 static proof가 있는 check와 unresolved check를 구분한다.
- basis node는 family-specific ExecutionBasisPayload와 IterationDomain/axis role을 가진다. 현재 A3의 `OpKind::Basis.kind`는 routing을 위한 outer execution basis이고, `CallOp.execution_basis`가 전체 outer→inner composition을 보존한다.
- EffectSummary와 SpeculationSemantics는 conservative PrimitiveContract에서 초기화되며 이후 proof-driven refinement가 가능하다.
- src/lowering.rs의 ExecutionBasisLoweringCapability registry는 legality만 판정하며 cost/preference와 분리된다.
- native candidate가 없으면 RuntimeSemanticFallback으로 분류하며 invalid J로 취급하지 않는다.
- src/expansion.rs는 원 semantic op를 지우지 않는 optional multi-node ExecutionBasisExpansion sidecar를 제공한다.
- 첫 실제 expansion은 dyadic E. 이며 J Dictionary의 x E. y ↔ ($x) x&-: ;.3 y identity를 근거로 WindowView → CellApply(Match) graph를 제공한다.
  - reference: https://www.jsoftware.com/help/dictionary/decapdot.htm

아직 구현하지 않은 핵심:
- multi-block CFG와 branch/loop/exceptional control-flow (single Function/Region/Block/Return container는 구현됨)
- richer AccessRelation/AxisSemantics payload와 cell-rank가 완성된 ResolvedInstantiation
- general ConstraintSet lattice / runtime Guard
- SemanticCheck discharge/refinement witness가 SpeculationSemantics refinement로 이어지는 proof pass
- WindowView/SegmentView/Contract 등 v1 basis의 executable lowering capability
- CostProfile / preference ranking
- optimized native lowering과 reference executor 사이의 broad semantic equivalence corpus
- environment/state를 받는 general A3 executor

현재 correctness oracle로 src/logical_executor.rs의 closed-plan reference executor를 추가했다. 이 경로는 기존 semantic kernels를 재사용하며 physical schedule/bufferization을 하지 않는다. A3의 operation order, zero-result SemanticCheck, SSA wiring, basis payload가 기존 runtime과 같은 결과/error class를 내는지 검증하는 용도다.

기존 transition Node에는 zero-result SemanticCheck를 역이식하지 않는다. 새로운 A3 op/result-separated IR에서만 표현한다.

##### basis contract 검증

각 basis op에는 최소 세 종류의 test가 필요하다.

1. verifier negative tests
   - 잘못된 axis/index/shape/role/descriptor를 reject
2. semantic/reference equivalence tests
   - generic/reference path와 optimized lowering의 result/error equivalence
3. composition tests
   - CellApply+Reduce, Window+Contract, GroupBy+Reduce, sparse representation 조합처럼 실제 fusion boundary를 포함

J error가 있는 case는 값만 비교하지 않고 **error class와 observable ordering**까지 비교한다.




#### 4.24.15 J syntax-derived Structural Opportunity IR: 문법을 optimization information source로 사용한다

과거 JAXA의 핵심 주장은 “J를 컴파일한다”보다 더 강했다.

> **J의 combinator syntax와 derived semantics가 계산 graph의 topology를 정적으로 드러내므로, compiler가 fusion·parallelism·materialization·lifetime 기회를 일반 DAG pattern matching보다 일찍 발견할 수 있다.**

RustJ는 이 정보를 generic Logical DAG로 펼친 뒤 다시 복원하지 않는다. J Graph IR이 FunctionEntity의 J 구조를 실제 noun application과 결합해 **GraphForm/GraphHint**를 만들고, Execution lowering이 이를 concrete execution ValueId에 투영한 `StructuralOpportunity`를 만든다.

이 원칙은 basis 분석과 다른 축이다.

~~~text
J Semantic Construction IR
          ↓
      J Graph IR
          ├→ graph algebra / rewrite analysis
          └→ GraphForm / GraphHint
                    ↓
           Execution lowering
                    ├→ Basis analysis
                    └→ StructuralOpportunity projection
                              │
                              ▼
                    Logical optimization substrate
                              │
                    semantic legality / proof
                              │
                    target/resource feasibility
                              │
                    schedule / physical plan
~~~

Basis는 “무슨 배열 연산인가”를 설명한다.

~~~text
+/      → Reduce
{       → Gather
;.3     → WindowView
u . v   → Contract
~~~

Structural opportunity는 “연산들이 어떤 topology로 연결되었는가”를 설명한다.

~~~text
@:          → Pipeline
hook/fork   → Branch / Join
adjoint/VJP → Parallel Fan-out
^:          → Iteration topology
"           → Cell-level parallel application topology
~~~

두 정보는 합쳐져야 한다.

~~~text
operation kind × graph topology × semantic facts
~~~

##### JAXA에서 계승하는 첫 세 topology

**1. @: — Pipeline fusion opportunity**

~~~j
f @: g @: h
~~~

J 결합 규칙만으로 실행 흐름을 다음처럼 알 수 있다.

~~~text
h → g → f
~~~

따라서 Semantic Analyzer는 일반 DAG optimizer가 나중에 producer-consumer chain을 재발견하기 전에 다음 정보를 기록한다.

~~~text
PipelineOpportunity
  source = Atop
  inputs
  ordered stage results
  intermediate values that may avoid materialization
  source span / semantic provenance
~~~

이 record는 “반드시 하나의 kernel로 fuse한다”는 뜻이 아니다.

**2. hook/fork — Branch/Join fusion + retained-value opportunity**

monadic hook:

~~~j
(+ F) y
~~~

~~~text
y ─────────────────┐
                   +
F(y) ───────────────┘
~~~

fork:

~~~j
f g h
~~~

~~~text
        f(y) ──┐
y ─────┤       g → result
        h(y) ──┘
~~~

따라서 analyzer는 다음을 알고 있다.

~~~text
BranchJoinOpportunity
  shared_inputs
  branch_results
  join_result
  values live across branch computation
  J observable branch evaluation order
  source = Hook | Fork
~~~

과거 JAXA 문서의 “input y를 register에 유지”는 현행 RustJ Logical IR에서는 더 일반적으로 다음처럼 해석한다.

> **shared/live-across value를 fusion region 안에서 materialize하지 않고 join까지 유지할 가능성이 있다.**

register/shared memory/reload/recompute/global materialization 중 무엇을 쓸지는 Physical Planner가 결정한다.

**3. adjoint/VJP — Parallel fan-out opportunity**

Flow–Storage 연구의 backward 그림:

~~~text
dy + saved values
       │
       ├─ data adjoint       → dx → previous layer
       ├─ parameter adjoint  → grad_W → emit/accumulate
       └─ parameter adjoint  → grad_b → emit/accumulate
~~~

은 data dependency가 없는 여러 branch를 만들 수 있다. 따라서 향후 adjoint/VJP expansion은 다음 record를 생성한다.

~~~text
ParallelFanOutOpportunity
  inputs
  branch_results
  continuing flow value
  emitted parameter-adjoint values
  source = AdjointVjp
~~~

이 또한 실제 concurrent kernel 실행을 의미하지 않는다. effect/accumulation ordering과 target bandwidth/resource analysis가 뒤따른다.

##### opportunity와 legality를 분리한다

StructuralOpportunity는 **후보 발견**이다.

~~~text
J syntax / derived semantics
        ↓
StructuralOpportunity
        ↓
SemanticFusion/ParallelLegality
        ↓
TargetFusion/ParallelFeasibility
        ↓
Cost/Schedule decision
        ↓
Physical fusion / parallel execution
~~~

semantic legality는 다음을 본다.

- EffectSummary
- SpeculationSemantics
- J error ordering
- ConstraintSet / FactWitness
- numeric semantics
- rank/cell/frame assembly
- access/dependency relation
- alias/destination relation
- representation compatibility

target/resource feasibility는 그 뒤에 다음을 본다.

- register pressure / live-value pressure
- shared/local memory
- synchronization
- subgroup/warp/SIMD constraints
- memory bandwidth and coalescing
- launch overhead
- library/tensor-core realization constraints
- tiling feasibility

따라서 하드웨어 정보는 Semantic IR에 넣지 않지만 **fusion region을 확정하기 전에는 반드시 들어온다**.

~~~text
early:
  "여기는 fusion/parallel opportunity다"

middle:
  "J semantics상 합법하다"

later:
  "이 target에서 실제로 realizable/profitable하다"

commit:
  chosen FusionRegion / ParallelSchedule
~~~

##### Flow–Storage와의 연결

StructuralOpportunity는 4.24 Flow–Storage의 storage 결정을 앞당겨 확정하지 않는다.

Pipeline 내부 intermediate:

~~~text
Logical ArrayValue
  → materialization elision candidate
~~~

Hook/Fork의 live-across value:

~~~text
Logical ArrayValue
  → retain/reload/recompute/materialize choice
~~~

Adjoint parameter branch:

~~~text
Logical ArrayValue
  → continuing flow가 아니라 semantic emit/accumulate destination을 가질 수 있음
~~~

Physical planner가 다음 중 하나를 고른다.

~~~text
KeepVirtual
FuseAway
RegisterResident
WorkingStateResident
Recompute
Bufferize
ExternalResource
~~~

즉 **syntax가 lifetime/topology 힌트를 주고, Flow–Storage가 semantic obligation을 설명하며, hardware planner가 실제 storage를 선택한다.**

##### 현재 구현 seam

2026-10-01 현재 코드에는 첫 migration seam을 추가했다.

- ConjunctionId::Atop (@:)을 frontend registry에 추가했다.
- FunctionEntity는 @: derived verb를 PrimitiveConjunction(Atop) + 두 function operand로 보존한다.
- src/opportunity.rs에 target-independent StructuralOpportunity<V>를 추가했다.
- Pipeline, BranchJoin, ParallelFanOut topology를 정의했다.
- analysis::call_entity()가 Atop/Hook/Fork를 실제 call DAG로 펼칠 때 **동시에 opportunity record를 생성**한다.
- @: chain은 recursive semantic tree를 execution-order pipeline으로 flatten하여 하나의 Pipeline record로 남긴다.
- Hook/Fork는 J observable evaluation order를 유지한 branch result list와 live-across/shared input을 기록한다.
- transition LogicalPlan과 A3 Logical IR 둘 다 opportunity sidecar를 운반하고 verifier가 value/span 무결성을 검사한다.
- `src/j_graph_ir.rs`는 적용된 J graph를 독립 compiler IR로 제공하며 `GraphForm`과 `GraphHint`를 node 자체에 기록한다. 현재 `Pipeline/Hook/Fork/Reduce/Rank`가 first-class form이다.
- execution `analysis::lower_graph()`는 이제 `BoundProgram`을 직접 해석하지 않고 J Graph IR을 소비한다. `Node.j_origin`이 graph node와 execution op의 many-to-one provenance를 보존한다.
- `Engine::analyze_compilation()`은 `j_graph`와 `execution` 두 IR을 함께 반환한다.
- ParallelFanOut은 type/schema만 먼저 정의했으며 실제 adjoint/VJP pass가 생길 때 producer를 연결한다.
- 현재 opportunity extraction은 call site에서 직접 보이는 semantic function graph를 기준으로 한다. name-bound derived verb를 interprocedurally 열어 topology를 전파하는 summary/cache는 아직 없으며, 이후 SpecializationEngine/binding-version analysis와 연결한다.

이 방식은 source semantics를 generic DAG로 펼치는 것과 J syntax가 제공한 optimization information을 보존하는 것을 동시에 만족한다.

##### 중요한 불변조건

1. StructuralOpportunity는 optimization hint보다 강한 **semantic-topology provenance**지만, physical schedule은 아니다.
2. Opportunity가 존재한다고 fusion/parallelization legality가 자동 성립하지 않는다.
3. J observable evaluation order는 opportunity 안에서도 잃지 않는다. 병렬 실행은 별도 proof가 있어야 한다.
4. basis normalization 때문에 Atop/Hook/Fork provenance를 버리지 않는다.
5. generic DAG pattern matching은 추가 기회를 찾는 보조 수단일 수 있지만, J 문법이 이미 준 topology를 다시 찾는 주 경로가 되어서는 안 된다.
6. hardware resource 정보는 opportunity discovery에 필요하지 않지만, 실제 fusion/parallel region 확정에는 필요하다.
7. 이 층의 목적은 J 문법의 표현을 미학적으로 보존하는 것이 아니라 **optimizer search space를 줄이고 materialization/lifetime/parallel 후보를 일찍 제공하는 것**이다.


### 4.25 과거 custom primitive inventory는 후보 목록으로 보존한다

`JAXA-complier`의 마지막 prototype registry는 source-level 품사까지 가지고 있었다.

**Parameterized adverb 후보**

```text
conv
depthwise_conv
linear
bn
ln
adam
cp
avgpool2d
maxpool2d
dropout
```

**Verb 후보**

```text
flatten
relu
gelu
softmax
scaled_dot_product_attn
crossentropy
```

이 목록과 당시 rank field를 그대로 RustJ의 확정 vocabulary/contract로 간주하지 않는다. **역사적 candidate inventory**다. 중요한 것은 source-level J 품사를 보존하고, parameterized adverb가 실제 parameter noun을 받은 뒤 derived computational contract를 생성한다는 구조다.

새 RustJ registry에 들어가려면 최소한 part of speech/valence, innate rank, parameter schema, shape/dtype/numeric rule, iteration domain, axis semantics, access relations, dependency/effect/alias contract, semantic reference 또는 충분한 semantic specification, conservative execution/lowering path, 필요한 realization/resource requirement model을 갖춰야 한다.

초기 구현 우선순위는 가장 작은 end-to-end 검증이 가능한 `relu`, `linear`, `conv2d`, `flatten/static-reindex`, `avgpool2d` 또는 단순 reduction으로 둔다. attention, optimizer, checkpoint/training-specific extension은 core registry 구조가 검증된 뒤 단계적으로 옮긴다.

---

## 5. Logical Plan, Physical Plan, Executor

### 5.1 Logical Array IR / Logical Execution Plan

이 계층은 Semantic Analyzer가 hook/fork/derived verb/rank 같은 고수준 의미 구조를 분석한 뒤 만든 **명시적 배열 dataflow**다.

여기서 `Reduce`, `CellApply`, `StaticReindex` 같은 이름은 **Logical IR에서 처음 등장하는 normalized operation**이다. J Semantic IR의 parser-produced function graph에는 이 이름으로 modifier application을 대체하지 않는다. `MapCells`는 `CellApply`의 uniform result/assembly 조건이 증명된 뒤 사용할 수 있는 더 제한적인 lowering form이다.

```text
Semantic IR
  / : Verb
  └─ + : Verb

       ↓ Semantic Analyzer

Logical IR
  Reduce(reducer=+)
```

마찬가지로:

```text
Semantic IR
  " : Verb                 // derived result POS
  head = PrimitiveConjunction(Rank)
  ├─ left:  u
  └─ right: r

       ↓ Semantic Analyzer

Logical facts / op
  resolved RankSpec
  frame/cell mapping
  CellApply
    └─ optional later MapCells-style lowering when UniformProven
```

현재 코드의 `Callable.reduce` / `Callable.rank`는 기존 analyzer/runtime와 연결하기 위한 **migration field**다. 최종 A3-v0 Logical IR에서는 parser-derived operator graph를 해석한 결과를 normalized logical operation/fact로 표현하고, 이 bool/array shortcut을 semantic identity로 사용하지 않는다.

예를 들어 고수준의

```text
Apply(
  Fork(
    AdverbApplication(operator=/, operand=+),
    %,
    #
  ),
  y
)
```

는 분석 후 개념적으로 다음과 같은 dataflow가 될 수 있다.

```text
            Input y
           /       \
  Reduce(Add)      Tally
           \       /
             Divide
```

여기서 일반 dataflow 실행 의미는 fork 표기 없이도 표현할 수 있지만, **원래 source가 fork/hook/@:였다는 topology provenance는 optimization 정보로 계속 보존한다.** 4.24.15의 StructuralOpportunity sidecar가 pipeline/branch-join/live-across 정보를 명시적으로 운반하므로 optimizer가 generic DAG에서 이를 다시 pattern-match할 필요가 없다. 진단·debug provenance이기도 하지만 그것에 한정되지 않는다.

Logical Plan에서 보존할 정보:

- SSA ValueId와 region/block/control-flow 구조
- normalized array operation
- dtype/shape/rank/cell/frame facts
- iteration domain / axis semantics / access relation
- symbolic constraints와 witness/guard
- data dependency와 effect ordering token
- effect / alias / speculation facts
- invariance / semantic-mask facts
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



여기서 `Logical Optimizer`는 target-independent canonicalization/DCE/CSE와 semantic-preserving graph rewrites를 담당한다. 특정 tile/layout/device/resource를 선택하거나 target cost로 후보를 확정하는 일은 Schedule / Transform Plan 이후의 책임이다.

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

## 6. 논리 배열과 물리 배열 — 핵심 architecture decision

### 6.1 논리 J noun, boxed, sparse와 verb

dense noun의 extensional value는 기본적으로 다음으로 본다.

```text
DenseJArray
  atom type
  shape
  ordered atoms / logical value
```

그러나 current jsource와의 semantic compatibility를 위해 “모든 noun = type + shape + flat atoms뿐”이라고 고정하지 않는다.

**boxed**는 physical encoding이 아니라 J의 semantic atom/type 구조다.

```text
BoxedJArray
  shape
  ordered boxed atoms
    each atom -> J value
```

실제 backend가 box를 pointer, handle, arena index 등으로 표현하는 것은 physical 문제다.

**sparse**도 단순한 backend compression format이 아니다. J의 `$.`와 sparse type/operations가 sparse representation을 관찰하며, sparse axes와 sparse element(fill)가 의미에 참여한다.

```text
SparseJArray
  logical atom type
  shape
  sparse_axes
  sparse_element
  sparse index/value semantics
```

dense와 sparse가 같은 extensional mathematical array를 나타낼 수 있어도 J 프로그램이 sparse representation을 관찰할 수 있으므로 semantic representation class를 보존한다.

따라서 noun semantic model은 개념적으로:

```text
JNoun
  value/type/shape semantics
  semantic representation:
    Dense
    Boxed
    Sparse(SparseSemantics)
    other J-visible noun kinds as implemented
```

이다.

verb는 noun array를 입력받아 noun array를 반환하는 array transformer이며, J Semantic Array IR에서 first-class semantic entity로 표현한다. verb의 hook/fork/train/modifier composition은 semantic analysis 전에 보존한다.

다음은 논리 J noun의 semantic identity가 아니다.

- stride
- offset
- physical tile layout
- CPU/GPU device
- byte alignment
- sharding
- CUDA block/thread
- concrete sparse backend format(CSR/COO 등)

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

#### 6.4.1 현재 구현 상태와 완료 조건

Logical/Physical Array 분리는 **아키텍처 결정으로는 확정**됐지만 runtime representation까지 완전히 이행된 상태는 아니다.

현재 상태:

- [x] Logical execution `ValueId`와 physical `BufferId`를 별도 identity로 둔다.
- [x] `PhysicalArray`가 buffer/stride/offset을 소유하고 logical IR에는 이 정보를 넣지 않는다.
- [x] 같은 logical atom order를 서로 다른 physical stride/offset/backing으로 표현할 수 있다는 회귀 테스트를 둔다.
- [x] GraphFacts는 physical stride/layout/device를 소유하지 않는다.
- [ ] runtime `Value`의 dense payload가 아직 `CpuStorage`를 직접 포함한다. 이는 전환기 구현이며 최종 Logical Array abstraction으로 간주하지 않는다.
- [ ] dense logical value와 CPU/GPU backend storage 사이의 explicit representation adapter/handle 경계를 완성한다.
- [x] `facts::LayoutFact`를 `RepresentationClassFact`로 이름 변경하고 `Facts.layout`도 `Facts.representation_class`로 바꿨다. `Dense / AxisSparse`는 J-visible representation class이며 physical layout이 아님을 API 이름에서 명시한다.

완료 기준은 **logical value를 정의하거나 분석하는 데 `CpuStorage`, stride, offset, device, BufferId가 필요하지 않고**, 선택된 backend representation을 통해서만 그런 정보가 등장하는 상태다.

따라서 현재 `Value { shape, Data::Int(CpuStorage<_>), ... }` 구조는 semantic boundary의 최종형이 아니라 migration bridge다.

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

### 7.4 semantic representation과 physical encoding을 구분한다

모든 physical 표현을 affine byte-stride 모델 하나로 강제하지 않는다. 동시에 J-visible representation과 backend storage encoding도 섞지 않는다.

**Semantic/J-visible**

- dense noun
- boxed noun
- sparse noun + sparse axes/element semantics

**Physical/backend encoding**

- affine dense
- tiled
- packed bit
- device/backend-specific encoding
- sparse noun을 위한 concrete sparse format(COO/CSR/other)
- boxed noun을 위한 pointer/handle/arena representation

즉 `SparseJArray`를 GPU에서 dense buffer로 임시 materialize할 수는 있어도, 그 때문에 J-visible sparse identity/metadata를 잃어서는 안 된다. 반대로 같은 sparse semantics를 여러 physical sparse format으로 실현할 수 있다.

초기 G1은 read-only affine dense physical representation만 다룬다.

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
  AdverbApplication(operator=/, operand=+),
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

- primitive/type별 overflow retry와 coherent result promotion semantics
- J가 정의한 error precedence, suppression, retry behavior
- active `try./catch./catcht.`에 의한 J-visible error/throw control transfer
- binding/name-reference의 late lookup semantics
- side effect 순서
- comparison tolerance와 `!.` fit semantics
- empty/rank fill-cell 결과 type·shape semantics
- rank/modifier result-cell assembly(type/shape join, framing fill, assembly error) semantics
- sparse/boxed의 J-visible representation semantics
- primitive/derived-verb가 요구하는 floating numeric contract(허용된 reassociation, compensated/exact mode, tolerance 등)

FMA, reassociation, tree/vector reduction은 **무조건 금지하지도, 무조건 허용하지도 않는다.** `NumericSemantics`/`FitSemantics`가 허용한 경우에만 적용한다. GPU 병렬 오류 수집도 arbitrary first-lane error를 그대로 노출하지 않고 J의 observable error contract를 따른다.

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

- Enqueue는 ordinary name의 noun/verb/adverb/conjunction 품사를 최종 확정하지 않는다.
- Parser가 name을 사용할 때 current local/locale binding을 lookup하여 실제 value/type class를 얻는다.
- noun name은 현재 value로 resolve되는 반면, 일반 verb/adverb/conjunction name은 jsource의 `name~` reference와 같은 late lookup semantics가 필요할 수 있다.
- undefined non-by-value function name은 즉시 value error가 아니라 nameref 형태로 남을 수 있는 jsource 경로가 있으므로, undefined name을 전부 frontend 즉시 오류로 만들지 않는다.
- extension name도 이 규칙의 예외가 아니다.
- static binding/version proof가 있을 때만 NameRef를 stable primitive/builder identity로 specialize한다.
- `f.` 같은 J의 fix semantics는 late name reference를 실제 value로 고정하는 별도 의미이므로 일반 compilation specialization과 혼동하지 않는다.
- nameref는 생성 시 기대한 part of speech를 보존하고, 실행 시 current lookup 결과의 품사가 달라지면 J처럼 domain error가 되어야 한다.
- name/version 정보를 IR과 plan guard에 반영해야 한다.
- 한 sentence의 모든 name을 문장 시작 시점 environment로 일괄 resolve하지 않는다. 우측→좌측 evaluation/assignment가 만든 namespace mutation 시점을 보존한다.
- `=.`/`=:`는 binding을 갱신하면서 assigned J entity(noun/verb/adverb/conjunction 등)도 반환하므로 statement-only IR로 축소하지 않는다.
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

### M — 현재 구조 수렴 실행 순서

이 상위 체크리스트는 **지금 어떤 순서로 구조를 수렴시킬지**를 추적한다. 세부 완료 조건은 아래 A/F/P/G 체크리스트를 그대로 사용하며, 같은 일을 중복 정의하지 않는다.

핵심 목표는 새 계층을 더 만드는 것이 아니라 현재 공존하는 과도기 경계를 제거해 다음 canonical pipeline으로 수렴하는 것이다.

```text
J source
  → jsource-compatible frontend
  → FunctionEntity / J Semantic Construction IR
  → J Graph IR / Graph Basis
  → Execution Semantic Lowering
  → canonical logical_ir::Plan / Execution Basis
  → Route Partition
  → Schedule / Transform
  → Physical Planner / Bufferization
  → Physical Execution Plan
  → Executor
```

Logical J value와 physical representation의 분리는 이 전체 pipeline에 교차하는 불변식으로 유지한다.

#### M0 — 구조 기준선 고정

- [x] 목표 compiler stage와 각 stage의 책임을 문서에서 확정했다.
- [x] Graph Basis와 Execution Basis를 별도 계층으로 분리했다.
- [x] Logical Array/J noun과 Physical Array/representation을 별도 계층으로 분리했다.
- [x] 현재 코드의 중복 execution IR 경계(`analysis::LogicalPlan` → `logical_ir::Plan`)를 구조 부채로 식별했다.
- [x] `physical.rs`가 아직 Physical Planner가 아니라 read-only CPU affine representation foundation임을 명시했다.
- [x] `runtime.rs`의 `ResolvedVerb { reduce, rank, ... }` flattening은 과도기 runtime 구현이며 canonical semantic/compiler model이 아님을 확인했다.
- [x] ArrayFire와 `math_arrayfire`를 semantic oracle이 아니라 execution/fusion/adapter 참고 구현으로 배치했다.
- [x] compiler module ownership/dependency 표를 코드 구조와 맞춰 확정하고 reverse dependency 금지선을 문서화했다.

현재/목표 module ownership은 다음을 기준으로 한다.

| 책임 | 현재 주요 모듈 | canonical owner / 목표 | 금지되는 역방향 의존 |
|---|---|---|---|
| word formation | `scanner.rs` | frontend word former | graph/logical/physical/runtime가 scanner 구현 세부에 의존하지 않음 |
| enqueue/word interpretation | `enqueuer.rs` | frontend enqueuer | target/backend 정보를 enqueue가 읽지 않음 |
| parser + semantic construction | `semantic.rs` | frontend/parser + immutable `FunctionEntity` | Graph/Logical/Physical 선택을 parser가 소유하지 않음 |
| J graph algebra | `j_graph_ir.rs`, `j_graph_rewrite.rs`, `j_graph_resource.rs`, `j_graph_memory.rs` | J Graph IR / Graph Analyzer | Logical/Physical plan을 다시 semantic identity로 역주입하지 않음 |
| execution semantic contract | `execution_semantics.rs` | 독립 target-independent execution-semantics contract | schedule/buffer/device를 포함하지 않음 |
| canonical Logical Execution IR | `logical_ir.rs` | `logical_ir::Plan` | `physical.rs`, kernel/runtime concrete storage에 의존하지 않음 |
| compilation aggregate | `compilation.rs` | cross-stage analysis bundle | lowering semantics 자체를 소유하지 않음 |
| route legality/capability | `lowering.rs` | verified Logical IR 이후 lowering/route layer | semantic/parser를 target 편의에 맞게 변경하지 않음 |
| schedule/transform | 아직 없음 | 별도 planner-side representation | canonical Logical IR을 destructive하게 schedule-specific IR로 덮지 않음 |
| physical representation | `physical.rs`, `storage.rs` | representation layer | semantic facts를 physical layout으로 정의하지 않음 |
| Physical Plan/bufferization | 아직 없음 | Physical Planner | J parser/FunctionEntity를 직접 해석하지 않음 |
| backend kernels | `kernels.rs`, `numeric.rs`, `simd.rs` 등 | backend realization | kernel 구현 세부가 semantic legality를 정의하지 않음 |
| interpreter/reference runtime | `runtime.rs`, `logical_executor.rs` | transitional/reference execution | compiler canonical IR의 의미를 runtime flattening으로 정의하지 않음 |

M1 transition container는 제거되었다. `analysis`의 remaining re-export는 target-independent execution-semantic contract와 compilation aggregate compatibility surface이며 canonical Logical IR container를 소유하지 않는다.

M0 이후 적용할 dependency 방향:

```text
frontend
  ↓
semantic FunctionEntity
  ↓
J Graph IR
  ↓
execution semantic contracts
  ↓
logical_ir::Plan
  ↓
route / schedule
  ↓
physical plan / representation
  ↓
backend / executor
```

옆 단계의 provenance/type identity 참조는 허용하지만, **아래 단계의 concrete realization 정보가 위 단계의 semantic identity를 결정하는 dependency는 금지**한다.

**M0 완료 조건:** **완료.** 새 구현이 어느 stage에 속하는지 한 곳으로 결정할 수 있고, 과도기 compatibility bridge를 새 canonical interface로 오인하지 않는다.

#### M1 — canonical Logical Execution IR로 cutover

목표: `logical_ir::Plan`을 유일한 canonical execution IR로 만들고 crate-private `transition_ir::LogicalPlan` compatibility 계층(현재 `analysis`에서 re-export)을 제거한다.

- [x] `ExecutionBasisKind`, `ExecutionBasis`, `AccessFact`, `AccessRelation`, `CallTarget/Callable`, `ResolvedInstantiation`, symbol/scope처럼 계속 필요한 target-independent execution-semantic contract를 `execution_semantics.rs`로 분리했다. `analysis` re-export는 compatibility만 담당한다.
- [x] J Graph IR execution-semantic lowering 중 canonical `logical_ir::Plan`을 incremental하게 직접 생성하는 경로를 만들었다. legacy transition plan은 같은 lowering record에서 compatibility output으로만 병행 생성한다.
  - [x] legacy container 타입을 `analysis.rs`에서 crate-private `transition_ir.rs`로 격리해 direct lowerer가 제거해야 할 seam을 한 모듈로 축소했다.
  - [x] A3 iteration-domain/constraint helper가 `transition::Node` 전체가 아니라 `Facts/RankPlan/ExecutionBasis/ResolvedInstantiation`을 받도록 분리했다.
  - [x] graph-node lowering builder가 incremental projector를 직접 호출해 A3 op/value/check를 생성하도록 전환했다.
- [x] 완성된 transition plan을 다시 읽는 `Plan::from_transition` pass를 제거하고, fact/check/order/provenance 생성을 incremental `TransitionProjection`으로 옮겼다.
  - [x] 모든 외부 consumer를 `CompilationAnalysis.logical`로 전환했다.
  - [x] graph lowering builder가 A3 op/value/check를 직접 생성하도록 전환했고 `Plan::from_transition`을 삭제했다.
- [x] `Engine::analyze_a3`와 `analyze_diagnostic`을 canonical Logical IR API로 전환했다. `Engine::analyze`만 M1 compatibility transition API로 명시해 남겨 두었다.
- [x] `CompilationAnalysis`는 `j_graph + rewrite candidates/resource evaluation + canonical logical plan`을 묶는 analysis result로 재정의했다.
  - [x] canonical A3 `logical: logical_ir::Plan`을 추가하고 `Engine::analyze_a3`, route/expansion test consumers를 이 필드로 전환했다.
  - [x] compatibility `transition` field와 `transition_ir` module을 제거했다.
- [x] `analysis::LogicalPlan`, 구형 `analysis::ValueId/Node/Write` 및 transition IR container를 제거했다.
- [x] A3 verifier/reference executor/lowering tests는 `Engine::analyze_a3`와 `logical_ir::Plan` direct-lowering 경로를 사용한다.
- [x] direct A3 regression test로 Graph origin/source span, name version, semantic check 선행 순서와 write ordering을 고정했다.
- [x] `analysis.rs` dependency audit를 완료했다. canonical IR container/transition 책임은 없고 `CompilationAnalysis` 및 execution-semantic compatibility re-export만 남아 있다.

**M1 완료 조건:** `J Graph IR → logical_ir::Plan`이 직접 연결되고 `analysis::LogicalPlan`이 코드에서 사라진다.

#### M2 — jsource-compatible frontend/parser cutover

목표: 현재 heuristic parser를 jsource-compatible Word Formation → Enqueue → 9-row Parser pipeline으로 교체한다.

- [ ] F0 differential 0-mismatch 기록을 완료한다.
- [ ] F1 Enqueuer/PrimitiveResolver를 완료한다.
- [ ] F2 Parse Queue를 완료한다.
- [ ] P1 parser control class와 semantic entity/value를 분리한다.
- [ ] P2 하나의 9-row reduction engine으로 전환한다.
- [ ] P3 modifier/Hook/Fork/bident/trident construction semantics를 연결한다.
- [ ] P4 parser-time name resolution/assignment sequencing을 연결한다.
- [ ] P5 construction semantics와 compiler facts 경계를 완료한다.
- [ ] P6 differential/conformance gate를 통과한다.
- [ ] P7 legacy parser heuristic(`reduce_modifier_applications`, `collapse_verb_trains` 중심 경로)을 제거한다.
- [ ] P8에서 canonical FunctionEntity → J Graph IR → Logical IR handoff를 재검증한다.

**M2 완료 조건:** supported frontend domain의 parser reduction을 jsource row/semantic action으로 설명할 수 있고 compiler가 별도 언어 문법을 갖지 않는다.

#### M3 — Logical/Physical Array 경계의 코드 수렴

목표: logical value identity에 CPU/GPU/layout identity가 역류하지 않게 하고 representation 선택을 physical planning으로 이동한다.

- [x] `LayoutFact`를 `RepresentationClassFact`로, `Facts.layout`을 `Facts.representation_class`로 바꿔 physical layout과 구분했다.
- [ ] Dense/Boxed/Sparse처럼 J-visible representation semantics와 row-major/column-major/stride/tile/device 같은 physical representation을 타입/API에서도 구분한다.
- [ ] 현재 `Value::Data`의 dense `CpuStorage` 직접 소유를 migration artifact로 한정하고, canonical compiler value identity가 CPU backing을 요구하지 않게 한다.
- [ ] sparse의 J-visible axes/fill/semantic representation과 concrete coordinate/value buffer encoding의 경계를 점검한다.
- [ ] `PhysicalArray`는 BufferId/lease/shape mapping/stride/offset/encoding 같은 representation-only 책임만 갖게 유지한다.
- [ ] 같은 logical value의 복수 physical representation과 여러 logical value의 safe buffer reuse를 표현할 planner-side identity를 정의한다.
- [ ] G2 structural view 작업을 이 경계 위에서 구현한다.

**M3 완료 조건:** Logical IR/semantic facts에는 stride/offset/device/buffer가 없고, physical representation 변경이 J value identity를 바꾸지 않는다.

#### M4 — 최소 RustJ-native CPU vertical slice

목표: optimizer가 똑똑하지 않아도 canonical compiler pipeline이 end-to-end로 실제 실행되게 한다.

- [ ] Logical payload와 분리된 최소 `Schedule/TransformPlan`을 정의한다.
- [ ] 첫 planner는 비용 최적화 없이 deterministic all-CPU policy를 사용한다.
- [ ] 최소 Physical Plan op를 `Bind/View/Materialize/Kernel/Return` 수준으로 정의한다.
- [ ] logical ValueId → physical representation/BufferId binding을 구현한다.
- [ ] G2 transpose/reverse/slice/compatible reshape/zero-stride agreement view를 planner에서 선택 가능하게 한다.
- [ ] G3의 첫 kernel로 contiguous/fixed/general-stride add를 연결한다.
- [ ] G3 cell mapping과 ExecutionBasis `CellApply`를 physical view iteration에 연결한다.
- [ ] G4 CPU Physical Executor를 구현한다.
- [ ] `source → FunctionEntity → J Graph → logical_ir::Plan → Physical Plan → CPU Executor` vertical test를 만든다.
- [ ] 기존 semantic/reference executor와 결과/error contract를 비교한다.

**M4 완료 조건:** 기존 interpreter 직접 실행을 거치지 않는 최소 compiler-native CPU 경로가 하나 이상 동작한다.

#### M5 — Route/Schedule/Cost 확장

M4 이후에만 optimizer 선택 문제를 키운다.

- [ ] `RouteRegion`에 boundary inputs/outputs, chosen route, preconditions/witnesses, semantic provenance를 추가한다.
- [ ] legality와 profitability를 계속 분리한다.
- [ ] StructuralOpportunity/use-def를 schedule candidate와 연결한다.
- [ ] materialize/view/fuse 후보의 physical feasibility를 계산한다.
- [ ] TargetProfile/ResourceEstimate/CostEstimate 최소 schema를 구현한다.
- [ ] cold compile, warm execution, copy, layout conversion, transfer, synchronization 비용을 분리한다.
- [ ] simple CPU cost model로 multiple legal realization 중 하나를 선택한다.

**M5 완료 조건:** planner가 단순 고정 policy가 아니라 여러 합법 physical plan 중 cost/resource 근거로 선택할 수 있다.

#### M6 — External/ArrayFire/GPU route

M4의 compiler-native vertical slice와 M5의 route contract가 안정된 뒤 진행한다.

- [ ] Graph/Execution Basis ↔ ArrayFire capability matrix를 만든다.
- [ ] ArrayFire route의 dtype/rank/shape/layout/J-semantic precondition을 명시한다.
- [ ] J row-major ↔ ArrayFire column-major mismatch를 view/copy/consumer-absorption 선택 문제로 physical planner에 연결한다.
- [ ] external handle lifetime/lock/release/sync를 Physical Plan resource/token 경계로 모델링한다.
- [ ] MLIR adapter와 StableHLO-safe subset adapter의 공통 negotiation interface를 정의한다.
- [ ] external route failure가 J semantic failure가 아니라 route unsupported/fallback으로 처리되는 테스트를 만든다.
- [ ] 실제 GPU storage/kernel은 별도 요청과 검증 가능한 환경이 있을 때 재개한다.

**M6 완료 조건:** external library/backend가 J semantics를 정의하지 않고, verified Logical IR의 합법적인 realization route 중 하나로만 동작한다.

### A0 — 문서/아키텍처 경계

### A0 — 문서/아키텍처 경계

- [x] RustJ 내부 compiler stage의 논리적 경계를 확정한다.
- [x] Semantic Analyzer 입력 전에 hook/fork/train/derived verb/rank를 제거하지 않는 원칙을 확정한다.
- [x] `J Semantic Array IR`과 `Logical Array IR / Plan`을 구분한다.
- [x] generic boundary 후보를 semantic analysis 이후의 Logical Array IR로 이동한다.
- [x] 문서를 `PROJECT.ko.md`로 통합한다.
- [ ] 실제 코드 dependency에서도 source frontend → J Semantic Array IR → Semantic Analyzer/Lowering → Logical Plan 경계를 만든다.

### A0.5 — jsource-compatible parser 이행 체크리스트

이 절의 **F0–F2 + P0–P7이 frontend/parser migration의 authoritative checklist**다. F0은 word formation, F1은 enqueue/primitive resolution, F2는 parse-queue skeleton을 담당한다. 그 뒤 P 단계에서 parser semantic construction/name-resolution/cutover를 완성한다. P8은 A1/A2/A3로 넘기는 integration handoff다.

> **운영 원칙 (2026-10-01 확정):** Tokenizer/word formation, Enqueuer, Parser는 RustJ 고유 frontend 문법을 새로 설계하지 않고 current jsource의 observable frontend semantics를 충실히 이식한다. representation은 Rust-native여도 되지만 word boundary, enqueue classification/lookup timing, parser row eligibility/order, modifier construction boundary/result POS/error semantics는 jsource가 기준이다. Parser가 만든 completed `FunctionEntity` DAG가 canonical semantic source이며 downstream이 이를 `reduce/rank` 같은 축약 필드로 대체해서는 안 된다.
>
> **현재 실행 순서:** F0 differential 0-mismatch 기록 → F1 Enqueuer 분리 → F2 parse queue → P1 stack/value model → P2 9-row engine → P3 construction semantics → P4 name/assignment sequencing → P5 semantic/compiler fact 분리 → P6 conformance gate → P7 legacy parser 제거. 각 단계는 아래 완료 조건을 만족한 경우에만 완료로 체크한다.

검토 기준은 2026-09-30의 `jsoftware/jsource` master(`13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`)이다. 특히 다음을 함께 oracle로 본다.

- `jsrc/p.c::cases[]`: 9-row J parsing rule의 선언형 기준. 현재 runtime parser가 직접 순회하는 테이블은 아니고 tacit translator에도 사용된다.
- `jsrc/p.c` runtime parser의 `ptcol`/bit-mask dispatch: 같은 row eligibility/order를 실제 parser hot path에서 구현한다.
- `jsrc/cf.c`의 Hook/Fork 및 bident/trident dispatch.
- 각 ADV/CONJ constructor(`jtslash`, `jtqq` 등): modifier application 시 J-defined construction validation과 실제 result POS를 결정한다.

RustJ가 그대로 맞춰야 하는 것은 **word/class resolution timing, row eligibility와 precedence, reduction extent, result parser class/POS, construction-time J errors, assignment/parenthesis/name-resolution semantics**다. C의 bit packing, refcount, in-place bookkeeping, cached function pointer, `localuse` 최적화는 이식 대상이 아니다.

RustJ는 compiler이지만 jsource parser가 실행과 분리된 정적 AST parser가 아니라는 점을 보존해야 한다. rows 0–2의 verb application은 **parser-visible effect/value dependency가 없다는 것이 증명된 경우에만** Noun-producing semantic application으로 defer할 수 있다. 그 실행이 이후 name/locale lookup, assignment state, modifier operand value, result POS 또는 construction-time error에 영향을 줄 수 있으면 정적 parser가 효과를 무시한 채 진행해서는 안 된다. v0 correctness baseline은 동일한 9-row engine의 runtime semantic action/fallback을 사용하고, 이후 guard/multiversion으로 정적 범위를 넓힌다. rows 3–4 역시 modifier application 시점에 필요한 J construction semantics를 수행하여 completed entity/POS/error를 결정해야 한다.

#### F0 — jsource word formation 이식

- [x] `w.c::state`의 character-class × state transition table을 Rust enum/table로 **직접 이식**한다. `src/scanner.rs::TRANSITIONS`가 SS..SDDD 16개 state와 CX/CDD/CDDZ/CU/CS/CA/CN/CB/C9/CD/CC/CQ transition을 명시적으로 보존한다.
- [x] 기존 handwritten `scanner::transition`을 제거하고 lookup-only `TRANSITIONS[state][class]`로 교체했다. follow-on numeric rewind와 UNDD 처리는 `jtwordil`의 별도 boundary action으로 유지한다.
- [x] numeric follow-on, quoted literal, `NB.`, `NB..`/`NB.:`, `{{`/`}}`, inflection word boundary를 differential corpus로 만든다. `tools/word_conformance.py`가 state-prefix × 256-byte sweep, 특수 사례, seeded random 2,000건을 포함한다.
- [x] unmatched quote를 jsource `EVOPENQ`에 대응하는 `open quote`로 분류하고 시작 quote byte span을 보존한다.
- [x] word/parser/runtime source span은 byte offset으로 보존하고 사용자 진단 시 Unicode line/column으로 변환한다.
- [x] `;:` 기반 word-formation oracle과 RustJ raw `wordil`-equivalent spans를 byte 단위로 비교하는 adapter/probe를 만든다 (`tools/word_conformance.py`, `examples/scan_words.rs`). trailing `NB.` raw field와 parser-visible count는 별도 contract로 유지한다.
- [x] pinned jsource `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528` against 6,618-case differential에서 **failed=0 / zero_mismatch=true**를 확인했다 (seed `20260927`, open-quote cases `1109`).

**F0 완료 조건:** supported source domain에서 word boundaries/comment cutoff/error가 pinned jsource `jtwordil`과 일치한다.

#### F1 — jsource enqueue + PrimitiveResolver 이식

- [x] `syntax::lex_spanned`가 수행하던 word interpretation을 `enqueuer::enqueue`로 이동하고 `syntax`는 legacy Token adapter로 축소했다.
- [x] `EnqueuedWord { class, payload, span, word_index, flags }`와 `EnqueueClass`/`EnqueueFlags`를 정의해 parser-facing class/payload/provenance를 명시적으로 분리했다.
- [ ] enqueue의 classification order와 parser class/POS 결정 순서를 `jtenqueue`와 동일하게 유지한다. RustJ convenience lexer가 먼저 품사를 확정하지 않게 한다.
- [ ] core J primitive lookup을 jsource `spellin -> ds`와 같은 위치와 precedence로 구현한다.
- [x] `PrimitiveResolver`/`PrimitiveContext`를 만들고 enqueue용 core primitive resolution과 parser/name-binding용 extension lookup을 분리했다.
- [x] `PrimitiveHandle { semantic_id, source_origin, result_pos, semantic_info, lowering_key }`를 정의했다.
- [ ] compile 시작 시 `PrimitiveContext`와 `TargetContext`를 함께 확정하되 enqueuer는 semantic primitive resolution에 `PrimitiveContext`만 사용한다.
- [ ] built-in과 extension 모두 동일 `lowering_key -> active TargetContext` lookup protocol을 사용하게 한다.
- [x] extension-like spelling은 enabled extension catalog에 있어도 enqueue에서는 ordinary NAME + lookup metadata로 진행한다.
- [ ] numeric/string construction, name validation, assignment/copula classification을 jsource `jtenqueue` 순서대로 이식한다.
- [x] ordinary NAME은 처음 non-lookup으로 두고, trailing NAME/뒤에 non-assignment가 오는 NAME만 lookup으로 전환하며 copula 직전 assignment target NAME은 non-lookup으로 유지한다.
- [x] `EnqueueFlags`에 `global_assignment/local_assignment/assignment_to_name`을 분리했다. 현재 지원 copula `=:`는 global이며 NAME 직후 copula는 to-name flag를 보존한다.
- [x] one-word sentence는 Noun/Name/Verb/Adverb/Conjunction만 결과 가능 class로 허용하고 copula/괄호 단독 문장을 enqueue 단계에서 거부한다.
- [ ] jsource sentence-word refcount/inplacing flags와 special in-place sentence rewrites는 optimization-only로 명시적으로 제외한다.
- [x] parser-time NAME lookup이 extension binding의 Verb/Adverb/Conjunction POS를 얻은 뒤 core와 같은 modifier/parser class 경로에 참여하는 테스트를 만들었다.

**F1 완료 조건:** parser가 raw spelling을 다시 해석하지 않고 `EnqueuedWord` queue만으로 core/extension primitive, name lookup, assignment semantics를 결정할 수 있으며 hardware implementation 선택은 아직 일어나지 않는다.

#### F2 — jsource parse queue skeleton

- [x] semantic parser 입력을 legacy `Token`에서 `EnqueuedWord` queue로 바꾸고 span/word-index/flags를 parser 진입까지 보존한다.
- [x] `EnqueueClass`와 `EnqueuedPayload`를 분리하고 parser가 동일 `EnqueuedWord` carrier에서 둘을 함께 운반한다.
- [ ] jsource Mark/Edge sentinel을 명시적으로 표현한다.
- [x] ordinary lookup NAME은 `EnqueueFlags.lookup_name`을 확인한 뒤 parser item 생성 전에 `ParserNameBinding`으로 resolve한다.
- [x] pinned `cases[]`를 옮긴 `match_parse_row([ParseClass; 4])`가 semantic payload를 보지 않고 parser class만으로 eligibility와 first-match precedence를 결정한다.
- [ ] matcher row ordering/reduction extent/result reinsertion을 `cases[]`/runtime `ptcol` behavior와 동일하게 구현하고 별도 train/modifier heuristic을 제거한다.
- [ ] reduction result를 동일 queue/stack representation으로 재삽입한다.
- [x] `ParseClass`를 F2 row matcher와 application/modifier reduction의 공통 class domain으로 사용한다.

**F2 완료 조건:** parser는 jsource-compatible enqueue queue를 유일한 입력으로 받아 9-row engine으로 넘길 수 있다.

#### P0 — 기준선과 differential oracle 고정

- [x] jsource 9-row parsing rule(row 0–8)의 eligibility와 precedence를 `cases[]`와 runtime dispatch 양쪽에서 확인한다.
- [x] ordinary non-assignment NAME은 stack class matching 전에 lookup되며, noun은 value로, 일반 verb/adverb/conjunction은 현재 POS를 가진 nameref/value semantics로 들어감을 확인한다.
- [x] rows 3–4의 modifier action 결과 `yy`의 실제 `AT(yy)`가 다음 parser class가 됨을 확인한다.
- [x] modifier application 결과가 하나의 completed J entity로 stack에 재삽입된 뒤 후속 reduction에 참여함을 확인한다.
- [x] `+/ % #`에서 `+/`가 하나의 derived VERB entity로 만들어진 뒤 Fork의 `f` operand가 됨을 확인한다.
- [x] 현재 RustJ의 `reduce_modifier_applications -> collapse_verb_trains -> noun/verb application` 구조가 jsource table dispatch를 근사하는 과도기 구현임을 확인한다.
- [x] parser와 compiler-analysis 책임 경계를 고정한다.
- [x] differential oracle의 최소 contract를 정한다:
  - 성공/실패 및 J error class,
  - deterministic noun result,
  - assignment 후 name class(`4!:0`),
  - constructed function/modifier의 atomic/linear representation(`5!:1`, `5!:5`)이 유용한 경우,
  - RustJ 내부에서는 row id/input classes/span/result class를 기록하는 optional ParseTrace.
- [x] 위 contract를 실제 test harness API로 만든다. `tools/oracle.py` JSON-lines protocol이 `eval`, `sentence`, `name_class`, `representation(atomic|linear)`을 제공하며 기존 string eval 요청과 호환된다.

**P0 완료 조건:** **완료.** observable contract를 사용하는 oracle API가 존재하고, CI/reference build는 parser 검토 기준 jsource revision `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`을 고정한다.

#### P1 — parser stack model과 semantic value model 분리

- [x] parser class/control(`EnqueueClass`/`ParseClass`)과 semantic payload/entity(`EnqueuedPayload`/`ParseValue`/`FunctionEntity`)를 별도 타입으로 분리했다.
- [x] `ParseClass`가 `Noun | Verb | Adverb | Conjunction | Name | Assignment | LParen | RParen | Mark`를 명시적으로 표현한다.
- [x] ordinary lookup NAME은 stack item 생성 전에 `ParserNameBinding`으로 현재 noun/function POS를 resolve하며, unresolved ordinary name만 jsource식 late verb nameref로 남긴다.
- [x] Noun과 `FunctionEntity`는 별도 representation을 유지하면서 `Item/ParseClass`에서 동일한 parser class interface로 참여한다.
- [x] source primitive, named modifier, completed derived entity가 모두 `FunctionEntity.result_pos -> ParseClass` 경로로 parser에 참여한다.
- [x] `FunctionEntity.result_pos`를 completed function entity의 parser POS 근거로 사용한다.
- [x] enqueue lookup/to-name flags와 parser assignment path로 assignment-target NAME을 ordinary semantic `NameRef`와 분리했다.
- [x] `EnqueuedWord`의 byte span/word index를 parser 진입까지 보존하고 completed Expr/FunctionEntity reduction span 및 진단 blame provenance로 전달한다.

**P1 완료 조건:** parser control state와 J semantic entity가 분리되어 있으며 ordinary names, assignment names, nouns, verbs/modifiers를 jsource class rules대로 stack에 올릴 수 있다.

#### P2 — 하나의 9-row reduction engine으로 전환

- **jsource invariant:** conjunction chain은 parse-table 구조상 left-to-right로 결합한다. 예: `u @: v @: w`의 semantic graph는 `(u @: v) @: w`이다.
- [x] row 0 `EDGE VERB NOUN ANY`를 right-to-left stack reducer에서 first-match precedence로 선택하고 monadic `Expr` application으로 defer한다.
- [x] row 1 `EDGE+AVN VERB VERB NOUN`의 정확한 four-class eligibility/reduction extent를 production stack reducer에 구현했다.
- [x] row 2 `EDGE+AVN NOUN VERB NOUN`을 production stack reducer에서 선택하고 dyadic `Expr` application으로 defer한다.
- [ ] row 3 `EDGE+AVN (VERB|NOUN) ADV ANY`를 modifier semantic constructor 호출로 구현한다.
- [ ] row 4 `EDGE+AVN (VERB|NOUN) CONJ (VERB|NOUN)`를 modifier semantic constructor 호출로 구현한다.
- [ ] row 5 `EDGE+AVN (VERB|NOUN) VERB VERB`의 Fork construction을 구현한다.
- [ ] row 6 `EDGE CAVN CAVN ANY`를 Hook/bident/trident semantic dispatch로 구현한다.
- [ ] row 7 `(NAME|NOUN) ASGN CAVN ANY` assignment reduction과 effect/result semantics를 구현한다.
- [x] row 8 `LPAR CAVN RPAR ANY`를 production stack action으로 구현하고 recursive parenthesis parser를 제거했다. grouped noun은 `ExprKind::Group`/depth를, grouped function은 semantic identity를 유지한 채 parser provenance span을 괄호 전체로 보존한다.
- [ ] 각 reduction 결과를 같은 parser stack에 되돌리고 다시 **동일한 row matcher**로 scan/reduce한다.
- [ ] row action abstraction이 `ReadyParseValue`와 `RequiresRuntimeSemanticParse`를 구분할 수 있게 하여, 정적 compiler path가 parser-visible runtime dependency를 숨기지 않게 한다.
- [ ] runtime semantic fallback도 별도 grammar/parser를 만들지 않고 동일한 9-row matcher를 사용하게 한다.
- [x] 기존 flat-vector modifier/train/application reducer를 삭제하고 production expression reduction을 right-to-left stack + ordered `match_parse_row`로 cutover했다. 아직 미구현 semantic form은 해당 row action에서 명시적으로 남긴다.
- [ ] one-word sentence의 별도 jsource path와 관찰 가능한 결과가 동일하도록 테스트한다.

**P2 완료 조건:** 모든 parser reduction 선택을 jsource row 번호와 input class 조합으로 설명할 수 있고, deferred semantic action과 runtime semantic action이 동일한 parser engine을 공유한다. parser-visible effect/value dependency를 무시한 정적 진행 경로가 없다.

#### P3 — modifier/Hook/Fork/bident/trident construction semantics

- [ ] rows 3–4에서 result POS를 RustJ가 임의로 고정하지 않고 **modifier semantic constructor가 반환한 실제 POS**를 다음 parser class로 사용한다.
- [ ] row 3의 `VERB ADV`와 `NOUN ADV`를 각 adverb의 J construction semantics에 따라 처리한다.
- [ ] row 4의 `(VERB|NOUN) CONJ (VERB|NOUN)` 전체 parser form을 각 conjunction의 J construction semantics에 따라 처리한다.
- [ ] `cf.c::bidents[]`를 단순 result-POS 표가 아니라 `SyntaxError | ImmediateSemanticApply | BuildDerivedModifier(result_pos)`의 semantic disposition으로 옮긴다.
- [ ] `cf.c::tridents[]`를 `SyntaxError | ImmediateSemanticApply | BuildFork | BuildDerivedModifier(result_pos)`의 semantic disposition으로 옮긴다.
- [ ] VV Hook과 NVV/VVV Fork의 construction boundary를 jsource와 동일하게 만든다.
- [ ] 긴 train은 별도 `LongTrain` algorithm이 아니라 row 5/6 반복 reduction의 결과로만 형성한다.
- [ ] modifier application마다 completed entity 하나를 만들고 후속 reduction은 그 entity ref만 보게 한다.
- [ ] parser semantic operand와 jsource execution auxiliary(`fgh` helper slot, `localuse`, cached executor)를 구분한다.

**P3 완료 조건:** derived Verb/Adverb/Conjunction과 immediate semantic application의 구분까지 jsource parser construction behavior와 일치한다.

#### P4 — parser-time name resolution과 assignment sequencing

- [ ] enqueue 단계는 ordinary NAME과 lookup metadata를 전달하고 extension 이름을 keyword로 만들지 않는다.
- [ ] parser가 ordinary name을 stack에 넣기 직전에 현재 local/locale binding을 조회해 noun/verb/adverb/conjunction class를 얻는다.
- [ ] noun name의 by-value resolution과 일반 function/modifier name의 nameref semantics를 구분한다.
- [ ] jsource의 nameless modifier by-value 최적화는 언어 semantics와 분리하고 RustJ에서 필수로 복제하지 않는다.
- [ ] named Verb/Adverb/Conjunction이 primitive entity와 동일한 row 0–6 경로에 참여하게 한다.
- [ ] sentence 시작 시 전체 binding snapshot을 만들지 않고 observable right-to-left lookup/assignment sequencing을 보존한다.
- [ ] assignment-target NAME은 ordinary name lookup과 별도 취급한다.
- [ ] `=.` / `=:`의 symbol-table 선택과 assignment result semantics를 테스트한다.
- [ ] current POS를 가진 nameref가 later resolution 시 다른 POS로 바뀐 경우의 J-compatible error contract를 보존한다.
- [ ] extension builder(`conv` 등)의 shadow/rebind도 ordinary J name semantics를 따르게 한다.
- [ ] row 0–2 실행이 같은 sentence의 이후 parser-time name/locale/POS lookup에 영향을 주는 사례를 식별하고 parser-visible effect로 분류한다.
- [ ] deferred noun value가 뒤 row 3/4 modifier construction의 실제 operand value로 필요한 경우 정적 placeholder로 construction을 완료하지 않는다.
- [ ] v0에서는 이러한 dynamic parse dependency를 `RuntimeSemanticParse`/coverage fallback으로 보내고, 정적 compile 성공으로 오인하지 않는다.
- [ ] 추후 guard/multiversion을 추가하더라도 observable reduction/order/error semantics가 runtime semantic baseline과 같음을 요구한다.

**P4 완료 조건:** parser 결과가 spelling이 아니라 그 시점의 J binding, assignment state, parse row에 의해 결정된다.

#### P5 — construction-time J semantics와 compiler-analysis facts 분리

**구현 방향:** parser/J Semantic Construction IR의 `FunctionEntity` 자체를 compiler convenience를 위해 변형하지 않는다. 대신 그 immutable graph에서 **별도 J Graph IR을 파생**하여 applied topology와 syntax-derived optimization hint를 표현한다. J Graph IR은 graph analysis에 필요한 조기 abstract `GraphFacts(dtype/shape/rank)`를 가질 수 있지만, 이것은 실행 계약의 canonical resolved fact가 아니다. actual valence/effective-rank/cell/frame/agreement/check/effect/error/access/representation을 포함한 최종 call-time 사실은 Logical Execution IR의 `ResolvedCallFacts`/`ResolvedInstantiation`이 소유한다. lowering은 두 층이 공유하는 dtype/shape/rank가 일치하는지 검증한다. target 이후 정보는 `LoweringCapability/TargetFacts -> PhysicalDecision`에 둔다.

parser에서 **모든 의미 해석을 제거하지 않는다.** jsource modifier application이 그 자리에서 검증하고 result entity를 만드는 의미는 그대로 수행한다. 제거 대상은 target/call-dependent compiler facts다.

- [ ] pure/semantic `ModifierSemanticConstructor` interface를 두어 parser row 3/4가 J-defined construction validation과 result POS/entity 생성을 요청하게 한다. operand value가 runtime-dependent하면 같은 action을 runtime semantic parser에서 수행할 수 있어야 한다.
- [ ] parser-produced `FunctionEntity`는 identity/result POS/source operands/span과 **intrinsic `FunctionSemanticInfo`**를 immutable하게 소유한다.
- [ ] modifier construction facts는 해당 completed `FunctionEntity.semantic_info.construction`에 보존하여 재귀 traversal에서 바로 참조할 수 있게 한다.
- [ ] `"` constructor는 jsource `jtqq`와 동일한 noun/verb operand legality, rank/length/domain validation 및 requested-rank normalization을 **modifier application 시점의 construction semantics**로 처리한다. 이는 반드시 compile-time이라는 뜻은 아니며 runtime parser fallback에서도 같은 규칙을 사용한다.
- [ ] normalized requested rank 같은 node-intrinsic construction fact와 **actual argument rank를 이용한 effective rank/cell/frame 계산**을 분리한다. 후자는 call/Logical IR node의 `ResolvedCallFacts`에 둔다.
- [x] applied `/`는 completed derived entity로 만들고 `Verb.reduce` 같은 compiler migration boolean을 semantic identity에서 제거했다.
- [x] `Verb.reduce` 사용처를 제거했다. reduction identity/basis/fact inference는 completed `FunctionEntity`의 Insert 구조에서 Semantic Analyzer/Lowering이 유도한다.
- [ ] `Verb.rank` 필드/사용처 제거는 완료했다. 남은 일은 requested-rank를 construction fact로 정규화하고 call-time `ResolvedRankContract/CellApply`와 명시적으로 분리하는 것이다.
- [x] `Callable.reduce` / `Callable.rank` migration field를 제거했다. Analyzer는 shared `FunctionEntity` 구조를 직접 따라 execution basis/facts/outer rank boundary를 유도한다.
- [ ] innate rank, effective rank, frame/cell split, agreement/repetition, access, optimizer proofs가 parser entity 필드에 들어가지 않게 한다.
- [ ] architecture/device/lowering/cost metadata가 parser/`FunctionEntity.semantic_info`에 들어가지 않게 한다.

**P5 완료 조건:** parser는 J entity construction의 성공/실패와 completed semantic identity를 정확히 결정하지만, call-dependent 및 target-dependent compiler facts는 소유하지 않는다.

#### P6 — differential/conformance test matrix

- [ ] 9개 parse row 각각의 최소 positive sentence를 jsource와 differential 비교한다.
- [ ] row precedence가 충돌할 수 있는 competing-pattern 문장을 추가한다.
- [ ] `+/ % #`, 2-verb Hook, 3/4/5개 이상 train을 구조 golden으로 비교한다.
- [ ] 연속 adverb/conjunction 및 derived modifier가 다시 modifier operand가 되는 사례를 추가한다.
- [ ] modifier constructor의 construction-time rank/length/domain error를 jsource와 비교한다.
- [ ] Verb/Adverb/Conjunction을 name에 할당한 뒤 사용하는 사례를 추가한다.
- [ ] sentence 중간 assignment/name lookup이 뒤 reduction의 class에 영향을 주는 사례를 추가한다.
- [ ] row 0–2의 effectful 실행이 왼쪽의 name/locale/POS resolution을 바꾸는 문장을 differential corpus에 포함한다.
- [ ] runtime-dependent noun이 modifier operand가 되어 construction success/error/POS가 runtime에 결정되는 사례를 포함한다.
- [ ] parentheses가 reduction boundary를 바꾸는 사례를 추가한다.
- [ ] one-word sentence path를 별도 regression으로 둔다.
- [ ] deterministic noun result와 J error class를 비교한다.
- [ ] assigned entity의 POS는 `4!:0`, 유용한 derived structure는 `5!:1`/ `5!:5`를 oracle로 비교한다.
- [ ] RustJ ParseTrace로 row id/input classes/span/result class를 golden화하되 jsource 내부 bitmask/주소/flags와 비교하지 않는다.
- [ ] 아직 lowering하지 못하는 합법 J form의 **parser success**와 이후 `UnsupportedImplementation`을 syntax error와 구분한다.
- [ ] 현재 지원 범위의 differential suite를 CI 필수 gate로 만든다.

**P6 완료 조건:** 지원 parser surface의 변경은 jsource observable differential + RustJ row trace golden 없이 merge되지 않는다.

#### P7 — cutover와 legacy parser 제거

- [ ] 새 9-row engine이 기존 parser/semantic golden을 모두 통과한다.
- [ ] Analyzer golden(`(+/ % #) y` 포함)이 새 parser output에서도 동일한 completed entity graph를 입력으로 받는다.
- [ ] old `reduce_modifier_applications`를 제거한다.
- [ ] old `collapse_verb_trains`를 제거한다.
- [ ] noun/verb application을 수동으로 조립하던 legacy loop를 제거한다.
- [ ] parser-only migration fields와 dead compatibility code를 제거한다.
- [ ] `parse`, `parse_analysis`, `parse_runtime`가 parser semantics를 하나의 engine에서 공유하게 한다.
- [ ] parser 전환 후 전체 test/CI를 통과시킨다.

**P7 완료 조건:** RustJ의 J parser semantics를 정의하는 코드 경로가 하나이며, 그 경로는 jsource-compatible class resolution + 9-row reduction + semantic constructor contract를 따른다.

#### P8 — A1/A2/A3로의 integration handoff

P8은 **parser migration 선행 게이트가 아니다.** P0–P7에서 얻은 clean parser output을 기존 compiler milestones가 소비하도록 연결하는 후속 작업이다.

- [ ] PrimitiveSpec/ExtensionSpec의 target-independent semantic contract와 parser entity identity를 연결한다.
- [x] P8에서 J Graph IR을 parser output과 execution IR 사이의 명시적 compiler boundary로 추가한다.
- [x] `GraphForm/GraphHint`에서 J syntax-derived topology를 기록하고 execution lowering이 이를 소비하도록 한다.
- [ ] `GraphRuleRefs`를 PrimitiveSpec/derived-composition rule registry와 연결한다.
- [ ] `FunctionEntity.semantic_info`의 intrinsic facts와 Logical IR node의 `ResolvedCallFacts` 책임을 분리한다.
- [ ] `ResolvedCallFacts`에서 valence/effective-rank/cell/frame/agreement/repetition/type/shape/effect/error/access를 계산한다.
- [ ] `+/ % #` matrix golden에서 최종 `%`의 implicit CellApply2를 명시적으로 만든다.
- [ ] Logical Optimizer가 semantic graph를 보존한 채 `FusionCandidate::Mean` 등을 별도 proof/candidate로 만든다.
- [ ] built-in과 NN/array extension op가 동일한 LogicalOp/lowering interface로 진입하게 한다.
- [ ] architecture-specific 구현 정보는 `LoweringRegistry × ArchitectureTarget`에서만 결합한다.
- [ ] concrete GPU model 정보는 `DeviceProfile`, 후보 선택 성능 정보는 `CostProfile/RuntimeProfile`로 분리한다.

**P8 완료 조건:** parser를 다시 변경하지 않고 A1/A2/A3의 semantic analysis, CPU/GPU lowering, NN extension을 확장할 수 있다. dynamic J parsing이 필요한 form은 compiler coverage와 runtime semantic fallback의 명시적 경계로 남는다.

#### 진행 규칙

- F0–F2 + P0–P7을 frontend/parser migration의 **single source of truth**로 사용한다. P8은 기존 A1/A2/A3 체크리스트와 함께 추적한다.
- 이후 frontend 작업의 진행상황 보고는 반드시 이 체크리스트의 phase/item 기준으로 보고한다. 새 작업이 생기면 임시 TODO를 코드에만 남기지 않고 먼저 해당 phase에 checklist item으로 추가한다.
- jsource와의 차이를 발견하면 "RustJ 구현 편의"로 봉합하지 않고 해당 phase의 compatibility defect로 기록한다. observable semantics가 같다는 differential proof가 있기 전에는 의도적 차이로 간주하지 않는다.
- frontend 구현은 원칙적으로 F0 → F1 → F2 → P1 → P2 → P3 → P4 → P5 → P6 → P7 순으로 진행하되, 앞 phase interface를 깨지 않는 oracle/test 작업은 병행할 수 있다.
- 완료 즉시 같은 변경에서 `[ ] -> [x]`로 갱신한다.
- 부분 구현을 완료로 표시하지 않는다. 각 phase의 완료 조건을 만족해야 phase 완료로 본다.
- jsource와 의도적으로 다른 observable parser behavior가 필요해지면 구현 전에 rationale과 semantic impact를 이 문서에 기록한다.
- downstream IR 요구사항은 P8/A1/A2/A3에 추가하고 parser entity에 임시 compiler field로 밀어 넣지 않는다.


### A0.6 — Structured diagnostic context

- [x] J-compatible machine error와 diagnostic provenance/context를 분리한다.
- [x] `ErrorContext`에 phase/span/original word index/current name/operation/valence/argument summary/notes를 표현할 수 있게 한다.
- [x] inner context 우선 merge 규칙을 만들어 outer stage가 더 정확한 span/blame/context를 덮어쓰지 않게 한다.
- [x] `DiagnosticAnalyzer`와 Python-style renderer를 분리한다.
- [x] enqueue word에 original word index를 보존하고 parser diagnostic으로 전달하기 시작한다.
- [x] runtime monad/dyad failure에 executing primitive, valence, x/y type/shape/rank summary를 붙인다.
- [x] interpreter execution과 compiler `analyze_diagnostic`이 동일 context/error infrastructure를 사용한다.
- [x] stable `eval()/analyze()/parse()` machine API는 context wrapper를 제거하고 기존 J error variant/kind를 유지한다.
- [x] F1 `EnqueuedWord`가 byte span과 original word index를 canonical provenance로 직접 소유한다.
- [ ] F2 parser stack entry가 original word index를 모든 reduction 동안 보존하고 jsource `infererrtok`에 대응하는 blame inference를 구현한다.
- [ ] Hook/Fork/derived modifier 실행 시 failing semantic entity의 compact linear representation을 diagnostic context에 넣는다.
- [ ] rank/agreement failure analyzer가 effective cell/frame facts를 사용해 J처럼 어느 frame/shape가 불일치하는지 구조적으로 설명한다.
- [ ] index error analyzer가 offending selector/index/path를 작은 structured detail로 보존하고 설명한다.
- [ ] domain error analyzer가 primitive contract와 argument dtype/value summary를 이용해 구체적 원인을 설명한다.
- [ ] assembly error analyzer가 cell-result type/shape join failure 위치를 설명한다.
- [ ] lowering/backend failure도 Logical IR source origin + semantic operation context로 동일 renderer에 연결한다.
- [ ] diagnostic context가 큰 noun payload를 소유/복사하지 않는지 테스트한다.
- [ ] 최신 J error corpus의 대표 사례를 RustJ diagnostic golden으로 추가하되 문구 자체보다 semantic information completeness를 검증한다.

**완료 조건:** AOT/interpreter/JIT/backend 어느 경로에서 실패해도 J error class는 안정적으로 유지되고, 동일한 structured context → analyzer → renderer 경로로 source 위치와 semantic 원인을 설명할 수 있다.

### A1 — J Semantic Construction IR / FunctionEntity 경계

- [ ] 현재 `semantic.rs`가 noun/verb/adverb/conjunction과 derived composition을 얼마나 보존하는지 감사한다.
- [x] jsource `p.c::cases[]`의 parser function-construction rows를 기준으로 immutable shared `FunctionEntity` graph를 만든다.
- [x] `/` ADV와 `"` CONJ를 source operator identity로 보존하고 parser application 결과의 parent로 사용한다. modifier별 `Insert/Rank/...` semantic node enum을 만들지 않는다.
- [x] 큰 derived function/train의 `Arc<FunctionEntity>` sharing test로 subtree deep-copy가 없음을 검증한다.
- [x] semantic FunctionEntity와 runtime/backend executor specialization의 층을 분리한다.
- [ ] primitive verb identity와 monad/dyad valence를 명시한다.
- [x] Hook / Fork parser-production parent를 first-class semantic identity로 표현하고, train은 별도 `Train` node 없이 shared Hook/Fork graph로 구성한다.
- [ ] adverb/conjunction/hook/trident application으로 생긴 DerivedEntity와 result part of speech(Verb/Adverb/Conjunction)를 보존한다.
- [ ] boxed noun의 ordinary-data 사용과 modifier-context gerund interpretation을 구분한다.
- [ ] `::` adverse와 `:.` obverse처럼 forward graph 밖의 latent error/inverse semantics를 보존한다.
- [ ] rank-derived verb와 cell/frame 의미를 Semantic Analyzer가 분석할 수 있게 표현한다.
- [ ] rank conjunction의 verb"rank-noun, verb"verb, noun/gerund"rank forms를 source operand 품사 손실 없이 표현한다.
- [ ] Infinite/Absolute/Relative RankSpec과 monad/left/right rank triple을 보존한다.
- [ ] name reference/binding/version과 source span을 필요한 범위에서 연결한다.
- [ ] `NameRef.expected_part_of_speech`와 runtime lookup POS mismatch의 domain error를 모델링한다.
- [ ] sentence 전체의 name environment를 선행 snapshot하지 않고 우측→좌측 assignment/name lookup sequencing을 보존한다.
- [ ] explicit definition의 DefinitionCode와 invocation CallFrame을 분리한다.
- [ ] local slot hint와 실제 local binding을 구분하고 unbound local candidate의 locale fallback을 보존한다.
- [ ] `=.` local assignment와 `=:` public/locale assignment를 구분한다.
- [ ] assignment가 namespace write effect와 assigned-entity result(noun/verb/adverb/conjunction)를 동시에 갖는지 테스트한다.
- [ ] primitive contract를 semantic node에 연결한다.
- [ ] J dyadic rank의 prefix frame agreement와 residual-frame repetition을 명시적으로 테스트한다.
- [ ] zero-cell rank execution의 fill-cell/prototype result type·shape semantics를 테스트한다.
- [ ] rank cell 결과의 type/shape가 다른 경우 J result assembly(type join, shape join, framing fill, assembly error)를 테스트한다.
- [ ] uniform cell-result proof가 있을 때만 rank map을 고정-shape parallel output으로 낮춘다.
- [ ] boxed와 sparse를 physical encoding이 아닌 J-visible semantic representation으로 보존한다.
- [ ] comparison tolerance/`!.` fit context와 J error precedence를 semantic contract에 포함한다.
- [ ] empty operand에서의 context-sensitive type/domain semantics를 dense atomic dyad golden test로 검증한다.
- [ ] Semantic Analyzer가 source parser 없이 J Semantic Array IR만으로 분석 가능하게 한다.
- [ ] Semantic Analyzer / Lowering이 semantic structure를 Logical Array IR / Plan으로 낮추는 테스트를 작성한다.
- [ ] fork/hook의 J-compatible observable execution order를 보존하고 pure/speculatable proof가 있을 때만 branch 병렬화를 허용하는 golden test를 둔다.
- [ ] reduction derived verb와 rank-conjunction-derived verb를 대표 golden test로 둔다.

완료 조건: Semantic Analyzer를 scanner/parser 없이 테스트할 수 있으면서도 hook/fork/train/rank 및 derived verb/adverb/conjunction의 의미 구조가 분석 입력에 남아 있다.

### A1.5 — J Graph IR / JAXA Array Operation Graph IR

**목표:** JAXA의 핵심 연구 표면을 first-class compiler IR로 만든다. parser가 만든 immutable FunctionEntity를 actual noun application과 결합하여, J 문법 자체가 제공하는 graph topology와 optimization hint를 잃지 않는 applied operation graph를 만든다.

> **현재 위상:** `j_graph_ir` v0.3는 **explicit applied-operation graph + access-pattern basis + witnessed rewrite/resource analysis** 단계다. `@:`/Hook/Fork 내부 stage/branch가 실제 `ValueId` node로 전개되고, `/`, `"`, `\`의 Reduce/CellApply/Window 구조가 graph-level basis/resource identity로 보존된다. stage별 GraphFacts/use-count/analyzability와 `ResourceExprGraph`, witnessed rewrite candidate, conservative source-vs-replacement resource evaluation, existing `LoweringRegistry + TargetCapabilities`에 대한 target-only feasibility bridge가 존재한다. 아직 없는 것은 full rewrite-specific shape algebra, executable WindowView lowering, fusion-candidate별 lifetime extension, resolved TargetProfile/ResourceEstimate/CostProfile, 실제 candidate selection/partition이다.

#### A1.5.1 과거 JAXA 역대조 감사

2026-10-01 `yunskim/JAXA`, `yunskim/JAXA-complier`, `yunskim/japchae`, `yunskim/jaxa-analyzer`를 현행 RustJ J Graph IR과 **여러 독립 관점으로 반복 대조**했다. 이번 감사는 (1) language/graph intent, (2) resource/Flow–Storage/static-memory, (3) basis/rewrite/equivalence, (4) frontend prototype, (5) superseded claim 역검토의 다섯 패스로 수행했다.

| 과거 JAXA 개념 | 현행 RustJ 상태 | 판정 |
|---|---|---|
| Semantic AST와 Array Operation IR 분리 | `FunctionEntity`와 `j_graph_ir::Plan`을 별도 boundary로 둠 | 반영 |
| J syntax에서 static graph 직접 유도 | `@:`, Hook/Fork, `/`, `"`를 `GraphForm`으로 분류 | 반영 |
| `@:` pipeline / Hook/Fork branch-join | `Pipeline`, `Hook`, `Fork` + GraphHint | 반영 |
| applied graph stage별 shape 전파 | stage/branch가 explicit `ValueId` node이며 `GraphFacts`를 보유. 다만 구현은 아직 execution `Facts` container로 seed/convert하며 `facts::infer_semantic_call`을 직접 호출한다 | **초기 구현 — rule 공유는 맞지만 container coupling 제거 필요** |
| primitive shape/dtype/rank/effect contract | `GraphRuleRefs` + layout-independent `SemanticFacts` transfer를 J Graph build에서 적용. Execution `Facts` container와 GraphFacts container는 분리 | **초기 구현** |
| primitive symbolic resource contract | `GraphOperationContract`가 iteration/access/fusion/temporary/accumulator/working_state symbolic requirement를 가짐. 구체 resource expression registry는 미완성 | **부분 반영** |
| iteration/reduction/access pattern contract | `IterationContract`/`AccessContract`/`FusionStructure`를 J Graph op에 연결 | **초기 구현** |
| pipeline/reduction/branch/join별 resource composition | Region은 Pipeline/BranchJoin, Apply node는 Reduction/Window/CellMap/Structural composition identity를 직접 기록. `j_graph_resource`가 node/region identity를 노출하나 full symbolic evaluator는 후속 | **부분 반영** |
| intermediate edge materialization/traffic 분석 | `j_graph_memory`가 pipeline/branch/view materialization opportunity와 logical extent를 계산. traffic/selected materialization plan은 후속 | **초기 구현** |
| register/live-value pressure 분석 | J Graph use-def/live-range를 계산하고 branch `live_across`가 join까지 lifetime을 확장. register pressure로의 target mapping은 후속 | **초기 구현** |
| target profile과 graph resource demand 결합 | full TargetProfile/ResourceEstimate는 후속. 다만 Graph rewrite basis를 existing `LoweringRegistry + TargetCapabilities`에 투영하는 target-only feasibility query를 추가해 `Supported / RequiresCallFacts / Unsupported`를 구분한다 | **초기 연결** |
| fusion partition 산출 | candidate/opportunity까지만 있고 partition/feasibility 계산 없음 | **미구현** |
| reshape/flatten/transpose를 virtual view로 취급 | Ravel/Reverse/Transpose에 `VirtualIndexingCandidate`를 J Graph에서 기록 | **초기 구현** |
| Flow–Storage | Logical Execution/Planner 쪽에 별도 모델로 보존 | **의도적으로 downstream — 적절** |
| checkpoint/rematerialization/reversible recovery | `StorageRequirement::ExplicitCheckpoint`와 logical/physical 분리는 설계됨. physical `Rematerialize` decision은 이번 반복 감사에서 명시적 planning option으로 보강했으나 구현은 없음 | **부분 반영 — 후속** |
| adjoint/VJP graph + parameter-adjoint fan-out | `ParallelFanOut` schema만 있고 transform 없음 | 연구/후속 |
| Graph basis → rewrite → equivalence algebra | roadmap/설계만 있음 | 과거 연구와 동일하게 아직 열린 문제 |
| resource-aware rewrite pruning | 없음 | 과거에도 future work; 미구현 |
| basis access-pattern taxonomy | Graph Basis에 Window access family를 추가해 `u\`를 `PrefixInfix`로 보존하고 `(+/)\`를 `Window → Reduce`로 표현. Scan은 별도 Graph Basis 원소로 승격할지 열린 질문으로 유지 | **초기 구현** |
| symbolic resource function/composition | `GraphOperationContract`와 `j_graph_resource`가 최소 합성을 수행한다. `ResourceExprGraph`가 ValueAtoms/Requirement/Sum/Max 식을 보존하고 Pipeline/BranchJoin의 internal/elidable/retained/peak-live provenance를 표현한다. PrefixInfix/Rank는 inner resource requirement를 합성한다. richer accumulator/window-size 함수와 target realization은 후속 | **초기 구현** |
| resource-aware pruning soundness | checklist에는 있으나 local/global resource 구분, monotonicity/soundness proof requirement가 명문화되지 않았음 | **설계 보강 필요** |
| Basis → Rewrite → Equivalence → Optimization 의존 순서 | 각 기능은 roadmap에 있으나 선행관계가 약하게 표현됨 | **설계 보강 필요** |
| static-analyzable subset / validation boundary | `GraphAnalyzability`로 Static / StaticWithUnknownFacts / RequiresSpecialization / DynamicSemanticFallback을 구분 | **초기 구현** |
| jsource-style graph normalization(capped fork→atop, tine simplification) | 현 `j_graph_ir`에는 별도 normalization pass 없음 | **미구현/확인 필요** |
| multi-device static partition | 없음 | future work |

**핵심 판정:** 현재 IR은 JAXA의 가장 중요한 **“표기에서 graph topology를 직접 얻는다”**는 주장을 복구했다. 그러나 과거 연구에서 `Array Operation IR`이라는 말은 topology만이 아니라 **shape/flow/fusion/resource 분석을 수행할 수 있는 stage-level operation graph**를 의미했다. 현행 v0.1은 아직 그 수준까지 가지 않았다.

v0.2에서 explicit stage/branch graph를 도입했고, v0.3에서 Window access family, graph-only fact domain 경계, node-level Reduction/Window/CellMap resource composition을 추가했다.

1. `@:` stage, Hook/Fork branch/join은 이제 실제 J Graph `ValueId` node/edge다.
2. 원래 J combinator identity는 `Region(Pipeline/Hook/Fork)`으로 별도 보존한다.
3. stage별 `Facts`, `GraphOperationContract`, `GraphAnalyzability`, use-count를 graph에서 질의할 수 있다.
4. `j_graph_memory`가 logical extent, graph-order live range, pipeline/branch/view materialization opportunity를 계산한다.

남은 핵심 부족은 **rewrite candidate를 실제 executable lowering/schedule/target resource model로 연결하는 단계**다. Graph 쪽에서는 `ResourceExprGraph`로 internal/elidable traffic, retained/peak-live, temporary/accumulator/window-state requirement와 canonical state lifetime을 표현하고, rewrite candidate도 동일 logical-atom/symbolic-state domain에서 비교한다. 다음 경계는 rewrite-specific facts의 확대, WindowView 등 execution lowering capability, fusion 선택에 따른 lifetime extension, resolved TargetProfile 기반 ResourceEstimate/CostEstimate, 그리고 그 뒤의 candidate selection/partition이다.

#### 2026-10-01 반복 감사에서 추가로 확정한 JAXA 계승 원칙

1. **Graph Basis의 historical lower bound는 arithmetic atom이 아니라 access pattern 계층이다.** Japchae D-24의 핵심은 너무 작은 scalar `+`/`*`로 분해해 algorithm/access identity를 잃지 말라는 것이다. RustJ의 Graph Basis는 이 원칙을 유지한다. `Conv` 같은 structured op를 Graph Basis에서 black box로 유지하는 결정과 Execution Basis에서 필요 시 분해하는 결정은 독립적이다.
2. **Graph Basis vocabulary에는 windowing 계열이 필요하다.** historical 후보는 `map / reduce / window-reduce / static-reindex / dynamic-gather`이고 `scan`은 독립 패턴인지 열린 질문이었다. RustJ는 `WindowReduce` 또는 이에 동등한 graph-level window access identity를 추가해야 한다. ExecutionBasis::WindowView가 존재한다는 사실로 이 요구를 대체하지 않는다.
3. **Basis 연구가 rewrite/equivalence보다 선행한다.** 작업 의존은 `Basis → Rewrite → Equivalence → Optimization`으로 둔다. 완전한 최소 basis 증명까지 기다릴 필요는 없지만, rewrite rule은 어떤 Graph Basis identity를 보존/변환하는지 명시해야 한다.
4. **resource contract는 node별 고정 숫자도, 단순 enum 합도 아니다.** target-independent graph 층은 symbolic requirement/access/liveness/materialization 관계를 합성하고, schedule/target 이후 concrete register/shared/global resource를 계산한다. 현재 최소 `SymbolicResourceExpr`는 seam일 뿐 최종 모델이 아니다.
5. **resource-aware pruning은 매우 후순위다.** basis/rewrite/equivalence가 먼저 서야 하며, pruning은 (a) 해당 resource bound가 부분 graph에서 local하게 결정 가능한지, (b) pruning predicate가 monotone하거나 그 밖의 soundness proof를 갖는지 확인된 경우에만 허용한다. 그렇지 않으면 후보 생성 후 cost/resource evaluation만 수행한다.
6. **static memory claim은 logical determinability로 해석한다.** graph에서 extent/use/lifetime/storage obligation을 정적으로 알 수 있다는 주장은 유지하지만 physical offset/buffer/layout을 J Graph semantic fact로 올리지 않는다.
7. **adjoint/VJP는 basis/rewrite보다 앞서지 않는다.** historical 연구도 복합 graph의 AD는 basis/graph expansion 위에서 자연스럽게 닫히는 문제로 보았다. 현재 `ParallelFanOut` schema는 유지하되 실제 AD transform은 Graph Basis와 rewrite/equivalence surface가 더 성숙한 뒤 진행한다.
8. **frontend 역사 prototype은 current jsource보다 우선하지 않는다.** `JAXA-complier`의 tokenizer/enqueuer/parser Python prototype은 유용한 참고 구현이지만, name lookup timing과 parser behavior의 oracle은 current jsource `w.c/p.c/cf.c`다. 특히 전체 name 품사를 parser 전에 미리 확정하는 모델로 되돌아가지 않는다.
9. **초기 JAXA의 강한 구현 주장은 그대로 계승하지 않는다.** `"RjP`, rank 변화=항상 fusion boundary, 모든 shape op=항상 zero-copy, parse-time complete graph, fixed physical offset, primitive 고정 register 숫자는 후기 연구 또는 current RustJ 계층 분리와 충돌하므로 superseded다.


**목표:** JAXA의 핵심 연구 표면을 first-class compiler IR로 만든다. parser가 만든 immutable FunctionEntity를 actual noun application과 결합하여, J 문법 자체가 제공하는 graph topology와 optimization hint를 잃지 않는 applied operation graph를 만든다.

- [x] `src/j_graph_ir.rs`에 독립 J Graph IR을 추가하고 `Engine::analyze_j_graph()` inspection API를 제공한다.
- [x] `GraphBasis` / `GraphBasisKind`를 Execution basis 타입과 분리하고, derived rank/reduction처럼 outer→inner graph-basis composition을 보존하는 최소 seam을 추가했다.
- [x] Graph Basis에 Window access family를 추가하고 J `\`을 `GraphForm::PrefixInfix`로 보존한다. operand basis를 중첩해 `(+/)\`가 `Window → Reduce`가 되게 했다. Scan은 독립 Graph Basis 원소인지 열린 질문으로 명시한다.
- [x] 기존 `SymbolicResourceExpr` requirement leaf 위에 `ResourceExprGraph`를 추가해 logical `ValueAtoms`, symbolic requirement, `Sum`, `Max` composition을 표현한다. node requirement와 region internal/elidable/retained/peak-live 식 provenance를 보존하며 concrete target 숫자는 넣지 않는다.
- [x] GraphFacts inference에서 execution `Facts`/`LayoutFact` container seed/return adapter를 제거하고 layout-independent `SemanticFacts` domain/API를 사용한다. primitive shape/dtype rule source는 execution inference와 공유한다.
- [ ] Reduction/CellMap/Window resource composition identity를 실제 Apply node와 verifier/resource summary에 연결했고, Rank/PrefixInfix가 inner accumulator/working-state requirement를 보존하도록 합성했다. 남은 일은 이 node composition을 Pipeline/BranchJoin과 같은 symbolic lifetime/traffic evaluator까지 확장하는 것이다.
- [ ] Pipeline/BranchJoin의 edge traffic/retained/peak-live와 child resource state를 `ResourceExprGraph`로 합성했다. 각 temporary/accumulator/window-state에는 canonical graph-order `ResourceStateLiveRange`와 `may_extend_across_fusion`을 기록하고 region별 canonical peak와 conservative all-child-state upper bound를 모두 만든다. 남은 일은 실제 fusion candidate별 lifetime extension/overlap 제약과 target realization 함수까지 연결하는 것이다.
- [x] Graph Basis → rewrite candidate generation → equivalence validation → candidate resource evaluation → sound resource pruning 순서를 `GRAPH_OPTIMIZATION_ORDER`와 rule registry API에 반영했다.
- [x] resource-aware pruning은 `ResourceBoundLocality::Local` + `PruningMonotonicity::ProvenMonotone`가 모두 있는 rule에만 early pruning을 허용하도록 contract를 정의했다. 현재 `E.` rewrite는 global-context-dependent/unproven이라 pruning 불가다.
- [x] `GraphForm`으로 Atomic / Pipeline(`@:`) / Hook / Fork / Reduce(`/`) / PrefixInfix(`\`) / Rank(`"`) / generic Modifier를 구분한다.
- [x] `GraphHint`로 PipelineFusionCandidate / IntermediateMaterializationElision / BranchJoinFusionCandidate / RetainedValueCandidate / ParallelBranchCandidate / ReductionStructure / WindowStructure / CellParallelStructure를 기록한다.
- [x] `GraphRuleRefs`로 shape/dtype/rank-cell/effect rule source와 resource rule의 StructuralComposition/Unknown을 명시한다.
- [x] `Engine::analyze_compilation()`이 `j_graph`와 `execution` 두 IR을 함께 반환한다.
- [x] execution lowering은 BoundProgram을 직접 canonicalize하지 않고 J Graph IR을 소비한다.
- [x] execution node/A3 op가 `j_origin`으로 originating J Graph node를 보존한다.
- [x] Hook/Fork/@: topology 분류의 단일 소스를 `j_graph_ir::classify_function()`으로 두고 execution analyzer의 독립 pattern rediscovery를 제거한다.
- [ ] `\` Prefix/Infix의 graph vocabulary는 추가했다. 남은 Cut/Window(`;.`), Dot/Contract, Power/Iteration, Key/GroupBy 및 별도 Scan basis 여부를 GraphForm/GraphHint로 확장한다.
- [ ] `ExecutionBasis::WindowView` semantic payload는 `WindowShapeSpec::PatternShape`로 구현했고 expansion verifier가 inputs/payload 일치를 강제한다. `FindViaWindowMatch` 전체에 대해서는 current `E.` rank≤1 semantics와 동일한 CPU `ReferenceRewriteComposite` evaluator/capability를 추가했다. standalone WindowView value/kernel lowering은 아직 없으며 일반 Window rewrite를 위해 후속 구현한다.

- [x] current primitive/rank/reduce 범위에서 stage별 shape/dtype/rank facts를 J Graph build 중 전파한다. richer rule registry는 계속 확장한다.
- [x] `GraphOperationContract`로 iteration/access/fusion 및 temporary/accumulator/working_state symbolic requirement의 최소 seam을 추가했다.
- [x] graph-level use-def/common-input/live-range를 J Graph 및 `j_graph_memory`에서 계산한다.
- [x] logical extent(atom count)와 materialization opportunity를 J Graph에서 정적으로 계산한다.
- [x] `j_graph_resource`에서 Pipeline/BranchJoin의 internal/elidable/retained/peak-live atom volume과 reduction accumulator requirement를 합성하는 최소 evaluator를 구현했다. Reduction/CellMap 단독-region 및 traffic 식은 계속 확장한다.
- [ ] graph rewrite에 대해서는 existing `TargetCapabilities`/`LoweringRegistry`와의 초기 feasibility bridge를 추가했다. representation/schedule/full TargetProfile을 결합한 downstream `ResourceEstimate`는 여전히 후속이다.
- [x] 최소 graph rewrite registry/candidate sidecar를 추가하고 첫 rule로 `E.` Search → Window+CellApply(Match) 후보를 J Dictionary equivalence witness와 함께 생성한다. 일반 rule set 확장은 계속 필요하다.
- [x] graph candidate마다 source ValueId/span/basis provenance, registered semantic-equivalence witness, `RewriteFactRuleId`를 유지한다. verifier는 provenance/rule/witness뿐 아니라 rewrite-local GraphFacts를 rule로 재계산해 stale/invented facts도 거부한다.
- [ ] adjoint/VJP transform을 J Graph IR transform으로 추가하고 fan-out / accumulation topology를 explicit하게 만든다.
- [ ] name-bound derived verb의 graph summary를 binding version + SpecializationKey로 interprocedurally 전파한다.
- [ ] graph rewrite candidate를 source/replacement의 동일 logical-atom + symbolic-state resource domain에서 평가하고 rewrite-local GraphFacts를 보존한다. existing `LoweringRegistry + TargetCapabilities`로 replacement GraphBasis를 execution basis에 투영해 target-only feasibility도 질의한다. unknown cost는 `Incomparable`, call-dependent legality는 `RequiresCallFacts`로 남긴다. `RewritePlanningReport`가 resource + target readiness를 합쳐 `TargetUnsupported / NeedsCallFacts / NeedsResourceFacts / ReadyForCosting`까지만 판정하며 후보 선택은 하지 않는다. 현재 `E.` candidate는 CPU에서 standalone WindowView 없이도 whole-rule `ReferenceRewriteComposite`로 target-feasible하다. 다만 candidate-local Window extent/traffic이 아직 unknown이라 planning state는 `NeedsResourceFacts`이고, GPU/standalone WindowView route는 Unsupported다. 남은 일은 rewrite-specific Window shape/resource 식, 일반 WindowView lowering, full TargetProfile/ResourceEstimate/CostProfile과 연결하는 것이다.

**완료 조건:** 대표 J expressions(`@:`, Hook, Fork, Reduce, Rank, 이후 Window/Contract/Key/Power)가 generic execution DAG를 만들기 전에 J Graph IR에서 구조적으로 식별되고, graph optimizer가 source reparsing이나 execution-DAG pattern recovery 없이 fusion/lifetime/parallel/rewrite 후보를 만들 수 있다.

#### A1.5.2 JAXA의 static-memory claim을 RustJ에서 해석하는 방식

JAXA의 중요한 주장 중 하나는 **배열 연산을 J DSL로 정적으로 표현하면 graph를 실행하기 전에 필요한 메모리 구조를 상당 부분 결정할 수 있다**는 것이다. RustJ는 이 주장을 버리지 않되, `logical memory`와 `physical memory`를 구분한다.

J Graph 단계에서 정적으로 계산할 수 있는 것:

- 각 logical ArrayValue의 shape / rank / dtype fact
- shape가 known이면 atom count
- producer-consumer use-def와 fan-out
- graph-order lifetime / last-use
- Hook/Fork의 live-across value
- pipeline/branch 내부 intermediate
- 어떤 value가 materialization-elision 후보인지
- static reindex/view가 virtual하게 유지될 가능성
- reduction accumulator / temporary / working_state의 symbolic requirement
- representation model이 주어졌을 때 logical extent의 represented byte size

J Graph 단계에서 **아직 결정하지 않는 것**:

- 실제 register allocation
- register class별 사용량
- shared/LDS/scratchpad의 concrete byte 수
- tile/workgroup별 local storage
- packed-bool/box/sparse 등의 final representation
- alignment/padding/buffer offset
- spill/occupancy
- exact physical allocation/reuse

따라서:

~~~text
J syntax / J Graph
    ↓
Static logical memory analysis
    shape → atom count → use/lifetime → materialization opportunity
    ↓
Representation + Schedule + Target
    ↓
ResourceEstimate / bufferization
    register/shared/global bytes, peak physical memory, traffic
~~~

현재 구현:

- `src/j_graph_memory.rs`의 `StaticMemoryAnalysis`
- `LogicalExtent { shape, atoms, dtype }`
- `GraphOrderLiveRange { defined_at, last_use }`
- `PipelineIntermediate`, `RetainedAcrossBranch`, `BranchIntermediate`, `VirtualView` materialization opportunity
- explicit `AtomRepresentation`을 제공할 때만 byte size 평가
- 모든 logical value를 materialize한다고 가정한 `graph_order_peak_materialized_bytes()` 제공

이 conservative peak는 최종 resource estimate가 아니다. fusion/materialization selection 전의 upper-bound-like graph estimate이며, JAXA의 핵심인 **“graph에서 memory obligation을 정적으로 계산한다”**는 주장을 검증하기 위한 분석 결과다.

현재 `j_graph_resource` 최소 evaluator는 region별 `internal_atoms`, `elidable_materialization_atoms`, `retained_live_atoms`, `graph_order_peak_live_atoms`, `has_reduction_accumulator`, `has_unknown_resource_requirement`를 계산한다. 이것은 target-independent logical resource summary다.

향후 `ResourceCompositionRule` evaluator는 다음 불변조건을 따른다.

- Pipeline: stage temporary는 lifetime이 겹치지 않으면 재사용 가능하며 internal edge materialization을 제거할 수 있다.
- Reduction: 큰 producer result 대신 accumulator state로 직접 소비할 수 있는지를 표현한다.
- Branch: sibling branch의 live temporary와 retained input이 겹칠 수 있다.
- Join: 두 branch result가 join 시점에 동시에 live할 수 있다.
- memory traffic은 node resource의 단순 합이 아니라 **materialized edge의 write/read**를 중심으로 계산한다.
- register pressure는 primitive register 숫자의 합이 아니라 **simultaneously-live symbolic values**를 중심으로 계산한다.


### A2 — Extension Primitive Registry와 analysis contract

> **구현 주의:** 아래 목록 전체는 A2의 장기 architecture inventory다. A3-v0/첫 CPU vertical slice를 막는 하나의 거대한 선행 milestone로 취급하지 않는다.
>
> **A2-v0 blocking subset**
> - built-in/extension이 공유하는 최소 semantic capability interface
> - valence별 rank + shape/type/effect/error 최소 contract
> - `ValueFacts`의 최소 Type/Rank/Shape domain + compile-time Witness
> - Map/Reduce 수준의 IterationDomain
> - `AccessFact = Known(simple) | Opaque`
> - lowering eligibility/coverage manifest
> - 첫 실행 op에 필요한 native CPU lowering
>
> **A2-later**
> - richer ArrayPropertyFacts + full morphology worklist/fixpoint
> - interprocedural morphology/specialization cache
> - GraphIndex/AnalysisIndex batch-analysis view
> - full TargetProfile/TargetQueries
> - target locale chain
> - ResourceEstimate/CostEstimate/CompiledResourceReport
> - mixed RoutePartition boundary bridge
> - richer Window/Scan/Gather/Scatter access/resource model



- [ ] extension name을 parser keyword로 만들지 않고 ordinary name binding으로 등록한다.
- [ ] Enqueue는 extension도 ordinary NAME/lookup metadata로 처리하고, parser-time normal name lookup이 현재 binding의 품사를 결정하게 한다.
- [ ] parameterized adverb(`conv`, `linear` 등)와 그 결과 derived computational verb/op identity를 분리한다.
- [ ] built-in과 extension-derived computational entity가 공유하는 semantic capability interface를 정의한다.
- [ ] extension builder(adverb/conjunction/verb) identity와 derived computational entity identity를 분리한다.
- [ ] `PrimitiveSpec`을 semantic identity/version record로 축소하고 semantic capability interface와 lowering/realization registry를 분리한다.
- [ ] monad / dyad-left / dyad-right별 innate RankSpec과 cell axis-role contract를 정의한다.
- [ ] `IterationDomain`, `AccessRelation`, `InvarianceFact`, `ConstraintSet`, `SemanticMaskSemantics`를 정의하여 leading axis보다 일반적인 hardware-relevant logical contract를 만든다.
- [ ] access-pattern taxonomy(Map/Reduce/WindowReduce/Scan/StaticReindex/Gather/Scatter)를 최소 형태로 정의한다.
- [ ] shape/dtype/effect/alias/semantic-reference 계약을 정의한다.
- [ ] `ValueFacts`를 Type/Rank/Shape/ItemCount/Constant/ArrayProperty/Constraint의 abstract-domain 집합으로 정의한다.
- [ ] `ArrayPropertyFacts` domain(IntegralValued/NonNegative/Unique/Sorted/Permutation/KnownRange 등)의 최소형과 primitive transfer rule interface를 정의한다.
- [ ] optimizer가 사용하는 추론 fact에 `FactWitness`/provenance를 연결하는 최소 contract를 정의한다.
- [ ] logical `ConstraintSet`과 downstream `RepresentationFacts`를 분리한다.
- [ ] `CompilationTarget = BackendFamily + ArchitectureTarget + DeviceProfile + RuntimeProfile`을 정의하고, 기존 `TargetProfile`은 resolved effective view로 사용한다.
- [ ] compile invocation 시작 시 `CompilationTargetLocale` / `TargetContext`를 확정하고 lowering lookup의 root로 사용한다.
- [ ] AOT CLI와 향후 JIT API가 동일한 `TargetSelector/TargetOptions` contract를 사용하게 한다. JIT는 runtime discovery를 추가 evidence로만 사용하고 별도 target-selection 체계를 만들지 않는다.
- [ ] `target=auto`와 explicit target constraint의 precedence를 정의하고 AOT/JIT 양쪽에서 동일하게 테스트한다.
- [ ] built-in primitive와 extension-derived op가 source identity와 무관하게 동일 active target locale/path에서 lowering/capability를 조회하는 테스트를 추가한다.
- [ ] execution hierarchy/register allocation rules/memory & resource coupling/compute & execution scope/sync & memory ordering/data movement/execution mode/ABI capability를 architecture/device profile에 올바르게 분리한다.
- [ ] compiler target locale chain(device → architecture → family → backend → cpu/gpu → generic)을 정의한다.
- [ ] built-in J primitive도 extension과 동일하게 target lowering binding을 locale chain에서 조회한다.
- [ ] lowering lookup key에 primitive/source identity뿐 아니라 resolved valence와 derived rank/fit/numeric semantics를 포함한다.
- [ ] hard target facts와 empirical `CostProfile`을 분리한다.
- [ ] Physical Plan에 logical-axis mapping/tile/vector-subgroup-workgroup/memory-space/layout/pipeline 정보를 기록한다.
- [ ] `ResourceEstimate`를 graph + schedule + TargetProfile의 함수로 계산하고 `CostEstimate`를 별도 계층으로 둔다.
- [ ] backend가 실제 register/spill/shared-memory 결과를 돌려주는 `CompiledResourceReport`와 re-plan 경로를 정의한다.
- [ ] `TargetProfile`을 stable facts와 architecture-specific `TargetQueries`로 분리한다.
- [ ] Logical Array IR → MLIR export adapter의 최소 contract를 설계한다.
- [ ] whole-program route 선택이 아니라 subgraph/region 단위 `RoutePartition`과 boundary value bridge를 정의한다.
- [ ] StableHLO로 안전하게 내릴 수 있는 subset을 명시하고 unsupported semantics를 거부하는 규칙을 만든다.
- [ ] resource 함수는 고정 숫자가 아니라 fusion context/target에 대한 함수로 둔다.
- [ ] register estimate는 primitive별 합이 아니라 scheduled liveness peak로 계산한다.
- [ ] accumulator requirement(logical)와 accumulator realization(schedule/target)을 분리한다.
- [ ] scratchpad/shared usage를 tile/reuse/pipeline-stage 함수로 계산한다.
- [ ] 첫 extension set(`relu`, `linear`, `conv2d`, `flatten`, reduction/pool)을 port한다.
- [ ] noun snapshot과 verb/adverb/conjunction nameref late lookup, alias/shadow/rebind, `f.` fix semantics를 구분하는 테스트를 추가한다.
- [ ] mutable extension state가 hidden verb field가 아니라 explicit StateResource로 나타나는 테스트를 추가한다.
- [ ] standard-J reference definition이 가능한 extension은 차등 oracle test를 추가한다.

완료 조건: 새 NN primitive 하나를 추가할 때 scanner/parser 수정 없이 registry/spec/lowering만 추가하면 되고, Semantic Analyzer가 rank·iteration domain·axis semantics·access relation·numeric/dependency/effect contract를 읽을 수 있으며, RustJ-native route에서는 별도 TargetProfile을 이용해 schedule/ResourceEstimate를 만들고 external route에서는 adapter가 같은 Logical IR contract를 검증해 lowering할 수 있다.

### A3 — Logical Execution IR core, verification, scheduling boundary

> **단계화:** APEX/Co-dfns/TAIL 반영 항목은 단계적으로 도입한다. 첫 verified single-block Logical IR(A3-v0)은 SSA ValueId + 최소 Type/Rank/Shape/Witness + verifier를 우선한다. full GraphIndex, full morphology fixpoint, interprocedural SpecializationKey cache, richer ArrayPropertyFacts는 A3-v0의 선행조건이 아니며 v1/later에서 추가한다.
>
- [x] A3-v0에 SSA `ValueId`와 explicit single Function/Region/Block/`Return` Terminator 최소 구조를 정의했다.
- [x] J name/symbol identity, `BindingVersion`, A3 SSA `ValueId`를 서로 다른 타입/field로 구분한다.
- [ ] immutable semantic/Logical DAG에서 유도되는 `GraphIndex` / `AnalysisIndex` sidecar(parent/depth/preorder/subtree/op/entity/scope/use-def/source-origin)를 정의한다.
- [ ] graph index는 derived analysis view이며 semantic DAG의 canonical identity를 대체하지 않는다는 verifier/invariant를 둔다.
- [ ] morphology transfer를 worklist/fixpoint로 실행할 최소 `MorphologyEngine` interface를 정의한다.
- [ ] interprocedural summary와 call-site specialization을 `SpecializationKey` + cache로 표현한다.
- [ ] specialization key에 포함할 fact relevance 정책과 code-explosion merge/widening 정책을 정의한다.
- [ ] PureArray/GuardedDynamic/Stateful/RuntimeSemantic region 분류를 EffectAnalysis/RoutePartition contract에 추가한다.
- [ ] `CellApply/Map/Reduce/Scan/Reindex/Loop` 같은 high-level parallel structure의 early scalarization을 금지하는 Logical IR invariant를 추가한다.
- [x] A3 `CallOp + ExecutionBasisPayload` 공통 contract와 `ExecutionBasisKind` identity를 정의하고, target-specific realization은 `ExecutionBasisLoweringCapability` registry로 분리했다. 이 vocabulary는 GraphBasis와 별도 계층이다.
- [x] J syntax-derived `StructuralOpportunity` sidecar를 추가했다. `@:`는 Pipeline, hook/fork는 BranchJoin topology와 live-across/shared-input provenance를 analysis/A3 IR에 보존한다.
- [x] StructuralOpportunity discovery와 semantic legality/target feasibility/physical fusion commitment을 서로 다른 단계로 분리했다.
- [ ] adjoint/VJP expansion이 생기면 data-adjoint/parameter-adjoint branch를 `ParallelFanOut` opportunity로 연결한다.
- [ ] name-bound derived verb의 FunctionEntity/topology summary를 binding version + SpecializationKey로 전파해 `@:`/hook/fork opportunity가 call boundary에서 사라지지 않게 한다.
- [ ] StructuralOpportunity와 use-def/GraphIndex를 결합해 pipeline intermediate materialization-elision 및 branch live-range 분석을 일반화한다.
- [ ] J Graph IR의 GraphForm/GraphHint vocabulary를 Cut/Window, Dot/Contract, Power/Iteration, Key/GroupBy 등 J graph algebra 전반으로 확장한다.
- [ ] primitive마다 J Graph IR용 shape/dtype/rank/effect/resource rule reference를 연결하고, 아직 모르는 항목은 명시적 Unknown으로 둔다.
- [ ] Graph basis verb 위 rewrite/equivalence rule을 J Graph IR에서 표현하여 동일 execution semantics를 갖는 여러 J graph 후보를 생성할 수 있게 한다.
- [ ] target ResourceEstimate/register/shared-memory model을 opportunity별 feasibility query로 연결하되 Logical IR payload에는 concrete hardware allocation을 넣지 않는다.
- [x] ResolvedInstantiation 최소 record를 정의하여 우선 target/valence/input-output dtype·rank/requested-rank instance를 기록한다. cell-rank/value-role/numeric-policy 확장은 후속 refinement다.
- [x] ValueRoleFacts 최소형을 추가했다. 현재 ShapeVector/AxisPermutation/IndexVector/CountVector를 실제 분석에서 생산하며 나머지 role enum은 후속 basis가 사용한다.
- [x] J-visible predicate failure를 표현하는 first-class zero-result SemanticCheck를 A3 IR에 정의하고 compiler assertion과 분리했다.
- [x] ExecutionBasisExpansion sidecar에 applicability ConstraintSet + equivalence witness를 두고 original semantic/structured identity를 보존한다. 첫 rule은 E. → WindowView + CellApply(Match)다.
- [x] A3-v0 correctness executor 범위를 Elementwise/CellApply/Reduce/StaticReindex/IndexSpace/SemanticCheck 중심으로 제한했다. `logical_executor::execute_closed`는 closed expression reference path이며 native Physical Executor와는 별개다.
- [x] A3 verifier negative tests, runtime/reference-equivalence tests, Rank/Reduce 및 E. expansion composition tests의 golden scaffold를 추가했다.
- [x] `ParameterizedLoweringRecipe` interface와 `LoweringRegistry` 후보 생성 경로를 정의해 resolved call facts + target capability에서 multiple realization 후보를 만들 수 있게 했다. cost ranking/schedule 선택은 아직 downstream 과제다.
- [ ] pure graph region과 CFG region을 구분한다.
- [x] v0 `ConstraintSet + FactWitness`를 정의하고 PrefixAgreement/CellFrameAgreement/IndicesInBounds를 우선 연결했다. runtime branching `Guard`는 v1로 유지한다.
- [x] v0 `EffectSummary + SpeculationSemantics` resolved-call interface를 정의했다. explicit `EffectToken`은 v1로 유지한다.
- [x] v0 `PossibleErrors { known, unknown }`와 first-class `SemanticCheck`로 MayRaise를 보존한다. primitive별 완전한 error-set refinement와 exceptional CFG edge는 후속이다.
- [x] A3 `DestinationRelation`을 정의해 logical alias/reuse legality seam과 physical `BufferId`를 분리했다. 현재 call 기본값은 보수적으로 `Unknown`이다.
- [x] A3 verifier가 schema/container/op-value producer/order/basis payload/instantiation/constraint/zero-result check invariants를 검증한다.
- [x] A3-v0 `SemanticCapabilityView`를 정의해 result facts(shape/type/rank 포함), iteration/axis domain, access, effect, speculation, possible errors, destination/alias seam을 공통 API로 노출한다. richer property/alias interface는 후속 확장한다.
- [ ] schedule/transform representation을 Logical payload IR과 분리한다.
- [ ] external adapter capability negotiation과 guarded lowering을 정의한다.
- [x] A3 `IrSchemaVersion`과 compiler version/primitive registry version provenance를 IR header에 추가했다.
- [x] v0 single-block graph/check/witness/verifier/reference-executor golden tests를 추가했다. branch/loop/effect-token/dynamic-guard tests는 v1로 유지한다.

완료 조건: Logical IR이 RustJ-native planner와 external adapter 양쪽에서 동일한 verifier/interface contract를 통해 소비될 수 있고, buffer/layout/schedule을 넣지 않아도 control/effect/dynamic constraint semantics를 잃지 않는다.

구현은 단계적으로 한다.

```text
A3-v0
  single Function
  single Region / single Block
  pure array ops
  SSA ValueId
  verifier
  shape/axis/numeric contracts
  simple Known access or explicit Opaque access fact
  EffectSummary/Speculation interface
  ConstraintSet + compile-time Witness의 최소형
  Basis core: Elementwise / CellApply / Reduce / StaticReindex / IndexSpace
  first-class SemanticCheck + possible J error set

A3-v1
  multi-block CFG
  branch / loop / runtime Guard
  try/catch/throw exceptional edges
  EffectToken
  richer alias/destination analysis

A3-v2
  portable serialization/version migration
  async/control-effect extensions as needed
```

즉 장기 IR이 Region/Block을 지원한다고 해서 첫 구현에서 전체 CFG framework를 완성할 필요는 없다.

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
- [ ] primitive-specific overflow/retry/promotion 및 error precedence 보존
- [ ] comparison tolerance/`!.` contract 보존
- [ ] 일반 float reduction과 compensated `+/!.0` 같은 derived numeric policy를 구분한다.
- [ ] NaN/Inf/signed zero/empty/fill-cell 테스트

### G4 — RustJ-native 최소 Physical Plan과 CPU Executor

- [ ] Buffer bind
- [ ] View
- [ ] Materialize
- [ ] Kernel call
- [ ] Output ownership
- [ ] last physical use
- [ ] buffer reuse proof
- [ ] layout-compatible view 유지
- [ ] CPU executor
- [ ] source → J Semantic Array IR → Semantic Analyzer/Lowering → Logical Array IR/Plan → RustJ-native Schedule/Physical Plan → CPU end-to-end

### G5 — RustJ-native 성능 및 physical 확장 경계

- [ ] structural view 생성 비용
- [ ] copy/allocation/peak/retained bytes
- [ ] general-stride indexing 비용
- [ ] materialization 비용 비교
- [ ] contiguous 기존 성능 회귀 확인
- [ ] Windows default/portable 전체 회귀
- [ ] 지원 layout/type/operation 표 갱신
- [ ] tiled/placement/transfer/completion 확장 경계 확인

### C — frontend / 언어 의미 확장

Semantic IR/Logical IR 경계의 정확성을 막는 frontend 결함은 즉시 수정한다. 일반적인 언어 기능 확장은 A1~A3의 core IR 경계를 먼저 안정화한 뒤 진행하며, G2~G5와는 필요 의존성에 따라 병행한다.

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
- GPU test
- CUDA benchmark

Linux CI/GitHub Actions는 **별도 요청이 있을 때만 확인**하며, 기본 구조 진행/체크리스트의 완료 gate로 사용하지 않는다.

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

기준일: 2026-10-01.

- 제한된 CPU J interpreter/runtime 경로가 동작한다.
- state-table word formation과 transitional Semantic IR parser가 존재한다.
- parser-produced shared `FunctionEntity`가 primitive, modifier application, hook/fork/train, rank/@: 구조를 보존한다.
- J Graph IR이 별도 canonical analysis surface로 존재하고 Graph Basis, structural opportunity, graph rewrite/resource analysis 기초가 구현되어 있다.
- crate-private `transition_ir::LogicalPlan`과 `logical_ir.rs`의 A3 SSA `Plan`이 아직 병행 출력된다. 그러나 canonical A3는 이제 J Graph lowering 중 incremental하게 직접 생성되며 완성된 transition plan을 재변환하지 않는다. **M1의 남은 핵심은 compatibility `transition` output/API와 transition container 타입 자체를 삭제하는 것**이다.
- A3-v0에는 SSA ValueId, Function/Region/Block/Return, Execution Basis payload, SemanticCheck, ConstraintSet/FactWitness, Effect/Speculation/PossibleErrors/DestinationRelation, verifier가 구현되어 있다.
- `SemanticCapabilityView`, `ParameterizedLoweringRecipe`, `LoweringRegistry`, 기본 target legality/candidate generation과 contiguous route partition prototype이 구현되어 있다.
- 현재 RoutePartition은 class + operation range 중심의 prototype이며 boundary values/preconditions/chosen external route/bridge representation은 아직 없다.
- `logical_executor::execute_closed`는 A3 correctness/reference executor이며 native Physical Executor는 아니다.
- Logical/Physical Array 분리 원칙은 문서와 테스트로 고정되어 있고, G1 read-only CPU affine `PhysicalArray`/BufferId/BufferLease가 구현되어 있다.
- `Value`의 dense payload가 아직 `CpuStorage`를 직접 소유하므로 runtime carrier는 완전한 logical/physical 분리 이전의 migration state다.
- `facts::RepresentationClassFact`는 Dense/AxisSparse J-visible representation class만 나타내며, stride/offset/device/buffer 같은 physical layout은 포함하지 않는다.
- sparse/boxed/packed-bit 기반 구현이 일부 있으나 semantic representation과 concrete backend encoding 경계는 추가 정리가 필요하다.
- G2~G5와 Schedule/Physical Planner/Physical Execution Plan/CPU native executor는 미완료다.
- F1/F2/P1~P7의 jsource-compatible Enqueue/9-row parser cutover는 미완료이며 현재 modifier/train heuristic parser는 transitional implementation이다.
- MLIR adapter, StableHLO adapter, ArrayFire external route는 아직 참고/설계 단계다.
- TargetProfile/CostProfile/ResourceEstimate/CostEstimate의 완전한 구현은 아직 없다.
- 실제 CUDA storage/kernel은 없다.
- **Linux CI/GitHub Actions 결과는 별도 요청이 없으면 구조 진행 판단과 완료 gate에서 생략한다.** 로컬/명시적으로 실행한 검증만 완료 기록에 사용한다.

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

### ArrayFire / J ArrayFire add-on / fusion systems

확인 기준: 2026-10-01.

주요 참고 자료:

- ArrayFire JIT: https://arrayfire.org/docs/jit.htm
- ArrayFire Unified Backend: https://arrayfire.org/docs/unifiedbackend.htm
- CUDA interoperability: https://arrayfire.org/docs/interop_cuda.htm
- Memory manager API: https://arrayfire.org/docs/group__memory__manager.htm
- Jsoftware ArrayFire add-on, pinned at `b0543c8278fe7a50e0ac9f938a936b4a84ee239b`:
  https://github.com/jsoftware/math_arrayfire/tree/b0543c8278fe7a50e0ac9f938a936b4a84ee239b
- J add-on manual:
  https://github.com/jsoftware/math_arrayfire/blob/b0543c8278fe7a50e0ac9f938a936b4a84ee239b/man.txt
- Alex Shroyer의 J GPU/ArrayFire prototype 자료:
  https://alexshroyer.com/papers/matmul_j_gpu.pdf

ArrayFire 자체에서 참고할 핵심:

- elementwise 연산을 즉시 실행하지 않고 AST/lazy expression graph로 누적한 뒤 필요할 때 한 kernel로 JIT fusion한다.
- explicit `eval` 또는 JIT가 지원하지 않는 consumer가 evaluation boundary가 되며, `sync`는 평가 시작과 완료 대기를 구분한다.
- CUDA/OpenCL/oneAPI/CPU backend를 공통 array API 뒤에 두며 backend 선택은 array 계산 의미와 분리한다.
- device pointer, stream, lock/unlock, custom memory manager를 명시해 외부 kernel/library와의 ownership·lifetime·synchronization 경계를 관리한다.
- JIT compilation cache가 있으므로 cold compile cost와 warm execution cost를 분리해 측정해야 한다.

Jsoftware의 `math_arrayfire` add-on에서 특히 참고할 부분:

- 이것은 J 전체를 GPU compiler로 바꾸는 구현이 아니라 J에서 ArrayFire C API로 들어가는 **library adapter/offload 사례**다.
- J array는 row-major, ArrayFire array는 column-major이므로 add-on은 `rcc` 변환을 사용한다. 이는 logical atom order와 physical layout을 동일시하면 adapter 경계에서 불필요한 전역 변환 비용이 생길 수 있다는 실제 사례다.
- `families.ijs`는 `af_add`, `af_mul`, `af_sum` 같은 concrete ArrayFire function family를 직접 매핑한다. J의 일반 `/`, `\\`, rank, derived verb 의미 전체가 자동으로 ArrayFire op로 번역되는 구조는 아니다.
- add-on은 `af_array` handle을 별도 추적하고 release/hold/device GC를 관리한다. RustJ의 ValueId와 외부 backend buffer/handle을 분리해야 한다는 근거로 사용할 수 있다.
- add-on의 shape/rank validation은 ArrayFire `dim4` 경계에 맞춰 사실상 rank 4 이하를 전제로 한다. 이는 backend capability/precondition이지 J 언어의 rank 제한이 되어서는 안 된다.
- CPU/CUDA/OpenCL backend를 바꿔 쓸 수 있지만, backend 변경 자체가 J noun의 semantic identity를 바꾸지는 않는다.

RustJ 적용:

1. **ArrayFire는 Graph/Execution optimizer의 선행 구현 사례로 참고한다.**
   - lazy graph, evaluation boundary, fusion trigger를 참고하되 J Semantic IR 자체를 lazy ArrayFire AST처럼 축소하지 않는다.
   - fusion 여부와 materialization은 semantic legality가 확정된 뒤 Logical/Physical planning에서 결정한다.

2. **J ArrayFire add-on은 external-library route의 adapter 사례로 참고한다.**
   - `J logical value → adapter capability check → external array handle → execution → logical result` 경계를 설계할 때 직접 비교한다.
   - op coverage, dtype/rank/shape/layout 조건은 route precondition으로 명시한다.

3. **row-major/column-major mismatch를 Physical Planner 검증 사례로 사용한다.**
   - RustJ logical array는 layout-neutral하게 유지한다.
   - ArrayFire route가 column-major representation을 요구하면 view/consumer absorption/copy 중 어느 것이 합법적이고 싼지 physical plan에서 선택한다.
   - adapter 편의를 위해 J logical atom order를 바꾸지 않는다.

4. **evaluation/synchronization을 Physical Plan의 별도 개념으로 둔다.**
   - lazy value의 존재, kernel submission, device completion은 서로 다른 상태다.
   - 향후 AsyncToken/Timepoint, transfer, external library call의 legality와 lifetime 검증에 ArrayFire의 `eval/sync` 및 interop 경계를 비교한다.

5. **cost model과 benchmark 방법론에 cold/warm JIT를 분리한다.**
   - compile latency, kernel-cache hit, host/device transfer, layout conversion, intermediate materialization을 별도 비용 항목으로 본다.
   - 단순 warm-kernel 수치만으로 route profitability를 판단하지 않는다.

6. **Graph Basis ↔ external capability matrix를 만들 때 실물 비교 대상으로 사용한다.**
   - Elementwise, Reduce, Scan, Gather/Index, MatMul, Conv, Sparse 등 RustJ basis family가 ArrayFire API에서 직접 지원되는지,
   - J 의미를 그대로 보존하는지,
   - adapter shim 또는 fallback이 필요한지를 구분한다.

중요한 비채택 사항:

- ArrayFire `af::array`를 RustJ Logical Array/J noun과 동일시하지 않는다.
- ArrayFire의 rank/dim4 제한, column-major layout, dtype 범위를 J semantics에 역류시키지 않는다.
- ArrayFire의 fixed reduction API를 J의 일반 adverb `/` 또는 `\\` 의미론과 동일시하지 않는다.
- ArrayFire JIT가 fuse할 수 있다는 사실만으로 RustJ Graph rewrite/fusion의 semantic legality가 증명되었다고 보지 않는다.
- J add-on의 FFI 함수 매핑을 RustJ compiler architecture 자체로 채택하지 않는다.

따라서 ArrayFire는 RustJ의 GPU backend 후보 하나라기보다, **lazy array execution, kernel fusion, external-library routing, physical layout mismatch, device-handle lifetime과 synchronization을 동시에 검증할 수 있는 비교 기준**으로 다룬다.

### JAX / multi-device systems

참고:

- logical/global value와 placement/sharding 분리

RustJ 적용:

- logical J noun과 physical placement 분리

### MLIR core / interfaces / regions

참고:

- SSA value + Region/Block 구조로 pure graph와 control flow를 함께 표현
- verifier를 operation contract의 일부로 둠
- concrete op를 special-case하지 않고 operation/type/attribute interface를 통해 분석 capability를 질의
- side effect와 speculation safety를 별도 interface로 모델링
- data-flow analysis를 typed lattice와 monotonic join으로 구성

RustJ 적용:

- Logical IR에 SSA ValueId + Function/Region/Block/Terminator를 둔다.
- Primitive/LogicalOp capability를 Shape/Axis/Access/Effect/Alias/Speculation interface로 분리한다.
- 생성/변환 뒤 verifier를 필수 경계로 둔다.
- fact domain마다 typed lattice를 사용한다.

### MLIR Shape / dynamic constraints

참고:

- unknown shape와 invalid shape를 구분
- compile-time constraint가 증명되지 않으면 witness/assuming 구조로 의존성을 명시
- shape computation과 value computation을 분리하되 필요하면 runtime shape value로 reify

RustJ 적용:

- `ConstraintSet`만 metadata로 저장하지 않고 Witness/Guard를 둔다.
- dynamic specialization은 fast/fallback region으로 표현한다.
- semantic error가 증명된 Invalid와 단순 Unknown을 구분한다.

### MLIR Linalg / Bufferization

참고:

- indexing map + iterator type으로 structured computation을 표현
- tensor-level tiling/fusion/vectorization을 먼저 하고 bufferization을 늦춤
- Destination-Passing Style과 alias relation을 buffer reuse 분석의 입력으로 사용
- bufferization은 SSA use-def와 conflict 분석을 통해 실제 in-place/out-of-place를 결정

RustJ 적용:

- `IterationDomain + AccessRelation`을 Logical IR의 핵심 contract로 둔다.
- `StorageRequirement`와 실제 `MaterializationDecision`을 분리한다.
- `DestinationRelation`은 BufferId가 아니라 후속 bufferization hint/contract다.

### MLIR Transform dialect / TVM TensorIR schedule

참고:

- payload IR과 transformation/schedule description을 분리
- 같은 semantic computation에 여러 schedule을 적용 가능
- TVM은 graph-level Relax와 lower-level TensorIR/schedule을 구분하고 external codegen도 허용

RustJ 적용:

- Logical IR에 tile/vector/workgroup 결정을 박지 않는다.
- native route의 `Schedule / Transform Plan`을 별도 표현으로 둔다.
- MLIR/TVM류 external optimizer를 재구현하지 않고 adapter를 통해 활용할 수 있게 한다.
- external backend는 whole-program 선택이 아니라 legal subgraph/region partition으로 적용할 수 있게 한다.

### IREE Flow / Stream / HAL

참고:

- tensor dataflow(Flow), async scheduling/resource lifetime(Stream), hardware abstraction(HAL)을 분리
- resource size와 lifetime을 명시적으로 추적
- async execution은 timepoint로 availability/order를 표현
- allocation/reuse는 scheduling 뒤에 구체화

RustJ 적용:

- Logical ArrayValue와 physical resource를 분리한다.
- native Physical Plan의 async dependency는 explicit Timepoint/AsyncToken으로 표현한다.
- resource lifetime과 buffer reuse는 physical timeline을 기준으로 판단한다.

### StableHLO / VHLO

참고:

- portable high-level op set과 명시적 specification/verifier/type inference
- side-effecting op는 token으로 ordering 가능
- custom_call/composite로 확장 가능하지만 semantic contract가 필요
- portable artifact는 별도의 versioned VHLO/compatibility layer로 관리

RustJ 적용:

- StableHLO는 전체 J IR이 아니라 안전한 tensor/NN subset export target이다.
- external effect mapping은 token/adapter contract로 검증한다.
- RustJ Logical IR도 외부 interchange를 시작할 때 schema version과 migration 정책을 둔다.

### Triton

참고:

- tensor/block program과 backend schedule configuration을 분리
- block size, warp 수, pipeline stage, register limit은 semantic op가 아니라 compilation configuration
- layout/access constraint가 codegen 품질에 직접 영향

RustJ 적용:

- tile/warp/stage/register cap은 Physical Schedule/TargetProfile 쪽에 둔다.
- Logical IR에는 이를 선택할 수 있게 하는 axis/access/constraint fact만 유지한다.

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

앞으로 사람이 유지하는 프로젝트 기준 문서는 **이 `PROJECT.ko.md` 하나**다.

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
- `PROJECT.ko.md` 링크

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

앞으로 새 설계 결정을 이 네 저장소 중 하나에 먼저 기록하고 나중에 RustJ로 옮기는 workflow를 사용하지 않는다. **RustJ `PROJECT.ko.md`가 최초 기록 장소이자 최종 권위 문서**다.

기존 저장소는 prototype 코드, 연구 이력, 참고 구현을 확인할 때만 사용한다.

### 15.5.1 current jsource semantic cross-check (2026-09-30)

기준 revision: `jsoftware/jsource@13994ffa1ed5f06f79fad6e9822a7ed2d29b1528` (2026-09-30 current master at this review).

구현 기법을 그대로 복제할 필요는 없지만 다음 source-level semantic facts는 RustJ 설계 제약으로 채택한다.

| jsource 확인점 | RustJ 설계 결론 |
|---|---|
| `w.c` enqueue는 alphabetic word를 ordinary NAME으로 만들고 lookup flag/hint를 붙인다 | extension도 enqueue에서 ADV/VERB로 고정하지 않는다 |
| `p.c` parser가 NAME을 stack할 때 local/locale lookup하고 noun은 value, 일반 ACV는 nameref로 처리한다 | noun snapshot과 function-name late binding을 분리한다 |
| `sc.c` nameref 실행은 현재 lookup value의 part of speech가 reference 생성 시 기대한 품사와 같은지 검사한다 | `NameRef.expected_part_of_speech`를 보존하고 mismatch는 domain error로 처리한다 |
| `p.c`는 parse reduction 중 name lookup/verb execution/assignment를 수행한다 | 문장 전체 name snapshot을 만들지 않고 J의 우측→좌측 observable sequencing을 effect/name dependency로 보존한다 |
| `j.h::FORK1/FORK2`는 일반 fork에서 right tine을 먼저 실행하고 이후 left tine, middle verb 순으로 실행한다 | train value graph를 자동 병렬 독립으로 보지 않고 effect/name/error order edge를 proof 전까지 보존한다 |
| `wc.c`/`cx.c`는 `try./catch./catchd./catcht./throw.`를 linked control flow로 실행하고 error/throw를 handler로 전달한다 | J-visible errors를 항상 fatal diagnostic으로 취급하지 않고 exceptional CFG/control effect로 보존한다 |
| `c.c::ad12`의 `u::v`는 ordinary failure에서 fallback `v`를 실행/반환하지만 throw/exit는 전파한다 | adverse derived verb를 expression-level error-handler semantics로 보존한다 |
| `c.c::jtobverse`와 inverse logic은 `u:.v`의 second operand를 inverse semantics에 사용한다 | forward dataflow가 같아 보여도 obverse metadata/operand를 dead-code로 제거하지 않는다 |
| `cf.c`의 bident/trident tables는 noun/verb/adverb/conjunction 조합에서 verb 외에 derived adverb/conjunction도 생성한다 | Semantic IR을 DerivedVerb 중심으로 제한하지 않고 result POS를 가진 `DerivedEntity`로 일반화한다 |
| `p.c::cases[]` row 3/4/5/6은 adverb application, conjunction application, fork, hook의 parser reduction shape를 정의한다 | RustJ Semantic Function DAG의 parent/operand shape도 이 parse production을 기준으로 만든다 |
| `jtype.h`/`ja.h`의 `V` + `fdef/fdeffill`은 parser reduction 결과를 shared function block으로 realization하고 specialized executor를 선택한다 | shared graph 원칙과 semantic/executor 분리는 채택하되, `fgh`의 execution auxiliary를 source DAG child로 자동 복제하지 않는다 |
| rank/insert/fit/power/under 등은 parser의 ADV/CONJ application production을 통해 derived function이 되고, jsource execution object는 form별 auxiliary metadata를 추가할 수 있다 | RustJ semantic DAG는 operator identity + parser operands를 보존하고 execution-only auxiliary는 downstream attachment로 분리한다 |
| `cf.c`는 boxed noun을 특정 modifier 문맥에서 gerund로 해석한다 | gerund를 별도 전역 noun type으로 만들지 않고 boxed noun + contextual GerundSemantics로 보존한다 |
| parser assignment reduction은 assigned J entity를 parse stack/result에 남기면서 symbol table을 갱신한다 | assignment를 entity-producing effectful expression으로 모델링한다 |
| `cr.c`/rank conjunction은 negative requested rank를 argument rank에 상대적으로 resolve하고 infinite rank를 별도로 다룬다 | rank IR을 nonnegative integer 하나로 축소하지 않고 Infinite/Absolute/Relative `RankSpec`을 둔다 |
| `cr.c::jtqq`는 `Verb"RankNoun` 외에도 right Verb rank extraction과 left Noun gerund/constant-verb form을 처리한다 | J Semantic IR의 rank node를 verb+integer pair로 제한하지 않고 original entity operands를 보존한다 |
| `cr.c` rank dyad는 frame prefix agreement를 검사하고 residual frame에 cell을 반복한다 | NumPy broadcasting으로 대체하지 않는다 |
| `cr.c`는 zero cells에서 fill-cell을 실행해 result cell type/shape를 정한다 | zero-trip elimination 전에 fill/prototype semantics를 해결한다 |
| `result.h`는 rank/modifier의 cell results가 type/shape 불일치하면 homogeneous fast path에서 assembly path로 전환하고 type/shape join + framing fill을 수행한다 | rank map을 항상 static uniform tensor map으로 가정하지 않고 `RankAssemblySemantics`를 보존한다 |
| `cv.c`의 `!.`는 comparison tolerance 또는 fill을 바꾸는 derived verb를 만든다 | tolerance/fill override를 semantic contract로 보존한다 |
| `ar.c`는 일반 float reduction에 SIMD/multiple-accumulator 경로를 사용할 수 있고 `+/!.0`에는 compensated summation 경로가 있다 | exact source operation order를 blanket semantic invariant로 만들지 않고 primitive/fit별 numeric policy를 보존한다 |
| `va2.c`는 unsupported argument-type pair라도 empty operand가 있으면 notional safe type로 재해석하는 경로를 가진다 | type/domain inference를 dtype pair만으로 고정하지 않고 emptiness/fill context를 입력으로 받는다 |
| `va2.c`는 agreement/rank-shape 검사와 domain/type/value error의 precedence를 의도적으로 관리한다 | GPU parallel error reporting도 J error contract를 따른다 |
| `va2.c`는 retryable overflow를 retry/repair하고 result type consistency를 유지한다 | primitive/type별 overflow promotion을 lane-local 임의 처리로 바꾸지 않는다 |
| sparse가 AT/type 및 `$.`를 통해 J-visible하고 axes/element를 가진다 | sparse를 단순 physical compression format으로 보지 않는다 |
| BOX는 noun type이고 boxed atom이 J value를 담는다 | boxed를 physical encoding 목록에서 제외한다 |

이 검토에서 발견된 차이는 **포기한 jsource implementation detail이 아니라 observable semantics에 영향을 주는 항목만** 반영했다.

### 15.6 설계 일관성 불변식

앞으로 문서를 수정할 때 다음 불변식을 독립적으로 점검한다.

1. **J semantic structure 보존** — hook/fork/train/derived verb/rank는 Semantic Analyzer가 보기 전에 불필요하게 소거하지 않는다.
2. **Semantic Analyzer는 target-independent** — target capability, cost, tile/layout/device 선택은 analyzer 책임이 아니다.
3. **Logical IR은 target-independent structural-fact rich, schedule-free** — iteration/access/dependency/constraint처럼 hardware planning에 유용한 구조적 fact를 표현하지만 특정 target/warp/tile/buffer id는 갖지 않는다. 기존 ‘hardware-aware’는 이 뜻의 약칭이다.
4. **Primitive semantics와 realization 분리** — PrimitiveSpec/capability interface에 vendor resource 숫자를 넣지 않는다.
5. **Storage requirement와 materialization 분리** — logical persistence 요구와 실제 buffer allocation/copy를 같은 개념으로 쓰지 않는다.
6. **Schedule과 payload IR 분리** — fusion/tile/vectorization 선택은 native Schedule Plan이나 external compiler가 담당한다.
7. **Physical Planner는 Route A 전용** — external route가 RustJ Physical Plan을 반드시 거친다고 쓰지 않는다.
8. **Native fallback은 보장 아님** — 지원되는 native path가 있을 때만 fallback이며, 없으면 Unsupported가 정상 결과다.
9. **Effect와 error ordering 명시** — pure data dependency만으로 표현되지 않는 ordering은 effect token/speculation contract로 보존한다.
10. **Dynamic assumption은 witness/guard로 추적** — optimization이 암묵적 shape 또는 representation 가정에 기대지 않는다.
11. **Logical constraint와 representation fact 분리** — divisibility/shape 관계와 stride/alignment/address-space를 같은 fact domain에 넣지 않는다.
12. **Semantic mask와 predication 분리** — source/operation 의미의 mask만 Logical IR에 두고 tail/vector/workgroup mask는 schedule에서 만든다.
13. **Conflict semantics와 atomic realization 분리** — conflicting update를 Logical IR에 표현하되 atomic instruction 사용은 downstream이 결정한다.
14. **External IR은 projection** — RustJ Logical IR을 MLIR/StableHLO의 표현력에 맞춰 축소하지 않는다.
15. **Late bufferization** — alias/destination contract는 logical에 둘 수 있지만 BufferId/materialization은 downstream에서 정한다.
16. **Verifier first** — 잘못된 IR을 downstream이 추측해서 복구하게 하지 않는다.
17. **Version boundary 명시** — external interchange를 시작하면 IR schema와 registry/compiler provenance를 기록한다.
18. **Async dependency는 explicit** — physical async execution에서 host statement order를 dependency로 암묵 사용하지 않는다.
19. **Resource와 cost 분리** — ResourceEstimate는 resolved TargetProfile hard facts에 의존하고, empirical CostProfile은 CostEstimate/ranking에만 사용한다.
20. **Route는 혼합 가능** — external/native route는 whole-program exclusive choice가 아니라 legal region/subgraph 단위로 partition할 수 있다.
21. **Analysis state와 semantic error 분리** — lattice의 unknown/unreachable과 J의 domain/rank/length error를 같은 상태로 표현하지 않는다.
22. **RoutePartition은 plan** — route 배정은 Logical IR semantic identity가 아니며 target/backend 조건에 따라 재계산 가능하다.
23. **Extension name은 keyword가 아니다** — ordinary J binding/품사 해소를 사용하고 tokenizer/parser spelling special-case를 만들지 않는다.
24. **Surface builder와 derived op를 구분** — parameterized adverb identity와 그 결과 computational verb/LogicalOp identity를 같은 것으로 취급하지 않는다.
25. **Full J semantics와 analyzable array profile 분리** — advanced compiler contract가 없다는 이유만으로 valid J semantics를 부정하지 않는다.
26. **Mutable state externalization** — weight/grad/optimizer/checkpoint 같은 mutable array state를 primitive/verb hidden field에 숨기지 않는다.
27. **SSA는 namespace의 대체물이 아니다** — runtime J name lookup/assignment가 필요한 곳을 무리하게 SSA binding으로 고정하지 않는다.
28. **Definition code와 invocation frame 분리** — 재귀/동시 호출이 local values를 공유하지 않게 한다.
29. **J validity와 compilation eligibility 분리** — advanced lowering이 없다는 이유로 valid J program을 semantic error로 분류하지 않는다.
30. **Prototype implementation language는 architecture가 아니다** — Python registry/CUDA-family field 같은 역사 구현 선택을 RustJ semantic boundary로 승격하지 않는다.
31. **Alias legality와 in-place realization 분리** — `MayReuse` 가능성과 실제 buffer reuse 결정을 같은 bool로 표현하지 않는다.
32. **Array value와 storage identity 분리** — 모든 J 데이터가 array라는 의미론과 SSA ValueId/StateResource/BufferId의 compiler identity를 혼동하지 않는다.
33. **Computational built-in도 hardware lowering 대상** — hardware-aware compilation 대상인 기존 J computational entity와 extension-derived op를 같은 lowering/capability architecture에서 다룬다. namespace/control/system-foreign semantics를 억지로 pure GPU op로 만들지는 않는다.
34. **Backend/architecture/device 분리** — CUDA/ROCm 같은 backend, ISA/microarchitecture target, exact device capacity를 하나의 profile identity로 뭉개지 않는다.
35. **Target locale path는 명시적** — device→architecture→family→backend→class→generic 순서는 compiler namespace의 explicit resolution path이며 숫자 버전 상속으로 추론하지 않는다.
36. **Locale lookup과 candidate selection 분리** — locale은 lowering/capability 후보를 찾고, 실제 realization 선택은 legality/resource/cost 분석이 한다.
37. **Ordinary name 품사는 parser-time lookup** — enqueue가 local/locale name의 noun/verb/adverb/conjunction class를 미리 고정하지 않는다.
38. **Function names preserve late binding** — verb/adverb/conjunction NameRef를 static proof 없이 현재 value identity로 얼리지 않는다.
39. **J agreement is prefix agreement** — NumPy trailing broadcasting으로 대체하지 않고 rank/cell/frame repetition 의미를 보존한다.
40. **Empty execution has fill-cell semantics** — zero-trip이라는 이유로 prototype/type/shape/error 의미를 생략하지 않는다.
41. **Boxed/sparse are J-visible semantics** — boxed hierarchy와 sparse axes/element를 단순 physical encoding으로 취급하지 않는다.
42. **Tolerance/Fit are semantics** — comparison tolerance와 `!.`에 의한 numeric/fill variation을 backend optimization에서 잃지 않는다.
43. **Floating order is contract-driven** — 일반 J float 연산의 exact scalar execution order를 전역 불변식으로 가정하지 않고, primitive/derived verb의 reassociation·accuracy·compensated semantics를 따른다.
44. **Error contract is observable semantics** — J가 정한 precedence/suppression/retry를 보존하고 parallel first-error를 임의로 노출하지 않는다.
45. **Nameref keeps expected POS** — late lookup은 허용하지만 reference 생성 시의 verb/adverb/conjunction 품사 계약을 버리지 않으며 mismatch는 J의 domain error semantics를 따른다.
46. **Sentence environment is not pre-snapshotted** — 우측→좌측 evaluation 중 name lookup/assignment/locale mutation의 observable sequencing을 보존한다.
47. **Rank map is not always fixed-shape** — per-cell result type/shape uniformity를 증명하지 못하면 J의 result assembly/type join/framing fill semantics를 보존한다.
48. **Assignment is entity + effect** — `=.`/`=:`를 void statement로 낮추지 않고 binding mutation과 assigned J entity result를 함께 보존한다.
49. **Type semantics may depend on emptiness** — dtype pair만으로 domain/promotion을 확정하지 않고 J의 empty/fill/prototype context를 반영한다.
50. **Rank is not just usize** — infinite rank와 argument-relative negative rank를 semantic RankSpec으로 보존하고 적용 시 effective cell rank를 resolve한다.
51. **Rank conjunction is entity-based** — `"`의 left/right operand를 verb+integer로 가정하지 않고 J의 noun/gerund/verb-rank forms를 semantic analysis 전까지 보존한다.
52. **Lowering key includes semantic valence/context** — raw primitive id/spelling만으로 backend lowering을 선택하지 않고 resolved valence와 derived numeric/rank/effect semantics를 포함한 operation key를 사용한다.
53. **Innate rank is valence-specific** — primitive rank를 단일 값으로 두지 않고 monad와 dyadic left/right rank contract를 분리한다.
54. **J errors may be control flow** — try/catch/throw 영역 안의 observable error를 fatal diagnostic으로 접지 않고 exceptional successor/동등 runtime semantics를 보존한다.
55. **Train graph does not imply branch independence** — hook/fork/train의 value graph가 병렬 가능해 보여도 J의 name/effect/error execution order를 proof 없이 제거하지 않는다.
56. **Forward equivalence is not full derived-verb equivalence** — `::`, `:.` 등 modifier가 붙인 error/inverse/latent semantics를 현재 forward dataflow가 같다는 이유로 소거하지 않는다.
57. **Derived entity is not always a verb** — parser가 생성할 수 있는 derived adverb/conjunction의 result POS와 operands를 J Semantic IR에서 표현한다.
58. **Gerund is contextual noun semantics** — boxed noun을 전역적으로 gerund type으로 바꾸지 않고 modifier가 요구할 때 gerund interpretation을 적용한다.
59. **Architecture inventory is not a blocking milestone** — TargetProfile/resource/mixed-route의 장기 설계를 유지하되 A1→A3-v0→최소 CPU vertical slice를 먼저 증명한다.
60. **Unknown access is an optimization barrier, not a semantic error** — AccessRelation이 Opaque여도 valid J semantics와 conservative/runtime lowering 가능성을 유지한다.
61. **Mixed route is staged** — architecture는 mixed route를 허용하지만 첫 external implementation은 verified single-block region 전체를 한 route로 보낸다.
62. **Parser rules define the Function DAG** — ADV/CONJ application과 hook/fork의 parent/operand shape는 jsource parser reduction 규칙을 따르며 modifier별 독자 AST shape를 발명하지 않는다.
63. **Parse operands and executor auxiliaries are separate** — semantic DAG에는 parser operands/J semantics를 보존하고 jsource `fgh/localuse`의 실행 최적화 보조 객체를 자동 semantic child로 승격하지 않는다.
64. **Semantic modifier syntax is not LogicalOp syntax** — `/`, `"` 등의 parser-produced operator DAG를 Semantic IR에서 `Reduce`/`CellApply`로 조기 치환하지 않는다. normalized op는 Semantic Analyzer 이후에만 만들며, `MapCells`는 uniform-result proof 뒤의 제한된 lowering form이다.
65. **Parser production coverage is explicit** — 미지원 jsource parse row/form을 임의의 대체 AST로 해석하지 않고 coverage manifest에 pending으로 남긴다.
66. **Rank conjunction and implicit loop are distinct** — `"`는 parent conjunction + two operands로 derived function을 만들고, cell iteration은 function application의 공통 semantics다.
67. **Rank boundaries are semantic until proven fusible** — nested explicit/innate rank boundaries를 effective rank 하나로 early collapse하지 않는다.
68. **CellApply is logical, not physical** — frame iteration/repetition/assembly를 logical CellApply로 표현하고 CPU/GPU loop/thread mapping은 downstream이 선택한다.
69. **IRS means loop absorption, not different semantics** — jsource IRS와 같은 fast path는 generic CellApply와 동등해야 하며 lowering capability로 표현한다.
70. **Implicit loop owns empty and assembly semantics** — fill-cell prototype과 heterogeneous result assembly는 primitive kernel의 우연한 동작이 아니라 CellApply contract다.

이 목록과 충돌하는 문장이 생기면 더 오래된 문장을 유지하지 말고 권위 설계를 이 불변식에 맞춰 갱신한다.

---

## 16. 다음 작업

현재 가장 먼저 증명해야 하는 것은 전체 compiler stack이 아니라 **J semantic structure → verified Logical IR → 최소 CPU 실행**의 얇은 수직 슬라이스다.

장기 architecture를 유지하되 구현 순서는 다음처럼 좁힌다.

### 16.1 Proof Slice 0 — A1 semantic preservation

1. 현재 `semantic.rs`, `analysis.rs`, `facts.rs`, `contracts.rs`의 실제 책임과 목표 architecture의 차이를 표로 만든다.
2. `semantic.rs`가 noun/verb/adverb/conjunction, hook/fork/train, derived entity, gerund interpretation, rank, nameref를 얼마나 보존하는지 감사한다.
3. 첫 slice의 `J Semantic Array IR`은 jsource parser row 3/4/5/6가 만든 shared `FunctionEntity` graph로 명시한다. modifier별 special semantic node를 추가하지 않는다.
4. 다음 semantic golden을 우선 고정한다.
   - primitive valence
   - reduction-derived verb
   - fork/train 구조 보존
   - rank form 보존
   - noun snapshot vs function late binding
   - assignment entity+effect / same-sentence lookup

**성공 기준:** scanner/parser 없이 handcrafted J Semantic IR을 Semantic Analyzer에 넣어 테스트할 수 있고, 분석 전에는 J composition identity가 사라지지 않는다.

### 16.1.1 현재 Proof Slice 0 진행 상태

현재 코드에 다음 migration seam이 들어갔다.

- [x] primitive/name function identity를 `Arc<FunctionEntity>` shared handle로 표현
- [x] lexer가 `/`를 ADV, `"`를 CONJ로 분류하고 parser row 3/4의 현재 지원 subset이 operator-parent graph를 만든다
- [x] isolated pure verb train을 jsource 규칙대로 오른쪽부터 fork, 짝수 길이는 최종 hook으로 구성
- [x] hook/fork graph가 큰 train에서 subtree deep-copy 없이 공유됨
- [x] Semantic Analyzer가 canonical fork `(+/ % #) y`를 Tally / reduction-semantics(+) / Divide dataflow로 낮추면서 jsource-compatible observable order edge를 보존
- [x] nested hook/fork long train도 동일 graph lowering path를 사용
- [x] A3-v0 `LogicalPlan::verify()` 기초와 `AccessFact::Known | Opaque` seam 존재

아직 남은 A1 핵심:

- [ ] ordinary name parser-time POS를 noun/verb/adverb/conjunction 전체로 일반화
- [ ] jsource row 3/4의 NOUN operand forms와 full result-POS rule을 구현
- [ ] row 5/6을 pure edge phrase helper가 아니라 일반 parser-stack production으로 확대하고 mixed-sentence precedence를 differential test로 고정
- [ ] current `Callable.reduce/rank` migration fields를 normalized LogicalOp/Rank facts로 제거
- [ ] derived adverb/conjunction 및 modifier train
- [ ] built-in/extension ADV·CONJ operator identity/POS registry와 typed semantic contract interface
- [ ] legacy `VerbTarget/reduce/rank` runtime compatibility field 제거
- [ ] function graph 안의 rank/fit/adverse/obverse 등 더 많은 J form을 registry-driven analysis로 이전
- [ ] explicit definition/locale/gerund 전체 의미

현재 `FunctionEntity.operands`는 v0 단순성을 위해 `Vec`를 사용한다. 대부분 parser productions가 1–3 semantic operand를 갖지만, jsource `fgh[3]`의 slot layout 자체를 semantic ABI로 채택하지 않는다. **inline-small-vector + rare spill** 같은 저장 최적화는 large-train memory profile을 측정한 뒤 적용할 수 있다.

### 16.2 Proof Slice 1 — A3-v0 verified Logical IR

Implicit loop는 이 단계에서 backend loop로 만들지 않고 `CellApplyPlan`/logical `CellApply` seam을 먼저 추가한다. explicit `"` parent-conjunction 구조와 callable innate rank가 같은 planner를 사용해야 한다.

5. single Function / single Region / single Block의 SSA `ValueId` core와 verifier를 만든다.
6. v0 fact는 다음으로 제한한다.

```text
shape / dtype / valence
IterationDomain: Map | Reduce
AxisSemantics: minimum required subset
AccessFact: Known(simple relation) | Opaque
NumericSemantics: minimum
EffectSummary / SpeculationSemantics
ConstraintSet + compile-time Witness: minimum
```

7. `Unknown/Opaque` access는 semantic failure가 아니라 access-sensitive optimization barrier로 처리한다.
8. Semantic Analyzer가 target 정보 없이 J Semantic IR + minimal capability를 읽어 single-block Logical IR을 만들게 한다.
9. operation verifier가 malformed IR, fact contradiction, illegal valence/input relation을 거부하게 한다.

**첫 canonical proof example:**

```j
(+/ % #) y
```

```text
J Semantic IR
  Fork
  ├─ / : Verb
  │   └─ + : Verb
  ├─ % : Verb
  └─ # : Verb
        ↓ Semantic Analyzer
Logical IR
  Reduce(+)
  Tally
  Divide
```

source/semantic provenance와 필요한 numeric/rank contract를 유지한다.

### 16.2.1 Canonical E2E audit — `(+/ % #) y`

이 표현을 compiler pipeline의 고정 추적 표본으로 사용한다. 단순 vector 입력만 사용하면 implicit cell application이 드러나지 않으므로 두 입력을 구분한다.

```j
y =: 1 2 3 4
(+/ % #) y
```

은 parser/train/lowering smoke test로 사용한다. 더 강한 E2E semantic test는 다음이다.

```j
y =: i. 2 3
(+/ % #) y
```

J 의미상:

```text
+/ y  -> 3 5 7
#  y  -> 2
3 5 7 % 2 -> 1.5 2.5 3.5
```

current jsource는 이 source structure를 fork로 유지한다. `cf.c::jtfolk`는 `f=+/`, `g=%`, `h=#`인 fork를 인식해 monadic executor를 `jtmean`으로 specialize하지만, derived entity 자체는 `CFORK`와 원래 f/g/h operands를 유지한다. `ar.c::jtslash`가 만든 `+/`의 monadic rank는 infinite(`RMAX`)이고, primitive table에서 `#`의 monadic rank도 infinite, `%`의 dyadic left/right rank는 `0 0`이다. 따라서 matrix 입력에서는 마지막 `%`가 다음 implicit cell application을 요구한다.

```text
left  = +/ y : shape [3], dyad cell rank 0
right = #  y : shape [],  dyad cell rank 0

CellApply2
  left_frame  = [3]
  left_cell   = []
  right_frame = []
  right_cell  = []
  agreement   = prefix
  repetition  = repeat right scalar over left residual frame
  result      = shape [3]
```

2026-09-30 current RustJ의 실제 단계별 상태:

| 단계 | 현재 표본에서의 상태 | 실제 표현 / 문제 |
|---|---|---|
| scanner / word formation | 연결됨 | `+`, `/`, `%`, `#`를 별도 J word로 형성 |
| token/POS classification | 연결됨 | `+`/ `%`/ `#` = Verb, `/` = Adverb |
| modifier parser reduction | 연결됨 | `+ /` -> applied adverb FunctionEntity, parent identity는 `/` |
| train construction | 이 표본에서 연결됨 | `+/ % #` -> shared `Fork(f=+/, g=%, h=#)` graph |
| noun binding | 연결됨 | analysis path에서 `y`는 `ReadName + NameVersion` |
| Semantic Analyzer structural lowering | 연결됨 | fork 실행 의미를 h -> f -> g 순으로 `Tally`, reduction `+`, `Divide` call dataflow로 생성 |
| fact inference | 부분 연결 | shape/dtype 일부와 `ReduceLeadingAxis`/elementwise access를 계산 |
| innate rank contract | **미구현** | primitive/derived callable의 monad/dyad-left/dyad-right `RankSpec`이 contract에 없음 |
| implicit CellApply | **미구현** | matrix case의 `Divide([3], scalar)`가 logical `CellApply2`로 나타나지 않음 |
| current rank fact | migration-only | `facts::RankPlan`은 explicit `Callable.rank`가 있을 때만 생김; plain `%`의 innate rank-0 iteration은 기록하지 않음 |
| normalized LogicalOp | 부분 구현 | 아직 `Operation::Call + Callable.reduce` 중심이며 canonical `Reduce` / `CellApply` payload op로 분리되지 않음 |
| LogicalPlan verifier | 기초만 존재 | ID/span/fact-shape/order 참조를 검증하지만 valence/rank/CellApply/agreement semantic legality verifier는 아직 없음 |
| target-independent canonicalization | 미구현 | mean pattern 같은 rewrite가 없음 |
| RoutePartition | 미구현 | 실제 route module/plan 없음 |
| native Schedule / Physical Plan | 미연결 | `physical.rs`의 G1 affine representation은 존재하지만 LogicalPlan에서 생성되지 않음 |
| LogicalPlan executor | **없음** | `analysis.rs`가 inspection-only이며 execute-plan API가 없음 |
| legacy direct runtime | 별도 경로 | Semantic Analyzer/LogicalPlan을 우회하며 current `resolve_verb`는 `VerbTarget::Derived` fork를 실행하지 못함 |
| true compiler E2E | **아직 성립하지 않음** | source -> verified Logical IR까지가 현재 연결된 compiler slice |

특히 현재 analyzer는 matrix case에서도 `ShapeRule::PrefixAgreement`만으로 마지막 `Divide`의 result shape `[3]`을 추론할 수 있다. 그러나 **shape가 맞는 것과 J cell-iteration semantics를 표현한 것은 다르다.** 현재 `rank_plan=None`인 채 elementwise map으로 보이는 것은 최종 설계에서 허용할 수 없는 migration 상태다.

따라서 이 표본의 첫 실제 compiler E2E proof 순서는 다음으로 고정한다.

1. primitive/derived callable에 valence별 innate `RankSpec`을 연결한다.
2. explicit `"`와 innate rank가 함께 사용하는 pure `CellApplicationPlanner`를 만든다.
3. matrix `(+/ % #) y`의 마지막 `%`가 위의 `CellApply2`를 생성하는 golden test를 추가한다.
4. `+/`를 migration bool이 아닌 normalized `Reduce(+)`로 내리고 `#`/ `%`와 함께 verified Logical IR을 만든다.
5. verifier가 CellApply의 frame/cell split, prefix agreement, repetition relation, input/result fact consistency를 검증하게 한다.
6. 이 single-region Logical IR을 기존 CPU kernels에 연결하는 최소 native executor를 만든다.
7. generic `Reduce + Tally + CellApply(Divide)` 결과를 jsource와 differential test한다.
8. 그 뒤에만 `(+/ % #)` -> fused Mean 후보를 Logical Optimizer/native lowering에 추가한다. 이 specialization은 semantic Fork graph를 대체하지 않고 generic path와 의미 동등성이 검증되어야 한다.

기존 `tests/analysis.rs::canonical_mean_fork_lowers_to_reduce_tally_divide_in_jsource_order`는 parser/analyzer ordering smoke test로 유지하되, **compiler end-to-end 성공 증거로 간주하지 않는다.** 그 테스트는 `analyze + verify`까지만 호출하며 실행 경로를 통과하지 않는다.

### 16.3 Proof Slice 2 — 최소 CPU end-to-end

10. A2 전체를 구현하지 말고 **A2-v0 capability subset**만 연결한다.
11. 먼저 dyadic `+`와 dense numeric reduction 한 개를 native CPU lowering에 연결한다.
12. 최소 RustJ-native execution path를 만든다.

```text
Source
 → J Semantic IR
 → Semantic Analyzer
 → verified Logical IR
 → minimal native CPU lowering/executor
 → J Value
```

13. 기존 RustJ/reference execution과 differential test한다.
14. 이 slice에 필요한 G3/G4 항목만 구현한다. full TargetProfile, ResourceEstimate, mixed route는 blocking requirement가 아니다.

**성공 기준:** architecture diagram의 핵심 경계가 실제 코드에서 한 번 end-to-end로 통과한다.

### 16.4 Semantic hard cases를 golden으로 잠근다

15. 가능한 순서대로 다음을 differential/golden test로 추가한다.

- prefix agreement
- empty fill-cell / empty type semantics
- rank result assembly
- nameref expected-POS mismatch
- assignment entity+effect와 right-to-left lookup sequencing
- fork/hook observable effect order
- adverse/obverse latent semantics
- tolerance/`!.`
- overflow retry/promotion + error precedence

모든 항목을 첫 vertical slice 전에 완성할 필요는 없다. 다만 해당 semantic feature를 optimization/lowering 대상으로 열기 전에 대응 golden이 있어야 한다.

### 16.5 그 다음 확장

16. G2 structural view 또는 첫 external adapter 중 하나를 선택해 다음 proof slice로 진행한다.
17. external route를 선택하면 처음에는 **verified single-block region 전체를 한 route**로 내린다. elementwise + reduction + static reindex 수준의 MLIR adapter와 MLIR verifier/differential test부터 시작한다.
18. mixed contiguous-subgraph RoutePartition과 boundary bridge는 external 단일-route가 안정된 뒤 추가한다.
19. `CompilationTarget` MVP, `TargetFacts + TargetQueries`, target locale chain은 실제 target-dependent planning이 필요해지는 시점에 구현한다.
20. RustJ-native Schedule/Physical Plan의 richer tile/memory-space/synchronization 모델도 그 시점에 확장한다.
21. `ResourceEstimate`, `CostEstimate`, `CompiledResourceReport`, runtime measurement feedback/re-plan은 실제 schedule 후보가 둘 이상 생긴 뒤 구현한다.
22. StableHLO export는 의미가 정확히 맞는 tensor/NN subset부터 별도 adapter로 검토한다.
23. branch/loop/try-catch-throw exceptional CFG/effect token은 A3-v1에서 추가한다.
24. portable serialization/version migration과 async timepoint는 필요 시 A3-v2/physical extension으로 추가한다.
25. 기존 LogicalPlan 결과와 새 pipeline의 의미 동등성을 계속 비교한다.

### 16.6 구현 단계와 장기 architecture를 혼동하지 않는다

다음은 **장기 architecture invariant**이지 첫 proof slice의 완료 조건이 아니다.

- CPU/GPU는 동일한 semantic/logical architecture의 peer target이다.
- mixed route가 가능하다.
- TargetProfile/TargetQueries와 resource/cost feedback 구조가 존재할 수 있다.
- full J semantics와 analyzable compiler profile을 분리한다.

반대로 현재 implementation claim은 보수적으로 쓴다.

- G1 physical boundary foundation은 존재한다.
- A1/A2/A3는 prototype 요소가 있으나 목표 architecture는 아직 미완성이다.
- GPU 관련 절은 현재 **계약/확장 경계**이며 GPU end-to-end 완료를 뜻하지 않는다.

특히 세 가지 shortcut을 금지한다.

- hook/fork/train/adverb/conjunction 정보를 “generic하게 만들기 위해” semantic analysis 이전에 소거하지 않는다.
- 외부 IR을 쓰기 쉽도록 RustJ Logical IR을 외부 IR의 표현력에 맞춰 축소하지 않는다.
- optimization을 쉽게 하려고 semantic storage/effect/error ordering을 physical schedule 정보와 섞지 않는다.


---

## 라이선스 정책

RustJ의 공개 오픈소스 배포 경로는 GNU General Public License version 3, 즉 `GPL-3.0-only`이다.

동시에 RustJ는 current `jsource`와 같은 방향으로 별도 상용 라이선스 경로를 유지한다. 단, 상용 RustJ 라이선스는 RustJ 저작권자가 실제로 재라이선스할 수 있는 권리 범위에서만 제공한다.

특히 RustJ의 상용 이용·배포가 J SOURCE에서 유래한 코드나 Jsoftware의 권리에 의존하는 경우에는, 필요한 범위의 Jsoftware 상용 J SOURCE 라이선스 및 기타 upstream 권리를 별도로 확보하고 그 계약 조건을 준수해야 한다. RustJ의 `LICENSE`는 Jsoftware가 보유한 상용 권리를 대신 부여하지 않는다.

따라서 현재 정책은 다음과 같다.

- 필요한 Jsoftware 상용 권리를 확보하지 않은 공개 RustJ 배포: `GPL-3.0-only`
- 필요한 Jsoftware 상용 권리와 RustJ 측 권리가 모두 확보된 경우: 별도 RustJ 상용 라이선스 제공 가능
- 외부 기여물은 상용 재라이선스가 가능하도록 `CLA.ko.md`에 따른 권리를 프로젝트에 부여해야 한다.
- CLA는 기여자의 저작권을 프로젝트로 양도하지 않는다. 대신 GPL 배포와 별도 상용 라이선스에 필요한 재라이선스 권리를 부여한다.
- CLA는 Jsoftware 또는 다른 제3자의 권리를 확장하지 않는다.

`LICENSE`가 라이선스 고지의 기준이고, `COPYING`은 GNU GPL v3 전문을 보존한다. Cargo의 `license` 메타데이터는 공개 오픈소스 선택지를 나타내기 위해 `GPL-3.0-only`로 유지하며 `GPL-3.0-or-later`로 변경하지 않는다.
