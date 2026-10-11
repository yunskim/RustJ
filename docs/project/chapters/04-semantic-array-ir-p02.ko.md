[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)

> 문서 분할 안내: 기존 `PROJECT.ko.md`의 내용을 이동한 모듈입니다. 원문의 단일 파일 정본·신규 Markdown 금지 문구는 최신 [Agent 문서 규약](../../../AGENTS.md)보다 우선하지 않습니다.

# 3. J Semantic Array IR — 계속 (2/4)

#### 3.3.4 Function DAG의 shape는 J parser reduction rule이 결정한다

RustJ가 derived function의 DAG shape를 별도 IR 취향으로 새로 정의하지 않는다. **current jsource의 parser reduction table(`p.c::cases[]`)을 semantic DAG construction의 기준으로 삼는다.**

현재 jsource parse table에서 function construction에 직접 대응하는 핵심 row는 다음과 같다.

```text
row 3
  (VERB | NOUN)  ADV
    → adverb application

row 4
  (VERB | NOUN)  CONJ  (VERB | NOUN)
    → conjunction application

row 5
  (VERB | NOUN)  VERB  VERB
    → fork

row 6
  CAVN  CAVN
    → hook / bident
```

반면 row 0–2는 monad/dyad execution, row 7은 assignment, row 8은 parenthesis reduction이다.

따라서 semantic DAG에서 **modifier vocabulary별 node kind를 새로 발명하지 않는다.** source operator의 품사와 parser production이 parent/operand 관계를 결정한다.

예를 들어 insert는:

```text
source:
  + /

parser row 3:
  left operand = +
  operator     = /

Semantic Function DAG:

/ : Verb                 // applied adverb result
└─ + : Verb
```

이다.

즉 `Insert(+)`라는 별도 semantic node를 만들지 않는다. `/` 자체가 ADV operator identity이며, operand를 가진 derived result의 parent가 된다.

rank도 동일하다.

```text
source:
  u " r

parser row 4:
  left operand  = u
  operator      = "
  right operand = r

Semantic Function DAG:

" : Verb                 // applied conjunction result
├─ left:  u
└─ right: r
```

따라서 `Rank(u,r)` 같은 unary-like special node를 만들지 않는다. **`"` conjunction이 parent이고 양쪽 parser operand를 그대로 가진다.**

hook/fork는 source operator token이 따로 없으므로 parser production 자체가 parent identity다.

```text
Hook
├─ f
└─ g

Fork
├─ f
├─ g
└─ h
```

긴 train도 별도 `LongTrain` node를 만들지 않는다. jsource parser가 row 5/6 reduction을 반복해 hook/fork derived function을 만드는 것처럼 shared Hook/Fork node graph로 축약한다.

#### 3.3.5 jsource `V.fgh`는 parse DAG의 oracle이 아니라 realization cross-check다

jsource의 `V` block은 VERB/ADV/CONJ이 공유하는 execution/function representation이고 `fgh[3]`, valence entry points, rank, id, flags/local metadata를 가진다. 이는 RustJ가 **공통 shared function object**를 쓰고 subtree를 deep-copy하지 않아야 한다는 좋은 reference다.

그러나 `fgh[3]`의 모든 slot을 곧바로 source semantic DAG child로 복제하지 않는다.

이유는 일부 derived form에서 jsource가 parser operand 외에 **실행 최적화용 보조 function/object**도 `f/g/h` 또는 local metadata에 넣기 때문이다.

예:

- insert/reduction의 보조 rank/execution object
- under의 inverse/optimized execution auxiliary
- precomputed tables/hash/info 같은 realization metadata

따라서 authoritative 관계는:

```text
source words
  ↓
jsource parse reduction rule
  ↓
RustJ Semantic Function DAG       // semantic operands only
  ↓
Semantic Analyzer
  ↓
Logical IR
  ↓
runtime/backend specialization
```

이고, `V.fgh`는 **각 parse reduction 결과가 jsource에서 어떤 function object로 realization되는지 검증하는 자료**로 사용한다.

RustJ의 현재 `FunctionEntity` 원칙:

```text
FunctionEntity
  result_part_of_speech
  head:
    PrimitiveVerb
    PrimitiveAdverb
    PrimitiveConjunction
    NameRef
    Hook
    Fork
  operands:
    shared FunctionEntity ref
    semantic Noun operand
  source/provenance
```

같은 operator identity라도 operand가 없고 result POS가 ADV/CONJ이면 source operator entity이고, parser application 뒤에는 같은 operator identity가 operand를 가지며 result POS가 derived result의 품사가 된다.

