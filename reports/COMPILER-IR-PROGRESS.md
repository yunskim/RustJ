# 컴파일러 분석 IR 진행 — 2026-09-28

## 방향

C J의 관찰 가능한 의미를 보존하되 C 인터프리터의 symbol table·실행 구조를 복제하지 않는다. 기존 evaluator는 Rust 의미 기준선으로 유지한다. local/global 기능을 evaluator에 모두 확장한 뒤 compiler에 착수하는 순서는 폐기하고, compiler의 심볼·scope·값·호출 표현부터 만든다. 미지원 J 의미는 계속 별도 추적한다.

## 이번 구현

`Engine::analyze(source)` → 환경을 반영하는 Semantic IR/binding → `analysis::LogicalPlan`.

- plan-local SymbolId와 ValueId로 이름 및 데이터 의존성을 표현한다. runtime pointer나 BufferId가 아니다.
- Scope는 품사와 독립적으로 둔다. 현재 frontend는 CurrentGlobal만 출력한다. LocalFrame 표현이 있다는 이유로 local 함수 지원 완료로 표시하지 않는다.
- ReadNoun은 관찰한 NameVersion을 명시한다. 분석 이후 재정의가 발생해도 이전 계획의 버전은 바뀌지 않는다.
- primitive 호출과 Dynamic(SymbolId) 호출을 구분한다. named verb는 현재 정의의 primitive로 임의 고정하지 않는다.
- rank/reduction 정보를 보존하며, 파생 verb와 동적 호출은 정밀 계약이 완성될 때까지 Unknown이다.
- 계산의 데이터 의존성과 별도로 보수적인 order_after 간선을 둬 지원 부분집합의 오른쪽 우선 오류/효과 순서를 보존한다.
- 대입은 값 생성과 분리된 Write로 표현한다. RHS 및 순서 의존성이 성공한 뒤에만 커밋한다는 조건과 이전/예정 버전을 보존한다.
- 분석은 kernel을 호출하거나 계산 결과 배열을 생성하지 않는다. 소스의 literal 파싱·IR 저장 자체에는 메모리를 사용한다.

예제: `cargo run --example logical_plan -- "1 + 2 * 3"`은 값 7을 계산하는 대신 literal과 두 호출의 계획을 출력한다. `i.9223372036854775807`도 결과 배열 생성 없이 계획을 만들 수 있다.

## 검증 기준

`tests/analysis.rs`에서 다음을 확인한다.

- 거대 i. 및 런타임 domain error 식의 분석이 실행/대입/출력 캐시 변경 없이 끝난다.
- 동일 noun의 반복 읽기는 동일 symbol 및 버전에 연결되고 verb 참조는 별도로 남는다.
- 재정의 이후 새 분석에는 새 noun 버전이 반영되고 이전 snapshot은 바뀌지 않는다.
- 경쟁하는 오류의 우측 호출 → 좌측 호출 → 상위 호출 순서, 모든 데이터/순서 간선이 이전 노드를 가리키는 성질.
- primitive overflow 계약과 derived/dynamic Unknown barrier 보존.

## 명확한 미완료 범위

이 계획은 검사 전용 snapshot이다. cached plan 실행 API, Engine 간 유효성, 버전 guard·deoptimization/recompilation, 직접 호출 specialization, codegen은 아직 없다. 현재 plan의 public 필드를 사용자가 변경해도 실행기로 전달할 수 없다.

정밀 dtype/shape/rank 추론, CFG/분기/loop SSA·phi, 실제 local frame/locale 해석, general train·함수값, canonical PrimitiveId 및 registry/with, 효과 분석·fusion legality는 미완료다. 현재는 straight-line dataflow 및 보수적 순서 모델이며 C2 전체 완료가 아니다. source span은 각 연산에 보존하되 괄호 자체는 lower 단계에서 제거한다.

## 다음 구현 순서

1. canonical primitive 계약 및 자료형/shape 정보(Unknown 포함)를 논리 계획에 연결한다.
2. rank/cell/frame/agreement와 오류 의존성을 분석하고 조건부 최적화의 증명 조건을 명시한다.
3. lexical symbol/scope 해석과 함수값/동적 locale 경계를 확장한다. local 값은 가능한 경우 SSA/slot으로 연결한다.
4. guard·재컴파일 정책을 갖춘 compiled-plan 수명 모델, physical planner, CPU lowering을 구현한다.

GitHub CI 제외, CUDA 구현 보류를 유지한다.

## 이번 변경의 실제 검증

Windows/WSL 각각 default/portable의 일반 Rust 테스트 45개 + doctest 1개 통과. Windows fmt/clippy, Linux Python harness 3개 통과. logical_plan 예제에서 kernel 실행 없이 3개 literal과 2개 호출의 계획 출력을 확인했다. 새 release 빌드로 기존 evaluator의 C 차등 회귀도 재검증했다: C j64/j64avx2 × Rust default/portable × 기본/참조 8조합, 각 1,940문장. j64는 1,939 일치 + 기존 dtype 차이 1건, j64avx2는 1,940 일치. 이 차등 검사는 아직 실행기가 없는 LogicalPlan 자체의 수치 실행 검증을 의미하지 않는다. upstream 전체·Miri·sanitizer·GPU·CI는 실행하지 않았다.

## dtype/shape/rank facts 기초 (2026-09-28)

