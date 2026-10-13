[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)


<a id="read-advanced"></a>
<a id="심화-기술-계약--primitive-rank-하드웨어-계획-flowstorage"></a>
## 심화 기술 계약 — primitive, Rank, 하드웨어 계획, Flow–Storage

아래는 기존 §4.5~§4.24의 상세 계약을 모아둔 참조 자료다. **핵심 단계 흐름을 이해하는 데 필수 선행 독서는 아니다.** 기존 항목 번호와 앵커는 세부 체크리스트의 교차 참조를 보호하기 위해 보존한다.

<a id="45-primitive-contract"></a>
### 4.5 Primitive contract

semantic analysis에서 모든 primitive는 공통 `PrimitiveContract` interface를 통해 해석한다. built-in J primitive와 name 기반 extension primitive의 **등록 경로는 달라도 분석 interface는 같다.**

이 절의 contract는 “분석기가 반드시 물어볼 수 있어야 하는 질문”을 정의한다. 구체적인 저장 구조는 4.10의 **PrimitiveSpec semantic record + capability interfaces + lowering registry** 분리를 따른다.

semantic 쪽에서 최소한 다음을 표현하거나 명시적으로 `Unknown`으로 둘 수 있어야 한다.

```text
identity / part of speech / supported valences
innate ranks (monad / dyad-left / dyad-right)
shape rule
type / promotion rule (shape/empty/fill context 의존 가능)
axis-role / access-pattern rule
error contract
  domain
  rank / shape
  overflow / promotion
  observable error ordering
effect contract
alias / mutation legality
safe rewrite / reassociation constraints
semantic reference definition (optional)
```

fusion cost, accumulator realization, register/shared-memory 양, concrete layout, tile 크기, device-specific intrinsic은 semantic identity 자체가 아니다. target-independent semantic capability가 제공한 facts와 downstream TargetProfile/schedule을 바탕으로 native planner 또는 external compiler가 결정한다.

따라서 `PrimitiveContract`는 analyzer가 보는 공통 interface이고, `PrimitiveSpec`은 그 contract를 실제로 제공하는 versioned registry record라는 관계로 사용한다.

<a id="extension-primitive-inventory"></a>

<a id="46-확장-primitive-목록--기능-필요성-채택-상태"></a>
### 4.6 확장 primitive 목록 — 기능, 필요성, 채택 상태

**단일 Extension 통합 목록(2026-10-10):** 아래 표 하나에 JAXA/Japchae의 역사적 prototype·연구 항목과 NN-01–NN-34 도입 체크리스트를 합치고 품사·계약·고정 출처·선택적 분해 경로를 보존했다. 체크박스는 **NN 연산군의 유일한 진행 상태**이다. 행의 모든 적용 대상에 NN-G0–G7 검증 증거가 있거나 명시적 보류·제외 결정이 기록될 때만 완료한다. EXT-R*/EXT-H*는 별도 소관의 연구·이력 항목으로 NN 체크박스가 없다. §A2는 공통 Registry/IR 기반 구현 상태를 소유한다. 과거 prototype은 실행 지원 증거가 아니다.

