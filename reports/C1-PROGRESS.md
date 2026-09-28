# C1 첫 구현 및 채팅 인계 — 2026-09-27

“GPU 친화적 배열 설계” 채팅의 원격 main 6c20232까지 fast-forward 반영했다. compiler-first, JArray/PhysicalArray 분리, Semantic IR → Jaxa → Logical Plan → Physical Planner → Codegen → Executor 순서를 따른다. CUDA는 보류한다. with는 semantic annotation/binding이며 backend 설정을 넣지 않는다.

## 실제 코드

- semantic::parse: 기존 부분집합 Token으로 Literal, ReadName, Monad, Dyad 및 문장 대입을 만든다. primitive 호출·name lookup·상태 변경 없이 구문 구조를 생성한다. 원본 source와 이름을 보존한다. rank 목록과 reduction도 verb 노드에 보존한다.
- contracts::lookup: valence별 operation class, effect, value/overflow rule, 오류 가능성·평가 순서·alias proof·재결합 금지를 나타내는 보수적 기본 계약. 미등록 이름은 Unknown이며 안전한 최적화 대상으로 추측하지 않는다.
- Engine::eval_semantic_reference / --semantic-reference: IR 결과를 검증하기 위한 명시적인 reference interpreter. 기본 경로와 parser 및 IR evaluator를 공유하며 출력 버퍼 pooling만 끈다. 두 경로의 비교는 독립 parser 검증이 아니다. 최종 physical executor나 native codegen으로 부르지 않는다. 기존 kernel을 사용하므로 독립 수치 구현은 아니며 C 차등 검증을 유지한다.
- conformance.py --semantic-reference 로컬 검증: 기존 corpus를 두 경로에서 비교한다. 기본 경로는 공통 IR을 소비하면서 기존 출력 버퍼 재사용을 유지한다.

## C1 미완료 항목

각 IR 표현식·verb·대입 대상에 byte span을 연결했다. 괄호는 Group 노드로 보존하여 내부 이름의 정확한 위치를 유지한다. Engine::prepare_semantic은 실행 없이 이름 참조를 현재 Engine의 이름별 대입 버전에 연결하고, 대입의 이전/예정 버전을 기록한다. 성공한 대입만 버전을 증가시키며 직접/IR 경로가 같은 저장 절차를 쓴다. BoundProgram은 분석 시점의 snapshot이며 재실행 가능한 cached plan이나 완성된 SSA가 아니다. 버전은 다른 Engine 사이에서 비교할 수 없고 physical buffer의 버전도 아니다.

word formation은 16-state scanner로 분리했고 NB..·NB.: 결함을 수정했다. primitive/value lowering은 여전히 부분집합이다. locale/scope binding, 정밀한 shape/dtype/rank/effect 계약, canonical PrimitiveId 및 custom registry, with binding은 후속이다. 직접 parser 중복을 제거하고 semantic::parse로 통합했다. literal은 복제하지 않고 evaluator로 이동한다.

공통 parser는 실행 전에 문장 전체를 검사한다. prepare_semantic의 정적 binding은 분석 API이며 실행 경로에서는 eager name lookup을 하지 않는다. 실행은 오른쪽 인자부터 평가한다. 기존 직접 parser가 왼쪽 괄호부터 실행하던 복합 오류를 수정하고 C 기준 회귀 사례를 추가했다. 미정의 이름이 J에서 verb/train으로 해석될 수 있는 문장까지 지원하는 것은 아니다. 예를 들어 `(1 2+1 2 3)+missing`의 C length error와 Rust value error는 아직 차이가 남는다. 이를 허용 예외로 숨기지 않으며 name/품사 해석 확장 과제로 기록한다. 재귀 평가 깊이 128 제한은 이제 두 경로에 적용되므로 긴 평면 식에도 영향을 줄 수 있다. IR 할당 비용을 포함한 파싱 성능은 아직 측정하지 않았다.
## 다음 작업 순서

1. 완료: C word formation과 scanner/span 비교 및 NB..·NB.: 오류 수정.
2. span과 Engine 내 name/version binding 기초 완료. 공유 frontend 통합과 지원 부분집합의 복합 오류 검증 완료. 일반 name/품사 해석과 깊은 식 처리는 후속이다.
3. primitive registry의 정밀 계약과 JAXA taxonomy/with binding을 구현한다. Unknown은 barrier로 유지한다.
4. C2에서 rank/cell/frame/agreement를 분석하고 실행 없는 Logical Plan을 만든다.
5. C3 physical view/planning과 CPU backend. CUDA 코드는 재개 요청 전 구현하지 않는다.

