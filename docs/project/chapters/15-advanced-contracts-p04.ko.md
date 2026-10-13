[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)

<a id="41149-첫-differential-test-matrix"></a>
##### 4.11.4.9 첫 differential test matrix

```text
A. direct/no-frame
B. monadic frame iteration
C. dyadic equal-frame cells
D. dyadic prefix agreement + left repeat
E. dyadic prefix agreement + right repeat
F. explicit " rank
G. negative rank
H. nested " boundaries
I. empty frame fill/prototype
J. heterogeneous result-cell assembly
K. sparse argument
L. generic CellApply vs absorbed path equivalence
```

특히 nested rank는 jsource `cr.c`가 명시한 intermediate fill-boundary 사례를 포함한다. 이 테스트가 통과하기 전에는 adjacent rank boundaries를 자동 collapse하지 않는다.

<a id="411410-jsource-구현에서-가져올-구조적-아이디어"></a>
##### 4.11.4.10 jsource 구현에서 가져올 구조적 아이디어

2026-09-30 current jsource master `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`의 `t.c`, `cr.c`, `ar.c`, `cf.c`, `result.h`, `jtype.h`를 기준으로 검토한다.

RustJ는 jsource의 C object layout, pointer/rank bit encoding, function-pointer dispatch를 복제하지 않는다. 대신 오랜 기간 검증된 다음 구조를 compiler architecture로 가져온다.

1. **primitive rank는 semantic contract다.** `t.c`의 monad / dyad-left / dyad-right rank table을 RustJ `PrimitiveSpec`의 기준 자료로 사용한다.
2. **explicit rank와 innate rank는 별도 boundary다.** `jtqq`가 source requested rank를 보존하고 `rank2ex`가 underlying rank와 outer/inner frame을 따로 처리하는 구조를 따른다.
3. **generic cell execution이 correctness baseline이다.** `rank1ex/rank2ex`에 대응하는 generic `CellApply`가 먼저 존재하고 IRS류는 loop-absorption optimization으로 취급한다.
4. **cell projection은 view다.** jsource virtual block처럼 logical cell을 복사하지 않고 projection/view로 표현한다.
5. **result assembly는 독립 semantics다.** `result.h`의 homogeneous fast path, type promotion, dynamic shape/fill, sparse/boxed, assembly error를 `CellApply` assembly contract로 옮긴다.
6. **empty frame은 zero-trip이 아니다.** fill cell/prototype invocation으로 result cell의 type/shape를 결정한다.
7. **reduce identity와 optimized reducer를 분리한다.** `jtslash`가 `u/` identity를 보존하면서 type/primitive별 reducer를 선택하는 것처럼, RustJ는 semantic `/(u)`에서 Logical `Reduce(u)`로 낮춘 뒤 realization을 선택한다.
8. **structural specialization은 원래 graph를 지우지 않는다.** `cf.c`가 `(+/ % #)`를 `jtmean`으로 specialize해도 `CFORK`와 f/g/h를 보존하는 방식을 따른다.
9. **sparse는 같은 semantics의 다른 realization이다.** dense와 sparse의 Logical `CellApply/Reduce` 의미는 공유하고 lowering/assembly 구현을 분리한다.
10. **jsource optimization flag는 의미를 재분류한 뒤에만 사용한다.** `VIRS*`, `VFUSEDOK2`, `WILLOPEN`, in-place/pristine flag를 그대로 Semantic IR bit로 가져오지 않는다.

대표 대응은 다음과 같다.

| jsource 개념 | RustJ에서의 의미 |
|---|---|
| primitive `mr/lr/rr` | `PrimitiveSpec.innate_rank` |
| `u"n` saved requested ranks | `RankBoundary` / requested `RankSpec` |
| `rank1ex/rank2ex` | Logical `CellApply` + generic CPU reference executor |
| `VIRS1/VIRS2` | `CanAbsorbCellApply` lowering capability |
| `VISATOMIC1/2` | atomic/cell semantic fact + realization capability |
| `VFUSEDOK2` | native lowering/kernel capability |
| `VNOLOCCHG` | effect summary의 `NoLocaleMutation` 증거 |
| `VNONAME/VNOSELF` | binding/self dependency summary의 참고 증거 |
| `VF2RANKATOP/RANKONLY` | Logical IR 이후 `CellApply` fusion candidate/proof |
| `VF2BOXATOP/ATOPOPEN/WILLOPEN` | explicit box/open semantics + use-def 기반 materialization-elision candidate |
| `VF2USESITEMCOUNT*` | shape/item-count fact 소비 capability |
| in-place/pristine/zappable flags | liveness/alias/bufferization 이후 physical reuse legality |
| pointer/rank bit encoding | 가져오지 않음 |

