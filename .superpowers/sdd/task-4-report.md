# Task 4 implementation report

## Scope

Implemented Task 4 for Issue #5567: generated field-name resolution and
`UseFormReturn::apply_server_error` routing. The work is limited to the form
runtime, the `form!` and `ClientForm` generators, and their integration tests.
Task 5's typed submit helper was not implemented.

## Implementation

- Added `FormRuntimeSource::runtime_field_by_name` with a default `None`
  implementation so manual runtime sources remain source compatible.
- Generated serialized-name-to-field-token matches for `form!` forms.
- Made `ClientForm` delegate resolution to its generated `field_from_name`
  helper. The helper now accepts explicit `serde(rename = "...")` names while
  retaining normalized and raw Rust identifier names.
- Added `UseFormReturn::apply_server_error`:
  - groups repeated matching messages per field with newline separators;
  - replaces field error state with routed server errors;
  - aggregates unmatched or nested paths as `field: message` beneath the safe
    server message at form level;
  - clears both form and submit errors when every supplied field error routed;
  - otherwise keeps the aggregate in both form and submit error state;
  - calls `sync_first_error()` after state updates.

## Tests

Added strict integration coverage for:

- matching plus unmatched field errors;
- all errors matching fields and stale submit-error clearing;
- duplicate messages for one field;
- unmatched nested paths aggregating at form level;
- server errors with no field entries;
- `ClientForm` resolution for `serde(rename = "displayName")`, raw identifiers,
  and an unmatched resolver result.

## TDD evidence

### RED

The requested command was run after adding the routing tests and before
production implementation:

```text
cargo test -p reinhardt-pages --test use_form_integration use_form_routes_server_field_errors
```

The shared target initially stopped before test compilation because its volume
had only 37 MiB available and failed with `No space left on device`. No shared
cache was modified. The same command was rerun with an isolated target under
`/tmp`, where it failed as expected because `UseFormReturn` had no
`apply_server_error` method (five `E0599` diagnostics in the new tests).

### GREEN

```text
CARGO_TARGET_DIR=/tmp/reinhardt-task-5567.eR0z3t \
  CARGO_BUILD_BUILD_DIR=/tmp/reinhardt-task-5567.eR0z3t/build \
  cargo test -p reinhardt-pages --test use_form_integration \
  use_form_routes_server_field_errors
```

Result: 1 passed; 0 failed.

### Full focused verification

```text
CARGO_TARGET_DIR=/tmp/reinhardt-task-5567.eR0z3t \
  CARGO_BUILD_BUILD_DIR=/tmp/reinhardt-task-5567.eR0z3t/build \
  cargo test -p reinhardt-pages --test use_form_integration \
  --test client_form_integration
```

Result: `use_form_integration` 42 passed; `client_form_integration` 11 passed.

```text
CARGO_TARGET_DIR=/tmp/reinhardt-task-5567.eR0z3t \
  CARGO_BUILD_BUILD_DIR=/tmp/reinhardt-task-5567.eR0z3t/build \
  cargo clippy -p reinhardt-pages --test use_form_integration \
  --test client_form_integration -- -D warnings
cargo fmt --all -- --check
git diff --check
```

All commands passed.

## Commit and delivery

The Task 4 report is intentionally included in the Task 4 commit. No push was
performed.