## 검증

Rust 테스트에 실행 없는 파싱(거대 i.도 배열 생성 없이 IR 생성), 우측 결합, rank/reduction 보존, 대입 실패·별칭, unknown barrier를 추가했다. 사용자 요청으로 이번 CI 추가는 제외했다. 로컬에서 C 두 빌드 × Rust default/portable 각각의 직접/IR 경로를 검사한다.

노드의 UTF-8 byte 위치, 괄호/수식자 위치, 반복 이름 참조, 실행 없는 binding, 두 evaluator의 대입 실패 및 별칭·버전 보존 회귀 테스트를 추가했다.

이전 span/binding 변경 검증: Linux에서 default/portable 각각 Rust 37개 테스트와 doctest 1개 통과, fmt/clippy 및 Python harness 3개 통과. C j64/j64avx2 × Rust default/portable × 직접/IR의 8개 조합에서 각 1,906문장을 비교했다. j64는 조합당 1,905 일치 + 기존 rank subtraction dtype 차이 1건, j64avx2는 1,906 일치이며 새로운 불일치는 없다. 전체 upstream suite·Windows 실행·Miri·sanitizer·GPU·CI 검증은 이번에 수행하지 않았다.

## 공통 frontend 통합 검증

Windows와 WSL Linux에서 default/portable 각각 Rust 테스트 38개 및 doctest 1개 통과. Windows clippy/fmt와 Linux Python harness 테스트 3개 통과. 복합 오류·실패한 대입 이력 9개를 추가하여 C j64/j64avx2 × Rust default/portable × 기본/참조의 8개 조합에서 각 1,915문장을 검증했다. j64는 1,914 일치 + 기존 dtype 차이 1건, j64avx2는 1,915 일치이며 이 corpus의 새 불일치는 없다. C 비교는 WSL Linux 라이브러리로 수행했다. 위에 명시한 미정의 이름의 품사 해석 차이는 corpus 통과 범위 밖의 확인된 제약이다. CI, upstream 전체 suite, Miri/sanitizer, GPU 검증은 수행하지 않았다.

## 2026-09-28: noun snapshot / named verb 참조 첫 구현

Engine binding을 Noun/Verb로 나눴다. 실행용 frontend는 정의된 noun을 읽을 때 값을 취득해 Literal로 넣고, 정의된 verb 및 일반 미정의 이름은 Named reference로 남긴다. 호출 시 참조 체인을 조회하여 primitive 또는 단일 reduction/rank verb로 실행한다. 참조 대상이 noun으로 바뀌면 domain error, 아직 미정의면 value error를 반환한다. 순환·과도한 참조 체인은 limit error로 제한한다. noun/verb 대입 모두 성공한 뒤 버전을 갱신한다.

`prepare_semantic`도 Engine의 품사를 반영한다. noun read는 버전으로 연결하고 verb reference는 별도 목록에 남겨 현재 noun 버전에 고정하지 않는다. 환경 없는 `semantic::parse`는 기존처럼 이름을 unresolved noun 자리로 보존하므로, Engine 의존 품사 분석의 대체물이 아니다.

지원: `f=:+`, `g=:f`, `f=:*`, `g 3`, `2 g 3`, `g=:future`, 이후 `future=:-`, `sum=:+/`, 기본 named verb의 reduction/rank. 현재 API의 반환형은 여전히 noun이므로 대입 없는 verb 결과 출력은 Unsupported다. train/hook/fork, 명시적 정의, locale/scope, 중첩 named modifier 조합은 미완료다. 특히 `(1 2+1 2 3)+missing`처럼 우측 verb로 인해 train이 형성되는 원래 사례는 이 첫 구현에서 해결 완료로 표시하지 않는다. 문장 내부 대입도 미지원이므로 noun을 읽는 정확한 시점의 부작용 상호작용은 후속 검증 대상이다.

named verb 변경 검증 완료: Windows/WSL 기본·portable 각각 일반 테스트 41개 + doctest 1개, Windows clippy/fmt, Python harness 3개 통과. C 비교는 25개 상태 사례를 추가한 1,940문장 × 8조합에서 j64 1,939 일치+기존 dtype 차이 1건, j64avx2 1,940 일치. portable 임시 실행 파일 경로 누락으로 한 차례 검사 시작이 실패했으며 저장소의 target/portable-linux에 다시 빌드하여 portable 4조합을 모두 재검증했다. CI는 실행하지 않았다.
