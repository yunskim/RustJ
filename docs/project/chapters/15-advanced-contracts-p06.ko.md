[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)

> 문서 분할 안내: 기존 `PROJECT.ko.md`의 내용을 이동한 모듈입니다. 원문의 단일 파일 정본·신규 Markdown 금지 문구는 최신 [Agent 문서 규약](../../../AGENTS.md)보다 우선하지 않습니다.

# 심화 기술 계약 — primitive, Rank, 하드웨어 계획, Flow–Storage — 계속 (6/10)

#### 4.15.14 J error precedence와 internal semantic probes를 구분한다

backend 병렬 실행이 “먼저 발견한 lane의 오류”를 임의로 사용자 오류로 선택하면 안 된다.

current jsource의 atomic dyad 경로는 예를 들어 agreement/rank-shape 검사를 type/domain conversion보다 먼저 수행하며, source-level error class의 우선순위를 의도적으로 보존한다.

```text
ErrorSemantics
  possible_errors
  precedence / validation order where observable
  retryable_internal_conditions
  suppressed semantic-probe errors
  externally observable error
```

를 primitive/derived-op contract가 표현할 수 있어야 한다.

중요한 구분:

- J가 사용자에게 보고하는 domain/rank/length/value/... error
- overflow처럼 retry를 유도하는 internal condition
- empty fill-cell/prototype 평가에서 J가 의도적으로 suppress하는 computational error
- out-of-memory 같은 exigent error

따라서 “오류 순서 보존”은 **jsource가 정의한 observable error precedence와 suppression/retry semantics를 보존한다**는 뜻이다. 내부적으로 발생한 모든 임시 오류를 그대로 노출한다는 뜻이 아니다.

#### 4.15.15 J error는 exceptional control flow가 될 수 있다

explicit definition 안의 J error를 항상 process-level failure 또는 compile diagnostic으로 취급하지 않는다.

current jsource(`cx.c`, `wc.c`)는 `try.`, `catch.`, `catchd.`, `catcht.`, `throw.`를 control-word graph로 만들고, 실행 중 발생한 error/throw에 따라 handler target으로 이동한다.

따라서:

```text
JSemanticOutcome
  Normal(JEntity)
  Raise(JError)
  Throw
```

와 같은 개념을 CFG semantics가 표현할 수 있어야 한다.

장기 Logical/Control IR에서는:

```text
Op
  normal_successor
  exceptional_successor(s) when inside an active handler region

TryRegion
  body
  handlers:
    ordinary-error handler
    debug/error-class-specific handler as required by J semantics
    throw handler
```

형태 또는 동등한 runtime lowering을 허용한다.

중요한 원칙:

- `domain/rank/length/value/...` error는 handler가 없으면 외부로 전파되지만, active `try.` 안에서는 J control flow의 일부가 될 수 있다.
- `throw.`는 일반 return이 아니라 explicit exceptional control effect다.
- optimizer는 potentially-raising op를 catch boundary 밖으로 hoist하거나 handler를 건너 duplicate/eliminate하지 않는다. 그러려면 `SpeculationSemantics`/error proof가 필요하다.
- GPU/external region 안에서 생긴 J-visible error도 surrounding J handler semantics를 보존할 수 있어야 한다. adapter/backend가 typed error outcome을 되돌릴 수 없다면 해당 region offload를 거부하거나 error-free proof가 필요하다.
- retryable internal overflow/prototype probe error는 user-visible exceptional edge와 구분한다.

A3-v0의 pure single-block subset에는 full exceptional CFG를 요구하지 않는다. `try/catch/throw` lowering은 A3-v1의 multi-block CFG와 함께 구현하되, v0 operation contract는 `MayRaise`/error set을 잃지 않아야 한다.

explicit control word만 error handler인 것은 아니다. current jsource의 `u :: v` adverse conjunction은 derived verb 내부에 handler semantics를 만든다.

```text
AdverseDerived(u, v)
  run u
    normal -> return result
    ordinary catchable error -> clear/catch according to J semantics, run/use v
    throw / exit class -> propagate
```

