# Native MSW Mock Server Design

## Summary

Issue #4916 adds native HTTP support to `reinhardt-test::msw` without
promising transparent interception. The native backend will run a local HTTP
mock server and expose its base URL for explicit injection into the code under
test. This keeps the existing MSW-style handler API while avoiding global proxy
or client-middleware behavior that native Rust clients do not share.

The first implementation targets REST handler coverage for native tests. It
does not migrate `reinhardt-storages` S3 fixtures, add AWS SDK interceptors, or
provide process-wide environment/proxy helpers.

## Goals

- Make `reinhardt_test::msw::MockServiceWorker` available on native targets.
- Reuse the existing `rest::*`, `RestHandler`, `UrlMatcher`, `MockResponse`,
  and recorder APIs across WASM and native runtimes.
- Start a loopback server on `127.0.0.1:0` from `MockServiceWorker::start()`.
- Expose the server base URL through `MockServiceWorker::url()` for explicit
  endpoint injection.
- Preserve existing WASM `window.fetch` interception behavior.
- Document that native MSW is a local mock server, not transparent native
  traffic interception.

## Non-Goals

- No `HTTP_PROXY`, service-specific environment variable, or global process
  configuration helper in the first version.
- No `tower::Layer`, `reqwest` middleware, AWS SDK interceptor, or
  `aws_smithy_runtime` integration.
- No passthrough proxying to real upstream services.
- No binary response body redesign. Current `MockResponse` stores `String`
  bodies, so object-storage binary mocks remain out of scope for the first
  native MSW backend.
- No mandatory migration of existing `wiremock` tests such as
  `reinhardt-storages` S3 tests.

## Architecture

`reinhardt-test::msw` should be split conceptually into shared core and
target-specific runtime code.

Shared core:

- `context`
- `handler`
- `matcher`
- `recorder`
- `response`
- `rest`

These modules remain the common API and behavior surface. They already compile
far enough on native targets for unit tests, so the main work is to remove the
runtime-only assumption that a worker always means `window.fetch`.

The shared core must not keep a single mutability model for both targets. The
current WASM runtime can keep `Rc<RefCell<...>>` style state because it runs
inside browser-local JavaScript execution. The native runtime needs state that
can safely cross an async server task boundary, such as `Arc<Mutex<...>>` or an
equivalent internal store. This applies to handlers, recorder state, and
one-shot handler consumption.

WASM runtime:

- Keeps the current `window.fetch` override.
- Keeps the existing single-active-interceptor behavior and recovery of stale
  global fetch state.
- Keeps current `wasm_bindgen_test` integration coverage.

Native runtime:

- Owns a loopback HTTP server task.
- Uses native-safe shared state for handlers and request recording.
- Converts each HTTP request into `InterceptedRequest`.
- Records the request before handler lookup, matching WASM behavior.
- Applies the first matching handler.
- Converts `MockResponse` into an HTTP response.
- Stops the server from `stop().await` and `Drop`.

The native runtime must not be described as intercepting arbitrary native HTTP
traffic. Tests must inject `worker.url()` into the HTTP client or service
configuration they want to exercise.

## Public API

Existing cross-target API:

```rust
let worker = MockServiceWorker::new();
worker.handle(rest::get("/api/data").respond(MockResponse::json(42)));
worker.start().await;
worker.calls_to("/api/data").assert_called();
worker.stop().await;
```

The following methods retain the same meaning across WASM and native targets:

- `MockServiceWorker::new()`
- `MockServiceWorker::with_policy(policy)`
- `MockServiceWorker::start().await`
- `MockServiceWorker::stop().await`
- `MockServiceWorker::reset()`
- `MockServiceWorker::reset_handlers()`
- `MockServiceWorker::handle(handler)`
- `MockServiceWorker::calls_to(pattern)`
- `MockServiceWorker::all_calls()`

New error-returning startup API:

```rust
worker.try_start().await?;
```

`start().await` remains the convenience API and should panic with a clear
message if `try_start()` fails. `try_start()` returns an `MswError` for bind
failures, unsupported policy combinations, and invalid lifecycle transitions.

Native URL API:

```rust
let endpoint = worker.url();
```

`url()` returns `&str` and panics with a clear message if called before native
startup. This catches forgotten `start().await` calls immediately in test code.
On WASM, `url()` is not required for the initial design.