현재 구현은 이 parser table의 **부분집합**만 지원한다. 따라서 지원 여부를 production 단위로 관리한다.

```text
ParseConstructionCoverage (현재)
  row 3: VERB ADV                  implemented
         NOUN ADV                  pending

  row 4: VERB CONJ NOUN            implemented for supported literal noun
         VERB CONJ VERB            structurally representable; lowering partial
         NOUN CONJ (VERB|NOUN)     pending

  row 5/6:
         edge-bounded pure verb phrase hook/fork
                                      implemented
         full mixed-sentence table   pending
```

미지원 production을 독자적인 다른 DAG로 대신 해석하지 않는다. 해당 production을 구현할 때 jsource parse rule과 result-POS rule을 추가한다.

예:

```text
/ : Adverb
  operands = []

/ : Verb
  operands = [+]
```

```text
" : Conjunction
  operands = []

" : Verb
  operands = [u, r]
```

이 구조는 modifier 수가 늘어도 semantic core에 `Insert/Rank/Power/Under/Fit/...` 식의 거대한 closed enum을 추가하지 않는다. 새 built-in/extension operator는 자신의 identity/POS/semantic contract를 registry에 추가하고, **DAG application shape는 기존 J parser production을 따른다.**

큰 derived expression은 immutable shared handle/`EntityId` DAG로 표현하여 assignment, alias, analysis plan 사이에서 subtree를 복사하지 않는다.

#### 3.3.6 modifier reduction 결과는 다음 parser reduction에서 하나의 function entity다

current jsource의 parser는 이 경계를 명확하게 가진다. `p.c`의 parser lines 3-4는 ADV/CONJ를 실제로 적용해 결과 `yy` function object를 만든 뒤 그 결과의 parsing type을 stack에 다시 기록한다. 따라서 이후 hook/fork/train reduction은 원래 modifier token과 operand token을 다시 보지 않고 **이미 만들어진 결과 entity 하나**를 operand로 받는다.

예를 들어:

```text
source words
  +  /  %  #

parser modifier reduction
  +  /
   ↓
  +/ : VERB entity E2

remaining function phrase
  E2  %  #

fork reduction
  ↓
  Fork(E2, %, #)
```

jsource에서 `ar.c::jtslash`는 실제로 새 function block `z`를 할당하고:

```text
id      = CSLASH
AT/type = VERB
fgh[0]  = original operand verb u
```

를 설치한다. `+/ `의 경우 `fgh[0]`은 primitive `+` object다. `fgh[2]`에는 dyadic execution을 위한 precomputed rank compound가 들어가지만, 이것은 parser-derived semantic operand가 아니라 execution auxiliary이므로 RustJ Semantic IR child로 복제하지 않는다.

그 뒤 `cf.c::jtfolk`는 `f`, `g`, `h`를 이미 완성된 J values/function objects로 받아 새 `CFORK/VERB` function block을 만들고 그대로 `fgh[0..2]`에 저장한다. `(+/%#)` specialization도 다음처럼 **f가 이미 CSLASH verb라는 전제**로 검사한다.

```text
f.id == CSLASH
g.id == CDIV
h.id == CPOUND
f.fgh[0].id == CPLUS
```

따라서 RustJ의 parser/semantic invariant는 다음으로 고정한다.

```text
ModifierApplicationReduction
  operands + modifier
      ↓
  new FunctionEntity(result_pos = ...)
      ↓
  one parser item / one J entity

Hook/Fork reduction
  operands = EntityRef to already-completed entities
```

즉 Fork 내부에 `AdverbApplication(/,+)`라는 아직 미완성 parser fragment가 들어가는 것이 아니라, **`+/`라는 완성된 derived Verb entity에 대한 reference가 들어간다.**

RustJ의 `FunctionEntity`는 이 점에서는 jsource `V` function block의 semantic subset을 직접 따른다.

```text
jsource V/function block            RustJ FunctionEntity
-------------------------           --------------------
AT = VERB/ADV/CONJ                  result_pos
id = CSLASH/CFORK/...               head / construction identity
semantic f/g/h operand              EntityRef operands
execution function pointer          제외: lowering/runtime registry
localuse/cache/rank helper          제외 또는 별도 analysis/lowering metadata
runtime memory flags                제외: downstream proof/physical state
```

이 구조에서:

```text
E1 = + : primitive Verb

E2 = +/ : derived Verb
     head = Insert(/)
     operands = [E1]

E3 = % : primitive Verb
E4 = # : primitive Verb

E5 = Fork : derived Verb
     operands = [E2, E3, E4]
```

가 canonical semantic representation이다.

