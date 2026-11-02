[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)

<a id="13-프레임워크-조사에서-채택한-원칙"></a>
## 13. 프레임워크 조사에서 채택한 원칙

외부 프레임워크의 언어 의미를 가져오는 것이 아니라 검증된 구현 아이디어를 참고한다.


<a id="참고-구현과-jaxa-설계-배경--필요할-때-읽는-심화-자료"></a>
### 참고 구현과 JAXA 설계 배경 — 필요할 때 읽는 심화 자료

이하의 선행 구현·JAXA 역사와 source 비교는 설계 근거다. **처음 읽는 사람은 `1~`8의 본문을 먼저 읽어도 된다.** 과거 소제목의 번호는 기존 참조·앵커 보존을 위해 유지한다.

<a id="참고-구현reference-implementations"></a>
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

- **Remora / Bohrium / Lift / MLIR Linalg — adjacent array-language / IR compiler references**
  - Remora는 J/APL 계열의 rank polymorphism, frame/cell semantics와 implicit lifting을 정형화한 비교 대상이다. 근거: https://arxiv.org/abs/1907.00509
  - Bohrium은 기존 NumPy-style array operation을 lazy하게 수집해 fusion, allocation/materialization, host-device data movement와 backend-specific execution을 늦추는 선례다. 이를 각 operation마다 CPU/GPU를 동적으로 선택하는 모델로 과장하지 않는다. 근거/논문 목록: https://bohrium.readthedocs.io/publications.html
  - Lift는 portable map/reduce pattern에서 OpenCL-specific functional pattern까지 rewrite-driven하게 변환하며 hardware mapping을 점진적으로 구체화하는 optimizer 연구의 비교 대상이다. rewrite와 hardware mapping의 완전한 분리 선례로 해석하지 않는다. 근거: https://doi.org/10.1109/CGO.2017.7863730
  - MLIR Linalg는 structured operation과 implicit iteration을 보존한 뒤 tiling/vectorization/lowering에서 loop를 materialize하는 계층화의 비교 대상이다. 근거: https://mlir.llvm.org/docs/Tutorials/transform/Ch0/
  - 어느 시스템도 RustJ의 J semantic specification은 아니며, compiler layering과 optimization technique의 근거로만 사용한다.

따라서 reference 우선순위는 목적별로 다르다.

```text
J semantic correctness       → jsource
array graph/JIT fusion       → ArrayFire
J↔external GPU adapter       → jsoftware/math_arrayfire
array-compiler middle-end    → APEX / Co-dfns / TAIL-Futhark
rank/structured-IR comparison → Remora / Bohrium / Lift / MLIR Linalg
```

