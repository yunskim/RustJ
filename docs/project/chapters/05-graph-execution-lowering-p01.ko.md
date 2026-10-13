[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)


<a id="read-graph-execution"></a>
<a id="4-j-graph-analyzer와-execution-semantic-lowering"></a>
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

<a id="41-j-graph-analyzer의-책임"></a>
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

<a id="410-canonical-j-graph-example-suite"></a>
#### 4.1.0 Canonical J Graph example suite

J Graph IR을 읽을 때 예제마다 같은 다섯 질문을 사용한다.

~~~text
1. source semantic construction은 무엇인가?
2. applied noun graph는 어떻게 생기는가?
3. 어떤 J provenance / GraphForm / Region을 보존하는가?
4. 어떤 optimization candidate를 노출할 수 있는가?
5. syntax만 보고 무엇을 결정하면 안 되는가?
~~~

| Source form | semantic construction | applied graph / provenance | 가능한 candidate | syntax만으로 결정 금지 |
|---|---|---|---|---|
| `f @: g` | Atop conjunction으로 만든 derived Verb | `input → Apply g → Apply f`, `Pipeline` provenance/region | pipeline fusion, intermediate materialization elision | fused kernel 선택, target placement, error/check removal |
| `(f g h) y` ordinary fork | `FunctionHead::Fork`, original f/g/h 유지 | shared input fan-out → `h(y)` / `f(y)` → dyadic `g`, `Fork` provenance; observable branch order 보존 | parallel branch, branch-join fusion, retained/live-across | 실제 branch 동시 실행, branch 순서 변경 |
| `([: g h) y` capped fork | source는 여전히 Fork + immutable capped construction fact | first operand는 실행하지 않고 `h(y) → g(...)` pipeline으로 applied topology 파생 | pipeline fusion/materialization 후보 | ordinary fork처럼 parallel/retained candidate를 부여하거나 `[:`를 callable branch로 실행 |
| `(f g) y` hook | Hook derived Verb | `g(y)`와 retained/shared `y`를 사용해 dyadic `f`; Hook provenance | retained-input/materialization, legal한 경우 branch/join fusion | shared input을 버리거나 임의 재배치 |
| `u"r y` | Rank conjunction derived Verb; requested rank/source operand 보존 | outer `CellApply` + inner operation basis; `GraphForm::Rank` | cell-level parallelism, nested CellApply fusion/absorption | rank를 physical loop/thread mapping으로 고정, nested rank boundary collapse |
| `u/ y` | Insert adverb derived Verb | `GraphForm::Reduce`, outer `GraphBasis::Reduce`, operand `u` provenance | reduction realization, legal한 map/reduce fusion | tree reassociation, identity/empty handling 변경, arbitrary parallel reduction |
| `u\ y` | Prefix/Infix adverb derived Verb | source Prefix/Infix form + Window-family graph basis; prefix/window semantics 보존 | witnessed Scan candidate, window/reduce rewrite | 곧바로 Scan으로 치환, associativity/error/numeric proof 생략 |

대표 topology:

~~~text
f @: g
  input ─→ g ─→ f ─→ result
          └──── Pipeline provenance ────┘

ordinary fork
                 ┌─→ h(y) ─┐
  y ─────────────┤          ├─→ g ─→ result
                 └─→ f(y) ─┘
                    Fork provenance

capped fork
  y ─→ h(y) ─→ g ─→ result
      source Fork provenance + capped construction fact
      no executable first branch

hook
  y ────────────────┐
   └─→ g(y) ────────┴─→ f ─→ result
      retained/shared input
~~~

이 suite의 목적은 모든 form을 하나의 generic graph pattern으로 환원하는 것이 아니다. **같은 applied DAG 모양이 우연히 나와도 source construction provenance가 다르면 legality/error/name semantics가 다를 수 있으므로**, J Graph IR은 applied dependency와 J construction identity를 함께 보존한다.

현재 구현 상태를 과장하지 않는다.

- `@:`/ordinary fork/hook/rank/reduce/prefix-infix의 GraphForm/GraphBasis/hint 기반은 존재한다.
- capped fork는 source Fork를 유지하면서 h→g pipeline으로 분석하고 parallel/retained hint를 붙이지 않는 회귀가 있다.
- scan은 source prefix/window 의미를 보존한 뒤 별도 witness/analysis가 candidate를 만들며, source를 즉시 Scan op로 파괴하지 않는다.
- 위 candidate가 존재한다는 사실은 §4.1.4의 proof-discharge/selection lifecycle이 구현 완료됐다는 뜻이 아니다.

