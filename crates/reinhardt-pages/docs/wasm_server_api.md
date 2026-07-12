# wasm_server_api Macro

`#[wasm_server_api]` declares public APIs that must keep the same Rust surface
on browser WASM and server/native targets while allowing target-specific
implementations.

Use it when a Reinhardt Pages API needs different runtime behavior across the
WASM/server boundary but must expose one public name, signature, visibility, and
documentation contract. The macro is a compile-time guard against drift.

## When to Use It

Use `#[wasm_server_api]` for small public functions where:

- The WASM implementation needs browser APIs, hydration state, or client-side
  scheduling.
- The server implementation needs SSR behavior, native I/O, or an explicit
  unsupported-target branch.
- Reviewers should be able to see both implementations next to each other.
- A signature, visibility, or documentation mismatch would be a public API bug.

Do not use it for:

- APIs whose behavior is identical on every target. A normal function is clearer.
- Large modules where target-specific submodules are easier to review.
- `#[server_fn]` RPC functions. Those already generate a WASM client stub and a
  server handler contract.
- Private helper functions that do not define public cross-target API shape.

## Syntax

Apply the macro to an inline module. Each target-specific public function must
have exactly one `#[wasm]` or `#[server]` marker, and every marked function must
have a matching counterpart with the same name and signature.

```rust
use reinhardt_pages::wasm_server_api;

#[wasm_server_api]
pub mod platform_api {
    #[doc = "Returns the active target family name."]
    #[wasm]
    pub fn target_name() -> &'static str {
        "wasm"
    }

    #[doc = "Returns the active target family name."]
    #[server]
    pub fn target_name() -> &'static str {
        "server"
    }
}
```

The expansion emits the WASM variant behind:

```rust
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
```

and the server/native variant behind:

```rust
#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
```

Unmarked module items are preserved unchanged.

## Validation

The macro rejects:

- Missing `#[wasm]` or `#[server]` counterparts.
- Duplicate variants for the same target.
- Non-public target variants.
- Mismatched signatures.
- Mismatched non-target attributes.
- `#[wasm]` or `#[server]` markers on non-function items.

This keeps unsupported target behavior explicit: if a target cannot support the
operation, write the unsupported branch as the matching implementation rather
than omitting it.