<a id="411411-이후-ir에서-사용할-정보의-provenance와-삽입-시점"></a>
##### 4.11.4.11 이후 IR에서 사용할 정보의 provenance와 삽입 시점

jsource 조사에서 얻은 정보는 **가능한 한 이른 시점**이 아니라 **의미를 정확히 알 수 있는 가장 이른 시점**에 넣는다. Parser가 알 수 없는 fact를 Parser IR에 미리 넣거나, target-dependent hint를 Logical IR semantic payload에 박지 않는다.

정보 수명은 다음 일곱 종류로 구분한다.

```text
A. SourceSemanticIdentity
   parser가 직접 만든 J entity/operand/binding 구조

B. StaticSemanticContract
   primitive/extension registry가 제공하는 target-independent 의미

C. ResolvedSemanticFact
   actual valence/argument facts와 결합한 뒤 Analyzer가 계산한 의미

D. OptimizationProof
   graph/use-def 분석으로 얻는 재계산 가능한 proof/candidate

E. LoweringCapability
   어떤 backend/kernel이 어떤 semantic form을 직접 처리할 수 있는가

F. RepresentationFact
   선택된 representation/layout에 대해 알려진 사실

G. PhysicalDecision
   concrete schedule/buffer/device realization
```

핵심 규칙:

> **A/B/C는 의미 보존에 관여하므로 optimization 전에 존재해야 하고, D는 invalidation 가능한 분석 결과이며, E는 IR 의미가 아니라 registry 정보이고, F/G는 route/schedule 이후에만 생긴다.**

구체적인 정보와 최초 삽입 시점은 다음과 같다.

