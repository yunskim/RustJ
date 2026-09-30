# RustJ — J 배열 컴파일러

RustJ는 J의 언어·배열 의미론을 보존하면서 CPU와 GPU를 동등한 실행 대상으로 삼는 Rust 기반 배열 컴파일러를 목표로 합니다.

`jsource`를 줄 단위로 번역하는 프로젝트가 아니라, J frontend와 배열 컴파일러 middle-end를 분리합니다.

```text
J Source
  ↓
RustJ frontend
  ↓
J Semantic Array IR
  │  noun / verb / adverb / conjunction
  │  hook / fork / train / derived verb / rank
  ↓
Jaxa
  ↓
Logical Array IR / Execution Plan
  ↓
Physical Plan
  ↓
CPU / GPU backend
  ↓
Runtime / Executor
```

**상세 아키텍처, RustJ/Jaxa 경계, Array IR, 구현 계획, 체크리스트, 지원 범위와 검증 정책의 유일한 기준 문서는 [PROJECT.md](PROJECT.md)입니다.**

## 현재 상태

현재 저장소는 목표 compiler pipeline으로 이동 중인 전환 단계입니다.

- 제한된 J frontend와 CPU 직접 실행 경로
- J Semantic IR 기초
- primitive contract와 dtype/shape/rank fact
- 초기 LogicalPlan
- CPU Inline/Owned/Shared storage
- runtime AVX2 + portable fallback
- sparse/boxed/packed-bit 기반 일부
- read-only affine PhysicalArray(G1)
- hook/fork/train/derived verb를 보존하는 J Semantic Array IR → Jaxa 경계는 아직 완전 분리되지 않음
- 실제 CUDA backend는 아직 미구현

다음 compiler architecture 작업은 현재 Semantic IR이 J의 verb composition을 충분히 보존하는지 감사하고, `J Semantic Array IR → Jaxa → Logical Array IR/Plan` 경계를 코드에서 명시하는 것입니다.

## 빌드와 실행

```sh
cargo build --release
./target/release/rustj -e '+/"1 i. 2 3'
./target/release/rustj examples/milestone.ijs
./target/release/rustj
```

JSON 출력:

```sh
printf 'a =: i. 4\na + 2\n' | ./target/release/rustj --json
```

## 기본 검증

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test --features portable
```

추가 conformance·성능·메모리 검증 원칙은 [PROJECT.md](PROJECT.md)의 검증 절을 따릅니다.

## 문서 정책

사람이 유지하는 설계·계획·진행 Markdown 문서를 더 늘리지 않습니다.

- 프로젝트 기준: `PROJECT.md`
- 사용 진입점: `README.md`
- 실측/기계 원자료: `reports/*.json`, `reports/*.jsonl`

과거 개별 Markdown 보고서의 세부 이력은 Git history에서 확인합니다.
