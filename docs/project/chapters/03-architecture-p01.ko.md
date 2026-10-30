[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)


<a id="read-architecture"></a>
<a id="2-최상위-아키텍처"></a>
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
J Parser + Semantic Construction  [하나의 통합 프런트엔드 단계]
   │ Queue/Stack reductions · parser-time name/POS lookup
   │ adverb/conjunction actions · hook/fork/train construction
   │ noun/verb construction · 필요한 binding/runtime-host actions
   ↓
J Semantic Construction IR / JEntity / FunctionEntity
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

**프런트엔드 레이어 경계(2026-10-07 교정):** 여기서 Word Formation과 Enqueue는 파서에 입력을 만드는 *처리 단계*이며, **J Parser와 Semantic Construction을 서로 독립적으로 완료되는 두 compiler pass로 정의하지 않는다.** jsource의 파서 규칙은 queue/stack 이동과 grammar action을 반복하면서 adverb/conjunction의 파생 함수 구성, verb 적용, name lookup 및 assignment 등 **파싱과 의미 동작/실행을 엮어 수행**한다. RustJ의 `src/parser.rs` 역시 reductions와 함께 `JEntity`/`FunctionEntity`를 구성한다. `src/semantic.rs`는 의미 객체·binding/version의 **타입/계약 모듈**이지 파싱이 끝난 뒤 별도로 반드시 실행되는 Semantic Construction pass가 아니다. 다만 RustJ 분석용 `parse`는 임의의 noun kernel을 실행하지 않고 의미 구조를 구성하며, runtime parser/host 경로의 관찰 가능 동작은 jsource와 맞춰야 한다. 이후 `J Graph IR`와 `Execution Semantic Lowering`은 **별개의 compiler 단계**로 유지한다.

핵심 원칙은 **J의 고수준 배열 변환 구조를 Semantic Analyzer가 보기 전에 없애지 않고, analyzer/lowering 단계가 그 구조를 분석한 뒤 backend-independent logical dataflow로 낮추는 것**이다.


<a id="frontend-결과물과-j-graph-ir-결과물의-차이"></a>
### Frontend 결과물과 J Graph IR 결과물의 차이

이 경계는 RustJ를 읽을 때 가장 쉽게 혼동되는 부분이므로 명시적으로 구분한다. **둘은 같은 프로그램을 서로 다른 질문으로 표현하며, 서로 대체하는 IR이 아니다.**

- **Frontend / J Semantic Construction의 질문:** “이 J 표현은 무엇을 의미하는가?”
- **J Graph IR의 질문:** “그 의미를 실제 noun input에 적용하면 어떤 배열 계산과 dependency/topology가 생기는가?”
- **Execution Semantic Lowering의 질문:** “그 계산을 어떤 explicit execution operation, check, effect/error order로 표현할 것인가?”
- **Physical planning의 질문:** “그 실행 의미를 어떤 kernel, buffer, layout, device와 schedule로 실현할 것인가?”

여기서 이해를 돕기 위해 “frontend AST”라고 부를 수는 있지만, 현행 RustJ의 기준 산출물은 일반적인 syntax AST가 아니라 parser reduction이 구성한 **`JEntity` / `FunctionEntity` semantic DAG와 관련 frontend context**다. 이 층은 J의 품사와 결합 의미, primitive/derived function, adverb/conjunction operand, hook/fork/train, rank, name/binding/version과 source provenance를 보존한다. 아직 fusion, materialization, kernel 또는 device를 결정하지 않는다.

반면 J Graph IR은 완성된 function/entity가 input에 **적용된 계산**을 대상으로 한다. 따라서 source combinator를 그대로 보존하는 것에 그치지 않고 producer/consumer, branch/join, rank/cell boundary, reduction과 같은 배열 계산 topology를 first-class graph form/basis로 드러낸다. shape/type/rank fact와 optimization hint를 붙일 수 있지만, 이 단계 역시 physical plan은 아니다.

