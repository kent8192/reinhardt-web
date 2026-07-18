# Task 3 Report: ValidationErrors conversion

## Scope

Implemented `From<ValidationErrors> for ServerFnError` in
`crates/reinhardt-pages/src/server_fn/server_fn_trait.rs`.

The conversion walks `ValidationErrors::field_errors()` directly, preserving
the collection's `BTreeMap` order and each field's insertion order. It emits
one `ServerFnFieldError` per validation error and uses `ValidationError`'s
`Display` implementation for the message. The resulting error uses the
existing validation constructor, including kind `Validation` and status `422`.

## RED evidence

Added the strict unit test
`validation_errors_convert_to_server_fn_field_errors` before implementing the
conversion.

Attempted:

```text
cargo test -p reinhardt-pages \
  server_fn::server_fn_trait::tests::validation_errors_convert_to_server_fn_field_errors \
  --lib
```

The first attempt could not reach the expected missing-`From` failure because
the shared Cargo build volume was full. Retrying with the required `testing`
feature reached existing unrelated test-only compilation failures in
`crates/reinhardt-pages/src/app/navigation.rs` and
`crates/reinhardt-pages/src/testing/component/tests.rs` (`Loader`, generated
`reinhardt_pages` paths, `install_task_sink`, and duplicate `deps` imports).
Therefore, the unit-test binary's expected RED assertion could not be observed
in this worktree; this report does not claim otherwise.

## GREEN evidence

```text
cargo --config "build.build-dir='/tmp/reinhardt-task3-target.gGGfSG/build'" \
  --config "build.target-dir='/tmp/reinhardt-task3-target.gGGfSG/target'" \
  check -p reinhardt-pages --features testing --lib
```

Result: `Finished dev profile` successfully. This verifies the production
conversion compiles.

```text
cargo --config "build.build-dir='/tmp/reinhardt-task3-target.gGGfSG/build'" \
  --config "build.target-dir='/tmp/reinhardt-task3-target.gGGfSG/target'" \
  test -p reinhardt-pages --test client_form_integration \
  client_form_validation_maps_dto_field_errors
```

Result: `1 passed; 0 failed`.

Also ran `cargo fmt --all -- --check` and `git diff --check` successfully.

## Commit

The implementation and unit test are committed as the Task 3 commit on the
non-protected branch. No push was performed.

## Concerns

- The requested field names (`email`, `name`) are ordered lexicographically by
  the existing `BTreeMap`, so the strict test asserts `email`, `name`, `name`.
- The unit-test binary remains blocked by the unrelated pre-existing compile
  failures listed above; the integration test and library check are green.
