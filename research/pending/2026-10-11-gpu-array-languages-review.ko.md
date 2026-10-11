# 임시 보관 — GPU Array Languages 프로젝트 재검토와 RustJ 설계 보강 후보

> 상태: **임시 검토 메모 / PROJECT.ko.md에 미반영**
>
> 검토일: 2026-10-11 (KST)
>
> 원문: https://codereport.github.io/GPUArrayLanguages/
>
> RustJ 기준: GitHub `main`의 설계 및 소스 정적 조사(2026-10-11). 확인 당시 tree SHA: `0b93c62dc58522c5fa38ab530c510f56d9db207e`.
>
> **보관 목적:** `PROJECT.ko.md`를 별도로 편집 중이므로 충돌·중복 없이 나중에 최신 정본에 선별 통합한다. 이 문서는 확정된 설계 변경이나 구현 완료를 뜻하지 않는다.

## 1. 결론

- 현재 RustJ의 **J Semantic / J Graph IR → Logical Execution IR → Schedule/Transform → Physical Plan**이라는 책임 경계는 유지한다.
- 새 canonical IR 또는 특정 언어의 타입 시스템·GPU runtime을 필수 단계로 추가할 근거는 아직 없다.
- 기존 설계에는 Rank/frame/cell/zero-frame prototype·조립, 의미상 오류/부작용 우선순위, high-level array basis, symbolic shape 및 iteration/access facts, heterogeneous placement, GPU fusion/flattening 후보가 이미 있다.
- 보강 후보는 **(A) 분석 pass 의존성/무효화, (B) index domain의 대수적 facts, (C) AD accumulation/VJP 계약, (D) 알고리즘군별 정합성·성능 벤치마크**다.
- *설계 누락*과 *이미 계획했지만 구현하지 않은 부분*을 구별한다. M2→M3→M4 진행을 이 메모 때문에 멈추지 않는다.

## 2. 비교 프로젝트별 판단

| 참조 | 참고할 특징 | RustJ 판단 |
|---|---|---|
| Futhark | fusion, high-level SOAC, nested-parallel flattening, uniqueness/memory reuse | 기본 설계 반영. 합법성 증명과 실제 선택·실행은 미완성 |
| SaC / Single Assignment C | with-loop 최적화, array shape specialization, temporary elimination | 새로운 IR 불필요. Logical optimizer 및 representation/buffer planner에 흡수 |
| Dex (배열 언어; J의 `Dex` conjunction과 무관) | index sets, index-aware parallel loops, associative accumulation effect, AD | index facts와 AD 누적 계약 보강 후보 |
| Co-dfns | columnar / inverted-table AST, data-parallel passes, kernel-count/memory-cost 관점 | GraphIndex/AnalysisIndex 및 nanopass 계획에 반영 |
| Accelerate | scalar/array 구획, producer/consumer fusion, segmented reduction | 기존 region 분할, high-level basis, SegmentView로 표현 가능 |
| Copperhead | nested data parallelism과 GPU lowering | nested-structure preservation 및 최적화 경로의 비교 대상 |
| TAIL / apltail | typed/rank-aware APL IR → Futhark | Graph→Logical 분석 원칙과 부합; static 제한을 full J에 강제하지 않음 |
| Remora | rank polymorphism과 frame/cell | J rank와 empty/prototype 조립의 차이를 유지하며 비교 |
| APEX | morphology, type/rank/shape/value facts, SSA 및 specialization | MorphologyEngine/worklist/SpecializationKey의 구현 백로그에 반영 |
| NESL / Nessie | irregular nested parallelism, flatten/segment | NestedTraverse/SegmentView 및 irregular workload 평가에 활용 |
| DaCe *(보충 참조; 원문 목록 소속 여부와 별도로 비교)* | data-centric graph, access/movement analysis, analysis-pass dependencies | Physical Planner 및 pass invalidation 설계의 참고. 새 필수 SDFG 계층은 만들지 않음 |

추가 비교 대상(JAX/XLA/MLIR/TVM/IREE/PyTorch/TensorFlow/Nx 등)은 RustJ 문서에 이미 기재된 backend adapter·execution effects·schedule·memory planning 용도로 유지하며, 이 메모만으로 신규 의존성을 추가하지 않는다.

## 3. 설계 보강 후보 — 수용 전 검토

