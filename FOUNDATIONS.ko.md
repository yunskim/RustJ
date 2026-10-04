[English](FOUNDATIONS.md) | **한국어 — 정본(canonical)**

# RustJ 설계 기반 문서: 왜 J를 컴파일하는가

> 문서 성격: **설계 헌법 / mandatory architecture review**
>
> 이 문서는 구현 체크리스트가 아니다. RustJ가 왜 compiler-oriented architecture를 택하는지, 그 선택이 J의 역사와 의미론에 비추어 정당한지, 어떤 경우에 compiler가 실제로 J에서 후퇴(regression)가 되는지를 설명하는 **기본 판단 문서**다.
>
> 다음 영역을 변경하기 전에는 반드시 이 문서를 다시 검토한다.
>
> - word formation / enqueuer / parser
> - J Semantic IR / FunctionEntity / JEntity boundary
> - interpreter fallback / JIT / AOT 경계
> - rank / CellApply / hook / fork / modifier lowering
> - name binding / locale / execute / dynamic semantics
> - Logical IR / fusion / route partition
> - CPU/GPU target 및 target locale
> - compiler가 지원하지 못하는 J form의 처리 정책
>
> 실제 아키텍처와 구현 진행 상태의 권위 문서는 PROJECT.ko.md다. 이 문서는 그 설계가 **왜 그런 방향이어야 하는가**를 규정한다. 두 문서의 원칙이 충돌한다면 조용히 한쪽을 무시하지 말고, 설계 결정 자체를 다시 검토하고 두 문서를 함께 갱신한다.

---

## 1. 질문

RustJ는 J를 interpreter가 아니라 compiler-oriented system으로 만들고 있다.

그런데 여기에는 근본적인 의문이 있다.

> **APL과 J는 훨씬 자원이 부족했던 시대에도 interpreter를 선택했다. 그 선택에는 언어 구조상의 이유가 있었을 텐데, 오늘 RustJ가 compiler를 중심에 놓는 것이 오히려 회귀가 되는 것은 아닌가?**

이 질문은 단순한 역사 이야기가 아니다.

잘못 답하면 RustJ는 다음과 같은 언어가 될 수 있다.

~~~text
J-like source
   ↓
compiler 편의에 맞춘 정적 언어
   ↓
J와 비슷하지만 J는 아닌 언어
~~~

반대로 역사적 이유를 정확히 이해하면 다음이 가능하다.

~~~text
J semantics
   ↓
동일한 J frontend / semantic model
   ↓
상황에 따라
   ├─ AOT compilation
   ├─ JIT specialization
   ├─ runtime semantic execution
   └─ external compiler/library route
~~~

이 문서의 결론은 다음과 같다.

> **RustJ가 compiler를 택하는 것 자체는 회귀가 아니다.**
>
> **J semantics를 compiler에 맞게 축소한다면 회귀다.**
>
> RustJ의 올바른 목표는 “J를 정적 언어로 다시 설계하는 compiler”가 아니라 **J semantics를 소유하고, 분석 가능한 영역을 현대 하드웨어에 맞게 compile하는 execution system**이다.

### 1.1 핵심 불변조건 — Logical Array와 Physical Array는 같은 것이 아니다

이 문서가 보호해야 할 핵심 경계 중 하나는 **J가 관찰하는 배열의 의미와 backend가 사용하는 물리 표현을 분리하는 것**이다.

```text
Logical Array
    J-visible type/value
    shape
    ordered atoms
    boxed/sparse 등 J-visible semantics

        ≠

Physical Array / Representation
    buffer/storage
    strides
    offset
    layout/tiling
    alignment
    memory space
    device placement
    sharding/transfer
```

이 분리가 필요한 이유는 단순하다.

**J semantics는 특정 memory layout이나 device에서 정의되지 않는다.**

예를 들어 transpose는 J에서는 logical axis/order 변환이다. 그것을 stride-only view로 유지할지, consumer가 index map을 흡수할지, 실제 copy를 만들지는 backend와 schedule에 따라 달라질 수 있다. 이 선택을 J noun의 semantic identity에 넣으면 implementation detail이 언어 의미로 역류한다.

따라서 다음은 설계 불변조건이다.

- logical shape/order/type과 physical stride/layout을 동일시하지 않는다.
- `ValueId`와 `BufferId`를 동일시하지 않는다.
- logical value 존재와 materialized memory buffer 존재를 동일시하지 않는다.
- CPU/GPU placement를 J value identity로 만들지 않는다.

### 1.2 공통 J entity와 array semantics를 혼동하지 않는다

jsource의 공통 `A` handle은 중요한 선례지만, 그 구현을 곧바로 “noun과 verb는 모두 같은 array다”라는 의미론으로 읽으면 안 된다.

```text
semantic RHS        JEntity = Noun | Function
function POS        Verb | Adverb | Conjunction
array semantics     Noun의 J-visible type / shape / ordered atoms
function semantics  FunctionEntity의 construction / POS / operands / contracts
reference/control   lexical NAME / NameRef / binding version / provenance
physical            buffer / layout / device / allocator
```

따라서 다음은 mandatory invariant다.

- `JEntity`는 parser·binding·assignment·semantic operand 경계의 **얇은 carrier**다. `Value`와 `FunctionEntity`의 내부 모델을 합치지 않는다.
- lexical NAME/unresolved reference는 JEntity POS가 아니다. lookup 결과로 만들어진 executable function nameref는 FunctionEntity 내부의 `NameRef` identity가 될 수 있다.
- noun lookup snapshot과 function nameref late lookup의 observable timing 차이를 공통 carrier가 지우면 안 된다.
- Verb/Adverb/Conjunction 자체에 noun-style shape/rank를 부여하지 않는다.
- jsource의 gerund 실행용 fake-BOX/function payload carrier는 semantic `EntityArray`의 증거가 아니다.
- gerund와 유사한 higher-order semantics는 먼저 operator-specific `GerundView` / `InterpretedEntitySequence`로 표현한다.
- generic `EntityCollectionView`는 둘 이상의 독립적인 J semantics에서 동일한 shaped-entity law가 확인된 뒤에만 추출한다.
- `JEntity` 도입은 중복 carrier seam을 줄이는 incremental migration이어야 하며, frontend 전체를 한 번에 다시 쓰는 broad rewrite가 되어서는 안 된다.
- backend가 원하는 broadcasting/layout semantics로 J agreement/rank semantics를 바꾸지 않는다.
- sparse/boxed의 **J-visible semantic representation**과 CSR/COO/pointer/handle 같은 **backend encoding**을 구분한다.
- physical realization을 바꾸더라도 같은 logical array semantics를 관찰해야 한다.

이 경계 덕분에 같은 logical value에 대해 여러 physical representation을 선택할 수 있고, 반대로 서로 다른 logical values가 lifetime 분석을 통해 같은 physical buffer를 재사용할 수도 있다.

즉 RustJ에서 physical optimization의 자유는 **logical semantics를 먼저 고정했기 때문에** 생긴다.

```text
J logical semantics
        ↓
analysis / legality
        ↓
representation choice
        ↓
physical planning
        ↓
buffer / layout / device
```

compiler가 이 순서를 뒤집어 physical representation을 먼저 정한 뒤 J 의미를 거기에 맞추기 시작하면, 그것은 RustJ가 피해야 할 regression이다.

---

# Part I. 역사적 사실: 왜 APL은 interpreter였는가

## 2. 첫 APL 구현은 왜 interpreter였는가

Falkoff와 Iverson의 회고인 *The Evolution of APL*은 이 질문에 거의 직접 답한다.

1965년 실제 구현을 시작할 때 두 가지 목표가 있었다.

1. 구현은 우선 **experimental**이어야 했다.
2. machine representation 때문에 언어가 타협하는 일을 **최소화**해야 했다.

그 조건에서 Larry Breed와 Philip Abrams가 interpreter를 제안하고 구현했다. Iverson은 후에 APL 구현들이 interpretive 형태를 오래 유지한 이유를, compilation이 특별히 어렵기 때문이 아니라 **array language라서 interpretation 자체가 상당히 효율적일 수 있기 때문**이라고 설명했다. [H1]

이 설명은 RustJ에 매우 중요하다.

초기 선택은 다음이 아니었다.

~~~text
compiler를 만들 기술이 없었다
        ↓
어쩔 수 없이 interpreter
~~~

오히려 다음에 가까웠다.

~~~text
언어 의미를 machine에 종속시키지 않는다
+
실험 가능한 interactive implementation이 필요하다
+
array primitive가 큰 단위의 일을 수행한다
        ↓
interpreter가 합리적이다
~~~

### 2.1 선언을 넣지 않은 이유와 interpreter

같은 문헌은 declaration 문제도 설명한다.

초기 APL 설계자들은 구현을 쉽게 하려면 declaration이 필요하다는 제안을 받았지만 이를 거부했다. arrays가 primitive인 언어에서 declaration이 실제 값이 이미 가지고 있는 정보와 중복되거나 충돌할 수 있다고 보았고, interpretive implementation 덕분에 declaration 없는 설계를 현실적으로 유지할 수 있었다. [H1]

이것은 RustJ에 그대로 적용되는 경고다.

다음과 같은 방향은 위험하다.

~~~text
GPU compiler가 shape를 알아야 한다
      ↓
사용자에게 shape declaration 요구

compiler가 type을 알아야 한다
      ↓
J source에 static type declaration 추가

optimizer가 verb를 알아야 한다
      ↓
name rebinding을 금지
~~~

이것은 performance implementation의 필요를 **언어 요구사항으로 역류**시키는 것이다.

RustJ는 반대로 해야 한다.

~~~text
J source는 J 그대로
      ↓
실제 binding/value/semantic contract
      ↓
Analyzer가 알 수 있는 것을 추론
      ↓
모르면 guard / JIT / runtime semantic path
~~~

---

## 3. “컴퓨터가 느렸는데 왜 interpreter였나?” — 배열 언어의 granularity

1968년 APL\360 Terminal System 문서는 이 점을 더욱 구체적으로 보여 준다.

당시 시스템은 **completely interpretive**였고, 사용자 정의 프로그램은 대체로 입력 형태로 workspace에 보관되었다. 그런데 문서는 array operation 때문에 translation/interpretation overhead가 실제 계산량에 비해 작을 수 있다고 설명한다. 예로 matrix multiplication과 +/⍳2000 같은 연산을 든다. [H3]

핵심은 **dispatch granularity**다.

일반 scalar interpreter를 단순화하면 다음과 같다.

~~~text
load
dispatch
add
dispatch
store
dispatch
increment
dispatch
branch
dispatch
...
~~~

연산 하나가 작기 때문에 interpreter dispatch cost가 자주 나타난다.

APL/J에서는 다음과 같은 한 primitive invocation이 이미 큰 계산일 수 있다.

~~~text
a + b
      ↓
한 번의 primitive dispatch
      ↓
N개 atom에 대한 덧셈
~~~

따라서 대략적인 비용을 다음처럼 생각할 수 있다.

~~~text
T_interpret
  ≈ T_word/parser
  + Σ (T_dispatch(i) + T_array_primitive(i))
~~~

N개의 atom을 다루는 primitive라면 대략:

~~~text
T_dispatch = O(1)
T_primitive = O(N)
~~~

이므로 N이 커질수록 dispatch의 상대적 비중은 작아진다.

이것이 “APL/J가 interpreter여도 빠를 수 있다”는 주장의 핵심이다.

### 3.1 중요한 오해: J interpreter는 scalar bytecode interpreter가 아니다

“interpreter”라는 한 단어만 보고 Python bytecode interpreter와 같은 성능 구조를 상상하면 안 된다.

J의 primitive는 C 구현에서 배열 전체를 처리하며, rank/cell 처리에도 specialized path가 있고, derived function construction 단계에서 실행 함수를 선택하거나 결합된 형태를 special-case한다.

현재 jsource에는 예를 들어 다음이 있다.

