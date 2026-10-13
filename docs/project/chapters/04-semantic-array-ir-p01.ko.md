[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)


<a id="read-semantic"></a>
<a id="3-j-semantic-array-ir"></a>
## 3. J Semantic Array IR

<a id="31-j의-계산-모델"></a>
### 3.1 J의 계산 모델

J의 계산은 배열 중심이다.

- noun은 array value다.
- monadic verb는 개념적으로 `Array → Array`다.
- dyadic verb는 개념적으로 `Array × Array → Array`다.
- adverb와 conjunction은 verb 등 J entity를 받아 새로운 derived entity를 만든다.
- hook/fork/train은 verb를 합성하여 새로운 verb를 만든다.

따라서 compiler가 보존해야 하는 것은 단순한 배열 값뿐 아니라 **array transformer의 합성 구조**다.

예를 들어:

```j
(+/ % #) y
```

를 semantic analysis 전에 곧바로

```text
Divide(Reduce(Add, y), Tally(y))
```

로만 바꾸면 최종 데이터 의존성은 남지만, 원래의 fork 구조가 가진 분석 정보를 잃는다.

고수준 IR에서는 개념적으로 다음처럼 보존할 수 있다.

```text
Apply
├─ verb:
│   Fork
│   ├─ / : Verb
│   │   └─ + : Verb
│   ├─ % : Verb
│   └─ # : Verb
└─ argument:
    y
```

Semantic Analyzer가 이 구조를 분석한 뒤에야 explicit dataflow로 낮춘다.

<a id="32-jsource에서-확인한-근거"></a>
### 3.2 jsource에서 확인한 근거

확인 기준: `jsoftware/jsource` master `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528` (2026-09-30 확인), 특히 `jsrc/jtype.h`, `jsrc/cf.c`, `jsrc/jc.h`.

현재 jsource의 내부 구현도 derived verb의 구조를 실행 전까지 보존한다.

`jsrc/jtype.h`의 핵심 구조:

```c
typedef struct AD AD;
typedef AD *A;
```

noun, verb, adverb, conjunction 등 J entity는 공통 `A` block 체계에 존재하고 type bit로 품사를 구별한다.

```text
NOUN = numeric + character + box
FUNC = VERB + ADV + CONJ
```

verb/adverb/conjunction의 payload는 `FAV(x)`로 `V` 구조를 읽는다. `V`에는 개념적으로 다음이 있다.

```text
V
├─ fgh[3]          operands / components
├─ valencefns[2]   monad / dyad entry
├─ monad/dyad rank
├─ id
└─ optimization / execution metadata
```

jsource 주석은 `fgh[3]`의 `h`가 fork에 사용된다고 명시한다. `jtfolk`는 “derived verb for a fork”를 만들고 `jthook`도 hook/trident 구조를 derived verb로 만든다. `CFORK`, `CHOOK`, `CADVF` 같은 identity도 유지된다.

또한 function block의 `AN`, `AR` field는 사용하지 않는다고 명시되어 있다. 즉 **noun array와 verb는 같은 J entity allocation 체계를 공유하지만 동일한 noun representation은 아니다.** RustJ는 이 물리 표현을 복제할 필요는 없지만, **derived verb composition을 first-class semantic structure로 보존한다는 점은 중요한 reference**다.

<a id="33-ir이-보존해야-하는-구조"></a>
### 3.3 IR이 보존해야 하는 구조

jsource와 다시 대조한 결과, RustJ의 공통 entity abstraction은 **jsource의 모든 `A` block type**을 복제하는 것이 아니라 parser/evaluator가 실제 semantic RHS로 다루는 `RHS = NOUN + FUNC`, `FUNC = VERB + ADV + CONJ`에 대응시키는 것이 가장 정확하다.

jsource의 `AD/A`는 noun과 function뿐 아니라 NAME/SYMB 등 runtime block에도 쓰이는 물리적 공통 allocation handle이다. RustJ의 `JEntity`는 이 물리 universe 전체가 아니라 **J expression이 산출·전달하는 semantic entity**만 모델링한다.

개념 모델:

```text
JEntity                         // semantic RHS value
├─ Noun
│   └─ Logical J noun value
└─ Function
    └─ FunctionEntity
        ├─ result POS: Verb | Adverb | Conjunction
        ├─ primitive / explicit / derived identity
        ├─ Hook / Fork / modifier-application construction
        └─ semantic operands: JEntityRef...

separate reference/control layer
├─ lexical NAME / parser control class
├─ NameRef / late-binding reference
├─ binding + version
└─ source/provenance
```

중요한 점은 **NameRef가 noun/verb/adverb/conjunction과 같은 다섯 번째 J entity kind가 아니라는 것**이다. lexical NAME은 lookup 전 parser item이고, function nameref는 binding/lookup 시점을 보존하는 reference representation이다. resolve된 결과가 `JEntity`다. 현재 `FunctionHead::NameRef` 같은 표현은 function identity 내부에서 이 indirection을 보존하는 구현일 수 있지만, 상위 POS universe 자체를 늘리지는 않는다.

또한 jsource의 공통 `AD` header에 `AN/AR/shape` field가 존재한다고 해서 function에 J-array rank/shape semantics가 있다는 뜻은 아니다. current `jtype.h`는 **function의 AN/AR fields를 사용하지 않는다고 명시**한다. RustJ도 `Verb`/`Adverb`/`Conjunction` 자체에 noun-style semantic shape/rank를 부여하지 않는다.

따라서 invariants는 다음과 같다.

1. `JEntity = Noun | Function(POS=Verb|Adverb|Conjunction)`을 semantic RHS universe의 기본 분리로 본다.
2. primitive와 **derived function entity(verb/adverb/conjunction)**의 identity와 result POS를 보존한다.
3. hook/fork/train 및 modifier application의 semantic operand 관계를 보존한다.
4. modifier/bident/trident parser action의 결과를 항상 Function이라고 가정하지 않는다. jsource constructor table에는 즉시 실행되어 NOUN을 만드는 production도 있으므로 parser action 결과의 일반형은 `JEntity`다.
5. monad/dyad valence와 rank contract를 function semantics에 보존하되, 이것은 function object의 array rank/shape가 아니다.
6. rank conjunction의 left/right operand가 verb/noun/gerund/verb-rank form 중 무엇인지 분석 전에 보존한다.
7. name lookup과 binding/version의 시점 차이를 reference layer에서 보존한다. 특히 noun value snapshot과 function nameref late lookup을 같은 규칙으로 뭉개지 않는다.
8. source span/provenance는 entity identity와 구분해 유지할 수 있어야 한다.
9. jsource `V.fgh`는 execution object의 cross-check이지 semantic DAG child의 절대 oracle이 아니다. `h` 등은 실행용 parameter storage로도 재사용되므로 parser production에서 확인된 semantic operand만 DAG edge로 올린다.

<a id="331-derived-construction의-결과는-function으로-한정하지-않는다"></a>
#### 3.3.1 derived construction의 결과는 Function으로 한정하지 않는다

current jsource의 hook/bident/trident construction(`cf.c`)은 input POS 조합에 따라 Verb뿐 아니라 Adverb/Conjunction을 만들며, 일부 production은 즉시 실행되어 Noun을 만든다.

따라서 일반 모델은:

```text
parser_action(form, operands)
  -> JEntity
       ├─ Noun
       └─ Function { result_pos, form, operands }
```

이다. `DerivedVerb`는 Function 결과 중 POS가 Verb인 경우의 convenience view일 뿐이다.

예를 들어 `ADV + ADV + VERB`, `CONJ + VERB + CONJ`, `NOUN + CONJ + ADV` 같은 조합은 constructor table의 실제 result POS를 따라야 하며, `NOUN VERB NOUN`처럼 parser action이 값을 즉시 실행해 noun을 산출하는 경우도 공통 `JEntity` result boundary에서 표현할 수 있어야 한다.

<a id="331a-gerund는-새-atom-type이-아니라-contextually-interpreted-boxed-noun이다"></a>
#### 3.3.1a gerund는 새 atom type이 아니라 contextually interpreted boxed noun이다

J gerund의 **J-visible 출발점은 boxed noun**이고, 특정 modifier 문맥이 그 noun을 function/entity sequence로 해석한다.

