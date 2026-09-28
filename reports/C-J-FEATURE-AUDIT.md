# C J 기능 조사와 RustJ 지원 간격

기준: 2026-09-28, `jsource-inspection` revision `e75016ca74b5e595dd323226e6a4990172f72ec6`.
아래는 기능군 단위 조사다. 모든 primitive/foreign 조합의 완전한 목록이나 upstream 테스트 전체 통과 보고가 아니다. 경로는 조사한 jsource checkout 기준이다. RustJ의 지원은 현재 코드 기준이다.

## 1. 이름·품사·scope: 현재 설계에 먼저 반영할 기능

| C J 기능 | 근거 | RustJ 상태 |
|---|---|---|
| noun 이름을 읽을 때 현재 값 취득 | `jsrc/p.c:668-681` | 실행용 frontend에서 noun snapshot 구현 |
| 일반 named verb 참조 생성 및 호출 시 정의 해석 | `jsrc/p.c:702-710`, `jsrc/sc.c:364-381`, `jtunquote` | 기본 primitive/단일 modifier 및 named chain 구현 |
| noun/verb 외 adverb·conjunction 품사 | `jsrc/p.c`, `jsrc/a.c`, `jsrc/c*.c` | 이름 붙은 adverb/conjunction 미지원 |
| `=:` global 대입 | `jsrc/p.c:1010` | 단일 이름 테이블에 구현; locale 의미는 미완료 |
| `=.` 함수 local 대입, 함수 밖에서는 global | `jsrc/w.c:158`, `jsrc/p.c:825`, `jsrc/p.c:1010` | lexer와 local frame 모두 미지원 |
| local 우선 조회와 global locale 경로 조회 | `jsrc/s.c`, `jsrc/p.c` | 다중 scope 미지원 |
| 직접 locative `name_locale_` / 간접 locative `name__loc` | `jsrc/sn.c`, `jsrc/s.c:411-482` | 미지원 |
| named/numbered locale와 locale 검색 경로 | `jsrc/sl.c`, `test/g18x.ijs`, `test/g310r.ijs` | 미지원 |
| 특수 by-value 이름·locale 전환·캐시된 참조 | `jsrc/p.c:668-703`, `jsrc/sc.c` | 미지원 |
| 이름 분류·목록·삭제 | `test/g18x.ijs`의 `4!:55`, `jsrc/sn.c` | 미지원 |

scope와 품사는 독립된 축이다. local noun / local verb / global noun / global verb가 모두 가능하다. global은 단일 프로세스 전역 dictionary와 동의어가 아니라 해당 locale의 이름 공간이다.

C 실행으로 확인한 scope 사례 (`tools/audit_scope_semantics.py`):

- `a=:10`, `f=:3 : 'a=.y'`, `f 3`: 반환값 3, global a는 10 유지.
- `a=:10`, `f=:3 : 'a=:y'`, `f 3`: 반환값 3, global a도 3.
- 함수 밖 `outside=.7`: 값 7 저장.
- 함수 안 `private=.y`: 호출 후 외부에서 이름 분류 -1(미정의).
- `a_scopeprobe_=:5`, `loc=:<'scopeprobe'`: 직접/간접 locative 모두 5.
- 간접 locative용 locale 이름은 이 사례에서 boxed scalar다. unboxed 문자열을 사용한 초기 조사에서는 rank error가 났으며 성공 사례로 계산하지 않았다.

## 2. 언어와 배열 기능군

