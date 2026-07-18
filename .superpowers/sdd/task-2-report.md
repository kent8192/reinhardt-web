# Task 2 Report: Structured Server Function Error Integration

## Scope

Connected the Task 1 `ServerFnError` version 1 envelope to native server function handlers and generic client stubs. Existing URL and MessagePack success codecs remain unchanged, while their error responses are always JSON envelopes. Extractor failures retain sanitized user-facing messages.

## RED evidence

1. Added `validation_handler_returns_versioned_error_envelope` before changing status selection.

   ```text
   cargo test -p reinhardt-pages --test server_fn_native_handler_tests validation_handler_returns_versioned_error_envelope -- --exact --nocapture

   assertion `left == right` failed
     left: 500
    right: 422
   ```

   This demonstrated that the default registration implementation only propagated the legacy `server` kind and collapsed the version 1 validation envelope to 500.

2. Restored the prior generic client-stub error branch, added `generated_client_stub_decodes_generic_error_envelopes`, and ran it before restoring the implementation.

   ```text
   cargo test -p reinhardt-pages-macros generated_client_stub_decodes_generic_error_envelopes -- --nocapture

   generic client stubs must decode structured error envelopes
   ```

   The generated code used `ServerFnError::server(__status, __message)` and did not contain `from_http_response`.

3. Added the `from_http_response` behavior test before its implementation. The focused lib command was blocked before test execution by pre-existing `reinhardt-pages` test-harness compilation errors in navigation macro self-crate resolution and `install_task_sink` feature gating. The behavior test was then retained in the native integration target, where it executes independently.

4. The first MessagePack integration run exposed that this test target had no direct `base64` dev-dependency, although the generated MessagePack handler references `::base64`. Added the existing workspace dependency and reran the real handler test.

## GREEN evidence

```text
cargo test -p reinhardt-pages --test server_fn_native_handler_tests
PASS: 6 passed

cargo test -p reinhardt-pages --test server_fn_codec_integration_tests --features msgpack
PASS: 9 passed

cargo test -p reinhardt-pages-macros generated_client_stub_decodes_generic_error_envelopes -- --nocapture
PASS: 1 passed

cargo check -p reinhardt-pages --lib
PASS

cargo fmt --all -- --check
PASS

git diff --check
PASS
```

The native suite covers the version 1 validation envelope, status 422 propagation, typed authentication extractor errors, typed internal extractor errors, and sanitized raw HTTP error fallback. The codec suite proves URL JSON success behavior and MessagePack base64 success behavior are preserved while both codecs return version 1 JSON validation envelopes on error.

## Changed files

- `crates/reinhardt-pages/src/server_fn/registration.rs`
  - Propagate any valid envelope status from 100 through 599; malformed or invalid envelopes still use 500.
- `crates/reinhardt-pages/src/server_fn/server_fn_trait.rs`
  - Added `ServerFnError::from_http_response` and its private status-aware deserialization fallback constructor. Invalid raw bodies now become a sanitized deserialization error and are never copied into the user message.
- `crates/reinhardt-pages/macros/src/server_fn.rs`
  - Generic client stubs decode error bodies through `from_http_response`.
  - Authentication extractor failures now use the typed `auth` constructor while preserving the existing sanitized message.
  - Added macro expansion coverage for the generic client-stub error path.
- `crates/reinhardt-pages/tests/server_fn_native_handler_tests.rs`
  - Added native envelope, sanitizer, and HTTP response decoding coverage.
- `crates/reinhardt-pages/tests/server_fn_codec_integration_tests.rs`
  - Added URL and MessagePack native handler success/error coverage.
- `crates/reinhardt-pages/Cargo.toml`
  - Added the existing workspace `base64` crate as a dev-dependency required by generated MessagePack handler integration tests.

## Concern

The brief's all-features command was attempted:

```text
cargo test -p reinhardt-pages --test server_fn_native_handler_tests --test server_fn_codec_integration_tests --all-features
```

It is blocked before either target runs by pre-existing `model-server-fnset` compilation failures in `crates/reinhardt-pages/src/server_fn/model_set/runtime.rs`: missing `reinhardt_db::orm::TransactionScope`, immutable connections passed to APIs that require `&mut`, and `usize` to `u64` mismatch. These files are outside Task 2 and the default/MessagePack focused suites above are green.

## Commit

The Task 2 implementation is committed on the non-protected feature branch. No push was performed.
