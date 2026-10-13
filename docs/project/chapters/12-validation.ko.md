[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)


<a id="validation-policy"></a>

<a id="read-validation"></a>
<a id="11-검증-정책"></a>
## 11. 검증 정책

모든 구현 변경은 이 절을 따른다.

<a id="111-기본-완료-관문"></a>
### 11.1 기본 완료 관문

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test --features portable
```

관련 Python harness가 바뀌면 해당 테스트도 실행한다.

<a id="112-semantic-bug-fix"></a>
### 11.2 semantic bug fix

- 재현 사례 추가
- regression test 추가
- 필요하면 conformance case 추가
- C reference와 비교 가능한 경우 비교
- known deviation과 pass를 분리

<a id="113-storage--memory-변경"></a>
### 11.3 storage / memory 변경

확인 항목:

- alias preservation
- transactional assignment
- allocation/reuse
- retained memory
- empty/scalar
- stale BufferId
- overlap
- source scope 이후 owner 수명

<a id="114-simd-변경"></a>
### 11.4 SIMD 변경

- tail
- overflow/promotion
- exceptional float
- portable fallback
- runtime feature detection

<a id="115-실행했다고-주장할-수-있는-것만-기록"></a>
### 11.5 실행했다고 주장할 수 있는 것만 기록

실제로 실행하지 않았다면 실행한 것으로 기록하지 않는다.

현재 정책상 다음은 자동으로 완료됐다고 간주하지 않는다.

- upstream 전체 J suite
- Miri
- sanitizer
- GPU test
- CUDA benchmark

Linux CI/GitHub Actions는 **별도 요청이 있을 때만 확인**하며, 기본 구조 진행/체크리스트의 완료 gate로 사용하지 않는다.

<a id="116-c-reference"></a>
### 11.6 C reference

참조 `jsource` revision은 검증 재현성을 위해 고정한다.

C reference는 별도 프로세스/벤치마크 경로에서 oracle로 사용하며 RustJ runtime dependency로 사용하지 않는다.

<a id="117-성능-해석"></a>
### 11.7 성능 해석

작은 배열에서 Python/FFI overhead가 섞인 숫자를 언어 성능으로 해석하지 않는다.

성능 비교에는 다음을 분리한다.

- parse/compile
- execution
- allocation
- copy bytes
- retained bytes
- peak live bytes
- cold/warm
- cache hit/miss
- transfer
- synchronization

`Rust가 J보다 빠르다` 같은 전체적 결론은 제한된 microbenchmark만으로 주장하지 않는다.

<a id="118-semantic-hard-cases의-golden-관문"></a>
### 11.8 Semantic hard cases의 golden 관문

prefix agreement, zero-cell fill/prototype와 heterogeneous result assembly, name expected-POS mismatch, assignment entity+effect/right-to-left lookup, hook/fork observable order, adverse/obverse latent semantics, tolerance/`!.`, overflow retry/promotion 및 error precedence를 대응 semantic golden으로 잠근다. full-J 전체를 첫 CPU slice 전에 완성할 필요는 없으나, 해당 의미를 optimization/lowering 대상으로 열기 전에 값·dtype·shape·오류·효과 순서의 differential 검증이 있어야 한다. 첫 matrix cell 표본은 [§4.11.4.12](./15-advanced-contracts-p04.ko.md#mean-proof-example)를 사용한다.

<a id="119-ir-형식-검증의-미래-도입"></a>
### 11.9 IR 형식 검증의 미래 도입

IR의 구조적·의미론적 정확성, 최적화 및 lowering의 관찰 가능한 의미 보존에 대한 형식 검증은 장기 필요 과제다. 별도의 필수 도구나 CI gate를 현재 도입하지 않는다. 유일한 실행 체크리스트는 [§10 FV-01~FV-12](./11-roadmap-checklists-p05.ko.md#formal-ir-verification-checklist)이며, 현재 모든 실제 항목이 미완료다.

---
