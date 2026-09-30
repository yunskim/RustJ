# RustJ 실행 계획 및 완료 체크리스트

기준일: 2026-09-30. 구현 계획의 진행 상태는 이 문서에서 관리한다.
설계 기준: [구현 계획](IMPLEMENTATION-PLAN.md), [컴파일러 아키텍처](COMPILER-ARCHITECTURE.md).

## 운영 원칙

- C 커널을 실행 fallback으로 사용하지 않는 Rust compiler/runtime가 최종 목표다. C는 의미·성능 비교 기준으로만 사용한다.
- 이 컴퓨터에서는 Windows 네이티브 도구로만 테스트·검증한다. 과거 WSL 기록은 현재 검증과 구분한다.
- GitHub CI는 실행·추가하지 않는다. 로컬 검증을 단계별 완료 조건으로 삼는다.
- CUDA는 주요 목표이지만 구현은 재개 요청까지 보류한다. CPU 설계에서 논리 배열과 장치·물리 저장 표현을 분리한다.
- `[x]`는 구현과 기록된 검증 근거가 있는 항목이다. 일부만 구현되었으면 큰 항목을 완료 처리하지 않는다.
- 각 작업 종료 시 체크 상태, 변경 파일, 테스트 명령·환경·결과, 남은 실패를 갱신한다. 계획 변경만으로 구현 완료 표시를 하지 않는다.

## 최우선 작업 — GPU 친화적 배열 설계 (2026-09-30)

사용자 요청에 따라 아래 순서가 이전 P0~P5와 함수 정의 DEF 계획보다 우선한다.
세부 항목과 검증 근거는 [GPU 배열 계획·체크리스트](GPU-FRIENDLY-ARRAY-PLAN.md)에서 관리한다.

- [x] G1: 논리 값/물리 버퍼 분리와 checked PhysicalArray descriptor. [Windows 검증 기록](PHYSICAL-ARRAY-G1.md).
- [ ] G2: transpose/reverse/slice/compatible reshape의 strided view와 materialization.
- [ ] G3: CPU kernel 및 rank 실행에 연결, SIMD·alias·승격 검증.
- [ ] G4: 최소 PhysicalPlan 및 CPU executor 연결.
- [ ] G5: Windows 회귀·성능·복사/할당 비용 판정 및 지원표.

추가 설계 근거: [배열 프레임워크 조사](ARRAY-FRAMEWORK-DESIGN-REVIEW.md). D1~D5의 구체 검증 항목도 GPU 배열 체크리스트에서 추적한다.

[세 관점 설계 검토](GPU-ARRAY-DESIGN-AUDIT.md)의 A1~C4 수정 결정을 G1~G5에 반영했다. G1은 CPU standalone API와 Windows 검증을 완료했고 연산·물리 실행기 연결은 미완료다.

바로 다음 작업은 G2다. CUDA 실행 구현은 계속 보류한다.
아래 이전 단계의 체크 상태는 이력이며 실행 순서를 덮어쓰지 않는다.

## 확인된 완료 항목

- [x] 16-state word scanner 및 token byte span 구현 (`src/scanner.rs`, `src/syntax.rs`). C word 비교 기록은 `LEXER-AUDIT.md`에 있다.
- [x] 공통 Semantic IR parser 및 표현식·verb·대입 대상 span 구현 (`src/semantic.rs`).
- [x] Engine 내 이름별 대입 버전과 실행 없는 binding snapshot 구현. 완성된 SSA나 재사용 실행 계획은 아니다.
- [x] 기본/참조 evaluator의 파서 통합, literal 소유권 이동, 기본 경로 출력 버퍼 재사용 유지 (`src/runtime.rs`).
- [x] 성공한 대입만 값·버전을 갱신하고 실패 시 값·별칭 보존 (`tests/semantic.rs`).
- [x] primitive의 보수적 계약 기초와 Unknown 최적화 차단 (`src/contracts.rs`). 정밀 계약은 미완료다.
- [x] CPU 배열·rank 및 i./i:/I./e./E. 등 현재 지원 부분집합의 회귀 검증 기반 구축.
- [x] 이전 통합 변경의 Windows/WSL 기본·portable 테스트 및 C 비교 완료 기록 확인.