| 기능군 | C 소스·테스트 근거 | RustJ 상태 |
|---|---|---|
| word formation, primitive 등록, 파싱 | `w.c`, `t.c`, `p.c` | scanner와 부분집합 IR parser 구현; 전체 문법 아님 |
| 명시적 함수와 direct definition | `cx.c`, `test/g000.ijs`, `test/g310r.ijs` | 미지원 |
| tacit train/hook/fork | `cf.c`, `test/g13x.ijs`, `test/g400.ijs` | 미지원; 이름 관련 일부 오류 차이의 원인 |
| rank/cell/frame와 agreement | `cr.c`, rank 관련 tests | 부분 지원; 일반 prototype/padding 등 미완료 |
| reduce/scan·adverb | `a.c`, `au.c` | 일부 reduction만 구현; 일반 scan/adverb 미완료 |
| amend, key, oblique | `am.c`, `am1.c`, `ao.c` | 미지원 |
| power, under/each, cut, gerund | `cp.c`, `cu.c`, `cc.c`, `cg.c` | 미지원 |
| 제어문과 명시적 실행 | `cx.c`, `test/g000.ijs`의 for/end, `test/g001.ijs` 등 | 미지원 |
| bool/integer/float/character | `jlib.h`, `va*.c`, `vchar.c` | CPU 부분집합 구현 |
| complex, extended integer, rational, boxed, symbol, wide character | `jlib.h`, `vx.c`, `vz.c`, `vsb.c` | 미지원; 현재 byte char와 wide char를 혼동하지 않음 |
| sparse 배열과 sparse 연산 | `vs.c`, `vcatsp.c`, `vfromsp.c`, `visp.c` | 미지원 |
| 구조·선택·검색·정렬 | `vcat.c`, `vfrom.c`, `vi*.c`, `vg*.c` | 일부 array/search verb 구현; 일반 정렬 등 미지원 |
| 수학·랜덤·소수·다항식·행렬 연산 | `ve.c`, `vrand.c`, `v2.c`, `v0.c`, `test/g128x.ijs` | 기본 산술 외 대부분 미지원 |

`t.c`를 출발점으로 각 primitive의 단항/이항 및 modifier 조합을 manifest로 세분화하는 작업은 남았다. 소스 파일이 있다는 이유만으로 기능을 완료로 표시하지 않는다. 예를 들어 `vfft.c`는 `not implemented`라고 명시되어 있으므로 내장 FFT 지원 근거로 사용하지 않았다.

## 3. 시스템·런타임 기능군

| 기능군 | C 근거 | RustJ 상태 |
|---|---|---|
| foreign 기능 dispatch | `x.c` | 미지원 |
| 파일·입출력·파일 잠금 | `xf.c`, `io.c`, `xl.c`, `test/g1x*.ijs` | J 호환 기능 미지원; RustJ CLI 파일 입력과 구별 |
| DLL/외부 함수 호출 및 메모리 인터페이스 | `x15.c`, `test/g15x.ijs` | 미지원; 향후 소유권/수명 계약 필요 |
| 디버깅·오류·호출 정보·중단 | `d.c`, `dc.c`, `dsusp.c` | 제한된 오류 문자열만 구현 |
| thread/task | `ct.c` | 미지원; C가 동시 실행 구조를 전혀 갖지 않는다는 주장은 잘못됨 |
| 시간·공간 측정 | `xt.c` | J 호환 foreign 미지원 |
| 메모리 관리와 CPU 최적화 | `m.c`, `va*.c`, AVX 소스들 | 독립 Rust storage/pool/SIMD 구현; 구조 동일성 요구 없음 |
| Windows/Linux 등 플랫폼 빌드 | `makemsvc`, `makevs`, `make2`, `overview.txt` | Windows/WSL CPU 검증 중 |

CUDA는 RustJ 향후 목표다. 이번 조사는 C J의 GPU 지원 전체를 판정하는 조사가 아니며, C thread/task 지원을 CUDA 또는 Rust async 지원과 같은 것으로 취급하지 않는다. foreign·OS 의존 기능은 빌드별 추가 확인이 필요하다.

## 4. 구현 순서에 반영할 변경

- [x] noun 즉시 값 취득 / named verb 호출 시 조회의 기본 분리.
- [ ] AssignmentScope(Global/Local)와 품사(Noun/Verb/Adverb/Conjunction)를 독립적으로 표현.
- [ ] `=.` 토큰과 대입 IR, 함수 밖 global 처리.
- [ ] 명시적 함수의 call frame, local 이름 수명, local/global 조회 순서 및 shadowing.
- [ ] LocaleId와 locale별 global table, locale path, direct/indirect locative.
- [ ] named verb 참조가 보존해야 하는 이름·locale 문맥과 품사 변경 검사.
- [ ] train 및 일반 함수값 표현, 함수 내부 대입을 포함한 시점 검증.
- [ ] primitive/foreign별 지원 manifest와 실제 upstream 파일 실행 의존성 목록.

global/local을 문자열 접두사로 흉내 내거나 noun/verb flag와 합치지 않는다. 현재까지 실행한 것은 선택된 C 조사 사례와 RustJ 부분집합 차등 검사이며, 위 upstream 파일 전체는 실행하지 않았다. GitHub CI는 생략한다.
