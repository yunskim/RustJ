[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)

<a id="다른-array-languagecompiler-basis와의-역대조"></a>
##### 다른 array-language/compiler basis와의 역대조

비교 자료:
- Futhark minimal basis: https://www.futhark-lang.org/blog/2019-04-10-what-is-the-minimal-basis-for-futhark.html
- Futhark SOACs: https://www.futhark-lang.org/docs/prelude/doc/prelude/soacs.html
- Accelerate: https://hackage.haskell.org/package/accelerate/docs/Data-Array-Accelerate.html
- Lift: https://lift-project.readthedocs.io/en/latest/lift-overview/
- SaC: https://sac-home.org/_media/docs%3Atutorial.pdf
- MLIR Linalg: https://mlir.llvm.org/docs/Dialects/Linalg/
- NESL: https://www.cs.cmu.edu/afs/cs/project/pscico/doc/nesl/manual/

| RustJ structure | 외부 비교에서 반복되는 근거 | v0.2 결정 |
|---|---|---|
| Elementwise/MapN | Futhark map, Accelerate map/zipWith, Lift map, NESL apply-to-each, Linalg parallel iterator | 유지 |
| Reduce | Futhark/Accelerate/NESL에서 구조를 직접 보존; generic reconstruction은 fusion/parallel reduction 기회를 잃을 수 있음 | 강하게 유지 |
| Scan | Futhark/NESL에서 독립 parallel prefix 구조 | 강하게 유지 |
| RegularReindex | Accelerate backpermute, Linalg indexing maps, SaC index expressions | 유지 |
| Gather | indexed/irregular read의 memory/coalescing 특성이 regular reindex와 다름 | 유지 |
| Scatter/ScatterCombine | Futhark scatter/reduce_by_index, Accelerate permute, NESL indexed write | 강하게 유지 |
| WindowView | Lift slide/pad, Accelerate stencil, LAILA pull-array/index-function 접근과 같은 neighborhood reuse 정보 | 유지 |
| SegmentView | segmented fold/scan, NESL flatten/partition과 연결 | 유지 |
| Contract | Linalg contract처럼 contraction identity가 GEMM/tensor/microkernel 선택에 유용 | 유지 |
| CellApply | J rank/cell/frame와 result assembly를 explicit하게 보존해야 하며 uniform proof 뒤에만 MapCells/flat parallel form으로 낮출 수 있음 | J-specific structured basis로 유지 |
| Grade | 다른 언어에서는 library/algorithm으로 구성 가능하지만 J primitive identity와 ordering algorithm 선택이 직접 중요 | 유지, refinement는 후속 |
| Lookup/Classify/GroupBy | Futhark reduce_by_index/segmented routes와 유사한 hash/sort/atomic/segment 선택 공간 | 유지 |
| NestedTraverse | NESL nested sequences가 보여 주듯 irregular nested traversal은 dense Map과 다른 구조 | 유지 |
| LinearSolve | named structured op/library-call granularity를 보존하는 것이 solver 선택에 유리 | 유지 |
| StateMachine | 비교 언어의 core array basis에서는 약한 대응; J ;: semantics 때문에 우선 structured kernel로 보존 | provisional |

Futhark가 보여 주는 중요한 경고는 **표현상 minimal basis와 optimization basis가 다르다**는 점이다. 많은 연산을 map+iota+scatter 류로 재구성할 수 있어도 reduce 같은 identity를 지우면 fusion/parallel implementation 선택을 잃을 수 있다. RustJ는 따라서 “분해 가능”을 refinement 이유로 사용하지 않는다.

<a id="42412-j-primitive--provisional-execution-basis-coverage-matrix"></a>
#### 4.24.12 J primitive → provisional execution-basis coverage matrix