마지막 기록: 각 Rust 구성에서 일반 테스트 38개 + doctest 1개, Python 3개. C 2종 × Rust 2종 × 평가 모드 2종에서 조합당 1,915문장. j64는 1,914 일치와 기존 dtype 차이 1건, j64avx2는 1,915 일치. 이는 이전 실행 기록이며 이번 문서 작성 중 테스트를 재실행한 결과가 아니다. 두 평가 모드는 parser/evaluator를 공유하므로 독립 의미 검증이 아니다.

## P0 — 알려진 프런트엔드 결함 해결 (GPU 배열 단계 이후)

- [ ] `(1 2+1 2 3)+missing`의 C length error / Rust value error 차이를 최소 회귀 사례로 고정한다.
- [x] C parser의 이름 해석·품사 판정·오류 우선순위를 조사하고 noun/verb/unresolved name의 지원 경계를 정했다. 근거: NAME-SEMANTICS-AUDIT.md와 audit_name_semantics.py. 함수값 구현은 아직 미완료다.
- [x] 기본 named verb의 대입·호출·재정의·늦은 정의와 noun snapshot을 구현하고 Rust 회귀 테스트를 추가했다. train·locale·문장 내부 대입은 미완료다.
- [ ] noun은 이름을 읽을 때 현재 값/버전을 취득하고 verb는 호출 시 해석하는 별도 참조로 표현한다. noun snapshot·verb 재정의·미정의 verb의 나중 정의를 구현과 C 차등 corpus에 추가한다. 정의/미정의 이름, 좌우 인자, 괄호, 대입 실패도 검증한다.
- [x] 깊이 128 제한과 긴 평면 식의 영향을 재현하고, parser가 조립하는 AST 높이를 128로 제한하는 정책을 적용했다. 경계·혼합 괄호·20,000연산·대입 실패 회귀 테스트를 추가했다.
- [ ] IR 도입 전후의 파싱 시간·할당 및 짧은 식 반복 실행 비용을 측정하고 회귀를 기록한다.

완료 조건: 목표 재현 사례가 C 두 기준선과 일치하고 로컬 검증 관문을 통과한다. 일반 J 함수/train 전체 지원과 부분 해결을 구분한다. 미해결 의미 차이를 허용 목록 확대나 corpus 삭제로 숨기지 않는다.

## P1 — C1 primitive 계약과 binding 완성 (P0 이후)

- [ ] canonical PrimitiveId/registry로 lexer·IR·계약·kernel 연결을 통합한다.
- [ ] 지원 primitive의 단항/이항, dtype·shape·rank·빈 배열·오류·overflow 규칙을 표와 코드로 명시한다.
- [ ] effect·alias·평가 순서·재결합 금지 조건을 정밀화한다. 순수 연산이라는 이유만으로 fusion을 허용하지 않는다.
- [ ] JAXA custom registry와 `with` 의미 주석/binding을 구현한다. backend 설정은 의미 계약에 넣지 않는다.
- [ ] scope/locale 및 버전 유효성의 지원 범위를 정하고 계약에 반영한다.

완료 조건: 지원 연산의 계약 누락을 자동 검사하고, Unknown/미지원 연산은 명시적인 barrier 또는 진단으로 남긴다. registry와 실제 kernel 동작을 회귀 테스트로 연결한다.

## P2 — C2 Jaxa Analyzer와 Logical Plan (P1 이후)

