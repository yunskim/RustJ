# RustJ — J 배열 컴파일러

RustJ는 J의 언어 및 배열 의미론을 보존하면서, **Jaxa 기반 분석·계획 계층을 통해 CPU와 GPU용 최적화 코드를 생성하는 Rust 기반 J 컴파일러**를 목표로 합니다. 기존 C 코드를 번역하거나 C 엔진으로 fallback하지 않으며, `jsource`는 의미·오류·성능을 검증하는 reference implementation으로 사용합니다.

채택한 목표 파이프라인은 다음과 같습니다.

```text
J Source
  -> Parser
  -> Semantic IR
  -> Jaxa Analyzer
  -> Logical Execution Plan
  -> Physical Planner / Optimizer
  -> Physical Execution Plan
  -> Backend Lowering / Code Generation
  -> Runtime / Executor
```

CPU와 GPU는 같은 Logical Plan에서 출발하는 동등한 backend입니다. J의 `rank/cell/frame/agreement`는 의미 계층에 그대로 두고, stride·offset·layout·tiling·device placement·sharding은 physical planning 계층에서 결정합니다. 전체 설계 기준은 [컴파일러 아키텍처](reports/COMPILER-ARCHITECTURE.md)에 있습니다.

**현재 구현은 아직 위 compiler pipeline 전체를 구현하지 않았습니다.** 제한된 기능의 CPU 직접 평가 엔진, portable/AVX2 커널, 차등 검증 도구가 동작합니다. 이 경로는 새 compiler의 의미 기준선과 향후 reference interpreter/debug/fallback 기반으로 유지하면서, Semantic IR과 Jaxa Analyzer부터 단계적으로 compiler 경로를 세웁니다. 기존 J 전체를 대체한다고 주장하지 않습니다.

첫 GPU backend 후보는 CUDA이며, GPU 친화적 배열 설계는 목표 아키텍처에 반영했습니다. 다만 실제 CUDA 구현은 현재 보류 상태입니다. GPU가 없는 Linux에서도 CPU 빌드·검증을 유지하고, GPU 구현을 재개할 때 logical JArray와 physical array representation을 분리한 구조로 진행합니다.

현재 구현·검증 상태는 [M2 재검토](reports/M2-REVIEW.md), [지속 검증 전략](reports/VALIDATION-STRATEGY.md)에 있고, 이후 단계는 [구현 계획](reports/IMPLEMENTATION-PLAN.md), Jaxa에서 가져오는 의미/계획 분리는 [JAXA 검토](reports/JAXA-REVIEW.md)에 정리되어 있습니다. CPU 성능과 C AVX2 비교는 [최적화 보고서](reports/PERFORMANCE.md)를 참고합니다.

## 빌드와 실행

이 디렉터리에서 실행합니다.

```sh
cargo build --release
./target/release/rustj -e '+/"1 i. 2 3'
# 3 12
./target/release/rustj examples/milestone.ijs
./target/release/rustj
```

인자 없이 실행하면 한 줄에 한 문장을 읽습니다. `Ctrl-D`로 종료합니다. 스크립트는 첫 오류에서 종료합니다. 표준 입력 세션은 오류 이후에도 계속 읽되 종료 코드는 실패입니다.

```sh
printf 'a =: i. 4\na + 2\n' | ./target/release/rustj --json
```

JSON 출력은 타입 코드(1 Boolean, 2 literal bytes, 4 integer, 8 float), shape, 평탄화된 data를 포함합니다. 할당·주석 줄은 `{"silent":true}`, 오류는 `{"error":"length error"}` 형식입니다. NaN과 무한대는 문자열 `nan`, `inf`, `-inf`로 표현합니다. 텍스트 출력은 간단한 표시이며 J의 정확한 포맷 호환을 목표로 하지 않습니다.

## 구현 범위

| 기능 | 범위 |
|---|---|
| 값 | Boolean, i64, f64, 바이트 문자 배열 |
| 리터럴 | 숫자 스칼라·벡터, 밑줄 음수, 소수·지수, 무한대·NaN 리터럴, 작은따옴표 문자열 |
| 평가 | 우측부터 평가, 괄호, `NB.` 주석, 전역 명사 바인딩 `=:` |
| 산술 | `+ - * %`의 기본 실수 범위 모나드·다이애드, 모나드 `|` |
| 비교 | `= < >` 다이애드, 기본 부동소수점 허용오차 |
| 배열 | `i.` 모나드, `$ # ,` 모나드, `$ {` 다이애드, 스칼라·벡터 catenate `,` |
| 확장 | 스칼라 확장과 선행 형상 일치에 따른 확장 |
| reduction | `+/ -/ */ %/`의 기본 우측 fold, 빈 `+/ */` identity |
| rank | 정수 하나를 사용하는 모나드 rank, 예: `+/"1`, `$"1`, `#"0` |
| 오류 | domain, length, rank, index, value, limit, syntax; 미구현 기능은 unsupported |

