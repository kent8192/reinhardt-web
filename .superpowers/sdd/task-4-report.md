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

## Reviewer follow-up

### Production fix

- Extended `ClientForm` field-name resolution to parse all valid field rename
  forms relevant to serialization:
  - `#[serde(rename = "wire")]`;
  - `#[serde(rename(serialize = "wire", deserialize = "..."))]`, using only
    the serialize value;
  - container `#[serde(rename_all = "...")]` and its directional form, using
    the serialize rule when a field has no explicit serialize rename.
- Implemented every Serde-supported `rename_all` rule: `lowercase`,
  `UPPERCASE`, `PascalCase`, `camelCase`, `snake_case`,
  `SCREAMING_SNAKE_CASE`, `kebab-case`, and `SCREAMING-KEBAB-CASE`.
- Preserved normalized and raw Rust field names as resolver aliases, and kept
  unrelated Serde attributes on both container and field declarations
  consumable by the existing macro parser.
- Added strict `form_state().error` assertions for all-matched and
  unmatched-only server errors. Each seeds a previous value and verifies that
  `apply_server_error` synchronizes the current first error.
- `use_form_action` was not changed.

### Follow-up TDD evidence

The directional integration test was added before production parser changes:

```text
CARGO_TARGET_DIR=/tmp/reinhardt-task-5567-review.PGEsSu \
  CARGO_BUILD_BUILD_DIR=/tmp/reinhardt-task-5567-review.PGEsSu/build \
  cargo test -p reinhardt-pages --test client_form_integration \
  client_form_routes_directional_serialize_rename_and_raw_field_names
```

RED result: the existing macro rejected directional syntax with `expected =` at
`serde(rename(serialize = ...))`.

After the parser and container rename-all implementation, focused GREEN checks
passed for directional rename, raw identifiers, `camelCase`,
`SCREAMING-KEBAB-CASE`, all-matched first-error synchronization, and unmatched
nested form-level synchronization.

Final verification passed:

```text
CARGO_TARGET_DIR=/tmp/reinhardt-task-5567-review.PGEsSu \
  CARGO_BUILD_BUILD_DIR=/tmp/reinhardt-task-5567-review.PGEsSu/build \
  cargo test -p reinhardt-pages --test use_form_integration \
  --test client_form_integration
```

Result: `use_form_integration` 43 passed; `client_form_integration` 13 passed.

```text
CARGO_TARGET_DIR=/tmp/reinhardt-task-5567-review.PGEsSu \
  CARGO_BUILD_BUILD_DIR=/tmp/reinhardt-task-5567-review.PGEsSu/build \
  cargo clippy -p reinhardt-pages --test use_form_integration \
  --test client_form_integration -- -D warnings
cargo fmt --all -- --check
git diff --check
```

All follow-up commands passed.
