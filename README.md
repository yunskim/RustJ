# RustJ — J 배열 컴파일러

RustJ는 J의 언어·배열 의미론을 보존하면서 CPU와 GPU를 동등한 실행 대상으로 삼는 Rust 기반 배열 컴파일러를 목표로 합니다.

`jsource`를 줄 단위로 번역하는 프로젝트가 아니라, J frontend·semantic analysis·logical/physical planning·backend를 분리한 하나의 compiler system으로 설계합니다.

`Jaxa`는 현재 아키텍처의 별도 컴포넌트명이 아닙니다. 과거 `jaxa-analyzer` 연구 저장소의 아이디어는 RustJ middle-end 설계에 흡수합니다.

```text
J Source
  ↓
Frontend
  ↓
J Semantic Array IR
  ↓
Semantic Analyzer / Lowering
  ↓
Verified Logical Array IR / Execution Plan
  ↓
Route Partition
  │
  ├─ region(s): RustJ native → Logical Optimizer → Schedule → Physical Plan → Executor
  ├─ region(s): MLIR family → LLVM/NVVM/ROCDL/SPIR-V
  ├─ region(s): StableHLO-compatible subset → external compiler
  └─ region(s): verified library/custom-kernel route
```

**상세 아키텍처, compiler stage 경계, IR, 구현 계획, 체크리스트, 지원 범위와 검증 정책의 유일한 기준 문서는 [PROJECT.md](PROJECT.md)입니다.**

**왜 RustJ가 compiler-oriented architecture를 택하면서도 J semantics를 그대로 보존해야 하는지에 대한 설계 근거와 회귀 판정 기준은 [FOUNDATIONS.md](FOUNDATIONS.md)입니다. frontend·Semantic IR·interpreter/JIT/AOT 경계·rank/CellApply·target architecture를 바꾸기 전 반드시 검토합니다.**

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
- hook/fork/train/derived verb를 보존하는 J Semantic Array IR → Semantic Analyzer/Lowering 경계는 아직 완전 분리되지 않음
- 실제 CUDA backend는 아직 미구현

현재 compiler architecture 작업의 초점은 이미 들어간 shared FunctionEntity/hook/fork/rank-conjunction 구조를 바탕으로 valence별 innate rank contract와 logical `CellApply` planner를 추가하고, 동시에 `J Semantic Array IR → Semantic Analyzer/Lowering → Logical Array IR/Plan` 경계를 코드에서 명시하는 것입니다.

Logical Array IR 이후 실행 경로는 하나로 고정하지 않습니다. RustJ-native planner/executor 외에도 MLIR/LLVM 계열, StableHLO-compatible subset, SPIR-V/NVVM/ROCDL, 검증된 외부 library/kernel lowering을 사용할 수 있도록 설계합니다. RustJ는 J 의미·legality·lowering 조건을 책임지고, 이미 잘 만들어진 compiler IR과 optimizer를 가능한 범위에서 재사용합니다.

RustJ의 **언어 의미 목표는 장기적으로 J 전체**입니다. 다만 모든 J 기능이 동일한 hardware-aware 최적화 경로에 들어갈 필요는 없습니다. shape/rank/access/effect contract가 충분한 부분은 `Analyzable Array Profile`로 Logical Array IR과 advanced planner/external compiler route를 사용할 수 있고, 나머지는 의미를 보존하는 native/runtime lowering 또는 현재는 명시적 unsupported 상태로 남을 수 있습니다.

확장 primitive는 keyword가 아니라 ordinary J name binding으로 등록합니다. 예를 들어 역사 prototype의 `conv`는 parameterized adverb이고, parameter 적용 뒤에 derived Conv computational verb/op가 만들어지는 구조를 보존합니다.

ordinary name의 실제 품사는 enqueue에서 고정하지 않고 **parser가 그 name을 사용할 때 current local/locale binding을 lookup하여 결정**합니다. 일반 verb/adverb/conjunction name의 late-binding nameref 의미도 보존합니다.

`with`는 ordinary J name에 binding되는 conjunction extension으로 유지하되, typed semantic annotation/contract만 결합합니다. tile/device/register 같은 physical policy를 `with`에 넣지 않습니다.
하드웨어 lowering은 extension에만 적용하지 않습니다. `+`, `*`, `+/`, `|:` 같은 **기존 J primitive도** target-independent semantic capability와 backend/architecture별 lowering capability를 가집니다. 특정 GPU의 tile/register 숫자를 primitive 의미에 넣는 대신, primitive/op identity를 key로 target lowering을 조회합니다.

GPU/CPU target은 하나의 `TargetProfile` blob으로 보지 않고 다음처럼 구분합니다.

```text
BackendFamily → ArchitectureTarget → DeviceProfile → RuntimeProfile
                                      ↓
                              resolved TargetProfile
```

architecture/device별 lowering과 capability override는 사용자 J locale과 분리된 **compiler target locale chain**으로 resolution합니다.

```text
device
  → architecture
  → architecture family
  → backend family
  → cpu/gpu class
  → generic
```

locale lookup은 lowering 후보와 capability provider를 찾는 역할을 하고, 실제 schedule/realization 선택은 legality·ResourceEstimate·CostEstimate가 담당합니다.

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

사람이 유지하는 설계·계획·진행 Markdown 문서를 임의로 더 늘리지 않습니다. `FOUNDATIONS.md`는 사용자가 명시적으로 요청한 **설계 헌법/근거 문서**로서 예외이며, roadmap/checklist를 중복하지 않습니다.

- 설계 근거와 회귀 판정 기준: `FOUNDATIONS.md`
- 프로젝트 아키텍처·계획·체크리스트의 권위 문서: `PROJECT.md`
- 사용 진입점: `README.md`
- 실측/기계 원자료: `reports/*.json`, `reports/*.jsonl`

과거 개별 Markdown 보고서의 세부 이력은 Git history에서 확인합니다.


## 과거 설계 저장소

과거 `JAXA`, `JAXA-complier`, `japchae`, `jaxa-analyzer`에 흩어져 있던 primitive vocabulary, resource model, materialized-array/Flow–Storage, analyzer 설계는 2026-09-30 기준으로 `PROJECT.md`에 통합했습니다. 앞으로 새 설계 결정은 RustJ의 `PROJECT.md`에 직접 기록합니다.