rank를 적용한 빈 frame의 prototype 추론, 서로 다른 형상 결과의 padding은 미구현입니다. 부동소수점 값의 정수 변환은 현재 정확히 정수인 값만 허용합니다. 고급 수치 경계의 완전한 호환성은 아직 보장하지 않습니다.

미구현: 박스·희소·복소수·확장 정수·유리수·추가 수치 정밀도, Unicode 변환, verb 바인딩·train·함수 정의·제어 흐름·locale, 일반 adverb/conjunction, 다이애드 rank와 rank 목록, scan, 시스템 foreign, 파일 API, 직렬화, 임베딩 ABI, 병렬 실행. 선언하지 않은 일부 구문은 syntax/value 오류로 거부될 수 있습니다. undefined 이름을 지연된 verb로 취급하는 J의 동작도 지원하지 않습니다.

## 현재 구현의 내부 구조와 메모리

- `src/syntax.rs`: compact token, 원문 이름 차용과 리터럴
- `src/runtime.rs`: 제한된 문장 평가와 명사 환경
- `src/value.rs`: 논리 배열 값과 타입
- `src/storage.rs`: inline shape, 단독·공유 CPU 버퍼, 수명으로 제한한 차용 뷰
- `src/kernels.rs`: 연산, rank, reduction
- `src/assembly.rs`: rank 결과의 순차 조립과 타입 승격
- `src/numeric.rs`: 단일 순회 수치 커널, overflow 복구
- `src/simd.rs`: 실행 시 CPU 확인을 거치는 AVX2 커널
- `src/main.rs`: 콘솔·스크립트·JSON 입출력

값은 소유권을 넘겨 커널에 전달합니다. `CpuStorage<T>`는 Inline, Owned(Vec), Shared(Arc<Vec>)를 구분합니다. scalar와 rank 4 이하 shape는 내부 저장하고, 이름에 바인딩할 때만 `into_shared`로 전환합니다. 단독 소유 정수 add/sub는 출력·제어 블록·shape의 추가 할당 없이 기존 버퍼를 재사용합니다. 공유 입력은 새 출력에 직접 계산합니다.

Rust API에서 소유 값의 `clone()`은 데이터를 복사합니다. 복사 없이 공유할 때는 `value.into_shared()` 후 clone합니다. `ArrayView<'a>`는 원본의 shape와 CPU slice를 차용하며 rank/reduction 입력 셀 생성에 사용합니다. 결과 조립과 일부 소유 출력의 복사는 남아 있습니다. CUDA device storage에는 CPU slice API를 제공하지 않는 방향이며 실제 CUDA 구현은 아직 없습니다.

정수 쌍의 `+ - *`는 타입·연산을 루프 밖에서 결정한 const-generic 경로를 사용합니다. scalar와 같은 형상 경로는 원소별 나눗셈을 피합니다. 정수 계산과 overflow 감지를 한 번에 수행하고 실제 overflow에서만 전체 결과를 float로 승격합니다. 제자리 add/sub는 wrapping 연산의 역연산으로 원래 피연산자를 정확히 복원합니다. 곱셈은 제자리 복구를 하지 않고 새 출력에 기록합니다.

64원소 이상의 지원되는 배열에서는 AVX2를 실행 시 감지합니다. 정수 add/sub, 정수 scalar multiply, float add/sub를 지원합니다. 미지원 CPU는 portable 경로로 실행하며 `cargo test --features portable`로 강제 검증할 수 있습니다. CPU별 빌드나 AVX2 CPU를 실행의 필수 조건으로 요구하지 않습니다. 정수 scalar multiply는 정확한 i128 나눗셈으로 안전한 피연산자 범위를 계산한 뒤 AVX2 32비트 부분 곱으로 하위 64비트 결과를 구성합니다.

라이브러리는 기본적으로 `unsafe`를 거부하며, 초기화되지 않은 출력 버퍼의 길이 확정과 SIMD 모듈에만 제한적으로 허용합니다. SIMD 진입점은 길이·CPU 기능·중첩 조건을 확인하고 초기화 완료 후에만 결과를 공개합니다. 크기가 큰 데이터 버퍼에는 실패 가능한 예약과 shape overflow 검사를 사용합니다. 모든 작은 메타데이터·문자열 할당까지 회복 가능한 OOM을 보장하지는 않습니다. rank는 최종 버퍼에 셀을 순차 조립하고, 정수 add/sub reduction은 누적 버퍼를 재사용합니다. 다른 reduction과 일부 소유 출력에는 중간 할당이 남아 있습니다. scope 내 차용 뷰는 구현했고, scope 밖으로 나가는 공유 뷰·전용 scratch/output pool·병렬 커널은 아직 없습니다. inline Value가 Token을 키우던 문제는 compact scalar token과 boxed 배열 token으로 개선했습니다. 이름은 원문을 차용하므로 공개 `Token` API에는 source lifetime이 있습니다.

