# Direct / explicit definition 구현·검증 계획

작성: 2026-09-29. 기준점: definition audit 이후.

## 목표와 현재 상태

`{{ ... }}`와 `m : body`를 컴파일러의 함수 정의로 처리한다. C의 토큰 큐,
인터프리터 메모리 구조를 복제할 필요는 없지만, 입력 경계·valence·이름 해석·
제어 흐름·오류와 부작용 순서는 J의 동작을 따라야 한다.

현재 구현은 정의를 거부한다. stdin 본문이 전역 코드로 실행되던 문제의 차단은
유지한다. **이 문서 작성과 수용 테스트 추가는 함수 정의 구현 완료가 아니다.**
Windows 네이티브 도구로만 검증하며 GitHub CI와 CUDA 구현은 생략한다.

## 단계별 작업과 완료 조건

| 단계 | 구현할 내용 | 완료 조건 |
| --- | --- | --- |
| DEF-1 입력 경계 | 공통 SourceReader로 stdin/script/완성 문자열 경로를 정리한다. 문장, 수집 중인 정의, EOF를 구분한다. | 완성 전 본문을 실행하지 않는다. LF/CRLF, 문자열·이스케이프·주석, 중첩 DD, 닫히지 않은 정의와 EOF를 검증한다. |
| DEF-2 정의 AST | DefinitionKind, 본문 문장, monad/dyad section, byte span을 보존한다. DD를 같은 정의 AST로 낮춘다. | `3 : '...'`, `4 : '...'`, `3 : 0 ... )`, 단순 DD가 실행 없이 파싱된다. 잘못된 section/종결자는 바인딩을 바꾸기 전에 거부한다. |
| DEF-3 바인딩 | 함수별 local slots, 파라미터, global 참조와 `=.`/`=:`를 구분한다. | 호출마다 독립 프레임을 만들고, local shadowing·global 갱신·noun snapshot·deferred verb lookup·별칭 보존이 테스트된다. |
| DEF-4 실행 가능한 IR | 기본 단항/이항 호출부터 구현한 뒤 제어문 AST를 CFG로 낮춘다. | positive tests를 단계별로 활성화한다. if/else/return, for, try/catch를 각각 완료한 뒤 다음으로 진행한다. |
| DEF-5 고급 호출 | 중첩 함수, 재귀, valence 선택, 함수값의 수명과 호출 깊이 제한. | 재귀 프레임이 독립적이고 오류 이후 프레임이 정리된다. 직접/참조 실행 경로가 같은 결과와 부작용 순서를 낸다. |
| DEF-6 호환성 확장 | noun DD, noun explicit, explicit adverb/conjunction, 추가 제어문과 표준 라이브러리 별칭. | native Windows C oracle로 형태별 의미를 확정하고 수용 테스트를 추가한다. 전체 지원으로 묶어 선언하지 않는다. |

### DEF-1의 입력 처리 계약

- `push_line`은 완성된 입력, 추가 입력 필요, 오류를 구분한다. 수집만 하며 실행하지 않는다.
- `finish`는 EOF에서 미완성 정의를 오류로 만든다. CLI는 본문을 개별 문장으로 재해석하지 않는다.
- 문자열의 doubled quote, `NB.` 주석, `{{.` 같은 다른 단어를 단순 brace 카운트로 처리하지 않는다.
- `{{)n`은 일반 DD와 다른 raw-text 모드가 필요하다. 처음에는 명시적으로 미지원으로 거부해도 되며, 내부를 일반 코드로 해석하지 않는다.
- explicit block의 종결 `)`와 section `:`는 독립된 정의 문법으로 처리한다. 문자열·주석·괄호 표현식과 구분한다.
- 입력 버퍼 크기와 중첩 깊이 제한은 RustJ 자원 정책으로 문서화한다. C와 같은 제한이라고 주장하지 않는다.
- `verb define` 등의 이름은 일반 이름 해석 문제다. 라이브러리 별칭을 무조건 예약어로 고정하지 않는다. 현재 CLI 안전 guard는 임시 정책이다.

