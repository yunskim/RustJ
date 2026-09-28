# Lexer audit — 2026-09-27

결론: 현재 lex는 J word formation 전체 상태를 구현하지 않는다. if 분기 자체가 원인은 아니며 단어 경계·primitive 인식·숫자 변환을 합친 부분집합 구현이 원인이다. 이번 작업은 점검이며 lexer 교체나 오류 수정은 아직 하지 않았다.

## 근거

고정 C revision e75016ca74b5e595dd323226e6a4990172f72ec6의 jsrc/w.c:11–54는 SS, SS9, SX, SA, SN, SNB, SQQ, S9, S99, SQ, SNZ, SZ, SU, SDD, SDDZ, SDDD의 16개 상태를 정의한다. wordil은 문자 class와 전이표로 단어 span을 만들고 enqueue는 primitive/name/numeric 분류와 의미 변환을 수행한다. 직접 정의·제어어 처리는 상위 단계도 필요하며 상태표만 이식해서 전체 parser가 완성되는 것은 아니다.

## 재현한 결함

- `1 NB.. 2`, `1 NB.: 2`: C ;:는 각각 `1|NB..|2|`, `1|NB.:|2|`로 나눈다. C 평가 오류 코드 16인데 Rust는 starts_with("NB.")로 뒤를 버려 값 1을 반환한다. 단순 미지원보다 심각한 잘못된 성공이다.
- 숫자 뒤 콜론: `1 2: 3`의 C 경계는 `1|2:|3|`다. Rust는 앞의 1과 2를 숫자 목록으로 먼저 합친 후 콜론을 거절한다.
- `{{ y + 1 }}`의 C 경계는 `{{|y|+|1|}}`다. `{{.`는 `{|{.`로 되돌리고 `}}:`는 `}|}:`로 되돌린다. Rust에는 대응하는 직접 정의 상태가 없다.
- `a.`, `0:`, `if.` 등 접미사가 붙은 단어를 일반적으로 형성하지 않고 지원 primitive 목록만 별도로 검사한다.
- `foo_bar`는 C의 유효 이름이며 대입/참조가 가능하지만 Rust는 underscore 포함 이름 전체를 미지원 처리한다. 단어 형성 결함과 이름 의미의 미구현을 구별해야 한다.
- C ;:는 LF를 독립 단어로 보존한다. Rust lex는 ASCII whitespace로 취급해 숫자 필드까지 합칠 수 있다. CLI는 줄 단위 입력이므로 CLI 결과와 단일 Engine::eval의 다중행 입력을 같은 검사로 비교하면 안 된다.
- `1r2`, `2j3`은 C에서 각각 하나의 단어다. Rust도 스캔은 하나로 하지만 값 변환을 지원하지 않는다. 이는 상태 전이만 바꿔 해결되는 문제가 아니다.

정상 대조: `1  2\t3` 숫자 목록 묶기, `1 NB. comment`, 문자열 내부 NB. 및 doubled quote는 확인한 사례에서 일치했다.

## 검증 도구의 공백

현재 1904개 평가 사례는 lexer 상태/전이 커버리지가 아니다. C ;:의 단어 목록과 Rust span을 직접 비교하는 검사도 없다. oracle.eval은 소스 어디든 '=:'가 있으면 대입으로 판단하므로 문자열에 '=:'가 포함된 lexer 검사에 그대로 쓰면 안 된다. 이번 C 단어 검사에서는 run으로 임시 결과에 저장한 뒤 별도 이름을 읽어 이를 피했다. JGetM 기반 어댑터는 동사·boxed 결과를 직접 표현하지 못한다.

## 다음 수정 순서

1. 단어 형성 전용 scanner를 분리하고 byte span·분류를 출력한다. 모든 단어를 먼저 형성하며 미지원 primitive도 분해하지 않는다.
2. 숫자 묶음/콜론, quote parity, N→NB→NB.→comment, inflection, LF, {{/}} 복귀 상태를 명시적으로 모델링한다. enum+match 또는 전이표 어느 방식이든 전체 전이 규칙이 필요하다.
3. 별도 lowering 단계에서 현재 지원 Token/Value로 변환한다. 미지원 문법과 malformed spelling을 구분한다.
4. C ;: 기반 전용 oracle을 구현해 span/word를 차등 비교한다. NUL·LF·quote·NB.·숫자·점/콜론·brace 문자 class 조합, 상태×문자 class 커버리지, seed fuzz를 포함한다.
5. 발견한 실패를 회귀 corpus에 고정하고 기존 4개 C/Rust 평가 조합을 다시 통과시킨다. 의미 미지원은 word formation 통과와 별도로 보고한다.

C 테이블을 그대로 복사할 필요는 없지만 의미상 전이를 빠뜨리지 않아야 한다. 이 점검으로 전체 상태 호환을 주장하지 않는다.


## 후속 수정 — 2026-09-27

위 내용은 수정 전 감사 기록이다. src/scanner.rs에서 16-state byte scanner를 분리하고 src/syntax.rs의 lowering 앞에 연결했다. 숫자 후속 필드 병합, colon에 의한 숫자 verb 경계, N/NB/NB. 및 comment, quote parity, LF, doubled brace와 inflection 복귀를 구현했다. token별 byte span을 lex_spanned로 제공한다. NB..·NB.:는 spelling error가 되며 LF를 숫자 목록에 합치지 않는다.

단어를 형성할 수 있다는 것과 실행 지원은 다르다. 직접 정의, control word, 확장 숫자, locative, 미등록 primitive는 여전히 lowering/runtime 미지원이며 C1 전체 완료가 아니다. 닫히지 않은 quote는 scanner에서 오류를 내지만 현재 언어 API 오류 분류는 기존 Syntax를 유지한다.

로컬 검증: tools/word_conformance.py가 source를 byte 배열로 C에 전달하고 ;: 결과의 word/span을 비교한다. 대입 문자열·NUL·LF·비ASCII도 테스트 프로그램으로 실행하지 않고 데이터로 전달한다. 18개 접두사 × 256바이트 + 10개 회귀 + seed 20260927 무작위 10000개 = 14618개. C j64/j64avx2 양쪽에서 불일치 0, open-quote 오류 4425개 포함. 중괄호 교차 상태 6개 불일치를 조사해 수정 후 재실행했다. 모든 가능한 문자열의 증명이나 전체 J parser 호환성을 의미하지 않는다.

CI는 사용자 요청으로 생략했다. 새 scanner의 처리량/할당 비용 최적화는 이번에 측정하지 않았다. semantic node span 연결과 shared parser/name-version binding은 다음 작업이다.
