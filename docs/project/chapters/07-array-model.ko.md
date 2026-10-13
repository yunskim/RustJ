[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)


<a id="logical-physical-array-model"></a>

<a id="6-논리-배열과-물리-배열--핵심-architecture-decision"></a>
## 6. 논리 배열과 물리 배열 — 핵심 architecture decision

<a id="61-논리-j-noun-boxed-sparse와-verb"></a>
### 6.1 논리 J noun, boxed, sparse와 verb

dense noun의 extensional value는 기본적으로 다음으로 본다.

```text
DenseJArray
  atom type
  shape
  ordered atoms / logical value
```

그러나 current jsource와의 semantic compatibility를 위해 “모든 noun = type + shape + flat atoms뿐”이라고 고정하지 않는다.

**boxed**는 physical encoding이 아니라 J의 semantic atom/type 구조다.

```text
BoxedJArray
  shape
  ordered boxed atoms
    each atom -> J value
```

실제 backend가 box를 pointer, handle, arena index 등으로 표현하는 것은 physical 문제다.

**sparse**도 단순한 backend compression format이 아니다. J의 `$.`와 sparse type/operations가 sparse representation을 관찰하며, sparse axes와 sparse element(fill)가 의미에 참여한다.

```text
SparseJArray
  logical atom type
  shape
  sparse_axes
  sparse_element
  sparse index/value semantics
```

dense와 sparse가 같은 extensional mathematical array를 나타낼 수 있어도 J 프로그램이 sparse representation을 관찰할 수 있으므로 semantic representation class를 보존한다.

따라서 noun semantic model은 개념적으로:

```text
JNoun
  value/type/shape semantics
  semantic representation:
    Dense
    Boxed
    Sparse(SparseSemantics)
    other J-visible noun kinds as implemented
```

이다.

verb는 noun array를 입력받아 noun array를 반환하는 array transformer이며, J Semantic Array IR에서 first-class semantic entity로 표현한다. verb의 hook/fork/train/modifier composition은 semantic analysis 전에 보존한다.

다음은 논리 J noun의 semantic identity가 아니다.

- stride
- offset
- physical tile layout
- CPU/GPU device
- byte alignment
- sharding
- CUDA block/thread
- concrete sparse backend format(CSR/COO 등)

<a id="62-valueid와-bufferid"></a>
### 6.2 ValueId와 BufferId

`ValueId`와 `BufferId`는 다르다.

- `ValueId`: 계산의 논리 결과
- `BufferId`: 특정 물리 저장 공간

하나의 ValueId가 CPU와 GPU의 여러 representation을 가질 수 있다.

반대로 수명·alias·effect 조건이 검증된 여러 ValueId가 같은 BufferId를 재사용할 수도 있다. Logical ArrayValue가 존재한다고 별도 buffer가 필요한 것은 아니다. view, fused-away intermediate, rematerialized value의 실제 저장과 reshape/transpose/reverse/slice의 copy 여부는 downstream planner가 결정한다.

<a id="63-physicalarray"></a>
### 6.3 PhysicalArray

개념 모델:

```rust
PhysicalArray {
    storage / buffer,
    shape,
    strides,
    offset,
    encoding,
    placement,
    layout,
}
```

현재 `src/physical.rs`에는 다음 G1 기초가 구현되어 있다.

- scoped/generation `BufferId`
- `BufferLease`
- CPU backing registry
- dtype/encoding 확인
- signed strides / offset
- checked address span
- empty/scalar 처리
- read-only affine `PhysicalArray`
- standard-layout 판정
- logical slice의 제한적 노출

<a id="64-cpustorage의-위치"></a>
### 6.4 CpuStorage의 위치

현재 `CpuStorage<T>`의 Inline / Owned / Shared 구조는 버리지 않는다.

장기적으로는 J semantic Value의 유일한 저장 방식이 아니라 **CPU backend physical storage**로 재배치한다.

```text
ValueId
  ↓
Physical representation
  ↓
Storage
  ├─ CpuStorage
  ├─ CudaStorage
  ├─ MetalStorage
  └─ ...
```

<a id="641-현재-구현-상태와-완료-조건"></a>
#### 6.4.1 현재 구현 상태와 완료 조건

Logical/Physical Array 분리는 **아키텍처 결정으로는 확정**됐지만 runtime representation까지 완전히 이행된 상태는 아니다.

현재 상태:

- [x] Logical execution `ValueId`와 physical `BufferId`를 별도 identity로 둔다.
- [x] `PhysicalArray`가 buffer/stride/offset을 소유하고 logical IR에는 이 정보를 넣지 않는다.
- [x] 같은 logical atom order를 서로 다른 physical stride/offset/backing으로 표현할 수 있다는 회귀 테스트를 둔다.
- [x] GraphFacts는 physical stride/layout/device를 소유하지 않는다.
- [ ] runtime `Value`의 dense payload가 아직 `CpuStorage`를 직접 포함한다. 이는 전환기 구현이며 최종 Logical Array abstraction으로 간주하지 않는다.
- [ ] dense logical value와 CPU/GPU backend storage 사이의 explicit representation adapter/handle 경계를 완성한다.
- [x] `facts::LayoutFact`를 `RepresentationClassFact`로 이름 변경하고 `Facts.layout`도 `Facts.representation_class`로 바꿨다. `Dense / AxisSparse`는 J-visible representation class이며 physical layout이 아님을 API 이름에서 명시한다.

완료 기준은 **logical value를 정의하거나 분석하는 데 `CpuStorage`, stride, offset, device, BufferId가 필요하지 않고**, 선택된 backend representation을 통해서만 그런 정보가 등장하는 상태다.

따라서 현재 `Value { shape, Data::Int(CpuStorage<_>), ... }` 구조는 semantic boundary의 최종형이 아니라 migration bridge다.

---

