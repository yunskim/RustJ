[English](PROJECT.md) | **한국어 — 정본(canonical)**

# RustJ 통합 프로젝트 문서

> 상태: **유일한 권위 문서(authoritative project document)**  
> 문서 갱신일: 2026-10-13
>
> 앞으로 아키텍처, 설계 결정, 구현 계획, 지원 범위, 진행 상태, 검증 정책과 주요 검증 결과는 이 문서에 통합한다.  
> [FOUNDATIONS.ko.md](FOUNDATIONS.ko.md)는 RustJ가 왜 compiler-oriented architecture를 택하는지, interpreter 전통에서 무엇을 보존해야 하는지, 어떤 compiler 설계가 J에서 회귀가 되는지를 규정하는 **필수 설계 기반 문서**다. frontend·Semantic IR·runtime/JIT/AOT 경계·rank/CellApply·target 설계를 변경하기 전 반드시 함께 검토한다.  
> 그 외 개별 설계 보고서·진행 보고서·체크리스트 Markdown 파일은 새로 만들지 않는다. 기계가 생성한 측정 원자료(JSON/JSONL)는 `reports/`에 별도로 보존한다.

<a id="read-intro"></a>
## 처음 읽는 사람을 위한 소개 — RustJ를 어떻게 이해하면 되는가

이 문서는 구현 세부사항과 검증 기록까지 포함하는 통합 문서이므로, 처음부터 모든 절을 순서대로 읽으면 RustJ의 전체 그림보다 세부사항이 먼저 보일 수 있다. 처음 읽을 때는 먼저 이 절의 **네 단계 mental model**만 잡고, 이후 필요한 세부 절로 내려가는 것을 권장한다.

### RustJ를 한 문장으로 설명하면

RustJ는 **J의 언어 의미를 jsource와 호환되게 보존하면서, 그 의미를 분석 가능한 배열 계산으로 바꾸고, 최적화한 뒤 CPU·GPU·외부 compiler 등 여러 실행 경로로 내릴 수 있게 만드는 compiler/runtime**다.

중요한 점은 J source를 곧바로 loop나 GPU kernel로 번역하지 않는다는 것이다. J에는 Rank, adverb, conjunction, hook/fork/train, name lookup처럼 **언어 의미 자체가 계산 구조를 만드는 요소**가 많다. 이 의미를 너무 일찍 loop나 kernel로 바꾸면 J의 의미도 잃고 optimizer가 사용할 고수준 정보도 잃는다.

따라서 RustJ는 다음 네 질문을 서로 다른 단계에서 묻는다.

~~~text
J source
   ↓
Frontend
   "이 J 프로그램은 무엇을 의미하는가?"
   = 의미 구조
   ↓
J Graph IR
   "그 의미가 어떤 배열 계산을 만드는가?"
   = 계산 구조
   ↓
Execution IR
   "정확히 실행하려면 무엇이 반드시 일어나야 하는가?"
   = 실행 의미와 의존관계
   ↓
Physical Plan / backend
   "이 하드웨어에서 실제로 어떻게 실행할 것인가?"
   = 실제 실행 방법
   ↓
execution
~~~

처음에는 다음 네 줄만 기억해도 충분하다.

> **Frontend = 의미 구조**  
> **J Graph IR = 계산 구조**  
> **Execution IR = 실행 의미와 의존관계**  
> **Physical Plan = 실제 실행 방법**

### 왜 굳이 네 단계로 나누는가

각 단계가 답하는 질문이 다르기 때문이다.

| 단계 | 핵심 질문 | 대표적으로 보존/결정하는 것 | 아직 결정하지 않는 것 |
|---|---|---|---|
| Frontend | J가 무엇을 의미하는가? | noun/verb/adverb/conjunction, derived function, Rank, Hook/Fork, names/bindings | fusion, buffer, GPU kernel |
| J Graph IR | 어떤 계산 graph인가? | producer/consumer, branch/join, CellApply, Reduce, dataflow | 특정 CPU/GPU 실행법 |
| Execution IR | 올바른 실행에 무엇이 필요한가? | explicit operation, SSA/data dependency, semantic check, effect/error order | AVX2, GPU block size, concrete buffer |
| Physical Plan | 이 기계에서 어떻게 실행할까? | route, fusion 결정, schedule, layout, buffer, SIMD/thread/GPU mapping | — |

이 분리는 단계가 많아 보이기 위한 것이 아니다. **J 의미를 잃지 않으면서 최적화 선택을 가능한 늦게까지 열어 두기 위한 것**이다.

예를 들어 같은 논리적 실행 의미인 CellApply(f, y)는 target과 cost에 따라 다음 중 하나가 될 수 있다.

~~~text
CPU scalar loop
CPU SIMD + multicore
GPU kernel
MLIR / external compiler
verified library call
~~~

Execution IR에서 이미 하나를 고르면 optimizer와 backend의 선택 공간을 너무 일찍 닫게 된다.

### jsource와 무엇이 다른가

jsource도 parser reduction을 하고 derived verb, hook/fork/train 등의 의미 구조를 만든다. 차이는 **그 reduction 결과를 사용하는 방식**에 있다.

~~~text
jsource
tokens → enqueue → parser reduction
                     ↓
               semantic action
                     ↓
              runtime execution
~~~

jsource는 interpreter/runtime 흐름 안에서 parser reduction과 semantic action·실행이 밀접하게 이어진다. 따라서 별도의 compiler representation을 여러 층에 걸쳐 오래 유지할 필요가 상대적으로 작다.

RustJ도 frontend에서는 jsource-compatible reduction을 해야 한다. 그러나 reduction 결과를 바로 실행해서 소모하지 않는다.

~~~text
RustJ
parser reduction
      ↓
J semantic structure
      ↓
computation graph
      ↓
execution semantics
      ↓
physical realization
~~~

즉 **RustJ에 중간 단계가 많아서 차이가 생긴 것이 아니라, 분석·최적화·여러 backend를 지원하려면 의미를 단계별로 보존해야 하므로 중간 단계를 의도적으로 둔 것**이다.

### JAXA에서 무엇이 발전했는가

JAXA의 중요한 출발점은 J의 function composition과 array notation 자체에서 optimization-relevant structure를 읽어내는 것이었다. 예를 들어 Fork를 branch/join으로 보고, composition을 producer/consumer chain으로 보는 관점이다.

RustJ는 이 아이디어를 이어받으면서 한 가지 경계를 더 명확히 했다.

~~~text
JAXA의 핵심 관심
J syntax / analyzer
      ↓
optimization-relevant structure

RustJ
J semantic structure
      ↓
J computation graph
      ↓
execution semantics
      ↓
physical realization
~~~

즉 RustJ에서는 **J 언어의 의미 구조와 optimizer가 보는 계산 구조를 같은 것으로 취급하지 않는다.** 이것이 JAXA에서 RustJ로 넘어오며 명확해진 핵심 발전 중 하나다.

예를 들어 f"1 y에서 frontend는 먼저 다음 의미를 보존해야 한다.

~~~text
Rank conjunction (")
  ├─ left operand:  f
  └─ right operand: 1
          ↓
      derived verb
          ↓ apply
          y
~~~

그 다음 J Graph IR이 같은 의미를 optimizer 관점에서 다음과 같은 cell application으로 드러낼 수 있다.

~~~text
y
│
▼
Rank / CellApply
│  function = f
│  cell rank = 1
▼
result
~~~

첫 번째 표현은 **J가 무엇을 의미하는가**를 소유하고, 두 번째 표현은 **그 의미가 어떤 계산 구조를 만든다**를 소유한다. 둘 중 하나가 다른 하나를 대체하지 않는다.

### 다른 compiler와의 관계

이 전체 방향은 RustJ만의 특이한 발상이 아니다. LLVM, MLIR, XLA/JAX, Futhark 등 현대 compiler도 일반적으로 **높은 수준의 의미/연산 구조를 보존한 뒤 점진적으로 target-specific representation으로 lowering**한다.

RustJ의 특징은 그 보편적인 compiler 구조 앞단에 **full J semantics를 소유하는 frontend**를 강하게 둔다는 점이다. 다른 array compiler의 좋은 layering 원칙을 참고하되, Rank·derived verb·Hook/Fork·name semantics 같은 J 고유 의미를 compiler 편의를 위해 다시 정의하지 않는다.

### 처음 읽을 때의 권장 순서

**첫 번째 읽기(전체 그림):** 이 소개 → [§1 프로젝트 목적](#read-goal) → [§2 최상위 아키텍처](#read-architecture). 여기까지 읽으면 네 단계의 책임과 각 단계가 아직 결정하지 않는 사항을 설명할 수 있어야 한다.

**두 번째 읽기(핵심 계약):** [§3 J Semantic IR](#read-semantic) → [§4 J Graph·Execution Semantic Lowering](#read-graph-execution) → [§5 Logical/Physical Plan·Executor](#read-physical). 필요한 단계의 계약만 읽고, 모든 예제와 역사 기록을 연속해서 읽을 필요는 없다.

**세 번째 읽기(구현 시):** [§9 지원 범위](#read-scope) → [§10 이행 체크리스트](#read-roadmap) → [§11 검증 정책](#read-validation) → [§12 최신 상태](#read-status).

**선택적 심화:** [§13 선행 구현·JAXA 역사·소스 조사](#read-references)와 [심화 기술 계약](#read-advanced)은 특정 설계 선택을 검증할 때 참조한다. 과거 측정·완료 기록은 현재 HEAD의 성공 주장으로 읽지 않는다.


### 빠른 안내 — 현재 우선순위와 문서 읽기

- **정의 실행·E2E 최신 상태(2026-10-07):** 일반 explicit/direct 호출에 이어 for/for_name 반복, 중첩 direct/문자열 explicit의 독립 scope, A3 함수 참조 전달을 구현했다. 기존 definition 수용 테스트 21개 모두 활성화했다. 본문 CFG lowering/compiled execution, 일반 locale 및 CUDA 실행은 후속이다. 아래 Definition 후속 검증 절을 따른다.
- **목표와 원칙:** full J의 의미를 보존하는 Rust 커널/컴파일러. C는 차분 oracle이며 정상 실행 fallback이 아니다. Logical Array와 Physical Representation은 분리한다.
- **현재 우선순위:** M2 tokenizer → enqueuer → parser 의미 수렴을 계속한다. [§O.5 프레임워크 이행 체크리스트](#framework-migration-checklist)와 [§Q 전체 jsource 최적화 이행 체크리스트](#jsource-optimization-migration) 및 [§10 IO 이행 체크리스트](#out-of-core-io-checklist)를 M2→M3→M4 완료 게이트의 단일 추적표로 사용한다. Graph IR의 구조·부분 facts 보존과 최적화/실행 허가는 별개다. 이후 M3 경계를 정리하고 M4 Native CPU vertical slice를 검증한다. GPU 친화적 설계는 유지하되 CUDA 실행 구현은 유보한다. 외부 route는 capability를 증명한 영역에서 점진적으로 연다.
- **가장 최근의 Windows 검증 기록(2026-10-08, frontend admission/handoff 및 decimal/extended/rational literal 및 rational 산술/extended 나눗셈/rational reduce·균일 rank 조립 변경):** default/portable 각각 **746 passed / 0 failed / 0 ignored**, Python **85 passed**, fmt/clippy 통과. Definition 호출 감사 **144/144 matched**, 별도 NAME 효과/배열 감사 **104/104 matched**를 기록했다. 이는 해당 Windows 검증 범위의 결과이며 Linux·GitHub CI·GPU 또는 full J 동등성 검증이 아니다. 과거 gate 수치는 각 시점의 기록으로만 읽는다.
- **읽기 순서:** 설계 근거는 [FOUNDATIONS.ko.md](FOUNDATIONS.ko.md), 이름·효과·실행 경로의 조건은 [동적 의미와 컴파일 경계 계약](#dynamic-semantic-boundaries), 실행 가능한 작업과 검증은 §10–§11을 따른다. 과거 단계별 gate는 이력이며 최신 지원 상태와 구분한다. 정본·체크리스트를 별도 Markdown으로 분리하지 않는다.


<a id="read-goal"></a>
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

### 1.2 핵심 배열 모델 결정 — Logical Array와 Physical Array를 분리한다

Logical Array는 J-visible type·shape·atom order와 boxed/sparse 의미를 소유한다. Physical Array는 buffer·stride·offset·layout·placement 등 실행 표현을 소유한다. logical value의 존재는 별도 buffer 할당을 뜻하지 않는다.

모델과 완료 조건은 [§6](#logical-physical-array-model), GPU view/placement는 §7, alias/lifetime 조건은 §8에서 정의한다. 도입부에 상세 구조와 완료표를 복제하지 않는다.

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

<a id="read-architecture"></a>
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

#### 기억용 4단계 요약 — 상세 구현에서 다시 확인할 경계

처음 읽는 사람은 [도입부의 네 단계 모델](#read-intro)부터 읽는다. 이 절의 Rank/Fork 예제와 아래 §2.1 이후 계약은 그 모델을 구현 관점에서 구체화한 것이다.

**Execution IR은 semantic execution plan이지 하드웨어 실행 계획이 아니다.** Check·effect·error ordering과 SSA dependency는 여기서 확정하지만, AVX2 선택, GPU block 크기, concrete BufferId와 materialization은 physical planning이 담당한다. 동일한 `CellApply(f, y)`도 여러 backend realization을 가질 수 있다.

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

#### 2.5.1 RouteRegion boundary contract

mixed route의 핵심은 “어떤 op를 어느 backend로 보낼 것인가”보다 **region 경계에서 J semantics를 어떻게 끊김 없이 보존하는가**다. RoutePartition은 physical buffer plan이 아니므로 boundary는 먼저 logical/effect contract로 표현한다.

개념적으로:

~~~text
RouteBoundary
  producer_region
  consumer_region
  live_in / live_out ValueId
  effect_order_in / effect_order_out
  semantic_checks crossing or anchored
  required witnesses / guards
  logical representation requirements
  source / J Graph provenance
  bridge-lowering responsibility
~~~

`logical representation requirements`는 `Dense`, sparse/boxed semantics, dtype/shape/rank 같은 **J-visible 또는 route-precondition 수준**의 요구를 뜻한다. 구체 `BufferId`, device pointer, stride/alignment, host↔device copy는 RoutePartition이 아니라 bridge lowering / Physical Plan이 결정한다.

##### boundary legality

region 경계는 다음 조건을 모두 만족할 때만 허용한다.

1. boundary live-in/out이 모두 explicit SSA/logical value로 표현된다.
2. value가 없어도 관찰 가능한 effect/error dependency가 있으면 explicit ordering edge로 남는다.
3. `SemanticCheck`를 잘라내거나 두 route에서 중복 실행해 error precedence를 바꾸지 않는다.
4. producer와 consumer의 representation/precondition 사이에 합법적인 bridge가 존재하거나, 아직 미결이면 partition을 commit하지 않는다.
5. dynamic guard가 필요하면 guard는 region의 observable effect보다 먼저 평가된다.
6. region 전체가 선택된 route의 semantic capability를 만족한다. op별 support의 단순 교집합만으로 region legality를 선언하지 않는다.
7. external/runtime route 실패를 이유로 이미 effect를 수행한 producer region을 자동 replay하지 않는다.

##### live-out은 value만이 아니다

다음 두 경우를 구분한다.

~~~text
value live-out
  producer result가 다음 region의 input으로 사용됨

effect live-out
  반환값은 쓰이지 않아도 write / I/O / error-order dependency가 다음 region의 관찰 결과에 영향을 줌
~~~

fork/selector에서 선택되지 않은 branch의 값이 dead여도 effect/error가 live-out일 수 있다. RoutePartition은 pure dataflow liveness만 보고 branch나 check를 제거하면 안 된다.

##### bridge lowering

RoutePartition 이후 별도 bridge lowering이 실제 representation 이동을 만든다.

~~~text
Logical ValueId
    ↓ route boundary requirement
Bridge Lowering
    ├─ no-op compatible handoff
    ├─ materialize
    ├─ layout/encoding conversion
    ├─ host ↔ device transfer
    ├─ external handle wrap/unwrap
    └─ synchronization / completion edge
    ↓
consumer route representation
~~~

bridge 비용은 route selection의 cost input이 될 수 있지만, 이미 선택한 route를 정당화하기 위해 semantic constraint를 바꾸면 안 된다.

##### 현재 코드와 목표의 차이

현재 `lowering.rs::partition_plan`은 operation별 `RouteDecision`을 `ValueOnly / SemanticCheck / PureArray / RuntimeSemantic` class로 나눈 뒤 **인접한 같은 class를 contiguous range로 묶는 v0 분석 도구**다.

현재 `RouteRegion`은:

~~~text
class
operations: Range<usize>
~~~

만 가진다. 아직 chosen external route, boundary live-in/out, bridge, effect edge, region-wide legality proof를 소유하지 않는다. 따라서 현재 `partition_plan` 결과를 최종 mixed-route execution plan으로 해석하지 않는다.

##### 구현/검증 순서

mixed-route 구현에 착수할 때는 다음 순서로 확장한다.

1. RouteRegion live-in/live-out 계산 + verifier
2. value live-out과 effect live-out을 분리하는 회귀
3. SemanticCheck/order edge가 boundary에서 보존되는지 검증
4. route-wide legality aggregate와 guard ownership
5. representation-neutral BridgeRequirement
6. bridge lowering에서 concrete transfer/materialization 생성
7. same Logical IR에 target별 다른 RoutePartition을 만들어 semantic result/error가 같은지 differential 검증


##### M3-RB — A3→RouteBoundary 의미 보존 증명 및 검증 계약 (2026-10-07; 문서 설계만)

**범위:** 기존 §2.5.1 RouteBoundary 개념에 검증 가능한 승인 조건을 부여한다. 현행 `lowering.rs::partition_plan`은 `RouteRegion { class, operations: Range<usize> }`라는 **연속 구간 후보 분류기**이지 실행 승인기나 혼합장치 스케줄러가 아니다. 새 필수 Route IR, CPU Parallel IR, Data Movement IR은 만들지 않고, 추후 기존 route contract에 연결되는 검증 증거(sidecar 또는 동등한 형태)를 사용한다.

**권위 기준:** *변환 전 검증된 원본 A3 스냅샷*을 보존하여 비교한다. `Plan::verify`는 구조·schema/registry·SSA의 일부 검증기이며, 임의 수정된 `CallOp.effect`, `possible_errors`, `SpeculationSemantics`, `FactWitness`가 원래 J 의미에 참이라는 독립 인증서가 아니다. 최적화 후보가 수정한 A3를 자기 자신의 유일한 증거로 삼을 수 없다. 원본 source/primitive registry·op identity/`j_origin`/span과 생성자 증명을 연결한다. fingerprint는 stale 감지에는 유용하나 의미론 증명은 아니다.

**승인 인터페이스(개념, 새 Rust 타입 확정 아님):**

~~~text
Input:
  immutable verified source A3, proposed RouteRegion coverage
  chosen route/target, optional rewrite/fusion witnesses
  semantic facts/guards, required value/effect/bridge obligations

Output:
  source op/check/write mapping, region live-in/out, value+effect ordering
  per-region capability + discharged proofs or owned guards
  representation-neutral bridge obligations and commit/replay restrictions

Verdict:
  Verified      모든 의무를 이미 증명, 해당 route commit 가능
  GuardRequired 실행 전 올바른 guard/실패 경로 연결이 필요; 즉시 실행 허가 아님
  Rejected      위조·누락·미지원·미증명; 안전한 fallback 또는 Unsupported
~~~

**반드시 검증할 불변조건**

| ID / 소유 | 필요 증거와 검증 의무 | 거부해야 하는 반례 |
|---|---|---|
| **RB-01 op coverage** | 원본 단일 블록 `operations[0..N]`을 비중복·무간극·원래 순서의 region range로 **모두** 커버한다. 원본 OpId/span/`j_origin` 대응과 최종 result를 보존한다. Fusion/rewrite로 1:N 또는 N:1 대응이 생기면 별도의 *완전한 의미론적 등가 witness*를 요구한다 | 빠진 op, 중복·겹침 range, 역순, 유효하지 않은 ID, 원본에 없는 op provenance. N=0인 진짜 빈 A3에는 빈 partition을 허용 |
| **RB-02 value/live-in/out** | Call의 left/right, `SemanticCheck.constraint`의 입력, read/name/guard, 결과의 producer, 다른 region의 소비자, `Plan.result`까지 def-use closure를 계산한다. **Check는 출력 SSA가 없어도 값을 소비**한다 | 결과 값만 쫓아 Check 입력·cross-region producer를 유실, dangling live-in, stale value version, return value 누락 |
| **RB-03 Check obligations** | 원본의 zero-result `SemanticCheck`마다 **한 소유자**를 부여한다: (a) 원래 J error class·우선순위로 실행, (b) 안정된 입력/생성 경로가 검증된 `FactWitness`로 정당하게 discharge, (c) 오류·관찰 순서 동등성이 입증된 guard/check로 대체 | Check 탈락·중복·오류 class 변경·호출 뒤 이동·자기선언 witness. **증명된 Check 제거는 허용**하지만 Check의 무조건 삭제는 금지 |
| **RB-04 순서·첫 오류** | SSA def-use, `Operation.order_after`, Check→Call, observable effect/error edge, `Plan.write.after`의 순서 의무를 원본으로부터 보존한다. 첫 J 오류를 worker/GPU 완료 순서로 정의하지 않는다 | 값이 사용되지 않는 실패 가능 op·effect-live branch·Check 삭제, effect 앞뒤 재배열, 첫 오류 변경, error handler/try-catch 관찰 변화 |
| **RB-05 dynamic NAME와 Write** | `ReadNoun {symbol,version}`의 noun snapshot과 `CallTarget::Dynamic` / function NameRef의 late lookup을 분리한다. `Plan.write`는 **operations vector 밖의 독립 commit event**: symbol/value/previous/proposed/span/after를 반드시 보존 | op 전체를 커버했다는 이유로 Write 생략, 조기 function target 고정, 옛 binding snapshot 재사용, write 중복·순서 역전·effect 이후 무조건 replay |
| **RB-06 region-wide legality** | `LoweringRegistry::legal_candidates`의 *후보*를 넘어서, 선택 region 전체의 primitive/valence·Rank/CellApply/empty-frame fill, effect/error, target feature, runtime guards, alias/bridge 전제조건을 검증한다 | per-op support의 단순 합집합으로 전체 region 실행 승인, `ReferenceSequential`을 compiler-native kernel로 오인, Unknown effect/error를 Pure/NoError로 몰래 완화, tolerance/boxed/sparse/가변 cell 결과·재결합 미증명 승격 |
| **RB-07 bridge/fallback** | 경계에는 J-visible dtype/shape/rank/boxed/sparse와 value/effect ordering **요구사항**만 둔다. 구체 `BufferId`/stride/host↔device transfer/ready token은 후속 Physical/Bridge에서 결정한다. Guard는 observable effect 이전에 실행한다 | legal bridge 없는 cut, 뒤늦은 guard, committed effect 이후 producer replay, GPU 메모리 배치의 부재를 무시하고 verified 처리 |
| **RB-08 proof freshness/diagnostics** | 증거를 원본 A3/registry/op/version/guard 관찰 시점에 묶고, 검증 불가 의무와 Reject 이유를 source op/region에 귀속한다 | 다른 op의 proof 재사용, 바뀐 input/name version에 stale witness 재활용, 거짓 error summary, 미지원 backend/bridge를 Verified 처리 |

**구체적인 테스트 설계 (아직 테스트 코드를 쓰거나 실행하지 않음):**

| A3 기준 소스 | 정상 조건 | 하나씩 위조하여 거부해야 할 사례 |
|---|---|---|
| `1+2` | 두 literal의 ValueOnly → Elementwise 후보, result/value live-in 보존; CPU `ReferenceSequential`은 후보이지 native kernel 완성 증거가 아님 | result op 누락, value-live-in 누락, target capability 불일치 |
| `1 2+3 4` | PrefixAgreement의 생성·입력 shape 증거가 유효하면 Check 불필요 | witness만 Some으로 바꾸거나 실제 shape/version을 변경해 Check 생략 |
| `1 2+1 2 3` | zero-result Length Check가 Call보다 먼저 발생하고 J Length 유지 | Check 탈락/중복, Return만 보존, Check 뒤로 이동 |
| `3 { 10 20 30` | Gather의 Index Check/오류 우선순위 유지 | Check를 제거·Length로 변조·커널 뒤로 보냄 |
| `future 3` | 미지원 native call은 RuntimeSemantic 후보로 남김 | 근거 없는 GPU/native route 승격 |
| `a`, `a=:1+2` | noun snapshot과 별도 Write commit 및 이름 버전·순서 보존 | write 미매핑, 미래 버전 선읽기, write 이후 무조건 replay |
| `+/\"1 (2 3$ i.6)` 및 zero-frame 사례 | frame/cell 구분, zero-frame virtual fill 및 결과 type/shape assembly 의무를 보존 | empty frame을 no-op 처리, CellApply를 무증명 고정형 parallel map으로 변경 |
| 같은 A3의 CPU/GPU/External 후보 | 세 후보 모두 **같은 원본**에 대해 값/타입/Shape/오류/효과 의무를 충족해야 함. GPU 실구현은 별도 보류 | GPU 실행기가 없는 상태에서 실행 완료·성능 검증으로 기록 |

**실행할 검증 단계와 수용 게이트:**

1. **RB-V0 coverage:** 유효 A3 + 빈 블록 포함 모든 op-range 전수 매핑·순서·빈 partition, gap/overlap/invalid ID 음성 검사.
2. **RB-V1 semantic edges:** value/check/`Plan.write`/result 모두의 live-in/out, `order_after`, first-error, cross-region effect-live mapping을 검사. Check마다 실제 실행/증명된 discharge/동등 guard 하나를 입증.
3. **RB-V2 legality & guards:** 원본 semantic facts와 witness provenance, target/region-wide legality, guard placement 이전의 effect frontier, fallback safety를 재검증. GuardRequired는 아직 실행 불가.
4. **RB-V3 differential:** 각 정상 계획과 **단 하나의 invariant를 위조한 계획**을 비교. J 기준 C oracle, RustJ closed A3 reference(지원 입력만), RustJ interpreter의 value/type/Shape/atom/error precedence/effect/name state. Closed A3 executor가 dynamic NAME/Write oracle이 아닌 점을 명시.
5. **RB-V4 multi-route:** 같은 원본에서 target별 RoutePartition 후보를 만들고 증거를 교차 비교. 미구현 GPU/External은 정적 Rejected/GuardRequired 확인까지만; 실제 device/transfer 성능은 M4/M5/M6 별도 수용으로 둔다.

**완료 판정:** RB-01~08/RB-V0~V4가 정본·영어 미러에 들어간 것은 **M3-RB 설계 명세 완료**뿐이다. **구현/검증 완료**는 실재하는 verifier, 각 negative test의 PASS, 3자 의미론 차분 결과와 증거 기록이 필요하다. `partition_plan`의 range/class, A3 `verify` 통과, 문서 수정만으로 M3·M4·HE-01을 완료 처리하지 않는다.


<a id="sw-mlpl-reference-checklist"></a>
##### 외부 구현 사례: sw-MLPL → RustJ 적용·검증 체크리스트 (2026-10-08, 설계 조사만)

**검토 대상 및 성격:** [sw-MLPL](https://github.com/sw-ml-study/sw-mlpl/tree/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f)의 2026-10-03 `main` snapshot (`d53c35f810aa`)에서 구현·테스트·설계 문서를 대조했다. 이 절은 **기존 M3 RouteBoundary / M4 Physical CPU / M5+ heterogeneous planning의 구현 참고와 수용 항목**이며 새로운 필수 IR, 실행 경로, RustJ 언어 문법, 별도 진행 문서를 추가하지 않는다. RustJ 기존 RB-01~08, RB-V0~V4, M4→M5 검증, §17/HE 체크리스트가 우선한다. **[x] 외부 설계·소스 검토 / [ ] RustJ 도입·검증은 아직 미실시**이다. 이 문서 갱신을 기능 구현/테스트 통과로 세지 않는다.

**비교로 확정한 경계:** sw-MLPL은 Rust 인터프리터, NumPy식 trailing-axis broadcasting, named-axis metadata, 일부 compile-to-Rust, MLX resident tape 및 제한된 CUDA 경로를 갖는다([S0], [S2], [S3]). RustJ는 J의 Rank/CellApply 및 zero-frame virtual fill, NAME/locale/`".`, J dtype·boxed·sparse·fit·오류·효과 우선순위를 보존한다. sw-MLPL의 `AxisSpec`/device handle/컴파일 가능 subset을 RustJ의 J semantic identity, Logical `ValueId` 또는 전체 J 지원 범위로 **그대로 치환하지 않는다**. 특히 `[0,3]`의 zero frame과 `[2,0]`의 positive frame/empty cell은 원소 수가 같아도 실행·prototype 의무가 다르다(§P.12).

**실행 체크리스트** — 한 항목을 닫을 때 *변경 코드 위치 + PR/commit + 재현 입력/음성 케이스 + 실행한 테스트 명령과 실제 결과 + 해당 source 근거*를 §17의 기존 단계 체크리스트에 연결한다. CI 대기, 테스트 미실행, 특정 backend 부재는 통과가 아니다.

| ID / 단계 | RustJ에 채택할 방법 (기존 계약에 추가) | 수용 게이트 / 실패해야 할 반례 | 근거 |
|---|---|---|---|
| **SW-01 / M3** | [ ] `LoweringRegistry`의 *기계 판독 가능한 coverage/capability 표*를 primitive + valence + rank/cell + J dtype/representation + effect/error + route/target/guard 축으로 점검한다. supported, runtime/guarded, 구현 미지원 후보를 구분하되 **op별 supported ≠ region Verified**로 고정한다. 문서의 범위 선언과 테스트의 registry 목록이 일치하게 한다. | 새 lowering 등록/제거 시 coverage gate 갱신 요구; 미등록 op, 틀린 valence, blocked boxed/sparse, stale witness, zero-frame 의무 누락 및 `ReferenceSequential`을 native로 오인한 계획은 `Rejected`/`GuardRequired`; RB-06/08, RB-V2~4까지 통과해야 완료. | [S4], [S5] |
| **SW-02 / M3** | [ ] `RankPlan`/A3 `IterationDomain`/SemanticCheck에 이미 있는 shape, frame/cell/reduction-axis, 결과 cell 조립, 추론 증거를 **공통 구조화 진단/설명**으로 제공한다. axis 선택 규칙을 중복 구현하지 않고 source span/provenance 및 `ErrorKind + ErrorContext`를 유지한다. 사용자 문법에 MLPL named axes를 이식하지 않는다. | 동일한 Rank/Shape 문제의 A3/diagnostic 표시가 일관되며, 잘못된 축·중복/범위 오류·`[0,3]` 대 `[2,0]`·불명확한 결과 cell 조립·원래 첫 J 오류가 보존되는지 회귀 검증; diagnostic이 J 오류를 가리거나 변경하면 실패. | [S1], [S2], [S7] |
| **SW-03 / M4** | [ ] 여러 compiler-native 작은 사례를 묶어 컴파일 비용을 분산하는 **batched parity harness**를 기존 3자 차분(jsource C, RustJ semantic reference, 실제 Physical/native route)의 보조 도구로 사용한다. 지원 범위 밖 입력은 명시적으로 분류한다. | 수치 scalar 하나만이 아니라 dtype·shape·ordered atoms·boxed/sparse(해당 지원 범위)·J error class/first error·effect/name 상태까지 비교. 가상 Cell/fill, Checked overflow, fit/tolerance, 동적 NAME, unsupported route negative를 포함. compiler가 reference evaluator를 호출한 것은 native 통과가 아님. | [S6], [S4] |
| **SW-04 / M4** | [ ] parse/compile/실행 시간, allocation/peak memory, materialization, 읽기·쓰기 bytes를 나누고 **동일한 실제 lowering 결과**를 기준으로 micro/macro benchmark를 남긴다. 변경 blast radius에 맞는 빠른 proxy test와 느린 release gate를 구분한다. | CPU reference/native 동일 입력 비교, scalar/short array/reduce/reindex/빈 frame·복합 rank/effect-boundary 별도 기록; 결과 검증이 선행하며 환경/커밋/설정·median·회귀 기준 기록. 실행하지 않은 benchmark는 증거가 아님. | [S8], [S9] |
| **SW-05 / M5+** | [ ] CPU/GPU/External의 실제 Physical/Bridge 경계에 upload/download, materialize, submit, fallback, transfer bytes, sync/ready wait, guard miss를 **측정 이벤트**로 정의한다. 실제 storage/lease/version과 event를 연결하되 A3 `ValueId`에 device handle을 넣지 않는다. | zero-transfer CPU baseline; 같은 J값의 physical residency 변화가 J-visible 의미를 바꾸지 않음; transfer-before-produce/read-before-ready/stale version/double-commit 거부. GPU 실장치가 없으면 설계/정적 verifier만 기록하고 resident 실행·성능을 성공으로 표시하지 않음. | [S3], [S10] |
| **SW-06 / M4→M5+** | [ ] source span, original A3 OpId/`j_origin`, dtype/shape, effect/error check, physical route·materialization·fallback을 연결하는 **선택적 structured trace**를 만든다. 대형 noun 본문은 복사하지 않고 제한된 요약/ID만 기록하며 비용을 별도 측정한다. | trace enable/disable 결과가 J 값·오류·효과·순서에 영향을 주지 않는 회귀; source→A3→physical 대응 및 reject 이유 확인; 구조 진단과 runtime 관찰을 혼동하지 않는다. | [S7], [S11] |

**진행 의존성 및 도입 금지 사항**

- **M2 우선순위 유지:** jsource 기반 frontend/Rank/empty-frame 및 C oracle mismatch 수렴을 먼저 수행한다. SW-01/02가 M2 의미론 수정을 우회하는 shortcut이 되어서는 안 된다.
- **M3 → M4 → M5(→ M6):** SW-01/02는 RB 검증기에 붙이고, SW-03/04는 실제 native CPU vertical slice가 있을 때 측정한다. SW-05는 M5의 transfer/residency 모델 이후, GPU 실제 실행은 별도 M6/재개 승인 뒤에만 한다. SW-06은 관찰 필요 시 단계적으로 추가한다.
- **비채택:** NumPy broadcasting으로 J Rank 대체, J의 `".`/동적 이름을 금지하는 closed-world AOT, MLPL `Value`/`TensorHandle`을 J noun/FunctionEntity/A3 `ValueId`와 통합, autograd tape를 범용 J SSA로 대체, sw-MLPL의 숫자 하나 parity를 전체 J 적합성으로 간주하는 것은 모두 금지한다.
- **완료 상태 (2026-10-08):** [x] 자료 조사·적용 경계 결정 및 출처 고정, [ ] SW-01, [ ] SW-02, [ ] SW-03, [ ] SW-04, [ ] SW-05, [ ] SW-06. 각 항목은 **실제 구현 + 해당 negative/conformance test PASS + 출처·명령·commit/CI 증거**가 있기 전까지 열린 상태다.

**원본 자료 (모두 동일 commit으로 고정; 설계/문서와 실제 구현을 구분)**

- **[S0]** [sw-MLPL README: 언어·backend와 실험적 범위](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/README.md).
- **[S1]** [AxisSpec 공통 axis resolver 구현](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/components/array/crates/mlpl-axes/src/axis_spec.rs).
- **[S2]** [trailing-axis broadcast 구현](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/components/array-element/crates/mlpl-array-ops-element/src/broadcast.rs), [shape/empty 회귀 테스트](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/components/array-element/crates/mlpl-array-ops-element/tests/broadcast_shape_tests.rs).
- **[S3]** [CPU/device TensorHandle](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/components/array/crates/mlpl-tensor-handle/src/handle.rs), [DeviceOps trait](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/components/array/crates/mlpl-tensor-handle/src/ops.rs), [upload/download/submit/fallback counters](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/components/array/crates/mlpl-tensor-handle/src/metrics.rs).
- **[S4]** [컴파일러 coverage boundary 테스트](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/components/syntax-codegen/crates/mlpl-lower-rs/tests/coverage_boundary_tests.rs), [coverage 가이드](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/docs/compiler-coverage.md).
- **[S5]** [supported lowering dispatch coverage 테스트](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/components/syntax-codegen/crates/mlpl-lower-rs/tests/dispatch_coverage_tests.rs).
- **[S6]** [interpreter vs compiled parity harness](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/components/dev-tools/crates/mlpl-parity-tests/tests/parity_tests.rs) — 현재 수치 scalar 중심 비교이므로 RustJ 전체 오라클의 대체가 아니다.
- **[S7]** [typed TraceEvent 구조](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/components/autograd/crates/mlpl-trace/src/event.rs).
- **[S8]** [actual lowering 기반 bench 방식과 수치](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/docs/benchmarks.md), [benchmark harness](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/components/dev-tools/crates/mlpl-bench/benches/interp_vs_compiled.rs).
- **[S9]** [blast-radius별 빠른/느린 테스트 cadence](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/docs/testing-cadence.md).
- **[S10]** [sw-MLPL architecture: resident device seam과 GPU 범위](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/docs/architecture.md).
- **[S11]** [구조적 dataflow renderer 설계(도입 의무 아님)](https://github.com/sw-ml-study/sw-mlpl/blob/d53c35f810aa3a3ccd0f28e98da9a8f34992c61f/docs/dataflow-renderer-design.md).


MLIR은 여러 abstraction의 dialect를 한 module 안에서 공존시키고 dialect conversion으로 점진적으로 lowering할 수 있으므로 RustJ의 multi-level IR 구조와 특히 잘 맞는다.

향후 RustJ Logical IR 전체를 MLIR tooling 안에서 보존할 필요가 생기면 **RustJ-specific MLIR dialect**를 fidelity-preserving bridge로 둘 수 있다. 다만 v0의 필수 구현은 아니다. 초기 adapter는 안전하게 표현 가능한 op만 기존 `tensor/linalg/arith/scf` 등으로 직접 lowering하고, 의미 손실이 생기는 op는 거부한다. StableHLO는 ML framework/compiler 사이의 portability layer를 목표로 하는 high-level op set이므로 NN/tensor subset의 선택적 export 대상으로 본다. LLVM IR/SPIR-V는 더 낮은 execution target으로 사용한다.


---

<a id="read-semantic"></a>
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

jsource와 다시 대조한 결과, RustJ의 공통 entity abstraction은 **jsource의 모든 `A` block type**을 복제하는 것이 아니라 parser/evaluator가 실제 semantic RHS로 다루는 `RHS = NOUN + FUNC`, `FUNC = VERB + ADV + CONJ`에 대응시키는 것이 가장 정확하다.

jsource의 `AD/A`는 noun과 function뿐 아니라 NAME/SYMB 등 runtime block에도 쓰이는 물리적 공통 allocation handle이다. RustJ의 `JEntity`는 이 물리 universe 전체가 아니라 **J expression이 산출·전달하는 semantic entity**만 모델링한다.

개념 모델:

```text
JEntity                         // semantic RHS value
├─ Noun
│   └─ Logical J noun value
└─ Function
    └─ FunctionEntity
        ├─ result POS: Verb | Adverb | Conjunction
        ├─ primitive / explicit / derived identity
        ├─ Hook / Fork / modifier-application construction
        └─ semantic operands: JEntityRef...

separate reference/control layer
├─ lexical NAME / parser control class
├─ NameRef / late-binding reference
├─ binding + version
└─ source/provenance
```

중요한 점은 **NameRef가 noun/verb/adverb/conjunction과 같은 다섯 번째 J entity kind가 아니라는 것**이다. lexical NAME은 lookup 전 parser item이고, function nameref는 binding/lookup 시점을 보존하는 reference representation이다. resolve된 결과가 `JEntity`다. 현재 `FunctionHead::NameRef` 같은 표현은 function identity 내부에서 이 indirection을 보존하는 구현일 수 있지만, 상위 POS universe 자체를 늘리지는 않는다.

또한 jsource의 공통 `AD` header에 `AN/AR/shape` field가 존재한다고 해서 function에 J-array rank/shape semantics가 있다는 뜻은 아니다. current `jtype.h`는 **function의 AN/AR fields를 사용하지 않는다고 명시**한다. RustJ도 `Verb`/`Adverb`/`Conjunction` 자체에 noun-style semantic shape/rank를 부여하지 않는다.

따라서 invariants는 다음과 같다.

1. `JEntity = Noun | Function(POS=Verb|Adverb|Conjunction)`을 semantic RHS universe의 기본 분리로 본다.
2. primitive와 **derived function entity(verb/adverb/conjunction)**의 identity와 result POS를 보존한다.
3. hook/fork/train 및 modifier application의 semantic operand 관계를 보존한다.
4. modifier/bident/trident parser action의 결과를 항상 Function이라고 가정하지 않는다. jsource constructor table에는 즉시 실행되어 NOUN을 만드는 production도 있으므로 parser action 결과의 일반형은 `JEntity`다.
5. monad/dyad valence와 rank contract를 function semantics에 보존하되, 이것은 function object의 array rank/shape가 아니다.
6. rank conjunction의 left/right operand가 verb/noun/gerund/verb-rank form 중 무엇인지 분석 전에 보존한다.
7. name lookup과 binding/version의 시점 차이를 reference layer에서 보존한다. 특히 noun value snapshot과 function nameref late lookup을 같은 규칙으로 뭉개지 않는다.
8. source span/provenance는 entity identity와 구분해 유지할 수 있어야 한다.
9. jsource `V.fgh`는 execution object의 cross-check이지 semantic DAG child의 절대 oracle이 아니다. `h` 등은 실행용 parameter storage로도 재사용되므로 parser production에서 확인된 semantic operand만 DAG edge로 올린다.

#### 3.3.1 derived construction의 결과는 Function으로 한정하지 않는다

current jsource의 hook/bident/trident construction(`cf.c`)은 input POS 조합에 따라 Verb뿐 아니라 Adverb/Conjunction을 만들며, 일부 production은 즉시 실행되어 Noun을 만든다.

따라서 일반 모델은:

```text
parser_action(form, operands)
  -> JEntity
       ├─ Noun
       └─ Function { result_pos, form, operands }
```

이다. `DerivedVerb`는 Function 결과 중 POS가 Verb인 경우의 convenience view일 뿐이다.

예를 들어 `ADV + ADV + VERB`, `CONJ + VERB + CONJ`, `NOUN + CONJ + ADV` 같은 조합은 constructor table의 실제 result POS를 따라야 하며, `NOUN VERB NOUN`처럼 parser action이 값을 즉시 실행해 noun을 산출하는 경우도 공통 `JEntity` result boundary에서 표현할 수 있어야 한다.

#### 3.3.1a gerund는 새 atom type이 아니라 contextually interpreted boxed noun이다

J gerund의 **J-visible 출발점은 boxed noun**이고, 특정 modifier 문맥이 그 noun을 function/entity sequence로 해석한다.

2026-10-04 재검토한 current jsource `cg.c::jtfxeachv/jtfxeach`는 이 해석 결과를 실행하기 위해 원래 boxed noun의 rank/shape를 복사한 `BOX`-tagged 내부 carrier를 만든다. 그러나 jsource 주석 자체가 이 결과를 **“array of boxes라고 주장하지만 각 box slot에는 function type A가 들어갈 수 있다”**고 설명한다. 즉 이것은 J-visible `array of functions` semantic type의 증거가 아니라 runtime 내부 realization trick이다.

따라서 RustJ는 이 internal representation을 semantic type으로 승격하지 않는다. 기본 모델은 다음과 같다.

```text
Boxed Noun
  noun shape/rank
  ordinary boxed payload
      + operator-specific interpretation
          ↓
GerundView / InterpretedEntitySequence
  references to interpreted Function/JEntity values
  source-container shape only when that operator semantics needs it
  origin = boxed noun + interpretation context
```

핵심 규칙:

1. 개별 Verb/Adverb/Conjunction은 noun-style array shape/rank를 갖지 않는다.
2. gerund interpretation이 shape를 필요로 하면 그 shape는 **source boxed noun**에서 보존한다.
3. jsource의 function-containing fake-BOX carrier를 RustJ `Value::Boxed`로 복제하지 않는다. RustJ의 `Value::Boxed`는 J noun invariant를 유지한다.
4. generic `EntityArray`/`EntityArrayView`는 gerund 하나만으로 정당화하지 않는다. 먼저 operator-specific `GerundView` 또는 `InterpretedEntitySequence`로 충분한지 확인한다.
5. 같은 boxed noun이 ordinary data로 쓰이는 문맥과 gerund로 해석되는 문맥을 분리하고, name/function reference의 fix/late-binding 규칙을 보존한다.
6. current `decoded_gerund: Vec<FunctionEntity>`처럼 shape를 버리는 cache/view가 각 gerund operator의 observable semantics에 충분한지는 operator별로 검증한다.

즉 jsource에서 가져올 것은 **boxed noun → context-sensitive function interpretation**이라는 semantic rule이고, function A를 BOX payload에 직접 넣는 내부 storage trick은 가져오지 않는다.

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
   - enqueue-time primitive resolver는 **core J primitive spelling**의 lookup/classification만 담당한다. extension은 ordinary `NAME`으로 유지하고 parser-time 정상 binding/POS lookup에서 semantic entity를 얻는다. parser grammar에는 extension 전용 production을 추가하지 않는다.

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

#### 3.3.2.2 Canonical frontend sentence trace — `+/ y`

frontend orientation 표본은 **이미 noun으로 바인딩된 `y`**에 대해 다음 한 문장을 사용한다.

```j
+/ y
```

이 표본은 짧지만 word formation, enqueue classification, parser-time name lookup, modifier construction, monadic application을 모두 지난다.

~~~text
source bytes
  "+/ y"
    │
    ▼
Word Formation
  word[0] = "+"
  word[1] = "/"
  word[2] = "y"
    │
    ▼
Enqueue
  0: Verb(Add)
  1: Adverb(Insert)
  2: Name("y", lookup_name=true)
    │
    ▼
Parser stack entry for y
  lookup current sentence environment
  Name("y") ──lookup──→ Noun snapshot
    │
    ▼
9-row reduction: row 3 / Adverb
  Verb(+)  Adverb(/)
        │
        ▼
  completed FunctionEntity
    POS  = Verb
    head = PrimitiveAdverb(Insert)
    operand[0] = Function(Add)
    source provenance = words 0..2
    │
    ▼
stack reinsert + rescan
    │
    ▼
9-row reduction: row 0 / MonadEdge
  derived Verb(+/)  Noun(y)
        │
        ▼
  Expr::Monad / completed noun result
    │
    ├─ runtime parser host: 실제 J value를 계산하여 Noun으로 stack에 재삽입
    └─ analysis/static path: application structure를 보존하여 후속 J Graph/A3 분석
~~~

여기서 중요한 경계는 다음과 같다.

- `/`는 enqueue 시점에 `Reduce`가 아니다. **Adverb**이며 row 3이 `+`에 적용해 completed derived Verb `+/`를 만든다.
- `y`는 enqueue에서 단지 lookup 대상 NAME이다. parser stack에 들어갈 때 **그 시점의 환경**을 읽어 noun snapshot을 얻는다. sentence 시작 시 모든 name을 미리 고정하지 않는다.
- parser row 3의 결과는 target-independent `FunctionEntity`다. CPU/GPU/fusion 정보가 들어가지 않는다.
- row 0 application 뒤에야 noun computation이 생긴다. 이 applied computation이 이후 J Graph에서 reduction structure로 분석되고 Execution Semantic Lowering에서 `Reduce(Add)`가 될 수 있다.
- runtime parser action이 실제 noun을 계산하는 것과 static analyzer가 구조를 보존하는 것은 서로 다른 execution mode지만, **같은 9-row language semantics**를 공유해야 한다.

현재 회귀 근거:

- `tests/enqueuer.rs`는 core Verb/Adverb/Conjunction/Name/assignment의 enqueue class와 lookup flag/source provenance를 검사한다.
- `tests/parser_provenance.rs::all_nine_rows_preserve_original_word_coverage_and_inherited_tokens`는 `+/`가 `ParseRow::Adverb`로 reduction되고 원 word coverage/token provenance를 유지하는지 검사한다.
- `src/parser.rs::apply_parse_row`는 row 3 construction과 row 0 monadic application을 분리하고 reduction 뒤 stack rescan을 수행한다.
- `tests/analysis.rs`의 `+/1 2` 분석 회귀는 derived semantic head가 `PrimitiveAdverb(Insert)`로 남은 채 A3까지 내려가는 것을 검사한다.

이 trace는 frontend 전체 conformance를 대표하는 **orientation example**이지, locative/direct definition/gerund/value-dependent constructor까지 모두 지원된다는 증거가 아니다. 그 범위는 A0.5/F0–F2/P0–P8 checklist와 differential report가 결정한다.

#### 3.3.2.3 Compiler/interpreter/JIT 공통 structured diagnostics

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

과거 `reduce_modifier_applications -> collapse_verb_trains` staged helper는 최종 semantic model이 아니었다. 현재 선언 row matcher는 통합되어 있으나 runtime semantic actions와 전체 constructor/POS coverage는 아직 미완료다. 장기적으로는 jsource의 9-row parse behavior를 하나의 parser reduction engine에서 재현하고 differential tests로 검증한다.

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

현재 모듈 책임은 [§10 M0 ownership 표](#architecture-migration-checklist), 구현 상태는 [§12](#current-implementation-status)에 모아 둔다. compiler의 연결된 분석 경로는 `FunctionEntity → j_graph_ir::Plan → analysis::lower_graph → logical_ir::Plan`이다. `analysis.rs`는 FunctionEntity를 직접 canonicalize하는 분석기가 아니라 J Graph의 execution-semantic lowering을 담당한다.

현재 `semantic.rs`는 `/` ADV와 `"` CONJ의 identity/POS 및 지원 subset의 parser construction graph를 보존한다. 전체 jsource-compatible Enqueue/9-row cutover는 M2에서 추적한다. `Callable.reduce/rank`와 runtime `ResolvedVerb`의 flattening은 남은 migration 표현이며 canonical function identity가 아니다.

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

위 흐름은 **J semantic target**이다. 현재 runtime definition subset은 `LocalFrame`의 current-frame lookup 후 global namespace fallback까지 구현되어 있고, 일반 user locale/path 해석은 아직 미완료다. compiler-side `execution_semantics::Scope`도 `LocalFrame(u32)` vocabulary를 갖지만 현재 A3 symbol emission은 주로 `CurrentGlobal`에 머문다. 따라서 목표 locale semantics와 현재 implementation coverage를 같은 것으로 읽지 않는다.

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

#### 3.7.1 Explicit-definition control-flow handoff example

control-flow handoff의 orientation 표본으로 다음 monadic explicit definition을 사용한다.

```j
f =: 3 : 'if. y do. 1 else. 0 end.'
```

이 예제의 목적은 `if.` 자체를 지금 compile한다고 주장하는 것이 아니라, **현재 frontend가 소유하는 정보와 future A3 CFG가 소유해야 할 정보를 분리하는 것**이다.

현재 경계:

~~~text
source
  f =: 3 : 'if. y do. 1 else. 0 end.'
      │
      ▼
[현재 구현] DefinitionInput / DefinitionCode
  source/body
  definition mode + monad/dyad body ranges
  sentence/source provenance
  monad_controls:
    If / Do / Else / End ...
  previous-result/control-flow metadata
      │
      ▼
[현재 구현] FunctionEntity
  head = ExplicitDefinition(DefinitionCode)
  POS / source / construction identity
      │
      ▼
[현재 runtime subset] per-call LocalFrame / straight-line invocation
  supported mode 1/2
  fresh LocalFrame per call
  x/y/u/v/m/n binding
  local-first → global lookup
  local/global assignment + cleanup
  control node가 Body 이외이면 Unsupported
      │
      ───────── compiler body-graph / CFG lowering stop line ─────────
      │
      ▼
[planned] definition-body semantic graph / CFG construction
  entry: evaluate condition sentence under J semantics
          ├─ true  → then body
          └─ false → else body
  merge: preserve J previous-result / return semantics
  local/name reads and writes remain explicit effects/resources where needed
      │
      ▼
[planned A3 control-flow lowering]
  Region
    Block(entry)
      ... condition ops ...
      CondBranch(...)      // future terminator, not current A3-v0 API
    Block(then)
      ... noun/result ...
      Branch(merge)
    Block(else)
      ... noun/result ...
      Branch(merge)
    Block(merge)
      block argument / phi-like selected result
      Return(result)
~~~

현재 `logical_ir::Plan`에 `Function → Region → Block` container는 이미 있지만 **A3-v0의 `Terminator`는 현재 `Return`만 가진다.** 따라서 Region/Block 타입이 존재한다는 사실을 `if./while./try.` CFG lowering이 구현되었다는 뜻으로 해석하지 않는다. `Branch`, `CondBranch`, block argument/phi-like merge는 위 그림에서 **planned concept**이다.

J Graph IR과 CFG도 같은 것으로 취급하지 않는다.

- 각 basic block 안의 analyzable array expression은 J semantic construction에서 **J Graph IR applied computation**으로 보존·분석할 수 있다.
- `if./while./try.`의 control edge와 invocation frame/namespace/effect semantics는 **definition control-flow / A3 Region-Block** 책임이다.
- 따라서 J Graph IR을 억지로 generic CFG로 확장하거나, 반대로 A3 CFG가 J combinator provenance를 지워서는 안 된다.

future CFG lowering이 추가될 때 필요한 최소 증명:

1. `DefinitionCode.monad_controls/dyad_controls`와 source/control ranges에서 branch/loop edge를 결정적으로 재구성한다.
2. 현재 runtime `LocalFrame`의 per-call 독립성, x/y/u/v/m/n binding, local-first lookup 의미를 compiler CFG에서도 보존하고, 향후 locale/path 지원이 추가될 때 같은 resource/effect contract로 확장한다.
3. branch merge에서 J의 previous-result/explicit `return.` 의미를 block-result/terminator로 정확히 표현한다.
4. local/public assignment와 error/throw/catch edge를 ordinary SSA value flow와 분리한다.
5. recursion은 Function body를 공유하더라도 invocation frame/state는 공유하지 않는다.
6. verifier가 illegal branch target, missing merge/result, effect-order break, source-control provenance drift를 거부한다.

현재 회귀와 runtime tests는 `DefinitionCode` construction/control metadata뿐 아니라 **지원 straight-line invocation의 LocalFrame·scope·assignment/effect 동작도 일부 검사한다.** 그러나 control-flow body를 J Graph/A3 CFG로 낮추는 planned compiler 경계의 완료 증거는 아니다.

#### 3.7.2 jsource NAME 정합성 감사 및 F1/P4 수용 조건 (2026-10-07; 설계·코드 조사)

**명세 우선순위:** 고정 `jsource@13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`의 [`sn.c::vnm/vlocnm/nfs`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sn.c), [`w.c::jtenqueue`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/w.c), [`p.c::jtparsea`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c), [`s.c::jtsyrd/jtsymbis`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/s.c), [`sl.c`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sl.c) 및 [J Dictionary의 NAME/locative/assignment 관찰 의미](https://jsoftware.com/help/dictionary/dicti.htm)를 기준으로 한다. RustJ 고유 NAME **문법·의미 제한을 추가하지 않는다.** 허용되는 차이는 내부 carrier·hash layout·lookaside·refcount·JIT cache 구현 등 외부에서 보이지 않는 방식뿐이다. 아래는 **현재 main의 정적 감사**이며 실제 differential PASS 기록이 아니다.

**NJ-03 오류 전달 경계의 제한된 구현:** `RuntimeParserHost::lookup`은 `Result<Option<ParserNameBinding>>`으로 조회 실패와 미정의 이름을 구별한다. parser는 실패의 오류 종류를 유지하고 해당 NAME span과 원래 enqueue 단어 번호를 추가하며 이후 조회·version 관측·실행을 중단한다. 오류 전에 완료된 우측 호출의 효과와 capture는 보존하며 replay하지 않는다. 성공적인 read capture를 합성하지 않는다. 일반 NAME의 flat namespace 조회는 유지하며 base noun locative는 별도 scope 선택과 오류 전달을 사용한다. 일반 locale/path/holder 해석과 완전한 resolution witness는 여전히 미구현이다. NJ-03 전체·NJ-04 및 L3 완료를 뜻하지 않는다.

NAME 감사 도구는 `--reference-root .reference`로 Linux/WSL의 `build_reference.py` 산출물을 사용할 수 있다. 빌드 manifest의 고정 commit·platform·variant와 실제 library 존재를 검사하고 binary SHA256을 기록한다. 없는 variant는 미실행으로 남으며, unsupported/mismatch를 통과로 계산하지 않는다. 기존 Windows `--assets-root` 경로도 유지한다.

일반 locative는 이제 Simple/DirectLocative/IndirectLocative/BaseLocative 어휘 메타데이터를 가진 NAME으로 enqueue된다. env=2에서 locative NAME 대입 대상의 지역 copula는 전역으로 승격되며 env=0은 특수화하지 않는다. 원본 span·queue index를 보존한다. 정적 parser, gerund lookup, catalog 선언, 반복 변수는 일반 namespace 접근 전에 locative 실행을 계속 거부한다. 런타임의 base 및 named-own noun만 아래 제한된 경계로 허용한다. computed locative와 아래 경계 밖 abandon locative는 Unsupported다. #14에서 현재 코드로 이식한 NJ-01 분류 단위이며 NJ-04 locale/path 해석이나 L3 완료가 아니다. 회귀 검사는 세 환경·두 copula, malformed/limit 우선순위와 비변경 경계를 포함한다. 고정 소스/public C stage 비교는 어휘 문법과 값 probe를 검사하며 C 내부 queue flag는 소스 기반 golden이지 exported runtime trace가 아니다.

**고정 C locale 오류 전달:** 오라클은 jsource EVLOCALE(30)을 `locale error`로 전달하여 미정의 이름의 `value error`, 잘못된 holder 형태의 `rank error`와 구분한다. 잘못된 숫자 holder와 생성되지 않은 번호 locale을 고정 C 라이브러리로 검증하며, 알 수 없는 오류 번호의 기존 fallback은 유지한다. 검증 하네스 수정이며 indirect/numbered locale 구현이나 NJ-04/L3 완료를 뜻하지 않는다.

**Indirect 명사 검증 선행 행렬:** 자동 NAME 차분에 boxed holder 재대입, named own-miss z 조회와 own 쓰기, snapshot, 실패 RHS 후 값 보존, holder 없음/숫자/비박스 오류 및 오른쪽 오류 우선순위를 추가한다. Rust 미지원 결과는 unsupported gap으로 남기며 locale/value 오류 혼동은 의미론 불일치로 분류한다. C 관찰은 내부 holder/version witness의 증거가 아니며 indirect runtime 구현과 full NJ-04/L3는 미완료다.

**전체 primitive 이름 catalog:** 고정 jsource ws.c의 공개 spellin 표기 139개와 상수 동사 계열의 _1:…_9:·__:를 포함한 149개 표기에 PrimitiveName/PrimitiveNames를 부여한다. CPLUS 같은 기호별 코드 이름과 t.c의 monad/dyad handler 이름을 별도 metadata로 기록한다. 같은 기호의 valence 이름은 구분하며 atomic 공용 handler 이름은 의미 계약으로 추론하지 않는다. 기존 core identity의 names()로 실행/recognition-only 구분 없이 접근한다. noun·copula·괄호 및 미설치 spellin 항목도 이름을 기록하지만 함수 수용을 넓히지 않는다. 숫자 C opcode·ds 객체·kernel 재사용을 복제하지 않으며 registry/IR identity·rank·effect·실행 지원은 유지한다. pinned source 표 전체 대조와 core/Vocabulary exhaustive 회귀로 검증한다. 실행용 PrimitiveId·AdverbId·ConjunctionId와 frontend, J Graph/Logical IR, 분석, runtime, 예제, 테스트 및 문서의 primitive 참조도 동일한 기호 이름을 사용한다(Plus, Dollar, Slash, Qq, Atco 등). 의미별 별칭 Add/Shape/Insert/Rank/Atop은 primitive 식별자에서 제거한다. valence별 함수 이름과 논리 연산·커널 이름은 각 계층의 의미를 유지한다. 공개 Rust enum/API 및 Debug 이름은 변경되지만 표기, enum 순서, 의미론과 registry version 10은 유지한다. 모든 실행 identity 이름과 카탈로그 이름의 일치 및 lexer→semantic→Logical IR 전파를 회귀 검증한다.

**현재 rank 63 표현:** 현재 PrimitiveId::innate_ranks와 rank 정규화는 jsource 호환 정수 표현 63을 유지한다. 고정 jtype.h의 RANKT는 8비트이며 플래그 2비트를 제외한 6비트의 최대값 RMAX = 63이다. 이는 실제 배열 rank의 허용 최대값이기도 하므로 실제 rank 63은 63차원을 뜻한다. 동사의 innate rank 63은 모든 허용 인수를 전체 cell로 받는 사실상 무한 rank 역할을 하며, 적용 cell rank는 인수 rank를 넘지 않는다(인수 rank 3이면 cell rank 3). 배열 rank와 동사 rank의 역할을 혼동하지 않는다. 명시적 Infinite 표현은 장기 IR 설계이며 이번 변경에서 도입하지 않는다. [jsource 정의](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/jtype.h#L68-L79)

**Indirect holder 오류 조회 경계:** 단일 일반 holder 이름의 indirect 읽기는 parser-stack 조회 시 local → base → z 명사 holder를 확인한다. 미정의 holder는 value error, 비원자 holder는 rank error, 현재 생성 불가능한 비음수 정수 번호 locale는 locale error를 반환한다. 오른쪽 완료 효과와 최초 오류 순서를 보존하며 실패 조회에 flat binding witness를 만들지 않는다. scalar 비박스 타입은 domain error이며 비음수 scalar Boolean/boxed 정수·Boolean 번호 locale도 locale error로 구분한다. scalar box 내부를 rank → 비어 있음 → literal 타입 → locale 이름 순서로 검증하여 각각 rank/length/domain/ill-formed name 오류를 보존한다. 오류 조회는 오른쪽 완료 효과를 유지하고 왼쪽 효과를 실행하지 않으며 성공 binding witness를 만들지 않는다. scalar 정수 또는 boxed scalar 정수의 음수 debug 주소도 정지된 debugger call frame이 없는 현재 runtime에서는 locale error를 반환한다. 실행 중인 일반 local frame을 debug frame으로 대체하지 않으며 실패 후 frame 복원·전역 holder 보존을 검증한다. 실제 debugger/suspension·복합 holder·함수·static/compiler는 미지원 유지; 아래 제한된 명사 대입·abandon은 별도 경계다. 아래의 제한된 정상 boxed holder 읽기 외 일반 indirect witness와 full NJ-04/L3는 미완료다.

**Boxed holder base/named 명사 읽기 경계:** 단일 일반 holder가 scalar box 안의 rank 0/1 문자 named locale를 제공하면 해당 locale own table과 기본 z 경로에서 명사를 읽는다(번호 locale 제외). boxed base holder는 Engine base table을 공유하며 own hit는 Global(engine), z hit는 Locale(z)로 구별한다. 대상 조회는 호출 local을 우회하지만 holder 선택은 local 우선이며 named↔base 재대입·snapshot과 wrong Global/Locale witness 음성 검사를 포함한다. local → base → z holder 선택, 조회 시 holder 재대입 반영과 배열 snapshot을 검증한다. capture는 시작/발견 locale, holder version/generation 및 읽은 명사의 version/generation을 분리하며 구조 verifier의 음성 검사를 포함한다. 이 관찰은 indirect SimpleNameGuard/LocativeNounGuard 또는 compiler/hoisting/replay proof를 허용하지 않는다. 함수 호출·복합 holder·일반 locale path는 후속 계약이다. 단일 boxed holder 명사 abandon은 아래 별도 경계에서 지원하며 full NJ-04/L3는 미완료다. indirect noun 읽기와 abandon의 capture 검증은 존재하는 holder binding을 가리킬 수 없는 버전 또는 generation 0을 거부한다.

**단일 boxed holder 명사 abandon 경계:** `name__holder_:`는 stack 조회 시 local → base → z holder를 선택하고 boxed base/named 시작 locale의 own → 기본 z 경로에서 찾은 명사를 반환한 뒤 발견 table에서만 삭제한다. locative 대상은 explicit local table을 우회한다. 새 named locale 준비는 삭제 전 관찰보다 먼저 수행하여, 처음 z hit를 삭제해도 시작/발견 scope·holder와 noun version/generation을 캡처에 보존한다. holder 자체 삭제도 삭제 전 증거를 유지한다. 고정 p.c의 undefname 분기에 따라 abandon의 holder 조회 실패(rank/domain/length/잘못된 이름/부재 numeric locale)는 value error로 보고하며, 일반 읽기·대입의 상세 오류는 유지한다. 실패는 base/named/z binding과 버전을 삭제하거나 변경하지 않는다. RHS 최초 오류는 삭제를 방지하지만 abandon 완료 후 왼쪽 오류는 삭제를 되돌리지 않는다. 배열 snapshot, own 삭제 후 z fallback, z 삭제 후 ordinary value error, local holder 우선과 event/NameUse 변조 음성 검증을 고정 C 및 두 실행 경로로 확인한다. indirect guard·J Graph/compiler 수용은 계속 거부하며 함수·future reference·복합/번호/debug holder·일반 path/full L3는 미완료다.

**명사 인수 선택 동사 `[`, `]` runtime 경계:** `t.c`의 CLEFT/CRIGHT와 `v.c::jtright1/jtleft2/jtright2`에 따라 두 동사의 monad는 y, dyad `[`는 x, dyad `]`는 y를 그대로 반환한다. innate rank는 모두 RMAX(63)이며 operand는 공유 parser가 오른쪽부터 모두 평가한 뒤 선택한다. 무시되는 operand의 오류·효과를 생략하지 않는다. 수치 변환/shape agreement를 적용하지 않고 선택된 명사의 dtype·shape·boxed/sparse/extended 값과 snapshot을 보존한다. 기존 VocabularyPrimitive recognition에서 PrimitiveId::Left/Right로 실행 identity를 이관하고 registry version을 갱신한다. runtime noun 실행을 검증했으며 compiler alias/evaluation proof 계약은 unknown으로 유지한다. 함수 이름 이관·최적화 수용·일반 rank prototype 및 full L3 완료를 뜻하지 않는다. 고정 C 차분과 default/portable·semantic-reference 회귀는 noun 종류·RHS holder 변경·양 operand 오류 순서를 검증한다.

**단일 boxed holder 명사 대입 경계:** RHS 평가 완료 후 holder를 local → base → z에서 선택하고 boxed base/named locale의 own table에 명사를 기록한다. z에서 읽은 버전은 대입 previous version이 아니다. `=.`/`=:` 모두 explicit local target을 우회한다. RHS 실패는 target locale/binding을 만들거나 바꾸지 않으며, RHS holder 재대입은 완료 시점 대상에 반영한다. commit이 holder 자체를 바꾸어도 캡처 버전은 commit 직전 대상 이름을 고정하여 기록한다. 읽기와 대입은 동일 holder 오류 검증을 공유한다. pinned C 차분은 named/base/z 보존, local bypass, RHS 오류 우선순위와 자기 holder 재대입을 검증한다. 이 bounded runtime/capture 구현은 indirect compiler/guard/Graph 증명이 아니며 함수·복합 holder·computed·일반 path와 full NJ-04/L3는 미완료다. 단일 boxed holder 명사 abandon은 위 별도 경계에서 지원한다.

**NJ-04 base 명사 경계:** 런타임 `a__` 조회·명사 대입은 Engine의 base namespace를 명시적으로 선택하고 호출 지역 table을 우회한다. `a__`라는 별도 flat key를 만들지 않으며 기존 base `a`의 generation/version과 동일한 저장소를 사용한다. noun은 parser stack 진입 시 snapshot하고 대입·오류·효과 순서는 공유 parser로 보존한다. `BaseLocaleOnly`/`Bypassed` 관찰은 실제 검색과 lexical form을 verifier에서 대조한다. 일반 simple-name guard로 승격하지 않으며 locative capture의 J Graph 변환도 namespace dependency 증명 전까지 거부한다. 함수 locative, 미정의 future reference, computed/abandon locative, numbered/indirect locale 및 path·실행 locale 전환은 Unsupported로 남긴다. named locale 자체 table의 명사는 아래 별도 단위로 지원한다. 회귀·고정 C 차분에는 base 공유/배열 snapshot, 지역 shadow 우회·대입, 우측부터의 조회와 실패 대입의 상태 보존을 포함한다. NJ-04 전체·L3는 계속 미완료다.

**NJ-04 named-own 명사 경계:** `a_probe_` 같은 직접 locative의 simple name과 locale spelling을 분리하고 `Engine.named_locales`의 독립 table·ScopeInstanceId로 조회/대입한다. 명사 lookup 때 locale이 없으면 생성하며, RHS 실패 전에 대상 locale을 미리 만들지 않는다. `a_base_`는 기존 base binding을 사용한다. `DirectLocaleOnly(start)`와 실제 `FoundScope::Locale(hit)`을 구분하여 verifier가 일치·POS·local 우회를 검사한다. 각 locale의 version이 같더라도 scope/generation을 혼동하지 않는다. named 함수/미정의 future reference, numbered locale, 임의 path 검색, path 변경/indirect/호출 locale 전환, computed 및 아래 경계 밖 abandon·컴파일 수용은 별도 Unsupported 경계다. C 감사의 named/base z 명사 fixture는 해당 단위에서 검증하며 전체 path 수용으로 간주하지 않는다. 일반 NJ-04/NJ-05 및 L3는 미완료다.



**PR #14 이력 통합:** NAME lexical 분류·locative copula는 #25에서 현재 frontend에 이식되고, 오류 전달(#22/#23)·base/named-own noun(#26/#27)·대입 버전(#28)이 후속 구현됐다. #14의 오래된 parser 전면 거부는 병합 시 현재 bounded runtime 지원을 보존하는 것으로 대체한다. 단순 by-value/abandon의 기존 지원도 유지한다. full locale/path/function/by-value locative 및 플랫폼 NAME 수용은 여전히 미완료다.

**NJ-04 direct own-table witness 검증 보강:** `DirectLocaleOnly`의 시작 ID와 발견 ID를 함께 변조하여 named locale을 base로, 또는 `a_base_`를 named locale으로 위장하는 capture를 거부한다. NAME의 실제 `_base_` 별칭 표기와 engine scope 선택을 대조하며 정상 base/named noun snapshot은 유지한다. 회귀는 양쪽 위장과 정상 capture를 검사한다. 이는 own-table 역할 검증이며 default z-path·일반 locale identity 매핑·full resolution witness 또는 L3 완료를 의미하지 않는다.

**NJ-04 검색 종류 witness 보강:** base/direct locative의 조회 기록을 `GlobalOnly` 또는 `CurrentFrameThenGlobal` 일반 이름 검색으로 위장하면 verifier가 거부한다. 일반 검색은 lexical Simple NAME만 허용하며 정상 base/named capture는 유지한다. 세 locative 표기의 변조 회귀로 경계를 검증한다. default z-path·완전한 namespace witness 및 L3 완료는 별도 미완료 계약이다.

**NJ-04 z own-table 선행 단위:** `a_z_` 직접 명사 조회·대입을 독립 z locale table과 ScopeInstanceId로 지원한다. 다른 named-own과 같은 snapshot·버전·local 우회·실패 RHS 보존을 사용하며 함수·미정의 참조와 컴파일 수용은 계속 거부한다. 기본 z noun 검색은 아래 단위에서 지원하며 임의 path 변경은 계속 미지원이다. 회귀·고정 C 감사는 z 자체 값, 배열 snapshot, local shadow 우회 및 실패 대입 이후 값을 검증한다. NJ-04 전체/L3 미완료.

**NJ-04 named default z 명사 경계:** named direct locative의 own table에 명사가 없으면 기본 z table에서 조회한다. `NamedDefaultZ { start, z }`와 실제 `FoundScope::Locale(z)`는 시작·발견 ID를 분리하고 local 우회·noun POS를 검증한다. read version은 발견 table을 사용하고 assignment version/commit은 지정 named own table만 사용한다. 첫 own 대입은 z 버전을 이전 값으로 가져오지 않는다. 최초 named locale 생성 전에는 ID를 가정하지 않으며 성공 lookup 후 생성된 scope 관찰을 capture에 기록한다. snapshot·실패 RHS·local shadow 우회 회귀 및 고정 C 감사가 해당 경계를 검증한다. path 변경, path 변경/epoch guard, full namespace witness와 NJ-04/L3 전체는 미완료다.

**NJ-04 explicit base default z 명사 경계:** `a__`와 `a_base_`는 base own miss 시 z noun을 읽고 `BaseDefaultZ { z }`/`FoundScope::Locale(z)`로 base 시작과 z 발견을 분리한다. local table을 우회하고 base own 대입 version/commit을 사용하므로 첫 대입 이전 버전은 없음이며 z 값은 보존한다. 두 별칭의 동일 값, snapshot, local shadow 우회, 실패 RHS 보존 및 잘못된 found ID 거부를 회귀·고정 C 감사로 검증한다. path 변경, path epoch guard·임의 path·함수/indirect/numbered와 full NJ-04/L3는 미완료다.

**NJ-04 일반 NAME default z 명사 경계:** 일반 이름은 local → base own → z noun → extension registry 순서로 조회한다. `SimpleDefaultZ { z }`/실제 found locale·generation·version을 기록하며 base/지역 대입은 own 대상 버전만 사용한다. 일반 z noun path 관찰의 제한된 guard는 아래 경계에서 지원한다. 일반 z path noun abandon은 아래 경계에서 지원한다. 일반/base/named 일관성, noun snapshot, local shadow/첫 지역 대입, 실패 RHS와 z 보존을 회귀·고정 C 감사로 검증한다. 임의 path/epoch guard·호출 locale 전환·함수/indirect/numbered와 full NJ-04/L3는 미완료다.

**NJ-04 일반 z-path noun abandon:** 일반 `a_:`는 local/base own 우선순위를 유지하고 fallback으로 찾은 z 항목을 반환·삭제한다. 단일 지역 word의 local binding 비삭제 규칙은 유지하며, 지역 miss의 z 발견은 삭제한다. snapshot 수명, 오른쪽 오류 전 삭제 없음, 삭제 후 오류에서도 삭제 유지, own-table 우선순위와 실제 발견 locale capture를 회귀·고정 C 감사로 검증한다. ordered effect/guard 및 명시적 locative abandon·임의 path는 계속 별도 미완료 계약이며 full NJ-04/L3를 완료하지 않는다.

**NJ-04 z abandon 관찰 검증 보강:** 삭제 event의 독립 pre-action lookup도 frame/search-state 계약을 검증한다. 프레임 없음은 NoFrame, 프레임 있음은 engine/z와 다른 ID 및 Absent/DeclaredUnbound여야 한다. frontend NAME 관찰을 그대로 둔 채 event frame을 engine/z로 위장하거나 local state를 바꾸는 회귀가 이전 검증 우회를 재현하며 이제 거부된다. 기존 값·삭제 의미와 guard/compiler 수용 범위는 유지한다. 완전한 namespace witness 및 NJ-04/L3 전체는 미완료다.

**NJ-01 locative abandon enqueue 선행 경계:** 적법한 `a___:`, `a_base__:`, `a_probe__:`, `a__holder_:`는 NAME 주소 형식과 by-value/abandon 정책을 독립적으로 보존한다. NAME payload는 inflection을 제거한 주소, 원본 span/word index와 lookup/abandon flag는 유지한다. malformed/limit 검증은 이름 문법 단계에 남는다. static parser와 runtime은 아래 explicit-base 및 named own noun 경계 밖 미구현 locative 삭제를 명시적으로 Unsupported로 거부하여 flat key 조회나 기존 table 삭제로 잘못 해석하지 않는다. enqueue 성공은 locale 삭제 실행 또는 NJ-01/NJ-04/L3 전체 완료가 아니다. base own noun 실행은 아래 경계에서 지원하며 z fallback 삭제는 아래 경계에서 지원한다.

**NJ-04 explicit-base own noun abandon:** `a___:`/`a_base__:`는 local을 우회하여 base own noun을 반환·삭제한다. own 함수 및 미발견 path/future 삭제는 Unsupported로 유지하며 상태를 변경하지 않는다. noun snapshot 수명, RHS 오류 전 삭제 없음, 삭제 후 오류에도 삭제 유지, 양쪽 실행 경로 local shadow 우회 및 실제 base found capture를 회귀·고정 C 감사로 검증한다. static/deferred parser와 Graph/guard 수용은 별도 미지원이며 full NJ-04/L3는 미완료다.

**NJ-04 explicit-base default z noun abandon:** base own miss의 `a___:`/`a_base__:`는 실제 발견 z noun을 반환·삭제한다. local 우회, BaseDefaultZ/실제 found locale 및 삭제 전 version/generation을 보존한다. snapshot·오류 순서·지역 shadow 우회·두 alias 일관성과 event search/found 변조 거부를 회귀·고정 C 감사로 검증한다. own 함수·미발견 path/future 삭제 및 static/Graph/guard 수용은 별도 미지원이며 full NJ-04/L3는 미완료다.

**NJ-04 named own noun abandon:** `a_probe__:`/`a_z__:` 등 direct named 주소는 local을 우회하여 해당 locale own noun을 반환·삭제한다. 다른 table과 noun snapshot을 보존하고 최초 오류·삭제 후 오류·지역 shadow 및 실제 found capture/변조 거부를 회귀·고정 C 감사로 검증한다. 미발견 path/future 삭제·함수·indirect/numbered 및 static/Graph/guard 수용은 Unsupported로 유지하며 full NJ-04/L3는 미완료다.

**NJ-04 named default z noun abandon:** direct named own miss의 `a_probe__:`는 실제 발견 z noun을 반환·삭제한다. 시작 named locale과 found z·local 우회·삭제 전 version/generation을 분리하며 첫 locale 조회·snapshot·최초 오류·삭제 후 오류와 start/found/frame 변조 거부를 회귀·고정 C 감사로 검증한다. 임의 path/future·함수·indirect/numbered 및 static/Graph/guard 수용은 미지원이며 full NJ-04/L3는 미완료다.

**NJ-04 일반 z noun guard:** 일반 NAME의 SimpleDefaultZ noun hit도 제한된 SimpleNameGuard를 만들 수 있다. 검사 때 전체 lookup observation을 재확인하여 base shadow·z 재대입·삭제·동일 값 재생성 ABA·다른 engine을 거부하며 무관한 z 이름 대입은 허용한다. 이는 검사 시점의 사실이며 read hoisting·effect replay·컴파일 실행 허용이 아니다. abandon/function path guard와 임의 path epoch 및 full NJ-04/L3는 미완료다.

**NJ-04 locative noun guard:** 검증된 runtime base/direct named own 및 기본 z noun 관찰에서 별도 LocativeNounGuard를 만든다. 전체 lookup 재검사로 own shadow·재대입·삭제 후 동일 값 재생성 ABA·engine/frame 변경을 거부하며 무관한 이름 대입은 보존한다. SimpleNameGuard의 일반 이름 경계는 확장하지 않고 abandon/function/indirect/numbered·임의 path와 compiler/hoisting/effect replay 수용은 계속 미지원이다. 이는 check-time 사실이며 full NJ-04/L3 완료가 아니다.

**NJ-07 대입 버전 경계:** parser commit의 이전/새 버전은 `assignment_version(name, local)`로 실제 대입 대상 테이블에서 관찰한다. 지역 선언이 아직 없을 때 RHS가 전역 값을 읽더라도 지역 첫 commit의 이전 버전은 없음이며, 다음 지역 commit은 지역 버전을 사용한다. base/named-own locative는 지역 flag와 무관하게 해당 own table을 사용한다. 읽기용 `version`은 기존 조회 의미를 유지한다. 실패한 RHS는 commit을 만들지 않는다. 이 경계는 default z path의 읽기 발견 테이블과 쓰기 대상 테이블을 분리하기 위한 선행 단위이며, path 지원·완전한 namespace commit witness·L3 완료를 뜻하지 않는다. 회귀는 지역 첫/반복 commit, 전역 값 보존, 전역 첫 commit과 RHS 오류를 검사한다.

| ID | jsource 관찰 계약 | RustJ 현재 구조/차이 | 수용 조건 |
|---|---|---|---|
| **NJ-01 NAME 인식** | 단순 이름(내부 `_` 허용), direct `name_locale_`, indirect `name__holder`, base alias `name__`; 적법한 `name_:`는 `NAMEBYVALUE/NAMEABANDON` 별도 inflection. `nfs` 길이/오류 precedence, `vnm` 문법 유지 | 일반 locative 어휘 분류와 단순 by-value/abandon 경계는 구현됨. locative by-value/abandon 및 32-bit numeric locale 제한 등 플랫폼 경계 미검증 | 모든 jsource-valid NAME을 정상 parse queue에 전달하고 invalid/limit/spelling만 해당 J 오류로 구분. `Unsupported`를 최종 NAME 구문 결과로 인정하지 않음 |
| **NJ-02 enqueue 환경** | `jtenqueue env=0/1/2`, NAME to-lookup/assignment flags, copula 분류. locative 대상 `=.`는 explicit body에서도 **global** | env=0/1/2 일반·locative copula 분류와 locative global 승격은 소스 golden/public C로 검증됨. locative by-value flag와 완전한 내부 trace는 미검증 | 원본 queue 분류를 C 소스 유도 golden과 실행 가능한 J oracle에서 검증 |
| **NJ-03 조회 결과/오류 API** | `syrd`는 **미정의 이름**과 **locale 해석 실패**를 구분하고, 실제 lookup에는 local 여부·찾은 locale·이름의 POS·동적 error가 개입 | `RuntimeParserHost::lookup(&str)->Result<Option<ParserNameBinding>>`은 오류/성공/미정의를 구별한다. base 및 named-own noun의 명시 scope 선택·관찰을 제한적으로 구현; 일반 locale/path/holder 해석과 witness는 미구현. `binding_version(name)`도 전역 flat key 중심 | 최소 `Result<Option<...>>`에 해당하는 **오류 있는 lookup 경계**와 actual resolution witness(local/locale/path/context/version)를 분리하여 보존. 구체 타입 이름은 구현 때 선택 |
| **NJ-04 실제 namespace** | 일반 이름: invocation local → 현재 locale → **그 locale의 명시 path** (path의 path는 재귀 탐색하지 않음). Direct는 지정 locale부터, indirect는 holder의 현재 값으로 시작 locale 결정. 숫자 locale·locale 생성/변경/삭제·path 영향 | `Engine.names`의 base 명사 조회·대입과 마지막 `LocalFrame`을 우회하는 `a__` runtime 경계만 구현. named locale별 own-noun table은 별도 단위로 구현. 기본 z noun path는 아래 단위로 구현. 임의 path·검색 path 변경·current locale 상태/전환은 없음 | namespace를 locale-scoped로 구현하고 local, current, path, direct/indirect, name shadowing을 같은 의미·오류 순서로 수행 |
| **NJ-05 locative 호출 문맥** | `f_locale_` 호출 시 global name 실행의 **current locale**가 시작 locale로 전환되고 복원됨. 값이 경로상의 다른 locale에서 발견돼도 **검색한 locale ≠ 실행 locale** 가능 | Function `NameRef(String)`와 runtime 호출이 locale context·search hit locale·restore obligation을 별도로 모델링하지 못함 | lookup 시작 locale, 실제 발견 locale, 호출 실행 locale, 복원 이벤트를 구분. caller의 local frame이 callee로 잘못 전파되지 않게 함 |
| **NJ-06 값 vs NameRef** | noun은 stack 진입 시 값으로 snapshot, 일반 Verb/Adverb/Conjunction은 당시 POS에 따른 nameref와 호출 시 late lookup; locative 함수는 호출 locale context 필요. `name_:`는 by-value 후 binding abandon/delete. undefined 일반 NAME의 파서 fallback 경로는 즉시 오류와 동일하지 않음 | noun snapshot, Function `NameRef`, 일부 POS mismatch/domain은 존재. 하지만 `name_:` 삭제·full modifier/locative NameRef·원본 undefined 경로 미완료 | 재바인딩/품사 변경/특수 by-value-name과 실제 NameRef/실행 context에 대한 차분 수용 |
| **NJ-07 대입 대상** | local `=.`, global `=:`; locative는 모두 global. 중간·연속 대입, **계산된 단일 NAME / 여러 NAME**(문자열·boxed list), locally-defined name에 대한 public collision의 J Domain 오류 | parser `PendingAssignment`와 `assign_scoped(&str,..)`가 주로 단일 raw 이름을 전제로 함; 다중/계산된 name·locale table 선택 일반화 미완료 | RHS 평가 시점/결과 JEntity, name 계산, 해당 table commit, 첫 오류 및 side-effect ordering과 rollback 경계 대조 |
| **NJ-08 전체 문장 효과** | `p.c` stack/reduction에 따른 right-to-left lookup, assignment/locale mutation, lookup 후 함수 호출의 dynamic context. `18!:` locale/path 변동은 재조회·guard에 영향 | 일부 right-to-left runtime action·local frame은 구현. compiler proof는 단일 name/version으로 current locale/path 변경을 증명하지 못함 | 전체 문장 선행 snapshot/잘못된 함수 eager capture 금지. namespace-version뿐 아니라 관련 locale/path·local shadow·holder 재바인딩 witness를 필요한 시점에 재검증 |
| **NJ-09 compiler target** | 사용자 J locale이 언어 의미의 namespace | `CompilationTargetLocale`은 설계상 **별도 compiler namespace**로 이미 분리되어 있어 문제가 아님 | compiler target locale을 J language locale 조회·대입으로 오인하거나 두 binding table을 공유하지 않음 |

**구현 선행순서:** (1) NJ-01/02 NAME 블록과 enqueue의 원본 동등성 → (2) NJ-03/04 **오류를 전파할 수 있는 lookup·locale namespace** → (3) NJ-05/06 locative 호출·by-value/abandon·NameRef → (4) NJ-07/08 대입/오류/재바인딩/평가 순서 → (5) P6 C `j64`/`j64avx2`와 Rust reference의 독립 차분. F1/P4를 문법 인식만으로 완료 처리하지 않는다. unsupported는 **현재 구현 미수용**만 나타내며 J NAME feature를 제외하는 설계 선택이 아니다.

**부정·긍정 수용 시험 등록부:** `NJ-V01` simple/internal underscore·valid direct/indirect/base alias·`name_:` vs invalid/limit/spelling; `NJ-V02` env=0/1/2+locative `=.` 플래그; `NJ-V03` local shadow/unbound fallback/locale path 한 단계와 missing/invalid locale error; `NJ-V04` indirect holder의 값·재바인딩·오류, 숫자/소멸 locale; `NJ-V05` locative function의 실행 current locale, 경로에서 발견된 function, recursive/frame restoration; `NJ-V06` noun snapshot vs late function NameRef·POS 변화 및 `name_:`의 삭제; `NJ-V07` 계산된/복수 assignment·local/global 충돌·first J error/effect 순서. 기록: `source revision | 실제 C binary hash/variant | Rust commit | positive + one-mutated-negative | value/POS/locale/path/effect/error | pass/fail/unrun`. 현재 **모두 미수용**이며 기존 소규모 enqueue 테스트의 통과를 full name compliance로 승격하지 않는다.

#### 3.7.3 NAME 의미론적 조회와 최적화 provenance 분리 (2026-10-07; 설계 전용)

**결정:** `jsource`의 NAME 조회 **시점**은 그대로 두면서, NAME이 값으로 치환되더라도 **원본 NAME occurrence와 조회 사건·증거**를 J Graph/A3 및 최적화기에 남긴다. Compiler가 편하다는 이유로 noun을 늦게 조회하거나 verb를 미리 실행 함수로 확정하지 않는다. 새 실행용 NAME IR을 강제하지 않고 기존 `EnqueuedWord`, `ParseCapture`, `FunctionHead::NameRef`, J Graph `ReadNoun`, A3 `ReadNoun/VerbReference`를 우선 확장한다. 여기의 명칭은 **제안된 개념 계약**이지 구현된 Rust 타입이 아니다.

**jsource 기준:** 고정 [w.c `jtenqueue`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/w.c), [p.c `jtparsea`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c), [s.c `jtsyrd/jtsymbis`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/s.c), [sn.c `vnm/nfs`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sn.c). `p.c`는 NAME을 parser stack으로 들여올 때 **noun이면 그 순간의 값으로 치환**, 일반 verb라면 **당시 POS로 parser row를 결정하되 `nameref`를 보존하고 실제 호출에서 이름을 재조회**한다. Adverb/Conjunction과 gerund/fork는 생성 시점의 실제 modifier/operand 조회가 있으므로 “모든 function은 late”로 단순화하지 않는다. Direct/indirect locative, `name_:` by-value/abandon, `mnuvxy` 역시 별개의 J semantics다.

| source 역할 | J 실행이 사용하는 값 | 최적화기에 계속 남길 정보 |
|---|---|---|
| **Noun NAME** | 스택 진입 **시점**에 확보한 noun snapshot. 뒤의 재대입은 이 값에 영향 없음 | NAME spelling·syntax form·word/span·발생 ID, 실제 parser read 지점, local/locale/path resolution과 binding identity, 해당 snapshot의 dtype/Shape/rank |
| **일반 Verb NAME** | parser-time POS에 맞는 NameRef; **call-time** 실제 callable 조회·POS 오류 | 원래 NAME, 기대 POS, 관찰된 primitive/innate-rank **후보 힌트**와 재조회 의무. 미증명 함수 고정 금지 |
| **Modifier/gerund/fork NAME** | 각 constructor가 **생성 시점 실제로 읽거나 고정한** operand/function, 또는 별도 late ref | 생성 시점 snapshot 이벤트와 호출 시점 NameRef의 provenance·불변 범위 구분 |
| **Locative/특수 NAME** | direct/indirect locale 조회, 실행 locale 전환·복원, `name_:` abandon 효과 | 원 표기·holder read, local shadow, 검색 시작/실제 발견/함수 실행 locale, path·version·generation·원본 오류 순서 |

**독립 identity(한 sidecar/기존 IR에 구현해도 됨):**

~~~text
NameOccurrence {
  id, original_spelling, address_form: Simple|Direct|Indirect,
  read_policy: Ordinary|ByValue|ByValueAndAbandon, special_operand?,
  enqueue_word_index, source_span, parser_use_site
}
NameResolutionEvent {
  occurrence_id,
  time: StackNounRead|StackPOS|ConstructorRead|CallLookup|
        AssignmentTarget|LocaleRead|Abandon,
  outcome: NounSnapshot(value_id)|FunctionRef(expected_pos)|
           ConstructorCaptured(entity)|MissingNameRef|WriteResult|JError,
  lookup_witness?: {call_frame, local_present_or_absent,
                   start_locale, search_path_order, found_locale, execution_locale,
                   symbol_generation, binding_version, actual_pos,
                   indirect_holder_reads, locale_path_epochs},
  effect/error_order_edges, source_diagnostics
}
OptimizationNameEvidence {
  event_id, observed_pos?, candidate_callable_family?,
  observed_dtype_shape_rank?, constructor_fixed_facts?,
  status: Observed|ProvenStable|GuardRequired|Unknown,
  proof_or_guard_footprint?
}
~~~

**절대 동일시하지 않을 것:** source `NameOccurrence` ≠ 실제 `ResolutionEvent` ≠ `JEntity/SSA ValueId` ≠ 분석용 `OptimizationNameEvidence` ≠ 물리 `BufferId`. 같은 문자열 NAME이라도 occurrence, frame, locale, path, local 유무에 따라 다른 심볼이다. `NameVersion` 하나로 **조회에서 local이 없었다는 부정 증거**, path 무변화, indirect holder 재바인딩, locale 삭제·재생성을 증명할 수 없다.

**실행과 재사용의 시간 차이 예:**

~~~text
a =: 1 2 3
... a ...    J stack-read(a) -> noun v0
             capture event#A(name="a", resolved_binding, value=v0)
             J Graph ReadNoun(a,event#A) -> A3 SSA v0 + source link
a =: 9       이미 읽은 v0는 불변; 이후 읽기는 새로운 event.

f =: +
... f ...    parser POS=Verb, NameRef("f"), expected_pos=Verb
             optimization: observed Add-family HINT (not a frozen target)
f =: *
... f ...    새 호출은 현재 f를 읽거나 call-time guard로 적법성을 입증해야 함
~~~

이 예의 noun snapshot은 **실제 J parser read 시점**의 값이지 compiler가 사전 분석 시 얻은 값을 모든 미래 실행에서 캐시하라는 뜻이 아니다. 실행을 재사용하는 경우 매 invocation에서 올바른 read boundary의 값을 공급하거나 안정성 proof/guard 및 효과·오류 순서 증거가 필요하다. 반대로 이미 read를 수행한 뒤에는 같은 event를 재조회해서도 안 된다.

**Stage별 소유권과 최소 구현 변경:**

1. **F1 Enqueuer:** NAME의 문법/locative/by-value 분류와 source identity만 유지한다. lexical 단계에서 POS·binding·primitive target을 고정하지 않는다.
2. **P4 runtime parser:** `resolve_stack_item`의 noun value snapshot은 기존과 같은 타이밍으로 실행하되 origin event를 기록한다. 함수는 POS 검사와 미래 NameRef를 분리. 현재 `RuntimeParserHost::lookup(&str)->Result<Option<ParserNameBinding>>`은 **오류 전달 조회** 경계를 제공한다. 실제 locale 해석과 witness 연결은 후속 구현이다. `start_locale`·`found_locale`·`execution_locale`는 별개.
3. **Semantic/JEntity:** `NameUse`·`BoundProgram`·`ModifierSnapshot`·`NameRankSnapshot`의 관찰값을 guard와 혼동하지 않는다. frozen constructor operand는 별도 event이고 일반 NameRef는 late.
4. **J Graph:** `NodeKind::ReadNoun{name,version}`와 `VerbValue/Apply(NameRef)`에 event/provenance 링크를 유지. `GraphAnalyzability::RequiresSpecialization`은 미증명 dynamic function에서 유지. 값만 남기고 원본 name을 지우지 않는다.
5. **A3:** `OpKind::ReadNoun{symbol,version}`, `VerbReference(Callable)`, `Write`, `SemanticCheck`, `order_after`와 J Graph/source event의 연결을 검증. `Symbol{name,scope=CurrentGlobal|LocalFrame}`은 아직 locale/path 전체를 표현하지 못한다. namespace semantic effects를 단순 SSA 값·literal로 지우지 않는다.
6. **M3+ optimizer:** known POS/primitive/Shape/rank는 *transform candidate*만 부여한다. `ProvenStable` 및 `GuardRequired`의 proof에는 actual use 시점까지 local absence/shadow, current/search locale/path, indirect holder, binding versions, statement 내 namespace writes, first error/effect order를 포함. guard miss는 J 오류가 아니며 effect 이후 문장 전체 replay 금지.

**프레임워크 비교:**

| 참조 | 빌릴 부분 | 빌리면 안 되는 부분 |
|---|---|---|
| [MLIR SymbolRef/SymbolTable](https://mlir.llvm.org/docs/SymbolsAndSymbolTables/) | symbolic NAME uses와 SSA value 흐름 분리 | 정적 MLIR symbol table을 J의 동적 locale/path로 대체 |
| [LLVM MemorySSA](https://llvm.org/docs/MemorySSA.html) | read/write clobber, hoist legality와 순서 의존성 | J NameRef/POS/locale/first error를 일반 메모리 load 취급 |
| [Truffle Assumption](https://www.graalvm.org/truffle/javadoc/com/oracle/truffle/api/Assumption.html) | speculate한 바인딩·함수의 무효화 가능 guard | 단일 전역 boolean/flat version으로 path·holder 무변화를 주장 |
| [JAX captured constants](https://docs.jax.dev/en/latest/internals/constants.html) 및 [JIT global caveat](https://docs.jax.dev/en/latest/notebooks/Common_Gotchas_in_JAX.html) | captured value와 runtime input 분리 | JAX의 trace-time global 고정을 J의 dynamic name semantics에 이식 |

**수용 단계(구현/검증은 모두 UNRUN):**

- [ ] **NP-01:** NAME occurrence ID·form·span/word·중복 출현 원본 보존. **NP-V01:** 다른 occurrence/locale를 문자열만으로 병합하면 실패.
- [ ] **NP-02:** noun stack-read, function parser POS/call NameRef, modifier constructor snapshot을 정확한 시점에 분리. **NP-V02:** noun 재조회 지연이나 verb 조기 freeze 거부.
- [ ] **NP-03:** 오류 있는 locale-aware lookup, local presence/**absence**·path/holder witness·실행 locale. **NP-V03:** shadow/path/holder 바뀌었는데 cached binding 재사용 거부.
- [ ] **NP-04:** Parser capture→Semantic→J Graph `ReadNoun/NameRef`→A3 `ReadNoun/VerbReference` source event 연결. **NP-V04:** noun literal fold로 NAME provenance 소실하면 거부.
- [ ] **NP-05:** `Observed/ProvenStable/GuardRequired/Unknown` 구분과 guard footprint/first error·effect-order 증명. **NP-V05:** `Observed` 단독으로 Add kernel 특수화 거부.
- [ ] **NP-06:** locative NameRef, `name_:`, gerund/fork/Rank constructor, assignment·rebinding·POS 변화의 조회/고정 시점 보존. **NP-V06:** 조기/후기 조회 혼동 및 error 순서 변경 거부.
- [ ] **NP-07:** pinned J C `j64/j64avx2` ↔ Rust semantic reference·event ↔ Graph/A3·실제 route의 차분. **NP-V07:** J 값·POS·첫 오류·namespace effect 불일치를 실패로 판정.

등록 양식: `source revision | NAME occurrence/event→SSA IDs | oracle/binary | positive + one-invariant-negative | proof/guard owner | PASS/FAIL/UNRUN | CI SHA`. **문서 설계 완료가 F1/P4/P5/M3 구현 완료를 뜻하지 않는다.**

#### 3.7.4 NAME 최적화 캐시와 멀티프로세서 인계 계약 (2026-10-07; 설계 전용)

§3.7.3과 **NP-01~07이 유일한 수용 체크리스트**다. 이 절은 최신 NAME 감사(`cd6ac8c`)에 실행 수명·동시성·캐시 계약을 보강하며 새로운 NAME 구현이 완료됐다는 뜻이 아니다. 이전 채팅의 인계는 저장소의 M3→M4 계약을 기준으로 이어간다. 첫 구현은 단일 CPU로 닫고, 동일 계약을 이후 worker/device 실행으로 확장한다.

**Identity와 저장 비용(NP-01/03/04):** Engine 소유 interner의 `NameId`는 spelling 저장·해시 비용만 줄인다. 전역 의미 identity가 아니다. 제안 `BindingStamp = (EngineIdentity, ScopeInstanceIdentity, SymbolGeneration, Revision)`은 frame 재사용, expunge 후 같은 이름 재생성, locale 삭제·재생성에 따른 ABA를 막는다. 현재 per-name `NameVersion`을 이 stamp 전체로 간주하지 않는다. 요청한 NAME/검색 시작점과 실제 발견된 binding key도 구분한다. occurrence ID는 span과 expanded-word 위치를 포함하며 같은 spelling의 중복 출현을 병합하지 않는다. Provenance sidecar는 ID·요약·증거만 보유한다. 모든 중간 noun을 `Arc<Value>`로 붙잡으면 liveness와 버퍼 재사용이 훼손되므로 payload 수명은 실제 dataflow와 실행 lease가 소유한다.

**Guard에서 실행으로의 인계(NP-03/05):** guard 성공 후 포인터만 가져오는 방식은 검사와 사용 사이 재바인딩을 허용한다. 의미론적 lookup 지점에서 namespace owner가 lookup/POS/guard 검증과 immutable entity/value lease 확보를 하나의 일관된 작업으로 수행한다. 최초 단일 owner에서는 순서화된 실행으로, 향후 공유 namespace에서는 lock 또는 검증된 snapshot protocol로 구현할 수 있다. 이 lease는 **이미 수행한 read/call lookup 하나**를 보호하며 함수 본문의 다른 동적 NAME까지 고정하지 않는다. Guard footprint에는 local absence, path epoch, indirect holder와 실행 locale가 포함된다. Unknown namespace effect/foreign call은 증거 갱신 또는 분석 장벽이며, guard miss 후 이미 일어난 효과를 문장 전체 replay로 반복하지 않는다.

**최적화에 사용하는 방법(NP-04/05):** source NAME → resolution event → semantic function/value identity → relevant dtype/rank/shape/effect facts → proof/guard → specialization 순서를 유지한다. 예를 들어 `f`가 Add로 관찰되면 fusion 후보를 만들 수 있지만 call-time target/POS 및 lookup 의존성이 검증돼야 Add로 실행할 수 있다. `f`의 spelling을 primitive dispatch key로 사용하지 않는다. Summary cache와 executable cache는 분리하고, 관련 binding stamp·lookup dependencies·semantic identity·필요한 argument facts를 키로 삼는다. 실행 코드 캐시는 추가로 target/numeric policy를 포함한다. Shape와 상수를 무조건 전부 키에 넣지 않으며 bounded cache와 merge/widening을 적용한다. 동일 구조의 함수는 적법한 범위에서 코드를 공유하되 occurrence/source 진단은 별도 mapping으로 보존한다.

| 배열 compiler의 실제 구조 | RustJ에 적용할 설계 판단 |
|---|---|
| [JAX jaxpr](https://docs.jax.dev/en/latest/601/jaxpr.html)의 typed input/constant/intermediate 변수 | lookup으로 확보한 noun은 explicit SSA input/constant carrier로 전달하고 원 NAME/event는 sidecar에 남긴다. JAX의 trace-time global 고정은 가져오지 않는다. |
| [PyTorch Dynamo](https://docs.pytorch.org/docs/2.14/user_guide/torch_compiler/compile/programming_model.dynamo_core_concepts.html)의 guards·graph breaks | 동적 NAME 구간과 검증된 배열 region의 경계를 명시한다. J의 POS·locale·오류 순서를 보존하는 continuation은 RustJ가 별도로 소유한다. |
| [OpenXLA HLO→Thunks](https://openxla.org/xla/hlo_to_thunks)의 schedule 이후 buffer assignment | NAME/SSA identity로 주소나 버퍼 재사용을 결정하지 않는다. alias/liveness/readiness 증명 후 physical allocation과 실행 수명을 결정한다. |

**Worker/device 경계(NP-05):** namespace owner가 원래 J 시점에 확보한 immutable value versions, 검증된 callable 또는 닫힌 array region, readiness/effect/error dependencies와 leases를 physical 실행에 넘긴다. CPU worker가 임의 시점에 원 NAME을 재조회하지 않는다. Region 안에 더 늦은 동적 조회가 필요하면 semantic boundary를 유지하고 그 지점에서 owner로 돌아간다. Lease는 enqueue가 아닌 실제 작업 완료까지 유지한다. 별도 process에서는 raw pointer/Engine-local NameId를 직렬화하지 않고 value/region transport와 수신 측 identity remapping을 명시한다. 이것은 shared mutable namespace를 프로세스 간 자동 복제하는 설계가 아니다. CUDA 구현은 계속 보류다.

**검증 증거와 남은 구현(NP-07):** [native Windows C probe](tools/name_system_research.py)와 [기계 판독 결과](reports/name-system-research-windows.json)를 추가했다. noun snapshot, verb late lookup, undefined→defined, 기대 POS 오류, 문장 내 대입/조회 순서, direct locative, `f.` 고정, `name_:` abandon의 **8건 × C j64/AVX2 = 16/16 PASS**다. Source pin `13994ffa…`와 reference asset pin `ded7793…`은 별개이고 JSON에 실제 파일 SHA256을 기록한다. 재현: native Windows Python으로 `tools/name_system_research.py --assets-root <reference-assets-checkout>` 실행. 이 결과는 **C 의미 조사**이며 RustJ locative/abandon 구현이나 NP-07 전체 차분 통과가 아니다. 통합 기준 `cd6ac8c`의 깨끗한 작업 트리에서는 `cargo test --locked [--features portable] --test semantic --test parser_capture name`으로 default/portable 각각 **17/17 PASS**, `test_oracle_protocol.py` **9/9 PASS**, `git diff --check`를 확인했다. 전체 Rust suite·fmt/clippy·Linux·GPU·GitHub CI는 이번 설계 변경에서 실행하지 않았다.

NP-03 부정 테스트에 frame/locale/symbol 삭제·재생성과 stale stamp, NP-04에 최적화 후 provenance 및 중간 payload 비보유, NP-05에 guard→lease 사이 재바인딩·worker 완료 전 buffer reuse·효과 이후 guard miss를 추가한다. 구현 순서는 NP-01/02 occurrence/event → NP-03 namespace/stamp → NP-04 IR 연결 → NP-05 guard/lease → NP-06 특수 NAME → NP-07 독립 차분이다. **모든 전체 수용 게이트는 계속 미완료**이며 C probe 일부 통과로 체크하지 않는다.

#### 3.7.5 추가 배열 compiler 비교와 NAME 호환성 재검토 (2026-10-07)

**판정:** §3.7.3/4의 NAME carrier는 필요한 출발점이지만 충분한 compiler 계약이나 jsource 호환 구현은 아니다. 이름 출처 보존에 더해 **SSA use-def/CFG 합류, namespace effects, alias·escape·liveness, shape facts의 범위, transformation provenance와 verifier**가 있어야 최적화의 적법성을 검증할 수 있다. 다음은 upstream 자료에서 관찰한 구조와 RustJ에 대한 설계 판단을 구분한 보강이다. 새 IR 계층을 의무화하지 않는다.

| 추가 참조 / upstream 관찰 | RustJ에 채택할 부분 | 그대로 채택할 수 없는 부분 |
|---|---|---|
| [Futhark language reference §3.10/3.11](https://futhark.readthedocs.io/en/latest/language-reference.html): size variables, branch/loop alias 합류와 fixed point, consuming parameter와 alias-free return | 함수 요약에 argument/result alias·escape·read/write 정보; CFG 합류와 loop fixed point; 실제 입력 값의 shape에서 유도한 size identity | consuming type과 higher-order 제약을 J 문법 제한으로 추가하지 않는다. 일반 J는 alias가 살아 있으면 copy/다른 적법한 실행을 선택한다. |
| [TVM Relax](https://tvm.apache.org/docs/deep_dive/relax/learning.html), [Var/DataflowVar/MatchCast API](https://tvm.apache.org/docs/reference/api/python/relax/relax.html): visible/dataflow variable 구분, symbolic shape, runtime structural check, pure dataflow region | J NAME와 SSA 내부 변수 구분; shape symbol을 source spelling과 분리; proof로 닫힌 array region과 runtime semantic region의 명시 경계 | pure라는 분류만으로 J error·NAME lookup의 reorder가 허용되지는 않는다. `MatchCast` 스타일 검사는 J에 맞는 오류 시점/guard failure 계약이 필요하다. |
| [MLIR One-Shot Bufferization](https://mlir.llvm.org/docs/Bufferization/): tensor SSA use-def와 read-after-write conflict 분석 후 in-place/copy 선택 | noun snapshot의 alias와 생존 reader를 추적하고 마지막 semantic use 및 worker completion을 만족할 때만 버퍼 재사용 | binding version·NameId·reference count 하나를 uniqueness/alias/liveness 증명으로 사용하지 않는다. |

**다음 단계로 넘길 carrier의 보완(NP-01~06):**

1. **문법 축을 직교화:** §3.7.3의 개념 예시에서 `ByValueAbandon`을 locative와 배타적인 form으로 두었던 표기를 수정했다. `address_form`(simple/direct/indirect와 holder/terminal selector), `read_policy`(ordinary/by-value/by-value+abandon), special operand/context flags를 따로 보존한다. 원본 spelling을 없애거나 parser 전에 실제 binding을 고정하지 않는다. Numeric debug-frame locative와 implicit operand의 정확한 분류는 pinned source와 차분으로 닫는다.
2. **SSA/CFG와 namespace를 별도 합류:** `ReadNoun(event) -> ValueId`는 실행 시점의 read이며 compile-time 환경 캡처가 아니다. branch는 도달 가능한 값·facts·alias를 block arguments/phi 상당으로 합류하고, loop는 widening/fixed point를 적용한다. `NameRef`와 namespace state는 별도 효과 의존성으로 유지한다. 한쪽 branch의 binding proof를 합류 이후 무조건 적용하지 않는다. 현재 A3는 single-block/Return-only이므로 일반 NAME CFG 특수화는 미구현이다.
3. **요약을 값과 호출에 연결:** dtype/rank/shape constraints, alias/escape, effect/error 및 nested dynamic NAME dependency는 관련 value/call 분석에 둔다. Source NAME spelling이나 불변 FunctionEntity identity에 호출별 관찰을 넣지 않는다. Named function target guard가 성공해도 함수 본문의 dynamic name이 자동으로 stable/pure가 되지 않는다. Unknown call/execute는 보수적인 effect summary를 가진다.
4. **Provenance는 일대일이 아님:** fold/CSE/fusion/inlining 후 한 노드가 여러 occurrence/event에서 유래할 수 있다. Interned origin-set/공유 derivation DAG 상당의 many-to-many mapping으로 보존하되 실제 runtime lookup 사건을 중복 실행하지 않는다. 제거된 체크/조회에는 proof·guard·order witness를 연결한다. Source metadata만으로 eliminated operation의 적법성을 증명하지 않는다.
5. **Semantic state와 analysis state 구분:** 실제 조회 결과는 noun snapshot, expected-POS NameRef, constructor capture, 미정의 일반 function reference, 또는 J 오류다. `Unknown/Observed/GuardRequired/ProvenStable`은 별도 분석 상태다. 미정의 일반 NAME를 항상 즉시 Value 오류로 바꾸거나 `Unknown`을 J 오류처럼 실행하지 않는다.

**실제 구현을 C와 비교한 범위:** [audit 도구](tools/name_compatibility_audit.py)와 [Windows 결과](reports/name-compatibility-windows.json)는 `c151f36` runtime source의 **26개 독립 stateful fixture × j64/AVX2 × direct/semantic-reference = 104건**을 비교했다. **64 matched, 40 unsupported_gap, semantic mismatch 0**이며 각 조합은 같은 **16개 fixture 일치 / 10개 미지원**이다. 성공률을 전체 J NAME 호환율로 해석하지 않는다. 모든 setup 문장과 관찰한 query의 dtype/shape/data 또는 오류 종류를 비교하고, CLI가 unsupported definition에서 입력을 중단한 경우 미관찰 suffix를 별도 기록한다. C adapter는 기존 `Oracle.eval`의 assignment/result 관찰을 사용하므로 내부 lookup 사건·POS reflection·namespace 전체 상태의 동등성을 검사한 것은 아니다.

| NAME 기능 | 이 audit 및 코드 조사에서 확인한 현재 경계 |
|---|---|
| 일반 noun snapshot, array alias/reshape 후 기존 값 유지, 실패한 RHS의 binding 보존 | 해당 fixture들은 두 DLL·두 runtime 경로에서 일치. 모든 alias/storage encoding을 검증했다는 뜻은 아님 |
| 일반 verb late lookup·alias chain·undefined→defined·미정의 호출·기대 POS 오류, 문장 내 대입/조회 순서 | 해당 fixture 일치. rank/header 변화·recursive/self·모든 train/constructor 순서는 추가 수용 필요 |
| named adverb construction, nameless modifier alias, conjunction alias | 선정 fixture 일치. **모든 function은 동일한 late 정책**이라고 일반화하지 않음 |
| straight-line explicit **modifier**의 local shadow/unbound fallback | 해당 fixture 일치. `3 :` 일반 explicit verb의 local 호출 두 fixture는 Unsupported; local frame 전체 미구현이라고도 하지 않음 |
| direct/base/indirect locative, 실행 locale 문맥 | fixture Unsupported. source grammar 일부 검사는 있으나 locale namespace/path·실행 전환/복원 호환은 미완료 |
| `name_:`, `f.`, 계산된 단일/복수 NAME 대입 | fixture Unsupported. C 조사 통과를 Rust 구현 통과로 승격하지 않음 |
| locale 생성/삭제·path 한 단계·numeric/debug-frame locative·special `m n u v x y`의 전체 조합 | 이번 실행 audit 미검증. NJ/NP 수용 게이트 유지 |
| name reflection `4!:`/`5!:`, execute `".`, recursion/escape/fix, namespace mutation 후 guard invalidation | 전체 호환 미검증. 관찰 가능한 name class/list/representation 및 first error·effect trace까지 NP-06/07 확대 필요 |
| provenance 기반 compiled specialization, CFG guard, native M4 및 worker/device 인계 | 설계 단계/전체 수용 미완료. `--semantic-reference`는 컴파일된 CPU route가 아님 |

**검증과 구현 순서:** NP-01/02 event+직교 NAME syntax → NP-03 error-bearing locale namespace/stamp → NP-04 use-def+many-origin mapping → NP-05 effect/alias summaries·guard/lease verifier → NP-06 특수 NAME/reflective semantics → NP-07 runtime/event/Graph/A3/실제 compiled route 차분. 각 NP-V 부정 테스트에 branch-only proof escape, loop namespace clobber, merged origin 유실, 살아 있는 noun alias overwrite, nested NameRef 의존성 누락을 연결한다. 단일 CPU 경로의 수용 전에 병렬 worker가 namespace를 재해석하게 만들지 않는다.

이번 변경은 **설계·감사 도구만 추가**했으며 Rust semantic 구현은 수정하지 않았다. Native Windows의 enqueuer 19 + modifier_scope 8 + parser_capture 41 + semantic 54 = **122 tests를 default/portable 각각 통과**, audit 분류 회귀 **6건 통과**. 조사한 C source pin과 DLL asset pin은 서로 다르므로 실제 oracle은 binary hash로 구별하며, pinned source에서 같은 DLL을 재빌드한 증거는 없다. 전체 upstream J suite·전체 reflection/locale suite·Miri·GPU·Linux·GitHub CI는 실행하지 않았다. NP 전체 수용 게이트는 계속 미완료다.

#### 3.7.6 Parser→compiler의 구체 전달 계약: 지연 noun과 concrete JEntity (2026-10-07)

**정정·결정:** parser reduction과 kernel execution은 같은 동작이 아니다. Compiler 경로의 기본 출력은 **아직 배열 연산을 실행하지 않은 semantic computation program**이다. noun-producing row 0–2를 줄인다는 것은 noun-result expression/reference를 stack에 넣는 것이며, 결과 POS가 Noun이라는 이유로 데이터를 계산하지 않는다. 별도의 unreduced AST와 계산된 결과를 항상 함께 보관하는 방식도 아니다. 아래 계약은 기존 단계들을 명확하게 한 것이며 새 mandatory IR pass를 추가하지 않는다.

**현재 코드의 두 realization:** `parser::runtime_noun`은 host가 없으면 `Expr::Monad/Dyad`를 그대로 남기지만 host가 있으면 `RuntimeParserHost::apply`로 계산하여 concrete noun을 반환한다. `parse_analysis`/`StaticAnalyzer`는 전자, `parse_runtime_host`와 현재 semantic-reference runtime은 후자다. 후자는 J 동작을 확인하는 실행 경로이지 compiler가 모든 배열을 먼저 계산해야 한다는 설계가 아니다. `ParseCapture -> J Graph`는 실행 관찰·검증 경로이며 **실행을 선행해야만 graph를 만들 수 있는 필수 compiler 경로가 아니다**. 분석 경로에는 `Program -> BoundProgram -> j_graph_ir::Plan::from_bound_with_graph_facts`가 이미 있다.

| 경계 | 다음 단계에 넘기는 실제/목표 형태 | 실행·저장 제한 및 미완료 |
|---|---|---|
| Word Formation/Enqueue → Parser | `EnqueuedWord` queue: spelling/payload/class, NAME lookup flags, original word/span | 이름의 concrete binding/POS·배열 계산을 lexical 단계에서 고정하지 않음 |
| Parser+Semantic Construction → J Graph builder | 현재 `semantic::Program`: root `Expr`, 공유 `FunctionEntity` DAG, assignment metadata, reductions와 name/constructor observations. 지연 noun은 `Expr::{ReadName,Monad,Dyad}` 상당; literal·이미 확보한 noun만 concrete `Value` | `JEntity::{Noun(Value),Function(Arc<...>)}`는 **concrete entity transport**. Deferred expression을 `JEntity::Noun`인 척 넘기지 않음. 일반 ordered writes·full NAME events·continuation 확장은 아직 미완료 |
| Binding/analysis → J Graph | `BoundProgram`의 읽기 의존성·pending write와 `j_graph_ir::Plan`: `ReadNoun`, `Apply(FunctionEntity, argument ValueIds)`, regions/source mapping와 graph facts | catalog NameVersion/shape는 분석 관찰·입력 계약 후보이지 실행 binding guard가 아님. `ValueId`는 계산 결과의 identity이며 버퍼·materialized noun이 아님 |
| J Graph → Execution Semantic Lowering/A3 | `logical_ir::Plan`: operations/SSA values, `ReadNoun`, `VerbReference`, Basis/SemanticCall, SemanticCheck, writes 및 effect/error order | 고수준 J parent/operand·region origin 연결을 유지. 현재 single-block/Return-only; 일반 CFG·namespace/guard·runtime bridge가 완성된 것은 아님 |
| A3 → native physical route 또는 검증된 external adapter | 검증한 computation region + 입력 binding/guard 계약, 이후 physical plan의 buffer/lease/readiness·target realization | native route의 schedule/representation/codegen은 downstream. external route에 native optimizer/scheduler를 강제하지 않음. 실제 CPU M4 연결과 fallback capability는 별도 수용 대상 |
| 실행 완료 → namespace/caller | concrete noun 또는 function 결과와 해당 semantic point의 assignment commit/observable effects | 이때 실제 결과가 생긴다. 지연이 문장 사이에서 관찰 가능한 대입·오류를 임의로 늦추는 lazy-language 변경을 뜻하지 않음 |

**고정 예: `b =: a + c * d`** — 이 예는 `a,c,d`가 Noun이라는 입력 계약/POS가 확보된 구간이다. 일반 이름의 POS를 모른 채 같은 parse graph를 무조건 확정하는 예가 아니다.

~~~text
Parser semantic output (not evaluated):
  assignment target = b
  root = Dyad(+, ReadName(a), Dyad(*, ReadName(c), ReadName(d)))
  callable identities = primitive +, primitive *
  source/reduction/name-use metadata = linked, not a second AST

J Graph (illustrative IDs, right-to-left construction):
  v0 = ReadNoun(d, eventD)
  v1 = ReadNoun(c, eventC)
  v2 = Apply(*, left=v1, right=v0)
  v3 = ReadNoun(a, eventA)
  v4 = Apply(+, left=v3, right=v2)
  pending WriteName(b, v4)

A3: resolved/guarded call semantics + checks + data/effect/error dependencies
Physical: legal fused kernel or separate kernels + actual buffer/lease decisions
Runtime: supply values at semantic read points, execute, commit b at its write point
~~~

이 그림의 `eventD` 등은 NP의 제안 provenance 연결을 나타내는 개념 표기이며, 현재 `ReadNoun{name,version}`에 event 필드가 구현됐다는 뜻이 아니다.

이 graph의 read/apply/write는 **실행할 의미**이지 compiler가 지금 수행한 사건이 아니다. 외부 noun input은 실행 시 해당 read의 immutable value를 공급하거나 동등성을 증명한 입력 bridge로 전달한다. 이미 read된 snapshot을 후속 rebind로 바꾸지 않으며, 아직 실행하지 않은 read를 compile-time noun capture로 바꾸지도 않는다. 분석기로 이미 read/compute된 결과를 넘겨 graph를 복원하는 capture 경로와 혼동하지 않는다. NAME witness와 semantic sequencing을 보강하는 NP-01~07은 계속 미완료다.

**파싱이 실제 값/POS에 의존하면:** literal rank operand 같은 이미 알려진 작은 값은 정상 constructor에 사용한다. 아직 계산하지 않은 값이 constructor result POS·후속 lookup·동적 source를 결정하면, **닫힌 prefix/region + 필요한 값/클래스 요청 + 정확한 parser continuation**을 semantic runtime/JIT 경계로 넘기는 것이 목표다. 이는 새로운 concrete Rust enum이 아니라 필요한 handoff 계약이다. 그 요청을 충족하기 위해 필요한 의존 부분을 실행하고, 원래 frame/locale·queue/stack·효과 상태에서 계속한다. 현재는 그 일반 continuation이 미구현이며 지원되지 않는 분석 경계를 명시한다. 가짜 noun/FunctionEntity로 파싱을 계속하거나 문장 전체를 다시 실행하지 않는다. 런타임 지원 경로가 있다는 이유로 분석 가능한 전체 구간을 항상 eager 실행하는 것을 compiler 기본 경로로 채택하지 않는다.

**최소 수용 작업:** 기존 NP-02/04/05/07 및 M3→M4 게이트 안에서 (a) analysis→Program→Graph→A3까지 kernel 호출/배열 allocation 없이 연결되는 예, (b) runtime input binding과 source origin의 대응, (c) 이미 계산한 concrete 값과 deferred noun의 오용 거부, (d) guarded POS/NAME 변경·동적 constructor에서 정확한 boundary 보고를 검증한다. 새 병행 AST 정본이나 경쟁 checklist는 만들지 않는다.

**이번 확인:** native Windows `static_explain 'data + data * data'`를 `data: Float[1000000000000]` metadata만으로 실행해 `ReadNoun` 3개·`Apply` 2개의 graph를 생성했다. 데이터 배열이나 해당 multiply/add 결과를 계산한 것이 아니다. 이것은 현재의 지연 graph 구성 증거이며 full runtime binding guard·allocation 계측·M4 end-to-end 검증은 아니다. 기존 execution-free semantic 회귀 2건은 default/portable 각각 통과했고 `git diff --check`도 통과했다. 이번에는 문서만 변경했으며 전체 suite·fmt/clippy·GitHub CI를 실행하지 않았다. 다음 구현 우선순위는 이 지연 경로를 실제 CPU 실행으로 닫는 것이며, eager capture를 compiler의 유일한 입력으로 만드는 것이 아니다.

#### 3.7.7 SemanticHandoff v1 논리 데이터 구조 확정 (2026-10-07; 논리 계약·첫 구현)

**현재 계획의 부족과 이번 결정:** 앞 절들은 정보·순서·경계 계약이었으며 구체 carrier가 구현되지는 않았다. 이제 parser→analyzer 전달 단위를 **`SemanticHandoff` 하나**로 정한다. `program`은 reduction으로 구성한 의미 결과, `context`는 reduction 전 입력·해석 맥락, `origins`는 둘의 연결이다. 아래는 **논리 계약을 표현한 목표 Rust schema**다. 첫 구현은 기존 `Program` + `frontend: Option<Arc<FrontendContext>>`에 대응시키며 실제 필드·검증 범위는 이 절 끝의 구현 표에 명시한다. arena로 Program 전체를 이미 교체했다는 뜻은 아니다. 변경은 NP-01/02/04의 단계적 이행으로 수행하고, 같은 stage에 두 의미 정본을 영구 유지하지 않는다.

**확정 범위:** 아래 필드와 참조 관계는 논리 계약이다. `Vec/Box/Arc`, 정수 ID 폭, arena 배치, 직렬화 형식은 구현 예시이며 고정하지 않는다. 의미 결과와 reduction 이전 맥락을 모두 보존하되, 원본 AST를 별도로 다시 구성하거나 중간 배열 값을 계산할 의무는 없다.

~~~rust
struct SemanticHandoff {
    schema: u32,                         // 1; readers reject unknown versions
    unit: FrontendUnitId,                 // owner of all local IDs
    program: Program,                     // sole semantic program
    context: Arc<FrontendContext>,
    origins: Vec<NodeOrigin>,
    status: HandoffStatus,                // Complete or Blocked(DemandId)
}

// Target shape of the existing Program, not an additional compiler IR stage.
struct Program {
    nodes: Vec<SemanticNode>,             // NodeId indexes this arena
    root: Option<NodeId>,
}
enum SemanticNode {
    Literal(Value),                       // source/required concrete noun only
    ReadNoun(NameUseId),                   // deferred semantic read
    FunctionUse(Arc<FunctionEntity>),      // shared completed J function DAG
    Monad { function: NodeId, argument: NodeId },
    Dyad { function: NodeId, left: NodeId, right: NodeId },
    WriteName { target: NameUseId, value: NodeId, event: EventId },
}

struct FrontendContext {
    sources: Vec<SourceRecord>,            // original and expansion sources
    words: Vec<WordRecord>,               // WordId = expanded enqueue index
    items: Vec<ItemRecord>,               // immutable parser occurrences
    reductions: Vec<ReductionRecord>,
    names: Vec<NameSyntax>,
    name_uses: Vec<NameUseRecord>,
    observations: Vec<NameObservation>,
    events: Vec<SemanticEvent>,
    demands: Vec<BoundaryDemand>,
    function_origins: Vec<FunctionOperandOrigin>,
}
struct ItemRecord {
    producer: ItemProducer,               // Word | NameUse | Reduction | Mark
    class: ParseClass,
    semantic: Option<NodeId>,             // None for syntax/control items
    word_range: Range<usize>,
    blame_word: Option<WordId>,
}
struct ReductionRecord {
    row: ParseRow,
    window: [Option<ItemId>; 4],          // inspected stack window before action
    consumed: Box<[ItemId]>,
    produced: ItemId,
    name_uses: Box<[NameUseId]>,
    events: Box<[EventId]>,
}
struct NameUseRecord {
    syntax: NameSyntaxId,
    input_item: ItemId,
    phase: NameUsePhase,
    expected_pos: Option<JPos>,
    policy: NamePolicy,
    event: EventId,
    observation: Option<ObservationId>,
}
struct NodeOrigin {
    node: NodeId,
    items: Box<[ItemId]>,
    reductions: Box<[ReductionId]>,
    name_uses: Box<[NameUseId]>,
}
~~~

**보조 타입도 v1 논리 계약에 포함한다.** 아래 필드와 variant로 연결 관계를 명시한다.

| 타입 | v1 필드/variant와 의미 |
|---|---|
| ID들 | `FrontendUnitId`는 비재사용 unit identity. `SourceId/NodeId/WordId/ItemId/ReductionId/NameSyntaxId/NameUseId/ObservationId/EventId/DemandId`는 서로 구분되는 local identity. 모든 외부 link는 `(unit, local_id)`. Expr의 이동 위치·Arc 주소·source spelling을 ID로 사용하지 않음. ID 폭과 저장 방식은 구현 선택 |
| `SourceRecord`, `SourceSpan` | source는 `text`, `parent: Option<SourceSpan>`을 소유. 최초 source의 parent는 None, DD 등 생성 source는 생성 원인 span을 가리킴. 모든 source span은 `source: SourceId`와 `bytes: Range<usize>`로 구성하여 원문/확장문을 구분 |
| `WordRecord` | `span: SourceSpan`, `class: EnqueueClass`, `flags: EnqueueFlags`, `environment: EnqueueEnvironment`, `expansion_origin: Option<WordId>`; spelling은 해당 immutable source로 읽음. DD 확장에서 같은 span인 word도 다른 WordId. 확장문은 별도 SourceRecord와 parent mapping 유지 |
| `ItemProducer` | `Word(WordId)`, `NameUse(NameUseId)`, `Reduction(ReductionId)`, `Mark`; NAME lookup으로 품사가 바뀌면 이전 ItemRecord를 수정하지 않고 새 item 생성. 실제 reduction window의 context item과 consumed item을 구분 |
| `NameSyntax` | `word: WordId`, `address: Simple/Direct/Indirect`, `components: Box<[NameComponent]>`, `read_mode: Ordinary/ByValue/ByValueAndAbandon`, `special_operand: Option<ImplicitOperand>`; component는 `kind: SimplePart/NamedLocale/NumericLocale/HolderName/DebugFrame` + 원 source span. direct/indirect와 read_mode는 직교. 실제 locale/frame identity는 여기에 넣지 않음 |
| `JPos`, `NameUsePhase`, `NamePolicy` | POS=`Noun/Verb/Adverb/Conjunction`. Phase=`StackRead/StackPOS/ConstructorRead/CallLookup/AssignmentTarget/Abandon`. Policy=`CaptureAtRead/LateAtCall/ResolveAtConstruction/CaptureAtConstruction/Assignment/ReadAndAbandon`. `expected_pos=None`는 미확정 요구이고 Noun 가정이 아님. 하나의 NAME에 stack POS와 call lookup 등 여러 use 존재. 아래 첫 구현의 stack policy는 앞의 세 가지에 한정 |
| `NameObservation` | `origin: Catalog/Runtime`, `outcome: Noun/Function/Missing/JError`, `pos: Option<JPos>`, `lookup: Option<LookupWitness>`, constructor-fixed ranks/POS 등 작은 metadata. 실제 noun은 필요한 semantic Literal/constructor operand가 소유하며 observation에 payload를 복제하지 않음. Catalog/Runtime 어느 관찰도 자동 proof가 아님 |
| `LookupWitness` | `engine`, `frame_instance`, `start_locale`, `found_locale`, `execution_locale`, `binding_stamp`, `dependencies`. stamp=`scope_instance/symbol_generation/revision`; dependencies=`LocalPresent/LocalAbsent/LocalePathEpoch/HolderRead/NamespaceEpoch` typed records. 미확보 witness는 None으로 드러내고 flat NameVersion으로 채우지 않음. 실제 runtime 관찰과 future GuardRecipe는 다른 타입/소유자 |
| `SemanticEvent` | `kind: NameUse/Constructor/Call/Write/Check`, `owner: ItemId`, `node: Option<NodeId>`, `after: Box<[EventId]>`, `effect: NamespaceRead/NamespaceWrite/ArrayCall/UnknownCall/Constructor`, `realization: Planned/Observed`. compile-time 계획과 runtime 실행 사건 구분; constructor·call의 potential error 순서도 edge로 유지. 기본 보수적 순서 이후 완화는 별도 proof 필요 |
| `BoundaryDemand` | `kind: NeedNameClass/NeedConstructorValue/NeedConstructorResult/NeedNamespaceAction/NeedDynamicSemantics/NeedCallableProof`, `at_item: ItemId`, `row: Option<ParseRow>`, `required_nodes: Box<[NodeId]>`, `after: Box<[EventId]>`, `reason: typed reason`; v0에서 runtime continuation을 갖춘 척하지 않음 |
| `HandoffStatus` | `Complete`는 semantic construction complete일 뿐 execution admitted가 아님. `Blocked(DemandId)`는 prefix를 담은 inspection artifact이며 complete graph/실행 계획으로 낮추지 않음. actual JError는 별도 진단 결과; demand/Unknown과 혼합하지 않음 |
| `FunctionOperandOrigin` | `owner: NodeId`, `operand_path: [operand index]`, `items: [ItemId]`, `name_uses: [NameUseId]`; shared FunctionEntity DAG 안의 특정 operand 사용을 구별. 같은 함수 identity를 공유해도 owner/path별 NAME 맥락은 합치지 않음. 빈 path는 함수 root 사용 |

**대입·그룹·미완성 reduction:** `WriteName`은 문장 끝뿐 아니라 표현식 내부에도 존재하며 assigned entity를 결과로 산출한다. event가 namespace write의 의미 시점을 보존한다. 괄호는 의미 중립이면 새 semantic node를 강제하지 않지만 Item/Reduction/origin에는 남긴다. ReductionRecord는 완료된 reduction만 기록한다. 실패/중단 직전 window는 BoundaryDemand의 `window: [Option<ItemId>; 4]`에 보존하며, Blocked 상태에서 완성된 결과를 꾸며내지 않는다.

**Noun capture의 의미:** `CaptureAtRead`는 J의 의미적 read 시점에 binding/value identity를 고정한다는 계약이지 compiler에서 배열 내용을 즉시 계산하라는 지시가 아니다. 읽은 noun이 deferred computation의 결과이면 그 값의 semantic identity와 순서 의존성을 보존한다. 이후 rebinding으로 그 read의 대상을 바꾸지 않는다. 함수의 `LateAtCall`은 호출 시점 lookup이며 둘을 같은 name 문자열만으로 합치지 않는다.

**NodeId와 함수 identity:** arena는 기존 `Expr` tree의 저장 방식을 점진적으로 대체하는 것이며 함수 DAG를 복제하지 않는다. 같은 `Arc<FunctionEntity>`를 두 번 사용하는 경우 `FunctionUse` NodeId와 parser ItemId는 각각 별개다. 함수의 intrinsic operand identity에 call-site NAME/guard/target 정보를 넣지 않는다. Noun 연산 결과는 Monad/Dyad node이지 `JEntity::Noun`의 가짜 concrete Value가 아니다. NodeOrigin은 many-to-many여서 parentheses/NAME 치환처럼 여러 item이 같은 결과를 지시하는 경우도 표현한다.

**고정 연결 예(`a+c*d`, 각 입력 Noun POS 계약):** words는 W0=a,W1=+,W2=c,W3=*,W4=d. queue-origin NAME item과 stack-read 후 Noun item은 다른 ItemId이고 NameUse로 연결한다. R0은 `DyadNVN`, consumed=`[c의 noun item, * item, d의 noun item]`, produced=P0(Noun, semantic=Nmul). R1은 consumed=`[a의 noun item,+ item,P0]`, produced=P1(Noun, semantic=Nadd). Nmul origin은 R0/관련 NAME uses, Nadd origin은 R1/관련 NAME uses를 가리킨다. **span이 같다는 이유로 항목을 연결하거나 reduction 이전 맥락을 덮어쓰지 않는다.**

**Downstream 계약:** Graph/A3는 `(FrontendUnitId, NodeId)`를 origin key로 사용한다. 각 adapter가 target node → **원 semantic NodeId 집합**의 mapping을 산출하고 같은 `Arc<FrontendContext>`를 공유한다. Fusion/CSE는 origin 합집합, inlining은 caller/callee unit mapping을 유지한다. NodeOrigin과 NAME use를 원 spelling으로 다시 추측하지 않는다. Layout/BufferId·schedule·GPU placement·CostProfile은 이 schema에 들어가지 않는다. 변환 후 필요한 semantic program은 검증용으로 참조/보존하되 원 AST·runtime 중간 배열 전체를 복제하지 않는다.

**v1 verifier와 이행 계획(기존 NP/DB 게이트):**

1. **NP-01/04: 타입·저장소·verifier를 먼저 구현.** ID 범위/unit, producer/output 역참조, 이전 입력 occurrence 보존, 원 words coverage와 실제 matcher window/row, source/expansion span, callable/noun edge 종류, roots/origins coverage, event edge cycle, unresolved demand와 Complete의 모순을 검사한다. fabricated producer/node link 하나만 바꿔도 거부하는 부정 테스트 필수. 이 검사는 J 의미의 독립 증명은 아님.
2. **NP-02/04: parser emission.** 같은 matcher/action에서 Word→Item→NameUse→Reduction→SemanticNode를 함께 기록한다. 기존 Program/ParseReduction/NameUse/ParseCapture 소비자는 migration adapter를 거쳐 비교한다. Capture는 runtime 검증용으로 유지하고 compiler context를 만들기 위해 eager 실행하지 않는다. Arena 전환 시 기존 Expr를 두 번째 정본으로 남기지 않는다.
3. **NP-03/05: 실제 lookup API와 witness 연결.** catalog 관찰과 runtime witness를 구별하고 error-bearing locale lookup을 연결. 없는 locale/path/frame 증거를 가짜 값으로 채우지 않는다. concrete lookup support 미완료인 form은 demand/coverage 경계로 유지한다.
4. **NP-04/05: Graph/A3 origin adapter.** parent/operand/function-use와 각 name use가 다음 단계까지 이어지는 golden 및 fold/CSE origin 유실 부정 테스트. admission verifier 미완료 시 Complete handoff만으로 NAME 특수화를 실행하지 않는다.
5. **NP-06/07: C differential.** noun/verb/modifier·rank/gerund/fork·assignment/locative의 POS/값/오류/namespace 결과와 Rust handoff/context projections 비교. C에서 관찰할 수 없는 내부 queue/lookup는 source-derived fixture 또는 별도 계측 증거로 표시하며 값 비교 통과를 내부 trace 동등성으로 승격하지 않는다.

**최우선 구현 slice — parser 결과와 NAME scope (2026-10-07):** 논리 handoff의 실제 carrier는 기존 `semantic::Program`이다. 그 `expression/assignment/FunctionEntity`가 유일한 의미 정본이고, `frontend_context::FrontendContext`는 동일 parser action에서 만든 immutable 구조·출처 index다. index의 `Literal`에는 배열 payload가 없으며 독립 executor가 아니다. `Expr.origin`이 두 구조를 직접 연결한다. source span으로 결과를 재구성하거나 두 번째 parsing을 수행하지 않는다.

~~~text
Program
  expression: Expr { origin: NodeId?, kind: J semantic expression }
  assignment / constructor snapshots / shared FunctionEntity DAG
  frontend: Arc<FrontendContext>?

FrontendContext
  schema=3, unit: FrontendUnitId, realization: Deferred | Observed
  source, source_origin: SourceOrigin?, words: WordRecord[]
  items: ItemRecord[], nodes: NodeRecord[], origins: NodeOrigin[]
  name_uses: NameUseRecord[], reductions: ReductionRecord[]
  steps: Stack | FrontMark | Reduce, pending: PendingAction?, root: ItemId?
  complete: bool
~~~

| 확인할 조건 | 실제 구현·검증과 제한 |
|---|---|
| reduction 전 맥락과 결과 모두 보존 | WordId/ItemId/NodeId/ReductionId/NameUseId를 구분. NAME 치환은 새 item. reduction은 실제 4-slot window, consumed IDs, produced ID를 기록. 없는 suffix slot은 virtual Mark로 해석. parentheses는 같은 semantic node에 여러 item/reduction origin을 연결 |
| parsing 중 사용 가능 | 현재 matcher/action에서 기록하며 `FrontendContext::verify`가 right-to-left queue→stack과 FRONT MARK 시점, row 선택, 입력·출력·node 연결을 replay. `parse_frontend` 실패는 `FrontendFailure {error, context}`로 성공 prefix와 pending row/window 보존. 성공하지 않은 action의 가짜 output 없음. 이는 runtime continuation이 아님 |
| noun·verb·modifier NAME 시점 | `NameResolution=NounValue/FunctionValue/FunctionReference`, `NamePolicy=CaptureAtRead/LateAtCall/ResolveAtConstruction`. 실제 resolve branch가 정책을 기록. `u`가 또 다른 NameRef 값을 치환한 경우도 FunctionValue이며 u 자체를 late lookup으로 바꾸지 않음. CatalogClass/RuntimeClass/DiagnosticAssumption 및 optional binding version/lookup 관찰 구분. 진단용 noun 가정은 실행 POS proof가 아님 |
| local/global 실제 관찰 | `LookupObservation {engine, frame, search, local_state, found, binding_version, binding_generation, binding_class}`. Search=GlobalOnly/CurrentFrameThenGlobal; local_state=NoFrame/Bound/DeclaredUnbound/Absent; found=Local(scope)/Global(scope)/Extension/Missing. Engine/global namespace와 invocation frame은 비재사용 ScopeInstanceId. 같은 spelling·version=1이어도 다른 frame의 binding을 합치지 않음 |
| explicit/direct definition local name | `DefinitionCode.name_plan`의 monad/dyad별 `DefinitionScopePlan`에 local 선언 목록, 원 sentence/word/span과 ReadCurrentFrameThenGlobal/LocalAssignmentTarget/GlobalAssignmentTarget, dynamic assignment target 위치를 보존. 생성 시 body lookup/POS/값 계산 없음. actual invocation은 이 local 선언 목록을 사용하며 u/v/m/n/x/y는 invocation operand/argument로 따로 공급 |
| 선언과 실제 binding 구분 | `t=.t+u`에서 미초기화 local t의 RHS는 global fallback 가능하고 대입 뒤 읽기는 local. 선언은 read를 local SSA로 고정하는 증거가 아님. ordinary local function NAME은 late reference를 보존하며 반환 후 caller frame을 closure처럼 캡처하지 않음. implicit operand의 function value 치환과 구별 |
| 후속 최적화 입력 | J Graph의 각 node에 `parser_origins: [NodeId]`, 같은 context Arc 유지. A3 `parser_provenance`는 기존 operation.j_origin→Graph ID→parser NodeId 체인을 보존. derived-function 확장의 생성 node는 보수적인 enclosing call origins를 유지하므로 가장 작은 source occurrence의 유일 매핑으로 주장하지 않음. Context/Graph/A3 verifier는 누락·범위 오류·source 불일치를 거부 |
| 메모리·실행 경계 | analysis는 context를 항상 보존, runtime은 capture를 요청한 경우에 기록. 일반 runtime body는 추적 비용을 강제하지 않음. 함수는 기존 Arc 공유, 중간 배열을 context에 복제하지 않음. gerund singleton boxed header는 반복적으로 벗겨 기존 depth/rank/empty 순서와 limit를 유지하면서 Windows 재귀 stack 증가를 방지 |

**NAME 관련 최적화 금지 규칙:** `DefinitionScopePlan`은 source inventory이지 닫힌 local 집합·현재 POS·값 proof가 아니다. computed assignment, execute, 제어 흐름이 만들 수 있는 추가 이름을 배제하지 않는다. local 부재·global fallback을 lookup 삭제/hoist에 활용하려면 frame liveness, symbol generation, namespace/local epoch, locale/path 및 의미적 read 시점까지 별도 witness/guard가 필요하다. 현재 `LookupObservation`이나 flat binding version만으로 그 최적화를 허용하지 않는다. actual C name lookup의 buckets/lookaside는 저장 방식이며 RustJ가 복제할 계약은 lookup 결과·시점·fallback이다.

**수용 체크리스트:**

- [x] 논리 결과/입력/NAME ID 관계와 소유 범위 확정; context schema/unit 검증.
- [x] 기존 9개 action에서 emission; action 전 window와 완료 result 연결; failure prefix와 pending action.
- [x] source-derived C `p.c::cases` literal mask 표와 9^4=6,561개 class window의 first-row 선택 비교. C 내부 실행 trace를 계측한 검증과 구별.
- [x] 같은 구조의 queue/stack replay, 잘못된 producer/operand/origin·scope 관찰 부정 테스트.
- [x] Graph/A3까지 출처 전달 및 origin 유실 거부; metadata만 있는 10^12-element noun의 deferred 분석.
- [x] explicit/direct local declaration·global fallback·shadowing·global write·반환된 ordinary function NAME·implicit operand 치환 회귀.
- [x] 지원 중인 simple NAME의 runtime observation에 재사용 없는 BindingGeneration/POS 추가; 실제 NAME use ID에서 SimpleNameGuard 생성, Engine에서 검색 전체 재검사 및 typed invalidation 결과 제공.
- [ ] 완전한 LookupWitness/GuardRecipe, constructor별 모든 NAME use의 독립 event ID, function operand path별 provenance, multi-unit inlining origin merge.
- [ ] source expansion entry 전체와 typed BoundaryDemand/정확한 runtime continuation, Program 전체 arena 저장 전환. 현 실패 context를 실행 가능한 continuation으로 사용하지 않음.

이 slice는 **지원 중인 parsing action과 compilation 경로의 구조 보존**을 닫는다. full-J/locale/control executor 또는 NAME 특수화 실행 적법성 완료를 의미하지 않는다. 상세 실행 검증 결과는 아래에 기록한다.

**Simple NAME guard 첫 구현:** `SimpleNameGuard::from_name_use(context, NameUseId)`는 검증된 완료 context의 runtime bound simple NAME만 받으며 `(FrontendUnitId, NameUseId)`를 보존한다. catalog/diagnostic 가정, missing/extension, locative/by-value form은 이 recipe의 수용 범위 밖이다. `Engine::check_name_guard`는 원래 engine·현재 frame 정체성을 먼저 확인하고 current-frame→global→기본 z noun 검색을 다시 수행하여 local bound/declared-unbound/absent 상태, found scope, binding version·generation·POS를 대조한다. 결과는 `ValidAtCheck/EngineChanged/FrameChanged/LookupChanged`이며 J 오류나 자동 fallback이 아니다. 성공한 할당마다 `BindingGeneration`을 새로 발급하여 삭제·재생성 후 version/POS가 같아지는 ABA도 거부한다. 현재 expunge 언어 지원을 추가한 것은 아니며 ABA 회귀는 내부 table 삭제로 재현한다.

이 recipe는 검색을 생략하는 epoch fast path가 아니다. 같은 이름의 재할당과 fallback을 가리는 local 생성, invocation 변경은 무효화하고 관계없는 binding 변경은 허용한다. 실패한 RHS는 기존 guard를 유지한다. noun snapshot을 새 값으로 갱신하거나 ordinary verb의 late lookup을 고정하지 않는다. `ValidAtCheck` 이후 effect 또는 다른 mutable engine 접근을 넘는 유효성, locale/path, concurrent lease, compiled dispatcher와 effect 이전 route admission은 여전히 미구현이다. 따라서 full LookupWitness/GuardRecipe 체크리스트는 열린 상태로 유지한다.

`g=:f` 뒤 f만 변경하면 g의 guard는 여전히 `ValidAtCheck`일 수 있지만 g의 실행 결과는 달라진다. 이는 의도한 직접 binding 검증이며 전이적 function DAG의 실행 동일성을 증명하지 않는다. 후속 특수화는 고정하려는 각 late read의 별도 guard와 의미적 시점·효과 유효성 근거를 요구한다.

**전이적 alias-call guard와 단일 순수 호출 lease (2026-10-07):** `name_guards::AliasCallGuard`는 runtime context의 ordinary `LateAtCall` NAME에서 시작해 **단순 별칭 체인 → operand 없는 `+/-/*/%` Verb**만 수용한다. `Engine::prepare_alias_call_guard(&Arc<FrontendContext>, NameUseId)`는 기존 root 관찰의 유효성을 먼저 확인하고 각 alias의 spelling, 실제 LookupObservation, shared immutable target Arc를 `AliasRead[]`에 보존한다. `h→g→f`에서 f만 바뀌어도 read index 2로 무효화를 보고한다. literal noun snapshot, constructor-time modifier, implicit function-value substitution을 ordinary call-time 조회로 바꾸지 않는다. derived operands·hook/fork·rank/reduce·definition body를 미리 순회하거나 실행하지 않는다. 미정의/POS 변경/순환/깊이 한계는 typed admission 결과이며 원래 J 오류보다 먼저 자동 노출하는 dispatcher는 없다.

원 `FrontendContext` Arc와 `(unit, NameUseId)`를 guard가 공유하여 수명을 보장한다. `AliasCallGuard::verify`는 원 use와 root observation 연결, chain next-name/target, terminal primitive, operand 부재, generation 중복·누락·빈/과대 chain을 검사한다. `Engine::check_alias_call_guard`는 실제 namespace/frame과 모든 read를 재검사한다. 이 guard가 보존하는 것은 derived runtime evidence이며 별도 executable function IR이 아니다.

입력은 의미적 순서로 먼저 계산하고 **호출 직전** `validate_alias_call_guard`를 실행한다. 성공 시 `ValidatedAliasTarget<'engine>`가 Engine과 guard를 불변으로 빌리며, token을 소비하는 `apply_monad/apply_dyad`가 기존 산술 kernel을 한 번 실행한다. 살아 있는 token과 Engine mutation을 함께 사용하는 코드는 compile-fail doctest로 거부함을 확인한다. 이 slice는 기존 parser/runtime/Logical executor의 자동 경로 선택을 바꾸지 않으며 새 CLI 실행 모드도 아니다. raw kernel 오류를 반환하는 명시적 호출 API다. 실패 시 kernel을 실행하거나 문장/인자 효과를 replay하지 않는다. caller는 이미 보관한 noun으로 지원되는 Rust 실행을 명시적으로 선택할 수 있다. 전체 region admission, CFG continuation, arbitrary effectful function·locale/path·concurrent Engine lease는 계속 미구현이다.

`ParseCapture.frontend`는 같은 parser가 만든 `Program.frontend` Arc를 공개하고, 실패 시 완료된 prefix를 보존한다. 두 번째 parsing이나 event→context 재구성을 수행하지 않는다. 실패 context는 여전히 실행 가능한 continuation이 아니며 guard admission은 완료 context만 받는다. explicit invocation frame에서 global fallback이 local 생성으로 가려지거나 frame이 끝나는 경우의 전이적 guard 회귀도 포함한다.

- [x] ordinary alias chain의 전이적 조회·원 parser context 소유·typed invalidation 및 verifier.
- [x] 지원 산술의 단일 call-time borrow lease, 실제 kernel 실행, effect 재실행 금지 회귀.
- [x] 공개 capture의 성공/실패 context 보존, 순환·깊이·잘못된 recipe·local fallback/frame 종료 경계 회귀.
- [x] 계산된 인자를 move로 소유하는 AliasCall 및 typed miss; 동일 engine/frame의 명시적 단일 semantic call 이어 실행. 다른 engine/frame·invalid recipe에서는 소유권 반환.
- [ ] derived function operand별 의미적 lookup 시점·origin path와 일반 effect/region admission. 현 alias chain을 이 전체 영역의 proof로 사용하지 않음.

**계산 완료된 단일 호출 경계:** `AliasCall { guard: Arc<AliasCallGuard>, x: Option<Value>, y: Value }`는 원 parser context의 callee와 이미 계산한 논리 값들을 유지한다. `try_alias_call(&self, call)`은 `Executed(Result<Value>)` 또는 `Miss(AliasCallMiss { check, call })`를 반환한다. kernel의 length/domain 오류는 Executed의 J 오류이며 guard miss로 바꾸지 않는다. miss는 인자를 그대로 반환하고 함수나 인자를 실행하지 않는다. caller가 `resume_alias_call(&mut self, miss)`를 명시적으로 호출하면 현재 engine/frame을 다시 확인하고 **원 NameRef Arc + 저장한 x/y**를 기존 `call_entity`로 전달한다. 현재 binding의 POS/값 변경은 원 Rust 의미 경로가 Domain/Value 등으로 처리한다. 원 callee는 source span에서 재구성하지 않는다.

이어 실행 전에 engine/frame이 다르거나 recipe가 손상되면 `Miss`로 인자 소유권을 다시 돌려주며 실행하지 않는다. RHS/인자 계산을 다시 파싱하거나 namespace에서 noun을 재조회하지 않는다. 따라서 인자 계산 중 count 증가·함수 재대입이 있어도 count는 한 번만 증가하고 이미 읽은 noun은 이후 재대입에 영향받지 않는다. 이는 **이미 인자를 계산한 ordinary call 지점**에 한정한다. pending parser/constructor, control block, 문장 중간의 임의 위치를 재개하는 continuation 또는 자동 compiled dispatcher를 구현한 것이 아니다. lease API는 raw kernel 오류를, semantic resume는 기존 runtime 진단을 반환하며 두 경로의 전체 human diagnostic 동등성은 아직 검증하지 않았다.

**Alias/call-ready slice 실행 검증 (2026-10-07, Windows native):** default/portable 각각 **578 passed / 17 ignored / 0 failed**. library 75개, `tests/name_guards.rs` 10개 및 살아 있는 lease와 Engine mutation의 충돌을 거부하는 compile-fail doctest를 포함한다. fmt/clippy 통과. Python alias audit 3개·NAME audit 8개·oracle protocol 9개 통과. `examples/alias_guard_probe.rs`가 실제 lease kernel 호출과 miss 후 저장한 인자로 `resume_alias_call`을 실행한다. `tools/alias_guard_audit.py`의 27 fixture × C j64/AVX2 = **54 observation 모두 일치** (`reports/alias-call-guard-windows.json`). 배열/빈 배열/scalar 확장/length 오류, 인자 효과 1회, target noun·미정의 NAME의 Domain/Value를 포함한다. guard 상태는 Rust 자체 assertion이며 C에서 계측한 event가 아니다. C oracle 출력 dtype 범위 내 유한 비교이고 전체 numeric category/성능 증명도 아니다. 기존 scope 비교 **40 일치**, NAME **64 일치 / 40 미지원**, boundary **36 일치 / 4 미지원**도 재실행했다. 전체 diagnostic, GPU/Linux/remote CI 검증은 수행하지 않았다. 아래 guard 565건/parser 560건 기록은 이전 slice 결과다.

**Guard slice 실행 검증 (2026-10-07, Windows native):** default/portable 각각 **565 passed / 17 ignored / 0 failed** (library 73개, 신규 guard 회귀 5개 포함). 별칭의 전이적 late target에 대한 assertion 추가 후 관련 library scope 회귀 7개를 두 설정에서 재실행해 통과했다. fmt/clippy 통과; Python audit 8개·oracle protocol 9개 통과. C j64/AVX2 × 두 Rust 경로 재비교는 scope **40/40 일치**, 기존 NAME **64 일치 / 40 미지원**, semantic boundary **36 일치 / 4 미지원**이며 의미 불일치는 없다. C 값/오류 비교는 Rust guard 내부 적법성의 독립 증명으로 간주하지 않는다. 아래 560건 검증 기록은 앞선 parser-context slice의 결과다.

**실행 검증 (2026-10-07, Windows native):** default와 portable 각각 **560 passed / 17 ignored / 0 failed**. 신규 frontend 통합 회귀 15개와 library 전체 68개를 포함한다. `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings` 통과. Python audit 분류·전달 회귀 8개와 oracle protocol 회귀 9개 통과. C j64/AVX2 × Rust direct/semantic-reference 비교에서 explicit/direct NAME scope 10 fixture의 **40 observation 모두 일치** (`reports/name-scope-handoff-windows.json`); 기존 NAME 26 fixture는 **64 일치 / 40 미지원**, semantic boundary 10 fixture는 **36 일치 / 4 미지원**. 각 observation은 setup/error 후 read를 포함하는 문장 sequence이며 C 내부 event trace나 compiled route 검증이 아니다. 여러 줄 explicit block setup만 C script 입력으로 전달하고 양쪽 원문은 동일하게 보존한다. 보고서는 binary/DLL hash와 source/asset revision 차이를 기록한다. 미지원 locale·execute 등은 통과로 계산하지 않는다. GPU/Linux/remote CI 검증은 실행하지 않았다.

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

<a id="dynamic-semantic-boundaries"></a>

### 3.9 동적 의미와 컴파일 경계 계약

이 절은 §3.4/§3.7/§3.8의 순서·이름 의미와 FOUNDATIONS §21/§33을 실행 경로 선택의 조건으로 모은 **설계 계약**이다. 구문별 영구 컴파일 금지 목록을 만들지 않는다. J-valid 여부, graph 구성/부분 분석 가능 여부, 특정 변환의 적법성, 특정 backend의 실행 capability는 서로 다른 판정이다. frontend가 실제 constructor/POS를 아직 확정하지 못하면 source·원인·시점의 경계를 보고하며 가짜 completed graph를 만들지 않는다.

#### 3.9.1 경계별 필요한 근거와 처리

| 경계 | 필요한 근거 | 검사·사용 시점 | 근거가 없거나 무효일 때 | 현재 구현 상태 |
|---|---|---|---|---|
| **DB-N noun 입력** | J가 읽은 noun snapshot과 dtype/rank/shape; contents 조건은 별도 | parser가 요구하는 noun read 시점. 외부 분석 metadata는 실제 입력 연결 시 별도 확인 | 확보한 facts만 분석. payload를 상수로 추측하거나 후속 binding으로 기존 noun을 교체하지 않음 | noun snapshot과 WI1 metadata 검사는 존재. WI1은 atoms·binding·effect·storage 안전성 또는 실행 계획 재사용을 증명하지 않음 |
| **DB-F 이름으로 호출하는 함수** | 실제 lookup 환경, 현재 POS/callable, 전문화가 사용한 binding의 안정성 | J parser-time POS lookup과 call-time late lookup을 각각 보존. 생성 시 복사한 rank/cap 의미는 별도 constructor 사실 | NAME/source DAG와 동적 호출을 보존. 안정성 없는 inline/CSE/hoist/전문화를 거부 | late NAME/POS 처리와 constructor snapshot seam은 일부 구현. catalog version은 runtime witness가 아니며 통합 guard dispatcher 미구현 |
| **DB-L local/locale 검색·변경** | 호출 frame, local binding 유무, current locale/path, locative와 검색 결과의 유효성 | 실제 의미적 lookup 지점 및 영향 있는 namespace/context 변경 뒤 | runtime lookup/write와 순서를 유지. 단일 referent version만으로 전체 검색 결과를 고정하지 않음 | local/public assignment와 modifier frame 일부 구현. full locale/locative/path guard는 미구현 |
| **DB-X 문자열 실행 `".`** | 문자열 내용과 실행 환경/POS/효과 계약. 상수 문자열만으로 purity를 증명하지 못함 | 실행 지점. 동적 문자열은 지원되는 shared frontend/JIT 경로가 실제 존재할 때 처리 | opaque runtime 경계. 임의 상수 치환이나 문자열만의 cache key로 실행 환경을 생략하지 않음 | general execute/JIT/cache route는 이 계약으로 지원을 선언하지 않음; 구현·검증 후 capability 등록 |
| **DB-C 값 의존 modifier/definition 생성** | 필요한 noun 값, constructor 성공/오류, 실제 result POS와 생성 사실 | 해당 parser reduction 지점. semantic runtime action이 이후 이름/POS 해석에 영향을 주면 그 뒤 파싱도 같은 상태를 사용 | 생성 경계에서 shared semantic parsing 요구. 실행하거나 생성하지 않은 결과를 추측하지 않음 | shared parser/runtime constructor와 capture 일부 구현. static 값 부족·full definition/control coverage는 경계 유지 |
| **DB-E 효과·관찰 가능한 오류** | namespace/resource read/write, I/O/context 변경, error/throw 및 catch 가능성, speculation 계약 | 원래 의미적 실행 순서. 재배치/병렬화/생략 전 legality 확인 | ordering barrier 유지. effect unknown을 pure로, unused result를 unused effect로 간주하지 않음 | 일부 runtime 순서·오류 회귀와 초기 effect 계약 존재. full effect graph/optimizer legality·dispatch 완료를 뜻하지 않음 |
| **DB-A 값 의존 결과·boxed/sparse·rank assembly** | 실제 연산의 type/shape/구조·empty prototype·fill/assembly 계약 | 필요한 facts를 알 때 분석하고 실행 시 잔여 조건 확인 | unknown 또는 conservative lowering. 검증되지 않은 uniform tensor/affine access로 축소하지 않음 | foundations와 일부 실행/분석 존재. general prototype·heterogeneous assembly·full boxed/sparse 경계는 계속 미완료 |

이 표의 runtime 처리는 **존재하고 검증된 RustJ capability에 한정**한다. 가능한 fallback이 항상 있다고 가정하지 않으며 C 엔진으로 넘기지 않는다. 지원 경로가 없으면 implementation coverage 경계로 보고하고 J-invalid 오류와 구분한다.

#### 3.9.2 허가하지 않는 변환과 guard 실패 계약

- 안정성 근거 없이 NAME을 inline하거나, 문장 시작 환경으로 모든 이름을 고정하지 않는다. `=.`/`=:`/locale 변경·dynamic execute가 lookup에 미치는 영향을 보존한다. unbound local에서 locale로 내려간 lookup도 이후 local binding 생성으로 무효가 될 수 있다.
- 검사한 binding/metadata가 **사용 시점까지 같은 의미를 갖는다는 근거**를 요구한다. region entry의 version 검사 하나로 충분하다고 가정하지 않는다. 필요한 변경 지점에서 다시 검사하거나, 분석으로 그 사이의 변경 불가능성을 증명한다. concurrency가 있으면 snapshot/lease/synchronization 등 실제 계약이 추가로 필요하다.
- constructor-fixed rank·cap 정보와 executable binding witness를 섞지 않는다. implicit operand를 fix하여 modifier를 재구성한 **새 entity**의 생성 사실은 원래 entity의 late lookup 변경과 다르다.
- topology만으로 fork를 병렬 실행하거나 효과·오류가 있는 호출을 CSE/hoist/speculate/delete하지 않는다. Graph IR을 보존·분석할 수 있다는 사실은 해당 변환의 허가가 아니다.
- specialization guard miss는 J semantic error가 아니다. effect 이전에 재분석하거나 지원 route를 선택한다. 명시 입력 계약 위반과 J 자체의 오류는 각각 별도 정책을 따른다. compiler 분석용 실패를 원래 J 오류 대신 사용자에게 먼저 노출하지 않는다.
- **effect 이후 문장 전체 자동 replay를 금지한다.** 후속 runtime 전환은 이미 수행한 write/I/O, live noun snapshot, frame/locale 및 parser queue/stack·reduction 위치를 포함하는 정확한 continuation이 구현·검증되었을 때만 허용한다. 그 전에는 effect 이전 경로 선택만 지원하고 중간 전환 capability를 선언하지 않는다.
- backend precondition miss는 그 route의 거부다. 의미를 검증하지 않은 다른 backend나 native path를 자동으로 성공 가능한 fallback처럼 사용하지 않는다.

#### 3.9.3 unknown 분류와 구현 경계

| 미확정 정보 | 가능한 정적 작업 | 추가로 필요한 조건 |
|---|---|---|
| shape/type/extent | 알려진 graph topology, 부분 shape/resource 식과 경계 보고 | 해당 kernel/메모리 분석이 사용하는 잔여 조건 |
| 함수 identity/POS 또는 constructor 결과 | source/provenance와 아직 확정 가능한 구조 보존 | 실제 lookup/constructor와 안정성 근거. POS가 미확정이면 completed parse를 추측하지 않음 |
| effect/error/alias | 보수적 ordering·reuse barrier 유지 | 해당 변환을 허용할 legality proof |
| physical layout/device/capability | Logical 의미와 route 후보 보존 | representation/target adapter 검증. logical facts에 physical 가정을 넣지 않음 |

현재 `GraphAnalyzability`의 Static / StaticWithUnknownFacts / RequiresSpecialization / DynamicSemanticFallback과 `AnalysisBoundary`는 초기 분류다. **Static은 pure/error-free/reorderable/executable을 뜻하지 않는다.** `validate_noun_inputs()` 성공도 함수·효과·전체 실행 안전성을 뜻하지 않는다. 위 분류를 full guard/continuation/dispatcher의 구현으로 오인하지 않는다.

#### 3.9.4 Fallback / guard miss / replay decision table

RustJ에서 `fallback`이라는 말을 하나의 의미로 쓰지 않는다. 최소 다음 네 개념을 분리한다.

~~~text
route fallback
  실행 시작 전 capability/precondition을 보고 다른 verified route를 선택

guard miss
  specialization/optimization 가정이 false임을 effect 전에 확인

replay
  이미 시작한 region/sentence를 처음부터 다시 실행

continuation / deopt
  이미 완료한 의미적 effect를 보존한 채 정확한 semantic point에서 재개
~~~

현재 RustJ가 architecture상 일반적으로 허용하는 것은 **실행 전 route fallback**이다. effect 이전 guard miss도 지원 capability가 실제 존재할 때 reanalysis/reselection으로 처리할 수 있다. 일반 replay나 exact continuation/deopt는 구현·검증된 capability가 아니다.

| 발생 상황 | J semantic error인가? | 다른 route 선택 가능? | replay 가능? | 현재 원칙 |
|---|---|---|---|---|
| compile/lowering 시 target capability miss | 아니오 | **예**, 아직 실행 전이고 verified alternative가 있으면 | 필요 없음 | route miss로 처리; 없으면 개념상 UnsupportedImplementation(현재 concrete API는 `Error::Unsupported`) |
| specialization guard miss, observable effect 전 | 아니오 | **예**, reanalysis/verified fallback route가 있으면 | 원칙적으로 재실행보다 새 route 선택 | guard miss는 J error로 노출하지 않음 |
| explicit compiler/API contract violation | J 자체 오류와 별개 | contract가 허용한 정책에 따름 | 자동 replay 아님 | 잘못된 user/compiler contract와 J Domain/Rank 등을 구분 |
| A3 `SemanticCheck` 또는 semantic call이 내는 J Domain/Length/Rank/Index 등 | **예** | 아니오. 다른 backend로 바꿔 같은 J error를 회피하지 않음 | 아니오 | 원래 error class/precedence를 보고 |
| backend adapter precondition miss, region 실행 전 | 아니오 | **예**, verified alternative가 있으면 | 필요 없음 | 해당 route만 거부 |
| native/external implementation Unsupported, 아직 어떤 observable effect도 시작 전 | 아니오 | **조건부**. 전체 region이 untouched이고 대체 route가 검증된 경우만 | v0에서는 route 재선택으로 처리 | implementation coverage 경계 |
| kernel/external route가 일부 실행된 뒤 implementation failure | 보통 J semantic error가 아님 | 자동으로는 아니오 | **기본 금지** | cleanup 후 implementation failure/Unsupported를 보고; exact transactional contract 없이는 replay 금지 |
| namespace write/I/O/error-observable effect가 commit된 뒤 실패 | 경우에 따라 별도 J error가 이미 관찰 가능 | 자동으로는 아니오 | **금지** | exact continuation이 구현·검증되기 전에는 중간 fallback capability를 선언하지 않음 |
| async route가 일부 completion/token을 발행한 뒤 실패 | 자동 J error 아님 | token/resource 상태를 완전히 증명할 때만 | 기본 금지 | future async contract가 completion/effect frontier를 소유해야 함 |

##### `RuntimeSemanticFallback`의 정확한 뜻

현재 `lowering.rs::RouteDecision::RuntimeSemanticFallback`은:

> **이 operation에 현재 native ExecutionBasis realization이 없으므로 semantic/runtime route가 필요하다는 compile-time 분류**

다. 이것은 “native kernel을 실행하다 실패하면 언제든 interpreter로 되돌아간다”는 runtime deoptimization 보장이 아니다. 실제 runtime semantic executor가 해당 form을 지원하지 않으면 최종 결과는 개념상 `UnsupportedImplementation`, 현재 concrete API로는 `Error::Unsupported(...)` / kind `"unsupported"`일 수 있다.

##### fallback commit frontier

future dispatcher는 최소한 다음 frontier를 추적해야 한다.

~~~text
before_start
  아무 observable effect/consumer-visible transfer도 없음

guarded_but_uncommitted
  guards/checks는 수행했지만 replay-sensitive effect 없음

committed
  namespace write / I/O / externally visible mutation / non-rollback transfer 등이 발생
~~~

`before_start`와 `guarded_but_uncommitted`에서는 verified alternate route 선택이 가능하다. `committed` 이후에는 exact continuation/transaction rollback 증명이 없는 한 region-start replay를 금지한다.

##### 검증 요구

- guard miss가 J semantic error code로 바뀌지 않는 test
- effect 이전 route miss가 alternate verified route를 선택하는 test
- SemanticCheck 실패에서 backend 변경이 error를 숨기지 않는 test
- namespace write 뒤 Unsupported를 강제로 만들고 write가 두 번 실행되지 않는 negative replay test
- async/transfer를 도입할 때 completion frontier 이전/이후 실패를 분리하는 test

이행 항목의 정본은 [§10 DB0–DB7 체크리스트](#dynamic-boundary-checklist)에 둔다. 검사 대상과 구현 상태는 위 계약을 따른다.

#### 3.9.5 경계 판정 알고리즘과 검증 수용 조건 — 첫 CPU route (2026-10-07)

**적용 범위:** §3.7.6의 전달 계약을 첫 CPU route의 실행 전 admission에 적용한다. 일반 J의 언어 제한이 아니며, 다음 판정의 `Need...`는 J 오류가 아니라 **이 compiler route의 정보/구현 요구**다. 현재 `StaticAnalyzer`의 Unsupported/AnalysisBoundary는 일부 seam만 구현하며 아래의 구조화된 admission 결과와 dispatcher는 아직 없다. 문서만으로 실행 허가를 발행하지 않는다.

**판정 지점은 다음으로 고정한다.** 같은 9-row matcher를 쓰되 각 action 직전에 아래 정보를 검사한다. Deferred noun의 dtype/shape가 Unknown이라는 이유만으로 값을 계산하지 않는다.

| 판정 지점 / 코드 seam | Continue 조건 | 충족하지 못하면 반환할 요구 |
|---|---|---|
| queue NAME → stack, `resolve_stack_item` | 현재 의미적 lookup에 해당하는 POS가 입력 계약/증거로 정해졌고 앞선 namespace effect가 그 전제를 바꾸지 않음 | `NeedNameClass`: occurrence·lookup context·선행 effect 의존성. 누락된 catalog 항목을 J Value 오류로 바꾸지 않음; 실제 미정의 NAME의 J fallback과도 구분 |
| row 0–2, `runtime_noun`의 **분석 경로** | callable POS/valence와 noun-result 구조를 알 수 있음 | 기본은 `Expr::Monad/Dyad` 생성. 값 자체를 요구하지 않음. callable target 미확정이면 NameRef를 유지하고 별도 `NeedCallableProof`로 실행 admission을 보류 |
| row 3/4 및 noun-left fork/gerund/definition 생성, `completed_noun`/`CompletedParseResult`/`resolve_modifier` | constructor에 실제 필요한 operand가 concrete이거나 검증된 symbolic constructor가 의미·result POS·오류 조건을 완전히 표현 | `NeedConstructorValue` 또는 `NeedConstructorResult`: 필요한 operand/use, 검사할 값·POS, constructor identity. 현재 computed rank operand `(1+0)`도 이 분석 경계에 해당; 향후 검증된 constant evaluation은 이 요구를 해소할 수 있음 |
| row 7 대입, `apply_parse_row` | 단일 정적 target의 **최종 outer assignment**, RHS는 지연 graph에 남고 commit은 실행 성공 후 | 중간/계산된/복수 target·local/locale 변화는 `NeedNamespaceAction`. 현재 분석기는 non-final assignment에서 중단. symbolic namespace forwarding이 검증되기 전 기존 catalog로 후속 이름을 읽지 않음 |
| `".`·미확정 explicit modifier/opaque call 등 | 분석기가 해당 source/POS/효과 계약을 실제로 보유 | `NeedDynamicSemantics`: dynamic source/constructor/call와 원 위치. 임의 pure 가정 금지 |
| graph→A3→physical admission | 모든 요구가 해소됐고 실제 route가 지원되며 name/POS·nested-call 효과·error order·입력/수명 계약이 검증됨 | `NeedsGuardOrRoute`/`UnsupportedRoute`. graph가 만들어졌다는 사실만으로 실행하지 않음 |

제안 admission 결과에는 `kind | span/word/occurrence | parser row/use | required value/POS/callable | dependency/effect frontier | available evidence | selected action`을 담는다. **현재 Rust enum으로 구현됐다는 뜻은 아니다.** `Continue`는 graph 구성 허용이고 `ExecuteAllowed`는 별도 최종 판정이다. POS만 알아도 graph를 만들 수 있지만 함수 target·본문의 이름/효과·오류 계약 없이는 inline/fusion/native 실행을 허가하지 않는다.

**v0 dispatch는 실행 전에 한 번 결정한다.** 후보 문장 전체를 실제 workspace를 수정하지 않고 분석한다. 내부 namespace write/unknown-effect call/동적 구성 요구가 없고 필요한 proof·guard 및 target capability가 닫힌 구간만 compiled route에 수용한다. 유일하게 허용한 terminal write도 실제 commit은 실행 후다. 요구가 남으면 원 source를 **처음부터 아직 실행하지 않은 상태로** 지원되는 runtime route에 넘기거나 compiler coverage 거부를 반환한다. Runtime route의 실제 capability 확인은 필수이며 C fallback은 없다. Guard miss는 route 거부이지 J Domain/Value 오류가 아니다.

**v0에서 하지 않는 일:** prefix 실행 후 parser 중간 재개, 효과 이후 문장 전체 replay, 계산하지 않은 operand를 concrete JEntity로 운반, backend가 미지원인 call을 존재하지 않는 fallback에 위임. 따라서 처음에는 `a+a=:2` 같은 구간을 native compile하지 않아도 되지만, 지원되는 runtime 경로에서는 반드시 J 결과 `4`, 최종 `a=2`를 낸다. 이후 그 form의 compiler 수용은 ordered symbolic namespace/write와 verifier가 구현됐을 때 확장한다. 일반 continuation은 별도 후속 capability이며 state snapshot+exactly-once 테스트 전에는 활성화하지 않는다.

**수용은 세 층으로 분리한다.** 다음은 기존 NP/DB/M3→M4 게이트의 테스트 상세이며 경쟁 checklist가 아니다.

| 층 / 연결 게이트 | 통과해야 하는 관찰 | 반드시 실패시킬 단일 변이 | 현재 증거 |
|---|---|---|---|
| Analysis boundary / NP-02/04, DB-C | pure apply·unknown shape는 지연 graph, computed constructor·non-final write는 boundary, 분석 중 binding commit 없음 | deferred operand를 literal로 위장하거나 중간 write 뒤 이전 catalog read로 계속하기 | `tests/static_analysis.rs`의 기존 14 + 신규 3 회귀. API는 아직 구조화된 demand를 반환하지 않음 |
| Plan admission verifier / NP-03/05 | NAME/POS/constructor·입력·call effect 증거의 범위/시점, complete source mapping, 실행 전에 해소된 요구, error order와 수명, 지원되는 route | 하나의 guard 삭제, 다른 locale/frame stamp로 교체, nested NameRef dependency 누락, order edge 제거, 미해소 demand를 Ready로 변경 | **미구현/UNRUN**. `Plan::verify`의 구조 검사나 catalog version만으로 이 게이트를 대체할 수 없음 |
| C↔Rust 실제 compiled route / NP-07, DB-E, M4 | 값의 type/shape/data, 결과 POS·첫 오류, 효과 전후 namespace·alias와 source 관찰이 C/runtime/compiled 경로에서 동일 | `a+a=:2`를 3으로 계산, missing-left 오류를 right Length보다 먼저 노출, committed count를 0 또는 10으로 만들기, rebind 후 stale 함수 실행 | C↔현재 runtime 일부는 실행. **compiled route 및 NAME event/내부 effect trace 차분은 UNRUN** |

**oracle fixture를 먼저 고정했다:** [boundary audit](tools/name_compatibility_audit.py)의 `--boundary-fixtures-only` 및 [Windows 결과](reports/semantic-boundary-windows.json). 다음 10 fixtures × C j64/AVX2 × Rust direct/semantic-reference = **40건: 36 matched, 4 unsupported_gap**. 두 runtime 경로는 컴파일 경로가 아니다. Source와 DLL asset pin은 별개이고 실제 DLL/binary hash를 기록한다.

| fixture / 핵심 관찰 | C와 현재 runtime | compiler 수용에서의 요구 |
|---|---|---|
| `b=:a+a*a`, 기존 a 유지 | `[0,2,6]`, a=`[0,1,2]` 일치 | graph→A3에서 중간 배열 미실행; alias 보존; 실제 route 검증 |
| `n=:4` 뒤 `i.n` | `[0,1,2,3]` 일치 | unknown result shape는 parser value demand가 아님 |
| `(+/"1) i.2 3` / `(+/"(1+0)) i.2 3` | 둘 다 `[3,12]` 일치 | literal은 구성 가능, computed operand는 현재 analysis boundary |
| `a=:1` 뒤 `a+a=:2` | 결과 4, a=2 일치 | v0 중간 namespace write 경계; 기존 a=1로 parse를 계속하지 않음 |
| `adv=:1 : 'a=:u'` 뒤 `a+2 adv` | 결과 4, a=2 일치 | modifier 실행이 후속 lookup을 바꾸는 경계 |
| `missing+(1 2+1 2 3)` | **Length 오류**, missing Value보다 먼저 | semantic error order를 유지; admission miss가 J error를 먼저 발생시키지 않음 |
| `count=:0`, `adv=:1 : 'count=:count+u'`, `(1 2+1 2 3)+5 adv`, `count` | Length 오류 후 **count=5** 일치 | effect-before-error 보존, 실행 누락으로 0 또는 replay로 10을 만들면 실패 |
| `". '1+2'` | C=3, Rust Unsupported | dynamic semantic route 자체 미지원; 자동 fallback 성공 주장 금지 |
| `f=:+`, `g=:f`, `f=:7`, `g 3` | Domain 오류 일치 | POS/target guard와 정확한 semantic error/route-miss 구분 |

여기서 검증한 것은 **경계 입력에 대한 기존 runtime의 외부 동작**과 **현재 분석기의 중단/지연 동작**이다. 재개 state, typed demand API, admission verifier 및 compiled route가 통과했다고 기록하지 않는다. 후속 구현의 첫 완료 기준은 pure 입력 한 구간에서 `Deferred Program -> Graph -> A3 -> admitted CPU execution`이 kernel-call counter/할당 계측·C 값 비교로 닫히는 것이며, 이후 위의 한-변이 부정 테스트로 각 경계를 확장한다.

이번 Windows 검증: `static_analysis` **17/17 default 및 17/17 portable**, Python audit 분류 **6/6**·oracle protocol **9/9**, fmt·clippy(`--all-targets --all-features -D warnings`)·diff 검사 통과. 전체 Rust/upstream J suite·Linux·GPU·GitHub CI는 실행하지 않았다. NAME 처리의 위험을 후속 최적화에 떠넘기지 않기 위해, **admission verifier가 구현되고 위 부정 테스트를 통과하기 전에는 NAME 관찰 metadata만으로 특수화 실행을 수용하지 않는다.**

근거: [FOUNDATIONS §21/§33](FOUNDATIONS.ko.md), pinned C [p.c parser](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c), [sc.c NAME constructor](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sc.c#L364), [cx.c return fix](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L684), [af.c reconstruction](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/af.c#L193). DB0–DB7의 분류·이행 순서는 RustJ 설계 판단이며 upstream의 완성된 guard 시스템을 복제했다는 뜻이 아니다.

<a id="read-graph-execution"></a>
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

##### 현재 구현 경계의 정본

`j_graph_ir::classify_function()`이 `GraphForm/GraphHint`를 만들고, execution lowering은 이를 canonical `logical_ir::Plan`의 opportunity/provenance로 투영한다. `Operation.j_origin`이 graph→execution 대응을 보존한다. `CompilationAnalysis`는 `j_graph`와 `logical`을 함께 반환한다.

세부 구현과 남은 graph 연구는 [A1.5](#j-graph-implementation-checklist), 단일 execution IR cutover는 [M1](#architecture-migration-checklist), 전체 상태는 [§12](#current-implementation-status)에서 추적한다. adjoint/VJP expansion은 여전히 schema/연구 단계이며 실제 AD transform이나 병렬 실행 완료가 아니다.

##### 중요한 불변조건

1. StructuralOpportunity는 optimization hint보다 강한 **semantic-topology provenance**지만, physical schedule은 아니다.
2. Opportunity가 존재한다고 fusion/parallelization legality가 자동 성립하지 않는다.
3. J observable evaluation order는 opportunity 안에서도 잃지 않는다. 병렬 실행은 별도 proof가 있어야 한다.
4. basis normalization 때문에 Atop/Hook/Fork provenance를 버리지 않는다.
5. generic DAG pattern matching은 추가 기회를 찾는 보조 수단일 수 있지만, J 문법이 이미 준 topology를 다시 찾는 주 경로가 되어서는 안 된다.
6. hardware resource 정보는 opportunity discovery에 필요하지 않지만, 실제 fusion/parallel region 확정에는 필요하다.
7. 이 층의 목적은 J 문법의 표현을 미학적으로 보존하는 것이 아니라 **optimizer search space를 줄이고 materialization/lifetime/parallel 후보를 일찍 제공하는 것**이다.


<a id="syntax-graph-hints"></a>

#### 4.1.2 J syntax에서 얻는 graph 최적화 힌트 분류

검토일: 2026-10-02. 범위: primitive의 산술·배열 구현 정보를 사용하기 전에, J의 구문 구조에서 얻는 graph 정보를 분류하고 검토한다.
일반 컴파일러 정보 분류 → syntax로 해석 가능한 분류 → J 구문 대응 → graph 정보와 최적화 후보의 순서로 정리한다.
문서·설계 검토이며 구문 지원이나 실행 최적화를 새로 구현한 것은 아니다.

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

##### 4.1.2.5 Binding 정보와 일반 graph 대비 추가 가치

명시적 `u f.`는 이름 참조를 당시 referent로 고정하는 의미를 제공하지만, 순수한 grouping 문법만의 사실과는 구분한다. `$:` 포함 부분은 Fix의 예외다. 사용자가 쓰지 않은 Fix를 자동 적용해 동적 이름 의미를 바꾸지 않는다. [J Fix](https://www.jsoftware.com/help/dictionary/dfdot.htm)

일반 배열 graph가 leaf 호출과 데이터 edge만 가진다면 J의 rank/partition/반복 영역 및 바인딩 의미는 추가로 보존할 정보가 된다. 이미 fan-out edge를 가진 graph에도 **fork syntax가 parallel 후보를 제공한다**는 사실은 같다. 다만 graph의 표현력이 동일하다면 새 정보가 더 생기는 것은 아니다. J syntax는 graph 구성과 구조 인식의 source다.

현재 가장 중요한 설계는 leaf kernel 목록으로 일찍 풀지 않고 syntax에서 유래한 fork/hook/compose/cell/partition/loop/selection 구조를 보존하는 것이다. ‘syntax origin’ 기록은 구문이 어떤 최적화 후보를 제공했는지 설명하는 진단에도 사용할 수 있다.

##### 4.1.2.6 다른 배열 컴파일 프레임워크와의 대조

[정본 내 프레임워크 비교](#framework-graph-hints)에 JAX/XLA, MLIR Linalg, TVM Relax, Halide의 공식 설명과 소스를 바탕으로 F1–F9를 기록했다. 추가 힌트는 fork 재결합 영역, consumer 수, 입력 수명, cell mapping, window 공유 영역, loop invariant, 값/effect live-out이다. 각 항목은 출처와 J 적용 추론을 분리한다.

##### 4.1.2.7 RustJ에 반영할 설계와 검증 체크리스트

제안하는 처리 순서:

```text
품사/binding 확인과 parsing
  → syntax 구조에서 graph + 최적화 후보 생성
  → leaf 계약·효과·오류·alias로 적법성 검사
  → shape/layout와 비용으로 실행 전략 선택
```

Graph 노드/영역의 설계 후보: `Fork`, `Hook`, `Compose`, `CellApply`, `BoundOperand`, `LoopRegion`, `SelectionRegion`. 결과 선택은 `Fork` 가운데 verb의 계약을 해석한 값 사용 정보로 둔다. 실제 구현 API 이름은 아니다.

각 후보에 source span, syntax origin, 적용 valence/rank, 데이터 edge, 효과 검사 상태를 남긴다. 예를 들어 `ParallelCandidate(origin=Fork)`는 **병렬 후보를 발견한 상태**이지 `ParallelSafe`나 실행 계획 확정 상태가 아니다. 이름이 verb인지 noun인지 모르는 상태에서 토큰 세 개만 보고 fork를 확정하지 않는다.

2026-10-02 프레임워크 비교 당시의 코드 대조 기준(`89b87b8`)에서는 `src/j_graph_ir.rs`가 `GraphForm::Pipeline/Hook/Fork/Reduce/PrefixInfix/Rank`와 `GraphHint::ParallelBranchCandidate` 등을 이미 보존한다. `classify_function()`은 leaf kernel 구현을 분석하기 전에 fork에서 병렬 후보를 만든다. 이 사실은 실행 병렬화 완료를 뜻하지 않는다. Bond/Compose/Power/Agenda/Fix의 일반 지원과 S1–S12 전체 최적화는 별도로 검증해야 한다. 자세한 현재 상태는 A1.5를 따른다.

- [x] 사용자 예의 parallel 후보가 바깥 fork syntax에서 나온다고 명시했다.
- [x] leaf를 black box로 두고도 얻는 syntax 힌트 S1–S12를 검토했다.
- [x] 일반 compiler 정보 분류와 J syntax 대응을 분리했다.
- [x] modifier 적용 구조와 leaf kernel 힌트를 구분했다.
- [x] 후보 생성과 적법성·비용 판단을 별도 단계로 정리했다.
- [ ] 구문 지원 시 named verb만으로 fork/hook/compose graph를 검사하는 회귀를 추가한다.
- [ ] fork 두 branch 사이에 불필요한 반환값 의존 edge가 생기지 않는지 확인한다.
- [ ] 사용자 예의 바깥 fork와 내부 hook이 별도 구조로 보존되는지 확인한다.
- [ ] effectful branch에서도 parallel 후보는 생성되되 안전성 실패 후 실행 순서가 보존되는지 검증한다.
- [ ] rank/valence·이름 재정의·Agenda·Power의 영역 의미를 검증한다.
- [ ] 후보 적용 전후 값·dtype·shape·오류·효과 순서를 Windows 네이티브에서 비교한다.

기존 M1–M6/frontend 이행 순서와 G1–G5 배열 설계 우선순위가 실행 계획의 기준이다. 이번 검토는 그 계획을 보강하며 조사한 모든 syntax 구현을 선행 조건으로 추가하지 않는다. 실제 CUDA 보류는 유지한다. 이번에는 문서만 수정했으며 Rust/C 실행 테스트, 성능 측정, 새 parsing 또는 병렬 실행 구현은 수행하지 않았다.

<a id="framework-graph-hints"></a>

#### 4.1.4 Candidate lifecycle와 proof-discharge contract

J Graph IR이 candidate를 발견한 뒤 실제 transformation으로 commit하기까지의 상태를 **하나의 `selected` bool로 표현하지 않는다.** legality, target feasibility, resource feasibility, cost, selection은 서로 다른 질문이며 서로 다른 evidence를 가진다.

개념적으로 candidate는 다음의 **직교한 evidence 축**을 가진다.

~~~text
CandidateEvidence
  provenance        Verified | Stale/Invalid
  equivalence       Unknown | Proven | Disproven | Guarded(GuardId)
  semantic_legality obligation별 Unknown | Proven | Disproven | Guarded(GuardId)
  target_feasibility Unknown | Supported | RequiresFacts | Unsupported
  resource_state    Unknown | Symbolic | Resolved | ExceedsHardLimit
  cost_state        Uncosted | Estimated(CostEstimate)
  selection         Unselected | Selected | Rejected(reason)
  lowering_state    NotLowered | Lowered(Transform/Route identity)
~~~

이 축을 하나의 선형 enum으로 저장할 필요는 없다. planner/UI가 다음과 같은 **derived lifecycle summary**를 만들 수는 있다.

~~~text
Discovered
   ↓ source/provenance verification
AwaitingProofs
   ├─→ Illegal
   └─→ Legal or GuardedLegal
          ↓ hard target/resource feasibility
       Feasible
          ↓ cost evidence
       Costed
          ↓ compatibility + global/local choice
       Selected / Rejected
          ↓ committed lowering
       Lowered
~~~

단, 이 화살표는 분석 pass의 실행 순서를 강제하지 않는다. resource/work-depth/cost 분석은 legality proof가 끝나기 전에도 **speculative side analysis**로 계산할 수 있다. 금지되는 것은 필요한 legality proof가 끝나기 전에 candidate를 실행 plan으로 **commit**하는 것이다.

##### Evidence owner

| Evidence / 질문 | 주 소유자 | 의미 | selection에 대한 규칙 |
|---|---|---|---|
| source topology / provenance | J Graph verifier + candidate registry | candidate가 현재 source/region/rule version에서 실제로 유도되었는가 | stale provenance면 즉시 폐기 |
| algebraic equivalence | rewrite/scan/fusion rule의 witness validator | source와 replacement/composite identity가 같은가 | 필요한 equivalence가 Unknown이면 commit 금지 |
| rank/cell/assembly | Execution semantic facts + candidate legality checker | CellApply/assembly/error 의미가 보존되는가 | Proven 또는 effect 이전 Guard 필요 |
| numeric / tolerance / reassociation | primitive/derived numeric contract | overflow, `!.`, tolerance, floating-order contract가 보존되는가 | semantic relaxation 없이는 임의 reassociation 금지 |
| effect / error ordering | effect/error/speculation analysis | observable write/error 순서를 바꾸어도 되는가 | Disproven이면 candidate illegal; guard가 effect 뒤라면 사용 불가 |
| fanout / retention / alias | graph use/liveness + alias/storage facts | producer 복제, retained value, reuse가 합법인가 | external use를 잃거나 alias proof 없으면 해당 transform 금지 |
| resource / work-depth | `j_graph_resource` / `j_graph_work_depth` | symbolic state, internal traffic, work/depth, hard resource need | cost와 분리. hard target limit 초과만 feasibility 거부 근거 |
| target/lowering capability | LoweringRegistry × resolved target | 해당 op/region을 실제 realization으로 내릴 수 있는가 | source-basis support만으로 fused/composite legality를 추정하지 않음 |
| empirical profitability | CostProfile / planner | legal candidates 중 무엇이 유리한가 | legal candidate를 단지 느리다는 이유로 semantic invalid로 만들지 않음 |
| final compatibility/selection | Schedule/Transform planner | 후보들의 겹침·순서·route를 함께 선택 | selected set 전체가 상호 호환되어야 함 |

##### Guarded legality

`Guarded(GuardId)`는 “증명하지 못했지만 일단 실행”이라는 뜻이 아니다. 다음 조건을 모두 만족해야 한다.

1. guard가 transformation이 의존하는 fact를 실제로 검증한다.
2. guard는 해당 region의 observable effect보다 먼저 실행된다.
3. guard miss의 대체 route가 명시되어 있다.
4. miss 후 원래 J semantics를 재실행해도 중복 effect가 생기지 않는다.
5. guard identity/provenance가 specialization/candidate cache와 연결된다.

effect가 이미 commit된 뒤에는 guard miss/Unsupported를 이유로 source region을 자동 replay하지 않는다.

##### Candidate overlap과 selection

candidate overlap은 곧바로 오류도 아니고 곧바로 composition 가능도 아니다. v0에서는 보수적으로 다음을 적용한다.

- 같은 source operation을 **대체**하는 두 rewrite는 동시에 select하지 않는다.
- rewrite와 fusion이 같은 source operations를 겹쳐 소유하면 registered compatibility/composition rule이 없는 한 동시에 commit하지 않는다.
- 둘 다 유용할 수 있으면 먼저 하나를 선택·적용해 **새 graph/version**을 만든 뒤 그 결과에서 candidate discovery를 다시 수행할 수 있다.
- candidate의 profitability 비교 때문에 원 source graph/witness를 파괴하지 않는다.
- `CandidateId`/analysis-local index는 특정 Plan/graph version에 귀속되며 source/registry version이 바뀌면 재검증한다.
- selection 결과는 semantic IR 자체가 아니라 별도 Transform/Schedule plan이다.

##### 현재 코드와 목표 계약의 대응

현재 구현은 이 전체 lifecycle의 일부만 갖는다.

~~~text
GraphRewriteCandidate
  provenance + equivalence witness
  target-independent

FusionCandidate
  proof obligations
  legality = Unknown
  resource_transfer_proven = false
  selected = false

RewritePlanningReport
  TargetUnsupported / NeedsCallFacts / NeedsResourceFacts / ReadyForCosting

FusionReadinessReport
  AwaitingSemanticProofs
  fused target query deferred
  selected = false
~~~

아직 공통 `CandidateEvidence`/`ProofBundle`, obligation별 proof discharge 결과, compatibility-aware selection plan, committed transform identity는 구현되어 있지 않다. 위 타입 이름은 목표 개념을 설명하며 현재 API 완료를 주장하지 않는다.

##### 구현 시 분리 순서

현재 M2 frontend 우선순위를 바꾸지 않는다. optimizer 단계에 착수할 때는 다음처럼 **한 번에 한 의미 + 한 verifier/test**로 추가한다.

1. proof 결과의 공통 state/provenance 표현 + stale evidence verifier
2. fusion obligation별 discharge 결과를 기록하되 selection은 하지 않음
3. rewrite/scan 후보에도 동일한 legality view를 투영
4. target hard-feasibility와 semantic legality를 합치지 않는 readiness view
5. candidate compatibility/overlap 검사
6. costed candidate set에서 별도 SelectionPlan 생성
7. selected candidate만 committed transform/lowering으로 넘기고 source provenance를 검증

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

#### 4.4.1 분석 fact는 typed lattice로 관리하고 semantic error와 분리한다

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

<a id="read-physical"></a>
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

여기서 일반 dataflow 실행 의미는 fork 표기 없이도 표현할 수 있지만, **원래 source가 fork/hook/@:였다는 topology provenance는 optimization 정보로 계속 보존한다.** §4.1.1의 StructuralOpportunity sidecar가 pipeline/branch-join/live-across 정보를 명시적으로 운반하므로 optimizer가 generic DAG에서 이를 다시 pattern-match할 필요가 없다. 진단·debug provenance이기도 하지만 그것에 한정되지 않는다.

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

#### 5.2.1 M4 최소 PhysicalPlan v0 contract

M4의 첫 CPU vertical slice를 구현하기 전에 **compiler plan의 resource identity와 runtime handle을 분리한 최소 schema**를 문서로 고정한다.

가장 중요한 구분:

~~~text
Logical ValueId
    ≠
PlanBufferId        // compiler PhysicalPlan 안의 symbolic/planned storage slot
    ≠
physical::BufferId  // runtime BufferRegistry가 발급하는 checked handle
    ≠
raw address
~~~

현재 `physical::BufferId`는 registry identity + slot + generation을 가진 **runtime handle**이다. compiler가 lifetime/reuse를 계획하기 위해 사용하는 slot identity와 그대로 동일시하지 않는다. 실제 구현명은 달라질 수 있지만, plan-time identity와 executor-time lease/handle의 계층은 분리한다.

또한 하나의 planned buffer 위에 여러 physical view가 존재할 수 있으므로 buffer identity와 view identity도 분리한다.

~~~text
PlanBufferId
  storage/resource identity

PhysicalViewId
  PlanBufferId
  logical shape
  strides
  offset
  encoding
  access mode
~~~

##### v0 PhysicalPlan

첫 M4 범위는 **verified single-block pure-array CPU region + 필요한 SemanticCheck**로 제한할 수 있다. assignment/name mutation 같은 stateful effect는 첫 slice에서 RuntimeSemantic region에 남겨도 되며, 이것은 J language restriction이 아니다.

개념 schema:

~~~text
PhysicalPlan
  source_logical_schema / provenance
  resolved CPU target
  planned buffers
  physical views
  ordered/dependency-aware ops
  outputs

PhysicalOp
  BindInput
  Check
  View
  Materialize
  Kernel
  Return

future / non-M4:
  Transfer
  Sync / AsyncToken
~~~

각 op의 역할:

- **BindInput** — logical input/read value를 executor가 가진 runtime storage/lease와 연결한다. deep copy를 뜻하지 않는다.
- **Check** — A3 `SemanticCheck`를 J error kind/origin/order와 함께 실행한다. optimizer 편의를 위해 kernel 안으로 숨기거나 제거하지 않는다.
- **View** — transpose/reverse/slice 등 합법한 metadata-only physical view를 만든다. 새 backing allocation을 의미하지 않는다.
- **Materialize** — consumer requirement나 layout/alias 조건 때문에 logical atom order를 보존한 실제 copy/packing을 만든다. 이유/provenance를 남긴다.
- **Kernel** — 이미 선택된 `ParameterizedLoweringRecipe/RealizationFamily`를 입력/output view에 적용한다. rank/hook/fork/fusion legality를 executor에서 다시 판단하지 않는다.
- **Return** — 최종 physical view/ownership을 logical result로 넘긴다. temporary를 output으로 잘못 재사용하지 않게 ownership을 확정한다.
- **Transfer/Sync** — GPU/mixed-route에서만 필요할 수 있으며 M4 CPU v0의 필수 op가 아니다.

##### planned buffer와 lifetime

각 planned buffer는 최소 다음 정보가 필요하다.

~~~text
BufferRequirement
  memory space / CPU class
  encoding
  extent or size expression
  alignment requirement
  ownership class: input | temporary | output
  def / physical uses / last use
  optional reuse witness
~~~

M4 첫 slice가 fully-resolved CPU extent만 지원해도 된다. dynamic extent 지원이 없다는 사실을 J semantic restriction으로 올리지 않고 route capability로 둔다.

buffer reuse는 별도 semantic transform이 아니라 physical decision이다. 같은 `PlanBufferId` 또는 storage slot을 재사용하려면:

1. 이전 physical value의 last use가 끝났고,
2. outstanding view/lease가 그 storage를 관찰하지 않으며,
3. encoding/size/alignment/memory-space requirement가 맞고,
4. alias/destination contract가 허용하며,
5. J-visible effect/error order를 바꾸지 않는다는

reuse witness가 필요하다.

##### PhysicalPlan verifier

executor는 invalid plan을 추측해서 고치지 않는다. 최소 verifier는 다음을 검사한다.

- 모든 buffer/view/op id가 유효하고 use가 definition 뒤에 있다.
- 모든 view의 shape/stride/offset span이 backing extent 안에 있고 encoding/dtype contract가 맞다.
- buffer가 bind/allocation되기 전에 사용되지 않는다.
- `Check`의 ordering edge와 source origin이 A3 SemanticCheck에서 유도되었고 누락/중복되지 않는다.
- Kernel의 chosen realization이 resolved target과 lowering capability에 맞고 필요한 input/output view contract를 만족한다.
- write 가능한 overlapping views가 proof 없이 동시에 사용되지 않는다.
- Materialize가 logical atom order/value semantics를 보존한다.
- reuse는 last-use + alias/ownership witness 없이는 허용하지 않는다.
- Return은 유효한 output ownership/view를 가리키고 temporary lifetime 이후 dangling view를 만들지 않는다.
- M4 pure-region plan에는 숨은 namespace/write effect가 없다.

##### error / cleanup contract

- `Check` 실패는 원래 J semantic error로 보고한다.
- Kernel/library 자체의 implementation failure를 임의의 J Domain/Rank/Length error로 바꾸지 않는다.
- plan 실패 시 executor-owned temporary lease/resource는 정리하되 caller-owned input은 파괴하지 않는다.
- first M4 pure slice에서는 namespace assignment commit을 Physical Executor가 소유하지 않는다. stateful write를 native route에 넣을 때 별도 effect/commit contract를 추가한다.
- observable effect가 commit된 뒤 transparent replay하는 fallback은 금지한다.

##### M4→M5 구현 선행조건과 검증 프로토콜 (2026-10-07, **설계만 확정**)

**범위:** 실제 PhysicalPlan/Executor/View/Kernel/Transfer를 지금 구현하지 않는다. 본 절은 기존 §5.2.1의 계약, §17 M4 구현 항목, HE-01~HE-09, IO-20에 적용할 **수용 조건과 검증 증거**다. 별도 Parallel IR, Data Movement IR, 중복 체크리스트는 만들지 않는다. 기존 Draft PR #4의 제한적인 identity 구현은 검증된 전체 M4 경로로 취급하지 않으며 완료 수용과 구별한다.

**컴파일러 경계의 의무와 검증 방법**

| 경계 | 실행 전 필요한 조건·증거 | 수용/거부 검증 | 조건 미충족 시 |
|---|---|---|---|
| **A3 → Route/Physical** | \`logical_ir::Plan::verify\` 성공, schema/primitive-registry/source origin 일치, 한 region 내 포함된 모든 value/check/effect/order edge 목록, 해당 route의 capability·guard/witness 확보 | A3 op 하나씩 추적하는 source→physical mapping; plan에서 Check 하나 삭제·복제·재배열, producer 위조, stale version/guard 위조 시 verifier가 **반드시 거부** | Unknown을 Supported로 치환하지 않음. 해당 region을 합법한 reference/runtime route로 유지하거나 Unsupported |
| **장치·메모리 독립성** | \`ExecutionDevice\`, \`MemorySpace\`, intra-device schedule은 서로 독립한 물리 결정. plan-time \`PlanBufferId\`/view와 runtime \`BufferId\`/lease, logical \`ValueId\` 분리. M4는 CPU+Host+zero-transfer | logical/semantic IR에 thread 수·CUDA stream·buffer address가 없는지 구조 검토; 서로 다른 메모리 위치와 동일 execution device를 **미래 확장 가능**하게 식별; M4 plan에는 Transfer/Sync가 없음을 검사 | 혼합 실행 능력이 없으면 단일 CPU 경로만 지원; J 의미를 바꾸지 않음 |
| **BindInput** | 실제 dense noun의 dtype·shape·encoding·extent·version/owner가 source value와 일치, readonly lease 유효, alias 여부 추적, implicit deep copy 없음 | 바뀐 binding version·stale buffer generation·미등록 BufferId·타인 registry·잘못된 encoding을 각각 하나씩 위조하여 reject; 입력 버퍼 원본 불변 확인 | 입력을 파괴하거나 임의 재바인딩하지 않고 route 거부 |
| **View** | 대상 primitive/Rank·axis 의미가 증명됨, logical shape/atom order 유지; \`shape×stride+offset\`의 최소/최대 backing byte 범위를 부호 있는 checked 연산으로 계산하고 encoding/alignment 적합성 확인. 빈 frame과 빈 cell은 구분 | scalar, rank 1/2/3, \`0 3\`·\`3 0\`·singleton, negative stride, transpose/reverse, overflow·zero stride·OOB 위조. **View는 실제 메모리를 복사하거나 쓰지 않았는지** 확인 | 불확실한 view를 실행하지 않음; 합법한 Materialize/다른 route를 별도 선택 |
| **SemanticCheck / 오류** | A3 Check마다 유일한 provenance, 실행 위치, 의존성, 원래 J error kind 및 error precedence. Guard는 관찰 가능 effect 전 평가; 복구/handler가 오류를 볼 수 있음 | 두 개 이상 Check의 순서 변경, 한 Check 누락·중복, invalid Rank/Length/Index, effect/NAME rebinding/try-catch 관찰 사례. Reference executor와 **오류 종류·첫 오류·부수효과 횟수·상태** 비교 | 물리화 대상 제외; arbitrary first-thread error 또는 GPU trap→J error 임의 변환 금지 |
| **Kernel (첫 Add)** | \`ParameterizedLoweringRecipe\`/registered realization이 resolved target에 합법, valence·rank/cell·prefix agreement·J 타입 승격·overflow/fit·alias·stride 및 전처리 Check 충족. Logical IR의 Parallel 축은 동시 실행 허가 증거 아님 | scalar/vector/matrix, 서로 다른 dtype, bool/int/float, integer overflow/promotion, NaN/Inf, length error, empty arrays, SIMD tail, stride mismatch. 등록하지 않은 recipe 또는 shape/target capability mismatch는 verifier reject; **기존 semantic interpreter를 호출한 결과를 native kernel 통과로 세지 않음** | 기존 reference/지원 route 선택, kernel 미지원은 J Domain error가 아님 |
| **Materialize / Return** | view의 **J 논리 원소 순서**로 allocation·copy, output type/shape 보존; backing/lease와 output ownership, temp last-use 확인 | transpose/reverse의 physical address order와 J atom order 비교, empty/boxed/sparse(지원하지 않는 경우 reject), output dangling/temporary 조기 해제, read-only input mutation 및 조립 에러 비교 | incomplete result를 반환하지 않음; 실패 시 호출자 입력 보존 |
| **Reuse / resource** | def/use/last-use, outstanding view/lease, non-overlap proof, encoding/size/alignment/memory-space 호환, check/effect precedence, budget 상한이 모두 충족 | reuse 중 live alias·overlap write·buffer size 부족·allocation 실패·exception cleanup 등 각 invariant 하나씩 위조; reclaimed resource에 대한 subsequent use 거부 | reuse 비활성화, 안전한 별도 버퍼 사용 또는 route 거부 |
| **CPU↔GPU / I/O 후속(M5+)** | region/device placement, memory residency/version, transfer bytes, ready/completion, sync, ownership/lifetime, host effect/error 의존성 및 capacity. 외부 backend도 계약을 유지 | 가상 plan verifier 수준에서 stale data/transfer 누락/copy-before-producer/read-before-ready/transfer-after-free/중복 commit을 reject. 실제 GPU 결과 동등성·성능은 **검증 가능한 실장치와 명시적 재개 조건** 이후만 수행 | CPU/단일 장치 계획만 합법; 복합 장치 실행을 완료로 주장하지 않음 |

**공통 검증 절차와 완료 판정**

1. **A3 정상 계획 생성·검증 → Physical 계획 생성·검증 → 제한된 CPU 실행** 순서를 지키고, 검증되지 않은 plan은 실행기까지 도달할 수 없게 한다. 정상 사례를 복제해 **불변 조건 하나만** 훼손하는 음성 검증을 기본 패턴으로 삼는다. verifier 실패 이유와 source op/provenance를 기록한다.
2. **3자 의미론 비교:** 동일 입력에 대해 (a) jsource C 기준 실행, (b) RustJ \`logical_executor::execute_closed\`/해당 의미론 기준 경로, (c) 신규 Physical Executor의 값·dtype·Shape·원소 순서·J error class/precedence를 비교한다. C 기준의 해당 기능이 현지 환경에 없으면 미검증으로 남기며 RustJ 내부 2자 비교로 대체 완료 처리하지 않는다.
3. **범위·부정 사례:** zero-frame Rank virtual fill vs positive-frame empty cell, 복합 CellApply 결과 type/shape join, boxed/sparse, tolerance/fit, numeric reassociation, late NAME/assignment/effects, 예외·resource cleanup은 독립 regression으로 분리한다. 미지원 subtype은 **지원 범위 밖**으로 보고하며 J 문법/의미론 제한으로 바꾸지 않는다.
4. **기계적 테스트:** 구현 시 \`cargo fmt --check\`, default/portable \`cargo test\`, \`cargo clippy\`와 재현 가능한 Linux/Windows 경로를 실행하고 커밋 SHA·플랫폼·스레드/feature·케이스 수·passed/failed/ignored를 기록한다. 실행하지 않은 테스트와 CI 결과는 성공으로 보고하지 않는다.
5. **비용은 의미론 통과 후:** Sequential CPU를 zero-transfer baseline으로 두고, 실행 시간 외 bytes read/written/moved, allocation/peak residency, kernel/transfer launch, sync, guard misses, fallback 횟수, critical path를 별도 측정한다. \`ResourceEstimate\`와 \`CostEstimate\`와 실측 \`CostProfile\`을 혼합하거나 Unknown을 0으로 두지 않는다.
6. **M4 수용 게이트:** Check·View·Kernel(Add)·Materialize 중 최소 하나의 **실제 compiler-native 연산 경로가 필요 Check를 포함하여** A3→verified PhysicalPlan→CPU에서 실행되고, 음성 verifier와 jsource differential이 통과해야 한다. 단일 literal \`BindInput→Return\`, 설계 문서 또는 테스트 소스만으로는 M4 전체/HE-01을 완료하지 않는다. M5의 mixed placement와 M6 GPU·async 실구현은 M4의 선행 필수 조건이 아니다.

**상태 (2026-10-07):** 본 절의 **조건·검증 프로토콜만 문서 확정**. 모든 새로운 실행/성능 수용은 아직 미검증이며, M4/HE-01~09 미완료 체크박스를 유지한다. 구현 재개 시 각 수용 결과를 기존 §17/HE 체크리스트에 증거 링크로 기록한다.

**연계 체크리스트:** [sw-MLPL 실행·검증 참고 SW-03/04/06](#sw-mlpl-reference-checklist)는 위 M4 수용 게이트의 보조 구현 패턴이며, 이 문서 추가만으로 체크박스를 닫지 않는다.

##### 기존 아키텍처 대조 감사: M4 준비도와 검증 공백 (2026-10-07, 설계 검증 전용)

**결론:** J Graph → verified A3 → target-dependent lowering 후보 → Physical Representation이라는 기존 계층은 **그대로 확장 가능**하다. CPU 전용 Parallel Planner/IR, 새로운 Data Movement IR, GPU 실구현 선행은 필요하지 않다. 단, **A3 구조 검증**, **후보별 적법성 평가**, **원본 A3에서 Physical Plan까지 의미 보존의 증명**은 서로 다르다. 앞 단계의 성공이 뒤 단계의 완전한 실행 허가는 아니다. 아래는 현재 main의 실제 코드와 기존 테스트를 대조한 **정적 감사**이며 신규 테스트를 실행한 것이 아니다.

| 감사 ID / 소유자 | 실제 코드·테스트에서 확인 | 공백과 단계 구분 | 추후 검증 조건 |
|---|---|---|---|
| CA-01 / A3 구조·의미 | src/logical_ir.rs의 Plan::verify는 schema·registry provenance, single block, SSA producer/use-before-def, order_after가 앞선 op를 가리키는지, Check가 result를 만들지 않는지, 일부 rank/shape를 검증한다. tests/logical_ir.rs에 관련 회귀가 있다 | **M3→M4의 의미론 증명 미완료.** A3 verifier는 모든 effect/error summary를 J 원본에서 독립 재계산하거나, 삭제된 Check가 없는지 Physical Plan까지 대조하지 않는다. IterationAxisKind::Parallel은 실행 허가가 아니다 | 정상 A3 하나를 복제하여 Check 삭제·중복, order_after 삭제, effect/possible_errors/speculation의 근거 없는 완화, source origin·binding version 위조를 각각 주입한다. 구조 위조와 **구조는 통과할 수 있는 의미론 위조**를 구분하고, 후속 선택·검증에서 모두 차단 |
| CA-02 / Lowering | src/lowering.rs의 Requirement::satisfied·legal_candidates는 CPU/GPU family·feature, Pure, NoObservableError, EvaluationOrderRelaxed, known access/rank, 재결합 허용을 판정한다. tests/lowering.rs는 안전 조건이 없으면 SIMD/GPU·tree reduce를 닫는 검사를 포함 | **M3 증거 연결 미완료.** 후보 등록·CallOp에 적힌 사실이 곧 guard discharge/전체 Physical Plan의 적법성 인증서는 아니다. ReferenceSequential 후보는 실제 native CPU kernel 완성을 의미하지 않는다 | 근거 없는 effect/error/guard/witness 완화가 최종 선택에서 거부되는지 확인. target·valence·shape·stride·recipe와 proven semantic facts의 교차 검증이 필요 |
| CA-03 / RoutePartition | lowering.rs의 partition_plan은 ValueOnly/PureArray/SemanticCheck/RuntimeSemantic가 같은 연속된 op를 구간으로 묶는다 | **RouteBoundary verifier 미구현(M3→M4).** 구간 class/range만으로 source op 전수 대응, live-in/out, zero-result Check, effect-live/order edge, guard-before-effect, commit frontier, replay 안전성은 증명되지 않는다 | source A3 모든 op의 정확한 대응(적법한 rewrite/fusion에는 witness 필요), Check·effect·name version·order edge 보존, cross-region consumer readiness를 검증. Check 유실, 순서 반전, dangling live-in, stale version, effect 이후 replay 위조 시 거부 |
| CA-04 / Physical Representation | src/physical.rs에는 registry/slot/generation BufferId, read-only BufferLease, checked span을 가진 PhysicalArray가 있다. tests/physical.rs는 lease 생존, stale ID, alias, 음수 stride, transpose, zero/singleton axis를 검증 | **M4 구현 미완료.** main에는 plan-time PlanBufferId/PhysicalViewId, mutable output ownership, last-use/reuse proof, PhysicalPlan verifier, memory residency/ready/transfer plan이 없다. 현재 BufferId를 compiler slot으로 재사용하지 않는다 | buffer/view span overflow, 다른 ID 사이 alias, stale generation, overlapping mutable outputs, dangling Return, temporary early free, OOM cleanup과 reuse-after-free를 각각 독립 negative test로 검증. 쓰기·재사용은 증명 전 비활성 |
| CA-05 / Rank·오류 | src/facts.rs의 RankFrameExecution은 ZeroFrameNeedsFill과 CellsPresent를 구분한다. logical_executor::execute_closed는 Check→Call의 closed-noun 순차 reference다 | **M4/M5 의미론 조건.** reference는 동적 NAME/state에 대한 전체 oracle가 아니다. 일반 CellApply를 uniform map으로 가정하거나 first-error 순서를 worker 완료 순서로 바꾸면 오류 | zero-frame 가상 fill cell, positive-frame empty cell, result-cell dtype/shape join, boxed/sparse, tolerance/fit, overflow·numeric reassociation, first J error, try/catch/effect를 C jsource+RustJ baseline과 비교. 불명확한 연산은 순차 유지 |
| CA-06 / Device·memory·cost | TargetCapabilities는 CPU/GPU family와 feature를 제공한다. j_graph_work_depth.rs는 successful-path symbolic work/depth, j_graph_resource.rs는 logical resource 정보를 계산한다 | **M5 이후 구현 미완료.** 이 타입만으로 device placement ≠ memory placement, residency/BufferVersion, transfer/sync/timepoint, critical path/실측 비용을 표현·검증했다고 말할 수 없다 | 첫 Physical Plan은 단일 CPU/Host/zero-transfer로 충분. 이후 ready-before-read, copy-after-producer, transfer-before-free, stale version, memory capacity, cost Unknown!=0 및 실제 overlap 증명을 negative test로 확인 |

**독립 교차 감사(두 경로, 정적 코드 검토):**

- 정방향: logical_ir.rs → lowering.rs → physical.rs → facts.rs → j_graph_work_depth.rs, j_graph_resource.rs의 현재 API와 각 단계가 *실제로 보유하는 정보*를 대조했다.
- 역방향: tests/logical_ir.rs, tests/lowering.rs, tests/physical.rs, logical_executor.rs가 이미 통과하도록 설계된 범위와 아직 부정 검증을 제공하지 못하는 *단계 간 관계*를 대조했다.
- 두 검토의 일치된 결론은 **대규모 아키텍처 재설계 불필요, RouteBoundary 전체 coverage / guard discharge / PhysicalPlan verifier는 미완료**다. 기존 코드 테스트를 새로 실행하거나 통과 확인했다는 주장이 아니다.

**지금 확정할 계약(실제 구현·새 IR 도입은 보류):**

1. **A3/M3:** verified A3의 구조적 유효성, optimizer가 사용할 semantic witness/guard의 충족, selected physical plan의 전체 적법성을 세 단계로 분리한다. A3에 device/thread/memory layout을 넣지 않는다.
2. **M3→M4 Route 경계:** A3 source op 전수 매핑, live-in/out, Check/effect/order/late-NAME, guard-before-effect와 commit/replay frontier를 별도의 *검증 결과(sidecar 또는 동등한 기존 route 계약)*로 소유한다. 해당 표현의 Rust 타입/새 계층은 현재 고정하지 않는다.
3. **M4 Physical:** PlanBufferId와 런타임 BufferId, PhysicalViewId를 분리한다. view/span·buffer ownership/lease/last-use, selected realization, Check coverage, Return을 Physical verifier가 확인해야 한다. ExecutionDevice / MemorySpace / device-local schedule은 서로 다른 결정이다. M4는 CPU/Host/Sequential만 실구현해도 된다.
4. **M5 이후:** ResourceEstimate, predicted CostEstimate, measured CostProfile, 성공 경로의 work/depth 및 오류/효과 경로를 분리한다. device-memory migration/async overlap은 명시적 의존성·실장치 증명 후에만 비용 선택에 반영한다.

**결정 및 우선순위:** (1) M3의 원본 A3→Route Check/effect/witness 증거 의무 확정 → (2) M4의 CPU/Host/zero-transfer PhysicalPlan verifier·실제 native Add 경로 → (3) M5 배치·자원·비용 후보 → (4) M6+ 별도 재개된 GPU/비동기 실행. 설계 자체의 구조적 모순은 **발견되지 않았음**; 그러나 위 증거가 없을 때의 fail-closed 실행 경로는 반드시 필요하다. 기존 M2→M3→M4 우선순위, HE-01~09의 미완료 상태, 별도 CUDA 보류는 유지한다.

##### M3→M4 Route-to-Physical 인계 계약 (2026-10-07, **설계만 완료 / 구현 없음**)

**목적:** 위 §2.5.1의 `M3-RB/RB-01~08`과 이 §5.2.1의 `PhysicalPlan` verifier를 하나의 **증거 체인**으로 연결한다. M3가 J 의미의 적법성을 판단하고, M4가 **이미 허가받은 의미를 특정 buffer/view/kernel 순서로 실현해도 안전한지** 검증한다. M4가 누락된 Check/guard, Rank·오류 순서, name/version 증거를 새로 추측하여 M3 판정을 뒤집어서는 안 된다. M3의 합격이 M4의 실제 실행 성공을 보장하지도 않는다.

**입력·출력·소유권 표(새 필수 IR이나 Rust 자료형을 확정하지 않는 개념 계약):**

| 인계 항목 | M3가 제공·보증할 것 | M4가 독립적으로 검증·구체화할 것 | 불충분한 경우 |
|---|---|---|---|
| **H-01 원본 기준** | immutable source A3 identity, schema/primitive-registry, source op/`j_origin`, version 및 정당한 rewrite/fusion mapping | 받은 계획이 **같은 원본**에서 나온 것인지, selected physical op가 허용된 mapping에 매달리는지 대조 | stale/바뀐 증거를 사용하지 않고 plan reject |
| **H-02 Route 확정** | region 전체 coverage, 각 region 선택 route 및 CPU/GPU/external target capability와 선정 이유; 단순 `legal_candidates` 목록과 다름 | 각 `Kernel`/`View`/`Materialize`가 선택된 recipe·target·region에 속하고 *진짜 구현된* realization인지 검사 | 미구현 native kernel/외부 route를 성공으로 위장하지 않고 다른 합법 route나 Unsupported |
| **H-03 값 경계** | `ValueId`의 producer, region live-in/live-out, `Plan.result`, dtype/shape/rank·J-visible boxed/sparse와 value snapshot identity | plan-time `PlanBufferId`와 `PhysicalViewId` 바인딩, physical encoding·extent/stride/span, 최종 Return의 논리 dtype/shape/order와 input/output ownership 검증 | missing value, 모순된 backing/encoding/shape, dangling return reject |
| **H-04 Check 의무** | 원본의 zero-result `SemanticCheck`마다 실행·검증된 증거에 의한 discharge·동등 guard 대체 **중 정확히 하나**, J error kind와 original order | 반드시 실행할 Check를 `PhysicalOp::Check` 또는 의미 보존 equivalent에 매핑. **discharged Check를 임의 재삽입하거나 누락된 Check를 추론하지 않음** | coverage/증거 불일치, duplicate/late Check, J error priority 변경 reject |
| **H-05 효과·이름·Write** | `order_after` 및 effect/error edges, noun read snapshot vs late function NameRef, `Plan.write`(별도 commit), guard-before-effect 및 replay 경계 | 명시된 순서로 물리 Check/Kernel/Return/commit handoff. 첫 M4 pure CPU slice에서는 stateful commit을 **실행기 소유로 만들지 않음** | 증명 안 된 NAME/Write는 RuntimeSemantic 경로; effect 이후 재실행 금지 |
| **H-06 Guard/실행 준비** | 각 witness 의존 입력 버전과 `GuardRequired` 의무, 평가 시점, 실패 시 합법한 fallback 및 commit frontier | 물리적 plan에서 guard가 해당 연산·효과보다 **먼저** 배치되고 그 실행·결과·binding freshness가 보장되었는지 검사 | `GuardRequired`를 `Verified/Ready`로 자동 승격 금지, 가드 없는 실행 불가 |
| **H-07 데이터 의존성** | SSA def→use, region cross-value dependencies, Check/effect-first-error precedence와 effect-live edges | BindInput/alloc → Check → View/Materialize/Kernel → Return의 실제 use-before-def, buffer readiness, lease/last-use, overlap/reuse permission. 비동기면 completion edge 추가 | 순차 계획이라도 부적절한 재배열·조기 free·중복 쓰기 reject |
| **H-08 표현·장치** | J-visible value type/shape/rank/representation 및 허용 target의 **제약**만 제공; source A3에는 device pointer/stride/physical memory 없음 | 실행 장치, 메모리 공간, device-local schedule과 buffer residency를 **각각** 선택. M4 = CPU/Host/Sequential/zero-transfer, M5+에서 transfer/ready/sync 확장 | GPU·외부 backend 또는 residency가 없으면 CPU/합법 fallback, 무근거 transfer 가정 금지 |
| **H-09 비용·진단** | semantics/legality 결과와 선택 제약; cost 추정이 proof를 대신하지 않음 | physical feasibility/budget/peak bytes, predicted cost 및 선택 근거를 별도 기록, 실패 유형을 source op/region/plan buffer와 연결 | cost Unknown을 무료/실행 가능으로 취급하지 않고 선택 보류 |

**승인 단계와 책임의 단방향성:**

~~~text
원본 source A3 (Plan::verify 성공 / 독립 의미론 권위)
  → M3 Route candidate (partition_plan은 아직 후보 분석)
  → M3 RouteVerified + [원본 연산/Check/Write/순서/guard/bridge 증거]
      ├─ Rejected: 증거 없음 → 다른 합법 route / Unsupported
      ├─ GuardRequired: guard와 안전한 실패 경로가 먼저 충족될 때까지 실행 불가
      └─ Verified: 해당 region의 의미론·Route 적법성만 통과
  → M4 Physical candidate (선택 recipe / buffer / view / ordered ops)
  → M4 PhysicalVerified (타깃·storage·순서·resource·coverage 검증)
  → RuntimeReady (동적 guard, input versions, 실제 lease, capacity 재확인)
  → 실행 / 오류 / 자원 정리
~~~

`RouteVerified`, `PhysicalVerified`, `RuntimeReady`는 **개념적 승인 상태**이며 신규 enum/IR 구현 지시가 아니다. 정적으로 `Verified`여도 입력 name/version, guard, buffer lease 등이 실행 시 바뀌면 실행 전 재검증이 필요하다. 정당한 RuntimeSemantic/외부 route는 RustJ-native M4 physical executor를 **반드시** 거칠 필요가 없지만 동일한 경계 의미론 의무를 지켜야 한다.

**검증기 책임 분할(중복 검증과 허위 위임 방지):**

- **M3 Route 검증기:** 원본 A3와 후보 매핑, check coverage/discharge, 순서·오류·effect·Write/NAME, target semantics, guard ownership 및 representation-neutral bridge의 *합법성*을 인증한다. Physical stride·allocation 위치는 결정하지 않는다.
- **M4 Physical verifier:** M3의 서명 없는 제안/단순 class를 실행권으로 간주하지 않는다. 승인된 source mapping·Check/guard/order obligations와 선택된 physical ops의 **정확한 연결**을 재대조하고, selected recipe·resolved CPU capability, checked affine bounds, buffer/version/ownership/lease, output ownership, temporal safety와 resource feasibility를 확인한다. J semantic proof를 스스로 만들어내지 않는다.
- **Runtime admission/Executor:** 실제 name/input snapshot/guard outcome, lease generation/readiness, 장치 가능 여부를 **사용 시점**에 확인한다. verifier를 통과하지 않은 계획 실행, 관찰된 효과 뒤의 transparent replay, backend 내부 오류를 근거 없이 J Domain/Rank/Length로 바꾸기는 금지한다.
- **Cross-route 책임:** 첫 native CPU slice 밖의 runtime/external 부분을 하나의 PhysicalPlan으로 강제로 합칠 필요는 없다. 다만 각 boundary handoff의 value/version/check/error/effect 계약은 통합 검증 기록으로 추적한다.


##### H-K — A3 OpKind별 M3→M4 인계 규칙 (2026-10-07; 설계만)

**구체적 공백:** 현행 `lowering.rs::route_operation`은 `Literal`, `ReadNoun`, `VerbReference`를 모두 `NoKernel`로, `partition_plan`은 그 연속 구간을 `RouteRegionClass::ValueOnly`로 합친다. 이 분류는 **실행 후보 grouping**이지 세 OpKind가 동일한 버퍼·호출·이름 의미를 가진다는 뜻이 아니다. M4 승인 여부는 region class뿐 아니라 원본 A3 **OpKind마다** 다르게 확인해야 한다.

| 원본 A3 대상 | M3가 책임지는 증거 | M4가 할 수 있는 일 / 금지되는 일 |
|---|---|---|
| `Literal(value)` | 원본 payload, dtype/shape, producer OpId와 result ValueId, source origin | 지원하는 dense CPU literal은 읽기 전용 입력 버퍼로 바인딩 가능. 이를 항상 deep copy할 필요는 없고, 타입·Shape를 임의 변경할 수 없음 |
| `ReadNoun {symbol,version}` | 원래 J read 시점과 noun snapshot, namespace/scope/버전, observable ordering | `ValueOnly`여도 literal로 간주하거나 나중 값으로 바꿔 읽지 않는다. 런타임 snapshot/guard가 없으면 합법한 RuntimeSemantic 경로에 남긴다 |
| `VerbReference(Callable)` | FunctionEntity, function POS, primitive/derived callable identity와 동적 NAME lookup | J 함수 참조는 noun의 dense 배열 버퍼가 아니다. `NoKernel`만 보고 `BindInput`용 값이나 kernel operand로 취급 금지; 별도의 function semantics 지원 필요 |
| `Basis { kind,payload,call }` | 원본 primitive/derived target·valence·Rank/CellApply·fact/error/effect, selected recipe 및 Check/guard 의무 | M3가 승인한 선택 target에 해당 recipe/실제 native 구현이 **존재할 때만** Kernel/View 계획 가능. `legal_candidates`나 `ReferenceSequential`만으로 native 준비 완료 주장 금지 |
| `SemanticCall(call)` | 정규화되지 않은 호출 의미, dynamic binding, 효과/오류 | 현행은 RuntimeSemanticFallback. 별도의 동등성·지원 증거 없이 임의로 native Basis/Kernel로 승격 금지 |
| `SemanticCheck(check)` | producer/liveness, zero-result constraint, 원본 error kind/span/order 및 실행·증명된 discharge·동등 guard 중 하나의 소유자 | output SSA가 없더라도 제거 금지. 실행 의무가 있으면 Check를 원래 순서에 반영; 증명된 discharge만 무실행 허용 |
| `Plan.result / Return` | 마지막 result ValueId, logical type/shape/atom order, cross-region producer | 유효한 output ownership/lease로 Return. Return이 Write commit 완료라는 뜻은 아님 |
| `Plan.write` (operations 밖) | symbol/value/previous/proposed/span/after와 commit/effect owner | 모든 op range를 포함해도 Write는 자동 포함되지 않는다. 초기 M4 pure slice가 assignment를 commit하거나 Write를 무시·반복 실행하는 일 금지 |

**전역·국소 검증의 구분:** RB-01은 원본 A3 **전체**의 모든 operation과 별도의 Write 사건에 적용된다. 하나의 RustJ-native PhysicalPlan은 이 가운데 **M3가 승인한 native region**만 담당할 수 있다. 전역 coverage는 M3가, 각 native region의 선택 recipe/Buffer·Check·Return coverage는 M4가 책임진다. 다른 RuntimeSemantic/External region은 억지로 동일한 PhysicalPlan에 집어넣지 말고 경계 ValueId·ordering·effect와 연결한다.

**인계 시 세 번 확인할 사실(새 필수 IR이나 데이터 계층 추가 없음):**

1. **M3 RouteVerified:** immutable source A3의 원래 OpKind/Check/Write/ValueId/order와 region의 실제 연산이 일치하는가? GuardRequired는 실행 허가가 아니며 guard 위치·실패 경로를 확정해야 한다.
2. **M4 PhysicalVerified:** 승인된 native op 각각에 실제 target-compatible recipe 및 물리 Bind/Check/View/Kernel/Materialize/Return 대응이 있는가? `PlanBufferId ≠ PhysicalViewId ≠ runtime BufferId`, affine span, lease, encoding, ownership/last-use가 증명되는가? M4가 M3의 의미 증거를 새로 만들어내지는 않는다.
3. **RuntimeReady:** 실제 read/name snapshot, input version, guard outcome, lease generation 및 capacity가 **사용 시점**에도 유효한가? 실패하면 observable effect commit **이전**의 안전한 fallback만 허용하고, effect 이후 자동 replay 금지.

**부정 검증 계획 (H-KV1~5; 실행 테스트 미작성):**

- **H-KV1:** `1+2`에서 Literal/Kernel input 대응을 유지해야 함. `VerbReference`를 `ValueOnly`라는 이유로 dense `BindInput`으로 취급하거나 literal producer를 바꾼 경우 거부.
- **H-KV2:** `a`의 snapshot/version/read 시점, `a=:1+2`의 별도 Write/after를 한 가지만 변형해 거부. 결과 Value만 맞고 Write가 유실돼도 거부.
- **H-KV3:** `1 2+1 2 3`의 zero-result Length Check는 Call 전에 관찰되어야 함. Check를 버리거나 중복·후행 배치한 physical 계획 거부.
- **H-KV4:** `future 3`의 SemanticCall을 새 의미론 증거 없이 native 실행으로 가장하거나 `ReferenceSequential` 후보만으로 native kernel 성공 주장 시 거부.
- **H-KV5:** 같은 원본 A3가 CPU-native와 RuntimeSemantic으로 분할될 때 M3 global coverage와 M4 native-local coverage를 각각 검사; 중간 live-out, 첫 오류, effect-live, final Return 손실 거부.

**상태:** H-K는 기존 H-01~09/HM-V0~V4를 OpKind별로 구체화한 **설계 수용 보조 규칙**이다. 별도 canonical IR, CPU Parallel 레이어나 runtime 구현을 요구하지 않는다. 실제 validator/negative test/차분 결과가 없는 동안 M3/M4/HE-01 완료 항목은 열린 상태로 유지한다.

**검증 표본(각 정상이 확인된 후 불변조건 1개만 위조하는 계획):**

| 사례 | 정상 인계 증거 | 거부·실패로 판정할 변형 |
|---|---|---|
| `1+2` | 원본 SSA value/literal → 선택 CPU Elementwise route → 별도 verified physical input/Return mapping | M3에 `ReferenceSequential` 후보만 있는데 M4가 구현되지 않은 native Add를 실행 가능이라고 승인; ValueId/PlanBufferId를 혼동 |
| `1 2+1 2 3` | zero-result Length Check의 원본 provenance/order를 M3가 전달, M4가 Check→Kernel 이전 실행을 보장 | M3 Check는 승인했으나 M4 plan에서 삭제·중복·Call 뒤 배치; J Length 오류를 구현 오류로 바꿈 |
| `1 2+3 4` | 입력 shape에 맞는 PrefixAgreement witness가 원본에 고정돼 있다면 합법적 Check discharge | input shape/NAME version 변경 후 과거 witness로 PhysicalVerified; 증거 없는 discharge를 M4가 자체 승인 |
| `3 { 10 20 30` | Index Check 원래 오류 종류·우선순위와 물리 검사의 대응 | Check를 M4 materialize/Kernel 뒤로 옮기거나 Index 대신 Length로 변경 |
| `a` 및 `a=:1+2` | name snapshot과 별도 Write event가 M3에 남고, 지원되지 않는 stateful 부분은 RuntimeSemantic 소유 | Return만 물리화하며 Write를 drop, speculative name read, committed assignment 이후 fallback replay |
| 2D reverse/transpose / empty frame | 의미상 Shape/atom order와 zero-frame fill 의무를 M3가 소유, M4는 signed span/stride/encoding·복사 순서 확인 | 음수 stride OOB, zero-frame CellApply를 아무 계산 없이 결과형 가정, incompatible cell-result join |
| 같은 source의 CPU/GPU/External 후보 | 공통 A3/guard/error 계약, 배치·storage contract는 route별로 독립 | CPU-only 테스트로 GPU/transfer Ready 주장, 준비되지 않은 메모리에서 read, 미승인 overlap/async error reorder |

**인계 수용 테스트 순서 (테스트 작성·실행은 보류):**

1. **HM-V0 정적 인터페이스:** M3-RB RB-V0~V2 승인 증거의 필드/원본 binding을 M4 입력 계약과 하나씩 대조한다. 누락·중복·stale op/Check/Write/guard, 다른 source의 승인 보고서 재사용을 차단한다.
2. **HM-V1 타입·자원:** CPU/Host/no-transfer M4 후보의 `ValueId ↔ PlanBufferId ↔ PhysicalViewId ↔ BufferLease` 관계, affine span·encoding·ownership·last-use·output validity·OOM cleanup을 정적/실행 시점으로 나누어 확인한다.
3. **HM-V2 순서·오류:** zero-result Check, observable effect/order, guarded fallback/commit과 첫 J 오류를 M3→M4→Runtime 세 지점에서 교차 검증한다. guard failure는 실행 전 대체 경로로만 넘긴다.
4. **HM-V3 3자 차분:** 동일 원본 A3를 기준으로 jsource C(사용 가능할 때), RustJ semantic/reference 경로, native Physical 경로의 값/dtype/Shape/atom order/J error class/우선순위/관찰 state를 비교한다. C 또는 native 경로가 없다면 **미검증**, 대체 통과 처리하지 않는다.
5. **HM-V4 단계 승인:** M3 `RouteVerified` 증거가 있더라도 별도 `PhysicalVerified`·`RuntimeReady`가 없으면 미승인. M4 완료는 구현된 native Add/Check 등을 통한 E2E와 negative tests가 실제 통과한 후에만; GPU/async/transfer는 M5·M6+ 별도 검증.

**정리:** 별도 새 compiler layer는 필요하지 않다. ***M3는 의미론적 적법성의 증거를 소유하고 M4는 그 증거를 변경하지 않고 물리적으로 실현했는지 검증한다.*** 이 명세만으로 M3 검증기/M4 Physical Executor가 만들어졌거나 테스트가 PASS한 것은 아니다. §10의 M3-RB·M4 및 HE-01~09는 **설계 완료와 구현/검증 미완료를 계속 분리**한다.

##### H-A — M3→M4 첫 CPU 경로 실행 승인 판정표 (2026-10-07; 명세만)

**이 절의 역할:** 기존 H-01~09, H-K, HM-V0~V4의 반복이 아니라, 실제 `lowering.rs`의 `RouteDecision`/`RouteRegionClass`와 미래 M4 compiler-native CPU 경로 사이에 **실행 가능한 것과 단지 합법적 후보인 것**의 경계를 확정한다. 현재 `main`에는 `src/physical_plan.rs`가 없으며, `src/physical.rs`는 read-only 표현 기반이다. Draft PR #4의 제한적인 literal identity 코드를 완료된 native Add 실행기로 계산하지 않는다. 구현·테스트 추가 없음.

**기본 승인 흐름(기존 단계의 판정 질문):**

| 순서 | 확인 대상 | 승인하려면 | 증거 부족 시 |
|---|---|---|---|
| **A0: SourceA3** | 변환 전 원본 `logical_ir::Plan` | `verify()`, source/provenance와 original Check/Write/오류 의무를 고정하고 변환 후보와 독립 보관 | 손상된 원본/다른 registry → 거부 |
| **A1: RouteVerified** | `partition_plan`이 분류한 모든 원본 op와 선택할 *특정* region | RB-01~08 coverage, source op/operand/live-in/out, Check 실행·proof discharge·guard 소유, Write/Name/error-order 및 region-wide capability 증거 | `NoKernel`·`NativeExecutionBasis`·`RuntimeSemanticFallback`이라는 분류 자체는 허가 아님 |
| **A2: ExecutableRoute** | M4가 인수하는 native region | M3가 승인한 **구체적인** realization과 **실제 존재하는 CPU kernel/adapter**, dtype/rank/valence 및 guard 지원; `ReferenceSequential`라는 후보 이름만으로 native 구현을 추정하지 않음 | 유효한 semantic/reference route만 사용하거나 Unsupported. *native 실행 가능*으로 표시 금지 |
| **A3: PhysicalVerified** | 물리 연산·버퍼·View·Check·Return | 허가된 region/source mapping과 1:1 또는 등가 증명 대응. BindInput/Check/Kernel/Materialize 순서, PlanBufferId≠BufferId, span/encoding/ownership/lease/last-use, CPU Host capacity 검증 | invalid physical plan은 실행 직전에 임의 보정하거나 J 오류로 바꾸지 않고 거부 |
| **A4: RuntimeReady** | 실제 호출 시점 | input/name snapshot 버전 및 guard 판정, registry lease/generation, capacity·readiness 유효. effect 이전 대체 경로 존재 | stale/guard failure는 **effect 이전에만** 안전한 경로로 변경, commit 이후 자동 재실행 금지 |
| **A5: AcceptedNativeE2E** | 출력과 관찰 결과 | 실제 compiler-native 결과·dtype·Shape·atom order·첫 J error/effect가 RustJ 순차 및 가능할 때 jsource 기준과 일치, 관련 부정 사례 테스트 통과 | A3 분석 통과, reference route 결과, 문서·테스트 소스 존재는 M4 완료 증거가 아님 |

**같은 A3에서 계획을 구성하는 구체 추적 표본**

~~~text
J: 1+2
  Source A3: Literal(1) → ValueId(0)
             Literal(2) → ValueId(1)
             Basis(Elementwise, Add) → ValueId(2), result=ValueId(2)
  Route candidate: ValueOnly(리터럴 구간) → PureArray(Add 구간)
  M3: 원본 Add, value 연결, PrefixAgreement witness/guard, Check 의무를 대조
  현행 CPU Lowering: Elementwise의 ReferenceSequential 후보 존재
  M4 실행 판정: 실제 compiler-native Add 구현/물리 검증/실행 결과 전에는 A2~A5 미승인

J: 1 2+1 2 3
  Source A3: 두 Literal → zero-result SemanticCheck(Length) → Basis(Add)
  Route candidate: ValueOnly → SemanticCheck → PureArray
  M3: Length Check와 Check→Call ordering, operand live-in을 전수 인계
  M4: Check 실행 또는 독립 증명된 동등 대체 없이 Kernel 선실행 불가
  기대 관찰: Length error가 원래 시점에 보고되어야 함. 수행되지 않은 물리 경로는 미검증

J: a=:1+2
  Source A3: 계산 op + 별도 Plan.write {symbol,value,previous,proposed,after}
  M3: 계산 결과와 Write commit을 서로 다른 의무로 인계
  M4: 순수 Add region만 인수 가능. 반환값 생성이 assignment commit을 뜻하지 않음
  전체 문장 완료: 책임이 분명한 RuntimeSemantic/향후 effect owner 필요
~~~

**지원 상태 판정표:** 현재 코드에 `ReferenceSequential`, `GenericCellLoop`, `MetadataOrIndexReindex` 등 *등록된 후보*가 있다고 해서 CPU Physical Executor가 구현된 것이 아니다. `RouteDecision::NoKernel`은 literal/read/verb-ref가 **같은 종류의 값**이라는 뜻이 아니다(H-K 참고). `TargetCapabilities::gpu_generic()`의 **기능 묘사**는 실제 GPU 장치·런타임 가용성 또는 실행 허가가 아니다. M4-v0 정책은 CPU/Host/Sequential/zero-transfer만 선택하도록 제한하되, 그 제한을 상위 J 언어 의미론/Logical IR에 역류시키지 않는다.

**검증 스펙 HA-V1~HA-V7 (실행은 보류):**

| 테스트 ID | 정상 승인 조건 | 한 항목씩 바꾸어 확인할 거부 조건 |
|---|---|---|
| HA-V1 **A3→Route** | `1+2`의 op/region 커버리지, 정확한 result/operands, source origin | native Add op 유실, 두 region overlap, 타 A3 origin을 같은 것으로 가장 |
| HA-V2 **후보≠커널** | `ReferenceSequential` 후보를 그대로 분석 후보로 표시하고 실제 native 구현이 존재할 때만 A2 승격 | 등록 후보만으로 PhysicalVerified/RuntimeReady/NativeE2E 선언 |
| HA-V3 **Check→Kernel** | `1 2+1 2 3`의 원본 Length Check를 선행, `3 { 10 20 30`의 Index Check를 선행; first error class 보존 | Check 삭제/중복/후행 배치, Length↔Index 교체, Kernel-first schedule |
| HA-V4 **witness/guard** | `1 2+3 4`처럼 원래 shape witness가 유효하면 Check 없이 가능; guard 필요 시 효과 전에 완료 | 바뀐 input shape/name version에 과거 증명 재활용, GuardRequired를 실행 승인으로 오인 |
| HA-V5 **Name/Write** | `a`는 read snapshot 검증, `a=:1+2`는 별도 commit owner 확인 | `ReadNoun`을 상수 literal로 변경, late function lookup 조기 고정, `Plan.write` 삭제/중복·effect 후 replay |
| HA-V6 **Storage/Return** | 음수 stride/transpose/빈 Shape의 논리 순서·encoding·span, output ownership/lease generation | `ValueId==PlanBufferId` 간주, 미등록/조기 해제 버퍼, overlapping writable view, 잘못된 atom order·dangling Return |
| HA-V7 **cross-route** | native 부분과 RuntimeSemantic 부분의 M3 global coverage + M4 native local coverage, 오류/effect edge 보존 | native region만 정상이라 전체 표현식 승인, 외부 bridge 미구현 상태에서 M4 전체 실행 성공 주장, GPU 완료 허위 기록 |

**구현 우선순위와 수용 상태:** (1) 기존 M3-RB validator와 독립 source A3 기준의 HA-V1/3/4/5 논리 검증 → (2) M4 선택 recipe의 *실제* 구현 여부·CPU buffer/Check/Return verifier(HA-V2/6) → (3) 독립 reference/jsource의 HA-V7 및 native E2E 수용 → (4) M5/M6의 device/memory/transfer 비용·비동기 검증. **이번에 완료한 것은 이 판정표와 테스트 설계뿐이며**, 현행 코드의 M3/M4/HE-01 실행 승인 체크박스는 그대로 열어 둔다.


##### H-P — M3→M4 구현 인수 패킷과 물리 검증 체크리스트 계약 (2026-10-07; 문서 전용)

**사용법:** 이 절은 H-01~09, H-K, H-A(A0~A5), RB-01~08의 **실행 시점 판정표**다. 구현·PR 심사 때 각 행에 입력 증거, 검증 결과, 부정 사례와 증거 링크를 채운다. 새 필수 IR, `TaskId` 런타임, 멀티프로세서 전용 planner를 요구하지 않는다. **미제공/unknown은 승인으로 간주하지 않는다.** M3가 의미론·선택된 route의 합법성 증거를 제공하며 M4는 이 증거를 재작성하지 않고 선택한 물리 실현을 검증한다. 전역 A3 coverage는 M3 소유, native region의 실현 coverage는 M4 소유다.

**선행 단계가 전달해야 할 최소 패킷(개념적 묶음이며 새 Rust struct 요구 아님):**

| ID / 책임 | M3/상위 단계가 인계할 증거 | M4가 반드시 확인할 사항 | 증거 부족·불일치 시 |
|---|---|---|---|
| **HP-01 원본** | 별도 보존한 검증된 원본 A3, schema/primitive registry, 원본 op ID·span·J graph origin, 변경/재작성 대응과 증명의 적용 범위 | 단순 source **문자열** 일치가 아니라 **실제 원본 op payload/facts/Check/Write와 selected region provenance**를 대조. digest 단독·바뀐 A3의 `verify()` 성공만으로 원본 동일성 승인 금지 | stale/위조 source proof 거부 |
| **HP-02 범위** | 원본 모든 op·0-result Check·별도 `Plan.write`의 M3 global ownership, 각 region op 범위, live-in/out, final result, 교차 route bridge | 해당 native region에서 원본 op→physical task 관계가 **완전하고 증명된 매핑**인지 대조. 1:1을 강제하지 않으며 fusion/rewrite의 N:1·1:N 대응은 RB-01 등가 witness가 필수. 외부 구간과 live-out 누락 없음; `NoKernel`/`ValueOnly` 후보 분류만으로 binding 승인 불가 | coverage 누락·겹침·증거 없는 병합/중복 실행 거부 |
| **HP-03 호출과 recipe** | 원본 `OpKind`, callable POS/valence, derived Rank/CellApply/fit/tolerance/numeric policy, selected route와 **구체적 구현** capability/target/guard | 등록된 `ReferenceSequential` 등의 *후보*와 실제 구현 kernel/adapter를 구분; 지원 dtype/rank/shape/valence·CPU target 검증 | 지원되지 않는 native kernel은 실행 금지 |
| **HP-04 오류와 Check** | 각 `SemanticCheck`의 constraint/입력·기대 J error kind·원본 선후관계·execute / proven discharge / guard 중 유일한 owner | 결과 SSA 값이 없는 Check도 누락 금지. 실행/guard는 **해당 Check에 종속된** kernel·효과보다 앞서되, **원본에서 Check보다 먼저 발생해야 하는 다른 오류·효과보다 앞당겨서는 안 됨**. 입증된 discharge는 원본 error/order 조건과 불변인 입력 증거가 필요 | 누락·중복·선후관계·J error class 변조 또는 무근거 discharge 거부 |
| **HP-05 Name/Effect/Write** | noun read 시점의 value/binding-version/scope/locale 증거, function nameref의 late lookup/POS, effect edges, `Plan.write` 별도 commit owner | source statement 전체의 사전 snapshot 금지, Name 변동 뒤 guard 재확인, `Return`을 assignment commit으로 오인 금지; effect 후 무조건 replay 금지 | 미확정 binding·commit owner는 native 승인 보류 |
| **HP-06 logical value** | producer/def-use, dtype/rank/Shape/J atom order, boxed/sparse·0-cell Rank fill/prototype/assembly 의무, overflow·promotion·error witness | 물리화로 logical value가 사라지지 않으며 `ValueId`≠`PlanBufferId`≠runtime `BufferId`; 빈 배열 Shape와 J prefix frame 규칙 유지 | 변환 대신 semantic/reference route 또는 Unsupported |
| **HP-07 계획 버퍼** | 값별 storage/materialization 요구, readonly/externally owned/persistent/temporary 여부, alias·donation 허용 증거, 메모리 상한 | buffer encoding·capacity·alignment·memory space·view span/stride/offset·generation·실제 backing alias, overlapping writable view, last-use 뒤 재사용만 허용 | bounds/alias/ownership 모순 거부, input 불변 유지 |
| **HP-08 Task/Dependency** | 원본 data-flow·observable order·Check/guard/effect/first-error edge, 각 value ready와 region interface | `BindInput/Check/View/Materialize/Kernel/Return` 같은 task의 source 대응, 선행 task 및 ready 보장, DAG cycle/누락 edge 검출; 순차 CPU에서는 검증된 list order로 실현 가능 | 의존성 위반 task launch 거부 |
| **HP-09 Completion/수명** | task 사용 집합, 각 reader/writer·transfer의 완료 필요조건, 반환·임시 소유권 | `submitted`≠`completed`; **모든** 사용 완료 전 buffer 재활용·외부 반환 금지. v0 동기 CPU는 task 완료 시 즉시 completion으로 간주할 수 있지만 그 근거를 명시; 비동기 경로는 explicit event/timepoint/lease 필요 | completion 증거 없는 재사용·비동기 실행 금지 |
| **HP-10 동적 guard·fallback** | witness/proof provenance, 재검증할 runtime 조건, 해당 guard의 effect 이전 안전 지점, **기존에 적법한** 대체 route | guard 검사→사용 사이 stale version/shape 불가; failure 시 effect 이전에만 지원 route 선택; 부분 effect 이후 자동 재실행 금지. Backend 실패를 임의 J Domain/Rank/Length로 매핑 금지 | guard false면 native 실행 중지, safe fallback 없으면 명시적 실패 |
| **HP-11 장치·자원** | `TargetContext`/resolved capability 및 device/runtime availability, resource limits, 비용/자원 추정의 구분 | 첫 M4: CPU 실행·Host memory·단일 순차·zero transfer만 승인. 미래 GPU/멀티코어는 device placement, memory space, transfer readiness, intra-device scheduling 독립 검사; resource feasible ≠ profitable | 미지원 device/transfer 경로 승인 금지 |
| **HP-12 실행 결과 증거** | reference/Jsource 대응 가능 사례, 실행 route와 guard 기록, 검증할 dtype/Shape/atom·effect/error 결과 | 독립 RustJ semantic/A3 reference와 가능한 pinned J C reference, **실제 native route**의 3방향 비교; CI 명령·commit·artifact 연결 | 테스트 소스 존재/후보 선택만으로 M4 E2E 통과 주장 금지 |

**최소 자료흐름/승인 프로토콜:**

~~~text
M3: immutable SourceA3 + selected RouteRegion + original op/Check/Write coverage
    + live-in/out + semantic proof/witness/guards + observable order
  -> RouteVerified  (A0/A1, 아직 실행 허가는 아님)
M4: task(s) + planned buffer/view + def-use/dependency + readiness/completion
    + selected real CPU realization + lease/ownership/resource checks
  -> PhysicalVerified  (A2/A3, 특정 native region에 한정)
Call time: name/input versions + guards + actual buffer generations/leases
  -> RuntimeReady -> Execute -> observed result/error/effect -> independent comparison (A4/A5)
~~~

**최종 심사 시점과 판정(기존 A0~A5의 통과 기준; 새 gate 아님):**

| 기존 gate / 판정 책임 | PASS에 반드시 필요한 증거 | FAIL/UNRUN 및 후속 |
|---|---|---|
| **A0 SourceA3 — Frontend/A3 소유** | 원본 `verify()` 및 스키마/registry/op payload·facts·span·origin의 **변경되지 않은 권위 스냅샷** 확인; 수정 후보는 원본을 덮어쓰지 않음 | 원본 부재·stale → FAIL; snapshot 대조 미실행 → UNRUN |
| **A1 RouteVerified — M3 소유** | RB-01~08의 **전역** op·Check·Write 커버리지, 실현 가능한 선택 region, 모든 effect/error/guard/Name 및 bridge 의무의 증명 또는 안전하게 owner 지정. `GuardRequired`를 무조건 `Verified`로 표시하지 않음 | 누락·증거 없는 fusion·불명 capability → FAIL/UNRUN; 다른 적법 route만 가능 |
| **A2/A3 ExecutableRoute/PhysicalVerified — M4 소유** | 실제 설치/구현된 selected realization + 해당 region의 증명된 source↔task 매핑, checked buffer/view/alias·의존성·동기 CPU 완료/자원 경계. A1 PASS를 M4 PASS로 복사 금지 | 물리 verifier·native kernel 미구현은 UNRUN/미지원. 미정의 buffer/edge·alias 위반은 FAIL |
| **A4 RuntimeReady — 호출 시점 Runtime/Executor 소유** | 해당 **호출마다** live input/name/version·guard·lease/generation·readiness 재검증; effect 전 fallback 준비 여부 | guard false이면 native 미실행, effect 전 합법 route로만 분기. 이미 commit된 effect 뒤 retry 금지 |
| **A5 AcceptedNativeE2E — 독립 차분 검증 소유** | 실제 native route 실행 기록과 semantic reference 및 가능한 pinned J C oracle의 **값·Shape·첫 J error·effect** 일치, positive/negative 테스트와 CI commit/run 기록 | reference/fallback 실행만 성공하거나 증거 미실행이면 UNRUN; 불일치 시 FAIL. C oracle 불가 항목은 불가 사유를 명시하고 3방향 검증 완료로 표시하지 않음 |

**오류 분류:** 잘못된 물리 계획/미구현 capability/`GuardRequired`는 J 프로그램의 Domain·Rank·Length 오류가 아니다. 반면 적법한 J 실행에서 먼저 발생한 `SemanticCheck`는 원래 J error class와 순서를 유지한다. 증거의 존재(문서·테스트 정의)는 실행된 PASS와 별개다. 각 gate의 PASS는 **상위 gate PASS를 자동 승계하지 않는다**. `PhysicalVerified`는 특정 선택된 native region에 한정되며 전체 프로그램이나 GPU까지 확장하지 않는다.

**단계별 부정 테스트 등록부(각 테스트에서 정확히 한 조건만 위조):**

| 시험 | 기준 사례 / 실패 유도 | 기대 판정 |
|---|---|---|
| **HP-V01 source** | 같은 source 문자열로 literal atom, SSA facts, rank 또는 `j_origin` 변경 | 검증 실패. Draft PR #4의 `LiteralSourceWitness`는 **단일 literal v0의 부분적 보완**이지 일반적인 M3 원본 증명이 아님 |
| **HP-V02 coverage** | `1+2` Add op 누락, `a=:1+2`의 독립 `Write` 유실, 중복 region | RouteVerified/PhysicalVerified 거부 |
| **HP-V03 checks** | `1 2+1 2 3`의 Length Check 제거·후행·중복·Index로 변조 | Add 실행 전에 거부, 또는 정확한 J Length 오류 |
| **HP-V04 guard/name** | `a`의 noun snapshot/version 변경, late verb NameRef 조기 고정, shape guard 값 변경 | revalidation 불합격; 효과 이전의 적법한 대체 경로만 허용 |
| **HP-V05 buffer/view** | **정상 대조군:** 유효한 음수 stride·singleton 0-stride·빈 Shape·read-only alias. **각각의 위조군:** 실제 span 범위 이탈, stale generation, 잘못된 encoding, 겹치는 독점 write, 불법 last-use 재활용 | 합법적인 stride·빈 Shape를 거부하지 않음; bounds/alias/수명 위반만 거부 |
| **HP-V06 dependency** | Check/guard→Kernel, producer→reader, previous effect→commit edge 하나 삭제하거나 순환 추가 | plan verifier 거부, arbitrary first-error 노출 금지 |
| **HP-V07 completion** | 비동기 copy/kernel을 enqueue만 하고 입력 release, reader 완료 전에 재사용 | 사용 가능·재활용 거부; 실제 completion 이후만 허용 |
| **HP-V08 replay/error** | J error가 가능한 작업의 순서 뒤집기, effect commit 뒤 guard failure fallback 재실행 | first-error/effect order 보존 실패로 거부, 중복 effect 금지 |
| **HP-V09 baseline** | 닫힌 literal identity와 이후 실제 구현한 순차 CPU 단일 op의 차분 비교. Boxed/sparse·Rank zero-frame처럼 **그 시점 native 구현에서 미지원**인 정당한 J 입력은 semantic/reference fallback으로 별도 검사 | native 지원/미지원 정확 분류; semantic fallback 자체는 J 성공이어도 native E2E 승인으로 세지 않음 |

**증거 기록 양식(구현 시 각 gate마다 1건):** `gate ID | source A3 revision/schema/registry | source op/region IDs | M3 proof·guard owner | chosen recipe/capability | physical task/dependency/buffer/view IDs | runtime version/completion/effect frontier | positive/one-invariant-negative test IDs | CI commit/run·reference artifact | PASS/FAIL/UNRUN | 남은 예외`. `UNRUN`은 PASS가 아니다.

**현재 상태/우선순위:** `main`은 M3 RouteBoundary와 M4 full native verification을 아직 갖추지 않았다. Draft [PR #4](https://github.com/yunskim/RustJ/pull/4)의 제한된 literal identity 경로·source witness·CI 성공이 있더라도 Check, 실제 Add, 전체 source coverage, completion/비동기, GPU 지원을 증명하지 않는다. 먼저 **HP-01~06·08·10의 순차 CPU 의미/검증 계약**을 실현하고 HP-V01~04/06/08을 확인한다. 그 다음 실제 CPU Kernel과 HP-07/09 자원·수명 검증을 추가한다. HP-09의 비동기 completion, transfer, multi-device/worker scheduling은 이종 실행 단계에서 확장한다. M4-v0는 full async scheduler를 필수 선행조건으로 하지 않는다.

##### 현재 코드와의 대응

현재 `src/physical.rs`는:

~~~text
BufferRegistry / BufferLease / runtime BufferId
checked read-only affine PhysicalArray
shape / strides / offset / encoding validation
~~~

을 제공하는 **G1 representation foundation**이다. 아직 `PhysicalPlan`, plan-time buffer slot, planner, physical executor가 아니다. `logical_executor.rs`도 A3 semantic/reference executor이지 Physical Executor가 아니다.

##### 구현 순서 — 한 번에 한 의미 + 한 verifier/test

1. plan-time `PlanBufferId`/`PhysicalViewId`와 empty plan verifier
2. `BindInput + Return`만으로 identity plan E2E
3. `View` + span verifier, transpose/reverse metadata-only 회귀
4. `Check` + J error class/order regression
5. `Kernel` 한 종류(Add) + selected lowering capability verification
6. `Materialize` + logical-order copy/ownership test
7. last-use + reuse witness, alias negative tests
8. 여러 op를 연결한 verified Logical IR → PhysicalPlan → CPU result differential test

이 순서는 G4를 구현할 때의 최소 vertical slice이며 full GPU resource model을 선행 조건으로 만들지 않는다.

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

#### 5.5.1 External adapter boundary contract

external adapter는 RustJ semantic IR의 대체물이 아니라 **verified RouteRegion/Logical IR의 projection**이다. 첫 실제 MLIR/StableHLO/ArrayFire/library adapter를 구현하기 전에 모든 adapter가 공통으로 만족해야 할 boundary를 고정한다.

입력 계약:

~~~text
ExternalAdapterInput
  RouteRegion
    live_in / live_out
    ordered operations / SemanticChecks
    effect/error edges
    provenance
  discharged legality evidence / guards
  resolved TargetContext
  logical representation requirements
~~~

adapter는 unresolved legality를 backend optimizer가 알아서 해결할 것이라고 가정하지 않는다.

capability query는 최소 다음 축을 분리한다.

~~~text
AdapterCapability
  operation / ExecutionBasis / valence
  dtype classes
  rank / shape constraints
  dynamic-shape support
  representation / layout preconditions
  numeric/tolerance/reassociation policy
  effect/token support
  error/check representation
  alias / mutation contract
  async / completion semantics
~~~

`supports Add` 하나로는 충분하지 않다. 예를 들어 같은 Add라도 rank/cell mapping, dtype promotion, overflow/error semantics, layout/alias requirement가 다르면 다른 capability query가 필요할 수 있다.

출력은 단순 external module bytes가 아니라 RustJ가 검증할 수 있는 projection record를 포함해야 한다.

~~~text
ExternalRegionPlan
  adapter_id / adapter_schema_version
  source RouteRegion + A3 provenance
  translated external operations/module
  host-side checks retained
  mapped effect/token edges
  BridgeRequirements
  external input/output handles
  completion / ownership contract
  unsupported/compile-failure classification
~~~

##### SemanticCheck / error mapping

각 A3 `SemanticCheck`는 다음 중 하나여야 한다.

1. external launch 전에 RustJ host/native side에서 원래 순서대로 실행,
2. external IR이 **같은 J-visible error class와 precedence**를 보장할 수 있을 때 명시적으로 lowering,
3. 그렇지 않으면 해당 region을 external route에서 거부.

backend assertion/trap을 무조건 J Domain/Rank/Length error로 바꾸지 않는다. external compiler crash/unsupported/kernel launch failure도 J semantic error가 아니다.

##### Effect / token mapping

- pure region은 token 없이 projection할 수 있다.
- J-visible write/I/O/state ordering이 있는 region은 backend가 equivalent token/resource ordering을 표현할 수 있을 때만 projection한다.
- StableHLO token/custom_call 같은 escape hatch가 존재한다는 사실만으로 arbitrary J effect support를 선언하지 않는다.
- host-side effect와 external async operation이 섞이면 completion token이 §2.5.1 RouteBoundary와 §3.9.4 commit frontier에 연결되어야 한다.

##### Representation bridge / ownership

adapter는 J logical value를 backend layout과 동일시하지 않는다.

~~~text
A3 Logical Value
  ↓ BridgeRequirement
Physical/External bridge lowering
  ↓
external tensor/array/handle
  ↓ completion + ownership
A3 live_out / next RouteRegion
~~~

row-major/column-major, dense/sparse encoding, device memory, alignment, zero-copy 가능성은 adapter precondition/bridge/Physical Plan 책임이다. semantic dtype/shape/order를 layout에 맞춰 바꾸지 않는다.

##### Round-trip verifier

adapter output은 최소 다음을 검증할 수 있어야 한다.

- input RouteRegion의 모든 semantic operation이 translated op, host-side check, explicit bridge/effect action 중 정확한 대응을 가진다.
- live-in/out logical dtype/shape/rank/order contract가 projection 전후에 일치한다.
- dropped/reordered SemanticCheck/effect/error edge가 없다.
- adapter capability/witness가 실제 emitted external form의 requirement와 일치한다.
- external output handle의 ownership/completion이 다음 region이 사용하기 전에 확정된다.
- source A3 op/J Graph/source span으로 provenance를 역추적할 수 있다.
- unsupported form은 partial external module을 실행 가능한 성공 plan으로 반환하지 않는다.

이 verifier는 external compiler 자체의 optimizer correctness를 재증명하는 것이 아니라, **RustJ가 넘긴 의미와 adapter가 선언한 projection 사이의 계약**을 검증한다.

##### Failure classes

~~~text
AdapterUnsupported
  semantic/capability/precondition상 이 route를 만들 수 없음

AdapterCompileFailure
  backend compiler/API가 plan 생성에 실패

AdapterRuntimeFailure
  launch/execution/completion infrastructure 실패

JSemanticError
  RustJ SemanticCheck/operation contract가 정의한 실제 J error
~~~

앞의 세 항목을 임의로 `JSemanticError`로 재분류하지 않는다. 실행 전 failure이면 §3.9.4에 따라 verified alternate route를 선택할 수 있지만, observable effect/transfer commit 뒤에는 자동 replay하지 않는다.

##### 첫 adapter 구현 gate

- 하나의 small pure-array region만 지원해도 되지만 capability matrix를 명시한다.
- unsupported dtype/rank/shape/layout가 fail-closed인지 test한다.
- host-side SemanticCheck가 external launch보다 먼저 같은 error를 내는지 differential test한다.
- representation copy/view bridge가 logical atom order를 보존하는지 test한다.
- output ownership/completion 후에만 consumer region이 접근하는지 test한다.
- adapter plan에서 source provenance가 round-trip되는지 test한다.
- external backend를 바꾸어도 같은 verified Logical IR의 J result/error가 유지되는지 비교한다.

현재 **실제 production external adapter가 이 계약을 완료했다는 뜻은 아니다.** M6 이전에는 이 절이 implementation gate 역할만 한다.

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

#### 5.7.5 Cross-stage negative verifier matrix

positive E2E test만으로는 compiler boundary를 보호할 수 없다. 각 stage는 “정상 plan이 통과한다”뿐 아니라 **그 stage가 책임지는 잘못된 상태를 반드시 거부한다**는 negative matrix를 가진다.

| Stage / verifier | 반드시 거부해야 하는 forged/invalid state | 현재/계획 상태 |
|---|---|---|
| J Graph `Plan::verify` | schema/primitive-registry mismatch, invalid ValueId/RegionId, stale region result/stage, malformed pipeline/fork/hook topology, source/fact/rule provenance drift | **현재 존재**. schema는 `J_GRAPH_SCHEMA_VERSION = 0.10`와 exact match |
| rewrite candidate `verify` | stale source span/basis, unregistered rule/witness mismatch, replacement DAG forward reference, fact-rule mismatch, output semantic facts drift | **현재 존재** |
| scan/fusion analysis verifier | forged source order, unsupported rule version, missing/incorrect witness, external-use/retention/fanout drift, candidate를 근거 없이 `selected`로 위조 | **현재 일부 존재**; proof discharge/selection verifier는 future |
| A3 `Plan::verify` | schema/registry mismatch, invalid op/value/block/region references, use-before-def, source/j_origin drift, malformed constraint/check/effect/error/speculation contract, result/write/terminator inconsistency | **현재 존재**. schema는 `A3_SCHEMA_VERSION = 0.6`와 exact match |
| CandidateEvidence / SelectionPlan | stale graph/version evidence, required proof Unknown인데 Selected, Illegal candidate 선택, overlapping incompatible candidates 동시 선택 | **planned** — §4.1.4 |
| RouteRegion / RouteBoundary | missing live-in/out, value-dead but effect-live dependency drop, SemanticCheck 중복/누락/순서변경, region-wide capability 미증명, guard가 effect 뒤에 배치, bridge requirement 누락 | **planned** — §2.5.1 |
| PhysicalPlan | invalid plan buffer/view/op id, use-before-bind, view span overflow, selected kernel capability mismatch, unordered Check, unproved writable overlap/reuse, dangling Return | **planned M4** — §5.2.1 |
| ExternalRegionPlan | source op 누락, check/effect edge drop, declared capability와 emitted op 불일치, incomplete output completion/ownership, unsupported partial module을 success로 표시 | **planned M6** — §5.5.1 |

negative test 이름과 타입은 구현과 함께 정하되, **검증 책임 자체는 stage contract의 일부**다. downstream이 invalid upstream artifact를 관대하게 보정하는 식으로 책임을 이동하지 않는다.

cross-stage forged test의 기본 패턴:

~~~text
valid source
  ↓ build valid artifact
clone artifact
  ↓ mutate exactly one invariant
stage.verify() must fail
  ↓
error identifies the violated boundary
  ↓
no later planner/executor is invoked
~~~

한 test에서 여러 invariant를 동시에 깨뜨리지 않는다. 어느 verifier가 어떤 invariant를 소유하는지 분명하게 유지한다.

#### 5.7.6 Serialization / schema migration policy

현재 RustJ의 J Graph/A3는 주로 in-process compiler artifact이며 장기 portable serialization compatibility를 약속하지 않는다. 현재 verifier는:

~~~text
J Graph schema 0.10      exact match required
A3 schema 0.6           exact match required
PrimitiveRegistry       current REGISTRY_VERSION exact provenance required
~~~

를 기본으로 한다. **minor version이 다르다고 자동 호환으로 간주하지 않는다.** 외부 artifact reader가 생기기 전에는 exact-match fail-closed가 올바른 정책이다.

향후 저장/교환 format을 만들 때 다음 정책을 사용한다.

1. **Decode와 migrate를 분리한다.**
   - wire/file schema를 먼저 안전하게 decode한다.
   - source version별 explicit migration function이 있을 때만 current in-memory schema로 변환한다.
   - 알 수 없는 field/op/rule을 추측해 current 의미로 읽지 않는다.

2. **upgrade는 explicit chain만 허용한다.**

~~~text
v0.n artifact
   ↓ decode with v0.n schema
migrate_0_n_to_0_n1
   ↓
...
   ↓
current schema
   ↓
current verifier
~~~

migration 결과도 반드시 current verifier를 통과해야 한다.

3. **downgrade는 lossless writer가 있을 때만 허용한다.**
   - 새 semantic field/op/effect를 옛 schema가 표현하지 못하면 downgrade를 거부한다.
   - field를 조용히 drop해서 옛 artifact를 만들지 않는다.

4. **PrimitiveRegistryVersion mismatch는 schema mismatch와 별도다.**
   - primitive ID/contract mapping migration이 명시되어 있지 않으면 reject한다.
   - spelling이 같다는 이유만으로 semantic registry version을 무시하지 않는다.

5. **compiler version은 provenance, schema/registry가 compatibility key다.**
   - compiler version이 다르더라도 schema/registry+migration contract가 같을 수 있다.
   - 반대로 같은 compiler version 문자열만으로 compatibility를 보증하지 않는다.

6. **rule/witness registry도 versioned provenance를 유지한다.**
   - rewrite/fusion/scan witness meaning이 바뀌면 stale cached candidate를 재사용하지 않는다.

7. **PhysicalPlan portable cache는 별도 schema다.**
   - Logical IR schema와 같은 version으로 묶지 않는다.
   - target architecture/device/runtime/capability fingerprint를 함께 요구한다.
   - device-specific cached plan miss는 J error가 아니라 cache/route miss다.

8. **unsupported-version diagnostics는 semantic J error와 분리한다.**
   - `UnsupportedSchema/Registry/Migration` 계열 compiler diagnostic으로 보고한다.
   - Domain/Rank/Length 같은 J error로 위장하지 않는다.

pre-1.0 개발 단계에서는 schema를 자주 올릴 수 있다. 그 대신 version bump 없이 semantic field meaning을 바꾸는 것을 금지한다. portable artifact compatibility를 공식 약속하기 전에도 이 규율을 지킨다.

---

<a id="logical-physical-array-model"></a>

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

반대로 수명·alias·effect 조건이 검증된 여러 ValueId가 같은 BufferId를 재사용할 수도 있다. Logical ArrayValue가 존재한다고 별도 buffer가 필요한 것은 아니다. view, fused-away intermediate, rematerialized value의 실제 저장과 reshape/transpose/reverse/slice의 copy 여부는 downstream planner가 결정한다.

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


<a id="out-of-core-io-contract"></a>

### 8.5 느린 I/O·Out-of-core 배열 실행 계약 (2026-10-06, 설계 후보 / 구현 미착수)

**목표.** RAM/GPU 용량보다 큰 J noun·신경망 가중치·배열 데이터에 대해 (a) 원래 J 결과·오류·효과 순서를 보존하고, (b) 실제로 읽는 바이트와 복사량을 줄이며, (c) 필요 시 디스크/원격 데이터 이동과 계산을 중첩한다. 이것은 새로운 J syntax나 암묵적 GPU 구현이 아니라, §5의 Physical Planning/Execution Scheduling에 따른 물리 실행 확장이다. **최초 native CPU M4 vertical slice의 선행 조건으로 삼지 않는다.** 모든 기능은 [§10 IO-01~IO-30 수용 게이트](#out-of-core-io-checklist)를 통과하기 전에는 계획/후보일 뿐이다.

**원본/선행 기법 비교(직접 계승 vs 조건부 응용).**

| 출처 / 원본 | 검증한 아이디어 | RustJ 차용 범위 / 금지되는 비약 |
|---|---|---|
| Jsource `jlibrary/addons/data/jmf/jmf.ijs` | J noun에 file mapping; R/W·read-only·copy-on-write, header/shape와 unmap 참조 제약. non-jmf typed boxed mapping 제한과 JMF-backed boxed 회귀 fixture가 공존 | mapped dense array backend 후보. mmap=비동기 I/O·zero page fault·J 전체 boxed/sparse 지원으로 해석하지 않음 |
| Jsource `jsrc/xf.c`, J foreign `1!:11`/`1!:12` | file offset+length의 부분 read/write; 현재 C 경로는 `fread/fwrite` 기반 동기식 | 순차 byte-range 기준 경로와 J-visible foreign I/O 의미 비교. 그래프에서 effectful file foreign을 순수 scan으로 자동 대체 금지 |
| Jsource `jsrc/v.c` 등 | reference/in-place 가능 조건에 따라 버퍼 재사용·복사 억제 | alias/lifetime proof 후 reuse; mapped write와 implicit in-place를 동일시하지 않음 |
| Jd (J 데이터 라이브러리) | 열별 파일 매핑과 파티션 조건별 selective read를 실제 소스에서 확인 | 데이터셋 관리 원리를 참고하되 full-J 의미론으로 일반화하지 않음. Jd column-file on-demand mapping, partition-column read pruning, SIMD mapped-tail 방어를 소스에서 확인 |
| DuckDB (2026 async I/O) | compute pool·blocking I/O pool 분리, job/read-ahead, 메모리 governor 연동, park/resume | bounded read-ahead·메모리 예약·작업 완료 이벤트. DuckDB 전체 스케줄러 복제 불필요 |
| Polars Lazy | projection/predicate/slice pushdown, common subplan reuse | 정확한 access/effect/witness가 있을 때만 필요 byte-range 축소; 임의 J verb·reduction에 무조건 적용 금지 |
| Apache Arrow Dataset Scanner | `batch_readahead`와 `fragment_readahead`의 다른 단위, metadata/pruning | chunk/fragment 단위 선택과 bounded concurrency; 전체 Arrow 데이터모델 복제 불필요 |
| Ray Data | streaming block queues, downstream backpressure, spill/resource account | 독립 실행 단계의 bounded queue·pressure handling. shuffle/reduce barrier를 무시한 임의 streaming 금지 |
| DeepSpeed ZeRO-Infinity | CPU/NVMe/GPU offload와 transfer/compute overlap, prefetch | 추후 placement-aware weight prefetch; GPU runtime 보류 정책 유지 |
| FlexGen (ICML 2023) | device/CPU/disk placement와 layer/batch-block scheduling의 I/O 재사용·throughput tradeoff | 안정된 read-only weights와 합법적 scheduling 영역에 한해 중복 로드 감소; J 실행 순서 임의 변경 금지 |
| TensorFlow `tf.data` | prefetch/parallel map/overlap pipeline | 독립된 데이터 공급 단계의 후속 비교 후보; full J 적용 보장 아님 |

출처(확인 기준 2026-10-06): [Jsource jmf](https://github.com/jsoftware/jsource/blob/master/jlibrary/addons/data/jmf/jmf.ijs) · [Jsource xf.c](https://github.com/jsoftware/jsource/blob/master/jsrc/xf.c) · [DuckDB async I/O](https://duckdb.org/2026/07/31/asynchronous-io) · [Polars optimizer](https://docs.pola.rs/user-guide/lazy/optimizations/) · [Arrow Scanner](https://arrow.apache.org/docs/python/generated/pyarrow.dataset.Scanner.html) · [Ray Data execution](https://docs.ray.io/en/latest/data/data-internals.html) · [ZeRO-Infinity](https://www.deepspeed.ai/tutorials/zero/) · [FlexGen 논문](https://proceedings.mlr.press/v202/sheng23a.html). 프레임워크의 공개된 설계를 참고하는 것이며 해당 기능이 RustJ에 구현됐다는 뜻이 아니다.

**계층 소유권·공통 불변조건**

| 계층 | 입력/출력과 책임 | 금지 |
|---|---|---|
| J Semantic IR / J Graph IR | J noun의 type/shape/atom-order, boxed/sparse, Rank zero-frame prototype, access/effect/error 순서; 순수한 기회 후보만 sidecar로 표시 | 파일 offset·chunk 크기·prefetch queue·장치 DMA를 J-visible 연산 의미로 삽입 금지 |
| Verified Logical IR / analyzer | `AccessRelation`·필요 데이터 범위/재사용 가능성, unknown/opaque, purity와 `FactWitness/Guard` 유지. index/select/view에 대해 합법적 범위 pruning 후보 생성 | arbitrary verb/semantic error가 관찰 가능한 입력을 검사 없이 생략 금지 |
| Physical Planner / Schedule | `StorageSource`(Owned/Mapped/Chunked/Remote 후보), placement, byte-range, Read/Prefetch/Transfer/Materialize/Release, dependency, memory budget, cost 후보 | speculative I/O의 J-visible error/효과 재배치, semantic `StorageRequirement`을 materialization으로 동치화 금지 |
| Executor / Runtime | 처음에는 동기 `read_at`/`write_at` + chunk iteration; 이후 pending/ready/error/cancel completion, pinned lease, backpressure, finite queues | buffer lifetime을 I/O completion 전에 종료, 외부 변경된 bytes를 version 불변으로 가정, 완료 전 쓰기를 성공으로 보고 금지 |
| External adapters | 실제 backend의 read/write/async/mmap capability, errors, alignment, atomic/ordering, ownership을 선언 | J semantics 또는 non-CPU 성능을 암묵적 fallback 능력으로 가장 금지 |

**별도 identity.** `ValueId`는 J 논리값, `StateResource`는 변경 가능한 가중치/파일 상태의 의미적 identity, `BufferId/BufferLease`는 실제 메모리 소유자, `StorageObjectId/Version`은 외부 backing object와 읽기 일관성, `IoRequestId/CompletionToken`은 I/O 요청·완료를 식별한다. 하나로 합치지 않는다. `StorageObjectId/Version` 및 `IoRequestId`는 **명명 후보**이고 확정 Rust 타입/API가 아니다.

**읽기 계획과 정당성.** offset/length 계산은 checked arithmetic·overflow 검사, 정렬·EOF/short read·stale version·권한 실패를 정의한다. read-only immutable input 또는 스냅샷 증명 없는 speculative prefetch는 효과·오류 순서에 영향을 줄 수 있으므로 금지/보수적 barrier를 둔다. chunk size·layout을 바꾸어도 J shape/rank, boxed/sparse, empty frame/fill-cell result, tolerant comparison, 예외 발생 위치가 달라지면 안 된다. 빈 결과의 데이터 byte-range는 0일 수 있지만 J Rank의 가상 cell 실행/형식 추론 의무는 독립이다.

**실행 원칙.** 우선 동기 contiguous/chunked CPU 경로로 correctness baseline을 만든다. 다음에 source access proof로 scan bytes를 줄인다. 그 뒤 bounded async I/O queue와 적어도 2개의 버퍼로 `Read(n+1)`과 `Compute(n)`를 overlap한다. prefetch는 예약 memory budget와 consumer progress/backpressure로 제한하고, request-completion을 `AsyncToken/Timepoint`/equivalent dependency로 명시한다. weight reuse/batch-block schedule은 read-only/version stable, dependency/effect/latency-policy 합법성 증명 후에만 선택한다. `mmap`은 독립 후보이며 page-fault 비용·page cache·주소공간·unmap·COW semantics를 직접 측정한다. `io_uring`·direct I/O·remote object store·GPU DMA는 portable baseline 이전의 요구사항이 아니다.

**쓰기는 읽기와 분리.** J foreign 파일 쓰기·mapped shared write의 observable effect와 checkpoint persistence는 동일하지 않다. checkpoint는 immutable snapshot/version capture → 임시 파일/쓰기 완료 → 필요한 flush/fsync 계약 → atomic publish/rename 가능성 검사 → 성공 반환으로 구분한다. 플랫폼별 durability 보장과 crash consistency를 실제 지원 범위로 한정한다. delayed save로 user-visible state/error 순서가 바뀌면 최적화 금지. cancellation/partial write/retry에서 중복 side effect와 파일 파손을 검사한다.

**비용/계측 계약.** `ResourceEstimate`(peak resident, pinned/inflight bytes, open handles, max queued jobs)와 `CostEstimate`(cold/warm bytes, number/seeks/latency, effective bandwidth, CPU cycles, transfer/overlap time)를 분리한다. 성능 게이트에서 wall time, read/write bytes, physical requests, blocking wait, compute time, peak+retained memory, page faults/cache hits, spill count, throughput *및* latency를 비교한다. warm page cache 결과를 cold storage 승리로 포장하지 않는다.

**대표 실행 예.** `W1`·`W2`가 파일에 저장된 순수한 read-only tensor라고 증명되면 physical schedule은 `Prefetch(W1) → Await(W1) → Compute(W1)`와, 그 계산 중 `Prefetch(W2)`를 겹친다. 다음 `Compute(W2)`는 앞 층 결과 및 W2 완료를 모두 기다린다. Logical graph에는 이 I/O 순서를 박지 않는다. 분기/동적 NAME/수정 가능한 weight/관찰 가능한 file effect가 있는 경우 이 순서를 무조건 적용하지 않는다. 수용 작업표는 [§10 IO](#out-of-core-io-checklist)를 따른다.


#### 8.5.1 독립 재검토 — J 라이브러리·포맷·모델 로더 보강 (2026-10-06)

**발견:** 기존 §8.5는 async read-ahead에 비해 실제 저장 포맷, read-chunk/write-shard 배치, mapped SIMD tail, refcount/unmap, cache 일관성 계약이 약했다. 다음은 원본 구현/공식 설명과 RustJ 후보를 분리한 표이다. 소스나 문서 존재는 RustJ 구현 성공이 아니다.

| 확인한 출처 | 실제 구조 / 기능 | RustJ 적용·유의사항 |
|---|---|---|
| [J jfiles와 keyfiles](https://github.com/jsoftware/jsource/tree/0a5101cfdd834b23a0b89d455e4f327310520a08/jlibrary/addons/data/jfiles) | serialized J components를 offset-indexed read로 접근, keys로 개별 component 검색 | arbitrary serialized noun은 typed dense mapped bytes와 다른 encoding. 전체 jfiles 포맷 복제는 보류 |
| [Jd column.ijs](https://github.com/jsoftware/data_jd/blob/0492991263a05bafa84ceca15f0f8249cfc62dcf/base/column.ijs) | file-backed column map on demand, grow/remap, multi-process reference-count 위험 주석 | remap/lease/alias/동시 접근 검사; Jd의 DB lock을 일반 J noun 규칙으로 복제 금지 |
| [Jd api_read.ijs](https://github.com/jsoftware/data_jd/blob/0492991263a05bafa84ceca15f0f8249cfc62dcf/api/api_read.ijs) · [ptable 튜토리얼](https://www.jsoftware.com/jd_tuts.html) | partition column 조건으로 대상 파티션을 좁혀 읽기 | 실제 partition pruning 선례. arbitrary J verb의 자동 pushdown은 별도 effect/error/witness proof가 필요 |
| [Jd jmfx.ijs](https://github.com/jsoftware/data_jd/blob/0492991263a05bafa84ceca15f0f8249cfc62dcf/base/jmfx.ijs) | SIMD kernel의 파일 끝 beyond-end 접근을 우려해 padding; 4096 byte 페이지 가정 | **반면교사**: RustJ는 페이지 크기·허용 overread를 하드코딩하지 않고 masked tail/checked span/unmap-safe lease 구현 |
| [Zarr v3](https://zarr.readthedocs.io/en/stable/user-guide/arrays/) · [HDF5 chunk cache](https://docs.h5py.org/en/stable/high/file.html) | 논리 shape와 read chunk, write shard/캐시를 분리; access-pattern-dependent chunk shape | chunk read amplification, 파일 개수, shard write amplification과 메모리 경계 최적화; 자체 파일 포맷 확정 보류 |
| [Safetensors](https://github.com/huggingface/safetensors/blob/main/README.md) | tensor dtype/shape/byte-range index, 0-byte empty tensor, partial/lazy loads | verified storage manifest를 제안; invalid/overlap/overflow/off-end 검증, scalar/empty prototype 보존. 외부 포맷 adapter는 후속 |
| [llama.cpp loader](https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md) | mmap/no-mmap/mlock/direct-I/O/NUMA/lazy tensor rows 선택 | mmap을 universal default로 결정하지 말고 workload/OS cache/RSS/page faults/cold-warm으로 비교 |
| [IREE Stream dialect](https://iree.dev/reference/mlir-dialects/Stream/) | parameter.load/read/gather/write, AsyncTransfer, Timepoint로 외부 가중치 바이트 범위의 비동기 준비·저장과 완료 시점·resource lifetime 명시 | **직접 참고 1순위**: 정적인 read-only weights → Physical Stream ParameterRead/Load 후보 → Await completion → consumer, checkpoint write의 completion은 별도 durability 계약. J-visible foreign effect와 동일시 금지 |
| DuckDB, Polars, Arrow, Ray, ZeRO-Infinity, FlexGen | scan pruning, batch/fragment read-ahead, async governor, streaming pressure, staged weight reuse | read bytes 축소 → stable data version → bounded scheduling → overlap → legal reuse 순서로만 도입 |

**Boxed 표현 정정:** [jmf.ijs](https://github.com/jsoftware/jsource/blob/0a5101cfdd834b23a0b89d455e4f327310520a08/jlibrary/addons/data/jmf/jmf.ijs)는 non-jmf typed boxed mapping을 거부하지만 [JMF-backed boxed tests](https://github.com/jsoftware/jsource/blob/0a5101cfdd834b23a0b89d455e4f327310520a08/test/gmbx.ijs)가 존재한다. 반면 [jsrc/mbx.c](https://github.com/jsoftware/jsource/blob/0a5101cfdd834b23a0b89d455e4f327310520a08/jsrc/mbx.c)는 not supported라고 표시된다. 따라서 지원/미지원은 파일 형태·연산·원본 버전의 실제 C oracle로 세분화해야 한다.


**IREE Stream 의존성 경계:** `stream.async.parameter.load/read/gather/write`, `stream.async.transfer` 및 `stream.timepoint`의 구현을 I/O-specific Physical Schedule 선례로 추가한다. `parameter.load`의 사전 실행 허가와 J 외부 파일 읽기 효과의 비투기성은 다르며, buffer 사용 가능 시점 및 cancellation/error가 보존될 때만 이동·중복 제거한다. IO-13~18과 IO-20의 verifier 검토 입력으로 사용한다. [IREE 공식 Stream 사양](https://iree.dev/reference/mlir-dialects/Stream/).

**추가 계층 계약:** logical ValueId와 외부 StorageObject/Version, 물리 StorageEncoding(contiguous typed / chunked typed / serialized component / external), ReadChunk, WriteShard, BufferLease, IoCompletion을 서로 구분한다. 이들은 구현을 확정한 Rust 구조체 이름이 아닌 개념이다. File foreign I/O는 J-visible effect이지만, verified immutable storage read의 내부 스케줄은 효과가 없는 경우에 한해 변경 가능하다. Header/shape/index가 존재한다고 J Rank empty-frame fill-cell 의미를 생략하지 않는다. Cache key에는 object/version/byte-range/encoding을 포함하고, peak RSS와 OS page cache는 런타임이 완전히 예약하는 메모리로 주장하지 않는다.


<a id="io-framework-execution-comparison"></a>

#### 8.5.2 프레임워크별 I/O 최적화 실행 방식과 RustJ Graph IR 적용 (2026-10-06)

**다섯 가지 최적화 대상.** I/O 최적화는 단순한 비동기 파일 읽기가 아니라 **① 불필요한 읽기 제거, ② 저장/읽기 단위 선택, ③ I/O와 계산 중첩, ④ 읽은 데이터 재사용, ⑤ 메모리·완료·실패 통제**를 조합하는 문제다. 최적화의 장소를 구분해야 한다. J Graph/Verified Logical IR은 필요한 데이터와 의미론적 합법성을 판단하고, Physical Plan·Executor는 byte range, chunk, 배치, prefetch, buffer 수명, 실제 memory/resource budget을 결정한다.

| 프레임워크 / 공식 근거 | 실제 기법과 최적화 계층 | RustJ에서 가져올 부분 / 제약 |
|---|---|---|
| [Polars lazy optimizer](https://docs.pola.rs/user-guide/lazy/optimizations/) | Logical 단계에서 predicate/projection/slice를 scan까지 pushdown, common subplan/scan 재사용 | `AccessRelation`이 필요한 논리 원소/축과 실제 byte range를 증명한 경우 읽기를 제거. IO-09~12; Rank, dynamic binding, observable errors/effects에서는 opaque fallback |
| [DuckDB async I/O (2026-07-31 발표)](https://duckdb.org/2026/07/31/asynchronous-io) | Parquet row-group/CSV byte-boundary를 fetch job/task로 쪼갬. `REGULAR` 계산 pool과 주로 blocking I/O인 `ASYNC` pool 분리; 미완료 scan job은 park 후 깨움. read-ahead 깊이는 temporary-memory budget과 연동 | IO-13~17: bounded job queue, completion/wakeup, read-ahead depth를 resource governor로 축소. 공개 글은 v2.0 개발·출시 예정 기능을 설명하므로 모든 릴리스에서 지원된다고 단정하지 않음 |
| [IREE Stream](https://iree.dev/reference/mlir-dialects/Stream/) | Physical resource IR에서 `stream.async.parameter.load`는 새 resource, `read`는 기존 allocation 채움, `gather`는 여러 parameter archive range 결합. `timepoint`/`await`로 이용 가능 시점과 순서 명시 | IO-13~18·20의 직접 선례: immutable weights `Load/Gather → Await → Consumer`. IREE에서 speculative한 parameter read를 J의 관찰 가능한 `1!:` file foreign과 동치화 금지 |
| [Apache Arrow Scanner](https://arrow.apache.org/docs/python/generated/pyarrow.dataset.Scanner.html) | `batch_readahead`와 `fragment_readahead`를 따로 조정 | IO-14~16: chunk 내부와 여러 저장 source 간의 선행 읽기 수준을 분리하고 decoded/inflight buffers 예산화 |
| [Ray Data internals](https://docs.ray.io/en/latest/data/data-internals.html) | 연산자 output queue에 block reference streaming; downstream backpressure, memory-aware task scheduling, spill | IO-15~17: 큐가 차면 생산 속도를 제한. Reduction/shuffle처럼 경계가 필요한 경우 무조건 streaming하지 않음 |
| [Zarr arrays/sharding](https://zarr.readthedocs.io/en/stable/user-guide/arrays/) · [HDF5 chunk cache](https://docs.h5py.org/en/stable/high/file.html) | 물리 chunk·shard 및 캐시/축 접근 패턴으로 read amplification와 재사용 결정 | IO-26~29: `LogicalShape` ≠ `ReadChunk` ≠ `WriteShard`; 작은 slice도 chunk 전체 decode가 필요한지 측정 |
| [llama.cpp](https://github.com/ggml-org/llama.cpp) | 모델 가중치의 mmap/비-mmap·residency 등 로딩 전략 선택 | IO-23·30: mmap이 항상 최적이라고 가정하지 않음. page fault, cold/warm cache, RAM pressure, SSD/remote, reuse별 비교 |
| [DeepSpeed ZeRO](https://www.deepspeed.ai/tutorials/zero/) · [FlexGen](https://proceedings.mlr.press/v202/sheng23a.html) | CPU/NVMe/GPU Offload·prefetch·transfer/compute overlap; FlexGen은 layer/batch-block scheduling으로 weights 반복 로드 감소 | IO-18~20: immutable/version-stable weights에서만 재사용/선행 읽기를 후보화. 훈련의 mutable optimizer/gradient/checkpoint는 별도 snapshot 계약 |
| [TensorFlow tf.data](https://www.tensorflow.org/guide/data_performance) | `prefetch`·parallel map을 통한 input-producer와 모델 연산의 파이프라인 중첩 | IO-14~16: 독립 producer/consumer의 비교 사례. J의 임의 verb/cell 평가를 재배열하는 일반적 허가는 아님 |
| [PyTorch Distributed Checkpoint](https://pytorch.org/docs/stable/distributed.checkpoint.html) | 저장할 state의 staging/async persistence 분리 | IO-18: mutable state의 snapshot·write completion·publishing·failure/recovery를 명시. 비동기 제출만으로 durability 성공을 보고하지 않음 |

**같은 Logical Graph에서 다른 Physical Schedule을 선택하는 예.** `X → MatMul(W1) → Activation → MatMul(W2) → Y`의 J-visible 의존성을 유지한다. 외부 파일의 `W1`/`W2`가 **변하지 않는 read-only/versioned 데이터임이 증명**된 경우, 아래 physical 계획을 생성할 수 있다. 다른 모델에서 임의로 실행되는 것이라고 가정하지 않으며, 실제 구현 이전 설계 예시다.

~~~text
Logical / semantic value flow
X ──> MatMul(W1) ──> Activation ──> MatMul(W2) ──> Y

Physical / possible overlap
Reserve(W1) ──> Read(W1) ──> Ready(W1) ──> Compute(L1) ──> Activation ──┐
Reserve(W2) ──> Read(W2) ──> Ready(W2) ─────────────────────────────────┤
                                                                        ▼
                                                                  Compute(L2) ──> Y
            Read(W2) and Compute(L1) can overlap (if budget allows)
Compute(L2) awaits BOTH Activation and Ready(W2).
Release(buffer) follows last use AND all pending I/O/transfer completions.
~~~

**차용 방법의 핵심 차이.** Polars 방식은 **필요 없는 byte를 아예 요청하지 않도록** 만드는 기법이다. IREE 방식은 **읽기·전송·완료 토큰을 Physical DAG의 명시적 의존성**으로 만드는 기법이다. DuckDB·Arrow·Ray 방식은 **그 DAG의 읽기 요청을 bounded queue·backpressure·memory budget 아래에서 실행**하는 기법이다. DeepSpeed·FlexGen 방식은 **합법적 계산 순서/배치에서 weights의 생존 구간과 재사용을 늘려** 실제 반복 I/O를 줄인다. 이 기법들은 대체 관계가 아닌 계층별 조합 후보이다.

**의미론 경계와 비용 계약.** (A) verified immutable 저장소의 내부 물리 read, (B) J `1!:11`/`1!:12` 등 관찰 가능한 foreign I/O, (C) 수정 가능한 weights·checkpoint 쓰기를 별도로 모델링한다. A에서만 access witness/guard를 통한 범위 pruning, speculative prefetch, range coalescing을 허용한다. B는 J effect/error 순서, C는 version/snapshot/commit/durability 계약을 지킨다. 빈 Frame의 read bytes가 0이어도 J Rank의 가상 Cell·fill·dtype·shape 및 오류 판단은 생략하지 않는다. boxed/sparse, dynamic NAME/Rank, alias, stale file, EOF/short read 및 실패의 조기 관찰은 negative fixture 대상이다.

성능은 **cold/warm 실제 읽은 bytes, request 수, seek/latency, blocked I/O 시간, compute time, page faults, prefetch overlap, peak·retained memory/RSS, spill, 전체 throughput, 개별 호출 latency**를 따로 측정한다. Read-ahead 자체는 필요한 byte를 줄이지 못하며 메모리를 더 사용한다. 작은 range를 합치면 요청 수는 줄지만 초과 읽기가 발생할 수 있다. OS page cache/RSS는 모든 바이트를 RustJ governor에서 정확히 예약할 수 있는 메모리라고 주장하지 않는다. **의미론·자원 안전 → 실측 비용 → 실행 후보 선택** 순서를 유지한다.

**기존 단일 수용 원장 매핑:** 자료 읽기 제거·공통 scan/cache = IO-09~12·29, 파일/Chunk/Shard 형식 = IO-05~08·26~28, async/backpressure/transfer = IO-13~17·20, weights/checkpoint = IO-18~19, benchmark/선택 = IO-21~24·30, 소스·의미론 = IO-01~04·25. 새 체크리스트를 만들지 않고 [§10 IO-01~IO-30](#out-of-core-io-checklist)의 기존 상태만 갱신한다. 지금은 설계 기록이므로 **0/30 수용** 상태를 유지한다.


<a id="unified-data-movement-contract"></a>
#### 8.5.3 메모리 I/O·디스크 I/O 공통 Planning 계약 — 신규 IR 계층은 당장 만들지 않음 (2026-10-06)

**설계 결정.** 메모리 접근·복사, CPU↔GPU 이동, 파일/매핑된 배열의 구간 읽기·쓰기를 **기존 Physical Planner / Physical Execution Plan의 공통 데이터 이동 분석과 스케줄링 대상으로 취급**한다. `Data Movement IR`이라는 독립적인 신규 계층은 만들지 않는다. 대신 `AccessRegion` 증명, `Effect/Dependency` 계약, physical storage/transfer/ready/lifetime 정보를 기존 단계의 각 소유자가 보유한다. 추후 실제 비동기·복합 route 사례 둘 이상에서 기존 Physical Plan이 의존성·버퍼 수명·스케줄을 안전하게 표현하지 못한다는 구체적 증거가 나올 때에만 별도 resource/stream 실행 IR 추출을 재검토한다. 이것은 §5의 기존 J Graph IR → Verified Logical Execution IR → Physical Plan 구분을 없애는 결정이 아니다.

| 기존 단계 | 이 단계에서 표현하거나 계산하는 것 | 넣어서는 안 되는 것 |
|---|---|---|
| J Semantic IR / J Graph IR | J source topology, ValueId·Rank/CellApply·zero-frame 의미, 원본/후보 provenance; pass-local 접근 후보 sidecar는 허용 | BufferId, 파일 offset, DMA, concrete transfer, 물리 layout 및 GraphFacts 자체의 EffectSummary/선택 완료 상태 |
| Verified Logical Execution IR / 분석 | observable J file/namespace/state effect, error/ordering/guard, 값/효과 liveness, logical `AccessRelation` 및 Unknown/Proven/Guarded 증거 | 임의 file read를 pure 배열 load로 치환하거나, 증명 없는 read omission/reorder 승인 |
| 기존 Physical Planner / Representation / Schedule | `StorageObject/Version`·`BufferId/Lease` 분리, physical `Region`(buffer slice/byte range), `Read/Write/Copy/Transfer/Materialize/Release`, ready/completion 의존성, layout·placement·byte/resource/cost 추정; 공유 pass 인터페이스 사용 | 모든 접근을 실제 복사로 강제, 효과가 다른 파일과 메모리를 단일 semantics로 취급, target capability 없는 전송 확정 |
| Executor / Backend | 동기 CPU reference에서 실제 read_at/write_at/buffer copy 실행; 이후 검증된 async token, queue/backpressure, 장치별 구현 | 미완료 I/O의 조기 buffer 해제, file side effect 재실행, 미확정 GPU 실행 지원 주장 |

**공통 추상화의 경계.** `AccessRegion`은 어떤 논리 원소가 필요한지를 설명하며 physical `BufferSlice`와 `FileByteRange`는 다른 concrete region 종류다. 공통 분석은 region interval, producer/consumer, placement, lifetime, alias, ordering, transfer feasibility와 비용을 비교하지만 파일과 메모리를 하나의 alias domain·주소공간·오류 계약으로 뭉개지 않는다. `ValueId`, mutable `StateResource`, external `StorageObjectId/Version`, physical `BufferId/BufferLease`, `IoRequestId/CompletionToken`은 계속 분리한다. 이름은 설계 후보이며 확정 Rust API가 아니다.

**컴파일 타임 vs 런타임.** 컴파일 타임에 proven shape/dtype/access pattern, 사용/수명, 중간 배열 제거, buffer reuse, 필요한 데이터 구간·전송·파이프라인 후보를 계획한다. 런타임은 실제 파일 내용/변경·EOF·권한·데이터 크기, 사용 가능한 RAM/GPU 메모리, 페이지 캐시·대역폭·요청 완료를 확인하고 guard/capability/memory budget을 검사한다. 비용 모델은 memory traffic, file bytes/requests/seeks, transfer, kernel launch, sync, peak/inflight/retained bytes, cold/warm latency를 분리한다. Unknown을 0 비용 또는 재배치 허가로 해석하지 않는다.

**효과 안전성.** (A) proven immutable/version-stable 내부 backing read, (B) J `1!:`의 관찰 가능한 파일 I/O, (C) mutable state/checkpoint publish는 별개다. (A)에서도 실패가 관찰될 수 있으면 speculative read/오류 노출 위치가 보존된다는 별도 증명이 필요하다. (B)의 미사용 읽기라도 존재성/권한/오류를 생략하지 않으며, (C)의 쓰기 완료·durability를 단순 async submit으로 간주하지 않는다. zero-byte/empty frame에서도 J의 virtual fill-cell, dtype/shape 및 필요한 오류 검사는 남긴다.

**프레임워크 역할 분담(직접 복제 금지).** [IREE Stream](https://iree.dev/reference/mlir-dialects/Stream/)은 resource·async transfer·parameter/file read/write·ready token의 **물리 실행 그래프** 선례다. [MLIR Bufferization](https://mlir.llvm.org/docs/Bufferization/) 및 [Memory Effects](https://mlir.llvm.org/docs/Rationale/SideEffectsAndSpeculation/)는 논리값↔버퍼 분리와 operation별 effect/resource interface의 선례다. [XLA GPU architecture](https://openxla.org/xla/gpu_architecture)는 fusion·buffer assignment·layout/transfer를 점진적으로 구체화하는 선례이며 일반적인 J 파일 I/O의 공통 IR은 아니다. [TVM](https://tvm.apache.org/docs/)은 장치 배치·물리 메모리 선택, [DataFusion](https://datafusion.apache.org/)은 파일 scan과 filter/projection pushdown의 선례다. 어떤 프레임워크도 full-J foreign 오류·Rank prototype 계약을 자동으로 해결하지 않는다.

**대표 추적 사례.** `File(A, immutable version) → Slice → Elementwise → Consumer`는 verified access witness가 있을 때 physical byte-range read와 fused CPU 계산으로 낮출 수 있다. 반면 `1!:1`/`1!:11` 결과가 사용되지 않아도 파일 open/EOF/error 등 J-observable 경계가 있다면 dead-data elimination만으로 읽기를 지울 수 없다. CPU↔GPU copy는 같은 planner의 transfer 후보지만 현재 CUDA 구현 유보 상태를 유지한다.

**기존 이행 원장 연결.** 새 `DM-*` 작업표는 만들지 않는다. IO-03(계층/identity/effect interface), IO-09(접근 영역 증명), IO-13·14(동기→비동기 준비/완료), IO-17(재배치·실패·해제 안전), IO-20(공통 memory/file/transfer 자원·비용 planning), IO-22(세 경로·negative 검증)를 기존 [§10 IO 체크리스트](#out-of-core-io-checklist)의 수용 항목으로 사용한다. FW-05~09의 provenance/guard와 DB effect contract를 선행 증명으로 재사용한다. **현 상태: 설계 확정, 구현·실행 증거 없음; IO 0/30 [ ] 유지. M2 frontend와 최초 M4 CPU slice의 필수 조건으로 올리지 않는다.**

<a id="io-a-source-audit"></a>
#### 8.5.4 IO-A 원본 소스 교차 감사·실행 기준 (2026-10-06~07)

**범위·상태.** IO-01·IO-25의 1차 코드 감사를 pinned source에서 수행했고 IO-02의 effect/error 계약이 요구하는 반례를 도출했다. 이것은 **소스 확인만 완료한 부분 진행**이다. 원본 J 파일 foreign 일부 fixture는 실행했으나 mapped/Jd/전체 외부 효과 검증은 남아 있으므로 IO-01·02·25는 [ ]이고 IO 전체 수용 0/30을 유지한다. 외부 저장소의 동작은 RustJ 지원 사실이 아니다.

| 고정 원본과 확인 위치 | 원본에서 직접 확인한 내용 | RustJ에서 아직 증명할 사항 |
|---|---|---|
| [jsource `xf.c` @0a5101c](https://github.com/jsoftware/jsource/blob/0a5101cfdd834b23a0b89d455e4f327310520a08/jsrc/xf.c) `jtjiread` / `jtjiwrite` / `jtixin` | `1!:11`은 file open/size 확인 뒤 index·length를 검사하며 `j≤size, j+length≤size, length≥0`을 확인한다. `1!:12`는 `n=null` 호출이므로 같은 read end-bound를 강제하지 않고 nonnegative 시작 index를 확인한 뒤 write한다. 음수 시작 index는 size 상대 계산을 거친다. `jtrd/jtwa`는 동기 `fread/fwrite` 경로다. | invalid filename/open과 index error 우선순위, EOF/short I/O, write beyond EOF·파일 변경·권한·효과 후 재실행 금지. 단편 소스만으로 동시 truncate의 결과/atomicity를 보장하지 않음 |
| [jsource `jmf.ijs` @0a5101c](https://github.com/jsoftware/jsource/blob/0a5101cfdd834b23a0b89d455e4f327310520a08/jlibrary/addons/data/jmf/jmf.ijs) + [`gmbx.ijs`](https://github.com/jsoftware/jsource/blob/0a5101cfdd834b23a0b89d455e4f327310520a08/test/gmbx.ijs) | mapping mode 0=RW, 1=RO, 2=COW가 구분되고 unmap은 live refs에 의해 실패할 수 있다. `additem`은 type 32 boxed에 `not supported for boxed data` 검사를 둔다. 반면 `gmbx.ijs`는 mapped boxed 명칭을 사용하지만 확인한 실제 assertion은 `'' -: q`, `'' -: r`로, 일반 boxed payload의 읽기·쓰기 성공을 입증하지 않는다. | JMF 포맷·타입·모드별 boxed 경계를 실행 oracle로 정확히 구별; copy-on-write commit과 shared write 혼동 금지 |
| [data_jd `column.ijs` @0492991](https://github.com/jsoftware/data_jd/blob/0492991263a05bafa84ceca15f0f8249cfc62dcf/base/column.ijs) · [`jmfx.ijs`](https://github.com/jsoftware/data_jd/blob/0492991263a05bafa84ceca15f0f8249cfc62dcf/base/jmfx.ijs) | column map/remap과 타 프로세스 refcount 위험 주석, DB lock 기반 제한을 확인. `jmfx.ijs`의 overread 방어에는 `PAGESIZE=:4096` 가정과 trailing padding 보정이 들어 있다. | RustJ는 페이지 크기·SIMD overfetch를 하드코딩하지 않고 checked span/masked tail 및 remap·lease safety 실측; Jd lock을 J 의미 계약으로 일반화 금지 |
| [jsource `jfiles.ijs` @0a5101c](https://github.com/jsoftware/jsource/blob/0a5101cfdd834b23a0b89d455e4f327310520a08/jlibrary/addons/data/jfiles/jfiles.ijs) · [data_jd `api_read.ijs` @0492991](https://github.com/jsoftware/data_jd/blob/0492991263a05bafa84ceca15f0f8249cfc62dcf/api/api_read.ijs) | jfiles `j_read`는 `3!:2 @ (1!:11)`로 offset 기반 serialized component를 읽으며, `jread`가 directory index에서 범위를 취득한다. Jd `readptable`은 partition 조건으로 대상 table을 결정한다. | serialized component와 typed contiguous mapped backing 구분. Jd의 선택적 읽기를 임의 J verb/foreign read에 자동 적용 금지 |

**잔여 실행 fixture (IO-01/02/25 수용 선행).** 동일한 임시 파일을 대상으로 정상 `1!:11` 부분 읽기, 끝 위치·길이 경계, negative relative offset, `1!:12` EOF 이후 쓰기, 존재하지 않는 파일과 권한 오류, 처리 중 외부 변경·단축 읽기, 번호형 핸들과 파일명 입력의 오류/해제 차이를 고정 J C binary에서 관찰한다. JMF는 RW/RO/COW·live-ref unmap·mapped boxed 명칭 대비 실제 payload 종류·resize/remap을, Jd/jfiles는 partition/keyed component·serialized byte-range를 별도 원본 fixture로 조사한다. **예상 오류 클래스나 처리 순서를 실행 전에 합격 기준으로 단정하지 않고 observed J behavior를 baseline으로 고정**한다. 결과는 IO 원장의 commit·원본 pin·명령·실행환경·pass/fail/unsupported에 기록한다.

**IO-A 실행형 진단 도구 (2026-10-06).** [tools/file_io_audit.py](tools/file_io_audit.py)에 임시 파일만 사용하는 15개 고정 J C oracle 사례를 작성했다. 전체 읽기·크기, 정상 구간·EOF 0길이·음수 시작·범위 오류, 부분 쓰기·파일 끝 이후 쓰기, 존재하지 않는 파일, 결과가 버려지는 `1!:11` 실패를 포함한다. [tools/test_file_io_audit.py](tools/test_file_io_audit.py)는 fixture 고유성·파일명 quoting·임시 경로 제한·결과/실패 분류를 오프라인 검사하고, [Linux CI](.github/workflows/linux.yml)는 `j64/j64avx2 × default/portable`에서 고정 C library를 빌드한 뒤 비수용 진단 JSON을 artifact로 남기도록 연결했다. `--gate` 없는 실행은 실패 발견을 기록하되 합격이라고 주장하지 않는다. J C oracle 결과를 확인하기 전에는 오류 클래스·플랫폼 종속 EOF hole bytes·mapped boxed 의미를 확정하지 않는다. 소스 pin `0a5101cf`과 CI binary pin `13994ffa`의 `xf.c`·`jmf.ijs`·`gmbx.ijs`·`jfiles.ijs`는 각 파일의 Git blob SHA가 동일함을 교차 확인했다. **이것은 IO-01/02/25의 구현/동등성 수용이 아니며 계속 [ ] 상태다.**

**첫 CI 진단 오류 분석(수용 아님).** [Linux CI 37470524210](https://github.com/yunskim/RustJ/actions/runs/37470524210)의 `j64`와 `j64avx2` 초기 실행은 각각 15개 중 기대값 9개 일치·6개 재검토였다. 여섯 항목 모두 `length error`로, J 의미 차이의 증거가 아니라 **indexed foreign용 파일 인수의 잘못된 boxing**을 원인으로 분리했다. [J Files 원문](https://www.jsoftware.com/help/dictionary/dx001.htm)은 `1!:1 <'name'`과 달리 indexed `1!:11 'name';offset length`, `x 1!:12 'name';offset` 형식을 사용한다. [수정 commit 843ae79](https://github.com/yunskim/RustJ/commit/843ae79d922d57f40d8378f56d70e9cbb251b41a)에서 indexed 파일명 unboxed 전달로 교정하고, `length/rank/syntax` error를 파일 존재 실패로 세던 거짓 양성도 차단했다. [단위테스트 보완 bfcfe7e](https://github.com/yunskim/RustJ/commit/bfcfe7e9e1dffaefa095a837f641c18fdda7c1fe) 이후 재검증 결과가 확정되기 전에는 9/15를 기능 통과로 해석하지 않는다. IO-01/02/25와 전체 0/30 미수용 상태 유지.

**고정 J 실행 검증(2026-10-06).** [Linux CI 37470839455](https://github.com/yunskim/RustJ/actions/runs/37470839455)에서 `check` 작업은 성공했고, 실제 실행·로그가 확인된 `j64/default`, `j64/portable`, `j64avx2/default` 각각 **file-foreign 15/15 기대 동작 일치·재검토 0**이었다. `j64avx2/portable`는 이 기록 시점 실행 진행 중이므로 합계에 포함하지 않는다. source-based 15건을 이후 회귀로 고정하기 위해 [workflow 3d476f6](https://github.com/yunskim/RustJ/commit/3d476f6f4f8bd1c925e2216ee856cfdfcd5fc9d2)에 `--gate`를 추가했다. 이 **C oracle 자체의 fixture gate**는 RustJ의 세 경로 비교나 JMF/Jd/boxed/비동기 I/O 구현을 검증하지 않는다. 따라서 IO-01·02·25 및 전체 IO 0/30 [ ] 유지.

**IO-02 실행 순서 기준 확대(2026-10-07, 원본 J oracle 일부 검증).** [tools/file_io_audit.py](tools/file_io_audit.py)에 기존 독립 파일 사례 15개와 별도로, **6개 ordered-effect case / 총 14개 J step**를 추가했다. 같은 J 엔진과 같은 임시 파일에서 각 JDo 후 파일 내용을 재검사하여 (1) `0 [ write`의 버려진 결과와 살아 있는 쓰기 효과, (2) 오른쪽 read 실패가 왼쪽 write 실행을 막는 J 순서, (3) 이미 완료된 쓰기 뒤의 실패는 이전 파일 변경을 되돌리지 않음, (4) 잘못된 indexed write는 이전 정상 쓰기를 훼손하지 않음, (5) append 후 끝 구간 읽기, (6) truncate 후 전체 읽기를 구분한다. [tests](tools/test_file_io_audit.py)는 오류 전에 임의 쓰기가 실행되는 버그를 부정 사례로 주입하고 검출한다. [CI 37537959953](https://github.com/yunskim/RustJ/actions/runs/37537959953)에서 `check` 성공 및 `j64/default` C oracle **독립 15/15 + 순서 6/6(14 step) 일치**, 불일치 0을 실제 확인했다. 나머지 matrix는 검사 시점 미확정으로 이 기록의 완료 증거에서 제외한다. 이는 **C에서 확인한 의미론 기준**이며 RustJ native/optimized 동등성 및 mapped/foreign 전체 커버리지가 아니므로 IO-02 [ ], 전체 IO 0/30 [ ]을 유지한다.

**IO-02/IO-25 다음 검증 순서(기존 원장의 하위 실행 단위).** (a) `1!:1/2/3/4/11/12`의 파일명 vs 번호 핸들, 권한·close·flush, EOF/short read, 동시 truncate 효과를 실제 file fixtures로 확장; (b) 고정 `jlibrary`를 로드하는 J bootstrap을 분리 검증하고 `jmf` RW/RO/COW 및 refs/unmap/resize·typed/boxed 변형을 별도 smoke oracle로 수행; (c) Jd/jfiles는 테스트 fixture/원본 버전·의존성 확보 후 독립 단계로 실행. 원본 라이브러리 초기화 실패를 J 의미 실패로 오분류하거나 C oracle 15+6 일치를 RustJ 구현 수용으로 승격하지 않는다.

**IO-02 전체 고정 C matrix 재확인(2026-10-07).** [Linux run 37537959953](https://github.com/yunskim/RustJ/actions/runs/37537959953)의 `check` 및 `j64/default`, `j64/portable`, `j64avx2/default`, `j64avx2/portable` 네 reference job은 모두 **success**. 각 reference job은 독립 파일 사례 **15/15**, 순서 시나리오 **6/6(14 JDo step)**, `requires_review=0`, `ordered_requires_review=0`으로 동일하게 통과했다. 이는 조합별 C oracle probe 결과이며 RustJ의 같은 세 연산 경로를 비교하는 시험이 아니고 IO 수용점수는 변하지 않는다.

**IO-25 JMF 첫 독립 smoke 실행 시도(수용 전).** [tools/jmf_smoke.py](tools/jmf_smoke.py)에 J 원본 `jlibrary/bin/profile.ijs`와 `load 'jmf'` 부트스트랩을 고정 라이브러리·별도 임시 HOME으로 실행하고, 임시 JMF file에 대해 RW(0)→RO(1)→COW(2)를 **각각 map → 빈 noun 비교 → unmap 결과 0**으로 관찰하는 독립 실행 후보를 구현했다. [검증용 JMF 시나리오 테스트](tools/test_jmf_smoke.py)를 추가하고 [Linux CI](.github/workflows/linux.yml)에 비수용 탐색 단계를 연결했다. 원본 J profile/add-on 초기화 또는 JMF mapping이 실패하면 `blocked`를 별도로 기록하며, 이 시점에는 JMF CI 실행 결과/기능 지원을 주장하지 않는다. 이 작은 smoke는 **실제 boxed payload, mutable persistence, RO/COW 쓰기 차이, refs에 의한 unmap 거부, resize/remap, Jd partition** 검증이 아니므로 IO-25 [ ] 유지.

**JMF 부트스트랩 첫 실측 결과(2026-10-07, 미수용).** [Linux 37538470000](https://github.com/yunskim/RustJ/actions/runs/37538470000)의 `j64/default`와 `j64/portable` C job에서 JMF 탐색 단계는 `status=blocked`, `stage_count=2`였으며, 첫 `BINPATH_z_` 설정은 실행되고 `0!:0 <.../jlibrary/bin/profile.ijs`에 `domain error`가 발생했다. **map/unmap까지 도달하지 않았다.** 이 결과를 JMF RW/RO/COW의 부정 결과로 해석하지 않는다. [후속 진단 541be39](https://github.com/yunskim/RustJ/commit/541be39f42f1df655e7af5bde86d928321f4c9b3)는 실제 J script loader가 짧은 독립 fixture를 읽을 수 있는지 먼저 검사하고, 실패 원본에 `13!:12` 문맥을 포함하도록 분리했다. 후속 CI 결과는 이 기록 시점 미확정이다. IO-25는 [ ]이다.

<a id="io-static-source-contracts"></a>
#### 8.5.5 정적 입력·DataSource 계약과 역방향 데이터 요구량 분석 (2026-10-11, 설계 후보 / 미구현)

**결정 범위.** 과거 JAXA의 `with` 구상을 RustJ의 **ordinary-name conjunction으로서 typed semantic annotation/contract만 부착한다**는 기존 규약 안에서 재사용한다. `with`는 입력에 대한 *정적 의미론적 계약*을 보완하고, 외부 DataSource의 *접근 capability·실제 저장 특성·일관성 증거*는 별도의 binding 계약으로 취급한다. `with`에 파일 경로, chunk 크기, prefetch 깊이, tile/register/device 정책을 넣지 않는다. 데이터 원소를 컴파일 시점에 모두 읽을 필요는 없으며, 같은 generic J graph를 서로 다른 입력과 storage adapter에 반복 적용할 수 있어야 한다. 이는 **설계 후보**이며 현재 `with` 및 Out-of-core API가 구현됐다는 뜻이 아니다. 기존 [§8.5.3](#unified-data-movement-contract) 계층 및 [IO-01~IO-30](#out-of-core-io-checklist) 외에 새로운 필수 IR 또는 별도 수용 원장은 만들지 않는다.

**세 계약의 분리.**

| 계약 주체 | 사전에 알거나 보장하는 정보 | 하지 않는 일 |
|---|---|---|
| `with` / 함수·입력 semantic contract | 경계 입력의 J-visible dtype, shape(Exact/Symbolic/Dynamic), 인수 사이 차원 제약, 증거가 있는 배열 속성(정렬·유일성·범위 등), 선택적인 static value, 의미론적 adjoint 연결 후보 | 실행할 파일 경로·buffer address·저장 chunk·하드웨어 schedule을 결정하지 않음; 중간 결과 shape/Rank/Cell을 사용자에게 전부 선언시키지 않음 |
| DataSource의 *static requirement/capability contract* | logical domain/schema와의 호환성, sequential/seek/region read, repeatability, write/append/atomic publish 지원 여부, version/snapshot을 요구할 수 있는지, codec/encoding 지원 | capability가 빠르다는 보장 아님; read-only 핸들만으로 외부 불변성을 보장하지 않음 |
| 실제 *BoundSourceDescriptor*와 runtime witness | 연결 시 확인한 source identity, version/snapshot lease, storage layout/strides, read/write/codec chunk geometry, byte offsets/index, 압축·endianness, alignment, 파일 크기/권한, 가용 자원 | 확인되지 않은 snapshot·원소 성질·성공한 I/O·지연시간을 정적 사실로 승격하지 않음 |

**계약 강도 네 가지.** (1) **Hard/semantic requirement**(dtype, 확정 shape, J-visible type, 일관된 index domain, 허용 가능한 snapshot): 불일치 시 해당 특수화·실행 경로를 거부하거나 guard를 통한 안전한 재특수화/일반 경로로 이동한다. (2) **Capability**(region read, replay, 독립 부분 쓰기, seek): 후보가 *가능한가*만 제한하며 성능을 약속하지 않는다. (3) **Soft performance preference**(선호 read bytes, chunk aspect ratio, bandwidth hint): 비용 모델에서만 사용, legality 증명에는 사용할 수 없다. (4) **Runtime observation**(EOF, cache, latency, free memory, 실패/완료): 실제 binding/실행에서 확인하고, 잘못된 추정으로 J 의미를 바꾸지 않는다. 'Unknown'은 불변·저비용·무효과를 의미하지 않는다.

**Array Shape, Rank, Frame/Cell.** `shape=[N,784]` 같은 기호 차원은 generic graph의 반복 사용을 돕는다. `"` Rank conjunction의 *Cell 적용 의미*는 Shape declaration과 별개이며, Rank의 요청/실현·빈 Frame의 가상 Cell, fill/type/shape 조립 규칙을 계속 보존한다. Logical Array dtype·원소 순서·J-visible boxed/sparse 의미는 storage encoding과 다르다. 파생된 중간 배열 facts는 IR 분석으로 유도하고, `with`는 증명되지 않은 값을 보충하는 경계 정보만 제공한다. Static Value는 값 고정·version/provenance 보장 시에만 상수 최적화와 specialization key의 입력이 된다. 키에는 사용된 사실만 담고 widening/guard로 variant 폭증을 막는다.

**서로 다른 분할 단위.** `Logical ArrayRegion`(사용자에게 보이는 의미적 위치), `Storage ReadChunk`, `Storage WriteChunk/Shard`, `CodecChunk`, `IoRequestRange`, `ComputeTile`, `DevicePartition`은 하나의 `ChunkSize`가 아니다. 저장 포맷의 read/write/codec 경계와 compute tile은 달라도 된다. `ArrayRegion`에서 `FileByteRange`로 변환하는 책임은 layout/encoding을 검증한 physical adapter에 있으며, 일부 원소 요청이 전체 압축 block read/decode를 피할 수 있다는 보장은 없다.

**역방향 data-demand 분석(새 IR이 아닌 pass/sidecar 후보).** 최종 결과 또는 필요한 sink region을 seed로 놓고 그래프의 소비자에서 생산자 방향으로 `Demand`를 전파한다. Demand는 `None / KnownRegions / Full / Unknown`의 값 요구량과 dtype·shape/prototype 등 **별도 metadata/semantic-check 의무**를 표현하며, 여러 소비자는 영역의 합집합을 취한다. 원소별 연산은 대응 region, slice/transpose/reshape는 증명된 inverse index mapping, reduce는 요구 출력에 영향을 주는 reduction 축 전체, convolution은 kernel/stride/padding이 요구하는 halo, dynamic gather는 확인된 인덱스 또는 보수적 full/unknown으로 되돌린다. `GraphOperationContract::access`는 접근 *범주*일 뿐 정확한 region transformer가 아니므로, primitive별 transfer rule·witness가 없으면 reduction/Rank/boxed/sparse를 포함해 확대 분석하지 않는다. `Full` data demand는 '전체 bytes 필요'이지 '전체 배열 동시 메모리 상주'가 아니다. Streaming·Chunk 병렬화는 별도의 associativity, numeric promotion, errors/effect/order, state/halo 합법성 증명이 있어야 한다.

~~~text
J verb + with(input semantic requirements)
    -> J Graph IR + symbolic ArrayRegion demand sidecar
    -> Verified Logical Execution IR (effects/errors/checks/guards)
    + DataSource requirement/capability
    -> bind source, validate schema/version/layout/witness
    -> Physical candidates: exact/coalesced read, sequential scan,
       storage rechunk, compute tiling, bounded streaming, full read
    -> legal candidates only -> resource budget + measured/estimated cost
    -> executable plan (readiness, buffer lease, failure/cancel order)
~~~

**Binding/최적화 절차.** generic 컴파일은 Graph IR/정적 입력 요구와 가능한 access region을 보존한다. 실제 DataSource 결합 시 Hard 제약을 확인하고 source capability, physical chunk/codec/layout, version·lease를 수집한다. 실패하면 기존 의미론적으로 안전한 경로를 사용하거나 계약 불일치를 보고한다. 그다음 verified logical access demand를 byte ranges로 내리고, 작은 random reads 합치기·sequential scan·mmap·prefetch·rechunk·materialize 등의 **후보**를 비용 비교한다. 재사용되는 compiled plan은 관련 정적 조건과 source version이 유효할 때만 cache한다. 외부 변경이나 snapshot 검증 실패는 cache invalidation/guard failure이며, 이미 발생한 관찰 가능한 I/O를 소급하여 지우지 않는다.

**읽기·쓰기·효과 분리.** (A) 내부 immutable/versioned backing read, (B) J `1!:` foreign의 관찰 가능한 read/write/open/error, (C) mutable weight/checkpoint sink는 서로 다른 semantics다. ReadOnly ≠ externally immutable; repeatable ≠ snapshot; region-readable ≠ cheap random access; async submission ≠ successful completion/durability; mapped ≠ zero-copy. B의 사용되지 않는 read가 파일 오류를 낼 수 있으므로 단순 DCE·pushdown으로 제거할 수 없다. A의 speculative prefetch조차 사용자에게 보이는 첫 오류/순서를 바꾸지 않는다는 증명이 필요하다. C는 write range 겹침, ordering, commit/flush/durability, cancellation, partial failure를 별도 계약과 시험으로 검증한다. 빈 데이터 read를 생략하더라도 J 빈 Frame의 virtual Cell prototype/check 의무는 독립적으로 남는다.

**선행 시스템 대조 — 참고 모델이지 RustJ 기능 주장 아님.**

| 프레임워크 · 공식 출처 | 실제 추상화 / 교훈 | RustJ 도입 경계 |
|---|---|---|
| [TensorStore Schema](https://google.github.io/tensorstore/schema.html) · [Spec](https://google.github.io/tensorstore/spec.html) | dtype/rank/domain의 Hard constraint, storage layout의 Hard/Soft 제약, Read/Write/Codec Chunk 구분, 열 때 Schema 검증 | static/source contract의 강도와 물리 저장 배치 분리; TensorStore는 full-J 계산 의미론 제공자가 아님 |
| [DaCe SDFG](https://spcldace.readthedocs.io/en/latest/sdfg/ir.html) | Data Container, Tasklet, `Memlet`의 symbolic subset/volume로 계산과 데이터 이동 기술 | `ArrayRegion` 전파·witness 참고; Memlet을 파일 snapshot 보장으로 오해하지 않음 |
| [JAX JIT](https://docs.jax.dev/en/latest/jit-compilation.html) · [Shape Polymorphism](https://docs.jax.dev/en/latest/export/shape_poly.html) | Shape/DType abstract value, static argument specialization, symbolic dimension 제약 | `with`가 사전 계약을 제공하는 구조에 참고; JAX JIT가 일반 파일 I/O 계약까지 해결하지는 않음 |
| [Arrow Dataset Scanner](https://arrow.apache.org/docs/python/generated/pyarrow.dataset.Scanner.html) · [DuckDB Parquet](https://duckdb.org/docs/stable/data/parquet/overview.html) | column/filter pushdown, fragment/batch read-ahead 및 데이터 형식별 pruning | 순수하고 증명된 access에 한정; 임의 J verb나 foreign 오류를 없애지 않음 |
| [Dask Chunks](https://docs.dask.org/en/stable/array-chunks.html) | 논리 array shape와 계산 Chunk, rechunk 이동 비용이 독립 | Chunking은 physical policy; J-visible shape/rank 변경 금지 |

**기존 IO 체크리스트에만 반영하는 검증/이행 범위.** IO-03에 semantic `with` vs source capability vs bound physical descriptor의 stage ownership, provenance/guards, invalid-contract negative verifier를 포함한다. IO-04에는 sequential-only/single-pass vs region-readable/replayable, mutable vs versioned snapshot, Hard/Soft 차이, source binding mismatch matrix를 포함한다. IO-09/10에는 `None/Regions/Full/Unknown` demand transfer와 다중 소비자 union, slice/transpose/reduction/halo/indirect fallback 및 no-skip-on-error 증명을 포함한다. IO-11/12에는 immutable/versioned cache와 invalidation/witness를, IO-26/27에는 logical schema·read/write/codec chunk 분리 및 encoding/EOF/overflow 검증을, IO-16/17에는 full-demand streaming vs barrier, error visibility, buffer lease·completion을, IO-20/21/22에는 cost, exact/coalesced/scan 후보 측정 및 독립 3경로·negative 회귀를 포함한다. **IO-01~IO-30의 ID/상태는 변경하지 않으며 현재 구현 수용 0/30이다.** 이 계약 도입은 M2 또는 최초 M4 CPU 경로의 선행 조건이 아니다.

<a id="read-scope"></a>
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

<a id="architecture-migration-checklist"></a>

<a id="read-roadmap"></a>
## 10. 구현 계획과 체크리스트

이 절이 앞으로 유일한 구현 체크리스트다. **저장장치·느린 I/O·Out-of-core 작업은 [§10 IO-01~IO-30](#out-of-core-io-checklist)을 이 절 안의 단일 수용 목록으로 사용한다.** 순차 CPU 기준 실행과 J 의미 수렴은 계속 M2→M3→M4 우선이며, IO-A 조사만 병행할 수 있다.


<a id="jaxa-future-work-traceability"></a>

### JFW — JAXA Future Work 10개 항목의 RustJ 이행 추적표 (2026-10-13; 문서·설계만)

**근거와 권한:** [JAXA 원본 명세의 Future Work 1–10](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/JAXA-README.md)을 항목별로 역대조했다. 이 표는 **기존 §10 작업의 추적 색인**이며, 독립 로드맵·새 필수 IR·새 language semantics가 아니다. §10이 유일한 수용 체크리스트다. 현재 코드/문서에 *설계*가 있다는 사실은 *구현·증명·E2E 완료*가 아니다. 아래의 미완료 체크를 기존 M2→M3→M4보다 앞선 blocking gate로 승격하지 않는다.

| 원본 항목 / RustJ 식별자 | 적용 판단과 현재 경계 (2026-10-13) | 연결된 계층·근거 | 우선순위 |
|---|---|---|---|
| **FW1 Completeness Verification / JFW-01** | 범용 신경망 표현의 형식적 완전성 증명 대신, 실제 J 구문·Rank·shape·effect를 유지하는 *명시적 지원 범위/대표 모델 수용 매트릭스*부터 검증. 전체 완전성은 연구 항목 | Frontend, J Semantic IR, NN extension E2E·coverage manifest | 중기, 형식 증명 선택 |
| **FW2 Automatic Gradient Derivation / JFW-02** | 원래의 자동미분 생성 목표를 **명시적 adjoint/VJP primitive 및 사용자 작성 backward graph**로 변경. Autograd는 필수 기능이 아님. `AdjointVjp` topology는 예약, 실제 전개·실행은 미완료 | `src/opportunity.rs`, `src/nn_extensions.rs`, explicit StateResource | 중기, 방향 변경 |
| **FW3 Deterministic Memory Scheduling / JFW-03** | 버퍼/뷰 식별과 첫 CPU PhysicalPlan 계약은 있음; 범용 lifetime·offset 배치·재사용·activation retention·transfer plan은 미완료. 정적 크기가 증명되는 region에만 확정 계획 적용 | `src/physical_plan.rs`, §5, §8, M4 | 높음, CPU 경로 후 |
| **FW4 Equivalent Form Search / JFW-04** | `j_graph_rewrite` / fusion 후보와 `egg` 설계는 있으나 실행 가능한 범용 EqGraph 탐색·extraction 없음. 규칙·가드·증거를 등록하고 안전한 후보만 탐색 | J Graph Basis/Rewrite, [공통배열대수 가설](#common-array-algebra-hypothesis), `src/j_graph_fusion.rs` | 최상, 후속 연구 |
| **FW5 Basis Verb Identification / JFW-05** | 전역 최소 primitive 집합을 먼저 증명하지 않음. **Graph Basis(대수적 재작성 단위) ≠ Execution/Lowering Basis(물리 구현 단위)** 구분; 확장 연산 분해는 검증 이후 허용 | J Graph IR, Logical IR, Extended Primitive, §4–§5 | 높음, FW4/8와 연계 |
| **FW6 Distributed Execution / JFW-06** | device·memory-space·sharding·transfer·ready/synchronization의 계획 계약을 보존하되 실제 multi-GPU/multi-node 런타임은 장기 후속 | §7.5, HE, Physical Plan, `AGENTS.md` | 장기 |
| **FW7 Hardware Portability / JFW-07** | target locale/capability/lowering 계약 설계 존재. CPU-native E2E를 먼저 닫고 GPU·MLIR/LLVM·StableHLO 부분집합 등은 독립 적법성/실행 검증 후 확장. 현재 CUDA 실행 미구현 | TargetContext, LoweringRegistry, M4→M6, `AGENTS.md` | 높음, 단계적 |
| **FW8 Model Equivalence Verification / JFW-08** | 정확한 J 관찰 동등성과 오차가 있는 수치 후보를 분리. `numeric_error_policy`는 분석 전용; 동등성 증명기와 EqGraph의 안전한 e-class 병합은 미완료 | J-C 차등 오라클, FactWitness, `src/numeric_error_policy.rs`, `AGENTS.md` | 최상, FW4의 선행 검증 |
| **FW9 Structural Sparsity / JFW-09** | J-visible sparse 배열 의미 일부 구현, dense/sparse 표현 분리. sparse assembly·atomic kernel 경로에 미지원 사례 존재. MoE·sparse attention의 구조가 입력에 따라 동적일 수 있어 정적이라고 간주 금지 | `src/sparse.rs`, `src/assembly.rs`, `src/kernels.rs`, NN extension | 중기 |
| **FW10 Quantization Integration / JFW-10** | dtype/precision·cast와 accumulation policy의 독립 계약부터 검증. 원래 JAXA의 precision 표기를 표준 J 구문으로 무단 도입하지 않음. 양자화·mixed precision·자동 cast는 아직 완료 아님 | Frontend DType/Rank, NN family spec, target-specific lowering | 중기 |

**독립 검증·수용 체크리스트 (현재 상태; 각 항목의 완료는 기능별 구현 증거가 생긴 후에만 표시):**

- [x] **JFW-00 추적표 작성:** 원본 FW1–FW10 출처, RustJ 책임 단계, 현행 구현 경계, 우선순위, 한·영 mirror 매핑을 문서에 등록했다. **문서 작업만 완료**.
- [ ] **JFW-01 / V01:** ResNet·Transformer·비-NN 배열 사례를 포함한 구조 표현 매트릭스와 오류·Rank·shape·binding의 J-C 차등, 지원·미지원 부정 사례를 기록한다. 범용성 증명은 별도 상태다.
- [ ] **JFW-02 / V02:** 명시적 adjoint/VJP 등록→Frontend 인식→Graph/Logical fan-out→값·weight gradient/state effect 검증→CPU 실행을 단계별로 닫는다. 원시 forward 식에서 자동 미분을 생성하는 것과 혼동하지 않는다.
- [ ] **JFW-03 / V03:** M4 CPU multi-op에서 value-last-use, buffer lease/alias/ownership, 안전한 reuse, materialization/no-materialization을 증명·실측한다. 미지 크기/효과/오류는 안전한 경계를 유지한다.
- [ ] **JFW-04 / V04:** guard가 있는 정확한 rewrite 1개부터 end-to-end 후보 생성→합법성→extraction→기본 실행 대비 동등성/비용 비교를 실증한다. 탐색 성공 자체는 의미 증명이 아니다.
- [ ] **JFW-05 / V05:** Graph Basis별 J 관찰 의미·rank/shape·빈 결과·오류·numeric 가드를 등록한다. Conv 등 extension은 *참조 정의*와 *실제 대체 실행의 동등성 증거*를 구별한다. 최소 기저의 완전성 증명은 필요조건이 아니다.
- [ ] **JFW-06 / V06:** 단일 CPU·zero-transfer 경로 수용 뒤 장치/메모리 간 version·transfer·ready·sync·effect/error precedence를 검증하며, 다중 노드 실행은 별도로 검증한다.
- [ ] **JFW-07 / V07:** 동일 J 연산을 native CPU, 다른 target route에서 지원 전제와 결과·오류·효과까지 비교한다. 장치별 cost/profile과 언어 의미를 분리한다.
- [ ] **JFW-08 / V08:** provenance, Rank/Cell/Frame, prototype/fill, NAME, effect, first observable error, CCT/Fit, float/NaN/zero를 포함한 **Exact-J witness** 없이 e-class를 union하지 않는다. J의 `2^-44` 비교 허용 오차는 최적화 오차 예산이 아니며 근사 동등은 비추이적이다.
- [ ] **JFW-09 / V09:** 기존 sparse J semantics를 보존한 구조적 희소 연산/NN routing의 긍정·부정 테스트를 만들고, sparse pattern이 runtime dependent이면 guarded/dynamic route로 남긴다.
- [ ] **JFW-10 / V10:** 저장·계산·누적 dtype, 명시적/자동 cast의 안전 조건, quantize/dequantize 및 overflow/rounding/empty/error를 오라클과 비교한다. FP32 단독 통과를 mixed precision 완료로 세지 않는다.
- [ ] **JFW-X1 / VX1 — Rank-aware Fusion:** `CellApply`/Map/Reduce/Scan/reindex의 고수준 topology를 보존하고, fusion 전에 frame/cell agreement, empty/prototype, error/effect 순서를 증명한다.
- [ ] **JFW-X2 / VX2 — Resource-aware Search:** 연산수·bytes moved·temporary peak·layout/tiling·transfer·resource feasibility를 분리 추정한다. 안전하지 않은 early pruning은 금지하고, 선형 chain 비용 평가와 DAG branch/fanout 비용을 구분한다.
- [ ] **JFW-X3 / VX3 — Explicit State & Adjoint Planning:** forward/backward/optimizer의 `StateResource`, `ValueId`, `BufferId`를 구분하고 gradient emission·activation lifetime·effect/error barriers를 검증한다.

**공통 수용 규약:** 각 V01–V10, VX1–VX3에 `Test / Purpose / Guarantee / Failure meaning / Status`, 재현 입력(특히 반례), 지원 scope, 비교 J-C 기준/커밋, 실제 CI/PR head·merged-main 증거를 기록한다. `PASS`, `FAIL`, `UNRUN`, `BLOCKED`를 구분하며 문서만 추가한 상태는 `UNRUN`이다. 표·체크리스트는 구현 전부 **미완료**이며, 향후 증거가 있을 때 해당 항목만 갱신한다.

**진행 순서와 비목표:** M2 Frontend/의미 → M3 적법성/Logical single-block → M4 native CPU E2E를 먼저 닫는다. 이후 JFW-08/04/05의 *검증된 작은 부분집합* → JFW-03/X1/X2의 자원·fusion → JFW-02/X3·07·09·10의 실행 확장 → JFW-06/형식 완전성 연구 순서로 단계적으로 진행한다. 여러 항목의 연구·문서화는 병행할 수 있지만 CPU 기준 경로의 블로커가 되어서는 안 된다. **자동미분 엔진, 전역 최소 Basis 증명, 전체 J/모든 모델의 완전성 증명, 분산 실행, GPU 실행은 첫 CPU 수직 경로의 필수 선행조건이 아니다.**

<a id="dynamic-boundary-checklist"></a>

### DB — 동적 의미와 컴파일 경계 이행 (DB0–DB7)

이 목록은 §10 M2→M3→M4와 WI3/WI4/M5–M8의 연결 지점이며 별도 병렬 backend 구현 계획이 아니다. **M2 frontend 수렴을 먼저 진행한다.** 해당 semantic 변경을 구현할 때 필요한 회귀를 추가하고, 최적화 자체는 별도 승인된 단계에서 진행한다.

- [x] **DB0 계약 문서화:** 경계·검사 시점·변환 금지 조건·실패 처리와 초기 구현/미완료 상태를 정본과 mirror에 통합한다.
- [ ] **DB1 구조화된 경계 보고:** source span, semantic phase, reason, 필요한 fact/witness, 허용 분석·거부 변환, 필요한 route capability를 기록한다. unknown facts·J-invalid·implementation coverage·route rejection을 구별하며 valid unsupported 사례의 원인을 보존한다.
- [ ] **DB2 witness 유효성:** parser POS/constructor snapshot/call-time lookup을 구분한다. binding/frame/locale/path와 unbound-search 결과의 의존성을 추적하고 검사→사용 사이 mutation을 확인한다. WI4·M5/M6의 noun metadata 연결과 함께 구현한다.
- [ ] **DB3 효과·오류 경계:** lookup/write/context/I/O/resource/error를 ordered region 또는 동등한 명시적 의존성으로 연결한다. fork/selector·같은 이름의 두 호출·assignment expression에서 값과 효과의 live-out을 분리한다.
- [ ] **DB4 첫 실행 경로 선택:** Native CPU slice에서 effect 이전에 필요한 guard를 검사한다. miss 시 J 오류를 만들거나 replay하지 않고 실제 지원 경로/재분석/coverage 경계로 분기한다. 외부 route와 CUDA는 capability 증명 후 별도 확장한다.
- [ ] **DB5 중간 전환의 선행 조건:** effect 이후 전환이 필요해질 때 continuation 상태·소유권·정확히 한 번 효과·오류 위치 계약을 먼저 명세·검증한다. 완료 전 중간 fallback을 활성화하지 않는다. full continuation을 첫 CPU slice의 무조건적 선행 조건으로 만들지 않는다.
- [ ] **DB6 Windows 차분 gate:** NAME 재정의/POS 변경, local 미정의→정의, locale/path 변경, noun snapshot 뒤 재대입, 값 의존 constructor, 효과 뒤 오류/guard miss를 C 기본·AVX2와 비교한다. 값/type/shape뿐 아니라 lookup 시점·효과 순서·실패 후 binding과 실행 횟수를 검사한다. 현재 미지원 locale/execute는 별도 coverage로 보고한다.
- [ ] **DB7 중후반 의미·성능 gate:** 검증된 direct runtime과 Logical/Physical 실행을 같은 입력으로 대조한다. guard hit/miss와 empty/boxed/sparse 경계를 포함하며 성능·복사/할당은 의미 통과 뒤 별도 측정한다. frontend 통과나 메타데이터 분석을 backend 실행/성능 통과로 승격하지 않는다.

<a id="nv3a-spelling-errors"></a>

**NV3a fixed spelling 오류 분류 — 2026-10-05.** `jsrc/ws.c::spellin`과 `jsrc/w.c::jtenqueue`의 순서를 따른다. 설치·검증된 core dictionary가 우선이며, 등록되지 않은 colon inflection 또는 nonnumeric dot inflection은 spelling error다. numeric dot는 numeric constructor로 넘기고, quote와 simple name은 각각의 분류를 유지한다. 한 자리 constant function은 기존 core descriptor를 통과한다. `99:`/`1.5:`/`_99:`는 reviewed C에서 유효한 constant function이 아니므로 spelling error다. 잔여 잘못된 문자·미설치 primitive도 Unsupported가 아닌 spelling error다. obsolete spelling의 임의 예외 목록은 만들지 않는다.

`name_:`는 문법적으로 유효한 by-value/abandon lookup이다. simple-name validation을 거친 뒤 별도 Unsupported 경계로 남긴다. `foo__:`처럼 suffix 제거 후 명백히 ill-formed인 simple name은 ill-formed name으로 보고한다. bounded locative grammar는 후속 NV3b에서 검증한다. NAME 길이 제한과 오류 순서는 후속 NV3c에서 검증한다. locale lookup·abandon 효과와 complex/extended/rational numeric grammar는 아직 완료되지 않았다. 따라서 fixed spelling seam인 NV3a만 완료했고 NV3/NV5 전체 완료를 주장하지 않는다. invalid lexical spelling과 valid primitive의 constructor/executor 미지원은 계속 구별한다. 기존 enqueue diagnostic의 phase·source span·word index를 유지한다.

`tools/spelling_conformance.py`는 graphic ASCII 93개(quote 제외) × 7개 suffix의 **651** matrix와 추가 이름/숫자 경계를 양 DLL과 비교한다. 오류 종류·word formation·유효 미지원 경계를 각각 검증하며 primitive 실행 지원이나 전체 name/numeric grammar conformance로 세지 않는다. `tools/vocabulary_audit.py`도 기존 code-only 후보 2개와 legacy 8개의 Rust 오류를 C와 반드시 비교한다. 표준 Windows runner에 spelling report를 추가했다.

**NV3a 검증:** native Windows default/portable 각각 **465 passed / 17 ignored**, fmt/clippy/build 통과; Python **30 passed**. 양 DLL 각각 spelling **667 cases / failed 0**: 오류 종류 **453**(spelling **426**, number **22**, name **1**, syntax **4**), accepted enqueue controls **208**, valid Unsupported 경계 **6**. 이 중 matrix는 **651**이며, quote grammar와 전체 locative/numeric grammar는 범위 밖이다. 기존 세 runtime 경로 각각 **5,380 cases / 5,380 passed / failed 0**, stages **10,810**, words **6,623**, vocabulary POS **143**/binding **140**/noun **3**를 유지했다. Scan **285 / failed 0**와 runtime prefix 경계 **285 / executable prefix passes 0**, capture graph 경계 **257**, static 경계 **2**를 별도로 유지했다. 전체 보고서 16개의 binary/source/DLL hash를 확인했다. spelling pass는 실행 지원이나 GPU 성능 검증이 아니다. Linux/GitHub CI/CUDA는 실행하지 않았다.

<a id="nv3b-name-syntax"></a>

**NV3b bounded name syntax — 2026-10-05.** `sn.c::vnm/vlocnm`을 참조해 shared enqueue name validator를 보강한다. ASCII letter로 시작하고 alphanumeric/underscore로 구성된 이름에서 simple name, trailing direct locative와 empty/base locale `__`, indirect chains, 최종 numeric debug-frame component 및 그 음수 표기를 구분한다. numeric direct locale의 leading zero·x64의 18자리 제한, 잘못된 중간 숫자/isolated underscore/과도한 underscore를 검사한다. `name_:`는 suffix를 제거한 같은 이름 문법으로 검증한다. validator는 locale·symbol을 조회하거나 noun/function을 생성하지 않고 추가 heap allocation 없이 동작한다.

문법적으로 valid인 locative/by-value name은 계속 Unsupported이며 locale lookup·debug-frame 접근·abandon 효과를 구현했다고 주장하지 않는다. malformed name은 enqueue phase/span/word index를 보존한 ill-formed name이다. NV3b 당시 남겼던 NAME 전체·simple-name·locale storage 길이 제한과 오류 우선순위는 후속 NV3c에서 검증한다. full numeric grammar와 locale 실행은 여전히 NV3/DB2의 잔여 작업이다. 따라서 bounded syntax인 NV3b만 완료하며 전체 NV3/DB2/locales 완료가 아니다. 정의의 `for_name.` 분류도 shared enqueue validator를 통과하므로 이 동일한 bounded syntax 검증을 사용한다.

`tools/name_syntax_conformance.py`는 짧은 `a0_` 조합, direct/indirect/debug-frame 사례, 고정 seed의 mixed-case/digit/underscore 이름 및 각 `name_:` 형태를 생성해 양 DLL과 대조한다. C의 enqueue 오류를 실행 이후 value/locale 오류와 구별한다. 후자의 발생은 이름의 lexical validity만 확인하며 lookup/runtime 성공으로 세지 않는다. `sn.c`·`w.c`·`ws.c`·`jerr.h` source hash와 probe/DLL hash를 보고서에 보존하고 표준 Windows runner에 연결한다.

**NV3b 검증:** native Windows default/portable 각각 **466 passed / 17 ignored**, fmt/clippy/build 통과; Python **30 passed**. 양 DLL 각각 name syntax **4,030 cases / failed 0**: simple name **1,133**, valid Unsupported name **1,901**, invalid name **996**. spelling **667 / failed 0**와 vocabulary POS **143**/binding **140**/noun **3**, 세 runtime 경로 각각 **5,380 cases / 5,380 passed / failed 0**, stages **10,810**, words **6,623**를 유지했다. Scan **285 / failed 0**와 runtime prefix 경계 **285 / executable prefix passes 0**, capture graph 경계 **257**, static 경계 **2**를 별도로 유지하고 전체 보고서 18개의 binary/source/DLL hash를 확인했다. lexical validity 확인은 locale runtime 지원이나 실행 conformance pass가 아니다. Linux/GitHub CI/CUDA는 실행하지 않았다.

<a id="nv3c-name-limits"></a>

**NV3c NAME 길이와 오류 순서 — 2026-10-05.** `sn.c::nfs`의 J-visible compatibility 검사를 추가한다. underlying name의 전체 byte 길이는 **1 ≤ n < 32767**이며 이 범위를 벗어나면 ill-formed name이다. 저장될 simple-name 부분과 locale 부분은 각각 **255 bytes 이하**다. direct locative는 마지막 locale separator를 기준으로 나누며 empty/base locale도 구별한다. indirect locative는 첫 `__` 뒤 **전체 chain suffix**를 locale 크기로 검사한다. chain의 각 component만 255 이하인 것으로는 충분하지 않다. 이 값들은 J 호환성 조건이며 Rust storage/allocator/physical layout의 제한이 아니다.

검사 순서를 보존한다: 전체 길이 → indirect 마지막 numeric/debug-frame text의 digit validation → simple/locale 크기 → `vnm` 문법. 마지막 numeric component에 문자가 섞이면 다른 부분이 과도하게 길어도 ill-formed name이 먼저다. 그 외의 malformed locative는 component 크기를 먼저 검사하므로 limit error가 문법 오류보다 앞설 수 있다. `name_:`는 suffix를 제외한 underlying name에 동일한 검사를 적용한다. primitive inflection의 spelling 검사는 그보다 앞에 유지한다. Rust는 이 검사를 allocation 없이 수행하며 C NAME block·hash·symbol table을 도입하지 않는다. enqueue diagnostic phase/span/word index를 유지한다.

Windows name differential에 254/255/256/257, 32766/32767 경계, direct/indirect chain, 크기와 malformed text가 동시에 있는 경우 및 by-value 형태를 추가했다. 길이 오류와 spelling 우선순위도 C와 정확히 비교한다. valid locale/by-value/debug lookup은 계속 Unsupported이며 locale 실행/abandon 효과를 구현하지 않는다. numeric notation의 전체 유효성은 NV3d에 남긴다. 이 단계는 NAME 길이·오류의 수렴이며 전체 NV3/NV5 완료가 아니다.

**NV3c 검증:** native Windows default/portable 각각 **467 passed / 17 ignored**, fmt/clippy/build 통과; Python **30 passed**. 양 DLL 각각 name syntax **4,125 cases / failed 0**: simple name **1,135**, valid Unsupported name **1,927**, invalid name **1,022**, length limit **40**, spelling precedence **1**. 기존 4,030건에 길이/우선순위 fixture **95건**을 추가했다. spelling **667 / failed 0**, vocabulary POS **143**/binding **140**/noun **3**, 세 runtime 경로 각각 **5,380 cases / 5,380 passed / failed 0**, stages **10,810**, words **6,623**, Scan **285 / failed 0**를 유지했다. runtime prefix 경계 **285 / executable prefix passes 0**, capture graph 경계 **257**, static 경계 **2**는 별도다. 전체 보고서 18개의 binary/source/DLL hash를 확인했다. lexical/길이 비교는 locale runtime 성공을 뜻하지 않는다. Linux/GitHub CI/CUDA는 실행하지 않았다.

<a id="nv3d1-numeric-recognition"></a>

**NV3d1 숫자 word 문맥과 오류 검증 — 2026-10-05.** `src/numeric_input.rs`는 enqueue에서 `wn.c::numcase/connum`의 whole-word dispatch를 검증한다. numeric list의 모든 field가 공유하는 complex/based·extended integer·rational·precision 선택을 보존한다. 한 field만 따로 해석하면 `1x`와 `1.0 1x`, `1j2 1x`, `2b10 1x`의 차이를 놓친다. suffix/operand·rational infinity·rectangular/polar complex·based digit·p/x exponent 표기를 검증한 뒤 실제 malformed word는 ill-formed number로 보고한다. 숫자 family 문자 하나의 존재만으로 Unsupported를 선택하던 heuristic은 제거했다. `1xr2`는 C의 `numfd`가 `r2`의 생략된 numerator를 0으로 읽으므로 valid임을 회귀에 포함한다.

검증 상태 Valid/Invalid/Unknown을 구분한다. 검증된 extended/rational/complex/based 표기의 payload 생성은 아직 Unsupported이다. precision과 플랫폼-specific `strtod` hex/NaN payload 등 완전히 검증하지 않은 문법도 별도 Unsupported reason을 유지하며 Invalid로 추측하지 않는다. half/single 및 일부 quad 조합은 supplied C 자체의 nonce boundary이므로 C 성공 또는 J spelling/number 오류로 세지 않는다. 일반 integer/decimal은 기존 constructor로 바로 넘기므로 추가 float parse/normalized string allocation을 하지 않는다. 잘못된 숫자의 enqueue phase·word index·span을 유지하며 runtime target/array IR/physical allocation 정보를 숫자 문법에 도입하지 않는다.

`tools/numeric_syntax_conformance.py`가 scalar 표기와 교차 numeric lists, 잘못된 suffix/missing operand, infinity, 64-bit overflow, colon spelling 우선순위, precision/platform boundaries를 양 DLL과 비교한다. accepted noun controls, verified lexical errors, valid payload boundaries, unresolved recognition/error boundaries, C reference precision boundaries를 분리한다. latter boundaries는 정확한 오류 비교 pass 또는 numeric payload 실행 지원이 아니다. 표준 Windows runner에 추가하고 `wn.c`/`w.c`/`ws.c`/`jerr.h` 및 실제 binary/DLL hash를 보고서에 보존한다.

**NV3d1 검증:** native Windows default/portable 각각 **469 passed / 17 ignored**, fmt/clippy/build 통과; Python **30 passed**. 양 DLL 각각 numeric syntax **1,099 cases / failed 0**: accepted noun controls **110**, lexical error equality **652**, valid payload 경계 **327**, unresolved recognition **5**, C reference precision **4**, unresolved error **1**. 마지막 경계 1건은 C의 ill-formed number와 Rust의 Unknown/Unsupported 차이를 보존한 미완료 검증이며 pass로 바꾸지 않는다. 기존 name syntax **4,125 / failed 0**, spelling **667 / failed 0**, vocabulary POS **143**/binding **140**/noun **3**, 세 runtime 경로 각각 **5,380 cases / 5,380 passed / failed 0**, stages **10,810**, words **6,623**, Scan **285 / failed 0**를 유지했다. runtime prefix 경계 **285 / executable prefix passes 0**, capture graph 경계 **257**, static 경계 **2**는 별도다. 전체 보고서 20개의 binary/source/DLL hash를 확인했다. numeric syntax 검증은 exact/complex/based payload 실행이나 full precision 성공을 뜻하지 않는다. Linux/GitHub CI/CUDA는 실행하지 않았다.

<a id="nv3d2a-quad-hex-recognition"></a>

**NV3d2a quad·Windows hex 문법 검증 — 2026-10-05.** `src/numeric_input.rs`에서 whole-word precision 선택을 유지하면서 `numfq`의 mantissa·fractional scale·소문자 `e`·64-bit exponent·`fq` suffix 및 infinity/NaN 표기를 검증한다. `2fqz`는 이제 C와 같은 ill-formed number다. exponent 자체의 범위 초과와 scale 합산의 signed overflow를 구분하며, 후자는 Unknown으로 보존한다. quad payload 생성과 arbitrary-precision 할당의 자원 오류까지 구현한 것은 아니다.

Windows `strtod`의 hex mantissa·선택적 binary exponent를 Rust에서 검증한다. C `numfd`는 nominal field 끝에 NUL을 넣지 않고 `t >= s+n`을 허용한다. 따라서 `0Xad90`/`0Xb1`은 뒤의 hex digit까지 읽어 유효할 수 있으며, `_0X0ad90`은 magnitude가 음수인 비영 값이 되어 거부된다. 반면 `numbpx`는 `p`/`x` 구분자를 임시 NUL로 바꾸므로 그 앞의 읽기 범위는 좁혀야 한다. field 길이와 실제 읽기 범위를 별도로 전달하여 이 차이를 보존한다. C FFI나 C kernel 의존성을 추가하지 않는다.

음수 hex polar magnitude의 선행 bit/exponent가 기본 IEEE binary64 환경에서 비영 값을 입증할 때만 ill-formed number로 판정한다. 이 checkpoint에서는 0 mantissa를 허용하고, underflow로 음수 0이 될 수 있는 값·합산 overflow·입증하지 못한 hex ratio 부호를 Unknown으로 남겼다. 기본 반올림의 음수 0과 NaN word 경계는 아래 NV3d2b1에서 추가 검증했다. 변경된 rounding/FTZ 환경, parenthesized NaN payload의 word formation, 전체 플랫폼 `strtod` 확장, 숫자 construction의 자원/오류 동등성은 **NV3d2b**다. 유효한 complex/based/quad 표기의 payload 생성은 여전히 Unsupported다.

근거: pinned [wn.c](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wn.c)의 `numfd`, `numfq`, `numj`, `numbpx`; 실제 Windows DLL과 source revision은 별도로 기록한다. NV3d1의 수치는 당시 검증 기록이며, 당시 미확인 quad/hex 경계는 본 단계에서 아래와 같이 갱신했다.

**NV3d2a 검증:** native Windows default/portable 각각 **471 passed / 17 ignored**, fmt/clippy/build 통과; Python **30 passed**. 양 DLL 각각 numeric syntax **2,078 cases / failed 0**: accepted noun controls **182**, lexical error equality **1,084**, valid payload 경계 **607**, integer conversion 경계 **2**, C reference precision 경계 **200**, unresolved recognition **3**. unresolved error 경계는 **0**이다. `frontend_probe`는 Unsupported의 원문 이유를 별도 진단 field로 제공한다. 검증된 문법·integer overflow conversion·C precision 미지원·미확인 문법을 실제 진단 이유로 분류하며, 구문 표본의 scope만으로 valid를 주장하지 않는다. 경계 수를 숫자 실행 성공으로 합산하지 않는다. 기존 frontend/runtime/name/spelling/vocabulary/Scan 비교를 유지하고 보고서 20개의 source/DLL/binary hash를 확인했다. runtime prefix 경계 **285 / executable prefix passes 0**, capture graph 경계 **257**, static 경계 **2**는 별도다. Linux/GitHub CI/CUDA는 실행하지 않았다. NV3d/NV3d2 전체 완료로 표시하지 않는다.

<a id="nv3d2b1-rounding-nan-words"></a>

**NV3d2b1 기본 반올림과 NaN word 경계 — 2026-10-05.** `hex_nonnegative`는 실제 읽기 범위에서 mantissa의 선행 bit와 나머지 비트를 검사한다. 기본 IEEE binary64 round-to-nearest, ties-to-even에서 `2^-1075` 이하의 음수 magnitude는 `-0`으로 반올림되어 polar 입력으로 유효하다. 정확한 중간값보다 큰 magnitude는 음수 비영 값이므로 ill-formed number다. `_0X1P_1075ad90`과 `_0X1P_9999ad90`은 이제 문법이 확인된 payload 미지원이고, `_0X1.00000000000001P_1075ad90`은 C와 같은 오류다. 긴 mantissa의 끝에 있는 sticky bit도 버리지 않는다. 숫자 payload를 생성하거나 C FFI를 호출하지 않고 이 부호 조건만 검증한다.

`1jNaN(1)`, `1jnan()`, `1jNAN(foo)`, `1j_nan(1)`은 C와 Rust 양쪽에서 숫자 prefix·괄호·선택적 내부 word로 나뉜다. Windows `strtod`의 parenthesized NaN payload 문법을 J 숫자 word로 도입하지 않는다. C의 전체 문장은 syntax error지만 Rust는 앞의 complex noun 생성 미지원에서 멈춘다. 따라서 네 표본은 word formation equality와 **payload/parser coverage boundary**이며 syntax-error 동등성이나 parser 실행 성공으로 집계하지 않는다. 부호 없는/음수 NaN, Infinity, ratio 표기를 별도 숫자 표본에 추가한다.

이 단계는 기본 반올림 환경에서 polar 부호를 검증한 범위다. 외부에서 변경한 rounding/FTZ 환경은 검증하지 않았다. 문법이 확인된 매우 큰 hex exponent도 i128 중간 계산과 부호 보존 saturation으로 polar 부호만 판정한다. 지원하는 64-bit host의 mantissa 길이 보정은 i128 범위보다 작으므로 임계값과의 순서는 보존된다. 이를 실제 숫자 payload 생성 규칙으로 사용하지 않는다. quad fractional-scale 합산 overflow, hex ratio의 변환 후 부호 및 arbitrary-precision payload의 할당/자원 오류 동등성은 **NV3d2b2**로 남긴다. C의 signed overflow를 Rust에서 재현하거나 시스템 메모리를 소진시켜 자원 오류를 추측하지 않는다. 관련 source 근거는 pinned [wn.c](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wn.c)의 `numfd`·`numj`·`numfq`·`numxTEMP`, word formation은 [w.c](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/w.c)다.

**NV3d2b1 검증:** native Windows default/portable 각각 **473 passed / 17 ignored**, fmt/clippy/build 통과; Python **30 passed**. 양 DLL 각각 numeric syntax **2,201 cases / failed 0**: accepted noun controls **182**, lexical error equality **1,140**, valid payload 경계 **672**, integer conversion 경계 **2**, C reference precision 경계 **200**, unresolved recognition **1**, NaN word formation 경계 **4**. unresolved error 경계는 **0**이다. 남은 recognition 1건은 `2.1e_9223372036854775808fq`의 scale/exponent 합산 overflow이며 실행 성공이나 정확한 오류 비교 pass가 아니다. NaN 4건은 C syntax error와 Rust payload Unsupported를 별도로 기록한다. 기존 세 runtime 경로 각각 **5,380 cases / 5,380 passed / failed 0**, stages **10,810**, words **6,623**, name syntax **4,125**, spelling **667**, vocabulary POS **143**/binding **140**/noun **3**, Scan **285 / failed 0**를 유지했다. runtime prefix 경계 **285 / executable prefix passes 0**, capture graph 경계 **257**, static 경계 **2**는 별도다. 보고서 20개의 source/DLL/binary hash를 확인했다. Linux/GitHub CI/CUDA와 변경된 floating-point 환경 검증은 실행하지 않았다.

<a id="nv3d2b2a-exact-hex-ratios"></a>

**NV3d2b2a exact hex ratio와 quad 생성 경계 — 2026-10-05.** polar magnitude에 ratio가 있으면 원래 문자열의 부호만으로 유효성을 판단하지 않는다. `real_value`에 실제 읽기 범위를 전달하고 정확히 binary64로 표현 가능한 hex 피연산자를 내부 부호 검증에 사용한다. u64로 누적 가능한 mantissa에서 trailing zero bit를 제거한 뒤 최대 53개의 유효 bit, normal exponent 범위 또는 정확한 subnormal 배수를 입증한다. 조건을 만족할 때만 `f64::from_bits`로 정확한 내부 피연산자를 만든다. 이는 J noun/complex/quad payload 실행 지원이 아니다. 누적 범위 초과·추가 반올림·hex overflow/underflow 피연산자의 수치 생성은 보수적으로 미지원으로 남긴다.

C `numfd`의 비율 계산을 따라 분모 0은 numerator/denominator의 sign xor로 signed zero 또는 infinity를 만들며, 그 외에는 나눗셈 결과에 `0 <= magnitude` 조건을 적용한다. `_0X1r2ad90`은 오류, `_0X1r_2ad90`은 유효하고 `_0X1P_1074r2ad90`은 결과가 `-0`으로 반올림되어 유효하다. NaN 결과는 거부한다. 분모도 nominal field 뒤의 hex digit을 읽을 수 있으므로 `0X1r0X0ad90`의 분모를 0으로 단정하지 않는다. C FFI를 추가하지 않고 기존 enqueue 오류 span/index와 whole-word numeric mode를 유지한다.

`2.1e_9223372036854775808fq`는 lexical grammar 미확인 대신 **quad scale/exponent construction boundary**로 분류한다. mantissa·suffix·exponent 문법이 확인되어도 C의 signed scale 합산 overflow, 숫자 생성·할당/자원 오류 동등성까지 입증된 것은 아니다. Unsupported 이유와 보고서 category를 분리하며 정확한 J 오류 비교나 숫자 실행 pass로 집계하지 않는다. malformed field가 함께 있으면 기존 ill-formed number 우선순위를 보존한다.

근거: pinned [wn.c](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wn.c)의 `numfd` ratio·signed-zero 처리, `numj`의 polar nonnegative 조건, `numfq` scale 계산과 `numxTEMP` 자원 오류다. 남은 비정확 hex ratio·변경된 rounding/FTZ 환경·quad scale overflow의 정의·payload allocation/자원 오류 동등성은 **NV3d2b2b**다. 해당 경계는 RustJ 구현 범위이며 J 언어의 제한으로 만들지 않는다.

**NV3d2b2a 검증:** native Windows default/portable 각각 **474 passed / 17 ignored**, fmt/clippy/build 통과; Python **30 passed**. 양 DLL 각각 numeric syntax **2,485 cases / failed 0**: accepted noun controls **182**, lexical error equality **1,244**, valid payload 경계 **850**, integer conversion 경계 **2**, C reference precision 경계 **200**, quad construction 경계 **1**, NaN word formation 경계 **4**, unresolved recognition **1**, unresolved error **1**. 마지막 두 경계는 각각 `_0X1P_1075r1ad90`과 `0X1P9999r0X1P9999ad90`이며, C의 성공/ill-formed number와 Rust의 미지원 차이를 그대로 기록한다. 미확인 경계를 오류 동등성이나 실행 pass로 바꾸지 않는다. exact operand 교차 ratio **280개**, 미확인 conversion **2개**, malformed/quad construction 우선순위 **2개**를 기존 corpus에 추가했다. 기존 frontend/runtime/name/spelling/vocabulary/Scan 비교를 유지하고 보고서 20개의 source/DLL/binary hash를 확인했다. runtime prefix 경계 **285 / executable prefix passes 0**, capture graph 경계 **257**, static 경계 **2**는 별도다. Linux/GitHub CI/CUDA·변경된 FP 환경·메모리 소진 테스트는 실행하지 않았다.

<a id="vocabulary-migration-checklist"></a>

### NV — 현재 J vocabulary 수렴

설계 inventory의 정본은 [현재 vocabulary 감사](#current-j-vocabulary)다. M2 frontend 순서에 연결하고 신규 task/fold/GPU executor를 동시에 구현하지 않는다.

- [x] **NV0** NuVoc current index·관련 페이지를 읽고 source spelling/POS·C DLL provenance와 분리한다. 이전 vocabulary matrix의 Taylor·obsolete 항목과 누락된 현행 form을 수정한다.
- [x] **NV1** `[.`·`].`·`]:`를 정상 core enqueue·shared parser constructor 경로로 지원하고 noun/verb 결과·NAME snapshot/late lookup·modifier train·discarded noun 효과/오류 회귀를 추가한다.
- [x] **NV2** pinned core inventory의 spelling/POS 인식을 확장하고 semantic construction/실행 capability와 분리한다. descriptor 108개와 실제 noun `a.`/`a:`를 추가했다. C가 수용한 143개 POS, bare function binding/AR 140개와 noun payload 3개가 양 DLL에서 일치했다. inventory pass를 실행 지원으로 승격하지 않는다.
- [ ] **NV3** invalid/obsolete spelling의 정확한 J 오류를 C `spellin`/enqueue와 대조해 일반화한다. valid 미지원 primitive와 invalid spelling을 구별하며 임의 예외 목록으로 해결하지 않는다.
- [x] **NV3a** fixed ASCII spelling과 미등록 inflection의 enqueue 오류를 일반화했다. 현대 core dictionary를 우선 조회하고 obsolete 예외 목록 없이 C 오류 분류를 따른다. `name_:`와 유효 미지원 numeric family는 별도 coverage 경계이며 전체 NV3는 미완료다.
- [x] **NV3b** bounded direct/indirect/debug-frame/by-value 이름 문법을 shared enqueue에서 검증한다. valid lookup은 Unsupported이며 NAME 길이 제한은 NV3c에서 다루고 locale 실행·numeric grammar를 남긴다.
- [x] **NV3c** NAME 전체·simple-name·locale storage 길이 제한과 enqueue 오류 우선순위를 C `nfs` 및 양 Windows DLL로 검증했다.
- [ ] **NV3d** numeric grammar의 valid 미지원 family와 실제 ill-formed number를 C `connum`/`wn.c`로 구별한다.
- [x] **NV3d1** whole-word numeric family 선택과 검증된 extended/rational/complex/based 표기의 오류를 일반화했다. payload 생성 미지원과 문법 Unknown을 구분한다.
- [ ] **NV3d2** dedicated quad grammar·플랫폼별 `strtod` 확장 및 숫자 construction의 resource/error 경계를 검증한다. Unknown 문법을 실제 Invalid로 추측하지 않는다.
- [x] **NV3d2a** bounded quad·Windows hex 문법과 field/read-window 차이를 양 DLL로 검증했다. payload 실행 지원과 구분한다.
- [ ] **NV3d2b** resource·scale overflow·rounding/FTZ·NaN payload 및 남은 플랫폼 conversion 경계를 검증한다.
- [x] **NV3d2b1** 기본 ties-to-even의 hex polar 부호와 NaN 괄호 word 경계를 양 DLL로 검증했다. 매우 큰 exponent의 부호 판정과 숫자 payload 생성은 분리한다.
- [ ] **NV3d2b2** quad scale overflow·hex ratio 부호·변경된 FP 환경·payload 할당/자원 오류 동등성을 검증한다. 숫자 construction 미지원을 J 오류로 바꾸지 않는다.
- [x] **NV3d2b2a** exact hex 피연산자의 polar ratio 부호·signed zero·분모 읽기 범위와 quad 생성 경계를 양 DLL로 검증했다.
- [ ] **NV3d2b2b** 추가 반올림/overflow가 필요한 hex ratio·quad scale 정의·변경된 FP 환경·payload allocation/자원 오류 동등성을 검증한다.
- [ ] **NV4** 누락 family의 valence/rank/constructor/효과·오류 계약을 순차적으로 검토한다. `/..`·Fold·task/pyx·precision·scope의 의미를 단순 alias나 pure array kernel로 축소하지 않는다.
- [ ] **NV5** NuVoc 전체 form·structural/control inventory와 지원 행렬의 수렴을 확인한다. 각 단계마다 Windows 차분 gate를 갱신하고 full J 지원과 제한 corpus 통과를 구별한다.

**NV1 gate (historical, 2026-10-05):** native Windows default/portable 각각 **431 passed / 17 ignored**, fmt/clippy/build 통과, Python **27 passed**. 추가한 공통 구문 **37개**를 포함하여 C 기본·AVX2 각각 세 runtime 경로 **5,368 cases / 5,368 passed / runtime 경계 0 / failed 0**, stage **10,810 checks**, words **6,623 cases**, 실패 0. capture graph **254건**과 static **2건** 경계는 별도다. frontend report 10개와 vocabulary report 2개의 실제 source/reference/binary hash를 확인했다. vocabulary 후보 **145개**에서 **enqueue POS 검증 33 / Rust Unsupported 110 / source code-only 거부 후보 2 (`?:`, `` `. ``)**이며 legacy/invalid 표기 8개는 양 DLL의 spelling error를 확인했다. 이 숫자는 전체 NuVoc/J 실행 지원률이 아니다. 이 gate 당시 NV2–NV5와 DB1–DB7은 미완료였으며 ordered effect graph·정적 discarded-noun 보존 경계, 미지원 task/fold/precision/범용 scope 실행을 분리한다. optimizer/CUDA/Linux 실행 테스트/GitHub CI는 계속 유보한다.

**NV2 gate (2026-10-05):** native Windows default/portable 각각 **435 passed / 17 ignored**, fmt/clippy/build 통과, Python **27 passed**. 공통 C 구문 12개를 추가해 양 C DLL의 세 runtime 경로 각각 **5,380 cases / 5,380 passed / runtime 경계 0 / failed 0**, stage **10,810 checks**, words **6,623 cases**, 실패 0. capture graph **257건**, static **2건** 경계는 별도다. vocabulary 후보 145개 중 **enqueue POS 143 / Rust enqueue 미지원 0 / code-only 거부 후보 2**이며 **bare function parser binding/AR 140개 / noun payload 3개**를 비교했다. 전체 NuVoc/J 실행 지원률이 아니다. report 12개의 실제 DLL/binary/source hash를 확인하고 NV2를 완료 처리했다. 다음 단계는 NV3이며 NV4/NV5와 DB1–DB7은 미완료다.

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
- [x] 과거 중복 execution IR 경계를 구조 부채로 식별했고 M1에서 제거했다.
- [x] `physical.rs`는 read-only CPU affine representation 기반과 제한적 검색 알고리즘 선택기를 제공하지만 전체 Physical Plan/bufferization planner는 아직 없음을 명시했다.
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
| Physical Plan/bufferization | **전체 planner는 아직 없음** (CPU 검색 strategy selector만 구현) | Physical Planner | J parser/FunctionEntity를 직접 해석하지 않음 |
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

#### M1 — canonical Logical Execution IR로 cutover — 완료

`logical_ir::Plan`이 유일한 canonical execution IR이다. 아래는 현재 유지되는 완료 조건이며 제거한 과도기 container를 현행 모듈처럼 설명하지 않는다.

- [x] shared target-independent execution-semantic contract를 `execution_semantics.rs`로 분리했다.
- [x] `analysis::lower_graph()`가 J Graph IR에서 A3 op/value/check를 직접 생성한다.
- [x] `Plan::from_transition`, `TransitionProjection`, `transition_ir` module/container 및 `analysis::LogicalPlan`을 제거했다.
- [x] `CompilationAnalysis`는 `j_graph`, rewrite/resource views와 canonical `logical`만 묶으며 `transition` 필드가 없다.
- [x] `Engine::analyze`, `analyze_a3`, `analyze_diagnostic`은 canonical Logical IR을 반환한다.
- [x] verifier/reference-executor/lowering test consumer를 canonical `logical_ir::Plan`으로 전환했다.
- [x] Graph origin/source span, name version, semantic check와 write ordering의 direct-A3 regression을 추가했다.

**M1 완료 조건:** 충족. `J Graph IR → logical_ir::Plan` 직접 연결과 단일 execution container가 유지되어야 한다. 이 완료는 full frontend, implicit cell semantics 또는 native Physical Executor의 완료를 뜻하지 않는다.

#### M2 — jsource-compatible frontend/parser cutover

목표: 현재 heuristic parser를 jsource-compatible Word Formation → Enqueue → 9-row Parser pipeline으로 교체한다.

- [x] F0 differential 0-mismatch 기록을 완료했다. pinned source 기록과 Windows 일반/AVX2 배포본 재검증을 구분한다.
- [ ] F1 Enqueuer/PrimitiveResolver를 완료한다.
- [ ] F2 Parse Queue를 완료한다.
- [x] P1 parser control class와 semantic entity/value를 분리했다. parser-time lookup/effect sequencing의 완성은 P4/P2의 별도 gate다.
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

- [x] **M3-RB 설계 명세(2026-10-07):** §2.5.1의 RB-01~08 의미론 보존·proof/guard·source coverage·Write commit 및 RB-V0~V4 검증 기준을 확정했다. **문서만 완료**.
- [ ] **M3-RB 실행 검증(구현 보류):** 원본 A3 독립 대조, RouteBoundary validator, Check/effect/error/order negative tests, J oracle 차분 결과를 실행 후 수용한다. 현재 `partition_plan`은 분석 후보 분류기다.
- [x] **M3→M4 인계 설계(2026-10-07):** §5.2.1 H-01~09, HM-V0~V4로 RouteVerified 증거 → PhysicalVerified → RuntimeReady의 소유권·검증 의무를 문서화했다. **설계만 완료, 신규 구현 없음.**
- [ ] **M3→M4 인계 검증(미구현):** 원본 A3-Route-Physical Check/Write coverage, selected realization, guard/freshness 및 negative/differential 테스트의 실제 통과 증거 확보.
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
- [x] **문서 계약:** §5.2.1에서 최소 Physical Plan op를 `BindInput/Check/View/Materialize/Kernel/Return`으로 정의하고 plan-time/runtime identity·verifier·cleanup 경계를 고정했다.
- [x] **M4 인계 검증 계약(2026-10-07):** §5.2.1의 H-01~09/HM-V0~V4에서 물리화 전 반드시 필요한 M3 승인, buffer/view/lease, Check/guard/오류 순서와 runtime 재검증을 명문화했다. **문서 완료만** 의미한다.
- [x] **M3→M4 첫 CPU 경로 승인 판정 명세(2026-10-07):** §5.2.1 H-A의 A0~A5 단계와 HA-V1~7로 `ReferenceSequential` 후보와 실제 compiler-native 실행을 분리하고 Check/Write/guard/region-local 승인 부정 검증을 정의했다. **설계 전용**.
- [x] **M3→M4 인수 패킷·검증 판정표(문서 전용, 2026-10-07):** §5.2.1 H-P의 HP-01~12 입력 증거/검증/거부 및 HP-V01~09 단일 조건 변조 시험·판정 기록 양식 작성. 코드 구현이나 전역 승인과 무관.
- [ ] **HP-01/02 원본·커버리지:** immutable A3 identity와 모든 original op/zero-result Check/별도 Write·cross-route interface를 source 보존 증거에 매달아 검증한다(HP-V01/02).
- [ ] **HP-03/04 native capability·Check:** selected 실제 CPU recipe 존재, 원본 Check의 execute/discharge owner 및 첫 J error precedence를 검증한다(HP-V03).
- [ ] **HP-05/06 Name·semantic value:** noun snapshot/late NameRef/Write owner 및 zero-frame Rank·boxed/sparse·shape/atom J 의미를 보존한다(HP-V04).
- [ ] **HP-07/09 buffer·completion:** plan/runtime 버퍼 구분, view span/alias/generation/last-use, 동기 CPU completion 근거를 검사한다. 비동기 토큰/전송 구현은 M5/M6까지 필수 아님(HP-V05/07).
- [ ] **HP-08 dependency:** producer/check/guard/effect/error 선행 edge를 검증하고 누락·순환을 거부한다(HP-V06).
- [ ] **HP-10/11 runtime·fallback:** guard freshness/효과 전 대체 경로/자원 한계, CPU+Host+zero-transfer 제한을 검증한다(HP-V08).
- [ ] **HP-12 acceptance:** 실제 native 결과와 RustJ reference·가능한 J C oracle의 차분 및 부정 테스트 PASS/CI 증거를 gate별 기록한다(HP-V09).
- [ ] **M3→M4 승인 판정 실행 검증(미착수):** HA-V1~7 verifier·guard 연결 및 jsource/semantic/native 차분 테스트의 실제 PASS 기록.
- [ ] **구현:** 위 contract를 concrete `PhysicalPlan`/op Rust 타입과 verifier로 구현한다.
- [ ] logical ValueId → plan-time `PlanBufferId`/PhysicalView → runtime `BufferLease/BufferId` binding을 구현한다.
- [ ] G2 transpose/reverse/slice/compatible reshape/zero-stride agreement view를 planner에서 선택 가능하게 한다.
- [ ] G3의 첫 kernel로 contiguous/fixed/general-stride add를 연결한다.
- [ ] G3 cell mapping과 ExecutionBasis `CellApply`를 physical view iteration에 연결한다.
- [ ] G4 CPU Physical Executor를 구현한다.
- [ ] `source → FunctionEntity → J Graph → logical_ir::Plan → Physical Plan → CPU Executor` vertical test를 만든다.
- [ ] 기존 semantic/reference executor와 결과/error contract를 비교한다.

**M4 완료 조건:** 기존 interpreter 직접 실행을 거치지 않는 최소 compiler-native CPU 경로가 하나 이상 동작한다.

**PR #1 IL1/IL2 통합 경계:** `RankSpec`/`RankContract`와 `CallOp::cell_application(left_facts, right_facts)`는 현재 immutable FunctionEntity의 검증된 requested rank 및 primitive registry innate rank를 소비한다. 중첩 explicit/innate layer, J prefix agreement·residual repetition·iteration count·zero-frame prototype을 별도 관찰한다. 이전 draft의 분석기/SSA 교체 대신 현재 J Graph → A3 경계를 보존하는 재계산 inspection API로 이식한다. 기존 RankPlan·facts 추론·runtime/physical execution은 유지한다. unknown shape/rank는 None이며 조립/오류/최적화 legality나 실행 지원을 증명하지 않는다. L4 전체 및 M3/M4는 미완료다.

<a id="heterogeneous-execution-checklist"></a>

**M4-P0 제한된 코드 통합(PR #4):** `physical_plan`은 plan-time buffer/view ID와 runtime BufferRegistry를 분리하며, 완전히 빈 A3 또는 closed Bool/Int/Float/Char literal 하나의 BindInput → Return만 검증·실행한다. exact literal/source witness로 stale payload/facts/span을 거부하고 singleton stride와 alias를 검증한다. ExtendedInt/Rational/boxed/sparse 및 미지원 op·SemanticCheck·쓰기·이름 조회는 거부한다. Add/Check/View/Materialize/lifetime/reuse와 일반 M3 proof는 미구현이며 M3/M4 전체 완료가 아니다. 현재 런타임 경로는 변경하지 않는다.

#### HE — 이종 CPU/GPU 실행 계획 수렴 (2026-10-07, M4→M6 연계 체크리스트)

**설계 결정:** RustJ는 **CPU 멀티코어 컴파일러가 아니라 이종 배열 컴파일러**다. CPU thread 병렬화는 Physical Plan의 device-local 실행 전략일 뿐이다. `ParallelLegality` 또는 CPU 전용 `ParallelPhysicalPlanner`를 독립적인 canonical 상위 단계로 도입하지 않는다. 기존 `J Graph IR → verified logical_ir::Plan → RoutePartition → Schedule/Transform → Physical Planner → Physical Execution Plan → Executor`를 유지한다. 이 체크리스트는 본 §10의 기존 M/IO 이행 계획에 통합되며 별도 Markdown이나 새로운 필수 IR을 만들지 않는다.

**교차 프레임워크 감사 (참조 대상과 채택 경계).**

| 참조 | 실제 근거 | RustJ 도입/비도입 |
|---|---|---|
| [IREE Stream](https://iree.dev/reference/mlir-dialects/Stream/) 및 [stream passes](https://iree.dev/reference/mlir-passes/Stream/) | Flow에서 target affinity, resource, asynchronous scheduling을 명시한 Stream으로 내린 뒤 HAL에서 실제 device execution | Route/physical plan에 region affinity, readiness·completion dependency, ownership/lifetime 계약을 **점진적으로 도입**; IREE 전체 스택 또는 새로운 Stream IR 복제 금지 |
| [Kokkos View](https://kokkos.org/kokkos-core-wiki/ProgrammingGuide/View.html) · [Memory Spaces](https://kokkos.org/kokkos-core-wiki/API/core/memory_spaces.html) | ExecutionSpace와 MemorySpace가 1:1이 아니고 가시성·동기화가 별도 | **장치 실행 위치 ≠ 버퍼 저장 위치**를 별도 물리 사실로; unified/shared memory를 공짜 zero-copy로 간주 금지 |
| [MLIR scf.forall](https://mlir.llvm.org/docs/Dialects/SCFDialect/) · [tensor.parallel_insert_slice](https://mlir.llvm.org/docs/Dialects/TensorOps/) | 논리적인 병렬 반복·mapping·부분 결과 결합을 분리, 반복 간 side effect ordering은 미보장 | A3 `IterationDomain`과 schedule/device mapping 분리 유지. 독립 출력/조립 J 의미 proof 없이 동시화 금지 |
| [XLA CPU ParallelTaskAssignment](https://github.com/openxla/xla/blob/main/xla/service/cpu/parallel_task_assignment.cc) | FLOP/byte 기반 task 수와 thread overhead 평가, 내부 병렬화된 kernel 회피 | CPU backend 내부의 작업량·대역폭 비용 참고; GPU/CPU 장치 배치 비용을 CPU thread threshold로 대체 금지 |
| [Futhark multicore scheduler](https://github.com/diku-dk/futhark/blob/master/rts/c/scheduler.h) · [Rayon](https://docs.rs/rayon/latest/rayon/) | chunk/task amortization, nested work 분배, work stealing | **CPU 실행기 내부** 실행 전략; RustJ logical IR에 thread pool·thread count 인코딩 금지 |

**소유권/경계 계약:**
1. **Semantic/legality:** `logical_ir.rs`의 J 결과 타입·Shape, Rank/CellApply/zero-frame virtual fill, dynamic names/versions, comparison/fit, alias, effect와 error precedence가 권위다. `IterationAxisKind::Parallel`은 구조적 축이며 **동시 실행 가능하다는 증명이 아니다**. 동시 실행·분할·중복 계산은 기존 `lowering.rs` legality와 별도 witness/guard에서 결정한다; Unknown은 거절 또는 합법 baseline 경로로 유지한다.
2. **Route/device:** `RouteRegion`의 후보 장치(CPU/GPU/external)와 intra-device schedule(Sequential/SIMD/CPU workers/GPU grid)은 직교한다. full J에 모든 연산이 동일한 backend에 내려가야 한다는 의무가 없다. 단일 CPU/단일 region/all sequential은 정상적으로 유효한 계획이다.
3. **Placement/memory:** J `ValueId`, plan-time `PlanBufferId`/logical version, runtime `BufferId`/`BufferLease`/memory residency/ready state는 서로 다르다. Host/NUMA/GPU/managed/pinned/file memory 위치가 계산 실행 장치와 별도 축이다. view, no-op, copy, prefetch, migration, materialize 후보를 의미와 비용에 맞춰 고른다. 공유·통합 메모리는 접근권과 coherence/동기화 검증을 없애지 않는다.
4. **Dependencies/completion:** 후보 Physical Execution Plan은 value/data readiness, effect/order, error precedence, lease/lifetime, transfer/compute completion을 추적한다. 비동기 overlap은 dependency의 critical path와 실제 capability가 증명될 때만 이득으로 계산한다. 같은 output에 대한 중복 write, check-before-effect 위반, 오류의 arbitrary first-thread winner를 금지한다.
5. **Resource/cost:** `j_graph_work_depth.rs`와 `j_graph_resource.rs`의 target-independent facts를 소비하되 `ResourceEstimate`(가능 여부), `CostEstimate`(예측), 측정 `CostProfile`(관찰)을 합치지 않는다. compute FLOP·byte, memory bandwidth/capacity·residency, intermediate materialization, transfer bytes/latency, launch/sync, queueing와 peak in-flight bytes를 분리해 비교한다. 리소스 Unknown ≠ feasible, 비용 Unknown ≠ zero.
6. **범위:** 실제 GPU/CUDA·multi-device/async runtime 도입은 기존 M6/CUDA 보류 정책을 따른다. M4 최소 CPU slice에 GPU, NUMA, 새 필수 IR, 범용 혼합 스케줄러를 끼워 넣지 않는다. 외부 MLIR/StableHLO adapter의 독립적인 경로도 유지한다.

**실행 게이트/체크리스트 (이 표 외 중복 작업표 생성 금지):**

| 게이트 / 선행 | 상태 | 수용 기준 |
|---|---|---|
| HE-00 / M2 병행 | [x] 이종 실행 중심 원칙 및 IREE/Kokkos/MLIR/XLA/Futhark/Rayon 역할 문서화 | 본 결정과 `FOUNDATIONS.ko.md`·`AGENTS.md`에 반영한 **설계 완료만** 뜻함; runtime/benchmark 아님 |
| HE-01 / M4 | [ ] all-CPU / single-device / zero-transfer `PhysicalPlan` 구현 | 원래 `BindInput/Check/View/Materialize/Kernel/Return` 순차 baseline, `logical_executor` 및 J oracle 의미 대비; M4 완료 게이트에 포함 |
| HE-02 / M4→M5 | [ ] 실행 위치/메모리 위치/내부 병렬화를 분리한 planner-side contract와 verifier | A3 schema·J Graph·parser 불변, unknown capability와 resource 거절, non-overlap/liveness; multi-device 구현은 요구하지 않음 |
| HE-03 / M5 | [ ] `ExecutionLegality`/witness/guard **분석**을 기존 lowering 경계에 연결 | Effect·NAME version·alias·error precedence·rank fill·dynamic fallback 반례; IR의 Parallel 축만으로 허가하면 실패 |
| HE-04 / M5 | [ ] CPU Sequential/SIMD/worker 후보와 measured cost selection | worker count/tiling/compute bandwidth/launch/pool overhead, 반복 병렬 폭발과 nested oversubscription 방지; 모든 후보 합법성 검사 |
| HE-05 / M5 | [ ] Device/Memory placement·데이터 이동 비용 표현 | copy/migrate/prefetch/ready dependencies, capacity/residency, transfer+sync bytes/time, overlapping 가능 범위 구분; `IO-20`과 공통 physical planner 사용 |
| HE-06 / M5 이후 | [ ] 단일 CPU 실측과 가상의 혼합 candidate의 비용·정합성 비교 | 순차 baseline보다 작업이 적지 않으면 병렬·GPU 경로를 기본 선택하지 않음; cold/warm compile/transfer/performance 분리 |
| HE-07 / M6 이후 | [ ] CPU+GPU runtime completion/transfer 연결 및 end-to-end 검증 | 검증 가능한 실제 GPU와 별도 구현 재개 승인 전 **미착수**. data version, transfer/check order, async failure, invalid device, unsupported route 검증 |
| HE-08 / M6 이후 | [ ] 병렬 CellApply/Reduction/Scan의 J 의미 회귀 및 부정 사례 | zero-frame virtual cell, positive frame/empty cell, result-cell shape/type join, boxed/sparse, tolerance, floating reassociation, J error priority; 미증명 fold/scan은 순차 유지 |
| HE-09 / M5 이후 | [ ] 스케줄 단계별 관찰값·벤치마크 및 diagnostic | route+device+memory+worker 선택 이유, unknown/reject/guard 근거, bytes moved, sync count, peak memory, output J equivalence. GPU 실행 완료로 오인 금지 |

**작업 순서:** HE-00(설계) → 기존 M2→M3 수렴 → M4/HE-01 순차 vertical slice → HE-02/03 legality/representation → HE-04/05/06 cost·CPU worker·transfer 후보 → 별도 승인을 전제로 HE-07/08/09. `IO-20`도 이 범용 physical placement/transfer 모델에 합류하며 두 번째 전송 추상화를 만들지 않는다. 모든 실행·성능 수용은 §11 검증 정책과 실제 결과를 따른다. **이 문서 갱신만으로 HE-01~09를 완료 처리하지 않는다.**

---

#### M5 — Route/Schedule/Cost 확장

M4 이후에만 optimizer 선택 문제를 키운다. [sw-MLPL SW-05/06 계측 참고](#sw-mlpl-reference-checklist)는 기존 physical residency/transfer 비용 검증에만 적용하고 M5/M6 구현 완료를 뜻하지 않는다.

- [ ] `RouteRegion`에 boundary inputs/outputs, chosen route, preconditions/witnesses, semantic provenance를 추가한다.
- [ ] legality와 profitability를 계속 분리한다.
- [ ] StructuralOpportunity/use-def를 schedule candidate와 연결한다.
- [ ] materialize/view/fuse 후보의 physical feasibility를 계산한다.
- [ ] TargetProfile/ResourceEstimate/CostEstimate 최소 schema를 구현한다.
- [ ] cold compile, warm execution, copy, layout conversion, transfer, synchronization 비용을 분리한다.
- [ ] simple CPU cost model로 multiple legal realization 중 하나를 선택한다.

**M5 완료 조건:** planner가 단순 고정 policy가 아니라 여러 합법 physical plan 중 cost/resource 근거로 선택할 수 있다.

#### M6 — External/ArrayFire/GPU route

M4의 compiler-native vertical slice와 M5의 route contract가 안정된 뒤 진행한다. 모든 adapter는 먼저 §5.5.1 External adapter boundary contract를 만족해야 한다.

- [ ] Graph/Execution Basis ↔ ArrayFire capability matrix를 만든다.
- [ ] ArrayFire route의 dtype/rank/shape/layout/J-semantic precondition을 명시한다.
- [ ] J row-major ↔ ArrayFire column-major mismatch를 view/copy/consumer-absorption 선택 문제로 physical planner에 연결한다.
- [ ] external handle lifetime/lock/release/sync를 Physical Plan resource/token 경계로 모델링한다.
- [ ] MLIR adapter와 StableHLO-safe subset adapter의 공통 negotiation interface를 정의한다.
- [ ] external route failure가 J semantic failure가 아니라 route unsupported/fallback으로 처리되는 테스트를 만든다.
- [ ] 실제 GPU storage/kernel은 별도 요청과 검증 가능한 환경이 있을 때 재개한다.

**M6 완료 조건:** external library/backend가 J semantics를 정의하지 않고, verified Logical IR의 합법적인 realization route 중 하나로만 동작한다.

#### JE0–JE6 — JEntity / contextual higher-order view 보조 트랙

**목표:** jsource의 공통 `A` 표현에서 **semantic RHS(`NOUN + FUNC`)를 하나의 carrier로 전달하는 원리**를 참고하되 C runtime allocation 구조를 복제하지 않는다. RustJ의 최소 `JEntity`는 parser/binding/assignment/semantic-operand 경계의 얇은 sum type으로 사용한다. `Value`와 `FunctionEntity`의 내부 모델을 하나로 합치거나 모든 IR node의 공통 base type으로 만들지 않는다. higher-order collection은 gerund 등 실제 operator semantics가 요구할 때 operator-specific view부터 도입한다.

**위상과 실행 순서:** 이 트랙은 M0–M6의 critical path에 새 milestone을 끼워 넣지 않는다.

- JE0 감사는 M2와 병행한다.
- **JE1의 최소 `JEntity` carrier는 M2 중에도 도입할 수 있다.** 단, `AssignedValue`/`SymbolValue`/`ParserNameBinding`/`FunctionOperand` 같은 중복 carrier를 한 seam씩 줄이는 경우에만 한다. full frontend가 끝날 때까지 중복 타입을 더 굳힌 뒤 대규모 migration하는 것을 피한다.
- JE2 parser/binding/assignment 수렴은 M2 작업 자체와 함께 진행할 수 있지만, jsource compatibility case가 먼저 있어야 한다.
- gerund/general collection abstraction(JE3+)과 noun storage를 건드리는 migration은 M2가 안정된 뒤 수행하며, physical representation과 결합되는 변경은 M3 경계가 안정된 뒤로 미룬다.
- 어느 JE 단계도 첫 M4 CPU vertical slice를 불필요하게 막지 않는다.

**jsource에서 채택하는 원리 / 채택하지 않는 구현:**

- 채택: semantic RHS에서 noun과 function entity가 하나의 상위 J-entity universe에 속하고 POS/type가 해석을 결정한다.
- 채택: `JEntity`의 직접 범위는 jsource `RHS = NOUN + FUNC`, `FUNC = VERB + ADV + CONJ`에 대응한다. NAME/ASGN/MARK/SYMB 같은 parser/runtime block class를 같은 semantic entity kind로 억지 통합하지 않는다.
- 채택: function의 parser/binding transport는 common entity handle을 사용할 수 있지만 noun snapshot과 function nameref late lookup의 시점 차이를 보존한다.
- 채택: gerund처럼 boxed noun이 shape를 제공하는 문맥에서는 그 noun의 shape를 보존한 entity-collection view를 만들 수 있다. shape는 container에 속한다.
- 채택: derived function은 operand/function identity를 보존하는 first-class entity다.
- 채택: parser/binding/assignment는 noun뿐 아니라 verb/adverb/conjunction 결과도 하나의 J entity로 전달할 수 있어야 한다.
- 비채택: jsource `AD/A` allocation header, refcount, virtual/in-place flags, allocator metadata를 semantic identity와 결합하는 방식.
- 비채택: verb를 noun의 physical atom buffer와 동일한 representation으로 강제하는 방식.
- 비채택: jsource 공통 header에 rank/shape field가 있다는 이유로 Verb/Adverb/Conjunction 자체에 noun-style semantic rank/shape를 부여하는 방식. current jsource도 function의 AN/AR field를 사용하지 않는다.
- 비채택: unresolved lexical NAME/NameRef를 Noun/Function과 같은 추가 POS/entity kind로 취급하는 방식.
- 비채택: 공통 entity abstraction이 Logical/Physical Array 경계를 우회하거나 BufferId/device/layout을 semantic layer로 끌어올리는 방식.

##### JE0 — 현행 semantic carrier와 jsource 대응 감사

- [x] 상위 목표를 확정했다: `JEntity`는 noun/function을 묶는 semantic abstraction이고, common physical allocation abstraction이 아니다.
- [x] jsource의 공통 J-entity 원리와 RustJ의 `Value` / `FunctionEntity` / `FunctionOperand` 구조가 대응 가능함을 설계 수준에서 확인했다.
- [x] current jsource의 `RHS = NOUN + FUNC`, `FUNC = VERB + ADV + CONJ`를 다시 확인하고 RustJ `JEntity`의 직접 대응 범위를 semantic RHS로 한정했다.
- [x] current jsource가 function의 AN/AR를 사용하지 않는 것을 확인하여, function 자체에 noun-style shape/rank를 부여하지 않는 원칙을 고정했다.
- [x] `p.c` row 7이 noun/verb/adverb/conjunction RHS assignment를 허용하고, `sc.c::jtnamerefacv`가 noun value와 function nameref를 서로 다르게 처리하는 것을 확인해 공통 transport와 lookup timing을 분리했다.
- [x] `cf.c` bident/trident table에 Function뿐 아니라 즉시 Noun 결과가 존재함을 확인하여 parser construction의 일반 result type을 `JEntity`로 정정했다.
- [x] `cg.c::jtfxeachv/jtfxeach`가 source boxed noun의 rank/shape를 복사하지만, 결과는 jsource 주석상 **BOX라고 주장하는 내부 carrier에 function-typed A를 넣는 realization trick**임을 확인했다. 이것을 generic semantic `EntityArray`의 직접 선례로 사용하지 않는다.
- [x] current RustJ에 `FunctionOperand`, `ParserNameBinding`, `AssignedValue`, runtime `SymbolValue`, `ExprKind::{VerbValue,ModifierValue,Literal}` 등 Noun/Function carrier가 중복되어 있음을 확인했다. 최소 `JEntity`는 M2 이후의 장기 리팩터링보다 M2 seam 수렴에도 가치가 있다.
- [x] current `Verb { target, entity }` wrapper와 `FunctionEntity`의 책임이 완전히 수렴하지 않았음을 확인했다. `JEntity::Function` payload를 확정하기 전에 `VerbTarget`이 semantic identity인지 migration/execution adapter인지 감사한다.
- [x] lexical NAME과 function nameref를 구분했다. lexical/unresolved NAME은 JEntity가 아니지만, lookup 결과로 만들어진 executable nameref는 POS를 가진 Function entity이므로 `JEntity::Function` 안에서 `NameRef` head로 존재할 수 있다.
- [x] current jsource 기준 revision을 `0db94e768a845e2583c01d00538c3d16379677bb`(2026-10-03 master)로 고정해 이번 JE 감사의 비교 기준을 기록했다.
- [x] `Value`, `FunctionEntity`, `Verb`/`VerbTarget`, `FunctionOperand`, parser stack item, `ParserNameBinding`, `AssignedValue`, runtime `SymbolValue`, binding result, `NameRef`, `DefinitionCode`, gerund decode/view가 각각 어떤 semantic identity와 transport 책임을 보유하는지 inventory를 만든다.
- [x] `p.c` runtime parser와 `pv.c` tacit translator의 9-row 계열 코드를 구분해 근거를 기록한다. `pv.c::jtvis` 같은 translator action을 runtime observable semantics의 단독 oracle로 사용하지 않는다.
- [x] noun/verb/adverb/conjunction이 같은 parser/binding/assignment 경계를 통과하는 대표 jsource differential 사례를 정리한다.
- [x] 현재 `FunctionOperand::{Function,Noun}`와 다른 sum-type/enum 중 사실상 중복된 J-entity carrier를 식별한다.
- [x] current `Value`의 `CpuStorage` migration artifact가 `JEntity` API에 새 canonical dependency로 고착되지 않도록 금지 경계를 명시한다.

**JE0 완료 조건:** 모든 current semantic carrier와 lifetime/ownership/provenance 책임을 표로 설명할 수 있고, 새 타입을 만들기 전에 어떤 중복을 제거할지와 어떤 차이는 유지할지가 결정되어 있다.

##### JE0 현행 carrier 감사와 첫 migration seam (2026-10-04)

아래 표는 JE1 이전 감사 snapshot이다. `AssignedValue` 제거와 현재 API는 이어지는 JE1 구현 기록에서 관리한다.

| 현행 carrier | identity·ownership·lifetime | 유지할 metadata와 migration 결정 |
|---|---|---|
| `Value` / `Data` | noun의 J type·shape·atom order; boxed child는 `Arc<Value>`, sparse는 shared semantic array. owned dense clone은 payload copy이며 freeze 후 clone은 공유 | CPU payload는 현행 migration artifact다. 미래 `JEntity`는 `Value`를 transport하되 CpuStorage·host slice·BufferId·layout·device API를 새로 노출하지 않는다 |
| `FunctionEntity` | POS·head·ordered operands를 갖는 immutable `Arc` DAG; definition Code와 intrinsic noun snapshots를 소유 | Function payload는 `Arc<FunctionEntity>`로 충분하다. callable rank 계약과 noun shape를 혼동하지 않는다 |
| `Verb` / `VerbTarget` | `span + target + Arc<FunctionEntity>`; Primitive/Named target은 semantic head와 중복되고 Derived는 migration marker | production execution은 `resolve_function_entity`로 DAG를 조회한다. target 직접 검사는 현재 parser test host에서만 사용한다. 첫 seam에서 target을 새 semantic identity로 만들지 않고 wrapper의 occurrence span은 따로 보존한다 |
| `FunctionOperand` | Function child Arc 또는 freeze된 concrete noun + operand span; noun의 생성·대입 정보와 같은 것은 아니다 | JEntity와 payload union이 중복되지만 noun span이 추가되어 있다. 첫 migration 대상에서 제외하고 이후 zero-loss adapter로 연결한다 |
| parser `Item` / `ParseValue` | class·source/provenance·flags·occurrence·span override; noun은 `Expr + height`, function은 completed DAG/Verb wrapper | expression dependency, abstract/deferred noun 및 parser control state 때문에 concrete entity carrier와 통합할 수 없다. NAME/target/control을 JEntity POS로 만들지 않는다 |
| `ExprKind` / `Program` | literal/function result 외에도 Group·ReadName·Monad·Dyad 구조와 reduction/write provenance를 소유 | computation structure는 RHS carrier의 중복이 아니다. JEntity 도입으로 application graph를 제거하지 않는다 |
| `ParserNameBinding` | concrete noun snapshot, abstract noun class, function POS, known modifier + version이라는 lookup observation | 아직 값이 없는 abstract noun/POS와 실제 RHS를 구분하므로 enum을 유지한다. 모든 lookup을 entity snapshot으로 바꾸지 않는다 |
| `AssignedValue` | row 7의 concrete Noun/Verb/Modifier RHS를 host에 넘기고 대입 결과를 돌려받는 transport | **JE1 첫 seam**. Noun/Function 두 variant로 바꾸되 function POS는 DAG에서 얻고 occurrence/height/assignment source는 parser에 남긴다 |
| runtime `SymbolValue` / `Binding` | freeze된 noun 또는 shared function wrapper; 별도의 Engine-local NameVersion. replacement는 old noun을 pool에 retire | AssignedValue와 semantic RHS payload가 중복된다. 첫 JE1은 host boundary adapter만 교체하며 전체 symbol table과 pool migration은 JE2 이후다 |
| binding result / `BoundProgram` | 분석용 reads·versions·dynamic function references·pending write; 실행 가능한 cached plan이 아님 | entity identity와 binding/version proof를 합치지 않는다. read-only prepare는 commit하지 않는다 |
| lexical NAME / `FunctionHead::NameRef` | lexical queue name은 unresolved spelling/flags; lookup 후 function NameRef는 expected POS를 가진 executable function identity | noun은 lookup snapshot, function은 필요한 경우 적용 때 재조회한다. 같은 공통 carrier가 이 timing 차이를 없애면 안 된다 |
| `DefinitionSource` / `DefinitionCode` | shared original source·primitive context·span, immutable body/valence/control metadata; invocation locals 없음 | Function head가 Arc Code를 소유한다. noun DD는 Value이며 Code가 아니다. alias assignment는 본문 호출이 아니다 |
| gerund noun / `decoded_gerund` | source는 boxed noun; decode는 ordered `Vec<Arc<FunctionEntity>>` 실행 auxiliary이며 source semantic children이 아니다 | order와 noun snapshots는 보존하지만 Vec 자체에는 source shape·lookup observation이 없다. parent noun/span과 capture observations를 함께 봐야 한다. shaped view의 operator별 충분성은 JE3에 남기며 EntityArray를 만들지 않는다 |

**중복 판정:** `AssignedValue`와 `SymbolValue`가 첫 concrete RHS seam이고, `FunctionOperand`는 payload가 겹치지만 provenance 계약이 다르다. `ExprKind`, `ParserNameBinding`, stack/control, binding observations는 역할이 달라 유지한다. JE0 자체는 JEntity API를 구현한 단계가 아니다. JE1의 완료 상태와 JE2–JE6의 남은 경계는 아래 항목에서 관리한다.

**근거 구분:** 새 감사 revision `0db94e768a845e2583c01d00538c3d16379677bb`의 `p.c` L87–96은 주석상 tacit translator용 `cases[]`다. runtime `p.c`의 ptcol dispatch와 L1006–1043의 row 7은 이미 stacked된 CAVN RHS를 대입하고 그 값을 stack에 남긴다. `pv.c::jtvis` L158은 translator action이므로 runtime 대입 oracle로 사용하지 않는다. `sc.c::jtnamerefacv` L364–397은 noun value와 expected-POS function nameref를 구분한다. `cf.c` L292–308의 `{0,NOUN}`은 construction result가 항상 Function이라는 가정을 반박한다. `cg.c` L101–121의 BOX carrier는 source rank/shape를 가진 내부 realization이며 semantic EntityArray가 아니다. 새/기존 source의 선언형 row predicate와 constructor disposition은 native Windows Python으로 동일함을 확인하고 5개 파일 hash를 `reports/entity-carrier-source-audit.json`에 기록했다. 이것은 runtime ptcol trace 또는 새 revision DLL 검증이 아니다.

**이 감사에서 수정한 compatibility bug:** row 7이 모든 modifier를 `resolve_modifier`로 적용용 해석한 뒤 대입하여 `copy=:explicit_adverb` / `copy=:explicit_conjunction`을 거부했다. 이제 stacked RHS를 그대로 대입한다. nameless modifier는 기존 lookup 때 by-value로 stack되고, nonnameless modifier는 POS-bearing NameRef를 유지한다. C `5!:1`에서도 explicit/derived modifier alias의 head가 원본 이름임을 확인했다. static prepare는 application semantics가 미지원인 function도 POS-known NameRef로 대입 구조를 보존한다. 본문·지역 invocation frame을 실행하지 않으며 alias 재대입과 실제 modifier application을 분리한다. explicit body 호출의 기존 미완료 상태는 그대로 추적한다.

**JE0 gate:** Rust regression 5개로 네 RHS 품사의 grouped assignment result/POS·commit identity, noun snapshot/function late lookup와 POS mismatch, 48-level Hook DAG의 Arc 공유 및 host 종료 후 lifetime, 65,536-atom noun snapshot의 payload 공유와 rebinding lifetime, explicit modifier alias의 static non-commit·runtime NameRef/POS/span·본문 미실행·재조회 및 POS 변경 오류를 검증했다. C corpus/stage에 **71건**을 추가했다: noun assignment 13, noun value 19, function class/atomic representation 33, J error 6. valid explicit adverb/conjunction alias 사례는 미지원 예외로 분류하지 않고 모두 C와 일치한다.

Windows default/portable 각각 **363 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture **4,810 cases / 4,806 passed / 기존 runtime 경계 4 / failed 0**; stage **9,990 checks**, words **6,618 cases**. capture graph 경계는 106으로 따로 기록하며 static 경계 2와 구분한다. 새 source 감사 pin `0db94e7...`, conformance source pin `13994ff...`, 실제 DLL release `ded7793...`는 서로 구분한다. 새 revision DLL·full upstream suite·private runtime trace·explicit body invocation acceptance를 검증했다고 주장하지 않는다.

**JE0의 handoff:** `AssignedValue` ↔ runtime assignment 한 seam을 JE1으로 넘겼다. semantic nested DD 등 기존 M2 미완료 항목은 유지한다. JE0/JE1을 full frontend rewrite나 첫 M4 CPU slice의 선행조건으로 확장하지 않는다.

Sources: [runtime p.c](https://github.com/jsoftware/jsource/blob/0db94e768a845e2583c01d00538c3d16379677bb/jsrc/p.c#L1006), [translator pv.c](https://github.com/jsoftware/jsource/blob/0db94e768a845e2583c01d00538c3d16379677bb/jsrc/pv.c#L158), [nameref sc.c](https://github.com/jsoftware/jsource/blob/0db94e768a845e2583c01d00538c3d16379677bb/jsrc/sc.c#L364), [constructor cf.c](https://github.com/jsoftware/jsource/blob/0db94e768a845e2583c01d00538c3d16379677bb/jsrc/cf.c#L292), [gerund cg.c](https://github.com/jsoftware/jsource/blob/0db94e768a845e2583c01d00538c3d16379677bb/jsrc/cg.c#L101).

##### JE1 — 최소 공통 `JEntity` identity 도입

- [x] `JEntity`/`JEntityRef`의 최소 API를 **boundary carrier**로 설계한다. 직접 semantic variants는 `Noun`과 `Function`으로 두고 Function이 `Verb|Adverb|Conjunction` POS를 소유하는 구조를 우선 검토한다.
- [x] `JEntity`를 `Value`/`FunctionEntity` 내부 representation을 통합하는 base class로 사용하지 않는다. 최초 적용 seam은 assignment/binding/parser result 또는 `FunctionOperand` 중 differential test가 가장 잘 갖춰진 한 곳으로 제한한다.
- [x] `JEntity::Function`이 `Arc<FunctionEntity>`만으로 충분한지, 현재 `Verb`/`VerbTarget` wrapper에서 semantic하게 남겨야 할 것이 있는지 먼저 결정한다.
- [x] lexical NAME, unresolved reference, binding/version/provenance를 `JEntity` variant와 분리한 reference/control API로 설계한다.
- [x] `Verb`/`Adverb`/`Conjunction`을 별도 payload 복제로 만들지 않고 shared `FunctionEntity` + `FunctionPartOfSpeech` identity를 재사용한다.
- [x] noun payload는 logical J noun identity를 가리키며 physical buffer/layout/device를 소유하지 않게 한다.
- [x] source span/provenance와 binding/version은 entity payload 자체와 필요한 observation/binding metadata를 구분한다.
- [x] large derived function/train이 `JEntity` conversion에서 deep-copy되지 않는 sharing test를 추가한다.
- [x] noun/verb/adverb/conjunction round-trip 및 POS mismatch/error semantics regression을 추가한다.

**JE1 완료 조건:** parser/binding API가 noun과 function을 공통 entity handle로 전달할 수 있으면서 기존 `FunctionEntity` DAG와 J noun semantic identity를 훼손하지 않는다.

##### JE1 구현 — 대입 경계의 최소 JEntity (2026-10-04)

아래는 JE1 도입 시점의 snapshot이다. 임시 SymbolValue/Verb adapter의 제거와 현재 namespace carrier는 이어지는 JE2 구현 기록에서 관리한다.

- `semantic::JEntity::{Noun(Value), Function(Arc<FunctionEntity>)}`와 borrowed `JEntityRef::{Noun(&Value), Function(&FunctionEntity)}`를 도입했다. owning carrier를 이동하면 기존 payload를 그대로 넘기며 `as_ref()`는 copy·allocation·refcount update 없이 inspect한다. owning `JEntity`에는 자동 Clone을 제공하지 않는다: 아직 Owned Value의 clone은 전체 noun payload를 복사할 수 있기 때문이다. borrowed view의 Copy/Clone은 참조만 복사한다.
- `RuntimeParserHost::assign(name, JEntity) -> Result<JEntity>`가 row 7의 Noun/Verb/Adverb/Conjunction을 같은 boundary로 전달한다. 중복 `AssignedValue` enum을 제거했다. 함수의 실제 POS는 FunctionEntity가 소유하며 separate Verb/Modifier payload를 entity에 추가하지 않았다.
- parser는 verb의 occurrence span/compatibility target, noun의 Expr height, source/provenance·occurrence·assignment flags를 carrier 바깥에 보존한다. capture의 Commit source/class/function identity와 binding version도 기존 경로에 남는다. noun assignment는 기존 host에서 freeze하고 반환·symbol payload를 공유하며 replacement/pool retirement 규칙을 유지한다.
- runtime `SymbolValue`는 첫 migration의 compatibility adapter로 남는다. `Verb::from_entity`는 Verb POS를 확인하고 shared entity head에서 기존 Primitive/Named/Derived target과 intrinsic span을 복원한다. 이 adapter는 function DAG·DefinitionCode를 복사하거나 NameRef를 현재 함수 값으로 fix하지 않는다. parser reinsertion은 원래 occurrence wrapper의 span/target을 그대로 재사용한다.
- lexical NAME, abstract noun/POS observations, `ParserNameBinding`, `ExprKind`, `FunctionOperand`, symbol table 및 gerund auxiliary는 이번 seam에 포함하지 않았다. JEntity에 BufferId·stride·layout·target·device·schedule API를 추가하지 않는다. `Value`가 현재 CPU payload를 포함하는 migration artifact는 여전히 남으며 이를 logical/physical 분리 완료로 간주하지 않는다. Function에 noun shape/rank를 추가하지 않았다.

검증은 JE0의 네 품사 grouped/chain assignment·noun snapshot·function late lookup·explicit modifier alias·POS 오류·effect/provenance 비교 71건을 같은 production boundary에서 재사용한다. 기존 48-level Hook DAG test에 JEntity move/borrow round-trip과 refcount 불변을 추가했고, 새 tests는 65,536-atom Owned noun move/borrow 시 payload pointer 유지, explicit Verb/Adverb/Conjunction의 POS·shared DefinitionCode 유지, runtime Verb adapter의 identity/span·modifier POS 거부를 검증한다.

**JE1 gate:** Windows default/portable 각각 **366 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture **4,810 cases / 4,806 passed / 기존 runtime 경계 4 / failed 0**, stage **9,990 checks**, words **6,618 cases**. 새 언어 form을 추가한 변경이 아니므로 JE0의 71건과 전체 기존 corpus를 그대로 재검증했으며 report 10개의 binary/source hash를 확인했다. capture graph 경계 106과 static 경계 2는 별도이고, full upstream suite·definition invocation acceptance·새 audit revision DLL·private C trace 동등성은 미검증이다. conformance source pin은 `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`, 실제 reference DLL release는 `ded7793fe5795d79eda8e7138dce94aa056edf78`; JE0 source 감사 `0db94e768a845e2583c01d00538c3d16379677bb`와 구분한다.

JE1의 최소 API/첫 boundary는 완료이며 runtime `SymbolValue` seam을 JE2로 넘겼다. stack의 deferred noun/application structure와 lookup observations를 concrete JEntity로 강제하지 않는다. explicit body invocation·scope, semantic nested DD, JE3+ higher-order views, broader storage migration, full J conformance는 계속 별도 미완료다. optimization·CUDA·GitHub CI는 보류한다.

##### JE2 — parser/binding/assignment 경계 수렴

- [x] `FunctionOperand::as_entity_ref()`로 공통 borrowed JEntityRef를 제공하고 `span()`으로 provenance를 보존한다. noun의 별도 span과 함수 Arc 소유권을 유지하기 위해 owning enum은 유지한다.
- [ ] parser stack/value model이 noun/function에 대해 공통 entity transport를 사용하되 jsource 9-row class/POS 규칙은 그대로 유지하게 한다. runtime rows 0–2와 row 7의 completed-result transport는 아래 단계에서 완료했고 전체 stack variant 수렴은 별도다.
- [x] `CompletedParseResult`로 실행 완료 noun과 row 7의 네 RHS class를 JEntity 경계에 연결한다. deferred Expr·NAME·control은 concrete entity로 강제하지 않는다.
- [x] modifier train의 Noun/Verb/Adverb/Conjunction operand를 같은 completed-result 경계에서 이동하고 noun source span·freeze 정책·함수 DAG identity를 보존한다.
- [x] rank/@: conjunction operand를 completed-result 경계로 연결하고 right-before-left 검사·gerund quiet fallback·원본 Expr span을 유지한다.
- [x] noun-left fork의 constant operand와 지원 explicit/direct definition의 mode/body·생성 결과를 completed-result 경계로 연결한다. definition invocation은 별도 미완료다.
- [x] 생성/대입 경계 이행 후 남은 adapter 책임을 재감사하고 capture의 중복 함수 identity 조회를 borrowed helper로 통합한다.
- [x] assignment가 `JEntity`를 namespace에 write하고 같은 assigned `JEntity`를 expression result로 반환하는 contract를 공통화한다. `Binding.value`와 runtime host boundary를 JEntity로 연결하고 SymbolValue를 제거했다.
- [ ] name lookup이 binding에서 `JEntity`를 얻은 뒤 expected POS 검사를 수행하고 late-binding/version semantics를 유지하게 한다. top-level runtime lookup과 verb/modifier POS 검사는 완료했으며 전체 local/locale/definition scope는 미완료다.
- [x] jsource `jtnamerefacv`의 의미적 차이를 회귀로 고정한다: noun name은 lookup 시점 value/snapshot을 전달할 수 있지만 function name은 실행 시 재조회되는 nameref가 필요할 수 있다. JEntity binding에서도 기존 71건과 새 noun/function replacement 11건으로 timing·POS·binding semantics를 유지한다.
- [ ] explicit/direct definition constructor와 invocation 결과가 같은 entity boundary를 사용하게 한다.
- [ ] parser/runtime/static path가 서로 다른 entity wrapper를 만들지 않는지 differential/golden으로 확인한다.

**JE2 완료 조건:** noun과 function의 parser/binding/assignment transport가 하나의 semantic abstraction으로 수렴하고, POS·lookup timing·effect ordering은 jsource-compatible하게 유지된다.

##### JE2 구현 — namespace와 runtime result의 JEntity 수렴 (2026-10-04, partial JE2)

- runtime `SymbolValue`를 제거하고 `Binding { value: JEntity, version: NameVersion }`으로 저장한다. 함수 binding에는 `Arc<FunctionEntity>`만 남으며 별도 Verb wrapper/span/target 복제가 없다. runtime final result도 동일 JEntity를 사용한다. `ExprKind`는 noun application/dependency 구조를 보존하는 parser representation으로 남는다.
- `commit_binding(name, JEntity) -> Result<JEntity>`가 version 증가 가능성을 먼저 검사하고, noun을 한 번 freeze한 뒤 namespace 저장/대입 반환 payload를 공유한다. 함수는 동일 DAG를 share한다. host assign은 이 함수에 위임하며 기존 noun replacement·OutputPool retirement 경로를 유지한다. automatic JEntity Clone을 추가하지 않고 freeze된 Value/Function Arc만 명시적으로 공유한다.
- noun lookup은 기존 값 snapshot을 내고 function lookup은 DAG의 result_pos를 전달한다. by-value nameless modifier, POS-known static alias, modifier resolution의 late NameRef chain·versions를 유지한다. 과거 enum variant가 암묵적으로 검사하던 Verb/Modifier class는 각 조회 경로의 명시적인 POS 검사로 대체했다. verb NameRef가 modifier로 바뀌면 같은 domain error/current_name을 보존한다.
- 더 이상 namespace에 Verb adapter가 필요하지 않아 `Verb::from_entity`와 그 전용 unit test를 제거했다. 보장하던 identity/span·POS 거부는 실제 commit/lookup 경로의 tests로 이행했다. parser occurrence wrapper와 `VerbTarget`은 아직 유지하며 namespace의 intrinsic Function identity와 분리한다. storage representation·Value 내부·gerund/view·optimizer/target policy는 바꾸지 않았다.

새 runtime unit tests 3개는 65,536-atom noun의 stored/returned pointer와 네 RHS class의 shared identity·versions, version overflow의 noun/function replacement 거부·기존 binding/pool/commit 보존, unified Function의 expected-POS/domain/current_name을 검증한다. integration regression은 cache limit 0/4096에서 noun→Verb→Adverb→Conjunction 교체 중 noun alias 생존과 마지막 alias 해제 뒤 bounded retirement·noun 재대입을 검증한다. 기존 adapter test 한 개를 제거하고 이 실제 경로 검사로 대체했다. C corpus/stage에는 noun/function replacement **11건**을 추가하여 entity boundary fixture는 **82건**이다.

**Namespace seam gate:** Windows default/portable 각각 **369 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture **4,821 cases / 4,817 passed / 기존 runtime 경계 4 / failed 0**, stage **10,001 checks**, words **6,618 cases**. capture graph 경계 108과 static 경계 2는 별도이며 full upstream suite·definition invocation acceptance·private C runtime trace 동등성은 미검증이다. report 10개의 실제 binary/source hash를 확인한다. conformance source pin은 `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`, DLL release는 `ded7793fe5795d79eda8e7138dce94aa056edf78`; JE0 source 감사 revision `0db94e768a845e2583c01d00538c3d16379677bb`는 새 DLL 검증으로 취급하지 않는다.

**남은 JE2:** parser completed-result transport와 deferred application 구조의 경계, 전체 local/locale/definition scope, explicit body invocation 및 static/runtime/capture 경로의 더 넓은 수렴을 검증한다. 이번 namespace seam 완료를 JE2 전체 완료로 표시하지 않는다. FunctionOperand view는 아래 단계에서 완료했으며 semantic nested DD와 기존 M2 gaps도 계속 추적한다. JE3+ collection·broader storage migration·optimization·CUDA·GitHub CI는 보류한다.

##### JE2 구현 — provenance를 보존하는 borrowed operand view (2026-10-04, partial JE2)

`FunctionOperand::as_entity_ref()`는 noun과 모든 함수 POS를 공통 `JEntityRef`로 조회한다. `span()`은 noun operand의 저장된 source span 또는 함수 identity의 span을 빌려준다. 이후 application occurrence span과 합치지 않는다. 조회에서 Value 복사·Arc 증가·새 entity 할당이 없고, owning enum은 noun provenance와 함수 DAG 소유권을 보존하기 위해 유지한다. gerund collection이나 물리 배열 representation으로 확대하지 않는다.

실제 사용 경계는 nameless modifier의 by-value lookup 판정과 semantic binding의 function NameRef DAG 순회다. 기존 순회 순서·POS/lookup 정책은 유지한다. operand를 실행용 parser item으로 만드는 경계는 공유 Arc 소유권이 필요하므로 기존 materialization 경로를 유지한다.

회귀는 owned 65,536-atom noun의 pointer/span 보존을 추가하고, 기존 host 종료 후 noun snapshot·48단계 shared DAG·explicit Verb/Adverb/Conjunction tests를 공통 view로 확장하여 payload identity, 함수/DefinitionCode 참조 수 및 원본 위치를 확인한다. 새 J 문법은 추가하지 않아 기존 82건 entity-boundary C fixtures를 그대로 사용한다.

**Operand seam gate:** Windows default/portable 각각 **370 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture **4,821 cases / 4,817 passed / 기존 runtime 경계 4 / failed 0**, stage **10,001 checks**, words **6,618 cases / failed 0**. capture graph 경계 108과 static 경계 2는 별도다. report 10개의 실제 binary/source hash를 확인했다. source/DLL pin은 위 namespace gate와 같으며 full upstream suite·definition invocation acceptance·private C runtime trace 동등성은 미검증이다.

completed parser result와 deferred noun/application 구조를 구분한 공통 transport는 아래 단계에서 진행한다. 전체 local/locale/definition scope와 explicit body invocation은 미완료다. JE2 전체 완료로 표시하지 않는다.

##### JE2 구현 — completed parser result의 공통 transport (2026-10-04, partial JE2)

`CompletedParseResult { entity: JEntity, span, height, verb_adapter }`는 완료된 RHS의 이동 경계다. noun Value와 함수 Arc를 복사하지 않고 이동하며, source occurrence/height와 Verb의 span/target adapter는 immutable FunctionEntity identity 밖에 둔다. `from_item`은 literal 또는 grouped literal noun만 받아들이고, 미계산 call·ReadName은 기존 unsupported 경계를 유지한다. 이 helper는 계산하거나 이름을 재조회하지 않는다. JEntity에 Clone을 추가하지 않는다.

runtime rows 0–2는 기존 host.apply를 한 번 실행한 뒤 이 경계로 completed noun을 stack에 반환한다. host가 없는 analysis 경로는 기존 Expr 연산 구조를 보존한다. row 7은 같은 경계로 RHS를 host.assign에 전달하고 같은 반환 payload로 parser item을 복원한다. commit capture·provenance inheritance·POS·lookup timing·effect order는 기존 reduction pipeline이 처리한다. ParseValue의 Expr/Verb/Function/NAME/control variants, constructor 경로, final Program 구조 전체를 바꾸지는 않는다.

근거는 [jsource runtime p.c row 7](https://gi
... 574753 bytes omitted ...
�고 `ap.c/ar.c/va2.c` 변경을 포함한다. 두 revision의 `jtmovavg`, `jtmovfslash`, `jtmean`, `jtreduce`, `jtfslashatg`, `jtsumattymes1`의 **검토한 진입·핵심 guard 발췌는 동일**했다. 이는 함수 전체, 모든 동작 경로 또는 jsource 전체에 변화가 없다는 증명이 아니다.

##### G. 이후 optimizer 구현 시 필요한 차등 검증 항목 — 아직 미실행

| 대상 후보 | 반드시 비교할 경계 | 증명/실행 확인점 |
|---|---|---|
| `f/@:g` | 빈 배열·sparse, in-place 우세, result dtype 불일치, overflow reversion | source/result 동일, error order, effect 이전 fallback, 불법 재실행 없음 |
| `+/%#` 및 Window(Mean) | 길이 0/1/다중 cell, 정수·실수·NaN·overflow, window 길이 0/1/초과 | cell 길이·prototype·dtype·숫자 동등성 |
| `+/@:*"1 1` | mixed rank, empty/sparse, `!.0`·`!.1`, QP | rank/agreement, dtype/result shape, fallback |
| BOXATOP/WILLOPEN/USESITEMCOUNT | 중첩 박스·다양한 cell shape·raze·sparse·virtual alias | Result Assembly와 재귀화·usecount·검사 순서 |
| Search/Under/View | boxed rank/tolerance, prehash reuse, 이름 재바인딩, shared/in-place ravel | 검색 결과, inverse 시점, alias/pristinity |
| 모든 rewrite | source graph 버전, proof witness, effect/error dependency, guard miss | Unknown이면 commit 금지; side effect 이후 replay 금지 |

**결론:** jsource 소스는 optimization 후보를 발견하는 근거이지 RustJ rewrite의 correctness proof나 측정된 속도 향상의 증거가 아니다. 검증하지 않은 후보는 candidate로 남기며 선택된 transform으로 표시하지 않는다.

##### H. 추가 전역 조사에서 발견한 누락 계열 (2026-10-06)

기존 A절의 18개 항목은 jsource 전체를 대표하지 않는다. jsource 고정 revision의 `jsrc/` 파일 목록을 재확인하고, **기존 목록에 없거나 한 행에 과도하게 뭉쳐 있었던** 아래 계열을 별도 감사했다. 문서에 추가하는 것은 **설계 입력**이며 최적화 구현·성능 검증 완료를 뜻하지 않는다.

| 추가 계열 | 확인한 jsource 원본 | 실제 최적화와 필요한 적용 조건 | RustJ 책임 위치 |
|---|---|---|---|
| **Key / Group-by와 집계의 결합** | [`ao.c::jtkeyct` 분류·small-range 처리](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ao.c#L213-L260), [`jtsldot` 특수화](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ao.c#L824-L850) | `u/.`와 `f//.`에 대해 `+/`, 최소·최대, mean 등 허용된 그룹 집계를 일반적인 `reorder → Cut → Reduce` 경로와 달리 직접 그룹 결과에 누적한다. grouping의 **키 동등성·tolerance·그룹 순서·dtype·overflow** 및 지원 연산 종류가 제한이다. | **Graph Key/GroupReduce 후보 → typed group algorithm & storage planning** |
| **일반 inner product / matrix multiply 알고리즘 선택** | [`cip.c::jtdot/jtpdt`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cip.c#L715-L739), [`cip.c` small/cached/BLAS](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cip.c#L925-L956), [`gemm.c` 외부 BLAS/내장 경로](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/gemm.c#L923-L985) | 기존 문서의 **rank-1 dot-like** 특수 함수 `jtsumattymes1`보다 넓은 별도 계열이다. `+/ . *`에 대해 크기·type별 직접 계산, cache-aware multiply, dgemm/zgemm/igemm 경로 및 numeric fallback을 구분한다. **NaN·0×무한대·정수 promotion/overflow·빈 축** 검사 필수. | **Graph Contraction identity → Execution lowering / target-aware GEMM choice** |
| **Grade / Sort / Ranking의 값 범위·크기 기반 전환** | [`vg.c` quick/radix/small-range/merge 분기](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vg.c#L525-L557), [`vgsort.c` direct sort](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vgsort.c#L120-L149), [`vgranking.c` histogram+prefix](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vgranking.c#L34-L69) | Grade를 무조건 sort로 내리지 않는다. 값의 범위/키 길이/정렬 axis/규모에 따라 counting-histogram+prefix, radix, qsort, merge, direct item sort를 선택하고 ranking `/:@/:`는 별도 small-range 경로가 있다. `vgsortiqavx512.c`는 특정 CPU 빌드 조건. **안정적 동률 처리·J 비교 규칙**을 증명한다. | **Graph Grade/Rank idiom → order-contract + physical algorithm selection** |
| **tolerance-aware hash 및 테이블 전략** | [`viavx2.c` 인접 hash interval 확인](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx2.c#L10-L49), [`viavx.c` bit/packed table allocator](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx.c#L15-L43) | 기존 `vi.c`의 prehash/sort 항목을 확장한다. 근사 동등성을 단순 exact-key hash로 대체할 수 없어 인접 interval 및 signed-zero 등 J tolerance 계약을 반영한다. table layout/packed bits/AVX 최적화는 target 종속적이다. | **Search semantic witness + tolerance facts → Hash execution planner** |
| **Interval Index `I.`의 별도 알고리즘** | [`viix.c` boolean·small-range 경로](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viix.c#L18-L48), [branch-sensitive/branchless binary search](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viix.c#L55-L86) | 일반 `i.` 해시 검색과 다른 sorted-interval search다. 입력 ordering, type, range에 따라 lookup table 또는 binary-search 선택. 일부 AVX512 경로는 `C_FSGSBASE` 등의 빌드 가드를 사용한다. | **Graph IntervalLookup identity → search algorithm selection** |
| **Cut/Substring의 virtual slice 및 boxer 소비자 협력** | [`cc.c::jtrightcut0`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cc.c#L133-L147), [`cc.c::jtboxcut0` copy/virtual](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cc.c#L158-L210) | 일반적인 Cut을 무조건 모든 셀 복사/boxing으로 내리지 않는다. 제한된 1차원 segment에서 virtual result를 만들거나 다음 소비자가 열 것을 아는 경우 boxed substring의 복사를 피한다. 음수/역방향 범위, rank, lifetime, error/fallback 조건을 보존한다. | **Graph Cut/segment provenance + demand → View/Result Assembly lowering** |
| **Gather/From의 copy-versus-view와 SIMD gather** | [`vfrom.c::jtget1cell`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vfrom.c#L39-L53), [AVX2 gather 내부 경로](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vfrom.c#L70-L108) | 단일 cell은 크기가 작으면 복사, 크면 virtual block 선택 가능. 여러 index의 비연속 추출은 실제 gather/copy가 필요하며 모든 `{`를 view로 환원할 수 없다. index bounds와 결과 순서·alias를 보존한다. | **Graph Gather access facts → Physical copy/view/vector gather** |
| **Reshape/Compress/Catenate의 소유권·복사 비용 최적화** | [`vf.c` reshape inplace/virtual](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vf.c#L301-L331), [`vrep.c` compress inplace](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vrep.c#L53-L93), [`vcat.c` boxed ownership transfer](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vcat.c#L192-L221) | 기존 `jtravel` 항목만으로는 부족했다. result가 입력과 물리적으로 호환될 때 header/shape 변경, 일부 복사 회피, abandonment·pristinity·recursive contents의 소유권 이전 등 서로 다른 경로를 갖는다. alias, usecount, fill 및 identity 조건 검증. | **Graph layout/materialization facts → Buffer planner/ownership lowering** |
| **Sparse 자료형 전용 알고리즘 분기** | [`cpdtsp.c` sparse inner product](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cpdtsp.c#L1-L43), [`vgsp.c` sparse grade](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vgsp.c#L1-L25), [`visp.c` sparse index-of](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/visp.c#L1-L25), [`vfromsp.c` sparse From](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vfromsp.c#L1-L35) | sparse는 “dense 그래프의 느린 대체 경로”가 아니라 전용 표현과 계산 전략을 갖는다. sparse element/axis/shape, fill, 형 변환의 J-visible 의미가 다르므로 generic fusion legality를 가정하지 않는다. | **Sparse semantic facts/route eligibility → sparse-specific execution** |
| **동적 이름 참조와 locale lookup의 캐싱** | [`sc.c` cached lookup와 invalidation](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sc.c#L57-L100), [short/long-term cache와 locale 조건](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/sc.c#L140-L178) | 자료흐름 rewrite가 아닌 **동적 J binding 해석의 실행 비용 절감**. short-term timestamp/할당 invalidation, long-term cacheable name 및 permanent locale 조건, multi-thread 원자성 등 세부 제약이 있다. cache hit를 name lookup semantics의 변경으로 혼동하지 않는다. | **Name/Binding semantic boundary → Runtime lookup cache/guard** |

**별도 계열로 분리한 이유:** `GroupReduce`는 단순 Search가 아니고, full `+/ . *`는 rank-1 dot idiom보다 넓으며, `Sort/Ranking`은 단순 graph-pattern recognition을 넘어 선택 가능한 여러 알고리즘의 family다. 반면 virtual·inplace·hash table·SIMD 구현은 Graph semantic node를 새로 늘리는 이유가 아니라 downstream planner의 결정 자료다.

##### I. 범위 및 누락 여부에 대한 보수적 판정

- **기존 catalog에는 중요 계열의 누락이 있었다.** 위 H절에서 10개 family를 보강했다. 항목 수는 jsource 전체의 완전한 optimization inventory나 구현 우선순위를 의미하지 않는다.
- **소스 확인 범위:** pinned `jsrc/` 소스 트리의 직접 C/H 파일 목록을 먼저 확보하고, 기존 composition/rank/reduction/array 영역 밖의 grade·sorting·group/key·inner product/GEMM·index/hash·cut·shape/data movement·sparse·name cache의 진입점과 적용 guard를 대조했다.
- **남은 미조사 영역:** `jsrc/p.c` parser/assignment fast paths, `jsrc/cx.c` explicit-definition execution, `jsrc/va1.c/v0.c/v1.c/v2.c` 원시 산술·수치 dispatch, `jsrc/m.c/am.c` allocator/index amend, 기타 assembly/SIMD microkernel의 전수 목록은 이번에는 exhaustive하게 검증하지 않았다. “더 이상 빠진 기법이 없다”고 판정하지 않는다.
- **우선 적용 순서 제안:** (1) 고수준 `Key/GroupReduce`, `Contraction`, `Grade/Ranking`, `IntervalLookup` 후보·provenance를 표현할 수 있는지 확인; (2) J rank/tolerance/group ordering/numeric error witness 정의; (3) `Hash/Search`, sparse, view/gather, physical buffer, name cache 등 execution-route 별로 독립 구현·jsource 차등 시험. current frontend/optimizer gate를 건너뛰어 착수했다는 뜻은 아니다.
- **검증 경계:** 이 절은 GitHub 소스/설계 문서 검사다. native J 실행, RustJ differential test, CPU/GPU benchmark를 수행하지 않았으며 각 효익·동등성·fallback 적법성은 미검증이다.

##### J. RustJ 프레임워크에 맞춘 구현 연결 (2026-10-06)

이 절부터는 H–I의 source survey를 **현 RustJ 소스 코드 경계에 적용한 결과**를 기록한다. 전수 J semantics 증명이나 jsource 성능 재현을 의미하지 않는다.

| source-derived family / 아이디어 | 현재 RustJ 소유 경계 | 2026-10-06 코드 반영 | 실행 최적화 도입 전 남은 조건 |
|---|---|---|---|
| `f/@:g`의 Map→Reduce streaming | 기존 `src/j_graph_fusion.rs` `MapReduce` + `fusion_planning.rs` | **기존 분석을 재사용**, 중복 rewrite 미등록 | per-cell semantics, numeric/dtype, effects, target capability, fallback |
| `+/%#` mean fork 인식 | 신규 `src/j_graph_jsource.rs` source-pattern analyzer | `FunctionEntity`의 **단항 호출 ordinary Fork + Insert(Add) / Divide / Tally** 완전 일치 시 `MeanIdiom` *후보만 발견* | J rank/cell·empty·dtype·float order·error proof; 실제 `Mean` 실행 op 미구현 |
| reduction/window/scan 기법 | 기존 `GraphForm::Reduce/PrefixInfix`, `j_graph_scan.rs` + 신규 source opportunity | `ReductionFastPath`, `WindowAlgorithm` 후보 등록; 기존 Scan witness와 충돌하지 않음 | small-cell, moving aggregate, NaN/overflow, prefix vs infix proof 및 후속 알고리즘 |
| `i.` / `e.` / `E.` search | `GraphForm::Atomic`, `PrimitiveId`, 기존 `FindViaWindowMatch` rewrite | dyadic Search를 `SearchAlgorithm` source opportunity로 기록; `Find`의 기존 witnessed rewrite 유지 | tolerant equality, key type, prehash legality, algorithm cost |
| dyadic `I.` interval index | `PrimitiveId::Icap` | monadic indices와 혼동하지 않고 **dyad만 `IntervalLookup`** 후보 등록 | ordering·shape·type·tolerance, search route |
| dyadic `{` 및 static reindex | `GraphBasisKind::DynamicGather/StaticReindex`, physical buffer route | `GatherCopyOrView`, `ReindexCopyOrView` 후보 기록. **zero-copy를 단정하지 않음** | bounds, alias/ownership, result shape, fill, target stride/gather support |
| Key/GroupReduce, full `+/ . *`, Grade/Ranking | `J Graph`의 향후 source identity + Execution Semantic Lowering | source 파일·심볼·proof obligations만 **catalog 등록**, 가짜 applied graph node/실행 후보 생성 없음 | 해당 J constructor/typed semantic support, source graph canonicalization, jsource differential |
| tolerant hash, sparse, buffer reuse, name-ref cache | typed semantic facts / Execution/Physical planner / binding runtime | 발견 registry에 **`DownstreamOnly` 또는 `AwaitingFrontendOrFacts`** 표시; graph algebraic rewrite로 위장하지 않음 | tolerance/sparse/alias/locale-version, fallback 및 cost contracts |

**실제 코드 연결 및 불변 조건**

- `src/j_graph_jsource.rs`: pinned source evidence, family stable ID, 책임 계층, discovery coverage, proof requirement를 소유한다. `Plan::jsource_opportunities()`는 **원래 J Graph의 source id, span, basis, facts**를 보존한 sidecar만 산출하며 source graph를 변경하지 않는다.
- `src/compilation.rs`의 `CompilationAnalysis::jsource_opportunities`와 `src/runtime.rs::analyze_compilation_diagnostic`으로 분석 결과를 기존 rewrite/Logical IR과 **병렬 제공**한다. 최적화 실행 경로에 자동 연결하지 않는다.
- candidate의 legality는 `AwaitingSemanticProofs`, `selected=false`로 고정된다. `verify(&Plan)`는 source/provenance/evidence 상태를 재도출하여 변조·오래된 후보를 거부한다. 이 verifier는 **J 수치적 동등성 검증기가 아니다**.
- `MapReduceStreaming`은 이미 `j_graph_fusion`이 책임진다. 독립된 동일 후보를 자동 추가하지 않는다.
- `GroupAggregate`, `MatrixContraction`, `GradeRanking`의 source registry가 존재해도 해당 J 구문이 RustJ에서 파싱/실행 가능하다는 뜻은 아니다.
- `tests/j_graph_jsource.rs`는 registry 불변성, source provenance, 후보 변조 거부, mean fork exact-match/부정 사례, 일부 기본 graph 패턴, 기존 Find rewrite·MapReduce fusion의 분리를 검사하도록 추가했다. **실제 Rust 테스트 실행 결과는 아직 확인하지 못했다.**

**후속 구현 계획—현재 M2/frontend 단계 우선순위를 앞서지 않는다**

1. **source-coverage gate:** `Key (/.), Dot (.), Grade (/:, \:), Cut (;.), Under (&.)`의 정확한 품사·derived-verb operand·rank 계약이 구현되기 전에는 candidate를 임의 생성하지 않는다.
2. **witness gate:** source optimization마다 `Semantics + RankCellAssembly + DType/Numeric/!. + Sparse/Empty + Effect/Error + Alias/Fanout` proof를 작성한다. 실행 전에 guard miss는 source fallback이 있어야 하고 effect 이후 replay는 금지한다.
3. **algorithm basis gate:** `GroupReduce`, `Contraction`, `GradeSort`, `IntervalLookup`의 Execution Semantic operation/facts와 J-specific physical routes를 분리한다. J Graph에 `jtsort`, `jtfslashatg` 같은 C 함수 노드를 추가하지 않는다.
4. **target gate:** CPU vector/BLAS, sparse implementations, GPU library/custom-kernel 경로는 `LoweringRegistry`와 target/resource/cost에서 별도 선택한다. `Unknown`은 실패가 아니라 **보수적 미최적화 경로**다.
5. **regression gate:** jsource와 empty/sparse, boxed/tolerance, `!.0/!.1`, NaN/overflow, rank/frame, error/locale/cache-invalidating writes를 비교한다. differential과 성능 측정 없이 최적화 완료로 표시하지 않는다.

##### K. 후속 원본 재검토 — 단항 Mean guard와 런타임 최적화 경계 (2026-10-06)

**발견 및 수정:** 고정된 [`cf.c::jtfolk` 98행](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L93-L101)은 `(+/ % #)`에서 **`f1=jtmean`만** 지정한다. 이항 executor `f2`를 Mean으로 바꾸지 않는다. 이전 `src/j_graph_jsource.rs`는 fork의 구성만 확인하고 적용 valence를 보지 않아 이항 호출에도 `MeanIdiom`을 만들 수 있었다. 이제 **`NodeKind::Apply`가 `Valence::Monad`인 경우에만** mean 후보를 등록한다. `tests/j_graph_jsource.rs`에는 동일한 fork의 이항 호출에 대한 부정 회귀 사례를 추가했다. 이것은 후보 발견의 정확도 수정이며 mean 수치 동등성·kernel 구현의 증명이 아니다.

**이전 미조사 영역에 대한 2차 소스 감사:** 다음은 새 Graph rewrite를 무조건 등록하는 근거가 아니라 현재 계층에 대한 추가 설계 입력이다.

| jsource 추가 확인 | 확인 위치 | RustJ 해석 및 소유 계층 |
|---|---|---|
| parser 행 0–2의 inplace 실행, assignment `zombieval`, 조기 parse 종료 | [`p.c` 10–24행](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/p.c#L10-L24) | **Frontend/Runtime의 이름·대입 시점과 lifetime 관리**. Graph liveness만으로 assignment buffer donation을 선언하지 않는다. |
| explicit-definition local symbol table 재사용/복제, x/y precomputed bucket, 버려진 인자의 usecount 특수화 | [`cx.c` 270–329행](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cx.c#L270-L329) | **Explicit runtime/binding implementation**. 동적 scope·재진입·인자 alias 증명 전에는 Graph rewrite가 아니다. |
| 단항 numeric type-dispatch와 실패 유형별 정밀도 변경·재실행, sparse의 별도 fallback | [`va1.c` 313–383행](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/va1.c#L313-L383) | **Typed semantic error/promotion witness → Execution numeric route**. 단항 함수를 모두 무오류 SIMD kernel로 취급할 수 없다. retry는 effect/order 안전성 검증을 요구한다. |
| Amend/scatter의 index·sparse·dtype·read-only·usecount 조건부 inplace | [`am.c` 55–89행](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/am.c#L55-L89) · [568–581행](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/am.c#L568-L581) | **Amend semantic access/error contract → guarded Scatter/Buffer planner**. 단순 Gather/View 규칙과 구분하며 alias 및 transactional assignment를 보존한다. |
| recursive/virtual block과 refcount·allocation lifetime | [`m.c` 743–783행](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/m.c#L743-L783) | **Physical allocator/representation 관리**. C refcount flag를 semantic Graph property로 복제하지 않는다. |

**적용 판정:** 단항 Mean의 오검출은 즉시 수정한다. 나머지 다섯 경로는 기존 frontend/runtime, semantic numerical contract, Scatter/Buffer planner의 source-evidence backlog로 둔다. 새로운 J Graph node/실행 최적화가 구현된 것으로 표시하지 않는다. `FOUNDATIONS.ko.md`의 semantic/physical 경계 원칙과 모순되지 않아 해당 파일은 변경하지 않았다. **이번 변경은 pinned C source와 Rust 코드의 정적 검토이며, Rust 테스트·jsource 차등 실행·benchmark는 아직 실행하지 않았다.**

##### L. jsource 후보 → A3 Logical IR → 기존 LoweringRegistry 연결 (2026-10-06)

**이번 이행 단위:** `src/lowering.rs::LoweringRegistry::jsource_planning_reports`는 각 `JsourceOpportunity`의 출처를 `candidate.verify(&j_graph)`로 다시 확인한 뒤, `Operation.j_origin`으로 연결된 **canonical A3 호출**을 찾는다. `OpKind::Basis`는 **이미 등록된 일반 execution-basis lowering**에 `legal_candidates`로 질의하고, `OpKind::SemanticCall`은 기존 semantic-call 경계로 표시한다. J Graph/A3의 source 및 node-count 정합성과 양쪽 verifier 검사를 선행한다.

| 새 보고 내용 | 소유권 및 검증 경계 |
|---|---|
| `JsourcePlanningReport`의 family/source ValueId/span/decision owner | 기존 `JsourceFamilyRule` 및 J Graph provenance 유지 |
| `JsourceLinkedCall`의 A3 `OpId` 및 현재 대상의 기본 route | **기존 코드 경로 조사**일 뿐 해당 jsource 기법의 특수화 지원을 뜻하지 않음 |
| `unresolved_proofs` | `ProofRequirement` 전체가 **미충족**. type/shape 또는 기존 CPU reference route만으로 하나도 자동 면제하지 않음 |
| `NeedsLogicalCallLink` | canonical A3에서 대응 호출이 없는 후보. 실행 연결을 생성하지 않음 |
| `NeedsSemanticProof` | 대응 호출을 찾아도 equivalence/ordering/guard/fallback/cost가 증명되지 않으면 **최적화 실행 불허** |

이 연결을 통한 **조건부 lowering의 첫 gate**는 `source candidate validation → canonical logical call mapping → existing route legality`다. 단, 이를 *optimized implementation 선택*과 혼동하면 안 된다. **현재 추가한 결과에는 jsource-specialized kernel 선택, guard 삽입, rewrite commit, physical buffer 결정이 없다.** 실제 specialized lowering은 J-visible numerical/rank/empty/error/binding/alias 증명, 실패 전 source fallback, target-resource-cost 확인을 별도 통과한 뒤에만 후속 구현한다. 순서는 `ProofRequirement`별 witness/guard 도입 → 해당 family의 parameterized lowering recipe → guarded A3/Physical 선택 → C differential/benchmark다.

`tests/lowering.rs`에 Reduce, 단항 Mean, Gather, IntervalLookup의 J Graph→A3 origin 연결, CPU/GPU 기존 route와 미증명 분리, 오래된/조작 후보 및 누락된 logical origin을 검증하는 **정적 회귀 테스트를 추가했다. 테스트를 실행했다는 뜻은 아니다.** CI·Cargo·jsource differential·benchmark는 이 변경에서 실행하지 않았다.

<a id="jsource-audit-m"></a>

##### M. jsource 재감사: 기존 catalog 밖의 구조적·수치적·상태적 최적화 (2026-10-06)

**감사 방법과 범위.** 같은 [jsource 고정 revision](https://github.com/jsoftware/jsource/tree/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc)에서 (1) \`ca.c\`의 \`@/@:/&:\` 조합 dispatch 및 \`cf.c\`의 hook/fork construction, (2) \`ao/cc/v/vi/vg/vrand/vx/vz/va1/vo/a.c\`의 실제 실행·reversion 경로, (3) RustJ의 \`j_graph_ir.rs\`, \`j_graph_jsource.rs\`, \`analysis.rs\`, \`logical_ir.rs\`, \`execution_semantics.rs\`, \`lowering.rs\` 경계를 **서로 다른 출발점으로 대조**했다. 앞선 A/H/K의 대표 최적화 목록에는 다음 구조·소유권이 개별 항목으로 **빠져 있거나 충분히 분리되지 않았다.** 원본 dispatch의 존재는 RustJ parser 지원, 의미론적 등가성 또는 성능 개선의 증명이 아니다.

| 추가 확인한 계열 | pinned jsource의 직접 근거 및 실제 제약 | RustJ 기존 framework에 넣을 위치 / 도입 판정 |
|---|---|---|
| **Oblique reduction + convolution** (\`f//.@:(g/)\`) | [\`ca.c\` 특수 dispatch](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ca.c#L367-L374) → [\`ao.c::jtpolymult\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ao.c#L109-L161). 두 atomic dyadic operand에 \`VFUSEDOK2\`가 필요하고 실제 구현은 dense·nonempty vector, dtype/operation 조합별 분기와 generic fallback, 정수 overflow 시 재계산을 갖는다. | **J Graph의 Oblique/Reduce composition 후보 → Execution basis의 Contract/Segment-Reduce 또는 명시적 oblique access map → target-specific convolution algorithm**. GEMM의 단순 별칭으로 등록하지 않음. **미구현.** |
| **Cut → Scan/Window → Raze의 중간 boxing 제거** | [\`ca.c\` 인식](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ca.c#L358-L368) · [\`cc.c::jtrazecut1/2\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cc.c#L983-L1031): 한정된 \`;.n\`, atomic scan, dense input 등만 사용. **원본 주석은 cut이 하나도 없을 때 특수 경로가 일반 경로와 결과 축이 다를 수 있음을 명시한다.** | **J Graph의 multi-region producer/consumer fusion 후보 → SegmentView + Scan/Window + ConcatAssemble → result assembly/materialization**. 무조건 법칙으로 사용 금지. **zero-cut 의미론·fallback을 먼저 J oracle로 확정할 때까지 보류.** |
| **8-bit 문자 치환 LUT** (\`y {~ x i. ]\`) | [\`cf.c\` fork 인식](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L59-L65) → [\`v.c::jtcharmap\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/v.c#L165-L191). byte LIT, 256-entry table, \`x=a.\` shortcut, 중복 매핑의 첫 일치 우선순위, index error의 generic fallback. | **J Graph의 IndexOf→Gather idiom → Lookup/Gather execution → Physical byte LUT**. alphabet/encoding·rank·invalid index·duplicate semantics 증명 전에는 일반 \`i.\`를 대체하지 않음. **미구현.** |
| **Boolean/sparse predicate → indices 직접 생성** (\`# i.@#\`) | [\`cf.c::jthkiota\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L221-L234): boolean atom/vector는 \`ifb\` 경로, sparse boolean 1D는 fill과 sparse axes를 검사한 후 저장된 nonzero index 활용. [hook의 Key 인접 특수화](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L329-L339)도 존재. | **Graph IndexSpace + Compact(mask)/GroupBy 후보 → Execution masked-index or sparse-index route**. 일반 \`i.@#\` 중간 배열 제거 여부는 span/valence·shape/empty/sparse fill 증명 후 결정. **미구현.** |
| **Grade→scalar Gather의 order-statistic 선택** (\`x {/:~ y\`, \`x {/: y\`) | [\`cf.c\` hook dispatch](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L337-L345) → [\`vg.c::jtordstat/jtordstati\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vg.c#L779-L800). scalar order index, int/float vector 및 길이 조건에 제한된다. **\`jtordstat\`은 내부 pivot을 위해 \`jtrollksub\`를 호출한다.** | **Graph Grade→Gather idiom → order-statistic semantic contract → Physical select/quickselect**. 동률·정렬·index/error뿐 아니라 **난수 상태의 관찰 가능성**을 독립 확인해야 한다. generic Grade sort를 무조건 선택 연산으로 바꾸지 않음. **미구현.** |
| **Shape와 RNG 생성의 결합** (\`?@#\`, \`?@$ \`, \`?.@#\`) | [\`ca.c\` 선택](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ca.c#L353-L359) → [\`vrand.c::jtrollksub/jtrollk\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vrand.c#L609-L678). boolean·power-of-two·일반 range에서 난수 생성 알고리즘과 RNG draw 경로가 달라지고, \`jtrollk\`는 rank/type에 따라 generic으로 복귀한다. | **J Graph의 Generate/shape-demand 후보 → stateful RNG runtime/Execution Generate route**. 효과·난수 스트림·seed/engine/version 경계가 우선이며, pure fusion·일반 reassociation에 합류시키지 않음. **미구현.** |
| **Box + Append/Raze의 direct link** (\`,<\`, \`;<\`, \`,&<\`) | [\`cf.c\` hook 인식](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L315-L326), [\`vo.c::jtjlink\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vo.c#L107-L144). 특정 box 형식·recursive/virtual 보존·\`WILLOPEN\`·in-place, self-containment 회피와 fallback을 명시한다. | **Graph box/append demand → ConcatAssemble/boxed result assembly → ownership/materialization planner**. \`Box→Open\` 일반 소거로 해석하지 않음. **미구현.** |
| **명시적 \`M.\` memo cache** | [\`a.c::jtmemo12/jtmemo\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/a.c#L93-L185). 0-rank 정수형으로 변환 가능한 인자에 대한 두 인자 key, lookup/insert lock, 공유된 결과를 소유하는 동적 table; 다른 인자는 매번 원 함수를 실행한다. | **J derived verb의 explicit memo semantics + Stateful cache runtime**, 필요시 guarded call-specialization. 임의 Graph CSE나 모든 noun을 key로 쓰는 자동 memo와 구별; cache hit 시 호출이 생략되는 **사용자 요청된 \`M.\` 동작**과 동적 이름·효과·수명을 보존. **미구현.** |
| **수치 상수·정확도별 별도 algorithm family** | [\`ca.c\` digits/\`^@o.\`/constant modular power](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ca.c#L367-L387); [\`vx.c::jtdigits10\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vx.c#L257-L290), [\`vz.c::jtexppi\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vz.c#L293-L305), [\`va2.c\` exponent 0.5·power-of-2 modulus](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/va2.c#L1937-L1940). \`jtexppi\`는 특정 complex half-turn에서 exact-zero component를 만들어 일반 경로와 다르게 처리한다. | **Graph bound constant/composed numeric idiom → semantic numeric/promotion/fit witness → typed CPU/GPU specialized routine**. 수학적 항등식만으로 bitwise/오류 순서 등가를 주장하지 않음; backend routine은 새 Graph semantic op가 아니다. **미구현.** |
| **Hook의 비교·threshold 직접 실행** | [\`cf.c\` \`(compare |)\` 및 \`compare L.\` 인식](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L319-L333), [\`cf.c\` \`jtdeadband\` dispatch](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cf.c#L105-L115), [\`va1.c::jtdeadband\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/va1.c#L440-L462). latter는 AVX2/EMU_AVX2 guard, float scalar threshold/dense-nonempty 조건 및 fallback을 가진다. | **Graph Hook-comparison candidate → typed semantic comparator/threshold contract → target-feature-gated SIMD**. tolerance, NaN, negative-zero, rank/effect 보존. **미구현.** |

**이미 H절에 있는 GroupAggregate/GEMM/Grade/Search/virtual/sparse와 중복 등록하지 않는다.** 이 표는 그 범주에 감춰졌던 **서로 다른 operator topology, legality, target/runtime mechanism**을 드러내는 출처 목록이다. 새로운 family ID를 \`JSOURCE_FAMILY_RULES\`에 자동으로 늘리거나 파싱되지 않는 J 구문을 조작해 applied Graph 후보를 생성하지 않는다. 소스 근거 등록 시에도 \`AwaitingFrontendOrFacts\` 또는 \`DownstreamOnly\` 상태로 시작한다.

###### M.1 RustJ 프레임워크 수정 필요성 판정

**판정: canonical compiler architecture의 전면 변경은 불필요하다. 하지만 아래 경계의 국소적 확장은 최적화 실행을 시작하기 전에 필요하다.**

| 기존 구현에서 확인한 상태 | 판정 | 최소 변경 또는 유지할 원칙 |
|---|---|---|
| \`FunctionEntity\`, \`J Graph IR\` region, \`A3 Logical IR\` 및 \`ExecutionBasisKind::{Contract, Scan, GroupBy, Grade, Gather, ConcatAssemble, ...}\`가 분리되어 있음 | **재설계 불필요** | 새로운 C 특수 진입점별 Graph/SSA node를 만들지 말고 고수준 J topology와 기존 basis를 재사용한다. \`SegmentView/GroupBy/Contract\`의 \`ExecutionBasisPayload::Deferred\` 세부 payload는 실제 semantic 지원과 함께 보강한다. |
| \`JsourceOpportunity\`는 source \`ValueId\`·span·basis/facts를 보유하고, \`jsource_planning_reports\`는 A3 \`Operation.j_origin == source_value\`를 연결함 | **작지만 필요한 확장** | Cut–Scan–Raze, convolution 같은 **다중 연산/중첩 region 후보**를 위한 sidecar \`SourceAnchor\`(해당 graph version, optional \`RegionId\`, 원래 source value, A3 ordered operation set)와 검증기 설계. 동일 \`ValueId\`를 공유한 중첩 region을 오인하지 말 것. parser \`FunctionEntity\` identity는 그대로 둔다. |
| \`JsourcePlanningState\`는 \`NeedsLogicalCallLink/NeedsSemanticProof\`까지만 구별하고 \`unresolved_proofs\`는 family 전체를 반환; \`OpportunityLegality\`는 단일 \`AwaitingSemanticProofs\` | **실행 이전에 필요** | 기존 §4.1.4의 **obligation별 \`ProofBundle\`/witness/guard/failed state**를 실행 가능한 후보에 구현한다. source graph/version, valence, rank, empty/prototype, \`!.\`, numerical error, target feasibility를 각각 증명한다. \`selected\` bool 또는 합법적 baseline route만으로 증명 완료를 추정하지 않는다. |
| \`EffectSummary\`는 현재 \`Pure/Unknown\` 두 값뿐이며, \`M.\`, \`?\`, \`jtordstat\`의 RNG/cache와 error-control은 별도 상태 관계가 필요함 | **지원 시 국소 확장 필요** | \`StateResource\`/ordered effect dependency에 RNG state·explicit memo table·binding invalidation을 연결하는 규약을 추가한다. 단지 RustJ 수치 함수로 보인다는 이유로 RNG 소모·cache hit 부작용을 지워서는 안 된다. **지원 전에는 Unknown/RuntimeSemantic fallback**. 모든 현재 OpKind를 memory/effect SSA로 즉시 변경할 필요는 없다. |
| \`Logical IR\`은 \`SemanticCheck\`와 constraint/error origin을, 후속 계층은 target/cost/resource를 분리함 | **계층 유지** | zero-cut 결과 축·빈 segment/fill, sparse boolean fill, byte LUT error, random draw ordering의 semantic guard를 **effect 전에** 검사한다. guard miss는 지정된 J semantic route로, effect 이후 재시작은 금지. vector width, byte table, cache block, buffer ownership은 Physical Planner에서만 선택한다. |

**특별한 반례 두 개를 구현 gate로 고정한다.**

1. [\`cc.c::jtrazecut2\` 주석](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cc.c#L983-L993)은 **zero-cut에서 최적 경로와 일반 경로의 결과 축이 달라질 수 있다**고 명시한다. 따라서 jsource의 특수 구현을 **equivalence oracle로 사용하면 안 된다**. J 언어에서 요구되는 실제 결과·fill/assembly 정책을 독립적으로 확정하고 두 경로의 동작을 차등 분석해야 한다. 미해결이면 결합 최적화는 불허한다.
2. [\`vg.c::jtordstat\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vg.c#L779-L790)은 내부 pivot 생성에 \`jtrollksub\`를 부른다. 해당 RNG 상태 변화가 이후 J 호출에 관찰되는지 **확인하기 전**에 "Grade→Select는 pure rewrite"라고 선언하지 않는다. 난수 state를 관찰 가능하다고 확정한 경우에는 trace/seed 동등성 또는 J 사양상 허용된 차이를 별도로 증명해야 한다.

###### M.2 이행 순서·검증 범위

1. **M2/front-end coverage 우선 유지:** \`/. /.., ;., M., ?/?.\` 및 hook/fork/grade 조합의 정확한 POS·derived construction/valence·fallback을 먼저 인식한다. 아직 미지원인 형태는 \`AwaitingFrontendOrFacts\`.
2. **원본 출처와 J semantics 분리:** source form/region anchor와 expected Logical operation set을 남기고, 효과·RNG·cache·error/check dependency 및 숫자/비어 있음/shape 증명 의무를 정의한다.
3. **한 family씩 proof·guard·fallback:** 우선 byte-LUT 또는 boolean-index처럼 경계가 명료한 경로를 대상으로 source reference route와 optimized recipe를 동시에 비교한다. zero-cut·RNG·memo는 effect/assembly 증명 전 보류한다.
4. **후속 실현:** \`LoweringRegistry\`에 evidence를 소유한 parameterized recipe를 등록한 뒤 target/resource/cost, candidate overlap, 선택과 Physical Planner/Executor를 순차 연결한다.
5. **검증:** 각 특수 경로의 positive/negative guard, type/rank/empty/sparse/boxed/\`!.\`/tolerance/NaN/overflow/error precedence, RNG seed/trace, named binding 및 replay safety를 J/C differential과 Rust tests로 확인한다. **이번 M절은 source/architecture 정적 감사다. Cargo/CI/jsource 실행·benchmark를 수행하지 않았고, 확인한 10개 계열의 RustJ 구현·의미 등가성·성능을 주장하지 않는다.**

**미조사 잔여 범위:** assembly/architecture-specific microkernels 전수, 모든 primitive dispatch·runtime allocator, feature flags별 빌드 차이 및 현재 master 전체에 대한 일대일 diff는 완료하지 않았다. 이번 감사 결과를 "jsource 최적화 누락 없음" 또는 full-J support로 해석하지 않는다.

<a id="jsource-index-family"></a>

##### N. Roger Hui의 Index-Of family와 RustJ 단계적 구현 (2026-10-06)

**연구·원본 근거.** Roger Hui, *Index-Of, A 30-Year Quest* (J Conference, 2014; [후대의 서지 기록](https://www.sigapl.org/Articles/APL%20Since%201978_3386319.pdf)) 및 *Hashing for Tolerant Index-Of* ([Jsoftware 논문](https://www.jsoftware.com/papers/Hashing.htm), 2010). RustJ 설계의 실행 근거는 별도로 [고정 jsource \`vi.c\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vi.c#L140-L185), [\`viavx.c\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx.c), [\`viavx2.c\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx2.c), [\`visp.c\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/visp.c)의 dispatch/실행/guard이다. 발표의 역사적 중요성과 소스의 구현 상태를 구분한다.

**의미론상 연산 family / 알고리즘 family 구별.** J의 \`x i. y\`(first), \`x i: y\`(last), \`x e. y\`(membership), \`~. y\`(nub), \`~: y\`(nub sieve), \`x -. y\`(less), \`I.@e.\`(indices), \`u/. y\`(Key classification) 등은 equality/search를 공유할 수 있지만 **output convention, representative selection, rank/cell/frame, empty/prototype, tolerance/fit**은 서로 다르다. \`I.\` **이항**은 sorted interval index로 별도 \`IntervalLookup\`; \`E.\`는 연속 부분배열 window match이고 \`FindViaWindowMatch\`가 기존 소유자다. \`E.\`를 일반 \`i.\` 해시 후보로 분류하지 않는다. \`i. 4\`·\`i: 4\`의 단항 Generate도 dyadic 검색과 다른 의미다.

| jsource의 공유 검색 경로 | 선택의 관건 | RustJ 프레임워크 소유 계층 |
|---|---|---|
| Sequential scan | 아주 작은 검색 문제에서 해시 구축비 회피; [\`vi.c\` 후보 판정](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vi.c#L1113-L1148) | Execution algorithm/초기 CPU reference |
| Dense direct indexing, boolean/byte, bit-packed 및 좁은 integer range | \`[min,max]\` 값 범위, index-vs-presence table 폭, 초기화 비용·캐시 한도; [\`vi.c\` small-range 선택](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vi.c#L1148-L1238) | Physical algorithm과 cost, logical membership/position 분리 |
| General hash (first/last), reverse hashing | 입력·질의 상대 크기, duplicate representative, 해시 방식 및 저장 용량; [\`viavx.c\` reverse 선택](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx.c#L738-L850) | Execution algorithm/Physical strategy |
| Tolerance-aware float/complex/boxed search | \`!.ct\`/runtime \`cct\`, **근사동등성의 비추이성**, +0/-0, NaN, 두 인접 bucket 후보·exact insert vs tolerant probe; [\`viavx2.c\` dual-bucket 구현](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx2.c#L8-L78) | Semantic equality witness **후** target-specific implementation. 단순 표준 \`HashMap<f64, ..>\` 불가 |
| Boxed sort→binary search fallback | 실제 jsource \`jtiobs\`는 \`ct=0\`과 제한된 boxed 구성만; [\`vi.c\` guard](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vi.c#L735-L840) | Target algorithm, stable representative/비교 관계 증명 |
| Sparse search 및 Key self-classification | sparse fill/axes/empty, grouping first-occurrence order; [\`visp.c\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/visp.c#L68-L106), [\`ao.c::jtkeyct\`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ao.c#L213-L255) | \`LookupClassify\` / \`GroupBy\` → sparse/group route |
| Prehash 및 result-mode fused loops | 재사용하는 dictionary의 key equality/rank/tolerance/version, saved-table lifetime, output \`first/last/presence/compact/count/any/all\`; [\`vi.c\` mode 목록](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vi.c#L140-L185) | A3 result intent/provenance → cache/algorithm planning; grouped/count output은 별도 유효성 필요 |

###### N.1 첫 구현 단위 — 기존 IR 변경 없이 실행 경로 개선

2026-10-06 코드 연결:

- \`src/index_ops.rs\`: 내부 \`LookupResult::{First,Last,Membership}\`과 공통 \`lookup(indexed,queries,result)\`로 검색 의도와 materialized output을 분리한다. \`e.\`는 \`i.\`의 정수 위치 배열을 만들지 않고 Boolean 결과를 **직접 생성**한다. 출력 rank·frame과 불일치 셀의 missing/presence semantics는 기존 경로를 보존한다.
- 같은 파일: **integer/bool scalar**만 \`ExactScalarIndex::Direct\` 또는 \`Hashed\`를 허용한다. 최대 직접 테이블 **65,536 entry**, \`span <= 4 × (indexed items + queries)\`의 단순 memory/work heuristic, \`items × queries <= 32\`면 순차 검색. key span 계산은 \`i128\`으로 overflow를 회피하고, 최초/최후 일치 위치·not-found sentinel을 유지한다. **이 threshold는 측정된 최적 비용이 아니라 보수적인 시작값**이다.
- Float/complex/boxed/cell 전체 검색은 새 hash/direct 경로를 열지 않고 기존 \`atom_eq\` sequential fallback을 사용한다. 특히 비추이적 tolerant equality를 일반 key-based hash로 바꾸지 않는다.
- \`src/j_graph_jsource.rs\`: dyadic \`i:\`를 \`SearchAlgorithm\` 후보에 추가, \`E.\`는 제외하고 이미 있는 \`FindViaWindowMatch\` rewrite 책임을 유지. \`I.\` dyad와 단항 \`i./i:\`의 분리는 유지. candidate \`AwaitingSemanticProofs\`/not selected 정책 불변.
- \`tests/index_ops.rs\`·\`tests/j_graph_jsource.rs\`, \`src/index_ops.rs\` 내부 테스트에 scalar first/last/membership, negative/duplicate/missing, direct-vs-hash/extreme \`i64\`/small-query, Graph 후보 구분 회귀 사례를 추가했다. **이번에는 테스트를 실행하지 않았으며 정적 연결만 점검**한다.

이 변화는 **CPU reference implementation의 제한된 알고리즘 개선**이지 jsource의 모든 searching mode를 구현했거나 Graph 후보에 최적화 실행 권한을 준 것이 아니다. \`JSOURCE_FAMILY_RULES\`의 \`SearchAlgorithm\` 및 \`TolerantHash\`를 그대로 사용하며 새 Graph basis를 추가하지 않는다.

###### N.2 다음 구현 gate — 프레임워크 재설계 필요성

**N.2의 초기 확장 계획 중 기초 단계는 O절에서 구현되었다.** A3 `LookupClassify { search: SearchDescriptor }`가 first/last/membership/interval/self-classify 및 indexed/queried origin을 보유한다. 그러나 `!.ct`의 **runtime comparison-policy/version witness**, group/compact/count output, sortedness/uniqueness 및 prepared-lookup key의 완전한 의미론적 증명과 target-independent `CandidateEvidence`는 여전히 후속이다. `FunctionEntity`의 원래 구성·dynamic tolerance·Rank/CellApply 경계를 유지한다.

1. **Proof/semantics:** first/last/Key의 최초 등장 순서·rank/cell/frame/empty, 타입 혼합/box/sparse/tolerance/fit/error precedence를 J/C로 검증한다. 런타임 tolerance 변경은 cached hash를 무효화하거나 key에 포함한다.
2. **Algorithm extension:** 직결 출력의 경우에만 중간 materialization 제거 증명; 작은 범위의 1/2/4-byte packed/presence table, reverse hashing, 재사용 가능한 prehash를 **비용/메모리·alias·version guard와 함께** 하나씩 도입한다. GPU route는 전송·local memory·불균일 충돌 등의 physical resource 예산이 먼저다.
3. **Tolerant hashing 별도 연구 gate:** upstream 두 인접 interval 알고리즘을 그대로 이식하지 말고 \`cct\`/0/NaN/boxed·비추이성·일치 우선순위 증명을 갖춘 \`LookupEqualityWitness\` 아래서만 구현한다. guard miss는 효과 이전 generic J search로 돌아간다.
4. **선택/평가:** \`JsourcePlanningReport\`의 proven/unproven 구분과 결합, \`LoweringRegistry\`의 algorithm-route capability/target/empirical cost를 분리한다. 고정 65,536·32·4× 값은 benchmark와 실측 캐시 profile 전까지 튜닝값으로 취급한다.
5. **검증:** Rust reference, C J oracle, mixed rank/empty/sparse/box/exact+tolerant/negative-zero/large cardinality 및 differential/bench를 통과한 후에만 추후 search fast path의 일반 적용을 승인한다. **CI·Cargo·차등 실행·성능 측정은 이번 연결 작업에서 실행하지 않았다.**



###### N.3 두 번째 구현 단위 — Reverse Hashing과 per-Engine Prehash (2026-10-06)

**대응 근거.** [jsource \`viavx.c::indexofsub\`의 reverse hash 선택](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx.c#L738-L850)은 indexed/query cardinality와 지원 모드에 따라 반대 방향에 검색 테이블을 세운다. [\`vi.c\`의 prehash mode](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vi.c#L140-L185)는 파생 함수의 저장 테이블과 특정 비교 계약을 포함한다. 아래 RustJ 구현은 그 **기본 개념을 제한적으로 차용**한 것이며 C의 prehashed derived verb 전체를 재현하지 않는다.

| 구현 항목 | 실제 경로와 조건 | 보존 계약 / 제한 |
|---|---|---|
| **질의측 Reverse Hashing** | \`src/index_ops.rs::reverse_exact_index\`: indexed 원소 ≥64, \`indexed.len()/2 > query count\`, 단항 셀 \`Int/Bool\` 양측만. 작은 **질의 집합**의 distinct key를 \`HashMap\`에 넣고 원 indexed 배열을 한 번 스캔 | First/Membership은 앞→뒤, Last는 뒤→앞. distinct query key가 모두 해결되면 조기 종료. 중복 질의·없는 값은 각각 같은 position/not-found sentinel을 받는다. float tolerance/boxed/multicell은 제외 |
| **저장소 identity 기반 per-Engine Prehash** | \`ExactPrehashCache\` 하나를 \`Engine\` 내부에 유지. 양측 exact \`Int/Bool\`, indexed rank=1, 64~16,384 items, indexed backing이 \`CpuStorage::Shared(Arc<_>)\`일 때만 사전 빌드. \`i.\`/Membership은 First 공용, \`i:\`는 Last 별도 | **같은 Arc allocation + dtype/shape + First/Last 모드**가 맞아야 hit. 원 source \`Value\`를 공유 clone으로 cache에 보유하여 포인터 재사용/수명 혼동을 차단. binding 재정의로 새 storage가 생기면 재빌드. 오직 한 table만 보존하므로 메모리 상한 제한 |
| **실행 경로 분리** | \`src/runtime.rs::Engine::interpret_ir\`에서 \`pooled\`인 일반 primitive dyad \`i.\`/\`i:\`/\`e.\`의 exact scalar 데이터만 cache-aware entry로 보내며 기타 입력·명시 Rank는 기존 \`kernels::dyad\`로 복귀 | \`eval_semantic_reference\`는 cache 사용하지 않음. \`index_prehash_stats() -> (builds,hits)\`, \`clear_index_prehash()\`는 J value 의미를 바꾸지 않는 관측·clear API |
| **테스트** | \`src/index_ops.rs\` unit: reverse 중복/순서·direct/hash/sequential 참조 대조·Arc identity·rebind; \`tests/index_ops.rs\` integration: 실제 Engine의 cache hit/miss, 멤버십 공유, Last 분리, 새 binding, reference 경로 및 temporary reverse | **작성만 완료.** Cargo/CI/C differential/benchmark 미실행; 정적 연결 확인과 실행 성공을 구분 |

**메모리 안전 fallback:** 선택적 Reverse/Direct/Hash/Prehash 테이블의 `Error::Limit`(준비용 메모리 확보 실패)은 최적화 불허로 처리하여 원래 순차 검색으로 복귀한다. 최종 결과 버퍼의 실제 할당 실패는 별도의 J 실행 오류로 유지한다.

**프레임워크 판정:** canonical \`J Graph → A3 LookupClassify → Physical Planner\`에 새 IR node를 추가하지 않았다. 이 첫 CPU 실행 경로의 비용 threshold(64, 2:1, 16,384)는 실험으로 최적화한 값이 아닌 보수적 초기값이다. 현재 cache는 physical interpreter-local 구현이고 compiler \`JsourcePlanningReport\`의 semantic witness, \`LoweringRegistry\` target-route/cost 선택, guarded optimized transform에 연결되었다고 주장하지 않는다.

**추가로 필요한 것:** \`m&i.\` 또는 \`e.&n\`처럼 J의 derived verb 수준으로 prehash 수명을 관리하려면 별도 \`PreparedLookup\`/versioned dictionary identity/semantic equality contract와 guard/miss fallback을 A3 sidecar에 구현해야 한다. 일반 이름/locale·\`!.ct\` tolerance 변경·sparse/boxed까지 이 캐시를 무조건 확대하면 안 된다. Prehash 준비 과정에서 관찰 가능한 error/effect를 건너뛰지 않는다는 증명도 선행해야 한다. 우선 실제 J/C 출력 차등 및 작은/큰 query 비용 측정 후 threshold와 API를 확정한다.


<a id="algorithm-planning-migration"></a>

##### O. 타 프레임워크 대조를 통한 Algorithm Planning 확장 — i. family 첫 적용 (2026-10-06)

**프레임워크 결정:** jsource의 C 전용 특수 함수를 개별 J Graph IR 노드로 복제하거나, 그 선택을 모두 `src/index_ops.rs`에 넣지 않는다. `J semantic identity → A3 resolved operation/meaning → target legality + proof/guard evidence → Physical cost/choice → guarded executor/fallback`으로 분리한다. **이 구조는 검색에 대한 첫 구현이며 전체 optimizer의 SelectionPlan/자동 튜너 완성을 의미하지 않는다.** `PROJECT.ko.md`가 정본이고 `PROJECT.md`는 같은 구조의 영문 mirror다.

###### O.1 비교한 프레임워크와 채택·비채택 사항

| 검토한 프레임워크·근거 | 해결 방식 (원본 근거) | RustJ에 채택한 원칙 / 하지 않은 것 |
|---|---|---|
| **MLIR Dialect Conversion** ([공식 Dialect Conversion](https://mlir.llvm.org/docs/DialectConversion/)) | `ConversionTarget`가 Legal/Dynamic/Illegal을 분리하고, op/operand/type별 합법성을 판정. `partial conversion`은 나머지를 유지 가능 | **채택:** `SearchAlgorithmReadiness`에서 ordinary baseline, exact-scalar runtime guard, 의미론 증명 필요, target 불가, 다른 연산을 분리한다. 특정 자료형을 안다고 `TolerantNeighborHash`를 승인하지 않는다. **비채택:** J binding/verb identity를 MLIR dialect로 교체하지 않음 |
| **MLIR Transform dialect** ([공식 문서](https://mlir.llvm.org/docs/Dialects/Transform/)) | transform/control IR을 payload IR과 분리하고 recoverable/irrecoverable 실패를 구별 | **채택:** 분석 및 선택 결과는 canonical A3를 변경하지 않는 별도 report/plan으로 유지; guard 실패와 invalid transform을 별개로 취급. **비채택:** Transform dialect IR 그대로 도입하지 않음 |
| **IREE Flow→Stream→HAL / Codegen lowering config** ([파이프라인](https://github.com/iree-org/iree/blob/main/docs/website/docs/developers/general/developer-tips.md), [Codegen LoweringConfig](https://iree.dev/reference/mlir-dialects/IREECodegen/), [LLVM CPU pass](https://iree.dev/reference/mlir-passes/CodegenLLVMCPU/)) | execution/dispatch와 device/backend translation 전략, tiling/vector/bufferization을 구분 | **채택:** A3 `SearchDescriptor`에 hash table 크기, buffer, SIMD, GPU device를 넣지 않으며 target 선택은 Registry/Physical로 미룬다. **비채택:** 지원하지 않는 GPU 검색 커널을 형식상 등록하지 않음 |
| **Apache TVM MetaSchedule** ([공식 tutorial](https://tvm.apache.org/docs/deep_dive/tensor_ir/tutorials/meta_schedule.html)) | `SpaceGenerator → SearchStrategy → CostModel → Builder/Runner → Database`를 분리해 실제 HW 측정 결과로 schedule을 고른다 | **채택:** 합법적인 algorithm 후보/비용 입력/선택 기록을 서로 다른 개념으로 유지. 현재 `SearchPhysicalChoice`의 비용은 **측정값이 아닌 초기 크기 기반 heuristic**. **비채택:** 아직 자동 탐색·벤치 DB·예측 모델은 구현하지 않음 |
| **XLA GPU Priority Fusion / Cost Model** ([설계 설명](https://github.com/openxla/xla/discussions/10065), [pass 원본](https://github.com/openxla/xla/blob/main/xla/backends/gpu/transforms/priority_fusion.h)) | fusion의 합법성과 별도로 estimated `time(unfused)-time(fused)`, memory/compute/launch overhead 등을 근거로 우선순위 결정 | **채택:** semantic proof와 profitability, target feasibility/임시 메모리 사용을 섞지 않음. **비채택:** GPU cost 추정을 임의 상수로 수치화하지 않음 |
| **Futhark SOAC / incremental flattening** ([2026년 개발 설명](https://www.futhark-lang.org/blog/2026-07-31-full-flattening.html), [scan-scatter fusion](https://www.futhark-lang.org/blog/2026-03-24-scan-scatter-fusion.html)) | 고수준 array/dataflow를 보존하고 다양한 flatten/fuse/순차 실행 버전을 target/workload에 따라 선택 | **채택:** 여러 낮은 수준 알고리즘으로 내릴 수 있는 고수준 `LookupClassify`/향후 `GroupBy`·`Reduce` 구조 유지. **비채택:** J의 오류 순서·동적 binding/tolerance 없이 functional fusion 동치식을 사용하지 않음 |

###### O.2 실제 코드로 반영한 세 경계

1. **Execution Semantic Lowering / A3:** `src/logical_ir.rs`의 `ExecutionBasisPayload::LookupClassify { search: SearchDescriptor }`에 **original primitive와 valence에서 도출한** `SearchOutputKind::{FirstIndex,LastIndex,MembershipMask,IntervalIndex,SelfClassify}`, `SearchComparison::{JEquality,JOrderedInterval}`, indexed/queried SSA `ValueId`, original `rank_boundary`를 보유한다. `i.`/`i:` 및 이항 `I.`는 **왼쪽이 indexed set, 오른쪽이 query**이고, 이항 `e.`는 **오른쪽이 indexed set, 왼쪽이 query**다. A3는 이 방향을 이미 바르게 기록하며 Physical strategy가 인수 방향을 추측하지 않는다. 모호한/미지원 derived target은 `Deferred`로 남기고 새 실행 의미를 만들지 않는다. `A3_SCHEMA_VERSION`을 **0.4→0.5**로 올렸고 `Plan::verify`는 payload와 현재 `CallOp`이 일치하지 않는 변조를 거부한다. `JEquality`는 **runtime cct/`!.`를 준수할 의미**이지 float bit-exact hash 허가가 아니다.
2. **LoweringRegistry:** `src/lowering.rs`에서 기존 `ExecutionBasisKind::LookupClassify`에 **ordinary pure CPU reference** route를 등록하고, `SearchAlgorithm::{Sequential,DirectAddress,IndexedHash,ReverseQueryHash,PreparedHash,TolerantNeighborHash}`를 별도 알고리즘 후보로 열거한다. `search_algorithm_reports`는 `SearchDescriptor`에서 `Baseline/RequiresExactScalarGuard/NeedsSemanticProof/UnsupportedTarget/UnsupportedSearchForm` 상태를 계산한다. 현재 다른 J search form, GPU route, TolerantNeighborHash에 대해 근거 없는 실행 가능 상태를 만들지 않는다. `JsourceLinkedCall.search_algorithms`에도 같은 Registry 보고서를 노출하지만 `NeedsSemanticProof`와 candidate 미선택 상태는 그대로다. 기존 `JsourcePlanningReport`의 source provenance 및 미충족 `ProofRequirement`는 **그대로 유지**하며 실행 승인으로 승격시키지 않는다.
3. **Physical Search Planner:** `src/physical.rs::plan_search_algorithm`은 실제 runtime/target에서 얻은 `SearchWorkload`(indexed/query 수, `integer_span`, immutable shared index, prehash eligibility, reverse용 query 접근)를 받고 Registry에 후보 상태를 재질의한다. 순차, 좁은 정수 범위 direct, indexed hash, reverse hash, prepared hash를 provisional bound(32 comparisons; direct 최대 65,536 entries 및 `4×(items+queries)`; reverse 최소 64 indexed 및 크기비 2:1; prepared 64~16,384 immutable shared)에 따라 고른다. output의 index 위치/Boolean mask 의미는 변경하지 않고, `SearchPhysicalChoice`에는 selection basis와 estimated **table entries**만 보고한다. 이는 byte-accurate allocation, benchmarked cost, final BufferId/device schedule이 아니다. `src/index_ops.rs`는 경계에서 Int/Bool 단항 item/runtime dtype을 검사한 다음 planner 결정대로 table을 준비한다. **성능 경계:** 런타임은 매 검색마다 Registry 전체나 후보 Vec를 생성하지 않으며, `LoweringRegistry::search_algorithm_readiness`를 할당 없이 질의한다. 컴파일러 진단용 `search_algorithm_reports`만 후보 목록을 만든다. 선택적 table `Error::Limit`는 순차 fallback으로 되돌리지만 결과 버퍼 오류는 유지한다.

**경계 반례:** 근사 동등성은 비추이적일 수 있어 `!.ct`/floating/boxed를 exact hash로 보내지 않으며 `TolerantNeighborHash`는 `NeedsSemanticProof`. `I.` interval은 Index-Of 일반 hash 후보가 아니다. unknown J name/POS/locale·effect/error ordering·empty prototype은 sidecar/guard 없이 우회하지 않는다. 이 선택기는 **CPU interpreter exact scalar에 대한 검증 전제 실행 경로**이며 `J Graph candidate commit`, `PhysicalArray` 생성, GPU backend, 일반 A3-to-native codegen과 동일하지 않다.

###### O.2a 동작 예시 — `3 1 3 i: 3 4`

- **J Frontend / Graph IR:** 원문의 dyadic `i:`(Last index) semantics, 3의 마지막 위치 및 4의 not-found sentinel을 보존한다.
- **A3:** `LookupClassify { search: SearchDescriptor { output: LastIndex, indexed: left SSA ValueId, queried: right SSA ValueId, comparison: JEquality, .. } }`. 아직 해시 알고리즘이 아니다.
- **Registry:** CPU `Sequential=Baseline`, `IndexedHash/DirectAddress/ReverseQueryHash/PreparedHash=RequiresExactScalarGuard`, `TolerantNeighborHash=NeedsSemanticProof`로 보고한다.
- **Runtime/Physical:** 양측 Int/Bool 단항 셀임을 실행시 확인하며 `3 × 2 <= 32`이므로 순차 검색을 택한다. 예상 결과는 `2 3`이다. 같은 의미의 큰 입력에서는 검색 크기·범위·prehash 조건에 따라 다른 알고리즘을 고를 수 있지만, 의미론적 Last/missing은 불변이다.
- **검증 경계:** 해당 입력에 대한 A3/verifier·Registry·Physical 계획 테스트는 **추가만 했고 실행하지 않았다**. 이 예시는 코드 의도와 J 의미의 계약을 보여줄 뿐 실행 성공/성능 측정 증거는 아니다.

###### O.3 앞으로 공통화할 설계 계약과 검증

이 첫 search-specific adapter에서 확인한 **재사용 가능한 최적화 질문**은 다음과 같다.

~~~text
SemanticDescriptor (A3: meaning, input/value origins, comparison, rank)
  ↓
AlgorithmCandidateSet (Registry: target legality, proof obligations, runtime guards)
  ↓
VerifiedRuntimeFacts (or proven static witnesses; no Unknown→true)
  ↓
CostProfile / ResourceBudget (Physical: work, bytes, occupancy/cache/transfer)
  ↓
SelectionPlan (separate from semantic IR; overlap + fallback)
  ↓
CommittedLowering (only witnessed/guarded, preserves errors/effects)
~~~

- **현재 구현:** 공통 보고서 자료형 `AlgorithmCandidate<Algorithm, Readiness>`와 검색 특화 alias `SearchAlgorithmReport`를 추가했다. 다만 Reduce/GroupBy/Grade/Contract용 **실제 후보 registry·obligation별 witness·공통 CostEstimate/SelectionPlan**으로 일반화하는 것은 **M2/M3 의미론 및 M4 기준선 검증 후, 둘 이상의 독립된 연산 가족에서 공통 계약이 실제로 요구될 때의 후속 작업**이다. Reduce/Scan의 reassociation, GroupBy의 equality/representative, Grade의 tie/order, Contract의 numeric accumulation/precision은 **각기 다른 semantic witness**를 요구한다. `LookupClassify` 전용 enum을 이들에 억지로 재사용하지 않는다.
- `SearchDescriptor`는 A3 J meaning만 표현하고, 향후 `ComparisonPolicy`를 `cct`/`!.` version guard가 붙은 contract로 구체화한다. Rank/CellApply가 외부 layer인 경우는 nested basis와 A3 call instantiation을 함께 검증한다. prehash 및 locale/name binder는 runtime key/version/lifetime에 귀속한다.
- target별 등록과 선택은 독립 검증한다: GPU/XLA식 cost model은 **GPU 실제 realization과 hard resource feasibility 등록 후에만** 합법 후보를 비교한다. out-of-budget/Unknown을 단순히 느린 비용으로 처리하지 않는다.
- 회귀 검사에 A3 `SearchDescriptor`의 mode·indexed/probe origin 및 forged payload 거부, Registry의 target·proof 상태, Physical의 5개 전략/false guard/GPU/Interval 차단을 추가했다. 기존 `i.` first/last/membership/empty/duplicate 및 prehash rebinding 테스트는 별도로 유지한다. **이번 변경에서는 Cargo·CI·jsource differential·benchmark를 실행하지 않았으므로 결과의 실행 검증과 성능 개선을 주장하지 않는다.**



###### O.4 검색 계열을 통한 공통 프레임워크 역설계 — 독립 4관점 재검토 (2026-10-06)

**검토 질문과 판정.** jsource `i.` family를 *어떻게 해시로 구현할까*보다 **왜 한 의미론적 연산에 여러 실행 알고리즘·결과 소비 형태·재사용 수명이 존재하며, RustJ 전체가 이를 어떤 경계로 표현해야 하는가**의 사례로 사용한다. **프레임워크의 큰 층 구분은 이미 옳다. 하지만 ‘검색 의미론 ↔ 후보의 증거 ↔ 자료구조 준비/수명 ↔ 비용/선택 ↔ 독립 실행 기준선’ 사이의 실행 가능한 연결 계약이 미완성**이다. 별도 검색 전용 IR, `vi.c` 모드별 연산, 즉각적인 범용 optimizer 또는 새로운 공통 crate를 추가하지 않는다. 기존 §O.3과 §4.1.4의 모델을 **구현 가능한 소규모 인터페이스 계약**으로 점진적으로 닫는다.

**검토 A — 원본 의미론에서 독립 출발.** pinned [jsource `vi.c`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vi.c)는 `IIDOT`(first), `IICO`(last), `IEPS`(membership), `INUB/INUBSV`(nub·nub sieve), `IIFBEPS`(`I.@e.`), 관련 집계/마스크 및 `IPH...` prehash 모드를 공통 검색 내부 엔진의 변형으로 다룬다. **재사용할 관찰:** domain을 검사하는 계산, 비교 정책, source-order 대표 위치, 결과 요구(output demand), 재사용 가능한 탐색 상태가 서로 구분된다. **재사용하지 않을 것:** C의 mode bit/entrypoint, 해시 배치, SIMD 분기, `viavx2.c`의 tolerant bitmask를 J 언어 의미나 Graph IR 구조로 고정하는 일. 특히 `i.`/ `i:`의 단항 생성형, `I.` 이항 interval, `E.` window find, Nub/Key는 **동일 verb가 아니라 별도의 observable J contract**다. 연산별 의미를 보존한 뒤 legality가 증명된 공통 하위 연산이나 multi-op region만 재사용한다.

**검토 B — RustJ 계층/IR에서 독립 출발.** 현재 `src/logical_ir.rs::SearchDescriptor`는 원본 `ValueId`, `indexed/queried` 방향, first/last/membership/interval/self-classify intent, `JEquality` 또는 ordered-interval 의미, rank boundary를 보유한다. `src/j_graph_jsource.rs`는 후보 발견만 하고 미증명·미선택 상태를 유지한다. 이것은 **보존**한다. 다만 `JEquality`라는 표식만으로 실제 `!.t`/전역 `cct`, 비교 타입 승격, boxed/complex/sparse, rank/cell/frame·empty prototype·error-order 증거가 성립하지 않는다. `E.`는 이미 별도 window rewrite candidate의 의미 영역이므로, C에서 내부 검색 루틴을 공유한다는 이유로 `LookupClassify`에 흡수하지 않는다. `I.@e.`와 membership-consumer 합성은 **복수 원본 graph node/consumer demand/provenance를 가진 region 후보**이며 primitive 하나를 새로운 특수 IR opcode로 대체하지 않는다.

**검토 C — 물리 실행/비용에서 독립 출발.** [BQN 검색 구현 설명](https://mlochbaum.github.io/BQN/implementation/primitive/search.html)에서 normal/reverse/sparse-table lookup은 **탐색 대상과 query의 어느 쪽에서 자료구조를 만들고 순회하는가**, one-shot vs persistent hash는 **준비 비용과 상태 수명**, direct lookup/hash/SIMD는 **실제 자료구조/타깃**, membership/index/first/last는 **관찰 결과**가 다르다. 현재 `src/lowering.rs::SearchAlgorithm`의 `DirectAddress/IndexedHash/ReverseQueryHash/PreparedHash/TolerantNeighborHash`는 첫 세 축을 **하나의 열거형에 혼합**하고 있다. 현행 제한된 CPU 구현은 유지할 수 있으나 범용화 시 ‘알고리즘 열거형을 모든 다른 primitive에 복사’하면 안 된다. 향후 후보 recipe의 직교한 선택 축을 **(1) 탐색·순회 방향 (2) 키/테이블 표현 (3) 구축·재사용/무효화 수명 (4) 출력 materialization/consumer (5) 대상·자원/비용**으로 *설명*하고, 실제 조합은 검증된 후보만 등록한다. 이것은 **후일 optimizer/Physical 전용 표현**이며 J Graph 또는 A3 스키마 변경 요구가 아니다. 임의 Cartesian-product 후보 폭발도 금지한다.

**검토 D — 반례/독립 oracle에서 독립 출발.** `src/index_ops.rs::lookup`의 `None` cache 경로도 `optional_exact_scalar_index` → `plan_search_algorithm`을 호출하므로 `Engine::eval_semantic_reference`는 **캐시 없는 reference일 뿐 엄밀한 sequential reference가 아니다**. `near`와 pinned C `TCMPEQ`의 이진64 경계 차이는 comparator identity 없이 최적화 동등성을 주장할 수 없다는 반례다. 비추이적인 tolerant equality는 대표 원소 하나의 해시 등가류로 축약할 수 없고, first/last와 nub/group 대표 선택의 proof도 다르다. 입력 변경·NAME rebinding·정책 변경·backend 전환·empty cell·guard 실패 시 어느 캐시/후보가 무효인지 증명 없이는 reuse하지 않는다. **언어 oracle=실제 jsource C**, **Rust 순차 semantic baseline**, **선택된 RustJ optimized route**의 세 경로가 독립적이어야 한다.

**프레임워크 대조 재검산(원본 비교, 자동 채택 아님).** [MLIR Dialect Conversion](https://mlir.llvm.org/docs/DialectConversion/)은 **analysis conversion**에서 IR을 고치지 않고 합법적으로 낮출 수 있는지 검사하고, **dynamic legality**는 실제 op 인자·속성에 따라 변환 허용을 결정한다. [MLIR Transform Dialect](https://mlir.llvm.org/docs/Tutorials/transform/)은 변환 대상(payload)과 별도 transform/schedule 표현을 사용할 수 있음을 보여 준다. RustJ에는 각각 **‘후보 발견≠채택’, ‘J semantics·runtime guard 없는 법적 허용 금지’, ‘원본 Graph/A3≠선택 계획’**으로 대응한다. 단, MLIR의 IR conversion에 성공했다고 J `!.t`·dynamic NAME·오류 순서를 증명한 것은 아니며, 지금 RustJ에 Transform Dialect나 MLIR 의존성을 설치해야 한다는 뜻도 아니다.

**계층별 증분 확장 계약 — 입력 / 유지할 정보 / 해당 계층에서 금지할 결정**

| 계층 | 유지·도출해야 할 최소 계약 | 금지 및 지금의 작업 |
|---|---|---|
| M2 Parser / FunctionEntity | primitive/derived identity, valence, source binding, `!.t`/동적 설정의 observable scope | C mode/CPU 특수 함수로 파싱 결과를 교체 금지; **M2 일반 parser 작업 우선** |
| J Graph / Analyzer | 원본 topology, query/domain role, producer/consumer·fanout·source provenance, composition 후보 | output demand를 잃거나 multi-op fusion을 일찍 확정하지 않음 |
| Execution semantics / A3 | first/last/member/interval/window/group의 **별도** 결과 의미, frame/cell/shape/empty, 비교 정책 계약, 오류·효과 | hash/direct/prehash/target을 J equality 또는 `SearchDescriptor` 안에 내장하지 않음 |
| Candidate discovery / proof (§4.1.4) | `RuleId`, source graph version/region, semantic obligation별 `Unknown/Proven/Disproven/Guarded`, exact runtime witness | observed source idiom이나 기본 dtype만으로 legal 선언 금지; stale candidate 폐기 |
| Target/resource/cost | runtime 또는 정적 근거로 검증한 dtype/item shape/count/range, table byte upper bound, build/probe/reuse, memory tier/transfer, target capability | 추정 work를 측정 시간으로 오인, unknown resource를 zero/cheap로 오인 금지 |
| 선택 계획 / index-state lifetime | candidate compatibility, direction/representation, 일회성·persistent 인덱스의 backing/policy identity/epoch, escape/invalidation, failure-before-effect fallback | J Graph를 변형해 cache/selection을 저장하거나 J 이름만으로 prepared state를 영속화하지 않음 |
| Executor / verification | 순차 reference, guarded route, J C oracle와 동일 결과/오류·효과 순서, 기록된 선택 원인 | 순차 oracle 내부에서 Physical 선택기 재호출 금지; guard miss 후 observable effect 재실행 금지 |

**최소 공통 프레임워크 확장 순서(현재 구현 지시가 아닌 수용 게이트):**

1. **M2:** `i.` family를 위한 새 hash가 아니라 search/reference의 *독립된 순차 baseline*과 실 J C differential, CCT·fit·Rank 등의 의미론 불일치 분류를 먼저 닫는다. 기존 `SearchDescriptor`의 방향·output 검증은 보존한다.
2. **M3:** §4.1.4의 **후보 provenance + obligation별 proof/guard + invalidation** 공통 evidence를 기존 search/rewrite/fusion 분석 sidecar에 투영할 최소 인터페이스를 정한다. `Unknown`을 완료로 해석하지 않는 verifier/negative test를 하나씩 만든다. **완전한 범용 registry를 먼저 설계하지 않는다.**
3. **M4:** 독립 CPU reference vertical slice와 기존 Int/Bool guard된 Physical route를 같은 J oracle로 검증한다. `SearchAlgorithm`·`PreparedHash` 등의 기존 enum을 일단 유지하며, 부하/표본 비용 없이 임계값을 바꾸지 않는다.
4. **최적화 착수 시:** 두 번째 독립 가족(예: Reduce/Scan의 reassociation, GroupBy의 대표 원소)에서 **실제 동일한 증거·수명·selection 인터페이스가 필요함을 증명한 후에만** 공통 `CandidateEvidence`/recipe/SelectionPlan으로 추출한다. `SearchDescriptor`나 search mode enum을 generic operator로 포장하지 않는다.
5. **이후:** one-shot vs reusable index, reverse-query 구축, materialization-elision, table footprint, GPU/외부 route 등의 물리 후보는 **full-J legality + target/resource hard gate + 실제 측정** 후에만 별도 plan으로 commit한다.

**반복 독립 검사와 실패 조건:** (A) jsource 실행 모드 이름을 제거해도 semantic descriptor가 J 결과를 결정하는가? (B) 모든 선택 후보를 비활성화해도 reference는 올바른가? (C) 비교 정책만 바꾸거나 source binding을 교체했을 때 캐시/guard가 무효로 되는가? (D) 동일 입력을 CPU/외부 adapter로 내릴 때 J Graph/A3가 불변인가? (E) 단일 search output이 아닌 `I.@e.`/Nub/Key/consumer fusion에서도 잘못된 범용화 없이 witness가 재사용되는가? **현재 답:** (A)의 의도/분리 일부 구현, (B) 독립 baseline 부재, (C) 동적 정책 미지원, (D) 구조 규정은 있으나 각 target 실행 미검증, (E) 해당 derived family semantic coverage 미완료. 따라서 공통 범용 optimizer ‘완료’나 search-specialized 성능 이득을 주장할 수 없다.

**이번 검토 범위:** 파일별 read-only 정적 점검 + pinned jsource 원본 + BQN/MLIR 설계 대조. **Rust/Cargo 실행, 전체 J binary differential, benchmark 또는 optimizer 새로운 API 구현은 하지 않았다.** 이 결정은 §P.0의 특수 코드 확장 동결 및 §P.3의 M2 우선순위를 **강화**하며 대체하지 않는다.




<a id="framework-migration-checklist"></a>

###### O.5 지연 최적화 프레임워크 이행 계획·체크리스트 (2026-10-06, 살아 있는 작업표)

**목적·범위.** §O.4의 네 관점 독립 검토를 **실제로 수정하고 검사할 수 있는 단일 체크리스트**로 전환한다. `i.` 계열은 최초 검증 사례이고, 최종 목표는 **J 의미론을 유지한 채 다른 연산군에도 적용 가능한 ‘발견 → 증명/가드 → 하드 자원/타깃 판정 → 비용 → 선택 → 실행’ 계약**이다. §P.1은 계속 **검색 알고리즘 자체의 상세 체크리스트**로 유지한다. **이 표가 단계 간 우선순위·완료 판정의 정본**이며, 새 로드맵 Markdown이나 새 범용 optimizer crate를 만들지 않는다.

**현재 진행 상태:** **이행 계획 문서화 완료 / 아래 구현·검증 게이트 0/18 수용.** 기존 `SearchDescriptor`, `AlgorithmCandidate<_,_>`, `GraphRewriteCandidate`, `FusionCandidate`, `ExactPrehashCache` 등은 **활용 가능한 현재 코드**이지 새 계약의 수용 완료를 뜻하지 않는다. **전체 프로젝트 우선순위는 M2 tokenizer → enqueuer → parser/POS/name/derived entity이며, M3/M4를 앞당기는 명분으로 검색 특수 알고리즘을 추가하지 않는다.**

**체크 상태 규칙.** [ ] = 수용 전(코드 일부 존재/작성/검증 대기 포함); [x] = **필요한 코드 + 실제 실행 검증 + 증거 링크를 모두 완료한 경우에만**. 단순 소스·테스트 파일 존재, 내부 Rust reference끼리의 일치, static assertion, 이전 milestone 수치는 완료 근거가 아니다. 각 항목을 완료할 때 같은 표의 **검증 기록**에 `commit SHA | 실제 명령 | 환경/target | pass/fail/ignored | jsource 고정 commit·실제 oracle 실행 범위 | 잔여 제한`을 적는다. CI 실행 여부는 별도 표시하며 실시하지 않았으면 ‘미실행’이라고 쓴다. 여기에 적는 날짜는 계획 작성일이지 완료일이 아니다.

**순서 및 게이트.** A(M2·기준 의미론) → B(M3·증거 인터페이스) → C(M4·독립 CPU 경로) → D(실측 이후 최소 공통 Physical 확장). 각 단계는 **자기 범위 내 이미 지원하는 J 의미론**에 대한 진짜 검증으로 통과시킬 수 있지만, 아직 지원하지 않는 J 형태는 `Unsupported/Unknown`으로 표시해야 한다. ‘full J 완료’를 허위 전제하지 않는다. A의 일반 frontend 미완료 항목이 첫 작업이고, search reference 작업은 이에 종속된 작은 수직 검증 과제로 진행한다. **한 번에 한 의미 + 하나의 회귀/negative test**를 기본 변경 단위로 한다.

| ID / 단계 | 완료 체크 | 수정 대상·실행 작업 | 선행 조건 / 수용 기준·검증 기록 |
|---|---|---|---|
| FW-01 / A·M2 | [ ] 프런트엔드 의미론 범위 고정 | `src/tokenizer.rs`, `src/enqueuer.rs`, `src/parser.rs`, `src/semantic.rs` 등 **실재 경로 확인 후** 미완료 syntax/POS/name/derived-entity 사건을 분류 | 일반 M2 우선순위를 유지. 기존 지원 subset의 parse/resolve 결과와 C oracle 대조; unsupported와 unresolved을 기록. **검증: 미실행** |
| FW-02 / A·M2 | [ ] 의미론 실행 모드 분리 | `src/runtime.rs`, `src/kernels.rs`, `src/index_ops.rs`: 현재 `pooled: bool`의 buffer pool 정책과 검색 physical 허가를 직교시킨다. reference `i.`/`i:`/`e.`는 `lookup`의 **순차 실행**만 통과; ranked/cell/derived call이 동작하는 범위에서 우회 없는지 확인 | `eval_semantic_reference`에서 `plan_search_algorithm`/prehash 호출이 **실제로 0회**임을 계측·negative test로 검증. 새 공개 API, 전역 mutable switch 또는 source IR 변경 불필요. **부분 구현/검증(2026-10-06):** `45ccebb`, `3436252`에서 `search_reference.rs` 순차 `i./i:/e.` 기준 경로와 prehash 미사용 회귀 검증. [Linux milestone 성공](https://github.com/yunskim/RustJ/actions/runs/37438389996), [Basis compile probe 성공](https://github.com/yunskim/RustJ/actions/runs/37438390025). **검증 추가:** [`80d0616`](https://github.com/yunskim/RustJ/commit/80d06163ea2d7966641bacd24cb1ac9ecc9e316c) 테스트 전용 same-thread planner 호출 카운터 및 최적화 경로 positive control. 서식·Clippy 수정 [`b86aafb`](https://github.com/yunskim/RustJ/commit/b86aafb0969c8e6b51fe827c6c83c1cce96e934b). [Linux milestone check 성공](https://github.com/yunskim/RustJ/actions/runs/37440725863), [Basis probe 테스트/Clippy 성공](https://github.com/yunskim/RustJ/actions/runs/37440725669); 전체 J C differential job은 별도. **미수용 [ ]:** rank/derived 우회 검증, C binary와의 3방향 의미론 비교, 전체 FW-02 수용 검증은 미완료. |
| FW-03 / A·M2 | [ ] 언어 비교 의미 정렬 | `src/comparison_policy.rs`, `src/kernels.rs`, 관련 semantic contract: 고정 Rust `near` vs C `TCMPEQ`, 전역 `cct`, `!.t` scope/override, 타입·오류·NaN/±0 등 지원 범위 분명히 한다 | 먼저 실제 **고정 jsource C binary oracle**로 경계 예제를 확인. 동적 설정 미지원이면 `Unknown/Unsupported` 처리하고 tolerant optimization 허가하지 않음. **FW-03 진단 기록(2026-10-06, 미수용):** 고정 C [`jsource@13994ffa`](https://github.com/jsoftware/jsource/tree/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528) `vcomp.h::TCMPEQ`/`i.c::cct=1-FUZZ`와 Rust `FixedRustNearV0` 사이의 binary64 경계를 [`tools/tolerance_audit.py`](https://github.com/yunskim/RustJ/blob/main/tools/tolerance_audit.py)로 실제 3경로 비교. [CI 37445080131](https://github.com/yunskim/RustJ/actions/runs/37445080131) Ubuntu Linux `j64`,`j64avx2` × Rust default/portable **각 48건 중 40 3-way 일치 / 8 C-vs-Rust 의미 불일치 / 0 Rust baseline-vs-optimized 불일치**(4 조합 총 192 관찰). `python3 tools/tolerance_audit.py --reference-revision 13994ffa1ed5f06f79fad6e9822a7ed2d29b1528 --report reports/tolerance-audit-<variant>-<backend>.json`; 모든 미일치는 report 원자료에 보존하며 CI 성공은 **진단 자체 성공**이지 J 동등성 합격이 아님. `9!:19`, `!.t`, Rank·boxed·sparse 및 전체 오류 우선순위는 미검증. **[ ] 유지.** **후속 구현/검증:** [`9aab336`](https://github.com/yunskim/RustJ/commit/9aab3362ac542ed96c0bd3ba6eb2ab6c109f14f4) pinned 기본 CCT 적용. [CI 37445891526](https://github.com/yunskim/RustJ/actions/runs/37445891526) j64·j64avx2 × default·portable **각 48/48, 총 192/192 일치, Rust reference-vs-optimized 불일치 0**. 위 40/8은 변경 전 이력. 범위/미완료 의무는 §P.9를 따른다. **[ ] 유지.** |
| FW-04 / A·M2 | [ ] 검색 reference 회귀·3방향 fixture | `src/index_ops.rs` 및 기존 differential tests: first/last, membership, 빈 셀·frame, 중복, 타입, 오류·Rank를 **C oracle / 순차 Rust / 최적화 Rust**로 분리한 케이스 구축 | 불일치 분류(언어 의미 vs optimized route), 실패 시 미지원/대체 실행 명시. 2개 Rust 경로가 동일 planner를 사용하면 3방향 증거로 세지 않음. **부분 구현(2026-10-06):** [`1f3d1cc`](https://github.com/yunskim/RustJ/commit/1f3d1cccfc5c30b385f5d886adc9cd9ac5872f22)의 exhaustive 3값 정수 4,840 `(indexed,query)` 조합에서 first/last/membership의 독립 위치 witness와 최적화 경로 비교 추가. **검증:** [`b86aafb`](https://github.com/yunskim/RustJ/commit/b86aafb0969c8e6b51fe827c6c83c1cce96e934b) Linux milestone check의 `cargo test` 성공. **C oracle/Rank/boxed/sparse/CCT 3방향 검증 미완료 [ ].** **2026-10-06 확장 (검증 대기):** [`bc648cc`](https://github.com/yunskim/RustJ/commit/bc648cc0ba43761e103cd42c58830147c30ebb9c)에 독립 프로세스 3방향 C/Rust baseline/Rust optimized harness 45문장 추가; [`4e8b4dd`](https://github.com/yunskim/RustJ/commit/4e8b4ddd1d4a5b9d065830fb0211137f9b894d26)에서 분류 테스트, [`09f9a3b`](https://github.com/yunskim/RustJ/commit/09f9a3ba7804fc769027b16c7c9f5abc630339ac)에서 j64·j64avx2 × default·portable CI gate 등록. 지원 dense subset만 검증하며 Rank/boxed/sparse/CCT는 미검증. **실행 증거(2026-10-06):** [Linux milestone run 37441174098](https://github.com/yunskim/RustJ/actions/runs/37441174098), 명령 `python3 tools/search_three_way.py --reference-revision 13994ffa1ed5f06f79fad6e9822a7ed2d29b1528 --report reports/search-three-way-<variant>-<backend>.json`; GitHub Actions Ubuntu Linux, J C `j64`/`j64avx2` × Rust default/portable 네 조합 **각 45 passed / 0 failed / 0 ignored, 합계 180 비교**. 실제 jsource pin `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`; 개별 fixture/해시/불일치 종류는 CI JSON artifact. **제한:** supported dense search 정수·bool·문자·정확 부동소수, 중복·빈 배열·multi-item·재검색; Rank-파생 우회/boxed/sparse/동적 CCT·Fit 미검증. **FW-04 전체 [ ] 유지.** **Rank 차분 진단(2026-10-06, 수용 아님):** [`5b4a7d6`](https://github.com/yunskim/RustJ/commit/5b4a7d6ddf0fdf8e966ee06f63173e71cdbce423) `tools/ranked_search_audit.py`, [`c300ff3`](https://github.com/yunskim/RustJ/commit/c300ff3295499f04e41c928a78e38cd3ed95d7fc) corpus 단위 테스트, [`d88c38b`](https://github.com/yunskim/RustJ/commit/d88c38b34530c84d7ed02b46a9cbf5a55186a50f) CI matrix 연결. [CI 37446732381](https://github.com/yunskim/RustJ/actions/runs/37446732381) Ubuntu Linux `j64`/`j64avx2` × Rust default/portable 각 **14 cases / 13 C·Rust reference·Rust optimized 일치 / 1 Rust 양 경로 Unsupported**, 합계 **52/56 일치(미해결 4개 관찰)**. 불일치 입력 `(i.0 3) (i."1 1) (i.0 3)`에서 C는 Int shape `[0,3]` 빈 결과, Rust 양 경로는 `Unsupported`(Rank empty-frame prototype inference). **이 사례를 성공/허용 불일치로 간주하지 않는다.** J Rank 결과 prototype·dtype·shape/오류 계약을 먼저 구현하고 3방향 재검증할 것. P.1/search exact optimization 증명으로 전용하지 않음; FW-04/JX-04 [ ] 유지. **P.10 zero-frame Rank 보강(2026-10-06):** [CI 37449102885](https://github.com/yunskim/RustJ/actions/runs/37449102885) pinned j64·j64avx2 × default·portable 4조합 각 20/20(총 80/80), supported dense typed fill-cell·monad/dyad·search/reference/optimized 3방향 일치. [Basis probe 37449102980](https://github.com/yunskim/RustJ/actions/runs/37449102980) 성공. Boxed/sparse, computational error fallback, 사용자 정의 effectful Rank 및 mixed-result padding 미지원으로 **[ ] 유지**. **후속 단위 작업:** [§P.11 RK-01~RK-12](#rank-cellapply-followups) 체크리스트에 실행 증거·진행 상태를 기록한다. **상위 [ ] 유지.** |
| FW-05 / B·M3 | [ ] 후보 원본·버전 식별 계약 | `src/j_graph_ir.rs`, `src/j_graph_rewrite.rs`, `src/j_graph_fusion.rs`, `src/j_graph_jsource.rs`: 기존 source ID/span/basis에 **graph/schema/rule identity 또는 유효성 검증 수단**을 연결. original J Graph는 불변 | stale graph / changed rule / forged origin을 거부하는 verifier negative tests. 단순 span 일치를 유일한 유효성 증명으로 취급하지 않음. **검증: 미실행** |
| FW-06 / B·M3 | [ ] obligation별 증거 상태 | §4.1.4의 `CandidateEvidence` 목표를 search/rewrite/fusion **sidecar view 또는 최소 adapter**로 구체화: `Unknown / Proven(witness) / Guarded(guard,fallback) / Disproven`과 provenance를 분리 | 동일 `selected` bool·SearchAlgorithm enum·단일 큰 공통 IR로 합치지 않음. `Unknown`이 legal/selected로 승격되지 않는 negative tests. **검증: 미실행** |
| FW-07 / B·M3 | [ ] 가드·효과/오류 선후 관계 계약 | `src/lowering.rs`, effect/error/verifier 및 실행 adapter: guard를 **효과 발생 전**에 두고 miss 시 정확한 reference fallback 명시 | Observable effect 뒤 자동 재실행·중복 오류·fallback 누락을 거부. `Guarded` 상태만으로 선택 허가하지 않음. **검증: 미실행** |
| FW-08 / B·M3 | [ ] 비교 정책·이름·Rank의 동적 witness | `src/logical_ir.rs::SearchDescriptor` 주변 semantic policy 계약과 `comparison_policy.rs`: `!.t`/runtime CCT, dynamic NAME, rank/cell/empty 등이 후보별 어떤 증명을 요구하는지 표준화 | `JEquality`라는 라벨·dtype만으로 `exact hash`를 정당화하지 않음; 미확정 policy/Rank이면 선택 거부·reference 경로. **검증: 미실행** |
| FW-09 / B·M3 | [ ] 기존 3종 후보에 같은 legality view 투영 | `j_graph_rewrite.rs`, `j_graph_fusion.rs`, `j_graph_jsource.rs` + `fusion_planning.rs`에 provenance/obligation 상태를 **별도 원본 타입을 유지하며** 노출 | 후보 중복·기존 E. window witness를 검색 witness로 오용·unresolved resource를 legal로 처리하는 경우 거부. **검증: 미실행** |
| FW-10 / B·M3 | [ ] 두 번째 독립 가족 교차 검증 | 기존 Reduce/Scan **또는** GroupBy의 semantic candidate 하나를 선정해 FW-05~09의 증거/가드 인터페이스 적합성 확인 | 단지 두 가족에 enum 이름만 공유하는 것은 불합격. 다른 numeric/대표원소/order proof를 보존하면서 공통성이 실제 입증될 때만 구현 인터페이스 추출; 범용 registry 선행 구축 금지. **검증: 미실행** |
| FW-11 / C·M4 | [ ] Native CPU semantic vertical slice | 실제 supported subset의 J Graph → A3 verification → baseline CPU execution 경계를 닫고 reference 실행과 비교 | source/error/effect order를 유지. C binary oracle 범위·명령·환경·차분 결과를 기록. **검증: 미실행** |
| FW-12 / C·M4 | [ ] 기존 exact Int/Bool 경로 독립 검증 | `src/index_ops.rs`, `src/physical.rs`: Sequential / Direct / Indexed / Reverse / Prepared 별 output first/last/member와 allocation failure fallback 확인 | 같은 planner에 의존하지 않는 순차 reference 및 C oracle에 대해 3방향 검증. Rust default/portable는 각각 실제 결과 기재. **검증: 미실행** |
| FW-13 / C·M4 | [ ] target·hard resource·cost의 독립 승인 | `src/lowering.rs`, `src/physical.rs`, `src/j_graph_resource.rs`: static/runtime exact guard, table **byte** bound, estimated work·temporary allocation, target capability, measured cost를 서로 별도 상태로 보고 | hard limit `Unknown`은 값 0이 아니며 ‘느림’과 ‘불법/부적합’을 혼동하지 않음. 미측정 threshold는 heuristic으로 표시. **검증: 미실행** |
| FW-14 / D·후순위 | [ ] 물리 recipe의 선택 축 분리 | 충분한 두 가족 증거·실측이 나온 뒤 `SearchAlgorithm`과 `SearchWorkload/Choice`의 축(빌드/순회 방향, hash/direct/SIMD 표현, lifetime, output demand, target)을 재설계할 필요성 판정 | **구체적 공통 사용 사례 2개**와 기존 API migration·동등성 테스트 없는 대대적 분해 금지. J Graph/A3 schema에 물리 선택을 넣지 않음. **검증: 미실행** |
| FW-15 / D·후순위 | [ ] prepared-state 수명/무효화 계약 | `src/index_ops.rs::ExactPrehashCache`와 향후 다른 가족의 prepared state 비교: backing identity/immutability, first/last representative, policy/version, binding epoch, target, retention | 기존 exact Int/Bool 키에는 무관한 policy를 억지로 넣지 않음. 실제 영향을 주는 속성만 witness/key 포함; rebinding·cache miss·정책 변경 negative tests. **검증: 미실행** |
| FW-16 / D·후순위 | [ ] 별도 SelectionPlan·충돌/호환성 검사 | §4.1.4 target/semantic/resource/cost evidence로 검증된 후보 집합의 overlap·compatibility 검사와 별도 selection/committed lowering 식별자 | Graph/SSA source와 original witness 보존. legality 없는 선택, overlapping incompatible candidates, stale commit 거부. **검증: 미실행** |
| FW-17 / D·후순위 | [ ] 실측·타깃 확장 승인 | 독립 cost benchmarking 후에만 jsource/BQN 작은 배열·SIMD·reverse/prehash 조정 및 CPU/외부 route 검토. GPU는 기존 보류 정책 유지 | 동일 semantics + 메모리 상한 + 실측 win + portable fallback+negative tests 통과 시 **개별** 전략 활성화. **검증: 미실행** |
| FW-18 / 전체 수용 | [ ] 반복 독립 검토·회귀 기록 | A) 원본 J 의미 B) Graph/A3 불변 C) proof·invalidations D) CPU/target/resource E) C·순차·optimized 3방향/성능의 **서로 독립된 재검토**를 수행하고 §P.2와 연결 | 충돌 시 앞 단계 재개; pass/fail/ignored·실행 환경·원본 revision·bench 근거가 남고 기존 §P.1 체크 상태와 모순이 없어야 최종 수용. **검증: 미실행** |

**의존성/중단 규칙.** FW-02가 완료되기 전에는 “optimized/reference 통과”라는 내부 상호 비교만으로 FW-12를 완료하지 않는다. FW-03·08의 dynamic 비교 의미가 미완성인 동안 tolerant hash의 `NeedsSemanticProof`를 변경하지 않는다. FW-05~09에서 공통 증거를 정의했다고 FW-10의 **실제 두 번째 가족 검증**이 된 것은 아니다. FW-11~13이 완료되고 명시적인 메모리·타깃·실측 증거가 없으면 FW-14~17의 물리 전략/비용 임계값 변경을 시작하지 않는다. 버그 수정과 의미론 정확성 보완은 언제나 우선 가능하다.

**검증 진행 로그 템플릿(완료 시 해당 행에 복사):** `FW-ID | 코드 commit | 실행한 명령과 환경 | passed/failed/ignored | 사용한 jsource commit+실제 J 실행 범위 | 확인된 unsupported/known gaps | 다음 게이트`. 계획만 추가한 지금은 모든 실행 검증 항목이 **미실행**이다. 별도 일일 보고 파일 대신 이 절을 갱신하고 영문 §O.5와 체크 상태를 함께 맞춘다.



<a id="index-family-roadmap"></a>

##### P. i. family 통합 로드맵·진행 체크리스트 — Roger Hui × Marshall Lochbaum (2026-10-06)

**교차 참조:** [§O.5 프레임워크 이행 계획](#framework-migration-checklist)은 **공통 구조·단계 게이트**를 관리하며, 여기 §P.1은 검색-family 알고리즘별 세부 검증을 관리한다. 두 표의 [x] 규칙은 다르므로 이행 계획의 [x]를 코드 존재만으로 변경하지 않는다.

**정본 운영 규칙.** 이 체크리스트는 [§N: i. 원본 및 1·2차 구현](#jsource-index-family), [§O: A3/Registry/Physical 경계](#algorithm-planning-migration), [§4.1.4: proof→selection lifecycle](#)에 종속된 **살아 있는 구현 게이트**다. 항목마다 **근거 / 실행 계층 / 완료 조건 / 검증 상태**를 명시한다. [x]는 **코드·문서가 저장소에 존재함을 정적으로 확인**했음을 뜻할 수 있으며, 테스트 실행·jsource 동등성·성능 달성을 자동으로 의미하지 않는다. 별도 문서를 늘리지 않고 이 절에서 관리하며 완료될 때마다 실제 검증 근거를 기록한다. **Linux milestone은 별도로 만들거나 완료 조건에 넣지 않는다.**

**독립 참고 원본 및 적용 기준:**

- Roger Hui, [*Index-Of, a 30-Year Quest*](https://www.jsoftware.com/papers/indexof/indexof.htm) 및 [*Hashing for Tolerant Index-Of*](https://www.jsoftware.com/papers/Hashing.htm): J의 첫·마지막 일치, Nub/Key의 대표 원소, tolerant comparison 및 소스 실행 모드의 출발점. 실제 동작의 대조 원본은 pinned [jsource `vi.c`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vi.c#L140-L185), [`viavx.c`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx.c#L738-L850), [`viavx2.c`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx2.c#L8-L98). C fast path 자체를 동등성 증명으로 사용하지 않는다.
- Marshall Lochbaum, [*BQN: Implementation of search functions*](https://mlochbaum.github.io/BQN/implementation/primitive/search.html): 작은 배열 순차/SIMD, lookup table, one-shot hash, reverse hash, lazy/sparse **table initialization**, open-addressing/linear probing, collision observation, partitioning/radix 및 cache 비용을 비교하는 **알고리즘·물리 최적화 참고**. BQN과 J의 비교 동등성, Fit, Rank, Boxed/AxisSparse 의미가 같다고 가정하지 않는다. BQN의 *sparse lookup*은 **직접 주소 테이블 일부만 초기화하는 기법**이며 J sparse array와 다르다.
- 외부 프레임워크 [§O](#algorithm-planning-migration)의 MLIR dynamic legality, IREE target lowering, TVM measured cost, XLA resource/cost 분리는 **알고리즘 소유권 경계**로 재사용한다. 새로운 IR 계층이나 C 함수별 Graph node를 만들지 않는다.

###### P.0 지연 최적화 정책과 M2 우선순위 검토 (2026-10-06)

**판정: 구조적 분리는 대체로 적합하지만, 작업 우선순위와 일부 조기 실행 전문화는 조정해야 한다.** 프로젝트 최상단의 정본 우선순위는 **M2 word formation → enqueue → J parser/이름·POS·derived entity 의미론 수렴, 이후 M3 경계와 M4 Native CPU vertical slice**이다. `FOUNDATIONS.ko.md` Part XX는 J source를 실행 계획으로 취급하지 않고, 높은 수준의 rank/train/reduce/scan 구조와 source provenance를 보존하며, logical legality와 target-specific profitability/materialization을 분리하라고 규정한다. 여기서 **지연된 최적화**는 모든 분석을 뒤로 미루는 것이 아니라, **특수 실행 알고리즘의 확정·materialization·선택을 충분한 의미론/타깃/비용 증거 이후로 미루는 것**이다.

| 작업 분류 | 현재 판단 | 단계 경계 및 조치 |
|---|---|---|
| jsource 검색 family의 primitive/valence/Rank 의미, `!.t`/전역 `cct`, 오류·First/Last·비추이성·원본 C comparator 반례 | **현재 의미론 과제** | M2 의미론/differential 및 M3 semantic contract에서 해결한다. jsource는 **언어 oracle/반례 발견 수단**이며 특수 해시 알고리즘 복사 지시가 아니다 |
| J Graph source topology·A3 `SearchDescriptor`·원본 SSA provenance·`LookupClassify` | **유지** | 비교 정책/결과 intent/Rank·frame 및 아직 모르는 facts를 보존한다. 임시 table size, bucket, hashing, SIMD를 IR 의미로 승격하지 않는다 |
| `JsourceOpportunity`, `SearchAlgorithmReadiness`, `ComparisonPolicySnapshot` | **증거/경계에 한해 유지** | 후보 보고는 `selected=false`, 허용되지 않은 tolerance route는 `NeedsSemanticProof`; 현재 snapshot은 고정 Rust 비교식의 identity일 뿐 J 동적 policy 구현이 아니다 |
| 현재 동작 중인 CPU Int/Bool Direct/Hash/Reverse/Prehash | **기존 제한적 interpreter optimization; 확장 중지** | 실행 시 exact scalar guard·순차 fallback을 지키는 범위에서만 유지. benchmark와 전체 J differential 없이 일반 compiler PhysicalPlan의 완성이나 jsource 동등성이라고 부르지 않는다. 의미론 회귀가 발견되면 우선 reference로 회귀시킨다 |
| `src/tolerant_search.rs` 연구 bucket, BQN SIMD/작은 범위 table, open-addressing·radix/partition, 임계값 조정, 실전 tolerant hashing | **후순위 / 동결** | M2 의미론이 닫히고 M3 검증 계약, M4 CPU reference vertical slice 및 비교·비용 측정이 마련된 후 독립적으로 평가한다. 후보 연구 코드는 `#[cfg(test)]` 외 실행 경로로 올리지 않는다 |

**지금 당장 진행할 일:** `=, i., i:, e., E.`의 기본/변경 tolerance, Rank·cell/frame·boxed/sparse/empty 및 에러 순서에서 C oracle과 Rust reference 차이를 수집하고, J semantic model/IR 경계를 정리한다. 이미 확인한 `TCMPEQ` 경계 반례는 **의미론 수정이 필요한 검증 입력**이며, jsource의 `viavx2.c` 최적화 구현을 당장 따라 만들 근거가 아니다. M2의 일반 frontend 작업을 이 연구가 장기간 선점하지 않도록 한다.

**재개 조건:** 검증된 reference semantics + source/differential evidence + 명시적 target guard/fallback + M4 기준선 측정 + 상한 자원/비용 근거가 함께 있을 때만 실제 알고리즘 특수화를 검토한다. 이 조건을 충족하기 전 P.1의 8–11단계 작업을 **착수 대상에서 제외**하며, 5–7·12단계의 아직 검증되지 않은 항목을 완료 처리하지 않는다. 새 jsource C special-case마다 RustJ 고유 IR·executor branch를 만드는 설계는 금지한다.

**지연 최적화 재검토에서 드러난 기준선 혼합:** `eval_semantic_reference`는 캐시 사용을 끄지만 `index_ops::lookup`의 일반 exact scalar Physical search planner는 여전히 호출한다. 'reference path'라는 명칭이 최적화와 독립인 순차 oracle을 보장하지 않는다. P.3에서 M2 우선 과제로 분리한다.

###### P.1 구현 단계와 수용 기준 (계속 갱신)

| 순서 | 체크 항목 | RustJ 소유 계층·수용 기준 | 현재 상태 |
|---|---|---|---|
| 0 | [x] J 검색 family를 원본 이름·valence·의도에 따라 분류 | J Graph/semantic identity: dyadic `i.`/ `i:`/ `e.`와 monadic generate, dyadic `I.` interval, `E.` window find를 혼동하지 않음 | 코드·§N 정적 확인, full-J 미완료 |
| 1 | [x] 검색 의미와 실행 전략 분리 | A3 `SearchDescriptor` first/last/member/interval/self; `Operation.j_origin`, rank boundary, J equality를 보유하고 Physical table 정보는 배제; A3 schema 0.6/verifier | 구현·회귀 테스트 추가; **실행 미확인** |
| 2 | [x] 알고리즘 후보·가드 상태와 비용 선택기 분리 | `LoweringRegistry`의 Baseline/Guard/Proof/Unsupported, `physical.rs::plan_search_algorithm`의 `SearchWorkload/Choice`. Unknown→legal 금지. CPU exact scalar에 한정 | 구현·정적 확인; **벤치마크 미실시** |
| 3 | [x] 기본 검색 전략과 optional cache 준비 | CPU 순차, 좁은 정수 Direct, Hash, query-side Reverse, immutable-Arc per-Engine Prehash; direct Boolean `e.` 결과와 table allocation 실패 시 순차 fallback | `index_ops.rs`/runtime 구현, 아직 conformance 미통과 |
| 4 | [x] Tolerant equality 반례와 후보 포괄성 **연구 테스트 작성·연결** | `src/tolerant_search.rs`(테스트 전용, `#[cfg(test)]`)에서 비추이성, first/last, 지수 bucket ±1, ±0, NaN, ±Inf, subnormal 및 독립 **linear search index** 대조 | **테스트 코드만 작성**, 실행·C oracle 검증 미완료; 런타임 미연결 |
| 5 | [ ] J 비교 계약 구체화 | `ComparisonPolicy`: `!.ct`/runtime cct/version, float/complex/boxed/AxisSparse, exact build vs tolerant probe, rank/cell/frame, first/last representative, error/effect precedence를 source-linked witness로 확정 | 의미론적 proof 미완료 |
| 6 | [ ] Tolerant candidate-filter 완전성 증명 | 현재 fixed `kernels::near(a,b)`의 **t = 2^-44** 조건에서 부호·절대값 지수의 인접 bucket 탐색이 가능한 모든 일치 위치를 포함함을 증명. exact match는 +0/-0·동부호 Infinity 별도, NaN 제외. **원래 predicate로 모든 candidate 재확인·원래 source index로 최솟값/최댓값 선택** | 조건부 수학 논증·연구 코드; 일반 J equality의 증명은 아님 |
| 7 | [ ] runtime tolerance-guard + miss fallback과 prehash invalidation | C reference/다중 precision·fit/rank·동적 변경에 대한 guard; 실패하면 효과 이전 기존 순차 경로. 인덱스의 `cct`/argument backing identity/version과 lifetime을 검증 | 실행 미구현·미검증 |
| 8 | [ ] BQN 기반 작은 인자·Small-range 고도화 | 작은 인자별 순차 양방향/SIMD, byte/2-byte 직접 테이블, packed presence, sparse **table initialization**; CPU feature·메모리 상한·first/last 결과가 기존과 같을 때만 선택 | 알고리즘·비용 검증 대기 |
| 9 | [ ] 대규모 해시 충돌·캐시 개선 | one-shot workload에 맞는 open-addressing/linear-probing **대안**(현재 Rust `HashMap`은 당장 교체 금지); 충돌 계수, 해시/비교 adversarial guard, high-collision sorting/radix fallback, partitioning/캐시 동작 비교 | 성능·구현 대기 |
| 10 | [ ] BQN reverse/Prehash 선택과 비용 보정 | query/indexed 크기 비율, uniq cardinality, table 초기화·clear 비용, cache residency, reuse 횟수, persistent vs one-shot 준비비를 `ResourceEstimate`와 `CostEstimate`로 분리. hard memory guard가 비용보다 우선 | fixed heuristic만 있음 |
| 11 | [ ] `i.` 외 가족으로 공통 알고리즘 선택 패턴 이식 | Nub/Key/`I.@e.`의 결과 모드와 순서 계약을 확장한 후, Reduce/GroupBy/Grade/Contract는 **각자 독립 semantic proof**를 둔 generic `AlgorithmCandidate`/SelectionPlan을 사용 | interface prototype 일부만 존재 |
| 12 | [ ] Release gate: Rust/C differential + 비용 실측 | 기본/portable Rust tests, J C 두 경로의 first/last/NaN/±0/empty/rank/boxed/sparse/`!.ct`/error, 참조 순차 vs 모든 활성 fast path, 무작위·충돌 유도 벤치 및 memory/time 회귀. **측정 전 threshold 고정 최적화 주장 금지** | 수행하지 않음 |

###### P.2 독립 검증 루프 (반복·결과 기록)

- [ ] **A — source-first 독립 검토:** Hui 논문 → 고정 jsource dispatch(`vi.c`)/tolerance(`viavx2.c`)/prehash 모드 → J 오류·Rank·Fit 계약을 추출. BQN 코드를 의미론적 oracle로 대체하지 않음.
- [ ] **B — mathematics-first 독립 검토:** `a == b || finite ∧ |a-b| ≤ t·max(|a|,|b|)`의 비추이성과 exponent bucket 포괄성, IEEE-754 rounding/underflow/overflow 및 대표 인덱스 순서를 **해시 코드와 별도** 증명. \(0 ≤ t < 1/2\) 같은 가정을 witness로 기록.
- [ ] **C — reference-first 독립 검토:** 모든 query에 대해 J C oracle, 현재 Rust 순차 near oracle, 후보 인덱스 검색을 3방향 비교한다. **Rust near와 일치한다**는 사실만으로 J C 일치가 성립하지 않음.
- [ ] **D — compiler-boundary 독립 검토:** Graph candidate → A3 descriptor/verifier → Registry dynamic legality → Physical cost/resource → runtime guard/fallback을 파일별로 읽고 stale origin, `e.` 인자 방향, GPU/Interval/Tolerant 미승인을 다시 확인한다.
- [ ] **E — adversarial/performance 독립 검토:** high collision, all-identical, near-chain, sorted/unsorted, tiny/massive range, no-match majority, cache-pressure, table build/miss/allocation과 source-order stable output을 계측. speed win이 없는 방안은 보류·폐기한다.

**반복 기준:** 한 번의 점검에서 실패 사례가 나오면 관련 코드·proof/fallback과 회귀 테스트를 수정하고 A~E 중 직접 관련되지 않은 관점에서도 다시 검증한다. [ ]를 [x]로 바꿀 때는 **실행 명령, 환경, 테스트 개수/결과, 비교 원본의 commit 또는 측정 데이터**를 같은 절에 기록한다. Linux 전용 milestone은 생성하지 않는다. CI를 실행하지 않았으면 통과라고 기록하지 않는다.

###### P.3 현재 known gaps / 바로 다음 순서 — M2 우선으로 재정렬 (2026-10-06)

1. **M2 frontend 수렴이 우선:** word formation → enqueue → parser/name/POS/derived entity의 실제 미해결 의미론과 회귀 검증을 먼저 마무리한다. 검색 최적화 연구는 frontend 작업을 선점하지 않는다.
2. **독립적인 의미론 기준선 복구:** `eval_semantic_reference(...)`는 `pooled=false`지만, `kernels::dyad("i."/"i:"/"e.")`가 `index_ops::lookup(..., None)`을 호출하고 그 내부에서 **여전히 `optional_exact_scalar_index` → `plan_search_algorithm`**을 실행한다. 따라서 지금의 `semantic reference`는 **prehash를 피할 뿐 Direct/IndexedHash/ReverseQueryHash 선택에서 독립적이지 않다**. M2/M3 기준선으로 사용할 때는 순차·명세 중심 검색과 optional physical strategy를 실제로 분리하고 동일 J 입력에 두 경로가 일치하는지 검사해야 한다. 코드 변경 및 테스트 통과는 **아직 미실행**.
3. **J 언어 의미를 독립 검증:** `=`, `i.`, `i:`, `e.`, `E.`, `!.ct`, `9!:18/9!:19`의 원본 J C 결과를 수집하고 기본 비교식 `near`와 `TCMPEQ` 경계 차이, rank/cell/frame/empty/boxed/sparse/error를 참조 실행과 대조한다. 이 작업은 특수 해시 알고리즘 복제가 아니라 의미론 정확성 검증이다.
4. **Graph/A3/Registry 보존, Physical commitment 연기:** source topology·comparison intent·provenance·unknown proof facts와 실행 합법성 보고는 보존한다. P.1의 기존 Int/Bool 특수 실행은 현재 구현물로만 취급하고 후속 확장을 동결한다. BQN SIMD/작은 direct table/radix/충돌 전략, tolerant bucket 활성화, 휴리스틱 튜닝은 M2/M3 의미론 안정화와 M4 기준 CPU 실행·실측 뒤에만 재검토한다. `TolerantNeighborHash`는 `NeedsSemanticProof` 상태를 유지하며 Runtime/Physical 실행 경로에 올리지 않는다.

**검증 정책:** semantic-reference와 optimized 경로가 동일한 내부 planner를 공유하는 동안 두 결과의 일치는 독립적인 최적화 검증 증거가 아니다. 추후 확인된 C oracle 및 순차 semantics 기준선과의 **3방향 비교**, 오류/효과 순서, fallback, 실제 비용 측정을 별도 gate로 둔다.

###### P.4 독립 검토 기록 #1 — 제한된 float search 후보 완전성 (2026-10-06)

**의미론·수학 관점 (코드 구현과 독립):** 현재 RustJ `kernels::near`는 `a == b || finite(a,b) && |a-b| <= t * max(|a|,|b|)`이고 `t = 2^-44`. `a,b`가 0이 아닌 유한수이며 비교가 성공했다면 서로 부호가 다를 수 없다(반대 부호이면 차이는 두 절대값의 합으로 `t<1` 조건을 위반). `M=max(|a|,|b|)`, `m=min(|a|,|b|)`이면 `M-m <= t M`이므로 `m >= (1-t)M > M/2`(`t<1/2`). 따라서 이진 지수 `floor(log2 |a|)`와 `floor(log2 |b|)` 차이는 최대 1. **query와 동일 부호인 지수 `e-1,e,e+1` bucket은 성공 가능한 source 항목을 누락하지 않는 필요조건**이다. 결과는 bucket 대표값이 아니라 **모든 원본 index를 취합한 뒤 원래 `near`로 재검사**하고, `i.`은 최소 원본 index, `i:`는 최대 원본 index를 택한다. 비추이적 연쇄 `a≈b≈c`, `a≉c`에서 앞의 원소 하나로 bucket을 대표시키는 방식은 불허한다. +0/-0, 동부호 ±Infinity는 exact equality로 별도 취급하고 NaN은 불일치한다.

**수치 반증 탐색 (Rust/C 테스트 아님):** 동일한 IEEE-754 `f64` 의미를 JavaScript 숫자 모델로 모사해 **215개 값의 1,160개 일치 쌍**을 조사했다. 지수 경계를 넘는 일치 쌍은 132개, 같은 부호·지수차 ≤1 필요조건을 어긴 사례는 **0개**였다. `(1,1+0.75t,1+1.5t)`에서 근사 동등성은 `true,true,false`로 비추이성을 확인했다. **표본 확인은 일반 수학 증명·Rust 테스트 실행·J C 비교·성능 보증을 대신하지 않는다.** 이 절에서 확인한 것은 고정된 현재 Rust `near`에만 적용되며 J `!.ct`/복소수/박스/Rank 등에는 자동 일반화하지 않는다.

**계층·소스 독립 검토:** BQN 원문은 작은 인자 SIMD, sparse **table** initialization, reverse lookup과 large-input partitioning의 알고리즘 참고로만 사용한다. 실제 C의 tolerant 해시(`viavx2.c`)는 두 인접 tolerance 구간과 exact insert / tolerant probe를 활용하므로, 연구용 **부호·지수 3-bucket** 설계를 jsource의 bitmask 알고리즘과 동일하다고 주장하지 않는다. A3/Registry/Physical/Runtime/문서의 provenance/guard/비승격·체크리스트 항목 **13개 정적 확인을 통과**했다. `src/tolerant_search.rs`는 **`#[cfg(test)]` 등록만** 되어 있으며 실제 Runtime을 호출하지 않는다. **Cargo/CI/C differential/benchmark를 수행하지 않았으므로 P.1의 실행 수용 게이트는 계속 [ ]로 둔다.**


###### P.5 독립 검토 기록 #2 — IEEE-754 인접 비트·원본 순서 보강 (2026-10-06)

**이번 변경과 완료 범위.** 연구 전용 `src/tolerant_search.rs`에 `fixed_near_candidate_filter_covers_ieee_neighbor_words_and_first_last` 회귀 테스트를 추가했다. subnormal/normal, 1·2의 거듭제곱 경계, 최대 유한수/Infinity/NaN, 부호별 인접 비트 패턴과 고정 seed 2,048개 임의 `f64` 비트 패턴을 포함한다. 각 query에 대해 (a) **원본 전체 배열**을 순차 `kernels::near` 비교한 참조 결과가 candidate 집합에 포함되는지, (b) 모든 candidate index가 유일·원본 순서대로 정렬되는지, (c) First/Last 반환값이 순차 참조와 같은지를 따로 확인하도록 작성했다. 이 테스트는 `#[cfg(test)]`로만 컴파일되는 기존 실험 모듈에 있으며 **실제 Rust 테스트 실행 완료를 의미하지 않는다**.

**별도 언어로 수행한 수치 반증 탐색.** 소스 버킷 함수와 독립적으로 작성한 Python IEEE-754 `double` 모델에 원본 원소 **4,224개**, query **310개**, 가능한 쌍 **1,309,440개**를 대입했다. `near` 일치 **674개** 중 서로 다른 버킷에 속한 쌍 **143개**를 포함하여 후보 누락 **0개**였으며, 전 query에서 순차 First/Last와 후보 기반 결과가 일치했다. 근사비교 비추이성 검산은 `true,true,false`였다. 이 결과는 **Python 모델의 유한 표본 검사**이며 Rust `cargo test`, J C oracle, 모든 `f64` 조합의 기계적 완전성 증명, 성능 측정이 아니다.

**독립적인 증명·안전성 검토.** `t=2^-44`는 `1/2`보다 압도적으로 작으므로 성공하는 비영 유한수 두 값은 부호가 같고 큰 절대값 `M`에 대해 작은 절대값 `m>(1/2)M`이어야 한다. 따라서 부호+이진 지수 차이 최대 1이라는 **필요조건만** bucket 필터에 사용한다. 비교에 사용한 실제 floating-point `near`를 후보마다 다시 평가하므로 거짓양성은 허용되지만 거짓음성은 허용되지 않는다. 계산된 `t*M`이 subnormal에서 0으로 underflow하면 일치 범위는 더 좁아질 뿐이며, `a==b` 경로는 signed zero와 Infinity를 별도 수용한다. 이 설명은 **현재 고정 함수**의 증명 개요이고, `!.ct`·동적 cct·복소수·박스·Sparse·Rank·셀 비교로 확대하는 정당화가 아니다. 단일 대표 원소 해싱과 tolerance 전역 cache는 여전히 금지한다.

**검증 상태 및 다음 게이트.** 이 환경에서 Rust/Cargo 실행 환경을 확보하지 못했으므로 `cargo test tolerant_search`, `cargo test --features portable tolerant_search`, `cargo fmt --check`, `cargo clippy`, jsource C 비교 및 성능 측정은 **미실행**으로 유지한다. P.1의 5–7, 12와 P.2의 C/E는 완료 처리하지 않는다. 다음 증거는 서로 다른 `!.ct` 설정, float/complex/boxed/Rank·empty 경계에 대한 **실제 C 결과**와 Rust 결과 차등, tolerance policy/version witness, 그리고 guard miss 시 원래 순차 의미를 보존하는 조건이다. `TolerantNeighborHash`의 `NeedsSemanticProof`는 변경하지 않았고 Physical 선택·런타임 실행에 연결하지 않았다.


###### P.6 원본 J 비교 정책·증명 경계 — source-first 독립 검토 (2026-10-06)

**확인 가능한 J 의미론 원본:** J Dictionary의 [Equal (=)](https://www.jsoftware.com/help/dictionary/d000.htm)은 유한 float/complex 비교 기본 허용오차를 `2^-44`로 설명하고 `!.t` 재정의를 명시한다. [Fit (!.)](https://www.jsoftware.com/docs/help806/dictionary/d411.htm)에는 `i.`, `i:`, `e.`, `E.` 등의 fit 대상이 열거되어 있고, [전역 매개변수 9!:18/9!:19](https://www.jsoftware.com/help/dictionary/dx009.htm)는 tolerance 조회/변경 계약이다. 즉 기본 값이 우연히 현재 `kernels::near`의 상수와 일치해도 **실행 시 적용되는 비교 정책의 동일성**을 추정할 수 없다.

**확인한 pinned C 알고리즘:** [jsource `viavx2.c`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx2.c)는 `TFINDXYT`에서 원본 버킷과 인접 tolerance 구간을 모두 읽고, `IOFT`에서 `PUSHCCT(1.0)`로 table을 **불허용오차 비교로 구성한 다음** 검색 시 허용오차를 사용한다. `IIDOT`·`IICO`는 서로 다른 representative selection(최소/최대 원본 위치)을 갖는다. 이것은 **인접 구간의 후보를 모두 확인해야 한다는 source 증거**이지, 연구 중인 3개 이진 지수 bucket 구현과 동일하다는 증거가 아니다. 단일 approximate-equivalence 대표로 중복 항목을 삭제하면 `a≈b`, `b≈c`, `a≉c`에서 `[a,b] i. c`의 유일한 일치 위치를 잃는다. 이 부정 사례를 별도 연구 테스트에 추가했다.

**제안하는 `ComparisonPolicy` 증명 입력(설계 전용, 미구현):** (1) 호출할 때 해석된 J primitive/valence와 `!.t` override 유무, (2) 현재 `9!:19` 정책 값 및 의미론적 버전/소유 경계(호출 전/후 변경 포함), (3) argument dtype과 rank/cell/frame, boxed/sparse 구조, (4) 사용한 equality predicate의 정체성과 NaN/±0/Infinity/복소수 규칙, (5) 허용되는 tolerance 범위와 검증 witness, (6) source 값·원본 인덱스·First/Last/Presence 결과 모드, (7) 관찰 가능한 오류·효과 경계 및 순차 fallback을 기록해야 한다. 정책·입력·임시 테이블 lifetime이 바뀌면 prepared index를 재사용하지 않는다. A3 `SearchDescriptor`에는 **의미·정책 출처만** 보존하고, 테이블 구조와 비용은 Physical 소유다.

**실행 승인 필요조건(미승인 유지):** CPU finite scalar float 검색에서조차 **비교 정책을 실제로 witness/guard**하고, 모든 가능 일치 위치가 후보로 보존된다는 proof가 있으며, 후보마다 정확히 동일한 semantic comparator로 재검사하고, 최소/최대 원본 위치 및 no-match sentinel을 유지해야 한다. 가드 실패는 observable effect 전에 일반 순차 검색으로 돌아가야 한다. `NaN`/무한대/underflow, 혼합 타입, rank/cell, boxed/sparse, C와 Rust 기본 비교 차이를 회피하는 **명시적 지원 범위**가 필요하다. 지금 단계에서 proof는 고정 Rust `near`에만 제한되고, C differential·동적 policy와 `!.t`의 등가는 **미검증**이다. 따라서 `LoweringRegistry`의 `TolerantNeighborHash=NeedsSemanticProof`와 테스트 전용 배선을 유지한다.


###### P.7 고정 비교 정책 스냅샷 구현 — dynamic J CCT 미지원 경계 (2026-10-06)

> **역사적 상태: 아래 `FixedRustNearV0` 실행 설명은 첫 구현 당시 기록이다. 현재 기본 CPU 실행은 §P.9의 `PinnedJDefaultCctV0`로 교체됐다.**

**구현:** `src/comparison_policy.rs`에 내부 `ComparisonPolicySnapshot`과 유일하게 구성 가능한 `FixedRustNearV0` 식별자를 둔다. 이 스냅샷은 현재 `kernels::near`에서 사용하던 `a == b || finite(a,b) && |a-b| <= 2^-44 max(|a|,|b|)` 식을 그대로 소유한다. `kernels::near`는 이를 호출하도록 위임하며, `src/index_ops.rs::lookup`과 `find`는 **검색 호출당 한 번** 동일 정책을 포착해 각각 `i.`/`i:`/`e.`, `E.`의 atom comparisons에 전달한다. 외부 `expansion.rs` 사용을 위한 `atom_eq` 래퍼는 그대로 유지한다. Graph IR·A3 `SearchDescriptor`·Physical index/table 선택을 수정하지 않았고, float 검색은 여전히 **원본 순차 경로**를 사용한다. Int/Bool exact-only prehash는 float tolerance와 독립이며 변경하지 않았다.

**새 회귀 테스트(작성만, 실행 미확인):** `src/comparison_policy.rs`에서 비추이 연쇄, ±0, NaN, ±Infinity, 최소 subnormal 대 zero를 검사한다. `src/index_ops.rs`의 `fixed_float_policy_preserves_first_last_membership_and_find`는 다중 일치·근사적 비추이성·missing sentinel 및 4가지 검색 결과 `i.`·`i:`·`e.`·`E.`를 독립적으로 확인한다. 기존 연구 harness `src/tolerant_search.rs`는 추가한 정책 모듈과 별개로 test-only다. 입력형 불변성과 정책 동일성은 코드 검토로 확인하되 **Cargo fmt/test/clippy, 실제 jsource 실행, differential/benchmark는 미실행**이다.

**설계상 의미:** '비교 정책'을 실행 코드의 명시적 입력으로 캡처하는 첫 단계이고 **J의 `!.t`/전역 `9!:19`를 구현한 것이 아니다**. `FixedRustNearV0`는 현재 Rust CPU 함수의 정체성일 뿐, jsource `cct`-based 구현 동등성 witness가 아니다. 다음 기능을 활성화하려면 `Fit`-derived verb/동적 설정의 semantic ownership, 실제 호출 시점 policy identity + epoch, guarded supported-range proof, 전체 rank/cell·type·error 조건, 적절한 cache key/invalidation 및 독립 J C differential을 각각 충족해야 한다. 자동 TolerantNeighborHash 후보 선택, prepared float hash, LLVM/GPU 경로는 계속 금지한다. P.1 단계 5–7/12는 **[ ] 그대로**다.


###### P.8 jsource 기본 비교식의 경계 불일치 — 미해결 의미론 차이 발견 (2026-10-06)

> **역사적 발견: 아래 8개 경계 불일치는 §P.9 실행 수정 이전의 상태다. 고정 기본 CCT는 이후 3방향 검증에서 수렴했지만 동적 CCT/Fit 구현을 뜻하지 않는다.**

**독립 source-first 재감사:** 고정한 jsource [`jsrc/vcomp.h::TCMPEQ`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vcomp.h), [`jsrc/i.c` 초기화](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/i.c), [`jsrc/viavx.h::jeqd`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/viavx.h)를 추가 확인했다. 기본 `jt->cct=1.0-FUZZ`, `FUZZ=2^-44`이며, C 소스의 단일 float 비교식은 `(a > cct*b) != (b <= cct*a)`다. 검색 계열의 tolerant probe 역시 `jeqd`를 통해 `TCMPEQ`를 호출한다. [`jsrc/xa.c`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/xa.c)에서 `9!:19` 입력은 `0 <= t <= 5.820766091e-11`을 검사하고 `cct=1.0-t`로 저장한다. 유효 범위와 변경 수명은 이 원본에 맞춰야 한다.

**소스 식의 IEEE-754 모델을 통한 실제 반례(실행한 J C 바이너리 아님):** `t=2^-44`, `a=1.0`, `b=1.0-t`이면 현재 RustJ `near`의 `|a-b| <= t*max(...)`는 **true**, 위 jsource C macro를 이진64 연산으로 모델링한 식은 **false**다. `b=1.0+t`에서도 같은 불일치가 나타난다. 이는 반올림·경계의 **strict-vs-inclusive** 차이여서, 수학적으로 같은 허용오차 폭을 논의한 것만으로 기계 수준의 결과 동등성이 보장되지 않는다는 구체적 증거다. 별도 Python IEEE-754 반증 탐색은 **의도적으로 허용오차 경계에 집중한 120,000쌍에서 19,968개의 모델 불일치**를 찾았다. 이 빈도는 일반 입력의 J-vs-Rust mismatch rate가 아니며 C 원본 실행 결과도 아니다. 부호·크기·subnormal·다양한 target CPU는 별도 검증해야 한다.

**독립 C 재현(전체 J 실행 아님):** pinned `TCMPEQ`의 식을 포함한 작은 C 프로그램을 Debian GCC **14.2.0**, `cc -std=c11 -O0` 및 `-O2`로 각각 컴파일·실행했다. 두 빌드 모두 `(a,b)=(1,1-2^-44)` 및 `(1,1+2^-44)`에서 기존 RustJ 수식 모델은 **true**, upstream C 매크로 수식은 **false**를 반환했다. ±0·Infinity·NaN 검산 사례도 두 빌드에서 일치했다. [`reports/cct-macro-boundary-probe.json`](reports/cct-macro-boundary-probe.json)에 입력·컴파일러·결과·미실행 범위를 기록했다. **Rust 컴파일·전체 jsource 프로그램 실행·J oracle 차등 테스트는 여전히 하지 못했다**(`cargo`, `rustc`, J 인터프리터가 현재 환경에 없음). 이 자료는 소스 매크로 재현이지 J 엔진 통합 검증이 아니다.

**코드·승인 상태:** `src/comparison_policy.rs`의 테스트 전용 `source_cct_macro_model` 및 `jsource_cct_macro_model_exposes_fixed_near_boundary_gap`이 이 반례를 기계적 회귀 입력으로 기록했다. `FixedRustNearV0`의 기존 실행 의미는 **의도적으로 변경하지 않았다**. 실제 J C binary/엔진을 사용한 `=, i., i:, e., E.` 경계 검증과 dtype/rank/fit·error 차이를 확정한 뒤, 공통 comparator를 source-equivalent로 교체할지 결정해야 한다. 임의로 float hash를 활성화하거나 source model을 완전한 J runtime oracle로 선언하지 않는다. **P.1 #5 및 #12는 여전히 미완료이며, 이전 P.4의 수학적 후보-상계 증명은 *구 Rust fixed-near comparator*에 한정된다.**



###### P.9 기본 J CCT 비교 정책으로 실행 경로 정렬 — FW-03 부분 증거 (2026-10-06)

**변경 이유와 최초 관찰.** `P.8`의 반례를 고정 `jsource@13994ffa1ed5f06f79fad6e9822a7ed2d29b1528` C binary에서 직접 확인했다. [FW-03 진단 CI 37445080131](https://github.com/yunskim/RustJ/actions/runs/37445080131)은 j64·j64avx2 × Rust default·portable 네 조합 각각 **48개 중 40 일치 / 8 C-vs-Rust 불일치 / Rust baseline 대 optimized 불일치 0**을 기록했다. 8개는 `1.0 ± 2^-44`의 `=`, `i.`, `i:`, `e.`(상·하한 × 네 연산)였다. 원본 삼방향 값·pin·binary 해시는 CI artifact `reports/tolerance-audit-*.json`에 보존한다.

**최소 구현.** `src/comparison_policy.rs`에 `PinnedJDefaultCctV0`을 추가하고 `float_equal(a,b) := (a > cct*b) != (b <= cct*a)`, `cct = 1 - 2^-44`로 고정했다. `src/kernels.rs::near`(비교), `src/index_ops.rs`(최적화 검색), `src/search_reference.rs`(독립 순차 검색)에 각각 사용한다. 이전 `FixedRustNearV0`은 `#[cfg(test)]` 회귀 witness로만 보존된다. 원본 J Graph/A3, cost model, exact Int/Bool hash 선택, float 순차 검색 정책은 유지한다.

**실제 실행 검증.** [Linux milestone CI 37445891526](https://github.com/yunskim/RustJ/actions/runs/37445891526)에서 고정 C J j64·j64avx2 × Rust default·portable 네 조합 각각 **48/48 일치, 총 192/192, Rust baseline 대 optimized 불일치 0**. `tools/tolerance_audit.py`는 기존 8개 불일치를 조용히 통과시키지 않고 `previously_known_gaps_now_matching`으로 분류하며, 새 C-vs-Rust 불일치 또는 두 Rust 실행 경로 불일치가 생기면 CI 실패로 보고한다. 이는 **기본 CCT·dense scalar 경계 표본**의 수용 증거이며 full J 동등성 증명은 아니다.

**잔여 의무.** `9!:19` 동적 전역 설정·`!.t` Fit, boxed/sparse·복소수·Rank/Cell·오류 순서 및 J numeric 타입 전체는 검증/구현이 남았다. `FW-03`, `FW-04`, `JX-04`, `P.1 #5/#12`는 **[ ]** 유지. P.4의 *구 Rust near* 한정 tolerant hashing 후보·수학적 증명을 새 default J CCT 해시 정당화로 승격하지 않는다. 자동 tolerant hash, float prepared hash, GPU/LLVM 경로도 아직 미허가다.

###### P.10 Rank zero-frame fill-cell 실행·검증 — FW-04 부분 증거 (2026-10-06)

**jsource 의미론과 문제 재현.** 고정 [`jsrc/cr.c`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c)의 `jtrank1ex`·`jtrank2ex`는 frame의 셀 수가 0이면 type-correct fill cell 또는 반복되지 않는 첫 실제 cell을 이용해 operand를 평가하고, 그 결과의 type·shape로 빈 배열을 조립한다. 계산 오류 발생 시 `WITHDEBUGOFF`/non-exigent error fallback 등의 추가 계약도 있다. 이전 RustJ는 `(i.0 3) (i."1 1) (i.0 3)`을 `Unsupported`로 거부했고, [기준 Rank 진단 CI 37446732381](https://github.com/yunskim/RustJ/actions/runs/37446732381)에서 각 변형 14건 중 **13 일치 / 1 이 의미 불일치**를 확인했다.

**구현 범위.** `src/value.rs::rank_fill_cell`과 `empty_rank_result`에서 supported dense Bool/Int/Float/Char 입력의 첫 실제 cell 또는 type-correct fill cell, 그리고 결과 type·cell shape + 빈 frame 조립을 제공한다. `src/logical_executor.rs::apply_ranked`의 dyad/monad 및 `src/kernels.rs::ranked_dyad_ranks`·`ranked`가 공통 Rank/CellApply 요구를 사용한다. 검색 전용 Graph node·임의 물리 전략·optimizer 승인은 추가하지 않았다. **`primitive_fill` 실제 primitive witness가 있는 경우에만 zero-frame 자동 fill 평가**를 허용한다. 사용자 정의/동적 동사의 empty-frame 호출은 원본 J에서 효과를 발생시킬 수 있으므로 `Unsupported`로 명시하고, 증거 없는 부작용 speculative 실행을 금지한다. Boxed/sparse fill과 computational error fallback도 미지원이다. `requires_empty_frame_prototype` 및 정적 `Facts::default`는 지우지 않고 보수적으로 보존한다.

**실제 3방향 증거.** [Linux CI 37449102885](https://github.com/yunskim/RustJ/actions/runs/37449102885)의 check·`j64`·`j64avx2` × Rust default/portable가 모두 성공. `python3 tools/ranked_search_audit.py --reference-revision 13994ffa1ed5f06f79fad6e9822a7ed2d29b1528 --report reports/ranked-search-audit-<variant>-<backend>.json` 명령의 20개 fixture에서 **각 20/20, 총 80/80 C J · 독립 Rust semantic reference · Rust 최적화 실행 일치, 불일치 0**. 빈 frame i./i:/e., 문자/float, primitive 덧셈·reduction, scalar/row/broadcast와 frame mismatch를 포함한다. [Basis compile probe 37449102980](https://github.com/yunskim/RustJ/actions/runs/37449102980)도 성공하여 Rust tests(기본/portable)와 Clippy 등의 검사 근거를 별도 제공한다. 새 의미 지원을 반영해 `tests/analysis.rs`·`tests/semantics.rs`의 오래된 Unsupported 기대를 정정했고, 사용자 정의 동사의 빈 frame 효과 미검증 경계는 `tests/empty_scope.rs`로 계속 보호한다.

**미완료 / 다음 게이트.** J의 일반적인 boxed·sparse filler, 계산 오류 억제·회복, 효과/이름·오류 순서, mixed-type cell·heterogeneous cell-result padding 및 rank-implicit-loop 전체는 이 20개 통과로 증명되지 않는다. `FW-04`·`JX-04` 및 Rank 관련 상위 게이트는 **[ ] 유지**한다. 다음 변경은 C oracle에 부정 사례를 먼저 고정하고, 효과/자원 한계를 보존하는 의미론 계약을 독립 검증한 뒤 수용한다.

<a id="rank-cellapply-followups"></a>

###### P.11 Rank/CellApply 후속 이행 체크리스트 — FW-04·JX-04 세부 실행 계획 (2026-10-06)

**목적·수용 경계.** §P.10에서 구현한 zero-frame Rank의 dense subset을 출발점으로, J의 일반 Rank/CellApply 의미론까지 검증·확장할 작업표다. 상위 §O.5 FW-04와 §Q JX-04의 **하위 실행 목록**이며 별도 최적화 허가 체계가 아니다. **현황: 범위 한정 5/12 완료, FW-04·JX-04는 [ ] 유지.** 고정된 [jsource@13994ffa](https://github.com/jsoftware/jsource/tree/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc)의 cr.c (jtrank1ex/jtrank2ex)와 result.h를 기준으로 한다. 아래 완료 항목은 지원 subset에 한정한다.

| ID / 단계 | 체크 | 구현 소유자·완료 판정 | 반드시 보존할 검증 증거·부정 사례 |
|---|---|---|---|
| RK-01 / A·M2 | [x] **고정 C 원본의 빈 frame 계약 조사** | cr.c의 fill-cell/첫 실제 cell 평가 → 결과 타입·shape 조립 및 오류 fallback 차이를 §P.10에 명시 | [기준 CI 37446732381](https://github.com/yunskim/RustJ/actions/runs/37446732381) 4환경 각 13/14 일치, 빈 frame 1건 불일치 기록(수정 전) |
| RK-02 / A·M2 | [x] **Dense fill-cell 값·타입·shape 구성** | src/value.rs의 rank_fill_cell/empty_rank_result: Bool·Int·Float·Char 타입별 fill, 비어 있지 않은 인자의 첫 실제 cell, 빈 결과 dtype 유지 | 지원된 타입/shape 및 boxed 거부 단위 테스트. Boxed/sparse의 임의 0 치환 금지 |
| RK-03 / A·M2 | [x] **일반 Rank monad/dyad 배선** | src/logical_executor.rs, src/kernels.rs, src/runtime.rs: 검색 전용 예외가 아닌 공통 CellApply 실행. 정적 Facts와 requires_empty_frame_prototype를 보수적으로 유지 | tests/semantic.rs, tests/analysis.rs; 결과 shape·데이터 타입 검사 |
| RK-04 / A·M2 | [x] **미검증 사용자 효과 자동 실행 차단** | runtime primitive witness로만 빈 frame fill 실행 허가. 동적·명시 정의 본문은 독립 효과 계약 없이는 Unsupported | tests/empty_scope.rs의 카운터·바인딩·정의 본문 미호출 검사. 실제 사용자 정의 Rank 지원 완료 아님 |
| RK-05 / A·FW-04 | [x] **현 지원 subset 독립 3경로 회귀** | tools/ranked_search_audit.py, 테스트와 Linux CI에서 C·Rust 순차 기준·Rust 최적화 실행을 별도로 비교 | [Linux CI 37449102885](https://github.com/yunskim/RustJ/actions/runs/37449102885) j64·j64avx2 × default·portable 각 20/20, 총 80/80; [Basis probe 37449102980](https://github.com/yunskim/RustJ/actions/runs/37449102980) 성공 |
| RK-06 / B·M2 | [x] **빈 frame·Rank 적대 입력 확대** | tools/ranked_search_audit.py·tests/semantic.rs: frame 선두/중간 0, 한쪽만 빈 배열, 길이 불일치, 음수·과대 rank, 타입 불일치, 중첩 Rank의 실행/오류 비교 | 원본 C 사례 먼저 고정, 결과값·오류 종류·Rust Unsupported를 별도 기록. 새 입력을 4환경 3경로로 검증  **진단 증거(2026-10-06; [CI 37452153562](https://github.com/yunskim/RustJ/actions/runs/37452153562)):** pinned C j64/j64avx2 × Rust default/portable 4조합 각각 24개 중 22개 일치, 2개 `rust_semantic_mismatch`(`empty_type_mismatch`, `nested_rank_empty`). 전체 green은 이 적대 corpus가 탐색/비수용 게이트이기 때문. [`45af152`](https://github.com/yunskim/RustJ/commit/45af1526c41e33d9a5b3fda8bf7bfd5bf92d9b4e) 및 [`59bb6d8`](https://github.com/yunskim/RustJ/commit/59bb6d8adff86db0fd8c1d43bb41cc64bbc6dc52)에서 해당 3경로 관찰값을 로그에 노출하고 테스트를 추가. **당시에는 2건 미해결이었으며 아래 32건 엄격 재검증으로 해당 범위를 수용했다.**  **부분 수정·CI 검증(2026-10-06):** [`4d5f903`](https://github.com/yunskim/RustJ/commit/4d5f90348a91ece4c15139d0ae56affb1ade68c6)·[`edad61b`](https://github.com/yunskim/RustJ/commit/edad61bc17ebeb46198f1d1f050bcb9bc95f0a42)에서 *원시 동사→Rank→Rank*의 제한된 값 전용 fill-cell 증거를 독립 A3/Rust 실행기 양쪽에 반영; [`a4c5953`](https://github.com/yunskim/RustJ/commit/a4c5953349ea8fba34df7bdcaed3e683ee773b01)·[`f074620`](https://github.com/yunskim/RustJ/commit/f074620e6317e5f86ffb425f07fba00a00847cdb) 정상·사용자 효과 차단 테스트. [Linux CI 37453410364](https://github.com/yunskim/RustJ/actions/runs/37453410364) check + 4 reference jobs **5/5 성공**; 기존 24개 적대 corpus는 각 조합 **23 pass / 1 `rust_semantic_mismatch`(`empty_type_mismatch`)**로 감소. 중첩 Rank 1건 해결, **당시 RK-06 [ ]였으며 이후 32건을 별도 검증했다. Rank 전체·ZF-IR-03 증명 완료를 뜻하지 않는다.** |
| RK-07 / B·M2 — **다음 작업** | [ ] **계산 오류 억제·재시도·오류 우선순위** | cr.c의 non-exigent 오류→fill 결과 대체, exigent 오류 전달, 타입 불일치 재시도의 조건을 분리; src/error.rs 및 Rank executor 계약 갱신 | 계산 오류, 타입 충돌, overflow, shape 오류의 원본 동작·오류 순서 고정. 80/80에 포함되지 않은 범위 |
| RK-08 / B·M3·FW-07 | [ ] **사용자 정의 동사·효과·동적 이름** | src/runtime.rs에서 동적 이름 조회 시점, fill 동사 실행 횟수, 부작용, 실패 후 재실행 가능성 및 guarded fallback을 정의 | C oracle의 카운터 증가·이름 변경·nested user verb·오류 후 상태 비교. 사전 증명 없는 speculative 실행 금지 유지 |
| RK-09 / B·M3 | [ ] **boxed·sparse fill/prototype** | src/value.rs·src/storage.rs·src/sparse.rs: box 내부 fill, sparse 축·fill 값과 atom type·shape 보존 | 타입별 C 결과와 비교; 임의 dense/0으로 일반화하거나 미지원 실행을 통과로 세지 않음 |
| RK-10 / B·M3 | [ ] **이질적 결과 cell·타입 승격·padding** | src/assembly.rs·src/logical_executor.rs·src/kernels.rs: result.h 기준 혼합 dtype·shape, fill/padding, 전체 cell 평가 후 오류 순서를 분리 | mixed result shape, char/numeric, empty cells, size mismatch, error precedence를 C·두 Rust 경로 비교 |
| RK-11 / C·M3 | [ ] **Rank + 파생 동사/implicit loop 연동** | semantic/Logical/Graph의 원래 modifier 관계와 rank/cell/frame facts 보존; fork/hook/@:/중첩 rank·late name은 실행 가능성과 별개 | negative/oversized rank, modifier composition, frame 반복, late binding의 3경로 테스트. Graph 후보가 승인된 실행으로 자동 변환되지 않음. **빈 frame과 빈 cell의 Graph 의미론·회귀 하위 체크리스트 §P.12 ZF-IR-01~04 참조; RK-11 전체 미완료.** |
| RK-12 / D·FW-04/JX-04 | [ ] **지원 범위·수용 증거 최종 대조** | RK-06~11 각각에 J 기준·baseline 독립성·Guard·효과/오류·자원 한계 증거를 연결하고 미지원 범위를 명시 | pinned C commit·명령·4조합 CI·cases/실패 분류·reports JSON을 남긴 뒤 해당 단계의 완료 여부를 개별 판정. 상위 FW-04/JX-04는 다른 의무가 남으면 [ ] |

**반복 작업 순서와 상태 갱신 규칙.** RK-06 → RK-07 → RK-08 → RK-09 → RK-10 → RK-11 → RK-12로 진행한다. 매 작업은 ① C 원본의 정상/부정 입력부터 고정 ② C / Rust 독립 semantic reference / Rust 실행 차이를 세 범주로 분류 ③ 가장 작은 공통 의미론 수정 ④ Rust default·portable fmt/clippy/test와 C j64·j64avx2 차분 실행 ⑤ CI 링크·수치·커밋·불일치를 해당 RK 행에 기록한 뒤에만 [x] 처리. **Linux CI 통과만으로 미지원 Rank를 전체 합격 처리하지 않으며** 부정 사례와 실행 효과를 임의로 묵살하지 않는다. M2 프런트엔드 수렴 우선순위는 유지한다. GPU/Hash/Graph 후보 선택은 FW-05~FW-13의 독립 증명·가드·자원·비용 허가 전에 열지 않는다.


<a id="rank-graph-zero-frame"></a>

##### P.12 빈 Frame·빈 Cell의 J Graph IR 의미론 — 구조 증거와 실행 생략 허가 분리 (2026-10-06)

**설계 결론.** 빈 배열은 별도의 `EmptyArray` 연산 노드가 아니라 *논리 Shape·타입·원래 Rank/CellApply 적용 관계*에 속한다. `0 3`과 `2 0`은 둘 다 원소 수가 0이지만, rank 1을 적용할 때 전자는 **frame=[0], cell=[3]**(실제 셀 호출 0회, J fill-cell 의미론 필요), 후자는 **frame=[2], cell=[0]**(빈 셀에 대한 실제 호출 2회)이다. 따라서 `element_count == 0`은 **연산 실행이나 효과·오류 평가가 0회**라는 증명이 아니다. 고정 jsource `jsrc/cr.c::jtrank1ex/jtrank2ex`와 §P.10의 fill 계약을 참조한다.

**구현 경계.** `src/facts.rs`의 `rank_plan_for_shapes`는 frame/cell 분해를 단일 구조적 의미론으로 제공하고, `RankPlan::frame_execution()`은 `ZeroFrameNeedsFill / CellsPresent / IncompatibleFrames`만 구별한다. `has_empty_input_cell()`은 별도 속성이다. `src/j_graph_ir.rs::Plan::rank_frame_plan(ValueId)`은 실제 `GraphForm::Rank`이며 requested rank와 입력 Shape을 알 때만 **읽기 전용** RankPlan을 반환한다. `src/facts.rs`의 J Graph projection과 A3 inference가 같은 rank/frame 분해를 사용한다. 기존 `GraphFacts`의 dtype/shape가 빈 frame의 결과에 대해 **Unknown**이면 그대로 남겨두고, J Graph에 물리적 버퍼·실행 결정이나 새로운 형식 노드를 추가하지 않는다.

**의무와 금지 사항.** 구조적 `ZeroFrameNeedsFill`은 `ProvenEmptyResult`도 `SafeToElide`도 아니다. 원본 함수의 부작용·동적 이름·computational/error fallback·boxed/sparse filler·결과 cell 타입/shape를 별도 증명하지 못하면 fill-cell 평가를 생략하는 후보를 선택하지 않는다. A3 `IterationDomain`에 0 크기 frame 축이 보이더라도 kernel 생략/할당 제거/Graph fusion을 자동 허가하지 않는다. 생략 후보와 실제 선택은 FW-05~FW-13의 증거·Guard·자원·비용 게이트에 종속된다.

**타 배열 컴파일러 비교와 적용 경계(2026-10-06).** 아래 네 모델은 동일한 J 의미론을 구현했다는 근거가 아니라 서로 다른 *분석·최적화 단계*의 참고 자료다.

| 참고 시스템 | 확인한 접근 | RustJ 적용과 한계 |
|---|---|---|
| [XLA ZeroSizedHloElimination](https://github.com/openxla/xla/blob/main/xla/hlo/transforms/simplifiers/zero_sized_hlo_elimination.h), [패스 설명](https://openxla.org/xla/hlo_passes) | 원소 수 0의 HLO 출력을 빈 상수로 대체 | **빈 결과의 의미론 및 생략 적법성을 먼저 증명한 경우**에만 유사한 후보 생성. J의 fill-cell·효과·오류 무시 금지 |
| [Futhark 크기 타입](https://www.futhark-lang.org/blog/2020-03-15-futhark-0.15.1-released.html) | `[n]a`처럼 배열 크기·원소 타입을 표현; `[0][2]i32`와 `[2][0]i32` 구별 | Rank frame 기하와 독립적으로 **결과 cell의 dtype/shape**를 증명하는 분석을 참고. J 동적 이름/효과/이질적 조립은 별도 |
| [MLIR Linalg](https://mlir.llvm.org/docs/Dialects/Linalg/) | 인덱싱·반복 공간과 region의 계산 본문을 분리 | `CellApply` 반복 구조를 실제 cell 적용·결과 조립과 분리해 보존. `tensor.empty`는 원소 수가 0이라는 뜻이 아니라 내용 미지정 텐서 생성 |
| [StableHLO reduce](https://openxla.org/stablehlo/spec#reduce) | 축·reducer·init_values·결과 형식을 명시 | 결과 형식이 계약으로 확정되는 reduction에만 별도 추론 적용. J 일반 `/`와 Rank fill-cell을 초기값 기반 reduction으로 무조건 치환하지 않음 |

**ZF-IR-03의 추가 상세 계약(계획·미구현).** `RankPlan::frame_execution()`은 작업 생략 허가가 아니다. 불변 원본 J Graph 옆에 재계산 가능한 증명 sidecar를 두고 (1) graph/value 출처·원본 버전/동적 이름, (2) 결과-cell J dtype/shape·boxed/sparse fill·조립, (3) fill-cell 결과를 재구성하는 의미론, (4) 효과·오류 종류/순서·핸들러, (5) 최초 관찰 가능 효과 이전 Guard와 정확한 semantic reference fallback을 **각각** `Unknown / Proven(witness) / Guarded(guard+fallback) / Disproven`으로 구별한다. 구조상 `frame=[0]`이거나 일부 결과 Shape을 안다는 사실만으로 `Unknown`이 `Proven`이 되지 않는다. `Guarded` 역시 guard/fallback 검증 전에 선택 불가. 원본은 불변, 실행 선택은 FW-05~FW-13과 ZF-IR-04 뒤에만 둔다.

**작업 순서.** RK-06의 C oracle 기준 적대 Rank/empty-frame fixture → RK-07~10 오류/효과/boxed/sparse/조립 검증 → ZF-IR-03 sidecar·Unknown/Guard negative tests → ZF-IR-04 3경로 CPU/물리 승인. M2 우선순위를 바꾸지 않으며 위 계획 문서화만으로 ZF-IR-03/04·FW-06/07의 [ ]를 변경하지 않는다.

**정확한 RK-06 불일치 입력과 결과(추가 검증).** [새 진단 CI 37452934704](https://github.com/yunskim/RustJ/actions/runs/37452934704)의 `j64/default` job 출력에서 `(0 3 $ 'abc') (+"1 1) (i.0 3)`은 pinned jsource의 **정수형(타입코드 4), shape `[0,3]`, 빈 데이터**에 대해 독립 Rust reference/optimized가 모두 **`domain error`**를 반환했다. `(i.0 3) ((+"0 0)"1 1) (i.0 3)`은 C의 **Int(타입코드 4), shape `[0,3]`, 빈 데이터**에 대해 Rust 양 경로가 모두 **`unsupported`**를 반환했다. J 원본의 fill-cell 계산 오류 처리(`cr.c::jtrank2ex`의 비필수 오류 억제·재시도 포함)와 계층적 Rank 재귀 의미론을 독립 확인해야 하며, **빈 결과를 전부 원래 입력 타입으로 처리하거나 단순히 `domain error`를 무시하는 수정은 금지한다.** 위 증거는 `j64/default` job의 실제 세 경로 출력이다.

**RK-06 추가 C-oracle 관찰 항목(작성 완료, CI 검증 대기).** [`a4ffb0c`](https://github.com/yunskim/RustJ/commit/a4ffb0c240e5a4271bb9f20b86627f0f5fca157f), [`b241c97`](https://github.com/yunskim/RustJ/commit/b241c9730e1993cda3a402b9256f1e184d9fc501), [`d5f8a03`](https://github.com/yunskim/RustJ/commit/d5f8a0312952e3d47ee53a48b7eb6b1a67c0d012)에서 `tools/ranked_search_audit.py --adversarial`을 기존 24→**32 입력**으로 확대했다. 새 8건은 char/int 방향, char/float·bool, 한쪽 인자만 빈 frame, 정상 frame의 실제 char/int 셀과 빈 셀, 중첩 Rank 비빈 셀을 포함한다. `jsrc/cr.c::jtrank2ex`의 `EVINHOMO` fill-cell 재시도(원래 데이터 비어 있는지에 따라 타입 선택), *재시도 후* 비필수 계산 오류의 scalar 0 대체, exigent 오류 전달을 **서로 다른 의미론 경계**로 관찰하기 위한 탐색 corpus다. **실제 고정 C oracle 교차 검증(2026-10-06):** [Linux CI 37453629236](https://github.com/yunskim/RustJ/actions/runs/37453629236)의 check + pinned j64/j64avx2 × Rust default/portable **5/5 jobs 성공**. 네 reference 조합에서 각각 **32건 중 25 pass / 7 `rust_semantic_mismatch`**, 합계 100/128 일치·28 불일치 관찰(독립된 7개 입력 × 4조합). 불일치 이름은 `empty_type_mismatch`, `empty_type_mismatch_reversed`, `empty_char_float_fill`, `empty_char_bool_fill`, `empty_char_left_real_right`, `empty_char_right_real_left`, `positive_frame_empty_char_cells`. 앞의 6개에서 pinned C는 정수형 shape `[0,3]`, 마지막 1개는 정수형 shape `[2,0]`을 반환하고, Rust 기준/최적화 두 경로는 모두 `domain error`. `nonempty_char_int_domain`과 `nested_rank_nonempty_cells`은 일치. 이 **green CI는 비수용 진단 실행 성공**이며 해당 7개를 성공·허용된 불일치로 계산하지 않는다. 위 25/32는 수정 전 이력이다. 아래에서는 RK-06의 **고정 32건 유한 집합만** 엄격 회귀 기준으로 수용한다. RK-07·ZF-IR-03은 [ ] 유지하며 기존 20건 엄격 회귀 gate도 변경하지 않는다.

**RK-06 유한 차분 집합 완료 판정(2026-10-06; RK-07 전체 완료는 아님).** [최종 엄격 CI 37458845392](https://github.com/yunskim/RustJ/actions/runs/37458845392)에서 **check + pinned C j64/j64avx2 × Rust default/portable 4개 reference job, 총 5/5 성공**. 동일 고정 C revision `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`에 대해 새 RK-06 **32건 × 4조합 = 128개의 3경로 비교 모두 `pass`, mismatch 0**. 새 `--adversarial --gate-adversarial`은 각 불일치에서 종료코드 1을 반환하고 정확한 C / Rust reference / Rust optimized 관찰을 출력·보고서로 남기며, [`365c3ad`](https://github.com/yunskim/RustJ/commit/365c3adcb9e716e34f3083f50c317e243a51ad23)에서 Linux CI 네 조합의 **차단 게이트**로 승격했다. 기존 20건 Rank 엄격 gate도 유지. [`1644644`](https://github.com/yunskim/RustJ/commit/1644644444d8e51ee2097f4acd659b1e4b893013)·[`b631f6f`](https://github.com/yunskim/RustJ/commit/b631f6fb488f449fcc4c35b44da6ac5bcca444d8)에서 부정 사례가 실패를 유발하는 계약도 시험했다. 이 **32개 고정·dense 입력 집합의 RK-06만 [x]**이며, 무한한 J Rank 일반 의미론을 승인한 것이 아니다.

**RK-07 현재 구현 경계와 잔여 수용 조건.** [`9ab2952`](https://github.com/yunskim/RustJ/commit/9ab2952100ae7b2ae1e6338a761ecf4fc798804f)에서 값 전용 Rank의 **결과 Frame=0인 fill-cell 계산 `Domain` 오류**에 한정해 integer-zero 복구를 추가했다. [`cbd13af`](https://github.com/yunskim/RustJ/commit/cbd13af0864a85452a2cface91f283bf07b9f4e2)·[`85f1732`](https://github.com/yunskim/RustJ/commit/85f1732bb29da8b1573781e6fb6b86711d91468e)·[`a5541b2`](https://github.com/yunskim/RustJ/commit/a5541b22705b4f53aeb8a2094d958d5d749ec557)에서 *원시 `+`의 내재적 Rank 0* 결과 셀 shape를 유지하여 [0,3]을 [0]으로 잘못 축소하지 않는다. [`a23bcf4`](https://github.com/yunskim/RustJ/commit/a23bcf40b104e359fe135da9d76c8d0ae521e672)·[`6be596a`](https://github.com/yunskim/RustJ/commit/6be596a81ccec1edb346fe2554a42e7811c406ef)에서 양수 바깥 Frame [2] 안의 원자 수 0인 `+` 셀도 정수형 [2,0]으로 처리한다. [`0d85f03`](https://github.com/yunskim/RustJ/commit/0d85f030b00122d2239150a8d868327ff8810f0c)·[`75a36eb`](https://github.com/yunskim/RustJ/commit/75a36eb7886ca2f68d9527412e41453b93d9af62)에는 위 7건 및 **실제 nonempty char/int의 Domain**, Frame 불일치의 Length, 미지원·자원 오류의 전달을 확인하는 회귀가 있다. 근거는 고정 원본 [`jsrc/cr.c::jtrank2ex` L397–418](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c#L397-L418), [`jtrank2ex0` L541–569](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c#L541-L569), [`jerr.h::EXIGENTERROR` L89–90](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/jerr.h#L89-L90). **잔여 [ ]:** 일부 `,`/dense의 `EVINHOMO` 타입 재시도는 아래와 같이 구현했으나, 다른 동사까지 일반화할 **명시적 내부 오류 분류와 provenance**, 나머지 non-exigent/exigent 오류 계층·오류 우선순위, 사용자 동사 효과·이름 버전·boxed/sparse 및 비균일 셀 조립은 미완료다. `Unsupported`·`Limit` 등을 임의 복구하지 않으며 zero-frame shape fact만으로 실행 생략 또는 GPU lowering을 허용하지 않는다. **RK-07·08~12, ZF-IR-03/04, FW-06/07은 [ ] 유지.**

**RK-07 — `EVINHOMO` 타입 재시도 1차 구현 및 고정 C 수용 경계 (2026-10-06).** 고정 원본 [`jsrc/cr.c::jtrank2ex` L397–418](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c#L397-L418)·[`jtrank2ex0` L541–569](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/cr.c#L541-L569)은 *fill-cell 첫 호출 → `EVINHOMO` 시 자료형 선택 → 값 변환이 아닌 해당 자료형의 기본 fill-cell 재생성 → 단 한 번 재시도 → 남은 non-exigent 오류는 정수 0, exigent 오류는 전달* 순서다. 원래 양쪽 배열 중 **원소가 존재하는 쪽**의 dtype이 우선이며 **모두 빈 배열**일 때만 [`j.h::TYPEPRIORITY`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/j.h#L2040-L2046)의 bool < char < int < float 우선순위를 따른다. 이 정보는 Graph IR의 Frame 크기만으로 얻을 수 없으므로 타입·원래 원자 수·실행 동사의 근거를 보존한다.

[`ae7258d`](https://github.com/yunskim/RustJ/commit/ae7258d2d391ee43cecb5477f1ccd52f96025e80)에서 RK-06의 기존 엄격 **32건을 건드리지 않고** 별도 `--retry-probes` 10건(빈 frame과 양수 frame 안 빈 cell, char/int 방향, char/float·bool, 단측 nonempty, 정상 오류/Frame 길이 불일치)을 추가했다. 처음 [탐색 CI 37460517931](https://github.com/yunskim/RustJ/actions/runs/37460517931)의 j64/default에서는 **3 pass / 7 불일치**였다. 예를 들어 C의 **int [0,6]** 또는 **float [0,6]** 결과에 Rust 양 경로는 **int [0]**을 반환했으며, 양수 바깥 frame의 혼합 빈 셀 C **int [2,0]**에 Rust는 Domain이었다. [`d192ad6`](https://github.com/yunskim/RustJ/commit/d192ad6889b6dbcdf654321b7668b040cde54ae9)은 입력 값을 버리고 목표 dtype과 shape만 보존하는 기본 filler 생성기를 추가했다. [`4e35d44`](https://github.com/yunskim/RustJ/commit/4e35d443eb4eb8f2c8fa4fc8528439fc18aa141e)·[`5485011`](https://github.com/yunskim/RustJ/commit/5485011de69fd2354d19059cd79612d1c4fcd3e8)·[`a6a254c`](https://github.com/yunskim/RustJ/commit/a6a254cac2cee70a963abd2b19a4ef2194f29632)은 **구체적 순수 원시 동사 `,`의 dense char/numeric 불균질 오류**에서만 1회 타입 재시도를 허용하고, [`48e8a6a`](https://github.com/yunskim/RustJ/commit/48e8a6a203e1f58f28063b0765f60a19ff59731e)은 양수 frame 내부 원자 수 0인 연결 셀의 출력 타입을 보존했다. [`b370ac8`](https://github.com/yunskim/RustJ/commit/b370ac855f6cc81b02a38c569541100f98f2356e)·[`177b18c`](https://github.com/yunskim/RustJ/commit/177b18c50c4c2ad70a83aa3d7aee793c86a065da)은 값·자료형·형상 및 1회 재시도, 실제 원소가 있는 일반 셀의 Domain, frame 불일치 Length, Limit 오류 비재시도를 회귀 테스트로 고정했다.

**수용 증거:** [Linux CI 37461525543](https://github.com/yunskim/RustJ/actions/runs/37461525543), 원본 C 고정 commit `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`. check(파이썬 테스트·fmt·clippy·Rust default/portable 테스트·빌드) + j64/j64avx2 × default/portable **5/5 작업 통과**. **RK-07 신규 10건 × 4조합 = 40/40 독립 C / Rust 의미 기준 / Rust 실행 비교 `pass`, 불일치 0**. [`37e04dc`](https://github.com/yunskim/RustJ/commit/37e04dc8109065e86e4c2bca333416d0c9b441f4)·[`706fa36`](https://github.com/yunskim/RustJ/commit/706fa367aa860760d5b4df3b17dbb9b26ccf2da3)·[`b245f9e`](https://github.com/yunskim/RustJ/commit/b245f9e8a27bac0ffd21b724a9ca8df0d18b941c)의 `--retry-probes --gate-retry-probes`가 각 불일치에서 CI를 실패시키며 C·두 Rust 경로의 실제 출력과 report JSON은 CI artifact로 보존한다. 기존 RK-06 32건/4조합 **엄격 게이트도 유지**한다.

| RK-07 하위 작업 | 상태 | 다음 검증 기준 |
|---|---|---|
| RK-07-E1 · `,` 원시 동사의 타입 재시도 | [x] 제한적 dense 구현 | 최초 실패 후 실제 원자 유무에 따른 타입 선택, 기본 fill-cell 생성, 1회 재시도. 10건 pinned C 차분 |
| RK-07-E2 · 증거의 지속성 | [x] 해당 10건의 엄격 게이트 | 4조합 40/40, 하나라도 차이면 CI 실패·report JSON 잔존 |
| RK-07-E3 · 내부 오류 종류 | [ ] | `EVINHOMO`의 일반화 가능한 내부 분류·실행 단계별 provenance, 다른 primitive의 비필수 오류와 구별; `Error::Domain`만 보고 임의로 재시도 금지  **부분 진전(2026-10-06, 전체 [ ]):** [`5aa7074`](https://github.com/yunskim/RustJ/commit/5aa7074762ca6eac1570bc17d728b9ee07b6c3fc)에서 `Option<VerifiedValueOnlyZeroFrame>` 증명 토큰을 `recover_zero_frame_fill_domain`에 요구한다. 일반 셀/미검증·효과 호출의 `Domain`은 복구 금지, 증명된 빈 frame 값 전용 경로에서만 기존 정수 0 대체. 독립 A3 reference와 일반 kernel 호출자가 모두 토큰을 전달하며 부정 회귀를 추가. `EVINHOMO` 내부 코드별 provenance/오류 계층은 여전히 미구현이고, ZF-IR-03·FW-06/07 실행 생략 허가 아님. [Linux CI 37467064363](https://github.com/yunskim/RustJ/actions/runs/37467064363) **check + j64/j64avx2 × default/portable 5/5 jobs 성공**. 각 환경 RK-06 32/32 + RK-07 retry 10/10 + error-precedence 8/8, **200/200 pinned-C 3경로 일치·불일치 0**. [Basis probe 37467064362](https://github.com/yunskim/RustJ/actions/runs/37467064362) 성공. E3의 원본 내부 오류 분류·E4 exigent provenance는 **[ ] 유지**. |
| RK-07-E4 · exigent/비필수 분리 | [ ] | 고정 [`jerr.h::EXIGENTERROR` L87–90](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/jerr.h#L87-L90) 비교; 특히 Rust `Limit`가 C `EVLIMIT`와 할당 실패를 함께 대표할 수 있으므로 근거 없는 오류 억제 금지; 오류 순서 및 overflow 테스트 |
| RK-07-E5 · 실행 가능한 범위 확장 | [ ] | 파생·동적 이름·부작용·boxed/sparse·비균질 셀 조립; 기존 값 전용 증거가 사용자 동사의 효과 생략·GPU 실행 승인으로 확대되지 않음 |

**판정:** 상기 10건의 `EVINHOMO` 지원 dense 부분집합만 통과. **RK-07 전체·RK-08~12·ZF-IR-03/04·FW-06/07은 계속 [ ]**. 나머지 미검증 결과를 일반적인 일치·최적화 허가로 취급하지 않는다.

**RK-07 — 오류 우선순위 독립 C-oracle 8건(2026-10-06).** [탐색 CI 37463314265](https://github.com/yunskim/RustJ/actions/runs/37463314265)에서 고정 원본 C j64/j64avx2 × Rust default/portable **5/5 작업 성공**, 각 환경 **8/8** 3경로 일치(총 32/32, mismatch 0). 기존 E1/E2의 `EVINHOMO` 10건과 RK-06의 32건을 변경하지 않고 `--error-probes`를 별도로 추가했다. 입력은 (1) 결과 Frame 0 아래의 inner-cell Length, (2) 빈 char/정수 혼합 셀 길이 불일치, (3) 양수 Frame 실제 셀 Length, (4) Frame prefix Length 선행, (5) 빈 Frame 및 실제 셀 Index, (6) Frame 유무별 나눗셈을 구분한다. [`a84a508`](https://github.com/yunskim/RustJ/commit/a84a5084f3a4318f1f26dc95b0428e10317064cd)·[`cce5dec`](https://github.com/yunskim/RustJ/commit/cce5decdbbec2d6577c415d5696f8d7775738b10)에서 corpus와 부정 증거 보존 테스트를 추가했다. [`ba90688`](https://github.com/yunskim/RustJ/commit/ba9068801554e5836d9b57111e8e937440c23172)·[`970f6b0`](https://github.com/yunskim/RustJ/commit/970f6b07c09ed3969cfc158005c12a2795ce5010)·[`c90701d`](https://github.com/yunskim/RustJ/commit/c90701de2b456e556fb256128e976a7556793930)에서 `--error-probes --gate-error-probes`를 고정 8건 엄격 CI 게이트로 승격했다. **최종 증거 [Linux CI 37463712382](https://github.com/yunskim/RustJ/actions/runs/37463712382): 5/5 작업 성공, 8건 × 4조합 = 32/32 pinned-C 3경로 일치, 불일치 0건.** 실행 증거는 CI의 각 reference job 및 `rank-error-*.json` artifact에 남는다. 해당 **8개 유한 사례의 엄격 게이트만 수용**하며 내부 오류 provenance·나머지 exigent/비필수 분리·효과 및 일반 Rank 의미론이 미완료이므로 **RK-07 전체 [ ]**를 유지한다.

**RK-07-E4 원본 exigent 오류 분리 원칙.** 고정 [`jsrc/jerr.h` L7–55, L87–90](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/jerr.h#L87-L90)에서 `EXIGENTERROR`는 `EVALLOC`, `EVWSFULL`, `EVVALENCE`, `EVNONCE`, `EVVALUE`, `EVTHROW`, `EVSTACK`, `EVSYSTEM` 등 반드시 전파해야 할 원본 C 코드의 **선별적 집합**이다. 반면 `EVDOMAIN`, `EVLENGTH`, `EVINDEX`, `EVRANK`, `EVLIMIT`, `EVINHOMO`는 그 목록 밖에 있지만, 이 사실만으로 모든 Rust 오류를 억제할 수는 없다. RustJ `Error::Limit`는 원본 `EVLIMIT`와 자원·할당 실패를 정확히 분리하지 못하고, `Error::Unsupported`도 원본의 `EVNONCE`와 동치가 아니기 때문이다. 그러므로 **오류 종류 + 발생 단계 + 원본 내부 코드 + 효과 가능성**을 별도 witness로 갖기 전에는 이 두 Rust 오류를 억제·재시도하지 않는다. 다음 구현 작업은 오류 provenance를 구별하는 내부 분류(E4), 오류 선행 순서·exigent 전파의 추가 C 테스트, 사용자 정의 동사의 효과/동적 이름(E5)이다. 8건의 유한 차분 일치를 모든 non-exigent 오류의 복구 증거 또는 Graph IR 실행 생략 허가로 확장하지 않는다.

**지속 체크리스트 (RK-11 및 FW-04/JX-04의 하위 항목).**

| ID | 현재 | 수정·검증 수용 기준 |
|---|---|---|
| ZF-IR-01 / M2 | [x] **Rank/frame 구조 의미론 공통화** | `rank_plan_for_shapes`와 `RankFrameExecution`; 동일 입력에 대해 J Graph와 A3의 frame·cell·빈 frame 분류가 일치해야 함. 코드 커밋 [911e113](https://github.com/yunskim/RustJ/commit/911e113c8761f3e7b25ca6b932f6c8ab56e0b398)·[fb9f884](https://github.com/yunskim/RustJ/commit/fb9f8842ac74ef289e0852927f3ef65d54574f11). [Linux CI 37451816951](https://github.com/yunskim/RustJ/actions/runs/37451816951) check + 4 reference jobs 성공 |
| ZF-IR-02 / M2 | [x] **J Graph 읽기 전용 조회 + 역방향 검증** | `Plan::rank_frame_plan`([89984f1](https://github.com/yunskim/RustJ/commit/89984f10aa6e3869e2f3d4f77730e2537030854d)), `tests/j_graph_ir.rs`([b52fd51](https://github.com/yunskim/RustJ/commit/b52fd51d2420c27d4425b920f534b8941d0f0871)): `0 3`·`2 0`·중간 0·불일치 frame, Graph와 A3 RankPlan 비교, 비-Rank 조회 None, 결과 미추론 보존. [Linux CI 37451816951](https://github.com/yunskim/RustJ/actions/runs/37451816951) 5 jobs 성공 |
| ZF-IR-03 / M3·FW-06/07 | [ ] **빈 결과 생략의 독립 증명 의무** | result-cell 타입·shape·순수성·오류 억제·효과/동적 이름·Guard·fallback을 구별; Unknown→생략 승인 및 frame 0→무조건 kernel 생략을 거부하는 negative test |
| ZF-IR-04 / M3/M4·FW-11/13 | [ ] **실행 선택·물리 최적화 검증** | pinned C/J, 독립 Rust semantic reference, Rust optimized의 3경로 회귀 및 CPU 측정 후에만 빈 커널/버퍼 생략을 개별 활성화. 2 0의 빈-cell 호출·오류 및 zero-frame fill 오류·효과를 비교. GPU 승인 없음 |

**현 단계 판정·검증(2026-10-06):** ZF-IR-01/02 **2/4 완료**. [Linux milestone run 37451816951](https://github.com/yunskim/RustJ/actions/runs/37451816951), HEAD [`e9f821d`](https://github.com/yunskim/RustJ/commit/e9f821d0658005a1545131d31c31fb53d3521b26)에서 **check(기본/portable 테스트·fmt·clippy) + pinned j64/j64avx2 × default/portable 총 5/5 jobs 성공**을 확인했다. 이 기존 차분 실행은 **변경되지 않은 실행기의 회귀**에 대한 결과이지 ZF-IR 기반 kernel-elision 최적화를 실행·검증했다는 뜻이 아니다. ZF-IR-03/04는 구현되지 않았고, 상위 RK-11·FW-04·JX-04 **[ ] 유지**. §P.11의 RK-06~12 순서는 변경하지 않는다.

<a id="jsource-optimization-migration"></a>

##### Q. jsource 전계열 최적화 이행 계획·체크리스트 — 검색 이외 포함 (2026-10-06, 살아 있는 작업표)

**목적·정본.** §4.1.3.6 A–M은 jsource에서 확인한 **최적화 아이디어·원본 제약·RustJ 소유 계층의 조사 목록**이다. §O.5 **FW-01~FW-18**은 공통 증명·Guard·CPU 실행 게이트이고, §P.1은 **검색(i. family)의 세부 구현 게이트**다. 이 Q절은 그 **이외에도 존재하는 전체 source-derived family를 빠짐없이 작업 단위로 관리하려는 이행 추적표**이며, 세 표의 완료를 서로 대체하지 않는다. 별도 optimizer, C special entrypoint별 Graph node, 범용 registry를 선행 생성하지 않는다. 원본 검토 기준은 고정 jsource commit [13994ffa1ed5f06f79fad6e9822a7ed2d29b1528](https://github.com/jsoftware/jsource/tree/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc)이다.

**상태: 0 / 26 완료.** 아래의 모든 [ ]는 **코드 변경·실제 실행 검증까지 충족하는 수용 게이트**다. 기존 구현·후보 분석·작성된 테스트의 존재만으로 [x]를 부여하지 않는다. M2 일반 frontend와 **FW-01을 우선**하고, JX-01의 목록 정리처럼 정적 분류만 선행할 수 있다. 나머지는 A(기준 의미론) → B(M3 증명 계약) → C(연산군별 후보·구현) → D(M4/후속 실행 승인)의 종속성에 따른다. C단계의 **후보 표현과 proof 준비**는 선행 가능해도, 특수 실행의 **선택·활성화**는 FW-11~FW-13의 독립 CPU 기준선·자원·비용 게이트 이후다. 특히 JX-08이 입증되기 전에는 공통 family interface를 추출하지 않는다.

| ID / 게이트 | source-derived 계열과 직접 근거 | RustJ 수정·연결 위치 | 완료 수용 기준과 반드시 남길 negative/차분 테스트 |
|---|---|---|---|
| JX-01 / A | **원본·coverage 인벤토리:** ca/cf/cr/va2/ar/ap, ao/cip/gemm, vg/vgsort/vgranking, vi/viix, cc/cu/vfrom/vf/vrep/vcat, sparse/sc/a/vrand/am 등 §A/H/K/M | src/j_graph_jsource.rs의 catalog/source pin·coverage; §4.1.3.6 A–M | family마다 **원본 source form → entrypoint → guard/fallback → 의미론 → RustJ owner → 상태**를 연결하고 중복·미조사 항목을 명시. pinned revision 이후 소스 변경은 별도 diff로 취급. exhaustive라고 표시하지 않음 |
| JX-02 / A·FW-01 | **J 구문/derived entity coverage:** @:, fork/hook, rank, /., dot, grade, cut, under, M., ?/?. | src/tokenizer.rs, src/enqueuer.rs, src/parser.rs, src/semantic.rs, src/j_graph_ir.rs | 지원하는 valence/POS/operand/late-name만 source entity로 인식. 미지원은 AwaitingFrontendOrFacts; 파싱 불가 표현에서 가짜 candidate 생성 금지. C parser/oracle와 정상·부정 사례 비교 |
| JX-03 / A·FW-02 | **진짜 순차 기준 의미론:** Reduce, Scan, Search 및 추후 연산군 | src/runtime.rs, src/kernels.rs, src/index_ops.rs, semantic/runtime tests | buffer pooling과 최적화 허가를 분리. 지원 subset에서 candidate selection·prehash·특수 알고리즘 호출 0회인 Rust baseline을 계측; C는 외부 비교 oracle이지 정상 fallback이 아님 **진행(2026-10-06):** FW-02 `i./i:/e.` 독립 dense reference와 prehash 불변성 테스트 추가(`45ccebb`,`3436252`; 위 FW-02 CI 증거). Reduce/Scan 전체 및 호출 경로 계측 미완료로 수용 보류. |
| JX-04 / A·FW-03/04 | **J 의미론 fixture:** rank/cell/frame, tolerance/Fit, empty/prototype, boxed/sparse, numeric overflow·오류 우선순위 | tests/semantic.rs, tests/j_graph_jsource.rs, tests/index_ops.rs 및 기존 differential harness | 고정 C binary / 순차 Rust / 최적화 Rust의 **서로 독립된** 결과를 기록. 현재 미지원과 C 원본의 fast-path 이상 징후를 분리; Rust near를 J CCT의 oracle로 가장하지 않음 **진행(2026-10-06):** [`1f3d1cc`](https://github.com/yunskim/RustJ/commit/1f3d1cccfc5c30b385f5d886adc9cd9ac5872f22) Rust-only 4,840조합 fixture 추가. J C binary의 동일 입력·관찰 결과와 3방향 연결 전이므로 수용 보류. **2026-10-06 확장 (검증 대기):** [`bc648cc`](https://github.com/yunskim/RustJ/commit/bc648cc0ba43761e103cd42c58830147c30ebb9c)에 독립 프로세스 3방향 C/Rust baseline/Rust optimized harness 45문장 추가; [`4e8b4dd`](https://github.com/yunskim/RustJ/commit/4e8b4ddd1d4a5b9d065830fb0211137f9b894d26)에서 분류 테스트, [`09f9a3b`](https://github.com/yunskim/RustJ/commit/09f9a3ba7804fc769027b16c7c9f5abc630339ac)에서 j64·j64avx2 × default·portable CI gate 등록. 지원 dense subset만 검증하며 Rank/boxed/sparse/CCT는 미검증. **실행 증거(2026-10-06):** [Linux milestone run 37441174098](https://github.com/yunskim/RustJ/actions/runs/37441174098), 명령 `python3 tools/search_three_way.py --reference-revision 13994ffa1ed5f06f79fad6e9822a7ed2d29b1528 --report reports/search-three-way-<variant>-<backend>.json`; GitHub Actions Ubuntu Linux, J C `j64`/`j64avx2` × Rust default/portable 네 조합 **각 45 passed / 0 failed / 0 ignored, 합계 180 비교**. 실제 jsource pin `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`; 개별 fixture/해시/불일치 종류는 CI JSON artifact. **제한:** supported dense search 정수·bool·문자·정확 부동소수, 중복·빈 배열·multi-item·재검색; Rank-파생 우회/boxed/sparse/동적 CCT·Fit 미검증. **FW-04 전체 [ ] 유지.** **FW-03 tolerance 진단:** [CI 37445080131](https://github.com/yunskim/RustJ/actions/runs/37445080131), j64·j64avx2 × default·portable 각 48건에 8개 C-vs-Rust 의미 불일치, reference-vs-optimized 0개. GitHub reports/tolerance-audit 원자료에 구분 보존; semantic acceptance 아님. **[ ] 유지.** **후속 구현/검증:** [`9aab336`](https://github.com/yunskim/RustJ/commit/9aab3362ac542ed96c0bd3ba6eb2ab6c109f14f4) pinned 기본 CCT 적용. [CI 37445891526](https://github.com/yunskim/RustJ/actions/runs/37445891526) j64·j64avx2 × default·portable **각 48/48, 총 192/192 일치, Rust reference-vs-optimized 불일치 0**. 위 40/8은 변경 전 이력. 범위/미완료 의무는 §P.9를 따른다. **[ ] 유지.** **Rank 차분 진단(2026-10-06, 수용 아님):** [`5b4a7d6`](https://github.com/yunskim/RustJ/commit/5b4a7d6ddf0fdf8e966ee06f63173e71cdbce423) `tools/ranked_search_audit.py`, [`c300ff3`](https://github.com/yunskim/RustJ/commit/c300ff3295499f04e41c928a78e38cd3ed95d7fc) corpus 단위 테스트, [`d88c38b`](https://github.com/yunskim/RustJ/commit/d88c38b34530c84d7ed02b46a9cbf5a55186a50f) CI matrix 연결. [CI 37446732381](https://github.com/yunskim/RustJ/actions/runs/37446732381) Ubuntu Linux `j64`/`j64avx2` × Rust default/portable 각 **14 cases / 13 C·Rust reference·Rust optimized 일치 / 1 Rust 양 경로 Unsupported**, 합계 **52/56 일치(미해결 4개 관찰)**. 불일치 입력 `(i.0 3) (i."1 1) (i.0 3)`에서 C는 Int shape `[0,3]` 빈 결과, Rust 양 경로는 `Unsupported`(Rank empty-frame prototype inference). **이 사례를 성공/허용 불일치로 간주하지 않는다.** J Rank 결과 prototype·dtype·shape/오류 계약을 먼저 구현하고 3방향 재검증할 것. P.1/search exact optimization 증명으로 전용하지 않음; FW-04/JX-04 [ ] 유지. **P.10 zero-frame Rank 보강(2026-10-06):** [CI 37449102885](https://github.com/yunskim/RustJ/actions/runs/37449102885) pinned j64·j64avx2 × default·portable 4조합 각 20/20(총 80/80), supported dense typed fill-cell·monad/dyad·search/reference/optimized 3방향 일치. [Basis probe 37449102980](https://github.com/yunskim/RustJ/actions/runs/37449102980) 성공. Boxed/sparse, computational error fallback, 사용자 정의 effectful Rank 및 mixed-result padding 미지원으로 **[ ] 유지**. **후속 단위 작업:** [§P.11 RK-01~RK-12](#rank-cellapply-followups) 체크리스트에 실행 증거·진행 상태를 기록한다. **상위 [ ] 유지.** |
| JX-05 / B·FW-05 | **multi-region source anchor:** Cut–Scan–Raze, convolution, composition·중첩 rank | src/j_graph_ir.rs, src/j_graph_jsource.rs, src/analysis.rs, src/lowering.rs | 불변 원본 graph의 schema·source value·region·ordered A3 op 집합·rule/version을 sidecar로 검증. stale/forged/span 재사용·다른 region 혼동을 negative test로 거부 |
| JX-06 / B·FW-06/08/09 | **family별 proof obligations:** valence, rank, equality, empty, error, ownership, resource | src/j_graph_jsource.rs, src/lowering.rs 및 기존 proof/verifier 경계 | Unknown/Proven/Guarded/Disproven을 obligation별 witness와 함께 보존; 미충족 하나라도 있으면 commit/selected 금지. 기존 rewrite/fusion/search의 증거를 잘못 재사용하지 않는 테스트 |
| JX-07 / B·FW-07 | **Guard·fallback·효과 경계:** RNG, name lookup, memo, allocation, numeric retry | src/lowering.rs, src/runtime.rs, src/execution_semantics.rs | guard는 observable effect보다 먼저, miss는 명시된 정확한 reference path로. 오류·난수 소비·cache mutation 뒤 자동 재실행·중복 실행 금지. guard miss와 post-effect replay negative tests |
| JX-08 / B·FW-10 | **두 번째 독립 연산군 검증:** Reduce/Scan **또는** GroupReduce | src/j_graph_scan.rs 또는 GroupBy basis + src/j_graph_jsource.rs, src/lowering.rs | 검색과 서로 다른 numeric order/association 또는 group representative/order obligations로 JX-05~07 계약을 실제 사용. 그 전에는 공통 Candidate/SelectionPlan 추출 금지 |
| JX-09 / C | **composition·rank/MapReduce streaming:** ca.c, cf.c, cr.c, va2.c::jtfslashatg | src/j_graph_fusion.rs, src/j_graph_composition.rs, src/fusion_planning.rs | @:·capped fork의 source 정체성 및 inner/outer rank domain 유지; dense·empty·dtype·effect·in-place 비용이 맞을 때만 streaming. 기존 fusion 후보 중복 금지, generic fallback 차분 |
| JX-10 / C | **Reduce/Mean/shape fast path:** ar.c::jtreduce/jtmean, cf.c::jtfolk | src/j_graph_jsource.rs, src/j_graph_scan.rs, Reduce execution lowering | +/%#은 **단항 호출**에서만 Mean 후보. 길이 0·1·2, cell shape, promotion, FP 순서/overflow/prototype을 증명. rewrite로 1-pass fusion이나 산술 재결합을 자동 승인하지 않음 |
| JX-11 / C | **Prefix Scan / Infix Window / MovingAverage:** ap.c::jtpscan/jtmovfslash/jtmovavg | src/j_graph_scan.rs, src/j_graph_jsource.rs, semantic Window/Scan lowering | prefix와 sliding window의 결과·순서·NaN/overflow·window 길이·fallback 독립 검증. Mean 후보에서 다시 발견한 Window 후보의 source provenance 유지 |
| JX-12 / C | **Key/GroupBy + GroupReduce:** ao.c::jtkeyct/jtsldot | src/j_graph_jsource.rs, src/execution_semantics.rs의 GroupBy, src/analysis.rs | key equality/CCT, 그룹 첫 등장 순서, 대표원소, 타입·overflow, empty/sparse를 증명한 후에만 직접 집계 후보를 합법화. 단순 Search 해시 규칙 재사용 불가 |
| JX-13 / C | **Contraction·full Dot/GEMM·oblique convolution:** cip.c, gemm.c, ao.c::jtpolymult, cr.c/va2.c의 제한된 sum-times | src/execution_semantics.rs의 Contract, src/analysis.rs, src/lowering.rs | 일반 dot과 제한된 rank-1 sum-times, oblique convolution을 구분. rank/agreement, Fit, int overflow/retry, float order, sparse 및 BLAS capability 검증; 라이브러리 route는 후속 target 결정 |
| JX-14 / C | **Grade/Sort/Ranking·order statistic:** vg.c, vgsort.c, vgranking.c, vg.c::jtordstat | src/j_graph_jsource.rs, Grade/Ranking semantic basis, src/lowering.rs | tie/order, key/type/range, rank, index 오류를 증명. order-statistic의 내부 **RNG draw**가 후속 J 난수 상태에 미치는 영향 미확정이면 pure quickselect rewrite 금지; state trace 음성 테스트 |
| JX-15 / C·§P | **Search/IndexOf·Interval I. 별개:** vi.c, viavx.c, viix.c, viavx2.c | src/index_ops.rs, src/logical_ir.rs의 SearchDescriptor, src/physical.rs | 검색 세부 구현은 **§P.1과 FW-02/03/04/12**로 위임. dyadic I.의 정렬/interval witness, i./i:/e. 대표·결과 모드 및 E. window 구별; tolerance 미증명 최적화 금지 |
| JX-16 / C | **byte char-map LUT·Boolean/sparse mask→indices:** cf.c::jthkiota, v.c::jtcharmap | src/j_graph_jsource.rs, IndexSpace/Lookup/Gather semantic basis | byte alphabet, 256-entry 범위, 중복 첫 일치, invalid index의 오류와 mask/sparse-fill·rank 검증. source-form candidate에서 특수 LUT/compact 실행까지 단번에 연결 금지 |
| JX-17 / C | **Cut→Scan→Raze 및 Box+Append/Raze:** ca.c, cc.c::jtrazecut1/2, vo.c::jtjlink | src/j_graph_ir.rs, src/j_graph_fusion.rs, ConcatAssemble/Result Assembly 경계 | **zero-cut 시 source specialized 경로와 generic의 결과 축 불일치**를 별도 blocker로 남김. 원본 의미, fill/shape, boxed lifetime, effect 및 fallback 일치 전에는 변환 불허 |
| JX-18 / C | **Shape/Count 단축·상수 수치·Hook threshold:** ca.c, v.c, cf.c, va1.c, vx.c, vz.c | src/j_graph_ir.rs facts, src/analysis.rs, numeric lowering | #@, 및 bound constants는 shape/count와 계산 요구를 분리하되 error/prototype/sparse를 보존. floor-log/digits/power/deadband는 domain, Fit, FP 반올림, SIMD guard/NaN 음성 테스트 |
| JX-19 / C | **Under/Each·inverse precomputation:** cu.c::jtsunder 등 | src/semantic.rs, src/j_graph_ir.rs, src/execution_semantics.rs | forward→inner→inverse, dynamic name binding 시점, effect/alias, identity와 역함수 유효성 입증. stale inverse cache 또는 이름 재정의 시 reference fallback |
| JX-20 / C | **Virtual View/Gather/Reshape/Compress/Catenate/Result Assembly:** v.c, vfrom.c, vf.c, vrep.c, vcat.c, result.h | src/physical.rs, src/storage.rs, src/lowering.rs 및 result-assembly 경계 | Box→Open 보편 소거 금지. contiguous/noncontiguous, recursive box, view lifetime, alias/pristinity/usecount, rank/shape/error 검증; 원본을 변경해 관찰 가능한 값이 바뀌는 alias negative test |
| JX-21 / C | **sparse-specific execution:** cpdtsp.c, vgsp.c, visp.c, vfromsp.c | src/sparse.rs, tests/sparse_runtime.rs, src/execution_semantics.rs | sparse axes/fill/empty/prototype, boxed/numeric 타입과 density/resource, dense와 다른 오류·순서 검증. sparse를 불투명 dense 배열이라고 가장한 fuse/convert 금지 |
| JX-22 / C | **Amend/Scatter·buffer donation·lifetime:** am.c, m.c, p.c, cx.c | src/runtime.rs, src/storage.rs, src/physical.rs, effect/assignment 경계 | index/type/readonly, shared alias, recursive boxes, assignment commit/error 순서 및 failed-retry를 증명. SSA liveness만으로 inplace 허가하지 않음; source mutation negative tests |
| JX-23 / C | **동적 name/locale lookup cache:** sc.c, cx.c | src/semantic.rs, src/runtime.rs, name-version/binding runtime | late binding, locale epoch, invalidation, redefinition/reentrancy, thread safety를 검증하고 cache hit/miss 출력 동일성 비교. 이름 참조 cache를 Graph 상수 전파로 취급하지 않음 |
| JX-24 / C | **명시적 M. memo·RNG generate/shape composition:** a.c::jtmemo, vrand.c::jtrollk, vg.c stateful route | src/semantic.rs, src/runtime.rs, src/execution_semantics.rs의 effect/state contract | M.의 사용자 지정 memo 동작과 임의 CSE를 구분. cache key/효과/수명, RNG seed·draw 순서·state trace를 관찰 가능한 계약으로 증명. 미지원이면 RuntimeSemantic/Unknown 유지 |
| JX-25 / D·FW-11~17 | **Physical target·자원·실측 cost 승인:** jsource SIMD/AVX, GEMM, hash, view, inplace, memory | src/lowering.rs, src/physical.rs, src/j_graph_resource.rs, src/fusion_planning.rs | baseline CPU와 독립 3방향 검증 후 target capability, hard allocation bytes, measured latency/memory를 분리. 동일 의미·guard/fallback·실측 win 없으면 SIMD/BLAS/GPU route 및 캐시 임계값 변경 보류 |
| JX-26 / 최종·FW-18 | **반복 독립 검토·미조사 잔여:** primitive numeric/allocator/architecture-specific paths, 새로운 jsource commits | §4.1.3.6 A–M, 이 §Q, §O.5, §P.1, reports/ | **원본 C 진입·fallback / J 의미·반례 / Graph-A3 provenance / guard-effect·runtime / target-cost·bench**를 서로 독립 재검토. commit·OS/toolchain·실행 명령·pass/fail/ignored·oracle pin/범위·known gaps를 남기고 FW/P와 체크 동기화 |

###### Q.1 JX-01/FW-01 정적 source/frontend 감사 (2026-10-06)

**상태:** JX-01 [ ], FW-01 [ ] 유지. 고정 jsource revision에서 현재 registry가 지목한 **C/H 원본 16/16 파일 존재**를 직접 확인했다. 대표 심볼·텍스트는 15개 파일에서 발견했다. `vcat.c`의 `boxed ownership transfer`는 실제 C 심볼이 아닌 설명이므로 일치로 세지 않았다. **파일/심볼 확인은 guard·fallback 전체, 실제 J 실행 또는 최적화 의미론의 증명이 아니다.**

`src/j_graph_jsource.rs::JSOURCE_FAMILY_RULES`의 16개는 **AnalysisOnly 7 / ExistingAnalyzer 1 / AwaitingFrontendOrFacts 4 / DownstreamOnly 4**로 나뉜다.

| family / 현재 발견 상태 | source → pinned C/H | 검증할 조건·reference 복귀 / RustJ 책임·연계 게이트 |
|---|---|---|
| `ReductionFastPath` / AnalysisOnly | `f/ y` → `ar.c::jtreduce` | empty/singleton/two-item, identity, overflow → Reduce; ExecutionAlgorithm / JX-10 |
| `MeanIdiom` / AnalysisOnly | 단항 `(+/ % #) y` → `cf.c::jtfolk`, `ar.c::jtmean` | dyad 제외, rank·FP order → 원 fork; ExecutionSemantics / JX-10 |
| `WindowAlgorithm` / AnalysisOnly | `f\ y`, `x f\. y` → `ap.c::jtmovfslash` | Scan≠Window, length·NaN/overflow → generic; ExecutionAlgorithm / JX-11 |
| `SearchAlgorithm` / AnalysisOnly | `i.` / `i:` / `e.` dyad → `vi.c::indexofsub` | first/last/member, CCT·rank·boxed/sparse → sequential; ExecutionAlgorithm / §P·JX-15 |
| `IntervalLookup` / AnalysisOnly | 이항 `x I. y` → `viix.c` | sortedness/type/empty, 단항과 구별 → baseline; ExecutionAlgorithm / JX-15 |
| `GatherCopyOrView` / AnalysisOnly | `x { y` → `vfrom.c::jtget1cell` | bounds/alias/contiguity → copy; PhysicalPlanner / JX-20 |
| `ReindexCopyOrView` / AnalysisOnly | `$` / `|.` / `|:` → `vf.c` | fill/shape/usecount → materialize; PhysicalPlanner / JX-20 |
| `MapReduceStreaming` / ExistingAnalyzer | `f/@:g` → `va2.c::jtfslashatg` | dense/type/empty/inplace/overflow → generic map/reduce; 기존 GraphFusion / JX-09 |
| `ResultAssemblyDemand` / AwaitingFrontendOrFacts | box/open/raze → `result.h` | recursive boxes·raze·effect → generic assembly; ExecutionSemantics / JX-17/20 |
| `GroupAggregate` / AwaitingFrontendOrFacts | `u/.`, `f//.` → `ao.c::jtkeyct/jtsldot` | CCT·group order·representative/type → generic group; ExecutionAlgorithm / JX-12 |
| `MatrixContraction` / AwaitingFrontendOrFacts | `+/ . *` → `cip.c::jtpdt`, `gemm.c` | rank/Fit/overflow/FP order/sparse → generic dot; ExecutionAlgorithm / JX-13 |
| `GradeRanking` / AwaitingFrontendOrFacts | `/:`, `\:` → `vg.c` | tie/order/type/axis → generic grade; ExecutionAlgorithm / JX-14 |
| `TolerantHash` / DownstreamOnly | tolerant search → `viavx2.c` | CCT 비추이성/±0/NaN → sequential; ExecutionAlgorithm / §P·JX-15 |
| `SparseAlgorithm` / DownstreamOnly | sparse dot/grade/search/from → `cpdtsp.c` 등 | axes/fill/empty/type → sparse reference; ExecutionAlgorithm / JX-21 |
| `BufferOwnership` / DownstreamOnly | boxed concat/reshape/compress → `vcat.c` 등 | alias/usecount/recursive box → allocating result; PhysicalPlanner / JX-20/22 |
| `NameLookupCache` / DownstreamOnly | dynamic name/locale → `sc.c::jtunquote` | epoch·locale·reentrancy/invalidation → actual lookup; RuntimeBinding / JX-23 |

**registry 외의 source backlog도 유지:** Cut→Scan→Raze(`cc.c`, JX-17), oblique convolution(`ao.c`, JX-13), char LUT(`v.c`, JX-16), boolean/sparse→indices(`cf.c`, JX-16), RNG order statistic(`vg.c`, JX-14), RNG shape(`vrand.c`, JX-24), Box+Append(`vo.c`, JX-17), 명시적 `M.` memo(`a.c`, JX-24), Under/Each(`cu.c`, JX-19), bound numeric/deadband(`vx.c/vz.c/va1.c`, JX-18), Amend/Scatter(`am.c`, JX-22), assignment/definition fast path(`p.c/cx.c`, JX-22). 이 목록은 §4.1.3.6 H/K/M의 조사 결과로, **새 실행 후보·전수 조사 완료를 의미하지 않는다.**

**FW-01 프런트엔드 구분:**
- **Word formation:** `src/tokenizer.rs::scan/parse_word_spans`와 `tests/syntax.rs`의 이전 F0 실행 이력은 이번 검사 결과로 재계산하지 않는다.
- **Enqueue/POS:** `src/primitive.rs`, `src/enqueuer.rs`, `tests/enqueuer.rs`. `/.`, `.`, `/:`, `\:`, `;.`, `&.`, `M.`, `?`, `?.`, `!.`의 기본 품사 분류는 실제 derived constructor·executor·최적화 허가가 아니다. locative/name-by-value·일부 numeric payload는 Unsupported다.
- **Parser/derived:** `src/parser.rs`, `src/semantic.rs::FunctionEntity`, `tests/semantic.rs`에서 `@:`, Hook/Fork, Rank, Insert/PrefixInfix 일부를 표현한다. Key/Dot/Cut/Under/Memo/Grade 및 동적 NAME·valence·POS·오류/효과 순서의 실행/차분 검증은 남았다.
- **Source opportunity:** `src/j_graph_jsource.rs::discover`는 단항 Mean만 인정하며 `E.` window와 search를 구분한다. 후보는 모두 아직 legal/selected가 아니다.
- **추가 회귀 소스:** [커밋 e364535](https://github.com/yunskim/RustJ/commit/e36453575430879e4bc546c62350107a3e698e84)에서 `tests/j_graph_jsource.rs::optimization_vocabulary_pos_is_not_a_compiler_optimization_license`를 추가했다. **Cargo/default·portable/실제 J C oracle은 아직 미확인**.

**실제 CI에서 확인한 미해결 결과(2026-10-06):** 신규 POS 테스트 커밋 [e364535](https://github.com/yunskim/RustJ/commit/e36453575430879e4bc546c62350107a3e698e84)의 [Basis compile probe](https://github.com/yunskim/RustJ/actions/runs/37428376593)는 `cargo test` 중 **기존 `tests/index_ops.rs::member_preserves_cell_shapes_and_empty_query_semantics` 실패**(실제 bool shape `[0]` vs 잘못된 기대 `[2]`)로 종료되어 새 Jsource 테스트 완료 근거가 아니다. J 공식 Dictionary의 `x e. y ↔ (#y)>y i. x` 계약에 따라 **빈 왼쪽 질의는 빈 결과**, **빈 오른쪽 lookup은 길이 2의 거짓 결과**로 분리한 [80ad4a7](https://github.com/yunskim/RustJ/commit/80ad4a73b14099431966d2f697fa73f98778e159)를 반영했다. 이는 테스트의 방향 오류 수정이며 실제 전체 J C binary 비교가 아니다. 같은 e364535의 [Linux milestone](https://github.com/yunskim/RustJ/actions/runs/37428376492)은 **기존 다수 Rust 소스의 `cargo fmt --check` 차이** 때문에 check job 실패, C reference 네 조합의 개별 job은 성공했다. **80ad4a7 후속 default/portable 결과와 신규 POS 테스트 통과는 이 기록 시점에 미확정**이므로 JX-01/FW-01/FW-04·전체 CI [x] 금지.

**다음 한 단계:** 새 테스트의 실제 default·portable 결과 및 pinned J C의 POS/derived syntax/error 차분을 확보한 뒤 **확인된 의미론 불일치 한 종류 + 회귀 하나**씩 수정한다. FW-01·JX-01 체크는 결과 명령·환경·commit·unsupported 범위가 기록되기 전까지 열어 둔다.

###### Q.2 FW-01 / JX-01 실행 회귀 및 원본 비교 게이트 — 두 번째 이행 기록 (2026-10-06)

**구현 판정: [ ] FW-01 · [ ] JX-01 · [ ] JX-10 유지.** 단항 Mean 후보를 실제로 놓치는 오류를 GitHub Actions [02854b6 진단 로그](https://github.com/yunskim/RustJ/actions/runs/37429620894)에서 재현했다. `(+/ % #) y`는 **전체 Fork 호출은 단항이지만, Graph region의 마지막 `g=%` Apply는 분기 결과 두 개를 받는 이항**이다. 이전 `src/j_graph_jsource.rs::discover`는 join의 `Valence::Monad`를 요구해 Mean 후보를 항상 버렸다. [05dae2d](https://github.com/yunskim/RustJ/commit/05dae2d9c250a466c1f0b89cfdc4b776e72d74c5)는 원래 `RegionKind::Fork` 및 `region.inputs.len()==1`로 **외부 호출의 valence**를 판정하고, join의 `Valence::Dyad`를 확인한다. Graph 실행·순서·원본 function entity는 변경하지 않는다. 고정 `jsrc/cf.c::jtfolk`는 `+/ % #` 패턴에 대해 `f1=jtmean`만 설정하므로 **단항 후보만** 발견하는 제한은 적절하다.

[268c83f](https://github.com/yunskim/RustJ/commit/268c83f656812b2a9fc951cb91c84e5e6f2a368b)는 임시 `MEAN_DIAG` 출력을 제거하고 **outer Fork=단항/inner join=이항** 및 이항 호출의 후보 부재를 테스트한다. 기존 `e.` empty-query 회귀도 [d818a3e](https://github.com/yunskim/RustJ/commit/d818a3e807453327d7cc4cd265dc503acf36d0ec)에서 **Boolean membership 결과**와 **Int `i.` missing sentinel**을 구분해 수정했다. `e.`의 빈 왼쪽 프레임 `[0]` 및 오른쪽 비었을 때 길이 2의 false 결과를 각각 확인한다.

**실제 CI 증거:** Ubuntu GitHub Actions, Rust stable, [268c83f의 Basis compile probe](https://github.com/yunskim/RustJ/actions/runs/37432213914)에서 `cargo test`, `cargo test --features portable`, `cargo build --release`는 **성공**했다. Clippy는 기존 `src/index_ops.rs:330`의 inclusive-range 표현 경고로 실패했다. [dd2c118](https://github.com/yunskim/RustJ/commit/dd2c118de8bb025a2f4fa3f21be9f22063c6be52)로 범위 검사를 동일한 의미의 `(64..=MAX_PREHASH_ITEMS).contains(&items)`로 바꿨고, [37432369878](https://github.com/yunskim/RustJ/actions/runs/37432369878)에서 **default + portable tests / release build / Clippy 모두 통과**했다. 이는 *Rust 회귀 승인*이지 J 언어 전체 의미론 인증이 아니다.

**남은 독립 게이트:** Linux milestone [37432213939](https://github.com/yunskim/RustJ/actions/runs/37432213939)의 `check`는 `cargo fmt --check`에서 실패했다. 포맷 차이는 13개 Rust 소스·테스트 파일에 걸쳐 있으므로 **CI 전체 성공으로 표시하지 않는다**. j64/j64avx2 C reference job 결과와 검사 스위트 성공 여부는 별도 기록한다. [a91dd57](https://github.com/yunskim/RustJ/commit/a91dd5741f298e40782cea9a73801ff6dad0863e)는 `tools/conformance.py::cases`에 **단항 Mean의 보통 배열/길이 1/빈 배열/다차원 배열**을 추가해 pinned J C와 Rust 결과를 차분할 준비를 했다. 이 신규 C 차분 결과는 **아직 승인되지 않았다**. Native J 그래프 의미론 검증·JX-10 특수 실행 선택은 미완료다. 후속 실행에서 실패가 나면 `case + oracle output + Rust output + pin + backend`를 기록해 의미론 차이와 미지원/상위 원본 특수경로 차이를 분리한다.

###### Q.3 FW-01/JX-01 GitHub CI 녹색 검증과 기준선 기록 (2026-10-06)

**코드/검증 커밋:** [89bbfd0](https://github.com/yunskim/RustJ/commit/89bbfd0e55163c90b5059e90d10b0ba0bd87ded5). GitHub Actions [Linux milestone 37433098574](https://github.com/yunskim/RustJ/actions/runs/37433098574)의 **5개 job이 모두 성공**했다. `check`는 Python 도구 테스트, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo test --features portable`, `cargo build --release`, milestone 예제 실행을 모두 통과했다. [Basis compile probe 37433098567](https://github.com/yunskim/RustJ/actions/runs/37433098567)도 완료·성공했다. 이전 포맷 오류는 GitHub CI rustfmt 제안 104 hunk / 13 파일을 **의미론 변경 없이** 원자적으로 적용하여 제거했다.

**고정 J C 차분 실행:** 같은 Linux milestone의 참조 job 네 개 모두 성공했다. CI는 실제 `jsource` commit `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`을 fetch·빌드했으며, [ed56b33](https://github.com/yunskim/RustJ/commit/ed56b336dd198b661cb1e4e80814488b4d890a57)부터 `tools/conformance.py`와 `tools/word_conformance.py` 보고서의 `reference_revision`에도 이 SHA를 기록한다.

| 고정 C variant × Rust backend | 실행 케이스 | 일치(pass) | 예외로 이미 기록된 known deviation | 예상 밖 failure |
|---|---:|---:|---:|---:|
| j64 × default | 5,384 | 5,383 | 1 | 0 |
| j64 × portable | 5,384 | 5,383 | 1 | 0 |
| j64avx2 × default | 5,384 | 5,384 | 0 | 0 |
| j64avx2 × portable | 5,384 | 5,384 | 0 | 0 |

이 known deviation은 **`(i.2 3) -"1 0 (i.2 3 4)`에서 동일한 값·형상이나 J64가 Float, Rust가 Int를 내는 기존의 한정된 타입 차이**로, `tools/conformance.py::known`에 사례·type·shape guard가 박혀 있다. 신규 단항 `(+/ % #)`의 보통 길이·길이 1·빈 배열·rank/frame 네 차분 사례는 이 예외에 해당하지 않고 정상 통과했다. `failed=0`는 이 **supported subset**과 known deviation 분리 기준으로만 유효하다. J upstream 전체 테스트 스위트나 모든 numeric/locale/effect 의미론을 증명한 것은 아니다.

**체크리스트 상태는 [ ] FW-01 / [ ] JX-01 / [ ] JX-10을 그대로 유지한다.** 이번에 끝난 것은 해당 부분집합에 대한 Mean source-candidate 회귀 및 독립 C 차분 **하위 검증**이다. FW-01은 Key/Dot/Cut/Grade/Under/Memo 등 미구현 frontend constructor, rank/effect/error 및 다른 단일 단계의 C 차분 검토가 남아 있다. JX-01은 고정 원본 source family의 **각 guard·fallback 전수 범위**를 확정해야 하며, JX-10은 proof/Guard/순차-특수 3방향 검증과 성능 실측 전에는 최적화 실행을 승인할 수 없다.

**다음 작업:** FW-01/JX-02의 지원되지 않는 **Key `/.` 파생 동사 구성**부터 고정 J C parser/POS·valence와 RustJ `VocabularyPrimitive` 구분을 입력별로 조사한다. 비지원 구문은 `AwaitingFrontendOrFacts`로 유지하고, 반례와 정상/부정 테스트를 확보한 뒤 한 연산군씩 구현한다.

###### Q.4 FW-01/JX-02의 첫 Key `/.` 파생 동사 구성 — 실행과 최적화 허가는 분리 (2026-10-06)

**현황:** `FW-01 [ ]`, `JX-02 [ ]`, `JX-12 [ ]`, `JX-01 [ ]` 유지. `/.`를 **word/POS 인식에서 동사 피연산자 파생 동사의 비실행 구성까지** 이행한 부분 단계다. Key·Oblique의 범용 연산, 명사 gerund의 구성, GroupBy 최적화는 아직 구현·승인되지 않았다.

**고정 원본 계약:** `jsrc/ao.c::jtsldot`는 `u/.`를 하나의 derived verb로 생성하면서 **단항 `jtoblique` / 이항 `jtkey`**를 별도 등록하고 세 innate rank 모두 `RMAX`로 설정한다. 동사 피연산자는 직접 보존하고, **명사 gerund 피연산자는 `fxeachv`로 별도 해석**하므로 '모든 명사 피연산자'가 유효하다는 뜻은 아니다. `jtkeyct`는 분류에 `CCT`와 `indexofsub(IFORKEY)`를 쓰고 group별 실행으로 넘어가며, sparse·boxed·특수 reduction 경로는 guard/fallback이 다르다. 따라서 Key 실행을 일반 해시 GroupBy로 단순 치환하면 안 된다. 이 기록은 고정 SHA `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`의 `ao.c` 및 `cf.c` 확인에 한정한다.

**이번 이행의 세부 체크리스트(전체 FW/JX 게이트와 분리):**
- [x] `src/primitive.rs`의 `AdverbId::Sldot` 추가, `/.`를 중복 `VocabularyPrimitive`에서 제거, `REGISTRY_VERSION = 9`. `EnqueueClass::Adverb` 유지. 단독 modifier는 여전히 실행하지 않는다.
- [x] `src/parser.rs::apply_adverb`의 기존 동사-left 구조 경로에서 `FunctionHead::PrimitiveAdverb(Key)`와 원래 함수 operand를 보존하는 파생 동사 구성. `src/semantic.rs::innate_ranks`에 `[63;3]` 반영.
- [x] `src/j_graph_ir.rs`에서 Key 파생 함수를 **불투명 `GraphForm::Modifier`**로 표현. Reduce/Window/GroupAggregate 후보를 합성하지 않으며 `JsourceFamily::GroupAggregate`는 `AwaitingFrontendOrFacts` 상태 유지.
- [x] 정상·부정 Rust 회귀: `tests/semantic.rs::key_derived_verb_keeps_operator_and_operand_without_licensing_execution`, `tests/j_graph_jsource.rs::key_construction_preserves_an_opaque_graph_boundary_without_groupby_selection`, vocabulary/POS 테스트 및 기존 `tests/primitive.rs` 확장. 단항·이항 syntax는 parse되지만 실행은 명시적 `unsupported`; 명사 `3/.`도 지원된다고 가장하지 않음.
- [x] 고정 J C에 대한 **상태 보존형 구성·binding/alias 6개 fixture**를 `tools/conformance.py`에 추가. [CI 37435150581](https://github.com/yunskim/RustJ/actions/runs/37435150581) **전체 5 job 성공**, [Basis probe 37435150506](https://github.com/yunskim/RustJ/actions/runs/37435150506) 성공. `cargo fmt --check`, Clippy, 기본/portable test, release build 및 Python 도구 테스트가 녹색.
- [ ] 명사 gerund `m/.` 생성에 필요한 `fxeachv`/AR decoding, 유효·무효 gerund 및 오류 우선순위의 pinned J C 비교.
- [ ] 단항 Oblique와 이항 Key의 **독립 순차 reference**, group order/representative, tolerance(CCT)·rank·empty·boxed·sparse·effect/error/overflow 차분을 구현.
- [ ] CPU 특수 recipe/prepare-lifetime·ProofBundle·guard/fallback/무효화·성능 근거. 검증 전에는 GroupBy 실행 경로 선택·최적화 후보 승격 **금지**.

**네 갈래 실제 차분 증거:** `jsource` pinned revision SHA `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`; source-backed OCI/Linux GitHub Actions, seed `20260926`, randomized rounds `100`. `tools/conformance.py`의 보고서에는 동일 pin과 binary/reference SHA256이 포함된다.

| C variant / Rust 구성 | 사례 | 동등 확인 | 기존 좁은 known deviation | 새 실패 |
|---|---:|---:|---:|---:|
| `j64` / default | 5,390 | 5,389 | 1 | 0 |
| `j64` / portable | 5,390 | 5,389 | 1 | 0 |
| `j64avx2` / default | 5,390 | 5,390 | 0 | 0 |
| `j64avx2` / portable | 5,390 | 5,390 | 0 | 0 |

`j64`의 known deviation 1건은 앞의 Q.3에서 기록한 별도 rank 연산의 **Int/Float 타입 차이**이며 Key 테스트의 실패가 아니다. 새 6개 fixture는 **파생 동사 생성·이름 바인딩만** 비교하고 `x u/. y` 또는 `u/. y` 실행을 비교하지 않는다. J upstream 전체 테스트 스위트도 실행하지 않았다.

**구현 단위 커밋:** [78ba45a](https://github.com/yunskim/RustJ/commit/78ba45a9bcbc47e15ef57299a77c7131be523cde) 기능·회귀·C fixture, [de5c4dc](https://github.com/yunskim/RustJ/commit/de5c4dc10b11570836794fda42d3adf84a0c10bf) fmt 정리, [d8db5f9](https://github.com/yunskim/RustJ/commit/d8db5f95991f6116e672d41eaee6876d5faa3f19) Graph 후보 부정 회귀, [a4f4dd0](https://github.com/yunskim/RustJ/commit/a4f4dd078c26d4fe157357dbb65f37b56375dff2) fmt 마무리.

**다음 우선순위:** FW-01/JX-02의 범위 밖까지 Key 구현을 성급히 확장하지 않는다. 프런트엔드 미지원 목록을 유지하고 **FW-02의 독립 sequential `i.` 검색 기준선**부터 착수하여 optimized search와 의미론 검증 경로를 분리한다. 이후 FW-10/JX-08의 두 번째 독립 연산 계열로 Key/Reduce를 검증해 공통 인터페이스 필요성을 판단한다.

**체크리스트 사용 규칙.** 각 JX 행은 **(1) C 원본 pin·조건 확인 → (2) J 의미론/unsupported 범위 확정 → (3) Graph 후보 및 source witness → (4) obligation별 proof/Guard·fallback → (5) 독립 reference·negative·C differential → (6) target/resource/실측 선택**의 여섯 열을 통과해야 완료한다. 실제 결과가 없으면 해당 행은 [ ]로 유지하며, 한 번에 **하나의 의미론 변경 + 해당 회귀/반례 하나**를 우선한다. 실패 또는 upstream drift가 발견되면 해당 연산군의 증명을 무효화하고 FW 관련 선행 게이트까지 되돌아간다. 각 완료 행에는 **JX-ID / code commit / 실행 명령·환경 / passed·failed·ignored / jsource commit·실제 oracle 범위 / fallback·negative 결과 / 측정값 / known gaps / 다음 게이트**를 기록한다. 당장은 **JX-01의 출처·범위 추적과 FW-01(M2)**부터 이어가며 특수 최적화를 새로 활성화하지 않는다.


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


---

<a id="read-advanced"></a>
## 심화 기술 계약 — primitive, Rank, 하드웨어 계획, Flow–Storage

아래는 기존 §4.5~§4.24의 상세 계약을 모아둔 참조 자료다. **핵심 단계 흐름을 이해하는 데 필수 선행 독서는 아니다.** 기존 항목 번호와 앵커는 세부 체크리스트의 교차 참조를 보호하기 위해 보존한다.

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

<a id="extension-primitive-inventory"></a>

### 4.6 확장 primitive 목록 — 기능, 필요성, 채택 상태

**단일 Extension 통합 목록(2026-10-10):** 아래 표 하나에 JAXA/Japchae의 역사적 prototype·연구 항목과 NN-01–NN-34 도입 체크리스트를 합치고 품사·계약·고정 출처·선택적 분해 경로를 보존했다. 체크박스는 **NN 연산군의 유일한 진행 상태**이다. 행의 모든 적용 대상에 NN-G0–G7 검증 증거가 있거나 명시적 보류·제외 결정이 기록될 때만 완료한다. EXT-R*/EXT-H*는 별도 소관의 연구·이력 항목으로 NN 체크박스가 없다. §A2는 공통 Registry/IR 기반 구현 상태를 소유한다. 과거 prototype은 실행 지원 증거가 아니다.

| 저장소 | 확인한 최신 HEAD | 근거의 역할 |
|---|---|---|
| `yunskim/JAXA` | [`12bc0659`](https://github.com/yunskim/JAXA/tree/12bc0659a1e008597bf07449c70029bc61eff93b) · 2026-03-24 | 초기 문제의식; 후속 결정의 우선 근거가 아님 |
| `yunskim/JAXA-complier` | [`ceba0589`](https://github.com/yunskim/JAXA-complier/tree/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368) · 2026-04-26 | 품사를 갖춘 실제 prototype inventory |
| `yunskim/japchae` | [`510c31b5`](https://github.com/yunskim/japchae/tree/510c31b5fe3d4bf6ca0c6fa9aeae8556ca134607) · 2026-06-19 | pooling/dropout 및 상태 설계 이력 |
| `yunskim/jaxa-analyzer` | [`7275d5ba`](https://github.com/yunskim/jaxa-analyzer/tree/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33) · 2026-09-30 | 후기 cast·effect·AD·Flow–Storage 검토; 현행 설계는 RustJ로 이관 |

**출처 키**: [P — prototype registry](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py); [T — historical training modes](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/README.md); [A — extension architecture §§1.10, 3.4–3.5, 4.3, 7.4–7.7](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md); [F — later Flow–Storage scope §8](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/flow_storage_scope_and_compiler_positioning.md); [W — Flow–Storage technical review](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/flow_storage_model_review_and_open_questions.md); [D — pooling/dropout decisions](https://github.com/yunskim/japchae/blob/510c31b5fe3d4bf6ca0c6fa9aeae8556ca134607/documents/decisions.md).

아래 “필요한 이유와 최소 계약”은 원본 기능 설명을 현행 RustJ 설계에 적용한 요구사항이다. 원본 prototype가 이 계약을 모두 구현했다는 뜻은 아니다. [영어 mirror의 같은 목록](PROJECT.md#extension-primitive-inventory).

**확장이 필요한 이유:** 모든 계산이 새 primitive 없이는 표현 불가능하다는 뜻이 아니다. 표준 J로 표현 가능한 계산은 reference definition을 먼저 제공한다. 확장 이름은 parameter schema, shape/numeric/effect 계약, 고수준 graph identity 및 검증된 library/native/external lowering의 연결점을 제공한다. 지원 경로가 없으면 등록 완료로 표시하지 않는다.

모든 source 이름은 ordinary J binding이며 예약 keyword가 아니다. **Adverb → parameter/verb operand를 받아 derived verb 생성**, **Verb → 배열 계산**, **Conjunction → 두 operand로 derived entity 생성**, **등록 API → 선언/관계 설정**을 구분한다. builder의 품사와 생성된 계산 verb의 valence/rank를 혼동하지 않는다.

<a id="nn-extension-checklist"></a>

| 우선 | ID·상태 | 이름/연산군 — 단일 통합 목록 | 표면 품사 | Graph 의미 및 최소 계약 | 선택적 Logical 분해 / 실행 | 고정 출처 |
|---|---|---|---|---|---|---|
| P0 | [ ] NN-01 | `relu`, `sigmoid`, `tanh` | Verb | 원소별 활성화; 편의용 이름/수치 규약(표준 J 식으로 충분하면 별도 원자 op는 선택) 기존 계약: 가장 작은 map/fusion 검증 대상이다. numeric domain·NaN·signed zero·미분 경계 정책을 reference와 비교한다. | Map/elementwise; 기존 J primitive로도 참조 | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py) |
| P0 | [ ] NN-02 | `linear` builder (`torch.nn.functional.linear`) | Adverb builder | 명시적 W/bias를 받는 affine/dense layer; batch·axis 및 parameter schema 기존 계약: 행렬 contraction과 bias 결합을 분석하고 다른 layer의 weight를 구분한다. 단순 matmul과 parameter/resource binding을 분리한다. | MatMul + Bias → GEMM/fused route | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| P0 | [ ] NN-03 | `flatten` (`torch.flatten`) | Verb | J atom order, 지정 축·frame을 보존하는 feature flatten 기존 계약: 공간 layer와 linear 입력을 연결하고 logical reindex를 보존한다. J atom 순서·축 범위가 필요하며 항상 zero-copy라는 뜻은 아니다. | StaticReindex/Reshape; zero-copy는 별도 판정 | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py) |
| P0 | [ ] NN-04 | `conv2d` (historical `conv` name not exposed) | Adverb builder | 2D convolution; kernel/stride/padding/dilation/groups/축과 출력 shape 기존 계약: 국소 window·채널 contraction을 고수준 graph에 남겨 reference/library/kernel 경로를 비교한다. kernel·stride·padding·dilation·bias 및 explicit weight resource 계약이 필요하다. | Window/Gather + Contract/Reduce 또는 외부 Conv 커널 | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| P0 | [ ] NN-05 | `avg_pool2d`, `max_pool2d` builders | Adverb builders | 윈도우 pooling; padding 분모·NaN·동률·빈 window 기존 계약: window 접근·겹침·reduction을 보존한다. padding 포함 분모와 빈 window 계약이 필요하며 producer/consumer fusion은 후속 판단이다. · window reduction과 backward index 요구를 분석한다. NaN·동률·padding·빈 window와 backward 선택 규칙을 명세한다. | WindowReduce / library | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [D](https://github.com/yunskim/japchae/blob/510c31b5fe3d4bf6ca0c6fa9aeae8556ca134607/documents/decisions.md) |
| P0 | [ ] NN-06 | `softmax`, `log_softmax` builders | Verb (축 계약 미정) | 축별 수치안정 정규화; empty/Inf/NaN·축 규약 기존 계약: exp/map와 axis reduction을 한 구조로 분석한다. 안정화 방식·축·빈 입력·무한대/NaN을 명세하고 고정 fusion 경계로 취급하지 않는다. | MaxReduce + Exp + SumReduce + Normalize / fused | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py) |
| P0 | [ ] NN-07 | `cross_entropy` builder | Verb (dyad) | label 형식, log 안정화, reduction 및 손실 반환 shape 기존 계약: loss 축 reduction과 입력별 미분 계약을 드러낸다. logits/probability 여부·label 형식·평균/합 reduction·log 안정성을 먼저 확정한다. | Gather/LogSumExp/Reduce 또는 전용 loss | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| P1 | [ ] NN-08 | `conv1d`, `conv3d`; depthwise/grouped via `groups` parameter | Adverb builders | 공간 차원·groups·channels/innate cell rank 계약; conv2d 단순 이름 바꾸기 금지 기존 계약: 일반 convolution과 다른 채널 연결·재사용 구조를 분석한다. multiplier와 출력 shape를 명세해야 하며 prototype의 identity shape rule만으로 등록할 수 없다. | Window + Contraction / target Conv | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py) |
| P1 | [ ] NN-09 | `layer_norm`, `rms_norm`, `batch_norm` builders | Adverb builders | feature/batch 통계축, epsilon, affine, training/inference running-state 구분 기존 계약: 배치 축 reduction, 학습/추론 차이, running-stat read/write를 드러낸다. epsilon·통계 축·갱신 규칙이 필요하며 고정 barrier는 semantic 계약이 아니다. · 배치 통계와 sample-local reduction을 구분하고 affine parameter·epsilon·축을 검증한다. sample 독립이 reduction 내부 동기화 부재를 보장하지 않는다. | Reduce + Normalize + Scale/Shift; effect 있는 BN은 별도 | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| P1 | [ ] NN-10 | `gelu`, `silu` | Verb | 활성화 수치 정의와 근사 버전 구분 기존 계약: 정확식과 근사식을 구분한 numeric 계약으로 backend 간 오차를 검증한다. 단순한 이름만으로 approximation을 선택하지 않는다. | Elementwise; fused activation | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py) |
| P1 | [ ] NN-11 | `embedding` builder | Verb (후보) | 정수 인덱스 조회·범위 오류, padding index, 선택적 weight 공유 기존 계약: parameter gradient만 있고 index 미분은 없는 사례로 AD 분해를 검증한다. 입력별 미분 가능성과 scatter/accumulate 계약이 필요하다. | Gather; 역방향은 indexed scatter-add | [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| P1 | [ ] NN-12 | `scaled_dot_product_attention` builder | Verb (다중 인자 결합 규약 미정) | Q/K/V, scale, causal/padding mask, layout·head 축, 출력 dtype 기존 계약: contraction–softmax–contraction 구조를 유지해 tiled/library 경로를 비교한다. mask·scale·축·dtype 계약이 필요하며 FlashAttention 지원을 뜻하지 않는다. | MatMul + Mask + Softmax + MatMul / fused attention; FlashAttention과 동의어 아님 | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py) |
| P1 | [ ] NN-13 | `rotary_embedding` (provisional; no matching public functional API) | Verb (후보) | 위치 인덱스, 회전 축, 주파수/precision 수치 규약 | Slice/Pairwise Map/Rotate; 단순 positional add는 기본 J 조합 | RustJ 신규 후보 |
| P1 | [ ] NN-14 | `matmul` / existing-J contraction recognition | 기존 J 별칭 / contraction 인식 | `+/ .*` 같은 기존 J 표현의 인식·최적화 기준; 새 필수 문법/extension 아님 기존 계약: `+/ .*` 등 표준 J reference로 표현 가능한 계산의 contract/lowering alias 후보이며 필수 새 syntax가 아니다. | Dot/Reduce/Rank → GEMM/batched GEMM | [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| P1 | [ ] NN-24 | `logsumexp` (`torch.logsumexp`) | Verb (축/keepdims 계약 미정) | softmax·loss의 안정적 reduction 공통부; reduction 축·empty reduction(-∞ 가능 여부)·Inf/NaN·dtype/누산 정밀도와 반환 shape를 명시. 일반 `^.`와 `+/`의 무조건 재결합을 허용하지 않음. | MaxReduce + Exp + SumReduce + Log; 검증된 numeric rule에 한해 fused reduction | [ONNX ReduceLogSumExp](https://onnx.ai/onnx/operators/onnx__ReduceLogSumExp.html) |
| P1 | [ ] NN-25 | `topk` (`torch.topk`; no assumed `argpartition` API) | Verb (k·axis 인자 계약 미정) | MoE routing·토큰 sampling·검색의 값과 index 동시 결과, stable tie-break, NaN/empty/k 범위 및 출력 조립을 명세. 기존 J grade/sort 구현과 의미 비교; 새 언어 primitive 강제 안 함. | Grade/Sort + Select 또는 증명된 selection kernel | [ONNX TopK](https://onnx.ai/onnx/operators/onnx__TopK.html) |
| P1 | [ ] NN-26 | `group_norm`, `instance_norm` builders | Adverb builders (후보) | channel grouping·group 수 divisibility·batch/sample 축·epsilon·affine·statistics precision을 구분; 기존 `layernorm`/`batchnorm`의 파라미터 차이만으로 동일 계약이라 가정하지 않음. | Reshape/Group + Reduce + Normalize + Affine; library route | [ONNX GroupNormalization](https://onnx.ai/onnx/operators/onnx__GroupNormalization.html) |
| P1 | [ ] NN-27 | `segment_reduce`, `embedding_bag` builders | Verbs/builders (후보) | variable-length offsets/lengths·빈 segment·sum/mean/max·padding index·반복 index·gradient accumulation 의미. 기본 `SegmentView`와 `Reduce` 재사용, 신규 공통 IR 계층 금지. | SegmentView + Reduce / Gather + SegmentView + Reduce | [PyTorch segment_reduce](https://docs.pytorch.org/docs/stable/generated/torch.segment_reduce.html), [PyTorch native operator interface](https://github.com/pytorch/pytorch/blob/main/aten/src/ATen/native/README.md) |
| P1 | [ ] NN-28 | `scatter_reduce`, `index_add` builders | Verbs (후보; 기존 execution basis 이용) | sparse gradient·MoE reassembly의 중복 index 충돌, 순서·결정성, overwrite vs add/max/min, bounds 및 J observable error를 명세. `Scatter`/`ScatterCombine` 실행 Basis는 **이미 존재**하므로 신규 IR primitive 요구가 아님. | 기존 Scatter/ScatterCombine + Reduce; 원자/분할 병렬화는 downstream 합법성 검증 | [ONNX ScatterND](https://onnx.ai/onnx/operators/onnx__ScatterND.html), [ONNX ScatterElements](https://onnx.ai/onnx/operators/onnx__ScatterElements.html) |
| P2 | [ ] NN-15 | `dropout` builder | Adverb builder | 명시적 RNG seed/state/version, 학습/추론, mask 저장·재사용 기존 계약: 학습/추론 차이와 RNG 의존성을 명시해 잘못된 CSE·재계산을 막는다. seed/state·scaling·mask 재사용 계약이 필요하다. | RNG + Mask + Scale; 효과/재현성 보존 | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [D](https://github.com/yunskim/japchae/blob/510c31b5fe3d4bf6ca0c6fa9aeae8556ca134607/documents/decisions.md) |
| P2 | [ ] NN-16 | `vjp` (`torch.func.vjp` reference; source/AD API undecided) | 입력별 VJP verbs / 과거 adverb factory | 입력별 독립 backward 계산, residual 필요성, gradient shape 기존 계약: 학습 계산도 일급 graph로 표현한다. 후기 설계는 입력별 data/parameter VJP를 독립 entity로 등록하므로 하나의 backward spelling에 출력·state를 숨기지 않는다. | 구성 가능한 것은 조합 미분, 불가능한 것은 등록된 VJP 커널 | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| P2 | [ ] NN-17 | `SGD`, `Adam`, `AdamW` (`torch.optim` builders; unexposed) | Adverb builders | step, weight, gradient, moment, decay의 explicit read/write/version 기존 계약: weight·gradient·moment·step의 read/write 및 version 의존을 분석한다. mutable state는 explicit StateResource이며 in-place/fusion/실행 시점은 별도 결정이다. | Elementwise updates + Reduce(필요 시); optimizer state는 `StateResource` | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| P2 | [ ] NN-18 | `register_autograd` (`torch.library` inspiration; metadata only) | 등록 API, 계산 연산 아님 | 계산 op가 아닌 정적 등록 API; J inverse/obverse(`:. `) 재정의 금지 기존 계약: shape/type·선형화·residual·공유 resource 계약을 검증하고 미래 AD의 rule을 제공한다. inverse/obverse나 parallel annotation이 아니며 API spelling과 schema는 후속 확정한다. | 추후 AD 변환에서 참조; 현재 자동미분 완료 주장 금지 | [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| P2 | [ ] NN-19 | `to` (`torch.Tensor.to` reference; candidate, unexposed) | Adverb / Verb / 미결 | mixed precision·residual 보존·side output/effect의 설계 대상; `cp`/`emit` surface 채택 미결 기존 계약: conversion을 producer 구조와 함께 분석해 mixed-precision 후보를 보존한다. rounding·overflow·NaN 계약이 필요하며 adverb 품사만으로 fusion/in-place를 강제하지 않는다. · 독립 conversion graph node를 표현한다. device 전송 기능이 아니며 별도 커널 강제와도 다르다. 지원 dtype family와 numeric rule은 추가 확정이 필요하다. · gradient·loss·통계의 side output과 write/accumulate 효과를 보존한다. destination·충돌·누적 순서를 명세한다. 후기 문서는 독립 surface op와 Write/Accumulate 중 표현 선택을 미결로 둔다. · backward residual의 lifetime와 저장/재계산 trade-off를 분석한다. 강제 보존·저장 후보·실제 materialization을 구분하며 후기 문서는 surface 채택을 미결로 둔다. | Type conversion, explicit Value/State effect; physical 배치 아님 | [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md), [F](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/flow_storage_scope_and_compiler_positioning.md), [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py) |
| P2 | [ ] NN-29 | `interpolate`, `grid_sample` builders | Adverb/Verb builders (후보) | 영상·feature resampling; nearest/linear/cubic, coordinate transform, align_corners, padding, out-of-bounds 및 빈 spatial extent를 구분. 정적 reshape와 다른 **값 보간** 연산. | Gather + WeightedMap 또는 Window/Interpolation kernel; graph 의미는 유지 | [ONNX GridSample](https://onnx.ai/onnx/operators/onnx__GridSample.html) |
| P2 | [ ] NN-30 | `rand`, `randn`, `bernoulli`, `multinomial` (explicit RNG contract pending) | Verbs/explicit-key builders (후보) | 초기화·샘플링·dropout의 공통 RNG 의미; explicit key/state, shape/dtype, seed 재현성, device 간 sequence 보장 범위, 분포·수치 결과를 명시. J 기존 RNG 어휘·상태를 임의 변경하지 않으며 hidden RNG 효과를 감추지 않음. | 명시적 RNG 생성 연산(key 또는 StateResource) → 분포별 Map/변환; RNG 생성 자체는 기존 순수 Basis만으로 분해 불가 | [JAX explicit PRNG key model](https://docs.jax.dev/en/latest/jax.random.html), [StableHLO RNG spec](https://openxla.org/stablehlo/spec) |
| P3 | [ ] NN-20 | `conv_transpose2d`, `adaptive_avg_pool2d`, `adaptive_max_pool2d` | Adverb/verbs (후보) | 업샘플·동적 출력 크기·경계/윈도우 규약 | Scatter/WindowReduce 또는 전용 kernel | RustJ 신규 후보 |
| P3 | [ ] NN-21 | `quantize_per_tensor`, `dequantize` candidates | Verbs (후보) | scale/zero-point, saturation/rounding, accumulate dtype 계약 후에만 | Q/DQ + Contract 또는 정수 GEMM | RustJ 신규 후보 |
| P3 | [ ] NN-22 | KV-cache/paged attention (no stable PyTorch public op; `kv_cache_update` unexposed) | 효과 있는 연산 (후보) | 추론 상태·버전·alias/순서·증분 인덱싱; effect-free attention과 분리 | Gather/Scatter + Attention, backend-specific fused route | RustJ 신규 후보 |
| P3 | [ ] NN-23 | `grouped_mm` (`torch.nn.functional`; MoE routing still composed) | Verbs/builders (후보) | routing index·capacity/drop/pad·재조립·gradient contract 기존 계약: 논의 중 역할/예시 이름이다. `@.`는 표준 J primitive이며 확장 목록에 넣지 않는다. drop/pad·token/probability 공동 매핑·재조립을 명세한 뒤 실제 이름/품사를 정한다. | TopK/Gather/Scatter/SegmentedReduce; 모듈 전체를 필수 원자로 만들지 않음 | [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| P3 | [ ] NN-31 | `RNN`, `GRU`, `LSTM` (`torch.nn` module classes; builders, unexposed) | Adverb builders (후보; whole-sequence opaque 선택) | hidden/cell-state의 명시적 입력·출력, sequence lengths, direction, layout, gate order, 시간축 iteration과 zero-length sequence 의미. 전체 층은 J 합성을 우선하고 fused library가 필요할 때 Graph atom 유지 후보. | Scan/StateMachine + MatMul/Pointwise 또는 검증된 RNN library | [ONNX LSTM](https://onnx.ai/onnx/operators/onnx__LSTM.html) |
| P3 | [ ] NN-32 | `nms` (`torchvision.ops.nms`; not torch core) | Verb (후보) | detection box IoU·score sorting·threshold·tie-break·동적 선택 개수·빈 결과 shape; 단순 Elementwise/Reduce가 아닌 선택 알고리즘. | Grade/TopK + Select/Iterate 또는 전용 NMS kernel | [ONNX NonMaxSuppression](https://onnx.ai/onnx/operators/onnx__NonMaxSuppression.html) |
| P3 | [ ] NN-33 | `roi_align` (`torchvision.ops.roi_align`) | Verb/builder (후보) | ROI 좌표계·sampling grid·output bins·coordinate transformation·bilinear interpolation 및 출력 shape 명세. `grid_sample` 구현 가능성을 우선 검증. | GridSample + Reduce / library ROI Align | [ONNX RoiAlign](https://onnx.ai/onnx/operators/onnx__RoiAlign.html) |
| P3 | [ ] NN-34 | `ctc_loss` builder | Verb/derived builder (후보) | 가변 길이 시퀀스 alignment의 dynamic programming, blank token, 길이 mask, log-domain 안정성, impossible-alignment·gradient·reduction rule 명세. 일반 crossentropy의 별칭 아님. | LogSoftmax + Scan/StateMachine + Reduce 또는 전용 CTC kernel | [PyTorch CTCLoss](https://docs.pytorch.org/docs/stable/generated/torch.nn.modules.loss.CTCLoss.html) |
| — | EXT-R1 · 연구; A2/effect 소관 절 관리 | `store` | Adverb (연구) | 멀리 떨어진 forward/backward 소비자가 같은 결정 결과를 사용하게 한다. write destination·version·반환값·effect 순서가 필요하다. | 기존 A2 / effect 계약 참조 | [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md), [W](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/flow_storage_model_review_and_open_questions.md) |
| — | EXT-R2 · 연구; A2/effect 소관 절 관리 | `load` | Adverb (연구) | routing 결정을 재계산하지 않고 동일 version으로 재사용한다. resource identity·read version·미초기화 오류를 명세해야 하며 전체 POS/operand schema는 아직 확정되지 않았다. | 기존 A2 / effect 계약 참조 | [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md), [W](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/flow_storage_model_review_and_open_questions.md) |
| — | EXT-R3 · 설계 채택; A2에서 관리 | `with` | Conjunction | adjoint 관계·resource binding·numeric/storage 정책을 계산 identity와 함께 검증한다. RustJ 설계에서 채택했지만 runtime 구현 완료는 아니다. device/tile/register/layout은 받지 않는다. | 기존 A2 / effect 계약 참조 | [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| — | EXT-R4 · 탐색 연구; 구현 게이트 아님 | `FFT / IFFT` | Verbs (연구) | 일반 배열 언어의 확장성 연구 사례다. normalization·축·complex dtype 및 adjoint 관계를 확정하기 전 등록된 primitive로 표시하지 않는다. | 기존 A2 / effect 계약 참조 | [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| — | EXT-H1 · 직접 채택하지 않음 | `grad` | Adverb (역사) | 당시 gradient 누적/optimizer 분리 목적을 기록한다. 고정 offset과 inverse-slot 기반 학습 mode는 채택하지 않고 explicit write/accumulate resource 계약으로 재설계한다. | 기존 A2 / effect 계약 참조 | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [T](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/README.md) |
| — | EXT-H2 · 직접 채택하지 않음 | `consume` | Adverb (역사) | 중간 gradient 저장 축소라는 목적은 후보로 유지한다. 즉시 weight update를 기본 의미로 만들지 않고 이전 weight version의 모든 reader 및 effect legality를 먼저 검증한다. | 기존 A2 / effect 계약 참조 | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [T](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/README.md) |

#### 4.6.1 연구 사례와 core J의 경계

다음은 후기 연구의 확장성 검토 사례이며 위 prototype registry의 완성 항목이 아니다.

MatMul·FFT·Embedding·MoE 연구 항목까지 **위 단일 목록**에 통합했다. 과거 JAXA 이름은 출처 이력이지 실행 지원의 증거가 아니다.

`+`, `/`, `\`, `"`, `@:`, `[`, `]`, `@.`, `|:`, `:.`, `^:`는 core J 어휘이므로 확장 primitive로 중복 등록하지 않는다. core와 extension-derived op 모두 같은 semantic/capability/lowering 원칙을 따른다.

#### 4.6.2 최신 연구와 RustJ의 적용 경계

1. **`emit`·`cp`는 미결이다.** 후기 범위 문서 §8은 surface primitive, core effect, annotation/planner 표현을 다시 검토한다. 이전 문서의 “기본 어휘”만으로 채택 완료라 쓰지 않는다.
2. **`with`는 현행 설계에서 채택한다.** ordinary-name conjunction으로 typed semantic contract만 결합한다. 역사 문서의 optimizer/gerund 묶기 예는 자동으로 현재 operand schema가 되지 않는다.
3. **`adjoint`는 관계다.** 비선형에는 선형화 지점과 residual을 갖춘 VJP 계약이 필요하다. `:.`/`^:_1`의 inverse 의미를 adjoint로 바꾸지 않는다. `(loss_adjoint [ (loss emit))`의 parallel 후보는 바깥 fork syntax에서 나오며, `emit`이 실제 adverb binding이면 `(loss emit)`은 modifier application이다. 둘 다 verb일 때만 hook이다.
4. **dtype와 품사만으로 physical 실행을 확정하지 않는다.** 역사 cast 문서의 “adverb이면 fusion 강제”, “narrowing이면 in-place”, “widening이면 새 버퍼”는 현행 불변식이 아니다. numeric correctness·alias·use/liveness·target/cost 판단 후 실제 realization을 선택한다.
5. **상태는 explicit resource다.** prototype의 hidden weight/moment, fixed offset, 고정 barrier/stream/register 숫자를 primitive identity로 복사하지 않는다. `ValueId`, `StateResource`, `BufferId`를 구분한다.

<a id="nn-explicit-adjoint-candidates"></a>
#### 4.6.2a 명시적 Adjoint/VJP 후보 계약 — NN-01~NN-34 (2026-10-11)

**결정:** RustJ는 범용 자동미분/역방향 그래프 생성기를 필수 구성요소로 두지 않는다. Extended primitive의 Forward와 **입력별 명시적 Adjoint/VJP**는 각각 독립적인 배열 변환이며 동일한 J Rank/Cell/Frame, Graph IR, Logical IR, 합법성·비용 분석을 사용한다. 이 표는 위의 **단일 NN-01~NN-34 완료 체크리스트에 종속되는 계약 후보 표**이지 별도의 채택/완료 목록이 아니다. 표의 이름은 아직 등록된 J NAME이나 실행 가능 커널이라는 뜻이 아니다.

**수학적 경계:** 선형 A에는 A*가 정의된다. 비선형 f에는 primal x와 cotangent v를 받는 `VJP_f(x,v)=Df(x)^*v`가 필요하다. 비미분 입력(정수 index, mask, RNG draw, state mutation)에는 임의의 0-gradient/straight-through estimator(STE)를 자동으로 부여하지 않는다. 입력별 cotangent shape/dtype 및 복소 inner product를 검증한다. **Forward·Adjoint의 intrinsic rank가 같다고 가정하지 않으며**, 공유 parameter-gradient는 batch/frame 축 Reduce를 필요로 할 수 있다. J의 prefix agreement, 빈 Frame virtual fill-cell, heterogeneous result assembly, observable error/effect order는 각각 증명해야 한다.

**참고 기준:** [N] NNlib의 호출 가능한 `∇softmax` 등, [P] PyTorch `torch.nn.grad.conv2d_input/weight`, [C] cuDNN `ConvolutionBwdData/Filter`, [O] oneDNN Backward, [R] ChainRulesCore의 primal+pullback 규칙, [J] 과거 JAXA의 독립 `grad_x/grad_w` 및 관계 등록, [D] RustJ의 설계 도출(외부 라이브러리에 동일 이름/API가 있다는 주장이 아님). **외부 API는 참고 근거이지 J Rank·숫자·에러 의미론과의 동등성 증명이 아니다.**

| 계열 | Forward | 분류 | 입력 Adjoint 후보 | 파라미터 Adjoint 후보 | 필요한 primal/residual | Rank/Logical IR 방향 | 미결 계약/주의 | 근거 |
|---|---|---|---|---|---|---|---|---|
| NN-01 | `relu / sigmoid / tanh` | 명시적 VJP 후보 | `relu_vjp / sigmoid_vjp / tanh_vjp` | — | 입력 또는 출력 y, cotangent | Frame별 elementwise Map | ReLU x=0 subgradient, sigmoid/tanh 정밀도 | [D] |
| NN-02 | `linear` | 명시적 VJP 후보 | `linear_grad_input` | `linear_grad_weight, linear_grad_bias` | X, W, dY, input/weight shape | batched dW/db는 공유 축 Reduce | bias broadcast/축 정합 및 누산 정밀도 | [D] |
| NN-03 | `flatten` | 명시적 VJP 후보 | `flatten_vjp (= inverse reshape)` | — | 원본 shape/axis, dY | Frame/Cell 재배열; 복사 강제 금지 | 빈 배열·boxed/heterogeneous J 결과 조립 | [D] |
| NN-04 | `conv2d` | 명시적 VJP 후보 | `conv2d_grad_input` | `conv2d_grad_weight, conv2d_grad_bias` | W, X, dY, stride/padding/dilation/groups, input/output sizes | dX Window/Scatter; dW batched Reduce/Contract | 경계·그룹·padding, 산술 및 레이아웃 증명 | [P], [C], [O] |
| NN-05 | `avg_pool2d / max_pool2d` | 조건부 VJP 후보 | `avg_pool2d_vjp / max_pool2d_vjp` | — | window geometry; max의 선택 인덱스/동률 정책 | window별 Scatter + overlap Reduce | NaN/ties, ceil_mode, count_include_pad, 빈 창 | [O], [D] |
| NN-06 | `softmax / log_softmax` | 명시적 VJP 후보 | `softmax_vjp / log_softmax_vjp` | — | 출력 y, dY, reduction axis | Cell별 Reduce + Map | 안정적 수식, 빈 reduction, Inf/NaN 및 dtype | [N] |
| NN-07 | `cross_entropy` | 조건부 VJP 후보 | `cross_entropy_vjp_logits` | `선택적 class weight VJP 미정` | logits/log-prob 또는 확률, labels, reduction divisor | class-axis Reduce + batch-axis Reduce | 정수 target은 no VJP; ignore_index/label_smoothing 검증 | [D] |
| NN-08 | `conv1d / conv3d` | 명시적 VJP 후보 | `conv1d_grad_input / conv3d_grad_input` | `conv1d_grad_weight / conv3d_grad_weight, bias` | X, W, dY, spatial hyperparameters | spatial Window + batch Reduce | 차원별 layout, groups, dilations, boundary | [C], [O] |
| NN-09 | `layer_norm / rms_norm / batch_norm` | 조건부 VJP 후보 | `*_norm_grad_input` | `*_norm_grad_weight / *_norm_grad_bias` | input, mean/rstd 또는 rms, affine parameters, training flag | normalization axes Reduce, batch statistics 공유 | running-stat StateResource/version; forward residual precision | [O], [D] |
| NN-10 | `gelu / silu` | 명시적 VJP 후보 | `gelu_vjp / silu_vjp` | — | input x 또는 유효 residual, exact/approx mode | elementwise Map | 근사 GELU 모드별 서로 다른 derivative | [D] |
| NN-11 | `embedding` | 조건부 VJP 후보 | `indices: no VJP` | `embedding_grad_weight (scatter-add)` | indices, dY, padding_idx, weight shape | Gather 역방향 Scatter/SegmentReduce | indices 정수; 반복 index, max_norm state, sparse mode | [D] |
| NN-12 | `scaled_dot_product_attention` | 조건부 VJP 후보 | `attention_grad_q / _grad_k / _grad_v` | — | Q,K,V, mask, scale, softmax stats, dropout mask/key | Contract + Softmax VJP + Contract; head/frame Map | masked rows, GQA, dropout RNG, precision, flash backend 별도 | [D] |
| NN-13 | `rotary_embedding` | 조건부 VJP 후보 | `rotary_embedding_vjp (inverse rotation)` | `선택적 frequency VJP는 별도 정책` | position, sin/cos, rotated axes, dY | paired-element Map/StaticReindex | position index no VJP; frequency trainability 미정 | [D] |
| NN-14 | `matmul / J contraction` | 명시적 VJP 후보 | `matmul_grad_input` | `matmul_grad_other` | X, W, dY, transpose flags, batch-prefix frame | transpose/Contract + broadcast axes Reduce | 복소 conjugate adjoint, J prefix-rank 정합 | [O], [D] |
| NN-15 | `dropout` | 조건부 VJP 후보 | `dropout_vjp_input` | — | forward와 동일한 mask 또는 key/version, p, training | masked Map | 새 RNG draw 금지, RNG 재현성/effect barrier | [D] |
| NN-16 | `vjp (explicit invocation)` | 계산 VJP 해당 없음 | `등록된 input-specific VJP를 명시 호출` | — | primal, output cotangent, residual handle | 대상 primitive의 Rank/Cell 계약 상속 | forward graph 자동 미분/생성 금지; 미등록 VJP 오류 | [R], [J] |
| NN-17 | `SGD / Adam / AdamW` | 계산 VJP 해당 없음 | `일반적인 VJP 계약 없음` | — | weights, gradients, moments, step StateResource | elementwise state transition, 명시적 순서 | optimizer state update는 adjoint가 아님 | [D] |
| NN-18 | `register_adjoint (declaration)` | 계산 VJP 해당 없음 | `계산 없음: forward→input-index VJP 관계 선언` | — | dtype/shape/rank, primal/residual, input-slot identity | 등록 메타데이터만; Graph 실행노드 생성 안 함 | J :. / ^:_1 inverse 재정의 금지 | [R], [J] |
| NN-19 | `to (dtype conversion)` | 조건부 VJP 후보 | `floating_cast_vjp (정의 시)` | — | source/dest dtype, rounding/overflow policy | elementwise Map | 정수/양자화 cast는 일반 VJP 없음; STE 기본 채택 금지 | [D] |
| NN-20 | `conv_transpose2d / adaptive pooling` | 조건부 VJP 후보 | `transpose_conv_grad_input / adaptive_pool_vjp` | `transpose_conv_grad_weight, bias` | X,W,dY, output_size; adaptive window/indices | Window/Scatter + Reduce | adaptive max tie/NaN; output_padding/shape consistency | [C], [O], [D] |
| NN-21 | `quantize_per_tensor / dequantize` | VJP 없음 | `일반 양자화의 정확한 VJP 없음` | — | scale, zero_point, rounding, saturation, dtype | 직접 Adjoint 경로 없음 | STE는 별도 surrogate policy; VJP로 위장 금지 | [D] |
| NN-22 | `kv_cache_update` | VJP 없음 | `state mutation 자체 VJP 없음` | — | cache version, index/offset, alias, explicit state | StateResource read/write dependency | pure gather 경로의 VJP와 상태 갱신을 구분 | [D] |
| NN-23 | `grouped_mm` | 조건부 VJP 후보 | `grouped_mm_grad_input` | `grouped_mm_grad_weight` | expert assignment, offsets, X, W, dY | expert별 batched Contract + Reduce | routing index no VJP; capacity/drop/pad/reassembly | [D] |
| NN-24 | `logsumexp` | 명시적 VJP 후보 | `logsumexp_vjp` | — | input/output, reduction axes, stable max stats | Reduce + broadcast Map | empty/Inf/NaN, keepdims, accumulation dtype | [N], [D] |
| NN-25 | `topk` | 조건부 VJP 후보 | `topk_values_vjp (scatter to chosen indices)` | — | forward-selected indices, input shape, dValues | Scatter; repeated collisions require proof | index VJP 없음; ties/stability/NaN policy | [D] |
| NN-26 | `group_norm / instance_norm` | 명시적 VJP 후보 | `group_norm_vjp / instance_norm_vjp` | `affine scale/bias VJP` | per-group stats, input/scale, eps, dY | group-axis Reduce + Map, batch frame | group divisibility, no running stats 혼동 | [O], [D] |
| NN-27 | `segment_reduce / embedding_bag` | 조건부 VJP 후보 | `segment_reduce_vjp / embedding_bag_vjp` | `embedding_bag_grad_weight` | segments/offsets, indices, reduction type, argmax if max | SegmentView/Scatter + Reduce | empty segments, duplicate indices, max ties, sparse accumulation | [D] |
| NN-28 | `scatter_reduce / index_add` | 조건부 VJP 후보 | `scatter_reduce_grad_base / index_add_grad_input` | `scatter_reduce_grad_src / index_add_grad_src` | indices, reduce mode, include_self, winning ties | Gather + collision-aware ScatterCombine | overwrite collisions, min/max tie/order, deterministic accumulation | [D] |
| NN-29 | `interpolate / grid_sample` | 조건부 VJP 후보 | `interpolate_vjp_input / grid_sample_vjp_input` | `grid_sample_vjp_grid (mode dependent)` | grid/coords, input, mode, padding, align_corners | WeightedScatter/Window + Reduce | nearest grid derivative undefined; bounds, NaN, precision | [D] |
| NN-30 | `rand / randn / bernoulli / multinomial` | VJP 없음 | `RNG sampling 자체에 일반 VJP 없음` | — | explicit key/state, distribution parameters | ExplicitRng route, not differentiation | reparameterization/score estimator는 별도 API | [D] |
| NN-31 | `RNN / GRU / LSTM` | 조건부 VJP 후보 | `recurrent_vjp_input / _vjp_initial_state` | `recurrent_vjp_weight / _vjp_bias` | gate activations, hidden/cell states, lengths, direction | reverse-time Scan + per-step Contract/Map | zero-length, dropout, gate order/state version, truncation | [D] |
| NN-32 | `nms` | VJP 없음 | `discrete box selection: 일반 VJP 없음` | — | selected indices, ties, thresholds | selection graph only | continuous surrogate는 별도 op, NN-32 VJP 주장 금지 | [D] |
| NN-33 | `roi_align` | 조건부 VJP 후보 | `roi_align_vjp_features` | `ROI-coordinate VJP는 별도 정책/검증` | ROI coords, sampling grid, feature/input shapes | GridSample adjoint + accumulation | boxes/coordinates gradient 지원 범위 미확정 | [D] |
| NN-34 | `ctc_loss` | 조건부 VJP 후보 | `ctc_loss_vjp_log_probs` | — | log_probs, targets, input/target lengths, alpha/beta DP | log-domain Scan/StateMachine + Reduce | blank/zero_infinity, impossible alignment, padded frames | [D] |

[N]: https://fluxml.ai/NNlib.jl/dev/reference/
[P]: https://github.com/pytorch/pytorch/blob/main/torch/nn/grad.py
[C]: https://docs.nvidia.com/deeplearning/cudnn/latest/developer/graph-api.html
[O]: https://uxlfoundation.github.io/oneDNN/v3.12/dev_guide_convolution.html
[R]: https://juliadiff.org/ChainRulesCore.jl/stable/
[J]: https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md

**구현·수용 순서 (기존 단일 NN 체크리스트의 하위 게이트):**
- [ ] **NN-AG1:** 미분 가능한 입력 slot·cotangent/result shape/dtype·intrinsic/derived rank·axis/broadcast reduce·동적 shape guard를 Forward와 독립적으로 지정한다.
- [ ] **NN-AG2:** residual/activation의 값·버전·lifetime·storage/recompute 후보를 효과·오류 순서와 분리해 증명한다. `cp`/`emit`의 표면 문법 채택 여부는 기존 A2에서 결정한다.
- [ ] **NN-AG3:** 연산별 명시적 VJP를 독립 Graph 연산과 Logical decomposition 후보로 설계하며, 등록 선언은 실행 Graph node가 아니다. NN-16은 등록된 입력별 VJP의 **명시적 호출**만, NN-18은 `register_adjoint`라는 **관계 등록 후보**만 다룬다. `register_autograd`와 자동 backward 생성은 채택하지 않는다. J `:. `/`^:_1`의 inverse 계약은 불변이다.
- [ ] **NN-AG4:** dot-product adjoint 테스트(선형), finite-difference directional/VJP 검사(매끄러운 비선형), shape/rank·empty/frame·dtype/numeric·오류/effect 부정 사례를 수행한다. 테스트 도구가 역방향 프로그램을 자동 생성하는 것은 아니다.
- [ ] **NN-AG5:** NNlib/oneDNN/cuDNN/PyTorch 참조와 동등성 증거를 확보하고, Native/External lowering을 정식 선택하기 전 J 의미 차이·backend 지원 범위·workspace를 기록한다.

**진행 상태:** 34/34 계열 후보 계약을 문서와 `src/nn_extensions.rs::ADJOINT_CANDIDATES`에 연결함. **구현된 VJP 0개로 간주**하며, 의미론 적법성·실행·성능·CI 통과를 주장하지 않는다.

#### 4.6.3 기존 조사·구현 증거 기록 (2026-10-02 스냅샷)

이 절의 코드 대조 기준은 **2026-10-02 조사 당시 checkout `89b87b8`**이다. 현재 `main`의 구현 상태를 뜻하지 않으며 최신 상태는 §12/A1.5를 따른다. `src/primitive.rs`의 `ExtensionPrimitive`/`PrimitiveResolver::resolve_extension_binding`과 `src/runtime.rs`의 parser name-binding seam은 존재한다. 그러나 이 seam 또는 테스트용 extension handle은 위 NN/effect family의 semantic contract와 실행 kernel을 구현한 증거가 아니다. `tests/semantic.rs::unknown_contracts_are_barriers`는 `conv`와 `with`의 unknown contract가 보수적으로 처리됨을 확인하는 기존 테스트다. 이번에는 코드를 읽었으며 실행 테스트를 재수행하지 않았다.

**조사 당시 수행한 작업(기록 전용; 현행 NN 진행 체크박스 아님):**
- 네 저장소의 당시 HEAD를 확인하고 commit 고정 출처로 기능·품사·연구 분류를 기록했다.
- 과거 forward/backward 및 `grad`/`consume`, 후기 cast/storage/registration 어휘를 구분했다.
- 당시 한국어 정본과 영어 mirror에 통합했다.

**기존 미완료 체크 4개의 관리 위치를 통일한다.** parameter/POS/rank는 **NN-G0**, shape/dtype/reference·empty/error는 **NN-G1**, effect/alias/StateResource는 **NN-G2**, 개별 실행 경로는 **NN-G5**, Windows/J 오라클 비교와 오류/효과 검증은 **NN-G6**에서 각각 추적한다. 공통 extension registry 및 분석 기반의 구현 상태는 **§A2**에서만 관리한다. 이 역사 절에서 별도 완료 상태를 더 이상 갱신하지 않는다.

당시 권장 첫 후보(`relu` → `linear`/`flatten` → `conv`/`avgpool2d`)는 현행 **NN-D1/NN-D2** 실행 계획으로 대체했다. 기존 M1–M6/frontend 이행 순서가 계속 우선하고 실제 CUDA 구현 보류는 유지한다.





#### 4.6.3a NN/배열 Extension-first 수용 게이트·실행 체크리스트 (2026-10-10; 설계/계획만)

**결정:** 먼저 ordinary J 이름의 extension을 추가하고 **J Graph IR에서는 의미가 정의된 단일 계산 primitive**로 보존한다. 그다음 **Execution Semantic Lowering / Logical Execution IR**에서 검증된 조합으로 *선택적으로* 분해하거나, 원자 op를 유지한 채 native CPU, 검증된 외부 library, MLIR/StableHLO subset, GPU kernel로 내린다. "확장 → 나중에 native"는 **source API/semantic identity의 불변**을 뜻하며, Graph IR primitive를 반드시 core J 문법 primitive로 승격한다는 뜻은 아니다. 분해와 fusion, 특정 backend 사용은 별도 합법성/수치/비용 검증 결과다. [JAXA 원자/참조 정의 연구(A)](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md), [prototype 목록(P)](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [현행 A2 계약](#a2--extension-primitive-registry와-analysis-contract).

**검증 전용 체크리스트:** 기존 §4.6.3의 미완료 4개를 NN-G0–G7에 합쳤으며, 개별 진행 상태는 **위 단일 표에서만** 관리한다. §A2는 공통 기반, 통합 표는 NN 항목별 수용을 관리한다. 아래 모든 미체크는 *미검증 도입 후보/미완료 과제*이며, 실제 사용 가능함을 의미하지 않는다. P0/P1/P2/P3은 **이 확장 track 내부의 상대 우선순위**이다. 기존 frontend·M3 검증·M4 CPU 실행 gate를 선행/차단하는 신규 milestone이 아니다. JAXA 명칭·품사는 기존 §4.6 prototype의 이력일 뿐, 신규 이름·표면 품사는 호환성 검증 후 확정한다. 특히 stateful layer는 hidden weight를 보유하는 primitive가 아니라 explicit `StateResource` 인자를 가진 계산 및 parameterized builder로 해석한다.

**2026-10-10 독립 연산 커버리지 감사(도입 후보만).** [ONNX 연산 목록](https://onnx.ai/onnx/operators/), [StableHLO 실행 의미론](https://openxla.org/stablehlo/spec), [PyTorch segment reduction](https://docs.pytorch.org/docs/stable/generated/torch.segment_reduce.html), [JAX explicit PRNG](https://docs.jax.dev/en/latest/jax.random.html)를 RustJ 기존 `ExecutionBasisKind`(`Gather`, `Scatter`, `ScatterCombine`, `SegmentView`, `Reduce`, `Scan`, `Grade`, `GroupBy`, `StateMachine`)와 독립 대조했다. **NN-24–NN-34의 누락 후보 11개**를 이 **같은 표**에 추가했고 기존 ID는 보존했다. P1–P3는 상대 우선순위이며 ID는 append-only이므로 표 내 번호가 반드시 정렬되지는 않는다. 고수준 extension 후보가 **새 execution-basis 종류 추가를 뜻하지는 않는다.** 특히 `scatter_reduce`·top-k는 현재 공통 basis/기존 J 기능을 활용할 수 있으므로 별도 source 이름이나 Graph atom은 정확성·성능 증거 후 선택한다. `pad`·`reshape`·`concat`·`split`·`where`·`sort`·`einsum`, residual/MLP/SwiGLU 블록, collectives는 별도 J primitive로 중복 등록하지 않으며 먼저 기존 J·derived verb·Logical IR 경로를 검증한다. 신규 이름/품사는 모두 제안이고, 외부 프레임워크와의 상호운용성 때문에 J Rank·빈 Cell/Frame·fill·오류·effect 의미론을 변경하지 않는다. 참조한 외부 명세 링크는 2026-10-10에 확인한 가변 문서이며 구현 커밋 고정 근거는 아니다. 새 연산 실행 또는 CI 통과를 주장하지 않는다.

**2026-10-11 PyTorch 명칭·intrinsic-rank·후속 spec 계약 (NN-D1a, frontend preview only):** 34개 고유 NN 가족 ID를 바꾸지 않고 공개된 `torch.nn.functional`, `torch`, `torch.optim`, `torchvision.ops`의 **leaf spelling**을 우선하여 목록의 이름을 정규화했다. 예: `cross_entropy`, `avg_pool2d`, `batch_norm`, `scaled_dot_product_attention`, `nms`. `NN-13`/ `NN-22` 등 일치하는 안정적인 PyTorch 공개 API가 없는 설계 후보는 공식 PyTorch op라 표기하지 않는다. PyTorch의 Python 호출 표면이 3인자 이상이더라도 J는 verb의 1·2항 계산과 adverb/conjunction의 parameterized derived verb 생성으로 구분한다. 이것은 PyTorch 문법을 J parser에 수입하지 않는다.

**기계 판독 spec:** `src/nn_extensions.rs`의 `FAMILIES`(34개)는 parameter schema, shape, dtype/promotion, effect, Logical route *candidate* 및 empty/fill·error-order·numeric proof를 각각 명시한다. `NAMES`는 실제 leaf name, PyTorch API 경로, POS, `intrinsic_ranks`/builder 생성 후 `derived_verb_ranks`, frontend-preview 사용 여부를 보관한다. 단항 elementwise(`relu`, `sigmoid`, `tanh`, `gelu`, `silu`)는 `[0,0,0]`; 전체 배열 연산의 직접 verb는 `[63,63,63]`(J unbounded, PyTorch batch broadcasting 아님); parameterized adverb의 **자체** 계산 rank는 `None`이고 생성될 verb의 후보 rank는 `[63,63,63]`이다. Builder axis/config가 미확정인 경우 생성 rank 역시 의미론 검증 전 잠정값이다. `63`을 J Rank의 일반 셀 분해나 축 기준으로 재해석하지 않는다.

**Frontend E2E 연결:** `nn_extensions::frontend_preview_context()`는 별도 명시 opt-in 엔진에서만 카탈로그의 `frontend_preview` NAME에 POS/semantic ID/lowering key를 붙인다. `examples/frontend_e2e.rs`는 원래 J pipeline 검증 이후 `NN_EXTENSION_FRONTEND ... status=RECOGNIZED_ONLY` 출력으로 이름·rank·후속 spec·미검증 gate를 조회하고 `tests/frontend_stages.rs`는 34개 ID·중복·NAME 유지·POS 인식·사용자 이름 우선을 검사한다. `Engine::new()` 기본 어휘는 변경하지 않는다. **현재 parser는 spec을 실행 가능한 FunctionEntity로 고정하지 않는다**; 이름 식별과 실제 Graph atom/CPU 실행/분해·비교 결과는 구분한다. 검증 전 `NN-G1–G7`, `NN-D1` 완료를 표시하지 않는다. PyTorch 참고: [functional API](https://docs.pytorch.org/docs/stable/nn.functional.html), [PyTorch tensor ops](https://docs.pytorch.org/docs/stable/torch.html), [Torchvision ops](https://docs.pytorch.org/vision/stable/ops.html).

**단일 목록 정책:** NN-01–NN-34의 우선순위·체크상태·품사·계약·분해 후보·출처는 **위 통합 표에서만 관리**한다. 이하에는 수용 검증 규약과 실행 단계만 남기며 기능 목록을 반복하지 않는다.

**의도적으로 extension primitive로 중복하지 않을 것:** `+`, `*`, `+/`, `|:`, Rank `"`, `@:`, fork/hook 등 기존 J 어휘; 일반 residual connection·MLP/Transformer block·모델/학습 loop는 **라이브러리의 derived verb 조합**을 우선한다. `matmul`/reshape처럼 기존 J가 표현하는 연산은 이름을 추가하더라도 최적화 경로를 기존 J 표현과 공유해야 한다. `with`는 이미 선택된 *typed semantic-contract conjunction*이고 device/tile 지정 통로가 아니다.

**Extension별 완료 조건 (각 NN-xx 항목을 체크하려면 해당 유형에 적용되는 gate를 모두 통과):** 계산 extension에는 NN-G3/G5의 Graph primitive 및 실행 경로가 필수다. `adjoint` 같은 **선언/등록 API**는 계산 Graph node로 위장하지 않고 별도 registry schema·정적 검증·테스트로 gate를 충족한다. `matmul`처럼 기존 J 계산을 그대로 인식하는 별칭은 별도 extension node를 강제하지 않고 동등한 Graph 분석/Lowering 증거를 제공한다.

- [ ] **NN-G0 / 공개 identity:** 일반 J name 및 실제 POS(verb/adverb/conjunction), builder operand schema, 생성된 verb의 valence/innate rank, late NAME/locale/rebinding 및 버전 지정. 파서 keyword 추가 금지.
- [ ] **NN-G1 / 정밀 의미론:** 표준 J 참조 정의가 가능하면 oracle로 제공하고, 그렇지 않으면 독립적인 수학/프레임워크 oracle을 버전 고정. axis 역할, frame/cell, empty-frame의 prototype/fill/assembly, empty cell, scalar, 동적 shape, type/promotion, NaN/Inf/signed zero, boundary/error precedence를 명세.
- [ ] **NN-G2 / 상태 및 미분:** effect/alias 및 명시적 `StateResource`와 `ValueId` 구분; RNG, 파라미터, buffer/version/read/write ordering; 학습 항목은 입력별 VJP, residual/gradient 합산 규칙. `BufferId`·layout·tile·device 숫자를 의미론에 넣지 않음.
- [ ] **NN-G3 / Graph IR:** 출처·binding/derived-function identity 및 계약을 갖춘 단일 primitive call로 Graph IR에 등록; unknown contract는 보수적으로 barrier/unsupported로 처리하며 실행 가능함으로 오인하지 않음.
- [ ] **NN-G4 / 선택적 분해:** *Graph IR 원본은 보존*하고, 다음 Execution Semantic Lowering/Logical IR에서 분해 후보 + semantic equivalence witness/guard + provenance를 만든다. 불충분한 증거, 수치 순서/오류/effect 차이 또는 비가역 레이아웃 변화 시 분해 불허; 원자 경로 유지 또는 명시적 unsupported.
- [ ] **NN-G5 / 실행 경로:** portable CPU/reference **또는** 검증된 external/library route 하나 이상의 실사용 경로, preconditions 및 fallback/unsupported를 증명. 이름/Graph 등록만으로 implementation complete 체크 금지. Native/GPU/fused/FlashAttention은 각각 독립 완료 상태.
- [ ] **NN-G6 / 검증 증거:** 정상/경계/음성 case의 value·dtype·shape·rank·J error/effect order 비교, 미분 gradient check(해당 시), 각 단계 verifier와 분해 on/off 동등성, target별 reproducible CI SHA·테스트 결과. J 호환식은 pinned jsource와, 신규 NN 의미는 버전 고정 oracle과 비교; 실행하지 않은 것은 `UNRUN`.
- [ ] **NN-G7 / 승격 기준:** 공개 J interface와 observable semantics 그대로 유지하며, 구체 workload의 의미·비용·성능 근거를 확보한 경우에만 optimized native realization/패턴 recognition을 추가. Graph IR atom, Logical 분해, native kernel, **J core primitive 편입**을 서로 다른 결정으로 기록.

**트랙 진행 체크리스트 (설계만 기록; 구현·CI 완료로 간주하지 않음):**

- [x] **NN-D0:** 기존 §4.6 JAXA/Japchae prototype 근거 및 현행 A2/IR 불변식에 연결된 도입 후보/우선순위·진입/종료 gate를 문서화.
- [ ] **NN-D1:** P0 `relu` 한 개에 대해 name binding → Graph atom → reference CPU → decomposition/no-decomposition 동일 결과 vertical slice 및 음성 verifier 실증.
  - **NN-D1a (registration admission, partial implementation):** `PrimitiveResolver::with_extensions`의 유효 NAME·중복·semantic identity/lowering key·registry version 사전 검증과 음성 회귀 테스트를 추가하는 단계. 이는 등록 계약 검증만이며 `relu` Graph atom, CPU 실행, J Rank/빈 배열 및 분해 동등성은 미검증/미지원이다. PR-head CI·병합 뒤 main CI가 확인되기 전에는 PASS로 기록하지 않는다.
- [ ] **NN-D2:** `linear` + `flatten` + `softmax` 최소 MLP, `conv2d` + `avgpool2d` 최소 CNN으로 Shape/Rank/empty-frame 증거 수집.
- [ ] **NN-D3:** P1 attention/embedding/norm 연결로 작은 Transformer inference; 마스킹과 numeric stability/target route 차이 검증.
- [ ] **NN-D4:** P2 backward/optimizer/state/AD 관계는 명시적 effect·gradient 계약 확보 후 별도 구현; P3은 구체 workload와 외부 커널 수요가 확인된 뒤 판단.
- [ ] **NN-D5:** 각 완료 항목을 §A2/해당 M-stage gate 및 영어 mirror와 일치시키고, 정확한 main SHA/CI/미검증 범위를 기록.
- [ ] **NN-D6:** NN-24–NN-30을 기존 J/ExecutionBasis로 우선 실현할 수 있는지 확인하고, 새로운 Graph atom·execution basis는 별도 의미론 증거·reference oracle이 확보된 경우에만 추가한다. NN-31–NN-34는 실제 RNN/영상/CTC workload 수요 확인 뒤 구현 또는 명시적 보류를 결정한다.

**참조/경계:** [§4.6 단일 확장 목록](#nn-extension-checklist), [A2 extension/analysis checklist](#a2--extension-primitive-registry와-analysis-contract), [Graph vs Execution Basis](#a15--j-graph-ir--jaxa-array-operation-graph-ir), [JAXA 후기 설계 문서(A)](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md). 과거 문서의 "conv는 hidden weight 소유 blackbox"는 RustJ의 **explicit StateResource** 규약보다 우선하지 않는다. 이 절은 신규 구현과 검증을 주장하지 않는다.


### 4.7 과거 JAXA/Japchae 저장소 통합 기준

2026-09-30에 다음 네 저장소의 최신 내용을 다시 대조했다. **2026-10-02 재확인한 원격 HEAD와 확장 primitive별 최신 대조는 [§4.6](#extension-primitive-inventory)를 따른다.** 이 표의 substantive baseline은 당시 조사 이력이다.

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

#### 4.8.1 실행 전 입력 정보 — ``with (X`Y)`` 후보와 metadata 검사

**사용자 의도:** 실행 전 분석·최적화에 필요한 입력 정보를 어떻게 확보할지 검토한다. `with`/gerund 채택은 필수가 아니며 아직 표면 schema를 확정하지 않는다. 기존 noun metadata 추론, 외부 API/catalog, 파일·데이터셋 header/schema 또는 optional annotation에서 facts를 받아 같은 분석 경계로 전달할 수 있다. header 조회도 실제 I/O이므로 effect를 숨기지 않는다. graph topology는 shape 없이도 일부 분석 가능하고, dtype·rank·extent·차원 관계를 알수록 더 정확한 legality/resource 분석이 가능하다. 값에 의존하는 조건은 metadata로 증명하지 않는다. frontend 작업 우선순위와 optimizer 실행 보류는 유지한다.

**출처 확인(2026-10-04):** `jaxa-analyzer`의 pinned `7275d5ba` 아키텍처 문서 §7.7-2, entry-point 항목에 `X =: 2 3 source with fp16`, `Y =: 2 3 source with fp16`, ``run =: (#@:[ ([ optimizer (''"_)) graph) with (X`Y)`` 제안이 있다. 같은 절의 `with` 항목은 annotation/adjoint/optimizer 묶기를 제안하며, 뒤 항목은 dtype·hardware·tile 정보를 평평하게 섞는 문제를 명시한다. 이는 제안/검토 이력이며 실행 구현 완료의 근거가 아니다. `X/Y`는 여기서 학습 배열 전체가 아니라 **source에 계약을 붙인 entity**로 해석할 후보다. 표준 J의 tie는 verb의 atomic representation을 담는 noun을 만들며 noun operand에서는 concatenation을 수행한다. 따라서 noun으로 binding된 학습 배열에 backtick을 쓴다고 외부 입력 handle이 자동으로 생기지 않는다. [JAXA 원 제안](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md#L861), [J tie](https://www.jsoftware.com/help/dictionary/d610.htm).

**RustJ 설계 예시 — 아직 실행 가능한 extension 문법이 아니다:**

```j
X =: 2 3 source with fp16
Y =: 2 3 source with fp16
run =: graph with (X`Y)
```

원문의 표면 형태를 유지하되 `source`, `fp16`, `with`는 모두 ordinary binding이다. tokenizer/parser에 spelling별 규칙을 추가하지 않는다. `with`의 semantic adapter는 gerund의 원본 noun·순서·span·NameRef를 보존하고, 등록된 descriptor/annotation capability만 읽는다. 임의 verb를 호출해 metadata를 얻거나 외부 데이터를 로드하지 않는다. capability를 모르면 분석 경계를 보고하고 실제 J 문법을 invalid로 판정하지 않는다. 실제 source 호출의 I/O/state effect는 별도 contract로 남긴다.

`with`를 선택하는 경우의 후보 입력 descriptor는 schema version, input identity/slot, dtype/rank/shape facts 및 provenance를 담는다. dyadic input bundle에서는 두 entry의 좌·우 slot을 명시·검증하고 원문의 X/Y 순서를 보존한다. 단순 두 항목이라는 이유로 adjoint/optimizer 묶기를 입력으로 해석하지 않는다. 함수 이름의 observed target를 쓸 때에는 ordinary NAME late binding 및 별도 runtime binding/version guard가 필요하다. 실제 noun 입력은 parser에서 읽을 때의 snapshot을 유지하며, 이를 function처럼 지연시키지 않는다.

전달할 정보의 상세 분류·필요 수준·현재 구현 경계는 [§4.8.2 입력 정보 명세](#preexecution-input-information)를 따른다.

큰 데이터셋은 batch별 실제 noun을 입력에 연결한다. IR은 `ReadNoun`와 facts를 보유하고 원소 전체를 literal로 넣지 않는다. 같은 계약의 batch는 같은 분석 구조를 사용할 수 있지만 **실행 계획 재사용에는 별도의 함수·effect·runtime guard가 필요하다.** parameter 값은 step마다 바뀌어도 payload를 compile constant로 고정하지 않는다. contents/pointer/runtime noun version을 무조건 compile-cache key로 삼지 않는다. cache는 graph/contract/schema 및 필요한 semantic/target specialization을 기준으로 설계하고 실제 NAME witness를 별도로 확인한다. 값 의존 결과 크기는 unknown/guard 경계로 남긴다. 학습 pipeline/optimizer 실행 구현은 이번 범위가 아니다.

**이번 구현:** `StaticAnalysis::validate_noun_inputs()`가 사용된 noun 선언과 live `&Value`의 dtype·rank·shape를 검사한다. 배열 원소를 읽거나 clone/retain하지 않으며 supplied metadata만 검사한다. type→rank→extent 순으로 Domain/Rank/Length를 반환하고 missing NAME은 Value, duplicate/extra 입력은 Domain으로 거부한다. unknown facts는 새로운 제약을 만들지 않는다. catalog revision은 runtime witness가 아니며 이 검사는 함수 binding·effect·alias·device·값 범위 또는 전체 실행 안전성을 증명하지 않는다. 현재 runtime 실행 경로에 자동 삽입한 guard도 아니다. `with` 해석은 아직 미구현이다.

체크리스트(기존 frontend 우선순위 유지):

- [x] **WI0** 원 제안과 J tie 의미를 확인하고 입력·adjoint·numeric·physical 정보의 경계를 정본에 통합한다.
- [x] **WI1** 데이터 없는 noun 분석 결과에 live noun metadata 검사 API를 추가한다. 서로 다른 batch 수용, dtype/rank/extent·missing/duplicate/extra 거부, unknown/partial facts·empty array·함수 미지원 유지 및 payload pointer/refcount 불변을 Windows regression 4개로 검증한다.
- [ ] **WI2 (선택 후보)** 등록된 source/input descriptor와 typed bundle schema를 정의하고 gerund construction의 원본·순서·lookup timing/provenance를 유지하는 `with` semantic adapter를 구현한다. ordinary noun operand와 잘못된 descriptor를 별도로 검증한다.
- [ ] **WI3** 전달 문법과 독립적으로 input facts를 static catalog/ReadNoun에 연결하고 함수 late binding·redefinition·POS 변경·unknown capability의 C/frontend regression을 추가한다. annotation 없이도 가능한 추론은 유지한다.
- [ ] **WI4** 실제 input binding/lifetime 및 guard failure 경로를 연결한다. effect 이전 검증·batch 교체·부분 실패·계약 충돌을 확인한다. 외부 버퍼/mmap/device adapter는 별도 physical 작업이다.
- [ ] **WI5** 기호 차원·기호 동등성·캐시 전제 및 학습 state contracts를 후속 구현한다. optimizer/CUDA/AD 실행은 계속 보류한다.

비교 근거: [JAX abstract evaluation](https://docs.jax.dev/en/latest/601/jax-primitives.html)은 값 없이 shape/type을 분석하고, [PyTorch export](https://docs.pytorch.org/docs/stable/export)는 input/parameter와 dynamic shape constraints를 구분한다. 이들은 descriptor 기반 분석의 참고이며 J NAME timing을 대체하지 않는다.

<a id="preexecution-input-information"></a>

#### 4.8.2 실행 전 입력 정보 명세 (2026-10-04)

**필요성은 optimization별로 다르다.** full J 실행에 아래 모든 선언을 요구하지 않는다. topology 분석에는 known callable/graph만으로 충분한 부분이 있고, shape/type-dependent lowering에는 해당 facts 또는 runtime 조건이 필요하다. 각 입력에 대해 최소한 **어느 graph 입력인가 + noun이라는 품사 + known/unknown 구분 + facts의 근거/유효 scope**를 연결한다. 그 위에 dtype/rank/shape를 확보한 만큼 분석한다. schema는 `with` 문법과 독립적이며 noun metadata·외부 API/catalog·file header·선택 annotation에서 같은 facts를 얻을 수 있다.

| ID / 분류 | 알고 싶은 정보와 예시 | 필요한 분석·최적화 | 확보/검증 방법과 현재 경계 |
|---|---|---|---|
| IN0 입력 식별 | graph input ID, NAME/slot, schema revision, noun snapshot 시점 | graph edge 연결, 잘못된 입력 연결 방지 | catalog/호출 mapping; catalog version은 runtime NAME witness가 아님. 현재 used-input catalog와 missing/duplicate/extra 검사 존재 |
| IN1 원소 타입 | J logical dtype, numeric category; 외부의 fp16/fp32 등 정밀 dtype은 explicit conversion/encoding 계약과 연결 | type propagation, 합법 연산·kernel 후보, 저장 크기 | noun/header/API. 현재 TypeFact와 dtype 검사 존재; `Value::Float` CPU payload는 f64이며 fp16/fp32 NN 입력 지원을 뜻하지 않음 |
| IN2 rank·shape | rank=2, shape=[256,784]; scalar/empty 및 unknown 포함 | rank/cell split, agreement, 결과 shape·logical extent | shape가 있으면 rank/count 유도; 현재 concrete shape/unknown·rank 및 shape 검사 지원. J prefix agreement를 NumPy broadcasting으로 바꾸지 않음 |
| IN3 차원 관계·범위 | X=[B,K], Y=[B,N], W=[K,N]; X.B=Y.B, 0≤B≤1024 | graph 전역 shape 관계, dynamic-size 분석, 메모리 상한·guard | shared symbolic scope/constraint 및 실행 시 metadata 검사 필요. shape 미확정과 동일 기호는 다름. 0/1/empty를 임의 배제하지 않음. 현재 기호 차원/관계 guard 미구현 |
| IN4 논리 representation·중첩 schema | dense/axis-sparse/boxed; sparse axes/fill schema·stored count, boxed child dtype/shape가 known인지 | sparse/boxed legality, 결과 assembly, 비용/extent 분석 | J-visible facts와 storage encoding 구분. 일부 representation facts 존재하나 WI1 검사는 이 조건을 확인하지 않음. structural count를 sample에서 추정해 semantic proof로 쓰지 않음 |
| IN5 실행 중 변동과 작은 상수 | 데이터·학습 weight는 runtime 입력; axes/rank/window 등 일부 작은 constructor noun은 값이 필요할 수 있음 | constant folding, modifier construction, relevant specialization key | 값이 필요한 operand만 explicit constant/witness로 제공. 데이터셋·weight 내용을 compile literal/cache key로 넣지 않음; value-dependent constructor는 현재 경계 유지. [JAX static arguments](https://docs.jax.dev/en/latest/aot.html) |
| IN6 의미상 역할·effect | batch/data/label/parameter/gradient/state, read/write/accumulate, step dependency, 선택 AD target | 상태 갱신 순서, 학습 graph/AD, side-effect legality | 역할은 dtype나 대문자 NAME에서 추정하지 않음. 실제 function/effect contract와 StateResource에 연결; training/AD 통합 미구현. [PyTorch graph signatures](https://docs.pytorch.org/docs/main/user_guide/torch_compiler/export/api_reference.html) |
| IN7 값 성질 | KnownRange, finite, nonnegative, sorted, unique, valid indices 등 | bounds/check elimination, search specialization, 값 의존 결과 shape | proof/명시 계약/실행 시 검사. 표본 통계는 cost hint일 뿐 legality proof가 아님; full array 검사에는 O(N) 비용이 들 수 있음. 현재 WI1은 내용 미검사 |
| IN8 numeric policy | cast, accumulator precision, overflow/promotion, cct/fit, NaN/Inf, reassociation·determinism 허용 범위 | fusion/reduction 재배치·mixed precision의 합법성 | input dtype만으로 결정 불가; 기존 operation/semantic policy에서 확보. float16 선언만으로 J 수치/오류 의미를 바꾸지 않음 |
| IN9 수명·소유권·alias | 외부 owner/release, 다른 입력과 overlap, read-only/mutation, 호출 후 reuse/ownership donation 가능 여부 | buffer reuse/in-place, 안전한 외부 import, memory retention | adapter/런타임 borrow·ownership 증거 및 liveness 분석. logical noun identity와 physical BufferId 분리. WI1은 수명 보장/alias/donation을 검사하지 않음. [JAX donation](https://docs.jax.dev/en/latest/buffer_donation.html) |
| IN10 실제 representation·target | device, physical dtype/encoding, strides/offset/layout/alignment, transfer/sharding, target capability | concrete kernel/route/schedule 및 실제 byte/transfer 비용 | representation adapter + TargetProfile에 보관; semantic `with` 입력 계약에 섞지 않음. 관측 layout을 specialize하면 해당 guard 필요. [PyTorch tensor guards](https://docs.pytorch.org/docs/main/user_guide/torch_compiler/torch.compiler_dynamo_overview.html) |
| IN11 facts의 근거·유효성 | unknown/관측/추론/명시 조건/proof, scope/version, runtime guard·충돌 처리 | 최적화 전제 추적, stale specialization 방지, cache reuse | 모든 facts에 provenance 또는 witness 연결. 표본·관측을 영구 상수로 승격하지 않음. caller/locale/NAME witness와 payload snapshot을 구분. 현재 catalog revision은 분석 내 revision만 제공 |

IN0–IN5는 input schema/facts, IN6–IN8은 실제 연산/state 계약과의 연결, IN9–IN10은 runtime/physical 경계, IN11은 전체 facts에 적용되는 provenance다. 모두를 immutable FunctionEntity에 넣지 않는다. argument-dependent facts는 call/analysis에, 증명은 pass의 witness에, physical facts는 downstream representation에 둔다. axis 의미(batch/channel/feature 등)는 descriptor 해석·shape 관계에 유용한 optional label이며 표준 J rank/axis semantics를 대신하지 않는다. 실제 device는 입력 metadata가 될 수 있지만 tile/kernel 선택은 compiler가 도출하는 계획이다.

최적화별 요구 수준:

| 목적 | 주요 정보 | 모를 때의 처리 |
|---|---|---|
| topology·공통 입력·분기/합류 후보 발견 | graph/callable identity, input 연결, source provenance | known 구조만 분석; 미확정 callable은 경계로 유지 |
| 연산 fusion/재배치의 legality | 위 구조 + operation effect/error/numeric/alias 계약, 필요한 shape/type 관계 | topology hint만으로 실행 순서를 바꾸지 않음; proof/guard 없는 후보는 적용하지 않음 |
| logical memory/resource 분석 | type/rank/shape, sparse/boxed schema, use-def/liveness | 기호식/범위/unknown으로 보고. logical extent 합계를 peak allocation으로 표시하지 않음 |
| kernel 선택·SIMD/GPU schedule | 합법성 정보 + 실제 representation·target/capability | guarded route 또는 검증된 지원 경로만 사용. unknown을 임의 layout/device로 가정하지 않음 |
| buffer 재사용·in-place | representation·ownership/alias/lifetime + graph liveness | shared/external owner가 보이면 conservative 처리; 허용만으로 reuse를 보장하지 않음 |

학습 step의 **개념 명세 예시**(현행 dtype/기호 차원/AD 지원을 주장하지 않음):

```text
X: data,      float32[B,784]
Y: label,     float32[B,10]
W: parameter,float32[784,10]   // runtime value changes between steps
constraints: X.dim0 = Y.dim0, 0 <= B <= 1024
state: W read -> compute -> explicit update; optimizer state is separate
constants: only the required axis/window/rank operands
witness: verify metadata and any specialized NAME/semantic policies before effects
physical inputs: adapter-provided placement/layout/lifetime, separate from this schema
```

조건은 실제 program/domain이 허용해야 하며 `B=0` 결과와 오류도 기존 J 의미를 따른다. 사용자가 이미 batch별 step을 정의한 경우 dataset 전체 크기/내용 없이 그 step을 분석할 수 있다. compiler가 전체 데이터셋 reduction·batch statistics를 임의의 minibatch 계산으로 바꾸는 것은 다른 문제이며 별도 equivalence proof가 필요하다. 계약이 같아도 weight의 값은 바뀔 수 있다; 캐시에는 **특정 최적화가 실제로 사용하는 facts만** 넣는다. 값 의존 출력(예: 필터의 결과 길이)은 shape만으로 정확히 알 수 없으며 unknown/후속 shape 계산 경계를 유지한다.

정보 획득 순서는 기존 semantic contract/known literals → 준비된 noun/header/API metadata → 필요한 optional declarations → 합법성을 위해 필요한 proof/runtime guard다. 내용 스캔·별칭 증명·GPU 구현을 기본 입력 명세의 선행 조건으로 요구하지 않는다. specialization guard miss는 곧 J semantic error가 아니며 가능한 재분석/검증된 runtime 경로를 사용한다. 명시 계약 위반의 오류 정책과 단순 optimization precondition miss를 구분해야 한다. WI1 metadata API의 오류는 명시 검사의 결과이며 자동 compiler dispatch 정책이 아니다.

프레임워크 근거: [JAX ShapeDtypeStruct/AOT](https://docs.jax.dev/en/latest/aot.html)·[기호 shape constraints](https://docs.jax.dev/en/latest/export/shape_poly.html), [TensorFlow TensorSpec/input_signature](https://www.tensorflow.org/guide/function), [TVM Relax symbolic shapes](https://tvm.apache.org/docs/deep_dive/relax/learning.html), [PyTorch input/state signatures](https://docs.pytorch.org/docs/main/user_guide/torch_compiler/export/api_reference.html)·[tensor guards](https://docs.pytorch.org/docs/main/user_guide/torch_compiler/torch.compiler_dynamo_overview.html). 이 표의 IN0–IN11 분류와 적용 우선순위는 RustJ 설계 판단이다.

문서 체크리스트:

- [x] **WI0a** 전달 문법과 독립적인 IN0–IN11 inventory, optimization별 최소 정보 및 framework sources를 정리한다.
- [ ] **WI3a** 필요한 facts의 provenance/refinement와 conflicting/unknown 입력을 검증한다. 동등/범위 기호 차원은 WI5와 함께 처리한다.
- [ ] **WI4a** guard miss와 explicit contract violation, supported route 선택을 구분하고 effect 전에 검증 가능한 전제를 연결한다. scalar/empty/boxed/sparse·changing batch/weights 및 NAME 변경을 포함한다.

이번 변경은 설계 문서 정리다. WI1 이후 새 runtime/optimizer 구현 또는 새 test gate 완료를 주장하지 않는다.

##### 설계 영향과 출처 — 채택할 아이디어와 RustJ 판단의 구분

아래 표는 **설계에 영향을 준 개념의 출처**다. 각 프레임워크가 RustJ의 조건을 그대로 구현한다는 뜻이 아니며, 이 조건은 아직 구현되지 않은 외부 입력/실행 adapter의 명세다. J NAME snapshot/late binding, scalar/empty, sparse/boxed 및 observable error 의미는 RustJ/jsource 경계에서 유지한다.

| 영향을 준 프레임워크/연구 | 참고한 구체적인 방법 | 관련 inventory | RustJ에 적용하는 판단과 차이 |
|---|---|---|---|
| JAX | 값 없는 shape/dtype descriptor와 실제 값이 필요한 static argument | IN1–IN3, IN5 | 입력 배열과 compile-time 작은 상수를 분리. ndarray 원소를 만들어 graph 분석하지 않는다. [ShapeDtypeStruct](https://docs.jax.dev/en/latest/_autosummary/jax.ShapeDtypeStruct.html), [AOT/static args](https://docs.jax.dev/en/latest/aot.html) |
| JAX shape-polymorphic export | 같은 symbolic scope의 기호 차원·동등/범위 조건 | IN3, IN11 | shared B와 독립 unknown을 구별. 기호 scope/provenance를 추적하고 J의 empty dimension을 임의 배제하지 않는다. [기호 shape](https://docs.jax.dev/en/latest/export/shape_poly.html) |
| TensorFlow | TensorSpec/input_signature, 일부 차원의 wildcard, incompatible signature 검사 | IN1–IN3, IN11 | 고정·미확정 facts를 부분적으로 허용. 단순 wildcard와 입력 사이의 동등 관계는 구별하며 선언을 full J의 필수 조건으로 만들지 않는다. [tf.function](https://www.tensorflow.org/guide/function) |
| TVM Relax | 입력과 연산 사이의 shape/dtype·기호 관계 전파 | IN1–IN3, IN10 | logical 관계와 physical lowering을 분리하고 이후 symbol analysis의 근거로 사용. TVM의 array agreement를 J에 복사하지 않는다. [Relax](https://tvm.apache.org/docs/deep_dive/relax/learning.html) |
| PyTorch Dynamo/export | 관측 metadata에 대한 guards, input/parameter/state signature 구분 | IN0–IN3, IN6, IN10–IN11 | specialization에서 사용한 dtype/rank/size/layout 조건을 확인. 실제 J function NAME witness와 noun snapshot timing은 별도로 유지. [Dynamo guards](https://docs.pytorch.org/docs/main/user_guide/torch_compiler/torch.compiler_dynamo_overview.html), [export signature](https://docs.pytorch.org/docs/main/user_guide/torch_compiler/export/api_reference.html) |
| JAX buffer donation | 호출 후 필요 없는 입력 버퍼를 결과 저장소로 재사용하도록 허용; 호환 결과 필요 | IN9 | ownership transfer를 명시하고 이전 handle의 사용을 차단할 수 있을 때만 후보. donation 허용은 실제 reuse 보장이 아니다. [Donation](https://docs.jax.dev/en/latest/buffer_donation.html) |
| MLIR One-Shot Bufferize | use-def/alias·read-after-write 충돌 및 writable 여부에 따른 in-place/out-of-place 결정 | IN9–IN10 | live J snapshot/외부 alias를 덮어쓰지 않는다는 증거가 있어야 reuse. 불명확하면 별도 output/copy 또는 지원 route 경계. [Bufferization](https://mlir.llvm.org/docs/Bufferization/) |
| 역사 JAXA 연구 | source 계약과 entry graph를 gerund/with로 연결하는 표면 아이디어 | IN0–IN3, IN6 | optional 전달 후보만 계승; physical policy를 섞지 않고 등록된 capability와 ordinary binding을 사용. [JAXA 제안](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md#L873) |

##### 입력 metadata 조건 (M0–M8)

이 조건은 **metadata를 정당한 최적화 전제로 사용할 때** 적용한다. dtype/rank/shape만으로 buffer 안전성·함수 의미·내용 불변성까지 증명하지 않는다. 물리 접근 검사는 compiler semantic facts와 별도의 adapter 의무다.

| 조건 | 명세 | 실패/미확정 처리 및 현재 상태 |
|---|---|---|
| M0 입력 mapping | 사용되는 각 noun input의 ID/slot/이름을 명확히 연결; 중복·누락·다른 품사 입력을 구별 | WI1 API는 used noun의 missing/duplicate/extra를 검사. 함수/locale 재사용 증명은 별도 |
| M1 논리 일관성 | shape가 있으면 rank=shape.len; 각 extent는 유효한 비음수 크기; scalar는 rank 0·atom 1, empty는 0 extent·atom 0; count 계산은 overflow-safe | 현재 선언/Value 생성의 해당 검증을 사용. 표현 한계로 처리하며 negative/empty를 임의 shape로 보정하지 않음 |
| M2 dtype·encoding 연결 | 외부 dtype/byte order/encoding과 J logical dtype의 연결은 adapter가 확인; 변환은 explicit하고 numeric semantics를 보존 | fp32 bytes를 f64 slice로 재해석하지 않음. WI1은 Value의 logical dtype만 확인하며 외부 encoding/import 검사는 미구현 |
| M3 저장 범위 | 직접 접근 전에 dense byte capacity 또는 strided view의 offset·stride·접근 가능 범위를 overflow-safe하게 확인; alignment/device/backend 요구도 확인 | shape만으로 모든 strided 접근이 안전하다고 가정하지 않음. CPU affine foundation과 향후 external adapter 의무를 구분; WI1의 검사 범위 아님 |
| M4 unknown·symbolic | unknown에 임의 상수/양수/no-alias를 부여하지 않음. 같은 scope의 symbolic equality/bounds만 필요한 specialization 전제로 사용 | symbolic relation/guard는 WI5 후속. 미확정 조건은 conditional report 또는 지원되는 runtime shape 경계 |
| M5 근거·유효 기간 | 각 fact에 source/proof/관측 scope를 연결. file header·catalog 선언은 실제 버퍼의 존재/수명/최신성을 보장하지 않음 | 분석용 선언과 runtime witness 분리. actual Value/buffer generation 또는 동등한 소유권 lease에 조건을 연결; 통합 미구현 |
| M6 검사→사용 일관성 | 검사한 metadata·NAME·storage가 실제 실행 대상과 같아야 함. 외부 reshape/reallocation/mutation 경쟁은 snapshot·lease·synchronization 계약으로 통제 | 매 batch 새 입력의 relevant facts를 확인. 검사 후 변경 가능성이 남으면 해당 specialization/import를 사용하지 않음. WI1의 성공은 이후 호출까지의 보장이 아님 |
| M7 부분 facts·내용 조건 | 확보된 정보만 refinement. finite/sorted/range 같은 contents 조건은 별도 proof/검사; 표본 통계는 legality 근거가 아님 | 값 없는 schema 분석을 유지. boxed/sparse의 알려지지 않은 child/구조 facts는 unknown; 현재 WI1은 atoms/representation 조건 미검사 |
| M8 cache/guard 실패 | 최적화가 실제 사용하는 metadata·function/semantic 정책만 cache precondition으로 추적; batch/weight 값·pointer 변경만으로 불필요한 재컴파일하지 않음 | 명시 계약 오류와 specialization miss 구별. effect 후 자동 replay 금지; 재분석/지원 경로 선택은 실제 capability가 있을 때만. dispatcher 미구현 |

##### 소유권과 입력 저장소 조건 (O0–O8)

| 조건 | 명세 | 허용되는 동작과 한계 |
|---|---|---|
| O0 import 방식 | owned transfer, shared immutable 또는 borrowed 외부 버퍼를 명시적으로 구별. raw pointer/Arc refcount 하나만으로 외부 독점성을 추론하지 않음 | RustJ가 소유한 버퍼와 외부 aliases의 증거는 다름. unknown ownership은 read-only 후보에도 lifetime/동기화 확인 필요 |
| O1 수명 | 모든 read/write·transfer가 끝날 때까지 owner/lease 유지. 반환한 view가 입력을 참조하면 그 view의 수명까지 연장 | 이후 GPU/async 경로는 enqueue 시점이 아니라 실제 completion까지 유지해야 함; 현재 해당 실행 미구현 |
| O2 외부 mutation | J noun snapshot으로 노출한 데이터는 외부 변경으로 뒤에서 바뀌지 않아야 함 | immutable lease, synchronized snapshot copy 또는 explicit state/effect 경로 사용. wrapper가 read-only라는 이유만으로 producer의 쓰기가 차단되었다고 보지 않음 |
| O3 alias/overlap | 입력 간 views, 외부 alias, 살아 있는 global/boxed noun snapshots 및 graph 내부 소비자를 고려 | alias 부재가 증명되지 않으면 in-place/reuse를 허용하지 않거나 검증된 overlap-safe 구현을 사용. user의 no-alias 주장만으로 safe Rust reference를 만들지 않음 |
| O4 쓰기 권한 | 실제 backing이 writable이고 semantic operation의 write contract가 있어야 함 | readonly/mmap/protected backing을 덮어쓰지 않음; allocator 선택과 observable state write를 혼동하지 않음 |
| O5 donation/reuse | 명시 transfer 권한 + live old-value reads 없음 + alias/ownership 증거 + 호환 output representation + J 오류/효과 순서 보존이 필요 | transfer가 성립하면 이전 input handle/alias의 사용이 차단되어야 함. shared noun을 단지 refcount가 낮아 보인다는 이유로 consume하지 않음. output/copy가 여전히 필요할 수 있음 |
| O6 release 책임 | ownership 이동 또는 borrow의 시작/끝과 release 주체를 명시; transferred owner reference는 정확히 한 번 반환/해제 | validation 실패·부분 import·호출 실패·최종 output 종료에서 누수/double release 방지. metadata만 존재하는 선언은 release할 실제 buffer를 갖지 않음 |
| O7 zero-copy 전제 | lifetime·mutation·alias·alignment·encoding·layout/backend 접근 조건이 충족될 때만 직접 import | 조건 미충족 시 지원되는 값 보존 copy/conversion을 명시적으로 선택하거나 import 경계 보고. copy 중 외부 writer와의 동기화도 필요; 항상 zero-copy라고 약속하지 않음 |
| O8 실패·소비 시점 | 실제 호출/효과/ownership transfer 이전 가능한 검증을 완료; transfer commit 시점과 실패 뒤 handle 유효성을 adapter contract에 기록 | 시작 전 실패와 이미 소비/효과가 있는 실패를 구분. generic rollback·재실행을 약속하지 않으며 J transactional assignment 및 live snapshot을 보존 |

M/O 조건은 RustJ의 **채택할 계약과 후속 구현 요구**다. 현재 `StaticAnalysis::validate_noun_inputs()`는 borrowed `&Value`의 dtype/rank/shape와 입력 mapping만 검사한다. 외부 import·lease·encoding/stride capacity·alias·donation·자동 dispatch는 구현 완료가 아니다. 소유권 조건은 physical/runtime 경계에 남고 semantic `with`가 unsafe pointer 접근 권한을 부여하지 않는다.

- [x] **WI0b** 항목별 framework 영향·출처 및 M0–M8/O0–O8 조건을 정본/영어 mirror에 기록한다.
- [ ] **WI4b** adapter/guard 연결 시 stale metadata·외부 mutation·overlap·readonly backing·잘못된 capacity·donation 후 사용·실패 release·view lifetime 사례를 검증한다. async/device completion은 해당 backend를 구현할 때만 추가한다.

##### 정적 분석 우선: 실제 배열 없이 계약에서 분석한다

**사용자 요구:** 실행 전 최적화의 기반은 정적 분석이다. M/O 조건을 모두 actual buffer의 runtime 검사로만 얻는 구조로 만들지 않는다. 분석 입력은 실제 noun이 아니라 noun의 schema/facts와 parser가 보존한 계산 구조일 수 있다. 외부 API·header가 metadata를 제공하더라도 그 조회 자체의 I/O와 **이후 데이터 없는 정적 graph 분석**은 구별한다. source verb를 호출하거나 dummy/training array를 생성하여 shape를 알아내는 것을 기본 전제로 삼지 않는다. `with`는 이 분석에 필요한 정보를 전달할 수 있는 optional surface일 뿐이다.

```text
J frontend에서 보존한 graph + input schema + operation/ownership contracts
    -> static fact propagation / constraints / use-def / liveness
    -> 분석 보고와 최적화 후보의 legality 전제
    -> 정적으로 증명되지 않은 필요한 조건만 residual obligations로 보존
    -> 실행 때 actual inputs 연결 + 필요한 guard/lease 확인
    -> 지원되는 실행 경로
```

위 화살표는 설계 단계 구분이며 optimizer 실행·모든 조건 solver·dispatcher의 구현 완료가 아니다. kernel 실행 없이 만드는 정적 보고에는 graph 구조, known/unknown facts, 결과 extent, 자원식/상한, live values와 미해결 조건을 담는다. compiler는 facts를 unknown으로 남겨도 부분 분석할 수 있다. unknown과 unreachable, compile-route boundary와 invalid J를 혼동하지 않는다.

| 정보/조건 | 실행 전에 분석할 수 있는 부분 | 실제 입력이 연결될 때 남을 수 있는 조건 |
|---|---|---|
| IN0 / M0 | input slot/POS 및 graph edges, used-input inventory를 syntax/schema에서 결정 | 실제 공급 mapping, ordinary NAME의 current binding/POS/version witness |
| IN1–IN3 / M1,M4 | declared dtype/rank/shape, small constants와 연산 shape rules로 결과 facts/차원 관계 추론; known 제약은 정적으로 검증 | 공급 metadata가 declared facts 및 미증명 equality/bounds와 일치하는지 확인 |
| IN4 / M7 | 알려진 boxed child/sparse schema로 applicability/assembly 분석 | actual child schema·stored structure 등 선언에서 증명하지 못한 facts. 값 의존 shape는 unknown 유지 |
| IN5–IN8 | explicit configuration constants, semantic numeric/effect/state contracts로 legality와 dependency 분석 | 실제 함수 identity/정책, 별도 contents 조건 중 해당 route가 요구하는 것만 확인 |
| IN9 / O1–O5 | closed graph의 use-def/liveness로 old-value read와 내부 alias 충돌 분석; 명시 ownership 계약 및 managed Rust borrow/lifetime 정보 활용 | 외부 owner의 lease, 외부 alias/mutation·writability·exclusive transfer를 보장하는 adapter 증거. 선언만으로 외부 독점성 증명하지 않음 |
| IN10 / M2,M3,O7 | known target/representation 계약으로 후보·허용 encoding 및 조건을 도출 | actual backing capacity/strides/device/alignment와 lifetime의 적합성. physical 사실도 정적으로 주어졌으면 이미 증명된 부분은 반복 검사하지 않음 |
| IN11 / M5,M6,M8,O6,O8 | 각 fact의 provenance/scope와 경계별 proof obligations, cache preconditions, ownership transfer/release 흐름 정리 | 미해결 전제의 witness 유효성 및 검사→사용 안정성. 효과 후 실패는 자동 replay로 복구하지 않음 |

소유권의 정적 분석은 **내부 graph에서 더 이상 읽지 않는다는 사실**과 **외부에서 독점적으로 소유한다는 계약**을 구별한다. 전자는 graph 분석으로 증명할 수 있지만 후자는 외부 adapter의 보장 또는 managed lifetime/ownership 증거가 있어야 한다. runtime alias 체크를 언제나 값싼 일반 해법으로 가정하지 않는다. 보장이 없으면 out-of-place/지원 route 경계를 유지하며 correctness를 희생하지 않는다. 명시 contract에서 가능한 compile-time 증명과 residual guard가 함께 존재할 수 있다.

shape가 concrete하면 일부 logical extent를 정확히 계산하고, symbolic이면 식 또는 proven upper bound를 보고하며, 관계가 unknown이면 그 부분의 수치를 확정하지 않는다. symbolic 분석 자체도 정적 분석이다. concrete layout/schedule이 없으면 실제 peak allocation·성능 수치를 단정하지 않는다. 전체 학습 데이터셋의 원소 수나 내용이 graph 크기에 비례하여 compiler 안에 저장되어야 하는 것은 아니다.

**현재 경계:** `StaticAnalyzer::declare_noun()`/`analyze()`는 실제 arrays 없이 supported syntax와 concrete/unknown facts로 보고를 만든다. WI1 `validate_noun_inputs()`는 후속 입력 적합성 검사의 최소 seam이다. arbitrary per-dimension symbolic facts/constraint solver, ownership obligations의 전체 정적 증명, external leases, optimizer 변환 및 dispatch는 아직 미구현이다. 이러한 후속 작업은 frontend semantic 보존을 전제로 하며 actual data 실행을 정적 분석의 필수 조건으로 추가하지 않는다.

- [x] **WI0c** 정적 facts/proof와 residual runtime obligation을 분류하고 M/O의 정적 분석 역할을 명시한다.
- [ ] **WI3b** metadata-only 분석에서 shape/resource/use-def 결과와 미해결 조건을 보고한다. regression은 배열 미할당·kernel 미실행 및 unknown 보존, compile-time 제약 충돌/부분 facts refinement를 검증한다.
- [ ] **WI4c** 정적으로 증명한 조건과 residual guard/adapter 보장을 실행 경계에 연결하되 runtime-only 분석 구조로 바꾸지 않는다. 소유권/lifetime proof 범위와 미확정 external alias를 명시한다.

##### 입력 정보 문제의 프레임워크 비교 (2026-10-04)

| 프레임워크 | 실행 전 입력 정보를 얻는 방법 | shape 변화/재사용 처리 | RustJ에 참고할 부분과 한계 |
|---|---|---|---|
| JAX | `ShapeDtypeStruct`로 실제 원소 없이 trace/lower/compile 가능. static argument는 실제 값 필요 | AOT artifact는 signature에 specialize되어 불일치 입력을 거부. shape polymorphism은 별도 export 경로 | 입력 metadata와 실제 noun을 분리. 값 상수 specialization과 배열 입력을 구별. [AOT](https://docs.jax.dev/en/latest/aot.html), [shape polymorphism](https://docs.jax.dev/en/latest/export/shape_poly.html) |
| PyTorch | `torch.compile`은 입력 metadata와 guards를 사용. export는 실제 원소가 없는 FakeTensor/Proxy로 연산을 기록 | compile의 guard가 실패하면 재capture/recompile 가능; dynamic shape는 기호 크기와 제약을 사용. export는 별도 명시 제약 | 관측 facts와 검증 조건을 분리. default static→dynamic generalization을 선택적으로 참고하되 J NAME witness를 유지. [export model](https://docs.pytorch.org/docs/main/user_guide/torch_compiler/export/programming_model.html), [dynamic shapes](https://docs.pytorch.org/docs/main/user_guide/torch_compiler/torch.compiler_dynamic_shapes.html), [guards/recompile](https://github.com/pytorch/pytorch/blob/main/docs/source/user_guide/torch_compiler/torch.compiler_dynamo_overview.md) |
| TensorFlow | `TensorSpec`/`input_signature`; `get_concrete_function(TensorSpec(...))`로 tensor graph 실행과 tracing 분리 가능 | `None` 차원은 wildcard로 trace 재사용. 고정 signature와 맞지 않는 입력은 거부 | 일부 크기만 미확정으로 둘 수 있음. wildcard 자체는 서로 다른 입력의 차원 동등성을 표현하지 않음. [tf.function](https://www.tensorflow.org/guide/function) |
| TVM Relax | graph input에 shape/dtype type를 선언하고 symbolic dimension `n`을 연산·함수 사이에 연결 | 기호 관계를 IR에서 추론; 모르는 shape는 runtime shape 표현으로 남길 수 있음 | 단순 unknown과 `X.batch = Y.batch` 관계를 구별하는 후속 abstract domain. [Relax abstraction](https://tvm.apache.org/docs/deep_dive/relax/learning.html), [shape API](https://tvm.apache.org/docs/reference/api/python/relax/relax.html) |

tracing은 frontend/host 코드를 실행할 수 있으므로 “사용자 코드를 전혀 실행하지 않는다”와 같지 않다. descriptor 방식도 데이터 의존 결과 크기·조건·오류를 자동으로 증명하지 않는다. RustJ는 frontend에서 보존한 graph와 known facts를 분석하고, unknown facts 및 J effect/name timing을 유지한다. shape/type 정보가 없더라도 topology 수준 기회를 찾을 수 있으며, 확보한 정보에 따라 legality·logical resource 분석을 정밀하게 한다. 실제 optimization pass는 계속 보류한다.

**WI1 gate:** native Windows default/portable 각각 **396 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 세 runtime 경로 **4,942 cases / 4,938 passed / 기존 runtime 경계 4 / failed 0**, stage **10,163 checks / failed 0**, words **6,618 cases / failed 0**. capture graph 경계 164건 및 static 경계 2건은 별도다. report 10개의 실제 binary/source hash를 다시 확인했다. `with` 실행·기호 차원·학습/AD·CUDA·optimizer 실행 완료를 뜻하지 않는다. full upstream/private C trace 및 ignored definition acceptance는 미검증이며 Linux/GitHub CI는 실행하지 않았다. 이후 ordinary-reference 단계에서 NAME 제한을 해소했으며 implicit locative와 전체 operator scope는 후속 경계이며 직선 호출은 아래 operator-call 단계에서 지원한다. `with` adapter는 필수 선행 조건으로 만들지 않는다.

<a id="array-compiler-static-analysis"></a>

### 4.8.3 배열 컴파일러의 정적 분석 대상과 프레임워크 비교 (2026-10-04)

**분석 대상은 입력 배열뿐 아니라 프로그램의 중간 값·연산·사용 관계·제어 흐름·저장소다.** §4.8.2의 IN0–IN11은 분석의 초기 정보이며 아래 SA0–SA8은 그 정보와 보존한 graph/contract에서 도출할 질문이다. 정적 분석은 실행 전 facts·제약·증명·미해결 조건을 산출한다. fusion, DCE, tiling, buffer 배치는 그 결과를 사용하는 변환이며 분석과 구별한다. XLA도 analysis와 HLO 변환 pass를 구분한다. [XLA analysis passes](https://openxla.org/xla/hlo_passes)

| ID / 분석 대상 | 정적으로 확인할 질문과 산출물 | 최적화에 쓰일 근거 / 미확정 경계 |
|---|---|---|
| SA0 — type/rank/shape 관계 | 각 중간 값의 dtype, rank, extent, 차원 동등성·범위, empty 가능성 | 연산 적합성·크기식·specialization의 전제. 기호 크기도 정적 정보지만 제약을 풀지 못하면 unknown을 유지. [JAX symbolic shapes](https://docs.jax.dev/en/latest/export/shape_poly.html) |
| SA1 — 상수·값의 추상 성질 | 작은 compile-time operand와 그 의존 closure; 증명된 값 범위·조건 | 상수 계산·분기 단순화의 전제. 큰 학습 배열의 내용을 읽는 분석으로 바꾸지 않는다. sorted/unique/finite는 선언만으로 증명되지 않음. [TVM compile-time analysis](https://tvm.apache.org/docs/reference/api/python/relax/analysis.html#tvm.relax.analysis.computable_at_compile_time) |
| SA2 — use-def / fan-out / 결과 사용 | 값의 생산자·사용자, 공통 입력, 미사용 결과, 마지막 사용 | 공유·fusion·제거 후보와 중간 배열 보존 필요성. 미사용 결과도 effect/error가 있으면 제거를 허용하지 않음. [XLA dataflow](https://openxla.org/xla/hlo_passes) |
| SA3 — 반복·인덱스·접근 의존성 | iteration domain, 입력/출력 index map, parallel/reduction 축, read/write 충돌 | tiling·vectorization·병렬화의 legality. 간접 gather/scatter와 비affine 접근은 추가 정보가 필요. [MLIR Linalg](https://mlir.llvm.org/docs/Dialects/Linalg/), [Affine](https://mlir.llvm.org/docs/Dialects/Affine/) |
| SA4 — 제어 흐름·호출·효과·오류 | branch/region/call 의존성, 상태 read/write, 실행 순서와 speculation 안전성 | 이동·제거·병렬 실행의 전제. effect-free와 안전한 선행 실행은 별개이며 J 오류를 UB로 바꾸지 않는다. [MLIR effects/speculation](https://mlir.llvm.org/docs/Rationale/SideEffectsAndSpeculation/) |
| SA5 — alias / mutation / ownership | 같은 저장소를 가리키는 값, overwrite 후 old-value read, escape와 외부 소유권 조건 | in-place/reuse/copy 결정의 전제. 내부 no-later-read는 외부 exclusive ownership 증명이 아님. [MLIR Bufferization](https://mlir.llvm.org/docs/Bufferization/) |
| SA6 — 수명·materialization·메모리 | live range, 보관해야 할 중간 값, 논리 크기식, 선택한 순서에서의 live bytes | materialization 제거·재계산·저장소 재사용의 근거. graph 순서의 추정과 schedule/layout 확정 후 물리 peak를 구별. [XLA scheduling/buffers](https://openxla.org/xla/hlo_to_thunks) |
| SA7 — 표현·layout·target 적합성 | 논리 reshape/index 관계, 지원 dtype/representation 및 접근 모델; 이후 stride/device/target 제약 | view/copy와 kernel route의 전제. 논리 reshape 증명만으로 외부 버퍼의 zero-copy를 보장하지 않음. [TVM reshape analysis](https://tvm.apache.org/docs/reference/api/python/relax/analysis.html#tvm.relax.analysis.has_reshape_pattern) |
| SA8 — 자원·코스트·IR 일관성 | 연산량·전송/저장 크기식, 정의/사용·type·region 불변식 검증 | 후보 비교와 잘못된 IR 검출. 비용식은 실측 시간 또는 정확한 물리 메모리가 아님. [XLA cost/verifier](https://openxla.org/xla/hlo_passes) |

#### 같은 질문을 각 프레임워크가 다루는 방식

서로 다른 계층을 비교한다. JAX/PyTorch/TensorFlow는 graph 확보와 입력 제약을 포함하는 frontend이고 XLA는 backend, MLIR은 IR 기반시설이다. 아래는 우열·전체 기능표가 아니라 확인한 분석 방법과 RustJ가 참고할 지점이다. tracing으로 graph를 얻은 뒤의 정적 분석과, RustJ가 소스에서 보존한 graph를 분석하는 방법도 구별한다.

| 프레임워크 / 계층 | 확인한 분석 표현·방법 | 한계 / RustJ에 적용할 부분 |
|---|---|---|
| JAX / frontend | symbolic dimension 식·제약으로 shape 관계를 다룸 | 모든 기호 비교를 판정하지 못함. 관계와 미해결 조건을 보존하는 방향을 참고. [Shape polymorphism](https://docs.jax.dev/en/latest/export/shape_poly.html) |
| OpenXLA / backend | HLO value/use, must-alias, cost, verifier; schedule 이후 buffer slice 배치 | 수명은 순서에 의존. 의미 graph·분석·physical 결정을 분리. [Analyses](https://openxla.org/xla/hlo_passes), [buffer assignment](https://openxla.org/xla/hlo_to_thunks) |
| PyTorch / capture frontend | symbolic sizes와 ShapeEnv/guards로 관측 graph의 shape 전제를 관리 | tracing specialization의 guard와 데이터 의존 제어를 구별. RustJ는 J NAME witness와 source semantics를 별도로 유지. [Dynamic shapes](https://docs.pytorch.org/docs/main/user_guide/torch_compiler/torch.compiler_dynamic_shapes.html) |
| TensorFlow / graph frontend | TensorSpec/input_signature와 graph 함수로 dtype/shape 전제를 표현 | wildcard 차원과 입력 간 동등성은 별개. 입력 계약과 graph 분석을 연결하되 tracing을 필수로 하지 않음. [tf.function](https://www.tensorflow.org/guide/function) |
| TVM Relax / graph + kernel IR | use-def, compile-time 의존성, reshape index 증명, memory-planning 전후 할당량 추정 API | estimate_memory_usage는 제어 흐름·함수 간 호출을 고려하지 않는 할당량 합계로 과대 추정 가능. 정확한 peak로 인용하지 않음. [Analysis API](https://tvm.apache.org/docs/reference/api/python/relax/analysis.html) |
| MLIR / structured IR | Linalg index map/iterator, Affine dependence, effect/speculation interface, bufferization | affine 모델 밖의 접근과 외부 alias에 별도 근거 필요. J rank/cell/assembly 의미를 유지한 뒤 이러한 관계를 도출. [Linalg](https://mlir.llvm.org/docs/Dialects/Linalg/), [Affine](https://mlir.llvm.org/docs/Dialects/Affine/), [effects](https://mlir.llvm.org/docs/Rationale/SideEffectsAndSpeculation/), [bufferization](https://mlir.llvm.org/docs/Bufferization/) |
| Futhark / 배열 언어·컴파일러 | size가 있는 배열 type; scan/scatter fusion 사례에서 producer/consumer와 control dependency를 함께 다룸 | regular-array 제한은 RustJ 언어 제한으로 채택하지 않음. 사용 관계는 분석 가능해도 filter 결과 크기는 데이터에 의존. [Language reference](https://futhark.readthedocs.io/en/latest/language-reference.html), [scan-scatter 사례](https://futhark-lang.org/blog/2026-03-24-scan-scatter-fusion.html) |

**J syntax에서 얻는 정보의 범위:** `(loss_adjoint [ (loss emit))`의 fork는 같은 인자로부터 두 branch를 만들고 결과 선택/결합 관계를 드러낸다. 이는 SA2/SA4의 분석 입력이며 `adjoint`라는 이름 자체가 parallel 힌트는 아니다. fork만으로 무조건 병렬 실행을 허용하지 않는다. 각 branch의 효과·오류·name lookup·공유 상태 의존성을 확인해야 한다. modifier/train 구조는 SA0/SA3에 rank/cell/반복 관계를 제공할 수 있으나, 구체 인덱스 map과 uniform assembly는 별도 증명한다. 이 해석은 RustJ의 설계 결정이며 위 프레임워크의 J 지원을 주장하지 않는다.

**현재 구현과 후속 경계:** `static_analysis.rs`의 schema 기반 분석, `j_graph_ir.rs`의 보존 graph와 concrete/unknown GraphFacts, `j_graph_memory.rs`의 use/liveness·논리 extent·명시 가정하 graph-order 메모리 추정, `j_graph_resource.rs`의 자원식이 초기 기반이다. canonical Logical IR의 checks/effects/witness/verifier는 후속 분석 계약의 기반이다. 임의 축별 symbolic solver, 일반 loop/index dependence, 완전한 alias/escape/외부 ownership 증명, target별 schedule/물리 peak 분석은 완료되지 않았다. SA 표는 구현 완료 목록이 아니다. optimizer 실행·CUDA는 계속 보류하고 tokenizer/enqueuer/parser의 J 호환성과 graph 보존 작업을 우선한다.

후속 report는 각 fact에 `source node/span + assumptions + proof scope + known/symbolic/unknown + residual obligation`을 연결하고, 메모리는 `logical extent / assumed-order live bytes / physical allocation estimate`를 구분한다. 함수 identity·J prefix agreement·empty/fill/boxed/sparse·numeric policy·관측 가능한 오류/효과를 증명 없이 단순 tensor 규칙으로 치환하지 않는다.

- [x] **WI0d** 배열 컴파일러의 분석 대상 SA0–SA8, 프레임워크별 방법/한계 및 현재 구현 경계를 출처와 함께 정리한다.
- [ ] **WI3c** frontend graph/facts report에 SA 분류·증명 범위·미해결 조건을 연결하고 현재 지원 부분부터 regression을 추가한다. 일반 solver나 physical scheduler를 frontend 완성의 선행 조건으로 만들지 않는다.

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

**용어 주의:** `UnsupportedImplementation`은 이 문서의 **architecture-level 분류명**이다. 현재 Rust 코드의 concrete error variant는 `Error::Unsupported(String)`이고 `Error::kind()`는 `"unsupported"`를 반환한다. 새 enum variant가 이미 존재한다고 가정하지 않는다. 문서·테스트에서 concrete API를 말할 때는 `Error::Unsupported`/`"unsupported"`를 사용하고, J semantic error와 implementation-coverage miss를 구분하는 개념 설명에서만 `UnsupportedImplementation`을 사용한다.

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

설계 목표: jsource 내부의 `RMAX` 같은 정수 표현과 semantic `Infinite`를 분리한다. 현재 구현은 위의 rank 63 호환 표현을 유지하며, 명시적 `Infinite` IR 표현은 장기 설계 목표로 남겨 둔다.

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
5. `logical_ir::Plan::verify`가 must-preserve fact의 정합성을 검증한다.
6. use-def 기반 `OptimizationFacts` pass를 추가하여 uniform-result, mean/fusion, CellApply fusion, materialization-elision candidate를 만든다.
7. `LoweringRegistry`에는 IRS/FUSEDOK에 대응하는 capability만 등록하고 semantic fact와 섞지 않는다.
8. 실제 stride/alignment/in-place/buffer reuse는 Logical IR 구현보다 뒤의 representation/physical 단계에서만 추가한다.


<a id="mean-proof-example"></a>

##### 4.11.4.12 Canonical E2E 표본 — `(+/ % #) y`

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

현재 코드 대조 기준: **2026-10-02, `98ae387`**. 다음은 재실행한 테스트 결과가 아니라 source/API 검토 결과다.

| 경계 | 현재 상태 | 아직 검증/구현할 것 |
|---|---|---|
| FunctionEntity → J Graph | 이 표본의 shared Fork와 applied stage/branch 구조를 보존한다 | 전체 frontend의 mixed phrase/POS/construction 의미는 M2 gate를 따른다 |
| J Graph → canonical Logical IR | `analysis::lower_graph()`가 `logical_ir::Plan`을 직접 생성한다. `CompilationAnalysis.logical`과 `Engine::analyze/analyze_a3`가 같은 canonical plan을 사용한다 | canonical container cutover는 M1 완료. 새로운 lowering은 transition IR을 만들지 않는다 |
| Reduce / CellApply payload | A3의 `ExecutionBasisPayload`, `CallOp.execution_basis`와 explicit rank-plan seam은 존재한다 | plain `%`의 innate rank와 일반 implicit CellApply/assembly를 완성해야 한다. `Callable.reduce/rank` migration field도 아직 남는다 |
| verifier / reference execution | `Plan::verify()`와 `logical_executor::execute_closed()`가 존재한다 | closed-plan reference 실행을 named/environment 전체 지원 또는 Physical Executor로 오인하지 않는다 |
| route / physical execution | `LoweringRegistry`와 contiguous route-partition prototype이 존재한다 | 선택된 native Schedule/Physical Plan/CPU Physical Executor는 M4 이후의 미완료 경계다 |

###### 같은 표본의 stage-by-stage compiler trace

이 표본은 앞으로 문서와 구현에서 **한 source를 처음부터 끝까지 같은 provenance로 추적하는 canonical trace**로 사용한다. 현재 구현된 경계와 planned 경계를 섞지 않는다.

~~~text
J source
  (+/ % #) y
      │
      ▼
[현재 구현] parser / Semantic Construction
  Fork
    f = +/        // Insert(+) derived Verb
    g = %
    h = #
  source span / operand identity / observable fork order 보존
      │
      ▼
[현재 구현] J Graph IR
  input y
      ├─ h branch: Apply Tally(y) ──────────────┐
      └─ f branch: Apply Reduce(Add, y) ────────┤
                                                ▼
                                      Apply Divide(f, h)
  + Fork region/provenance
  + branch/join/use/liveness facts
  + Graph Basis / GraphHint
      │
      ├─→ [현재 분석] rewrite/fusion/resource/work-depth 후보·side analysis
      │       candidate는 source graph를 대체하거나 실행을 확정하지 않음
      │
      ▼
[현재 구현] Execution Semantic Lowering → canonical A3
  observable order: h → f → g
  Tally(y)
  Reduce(Add, y)
  Divide(left=reduce, right=tally)
    + valence/rank/cell/frame facts
    + prefix-agreement / repetition constraint or witness
    + SemanticCheck only when a required constraint remains unresolved
    + effect/error/speculation/order metadata
      │
      ▼
[현재 구현] A3 verifier / reference execution capability
  Plan::verify()
  logical_executor::execute_closed() on its supported closed subset
      │
      ▼
[현재 프로토타입] route analysis
  LoweringRegistry::route_operation / partition_plan
  native realization이 없는 op은 RuntimeSemanticFallback class일 수 있음
  contiguous class grouping은 아직 final mixed-route plan이 아님
      │
      ───────────── 현재 compiler-native physical 실행의 stop line ─────────────
      │
      ▼
[planned M4] deterministic CPU Schedule / PhysicalPlan
  BindInput(y)
  Check(...)             // A3에 unresolved SemanticCheck가 있을 때만
  Kernel Tally
  Kernel Reduce(Add)
  View/iteration mapping // right scalar를 J cell semantics에 따라 반복, 필요 시 metadata-only
  Kernel Divide
  Return(result)
      │
      ▼
[planned M4 validation]
  result = 1.5 2.5 3.5
  + jsource와 value/error/order differential
  + allocation/view/reuse invariants
~~~

중요한 점은 **Mean이라는 fused operation이 correctness baseline이 아니라는 것**이다. 이 표본의 첫 compiler-native 성공 조건은 위 generic `Tally + Reduce + CellApply/Divide` 경로다.

그 이후에만 다음과 같은 optimization candidate를 별도로 고려할 수 있다.

~~~text
source generic graph
  Tally + Reduce(Add) + CellApply(Divide)
          │
          └─→ possible Mean-style fused/composite candidate
                 equivalence / rank-cell assembly
                 numeric + error/effect order
                 fanout/retention
                 resource/work-depth
                 target capability
                 profitability
                 모두 필요한 proof 이후에만 select
~~~

현재 `j_graph_fusion`의 등록 규칙이나 `fusion_planning`이 이 Mean specialization 전체를 이미 선택·lowering한다고 해석하지 않는다. 이 부분은 **future optimization example**이며, 먼저 generic graph의 semantic/physical E2E가 검증되어야 한다.

##### 이 trace의 provenance 연결 규칙

- parser `FunctionEntity`의 Fork/operand/source span을 J Graph region과 연결한다.
- 각 J Graph `ValueId`는 A3 operation의 `j_origin`으로 추적 가능해야 한다.
- candidate는 source graph/version과 rule/witness provenance를 가진다.
- RoutePartition과 PhysicalPlan은 canonical A3를 destructive하게 덮어쓰지 않고 source logical operation/provenance를 참조한다.
- Physical `Kernel`/`Check` 실패를 보고할 때 가능한 한 A3 op → J Graph origin → source span으로 역추적한다.
- planned M4 trace가 실제 구현되기 전에는 이 그림의 PhysicalPlan 구간을 구현 완료 증거로 사용하지 않는다.

shape `[3]` 추론만으로 `%`의 prefix agreement와 residual-frame repetition을 구현했다고 판정하지 않는다. 이 표본의 semantic contract 검증은 다음으로 좁힌다.

- [ ] primitive/derived callable의 valence별 innate `RankSpec`과 explicit `"`가 공통 CellApplicationPlanner를 사용한다.
- [ ] matrix 표본의 `Divide`가 위 `CellApply2` frame/cell/repetition을 보존하는 golden을 추가한다.
- [ ] verifier가 input/result facts, frame/cell split, prefix agreement와 repetition relation을 검사한다.
- [ ] `Reduce + Tally + CellApply(Divide)`를 M4의 최소 physical 경로로 실행하고 jsource와 differential 검증한다.
- [ ] 그 generic 경로의 의미 동등성이 검증된 뒤에만 fused Mean 후보를 추가한다. semantic Fork identity는 유지한다.

`tests/analysis.rs::canonical_mean_fork_lowers_to_reduce_tally_divide_in_jsource_order`는 `analyze_a3 + verify`와 h → f → g ordering을 확인하는 smoke test다. 실제 실행이나 compiler-native E2E 완료의 증거는 아니다. M4의 실행 체크리스트와 이 semantic 표본의 조건은 별개로 추적한다.

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

##### 4.16.5.1 Compilation 시작 시 active target locale을 확정한다

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

##### 4.16.5.2 AOT와 JIT는 동일한 target selection contract를 사용한다

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

#### 4.16.6 Target locale chain: J locale 방식을 compiler lookup에 재사용한다

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
- 현재 vocabulary 기준: [NuVoc](https://code.jsoftware.com/wiki/NuVoc), 2026-10-05 확인, [관찰 revision 60409](https://code.jsoftware.com/mediawiki/index.php?title=NuVoc&oldid=60409). 과거 [Dictionary Vocabulary](https://www.jsoftware.com/help/dictionary/vocabul.htm)는 역사적 참고이며 현재 지원 목록으로 사용하지 않는다.
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
| /. /.. | Oblique / Key / Key dyad | oblique SegmentView/Reindex → CellApply; key Classify/GroupBy → SegmentView → CellApply, optionally grouped reduction | grouping equality/order, segment descriptors, assembly | Compose; avoids materialized groups and opens reduce-by-index route |
| /: \: | Grade Up/Down / Sort | Grade; sort result can be Grade → Gather | comparison order, stability/tolerance, dtype | Direct; radix/merge/small/GPU algorithm identity retained |
| \ | Prefix / Infix | insert-compatible prefix Scan; general prefix SegmentView(prefix family) → CellApply; infix WindowView/SegmentView → CellApply | window length, order, boundaries, assembly | Direct/Compose |
| \. | Suffix / Outfix | suffix Scan when legal or segment family; outfix SegmentView + Concat/Assemble → CellApply | same as above | Compose |
| [ ] [: | Same/Left, Same/Right, Cap | value projection / function-graph semantics | valence, provenance | Control/value; no new compute basis |
| [. ]. ]: | Lev / Dex conjunction, Ident adverb | constructor에서 기존 noun/verb operand 선택 | 실제 result POS, operand reduction/효과, noun snapshot·verb late binding | Control/value; runtime 배열 kernel이나 병렬 힌트가 아님 |
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
| b. | Boolean/bitwise 또는 Verb Info | noun-operand bitwise kernel 또는 function rank/identity/obverse query | operand 종류, function header, numeric domain | Cell kernel/control; 정보 조회와 계산을 구분 |
| C. | Cycle-Direct / Permute | permutation representation conversion + Permute/Gather | cycle/direct validity | Value/Rep + Direct |
| e. | Raze In / Member | monad value traversal/raze-in; dyad Lookup(Membership) | equality/tolerance, boxed semantics | Direct + Value |
| E. | Find Matches (dyad) | WindowView → Elementwise(Match) → Reduce/Match as applicable | pattern shape, equality, boundaries | Compose; explicit windows expose fusion |
| f. f: | Fix 계열 | function/name fixing 정책을 가진 transformation | binding/POS, fix mode, implicit locative·recursion 범위 | Control; 두 표기의 동등성은 가정하지 않음 |
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
| u: x: | Unicode/Extended Precision | representation/runtime conversion or Elementwise conversion | encoding/numeric exactness | Value/Rep/Runtime |
| S: | Spread | NestedTraverse(level selector) → CellApply(u) → FlatAssemble | level selector, heterogeneous result/assembly | Direct/Compose |
| c. | Convert to Precision (dyad) | numeric precision conversion | requested precision, logical type/promotion/error | Value/Rep/Cell kernel; 실제 encoding은 downstream |
| m. | Modular arithmetic conjunction | 허용 operand의 modular numeric kernel 생성 | modulus, integer/exact domain, inverse 존재·오류 | Control/Cell kernel |
| F. F.. F.: F: F:. F:: Z: | Fold variants / fold status·termination | recurrence/control region + result collection | direction, valence, state/termination, empty/fill·error order | Control; 일반 Reduce/Scan으로 임의 치환하지 않음 |
| t. T. | Execute as task / thread·task·debug control | task creation, pyx result, synchronization/context effects | capture/namespace, await/open/error point, shared state·resource lifetime | Runtime/control; Taylor 연산이 아니며 fork 병렬 후보와 구별 |
| constant functions | constant broadcast/Generate when array result required | Generate/constant | dtype/shape from call context | Direct/value |
| ~ | Reflex/Passive/Evoke | function semantic transformation/name resolution | valence/binding | Control |
| ^: | Power | Iterate/loop over analyzed basis graph; static count may unroll/fuse | count, fixed-point/inverse semantics, effects | Control; no new array basis |
| $: $:: | Self-Reference / Shorten self-reference scope | recursion 및 scope transformation | 함수 identity, enclosing self-reference 범위, POS | Control; `$::`는 adverb이며 `$:` alias가 아님 |
| : :. :: | Explicit/Monad-Dyad, Obverse, Adverse | function/control/exception semantic graph | POS, inverse/obverse, error semantics | Control |
| =. =: | local/global assignment | binding/effect operation | locale/scope/version/effect ordering | Runtime/control |
| _ _. _: | infinity/indeterminate constants | constant/value semantics | numeric type | Value |
| NB. | Comment | frontend only | none | frontend, no IR op |

**Coverage conclusion v0.2 (정정):** 위 표는 검토한 primitive/form의 설계 분류이며 현재 NuVoc 전체의 parser/runtime/backend 지원 증명이 아니다. task/precision/fold/scope·modifier selection을 추가했지만 모든 항목의 semantic expansion·검증은 미완료다. 새로운 array basis가 필요한지는 각 미지원 family의 실제 계약을 검토한 뒤 판정한다.

<a id="current-j-vocabulary"></a>

##### 현재 vocabulary 감사 — NuVoc와 구현 상태 (2026-10-05)

현재 NuVoc index와 관련 개별 페이지를 읽고 pinned `ws.c` spelling→code, `t.c` POS/constructor와 Windows C 기본·AVX2의 `4!:0` 결과를 대조했다. 웹 문서의 최신성, 검토 source pin, 실행 DLL release는 별도 provenance다. wiki를 보고 DLL이 최신 source로 빌드되었다고 주장하거나 POS/rank를 추측하지 않는다. NuVoc 개별 문서 `c.`, ObsoleteSyntax, `[.`, `$::`는 웹 fetch가 실패했으므로 index와 C 근거만 사용했다.

- **오래된 설명 수정:** `t.`는 task conjunction, `T.`는 thread/task/debug verb다. task 결과의 pyx/open·오류 전달·공유 namespace 의미를 effect/resource 경계로 다루며 Rust async나 GPU 실행을 이미 지원한다는 뜻이 아니다. [Task](https://code.jsoftware.com/wiki/Vocabulary/tdot), [Threads](https://code.jsoftware.com/wiki/Vocabulary/tcapdot).
- **누락된 현행 표기 추가:** `c.`, `m.`, `f:`, `/..`, `$::`, Fold 여섯 표기와 `Z:`, Lev/Dex/Ident, `{{ }}` direct definition과 `u.`/`v.` caller-context 의미를 inventory에 포함한다. `/..`는 key를 operand dyad의 왼쪽 인자로 제공하므로 `/.` alias가 아니다. [Key](https://code.jsoftware.com/wiki/Vocabulary/slashdot), [Modular](https://code.jsoftware.com/wiki/Vocabulary/mdot), [Fold](https://code.jsoftware.com/wiki/Vocabulary/fcap).
- **역사적 표기 분리:** `d.`/`D.`/`D:`/`t:`/`..`/`.:`/`s:`/`I:`는 현재 C 양 버전의 spelling error를 확인했다. 현행 coverage matrix에서 제거했다. NV3a에서는 fixed spelling의 오류 분류를 일반화해 Rust도 spelling error로 보고한다. 전체 name/numeric grammar는 아래 NV3의 후속 작업이다. `s:`는 NuVoc obsolete 구역의 symbol verb이며, 양 C DLL에서도 거부했다. 내부 Symbol 타입이 존재하는 것과 현행 J의 `s:` 지원은 별개다. `I:` 역시 현행 `I.`/`i:`와 혼동하지 않는다.
- **rank/용어 교정:** `@`는 Atop, `@:`는 At다. 내부 `ConjunctionId::Atco`은 기존 `@:` 식별자이며 주석으로 이를 명시했다. `@`를 동등한 alias로 등록하지 않는다. NuVoc의 동작 설명과 C의 `b.0` intrinsic header/IRS 경로를 구분하고 wiki 표의 rank만 보고 이미 검증한 header를 덮어쓰지 않는다. `u"v`/`m"v` Copy Rank도 별도 form이다. [Copy Rank](https://code.jsoftware.com/wiki/Vocabulary/quotev).

**NV1 코드 변경(이전 단계):** `[.`·`].`는 각각 왼쪽·오른쪽 noun/verb를 반환하는 conjunction이며 `]:`는 operand를 반환하는 adverb다. primitive registry **7**, Graph IR **0.7**에 반영했다. 선택된 함수의 원래 entity/NAME·late lookup과 noun snapshot을 유지한다. 선택 전에 필요한 noun reduction·assignment·error를 생략하지 않는다. 이 constructor는 선택 operand 자체를 결과로 내며 전체 구문/constructor provenance는 source·reduction·capture에서 보존한다. 선택 modifier를 새 배열 kernel로 만들지 않는다. tokenizer state machine은 변경할 필요가 없었다. capture의 `ConstructionNounSuccess.selected_input`으로 noun 선택 결과를 원래 dependency node에 연결하고 선택되지 않은 계산 노드도 유지한다. 아직 ordered-effect graph가 필요한 대입은 기존 경계를 유지한다. 정적 parser에서는 선택되지 않은 noun 계산을 보존할 경로가 없으면 명시적 Unsupported로 남기며, J 의미상의 오류로 바꾸거나 해당 계산을 삭제해 실행 가능하다고 승인하지 않는다.

`tools/vocabulary_audit.py`는 pinned `ws.c`에서 core spelling 후보를 추출해 C POS와 Rust enqueue를 대조한다. 구조 토큰·control words·direct-definition framing·이름 전체를 이 primitive 감사 하나로 검증하지 않는다. **word formation pass / enqueue POS verified / Unsupported coverage / runtime conformance**를 각각 구분한다. 원자료는 `reports/vocabulary-*-windows.json`에 보존한다. `ws.c`의 비영(非零) code만으로 설치된 primitive라고 판단하지 않는다. `w.c`는 `ds(e)`의 permanent usecount도 확인한다. `?:`/`` `. ``는 source code 후보지만 검토한 `t.c`에 설치되지 않고 양 DLL이 거부하므로 현행 valid primitive나 Rust 미지원 pass로 세지 않는다. 이 감사는 numeric grammar의 `__`/`_.` 전체 inventory를 다루지 않는다. 검증 숫자는 아래 NV1/NV2 gate에 기록한다. 표준 `tools/check-frontend-windows.ps1 -Avx2`도 양 DLL별 vocabulary 감사를 실행하므로 이후 frontend 검증 때 함께 갱신한다.

<a id="nv2-core-recognition"></a>

##### NV2 — core 표기·품사 인식과 실행 capability 분리

`PrimitiveSemanticId::Vocabulary` / `FunctionHead::VocabularyPrimitive`는 검증된 spelling/POS를 가진 operand-free core 함수 identity다. private-field `VocabularyPrimitive` catalog 108개를 core resolver에 추가해 `EnqueuedPayload::Function`으로 정상 queue와 공유 parser에 전달한다. 기존 실행 primitive를 덮어쓰지 않으며 extension NAME이나 독립 parser를 만들지 않는다. registry **8**, Graph IR **0.8**이다. `t.`/`T.`·Fold·Key dyad·precision·scope·constant functions도 bare binding과 AR/POS 보존 대상이지만 이번 단계에서 실행 구현을 주장하지 않는다.

새 descriptor의 rank/shape/dtype/effect/resource 계약은 미확인 상태로 둔다. `innate_ranks()`는 None, graph rule은 DynamicOrUnknown, 실행 lowering/executor 및 새 descriptor 자체의 modifier 적용은 Unsupported다. 기존 rank/selector/train이 검증된 구조를 만들 수 있으면 identity를 보존하되 새 kernel/purity/finite rank를 만들지 않는다. RHS Copy Rank에 미확인 descriptor가 들어가면 header를 추측하지 않는다. primitive modifier의 by-value stacking은 검토 `t.c`의 VF2NAMELESS 근거이며 purity와 별개다. ordinary verb alias의 late binding은 그대로 유지한다.

`a.`는 byte 0–255의 char vector, `a:`는 empty bool vector를 담는 scalar box인 실제 noun이다. `OnceLock<Value>`에 immutable shared payload를 보존해 enqueue occurrence가 버퍼를 복사하지 않는다. 이 두 bounded builtin을 추가한 것이 대형 input/intermediate noun snapshot cache를 도입한 것은 아니다. 둘 다 기존 logical Value 경로로 처리하며 device/layout를 noun 의미에 넣지 않는다.

`tools/vocabulary_audit.py`는 양 C DLL의 bare binding `4!:0`/`5!:1`과 Rust parser projection을 140개 함수에서 비교하고 세 noun의 type/shape/payload도 확인한다. 이 단계에서 gerund의 `!`를 실제 함수 identity로 decode한 뒤 Prefix entity를 구성하는 것은 가능해졌다. 그 함수의 실행은 여전히 Unsupported이므로 기존 회귀 테스트를 construction 성공 / 실행 capability 경계로 교정하고 공통 C fixture를 추가했다. core spelling 인식과 full modifier/control/name semantics는 별개이며 NV3–NV5를 계속 따른다.

근거: [ws.c spelling codes](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ws.c), [t.c permanent nouns·POS·VF2NAMELESS](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/t.c#L81), [w.c installed primitive check](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/w.c#L140). source pin과 실행 DLL provenance의 분리는 유지한다.

**jsource 특수 경로 누락 재감사:** 아래 basis 대응 외에 확인된 convolution (\`f//.@:(g/)\`), Cut–Scan–Raze, byte LUT, boolean-index, order-statistic, shape-RNG, boxed link, explicit \`M.\`, numeric exactness, hook comparisons는 [§4.1.3.6 M](#jsource-audit-m)의 source-guard/ownership 표를 기준으로 한다. 이 목록을 새로운 basis IR node의 필연적 추가로 해석하지 않는다.

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

##### 현재 A3 direct-lowering 구현 경계

M1 cutover 이후 `analysis::lower_graph()`가 canonical `logical_ir::Plan`을 직접 생성한다. 모듈/API와 완료 상태의 정본은 [§10 M1](#architecture-migration-checklist), [§12](#current-implementation-status)다. 아래는 basis 계약의 구현 경계다.

shared execution-semantic contract:
- `CallOp.execution_basis`: outer→inner `ExecutionBasis.layers`를 기록한다. 예: ranked reduction은 `[CellApply, Reduce]`를 보존한다.
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

zero-result SemanticCheck는 canonical A3 op/result-separated IR에서 표현한다. 제거된 transition container를 새 기능을 위해 복원하지 않는다.

##### basis contract 검증

각 basis op에는 최소 세 종류의 test가 필요하다.

1. verifier negative tests
   - 잘못된 axis/index/shape/role/descriptor를 reject
2. semantic/reference equivalence tests
   - generic/reference path와 optimized lowering의 result/error equivalence
3. composition tests
   - CellApply+Reduce, Window+Contract, GroupBy+Reduce, sparse representation 조합처럼 실제 fusion boundary를 포함

J error가 있는 case는 값만 비교하지 않고 **error class와 observable ordering**까지 비교한다.





---


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

앞으로 사람이 유지하는 프로젝트 기준 문서는 **이 `PROJECT.ko.md`가 정본이며 `PROJECT.md`는 같은 설계의 영어 mirror**다. Syntax 힌트 검토와 프레임워크 비교도 §4.1.2–3에 통합했으며 별도 설계 보고서를 유지하지 않는다. 새 설계·구조 결정은 한국어 정본을 먼저 갱신하고 영어 mirror를 같은 변경에서 갱신한다.

정본 유지 규칙:

- 전체 구현 상태와 코드 검토 기준은 §12에서 요약하고, 완료/미완료 관문은 §10의 해당 stage 체크리스트에서 관리한다. 다른 절은 고유 계약만 설명하고 이 정본 위치를 참조한다.
- syntax/graph 연구는 §4.1, 확장 primitive inventory는 §4.6, cell 의미의 proof 표본은 §4.11.4, 공통 검증 정책은 §11에 둔다.
- 역사 감사와 과거 설계는 날짜/검토 기준을 명시한다. 이전 v0.1 또는 transition 단계의 상태를 현재 구현으로 서술하지 않는다.
- 절을 이동하면 한·영 heading/상호 참조를 함께 갱신하고, 고정 출처·후보/채택/구현 상태·고유 검증 조건을 보존한다.


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
- COMPILER-HINTS-J-SYNTAX — §4.1.2로 통합
- J-GRAPH-FRAMEWORK-OPTIMIZATION-REVIEW — §4.1.3로 통합
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

### 15.7 문서 완전성 감사 — stage contract가 닫혀 있는가? (2026-10-06)

RustJ 문서는 개별 주제의 깊이는 충분하지만, 설계가 커지면서 **각 stage 자체의 설명보다 stage 사이의 연결 계약이 흩어지는 문제**가 생겼다. 따라서 “문서가 길다/자세하다”와 “독자가 compiler를 처음부터 끝까지 재구성할 수 있다”를 같은 것으로 보지 않는다.

앞으로 주요 compiler stage는 최소한 다음 질문에 답해야 문서적으로 **닫힌(closed) contract**로 본다.

~~~text
1. 왜 이 stage가 존재하는가?
2. 입력은 무엇인가?
3. 출력은 무엇인가?
4. 반드시 보존해야 하는 semantic information은 무엇인가?
5. 이 stage가 결정하면 안 되는 것은 무엇인가?
6. 앞/뒤 stage와의 contract는 무엇인가?
7. 대표 source → IR/plan 예제가 있는가?
8. 현재 구현은 어디까지인가?
9. 어떤 verifier/test가 그 경계를 증명하는가?
10. 아직 미구현·미정인 부분은 무엇인가?
~~~

아래 평가는 **설계의 품질 평가가 아니라 문서 연결 완전성 평가**다. `충분`은 모든 구현이 끝났다는 뜻이 아니며, `부분`은 설계가 틀렸다는 뜻이 아니다.

| Stage / 경계 | 현재 문서 상태 | 이미 강한 부분 | 아직 닫히지 않은 부분 |
|---|---|---|---|
| word formation → enqueue → parser | **문서 계약 닫힘 / 구현 수렴 중** | A0.5/F0–F2/P0–P8, jsource oracle, 9-row reduction, name/assignment sequencing, differential gates, §3.3.2.2 `+/ y` canonical frontend trace | locative/definition/gerund/value-dependent constructor의 실제 지원 범위는 A0.5 checklist와 differential gate가 결정; orientation contract 자체는 닫힘 |
| Semantic Construction / binding / dynamic semantics | **현재 frontend E2E subset 완료 / 일반 compiled CFG 미완료** | FunctionEntity/JEntity, late NameRef, definition metadata, 지원 per-call LocalFrame·if/while/for/try·nested direct/string explicit, A3 definition reference | full J/locale·locative, body Graph·Logical analysis, source-frame diagnostics 및 compiled Branch/CondBranch/value-merge는 후속이다. A3 Terminator는 Return-only |
| J Semantic → J Graph IR | **문서 계약 닫힘** | GraphForm/GraphBasis/GraphHint, provenance, applied graph, Graph/Execution 분리, §4.1.0 canonical suite로 `@:`/ordinary+capped fork/hook/rank/reduce/prefix-infix를 동일 형식 비교 | 남은 gap은 form별 실제 lowering/test coverage이지 stage ownership 설명 부재가 아님 |
| Graph analysis → candidate/proof | **문서 계약 보강됨 / 구현 부분** | §4.1.4에 orthogonal evidence, derived lifecycle, evidence owner, guarded legality, overlap/selection 규칙을 통합 | 공통 `CandidateEvidence/ProofBundle`·obligation discharge·SelectionPlan은 **미구현**. 개별 proof algorithm의 세부 구현은 해당 optimizer 착수 시 verifier/test와 함께 확정 |
| J Graph → Execution Semantic Lowering → A3 | **문서 계약 대체로 닫힘 / CFG 구현 미완료** | direct lowering, Graph/Execution fact drift check, Execution Basis, SemanticCheck, effect/error/speculation, verifier, schema version, canonical mean trace, §3.7.1 planned/current control-flow handoff | A3-v0는 실제로 single-block/Return-only다. 남은 것은 문서 예제가 아니라 **Branch/CondBranch/block merge를 포함한 executable explicit-definition CFG lowering과 differential E2E** |
| Route analysis / partition | **문서 계약 보강됨 / 구현 부분** | §2.5.1에 live-in/out, effect live-out, SemanticCheck, guard, representation-neutral bridge, region legality 계약을 통합 | 현재 `RouteRegion { class, operations }`와 contiguous grouping은 v0 helper. 실제 bridge/region-wide verifier와 mixed-route executor는 미구현 |
| Schedule / Physical Planner | **M4-v0 본체 미완료 / i. family 선택기 일부 구현** | §5.2.1에 `PlanBufferId ≠ runtime BufferId`, PhysicalView, BindInput/Check/View/Materialize/Kernel/Return, lifetime/reuse/verifier/error-cleanup 계약을 정의 | `physical_plan`의 빈 A3/closed dense literal identity 타입·verifier·executor는 제한적으로 구현됨(#4). 일반 planner, Add/Check/View/Materialize/lifetime/reuse는 미구현. Transfer/Sync/async는 M4 이후 |
| Native Executor | **M4-v0 계약 대체로 닫힘 / 구현 미완료** | §5.2.1/§5.3에 op 역할, verifier, cleanup/error, executor non-responsibility, canonical mean planned route를 연결 | 실제 Physical Executor와 differential E2E test가 없음. stateful/async execution contract는 후속 |
| fallback / guard miss / replay | **문서 계약 보강됨 / dispatcher 미구현** | §3.9.4에 route fallback/guard miss/replay/continuation 구분, decision table, commit frontier, RuntimeSemanticFallback의 정확한 의미를 통합 | integrated guard dispatcher/exact continuation/transaction rollback은 미구현이며 capability로 주장하지 않음 |
| external route / GPU | **boundary contract 고정 / 구현 보류** | §5.5.1에 adapter input/capability/output, SemanticCheck/error/effect/token mapping, bridge/ownership, round-trip verifier, failure class를 정의 | production adapter는 미구현. M6까지 implementation gate로만 유지하며 CUDA는 의도적으로 보류 |
| validation / versioning | **문서 계약 보강됨 / 후속 verifier 구현 필요** | frontend differential gate, J Graph 0.10/A3 0.6 exact schema+registry verifier, §5.7.5 negative matrix, §5.7.6 explicit migration/downgrade policy | Candidate/Route/Physical/External negative verifiers는 각 stage 구현과 함께 추가; portable serialization 자체는 아직 미제공 |

#### 15.7.1 1차 감사에서 닫은 문서 계약

현재 구현 우선순위 M2를 바꾸지 않은 채, 2026-10-06 1차 감사에서 다음 다섯 빈칸을 **문서 계약 수준에서** 닫았다. 이는 구현 완료 체크가 아니다.

- [x] **Candidate lifecycle / proof discharge** — §4.1.4
- [x] **RouteRegion boundary contract** — §2.5.1
- [x] **M4 최소 PhysicalPlan v0 schema / verifier / cleanup contract** — §5.2.1
- [x] **Fallback / guard miss / no-replay decision table** — §3.9.4
- [x] **Canonical end-to-end compiler trace** — §4.11.4.12의 `(+/ % #) y`

특히 canonical trace에는 **현재 구현 stop line**을 넣어 route prototype 이후의 Schedule/PhysicalPlan/CPU result 구간을 `planned M4`로 표시했다. 목표 architecture 그림을 구현 완료 증거로 사용하지 않는다.

#### 15.7.2 2차 감사에서 닫은 문서 계약

- [x] **Frontend canonical sentence trace** — §3.3.2.2 `+/ y`
- [x] **Explicit-definition control-flow handoff** — §3.7.1
- [x] **Canonical J Graph example suite** — §4.1.0
- [x] **External adapter boundary contract** — §5.5.1
- [x] **Cross-stage negative verifier matrix + serialization migration policy** — §5.7.5–§5.7.6

이 단계에서도 구현하지 않은 타입/API를 구현 완료처럼 쓰지 않았다. 특히 Branch/CondBranch, CandidateEvidence/SelectionPlan, RouteBoundary concrete type, PhysicalPlan, ExternalRegionPlan은 **target concepts**이며 현재 코드에 모두 존재하는 API가 아니다.

#### 15.7.3 현재 남은 빈칸 — implementation-driven detail

1·2차 감사 후 남은 큰 항목은 architecture owner가 없는 빈칸이라기보다 **구현에 들어가야 구체 타입과 proof algorithm을 정할 수 있는 세부사항**이다.

- 각 fusion/rewrite/scan obligation의 실제 proof algorithm과 cached-evidence invalidation 구현
- explicit-definition CFG의 concrete Branch/CondBranch/block-argument API와 loop/try/select lowering
- RouteBoundary/BridgeRequirement의 concrete Rust 타입과 mixed-route executor integration
- PhysicalPlan concrete op structs, planner algorithm, Buffer reuse implementation
- 첫 external adapter별 actual capability matrix와 emitted-IR round-trip implementation
- async token/timepoint, stateful native route, exact continuation/transactional rollback처럼 아직 의도적으로 후순위인 semantics
- portable artifact를 실제 파일/프로세스 경계로 내보낼 때 선택할 wire format

이 항목은 지금 임의 타입을 미리 고정하지 않는다. 해당 구현 단계가 시작될 때 **현재 stage contract → 최소 타입 → verifier/negative test 하나 → 다음 의미** 순서로 구체화한다.

#### 15.7.4 문서 유지 규칙

- 새 major stage/type을 추가할 때 위 10개 질문 중 해당 항목을 함께 갱신한다.
- 구현 타입 이름과 문서의 개념 이름이 다르면 “현재 구현명 / 목표 개념명”을 명시한다.
- 목표 architecture 그림과 현재 구현 상태를 같은 시제로 쓰지 않는다.
- framework 비교를 수정하면 `FOUNDATIONS`, `PROJECT`, `README`, `AGENTS`에 같은 주장을 중복해 둔 곳이 없는지 교차 검색한다.
- 새 validation gate를 완료해 정본에 기록할 때 문서 상단 `최신 검증`과 §12 현재 요약도 같은 변경에서 갱신한다. historical gate의 “최신” 표현은 당시 시점임을 명시한다.
- 한 stage의 상세 절이 길어질수록 **입력/출력/금지/다음 경계** 요약을 절 앞이나 끝에 유지한다.
- 문서 감사에서 발견한 항목을 별도 Markdown 보고서로 분리하지 않는다. 이 절과 §10/§16의 기존 체크리스트에 흡수한다.

---

## 16. 다음 작업 — 정본 체크리스트로 이동

작업 순서는 [§10 M0–M6](#architecture-migration-checklist)만 유지하고, 현재 구현 상태는 [§12](#current-implementation-status)를 따른다. 이 절에 별도의 Proof Slice 완료표나 과거 단계별 상태표를 복제하지 않는다.

1. **M2/frontend**: F0–F2/P0–P8의 jsource-compatible construction/name/assignment cutover를 완료한다. M1 canonical Logical IR cutover는 완료 상태다.
2. **M3/배열 경계**: logical value와 physical representation의 code/API 분리를 수렴시킨다.
3. **M4/CPU vertical slice**: verified Logical IR → 최소 Schedule/Physical Plan → CPU Physical Executor를 연결한다. [matrix mean 표본](#mean-proof-example)은 implicit cell semantics를 검증하며 analyzer smoke test만으로 실행 완료를 판정하지 않는다.
4. **M5–M6**: 그 뒤 route/schedule/resource/cost 선택과 검증된 external adapter를 확장한다. 실제 CUDA 구현은 사용자가 재개하기 전까지 보류한다.
5. **JE0–JE6 보조 트랙**: JE0과 최소 boundary `JEntity` seam(JE1/JE2 일부)은 M2와 병행할 수 있다. generic higher-order collection과 storage-affecting migration은 M2/M3 안정화 뒤로 미룬다. 이 트랙은 M4 첫 CPU vertical slice의 선행 조건이 아니다.

6. **IO-01~IO-30 병행·후속 트랙:** [§8.5 I/O 설계 계약](#out-of-core-io-contract)과 [§10 IO 체크리스트](#out-of-core-io-checklist)를 기준으로, M2 중에는 IO-01·IO-25의 source audit와 IO-02의 효과/오류 계약 검토만 병행한다. 동기 I/O baseline(IO-05~08, IO-26~28)은 첫 native CPU vertical slice 이후, 불필요한 읽기 제거와 cache(IO-09~12, IO-29)는 의미 증명 이후, async/transfer(IO-13~20)는 그다음, 비교/확장(IO-21~24, IO-30)은 실측으로 수용한다. **문서 계획을 구현 완료로 표시하지 않는다.**

장기 architecture의 full TargetProfile/mixed-route/async 모델이나 확장 primitive 전체를 첫 CPU slice의 선행 조건으로 삼지 않는다. 해당 의미를 최적화 대상으로 열 때는 [§11의 검증 정책](#validation-policy)과 대응 semantic golden을 먼저 충족한다.

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

