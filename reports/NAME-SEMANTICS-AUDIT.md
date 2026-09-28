# 이름 해석 및 깊은 식 조사 — 2026-09-28

## 미정의 이름은 항상 값 오류가 아니다

고정 C 소스 `jsource-inspection/jsrc/p.c:707-710`은 일반 미정의 이름을 동사 참조로 만든다. 명시적 정의의 특수 이름 등은 별도 경로다. `tools/audit_name_semantics.py`는 매 사례마다 새 J 인스턴스에서 대입을 실행하고 `4!:0`으로 결과 품사를 확인한다. noun 전용 JGetM adapter로 동사 값을 읽으면 생기는 도구 오류와 언어 오류를 구분한다.

| 식 | C 대입 결과 | 대입된 이름의 분류 |
|---|---|---|
| `missing` | 성공 | verb (3) |
| `1+missing` | 성공 | verb (3) |
| `missing+1` | value error | 미정의 (-1) |
| `(1 2+1 2 3)+missing` | length error | 미정의 (-1) |
| `1 2+1 2 3+missing` | 성공 | verb (3) |
| `missing+(1 2+1 2 3)` | length error | 미정의 (-1) |
| `(1+2)+missing` | 성공 | verb (3) |

이 검사는 C 의미 조사이며 Rust 호환성 통과가 아니다. Rust는 현재 이름을 noun으로만 취급하고 `Engine`은 noun 값만 저장한다. 따라서 단순히 Value 오류를 늦추는 수정은 올바른 해결이 아니다.

다음 구현 순서:

1. symbol binding에 noun/verb 구분 및 unresolved verb reference를 표현한다.
2. 분석 전 이름의 품사를 반영할 수 있도록 frontend를 분리한다. 실행 없는 parse API와 이름에 의존하는 문법 판정을 구분한다.
3. primitive/named/derived verb 및 필요한 train 표현을 추가한다. 지원하지 않는 함수 조합을 noun으로 잘못 해석하지 않는다.
4. 함수값 대입·호출·재정의·늦은 이름 해석을 검증한다. noun-only CLI/oracle 경계도 명시적으로 확장한다.
5. 위 사례를 정식 Rust/C 차등 corpus에 편입한다. 미완료 동안 허용 차이 목록을 넓히지 않는다.

## 파싱 단계의 깊이 제한

기존 evaluator는 깊이 128에서 오류를 냈지만 parser는 긴 평면 식을 무제한 깊이의 트리로 만들 수 있었다. 평가 오류 이후 또는 parse-only API 사용 후 재귀적인 트리 해제도 위험 경로였다.

이번 수정은 AST를 조립할 때 각 subtree의 높이를 추적하고, 루트부터 leaf까지 128개 간선을 넘기기 전에 limit error를 반환한다. 괄호·단항·이항을 합산하며 literal 데이터 길이를 표현식 깊이로 세지 않는다. 기존 괄호 파싱 깊이 제한과 함께 작동한다. parser가 만든 트리에 대한 안전 정책이며, 외부에서 직접 조작한 public Expr 트리의 임의 깊이를 보장하지 않는다.

128단계 성공, 129단계 및 혼합 괄호 초과, 20,000단항/이항 식 거절, 실패한 대입의 버전 보존을 회귀 테스트에 추가했다. 이는 깊은 식의 지원 확대가 아니라 명시적인 제한 정책이다. C와 같은 최대 표현식 크기를 보장하지 않는다. 향후 arena/반복 순회로 제한을 완화할 수 있다.

## noun 즉시 값 취득 / verb 실행 시 해석 — 사용자 지적 반영

일반적인 정의된 이름에 대한 핵심 규칙은 `p.c:668-681`의 noun 값 사용과 `p.c:702-703`의 non-noun name reference 생성이다. `sc.c:364-381`의 namerefacv는 noun이면 값을 유지하고 verb이면 unquote를 실행 함수로 갖는 참조를 만든다. unquote는 호출 시 정의를 해석하고 현재 품사가 기대와 맞는지 검사한다. 특수 by-value 이름, 캐시, locale 처리는 별도 규칙을 갖는다.

C에서 확인한 시간적 동작:

- `a=:1; b=:a; a=:2` 이후 `b`는 1이다. noun의 현재 값을 취득하며, 나중의 이름 재정의를 따라가지 않는다.
- `f=:+; g=:f; f=:*` 이후 `g 3`은 1이다. verb 참조는 호출 시 새 정의를 사용한다.
- `g=:unknownverb; unknownverb=:-` 이후 `g 3`은 -3이다. 미정의 이름의 참조도 나중에 정의하여 호출할 수 있다.

위의 세 사례를 audit 도구에 추가했다. 앞선 미정의 이름 설명은 이 noun/verb 기본 규칙을 충분히 구분하지 않았다. RustJ 설계에서는 모든 ReadName을 동일하게 늦게 조회하는 방식으로 일반화하지 않는다. noun은 문장 처리 중 취득한 값/버전에 연결하고, verb는 참조 및 호출 시 해석 규칙을 별도로 표현해야 한다. 정적 최적화로 verb를 고정할 때도 재정의·품사 변경의 유효성 검증이 필요하다. 현재 noun-only IR에는 이 구분이 완성되지 않았다.

검증 기록: C j64/j64avx2 각각에서 이름 의미 조사 10사례의 기대 결과를 확인했다. 이는 Rust 함수값 지원 검증이 아니다. 깊이 제한 변경의 Windows/WSL 기본·portable 테스트는 각 일반 39개 + doctest 1개 통과, Windows clippy와 Python harness 3개 통과. 기존 1,915문장 × 8조합 차등 검사에서 새 불일치는 없고 j64의 기존 dtype 차이 1건만 유지했다. GitHub CI는 실행하지 않았다.