### GA-01 [ ] Pass dependency / analysis invalidation (권장 P1, M3)

**문제:** graph rewrite 뒤 `ShapeFacts`, `AccessRelation`, `Work/Depth`, `FusionCandidate`, proof bundle을 언제 재계산해야 하는지 전 IR에 걸친 공통 규칙이 부족하다. 현재도 source graph version, witness, NAME/cache invalidation이 존재하므로 중복 프레임워크를 만들면 안 된다.

**최소 제안:**
- pass contract: `requires`, `produces`, `may_change`, `preserves` / `invalidates`; region 또는 graph version과 analysis schema version.
- 모든 사실은 소유 node/region과 해당 version을 기록하고 변경 후 오래된 witness/candidate 사용을 verifier가 차단.
- 재계산과 incremental update의 비용이 타당한지 측정. 초기는 단순 invalidation + recompute면 충분.
- `FactWitness`, `ProofBundle`, source provenance의 기존 타입을 우선 재사용; NameRef/cache 유효성 규칙과 경계를 구분.

**검증:** stale shape, stale access map, rewrite 뒤 재사용된 fusion legality, changing binding guard, branched region에서 일부 invalidation 누락에 대한 negative tests.

### GA-02 [ ] Index domain algebraic facts (권장 P1, M3)

**문제:** 기존 `IndexSpace` / `IterationDomain` / `AccessRelation`에 직접 대응하는 더 정교한 facts 및 검증 흐름을 설계할 여지가 있다. Dex의 typed index set을 그대로 받아들이는 것이 아니다.

**최소 제안:**
- target-independent index-domain facts: extent/finite, stable iteration order, uniqueness/injectivity, coverage/surjectivity, permutation/bijection, mapping invertibility, static/dynamic domain.
- 각 사실은 `Known/Unknown` 및 근거/guard를 보유. `J` index bounds, rank, fill, tolerance/equality, domain 및 duplicate-index semantics와 연결.
- Gather/Scatter/Permute/GroupBy/SegmentView의 rewrite/parallelization legality에서 사용.
- physical layout/stride/coalescing/predication/atomics는 여전히 후속 physical/schedule 단계의 사실.

**검증:** 중복 index, 비단사 scatter, 부분 coverage, zero-length index, 역매핑 불가능, 동적 index 변경, J error precedence.

### GA-03 [ ] AD / VJP accumulation contract (권장 P3, NN 후속)

**현황:** `PROJECT.ko.md`에 adjoint/VJP expansion, `ParallelFanOut`, `ScatterCombine`, NN embedding gradient 경로 및 explicit `StateResource`가 이미 계획되어 있다. 하지만 AD transform의 입력별 미분 가능성, gradient 합성 및 순서/결정성 계약은 아직 통합·구현되지 않았다.

**최소 제안:**
- AD를 일반 frontend 언어 의미나 모든 primitive의 전제조건으로 만들지 않는다. optional verified transform / extension-specific decomposition으로 시작.
- differentiability eligibility 및 stop-gradient/non-differentiable input을 명시.
- tangent/adjoint dtype·shape·rank·empty handling과 gradient seed/zero를 입력별로 정의.
- 중복 대상 누적을 `ScatterCombine` 또는 별도 associative accumulation 후보로 연결하되, 원자적 scatter나 무조건 병렬화 허가로 오인하지 않음.
- floating reassociation/overflow, deterministic result, RNG state, alias, effect/error order의 적법성 검사를 통과해야 함.
- checkpoint/tape/materialization/donation은 나중의 memory/physical 계획으로 분리.

**검증:** embedding duplicate indices, scalar/broadcast-like-looking J rank의 차이, zero-frame, repeated accumulation, non-pure function rejection, numerical error priority.

### GA-04 [ ] Workload-based correctness/performance matrix (권장 P2, M5~M6)

**문제:** primitive별 conformance와 microbenchmark만으로 nested/irregular parallelism, transfer, fusion, critical-path tradeoff를 충분히 포착하기 어렵다.

**제안 workload:**
1. **Regular:** chained map→reduce, stencil, matmul/conv.
2. **Nested:** variable segment lengths, nested map/reduce, segmented scan.
3. **Irregular:** sort/group-by, scatter collisions, Quickhull-like worklist.
4. **Data movement & fused NN:** FlashAttention-like attention decomposition, index-heavy embedding, host↔device transfer.
5. **다중 단계:** N-body 및 MultiGrid 등에서 gather/update/iteration과 memory movement를 분리 계측.

