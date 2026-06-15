# Reinhardt Pages for React developers

This guide maps common React frontend concepts to Reinhardt Pages. It is
written for developers who already know React, JSX, hooks, server actions, and
hydration, and want to transfer that mental model to a Rust + WASM Reinhardt
application.

## Quick mapping

| React concept | Reinhardt Pages concept | Main difference |
| --- | --- | --- |
| JSX | `page!` macro | Rust expressions and typed parameters are used inside the macro. |
| Function component | Rust function returning `Page` | Props are normal typed Rust arguments or structs. |
| Fragment | Multiple top-level `page!` nodes or `Page::fragment` | The output is a `Page::Fragment`, not a virtual DOM fragment. |
| `useState` | `use_state` returning `(Signal<T>, SetState<T>)` | Reads use `signal.get()`, writes use `set(value)` or `signal.update(...)`. |
| `useEffect` | `use_effect(f, deps)` | Dependencies are explicit Rust tuples, for example `(count.clone(),)`. |
| `useLayoutEffect` | `use_layout_effect(f, deps)` | Same dependency model, layout timing. |
| `useMemo` | `use_memo(f, deps)` | Returns `Memo<T>`; read it with `.get()`. |
| `useCallback` | `use_callback(f, deps)` / `use_callback_with(f, deps)` | Returns a typed `Callback`, usually for event handlers. |
| `useReducer` | `use_reducer(reducer, initial)` | The reducer is a pure Rust function from `(&State, Action)` to `State`. |
| `useRef` | `use_ref(initial)` | Mutating a `Ref<T>` does not notify the reactive graph. |
| `useContext` | `Context<T>` + `provide_context` + `use_context` | Missing context returns `Option<T>`. |
| `useTransition` | `use_transition()` | Returns `TransitionState` with `is_pending` and `start_transition`. |
| `useDeferredValue` | `use_deferred_value(signal)` | Defers a `Signal<T>` value. |
| React actions / server functions | `use_action` + `#[server_fn]` | Server calls are typed Rust functions with generated WASM client stubs. |

## React 19 and 19.2 parity classification

React 19 and 19.2 added frontend APIs around form actions, async reads,
server/client boundaries, metadata, asset loading, and transition boundaries.
Reinhardt Pages maps those expectations to Rust-first APIs instead of copying
React API names.

| React concept | Reinhardt classification | Tracking |
| --- | --- | --- |
| `useActionState` | Documentation-only mapping to `form!`, `use_form`, `use_action`, and `#[server_fn]`; no React-named clone. | #5309 |
| `<form action={function}>` | Explicit non-goal. Reinhardt keeps static form contracts and typed RPC bindings separate. | #5309 |
| Generic `use(...)` for Promise reads | Explicit non-goal. Use `use_resource(fetcher, deps)` for async data. | #5310 |
| Generic `use(...)` for Context reads | Explicit non-goal. Use typed `Context<T>` with `use_context`. | #5310 |
| Suspense integration for async reads | Existing API with different semantics through `SuspenseBoundary` and resource tracking. | #5310 |
| React Server Components and RSC/Flight transport | Explicit non-goal. Reinhardt does not provide React-compatible component transport. | #5311 |
| `"use client"` / `"use server"` directives | Explicit non-goal. Reinhardt uses Rust/WASM targets and `#[server_fn]`, not directive strings. | #5311 |
| Server reference passing | Explicit non-goal. `#[server_fn]` generates typed client stubs but does not serialize server function references into client components. | #5311 |
| Automatic metadata hoisting | Explicit non-goal. Reinhardt metadata stays explicit through `head!` and `Head`; component body nodes are not hoisted into the document head. | #5312 |
| React DOM asset APIs such as `preinit`, `preload`, `preconnect`, and `prefetchDNS` | Reinhardt-native explicit `Head` / `LinkTag` asset hint helpers with exact duplicate SSR deduplication; no browser-only imperative asset API. | #5312 |
| `createPortal` | Candidate follow-up. `ClientLauncher::ensure_portal` is a launcher helper, not a general portal API. | #5313 |
| Custom element property, attribute, and event interop | Candidate follow-up against `page!` attributes, typed events, and DOM abstraction. | #5314 |
| `ref` as a regular prop | Candidate follow-up. Current guidance is `use_ref` and typed component props. | #5314 |
| `Activity` and `ViewTransition` | Candidate follow-up. Existing `use_transition`, `use_deferred_value`, signals, and `SuspenseBoundary` are related but not equivalent. | #5315 |
| Cross-target API parity guardrails | Implementation follow-up for a wasm/server parity macro before broadening dual-target public APIs. | #5199 |

