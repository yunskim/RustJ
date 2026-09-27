# J의 메모리 관리와 RustJ 설계 방향

현재 상태 보완: 아래의 기존 Rust 표현 설명은 M1 분석 시점이다. 이후 inline scalar/shape와 단독·공유 CPU storage, 차용 뷰를 구현했다. 구현 범위·측정·남은 과제는 [M2 결과](MILESTONE-2.md), 후속 논리/물리 메모리 계획은 [JAXA 검토](JAXA-REVIEW.md)를 참조한다.

후속 조사: [Rust 배열 프로젝트를 반영한 구체적 개선안](RUST-ARRAY-REFERENCES.md). Arrow·ndarray·faer·Polars를 참고해 소유권 회수 조건, scratch와 출력 풀의 분리, 연속 배열 유지, 제한된 연산 결합을 추가했다. 아래 적용 순서에도 scratch 분리를 반영했다.

분석 기준: jsoftware/jsource 커밋 `e75016ca74b5e595dd323226e6a4990172f72ec6`. Linux x86-64의 일반적인 dense 배열 경로를 중심으로 조사했다. GMP, 메모리 매핑, 특수 런타임 객체에는 별도 경로가 있다. 아래 Rust 설계는 제안이며 아직 구현하지 않았다.

## 1. 실제 C 정책

### 배열 표현과 할당 단위

`AD`는 64비트에서 기본 헤더 7워드(56바이트), rank개 shape 워드, 연속 payload로 구성된다. 타입, 원소 수, 참조 카운트, 플래그, 데이터 오프셋, backer 또는 임시 스택 정보 등이 헤더에 있다. 일반 dense 배열은 헤더·shape·데이터를 한 블록에 배치한다. 가상 배열은 같은 데이터 오프셋 표현을 이용해 외부 backer의 데이터를 가리킨다.

블록 시작 주소의 캐시라인 정렬과 데이터 주소의 정렬은 다르다. rank 1에서는 데이터가 64바이트 뒤에 있지만, 모든 rank의 데이터가 32/64바이트 정렬되는 것은 아니다.