따라서 `ErrorHandlerSemantics`는 function-level `try.`와 expression/derived-verb-level adverse 모두 표현할 수 있어야 한다. `v`가 noun인 경우 fallback value 자체가 결과가 될 수 있다는 점도 보존한다.





### 4.16 CompilationTarget은 backend / architecture / device를 분리한다

기존의 하나짜리 `TargetProfile`은 역할이 너무 넓다. 같은 CUDA/ROCm backend에서도 architecture가 다르면 instruction, register organization, subgroup/wave behavior, scratchpad 기능 등이 달라지고, 같은 architecture를 쓰는 device끼리도 compute-unit 수, memory/cache capacity 등이 다를 수 있다.

현행 모델은 다음을 구분한다.

```text
CompilationTarget
├─ BackendFamily
├─ ArchitectureTarget
├─ DeviceProfile
├─ RuntimeProfile
└─ resolved TargetProfile view
```

의미:

```text
BackendFamily
  어떤 compiler/runtime ecosystem인가?
  CUDA / ROCm / LLVM-CPU / SPIR-V / Metal / ...

ArchitectureTarget
  어떤 ISA/microarchitecture capability에 맞춰 code를 만들 수 있는가?
  architecture family + concrete compiler/ISA target + feature set

DeviceProfile
  그 architecture를 구현한 실제 device가 얼마만큼의 자원을 제공하는가?

RuntimeProfile
  현재 driver/runtime/toolchain에서 실제 사용할 수 있는 기능은 무엇인가?

TargetProfile
  위 profile fragments와 query providers를 resolution한
  compile invocation용 effective view
```

`CostProfile`은 여전히 별도다. 동일 device라도 runtime/clock/workload/calibration에 따라 달라지는 empirical 성능값이기 때문이다.

#### 4.16.1 BackendFamily

backend family는 source primitive identity가 아니라 lowering ecosystem이다.

예:

```text
generic
cpu
gpu
cuda
rocm
spirv
metal
llvm_cpu
```

BackendFamily가 제공할 수 있는 것:

- 공통 lowering interfaces
- ABI/data-layout family
- memory/execution model의 공통 query
- generic kernel/library strategy
- architecture target naming/selection 규칙

`CUDA`, `ROCm` 같은 이름만으로 concrete instruction legality를 전부 판단하지 않는다.

#### 4.16.2 ArchitectureTarget

architecture target은 **code-generation legality와 architecture-specific capability**의 핵심 단위다.

```text
ArchitectureTarget
  vendor
  architecture_family
  compiler_or_isa_target
  feature_set
  execution_hierarchy_rules
  register_model
  memory/scratchpad model
  instruction capabilities
  synchronization/memory-ordering capabilities
  allocation rules
```

예시적인 naming:

```text
nvidia family / sm target
amd family / gfx target
intel GPU family / target
cpu ISA + microarchitecture feature set
```

중요한 원칙:

- architecture version 숫자의 대소만으로 feature inheritance를 가정하지 않는다.
- capability는 명시적인 feature/query로 판정한다.
- family-level 공통 정보와 concrete architecture override를 둘 수 있다.
- architecture-specific instruction lowering은 이 수준에서 등록할 수 있다.

#### 4.16.3 DeviceProfile

DeviceProfile은 architecture identity가 아니라 **concrete capacity/topology**를 제공한다.

```text
DeviceProfile
  architecture_target
  compute-unit / SM / core count
  register capacities
  scratchpad/shared/LDS capacities
  cache capacities/topology
  memory capacity
  supported configurable resource modes
  link/topology facts
  launch/concurrency hard limits that are device-specific
```

같은 ArchitectureTarget을 공유하는 device라도 DeviceProfile은 다를 수 있다.

구분:

```text
ArchitectureTarget
  "어떤 code/instruction이 legal한가?"

DeviceProfile
  "이 legal code를 이 장치에서 어떻게 schedule하는 것이 가능한가?"
```

#### 4.16.4 RuntimeProfile

일부 capability는 silicon만으로 결정되지 않고 driver/runtime/compiler stack의 지원에도 의존한다.

```text
RuntimeProfile
  backend/runtime version
  driver capability
  compiler/toolchain feature availability
  library availability/version
  runtime launch/interop capability
```