| 저장소 | 확인한 최신 HEAD | 근거의 역할 |
|---|---|---|
| `yunskim/JAXA` | [`12bc0659`](https://github.com/yunskim/JAXA/tree/12bc0659a1e008597bf07449c70029bc61eff93b) · 2026-03-24 | 초기 문제의식; 후속 결정의 우선 근거가 아님 |
| `yunskim/JAXA-complier` | [`ceba0589`](https://github.com/yunskim/JAXA-complier/tree/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368) · 2026-04-26 | 품사를 갖춘 실제 prototype inventory |
| `yunskim/japchae` | [`510c31b5`](https://github.com/yunskim/japchae/tree/510c31b5fe3d4bf6ca0c6fa9aeae8556ca134607) · 2026-06-19 | pooling/dropout 및 상태 설계 이력 |
| `yunskim/jaxa-analyzer` | [`7275d5ba`](https://github.com/yunskim/jaxa-analyzer/tree/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33) · 2026-09-30 | 후기 cast·effect·AD·Flow–Storage 검토; 현행 설계는 RustJ로 이관 |

**출처 키**: [P — prototype registry](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py); [T — historical training modes](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/README.md); [A — extension architecture §§1.10, 3.4–3.5, 4.3, 7.4–7.7](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md); [F — later Flow–Storage scope §8](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/flow_storage_scope_and_compiler_positioning.md); [W — Flow–Storage technical review](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/flow_storage_model_review_and_open_questions.md); [D — pooling/dropout decisions](https://github.com/yunskim/japchae/blob/510c31b5fe3d4bf6ca0c6fa9aeae8556ca134607/documents/decisions.md).

아래 “필요한 이유와 최소 계약”은 원본 기능 설명을 현행 RustJ 설계에 적용한 요구사항이다. 원본 prototype가 이 계약을 모두 구현했다는 뜻은 아니다. [영어 mirror의 같은 목록](../../../PROJECT.md#extension-primitive-inventory).

**확장이 필요한 이유:** 모든 계산이 새 primitive 없이는 표현 불가능하다는 뜻이 아니다. 표준 J로 표현 가능한 계산은 reference definition을 먼저 제공한다. 확장 이름은 parameter schema, shape/numeric/effect 계약, 고수준 graph identity 및 검증된 library/native/external lowering의 연결점을 제공한다. 지원 경로가 없으면 등록 완료로 표시하지 않는다.

모든 source 이름은 ordinary J binding이며 예약 keyword가 아니다. **Adverb → parameter/verb operand를 받아 derived verb 생성**, **Verb → 배열 계산**, **Conjunction → 두 operand로 derived entity 생성**, **등록 API → 선언/관계 설정**을 구분한다. builder의 품사와 생성된 계산 verb의 valence/rank를 혼동하지 않는다.

<a id="nn-extension-checklist"></a>

| 우선 | ID·상태 | 이름/연산군 — 단일 통합 목록 | 표면 품사 | Graph 의미 및 최소 계약 | 선택적 Logical 분해 / 실행 | 고정 출처 |
|---|---|---|---|---|---|---|
| P0 | [ ] NN-01 | `relu`, `sigmoid`, `tanh` | Verb | 원소별 활성화; 편의용 이름/수치 규약(표준 J 식으로 충분하면 별도 원자 op는 선택) 기존 계약: 가장 작은 map/fusion 검증 대상이다. numeric domain·NaN·signed zero·미분 경계 정책을 reference와 비교한다. | Map/elementwise; 기존 J primitive로도 참조 | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py) |
| P0 | [ ] NN-02 | `linear` builder (`torch.nn.functional.linear`) | Adverb builder | 명시적 W/bias를 받는 affine/dense layer; batch·axis 및 parameter schema 기존 계약: 행렬 contraction과 bias 결합을 분석하고 다른 layer의 weight를 구분한다. 단순 matmul과 parameter/resource binding을 분리한다. | MatMul + Bias → GEMM/fused route | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| P0 | [ ] NN-03 | `flatten` (`torch.flatten`) | Verb | J atom order, 지정 축·frame을 보존하는 feature flatten 기존 계약: 공간 layer와 linear 입력을 연결하고 logical reindex를 보존한다. J atom 순서·축 범위가 필요하며 항상 zero-copy라는 뜻은 아니다. | StaticReindex/Reshape; zero-copy는 별도 판정 | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py) |
| P0 | [ ] NN-04 | `conv2d` (historical `conv` name not exposed) | Adverb builder | 2D convolution; kernel/stride/padding/dilation/groups/축과 출력 shape 기존 계약: 국소 window·채널 contraction을 고수준 graph에 남겨 reference/library/kernel 경로를 비교한다. kernel·stride·padding·dilation·bias 및 explicit weight resource 계약이 필요하다. | Window/Gather + Contract/Reduce 또는 외부 Conv 커널 | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| P0 | [ ] NN-05 | `avg_pool2d`, `max_pool2d` builders | Adverb builders | 윈도우 pooling; padding 분모·NaN·동률·빈 window 기존 계약: window 접근·겹침·reduction을 보존한다. padding 포함 분모와 빈 window 계약이 필요하며 producer/consumer fusion은 후속 판단이다. · window reduction과 backward index 요구를 분석한다. NaN·동률·padding·빈 window와 backward 선택 규칙을 명세한다. | WindowReduce / library | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [D](https://github.com/yunskim/japchae/blob/510c31b5fe3d4bf6ca0c6fa9aeae8556ca134607/documents/decisions.md) |
| P0 | [ ] NN-06 | `softmax`, `log_softmax` builders | Verb (축 계약 미정) | 축별 수치안정 정규화; empty/Inf/NaN·축 규약 기존 계약: exp/map와 axis reduction을 한 구조로 분석한다. 안정화 방식·축·빈 입력·무한대/NaN을 명세하고 고정 fusion 경계로 취급하지 않는다. | MaxReduce + Exp + SumReduce + Normalize / fused | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py) |
| P0 | [ ] NN-07 | `cross_entropy` builder | Verb (dyad) | label 형식, log 안정화, reduction 및 손실 반환 shape 기존 계약: loss 축 reduction과 입력별 미분 계약을 드러낸다. logits/probability 여부·label 형식·평균/합 reduction·log 안정성을 먼저 확정한다. | Gather/LogSumExp/Reduce 또는 전용 loss | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| P1 | [ ] NN-08 | `conv1d`, `conv3d`; depthwise/grouped via `groups` parameter | Adverb builders | 공간 차원·groups·channels/innate cell rank 계약; conv2d 단순 이름 바꾸기 금지 기존 계약: 일반 convolution과 다른 채널 연결·재사용 구조를 분석한다. multiplier와 출력 shape를 명세해야 하며 prototype의 identity shape rule만으로 등록할 수 없다. | Window + Contraction / target Conv | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py) |
| P1 | [ ] NN-09 | `layer_norm`, `rms_norm`, `batch_norm` builders | Adverb builders | feature/batch 통계축, epsilon, affine, training/inference running-state 구분 기존 계약: 배치 축 reduction, 학습/추론 차이, running-stat read/write를 드러낸다. epsilon·통계 축·갱신 규칙이 필요하며 고정 barrier는 semantic 계약이 아니다. · 배치 통계와 sample-local reduction을 구분하고 affine parameter·epsilon·축을 검증한다. sample 독립이 reduction 내부 동기화 부재를 보장하지 않는다. | Reduce + Normalize + Scale/Shift; effect 있는 BN은 별도 | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| P1 | [ ] NN-10 | `gelu`, `silu` | Verb | 활성화 수치 정의와 근사 버전 구분 기존 계약: 정확식과 근사식을 구분한 numeric 계약으로 backend 간 오차를 검증한다. 단순한 이름만으로 approximation을 선택하지 않는다. | Elementwise; fused activation | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py) |
| P1 | [ ] NN-11 | `embedding` builder | Verb (후보) | 정수 인덱스 조회·범위 오류, padding index, 선택적 weight 공유 기존 계약: parameter gradient만 있고 index 미분은 없는 사례로 AD 분해를 검증한다. 입력별 미분 가능성과 scatter/accumulate 계약이 필요하다. | Gather; 역방향은 indexed scatter-add | [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| P1 | [ ] NN-12 | `scaled_dot_product_attention` builder | Verb (다중 인자 결합 규약 미정) | Q/K/V, scale, causal/padding mask, layout·head 축, 출력 dtype 기존 계약: contraction–softmax–contraction 구조를 유지해 tiled/library 경로를 비교한다. mask·scale·축·dtype 계약이 필요하며 FlashAttention 지원을 뜻하지 않는다. | MatMul + Mask + Softmax + MatMul / fused attention; FlashAttention과 동의어 아님 | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py) |
| P1 | [ ] NN-13 | `rotary_embedding` (provisional; no matching public functional API) | Verb (후보) | 위치 인덱스, 회전 축, 주파수/precision 수치 규약 | Slice/Pairwise Map/Rotate; 단순 positional add는 기본 J 조합 | RustJ 신규 후보 |
| P1 | [ ] NN-14 | `matmul` / existing-J contraction recognition | 기존 J 별칭 / contraction 인식 | `+/ .*` 같은 기존 J 표현의 인식·최적화 기준; 새 필수 문법/extension 아님 기존 계약: `+/ .*` 등 표준 J reference로 표현 가능한 계산의 contract/lowering alias 후보이며 필수 새 syntax가 아니다. | Dot/Reduce/Rank → GEMM/batched GEMM | [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| P1 | [ ] NN-24 | `logsumexp` (`torch.logsumexp`) | Verb (축/keepdims 계약 미정) | softmax·loss의 안정적 reduction 공통부; reduction 축·empty reduction(-∞ 가능 여부)·Inf/NaN·dtype/누산 정밀도와 반환 shape를 명시. 일반 `^.`와 `+/`의 무조건 재결합을 허용하지 않음. | MaxReduce + Exp + SumReduce + Log; 검증된 numeric rule에 한해 fused reduction | [ONNX ReduceLogSumExp](https://onnx.ai/onnx/operators/onnx__ReduceLogSumExp.html) |
| P1 | [ ] NN-25 | `topk` (`torch.topk`; no assumed `argpartition` API) | Verb (k·axis 인자 계약 미정) | MoE routing·토큰 sampling·검색의 값과 index 동시 결과, stable tie-break, NaN/empty/k 범위 및 출력 조립을 명세. 기존 J grade/sort 구현과 의미 비교; 새 언어 primitive 강제 안 함. | Grade/Sort + Select 또는 증명된 selection kernel | [ONNX TopK](https://onnx.ai/onnx/operators/onnx__TopK.html) |
| P1 | [ ] NN-26 | `group_norm`, `instance_norm` builders | Adverb builders (후보) | channel grouping·group 수 divisibility·batch/sample 축·epsilon·affine·statistics precision을 구분; 기존 `layernorm`/`batchnorm`의 파라미터 차이만으로 동일 계약이라 가정하지 않음. | Reshape/Group + Reduce + Normalize + Affine; library route | [ONNX GroupNormalization](https://onnx.ai/onnx/operators/onnx__GroupNormalization.html) |
| P1 | [ ] NN-27 | `segment_reduce`, `embedding_bag` builders | Verbs/builders (후보) | variable-length offsets/lengths·빈 segment·sum/mean/max·padding index·반복 index·gradient accumulation 의미. 기본 `SegmentView`와 `Reduce` 재사용, 신규 공통 IR 계층 금지. | SegmentView + Reduce / Gather + SegmentView + Reduce | [PyTorch segment_reduce](https://docs.pytorch.org/docs/stable/generated/torch.segment_reduce.html), [PyTorch native operator interface](https://github.com/pytorch/pytorch/blob/main/aten/src/ATen/native/README.md) |
| P1 | [ ] NN-28 | `scatter_reduce`, `index_add` builders | Verbs (후보; 기존 execution basis 이용) | sparse gradient·MoE reassembly의 중복 index 충돌, 순서·결정성, overwrite vs add/max/min, bounds 및 J observable error를 명세. `Scatter`/`ScatterCombine` 실행 Basis는 **이미 존재**하므로 신규 IR primitive 요구가 아님. | 기존 Scatter/ScatterCombine + Reduce; 원자/분할 병렬화는 downstream 합법성 검증 | [ONNX ScatterND](https://onnx.ai/onnx/operators/onnx__ScatterND.html), [ONNX ScatterElements](https://onnx.ai/onnx/operators/onnx__ScatterElements.html) |
| P2 | [ ] NN-15 | `dropout` builder | Adverb builder | 명시적 RNG seed/state/version, 학습/추론, mask 저장·재사용 기존 계약: 학습/추론 차이와 RNG 의존성을 명시해 잘못된 CSE·재계산을 막는다. seed/state·scaling·mask 재사용 계약이 필요하다. | RNG + Mask + Scale; 효과/재현성 보존 | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [D](https://github.com/yunskim/japchae/blob/510c31b5fe3d4bf6ca0c6fa9aeae8556ca134607/documents/decisions.md) |
| P2 | [ ] NN-16 | `vjp` (`torch.func.vjp` reference; source/AD API undecided) | 입력별 VJP verbs / 과거 adverb factory | 입력별 독립 backward 계산, residual 필요성, gradient shape 기존 계약: 학습 계산도 일급 graph로 표현한다. 후기 설계는 입력별 data/parameter VJP를 독립 entity로 등록하므로 하나의 backward spelling에 출력·state를 숨기지 않는다. | 구성 가능한 것은 조합 미분, 불가능한 것은 등록된 VJP 커널 | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| P2 | [ ] NN-17 | `SGD`, `Adam`, `AdamW` (`torch.optim` builders; unexposed) | Adverb builders | step, weight, gradient, moment, decay의 explicit read/write/version 기존 계약: weight·gradient·moment·step의 read/write 및 version 의존을 분석한다. mutable state는 explicit StateResource이며 in-place/fusion/실행 시점은 별도 결정이다. | Elementwise updates + Reduce(필요 시); optimizer state는 `StateResource` | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| P2 | [ ] NN-18 | `register_autograd` (`torch.library` inspiration; metadata only) | 등록 API, 계산 연산 아님 | 계산 op가 아닌 정적 등록 API; J inverse/obverse(`:. `) 재정의 금지 기존 계약: shape/type·선형화·residual·공유 resource 계약을 검증하고 미래 AD의 rule을 제공한다. inverse/obverse나 parallel annotation이 아니며 API spelling과 schema는 후속 확정한다. | 추후 AD 변환에서 참조; 현재 자동미분 완료 주장 금지 | [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| P2 | [ ] NN-19 | `to` (`torch.Tensor.to` reference; candidate, unexposed) | Adverb / Verb / 미결 | mixed precision·residual 보존·side output/effect의 설계 대상; `cp`/`emit` surface 채택 미결 기존 계약: conversion을 producer 구조와 함께 분석해 mixed-precision 후보를 보존한다. rounding·overflow·NaN 계약이 필요하며 adverb 품사만으로 fusion/in-place를 강제하지 않는다. · 독립 conversion graph node를 표현한다. device 전송 기능이 아니며 별도 커널 강제와도 다르다. 지원 dtype family와 numeric rule은 추가 확정이 필요하다. · gradient·loss·통계의 side output과 write/accumulate 효과를 보존한다. destination·충돌·누적 순서를 명세한다. 후기 문서는 독립 surface op와 Write/Accumulate 중 표현 선택을 미결로 둔다. · backward residual의 lifetime와 저장/재계산 trade-off를 분석한다. 강제 보존·저장 후보·실제 materialization을 구분하며 후기 문서는 surface 채택을 미결로 둔다. | Type conversion, explicit Value/State effect; physical 배치 아님 | [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md), [F](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/flow_storage_scope_and_compiler_positioning.md), [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py) |
| P2 | [ ] NN-29 | `interpolate`, `grid_sample` builders | Adverb/Verb builders (후보) | 영상·feature resampling; nearest/linear/cubic, coordinate transform, align_corners, padding, out-of-bounds 및 빈 spatial extent를 구분. 정적 reshape와 다른 **값 보간** 연산. | Gather + WeightedMap 또는 Window/Interpolation kernel; graph 의미는 유지 | [ONNX GridSample](https://onnx.ai/onnx/operators/onnx__GridSample.html) |
| P2 | [ ] NN-30 | `rand`, `randn`, `bernoulli`, `multinomial` (explicit RNG contract pending) | Verbs/explicit-key builders (후보) | 초기화·샘플링·dropout의 공통 RNG 의미; explicit key/state, shape/dtype, seed 재현성, device 간 sequence 보장 범위, 분포·수치 결과를 명시. J 기존 RNG 어휘·상태를 임의 변경하지 않으며 hidden RNG 효과를 감추지 않음. | 명시적 RNG 생성 연산(key 또는 StateResource) → 분포별 Map/변환; RNG 생성 자체는 기존 순수 Basis만으로 분해 불가 | [JAX explicit PRNG key model](https://docs.jax.dev/en/latest/jax.random.html), [StableHLO RNG spec](https://openxla.org/stablehlo/spec) |
| P3 | [ ] NN-20 | `conv_transpose2d`, `adaptive_avg_pool2d`, `adaptive_max_pool2d` | Adverb/verbs (후보) | 업샘플·동적 출력 크기·경계/윈도우 규약 | Scatter/WindowReduce 또는 전용 kernel | RustJ 신규 후보 |
| P3 | [ ] NN-21 | `quantize_per_tensor`, `dequantize` candidates | Verbs (후보) | scale/zero-point, saturation/rounding, accumulate dtype 계약 후에만 | Q/DQ + Contract 또는 정수 GEMM | RustJ 신규 후보 |
| P3 | [ ] NN-22 | KV-cache/paged attention (no stable PyTorch public op; `kv_cache_update` unexposed) | 효과 있는 연산 (후보) | 추론 상태·버전·alias/순서·증분 인덱싱; effect-free attention과 분리 | Gather/Scatter + Attention, backend-specific fused route | RustJ 신규 후보 |
| P3 | [ ] NN-23 | `grouped_mm` (`torch.nn.functional`; MoE routing still composed) | Verbs/builders (후보) | routing index·capacity/drop/pad·재조립·gradient contract 기존 계약: 논의 중 역할/예시 이름이다. `@.`는 표준 J primitive이며 확장 목록에 넣지 않는다. drop/pad·token/probability 공동 매핑·재조립을 명세한 뒤 실제 이름/품사를 정한다. | TopK/Gather/Scatter/SegmentedReduce; 모듈 전체를 필수 원자로 만들지 않음 | [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| P3 | [ ] NN-31 | `RNN`, `GRU`, `LSTM` (`torch.nn` module classes; builders, unexposed) | Adverb builders (후보; whole-sequence opaque 선택) | hidden/cell-state의 명시적 입력·출력, sequence lengths, direction, layout, gate order, 시간축 iteration과 zero-length sequence 의미. 전체 층은 J 합성을 우선하고 fused library가 필요할 때 Graph atom 유지 후보. | Scan/StateMachine + MatMul/Pointwise 또는 검증된 RNN library | [ONNX LSTM](https://onnx.ai/onnx/operators/onnx__LSTM.html) |
| P3 | [ ] NN-32 | `nms` (`torchvision.ops.nms`; not torch core) | Verb (후보) | detection box IoU·score sorting·threshold·tie-break·동적 선택 개수·빈 결과 shape; 단순 Elementwise/Reduce가 아닌 선택 알고리즘. | Grade/TopK + Select/Iterate 또는 전용 NMS kernel | [ONNX NonMaxSuppression](https://onnx.ai/onnx/operators/onnx__NonMaxSuppression.html) |
| P3 | [ ] NN-33 | `roi_align` (`torchvision.ops.roi_align`) | Verb/builder (후보) | ROI 좌표계·sampling grid·output bins·coordinate transformation·bilinear interpolation 및 출력 shape 명세. `grid_sample` 구현 가능성을 우선 검증. | GridSample + Reduce / library ROI Align | [ONNX RoiAlign](https://onnx.ai/onnx/operators/onnx__RoiAlign.html) |
| P3 | [ ] NN-34 | `ctc_loss` builder | Verb/derived builder (후보) | 가변 길이 시퀀스 alignment의 dynamic programming, blank token, 길이 mask, log-domain 안정성, impossible-alignment·gradient·reduction rule 명세. 일반 crossentropy의 별칭 아님. | LogSoftmax + Scan/StateMachine + Reduce 또는 전용 CTC kernel | [PyTorch CTCLoss](https://docs.pytorch.org/docs/stable/generated/torch.nn.modules.loss.CTCLoss.html) |
| — | EXT-R1 · 연구; A2/effect 소관 절 관리 | `store` | Adverb (연구) | 멀리 떨어진 forward/backward 소비자가 같은 결정 결과를 사용하게 한다. write destination·version·반환값·effect 순서가 필요하다. | 기존 A2 / effect 계약 참조 | [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md), [W](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/flow_storage_model_review_and_open_questions.md) |
| — | EXT-R2 · 연구; A2/effect 소관 절 관리 | `load` | Adverb (연구) | routing 결정을 재계산하지 않고 동일 version으로 재사용한다. resource identity·read version·미초기화 오류를 명세해야 하며 전체 POS/operand schema는 아직 확정되지 않았다. | 기존 A2 / effect 계약 참조 | [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md), [W](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/flow_storage_model_review_and_open_questions.md) |
| — | EXT-R3 · 설계 채택; A2에서 관리 | `with` | Conjunction | adjoint 관계·resource binding·numeric/storage 정책을 계산 identity와 함께 검증한다. RustJ 설계에서 채택했지만 runtime 구현 완료는 아니다. device/tile/register/layout은 받지 않는다. | 기존 A2 / effect 계약 참조 | [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| — | EXT-R4 · 탐색 연구; 구현 게이트 아님 | `FFT / IFFT` | Verbs (연구) | 일반 배열 언어의 확장성 연구 사례다. normalization·축·complex dtype 및 adjoint 관계를 확정하기 전 등록된 primitive로 표시하지 않는다. | 기존 A2 / effect 계약 참조 | [A](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md) |
| — | EXT-H1 · 직접 채택하지 않음 | `grad` | Adverb (역사) | 당시 gradient 누적/optimizer 분리 목적을 기록한다. 고정 offset과 inverse-slot 기반 학습 mode는 채택하지 않고 explicit write/accumulate resource 계약으로 재설계한다. | 기존 A2 / effect 계약 참조 | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [T](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/README.md) |
| — | EXT-H2 · 직접 채택하지 않음 | `consume` | Adverb (역사) | 중간 gradient 저장 축소라는 목적은 후보로 유지한다. 즉시 weight update를 기본 의미로 만들지 않고 이전 weight version의 모든 reader 및 effect legality를 먼저 검증한다. | 기존 A2 / effect 계약 참조 | [P](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [T](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/README.md) |

<a id="461-연구-사례와-core-j의-경계"></a>
#### 4.6.1 연구 사례와 core J의 경계

다음은 후기 연구의 확장성 검토 사례이며 위 prototype registry의 완성 항목이 아니다.

MatMul·FFT·Embedding·MoE 연구 항목까지 **위 단일 목록**에 통합했다. 과거 JAXA 이름은 출처 이력이지 실행 지원의 증거가 아니다.

`+`, `/`, `\`, `"`, `@:`, `[`, `]`, `@.`, `|:`, `:.`, `^:`는 core J 어휘이므로 확장 primitive로 중복 등록하지 않는다. core와 extension-derived op 모두 같은 semantic/capability/lowering 원칙을 따른다.

<a id="462-최신-연구와-rustj의-적용-경계"></a>
#### 4.6.2 최신 연구와 RustJ의 적용 경계

1. **`emit`·`cp`는 미결이다.** 후기 범위 문서 §8은 surface primitive, core effect, annotation/planner 표현을 다시 검토한다. 이전 문서의 “기본 어휘”만으로 채택 완료라 쓰지 않는다.
2. **`with`는 현행 설계에서 채택한다.** ordinary-name conjunction으로 typed semantic contract만 결합한다. 역사 문서의 optimizer/gerund 묶기 예는 자동으로 현재 operand schema가 되지 않는다.
3. **`adjoint`는 관계다.** 비선형에는 선형화 지점과 residual을 갖춘 VJP 계약이 필요하다. `:.`/`^:_1`의 inverse 의미를 adjoint로 바꾸지 않는다. `(loss_adjoint [ (loss emit))`의 parallel 후보는 바깥 fork syntax에서 나오며, `emit`이 실제 adverb binding이면 `(loss emit)`은 modifier application이다. 둘 다 verb일 때만 hook이다.
4. **dtype와 품사만으로 physical 실행을 확정하지 않는다.** 역사 cast 문서의 “adverb이면 fusion 강제”, “narrowing이면 in-place”, “widening이면 새 버퍼”는 현행 불변식이 아니다. numeric correctness·alias·use/liveness·target/cost 판단 후 실제 realization을 선택한다.
5. **상태는 explicit resource다.** prototype의 hidden weight/moment, fixed offset, 고정 barrier/stream/register 숫자를 primitive identity로 복사하지 않는다. `ValueId`, `StateResource`, `BufferId`를 구분한다.

<a id="nn-explicit-adjoint-candidates"></a>
<a id="462a-명시적-adjointvjp-후보-계약--nn-01nn-34-2026-10-11"></a>
#### 4.6.2a 명시적 Adjoint/VJP 후보 계약 — NN-01~NN-34 (2026-10-11)

**결정:** RustJ는 범용 자동미분/역방향 그래프 생성기를 필수 구성요소로 두지 않는다. Extended primitive의 Forward와 **입력별 명시적 Adjoint/VJP**는 각각 독립적인 배열 변환이며 동일한 J Rank/Cell/Frame, Graph IR, Logical IR, 합법성·비용 분석을 사용한다. 이 표는 위의 **단일 NN-01~NN-34 완료 체크리스트에 종속되는 계약 후보 표**이지 별도의 채택/완료 목록이 아니다. 표의 이름은 아직 등록된 J NAME이나 실행 가능 커널이라는 뜻이 아니다.

**수학적 경계:** 선형 A에는 A*가 정의된다. 비선형 f에는 primal x와 cotangent v를 받는 `VJP_f(x,v)=Df(x)^*v`가 필요하다. 비미분 입력(정수 index, mask, RNG draw, state mutation)에는 임의의 0-gradient/straight-through estimator(STE)를 자동으로 부여하지 않는다. 입력별 cotangent shape/dtype 및 복소 inner product를 검증한다. **Forward·Adjoint의 intrinsic rank가 같다고 가정하지 않으며**, 공유 parameter-gradient는 batch/frame 축 Reduce를 필요로 할 수 있다. J의 prefix agreement, 빈 Frame virtual fill-cell, heterogeneous result assembly, observable error/effect order는 각각 증명해야 한다.

**참고 기준:** [N] NNlib의 호출 가능한 `∇softmax` 등, [P] PyTorch `torch.nn.grad.conv2d_input/weight`, [C] cuDNN `ConvolutionBwdData/Filter`, [O] oneDNN Backward, [R] ChainRulesCore의 primal+pullback 규칙, [J] 과거 JAXA의 독립 `grad_x/grad_w` 및 관계 등록, [D] RustJ의 설계 도출(외부 라이브러리에 동일 이름/API가 있다는 주장이 아님). **외부 API는 참고 근거이지 J Rank·숫자·에러 의미론과의 동등성 증명이 아니다.**

| 계열 | Forward | 분류 | 입력 Adjoint 후보 | 파라미터 Adjoint 후보 | 필요한 primal/residual | Rank/Logical IR 방향 | 미결 계약/주의 | 근거 |
|---|---|---|---|---|---|---|---|---|
| NN-01 | `relu / sigmoid / tanh` | 명시적 VJP 후보 | `relu_vjp / sigmoid_vjp / tanh_vjp` | — | 입력 또는 출력 y, cotangent | Frame별 elementwise Map | ReLU x=0 subgradient, sigmoid/tanh 정밀도 | [D] |
| NN-02 | `linear` | 명시적 VJP 후보 | `linear_grad_input` | `linear_grad_weight, linear_grad_bias` | X, W, dY, input/weight shape | batched dW/db는 공유 축 Reduce | bias broadcast/축 정합 및 누산 정밀도 | [D] |
| NN-03 | `flatten` | 명시적 VJP 후보 | `flatten_vjp (= inverse reshape)` | — | 원본 shape/axis, dY | Frame/Cell 재배열; 복사 강제 금지 | 빈 배열·boxed/heterogeneous J 결과 조립 | [D] |
| NN-04 | `conv2d` | 명시적 VJP 후보 | `conv2d_grad_input` | `conv2d_grad_weight, conv2d_grad_bias` | W, X, dY, stride/padding/dilation/groups, input/output sizes | dX Window/Scatter; dW batched Reduce/Contract | 경계·그룹·padding, 산술 및 레이아웃 증명 | [P], [C], [O] |
| NN-05 | `avg_pool2d / max_pool2d` | 조건부 VJP 후보 | `avg_pool2d_vjp / max_pool2d_vjp` | — | window geometry; max의 선택 인덱스/동률 정책 | window별 Scatter + overlap Reduce | NaN/ties, ceil_mode, count_include_pad, 빈 창 | [O], [D] |
| NN-06 | `softmax / log_softmax` | 명시적 VJP 후보 | `softmax_vjp / log_softmax_vjp` | — | 출력 y, dY, reduction axis | Cell별 Reduce + Map | 안정적 수식, 빈 reduction, Inf/NaN 및 dtype | [N] |
| NN-07 | `cross_entropy` | 조건부 VJP 후보 | `cross_entropy_vjp_logits` | `선택적 class weight VJP 미정` | logits/log-prob 또는 확률, labels, reduction divisor | class-axis Reduce + batch-axis Reduce | 정수 target은 no VJP; ignore_index/label_smoothing 검증 | [D] |
| NN-08 | `conv1d / conv3d` | 명시적 VJP 후보 | `conv1d_grad_input / conv3d_grad_input` | `conv1d_grad_weight / conv3d_grad_weight, bias` | X, W, dY, spatial hyperparameters | spatial Window + batch Reduce | 차원별 layout, groups, dilations, boundary | [C], [O] |
| NN-09 | `layer_norm / rms_norm / batch_norm` | 조건부 VJP 후보 | `*_norm_grad_input` | `*_norm_grad_weight / *_norm_grad_bias` | input, mean/rstd 또는 rms, affine parameters, training flag | normalization axes Reduce, batch statistics 공유 | running-stat StateResource/version; forward residual precision | [O], [D] |
| NN-10 | `gelu / silu` | 명시적 VJP 후보 | `gelu_vjp / silu_vjp` | — | input x 또는 유효 residual, exact/approx mode | elementwise Map | 근사 GELU 모드별 서로 다른 derivative | [D] |
| NN-11 | `embedding` | 조건부 VJP 후보 | `indices: no VJP` | `embedding_grad_weight (scatter-add)` | indices, dY, padding_idx, weight shape | Gather 역방향 Scatter/SegmentReduce | indices 정수; 반복 index, max_norm state, sparse mode | [D] |
| NN-12 | `scaled_dot_product_attention` | 조건부 VJP 후보 | `attention_grad_q / _grad_k / _grad_v` | — | Q,K,V, mask, scale, softmax stats, dropout mask/key | Contract + Softmax VJP + Contract; head/frame Map | masked rows, GQA, dropout RNG, precision, flash backend 별도 | [D] |
| NN-13 | `rotary_embedding` | 조건부 VJP 후보 | `rotary_embedding_vjp (inverse rotation)` | `선택적 frequency VJP는 별도 정책` | position, sin/cos, rotated axes, dY | paired-element Map/StaticReindex | position index no VJP; frequency trainability 미정 | [D] |
| NN-14 | `matmul / J contraction` | 명시적 VJP 후보 | `matmul_grad_input` | `matmul_grad_other` | X, W, dY, transpose flags, batch-prefix frame | transpose/Contract + broadcast axes Reduce | 복소 conjugate adjoint, J prefix-rank 정합 | [O], [D] |
| NN-15 | `dropout` | 조건부 VJP 후보 | `dropout_vjp_input` | — | forward와 동일한 mask 또는 key/version, p, training | masked Map | 새 RNG draw 금지, RNG 재현성/effect barrier | [D] |
| NN-16 | `vjp (explicit invocation)` | 계산 VJP 해당 없음 | `등록된 input-specific VJP를 명시 호출` | — | primal, output cotangent, residual handle | 대상 primitive의 Rank/Cell 계약 상속 | forward graph 자동 미분/생성 금지; 미등록 VJP 오류 | [R], [J] |
| NN-17 | `SGD / Adam / AdamW` | 계산 VJP 해당 없음 | `일반적인 VJP 계약 없음` | — | weights, gradients, moments, step StateResource | elementwise state transition, 명시적 순서 | optimizer state update는 adjoint가 아님 | [D] |
| NN-18 | `register_adjoint (declaration)` | 계산 VJP 해당 없음 | `계산 없음: forward→input-index VJP 관계 선언` | — | dtype/shape/rank, primal/residual, input-slot identity | 등록 메타데이터만; Graph 실행노드 생성 안 함 | J :. / ^:_1 inverse 재정의 금지 | [R], [J] |
| NN-19 | `to (dtype conversion)` | 조건부 VJP 후보 | `floating_cast_vjp (정의 시)` | — | source/dest dtype, rounding/overflow policy | elementwise Map | 정수/양자화 cast는 일반 VJP 없음; STE 기본 채택 금지 | [D] |
| NN-20 | `conv_transpose2d / adaptive pooling` | 조건부 VJP 후보 | `transpose_conv_grad_input / adaptive_pool_vjp` | `transpose_conv_grad_weight, bias` | X,W,dY, output_size; adaptive window/indices | Window/Scatter + Reduce | adaptive max tie/NaN; output_padding/shape consistency | [C], [O], [D] |
| NN-21 | `quantize_per_tensor / dequantize` | VJP 없음 | `일반 양자화의 정확한 VJP 없음` | — | scale, zero_point, rounding, saturation, dtype | 직접 Adjoint 경로 없음 | STE는 별도 surrogate policy; VJP로 위장 금지 | [D] |
| NN-22 | `kv_cache_update` | VJP 없음 | `state mutation 자체 VJP 없음` | — | cache version, index/offset, alias, explicit state | StateResource read/write dependency | pure gather 경로의 VJP와 상태 갱신을 구분 | [D] |
| NN-23 | `grouped_mm` | 조건부 VJP 후보 | `grouped_mm_grad_input` | `grouped_mm_grad_weight` | expert assignment, offsets, X, W, dY | expert별 batched Contract + Reduce | routing index no VJP; capacity/drop/pad/reassembly | [D] |
| NN-24 | `logsumexp` | 명시적 VJP 후보 | `logsumexp_vjp` | — | input/output, reduction axes, stable max stats | Reduce + broadcast Map | empty/Inf/NaN, keepdims, accumulation dtype | [N], [D] |
| NN-25 | `topk` | 조건부 VJP 후보 | `topk_values_vjp (scatter to chosen indices)` | — | forward-selected indices, input shape, dValues | Scatter; repeated collisions require proof | index VJP 없음; ties/stability/NaN policy | [D] |
| NN-26 | `group_norm / instance_norm` | 명시적 VJP 후보 | `group_norm_vjp / instance_norm_vjp` | `affine scale/bias VJP` | per-group stats, input/scale, eps, dY | group-axis Reduce + Map, batch frame | group divisibility, no running stats 혼동 | [O], [D] |
| NN-27 | `segment_reduce / embedding_bag` | 조건부 VJP 후보 | `segment_reduce_vjp / embedding_bag_vjp` | `embedding_bag_grad_weight` | segments/offsets, indices, reduction type, argmax if max | SegmentView/Scatter + Reduce | empty segments, duplicate indices, max ties, sparse accumulation | [D] |
| NN-28 | `scatter_reduce / index_add` | 조건부 VJP 후보 | `scatter_reduce_grad_base / index_add_grad_input` | `scatter_reduce_grad_src / index_add_grad_src` | indices, reduce mode, include_self, winning ties | Gather + collision-aware ScatterCombine | overwrite collisions, min/max tie/order, deterministic accumulation | [D] |
| NN-29 | `interpolate / grid_sample` | 조건부 VJP 후보 | `interpolate_vjp_input / grid_sample_vjp_input` | `grid_sample_vjp_grid (mode dependent)` | grid/coords, input, mode, padding, align_corners | WeightedScatter/Window + Reduce | nearest grid derivative undefined; bounds, NaN, precision | [D] |
| NN-30 | `rand / randn / bernoulli / multinomial` | VJP 없음 | `RNG sampling 자체에 일반 VJP 없음` | — | explicit key/state, distribution parameters | ExplicitRng route, not differentiation | reparameterization/score estimator는 별도 API | [D] |
| NN-31 | `RNN / GRU / LSTM` | 조건부 VJP 후보 | `recurrent_vjp_input / _vjp_initial_state` | `recurrent_vjp_weight / _vjp_bias` | gate activations, hidden/cell states, lengths, direction | reverse-time Scan + per-step Contract/Map | zero-length, dropout, gate order/state version, truncation | [D] |
| NN-32 | `nms` | VJP 없음 | `discrete box selection: 일반 VJP 없음` | — | selected indices, ties, thresholds | selection graph only | continuous surrogate는 별도 op, NN-32 VJP 주장 금지 | [D] |
| NN-33 | `roi_align` | 조건부 VJP 후보 | `roi_align_vjp_features` | `ROI-coordinate VJP는 별도 정책/검증` | ROI coords, sampling grid, feature/input shapes | GridSample adjoint + accumulation | boxes/coordinates gradient 지원 범위 미확정 | [D] |
| NN-34 | `ctc_loss` | 조건부 VJP 후보 | `ctc_loss_vjp_log_probs` | — | log_probs, targets, input/target lengths, alpha/beta DP | log-domain Scan/StateMachine + Reduce | blank/zero_infinity, impossible alignment, padded frames | [D] |

[N]: https://fluxml.ai/NNlib.jl/dev/reference/
[P]: https://github.com/pytorch/pytorch/blob/main/torch/nn/grad.py
[C]: https://docs.nvidia.com/deeplearning/cudnn/latest/developer/graph-api.html
[O]: https://uxlfoundation.github.io/oneDNN/v3.12/dev_guide_convolution.html
[R]: https://juliadiff.org/ChainRulesCore.jl/stable/
[J]: https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md

**구현·수용 순서 (기존 단일 NN 체크리스트의 하위 게이트):**
- [ ] **NN-AG1:** 미분 가능한 입력 slot·cotangent/result shape/dtype·intrinsic/derived rank·axis/broadcast reduce·동적 shape guard를 Forward와 독립적으로 지정한다.
- [ ] **NN-AG2:** residual/activation의 값·버전·lifetime·storage/recompute 후보를 효과·오류 순서와 분리해 증명한다. `cp`/`emit`의 표면 문법 채택 여부는 기존 A2에서 결정한다.
- [ ] **NN-AG3:** 연산별 명시적 VJP를 독립 Graph 연산과 Logical decomposition 후보로 설계하며, 등록 선언은 실행 Graph node가 아니다. NN-16은 등록된 입력별 VJP의 **명시적 호출**만, NN-18은 `register_adjoint`라는 **관계 등록 후보**만 다룬다. `register_autograd`와 자동 backward 생성은 채택하지 않는다. J `:. `/`^:_1`의 inverse 계약은 불변이다.
- [ ] **NN-AG4:** dot-product adjoint 테스트(선형), finite-difference directional/VJP 검사(매끄러운 비선형), shape/rank·empty/frame·dtype/numeric·오류/effect 부정 사례를 수행한다. 테스트 도구가 역방향 프로그램을 자동 생성하는 것은 아니다.
- [ ] **NN-AG5:** NNlib/oneDNN/cuDNN/PyTorch 참조와 동등성 증거를 확보하고, Native/External lowering을 정식 선택하기 전 J 의미 차이·backend 지원 범위·workspace를 기록한다.

**진행 상태:** 34/34 계열 후보 계약을 문서와 `src/nn_extensions.rs::ADJOINT_CANDIDATES`에 연결함. **구현된 VJP 0개로 간주**하며, 의미론 적법성·실행·성능·CI 통과를 주장하지 않는다.

<a id="463-기존-조사구현-증거-기록-2026-10-02-스냅샷"></a>
#### 4.6.3 기존 조사·구현 증거 기록 (2026-10-02 스냅샷)

이 절의 코드 대조 기준은 **2026-10-02 조사 당시 checkout `89b87b8`**이다. 현재 `main`의 구현 상태를 뜻하지 않으며 최신 상태는 §12/A1.5를 따른다. `src/primitive.rs`의 `ExtensionPrimitive`/`PrimitiveResolver::resolve_extension_binding`과 `src/runtime.rs`의 parser name-binding seam은 존재한다. 그러나 이 seam 또는 테스트용 extension handle은 위 NN/effect family의 semantic contract와 실행 kernel을 구현한 증거가 아니다. `tests/semantic.rs::unknown_contracts_are_barriers`는 `conv`와 `with`의 unknown contract가 보수적으로 처리됨을 확인하는 기존 테스트다. 이번에는 코드를 읽었으며 실행 테스트를 재수행하지 않았다.

**조사 당시 수행한 작업(기록 전용; 현행 NN 진행 체크박스 아님):**
- 네 저장소의 당시 HEAD를 확인하고 commit 고정 출처로 기능·품사·연구 분류를 기록했다.
- 과거 forward/backward 및 `grad`/`consume`, 후기 cast/storage/registration 어휘를 구분했다.
- 당시 한국어 정본과 영어 mirror에 통합했다.

**기존 미완료 체크 4개의 관리 위치를 통일한다.** parameter/POS/rank는 **NN-G0**, shape/dtype/reference·empty/error는 **NN-G1**, effect/alias/StateResource는 **NN-G2**, 개별 실행 경로는 **NN-G5**, Windows/J 오라클 비교와 오류/효과 검증은 **NN-G6**에서 각각 추적한다. 공통 extension registry 및 분석 기반의 구현 상태는 **§A2**에서만 관리한다. 이 역사 절에서 별도 완료 상태를 더 이상 갱신하지 않는다.

당시 권장 첫 후보(`relu` → `linear`/`flatten` → `conv`/`avgpool2d`)는 현행 **NN-D1/NN-D2** 실행 계획으로 대체했다. 기존 M1–M6/frontend 이행 순서가 계속 우선하고 실제 CUDA 구현 보류는 유지한다.





<a id="463a-nn배열-extension-first-수용-게이트실행-체크리스트-2026-10-10-설계계획만"></a>
#### 4.6.3a NN/배열 Extension-first 수용 게이트·실행 체크리스트 (2026-10-10; 설계/계획만)

**결정:** 먼저 ordinary J 이름의 extension을 추가하고 **J Graph IR에서는 의미가 정의된 단일 계산 primitive**로 보존한다. 그다음 **Execution Semantic Lowering / Logical Execution IR**에서 검증된 조합으로 *선택적으로* 분해하거나, 원자 op를 유지한 채 native CPU, 검증된 외부 library, MLIR/StableHLO subset, GPU kernel로 내린다. "확장 → 나중에 native"는 **source API/semantic identity의 불변**을 뜻하며, Graph IR primitive를 반드시 core J 문법 primitive로 승격한다는 뜻은 아니다. 분해와 fusion, 특정 backend 사용은 별도 합법성/수치/비용 검증 결과다. [JAXA 원자/참조 정의 연구(A)](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md), [prototype 목록(P)](https://github.com/yunskim/JAXA-complier/blob/ceba0589a80fd1a1e0630b39f3f61c7bf0b6c368/docs/JAXA/primitives.py), [현행 A2 계약](./11-roadmap-checklists-p05.ko.md#a2--extension-primitive-registry와-analysis-contract).

**검증 전용 체크리스트:** 기존 §4.6.3의 미완료 4개를 NN-G0–G7에 합쳤으며, 개별 진행 상태는 **위 단일 표에서만** 관리한다. §A2는 공통 기반, 통합 표는 NN 항목별 수용을 관리한다. 아래 모든 미체크는 *미검증 도입 후보/미완료 과제*이며, 실제 사용 가능함을 의미하지 않는다. P0/P1/P2/P3은 **이 확장 track 내부의 상대 우선순위**이다. 기존 frontend·M3 검증·M4 CPU 실행 gate를 선행/차단하는 신규 milestone이 아니다. JAXA 명칭·품사는 기존 §4.6 prototype의 이력일 뿐, 신규 이름·표면 품사는 호환성 검증 후 확정한다. 특히 stateful layer는 hidden weight를 보유하는 primitive가 아니라 explicit `StateResource` 인자를 가진 계산 및 parameterized builder로 해석한다.

**2026-10-10 독립 연산 커버리지 감사(도입 후보만).** [ONNX 연산 목록](https://onnx.ai/onnx/operators/), [StableHLO 실행 의미론](https://openxla.org/stablehlo/spec), [PyTorch segment reduction](https://docs.pytorch.org/docs/stable/generated/torch.segment_reduce.html), [JAX explicit PRNG](https://docs.jax.dev/en/latest/jax.random.html)를 RustJ 기존 `ExecutionBasisKind`(`Gather`, `Scatter`, `ScatterCombine`, `SegmentView`, `Reduce`, `Scan`, `Grade`, `GroupBy`, `StateMachine`)와 독립 대조했다. **NN-24–NN-34의 누락 후보 11개**를 이 **같은 표**에 추가했고 기존 ID는 보존했다. P1–P3는 상대 우선순위이며 ID는 append-only이므로 표 내 번호가 반드시 정렬되지는 않는다. 고수준 extension 후보가 **새 execution-basis 종류 추가를 뜻하지는 않는다.** 특히 `scatter_reduce`·top-k는 현재 공통 basis/기존 J 기능을 활용할 수 있으므로 별도 source 이름이나 Graph atom은 정확성·성능 증거 후 선택한다. `pad`·`reshape`·`concat`·`split`·`where`·`sort`·`einsum`, residual/MLP/SwiGLU 블록, collectives는 별도 J primitive로 중복 등록하지 않으며 먼저 기존 J·derived verb·Logical IR 경로를 검증한다. 신규 이름/품사는 모두 제안이고, 외부 프레임워크와의 상호운용성 때문에 J Rank·빈 Cell/Frame·fill·오류·effect 의미론을 변경하지 않는다. 참조한 외부 명세 링크는 2026-10-10에 확인한 가변 문서이며 구현 커밋 고정 근거는 아니다. 새 연산 실행 또는 CI 통과를 주장하지 않는다.

**2026-10-11 PyTorch 명칭·intrinsic-rank·후속 spec 계약 (NN-D1a, frontend preview only):** 34개 고유 NN 가족 ID를 바꾸지 않고 공개된 `torch.nn.functional`, `torch`, `torch.optim`, `torchvision.ops`의 **leaf spelling**을 우선하여 목록의 이름을 정규화했다. 예: `cross_entropy`, `avg_pool2d`, `batch_norm`, `scaled_dot_product_attention`, `nms`. `NN-13`/ `NN-22` 등 일치하는 안정적인 PyTorch 공개 API가 없는 설계 후보는 공식 PyTorch op라 표기하지 않는다. PyTorch의 Python 호출 표면이 3인자 이상이더라도 J는 verb의 1·2항 계산과 adverb/conjunction의 parameterized derived verb 생성으로 구분한다. 이것은 PyTorch 문법을 J parser에 수입하지 않는다.

**기계 판독 spec:** `src/nn_extensions.rs`의 `FAMILIES`(34개)는 parameter schema, shape, dtype/promotion, effect, Logical route *candidate* 및 empty/fill·error-order·numeric proof를 각각 명시한다. `NAMES`는 실제 leaf name, PyTorch API 경로, POS, `intrinsic_ranks`/builder 생성 후 `derived_verb_ranks`, frontend-preview 사용 여부를 보관한다. 단항 elementwise(`relu`, `sigmoid`, `tanh`, `gelu`, `silu`)는 `[0,0,0]`; 전체 배열 연산의 직접 verb는 `[63,63,63]`(J unbounded, PyTorch batch broadcasting 아님); parameterized adverb의 **자체** 계산 rank는 `None`이고 생성될 verb의 후보 rank는 `[63,63,63]`이다. Builder axis/config가 미확정인 경우 생성 rank 역시 의미론 검증 전 잠정값이다. `63`을 J Rank의 일반 셀 분해나 축 기준으로 재해석하지 않는다.

**Frontend E2E 연결:** `nn_extensions::frontend_preview_context()`는 별도 명시 opt-in 엔진에서만 카탈로그의 `frontend_preview` NAME에 POS/semantic ID/lowering key를 붙인다. `examples/frontend_e2e.rs`는 원래 J pipeline 검증 이후 `NN_EXTENSION_FRONTEND ... status=RECOGNIZED_ONLY` 출력으로 이름·rank·후속 spec·미검증 gate를 조회하고 `tests/frontend_stages.rs`는 34개 ID·중복·NAME 유지·POS 인식·사용자 이름 우선을 검사한다. `Engine::new()` 기본 어휘는 변경하지 않는다. **현재 parser는 spec을 실행 가능한 FunctionEntity로 고정하지 않는다**; 이름 식별과 실제 Graph atom/CPU 실행/분해·비교 결과는 구분한다. 검증 전 `NN-G1–G7`, `NN-D1` 완료를 표시하지 않는다. PyTorch 참고: [functional API](https://docs.pytorch.org/docs/stable/nn.functional.html), [PyTorch tensor ops](https://docs.pytorch.org/docs/stable/torch.html), [Torchvision ops](https://docs.pytorch.org/vision/stable/ops.html).

**NN-04 / conv2d 전용 구현 계획:** [§7.10.7 단일 연산 CPU E2E 실행 체크리스트](./11-roadmap-checklists-p04.ko.md#conv2d-only-implementation-checklist)를 따른다. 실제 완료 상태는 위 통합 표의 **[ ] NN-04** 한 곳에서만 관리한다. **NN-D2의 conv2d+avgpool2d CNN·NN-D1 relu 완료는 이번 conv2d 한 연산의 선행 조건이 아니며**, NN-08/adjoint/GPU/SPORES·egg도 초기 완료 범위 밖이다. C2D-00은 문서 계획만 완료, C2D-01–08은 미구현·UNRUN이다.

**단일 목록 정책:** NN-01–NN-34의 우선순위·체크상태·품사·계약·분해 후보·출처는 **위 통합 표에서만 관리**한다. 이하에는 수용 검증 규약과 실행 단계만 남기며 기능 목록을 반복하지 않는다.

**의도적으로 extension primitive로 중복하지 않을 것:** `+`, `*`, `+/`, `|:`, Rank `"`, `@:`, fork/hook 등 기존 J 어휘; 일반 residual connection·MLP/Transformer block·모델/학습 loop는 **라이브러리의 derived verb 조합**을 우선한다. `matmul`/reshape처럼 기존 J가 표현하는 연산은 이름을 추가하더라도 최적화 경로를 기존 J 표현과 공유해야 한다. `with`는 이미 선택된 *typed semantic-contract conjunction*이고 device/tile 지정 통로가 아니다.

**Extension별 완료 조건 (각 NN-xx 항목을 체크하려면 해당 유형에 적용되는 gate를 모두 통과):** 계산 extension에는 NN-G3/G5의 Graph primitive 및 실행 경로가 필수다. `adjoint` 같은 **선언/등록 API**는 계산 Graph node로 위장하지 않고 별도 registry schema·정적 검증·테스트로 gate를 충족한다. `matmul`처럼 기존 J 계산을 그대로 인식하는 별칭은 별도 extension node를 강제하지 않고 동등한 Graph 분석/Lowering 증거를 제공한다.

- [ ] **NN-G0 / 공개 identity:** 일반 J name 및 실제 POS(verb/adverb/conjunction), builder operand schema, 생성된 verb의 valence/innate rank, late NAME/locale/rebinding 및 버전 지정. 파서 keyword 추가 금지.
- [ ] **NN-G1 / 정밀 의미론:** 표준 J 참조 정의가 가능하면 oracle로 제공하고, 그렇지 않으면 독립적인 수학/프레임워크 oracle을 버전 고정. axis 역할, frame/cell, empty-frame의 prototype/fill/assembly, empty cell, scalar, 동적 shape, type/promotion, NaN/Inf/signed zero, boundary/error precedence를 명세.
- [ ] **NN-G2 / 상태 및 미분:** effect/alias 및 명시적 `StateResource`와 `ValueId` 구분; RNG, 파라미터, buffer/version/read/write ordering; 학습 항목은 입력별 VJP, residual/gradient 합산 규칙. `BufferId`·layout·tile·device 숫자를 의미론에 넣지 않음.
- [ ] **NN-G3 / Graph IR:** 출처·binding/derived-function identity 및 계약을 갖춘 단일 primitive call로 Graph IR에 등록; unknown contract는 보수적으로 barrier/unsupported로 처리하며 실행 가능함으로 오인하지 않음.
- [ ] **NN-G4 / 선택적 분해:** *Graph IR 원본은 보존*하고, 다음 Execution Semantic Lowering/Logical IR에서 분해 후보 + semantic equivalence witness/guard + provenance를 만든다. 불충분한 증거, 수치 순서/오류/effect 차이 또는 비가역 레이아웃 변화 시 분해 불허; 원자 경로 유지 또는 명시적 unsupported.
- [ ] **NN-G5 / 실행 경로:** portable CPU/reference **또는** 검증된 external/library route 하나 이상의 실사용 경로, preconditions 및 fallback/unsupported를 증명. 이름/Graph 등록만으로 implementation complete 체크 금지. Native/GPU/fused/FlashAttention은 각각 독립 완료 상태.
- [ ] **NN-G6 / 검증 증거:** 정상/경계/음성 case의 value·dtype·shape·rank·J error/effect order 비교, 미분 gradient check(해당 시), 각 단계 verifier와 분해 on/off 동등성, target별 reproducible CI SHA·테스트 결과. J 호환식은 pinned jsource와, 신규 NN 의미는 버전 고정 oracle과 비교; 실행하지 않은 것은 `UNRUN`.
- [ ] **NN-G7 / 승격 기준:** 공개 J interface와 observable semantics 그대로 유지하며, 구체 workload의 의미·비용·성능 근거를 확보한 경우에만 optimized native realization/패턴 recognition을 추가. Graph IR atom, Logical 분해, native kernel, **J core primitive 편입**을 서로 다른 결정으로 기록.

**트랙 진행 체크리스트 (설계만 기록; 구현·CI 완료로 간주하지 않음):**

- [x] **NN-D0:** 기존 §4.6 JAXA/Japchae prototype 근거 및 현행 A2/IR 불변식에 연결된 도입 후보/우선순위·진입/종료 gate를 문서화.
- [ ] **NN-D1:** P0 `relu` 한 개에 대해 name binding → Graph atom → reference CPU → decomposition/no-decomposition 동일 결과 vertical slice 및 음성 verifier 실증.
  - **NN-D1a (registration admission, partial implementation):** `PrimitiveResolver::with_extensions`의 유효 NAME·중복·semantic identity/lowering key·registry version 사전 검증과 음성 회귀 테스트를 추가하는 단계. 이는 등록 계약 검증만이며 `relu` Graph atom, CPU 실행, J Rank/빈 배열 및 분해 동등성은 미검증/미지원이다. PR-head CI·병합 뒤 main CI가 확인되기 전에는 PASS로 기록하지 않는다.
- [ ] **NN-D2:** `linear` + `flatten` + `softmax` 최소 MLP, `conv2d` + `avgpool2d` 최소 CNN으로 Shape/Rank/empty-frame 증거 수집.
- [ ] **NN-D3:** P1 attention/embedding/norm 연결로 작은 Transformer inference; 마스킹과 numeric stability/target route 차이 검증.
- [ ] **NN-D4:** P2 backward/optimizer/state/AD 관계는 명시적 effect·gradient 계약 확보 후 별도 구현; P3은 구체 workload와 외부 커널 수요가 확인된 뒤 판단.
- [ ] **NN-D5:** 각 완료 항목을 §A2/해당 M-stage gate 및 영어 mirror와 일치시키고, 정확한 main SHA/CI/미검증 범위를 기록.
- [ ] **NN-D6:** NN-24–NN-30을 기존 J/ExecutionBasis로 우선 실현할 수 있는지 확인하고, 새로운 Graph atom·execution basis는 별도 의미론 증거·reference oracle이 확보된 경우에만 추가한다. NN-31–NN-34는 실제 RNN/영상/CTC workload 수요 확인 뒤 구현 또는 명시적 보류를 결정한다.

**참조/경계:** [§4.6 단일 확장 목록](#nn-extension-checklist), [A2 extension/analysis checklist](./11-roadmap-checklists-p05.ko.md#a2--extension-primitive-registry와-analysis-contract), [Graph vs Execution Basis](./11-roadmap-checklists-p04.ko.md#a15--j-graph-ir--jaxa-array-operation-graph-ir), [JAXA 후기 설계 문서(A)](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md). 과거 문서의 "conv는 hidden weight 소유 blackbox"는 RustJ의 **explicit StateResource** 규약보다 우선하지 않는다. 이 절은 신규 구현과 검증을 주장하지 않는다.


<a id="47-과거-jaxajapchae-저장소-통합-기준"></a>
### 4.7 과거 JAXA/Japchae 저장소 통합 기준

2026-09-30에 다음 네 저장소의 최신 내용을 다시 대조했다. **2026-10-02 재확인한 원격 HEAD와 확장 primitive별 최신 대조는 [§4.6](#extension-primitive-inventory)를 따른다.** 이 표의 substantive baseline은 당시 조사 이력이다.

| 저장소 | 검토 기준 | 이 문서에 흡수하는 핵심 |
|---|---|---|
| `yunskim/JAXA` | main `12bc0659`, 2026-03-24 | 정적 graph, fusion, J 조합 구조를 이용한 NN 표현이라는 초기 문제의식 |
| `yunskim/JAXA-complier` | main `ceba0589`, 2026-04-26 | custom primitive registry prototype, rank/shape/analyzer metadata의 실제 자료구조 |
| `yunskim/japchae` | main `510c31b5`, 2026-06-19 | primitive identity/realization 2층 모델, resource function, materialized arrays, analyzer output schema |
| `yunskim/jaxa-analyzer` | substantive baseline `3eef3942`, 2026-07-28; 2026-09-30에는 RustJ 이관 상태 주석 추가 | J frontend/vocabulary/name extension, semantic AST, primitive contract, Flow–Storage/resource 분석의 최신 정리 |

앞으로 위 저장소들은 **historical research/prototype source**다. 설계 결정을 수정할 때 원본을 다시 고쳐 여러 갈래를 유지하지 않고 이 `PROJECT.ko.md`를 갱신한다.

결정 충돌 시 단순한 repository 날짜보다 **같은 주제에 대한 후속 결정**을 우선한다. 대표적인 예:

- 초기 `JAXA`의 complex rank/precision 표기 `"RjP`는 표준 J와 호환되지 않으므로 채택하지 않는다.
- J의 rank conjunction `"`은 표준 J 의미 그대로 유지한다.
- dtype/precision, accumulator precision, layout, hardware resource는 rank 문법에 억지로 넣지 않고 각각 semantic contract와 planning 계층에 둔다.
- 4월 prototype의 고정 `memory_layout`, `tiling_axis`, register 숫자는 최종 semantic identity가 아니다. 6~7월의 identity/realization 분리는 중요한 중간 단계였고, 현행 RustJ에서는 이를 더 분리하여 `PrimitiveSpec semantic record + capability interfaces + lowering registry + TargetProfile`로 사용한다.

<a id="471-역사-저장소와-현행-rustj의-충돌-해소표"></a>
#### 4.7.1 역사 저장소와 현행 RustJ의 충돌 해소표

과거 문서의 문장을 그대로 현재 설계로 읽지 않는다. 다음 항목은 명시적으로 재해석한다.

| 과거 JAXA/Japchae 주장 | 현행 RustJ 결정 |
|---|---|
| JAXA graph는 **parse time에 완전히 확정**된다 | parser만으로 충분하지 않다. resolvable name/품사 binding과 Semantic Analyzer를 거쳐 verified Logical IR이 만들어진다. dynamic constraint는 witness/guard로 남을 수 있다. |
| JAXA는 **J 전체가 아닌 제한된 vocabulary 언어**다 | RustJ의 언어 목표는 장기적으로 J 전체 의미다. 다만 hardware-aware Logical Array IR 및 advanced optimization route에 들어갈 수 있는 영역은 별도의 **analyzable array profile/subset**일 수 있다. |
| custom primitive spelling을 enqueue에서 built-in 실패 후 직접 가로챈다 | extension spelling은 reserved keyword가 아니다. ordinary J name environment에 binding으로 등록한다. enqueue는 ordinary NAME/lookup metadata만 만들고, parser가 정상 J name lookup을 통해 현재 binding과 품사를 얻는다. registry 추가 때문에 tokenizer/parser 구현을 수정하지 않는다. |
| `conv`, `linear`은 computational verb다 | 최신 prototype의 표면 품사는 parameterized **adverb**다. noun parameter를 받아 derived computational verb를 만든다. analyzer가 보는 Conv/Linear logical op identity는 이 derived verb에서 나온다. |
| prototype의 `rank_monad/rank_left/rank_right` 값을 builder의 영구 rank로 본다 | builder와 derived verb를 분리한다. rank는 parameter 적용 후 생성된 derived computational verb의 contract에서 확정한다. 예: derived `conv_kd`는 cell rank `k+1`. |
| rank가 같으면 fusion 가능하고 rank 변화가 fusion boundary다 | rank/cell/frame은 중요한 입력이지만 fusion legality의 충분조건이 아니다. access/dependency/effect/storage/speculation과 schedule/target까지 함께 본다. |
| flatten/reshape/transpose는 본질적으로 항상 stride remap이라 copy가 없다 | semantic level에서는 StaticReindex/view 후보일 뿐이다. 실제 view 유지, layout absorption, copy/materialization은 representation과 downstream consumer/target이 결정한다. |
| primitive가 `memory_layout`, `tiling_axis`, register/shared-memory 숫자를 가진다 | semantic primitive에는 axis/access/numeric/effect contract만 둔다. target-dependent resource/layout은 lowering/resource model + TargetProfile + Schedule에서 결정한다. |
| “JAXA analyzer”가 hardware profile을 받아 fusion/resource를 결정한다 | 옛 analyzer가 여러 단계를 한 이름으로 묶었다. 현행 RustJ의 **Semantic Analyzer는 target-independent**이고, hardware-dependent 부분은 RoutePartition 이후 native Schedule/Physical Planner/ResourceEstimate 또는 external compiler가 담당한다. |
| `PrimitiveResourceSpec`에 temporary/register/shared/instruction cost를 함께 둔다 | semantic requirement와 realization/cost를 분리한다. accumulator/reduction/reuse 같은 구조는 capability에, concrete resource expression은 lowering/resource model에, empirical performance는 CostEstimate에 둔다. |
| 모든 materialized array는 compile-time fixed offset을 가져야 정적 모델이 완성된다 | semantic resource identity/lifetime이 정적으로 알려질 수는 있지만 **physical offset은 semantic requirement가 아니다**. native late bufferization이나 external compiler가 실제 allocation/offset을 결정한다. |
| single stream 순서가 region 간 dependency의 기본 보장이다 | 특정 stream 가정은 architecture invariant가 아니다. physical async dependency는 explicit token/timepoint/event 또는 backend dependency로 표현한다. |
| fusion하지 않으면 중간값은 사실상 DRAM으로 간다 | 특정 GPU 구현의 직관일 뿐 architecture invariant가 아니다. cache, persistent kernel, producer-consumer scheduling, external backend가 다른 realization을 선택할 수 있다. 핵심은 logical value와 physical materialization을 분리하는 것이다. |
| mutable optimizer state를 stateful verb 내부에 둘 수 있다 | 후기 Japchae 결정대로 **mutable array state는 verb/primitive 밖의 explicit resource로 드러낸다.** weight, grad, optimizer state, checkpoint는 역할이 아니라 lifetime/effect/storage requirement로 구분한다. |
| weight/grad/activation을 모두 하나의 flat “materialized array object” 종류로 둔다 | J 의미상 모두 array라는 통찰은 유지하지만 IR identity는 분리한다. ephemeral computation result는 SSA `ValueId`, persistent/mutable named state는 `StateResource`, 실제 materialized storage는 downstream `BufferId`다. activation도 저장이 필요할 때만 StorageRequirement/BufferId를 얻는다. |
| `with`에 optimizer/adjoint/hardware/dtype/tile 정보를 모두 넣는다 | `with` **conjunction 자체는 채택**하되, 서로 다른 planning 층을 평평하게 섞는 방식은 채택하지 않는다. `with`는 ordinary J name에 binding된 conjunction으로 typed semantic annotation/contract만 결합하며 tile/device/register 같은 physical policy는 받지 않는다. |
| parameterized layer/verb는 weight storage 때문에 반드시 source name을 가져야 한다 | 계산 entity의 이름과 state resource identity를 분리한다. derived verb는 익명일 수 있고, 필요한 mutable state는 explicit `StateResource` identity로 참조한다. |
| graph를 남기려면 noun reduction/evaluation을 일반적으로 금지해야 한다 | RustJ 전체 J semantics에는 적용하지 않는다. J의 noun/value evaluation은 그대로 보존하고, compiler가 필요한 verb/adverb/conjunction composition을 `J Semantic Array IR`에서 별도로 first-class로 유지한다. |
| `load/store/emit/cp`는 JAXA의 기본 in-band memory vocabulary다 | core J semantics로 자동 채택하지 않는다. 필요한 경우 ordinary name/adverb extension으로 등록하고 `EffectSemantics + StorageRequirement/StateResource` contract를 갖춘 analyzable profile 기능으로 다룬다. |
| semantic/resource contract는 Python registry가 제공한다 | Python은 역사 prototype 구현 선택이다. 현행 RustJ architecture는 Rust capability interfaces, versioned data profiles, external adapter/registry를 사용하며 Python runtime dependency를 요구하지 않는다. |
| `requires_sync=true`가 primitive semantic property다 | logical contract에는 dependency/collective/conflicting-update 요구만 둔다. barrier/event/atomic 등 구체 synchronization은 schedule/target 이후 정한다. |
| `in_place=true`가 primitive의 고정 실행 property다 | semantic 쪽에는 alias/destination legality(`DestinationRelation`)만 둔다. 실제 in-place reuse는 liveness/conflict/bufferization 이후 결정한다. |
| `cuda_family`가 primitive identity 일부다 | backend/architecture 정보는 target lowering locale/registry의 metadata다. source/semantic primitive identity와 분리하고, BackendFamily/ArchitectureTarget/DeviceProfile을 각각 구분한다. |

이 표는 역사 저장소의 아이디어를 폐기한다는 뜻이 아니다. **어느 층에 속하는지를 현재 compiler architecture에 맞게 재배치**하는 기준이다.


<a id="48-확장-primitive는-ordinary-name-binding으로-추가하고-parser-time-lookup을-따른다"></a>
### 4.8 확장 primitive는 ordinary name binding으로 추가하고 parser-time lookup을 따른다

NN/array extension은 J의 새로운 keyword나 punctuation을 추가하지 않고 **ordinary name**으로 추가한다.

current jsource(`p.c`, `w.c`)의 중요한 semantic 경계는 다음과 같다.

```text
source word "conv"
      ↓
Tokenizer / Enqueue
  ordinary NAME
  source/name metadata
  lookup-name flag / optional lookup hint
      ↓
J Parser
  name을 stack에 올릴 시점에
  current local/locale environment에서 lookup
      ↓
현재 binding의 실제 품사
  noun | verb | adverb | conjunction
      ↓
J reduction / derived entity construction
```

즉 **enqueue가 ordinary name의 품사를 미리 고정하지 않는다.** parser가 문장을 처리하면서 현재 binding을 lookup하고 그 value/type class를 사용한다.

extension registry의 역할은 reserved-word table이 아니라 **ordinary J name environment를 seed/register할 extension entity와 semantic/lowering metadata를 제공하는 것**이다.

예를 들어 초기 environment에:

```text
conv  -> ExtensionAdverb::Conv
with  -> ExtensionConjunction::With
relu  -> ExtensionVerb::Relu
```

를 binding할 수 있다. 그 뒤 parser는 extension 여부를 알 필요 없이 정상 name lookup을 한다.

중요한 불변식:

1. Tokenizer/Enqueuer는 `conv`, `with` 등의 spelling을 특별 token/POS로 hard-code하지 않는다.
2. Enqueue는 ordinary NAME 및 lookup metadata/hint를 만들 뿐 semantic 품사를 확정하지 않는다.
3. Parser가 해당 name을 사용할 때 **현재 local/locale binding을 lookup**하여 noun/verb/adverb/conjunction class를 얻는다.
4. 사용자가 정상 J binding 규칙으로 extension name을 shadow/rebind하면 그 현재 binding semantics가 우선한다.
5. noun name은 jsource처럼 value로 resolve될 수 있고, 일반 verb/adverb/conjunction name은 late binding을 보존하는 name-reference semantics가 필요하다.
6. 따라서 `f =: conv`를 항상 “현재 Conv builder identity의 정적 복사”라고 가정하지 않는다. J의 name-reference semantics를 보존하고, static binding이 증명된 경우에만 stable extension identity로 specialize한다.
7. parameterized adverb를 실제로 적용하여 derived verb가 만들어진 뒤에 그 derived computational entity의 semantic capability를 분석한다.
8. 새 extension을 추가할 때 tokenizer/enqueuer/parser의 **코드**를 수정하지 않고 ordinary binding data + semantic/lowering capability를 추가한다.

4월 `JAXA-complier` prototype의 “built-in lookup 실패 후 custom registry를 검사” 방식은 사용하지 않는다. extension도 J의 normal name-resolution 경로에 참여한다.

static specialization이 유용할 경우:

```text
NameRef
  name / locative semantics
  expected_part_of_speech
  + optional binding/version guard
  + optional proven ExtensionAdverb::Conv
      ↓
specialized derived entity
```

처럼 guard/proof를 남긴다. name을 compile time에 봤다는 이유만으로 향후 rebinding 가능성을 제거하지 않는다.

current jsource의 nameref execution(`sc.c`)은 lookup된 현재 value가 nameref 생성 시 기대한 품사와 같은지 검사하고, 달라졌으면 domain error를 낸다.

따라서:

```text
NameRef(expected = Verb)
  runtime lookup -> Verb        OK
  runtime lookup -> Adverb      DomainError
  runtime lookup -> Noun        DomainError
  runtime lookup -> undefined   ValueError when invoked/resolved as required
```

처럼 **late binding과 expected part-of-speech contract를 동시에 보존**한다.

<a id="481-실행-전-입력-정보--with-xy-후보와-metadata-검사"></a>
#### 4.8.1 실행 전 입력 정보 — ``with (X`Y)`` 후보와 metadata 검사

**사용자 의도:** 실행 전 분석·최적화에 필요한 입력 정보를 어떻게 확보할지 검토한다. `with`/gerund 채택은 필수가 아니며 아직 표면 schema를 확정하지 않는다. 기존 noun metadata 추론, 외부 API/catalog, 파일·데이터셋 header/schema 또는 optional annotation에서 facts를 받아 같은 분석 경계로 전달할 수 있다. header 조회도 실제 I/O이므로 effect를 숨기지 않는다. graph topology는 shape 없이도 일부 분석 가능하고, dtype·rank·extent·차원 관계를 알수록 더 정확한 legality/resource 분석이 가능하다. 값에 의존하는 조건은 metadata로 증명하지 않는다. frontend 작업 우선순위와 optimizer 실행 보류는 유지한다.

**출처 확인(2026-10-04):** `jaxa-analyzer`의 pinned `7275d5ba` 아키텍처 문서 §7.7-2, entry-point 항목에 `X =: 2 3 source with fp16`, `Y =: 2 3 source with fp16`, ``run =: (#@:[ ([ optimizer (''"_)) graph) with (X`Y)`` 제안이 있다. 같은 절의 `with` 항목은 annotation/adjoint/optimizer 묶기를 제안하며, 뒤 항목은 dtype·hardware·tile 정보를 평평하게 섞는 문제를 명시한다. 이는 제안/검토 이력이며 실행 구현 완료의 근거가 아니다. `X/Y`는 여기서 학습 배열 전체가 아니라 **source에 계약을 붙인 entity**로 해석할 후보다. 표준 J의 tie는 verb의 atomic representation을 담는 noun을 만들며 noun operand에서는 concatenation을 수행한다. 따라서 noun으로 binding된 학습 배열에 backtick을 쓴다고 외부 입력 handle이 자동으로 생기지 않는다. [JAXA 원 제안](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md#L861), [J tie](https://www.jsoftware.com/help/dictionary/d610.htm).

**RustJ 설계 예시 — 아직 실행 가능한 extension 문법이 아니다:**

```j
X =: 2 3 source with fp16
Y =: 2 3 source with fp16
run =: graph with (X`Y)
```

원문의 표면 형태를 유지하되 `source`, `fp16`, `with`는 모두 ordinary binding이다. tokenizer/parser에 spelling별 규칙을 추가하지 않는다. `with`의 semantic adapter는 gerund의 원본 noun·순서·span·NameRef를 보존하고, 등록된 descriptor/annotation capability만 읽는다. 임의 verb를 호출해 metadata를 얻거나 외부 데이터를 로드하지 않는다. capability를 모르면 분석 경계를 보고하고 실제 J 문법을 invalid로 판정하지 않는다. 실제 source 호출의 I/O/state effect는 별도 contract로 남긴다.

`with`를 선택하는 경우의 후보 입력 descriptor는 schema version, input identity/slot, dtype/rank/shape facts 및 provenance를 담는다. dyadic input bundle에서는 두 entry의 좌·우 slot을 명시·검증하고 원문의 X/Y 순서를 보존한다. 단순 두 항목이라는 이유로 adjoint/optimizer 묶기를 입력으로 해석하지 않는다. 함수 이름의 observed target를 쓸 때에는 ordinary NAME late binding 및 별도 runtime binding/version guard가 필요하다. 실제 noun 입력은 parser에서 읽을 때의 snapshot을 유지하며, 이를 function처럼 지연시키지 않는다.

전달할 정보의 상세 분류·필요 수준·현재 구현 경계는 [§4.8.2 입력 정보 명세](#preexecution-input-information)를 따른다.

큰 데이터셋은 batch별 실제 noun을 입력에 연결한다. IR은 `ReadNoun`와 facts를 보유하고 원소 전체를 literal로 넣지 않는다. 같은 계약의 batch는 같은 분석 구조를 사용할 수 있지만 **실행 계획 재사용에는 별도의 함수·effect·runtime guard가 필요하다.** parameter 값은 step마다 바뀌어도 payload를 compile constant로 고정하지 않는다. contents/pointer/runtime noun version을 무조건 compile-cache key로 삼지 않는다. cache는 graph/contract/schema 및 필요한 semantic/target specialization을 기준으로 설계하고 실제 NAME witness를 별도로 확인한다. 값 의존 결과 크기는 unknown/guard 경계로 남긴다. 학습 pipeline/optimizer 실행 구현은 이번 범위가 아니다.

**이번 구현:** `StaticAnalysis::validate_noun_inputs()`가 사용된 noun 선언과 live `&Value`의 dtype·rank·shape를 검사한다. 배열 원소를 읽거나 clone/retain하지 않으며 supplied metadata만 검사한다. type→rank→extent 순으로 Domain/Rank/Length를 반환하고 missing NAME은 Value, duplicate/extra 입력은 Domain으로 거부한다. unknown facts는 새로운 제약을 만들지 않는다. catalog revision은 runtime witness가 아니며 이 검사는 함수 binding·effect·alias·device·값 범위 또는 전체 실행 안전성을 증명하지 않는다. 현재 runtime 실행 경로에 자동 삽입한 guard도 아니다. `with` 해석은 아직 미구현이다.

체크리스트(기존 frontend 우선순위 유지):

- [x] **WI0** 원 제안과 J tie 의미를 확인하고 입력·adjoint·numeric·physical 정보의 경계를 정본에 통합한다.
- [x] **WI1** 데이터 없는 noun 분석 결과에 live noun metadata 검사 API를 추가한다. 서로 다른 batch 수용, dtype/rank/extent·missing/duplicate/extra 거부, unknown/partial facts·empty array·함수 미지원 유지 및 payload pointer/refcount 불변을 Windows regression 4개로 검증한다.
- [ ] **WI2 (선택 후보)** 등록된 source/input descriptor와 typed bundle schema를 정의하고 gerund construction의 원본·순서·lookup timing/provenance를 유지하는 `with` semantic adapter를 구현한다. ordinary noun operand와 잘못된 descriptor를 별도로 검증한다.
- [ ] **WI3** 전달 문법과 독립적으로 input facts를 static catalog/ReadNoun에 연결하고 함수 late binding·redefinition·POS 변경·unknown capability의 C/frontend regression을 추가한다. annotation 없이도 가능한 추론은 유지한다.
- [ ] **WI4** 실제 input binding/lifetime 및 guard failure 경로를 연결한다. effect 이전 검증·batch 교체·부분 실패·계약 충돌을 확인한다. 외부 버퍼/mmap/device adapter는 별도 physical 작업이다.
- [ ] **WI5** 기호 차원·기호 동등성·캐시 전제 및 학습 state contracts를 후속 구현한다. optimizer/CUDA/AD 실행은 계속 보류한다.

비교 근거: [JAX abstract evaluation](https://docs.jax.dev/en/latest/601/jax-primitives.html)은 값 없이 shape/type을 분석하고, [PyTorch export](https://docs.pytorch.org/docs/stable/export)는 input/parameter와 dynamic shape constraints를 구분한다. 이들은 descriptor 기반 분석의 참고이며 J NAME timing을 대체하지 않는다.

<a id="preexecution-input-information"></a>

<a id="482-실행-전-입력-정보-명세-2026-10-04"></a>
#### 4.8.2 실행 전 입력 정보 명세 (2026-10-04)

**필요성은 optimization별로 다르다.** full J 실행에 아래 모든 선언을 요구하지 않는다. topology 분석에는 known callable/graph만으로 충분한 부분이 있고, shape/type-dependent lowering에는 해당 facts 또는 runtime 조건이 필요하다. 각 입력에 대해 최소한 **어느 graph 입력인가 + noun이라는 품사 + known/unknown 구분 + facts의 근거/유효 scope**를 연결한다. 그 위에 dtype/rank/shape를 확보한 만큼 분석한다. schema는 `with` 문법과 독립적이며 noun metadata·외부 API/catalog·file header·선택 annotation에서 같은 facts를 얻을 수 있다.

| ID / 분류 | 알고 싶은 정보와 예시 | 필요한 분석·최적화 | 확보/검증 방법과 현재 경계 |
|---|---|---|---|
| IN0 입력 식별 | graph input ID, NAME/slot, schema revision, noun snapshot 시점 | graph edge 연결, 잘못된 입력 연결 방지 | catalog/호출 mapping; catalog version은 runtime NAME witness가 아님. 현재 used-input catalog와 missing/duplicate/extra 검사 존재 |
| IN1 원소 타입 | J logical dtype, numeric category; 외부의 fp16/fp32 등 정밀 dtype은 explicit conversion/encoding 계약과 연결 | type propagation, 합법 연산·kernel 후보, 저장 크기 | noun/header/API. 현재 TypeFact와 dtype 검사 존재; `Value::Float` CPU payload는 f64이며 fp16/fp32 NN 입력 지원을 뜻하지 않음 |
| IN2 rank·shape | rank=2, shape=[256,784]; scalar/empty 및 unknown 포함 | rank/cell split, agreement, 결과 shape·logical extent | shape가 있으면 rank/count 유도; 현재 concrete shape/unknown·rank 및 shape 검사 지원. J prefix agreement를 NumPy broadcasting으로 바꾸지 않음 |
| IN3 차원 관계·범위 | X=[B,K], Y=[B,N], W=[K,N]; X.B=Y.B, 0≤B≤1024 | graph 전역 shape 관계, dynamic-size 분석, 메모리 상한·guard | shared symbolic scope/constraint 및 실행 시 metadata 검사 필요. shape 미확정과 동일 기호는 다름. 0/1/empty를 임의 배제하지 않음. 현재 기호 차원/관계 guard 미구현 |
| IN4 논리 representation·중첩 schema | dense/axis-sparse/boxed; sparse axes/fill schema·stored count, boxed child dtype/shape가 known인지 | sparse/boxed legality, 결과 assembly, 비용/extent 분석 | J-visible facts와 storage encoding 구분. 일부 representation facts 존재하나 WI1 검사는 이 조건을 확인하지 않음. structural count를 sample에서 추정해 semantic proof로 쓰지 않음 |
| IN5 실행 중 변동과 작은 상수 | 데이터·학습 weight는 runtime 입력; axes/rank/window 등 일부 작은 constructor noun은 값이 필요할 수 있음 | constant folding, modifier construction, relevant specialization key | 값이 필요한 operand만 explicit constant/witness로 제공. 데이터셋·weight 내용을 compile literal/cache key로 넣지 않음; value-dependent constructor는 현재 경계 유지. [JAX static arguments](https://docs.jax.dev/en/latest/aot.html) |
| IN6 의미상 역할·effect | batch/data/label/parameter/gradient/state, read/write/accumulate, step dependency, 선택 AD target | 상태 갱신 순서, 학습 graph/AD, side-effect legality | 역할은 dtype나 대문자 NAME에서 추정하지 않음. 실제 function/effect contract와 StateResource에 연결; training/AD 통합 미구현. [PyTorch graph signatures](https://docs.pytorch.org/docs/main/user_guide/torch_compiler/export/api_reference.html) |
| IN7 값 성질 | KnownRange, finite, nonnegative, sorted, unique, valid indices 등 | bounds/check elimination, search specialization, 값 의존 결과 shape | proof/명시 계약/실행 시 검사. 표본 통계는 cost hint일 뿐 legality proof가 아님; full array 검사에는 O(N) 비용이 들 수 있음. 현재 WI1은 내용 미검사 |
| IN8 numeric policy | cast, accumulator precision, overflow/promotion, cct/fit, NaN/Inf, reassociation·determinism 허용 범위 | fusion/reduction 재배치·mixed precision의 합법성 | input dtype만으로 결정 불가; 기존 operation/semantic policy에서 확보. float16 선언만으로 J 수치/오류 의미를 바꾸지 않음 |
| IN9 수명·소유권·alias | 외부 owner/release, 다른 입력과 overlap, read-only/mutation, 호출 후 reuse/ownership donation 가능 여부 | buffer reuse/in-place, 안전한 외부 import, memory retention | adapter/런타임 borrow·ownership 증거 및 liveness 분석. logical noun identity와 physical BufferId 분리. WI1은 수명 보장/alias/donation을 검사하지 않음. [JAX donation](https://docs.jax.dev/en/latest/buffer_donation.html) |
| IN10 실제 representation·target | device, physical dtype/encoding, strides/offset/layout/alignment, transfer/sharding, target capability | concrete kernel/route/schedule 및 실제 byte/transfer 비용 | representation adapter + TargetProfile에 보관; semantic `with` 입력 계약에 섞지 않음. 관측 layout을 specialize하면 해당 guard 필요. [PyTorch tensor guards](https://docs.pytorch.org/docs/main/user_guide/torch_compiler/torch.compiler_dynamo_overview.html) |
| IN11 facts의 근거·유효성 | unknown/관측/추론/명시 조건/proof, scope/version, runtime guard·충돌 처리 | 최적화 전제 추적, stale specialization 방지, cache reuse | 모든 facts에 provenance 또는 witness 연결. 표본·관측을 영구 상수로 승격하지 않음. caller/locale/NAME witness와 payload snapshot을 구분. 현재 catalog revision은 분석 내 revision만 제공 |

IN0–IN5는 input schema/facts, IN6–IN8은 실제 연산/state 계약과의 연결, IN9–IN10은 runtime/physical 경계, IN11은 전체 facts에 적용되는 provenance다. 모두를 immutable FunctionEntity에 넣지 않는다. argument-dependent facts는 call/analysis에, 증명은 pass의 witness에, physical facts는 downstream representation에 둔다. axis 의미(batch/channel/feature 등)는 descriptor 해석·shape 관계에 유용한 optional label이며 표준 J rank/axis semantics를 대신하지 않는다. 실제 device는 입력 metadata가 될 수 있지만 tile/kernel 선택은 compiler가 도출하는 계획이다.

최적화별 요구 수준:

| 목적 | 주요 정보 | 모를 때의 처리 |
|---|---|---|
| topology·공통 입력·분기/합류 후보 발견 | graph/callable identity, input 연결, source provenance | known 구조만 분석; 미확정 callable은 경계로 유지 |
| 연산 fusion/재배치의 legality | 위 구조 + operation effect/error/numeric/alias 계약, 필요한 shape/type 관계 | topology hint만으로 실행 순서를 바꾸지 않음; proof/guard 없는 후보는 적용하지 않음 |
| logical memory/resource 분석 | type/rank/shape, sparse/boxed schema, use-def/liveness | 기호식/범위/unknown으로 보고. logical extent 합계를 peak allocation으로 표시하지 않음 |
| kernel 선택·SIMD/GPU schedule | 합법성 정보 + 실제 representation·target/capability | guarded route 또는 검증된 지원 경로만 사용. unknown을 임의 layout/device로 가정하지 않음 |
| buffer 재사용·in-place | representation·ownership/alias/lifetime + graph liveness | shared/external owner가 보이면 conservative 처리; 허용만으로 reuse를 보장하지 않음 |

학습 step의 **개념 명세 예시**(현행 dtype/기호 차원/AD 지원을 주장하지 않음):

```text
X: data,      float32[B,784]
Y: label,     float32[B,10]
W: parameter,float32[784,10]   // runtime value changes between steps
constraints: X.dim0 = Y.dim0, 0 <= B <= 1024
state: W read -> compute -> explicit update; optimizer state is separate
constants: only the required axis/window/rank operands
witness: verify metadata and any specialized NAME/semantic policies before effects
physical inputs: adapter-provided placement/layout/lifetime, separate from this schema
```

조건은 실제 program/domain이 허용해야 하며 `B=0` 결과와 오류도 기존 J 의미를 따른다. 사용자가 이미 batch별 step을 정의한 경우 dataset 전체 크기/내용 없이 그 step을 분석할 수 있다. compiler가 전체 데이터셋 reduction·batch statistics를 임의의 minibatch 계산으로 바꾸는 것은 다른 문제이며 별도 equivalence proof가 필요하다. 계약이 같아도 weight의 값은 바뀔 수 있다; 캐시에는 **특정 최적화가 실제로 사용하는 facts만** 넣는다. 값 의존 출력(예: 필터의 결과 길이)은 shape만으로 정확히 알 수 없으며 unknown/후속 shape 계산 경계를 유지한다.

정보 획득 순서는 기존 semantic contract/known literals → 준비된 noun/header/API metadata → 필요한 optional declarations → 합법성을 위해 필요한 proof/runtime guard다. 내용 스캔·별칭 증명·GPU 구현을 기본 입력 명세의 선행 조건으로 요구하지 않는다. specialization guard miss는 곧 J semantic error가 아니며 가능한 재분석/검증된 runtime 경로를 사용한다. 명시 계약 위반의 오류 정책과 단순 optimization precondition miss를 구분해야 한다. WI1 metadata API의 오류는 명시 검사의 결과이며 자동 compiler dispatch 정책이 아니다.

프레임워크 근거: [JAX ShapeDtypeStruct/AOT](https://docs.jax.dev/en/latest/aot.html)·[기호 shape constraints](https://docs.jax.dev/en/latest/export/shape_poly.html), [TensorFlow TensorSpec/input_signature](https://www.tensorflow.org/guide/function), [TVM Relax symbolic shapes](https://tvm.apache.org/docs/deep_dive/relax/learning.html), [PyTorch input/state signatures](https://docs.pytorch.org/docs/main/user_guide/torch_compiler/export/api_reference.html)·[tensor guards](https://docs.pytorch.org/docs/main/user_guide/torch_compiler/torch.compiler_dynamo_overview.html). 이 표의 IN0–IN11 분류와 적용 우선순위는 RustJ 설계 판단이다.

문서 체크리스트:

- [x] **WI0a** 전달 문법과 독립적인 IN0–IN11 inventory, optimization별 최소 정보 및 framework sources를 정리한다.
- [ ] **WI3a** 필요한 facts의 provenance/refinement와 conflicting/unknown 입력을 검증한다. 동등/범위 기호 차원은 WI5와 함께 처리한다.
- [ ] **WI4a** guard miss와 explicit contract violation, supported route 선택을 구분하고 effect 전에 검증 가능한 전제를 연결한다. scalar/empty/boxed/sparse·changing batch/weights 및 NAME 변경을 포함한다.

이번 변경은 설계 문서 정리다. WI1 이후 새 runtime/optimizer 구현 또는 새 test gate 완료를 주장하지 않는다.

<a id="설계-영향과-출처--채택할-아이디어와-rustj-판단의-구분"></a>
##### 설계 영향과 출처 — 채택할 아이디어와 RustJ 판단의 구분

아래 표는 **설계에 영향을 준 개념의 출처**다. 각 프레임워크가 RustJ의 조건을 그대로 구현한다는 뜻이 아니며, 이 조건은 아직 구현되지 않은 외부 입력/실행 adapter의 명세다. J NAME snapshot/late binding, scalar/empty, sparse/boxed 및 observable error 의미는 RustJ/jsource 경계에서 유지한다.

| 영향을 준 프레임워크/연구 | 참고한 구체적인 방법 | 관련 inventory | RustJ에 적용하는 판단과 차이 |
|---|---|---|---|
| JAX | 값 없는 shape/dtype descriptor와 실제 값이 필요한 static argument | IN1–IN3, IN5 | 입력 배열과 compile-time 작은 상수를 분리. ndarray 원소를 만들어 graph 분석하지 않는다. [ShapeDtypeStruct](https://docs.jax.dev/en/latest/_autosummary/jax.ShapeDtypeStruct.html), [AOT/static args](https://docs.jax.dev/en/latest/aot.html) |
| JAX shape-polymorphic export | 같은 symbolic scope의 기호 차원·동등/범위 조건 | IN3, IN11 | shared B와 독립 unknown을 구별. 기호 scope/provenance를 추적하고 J의 empty dimension을 임의 배제하지 않는다. [기호 shape](https://docs.jax.dev/en/latest/export/shape_poly.html) |
| TensorFlow | TensorSpec/input_signature, 일부 차원의 wildcard, incompatible signature 검사 | IN1–IN3, IN11 | 고정·미확정 facts를 부분적으로 허용. 단순 wildcard와 입력 사이의 동등 관계는 구별하며 선언을 full J의 필수 조건으로 만들지 않는다. [tf.function](https://www.tensorflow.org/guide/function) |
| TVM Relax | 입력과 연산 사이의 shape/dtype·기호 관계 전파 | IN1–IN3, IN10 | logical 관계와 physical lowering을 분리하고 이후 symbol analysis의 근거로 사용. TVM의 array agreement를 J에 복사하지 않는다. [Relax](https://tvm.apache.org/docs/deep_dive/relax/learning.html) |
| PyTorch Dynamo/export | 관측 metadata에 대한 guards, input/parameter/state signature 구분 | IN0–IN3, IN6, IN10–IN11 | specialization에서 사용한 dtype/rank/size/layout 조건을 확인. 실제 J function NAME witness와 noun snapshot timing은 별도로 유지. [Dynamo guards](https://docs.pytorch.org/docs/main/user_guide/torch_compiler/torch.compiler_dynamo_overview.html), [export signature](https://docs.pytorch.org/docs/main/user_guide/torch_compiler/export/api_reference.html) |
| JAX buffer donation | 호출 후 필요 없는 입력 버퍼를 결과 저장소로 재사용하도록 허용; 호환 결과 필요 | IN9 | ownership transfer를 명시하고 이전 handle의 사용을 차단할 수 있을 때만 후보. donation 허용은 실제 reuse 보장이 아니다. [Donation](https://docs.jax.dev/en/latest/buffer_donation.html) |
| MLIR One-Shot Bufferize | use-def/alias·read-after-write 충돌 및 writable 여부에 따른 in-place/out-of-place 결정 | IN9–IN10 | live J snapshot/외부 alias를 덮어쓰지 않는다는 증거가 있어야 reuse. 불명확하면 별도 output/copy 또는 지원 route 경계. [Bufferization](https://mlir.llvm.org/docs/Bufferization/) |
| 역사 JAXA 연구 | source 계약과 entry graph를 gerund/with로 연결하는 표면 아이디어 | IN0–IN3, IN6 | optional 전달 후보만 계승; physical policy를 섞지 않고 등록된 capability와 ordinary binding을 사용. [JAXA 제안](https://github.com/yunskim/jaxa-analyzer/blob/7275d5ba7b7c39d5e3d304cb49e565b8e16ddf33/docs/JAXA_%EC%95%84%ED%82%A4%ED%85%8D%EC%B2%98_J%EC%97%B0%EC%82%B0_Python%EC%9E%90%EC%9B%90.md#L873) |

<a id="입력-metadata-조건-m0m8"></a>
##### 입력 metadata 조건 (M0–M8)

이 조건은 **metadata를 정당한 최적화 전제로 사용할 때** 적용한다. dtype/rank/shape만으로 buffer 안전성·함수 의미·내용 불변성까지 증명하지 않는다. 물리 접근 검사는 compiler semantic facts와 별도의 adapter 의무다.

| 조건 | 명세 | 실패/미확정 처리 및 현재 상태 |
|---|---|---|
| M0 입력 mapping | 사용되는 각 noun input의 ID/slot/이름을 명확히 연결; 중복·누락·다른 품사 입력을 구별 | WI1 API는 used noun의 missing/duplicate/extra를 검사. 함수/locale 재사용 증명은 별도 |
| M1 논리 일관성 | shape가 있으면 rank=shape.len; 각 extent는 유효한 비음수 크기; scalar는 rank 0·atom 1, empty는 0 extent·atom 0; count 계산은 overflow-safe | 현재 선언/Value 생성의 해당 검증을 사용. 표현 한계로 처리하며 negative/empty를 임의 shape로 보정하지 않음 |
| M2 dtype·encoding 연결 | 외부 dtype/byte order/encoding과 J logical dtype의 연결은 adapter가 확인; 변환은 explicit하고 numeric semantics를 보존 | fp32 bytes를 f64 slice로 재해석하지 않음. WI1은 Value의 logical dtype만 확인하며 외부 encoding/import 검사는 미구현 |
| M3 저장 범위 | 직접 접근 전에 dense byte capacity 또는 strided view의 offset·stride·접근 가능 범위를 overflow-safe하게 확인; alignment/device/backend 요구도 확인 | shape만으로 모든 strided 접근이 안전하다고 가정하지 않음. CPU affine foundation과 향후 external adapter 의무를 구분; WI1의 검사 범위 아님 |
| M4 unknown·symbolic | unknown에 임의 상수/양수/no-alias를 부여하지 않음. 같은 scope의 symbolic equality/bounds만 필요한 specialization 전제로 사용 | symbolic relation/guard는 WI5 후속. 미확정 조건은 conditional report 또는 지원되는 runtime shape 경계 |
| M5 근거·유효 기간 | 각 fact에 source/proof/관측 scope를 연결. file header·catalog 선언은 실제 버퍼의 존재/수명/최신성을 보장하지 않음 | 분석용 선언과 runtime witness 분리. actual Value/buffer generation 또는 동등한 소유권 lease에 조건을 연결; 통합 미구현 |
| M6 검사→사용 일관성 | 검사한 metadata·NAME·storage가 실제 실행 대상과 같아야 함. 외부 reshape/reallocation/mutation 경쟁은 snapshot·lease·synchronization 계약으로 통제 | 매 batch 새 입력의 relevant facts를 확인. 검사 후 변경 가능성이 남으면 해당 specialization/import를 사용하지 않음. WI1의 성공은 이후 호출까지의 보장이 아님 |
| M7 부분 facts·내용 조건 | 확보된 정보만 refinement. finite/sorted/range 같은 contents 조건은 별도 proof/검사; 표본 통계는 legality 근거가 아님 | 값 없는 schema 분석을 유지. boxed/sparse의 알려지지 않은 child/구조 facts는 unknown; 현재 WI1은 atoms/representation 조건 미검사 |
| M8 cache/guard 실패 | 최적화가 실제 사용하는 metadata·function/semantic 정책만 cache precondition으로 추적; batch/weight 값·pointer 변경만으로 불필요한 재컴파일하지 않음 | 명시 계약 오류와 specialization miss 구별. effect 후 자동 replay 금지; 재분석/지원 경로 선택은 실제 capability가 있을 때만. dispatcher 미구현 |

<a id="소유권과-입력-저장소-조건-o0o8"></a>
##### 소유권과 입력 저장소 조건 (O0–O8)

| 조건 | 명세 | 허용되는 동작과 한계 |
|---|---|---|
| O0 import 방식 | owned transfer, shared immutable 또는 borrowed 외부 버퍼를 명시적으로 구별. raw pointer/Arc refcount 하나만으로 외부 독점성을 추론하지 않음 | RustJ가 소유한 버퍼와 외부 aliases의 증거는 다름. unknown ownership은 read-only 후보에도 lifetime/동기화 확인 필요 |
| O1 수명 | 모든 read/write·transfer가 끝날 때까지 owner/lease 유지. 반환한 view가 입력을 참조하면 그 view의 수명까지 연장 | 이후 GPU/async 경로는 enqueue 시점이 아니라 실제 completion까지 유지해야 함; 현재 해당 실행 미구현 |
| O2 외부 mutation | J noun snapshot으로 노출한 데이터는 외부 변경으로 뒤에서 바뀌지 않아야 함 | immutable lease, synchronized snapshot copy 또는 explicit state/effect 경로 사용. wrapper가 read-only라는 이유만으로 producer의 쓰기가 차단되었다고 보지 않음 |
| O3 alias/overlap | 입력 간 views, 외부 alias, 살아 있는 global/boxed noun snapshots 및 graph 내부 소비자를 고려 | alias 부재가 증명되지 않으면 in-place/reuse를 허용하지 않거나 검증된 overlap-safe 구현을 사용. user의 no-alias 주장만으로 safe Rust reference를 만들지 않음 |
| O4 쓰기 권한 | 실제 backing이 writable이고 semantic operation의 write contract가 있어야 함 | readonly/mmap/protected backing을 덮어쓰지 않음; allocator 선택과 observable state write를 혼동하지 않음 |
| O5 donation/reuse | 명시 transfer 권한 + live old-value reads 없음 + alias/ownership 증거 + 호환 output representation + J 오류/효과 순서 보존이 필요 | transfer가 성립하면 이전 input handle/alias의 사용이 차단되어야 함. shared noun을 단지 refcount가 낮아 보인다는 이유로 consume하지 않음. output/copy가 여전히 필요할 수 있음 |
| O6 release 책임 | ownership 이동 또는 borrow의 시작/끝과 release 주체를 명시; transferred owner reference는 정확히 한 번 반환/해제 | validation 실패·부분 import·호출 실패·최종 output 종료에서 누수/double release 방지. metadata만 존재하는 선언은 release할 실제 buffer를 갖지 않음 |
| O7 zero-copy 전제 | lifetime·mutation·alias·alignment·encoding·layout/backend 접근 조건이 충족될 때만 직접 import | 조건 미충족 시 지원되는 값 보존 copy/conversion을 명시적으로 선택하거나 import 경계 보고. copy 중 외부 writer와의 동기화도 필요; 항상 zero-copy라고 약속하지 않음 |
| O8 실패·소비 시점 | 실제 호출/효과/ownership transfer 이전 가능한 검증을 완료; transfer commit 시점과 실패 뒤 handle 유효성을 adapter contract에 기록 | 시작 전 실패와 이미 소비/효과가 있는 실패를 구분. generic rollback·재실행을 약속하지 않으며 J transactional assignment 및 live snapshot을 보존 |

M/O 조건은 RustJ의 **채택할 계약과 후속 구현 요구**다. 현재 `StaticAnalysis::validate_noun_inputs()`는 borrowed `&Value`의 dtype/rank/shape와 입력 mapping만 검사한다. 외부 import·lease·encoding/stride capacity·alias·donation·자동 dispatch는 구현 완료가 아니다. 소유권 조건은 physical/runtime 경계에 남고 semantic `with`가 unsafe pointer 접근 권한을 부여하지 않는다.

- [x] **WI0b** 항목별 framework 영향·출처 및 M0–M8/O0–O8 조건을 정본/영어 mirror에 기록한다.
- [ ] **WI4b** adapter/guard 연결 시 stale metadata·외부 mutation·overlap·readonly backing·잘못된 capacity·donation 후 사용·실패 release·view lifetime 사례를 검증한다. async/device completion은 해당 backend를 구현할 때만 추가한다.

<a id="정적-분석-우선-실제-배열-없이-계약에서-분석한다"></a>
##### 정적 분석 우선: 실제 배열 없이 계약에서 분석한다

**사용자 요구:** 실행 전 최적화의 기반은 정적 분석이다. M/O 조건을 모두 actual buffer의 runtime 검사로만 얻는 구조로 만들지 않는다. 분석 입력은 실제 noun이 아니라 noun의 schema/facts와 parser가 보존한 계산 구조일 수 있다. 외부 API·header가 metadata를 제공하더라도 그 조회 자체의 I/O와 **이후 데이터 없는 정적 graph 분석**은 구별한다. source verb를 호출하거나 dummy/training array를 생성하여 shape를 알아내는 것을 기본 전제로 삼지 않는다. `with`는 이 분석에 필요한 정보를 전달할 수 있는 optional surface일 뿐이다.

```text
J frontend에서 보존한 graph + input schema + operation/ownership contracts
    -> static fact propagation / constraints / use-def / liveness
    -> 분석 보고와 최적화 후보의 legality 전제
    -> 정적으로 증명되지 않은 필요한 조건만 residual obligations로 보존
    -> 실행 때 actual inputs 연결 + 필요한 guard/lease 확인
    -> 지원되는 실행 경로
```

위 화살표는 설계 단계 구분이며 optimizer 실행·모든 조건 solver·dispatcher의 구현 완료가 아니다. kernel 실행 없이 만드는 정적 보고에는 graph 구조, known/unknown facts, 결과 extent, 자원식/상한, live values와 미해결 조건을 담는다. compiler는 facts를 unknown으로 남겨도 부분 분석할 수 있다. unknown과 unreachable, compile-route boundary와 invalid J를 혼동하지 않는다.

| 정보/조건 | 실행 전에 분석할 수 있는 부분 | 실제 입력이 연결될 때 남을 수 있는 조건 |
|---|---|---|
| IN0 / M0 | input slot/POS 및 graph edges, used-input inventory를 syntax/schema에서 결정 | 실제 공급 mapping, ordinary NAME의 current binding/POS/version witness |
| IN1–IN3 / M1,M4 | declared dtype/rank/shape, small constants와 연산 shape rules로 결과 facts/차원 관계 추론; known 제약은 정적으로 검증 | 공급 metadata가 declared facts 및 미증명 equality/bounds와 일치하는지 확인 |
| IN4 / M7 | 알려진 boxed child/sparse schema로 applicability/assembly 분석 | actual child schema·stored structure 등 선언에서 증명하지 못한 facts. 값 의존 shape는 unknown 유지 |
| IN5–IN8 | explicit configuration constants, semantic numeric/effect/state contracts로 legality와 dependency 분석 | 실제 함수 identity/정책, 별도 contents 조건 중 해당 route가 요구하는 것만 확인 |
| IN9 / O1–O5 | closed graph의 use-def/liveness로 old-value read와 내부 alias 충돌 분석; 명시 ownership 계약 및 managed Rust borrow/lifetime 정보 활용 | 외부 owner의 lease, 외부 alias/mutation·writability·exclusive transfer를 보장하는 adapter 증거. 선언만으로 외부 독점성 증명하지 않음 |
| IN10 / M2,M3,O7 | known target/representation 계약으로 후보·허용 encoding 및 조건을 도출 | actual backing capacity/strides/device/alignment와 lifetime의 적합성. physical 사실도 정적으로 주어졌으면 이미 증명된 부분은 반복 검사하지 않음 |
| IN11 / M5,M6,M8,O6,O8 | 각 fact의 provenance/scope와 경계별 proof obligations, cache preconditions, ownership transfer/release 흐름 정리 | 미해결 전제의 witness 유효성 및 검사→사용 안정성. 효과 후 실패는 자동 replay로 복구하지 않음 |

소유권의 정적 분석은 **내부 graph에서 더 이상 읽지 않는다는 사실**과 **외부에서 독점적으로 소유한다는 계약**을 구별한다. 전자는 graph 분석으로 증명할 수 있지만 후자는 외부 adapter의 보장 또는 managed lifetime/ownership 증거가 있어야 한다. runtime alias 체크를 언제나 값싼 일반 해법으로 가정하지 않는다. 보장이 없으면 out-of-place/지원 route 경계를 유지하며 correctness를 희생하지 않는다. 명시 contract에서 가능한 compile-time 증명과 residual guard가 함께 존재할 수 있다.

shape가 concrete하면 일부 logical extent를 정확히 계산하고, symbolic이면 식 또는 proven upper bound를 보고하며, 관계가 unknown이면 그 부분의 수치를 확정하지 않는다. symbolic 분석 자체도 정적 분석이다. concrete layout/schedule이 없으면 실제 peak allocation·성능 수치를 단정하지 않는다. 전체 학습 데이터셋의 원소 수나 내용이 graph 크기에 비례하여 compiler 안에 저장되어야 하는 것은 아니다.

**현재 경계:** `StaticAnalyzer::declare_noun()`/`analyze()`는 실제 arrays 없이 supported syntax와 concrete/unknown facts로 보고를 만든다. WI1 `validate_noun_inputs()`는 후속 입력 적합성 검사의 최소 seam이다. arbitrary per-dimension symbolic facts/constraint solver, ownership obligations의 전체 정적 증명, external leases, optimizer 변환 및 dispatch는 아직 미구현이다. 이러한 후속 작업은 frontend semantic 보존을 전제로 하며 actual data 실행을 정적 분석의 필수 조건으로 추가하지 않는다.

- [x] **WI0c** 정적 facts/proof와 residual runtime obligation을 분류하고 M/O의 정적 분석 역할을 명시한다.
- [ ] **WI3b** metadata-only 분석에서 shape/resource/use-def 결과와 미해결 조건을 보고한다. regression은 배열 미할당·kernel 미실행 및 unknown 보존, compile-time 제약 충돌/부분 facts refinement를 검증한다.
- [ ] **WI4c** 정적으로 증명한 조건과 residual guard/adapter 보장을 실행 경계에 연결하되 runtime-only 분석 구조로 바꾸지 않는다. 소유권/lifetime proof 범위와 미확정 external alias를 명시한다.

