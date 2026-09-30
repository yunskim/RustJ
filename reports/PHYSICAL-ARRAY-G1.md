# G1 — 읽기 전용 CPU affine 물리 배열

구현일: 2026-09-30. src/physical.rs와 tests/physical.rs.
GPU 친화적 설계의 첫 CPU 저장 표현이며 GPU 실행 또는 물리 실행기 완료를 뜻하지 않는다.

## 구현 경계

- BufferId는 registry identity/slot/generation이며 analysis::ValueId와 별도 타입이다. raw address를 ID로 사용하지 않는다.
- BufferRegistry는 기존 Value의 CPU storage를 이동·공유 등록한다. 슬롯 재사용은 generation을 증가시키고 다른 registry/stale ID 조회를 거절한다.
- BufferLease는 Arc 기반 owning read lease다. registry 제거/drop 이후에도 기존 view가 유효하다. inline 값은 안정된 Arc backing에 보관한다.
- 서로 다른 ID의 동일 shared Vec allocation도 alias로 판정한다. 같은 Arc<Vec<u8>>를 Bool과 Char로 등록한 경우도 포함한다.
- PhysicalArray는 private fields의 immutable affine mapping이다. shape/원소 단위 signed strides/offset과 initialized backing을 checked 검증한다.
- encoding은 BoolByte/Int64/Float64/Char8만 허용하며 dtype는 실제 encoding에서 도출한다. boxed/sparse는 명시적인 Unsupported다.
- empty는 shape를 보존하고 offset/strides=0, singleton stride=0으로 정규화한다. scalar는 초기화된 원소 하나를 요구한다.
- nonempty atom count와 주소·byte span은 checked 계산한다. 음수/zero stride와 내부 겹침은 읽기 전용으로 허용한다.
- logical_slice는 표준 논리 순서에서만 slice를 제공한다. transpose/reverse의 memory 순서를 J 순서로 가장하지 않는다.
- atom/backing_index는 인덱스 rank와 각 축의 bounds를 검사한다. mutable/general pointer 생성 API와 unsafe 코드가 없다.
- 실제 backing/view 시작 주소의 alignment와 retained payload capacity를 조회할 수 있다. capacity는 Arc/registry/shape/stride metadata bytes를 포함하지 않는다.

현재 Value·evaluator·pool은 변경하지 않았다. PhysicalArray::from_value는 독립 Rust API adapter다.
등록 시 작은 owner/registry/stride metadata 할당은 발생할 수 있다. payload 복사 없음과 zero allocation은 다른 조건이다.
공유 여부와 initialized len은 immutable backing lifetime에 고정되어 있어 descriptor의 검증 이후 바뀌지 않는다.
제거된 ID는 lookup에서 무효지만 이미 확보한 lease는 값 읽기에 계속 유효하다.
첫 mutable/reuse 경로, memory-contiguous 별도 slice API, strided reshape, device ABI는 후속 범위다.
CPU backing은 완료된 immutable 값만 보유한다. CUDA readiness/transfer 기록을 지원한다고 선언하지 않는다.

## Windows 검증

- Windows native stable MSVC, target/windows-validation에서 default/portable 각각 활성 테스트 90개 + doctest 2개 통과.
- 새 physical 테스트 14개: 이동·registry 성장, lease 탈출 수명, stale/cross-registry ID, 공유 payload identity,
  교차 byte encoding alias, empty/singleton/scalar/high rank, 음수/zero/겹침 stride, index/span/overflow,
  transpose·prefix 반복 mapping, dtype/특수 실수, 미지원 encoding, view alignment/retained capacity.
- 작은 2D shape/stride/offset 5,733조합을 원소별 독립 주소 나열로 검사했다. descriptor의 lo/hi 공식을 oracle에 복제하지 않았다.
- doctest의 compile-fail 사례는 lease owner보다 borrowed slice가 오래 살아남지 못함을 확인한다.
- Windows cargo fmt, Clippy all-targets -D warnings 통과. 로그: target/physical-g1-windows.log (ignored local artifact).
- 함수 정의 수용 테스트 17개는 여전히 ignored다. 통과 수에 넣지 않는다.
- Linux/WSL 테스트, GitHub CI, CUDA, native Windows C oracle, 성능 측정 및 Miri/sanitizer는 실행하지 않았다.

## 다음 단계

G2에서 transpose/reverse/fill 없는 slice와 표준 연속 reshape를 checked metadata 변환으로 구현하고,
logical-order materialization과 dense reference 비교를 추가한다. G3/G4에서 실제 연산 및 source 경로에 연결한다.
현재 generic atom lookup은 correctness API이며 SIMD 또는 빠른 stride 루프를 구현한 것은 아니다.