| 정보 | jsource에서 보이는 근거 | RustJ 표현 | 최초 생성 단계 | 이후 활용 |
|---|---|---|---|---|
| operator identity, POS, operands, Hook/Fork 구조, source span | parser reduction / `fgh` cross-check | `FunctionEntity` | **Parser → Semantic IR** | derived analysis, pattern matching, diagnostics |
| NameRef와 binding/version identity | nameref/cache/fix machinery | `NameRef + BindingVersion/Guard` | **semantic binding** | specialization legality, deopt/guard, observable lookup order |
| name/self dependency | `VNONAME`, `VNOSELF`, `VXOPR` | `BindingDependencySummary` | **Function summary pass** | fix/specialization, hoisting, parallelization barrier |
| locale/path mutation 가능성 | `VNOLOCCHG` | `EffectSummary::LocaleMutation` | **PrimitiveSpec + Function summary** | order-edge 제거, CSE, speculation |
| try/catch/adverse/error behavior | `VTRY1/2`, adverse/runtime paths | `ErrorSemantics`, `Catchability` | **Semantic Analyzer** | observable order, fusion/speculation legality |
| supported valence | primitive table/action routine | `ValenceContract` | **PrimitiveSpec registry** | call legality, lowering key |
| innate monad/l/r rank | `t.c mr/lr/rr` | valence별 `RankSpec` | **PrimitiveSpec registry** | CellApply planning |
| explicit requested rank | `jtqq` saved rank noun/verb | `ConstructionFacts::RankBoundary` / requested `RankSpec` | **parser modifier semantic construction** | derived summary, nested CellApply |
| effective rank | `efr`, argument rank | `EffectiveCellRank` | **call analysis** | frame/cell split |
| frame/cell split | `rank1ex/rank2ex` | `CellApplicationPlan` | **call analysis** | Logical CellApply, schedule axes |
| prefix agreement/common frame | `ASSERTAGREE` | `AgreementFact` | **CellApplicationPlanner** | legality, repetition/index maps |
| residual repeated side | rank loop repeat state | `RepetitionFact` | **CellApplicationPlanner** | zero-stride/index-map lowering |
| atomic/cellwise semantics | `VISATOMIC1/2` | `AtomicitySemantics` | **PrimitiveSpec / derived summary** | access relation, absorption/fusion proof |
| result shape/rank | primitive semantics + cell assembly | `ValueFacts.shape/rank` | **Analyzer dataflow** | verifier, route, schedule |
| result dtype/type classes | type dispatch/promotion tables | `TypeSemantics + TypeFact` | **PrimitiveSpec + Analyzer** | kernel selection, conversion insertion |
| overflow retry/promotion | `EWOV*` retry paths | `OverflowSemantics` | **PrimitiveSpec / resolved op** | vector/reduction legality |
| comparison tolerance / fit | runtime cct, `!.` constructor | construction policy + `NumericSemantics` | **modifier semantic construction + call resolution** | comparison kernel, guard/runtime input |
| neutral/fill/empty behavior | `red0`, filler paths | `FillAndEmptySemantics` | **PrimitiveSpec + derived policy** | empty CellApply/Reduce |
| result-cell assembly policy | `result.h` | `AssemblyFact` | **Analyzer**, proof refined by optimizer | fixed output allocation vs dynamic assembly |
| uniform result-cell proof | homogeneous `result.h` fast path analog | `UniformCellResultProof` | **Analyzer/Logical Optimizer** | MapCells/GPU eligibility |
| semantic axis roles | primitive/derived operation meaning | `AxisSemantics` | **resolved-op analysis** | reduction/parallel/window mapping |
| logical access relation | atomic/reduce/reindex/gather semantics | `AccessRelation` | **resolved-op analysis** | fusion/locality/route planning |
| shape constraints, item count, nonempty/divisibility | shape/value facts | `ConstraintSet`, item-count fact | **Analyzer** | specialization, loop bounds, allocation |
| dense/sparse/boxed semantic representation | sparse/boxed branches | `Layout/RepresentationFact` | **value analysis** | route/lowering selection |
| `BoxAtop/Open/Raze` producer-consumer opportunity | `VF2BOXATOP/ATOPOPEN/WILLOPEN` | explicit ops + `MaterializationElisionCandidate` | semantic ops은 **Analyzer**, candidate는 **Logical Optimizer use-def pass** | box/open elimination, producer-consumer fusion |
| item-count propagation opportunity | `VF2USESITEMCOUNT*` | `CanConsumeItemCountFact` | **lowering capability registry + use-def analysis** | avoid materializing producer result |
| rank-loop absorption | `VIRS1/VIRS2` | `CanAbsorbCellApply` | **lowering registry** | native/external lowering choice |
| special fused executor | `VFUSEDOK2`, `jtmean`, composition special cases | `FusionCandidate + LoweringCapability` | candidate는 **Logical Optimizer**, capability는 **registry** | fused kernel selection |
| cell-loop fusion | `RANKATOP/RANKONLY` subsumption | `CellApplyFusionProof` | **Logical Optimizer** | remove nested loop boundary only if legal |
| actual alias/in-place reuse | in-place/pristine/zappable | `Liveness/AliasProof` + `MaterializationDecision` | **Physical Planner / Bufferization** | buffer reuse |
| actual offset/stride/alignment | virtual/physical array layout | `RepresentationFacts` | **representation/physical lowering** | vectorization, coalescing, view realization |
| address space/device placement | 없음/CPU implementation detail | physical placement facts | **Physical Planner** | transfer/synchronization |
| tile/vector/workgroup mapping | implementation fast path | `PhysicalSchedule` | **Schedule stage** | codegen |
| resource/cost estimate | implementation-specific | `ResourceEstimate/CostEstimate` | **schedule + target 이후** | candidate ranking |

<a id="semantic-node는-intrinsic-summary를-직접-소유한다"></a>
###### Semantic node는 intrinsic summary를 직접 소유한다

Parser-produced `FunctionEntity`는 source identity/operand 관계뿐 아니라 **그 entity 자체에 안정적으로 귀속되는 semantic summary를 직접 소유**한다. derived entity는 이미 완성된 child entity의 `semantic_info`를 재귀적으로 참조해 bottom-up으로 summary를 만든다.