- primitive table에 innate rank와 실행 함수가 등록됨
- VIRS1/VIRS2 같은 rank-loop absorption 능력
- VFUSEDOK2 같은 atomic/fused execution capability
- rank-specific execution functions
- derived verb construction 시 special implementation 선택
- +/%# fork를 mean implementation으로 연결하는 specialization

[S4][S5][S6][S7]

따라서 공정한 비교는 다음이 아니다.

~~~text
naive interpreter
vs
optimizing compiler
~~~

실제로는 더 가깝다.

~~~text
고도로 최적화된 array interpreter
+ semantic construction specialization
+ tuned C kernels

vs

semantic-preserving whole-graph compiler
+ tuned/native/library kernels
~~~

RustJ는 후자여야 한다.

---

## 4. 메모리가 부족한 시대에도 interpreter가 가능했던 이유

APL\360 문서는 당시 workspace가 약 36,000 System/360 bytes였다고 설명한다. 사용자 정의 프로그램은 상당 부분 입력 형태로 유지했기 때문에 비교적 적은 storage를 사용했다. [H3]

또한 APL의 초기 time-sharing 역사에 대한 McDonnell의 회고는 작은 System/360 환경과 workspace 중심 설계가 매우 효율적인 time-sharing을 가능하게 했음을 설명한다. 당시 한 예에서는 256KB 메모리의 360/50 환경이 언급된다. [H4]

이 점에서 resource scarcity와 compiler/interpreter 선택의 관계는 단순하지 않다.

### 4.1 CPU가 부족하면 무조건 compiler가 유리한 것이 아니다

compiler는 runtime instruction throughput을 개선할 수 있다.

하지만 당시의 전체 시스템 비용에는 다음도 있었다.

- compiler 자체의 코드와 working storage
- generated machine code storage
- compile latency
- interactive edit/execute cycle
- 사용자 workspace swapping
- 많은 작은 함수/정의의 관리

APL은 이미 primitive 하나가 많은 계산을 담당했기 때문에, program text를 compact하게 유지하고 primitive implementation을 공유하는 interpreter 구조가 충분히 경쟁력이 있었다.

~~~text
source function 1
source function 2
source function 3
       │
       └───────────────┐
                       ↓
                shared primitive +
                shared primitive ×
                shared primitive /
~~~

모든 사용자 함수를 각각 machine code로 materialize하지 않아도 됐다.

### 4.2 workspace와 interactive use

APL\360의 핵심 경험은 workspace와 즉시 실행이었다. terminal에서 문장을 입력하면 곧바로 평가되고 결과를 볼 수 있었다. [H3]

interpreter는 이 interactive model과 매우 잘 맞는다.

~~~text
edit
 ↓
execute immediately
 ↓
inspect values
 ↓
rebind names
 ↓
execute again
~~~

이는 단순 UI 문제가 아니다.

APL/J에서는 **현재 workspace/locale/name environment 자체가 의미의 일부**가 될 수 있다.

따라서 interpreter는 당시의 사용 모델과 언어 모델 모두에 자연스러웠다.

---

# Part II. J가 interpreter 구조를 계승한 이유

## 5. J의 출발점 자체가 작은 interpreter였다

Roger Hui의 *An Implementation of J*에 따르면 1989년 Arthur Whitney는 Ken Iverson을 방문해 한 페이지 분량의 interpreter fragment를 만들었고, Hui는 그것을 약 일주일 연구한 뒤 J 구현을 시작했다. [H5]

이 사실은 단순한 일화 이상이다.

초기 fragment에도 이미 중요한 개념이 있었다.

- arrays
- primitive function
- operator
- name assignment
- boxed values의 기초

즉 J의 구현 전통은 처음부터:

~~~text
J semantic objects
+
작은 parser/evaluator
+
array primitives
~~~

를 중심으로 출발했다.

---

## 6. J parser는 언어 명세와 implementation을 일부러 가깝게 만들었다

Roger Hui는 *Remembering Ken Iverson*에서 J parser를 *A Dictionary of APL*의 parsing process를 중심으로 의식적으로 설계했다고 회고한다. C 자료구조와 parser 코드가 dictionary의 parse table과 직접 대응하도록 하는 것이 목표였고, Iverson이 C source를 보고 syntax correctness를 검증할 수 있을 정도의 대응을 의도했다. [H6]

현재 jsource의 p.c도 같은 철학을 명시한다.

[S2]의 상단 comment에는 parsing이 Dictionary Chapter E를 따른다고 적혀 있고, parser stack은:

- word/value
- original sentence token number
- parse-table matching mask

를 가지고 동작한다.

또한 선언형 cases[]에는 9개 reduction row가 명시되어 있다.

~~~text
0 monad
1 monad after verb phrase
2 dyad
3 adverb
4 conjunction
5 fork
6 hook / bident / trident
7 assignment
8 parentheses
~~~

즉 J interpreter에서 parser는 단순한 syntax tree builder가 아니다.

parser가:

- 현재 name binding을 lookup하고
- 현재 part of speech를 결정하며
- modifier semantic action을 수행하고
- derived function을 만들고
- 일부 verb application을 실행하며
- 그 결과를 다시 stack에 넣고 parsing을 계속한다

는 점이 중요하다.

따라서 J를 일반적인 다음 구조로 재해석하면 semantics를 놓칠 수 있다.

~~~text
source
 ↓
static AST
 ↓
later evaluation
~~~

J의 실제 구조는 더 동적이다.

~~~text
word/enqueue
 ↓
parse-time semantic resolution
 ↓
completed J entities
 ↓
further parse reduction
~~~

이것이 RustJ frontend를 jsource-compatible하게 유지해야 하는 근본 이유다.

---

## 7. Enqueuer도 parser의 일부라고 봐야 한다

현재 jsource의 w.c에는 두 가지 핵심 단계가 함께 있다. [S1]

1. jtwordil: state-machine word formation
2. jtenqueue: word를 parse 가능한 J object로 변환

jtenqueue는 primitive spelling을 인식해 shared primitive object를 가져오고, 그렇지 않으면 name/numeric/string/assignment 등을 J 규칙에 따라 분류한다.

즉 다음 경계는 언어 semantics의 일부다.

~~~text
bytes
 ↓
word formation
 ↓
primitive / name / noun / assignment classification
 ↓
parse queue
 ↓
parser
~~~

RustJ가 frontend에서 compiler-friendly shortcut을 만들면 안 되는 이유다.

따라서 RustJ의 현재 원칙은 옳다.

> **word formation → enqueue → parse까지는 jsource를 사실상 구현 명세로 삼는다.**

RustJ의 독자적인 compiler architecture는 그 뒤부터 시작해야 한다.

---

# Part III. J가 “컴파일 불가능한 언어”였던 것은 아니다

## 8. interpreter 선택과 compilability는 다른 문제다

Iverson의 설명은 매우 명확하다.

APL이 interpretive 형태로 남은 것은 compilation이 본질적으로 불가능하거나 지나치게 어려워서라기보다, array language 특성 덕분에 interpretation도 충분히 효율적이었기 때문이라는 것이다. [H1]

APL compiler 연구도 실제로 존재했다.

Robert Bernecky는 1990년대에 APL compiler 연구에 집중했고, *Compiling APL*과 APEX compiler 등의 작업이 이어졌다. [H7][C1][C2]

따라서 역사에서 다음 결론을 끌어내면 안 된다.

~~~text
APL/J가 interpreter였다
        ↓
array language는 compiler에 부적합하다
~~~

정확한 결론은 다음에 가깝다.

~~~text
APL/J가 interpreter였다
        ↓
interpreter도 충분히 실용적이었다
+
dynamic/interactive language design에 잘 맞았다
~~~

이는 compiler의 가능성을 부정하지 않는다.

---

## 9. 오히려 J는 compiler에 유리한 성질도 많다

J는 scalar loop를 길게 작성해서 compiler가 고수준 intent를 다시 추론해야 하는 언어가 아니다.

예를 들어 reduction은 source 자체에 드러난다.

~~~text
+/ y
~~~

rank도 source semantics에 직접 존재한다.

~~~text
u"r y
~~~

function composition도 syntax/semantic construction 자체에 드러난다.

~~~text
f @ g
f & g
f g h
~~~

따라서 scalar imperative language에서 compiler가 어렵게 복구해야 할 정보가 J에서는 이미 언어 구조로 존재하는 경우가 많다.

~~~text
scalar imperative source
   ↓
loop analysis
alias analysis
pattern recognition
   ↓
"이게 reduction인가?"

J source
   ↓
+/
   ↓
"reduction이라는 semantic identity가 이미 존재"
~~~

이 점은 modern array compiler에 매우 유리하다.

단, **그 정보를 너무 빨리 없애지 않는 것**이 전제다.

### 9.1 J 문법은 operation identity뿐 아니라 graph topology도 제공한다

과거 JAXA / jaxa-analyzer 설계에서 더 강하게 제기했던 주장은 여기서 한 단계 더 나간다.

~~~text
J syntax
  → operation kind를 알려 줌
  + graph topology를 알려 줌
  → optimization opportunity를 정적으로 노출
~~~

대표적으로:

~~~text
f @: g @: h
  → ordered Pipeline
  → producer-consumer intermediate의 materialization-elision/fusion 후보

hook / fork
  → Branch / Join
  → shared input, live-across value, join lifetime 후보

adjoint/VJP expansion
  → Fan-out
  → data adjoint와 parameter adjoint의 parallel execution 후보
~~~

이 정보는 일반 SSA DAG로 모두 표현 가능하지만, **표현 가능하다는 것과 source가 이미 알려 준 구조를 버려도 된다는 것은 다르다.**

일반 DAG로 먼저 평탄화한 뒤 pipeline/branch/join을 다시 pattern-match하면 J 문법이 제공한 정보를 버렸다가 복원하는 셈이다. RustJ는 따라서 semantic structure를 펼칠 때 target-independent StructuralOpportunity sidecar를 동시에 생성한다.

중요한 구분:

~~~text
syntax-derived opportunity
  ≠ semantic legality proof
  ≠ target feasibility
  ≠ chosen physical schedule
~~~

예를 들어 hook/fork branch가 계산 의존성만 보면 독립이어도 J error/effect order 때문에 병렬화할 수 없을 수 있다. 반대로 semantic상 합법이어도 register pressure, bandwidth, synchronization 비용 때문에 실제 target에서는 직렬 schedule이 더 나을 수 있다.

따라서 RustJ의 단계는:

~~~text
J combinator structure
  ↓
StructuralOpportunity
  ↓
Effect / error / numeric / access proof
  ↓
Target/resource feasibility
  ↓
FusionRegion / ParallelSchedule
~~~

이다.

이 구조는 과거 JAXA의 “J 표기가 fusion 구조를 정적으로 보이게 한다”는 장점을 유지하면서, register/shared memory/tile 같은 구체 hardware 결정을 Physical Planner에 늦추는 현행 RustJ 계층화와 양립한다.

---

# Part IV. 현재 jsource는 이미 “단순 interpreter”가 아니다

## 9.2 J Graph IR과 Execution IR은 다른 질문에 답한다

JAXA의 주 관심사는 실행 IR 자체보다 **J 표기를 계산 graph 대수로 사용하는 것**이다.

따라서 RustJ는 다음 두 IR을 의도적으로 구분한다.

~~~text
J Semantic Construction IR
  FunctionEntity / Hook / Fork / modifier / @:
          ↓
J Graph IR
  J 표기로 표현된 applied array-operation graph
  topology + static optimization hints
          ↓
Logical Execution IR
  explicit execution dataflow + execution basis + checks/facts
~~~

### J Graph IR

질문:

> 이 식은 J의 결합 대수로 어떤 계산 graph이며, 문법만 보고 어떤 optimization opportunity를 알 수 있는가?

여기서는 `@:` chain, hook/fork fan-out/fan-in, `/` reduction, `"` cell application 같은 구조를 **일반 SSA로 펼치기 전에** first-class로 본다.

