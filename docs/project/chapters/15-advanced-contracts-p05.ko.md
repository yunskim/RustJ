[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)

> 문서 분할 안내: 기존 `PROJECT.ko.md`의 내용을 이동한 모듈입니다. 원문의 단일 파일 정본·신규 Markdown 금지 문구는 최신 [Agent 문서 규약](../../../AGENTS.md)보다 우선하지 않습니다.

# 심화 기술 계약 — primitive, Rank, 하드웨어 계획, Flow–Storage — 계속 (5/10)

### 4.12 access pattern은 fusion 분석의 semantic lower bound다

`japchae` D-24에서 primitive를 scalar arithmetic까지 지나치게 분해하면 matmul/conv 구조 정보가 사라져 다시 pattern recognition을 해야 한다는 문제가 확인되었다. fusion/array planning 관점의 유용한 lower bound는 **memory/access pattern**이다.

초기 분류:

```text
Map
Reduce
WindowReduce
Scan
StaticReindex
DynamicGather
Scatter
```

`StaticReindex`는 reshape/transpose/flatten/broadcast처럼 index mapping이 compile-time known인 경우다. 기본적으로 materialization이 아니라 view/remap 후보다. 실제 copy 여부는 downstream layout과 target을 본 physical planning에서 결정한다.

### 4.13 semantic reference definition과 implementation을 분리한다

name 기반 custom primitive의 구현은 black box여도 되지만 의미까지 black box여서는 안 된다. 표준 J로 표현 가능한 extension primitive는 가능하면 **기존 J primitive만으로 reference definition**을 제공한다.

reference definition은 execution implementation도 resource model도 아니며 optimization decomposition을 강제하지 않는다. 원래 J 엔진을 semantic oracle로 사용할 수 있게 하는 명세다.

```text
semantic validation
  extension result == reference J result

resource validation
  ResourceEstimate ~= compiled resource report

performance validation
  CostEstimate ~= measured backend behavior
```

reference definition을 실제 lowering으로 사용하는 것은 별도 검증을 통과한 뒤에만 허용한다.

### 4.14 Hardware-aware IR 설계 원칙: 네 층을 분리한다

leading axis, reduction axis, contiguity 같은 정보는 중요하지만 **하드웨어 분석에 필요한 전체 정보의 일부**다.

2026-09-30에 MLIR DataLayout/TargetSystemSpec, LLVM TargetTransformInfo, OpenXLA GPU pipeline, TVM TensorIR scheduling, Triton kernel configuration, NVIDIA CUDA, AMD ROCm/HIP 문서를 다시 검토했다.

공통적으로 확인되는 구조는 다음과 같다.

1. 고수준 연산 의미와 target hardware의 고정 사양을 분리한다.
2. logical computation에는 planner가 병렬화·locality·reduction·layout 가능성을 추론할 수 있는 정보가 있어야 한다.
3. tile size, thread/workgroup mapping, memory-space 배치 같은 값은 target을 본 뒤 schedule/physical 단계에서 결정한다.
4. register 사용량, occupancy, memory traffic 같은 값은 primitive의 고정 상수가 아니라 **graph + schedule + target**의 함수다.
5. hardware의 hard limit와 측정 기반 performance cost를 같은 것으로 취급하지 않는다.

RustJ에서는 이를 네 층으로 나눈다.

```text
A. Hardware-relevant semantic/logical facts
   "이 계산은 어떤 구조인가?"

B. CompilationTarget / resolved TargetProfile
   BackendFamily + ArchitectureTarget + DeviceProfile + RuntimeProfile
   "이 target은 무엇을 제공하고 무엇을 제한하는가?"

C. Physical Schedule / Realization
   "이 계산을 이 target에 어떻게 매핑할 것인가?"

D. Resource / Cost Estimate
   "그 매핑의 자원 사용량과 예상 비용은 얼마인가?"
```