```text
FunctionEntity                      // parser-owned, immutable, shared by Arc/EntityRef
  id
  identity / POS / operands / span
  semantic_info:
    construction
    valence_contract
    rank_contract
    binding_dependency
    effect_summary
    error_summary
    atomicity
    latent_semantics
```

primitive의 상세 규칙은 `PrimitiveSpec`을 authoritative source로 두되 node의 `semantic_info`는 이후 재귀 분석에 필요한 안정적인 contract/summary를 제공한다. 큰 derived function에서도 child subtree 자체는 shared reference이므로 복사되지 않으며, parent에는 **parent 자신의 summary 한 벌만** 저장된다.

side table/cache는 다음에만 사용한다.

- 비용이 큰 재계산 가능 분석
- 특정 pass에서만 유효한 proof
- use-def 전체를 봐야 하는 정보
- target/device/cost dependent 정보

즉 semantic traversal에서 매번 전역 table을 조회해야만 child의 의미를 알 수 있는 구조는 기본 모델로 삼지 않는다.

<a id="call-time에만-알-수-있는-정보는-resolvedcallfacts로-모은다"></a>
###### call-time에만 알 수 있는 정보는 ResolvedCallFacts로 모은다

rank agreement, actual frame/cell, repeated side, output constraints는 function 자체의 속성이 아니라 **function + valence + actual argument facts**의 속성이다.

```text
ResolvedCallFacts
  valence
  effective_cell_ranks
  CellApplicationPlan
  AgreementFact
  RepetitionFact
  AxisSemantics
  AccessRelations
  NumericSemantics
  FillAndEmptySemantics
  AssemblyFact
  Effect/Error/Alias requirements
  result ValueFacts
```

Semantic Analyzer가 이를 만든 뒤 필요한 부분을 verified Logical IR payload/annotation으로 내린다.

<a id="logical-ir-payload와-재계산-가능한-analysis-fact를-구분한다"></a>
###### Logical IR payload와 재계산 가능한 analysis fact를 구분한다

다음은 semantic correctness를 위해 Logical IR에 남아야 한다.

```text
must-preserve
  operation kind / operands / valence
  CellApply boundary and agreement/repetition semantics
  rank/fit/tolerance/fill policy
  dynamic assembly requirement
  observable effect/error/order requirements
  storage/state-resource identity
```

반면 다음은 side analysis로 두고 graph rewrite 후 재계산 가능하게 한다.

```text
recomputable
  UniformCellResultProof
  FusionCandidate::Mean
  CellApplyFusionProof
  MaterializationElisionCandidate
  invariance
  item-count propagation opportunity
  route eligibility
```

외부 IR adapter가 correctness를 위해 필요한 fact는 adapter precondition/witness로 승격하며, 단순 optimizer hint는 export semantics에 포함하지 않는다.

<a id="jsource의-memory-management-flags는-physical-ir로-바로-복사하지-않는다"></a>
###### jsource의 memory-management flags는 Physical IR로 바로 복사하지 않는다

`WILLOPEN`, in-place, pristine, zappable은 jsource의 reference-counted runtime에서 매우 중요한 정보지만 RustJ에서는 대부분 **use graph + liveness + ownership/alias + representation**으로 다시 유도하는 편이 낫다.

예를 들어:

```text
producer -> Box -> consumer Open
```

이 semantic graph가 있으면 Logical Optimizer가:

```text
MaterializationElisionCandidate
  producer result does not escape
  box/open semantics cancel under proof
```

를 만들고, 실제 buffer reuse 여부는 Physical Planner가 결정한다.

즉 jsource의 `WILLOPEN` bit를 parser/LogicalOp에 복사하지 않는다.

<a id="verifier는-fact의-출처와-가정을-구분한다"></a>
###### verifier는 fact의 출처와 가정을 구분한다

향후 analysis fact에는 필요하면 다음 provenance를 둔다.

```text
FactProvenance
  SourceSemantic
  PrimitiveSpec
  DerivedSummary
  InferredFrom(ValueId...)
  ProvenBy(pass)
  GuardedAssumption(GuardId)
```

