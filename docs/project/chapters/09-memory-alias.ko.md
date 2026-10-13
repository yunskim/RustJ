[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)

<a id="8-메모리alias수명-원칙"></a>
## 8. 메모리·alias·수명 원칙

<a id="81-읽기쓰기-분리"></a>
### 8.1 읽기/쓰기 분리

공유, zero-stride, 내부 overlap 가능 view는 기본적으로 read-only다.

첫 mutable 경로는 다음을 만족하는 출력으로 제한한다.

- 독점 소유
- 표준 연속
- non-overlap proof
- dtype/용량 호환
- live alias 없음

<a id="82-주소-검증"></a>
### 8.2 주소 검증

비어 있지 않은 affine view는 각 축의 signed delta를 checked 계산하고 접근 가능한 최저/최고 backing index를 확인한다.

span 검증은 접근 가능성을 증명하지만 non-overlap을 증명하지 않는다.

empty view는 원소 주소를 계산하지 않는다.

<a id="83-buffer-reuse"></a>
### 8.3 buffer reuse

buffer reuse는 명시적으로 증명한다.

확인할 것:

- 원래 allocation
- 전체/부분 영역
- offset
- 공유 owner
- live view
- alias
- dtype
- capacity
- alignment
- last physical use

GPU 재개 후에는 kernel 제출과 실제 device completion을 구분한다.

<a id="84-관찰-가능한-j-의미"></a>
### 8.4 관찰 가능한 J 의미

최적화 때문에 다음을 바꾸지 않는다.

- primitive/type별 overflow retry와 coherent result promotion semantics
- J가 정의한 error precedence, suppression, retry behavior
- active `try./catch./catcht.`에 의한 J-visible error/throw control transfer
- binding/name-reference의 late lookup semantics
- side effect 순서
- comparison tolerance와 `!.` fit semantics
- empty/rank fill-cell 결과 type·shape semantics
- rank/modifier result-cell assembly(type/shape join, framing fill, assembly error) semantics
- sparse/boxed의 J-visible representation semantics
- primitive/derived-verb가 요구하는 floating numeric contract(허용된 reassociation, compensated/exact mode, tolerance 등)

FMA, reassociation, tree/vector reduction은 **무조건 금지하지도, 무조건 허용하지도 않는다.** `NumericSemantics`/`FitSemantics`가 허용한 경우에만 적용한다. GPU 병렬 오류 수집도 arbitrary first-lane error를 그대로 노출하지 않고 J의 observable error contract를 따른다.

---


<a id="out-of-core-io-contract"></a>

<a id="85-느린-ioout-of-core-배열-실행-계약-2026-10-06-설계-후보--구현-미착수"></a>
### 8.5 느린 I/O·Out-of-core 배열 실행 계약 (2026-10-06, 설계 후보 / 구현 미착수)