- [ ] 실행하지 않고 dtype/shape/rank를 분석한다. 동적·불명 정보는 추측하지 않고 표현한다.
- [ ] J의 cell/frame/agreement, 빈 frame과 prototype 규칙을 분석 모델에 연결한다.
- [ ] ValueId, 의존성·effect·alias 및 materialization boundary를 포함한 Logical Plan을 생성한다.
- [ ] 거대 배열을 실제 생성하지 않는 분석 테스트와 분석 결과/실행 결과 비교를 추가한다.
- [ ] 의미·오류 순서를 보존하는 rewrite/fusion의 허용 조건을 구현한다.

완료 조건: 지원 부분집합의 계획을 검사할 수 있고, 분석 중 kernel 실행이나 결과 배열 생성이 없으며, 알려진 shape/dtype 예측이 실제 결과와 일치한다.

## P3 — C3 물리 계획·CPU 코드 생성 (P2 이후)

- [ ] 논리 ValueId와 물리 BufferId를 분리한다.
- [ ] strides/offset/alignment/소유권을 표현하는 PhysicalArray와 view를 구현한다.
- [ ] 수명·alias 분석에 근거한 buffer reuse와 필요한 materialization을 계획한다.
- [ ] CPU lowering/codegen 및 executor를 연결한다. 참조 evaluator와 compiled 경로를 명확히 구별한다.
- [ ] SIMD·tail·overflow 승격·빈 배열·비연속 view를 검증한다.
- [ ] 컴파일된 경로가 의미·성능 완료 조건을 충족한 뒤 기본 실행 경로로 전환한다.

완료 조건: 참조 evaluator 및 C와 결과·오류를 비교하고 메모리 안전성 검사를 수행한다. CPU artifact 생성과 실행이 확인되기 전에는 compiler 완성으로 표시하지 않는다.

## P4 — 성능 및 언어 범위 확대 (P0부터 지속, P3에서 종합 판정)

- [ ] CPU/OS/compiler/옵션/C revision을 고정한 재현 가능한 성능 보고를 만든다.
- [ ] 큰 배열 덧셈을 우선 기준으로 삼고 scalar·작은 배열·대형 배열, 할당 포함/제외, warm/cold 실행을 구분한다.
- [ ] 반복·교대 측정으로 처리량·시간 분포·할당·복사량·보유 메모리를 보고한다. C 대비 열세는 원인을 찾아 개선하며 측정 없이 우위를 주장하지 않는다.
- [ ] upstream 테스트의 의존성과 지원 가능 파일을 조사해 manifest를 만든다. 추출 사례와 원형 파일 통과를 구분한다.
- [ ] 추가 verb/modifier/함수 및 boxed·complex·sparse 등 지원 범위를 우선순위별로 분리한다.
- [ ] portable 메모리 코드의 Miri, native 경로 sanitizer, malformed-input 검증을 로컬에서 확대한다.

완료 조건: 성능 주장은 명시된 workload와 장비 범위로 제한한다. 전체 J 호환은 일부 corpus 통과와 분리하며 기능별 지원표를 유지한다.

## P5 — CUDA (보류, CPU 완료 조건에 포함하지 않음)

- [ ] 사용자 재개 요청 후 실제 CUDA 장치와 검증 환경 확보.
- [ ] 같은 Logical Plan에서 device storage·전송·GPU lowering/codegen 구현.
- [ ] stream/event/completion과 비동기 버퍼 수명·오류 처리 구현. CPU 계산에 무조건 async를 도입하지 않는다.
- [ ] CPU/GPU 결과 및 전송 포함 성능 비교, 실제 장치에서 해제·동기화 오류 검증.

## 변경마다 적용할 로컬 완료 관문