The umbrella tracker for this classification is #5198. It should close only
after this table reflects the final outcome of each focused follow-up.

## Metadata and asset loading

React 19 allows metadata such as `<title>`, `<meta>`, and `<link>` to appear in
component bodies and be hoisted into the document head. Reinhardt Pages keeps
that contract explicit. Document metadata belongs in `head!` or in a `Head`
value attached to the page; ordinary `page!` body nodes render where they are
declared.

Asset loading hints use the same explicit head model. `Head` and `LinkTag`
provide helpers for common browser hints:

- `preconnect`
- `dns_prefetch`
- `module_preload`
- `preload_script`
- `preload_style`
- `preload_image`
- `preload_font`

```rust,ignore
use reinhardt::pages::prelude::*;

fn document_head() -> Head {
    Head::new()
        .title("Dashboard")
        .preconnect("https://cdn.example.com")
        .dns_prefetch("https://cdn.example.com")
        .module_preload("/static/app.mjs")
        .preload_script("/static/app.js")
        .preload_style("/static/app.css")
        .preload_image("/static/hero.png")
        .preload_font("/static/font.woff2")
}
```

During SSR, Reinhardt removes exact duplicate head entries after rendering each
entry to HTML. The deduplication is conservative: entries with different
attributes, media conditions, `crossorigin` values, or Open Graph payloads remain
separate. Hydration does not scan component bodies for metadata or run a
browser-only imperative asset loader; the server-rendered head remains the
deterministic source of document-level metadata and hints.

## Components, props, and children

In React, a component is a function that returns JSX. In Reinhardt Pages, a
component is usually an ordinary Rust function that returns `Page`.

```rust,ignore
use reinhardt::pages::prelude::*;

#[derive(Clone)]
struct UserCardProps {
    name: String,
    role: String,
}

fn user_card(props: UserCardProps) -> Page {
    page!(|props: UserCardProps| {
        article {
            class: "user-card",
            h2 { { props.name.clone() } }
            p { { props.role.clone() } }
        }
    })(props)
}
```

Children are explicit values. Use a `Page` argument when the caller should pass
rendered children, or use `Page::fragment` when the component needs to group
multiple children without adding a wrapper.

```rust,ignore
use reinhardt::pages::prelude::*;

fn panel(title: String, body: Page) -> Page {
    page!(|title: String, body: Page| {
        section {
            class: "panel",
            h2 { { title.clone() } }
            { { body.clone() } }
        }
    })(title, body)
}
```

## JSX to `page!`

`page!` is the closest Reinhardt Pages equivalent to JSX. It produces `Page`
values that can be rendered on the server, hydrated in the browser, or composed
with other pages.

```rust,ignore
use reinhardt::pages::prelude::*;

fn counter_button(count: Signal<i32>, set_count: SetState<i32>) -> Page {
    page!(|count: Signal<i32>, set_count: SetState<i32>| {
        button {
            class: "counter",
            @click: {
                let count = count.clone();
                let set_count = set_count.clone();
                move |_event| set_count(count.get() + 1)
            },
            "Count: "
            { count.get().to_string() }
        }
    })(count, set_count)
}
```

The syntax is intentionally Rust-first:

- Attribute names are Rust identifiers where possible, such as `class`.
- Event handlers use `@event_name`, such as `@click`.
- Rust expressions are written in braces.
- Values captured by reactive closures should usually be cloned before moving
  them into nested event handlers or `watch` blocks.

## State and reactivity

React state is component-local and re-rendered through the virtual DOM.
Reinhardt state is fine-grained: `Signal<T>` tracks readers and notifies only
the dependent reactive work.

```rust,ignore
use reinhardt::pages::prelude::*;

fn counter() -> Page {
    let (count, set_count) = use_state(0);
    counter_button(count, set_count)
}
```

Use `watch { ... }` when a `page!` branch should re-evaluate as signals change.
Static `if` expressions are evaluated only when that `Page` is built.

```rust,ignore
page!(|count: Signal<i32>| {
    watch {
        if count.get() == 0 {
            p { "No clicks yet" }
        } else {
            p { { format!("Clicked {}", count.get()) } }
        }
    }
})(count)
```

