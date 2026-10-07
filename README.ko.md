[English](README.md) | **한국어 — 정본(canonical)**

# RustJ — J 배열 컴파일러

RustJ는 J의 언어·배열 의미론을 보존하면서 CPU와 GPU를 동등한 실행 대상으로 삼는 Rust 기반 배열 컴파일러를 목표로 합니다.

`jsource`를 줄 단위로 번역하는 프로젝트가 아니라, J frontend·semantic analysis·logical/physical planning·backend를 분리한 하나의 compiler system으로 설계합니다.

`Jaxa`는 현재 아키텍처의 별도 컴포넌트명이 아닙니다. 과거 `jaxa-analyzer` 연구 저장소의 아이디어는 RustJ middle-end 설계에 흡수합니다.

## JAXA에서 이어받은 설계 방향 — “배열 연산의 SQL”

JAXA에서는 한때 **“NN의 SQL”**, 또는 **“배열 연산의 SQL”**이라는 표현을 사용했습니다. RustJ에서는 후자를 더 일반적인 설계 비유로 사용합니다.

중요한 것은 SQL 문법을 흉내 내는 것이 아니라 다음 원칙입니다.

> **J source는 execution plan이 아니다.**

J의 rank, cell/frame, derived entity, train/composition, reduce/scan, shape/reindex 의미는 **무엇을 계산하는가**를 높은 수준에서 드러냅니다. RustJ는 이 정보를 가능한 한 오래 보존하고, **어떻게 실행하는가**는 legality·resource·cost를 고려해 compiler가 선택하도록 설계합니다.

```text
J source / semantics
    ↓
J Semantic IR / J Graph IR
    ↓
Logical Array / Execution IR
    ↓
logical rewrite / fusion
    ↓
execution planning
    ↓
CPU / SIMD / multicore / GPU / external route
```

관계형 DB에서 query와 physical plan을 분리하는 것처럼, RustJ에서도 logical computation과 physical realization을 분리합니다. 따라서 fusion, materialization, layout, schedule, device/thread mapping은 J source가 직접 고정하는 사항이 아닙니다.

full J는 name, assignment, effect, error/control semantics가 있으므로 순수 SQL 같은 declarative language라고 볼 수는 없습니다. RustJ의 의미는 **J를 정확하게 보존하는 것**이 우선이고, 그중 분석 가능한 array region에서 “배열 연산의 SQL”과 같은 optimization freedom을 활용합니다.

이 관점에서 RustJ의 장기적인 위치는 단순한 **“GPU를 지원하는 J”**보다 **“J를 high-level array language로 사용하는 heterogeneous array compiler/runtime”**에 가깝습니다. 과거 JAXA에서 만든 analyzer·logical/physical separation 아이디어는 이 구조의 선행 설계로 승계합니다.

## 핵심 배열 모델 — Logical Array와 Physical Array

RustJ는 배열을 **논리 계층과 물리 계층으로 분리**합니다. 이 구분은 핵심 아키텍처 결정입니다.

```text
Logical Array / J noun
  dtype / J-visible type
  shape
  ordered logical atoms
  boxed / sparse 등 J-visible semantics

        ≠

Physical Array / Representation
  buffer/storage
  strides / offset
  layout / tiling / alignment
  memory space
  CPU/GPU placement
  sharding / transfer
```

따라서 하나의 logical value가 여러 physical representation을 가질 수 있고, logical intermediate가 존재해도 반드시 별도 buffer를 가져야 하는 것은 아닙니다. reshape/transpose/reverse 같은 연산도 **논리적 의미와 실제 copy/materialization 여부를 분리**합니다.

RustJ는 J 의미를 먼저 보존한 뒤, downstream planner가 representation·layout·buffer·device를 선택합니다. 상세 결정은 [PROJECT.ko.md](PROJECT.ko.md)의 **“논리 배열과 물리 배열”** 절과 [FOUNDATIONS.ko.md](FOUNDATIONS.ko.md)의 관련 불변조건을 따릅니다.

### JEntity는 공통 배열 타입이 아니라 semantic boundary carrier