이 네 층을 하나의 `PrimitiveSpec` 또는 하나의 거대한 IR node에 섞지 않는다.

예를 들면 `leading axis`, `reduction axis`, `access relation`, `iteration dependency`는 A에 속한다. architecture instruction/subgroup/register-allocation rule과 exact device의 register/shared/LDS/cache capacity는 B에 속하되, ArchitectureTarget과 DeviceProfile 중 어디의 사실인지 구분한다. `tile=64x128`, `num_warps=4`, `vector_width=8`, memory-space 선택은 C이며, `registers/thread=72`, occupancy, HBM bytes, spill risk는 D다.

### 4.15 Logical Array IR이 가져야 하는 hardware-relevant contract

Logical Array IR은 hardware-independent여야 하지만 **hardware-blind여서는 안 된다.** CUDA의 `threadIdx.x`를 몰라도 planner가 어떤 logical axis를 thread/lane/vector에 배치할 수 있는지는 알아야 한다.

최소 모델:

```text
LogicalOp
├─ IterationDomain
├─ AxisSemantics
├─ AccessRelations
├─ NumericSemantics
├─ DependencyRequirements
├─ Effect / Alias
└─ Storage / LifetimeRequirements
```

#### 4.15.1 IterationDomain

연산이 어떤 index 공간 위에서 정의되는지를 표현한다.

```text
IterationAxis
  id
  extent
  kind:
    Parallel
    Reduction
    Scan
    Window
    SerialDependency
    Broadcast
    StaticReindex
    Gather
    Scatter
```

이것은 GPU thread axis가 아니다. logical iteration axis다. 이후 planner가 CPU thread, SIMD lane, GPU workgroup/subgroup/lane 등에 매핑한다.

#### 4.15.2 AxisSemantics

J rank가 frame/cell을 정하고, primitive contract가 cell 내부 axis role을 정한다. 필요한 정보는 단순한 `leading_axis` 하나보다 넓다.

```text
AxisSemantics
  frame_axes
  cell_axes
  input_axis_roles
  output_axis_roles
  reduction_axes
  preserved_axes
  introduced_axes
  contracted_axes
  window_axes
  broadcast_axes
  scan_axes
  output_axis_mapping
```

conv2d라면 input cell `[C_in,H,W]`, weight `[C_out,C_in,KH,KW]`, output cell `[C_out,OH,OW]`이고 output/parallel axes는 `[C_out,OH,OW]`, reduction axes는 `[C_in,KH,KW]`, window axes는 `[KH,KW]`다.

`leading axis`가 필요하다면 이 구조 안에서 `cell axis 0` 또는 특정 access map의 major/minor logical axis로 표현한다. standalone 특별 규칙으로 두지 않는다.

#### 4.15.3 AccessRelations

성능 분석에는 axis 이름보다 **실제로 각 operand를 어떤 index로 읽고 쓰는가**가 더 중요하다. 따라서 가능한 범위에서 input/output index relation을 표현한다.

```text
AccessRelation
  operand
  mode: Read | Write | ReadWrite
  index_map(iteration_axes, reduction_axes) -> operand_indices
  regularity:
    Affine
    StaticPermutation
    Windowed
    Indirect
    DataDependent
  reuse_axes
  bounds / masking requirement
```

conv2d의 개념적 관계:

```text
Y[oc, oh, ow]
X[ic, oh*stride_h + kh - pad_h, ow*stride_w + kw - pad_w]
W[oc, ic, kh, kw]
```

이 정보로 planner는 tile 내부 reuse, reduction/output parallelism, static reindex, gather/scatter 구조를 판단할 수 있다. 실제 contiguous/vectorized access, memory transaction, alignment legality는 이후의 representation facts와 target mapping을 함께 봐야 한다. 따라서 향후 `leading_axis`보다 더 일반적인 핵심 표현은 **axis-role + semantic access relation**이다.