Native `respond_with` closures must be usable from the native server task. The
native implementation should require `Send + Sync + 'static` for dynamic
response closures, while preserving the existing lighter WASM closure bounds
where practical. If a single cross-target type forces one bound set, prefer a
target-specific internal type alias rather than silently making WASM tests pay
for native thread-safety requirements.

## Native Request Flow

1. A test creates a `MockServiceWorker`.
2. The test registers REST handlers.
3. `worker.start().await` binds a local server on `127.0.0.1:0`.
4. The test passes `worker.url()` into the code under test.
5. The code under test sends real HTTP requests to the local server.
6. The native runtime builds `InterceptedRequest` with URL, method, selected
   headers, and UTF-8 request body when present.
7. The recorder stores the request before handler lookup.
8. The first matching handler supplies the response.
9. The response is written to the client.

## Native Error Behavior

`UnhandledPolicy::Error`:

- Return a deterministic HTTP 500 response with a diagnostic body naming the
  method and URL.
- This is easier to debug than a synthetic connection abort for unmatched
  requests.

`UnhandledPolicy::Warn`:

- Emit a warning through the test runtime logging path.
- Return the same deterministic HTTP 500 response as `Error`.
- Native passthrough does not exist in the first version, so warning plus
  passthrough would be misleading.

`UnhandledPolicy::Passthrough`:

- Unsupported for native startup in the first version.
- `try_start()` returns an `MswError`; `start().await` panics with the same
  explanation.

`rest::*.network_error()`:

- Native runtime should make the client observe a transport-level error by
  closing the request without a successful HTTP response.
- If the chosen server primitive cannot represent this cleanly, prefer a
  dedicated `MswError` during implementation over silently converting it into a
  normal mock response.

Handler delays:

- Native runtime applies configured delays before sending the response.
- Delay behavior should use Tokio time on native targets and keep the existing
  WASM timer behavior.

## Feature Flags and Dependencies

The current `msw` feature is WASM-shaped. The design changes its meaning to
"enable MSW-style request mocking" across supported targets.

Expected direction:

- Keep `wasm` and `wasm-full` as browser testing feature flags.
- Keep `msw` as the public MSW API feature.
- On WASM, `msw` continues enabling `wasm` and `reinhardt-pages/msw`.
- On native, `msw` enables only native server dependencies needed by the mock
  server runtime.

Implementation should prefer existing workspace HTTP/server dependencies when
they fit. If existing `reinhardt-testkit` server helpers do not allow the
needed lifecycle or network-error behavior, use a small native server runtime
inside `reinhardt-test::msw` instead of bending unrelated testkit APIs.

## Testing

Native tests should live in `crates/reinhardt-test/tests/native_msw.rs` and use
a normal HTTP client against `worker.url()`.

Required coverage:

- GET handler with JSON response.
- POST handler that echoes request body.
- Parameterized path matching.
- Multiple handlers and first-match behavior.
- `once()` handler consumption.
- Handler delay.
- `reset()` clears handlers and recorder.
- `reset_handlers()` preserves recorder.
- `UnhandledPolicy::Error` returns deterministic diagnostic response.
- `UnhandledPolicy::Passthrough` fails native startup clearly.
- Call recording and `calls_to()` assertions.
- `stop().await` and `Drop` release the server.

Existing WASM tests remain regression coverage for fetch interception and
server function transport.

## Documentation

Update `crates/reinhardt-test/src/msw.rs` and `crates/reinhardt-test/README.md`
to show separate WASM and native examples.

Native documentation must say plainly:

- Native MSW starts a local mock HTTP server.
- The test must inject `worker.url()` into the system under test.
- It does not intercept arbitrary `reqwest`, `hyper`, AWS SDK, or OS-level
  network calls.
- `Passthrough` is not supported on native targets in the first version.

## Rollout

1. Refactor MSW internals so shared handler and recorder state can be owned by
   both runtimes.
2. Add native `MockServiceWorker` runtime behind `msw` on native targets.
3. Add native integration tests.
4. Update docs and changelog.
5. Leave existing storage/auth/conf/utils `wiremock` tests untouched unless a
   later issue explicitly migrates them.

## Acceptance Criteria

- `cargo test -p reinhardt-test --features msw native_msw` passes on native.
- Existing WASM MSW tests continue to compile and pass under their current
  command.
- `cargo make fmt-check` passes.
- Public docs no longer describe MSW as WASM-only.
- Native docs do not imply transparent interception.
