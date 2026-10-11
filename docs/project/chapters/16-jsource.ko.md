[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)

> 문서 분할 안내: 기존 `PROJECT.ko.md`의 내용을 이동한 모듈입니다. 원문의 단일 파일 정본·신규 Markdown 금지 문구는 최신 [Agent 문서 규약](../../../AGENTS.md)보다 우선하지 않습니다.

## 14. jsource와 RustJ의 관계

현재 jsource는 CPU 실행을 전제로 한 C 구현이다.

문제는 C라는 언어 자체보다 다음 구조적 결합이다.

- C pointer 중심 배열 접근
- CPU address space 가정
- primitive 호출과 즉시 실행의 결합
- CPU allocation과 logical value의 결합
- 전체 graph 최적화 이전의 materialization
- backend-independent execution plan 부재

따라서 jsource를 그대로 Rust로 번역하면 주로 다음이 된다.

> CPU-oriented J interpreter written in Rust

RustJ의 목표는 다르다.

```text
jsource
  → semantic/reference oracle

RustJ frontend
  J source
    ↓
  J Semantic Array IR
    ↓
  Semantic Analyzer / Lowering
    ↓
  Logical Array IR / Plan
    ↓
  route selection
    ├─ RustJ native Physical Plan
    ├─ MLIR / LLVM / GPU dialects
    ├─ StableHLO-compatible route
    └─ verified library/custom backend
```

jsource에서 적극적으로 가져올 것:

- tokenizer/parser behavior
- primitive semantics
- rank/agreement
- type promotion
- boxing/sparse rules
- error semantics
- corner cases
- tests

그대로 상속하지 않을 것:

- A block의 물리 메모리 정체성
- pointer 기반 primitive architecture
- CPU loop structure
- CPU-only allocation strategy
- primitive 즉시 실행 dispatch
- backend-specific in-place machinery

---