#### 4.15.4 NumericSemantics

하드웨어 mapping을 결정하려면 dtype만으로 부족하다.

```text
NumericSemantics
  input dtypes
  result dtype
  accumulator requirement
  widening / narrowing rules
  overflow retry / promotion behavior
  comparison tolerance mode / context
  fit (!.) numeric policy when applicable
  reduction algorithm / accuracy mode when specified
  reassociation allowed?
  FMA contraction allowed?
  reduction order/rounding constraints when specified
  NaN / signed-zero constraints
```

CPU vector reduction이나 GPU tree reduction을 무조건 금지하지 않는다. current jsource 자체도 일반 floating reduction에 SIMD/여러 accumulator를 사용할 수 있다. 대신 각 primitive/derived verb가 요구하는 **numeric accuracy/order contract**를 표현하고, 그 contract가 허용하는 범위에서만 reassociation/vector/tree reduction을 선택한다.

#### 4.15.4a comparison tolerance와 `!.`는 semantic input이다

J의 비교는 단순 IEEE bitwise comparison으로 고정되지 않는다. current jsource는 runtime comparison tolerance(`jt->cct`)를 사용하고, `u!.ct` derived verb는 자체 tolerance를 보존할 수 있다. `!.`는 일부 verb에서는 fill을 바꾸는 의미도 가진다.

따라서 compiled/GPU lowering은:

```text
ComparisonSemantics
  Exact
  DefaultTolerance(context/versioned runtime state)
  ExplicitTolerance(value)

FitSemantics
  numeric_tolerance_override
  fill_override
  primitive-specific fit behavior
```

를 잃지 않는다.

tolerance가 runtime mutable state에 의존하면 다음 중 하나가 필요하다.

- compile-time captured value + binding/runtime guard
- explicit semantic input
- runtime semantic lowering

GPU kernel 안에서 임의의 exact comparison으로 바꾸면 안 된다.

또한 current jsource는 일반 `+/`와 `+/!.0`를 같은 reduction algorithm으로 취급하지 않는다. `+/!.0`에는 compensated summation 경로가 있으므로, `!.`를 단순 optimizer hint로 버리면 안 된다.

```text
ReductionNumericPolicy
  DefaultJReduction
  Compensated
  ExactOrExtended
  PrimitiveSpecific(...)
```

처럼 derived verb의 numeric policy가 lowering까지 전달되어야 한다.

#### 4.15.4b overflow/promotion은 primitive contract의 retry semantics다

current jsource의 atomic arithmetic은 retryable overflow를 별도 error class로 반환하고, 경우에 따라 wider result path로 전체 연산을 retry하거나 이미 계산한 결과를 일관된 widened representation으로 repair한다.

따라서 “integer op이면 원소마다 overflow한 lane만 float로 바꾼다” 같은 realization은 허용하지 않는다.

```text
OverflowSemantics
  primitive/type combination
  retryable?
  promoted result type/path
  coherent-result requirement
  reduction/prefix-specific behavior
```

를 contract로 둔다.

“전체 배열 overflow promotion”이라는 기존 문구는 모든 primitive에 동일한 promotion 규칙이 있다는 뜻이 아니다. **해당 J primitive/type 조합이 정의하는 retry/promotion을 결과 array 전체의 일관된 J type semantics로 보존한다**는 뜻이다.


#### 4.15.5 Logical dependency requirement

logical op은 CUDA barrier 자체를 갖지 않는다. 대신 어떤 범위의 dependency가 필요한지 표현한다.

```text
DependencyRequirement
  Independent
  Reduction(axis_set)
  OrderedScan(axis_set)
  WindowDependency(axis_set)
  ConflictingUpdate(resource_or_value, combine_semantics)
  CrossValueOrdering(effect_or_resource)
  Collective(logical_participants)
```