jsource와의 반복 대조 결과, RustJ는 noun과 function을 하나의 **semantic RHS universe**에서 다루되 이를 하나의 physical/runtime array type으로 합치지 않습니다.

```text
JEntity
├─ Noun
└─ Function
    └─ POS = Verb | Adverb | Conjunction
```

`JEntity`의 목적은 parser·binding·assignment·semantic operand 경계에서 `Noun | Function`을 손실 없이 전달하는 것입니다. `Value`와 `FunctionEntity` 내부 구조를 합치거나 모든 IR node의 공통 base type으로 쓰지 않습니다.

또한 Verb/Adverb/Conjunction 자체에는 noun-style shape/rank를 부여하지 않습니다. jsource의 common `A`/`AD` allocation header가 function에도 쓰이지만 function의 AN/AR은 semantic array shape/rank가 아닙니다.

gerund도 generic `EntityArray`의 근거로 보지 않습니다. J-visible 출발점은 boxed noun이며, modifier 문맥에서 필요할 때 `GerundView`/`InterpretedEntitySequence`처럼 **operator-specific interpretation view**를 만듭니다. generic `EntityCollectionView`는 둘 이상의 독립적인 J semantics가 같은 shaped-entity algebra를 요구할 때만 추출합니다.

- 공통화: semantic transport/identity 경계
- 분리 유지: noun value, function entity, lookup/reference, physical storage
- 금지: function에 noun shape/rank를 붙이는 일반화
- 보류: generic `EntityArray`/`EntityCollectionView`
- 우선: M2 중에도 중복 carrier를 한 seam씩 줄이는 최소 `JEntity`

