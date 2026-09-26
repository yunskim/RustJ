# RustJ — Linux 첫 마일스톤

J의 일부 문장을 독립적으로 실행하는 Rust 엔진입니다. 기존 C 코드를 번역하거나 C 엔진으로 fallback하지 않습니다. 표준 Rust 라이브러리만 사용하며 Cargo 외부 의존성이 없습니다.

**현재는 제한된 기능의 실험용 인터프리터입니다. 기존 J 전체를 대체하지 않습니다.** Linux x86-64, Ubuntu 24.04/WSL2와 Rust 1.98.0에서 검증했습니다. 배포 파일은 Linux ELF이며 Windows 실행 파일이 아닙니다.

주요 목표는 C 커널에 의존하지 않는 Rust 구현, 빠른 CPU 배열 연산, 그리고 **CUDA GPU에서 배열을 유지하며 연속 연산을 실행하는 것**입니다. GPU 지원은 아직 미구현이며 첫 GPU 백엔드는 CUDA로 한정합니다. 단계별 범위와 완료 기준은 [구현 계획](reports/IMPLEMENTATION-PLAN.md)에 있습니다. CUDA 의존성은 선택 사항으로 도입하여 GPU 없는 Linux에서도 CPU 빌드·실행을 유지할 계획입니다.

현재 성능 개선 내용과 C AVX2와의 비교는 [최적화 보고서](reports/PERFORMANCE.md)에 있습니다. 첫 마일스톤 보고서는 변경 전의 역사적 기록입니다.

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

## 내부 구조와 메모리

- `src/syntax.rs`: 토큰화와 리터럴
- `src/runtime.rs`: 제한된 문장 평가와 명사 환경
- `src/value.rs`: 타입별 연속 버퍼와 형상
- `src/kernels.rs`: 연산, rank, 결과 조립
- `src/numeric.rs`: 단일 순회 수치 커널, overflow 복구
- `src/simd.rs`: 실행 시 CPU 확인을 거치는 AVX2 커널
- `src/main.rs`: 콘솔·스크립트·JSON 입출력

값은 소유권을 넘겨 커널에 전달합니다. 현재 초기 저장소는 `Arc<Vec<T>>`로 구현했고, 이름에서 값을 읽을 때 저장소를 공유합니다. 최적화된 정수 덧셈·뺄셈은 단독 소유 버퍼를 `Arc::try_unwrap`으로 회수하여 재사용합니다. 공유 입력은 복사하지 않고 새 버퍼에 결과를 직접 기록하여 원본을 보존합니다. 아직 일반 혼합 타입 경로에는 복사가 남아 있습니다. 메타데이터의 작은 할당도 존재합니다. 이 표현은 최종 설계가 아니며 단독 소유 저장소와 공유 저장소를 분리하는 후속 개선이 가능합니다.

정수 쌍의 `+ - *`는 타입·연산을 루프 밖에서 결정한 const-generic 경로를 사용합니다. scalar와 같은 형상 경로는 원소별 나눗셈을 피합니다. 정수 계산과 overflow 감지를 한 번에 수행하고 실제 overflow에서만 전체 결과를 float로 승격합니다. 제자리 add/sub는 wrapping 연산의 역연산으로 원래 피연산자를 정확히 복원합니다. 곱셈은 제자리 복구를 하지 않고 새 출력에 기록합니다.

64원소 이상의 지원되는 배열에서는 AVX2를 실행 시 감지합니다. 정수 add/sub, 정수 scalar multiply, float add/sub를 지원합니다. 미지원 CPU는 portable 경로로 실행하며 `cargo test --features portable`로 강제 검증할 수 있습니다. CPU별 빌드나 AVX2 CPU를 실행의 필수 조건으로 요구하지 않습니다. 정수 scalar multiply는 정확한 i128 나눗셈으로 안전한 피연산자 범위를 계산한 뒤 AVX2 32비트 부분 곱으로 하위 64비트 결과를 구성합니다.

라이브러리는 기본적으로 `unsafe`를 거부하며, 초기화되지 않은 출력 버퍼의 길이 확정과 SIMD 모듈에만 제한적으로 허용합니다. SIMD 진입점은 길이·CPU 기능·중첩 조건을 확인하고 초기화 완료 후에만 결과를 공개합니다. 크기가 큰 데이터 버퍼에는 실패 가능한 예약과 shape overflow 검사를 사용합니다. 모든 작은 메타데이터·문자열 할당까지 회복 가능한 OOM을 보장하지는 않습니다. rank 결과 조립과 reduction은 정확성 우선 구현으로 중간 할당이 많습니다. 유지되는 zero-copy 뷰, arena, 메모리 풀, 병렬 커널은 아직 없습니다.

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

1. CPU/CUDA 저장소 경계를 포함한 배열 소유권 설계, inline scalar/shape와 차용 뷰 도입.
2. 선택적 CUDA 빌드와 장치 버퍼·전송·완료 이벤트 구현, 첫 i64/f64 배열 커널 검증.
3. GPU 상주 연속 연산, 제한된 버퍼 풀, 명시적인 CPU 전환과 전송 포함 성능 검증.
4. CPU reduction·rank의 중간 배열 제거와 배열 의미 확대: 빈 frame prototype, dyadic rank, 결과 padding.
5. 의미 보존이 검증된 GPU reduction·행렬 연산 확대 및 제한된 연산 결합.
6. verb·수정자 표현과 실행 규칙을 정식화한 후 함수·박스 지원.

각 단계의 완료 기준과 CPU/GPU 검증 정책은 [구현 계획](reports/IMPLEMENTATION-PLAN.md)을 따릅니다.

`.github/workflows/linux.yml`은 이 디렉터리를 저장소 루트로 게시했을 때 사용할 Linux CI 설정입니다. 로컬에서 검증했으며 원격 CI 실행은 아직 하지 않았습니다.