여기서 `Subgroup`, `Warp`, `Workgroup` 같은 execution scope는 Logical IR에 넣지 않는다. 이들은 Physical Schedule이 logical participants를 target execution hierarchy에 매핑한 뒤 생긴다.

Physical Planner 또는 외부 compiler가 target의 barrier/shuffle/atomic/multi-stage reduction/collective capability를 보고 구체적으로 실현한다. `ConflictingUpdate`가 있다고 해서 atomic instruction 사용을 미리 결정하지 않는다.

#### 4.15.6 Logical invariance와 derived target uniformity를 분리한다

`uniform/divergent`는 보통 SPMD execution mapping이 정해진 뒤 의미가 생긴다. 따라서 target-independent Logical IR의 기본 fact 이름은 `InvarianceFact`로 둔다.

```text
InvarianceFact
  InvariantOver(logical_axis_set)
  KnownVariantAlong(logical_axis_set)
  Unknown
```

적용 대상:

- branch condition
- indirect index
- logical address/index expression
- mask
- collective operand

Physical Schedule이 logical axes를 SIMD lane/subgroup/thread에 매핑한 뒤:

```text
InvarianceFact + AxisMapping
  → TargetUniformity / DivergenceFact
```

를 파생한다.

즉 GPU subgroup uniformity를 Logical IR의 고정 의미로 저장하지 않는다. 이는 CPU vector lane invariance에도 같은 logical fact를 재사용할 수 있게 한다.

#### 4.15.7 Symbolic shape / divisibility constraints

dynamic extent는 단순 `Unknown`으로 버리지 않고 가능한 제약을 보존한다.

```text
ConstraintSet
  Equal(a, b)
  UpperBound(x, n)
  LowerBound(x, n)
  MultipleOf(x, n)
  DivisibleBy(x, n)
  PowerOfTwo(x)
  NonZero(x)
```

이 정보는 tile size, unrolling, matrix/tensor shape legality와 runtime specialization을 결정할 때 사용한다. 주소 alignment나 physical contiguity는 logical shape constraint와 분리한다.

제약이 compile-time에 증명되지 않더라도 runtime guard로 specialization할 수 있다.

```text
if N % 16 == 0
  → vectorized/tensor path
else
  → conservative path
```

#### 4.15.8 Representation facts는 Logical semantics와 분리한다

stride, byte alignment, concrete contiguity, address space는 J logical value의 의미가 아니다.

```text
RepresentationFacts
  storage_extents_if_known
  strides
  base_offset
  byte_alignment
  contiguous_dimensions
  address_space / memory_space
  external_abi_layout
```

`RepresentationFacts`는 canonical Logical IR의 semantic identity가 아니라 downstream planning/adapter가 Logical value에 대해 알고 있는 **representation-side analysis state**다.

이 facts는 다음 출처에서만 생긴다.

- 함수/FFI/external buffer ABI contract
- 이미 존재하는 PhysicalArray/view
- bufferization/physical planner의 결정
- runtime guard로 검증한 representation assumption

따라서 `AccessRelation`이나 semantic `ConstraintSet` 안에 stride/alignment를 섞지 않는다.

```text
LogicalOp + AccessRelation
          +
RepresentationFacts
          +
TargetProfile
       → physical access legality/cost
```

representation assumption에 의존하는 fast path는 witness/guard 또는 adapter precondition으로 명시한다. MLIR에서도 byte alignment는 tensor semantic이 아니라 memref/alloc/load/store 수준의 representation property로 다뤄진다.


#### 4.15.9 Semantic mask와 schedule predication을 분리한다

Logical IR에는 **연산 의미 자체가 조건부 access/value를 요구할 때만** semantic mask를 둔다.

```text
SemanticMaskSemantics
  predicate source
  affected logical accesses/results
  false-branch value semantics
  bounds relation
  side-effect suppression rule
```

예:

- data-dependent select/filter/gather safety
- window/padding 의미 때문에 논리적으로 out-of-domain access를 구분해야 하는 연산
- source semantics에 포함된 conditional effect

