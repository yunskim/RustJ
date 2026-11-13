[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)



<a id="read-goal"></a>
<a id="1-프로젝트-목적"></a>
## 1. 프로젝트 목적

RustJ는 `jsource`의 C 구현을 줄 단위로 Rust로 번역하는 프로젝트가 아니다.

**프로젝트의 중심 목표**는 다음과 같다.

> **J의 함수 합성과 배열 대수를 컴파일러가 의미 수준에서 이해하여, 기존 실행 방식에서 놓치기 쉬운 동등한 계산 형태와 최적화 기회를 발견하고, J의 관찰 가능한 의미를 보존하면서 실제 실행에 활용한다.**

RustJ는 이것을 연구하고 구현하는 **Rust 기반 J 배열 컴파일러/runtime**다. CPU·GPU를 모두 표현 가능한 장기 설계 대상으로 유지하지만, 그것이 같은 시점에 두 backend를 완성한다거나 범용 GPU compiler 경쟁에서 승리해야 한다는 뜻은 아니다.

<a id="11-핵심-목표와-개발-우선순위"></a>
### 1.1 핵심 목표·성공 기준·개발 우선순위

| 구분 | 목적 및 판단 기준 |
|---|---|
| **차별화 목표(왜 RustJ인가?)** | J의 hook/fork/train, 합성, Rank, adverb/conjunction, 배열 축약·재배열에 담긴 **계산 구조**를 Graph IR에서 분석한다. 대수적 동치 후보를 검증하고, 선택한 계산을 Logical/Physical 경로로 연결한다. |
| **필수 조건(무엇을 포기할 수 없는가?)** | J-C를 기준으로 값·타입·shape/rank·빈 배열·prototype/fill·이름/효과·첫 오류·수치 의미를 보존한다. 최적화가 이 조건을 충족하지 못하면 거부한다. |
| **독립적인 첫 구현 성과** | M2 frontend 의미 수렴 → M3 논리/물리 경계 → M4 **검증된 native CPU 수직 실행 경로**를 구축한다. 고급 최적화가 없어도 지원 범위 안에서는 정확히 실행되는 컴파일러여야 한다. |
| **최적화의 실증 기준** | 작은 J 배열 계산 사례에서 원본 Graph와 재작성 후보를 모두 보존하고, 동등성/guard를 검증한 뒤, 실제 CPU 실행에서 시간·메모리 등 **측정한 비용**으로 유익함을 판정한다. 성능이 나쁘거나 증거가 부족하면 원본/검증 경로를 선택한다. |
| **확장 수단** | SPORES/egg, Futhark, MLIR, StableHLO, 외부 라이브러리 등은 연구·탐색·실행을 위한 선택적 방법이다. 필요한 기술만 근거와 구현 비용을 검증한 후 도입한다. |

**명시적 비목표:** Futhark·XLA·MLIR·SPORES를 각각 재구현하는 일, 모든 논문 아이디어를 구현하는 일, 즉시 세계 최고 수준의 GPU 커널 성능을 내는 일, 전체 J 연산의 보편적인 동치식을 증명하는 일을 단기 성공 조건으로 삼지 않는다. **목표는 기능 목록의 최대화가 아니라, J 함수·배열 대수의 특성이 실제로 검증 가능한 최적화를 만들 수 있음을 작은 end-to-end 사례부터 축적하는 것**이다.

**작업 선택 원칙:** 새 최적화·IR 기능을 제안할 때는 먼저 (1) J의 의미와 계산 구조 이해에 무엇을 추가하는지, (2) 기존 Graph/Logical/Physical 책임에서 해결 가능한지, (3) 독립적인 의미 검증과 비용 측정이 가능한지, (4) 기존 backend에 위임할 수 있는지를 확인한다. 연구 문헌의 구현 목록을 자동으로 RustJ 필수 로드맵으로 승격하지 않는다. [§10 이행 계획](./11-roadmap-checklists-p01.ko.md#architecture-migration-checklist), [§16 다음 작업](./18-next-tasks.ko.md), [Futhark 선행연구](../../research/futhark-compiler-ir-prior-art.ko.md)는 이 우선순위를 따른다.

`jsource`는 다음을 위한 reference implementation이다.

- J의 관찰 가능한 언어 의미
- primitive의 corner case
- rank/cell/frame/agreement
- 오류와 타입 승격
- 배열 동작과 이름 의미
- 차등 테스트 oracle
- CPU 구현·메모리 정책의 비교 대상

C J 엔진을 RustJ의 정상 실행 fallback으로 사용하지 않는다.

<a id="12-핵심-배열-모델-결정--logical-array와-physical-array를-분리한다"></a>
### 1.2 핵심 배열 모델 결정 — Logical Array와 Physical Array를 분리한다

Logical Array는 J-visible type·shape·atom order와 boxed/sparse 의미를 소유한다. Physical Array는 buffer·stride·offset·layout·placement 등 실행 표현을 소유한다. logical value의 존재는 별도 buffer 할당을 뜻하지 않는다.

모델과 완료 조건은 [§6](./07-array-model.ko.md#logical-physical-array-model), GPU view/placement는 §7, alias/lifetime 조건은 §8에서 정의한다. 도입부에 상세 구조와 완료표를 복제하지 않는다.

<a id="13-이름-정책"></a>
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
