[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)

<a id="입력-정보-문제의-프레임워크-비교-2026-10-04"></a>
##### 입력 정보 문제의 프레임워크 비교 (2026-10-04)

| 프레임워크 | 실행 전 입력 정보를 얻는 방법 | shape 변화/재사용 처리 | RustJ에 참고할 부분과 한계 |
|---|---|---|---|
| JAX | `ShapeDtypeStruct`로 실제 원소 없이 trace/lower/compile 가능. static argument는 실제 값 필요 | AOT artifact는 signature에 specialize되어 불일치 입력을 거부. shape polymorphism은 별도 export 경로 | 입력 metadata와 실제 noun을 분리. 값 상수 specialization과 배열 입력을 구별. [AOT](https://docs.jax.dev/en/latest/aot.html), [shape polymorphism](https://docs.jax.dev/en/latest/export/shape_poly.html) |
| PyTorch | `torch.compile`은 입력 metadata와 guards를 사용. export는 실제 원소가 없는 FakeTensor/Proxy로 연산을 기록 | compile의 guard가 실패하면 재capture/recompile 가능; dynamic shape는 기호 크기와 제약을 사용. export는 별도 명시 제약 | 관측 facts와 검증 조건을 분리. default static→dynamic generalization을 선택적으로 참고하되 J NAME witness를 유지. [export model](https://docs.pytorch.org/docs/main/user_guide/torch_compiler/export/programming_model.html), [dynamic shapes](https://docs.pytorch.org/docs/main/user_guide/torch_compiler/torch.compiler_dynamic_shapes.html), [guards/recompile](https://github.com/pytorch/pytorch/blob/main/docs/source/user_guide/torch_compiler/torch.compiler_dynamo_overview.md) |
| TensorFlow | `TensorSpec`/`input_signature`; `get_concrete_function(TensorSpec(...))`로 tensor graph 실행과 tracing 분리 가능 | `None` 차원은 wildcard로 trace 재사용. 고정 signature와 맞지 않는 입력은 거부 | 일부 크기만 미확정으로 둘 수 있음. wildcard 자체는 서로 다른 입력의 차원 동등성을 표현하지 않음. [tf.function](https://www.tensorflow.org/guide/function) |
| TVM Relax | graph input에 shape/dtype type를 선언하고 symbolic dimension `n`을 연산·함수 사이에 연결 | 기호 관계를 IR에서 추론; 모르는 shape는 runtime shape 표현으로 남길 수 있음 | 단순 unknown과 `X.batch = Y.batch` 관계를 구별하는 후속 abstract domain. [Relax abstraction](https://tvm.apache.org/docs/deep_dive/relax/learning.html), [shape API](https://tvm.apache.org/docs/reference/api/python/relax/relax.html) |

tracing은 frontend/host 코드를 실행할 수 있으므로 “사용자 코드를 전혀 실행하지 않는다”와 같지 않다. descriptor 방식도 데이터 의존 결과 크기·조건·오류를 자동으로 증명하지 않는다. RustJ는 frontend에서 보존한 graph와 known facts를 분석하고, unknown facts 및 J effect/name timing을 유지한다. shape/type 정보가 없더라도 topology 수준 기회를 찾을 수 있으며, 확보한 정보에 따라 legality·logical resource 분석을 정밀하게 한다. 실제 optimization pass는 계속 보류한다.

**WI1 gate:** native Windows default/portable 각각 **396 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 세 runtime 경로 **4,942 cases / 4,938 passed / 기존 runtime 경계 4 / failed 0**, stage **10,163 checks / failed 0**, words **6,618 cases / failed 0**. capture graph 경계 164건 및 static 경계 2건은 별도다. report 10개의 실제 binary/source hash를 다시 확인했다. `with` 실행·기호 차원·학습/AD·CUDA·optimizer 실행 완료를 뜻하지 않는다. full upstream/private C trace 및 ignored definition acceptance는 미검증이며 Linux/GitHub CI는 실행하지 않았다. 이후 ordinary-reference 단계에서 NAME 제한을 해소했으며 implicit locative와 전체 operator scope는 후속 경계이며 직선 호출은 아래 operator-call 단계에서 지원한다. `with` adapter는 필수 선행 조건으로 만들지 않는다.

<a id="array-compiler-static-analysis"></a>

<a id="483-배열-컴파일러의-정적-분석-대상과-프레임워크-비교-2026-10-04"></a>
### 4.8.3 배열 컴파일러의 정적 분석 대상과 프레임워크 비교 (2026-10-04)

**분석 대상은 입력 배열뿐 아니라 프로그램의 중간 값·연산·사용 관계·제어 흐름·저장소다.** §4.8.2의 IN0–IN11은 분석의 초기 정보이며 아래 SA0–SA8은 그 정보와 보존한 graph/contract에서 도출할 질문이다. 정적 분석은 실행 전 facts·제약·증명·미해결 조건을 산출한다. fusion, DCE, tiling, buffer 배치는 그 결과를 사용하는 변환이며 분석과 구별한다. XLA도 analysis와 HLO 변환 pass를 구분한다. [XLA analysis passes](https://openxla.org/xla/hlo_passes)

| ID / 분석 대상 | 정적으로 확인할 질문과 산출물 | 최적화에 쓰일 근거 / 미확정 경계 |
|---|---|---|
| SA0 — type/rank/shape 관계 | 각 중간 값의 dtype, rank, extent, 차원 동등성·범위, empty 가능성 | 연산 적합성·크기식·specialization의 전제. 기호 크기도 정적 정보지만 제약을 풀지 못하면 unknown을 유지. [JAX symbolic shapes](https://docs.jax.dev/en/latest/export/shape_poly.html) |
| SA1 — 상수·값의 추상 성질 | 작은 compile-time operand와 그 의존 closure; 증명된 값 범위·조건 | 상수 계산·분기 단순화의 전제. 큰 학습 배열의 내용을 읽는 분석으로 바꾸지 않는다. sorted/unique/finite는 선언만으로 증명되지 않음. [TVM compile-time analysis](https://tvm.apache.org/docs/reference/api/python/relax/analysis.html#tvm.relax.analysis.computable_at_compile_time) |
| SA2 — use-def / fan-out / 결과 사용 | 값의 생산자·사용자, 공통 입력, 미사용 결과, 마지막 사용 | 공유·fusion·제거 후보와 중간 배열 보존 필요성. 미사용 결과도 effect/error가 있으면 제거를 허용하지 않음. [XLA dataflow](https://openxla.org/xla/hlo_passes) |
| SA3 — 반복·인덱스·접근 의존성 | iteration domain, 입력/출력 index map, parallel/reduction 축, read/write 충돌 | tiling·vectorization·병렬화의 legality. 간접 gather/scatter와 비affine 접근은 추가 정보가 필요. [MLIR Linalg](https://mlir.llvm.org/docs/Dialects/Linalg/), [Affine](https://mlir.llvm.org/docs/Dialects/Affine/) |
| SA4 — 제어 흐름·호출·효과·오류 | branch/region/call 의존성, 상태 read/write, 실행 순서와 speculation 안전성 | 이동·제거·병렬 실행의 전제. effect-free와 안전한 선행 실행은 별개이며 J 오류를 UB로 바꾸지 않는다. [MLIR effects/speculation](https://mlir.llvm.org/docs/Rationale/SideEffectsAndSpeculation/) |
| SA5 — alias / mutation / ownership | 같은 저장소를 가리키는 값, overwrite 후 old-value read, escape와 외부 소유권 조건 | in-place/reuse/copy 결정의 전제. 내부 no-later-read는 외부 exclusive ownership 증명이 아님. [MLIR Bufferization](https://mlir.llvm.org/docs/Bufferization/) |
| SA6 — 수명·materialization·메모리 | live range, 보관해야 할 중간 값, 논리 크기식, 선택한 순서에서의 live bytes | materialization 제거·재계산·저장소 재사용의 근거. graph 순서의 추정과 schedule/layout 확정 후 물리 peak를 구별. [XLA scheduling/buffers](https://openxla.org/xla/hlo_to_thunks) |
| SA7 — 표현·layout·target 적합성 | 논리 reshape/index 관계, 지원 dtype/representation 및 접근 모델; 이후 stride/device/target 제약 | view/copy와 kernel route의 전제. 논리 reshape 증명만으로 외부 버퍼의 zero-copy를 보장하지 않음. [TVM reshape analysis](https://tvm.apache.org/docs/reference/api/python/relax/analysis.html#tvm.relax.analysis.has_reshape_pattern) |
| SA8 — 자원·코스트·IR 일관성 | 연산량·전송/저장 크기식, 정의/사용·type·region 불변식 검증 | 후보 비교와 잘못된 IR 검출. 비용식은 실측 시간 또는 정확한 물리 메모리가 아님. [XLA cost/verifier](https://openxla.org/xla/hlo_passes) |

<a id="같은-질문을-각-프레임워크가-다루는-방식"></a>
#### 같은 질문을 각 프레임워크가 다루는 방식

서로 다른 계층을 비교한다. JAX/PyTorch/TensorFlow는 graph 확보와 입력 제약을 포함하는 frontend이고 XLA는 backend, MLIR은 IR 기반시설이다. 아래는 우열·전체 기능표가 아니라 확인한 분석 방법과 RustJ가 참고할 지점이다. tracing으로 graph를 얻은 뒤의 정적 분석과, RustJ가 소스에서 보존한 graph를 분석하는 방법도 구별한다.

| 프레임워크 / 계층 | 확인한 분석 표현·방법 | 한계 / RustJ에 적용할 부분 |
|---|---|---|
| JAX / frontend | symbolic dimension 식·제약으로 shape 관계를 다룸 | 모든 기호 비교를 판정하지 못함. 관계와 미해결 조건을 보존하는 방향을 참고. [Shape polymorphism](https://docs.jax.dev/en/latest/export/shape_poly.html) |
| OpenXLA / backend | HLO value/use, must-alias, cost, verifier; schedule 이후 buffer slice 배치 | 수명은 순서에 의존. 의미 graph·분석·physical 결정을 분리. [Analyses](https://openxla.org/xla/hlo_passes), [buffer assignment](https://openxla.org/xla/hlo_to_thunks) |
| PyTorch / capture frontend | symbolic sizes와 ShapeEnv/guards로 관측 graph의 shape 전제를 관리 | tracing specialization의 guard와 데이터 의존 제어를 구별. RustJ는 J NAME witness와 source semantics를 별도로 유지. [Dynamic shapes](https://docs.pytorch.org/docs/main/user_guide/torch_compiler/torch.compiler_dynamic_shapes.html) |
| TensorFlow / graph frontend | TensorSpec/input_signature와 graph 함수로 dtype/shape 전제를 표현 | wildcard 차원과 입력 간 동등성은 별개. 입력 계약과 graph 분석을 연결하되 tracing을 필수로 하지 않음. [tf.function](https://www.tensorflow.org/guide/function) |
| TVM Relax / graph + kernel IR | use-def, compile-time 의존성, reshape index 증명, memory-planning 전후 할당량 추정 API | estimate_memory_usage는 제어 흐름·함수 간 호출을 고려하지 않는 할당량 합계로 과대 추정 가능. 정확한 peak로 인용하지 않음. [Analysis API](https://tvm.apache.org/docs/reference/api/python/relax/analysis.html) |
| MLIR / structured IR | Linalg index map/iterator, Affine dependence, effect/speculation interface, bufferization | affine 모델 밖의 접근과 외부 alias에 별도 근거 필요. J rank/cell/assembly 의미를 유지한 뒤 이러한 관계를 도출. [Linalg](https://mlir.llvm.org/docs/Dialects/Linalg/), [Affine](https://mlir.llvm.org/docs/Dialects/Affine/), [effects](https://mlir.llvm.org/docs/Rationale/SideEffectsAndSpeculation/), [bufferization](https://mlir.llvm.org/docs/Bufferization/) |
| Futhark / 배열 언어·컴파일러 | size가 있는 배열 type; scan/scatter fusion 사례에서 producer/consumer와 control dependency를 함께 다룸 | regular-array 제한은 RustJ 언어 제한으로 채택하지 않음. 사용 관계는 분석 가능해도 filter 결과 크기는 데이터에 의존. [Language reference](https://futhark.readthedocs.io/en/latest/language-reference.html), [scan-scatter 사례](https://futhark-lang.org/blog/2026-03-24-scan-scatter-fusion.html) |

**J syntax에서 얻는 정보의 범위:** `(loss_adjoint [ (loss emit))`의 fork는 같은 인자로부터 두 branch를 만들고 결과 선택/결합 관계를 드러낸다. 이는 SA2/SA4의 분석 입력이며 `adjoint`라는 이름 자체가 parallel 힌트는 아니다. fork만으로 무조건 병렬 실행을 허용하지 않는다. 각 branch의 효과·오류·name lookup·공유 상태 의존성을 확인해야 한다. modifier/train 구조는 SA0/SA3에 rank/cell/반복 관계를 제공할 수 있으나, 구체 인덱스 map과 uniform assembly는 별도 증명한다. 이 해석은 RustJ의 설계 결정이며 위 프레임워크의 J 지원을 주장하지 않는다.

**현재 구현과 후속 경계:** `static_analysis.rs`의 schema 기반 분석, `j_graph_ir.rs`의 보존 graph와 concrete/unknown GraphFacts, `j_graph_memory.rs`의 use/liveness·논리 extent·명시 가정하 graph-order 메모리 추정, `j_graph_resource.rs`의 자원식이 초기 기반이다. canonical Logical IR의 checks/effects/witness/verifier는 후속 분석 계약의 기반이다. 임의 축별 symbolic solver, 일반 loop/index dependence, 완전한 alias/escape/외부 ownership 증명, target별 schedule/물리 peak 분석은 완료되지 않았다. SA 표는 구현 완료 목록이 아니다. optimizer 실행·CUDA는 계속 보류하고 tokenizer/enqueuer/parser의 J 호환성과 graph 보존 작업을 우선한다.

후속 report는 각 fact에 `source node/span + assumptions + proof scope + known/symbolic/unknown + residual obligation`을 연결하고, 메모리는 `logical extent / assumed-order live bytes / physical allocation estimate`를 구분한다. 함수 identity·J prefix agreement·empty/fill/boxed/sparse·numeric policy·관측 가능한 오류/효과를 증명 없이 단순 tensor 규칙으로 치환하지 않는다.

- [x] **WI0d** 배열 컴파일러의 분석 대상 SA0–SA8, 프레임워크별 방법/한계 및 현재 구현 경계를 출처와 함께 정리한다.
- [ ] **WI3c** frontend graph/facts report에 SA 분류·증명 범위·미해결 조건을 연결하고 현재 지원 부분부터 regression을 추가한다. 일반 solver나 physical scheduler를 frontend 완성의 선행 조건으로 만들지 않는다.

<a id="49-vocabulary는-이름-목록이-아니라-form--contract다"></a>
### 4.9 Vocabulary는 이름 목록이 아니라 form + contract다

과거 연구의 중요한 결론은 primitive vocabulary가 단순 whitelist가 아니라는 점이다.

Structural vocabulary에는 primitive verb/adverb/conjunction, hook/fork/train, `@:`, `@`, `&`, rank `"`, insert/reduce `/`, scan `\` 및 기타 derived-verb builder가 포함된다.

Computational vocabulary에는 표준 J primitive와 name 기반 extension primitive(`conv`, `linear`, `relu` 등)가 포함된다.

Structural form에는 필요하면 operand/result 품사와 semantic constraint를 둔다. RustJ 자체는 장기적으로 J 전체 의미를 구현하므로 다음을 구분한다.

```text
valid J semantics
    ≠
advanced optimization contract available
```

J built-in이 valid J semantics를 가진다고 해서 반드시 hardware-aware Array Logical path에 들어갈 수 있는 것은 아니다.

현행 RustJ는 두 범위를 구분한다.

```text
RustJ language semantic coverage
  장기적으로 J 전체 의미

Analyzable Array Profile
  shape/rank/effect/access contract가 충분하여
  hardware-aware Logical Array IR과 advanced route에 안전하게 낮출 수 있는 영역
```

따라서 J built-in의 advanced resource/lowering contract가 부족하면 다음 중 하나다.

- semantic LogicalOp까지는 만들되 advanced optimization을 막고 conservative/native lowering 사용
- compiled runtime/effect call 경로로 낮춤
- 아직 구현되지 않은 feature라 명시적 Unsupported

어느 경우든 “유효한 J가 아니다”로 오해하지 않는다.

반면 RustJ 고유 extension primitive/adverb는 언어에 새 의미를 추가하는 것이므로 최소 semantic contract와 적어도 하나의 검증된 lowering/runtime 경로가 없으면 **등록 완료로 보지 않는다.**

Semantic Analyzer/validator는 syntax validity와 compilation eligibility를 구분한다.

```text
SemanticDisposition
  LowerToArrayLogical
  LowerToRuntimeSemantic
  KeepLateBound / RequiresGuard
  UnsupportedImplementation

JDiagnostic
  SyntaxError
  DomainError
  RankError
  LengthError
  ValueError
  ...
```

`UnsupportedImplementation`은 “J에서 잘못된 프로그램”이 아니라 **현재 RustJ 구현/route가 아직 실행하지 못한다**는 뜻이다.

**용어 주의:** `UnsupportedImplementation`은 이 문서의 **architecture-level 분류명**이다. 현재 Rust 코드의 concrete error variant는 `Error::Unsupported(String)`이고 `Error::kind()`는 `"unsupported"`를 반환한다. 새 enum variant가 이미 존재한다고 가정하지 않는다. 문서·테스트에서 concrete API를 말할 때는 `Error::Unsupported`/`"unsupported"`를 사용하고, J semantic error와 implementation-coverage miss를 구분하는 개념 설명에서만 `UnsupportedImplementation`을 사용한다.

extension registration 자체가 불완전한 경우에는 별도 registry/configuration diagnostic으로 본다.

```text
MissingSemanticContract
MissingLowering
InvalidExtensionRegistration
```

이 구분은 후기 `jaxa-analyzer`의 Validator 오류 분류를 RustJ의 full-J 목표에 맞게 일반화한 것이다.

<a id="491-analyzable-profile은-허용-목록테스트로-고정한다"></a>
#### 4.9.1 analyzable profile은 허용 목록/테스트로 고정한다

`Analyzable Array Profile`을 설명 문구로만 두지 않는다. 구현에서는 source/derived form별 compilation disposition을 **명시적 coverage manifest + golden test**로 고정한다.

개념적으로:

```text
CompilationCoverage
  semantic_form
  supported_valence/form constraints
  disposition:
    LowerToArrayLogical
    LowerToRuntimeSemantic
    KeepLateBound / RequiresGuard
    UnsupportedImplementation
  required_known_facts
  supported_routes
  reference/golden tests
```

v0 예:

```text
dyadic +
  → LowerToArrayLogical
  required: shape/type/agreement
  access: known elementwise map
  route: native-cpu

+/ on supported dense numeric cell
  → LowerToArrayLogical
  required: reduction/rank/numeric policy
  route: native-cpu

unknown dynamic named verb
  → KeepLateBound / RuntimeSemantic

system foreign / unsupported boxed-sparse form
  → RuntimeSemantic or UnsupportedImplementation
```

새 기능을 추가할 때 “advanced route인가?”를 문서 토론으로 매번 다시 결정하지 않고 이 manifest와 test를 갱신한다.

중요하게 이 목록은 **J 언어 validity 목록이 아니라 현재 compiler implementation/route eligibility 목록**이다.


<a id="410-primitivespec은-semantic-record이고-realization은-별도-registryinterface다"></a>
### 4.10 PrimitiveSpec은 semantic record이고, realization은 별도 registry/interface다

과거 prototype처럼 primitive 하나에 rank, shape, layout, register, tiling, backend implementation을 모두 넣지 않는다.

현행 모델은 **semantic record + capability interfaces + lowering/realization registry**로 나눈다.

```text
PrimitiveSpec
├─ identity / part of speech / supported valences
├─ innate ranks
│   ├─ monad: RankSpec
│   ├─ dyad-left: RankSpec
│   └─ dyad-right: RankSpec
├─ parameter schema
├─ semantic reference definition (optional)
└─ version / provenance

Primitive semantic interfaces
├─ ShapeInference
├─ TypeSemantics / TypePromotion
├─ AxisAndIterationSemantics
├─ AccessPattern
├─ NumericSemantics
├─ FillAndEmptySemantics
├─ ErrorSemantics
├─ EffectSemantics
├─ AliasSemantics
├─ SpeculationSemantics
└─ RewriteLegality

Lowering / realization interfaces
├─ RustJNativeLowering
├─ MlirLowering
├─ StableHloLowering
├─ LibraryLowering
└─ TargetSpecificLowering
```

이 방식은 MLIR의 operation interface 원칙과 유사하다. 분석기와 변환기는 concrete op 이름을 일일이 special-case하기보다 필요한 capability interface를 질의한다.

parameterized extension은 **surface builder와 derived computational entity를 분리**한다.

예를 들어 source의 `conv` binding은 adverb builder다.

```text
ExtensionBuilderId::Conv
  part_of_speech = Adverb
  parameter_schema = ...
  derive(parameter_noun) -> DerivedPrimitive
```

parameter가 적용된 뒤의 derived entity가 실제 computational contract를 가진다.

```text
DerivedPrimitive::Conv2d(params)
  innate_rank = 3
  ShapeInference
  AxisAndIterationSemantics
  AccessPattern
  NumericSemantics
  EffectSemantics
```

따라서 source-level `conv`와 LogicalOp `Conv2d`를 같은 `PrimitiveId` 하나로 뭉개지 않는다.

반면 NVIDIA/AMD/CPU의 실제 구현 선택은 같은 record 안에 target 숫자로 박지 않고 lowering registry가 제공한다.

```text
LoweringRegistry.lookup(
    resolved_semantic_operation,
    route,
    target_capabilities
) -> candidates
```

과거의 `register_fn`, `shared_memory_fn`, `accum_fn`은 semantic primitive property가 아니라 native planner용 realization/resource model 구현으로 옮긴다.

```text
ResourceUsage
  = R(LogicalGraph, PhysicalSchedule, TargetProfile)
```

이렇게 하면 extension primitive를 추가해도 모든 분석/optimizer/backend가 그 이름을 직접 알 필요가 없다.

<a id="4101-기존-j-primitive도-hardware-lowering의-주체다"></a>
#### 4.10.1 기존 J primitive도 hardware lowering의 주체다

hardware-aware capability/lowering 체계는 extension primitive에만 적용하지 않는다.

```text
J built-in
  +
  *
  %
  +/
  |:
  {
  $
  ...

extension-derived operation
  Conv2d
  Linear
  ...
```

모두 동일한 원칙을 따른다.

예를 들어 dyadic `+`는 target-independent하게 다음 semantic capability를 제공한다.

```text
PrimitiveId::Plus
  part_of_speech = Verb
  monad contract  = Conjugate/identity-for-supported-real-types semantics
  dyad contract   = Add semantics
  rank / agreement
  ShapeInference
  TypeSemantics
  AxisAndIterationSemantics
  AccessPattern
  NumericSemantics
  EffectSemantics
  RewriteLegality
```

그러나 GPU에서 실제로 실행하려면 별도의 lowering capability가 필요하다.

```text
Lowering bindings for Add
  generic
  CPU family
  GPU family
  CUDA
  ROCm
  architecture-specific override
  MLIR
  StableHLO-compatible lowering
  ...
```

즉:

> **primitive가 hardware-relevant information을 가져야 한다**는 말은 semantic record 안에 특정 GPU의 warp/tile/register 숫자를 넣는다는 뜻이 아니다. primitive identity를 key로 하여 (1) target-independent hardware-relevant semantic capability와 (2) target/backend/architecture별 lowering capability를 조회할 수 있어야 한다는 뜻이다.

개념 흐름:

```text
ResolvedSemanticOp
  source = PrimitiveId::Plus
  valence = Dyad
  semantic_op = Add
  derived_policies = {...}
    │
    ├─ semantic capabilities
    │    iteration/access/numeric/effect
    │
    └─ target lowering lookup
           +
       CompilationTarget
           ↓
       legal lowering candidates
           ↓
       Schedule / Physical realization
```



lowering key는 raw spelling/primitive id 하나가 아니다.

```text
LoweringKey
  source_entity_identity
  resolved_valence
  semantic_operation_kind
  derived semantic policies
    rank/cell context as needed
    fit/tolerance/numeric mode
    storage/effect mode
  representation preconditions (separate)
```

예를 들어 같은 `+` glyph라도 monadic `+ y`와 dyadic `x + y`는 같은 lowering으로 가정하지 않는다. `+/`, `+!.0`, `+"r` 같은 derived form도 semantic analysis 뒤 각각 필요한 reduction/numeric/rank contract를 가진 resolved operation으로 lookup한다.

locale binding은 여러 수준으로 등록할 수 있다.

```text
source primitive + valence handler
semantic op-class handler (ElementwiseAdd, ReductionAdd, ...)
specific derived-op specialization
```

가장 구체적인 binding만 의미가 맞는 경우에 사용한다.

모든 primitive가 모든 architecture에 전용 implementation을 가질 필요는 없다.

```text
architecture-specific binding exists
      → use/consider specialized lowering

otherwise backend-family binding exists
      → use generic backend lowering

otherwise generic/external lowering exists
      → use it if legal

otherwise
      → UnsupportedImplementation for that route/target
```

예를 들어 `|:` transpose는 semantic capability에서 static permutation을 알려주고, target lowering은 consumer가 permutation을 absorb할지, view/layout으로 유지할지, 실제 transpose kernel/copy를 만들지를 결정한다.

따라서 **컴파일 가능한 J built-in computational entity도 extension-derived entity와 동일한 capability/lowering architecture에 참여**한다. 다만 name/locale/control/system foreign처럼 runtime/effect semantics가 중심인 built-in을 억지로 pure array LogicalOp/GPU kernel로 만들지는 않는다.


<a id="411-rank와-axis-role은-서로-다른-정보다"></a>
### 4.11 rank와 axis role은 서로 다른 정보다

J rank가 알려주는 것은 argument를 frame과 cell로 어떻게 나누어 verb를 적용하는가이다. `AxisAndIterationSemantics` capability는 그 cell 내부의 각 축이 연산에서 어떤 역할을 하는가를 알려준다.

```text
full argument shape
        ↓ J rank semantics
frame axes | cell axes
             ↓ AxisAndIterationSemantics
       semantic axis roles
```

예를 들어 conv2d:

```text
full shape: [B, C, H, W]
innate cell rank: 3

frame: [B]
cell : [C, H, W]

cell axis 0 = channel / reduction input
cell axis 1 = spatial H
cell axis 2 = spatial W
```

따라서 흔히 말하는 **leading axis**도 전체 배열의 고정된 의미로 하드코딩하지 않는다. full array의 leading axis가 frame일 수 있고, primitive가 분석에 사용하는 것은 cell 내부 axis role이다. 필요하면 `cell axis 0`이 channel/reduction이라는 사실을 spec에 명시한다.

conv builder가 parameter noun을 받아 공간 차원을 확정한 뒤 생성하는 **derived conv verb**의 innate rank는 다음처럼 정리한다.

```text
conv1d cell = [C, W]       innate rank 2
conv2d cell = [C, H, W]    innate rank 3
conv3d cell = [C, D, H, W] innate rank 4
```

일반적으로 derived `conv_kd` verb의 innate rank는 `k + 1`이다. 공간 차원 외에 channel 축이 cell 안에 있어야 channel accumulation을 표현할 수 있기 때문이다. source adverb builder `conv` 자체에 하나의 고정 innate rank가 있다고 가정하지 않는다.

개념적인 `AxisRoleSpec`은 cell axis role, reduction axes, parallel axes, window axes, preserved axes, output-axis mapping을 가진다. `C/H/W` 같은 이름은 사람이 읽기 위한 label이고 analyzer는 reduction/parallel/window/static-reindex 같은 역할을 사용한다.

가변 reduction인 표준 J `+/` 같은 연산은 rank/cell 구조에서 axis가 유도된다. 반대로 derived conv verb처럼 축 역할이 연산 정체성에 고정된 연산은 `AxisAndIterationSemantics` capability가 그 역할을 제공한다.

<a id="4110-rankspec은-absoluteinfiniterelative-rank를-표현한다"></a>
#### 4.11.0 RankSpec은 absolute/infinite/relative rank를 표현한다

J의 rank를 단순 nonnegative `usize`로 모델링하지 않는다.

current jsource의 rank conjunction은 noun rank argument를 1~3개 값으로 해석하고, 음수 rank를 argument rank에 상대적으로 해석한다. infinite rank(`_`)도 별도 의미가 있다.

개념적으로:

```text
RankSpec
  Infinite
  Absolute(n)
  Relative(delta)   // negative rank: max(0, argument_rank + delta)
```

적용 시점:

```text
resolve_rank(RankSpec, argument_rank)
  → EffectiveCellRank
```

예:

```text
"_1 applied to rank-3 argument
  → effective cell rank 2

"_5 applied to rank-3 argument
  → clamp to 0

"_ applied to any argument
  → whole argument / infinite-rank semantics
```

monad/dyad rank list의 **source 원형은 `"` conjunction의 original operand에 보존**한다. Semantic Analyzer가 이를 해석한 뒤 별도의 analysis result로 resolved rank contract를 만들 수 있다.

```text
ResolvedRankContract
  monad_rank
  left_rank
  right_rank
```

`ResolvedRankContract`는 parser-produced Semantic IR node가 아니라 analysis 결과다.

`PrimitiveSpec`의 innate rank는 단수값이 아니라 **monad / dyad-left / dyad-right**별 `RankSpec`이다. 다만 current jsource primitive table의 innate rank는 nonnegative absolute rank 또는 infinite rank이므로, built-in `PrimitiveSpec.innate_rank`에는 `Absolute | Infinite`만 허용한다. `Relative`는 source `"`가 만드는 requested `RankBoundary`에 속한다. implementation integer sentinel과 동일시하지 않고 semantic `RankSpec`/resolved-rank abstraction을 사용한다.

설계 목표: jsource 내부의 `RMAX` 같은 정수 표현과 semantic `Infinite`를 분리한다. 현재 구현은 위의 rank 63 호환 표현을 유지하며, 명시적 `Infinite` IR 표현은 장기 설계 목표로 남겨 둔다.

또한 source-level rank conjunction을 `RankDerived(verb, integer)`로 고정하지 않는다. current jsource의 `jtqq`는 operand form이 더 넓다.

```text
FunctionEntity
  head = PrimitiveConjunction(Rank)   // source operator identity = "
  operands = [left_operand, right_operand]
  result_pos = parser/operator semantics가 정한 POS
```

분석 가능한 대표 form:

```text
Verb " RankNoun
Verb " Verb            // right verb의 monad/left/right ranks를 사용

Noun " RankNoun
  ├─ well-formed gerund + applicable rank → cyclic-gerund derived verb
  └─ otherwise → noun-derived constant verb semantics
```

따라서 J Semantic Array IR에서는 **원래 left/right operand의 품사와 value/entity identity를 보존**하고, Semantic Analyzer가 J의 rank-conjunction form 규칙을 적용해 derived verb를 만든다.

```text
FunctionEntity(head=", operands=[left, right])
        ↓ J semantic analysis
ResolvedRankDerived {
  source_entity,
  monad/left/right RankSpec,
  execution/assembly semantics
}
        ↓ function application
Logical CellApply
```

`MapCells` 같은 fixed-shape parallel form으로 더 낮추는 것은 `CellApply`의 uniformity/assembly 조건이 증명된 subset에서만 한다.


