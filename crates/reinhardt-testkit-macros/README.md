# reinhardt-testkit-macros

<!-- reinhardt-version-sync: reinhardt-testkit-macros -->

Procedural macros for [`reinhardt-testkit`](../reinhardt-testkit). The macros here are re-exported from `reinhardt-testkit` — depend on `reinhardt-testkit` rather than this crate directly.

## `with_di_overrides!`

Sets up a test `InjectionContext` with one or more dependencies overridden, and returns the context plus a guard token that reverts every override on drop.

```rust,no_run
use reinhardt_testkit::with_di_overrides;
use rstest::*;
use serial_test::serial;

#[rstest]
#[serial(di_registry)]
#[tokio::test]
async fn test_login_flow() {
    let (ctx, _di) = with_di_overrides! {
        singleton MockDatabase { url: "test://db".into() },
        singleton MockConfig { api_key: "test_key".into() },
        transient MockHttpClient => |_ctx| async {
            Ok(MockHttpClient::new())
        },
    };

    // ... assertions ...
}
```

See `reinhardt-testkit::fixtures::di_overrides` for the underlying API and the design rationale.
