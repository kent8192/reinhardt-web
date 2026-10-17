# reinhardt-testkit

Core testing infrastructure for the Reinhardt framework.

## API client headers

`APIClient::get_with_headers` and `APIClient::post_raw_with_headers` replace
client default headers with the same name, including names that differ only
in letter case. For example, a request with `Authorization: Bearer alice`
replaces a default `Authorization: Bearer bob` with a single header value.
Unrelated default headers remain present, and subsequent requests retain the
client defaults. Multiple per-request values for the same header are retained
in order, including names that differ only in letter case. Invalid per-request
header names or values return an error.

## Session authentication

`client.auth().session(...).with_staff(...).with_superuser(...)` writes fixture
metadata and installs a session cookie. `CookieSessionAuthMiddleware` also needs
an `AuthBackend` that resolves the current test account and its overridden
privileges. Session privilege flags alone do not authorize a request.

Use the same private test fixture as the builder's session backend and the
middleware's session/account backends. The complete executable example in
[`SessionAuthBuilder`](src/auth/builder.rs) implements the existing
`AsyncSessionBackend` and `AuthBackend` traits with separate session and account
registries. Directly modifying stored session flags does not change current
account privileges. Keep this trusted fixture confined to tests.

## DI mock fixtures

`reinhardt-testkit` exposes three layers for mocking DI dependencies in tests:

1. `with_di_overrides!` macro (most ergonomic)
2. `DiOverrideBuilder` + `injection_context_with_di_overrides` (closure form)
3. `injection_context_with_overrides` (legacy; scope-seeding only)

```rust,no_run
use reinhardt_testkit::with_di_overrides;
use rstest::*;

#[rstest]
#[tokio::test]
async fn example() {
    let (ctx, _di) = with_di_overrides! {
        singleton Config { url: "test".into() },
    };
    // Use `ctx` as your test InjectionContext.
}
```

See `instructions/TESTING_STANDARDS.md` (the TI- entry about `with_di_overrides!`) for the full rule set.
