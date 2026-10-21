# Conv1D blackbox와 사전 정의된 Algebraic View — 설계 초안

상태: 설계 제안 (미구현). 범위: Conv1D 하나. 이 문서의 IR/메타데이터 구문은 의사 코드이며 실제 RustJ API가 아니다.

## 1. 목표와 경계

Conv1D는 Graph IR에서 **하나의 opaque primitive**이다. 사용자는 필터 원소별 곱셈, 윈도우, 축약 노드를 보지 않는다. 단, primitive 등록 시 forward 및 두 부분 adjoint의 **대수적 의미식(Algebraic View)** 을 함께 제공한다. SPORES 스타일 최적화기는 의미식을 조회하여 Graph IR을 더 좋은 **고수준 그래프**로 재작성할 수 있다. Graph IR → Logical IR lowering도 **같은 의미 정의**를 사용한다. 이 과정에서 가상 전개는 실행이나 중간 배열 물질화를 뜻하지 않는다.

가중치 W는 예시에서는 Conv1D 호출의 *논리적 피연산자*이지만 원소별 내부 계산 노드로 공개되지 않는다. W가 구현 내부에 완전히 캡처된 경우, 최적화기가 W를 합치는 재작성을 수행하려면 별도의 합법적인 재구성 인터페이스가 필요하다. 의미식만으로 숨겨진 상태를 임의로 변경할 수 없다.

## 2. 완전한 예제: 입력, 출력, 수치

범위 제한: 실수형, 배치 1, 입력 채널 1, 출력 채널 1, kernel 2, stride=1, padding=0, dilation=1, groups=1, cross-correlation convention. 입력 `X=[1,2,3,4]` (shape [1,1,4]), 가중치 `W=[2,3]` (shape [1,1,2]); 출력 shape [1,1,3].

```text
Y[p] = sum_{k=0..1} X[p+k] * W[k]
Y[0] = 1*2 + 2*3 = 8
Y[1] = 2*2 + 3*3 = 13
Y[2] = 3*2 + 4*3 = 18
Output Y = [8,13,18]
```

Graph IR 입력은 X, W 및 conv spec이고 출력은 Y이다:
```text
%y = Conv1D(%x, %w, spec={stride:1,padding:0,dilation:1,groups:1})
```

## 3. Primitive 등록 계약 (개념 명세)

```text
primitive Conv1D {
  inputs: X: Tensor[N,Cin,L], W: Tensor[Cout,Cin,K]
  attributes: stride, padding, dilation, groups
  output: Y: Tensor[N,Cout,Lout]
  constraints: Cin % groups == 0; compatible weight channels;
               Lout = floor((L+2*padding-dilation*(K-1)-1)/stride)+1

  forward_view:
    Y[n,o,p] = sum(c,k, X[n,c,p*stride+k*dilation-padding] * W[o,c,k])
    // out-of-bounds input is zero; this formula assumes groups=1

  input_adjoint_view:
    dX[n,c,u] = sum(o,p,k where u=p*stride+k*dilation-padding,
                      dY[n,o,p] * W[o,c,k])

  weight_adjoint_view:
    dW[o,c,k] = sum(n,p,
                      X[n,c,p*stride+k*dilation-padding] * dY[n,o,p])

  reconstruction: match indexed sum-product with same spec and legal operands
  implementations: native_conv1d | generic_lowering
}
```

복소수는 켤레 인자, groups는 채널 그룹 제약을 포함한 별도 정의가 필요하다. Shape와 spec은 view의 정당성 조건이며 단순 주석이 아니다.

## 4. Graph IR의 SPORES 스타일 처리: 입력과 출력

**입력:** `Y = Conv1D(X,W1) + Conv1D(X,W2)`, 동일 X, spec, 출력 shape, 적법하게 결합 가능한 W1/W2.

예를 들어 `X=[1,2,3,4]`, `W1=[2,3]`, `W2=[4,1]`:
- 첫 conv: [8,13,18]
- 둘째 conv: [6,11,16]
- 원래 Y: [14,24,34]