이 원칙은 모든 modifier에 동일하게 적용한다. `u"r`, `u&v`, `u@v`, `u!.n` 등이 parser production을 완료하면 그 결과는 독립된 J function entity이고, 이후 train/modifier application은 그 entity reference를 operand로 사용한다. execution-only helper를 semantic operand로 승격시키지는 않는다.

semantic function identity/operands와 execution specialization도 분리한다. jsource가 동일한 parser-derived function object에 specialized executor를 선택할 수 있듯 RustJ도:

```text
Semantic FunctionEntity
  parser-derived identity + operands + J semantics

        ↓ analyze/lower

Logical operation / lowering candidate
  generic implementation
  optimized native implementation
  external IR lowering
  architecture-specific specialization
```

로 둔다.


### 3.4 너무 이른 정규화를 금지한다

다음 변환은 **semantic analysis 전에 무조건 수행하지 않는다.**

```text
Fork(f,g,h)
    → g(f(y), h(y))

Hook(f,g)
    → fully expanded expression

AdverbApplication(operator=/, operand=+)
    → Reduce(Add)

ConjunctionApplication(operator=", left_entity, right_entity)
    → logical CellApply after rank resolution
      (uniform result proof 전에는 fixed-shape MapCells로 축소하지 않음)
```

이런 정규화는 합법성과 분석 이득이 확인된 뒤 semantic lowering/rewrite 단계에서 수행한다.

이유:

- fork branch의 독립성/observable-order 판단 가능성
- 공통 argument 사용
- derived verb identity
- primitive composition
- rank propagation
- fusion 후보
- custom semantic annotation
- name/binding semantics

을 분석에 사용할 수 있기 때문이다.

#### 3.4.1 train dataflow와 observable execution order를 분리한다

current jsource의 fork/hook 실행(`j.h`의 `FORK1/FORK2`)은 일반 fork에서 **오른쪽 tine `h`를 먼저 실행하고, 그 다음 왼쪽 tine `f`, 마지막에 middle verb `g`**를 실행한다. hook/composition 계열도 해당 derived-verb 의미에 따른 실행 순서를 가진다.

pure computation만 보면:

```text
h(y) ─┐
      g
f(y) ─┘
```

라는 dataflow가 보이지만 이것만으로 `f`와 `h`가 semantic하게 병렬 독립이라는 뜻은 아니다.

다음이 있으면 observable-order edge를 보존한다.

- name/locale lookup 또는 mutation
- I/O / system foreign
- mutable `StateResource` effect
- catch 가능한 J error / throw
- runtime semantic context mutation
- 그 밖의 non-pure / non-speculatable behavior

개념적으로:

```text
ValueDependency
  h_result -> g.right
  f_result -> g.left

ObservableOrder
  h -> f -> g       // effectful/general fork compatibility order
```

두 tine이 pure하고, relevant late-bound name/effect/error dependency가 없으며, `SpeculationSemantics`가 허용한다고 증명된 경우에는 optimizer가 order edge를 제거하여 병렬 실행/fusion할 수 있다.

즉 source-level J 실행 순서를 physical serial schedule로 영구 고정하지는 않지만, **dataflow graph만 보고 순서 제약을 자동 폐기하지 않는다.**

#### 3.4.2 derived verb의 latent semantics를 forward dataflow로 소거하지 않는다

일부 J conjunction/modifier는 현재 forward call의 값 계산만 보면 없어 보여도 **향후 derived behavior**에 의미가 있다.

대표적인 예:

- `u :: v` adverse: `u`의 일반 오류를 잡아 `v`를 실행하지만 `throw.`/exit류는 같은 방식으로 잡지 않는다.
- `u :. v` obverse: forward 실행은 주로 `u`를 사용하지만 inverse 계산에서 `v`가 semantic하게 사용된다.
- rank/power/agenda/under 등도 operand entity와 derived metadata가 이후 변환·실행 의미에 영향을 줄 수 있다.

따라서:

```text
DerivedVerb
  forward semantics
  latent modifier semantics
  operand identities / NameRefs
  error/inverse/control contracts
```

를 필요한 범위에서 보존한다.

`u :. v`를 forward graph가 `u`와 같다는 이유로 그냥 `u`로 canonicalize하면 향후 inverse 의미가 달라진다. `u :: v`를 단순 `u` + unreachable fallback으로 취급하면 error behavior가 달라진다.

Logical lowering이 derived structure를 소거할 수 있는 것은 **그 latent semantics가 이후 프로그램에서 관찰되지 않거나 동등하게 다른 contract로 이전되었다는 proof**가 있을 때뿐이다.

### 3.5 물리 정보는 넣지 않는다