| 구분 | Frontend / J Semantic Construction | J Graph IR |
|---|---|---|
| 중심 대상 | J entity와 결합 의미 | 적용된 array operation graph |
| 주인공 | noun/verb/adverb/conjunction, derived function, hook/fork/train, rank operands, names | input/output value, producer/consumer, branch/join, GraphForm/GraphBasis, facts/hints |
| 보존해야 할 것 | J parser/semantic structure와 source provenance | J 의미에 근거한 계산 topology와 graph provenance |
| 아직 하지 않는 것 | graph optimization, physical execution 결정 | kernel/buffer/layout/device/schedule 결정 |
| 다음 단계 | J Graph IR construction | Execution Semantic Lowering |

<a id="sql-graph-logical-physical-analogy"></a>
**SQL 옵티마이저에 빗댄 구분:** Graph IR을 **관계대수적 동등 계산 탐색**, Logical IR을 **선택된 계산의 명시적 논리 계획**, Physical planning을 **구체 실행 알고리즘 결정**에 대응시키면 이해하기 쉽다.

| SQL에서의 역할 | RustJ 단계 | Conv2D의 예 |
|---|---|---|
| 관계대수식·논리적 재작성 | **Graph IR / Algebraic Expansion** | 고수준 Conv를 유지하며 Window+Contract/MatMul 후보를 비실행 관계식으로 탐색 |
| 논리적 실행 계획 | **Verified Logical IR / Logical Decomposition** | WindowView·OrderedContract 등의 연산, SSA 의존성, Rank·오류·효과 검사와 순서 명시 |
| Hash/Merge/Nested Loop Join의 구체 알고리즘 선택 | **Route / Schedule / Physical Plan** | Direct Conv/implicit GEMM/tiled im2col, buffer·layout·device 선택 |