최적화기는 두 blackbox의 **사전 등록된** forward_view를 분석용 표현으로 조회한다:
```text
sum_k X[p+k]*W1[k] + sum_k X[p+k]*W2[k]
  = sum_k X[p+k]*(W1[k]+W2[k])
```
Reconstruction contract가 이 식을 Conv1D로 재인식할 수 있으면 **출력 Graph IR 후보**:
```text
%w = Add(%w1, %w2)      // [6,4]
%y = Conv1D(%x, %w, spec)
                           // [14,24,34]
```
기존 Conv1D 노드를 무조건 분해하거나 교체하지 않는다. 후보의 채택은 비용 모델과 수치 의미론의 허용 조건에 따른다. SPORES의 수학적 재작성은 부동소수점에서 비트 단위 동일성을 보장하지 않는다.

**입력 가중치가 완전히 private인 경우:** Add(W1,W2)를 생성할 수 없으므로, 재구성 API가 지원하지 않으면 이 후보는 **불가**이다. Graph IR 최적화기는 opaque 상태를 외부로 누출하지 않는다.

## 5. Adjoint adverb: 입력과 출력

Adjoint는 독립된 Conv1DAdjInput/Conv1DAdjWeight primitive ID가 아니라 Conv1D에 대한 파생 연산이다. 어느 피연산자에 대한 부분 adjoint인지 명시한다.

```text
Input: dY=[1,2,3], W=[2,3], original X shape=[1,1,4]
Graph: dX = ApplyAdjoint(Conv1D, wrt=X, cotangent=dY, fixed=W, spec)
Output dX = [2, 7, 12, 9]

Input: X=[1,2,3,4], dY=[1,2,3], original W shape=[1,1,2]
Graph: dW = ApplyAdjoint(Conv1D, wrt=W, fixed=X, cotangent=dY, spec)
Output dW = [14,20]
```

검증: `dot(Conv1D(X,W),dY)=8*1+13*2+18*3=88`, `dot(X,dX)=2+14+36+36=88`, `dot(W,dW)=2*14+3*20=88`. 이는 **고정된 다른 인수에 대한 선형 사상의 adjoint**이지 일반 비선형 프로그램의 자동미분이 아니다.

## 6. 동일한 view의 Logical IR lowering 재사용

**입력:** 최적화된 Graph IR `%w=Add(W1,W2); %y=Conv1D(X,%w,spec)` 및 정적 shape/spec.
**출력:** 인덱스 관계, 곱셈, 축약이 명시된 Logical IR:
```text
%w = ElementwiseAdd(W1,W2)
%window = IndexMap(X, [n,c,p+k])  // 논리적 view, materialization 아님
%prod = Multiply(%window, %w[o,c,k])
%y = ReduceSum(%prod, axes=[c,k])
```
Graph IR SPORES의 조회와 Logical IR lowering은 **같은 정식 의미 정의**를 읽지만, 전자는 최적화 후보 탐색이고 후자는 선택한 실행 경로를 위한 표현 구체화다. Native 경로가 선택되면 전체 전개를 생략할 수 있다.

## 7. 인터페이스 I/O 요약

| 구성 | 입력 | 출력 |
|---|---|---|
| Primitive contract | X,W,spec | Y, shape/type facts |
| Algebraic View lookup | primitive ID, mode(forward/input-adjoint/weight-adjoint), shape/spec | 인덱스·축약·피연산자 역할을 담은 분석용 의미식 |
| Graph rewrite | Graph IR + views + legality/numerical constraints | 동등성 조건을 만족하는 고수준 Graph IR 후보 |
| Reconstruction | 변형된 의미식 + primitive registry | Conv1D 형태의 재구성 가능 여부/새 노드 |
| Logical lowering | 선택된 Graph IR + 동일한 view | IndexMap/Multiply/ReduceSum 등의 Logical IR |
| Physical selection | Logical IR 또는 합법적인 native Conv1D 호출 | 실행 커널/계획 |

## 8. 검증 체크리스트 (미구현)

- [ ] Conv1D 계약과 shape/spec 검증 (padding/stride/dilation/groups 포함)
- [ ] forward_view와 native Conv1D 결과 일치
- [ ] input-adjoint 및 weight-adjoint 내적 항등식 검사
- [ ] Graph IR blackbox 불변성 검사
- [ ] SPORES-style rewrite 및 reconstruction 합법성 검사
- [ ] private weight에서 불법 재작성 차단
- [ ] 동일 view로 Logical IR lowering 결과 검사
- [ ] native/generic 경로 수치 정책 검사 (IEEE 반올림·J tolerance와 구분)
- [ ] 복소수·groups·경계 인덱스 명세 확대

