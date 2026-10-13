[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)


<a id="architecture-migration-checklist"></a>

<a id="read-roadmap"></a>
<a id="10-구현-계획과-체크리스트"></a>
## 10. 구현 계획과 체크리스트

이 절이 앞으로 유일한 구현 체크리스트다. **저장장치·느린 I/O·Out-of-core 작업은 [§10 IO-01~IO-30](./11-roadmap-checklists-p05.ko.md#out-of-core-io-checklist)을 이 절 안의 단일 수용 목록으로 사용한다.** 순차 CPU 기준 실행과 J 의미 수렴은 계속 M2→M3→M4 우선이며, IO-A 조사만 병행할 수 있다.


<a id="jaxa-future-work-traceability"></a>

<a id="jfw--jaxa-future-work-10개-항목의-rustj-이행-추적표-2026-10-13-문서설계만"></a>
### JFW — JAXA Future Work 10개 항목의 RustJ 이행 추적표 (2026-10-13; 문서·설계만)

**근거와 권한:** [JAXA 원본 명세의 Future Work 1–10](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/JAXA-README.md)을 항목별로 역대조했다. 이 표는 **기존 §10 작업의 추적 색인**이며, 독립 로드맵·새 필수 IR·새 language semantics가 아니다. §10이 유일한 수용 체크리스트다. 현재 코드/문서에 *설계*가 있다는 사실은 *구현·증명·E2E 완료*가 아니다. 아래의 미완료 체크를 기존 M2→M3→M4보다 앞선 blocking gate로 승격하지 않는다.

| 원본 항목 / RustJ 식별자 | 적용 판단과 현재 경계 (2026-10-13) | 연결된 계층·근거 | 우선순위 |
|---|---|---|---|
| **FW1 Completeness Verification / JFW-01** | 범용 신경망 표현의 형식적 완전성 증명 대신, 실제 J 구문·Rank·shape·effect를 유지하는 *명시적 지원 범위/대표 모델 수용 매트릭스*부터 검증. 전체 완전성은 연구 항목 | Frontend, J Semantic IR, NN extension E2E·coverage manifest | 중기, 형식 증명 선택 |
| **FW2 Automatic Gradient Derivation / JFW-02** | 원래의 자동미분 생성 목표를 **명시적 adjoint/VJP primitive 및 사용자 작성 backward graph**로 변경. Autograd는 필수 기능이 아님. `AdjointVjp` topology는 예약, 실제 전개·실행은 미완료 | `src/opportunity.rs`, `src/nn_extensions.rs`, explicit StateResource | 중기, 방향 변경 |
| **FW3 Deterministic Memory Scheduling / JFW-03** | 버퍼/뷰 식별과 첫 CPU PhysicalPlan 계약은 있음; 범용 lifetime·offset 배치·재사용·activation retention·transfer plan은 미완료. 정적 크기가 증명되는 region에만 확정 계획 적용 | `src/physical_plan.rs`, §5, §8, M4 | 높음, CPU 경로 후 |
| **FW4 Equivalent Form Search / JFW-04** | `j_graph_rewrite` / fusion 후보와 `egg` 설계는 있으나 실행 가능한 범용 EqGraph 탐색·extraction 없음. 규칙·가드·증거를 등록하고 안전한 후보만 탐색 | J Graph Basis/Rewrite, [공통배열대수 가설](./11-roadmap-checklists-p04.ko.md#common-array-algebra-hypothesis), `src/j_graph_fusion.rs` | 최상, 후속 연구 |
| **FW5 Basis Verb Identification / JFW-05** | 전역 최소 primitive 집합을 먼저 증명하지 않음. **Graph Basis(대수적 재작성 단위) ≠ Execution/Lowering Basis(물리 구현 단위)** 구분; 확장 연산 분해는 검증 이후 허용 | J Graph IR, Logical IR, Extended Primitive, §4–§5 | 높음, FW4/8와 연계 |
| **FW6 Distributed Execution / JFW-06** | device·memory-space·sharding·transfer·ready/synchronization의 계획 계약을 보존하되 실제 multi-GPU/multi-node 런타임은 장기 후속 | §7.5, HE, Physical Plan, `AGENTS.md` | 장기 |
| **FW7 Hardware Portability / JFW-07** | target locale/capability/lowering 계약 설계 존재. CPU-native E2E를 먼저 닫고 GPU·MLIR/LLVM·StableHLO 부분집합 등은 독립 적법성/실행 검증 후 확장. 현재 CUDA 실행 미구현 | TargetContext, LoweringRegistry, M4→M6, `AGENTS.md` | 높음, 단계적 |
| **FW8 Model Equivalence Verification / JFW-08** | 정확한 J 관찰 동등성과 오차가 있는 수치 후보를 분리. `numeric_error_policy`는 분석 전용; 동등성 증명기와 EqGraph의 안전한 e-class 병합은 미완료 | J-C 차등 오라클, FactWitness, `src/numeric_error_policy.rs`, `AGENTS.md` | 최상, FW4의 선행 검증 |
| **FW9 Structural Sparsity / JFW-09** | J-visible sparse 배열 의미 일부 구현, dense/sparse 표현 분리. sparse assembly·atomic kernel 경로에 미지원 사례 존재. MoE·sparse attention의 구조가 입력에 따라 동적일 수 있어 정적이라고 간주 금지 | `src/sparse.rs`, `src/assembly.rs`, `src/kernels.rs`, NN extension | 중기 |
| **FW10 Quantization Integration / JFW-10** | dtype/precision·cast와 accumulation policy의 독립 계약부터 검증. 원래 JAXA의 precision 표기를 표준 J 구문으로 무단 도입하지 않음. 양자화·mixed precision·자동 cast는 아직 완료 아님 | Frontend DType/Rank, NN family spec, target-specific lowering | 중기 |

**독립 검증·수용 체크리스트 (현재 상태; 각 항목의 완료는 기능별 구현 증거가 생긴 후에만 표시):**

- [x] **JFW-00 추적표 작성:** 원본 FW1–FW10 출처, RustJ 책임 단계, 현행 구현 경계, 우선순위, 한·영 mirror 매핑을 문서에 등록했다. **문서 작업만 완료**.
- [ ] **JFW-01 / V01:** ResNet·Transformer·비-NN 배열 사례를 포함한 구조 표현 매트릭스와 오류·Rank·shape·binding의 J-C 차등, 지원·미지원 부정 사례를 기록한다. 범용성 증명은 별도 상태다.
- [ ] **JFW-02 / V02:** 명시적 adjoint/VJP 등록→Frontend 인식→Graph/Logical fan-out→값·weight gradient/state effect 검증→CPU 실행을 단계별로 닫는다. 원시 forward 식에서 자동 미분을 생성하는 것과 혼동하지 않는다.
- [ ] **JFW-03 / V03:** M4 CPU multi-op에서 value-last-use, buffer lease/alias/ownership, 안전한 reuse, materialization/no-materialization을 증명·실측한다. 미지 크기/효과/오류는 안전한 경계를 유지한다.
- [ ] **JFW-04 / V04:** guard가 있는 정확한 rewrite 1개부터 end-to-end 후보 생성→합법성→extraction→기본 실행 대비 동등성/비용 비교를 실증한다. 탐색 성공 자체는 의미 증명이 아니다.
- [ ] **JFW-05 / V05:** Graph Basis별 J 관찰 의미·rank/shape·빈 결과·오류·numeric 가드를 등록한다. Conv 등 extension은 *참조 정의*와 *실제 대체 실행의 동등성 증거*를 구별한다. 최소 기저의 완전성 증명은 필요조건이 아니다.
- [ ] **JFW-06 / V06:** 단일 CPU·zero-transfer 경로 수용 뒤 장치/메모리 간 version·transfer·ready·sync·effect/error precedence를 검증하며, 다중 노드 실행은 별도로 검증한다.
- [ ] **JFW-07 / V07:** 동일 J 연산을 native CPU, 다른 target route에서 지원 전제와 결과·오류·효과까지 비교한다. 장치별 cost/profile과 언어 의미를 분리한다.
- [ ] **JFW-08 / V08:** provenance, Rank/Cell/Frame, prototype/fill, NAME, effect, first observable error, CCT/Fit, float/NaN/zero를 포함한 **Exact-J witness** 없이 e-class를 union하지 않는다. J의 `2^-44` 비교 허용 오차는 최적화 오차 예산이 아니며 근사 동등은 비추이적이다.
- [ ] **JFW-09 / V09:** 기존 sparse J semantics를 보존한 구조적 희소 연산/NN routing의 긍정·부정 테스트를 만들고, sparse pattern이 runtime dependent이면 guarded/dynamic route로 남긴다.
- [ ] **JFW-10 / V10:** 저장·계산·누적 dtype, 명시적/자동 cast의 안전 조건, quantize/dequantize 및 overflow/rounding/empty/error를 오라클과 비교한다. FP32 단독 통과를 mixed precision 완료로 세지 않는다.
- [ ] **JFW-X1 / VX1 — Rank-aware Fusion:** `CellApply`/Map/Reduce/Scan/reindex의 고수준 topology를 보존하고, fusion 전에 frame/cell agreement, empty/prototype, error/effect 순서를 증명한다.
- [ ] **JFW-X2 / VX2 — Resource-aware Search:** 연산수·bytes moved·temporary peak·layout/tiling·transfer·resource feasibility를 분리 추정한다. 안전하지 않은 early pruning은 금지하고, 선형 chain 비용 평가와 DAG branch/fanout 비용을 구분한다.
- [ ] **JFW-X3 / VX3 — Explicit State & Adjoint Planning:** forward/backward/optimizer의 `StateResource`, `ValueId`, `BufferId`를 구분하고 gradient emission·activation lifetime·effect/error barriers를 검증한다.

**공통 수용 규약:** 각 V01–V10, VX1–VX3에 `Test / Purpose / Guarantee / Failure meaning / Status`, 재현 입력(특히 반례), 지원 scope, 비교 J-C 기준/커밋, 실제 CI/PR head·merged-main 증거를 기록한다. `PASS`, `FAIL`, `UNRUN`, `BLOCKED`를 구분하며 문서만 추가한 상태는 `UNRUN`이다. 표·체크리스트는 구현 전부 **미완료**이며, 향후 증거가 있을 때 해당 항목만 갱신한다.

**진행 순서와 비목표:** M2 Frontend/의미 → M3 적법성/Logical single-block → M4 native CPU E2E를 먼저 닫는다. 이후 JFW-08/04/05의 *검증된 작은 부분집합* → JFW-03/X1/X2의 자원·fusion → JFW-02/X3·07·09·10의 실행 확장 → JFW-06/형식 완전성 연구 순서로 단계적으로 진행한다. 여러 항목의 연구·문서화는 병행할 수 있지만 CPU 기준 경로의 블로커가 되어서는 안 된다. **자동미분 엔진, 전역 최소 Basis 증명, 전체 J/모든 모델의 완전성 증명, 분산 실행, GPU 실행은 첫 CPU 수직 경로의 필수 선행조건이 아니다.**

<a id="dynamic-boundary-checklist"></a>

<a id="db--동적-의미와-컴파일-경계-이행-db0db7"></a>
### DB — 동적 의미와 컴파일 경계 이행 (DB0–DB7)

이 목록은 §10 M2→M3→M4와 WI3/WI4/M5–M8의 연결 지점이며 별도 병렬 backend 구현 계획이 아니다. **M2 frontend 수렴을 먼저 진행한다.** 해당 semantic 변경을 구현할 때 필요한 회귀를 추가하고, 최적화 자체는 별도 승인된 단계에서 진행한다.

- [x] **DB0 계약 문서화:** 경계·검사 시점·변환 금지 조건·실패 처리와 초기 구현/미완료 상태를 정본과 mirror에 통합한다.
- [ ] **DB1 구조화된 경계 보고:** source span, semantic phase, reason, 필요한 fact/witness, 허용 분석·거부 변환, 필요한 route capability를 기록한다. unknown facts·J-invalid·implementation coverage·route rejection을 구별하며 valid unsupported 사례의 원인을 보존한다.
- [ ] **DB2 witness 유효성:** parser POS/constructor snapshot/call-time lookup을 구분한다. binding/frame/locale/path와 unbound-search 결과의 의존성을 추적하고 검사→사용 사이 mutation을 확인한다. WI4·M5/M6의 noun metadata 연결과 함께 구현한다.
- [ ] **DB3 효과·오류 경계:** lookup/write/context/I/O/resource/error를 ordered region 또는 동등한 명시적 의존성으로 연결한다. fork/selector·같은 이름의 두 호출·assignment expression에서 값과 효과의 live-out을 분리한다.
- [ ] **DB4 첫 실행 경로 선택:** Native CPU slice에서 effect 이전에 필요한 guard를 검사한다. miss 시 J 오류를 만들거나 replay하지 않고 실제 지원 경로/재분석/coverage 경계로 분기한다. 외부 route와 CUDA는 capability 증명 후 별도 확장한다.
- [ ] **DB5 중간 전환의 선행 조건:** effect 이후 전환이 필요해질 때 continuation 상태·소유권·정확히 한 번 효과·오류 위치 계약을 먼저 명세·검증한다. 완료 전 중간 fallback을 활성화하지 않는다. full continuation을 첫 CPU slice의 무조건적 선행 조건으로 만들지 않는다.
- [ ] **DB6 Windows 차분 gate:** NAME 재정의/POS 변경, local 미정의→정의, locale/path 변경, noun snapshot 뒤 재대입, 값 의존 constructor, 효과 뒤 오류/guard miss를 C 기본·AVX2와 비교한다. 값/type/shape뿐 아니라 lookup 시점·효과 순서·실패 후 binding과 실행 횟수를 검사한다. 현재 미지원 locale/execute는 별도 coverage로 보고한다.
- [ ] **DB7 중후반 의미·성능 gate:** 검증된 direct runtime과 Logical/Physical 실행을 같은 입력으로 대조한다. guard hit/miss와 empty/boxed/sparse 경계를 포함하며 성능·복사/할당은 의미 통과 뒤 별도 측정한다. frontend 통과나 메타데이터 분석을 backend 실행/성능 통과로 승격하지 않는다.

<a id="nv3a-spelling-errors"></a>

**NV3a fixed spelling 오류 분류 — 2026-10-05.** `jsrc/ws.c::spellin`과 `jsrc/w.c::jtenqueue`의 순서를 따른다. 설치·검증된 core dictionary가 우선이며, 등록되지 않은 colon inflection 또는 nonnumeric dot inflection은 spelling error다. numeric dot는 numeric constructor로 넘기고, quote와 simple name은 각각의 분류를 유지한다. 한 자리 constant function은 기존 core descriptor를 통과한다. `99:`/`1.5:`/`_99:`는 reviewed C에서 유효한 constant function이 아니므로 spelling error다. 잔여 잘못된 문자·미설치 primitive도 Unsupported가 아닌 spelling error다. obsolete spelling의 임의 예외 목록은 만들지 않는다.

`name_:`는 문법적으로 유효한 by-value/abandon lookup이다. simple-name validation을 거친 뒤 별도 Unsupported 경계로 남긴다. `foo__:`처럼 suffix 제거 후 명백히 ill-formed인 simple name은 ill-formed name으로 보고한다. bounded locative grammar는 후속 NV3b에서 검증한다. NAME 길이 제한과 오류 순서는 후속 NV3c에서 검증한다. locale lookup·abandon 효과와 complex/extended/rational numeric grammar는 아직 완료되지 않았다. 따라서 fixed spelling seam인 NV3a만 완료했고 NV3/NV5 전체 완료를 주장하지 않는다. invalid lexical spelling과 valid primitive의 constructor/executor 미지원은 계속 구별한다. 기존 enqueue diagnostic의 phase·source span·word index를 유지한다.

`tools/spelling_conformance.py`는 graphic ASCII 93개(quote 제외) × 7개 suffix의 **651** matrix와 추가 이름/숫자 경계를 양 DLL과 비교한다. 오류 종류·word formation·유효 미지원 경계를 각각 검증하며 primitive 실행 지원이나 전체 name/numeric grammar conformance로 세지 않는다. `tools/vocabulary_audit.py`도 기존 code-only 후보 2개와 legacy 8개의 Rust 오류를 C와 반드시 비교한다. 표준 Windows runner에 spelling report를 추가했다.

**NV3a 검증:** native Windows default/portable 각각 **465 passed / 17 ignored**, fmt/clippy/build 통과; Python **30 passed**. 양 DLL 각각 spelling **667 cases / failed 0**: 오류 종류 **453**(spelling **426**, number **22**, name **1**, syntax **4**), accepted enqueue controls **208**, valid Unsupported 경계 **6**. 이 중 matrix는 **651**이며, quote grammar와 전체 locative/numeric grammar는 범위 밖이다. 기존 세 runtime 경로 각각 **5,380 cases / 5,380 passed / failed 0**, stages **10,810**, words **6,623**, vocabulary POS **143**/binding **140**/noun **3**를 유지했다. Scan **285 / failed 0**와 runtime prefix 경계 **285 / executable prefix passes 0**, capture graph 경계 **257**, static 경계 **2**를 별도로 유지했다. 전체 보고서 16개의 binary/source/DLL hash를 확인했다. spelling pass는 실행 지원이나 GPU 성능 검증이 아니다. Linux/GitHub CI/CUDA는 실행하지 않았다.

<a id="nv3b-name-syntax"></a>

**NV3b bounded name syntax — 2026-10-05.** `sn.c::vnm/vlocnm`을 참조해 shared enqueue name validator를 보강한다. ASCII letter로 시작하고 alphanumeric/underscore로 구성된 이름에서 simple name, trailing direct locative와 empty/base locale `__`, indirect chains, 최종 numeric debug-frame component 및 그 음수 표기를 구분한다. numeric direct locale의 leading zero·x64의 18자리 제한, 잘못된 중간 숫자/isolated underscore/과도한 underscore를 검사한다. `name_:`는 suffix를 제거한 같은 이름 문법으로 검증한다. validator는 locale·symbol을 조회하거나 noun/function을 생성하지 않고 추가 heap allocation 없이 동작한다.

문법적으로 valid인 locative/by-value name은 계속 Unsupported이며 locale lookup·debug-frame 접근·abandon 효과를 구현했다고 주장하지 않는다. malformed name은 enqueue phase/span/word index를 보존한 ill-formed name이다. NV3b 당시 남겼던 NAME 전체·simple-name·locale storage 길이 제한과 오류 우선순위는 후속 NV3c에서 검증한다. full numeric grammar와 locale 실행은 여전히 NV3/DB2의 잔여 작업이다. 따라서 bounded syntax인 NV3b만 완료하며 전체 NV3/DB2/locales 완료가 아니다. 정의의 `for_name.` 분류도 shared enqueue validator를 통과하므로 이 동일한 bounded syntax 검증을 사용한다.

`tools/name_syntax_conformance.py`는 짧은 `a0_` 조합, direct/indirect/debug-frame 사례, 고정 seed의 mixed-case/digit/underscore 이름 및 각 `name_:` 형태를 생성해 양 DLL과 대조한다. C의 enqueue 오류를 실행 이후 value/locale 오류와 구별한다. 후자의 발생은 이름의 lexical validity만 확인하며 lookup/runtime 성공으로 세지 않는다. `sn.c`·`w.c`·`ws.c`·`jerr.h` source hash와 probe/DLL hash를 보고서에 보존하고 표준 Windows runner에 연결한다.

**NV3b 검증:** native Windows default/portable 각각 **466 passed / 17 ignored**, fmt/clippy/build 통과; Python **30 passed**. 양 DLL 각각 name syntax **4,030 cases / failed 0**: simple name **1,133**, valid Unsupported name **1,901**, invalid name **996**. spelling **667 / failed 0**와 vocabulary POS **143**/binding **140**/noun **3**, 세 runtime 경로 각각 **5,380 cases / 5,380 passed / failed 0**, stages **10,810**, words **6,623**를 유지했다. Scan **285 / failed 0**와 runtime prefix 경계 **285 / executable prefix passes 0**, capture graph 경계 **257**, static 경계 **2**를 별도로 유지하고 전체 보고서 18개의 binary/source/DLL hash를 확인했다. lexical validity 확인은 locale runtime 지원이나 실행 conformance pass가 아니다. Linux/GitHub CI/CUDA는 실행하지 않았다.

<a id="nv3c-name-limits"></a>

**NV3c NAME 길이와 오류 순서 — 2026-10-05.** `sn.c::nfs`의 J-visible compatibility 검사를 추가한다. underlying name의 전체 byte 길이는 **1 ≤ n < 32767**이며 이 범위를 벗어나면 ill-formed name이다. 저장될 simple-name 부분과 locale 부분은 각각 **255 bytes 이하**다. direct locative는 마지막 locale separator를 기준으로 나누며 empty/base locale도 구별한다. indirect locative는 첫 `__` 뒤 **전체 chain suffix**를 locale 크기로 검사한다. chain의 각 component만 255 이하인 것으로는 충분하지 않다. 이 값들은 J 호환성 조건이며 Rust storage/allocator/physical layout의 제한이 아니다.

검사 순서를 보존한다: 전체 길이 → indirect 마지막 numeric/debug-frame text의 digit validation → simple/locale 크기 → `vnm` 문법. 마지막 numeric component에 문자가 섞이면 다른 부분이 과도하게 길어도 ill-formed name이 먼저다. 그 외의 malformed locative는 component 크기를 먼저 검사하므로 limit error가 문법 오류보다 앞설 수 있다. `name_:`는 suffix를 제외한 underlying name에 동일한 검사를 적용한다. primitive inflection의 spelling 검사는 그보다 앞에 유지한다. Rust는 이 검사를 allocation 없이 수행하며 C NAME block·hash·symbol table을 도입하지 않는다. enqueue diagnostic phase/span/word index를 유지한다.

Windows name differential에 254/255/256/257, 32766/32767 경계, direct/indirect chain, 크기와 malformed text가 동시에 있는 경우 및 by-value 형태를 추가했다. 길이 오류와 spelling 우선순위도 C와 정확히 비교한다. valid locale/by-value/debug lookup은 계속 Unsupported이며 locale 실행/abandon 효과를 구현하지 않는다. numeric notation의 전체 유효성은 NV3d에 남긴다. 이 단계는 NAME 길이·오류의 수렴이며 전체 NV3/NV5 완료가 아니다.

**NV3c 검증:** native Windows default/portable 각각 **467 passed / 17 ignored**, fmt/clippy/build 통과; Python **30 passed**. 양 DLL 각각 name syntax **4,125 cases / failed 0**: simple name **1,135**, valid Unsupported name **1,927**, invalid name **1,022**, length limit **40**, spelling precedence **1**. 기존 4,030건에 길이/우선순위 fixture **95건**을 추가했다. spelling **667 / failed 0**, vocabulary POS **143**/binding **140**/noun **3**, 세 runtime 경로 각각 **5,380 cases / 5,380 passed / failed 0**, stages **10,810**, words **6,623**, Scan **285 / failed 0**를 유지했다. runtime prefix 경계 **285 / executable prefix passes 0**, capture graph 경계 **257**, static 경계 **2**는 별도다. 전체 보고서 18개의 binary/source/DLL hash를 확인했다. lexical/길이 비교는 locale runtime 성공을 뜻하지 않는다. Linux/GitHub CI/CUDA는 실행하지 않았다.

<a id="nv3d1-numeric-recognition"></a>

**NV3d1 숫자 word 문맥과 오류 검증 — 2026-10-05.** `src/numeric_input.rs`는 enqueue에서 `wn.c::numcase/connum`의 whole-word dispatch를 검증한다. numeric list의 모든 field가 공유하는 complex/based·extended integer·rational·precision 선택을 보존한다. 한 field만 따로 해석하면 `1x`와 `1.0 1x`, `1j2 1x`, `2b10 1x`의 차이를 놓친다. suffix/operand·rational infinity·rectangular/polar complex·based digit·p/x exponent 표기를 검증한 뒤 실제 malformed word는 ill-formed number로 보고한다. 숫자 family 문자 하나의 존재만으로 Unsupported를 선택하던 heuristic은 제거했다. `1xr2`는 C의 `numfd`가 `r2`의 생략된 numerator를 0으로 읽으므로 valid임을 회귀에 포함한다.

검증 상태 Valid/Invalid/Unknown을 구분한다. 검증된 extended/rational/complex/based 표기의 payload 생성은 아직 Unsupported이다. precision과 플랫폼-specific `strtod` hex/NaN payload 등 완전히 검증하지 않은 문법도 별도 Unsupported reason을 유지하며 Invalid로 추측하지 않는다. half/single 및 일부 quad 조합은 supplied C 자체의 nonce boundary이므로 C 성공 또는 J spelling/number 오류로 세지 않는다. 일반 integer/decimal은 기존 constructor로 바로 넘기므로 추가 float parse/normalized string allocation을 하지 않는다. 잘못된 숫자의 enqueue phase·word index·span을 유지하며 runtime target/array IR/physical allocation 정보를 숫자 문법에 도입하지 않는다.

`tools/numeric_syntax_conformance.py`가 scalar 표기와 교차 numeric lists, 잘못된 suffix/missing operand, infinity, 64-bit overflow, colon spelling 우선순위, precision/platform boundaries를 양 DLL과 비교한다. accepted noun controls, verified lexical errors, valid payload boundaries, unresolved recognition/error boundaries, C reference precision boundaries를 분리한다. latter boundaries는 정확한 오류 비교 pass 또는 numeric payload 실행 지원이 아니다. 표준 Windows runner에 추가하고 `wn.c`/`w.c`/`ws.c`/`jerr.h` 및 실제 binary/DLL hash를 보고서에 보존한다.

**NV3d1 검증:** native Windows default/portable 각각 **469 passed / 17 ignored**, fmt/clippy/build 통과; Python **30 passed**. 양 DLL 각각 numeric syntax **1,099 cases / failed 0**: accepted noun controls **110**, lexical error equality **652**, valid payload 경계 **327**, unresolved recognition **5**, C reference precision **4**, unresolved error **1**. 마지막 경계 1건은 C의 ill-formed number와 Rust의 Unknown/Unsupported 차이를 보존한 미완료 검증이며 pass로 바꾸지 않는다. 기존 name syntax **4,125 / failed 0**, spelling **667 / failed 0**, vocabulary POS **143**/binding **140**/noun **3**, 세 runtime 경로 각각 **5,380 cases / 5,380 passed / failed 0**, stages **10,810**, words **6,623**, Scan **285 / failed 0**를 유지했다. runtime prefix 경계 **285 / executable prefix passes 0**, capture graph 경계 **257**, static 경계 **2**는 별도다. 전체 보고서 20개의 binary/source/DLL hash를 확인했다. numeric syntax 검증은 exact/complex/based payload 실행이나 full precision 성공을 뜻하지 않는다. Linux/GitHub CI/CUDA는 실행하지 않았다.

<a id="nv3d2a-quad-hex-recognition"></a>

**NV3d2a quad·Windows hex 문법 검증 — 2026-10-05.** `src/numeric_input.rs`에서 whole-word precision 선택을 유지하면서 `numfq`의 mantissa·fractional scale·소문자 `e`·64-bit exponent·`fq` suffix 및 infinity/NaN 표기를 검증한다. `2fqz`는 이제 C와 같은 ill-formed number다. exponent 자체의 범위 초과와 scale 합산의 signed overflow를 구분하며, 후자는 Unknown으로 보존한다. quad payload 생성과 arbitrary-precision 할당의 자원 오류까지 구현한 것은 아니다.

Windows `strtod`의 hex mantissa·선택적 binary exponent를 Rust에서 검증한다. C `numfd`는 nominal field 끝에 NUL을 넣지 않고 `t >= s+n`을 허용한다. 따라서 `0Xad90`/`0Xb1`은 뒤의 hex digit까지 읽어 유효할 수 있으며, `_0X0ad90`은 magnitude가 음수인 비영 값이 되어 거부된다. 반면 `numbpx`는 `p`/`x` 구분자를 임시 NUL로 바꾸므로 그 앞의 읽기 범위는 좁혀야 한다. field 길이와 실제 읽기 범위를 별도로 전달하여 이 차이를 보존한다. C FFI나 C kernel 의존성을 추가하지 않는다.

음수 hex polar magnitude의 선행 bit/exponent가 기본 IEEE binary64 환경에서 비영 값을 입증할 때만 ill-formed number로 판정한다. 이 checkpoint에서는 0 mantissa를 허용하고, underflow로 음수 0이 될 수 있는 값·합산 overflow·입증하지 못한 hex ratio 부호를 Unknown으로 남겼다. 기본 반올림의 음수 0과 NaN word 경계는 아래 NV3d2b1에서 추가 검증했다. 변경된 rounding/FTZ 환경, parenthesized NaN payload의 word formation, 전체 플랫폼 `strtod` 확장, 숫자 construction의 자원/오류 동등성은 **NV3d2b**다. 유효한 complex/based/quad 표기의 payload 생성은 여전히 Unsupported다.

근거: pinned [wn.c](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wn.c)의 `numfd`, `numfq`, `numj`, `numbpx`; 실제 Windows DLL과 source revision은 별도로 기록한다. NV3d1의 수치는 당시 검증 기록이며, 당시 미확인 quad/hex 경계는 본 단계에서 아래와 같이 갱신했다.

**NV3d2a 검증:** native Windows default/portable 각각 **471 passed / 17 ignored**, fmt/clippy/build 통과; Python **30 passed**. 양 DLL 각각 numeric syntax **2,078 cases / failed 0**: accepted noun controls **182**, lexical error equality **1,084**, valid payload 경계 **607**, integer conversion 경계 **2**, C reference precision 경계 **200**, unresolved recognition **3**. unresolved error 경계는 **0**이다. `frontend_probe`는 Unsupported의 원문 이유를 별도 진단 field로 제공한다. 검증된 문법·integer overflow conversion·C precision 미지원·미확인 문법을 실제 진단 이유로 분류하며, 구문 표본의 scope만으로 valid를 주장하지 않는다. 경계 수를 숫자 실행 성공으로 합산하지 않는다. 기존 frontend/runtime/name/spelling/vocabulary/Scan 비교를 유지하고 보고서 20개의 source/DLL/binary hash를 확인했다. runtime prefix 경계 **285 / executable prefix passes 0**, capture graph 경계 **257**, static 경계 **2**는 별도다. Linux/GitHub CI/CUDA는 실행하지 않았다. NV3d/NV3d2 전체 완료로 표시하지 않는다.

<a id="nv3d2b1-rounding-nan-words"></a>

**NV3d2b1 기본 반올림과 NaN word 경계 — 2026-10-05.** `hex_nonnegative`는 실제 읽기 범위에서 mantissa의 선행 bit와 나머지 비트를 검사한다. 기본 IEEE binary64 round-to-nearest, ties-to-even에서 `2^-1075` 이하의 음수 magnitude는 `-0`으로 반올림되어 polar 입력으로 유효하다. 정확한 중간값보다 큰 magnitude는 음수 비영 값이므로 ill-formed number다. `_0X1P_1075ad90`과 `_0X1P_9999ad90`은 이제 문법이 확인된 payload 미지원이고, `_0X1.00000000000001P_1075ad90`은 C와 같은 오류다. 긴 mantissa의 끝에 있는 sticky bit도 버리지 않는다. 숫자 payload를 생성하거나 C FFI를 호출하지 않고 이 부호 조건만 검증한다.

`1jNaN(1)`, `1jnan()`, `1jNAN(foo)`, `1j_nan(1)`은 C와 Rust 양쪽에서 숫자 prefix·괄호·선택적 내부 word로 나뉜다. Windows `strtod`의 parenthesized NaN payload 문법을 J 숫자 word로 도입하지 않는다. C의 전체 문장은 syntax error지만 Rust는 앞의 complex noun 생성 미지원에서 멈춘다. 따라서 네 표본은 word formation equality와 **payload/parser coverage boundary**이며 syntax-error 동등성이나 parser 실행 성공으로 집계하지 않는다. 부호 없는/음수 NaN, Infinity, ratio 표기를 별도 숫자 표본에 추가한다.

이 단계는 기본 반올림 환경에서 polar 부호를 검증한 범위다. 외부에서 변경한 rounding/FTZ 환경은 검증하지 않았다. 문법이 확인된 매우 큰 hex exponent도 i128 중간 계산과 부호 보존 saturation으로 polar 부호만 판정한다. 지원하는 64-bit host의 mantissa 길이 보정은 i128 범위보다 작으므로 임계값과의 순서는 보존된다. 이를 실제 숫자 payload 생성 규칙으로 사용하지 않는다. quad fractional-scale 합산 overflow, hex ratio의 변환 후 부호 및 arbitrary-precision payload의 할당/자원 오류 동등성은 **NV3d2b2**로 남긴다. C의 signed overflow를 Rust에서 재현하거나 시스템 메모리를 소진시켜 자원 오류를 추측하지 않는다. 관련 source 근거는 pinned [wn.c](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wn.c)의 `numfd`·`numj`·`numfq`·`numxTEMP`, word formation은 [w.c](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/w.c)다.

**NV3d2b1 검증:** native Windows default/portable 각각 **473 passed / 17 ignored**, fmt/clippy/build 통과; Python **30 passed**. 양 DLL 각각 numeric syntax **2,201 cases / failed 0**: accepted noun controls **182**, lexical error equality **1,140**, valid payload 경계 **672**, integer conversion 경계 **2**, C reference precision 경계 **200**, unresolved recognition **1**, NaN word formation 경계 **4**. unresolved error 경계는 **0**이다. 남은 recognition 1건은 `2.1e_9223372036854775808fq`의 scale/exponent 합산 overflow이며 실행 성공이나 정확한 오류 비교 pass가 아니다. NaN 4건은 C syntax error와 Rust payload Unsupported를 별도로 기록한다. 기존 세 runtime 경로 각각 **5,380 cases / 5,380 passed / failed 0**, stages **10,810**, words **6,623**, name syntax **4,125**, spelling **667**, vocabulary POS **143**/binding **140**/noun **3**, Scan **285 / failed 0**를 유지했다. runtime prefix 경계 **285 / executable prefix passes 0**, capture graph 경계 **257**, static 경계 **2**는 별도다. 보고서 20개의 source/DLL/binary hash를 확인했다. Linux/GitHub CI/CUDA와 변경된 floating-point 환경 검증은 실행하지 않았다.

<a id="nv3d2b2a-exact-hex-ratios"></a>

**NV3d2b2a exact hex ratio와 quad 생성 경계 — 2026-10-05.** polar magnitude에 ratio가 있으면 원래 문자열의 부호만으로 유효성을 판단하지 않는다. `real_value`에 실제 읽기 범위를 전달하고 정확히 binary64로 표현 가능한 hex 피연산자를 내부 부호 검증에 사용한다. u64로 누적 가능한 mantissa에서 trailing zero bit를 제거한 뒤 최대 53개의 유효 bit, normal exponent 범위 또는 정확한 subnormal 배수를 입증한다. 조건을 만족할 때만 `f64::from_bits`로 정확한 내부 피연산자를 만든다. 이는 J noun/complex/quad payload 실행 지원이 아니다. 누적 범위 초과·추가 반올림·hex overflow/underflow 피연산자의 수치 생성은 보수적으로 미지원으로 남긴다.

C `numfd`의 비율 계산을 따라 분모 0은 numerator/denominator의 sign xor로 signed zero 또는 infinity를 만들며, 그 외에는 나눗셈 결과에 `0 <= magnitude` 조건을 적용한다. `_0X1r2ad90`은 오류, `_0X1r_2ad90`은 유효하고 `_0X1P_1074r2ad90`은 결과가 `-0`으로 반올림되어 유효하다. NaN 결과는 거부한다. 분모도 nominal field 뒤의 hex digit을 읽을 수 있으므로 `0X1r0X0ad90`의 분모를 0으로 단정하지 않는다. C FFI를 추가하지 않고 기존 enqueue 오류 span/index와 whole-word numeric mode를 유지한다.

`2.1e_9223372036854775808fq`는 lexical grammar 미확인 대신 **quad scale/exponent construction boundary**로 분류한다. mantissa·suffix·exponent 문법이 확인되어도 C의 signed scale 합산 overflow, 숫자 생성·할당/자원 오류 동등성까지 입증된 것은 아니다. Unsupported 이유와 보고서 category를 분리하며 정확한 J 오류 비교나 숫자 실행 pass로 집계하지 않는다. malformed field가 함께 있으면 기존 ill-formed number 우선순위를 보존한다.

근거: pinned [wn.c](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/wn.c)의 `numfd` ratio·signed-zero 처리, `numj`의 polar nonnegative 조건, `numfq` scale 계산과 `numxTEMP` 자원 오류다. 남은 비정확 hex ratio·변경된 rounding/FTZ 환경·quad scale overflow의 정의·payload allocation/자원 오류 동등성은 **NV3d2b2b**다. 해당 경계는 RustJ 구현 범위이며 J 언어의 제한으로 만들지 않는다.

**NV3d2b2a 검증:** native Windows default/portable 각각 **474 passed / 17 ignored**, fmt/clippy/build 통과; Python **30 passed**. 양 DLL 각각 numeric syntax **2,485 cases / failed 0**: accepted noun controls **182**, lexical error equality **1,244**, valid payload 경계 **850**, integer conversion 경계 **2**, C reference precision 경계 **200**, quad construction 경계 **1**, NaN word formation 경계 **4**, unresolved recognition **1**, unresolved error **1**. 마지막 두 경계는 각각 `_0X1P_1075r1ad90`과 `0X1P9999r0X1P9999ad90`이며, C의 성공/ill-formed number와 Rust의 미지원 차이를 그대로 기록한다. 미확인 경계를 오류 동등성이나 실행 pass로 바꾸지 않는다. exact operand 교차 ratio **280개**, 미확인 conversion **2개**, malformed/quad construction 우선순위 **2개**를 기존 corpus에 추가했다. 기존 frontend/runtime/name/spelling/vocabulary/Scan 비교를 유지하고 보고서 20개의 source/DLL/binary hash를 확인했다. runtime prefix 경계 **285 / executable prefix passes 0**, capture graph 경계 **257**, static 경계 **2**는 별도다. Linux/GitHub CI/CUDA·변경된 FP 환경·메모리 소진 테스트는 실행하지 않았다.

<a id="vocabulary-migration-checklist"></a>

<a id="nv--현재-j-vocabulary-수렴"></a>
### NV — 현재 J vocabulary 수렴

설계 inventory의 정본은 [현재 vocabulary 감사](./15-advanced-contracts-p09.ko.md#current-j-vocabulary)다. M2 frontend 순서에 연결하고 신규 task/fold/GPU executor를 동시에 구현하지 않는다.

- [x] **NV0** NuVoc current index·관련 페이지를 읽고 source spelling/POS·C DLL provenance와 분리한다. 이전 vocabulary matrix의 Taylor·obsolete 항목과 누락된 현행 form을 수정한다.
- [x] **NV1** `[.`·`].`·`]:`를 정상 core enqueue·shared parser constructor 경로로 지원하고 noun/verb 결과·NAME snapshot/late lookup·modifier train·discarded noun 효과/오류 회귀를 추가한다.
- [x] **NV2** pinned core inventory의 spelling/POS 인식을 확장하고 semantic construction/실행 capability와 분리한다. descriptor 108개와 실제 noun `a.`/`a:`를 추가했다. C가 수용한 143개 POS, bare function binding/AR 140개와 noun payload 3개가 양 DLL에서 일치했다. inventory pass를 실행 지원으로 승격하지 않는다.
- [ ] **NV3** invalid/obsolete spelling의 정확한 J 오류를 C `spellin`/enqueue와 대조해 일반화한다. valid 미지원 primitive와 invalid spelling을 구별하며 임의 예외 목록으로 해결하지 않는다.
- [x] **NV3a** fixed ASCII spelling과 미등록 inflection의 enqueue 오류를 일반화했다. 현대 core dictionary를 우선 조회하고 obsolete 예외 목록 없이 C 오류 분류를 따른다. `name_:`와 유효 미지원 numeric family는 별도 coverage 경계이며 전체 NV3는 미완료다.
- [x] **NV3b** bounded direct/indirect/debug-frame/by-value 이름 문법을 shared enqueue에서 검증한다. valid lookup은 Unsupported이며 NAME 길이 제한은 NV3c에서 다루고 locale 실행·numeric grammar를 남긴다.
- [x] **NV3c** NAME 전체·simple-name·locale storage 길이 제한과 enqueue 오류 우선순위를 C `nfs` 및 양 Windows DLL로 검증했다.
- [ ] **NV3d** numeric grammar의 valid 미지원 family와 실제 ill-formed number를 C `connum`/`wn.c`로 구별한다.
- [x] **NV3d1** whole-word numeric family 선택과 검증된 extended/rational/complex/based 표기의 오류를 일반화했다. payload 생성 미지원과 문법 Unknown을 구분한다.
- [ ] **NV3d2** dedicated quad grammar·플랫폼별 `strtod` 확장 및 숫자 construction의 resource/error 경계를 검증한다. Unknown 문법을 실제 Invalid로 추측하지 않는다.
- [x] **NV3d2a** bounded quad·Windows hex 문법과 field/read-window 차이를 양 DLL로 검증했다. payload 실행 지원과 구분한다.
- [ ] **NV3d2b** resource·scale overflow·rounding/FTZ·NaN payload 및 남은 플랫폼 conversion 경계를 검증한다.
- [x] **NV3d2b1** 기본 ties-to-even의 hex polar 부호와 NaN 괄호 word 경계를 양 DLL로 검증했다. 매우 큰 exponent의 부호 판정과 숫자 payload 생성은 분리한다.
- [ ] **NV3d2b2** quad scale overflow·hex ratio 부호·변경된 FP 환경·payload 할당/자원 오류 동등성을 검증한다. 숫자 construction 미지원을 J 오류로 바꾸지 않는다.
- [x] **NV3d2b2a** exact hex 피연산자의 polar ratio 부호·signed zero·분모 읽기 범위와 quad 생성 경계를 양 DLL로 검증했다.
- [ ] **NV3d2b2b** 추가 반올림/overflow가 필요한 hex ratio·quad scale 정의·변경된 FP 환경·payload allocation/자원 오류 동등성을 검증한다.
- [ ] **NV4** 누락 family의 valence/rank/constructor/효과·오류 계약을 순차적으로 검토한다. `/..`·Fold·task/pyx·precision·scope의 의미를 단순 alias나 pure array kernel로 축소하지 않는다.
- [ ] **NV5** NuVoc 전체 form·structural/control inventory와 지원 행렬의 수렴을 확인한다. 각 단계마다 Windows 차분 gate를 갱신하고 full J 지원과 제한 corpus 통과를 구별한다.

**NV1 gate (historical, 2026-10-05):** native Windows default/portable 각각 **431 passed / 17 ignored**, fmt/clippy/build 통과, Python **27 passed**. 추가한 공통 구문 **37개**를 포함하여 C 기본·AVX2 각각 세 runtime 경로 **5,368 cases / 5,368 passed / runtime 경계 0 / failed 0**, stage **10,810 checks**, words **6,623 cases**, 실패 0. capture graph **254건**과 static **2건** 경계는 별도다. frontend report 10개와 vocabulary report 2개의 실제 source/reference/binary hash를 확인했다. vocabulary 후보 **145개**에서 **enqueue POS 검증 33 / Rust Unsupported 110 / source code-only 거부 후보 2 (`?:`, `` `. ``)**이며 legacy/invalid 표기 8개는 양 DLL의 spelling error를 확인했다. 이 숫자는 전체 NuVoc/J 실행 지원률이 아니다. 이 gate 당시 NV2–NV5와 DB1–DB7은 미완료였으며 ordered effect graph·정적 discarded-noun 보존 경계, 미지원 task/fold/precision/범용 scope 실행을 분리한다. optimizer/CUDA/Linux 실행 테스트/GitHub CI는 계속 유보한다.

**NV2 gate (2026-10-05):** native Windows default/portable 각각 **435 passed / 17 ignored**, fmt/clippy/build 통과, Python **27 passed**. 공통 C 구문 12개를 추가해 양 C DLL의 세 runtime 경로 각각 **5,380 cases / 5,380 passed / runtime 경계 0 / failed 0**, stage **10,810 checks**, words **6,623 cases**, 실패 0. capture graph **257건**, static **2건** 경계는 별도다. vocabulary 후보 145개 중 **enqueue POS 143 / Rust enqueue 미지원 0 / code-only 거부 후보 2**이며 **bare function parser binding/AR 140개 / noun payload 3개**를 비교했다. 전체 NuVoc/J 실행 지원률이 아니다. report 12개의 실제 DLL/binary/source hash를 확인하고 NV2를 완료 처리했다. 다음 단계는 NV3이며 NV4/NV5와 DB1–DB7은 미완료다.

<a id="m--현재-구조-수렴-실행-순서"></a>
### M — 현재 구조 수렴 실행 순서

이 상위 체크리스트는 **지금 어떤 순서로 구조를 수렴시킬지**를 추적한다. 세부 완료 조건은 아래 A/F/P/G 체크리스트를 그대로 사용하며, 같은 일을 중복 정의하지 않는다.

핵심 목표는 새 계층을 더 만드는 것이 아니라 현재 공존하는 과도기 경계를 제거해 다음 canonical pipeline으로 수렴하는 것이다.

```text
J source
  → jsource-compatible frontend
  → FunctionEntity / J Semantic Construction IR
  → J Graph IR / Graph Basis
  → Execution Semantic Lowering
  → canonical logical_ir::Plan / Execution Basis
  → Route Partition
  → Schedule / Transform
  → Physical Planner / Bufferization
  → Physical Execution Plan
  → Executor
```

Logical J value와 physical representation의 분리는 이 전체 pipeline에 교차하는 불변식으로 유지한다.

<a id="m0--구조-기준선-고정"></a>
#### M0 — 구조 기준선 고정

- [x] 목표 compiler stage와 각 stage의 책임을 문서에서 확정했다.
- [x] Graph Basis와 Execution Basis를 별도 계층으로 분리했다.
- [x] Logical Array/J noun과 Physical Array/representation을 별도 계층으로 분리했다.
- [x] 과거 중복 execution IR 경계를 구조 부채로 식별했고 M1에서 제거했다.
- [x] `physical.rs`는 read-only CPU affine representation 기반과 제한적 검색 알고리즘 선택기를 제공하지만 전체 Physical Plan/bufferization planner는 아직 없음을 명시했다.
- [x] `runtime.rs`의 `ResolvedVerb { reduce, rank, ... }` flattening은 과도기 runtime 구현이며 canonical semantic/compiler model이 아님을 확인했다.
- [x] ArrayFire와 `math_arrayfire`를 semantic oracle이 아니라 execution/fusion/adapter 참고 구현으로 배치했다.
- [x] compiler module ownership/dependency 표를 코드 구조와 맞춰 확정하고 reverse dependency 금지선을 문서화했다.

현재/목표 module ownership은 다음을 기준으로 한다.

| 책임 | 현재 주요 모듈 | canonical owner / 목표 | 금지되는 역방향 의존 |
|---|---|---|---|
| word formation | `scanner.rs` | frontend word former | graph/logical/physical/runtime가 scanner 구현 세부에 의존하지 않음 |
| enqueue/word interpretation | `enqueuer.rs` | frontend enqueuer | target/backend 정보를 enqueue가 읽지 않음 |
| parser + semantic construction | `semantic.rs` | frontend/parser + immutable `FunctionEntity` | Graph/Logical/Physical 선택을 parser가 소유하지 않음 |
| J graph algebra | `j_graph_ir.rs`, `j_graph_rewrite.rs`, `j_graph_resource.rs`, `j_graph_memory.rs` | J Graph IR / Graph Analyzer | Logical/Physical plan을 다시 semantic identity로 역주입하지 않음 |
| execution semantic contract | `execution_semantics.rs` | 독립 target-independent execution-semantics contract | schedule/buffer/device를 포함하지 않음 |
| canonical Logical Execution IR | `logical_ir.rs` | `logical_ir::Plan` | `physical.rs`, kernel/runtime concrete storage에 의존하지 않음 |
| compilation aggregate | `compilation.rs` | cross-stage analysis bundle | lowering semantics 자체를 소유하지 않음 |
| route legality/capability | `lowering.rs` | verified Logical IR 이후 lowering/route layer | semantic/parser를 target 편의에 맞게 변경하지 않음 |
| schedule/transform | 아직 없음 | 별도 planner-side representation | canonical Logical IR을 destructive하게 schedule-specific IR로 덮지 않음 |
| physical representation | `physical.rs`, `storage.rs` | representation layer | semantic facts를 physical layout으로 정의하지 않음 |
| Physical Plan/bufferization | **전체 planner는 아직 없음** (CPU 검색 strategy selector만 구현) | Physical Planner | J parser/FunctionEntity를 직접 해석하지 않음 |
| backend kernels | `kernels.rs`, `numeric.rs`, `simd.rs` 등 | backend realization | kernel 구현 세부가 semantic legality를 정의하지 않음 |
| interpreter/reference runtime | `runtime.rs`, `logical_executor.rs` | transitional/reference execution | compiler canonical IR의 의미를 runtime flattening으로 정의하지 않음 |

M1 transition container는 제거되었다. `analysis`의 remaining re-export는 target-independent execution-semantic contract와 compilation aggregate compatibility surface이며 canonical Logical IR container를 소유하지 않는다.

M0 이후 적용할 dependency 방향:

```text
frontend
  ↓
semantic FunctionEntity
  ↓
J Graph IR
  ↓
execution semantic contracts
  ↓
logical_ir::Plan
  ↓
route / schedule
  ↓
physical plan / representation
  ↓
backend / executor
```

옆 단계의 provenance/type identity 참조는 허용하지만, **아래 단계의 concrete realization 정보가 위 단계의 semantic identity를 결정하는 dependency는 금지**한다.

**M0 완료 조건:** **완료.** 새 구현이 어느 stage에 속하는지 한 곳으로 결정할 수 있고, 과도기 compatibility bridge를 새 canonical interface로 오인하지 않는다.

<a id="m1--canonical-logical-execution-ir로-cutover--완료"></a>
#### M1 — canonical Logical Execution IR로 cutover — 완료

`logical_ir::Plan`이 유일한 canonical execution IR이다. 아래는 현재 유지되는 완료 조건이며 제거한 과도기 container를 현행 모듈처럼 설명하지 않는다.

- [x] shared target-independent execution-semantic contract를 `execution_semantics.rs`로 분리했다.
- [x] `analysis::lower_graph()`가 J Graph IR에서 A3 op/value/check를 직접 생성한다.
- [x] `Plan::from_transition`, `TransitionProjection`, `transition_ir` module/container 및 `analysis::LogicalPlan`을 제거했다.
- [x] `CompilationAnalysis`는 `j_graph`, rewrite/resource views와 canonical `logical`만 묶으며 `transition` 필드가 없다.
- [x] `Engine::analyze`, `analyze_a3`, `analyze_diagnostic`은 canonical Logical IR을 반환한다.
- [x] verifier/reference-executor/lowering test consumer를 canonical `logical_ir::Plan`으로 전환했다.
- [x] Graph origin/source span, name version, semantic check와 write ordering의 direct-A3 regression을 추가했다.

**M1 완료 조건:** 충족. `J Graph IR → logical_ir::Plan` 직접 연결과 단일 execution container가 유지되어야 한다. 이 완료는 full frontend, implicit cell semantics 또는 native Physical Executor의 완료를 뜻하지 않는다.

<a id="m2--jsource-compatible-frontendparser-cutover"></a>
#### M2 — jsource-compatible frontend/parser cutover

목표: 현재 heuristic parser를 jsource-compatible Word Formation → Enqueue → 9-row Parser pipeline으로 교체한다.

- [x] F0 differential 0-mismatch 기록을 완료했다. pinned source 기록과 Windows 일반/AVX2 배포본 재검증을 구분한다.
- [ ] F1 Enqueuer/PrimitiveResolver를 완료한다.
- [ ] F2 Parse Queue를 완료한다.
- [x] P1 parser control class와 semantic entity/value를 분리했다. parser-time lookup/effect sequencing의 완성은 P4/P2의 별도 gate다.
- [ ] P2 하나의 9-row reduction engine으로 전환한다.
- [ ] P3 modifier/Hook/Fork/bident/trident construction semantics를 연결한다.
- [ ] P4 parser-time name resolution/assignment sequencing을 연결한다.
- [ ] P5 construction semantics와 compiler facts 경계를 완료한다.
- [ ] P6 differential/conformance gate를 통과한다.
- [ ] P7 legacy parser heuristic(`reduce_modifier_applications`, `collapse_verb_trains` 중심 경로)을 제거한다.
- [ ] P8에서 canonical FunctionEntity → J Graph IR → Logical IR handoff를 재검증한다.

**M2 완료 조건:** supported frontend domain의 parser reduction을 jsource row/semantic action으로 설명할 수 있고 compiler가 별도 언어 문법을 갖지 않는다.

<a id="m3--logicalphysical-array-경계의-코드-수렴"></a>
#### M3 — Logical/Physical Array 경계의 코드 수렴

목표: logical value identity에 CPU/GPU/layout identity가 역류하지 않게 하고 representation 선택을 physical planning으로 이동한다.

- [x] **M3-RB 설계 명세(2026-10-07):** §2.5.1의 RB-01~08 의미론 보존·proof/guard·source coverage·Write commit 및 RB-V0~V4 검증 기준을 확정했다. **문서만 완료**.
- [ ] **M3-RB 실행 검증(구현 보류):** 원본 A3 독립 대조, RouteBoundary validator, Check/effect/error/order negative tests, J oracle 차분 결과를 실행 후 수용한다. 현재 `partition_plan`은 분석 후보 분류기다.
- [x] **M3→M4 인계 설계(2026-10-07):** §5.2.1 H-01~09, HM-V0~V4로 RouteVerified 증거 → PhysicalVerified → RuntimeReady의 소유권·검증 의무를 문서화했다. **설계만 완료, 신규 구현 없음.**
- [ ] **M3→M4 인계 검증(미구현):** 원본 A3-Route-Physical Check/Write coverage, selected realization, guard/freshness 및 negative/differential 테스트의 실제 통과 증거 확보.
- [x] `LayoutFact`를 `RepresentationClassFact`로, `Facts.layout`을 `Facts.representation_class`로 바꿔 physical layout과 구분했다.
- [ ] Dense/Boxed/Sparse처럼 J-visible representation semantics와 row-major/column-major/stride/tile/device 같은 physical representation을 타입/API에서도 구분한다.
- [ ] 현재 `Value::Data`의 dense `CpuStorage` 직접 소유를 migration artifact로 한정하고, canonical compiler value identity가 CPU backing을 요구하지 않게 한다.
- [ ] sparse의 J-visible axes/fill/semantic representation과 concrete coordinate/value buffer encoding의 경계를 점검한다.
- [ ] `PhysicalArray`는 BufferId/lease/shape mapping/stride/offset/encoding 같은 representation-only 책임만 갖게 유지한다.
- [ ] 같은 logical value의 복수 physical representation과 여러 logical value의 safe buffer reuse를 표현할 planner-side identity를 정의한다.
- [ ] G2 structural view 작업을 이 경계 위에서 구현한다.

**M3 완료 조건:** Logical IR/semantic facts에는 stride/offset/device/buffer가 없고, physical representation 변경이 J value identity를 바꾸지 않는다.

<a id="m4--최소-rustj-native-cpu-vertical-slice"></a>
#### M4 — 최소 RustJ-native CPU vertical slice

목표: optimizer가 똑똑하지 않아도 canonical compiler pipeline이 end-to-end로 실제 실행되게 한다.

- [ ] Logical payload와 분리된 최소 `Schedule/TransformPlan`을 정의한다.
- [ ] 첫 planner는 비용 최적화 없이 deterministic all-CPU policy를 사용한다.
- [x] **문서 계약:** §5.2.1에서 최소 Physical Plan op를 `BindInput/Check/View/Materialize/Kernel/Return`으로 정의하고 plan-time/runtime identity·verifier·cleanup 경계를 고정했다.
- [x] **M4 인계 검증 계약(2026-10-07):** §5.2.1의 H-01~09/HM-V0~V4에서 물리화 전 반드시 필요한 M3 승인, buffer/view/lease, Check/guard/오류 순서와 runtime 재검증을 명문화했다. **문서 완료만** 의미한다.
- [x] **M3→M4 첫 CPU 경로 승인 판정 명세(2026-10-07):** §5.2.1 H-A의 A0~A5 단계와 HA-V1~7로 `ReferenceSequential` 후보와 실제 compiler-native 실행을 분리하고 Check/Write/guard/region-local 승인 부정 검증을 정의했다. **설계 전용**.
- [x] **M3→M4 인수 패킷·검증 판정표(문서 전용, 2026-10-07):** §5.2.1 H-P의 HP-01~12 입력 증거/검증/거부 및 HP-V01~09 단일 조건 변조 시험·판정 기록 양식 작성. 코드 구현이나 전역 승인과 무관.
- [ ] **HP-01/02 원본·커버리지:** immutable A3 identity와 모든 original op/zero-result Check/별도 Write·cross-route interface를 source 보존 증거에 매달아 검증한다(HP-V01/02).
- [ ] **HP-03/04 native capability·Check:** selected 실제 CPU recipe 존재, 원본 Check의 execute/discharge owner 및 첫 J error precedence를 검증한다(HP-V03).
- [ ] **HP-05/06 Name·semantic value:** noun snapshot/late NameRef/Write owner 및 zero-frame Rank·boxed/sparse·shape/atom J 의미를 보존한다(HP-V04).
- [ ] **HP-07/09 buffer·completion:** plan/runtime 버퍼 구분, view span/alias/generation/last-use, 동기 CPU completion 근거를 검사한다. 비동기 토큰/전송 구현은 M5/M6까지 필수 아님(HP-V05/07).
- [ ] **HP-08 dependency:** producer/check/guard/effect/error 선행 edge를 검증하고 누락·순환을 거부한다(HP-V06).
- [ ] **HP-10/11 runtime·fallback:** guard freshness/효과 전 대체 경로/자원 한계, CPU+Host+zero-transfer 제한을 검증한다(HP-V08).
- [ ] **HP-12 acceptance:** 실제 native 결과와 RustJ reference·가능한 J C oracle의 차분 및 부정 테스트 PASS/CI 증거를 gate별 기록한다(HP-V09).
- [ ] **M3→M4 승인 판정 실행 검증(미착수):** HA-V1~7 verifier·guard 연결 및 jsource/semantic/native 차분 테스트의 실제 PASS 기록.
- [ ] **구현:** 위 contract를 concrete `PhysicalPlan`/op Rust 타입과 verifier로 구현한다.
- [ ] logical ValueId → plan-time `PlanBufferId`/PhysicalView → runtime `BufferLease/BufferId` binding을 구현한다.
- [ ] G2 transpose/reverse/slice/compatible reshape/zero-stride agreement view를 planner에서 선택 가능하게 한다.
- [ ] G3의 첫 kernel로 contiguous/fixed/general-stride add를 연결한다.
- [ ] G3 cell mapping과 ExecutionBasis `CellApply`를 physical view iteration에 연결한다.
- [ ] G4 CPU Physical Executor를 구현한다.
- [ ] `source → FunctionEntity → J Graph → logical_ir::Plan → Physical Plan → CPU Executor` vertical test를 만든다.
- [ ] 기존 semantic/reference executor와 결과/error contract를 비교한다.

**M4 완료 조건:** 기존 interpreter 직접 실행을 거치지 않는 최소 compiler-native CPU 경로가 하나 이상 동작한다.

**PR #1 IL1/IL2 통합 경계:** `RankSpec`/`RankContract`와 `CallOp::cell_application(left_facts, right_facts)`는 현재 immutable FunctionEntity의 검증된 requested rank 및 primitive registry innate rank를 소비한다. 중첩 explicit/innate layer, J prefix agreement·residual repetition·iteration count·zero-frame prototype을 별도 관찰한다. 이전 draft의 분석기/SSA 교체 대신 현재 J Graph → A3 경계를 보존하는 재계산 inspection API로 이식한다. 기존 RankPlan·facts 추론·runtime/physical execution은 유지한다. unknown shape/rank는 None이며 조립/오류/최적화 legality나 실행 지원을 증명하지 않는다. L4 전체 및 M3/M4는 미완료다.

<a id="heterogeneous-execution-checklist"></a>

**M4-P0 제한된 코드 통합(PR #4):** `physical_plan`은 plan-time buffer/view ID와 runtime BufferRegistry를 분리하며, 완전히 빈 A3 또는 closed Bool/Int/Float/Char literal 하나의 BindInput → Return만 검증·실행한다. exact literal/source witness로 stale payload/facts/span을 거부하고 singleton stride와 alias를 검증한다. ExtendedInt/Rational/boxed/sparse 및 미지원 op·SemanticCheck·쓰기·이름 조회는 거부한다. Add/Check/View/Materialize/lifetime/reuse와 일반 M3 proof는 미구현이며 M3/M4 전체 완료가 아니다. 현재 런타임 경로는 변경하지 않는다.

<a id="he--이종-cpugpu-실행-계획-수렴-2026-10-07-m4m6-연계-체크리스트"></a>
#### HE — 이종 CPU/GPU 실행 계획 수렴 (2026-10-07, M4→M6 연계 체크리스트)

**설계 결정:** RustJ는 **CPU 멀티코어 컴파일러가 아니라 이종 배열 컴파일러**다. CPU thread 병렬화는 Physical Plan의 device-local 실행 전략일 뿐이다. `ParallelLegality` 또는 CPU 전용 `ParallelPhysicalPlanner`를 독립적인 canonical 상위 단계로 도입하지 않는다. 기존 `J Graph IR → verified logical_ir::Plan → RoutePartition → Schedule/Transform → Physical Planner → Physical Execution Plan → Executor`를 유지한다. 이 체크리스트는 본 §10의 기존 M/IO 이행 계획에 통합되며 별도 Markdown이나 새로운 필수 IR을 만들지 않는다.

**교차 프레임워크 감사 (참조 대상과 채택 경계).**

| 참조 | 실제 근거 | RustJ 도입/비도입 |
|---|---|---|
| [IREE Stream](https://iree.dev/reference/mlir-dialects/Stream/) 및 [stream passes](https://iree.dev/reference/mlir-passes/Stream/) | Flow에서 target affinity, resource, asynchronous scheduling을 명시한 Stream으로 내린 뒤 HAL에서 실제 device execution | Route/physical plan에 region affinity, readiness·completion dependency, ownership/lifetime 계약을 **점진적으로 도입**; IREE 전체 스택 또는 새로운 Stream IR 복제 금지 |
| [Kokkos View](https://kokkos.org/kokkos-core-wiki/ProgrammingGuide/View.html) · [Memory Spaces](https://kokkos.org/kokkos-core-wiki/API/core/memory_spaces.html) | ExecutionSpace와 MemorySpace가 1:1이 아니고 가시성·동기화가 별도 | **장치 실행 위치 ≠ 버퍼 저장 위치**를 별도 물리 사실로; unified/shared memory를 공짜 zero-copy로 간주 금지 |
| [MLIR scf.forall](https://mlir.llvm.org/docs/Dialects/SCFDialect/) · [tensor.parallel_insert_slice](https://mlir.llvm.org/docs/Dialects/TensorOps/) | 논리적인 병렬 반복·mapping·부분 결과 결합을 분리, 반복 간 side effect ordering은 미보장 | A3 `IterationDomain`과 schedule/device mapping 분리 유지. 독립 출력/조립 J 의미 proof 없이 동시화 금지 |
| [XLA CPU ParallelTaskAssignment](https://github.com/openxla/xla/blob/main/xla/service/cpu/parallel_task_assignment.cc) | FLOP/byte 기반 task 수와 thread overhead 평가, 내부 병렬화된 kernel 회피 | CPU backend 내부의 작업량·대역폭 비용 참고; GPU/CPU 장치 배치 비용을 CPU thread threshold로 대체 금지 |
| [Futhark multicore scheduler](https://github.com/diku-dk/futhark/blob/master/rts/c/scheduler.h) · [Rayon](https://docs.rs/rayon/latest/rayon/) | chunk/task amortization, nested work 분배, work stealing | **CPU 실행기 내부** 실행 전략; RustJ logical IR에 thread pool·thread count 인코딩 금지 |

**소유권/경계 계약:**
1. **Semantic/legality:** `logical_ir.rs`의 J 결과 타입·Shape, Rank/CellApply/zero-frame virtual fill, dynamic names/versions, comparison/fit, alias, effect와 error precedence가 권위다. `IterationAxisKind::Parallel`은 구조적 축이며 **동시 실행 가능하다는 증명이 아니다**. 동시 실행·분할·중복 계산은 기존 `lowering.rs` legality와 별도 witness/guard에서 결정한다; Unknown은 거절 또는 합법 baseline 경로로 유지한다.
2. **Route/device:** `RouteRegion`의 후보 장치(CPU/GPU/external)와 intra-device schedule(Sequential/SIMD/CPU workers/GPU grid)은 직교한다. full J에 모든 연산이 동일한 backend에 내려가야 한다는 의무가 없다. 단일 CPU/단일 region/all sequential은 정상적으로 유효한 계획이다.
3. **Placement/memory:** J `ValueId`, plan-time `PlanBufferId`/logical version, runtime `BufferId`/`BufferLease`/memory residency/ready state는 서로 다르다. Host/NUMA/GPU/managed/pinned/file memory 위치가 계산 실행 장치와 별도 축이다. view, no-op, copy, prefetch, migration, materialize 후보를 의미와 비용에 맞춰 고른다. 공유·통합 메모리는 접근권과 coherence/동기화 검증을 없애지 않는다.
4. **Dependencies/completion:** 후보 Physical Execution Plan은 value/data readiness, effect/order, error precedence, lease/lifetime, transfer/compute completion을 추적한다. 비동기 overlap은 dependency의 critical path와 실제 capability가 증명될 때만 이득으로 계산한다. 같은 output에 대한 중복 write, check-before-effect 위반, 오류의 arbitrary first-thread winner를 금지한다.
5. **Resource/cost:** `j_graph_work_depth.rs`와 `j_graph_resource.rs`의 target-independent facts를 소비하되 `ResourceEstimate`(가능 여부), `CostEstimate`(예측), 측정 `CostProfile`(관찰)을 합치지 않는다. compute FLOP·byte, memory bandwidth/capacity·residency, intermediate materialization, transfer bytes/latency, launch/sync, queueing와 peak in-flight bytes를 분리해 비교한다. 리소스 Unknown ≠ feasible, 비용 Unknown ≠ zero.
6. **범위:** 실제 GPU/CUDA·multi-device/async runtime 도입은 기존 M6/CUDA 보류 정책을 따른다. M4 최소 CPU slice에 GPU, NUMA, 새 필수 IR, 범용 혼합 스케줄러를 끼워 넣지 않는다. 외부 MLIR/StableHLO adapter의 독립적인 경로도 유지한다.

**실행 게이트/체크리스트 (이 표 외 중복 작업표 생성 금지):**

| 게이트 / 선행 | 상태 | 수용 기준 |
|---|---|---|
| HE-00 / M2 병행 | [x] 이종 실행 중심 원칙 및 IREE/Kokkos/MLIR/XLA/Futhark/Rayon 역할 문서화 | 본 결정과 `FOUNDATIONS.ko.md`·`AGENTS.md`에 반영한 **설계 완료만** 뜻함; runtime/benchmark 아님 |
| HE-01 / M4 | [ ] all-CPU / single-device / zero-transfer `PhysicalPlan` 구현 | 원래 `BindInput/Check/View/Materialize/Kernel/Return` 순차 baseline, `logical_executor` 및 J oracle 의미 대비; M4 완료 게이트에 포함 |
| HE-02 / M4→M5 | [ ] 실행 위치/메모리 위치/내부 병렬화를 분리한 planner-side contract와 verifier | A3 schema·J Graph·parser 불변, unknown capability와 resource 거절, non-overlap/liveness; multi-device 구현은 요구하지 않음 |
| HE-03 / M5 | [ ] `ExecutionLegality`/witness/guard **분석**을 기존 lowering 경계에 연결 | Effect·NAME version·alias·error precedence·rank fill·dynamic fallback 반례; IR의 Parallel 축만으로 허가하면 실패 |
| HE-04 / M5 | [ ] CPU Sequential/SIMD/worker 후보와 measured cost selection | worker count/tiling/compute bandwidth/launch/pool overhead, 반복 병렬 폭발과 nested oversubscription 방지; 모든 후보 합법성 검사 |
| HE-05 / M5 | [ ] Device/Memory placement·데이터 이동 비용 표현 | copy/migrate/prefetch/ready dependencies, capacity/residency, transfer+sync bytes/time, overlapping 가능 범위 구분; `IO-20`과 공통 physical planner 사용 |
| HE-06 / M5 이후 | [ ] 단일 CPU 실측과 가상의 혼합 candidate의 비용·정합성 비교 | 순차 baseline보다 작업이 적지 않으면 병렬·GPU 경로를 기본 선택하지 않음; cold/warm compile/transfer/performance 분리 |
| HE-07 / M6 이후 | [ ] CPU+GPU runtime completion/transfer 연결 및 end-to-end 검증 | 검증 가능한 실제 GPU와 별도 구현 재개 승인 전 **미착수**. data version, transfer/check order, async failure, invalid device, unsupported route 검증 |
| HE-08 / M6 이후 | [ ] 병렬 CellApply/Reduction/Scan의 J 의미 회귀 및 부정 사례 | zero-frame virtual cell, positive frame/empty cell, result-cell shape/type join, boxed/sparse, tolerance, floating reassociation, J error priority; 미증명 fold/scan은 순차 유지 |
| HE-09 / M5 이후 | [ ] 스케줄 단계별 관찰값·벤치마크 및 diagnostic | route+device+memory+worker 선택 이유, unknown/reject/guard 근거, bytes moved, sync count, peak memory, output J equivalence. GPU 실행 완료로 오인 금지 |

**작업 순서:** HE-00(설계) → 기존 M2→M3 수렴 → M4/HE-01 순차 vertical slice → HE-02/03 legality/representation → HE-04/05/06 cost·CPU worker·transfer 후보 → 별도 승인을 전제로 HE-07/08/09. `IO-20`도 이 범용 physical placement/transfer 모델에 합류하며 두 번째 전송 추상화를 만들지 않는다. 모든 실행·성능 수용은 §11 검증 정책과 실제 결과를 따른다. **이 문서 갱신만으로 HE-01~09를 완료 처리하지 않는다.**

---

<a id="m5--routeschedulecost-확장"></a>
#### M5 — Route/Schedule/Cost 확장

M4 이후에만 optimizer 선택 문제를 키운다. [sw-MLPL SW-05/06 계측 참고](./03-architecture-p02.ko.md#sw-mlpl-reference-checklist)는 기존 physical residency/transfer 비용 검증에만 적용하고 M5/M6 구현 완료를 뜻하지 않는다.

- [ ] `RouteRegion`에 boundary inputs/outputs, chosen route, preconditions/witnesses, semantic provenance를 추가한다.
- [ ] legality와 profitability를 계속 분리한다.
- [ ] StructuralOpportunity/use-def를 schedule candidate와 연결한다.
- [ ] materialize/view/fuse 후보의 physical feasibility를 계산한다.
- [ ] TargetProfile/ResourceEstimate/CostEstimate 최소 schema를 구현한다.
- [ ] cold compile, warm execution, copy, layout conversion, transfer, synchronization 비용을 분리한다.
- [ ] simple CPU cost model로 multiple legal realization 중 하나를 선택한다.

**M5 완료 조건:** planner가 단순 고정 policy가 아니라 여러 합법 physical plan 중 cost/resource 근거로 선택할 수 있다.

<a id="m6--externalarrayfiregpu-route"></a>
#### M6 — External/ArrayFire/GPU route

M4의 compiler-native vertical slice와 M5의 route contract가 안정된 뒤 진행한다. 모든 adapter는 먼저 §5.5.1 External adapter boundary contract를 만족해야 한다.

- [ ] Graph/Execution Basis ↔ ArrayFire capability matrix를 만든다.
- [ ] ArrayFire route의 dtype/rank/shape/layout/J-semantic precondition을 명시한다.
- [ ] J row-major ↔ ArrayFire column-major mismatch를 view/copy/consumer-absorption 선택 문제로 physical planner에 연결한다.
- [ ] external handle lifetime/lock/release/sync를 Physical Plan resource/token 경계로 모델링한다.
- [ ] MLIR adapter와 StableHLO-safe subset adapter의 공통 negotiation interface를 정의한다.
- [ ] external route failure가 J semantic failure가 아니라 route unsupported/fallback으로 처리되는 테스트를 만든다.
- [ ] 실제 GPU storage/kernel은 별도 요청과 검증 가능한 환경이 있을 때 재개한다.

**M6 완료 조건:** external library/backend가 J semantics를 정의하지 않고, verified Logical IR의 합법적인 realization route 중 하나로만 동작한다.

<a id="je0je6--jentity--contextual-higher-order-view-보조-트랙"></a>
#### JE0–JE6 — JEntity / contextual higher-order view 보조 트랙

**목표:** jsource의 공통 `A` 표현에서 **semantic RHS(`NOUN + FUNC`)를 하나의 carrier로 전달하는 원리**를 참고하되 C runtime allocation 구조를 복제하지 않는다. RustJ의 최소 `JEntity`는 parser/binding/assignment/semantic-operand 경계의 얇은 sum type으로 사용한다. `Value`와 `FunctionEntity`의 내부 모델을 하나로 합치거나 모든 IR node의 공통 base type으로 만들지 않는다. higher-order collection은 gerund 등 실제 operator semantics가 요구할 때 operator-specific view부터 도입한다.

**위상과 실행 순서:** 이 트랙은 M0–M6의 critical path에 새 milestone을 끼워 넣지 않는다.

- JE0 감사는 M2와 병행한다.
- **JE1의 최소 `JEntity` carrier는 M2 중에도 도입할 수 있다.** 단, `AssignedValue`/`SymbolValue`/`ParserNameBinding`/`FunctionOperand` 같은 중복 carrier를 한 seam씩 줄이는 경우에만 한다. full frontend가 끝날 때까지 중복 타입을 더 굳힌 뒤 대규모 migration하는 것을 피한다.
- JE2 parser/binding/assignment 수렴은 M2 작업 자체와 함께 진행할 수 있지만, jsource compatibility case가 먼저 있어야 한다.
- gerund/general collection abstraction(JE3+)과 noun storage를 건드리는 migration은 M2가 안정된 뒤 수행하며, physical representation과 결합되는 변경은 M3 경계가 안정된 뒤로 미룬다.
- 어느 JE 단계도 첫 M4 CPU vertical slice를 불필요하게 막지 않는다.

**jsource에서 채택하는 원리 / 채택하지 않는 구현:**

- 채택: semantic RHS에서 noun과 function entity가 하나의 상위 J-entity universe에 속하고 POS/type가 해석을 결정한다.
- 채택: `JEntity`의 직접 범위는 jsource `RHS = NOUN + FUNC`, `FUNC = VERB + ADV + CONJ`에 대응한다. NAME/ASGN/MARK/SYMB 같은 parser/runtime block class를 같은 semantic entity kind로 억지 통합하지 않는다.
- 채택: function의 parser/binding transport는 common entity handle을 사용할 수 있지만 noun snapshot과 function nameref late lookup의 시점 차이를 보존한다.
- 채택: gerund처럼 boxed noun이 shape를 제공하는 문맥에서는 그 noun의 shape를 보존한 entity-collection view를 만들 수 있다. shape는 container에 속한다.
- 채택: derived function은 operand/function identity를 보존하는 first-class entity다.
- 채택: parser/binding/assignment는 noun뿐 아니라 verb/adverb/conjunction 결과도 하나의 J entity로 전달할 수 있어야 한다.
- 비채택: jsource `AD/A` allocation header, refcount, virtual/in-place flags, allocator metadata를 semantic identity와 결합하는 방식.
- 비채택: verb를 noun의 physical atom buffer와 동일한 representation으로 강제하는 방식.
- 비채택: jsource 공통 header에 rank/shape field가 있다는 이유로 Verb/Adverb/Conjunction 자체에 noun-style semantic rank/shape를 부여하는 방식. current jsource도 function의 AN/AR field를 사용하지 않는다.
- 비채택: unresolved lexical NAME/NameRef를 Noun/Function과 같은 추가 POS/entity kind로 취급하는 방식.
- 비채택: 공통 entity abstraction이 Logical/Physical Array 경계를 우회하거나 BufferId/device/layout을 semantic layer로 끌어올리는 방식.

<a id="je0--현행-semantic-carrier와-jsource-대응-감사"></a>
##### JE0 — 현행 semantic carrier와 jsource 대응 감사

- [x] 상위 목표를 확정했다: `JEntity`는 noun/function을 묶는 semantic abstraction이고, common physical allocation abstraction이 아니다.
- [x] jsource의 공통 J-entity 원리와 RustJ의 `Value` / `FunctionEntity` / `FunctionOperand` 구조가 대응 가능함을 설계 수준에서 확인했다.
- [x] current jsource의 `RHS = NOUN + FUNC`, `FUNC = VERB + ADV + CONJ`를 다시 확인하고 RustJ `JEntity`의 직접 대응 범위를 semantic RHS로 한정했다.
- [x] current jsource가 function의 AN/AR를 사용하지 않는 것을 확인하여, function 자체에 noun-style shape/rank를 부여하지 않는 원칙을 고정했다.
- [x] `p.c` row 7이 noun/verb/adverb/conjunction RHS assignment를 허용하고, `sc.c::jtnamerefacv`가 noun value와 function nameref를 서로 다르게 처리하는 것을 확인해 공통 transport와 lookup timing을 분리했다.
- [x] `cf.c` bident/trident table에 Function뿐 아니라 즉시 Noun 결과가 존재함을 확인하여 parser construction의 일반 result type을 `JEntity`로 정정했다.
- [x] `cg.c::jtfxeachv/jtfxeach`가 source boxed noun의 rank/shape를 복사하지만, 결과는 jsource 주석상 **BOX라고 주장하는 내부 carrier에 function-typed A를 넣는 realization trick**임을 확인했다. 이것을 generic semantic `EntityArray`의 직접 선례로 사용하지 않는다.
- [x] current RustJ에 `FunctionOperand`, `ParserNameBinding`, `AssignedValue`, runtime `SymbolValue`, `ExprKind::{VerbValue,ModifierValue,Literal}` 등 Noun/Function carrier가 중복되어 있음을 확인했다. 최소 `JEntity`는 M2 이후의 장기 리팩터링보다 M2 seam 수렴에도 가치가 있다.
- [x] current `Verb { target, entity }` wrapper와 `FunctionEntity`의 책임이 완전히 수렴하지 않았음을 확인했다. `JEntity::Function` payload를 확정하기 전에 `VerbTarget`이 semantic identity인지 migration/execution adapter인지 감사한다.
- [x] lexical NAME과 function nameref를 구분했다. lexical/unresolved NAME은 JEntity가 아니지만, lookup 결과로 만들어진 executable nameref는 POS를 가진 Function entity이므로 `JEntity::Function` 안에서 `NameRef` head로 존재할 수 있다.
- [x] current jsource 기준 revision을 `0db94e768a845e2583c01d00538c3d16379677bb`(2026-10-03 master)로 고정해 이번 JE 감사의 비교 기준을 기록했다.
- [x] `Value`, `FunctionEntity`, `Verb`/`VerbTarget`, `FunctionOperand`, parser stack item, `ParserNameBinding`, `AssignedValue`, runtime `SymbolValue`, binding result, `NameRef`, `DefinitionCode`, gerund decode/view가 각각 어떤 semantic identity와 transport 책임을 보유하는지 inventory를 만든다.
- [x] `p.c` runtime parser와 `pv.c` tacit translator의 9-row 계열 코드를 구분해 근거를 기록한다. `pv.c::jtvis` 같은 translator action을 runtime observable semantics의 단독 oracle로 사용하지 않는다.
- [x] noun/verb/adverb/conjunction이 같은 parser/binding/assignment 경계를 통과하는 대표 jsource differential 사례를 정리한다.
- [x] 현재 `FunctionOperand::{Function,Noun}`와 다른 sum-type/enum 중 사실상 중복된 J-entity carrier를 식별한다.
- [x] current `Value`의 `CpuStorage` migration artifact가 `JEntity` API에 새 canonical dependency로 고착되지 않도록 금지 경계를 명시한다.

**JE0 완료 조건:** 모든 current semantic carrier와 lifetime/ownership/provenance 책임을 표로 설명할 수 있고, 새 타입을 만들기 전에 어떤 중복을 제거할지와 어떤 차이는 유지할지가 결정되어 있다.

<a id="je0-현행-carrier-감사와-첫-migration-seam-2026-10-04"></a>
##### JE0 현행 carrier 감사와 첫 migration seam (2026-10-04)

아래 표는 JE1 이전 감사 snapshot이다. `AssignedValue` 제거와 현재 API는 이어지는 JE1 구현 기록에서 관리한다.

| 현행 carrier | identity·ownership·lifetime | 유지할 metadata와 migration 결정 |
|---|---|---|
| `Value` / `Data` | noun의 J type·shape·atom order; boxed child는 `Arc<Value>`, sparse는 shared semantic array. owned dense clone은 payload copy이며 freeze 후 clone은 공유 | CPU payload는 현행 migration artifact다. 미래 `JEntity`는 `Value`를 transport하되 CpuStorage·host slice·BufferId·layout·device API를 새로 노출하지 않는다 |
| `FunctionEntity` | POS·head·ordered operands를 갖는 immutable `Arc` DAG; definition Code와 intrinsic noun snapshots를 소유 | Function payload는 `Arc<FunctionEntity>`로 충분하다. callable rank 계약과 noun shape를 혼동하지 않는다 |
| `Verb` / `VerbTarget` | `span + target + Arc<FunctionEntity>`; Primitive/Named target은 semantic head와 중복되고 Derived는 migration marker | production execution은 `resolve_function_entity`로 DAG를 조회한다. target 직접 검사는 현재 parser test host에서만 사용한다. 첫 seam에서 target을 새 semantic identity로 만들지 않고 wrapper의 occurrence span은 따로 보존한다 |
| `FunctionOperand` | Function child Arc 또는 freeze된 concrete noun + operand span; noun의 생성·대입 정보와 같은 것은 아니다 | JEntity와 payload union이 중복되지만 noun span이 추가되어 있다. 첫 migration 대상에서 제외하고 이후 zero-loss adapter로 연결한다 |
| parser `Item` / `ParseValue` | class·source/provenance·flags·occurrence·span override; noun은 `Expr + height`, function은 completed DAG/Verb wrapper | expression dependency, abstract/deferred noun 및 parser control state 때문에 concrete entity carrier와 통합할 수 없다. NAME/target/control을 JEntity POS로 만들지 않는다 |
| `ExprKind` / `Program` | literal/function result 외에도 Group·ReadName·Monad·Dyad 구조와 reduction/write provenance를 소유 | computation structure는 RHS carrier의 중복이 아니다. JEntity 도입으로 application graph를 제거하지 않는다 |
| `ParserNameBinding` | concrete noun snapshot, abstract noun class, function POS, known modifier + version이라는 lookup observation | 아직 값이 없는 abstract noun/POS와 실제 RHS를 구분하므로 enum을 유지한다. 모든 lookup을 entity snapshot으로 바꾸지 않는다 |
| `AssignedValue` | row 7의 concrete Noun/Verb/Modifier RHS를 host에 넘기고 대입 결과를 돌려받는 transport | **JE1 첫 seam**. Noun/Function 두 variant로 바꾸되 function POS는 DAG에서 얻고 occurrence/height/assignment source는 parser에 남긴다 |
| runtime `SymbolValue` / `Binding` | freeze된 noun 또는 shared function wrapper; 별도의 Engine-local NameVersion. replacement는 old noun을 pool에 retire | AssignedValue와 semantic RHS payload가 중복된다. 첫 JE1은 host boundary adapter만 교체하며 전체 symbol table과 pool migration은 JE2 이후다 |
| binding result / `BoundProgram` | 분석용 reads·versions·dynamic function references·pending write; 실행 가능한 cached plan이 아님 | entity identity와 binding/version proof를 합치지 않는다. read-only prepare는 commit하지 않는다 |
| lexical NAME / `FunctionHead::NameRef` | lexical queue name은 unresolved spelling/flags; lookup 후 function NameRef는 expected POS를 가진 executable function identity | noun은 lookup snapshot, function은 필요한 경우 적용 때 재조회한다. 같은 공통 carrier가 이 timing 차이를 없애면 안 된다 |
| `DefinitionSource` / `DefinitionCode` | shared original source·primitive context·span, immutable body/valence/control metadata; invocation locals 없음 | Function head가 Arc Code를 소유한다. noun DD는 Value이며 Code가 아니다. alias assignment는 본문 호출이 아니다 |
| gerund noun / `decoded_gerund` | source는 boxed noun; decode는 ordered `Vec<Arc<FunctionEntity>>` 실행 auxiliary이며 source semantic children이 아니다 | order와 noun snapshots는 보존하지만 Vec 자체에는 source shape·lookup observation이 없다. parent noun/span과 capture observations를 함께 봐야 한다. shaped view의 operator별 충분성은 JE3에 남기며 EntityArray를 만들지 않는다 |

**중복 판정:** `AssignedValue`와 `SymbolValue`가 첫 concrete RHS seam이고, `FunctionOperand`는 payload가 겹치지만 provenance 계약이 다르다. `ExprKind`, `ParserNameBinding`, stack/control, binding observations는 역할이 달라 유지한다. JE0 자체는 JEntity API를 구현한 단계가 아니다. JE1의 완료 상태와 JE2–JE6의 남은 경계는 아래 항목에서 관리한다.

**근거 구분:** 새 감사 revision `0db94e768a845e2583c01d00538c3d16379677bb`의 `p.c` L87–96은 주석상 tacit translator용 `cases[]`다. runtime `p.c`의 ptcol dispatch와 L1006–1043의 row 7은 이미 stacked된 CAVN RHS를 대입하고 그 값을 stack에 남긴다. `pv.c::jtvis` L158은 translator action이므로 runtime 대입 oracle로 사용하지 않는다. `sc.c::jtnamerefacv` L364–397은 noun value와 expected-POS function nameref를 구분한다. `cf.c` L292–308의 `{0,NOUN}`은 construction result가 항상 Function이라는 가정을 반박한다. `cg.c` L101–121의 BOX carrier는 source rank/shape를 가진 내부 realization이며 semantic EntityArray가 아니다. 새/기존 source의 선언형 row predicate와 constructor disposition은 native Windows Python으로 동일함을 확인하고 5개 파일 hash를 `reports/entity-carrier-source-audit.json`에 기록했다. 이것은 runtime ptcol trace 또는 새 revision DLL 검증이 아니다.

**이 감사에서 수정한 compatibility bug:** row 7이 모든 modifier를 `resolve_modifier`로 적용용 해석한 뒤 대입하여 `copy=:explicit_adverb` / `copy=:explicit_conjunction`을 거부했다. 이제 stacked RHS를 그대로 대입한다. nameless modifier는 기존 lookup 때 by-value로 stack되고, nonnameless modifier는 POS-bearing NameRef를 유지한다. C `5!:1`에서도 explicit/derived modifier alias의 head가 원본 이름임을 확인했다. static prepare는 application semantics가 미지원인 function도 POS-known NameRef로 대입 구조를 보존한다. 본문·지역 invocation frame을 실행하지 않으며 alias 재대입과 실제 modifier application을 분리한다. explicit body 호출의 기존 미완료 상태는 그대로 추적한다.

**JE0 gate:** Rust regression 5개로 네 RHS 품사의 grouped assignment result/POS·commit identity, noun snapshot/function late lookup와 POS mismatch, 48-level Hook DAG의 Arc 공유 및 host 종료 후 lifetime, 65,536-atom noun snapshot의 payload 공유와 rebinding lifetime, explicit modifier alias의 static non-commit·runtime NameRef/POS/span·본문 미실행·재조회 및 POS 변경 오류를 검증했다. C corpus/stage에 **71건**을 추가했다: noun assignment 13, noun value 19, function class/atomic representation 33, J error 6. valid explicit adverb/conjunction alias 사례는 미지원 예외로 분류하지 않고 모두 C와 일치한다.

Windows default/portable 각각 **363 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture **4,810 cases / 4,806 passed / 기존 runtime 경계 4 / failed 0**; stage **9,990 checks**, words **6,618 cases**. capture graph 경계는 106으로 따로 기록하며 static 경계 2와 구분한다. 새 source 감사 pin `0db94e7...`, conformance source pin `13994ff...`, 실제 DLL release `ded7793...`는 서로 구분한다. 새 revision DLL·full upstream suite·private runtime trace·explicit body invocation acceptance를 검증했다고 주장하지 않는다.

**JE0의 handoff:** `AssignedValue` ↔ runtime assignment 한 seam을 JE1으로 넘겼다. semantic nested DD 등 기존 M2 미완료 항목은 유지한다. JE0/JE1을 full frontend rewrite나 첫 M4 CPU slice의 선행조건으로 확장하지 않는다.

Sources: [runtime p.c](https://github.com/jsoftware/jsource/blob/0db94e768a845e2583c01d00538c3d16379677bb/jsrc/p.c#L1006), [translator pv.c](https://github.com/jsoftware/jsource/blob/0db94e768a845e2583c01d00538c3d16379677bb/jsrc/pv.c#L158), [nameref sc.c](https://github.com/jsoftware/jsource/blob/0db94e768a845e2583c01d00538c3d16379677bb/jsrc/sc.c#L364), [constructor cf.c](https://github.com/jsoftware/jsource/blob/0db94e768a845e2583c01d00538c3d16379677bb/jsrc/cf.c#L292), [gerund cg.c](https://github.com/jsoftware/jsource/blob/0db94e768a845e2583c01d00538c3d16379677bb/jsrc/cg.c#L101).

<a id="je1--최소-공통-jentity-identity-도입"></a>
##### JE1 — 최소 공통 `JEntity` identity 도입

- [x] `JEntity`/`JEntityRef`의 최소 API를 **boundary carrier**로 설계한다. 직접 semantic variants는 `Noun`과 `Function`으로 두고 Function이 `Verb|Adverb|Conjunction` POS를 소유하는 구조를 우선 검토한다.
- [x] `JEntity`를 `Value`/`FunctionEntity` 내부 representation을 통합하는 base class로 사용하지 않는다. 최초 적용 seam은 assignment/binding/parser result 또는 `FunctionOperand` 중 differential test가 가장 잘 갖춰진 한 곳으로 제한한다.
- [x] `JEntity::Function`이 `Arc<FunctionEntity>`만으로 충분한지, 현재 `Verb`/`VerbTarget` wrapper에서 semantic하게 남겨야 할 것이 있는지 먼저 결정한다.
- [x] lexical NAME, unresolved reference, binding/version/provenance를 `JEntity` variant와 분리한 reference/control API로 설계한다.
- [x] `Verb`/`Adverb`/`Conjunction`을 별도 payload 복제로 만들지 않고 shared `FunctionEntity` + `FunctionPartOfSpeech` identity를 재사용한다.
- [x] noun payload는 logical J noun identity를 가리키며 physical buffer/layout/device를 소유하지 않게 한다.
- [x] source span/provenance와 binding/version은 entity payload 자체와 필요한 observation/binding metadata를 구분한다.
- [x] large derived function/train이 `JEntity` conversion에서 deep-copy되지 않는 sharing test를 추가한다.
- [x] noun/verb/adverb/conjunction round-trip 및 POS mismatch/error semantics regression을 추가한다.

**JE1 완료 조건:** parser/binding API가 noun과 function을 공통 entity handle로 전달할 수 있으면서 기존 `FunctionEntity` DAG와 J noun semantic identity를 훼손하지 않는다.

<a id="je1-구현--대입-경계의-최소-jentity-2026-10-04"></a>
##### JE1 구현 — 대입 경계의 최소 JEntity (2026-10-04)

아래는 JE1 도입 시점의 snapshot이다. 임시 SymbolValue/Verb adapter의 제거와 현재 namespace carrier는 이어지는 JE2 구현 기록에서 관리한다.

- `semantic::JEntity::{Noun(Value), Function(Arc<FunctionEntity>)}`와 borrowed `JEntityRef::{Noun(&Value), Function(&FunctionEntity)}`를 도입했다. owning carrier를 이동하면 기존 payload를 그대로 넘기며 `as_ref()`는 copy·allocation·refcount update 없이 inspect한다. owning `JEntity`에는 자동 Clone을 제공하지 않는다: 아직 Owned Value의 clone은 전체 noun payload를 복사할 수 있기 때문이다. borrowed view의 Copy/Clone은 참조만 복사한다.
- `RuntimeParserHost::assign(name, JEntity) -> Result<JEntity>`가 row 7의 Noun/Verb/Adverb/Conjunction을 같은 boundary로 전달한다. 중복 `AssignedValue` enum을 제거했다. 함수의 실제 POS는 FunctionEntity가 소유하며 separate Verb/Modifier payload를 entity에 추가하지 않았다.
- parser는 verb의 occurrence span/compatibility target, noun의 Expr height, source/provenance·occurrence·assignment flags를 carrier 바깥에 보존한다. capture의 Commit source/class/function identity와 binding version도 기존 경로에 남는다. noun assignment는 기존 host에서 freeze하고 반환·symbol payload를 공유하며 replacement/pool retirement 규칙을 유지한다.
- runtime `SymbolValue`는 첫 migration의 compatibility adapter로 남는다. `Verb::from_entity`는 Verb POS를 확인하고 shared entity head에서 기존 Primitive/Named/Derived target과 intrinsic span을 복원한다. 이 adapter는 function DAG·DefinitionCode를 복사하거나 NameRef를 현재 함수 값으로 fix하지 않는다. parser reinsertion은 원래 occurrence wrapper의 span/target을 그대로 재사용한다.
- lexical NAME, abstract noun/POS observations, `ParserNameBinding`, `ExprKind`, `FunctionOperand`, symbol table 및 gerund auxiliary는 이번 seam에 포함하지 않았다. JEntity에 BufferId·stride·layout·target·device·schedule API를 추가하지 않는다. `Value`가 현재 CPU payload를 포함하는 migration artifact는 여전히 남으며 이를 logical/physical 분리 완료로 간주하지 않는다. Function에 noun shape/rank를 추가하지 않았다.

검증은 JE0의 네 품사 grouped/chain assignment·noun snapshot·function late lookup·explicit modifier alias·POS 오류·effect/provenance 비교 71건을 같은 production boundary에서 재사용한다. 기존 48-level Hook DAG test에 JEntity move/borrow round-trip과 refcount 불변을 추가했고, 새 tests는 65,536-atom Owned noun move/borrow 시 payload pointer 유지, explicit Verb/Adverb/Conjunction의 POS·shared DefinitionCode 유지, runtime Verb adapter의 identity/span·modifier POS 거부를 검증한다.

**JE1 gate:** Windows default/portable 각각 **366 passed / 17 ignored**, fmt/clippy/build 통과. Python **27 passed**. j64/AVX2 각각 direct·semantic-reference·parser-capture **4,810 cases / 4,806 passed / 기존 runtime 경계 4 / failed 0**, stage **9,990 checks**, words **6,618 cases**. 새 언어 form을 추가한 변경이 아니므로 JE0의 71건과 전체 기존 corpus를 그대로 재검증했으며 report 10개의 binary/source hash를 확인했다. capture graph 경계 106과 static 경계 2는 별도이고, full upstream suite·definition invocation acceptance·새 audit revision DLL·private C trace 동등성은 미검증이다. conformance source pin은 `13994ffa1ed5f06f79fad6e9822a7ed2d29b1528`, 실제 reference DLL release는 `ded7793fe5795d79eda8e7138dce94aa056edf78`; JE0 source 감사 `0db94e768a845e2583c01d00538c3d16379677bb`와 구분한다.

JE1의 최소 API/첫 boundary는 완료이며 runtime `SymbolValue` seam을 JE2로 넘겼다. stack의 deferred noun/application structure와 lookup observations를 concrete JEntity로 강제하지 않는다. explicit body invocation·scope, semantic nested DD, JE3+ higher-order views, broader storage migration, full J conformance는 계속 별도 미완료다. optimization·CUDA·GitHub CI는 보류한다.

<a id="je2--parserbindingassignment-경계-수렴"></a>
##### JE2 — parser/binding/assignment 경계 수렴

- [x] `FunctionOperand::as_entity_ref()`로 공통 borrowed JEntityRef를 제공하고 `span()`으로 provenance를 보존한다. noun의 별도 span과 함수 Arc 소유권을 유지하기 위해 owning enum은 유지한다.
- [ ] parser stack/value model이 noun/function에 대해 공통 entity transport를 사용하되 jsource 9-row class/POS 규칙은 그대로 유지하게 한다. runtime rows 0–2와 row 7의 completed-result transport는 아래 단계에서 완료했고 전체 stack variant 수렴은 별도다.
- [x] `CompletedParseResult`로 실행 완료 noun과 row 7의 네 RHS class를 JEntity 경계에 연결한다. deferred Expr·NAME·control은 concrete entity로 강제하지 않는다.
- [x] modifier train의 Noun/Verb/Adverb/Conjunction operand를 같은 completed-result 경계에서 이동하고 noun source span·freeze 정책·함수 DAG identity를 보존한다.
- [x] rank/@: conjunction operand를 completed-result 경계로 연결하고 right-before-left 검사·gerund quiet fallback·원본 Expr span을 유지한다.
- [x] noun-left fork의 constant operand와 지원 explicit/direct definition의 mode/body·생성 결과를 completed-result 경계로 연결한다. definition invocation은 별도 미완료다.
- [x] 생성/대입 경계 이행 후 남은 adapter 책임을 재감사하고 capture의 중복 함수 identity 조회를 borrowed helper로 통합한다.
- [x] assignment가 `JEntity`를 namespace에 write하고 같은 assigned `JEntity`를 expression result로 반환하는 contract를 공통화한다. `Binding.value`와 runtime host boundary를 JEntity로 연결하고 SymbolValue를 제거했다.
- [ ] name lookup이 binding에서 `JEntity`를 얻은 뒤 expected POS 검사를 수행하고 late-binding/version semantics를 유지하게 한다. top-level runtime lookup과 verb/modifier POS 검사는 완료했으며 전체 local/locale/definition scope는 미완료다.
- [x] jsource `jtnamerefacv`의 의미적 차이를 회귀로 고정한다: noun name은 lookup 시점 value/snapshot을 전달할 수 있지만 function name은 실행 시 재조회되는 nameref가 필요할 수 있다. JEntity binding에서도 기존 71건과 새 noun/function replacement 11건으로 timing·POS·binding semantics를 유지한다.
- [ ] explicit/direct definition constructor와 invocation 결과가 같은 entity boundary를 사용하게 한다.
- [ ] parser/runtime/static path가 서로 다른 entity wrapper를 만들지 않는지 differential/golden으로 확인한다.

**JE2 완료 조건:** noun과 function의 parser/binding/assignment transport가 하나의 semantic abstraction으로 수렴하고, POS·lookup timing·effect ordering은 jsource-compatible하게 유지된다.