### DEF-2/3의 AST·이름 계약

- `Definition`은 기존 `VerbTarget::Primitive/Named`와 별도 종류로 표현한다. DefinitionId와 타입이 있는 파라미터/로컬 slot을 사용한다.
- 본문은 여러 문장을 한 표현식으로 합치지 않는다. 제어문은 중첩 구조를 가진 AST로 남긴다.
- 정의를 생성하거나 `analyze`할 때 본문을 실행하거나 전역 noun을 값으로 고정하지 않는다.
- **호출 시** 본문 문장을 해석할 때 기존 noun-by-value와 named-verb-at-call 의미를 적용한다.
- 자유 변수, 중첩 정의의 외부 local 접근, 자기 참조는 C 조사 없이 Rust식 lexical closure 의미를 임의 도입하지 않는다.
- 실패한 정의 컴파일은 기존 이름·값·version을 유지한다. 함수 **실행** 중 이미 발생한 전역 대입까지 자동 롤백한다는 의미는 아니다.
- 자료형·shape/layout 추론과 순서 제약을 기존 Semantic IR → Logical Plan에 연결한다. 최초 구현에서 fusion이나 재배치를 추가하지 않는다.

### DEF-4/5의 실행 계약

- 먼저 제어문 없는 문장열, 마지막 noun 반환, 명시적인 valence를 구현한다.
- 각 제어문은 구조 검증 → CFG 생성 → 실행 → 부작용 순서 검증을 거친다.
- `return.` 뒤의 문장은 실행하지 않는다. loop 변수와 호출 인수는 global에 남기지 않는다.
- 오류 반환과 정상 반환 모두 local 프레임을 정리한다. 깊이/반복 자원 제한은 별도 테스트한다.
- 사용자 이름을 통한 재귀와 primitive 호출을 구분한다. 컴파일 시 상수화하려면 유효성 근거가 필요하다.

## 지금 작성한 실행 가능한 테스트

`tests/definition_acceptance.rs`의 17개 테스트는 현재 API에 대해 컴파일되지만,
미구현 단계에 해당하므로 `#[ignore = "DEF-..."]`로 명시되어 있다.
**ignored는 통과가 아니다.** 전체 구현 전까지 일반 테스트 수와 별도로 보고한다.

| 테스트 이름 | 주요 검증 | 단계 |
| --- | --- | --- |
| parsing_complete_definitions_preserves_source_and_binding_boundary | source/span/assignment, 실행 없는 analysis | 1–2 |
| cli_collects_a_definition_before_executing_any_body_line | stdin 완성 단위, LF/CRLF, 두 실행 경로 | 1, 4 |
| direct_monad_and_dyad_have_separate_parameter_frames | DD x/y와 호출 valence | 2–4 |
| explicit_string_and_multiline_bodies_agree | 문자열·block·dyad explicit 정의 | 2–4 |
| explicit_colon_line_selects_monad_or_dyad_section | `:` valence section | 2–4 |
| comments_and_literals_do_not_end_the_definition | 주석·문자열의 delimiter 무시 | 1–4 |
| nested_direct_verb_does_not_leak_a_local_function | 중첩 함수와 local function 수명 | 2–5 |
| local_shadowing_is_per_call_and_does_not_modify_globals | local/global 격리와 재호출 | 3–4 |
| body_globals_are_resolved_when_called | 정의 후 global noun/verb 재정의 | 3–4 |
| local_noun_copy_survives_reassignment_without_mutating_argument | noun 값과 배열 alias | 3–4 |
| definition_construction_does_not_run_global_side_effects | 생성 시 부작용 금지, 호출 시 global 대입 | 3–4 |
| invalid_redefinition_keeps_old_function_and_version | 잘못된 재정의의 원자성 | 2–4 |
| if_else_and_return_preserve_branch_execution | 분기와 early return | 4 |
| for_loop_accumulates_without_leaking_loop_names | 루프와 local loop 변수 | 4 |
| try_catch_handles_body_error | 예외 제어 흐름 | 4 |
| recursive_calls_keep_the_callers_argument | 재귀 프레임 독립성 | 5 |
| evaluator_paths_agree_on_function_calls_and_global_rebinding | 직접/참조 경로의 동일 결과 | 5 |

