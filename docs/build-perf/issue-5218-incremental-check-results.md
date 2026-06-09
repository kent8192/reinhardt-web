# Issue #5218 Incremental Check Measurements

These measurements compare `origin/main` at
`3016d326de63950ac553cd2246d02a0315bd7f06` with PR #5220 at the current
branch after reverting the experimental dev/test profile tuning back to
`debug = 1`.

Environment:

- Host: macOS on `aarch64-apple-darwin`
- Rust: `rustc 1.94.1 (e408947bf 2026-03-25)`
- Tool: `hyperfine --warmup 1 --runs 2`

## Results

| Scenario | `origin/main` mean | PR branch mean | Change |
|---|---:|---:|---:|
| `incremental-leaf-check` | 0.389s | 0.512s | 31.4% slower |
| `incremental-core-check` | 50.022s | 31.875s | 36.3% faster |

Raw command shapes:

```bash
touch crates/reinhardt-throttling/src/lib.rs && cargo check -p reinhardt-throttling
touch crates/reinhardt-core/src/lib.rs && cargo check --workspace
```

## Interpretation

The shared/core edit loop currently lands inside the expected 30-60% reduction
range. The leaf package check does not: it is slower in this local run, so PR
#5220 must not claim a universal incremental-check improvement.

The earlier experimental profile change (`line-tables-only`,
`split-debuginfo = "unpacked"`, `codegen-units = 256`, explicit
`incremental = true`) produced worse local numbers for this repository and was
reverted. Keep the default `debug = 1` profile unless a future benchmark proves
a different profile is faster across the target scenarios.

These measurements do not prove compile-free `page!` editing. Inline `page!`
macro edits still compile as Rust; compile-free template edits require a
separate dev-mode runtime/template architecture.
