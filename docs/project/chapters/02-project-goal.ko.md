[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)



<a id="read-goal"></a>
<a id="1-프로젝트-목적"></a>
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