특히 dynamic shape/rank/name binding을 compile-time fact로 사용했다면 `GuardedAssumption` 또는 runtime semantic dependency가 있어야 한다. 근거 없는 `Known` fact는 verifier가 허용하지 않는다.

<a id="이-설계를----y에-적용하면"></a>
###### 이 설계를 `(+/ % #) y`에 적용하면

`y =: i. 2 3`에서:

```text
Parser/Semantic IR
  Fork(
    +/ : derived Verb,
    %  : Verb,
    #  : Verb
  )

where the first fork operand is one shared derived-function entity:

  +/ : Verb
    head = PrimitiveAdverb(Insert)   // /
    operands = [ + : Verb ]

PrimitiveSpec
  + dyad rank = 0 0
  % dyad rank = 0 0
  # monad rank = Infinite

Function summary
  fork structure preserved
  pure/no-locale-change subset proven where applicable

Call analysis
  +/ y:
    Reduce(Add)
    reduction axis = leading axis of current cell
    result shape = [3]

  # y:
    Tally
    result shape = []

  (3 5 7) % 2:
    effective ranks = 0 / 0
    left frame = [3]
    right frame = []
    repeat right cell
    Logical CellApply2
    output shape = [3]

Logical Optimizer
  generic graph remains correctness form
  recognize FusionCandidate::Mean
  prove UniformCellResult if facts suffice

Route/lowering
  generic Reduce + Tally + CellApply(Divide)
  or verified fused mean kernel

Schedule/Physical
  choose SIMD/thread/GPU mapping
  choose buffers/views/reuse
```

따라서 `(+/ % #)` 예제에서 **mean이라는 정보는 Parser가 넣는 정보가 아니다.** Parser는 Fork를 보존하고, Analyzer는 rank/cell/access semantics를 넣고, Logical Optimizer가 mean pattern을 발견하며, target lowering이 실제 fused mean implementation을 선택한다.

<a id="구현-순서에-반영"></a>
###### 구현 순서에 반영

IL1/IL2 구현과 함께 다음 infrastructure를 추가한다.

1. `PrimitiveSpec`에 현재 지원 primitive의 valence별 innate rank/atomicity/effect/error/fill 핵심 contract를 넣는다.
2. `FunctionEntity.semantic_info`를 추가하여 shared DAG의 각 node가 자신의 name/self/effect/error/rank/atomicity/latent semantic summary를 직접 소유하게 한다. construction 시 child summary를 bottom-up으로 조합하고, pass-local 재계산 proof만 별도 cache/table로 둔다.
3. current `Node.facts + rank_plan + access`를 점진적으로 `ResolvedCallFacts / LogicalFacts` 구조로 모으되 한 번에 거대한 enum/struct로 바꾸지 않는다.
4. `CellApplicationPlanner`가 effective rank, frame/cell, agreement, repetition, empty policy를 생산하게 한다.
5. `logical_ir::Plan::verify`가 must-preserve fact의 정합성을 검증한다.
6. use-def 기반 `OptimizationFacts` pass를 추가하여 uniform-result, mean/fusion, CellApply fusion, materialization-elision candidate를 만든다.
7. `LoweringRegistry`에는 IRS/FUSEDOK에 대응하는 capability만 등록하고 semantic fact와 섞지 않는다.
8. 실제 stride/alignment/in-place/buffer reuse는 Logical IR 구현보다 뒤의 representation/physical 단계에서만 추가한다.


<a id="mean-proof-example"></a>

<a id="411412-canonical-e2e-표본-----y"></a>
##### 4.11.4.12 Canonical E2E 표본 — `(+/ % #) y`

이 표현을 compiler pipeline의 고정 추적 표본으로 사용한다. 단순 vector 입력만 사용하면 implicit cell application이 드러나지 않으므로 두 입력을 구분한다.

```j
y =: 1 2 3 4
(+/ % #) y
```

은 parser/train/lowering smoke test로 사용한다. 더 강한 E2E semantic test는 다음이다.

```j
y =: i. 2 3
(+/ % #) y
```

J 의미상:

```text
+/ y  -> 3 5 7
#  y  -> 2
3 5 7 % 2 -> 1.5 2.5 3.5
```

current jsource는 이 source structure를 fork로 유지한다. `cf.c::jtfolk`는 `f=+/`, `g=%`, `h=#`인 fork를 인식해 monadic executor를 `jtmean`으로 specialize하지만, derived entity 자체는 `CFORK`와 원래 f/g/h operands를 유지한다. `ar.c::jtslash`가 만든 `+/`의 monadic rank는 infinite(`RMAX`)이고, primitive table에서 `#`의 monadic rank도 infinite, `%`의 dyadic left/right rank는 `0 0`이다. 따라서 matrix 입력에서는 마지막 `%`가 다음 implicit cell application을 요구한다.

```text
left  = +/ y : shape [3], dyad cell rank 0
right = #  y : shape [],  dyad cell rank 0

CellApply2
  left_frame  = [3]
  left_cell   = []
  right_frame = []
  right_cell  = []
  agreement   = prefix
  repetition  = repeat right scalar over left residual frame
  result      = shape [3]
```

현재 코드 대조 기준: **2026-10-02, `98ae387`**. 다음은 재실행한 테스트 결과가 아니라 source/API 검토 결과다.

| 경계 | 현재 상태 | 아직 검증/구현할 것 |
|---|---|---|
| FunctionEntity → J Graph | 이 표본의 shared Fork와 applied stage/branch 구조를 보존한다 | 전체 frontend의 mixed phrase/POS/construction 의미는 M2 gate를 따른다 |
| J Graph → canonical Logical IR | `analysis::lower_graph()`가 `logical_ir::Plan`을 직접 생성한다. `CompilationAnalysis.logical`과 `Engine::analyze/analyze_a3`가 같은 canonical plan을 사용한다 | canonical container cutover는 M1 완료. 새로운 lowering은 transition IR을 만들지 않는다 |
| Reduce / CellApply payload | A3의 `ExecutionBasisPayload`, `CallOp.execution_basis`와 explicit rank-plan seam은 존재한다 | plain `%`의 innate rank와 일반 implicit CellApply/assembly를 완성해야 한다. `Callable.reduce/rank` migration field도 아직 남는다 |
| verifier / reference execution | `Plan::verify()`와 `logical_executor::execute_closed()`가 존재한다 | closed-plan reference 실행을 named/environment 전체 지원 또는 Physical Executor로 오인하지 않는다 |
| route / physical execution | `LoweringRegistry`와 contiguous route-partition prototype이 존재한다 | 선택된 native Schedule/Physical Plan/CPU Physical Executor는 M4 이후의 미완료 경계다 |

<a id="같은-표본의-stage-by-stage-compiler-trace"></a>
###### 같은 표본의 stage-by-stage compiler trace

이 표본은 앞으로 문서와 구현에서 **한 source를 처음부터 끝까지 같은 provenance로 추적하는 canonical trace**로 사용한다. 현재 구현된 경계와 planned 경계를 섞지 않는다.