- `contracts::ShapeRule`로 성공한 결과의 shape 관계를 명시했다: 보존, J prefix agreement, ravel, 축 역순, shape-of, tally. 미지원 계약은 Unknown이다.
- 각 논리 노드에 Facts를 연결했다. literal과 noun binding은 dtype/shape/rank를 알고, noun 정보는 앞서 기록한 대입 버전에 종속된다.
- 직접 primitive의 지원 규칙만 전파한다. 일반 rank modifier/reduction, named dynamic call, 값에 의존하는 i. shape는 아직 Unknown이다.
- Int/Int 덧셈·뺄셈·곱셈은 overflow를 계산해보지 않고 IntOrFloat로 표현한다. 이를 Exact(Int)로 잘못 고정하지 않는다. 다른 미정밀화 dtype 규칙은 Unknown으로 둔다.
- shape를 모르더라도 ravel 결과 rank=1, tally 결과 rank=0처럼 확실한 정보는 독립적으로 표현한다.
- 모든 정보는 '실행이 성공한다면'의 결과 사실이다. Known shape/type만으로 domain/length 오류 검사, 동적 정의 확인, order edge를 제거할 수 없다. 일치하지 않는 prefix shape는 결과 shape를 Unknown으로 남기며 분석 중 runtime 오류를 던지지 않는다.

회귀 테스트는 입력 binding·scalar 확장·prefix/suffix 구별·transpose/reverse/ravel·빈 배열·overflow 승격에서 추론한 사실을 실제 실행 결과와 비교한다. 완전한 primitive 계약, 일반 cell/frame/rank 분석과 오류 진단은 후속이다.

facts 변경 검증: Windows/WSL 기본·portable 각각 일반 테스트 47개 + doctest 1개 통과. fmt/clippy 및 Python harness 3개 통과. 새 release의 기존 evaluator 회귀는 1,940문장 × 8조합에서 j64 1,939 일치+기존 dtype 차이 1건, j64avx2 1,940 일치. 분석 자체는 새 테스트에서 실행 결과와 비교했으며 모든 J primitive에 대한 완전한 추론 검증은 아니다. CI는 생략했다.

## rank/cell/frame 및 reduction 분석 (2026-09-28)

`RankPlan`에 좌우 frame/cell shape, prefix agreement로 얻은 결과 frame, empty-frame prototype 필요 여부를 표현했다. rank는 양수일 때 인자 rank로 제한하고 음수일 때 상대 rank로 해석한다. 셀 분석 결과 shape 앞에 결과 frame을 붙인다. shape나 frame agreement를 확정할 수 없으면 결과를 추측하지 않는다.

직접 primitive와 `+ - * %` reduction을 분석한다. reduction은 scalar에서 shape를 유지하고 그 외에는 첫 축을 제거한다. 1개 item은 타입을 유지하고 `+/`·`*/`의 0개 item identity는 현재 kernel과 같은 Bool로 표현한다. Int reduction의 overflow 가능성은 IntOrFloat로 남긴다. 일반 named call과 미지원 파생 조합은 여전히 Unknown이다.

빈 cell과 빈 frame을 구분한다. 예를 들어 `[2,0]`의 각 rank-1 cell에 `+/`를 적용하는 경우와 `[0,3]`의 rank-1 cell을 순회하는 경우는 다르다. 후자는 현재 실행기에서 prototype 처리가 미지원이므로 `requires_empty_frame_prototype=true`와 Unknown facts를 남긴다. frame 불일치도 Unknown facts로 남겨 분석 단계에서 런타임 오류 순서를 바꾸지 않는다.

회귀 테스트는 일반/음수/초과 rank, 좌우 다른 rank, scalar, reduction 0/1/multiple items, overflow, 빈 cell/frame, incompatible frame을 실행 결과와 비교한다. 파생 verb의 effect/alias 계약은 여전히 보수적 Unknown이며, 이번 결과 shape 추론만으로 fusion을 허용하지 않는다. 일반 prototype/padding·데이터 의존 cell shape·재결합 증명은 미완료다.

## canonical PrimitiveId 기초 (2026-09-28)

현재 지원하는 21개 primitive spelling을 `primitive::PrimitiveId` 단일 표로 묶었다. lexer의 중복 spelling 목록을 제거하고, LogicalPlan의 직접 호출 대상을 문자열 대신 ID로 표현한다. contract 조회도 ID와 valence로 분기하며 문자열 lookup은 기존 인터페이스용 adapter로 유지한다. 현재 semantic parser와 CPU kernel의 문자열 인터페이스는 아직 남아 있다. 이것까지 완전히 통합했다고 주장하지 않는다.

단항/이항은 같은 ID의 별도 계약이며 unsupported valence는 Unknown이다. named call은 여전히 Dynamic(SymbolId)이다. custom registry/with는 미완료이고 등록되지 않은 이름을 built-in으로 추측하지 않는다. 등록 spelling의 lexer→verb value→LogicalPlan 연결, 중복 spelling, valence 구분과 dynamic barrier를 테스트했다. 기존 API 사용자를 위해 보존한 문자열 adapter는 최종 compiler backend 설계 제약이 아니다.

rank/PrimitiveId 변경을 합친 최신 검증: Linux 기본·portable 각각 일반 테스트 51개 + doctest 1개, fmt/clippy, Python harness 3개 통과. 새 release로 1,940문장 × 8조합 재검증: j64는 1,939 일치 + 기존 dtype 차이 1건, j64avx2는 1,940 일치. Windows에서는 환경 전환 뒤 Cargo PATH가 누락되었고, 절대 경로로 재시도한 호출도 완료 로그를 남기지 않아 이번 PrimitiveId 변경의 Windows 검증은 확인되지 않았다. 앞선 rank 단계의 Windows 테스트와 혼동하지 않는다. GitHub CI는 실행하지 않았다.