- [ ] 의미 변경의 최소 재현 및 실패한 대입 이후 상태 확인 테스트 추가.
- [ ] Windows 기본/portable Rust 테스트, fmt, clippy 실행. 이 컴퓨터에서는 Linux/WSL 검증을 실행하지 않는다.
- [ ] Python 검증기 테스트 실행.
- [ ] 의미/kernel 변경 시 C j64/j64avx2 × Rust 기본/portable × 필요한 실행 모드 차등 검증.
- [ ] storage/SIMD 변경 시 alias·재사용·메모리 상한·tail·overflow·특수 실수 검증.
- [ ] 알려진 차이와 예상 밖 실패를 분리하고 보고서에 환경·revision·seed·binary hash 보존.
- [ ] 성능 관련 변경은 correctness 통과 후 성능 측정.
- [ ] 이 체크리스트와 지원 범위·진행 기록 갱신. 수행하지 않은 검사를 통과로 표시하지 않음.

위 관문은 매 변경마다 새로 적용하는 템플릿이다. 현재 체크되지 않은 상태가 이전 검증의 실패를 뜻하지 않는다. GitHub CI는 관문에 포함하지 않는다.

## C J 기능 조사 반영 (2026-09-28)

[C J 기능·scope 조사](C-J-FEATURE-AUDIT.md)를 지원 범위 기준으로 추가한다. 다음 이름 설계는 대입 scope와 품사를 분리하며, =. / 함수 local frame / locale 및 locative / 일반 함수값·train 순으로 검증 범위를 확장한다. 기본 named verb 구현만으로 전체 이름 의미를 완료 처리하지 않는다.

## 컴파일러 방향에 따른 우선순위 수정 (2026-09-28)

사용자 확인에 따라 C interpreter 내부 구성을 복제하지 않는다. 앞선 scope 확장 순서는 의미 조사 목록으로 유지하되 compiler 작업의 선행 완료 조건으로 삼지 않는다. 실제 순서는 심볼·값·호출 분석 IR → 정밀 계약 및 shape/rank 분석 → scope/함수값 해석 확장 → physical plan/CPU codegen이다.

- [x] 검사 전용 LogicalPlan에 SymbolId/ValueId 및 품사와 독립적인 scope 표현 추가.
- [x] noun 버전 읽기와 named verb 동적 호출 대상을 분리.
- [x] 데이터 의존성·보수적 오류 순서·대입 commit 조건 표현.
- [x] 실행 없는 분석과 버전/순서/Unknown barrier 회귀 테스트 추가.
- [ ] dtype/shape/rank 추론 및 실제 local/locale 해석.
- [ ] 동적 참조를 직접 호출로 바꾸는 증명/guard, plan 유효성 및 재컴파일.

상세 범위는 [컴파일러 IR 진행 기록](COMPILER-IR-PROGRESS.md)을 따른다. C1/C2 전체 완료를 뜻하지 않는다.
- [x] 직접 primitive 일부의 ShapeRule 및 literal/noun dtype·shape·rank 사실 전파 구현. prefix agreement와 overflow 타입 불확실성 회귀 검증 추가.
- [ ] 일반 rank modifier/reduction의 cell/frame 결과 추론, primitive 전체 dtype·오류 계약. 현재 기본 facts만으로 이 항목을 완료 처리하지 않는다.

- [x] 알려진 입력 shape에 대한 직접 primitive의 rank/cell/frame 분해, 음수 rank, frame prefix agreement 및 기본 reduction 결과 facts 구현.
- [x] 빈 cell과 빈 frame 구분 및 prototype 미지원 표시; 실행 결과 비교 회귀 테스트 추가.
- [ ] 일반 빈-frame prototype/padding, 데이터 의존 cell shape, 파생 verb의 정밀 effect/alias 계약과 fusion 증명.

- [x] 21개 built-in PrimitiveId 표와 lexer·LogicalPlan·valence 계약 조회 연결.
- [ ] Semantic IR 및 CPU kernel 문자열 adapter 제거, custom registry/with 계약과 전체 지원 manifest 완성.