2026-10-04 재검토한 current jsource `cg.c::jtfxeachv/jtfxeach`는 이 해석 결과를 실행하기 위해 원래 boxed noun의 rank/shape를 복사한 `BOX`-tagged 내부 carrier를 만든다. 그러나 jsource 주석 자체가 이 결과를 **“array of boxes라고 주장하지만 각 box slot에는 function type A가 들어갈 수 있다”**고 설명한다. 즉 이것은 J-visible `array of functions` semantic type의 증거가 아니라 runtime 내부 realization trick이다.

따라서 RustJ는 이 internal representation을 semantic type으로 승격하지 않는다. 기본 모델은 다음과 같다.

```text
Boxed Noun
  noun shape/rank
  ordinary boxed payload
      + operator-specific interpretation
          ↓
GerundView / InterpretedEntitySequence
  references to interpreted Function/JEntity values
  source-container shape only when that operator semantics needs it
  origin = boxed noun + interpretation context
```

핵심 규칙:

1. 개별 Verb/Adverb/Conjunction은 noun-style array shape/rank를 갖지 않는다.
2. gerund interpretation이 shape를 필요로 하면 그 shape는 **source boxed noun**에서 보존한다.
3. jsource의 function-containing fake-BOX carrier를 RustJ `Value::Boxed`로 복제하지 않는다. RustJ의 `Value::Boxed`는 J noun invariant를 유지한다.
4. generic `EntityArray`/`EntityArrayView`는 gerund 하나만으로 정당화하지 않는다. 먼저 operator-specific `GerundView` 또는 `InterpretedEntitySequence`로 충분한지 확인한다.
5. 같은 boxed noun이 ordinary data로 쓰이는 문맥과 gerund로 해석되는 문맥을 분리하고, name/function reference의 fix/late-binding 규칙을 보존한다.
6. current `decoded_gerund: Vec<FunctionEntity>`처럼 shape를 버리는 cache/view가 각 gerund operator의 observable semantics에 충분한지는 operator별로 검증한다.

즉 jsource에서 가져올 것은 **boxed noun → context-sensitive function interpretation**이라는 semantic rule이고, function A를 BOX payload에 직접 넣는 내부 storage trick은 가져오지 않는다.

<a id="332-frontend-전체를-jsource-compatible-pipeline으로-유지한다"></a>
### 3.3.2 Frontend 전체를 jsource-compatible pipeline으로 유지한다

RustJ는 적어도 **word formation -> enqueue -> parser까지 current jsource를 구현 명세에 가깝게 따른다.** 이 구간에서 compiler-friendly 문법을 새로 만들지 않는다.

```text
source bytes
  ↓
Word Formation
  jsource: jtwordil / w.c state table
  RustJ:   word_former
  ↓
Enqueue
  jsource: jtenqueue
  RustJ:   enqueuer
       ├─ spelling / primitive resolution
       ├─ numeric & string construction
       ├─ name validation / lookup flags
       └─ assignment classification
  ↓
Parse Queue
  ↓
Parser
  jsource: p.c 9-row reduction behavior
  RustJ:   same language/reduction semantics
  ↓
J semantic values / completed function entities
  ↓
RustJ Semantic IR / Analyzer
  ↓
Logical / Optimization / Target / Physical IR
```

RustJ가 달리 구현해도 되는 것은 포인터 low-bit tagging, refcount, in-place bookkeeping, cache layout, branch-prediction tricks 같은 C implementation detail이다. 반대로 **word boundary, enqueue classification, primitive/name distinction, parser lookup timing, parse reduction, result POS/error semantics**은 jsource compatibility 대상이다.

<a id="enqueuer의-primitive-resolution-hook"></a>
##### Enqueuer의 primitive-resolution hook

jsource `jtenqueue`는 symbol spelling primitive와 ordinary NAME을 구분한다. RustJ도 이 경계를 유지한다. **enqueue 단계에서 primitive로 고정되는 것은 core J primitive spelling뿐**이고, alphabetic extension 이름은 ordinary NAME + lookup metadata로 들어간다.

```text
source "+"
   ↓ enqueue core primitive resolution
PrimitiveHandle(Core(Add))
   ↓
existing parser rows

source "conv"
   ↓ enqueue
NAME("conv", lookup=true)
   ↓ parser-time name lookup
current binding / expected POS
   ↓
extension-derived semantic entity
```