검토 기준:
- 현재 vocabulary 기준: [NuVoc](https://code.jsoftware.com/wiki/NuVoc), 2026-10-05 확인, [관찰 revision 60409](https://code.jsoftware.com/mediawiki/index.php?title=NuVoc&oldid=60409). 과거 [Dictionary Vocabulary](https://www.jsoftware.com/help/dictionary/vocabul.htm)는 역사적 참고이며 현재 지원 목록으로 사용하지 않는다.
- jsource 계열 special phrase inventory: https://jsoftware.com/help/dictionary/special.htm

이 표의 목적은 **J의 source primitive를 basis vocabulary로 대체하는 것**이 아니다. source/parser/Semantic IR에서는 원래 J primitive/derived identity를 보존하고, Semantic Analyzer가 call facts를 해석한 뒤 Logical IR에서 다음 basis graph를 만든다.

분류:
- **Direct**: 하나의 basis identity가 중심
- **Compose**: 둘 이상의 basis를 명시적으로 조합
- **Structured**: 큰 semantic/algorithm identity를 현재 유지
- **Control**: function/control IR이 basis graph의 조합/반복을 결정
- **Value/Rep**: value construction 또는 representation axis
- **Runtime**: stateful/dynamic/runtime semantic route
- **Cell kernel**: array traversal은 CellApply/Elementwise가 담당하고 내부 scalar/cell algorithm은 opaque 가능

| J primitive/form | 의미 요약 | provisional basis expansion | 필요한 facts/checks | 분류 / 성능상 이유 |
|---|---|---|---|---|
| = | Self-Classify / Equal | monad Classify; dyad Elementwise(Equal) | equality semantics, dtype, tolerance/fit | Direct; classification identity는 hash/sort/group route에 유용 |
| < | Box / Less Than | monad value Box; dyad Elementwise(Lt) | box/value type; comparison domain | Value/Rep + Direct |
| <. <: >. >: | floor/min, decrement/≤, ceiling/max, increment/≥ | Elementwise | dtype/promotion/domain | Direct |
| > | Open / Greater Than | monad value Open; dyad Elementwise(Gt) | boxed/open assembly; comparison domain | Value/Rep + Direct |
| + +. +: * *. *: - % %: ^ ^. &#124; | scalar arithmetic/logical/transcendental families | Elementwise(MapN) with scalar payload | dtype, promotion, domain, overflow/FP/error policy | Direct; scalar payload는 target intrinsic/SIMD 후보 |
| -. | Not / Less | monad Elementwise(Not); dyad Lookup(Membership in y) → Compact(items of x not present) | item-cell agreement, equality/tolerance, stable order | Compose; dyadic Less는 set/item exclusion이므로 scalar elementwise로 축소하지 않음 |
| -: | Halve / Match | monad Elementwise(Halve); dyad CellApply(MatchKernel), optionally shape/box check + Equal + Reduce(And) | shape, boxing, tolerant equality, rank/cell | Cell kernel/Compose; whole-cell Match identity를 보존 |
| %. | Matrix Inverse / Matrix Divide | LinearSolve or related structured linear-algebra kernel | rank/shape, singularity, numeric contract | Structured; LU/QR/SVD/library/sparse solver 선택을 보존 |
| $ | Shape Of / Reshape | monad ShapeFact; dyad RegularReindex with periodic/fill policy | target shape values, item count, fill/error | Compose; copy를 피하는 view/index-map 최적화 |
| $. | Sparse representation operations | same semantic basis op + RepresentationFacts::Sparse; explicit conversion/inspect is representation op | sparse axes/fill/index/value validity | Value/Rep; sparse는 별도 semantic basis가 아님 |
| ~. | Nub | Classify → Compact(first representative) | equality/tolerance, stable-first semantics | Compose; hash/sort specialization 가능 |
| ~: | Nub Sieve / Not-Equal | monad Classify → first-mask; dyad Elementwise(Ne) | equality/tolerance | Compose/Direct |
| &#124;. | Reverse / Rotate | RegularReindex | axis/rank, shift normalization | Direct; no-copy/index-map path |
| &#124;: | Transpose | RegularReindex(Permutation) | axis permutation validity | Direct; layout/coalescing optimization |
| . | Determinant / Dot Product | monad CellApply(DeterminantKernel); dyad Contract(u,v) | rank, contraction axes, reducer/combine semantics | Cell kernel + Direct; contraction identity enables GEMM/tensor routes |
| , | Ravel / Append | monad RegularReindex; dyad Concat/Assemble | shape agreement, dtype promotion | Direct |
| ,. ,: | Ravel Items/Stitch, Itemize/Laminate | RegularReindex and/or Concat/Assemble | rank/shape agreement | Compose |
| ; | Raze / Link | Open/boxed value traversal + Concat/Assemble | box/open validity, result assembly | Compose + Value/Rep |
| ;. | Cut | fret cuts SegmentView → CellApply; tessellation WindowView → CellApply | fret/window spec, boundaries, fill, assembly/errors | Compose; avoids building cut cells and exposes segment/window parallelism |
| ;: | Words / Sequential Machine | monad configured StateMachine; dyad general StateMachine | transition/input tables, state, emission, errors | Structured provisional; future SIMD/transition-composition possible |
| # | Tally / Copy | monad shape/item-count fact; dyad Replicate/Compact/Expand | counts nonnegative/integral, result size | Direct; prefix/compaction implementation 선택 가능 |
| #. | Base 2 / Base | Scan/Generate weights → Elementwise(Mul) → Reduce(Add) or preserved base kernel | radix/value domain, overflow | Compose; keep specialized kernel if cheaper |
| #: | Antibase 2 / Antibase | Generate/Scan radix weights + Elementwise(div/residue) + Assemble | radix/domain/result shape | Compose |
| ! | Factorial / Out Of | Elementwise scalar/cell combinatorial kernel | integer/general numeric domain | Cell kernel; internal algorithm need not be array-basis-expanded |
| !. | Fit | modifies semantic/numeric/error contract of operand graph | fit/tolerance/fill policy | Control/semantic annotation |
| !: | Foreign | runtime/foreign semantic route | foreign id, effects/capability | Runtime |
| / | Insert / Table | monad Reduce(u); dyad IndexSpace/FrameMap → CellApply(u) | reducer order/associativity/identity, rank/assembly | Direct/Compose; reduction identity retained even when non-reassociable |
| /. /.. | Oblique / Key / Key dyad | oblique SegmentView/Reindex → CellApply; key Classify/GroupBy → SegmentView → CellApply, optionally grouped reduction | grouping equality/order, segment descriptors, assembly | Compose; avoids materialized groups and opens reduce-by-index route |
| /: \: | Grade Up/Down / Sort | Grade; sort result can be Grade → Gather | comparison order, stability/tolerance, dtype | Direct; radix/merge/small/GPU algorithm identity retained |
| \ | Prefix / Infix | insert-compatible prefix Scan; general prefix SegmentView(prefix family) → CellApply; infix WindowView/SegmentView → CellApply | window length, order, boundaries, assembly | Direct/Compose |
| \. | Suffix / Outfix | suffix Scan when legal or segment family; outfix SegmentView + Concat/Assemble → CellApply | same as above | Compose |
| [ ] [: | Same/Left, Same/Right, Cap | value projection / function-graph semantics | valence, provenance | Control/value; no new compute basis |
| [. ]. ]: | Lev / Dex conjunction, Ident adverb | constructor에서 기존 noun/verb operand 선택 | 실제 result POS, operand reduction/효과, noun snapshot·verb late binding | Control/value; runtime 배열 kernel이나 병렬 힌트가 아님 |
| { | Catalogue / From | monad Cartesian IndexSpace + Gather + Assemble; dyad Gather | index bounds, boxed catalogue shapes | Direct/Compose |
| {. {: }. }: | Head/Tail/Take/Drop/Behead/Curtail | RegularReindex or scalar Gather | bounds, fill, rank | Direct |
| {:: | Map / Fetch | NestedTraverse to produce leaf paths; fetch = path-guided nested Gather/Open | path validity, boxed structure | Direct + Value/Rep |
| } | Item Amend / Amend | Scatter/Amend, possibly ScatterCombine when combining update is explicit | bounds, overlap/update ordering, alias | Direct; in-place/atomic/scatter choices |
| " | Rank | resolve rank/cell/frame then CellApply | RankSpec, frame agreement, fill/empty/result assembly, error order | Control → Direct basis; central J optimization boundary |
| ". | Do / Numbers | parse/execute or numeric-conversion runtime semantic route | dynamic binding/parser state, numeric syntax | Runtime |
| ": | Default Format / Format | formatting runtime/cell kernel | locale/format spec, dtype | Runtime/Cell kernel |
| Tie/Evoke Gerund | gerund construction/selection | function/gerund semantic graph | POS/binding/gerund selection | Control |
| @ @: @. | Atop/At/Agenda | compose/select function graphs; resulting calls lower normally | valence, selected branch, effects | Control |
| & &: &. &.: | Bond/Compose/Appose/Under | function graph transformation; Under keeps inverse/obverse contract | inverse availability, rank/valence, effects | Control |
| ? ?. | Roll/Deal | stateful random generation; array shape may use Generate | RNG state/seed, domain, uniqueness for deal | Runtime + optional Generate |
| a. a: | Alphabet / boxed empty constant | constant/value construction | encoding/value type | Value |
| A. | Anagram Index / Anagram | permutation rank/unrank cell kernel; application Permute/Gather | permutation validity/range | Structured cell kernel + Direct |
| b. | Boolean/bitwise 또는 Verb Info | noun-operand bitwise kernel 또는 function rank/identity/obverse query | operand 종류, function header, numeric domain | Cell kernel/control; 정보 조회와 계산을 구분 |
| C. | Cycle-Direct / Permute | permutation representation conversion + Permute/Gather | cycle/direct validity | Value/Rep + Direct |
| e. | Raze In / Member | monad value traversal/raze-in; dyad Lookup(Membership) | equality/tolerance, boxed semantics | Direct + Value |
| E. | Find Matches (dyad) | WindowView → Elementwise(Match) → Reduce/Match as applicable | pattern shape, equality, boundaries | Compose; explicit windows expose fusion |
| f. f: | Fix 계열 | function/name fixing 정책을 가진 transformation | binding/POS, fix mode, implicit locative·recursion 범위 | Control; 두 표기의 동등성은 가정하지 않음 |
| H. | Hypergeometric | CellApply specialized numeric kernel | parameter/domain/numeric policy | Cell kernel |
| i. | Integers / Index Of | monad IndexSpace/Generate; dyad LookupFirst | shape/integer domain; equality/tolerance | Direct |
| i: | Steps / Index Of Last | monad Generate; dyad LookupLast | same | Direct |
| I. | Indices / Interval Index | monad Compact(IndexSpace,predicate); dyad ordered interval Lookup | ordering, bounds | Direct/Compose |
| j. r. o. | complex/polar/circle families | Elementwise scalar payload | numeric/domain | Direct |
| L. | Level Of | NestedTraverse/nested metadata computation | box tree shape | Direct; tree traversal distinct from dense map |
| L: | Level At | NestedTraverse(level selector) → CellApply(u) → nested reconstruct | level selector, reconstruction, assembly | Direct/Compose |
| M. | Memo | memoization/cache around semantic function graph | key equality, effects/purity | Control/runtime |
| p. p.. | Polynomial roots/evaluation/derivative/integral | specialized cell kernel; evaluation may use Contract/Reduce | coefficient dtype, numeric stability, output shape | Cell kernel; expand only when optimization pays |
| p: q: | Primes / factorization | CellApply specialized variable-result kernel | integer domain, dynamic result assembly | Cell kernel |
| u: x: | Unicode/Extended Precision | representation/runtime conversion or Elementwise conversion | encoding/numeric exactness | Value/Rep/Runtime |
| S: | Spread | NestedTraverse(level selector) → CellApply(u) → FlatAssemble | level selector, heterogeneous result/assembly | Direct/Compose |
| c. | Convert to Precision (dyad) | numeric precision conversion | requested precision, logical type/promotion/error | Value/Rep/Cell kernel; 실제 encoding은 downstream |
| m. | Modular arithmetic conjunction | 허용 operand의 modular numeric kernel 생성 | modulus, integer/exact domain, inverse 존재·오류 | Control/Cell kernel |
| F. F.. F.: F: F:. F:: Z: | Fold variants / fold status·termination | recurrence/control region + result collection | direction, valence, state/termination, empty/fill·error order | Control; 일반 Reduce/Scan으로 임의 치환하지 않음 |
| t. T. | Execute as task / thread·task·debug control | task creation, pyx result, synchronization/context effects | capture/namespace, await/open/error point, shared state·resource lifetime | Runtime/control; Taylor 연산이 아니며 fork 병렬 후보와 구별 |
| constant functions | constant broadcast/Generate when array result required | Generate/constant | dtype/shape from call context | Direct/value |
| ~ | Reflex/Passive/Evoke | function semantic transformation/name resolution | valence/binding | Control |
| ^: | Power | Iterate/loop over analyzed basis graph; static count may unroll/fuse | count, fixed-point/inverse semantics, effects | Control; no new array basis |
| $: $:: | Self-Reference / Shorten self-reference scope | recursion 및 scope transformation | 함수 identity, enclosing self-reference 범위, POS | Control; `$::`는 adverb이며 `$:` alias가 아님 |
| : :. :: | Explicit/Monad-Dyad, Obverse, Adverse | function/control/exception semantic graph | POS, inverse/obverse, error semantics | Control |
| =. =: | local/global assignment | binding/effect operation | locale/scope/version/effect ordering | Runtime/control |
| _ _. _: | infinity/indeterminate constants | constant/value semantics | numeric type | Value |
| NB. | Comment | frontend only | none | frontend, no IR op |

**Coverage conclusion v0.2 (정정):** 위 표는 검토한 primitive/form의 설계 분류이며 현재 NuVoc 전체의 parser/runtime/backend 지원 증명이 아니다. task/precision/fold/scope·modifier selection을 추가했지만 모든 항목의 semantic expansion·검증은 미완료다. 새로운 array basis가 필요한지는 각 미지원 family의 실제 계약을 검토한 뒤 판정한다.

<a id="current-j-vocabulary"></a>

<a id="현재-vocabulary-감사--nuvoc와-구현-상태-2026-10-05"></a>
##### 현재 vocabulary 감사 — NuVoc와 구현 상태 (2026-10-05)

현재 NuVoc index와 관련 개별 페이지를 읽고 pinned `ws.c` spelling→code, `t.c` POS/constructor와 Windows C 기본·AVX2의 `4!:0` 결과를 대조했다. 웹 문서의 최신성, 검토 source pin, 실행 DLL release는 별도 provenance다. wiki를 보고 DLL이 최신 source로 빌드되었다고 주장하거나 POS/rank를 추측하지 않는다. NuVoc 개별 문서 `c.`, ObsoleteSyntax, `[.`, `$::`는 웹 fetch가 실패했으므로 index와 C 근거만 사용했다.

- **오래된 설명 수정:** `t.`는 task conjunction, `T.`는 thread/task/debug verb다. task 결과의 pyx/open·오류 전달·공유 namespace 의미를 effect/resource 경계로 다루며 Rust async나 GPU 실행을 이미 지원한다는 뜻이 아니다. [Task](https://code.jsoftware.com/wiki/Vocabulary/tdot), [Threads](https://code.jsoftware.com/wiki/Vocabulary/tcapdot).
- **누락된 현행 표기 추가:** `c.`, `m.`, `f:`, `/..`, `$::`, Fold 여섯 표기와 `Z:`, Lev/Dex/Ident, `{{ }}` direct definition과 `u.`/`v.` caller-context 의미를 inventory에 포함한다. `/..`는 key를 operand dyad의 왼쪽 인자로 제공하므로 `/.` alias가 아니다. [Key](https://code.jsoftware.com/wiki/Vocabulary/slashdot), [Modular](https://code.jsoftware.com/wiki/Vocabulary/mdot), [Fold](https://code.jsoftware.com/wiki/Vocabulary/fcap).
- **역사적 표기 분리:** `d.`/`D.`/`D:`/`t:`/`..`/`.:`/`s:`/`I:`는 현재 C 양 버전의 spelling error를 확인했다. 현행 coverage matrix에서 제거했다. NV3a에서는 fixed spelling의 오류 분류를 일반화해 Rust도 spelling error로 보고한다. 전체 name/numeric grammar는 아래 NV3의 후속 작업이다. `s:`는 NuVoc obsolete 구역의 symbol verb이며, 양 C DLL에서도 거부했다. 내부 Symbol 타입이 존재하는 것과 현행 J의 `s:` 지원은 별개다. `I:` 역시 현행 `I.`/`i:`와 혼동하지 않는다.
- **rank/용어 교정:** `@`는 Atop, `@:`는 At다. 내부 `ConjunctionId::Atco`은 기존 `@:` 식별자이며 주석으로 이를 명시했다. `@`를 동등한 alias로 등록하지 않는다. NuVoc의 동작 설명과 C의 `b.0` intrinsic header/IRS 경로를 구분하고 wiki 표의 rank만 보고 이미 검증한 header를 덮어쓰지 않는다. `u"v`/`m"v` Copy Rank도 별도 form이다. [Copy Rank](https://code.jsoftware.com/wiki/Vocabulary/quotev).

**NV1 코드 변경(이전 단계):** `[.`·`].`는 각각 왼쪽·오른쪽 noun/verb를 반환하는 conjunction이며 `]:`는 operand를 반환하는 adverb다. primitive registry **7**, Graph IR **0.7**에 반영했다. 선택된 함수의 원래 entity/NAME·late lookup과 noun snapshot을 유지한다. 선택 전에 필요한 noun reduction·assignment·error를 생략하지 않는다. 이 constructor는 선택 operand 자체를 결과로 내며 전체 구문/constructor provenance는 source·reduction·capture에서 보존한다. 선택 modifier를 새 배열 kernel로 만들지 않는다. tokenizer state machine은 변경할 필요가 없었다. capture의 `ConstructionNounSuccess.selected_input`으로 noun 선택 결과를 원래 dependency node에 연결하고 선택되지 않은 계산 노드도 유지한다. 아직 ordered-effect graph가 필요한 대입은 기존 경계를 유지한다. 정적 parser에서는 선택되지 않은 noun 계산을 보존할 경로가 없으면 명시적 Unsupported로 남기며, J 의미상의 오류로 바꾸거나 해당 계산을 삭제해 실행 가능하다고 승인하지 않는다.

`tools/vocabulary_audit.py`는 pinned `ws.c`에서 core spelling 후보를 추출해 C POS와 Rust enqueue를 대조한다. 구조 토큰·control words·direct-definition framing·이름 전체를 이 primitive 감사 하나로 검증하지 않는다. **word formation pass / enqueue POS verified / Unsupported coverage / runtime conformance**를 각각 구분한다. 원자료는 `reports/vocabulary-*-windows.json`에 보존한다. `ws.c`의 비영(非零) code만으로 설치된 primitive라고 판단하지 않는다. `w.c`는 `ds(e)`의 permanent usecount도 확인한다. `?:`/`` `. ``는 source code 후보지만 검토한 `t.c`에 설치되지 않고 양 DLL이 거부하므로 현행 valid primitive나 Rust 미지원 pass로 세지 않는다. 이 감사는 numeric grammar의 `__`/`_.` 전체 inventory를 다루지 않는다. 검증 숫자는 아래 NV1/NV2 gate에 기록한다. 표준 `tools/check-frontend-windows.ps1 -Avx2`도 양 DLL별 vocabulary 감사를 실행하므로 이후 frontend 검증 때 함께 갱신한다.

<a id="nv2-core-recognition"></a>

<a id="nv2--core-표기품사-인식과-실행-capability-분리"></a>
##### NV2 — core 표기·품사 인식과 실행 capability 분리

**Core identity 통합:** 모든 140개 core 함수 표기를 PrimitiveId(verb), AdverbId, ConjunctionId의 동일한 품사별 identity catalog에 넣는다. 미구현 여부로 별도의 VocabularyPrimitive 타입·semantic/head variant를 만들지 않는다. 35개 기존 구현 항목은 PrimitiveSupport::Bounded, 아직 미구현인 105개는 PrimitiveSupport::Unimplemented 태그를 가진다. Bounded도 valence/type/rank/route별 기존 제한을 유지하며 완전 지원을 뜻하지 않는다. ExtensionPrimitive/extension registry는 core 밖의 ordinary NAME 확장만 별도로 관리한다. 이름·품사와 지원 상태는 독립이고, 미구현 verb의 innate rank는 None이다. 기존 ID 순서는 유지하고 미구현 항목을 추가하며 상수 동사는 표기별 Fcons0…Fcons9/FconsNeg1…FconsNeg9/FconsInf/FconsNegInf ID와 공통 CFCONS 이름 metadata를 가진다. 모든 core ID가 동일한 enqueue payload와 FunctionHead 품사 경로를 따른다. identity 구조 변경에 따라 registry **11**, Graph IR **0.11**로 갱신해 이전 graph를 거부한다. 새 실행 의미론을 추가하거나 미확인 rank/effect/purity/resource proof를 부여하지 않는다.

Unimplemented 항목의 rank/shape/dtype/effect/resource 계약은 미확인 상태로 둔다. `innate_ranks()`는 None, graph rule은 DynamicOrUnknown, 실행 lowering/executor 및 Unimplemented primitive 자체의 modifier 적용은 Unsupported다. 기존 rank/selector/train이 검증된 구조를 만들 수 있으면 identity를 보존하되 새 kernel/purity/finite rank를 만들지 않는다. RHS Copy Rank에 미확인 core 항목이 들어가면 header를 추측하지 않는다. primitive modifier의 by-value stacking은 검토 `t.c`의 VF2NAMELESS 근거이며 purity와 별개다. ordinary verb alias의 late binding은 그대로 유지한다.

`a.`는 byte 0–255의 char vector, `a:`는 empty bool vector를 담는 scalar box인 실제 noun이다. `OnceLock<Value>`에 immutable shared payload를 보존해 enqueue occurrence가 버퍼를 복사하지 않는다. 이 두 bounded builtin을 추가한 것이 대형 input/intermediate noun snapshot cache를 도입한 것은 아니다. 둘 다 기존 logical Value 경로로 처리하며 device/layout를 noun 의미에 넣지 않는다.

`tools/vocabulary_audit.py`는 양 C DLL의 bare binding `4!:0`/`5!:1`과 Rust parser projection을 140개 함수에서 비교하고 세 noun의 type/shape/payload도 확인한다. 이 단계에서 gerund의 `!`를 실제 함수 identity로 decode한 뒤 Prefix entity를 구성하는 것은 가능해졌다. 그 함수의 실행은 여전히 Unsupported이므로 기존 회귀 테스트를 construction 성공 / 실행 capability 경계로 교정하고 공통 C fixture를 추가했다. core spelling 인식과 full modifier/control/name semantics는 별개이며 NV3–NV5를 계속 따른다.

근거: [ws.c spelling codes](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/ws.c), [t.c permanent nouns·POS·VF2NAMELESS](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/t.c#L81), [w.c installed primitive check](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/w.c#L140). source pin과 실행 DLL provenance의 분리는 유지한다.

**jsource 특수 경로 누락 재감사:** 아래 basis 대응 외에 확인된 convolution (\`f//.@:(g/)\`), Cut–Scan–Raze, byte LUT, boolean-index, order-statistic, shape-RNG, boxed link, explicit \`M.\`, numeric exactness, hook comparisons는 [§4.1.3.6 M](./14-framework-research-p02.ko.md#jsource-audit-m)의 source-guard/ownership 표를 기준으로 한다. 이 목록을 새로운 basis IR node의 필연적 추가로 해석하지 않는다.

<a id="jsource-special-code-inventory와의-교차검증"></a>
##### jsource special-code inventory와의 교차검증

| jsource special family | RustJ logical explanation |
|---|---|
| +/ .* | Contract(combine=*, aggregate=+) |
| $, 및 ravel-avoidance | RegularReindex/Reshape가 ravel materialization을 만들지 않음 |
| f;.n | SegmentView 또는 WindowView + CellApply(f) |
| f/;.n, +//., #/. | segment/group descriptors + Reduce/GroupBy; group cells를 만들지 않는 route |
| /:, /:~ | Grade |
| {/: | Grade → Gather의 fused candidate |
| +/\, =/\, f/\. | Scan 또는 ordered segment/window reduction |
| { | Gather |
| } / indexed amend phrase | Scatter/Amend |
| f"r | CellApply absorption/fusion candidate |
| i., i:, e. | Lookup/Classify |
| E. | WindowView + Match |
| +/%# mean family | Reduce(Add) + Tally + Divide, candidate fusion |

jsource가 phrase-by-phrase special code로 얻는 여러 이득을 RustJ에서는 **basis graph + capability/lowering rule**로 일반화할 수 있다.

<a id="42413-derived-forms-closure-check-실제-execution-basis-graph"></a>
#### 4.24.13 Derived forms closure check: 실제 execution-basis graph

primitive 한 개만 매핑해서는 J semantics coverage를 검증할 수 없다. modifier/derived form이 만들어 내는 implicit traversal과 control을 실제 basis graph로 내려 보아야 한다.

<a id="rank"></a>
##### Rank

~~~text
u"r y
  ↓ ResolveRank(r, rank(y))
Cell/Frame partition
  ↓
Frame IndexSpace
  ↓
CellApply(u)
  ↓
J result Assemble
~~~

dyad:

~~~text
x u"(lr,rr) y
   ↓                         ↓
Resolve left cells      Resolve right cells
          \              /
           FrameAgreement
                 ↓
          SemanticCheck
        (length/error order)
                 ↓
          Frame IndexSpace
                 ↓
             CellApply(u)
                 ↓
              Assemble
~~~

CellApply(Elementwise), CellApply(Reduce), CellApply(Contract)는 각각 larger elementwise, batched/segmented reduction, batched contraction route로 흡수할 수 있다. semantic CellApply 자체는 optimization 전에도 보존한다.

<a id="cut"></a>
##### Cut

~~~text
fret/mask
   ↓
SegmentDescriptor
   ↓
SegmentView(y)
   ↓
CellApply(u)
   ↓
Assemble
~~~

~~~text
window/tessellation spec
   ↓
WindowView(y)
   ↓
CellApply(u)
   ↓
Assemble
~~~

Cut 때문에 새 basis가 필요하지 않으며, 오히려 SegmentView와 WindowView를 구분할 근거가 된다.

<a id="key--oblique"></a>
##### Key / Oblique

~~~text
x
 ↓
Classify / GroupBy
 ↓
group descriptors ───── y
          \             /
           SegmentView
                ↓
            CellApply(u)
                ↓
             Assemble
~~~

u가 associative reduction으로 분석되면:

~~~text
GroupBy
   ↓
GroupedReduce / ScatterCombine
~~~

group array를 실제로 materialize하지 않는 것이 주요 성능 이점이다.

<a id="power"></a>
##### Power

~~~text
u^:n y
   ↓
Iterate {
  body = analyzed basis graph of u
  count = n
}
~~~

static small n에서는 unroll/cross-iteration fusion 후보가 될 수 있고 dynamic count는 loop로 남는다. fixed-point/inverse/boxed power의 J semantics는 control layer가 소유하며 새로운 array basis를 요구하지 않는다.

<a id="boxed-level--spread--fetch"></a>
##### Boxed Level / Spread / Fetch

~~~text
u L:n y
  ↓
NestedTraverse(level=n)
  ↓
CellApply(u)
  ↓
NestedReconstruct
~~~

~~~text
u S:n y
  ↓
NestedTraverse(level=n)
  ↓
CellApply(u)
  ↓
FlatAssemble
~~~

~~~text
x {:: y
  ↓
path-guided NestedTraverse/Gather
  ↓
selected nested value
~~~

NestedReconstruct와 FlatAssemble는 output assembly policy이고 NestedTraverse가 traversal identity를 보존한다.

<a id="sparse-composition"></a>
##### Sparse composition

Sparse는 basis를 복제하지 않는다.

~~~text
Reduce(+)
+ RepresentationFacts::Sparse
      ↓
sparse-aware lowering
~~~

~~~text
RegularReindex(Transpose)
+ RepresentationFacts::Sparse
      ↓
sparse index/value permutation
~~~

~~~text
CellApply(u)
+ RepresentationFacts::Sparse
      ↓
sparse cell route or conforming fallback
~~~

따라서 SparseReduce, SparseTranspose, SparseCellApply 같은 별도 semantic basis family를 만들지 않는다. sparse axes/fill/index/value는 representation facts이며 같은 J observable semantics를 다른 physical realization으로 실행한다.

<a id="closure-판정"></a>
##### closure 판정

Rank, Cut, Key/Oblique, Power, boxed Level/Spread/Fetch, sparse composition을 실제 graph로 내린 결과:

1. v0.2에 없는 새 array-computation basis family는 발견되지 않았다.
2. CellApply, WindowView, SegmentView, GroupBy/Classify, NestedTraverse의 필요성은 오히려 강화되었다.
3. StateMachine만은 다른 array compiler와의 대응 근거가 약하므로 **provisional structured kernel**로 유지한다.
4. basis를 더 작은 operation으로 refinement하는 작업은 이 closure의 선행조건이 아니다. 이후 benchmark/optimizer 구현에서 실질적 성능 이득이 확인되는 경우에만 ExecutionBasisExpansion rule을 추가한다.




<a id="42414-execution-basis-logical-ir-node-contract와-최소-analysislowering-interface"></a>
#### 4.24.14 Execution-basis Logical IR node contract와 최소 analysis/lowering interface

4.24.11–4.24.13의 v0.2 vocabulary를 실제 A3 Logical IR로 옮길 때 basis 이름마다 임의의 struct를 따로 만들지 않는다. 모든 basis node는 공통 contract를 공유하고, 각 operation family가 필요한 추가 payload/fact/check를 명시한다.

핵심 경계:

~~~text
J Semantic entity / Resolved semantic op
        │
        ├─ original semantic identity
        ├─ ResolvedInstantiation
        └─ ResolvedCallFacts
                 │
                 ▼
        optional ExecutionBasisExpansion
                 │
                 ▼
           Logical basis graph
                 │
                 ├─ ValueFacts / ValueRoleFacts
                 ├─ IterationDomain / AxisSemantics
                 ├─ AccessFact
                 ├─ ConstraintSet / FactWitness
                 ├─ SemanticCheck
                 ├─ EffectSummary / SpeculationSemantics
                 ├─ DestinationRelation / StorageRequirement
                 └─ provenance
                         │
                         ▼
                LoweringRegistry query
                         │
                         ▼
              legal realization candidates
~~~

basis op는 target-independent다. warp width, SIMD width, tile, workgroup, shared memory, register budget, concrete layout, library handle 같은 것은 basis node payload에 넣지 않는다.

<a id="resolvedinstantiation"></a>
##### ResolvedInstantiation

polymorphic primitive/derived verb가 실제 call에서 어떤 instance가 되었는지를 ResolvedCallFacts와 specialization 사이에 명시적으로 기록한다.

~~~text
ResolvedInstantiation
  semantic_operation_id
  valence
  input dtype/rank/cell-rank instances
  output dtype/rank instance if known
  explicit/innate rank boundary
  relevant ValueRoleFacts
  numeric/fit policy identity
  source/binding provenance
~~~

이 구조는 semantic identity 자체가 아니다. 동일 FunctionEntity도 actual arguments에 따라 여러 instantiation을 가질 수 있다.

용도:
- IR dump/diagnostics
- differential test provenance
- SpecializationKey material 추출
- lowering lookup
- cache provenance

SpecializationKey에는 이 전체를 복사하지 않고 algorithm/lowering 선택에 실제 영향을 주는 field만 추출한다.

<a id="valuerolefacts"></a>
##### ValueRoleFacts

ValueFacts.shape는 “이 value 자체의 shape”다. 반대로 어떤 noun의 값이 다른 operation의 shape/axis/index/window specification으로 사용된다는 사실은 별도 role이다.

~~~text
ValueRoleFacts
  ShapeVector
  AxisVector
  AxisPermutation
  RankSpecifier
  IndexVector
  CountVector
  WindowSpec
  StrideSpec
  DilationSpec
  PaddingSpec
  SegmentDescriptor
  Permutation
  StateIdLike
  Unknown
~~~

ValueRoleFacts는 J type system에 새로운 nominal type을 추가하지 않는다. 같은 integer vector가 문맥에 따라 ordinary data 또는 ShapeVector가 될 수 있다. role은 call/basis analysis가 만든 fact이며 witness를 가질 수 있다.

<a id="first-class-semanticcheck"></a>
##### first-class SemanticCheck

J-visible error condition을 compiler assertion과 구분한다.

~~~text
SemanticCheck
  predicate / required constraint
  J error kind
  semantic origin
  observable ordering dependency
  witness if already proven
~~~

예:
- shape agreement → length error
- index range → index error
- domain predicate → domain error
- permutation validity → index/domain error as defined by source primitive
- solver shape/singularity condition → corresponding J error contract

규칙:
1. compiler 내부 invariant assertion과 SemanticCheck를 같은 것으로 취급하지 않는다.
2. FactWitness가 predicate를 증명하면 check를 제거할 수 있다.
3. hoist/fuse/reorder는 SpeculationSemantics와 observable error order를 보존할 때만 허용한다.
4. A3-v0 single-block IR에서도 check는 MayRaise operation으로 존재할 수 있다. runtime branch/deoptimization이 필요한 Guard는 A3-v1의 책임이다.

현재 transition 구현의 src/analysis.rs::Node는 아직 “node 하나 = ValueId 하나”인 inspection snapshot이다. 따라서 이 구조에 zero-result SemanticCheck를 가짜 value-producing node로 억지로 넣지 않는다. 현재 단계에서는 direct ExecutionBasisKind, ResolvedInstantiation, ValueRoleFacts seam만 추가하고, first-class SemanticCheck는 A3의 operation/result 분리에서 zero-result operation으로 구현한다. 이것은 SemanticCheck를 후순위 의미로 낮추는 것이 아니라 잘못된 migration representation을 만들지 않기 위한 단계화다.