**SQL VIEW 전개가 조인 실행은 아니듯**, Graph의 `Algebraic Expansion`도 실제 윈도우 생성이나 implicit loop가 아니다. Logical의 `Logical Decomposition` 역시 실제 버퍼를 강제하지 않는다. J IR이 SQL 관계대수와 동일하다는 뜻도 아니다. J의 NAME, Rank, empty/fill, floating reduction, first-error/effect 제약으로 인해 각 후보에는 별도 합법성 증명이 필요하며, Conv2D 전 경로는 아직 설계 단계다. 단계별 **계산 가능한 예제**는 [`1.1 배열 연산의 SQL](./14-framework-research-p01.ko.md#sql-ir-planning-analogy)에 있다.

**Rank 예:** `(f"1) y`에서 frontend는 `"`를 `f`의 단순 attribute로 접어 넣지 않는다. Rank는 conjunction이고, 왼쪽 function operand `f`와 오른쪽 rank operand `1`을 받아 derived function을 만들며, 그 derived function이 `y`에 적용된다는 J 의미 구조를 보존한다. 실제 `FunctionEntity` 형태는 jsource parser reduction 규칙을 따르며, 아래 표기는 설명용 개념도다.

~~~text
frontend semantic view

Rank conjunction (")
  ├─ left operand:  f
  └─ right operand: 1
          ↓
     derived verb
          ↓ apply
          y
~~~

J Graph IR로 가면 관심점이 “Rank conjunction을 어떻게 썼는가”에서 “어떤 cell/frame 적용 계산이 생기는가”로 이동한다. 현재 graph 표현에서는 이 의미가 예를 들어 `GraphForm::Rank { .. }`와 `GraphBasisKind::CellApply` 같은 구조로 드러날 수 있다.

~~~text
J Graph view

y
│
▼
Rank / CellApply
│  function = f
│  cell rank = 1
▼
result
~~~

따라서 frontend에서는 implicit rank semantics를 **J 언어 구조로 보존**하고, Graph IR에서는 optimizer가 분석할 수 있도록 **cell-application boundary와 데이터 흐름을 노출**한다. 빈 frame에서의 virtual cell/fill 같은 J Rank 의미도 physical loop로 조기에 지워서는 안 된다.

**Fork 예:** `(f g h) y`에서 frontend는 fork를 `FunctionEntity`의 J 결합 구조로 보존한다. J Graph IR에서는 같은 의미가 input `y`의 fan-out, `f(y)`와 `h(y)`의 독립 producer, 그리고 `g`의 join으로 드러난다. 현재 graph 표현의 `GraphForm::Fork { .. }`는 이런 topology를 분석하기 위한 surface다.

~~~text
          y
         / \
        /   \
       ▼     ▼
     f(y)   h(y)
        \   /
         \ /
          ▼
    g(f(y), h(y))
          │
          ▼
        result
~~~

이렇게 graph가 되면 common input reuse, branch-local fusion, retained-value lifetime과 materialization 후보를 분석할 수 있다. 그러나 여기서도 “GPU kernel 하나로 합친다”, “AVX2를 쓴다”, “buffer #17을 할당한다” 같은 결정을 내려서는 안 된다.

요약하면 다음 경계를 지킨다.

~~~text
J source
  ↓
Frontend / J Semantic Construction
  = J가 무엇을 의미하는가
  ↓
JEntity / FunctionEntity semantic DAG
  ↓
J Graph IR
  = 그 의미가 어떤 배열 계산 graph를 만드는가
  ↓
Execution Semantic Lowering
  = 어떤 explicit execution semantics/check/effect가 필요한가
  ↓
Physical planning / backend
  = 실제로 어떻게 실행하는가
~~~

**J Graph IR은 frontend 결과물의 대체물이 아니다. Frontend가 J 의미를 소유하고, J Graph IR이 그 의미에서 optimizer가 사용할 계산 topology를 추출하는 단계적 관계다.** 이 경계를 무너뜨려 frontend에서 graph/physical 결정을 너무 일찍 하거나, 반대로 Graph IR에 source parser mechanics를 그대로 끌고 내려오지 않는다.

<a id="기억용-4단계-요약--상세-구현에서-다시-확인할-경계"></a>
#### 기억용 4단계 요약 — 상세 구현에서 다시 확인할 경계

처음 읽는 사람은 [도입부의 네 단계 모델](./01-introduction.ko.md#read-intro)부터 읽는다. 이 절의 Rank/Fork 예제와 아래 `2.1 이후 계약은 그 모델을 구현 관점에서 구체화한 것이다.

**Execution IR은 semantic execution plan이지 하드웨어 실행 계획이 아니다.** Check·effect·error ordering과 SSA dependency는 여기서 확정하지만, AVX2 선택, GPU block 크기, concrete BufferId와 materialization은 physical planning이 담당한다. 동일한 `CellApply(f, y)`도 여러 backend realization을 가질 수 있다.

<a id="21-rustj-compiler-stage의-위상"></a>
### 2.1 RustJ compiler stage의 위상

RustJ는 하나의 compiler system으로 개발한다. 별도 고유 컴포넌트명을 두기보다 각 compiler stage의 책임을 명확히 분리한다.

- **RustJ frontend / J Semantic Construction IR**: source text를 J word로 나누고 J parsing/binding 의미를 보존한다. immutable `FunctionEntity` graph가 primitive, adverb/conjunction application, hook/fork/train, rank, `@:` 같은 **J 함수 구성 자체**를 표현한다. 이 층은 언어 의미의 canonical source다.
- **J Graph IR (과거 JAXA의 Array Operation Graph 연구 계승)**: 완성된 J function을 실제 noun input에 적용한 **배열 연산 graph를 J 문법의 대수로 표현하는 compiler analysis surface**다. `@: → Pipeline`, hook/fork → Branch/Join, `/ → Reduce`, `" → CellParallel`처럼 parser/semantic construction에서 정적으로 유도되는 topology와 optimization hint를 first-class로 기록한다. JAXA의 주 관심사는 이 층이며, 역사적 JAXA의 `basis verb`도 주로 이 층의 **Graph Basis**를 뜻한다.
- **Graph Analyzer / Algebraic Optimizer — Algebraic Expansion(대수적 확장)**: J Graph IR에서 fusion topology, fan-out/fan-in, common-input reuse, retained-value lifetime, reduction/cell parallelism, materialization-elision, 이후 adjoint/VJP fan-out 등을 찾는다. 필요할 때 고수준 `Structured(Conv2d)` 등의 의미식을 **비파괴·비실행 분석용** indexed sum-product/접근 관계로 투영하여 **주변 연산까지 포함한 동등한 Graph 후보**를 탐색한다. 원본 노드는 유지하고 GraphBasis/rewrite/equivalence rule, 증명·guard·provenance로 새 고수준 Graph를 재구성할 수 있다. 이는 실행용 Window 물질화나 concrete kernel schedule이 아니다.
- **Execution Semantic Lowering — Logical Decomposition(논리적 연산 분해)**: 선택된 J Graph IR을 explicit dataflow와 normalized array operation으로 이루어진 `Logical Execution IR / Logical Execution Plan`으로 낮춘다. 필요한 경우 동일한 semantic/access 계약을 `WindowView/IndexMap → Multiply → ReduceSum/Contract` 등 **선택적 논리 접근·계산 표현**으로 구체화한다. 이는 기존 `ExecutionBasisExpansion` 계약에 기반한 선택적 연산 분해이며 실행 버퍼나 물리 루프를 만들지 않는다. J observable semantics, facts/checks, effect/error ordering, `j_origin`을 보존한다.
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
J Graph IR: J Operation Graph
        ↓
Graph Analysis / Algebraic Expansion / J-algebra optimization (optional)
        ↓  Optimized J Graph (original or witnessed selected candidate)
Execution Semantic Lowering + optional Logical Decomposition
        ↓
Logical Execution IR / Plan: Logical Operation Graph
        ↳ optional ExecutionBasisExpansion
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


<a id="211-두-compiler-ir은-목적이-다르며-둘-다-canonical-boundary다"></a>
### 2.1.1 두 compiler IR은 목적이 다르며 둘 다 canonical boundary다

현행 RustJ에는 서로 다른 질문에 답하는 두 canonical compiler IR 경계가 필요하다(과거 JAXA 연구를 계승한다).

<a id="a-j-graph-ir--이-배열-계산은-j-대수로-어떤-graph인가"></a>
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

<a id="b-logical-execution-ir--이-j-graph를-정확히-실행하려면-어떤-operationdata-dependency가-필요한가"></a>
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

<a id="두-ir-사이의-최적화-책임"></a>
#### 두 IR 사이의 최적화 책임

**세 그래프의 결과물 명칭(새로운 독립 IR 타입이 아님).** Graph와 Logical의 정본 컨테이너는 계속 기존 `j_graph_ir::Plan`과 `logical_ir::Plan`이다. 아래 명칭은 각각 그 안에 표현·선택되는 **계산 그래프의 상태/관점**을 뜻한다.

| 결과물 | 소유 단계 | 의미 |
|---|---|---|
| **J Operation Graph** (J 연산 그래프) | J Graph IR 생성 | 완료된 FunctionEntity를 실제 noun 인수에 적용한 원본 연산·의존성 그래프 |
| **Equivalent Graph Candidates** (동등 그래프 후보) | Algebraic Expansion / Graph optimizer | 원본을 포함한 대수적 대안들. Proof·guard·비용·실행 경로 판단 전에는 원본과 동일한 정본/실행 그래프가 아님 |
| **Optimized J Graph** (최적화된 J 그래프) | Graph 후보 선택 | 증명·조건을 만족해 선택된 J Graph IR 표현. 변환을 승인하지 않았다면 원본 J Operation Graph와 같을 수 있음 |
| **Logical Operation Graph** (논리 연산 그래프) | Execution Semantic Lowering / Logical Decomposition | 선택된 J Graph를 정규화된 logical operation/SSA 의존성으로 표현한 `Logical Execution IR` 내부의 그래프 관점. `SemanticCheck`, effect/first-error, `CellApply`, `j_origin`도 포함 |

**Logical Operation Graph는 별도의 세 번째 canonical IR이 아니다.** `Logical Decomposition`은 선택적이므로 `Structured(Conv2d)` 같은 고수준 노드를 그대로 포함할 수 있다. `Logical Operation Graph`의 간선은 단순 데이터 흐름뿐 아니라 의미 검사·효과·오류 순서도 설명하며, 물리 buffer, thread/loop, layout/device를 결정하지 않는다.

개념 예: `J Operation Graph: Conv2D(X,K) → MatMul(P)`에서 대수적 조건을 갖춘 후보로 `Optimized J Graph: KernelContract(K,P) → Conv2D(X,K')`를 선택할 수 있다. 그 후 선택적 `Logical Decomposition`으로 `Logical Operation Graph: WindowView/IndexMap → Multiply → ReduceSum`을 얻거나 지원되는 고수준 Conv 경로를 유지할 수 있다. 이는 구현 완료나 자동적인 Exact-J 동등성의 선언이 아니다.

**두 변환의 명명 계약:** `Algebraic Expansion`은 **Graph IR의 대수적 동등식 탐색**이며 출력은 원본을 포함한 동등 Graph 후보/증거다. `Logical Decomposition`은 **선택된 Graph의 Logical IR 연산 분해**이며 출력은 `IndexMap/WindowView/Multiply/ReduceSum` 등의 선택적 logical operation region이다. 과거 논의의 `Virtual Expansion`은 두 목적이 혼재될 수 있으므로 신규 단계 명칭으로 사용하지 않는다. Logical Decomposition은 J implicit rank loop를 전개하는 단계가 아니며, 그것은 `CellApply`/IterationDomain 의미론과 downstream schedule/backend에서 다룬다. 두 단계는 동일한 shape/rank/index/padding/numeric/error 의미 정의를 참조하되 독립된 증명 의무를 가진다. **어느 단계도 중간 배열 물질화나 실행 지원을 뜻하지 않는다.** [Conv 설계/예제](./11-roadmap-checklists-p04.ko.md#conv-virtual-expansion), [ExecutionBasisExpansion](./15-advanced-contracts-p10.ko.md#executionbasisexpansion)을 참고한다.

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

<a id="212-frontend-j-graph-logical-handoff-검증-계약"></a>
### 2.1.2 Frontend → J Graph IR → Logical IR: 검증 경계와 구현 상태 감사 (2026-10-22)

**결정:** 세 단계는 통합하지 않는다. Frontend/J Semantic Construction은 **J 표현의 의미와 함수 구성**을, J Graph IR은 **실제 인수에 적용된 계산 topology**를, Logical Execution IR은 **실행 operation·data dependency·J-visible check/effect/error 순서**를 책임진다. Graph optimizer는 실행 계획을 미리 결정하지 않으며, Logical/Physical planning이 J 문법상 topology를 뒤늦게 복원한다고 가정하지 않는다. 아래는 코드 감사에 근거한 **구현 현황**과 장기 **목표 계약**의 분리이며, 새 의미론 기능을 구현·검증했다는 선언이 아니다.

**감사 기준:** GitHub `main` `d1aa45065ce9e5ed50dbc725873f2bf8a1873b07`의 `src/runtime.rs`, `src/frontend_handoff.rs`, `src/j_graph_ir.rs`, `src/analysis.rs`, `src/logical_ir.rs`, `src/error.rs`, `src/admission.rs`, `src/j_graph_rewrite.rs`와 `README.ko.md`/`README.md`. 관련 검사 실행은 이 감사에 포함되지 않았다.

#### 현재 주 컴파일 분석 경로 (코드에서 확인)

```text
source
  -> parser::parse_analysis + semantic::bind
  -> BoundProgram
  -> j_graph_ir::Plan::from_bound_with_graph_facts + Plan::verify
  -> J Graph IR (original graph)
       + jsource_opportunities()
       + rewrite_candidates() / evaluate_rewrite_candidates()
  -> analysis::lower_graph(original_j_graph.clone(), ...)
       -> graph.verify / per-node GraphFacts vs ExecutionFacts / A3 verify
  -> CompilationAnalysis { j_graph, opportunities, candidates, resources, logical }
```

여기서 `Engine::analyze_compilation_diagnostic()`은 **원본 graph**를 lowering한다. 현재 `GraphRewriteCandidate`는 발견·검증·자원 평가의 **analysis-only 후보**이며 자동 선택·치환·실행 결과의 일부가 아니다. `VerifiedFrontend::from_program()`은 `admit_frontend_handoff`와 NAME-effects 경로 등에 있지만 위 주 분석 경로가 직접 호출하지는 않는다.

| 경계 | 현재 보장하는 사항 | 빠져 있거나 의도적으로 유보한 사항 |
|---|---|---|
| Frontend → BoundProgram | J parser reduction의 `FunctionEntity`, 이름·소스 의미, semantic binding | 주 compiler admission에서 `VerifiedFrontend`의 root·assignment·expression occurrence 대조를 일관되게 요구하는 계약 |
| BoundProgram → J Graph | `Plan::verify()`의 context 완전성·source 일치·parser origin 범위, graph facts/region 검증 | `VerifiedFrontend::from_program()`과의 검증 범위가 같다는 증거 없음; 단순 context 검증만으로 semantic Program/root 일치를 대신하지 않음 |
| J Graph → Logical | graph 재검증, GraphFacts/ExecutionFacts 일치 검사, `j_origin`/parser provenance, A3 verifier | fact drift가 현재 `Error::Unsupported`로 잘못 분류됨; rewrite 후보가 lowering 대상 graph를 바꾸지는 않음 |
| Logical v0 → compiled control | 단일 function·region·block의 SSA/Return 계약 | 다중 block, phi/block argument, CFG, explicit-definition 본문 제어 흐름 lowering은 아직 없음 |

**P0 — 내부 fact drift는 capability miss가 아니다.** `analysis::lower_graph`의 `GraphFacts::agrees_with` 실패는 컴파일러 내부 표상의 불일치이므로 `Error::Verification`/`FailureCategory::VerifierDefect`가 정당하다. 현재는 `Error::Unsupported`/`UnsupportedCapability`로 분류되어 `admission::Rejection::may_inspect_another_route()`가 다른 읽기 전용 경로 탐색을 허용할 수 있다. 수정 뒤에도 기존 J-visible 오류 종류나 순서를 바꾸지 않으며, verifier defect를 "미지원 기능"으로 숨기지 않는다. **본 절은 수정 요구만 정의한다.**

**P1 — frontend handoff를 무조건 모든 graph 생성자의 타입으로 강제하지 않는다.** `VerifiedFrontend::from_program()`은 frontend context가 가리키는 root, 최종 assignment target/source/copula 메타데이터, expression/occurrence edge 등을 추가 대조한다. 기존 graph verifier는 context 자체·source·origin 범위를 검사하지만 이 검사와 동등하다는 보장은 없다. 주 compiler 경로에서 검증된 handoff를 소비하거나 `VerifiedFrontend`와 동등한 검증을 명시적으로 선행하게 한다. 단, `from_capture`, 배열 batch 및 context 없는 합법적인 graph 생성은 **독립적인 입력 계약/admission**으로 관리한다. 일부 지원하지 않는 프로그램을 억지로 graph로 승인하지 않고 `Unsupported`를 유지해야 한다.

**P1 — 두 graph 생성 경로의 비교 범위를 제한한다.** `from_bound`는 정적 `j_graph_ir::Plan`을, `from_capture`는 관측·occurrence·constructor 정보가 더 있는 `CapturedGraph`를 반환한다. 동일한 **순수·정적·지원되는 J 문장**의 공통 graph node/region, source/provenance, 계산 의미가 일치하는지 비교하되 객체 전체·runtime observation·name guard의 완전한 동일성을 요구하지 않는다. 부작용/동적 이름/실패 capture는 무조건 동등성 예시로 선택하지 않는다.

**이후 — 선택된 graph를 lowering하는 목표 경로** (아직 실행 경로로 연결되지 않음):

```text
Verified frontend / bounded independent graph admission
    -> original J Graph IR
    -> graph candidates / equality saturation (SPORES/egg 포함 가능)
    -> Exact-J legality: value, shape/rank, error priority, NAME/effect/state
    -> required guard + target/resource feasibility + cost comparison
    -> selected, proof/witness-backed J Graph   [미구현]
    -> execution semantic lowering / A3 verifier
    -> route partition / physical planning
```

`Candidate` 생성, 동등성 근거 확인, target feasibility, resource feasibility, cost, **selected/committed transformation**은 서로 다른 상태다. 현재의 graph resource/target readiness를 곧바로 변환 실행 허가로 승격하지 않는다. 선택/증명이 미완성이면 원본 graph를 보수적으로 lowering하는 경로를 유지한다. 단, 원본 graph라도 이미 드러난 **verifier defect**를 정상 fallback으로 무시해서는 안 된다. 더 구체적인 candidate lifecycle 계약은 [`4.1.4](./05-graph-execution-lowering-p02.ko.md#414-candidate-lifecycle와-proof-discharge-contract)를 따른다.

**이후 — CFG 확대는 별도 단계:** A3-v0 `logical_ir::Plan::verify()`는 function/region/block을 각각 하나로 제한한다. 이는 현재 straight-line 배열 계산의 명시된 범위이지 RustJ 언어 전체가 single-block이라는 뜻이 아니다. explicit definition의 control flow를 graph/logical로 실제 lowering할 때 다중 block, terminator, block argument/phi, effect/first-error 및 dominance 검증을 도입한다. 그 전까지 실행 인터프리터의 제어 흐름 지원을 compiled CFG 지원으로 표기하지 않는다.

**명칭과 README:** 현행 RustJ 컴포넌트 명은 **J Graph IR / Graph Analyzer**다. `JAXA`는 이 설계에 영향을 준 과거 연구명이다. `README.ko.md`와 `README.md`의 *상세* 파이프라인은 이미 J Graph IR을 포함하므로 "README에서 Graph IR이 빠졌다"는 지적은 현재 HEAD에 적용되지 않는다. 다만 일부 *축약된* 그림은 Semantic/Graph를 한 줄로 표시하므로 상세 책임을 이 문서로 연결한다. 역사적 `JAXA Array Operation Graph IR`이라는 명칭은 현행 시스템의 병렬 컴포넌트를 의미하지 않는다.

**실행·완료 추적:** [A1.5 FGX 경계 검증 체크리스트](./11-roadmap-checklists-p04.ko.md#fgx-frontend-graph-logical-경계-검증-체크리스트-2026-10-22)에서 P0/P1/P2, 부정 테스트, 승인 조건, `PASS/FAIL/UNRUN/BLOCKED`를 관리한다. 현재 설계·코드 감사만 했으며 해당 변경의 Rust default/portable, fmt, clippy, Python, J-C differential을 실행한 것이 아니다.

<a id="22-물리적으로-함께-논리적으로-독립"></a>
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

<a id="23-logical-ir은-실행-backend를-강제하지-않는다"></a>
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

<a id="24-ir-export-adapter의-계약"></a>
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

<a id="241-semantic-operation과-architecture-specific-realization을-분리한다"></a>
### 2.4.1 Semantic operation과 architecture-specific realization을 분리한다

J built-in primitive와 NN/array extension primitive는 **semantic operation 계층에서는 동일한 원칙**으로 관리한다.

예를 들어 `+`, `Reduce(+)`, `MatMul`, `Conv`, `Softmax`, `LayerNorm`, `Attention`은 parser/name binding 이후 target-independent computational entity로 표현될 수 있다. RustJ extension primitive는 parser grammar에 하드코딩하지 않는다. **enqueue-time `PrimitiveResolver`는 core J primitive spelling만 `spellin(...) -> ds(e)`에 대응해 고정하고, alphabetic/project extension은 ordinary J `NAME`으로 enqueue한 뒤 parser-time 정상 name lookup에서 현재 binding/POS를 얻는다.** 그 binding이 extension-derived computational entity로 해석된 뒤 built-in과 같은 semantic/capability/lowering architecture에 참여한다.

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