ArchitectureTarget이 지원하는 instruction/feature라도 현재 toolchain/runtime route가 노출하지 않으면 해당 lowering candidate는 사용할 수 없다.

#### 4.16.5 resolved TargetProfile

기존 문서의 `TargetProfile`이라는 이름은 삭제하지 않고 **resolved effective target view**라는 뜻으로 좁힌다.

```text
TargetProfile =
  resolve(
    BackendFamily profile,
    ArchitectureTarget profile,
    DeviceProfile,
    RuntimeProfile,
    target query providers
  )
```

TargetProfile은 logical IR에 복사하지 않는다. compile invocation, route partition, schedule/physical planner, external adapter가 질의한다.

##### 4.16.5.1 Compilation 시작 시 active target locale을 확정한다

각 compile invocation은 parser/J Semantic IR의 의미와 독립적으로 **active compiler target context**를 가진다. 단일-target MVP에서는 compile 시작 시 하나의 `CompilationTargetLocale`을 확정한다.

```text
CompilationSession
  semantic_context        // user J locale/binding semantics
  target_context:
    active_target_locale
    CompilationTarget
    DeviceProfile?
    RuntimeProfile?
```

`active_target_locale`은 user J locale과 절대 섞지 않는다. parser와 J Semantic IR은 target을 몰라도 동일한 의미를 만들어야 한다.

예:

```text
active target:
  compiler.device.h100_0

resolution path:
  compiler.device.h100_0
    -> compiler.arch.nvidia.sm90
    -> compiler.family.nvidia
    -> compiler.backend.cuda
    -> compiler.gpu
    -> compiler.generic
```

모든 hardware-lowerable semantic operation은 source가 built-in인지 extension인지와 무관하게 **같은 active target locale/path**에서 capability/lowering을 조회한다.

```text
+/          -> ResolvedOp::Reduce(Add) ----┐
conv2d ext  -> ResolvedOp::Conv2d ---------+-> same TargetLocale lookup
attention   -> ResolvedOp::Attention ------┘
```

따라서 primitive 자체가 architecture 정보를 갖는 것이 아니라:

```text
Resolved Semantic Op
  + active TargetLocale
  + resolved target/device facts
       ↓
Lowering candidates
```

가 된다.

중요한 규칙:

- target locale 선택은 **J parse/semantic validity에 영향을 주지 않는다**.
- target locale은 lowering/capability discovery의 시작점이다.
- built-in J primitive와 extension-derived op는 동일한 lookup protocol을 사용한다.
- locale lookup은 후보를 찾을 뿐 최종 kernel을 고르지 않는다. legality/resource/cost 분석이 후보 중 realization을 선택한다.
- device-specific locale에 binding이 없으면 explicit parent path를 따라 architecture/family/backend/class/generic으로 fallback한다.
- 숫자 architecture 버전으로 암묵적 상속을 추론하지 않는다.
- 단일-target MVP 이후 heterogeneous execution이 필요해지면 `CompilationSession`이 여러 `TargetContext`를 보유하고 RoutePartition이 region별 context를 선택하도록 확장한다. 이 경우에도 각 region의 built-in/extension은 선택된 동일 target locale chain을 사용한다.

##### 4.16.5.2 AOT와 JIT는 동일한 target selection contract를 사용한다

AOT build와 향후 JIT compilation은 서로 다른 target-selection 시스템을 만들지 않는다. 둘 다 동일한 입력 contract를 사용한다.

```text
CompileOptions / JitOptions
  target_selector
  device_selector?
  target_profile?
  runtime_policy?
        ↓
TargetSelector::resolve(...)
        ↓
CompilationTargetLocale
TargetContext
        ↓
LoweringRegistry lookup
```

차이는 **resolution 시점과 available evidence**뿐이다.

- **AOT**: 명시적 CLI/config/default host target을 기준으로 target context를 만든다.
- **JIT**: 동일한 option model을 사용하되 실제 runtime device, driver/runtime capability, currently available backend를 추가 evidence로 사용할 수 있다.
- 사용자가 `--target`/동등 옵션을 명시하면 JIT도 그 constraint를 존중한다.
- `auto`/unspecified일 때만 JIT가 runtime discovery를 이용해 concrete target/device를 선택한다.
- JIT가 선택한 concrete target도 결국 동일한 compiler target locale/path로 normalize된다.
- built-in/extension 구분 없이 모든 operation은 해당 active target locale을 통해 lowering capability를 조회한다.

