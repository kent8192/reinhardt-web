# Arena-Backed Copy Handles for Reactive Primitives

## Summary

Issue #5575 makes Reinhardt reactive handles `Copy` by moving owned storage out
of individual handles and into scoped arenas. The target handles are
`Signal<T>`, `Memo<T>`, `Action<T, E>`, `Resource<T, E>`, and
`Callback<A, R>`.

The design is intentionally breaking. Existing handle names remain the primary
API, but their ownership model changes: handles become small copied keys, and a
`ReactiveScope` owns all backing nodes until the scope is disposed.

## Goals

- Make the existing `Signal<T>`, `Memo<T>`, `Action<T, E>`,
  `Resource<T, E>`, and `Callback<A, R>` handle types implement `Copy`.
- Remove `.clone()` ceremony when handles are captured by hook closures and
  dependency tuples.
- Keep public method names and behavior source-compatible where possible.
- Tie node lifetime to explicit reactive scopes rather than individual handle
  drops.
- Preserve SSR request isolation and hydration behavior.
- Provide deterministic diagnostics for node creation outside a scope and stale
  handle access after disposal.

## Non-Goals

- This does not change hook dependency tuple semantics.
- This does not change `page!` capture syntax.
- This does not introduce cancellable async tasks as part of the first
  implementation. Late async completion is guarded by generation checks.
- This does not add a thread-root fallback scope. Node creation requires an
  active scope.

## Approach

Use a two-layer arena design:

- `reinhardt-core::reactive` owns `ReactiveScope`, `ScopeId`, generational
  `NodeKey`, and the core arena for `Signal<T>`, `Memo<T>`, and `Effect`.
- `reinhardt-pages` owns a Pages arena for `Action<T, E>`,
  `Resource<T, E>`, and `Callback<A, R>` storage, keyed by the same active
  `ReactiveScope` identity and generation rules.

This preserves crate boundaries. Core owns general reactive primitives and
dependency graph state. Pages owns browser, hook, callback, and async wrapper
state.

## ReactiveScope Contract

Node creation requires an active `ReactiveScope`.

Normal Pages entrypoints create scopes automatically:

- `SsrRenderer::render*`
- hydration entrypoints
- CSR mount entrypoints
- component rendering paths used by those entrypoints

Low-level users, tests, custom renderers, and benchmarks can create scopes
explicitly:

```rust
use reinhardt_core::reactive::{ReactiveScope, Signal};

ReactiveScope::run(|| {
    let count = Signal::new(0);
    count.set(1);
    assert_eq!(count.get(), 1);
});
```

`Signal::new`, `Memo::new`, `Memo::new_with_deps`, `Effect::new`, and the Pages
hooks panic with a diagnostic when called without an active scope.

## Core Arena Model

Core handles become copied keys:

```rust
pub struct Signal<T: 'static> {
    key: NodeKey,
    _marker: PhantomData<fn() -> T>,
}
```

`Signal<T>` stores the value in the current scope's core arena. On native
targets, the slot uses synchronized interior mutability compatible with the
current `Send + Sync` expectations. On WASM, it uses single-threaded interior
mutability.

`Memo<T>` stores its computation closure, cached value, dirty flag, and explicit
dependency notifier in the core arena. `Effect` stores its effect closure,
cleanup slot, timing, and dependency subscriptions in the core arena.

The dependency graph remains in core runtime state, but graph nodes are scoped.
Removing a scope removes all dependency graph entries owned by that scope.

## Pages Arena Model

Pages handles also become copied keys. Pages storage is scoped separately from
core storage but tied to the same active `ReactiveScope`.

`Action<T, E>` stores:

- a core `Signal<ActionPhase<T, E>>` for visible state,
- the dispatch closure,
- payload storage,
- optimistic success/error callbacks.

`Resource<T, E>` stores:

- a core `Signal<ResourceState<T, E>>` for visible state,
- the refetch closure,
- the dependency-tracking effect key that keeps refetch subscriptions alive.

`Callback<A, R>` stores:

- the typed callback closure,
- dependency tuple identity for `use_callback` / `use_callback_with`,
- call-site identity for stable callback behavior.

The Pages arena clears these slots when the owning `ReactiveScope` is disposed.

## Lifecycle

Scope disposal is the ownership cleanup path.

Dropping an individual handle does nothing because handles are `Copy`. Dropping
the owning scope:

- runs effect cleanup functions,
- removes dependency graph entries,
- drops stored values and closures,
- clears Pages arena slots,
- bumps slot generations so stale copied handles cannot alias reused slots.