이 문서는 우선 Conv1D만 다룬다. 다른 primitive로의 일반화는 위 검증 이후 결정한다.

## 9. 초기 검증 결과 (2026-10-21)

**범위:** stride=1, padding=0, dilation=1, 단일 채널 실수 Conv1D의 수학적 의미 검증. Python/NumPy 독립 실행으로 기본 예제와 시드 고정 무작위 1,000개 사례에서 forward의 필터 선형성 및 두 부분 adjoint 내적 항등식이 통과했다. 별도의 표준 라이브러리 재현 테스트를 [tests/conv1d_algebraic_view_contract.py](../../tests/conv1d_algebraic_view_contract.py)에 추가했다. 이 저장소 테스트 파일은 이번 작업에서 **커밋만 했고 실행하지 않았다**.

- 기본 예제: forward `[8,13,18]`, input adjoint `[2,7,12,9]`, weight adjoint `[14,20]`; 내적은 모두 `88`.
- 무작위 1,000개: `dot(conv(X,W),dY) ≈ dot(X,adj_input(dY,W)) ≈ dot(W,adj_weight(X,dY))`, `conv(X,W1)+conv(X,W2) ≈ conv(X,W1+W2)`.
- **미검증:** RustJ Graph IR의 opaque 노드 유지, 실제 SPORES/e-graph 조회·재작성·reconstruction, 동일 view를 사용하는 Logical IR lowering, private weight 캡슐화, native kernel 비교, 일반 stride/padding/dilation/groups 및 복소수.
- **판정:** 수학적 contract의 제한된 부분은 검증됨. IR 통합 설계 전체가 검증된 것은 아님. IEEE 754에서 재작성은 일반적으로 비트 동일하지 않으며, J 비교 허용 오차만으로 모든 재작성의 안전성을 증명할 수 없다.

## 10. Rust 최소 통합 프로토타입 (2026-10-21)

- 구현: [src/conv1d_algebraic_view.rs](../../src/conv1d_algebraic_view.rs); `src/lib.rs`에 모듈 등록.
- `view(ViewMode::Forward, spec)`를 Graph IR 유사 표현의 `rewrite_sum_of_convs`와 Logical IR 유사 표현의 `lower`가 **공유**한다.
- 단일 규칙 `Conv1D(X,W1)+Conv1D(X,W2) -> Conv1D(X,W1+W2)` 및 기본 예제 결과 `[14,24,34]`에 대한 Rust unit test 2개 작성.
- **중요 제한:** 독립된 프로토타입 타입이다. 기존 `j_graph_ir`, `logical_ir`, SPORES/egg와 연결되지 않았다. `Logical::IndexMapWindow/Multiply/ReduceSumKernel`는 lowering 형태를 나타내는 표식이며 실제 Logical IR 실행 의미를 갖추지 않았다. 내부 가중치를 외부에 공개하지 않는 정식 캡슐화도 미구현이다.
- **검증 상태:** 코드와 테스트는 커밋했지만 Rust 빌드·단위 테스트는 이 작업에서 실행하지 못했다. 컴파일 통과나 IR 통합 완료로 간주하지 않는다.
- **다음 순서:** (1) `cargo test conv1d_algebraic_view` 실행 및 오류 수정 (2) 실제 Graph IR 타입에 opaque Conv1D 노드 연결 (3) 실제 Logical IR lowering 연결 (4) algebraic view 기반 reconstruction/legality (5) adjoint view 연결 (6) FP 정책 검증.

## 11. 후속 검증 단계 (2026-10-21)

`src/conv1d_algebraic_view.rs`에 `eval_adjoint`를 추가해 동일한 `view(mode,spec)` 등록 계약을 Forward, InputAdjoint, WeightAdjoint 모두가 사용하도록 했다. 내적 항등식 및 잘못된 shape/모드 거부를 검사하는 세 번째 Rust 단위 테스트를 추가했다.