따라서:

```text
AOT:
  rustj build --target sm90
       ↓
  TargetSelector
       ↓
  compiler.arch.nvidia.sm90

JIT:
  jit(options: target=auto)
       ↓
  runtime discovery = H100
       ↓
  TargetSelector
       ↓
  compiler.device.<H100>
       -> compiler.arch.nvidia.sm90
       -> ...
```

JIT 전용 primitive registry나 JIT 전용 hardware metadata 체계를 만들지 않는다. cache key에는 semantic/IR version과 함께 resolved target identity 및 lowering-relevant runtime capability version을 포함할 수 있다.

#### 4.16.6 Target locale chain: J locale 방식을 compiler lookup에 재사용한다

backend/architecture/device별 lowering과 capability override는 J의 **locale/path resolution 아이디어**를 compiler namespace에 재사용하면 단순하게 구현할 수 있다.

중요하게, **사용자 J locale과 compiler target locale은 namespace를 분리**한다. 의미와 lookup 방식은 재사용하지만 서로 binding을 섞지 않는다.

개념:

```text
User J Namespace
  base
  myapp
  ...

Compiler Target Namespace
  compiler.generic
  compiler.gpu
  compiler.backend.cuda
  compiler.family.nvidia.<family>
  compiler.arch.nvidia.<arch-target>
  compiler.device.<device-id>
```

target이 정해지면 explicit locale path를 구성한다.

예:

```text
TargetLocaleChain
  device-specific
    → concrete architecture
    → architecture family
    → backend family
    → device class (gpu/cpu)
    → generic
```

예시적인 형태:

```text
compiler.device.<exact-device>
  → compiler.arch.nvidia.<sm-target>
  → compiler.family.nvidia.<family>
  → compiler.backend.cuda
  → compiler.gpu
  → compiler.generic
```

또는:

```text
compiler.device.<exact-device>
  → compiler.arch.amd.<gfx-target>
  → compiler.family.amd.<family>
  → compiler.backend.rocm
  → compiler.gpu
  → compiler.generic
```

이 path는 **숫자 버전의 자동 상속이 아니라 명시적으로 구성한 resolution path**다.

#### 4.16.7 Lowering lookup도 locale resolution을 사용한다

각 target locale은 primitive/op에 대한 lowering binding을 가질 수 있다.

```text
compiler.generic
  ElementwiseAdd(Dyad) -> GenericAddLowering

compiler.gpu
  ElementwiseAdd(Dyad) -> GenericGpuElementwiseAdd

compiler.backend.cuda
  ElementwiseAdd(Dyad) -> CudaElementwiseAdd

compiler.arch.nvidia.<arch>
  Matmul(...) -> ArchitectureSpecificMatmul
```

lookup:

```text
resolve_lowering(
    key = ResolvedSemanticOp {
      source = PrimitiveId::Plus,
      valence = Dyad,
      semantic_op = ElementwiseAdd,
      derived_policies = ...
    },
    locale_chain = target.locales
)
```

검색 순서:

```text
most-specific device
  ↓
architecture
  ↓
family
  ↓
backend
  ↓
cpu/gpu class
  ↓
generic
```

가장 구체적인 legal binding이 우선하지만, 후보는 하나로 즉시 확정하지 않아도 된다. locale lookup은 **candidate discovery**를 담당하고, 실제 선택은 legality/resource/cost 평가가 한다.

```text
locale lookup
  → lowering candidates
  → capability/precondition verification
  → ResourceEstimate
  → optional CostEstimate
  → choose schedule/realization
```

device-specific lowering override는 허용하지만 남발하지 않는다. exact device locale은 주로 capacities/cost/profile override에 사용하고, code-generation 차이가 architecture에서 설명 가능하면 architecture locale에 둔다.


---

[← 이전 부분](./15-advanced-contracts-p05.ko.md) | [다음 부분 →](./15-advanced-contracts-p07.ko.md)
