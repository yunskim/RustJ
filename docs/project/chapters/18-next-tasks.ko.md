[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)

> 문서 분할 안내: 기존 `PROJECT.ko.md`의 내용을 이동한 모듈입니다. 원문의 단일 파일 정본·신규 Markdown 금지 문구는 최신 [Agent 문서 규약](../../../AGENTS.md)보다 우선하지 않습니다.

## 16. 다음 작업 — 정본 체크리스트로 이동

작업 순서는 [§10 M0–M6](./10-implementation-scope.ko.md#architecture-migration-checklist)만 유지하고, 현재 구현 상태는 [§12](./12-validation.ko.md#current-implementation-status)를 따른다. 이 절에 별도의 Proof Slice 완료표나 과거 단계별 상태표를 복제하지 않는다.

1. **M2/frontend**: F0–F2/P0–P8의 jsource-compatible construction/name/assignment cutover를 완료한다. M1 canonical Logical IR cutover는 완료 상태다.
2. **M3/배열 경계**: logical value와 physical representation의 code/API 분리를 수렴시킨다.
3. **M4/CPU vertical slice**: verified Logical IR → 최소 Schedule/Physical Plan → CPU Physical Executor를 연결한다. [matrix mean 표본](./15-advanced-contracts-p04.ko.md#mean-proof-example)은 implicit cell semantics를 검증하며 analyzer smoke test만으로 실행 완료를 판정하지 않는다.
4. **M5–M6**: 그 뒤 route/schedule/resource/cost 선택과 검증된 external adapter를 확장한다. 실제 CUDA 구현은 사용자가 재개하기 전까지 보류한다.
5. **JE0–JE6 보조 트랙**: JE0과 최소 boundary `JEntity` seam(JE1/JE2 일부)은 M2와 병행할 수 있다. generic higher-order collection과 storage-affecting migration은 M2/M3 안정화 뒤로 미룬다. 이 트랙은 M4 첫 CPU vertical slice의 선행 조건이 아니다.

6. **IO-01~IO-30 병행·후속 트랙:** [§8.5 I/O 설계 계약](./09-memory-alias.ko.md#out-of-core-io-contract)과 [§10 IO 체크리스트](./11-roadmap-checklists-p05.ko.md#out-of-core-io-checklist)를 기준으로, M2 중에는 IO-01·IO-25의 source audit와 IO-02의 효과/오류 계약 검토만 병행한다. 동기 I/O baseline(IO-05~08, IO-26~28)은 첫 native CPU vertical slice 이후, 불필요한 읽기 제거와 cache(IO-09~12, IO-29)는 의미 증명 이후, async/transfer(IO-13~20)는 그다음, 비교/확장(IO-21~24, IO-30)은 실측으로 수용한다. **문서 계획을 구현 완료로 표시하지 않는다.**

장기 architecture의 full TargetProfile/mixed-route/async 모델이나 확장 primitive 전체를 첫 CPU slice의 선행 조건으로 삼지 않는다. 해당 의미를 최적화 대상으로 열 때는 [§11의 검증 정책](./11-roadmap-checklists-p05.ko.md#validation-policy)과 대응 semantic golden을 먼저 충족한다.

---