DEF-2가 실제 AST를 도입하면 첫 테스트에 Definition/section/control-node 종류를
직접 검증하는 assertions를 추가한다. 현재 source/span 검사는 그 구조 검사를
대신하지 않는다. DEF-1의 증분 reader unit test도 해당 API 구현과 함께 추가한다.

## 추가해야 할 테스트 — 아직 작성·완료하지 않은 항목

- [ ] reader에 한 줄/한 글자씩 공급해도 같은 완성 단위·byte span이 나오는지.
- [ ] 파일 입력과 `-e`에서 완성 정의, stdin의 EOF/오류 위치, UTF-8 span.
- [ ] control AST의 잘못된 중첩, dangling else/end/catch, 빈 valence section.
- [ ] while/whilst, select/case/fcase, break/continue, try/catchd/catcht와 throw.
- [ ] `0 : 0`, `{{)n`, DD의 명시적 kind, `1 :`, `2 :` operand 및 파생 함수 의미.
- [ ] 자유 local 캡처와 재귀 제한, 오류 뒤 frame cleanup, 함수 배열 결과의 공유 수명.
- [ ] C와 타입·shape·값·오류·global 부작용 순서 비교. synthetic 테스트로 대체하지 않는다.

## TDD 실행과 완료 판정

Windows에서 `tools/check-windows.ps1`로 fmt/default/portable/Clippy를 실행한다.
양성 테스트는 다음 명령으로 별도 실행한다.

```
powershell -File tools/check-windows.ps1 -DefinitionAcceptance
```

`-DefinitionAcceptance`는 `--include-ignored`로 이미 활성화한 테스트와 아직
보류한 테스트를 모두 실행한다. 초기 실행은 실패해야 한다. 해당 단계 구현 후 **정확한 테스트의 ignore를 제거**하고
전체 회귀를 통과시킨다. 모든 definition 테스트를 일괄 enable하거나, expected 값을
현재 Unsupported 결과로 바꿔 통과시키지 않는다. 기존 `tests/definitions.rs`의 미지원
기대 사례는 지원이 시작되는 형태만 양성 테스트로 교체한다. 본문 오실행 방지 테스트는
미완성/잘못된 정의 사례로 계속 유지한다.

현재 native Windows C oracle은 컴파일러 제약으로 미실행이다. 수용 테스트는
구현할 동작의 계약이며 C 실행으로 확인된 호환성 증명이 아니다. DEF-6 완료 전까지
직접/참조 Rust 경로 간 일치도 C와의 일치로 보고하지 않는다.

## 우선 작업

다음 구현 단위는 DEF-1 SourceReader와 DEF-2의 제어문 없는 verb definition AST다.
이 단위를 마친 뒤 monad/dyad 호출과 local assignment에 집중한다. 새 일반 verb나
GPU 기능보다 함수 정의의 입력·binding 경계를 먼저 완성한다.

## 작성 직후 검증 기록

- Windows default/portable: 각각 활성 테스트 76개와 doctest 1개 통과.
- definition 양성 수용 테스트 17개는 일반 실행에서 ignored로 별도 표시된다.
- 양성 테스트 별도 실행: **0 통과, 17 실패**, 현재 Unsupported 경계에서 실패함을 확인했다.
- Clippy all-targets, warnings denied 통과. 테스트 자체도 컴파일 검사를 통과했다.
- native C oracle 실행은 미완료. Linux 테스트와 GitHub CI는 실행하지 않았다.

아직 완료하지 않은 parser/runtime 동작을 테스트 통과 수에 포함하지 않는다.