```text
J Source
  ↓
Frontend / parser-time J semantics
  ↓
J Semantic Construction IR / FunctionEntity
  ↓
J Graph IR / Graph Analyzer
  ↓
Execution Semantic Lowering
  ↓
Verified Logical Execution IR / Plan
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
- **Remora**: J/APL 계열의 rank polymorphism, frame/cell semantics, implicit lifting을 정형화한 비교 연구.  
  Paper: https://arxiv.org/abs/1907.00509
- **Bohrium**: 기존 NumPy-style array operation을 lazy하게 수집해 fusion, allocation/materialization, host-device data movement와 backend-specific execution을 늦추는 선례. CPU/GPU를 op마다 동적으로 선택하는 모델로 해석하지 않습니다.  
  Publications: https://bohrium.readthedocs.io/publications.html
- **Lift**: portable map/reduce pattern을 rewrite하여 OpenCL-specific functional pattern까지 점진적으로 hardware mapping을 구체화하는 비교 연구. “rewrite와 hardware mapping의 완전한 분리”로 해석하지 않습니다.  
  Paper: https://doi.org/10.1109/CGO.2017.7863730
- **MLIR Linalg**: structured operation과 implicit iteration을 보존한 뒤 tiling/vectorization/lowering에서 loop를 materialize하는 참고 IR.  
  Docs: https://mlir.llvm.org/docs/Tutorials/transform/Ch0/
- **JAX / jaxpr**: explicitly typed, functional, first-order ANF라는 transformation-friendly IR의 비교 기준. J combinator provenance 보존의 선례는 아닙니다.  
  Docs: https://docs.jax.dev/en/latest/601/jaxpr.html
- **XLA HLO Fusion**: fusion computation이 이미 IR에 묶인 committed representation의 비교 기준. RustJ의 `FusionCandidate`는 그보다 앞선 pre-selection analysis object입니다.  
  Docs: https://openxla.org/xla/operation_semantics#fusion

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

이들 연구 compiler에서 **각기 나타나는** static-rank, static-scope, no-execute, pure-subset 등의 제한은 RustJ language restriction이 아니라 **특정 compiler route의 precondition**으로만 취급합니다. 모든 비교 대상이 이 제한을 전부 공유한다는 뜻은 아닙니다. 상세 근거와 거부 항목은 [FOUNDATIONS.ko.md](FOUNDATIONS.ko.md) Part XX, 구현 계약과 체크리스트는 [PROJECT.ko.md](PROJECT.ko.md) 4.24.3–4.24.10/A2/A3를 따릅니다.

## 현재 상태

현재 저장소는 목표 compiler pipeline으로 이동 중인 전환 단계입니다.

- 제한된 J frontend와 CPU 직접 실행 경로
- shared immutable `FunctionEntity` semantic DAG와 hook/fork/train/derived modifier/rank 구조
- 별도 canonical analysis surface인 J Graph IR과 Graph Basis/rewrite/resource-analysis 기초
- **M1 완료:** J Graph lowering이 canonical A3 `logical_ir::Plan`을 직접 생성하며 transition IR/container는 제거됨
- A3-v0의 SSA ValueId, Execution Basis, semantic check/constraint/effect/error contract와 verifier/reference executor
- explicit/direct 정의의 원문·제어·NAME metadata와 mode-3/4 호출, 지원 if/while/for/try 및 중첩 direct/문자열 explicit의 독립 local scope, A3 정의 함수 참조를 지원한다. 현재 범위의 frontend E2E는 검증됐다. statement/control 실패는 본문 위치와 중첩 호출 경로를 진단 frame으로 보존한다. 실행 전/반환 후 오류 경계와 파일 전체 원문 추적, 전체 J 표현력, 본문 Graph/Logical 분석·CFG 컴파일 및 일반 locale/locative는 미완료다. PROJECT.ko.md의 감사 결과와 보완 실행 체크리스트를 따른다.
- 문자열 단일·다중 및 runtime 계산된 문자열 대입 대상을 지원한다. local/global, scalar 확장·item/open, 순서 있는 부분 실패를 보존한다. 다중 대입의 Graph/Logical 변환과 boxed/atomic-representation target은 미지원이다.
- CPU Inline/Owned/Shared storage, runtime AVX2 + portable fallback
- sparse/boxed/packed-bit 기반 일부와 read-only affine PhysicalArray(G1)
- M2 jsource-compatible frontend cutover와 M3 logical/physical value 경계 수렴, M4 native Physical Planner/CPU executor는 미완료
- 실제 CUDA backend는 아직 미구현

현재 우선순위는 **M2 frontend**입니다. word formation/enqueue/9-row parser와 definition/name/assignment semantics를 jsource-compatible하게 수렴시킨 뒤, M3 logical/physical 경계를 마무리하고 M4의 `Verified Logical IR → Schedule/Physical Plan → CPU Physical Executor` vertical slice를 연결합니다.

Logical Array IR 이후 실행 경로는 하나로 고정하지 않습니다. RustJ-native planner/executor 외에도 MLIR/LLVM 계열, StableHLO-compatible subset, SPIR-V/NVVM/ROCDL, 검증된 외부 library/kernel lowering을 사용할 수 있도록 설계합니다. RustJ는 J 의미·legality·lowering 조건을 책임지고, 이미 잘 만들어진 compiler IR과 optimizer를 가능한 범위에서 재사용합니다.

RustJ의 **언어 의미 목표는 장기적으로 J 전체**입니다. 다만 모든 J 기능이 동일한 hardware-aware 최적화 경로에 들어갈 필요는 없습니다. shape/rank/access/effect contract가 충분한 부분은 `Analyzable Array Profile`로 Logical Array IR과 advanced planner/external compiler route를 사용할 수 있고, 나머지는 의미를 보존하는 native/runtime lowering 또는 현재는 명시적 unsupported 상태로 남을 수 있습니다.

확장 primitive는 keyword가 아니라 ordinary J name binding으로 등록합니다. 예를 들어 역사 prototype의 `conv`는 parameterized adverb이고, parameter 적용 뒤에 derived Conv computational verb/op가 만들어지는 구조를 보존합니다.

ordinary name의 실제 품사는 enqueue에서 고정하지 않고 **parser가 그 name을 사용할 때 정상 J name environment에서 lookup하여 결정**합니다. 일반 verb/adverb/conjunction name의 late-binding nameref 의미도 보존합니다. **현재 runtime definition subset은 current `LocalFrame` → global namespace lookup까지 구현되어 있고, 일반 user locale/path lookup은 아직 미완료**입니다.

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