반대로 vector tail, partial tile, workgroup 경계 때문에 생기는 mask는 Logical IR의 의미가 아니다.

```text
Logical shape/access facts
        ↓
Schedule / vectorization / tiling
        ↓
PredicationPlan
  tail mask
  masked load/store
  boundary predicate
```

따라서 CPU AVX mask나 GPU lane predicate는 downstream schedule/backend 결정이다. 같은 Logical IR이 target에 따라 masked instruction, scalar remainder loop, padded tile 중 다른 realization을 선택할 수 있다.

#### 4.15.10 SSA, region/block, control flow

현재 array dataflow만으로는 향후 direct/explicit definition의 조건분기·반복·호출을 충분히 표현할 수 없다.

Logical Array IR은 장기적으로 최소한 다음 구조를 허용한다.

```text
Function
  Region
    Block(args...)
      Op...
      Terminator(successors / return)
```

value는 SSA `ValueId`로 표현한다. pure array graph는 single-block graph region으로 표현할 수 있고, control flow가 필요한 경우 CFG region/block을 사용한다.

J의 hook/fork/train을 이 CFG로 일찍 풀라는 뜻은 아니다. 그것들은 J Semantic Array IR에서 보존한 뒤 semantic lowering 결과로 필요한 control/dataflow만 만든다.

#### 4.15.11 Constraint witness / runtime guard

`ConstraintSet`은 metadata 목록만으로 끝내지 않는다. 어떤 최적화가 특정 runtime assumption에 의존하는지 추적할 수 있어야 한다.

개념 모델:

```text
WitnessId
Assert(constraint) -> WitnessId
Assume(witness) {
  optimized region
}
Guard(constraint,
      fast_region,
      fallback_region)
```

compile-time에 증명된 constraint는 witness 없이 fact로 정착할 수 있다. 동적 constraint는 guard를 통해 specialization 경로와 fallback 경로를 동시에 보존한다.

이는 MLIR Shape dialect의 witness/assuming 아이디어와 같은 목적을 가진다.

#### 4.15.12 effect resource와 ordering token

SSA data dependency만으로는 I/O, mutable state, explicit storage update, runtime call의 관찰 가능한 순서를 모두 표현할 수 없다.

Logical IR은 effect를 두 수준으로 표현한다.

```text
EffectSummary
  semantic_or_external_resource
  Read | Write | CreateResource | DestroyResource | IO | External | Unknown
  stage/order constraints

EffectToken
  explicit ordering edge when data dependency alone is insufficient
```

모든 pure op에 token을 붙이지 않는다. observable side effect나 external call처럼 순서가 의미에 포함되는 경우에만 explicit token/effect edge를 사용한다. 여기의 Create/Destroy는 **언어 또는 external-resource semantics에 실제로 존재하는 resource**에 한정하며, downstream buffer allocation/free를 뜻하지 않는다.

StableHLO의 side-effecting op token과 MLIR MemoryEffect/Speculation interface를 참고하되, J의 error ordering까지 포함할 수 있도록 `SpeculationSemantics`를 별도로 둔다.

#### 4.15.13 speculation / may-error semantics

J에서는 domain/rank/length/overflow 등의 오류 발생 순서도 관찰 가능할 수 있다. 따라서 “memory effect가 없다”와 “마음대로 speculative execution 가능”은 다르다.

```text
SpeculationSemantics
  AlwaysSafe
  SafeIf(constraints)
  MayRaiseObservableError(error_set)
  MayThrow
  MayNotTerminate
  HasNonLocalControlEffect
```

optimizer와 external adapter는 이 contract를 보고 hoist, duplicate, eliminate, reassociate 가능성을 판단한다.


---

[← 이전 부분](./15-advanced-contracts-p04.ko.md) | [다음 부분 →](./15-advanced-contracts-p06.ko.md)
