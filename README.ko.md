[English](README.md) | **한국어 — 정본(canonical)**

# RustJ — J 배열 컴파일러

RustJ는 J의 언어·배열 의미론을 보존하면서 CPU와 GPU를 동등한 실행 대상으로 삼는 Rust 기반 배열 컴파일러를 목표로 합니다.

`jsource`를 줄 단위로 번역하는 프로젝트가 아니라, J frontend·semantic analysis·logical/physical planning·backend를 분리한 하나의 compiler system으로 설계합니다.

`Jaxa`는 현재 아키텍처의 별도 컴포넌트명이 아닙니다. 과거 `jaxa-analyzer` 연구 저장소의 아이디어는 RustJ middle-end 설계에 흡수합니다.

## 장기 비전 — “NN의 SQL”

JAXA의 원래 슬로건과 장기 목표는 **“NN의 SQL”**이었습니다.

여기서 SQL은 문법을 뜻하는 것이 아니라 **역할 분리**를 뜻합니다.

```text
사용자 / 모델 작성자
    ↓
무엇을 계산할지 선언
    ↓
배열·NN 계산의 logical graph / algebra
    ↓
rewrite / equivalence / resource reasoning
    ↓
여러 physical execution plan 후보
    ↓
target·resource·cost에 따른 선택
    ↓
CPU / GPU / library / accelerator 실행
```

관계형 데이터베이스에서 사용자가 join 순서, index access, buffer 배치 같은 physical plan을 직접 지정하지 않아도 되는 것처럼, JAXA의 비전은 **신경망·배열 계산에서도 사용자가 계산의 의미를 표현하고 compiler가 실행 전략을 선택하게 하는 것**입니다.

RustJ는 이 비전을 J를 정확히 구현하는 compiler architecture 위에서 다시 살립니다. J의 압축된 array algebra를 loss 없이 보존하고, 그 위에 `Graph Basis → rewrite/equivalence → symbolic resource analysis → execution/physical planning`을 구축하는 이유도 여기에 있습니다.

따라서 RustJ의 두 목표는 경쟁하지 않습니다.

```text
직접 목표
    full-J semantics를 보존하는 현대적인 J compiler/runtime

장기 비전
    JAXA의 “NN의 SQL”을 가능하게 하는
    declarative array-computing compiler core
```

장기적으로 이 compiler core에 J 외의 array/NN frontend를 붙일 가능성도 열어두지만, 현재 단계에서는 **J 의미론을 정확하게 보존하는 것이 우선**입니다. “NN의 SQL”은 현재 구현 완료를 의미하는 제품 설명이 아니라, Graph IR·optimizer·resource/planner 설계가 향하는 **north star**입니다.

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

**상세 아키텍처, compiler stage 경계, IR, 구현 계획, 체크리스트, 지원 범위와 검증 정책의 유일한 기준 문서는 [PROJECT.ko.md](PROJECT.ko.md)입니다.**

**왜 RustJ가 compiler-oriented architecture를 택하면서도 J semantics를 그대로 보존해야 하는지에 대한 설계 근거와 회귀 판정 기준은 [FOUNDATIONS.ko.md](FOUNDATIONS.ko.md)입니다. frontend·Semantic IR·interpreter/JIT/AOT 경계·rank/CellApply·target architecture를 바꾸기 전 반드시 검토합니다.**

## 설계 연구 기반

RustJ middle-end는 jsource compatibility 외에도 기존 array-language compiler 연구를 근거로 설계합니다.

- **APEX — The APL Parallel Executor**: array morphology, SSA, interprocedural specialization, array property/data-flow 분석.  
  Source: https://gitlab.com/bernecky/apex , https://www.snakeisland.com/ms.pdf
- **Aaron W. Hsu / Co-dfns**: Node Coordinate Matrix, columnar/inverted-table AST, data-parallel/nanopass compiler pass, GPU critical-path/kernel-count/memory-traffic 관점.  
  Paper: https://dl.acm.org/doi/10.1145/2935323.2935331  
  Source pin: https://github.com/Co-dfns/Co-dfns/tree/4e6d3e3002f2109360d24278776c5b5a4f65db0d
- **APL → TAIL → Futhark**: typed/rank-aware array IR, explicit map/reduction nests, loop fusion, nested-parallelism flattening, GPU lowering.  
  Dyalog'16: https://elsman.com/pdf/Dyalog16.pdf  
  FHPC'16: https://elsman.com/pdf/fhpc16futhark.pdf