~~~text
J source
  (+/ % #) y
      │
      ▼
[현재 구현] parser / Semantic Construction
  Fork
    f = +/        // Insert(+) derived Verb
    g = %
    h = #
  source span / operand identity / observable fork order 보존
      │
      ▼
[현재 구현] J Graph IR
  input y
      ├─ h branch: Apply Tally(y) ──────────────┐
      └─ f branch: Apply Reduce(Add, y) ────────┤
                                                ▼
                                      Apply Divide(f, h)
  + Fork region/provenance
  + branch/join/use/liveness facts
  + Graph Basis / GraphHint
      │
      ├─→ [현재 분석] rewrite/fusion/resource/work-depth 후보·side analysis
      │       candidate는 source graph를 대체하거나 실행을 확정하지 않음
      │
      ▼
[현재 구현] Execution Semantic Lowering → canonical A3
  observable order: h → f → g
  Tally(y)
  Reduce(Add, y)
  Divide(left=reduce, right=tally)
    + valence/rank/cell/frame facts
    + prefix-agreement / repetition constraint or witness
    + SemanticCheck only when a required constraint remains unresolved
    + effect/error/speculation/order metadata
      │
      ▼
[현재 구현] A3 verifier / reference execution capability
  Plan::verify()
  logical_executor::execute_closed() on its supported closed subset
      │
      ▼
[현재 프로토타입] route analysis
  LoweringRegistry::route_operation / partition_plan
  native realization이 없는 op은 RuntimeSemanticFallback class일 수 있음
  contiguous class grouping은 아직 final mixed-route plan이 아님
      │
      ───────────── 현재 compiler-native physical 실행의 stop line ─────────────
      │
      ▼
[planned M4] deterministic CPU Schedule / PhysicalPlan
  BindInput(y)
  Check(...)             // A3에 unresolved SemanticCheck가 있을 때만
  Kernel Tally
  Kernel Reduce(Add)
  View/iteration mapping // right scalar를 J cell semantics에 따라 반복, 필요 시 metadata-only
  Kernel Divide
  Return(result)
      │
      ▼
[planned M4 validation]
  result = 1.5 2.5 3.5
  + jsource와 value/error/order differential
  + allocation/view/reuse invariants
~~~

중요한 점은 **Mean이라는 fused operation이 correctness baseline이 아니라는 것**이다. 이 표본의 첫 compiler-native 성공 조건은 위 generic `Tally + Reduce + CellApply/Divide` 경로다.

그 이후에만 다음과 같은 optimization candidate를 별도로 고려할 수 있다.

~~~text
source generic graph
  Tally + Reduce(Add) + CellApply(Divide)
          │
          └─→ possible Mean-style fused/composite candidate
                 equivalence / rank-cell assembly
                 numeric + error/effect order
                 fanout/retention
                 resource/work-depth
                 target capability
                 profitability
                 모두 필요한 proof 이후에만 select
~~~

현재 `j_graph_fusion`의 등록 규칙이나 `fusion_planning`이 이 Mean specialization 전체를 이미 선택·lowering한다고 해석하지 않는다. 이 부분은 **future optimization example**이며, 먼저 generic graph의 semantic/physical E2E가 검증되어야 한다.

<a id="이-trace의-provenance-연결-규칙"></a>
##### 이 trace의 provenance 연결 규칙

- parser `FunctionEntity`의 Fork/operand/source span을 J Graph region과 연결한다.
- 각 J Graph `ValueId`는 A3 operation의 `j_origin`으로 추적 가능해야 한다.
- candidate는 source graph/version과 rule/witness provenance를 가진다.
- RoutePartition과 PhysicalPlan은 canonical A3를 destructive하게 덮어쓰지 않고 source logical operation/provenance를 참조한다.
- Physical `Kernel`/`Check` 실패를 보고할 때 가능한 한 A3 op → J Graph origin → source span으로 역추적한다.
- planned M4 trace가 실제 구현되기 전에는 이 그림의 PhysicalPlan 구간을 구현 완료 증거로 사용하지 않는다.

shape `[3]` 추론만으로 `%`의 prefix agreement와 residual-frame repetition을 구현했다고 판정하지 않는다. 이 표본의 semantic contract 검증은 다음으로 좁힌다.

- [ ] primitive/derived callable의 valence별 innate `RankSpec`과 explicit `"`가 공통 CellApplicationPlanner를 사용한다.
- [ ] matrix 표본의 `Divide`가 위 `CellApply2` frame/cell/repetition을 보존하는 golden을 추가한다.
- [ ] verifier가 input/result facts, frame/cell split, prefix agreement와 repetition relation을 검사한다.
- [ ] `Reduce + Tally + CellApply(Divide)`를 M4의 최소 physical 경로로 실행하고 jsource와 differential 검증한다.
- [ ] 그 generic 경로의 의미 동등성이 검증된 뒤에만 fused Mean 후보를 추가한다. semantic Fork identity는 유지한다.

`tests/analysis.rs::canonical_mean_fork_lowers_to_reduce_tally_divide_in_jsource_order`는 `analyze_a3 + verify`와 h → f → g ordering을 확인하는 smoke test다. 실제 실행이나 compiler-native E2E 완료의 증거는 아니다. M4의 실행 체크리스트와 이 semantic 표본의 조건은 별개로 추적한다.