J 문법에서 결정적으로 알 수 있는 것은 가능한 한 여기에서 기록한다.

- pipeline / branch / join
- shared input / common subflow
- syntactic reduction / cell application
- intermediate materialization-elision 후보
- retained-value lifetime 후보
- dependency상 parallel branch 후보
- observable J evaluation-order topology
- 향후 window/segment/contraction/iteration/adjoint topology
- primitive별 shape/dtype/rank/effect/resource rule identity 또는 Unknown

이 층은 JAXA의 graph algebra, **graph basis / rewrite / equivalence**, fusion topology 연구의 주 표면이다.

### Basis도 두 층이다

JAXA의 역사적 연구에서 말한 `basis verb`는 주로 **J Graph IR의 graph basis**를 뜻한다. 이것은 “실행기가 직접 제공해야 하는 최소 instruction 집합”이 아니라, J 식을 동등한 graph로 재작성하고 access/fusion/resource 구조를 분석할 때 사용하는 **대수적 생성원/구조 단위**다.

반면 Logical Execution IR에는 별도의 **execution basis**가 있다. 이것은 선택된 J graph를 정확히 실행하기 위해 normalized dataflow로 낮춘 뒤의 operation vocabulary다. 두 basis는 이름이 일부 겹칠 수 있어도 같은 타입이나 같은 최소성 조건을 공유하지 않는다.

~~~text
J Function / Derived Verb
        ↓
GraphBasis composition
        ↓
Logical Execution lowering
        ↓
ExecutionBasis composition
        ↓
backend / library / custom kernel
~~~

예를 들어 `conv`를 graph level에서 black box로 남긴다는 과거 결정은 **GraphBasis에서 convolution의 구조적 정체성을 보존한다**는 뜻이다. 이것은 execution lowering에서 convolution을 더 작은 operation으로 분해하지 말라는 뜻이 아니다.

~~~text
GraphBasis::Conv
  → ExecutionBasis::WindowView + Contract
  → ExecutionBasis::Reindex + Elementwise + Reduce
  → library/custom fused realization
~~~

어느 경로를 택하더라도 Graph IR의 원래 Conv identity와 equivalence/provenance를 잃지 않는다. 즉 **graph black-box 보존과 execution decomposition은 서로 모순되지 않는다.**

문법이 정보의 **근원**이라는 말과 parser node가 optimization metadata를 **소유**한다는 말은 다르다. parser/FunctionEntity는 J construction semantics를 정확히 보존하고, J Graph IR builder가 그 구조를 applied noun graph와 결합해 GraphForm/GraphHint를 결정적으로 유도한다. 이렇게 하면 JAXA의 정적 정보 이점을 살리면서 target/cost/pass-local fact가 parser 의미 객체로 역류하는 것을 막을 수 있다.

### Logical Execution IR

질문:

> 선택된 J graph를 J observable semantics를 보존하면서 정확히 실행하려면 어떤 operation과 dependency가 필요한가?

여기서는 CellApply, Reduce, Gather, `ExecutionBasisKind` / `ExecutionBasisPayload`, ResolvedInstantiation, SemanticCheck, EffectSummary, AccessFact 같은 **실행 계약**이 중심이다.

하나의 J Graph node가 여러 execution op로 펼쳐질 수 있다. 따라서 execution op는 `j_origin`을 보존하지만 J Graph IR을 대체하지 않는다.

### 중요한 비대칭

~~~text
J Graph IR  →  Execution IR
     자연스러운 lowering

Execution IR  →  J Graph IR
     일반적으로 원래 J 대수를 유일하게 복원할 수 없음
~~~

그러므로 pipeline/hook/fork 정보를 후자의 generic DAG에서 다시 pattern-match하는 것을 주 경로로 삼으면 안 된다. J 문법이 이미 준 정보를 전자에서 잃지 않는 것이 우선이다.

StructuralOpportunity는 두 IR 사이의 bridge다. source는 J Graph IR의 GraphForm/GraphHint이고, execution lowering이 이를 concrete execution values에 투영하여 legality/resource analysis가 사용할 수 있게 한다.

### 최적화도 두 층으로 나뉜다

~~~text
J Graph IR
  graph-basis algebra / rewrite / topology optimization
  fusion candidate discovery
  AD graph derivation
        ↓
Execution IR
  semantic proof / check elimination
  execution-basis expansion / access composition
  legal fusion confirmation
        ↓
Physical planner
  target resource feasibility / cost / schedule
~~~

이 분리는 JAXA의 핵심 주장과 RustJ의 compiler correctness 목표를 동시에 보존한다.

---

## 9.3 JAXA의 static-memory 주장은 logical extent와 physical allocation을 구분한다

JAXA가 J를 배열 연산 DSL로 사용할 때 얻는 장점 중 하나는 **graph를 실행하기 전에 배열 값의 shape, 크기, dependency와 lifetime을 상당 부분 정적으로 알 수 있다**는 점이다.

RustJ는 이 주장을 다음 두 층으로 해석한다.

~~~text
J Graph IR
  logical array extents
  atom counts
  use-def / fan-out
  live ranges
  materialization opportunities
  accumulator/temporary symbolic requirements
        ↓
Representation + Schedule + Target
        ↓
Physical resource planning
  register allocation
  shared/LDS/scratchpad bytes
  packed/layout/alignment
  tile-local storage
  physical buffers/offsets
  spill/occupancy
~~~

따라서 “메모리를 정적으로 결정한다”는 말은 **모든 J Graph value마다 즉시 GPU address와 register number가 정해진다**는 뜻이 아니다.

정적으로 결정 가능한 핵심은:

- 어떤 logical array value가 존재하는가
- shape가 알려지면 atom count가 얼마인가
- 어떤 value가 어느 consumer까지 살아 있어야 하는가
- 어떤 intermediate가 fusion으로 materialization을 피할 수 있는가
- 어떤 branch input이 join까지 retained되어야 하는가
- 어떤 reduction이 accumulator state를 요구하는가
- 선택한 representation을 주면 logical extent가 몇 bytes인지

반면 register/shared-memory/physical buffer는 `ResourceUsage = R(Graph, Schedule, Hardware)` 원칙에 따라 뒤에서 정한다.

이 분리를 지키면 JAXA의 static analyzability 이점을 유지하면서도 CPU/GPU/기타 backend에 공통인 compiler architecture를 보존할 수 있다.

현재 RustJ 구현에서 `j_graph_memory`가 logical extent/liveness/materialization 분석을, `j_graph_resource`가 topology-aware symbolic composition의 최소형을 담당한다.

---
## 10. derived function은 실행 가능한 semantic object다

예를 들어 /는 parser가 나중에 해석할 장식이 아니다.

jtslash는 operand를 받아 completed derived verb를 만든다. [S4]

따라서:

~~~text
+ /
~~~

는 이후 parser에서:

~~~text
+
/
~~~

두 조각으로 남아 있는 것이 아니라:

~~~text
+/
  └─ operand: +
~~~

이라는 하나의 completed J verb가 된다.

RustJ가 FunctionEntity를 first-class semantic node로 유지해야 하는 이유다.

---

## 11. rank는 interpreter 내부에서도 generic loop + specialization 구조다

jsource의 rank execution은 중요한 교훈을 준다.

cr.c의 jtrank1ex / jtrank2ex는 function rank와 actual argument rank를 사용해 frame/cell을 나누고 cell별 실행 및 결과 assembly를 수행한다. empty frame에서는 fill cell을 만들어 실행하는 특별 semantics도 있다. [S5]

동시에 VIRS 등의 flag를 사용해 일부 함수는 generic rank loop를 흡수하거나 특수 경로로 실행할 수 있다. [S6]

즉 이미 jsource 안에서도 개념적으로:

~~~text
semantic CellApply
       ↓
generic rank execution
       or
specialized absorbed execution
~~~

이라는 구분이 존재한다.

RustJ가 Logical CellApply와 target lowering을 분리하는 것은 J와 동떨어진 발상이 아니다.

오히려 jsource에 암묵적으로 섞여 있는 두 층을 더 명시적으로 분리하는 것이다.

---

## 12. J는 이미 local fusion/specialization을 한다

cf.c는 특정 fork 형태를 알아보고 specialized function을 선택한다. 대표적으로 +/%# 형태는 mean specialization으로 연결될 수 있다. [S7]

또 primitive metadata에는 fused/atomic/rank 지원을 나타내는 flag들이 존재한다. [S6]

이는 중요한 관찰이다.

현재 J implementation도 이미 다음을 한다.

~~~text
J semantic structure
      ↓
recognize known composition
      ↓
choose specialized executor
~~~

RustJ compiler가 하려는 일과 철학적으로 완전히 다른 것이 아니다.

차이는 **범위와 명시성**이다.

jsource:

~~~text
local derived-function construction
+
special handlers
+
primitive-level optimization
~~~

RustJ:

~~~text
preserved semantic graph
+
whole-region logical analysis
+
fusion / layout / placement / target planning
+
CPU/GPU/external routes
~~~

따라서 RustJ를 “J의 interpreter 전통을 버린다”고 보는 것보다:

> **J가 이미 하던 semantic specialization을 명시적인 compiler IR과 더 넓은 optimization scope로 확장한다**

고 보는 것이 정확하다.

---

# Part V. 그럼 왜 과거에는 whole-program compiler의 이득이 상대적으로 작았나

## 13. primitive 내부 최적화만으로도 큰 비율의 성능을 얻을 수 있었다

전통적인 APL/J workload에서 큰 array operation이 primitive 하나에 들어가면:

~~~text
parse/dispatch
 ↓
고도로 최적화된 C loop
~~~

만으로도 중요한 계산 대부분이 native code에서 수행된다.

예를 들어:

~~~text
+/ y
~~~

의 비용 대부분이 reduction primitive 내부에서 발생하면, source 전체를 machine code로 바꾸어도 없앨 수 있는 interpreter overhead는 상대적으로 작을 수 있다.

이는 1968년 APL\360 문서가 이미 지적했던 구조다. [H3]

### 13.1 compiler가 특히 이득을 얻는 영역

compiler의 가치가 커지는 것은 primitive **사이**의 비용이 중요해질 때다.

예:

~~~text
t1 = primitive A
t2 = primitive B(t1)
t3 = primitive C(t2)
~~~

interpreter가 각 primitive를 이미 잘 실행한다 해도:

- intermediate allocation
- memory traffic
- repeated traversal
- dispatch
- synchronization
- representation conversion

은 남는다.

whole-graph compiler는 이 경계를 최적화할 수 있다.

---

# Part VI. 현대 CPU/GPU에서 비용 구조가 달라졌다

## 14. GPU에서는 primitive 하나의 속도만으로 충분하지 않다

현대 discrete GPU에서는 계산 자체 외에 다음이 중요하다.

- host↔device transfer
- global-memory traffic
- intermediate materialization
- launch/synchronization
- workgroup/block mapping
- shared memory/register 사용
- memory coalescing

NVIDIA의 Best Practices Guide도 host/device transfer 최소화와 memory usage 최적화를 핵심 원칙으로 둔다. intermediate data를 device에 유지하고 여러 작은 transfer를 합치는 것이 중요하다고 설명한다. [G1]

따라서 다음 방식은 semantic하게 맞더라도 physical하게 비효율적일 수 있다.

~~~text
J primitive A
  ↓ GPU kernel launch
materialize temp
  ↓
J primitive B
  ↓ GPU kernel launch
materialize temp
  ↓
J primitive C
~~~

compiler는 전체 graph를 보면서:

~~~text
A → B → C
   ↓
fusion?
single traversal?
keep on device?
different layout?
library call?
~~~

를 판단할 수 있다.

이것은 전통적인 primitive-at-a-time interpreter가 본질적으로 못한다는 뜻은 아니다. interpreter에도 tracing/fusion/JIT를 추가할 수 있다.