`PrimitiveHandle`의 semantic 정보는 target-independent다. built-in과 extension-derived computational entity는 parser/name-binding 이후 공통 semantic capability와 `lowering_key` 체계로 수렴할 수 있지만, **extension visibility나 현재 binding을 enqueue가 keyword처럼 고정하지 않는다.**

따라서 compile invocation의 `PrimitiveContext`는 core primitive catalog와 extension binding catalog를 함께 가질 수 있어도 역할이 다르다.

- enqueuer: core J spelling만 `resolve_core_for_enqueue`로 해석한다.
- parser/name environment: ordinary NAME의 현재 binding을 lookup하고 extension entity/POS를 결정한다.
- target lowering: semantic entity의 stable lowering key를 active `TargetContext`에서 조회한다.

중요한 규칙:

- Core J primitive의 spelling/POS/semantic contract는 jsource와 호환되어야 한다.
- Extension name은 parser keyword가 아니며 enqueue에서 Verb/Adverb/Conjunction으로 고정하지 않는다.
- extension shadow/rebind는 ordinary J name semantics를 따른다.
- target hardware 정보 자체는 semantic entity에 복사하지 않는다.
- enqueuer에서 target-specific kernel을 선택하지 않는다.
- portable extension은 target implementation이 없더라도 semantic resolution 자체는 가능하며, lowering 단계에서 unsupported/fallback을 판단할 수 있다.
- 명시적으로 target-gated인 extension을 향후 지원하더라도 그 gating은 compile profile의 명시적 정책이어야 하며 core J semantics를 변경해서는 안 된다.

<a id="3321-frontend-compatibility-invariant"></a>
#### 3.3.2.1 Frontend compatibility invariant

`word formation -> enqueue -> parse` 구간은 RustJ가 새 문법을 설계하는 곳이 아니다. 이 세 단계는 **jsource frontend semantics를 충실히 이식**한다.

1. **Word formation / lexer**
   - `w.c::state` state machine을 기준으로 한다.
   - character class, state transition, word boundary emission, follow-on numeric 처리, quote/comment/DD(`{{ }}`) 처리까지 같은 규칙을 사용한다.
   - Rust enum/table로 표현은 바꿀 수 있지만 handwritten heuristic으로 별도 규칙을 만들지 않는다.

2. **Enqueue**
   - `jtenqueue`의 classification order를 기준으로 한다.
   - primitive lookup, constant construction, name validation, assignment classification, lookup-name marking, result parser class/POS를 같은 semantic 순서로 결정한다.
   - jsource의 pointer low-bit QC tagging은 Rust의 explicit enum/flags로 바꾸되 의미는 보존한다.
   - enqueue-time primitive resolver는 **core J primitive spelling**의 lookup/classification만 담당한다. extension은 ordinary `NAME`으로 유지하고 parser-time 정상 binding/POS lookup에서 semantic entity를 얻는다. parser grammar에는 extension 전용 production을 추가하지 않는다.

3. **Parser**
   - `p.c::cases[]`와 runtime `ptcol` dispatch의 9-row rule을 기준으로 한다.
   - eligibility, precedence, reduction extent, result POS, reduction 후 stack 재삽입/rescan을 같은 규칙으로 구현한다.
   - Hook/Fork/bident/trident와 modifier construction도 jsource constructor semantics를 따른다.

```text
jsource
  wordil state machine
        ↓
  enqueue classification
        ↓
  9-row parser reduction
        ↓
  J semantic result

RustJ
  same frontend semantics
        ↓
  J Semantic IR
        ↓
  RustJ-specific analyzer / logical / target / physical IR
```

따라서 frontend 단계에서 차이가 허용되는 것은 **representation과 implementation technique**뿐이다. J-visible word formation/classification/parsing behavior는 compatibility 대상이다.

<a id="3322-canonical-frontend-sentence-trace---y"></a>
#### 3.3.2.2 Canonical frontend sentence trace — `+/ y`

frontend orientation 표본은 **이미 noun으로 바인딩된 `y`**에 대해 다음 한 문장을 사용한다.

```j
+/ y
```

이 표본은 짧지만 word formation, enqueue classification, parser-time name lookup, modifier construction, monadic application을 모두 지난다.

~~~text
source bytes
  "+/ y"
    │
    ▼
Word Formation
  word[0] = "+"
  word[1] = "/"
  word[2] = "y"
    │
    ▼
