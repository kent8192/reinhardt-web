# Native Component Test Harness

**Issue**: [#5583](https://github.com/kent8192/reinhardt-web/issues/5583)
**Date**: 2026-07-04
**Status**: Approved design (pending implementation plan)

## Summary

Add a native component testing harness for `reinhardt-pages` that can render a
`Page` into an in-memory interactive DOM, query it with Testing
Library-inspired role/text/label helpers, dispatch common events, mock
`server_fn` calls per test, and await reactive settling without a browser or
WASM toolchain.

The first public surface lives in `reinhardt_pages::testing::component` and is
re-exported through the `reinhardt` facade as `reinhardt::test::pages`.

## Motivation

Meaningful component behavior tests currently need the WASM path: browser
execution, `wasm-pack`, mocked fetch, and a headless runtime. That path remains
valuable for hydration and browser API coverage, but it is too expensive for
the majority of component interaction tests.

The framework already has a native `Page` representation, native event handler
storage via `DummyEvent`, SSR rendering, native client API stubs, and
MSW-style `MockableServerFn` metadata. The missing layer is an interactive
test runtime that uses those native structures directly.

## Goals

- Render a `Page` under plain `cargo test` with no browser, no `wasm-pack`, and
  no loopback server for the component harness itself.
- Query rendered output by user-facing semantics: role, accessible name, label,
  placeholder, and text.
- Dispatch common events against stored `Page` event handlers.
- Let `screen.settle().await` drive test-scoped async work and reactive
  rerendering deterministically.
- Mock `server_fn` calls in process, per test, using generated
  `MockableServerFn` marker metadata.
- Produce stable pretty output suitable for diagnostic messages and snapshots.

## Non-Goals

- Browser E2E, hydration correctness, layout, CSS, focus ring, scrolling, and
  visual assertions.
- Full DOM, `web-sys`, or ARIA implementation parity.
- Replacing the WASM smoke-test path.
- Replacing native direct `server_fn` unit tests for business logic.
- Intercepting arbitrary HTTP clients, `reqwest`, `hyper`, AWS SDK calls, or
  OS-level network traffic.

## Recommended Approach

Build a native interactive DOM from the `Page` tree itself.

Alternatives considered:

| Approach | Trade-off |
|---|---|
| Native `TestNode` tree from `Page` | Fast, preserves event handlers and reactive structure, matches existing types |
| SSR HTML parse plus event sidecar | Reuses HTML output but makes event and reactive mapping fragile |
| Broad native pseudo-`web-sys` DOM | Browser-like, but far beyond the accepted scope |

The `Page` tree is the authoritative structure for this harness. HTML parsing
would discard exactly the event and reactive metadata the harness needs.

## Public API

Primary module:

```rust
reinhardt_pages::testing::component
```

Facade re-export:

```rust
reinhardt::test::pages
```

Representative usage:

```rust
use reinhardt::test::pages::{Role, render};

#[rstest]
#[tokio::test]
async fn refresh_loads_jobs() {
    let screen = render(jobs(Path(1)));
    screen.mock_server_fn::<load_jobs::marker>(|args| Ok(vec![job("Index job")]));

    screen.get_by_role(Role::Button, "Refresh").click();
    screen.settle().await;

    assert!(screen.query_by_text("Index job").is_some());
}
```

Initial stable types:

| Type | Purpose |
|---|---|
| `Screen` | Owns the rendered test DOM, query engine, scheduler, and server function mocks |
| `ElementHandle` | Refers to a node inside a `Screen` by stable node id |
| `Role` | Typed role query input |
| `TextMatch` | Exact text matching for queries |
| `QueryError` | Structured query failure diagnostics |
| `EventError` | Structured event dispatch failure diagnostics |
| `SettleError` | Structured async settling failure diagnostics |

Initial query methods:

| Method family | Behavior |
|---|---|
| `get_by_role`, `get_by_text`, `get_by_label`, `get_by_placeholder` | Panic with rich diagnostics on zero or multiple matches |
| `try_get_by_*` | Return `Result<ElementHandle, QueryError>` |
| `query_by_*` | Return `Option<ElementHandle>` on zero or one match; panic on multiple matches |
| `find_by_*` | Await `settle()` loops until one match or timeout |
| `try_find_by_*` | Async `Result` variant for helper code |

Initial event methods:

| Method | Event type |
|---|---|
| `click()` / `try_click()` | `click` |
| `submit()` / `try_submit()` | `submit` |
| `input(value)` / `try_input(value)` | `input`, updates test element value first |
| `change(value)` / `try_change(value)` | `change`, updates test element value first |

Snapshot support starts with:

```rust
screen.pretty()
```

The returned string must be stable enough for `insta` snapshots.

## Architecture

`Screen` owns four coordinated subsystems:

1. **In-memory DOM tree**: `TestNode` values built from `Page`.
2. **Query engine**: role, accessible-name, label, placeholder, and text
   matching over `TestNode`.
3. **Test scheduler**: harness-scoped async task queue and reactive rerender
   queue.
4. **Server function mocks**: per-screen registry keyed by
   `MockableServerFn::PATH` and marker type.

The render pipeline is:

```text
Page -> TestNode tree -> query/event -> scheduler -> rerender
```

`ElementHandle` stores the owning screen reference and a node id. It does not
own nodes directly. After rerendering, a handle remains usable if the node id is
still present. If the node has been removed, event and read operations fail
with `DetachedElement`.

## Test DOM Model

`TestNode` stores:

- node id
- optional parent id
- node kind: element, text, fragment root, reactive anchor
- tag name for element nodes
- attributes
- children
- current form value for input-like nodes
- event handlers copied from `PageElement`
- source role/name cache invalidated on rerender

Fragments do not appear as queryable elements. They only group children under
the screen root. `Page::WithHead` renders only the view portion. Head metadata
is outside component interaction scope.

Reactive nodes are represented as anchors whose current child subtree is
replaceable. This keeps rerendering local and avoids rebuilding the full screen
for every signal change.

## Reactive Rendering

`Page::Reactive` and `Page::ReactiveIf` are evaluated inside a harness render
scope. The scope records which reactive anchor produced which subtree.

When signals or effects mark a reactive anchor dirty, the screen schedules that
anchor for rerender. Rerendering replaces the anchor's current child subtree
with a fresh `Page -> TestNode` expansion.

Normal native and SSR behavior must not change. Outside a component harness,
native task spawning remains a no-op and native resources stay in their
existing SSR-oriented loading behavior.

## Scheduler

The native platform layer gains an internal test hook point. Under a `Screen`
RAII guard, `platform::native::spawn_task` enqueues futures into the active
harness scheduler. Without the guard, it keeps the current no-op behavior.

`screen.settle().await` repeats this loop:

1. poll queued tasks until no immediately-ready work remains;
2. apply pending signal/effect notifications;
3. rerender dirty reactive anchors;
4. enqueue any new tasks produced by rerendered hooks;
5. stop when task and rerender queues are both empty.

`settle()` has a timeout and maximum iteration count. A self-triggering effect
or infinite refetch loop returns `SettleError::DidNotQuiesce` with pending task
counts and `screen.pretty()` output.

The first implementation requires an async test context such as
`#[tokio::test]` or async `rstest`. Blocking settle helpers are out of scope
for the first release.

## Server Function Mocks

The harness adds an in-process mock registry for generated native client stubs.
`screen.mock_server_fn::<S>(handler)` registers a handler where
`S: MockableServerFn` and the handler accepts `S::Args` and returns
`Result<S::Response, ServerFnError>`.

Generated native client stubs check the active harness registry before taking
their normal native path. When a matching handler exists, the stub:

1. serializes the original call arguments into `S::Args`;
2. records the call;
3. calls the registered handler;
4. returns the handler response using the same `ServerFnError` type as normal
   server functions.

If a component harness is active and no mock exists for a called server
function, the stub returns a deterministic `ServerFnError` that names the
missing path. It must not perform external HTTP or use the native
`MockServiceWorker` loopback runtime.

Call inspection:

```rust
let calls = screen.calls_to_server_fn::<load_jobs::marker>();
assert_eq!(calls.len(), 1);
```

The existing `MockServiceWorker` remains the right tool for native tests that
explicitly exercise HTTP clients against a loopback server. The component
harness registry is for in-process component tests only.

## Queries And Accessibility

Query priority follows Testing Library's user-centric model:

1. role plus accessible name;
2. label for form controls;
3. placeholder for inputs;
4. text for non-interactive content.

Initial `Role` variants:

- `Alert`
- `Button`
- `Checkbox`
- `Combobox`
- `Dialog`
- `Form`
- `Heading`
- `Link`
- `List`
- `Listbox`
- `ListItem`
- `Main`
- `Navigation`
- `Option`
- `Progressbar`
- `Radio`
- `Status`
- `Textbox`

`Role::Custom(String)` is intentionally not included in the first release.
Unknown roles should become explicit enum additions so test semantics stay
auditable.

Role resolution order:

1. explicit `role` attribute;
2. supported HTML implicit roles.

Initial implicit role coverage:

| HTML | Role |
|---|---|
| `button` | `Button` |
| `a[href]` | `Link` |
| `input[type=text/search/email/password/url/tel]` | `Textbox` |
| `input[type=checkbox]` | `Checkbox` |
| `input[type=radio]` | `Radio` |
| `textarea` | `Textbox` |
| `select` | `Combobox` or `Listbox` based on attributes |
| `form` | `Form` when it has an accessible name |
| `nav` | `Navigation` |
| `main` | `Main` |
| `h1` through `h6` | `Heading` |
| `dialog` | `Dialog` |
| `ul` / `ol` | `List` |
| `li` | `ListItem` |
| `option` | `Option` |
| `progress` | `Progressbar` |

Accessible name resolution for MVP:

1. `aria-label`;
2. `aria-labelledby` references;
3. `label[for=id]` for form controls;
4. nearest parent `label`;
5. button/link/heading text content;
6. input placeholder as a fallback for input-centric queries.

Elements with `hidden` or `aria-hidden="true"` are excluded by default.
Hidden opt-in can be added later.

`TextMatch` starts with exact string matching. Regex, predicate matching, and
case-insensitive matching are deferred.

Diagnostics include the failed query, candidate roles/names, and pretty DOM.

## Event Dispatch

Event methods invoke stored `PageEventHandler` values with `DummyEvent`.

For input-like events, the test node stores a current value before dispatching
the handler. The event object remains `DummyEvent` in the first release; richer
event payloads can be added when the framework has a cross-target event value
abstraction.

Event bubbling is out of scope for the first release. The handler attached to
the target element is called directly. This matches the current `PageElement`
storage model and keeps the first harness deterministic.

## Error Handling

The API provides both panic-first test ergonomics and `Result`-returning helper
variants.

| Error | Examples |
|---|---|
| `QueryError::NotFound` | no matching role/text/label/placeholder |
| `QueryError::MultipleMatches` | `get_by_*` found more than one element |
| `EventError::DetachedElement` | stale handle after rerender removed node |
| `EventError::MissingHandler` | `click()` on an element without a click handler |
| `SettleError::DidNotQuiesce` | timeout or max iteration count exceeded |
| `SettleError::TaskFailed` | scheduler task failed or panicked |

Panic messages should include the structured error plus `screen.pretty()`.
`Result` variants should carry enough data for framework tests to assert on the
failure without string matching.

## Testing Strategy

Tests live primarily in `crates/reinhardt-pages`.

Required coverage:

| Area | Coverage |
|---|---|
| Render tree | element, text, fragment, keyed fragment, `WithHead` view content |
| Queries | role/name, label, placeholder, text, hidden exclusion, multiple match error |
| Events | `click`, `submit`, `input`, `change`, missing handler, detached handle |
| Reactivity | signal-driven rerender after event |
| Async hooks | `use_action` success/error and `use_resource` success/error/refetch |
| Server functions | registered mock, unregistered mock error, call recording |
| Settling | quiescent case, timeout/non-quiescent diagnostic |
| Snapshots | stable `screen.pretty()` output |
| Facade | `reinhardt::test::pages` re-export compiles in a downstream-style test |

Validation commands for the implementation phase should include at least:

```bash
cargo test -p reinhardt-pages component_testing --all-features
cargo test -p reinhardt-pages --test ui --all-features
cargo make fmt-check
cargo clippy -p reinhardt-pages --all-features --tests -- -D warnings
```

If public API changes affect semver checks, run the repository's local semver
check workflow before marking the eventual PR ready.

## Documentation

Update these documentation surfaces with implementation:

- `crates/reinhardt-pages/src/testing.rs`
- `crates/reinhardt-pages/README.md`
- `crates/reinhardt-pages/CHANGELOG.md`
- the `reinhardt` facade docs or README surface that exports
  `reinhardt::test::pages`

The docs should position this as the native component testing layer. They
should keep direct `server_fn` invocation as the default for business logic and
WASM/browser tests as the default for hydration and browser API coverage.

## Performance Acceptance

The harness should make a representative interaction test at least one order
of magnitude faster than the equivalent WASM/browser path. The first
implementation should record the measured native command and representative
duration in the PR notes or a small benchmark note; timing should not become a
brittle unit-test assertion. The acceptance target is browser-toolchain-free
execution in the single-digit to low-double-digit millisecond range for a
small component interaction.

## Implementation Boundaries

The implementation should stay within these boundaries:

- Keep normal native `spawn_task` behavior unchanged outside active component
  test scopes.
- Use RAII guards for scheduler and mock registry activation.
- Avoid introducing broad browser DOM abstractions.
- Avoid adding new unresolved placeholder markers.
- Keep query semantics explicit and documented rather than silently accepting
  unsupported ARIA behavior.
- Preserve existing WASM MSW behavior and native `MockServiceWorker` behavior.

## Open Design Decisions Closed By This Spec

| Decision | Choice |
|---|---|
| MVP scope | Full vertical slice: render, query, event, settle, server_fn mock |
| API location | `reinhardt_pages::testing::component`, facade as `reinhardt::test::pages` |
| Async model | async tests using `screen.settle().await` |
| Server function mocking | in-process registry keyed by `MockableServerFn` metadata |
| DOM model | `Page` to native `TestNode`, not parsed SSR HTML |
| Snapshot model | `screen.pretty()` stable text output |
