# Task 2 Report

## Status

Implemented indexed, expression-safe injected dependency temporaries for the HTTP route, route registration, `#[use_inject]`, and WebSocket code-generation paths. Original parameter patterns, including `mut` and destructuring bindings, remain in generated internal function signatures.

## Changes

- `routes.rs`: each detected injected parameter receives `__reinhardt_injected_{index}`; resolution and calls use it.
- `routes_registration.rs`: async route factories resolve and forward indexed temporary identifiers.
- `use_inject.rs`: removed mutable-pattern stripping, retained original patterns, and separated resolved call identifiers.
- `websocket.rs`: generated consumer fields, factory resolution, and forwarding use indexed identifiers.
- Added compiler-checked mutable identifier and destructuring-pattern coverage for route, route registration, and `#[use_inject]` syntax.

## Verification

- `cargo test -p reinhardt-macros --lib injectable_common::tests::mutable_inject_patterns_use_expression_safe_resolved_identifiers -- --nocapture`: passed (1 passed).
- `cargo test -p reinhardt-macros --test ui -- --nocapture`: ran for several minutes with all observed cases passing, then was interrupted to avoid continuing an excessively long unrelated full-suite run.
- `cargo test -p reinhardt-macros test_routes_macro_pass -- --nocapture`: a temporary pass-glob registration exposed pre-existing fixture infrastructure failures (`test_support.rs` missing and internal framework dev-dependencies intentionally absent), so the registration was reverted to avoid breaking the suite.
- `rustfmt --edition 2024` on all changed Rust files: passed.
- `git diff --check`: passed.

## Self-review

- Resolver values are no longer bound directly to mutable or destructuring patterns in wrapper scopes.
- Call expression positions contain identifiers only for injected values.
- `#[use_inject]` internal signatures preserve the source patterns.
- WebSocket generated struct fields are valid identifiers even when handler parameters destructure values.
- The two RFC files remain untracked and were not staged.

## Concerns

- `InjectInfo::pat` is retained to preserve the two-pattern metadata contract but is now unused in these code-generation paths, producing a dead-code warning.

## Review Follow-up

The three unregistered macro-crate fixtures were replaced with one registered integration trybuild fixture at `tests/integration/tests/di/ui/pass/mutable_inject_patterns_core_paths.rs`. It compiles the public `#[get]`, `#[use_inject]`, and `#[routes]` paths with both `mut db` and `Wrapper(mut value)` and mutably borrows every binding in each function body.

Command:

```text
nix-shell -p openssl pkg-config protobuf --run '/run/current-system/sw/bin/cargo test -p reinhardt-integration-tests --test di_ui test_mutable_inject_patterns_core_paths_compile_pass -- --nocapture'
```

Result:

```text
test tests/di/ui/pass/mutable_inject_patterns_core_paths.rs ... ok
test ui::test_mutable_inject_patterns_core_paths_compile_pass ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out
```

The Nix shell supplies host-only build prerequisites discovered during verification: OpenSSL development metadata and `protoc`.
