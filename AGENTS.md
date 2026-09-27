# RustJ change validation

Follow reports/VALIDATION-STRATEGY.md for every implementation change.

- Add a reproducer and regression coverage for every semantic bug fix.
- Extend tools/conformance.py for newly supported semantics. Preserve stateful reproductions.
- Run Rust default and portable tests, fmt, clippy, and Python harness tests.
- For semantic/kernel changes, compare both C variants against both Rust backends. Report known deviations separately from passes; never broaden exceptions to make a check green.
- For storage changes, check alias preservation, transactional assignment, allocation behavior, and retained-memory limits.
- For SIMD changes, cover tails, integer overflow/promotion, and exceptional floating-point values.
- State which checks actually ran, and document any unavailable checks. Do not claim upstream suite, Miri, sanitizer, GPU, or remote CI coverage unless executed.
- CUDA implementation remains deferred until the user requests resumption.