고수준 semantic IR에 다음 physical decision은 넣지 않는다.

```text
CudaBlockSize
Tile128
GpuSharedMemory
PhysicalStride
PhysicalOffset
DeviceTransfer
ConcreteBufferId
StreamEvent
```

이들은 downstream scheduling/bufferization/backend의 책임이다. RustJ-native route에서는 Physical Planner가 담당하고, external route에서는 해당 compiler의 lower-level IR/pass가 담당할 수 있다.

### 3.6 현재 구현과 목표 경계의 차이

현재 모듈 책임은 [§10 M0 ownership 표](./10-implementation-scope.ko.md#architecture-migration-checklist), 구현 상태는 [§12](./12-validation.ko.md#current-implementation-status)에 모아 둔다. compiler의 연결된 분석 경로는 `FunctionEntity → j_graph_ir::Plan → analysis::lower_graph → logical_ir::Plan`이다. `analysis.rs`는 FunctionEntity를 직접 canonicalize하는 분석기가 아니라 J Graph의 execution-semantic lowering을 담당한다.

현재 `semantic.rs`는 `/` ADV와 `"` CONJ의 identity/POS 및 지원 subset의 parser construction graph를 보존한다. 전체 jsource-compatible Enqueue/9-row cutover는 M2에서 추적한다. `Callable.reduce/rank`와 runtime `ResolvedVerb`의 flattening은 남은 migration 표현이며 canonical function identity가 아니다.

### 3.7 name resolution과 explicit definition의 호출 의미

과거 `jaxa-analyzer`의 jsource 조사에서 확인한 name/local-frame 의미를 현행 IR에서도 보존한다.

핵심 분리:

```text
DefinitionCode
  body / valence / local-layout hints / source

Invocation
  fresh CallFrame
  x/y/u/v/m/n bindings
  runtime local values
  current locale context
```

동일 definition의 재귀 호출은 code를 공유할 수 있지만 **local value frame은 호출마다 독립**이어야 한다.

J의 simple name lookup은 Python의 정적 LOCAL/GLOBAL 이분법과 다르다.

```text
simple name lookup:
  current invocation local table
    ↓ if no bound value
  current locale
    ↓
  locale path
    ↓
  value error
```

위 흐름은 **J semantic target**이다. 현재 runtime definition subset은 `LocalFrame`의 current-frame lookup 후 global namespace fallback까지 구현되어 있고, 일반 user locale/path 해석은 아직 미완료다. compiler-side `execution_semantics::Scope`도 `LocalFrame(u32)` vocabulary를 갖지만 현재 A3 symbol emission은 주로 `CurrentGlobal`에 머문다. 따라서 목표 locale semantics와 현재 implementation coverage를 같은 것으로 읽지 않는다.

definition-time에 local slot/search hint가 존재한다는 사실은 그 name이 항상 local binding이라는 뜻이 아니다. slot이 아직 unbound이면 locale lookup으로 fallback할 수 있다.

assignment도 구분한다.

```text
=.   current invocation/local semantics
=:   locale/public assignment semantics
```

두 copula 모두 namespace write effect만 있는 void statement로 모델링하지 않는다. J sentence 안에서 assignment는 noun/verb/adverb/conjunction 등 **assigned J entity를 결과로 남겨 다른 표현과 결합될 수 있다.**

```text
AssignmentExpr
  target
  kind: Local (=.) | Public (=:)
  rhs: JEntity
  effect: namespace write
  result: assigned JEntity
```

따라서 assignment lowering은:

```text
entity = evaluate/derive RHS
write binding(target, entity)
return entity
```

라는 value+effect 의미를 함께 보존한다. physical in-place assignment optimization은 이 semantic result를 바꾸면 안 된다.

direct/explicit definition의 structured control flow는 장기 Logical IR의 Region/Block으로 낮출 수 있지만, 이 lowering 때문에 다음 의미를 잃으면 안 된다.

- monad/dyad body 구분
- `x y u v m n`의 valence/definition-kind별 binding
- 호출별 local frame 독립성
- local-first + locale fallback lookup
- direct/indirect locative semantics
- recursive call의 독립 frame
- source/control-flow diagnostics

따라서 SSA `ValueId`는 J namespace 자체의 대체물이 아니다. static binding이 증명된 경우에는 SSA value로 낮출 수 있지만, runtime name lookup/assignment semantics가 필요한 곳은 명시적인 name/resource/effect operation 또는 runtime lowering으로 보존한다.


---

[← 이전 부분](./04-semantic-array-ir-p01.ko.md) | [다음 부분 →](./04-semantic-array-ir-p03.ko.md)