For SSR, each render request gets a fresh scope and drops it after producing the
HTML. Reactive state from one request cannot be reused by another request.

For hydration and CSR, the mounted root owns the scope for the mounted app
lifetime. Unmounting the root disposes the scope.

## Async Completion

`Action` and `Resource` tasks capture copied keys and the generation observed at
dispatch/refetch time. Completion performs a guarded write:

- if the scope and node generation still match, the result updates the state
  signal;
- if the scope or node is gone, the result is discarded.

This avoids resurrecting disposed state. It also avoids requiring a cancellable
future contract for arbitrary user futures and browser fetches.

## Diagnostics

The arena lookup helper provides uniform diagnostics.

Creation without active scope panics with a message that points users to
`ReactiveScope::run` or the Pages entrypoints.

Direct stale handle access after disposal panics deterministically. The
diagnostic includes the node kind, slot index, expected generation, actual
generation, and scope id when available.

Async late completion after disposal does not panic because no user code is
directly accessing the stale handle. It discards the completion.

Debug builds may include richer metadata such as creation location. Release
builds still check generations and must never alias stale handles to reused
slots.

## API Migration

Public method names remain as stable as possible:

- `Signal<T>` keeps `.get()`, `.get_untracked()`, `.with_untracked()`,
  `.set()`, `.update()`, and `.id()`.
- `Memo<T>` keeps `.get()`, `.get_untracked()`, and `.id()`.
- `Action<T, E>` keeps `.phase()`, `.dispatch(payload)`, `.reset()`,
  `.is_pending()`, `.result()`, `.error()`, and optimistic helpers.
- `Resource<T, E>` keeps `.get()`, `.set()`, `.refetch()`, `.is_loading()`,
  and related predicates.
- `Callback<A, R>` keeps `Callback::new`, `.call(args)`, and event handler
  conversion.

The visible app migration is removing handle clones. For example:

```rust
let upload_click = use_callback(
    move |_| {
        index_action.reset();
        search_action.reset();
        upload_action.dispatch(route_project_id.get());
    },
    (route_project_id,),
);
```

The dependency tuple receives copied handles. The closure also captures copied
handles.

Low-level code that creates reactive nodes outside Pages entrypoints must wrap
the work in `ReactiveScope::run`.

Documentation that says `Signal::clone()` is cheap should be updated to say
reactive handles are `Copy` and can be passed directly.

## Implementation Boundaries

The implementation should start in `reinhardt-core::reactive`:

- introduce `ReactiveScope`, `ScopeId`, `NodeKey`, and core arena lookup;
- move `Signal` value storage into scoped slots;
- migrate `Memo` and `Effect` storage into scoped slots;
- keep dependency graph behavior equivalent.

Then migrate `reinhardt-pages`:

- add Pages arena storage tied to the current `ReactiveScope`;
- convert `Action`, `Resource`, and `Callback` to copied keys;
- preserve existing hook semantics and re-export surfaces;
- wire automatic scope creation in SSR, hydration, CSR mount, and component
  rendering entrypoints.

## Testing

Focused coverage should include:

- compile assertions that `Signal<T>`, `Memo<T>`, `Action<T, E>`,
  `Resource<T, E>`, and `Callback<A, R>` implement `Copy`;
- `ReactiveScope::run` tests showing node creation succeeds inside a scope and
  panics outside one;
- stale handle tests after scope disposal;
- dependency tracking tests after arena migration;
- hook tests proving handles can be captured and listed in dependency tuples
  without `.clone()`;
- `Action` and `Resource` late-completion tests showing disposed scopes do not
  receive writes;
- SSR tests proving separate render calls do not share reactive nodes;
- hydration or WASM smoke tests proving a client scope lives through mount and
  is disposed on unmount when an unmount handle is available.

Suggested validation sequence:

```bash
cargo test -p reinhardt-core --features reactive --lib
cargo test -p reinhardt-pages reactive hooks resource action --lib
cargo check -p reinhardt-pages --all-features
cargo make fmt-check
cargo clippy -p reinhardt-pages --all-features --lib -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p reinhardt-pages --all-features --no-deps
```

## Acceptance Mapping

- `Signal<T>: Copy`: handled by core arena keys.
- Zero handle clones in the `sources.rs` callback example: dependency tuples
  and closures both receive copied handles.
- SSR and hydration unchanged: public entrypoints create and own scopes.
- No leaks across SSR requests: every render request uses a fresh scope.
- Use-after-dispose diagnostics: generation mismatch is checked on arena
  lookup.
- Migration guide: update `crates/reinhardt-pages/docs/react_to_reinhardt.md`
  and related reactive examples.