ArrayFire 관련 구체적인 Source → Observation → RustJ 적용·비채택 사항은 `13의 **ArrayFire / J ArrayFire add-on / fusion systems** 절을 따른다.

현재 구현은 목표 compiler pipeline 전체를 완성한 상태가 아니다. 제한된 J frontend와 CPU 직접 실행 경로, Semantic IR, J Graph와 canonical Logical IR 분석, CPU storage/SIMD, sparse/boxed 기초, 읽기 전용 affine PhysicalArray가 함께 존재하는 **전환 단계**다.

<a id="11-jaxa에서-이어받은-설계-원칙--배열-연산의-sql"></a>
### 1.1 JAXA에서 이어받은 설계 원칙 — “배열 연산의 SQL”

JAXA 문서에서는 **“NN의 SQL”**, **“배열 연산의 SQL”**이라는 비유를 사용했다. RustJ에서는 이 아이디어를 신경망 전용 표현보다 더 일반적인 **high-level array language / array query language** 관점으로 승계한다.

이 표현의 핵심은 SQL과 비슷한 문법을 만들거나 J 전체를 순수 declarative language라고 주장하는 것이 아니다. J에는 name lookup, assignment, effect, error/control semantics가 있으므로 full J는 SQL과 같은 순수 질의 언어가 아니다. 비유가 가리키는 것은 다음 한 문장이다.

> **J source is not an execution plan.**

J의 array semantics와 function composition은 가능한 한 **무엇을 계산하는가**를 고수준으로 표현하고, **어떻게 실행하는가**는 semantic legality를 보존하는 범위에서 compiler가 선택한다.

~~~text
J source / J semantics
        ↓
J Semantic IR / J Graph IR
        ↓
Logical Array / Execution IR
        ↓
equivalence / fusion / logical optimization
        ↓
execution planning / route selection
        ↓
CPU / SIMD / multicore / GPU / external compiler / library
~~~

<a id="sql-ir-planning-analogy"></a>
**SQL 관계대수 → 논리적 실행 계획 → 물리적 실행 계획: RustJ IR 경계의 비유.**

SQL 옵티마이저의 역할에 대응하면 각 IR의 질문을 쉽게 구분할 수 있다. 다만 실제 DBMS는 관계대수식과 논리적 질의 계획을 하나의 IR로 다루기도 한다. 아래는 **RustJ 내부 책임의 개념적 대응**이지 DBMS가 모두 동일한 세 개의 IR을 사용한다는 뜻은 아니다.

| SQL/DB 개념 | RustJ 대응 | 묻는 질문 |
|---|---|---|
| 파싱과 의미 분석 | Frontend / J Semantic Construction | 원본 J 표현의 name, rank, derived function과 계산 의미는? |
| **관계대수 및 논리적 동치 재작성** | **J Graph IR / Graph Analyzer / Algebraic Expansion** | 같은 결과를 내는 다른 계산 그래프를 찾을 수 있는가? |
| **논리적 실행 계획의 명시화** | **Execution Semantic Lowering → Verified Logical IR / Plan** | 선택된 계산의 접근 관계, 연산, SSA 의존성, checks, effect/error order는? |
| **물리적 조인 알고리즘 선택** | **Route / Schedule / Physical Plan 또는 검증된 external lowering** | 어떤 kernel, 타일, 버퍼, layout, device로 실행할 것인가? |
| 실행 엔진 | RustJ Runtime / backend | 선택된 계획을 실행 |

SQL의 `(A JOIN B) JOIN C` ↔ `A JOIN (B JOIN C)` 같은 **관계대수 재작성**은 Graph의 동등 계산 후보 탐색에 가깝다. SQL의 **Hash Join / Merge Join / Nested Loop Join 선택**은 Physical 실행 방법 선택에 가깝다. 그 사이의 **Logical IR은 단순한 커널 선택 단계가 아니라 선택한 계산을 실제 데이터 접근·연산·검사·의존성으로 구체화**하는 단계다. RustJ에는 Logical Optimizer도 있으므로 동등성에 기반한 변경이 오직 Graph에만 존재한다고 해석해서는 안 된다.

**Conv로 보는 세 단계 (문서 E9의 수치 fixture; 실제 RustJ IR 출력이 아닌 설계 의사 코드).**

~~~text
X = [[1,2,3],      W = [[ 2,-1],
     [4,5,6],           [ 0, 3]]
     [7,8,9]]

(1) Graph 원본:
    Y = StructuredConv2D(X, ParameterRef(C1, weight@v7))

(2) Graph의 선택적 Algebraic Expansion (SQL VIEW unfolding에 해당):
    win[oh,ow,kh,kw] = X[oh+kh,ow+kw]
    Y[oh,ow] = sum_(kh,kw) win[oh,ow,kh,kw] * W[kh,kw]
    // 원본 StructuredConv를 보존하는 비실행 analysis sidecar

(3) Graph 동등 후보 탐색:
    VirtualIm2col(X) =
      [[1,2,4,5], [2,3,5,6], [4,5,7,8], [5,6,8,9]]
    Flatten(W) = [2,-1,0,3]
    Reshape(MatMul(VirtualIm2col(X), Flatten(W)),[2,2])
      = [[15,19],[27,31]]
    // 이 숫자들의 일치는 일반 J floating 동치의 증명이 아님

(4) Logical IR의 선택적 Logical Decomposition:
    %x = Input(X)
    %w = ParameterRead(C1, weight@v7)
    %a = WindowView(%x, kernel=[2,2], stride=1, pad=0)
    %y = OrderedContract(%a, %w, axes=[kh,kw])
    Result(%y)
    // 선택된 Graph가 고수준 Conv라면 검증된 경로에서 그대로 유지할 수도 있음
    // Logical은 SSA, shape/rank, guards, effect/error order 등을 기록

(5) Physical 후보:
    direct Conv | implicit GEMM | tiled im2col+GEMM | full im2col+GEMM
    // tile, concrete buffer, layout, SIMD/GPU, materialization 결정
~~~

SQL의 **뷰 전개가 실제 테이블의 생성이나 조인 실행을 뜻하지 않듯**, Graph의 `Algebraic Expansion`은 Window의 인덱스·sum-product 관계를 **분석용으로 노출**하는 것이다. Logical의 `Logical Decomposition`은 선택된 계산을 **명시적 논리 연산으로 표현**하는 별도 과정이다. 둘 다 실행 loop나 실제 window/im2col 배열의 물질화를 강제하지 않는다.

**중요한 한계:** J Graph IR 자체가 관계대수 IR인 것은 아니다. J의 Rank/Cell/Frame, 동적 NAME·assignment, 효과 및 오류 관찰 순서, 부동소수점 reduction 순서는 일반적인 SQL 동치 법칙과 다르다. 후보의 수학적 표현 가능성과 Exact-J 합법성·parameter snapshot/version·target/실제 lowering 가능성을 따로 검증해야 한다. 현재 Conv2D Algebraic Expansion / Contract SSA lowering / 관련 물리 경로는 **설계 단계이며 아직 실행 기능으로 구현되지 않았다**. 자세한 사례는 [Conv E9–E14](./11-roadmap-checklists-p04.ko.md#conv-virtual-expansion-examples), 단계 계약은 [`2 아키텍처](./03-architecture-p01.ko.md#sql-graph-logical-physical-analogy)를 참고한다.

사용자는 가능한 한 계산의 의미와 필요한 semantic/storage obligation을 표현하고, 다음 사항은 analyzer/compiler/backend에 맡긴다.

- 어떤 동등한 graph form을 사용할지
- fusion 또는 materialization을 할지
- 어떤 실행 basis와 route를 사용할지
- 어떤 memory/layout/schedule 전략을 사용할지
- CPU/SIMD/multicore/GPU 중 어느 realization을 선택할지
- 검증된 external compiler/library route를 사용할지

J가 이 역할에 유리한 이유는 source 자체에 optimizer가 활용할 수 있는 배열 구조가 풍부하기 때문이다.

- rank는 cell/frame 경계와 implicit iteration domain을 드러낸다.
- adverb/conjunction/derived entity는 reduce, scan, cell application, composition 같은 고차 구조를 보존한다.
- hook/fork/train/@:는 producer/consumer, branch/join, composition topology를 source 수준에서 제공한다.
- reshape/transpose/take/drop 등은 logical shape/reindex 의미와 physical materialization을 분리할 여지를 준다.
- whole-array notation은 scalar loop에서 고수준 의미를 역추론하는 비용을 줄인다.

<a id="jaxa-graph-ir의-역사적-출발점--j-표기에서-optimization-topology를-읽는다"></a>
#### JAXA Graph IR의 역사적 출발점 — J 표기에서 optimization topology를 읽는다

JAXA의 출발점은 추상적인 “graph compiler를 만들자”가 아니었다. 먼저 다음과 같은 구체적인 관찰이 있었다.

~~~text
u@:v
    → input → v → u
    → producer/consumer chain
    → kernel-fusion candidate

(f g h) y
    → 같은 input에서 f/h로 갈라진 뒤 g에서 합류
    → branch/join topology
    → branch parallelism / branch-local fusion candidate

(f g) y
    → 원 input과 g(y)가 f로 들어감
    → ordered dependency + input-lifetime relation
    → producer/consumer fusion candidate
~~~

즉 `@:`, Hook, Fork 같은 J의 function-composition 표기는 단순한 축약 문법이 아니라 **실행 방법을 확정하지 않은 채 계산 의존성과 topology를 source 수준에서 드러내는 표현**으로 볼 수 있다. JAXA는 이 정보를 scalar loop나 backend kernel로 낮추기 전에 compiler가 직접 읽고 보존하면 fusion·parallelism·materialization·lifetime 후보를 훨씬 일찍 발견할 수 있다고 보았다.

이 관찰에서 RustJ의 독립 `J Graph IR`이 나온다.

~~~text
J syntax / FunctionEntity
        ↓
syntax-derived computation topology
        ↓
J Graph IR
        ↓
candidate generation
  fusion / branch parallelism / materialization / reuse
        ↓
semantic legality
  effects / errors / names / alias / rank contracts
        ↓
profitability / resource / target choice
        ↓
Logical/Physical realization
~~~

여기서 세 단계를 혼동하지 않는다.

1. **topology가 optimization candidate를 드러내는 것**
2. **그 transformation이 J semantics상 합법임을 증명하는 것**
3. **실제로 그 strategy가 더 이득인지 선택하는 것**

Fork가 보인다고 두 branch를 무조건 병렬 실행하지 않고, `@:`가 보인다고 무조건 fusion하지 않는다. source structure는 후보의 근거이고, effect/error/name/alias 등의 semantic legality와 cost/resource 판단은 별도 단계다.

위 branch/join 도식은 **ordinary VVV fork**에 해당한다. 생성 시점에 capped 의미로 고정된 `[: g h`는 `input → h → g(monad)`의 순차 pipeline이고, noun-left fork는 `h(input)`과 고정 noun을 g에 전달한다. 따라서 단순히 Fork라는 parser row/head만 보고 두 실행 branch를 가정하지 않고 constructor 의미와 operand 품사를 함께 해석한다. 원래 source Fork/NAME DAG는 계속 보존한다.

이 아이디어의 각 구성 요소 자체를 RustJ의 최초 발명으로 주장하지 않는다. Hook/Fork의 dataflow 의미, function-level program transformation, graph-based fusion과 high-level array IR에는 각각 선행 연구와 구현이 있다. RustJ/JAXA의 설계상 중요한 결합은 **J의 tacit combinator algebra를 독립적인 semantic graph layer로 보존하고, 그 구조 자체에서 optimization candidate를 생성한 뒤 full-J semantic legality와 physical profitability를 분리해서 판단하는 것**이다.

<a id="관련-선행-연구와-rustj의-위치"></a>
##### 관련 선행 연구와 RustJ의 위치

이 출발 관찰 자체에는 직접적인 선행 연구가 있다. 따라서 RustJ의 연구적 위치를 평가할 때 **“J 구문이 최적화 힌트를 제공한다” 자체를 novelty로 주장하지 않는다.** 특히 Bernecky의 APL93 논문은 사용자가 JAXA에서 `@:`, Fork, Hook을 보며 출발한 문제의식과 매우 가깝다. Fork의 양쪽 가지 병렬성, composition의 pipeline 성격, expression-level merging을 통한 중간 배열·저장 비용 축소를 이미 명시적으로 논의한다.

- **Robert Bernecky, _The Role of APL and J in High-performance Computation_ (APL93, 1993)**
  - J tacit definition의 Fork에서 `f`와 `h` 계산이 병렬로 진행될 수 있음을 명시하고, tacit form이 data-flow/data-dependency 분석 부담을 줄인다고 설명한다.
  - expression-level static analysis로 배열 primitive sequence를 interleaved execution으로 합치는 **loop jamming / merging**을 논의한다. 이는 temporary 제거와 fusion 계열의 직접 선례다.
  - J composition을 cell 결과가 verb에서 verb로 전달되는 **pipeline**으로 설명하며 cell-level 병렬성을 지적한다.
  - paper: https://www.snakeisland.com/aplhiperf.pdf
  - DOI: https://doi.org/10.1145/166197.166201

- **John Backus, _Can Programming Be Liberated from the von Neumann Style?_ (CACM, 1978)**
  - program-combining forms와 그 algebra를 프로그램 변환의 대상으로 보는 function-level 계보의 중요한 선례다.
  - https://research.ibm.com/publications/can-programming-be-liberated-from-the-von-neumann-style-a-functional-style-and-its-algebra-of-programs

- **Accelerate / Futhark / Lift / MLIR Linalg**
  - Accelerate와 Futhark는 high-level array operations와 dependency structure를 보존해 fusion과 parallel lowering을 수행한다.
  - Lift는 map/reduce 같은 functional data-parallel pattern의 의미를 rewrite-rule 기반 optimization과 GPU mapping에 사용한다.
  - MLIR Linalg는 transformation에 필요한 structured semantics를 loop/CFG lowering 전에 보존하고 transformation validity와 profitability를 분리한다.
  - Accelerate: https://www.acceleratehs.org/publications.html
  - Futhark: https://futhark.readthedocs.io/
  - Lift: https://doi.org/10.1109/CGO.2017.7863730
  - MLIR Linalg: https://mlir.llvm.org/docs/Rationale/RationaleLinalgDialect/

따라서 현재의 보수적인 novelty framing은 다음과 같다.

~~~text
J syntax가 optimization-relevant structure를 드러낸다
    → 선행 연구 있음

Fork/Composition/Rank 등에서 parallelism·pipeline을 읽는다
    → 직접적인 J/APL 선행 연구 있음

high-level array operations를 보존해 fusion/rewrite를 한다
    → Accelerate / Futhark / Lift / MLIR 등에 선행 연구 있음

full J tacit combinator algebra를
독립 J Graph IR로 보존하고
그 topology에서 optimization candidate를 생성한 뒤
full-J semantic legality와
physical profitability를 별도 단계로 판단한다
    → RustJ/JAXA가 탐구하는 distinctive architectural combination
~~~

따라서 JAXA의 역사적 핵심 질문은 다음처럼 기록한다.

> **J의 함수 조합 표기가 이미 computation topology를 보여 준다면, 왜 그 의도를 loop로 잃어버린 뒤 다시 추론해야 하는가?**

RustJ의 J Graph IR은 이 질문에 대한 현재의 구현 답변이다.

따라서 RustJ의 중요한 compiler 원칙은 **이 정보를 너무 일찍 scalar loop, buffer, kernel로 낮추지 않는 것**이다. `/`와 `"` 같은 modifier identity, rank boundary, derived structure는 J Semantic IR/J Graph에서 보존하고, Semantic Analyzer 이후에만 `Reduce`, `CellApply`, `Scan`, reindex 등의 normalized logical operation으로 내린다. explicit loop/thread/block mapping은 더 downstream의 schedule/physical lowering에서 결정한다.

과거 JAXA가 주로 analyzer와 제한된 vocabulary를 대상으로 했다면, RustJ는 그 설계 비용을 승계해 **full-J frontend/semantic ownership + 점진적인 optimized backend coverage**로 확장한다. 분석 가능한 배열 영역은 aggressive logical/physical planning을 사용하고, 동적·effectful 영역은 J semantics를 보존하는 native/runtime route로 남길 수 있다.

과거 JAXA 문서의 표현:

> **JAXA specifies logical array intent, not physical execution procedure.**

> **JAXA does not execute fusion — the compiler does.**

는 이 원칙의 역사적 출발점으로 유지한다. 다만 current RustJ에서 더 정확한 장기 프레이밍은 **“GPU를 지원하는 J”가 아니라, J를 고수준 배열 언어로 사용하는 heterogeneous array compiler/runtime**이다.

이 프레이밍은 제품 범위나 구현 완료를 과장하기 위한 것이 아니다. current RustJ의 직접 목표는 여전히 **full J semantics를 보존하는 J compiler/runtime**이며, “배열 연산의 SQL”은 그 compiler layering과 optimization freedom을 설명하는 설계 비유다.


<a id="j-graph용-소스-대조최적화-계열-조사"></a>
### J Graph용 소스 대조·최적화 계열 조사

다음은 기존 `4.1.3의 근거 자료를 본문 흐름에서 분리한 것이다. 하위 제목·링크는 유지한다.

<a id="413-배열-컴파일-프레임워크와의-소스-기반-대조"></a>
#### 4.1.3 배열 컴파일 프레임워크와의 소스 기반 대조

검토일: 2026-10-02. 범위: J syntax가 만드는 graph 구조에서 얻을 수 있는 힌트를 다른 배열 컴파일 프레임워크의 공식 문서·실제 소스와 비교한다.
기준 문서: [정본 내 syntax 힌트](./05-graph-execution-lowering-p01.ko.md#syntax-graph-hints). leaf primitive의 산술 속성을 알아야 하는 변환과 syntax에서 발견할 수 있는 후보를 분리한다.

각 항목은 외부에서 확인한 사실, J에 적용하는 설계상의 추론, 추가 조건, 출처를 함께 기록한다. 코드는 열어 읽었지만 빌드·실행하지 않았다. 외부 GitHub `main` 링크는 이동하는 참조이며 고정 revision snapshot이 아니다. 구현 채택 시 revision을 고정하고 재검토해야 한다. RustJ의 통합 시점 코드 대조는 `89b87b8`을 기준으로 했다. CUDA 구현은 계속 보류한다.

<a id="4131-분류와-조사-결과의-대응"></a>
##### 4.1.3.1 분류와 조사 결과의 대응

| 일반 컴파일러 정보 | J graph에서 얻는 힌트 | 참고 프레임워크와 직접 출처 |
|---|---|---|
| 의존성·병렬성 | fork의 분기와 재결합 지점 | [TVM FuseOps](https://github.com/apache/tvm/blob/main/src/relax/transform/fuse_ops.cc) |
| 값 사용·수명 | hook의 원 입력 재사용, consumer 수, 영역 밖 사용 | [MLIR fusion](https://github.com/llvm/llvm-project/blob/main/mlir/lib/Dialect/Linalg/Transforms/ElementwiseOpFusion.cpp) |
| fusion 범위 | 합성 chain, fork diamond의 전체 영역, 여러 출력 | [XLA priority fusion](https://github.com/openxla/xla/blob/main/xla/backends/gpu/transforms/priority_fusion.cc) |
| 작업 분할·접근 패턴 | rank의 cell mapping, cut/window의 접근 영역 | [MLIR Linalg](https://mlir.llvm.org/docs/Dialects/Linalg/), [Halide lesson 8 소스](https://github.com/halide/Halide/blob/main/tutorial/lesson_08_scheduling_2.cpp) |
| 반복·상수·선택 | power의 carry, bond의 invariant, agenda의 nested graph | [JAX loops](https://github.com/jax-ml/jax/blob/main/jax/_src/lax/control_flow/loops.py), [JAX conditionals](https://github.com/jax-ml/jax/blob/main/jax/_src/lax/control_flow/conditionals.py) |
| 효과·적법성 | 후보 graph와 효과 의존성 분리 | [TVM Relax API](https://tvm.apache.org/docs/reference/api/python/relax/relax.html), [JAX loops](https://github.com/jax-ml/jax/blob/main/jax/_src/lax/control_flow/loops.py) |

<a id="4132-발견한-힌트와-j-적용-방법"></a>
##### 4.1.3.2 발견한 힌트와 J 적용 방법

<a id="f1--fork의-재결합-지점은-영역-전체-최적화-후보를-준다"></a>
###### F1 — Fork의 재결합 지점은 영역 전체 최적화 후보를 준다

**확인한 사실:** TVM `FuseOps`는 갈라진 경로가 다시 모이는 diamond를 다루기 위해 post-dominator 분석과 경로 검사를 사용한다. 단일 producer-consumer edge만 보는 것보다 영역 전체를 분석한다.

**J에서 얻는 힌트:** `(f g h) y`는 두 branch와 가운데 `g`를 명시한다. `((f g h)@:p) y`는 `p` 결과에서 갈라져 `g`로 모이는 diamond를 드러낸다. leaf 내부를 몰라도 branch별 parallel 및 전체 영역 fusion 후보를 만들 수 있다.

**조건:** 구문의 가운데 위치는 재결합 후보이지 전역 graph의 post-dominator 증명은 아니다. 영역 밖 consumer, effect edge와 선택 fork를 포함한 실제 graph를 분석해야 한다. `g`의 구현 계약은 fusion 실행의 후속 조건이다.

출처: [TVM fuse_ops.cc — fusion algorithm 주석과 GraphCreator](https://github.com/apache/tvm/blob/main/src/relax/transform/fuse_ops.cc), J 형태의 근거: [J Trains](https://www.jsoftware.com/help/dictionary/dictf.htm), [J At](https://www.jsoftware.com/help/dictionary/d622.htm).

<a id="f2--같은-producer의-consumer-수는-공유재계산fusion-선택-힌트다"></a>
###### F2 — 같은 producer의 consumer 수는 공유·재계산·fusion 선택 힌트다

**확인한 사실:** MLIR fusion 소스는 producer 결과가 다른 consumer에서도 사용되면 보존할 결과를 계산한다. 특정 reshape fusion 제어의 기본값은 producer의 단일 사용을 검사한다. 이는 모든 fusion에 단일 consumer를 요구한다는 뜻은 아니다.

**J에서 얻는 힌트:** `(u@:v) y`의 내부 결과는 전체 graph에서 단일 consumer인지 확인한다. `((f g h)@:p) y`의 `p` 결과는 두 branch에 공유된다. Hook `(f g) y`에서는 원 입력이 `g`와 downstream `f`에 사용되어 수명이 길어진다. 이 사용 관계는 syntax에서 만든 edge로 계산할 수 있다.

**활용:** 공유 결과를 한 번 materialize할지, branch마다 재계산할지, 여러 출력을 가진 fusion으로 보존할지 비교한다. 원 입력의 마지막 사용 전에는 버퍼를 회수하지 않는다.

**조건:** `p`를 복제하려면 효과·오류·binding 동등성이 필요하고 비용도 따져야 한다. 공유 logical value와 공유 physical allocation의 alias는 별개다.

출처: [MLIR ElementwiseOpFusion.cpp — getPreservedProducerResults, defaultControlFn](https://github.com/llvm/llvm-project/blob/main/mlir/lib/Dialect/Linalg/Transforms/ElementwiseOpFusion.cpp), [J Trains](https://www.jsoftware.com/help/dictionary/dictf.htm).

<a id="f3--fork에서는-parallel과-fusion을-경쟁-후보로-둔다"></a>
###### F3 — Fork에서는 parallel과 fusion을 경쟁 후보로 둔다

**확인한 사실:** XLA priority fusion 구현은 producer의 users와 fusion 가능성을 살피고, fused/unfused 실행시간 추정 차이를 우선순위에 사용한다. 일부 경로는 multi-output fusion도 검토한다.

**J에서 얻는 힌트:** fork의 두 branch는 별도 parallel 작업 후보이면서 같은 입력을 읽는 하나의 fusion 영역 후보이기도 하다. 합성 chain도 중간 배열 제거 후보가 된다. syntax는 실행 전략을 하나로 고정하지 않는다.

**활용:** 순차·병렬·공유 producer·재계산·fusion 후보의 작업량, 입력 재읽기, 중간 결과 크기를 비교한다. 비용 추정이 없는 단계에서는 안전한 기존 Rust 실행을 유지한다.

**조건:** leaf를 black box로 두면 graph 후보와 사용 횟수는 알 수 있어도 register pressure나 실제 kernel fusion 비용까지 알 수는 없다. CUDA를 구현하지 않고도 CPU 후보 분석을 설계할 수 있다.

출처: [XLA priority_fusion.cc — CalculateProducerPriority, EstimateRunTimes](https://github.com/openxla/xla/blob/main/xla/backends/gpu/transforms/priority_fusion.cc), [XLA architecture의 Fusion/Buffer Assignment](https://openxla.org/xla/gpu_architecture).

<a id="f4--rank-정보는-iteration-domain과-입력-mapping으로-보존한다"></a>
###### F4 — Rank 정보는 iteration domain과 입력 mapping으로 보존한다

**확인한 사실:** MLIR Linalg는 iteration 종류와 indexing maps를 계산 payload와 함께 구조적으로 표현한다. fusion 구현은 tensor 의미, iterator 조건, mapping과 loop bound 보존을 검사한다.

**J에서 얻는 힌트:** `u"n`은 cell 적용 영역을 지정한다. dyadic rank와 J agreement를 해석하면 frame iteration이 각 입력의 cell에 어떻게 대응하는지 얻는다. 이는 단순히 각 node에 숫자 rank 하나를 붙이는 것보다 많은 구조 정보다.

**활용:** `CellApply`에 iteration domain, 입력별 cell mapping, 결과 조립 mapping을 보존한다. mapping이 증명되면 cell별 batch 실행 및 cell-local 합성 후보를 만든다.

**조건:** frame을 곧바로 MLIR의 검증된 `parallel` iterator로 선언하지 않는다. 효과·결과 shape·empty prototype을 먼저 확인한다. NumPy broadcast나 affine mapping을 J agreement 대신 사용하지 않는다.

출처: [MLIR Linalg 공식 설명](https://mlir.llvm.org/docs/Dialects/Linalg/), [MLIR LinalgStructuredOps.td](https://github.com/llvm/llvm-project/blob/main/mlir/include/mlir/Dialect/Linalg/IR/LinalgStructuredOps.td), [MLIR areElementwiseOpsFusable](https://github.com/llvm/llvm-project/blob/main/mlir/lib/Dialect/Linalg/Transforms/ElementwiseOpFusion.cpp), [J Rank](https://www.jsoftware.com/help/dictionary/d600v.htm).

<a id="f5--합성-경계에-입력-mapping을-결합하면-복사-제거-후보가-된다"></a>
###### F5 — 합성 경계에 입력 mapping을 결합하면 복사 제거 후보가 된다

**확인한 사실:** MLIR의 elementwise fusion은 consumer iteration과 producer indexing map을 결합해 fused 입력 mapping을 만든다. XLA는 논리 shape와 physical layout을 구분하며 layout 충돌을 copy로 처리한다.

**J에서 얻는 힌트:** 합성 syntax는 producer-consumer 연결을 준다. 그 사이의 연산이 index mapping을 제공할 경우 graph에 mapping을 결합해 중간 배열을 피하는 후보를 만들 수 있다.

**정보 출처 구분:** 합성 연결은 syntax 정보다. transpose/reverse 등 구체적 index mapping은 해당 연산 의미에서 나온다. 합성 자체만으로 transpose가 metadata-only라고 주장하지 않는다.

**조건:** `@`/`@:`의 rank 경계, non-affine mapping, fill과 논리 원소 순서를 보존한다. 주소 span·alias·정렬은 physical 단계의 별도 증명이다.

출처: [MLIR fusion 소스 — consumerToProducerLoopsMap](https://github.com/llvm/llvm-project/blob/main/mlir/lib/Dialect/Linalg/Transforms/ElementwiseOpFusion.cpp), [XLA Layout Assignment](https://openxla.org/xla/gpu_architecture), [J At](https://www.jsoftware.com/help/dictionary/d622.htm).

<a id="f6--window의-겹침은-지역-재사용필요-영역-힌트를-준다"></a>
###### F6 — Window의 겹침은 지역 재사용·필요 영역 힌트를 준다

**확인한 사실:** Halide lesson 8은 inlining, compute_root, compute_at을 비교하며 중간 저장·중복 계산·지역성의 상충을 설명한다. 예제는 필요한 producer 영역만 계산하고 기존 행을 재사용하며 저장을 circular buffer로 축소하는 경우를 보여 준다. parallel loop가 끼면 그런 재사용이 제한되는 경우도 설명한다.

**J에서 얻는 힌트:** `k u\ y` 및 일부 `u;.n`은 window/tile의 경계·겹침을 드러낸다. producer-consumer 합성과 함께 있으면 필요한 입력 영역, 인접 window의 공유 영역, 마지막 조각을 분석할 수 있다.

**활용:** 전체 window 배열 생성, tile-local materialization, 순차 구간에서의 제한된 재사용을 경쟁 후보로 둔다. storage scope와 computation scope를 별개로 계획한다.

**조건:** J의 괄호·합성은 Halide `compute_at`/`store_at` schedule 선언이 아니다. 겹친 window는 출력 write 독립성을 보장하지 않으며 rolling sum의 수치 동등성도 별도 검증한다. tile 크기는 syntax에서 임의로 정하지 않는다.

출처: [Halide lesson 8 공식 설명](https://halide-lang.org/docs/tutorial/lesson_08_scheduling_2.html), [같은 lesson의 C++ 소스](https://github.com/halide/Halide/blob/main/tutorial/lesson_08_scheduling_2.cpp), [J Prefix/Infix](https://www.jsoftware.com/help/dictionary/d430.htm), [J Cut](https://www.jsoftware.com/help/dictionary/d331.htm).

<a id="f7--power에서는-carry와-invariant를-분리한다"></a>
###### F7 — Power에서는 carry와 invariant를 분리한다

**확인한 사실:** JAX loop 구현은 body graph의 constants와 carry를 구분한다. while 처리에는 그대로 전달되는 carry를 조건부로 body constant로 옮기는 분석이 있으며, scan은 carry의 shape/dtype 조건을 검사한다.

**J에서 얻는 힌트:** 비음수 정수 `u^:k`는 반복 영역과 loop-carried edge를 준다. noun bond로 결합된 입력 또는 body 안에서 변하지 않는 값은 invariant 후보가 된다.

**활용:** 불변 인자 준비·binding 검사·metadata 계산을 loop 밖으로 옮기는 후보를 만들고, 반복에 따라 변하는 값과 분리한다.

**조건:** J의 반복 결과는 shape/dtype가 바뀔 수 있으므로 JAX의 고정 carry 제약을 언어 규칙으로 도입하지 않는다. shape/dtype 안정성을 증명한 특수 경로에만 guard를 두고 일반 경로를 유지한다. 상태를 읽는 verb의 결과를 임의 hoist하지 않는다.

출처: [JAX loops.py — _check_carry_type, while_loop의 forwarding/constant 처리](https://github.com/jax-ml/jax/blob/main/jax/_src/lax/control_flow/loops.py), [JAX jaxpr의 loop 설명](https://docs.jax.dev/en/latest/601/jaxpr.html), [J Power](https://www.jsoftware.com/help/dictionary/d202n.htm), [J Bond](https://www.jsoftware.com/help/dictionary/d630n.htm).

<a id="f8--agenda와-중첩-train은-flat-dag-대신-영역으로-보존한다"></a>
###### F8 — Agenda와 중첩 train은 flat DAG 대신 영역으로 보존한다

**확인한 사실:** Jaxpr는 branch와 loop body를 sub-jaxpr로 표현한다. conditional 소스는 branch 효과를 합성하며, Relax의 If는 별도 branch SeqExpr를 가진다.

**J에서 얻는 힌트:** Agenda는 선택되는 verb/train 영역을, 중첩 train은 합성 영역의 topology를 드러낸다. scalar selector가 상수면 선택 해소 후보를 만들 수 있다.

**활용:** branch별 상수·shape·효과 facts의 범위를 보존하고, 선택되지 않은 branch를 실행하지 않은 채 내부 최적화를 계획한다.

**조건:** 일반 Agenda selector는 train 구성도 가능하다. 모든 경우를 binary if로 낮추거나 branch 값을 eager 계산한 뒤 선택하지 않는다. 괄호만으로 schedule fence를 추가하지 않는다.

출처: [JAX jaxpr](https://docs.jax.dev/en/latest/601/jaxpr.html), [JAX conditionals.py — _join_cond_effects](https://github.com/jax-ml/jax/blob/main/jax/_src/lax/control_flow/conditionals.py), [TVM expr.h — IfNode/SeqExprNode](https://github.com/apache/tvm/blob/main/include/tvm/relax/expr.h), [J Agenda](https://www.jsoftware.com/help/dictionary/d621.htm).

<a id="f9--값의-live-out과-효과의-live-out을-분리한다"></a>
###### F9 — 값의 live-out과 효과의 live-out을 분리한다

**확인한 사실:** Relax API는 일반 binding block과 pure dataflow block을 구분한다. MLIR fusion은 다른 consumer에 필요한 producer 결과를 보존한다. JAX loop 구현은 허용되는 효과를 별도로 검사한다.

**J에서 얻는 힌트:** `(f [ h)`의 fork는 parallel 후보를 주며 selector는 왼쪽 값만 반환됨을 알려 준다. 오른쪽 branch의 값이 영역 밖으로 나가지 않는다는 사실과 그 branch의 효과가 관찰되지 않는다는 사실은 다르다.

**활용:** graph의 외부 결과 목록과 effect 결과/의존성을 분리한다. ‘값이 미사용’이라는 이유만으로 `loss emit`을 지우지 않고, 반환 payload를 줄일 수 있는지와 효과를 유지할지를 따로 검사한다.

**조건:** J fork를 무조건 TVM pure dataflow block으로 선언하지 않는다. syntax는 후보와 값 흐름을 제공하고 순수성·효과는 별도 계약에서 얻는다.

출처: [TVM Relax DataflowBlock API](https://tvm.apache.org/docs/reference/api/python/relax/relax.html), [MLIR getPreservedProducerResults](https://github.com/llvm/llvm-project/blob/main/mlir/lib/Dialect/Linalg/Transforms/ElementwiseOpFusion.cpp), [JAX loop effect 검사](https://github.com/jax-ml/jax/blob/main/jax/_src/lax/control_flow/loops.py), [J Trains](https://www.jsoftware.com/help/dictionary/dictf.htm).

<a id="4133-사용자-예에-적용한-결과"></a>
##### 4.1.3.3 사용자 예에 적용한 결과

`(loss_adjoint [ (loss emit))`는 바깥 fork syntax에서 F1/F3의 parallel 후보를 만든다. 내부 `(loss emit)`이 두 verb의 hook이면 F2의 원 입력 재사용과 producer-consumer edge를 보존한다. `[`의 선택 의미로 F9의 값 live-out을 얻는다.

이는 하나의 구문에서 다음 세 정보를 분리해 추출하는 사례다.

- **Topology**: 두 branch와 재결합 위치 — syntax에서 얻는다.
- **Value use**: 왼쪽 값 반환, 내부 hook의 원 입력 사용 — 조합/selector 의미에서 얻는다.
- **Legality**: 공유 slot·I/O·alias·오류 충돌 — 후속 계약과 분석에서 얻는다.

Topology가 parallel 후보의 출처라는 점을 유지한다. 외부 프레임워크는 이 후보를 재결합·사용 횟수·영역·비용·효과 정보로 정교화하는 참고 자료다. 해당 구문에 관한 해석 근거는 [J Trains](https://www.jsoftware.com/help/dictionary/dictf.htm), 비교 근거는 [TVM fusion](https://github.com/apache/tvm/blob/main/src/relax/transform/fuse_ops.cc), [MLIR fusion](https://github.com/llvm/llvm-project/blob/main/mlir/lib/Dialect/Linalg/Transforms/ElementwiseOpFusion.cpp)이다.

<a id="4134-j-graph-ir에-추가할-정보와-우선순위"></a>
##### 4.1.3.4 J graph IR에 추가할 정보와 우선순위

아래 필드와 기능은 설계 제안이며 현재 구현된 API가 아니다. 다른 프레임워크의 IR나 purity 제약을 그대로 복사하지 않는다.

| 우선순위 | 보존/계산할 graph 정보 | 활용 | 근거 |
|---|---|---|---|
| 1 | syntax origin, region topology, data/effect edge 분리 | Fork 후보의 출처와 거부 이유 설명 | F1/F9: [TVM fusion](https://github.com/apache/tvm/blob/main/src/relax/transform/fuse_ops.cc), [Relax API](https://tvm.apache.org/docs/reference/api/python/relax/relax.html) |
| 1 | consumer 목록, 원 입력 사용, region live-out | Hook 수명, 공유 결과 보존 | F2: [MLIR fusion](https://github.com/llvm/llvm-project/blob/main/mlir/lib/Dialect/Linalg/Transforms/ElementwiseOpFusion.cpp) |
| 1 | cell/frame domain, 입력별 logical mapping | G2–G3의 rank/view 연결 | F4/F5: [MLIR Linalg](https://mlir.llvm.org/docs/Dialects/Linalg/) |
| 2 | 재결합 후보와 실제 post-dominator/외부 사용 | 영역 전체 fusion 탐색 | F1: [TVM fusion](https://github.com/apache/tvm/blob/main/src/relax/transform/fuse_ops.cc) |
| 2 | parallel/shared/recompute/fusion 후보 비용 | 한 전략을 syntax에서 강제하지 않음 | F3/F6: [XLA 소스](https://github.com/openxla/xla/blob/main/xla/backends/gpu/transforms/priority_fusion.cc), [Halide 소스](https://github.com/halide/Halide/blob/main/tutorial/lesson_08_scheduling_2.cpp) |
| 3 | loop carry/invariant와 branch-local facts | Power/Agenda 구문 지원 후 영역 최적화 | F7/F8: [JAX loops](https://github.com/jax-ml/jax/blob/main/jax/_src/lax/control_flow/loops.py), [JAX conditionals](https://github.com/jax-ml/jax/blob/main/jax/_src/lax/control_flow/conditionals.py) |

2026-10-02 프레임워크 비교 당시의 코드 대조 기준(`89b87b8`)에는 `src/j_graph_ir.rs`의 explicit stage/branch graph, `GraphForm`/`GraphHint`, use-count/liveness 및 symbolic resource 분석 seam이 있다. F1–F9는 이 구조를 보강할 설계 근거이며 새 최적화 실행의 완료 보고가 아니다. 현재 구현 상세는 A1.5를 따른다. G1–G5 우선순위를 유지하고, 미지원 syntax 전체 구현이나 실제 CUDA 작업을 이번 조사에 포함하지 않는다.

<a id="4135-체크리스트"></a>
##### 4.1.3.5 체크리스트

- [x] JAX, XLA, MLIR Linalg, TVM Relax, Halide의 공식 자료와 관련 소스를 읽었다.
- [x] F1–F9마다 관찰 사실·J 적용 추론·조건·출처를 기록했다.
- [x] fork syntax의 parallel 후보와 leaf primitive의 kernel 정보를 분리했다.
- [x] source 링크를 각 발견과 우선순위 항목에 붙였다.
- [ ] 구현 채택 시 프레임워크 revision을 고정하고 근거를 재확인한다.
- [ ] named black-box verb를 사용해 topology/consumer/live-out 추출을 검증한다.
- [ ] 숨은 효과가 있는 fork의 후보 생성과 실행 거부를 별도로 검증한다.
- [ ] 공유 producer 복제, hook 입력 조기 회수, Agenda 비선택 실행을 차단하는 회귀를 추가한다.
- [ ] rank mapping·empty prototype·shape 변화 power를 검증한다.
- [ ] 실행 연결 후 Windows 네이티브에서 값·오류·효과 순서와 복사/할당/성능을 비교한다.

이번 변경은 문서만 작성했다. 외부 프레임워크 빌드, Rust/C 실행 테스트, 성능 측정 또는 CUDA 검증은 수행하지 않았다.

<a id="4136-jsource에서-채굴한-최적화-원리와-rustj-적용-위치"></a>
#### 4.1.3.6 jsource에서 채굴한 최적화 원리와 RustJ 적용 위치

검토 기준: `jsoftware/jsource` source review pin **`13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`**. 이 절은 current jsource의 hand-written specialization을 RustJ에 그대로 복제하기 위한 목록이 아니다. jsource가 오랜 기간 축적한 **J-specific optimization knowledge를 추출해 Graph IR의 fact/rewrite 체계와 downstream execution planning으로 재배치하기 위한 source-derived design input**이다.

핵심 해석:

~~~text
jsource
  derived verb / train structure
      ↓
  propagated flags + special-form recognition
      ↓
  specialized entry point / rank path / virtual block / inplace path

RustJ
  J semantic construction + applied J Graph
      ↓
  target-independent facts / provenance / rewrite candidates
      ↓
  proof / demand / rank-shape / materialization analysis
      ↓
  Execution Semantic Lowering
      ↓
  bufferization / scheduling / CPU-GPU-library realization
~~~

따라서 **jsource의 아이디어를 Graph IR에 “특수 함수”로 복제하지 않는다.** J Graph IR은 우선 source 구조·적용 topology·최적화 후보를 발견하고, call-dependent 사실과 legality는 Execution Semantic Lowering 및 proof 단계에서 확정하며, concrete buffer reuse·cache-blocking·SIMD routine 선택은 후속 실행계획에서 담당한다. 아래는 **대표 사례 목록**이지 jsource 전수 조사 완료를 뜻하지 않는다.

<a id="a-source-derived-optimization-catalog"></a>
##### A. source-derived optimization catalog

| jsource 관찰 | 직접 근거 | RustJ에서 추출할 일반 원리 | 적용 위치 |
|---|---|---|---|
| derived verb가 downstream execution에 필요한 성질을 flag로 전달 | `ca.c::jtatop/jtatco`, `jtype.h`의 `VF2*` | source-derived operation traits와 실행 엔진 전용 hint를 분리한다. `WILLOPEN/USESITEMCOUNT`는 J 의미 불변조건 자체가 아니라 안전한 조건에서 전파되는 Result Assembly 소비자/생산자 실행 계약이다. | **J Graph provenance + call-dependent facts / 후속 assembly** |
| `@:`/capped fork/atomic reduce-composition을 special form으로 인식 | `ca.c`의 `SPECAT/SPECATCO`, `cf.c::jtfolk`; capped fork 저장형을 `f@:g`와 같은 실행 구조로 다루는 주석 | 서로 다른 J construction이 같은 applied topology를 만들 수 있음을 이용하되 source provenance는 유지한다. composition은 producer-consumer fusion/materialization 후보의 강한 출처다. | **J Graph canonicalization + candidate discovery** |
| atomic `f/@:g`에서 중간 배열을 줄이는 cell-at-a-time 경로 | `ca.c::jtatco`, `va2.c::jtfslashatg` | **일반적인 단일 fused kernel의 증거가 아니다.** dense·비어 있지 않음·dtype compatibility·in-place 유리 여부 등 조건을 검사하며 일부 경우 generic path로 돌아간다. Graph에서는 transform→reduce와 streaming/materialization-elision 후보만 기록한다. | **Graph 후보 → legality/Execution scheduling** |
| nested rank loop를 합치거나 outer rank loop가 inner rank processing을 흡수 | `jtype.h`의 `VF2RANKATOP1/2`, `VF2RANKONLY1/2`; `ca.c`의 “subsumed into a higher rank loop”; `cr.c`의 IRS/rank path 선택 | rank를 단순 wrapper가 아니라 cell/frame iteration-domain 정보로 보존하고 compatible nested rank/CellApply domain의 absorption/fusion 후보를 만든다. | **Graph/Execution semantic analysis**, concrete loop는 후속 |
| `+/%#` fork를 mean으로 특수화 | `cf.c::jtfolk`가 `jtmean`을 선택; `ar.c::jtmean`은 reduce 후 cell 길이로 나눔 | `Divide(Sum(x), Count(x))`의 high-level **mean 후보**를 인식할 수 있다. jsource 자체도 반드시 한 pass로 fusion하는 것은 아니며 floating order·rank·빈 셀·promotion 동등성 증명이 필요하다. | **Graph idiom 후보 → execution algorithm selection** |
| mean이 window/infix 안에 들어가면 moving average 경로 선택 | `ap.c::jtbslash/jtmovavg/jtmovsumavg`, 지원 입력에 대한 generic fallback | `Mean` 인식으로 window-specific 후속 후보가 열리는 점은 graph rewrite rediscovery의 근거다. 실제로는 **window aggregate algorithm/자료형 특수화와 fallback**이 중요하며 일반적인 재결합·부동소수점 순서 변경을 허용하지 않는다. | **Graph idiom rediscovery → Window execution plan** |
| `+/@:*"1 1`을 전용 sum-times rank-1 routine으로 전환 | `cr.c::jtsumattymes1` 선택, 구현은 `va2.c::jtsumattymes1` | dot-like contraction 후보지만 **특정 rank·dtype·empty·sparse·fit(특히 `!.0`, `!.1`) 분기**를 가진다. 일반 Dot 의미론으로 무조건 치환하지 않는다. | **Graph idiom 후보 + numeric/rank witness → backend routine** |
| `#@,`, `#@$`, `*/@$` 등을 rank/atom-count shortcut으로 처리 | `ca.c`의 `jtnatoms/jtrank`; `v.c::jtrank/jtnatoms`에서 sparse는 shape 기반 별도 경로 | symbolic shape/rank/count와 **value demand 분리**의 근거다. sparse/empty/prototype 및 원 연산의 관찰 가능한 check/error 조건을 보존한다. | **Graph facts/shape rewrite + semantic guard** |
| `BOXATOP/WILLOPEN/ATOPOPEN/USESITEMCOUNT`로 Result Assembly와 consumer가 협력 | `ca.c` 주석·전파, `cr.c` rank result-assembly, `jtype.h`, `result.h` | **`Box→Open`의 보편적 대수적 소거가 아니다.** virtual contents 보유, 재귀화·EPILOG 생략, raze를 위한 item count/shape 검사를 조립 중 수행하는 조건부 실행 협력이다. rank/boxed/sparse/alias/유형 균일성 조건을 보존한다. | **Graph consumer-demand 후보 → result assembly/materialization** |
| ravel 등에서 실제 copy 대신 virtual block 또는 header/shape 조정 | `v.c::jtravel`의 `virtual`, `ASGNINPLACESGN`, `AFNJA`, `AFUNINCORPABLE` | logical View 가능성을 노출하되 **모든 reshape/take/transpose가 zero-copy 또는 단순 stride view라는 뜻은 아니다.** representation·alias·pristinity·lifetime 검사가 필요하다. | **Graph/View facts → representation/bufferization** |
| comparison/search/set 조합을 전용 알고리즘으로 승격 | `ca.c`의 `jtranking` 및 comparison-combination/`I.@e.`; `cf.c::jtfolk`의 `jtintersect` (`#if C_VIAVX` 조건부) | Ranking/Intersection/Search 후보를 인식하되 `jtintersect`는 빌드별 SIMD capability에 의존하고 tolerance/type 조건도 따로 확인한다. 특수 경로의 존재와 J 의미 동등성 proof는 구분한다. | **Graph idiom candidate → target/algorithm selection** |
| use count와 inplaceability를 보고 입력 storage 재사용 | `v.c::jtravel`의 `ASGNINPLACESGN`, use count 및 `AFNJA/AFUNINCORPABLE`·pristinity 조건; `JTINPLACE*` 계열 | SSA liveness만으로 충분하지 않다. ownership, sharing/alias, recursive boxes, rank/result shape, observable error/retry까지 검증한 뒤 buffer를 재사용한다. | **후속 Buffer Planner / Physical lowering** |
| cache footprint, concrete primitive routine, AVX/SIMD 등에 맞춘 special execution path | `va2.c::jtfslashatg`의 cache-footprint 조건 및 각 primitive special entry point | Graph IR은 “fusable/streamable” 같은 freedom과 logical extent를 남기고 concrete chunk size·SIMD·GPU workgroup·library kernel 선택은 target/cost layer에서 결정한다. | **Target lowering / schedule / cost** |
| reduction은 빈 셀·길이 1·길이 2·자료형에 따라 경로를 바꿈 | `ar.c::jtreduce`의 `red0/head/jtreduce2`, `jtslash`의 specialized insert | shape facts로 neutral/singleton reduction 후보를 찾고, small-cell fast path는 downstream에 둔다. 빈 셀 identity·prototype·overflow·numeric contract가 우선이다. | **Graph shape/identity facts → Reduce lowering** |
| scan/infix는 일반 Window 반복과 별도 specialization이 있음 | `ap.c::jtbslash`, `jtpscan`, `jtmovfslash`의 sum/min/max/boolean/XOR 등 specialized paths | prefix Scan과 moving Window를 구분한다. sliding algorithm 후보는 access overlap/carry를 추적하되 부동소수점 순서·NaN·overflow fallback을 고려한다. | **Graph Scan/Window → specialized schedule** |
| index-of/membership/nub/search는 prehash·sort+binary search 등 서로 다른 구현을 사용 | `vi.c`의 `IPHIDOT/IPHEPS/...` mode, `jtiobs` sorting path | prehash mode 정의와 실제 fast-path eligibility는 다르다. `jtiobs`는 `ct=0` tolerance와 특정 boxed 고차원·numeric-box 형태에 제한된다. type/rank/tolerance·반복 검색·비용에 따라 hash/sort/generic route를 선택한다. | **Graph Search candidate → Execution algorithm/cost selection** |
| under/each는 단순 일반 역함수 실행 외 special structural path가 있음 | `cu.c`의 `u&.>` fast path, `u&.,`/index/cut의 `jtsunder`, `nameless(wvb)` inverse precomputation | forward/inner/inverse 의미, 동적 이름의 binding 시점, effect/alias를 보존한다. static inverse caching/structural route에는 증명 또는 가드가 필요하다. | **J Graph Under provenance → Execution lowering** |
| noun·상수 결합이 특정 산술 경로 선택을 가능하게 함 | `ca.c::jtatop/jtatco`의 `2&^.` floor/ceil-log와 modulus-bound modular-power 등 | bound operand를 specialization fact로 활용한다. arbitrary algebraic rewrite 대신 도메인·정확도·overflow·numeric type에 대한 witness를 요구한다. | **Graph bound-constant candidate → numeric specialized route** |

<a id="b-pinned-source-links"></a>
##### B. pinned source links

- [`jsrc/ca.c` — Result Assembly flag semantics](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ca.c#L269-L289) · [`jtatop/jtatco` patterns and propagation](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ca.c#L293-L516)
- [`jsrc/cf.c` — `jtfolk`, capped fork normalization, `+/%# → jtmean`, comparison/intersection specialization](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L55-L198)
- [`jsrc/cr.c` — rank/IRS selection and `+/@:*"1 1 → jtsumattymes1`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c#L779-L799)
- [`jsrc/ap.c` — dispatch to moving average](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ap.c#L940-L965) · [moving-average fallback](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ap.c#L769-L780)
- [`jsrc/va2.c::jtfslashatg` — cell-at-a-time path, fallback, overflow](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/va2.c#L1806-L1850)
- [`jsrc/v.c` — `jtrank/jtnatoms`, ravel virtual block and inplace/header reuse](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/v.c#L8-L40)
- [`jsrc/jtype.h` — BOXATOP/WILLOPEN/USESITEMCOUNT/RANKATOP/RANKONLY contracts](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/jtype.h#L1280-L1320)
- [`jsrc/result.h` — WILLBEOPENED/COUNTITEMS result-assembly contract](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/result.h#L20-L36)

- [`jsrc/ar.c` — `jtreduce` empty/singleton/two-item 처리, `jtslash`, `jtmean`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ar.c#L818-L849) · [mean 구현](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ar.c#L1018-L1025)
- [`jsrc/ap.c` — `jtmovfslash` window sum/min/max/boolean/XOR, `jtbslash` dispatch](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ap.c#L901-L965) · [numeric/fallback](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ap.c#L750-L780)
- [`jsrc/vi.c` — index-of/membership/nub modes + prehashed variants](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vi.c#L142-L184) · [sort + binary-search route](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vi.c#L804-L839)
- [`jsrc/cu.c` — under/each 및 structural-under fast paths](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cu.c#L391-L439)
- [`jsrc/va2.c` — dot-like `jtsumattymes1` type/rank/fit 경계](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/va2.c#L1640-L1687)