- [x] token·Semantic IR VerbTarget·LogicalPlan·facts 분석까지 typed PrimitiveId 연결, named target과 primitive를 상호 배타적으로 표현.
- [x] 참조 실행기의 resolved target과 출력 pool 산술 선택을 ID로 연결.
- [ ] 기존 CPU kernel API 경계의 spelling adapter 및 kernel 내부 문자열 dispatch 정리.

## Extended scalar types and boxed nouns

- [x] Shared Scalar/DType vocabulary and exact immutable payloads.
- [x] Explicit unsupported CPU lowering for types without array storage.
- [ ] New array storage, literals and verbs; see [SCALAR-TYPES.md](SCALAR-TYPES.md).

## Boxed runtime and sparse storage — 2026-09-29

- [x] Recursive boxed arrays; boxing and scalar/uniform opening.
- [x] Shared storage, index/reshape/catenate/rank and alias regression tests.
- [x] Axis-sparse validated storage and bounded explicit dense conversion.
- [x] Windows-only default/portable (62 + 1 each), Clippy and Python comparator tests.
- [ ] Build native Windows pinned C reference; execute new boxed differential cases.
- [ ] Boxed padding/fill/empty prototypes/comparison/search.
- [ ] Integrate sparse storage into compiler/runtime and implement `$.`.
- [ ] Complex/extended/rational/symbol array storage and language execution.

Details: [scalar types](SCALAR-TYPES.md), [sparse storage](SPARSE-ARRAYS.md).
Reproducible local validation: `powershell -File tools/check-windows.ps1`.

## Dense/sparse conversion and packed bits — 2026-09-29

- [x] Dense-to-sparse conversion without dense-sized scratch/index arrays.
- [x] Exact float bit round trips, including signed zero and NaN payloads.
- [x] BitStorage: shared unaligned slices, tail masking, AND/OR/XOR/popcount.
- [x] Windows default/portable: 67 tests + 1 doctest each; Clippy; Python: 6 tests.
- [ ] Integrate sparse and bit layouts into Value/physical plans and J verbs.
- [ ] Native C differential verification remains blocked on a compatible Windows C compiler.

See [BIT-STORAGE.md](BIT-STORAGE.md). No speedup claim without benchmarks.

## Sparse runtime and definition audit — 2026-09-29

- [x] Sparse Value, metadata-only serialization, element/layout facts.
- [x] Basic `$.` conversion/construction and scalar component queries.
- [x] Explicit rejection of sparse inputs in unsupported dense kernels.
- [x] Audit direct/explicit definitions: parsing/execution remain unsupported.
- [x] Contain stdin bug where rejected definitions ran body lines globally.
- [ ] Definition input framing, AST, controls, local binding and IR execution.
- [ ] Native Windows C verification; existing reference compiler blocker remains.

See [DEFINITION-PARSING-AUDIT.md](DEFINITION-PARSING-AUDIT.md).

Validation for the runtime/audit follow-up: Windows default and portable each
75 + 1 tests; Clippy and Python harness (6) passed. Definition parsing remains
a separate unimplemented milestone; the CLI change only prevents body leakage.

## Definition implementation plan and acceptance contract — 2026-09-29

- [x] [단계별 구현 계획](DEFINITION-IMPLEMENTATION-PLAN.md): DEF-1 input through DEF-6 compatibility.
- [x] 17 positive acceptance tests compile, each explicitly ignored with its unimplemented milestone.
- [x] Explicit acceptance run confirms 0 passed / 17 failed; no definition capability claimed.
- [x] Active CLI regression: delimiter text in strings/comments does not abort following sentences.
- [x] Windows default/portable: 76 active tests + 1 doctest each; Clippy passes.
- [ ] DEF-1 source reader and DEF-2 definition AST implementation.
- [ ] DEF-3 local binding and DEF-4/5 execution/control/recursion.
- [ ] DEF-6 remaining forms and native Windows C verification.

Full acceptance command: `powershell -File tools/check-windows.ps1 -DefinitionAcceptance`.
It includes ignored tests, so it is expected to fail until the implementation is ready.