Enqueue
  0: Verb(Add)
  1: Adverb(Insert)
  2: Name("y", lookup_name=true)
    │
    ▼
Parser stack entry for y
  lookup current sentence environment
  Name("y") ──lookup──→ Noun snapshot
    │
    ▼
9-row reduction: row 3 / Adverb
  Verb(+)  Adverb(/)
        │
        ▼
  completed FunctionEntity
    POS  = Verb
    head = PrimitiveAdverb(Insert)
    operand[0] = Function(Add)
    source provenance = words 0..2
    │
    ▼
stack reinsert + rescan
    │
    ▼
9-row reduction: row 0 / MonadEdge
  derived Verb(+/)  Noun(y)
        │
        ▼
  Expr::Monad / completed noun result
    │
    ├─ runtime parser host: 실제 J value를 계산하여 Noun으로 stack에 재삽입
    └─ analysis/static path: application structure를 보존하여 후속 J Graph/A3 분석
~~~

여기서 중요한 경계는 다음과 같다.

- `/`는 enqueue 시점에 `Reduce`가 아니다. **Adverb**이며 row 3이 `+`에 적용해 completed derived Verb `+/`를 만든다.
- `y`는 enqueue에서 단지 lookup 대상 NAME이다. parser stack에 들어갈 때 **그 시점의 환경**을 읽어 noun snapshot을 얻는다. sentence 시작 시 모든 name을 미리 고정하지 않는다.
- parser row 3의 결과는 target-independent `FunctionEntity`다. CPU/GPU/fusion 정보가 들어가지 않는다.
- row 0 application 뒤에야 noun computation이 생긴다. 이 applied computation이 이후 J Graph에서 reduction structure로 분석되고 Execution Semantic Lowering에서 `Reduce(Add)`가 될 수 있다.
- runtime parser action이 실제 noun을 계산하는 것과 static analyzer가 구조를 보존하는 것은 서로 다른 execution mode지만, **같은 9-row language semantics**를 공유해야 한다.

현재 회귀 근거:

- `tests/enqueuer.rs`는 core Verb/Adverb/Conjunction/Name/assignment의 enqueue class와 lookup flag/source provenance를 검사한다.
- `tests/parser_provenance.rs::all_nine_rows_preserve_original_word_coverage_and_inherited_tokens`는 `+/`가 `ParseRow::Adverb`로 reduction되고 원 word coverage/token provenance를 유지하는지 검사한다.
- `src/parser.rs::apply_parse_row`는 row 3 construction과 row 0 monadic application을 분리하고 reduction 뒤 stack rescan을 수행한다.
- `tests/analysis.rs`의 `+/1 2` 분석 회귀는 derived semantic head가 `PrimitiveAdverb(Insert)`로 남은 채 A3까지 내려가는 것을 검사한다.

이 trace는 frontend 전체 conformance를 대표하는 **orientation example**이지, locative/direct definition/gerund/value-dependent constructor까지 모두 지원된다는 증거가 아니다. 그 범위는 A0.5/F0–F2/P0–P8 checklist와 differential report가 결정한다.

<a id="3323-compilerinterpreterjit-공통-structured-diagnostics"></a>
#### 3.3.2.3 Compiler/interpreter/JIT 공통 structured diagnostics

source provenance는 compiler 전용 부가기능이 아니라 frontend semantic infrastructure다. 최신 J의 `d.c` + `eformat_j_` 구조를 참고해 **J error class와 실패 당시의 semantic context를 분리하여 보존한 뒤, 별도 DiagnosticAnalyzer가 설명을 생성**한다. parser conformance 기준 revision은 그대로 유지하고, diagnostic design은 2026-09-30 current jsource master(`1d43f4eb7e43c8243f64dc4e6f31afde4e19d6a9`)의 error-context/eformat 구조도 참고한다.

```text
J-compatible ErrorKind
  syntax / domain / length / rank / ...
        +
ErrorContext
  phase
  SourceSpan
  original word index
  current name
  executing semantic operation
  valence
  small x/y summaries
  structured FailureDetail
        ↓
DiagnosticAnalyzer
  generic semantic explanation
  primitive-specific analyzer
  parser/enqueue explanation
        ↓
Diagnostic
        ↓
Python-style Renderer
  File / line / column
  source excerpt
  caret/range
  readable class
  executing fragment
  argument facts
  semantic explanation
        ↓
AOT compiler / interpreter / JIT / REPL
```