근거: [jtype.h:191](https://github.com/jsoftware/jsource/blob/e75016ca74b5e595dd323226e6a4990172f72ec6/jsrc/jtype.h#L191), [j.h:1418](https://github.com/jsoftware/jsource/blob/e75016ca74b5e595dd323226e6a4990172f72ec6/jsrc/j.h#L1418).

| 항목 | C의 정책 | 의미 |
|---|---|---|
| 작은 블록 | 64·128·256·512·1024바이트 클래스 | payload가 아니라 헤더 등을 포함한 블록 크기 기준 |
| 풀 보충 | 64KiB 청크를 같은 크기의 블록으로 분할 | 매번 시스템 할당기를 호출하지 않고 free-list에서 꺼냄 |
| 큰 블록 | 필요한 크기를 2의 거듭제곱으로 올려 `malloc` | 정렬 공간과 32바이트 tail padding을 추가 |
| 작은 블록 해제 | 해당 풀의 free-list로 반환 | 즉시 시스템에 반환하지 않음 |
| 풀 회수 | 완전히 빈 청크를 찾아 `free` | 일부 사용 중인 청크는 남음 |
| 큰 블록 해제 | 원래 malloc 포인터로 `free` | 이 경로에는 J 자체의 큰 버퍼 재사용 캐시가 없음 |

풀 스캔은 약 1MiB에 해당하는 해제량을 기준으로 한 biased accounting/지연 플래그로 요청된다. 이것은 매번 RSS가 1MiB 줄 때 실행되는 규칙도, 객체 그래프를 순회하는 tracing GC도 아니다.

근거: [m.h:16](https://github.com/jsoftware/jsource/blob/e75016ca74b5e595dd323226e6a4990172f72ec6/jsrc/m.h#L16), [m.h:32](https://github.com/jsoftware/jsource/blob/e75016ca74b5e595dd323226e6a4990172f72ec6/jsrc/m.h#L32), [m.c:265](https://github.com/jsoftware/jsource/blob/e75016ca74b5e595dd323226e6a4990172f72ec6/jsrc/m.c#L265), [m.c:1239](https://github.com/jsoftware/jsource/blob/e75016ca74b5e595dd323226e6a4990172f72ec6/jsrc/m.c#L1239), [m.c:1302](https://github.com/jsoftware/jsource/blob/e75016ca74b5e595dd323226e6a4990172f72ec6/jsrc/m.c#L1302), [m.c:1510](https://github.com/jsoftware/jsource/blob/e75016ca74b5e595dd323226e6a4990172f72ec6/jsrc/m.c#L1510).

### 초기화와 수명 관리

수치 배열 등 direct 데이터는 할당 후 전체를 0으로 채우지 않는다. 생산 커널이 결과를 기록한다. boxed 배열 등 참조를 담는 indirect 데이터에는 실패 시 안전하게 정리할 수 있도록 초기화가 필요하다.

일반 할당은 임시 정리 스택(tstack)에 등록된다. 연산 종료 시 결과를 보호하고 나머지 임시 객체를 참조 카운트와 함께 정리한다. 이 스택은 메모리 자체를 한 번에 폐기하는 arena가 아니라 정리할 객체의 목록이다. C의 `gc`라는 함수 이름도 tracing GC를 뜻하지 않는다.

참조 카운트에는 제자리 수정 가능 상태와 영구 객체 표시가 결합되어 있다. 영구 객체는 카운트 조작을 건너뛰고, 단독 소유 등 일부 경로에는 빠른 처리가 있다. 동시에 atomic 증가·감소 경로도 존재한다. 따라서 C가 비원자적 카운트만 쓰기 때문에 빠르다고 설명하는 것은 틀리다.

boxed 객체는 매번 전체 자손을 재귀적으로 증가·감소시키지 않도록 소유 관계를 관리한다. Rust에서도 부모를 공유할 때 전체 자손을 clone하는 설계는 피해야 한다.

근거: [m.c:856](https://github.com/jsoftware/jsource/blob/e75016ca74b5e595dd323226e6a4990172f72ec6/jsrc/m.c#L856), [m.c:1302](https://github.com/jsoftware/jsource/blob/e75016ca74b5e595dd323226e6a4990172f72ec6/jsrc/m.c#L1302), [m.c:1373](https://github.com/jsoftware/jsource/blob/e75016ca74b5e595dd323226e6a4990172f72ec6/jsrc/m.c#L1373), [jtype.h:231](https://github.com/jsoftware/jsource/blob/e75016ca74b5e595dd323226e6a4990172f72ec6/jsrc/jtype.h#L231), [jtype.h:647](https://github.com/jsoftware/jsource/blob/e75016ca74b5e595dd323226e6a4990172f72ec6/jsrc/jtype.h#L647).

### 제자리 수정과 뷰

새 임시 배열은 처음부터 제자리 수정 가능한 상태로 태어난다. 마지막 사용을 마친 입력을 결과로 재활용하고, 특정 재할당 대상에는 parser와 협력하는 별도 규칙이 있다. 단순히 참조 카운트가 1이라는 숫자만으로 모든 입력을 덮어써도 된다는 뜻은 아니다. 별칭, 연산의 실패·타입 승격, 평가 순서까지 보존해야 한다.

`virtual`은 보통 헤더만 만들고 원본 payload를 공유한다. virtual의 virtual은 원래 backer로 연결을 단순화한다. 조건이 맞으면 입력 헤더 자체도 재사용한다. `realize`는 필요할 때 실제 복사본을 만든다. 특히 반환되는 작은 뷰 때문에 원본만 살아남는 경우, `gc` 경로가 뷰를 실체화해 backer를 해제할 수 있다. 현재 코드의 판단을 일반적인 크기 비율 휴리스틱으로 설명해서는 안 된다.

더 짧은 수명의 뷰에는 rank 4 이하를 지원하는 스택상의 `fauxvirtual`도 있다. 이 경우 backer 참조 카운트 증가를 피하는 대신, 뷰가 허용된 범위를 벗어나지 않아야 한다.

근거: [m.c:779](https://github.com/jsoftware/jsource/blob/e75016ca74b5e595dd323226e6a4990172f72ec6/jsrc/m.c#L779), [m.c:821](https://github.com/jsoftware/jsource/blob/e75016ca74b5e595dd323226e6a4990172f72ec6/jsrc/m.c#L821), [m.c:856](https://github.com/jsoftware/jsource/blob/e75016ca74b5e595dd323226e6a4990172f72ec6/jsrc/m.c#L856), [ja.h:378](https://github.com/jsoftware/jsource/blob/e75016ca74b5e595dd323226e6a4990172f72ec6/jsrc/ja.h#L378), [jtype.h:691](https://github.com/jsoftware/jsource/blob/e75016ca74b5e595dd323226e6a4990172f72ec6/jsrc/jtype.h#L691).

### 스레드

작은 블록 풀은 스레드별이다. 다른 스레드에서 해제된 작은 블록은 원래 소유 스레드로 묶어서 반환하는 repatriation 큐를 사용한다. 원래 스레드가 풀 회수와 관련 accounting을 담당한다. 단순히 모든 스레드가 같은 풀을 잠그는 설계가 아니다.

근거: [m.c:1453](https://github.com/jsoftware/jsource/blob/e75016ca74b5e595dd323226e6a4990172f72ec6/jsrc/m.c#L1453).

## 2. Rust로 가져올 것과 바꿀 것

| 정책 | RustJ 결정 방향 | 기대 효과·주의점 |
|---|---|---|
| 연속된 동종 원소 | 유지 | 벡터화와 순차 메모리 접근의 기본 |
| 작은 값의 적은 할당 횟수 | 스칼라 inline, 저차원 shape inline부터 도입 | 현재의 shape Vec·Arc 제어부·payload 분리 할당 감소 |
| 임시 입력 재사용 | 소유권을 소비하는 커널 API로 표현 | 읽기+쓰기 외 불필요한 복사·출력 할당 제거 |
| scope 내부 virtual | `ArrayView<'a, T>`와 slice 차용 | rank-cell마다 할당·복사·참조 카운트 조작 방지 |
| scope 밖으로 나가는 virtual | 명시적인 공유 backing storage | 값의 수명을 안전하게 유지하되 큰 원본 붙잡기 문제 측정 |
| 수치 결과 초기화 생략 | `MaybeUninit` 기반 내부 생성 경로 유지 | 0 초기화 후 덮어쓰는 추가 쓰기 제거 |
| 작은 객체 풀 | inline 도입 후 남는 병목에 한해 도입 | 풀부터 만들면 없어질 할당을 최적화할 수 있음 |
| 큰 블록 power-of-two | 정렬을 만족하는 필요 용량 중심으로 변경 | 경계에서 발생하는 큰 내부 낭비 방지 |
| 큰 버퍼 재활용 | 크기·총량 한도가 있는 캐시를 실험 | 반복 할당 비용 감소와 RSS 증가의 균형 필요 |
| 수작업 tstack·zap 포인터 | 이동·Drop·차용으로 대체 | 정상·오류 경로 정리를 같은 소유권 규칙으로 처리 |
| atomic 비용 | 우선 공유 횟수를 줄이고 단독 소유 상태를 분리 | 모든 값을 Rc로 바꾸면 향후 스레드 이동에 제약 |
| 비트에 압축된 상태 | 타입과 명시적인 불변 조건을 먼저 사용 | 실제 크기·분기 비용을 측정한 후 압축 여부 결정 |

Rust의 소유권 검사는 런타임의 동적 공유 관계를 모두 없애주지 않는다. 공유 boxed 값에는 여전히 수명 관리가 필요하며, 순환 가능한 런타임 객체가 추가되면 Rc/Arc만으로 순환 해제를 해결할 수 없다. 병렬 실행에서 Rc를 Arc로 전환하는 것도 살아 있는 별칭과 소유권 이전을 고려한 설계가 필요하다.

정렬된 사용자 할당 버퍼는 할당 때 사용한 크기·정렬·할당기를 해제까지 보존해야 한다. 이를 일반 Vec의 raw parts로 무조건 감싸서는 안 된다. tail padding도 C를 그대로 흉내 내기보다 커널이 실제로 요구하는지 확인한다. `MaybeUninit`을 쓴다고 할당 범위를 넘는 읽기나 미초기화 값 읽기가 자동으로 허용되는 것은 아니다.

## 3. 제안하는 배열 구조

논리적인 `Array`는 타입·원소 수·shape·storage를 표현한다. `Shape`는 자주 쓰이는 낮은 rank를 inline으로 저장하고 큰 rank만 추가 할당한다. 정확히 몇 축을 inline으로 둘지는 Array 크기 증가와 실제 rank 분포를 비교해 정한다.

`Storage`는 inline scalar, 단독 소유 dense buffer, 공유 backing storage를 구분한다. 커널 내부는 타입이 결정된 slice 또는 mutable slice를 받아 원소마다 타입 분기를 하지 않는다. 공유 상태로의 전환은 실제로 값을 공유해야 할 때 수행한다. C와 같은 가변 길이 단일 할당 구조는 작은 배열의 추가 후보로 비교하되, 전체 엔진이 그 구조에 의존하도록 먼저 고정하지 않는다.

`ArrayView<'a, T>`는 scope 안에서 shape와 연속 영역을 차용한다. 이후 필요한 연산을 위해 stride를 지원할 수 있지만, 일반 dense 커널은 연속 경로를 별도로 유지한다. 반환·전역 저장 등 scope 밖으로 나갈 때만 공유 소유권을 부여하거나 복사한다. 작은 뷰가 매우 큰 backer를 유지할 때에는 실제 retained bytes와 복사량에 근거한 실체화 정책을 둔다.

재할당은 성공한 새 값을 환경에 반영하는 시점까지 기존 변수의 관찰 가능한 의미를 지켜야 한다. 예를 들어 정수 overflow 후 실수로 승격하거나 오류를 반환하는 경로에서는 파괴적인 입력 재사용이 안전한지 별도로 검증한다.

## 4. 큰 배열에 대한 구체적 개선 여지

64비트 rank-1 i64 배열의 헤더와 shape는 64바이트다. C의 일반 할당 계산에서:

| 원소 수 | 헤더 포함 필요량 | power-of-two 블록 크기 |
|---|---:|---:|
| 1,000,000 | 8,000,064바이트 | 8,388,608바이트 |
| 4,000,000 | 32,000,064바이트 | 33,554,432바이트 |
| 1,048,576 | 8,388,672바이트 | 16,777,216바이트 |

여기에 C의 정렬·tail 공간이 추가된다. 마지막 예는 payload가 정확히 8MiB여도 헤더 때문에 거의 두 배의 블록을 요청한다. 이것은 요청 용량의 낭비이며, 모든 바이트가 즉시 물리 메모리로 상주한다는 뜻은 아니다. Rust는 고정 크기 연산 결과를 필요한 용량으로 할당하고, append처럼 성장하는 배열에는 별도의 기하급수적 용량 증가 정책을 적용할 수 있다.

앞서 동일 환경에서 기록한 warmed `r=:a+2` 20회 진단은 1백만 원소에서 Rust/C minor faults가 0/1, 4백만 원소에서 0/156,260이었다(`page-faults.jsonl`). 반복 할당 시 페이지 관리 차이가 중요하다는 증거다. 그러나 이 수치만으로 malloc 내부의 mmap 선택·해제 임계값을 확정할 수 없다. 소스에서 확인한 일반 큰 배열 경로는 `malloc/free`다. syscall 추적이나 allocator 통계가 있어야 정확한 원인을 더 좁힐 수 있다.

또한 큰 배열 덧셈은 메모리 대역폭에 제한될 수 있다. 같은 바이트 수를 읽고 쓴다면 Rust라는 언어 자체가 C보다 빠름을 보장하지 않는다. 승부점은 할당, 복사, 초기화, 재사용, 그리고 여러 연산 사이의 중간 배열을 얼마나 줄이는가다.

## 5. 적용 순서와 검증

1. 할당 횟수·요청 용량·복사량·minor faults·peak/retained RSS를 같은 workload에서 기록한다. 결과 검증은 타이밍 밖에서 수행한다.
2. inline scalar와 shape, 차용 뷰를 도입한다. 스칼라, 작은 배열, rank 연산에서 효과를 먼저 확인한다.
3. 단독 소유와 공유 storage를 분리하고 소비형 커널 API를 정리한다. 공유 입력 불변성, 겹치는 뷰, 정수 승격, 오류 경로를 검증한다.
4. faer 방식의 호출자 제공 scratch 공간과 출력 버퍼를 분리한다. 반복 실행에서 필요한 scratch를 재사용하고 scope 밖으로 탈출하지 못하게 한다.
5. 큰 출력 버퍼의 정확한 용량·정렬과 한도 있는 재사용 캐시를 비교한다. 캐시에는 크기별 개수뿐 아니라 전체 바이트 상한과 큰 블록 제외/반환 정책이 필요하다. scratch 재사용과 출력 캐시는 따로 측정한다.
6. 작은 객체 풀이 여전히 필요한지 측정한다. 병렬 커널 도입 시 스레드 간 소유권 이동과 원격 반환 정책을 별도로 검증한다.

크기 테스트는 64/128/256/512/1024바이트 클래스 경계와 큰 2의 거듭제곱 경계의 전후를 포함한다. n=0, scalar, rank 1/2/4/5, 긴 반복 실행, 크기를 바꾸는 실행, 작은 뷰가 큰 배열에서 탈출하는 경우도 필요하다. 지속 실행에서 시간만 좋아지고 retained RSS가 계속 증가하는 캐시는 성공으로 판정하지 않는다.

현재 구현은 `Value { shape: Vec<usize>, data: Data }`와 `Arc<Vec<T>>`를 사용한다. 단독 소유 입력 재사용과 수치 출력의 불필요한 0 초기화 제거는 이미 있지만, inline scalar/shape, 일반 차용 배열 뷰, 전용 버퍼 캐시는 없다. 따라서 우선순위는 C 헤더를 문자 그대로 복제하는 것보다 이 세 가지 할당·복사 원인을 제거하는 것이다.