**실행 환경 제약:** 현재 실행 컨테이너에는 `cargo`와 `rustc`가 없고 GitHub raw 파일 네트워크 접근도 불가능하여 Rust 테스트를 실행하지 못했다. 따라서 Rust 컴파일 성공, 테스트 통과, 실제 RustJ IR 연결을 주장하지 않는다. 로컬/Work 환경에서 `cargo test conv1d_algebraic_view`를 실행해야 한다.

**현재의 중요한 기술적 한계:** `AlgebraicView`는 mode/spec을 보유하지만 인덱스·축약을 표현하는 공통 symbolic AST는 아니다. Graph rewrite는 하드코딩된 패턴, Logical lowering은 표식 벡터에 의존한다. 따라서 지금 단계는 *공통 레지스트리 조회의 배선(prototype)*만 보여 주며, **동일한 대수 의미식에서 최적화와 lowering을 파생한다는 핵심 설계는 아직 검증되지 않았다**. 다음 작업은 인덱스/축약 AST와 재구성 계약을 도입한 뒤 두 경로가 실제로 해당 AST를 소비하도록 변경하는 것이다.

## 12. 공통 symbolic AST 도입 (2026-10-21)

`src/conv1d_algebraic_view.rs`의 `AlgebraicView`를 단순 mode/spec 저장 구조에서 `Expr` AST로 변경했다. AST는 `Read(Operand, Index)`, `Mul`, `Sum`, `ScatterSum`을 가지며, `Axis`와 `OutputPlusKernel` 인덱스 맵을 명시한다. `view(Forward)`는 `Sum_K(Input[p+k] * Weight[k])` 형태를 실제 AST로 등록한다. `view(InputAdjoint)`는 scatter-sum, `view(WeightAdjoint)`는 output 축 합산을 등록한다.

Graph 재작성과 Logical lowering 모두 동일한 `forward_contraction(&view(...).expr)` 패턴 인식기를 사용하도록 변경했다. 따라서 이제는 **공통 symbolic 표현을 읽는 최소 프로토타입**이다. 단, 재작성 자체는 여전히 수동으로 작성한 분배 규칙 하나이며, Logical IR은 실제 RustJ IR이 아닌 프로토타입 표식이다. Adjoint oracle은 view의 AST 종류를 확인하지만 AST를 일반적으로 평가하지는 않는다.

**실행 상태:** 저장소에 코드 변경을 커밋했으나 현재 실행 환경에 cargo/rustc가 없어 Rust 컴파일 및 테스트는 미실행이다. `cargo test conv1d_algebraic_view`가 필수다. 기존 실제 Graph IR/Logical IR/SPORES와의 통합도 아직 미완료다.

**후속 검증:** symbolic AST의 alpha-equivalence, 축 범위 및 shape 제약, reconstruction의 피연산자 캡슐화, floating-point 재작성 합법성, 실제 IR 변환 및 실행 검증.

## 13. 실제 IR 경계 조사와 비침습적 어댑터 (2026-10-21)

실제 RustJ 소스 확인:
- `j_graph_ir::Plan`은 `nodes: Vec<Node>`, `result: Option<ValueId>`를 가지며 `NodeKind::Apply`는 `function`, `contract`, `left/right` 및 provenance를 포함한다. 현재 Conv1D 전용 `NodeKind`는 확인되지 않았으므로 기존 Apply 표현을 임의로 변형하지 않는다.
- `logical_ir::Plan`은 `operations: Vec<Operation>`와 `values: Vec<ValueData>`를 보유하고, `OpKind::Basis`/`SemanticCall`은 실행 의미 계약을 포함한다. 여기에도 프로토타입의 `IndexMapWindow`와 일대일 대응하는 정식 OpKind가 아직 없다.

`GraphViewRequest`는 실제 `j_graph_ir::ValueId`로 origin/input/weight를 받으며, `prepare_logical_view`는 **같은 symbolic Expr**와 provenance를 유지한 `LogicalViewPlan`을 반환하도록 추가했다. 이는 실제 RustJ 타입과 접하는 *준비 단계*이지, 정식 Graph IR → A3 Logical IR lowering이 아니다.

다음 단계는 Conv1D를 어떤 `FunctionEntity`/extended primitive ID로 인식할지 계약을 확정하고, shape/type/error/effect 순서를 보존하는 A3 표현을 설계하는 것이다. 검증은 사용자의 요청에 따라 모아서 실행하며, 이번 변경에서는 테스트를 실행하지 않았다.
