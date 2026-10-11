# PROJECT 문서 모듈화 계획 및 검증 체크리스트

> 상태: **한국어 문서 링크 기반 분할 완료·영어 미러 이전 대기** (2026-10-11). `PROJECT.ko.md`는 40개 모듈의 목차로 전환했으며, `PROJECT.md` 영어 통합본은 그대로 유지한다. 구현/CI 수용 상태는 바뀌지 않는다.
> 실행 이슈: [#64 — PROJECT 문서 분할](https://github.com/yunskim/RustJ/issues/64). 이 문서가 구조 전환 계획의 한국어 정본이고, 이슈는 단계별 실행 증거와 상태 추적용이다.

## 목표와 정본

- `PROJECT.ko.md`와 `PROJECT.md`의 루트 경로는 보존하되 **짧은 프로젝트 개요 + 목차 + 중요 진입 링크**로 축소한다.
- 실제 설계 계약, 근거, 로드맵, 검증 정책, 체크리스트는 `docs/project/` 아래 **주제별 파일**에 둔다. `*.ko.md`가 정본이며 같은 상대경로의 `*.md`가 영어 미러이다.
- 파일 크기보다 **한 파일 한 책임**을 우선하며 200–400행을 권장 크기로 삼는다(강제 상한은 아님). 한 주제의 상호의존된 계약/테이블/검증 근거는 임의로 쪼개지 않는다.
- GitHub Markdown의 상대경로 링크를 사용한다. 런타임 `include`/embed, 별도 사이트 생성기, 원문을 복제한 거대 통합본을 정본으로 사용하지 않는다.
- `FOUNDATIONS.ko.md`는 설계 정합성의 필수 기준으로 유지하고, `LICENSE`/`COPYING`은 라이선스 정본으로 유지한다. 중복된 정책 내용을 다른 곳에서 별도 규정하지 않는다.
- **과도기 원칙:** 실제 이전을 완료하지 않은 항목은 기존 `PROJECT.ko.md`가 여전히 정본이다. 이전한 항목만 새 경로가 정본이며, 루트 문서는 최종 전환 전까지 섹션별 상태를 분명히 표시한다.

## 확인된 시작점

| 항목 | 2026-10-11 `main` 조사 결과 |
| --- | --- |
| 한국어 프로젝트 문서 | `PROJECT.ko.md`: 1,516,273 bytes |
| 영어 프로젝트 문서 | `PROJECT.md`: 945,743 bytes, 약 6,244행 |
| 기존 문서 구조 | 별도의 주제별 `docs/project/` Markdown 모듈 없음 |
| 기존 규약 | `AGENTS.md`가 단일 `PROJECT.ko.md` 정본과 새 설계/상태 파일 금지를 규정 |
| 주의 사항 | 두 대형 문서의 전체 섹션 및 체크박스 1:1 인벤토리는 아직 미완료 |

## 목적지 설계(초안; P1에서 소스 구간 매핑 후 확정)

| 기존 내용 | 예상 한국어 정본 목적지(영어 `*.md` 쌍 생성) |
| --- | --- |
| 서론, 목표, 상위 아키텍처, route boundary | `docs/project/architecture/{overview,route-boundaries}.ko.md` |
| Part I — Frontend 호환성 및 NAME/파서/정의 | `docs/project/frontend/{pipeline,names-and-definitions}.ko.md` |
| Part II — FunctionEntity/JEntity/extension | `docs/project/semantics/{entities,extension-primitives}.ko.md` |
| Part III — Rank, CellApply, Logical/Physical Array | `docs/project/semantics/{rank-and-cells,array-model}.ko.md` |
| Part IV — J Graph IR와 graph basis | `docs/project/ir/graph.ko.md` |
| Part V–VII — rewrite, facts, resources | `docs/project/optimization/{rewrites,facts-and-resources}.ko.md` |
| Part VIII–X — Logical Execution, targets, storage | `docs/project/execution/{logical-lowering,target-planning,flow-and-storage}.ko.md` |
| Part XI — validation, differential, schema | `docs/project/validation/policy.ko.md` |
| Part XII–XIII — 지원 상태, 단계별 작업, roadmap | `docs/project/status/{implementation,roadmap,checklists}.ko.md` (필요시 주제별 재분리) |
| Part XIV — JAXA 역사와 source/optimization 연구 | `docs/project/research/{jaxa-history,optimization-sources}.ko.md` |
| Part XV — 문서/라이선스 관련 정책 | 기존 `AGENTS.md`, `LICENSE`, `FOUNDATIONS.ko.md` 소유권과 조정; 필요한 설명만 링크 |

위 경로는 제안이며, **기존 절/체크리스트를 실제로 목록화한 뒤 경계를 확정**한다. 서로 다른 목적으로 동일 기술 계약을 반복하지 않는다.

## 실행 단계

### P0 — 기준 인벤토리
- [x] GitHub `main`에서 두 루트 파일과 현재 문서 규약의 크기/구조 확인
- [x] 기존 단일 정본 및 새 문서 생성 금지 규약 확인
- [ ] 이전 시작 직전 `main` SHA, 양 문서 blob SHA, 전체 섹션 트리 동결
- [ ] 한국어와 영어의 제목, 코드 블록, 표, 체크박스, 링크, 각주/출처를 독립적으로 전수 기록
- [ ] `README`, `AGENTS`, `FOUNDATIONS`, 기타 코드/도구에서 기존 파일/anchor를 참조하는 위치 수집

### P1 — 원문 소유권과 매핑
- [ ] 모든 이전 단위에 `원본 파일/제목/anchor/행 범위 → 신규 파일/anchor` 매핑 부여
- [ ] 각 체크리스트의 **단일 소유자** 및 연관 구현 gate 설정; 이미 완료된 항목도 근거 링크 유지
- [ ] 한영 간 누락·순서·상태 불일치를 기록하고, 의미상 변경은 별도 검토로 분리
- [ ] 공통 내용은 `FOUNDATIONS` 등 기존 정본과 대조하여 링크로 대체할 중복 선정

### P2 — 규약 및 기본 뼈대
- [ ] `AGENTS.md`에 루트 인덱스 + `docs/project/**/*.ko.md`의 과도기/최종 소유권 규약 반영
- [ ] 네이밍, 언어 쌍, 문서 링크, 체크리스트 업데이트 책임 및 목차 소유자 고정
- [ ] 계획 파일의 한영 미러 작성 및 저장소 PR에서 검토

### P3 — 의미 변경 없는 단계적 분할
- [x] 한국어 `PROJECT.ko.md`의 14,498줄을 40개 링크형 상세 문서로 분리 (commit `b3bea373f4ff00d42225603932a0df531e60a56b`)
- [ ] 영어 `PROJECT.md`의 주제별 미러 분리 및 한영 정합성 검증
- [ ] 아키텍처/Frontend/NAME/JEntity **한영 쌍** 정리
- [ ] Rank·배열 의미론·NN extension **한영 쌍** 정리
- [ ] Graph IR·rewrite·resource·Logical/Physical 실행 **한영 쌍** 정리
- [ ] 검증 정책·현재 구현 상태·활성 체크리스트 **한영 쌍** 정리
- [ ] JAXA/연구/출처/이력 **한영 쌍** 정리
- [x] 한국어 루트 `PROJECT.ko.md`를 링크 중심 인덱스로 전환 (257줄)
- [ ] 영어 루트 `PROJECT.md`를 링크 중심 인덱스로 전환

### P4 — 검증, 통합, 정본 전환
- [ ] 원본 대비 모든 제목·표·코드 블록·체크박스·참조·인용의 보존 증거 생성
- [x] 한국어 모듈/인덱스 내부 파일 링크 검사: 363개 참조의 로컬 경로 모두 존재 (신규 branch head 기준)
- [x] 한국어 기존 명시 앵커 63개에 대한 루트 이동 링크 생성
- [ ] 자동 생성 heading 앵커 및 repo-wide inbound 링크 전체 호환성 검증; 한영 링크 검사
- [ ] 한영 미러의 파일/제목/상태/체크박스 의미를 대조
- [ ] 문서 링크 및 구조 검증을 재실행 가능한 스크립트/CI로 추가 (기존 CI 방침과 충돌하지 않도록)
- [ ] PR 최신 head 검증 후 병합; 병합된 `main` 다시 확인하여 정본 전환 완료 선언

## 검증 기준 (필수)

1. **무손실:** 코드 예제, 표, 각주, 출처 링크, 부정 조건, 실패/미구현 상태, 체크리스트의 `[x]`/`[ ]`를 모두 보존한다. 날짜 붙은 역사 기록을 현재 구현 완료로 해석하지 않는다.
2. **섹션 매핑:** 원본 섹션마다 정확히 하나의 목적지 또는 '기존 다른 정본으로 참조' 사유가 있다. 누락은 0건이고 의도하지 않은 중복도 0건이어야 한다.
3. **경계 보존:** J semantics/Rank/NAME/effects/IR 단계 분리/CPU·GPU route 및 native/external 경로의 규범적 의미를 변경하지 않는다.
4. **링크:** 모든 루트·모듈 상호 링크가 GitHub에서 작동해야 한다. 외부 deep link는 GitHub Markdown의 자동 redirect가 없으므로 경로·anchor 영향과 호환 처리/제약을 명시한다.
5. **한영 정합성:** 한국어 정본과 영어 미러에서 모든 필수 게이트, 진행 상태 및 부정 제약이 대응한다. 원래 있던 차이는 별도 이슈로 기록한다.
6. **측정:** 검사한 정확한 commit SHA 및 PASS/FAIL/UNRUN을 이슈 #64에 기록한다. 문서 변경만으로 Rust 테스트, CI 또는 semantic validation을 PASS 처리하지 않는다.

## 반복 루프

`inventory → slice 선택 → source/destination mapping → 한영 파일 이동 → 링크/보존 검사 → PR 검토 → 기준 main 재확인 → 이슈 #64 체크 → 다음 slice`.

분할을 위해 도중에 설계를 재작성하지 않는다. 새 기능·NN extension의 구현 상태는 원래 소유한 구현/검증 체크리스트가 계속 결정한다.