이 연구를 그대로 복제하지 않습니다. RustJ는 **full J semantics를 먼저 보존**하고 다음 요소만 middle-end에 흡수합니다.

```text
GraphIndex / AnalysisIndex
MorphologyEngine
ArrayPropertyFacts + FactWitness
SpecializationKey/cache
High-level Logical Parallel IR
Pure/effect region partition
ParameterizedLoweringRecipe
```

각 연구 compiler의 static-rank/static-scope/no-execute/pure-subset 제한은 RustJ language restriction이 아니라 **특정 compiler route의 precondition**으로만 취급합니다. 상세 근거와 거부 항목은 [FOUNDATIONS.ko.md](FOUNDATIONS.ko.md) Part XX, 구현 계약과 체크리스트는 [PROJECT.ko.md](PROJECT.ko.md) 4.24.3–4.24.10/A2/A3를 따릅니다.

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

추가 conformance·성능·메모리 검증 원칙은 [PROJECT.ko.md](PROJECT.ko.md)의 검증 절을 따릅니다.

## 문서 정책

사람이 유지하는 설계·계획·진행 Markdown 문서를 임의로 더 늘리지 않습니다. `FOUNDATIONS.ko.md`는 사용자가 명시적으로 요청한 **설계 헌법/근거 문서**로서 예외이며, roadmap/checklist를 중복하지 않습니다.

- 설계 근거와 회귀 판정 기준: `FOUNDATIONS.ko.md`
- 프로젝트 아키텍처·계획·체크리스트의 권위 문서: `PROJECT.ko.md`
- 사용 진입점: `README.ko.md`
- 기여자 라이선스 계약: `CLA.ko.md` (정본) / `CLA.md` (영어 mirror)
- 실측/기계 원자료: `reports/*.json`, `reports/*.jsonl`

과거 개별 Markdown 보고서의 세부 이력은 Git history에서 확인합니다.


## 과거 설계 저장소

과거 `JAXA`, `JAXA-complier`, `japchae`, `jaxa-analyzer`에 흩어져 있던 primitive vocabulary, resource model, materialized-array/Flow–Storage, analyzer 설계는 2026-09-30 기준으로 `PROJECT.ko.md`에 통합했습니다. 앞으로 새 설계 결정은 RustJ의 `PROJECT.ko.md`에 직접 기록합니다.


## 라이선스

RustJ의 공개 오픈소스 배포 경로는 **GNU General Public License version 3 (GPL-3.0-only)** 입니다.

동시에 RustJ는 `jsource`와 같은 방향으로 **별도 상용 라이선스 경로**를 유지합니다. 다만 상용 RustJ 라이선스는 RustJ 저작권자가 실제로 부여할 수 있는 권리의 범위에서만 제공됩니다.

특히 RustJ의 상용 이용·배포가 J SOURCE에서 유래한 코드나 그 밖의 Jsoftware 권리에 의존하는 경우에는, 필요한 범위의 **Jsoftware 상용 J SOURCE 라이선스와 관련 upstream 권리**를 별도로 확보하고 그 계약 조건을 준수해야 합니다. RustJ의 `LICENSE` 자체가 Jsoftware의 상용 권리를 대신 부여하지는 않습니다.

따라서 운영 방향은 다음과 같습니다.

- 필요한 Jsoftware 상용 권리를 확보하지 않은 공개 RustJ 배포: **GPL-3.0-only**
- 필요한 Jsoftware 상용 권리와 RustJ 측 권리가 모두 확보된 경우: **별도 RustJ 상용 라이선스 제공 가능**

정확한 조건은 [LICENSE](LICENSE)를 따르며, GNU GPL v3 전문은 [COPYING](COPYING)에 포함되어 있습니다.

외부 기여자는 RustJ가 GPL 공개 버전과 향후 상용 라이선스 버전을 모두 유지할 수 있도록 [CLA.ko.md](CLA.ko.md)에 동의해야 합니다. 영어판은 [CLA.md](CLA.md)입니다.

Cargo의 SPDX 메타데이터는 공개 오픈소스 선택지를 나타내기 위해 `GPL-3.0-only`로 유지합니다. 조건부 상용 라이선스 경로는 SPDX expression으로 표현하지 않고 `LICENSE`에서 규정합니다.