<a id="j-graph-opportunities"></a>

<a id="411-j-syntax-derived-structural-opportunity-ir-문법을-optimization-information-source로-사용한다"></a>
#### 4.1.1 J syntax-derived Structural Opportunity IR: 문법을 optimization information source로 사용한다

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
fork        → Parallel branch candidate
adjoint/VJP → expanded graph topology (후속 연구)
^:          → Iteration topology
"           → Cell-level parallel application topology
~~~

두 정보는 합쳐져야 한다.

~~~text
operation kind × graph topology × semantic facts
~~~

<a id="jaxa에서-계승하는-첫-세-topology"></a>
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

**3. adjoint/VJP expansion — 생성된 graph에서 발견하는 fan-out opportunity**

Adjoint/VJP 관계 등록 자체는 parallel 힌트가 아니다. `(loss_adjoint [ (loss emit))`의 병렬 후보는 `[`를 가운데 둔 바깥 fork syntax에서 나오며, 아래 미래 AD expansion 사례도 실제 생성된 branch topology가 후보의 근거다. 후보 발견과 effect/error/alias proof를 통한 실행 허가를 분리한다.

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

<a id="opportunity와-legality를-분리한다"></a>
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

<a id="flowstorage와의-연결"></a>
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

<a id="현재-구현-경계의-정본"></a>
##### 현재 구현 경계의 정본

`j_graph_ir::classify_function()`이 `GraphForm/GraphHint`를 만들고, execution lowering은 이를 canonical `logical_ir::Plan`의 opportunity/provenance로 투영한다. `Operation.j_origin`이 graph→execution 대응을 보존한다. `CompilationAnalysis`는 `j_graph`와 `logical`을 함께 반환한다.