**목표.** RAM/GPU 용량보다 큰 J noun·신경망 가중치·배열 데이터에 대해 (a) 원래 J 결과·오류·효과 순서를 보존하고, (b) 실제로 읽는 바이트와 복사량을 줄이며, (c) 필요 시 디스크/원격 데이터 이동과 계산을 중첩한다. 이것은 새로운 J syntax나 암묵적 GPU 구현이 아니라, §5의 Physical Planning/Execution Scheduling에 따른 물리 실행 확장이다. **최초 native CPU M4 vertical slice의 선행 조건으로 삼지 않는다.** 모든 기능은 [§10 IO-01~IO-30 수용 게이트](./11-roadmap-checklists-p05.ko.md#out-of-core-io-checklist)를 통과하기 전에는 계획/후보일 뿐이다.

**원본/선행 기법 비교(직접 계승 vs 조건부 응용).**

| 출처 / 원본 | 검증한 아이디어 | RustJ 차용 범위 / 금지되는 비약 |
|---|---|---|
| Jsource `jlibrary/addons/data/jmf/jmf.ijs` | J noun에 file mapping; R/W·read-only·copy-on-write, header/shape와 unmap 참조 제약. non-jmf typed boxed mapping 제한과 JMF-backed boxed 회귀 fixture가 공존 | mapped dense array backend 후보. mmap=비동기 I/O·zero page fault·J 전체 boxed/sparse 지원으로 해석하지 않음 |
| Jsource `jsrc/xf.c`, J foreign `1!:11`/`1!:12` | file offset+length의 부분 read/write; 현재 C 경로는 `fread/fwrite` 기반 동기식 | 순차 byte-range 기준 경로와 J-visible foreign I/O 의미 비교. 그래프에서 effectful file foreign을 순수 scan으로 자동 대체 금지 |
| Jsource `jsrc/v.c` 등 | reference/in-place 가능 조건에 따라 버퍼 재사용·복사 억제 | alias/lifetime proof 후 reuse; mapped write와 implicit in-place를 동일시하지 않음 |
| Jd (J 데이터 라이브러리) | 열별 파일 매핑과 파티션 조건별 selective read를 실제 소스에서 확인 | 데이터셋 관리 원리를 참고하되 full-J 의미론으로 일반화하지 않음. Jd column-file on-demand mapping, partition-column read pruning, SIMD mapped-tail 방어를 소스에서 확인 |
| DuckDB (2026 async I/O) | compute pool·blocking I/O pool 분리, job/read-ahead, 메모리 governor 연동, park/resume | bounded read-ahead·메모리 예약·작업 완료 이벤트. DuckDB 전체 스케줄러 복제 불필요 |
| Polars Lazy | projection/predicate/slice pushdown, common subplan reuse | 정확한 access/effect/witness가 있을 때만 필요 byte-range 축소; 임의 J verb·reduction에 무조건 적용 금지 |
| Apache Arrow Dataset Scanner | `batch_readahead`와 `fragment_readahead`의 다른 단위, metadata/pruning | chunk/fragment 단위 선택과 bounded concurrency; 전체 Arrow 데이터모델 복제 불필요 |
| Ray Data | streaming block queues, downstream backpressure, spill/resource account | 독립 실행 단계의 bounded queue·pressure handling. shuffle/reduce barrier를 무시한 임의 streaming 금지 |
| DeepSpeed ZeRO-Infinity | CPU/NVMe/GPU offload와 transfer/compute overlap, prefetch | 추후 placement-aware weight prefetch; GPU runtime 보류 정책 유지 |
| FlexGen (ICML 2023) | device/CPU/disk placement와 layer/batch-block scheduling의 I/O 재사용·throughput tradeoff | 안정된 read-only weights와 합법적 scheduling 영역에 한해 중복 로드 감소; J 실행 순서 임의 변경 금지 |
| TensorFlow `tf.data` | prefetch/parallel map/overlap pipeline | 독립된 데이터 공급 단계의 후속 비교 후보; full J 적용 보장 아님 |

출처(확인 기준 2026-10-06): [Jsource jmf](https://github.com/jsoftware/jsource/blob/master/jlibrary/addons/data/jmf/jmf.ijs) · [Jsource xf.c](https://github.com/jsoftware/jsource/blob/master/jsrc/xf.c) · [DuckDB async I/O](https://duckdb.org/2026/07/31/asynchronous-io) · [Polars optimizer](https://docs.pola.rs/user-guide/lazy/optimizations/) · [Arrow Scanner](https://arrow.apache.org/docs/python/generated/pyarrow.dataset.Scanner.html) · [Ray Data execution](https://docs.ray.io/en/latest/data/data-internals.html) · [ZeRO-Infinity](https://www.deepspeed.ai/tutorials/zero/) · [FlexGen 논문](https://proceedings.mlr.press/v202/sheng23a.html). 프레임워크의 공개된 설계를 참고하는 것이며 해당 기능이 RustJ에 구현됐다는 뜻이 아니다.

**계층 소유권·공통 불변조건**

| 계층 | 입력/출력과 책임 | 금지 |
|---|---|---|
| J Semantic IR / J Graph IR | J noun의 type/shape/atom-order, boxed/sparse, Rank zero-frame prototype, access/effect/error 순서; 순수한 기회 후보만 sidecar로 표시 | 파일 offset·chunk 크기·prefetch queue·장치 DMA를 J-visible 연산 의미로 삽입 금지 |
| Verified Logical IR / analyzer | `AccessRelation`·필요 데이터 범위/재사용 가능성, unknown/opaque, purity와 `FactWitness/Guard` 유지. index/select/view에 대해 합법적 범위 pruning 후보 생성 | arbitrary verb/semantic error가 관찰 가능한 입력을 검사 없이 생략 금지 |
| Physical Planner / Schedule | `StorageSource`(Owned/Mapped/Chunked/Remote 후보), placement, byte-range, Read/Prefetch/Transfer/Materialize/Release, dependency, memory budget, cost 후보 | speculative I/O의 J-visible error/효과 재배치, semantic `StorageRequirement`을 materialization으로 동치화 금지 |
| Executor / Runtime | 처음에는 동기 `read_at`/`write_at` + chunk iteration; 이후 pending/ready/error/cancel completion, pinned lease, backpressure, finite queues | buffer lifetime을 I/O completion 전에 종료, 외부 변경된 bytes를 version 불변으로 가정, 완료 전 쓰기를 성공으로 보고 금지 |
| External adapters | 실제 backend의 read/write/async/mmap capability, errors, alignment, atomic/ordering, ownership을 선언 | J semantics 또는 non-CPU 성능을 암묵적 fallback 능력으로 가장 금지 |

**별도 identity.** `ValueId`는 J 논리값, `StateResource`는 변경 가능한 가중치/파일 상태의 의미적 identity, `BufferId/BufferLease`는 실제 메모리 소유자, `StorageObjectId/Version`은 외부 backing object와 읽기 일관성, `IoRequestId/CompletionToken`은 I/O 요청·완료를 식별한다. 하나로 합치지 않는다. `StorageObjectId/Version` 및 `IoRequestId`는 **명명 후보**이고 확정 Rust 타입/API가 아니다.

**읽기 계획과 정당성.** offset/length 계산은 checked arithmetic·overflow 검사, 정렬·EOF/short read·stale version·권한 실패를 정의한다. read-only immutable input 또는 스냅샷 증명 없는 speculative prefetch는 효과·오류 순서에 영향을 줄 수 있으므로 금지/보수적 barrier를 둔다. chunk size·layout을 바꾸어도 J shape/rank, boxed/sparse, empty frame/fill-cell result, tolerant comparison, 예외 발생 위치가 달라지면 안 된다. 빈 결과의 데이터 byte-range는 0일 수 있지만 J Rank의 가상 cell 실행/형식 추론 의무는 독립이다.

**실행 원칙.** 우선 동기 contiguous/chunked CPU 경로로 correctness baseline을 만든다. 다음에 source access proof로 scan bytes를 줄인다. 그 뒤 bounded async I/O queue와 적어도 2개의 버퍼로 `Read(n+1)`과 `Compute(n)`를 overlap한다. prefetch는 예약 memory budget와 consumer progress/backpressure로 제한하고, request-completion을 `AsyncToken/Timepoint`/equivalent dependency로 명시한다. weight reuse/batch-block schedule은 read-only/version stable, dependency/effect/latency-policy 합법성 증명 후에만 선택한다. `mmap`은 독립 후보이며 page-fault 비용·page cache·주소공간·unmap·COW semantics를 직접 측정한다. `io_uring`·direct I/O·remote object store·GPU DMA는 portable baseline 이전의 요구사항이 아니다.

**쓰기는 읽기와 분리.** J foreign 파일 쓰기·mapped shared write의 observable effect와 checkpoint persistence는 동일하지 않다. checkpoint는 immutable snapshot/version capture → 임시 파일/쓰기 완료 → 필요한 flush/fsync 계약 → atomic publish/rename 가능성 검사 → 성공 반환으로 구분한다. 플랫폼별 durability 보장과 crash consistency를 실제 지원 범위로 한정한다. delayed save로 user-visible state/error 순서가 바뀌면 최적화 금지. cancellation/partial write/retry에서 중복 side effect와 파일 파손을 검사한다.

**비용/계측 계약.** `ResourceEstimate`(peak resident, pinned/inflight bytes, open handles, max queued jobs)와 `CostEstimate`(cold/warm bytes, number/seeks/latency, effective bandwidth, CPU cycles, transfer/overlap time)를 분리한다. 성능 게이트에서 wall time, read/write bytes, physical requests, blocking wait, compute time, peak+retained memory, page faults/cache hits, spill count, throughput *및* latency를 비교한다. warm page cache 결과를 cold storage 승리로 포장하지 않는다.

**대표 실행 예.** `W1`·`W2`가 파일에 저장된 순수한 read-only tensor라고 증명되면 physical schedule은 `Prefetch(W1) → Await(W1) → Compute(W1)`와, 그 계산 중 `Prefetch(W2)`를 겹친다. 다음 `Compute(W2)`는 앞 층 결과 및 W2 완료를 모두 기다린다. Logical graph에는 이 I/O 순서를 박지 않는다. 분기/동적 NAME/수정 가능한 weight/관찰 가능한 file effect가 있는 경우 이 순서를 무조건 적용하지 않는다. 수용 작업표는 [§10 IO](./11-roadmap-checklists-p05.ko.md#out-of-core-io-checklist)를 따른다.


<a id="851-독립-재검토--j-라이브러리포맷모델-로더-보강-2026-10-06"></a>
#### 8.5.1 독립 재검토 — J 라이브러리·포맷·모델 로더 보강 (2026-10-06)

**발견:** 기존 §8.5는 async read-ahead에 비해 실제 저장 포맷, read-chunk/write-shard 배치, mapped SIMD tail, refcount/unmap, cache 일관성 계약이 약했다. 다음은 원본 구현/공식 설명과 RustJ 후보를 분리한 표이다. 소스나 문서 존재는 RustJ 구현 성공이 아니다.

| 확인한 출처 | 실제 구조 / 기능 | RustJ 적용·유의사항 |
|---|---|---|
| [J jfiles와 keyfiles](https://github.com/jsoftware/jsource/tree/0a5101cfdd834b23a0b89d455e4f327310520a08/jlibrary/addons/data/jfiles) | serialized J components를 offset-indexed read로 접근, keys로 개별 component 검색 | arbitrary serialized noun은 typed dense mapped bytes와 다른 encoding. 전체 jfiles 포맷 복제는 보류 |
| [Jd column.ijs](https://github.com/jsoftware/data_jd/blob/0492991263a05bafa84ceca15f0f8249cfc62dcf/base/column.ijs) | file-backed column map on demand, grow/remap, multi-process reference-count 위험 주석 | remap/lease/alias/동시 접근 검사; Jd의 DB lock을 일반 J noun 규칙으로 복제 금지 |
| [Jd api_read.ijs](https://github.com/jsoftware/data_jd/blob/0492991263a05bafa84ceca15f0f8249cfc62dcf/api/api_read.ijs) · [ptable 튜토리얼](https://www.jsoftware.com/jd_tuts.html) | partition column 조건으로 대상 파티션을 좁혀 읽기 | 실제 partition pruning 선례. arbitrary J verb의 자동 pushdown은 별도 effect/error/witness proof가 필요 |
| [Jd jmfx.ijs](https://github.com/jsoftware/data_jd/blob/0492991263a05bafa84ceca15f0f8249cfc62dcf/base/jmfx.ijs) | SIMD kernel의 파일 끝 beyond-end 접근을 우려해 padding; 4096 byte 페이지 가정 | **반면교사**: RustJ는 페이지 크기·허용 overread를 하드코딩하지 않고 masked tail/checked span/unmap-safe lease 구현 |
| [Zarr v3](https://zarr.readthedocs.io/en/stable/user-guide/arrays/) · [HDF5 chunk cache](https://docs.h5py.org/en/stable/high/file.html) | 논리 shape와 read chunk, write shard/캐시를 분리; access-pattern-dependent chunk shape | chunk read amplification, 파일 개수, shard write amplification과 메모리 경계 최적화; 자체 파일 포맷 확정 보류 |
| [Safetensors](https://github.com/huggingface/safetensors/blob/main/README.md) | tensor dtype/shape/byte-range index, 0-byte empty tensor, partial/lazy loads | verified storage manifest를 제안; invalid/overlap/overflow/off-end 검증, scalar/empty prototype 보존. 외부 포맷 adapter는 후속 |
| [llama.cpp loader](https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md) | mmap/no-mmap/mlock/direct-I/O/NUMA/lazy tensor rows 선택 | mmap을 universal default로 결정하지 말고 workload/OS cache/RSS/page faults/cold-warm으로 비교 |
| [IREE Stream dialect](https://iree.dev/reference/mlir-dialects/Stream/) | parameter.load/read/gather/write, AsyncTransfer, Timepoint로 외부 가중치 바이트 범위의 비동기 준비·저장과 완료 시점·resource lifetime 명시 | **직접 참고 1순위**: 정적인 read-only weights → Physical Stream ParameterRead/Load 후보 → Await completion → consumer, checkpoint write의 completion은 별도 durability 계약. J-visible foreign effect와 동일시 금지 |
| DuckDB, Polars, Arrow, Ray, ZeRO-Infinity, FlexGen | scan pruning, batch/fragment read-ahead, async governor, streaming pressure, staged weight reuse | read bytes 축소 → stable data version → bounded scheduling → overlap → legal reuse 순서로만 도입 |

**Boxed 표현 정정:** [jmf.ijs](https://github.com/jsoftware/jsource/blob/0a5101cfdd834b23a0b89d455e4f327310520a08/jlibrary/addons/data/jmf/jmf.ijs)는 non-jmf typed boxed mapping을 거부하지만 [JMF-backed boxed tests](https://github.com/jsoftware/jsource/blob/0a5101cfdd834b23a0b89d455e4f327310520a08/test/gmbx.ijs)가 존재한다. 반면 [jsrc/mbx.c](https://github.com/jsoftware/jsource/blob/0a5101cfdd834b23a0b89d455e4f327310520a08/jsrc/mbx.c)는 not supported라고 표시된다. 따라서 지원/미지원은 파일 형태·연산·원본 버전의 실제 C oracle로 세분화해야 한다.


**IREE Stream 의존성 경계:** `stream.async.parameter.load/read/gather/write`, `stream.async.transfer` 및 `stream.timepoint`의 구현을 I/O-specific Physical Schedule 선례로 추가한다. `parameter.load`의 사전 실행 허가와 J 외부 파일 읽기 효과의 비투기성은 다르며, buffer 사용 가능 시점 및 cancellation/error가 보존될 때만 이동·중복 제거한다. IO-13~18과 IO-20의 verifier 검토 입력으로 사용한다. [IREE 공식 Stream 사양](https://iree.dev/reference/mlir-dialects/Stream/).

**추가 계층 계약:** logical ValueId와 외부 StorageObject/Version, 물리 StorageEncoding(contiguous typed / chunked typed / serialized component / external), ReadChunk, WriteShard, BufferLease, IoCompletion을 서로 구분한다. 이들은 구현을 확정한 Rust 구조체 이름이 아닌 개념이다. File foreign I/O는 J-visible effect이지만, verified immutable storage read의 내부 스케줄은 효과가 없는 경우에 한해 변경 가능하다. Header/shape/index가 존재한다고 J Rank empty-frame fill-cell 의미를 생략하지 않는다. Cache key에는 object/version/byte-range/encoding을 포함하고, peak RSS와 OS page cache는 런타임이 완전히 예약하는 메모리로 주장하지 않는다.


<a id="io-framework-execution-comparison"></a>

<a id="852-프레임워크별-io-최적화-실행-방식과-rustj-graph-ir-적용-2026-10-06"></a>
#### 8.5.2 프레임워크별 I/O 최적화 실행 방식과 RustJ Graph IR 적용 (2026-10-06)

**다섯 가지 최적화 대상.** I/O 최적화는 단순한 비동기 파일 읽기가 아니라 **① 불필요한 읽기 제거, ② 저장/읽기 단위 선택, ③ I/O와 계산 중첩, ④ 읽은 데이터 재사용, ⑤ 메모리·완료·실패 통제**를 조합하는 문제다. 최적화의 장소를 구분해야 한다. J Graph/Verified Logical IR은 필요한 데이터와 의미론적 합법성을 판단하고, Physical Plan·Executor는 byte range, chunk, 배치, prefetch, buffer 수명, 실제 memory/resource budget을 결정한다.

| 프레임워크 / 공식 근거 | 실제 기법과 최적화 계층 | RustJ에서 가져올 부분 / 제약 |
|---|---|---|
| [Polars lazy optimizer](https://docs.pola.rs/user-guide/lazy/optimizations/) | Logical 단계에서 predicate/projection/slice를 scan까지 pushdown, common subplan/scan 재사용 | `AccessRelation`이 필요한 논리 원소/축과 실제 byte range를 증명한 경우 읽기를 제거. IO-09~12; Rank, dynamic binding, observable errors/effects에서는 opaque fallback |
| [DuckDB async I/O (2026-07-31 발표)](https://duckdb.org/2026/07/31/asynchronous-io) | Parquet row-group/CSV byte-boundary를 fetch job/task로 쪼갬. `REGULAR` 계산 pool과 주로 blocking I/O인 `ASYNC` pool 분리; 미완료 scan job은 park 후 깨움. read-ahead 깊이는 temporary-memory budget과 연동 | IO-13~17: bounded job queue, completion/wakeup, read-ahead depth를 resource governor로 축소. 공개 글은 v2.0 개발·출시 예정 기능을 설명하므로 모든 릴리스에서 지원된다고 단정하지 않음 |
| [IREE Stream](https://iree.dev/reference/mlir-dialects/Stream/) | Physical resource IR에서 `stream.async.parameter.load`는 새 resource, `read`는 기존 allocation 채움, `gather`는 여러 parameter archive range 결합. `timepoint`/`await`로 이용 가능 시점과 순서 명시 | IO-13~18·20의 직접 선례: immutable weights `Load/Gather → Await → Consumer`. IREE에서 speculative한 parameter read를 J의 관찰 가능한 `1!:` file foreign과 동치화 금지 |
| [Apache Arrow Scanner](https://arrow.apache.org/docs/python/generated/pyarrow.dataset.Scanner.html) | `batch_readahead`와 `fragment_readahead`를 따로 조정 | IO-14~16: chunk 내부와 여러 저장 source 간의 선행 읽기 수준을 분리하고 decoded/inflight buffers 예산화 |
| [Ray Data internals](https://docs.ray.io/en/latest/data/data-internals.html) | 연산자 output queue에 block reference streaming; downstream backpressure, memory-aware task scheduling, spill | IO-15~17: 큐가 차면 생산 속도를 제한. Reduction/shuffle처럼 경계가 필요한 경우 무조건 streaming하지 않음 |
| [Zarr arrays/sharding](https://zarr.readthedocs.io/en/stable/user-guide/arrays/) · [HDF5 chunk cache](https://docs.h5py.org/en/stable/high/file.html) | 물리 chunk·shard 및 캐시/축 접근 패턴으로 read amplification와 재사용 결정 | IO-26~29: `LogicalShape` ≠ `ReadChunk` ≠ `WriteShard`; 작은 slice도 chunk 전체 decode가 필요한지 측정 |
| [llama.cpp](https://github.com/ggml-org/llama.cpp) | 모델 가중치의 mmap/비-mmap·residency 등 로딩 전략 선택 | IO-23·30: mmap이 항상 최적이라고 가정하지 않음. page fault, cold/warm cache, RAM pressure, SSD/remote, reuse별 비교 |
| [DeepSpeed ZeRO](https://www.deepspeed.ai/tutorials/zero/) · [FlexGen](https://proceedings.mlr.press/v202/sheng23a.html) | CPU/NVMe/GPU Offload·prefetch·transfer/compute overlap; FlexGen은 layer/batch-block scheduling으로 weights 반복 로드 감소 | IO-18~20: immutable/version-stable weights에서만 재사용/선행 읽기를 후보화. 훈련의 mutable optimizer/gradient/checkpoint는 별도 snapshot 계약 |
| [TensorFlow tf.data](https://www.tensorflow.org/guide/data_performance) | `prefetch`·parallel map을 통한 input-producer와 모델 연산의 파이프라인 중첩 | IO-14~16: 독립 producer/consumer의 비교 사례. J의 임의 verb/cell 평가를 재배열하는 일반적 허가는 아님 |
| [PyTorch Distributed Checkpoint](https://pytorch.org/docs/stable/distributed.checkpoint.html) | 저장할 state의 staging/async persistence 분리 | IO-18: mutable state의 snapshot·write completion·publishing·failure/recovery를 명시. 비동기 제출만으로 durability 성공을 보고하지 않음 |

**같은 Logical Graph에서 다른 Physical Schedule을 선택하는 예.** `X → MatMul(W1) → Activation → MatMul(W2) → Y`의 J-visible 의존성을 유지한다. 외부 파일의 `W1`/`W2`가 **변하지 않는 read-only/versioned 데이터임이 증명**된 경우, 아래 physical 계획을 생성할 수 있다. 다른 모델에서 임의로 실행되는 것이라고 가정하지 않으며, 실제 구현 이전 설계 예시다.

~~~text
Logical / semantic value flow
X ──> MatMul(W1) ──> Activation ──> MatMul(W2) ──> Y

Physical / possible overlap
Reserve(W1) ──> Read(W1) ──> Ready(W1) ──> Compute(L1) ──> Activation ──┐
Reserve(W2) ──> Read(W2) ──> Ready(W2) ─────────────────────────────────┤
                                                                        ▼
                                                                  Compute(L2) ──> Y
            Read(W2) and Compute(L1) can overlap (if budget allows)
Compute(L2) awaits BOTH Activation and Ready(W2).
Release(buffer) follows last use AND all pending I/O/transfer completions.
~~~

**차용 방법의 핵심 차이.** Polars 방식은 **필요 없는 byte를 아예 요청하지 않도록** 만드는 기법이다. IREE 방식은 **읽기·전송·완료 토큰을 Physical DAG의 명시적 의존성**으로 만드는 기법이다. DuckDB·Arrow·Ray 방식은 **그 DAG의 읽기 요청을 bounded queue·backpressure·memory budget 아래에서 실행**하는 기법이다. DeepSpeed·FlexGen 방식은 **합법적 계산 순서/배치에서 weights의 생존 구간과 재사용을 늘려** 실제 반복 I/O를 줄인다. 이 기법들은 대체 관계가 아닌 계층별 조합 후보이다.

**의미론 경계와 비용 계약.** (A) verified immutable 저장소의 내부 물리 read, (B) J `1!:11`/`1!:12` 등 관찰 가능한 foreign I/O, (C) 수정 가능한 weights·checkpoint 쓰기를 별도로 모델링한다. A에서만 access witness/guard를 통한 범위 pruning, speculative prefetch, range coalescing을 허용한다. B는 J effect/error 순서, C는 version/snapshot/commit/durability 계약을 지킨다. 빈 Frame의 read bytes가 0이어도 J Rank의 가상 Cell·fill·dtype·shape 및 오류 판단은 생략하지 않는다. boxed/sparse, dynamic NAME/Rank, alias, stale file, EOF/short read 및 실패의 조기 관찰은 negative fixture 대상이다.

성능은 **cold/warm 실제 읽은 bytes, request 수, seek/latency, blocked I/O 시간, compute time, page faults, prefetch overlap, peak·retained memory/RSS, spill, 전체 throughput, 개별 호출 latency**를 따로 측정한다. Read-ahead 자체는 필요한 byte를 줄이지 못하며 메모리를 더 사용한다. 작은 range를 합치면 요청 수는 줄지만 초과 읽기가 발생할 수 있다. OS page cache/RSS는 모든 바이트를 RustJ governor에서 정확히 예약할 수 있는 메모리라고 주장하지 않는다. **의미론·자원 안전 → 실측 비용 → 실행 후보 선택** 순서를 유지한다.

**기존 단일 수용 원장 매핑:** 자료 읽기 제거·공통 scan/cache = IO-09~12·29, 파일/Chunk/Shard 형식 = IO-05~08·26~28, async/backpressure/transfer = IO-13~17·20, weights/checkpoint = IO-18~19, benchmark/선택 = IO-21~24·30, 소스·의미론 = IO-01~04·25. 새 체크리스트를 만들지 않고 [§10 IO-01~IO-30](./11-roadmap-checklists-p05.ko.md#out-of-core-io-checklist)의 기존 상태만 갱신한다. 지금은 설계 기록이므로 **0/30 수용** 상태를 유지한다.


<a id="unified-data-movement-contract"></a>
<a id="853-메모리-io디스크-io-공통-planning-계약--신규-ir-계층은-당장-만들지-않음-2026-10-06"></a>
#### 8.5.3 메모리 I/O·디스크 I/O 공통 Planning 계약 — 신규 IR 계층은 당장 만들지 않음 (2026-10-06)

**설계 결정.** 메모리 접근·복사, CPU↔GPU 이동, 파일/매핑된 배열의 구간 읽기·쓰기를 **기존 Physical Planner / Physical Execution Plan의 공통 데이터 이동 분석과 스케줄링 대상으로 취급**한다. `Data Movement IR`이라는 독립적인 신규 계층은 만들지 않는다. 대신 `AccessRegion` 증명, `Effect/Dependency` 계약, physical storage/transfer/ready/lifetime 정보를 기존 단계의 각 소유자가 보유한다. 추후 실제 비동기·복합 route 사례 둘 이상에서 기존 Physical Plan이 의존성·버퍼 수명·스케줄을 안전하게 표현하지 못한다는 구체적 증거가 나올 때에만 별도 resource/stream 실행 IR 추출을 재검토한다. 이것은 §5의 기존 J Graph IR → Verified Logical Execution IR → Physical Plan 구분을 없애는 결정이 아니다.

| 기존 단계 | 이 단계에서 표현하거나 계산하는 것 | 넣어서는 안 되는 것 |
|---|---|---|
| J Semantic IR / J Graph IR | J source topology, ValueId·Rank/CellApply·zero-frame 의미, 원본/후보 provenance; pass-local 접근 후보 sidecar는 허용 | BufferId, 파일 offset, DMA, concrete transfer, 물리 layout 및 GraphFacts 자체의 EffectSummary/선택 완료 상태 |
| Verified Logical Execution IR / 분석 | observable J file/namespace/state effect, error/ordering/guard, 값/효과 liveness, logical `AccessRelation` 및 Unknown/Proven/Guarded 증거 | 임의 file read를 pure 배열 load로 치환하거나, 증명 없는 read omission/reorder 승인 |
| 기존 Physical Planner / Representation / Schedule | `StorageObject/Version`·`BufferId/Lease` 분리, physical `Region`(buffer slice/byte range), `Read/Write/Copy/Transfer/Materialize/Release`, ready/completion 의존성, layout·placement·byte/resource/cost 추정; 공유 pass 인터페이스 사용 | 모든 접근을 실제 복사로 강제, 효과가 다른 파일과 메모리를 단일 semantics로 취급, target capability 없는 전송 확정 |
| Executor / Backend | 동기 CPU reference에서 실제 read_at/write_at/buffer copy 실행; 이후 검증된 async token, queue/backpressure, 장치별 구현 | 미완료 I/O의 조기 buffer 해제, file side effect 재실행, 미확정 GPU 실행 지원 주장 |

**공통 추상화의 경계.** `AccessRegion`은 어떤 논리 원소가 필요한지를 설명하며 physical `BufferSlice`와 `FileByteRange`는 다른 concrete region 종류다. 공통 분석은 region interval, producer/consumer, placement, lifetime, alias, ordering, transfer feasibility와 비용을 비교하지만 파일과 메모리를 하나의 alias domain·주소공간·오류 계약으로 뭉개지 않는다. `ValueId`, mutable `StateResource`, external `StorageObjectId/Version`, physical `BufferId/BufferLease`, `IoRequestId/CompletionToken`은 계속 분리한다. 이름은 설계 후보이며 확정 Rust API가 아니다.

**컴파일 타임 vs 런타임.** 컴파일 타임에 proven shape/dtype/access pattern, 사용/수명, 중간 배열 제거, buffer reuse, 필요한 데이터 구간·전송·파이프라인 후보를 계획한다. 런타임은 실제 파일 내용/변경·EOF·권한·데이터 크기, 사용 가능한 RAM/GPU 메모리, 페이지 캐시·대역폭·요청 완료를 확인하고 guard/capability/memory budget을 검사한다. 비용 모델은 memory traffic, file bytes/requests/seeks, transfer, kernel launch, sync, peak/inflight/retained bytes, cold/warm latency를 분리한다. Unknown을 0 비용 또는 재배치 허가로 해석하지 않는다.

**효과 안전성.** (A) proven immutable/version-stable 내부 backing read, (B) J `1!:`의 관찰 가능한 파일 I/O, (C) mutable state/checkpoint publish는 별개다. (A)에서도 실패가 관찰될 수 있으면 speculative read/오류 노출 위치가 보존된다는 별도 증명이 필요하다. (B)의 미사용 읽기라도 존재성/권한/오류를 생략하지 않으며, (C)의 쓰기 완료·durability를 단순 async submit으로 간주하지 않는다. zero-byte/empty frame에서도 J의 virtual fill-cell, dtype/shape 및 필요한 오류 검사는 남긴다.

**프레임워크 역할 분담(직접 복제 금지).** [IREE Stream](https://iree.dev/reference/mlir-dialects/Stream/)은 resource·async transfer·parameter/file read/write·ready token의 **물리 실행 그래프** 선례다. [MLIR Bufferization](https://mlir.llvm.org/docs/Bufferization/) 및 [Memory Effects](https://mlir.llvm.org/docs/Rationale/SideEffectsAndSpeculation/)는 논리값↔버퍼 분리와 operation별 effect/resource interface의 선례다. [XLA GPU architecture](https://openxla.org/xla/gpu_architecture)는 fusion·buffer assignment·layout/transfer를 점진적으로 구체화하는 선례이며 일반적인 J 파일 I/O의 공통 IR은 아니다. [TVM](https://tvm.apache.org/docs/)은 장치 배치·물리 메모리 선택, [DataFusion](https://datafusion.apache.org/)은 파일 scan과 filter/projection pushdown의 선례다. 어떤 프레임워크도 full-J foreign 오류·Rank prototype 계약을 자동으로 해결하지 않는다.

**대표 추적 사례.** `File(A, immutable version) → Slice → Elementwise → Consumer`는 verified access witness가 있을 때 physical byte-range read와 fused CPU 계산으로 낮출 수 있다. 반면 `1!:1`/`1!:11` 결과가 사용되지 않아도 파일 open/EOF/error 등 J-observable 경계가 있다면 dead-data elimination만으로 읽기를 지울 수 없다. CPU↔GPU copy는 같은 planner의 transfer 후보지만 현재 CUDA 구현 유보 상태를 유지한다.

**기존 이행 원장 연결.** 새 `DM-*` 작업표는 만들지 않는다. IO-03(계층/identity/effect interface), IO-09(접근 영역 증명), IO-13·14(동기→비동기 준비/완료), IO-17(재배치·실패·해제 안전), IO-20(공통 memory/file/transfer 자원·비용 planning), IO-22(세 경로·negative 검증)를 기존 [§10 IO 체크리스트](./11-roadmap-checklists-p05.ko.md#out-of-core-io-checklist)의 수용 항목으로 사용한다. FW-05~09의 provenance/guard와 DB effect contract를 선행 증명으로 재사용한다. **현 상태: 설계 확정, 구현·실행 증거 없음; IO 0/30 [ ] 유지. M2 frontend와 최초 M4 CPU slice의 필수 조건으로 올리지 않는다.**

<a id="io-a-source-audit"></a>
<a id="854-io-a-원본-소스-교차-감사실행-기준-2026-10-0607"></a>
#### 8.5.4 IO-A 원본 소스 교차 감사·실행 기준 (2026-10-06~07)

**범위·상태.** IO-01·IO-25의 1차 코드 감사를 pinned source에서 수행했고 IO-02의 effect/error 계약이 요구하는 반례를 도출했다. 이것은 **소스 확인만 완료한 부분 진행**이다. 원본 J 파일 foreign 일부 fixture는 실행했으나 mapped/Jd/전체 외부 효과 검증은 남아 있으므로 IO-01·02·25는 [ ]이고 IO 전체 수용 0/30을 유지한다. 외부 저장소의 동작은 RustJ 지원 사실이 아니다.

| 고정 원본과 확인 위치 | 원본에서 직접 확인한 내용 | RustJ에서 아직 증명할 사항 |
|---|---|---|
| [jsource `xf.c` @0a5101c](https://github.com/jsoftware/jsource/blob/0a5101cfdd834b23a0b89d455e4f327310520a08/jsrc/xf.c) `jtjiread` / `jtjiwrite` / `jtixin` | `1!:11`은 file open/size 확인 뒤 index·length를 검사하며 `j≤size, j+length≤size, length≥0`을 확인한다. `1!:12`는 `n=null` 호출이므로 같은 read end-bound를 강제하지 않고 nonnegative 시작 index를 확인한 뒤 write한다. 음수 시작 index는 size 상대 계산을 거친다. `jtrd/jtwa`는 동기 `fread/fwrite` 경로다. | invalid filename/open과 index error 우선순위, EOF/short I/O, write beyond EOF·파일 변경·권한·효과 후 재실행 금지. 단편 소스만으로 동시 truncate의 결과/atomicity를 보장하지 않음 |
| [jsource `jmf.ijs` @0a5101c](https://github.com/jsoftware/jsource/blob/0a5101cfdd834b23a0b89d455e4f327310520a08/jlibrary/addons/data/jmf/jmf.ijs) + [`gmbx.ijs`](https://github.com/jsoftware/jsource/blob/0a5101cfdd834b23a0b89d455e4f327310520a08/test/gmbx.ijs) | mapping mode 0=RW, 1=RO, 2=COW가 구분되고 unmap은 live refs에 의해 실패할 수 있다. `additem`은 type 32 boxed에 `not supported for boxed data` 검사를 둔다. 반면 `gmbx.ijs`는 mapped boxed 명칭을 사용하지만 확인한 실제 assertion은 `'' -: q`, `'' -: r`로, 일반 boxed payload의 읽기·쓰기 성공을 입증하지 않는다. | JMF 포맷·타입·모드별 boxed 경계를 실행 oracle로 정확히 구별; copy-on-write commit과 shared write 혼동 금지 |
| [data_jd `column.ijs` @0492991](https://github.com/jsoftware/data_jd/blob/0492991263a05bafa84ceca15f0f8249cfc62dcf/base/column.ijs) · [`jmfx.ijs`](https://github.com/jsoftware/data_jd/blob/0492991263a05bafa84ceca15f0f8249cfc62dcf/base/jmfx.ijs) | column map/remap과 타 프로세스 refcount 위험 주석, DB lock 기반 제한을 확인. `jmfx.ijs`의 overread 방어에는 `PAGESIZE=:4096` 가정과 trailing padding 보정이 들어 있다. | RustJ는 페이지 크기·SIMD overfetch를 하드코딩하지 않고 checked span/masked tail 및 remap·lease safety 실측; Jd lock을 J 의미 계약으로 일반화 금지 |
| [jsource `jfiles.ijs` @0a5101c](https://github.com/jsoftware/jsource/blob/0a5101cfdd834b23a0b89d455e4f327310520a08/jlibrary/addons/data/jfiles/jfiles.ijs) · [data_jd `api_read.ijs` @0492991](https://github.com/jsoftware/data_jd/blob/0492991263a05bafa84ceca15f0f8249cfc62dcf/api/api_read.ijs) | jfiles `j_read`는 `3!:2 @ (1!:11)`로 offset 기반 serialized component를 읽으며, `jread`가 directory index에서 범위를 취득한다. Jd `readptable`은 partition 조건으로 대상 table을 결정한다. | serialized component와 typed contiguous mapped backing 구분. Jd의 선택적 읽기를 임의 J verb/foreign read에 자동 적용 금지 |

**잔여 실행 fixture (IO-01/02/25 수용 선행).** 동일한 임시 파일을 대상으로 정상 `1!:11` 부분 읽기, 끝 위치·길이 경계, negative relative offset, `1!:12` EOF 이후 쓰기, 존재하지 않는 파일과 권한 오류, 처리 중 외부 변경·단축 읽기, 번호형 핸들과 파일명 입력의 오류/해제 차이를 고정 J C binary에서 관찰한다. JMF는 RW/RO/COW·live-ref unmap·mapped boxed 명칭 대비 실제 payload 종류·resize/remap을, Jd/jfiles는 partition/keyed component·serialized byte-range를 별도 원본 fixture로 조사한다. **예상 오류 클래스나 처리 순서를 실행 전에 합격 기준으로 단정하지 않고 observed J behavior를 baseline으로 고정**한다. 결과는 IO 원장의 commit·원본 pin·명령·실행환경·pass/fail/unsupported에 기록한다.

**IO-A 실행형 진단 도구 (2026-10-06).** [tools/file_io_audit.py](../../../tools/file_io_audit.py)에 임시 파일만 사용하는 15개 고정 J C oracle 사례를 작성했다. 전체 읽기·크기, 정상 구간·EOF 0길이·음수 시작·범위 오류, 부분 쓰기·파일 끝 이후 쓰기, 존재하지 않는 파일, 결과가 버려지는 `1!:11` 실패를 포함한다. [tools/test_file_io_audit.py](../../../tools/test_file_io_audit.py)는 fixture 고유성·파일명 quoting·임시 경로 제한·결과/실패 분류를 오프라인 검사하고, [Linux CI](../../../.github/workflows/linux.yml)는 `j64/j64avx2 × default/portable`에서 고정 C library를 빌드한 뒤 비수용 진단 JSON을 artifact로 남기도록 연결했다. `--gate` 없는 실행은 실패 발견을 기록하되 합격이라고 주장하지 않는다. J C oracle 결과를 확인하기 전에는 오류 클래스·플랫폼 종속 EOF hole bytes·mapped boxed 의미를 확정하지 않는다. 소스 pin `0a5101cf`과 CI binary pin `13994ffa`의 `xf.c`·`jmf.ijs`·`gmbx.ijs`·`jfiles.ijs`는 각 파일의 Git blob SHA가 동일함을 교차 확인했다. **이것은 IO-01/02/25의 구현/동등성 수용이 아니며 계속 [ ] 상태다.**

**첫 CI 진단 오류 분석(수용 아님).** [Linux CI 37470524210](https://github.com/yunskim/RustJ/actions/runs/37470524210)의 `j64`와 `j64avx2` 초기 실행은 각각 15개 중 기대값 9개 일치·6개 재검토였다. 여섯 항목 모두 `length error`로, J 의미 차이의 증거가 아니라 **indexed foreign용 파일 인수의 잘못된 boxing**을 원인으로 분리했다. [J Files 원문](https://www.jsoftware.com/help/dictionary/dx001.htm)은 `1!:1 <'name'`과 달리 indexed `1!:11 'name';offset length`, `x 1!:12 'name';offset` 형식을 사용한다. [수정 commit 843ae79](https://github.com/yunskim/RustJ/commit/843ae79d922d57f40d8378f56d70e9cbb251b41a)에서 indexed 파일명 unboxed 전달로 교정하고, `length/rank/syntax` error를 파일 존재 실패로 세던 거짓 양성도 차단했다. [단위테스트 보완 bfcfe7e](https://github.com/yunskim/RustJ/commit/bfcfe7e9e1dffaefa095a837f641c18fdda7c1fe) 이후 재검증 결과가 확정되기 전에는 9/15를 기능 통과로 해석하지 않는다. IO-01/02/25와 전체 0/30 미수용 상태 유지.

**고정 J 실행 검증(2026-10-06).** [Linux CI 37470839455](https://github.com/yunskim/RustJ/actions/runs/37470839455)에서 `check` 작업은 성공했고, 실제 실행·로그가 확인된 `j64/default`, `j64/portable`, `j64avx2/default` 각각 **file-foreign 15/15 기대 동작 일치·재검토 0**이었다. `j64avx2/portable`는 이 기록 시점 실행 진행 중이므로 합계에 포함하지 않는다. source-based 15건을 이후 회귀로 고정하기 위해 [workflow 3d476f6](https://github.com/yunskim/RustJ/commit/3d476f6f4f8bd1c925e2216ee856cfdfcd5fc9d2)에 `--gate`를 추가했다. 이 **C oracle 자체의 fixture gate**는 RustJ의 세 경로 비교나 JMF/Jd/boxed/비동기 I/O 구현을 검증하지 않는다. 따라서 IO-01·02·25 및 전체 IO 0/30 [ ] 유지.

**IO-02 실행 순서 기준 확대(2026-10-07, 원본 J oracle 일부 검증).** [tools/file_io_audit.py](../../../tools/file_io_audit.py)에 기존 독립 파일 사례 15개와 별도로, **6개 ordered-effect case / 총 14개 J step**를 추가했다. 같은 J 엔진과 같은 임시 파일에서 각 JDo 후 파일 내용을 재검사하여 (1) `0 [ write`의 버려진 결과와 살아 있는 쓰기 효과, (2) 오른쪽 read 실패가 왼쪽 write 실행을 막는 J 순서, (3) 이미 완료된 쓰기 뒤의 실패는 이전 파일 변경을 되돌리지 않음, (4) 잘못된 indexed write는 이전 정상 쓰기를 훼손하지 않음, (5) append 후 끝 구간 읽기, (6) truncate 후 전체 읽기를 구분한다. [tests](../../../tools/test_file_io_audit.py)는 오류 전에 임의 쓰기가 실행되는 버그를 부정 사례로 주입하고 검출한다. [CI 37537959953](https://github.com/yunskim/RustJ/actions/runs/37537959953)에서 `check` 성공 및 `j64/default` C oracle **독립 15/15 + 순서 6/6(14 step) 일치**, 불일치 0을 실제 확인했다. 나머지 matrix는 검사 시점 미확정으로 이 기록의 완료 증거에서 제외한다. 이는 **C에서 확인한 의미론 기준**이며 RustJ native/optimized 동등성 및 mapped/foreign 전체 커버리지가 아니므로 IO-02 [ ], 전체 IO 0/30 [ ]을 유지한다.

**IO-02/IO-25 다음 검증 순서(기존 원장의 하위 실행 단위).** (a) `1!:1/2/3/4/11/12`의 파일명 vs 번호 핸들, 권한·close·flush, EOF/short read, 동시 truncate 효과를 실제 file fixtures로 확장; (b) 고정 `jlibrary`를 로드하는 J bootstrap을 분리 검증하고 `jmf` RW/RO/COW 및 refs/unmap/resize·typed/boxed 변형을 별도 smoke oracle로 수행; (c) Jd/jfiles는 테스트 fixture/원본 버전·의존성 확보 후 독립 단계로 실행. 원본 라이브러리 초기화 실패를 J 의미 실패로 오분류하거나 C oracle 15+6 일치를 RustJ 구현 수용으로 승격하지 않는다.

**IO-02 전체 고정 C matrix 재확인(2026-10-07).** [Linux run 37537959953](https://github.com/yunskim/RustJ/actions/runs/37537959953)의 `check` 및 `j64/default`, `j64/portable`, `j64avx2/default`, `j64avx2/portable` 네 reference job은 모두 **success**. 각 reference job은 독립 파일 사례 **15/15**, 순서 시나리오 **6/6(14 JDo step)**, `requires_review=0`, `ordered_requires_review=0`으로 동일하게 통과했다. 이는 조합별 C oracle probe 결과이며 RustJ의 같은 세 연산 경로를 비교하는 시험이 아니고 IO 수용점수는 변하지 않는다.

**IO-25 JMF 첫 독립 smoke 실행 시도(수용 전).** [tools/jmf_smoke.py](../../../tools/jmf_smoke.py)에 J 원본 `jlibrary/bin/profile.ijs`와 `load 'jmf'` 부트스트랩을 고정 라이브러리·별도 임시 HOME으로 실행하고, 임시 JMF file에 대해 RW(0)→RO(1)→COW(2)를 **각각 map → 빈 noun 비교 → unmap 결과 0**으로 관찰하는 독립 실행 후보를 구현했다. [검증용 JMF 시나리오 테스트](../../../tools/test_jmf_smoke.py)를 추가하고 [Linux CI](../../../.github/workflows/linux.yml)에 비수용 탐색 단계를 연결했다. 원본 J profile/add-on 초기화 또는 JMF mapping이 실패하면 `blocked`를 별도로 기록하며, 이 시점에는 JMF CI 실행 결과/기능 지원을 주장하지 않는다. 이 작은 smoke는 **실제 boxed payload, mutable persistence, RO/COW 쓰기 차이, refs에 의한 unmap 거부, resize/remap, Jd partition** 검증이 아니므로 IO-25 [ ] 유지.

**JMF 부트스트랩 첫 실측 결과(2026-10-07, 미수용).** [Linux 37538470000](https://github.com/yunskim/RustJ/actions/runs/37538470000)의 `j64/default`와 `j64/portable` C job에서 JMF 탐색 단계는 `status=blocked`, `stage_count=2`였으며, 첫 `BINPATH_z_` 설정은 실행되고 `0!:0 <.../jlibrary/bin/profile.ijs`에 `domain error`가 발생했다. **map/unmap까지 도달하지 않았다.** 이 결과를 JMF RW/RO/COW의 부정 결과로 해석하지 않는다. [후속 진단 541be39](https://github.com/yunskim/RustJ/commit/541be39f42f1df655e7af5bde86d928321f4c9b3)는 실제 J script loader가 짧은 독립 fixture를 읽을 수 있는지 먼저 검사하고, 실패 원본에 `13!:12` 문맥을 포함하도록 분리했다. 후속 CI 결과는 이 기록 시점 미확정이다. IO-25는 [ ]이다.

<a id="io-static-source-contracts"></a>
<a id="855-정적-입력datasource-계약과-역방향-데이터-요구량-분석-2026-10-11-설계-후보--미구현"></a>
#### 8.5.5 정적 입력·DataSource 계약과 역방향 데이터 요구량 분석 (2026-10-11, 설계 후보 / 미구현)

**결정 범위.** 과거 JAXA의 `with` 구상을 RustJ의 **ordinary-name conjunction으로서 typed semantic annotation/contract만 부착한다**는 기존 규약 안에서 재사용한다. `with`는 입력에 대한 *정적 의미론적 계약*을 보완하고, 외부 DataSource의 *접근 capability·실제 저장 특성·일관성 증거*는 별도의 binding 계약으로 취급한다. `with`에 파일 경로, chunk 크기, prefetch 깊이, tile/register/device 정책을 넣지 않는다. 데이터 원소를 컴파일 시점에 모두 읽을 필요는 없으며, 같은 generic J graph를 서로 다른 입력과 storage adapter에 반복 적용할 수 있어야 한다. 이는 **설계 후보**이며 현재 `with` 및 Out-of-core API가 구현됐다는 뜻이 아니다. 기존 [§8.5.3](#unified-data-movement-contract) 계층 및 [IO-01~IO-30](./11-roadmap-checklists-p05.ko.md#out-of-core-io-checklist) 외에 새로운 필수 IR 또는 별도 수용 원장은 만들지 않는다.

**세 계약의 분리.**

| 계약 주체 | 사전에 알거나 보장하는 정보 | 하지 않는 일 |
|---|---|---|
| `with` / 함수·입력 semantic contract | 경계 입력의 J-visible dtype, shape(Exact/Symbolic/Dynamic), 인수 사이 차원 제약, 증거가 있는 배열 속성(정렬·유일성·범위 등), 선택적인 static value, 의미론적 adjoint 연결 후보 | 실행할 파일 경로·buffer address·저장 chunk·하드웨어 schedule을 결정하지 않음; 중간 결과 shape/Rank/Cell을 사용자에게 전부 선언시키지 않음 |
| DataSource의 *static requirement/capability contract* | logical domain/schema와의 호환성, sequential/seek/region read, repeatability, write/append/atomic publish 지원 여부, version/snapshot을 요구할 수 있는지, codec/encoding 지원 | capability가 빠르다는 보장 아님; read-only 핸들만으로 외부 불변성을 보장하지 않음 |
| 실제 *BoundSourceDescriptor*와 runtime witness | 연결 시 확인한 source identity, version/snapshot lease, storage layout/strides, read/write/codec chunk geometry, byte offsets/index, 압축·endianness, alignment, 파일 크기/권한, 가용 자원 | 확인되지 않은 snapshot·원소 성질·성공한 I/O·지연시간을 정적 사실로 승격하지 않음 |

**계약 강도 네 가지.** (1) **Hard/semantic requirement**(dtype, 확정 shape, J-visible type, 일관된 index domain, 허용 가능한 snapshot): 불일치 시 해당 특수화·실행 경로를 거부하거나 guard를 통한 안전한 재특수화/일반 경로로 이동한다. (2) **Capability**(region read, replay, 독립 부분 쓰기, seek): 후보가 *가능한가*만 제한하며 성능을 약속하지 않는다. (3) **Soft performance preference**(선호 read bytes, chunk aspect ratio, bandwidth hint): 비용 모델에서만 사용, legality 증명에는 사용할 수 없다. (4) **Runtime observation**(EOF, cache, latency, free memory, 실패/완료): 실제 binding/실행에서 확인하고, 잘못된 추정으로 J 의미를 바꾸지 않는다. 'Unknown'은 불변·저비용·무효과를 의미하지 않는다.

**Array Shape, Rank, Frame/Cell.** `shape=[N,784]` 같은 기호 차원은 generic graph의 반복 사용을 돕는다. `"` Rank conjunction의 *Cell 적용 의미*는 Shape declaration과 별개이며, Rank의 요청/실현·빈 Frame의 가상 Cell, fill/type/shape 조립 규칙을 계속 보존한다. Logical Array dtype·원소 순서·J-visible boxed/sparse 의미는 storage encoding과 다르다. 파생된 중간 배열 facts는 IR 분석으로 유도하고, `with`는 증명되지 않은 값을 보충하는 경계 정보만 제공한다. Static Value는 값 고정·version/provenance 보장 시에만 상수 최적화와 specialization key의 입력이 된다. 키에는 사용된 사실만 담고 widening/guard로 variant 폭증을 막는다.

**서로 다른 분할 단위.** `Logical ArrayRegion`(사용자에게 보이는 의미적 위치), `Storage ReadChunk`, `Storage WriteChunk/Shard`, `CodecChunk`, `IoRequestRange`, `ComputeTile`, `DevicePartition`은 하나의 `ChunkSize`가 아니다. 저장 포맷의 read/write/codec 경계와 compute tile은 달라도 된다. `ArrayRegion`에서 `FileByteRange`로 변환하는 책임은 layout/encoding을 검증한 physical adapter에 있으며, 일부 원소 요청이 전체 압축 block read/decode를 피할 수 있다는 보장은 없다.

**역방향 data-demand 분석(새 IR이 아닌 pass/sidecar 후보).** 최종 결과 또는 필요한 sink region을 seed로 놓고 그래프의 소비자에서 생산자 방향으로 `Demand`를 전파한다. Demand는 `None / KnownRegions / Full / Unknown`의 값 요구량과 dtype·shape/prototype 등 **별도 metadata/semantic-check 의무**를 표현하며, 여러 소비자는 영역의 합집합을 취한다. 원소별 연산은 대응 region, slice/transpose/reshape는 증명된 inverse index mapping, reduce는 요구 출력에 영향을 주는 reduction 축 전체, convolution은 kernel/stride/padding이 요구하는 halo, dynamic gather는 확인된 인덱스 또는 보수적 full/unknown으로 되돌린다. `GraphOperationContract::access`는 접근 *범주*일 뿐 정확한 region transformer가 아니므로, primitive별 transfer rule·witness가 없으면 reduction/Rank/boxed/sparse를 포함해 확대 분석하지 않는다. `Full` data demand는 '전체 bytes 필요'이지 '전체 배열 동시 메모리 상주'가 아니다. Streaming·Chunk 병렬화는 별도의 associativity, numeric promotion, errors/effect/order, state/halo 합법성 증명이 있어야 한다.

~~~text
J verb + with(input semantic requirements)
    -> J Graph IR + symbolic ArrayRegion demand sidecar
    -> Verified Logical Execution IR (effects/errors/checks/guards)
    + DataSource requirement/capability
    -> bind source, validate schema/version/layout/witness
    -> Physical candidates: exact/coalesced read, sequential scan,
       storage rechunk, compute tiling, bounded streaming, full read
    -> legal candidates only -> resource budget + measured/estimated cost
    -> executable plan (readiness, buffer lease, failure/cancel order)
~~~

**Binding/최적화 절차.** generic 컴파일은 Graph IR/정적 입력 요구와 가능한 access region을 보존한다. 실제 DataSource 결합 시 Hard 제약을 확인하고 source capability, physical chunk/codec/layout, version·lease를 수집한다. 실패하면 기존 의미론적으로 안전한 경로를 사용하거나 계약 불일치를 보고한다. 그다음 verified logical access demand를 byte ranges로 내리고, 작은 random reads 합치기·sequential scan·mmap·prefetch·rechunk·materialize 등의 **후보**를 비용 비교한다. 재사용되는 compiled plan은 관련 정적 조건과 source version이 유효할 때만 cache한다. 외부 변경이나 snapshot 검증 실패는 cache invalidation/guard failure이며, 이미 발생한 관찰 가능한 I/O를 소급하여 지우지 않는다.

**읽기·쓰기·효과 분리.** (A) 내부 immutable/versioned backing read, (B) J `1!:` foreign의 관찰 가능한 read/write/open/error, (C) mutable weight/checkpoint sink는 서로 다른 semantics다. ReadOnly ≠ externally immutable; repeatable ≠ snapshot; region-readable ≠ cheap random access; async submission ≠ successful completion/durability; mapped ≠ zero-copy. B의 사용되지 않는 read가 파일 오류를 낼 수 있으므로 단순 DCE·pushdown으로 제거할 수 없다. A의 speculative prefetch조차 사용자에게 보이는 첫 오류/순서를 바꾸지 않는다는 증명이 필요하다. C는 write range 겹침, ordering, commit/flush/durability, cancellation, partial failure를 별도 계약과 시험으로 검증한다. 빈 데이터 read를 생략하더라도 J 빈 Frame의 virtual Cell prototype/check 의무는 독립적으로 남는다.

**선행 시스템 대조 — 참고 모델이지 RustJ 기능 주장 아님.**

| 프레임워크 · 공식 출처 | 실제 추상화 / 교훈 | RustJ 도입 경계 |
|---|---|---|
| [TensorStore Schema](https://google.github.io/tensorstore/schema.html) · [Spec](https://google.github.io/tensorstore/spec.html) | dtype/rank/domain의 Hard constraint, storage layout의 Hard/Soft 제약, Read/Write/Codec Chunk 구분, 열 때 Schema 검증 | static/source contract의 강도와 물리 저장 배치 분리; TensorStore는 full-J 계산 의미론 제공자가 아님 |
| [DaCe SDFG](https://spcldace.readthedocs.io/en/latest/sdfg/ir.html) | Data Container, Tasklet, `Memlet`의 symbolic subset/volume로 계산과 데이터 이동 기술 | `ArrayRegion` 전파·witness 참고; Memlet을 파일 snapshot 보장으로 오해하지 않음 |
| [JAX JIT](https://docs.jax.dev/en/latest/jit-compilation.html) · [Shape Polymorphism](https://docs.jax.dev/en/latest/export/shape_poly.html) | Shape/DType abstract value, static argument specialization, symbolic dimension 제약 | `with`가 사전 계약을 제공하는 구조에 참고; JAX JIT가 일반 파일 I/O 계약까지 해결하지는 않음 |
| [Arrow Dataset Scanner](https://arrow.apache.org/docs/python/generated/pyarrow.dataset.Scanner.html) · [DuckDB Parquet](https://duckdb.org/docs/stable/data/parquet/overview.html) | column/filter pushdown, fragment/batch read-ahead 및 데이터 형식별 pruning | 순수하고 증명된 access에 한정; 임의 J verb나 foreign 오류를 없애지 않음 |
| [Dask Chunks](https://docs.dask.org/en/stable/array-chunks.html) | 논리 array shape와 계산 Chunk, rechunk 이동 비용이 독립 | Chunking은 physical policy; J-visible shape/rank 변경 금지 |

**기존 IO 체크리스트에만 반영하는 검증/이행 범위.** IO-03에 semantic `with` vs source capability vs bound physical descriptor의 stage ownership, provenance/guards, invalid-contract negative verifier를 포함한다. IO-04에는 sequential-only/single-pass vs region-readable/replayable, mutable vs versioned snapshot, Hard/Soft 차이, source binding mismatch matrix를 포함한다. IO-09/10에는 `None/Regions/Full/Unknown` demand transfer와 다중 소비자 union, slice/transpose/reduction/halo/indirect fallback 및 no-skip-on-error 증명을 포함한다. IO-11/12에는 immutable/versioned cache와 invalidation/witness를, IO-26/27에는 logical schema·read/write/codec chunk 분리 및 encoding/EOF/overflow 검증을, IO-16/17에는 full-demand streaming vs barrier, error visibility, buffer lease·completion을, IO-20/21/22에는 cost, exact/coalesced/scan 후보 측정 및 독립 3경로·negative 회귀를 포함한다. **IO-01~IO-30의 ID/상태는 변경하지 않으며 현재 구현 수용 0/30이다.** 이 계약 도입은 M2 또는 최초 M4 CPU 경로의 선행 조건이 아니다.