## 검증

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test --features portable
```

테스트에는 우측 평가, rank, 형상, 빈 배열, overflow 승격, 큰 정수의 정확한 비교, 이름 별칭 보존, 실패한 할당의 상태 보존, 실제 포인터 재사용, CLI 종료 상태, 제한된 malformed-input smoke fuzz가 포함됩니다. sanitizer/Miri 검증이나 완전한 퍼징을 수행했다는 의미는 아닙니다.

### 실제 J와 비교

Linux에 GCC, make, Python 3, git이 필요합니다. 참조 소스는 `e75016ca74b5e595dd323226e6a4990172f72ec6`으로 고정합니다.

```sh
git clone https://github.com/jsoftware/jsource.git /path/to/jsource
git -C /path/to/jsource checkout e75016ca74b5e595dd323226e6a4990172f72ec6
python3 tools/build_reference.py --source /path/to/jsource
cargo build --release
python3 tools/conformance.py
```

빌드 도구는 참조 소스를 `.reference/`로 추출하여 그 복사본만 빌드합니다. 기본 `--source`는 `../jsource-inspection`입니다. Python oracle은 **별도 프로세스**에서 해당 `libj.so`를 로드하고 `JDo`/`JGetM`으로 값을 추출합니다. RustJ는 이 라이브러리를 로드하지 않습니다.

차등 검증은 제한된 기능을 대상으로 직접 작성한 사례와 고정 seed 생성 사례입니다. upstream의 444개 `.ijs` 파일을 실행한 것이 아닙니다. 보고서는 총 사례 수, 통과·실패·제외 수와 차이를 기록합니다. float 결과 비교는 상대 허용오차 `1e-14`, Boolean 비교 결과와 정수·형상·타입은 정확히 비교합니다.

### 성능과 할당

```sh
python3 tools/benchmark.py
```

기존 배열에 대한 정수 덧셈과 소유권을 넘기는 Rust 커널을 측정합니다. 3회 중앙값이며, 컴파일·프로세스 시작·출력은 측정 구간에서 제외합니다. 별도의 한 번 실행에서 Rust 할당 횟수, 누적 할당 바이트, 추가 live bytes 최댓값을 수집합니다. 할당 계측의 `unsafe`는 벤치마크 파일에만 있으며 System allocator로 전달합니다.

C 측 호출에는 Python ctypes 비용이 들어가므로 작은 배열의 시간은 직접적인 언어 성능 비교로 해석하면 안 됩니다. 참조 C 빌드는 기본 j64이고 AVX2/AVX512 최적화 빌드가 아닙니다. **Rust가 기존 J보다 빠르다는 결론을 내리는 벤치마크가 아닙니다.**

결과와 제한은 [첫 검증 보고서](reports/MILESTONE-1.md), 기계 판독 결과는 `reports/conformance.json`, `reports/benchmark.json`에 있습니다.

### 개선 전후와 C AVX2의 공정한 재측정

이전 측정은 역사적 자료입니다. 새 시간 비교는 할당 계측기와 Python 호출을 측정 경로에서 제거했습니다. 두 엔진을 한 프로세스에서 호출하고 동일한 문장을 파싱·실행합니다.

```sh
python3 tools/build_reference.py --source /path/to/jsource --variant j64avx2
J_LIBRARY="$PWD/.reference/bin/linux/j64avx2/libj.so" cargo bench --bench comparison
```

CPU 고정·첫 마일스톤과의 비교는 `dist/rustj-m1-source.tar.gz`가 있을 때 아래처럼 실행합니다. 모든 빌드를 먼저 마친 뒤 다른 작업 없이 측정합니다.

```sh
python3 tools/prepare_before.py
cargo bench --bench comparison --no-run
cargo bench --manifest-path .baseline/Cargo.toml --bench comparison --no-run
python3 tools/compare.py
```

이전 소스 묶음이 없으면 현재 엔진과 C의 비교만 실행할 수 있습니다. 결과는 `reports/comparison.json`에 조건·소스 해시·원시 중앙값/최소/최대와 함께 저장합니다. native comparison 벤치마크는 Linux 전용이며 `J_LIBRARY`를 명시해야 합니다. C 라이브러리는 벤치마크에서만 로드합니다.

## 다음 구현 우선순위

1. **Semantic IR과 primitive contract**: parser와 직접 실행을 분리하고 source span, name/version, dtype/shape/rank/error/effect/alias 계약을 명시합니다.
2. **Jaxa Analyzer와 Logical Plan**: rank/cell/frame/agreement를 해석해 `RankMap`, Map, Reduce, Scan, Gather, Structural operation으로 구성된 실행 계획을 만듭니다. Analyzer는 실행하지 않습니다.
3. **Physical Planner와 CPU compiler backend**: logical ValueId와 physical BufferId를 분리하고 strides/view, liveness, materialization, layout, fusion을 계획합니다. 먼저 CPU를 통해 compiler pipeline을 기본 실행 경로로 검증합니다.
4. **GPU backend**: device placement, transfer, dense/tiled layout, frame/cell 기반 GPU work partition, kernel codegen과 GPU 상주 실행을 추가합니다. 실제 CUDA 구현은 재개 요청과 검증 가능한 GPU 환경이 마련될 때 진행합니다.
5. **JIT specialization/cache와 multi-device**: dtype/rank/shape-layout class/backend capability를 이용한 specialization, compiled artifact cache, multi-GPU sharding을 확장합니다.

각 단계의 완료 기준과 CPU/GPU 검증 정책은 [구현 계획](reports/IMPLEMENTATION-PLAN.md)을 따릅니다.

`.github/workflows/linux.yml`은 이 디렉터리를 저장소 루트로 게시했을 때 사용할 Linux CI 설정입니다. 로컬에서 검증했으며 원격 CI 실행은 아직 하지 않았습니다.

## 지속 검증

변경마다 [검증 전략](reports/VALIDATION-STRATEGY.md)을 적용합니다. Linux CI는 C 두 빌드 × Rust 기본/portable을 비교하고, 고정 seed 상태 테스트 및 일일 확장 테스트를 실행합니다. 알려진 기준선 차이는 통과와 분리하며 실패 재현 이력과 보고서를 보관합니다. upstream 전체 테스트와 sanitizer/Miri는 아직 미실행입니다.

## 배열 조작 verb

- `|. y`: 첫 축의 항목 순서 뒤집기. `|. i.2 3`은 행 순서를 뒤집습니다.
- `n |. y`: 첫 축 회전. 양수는 왼쪽, 음수는 오른쪽으로 회전합니다.
- `|: y`: 축 순서를 역순으로 전치합니다. 행렬에서는 행·열 교환입니다.
- `n {. y`: 앞/뒤에서 항목 가져오기. 부족하면 숫자는 0, 문자는 공백으로 채웁니다.
- `n }. y`: 앞/뒤에서 항목 버리기. 범위를 넘으면 첫 축 길이가 0입니다.

현재 n은 스칼라 정수입니다. 여러 축의 개수 목록, dyadic 전치, monadic head/tail, 사용자 지정 fill은 미지원입니다. 입력은 보존하며 결과를 한 번 할당해 직접 채웁니다(별도 인덱스 배열 없음). 일반 경로는 데이터 복사가 있으며 전용 SIMD 가속이나 무복사 strided view를 제공한다는 뜻은 아닙니다.

## i. 계열

- `i. y`: 기존 인덱스 배열 생성(음수 축 포함).
- `x i. y`: x의 항목에서 y의 첫 위치 검색. 미발견은 x의 항목 수.
- `x i: y`: 마지막 위치 검색. 다차원 x에서는 첫 축의 항목 단위로 비교.
- `i: n`: 실수 스칼라 n의 -n부터 n까지 1 간격 생성. 음수 n은 역순. 예: `i:2.5` → `_2.5 _1.5 _0.5 0.5 1.5 2.5`.
- `I. y`: scalar/vector의 비음수 정수 개수만큼 인덱스 반복. `I.2 0 3` → `0 0 2 2 2`.

정수·불리언 원소 검색은 해시 테이블을 사용합니다. 실수·문자·다차원 항목 검색은 현재 순차 비교하므로 큰 검색의 성능은 별도 개선 대상입니다. `i:` 배열 입력의 결과 padding, 복소 간격 지정, 고차원 `I.`, dyadic `I.` 구간 검색은 미지원입니다.

`e.`와 `E.`도 검색 계열에 포함합니다. `x e. y`는 y에 x의 항목이 있는지 반환하고, `'ana' E. 'banana'`는 `0 1 0 1 0 0`처럼 패턴 시작 위치를 표시합니다. E.는 현재 스칼라/벡터/문자열만 지원하며 겹침을 허용하고 빈 패턴도 처리합니다. 다차원 E.와 KMP 등의 전용 검색 가속은 미구현입니다. e.는 index-of와 같은 항목 shape 및 수치 비교 규칙을 사용합니다.