**지표:** J semantic/conformance first; compile cold/warm, execution latency/throughput, work/depth, kernel count, launch/sync, bytes transferred, peak memory, fusion/materialization, CPU baseline 대비 성능. unsupported GPU를 성능 통과로 기록하지 않는다.

**주의:** 이 workload 이름이 J에서 곧바로 실행 가능하거나 RustJ에서 구현되었다는 뜻이 아니다. M5 비용 선택 및 이후 GPU runtime 수용에 맞춰 작은 테스트부터 단계적으로 추가한다.

## 4. 이미 설계되어 있고 구현이 남은 항목 (중복 신규 과제로 만들지 말 것)

- Graph Basis 및 `FusionCandidate`, `Work/Depth`: 분석 위주; 실행 optimizer/cost selection 미완료.
- `MorphologyEngine`, Symbolic Shape fixpoint / specialization cache: 기존 A3 체크리스트 대상.
- `SegmentView`, `NestedTraverse`, `ScatterCombine`, `Scan`: vocabulary와 의미 계약 일부 있음; 일반 CPU/GPU 실행 미완료.
- HE-01~09 heterogeneous plan: 계획과 최소 CPU `physical_plan` 구현을 구별.
- 실제 GPU/CUDA backend는 미구현·보류. 검증 환경과 별도 재개 승인 전 완료 표시 금지.

## 5. PROJECT.ko.md에 **나중에** 통합하는 절차 (현 단계 미실행)

- [ ] 편집 중인 `PROJECT.ko.md` 변경이 완료된 후 **새로운 GitHub main HEAD**를 재조회한다.
- [ ] 현행 절(예: §4.15 iteration/access, §4.24 array facts/basis, §10 A3/M3/HE/M5/NN, §11 validation)과 중복·충돌을 대조한다.
- [ ] GA-01~GA-04 중 정본에 없는 최소 계약만 해당 기존 체크리스트에 통합한다. 새로운 canonical IR 및 중복 roadmap/체크리스트를 만들지 않는다.
- [ ] 근거·reference 및 구현/설계 상태를 명시한다. 문서만 업데이트할 경우 모든 구현 테스트를 완료 처리하지 않는다.
- [ ] `PROJECT.md` 영어 mirror를 **같은 변경**에서 갱신한다. 변경이 architecture rationale에 영향을 줄 경우 `FOUNDATIONS.ko.md` / `FOUNDATIONS.md`, `AGENTS.md`에서 stale wording을 점검한다.
- [ ] 추가 반영된 테스트와 실행 근거가 있을 때만 완료 checkbox를 바꾼다.
- [ ] 통합 완료 후 **이 임시 파일은 삭제**하고 중복 문서가 남지 않도록 한다.

## 6. 출처·검토 기준

- 목록의 출발점: https://codereport.github.io/GPUArrayLanguages/
- RustJ canonical: https://github.com/yunskim/RustJ/blob/main/PROJECT.ko.md
- RustJ guardrails: https://github.com/yunskim/RustJ/blob/main/FOUNDATIONS.ko.md
- RustJ stage/architecture summary: https://github.com/yunskim/RustJ/blob/main/README.md
- Futhark: https://futhark-lang.org/
- TAIL / APL→Futhark 연구: https://elsman.com/pdf/fhpc16futhark.pdf
- APEX: https://gitlab.com/bernecky/apex
- Co-dfns: https://github.com/Co-dfns/Co-dfns/
- Dex: https://github.com/google-research/dex-lang
- Accelerate: https://github.com/AccelerateHS/accelerate
- DaCe (추가 대조): https://github.com/spcl/dace
- MLIR Bufferization: https://mlir.llvm.org/docs/Bufferization/
- IREE Stream: https://iree.dev/reference/mlir-dialects/Stream/

이 메모는 기존 정적 코드·설계 조사 내용의 보관본이다. **새로운 Rust 테스트, J reference differential, GPU 실측은 실행하지 않았다.** 외부 프로젝트 동향 및 링크는 최종 정본 반영 시 다시 확인한다.