세부 구현과 남은 graph 연구는 [A1.5](./11-roadmap-checklists-p04.ko.md#j-graph-implementation-checklist), 단일 execution IR cutover는 [M1](./11-roadmap-checklists-p01.ko.md#architecture-migration-checklist), 전체 상태는 [§12](./13-current-status.ko.md#current-implementation-status)에서 추적한다. adjoint/VJP expansion은 여전히 schema/연구 단계이며 실제 AD transform이나 병렬 실행 완료가 아니다.

<a id="중요한-불변조건"></a>
##### 중요한 불변조건

1. StructuralOpportunity는 optimization hint보다 강한 **semantic-topology provenance**지만, physical schedule은 아니다.
2. Opportunity가 존재한다고 fusion/parallelization legality가 자동 성립하지 않는다.
3. J observable evaluation order는 opportunity 안에서도 잃지 않는다. 병렬 실행은 별도 proof가 있어야 한다.
4. basis normalization 때문에 Atop/Hook/Fork provenance를 버리지 않는다.
5. generic DAG pattern matching은 추가 기회를 찾는 보조 수단일 수 있지만, J 문법이 이미 준 topology를 다시 찾는 주 경로가 되어서는 안 된다.
6. hardware resource 정보는 opportunity discovery에 필요하지 않지만, 실제 fusion/parallel region 확정에는 필요하다.
7. 이 층의 목적은 J 문법의 표현을 미학적으로 보존하는 것이 아니라 **optimizer search space를 줄이고 materialization/lifetime/parallel 후보를 일찍 제공하는 것**이다.


<a id="syntax-graph-hints"></a>

<a id="412-j-syntax에서-얻는-graph-최적화-힌트-분류"></a>
#### 4.1.2 J syntax에서 얻는 graph 최적화 힌트 분류

검토일: 2026-10-02. 범위: primitive의 산술·배열 구현 정보를 사용하기 전에, J의 구문 구조에서 얻는 graph 정보를 분류하고 검토한다.
일반 컴파일러 정보 분류 → syntax로 해석 가능한 분류 → J 구문 대응 → graph 정보와 최적화 후보의 순서로 정리한다.
문서·설계 검토이며 구문 지원이나 실행 최적화를 새로 구현한 것은 아니다.

<a id="4121-먼저-바로잡는-기준-fork-구조-자체가-병렬-실행-후보를-드러낸다"></a>
##### 4.1.2.1 먼저 바로잡는 기준: fork 구조 자체가 병렬 실행 후보를 드러낸다

`(loss_adjoint [ (loss emit))`의 parallel 가능성은 **`[`를 가운데 verb로 둔 fork 구문 자체**에서 나온다. `loss_adjoint`가 미분 연산이라는 사실이나 `loss`의 구현을 알아야 이 후보를 발견하는 것은 아니다.

바깥 형태를 `(f [ h)`로 보면 단항 적용 시 다음 구조가 된다.

```text
              input
              /   \
            f       h
              \   /
          가운데 verb: [
          결과: 왼쪽 branch의 값
```

이 구문만으로 입력 fan-out, 두 branch 사이의 직접 반환값 의존성 부재, 가운데 결합 위치를 알 수 있다. 가운데 verb가 `[`라는 의미를 더하면 왼쪽 결과 선택을 알 수 있다. 따라서 정보의 출처는 다음처럼 구분한다.

1. **Syntax**: 세 verb의 fork 형태가 두 branch를 같은 입력에 적용하는 graph를 만든다.
2. **가운데 verb의 의미**: `[`는 왼쪽 결과를 선택한다. 이는 산술 kernel의 구현 정보가 아니다.
3. **후속 적법성 검사**: 숨은 상태 의존성·효과·alias·관찰 가능한 오류가 허용하는지 확인한다.
4. **후속 비용 판단**: 병렬 실행·동기 실행·fusion 중 실행 전략을 선택한다.

‘parallel 후보를 발견하는 근거’와 ‘실제로 parallel 실행해도 되는지의 검사’를 혼동하지 않는다. 구문에서 후보가 나온다는 사용자의 설명을 기준으로 한다. [J Trains](https://www.jsoftware.com/help/dictionary/dictf.htm)

`(loss emit)` 안의 hook 구조도 별도로 해석한다. 두 이름이 verb로 해석된 경우 내부는 `y loss (emit y)`의 의존성을 가지며, 바깥 fork의 fan-out과 내부 hook의 의존성은 함께 보존한다. 실제 JAXA binding이 다른 품사라면 먼저 해당 품사/파싱을 확정한다. `emit`의 효과는 JAXA 계약에서 얻으며 표준 J 키워드로 간주하지 않는다.

<a id="4122-먼저-분류-graph에서-읽을-수-있는-일반-컴파일러-정보"></a>
##### 4.1.2.2 먼저 분류: graph에서 읽을 수 있는 일반 컴파일러 정보

| 분류 | 일반적으로 필요한 정보 | primitive 내부를 보지 않고 syntax에서 얻을 수 있는 부분 |
|---|---|---|
| 의존성·병렬성 | 독립 branch, producer-consumer, join | fork/hook/합성의 graph 연결 |
| 값 흐름·사용 관계 | 어떤 입력을 어디에 전달하고 어떤 결과를 사용하는가 | train의 입력 routing, 결과 선택 형태 |
| fusion 범위 | 연산 연결과 cell 적용 경계 | 합성·capped fork·rank 구조 |
| 작업 분할 | 적용 단위, frame/cell, 구간 경계 | rank와 partition/window modifier |
| 반복·선택 | loop-carried edge, 반복 수, 분기 영역 | power와 agenda |
| 상수·특수화 | 결합된 인자와 호출 대상의 안정성 | noun bond, noun을 포함한 train, 명시적 Fix |
| 재사용 후보 | 동일 입력에 대한 반복 적용, 여러 consumer | 같은 graph 부분이 반복되는 구문 구조 |

순수성, 정확한 dtype/shape, 결합법칙, 메모리 정렬·독점 소유, SIMD 폭, CUDA 배치는 syntax만으로 일반적으로 보장되지 않는다. 이번에는 이런 선언이나 실행 기능을 추가하지 않는다.

위 분류는 [GCC 속성](https://gcc.gnu.org/onlinedocs/gcc-15.2.0/gcc/Common-Function-Attributes.html), [LLVM 언어 참조](https://llvm.org/docs/LangRef.html)의 최적화 정보 구분을 참고해 graph 관점으로 정리한 것이다.

<a id="4123-syntax-자체에서-얻는-힌트-leaf-verb를-black-box로-둔-검토"></a>
##### 4.1.2.3 Syntax 자체에서 얻는 힌트: leaf verb를 black box로 둔 검토

여기서 syntax는 train 문법과 verb를 조합하는 modifier 구문까지 포함한다. J 용어로는 `@`, rank 등의 modifier도 primitive에 속할 수 있지만, 이 문서에서는 **leaf 연산이 무엇을 계산하는지의 정보**와 **연산을 어떻게 연결·적용하는지의 정보**를 분리한다.

`f`, `g`, `h`, `u`, `v`가 어떤 산술 연산인지 몰라도 다음 정보를 얻을 수 있다. 표의 식은 단항/양항 적용을 명시했으며 설명용 정규화에는 rank 적용 경계가 별도로 따라붙는다.

| ID | J 표현 | syntax/조합 의미에서 드러나는 graph 정보 | 컴파일러에게 주는 후보 |
|---|---|---|---|
| S1 | `(f g h) y` | 같은 입력에서 `f`와 `h`로 fan-out, 결과를 `g`에 전달 | branch parallel, 공통 입력 접근 계획 |
| S2 | `(f [ h) y`, `(f ] h) y` | S1의 fork + 가운데 selector에 따른 결과 사용 관계 | branch parallel, 불필요한 결과 저장 축소 |
| S3 | `(f g) y` | `g(y)`를 계산한 뒤 원 입력과 함께 `f`에 전달 | 원 입력 수명 보존, producer-consumer fusion |
| S4 | `([: u v) y`, `(u@:v) y` | `v` 결과를 `u`로 전달하는 연쇄 | 중간 결과 materialization 축소 후보 |
| S5 | `(u@v) y`와 `(u@:v) y` | 합성의 rank/cell 경계가 다름 | cell-local fusion과 whole-array fusion의 구분 |
| S6 | `x (u&v) y` | 양쪽 입력을 각각 `v`에 통과시킨 뒤 `u`로 결합 | 두 변환의 parallel 후보, 같은 변환 코드 특수화 |
| S7 | `(u"n) y` | 명시한 rank의 cell에 같은 verb를 적용 | cell 단위 작업 분할·batch kernel 후보 |
| S8 | `(m&u) y`, `(u&n) y`, `(N g h) y` | noun operand가 결합됨; noun train은 상수 branch 형태 | 결합 값 특수화, 반복 전달·계산 축소 후보 |
| S9 | `(u^:k) y` | 반복 영역과 이전 결과→다음 입력 edge | 작은 상수 반복 전개, 임시 값 수명 계획 |
| S10 | gerund의 `@.` | selector와 선택되는 verb/train 영역 | 상수 selector 해소, 실행 영역 분리 |
| S11 | 중첩 train·괄호로 구성한 verb | 정확한 grouping과 producer/consumer 영역 | graph 구조 보존, 최적화 탐색 범위 명시 |
| S12 | `(f g f) y` | 동일한 이름 표현을 같은 입력에 두 번 적용하는 형태 | 공통식 제거 후보. 동일 binding·효과 계약 확인 필요 |

근거: [Trains](https://www.jsoftware.com/help/dictionary/dictf.htm), [Atop](https://www.jsoftware.com/help/dictionary/d620.htm), [At](https://www.jsoftware.com/help/dictionary/d622.htm), [Compose](https://www.jsoftware.com/help/dictionary/d630v.htm), [Rank](https://www.jsoftware.com/help/dictionary/d600v.htm), [Bond](https://www.jsoftware.com/help/dictionary/d630n.htm), [Power](https://www.jsoftware.com/help/dictionary/d202n.htm), [Agenda](https://www.jsoftware.com/help/dictionary/d621.htm).

특히 S1–S6은 leaf verb의 배열 연산 의미를 몰라도 graph 연결을 얻는다. S7–S10은 modifier의 일반 적용 의미와 상수 operand를 해석하며, leaf verb의 산술 속성은 요구하지 않는다.

<a id="41231-구문별로-후보와-허가를-분리한다"></a>
###### 4.1.2.3.1 구문별로 후보와 허가를 분리한다

- **Fork**: 두 branch의 반환값이 서로의 직접 입력이 아니라는 사실을 제공한다. 숨은 global/slot 상태 의존성까지 없다는 선언은 아니다. 효과·오류 검사는 후보를 없애는 근거일 수 있지만, 후보의 출처는 syntax다.
- **선택 fork**: `[`, `]`는 결과 사용 관계를 알려준다. 선택되지 않은 branch의 효과와 오류를 무조건 없애지 않는다. 그 결과 payload를 만들지 않아도 동일 실행 의미가 가능한지는 후속 분석이다.
- **Hook**: 원 입력을 downstream verb가 다시 쓰므로 입력 수명과 두 입력 edge를 보존한다. 내부 stage를 두 독립 branch로 실행하는 힌트는 아니다.
- **합성**: 순서 있는 데이터 의존성을 준다. fusion 후보이지만 leaf가 black box면 내부 fusion을 구현할 수 있다는 뜻은 아니다. 효과·오류·rank를 보존해야 한다.
- **Dyadic compose**: 동일 verb가 다른 두 입력에 적용되는 구조다. 두 호출을 하나의 값 계산으로 합칠 수 있다는 뜻은 아니다. `&`의 noun bond와 verb compose는 operand 품사에 따라 구분한다.
- **Rank**: cell/frame 경계를 준다. dtype·cell shape·물리 연속성·순수성은 별도 정보다. J agreement, 음수 rank, 빈 frame/prototype과 오류 순서를 보존한다.
- **Power**: 비음수 정수 반복은 loop-carried edge를 준다. 반복 자체는 parallel 후보가 아니다. 음수·무한·boxed·배열 power를 같은 고정 loop로 낮추지 않는다.
- **Agenda**: scalar selector이면 단일 선택 영역 후보다. 일반 selector는 train을 구성할 수도 있으므로 모든 `@.`를 if로 바꾸지 않는다. 비선택 영역을 임의로 추측 실행하지 않는다.
- **괄호**: grouping이 parse 구조를 바꿀 때만 정보가 달라진다. 같은 parse를 둘러싼 여분의 괄호는 fusion fence·순서 보장·메모리 materialization 선언이 아니다.
- **반복 표현**: 같은 이름의 두 출현만으로 공통식 제거를 허가하지 않는다. binding·입력·효과·관찰 가능한 오류의 동등성을 확인해야 한다.

<a id="4124-leaf-primitive-힌트와-섞지-않을-modifier-구조"></a>
##### 4.1.2.4 Leaf primitive 힌트와 섞지 않을 modifier 구조

다음은 syntax로 표현되는 고수준 적용 구조다. 구조 자체와 leaf를 알아야 얻는 최적화 정보를 별도 열로 분리한다.

| 표현 | leaf를 몰라도 얻는 적용 구조 | leaf를 알아야 얻는 추가 정보 |
|---|---|---|
| `u/ y` | item 사이에 dyad를 삽입하는 구조 | `u`의 항등원·결합법칙·수치 재결합 허용 |
| `u\ y` | 각 prefix에 `u` 적용 | `u`가 특정 reduce일 때 효율적인 scan 인식 |
| `k u\ y` | window/chunk 길이와 겹침·마지막 조각 구조 | rolling 집계 변환의 동등성 |
| `u;.n` | cut 종류와 구간/tile 적용 구조 | 구간 안의 kernel 및 출력 특수화 |
| `keys u/. values` | key별 그룹에 `u` 적용 | key 비교 계약과 그룹 집계 kernel |
| dyadic `u . v` | 두 verb를 결합하는 contraction 적용 구조 | `+/ . *` 등 구체적 dot/GEMM 인식 |

즉 `+/\`의 **prefix 적용 구조**는 modifier 쪽 정보이고, **덧셈 누적을 위한 scan kernel**은 leaf `+`와 reduce 의미를 더한 정보다. `+/ % #`도 fork라는 syntax 정보와 sum/tally/divide라는 leaf 정보를 분리한다.

근거: [Insert/Table](https://www.jsoftware.com/help/dictionary/d420.htm), [Prefix/Infix](https://www.jsoftware.com/help/dictionary/d430.htm), [Cut](https://www.jsoftware.com/help/dictionary/d331.htm), [Key/Oblique](https://www.jsoftware.com/help/dictionary/d421.htm), [Dot](https://www.jsoftware.com/help/dictionary/d300.htm).

<a id="4125-binding-정보와-일반-graph-대비-추가-가치"></a>
##### 4.1.2.5 Binding 정보와 일반 graph 대비 추가 가치

명시적 `u f.`는 이름 참조를 당시 referent로 고정하는 의미를 제공하지만, 순수한 grouping 문법만의 사실과는 구분한다. `$:` 포함 부분은 Fix의 예외다. 사용자가 쓰지 않은 Fix를 자동 적용해 동적 이름 의미를 바꾸지 않는다. [J Fix](https://www.jsoftware.com/help/dictionary/dfdot.htm)

일반 배열 graph가 leaf 호출과 데이터 edge만 가진다면 J의 rank/partition/반복 영역 및 바인딩 의미는 추가로 보존할 정보가 된다. 이미 fan-out edge를 가진 graph에도 **fork syntax가 parallel 후보를 제공한다**는 사실은 같다. 다만 graph의 표현력이 동일하다면 새 정보가 더 생기는 것은 아니다. J syntax는 graph 구성과 구조 인식의 source다.

현재 가장 중요한 설계는 leaf kernel 목록으로 일찍 풀지 않고 syntax에서 유래한 fork/hook/compose/cell/partition/loop/selection 구조를 보존하는 것이다. ‘syntax origin’ 기록은 구문이 어떤 최적화 후보를 제공했는지 설명하는 진단에도 사용할 수 있다.

