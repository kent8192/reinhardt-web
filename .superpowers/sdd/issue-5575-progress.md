# Issue #5575 SDD Progress

## Scope

- Plan: `docs/superpowers/plans/2026-07-05-issue-5575-arena-copy-handles.md`
- Base: `develop/0.4.0`
- Branch: `docs/issue-5575-arena-copy-handles-design`
- Preserved unrelated change: `.serena/project.yml`

## Task status

- Task 1: complete in `567e32805a` (`feat(core): add reactive scope arena foundation`)
- Task 2: complete
- Task 3: complete
- Task 4: pending
- Task 5: pending
- Task 6: pending
- Task 7: pending
- Task 8: pending

## Verification ledger

### Task 1 baseline

- `cargo test -p reinhardt-core --features reactive --lib reactive::scope`
  - Result: PASS
  - Evidence: 4 passed, 0 failed, 814 filtered out

## RED/GREEN reports

### Task 2: `Signal<T>`

RED:

- `cargo test -p reinhardt-core --features reactive --lib signal_is_copy`
  - Result: compile failure because `Signal<i32>: Copy` was not implemented.
- `cargo test -p reinhardt-core --features reactive --lib signal_new_requires_scope`
  - Result: failed because the test did not panic.
- `cargo test -p reinhardt-core --features reactive --lib signal_panics_after_scope_dispose`
  - Result: failed because the test did not panic.

GREEN:

- `cargo test -j1 -p reinhardt-core --features reactive --lib reactive::signal`
  - Result: PASS, 12 passed, 0 failed.
- `cargo test -j1 -p reinhardt-core --features reactive --lib into_deps_single_signal`
  - Result: PASS, 1 passed, 0 failed.

### Task 3: `Memo<T>` and `Effect`

RED:

- `cargo test -j1 -p reinhardt-core --features reactive --lib memo_is_copy`
  - Result: compile failure because `Memo<i32>: Copy` and `Effect: Copy` were not implemented.
- Existing focused tests initially failed after the `Signal` migration because they created nodes without a scope.
- `new_with_deps_cleanup_can_dispose_same_effect_without_reentrant_borrow`
  - Result: failed with `RefCell already borrowed`, proving cleanup had to release its borrow before self-disposal.

GREEN:

- `cargo test -j1 -p reinhardt-core --features reactive --lib reactive::memo::tests -- --test-threads=1`
  - Result: PASS, 11 passed, 0 failed.
- `cargo test -j1 -p reinhardt-core --features reactive --lib reactive::effect::tests -- --test-threads=1`
  - Result: PASS, 19 passed, 0 failed.