예:

```text
  File "model.ijs", line 27, column 14
    z =: x +"1 y
             ^~~

LengthError: length error
  during execution
  while executing dyad +"1
  x: type 4, rank 2, shape [2, 3]
  y: type 4, rank 2, shape [4, 3]
  argument shapes [2, 3] and [4, 3] do not conform
```

최신 J에서 가져올 핵심은 표시 모양 자체보다 다음 구조다.

- parser stack이 original token number를 유지하고 오류 시 blame token을 추론한다.
- enqueue/parse/execution/assembly 등 **실패 phase**를 구분한다.
- 실행 중인 entity와 monad/dyad valence를 보존한다.
- rank/shape/type/value/index 같은 실제 argument context를 이용해 terse error보다 구체적인 설명을 만든다.
- error formatter가 큰 noun을 복사하거나 hash table을 만들지 않도록 한다.
- formatting/analyzer 오류가 원래 J error를 덮어쓰지 않는다.

RustJ 원칙:

- 내부 provenance는 UTF-8 byte span을 canonical representation으로 사용하고 original enqueue-word index를 별도로 보존한다.
- 표시할 때 1-based line과 Unicode-scalar column으로 변환한다.
- 가장 안쪽 단계가 붙인 정확한 context를 outer compiler/runtime layer가 덮어쓰지 않는다.
- `ErrorContext.arguments`에는 전체 array가 아니라 type/shape/rank 등 작은 summary만 둔다.
- primitive-specific analyzer가 필요한 경우 문자열 메시지를 즉석에서 조립하지 않고 `FailureDetail` 같은 structured failure detail을 추가한다. `DiagnosticAnalyzer`가 이를 human explanation으로 변환하되 J machine error class는 바꾸지 않는다.
- JSON/conformance API와 기존 `eval()/analyze()`는 wrapper를 제거한 J-compatible machine error를 반환한다.
- `parse_diagnostic()/eval_diagnostic()/analyze_diagnostic()`은 같은 semantics를 실행하면서 context를 유지한다.
- runtime kernel error와 향후 physical backend error도 semantic/logical operation의 source origin으로 돌아갈 수 있어야 한다.
- 향후 multi-file/import/definition이 생기면 `SourceId + byte span`으로 일반화한다.
- interpreter/JIT용 별도 error system을 만들지 않는다.

<a id="333-parser-language-rules-are-jsource-compatible"></a>
#### 3.3.3 Parser language rules are jsource-compatible

RustJ는 parser 단계에서 별도의 언어 규칙을 발명하지 않는다. **token/word가 J parser에 들어온 뒤 어떤 fragment가 언제 reduction되고, 어떤 part of speech의 결과 entity가 다시 parser stack에 놓이는지는 current jsource의 parser 규칙을 기준으로 한다.**

즉 RustJ와 jsource의 차이는 parser language semantics가 아니라 **parser 이후의 representation/analysis/lowering**에서 만든다.

```text
J words / names
  ↓
jsource-compatible parser reductions
  - row 0: monad
  - row 1: monad after verb phrase
  - row 2: dyad
  - row 3: adverb application
  - row 4: conjunction application
  - row 5: fork
  - row 6: hook / bident / trident
  - row 7: assignment
  - row 8: parentheses
  ↓
J values / completed function entities
  ↓
RustJ Semantic Analyzer
  ↓
RustJ Logical IR / compiler facts
```

RustJ는 jsource의 C parser implementation details(bit-packed parse masks, refcount/inplacing mechanics, function pointers, cache tricks)를 복제할 필요는 없다. 그러나 **reduction eligibility, reduction ordering, result POS, parser-time name lookup semantics, completed modifier entity boundaries**는 호환되어야 한다.

과거 `reduce_modifier_applications -> collapse_verb_trains` staged helper는 최종 semantic model이 아니었다. 현재 선언 row matcher는 통합되어 있으나 runtime semantic actions와 전체 constructor/POS coverage는 아직 미완료다. 장기적으로는 jsource의 9-row parse behavior를 하나의 parser reduction engine에서 재현하고 differential tests로 검증한다.

