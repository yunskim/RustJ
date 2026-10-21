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