`Signal::clone()` is cheap. Prefer cloning the signal handle instead of
extracting a value early when the UI must remain reactive.

## Effects and dependency tuples

React uses dependency arrays. Reinhardt Pages uses explicit dependency tuples.
The tuple is part of the Rust call, not inferred from signal reads inside the
effect body.

```rust,ignore
use reinhardt::pages::prelude::*;

let (count, _set_count) = use_state(0);

use_effect(
    {
        let count = count.clone();
        move || {
            log::info!("count = {}", count.get());
            None::<fn()>
        }
    },
    (count.clone(),),
);
```

Important differences from React:

- Pass `()` for mount-only effects.
- Pass `(signal.clone(),)` for one dependency. The trailing comma matters.
- Reading a signal inside `use_effect`, `use_layout_effect`,
  `use_memo`, or `use_callback` does not create hidden subscriptions.
  Subscriptions come from the dependency tuple.
- Cleanup is returned as `Option<C>` from the closure, matching React's
  cleanup behavior in a Rust type.

## Memoization and callbacks

Use `use_memo` for derived values and `use_callback` or `use_callback_with` for
stable callback handles.

```rust,ignore
use reinhardt::pages::prelude::*;

let (items, _set_items) = use_state(vec![1, 2, 3, 4]);
let (threshold, _set_threshold) = use_state(2);

let visible = use_memo(
    {
        let items = items.clone();
        let threshold = threshold.clone();
        move || {
            items
                .get()
                .into_iter()
                .filter(|item| *item > threshold.get())
                .collect::<Vec<_>>()
        }
    },
    (items.clone(), threshold.clone()),
);

let visible_for_click = visible.clone();
let on_click = use_callback(
    move |_event| {
        log::info!("visible item count = {}", visible_for_click.get().len());
    },
    (visible.clone(),),
);
```

## Reducers, refs, and context

`use_reducer` is useful when the next state depends on an action.

```rust,ignore
use reinhardt::pages::prelude::*;

#[derive(Clone)]
struct CounterState {
    value: i32,
}

enum CounterAction {
    Increment,
    Reset,
}

fn reducer(state: &CounterState, action: CounterAction) -> CounterState {
    match action {
        CounterAction::Increment => CounterState {
            value: state.value + 1,
        },
        CounterAction::Reset => CounterState { value: 0 },
    }
}

let (state, dispatch) = use_reducer(reducer, CounterState { value: 0 });
dispatch(CounterAction::Increment);
```

Use `use_ref` for mutable values that should not trigger reactive updates, such
as timers, previous values, or DOM handles.

```rust,ignore
let render_count = use_ref(0);
render_count.update(|count| *count += 1);
```

Context is type-safe. A missing provider returns `None` instead of throwing.

```rust,ignore
use reinhardt::pages::prelude::*;

static THEME: Context<String> = create_context();

provide_context(&THEME, "dark".to_string());
let theme = use_context(&THEME).unwrap_or_else(|| "light".to_string());
```

## Transitions and deferred values

`use_transition` marks state updates as lower priority. On WASM, transition
work runs asynchronously; on native SSR it runs synchronously.

```rust,ignore
let transition = use_transition();
let (query, set_query) = use_state(String::new());

transition.start_transition({
    let set_query = set_query.clone();
    move || set_query("rust".to_string())
});

let deferred_query = use_deferred_value(query.clone());
```

Render loading or stale-state UI by reading `transition.is_pending.get()` and
`deferred_query.get()`.

## Forms, actions, and `#[server_fn]`

React server actions and client actions map to separate pieces in Reinhardt
Pages:

- `form!` defines a static form contract, including field metadata,
  validation, and rendered form structure.
- `use_form(&form)` builds runtime form state such as field values, validation
  errors, submit state, and submit lifecycle callbacks.
- `#[server_fn]` defines the server operation and generates the WASM client
  stub.
- `use_action` wraps an async mutation and exposes `Idle`, `Pending`,
  `Success`, and `Error` phases.

React `useActionState` combines form submission, pending state, result state,
and errors behind one hook. Reinhardt keeps those responsibilities explicit:
use `use_form` for typed form state and validation, then use `use_action` to
run the `#[server_fn]` mutation after the form is valid. React's DOM
`action={function}` behavior is not supported directly.