그러나 그 순간 이미 compiler machinery를 도입하고 있는 것이다.

---

## 15. RustJ에서 compiler의 핵심 이득은 “dispatch 제거”가 아니다

RustJ compiler의 목적을 다음처럼 잡으면 약하다.

> interpreter dispatch를 없애기 위해 compile한다.

J에서는 dispatch가 이미 충분히 coarse-grained할 수 있기 때문이다.

RustJ의 더 중요한 compiler 목적은 다음이다.

1. **primitive 사이의 dataflow를 본다.**
2. **rank/cell semantics를 explicit logical form으로 만든다.**
3. **intermediate materialization을 줄인다.**
4. **fusion legality를 증명한다.**
5. **CPU/GPU placement와 transfer를 계획한다.**
6. **target architecture에 맞는 implementation을 선택한다.**
7. **library/custom kernel/external compiler를 함께 사용할 수 있다.**
8. **dynamic 부분과 static 부분의 경계를 guard/JIT로 관리한다.**

즉 compiler의 존재 이유는:

~~~text
primitive dispatch cost
~~~

보다:

~~~text
cross-primitive optimization
+
global execution planning
+
heterogeneous hardware mapping
~~~

에 있다.

---

# Part VII. 언제 compiler가 진짜 회귀가 되는가

## 16. 회귀 조건 1: J source를 compiler-friendly language로 바꿀 때

다음은 명백한 위험 신호다.

~~~text
shape declaration을 반드시 써야 함
type declaration을 반드시 써야 함
dynamic name rebinding 금지
runtime POS 변경 금지
execute 금지
boxed/sparse 일부 의미 삭제
~~~

compiler coverage를 넓히기 위해 J 자체를 제한한다면 RustJ는 J implementation이 아니라 J-inspired static language가 된다.

그것이 목표라면 별개의 프로젝트일 수 있지만 RustJ의 목표는 아니다.

---

## 17. 회귀 조건 2: parser를 일반 compiler parser처럼 재해석할 때

J의 parser는 semantics와 밀접하다.

따라서:

~~~text
source
 ↓
conventional static AST
 ↓
later semantic pass
~~~

로 바꾸면서 jsource의:

- parser-time name lookup
- actual POS resolution
- modifier application
- result POS
- right-to-left sequencing
- assignment interaction

을 잃으면 회귀다.

그래서 RustJ frontend 원칙은:

~~~text
word formation
    jsource-compatible
 ↓
enqueue
    jsource-compatible
 ↓
parser
    jsource-compatible
 ↓
J Semantic IR
~~~

이어야 한다.

[S1][S2]

---

## 18. 회귀 조건 3: derived J identity를 너무 일찍 지울 때

예를 들어 +/를 parser 직후 바로 generic Reduce(Add)로 바꾸고 원래 / application을 잃는다면:

- J semantic representation
- introspection
- error explanation
- modifier behavior
- name/derived function identity

를 나중에 복원하기 어려워진다.

따라서:

~~~text
Semantic:
  +/
  "/" applied to "+"

Logical lowering:
  Reduce(Add)
~~~

순서를 지킨다.

마찬가지로:

~~~text
Semantic:
  u"r

Logical:
  RankBoundary / CellApply facts
~~~

를 구분한다.

---

## 19. 회귀 조건 4: rank를 단순 broadcasting으로 바꿀 때

J rank semantics는 NumPy-style trailing broadcasting과 다르다.

특히 다음을 보존해야 한다.

- innate rank
- explicit requested rank
- frame/cell
- prefix frame agreement
- residual repetition
- zero-cell fill execution
- heterogeneous cell result assembly
- error behavior

jsource의 generic rank executor가 상당히 복잡한 이유도 여기에 있다. [S5]

compiler가 단순한 map/broadcast model로 바꾸면 J에서 후퇴한다.

---

## 20. 회귀 조건 5: observable error/effect order를 optimizer가 바꿀 때

J에서 evaluation 중 error와 assignment/name lookup은 관찰 가능할 수 있다.

따라서 다음 최적화는 자동으로 안전하지 않다.

~~~text
reorder
parallelize
speculate
common-subexpression eliminate
~~~

pure/speculatable proof 없이 수행하면 semantics가 달라질 수 있다.

특히 hook/fork branch를 GPU에서 병렬 실행하고 싶더라도 J에서 observable한 error/effect order를 먼저 보존해야 한다.

---

## 21. 회귀 조건 6: dynamic form을 “unsupported syntax”로 축소할 때

compiler가 아직 못 다루는 합법 J는:

~~~text
invalid J
~~~

가 아니다.

구분은 반드시 다음과 같아야 한다.

~~~text
J-invalid
    → J semantic error

J-valid, static compiler handles
    → compile

J-valid, runtime-dependent
    → guard / JIT / runtime semantic path

J-valid, implementation coverage missing
    → UnsupportedImplementation
~~~

compiler coverage와 language validity를 섞는 순간 언어가 축소된다.

---

# Part VIII. RustJ가 취해야 할 모델

## 22. “compiler vs interpreter”가 아니라 “semantic ownership vs execution strategy”

RustJ의 가장 중요한 재정의는 다음이다.

잘못된 질문:

> RustJ는 compiler인가 interpreter인가?

더 정확한 질문:

> **누가 J semantics를 소유하며, 각 semantic region을 어떤 execution strategy로 실행할 것인가?**

목표 구조:

~~~text
                  J source
                     ↓
          jsource-compatible frontend
                     ↓
              J Semantic IR
                     ↓
          Semantic Analyzer / facts
                     ↓
         ┌───────────┴───────────┐
         │                       │
statically analyzable       runtime-dependent
         │                       │
Logical IR                  semantic runtime
         │                       │
AOT / JIT / GPU / CPU            │
         │                       │
         └───────────┬───────────┘
                     ↓
             identical J semantics
~~~

여기서 interpreter/runtime path는 architecture center가 아니라:

> **semantic completeness path**

다.

compiler path는:

> **primary optimizing execution path**

다.

두 경로는 서로 다른 언어를 구현해서는 안 된다.

---

## 23. interpreter path가 있어야 compiler architecture가 더 강해지는 이유

runtime semantic path가 존재하면 compiler는 억지로 모든 것을 static하게 만들 필요가 없다.

예:

~~~text
binding이 compile time에 안정적
     → specialize

binding이 version guard로 안정화 가능
     → guarded specialization / JIT

binding이 실제 runtime에만 결정
     → runtime semantic path
~~~

따라서 compiler는 “모든 J를 static하게 알아야 한다”는 잘못된 요구에서 벗어난다.

이 구조가 오히려 J semantics를 보호한다.

---

## 24. JIT는 별개의 언어 모델이 아니다

향후 JIT를 도입해도 frontend나 target option 체계를 새로 만들지 않는다.

~~~text
AOT options ─┐
             ├→ TargetSelector
JIT options ─┘       ↓
              TargetContext
                   ↓
             same lowering
~~~

차이는 정보가 언제 이용 가능한가뿐이다.

AOT:

~~~text
explicit --target / configured target
~~~

JIT:

~~~text
same target constraint
+
runtime device/binding facts
~~~

JIT는 dynamic J와 compiler의 자연스러운 접점이지, J semantics를 별도로 정의하는 두 번째 구현이 아니다.

---

# Part IX. RustJ semantic architecture에 대한 직접적인 결론

## 25. frontend는 jsource를 강하게 따른다

다음은 compiler가 창의성을 발휘할 곳이 아니다.

### Word formation

w.c의 state machine을 compatibility oracle로 사용한다. [S1]

### Enqueue

jtenqueue의 classification order와 primitive/name/noun/assignment semantics를 따른다. [S1]

extension primitive는 parser keyword가 아니라 jtenqueue의 primitive acquisition 지점을 일반화한 PrimitiveResolver에서 넣는다.

### Parser

p.c의 9-row reduction semantics를 따른다. [S2]

Rust implementation은 달라도 된다.

- pointer tagging
- refcount
- low-bit parse mask
- branch prediction
- memory pool

등은 복제할 필요가 없다.

하지만 다음은 바꾸면 안 된다.

- reduction eligibility
- reduction order
- parser-time lookup
- result POS
- modifier construction
- assignment/parenthesis semantics
- original token blame/error behavior

---

## 26. J Semantic IR은 compiler convenience IR이 아니다

J Semantic IR의 역할:

> **J에서 이미 확정된 의미를 가장 손실 없이 소유하는 층**

따라서 FunctionEntity는 최소한 다음을 재귀적으로 보존한다.

~~~text
FunctionEntity
  identity/head
  result POS
  operands
  span/provenance
  intrinsic semantic_info
~~~

그리고 intrinsic semantic info는 node 자체에서 안정적인 정보만 가진다.

- construction facts
- valence/rank contract
- effect/error summary
- binding/self dependency
- atomicity
- latent modifier semantics

실제 argument에 따라 달라지는 것은 Logical call node로 내려간다.

---

## 27. Logical IR에서 비로소 compiler canonicalization을 한다

예:

~~~text
J Semantic IR
  +/
      operand +
~~~

분석 후:

~~~text
Logical IR
  Reduce(Add)
~~~

또:

~~~text
J Semantic IR
  u"r
~~~

분석 후:

~~~text
ResolvedCallFacts
  effective rank
  frame
  cell
  agreement
  repetition

Logical
  CellApply(...)
~~~

이 경계가 RustJ의 핵심이다.

---

# Part X. 대표 예제: (+/ % #) y

## 28. interpreter 관점

source:

~~~text
(+/ % #) y
~~~

parser에서는 먼저 +/가 completed verb가 된다.

그 뒤 fork:

~~~text
Fork
├─ +/
├─ %
└─ #
~~~

로 구성된다.

현재 jsource는 이와 같은 특정 형태를 recognized specialization으로 mean 실행에 연결할 수 있다. [S7]

즉 J interpreter도 이미 semantic structure를 이용한다.

---

## 29. RustJ compiler 관점

RustJ는 parser semantic identity를 먼저 보존한다.

~~~text
Fork
├─ DerivedVerb("/")
│    └─ PrimitiveVerb("+")
├─ PrimitiveVerb("%")
└─ PrimitiveVerb("#")
~~~

Analyzer가 execution semantics를 얻는다.

~~~text
v0 = y
v1 = Tally(v0)
v2 = Reduce(Add, v0)
v3 = Divide(v2, v1)
~~~

그리고 optimizer가 별도의 proof/candidate를 만든다.

~~~text
FusionCandidate::Mean
~~~

중요한 차이:

~~~text
Fork == Mean
~~~

이라고 parser에서 선언하지 않는다.

정확한 순서는:

~~~text
J semantics
 ↓
canonical logical meaning
 ↓
optimization proof
 ↓
target-specific realization
~~~

이다.

이것이 semantic-preserving compilation이다.

---

# Part XI. 현대 하드웨어와 target architecture

## 30. primitive semantics와 hardware capability를 섞지 않는다

+의 의미는 H100과 CPU에서 달라지지 않는다.

따라서:

~~~text
PrimitiveSpec(Add)
  J semantic contract
~~~

와:

~~~text
LoweringCapability(Add, sm90)
LoweringCapability(Add, AVX512)
~~~

를 분리한다.

extension primitive도 똑같이 취급한다.

~~~text
Core(Add) ────────┐
Extension(Conv) ──┼→ same target-locale lowering lookup
Attention ────────┘
~~~

이 구조는 compiler를 도입하면서도 language semantics에 hardware를 역류시키지 않는 장치다.

---

## 31. compile target 선택은 semantics 선택이 아니다

compile invocation 시작 시 active compiler target locale/context를 고른다.

~~~text
CompilationSession
  semantic context
  target context
~~~

하지만:

~~~text
target = sm90
~~~

라고 해서 parser가 다른 +를 만들면 안 된다.

target은 오직:

~~~text
semantic op
+
target context
 ↓
implementation candidates
~~~

에서 사용한다.

AOT/JIT 모두 이 계약을 공유한다.

---

# Part XII. “J 전체를 compile해야 하는가?”에 대한 답

## 32. full semantic coverage와 full optimization coverage는 다르다

RustJ의 언어 목표:

> **J 전체 semantics**

RustJ의 compiler optimization coverage:

> **점진적으로 확대**

두 목표를 동일시하면 구현이 무너진다.

다음 네 영역을 명시적으로 구분한다.

| 영역 | 의미 |
|---|---|
| Static compilable | 현재 정보만으로 의미와 lowering이 결정됨 |
| Guarded compilable | binding/shape/type/version guard 후 compile 가능 |
| JIT compilable | runtime value/binding을 본 후 compile 가능 |
| Runtime semantic | 현재 compiler가 정적으로 없애면 안 되는 dynamic semantics |

이 네 영역은 모두 **합법 J**일 수 있다.

---

## 33. 처음부터 모든 dynamic feature를 compile하려 하지 않는다

대표적으로:

- execute
- highly dynamic locale lookup
- runtime-dependent POS
- value-dependent modifier construction
- heterogeneous rank-result assembly
- arbitrary boxed/dynamic structures

등은 처음부터 AOT 정적 compiler에 억지로 넣지 않는다.

먼저 semantics를 정확히 소유한다.

그 뒤:

~~~text
proof
guard
specialization
JIT
~~~

을 통해 compiler coverage를 넓힌다.

### 33.1 사례 A — name rebinding

J에서는 이름이 같은 종류의 값에 영원히 고정된다고 가정하면 안 된다.

~~~text
f =: +
...
f =: *
~~~

어떤 compiled region이 f를 +로 specialize했다면 다음 중 하나가 필요하다.

~~~text
binding/version proof
or
runtime guard
or
JIT recompilation
or
dynamic call
~~~

잘못된 해결책은 “compiled code에서는 f를 다시 bind할 수 없다”라고 언어를 제한하는 것이다.

RustJ에서는 name/binding/version을 semantic provenance로 보존하고, specialization은 별도의 proof로 둔다.

### 33.2 사례 B — runtime part of speech

J name은 상황에 따라 noun, verb, adverb, conjunction 등 서로 다른 J entity class를 가리킬 수 있다.

따라서 compiler가 name spelling만 보고 다음을 영구 확정하면 안 된다.

~~~text
name "g"
   ↓
always Verb
~~~

올바른 구조는:

~~~text
name use
  ↓
J-compatible parser-time lookup
  ↓
current POS
  ↓
semantic entity / nameref
  ↓
compiler specialization only with stability proof
~~~

이다.

이 원칙은 parser를 일반적인 symbol-resolution AST parser로 단순화하지 말아야 하는 이유와 직접 연결된다.

### 33.3 사례 C — execute

문자열 실행은 AOT compiler가 가장 불편해하는 기능 중 하나다.

하지만 다음 결론은 허용되지 않는다.

~~~text
execute가 dynamic하다
    ↓
J에서 execute를 제거
~~~

대신 단계적으로 생각한다.

~~~text
compile-time constant string
    → parse/analyze/compile 가능

runtime string with stable cache key
    → JIT/cache 가능

arbitrary runtime string
    → shared runtime semantic frontend
~~~

즉 execute는 compiler architecture를 부정하는 기능이 아니라, **AOT/JIT/runtime semantic path의 경계를 시험하는 기능**이다.

### 33.4 사례 D — u"r와 implicit rank execution

u"r에서 parser가 만드는 것은 completed J derived entity다.

actual argument가 들어오기 전에는 다음을 모두 알 수 없다.

- effective cell rank
- frame extent
- repetition
- result assembly details

따라서:

~~~text
FunctionEntity(u"r)
  owns requested-rank construction semantics

Call node
  owns actual effective-rank/frame/cell facts
~~~

로 나눈다.

compiler는 call facts가 충분할 때 fixed parallel map/fused kernel로 낮출 수 있고, 그렇지 않으면 generic CellApply semantics를 유지한다.

### 33.5 사례 E — hook/fork와 speculative parallelism

fork의 두 branch가 계산상 독립적으로 보인다고 해서 자동으로 병렬 실행하면 안 된다.

다음이 관찰 가능할 수 있기 때문이다.

- name lookup
- assignment/effect
- error order
- dynamic execution

따라서 branch parallelization은:

~~~text
semantic fork
   ↓
effect/error analysis
   ↓
purity/speculation proof
   ↓
parallel candidate
~~~

순서로만 가능하다.

### 33.6 사례 F — heterogeneous rank result assembly

cell application 결과가 항상 동일 dtype/shape이라고 가정하면 GPU map으로 쉽게 낮출 수 있다.

하지만 J semantics가 heterogeneous result의 type/shape join, fill, boxing 또는 assembly error를 허용/요구하는 경우에는 이 가정이 잘못될 수 있다.

따라서 fixed-shape parallel output은:

~~~text
UniformCellResultProof
~~~

가 있을 때만 허용한다.

이 사례는 “compiler가 처리하기 어려운 J semantics를 없애지 말고, proof가 있는 경우에만 더 좁은 physical form으로 specialization한다”는 전체 원칙의 대표 사례다.

---

# Part XIII. 역사에서 얻는 더 깊은 교훈

## 34. APL의 interpreter는 “실패한 compiler의 대안”이 아니었다

초기 APL interpreter는 다음을 가능하게 했다.

- 언어 experimentation
- machine compromise 최소화
- immediate interaction
- compact workspace
- declaration-free values
- 큰 array primitive에 의한 효율적인 interpretation

[H1][H3][H4]

즉 interpreter는 당시 언어 철학과 hardware/environment에 맞는 **positive design choice**였다.

RustJ는 이를 존중해야 한다.

---

## 35. 하지만 그 역사에서 “영원히 interpreter여야 한다”는 결론은 나오지 않는다

### 35.1 역사 자료가 증명하지 않는 것

역사 자료를 설계 교리로 과도하게 읽으면 안 된다.

[H1]이 보여 주는 것은:

- interpreter가 당시 합리적이었다.
- array granularity가 interpreter overhead를 amortize할 수 있었다.
- language design을 machine convenience에 종속시키고 싶지 않았다.

하지만 이것만으로 다음은 증명되지 않는다.

- 모든 J workload에서 interpreter가 compiler보다 빠르다.
- GPU에서도 primitive-at-a-time execution이 최적이다.
- whole-graph optimization의 가치가 없다.
- dynamic semantics와 compilation이 양립할 수 없다.
- JIT가 J 철학에 어긋난다.
- 미래에도 implementation strategy가 같아야 한다.

반대로 compiler 연구가 존재했다는 사실도 “compiler가 언제나 interpreter보다 우월하다”는 뜻은 아니다.

RustJ는 역사적 권위를 어느 한 execution strategy의 영구적 승리 선언으로 사용하지 않는다.

사용해야 할 역사적 원칙은 더 추상적이다.

> **언어의 의미와 표현력을 implementation convenience보다 우선한다.**

환경은 달라졌다.

과거 비용의 중심:

~~~text
primitive computation
~~~

현대 heterogeneous accelerator에서 추가된 중요 비용:

~~~text
transfer
placement
global-memory traffic
intermediate materialization
kernel scheduling
layout
synchronization
cross-op fusion
~~~

[G1]

그리고 APL compiler 연구가 실제로 오래전부터 존재했다는 사실도 “array semantics와 compilation은 모순”이라는 주장을 반박한다. [H7][C1][C2]

따라서 역사에서 취해야 할 교훈은:

> interpreter를 유지하라

가 아니다.

더 정확하게는:

> **machine 때문에 language semantics를 훼손하지 마라.**

이다.

---

# Part XIV. RustJ의 설계 헌장

## 36. 반드시 지켜야 할 12개 원칙

### 원칙 1 — J semantics가 최상위다

compiler convenience보다 J semantics가 우선한다.

### 원칙 2 — frontend는 jsource-compatible하다

word formation → enqueue → parser는 jsource를 구현 명세 수준으로 따른다.

### 원칙 3 — compiler coverage와 language validity를 분리한다

compiler가 못한다고 J-invalid가 되는 것은 아니다.

### 원칙 4 — Semantic IR에서 J identity를 보존한다

modifier, hook, fork, name, POS, rank construction을 너무 일찍 canonical op로 지우지 않는다.

### 원칙 5 — call-dependent information은 call node가 가진다

actual shape/rank/frame/cell/agreement는 FunctionEntity intrinsic identity가 아니다.

### 원칙 6 — optimization은 proof다

Mean, fusion, materialization elimination 등은 semantic identity가 아니라 proof/candidate다.

### 원칙 7 — target information은 downstream이다

GPU architecture/device/layout/tile/workgroup은 J semantics가 아니다.

### 원칙 8 — dynamic semantics에는 escape hatch가 있어야 한다

guard/JIT/runtime semantic path를 허용한다.

### 원칙 9 — interpreter와 compiler가 서로 다른 J를 구현하면 안 된다

같은 frontend, 같은 semantic object, 같은 error/effect rules를 공유한다.

### 원칙 10 — error/effect order를 performance 때문에 바꾸지 않는다

reordering은 proof가 있을 때만 한다.

### 원칙 11 — built-in과 extension은 lowering에서 동등하다

core J primitive도 extension과 동일한 target/capability architecture를 사용한다.

### 원칙 12 — differential conformance가 최종 심판이다

설계 설명보다 jsource와의 observable behavior comparison이 우선한다.

---

# Part XV. Architecture review: 이 질문에 답하지 못하면 변경하지 않는다

## 37. frontend 변경 전

- jsource의 어느 source/function이 oracle인가?
- word/class/POS/reduction timing이 바뀌지 않는가?
- parser-time lookup 순서가 동일한가?
- modifier 결과 POS가 동일한가?
- assignment/parenthesis/error behavior가 동일한가?
- differential test가 있는가?

---

## 38. Semantic IR 변경 전

- 이 정보는 J entity 자체에 불변인가?
- child node를 재귀적으로 분석하기 위해 node가 직접 가져야 하는가?
- 실제 argument가 있어야 결정되는 정보는 아닌가?
- optimization proof를 semantic fact로 잘못 넣는 것은 아닌가?
- target/device 정보가 역류한 것은 아닌가?
- 원래 J modifier/train identity를 잃는가?

---

## 39. Logical IR 변경 전

- semantic lowering이 reversible/explainable한가?
- J의 rank/cell/frame/agreement를 충분히 보존하는가?
- error/effect ordering dependency가 표현되는가?
- unknown을 semantic error로 바꾸고 있지 않은가?
- fallback/JIT path를 막지 않는가?

---

## 40. optimizer 변경 전

- transformation이 J-visible result를 보존하는가?
- error order가 바뀌지 않는가?
- assignment/name/effect가 재배치되지 않는가?
- empty frame/fill/assembly semantics가 같은가?
- proof invalidation 조건이 명시되어 있는가?
- target-independent proof와 target-specific profitability가 구분되어 있는가?

---

## 41. GPU/backend 변경 전

- primitive semantics에 hardware metadata를 넣고 있지 않은가?
- same semantic op가 다른 backend에서 동일한 J result/error를 내는가?
- transfer/materialization/sync가 physical decision으로 남아 있는가?
- native/external route 경계에서 semantics를 잃지 않는가?
- compiler가 못 다루는 경우 J 자체를 제한하지 않고 명시적 fallback/unsupported를 제공하는가?

---

# Part XVI. 이 문서가 허용하지 않는 설계 주장

다음 문장은 근거 없이 사용하면 안 된다.

> “J는 dynamic해서 compile할 수 없다.”

틀린 일반화다. APL compiler 연구와 현대 specialization/JIT techniques가 존재한다.

> “compiler이므로 parser는 일반 AST parser로 바꾸자.”

J parser semantics를 잃을 가능성이 높다.

> “GPU를 위해 rank를 broadcasting으로 단순화하자.”

J semantics가 달라진다.

> “성능을 위해 hook/fork branch를 항상 parallelize하자.”

observable error/effect order를 깨뜨릴 수 있다.

> “static shape가 아니면 unsupported J다.”

compiler coverage와 language validity를 혼동한다.

> “interpreter fallback이 있으면 compiler architecture가 아니다.”

틀리다. semantic runtime은 completeness mechanism이고 compiler는 primary optimizing execution strategy일 수 있다.

> “J가 interpreter였으니 compiler는 J답지 않다.”

역사적 근거가 없다. 초기 설계자 자신이 compilation difficulty보다 efficient array interpretation을 이유로 들었다. [H1]

---

# Part XVII. 최종 판단

## 42. RustJ compiler는 회귀인가?

다음 구조라면 **회귀다**.

~~~text
J source
 ↓
compiler가 처리하기 쉬운 subset으로 언어 축소
 ↓
static types/shapes/bindings 강제
 ↓
J semantics 일부 상실
 ↓
fast machine code
~~~

다음 구조라면 **회귀가 아니다**.

~~~text
J source
 ↓
jsource-compatible frontend
 ↓
complete J semantic representation
 ↓
semantic analysis
 ↓
statically provable region ──→ compiler
dynamic region ──────────────→ JIT/runtime semantic execution
 ↓
identical observable J semantics
~~~

그리고 RustJ는 두 번째 구조를 목표로 한다.

---

## 43. RustJ가 역사적으로 자연스러운 연장선인 이유

APL/J의 interpreter 전통에서 가장 중요한 아이디어는:

> **언어를 machine에 맞춰 왜곡하지 않는다.**

RustJ가 이 원칙을 지키면서:

- J semantic objects를 그대로 보존하고
- compiler facts를 별도 층으로 관리하며
- whole-graph dataflow를 분석하고
- GPU/CPU target을 downstream에서 선택하고
- dynamic 영역에는 runtime/JIT path를 남긴다면

RustJ compiler는 APL/J 철학의 부정이 아니다.

오히려 다음과 같은 확장이다.

~~~text
APL/J traditional implementation
  high-level array semantics
        ↓
  optimized primitive execution
        ↓
  local derived specialization

RustJ
  same high-level J semantics
        ↓
  explicit semantic graph
        ↓
  cross-primitive logical analysis
        ↓
  global/region specialization
        ↓
  CPU / GPU / library / JIT
~~~

즉 RustJ의 compiler는 **interpreter를 이기기 위한 compiler**가 아니라,

> **J가 원래 primitive 내부에서 얻던 semantic leverage를 primitive 사이, region 전체, heterogeneous hardware까지 확장하기 위한 compiler**

여야 한다.

이 문장이 향후 compiler 설계의 가장 중요한 기준이다.

---

# Part XVIII. Source guide

## 44. 역사적 1차/주요 자료

### [H1] Falkoff & Iverson — The Evolution of APL

https://www.jsoftware.com/papers/APLEvol1.htm

이 문서에서 확인할 핵심:

- 1965년 interpreter 선택의 배경
- experimental implementation
- machine compromise 최소화
- declaration을 피한 설계
- array nature가 efficient interpretation을 가능하게 한다는 회고
- compilation이 본질적으로 불가능해서 interpreter가 된 것이 아니라는 점

이 문서는 **“왜 interpreter였는가?”에 대한 최우선 역사 자료**로 취급한다.

### [H2] Falkoff & Iverson — The Design of APL

https://www.jsoftware.com/papers/APLDesign1.htm

핵심:

- simplicity / generality / experimentation이라는 설계 원칙
- implementation convenience가 language semantics를 지배하지 않게 하려는 태도
- workspace와 system facilities의 설계 배경

### [H3] Falkoff & Iverson — The APL\360 Terminal System

https://www.jsoftware.com/papers/APL360TerminalSystem1.htm

핵심:

- completely interpretive execution
- source-like function storage
- 약 36KB workspace
- array operation 때문에 interpretation overhead가 작을 수 있다는 실제 설명
- immediate terminal execution과 workspace model

**resource scarcity에서도 interpreter가 실용적이었던 이유를 검토할 때 반드시 본다.**

### [H4] Eugene McDonnell — The Socio-Technical Beginnings of APL

https://www.jsoftware.com/papers/eem/socio1.htm

핵심:

- 초기 APL time-sharing architecture
- workspace가 scheduling/memory-management unit으로 사용된 방식
- 매우 제한된 hardware resource에서 practical service를 만든 배경

### [H5] Roger Hui — An Implementation of J / Incunabulum

https://www.jsoftware.com/ioj/ioj.htm  
https://www.jsoftware.com/ioj/iojATW.htm

핵심:

- Arthur Whitney의 one-page interpreter
- Roger Hui가 이를 연구한 뒤 J 구현을 시작한 역사
- J implementation의 semantic object / array interpreter 계보

### [H6] Roger Hui — Remembering Ken Iverson

https://www.jsoftware.com/papers/remembering.htm

핵심:

- Dictionary parse table을 중심으로 J parser를 설계한 이유
- C implementation과 language specification을 의도적으로 가깝게 만든 배경
- word formation과 parsing을 interpreter의 핵심으로 본 관점

### [H7] Jsoftware — APL Quotations and Anecdotes

https://www.jsoftware.com/papers/APLQA.htm

핵심:

- Bob Bernecky가 1990년 이후 APL compiler 작업에 집중했다는 역사적 기록
- “APL/J는 compiler 연구 대상이 아니었다”는 잘못된 인상을 교정하는 보조 자료

### [H8] Vector Vol. 8 No. 2 — J/APL discussion

https://www.jsoftware.com/papers/Vector_8_2_BarmanCamacho.pdf

Bob Bernecky의 당시 의견으로 J의 규칙성과 compilability가 언급된다. 이는 설계자 전체의 공식 입장으로 취급하지 않고 **역사적 관찰**로만 사용한다.

---

## 45. APL compiler 자료

### [C1] Robert Bernecky — Compiling APL

Springer, *Arrays, Functional Languages, and Parallel Systems*

https://link.springer.com/book/10.1007/978-1-4615-4002-1

APL compilation이 실제 연구 주제였음을 보여 주는 직접 자료.

### [C2] Robert Bernecky — APEX compiler

1997 technical report announcement:

https://groups.google.com/g/comp.lang.apl/c/vp1f5s-_KqE

APEX는 APL compilation, compiler IR/dataflow, generated-code performance 연구의 중요한 사례다.

### [C3] A Strategy for Compiling APL

https://www.jsoftware.com/papers/Vector_20_4_Smith.pdf

APL의 dynamic representation을 그대로 object wrapper로 옮기는 단순 compilation이 왜 느릴 수 있는지, 더 구체적인 representation/type strategy가 왜 필요한지를 보여 주는 실무적 사례다.

---

## 46. RustJ가 기준으로 삼는 jsource source

Parser/frontend semantic compatibility 기준 commit:

**13994ffa1ed5f06f79fad6e9822a7ed2d29b1528**

### [S1] jsrc/w.c — word formation + enqueue

https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/w.c

검토 함수:

- jtwordil
- jtwords
- jtenqueue

설계 판단:

- lexer state-machine compatibility
- primitive acquisition 위치
- name/numeric/string/assignment classification
- enqueue error provenance

### [S2] jsrc/p.c — parser

https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c

검토 대상:

- cases[]
- runtime ptcol/mask dispatch
- parser-time name lookup
- original token blame
- rows 0–8 reduction behavior
- reduction result reinsertion

RustJ frontend/parser 변경의 **최우선 source oracle**.

### [S3] jsrc/cf.c — hook/fork construction and specialization

https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c

검토 대상:

- jthook
- jtfolk
- bident/trident behavior
- +/%# mean specialization 등

### [S4] jsrc/ar.c — slash / reduction-derived verb

https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ar.c

검토 대상:

- jtslash
- completed derived verb construction
- reduction implementation selection

### [S5] jsrc/cr.c — rank conjunction and implicit rank execution

https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c

검토 대상:

- jtqq
- jtrank1ex
- jtrank2ex
- fill-cell behavior
- result assembly
- IRS/rank specialization

### [S6] jsrc/t.c / jsrc/jtype.h — primitive metadata

https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/t.c  
https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/jtype.h

검토 대상:

- primitive valence functions
- mr/lr/rr
- VIRS1/VIRS2
- VISATOMIC
- VFUSEDOK
- name/self/effect-related execution flags

주의: 이 flag를 RustJ Semantic IR에 그대로 복사하지 않는다. **semantic contract / optimizer proof / lowering capability / physical decision으로 재분류**한다.

### [S7] current jsource specialization patterns

대표 파일:

https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c  
https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ca.c  
https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ar.c

의미:

- 현재 J도 composition/rank/atomicity를 이용해 specialized execution path를 고른다.
- RustJ compiler는 이 철학을 더 넓은 graph/target 수준으로 확장하되 semantic identity와 optimization을 분리한다.

---

## 47. 현대 GPU cost model 참고

### [G1] NVIDIA CUDA C++ Best Practices Guide

https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html

관련 항목:

- data transfer between host and device
- memory optimization
- coalescing
- execution configuration

RustJ에서 GPU compiler의 필요성을 평가할 때 “arithmetic throughput”만 보지 말고 **transfer + materialization + memory traffic + execution mapping** 전체를 보아야 한다는 근거로 사용한다.

---

# Part XIX. 문서 유지 규칙

## 48. 이 문서를 언제 갱신해야 하는가

다음 중 하나라도 바뀌면 FOUNDATIONS.ko.md를 검토한다.

1. RustJ가 full J semantics 목표를 포기하거나 수정할 때
2. interpreter/runtime fallback의 역할이 바뀔 때
3. JIT가 별도 language path를 갖게 될 때
4. frontend가 jsource와 의도적으로 달라질 때
5. compiler가 static declarations를 J source에 요구하려 할 때
6. Semantic IR에서 J identity 보존 정책이 바뀔 때
7. rank/CellApply 의미 모델이 바뀔 때
8. target-specific information이 semantic layer로 이동할 때
9. GPU/CPU route 정책이 language validity에 영향을 주기 시작할 때
10. 새로운 역사적/기술적 증거가 이 문서의 전제를 뒤집을 때

---

## 49. 설계 review에서 이 문서를 사용하는 방법

아키텍처 변경 PR/작업에서는 필요한 경우 다음 형식으로 판단을 남긴다.

~~~text
FOUNDATIONS review

J semantics preserved?
  yes/no + reason

Frontend compatibility affected?
  yes/no + jsource oracle

Dynamic behavior restricted?
  yes/no

Compiler coverage confused with language validity?
  yes/no

Semantic identity erased early?
  yes/no

Error/effect ordering affected?
  yes/no + proof

Target facts leaking upstream?
  yes/no

Runtime/JIT fallback preserved?
  yes/no

Differential test:
  ...
~~~

모든 작은 구현 변경에 이 템플릿을 붙일 필요는 없다.

하지만 **언어 의미나 compiler/interpreter 경계를 바꾸는 결정이라면 반드시 이 질문들을 검토한다.**

---

# 50. 한 문장 원칙

> **RustJ는 J를 compiler에 맞게 바꾸지 않는다. J를 먼저 정확히 보존하고, 그 의미를 이용해 interpreter가 primitive 내부에서 얻던 최적화 범위를 whole graph와 현대 CPU/GPU까지 확장한다.**

이 원칙이 깨진다면 compiler architecture는 발전이 아니라 회귀다.


---

# Part XX. 배열 언어·IR compiler 연구에서 얻은 설계 규칙

이 절은 기존 APEX / Co-dfns / TAIL-Futhark 검토에 2026-10-04의 비교 검토를 추가해 RustJ middle-end 원칙을 보강한다.

주요 비교 계열은 다음과 같다.

1. Robert Bernecky의 **APEX: The APL Parallel Executor** source/research lineage.
2. Aaron W. Hsu의 **The Key to a Data Parallel Compiler**와 현재 Co-dfns source.
3. Elsman/Henriksen 외의 **APL → TAIL → Futhark** GPU compilation 연구 및 Dyalog'16 발표.
4. **Remora** — J/APL의 rank-polymorphic 계산 모델을 분리해 frame/cell/implicit lifting을 정형화한 연구 언어.
5. **Bohrium** — 기존 NumPy-style 프로그램의 array operation을 지연된 IR로 수집해 fusion/materialization과 CPU/GPU 등 실행 방식을 뒤에서 선택한 계열.
6. **Lift** — map/reduce 같은 high-level functional array pattern을 rewrite하고 hardware mapping을 별도 단계로 탐색하는 연구.
7. **MLIR Linalg** — structured operation, indexing/iterator semantics와 implicit iteration을 보존하고 tiling/vectorization/lowering에서 explicit loop를 materialize하는 IR 계열.

이 자료들은 모두 array-language/compiler 문제를 다루지만 서로 다른 질문에 답한다.

~~~text
APEX
  → 어떤 array facts를 추론해야 하는가?
  → SSA/interprocedural analysis/specialization을 어떻게 쓰는가?

Co-dfns / Hsu
  → compiler graph 자체를 어떻게 compact columnar form으로 보고
    batch/data-parallel pass로 처리할 수 있는가?

TAIL / Futhark
  → 분석된 array semantics를 어떤 high-level parallel IR로 보존해야
    fusion/flattening/GPU lowering이 가능한가?

Remora
  → rank polymorphism과 frame/cell implicit lifting을
    독립적인 semantic model로 어떻게 정형화하는가?

Bohrium
  → 기존 array API의 계산을 어떻게 지연 수집하여
    fusion/materialization/heterogeneous execution을 뒤에서 결정하는가?

Lift
  → high-level array rewrite와 hardware mapping을 어떻게 분리하는가?

MLIR Linalg
  → structured computation과 implicit iteration을 얼마나 오래 보존하고
    언제 loop/tiling/vector lowering으로 materialize하는가?
~~~

이 비교에서 RustJ에 추가로 확인되는 상위 원칙은 다음과 같다.

- **J source는 execution plan이 아니다.**
- rank, derived entity, train/composition, reduce/scan, reindex/shape transform은 optimizer가 사용할 수 있는 고수준 정보다.
- 이런 구조를 scalar loop나 backend kernel로 조기 분해하지 않는다.
- full J semantics와 특정 optimized route의 eligibility를 분리한다.
- logical rewrite와 physical schedule/device/materialization 선택을 분리한다.
- 기존 J를 frontend로 유지하는 것이 목표이며, compiler-friendly subset을 만들기 위해 J semantics를 축소하지 않는다.

RustJ는 이 연구들의 제한된 language subset이나 static assumption을 그대로 채택하지 않는다. **J semantic completeness는 그대로 유지하고, 각 compiler route가 요구하는 정적 조건은 route precondition 또는 specialization guard로 취급한다.**

직접 비교 근거:

- Remora / *The Semantics of Rank Polymorphism*: https://arxiv.org/abs/1907.00509
- Bohrium publication index (NumPy CPU/GPU/cluster, vector VM, fusion lineage): https://bohrium.readthedocs.io/publications.html
- Lift / *A Functional Data-Parallel IR for High-Performance GPU Code Generation*: https://doi.org/10.1109/CGO.2017.7863730
- MLIR Linalg structured-operation primer / implicit-loop materialization: https://mlir.llvm.org/docs/Tutorials/transform/Ch0/

이 출처들은 RustJ semantic specification이 아니라 위 compiler 원칙을 검증·비교하기 위한 자료다.

## 51. APEX: Array Morphology를 정식 abstract interpretation으로 본다

APEX 연구의 가장 중요한 교훈은 array compiler가 단순 dtype inference를 넘어 **type, rank, shape, element count, constant/value knowledge, array property**를 data-flow property로 추적해야 한다는 점이다. APEX는 SSA와 interprocedural/semi-global analysis를 사용해 같은 source name의 서로 다른 값들을 분리하고, array morphology 정보를 반복 전파했다. [APEX-1][APEX-2]

RustJ는 기존 `ValueFacts`/`ResolvedCallFacts`를 다음처럼 정식 abstract domain으로 발전시킨다.

~~~text
ValueFacts
  TypeFact
  RankFact
  ShapeFact
  ItemCountFact
  ConstantFact
  ArrayPropertyFacts
  ConstraintSet
  provenance / witness
~~~

각 fact는 단순 optional field가 아니라 최소한 다음 연산을 갖는 lattice/abstract-domain 구성요소로 본다.

~~~text
bottom / unknown
join / merge
transfer
refinement
widening 또는 specialization merge가 필요한 경우의 정책
~~~

분석기는 local one-shot inference가 아니라 worklist/fixpoint 구조를 가질 수 있어야 한다.

~~~text
seed facts
   ↓
node transfer
   ↓
join
   ↓
worklist
   ↓
interprocedural summary/specialization propagation
   ↓
fixed point
~~~

**RustJ 결정:** morphology engine은 J Semantic IR 자체가 아니라 Semantic Analyzer 이후 Logical analysis substrate다. parser/FunctionEntity에 actual shape/type facts를 역류시키지 않는다.

## 52. ArrayPropertyFacts를 first-class analysis domain으로 둔다

APEX의 morphology 연구는 shape/type 외에도 array predicates가 algorithm selection에 중요하다는 점을 보여 준다. RustJ는 다음과 같은 property를 extensible fact domain으로 둔다.

~~~text
ArrayPropertyFacts
  IntegralValued
  NonNegative
  Unique
  SortedAscending / SortedDescending
  Permutation
  KnownRange
  AllEqual
  ...
~~~

예를 들어 `f64 [1.0, 2.0, 3.0]`은 storage dtype은 float이지만 `IntegralValued=Proven`일 수 있다. 이 distinction은 index legality, shape argument validation, bounds-check elimination, algorithm specialization에 유용하다.

**금지:** property proof가 없는데 dtype만 보고 property를 추측하지 않는다.

## 53. FactWitness: fact와 “왜 참인지”를 함께 보존한다

APEX 연구는 shape 정보의 **origin/provenance**를 추적하면 더 많은 runtime conformability check를 제거할 수 있다는 방향을 제시했다. RustJ는 이를 명시적 구조로 채택한다. [APEX-2]

~~~text
Fact<T>
  abstract_value
  witness
~~~

예:

~~~text
ShapeFact = [N,M]
Witness =
  Constant
  SameAs(ValueId, axis)
  ReshapeConstraint
  CellApplyFrame
  BindingVersion
  RuntimeGuard(GuardId)
  DerivedFrom(NodeId, RuleId)
~~~

이를 통해 optimizer는 “shape가 같다”뿐 아니라 “왜 같다고 믿어도 되는가”를 확인할 수 있다.

**RustJ 결정:** check elimination/fusion/specialization은 witness 없는 optimistic fact를 사용하지 않는다.

## 54. SSA는 J namespace를 대체하지 않고 post-semantic dataflow를 표현한다

APEX에서 SSA는 같은 이름이 서로 다른 morphology를 가질 때 분석을 정밀하게 하고 liveness를 명확하게 만드는 데 유용했다. [APEX-2]

RustJ에서도 SSA를 사용하지만 경계는 다음과 같다.

~~~text
J Name
  ≠ BindingVersion
  ≠ SSA ValueId
~~~

~~~text
jsource-compatible frontend
  ↓
J Semantic IR / binding semantics
  ↓
binding/effect resolution
  ↓
Logical SSA
~~~

dynamic locale/name semantics가 남는 경우에는 억지로 SSA local value로 치환하지 않고:

~~~text
NameRead
NameWrite
LocaleResource
EffectToken
~~~

으로 표현한다.

**거부:** APEX-style compiler restriction을 이유로 dynamic name/POS를 J에서 금지하지 않는다.

## 55. Call-site specialization은 source clone이 아니라 SpecializationKey/cache로 구현한다

APEX는 동일 function이 서로 다른 call morphology를 받을 때 call-site별 specialized body를 만드는 접근을 사용한다. RustJ는 이를 source/function clone보다는 cacheable specialization으로 일반화한다. [APEX-2]

~~~text
SpecializationKey
  FunctionEntityId
  binding version / guard
  valence
  relevant dtype classes
  effective rank facts
  relevant shape class
  relevant ArrayPropertyFacts
  fit/tolerance/rank policy
~~~

동일 key는 analyzed region을 재사용한다.

exact shape/constant를 항상 key에 넣으면 specialization explosion이 생기므로, fact마다 **specialization relevance**를 둔다.

~~~text
dtype      usually relevant
rank       usually relevant
exact shape  conditional
constant     conditional
Sorted/Unique algorithm-dependent
~~~

JIT는 같은 cache protocol을 사용하고 runtime facts를 추가 evidence로 제공한다.

## 56. Co-dfns: semantic DAG와 별도로 compact GraphIndex/SoA analysis view를 둔다

Hsu의 *The Key to a Data Parallel Compiler*는 AST node 관계를 **Node Coordinate Matrix**로 encoding하고 Key/grouping 연산과 결합해 subtree computation을 data-parallel하게 수행할 수 있음을 제시한다. [CODFNS-1]

현재 Co-dfns parser도 AST를 object-per-node tree가 아니라 다음과 같은 **inverted/columnar table**로 반환한다. [CODFNS-2]

~~~text
parent
depth
type
kind
name
lex
varbind
source start
source end
~~~

RustJ는 이 아이디어를 semantic representation 자체로 강제하지 않는다. canonical layer는 immutable shared `FunctionEntity`/Logical DAG로 유지한다.

대신 pass용 sidecar를 둔다.

~~~text
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
  source origin index
~~~

이 view는 다음을 batch 처리하기 쉽게 한다.

- 같은 op/entity/scope/specialization key별 grouping
- use-def/liveness
- bottom-up/top-down summary
- dependency-level parallel analysis
- compiler 자체의 SIMD/멀티코어 batch pass

**거부:** 모든 compiler pass를 NCM/Key만으로 작성하거나 semantic DAG를 하나의 dense matrix representation으로 고정하지 않는다. GraphIndex는 derived analysis view다.

## 57. Co-dfns의 nanopass 원칙은 채택하되 pass마다 IR clone을 강제하지 않는다

Co-dfns 관련 논문은 compiler를 작은 fully data-parallel pass들의 composition으로 구성하는 nanopass 성격을 설명한다. [CODFNS-1][CODFNS-3]

RustJ의 Semantic Analyzer/middle-end도 가능한 한 작은 책임으로 나눈다.

~~~text
ResolveBindings
InferValueFacts
ResolveRank
PlanCellApplication
InferShape
InferArrayProperties
ResolveEffects
BuildLogicalOps
ProveUniformAssembly
DetectFusion
RouteAnalysis
~~~

다만 각 pass가 whole IR를 복사해야 한다는 뜻은 아니다. immutable semantic identity + node/value fact tables + invalidation-aware derived caches를 사용한다.

## 58. Co-dfns의 performance model을 CostProfile에 반영한다

Co-dfns의 performance guide는 GPU 비용을 단순 FLOP 수가 아니라 **critical path, kernel count, memory traffic, host↔device transfer, fusion 가능성**으로 설명한다. 또한 GPU와 interpreter에서 같은 primitive라도 성능/복잡도 특성이 달라질 수 있음을 명시한다. [CODFNS-4]

따라서 RustJ cost/resource model은 최소한 다음을 다룬다.

~~~text
critical_path_depth
kernel_launch_count
bytes_read
bytes_written
temporary_bytes
transfer_bytes
synchronization_points
parallelism/occupancy opportunity
arithmetic work
~~~

이 정보는 semantic contract가 아니라 Schedule/ResourceEstimate/CostEstimate에서 계산한다.

## 59. TAIL: typed array IR의 역할은 J Semantic IR이 아니라 specialized Logical IR에 있다

Dyalog'16 자료와 FHPC'16 논문은 dynamically typed/rank-polymorphic APL을 typed array IL인 TAIL로 내린 뒤 Futhark로 변환한다. TAIL type system은 base type뿐 아니라 rank/shape type 및 polymorphic call의 explicit type/rank instantiation을 표현한다. [TAIL-1][TAIL-2]

RustJ에서 이에 대응하는 층은:

~~~text
J Semantic IR
  // J identity/dynamic semantics 보존
       ↓
Semantic Analyzer
       ↓
Specialized Logical IR
  resolved dtype/rank
  shape constraints
  CellApply plan
  specialization witness
~~~

이다.

**거부:** TAIL의 static assumptions을 parser/J Semantic IR의 언어 제약으로 올리지 않는다.

## 60. high-level parallel structure를 backend 직전까지 보존한다

FHPC'16은 APL의 high-level array combinator가 원래의 parallel intent를 보존하며, Futhark로 lowering할 때 map nests와 reduction nests를 explicit하게 만들면 fusion, nested-parallelism flattening, coalesced-memory optimization을 활용할 수 있음을 보여 준다. [TAIL-2]

RustJ Logical IR은 다음 구조를 scalar loop로 너무 빨리 분해하지 않는다.

~~~text
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
Loop/Power
~~~

특히:

~~~text
CellApply(Reduce(...))
Reduce(CellApply(...))
nested CellApply
~~~

같은 nested parallel structure를 보존한다.

flattening, segmented lowering, thread/block mapping은 semantics가 아니라 optimizer/schedule/physical decision이다.

## 61. external backend에는 scalar primitive soup가 아니라 fusion-friendly structure를 넘긴다

Futhark가 map/reduce/scan 같은 SOAC 구조를 이용해 fusion/parallel lowering을 수행한다는 점에서, RustJ external adapter도 가능한 한 고수준 operation을 유지해야 한다. [TAIL-1][TAIL-2]

따라서 lowering capability는 단순히:

~~~text
supports Add
~~~

만 표현해서는 부족하다.

장기적으로 다음과 같이 표현할 수 있어야 한다.

~~~text
supports Map(Add)
supports Reduce(Add, axis pattern)
supports nested Map/Reduce
supports segmented reduction
supports view/reindex form
supports shape-polymorphic map under constraints
~~~

backend가 이 구조를 표현하지 못하면 route adapter가 decomposition 여부와 cost를 판단한다. backend limitation 때문에 J Semantic IR을 단순화하지 않는다.

## 62. parameterized lowering recipe를 정식 개념으로 둔다

TAIL→Futhark translation에서 일부 APL primitive는 statically known type/rank 정보를 이용해 specialized code skeleton으로 생성된다. Dyalog'16에서도 constant folding, `take` 내부 branch 제거 같은 예가 제시된다. [TAIL-1][TAIL-2]

RustJ는:

~~~text
Semantic Op
+
ResolvedCallFacts
+
Target Capability
       ↓
Parameterized Lowering Recipe
       ↓
candidate realization(s)
~~~

을 허용한다.

예를 들어 `Take`는:

~~~text
sign known?
bounds known?
fill needed?
view legal?
rank/layout?
~~~

에 따라:

~~~text
ViewTake
DirectSlice
PadAndSlice
GenericTakeKernel
RuntimeSemanticFallback
~~~

중 하나가 될 수 있다.

## 63. full J program에서 pure array region을 추출한다

Futhark는 pure target이므로 APL 연구는 I/O/effects를 parameter/result boundary로 옮기거나 지원 범위를 제한했다. [TAIL-2]

RustJ는 full J를 제한하는 대신 이 원리를 **region partition**에 사용한다.

~~~text
Full J semantic graph
      ↓
EffectAnalysis
      ↓
RegionPartition
  PureArrayRegion
  GuardedDynamicRegion
  StatefulRegion
  RuntimeSemanticRegion
~~~

external GPU route는 PureArrayRegion 또는 capability가 증명된 region만 받는다.

**핵심:** backend subset은 RustJ language subset이 아니다.

## 64. backend-specific subset restriction을 J language restriction으로 승격하지 않는다

Dyalog'16 발표는 해당 compiler의 제한으로 static scoping/static rank inference, limited nested arrays, whole-program compilation, no execute를 명시한다. [TAIL-1]

Co-dfns manual 역시 현재 compiled subset에 execute, 일부 namespace/free-reference, train 등의 제한이 있음을 명시한다. [CODFNS-2]

RustJ는 이런 제한을 그대로 채택하지 않는다.

~~~text
route cannot lower X
  ≠ X is invalid J
~~~

대신:

~~~text
route precondition fails
  → another native/external route
  → guarded JIT
  → runtime semantic path
  → UnsupportedImplementation only if RustJ coverage itself is missing
~~~

로 처리한다.

## 65. APEX/Co-dfns에서 의도적으로 거부하는 semantic compromise

다음은 compiler performance를 이유로 RustJ에 가져오지 않는다.

- dynamic name/POS를 금지해 분석을 단순화
- type/rank declaration 없이는 합법 J를 실행하지 못하게 함
- backend representation 때문에 J overflow/promotion/empty semantics 변경
- runtime conformability/error check를 제거해 fusion을 허용
- pure external backend가 effect를 지원하지 않는다는 이유로 J effect 자체를 금지

올바른 해법은 각각:

~~~text
binding/version proof or guard/JIT/runtime
optional annotation, never mandatory semantics
semantic-preserving retry/promotion/fallback
proof-based check elimination or check hoisting
effect-region partition / runtime semantic route
~~~

이다.

## 66. 세 연구를 반영한 RustJ middle-end 기준 구조

~~~text
J Semantic IR
  immutable FunctionEntity/J values
  J semantics authoritative
        ↓

Binding / Effect Analysis
        ↓

Logical SSA
  ValueId
  StateResource
  LocaleResource
  EffectToken
        ↓

GraphIndex / AnalysisIndex
  compact SoA view for batch passes
        ↓

MorphologyEngine
  TypeFact
  RankFact
  ShapeFact
  ItemCountFact
  ConstantFact
  ArrayPropertyFacts
  ConstraintSet
  FactWitness
        ↓
interprocedural fixed point
        ↓

SpecializationEngine
  FunctionEntity × RelevantFacts × Guards
        ↓

High-level Logical Parallel IR
  CellApply / Map / Reduce / Scan / Reindex / Loop / ...
        ↓

Optimization Proofs
  UniformCellResult
  Fusion
  ParallelIndependence
  CheckHoisting
  MaterializationElision
  ViewLegality
        ↓

RoutePartition
  native / external / library / runtime semantic
        ↓

Schedule / Physical Plan
~~~

이 구조는 APEX/Co-dfns/TAIL을 복제하는 것이 아니라, 각 연구에서 검증된 compiler insight를 **full J semantics를 보존하는 RustJ 층 분리**에 맞게 재해석한 것이다.

## 67. 이 절의 architecture review 질문

middle-end 변경 전 다음을 확인한다.

- 새 fact는 semantic identity인가, call-dependent morphology인가?
- fact가 true라는 witness/provenance가 있는가?
- 새 specialization dimension이 실제 code generation/algorithm choice에 필요한가?
- specialization explosion을 어떻게 merge/widen할 것인가?
- semantic DAG를 분석 편의 때문에 특정 matrix/SoA representation에 종속시키고 있지 않은가?
- high-level parallel structure를 너무 일찍 scalarize하고 있지 않은가?
- external backend restriction을 J language restriction으로 착각하고 있지 않은가?
- check elimination이 J error semantics를 바꾸지 않는가?
- fusion의 이득을 arithmetic op 수가 아니라 memory pass/kernel/transfer 관점에서도 평가하는가?
- nested parallelism flattening은 schedule/optimization proof로 남아 있는가?

---

## 68. 추가 소스: APEX / Co-dfns / TAIL-Futhark

### [APEX-1] Robert Bernecky — APEX source repository

https://gitlab.com/bernecky/apex

Robert Bernecky의 APEX source distribution. RustJ는 source repository를 APEX implementation lineage의 1차 자료로 사용하되, architecture claim은 thesis/papers와 함께 교차 검증한다.

### [APEX-2] Robert Bernecky — APEX / array morphology thesis material

https://www.snakeisland.com/ms.pdf

검토 항목:

- SSA conversion
- array morphology/data-flow analysis
- interprocedural/semi-global propagation
- call-site specialization/cloning
- type/rank/shape/value/property inference
- liveness/storage reuse
- loop fusion/temporary elimination
- backend representation과 APL semantics 사이의 tension

### [CODFNS-1] Aaron W. Hsu — The Key to a Data Parallel Compiler

https://dl.acm.org/doi/10.1145/2935323.2935331

DOI: 10.1145/2935323.2935331

핵심 근거:

- Node Coordinate Matrix로 AST inter-node relationships encoding
- Key/grouping operation을 이용한 subtree computation
- compiler implementation 자체를 data-parallel array operations로 구성하는 전략

### [CODFNS-2] Co-dfns current source — parser/AST manual

검토 pin: `4e6d3e3002f2109360d24278776c5b5a4f65db0d` (2026-09-29)

https://github.com/Co-dfns/Co-dfns/blob/4e6d3e3002f2109360d24278776c5b5a4f65db0d/docs/MANUAL.md

핵심 근거:

- parser returns AST/exports/symbols/source
- AST is an inverted table
- parent/depth/type/kind/name/lex/varbind/source range columns
- compiler subset limitations are explicit rather than silently reinterpreting source

### [CODFNS-3] Co-dfns current compiler transform source

검토 pin: `4e6d3e3002f2109360d24278776c5b5a4f65db0d`

https://github.com/Co-dfns/Co-dfns/blob/4e6d3e3002f2109360d24278776c5b5a4f65db0d/cmp/TT.apl

검토 항목:

- columnar AST transforms
- scope/binding linkage
- mutation marking
- dfn lifting
- dead-path removal
- primitive mapping/lowering preparation

### [CODFNS-4] Co-dfns Performance Guidelines

검토 pin: `4e6d3e3002f2109360d24278776c5b5a4f65db0d`

https://github.com/Co-dfns/Co-dfns/blob/4e6d3e3002f2109360d24278776c5b5a4f65db0d/docs/PERFORMANCE.md

핵심 근거:

- GPU critical path
- kernel count
- fusion
- memory bandwidth/traffic
- CPU↔GPU copy cost
- primitive별 parallel complexity의 차이

### [TAIL-1] Martin Elsman et al. — Dyalog'16: Compiling a Subset of APL into Performance Efficient GPU Programs

https://elsman.com/pdf/Dyalog16.pdf

핵심 근거:

- APL의 dynamic typing/rank polymorphism/value-sensitive typing이 compiler challenge
- APL → typed array IL(TAIL) → Futhark
- Futhark의 constant folding/loop fusion/nested parallelism flattening/coalesced-memory optimization
- 연구 compiler subset의 static scope/static rank/whole-program/no-execute 제한
- 해당 제한을 RustJ에서는 route precondition으로만 사용

### [TAIL-2] Henriksen et al. — APL on GPUs: A TAIL from the Past, Scribbled in Futhark, FHPC'16

https://elsman.com/pdf/fhpc16futhark.pdf

DOI: 10.1145/2975991.2975997

핵심 근거:

- high-level array combinator가 parallel intent를 보존
- TAIL의 base/rank/shape-oriented type system
- polymorphic calls의 explicit type/rank instance lists
- map/reduction nests를 explicit하게 Futhark에 전달
- fusion, nested parallelism, coalesced access optimization
- high-level loop placement가 GPU memory-bound/compute-bound behavior를 바꿀 수 있음

