[English](PROJECT.md) | **한국어 — 정본(canonical)**

# RustJ 프로젝트 문서 — 개요와 목차

RustJ는 J 언어의 관찰 가능한 의미론을 보존하면서 CPU·GPU 등 다양한 경로로 실행할 수 있는 배열 컴파일러를 목표로 합니다.

**문서 정본 구조:** 이 페이지는 진입점이며, 상세 설계·구현 상태·체크리스트는 아래 연결된 `docs/project/chapters/*.ko.md`가 주제별 정본입니다. 영어 문서 [PROJECT.md](PROJECT.md)는 별도 분할 전까지 기존 통합 영어 문서를 유지합니다. [FOUNDATIONS.ko.md](FOUNDATIONS.ko.md)는 필수 설계 기반 문서입니다.

- [문서 분할 계획](docs/project/MODULARIZATION_PLAN.ko.md) · [실행 체크리스트 #64](https://github.com/yunskim/RustJ/issues/64)
- [Agent 작업·문서 규약](AGENTS.md) · [프로젝트 README](README.ko.md)
- [라이선스](LICENSE) · [기여자 계약](CLA.ko.md)

## 신규 설계 초안

- [Conv1D blackbox · Algebraic View · SPORES · Adjoint · Logical IR lowering (입출력 예제)](docs/project/conv1d-algebraic-view.ko.md) — 제한적 프로토타입 구현, 정식 IR 통합 보류(NAME/locale 선행)

## 상세 문서

### 처음 읽는 사람을 위한 소개 — RustJ를 어떻게 이해하면 되는가

- [본문](./docs/project/chapters/01-introduction.ko.md)

### 1. 프로젝트 목적

- [본문](./docs/project/chapters/02-project-goal.ko.md)

### 2. 최상위 아키텍처

- [본문 1/2](./docs/project/chapters/03-architecture-p01.ko.md)
- [본문 2/2 — 2.4.2 J semantics 보존과 compiler fact 축적 원칙](./docs/project/chapters/03-architecture-p02.ko.md)

### 3. J Semantic Array IR

- [본문 1/4](./docs/project/chapters/04-semantic-array-ir-p01.ko.md)
- [본문 2/4 — 3.3.4 Function DAG의 shape는 J parser reduction rule이 결정한다](./docs/project/chapters/04-semantic-array-ir-p02.ko.md)
- [본문 3/4 — 3.7.1 Explicit-definition control-flow handoff example](./docs/project/chapters/04-semantic-array-ir-p03.ko.md)
- [본문 4/4 — 3.8 sentence evaluation order와 namespace mutation](./docs/project/chapters/04-semantic-array-ir-p04.ko.md)

### 4. J Graph Analyzer와 Execution Semantic Lowering

- [본문 1/2](./docs/project/chapters/05-graph-execution-lowering-p01.ko.md)
- [본문 2/2 — 4.1.2.6 다른 배열 컴파일 프레임워크와의 대조](./docs/project/chapters/05-graph-execution-lowering-p02.ko.md)

### 5. Logical Plan, Physical Plan, Executor

- [본문 1/2](./docs/project/chapters/06-plans-executor-p01.ko.md)
- [본문 2/2 — H-P — M3→M4 구현 인수 패킷과 물리 검증 체크리스트 계약 (2026-10-07; 문서 전용)](./docs/project/chapters/06-plans-executor-p02.ko.md)

### 6. 논리 배열과 물리 배열 — 핵심 architecture decision

- [본문](./docs/project/chapters/07-array-model.ko.md)

### 7. GPU 친화적 배열 설계

- [본문](./docs/project/chapters/08-gpu.ko.md)

### 8. 메모리·alias·수명 원칙

- [본문](./docs/project/chapters/09-memory-alias.ko.md)

### 9. 언어 및 구현 범위

- [본문](./docs/project/chapters/10-implementation-scope.ko.md)

### 10. 구현 계획과 체크리스트

- [본문 1/5](./docs/project/chapters/11-roadmap-checklists-p01.ko.md)
- [본문 2/5 — JE2 구현 — namespace와 runtime result의 JEntity 수렴 (2026-10-04, parti](./docs/project/chapters/11-roadmap-checklists-p02.ko.md)
- [본문 3/5 — 최적화에 사용할 frontend 정보 보존 (2026-10-03)](./docs/project/chapters/11-roadmap-checklists-p03.ko.md)
- [본문 4/5 — P2 — 하나의 9-row reduction engine으로 전환](./docs/project/chapters/11-roadmap-checklists-p04.ko.md)
- [§7.10.8 — JAXA Convolution adverb의 인자와 SPORES 가상 전개 설계](./docs/project/chapters/11-roadmap-checklists-p04.ko.md#conv-virtual-expansion)
- [§7.10.9 — Virtual Expansion 실제 계산·SPORES 최적화 예제집](./docs/project/chapters/11-roadmap-checklists-p04.ko.md#conv-virtual-expansion-examples)
- [§7.10.10 — Virtual Expansion 관련 연구·차별성·검증 체크리스트](./docs/project/chapters/11-roadmap-checklists-p04.ko.md#conv-virtual-expansion-related-work)
- [§7.10.11 — Virtual Expansion 입력·출력·후보 선택과 자원 계산 예제](./docs/project/chapters/11-roadmap-checklists-p04.ko.md#conv-virtual-expansion-walkthrough)
- [본문 5/5 — A1.5.2 JAXA의 static-memory claim을 RustJ에서 해석하는 방식](./docs/project/chapters/11-roadmap-checklists-p05.ko.md)

### 11. 검증 정책

- [본문](./docs/project/chapters/12-validation.ko.md)

### 12. 현재 검증·구현 상태 요약

- [본문](./docs/project/chapters/13-current-status.ko.md)

### 13. 프레임워크 조사에서 채택한 원칙

- [본문 1/3](./docs/project/chapters/14-framework-research-p01.ko.md)
- [본문 2/3 — C. RustJ 계층 경계](./docs/project/chapters/14-framework-research-p02.ko.md)
- [본문 3/3 — P.3 현재 known gaps / 바로 다음 순서 — M2 우선으로 재정렬 (2026-10-06)](./docs/project/chapters/14-framework-research-p03.ko.md)

### 심화 기술 계약 — primitive, Rank, 하드웨어 계획, Flow–Storage

- [본문 1/10](./docs/project/chapters/15-advanced-contracts-p01.ko.md)
- [본문 2/10 — 입력 정보 문제의 프레임워크 비교 (2026-10-04)](./docs/project/chapters/15-advanced-contracts-p02.ko.md)
- [본문 3/10 — 4.11.1 J agreement는 prefix frame agreement다](./docs/project/chapters/15-advanced-contracts-p03.ko.md)
- [본문 4/10 — 4.11.4.9 첫 differential test matrix](./docs/project/chapters/15-advanced-contracts-p04.ko.md)
- [본문 5/10 — 4.12 access pattern은 fusion 분석의 semantic lower bound다](./docs/project/chapters/15-advanced-contracts-p05.ko.md)
- [본문 6/10 — 4.15.14 J error precedence와 internal semantic probes를 구분한다](./docs/project/chapters/15-advanced-contracts-p06.ko.md)
- [본문 7/10 — 4.16.8 Capability lookup도 locale fragment를 합성한다](./docs/project/chapters/15-advanced-contracts-p07.ko.md)
- [본문 8/10 — 4.22 외부 compiler 조사에서 얻은 직접적인 설계 근거](./docs/project/chapters/15-advanced-contracts-p08.ko.md)
- [본문 9/10 — 다른 array-language/compiler basis와의 역대조](./docs/project/chapters/15-advanced-contracts-p09.ko.md)
- [본문 10/10 — ExecutionBasisExpansion](./docs/project/chapters/15-advanced-contracts-p10.ko.md)

### 14. jsource와 RustJ의 관계

- [본문](./docs/project/chapters/16-jsource.ko.md)

### 15. 문서 정책

- [본문](./docs/project/chapters/17-documentation-policy.ko.md)

### 16. 다음 작업 — 정본 체크리스트로 이동

- [본문](./docs/project/chapters/18-next-tasks.ko.md)

### 라이선스 정책

- [본문](./docs/project/chapters/19-license.ko.md)

## 주요 체크리스트 및 계약 바로가기

- [architecture-migration-checklist](docs/project/chapters/11-roadmap-checklists-p01.ko.md#architecture-migration-checklist)
- [dynamic-boundary-checklist](docs/project/chapters/11-roadmap-checklists-p01.ko.md#dynamic-boundary-checklist)
- [nn-extension-checklist](docs/project/chapters/15-advanced-contracts-p01.ko.md#nn-extension-checklist)
- [framework-migration-checklist](docs/project/chapters/14-framework-research-p02.ko.md#framework-migration-checklist)
- [jsource-optimization-migration](docs/project/chapters/14-framework-research-p03.ko.md#jsource-optimization-migration)
- [validation-policy](docs/project/chapters/12-validation.ko.md#validation-policy)
- [current-implementation-status](docs/project/chapters/13-current-status.ko.md#current-implementation-status)
- [out-of-core-io-checklist](docs/project/chapters/11-roadmap-checklists-p05.ko.md#out-of-core-io-checklist)
- [formal-ir-verification-checklist](docs/project/chapters/11-roadmap-checklists-p05.ko.md#formal-ir-verification-checklist)
- [extension-primitive-inventory](docs/project/chapters/15-advanced-contracts-p01.ko.md#extension-primitive-inventory)


## 기존 링크 호환성

기존 명시 앵커와 제목 앵커는 원문을 소유한 모듈로 연결합니다. 영어 미러는 통합 문서를 유지합니다.

<a id="read-intro"></a>
- [read-intro](docs/project/chapters/01-introduction.ko.md#read-intro)

<a id="rustj-통합-프로젝트-문서"></a>
- [rustj-통합-프로젝트-문서](docs/project/chapters/01-introduction.ko.md#rustj-통합-프로젝트-문서)

<a id="처음-읽는-사람을-위한-소개--rustj를-어떻게-이해하면-되는가"></a>
- [처음-읽는-사람을-위한-소개--rustj를-어떻게-이해하면-되는가](docs/project/chapters/01-introduction.ko.md#처음-읽는-사람을-위한-소개--rustj를-어떻게-이해하면-되는가)

<a id="rustj를-한-문장으로-설명하면"></a>
- [rustj를-한-문장으로-설명하면](docs/project/chapters/01-introduction.ko.md#rustj를-한-문장으로-설명하면)

<a id="왜-굳이-네-단계로-나누는가"></a>
- [왜-굳이-네-단계로-나누는가](docs/project/chapters/01-introduction.ko.md#왜-굳이-네-단계로-나누는가)

<a id="jsource와-무엇이-다른가"></a>
- [jsource와-무엇이-다른가](docs/project/chapters/01-introduction.ko.md#jsource와-무엇이-다른가)

<a id="jaxa에서-무엇이-발전했는가"></a>
- [jaxa에서-무엇이-발전했는가](docs/project/chapters/01-introduction.ko.md#jaxa에서-무엇이-발전했는가)

<a id="다른-compiler와의-관계"></a>
- [다른-compiler와의-관계](docs/project/chapters/01-introduction.ko.md#다른-compiler와의-관계)

<a id="처음-읽을-때의-권장-순서"></a>
- [처음-읽을-때의-권장-순서](docs/project/chapters/01-introduction.ko.md#처음-읽을-때의-권장-순서)

<a id="빠른-안내--현재-우선순위와-문서-읽기"></a>
- [빠른-안내--현재-우선순위와-문서-읽기](docs/project/chapters/01-introduction.ko.md#빠른-안내--현재-우선순위와-문서-읽기)

<a id="read-goal"></a>
- [read-goal](docs/project/chapters/02-project-goal.ko.md#read-goal)

<a id="1-프로젝트-목적"></a>
- [1-프로젝트-목적](docs/project/chapters/02-project-goal.ko.md#1-프로젝트-목적)

<a id="12-핵심-배열-모델-결정--logical-array와-physical-array를-분리한다"></a>
- [12-핵심-배열-모델-결정--logical-array와-physical-array를-분리한다](docs/project/chapters/02-project-goal.ko.md#12-핵심-배열-모델-결정--logical-array와-physical-array를-분리한다)

<a id="13-이름-정책"></a>
- [13-이름-정책](docs/project/chapters/02-project-goal.ko.md#13-이름-정책)

<a id="read-architecture"></a>
- [read-architecture](docs/project/chapters/03-architecture-p01.ko.md#read-architecture)

<a id="2-최상위-아키텍처"></a>
- [2-최상위-아키텍처](docs/project/chapters/03-architecture-p01.ko.md#2-최상위-아키텍처)

<a id="frontend-결과물과-j-graph-ir-결과물의-차이"></a>
- [frontend-결과물과-j-graph-ir-결과물의-차이](docs/project/chapters/03-architecture-p01.ko.md#frontend-결과물과-j-graph-ir-결과물의-차이)

<a id="기억용-4단계-요약--상세-구현에서-다시-확인할-경계"></a>
- [기억용-4단계-요약--상세-구현에서-다시-확인할-경계](docs/project/chapters/03-architecture-p01.ko.md#기억용-4단계-요약--상세-구현에서-다시-확인할-경계)

<a id="21-rustj-compiler-stage의-위상"></a>
- [21-rustj-compiler-stage의-위상](docs/project/chapters/03-architecture-p01.ko.md#21-rustj-compiler-stage의-위상)

<a id="211-두-compiler-ir은-목적이-다르며-둘-다-canonical-boundary다"></a>
- [211-두-compiler-ir은-목적이-다르며-둘-다-canonical-boundary다](docs/project/chapters/03-architecture-p01.ko.md#211-두-compiler-ir은-목적이-다르며-둘-다-canonical-boundary다)

<a id="a-j-graph-ir--이-배열-계산은-j-대수로-어떤-graph인가"></a>
- [a-j-graph-ir--이-배열-계산은-j-대수로-어떤-graph인가](docs/project/chapters/03-architecture-p01.ko.md#a-j-graph-ir--이-배열-계산은-j-대수로-어떤-graph인가)

<a id="b-logical-execution-ir--이-j-graph를-정확히-실행하려면-어떤-operationdata-dependency가-필요한가"></a>
- [b-logical-execution-ir--이-j-graph를-정확히-실행하려면-어떤-operationdata-dependency가-필요한가](docs/project/chapters/03-architecture-p01.ko.md#b-logical-execution-ir--이-j-graph를-정확히-실행하려면-어떤-operationdata-dependency가-필요한가)

<a id="두-ir-사이의-최적화-책임"></a>
- [두-ir-사이의-최적화-책임](docs/project/chapters/03-architecture-p01.ko.md#두-ir-사이의-최적화-책임)

<a id="22-물리적으로-함께-논리적으로-독립"></a>
- [22-물리적으로-함께-논리적으로-독립](docs/project/chapters/03-architecture-p01.ko.md#22-물리적으로-함께-논리적으로-독립)

<a id="23-logical-ir은-실행-backend를-강제하지-않는다"></a>
- [23-logical-ir은-실행-backend를-강제하지-않는다](docs/project/chapters/03-architecture-p01.ko.md#23-logical-ir은-실행-backend를-강제하지-않는다)

<a id="24-ir-export-adapter의-계약"></a>
- [24-ir-export-adapter의-계약](docs/project/chapters/03-architecture-p01.ko.md#24-ir-export-adapter의-계약)

<a id="241-semantic-operation과-architecture-specific-realization을-분리한다"></a>
- [241-semantic-operation과-architecture-specific-realization을-분리한다](docs/project/chapters/03-architecture-p01.ko.md#241-semantic-operation과-architecture-specific-realization을-분리한다)

<a id="sw-mlpl-reference-checklist"></a>
- [sw-mlpl-reference-checklist](docs/project/chapters/03-architecture-p02.ko.md#sw-mlpl-reference-checklist)

<a id="242-j-semantics-보존과-compiler-fact-축적-원칙"></a>
- [242-j-semantics-보존과-compiler-fact-축적-원칙](docs/project/chapters/03-architecture-p02.ko.md#242-j-semantics-보존과-compiler-fact-축적-원칙)

<a id="25-route-partition은-whole-program-exclusive-choice가-아니다"></a>
- [25-route-partition은-whole-program-exclusive-choice가-아니다](docs/project/chapters/03-architecture-p02.ko.md#25-route-partition은-whole-program-exclusive-choice가-아니다)

<a id="251-routeregion-boundary-contract"></a>
- [251-routeregion-boundary-contract](docs/project/chapters/03-architecture-p02.ko.md#251-routeregion-boundary-contract)

<a id="boundary-legality"></a>
- [boundary-legality](docs/project/chapters/03-architecture-p02.ko.md#boundary-legality)

<a id="live-out은-value만이-아니다"></a>
- [live-out은-value만이-아니다](docs/project/chapters/03-architecture-p02.ko.md#live-out은-value만이-아니다)

<a id="bridge-lowering"></a>
- [bridge-lowering](docs/project/chapters/03-architecture-p02.ko.md#bridge-lowering)

<a id="현재-코드와-목표의-차이"></a>
- [현재-코드와-목표의-차이](docs/project/chapters/03-architecture-p02.ko.md#현재-코드와-목표의-차이)

<a id="구현검증-순서"></a>
- [구현검증-순서](docs/project/chapters/03-architecture-p02.ko.md#구현검증-순서)

<a id="m3-rb--a3routeboundary-의미-보존-증명-및-검증-계약-2026-10-07-문서-설계만"></a>
- [m3-rb--a3routeboundary-의미-보존-증명-및-검증-계약-2026-10-07-문서-설계만](docs/project/chapters/03-architecture-p02.ko.md#m3-rb--a3routeboundary-의미-보존-증명-및-검증-계약-2026-10-07-문서-설계만)

<a id="외부-구현-사례-sw-mlpl--rustj-적용검증-체크리스트-2026-10-08-설계-조사만"></a>
- [외부-구현-사례-sw-mlpl--rustj-적용검증-체크리스트-2026-10-08-설계-조사만](docs/project/chapters/03-architecture-p02.ko.md#외부-구현-사례-sw-mlpl--rustj-적용검증-체크리스트-2026-10-08-설계-조사만)

<a id="read-semantic"></a>
- [read-semantic](docs/project/chapters/04-semantic-array-ir-p01.ko.md#read-semantic)

<a id="3-j-semantic-array-ir"></a>
- [3-j-semantic-array-ir](docs/project/chapters/04-semantic-array-ir-p01.ko.md#3-j-semantic-array-ir)

<a id="31-j의-계산-모델"></a>
- [31-j의-계산-모델](docs/project/chapters/04-semantic-array-ir-p01.ko.md#31-j의-계산-모델)

<a id="32-jsource에서-확인한-근거"></a>
- [32-jsource에서-확인한-근거](docs/project/chapters/04-semantic-array-ir-p01.ko.md#32-jsource에서-확인한-근거)

<a id="33-ir이-보존해야-하는-구조"></a>
- [33-ir이-보존해야-하는-구조](docs/project/chapters/04-semantic-array-ir-p01.ko.md#33-ir이-보존해야-하는-구조)

<a id="331-derived-construction의-결과는-function으로-한정하지-않는다"></a>
- [331-derived-construction의-결과는-function으로-한정하지-않는다](docs/project/chapters/04-semantic-array-ir-p01.ko.md#331-derived-construction의-결과는-function으로-한정하지-않는다)

<a id="331a-gerund는-새-atom-type이-아니라-contextually-interpreted-boxed-noun이다"></a>
- [331a-gerund는-새-atom-type이-아니라-contextually-interpreted-boxed-noun이다](docs/project/chapters/04-semantic-array-ir-p01.ko.md#331a-gerund는-새-atom-type이-아니라-contextually-interpreted-boxed-noun이다)

<a id="332-frontend-전체를-jsource-compatible-pipeline으로-유지한다"></a>
- [332-frontend-전체를-jsource-compatible-pipeline으로-유지한다](docs/project/chapters/04-semantic-array-ir-p01.ko.md#332-frontend-전체를-jsource-compatible-pipeline으로-유지한다)

<a id="enqueuer의-primitive-resolution-hook"></a>
- [enqueuer의-primitive-resolution-hook](docs/project/chapters/04-semantic-array-ir-p01.ko.md#enqueuer의-primitive-resolution-hook)

<a id="3321-frontend-compatibility-invariant"></a>
- [3321-frontend-compatibility-invariant](docs/project/chapters/04-semantic-array-ir-p01.ko.md#3321-frontend-compatibility-invariant)

<a id="3322-canonical-frontend-sentence-trace---y"></a>
- [3322-canonical-frontend-sentence-trace---y](docs/project/chapters/04-semantic-array-ir-p01.ko.md#3322-canonical-frontend-sentence-trace---y)

<a id="3323-compilerinterpreterjit-공통-structured-diagnostics"></a>
- [3323-compilerinterpreterjit-공통-structured-diagnostics](docs/project/chapters/04-semantic-array-ir-p01.ko.md#3323-compilerinterpreterjit-공통-structured-diagnostics)

<a id="333-parser-language-rules-are-jsource-compatible"></a>
- [333-parser-language-rules-are-jsource-compatible](docs/project/chapters/04-semantic-array-ir-p01.ko.md#333-parser-language-rules-are-jsource-compatible)

<a id="334-function-dag의-shape는-j-parser-reduction-rule이-결정한다"></a>
- [334-function-dag의-shape는-j-parser-reduction-rule이-결정한다](docs/project/chapters/04-semantic-array-ir-p02.ko.md#334-function-dag의-shape는-j-parser-reduction-rule이-결정한다)

<a id="335-jsource-vfgh는-parse-dag의-oracle이-아니라-realization-cross-check다"></a>
- [335-jsource-vfgh는-parse-dag의-oracle이-아니라-realization-cross-check다](docs/project/chapters/04-semantic-array-ir-p02.ko.md#335-jsource-vfgh는-parse-dag의-oracle이-아니라-realization-cross-check다)

<a id="336-modifier-reduction-결과는-다음-parser-reduction에서-하나의-function-entity다"></a>
- [336-modifier-reduction-결과는-다음-parser-reduction에서-하나의-function-entity다](docs/project/chapters/04-semantic-array-ir-p02.ko.md#336-modifier-reduction-결과는-다음-parser-reduction에서-하나의-function-entity다)

<a id="34-너무-이른-정규화를-금지한다"></a>
- [34-너무-이른-정규화를-금지한다](docs/project/chapters/04-semantic-array-ir-p02.ko.md#34-너무-이른-정규화를-금지한다)

<a id="341-train-dataflow와-observable-execution-order를-분리한다"></a>
- [341-train-dataflow와-observable-execution-order를-분리한다](docs/project/chapters/04-semantic-array-ir-p02.ko.md#341-train-dataflow와-observable-execution-order를-분리한다)

<a id="342-derived-verb의-latent-semantics를-forward-dataflow로-소거하지-않는다"></a>
- [342-derived-verb의-latent-semantics를-forward-dataflow로-소거하지-않는다](docs/project/chapters/04-semantic-array-ir-p02.ko.md#342-derived-verb의-latent-semantics를-forward-dataflow로-소거하지-않는다)

<a id="35-물리-정보는-넣지-않는다"></a>
- [35-물리-정보는-넣지-않는다](docs/project/chapters/04-semantic-array-ir-p02.ko.md#35-물리-정보는-넣지-않는다)

<a id="36-현재-구현과-목표-경계의-차이"></a>
- [36-현재-구현과-목표-경계의-차이](docs/project/chapters/04-semantic-array-ir-p02.ko.md#36-현재-구현과-목표-경계의-차이)

<a id="37-name-resolution과-explicit-definition의-호출-의미"></a>
- [37-name-resolution과-explicit-definition의-호출-의미](docs/project/chapters/04-semantic-array-ir-p02.ko.md#37-name-resolution과-explicit-definition의-호출-의미)

<a id="371-explicit-definition-control-flow-handoff-example"></a>
- [371-explicit-definition-control-flow-handoff-example](docs/project/chapters/04-semantic-array-ir-p03.ko.md#371-explicit-definition-control-flow-handoff-example)

<a id="372-jsource-name-정합성-감사-및-f1p4-수용-조건-2026-10-07-설계코드-조사"></a>
- [372-jsource-name-정합성-감사-및-f1p4-수용-조건-2026-10-07-설계코드-조사](docs/project/chapters/04-semantic-array-ir-p03.ko.md#372-jsource-name-정합성-감사-및-f1p4-수용-조건-2026-10-07-설계코드-조사)

<a id="373-name-의미론적-조회와-최적화-provenance-분리-2026-10-07-설계-전용"></a>
- [373-name-의미론적-조회와-최적화-provenance-분리-2026-10-07-설계-전용](docs/project/chapters/04-semantic-array-ir-p03.ko.md#373-name-의미론적-조회와-최적화-provenance-분리-2026-10-07-설계-전용)

<a id="374-name-최적화-캐시와-멀티프로세서-인계-계약-2026-10-07-설계-전용"></a>
- [374-name-최적화-캐시와-멀티프로세서-인계-계약-2026-10-07-설계-전용](docs/project/chapters/04-semantic-array-ir-p03.ko.md#374-name-최적화-캐시와-멀티프로세서-인계-계약-2026-10-07-설계-전용)

<a id="375-추가-배열-compiler-비교와-name-호환성-재검토-2026-10-07"></a>
- [375-추가-배열-compiler-비교와-name-호환성-재검토-2026-10-07](docs/project/chapters/04-semantic-array-ir-p03.ko.md#375-추가-배열-compiler-비교와-name-호환성-재검토-2026-10-07)

<a id="376-parsercompiler의-구체-전달-계약-지연-noun과-concrete-jentity-2026-10-07"></a>
- [376-parsercompiler의-구체-전달-계약-지연-noun과-concrete-jentity-2026-10-07](docs/project/chapters/04-semantic-array-ir-p03.ko.md#376-parsercompiler의-구체-전달-계약-지연-noun과-concrete-jentity-2026-10-07)

<a id="377-semantichandoff-v1-논리-데이터-구조-확정-2026-10-07-논리-계약첫-구현"></a>
- [377-semantichandoff-v1-논리-데이터-구조-확정-2026-10-07-논리-계약첫-구현](docs/project/chapters/04-semantic-array-ir-p03.ko.md#377-semantichandoff-v1-논리-데이터-구조-확정-2026-10-07-논리-계약첫-구현)

<a id="dynamic-semantic-boundaries"></a>
- [dynamic-semantic-boundaries](docs/project/chapters/04-semantic-array-ir-p04.ko.md#dynamic-semantic-boundaries)

<a id="38-sentence-evaluation-order와-namespace-mutation"></a>
- [38-sentence-evaluation-order와-namespace-mutation](docs/project/chapters/04-semantic-array-ir-p04.ko.md#38-sentence-evaluation-order와-namespace-mutation)

<a id="39-동적-의미와-컴파일-경계-계약"></a>
- [39-동적-의미와-컴파일-경계-계약](docs/project/chapters/04-semantic-array-ir-p04.ko.md#39-동적-의미와-컴파일-경계-계약)

<a id="391-경계별-필요한-근거와-처리"></a>
- [391-경계별-필요한-근거와-처리](docs/project/chapters/04-semantic-array-ir-p04.ko.md#391-경계별-필요한-근거와-처리)

<a id="392-허가하지-않는-변환과-guard-실패-계약"></a>
- [392-허가하지-않는-변환과-guard-실패-계약](docs/project/chapters/04-semantic-array-ir-p04.ko.md#392-허가하지-않는-변환과-guard-실패-계약)

<a id="393-unknown-분류와-구현-경계"></a>
- [393-unknown-분류와-구현-경계](docs/project/chapters/04-semantic-array-ir-p04.ko.md#393-unknown-분류와-구현-경계)

<a id="394-fallback--guard-miss--replay-decision-table"></a>
- [394-fallback--guard-miss--replay-decision-table](docs/project/chapters/04-semantic-array-ir-p04.ko.md#394-fallback--guard-miss--replay-decision-table)

<a id="runtimesemanticfallback의-정확한-뜻"></a>
- [runtimesemanticfallback의-정확한-뜻](docs/project/chapters/04-semantic-array-ir-p04.ko.md#runtimesemanticfallback의-정확한-뜻)

<a id="fallback-commit-frontier"></a>
- [fallback-commit-frontier](docs/project/chapters/04-semantic-array-ir-p04.ko.md#fallback-commit-frontier)

<a id="검증-요구"></a>
- [검증-요구](docs/project/chapters/04-semantic-array-ir-p04.ko.md#검증-요구)

<a id="395-경계-판정-알고리즘과-검증-수용-조건--첫-cpu-route-2026-10-07"></a>
- [395-경계-판정-알고리즘과-검증-수용-조건--첫-cpu-route-2026-10-07](docs/project/chapters/04-semantic-array-ir-p04.ko.md#395-경계-판정-알고리즘과-검증-수용-조건--첫-cpu-route-2026-10-07)

<a id="read-graph-execution"></a>
- [read-graph-execution](docs/project/chapters/05-graph-execution-lowering-p01.ko.md#read-graph-execution)

<a id="j-graph-opportunities"></a>
- [j-graph-opportunities](docs/project/chapters/05-graph-execution-lowering-p01.ko.md#j-graph-opportunities)

<a id="syntax-graph-hints"></a>
- [syntax-graph-hints](docs/project/chapters/05-graph-execution-lowering-p01.ko.md#syntax-graph-hints)

<a id="4-j-graph-analyzer와-execution-semantic-lowering"></a>
- [4-j-graph-analyzer와-execution-semantic-lowering](docs/project/chapters/05-graph-execution-lowering-p01.ko.md#4-j-graph-analyzer와-execution-semantic-lowering)

<a id="41-j-graph-analyzer의-책임"></a>
- [41-j-graph-analyzer의-책임](docs/project/chapters/05-graph-execution-lowering-p01.ko.md#41-j-graph-analyzer의-책임)

<a id="410-canonical-j-graph-example-suite"></a>
- [410-canonical-j-graph-example-suite](docs/project/chapters/05-graph-execution-lowering-p01.ko.md#410-canonical-j-graph-example-suite)

<a id="411-j-syntax-derived-structural-opportunity-ir-문법을-optimization-information-source로-사용한다"></a>
- [411-j-syntax-derived-structural-opportunity-ir-문법을-optimization-information-source로-사용한다](docs/project/chapters/05-graph-execution-lowering-p01.ko.md#411-j-syntax-derived-structural-opportunity-ir-문법을-optimization-information-source로-사용한다)

<a id="jaxa에서-계승하는-첫-세-topology"></a>
- [jaxa에서-계승하는-첫-세-topology](docs/project/chapters/05-graph-execution-lowering-p01.ko.md#jaxa에서-계승하는-첫-세-topology)

<a id="opportunity와-legality를-분리한다"></a>
- [opportunity와-legality를-분리한다](docs/project/chapters/05-graph-execution-lowering-p01.ko.md#opportunity와-legality를-분리한다)

<a id="flowstorage와의-연결"></a>
- [flowstorage와의-연결](docs/project/chapters/05-graph-execution-lowering-p01.ko.md#flowstorage와의-연결)

<a id="현재-구현-경계의-정본"></a>
- [현재-구현-경계의-정본](docs/project/chapters/05-graph-execution-lowering-p01.ko.md#현재-구현-경계의-정본)

<a id="중요한-불변조건"></a>
- [중요한-불변조건](docs/project/chapters/05-graph-execution-lowering-p01.ko.md#중요한-불변조건)

<a id="412-j-syntax에서-얻는-graph-최적화-힌트-분류"></a>
- [412-j-syntax에서-얻는-graph-최적화-힌트-분류](docs/project/chapters/05-graph-execution-lowering-p01.ko.md#412-j-syntax에서-얻는-graph-최적화-힌트-분류)

<a id="4121-먼저-바로잡는-기준-fork-구조-자체가-병렬-실행-후보를-드러낸다"></a>
- [4121-먼저-바로잡는-기준-fork-구조-자체가-병렬-실행-후보를-드러낸다](docs/project/chapters/05-graph-execution-lowering-p01.ko.md#4121-먼저-바로잡는-기준-fork-구조-자체가-병렬-실행-후보를-드러낸다)

<a id="4122-먼저-분류-graph에서-읽을-수-있는-일반-컴파일러-정보"></a>
- [4122-먼저-분류-graph에서-읽을-수-있는-일반-컴파일러-정보](docs/project/chapters/05-graph-execution-lowering-p01.ko.md#4122-먼저-분류-graph에서-읽을-수-있는-일반-컴파일러-정보)

<a id="4123-syntax-자체에서-얻는-힌트-leaf-verb를-black-box로-둔-검토"></a>
- [4123-syntax-자체에서-얻는-힌트-leaf-verb를-black-box로-둔-검토](docs/project/chapters/05-graph-execution-lowering-p01.ko.md#4123-syntax-자체에서-얻는-힌트-leaf-verb를-black-box로-둔-검토)

<a id="41231-구문별로-후보와-허가를-분리한다"></a>
- [41231-구문별로-후보와-허가를-분리한다](docs/project/chapters/05-graph-execution-lowering-p01.ko.md#41231-구문별로-후보와-허가를-분리한다)

<a id="4124-leaf-primitive-힌트와-섞지-않을-modifier-구조"></a>
- [4124-leaf-primitive-힌트와-섞지-않을-modifier-구조](docs/project/chapters/05-graph-execution-lowering-p01.ko.md#4124-leaf-primitive-힌트와-섞지-않을-modifier-구조)

<a id="4125-binding-정보와-일반-graph-대비-추가-가치"></a>
- [4125-binding-정보와-일반-graph-대비-추가-가치](docs/project/chapters/05-graph-execution-lowering-p01.ko.md#4125-binding-정보와-일반-graph-대비-추가-가치)

<a id="framework-graph-hints"></a>
- [framework-graph-hints](docs/project/chapters/05-graph-execution-lowering-p02.ko.md#framework-graph-hints)

<a id="4126-다른-배열-컴파일-프레임워크와의-대조"></a>
- [4126-다른-배열-컴파일-프레임워크와의-대조](docs/project/chapters/05-graph-execution-lowering-p02.ko.md#4126-다른-배열-컴파일-프레임워크와의-대조)

<a id="4127-rustj에-반영할-설계와-검증-체크리스트"></a>
- [4127-rustj에-반영할-설계와-검증-체크리스트](docs/project/chapters/05-graph-execution-lowering-p02.ko.md#4127-rustj에-반영할-설계와-검증-체크리스트)

<a id="414-candidate-lifecycle와-proof-discharge-contract"></a>
- [414-candidate-lifecycle와-proof-discharge-contract](docs/project/chapters/05-graph-execution-lowering-p02.ko.md#414-candidate-lifecycle와-proof-discharge-contract)

<a id="evidence-owner"></a>
- [evidence-owner](docs/project/chapters/05-graph-execution-lowering-p02.ko.md#evidence-owner)

<a id="guarded-legality"></a>
- [guarded-legality](docs/project/chapters/05-graph-execution-lowering-p02.ko.md#guarded-legality)

<a id="candidate-overlap과-selection"></a>
- [candidate-overlap과-selection](docs/project/chapters/05-graph-execution-lowering-p02.ko.md#candidate-overlap과-selection)

<a id="현재-코드와-목표-계약의-대응"></a>
- [현재-코드와-목표-계약의-대응](docs/project/chapters/05-graph-execution-lowering-p02.ko.md#현재-코드와-목표-계약의-대응)

<a id="구현-시-분리-순서"></a>
- [구현-시-분리-순서](docs/project/chapters/05-graph-execution-lowering-p02.ko.md#구현-시-분리-순서)

<a id="42-execution-semantic-lowering의-책임"></a>
- [42-execution-semantic-lowering의-책임](docs/project/chapters/05-graph-execution-lowering-p02.ko.md#42-execution-semantic-lowering의-책임)

<a id="43-두-middle-end가-공통으로-하지-않는-일"></a>
- [43-두-middle-end가-공통으로-하지-않는-일](docs/project/chapters/05-graph-execution-lowering-p02.ko.md#43-두-middle-end가-공통으로-하지-않는-일)

<a id="44-execution-semantic-lowering-이후의-generic-경계"></a>
- [44-execution-semantic-lowering-이후의-generic-경계](docs/project/chapters/05-graph-execution-lowering-p02.ko.md#44-execution-semantic-lowering-이후의-generic-경계)

<a id="441-분석-fact는-typed-lattice로-관리하고-semantic-error와-분리한다"></a>
- [441-분석-fact는-typed-lattice로-관리하고-semantic-error와-분리한다](docs/project/chapters/05-graph-execution-lowering-p02.ko.md#441-분석-fact는-typed-lattice로-관리하고-semantic-error와-분리한다)

<a id="read-physical"></a>
- [read-physical](docs/project/chapters/06-plans-executor-p01.ko.md#read-physical)

<a id="5-logical-plan-physical-plan-executor"></a>
- [5-logical-plan-physical-plan-executor](docs/project/chapters/06-plans-executor-p01.ko.md#5-logical-plan-physical-plan-executor)

<a id="51-logical-array-ir--logical-execution-plan"></a>
- [51-logical-array-ir--logical-execution-plan](docs/project/chapters/06-plans-executor-p01.ko.md#51-logical-array-ir--logical-execution-plan)

<a id="52-schedule--transform-plan과-physical-plan을-구분한다"></a>
- [52-schedule--transform-plan과-physical-plan을-구분한다](docs/project/chapters/06-plans-executor-p01.ko.md#52-schedule--transform-plan과-physical-plan을-구분한다)

<a id="521-m4-최소-physicalplan-v0-contract"></a>
- [521-m4-최소-physicalplan-v0-contract](docs/project/chapters/06-plans-executor-p01.ko.md#521-m4-최소-physicalplan-v0-contract)

<a id="v0-physicalplan"></a>
- [v0-physicalplan](docs/project/chapters/06-plans-executor-p01.ko.md#v0-physicalplan)

<a id="planned-buffer와-lifetime"></a>
- [planned-buffer와-lifetime](docs/project/chapters/06-plans-executor-p01.ko.md#planned-buffer와-lifetime)

<a id="physicalplan-verifier"></a>
- [physicalplan-verifier](docs/project/chapters/06-plans-executor-p01.ko.md#physicalplan-verifier)

<a id="error--cleanup-contract"></a>
- [error--cleanup-contract](docs/project/chapters/06-plans-executor-p01.ko.md#error--cleanup-contract)

<a id="m4m5-구현-선행조건과-검증-프로토콜-2026-10-07-설계만-확정"></a>
- [m4m5-구현-선행조건과-검증-프로토콜-2026-10-07-설계만-확정](docs/project/chapters/06-plans-executor-p01.ko.md#m4m5-구현-선행조건과-검증-프로토콜-2026-10-07-설계만-확정)

<a id="기존-아키텍처-대조-감사-m4-준비도와-검증-공백-2026-10-07-설계-검증-전용"></a>
- [기존-아키텍처-대조-감사-m4-준비도와-검증-공백-2026-10-07-설계-검증-전용](docs/project/chapters/06-plans-executor-p01.ko.md#기존-아키텍처-대조-감사-m4-준비도와-검증-공백-2026-10-07-설계-검증-전용)

<a id="m3m4-route-to-physical-인계-계약-2026-10-07-설계만-완료--구현-없음"></a>
- [m3m4-route-to-physical-인계-계약-2026-10-07-설계만-완료--구현-없음](docs/project/chapters/06-plans-executor-p01.ko.md#m3m4-route-to-physical-인계-계약-2026-10-07-설계만-완료--구현-없음)

<a id="h-k--a3-opkind별-m3m4-인계-규칙-2026-10-07-설계만"></a>
- [h-k--a3-opkind별-m3m4-인계-규칙-2026-10-07-설계만](docs/project/chapters/06-plans-executor-p01.ko.md#h-k--a3-opkind별-m3m4-인계-규칙-2026-10-07-설계만)

<a id="h-a--m3m4-첫-cpu-경로-실행-승인-판정표-2026-10-07-명세만"></a>
- [h-a--m3m4-첫-cpu-경로-실행-승인-판정표-2026-10-07-명세만](docs/project/chapters/06-plans-executor-p01.ko.md#h-a--m3m4-첫-cpu-경로-실행-승인-판정표-2026-10-07-명세만)

<a id="h-p--m3m4-구현-인수-패킷과-물리-검증-체크리스트-계약-2026-10-07-문서-전용"></a>
- [h-p--m3m4-구현-인수-패킷과-물리-검증-체크리스트-계약-2026-10-07-문서-전용](docs/project/chapters/06-plans-executor-p02.ko.md#h-p--m3m4-구현-인수-패킷과-물리-검증-체크리스트-계약-2026-10-07-문서-전용)

<a id="현재-코드와의-대응"></a>
- [현재-코드와의-대응](docs/project/chapters/06-plans-executor-p02.ko.md#현재-코드와의-대응)

<a id="구현-순서--한-번에-한-의미--한-verifiertest"></a>
- [구현-순서--한-번에-한-의미--한-verifiertest](docs/project/chapters/06-plans-executor-p02.ko.md#구현-순서--한-번에-한-의미--한-verifiertest)

<a id="53-rustj-native-executor"></a>
- [53-rustj-native-executor](docs/project/chapters/06-plans-executor-p02.ko.md#53-rustj-native-executor)

<a id="54-실행-경로는-하나가-아니다"></a>
- [54-실행-경로는-하나가-아니다](docs/project/chapters/06-plans-executor-p02.ko.md#54-실행-경로는-하나가-아니다)

<a id="route-a--rustj-native"></a>
- [route-a--rustj-native](docs/project/chapters/06-plans-executor-p02.ko.md#route-a--rustj-native)

<a id="route-b--mlir"></a>
- [route-b--mlir](docs/project/chapters/06-plans-executor-p02.ko.md#route-b--mlir)

<a id="route-c--stablehlo--openxla-compatible-subset"></a>
- [route-c--stablehlo--openxla-compatible-subset](docs/project/chapters/06-plans-executor-p02.ko.md#route-c--stablehlo--openxla-compatible-subset)

<a id="route-d--direct-external-librarykernel"></a>
- [route-d--direct-external-librarykernel](docs/project/chapters/06-plans-executor-p02.ko.md#route-d--direct-external-librarykernel)

<a id="55-외부-ir을-사용할-때-rustj가-끝까지-책임지는-것"></a>
- [55-외부-ir을-사용할-때-rustj가-끝까지-책임지는-것](docs/project/chapters/06-plans-executor-p02.ko.md#55-외부-ir을-사용할-때-rustj가-끝까지-책임지는-것)

<a id="551-external-adapter-boundary-contract"></a>
- [551-external-adapter-boundary-contract](docs/project/chapters/06-plans-executor-p02.ko.md#551-external-adapter-boundary-contract)

<a id="semanticcheck--error-mapping"></a>
- [semanticcheck--error-mapping](docs/project/chapters/06-plans-executor-p02.ko.md#semanticcheck--error-mapping)

<a id="effect--token-mapping"></a>
- [effect--token-mapping](docs/project/chapters/06-plans-executor-p02.ko.md#effect--token-mapping)

<a id="representation-bridge--ownership"></a>
- [representation-bridge--ownership](docs/project/chapters/06-plans-executor-p02.ko.md#representation-bridge--ownership)

<a id="round-trip-verifier"></a>
- [round-trip-verifier](docs/project/chapters/06-plans-executor-p02.ko.md#round-trip-verifier)

<a id="failure-classes"></a>
- [failure-classes](docs/project/chapters/06-plans-executor-p02.ko.md#failure-classes)

<a id="첫-adapter-구현-gate"></a>
- [첫-adapter-구현-gate](docs/project/chapters/06-plans-executor-p02.ko.md#첫-adapter-구현-gate)

<a id="56-외부-ir-선택-원칙"></a>
- [56-외부-ir-선택-원칙](docs/project/chapters/06-plans-executor-p02.ko.md#56-외부-ir-선택-원칙)

<a id="57-logical-ir은-verifierinterfaceversion-경계를-가진다"></a>
- [57-logical-ir은-verifierinterfaceversion-경계를-가진다](docs/project/chapters/06-plans-executor-p02.ko.md#57-logical-ir은-verifierinterfaceversion-경계를-가진다)

<a id="571-verifier"></a>
- [571-verifier](docs/project/chapters/06-plans-executor-p02.ko.md#571-verifier)

<a id="572-capability-interfaces"></a>
- [572-capability-interfaces](docs/project/chapters/06-plans-executor-p02.ko.md#572-capability-interfaces)

<a id="573-canonicalization과-rewrite-provenance"></a>
- [573-canonicalization과-rewrite-provenance](docs/project/chapters/06-plans-executor-p02.ko.md#573-canonicalization과-rewrite-provenance)

<a id="574-ir-serializationversioning"></a>
- [574-ir-serializationversioning](docs/project/chapters/06-plans-executor-p02.ko.md#574-ir-serializationversioning)

<a id="575-cross-stage-negative-verifier-matrix"></a>
- [575-cross-stage-negative-verifier-matrix](docs/project/chapters/06-plans-executor-p02.ko.md#575-cross-stage-negative-verifier-matrix)

<a id="576-serialization--schema-migration-policy"></a>
- [576-serialization--schema-migration-policy](docs/project/chapters/06-plans-executor-p02.ko.md#576-serialization--schema-migration-policy)

<a id="logical-physical-array-model"></a>
- [logical-physical-array-model](docs/project/chapters/07-array-model.ko.md#logical-physical-array-model)

<a id="6-논리-배열과-물리-배열--핵심-architecture-decision"></a>
- [6-논리-배열과-물리-배열--핵심-architecture-decision](docs/project/chapters/07-array-model.ko.md#6-논리-배열과-물리-배열--핵심-architecture-decision)

<a id="61-논리-j-noun-boxed-sparse와-verb"></a>
- [61-논리-j-noun-boxed-sparse와-verb](docs/project/chapters/07-array-model.ko.md#61-논리-j-noun-boxed-sparse와-verb)

<a id="62-valueid와-bufferid"></a>
- [62-valueid와-bufferid](docs/project/chapters/07-array-model.ko.md#62-valueid와-bufferid)

<a id="63-physicalarray"></a>
- [63-physicalarray](docs/project/chapters/07-array-model.ko.md#63-physicalarray)

<a id="64-cpustorage의-위치"></a>
- [64-cpustorage의-위치](docs/project/chapters/07-array-model.ko.md#64-cpustorage의-위치)

<a id="641-현재-구현-상태와-완료-조건"></a>
- [641-현재-구현-상태와-완료-조건](docs/project/chapters/07-array-model.ko.md#641-현재-구현-상태와-완료-조건)

<a id="7-gpu-친화적-배열-설계"></a>
- [7-gpu-친화적-배열-설계](docs/project/chapters/08-gpu.ko.md#7-gpu-친화적-배열-설계)

<a id="71-stride--offset-view"></a>
- [71-stride--offset-view](docs/project/chapters/08-gpu.ko.md#71-stride--offset-view)

<a id="72-논리-순서와-메모리-연속성"></a>
- [72-논리-순서와-메모리-연속성](docs/project/chapters/08-gpu.ko.md#72-논리-순서와-메모리-연속성)

<a id="73-zero-stride"></a>
- [73-zero-stride](docs/project/chapters/08-gpu.ko.md#73-zero-stride)

<a id="74-semantic-representation과-physical-encoding을-구분한다"></a>
- [74-semantic-representation과-physical-encoding을-구분한다](docs/project/chapters/08-gpu.ko.md#74-semantic-representation과-physical-encoding을-구분한다)

<a id="75-placement--sharding"></a>
- [75-placement--sharding](docs/project/chapters/08-gpu.ko.md#75-placement--sharding)

<a id="76-gpu-실행-원칙"></a>
- [76-gpu-실행-원칙](docs/project/chapters/08-gpu.ko.md#76-gpu-실행-원칙)

<a id="out-of-core-io-contract"></a>
- [out-of-core-io-contract](docs/project/chapters/09-memory-alias.ko.md#out-of-core-io-contract)

<a id="io-framework-execution-comparison"></a>
- [io-framework-execution-comparison](docs/project/chapters/09-memory-alias.ko.md#io-framework-execution-comparison)

<a id="unified-data-movement-contract"></a>
- [unified-data-movement-contract](docs/project/chapters/09-memory-alias.ko.md#unified-data-movement-contract)

<a id="io-a-source-audit"></a>
- [io-a-source-audit](docs/project/chapters/09-memory-alias.ko.md#io-a-source-audit)

<a id="io-static-source-contracts"></a>
- [io-static-source-contracts](docs/project/chapters/09-memory-alias.ko.md#io-static-source-contracts)

<a id="8-메모리alias수명-원칙"></a>
- [8-메모리alias수명-원칙](docs/project/chapters/09-memory-alias.ko.md#8-메모리alias수명-원칙)

<a id="81-읽기쓰기-분리"></a>
- [81-읽기쓰기-분리](docs/project/chapters/09-memory-alias.ko.md#81-읽기쓰기-분리)

<a id="82-주소-검증"></a>
- [82-주소-검증](docs/project/chapters/09-memory-alias.ko.md#82-주소-검증)

<a id="83-buffer-reuse"></a>
- [83-buffer-reuse](docs/project/chapters/09-memory-alias.ko.md#83-buffer-reuse)

<a id="84-관찰-가능한-j-의미"></a>
- [84-관찰-가능한-j-의미](docs/project/chapters/09-memory-alias.ko.md#84-관찰-가능한-j-의미)

<a id="85-느린-ioout-of-core-배열-실행-계약-2026-10-06-설계-후보--구현-미착수"></a>
- [85-느린-ioout-of-core-배열-실행-계약-2026-10-06-설계-후보--구현-미착수](docs/project/chapters/09-memory-alias.ko.md#85-느린-ioout-of-core-배열-실행-계약-2026-10-06-설계-후보--구현-미착수)

<a id="851-독립-재검토--j-라이브러리포맷모델-로더-보강-2026-10-06"></a>
- [851-독립-재검토--j-라이브러리포맷모델-로더-보강-2026-10-06](docs/project/chapters/09-memory-alias.ko.md#851-독립-재검토--j-라이브러리포맷모델-로더-보강-2026-10-06)

<a id="852-프레임워크별-io-최적화-실행-방식과-rustj-graph-ir-적용-2026-10-06"></a>
- [852-프레임워크별-io-최적화-실행-방식과-rustj-graph-ir-적용-2026-10-06](docs/project/chapters/09-memory-alias.ko.md#852-프레임워크별-io-최적화-실행-방식과-rustj-graph-ir-적용-2026-10-06)

<a id="853-메모리-io디스크-io-공통-planning-계약--신규-ir-계층은-당장-만들지-않음-2026-10-06"></a>
- [853-메모리-io디스크-io-공통-planning-계약--신규-ir-계층은-당장-만들지-않음-2026-10-06](docs/project/chapters/09-memory-alias.ko.md#853-메모리-io디스크-io-공통-planning-계약--신규-ir-계층은-당장-만들지-않음-2026-10-06)

<a id="854-io-a-원본-소스-교차-감사실행-기준-2026-10-0607"></a>
- [854-io-a-원본-소스-교차-감사실행-기준-2026-10-0607](docs/project/chapters/09-memory-alias.ko.md#854-io-a-원본-소스-교차-감사실행-기준-2026-10-0607)

<a id="855-정적-입력datasource-계약과-역방향-데이터-요구량-분석-2026-10-11-설계-후보--미구현"></a>
- [855-정적-입력datasource-계약과-역방향-데이터-요구량-분석-2026-10-11-설계-후보--미구현](docs/project/chapters/09-memory-alias.ko.md#855-정적-입력datasource-계약과-역방향-데이터-요구량-분석-2026-10-11-설계-후보--미구현)

<a id="read-scope"></a>
- [read-scope](docs/project/chapters/10-implementation-scope.ko.md#read-scope)

<a id="9-언어-및-구현-범위"></a>
- [9-언어-및-구현-범위](docs/project/chapters/10-implementation-scope.ko.md#9-언어-및-구현-범위)

<a id="91-현재-지원하는-주요-값"></a>
- [91-현재-지원하는-주요-값](docs/project/chapters/10-implementation-scope.ko.md#91-현재-지원하는-주요-값)

<a id="92-현재-직접-실행-경로에서-지원하는-대표-기능"></a>
- [92-현재-직접-실행-경로에서-지원하는-대표-기능](docs/project/chapters/10-implementation-scope.ko.md#92-현재-직접-실행-경로에서-지원하는-대표-기능)

<a id="93-배열-조작"></a>
- [93-배열-조작](docs/project/chapters/10-implementation-scope.ko.md#93-배열-조작)

<a id="94-indexsearch-계열"></a>
- [94-indexsearch-계열](docs/project/chapters/10-implementation-scope.ko.md#94-indexsearch-계열)

<a id="95-이름과-품사"></a>
- [95-이름과-품사](docs/project/chapters/10-implementation-scope.ko.md#95-이름과-품사)

<a id="96-direct--explicit-definition"></a>
- [96-direct--explicit-definition](docs/project/chapters/10-implementation-scope.ko.md#96-direct--explicit-definition)

<a id="97-아직-큰-미지원-영역"></a>
- [97-아직-큰-미지원-영역](docs/project/chapters/10-implementation-scope.ko.md#97-아직-큰-미지원-영역)

<a id="architecture-migration-checklist"></a>
- [architecture-migration-checklist](docs/project/chapters/11-roadmap-checklists-p01.ko.md#architecture-migration-checklist)

<a id="read-roadmap"></a>
- [read-roadmap](docs/project/chapters/11-roadmap-checklists-p01.ko.md#read-roadmap)

<a id="jaxa-future-work-traceability"></a>
- [jaxa-future-work-traceability](docs/project/chapters/11-roadmap-checklists-p01.ko.md#jaxa-future-work-traceability)

<a id="dynamic-boundary-checklist"></a>
- [dynamic-boundary-checklist](docs/project/chapters/11-roadmap-checklists-p01.ko.md#dynamic-boundary-checklist)

<a id="nv3a-spelling-errors"></a>
- [nv3a-spelling-errors](docs/project/chapters/11-roadmap-checklists-p01.ko.md#nv3a-spelling-errors)

<a id="nv3b-name-syntax"></a>
- [nv3b-name-syntax](docs/project/chapters/11-roadmap-checklists-p01.ko.md#nv3b-name-syntax)

<a id="nv3c-name-limits"></a>
- [nv3c-name-limits](docs/project/chapters/11-roadmap-checklists-p01.ko.md#nv3c-name-limits)

<a id="nv3d1-numeric-recognition"></a>
- [nv3d1-numeric-recognition](docs/project/chapters/11-roadmap-checklists-p01.ko.md#nv3d1-numeric-recognition)

<a id="nv3d2a-quad-hex-recognition"></a>
- [nv3d2a-quad-hex-recognition](docs/project/chapters/11-roadmap-checklists-p01.ko.md#nv3d2a-quad-hex-recognition)

<a id="nv3d2b1-rounding-nan-words"></a>
- [nv3d2b1-rounding-nan-words](docs/project/chapters/11-roadmap-checklists-p01.ko.md#nv3d2b1-rounding-nan-words)

<a id="nv3d2b2a-exact-hex-ratios"></a>
- [nv3d2b2a-exact-hex-ratios](docs/project/chapters/11-roadmap-checklists-p01.ko.md#nv3d2b2a-exact-hex-ratios)

<a id="vocabulary-migration-checklist"></a>
- [vocabulary-migration-checklist](docs/project/chapters/11-roadmap-checklists-p01.ko.md#vocabulary-migration-checklist)

<a id="heterogeneous-execution-checklist"></a>
- [heterogeneous-execution-checklist](docs/project/chapters/11-roadmap-checklists-p01.ko.md#heterogeneous-execution-checklist)

<a id="10-구현-계획과-체크리스트"></a>
- [10-구현-계획과-체크리스트](docs/project/chapters/11-roadmap-checklists-p01.ko.md#10-구현-계획과-체크리스트)

<a id="jfw--jaxa-future-work-10개-항목의-rustj-이행-추적표-2026-10-13-문서설계만"></a>
- [jfw--jaxa-future-work-10개-항목의-rustj-이행-추적표-2026-10-13-문서설계만](docs/project/chapters/11-roadmap-checklists-p01.ko.md#jfw--jaxa-future-work-10개-항목의-rustj-이행-추적표-2026-10-13-문서설계만)

<a id="db--동적-의미와-컴파일-경계-이행-db0db7"></a>
- [db--동적-의미와-컴파일-경계-이행-db0db7](docs/project/chapters/11-roadmap-checklists-p01.ko.md#db--동적-의미와-컴파일-경계-이행-db0db7)

<a id="nv--현재-j-vocabulary-수렴"></a>
- [nv--현재-j-vocabulary-수렴](docs/project/chapters/11-roadmap-checklists-p01.ko.md#nv--현재-j-vocabulary-수렴)

<a id="m--현재-구조-수렴-실행-순서"></a>
- [m--현재-구조-수렴-실행-순서](docs/project/chapters/11-roadmap-checklists-p01.ko.md#m--현재-구조-수렴-실행-순서)

<a id="m0--구조-기준선-고정"></a>
- [m0--구조-기준선-고정](docs/project/chapters/11-roadmap-checklists-p01.ko.md#m0--구조-기준선-고정)

<a id="m1--canonical-logical-execution-ir로-cutover--완료"></a>
- [m1--canonical-logical-execution-ir로-cutover--완료](docs/project/chapters/11-roadmap-checklists-p01.ko.md#m1--canonical-logical-execution-ir로-cutover--완료)

<a id="m2--jsource-compatible-frontendparser-cutover"></a>
- [m2--jsource-compatible-frontendparser-cutover](docs/project/chapters/11-roadmap-checklists-p01.ko.md#m2--jsource-compatible-frontendparser-cutover)

<a id="m3--logicalphysical-array-경계의-코드-수렴"></a>
- [m3--logicalphysical-array-경계의-코드-수렴](docs/project/chapters/11-roadmap-checklists-p01.ko.md#m3--logicalphysical-array-경계의-코드-수렴)

<a id="m4--최소-rustj-native-cpu-vertical-slice"></a>
- [m4--최소-rustj-native-cpu-vertical-slice](docs/project/chapters/11-roadmap-checklists-p01.ko.md#m4--최소-rustj-native-cpu-vertical-slice)

<a id="he--이종-cpugpu-실행-계획-수렴-2026-10-07-m4m6-연계-체크리스트"></a>
- [he--이종-cpugpu-실행-계획-수렴-2026-10-07-m4m6-연계-체크리스트](docs/project/chapters/11-roadmap-checklists-p01.ko.md#he--이종-cpugpu-실행-계획-수렴-2026-10-07-m4m6-연계-체크리스트)

<a id="m5--routeschedulecost-확장"></a>
- [m5--routeschedulecost-확장](docs/project/chapters/11-roadmap-checklists-p01.ko.md#m5--routeschedulecost-확장)

<a id="m6--externalarrayfiregpu-route"></a>
- [m6--externalarrayfiregpu-route](docs/project/chapters/11-roadmap-checklists-p01.ko.md#m6--externalarrayfiregpu-route)

<a id="je0je6--jentity--contextual-higher-order-view-보조-트랙"></a>
- [je0je6--jentity--contextual-higher-order-view-보조-트랙](docs/project/chapters/11-roadmap-checklists-p01.ko.md#je0je6--jentity--contextual-higher-order-view-보조-트랙)

<a id="je0--현행-semantic-carrier와-jsource-대응-감사"></a>
- [je0--현행-semantic-carrier와-jsource-대응-감사](docs/project/chapters/11-roadmap-checklists-p01.ko.md#je0--현행-semantic-carrier와-jsource-대응-감사)

<a id="je0-현행-carrier-감사와-첫-migration-seam-2026-10-04"></a>
- [je0-현행-carrier-감사와-첫-migration-seam-2026-10-04](docs/project/chapters/11-roadmap-checklists-p01.ko.md#je0-현행-carrier-감사와-첫-migration-seam-2026-10-04)

<a id="je1--최소-공통-jentity-identity-도입"></a>
- [je1--최소-공통-jentity-identity-도입](docs/project/chapters/11-roadmap-checklists-p01.ko.md#je1--최소-공통-jentity-identity-도입)

<a id="je1-구현--대입-경계의-최소-jentity-2026-10-04"></a>
- [je1-구현--대입-경계의-최소-jentity-2026-10-04](docs/project/chapters/11-roadmap-checklists-p01.ko.md#je1-구현--대입-경계의-최소-jentity-2026-10-04)

<a id="je2--parserbindingassignment-경계-수렴"></a>
- [je2--parserbindingassignment-경계-수렴](docs/project/chapters/11-roadmap-checklists-p01.ko.md#je2--parserbindingassignment-경계-수렴)

<a id="noun-reduction-capture"></a>
- [noun-reduction-capture](docs/project/chapters/11-roadmap-checklists-p02.ko.md#noun-reduction-capture)

<a id="je2-구현--namespace와-runtime-result의-jentity-수렴-2026-10-04-partial-je2"></a>
- [je2-구현--namespace와-runtime-result의-jentity-수렴-2026-10-04-partial-je2](docs/project/chapters/11-roadmap-checklists-p02.ko.md#je2-구현--namespace와-runtime-result의-jentity-수렴-2026-10-04-partial-je2)

<a id="je2-구현--provenance를-보존하는-borrowed-operand-view-2026-10-04-partial-je2"></a>
- [je2-구현--provenance를-보존하는-borrowed-operand-view-2026-10-04-partial-je2](docs/project/chapters/11-roadmap-checklists-p02.ko.md#je2-구현--provenance를-보존하는-borrowed-operand-view-2026-10-04-partial-je2)

<a id="je2-구현--completed-parser-result의-공통-transport-2026-10-04-partial-je2"></a>
- [je2-구현--completed-parser-result의-공통-transport-2026-10-04-partial-je2](docs/project/chapters/11-roadmap-checklists-p02.ko.md#je2-구현--completed-parser-result의-공통-transport-2026-10-04-partial-je2)

<a id="je2-구현--modifier-constructor-operand의-공통-경계-2026-10-04-partial-je2"></a>
- [je2-구현--modifier-constructor-operand의-공통-경계-2026-10-04-partial-je2](docs/project/chapters/11-roadmap-checklists-p02.ko.md#je2-구현--modifier-constructor-operand의-공통-경계-2026-10-04-partial-je2)

<a id="je2-구현--rankconjunction-operand와-오류-우선순위-2026-10-04-partial-je2"></a>
- [je2-구현--rankconjunction-operand와-오류-우선순위-2026-10-04-partial-je2](docs/project/chapters/11-roadmap-checklists-p02.ko.md#je2-구현--rankconjunction-operand와-오류-우선순위-2026-10-04-partial-je2)

<a id="je2-구현--noun-left-fork와-definition-생성-경계-2026-10-04-partial-je2"></a>
- [je2-구현--noun-left-fork와-definition-생성-경계-2026-10-04-partial-je2](docs/project/chapters/11-roadmap-checklists-p02.ko.md#je2-구현--noun-left-fork와-definition-생성-경계-2026-10-04-partial-je2)

<a id="je2-구현--잔여-adapter-감사와-frontend-체크리스트-수렴-2026-10-04-partial-je2"></a>
- [je2-구현--잔여-adapter-감사와-frontend-체크리스트-수렴-2026-10-04-partial-je2](docs/project/chapters/11-roadmap-checklists-p02.ko.md#je2-구현--잔여-adapter-감사와-frontend-체크리스트-수렴-2026-10-04-partial-je2)

<a id="je2p3-구현--immediate-constructor-결과-경계와-row-도달성-정정-2026-10-04-partial"></a>
- [je2p3-구현--immediate-constructor-결과-경계와-row-도달성-정정-2026-10-04-partial](docs/project/chapters/11-roadmap-checklists-p02.ko.md#je2p3-구현--immediate-constructor-결과-경계와-row-도달성-정정-2026-10-04-partial)

<a id="je2p3-구현--nonoperator-explicit-modifier의-첫-invocation-경계-2026-10-04-partial"></a>
- [je2p3-구현--nonoperator-explicit-modifier의-첫-invocation-경계-2026-10-04-partial](docs/project/chapters/11-roadmap-checklists-p02.ko.md#je2p3-구현--nonoperator-explicit-modifier의-첫-invocation-경계-2026-10-04-partial)

<a id="je2p3-구현--modifier-지역전역-대입과-직선-본문-실행-2026-10-04-partial"></a>
- [je2p3-구현--modifier-지역전역-대입과-직선-본문-실행-2026-10-04-partial](docs/project/chapters/11-roadmap-checklists-p02.ko.md#je2p3-구현--modifier-지역전역-대입과-직선-본문-실행-2026-10-04-partial)

<a id="je2p3-구현--ordinary-name-scope-경계-해소-2026-10-04-partial"></a>
- [je2p3-구현--ordinary-name-scope-경계-해소-2026-10-04-partial](docs/project/chapters/11-roadmap-checklists-p02.ko.md#je2p3-구현--ordinary-name-scope-경계-해소-2026-10-04-partial)

<a id="je2p3-구현--xy-operator의-deferred-verb와-직선-호출-2026-10-04-partial"></a>
- [je2p3-구현--xy-operator의-deferred-verb와-직선-호출-2026-10-04-partial](docs/project/chapters/11-roadmap-checklists-p02.ko.md#je2p3-구현--xy-operator의-deferred-verb와-직선-호출-2026-10-04-partial)

<a id="je2p3-구현--implicit-operand의-반환-시-고정-2026-10-04-partial"></a>
- [je2p3-구현--implicit-operand의-반환-시-고정-2026-10-04-partial](docs/project/chapters/11-roadmap-checklists-p02.ko.md#je2p3-구현--implicit-operand의-반환-시-고정-2026-10-04-partial)

<a id="je2p3-구현--implicit-operand의-caller-scope-호출-2026-10-04-partial"></a>
- [je2p3-구현--implicit-operand의-caller-scope-호출-2026-10-04-partial](docs/project/chapters/11-roadmap-checklists-p02.ko.md#je2p3-구현--implicit-operand의-caller-scope-호출-2026-10-04-partial)

<a id="je2p3-구현--wrapper와-train의-scope-aware-실행-2026-10-04-partial"></a>
- [je2p3-구현--wrapper와-train의-scope-aware-실행-2026-10-04-partial](docs/project/chapters/11-roadmap-checklists-p02.ko.md#je2p3-구현--wrapper와-train의-scope-aware-실행-2026-10-04-partial)

<a id="je2p3-구현--empty-identity와-순수-ravel-prototype-2026-10-04-partial"></a>
- [je2p3-구현--empty-identity와-순수-ravel-prototype-2026-10-04-partial](docs/project/chapters/11-roadmap-checklists-p02.ko.md#je2p3-구현--empty-identity와-순수-ravel-prototype-2026-10-04-partial)

<a id="je2p3-구현--noun-left-fork-호출과-snapshot-공유-2026-10-04-partial"></a>
- [je2p3-구현--noun-left-fork-호출과-snapshot-공유-2026-10-04-partial](docs/project/chapters/11-roadmap-checklists-p02.ko.md#je2p3-구현--noun-left-fork-호출과-snapshot-공유-2026-10-04-partial)

<a id="je2p3-구현--capped-fork의-생성-의미와-graph-표현-2026-10-05-partial"></a>
- [je2p3-구현--capped-fork의-생성-의미와-graph-표현-2026-10-05-partial](docs/project/chapters/11-roadmap-checklists-p02.ko.md#je2p3-구현--capped-fork의-생성-의미와-graph-표현-2026-10-05-partial)

<a id="je2p3-구현--verb-valued-rank의-생성-정보와-실행-이름-분리-2026-10-05-partial"></a>
- [je2p3-구현--verb-valued-rank의-생성-정보와-실행-이름-분리-2026-10-05-partial](docs/project/chapters/11-roadmap-checklists-p02.ko.md#je2p3-구현--verb-valued-rank의-생성-정보와-실행-이름-분리-2026-10-05-partial)

<a id="je3--operator-specific-higher-order-view-필요성-검증"></a>
- [je3--operator-specific-higher-order-view-필요성-검증](docs/project/chapters/11-roadmap-checklists-p02.ko.md#je3--operator-specific-higher-order-view-필요성-검증)

<a id="je4--gerundboxed-higher-order-semantics-통합"></a>
- [je4--gerundboxed-higher-order-semantics-통합](docs/project/chapters/11-roadmap-checklists-p02.ko.md#je4--gerundboxed-higher-order-semantics-통합)

<a id="je5--entity-algebra와-array-execution-algebra의-경계"></a>
- [je5--entity-algebra와-array-execution-algebra의-경계](docs/project/chapters/11-roadmap-checklists-p02.ko.md#je5--entity-algebra와-array-execution-algebra의-경계)

<a id="je6--migration-cleanup과-비용-검증"></a>
- [je6--migration-cleanup과-비용-검증](docs/project/chapters/11-roadmap-checklists-p02.ko.md#je6--migration-cleanup과-비용-검증)

<a id="a0--문서아키텍처-경계"></a>
- [a0--문서아키텍처-경계](docs/project/chapters/11-roadmap-checklists-p02.ko.md#a0--문서아키텍처-경계)

<a id="a05--jsource-compatible-parser-이행-체크리스트"></a>
- [a05--jsource-compatible-parser-이행-체크리스트](docs/project/chapters/11-roadmap-checklists-p02.ko.md#a05--jsource-compatible-parser-이행-체크리스트)

<a id="frontend-파일-경계"></a>
- [frontend-파일-경계](docs/project/chapters/11-roadmap-checklists-p02.ko.md#frontend-파일-경계)

<a id="최우선-실행-계획과-단계별-논리-동등성-2026-10-03"></a>
- [최우선-실행-계획과-단계별-논리-동등성-2026-10-03](docs/project/chapters/11-roadmap-checklists-p02.ko.md#최우선-실행-계획과-단계별-논리-동등성-2026-10-03)

<a id="noun-reduction과-컴파일용-구조-보존--조사-및-구현-계획-2026-10-03"></a>
- [noun-reduction과-컴파일용-구조-보존--조사-및-구현-계획-2026-10-03](docs/project/chapters/11-roadmap-checklists-p02.ko.md#noun-reduction과-컴파일용-구조-보존--조사-및-구현-계획-2026-10-03)

<a id="다른-언어배열-프레임워크의-처리"></a>
- [다른-언어배열-프레임워크의-처리](docs/project/chapters/11-roadmap-checklists-p02.ko.md#다른-언어배열-프레임워크의-처리)

<a id="값과-기록의-소유권"></a>
- [값과-기록의-소유권](docs/project/chapters/11-roadmap-checklists-p02.ko.md#값과-기록의-소유권)

<a id="실행-경로와-재사용-경계"></a>
- [실행-경로와-재사용-경계](docs/project/chapters/11-roadmap-checklists-p02.ko.md#실행-경로와-재사용-경계)

<a id="static-frontend-review"></a>
- [static-frontend-review](docs/project/chapters/11-roadmap-checklists-p03.ko.md#static-frontend-review)

<a id="최적화에-사용할-frontend-정보-보존-2026-10-03"></a>
- [최적화에-사용할-frontend-정보-보존-2026-10-03](docs/project/chapters/11-roadmap-checklists-p03.ko.md#최적화에-사용할-frontend-정보-보존-2026-10-03)

<a id="runtime-noun-reduction--별도-capture-v0-2026-10-03"></a>
- [runtime-noun-reduction--별도-capture-v0-2026-10-03](docs/project/chapters/11-roadmap-checklists-p03.ko.md#runtime-noun-reduction--별도-capture-v0-2026-10-03)

<a id="정적-modifier-구성과-분석-의존성-2026-10-03"></a>
- [정적-modifier-구성과-분석-의존성-2026-10-03](docs/project/chapters/11-roadmap-checklists-p03.ko.md#정적-modifier-구성과-분석-의존성-2026-10-03)

<a id="modifier-train의-구조품사-보존-2026-10-03"></a>
- [modifier-train의-구조품사-보존-2026-10-03](docs/project/chapters/11-roadmap-checklists-p03.ko.md#modifier-train의-구조품사-보존-2026-10-03)

<a id="오른쪽-operand를-묶은-modifier-bident-application-2026-10-03"></a>
- [오른쪽-operand를-묶은-modifier-bident-application-2026-10-03](docs/project/chapters/11-roadmap-checklists-p03.ko.md#오른쪽-operand를-묶은-modifier-bident-application-2026-10-03)

<a id="왼쪽-binding과-연속-adverb-application-2026-10-03"></a>
- [왼쪽-binding과-연속-adverb-application-2026-10-03](docs/project/chapters/11-roadmap-checklists-p03.ko.md#왼쪽-binding과-연속-adverb-application-2026-10-03)

<a id="adverbial-hook-및-derived-conjunction-적용-2026-10-03"></a>
- [adverbial-hook-및-derived-conjunction-적용-2026-10-03](docs/project/chapters/11-roadmap-checklists-p03.ko.md#adverbial-hook-및-derived-conjunction-적용-2026-10-03)

<a id="modifier-trident-action-확장-2026-10-03"></a>
- [modifier-trident-action-확장-2026-10-03](docs/project/chapters/11-roadmap-checklists-p03.ko.md#modifier-trident-action-확장-2026-10-03)

<a id="noun-left-rank-구성과-operand-역할-보존-2026-10-03"></a>
- [noun-left-rank-구성과-operand-역할-보존-2026-10-03](docs/project/chapters/11-roadmap-checklists-p03.ko.md#noun-left-rank-구성과-operand-역할-보존-2026-10-03)

<a id="primitive-gerund-구성과-audit-순서-2026-10-03"></a>
- [primitive-gerund-구성과-audit-순서-2026-10-03](docs/project/chapters/11-roadmap-checklists-p03.ko.md#primitive-gerund-구성과-audit-순서-2026-10-03)

<a id="복합-gerund-atomic-representation-해석-2026-10-03"></a>
- [복합-gerund-atomic-representation-해석-2026-10-03](docs/project/chapters/11-roadmap-checklists-p03.ko.md#복합-gerund-atomic-representation-해석-2026-10-03)

<a id="gerund-이름의-생성-시점-품사-확인-2026-10-03"></a>
- [gerund-이름의-생성-시점-품사-확인-2026-10-03](docs/project/chapters/11-roadmap-checklists-p03.ko.md#gerund-이름의-생성-시점-품사-확인-2026-10-03)

<a id="gerund-내부-noun-snapshot과-조회-capture-2026-10-03"></a>
- [gerund-내부-noun-snapshot과-조회-capture-2026-10-03](docs/project/chapters/11-roadmap-checklists-p03.ko.md#gerund-내부-noun-snapshot과-조회-capture-2026-10-03)

<a id="bidenttrident-ar-내부-noun-실행과-capture-2026-10-03"></a>
- [bidenttrident-ar-내부-noun-실행과-capture-2026-10-03](docs/project/chapters/11-roadmap-checklists-p03.ko.md#bidenttrident-ar-내부-noun-실행과-capture-2026-10-03)

<a id="nameless-modifier-snapshot과-train-내부-지연-조회-2026-10-03"></a>
- [nameless-modifier-snapshot과-train-내부-지연-조회-2026-10-03](docs/project/chapters/11-roadmap-checklists-p03.ko.md#nameless-modifier-snapshot과-train-내부-지연-조회-2026-10-03)

<a id="bidenttrident-constructor-inventory-전체-조합-검증-2026-10-03"></a>
- [bidenttrident-constructor-inventory-전체-조합-검증-2026-10-03](docs/project/chapters/11-roadmap-checklists-p03.ko.md#bidenttrident-constructor-inventory-전체-조합-검증-2026-10-03)

<a id="definition-입력-수집과-실행-없는-source-구조-2026-10-03-def-1-일부"></a>
- [definition-입력-수집과-실행-없는-source-구조-2026-10-03-def-1-일부](docs/project/chapters/11-roadmap-checklists-p03.ko.md#definition-입력-수집과-실행-없는-source-구조-2026-10-03-def-1-일부)

<a id="definitioncode-생성valencecapture-provenance-2026-10-04-partial-def-12"></a>
- [definitioncode-생성valencecapture-provenance-2026-10-04-partial-def-12](docs/project/chapters/11-roadmap-checklists-p03.ko.md#definitioncode-생성valencecapture-provenance-2026-10-04-partial-def-12)

<a id="control-word-분할과-body-진단-위치-2026-10-04-partial-def-2"></a>
- [control-word-분할과-body-진단-위치-2026-10-04-partial-def-2](docs/project/chapters/11-roadmap-checklists-p03.ko.md#control-word-분할과-body-진단-위치-2026-10-04-partial-def-2)

<a id="definition-control-구조와-valence별-audit-2026-10-04-partial-def-2"></a>
- [definition-control-구조와-valence별-audit-2026-10-04-partial-def-2](docs/project/chapters/11-roadmap-checklists-p03.ko.md#definition-control-구조와-valence별-audit-2026-10-04-partial-def-2)

<a id="이전-b-block-결과-자격-metadata-2026-10-04-partial-def-2"></a>
- [이전-b-block-결과-자격-metadata-2026-10-04-partial-def-2](docs/project/chapters/11-roadmap-checklists-p03.ko.md#이전-b-block-결과-자격-metadata-2026-10-04-partial-def-2)

<a id="gotolabel-연결과-구조-진입-audit-2026-10-04-partial-def-2"></a>
- [gotolabel-연결과-구조-진입-audit-2026-10-04-partial-def-2](docs/project/chapters/11-roadmap-checklists-p03.ko.md#gotolabel-연결과-구조-진입-audit-2026-10-04-partial-def-2)

<a id="한-문장의-여러-direct-definition-보존-2026-10-04-partial-def-12"></a>
- [한-문장의-여러-direct-definition-보존-2026-10-04-partial-def-12](docs/project/chapters/11-roadmap-checklists-p03.ko.md#한-문장의-여러-direct-definition-보존-2026-10-04-partial-def-12)

<a id="tagged-noun-dd의-raw-입력noun-보존-2026-10-04-partial-def-12"></a>
- [tagged-noun-dd의-raw-입력noun-보존-2026-10-04-partial-def-12](docs/project/chapters/11-roadmap-checklists-p03.ko.md#tagged-noun-dd의-raw-입력noun-보존-2026-10-04-partial-def-12)

<a id="정적-분석-수용-기준과-기존-frontend-구조-재검토-2026-10-03"></a>
- [정적-분석-수용-기준과-기존-frontend-구조-재검토-2026-10-03](docs/project/chapters/11-roadmap-checklists-p03.ko.md#정적-분석-수용-기준과-기존-frontend-구조-재검토-2026-10-03)

<a id="구현-순서와-완료-체크리스트"></a>
- [구현-순서와-완료-체크리스트](docs/project/chapters/11-roadmap-checklists-p03.ko.md#구현-순서와-완료-체크리스트)

<a id="테스트-matrix와-수용-조건"></a>
- [테스트-matrix와-수용-조건](docs/project/chapters/11-roadmap-checklists-p03.ko.md#테스트-matrix와-수용-조건)

<a id="f0--jsource-word-formation-이식"></a>
- [f0--jsource-word-formation-이식](docs/project/chapters/11-roadmap-checklists-p03.ko.md#f0--jsource-word-formation-이식)

<a id="f1--jsource-enqueue--primitiveresolver-이식"></a>
- [f1--jsource-enqueue--primitiveresolver-이식](docs/project/chapters/11-roadmap-checklists-p03.ko.md#f1--jsource-enqueue--primitiveresolver-이식)

<a id="f2--jsource-parse-queue-skeleton"></a>
- [f2--jsource-parse-queue-skeleton](docs/project/chapters/11-roadmap-checklists-p03.ko.md#f2--jsource-parse-queue-skeleton)

<a id="p0--기준선과-differential-oracle-고정"></a>
- [p0--기준선과-differential-oracle-고정](docs/project/chapters/11-roadmap-checklists-p03.ko.md#p0--기준선과-differential-oracle-고정)

<a id="p1--parser-stack-model과-semantic-value-model-분리"></a>
- [p1--parser-stack-model과-semantic-value-model-분리](docs/project/chapters/11-roadmap-checklists-p03.ko.md#p1--parser-stack-model과-semantic-value-model-분리)

<a id="j-graph-implementation-checklist"></a>
- [j-graph-implementation-checklist](docs/project/chapters/11-roadmap-checklists-p04.ko.md#j-graph-implementation-checklist)

<a id="graph-prior-art-followup"></a>
- [graph-prior-art-followup](docs/project/chapters/11-roadmap-checklists-p04.ko.md#graph-prior-art-followup)

<a id="common-array-algebra-hypothesis"></a>
- [common-array-algebra-hypothesis](docs/project/chapters/11-roadmap-checklists-p04.ko.md#common-array-algebra-hypothesis)

<a id="eqsat-successors-plan"></a>
- [eqsat-successors-plan](docs/project/chapters/11-roadmap-checklists-p04.ko.md#eqsat-successors-plan)

<a id="eqgraph-architecture"></a>
- [eqgraph-architecture](docs/project/chapters/11-roadmap-checklists-p04.ko.md#eqgraph-architecture)

<a id="eqgraph-error-semantics"></a>
- [eqgraph-error-semantics](docs/project/chapters/11-roadmap-checklists-p04.ko.md#eqgraph-error-semantics)

<a id="frontend-equality-synthesis"></a>
- [frontend-equality-synthesis](docs/project/chapters/11-roadmap-checklists-p04.ko.md#frontend-equality-synthesis)

<a id="spores-relational-integration"></a>
- [spores-relational-integration](docs/project/chapters/11-roadmap-checklists-p04.ko.md#spores-relational-integration)

<a id="conv-graph-ir-optimization"></a>
- [conv-graph-ir-optimization](docs/project/chapters/11-roadmap-checklists-p04.ko.md#conv-graph-ir-optimization)

<a id="conv2d-graph-spores-semantic-blackbox"></a>
- [conv2d-graph-spores-semantic-blackbox](docs/project/chapters/11-roadmap-checklists-p04.ko.md#conv2d-graph-spores-semantic-blackbox)

<a id="gf2-composition-review"></a>
- [gf2-composition-review](docs/project/chapters/11-roadmap-checklists-p04.ko.md#gf2-composition-review)

<a id="gf6a-fusion-readiness"></a>
- [gf6a-fusion-readiness](docs/project/chapters/11-roadmap-checklists-p04.ko.md#gf6a-fusion-readiness)

<a id="gf5-work-depth"></a>
- [gf5-work-depth](docs/project/chapters/11-roadmap-checklists-p04.ko.md#gf5-work-depth)

<a id="gf4-fusion-registry"></a>
- [gf4-fusion-registry](docs/project/chapters/11-roadmap-checklists-p04.ko.md#gf4-fusion-registry)

<a id="gf3-scan-identity"></a>
- [gf3-scan-identity](docs/project/chapters/11-roadmap-checklists-p04.ko.md#gf3-scan-identity)

<a id="p2--하나의-9-row-reduction-engine으로-전환"></a>
- [p2--하나의-9-row-reduction-engine으로-전환](docs/project/chapters/11-roadmap-checklists-p04.ko.md#p2--하나의-9-row-reduction-engine으로-전환)

<a id="p3--modifierhookforkbidenttrident-construction-semantics"></a>
- [p3--modifierhookforkbidenttrident-construction-semantics](docs/project/chapters/11-roadmap-checklists-p04.ko.md#p3--modifierhookforkbidenttrident-construction-semantics)

<a id="p4--parser-time-name-resolution과-assignment-sequencing"></a>
- [p4--parser-time-name-resolution과-assignment-sequencing](docs/project/chapters/11-roadmap-checklists-p04.ko.md#p4--parser-time-name-resolution과-assignment-sequencing)

<a id="p5--construction-time-j-semantics와-compiler-analysis-facts-분리"></a>
- [p5--construction-time-j-semantics와-compiler-analysis-facts-분리](docs/project/chapters/11-roadmap-checklists-p04.ko.md#p5--construction-time-j-semantics와-compiler-analysis-facts-분리)

<a id="p6--differentialconformance-test-matrix"></a>
- [p6--differentialconformance-test-matrix](docs/project/chapters/11-roadmap-checklists-p04.ko.md#p6--differentialconformance-test-matrix)

<a id="p7--cutover와-legacy-parser-제거"></a>
- [p7--cutover와-legacy-parser-제거](docs/project/chapters/11-roadmap-checklists-p04.ko.md#p7--cutover와-legacy-parser-제거)

<a id="p8--a1a2a3로의-integration-handoff"></a>
- [p8--a1a2a3로의-integration-handoff](docs/project/chapters/11-roadmap-checklists-p04.ko.md#p8--a1a2a3로의-integration-handoff)

<a id="진행-규칙"></a>
- [진행-규칙](docs/project/chapters/11-roadmap-checklists-p04.ko.md#진행-규칙)

<a id="a06--structured-diagnostic-context"></a>
- [a06--structured-diagnostic-context](docs/project/chapters/11-roadmap-checklists-p04.ko.md#a06--structured-diagnostic-context)

<a id="a1--j-semantic-construction-ir--functionentity-경계"></a>
- [a1--j-semantic-construction-ir--functionentity-경계](docs/project/chapters/11-roadmap-checklists-p04.ko.md#a1--j-semantic-construction-ir--functionentity-경계)

<a id="a15--j-graph-ir--jaxa-array-operation-graph-ir"></a>
- [a15--j-graph-ir--jaxa-array-operation-graph-ir](docs/project/chapters/11-roadmap-checklists-p04.ko.md#a15--j-graph-ir--jaxa-array-operation-graph-ir)

<a id="2026-10-05-선행연구-후속-작은-basis와-composition-algebra"></a>
- [2026-10-05-선행연구-후속-작은-basis와-composition-algebra](docs/project/chapters/11-roadmap-checklists-p04.ko.md#2026-10-05-선행연구-후속-작은-basis와-composition-algebra)

<a id="2026-10-11-연구-가설-배열-계산의-공통-접근-대수와-증명-가능한-재작성"></a>
- [2026-10-11-연구-가설-배열-계산의-공통-접근-대수와-증명-가능한-재작성](docs/project/chapters/11-roadmap-checklists-p04.ko.md#2026-10-11-연구-가설-배열-계산의-공통-접근-대수와-증명-가능한-재작성)

<a id="1-문제-정의-표현-가능성동치성최적화-유용성은-서로-다른-주장"></a>
- [1-문제-정의-표현-가능성동치성최적화-유용성은-서로-다른-주장](docs/project/chapters/11-roadmap-checklists-p04.ko.md#1-문제-정의-표현-가능성동치성최적화-유용성은-서로-다른-주장)

<a id="2-논리적-배열-모델과-좌표--값-모델의-한계"></a>
- [2-논리적-배열-모델과-좌표--값-모델의-한계](docs/project/chapters/11-roadmap-checklists-p04.ko.md#2-논리적-배열-모델과-좌표--값-모델의-한계)

<a id="3-공통-구조-후보-dacc--j-semantic-envelope"></a>
- [3-공통-구조-후보-dacc--j-semantic-envelope](docs/project/chapters/11-roadmap-checklists-p04.ko.md#3-공통-구조-후보-dacc--j-semantic-envelope)

<a id="4-여섯-연산군에서-공통-구조를-유도하는-예"></a>
- [4-여섯-연산군에서-공통-구조를-유도하는-예](docs/project/chapters/11-roadmap-checklists-p04.ko.md#4-여섯-연산군에서-공통-구조를-유도하는-예)

<a id="5-공통-재작성-후보--모두-proof-obligation이-있음"></a>
- [5-공통-재작성-후보--모두-proof-obligation이-있음](docs/project/chapters/11-roadmap-checklists-p04.ko.md#5-공통-재작성-후보--모두-proof-obligation이-있음)

<a id="6-같은-대수에서-역방향-arrayregion-demand를-얻는-법"></a>
- [6-같은-대수에서-역방향-arrayregion-demand를-얻는-법](docs/project/chapters/11-roadmap-checklists-p04.ko.md#6-같은-대수에서-역방향-arrayregion-demand를-얻는-법)

<a id="7-선행연구-비교--동일성-주장이-아니라-부분적-증거"></a>
- [7-선행연구-비교--동일성-주장이-아니라-부분적-증거](docs/project/chapters/11-roadmap-checklists-p04.ko.md#7-선행연구-비교--동일성-주장이-아니라-부분적-증거)

<a id="71-spores-원-논문-재검토--nn-적용-범위와-완전성의-정확한-의미-2026-10-11"></a>
- [71-spores-원-논문-재검토--nn-적용-범위와-완전성의-정확한-의미-2026-10-11](docs/project/chapters/11-roadmap-checklists-p04.ko.md#71-spores-원-논문-재검토--nn-적용-범위와-완전성의-정확한-의미-2026-10-11)

<a id="72-tensat-원-논문구현-재검토--equivalent-form-graph-search의-직접-선례-2026-10-11"></a>
- [72-tensat-원-논문구현-재검토--equivalent-form-graph-search의-직접-선례-2026-10-11](docs/project/chapters/11-roadmap-checklists-p04.ko.md#72-tensat-원-논문구현-재검토--equivalent-form-graph-search의-직접-선례-2026-10-11)

<a id="73-spores-이후의-실용-기술-egg-relational-e-matching-egglog-2026-10-12"></a>
- [73-spores-이후의-실용-기술-egg-relational-e-matching-egglog-2026-10-12](docs/project/chapters/11-roadmap-checklists-p04.ko.md#73-spores-이후의-실용-기술-egg-relational-e-matching-egglog-2026-10-12)

<a id="74-rustj-등가-포화-adapter-계약--법적-등가성-구조적-탐색-추출을-분리"></a>
- [74-rustj-등가-포화-adapter-계약--법적-등가성-구조적-탐색-추출을-분리](docs/project/chapters/11-roadmap-checklists-p04.ko.md#74-rustj-등가-포화-adapter-계약--법적-등가성-구조적-탐색-추출을-분리)

<a id="75-기존-gffwio-수용-원장에-속하는-실행-체크리스트"></a>
- [75-기존-gffwio-수용-원장에-속하는-실행-체크리스트](docs/project/chapters/11-roadmap-checklists-p04.ko.md#75-기존-gffwio-수용-원장에-속하는-실행-체크리스트)

<a id="76-아키텍처-결정-기존-j-graph-ir-옆에-e-graph를-실제로-둔다-2026-10-12"></a>
- [76-아키텍처-결정-기존-j-graph-ir-옆에-e-graph를-실제로-둔다-2026-10-12](docs/project/chapters/11-roadmap-checklists-p04.ko.md#76-아키텍처-결정-기존-j-graph-ir-옆에-e-graph를-실제로-둔다-2026-10-12)

<a id="77-e-graph-오류실패-의미론-검토-오류는-계산의-관찰-가능한-결과다-2026-10-12"></a>
- [77-e-graph-오류실패-의미론-검토-오류는-계산의-관찰-가능한-결과다-2026-10-12](docs/project/chapters/11-roadmap-checklists-p04.ko.md#77-e-graph-오류실패-의미론-검토-오류는-계산의-관찰-가능한-결과다-2026-10-12)

<a id="78-frontend-기반-j-동치-규칙-자동-발견생성-구문-표면과-의미론적-계산의-분리-2026-10-12"></a>
- [78-frontend-기반-j-동치-규칙-자동-발견생성-구문-표면과-의미론적-계산의-분리-2026-10-12](docs/project/chapters/11-roadmap-checklists-p04.ko.md#78-frontend-기반-j-동치-규칙-자동-발견생성-구문-표면과-의미론적-계산의-분리-2026-10-12)

<a id="79-spores-관계형-접근-대수-통합-넓은-표현-범위와-제한된-동치-법칙을-분리-2026-10-12"></a>
- [79-spores-관계형-접근-대수-통합-넓은-표현-범위와-제한된-동치-법칙을-분리-2026-10-12](docs/project/chapters/11-roadmap-checklists-p04.ko.md#79-spores-관계형-접근-대수-통합-넓은-표현-범위와-제한된-동치-법칙을-분리-2026-10-12)

<a id="8-rustj-적용-위치--검증-방법--합격반증-조건"></a>
- [8-rustj-적용-위치--검증-방법--합격반증-조건](docs/project/chapters/11-roadmap-checklists-p04.ko.md#8-rustj-적용-위치--검증-방법--합격반증-조건)

<a id="a151-과거-jaxa-역대조-감사"></a>
- [a151-과거-jaxa-역대조-감사](docs/project/chapters/11-roadmap-checklists-p04.ko.md#a151-과거-jaxa-역대조-감사)

<a id="2026-10-01-반복-감사에서-추가로-확정한-jaxa-계승-원칙"></a>
- [2026-10-01-반복-감사에서-추가로-확정한-jaxa-계승-원칙](docs/project/chapters/11-roadmap-checklists-p04.ko.md#2026-10-01-반복-감사에서-추가로-확정한-jaxa-계승-원칙)

<a id="out-of-core-io-checklist"></a>
- [out-of-core-io-checklist](docs/project/chapters/11-roadmap-checklists-p05.ko.md#out-of-core-io-checklist)

<a id="formal-ir-verification-checklist"></a>
- [formal-ir-verification-checklist](docs/project/chapters/11-roadmap-checklists-p05.ko.md#formal-ir-verification-checklist)

<a id="a152-jaxa의-static-memory-claim을-rustj에서-해석하는-방식"></a>
- [a152-jaxa의-static-memory-claim을-rustj에서-해석하는-방식](docs/project/chapters/11-roadmap-checklists-p05.ko.md#a152-jaxa의-static-memory-claim을-rustj에서-해석하는-방식)

<a id="a2--extension-primitive-registry와-analysis-contract"></a>
- [a2--extension-primitive-registry와-analysis-contract](docs/project/chapters/11-roadmap-checklists-p05.ko.md#a2--extension-primitive-registry와-analysis-contract)

<a id="a3--logical-execution-ir-core-verification-scheduling-boundary"></a>
- [a3--logical-execution-ir-core-verification-scheduling-boundary](docs/project/chapters/11-roadmap-checklists-p05.ko.md#a3--logical-execution-ir-core-verification-scheduling-boundary)

<a id="g1--논리-값과-물리-표현의-경계"></a>
- [g1--논리-값과-물리-표현의-경계](docs/project/chapters/11-roadmap-checklists-p05.ko.md#g1--논리-값과-물리-표현의-경계)

<a id="g2--복사-없는-structural-view"></a>
- [g2--복사-없는-structural-view](docs/project/chapters/11-roadmap-checklists-p05.ko.md#g2--복사-없는-structural-view)

<a id="g3--cpu-kernel-및-cell-mapping-연결"></a>
- [g3--cpu-kernel-및-cell-mapping-연결](docs/project/chapters/11-roadmap-checklists-p05.ko.md#g3--cpu-kernel-및-cell-mapping-연결)

<a id="g4--rustj-native-최소-physical-plan과-cpu-executor"></a>
- [g4--rustj-native-최소-physical-plan과-cpu-executor](docs/project/chapters/11-roadmap-checklists-p05.ko.md#g4--rustj-native-최소-physical-plan과-cpu-executor)

<a id="g5--rustj-native-성능-및-physical-확장-경계"></a>
- [g5--rustj-native-성능-및-physical-확장-경계](docs/project/chapters/11-roadmap-checklists-p05.ko.md#g5--rustj-native-성능-및-physical-확장-경계)

<a id="io--느린-ioout-of-core-실행-이행-계획수용-체크리스트-2026-10-06"></a>
- [io--느린-ioout-of-core-실행-이행-계획수용-체크리스트-2026-10-06](docs/project/chapters/11-roadmap-checklists-p05.ko.md#io--느린-ioout-of-core-실행-이행-계획수용-체크리스트-2026-10-06)

<a id="c--frontend--언어-의미-확장"></a>
- [c--frontend--언어-의미-확장](docs/project/chapters/11-roadmap-checklists-p05.ko.md#c--frontend--언어-의미-확장)

<a id="fv--ir-정확성-형식-검증-장기-후속-체크리스트-2026-10-08-계획만-기록"></a>
- [fv--ir-정확성-형식-검증-장기-후속-체크리스트-2026-10-08-계획만-기록](docs/project/chapters/11-roadmap-checklists-p05.ko.md#fv--ir-정확성-형식-검증-장기-후속-체크리스트-2026-10-08-계획만-기록)

<a id="cuda--보류"></a>
- [cuda--보류](docs/project/chapters/11-roadmap-checklists-p05.ko.md#cuda--보류)

<a id="validation-policy"></a>
- [validation-policy](docs/project/chapters/12-validation.ko.md#validation-policy)

<a id="read-validation"></a>
- [read-validation](docs/project/chapters/12-validation.ko.md#read-validation)

<a id="11-검증-정책"></a>
- [11-검증-정책](docs/project/chapters/12-validation.ko.md#11-검증-정책)

<a id="111-기본-완료-관문"></a>
- [111-기본-완료-관문](docs/project/chapters/12-validation.ko.md#111-기본-완료-관문)

<a id="112-semantic-bug-fix"></a>
- [112-semantic-bug-fix](docs/project/chapters/12-validation.ko.md#112-semantic-bug-fix)

<a id="113-storage--memory-변경"></a>
- [113-storage--memory-변경](docs/project/chapters/12-validation.ko.md#113-storage--memory-변경)

<a id="114-simd-변경"></a>
- [114-simd-변경](docs/project/chapters/12-validation.ko.md#114-simd-변경)

<a id="115-실행했다고-주장할-수-있는-것만-기록"></a>
- [115-실행했다고-주장할-수-있는-것만-기록](docs/project/chapters/12-validation.ko.md#115-실행했다고-주장할-수-있는-것만-기록)

<a id="116-c-reference"></a>
- [116-c-reference](docs/project/chapters/12-validation.ko.md#116-c-reference)

<a id="117-성능-해석"></a>
- [117-성능-해석](docs/project/chapters/12-validation.ko.md#117-성능-해석)

<a id="118-semantic-hard-cases의-golden-관문"></a>
- [118-semantic-hard-cases의-golden-관문](docs/project/chapters/12-validation.ko.md#118-semantic-hard-cases의-golden-관문)

<a id="119-ir-형식-검증의-미래-도입"></a>
- [119-ir-형식-검증의-미래-도입](docs/project/chapters/12-validation.ko.md#119-ir-형식-검증의-미래-도입)

<a id="current-implementation-status"></a>
- [current-implementation-status](docs/project/chapters/13-current-status.ko.md#current-implementation-status)

<a id="read-status"></a>
- [read-status](docs/project/chapters/13-current-status.ko.md#read-status)

<a id="read-references"></a>
- [read-references](docs/project/chapters/13-current-status.ko.md#read-references)

<a id="12-현재-검증구현-상태-요약"></a>
- [12-현재-검증구현-상태-요약](docs/project/chapters/13-current-status.ko.md#12-현재-검증구현-상태-요약)

<a id="상세-구현검증-기록-2026-10-07"></a>
- [상세-구현검증-기록-2026-10-07](docs/project/chapters/13-current-status.ko.md#상세-구현검증-기록-2026-10-07)

<a id="definition-호출과-frontend-e2e-확인-2026-10-07"></a>
- [definition-호출과-frontend-e2e-확인-2026-10-07](docs/project/chapters/13-current-status.ko.md#definition-호출과-frontend-e2e-확인-2026-10-07)

<a id="definition-반복중첩a3-함수-값-후속-검증-2026-10-07"></a>
- [definition-반복중첩a3-함수-값-후속-검증-2026-10-07](docs/project/chapters/13-current-status.ko.md#definition-반복중첩a3-함수-값-후속-검증-2026-10-07)

<a id="frontend-e2e-재감사-표현력후속-전달오류-2026-10-07"></a>
- [frontend-e2e-재감사-표현력후속-전달오류-2026-10-07](docs/project/chapters/13-current-status.ko.md#frontend-e2e-재감사-표현력후속-전달오류-2026-10-07)

<a id="실제-범위와-재현"></a>
- [실제-범위와-재현](docs/project/chapters/13-current-status.ko.md#실제-범위와-재현)

<a id="후속-단계에-전달되는-것과-계약의-한계"></a>
- [후속-단계에-전달되는-것과-계약의-한계](docs/project/chapters/13-current-status.ko.md#후속-단계에-전달되는-것과-계약의-한계)

<a id="오류-처리-재검토--a06p8a3-연결"></a>
- [오류-처리-재검토--a06p8a3-연결](docs/project/chapters/13-current-status.ko.md#오류-처리-재검토--a06p8a3-연결)

<a id="외부-프레임워크에서-적용할-부분"></a>
- [외부-프레임워크에서-적용할-부분](docs/project/chapters/13-current-status.ko.md#외부-프레임워크에서-적용할-부분)

<a id="frontend-감사-보완-실행-계획-2026-10-07"></a>
- [frontend-감사-보완-실행-계획-2026-10-07](docs/project/chapters/13-current-status.ko.md#frontend-감사-보완-실행-계획-2026-10-07)

<a id="f1p4-name_-실행-체크리스트"></a>
- [f1p4-name_-실행-체크리스트](docs/project/chapters/13-current-status.ko.md#f1p4-name_-실행-체크리스트)

<a id="ordered-name-effect-ir-첫-실행-단위"></a>
- [ordered-name-effect-ir-첫-실행-단위](docs/project/chapters/13-current-status.ko.md#ordered-name-effect-ir-첫-실행-단위)

<a id="name-효과와-배열-ir-연결-실행-체크리스트"></a>
- [name-효과와-배열-ir-연결-실행-체크리스트](docs/project/chapters/13-current-status.ko.md#name-효과와-배열-ir-연결-실행-체크리스트)

<a id="name-배열-region의-구현-계약--2026-10-08"></a>
- [name-배열-region의-구현-계약--2026-10-08](docs/project/chapters/13-current-status.ko.md#name-배열-region의-구현-계약--2026-10-08)

<a id="다중-apply-batch-전달실행-계약--2026-10-08"></a>
- [다중-apply-batch-전달실행-계약--2026-10-08](docs/project/chapters/13-current-status.ko.md#다중-apply-batch-전달실행-계약--2026-10-08)

<a id="frontend-a06--정의-실행-전반환-오류-경계-보완"></a>
- [frontend-a06--정의-실행-전반환-오류-경계-보완](docs/project/chapters/13-current-status.ko.md#frontend-a06--정의-실행-전반환-오류-경계-보완)

<a id="frontend-a06--파일-원문-식별과-중첩-정의-provenance"></a>
- [frontend-a06--파일-원문-식별과-중첩-정의-provenance](docs/project/chapters/13-current-status.ko.md#frontend-a06--파일-원문-식별과-중첩-정의-provenance)

<a id="frontend-a06p8--오류-소유권과-검증된-handoff"></a>
- [frontend-a06p8--오류-소유권과-검증된-handoff](docs/project/chapters/13-current-status.ko.md#frontend-a06p8--오류-소유권과-검증된-handoff)

<a id="frontend-숫자-변환-n1--decimal-integer-overflow"></a>
- [frontend-숫자-변환-n1--decimal-integer-overflow](docs/project/chapters/13-current-status.ko.md#frontend-숫자-변환-n1--decimal-integer-overflow)

<a id="frontend-숫자-변환-n2a--integer-spelling과-bool-축소"></a>
- [frontend-숫자-변환-n2a--integer-spelling과-bool-축소](docs/project/chapters/13-current-status.ko.md#frontend-숫자-변환-n2a--integer-spelling과-bool-축소)

<a id="frontend-숫자-변환-n2b--scientific-real의-정확한-int-축소"></a>
- [frontend-숫자-변환-n2b--scientific-real의-정확한-int-축소](docs/project/chapters/13-current-status.ko.md#frontend-숫자-변환-n2b--scientific-real의-정확한-int-축소)

<a id="n2c--decimal-real-family-ratio-변환-2026-10-08"></a>
- [n2c--decimal-real-family-ratio-변환-2026-10-08](docs/project/chapters/13-current-status.ko.md#n2c--decimal-real-family-ratio-변환-2026-10-08)

<a id="n3a--유한-extended-integer-payload와-저장-계약-2026-10-08"></a>
- [n3a--유한-extended-integer-payload와-저장-계약-2026-10-08](docs/project/chapters/13-current-status.ko.md#n3a--유한-extended-integer-payload와-저장-계약-2026-10-08)

<a id="n3b--rational-literal비유한-값공유-저장-2026-10-08"></a>
- [n3b--rational-literal비유한-값공유-저장-2026-10-08](docs/project/chapters/13-current-status.ko.md#n3b--rational-literal비유한-값공유-저장-2026-10-08)

<a id="n3c--exact-rational-산술비교-2026-10-08"></a>
- [n3c--exact-rational-산술비교-2026-10-08](docs/project/chapters/13-current-status.ko.md#n3c--exact-rational-산술비교-2026-10-08)

<a id="n3d--extendedint-exact-나눗셈역수와-다음-3단계-2026-10-08"></a>
- [n3d--extendedint-exact-나눗셈역수와-다음-3단계-2026-10-08](docs/project/chapters/13-current-status.ko.md#n3d--extendedint-exact-나눗셈역수와-다음-3단계-2026-10-08)

<a id="n3e--rational-primitive-right-fold-reduction-2026-10-08"></a>
- [n3e--rational-primitive-right-fold-reduction-2026-10-08](docs/project/chapters/13-current-status.ko.md#n3e--rational-primitive-right-fold-reduction-2026-10-08)

<a id="n3f--exact-rank-결과-조립-2026-10-08"></a>
- [n3f--exact-rank-결과-조립-2026-10-08](docs/project/chapters/13-current-status.ko.md#n3f--exact-rank-결과-조립-2026-10-08)

<a id="13-프레임워크-조사에서-채택한-원칙"></a>
- [13-프레임워크-조사에서-채택한-원칙](docs/project/chapters/14-framework-research-p01.ko.md#13-프레임워크-조사에서-채택한-원칙)

<a id="참고-구현과-jaxa-설계-배경--필요할-때-읽는-심화-자료"></a>
- [참고-구현과-jaxa-설계-배경--필요할-때-읽는-심화-자료](docs/project/chapters/14-framework-research-p01.ko.md#참고-구현과-jaxa-설계-배경--필요할-때-읽는-심화-자료)

<a id="참고-구현reference-implementations"></a>
- [참고-구현reference-implementations](docs/project/chapters/14-framework-research-p01.ko.md#참고-구현reference-implementations)

<a id="11-jaxa에서-이어받은-설계-원칙--배열-연산의-sql"></a>
- [11-jaxa에서-이어받은-설계-원칙--배열-연산의-sql](docs/project/chapters/14-framework-research-p01.ko.md#11-jaxa에서-이어받은-설계-원칙--배열-연산의-sql)

<a id="jaxa-graph-ir의-역사적-출발점--j-표기에서-optimization-topology를-읽는다"></a>
- [jaxa-graph-ir의-역사적-출발점--j-표기에서-optimization-topology를-읽는다](docs/project/chapters/14-framework-research-p01.ko.md#jaxa-graph-ir의-역사적-출발점--j-표기에서-optimization-topology를-읽는다)

<a id="관련-선행-연구와-rustj의-위치"></a>
- [관련-선행-연구와-rustj의-위치](docs/project/chapters/14-framework-research-p01.ko.md#관련-선행-연구와-rustj의-위치)

<a id="j-graph용-소스-대조최적화-계열-조사"></a>
- [j-graph용-소스-대조최적화-계열-조사](docs/project/chapters/14-framework-research-p01.ko.md#j-graph용-소스-대조최적화-계열-조사)

<a id="413-배열-컴파일-프레임워크와의-소스-기반-대조"></a>
- [413-배열-컴파일-프레임워크와의-소스-기반-대조](docs/project/chapters/14-framework-research-p01.ko.md#413-배열-컴파일-프레임워크와의-소스-기반-대조)

<a id="4131-분류와-조사-결과의-대응"></a>
- [4131-분류와-조사-결과의-대응](docs/project/chapters/14-framework-research-p01.ko.md#4131-분류와-조사-결과의-대응)

<a id="4132-발견한-힌트와-j-적용-방법"></a>
- [4132-발견한-힌트와-j-적용-방법](docs/project/chapters/14-framework-research-p01.ko.md#4132-발견한-힌트와-j-적용-방법)

<a id="f1--fork의-재결합-지점은-영역-전체-최적화-후보를-준다"></a>
- [f1--fork의-재결합-지점은-영역-전체-최적화-후보를-준다](docs/project/chapters/14-framework-research-p01.ko.md#f1--fork의-재결합-지점은-영역-전체-최적화-후보를-준다)

<a id="f2--같은-producer의-consumer-수는-공유재계산fusion-선택-힌트다"></a>
- [f2--같은-producer의-consumer-수는-공유재계산fusion-선택-힌트다](docs/project/chapters/14-framework-research-p01.ko.md#f2--같은-producer의-consumer-수는-공유재계산fusion-선택-힌트다)

<a id="f3--fork에서는-parallel과-fusion을-경쟁-후보로-둔다"></a>
- [f3--fork에서는-parallel과-fusion을-경쟁-후보로-둔다](docs/project/chapters/14-framework-research-p01.ko.md#f3--fork에서는-parallel과-fusion을-경쟁-후보로-둔다)

<a id="f4--rank-정보는-iteration-domain과-입력-mapping으로-보존한다"></a>
- [f4--rank-정보는-iteration-domain과-입력-mapping으로-보존한다](docs/project/chapters/14-framework-research-p01.ko.md#f4--rank-정보는-iteration-domain과-입력-mapping으로-보존한다)

<a id="f5--합성-경계에-입력-mapping을-결합하면-복사-제거-후보가-된다"></a>
- [f5--합성-경계에-입력-mapping을-결합하면-복사-제거-후보가-된다](docs/project/chapters/14-framework-research-p01.ko.md#f5--합성-경계에-입력-mapping을-결합하면-복사-제거-후보가-된다)

<a id="f6--window의-겹침은-지역-재사용필요-영역-힌트를-준다"></a>
- [f6--window의-겹침은-지역-재사용필요-영역-힌트를-준다](docs/project/chapters/14-framework-research-p01.ko.md#f6--window의-겹침은-지역-재사용필요-영역-힌트를-준다)

<a id="f7--power에서는-carry와-invariant를-분리한다"></a>
- [f7--power에서는-carry와-invariant를-분리한다](docs/project/chapters/14-framework-research-p01.ko.md#f7--power에서는-carry와-invariant를-분리한다)

<a id="f8--agenda와-중첩-train은-flat-dag-대신-영역으로-보존한다"></a>
- [f8--agenda와-중첩-train은-flat-dag-대신-영역으로-보존한다](docs/project/chapters/14-framework-research-p01.ko.md#f8--agenda와-중첩-train은-flat-dag-대신-영역으로-보존한다)

<a id="f9--값의-live-out과-효과의-live-out을-분리한다"></a>
- [f9--값의-live-out과-효과의-live-out을-분리한다](docs/project/chapters/14-framework-research-p01.ko.md#f9--값의-live-out과-효과의-live-out을-분리한다)

<a id="4133-사용자-예에-적용한-결과"></a>
- [4133-사용자-예에-적용한-결과](docs/project/chapters/14-framework-research-p01.ko.md#4133-사용자-예에-적용한-결과)

<a id="4134-j-graph-ir에-추가할-정보와-우선순위"></a>
- [4134-j-graph-ir에-추가할-정보와-우선순위](docs/project/chapters/14-framework-research-p01.ko.md#4134-j-graph-ir에-추가할-정보와-우선순위)

<a id="4135-체크리스트"></a>
- [4135-체크리스트](docs/project/chapters/14-framework-research-p01.ko.md#4135-체크리스트)

<a id="4136-jsource에서-채굴한-최적화-원리와-rustj-적용-위치"></a>
- [4136-jsource에서-채굴한-최적화-원리와-rustj-적용-위치](docs/project/chapters/14-framework-research-p01.ko.md#4136-jsource에서-채굴한-최적화-원리와-rustj-적용-위치)

<a id="a-source-derived-optimization-catalog"></a>
- [a-source-derived-optimization-catalog](docs/project/chapters/14-framework-research-p01.ko.md#a-source-derived-optimization-catalog)

<a id="b-pinned-source-links"></a>
- [b-pinned-source-links](docs/project/chapters/14-framework-research-p01.ko.md#b-pinned-source-links)

<a id="jsource-audit-m"></a>
- [jsource-audit-m](docs/project/chapters/14-framework-research-p02.ko.md#jsource-audit-m)

<a id="jsource-index-family"></a>
- [jsource-index-family](docs/project/chapters/14-framework-research-p02.ko.md#jsource-index-family)

<a id="algorithm-planning-migration"></a>
- [algorithm-planning-migration](docs/project/chapters/14-framework-research-p02.ko.md#algorithm-planning-migration)

<a id="framework-migration-checklist"></a>
- [framework-migration-checklist](docs/project/chapters/14-framework-research-p02.ko.md#framework-migration-checklist)

<a id="index-family-roadmap"></a>
- [index-family-roadmap](docs/project/chapters/14-framework-research-p02.ko.md#index-family-roadmap)

<a id="c-rustj-계층-경계"></a>
- [c-rustj-계층-경계](docs/project/chapters/14-framework-research-p02.ko.md#c-rustj-계층-경계)

<a id="d-구현-원칙"></a>
- [d-구현-원칙](docs/project/chapters/14-framework-research-p02.ko.md#d-구현-원칙)

<a id="e-독립-재검토-결과-2026-10-06"></a>
- [e-독립-재검토-결과-2026-10-06](docs/project/chapters/14-framework-research-p02.ko.md#e-독립-재검토-결과-2026-10-06)

<a id="f-별도-기준으로-실시한-3회-검증-2026-10-06"></a>
- [f-별도-기준으로-실시한-3회-검증-2026-10-06](docs/project/chapters/14-framework-research-p02.ko.md#f-별도-기준으로-실시한-3회-검증-2026-10-06)

<a id="g-이후-optimizer-구현-시-필요한-차등-검증-항목--아직-미실행"></a>
- [g-이후-optimizer-구현-시-필요한-차등-검증-항목--아직-미실행](docs/project/chapters/14-framework-research-p02.ko.md#g-이후-optimizer-구현-시-필요한-차등-검증-항목--아직-미실행)

<a id="h-추가-전역-조사에서-발견한-누락-계열-2026-10-06"></a>
- [h-추가-전역-조사에서-발견한-누락-계열-2026-10-06](docs/project/chapters/14-framework-research-p02.ko.md#h-추가-전역-조사에서-발견한-누락-계열-2026-10-06)

<a id="i-범위-및-누락-여부에-대한-보수적-판정"></a>
- [i-범위-및-누락-여부에-대한-보수적-판정](docs/project/chapters/14-framework-research-p02.ko.md#i-범위-및-누락-여부에-대한-보수적-판정)

<a id="j-rustj-프레임워크에-맞춘-구현-연결-2026-10-06"></a>
- [j-rustj-프레임워크에-맞춘-구현-연결-2026-10-06](docs/project/chapters/14-framework-research-p02.ko.md#j-rustj-프레임워크에-맞춘-구현-연결-2026-10-06)

<a id="k-후속-원본-재검토--단항-mean-guard와-런타임-최적화-경계-2026-10-06"></a>
- [k-후속-원본-재검토--단항-mean-guard와-런타임-최적화-경계-2026-10-06](docs/project/chapters/14-framework-research-p02.ko.md#k-후속-원본-재검토--단항-mean-guard와-런타임-최적화-경계-2026-10-06)

<a id="l-jsource-후보--a3-logical-ir--기존-loweringregistry-연결-2026-10-06"></a>
- [l-jsource-후보--a3-logical-ir--기존-loweringregistry-연결-2026-10-06](docs/project/chapters/14-framework-research-p02.ko.md#l-jsource-후보--a3-logical-ir--기존-loweringregistry-연결-2026-10-06)

<a id="m-jsource-재감사-기존-catalog-밖의-구조적수치적상태적-최적화-2026-10-06"></a>
- [m-jsource-재감사-기존-catalog-밖의-구조적수치적상태적-최적화-2026-10-06](docs/project/chapters/14-framework-research-p02.ko.md#m-jsource-재감사-기존-catalog-밖의-구조적수치적상태적-최적화-2026-10-06)

<a id="m1-rustj-프레임워크-수정-필요성-판정"></a>
- [m1-rustj-프레임워크-수정-필요성-판정](docs/project/chapters/14-framework-research-p02.ko.md#m1-rustj-프레임워크-수정-필요성-판정)

<a id="m2-이행-순서검증-범위"></a>
- [m2-이행-순서검증-범위](docs/project/chapters/14-framework-research-p02.ko.md#m2-이행-순서검증-범위)

<a id="n-roger-hui의-index-of-family와-rustj-단계적-구현-2026-10-06"></a>
- [n-roger-hui의-index-of-family와-rustj-단계적-구현-2026-10-06](docs/project/chapters/14-framework-research-p02.ko.md#n-roger-hui의-index-of-family와-rustj-단계적-구현-2026-10-06)

<a id="n1-첫-구현-단위--기존-ir-변경-없이-실행-경로-개선"></a>
- [n1-첫-구현-단위--기존-ir-변경-없이-실행-경로-개선](docs/project/chapters/14-framework-research-p02.ko.md#n1-첫-구현-단위--기존-ir-변경-없이-실행-경로-개선)

<a id="n2-다음-구현-gate--프레임워크-재설계-필요성"></a>
- [n2-다음-구현-gate--프레임워크-재설계-필요성](docs/project/chapters/14-framework-research-p02.ko.md#n2-다음-구현-gate--프레임워크-재설계-필요성)

<a id="n3-두-번째-구현-단위--reverse-hashing과-per-engine-prehash-2026-10-06"></a>
- [n3-두-번째-구현-단위--reverse-hashing과-per-engine-prehash-2026-10-06](docs/project/chapters/14-framework-research-p02.ko.md#n3-두-번째-구현-단위--reverse-hashing과-per-engine-prehash-2026-10-06)

<a id="o-타-프레임워크-대조를-통한-algorithm-planning-확장--i-family-첫-적용-2026-10-06"></a>
- [o-타-프레임워크-대조를-통한-algorithm-planning-확장--i-family-첫-적용-2026-10-06](docs/project/chapters/14-framework-research-p02.ko.md#o-타-프레임워크-대조를-통한-algorithm-planning-확장--i-family-첫-적용-2026-10-06)

<a id="o1-비교한-프레임워크와-채택비채택-사항"></a>
- [o1-비교한-프레임워크와-채택비채택-사항](docs/project/chapters/14-framework-research-p02.ko.md#o1-비교한-프레임워크와-채택비채택-사항)

<a id="o2-실제-코드로-반영한-세-경계"></a>
- [o2-실제-코드로-반영한-세-경계](docs/project/chapters/14-framework-research-p02.ko.md#o2-실제-코드로-반영한-세-경계)

<a id="o2a-동작-예시--3-1-3-i-3-4"></a>
- [o2a-동작-예시--3-1-3-i-3-4](docs/project/chapters/14-framework-research-p02.ko.md#o2a-동작-예시--3-1-3-i-3-4)

<a id="o3-앞으로-공통화할-설계-계약과-검증"></a>
- [o3-앞으로-공통화할-설계-계약과-검증](docs/project/chapters/14-framework-research-p02.ko.md#o3-앞으로-공통화할-설계-계약과-검증)

<a id="o4-검색-계열을-통한-공통-프레임워크-역설계--독립-4관점-재검토-2026-10-06"></a>
- [o4-검색-계열을-통한-공통-프레임워크-역설계--독립-4관점-재검토-2026-10-06](docs/project/chapters/14-framework-research-p02.ko.md#o4-검색-계열을-통한-공통-프레임워크-역설계--독립-4관점-재검토-2026-10-06)

<a id="o5-지연-최적화-프레임워크-이행-계획체크리스트-2026-10-06-살아-있는-작업표"></a>
- [o5-지연-최적화-프레임워크-이행-계획체크리스트-2026-10-06-살아-있는-작업표](docs/project/chapters/14-framework-research-p02.ko.md#o5-지연-최적화-프레임워크-이행-계획체크리스트-2026-10-06-살아-있는-작업표)

<a id="p-i-family-통합-로드맵진행-체크리스트--roger-hui--marshall-lochbaum-2026-10-06"></a>
- [p-i-family-통합-로드맵진행-체크리스트--roger-hui--marshall-lochbaum-2026-10-06](docs/project/chapters/14-framework-research-p02.ko.md#p-i-family-통합-로드맵진행-체크리스트--roger-hui--marshall-lochbaum-2026-10-06)

<a id="p0-지연-최적화-정책과-m2-우선순위-검토-2026-10-06"></a>
- [p0-지연-최적화-정책과-m2-우선순위-검토-2026-10-06](docs/project/chapters/14-framework-research-p02.ko.md#p0-지연-최적화-정책과-m2-우선순위-검토-2026-10-06)

<a id="p1-구현-단계와-수용-기준-계속-갱신"></a>
- [p1-구현-단계와-수용-기준-계속-갱신](docs/project/chapters/14-framework-research-p02.ko.md#p1-구현-단계와-수용-기준-계속-갱신)

<a id="p2-독립-검증-루프-반복결과-기록"></a>
- [p2-독립-검증-루프-반복결과-기록](docs/project/chapters/14-framework-research-p02.ko.md#p2-독립-검증-루프-반복결과-기록)

<a id="rank-cellapply-followups"></a>
- [rank-cellapply-followups](docs/project/chapters/14-framework-research-p03.ko.md#rank-cellapply-followups)

<a id="rank-graph-zero-frame"></a>
- [rank-graph-zero-frame](docs/project/chapters/14-framework-research-p03.ko.md#rank-graph-zero-frame)

<a id="jsource-optimization-migration"></a>
- [jsource-optimization-migration](docs/project/chapters/14-framework-research-p03.ko.md#jsource-optimization-migration)

<a id="p3-현재-known-gaps--바로-다음-순서--m2-우선으로-재정렬-2026-10-06"></a>
- [p3-현재-known-gaps--바로-다음-순서--m2-우선으로-재정렬-2026-10-06](docs/project/chapters/14-framework-research-p03.ko.md#p3-현재-known-gaps--바로-다음-순서--m2-우선으로-재정렬-2026-10-06)

<a id="p4-독립-검토-기록-1--제한된-float-search-후보-완전성-2026-10-06"></a>
- [p4-독립-검토-기록-1--제한된-float-search-후보-완전성-2026-10-06](docs/project/chapters/14-framework-research-p03.ko.md#p4-독립-검토-기록-1--제한된-float-search-후보-완전성-2026-10-06)

<a id="p5-독립-검토-기록-2--ieee-754-인접-비트원본-순서-보강-2026-10-06"></a>
- [p5-독립-검토-기록-2--ieee-754-인접-비트원본-순서-보강-2026-10-06](docs/project/chapters/14-framework-research-p03.ko.md#p5-독립-검토-기록-2--ieee-754-인접-비트원본-순서-보강-2026-10-06)

<a id="p6-원본-j-비교-정책증명-경계--source-first-독립-검토-2026-10-06"></a>
- [p6-원본-j-비교-정책증명-경계--source-first-독립-검토-2026-10-06](docs/project/chapters/14-framework-research-p03.ko.md#p6-원본-j-비교-정책증명-경계--source-first-독립-검토-2026-10-06)

<a id="p7-고정-비교-정책-스냅샷-구현--dynamic-j-cct-미지원-경계-2026-10-06"></a>
- [p7-고정-비교-정책-스냅샷-구현--dynamic-j-cct-미지원-경계-2026-10-06](docs/project/chapters/14-framework-research-p03.ko.md#p7-고정-비교-정책-스냅샷-구현--dynamic-j-cct-미지원-경계-2026-10-06)

<a id="p8-jsource-기본-비교식의-경계-불일치--미해결-의미론-차이-발견-2026-10-06"></a>
- [p8-jsource-기본-비교식의-경계-불일치--미해결-의미론-차이-발견-2026-10-06](docs/project/chapters/14-framework-research-p03.ko.md#p8-jsource-기본-비교식의-경계-불일치--미해결-의미론-차이-발견-2026-10-06)

<a id="p9-기본-j-cct-비교-정책으로-실행-경로-정렬--fw-03-부분-증거-2026-10-06"></a>
- [p9-기본-j-cct-비교-정책으로-실행-경로-정렬--fw-03-부분-증거-2026-10-06](docs/project/chapters/14-framework-research-p03.ko.md#p9-기본-j-cct-비교-정책으로-실행-경로-정렬--fw-03-부분-증거-2026-10-06)

<a id="p10-rank-zero-frame-fill-cell-실행검증--fw-04-부분-증거-2026-10-06"></a>
- [p10-rank-zero-frame-fill-cell-실행검증--fw-04-부분-증거-2026-10-06](docs/project/chapters/14-framework-research-p03.ko.md#p10-rank-zero-frame-fill-cell-실행검증--fw-04-부분-증거-2026-10-06)

<a id="p11-rankcellapply-후속-이행-체크리스트--fw-04jx-04-세부-실행-계획-2026-10-06"></a>
- [p11-rankcellapply-후속-이행-체크리스트--fw-04jx-04-세부-실행-계획-2026-10-06](docs/project/chapters/14-framework-research-p03.ko.md#p11-rankcellapply-후속-이행-체크리스트--fw-04jx-04-세부-실행-계획-2026-10-06)

<a id="p12-빈-frame빈-cell의-j-graph-ir-의미론--구조-증거와-실행-생략-허가-분리-2026-10-06"></a>
- [p12-빈-frame빈-cell의-j-graph-ir-의미론--구조-증거와-실행-생략-허가-분리-2026-10-06](docs/project/chapters/14-framework-research-p03.ko.md#p12-빈-frame빈-cell의-j-graph-ir-의미론--구조-증거와-실행-생략-허가-분리-2026-10-06)

<a id="q-jsource-전계열-최적화-이행-계획체크리스트--검색-이외-포함-2026-10-06-살아-있는-작업표"></a>
- [q-jsource-전계열-최적화-이행-계획체크리스트--검색-이외-포함-2026-10-06-살아-있는-작업표](docs/project/chapters/14-framework-research-p03.ko.md#q-jsource-전계열-최적화-이행-계획체크리스트--검색-이외-포함-2026-10-06-살아-있는-작업표)

<a id="q1-jx-01fw-01-정적-sourcefrontend-감사-2026-10-06"></a>
- [q1-jx-01fw-01-정적-sourcefrontend-감사-2026-10-06](docs/project/chapters/14-framework-research-p03.ko.md#q1-jx-01fw-01-정적-sourcefrontend-감사-2026-10-06)

<a id="q2-fw-01--jx-01-실행-회귀-및-원본-비교-게이트--두-번째-이행-기록-2026-10-06"></a>
- [q2-fw-01--jx-01-실행-회귀-및-원본-비교-게이트--두-번째-이행-기록-2026-10-06](docs/project/chapters/14-framework-research-p03.ko.md#q2-fw-01--jx-01-실행-회귀-및-원본-비교-게이트--두-번째-이행-기록-2026-10-06)

<a id="q3-fw-01jx-01-github-ci-녹색-검증과-기준선-기록-2026-10-06"></a>
- [q3-fw-01jx-01-github-ci-녹색-검증과-기준선-기록-2026-10-06](docs/project/chapters/14-framework-research-p03.ko.md#q3-fw-01jx-01-github-ci-녹색-검증과-기준선-기록-2026-10-06)

<a id="q4-fw-01jx-02의-첫-key--파생-동사-구성--실행과-최적화-허가는-분리-2026-10-06"></a>
- [q4-fw-01jx-02의-첫-key--파생-동사-구성--실행과-최적화-허가는-분리-2026-10-06](docs/project/chapters/14-framework-research-p03.ko.md#q4-fw-01jx-02의-첫-key--파생-동사-구성--실행과-최적화-허가는-분리-2026-10-06)

<a id="ndarray"></a>
- [ndarray](docs/project/chapters/14-framework-research-p03.ko.md#ndarray)

<a id="apache-arrow"></a>
- [apache-arrow](docs/project/chapters/14-framework-research-p03.ko.md#apache-arrow)

<a id="openxla"></a>
- [openxla](docs/project/chapters/14-framework-research-p03.ko.md#openxla)

<a id="arrayfire--j-arrayfire-add-on--fusion-systems"></a>
- [arrayfire--j-arrayfire-add-on--fusion-systems](docs/project/chapters/14-framework-research-p03.ko.md#arrayfire--j-arrayfire-add-on--fusion-systems)

<a id="jax--multi-device-systems"></a>
- [jax--multi-device-systems](docs/project/chapters/14-framework-research-p03.ko.md#jax--multi-device-systems)

<a id="mlir-core--interfaces--regions"></a>
- [mlir-core--interfaces--regions](docs/project/chapters/14-framework-research-p03.ko.md#mlir-core--interfaces--regions)

<a id="mlir-shape--dynamic-constraints"></a>
- [mlir-shape--dynamic-constraints](docs/project/chapters/14-framework-research-p03.ko.md#mlir-shape--dynamic-constraints)

<a id="mlir-linalg--bufferization"></a>
- [mlir-linalg--bufferization](docs/project/chapters/14-framework-research-p03.ko.md#mlir-linalg--bufferization)

<a id="mlir-transform-dialect--tvm-tensorir-schedule"></a>
- [mlir-transform-dialect--tvm-tensorir-schedule](docs/project/chapters/14-framework-research-p03.ko.md#mlir-transform-dialect--tvm-tensorir-schedule)

<a id="iree-flow--stream--hal"></a>
- [iree-flow--stream--hal](docs/project/chapters/14-framework-research-p03.ko.md#iree-flow--stream--hal)

<a id="stablehlo--vhlo"></a>
- [stablehlo--vhlo](docs/project/chapters/14-framework-research-p03.ko.md#stablehlo--vhlo)

<a id="triton"></a>
- [triton](docs/project/chapters/14-framework-research-p03.ko.md#triton)

<a id="중요한-비채택-사항"></a>
- [중요한-비채택-사항](docs/project/chapters/14-framework-research-p03.ko.md#중요한-비채택-사항)

<a id="read-advanced"></a>
- [read-advanced](docs/project/chapters/15-advanced-contracts-p01.ko.md#read-advanced)

<a id="extension-primitive-inventory"></a>
- [extension-primitive-inventory](docs/project/chapters/15-advanced-contracts-p01.ko.md#extension-primitive-inventory)

<a id="nn-extension-checklist"></a>
- [nn-extension-checklist](docs/project/chapters/15-advanced-contracts-p01.ko.md#nn-extension-checklist)

<a id="nn-explicit-adjoint-candidates"></a>
- [nn-explicit-adjoint-candidates](docs/project/chapters/15-advanced-contracts-p01.ko.md#nn-explicit-adjoint-candidates)

<a id="preexecution-input-information"></a>
- [preexecution-input-information](docs/project/chapters/15-advanced-contracts-p01.ko.md#preexecution-input-information)

<a id="심화-기술-계약--primitive-rank-하드웨어-계획-flowstorage"></a>
- [심화-기술-계약--primitive-rank-하드웨어-계획-flowstorage](docs/project/chapters/15-advanced-contracts-p01.ko.md#심화-기술-계약--primitive-rank-하드웨어-계획-flowstorage)

<a id="45-primitive-contract"></a>
- [45-primitive-contract](docs/project/chapters/15-advanced-contracts-p01.ko.md#45-primitive-contract)

<a id="46-확장-primitive-목록--기능-필요성-채택-상태"></a>
- [46-확장-primitive-목록--기능-필요성-채택-상태](docs/project/chapters/15-advanced-contracts-p01.ko.md#46-확장-primitive-목록--기능-필요성-채택-상태)

<a id="461-연구-사례와-core-j의-경계"></a>
- [461-연구-사례와-core-j의-경계](docs/project/chapters/15-advanced-contracts-p01.ko.md#461-연구-사례와-core-j의-경계)

<a id="462-최신-연구와-rustj의-적용-경계"></a>
- [462-최신-연구와-rustj의-적용-경계](docs/project/chapters/15-advanced-contracts-p01.ko.md#462-최신-연구와-rustj의-적용-경계)

<a id="462a-명시적-adjointvjp-후보-계약--nn-01nn-34-2026-10-11"></a>
- [462a-명시적-adjointvjp-후보-계약--nn-01nn-34-2026-10-11](docs/project/chapters/15-advanced-contracts-p01.ko.md#462a-명시적-adjointvjp-후보-계약--nn-01nn-34-2026-10-11)

<a id="463-기존-조사구현-증거-기록-2026-10-02-스냅샷"></a>
- [463-기존-조사구현-증거-기록-2026-10-02-스냅샷](docs/project/chapters/15-advanced-contracts-p01.ko.md#463-기존-조사구현-증거-기록-2026-10-02-스냅샷)

<a id="463a-nn배열-extension-first-수용-게이트실행-체크리스트-2026-10-10-설계계획만"></a>
- [463a-nn배열-extension-first-수용-게이트실행-체크리스트-2026-10-10-설계계획만](docs/project/chapters/15-advanced-contracts-p01.ko.md#463a-nn배열-extension-first-수용-게이트실행-체크리스트-2026-10-10-설계계획만)

<a id="47-과거-jaxajapchae-저장소-통합-기준"></a>
- [47-과거-jaxajapchae-저장소-통합-기준](docs/project/chapters/15-advanced-contracts-p01.ko.md#47-과거-jaxajapchae-저장소-통합-기준)

<a id="471-역사-저장소와-현행-rustj의-충돌-해소표"></a>
- [471-역사-저장소와-현행-rustj의-충돌-해소표](docs/project/chapters/15-advanced-contracts-p01.ko.md#471-역사-저장소와-현행-rustj의-충돌-해소표)

<a id="48-확장-primitive는-ordinary-name-binding으로-추가하고-parser-time-lookup을-따른다"></a>
- [48-확장-primitive는-ordinary-name-binding으로-추가하고-parser-time-lookup을-따른다](docs/project/chapters/15-advanced-contracts-p01.ko.md#48-확장-primitive는-ordinary-name-binding으로-추가하고-parser-time-lookup을-따른다)

<a id="481-실행-전-입력-정보--with-xy-후보와-metadata-검사"></a>
- [481-실행-전-입력-정보--with-xy-후보와-metadata-검사](docs/project/chapters/15-advanced-contracts-p01.ko.md#481-실행-전-입력-정보--with-xy-후보와-metadata-검사)

<a id="482-실행-전-입력-정보-명세-2026-10-04"></a>
- [482-실행-전-입력-정보-명세-2026-10-04](docs/project/chapters/15-advanced-contracts-p01.ko.md#482-실행-전-입력-정보-명세-2026-10-04)

<a id="설계-영향과-출처--채택할-아이디어와-rustj-판단의-구분"></a>
- [설계-영향과-출처--채택할-아이디어와-rustj-판단의-구분](docs/project/chapters/15-advanced-contracts-p01.ko.md#설계-영향과-출처--채택할-아이디어와-rustj-판단의-구분)

<a id="입력-metadata-조건-m0m8"></a>
- [입력-metadata-조건-m0m8](docs/project/chapters/15-advanced-contracts-p01.ko.md#입력-metadata-조건-m0m8)

<a id="소유권과-입력-저장소-조건-o0o8"></a>
- [소유권과-입력-저장소-조건-o0o8](docs/project/chapters/15-advanced-contracts-p01.ko.md#소유권과-입력-저장소-조건-o0o8)

<a id="정적-분석-우선-실제-배열-없이-계약에서-분석한다"></a>
- [정적-분석-우선-실제-배열-없이-계약에서-분석한다](docs/project/chapters/15-advanced-contracts-p01.ko.md#정적-분석-우선-실제-배열-없이-계약에서-분석한다)

<a id="array-compiler-static-analysis"></a>
- [array-compiler-static-analysis](docs/project/chapters/15-advanced-contracts-p02.ko.md#array-compiler-static-analysis)

<a id="입력-정보-문제의-프레임워크-비교-2026-10-04"></a>
- [입력-정보-문제의-프레임워크-비교-2026-10-04](docs/project/chapters/15-advanced-contracts-p02.ko.md#입력-정보-문제의-프레임워크-비교-2026-10-04)

<a id="483-배열-컴파일러의-정적-분석-대상과-프레임워크-비교-2026-10-04"></a>
- [483-배열-컴파일러의-정적-분석-대상과-프레임워크-비교-2026-10-04](docs/project/chapters/15-advanced-contracts-p02.ko.md#483-배열-컴파일러의-정적-분석-대상과-프레임워크-비교-2026-10-04)

<a id="같은-질문을-각-프레임워크가-다루는-방식"></a>
- [같은-질문을-각-프레임워크가-다루는-방식](docs/project/chapters/15-advanced-contracts-p02.ko.md#같은-질문을-각-프레임워크가-다루는-방식)

<a id="49-vocabulary는-이름-목록이-아니라-form--contract다"></a>
- [49-vocabulary는-이름-목록이-아니라-form--contract다](docs/project/chapters/15-advanced-contracts-p02.ko.md#49-vocabulary는-이름-목록이-아니라-form--contract다)

<a id="491-analyzable-profile은-허용-목록테스트로-고정한다"></a>
- [491-analyzable-profile은-허용-목록테스트로-고정한다](docs/project/chapters/15-advanced-contracts-p02.ko.md#491-analyzable-profile은-허용-목록테스트로-고정한다)

<a id="410-primitivespec은-semantic-record이고-realization은-별도-registryinterface다"></a>
- [410-primitivespec은-semantic-record이고-realization은-별도-registryinterface다](docs/project/chapters/15-advanced-contracts-p02.ko.md#410-primitivespec은-semantic-record이고-realization은-별도-registryinterface다)

<a id="4101-기존-j-primitive도-hardware-lowering의-주체다"></a>
- [4101-기존-j-primitive도-hardware-lowering의-주체다](docs/project/chapters/15-advanced-contracts-p02.ko.md#4101-기존-j-primitive도-hardware-lowering의-주체다)

<a id="411-rank와-axis-role은-서로-다른-정보다"></a>
- [411-rank와-axis-role은-서로-다른-정보다](docs/project/chapters/15-advanced-contracts-p02.ko.md#411-rank와-axis-role은-서로-다른-정보다)

<a id="4110-rankspec은-absoluteinfiniterelative-rank를-표현한다"></a>
- [4110-rankspec은-absoluteinfiniterelative-rank를-표현한다](docs/project/chapters/15-advanced-contracts-p02.ko.md#4110-rankspec은-absoluteinfiniterelative-rank를-표현한다)

<a id="4111-j-agreement는-prefix-frame-agreement다"></a>
- [4111-j-agreement는-prefix-frame-agreement다](docs/project/chapters/15-advanced-contracts-p03.ko.md#4111-j-agreement는-prefix-frame-agreement다)

<a id="4112-empty-frame은-아무-일도-하지-않음이-아니다"></a>
- [4112-empty-frame은-아무-일도-하지-않음이-아니다](docs/project/chapters/15-advanced-contracts-p03.ko.md#4112-empty-frame은-아무-일도-하지-않음이-아니다)

<a id="4113-rank-결과-assembly는-고정-shape-map보다-넓다"></a>
- [4113-rank-결과-assembly는-고정-shape-map보다-넓다](docs/project/chapters/15-advanced-contracts-p03.ko.md#4113-rank-결과-assembly는-고정-shape-map보다-넓다)

<a id="4114-implicit-loop는-모든-function-application의-cell-semantics다"></a>
- [4114-implicit-loop는-모든-function-application의-cell-semantics다](docs/project/chapters/15-advanced-contracts-p03.ko.md#4114-implicit-loop는-모든-function-application의-cell-semantics다)

<a id="41141-jsource-generic-rank-loop의-semantic-단계"></a>
- [41141-jsource-generic-rank-loop의-semantic-단계](docs/project/chapters/15-advanced-contracts-p03.ko.md#41141-jsource-generic-rank-loop의-semantic-단계)

<a id="41142-nested-rank-boundary를-early-collapse하지-않는다"></a>
- [41142-nested-rank-boundary를-early-collapse하지-않는다](docs/project/chapters/15-advanced-contracts-p03.ko.md#41142-nested-rank-boundary를-early-collapse하지-않는다)

<a id="41143-logical-ir에는-physical-for-loop가-아니라-cellapply를-둔다"></a>
- [41143-logical-ir에는-physical-for-loop가-아니라-cellapply를-둔다](docs/project/chapters/15-advanced-contracts-p03.ko.md#41143-logical-ir에는-physical-for-loop가-아니라-cellapply를-둔다)

<a id="41144-cell-projection은-copy가-아니라-view-semantics다"></a>
- [41144-cell-projection은-copy가-아니라-view-semantics다](docs/project/chapters/15-advanced-contracts-p03.ko.md#41144-cell-projection은-copy가-아니라-view-semantics다)

<a id="41145-emptyprototype와-result-assembly는-cellapply가-소유한다"></a>
- [41145-emptyprototype와-result-assembly는-cellapply가-소유한다](docs/project/chapters/15-advanced-contracts-p03.ko.md#41145-emptyprototype와-result-assembly는-cellapply가-소유한다)

<a id="41146-jsource-irs는-semantic-feature가-아니라-loop-absorption-optimization이다"></a>
- [41146-jsource-irs는-semantic-feature가-아니라-loop-absorption-optimization이다](docs/project/chapters/15-advanced-contracts-p03.ko.md#41146-jsource-irs는-semantic-feature가-아니라-loop-absorption-optimization이다)

<a id="41147-sparse-path는-별도-lowering이어도-semantic-cellapply는-같다"></a>
- [41147-sparse-path는-별도-lowering이어도-semantic-cellapply는-같다](docs/project/chapters/15-advanced-contracts-p03.ko.md#41147-sparse-path는-별도-lowering이어도-semantic-cellapply는-같다)

<a id="41148-구현-계획"></a>
- [41148-구현-계획](docs/project/chapters/15-advanced-contracts-p03.ko.md#41148-구현-계획)

<a id="mean-proof-example"></a>
- [mean-proof-example](docs/project/chapters/15-advanced-contracts-p04.ko.md#mean-proof-example)

<a id="41149-첫-differential-test-matrix"></a>
- [41149-첫-differential-test-matrix](docs/project/chapters/15-advanced-contracts-p04.ko.md#41149-첫-differential-test-matrix)

<a id="411410-jsource-구현에서-가져올-구조적-아이디어"></a>
- [411410-jsource-구현에서-가져올-구조적-아이디어](docs/project/chapters/15-advanced-contracts-p04.ko.md#411410-jsource-구현에서-가져올-구조적-아이디어)

<a id="411411-이후-ir에서-사용할-정보의-provenance와-삽입-시점"></a>
- [411411-이후-ir에서-사용할-정보의-provenance와-삽입-시점](docs/project/chapters/15-advanced-contracts-p04.ko.md#411411-이후-ir에서-사용할-정보의-provenance와-삽입-시점)

<a id="semantic-node는-intrinsic-summary를-직접-소유한다"></a>
- [semantic-node는-intrinsic-summary를-직접-소유한다](docs/project/chapters/15-advanced-contracts-p04.ko.md#semantic-node는-intrinsic-summary를-직접-소유한다)

<a id="call-time에만-알-수-있는-정보는-resolvedcallfacts로-모은다"></a>
- [call-time에만-알-수-있는-정보는-resolvedcallfacts로-모은다](docs/project/chapters/15-advanced-contracts-p04.ko.md#call-time에만-알-수-있는-정보는-resolvedcallfacts로-모은다)

<a id="logical-ir-payload와-재계산-가능한-analysis-fact를-구분한다"></a>
- [logical-ir-payload와-재계산-가능한-analysis-fact를-구분한다](docs/project/chapters/15-advanced-contracts-p04.ko.md#logical-ir-payload와-재계산-가능한-analysis-fact를-구분한다)

<a id="jsource의-memory-management-flags는-physical-ir로-바로-복사하지-않는다"></a>
- [jsource의-memory-management-flags는-physical-ir로-바로-복사하지-않는다](docs/project/chapters/15-advanced-contracts-p04.ko.md#jsource의-memory-management-flags는-physical-ir로-바로-복사하지-않는다)

<a id="verifier는-fact의-출처와-가정을-구분한다"></a>
- [verifier는-fact의-출처와-가정을-구분한다](docs/project/chapters/15-advanced-contracts-p04.ko.md#verifier는-fact의-출처와-가정을-구분한다)

<a id="이-설계를----y에-적용하면"></a>
- [이-설계를----y에-적용하면](docs/project/chapters/15-advanced-contracts-p04.ko.md#이-설계를----y에-적용하면)

<a id="구현-순서에-반영"></a>
- [구현-순서에-반영](docs/project/chapters/15-advanced-contracts-p04.ko.md#구현-순서에-반영)

<a id="411412-canonical-e2e-표본-----y"></a>
- [411412-canonical-e2e-표본-----y](docs/project/chapters/15-advanced-contracts-p04.ko.md#411412-canonical-e2e-표본-----y)

<a id="같은-표본의-stage-by-stage-compiler-trace"></a>
- [같은-표본의-stage-by-stage-compiler-trace](docs/project/chapters/15-advanced-contracts-p04.ko.md#같은-표본의-stage-by-stage-compiler-trace)

<a id="이-trace의-provenance-연결-규칙"></a>
- [이-trace의-provenance-연결-규칙](docs/project/chapters/15-advanced-contracts-p04.ko.md#이-trace의-provenance-연결-규칙)

<a id="412-access-pattern은-fusion-분석의-semantic-lower-bound다"></a>
- [412-access-pattern은-fusion-분석의-semantic-lower-bound다](docs/project/chapters/15-advanced-contracts-p05.ko.md#412-access-pattern은-fusion-분석의-semantic-lower-bound다)

<a id="413-semantic-reference-definition과-implementation을-분리한다"></a>
- [413-semantic-reference-definition과-implementation을-분리한다](docs/project/chapters/15-advanced-contracts-p05.ko.md#413-semantic-reference-definition과-implementation을-분리한다)

<a id="414-hardware-aware-ir-설계-원칙-네-층을-분리한다"></a>
- [414-hardware-aware-ir-설계-원칙-네-층을-분리한다](docs/project/chapters/15-advanced-contracts-p05.ko.md#414-hardware-aware-ir-설계-원칙-네-층을-분리한다)

<a id="415-logical-array-ir이-가져야-하는-hardware-relevant-contract"></a>
- [415-logical-array-ir이-가져야-하는-hardware-relevant-contract](docs/project/chapters/15-advanced-contracts-p05.ko.md#415-logical-array-ir이-가져야-하는-hardware-relevant-contract)

<a id="4151-iterationdomain"></a>
- [4151-iterationdomain](docs/project/chapters/15-advanced-contracts-p05.ko.md#4151-iterationdomain)

<a id="4152-axissemantics"></a>
- [4152-axissemantics](docs/project/chapters/15-advanced-contracts-p05.ko.md#4152-axissemantics)

<a id="4153-accessrelations"></a>
- [4153-accessrelations](docs/project/chapters/15-advanced-contracts-p05.ko.md#4153-accessrelations)

<a id="4154-numericsemantics"></a>
- [4154-numericsemantics](docs/project/chapters/15-advanced-contracts-p05.ko.md#4154-numericsemantics)

<a id="4154a-comparison-tolerance와-는-semantic-input이다"></a>
- [4154a-comparison-tolerance와-는-semantic-input이다](docs/project/chapters/15-advanced-contracts-p05.ko.md#4154a-comparison-tolerance와-는-semantic-input이다)

<a id="4154b-overflowpromotion은-primitive-contract의-retry-semantics다"></a>
- [4154b-overflowpromotion은-primitive-contract의-retry-semantics다](docs/project/chapters/15-advanced-contracts-p05.ko.md#4154b-overflowpromotion은-primitive-contract의-retry-semantics다)

<a id="4155-logical-dependency-requirement"></a>
- [4155-logical-dependency-requirement](docs/project/chapters/15-advanced-contracts-p05.ko.md#4155-logical-dependency-requirement)

<a id="4156-logical-invariance와-derived-target-uniformity를-분리한다"></a>
- [4156-logical-invariance와-derived-target-uniformity를-분리한다](docs/project/chapters/15-advanced-contracts-p05.ko.md#4156-logical-invariance와-derived-target-uniformity를-분리한다)

<a id="4157-symbolic-shape--divisibility-constraints"></a>
- [4157-symbolic-shape--divisibility-constraints](docs/project/chapters/15-advanced-contracts-p05.ko.md#4157-symbolic-shape--divisibility-constraints)

<a id="4158-representation-facts는-logical-semantics와-분리한다"></a>
- [4158-representation-facts는-logical-semantics와-분리한다](docs/project/chapters/15-advanced-contracts-p05.ko.md#4158-representation-facts는-logical-semantics와-분리한다)

<a id="4159-semantic-mask와-schedule-predication을-분리한다"></a>
- [4159-semantic-mask와-schedule-predication을-분리한다](docs/project/chapters/15-advanced-contracts-p05.ko.md#4159-semantic-mask와-schedule-predication을-분리한다)

<a id="41510-ssa-regionblock-control-flow"></a>
- [41510-ssa-regionblock-control-flow](docs/project/chapters/15-advanced-contracts-p05.ko.md#41510-ssa-regionblock-control-flow)

<a id="41511-constraint-witness--runtime-guard"></a>
- [41511-constraint-witness--runtime-guard](docs/project/chapters/15-advanced-contracts-p05.ko.md#41511-constraint-witness--runtime-guard)

<a id="41512-effect-resource와-ordering-token"></a>
- [41512-effect-resource와-ordering-token](docs/project/chapters/15-advanced-contracts-p05.ko.md#41512-effect-resource와-ordering-token)

<a id="41513-speculation--may-error-semantics"></a>
- [41513-speculation--may-error-semantics](docs/project/chapters/15-advanced-contracts-p05.ko.md#41513-speculation--may-error-semantics)

<a id="41514-j-error-precedence와-internal-semantic-probes를-구분한다"></a>
- [41514-j-error-precedence와-internal-semantic-probes를-구분한다](docs/project/chapters/15-advanced-contracts-p06.ko.md#41514-j-error-precedence와-internal-semantic-probes를-구분한다)

<a id="41515-j-error는-exceptional-control-flow가-될-수-있다"></a>
- [41515-j-error는-exceptional-control-flow가-될-수-있다](docs/project/chapters/15-advanced-contracts-p06.ko.md#41515-j-error는-exceptional-control-flow가-될-수-있다)

<a id="416-compilationtarget은-backend--architecture--device를-분리한다"></a>
- [416-compilationtarget은-backend--architecture--device를-분리한다](docs/project/chapters/15-advanced-contracts-p06.ko.md#416-compilationtarget은-backend--architecture--device를-분리한다)

<a id="4161-backendfamily"></a>
- [4161-backendfamily](docs/project/chapters/15-advanced-contracts-p06.ko.md#4161-backendfamily)

<a id="4162-architecturetarget"></a>
- [4162-architecturetarget](docs/project/chapters/15-advanced-contracts-p06.ko.md#4162-architecturetarget)

<a id="4163-deviceprofile"></a>
- [4163-deviceprofile](docs/project/chapters/15-advanced-contracts-p06.ko.md#4163-deviceprofile)

<a id="4164-runtimeprofile"></a>
- [4164-runtimeprofile](docs/project/chapters/15-advanced-contracts-p06.ko.md#4164-runtimeprofile)

<a id="4165-resolved-targetprofile"></a>
- [4165-resolved-targetprofile](docs/project/chapters/15-advanced-contracts-p06.ko.md#4165-resolved-targetprofile)

<a id="41651-compilation-시작-시-active-target-locale을-확정한다"></a>
- [41651-compilation-시작-시-active-target-locale을-확정한다](docs/project/chapters/15-advanced-contracts-p06.ko.md#41651-compilation-시작-시-active-target-locale을-확정한다)

<a id="41652-aot와-jit는-동일한-target-selection-contract를-사용한다"></a>
- [41652-aot와-jit는-동일한-target-selection-contract를-사용한다](docs/project/chapters/15-advanced-contracts-p06.ko.md#41652-aot와-jit는-동일한-target-selection-contract를-사용한다)

<a id="4166-target-locale-chain-j-locale-방식을-compiler-lookup에-재사용한다"></a>
- [4166-target-locale-chain-j-locale-방식을-compiler-lookup에-재사용한다](docs/project/chapters/15-advanced-contracts-p06.ko.md#4166-target-locale-chain-j-locale-방식을-compiler-lookup에-재사용한다)

<a id="4167-lowering-lookup도-locale-resolution을-사용한다"></a>
- [4167-lowering-lookup도-locale-resolution을-사용한다](docs/project/chapters/15-advanced-contracts-p06.ko.md#4167-lowering-lookup도-locale-resolution을-사용한다)

<a id="4168-capability-lookup도-locale-fragment를-합성한다"></a>
- [4168-capability-lookup도-locale-fragment를-합성한다](docs/project/chapters/15-advanced-contracts-p07.ko.md#4168-capability-lookup도-locale-fragment를-합성한다)

<a id="4169-executionhierarchy"></a>
- [4169-executionhierarchy](docs/project/chapters/15-advanced-contracts-p07.ko.md#4169-executionhierarchy)

<a id="41610-registerresources"></a>
- [41610-registerresources](docs/project/chapters/15-advanced-contracts-p07.ko.md#41610-registerresources)

<a id="41611-memoryhierarchy"></a>
- [41611-memoryhierarchy](docs/project/chapters/15-advanced-contracts-p07.ko.md#41611-memoryhierarchy)

<a id="41612-computecapabilities"></a>
- [41612-computecapabilities](docs/project/chapters/15-advanced-contracts-p07.ko.md#41612-computecapabilities)

<a id="41613-synchronization--memory-ordering--data-movement"></a>
- [41613-synchronization--memory-ordering--data-movement](docs/project/chapters/15-advanced-contracts-p07.ko.md#41613-synchronization--memory-ordering--data-movement)

<a id="41614-launchandschedulinglimits"></a>
- [41614-launchandschedulinglimits](docs/project/chapters/15-advanced-contracts-p07.ko.md#41614-launchandschedulinglimits)

<a id="41615-transferandtopology"></a>
- [41615-transferandtopology](docs/project/chapters/15-advanced-contracts-p07.ko.md#41615-transferandtopology)

<a id="41616-abi--datalayout"></a>
- [41616-abi--datalayout](docs/project/chapters/15-advanced-contracts-p07.ko.md#41616-abi--datalayout)

<a id="417-costprofile-hard-limit와-측정-성능을-분리한다"></a>
- [417-costprofile-hard-limit와-측정-성능을-분리한다](docs/project/chapters/15-advanced-contracts-p07.ko.md#417-costprofile-hard-limit와-측정-성능을-분리한다)

<a id="418-rustj-native-physical-plan이-기록하는-hardware-mapping"></a>
- [418-rustj-native-physical-plan이-기록하는-hardware-mapping](docs/project/chapters/15-advanced-contracts-p07.ko.md#418-rustj-native-physical-plan이-기록하는-hardware-mapping)

<a id="419-resourceestimate와-costestimate를-분리한다"></a>
- [419-resourceestimate와-costestimate를-분리한다](docs/project/chapters/15-advanced-contracts-p07.ko.md#419-resourceestimate와-costestimate를-분리한다)

<a id="resourceestimate"></a>
- [resourceestimate](docs/project/chapters/15-advanced-contracts-p07.ko.md#resourceestimate)

<a id="costestimate"></a>
- [costestimate](docs/project/chapters/15-advanced-contracts-p07.ko.md#costestimate)

<a id="4190-resourceestimate-derivation-invariants"></a>
- [4190-resourceestimate-derivation-invariants](docs/project/chapters/15-advanced-contracts-p07.ko.md#4190-resourceestimate-derivation-invariants)

<a id="4191-backend-compiled-resource-feedback"></a>
- [4191-backend-compiled-resource-feedback](docs/project/chapters/15-advanced-contracts-p07.ko.md#4191-backend-compiled-resource-feedback)

<a id="4192-targetprofile은-data--query-interface다"></a>
- [4192-targetprofile은-data--query-interface다](docs/project/chapters/15-advanced-contracts-p07.ko.md#4192-targetprofile은-data--query-interface다)

<a id="420-cpu와-gpu에서-실제로-필요한-정보"></a>
- [420-cpu와-gpu에서-실제로-필요한-정보](docs/project/chapters/15-advanced-contracts-p07.ko.md#420-cpu와-gpu에서-실제로-필요한-정보)

<a id="421-현재-rustj에-권장하는-최소-구현-범위"></a>
- [421-현재-rustj에-권장하는-최소-구현-범위](docs/project/chapters/15-advanced-contracts-p07.ko.md#421-현재-rustj에-권장하는-최소-구현-범위)

<a id="422-외부-compiler-조사에서-얻은-직접적인-설계-근거"></a>
- [422-외부-compiler-조사에서-얻은-직접적인-설계-근거](docs/project/chapters/15-advanced-contracts-p08.ko.md#422-외부-compiler-조사에서-얻은-직접적인-설계-근거)

<a id="423-old-analyzer-output-schema는-logicalphysical-계층으로-분해해-흡수한다"></a>
- [423-old-analyzer-output-schema는-logicalphysical-계층으로-분해해-흡수한다](docs/project/chapters/15-advanced-contracts-p08.ko.md#423-old-analyzer-output-schema는-logicalphysical-계층으로-분해해-흡수한다)

<a id="424-flowstorage-semantic-storage-requirement와-physical-materialization을-분리한다"></a>
- [424-flowstorage-semantic-storage-requirement와-physical-materialization을-분리한다](docs/project/chapters/15-advanced-contracts-p08.ko.md#424-flowstorage-semantic-storage-requirement와-physical-materialization을-분리한다)

<a id="4241-destination--alias-relation은-buffer-identity와-분리한다"></a>
- [4241-destination--alias-relation은-buffer-identity와-분리한다](docs/project/chapters/15-advanced-contracts-p08.ko.md#4241-destination--alias-relation은-buffer-identity와-분리한다)

<a id="4242-mutable-state는-primitive-내부에-숨기지-않는다"></a>
- [4242-mutable-state는-primitive-내부에-숨기지-않는다](docs/project/chapters/15-advanced-contracts-p08.ko.md#4242-mutable-state는-primitive-내부에-숨기지-않는다)

<a id="4243-apexco-dfnstail-연구를-반영한-middle-end-substrate"></a>
- [4243-apexco-dfnstail-연구를-반영한-middle-end-substrate](docs/project/chapters/15-advanced-contracts-p08.ko.md#4243-apexco-dfnstail-연구를-반영한-middle-end-substrate)

<a id="4244-morphologyengine-valuefacts를-latticefixpoint-분석으로-승격한다"></a>
- [4244-morphologyengine-valuefacts를-latticefixpoint-분석으로-승격한다](docs/project/chapters/15-advanced-contracts-p08.ko.md#4244-morphologyengine-valuefacts를-latticefixpoint-분석으로-승격한다)

<a id="4245-arraypropertyfacts와-factwitness"></a>
- [4245-arraypropertyfacts와-factwitness](docs/project/chapters/15-advanced-contracts-p08.ko.md#4245-arraypropertyfacts와-factwitness)

<a id="4246-graphindex는-semantic-dag의-replacement가-아니라-derived-analysis-view다"></a>
- [4246-graphindex는-semantic-dag의-replacement가-아니라-derived-analysis-view다](docs/project/chapters/15-advanced-contracts-p08.ko.md#4246-graphindex는-semantic-dag의-replacement가-아니라-derived-analysis-view다)

<a id="4247-specializationengine-call-site-clone-대신-cacheable-specializationkey"></a>
- [4247-specializationengine-call-site-clone-대신-cacheable-specializationkey](docs/project/chapters/15-advanced-contracts-p08.ko.md#4247-specializationengine-call-site-clone-대신-cacheable-specializationkey)

<a id="4248-high-level-parallel-structure를-logical-ir에-보존한다"></a>
- [4248-high-level-parallel-structure를-logical-ir에-보존한다](docs/project/chapters/15-advanced-contracts-p08.ko.md#4248-high-level-parallel-structure를-logical-ir에-보존한다)

<a id="4249-pure-region-extraction과-route-precondition"></a>
- [4249-pure-region-extraction과-route-precondition](docs/project/chapters/15-advanced-contracts-p08.ko.md#4249-pure-region-extraction과-route-precondition)

<a id="42410-parameterized-lowering-recipe"></a>
- [42410-parameterized-lowering-recipe](docs/project/chapters/15-advanced-contracts-p08.ko.md#42410-parameterized-lowering-recipe)

<a id="42410a-graph-basis와-execution-basis를-분리한다"></a>
- [42410a-graph-basis와-execution-basis를-분리한다](docs/project/chapters/15-advanced-contracts-p08.ko.md#42410a-graph-basis와-execution-basis를-분리한다)

<a id="42411-execution-basis-vocabulary-v02-최소성이-아니라-성능상-유효한-실행-최적화-경계"></a>
- [42411-execution-basis-vocabulary-v02-최소성이-아니라-성능상-유효한-실행-최적화-경계](docs/project/chapters/15-advanced-contracts-p08.ko.md#42411-execution-basis-vocabulary-v02-최소성이-아니라-성능상-유효한-실행-최적화-경계)

<a id="current-j-vocabulary"></a>
- [current-j-vocabulary](docs/project/chapters/15-advanced-contracts-p09.ko.md#current-j-vocabulary)

<a id="nv2-core-recognition"></a>
- [nv2-core-recognition](docs/project/chapters/15-advanced-contracts-p09.ko.md#nv2-core-recognition)

<a id="다른-array-languagecompiler-basis와의-역대조"></a>
- [다른-array-languagecompiler-basis와의-역대조](docs/project/chapters/15-advanced-contracts-p09.ko.md#다른-array-languagecompiler-basis와의-역대조)

<a id="42412-j-primitive--provisional-execution-basis-coverage-matrix"></a>
- [42412-j-primitive--provisional-execution-basis-coverage-matrix](docs/project/chapters/15-advanced-contracts-p09.ko.md#42412-j-primitive--provisional-execution-basis-coverage-matrix)

<a id="현재-vocabulary-감사--nuvoc와-구현-상태-2026-10-05"></a>
- [현재-vocabulary-감사--nuvoc와-구현-상태-2026-10-05](docs/project/chapters/15-advanced-contracts-p09.ko.md#현재-vocabulary-감사--nuvoc와-구현-상태-2026-10-05)

<a id="nv2--core-표기품사-인식과-실행-capability-분리"></a>
- [nv2--core-표기품사-인식과-실행-capability-분리](docs/project/chapters/15-advanced-contracts-p09.ko.md#nv2--core-표기품사-인식과-실행-capability-분리)

<a id="jsource-special-code-inventory와의-교차검증"></a>
- [jsource-special-code-inventory와의-교차검증](docs/project/chapters/15-advanced-contracts-p09.ko.md#jsource-special-code-inventory와의-교차검증)

<a id="42413-derived-forms-closure-check-실제-execution-basis-graph"></a>
- [42413-derived-forms-closure-check-실제-execution-basis-graph](docs/project/chapters/15-advanced-contracts-p09.ko.md#42413-derived-forms-closure-check-실제-execution-basis-graph)

<a id="rank"></a>
- [rank](docs/project/chapters/15-advanced-contracts-p09.ko.md#rank)

<a id="cut"></a>
- [cut](docs/project/chapters/15-advanced-contracts-p09.ko.md#cut)

<a id="key--oblique"></a>
- [key--oblique](docs/project/chapters/15-advanced-contracts-p09.ko.md#key--oblique)

<a id="power"></a>
- [power](docs/project/chapters/15-advanced-contracts-p09.ko.md#power)

<a id="boxed-level--spread--fetch"></a>
- [boxed-level--spread--fetch](docs/project/chapters/15-advanced-contracts-p09.ko.md#boxed-level--spread--fetch)

<a id="sparse-composition"></a>
- [sparse-composition](docs/project/chapters/15-advanced-contracts-p09.ko.md#sparse-composition)

<a id="closure-판정"></a>
- [closure-판정](docs/project/chapters/15-advanced-contracts-p09.ko.md#closure-판정)

<a id="42414-execution-basis-logical-ir-node-contract와-최소-analysislowering-interface"></a>
- [42414-execution-basis-logical-ir-node-contract와-최소-analysislowering-interface](docs/project/chapters/15-advanced-contracts-p09.ko.md#42414-execution-basis-logical-ir-node-contract와-최소-analysislowering-interface)

<a id="resolvedinstantiation"></a>
- [resolvedinstantiation](docs/project/chapters/15-advanced-contracts-p09.ko.md#resolvedinstantiation)

<a id="valuerolefacts"></a>
- [valuerolefacts](docs/project/chapters/15-advanced-contracts-p09.ko.md#valuerolefacts)

<a id="first-class-semanticcheck"></a>
- [first-class-semanticcheck](docs/project/chapters/15-advanced-contracts-p09.ko.md#first-class-semanticcheck)

<a id="executionbasisexpansion"></a>
- [executionbasisexpansion](docs/project/chapters/15-advanced-contracts-p10.ko.md#executionbasisexpansion)

<a id="공통-logicalbasisop-contract"></a>
- [공통-logicalbasisop-contract](docs/project/chapters/15-advanced-contracts-p10.ko.md#공통-logicalbasisop-contract)

<a id="lowering-capability-공통-형태"></a>
- [lowering-capability-공통-형태](docs/project/chapters/15-advanced-contracts-p10.ko.md#lowering-capability-공통-형태)

<a id="execution-basis별-최소-node-contract"></a>
- [execution-basis별-최소-node-contract](docs/project/chapters/15-advanced-contracts-p10.ko.md#execution-basis별-최소-node-contract)

<a id="execution-basis-간-composition-rule"></a>
- [execution-basis-간-composition-rule](docs/project/chapters/15-advanced-contracts-p10.ko.md#execution-basis-간-composition-rule)

<a id="a3-v0--v1-구현-범위"></a>
- [a3-v0--v1-구현-범위](docs/project/chapters/15-advanced-contracts-p10.ko.md#a3-v0--v1-구현-범위)

<a id="현재-a3-direct-lowering-구현-경계"></a>
- [현재-a3-direct-lowering-구현-경계](docs/project/chapters/15-advanced-contracts-p10.ko.md#현재-a3-direct-lowering-구현-경계)

<a id="basis-contract-검증"></a>
- [basis-contract-검증](docs/project/chapters/15-advanced-contracts-p10.ko.md#basis-contract-검증)

<a id="14-jsource와-rustj의-관계"></a>
- [14-jsource와-rustj의-관계](docs/project/chapters/16-jsource.ko.md#14-jsource와-rustj의-관계)

<a id="15-문서-정책"></a>
- [15-문서-정책](docs/project/chapters/17-documentation-policy.ko.md#15-문서-정책)

<a id="151-권위-문서"></a>
- [151-권위-문서](docs/project/chapters/17-documentation-policy.ko.md#151-권위-문서)

<a id="152-readme"></a>
- [152-readme](docs/project/chapters/17-documentation-policy.ko.md#152-readme)

<a id="153-reports"></a>
- [153-reports](docs/project/chapters/17-documentation-policy.ko.md#153-reports)

<a id="154-이번-통합에서-흡수한-기존-문서군"></a>
- [154-이번-통합에서-흡수한-기존-문서군](docs/project/chapters/17-documentation-policy.ko.md#154-이번-통합에서-흡수한-기존-문서군)

<a id="155-외부-역사-저장소"></a>
- [155-외부-역사-저장소](docs/project/chapters/17-documentation-policy.ko.md#155-외부-역사-저장소)

<a id="1551-current-jsource-semantic-cross-check-2026-09-30"></a>
- [1551-current-jsource-semantic-cross-check-2026-09-30](docs/project/chapters/17-documentation-policy.ko.md#1551-current-jsource-semantic-cross-check-2026-09-30)

<a id="156-설계-일관성-불변식"></a>
- [156-설계-일관성-불변식](docs/project/chapters/17-documentation-policy.ko.md#156-설계-일관성-불변식)

<a id="157-문서-완전성-감사--stage-contract가-닫혀-있는가-2026-10-06"></a>
- [157-문서-완전성-감사--stage-contract가-닫혀-있는가-2026-10-06](docs/project/chapters/17-documentation-policy.ko.md#157-문서-완전성-감사--stage-contract가-닫혀-있는가-2026-10-06)

<a id="1571-1차-감사에서-닫은-문서-계약"></a>
- [1571-1차-감사에서-닫은-문서-계약](docs/project/chapters/17-documentation-policy.ko.md#1571-1차-감사에서-닫은-문서-계약)

<a id="1572-2차-감사에서-닫은-문서-계약"></a>
- [1572-2차-감사에서-닫은-문서-계약](docs/project/chapters/17-documentation-policy.ko.md#1572-2차-감사에서-닫은-문서-계약)

<a id="1573-현재-남은-빈칸--implementation-driven-detail"></a>
- [1573-현재-남은-빈칸--implementation-driven-detail](docs/project/chapters/17-documentation-policy.ko.md#1573-현재-남은-빈칸--implementation-driven-detail)

<a id="1574-문서-유지-규칙"></a>
- [1574-문서-유지-규칙](docs/project/chapters/17-documentation-policy.ko.md#1574-문서-유지-규칙)

<a id="16-다음-작업--정본-체크리스트로-이동"></a>
- [16-다음-작업--정본-체크리스트로-이동](docs/project/chapters/18-next-tasks.ko.md#16-다음-작업--정본-체크리스트로-이동)

<a id="라이선스-정책"></a>
- [라이선스-정책](docs/project/chapters/19-license.ko.md#라이선스-정책)

