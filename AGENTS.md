# RustJ change validation

Use `PROJECT.md` as the single authoritative document for architecture, roadmap, support status, and validation policy.

For every implementation change:

- Update `PROJECT.md` when architecture, boundaries, roadmap, checklist, supported semantics, or validation status changes.
- Do not create new design/progress/checklist Markdown reports. Keep machine-generated measurement data in `reports/` when useful.
- Add a reproducer and regression coverage for every semantic bug fix.
- Extend conformance coverage for newly supported semantics.
- Run Rust default and portable tests, fmt, clippy, and relevant Python harness tests.
- For semantic/kernel changes, compare available C reference variants when the environment supports them. Report known deviations separately from passes.
- For storage changes, check alias preservation, transactional assignment, allocation behavior, overlap, and retained-memory limits.
- For SIMD changes, cover tails, integer overflow/promotion, exceptional floating-point values, and portable fallback.
- State which checks actually ran. Do not claim upstream suite, Miri, sanitizer, GPU, Linux CI, or remote CI coverage unless executed.
- CUDA implementation remains deferred until the user requests resumption and a verifiable GPU environment is available.
