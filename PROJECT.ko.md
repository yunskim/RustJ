[English](PROJECT.md) | **한국어 — 정본(canonical)**

# RustJ 프로젝트 문서 — 개요와 목차

RustJ는 J 언어의 관찰 가능한 의미론을 보존하면서 CPU·GPU 등 다양한 경로로 실행할 수 있는 배열 컴파일러를 목표로 합니다.

**문서 정본 구조:** 이 페이지는 진입점이며, 상세 설계·구현 상태·체크리스트는 아래 연결된 `docs/project/chapters/*.ko.md`가 주제별 정본입니다. 영어 문서 [PROJECT.md](PROJECT.md)는 별도 분할 전까지 기존 통합 영어 문서를 유지합니다. [FOUNDATIONS.ko.md](FOUNDATIONS.ko.md)는 필수 설계 기반 문서입니다.

- [문서 분할 계획](docs/project/MODULARIZATION_PLAN.ko.md) · [실행 체크리스트 #64](https://github.com/yunskim/RustJ/issues/64)
- [Agent 작업·문서 규약](AGENTS.md) · [프로젝트 README](README.ko.md)
- [라이선스](LICENSE) · [기여자 계약](CLA.ko.md)

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

- [architecture-migration-checklist](./docs/project/chapters/10-implementation-scope.ko.md#architecture-migration-checklist)
- [dynamic-boundary-checklist](./docs/project/chapters/11-roadmap-checklists-p01.ko.md#dynamic-boundary-checklist)
- [nn-extension-checklist](./docs/project/chapters/15-advanced-contracts-p01.ko.md#nn-extension-checklist)
- [framework-migration-checklist](./docs/project/chapters/14-framework-research-p02.ko.md#framework-migration-checklist)
- [jsource-optimization-migration](./docs/project/chapters/14-framework-research-p03.ko.md#jsource-optimization-migration)
- [validation-policy](./docs/project/chapters/11-roadmap-checklists-p05.ko.md#validation-policy)
- [current-implementation-status](./docs/project/chapters/12-validation.ko.md#current-implementation-status)
- [out-of-core-io-checklist](./docs/project/chapters/11-roadmap-checklists-p05.ko.md#out-of-core-io-checklist)
- [formal-ir-verification-checklist](./docs/project/chapters/11-roadmap-checklists-p05.ko.md#formal-ir-verification-checklist)
- [extension-primitive-inventory](./docs/project/chapters/15-advanced-contracts-p01.ko.md#extension-primitive-inventory)

## 이전 문서의 앵커 링크 호환

분할 이전의 `PROJECT.ko.md#...` 링크를 위해 중요 앵커를 아래에서 신규 파일로 연결합니다. 신규 링크에는 상세 문서의 직접 URL을 사용하세요.

<a id="a15--j-graph-ir--jaxa-array-operation-graph-ir"></a>
- [a15--j-graph-ir--jaxa-array-operation-graph-ir](./docs/project/chapters/11-roadmap-checklists-p04.ko.md#a15--j-graph-ir--jaxa-array-operation-graph-ir)
<a id="a2--extension-primitive-registry와-analysis-contract"></a>
- [a2--extension-primitive-registry와-analysis-contract](./docs/project/chapters/11-roadmap-checklists-p05.ko.md#a2--extension-primitive-registry와-analysis-contract)
<a id="algorithm-planning-migration"></a>
- [algorithm-planning-migration](./docs/project/chapters/14-framework-research-p02.ko.md#algorithm-planning-migration)
<a id="architecture-migration-checklist"></a>
- [architecture-migration-checklist](./docs/project/chapters/10-implementation-scope.ko.md#architecture-migration-checklist)
<a id="array-compiler-static-analysis"></a>
- [array-compiler-static-analysis](./docs/project/chapters/15-advanced-contracts-p02.ko.md#array-compiler-static-analysis)
<a id="current-implementation-status"></a>
- [current-implementation-status](./docs/project/chapters/12-validation.ko.md#current-implementation-status)
<a id="current-j-vocabulary"></a>
- [current-j-vocabulary](./docs/project/chapters/15-advanced-contracts-p09.ko.md#current-j-vocabulary)
<a id="dynamic-boundary-checklist"></a>
- [dynamic-boundary-checklist](./docs/project/chapters/11-roadmap-checklists-p01.ko.md#dynamic-boundary-checklist)
<a id="dynamic-semantic-boundaries"></a>
- [dynamic-semantic-boundaries](./docs/project/chapters/04-semantic-array-ir-p04.ko.md#dynamic-semantic-boundaries)
<a id="extension-primitive-inventory"></a>
- [extension-primitive-inventory](./docs/project/chapters/15-advanced-contracts-p01.ko.md#extension-primitive-inventory)
<a id="formal-ir-verification-checklist"></a>
- [formal-ir-verification-checklist](./docs/project/chapters/11-roadmap-checklists-p05.ko.md#formal-ir-verification-checklist)
<a id="framework-graph-hints"></a>
- [framework-graph-hints](./docs/project/chapters/05-graph-execution-lowering-p02.ko.md#framework-graph-hints)
<a id="framework-migration-checklist"></a>
- [framework-migration-checklist](./docs/project/chapters/14-framework-research-p02.ko.md#framework-migration-checklist)
<a id="gf2-composition-review"></a>
- [gf2-composition-review](./docs/project/chapters/11-roadmap-checklists-p04.ko.md#gf2-composition-review)
<a id="gf3-scan-identity"></a>
- [gf3-scan-identity](./docs/project/chapters/11-roadmap-checklists-p04.ko.md#gf3-scan-identity)
<a id="gf4-fusion-registry"></a>
- [gf4-fusion-registry](./docs/project/chapters/11-roadmap-checklists-p04.ko.md#gf4-fusion-registry)
<a id="gf5-work-depth"></a>
- [gf5-work-depth](./docs/project/chapters/11-roadmap-checklists-p04.ko.md#gf5-work-depth)
<a id="gf6a-fusion-readiness"></a>
- [gf6a-fusion-readiness](./docs/project/chapters/11-roadmap-checklists-p04.ko.md#gf6a-fusion-readiness)
<a id="graph-prior-art-followup"></a>
- [graph-prior-art-followup](./docs/project/chapters/11-roadmap-checklists-p04.ko.md#graph-prior-art-followup)
<a id="heterogeneous-execution-checklist"></a>
- [heterogeneous-execution-checklist](./docs/project/chapters/11-roadmap-checklists-p01.ko.md#heterogeneous-execution-checklist)
<a id="index-family-roadmap"></a>
- [index-family-roadmap](./docs/project/chapters/14-framework-research-p02.ko.md#index-family-roadmap)
<a id="io-a-source-audit"></a>
- [io-a-source-audit](./docs/project/chapters/09-memory-alias.ko.md#io-a-source-audit)
<a id="io-framework-execution-comparison"></a>
- [io-framework-execution-comparison](./docs/project/chapters/09-memory-alias.ko.md#io-framework-execution-comparison)
<a id="j-graph-implementation-checklist"></a>
- [j-graph-implementation-checklist](./docs/project/chapters/11-roadmap-checklists-p04.ko.md#j-graph-implementation-checklist)
<a id="j-graph-opportunities"></a>
- [j-graph-opportunities](./docs/project/chapters/05-graph-execution-lowering-p01.ko.md#j-graph-opportunities)
<a id="jsource-audit-m"></a>
- [jsource-audit-m](./docs/project/chapters/14-framework-research-p02.ko.md#jsource-audit-m)
<a id="jsource-index-family"></a>
- [jsource-index-family](./docs/project/chapters/14-framework-research-p02.ko.md#jsource-index-family)
<a id="jsource-optimization-migration"></a>
- [jsource-optimization-migration](./docs/project/chapters/14-framework-research-p03.ko.md#jsource-optimization-migration)
<a id="logical-physical-array-model"></a>
- [logical-physical-array-model](./docs/project/chapters/06-plans-executor-p02.ko.md#logical-physical-array-model)
<a id="mean-proof-example"></a>
- [mean-proof-example](./docs/project/chapters/15-advanced-contracts-p04.ko.md#mean-proof-example)
<a id="nn-extension-checklist"></a>
- [nn-extension-checklist](./docs/project/chapters/15-advanced-contracts-p01.ko.md#nn-extension-checklist)
<a id="noun-reduction-capture"></a>
- [noun-reduction-capture](./docs/project/chapters/11-roadmap-checklists-p02.ko.md#noun-reduction-capture)
<a id="nv2-core-recognition"></a>
- [nv2-core-recognition](./docs/project/chapters/15-advanced-contracts-p09.ko.md#nv2-core-recognition)
<a id="nv3a-spelling-errors"></a>
- [nv3a-spelling-errors](./docs/project/chapters/11-roadmap-checklists-p01.ko.md#nv3a-spelling-errors)
<a id="nv3b-name-syntax"></a>
- [nv3b-name-syntax](./docs/project/chapters/11-roadmap-checklists-p01.ko.md#nv3b-name-syntax)
<a id="nv3c-name-limits"></a>
- [nv3c-name-limits](./docs/project/chapters/11-roadmap-checklists-p01.ko.md#nv3c-name-limits)
<a id="nv3d1-numeric-recognition"></a>
- [nv3d1-numeric-recognition](./docs/project/chapters/11-roadmap-checklists-p01.ko.md#nv3d1-numeric-recognition)
<a id="nv3d2a-quad-hex-recognition"></a>
- [nv3d2a-quad-hex-recognition](./docs/project/chapters/11-roadmap-checklists-p01.ko.md#nv3d2a-quad-hex-recognition)
<a id="nv3d2b1-rounding-nan-words"></a>
- [nv3d2b1-rounding-nan-words](./docs/project/chapters/11-roadmap-checklists-p01.ko.md#nv3d2b1-rounding-nan-words)
<a id="nv3d2b2a-exact-hex-ratios"></a>
- [nv3d2b2a-exact-hex-ratios](./docs/project/chapters/11-roadmap-checklists-p01.ko.md#nv3d2b2a-exact-hex-ratios)
<a id="out-of-core-io-checklist"></a>
- [out-of-core-io-checklist](./docs/project/chapters/11-roadmap-checklists-p05.ko.md#out-of-core-io-checklist)
<a id="out-of-core-io-contract"></a>
- [out-of-core-io-contract](./docs/project/chapters/09-memory-alias.ko.md#out-of-core-io-contract)
<a id="preexecution-input-information"></a>
- [preexecution-input-information](./docs/project/chapters/15-advanced-contracts-p01.ko.md#preexecution-input-information)
<a id="rank-cellapply-followups"></a>
- [rank-cellapply-followups](./docs/project/chapters/14-framework-research-p03.ko.md#rank-cellapply-followups)
<a id="rank-graph-zero-frame"></a>
- [rank-graph-zero-frame](./docs/project/chapters/14-framework-research-p03.ko.md#rank-graph-zero-frame)
<a id="read-advanced"></a>
- [read-advanced](./docs/project/chapters/14-framework-research-p03.ko.md#read-advanced)
<a id="read-architecture"></a>
- [read-architecture](./docs/project/chapters/02-project-goal.ko.md#read-architecture)
<a id="read-goal"></a>
- [read-goal](./docs/project/chapters/01-introduction.ko.md#read-goal)
<a id="read-graph-execution"></a>
- [read-graph-execution](./docs/project/chapters/04-semantic-array-ir-p04.ko.md#read-graph-execution)
<a id="read-intro"></a>
- [read-intro](./docs/project/chapters/01-introduction.ko.md#read-intro)
<a id="read-physical"></a>
- [read-physical](./docs/project/chapters/05-graph-execution-lowering-p02.ko.md#read-physical)
<a id="read-references"></a>
- [read-references](./docs/project/chapters/13-current-status.ko.md#read-references)
<a id="read-roadmap"></a>
- [read-roadmap](./docs/project/chapters/10-implementation-scope.ko.md#read-roadmap)
<a id="read-scope"></a>
- [read-scope](./docs/project/chapters/09-memory-alias.ko.md#read-scope)
<a id="read-semantic"></a>
- [read-semantic](./docs/project/chapters/03-architecture-p02.ko.md#read-semantic)
<a id="read-status"></a>
- [read-status](./docs/project/chapters/12-validation.ko.md#read-status)
<a id="read-validation"></a>
- [read-validation](./docs/project/chapters/11-roadmap-checklists-p05.ko.md#read-validation)
<a id="static-frontend-review"></a>
- [static-frontend-review](./docs/project/chapters/11-roadmap-checklists-p03.ko.md#static-frontend-review)
<a id="sw-mlpl-reference-checklist"></a>
- [sw-mlpl-reference-checklist](./docs/project/chapters/03-architecture-p02.ko.md#sw-mlpl-reference-checklist)
<a id="syntax-graph-hints"></a>
- [syntax-graph-hints](./docs/project/chapters/05-graph-execution-lowering-p01.ko.md#syntax-graph-hints)
<a id="unified-data-movement-contract"></a>
- [unified-data-movement-contract](./docs/project/chapters/09-memory-alias.ko.md#unified-data-movement-contract)
<a id="validation-policy"></a>
- [validation-policy](./docs/project/chapters/11-roadmap-checklists-p05.ko.md#validation-policy)
<a id="vocabulary-migration-checklist"></a>
- [vocabulary-migration-checklist](./docs/project/chapters/11-roadmap-checklists-p01.ko.md#vocabulary-migration-checklist)

> 주의: 원래 파일의 모든 자동 생성 Heading 앵커까지 보존하는 것은 아닙니다. 외부 deep link는 위 이전표 또는 신규 문서 목차로 갱신해야 합니다.
