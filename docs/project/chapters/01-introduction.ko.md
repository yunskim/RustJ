[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)

[English](../../../PROJECT.md) | **한국어 — 정본(canonical)**

<a id="rustj-통합-프로젝트-문서"></a>
# RustJ 통합 프로젝트 문서

> 상태: **유일한 권위 문서(authoritative project document)**  
> 문서 갱신일: 2026-10-13
>
> 앞으로 아키텍처, 설계 결정, 구현 계획, 지원 범위, 진행 상태, 검증 정책과 주요 검증 결과는 이 문서에 통합한다.  
> [FOUNDATIONS.ko.md](../../../FOUNDATIONS.ko.md)는 RustJ가 왜 compiler-oriented architecture를 택하는지, interpreter 전통에서 무엇을 보존해야 하는지, 어떤 compiler 설계가 J에서 회귀가 되는지를 규정하는 **필수 설계 기반 문서**다. frontend·Semantic IR·runtime/JIT/AOT 경계·rank/CellApply·target 설계를 변경하기 전 반드시 함께 검토한다.  
> 그 외 개별 설계 보고서·진행 보고서·체크리스트 Markdown 파일은 새로 만들지 않는다. 기계가 생성한 측정 원자료(JSON/JSONL)는 `reports/`에 별도로 보존한다.

<a id="read-intro"></a>
<a id="처음-읽는-사람을-위한-소개--rustj를-어떻게-이해하면-되는가"></a>
## 처음 읽는 사람을 위한 소개 — RustJ를 어떻게 이해하면 되는가

이 문서는 구현 세부사항과 검증 기록까지 포함하는 통합 문서이므로, 처음부터 모든 절을 순서대로 읽으면 RustJ의 전체 그림보다 세부사항이 먼저 보일 수 있다. 처음 읽을 때는 먼저 이 절의 **네 단계 mental model**만 잡고, 이후 필요한 세부 절로 내려가는 것을 권장한다.

<a id="rustj를-한-문장으로-설명하면"></a>
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

<a id="왜-굳이-네-단계로-나누는가"></a>
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

<a id="jsource와-무엇이-다른가"></a>
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

<a id="jaxa에서-무엇이-발전했는가"></a>
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

<a id="다른-compiler와의-관계"></a>
### 다른 compiler와의 관계

이 전체 방향은 RustJ만의 특이한 발상이 아니다. LLVM, MLIR, XLA/JAX, Futhark 등 현대 compiler도 일반적으로 **높은 수준의 의미/연산 구조를 보존한 뒤 점진적으로 target-specific representation으로 lowering**한다.

RustJ의 특징은 그 보편적인 compiler 구조 앞단에 **full J semantics를 소유하는 frontend**를 강하게 둔다는 점이다. 다른 array compiler의 좋은 layering 원칙을 참고하되, Rank·derived verb·Hook/Fork·name semantics 같은 J 고유 의미를 compiler 편의를 위해 다시 정의하지 않는다.

<a id="처음-읽을-때의-권장-순서"></a>
### 처음 읽을 때의 권장 순서

**첫 번째 읽기(전체 그림):** 이 소개 → [§1 프로젝트 목적](./02-project-goal.ko.md#read-goal) → [§2 최상위 아키텍처](./03-architecture-p01.ko.md#read-architecture). 여기까지 읽으면 네 단계의 책임과 각 단계가 아직 결정하지 않는 사항을 설명할 수 있어야 한다.

**두 번째 읽기(핵심 계약):** [§3 J Semantic IR](./04-semantic-array-ir-p01.ko.md#read-semantic) → [§4 J Graph·Execution Semantic Lowering](./05-graph-execution-lowering-p01.ko.md#read-graph-execution) → [§5 Logical/Physical Plan·Executor](./06-plans-executor-p01.ko.md#read-physical). 필요한 단계의 계약만 읽고, 모든 예제와 역사 기록을 연속해서 읽을 필요는 없다.

**세 번째 읽기(구현 시):** [§9 지원 범위](./10-implementation-scope.ko.md#read-scope) → [§10 이행 체크리스트](./11-roadmap-checklists-p01.ko.md#read-roadmap) → [§11 검증 정책](./12-validation.ko.md#read-validation) → [§12 최신 상태](./13-current-status.ko.md#read-status).

**선택적 심화:** [§13 선행 구현·JAXA 역사·소스 조사](./13-current-status.ko.md#read-references)와 [심화 기술 계약](./15-advanced-contracts-p01.ko.md#read-advanced)은 특정 설계 선택을 검증할 때 참조한다. 과거 측정·완료 기록은 현재 HEAD의 성공 주장으로 읽지 않는다.


<a id="빠른-안내--현재-우선순위와-문서-읽기"></a>
### 빠른 안내 — 현재 우선순위와 문서 읽기

- **정의 실행·E2E 최신 상태(2026-10-07):** 일반 explicit/direct 호출에 이어 for/for_name 반복, 중첩 direct/문자열 explicit의 독립 scope, A3 함수 참조 전달을 구현했다. 기존 definition 수용 테스트 21개 모두 활성화했다. 본문 CFG lowering/compiled execution, 일반 locale 및 CUDA 실행은 후속이다. 아래 Definition 후속 검증 절을 따른다.
- **목표와 원칙:** full J의 의미를 보존하는 Rust 커널/컴파일러. C는 차분 oracle이며 정상 실행 fallback이 아니다. Logical Array와 Physical Representation은 분리한다.
- **현재 우선순위:** M2 tokenizer → enqueuer → parser 의미 수렴을 계속한다. [§O.5 프레임워크 이행 체크리스트](./14-framework-research-p02.ko.md#framework-migration-checklist)와 [§Q 전체 jsource 최적화 이행 체크리스트](./14-framework-research-p03.ko.md#jsource-optimization-migration) 및 [§10 IO 이행 체크리스트](./11-roadmap-checklists-p05.ko.md#out-of-core-io-checklist)를 M2→M3→M4 완료 게이트의 단일 추적표로 사용한다. Graph IR의 구조·부분 facts 보존과 최적화/실행 허가는 별개다. 이후 M3 경계를 정리하고 M4 Native CPU vertical slice를 검증한다. GPU 친화적 설계는 유지하되 CUDA 실행 구현은 유보한다. 외부 route는 capability를 증명한 영역에서 점진적으로 연다.
- **가장 최근의 Windows 검증 기록(2026-10-08, frontend admission/handoff 및 decimal/extended/rational literal 및 rational 산술/extended 나눗셈/rational reduce·균일 rank 조립 변경):** default/portable 각각 **746 passed / 0 failed / 0 ignored**, Python **85 passed**, fmt/clippy 통과. Definition 호출 감사 **144/144 matched**, 별도 NAME 효과/배열 감사 **104/104 matched**를 기록했다. 이는 해당 Windows 검증 범위의 결과이며 Linux·GitHub CI·GPU 또는 full J 동등성 검증이 아니다. 과거 gate 수치는 각 시점의 기록으로만 읽는다.
- **읽기 순서:** 설계 근거는 [FOUNDATIONS.ko.md](../../../FOUNDATIONS.ko.md), 이름·효과·실행 경로의 조건은 [동적 의미와 컴파일 경계 계약](./04-semantic-array-ir-p04.ko.md#dynamic-semantic-boundaries), 실행 가능한 작업과 검증은 §10–§11을 따른다. 과거 단계별 gate는 이력이며 최신 지원 상태와 구분한다. 정본·체크리스트를 별도 Markdown으로 분리하지 않는다.