```rust,ignore
use reinhardt::pages::prelude::*;
use reinhardt::pages::server_fn::{ServerFnError, server_fn};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Todo {
    id: u64,
    title: String,
}

#[server_fn]
pub async fn create_todo(title: String) -> Result<Todo, ServerFnError> {
    // Persist on the server.
    Ok(Todo { id: 1, title })
}

fn todo_form() -> Page {
    let create = use_action(|title: String| async move {
        create_todo(title).await.map_err(|error| error.to_string())
    });

    page!(|create: Action<Todo, String>| {
        button {
            disabled: create.is_pending(),
            @click: {
                let create = create.clone();
                move |_event| create.dispatch("Write docs".to_string())
            },
            "Create"
        }
        if create.result().is_some() {
            p {
                role: "status",
                "Todo created"
            }
        }
        if create.error().is_some() {
            p {
                role: "alert",
                { create.error().unwrap_or_default() }
            }
        }
    })(create)
}
```

For generated forms, read submit state from the runtime returned by `use_form`:

```rust,ignore
let runtime = use_form(&profile_form)
    .on_submit_start(|handle| {
        log::info!("submitting {:?}", handle.get_values());
    })
    .on_submit_success(|handle| {
        assert!(handle.form_state().is_submit_successful.get());
    })
    .build();

let state = runtime.form_state();
let pending = state.is_submitting.get();
let submit_succeeded = state.is_submit_successful.get();
let visible_error = state.error.get();
```

For optimistic UI, keep predicted state in `use_optimistic`, attach it with
`Action::with_optimistic`, and let the action confirm it on success or revert it
on error.

## Routing

React Router concepts map to `ClientRouter`, route handlers returning `Page`,
`Link` for anchor generation, and `use_router` for imperative navigation.

```rust,ignore
use reinhardt::pages::prelude::*;
use reinhardt::ClientRouter;

fn home() -> Page {
    page!(|| { h1 { "Home" } })()
}

fn app_router() -> ClientRouter {
    ClientRouter::new()
        .route("home", "/", home)
        .not_found(|| page!(|| { h1 { "Not found" } })())
}
```

## SSR and hydration

React hydrates server HTML into a virtual DOM tree. Reinhardt Pages renders
`Page` values on the server and hydrates the existing DOM with fine-grained
reactive markers and event handlers. The important mental shift is that most
updates flow through signals and reactive scopes, not a full component-tree
diff.

Practical consequences:

- Keep server-rendered markup deterministic for the initial state.
- Put client-only effects in `use_effect` or `use_layout_effect`, not in the
  server render path.
- Use `#[server_fn]` for typed client-to-server mutations instead of manually
  duplicating API request stubs.
- Treat `#[server_fn]` as typed RPC, not as React Server Actions reference
  serialization.
- Prefer signal reads inside `watch { ... }` for reactive view branches.

## Intentional differences from React

Reinhardt Pages is React-aligned, not React-compatible. These differences are
intentional:

- There is no React-style indexed hook slot list. Hooks are Rust functions that
  return handles such as `Signal<T>`, `Memo<T>`, `Ref<T>`, and `Action<T, E>`,
  so there is no hook-call-order rule for preserving slot identity. Still,
  create long-lived state at component construction time instead of inside
  frequently re-run `watch` bodies unless new state is intended.
- Effect, memo, and callback dependencies are explicit tuples, not arrays and
  not implicit captures.
- Updates are fine-grained through signals instead of virtual DOM diffing.
- Event and DOM APIs are typed Rust APIs over `web-sys` on WASM and native
  stubs during SSR.
- Missing context is represented as `Option<T>`.
- There is no catch-all React-style `use(...)` API. Async resource reads,
  context reads, and loading boundaries use separate typed APIs.
- There is no React-compatible RSC/Flight transport or directive-string
  boundary. Use `#[server_fn]` for client-to-server RPC.

## Migration checklist

When porting a React component:

1. Convert JSX markup to `page!`.
2. Convert props to typed Rust arguments or a `Props` struct.
3. Replace `useState` values with `Signal<T>` reads and `SetState<T>` writes.
4. Replace dependency arrays with dependency tuples.
5. Move async mutations behind `#[server_fn]` and trigger them with
   `use_action`.
6. Replace React Router route elements with `ClientRouter` route handlers.
7. Review SSR assumptions and keep browser-only work inside effects.
